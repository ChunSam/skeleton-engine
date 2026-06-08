//! Coin race — authoritative game server
//!
//! ```text
//! # Terminal 1
//! cargo run --example coin_race_server
//!
//! # Terminals 2, 3, ...
//! cargo run --example coin_race_game
//! ```
//!
//! Unlike `mp_server` (a dumb position relay), this server is **authoritative**: it owns
//! the coin field and the scoreboard. Clients never delete a coin on their own — they send
//! a `grab` claim and the server decides the winner of each coin (first claim wins). This is
//! what makes a contested-pickup game correct: two players touching the same coin in the same
//! frame both send `grab`, but only the first to reach the server scores.
//!
//! # Protocol (JSON text)
//!
//! | Direction | Message | Meaning |
//! |-----------|---------|---------|
//! | C → S | `{"type":"pos","x":<f32>,"y":<f32>}` | my local position |
//! | C → S | `{"type":"grab","coin":<N>}` | I claim coin N |
//! | S → C | `{"type":"hello","id":<N>,"target":<N>,"players":[{"id","score"}],"coins":[{"coin","x","y"}]}` | join snapshot |
//! | S → C | `{"type":"pos","id":<N>,"x":<f32>,"y":<f32>}` | another player's position |
//! | S → C | `{"type":"coin","coin":<N>,"x":<f32>,"y":<f32>}` | a coin (re)spawned |
//! | S → C | `{"type":"taken","coin":<N>,"by":<N>,"score":<N>}` | coin removed; `by` now has `score` |
//! | S → C | `{"type":"win","id":<N>}` | player N reached the target score |
//! | S → C | `{"type":"bye","id":<N>}` | player N disconnected |

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tungstenite::{accept, Message};

const ADDR: &str = "127.0.0.1:9002";
const MAX_JSON_MESSAGE_BYTES: usize = 4096;
/// Score required to win.
const TARGET_SCORE: u32 = 10;
/// Coins kept on the field at all times.
const COIN_COUNT: usize = 6;
/// Play-field bounds for coin spawns (matches the 800×600 client window with a margin).
const FIELD_MIN_X: f32 = 60.0;
const FIELD_MAX_X: f32 = 740.0;
const FIELD_MIN_Y: f32 = 60.0;
const FIELD_MAX_Y: f32 = 540.0;

// ── Wire types ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "pos")]
    Position { x: f32, y: f32 },
    #[serde(rename = "grab")]
    Grab { coin: usize },
}

#[derive(Debug, Serialize)]
struct PlayerSnapshot {
    id: usize,
    score: u32,
}

#[derive(Debug, Serialize)]
struct CoinSnapshot {
    coin: usize,
    x: f32,
    y: f32,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "hello")]
    Hello {
        id: usize,
        target: u32,
        players: Vec<PlayerSnapshot>,
        coins: Vec<CoinSnapshot>,
    },
    #[serde(rename = "pos")]
    Position { id: usize, x: f32, y: f32 },
    #[serde(rename = "coin")]
    Coin { coin: usize, x: f32, y: f32 },
    #[serde(rename = "taken")]
    Taken { coin: usize, by: usize, score: u32 },
    #[serde(rename = "win")]
    Win { id: usize },
    #[serde(rename = "bye")]
    Bye { id: usize },
}

// ── Authoritative state ─────────────────────────────────────────────────────────

struct Player {
    score: u32,
    sender: mpsc::Sender<Message>,
}

/// Dependency-free xorshift64 RNG — a game server doesn't need cryptographic randomness,
/// and pulling in `rand` just to scatter coins isn't worth the dependency.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform-ish f32 in `[lo, hi)`.
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let unit = (self.next_u64() >> 11) as f32 / (1u64 << 53) as f32;
        lo + unit * (hi - lo)
    }
}

struct Server {
    players: HashMap<usize, Player>,
    coins: HashMap<usize, (f32, f32)>,
    next_player_id: usize,
    next_coin_id: usize,
    winner: Option<usize>,
    rng: Rng,
}

impl Server {
    fn new(seed: u64) -> Self {
        let mut server = Self {
            players: HashMap::new(),
            coins: HashMap::new(),
            next_player_id: 1,
            next_coin_id: 1,
            // xorshift must never be seeded with 0.
            winner: None,
            rng: Rng(seed | 1),
        };
        for _ in 0..COIN_COUNT {
            server.spawn_coin();
        }
        server
    }

    /// Spawns one coin at a random field position and returns its `(id, x, y)`.
    fn spawn_coin(&mut self) -> (usize, f32, f32) {
        let id = self.next_coin_id;
        self.next_coin_id += 1;
        let x = self.rng.range(FIELD_MIN_X, FIELD_MAX_X);
        let y = self.rng.range(FIELD_MIN_Y, FIELD_MAX_Y);
        self.coins.insert(id, (x, y));
        (id, x, y)
    }

    /// Sends a message to every connected player except `except` (pass `None` for all).
    fn broadcast(&self, msg: &ServerMessage, except: Option<usize>) {
        let Ok(text) = serde_json::to_string(msg) else {
            return;
        };
        let frame = Message::Text(text.into());
        for (&pid, player) in self.players.iter() {
            if Some(pid) != except {
                let _ = player.sender.send(frame.clone());
            }
        }
    }
}

type Shared = Arc<Mutex<Server>>;

fn main() {
    let listener = TcpListener::bind(ADDR).expect("bind failed");
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15);
    let server: Shared = Arc::new(Mutex::new(Server::new(seed)));

    println!("coin_race_server: authoritative server on ws://{ADDR}");
    println!("  first to {TARGET_SCORE} coins wins · {COIN_COUNT} coins on the field");
    println!("  run `cargo run --example coin_race_game` in two or more windows");
    println!();

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("accept error: {e}");
                continue;
            }
        };
        let server = server.clone();
        thread::spawn(move || handle_client(server, stream));
    }
}

fn handle_client(server: Shared, stream: std::net::TcpStream) {
    let peer = stream.peer_addr().ok();
    let mut ws = match accept(stream) {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("WS handshake failed: {e}");
            return;
        }
    };
    // 5 ms read timeout → the loop drains the outbound queue ~200×/s while idle.
    ws.get_mut()
        .set_read_timeout(Some(Duration::from_millis(5)))
        .ok();

    let (tx, rx) = mpsc::channel::<Message>();

    // Register the player and build the join snapshot under one lock.
    let id = {
        let mut s = server.lock().unwrap();
        let id = s.next_player_id;
        s.next_player_id += 1;
        let players: Vec<PlayerSnapshot> = s
            .players
            .iter()
            .map(|(&pid, p)| PlayerSnapshot {
                id: pid,
                score: p.score,
            })
            .collect();
        let coins: Vec<CoinSnapshot> = s
            .coins
            .iter()
            .map(|(&coin, &(x, y))| CoinSnapshot { coin, x, y })
            .collect();
        s.players.insert(
            id,
            Player {
                score: 0,
                sender: tx,
            },
        );
        let hello = ServerMessage::Hello {
            id,
            target: TARGET_SCORE,
            players,
            coins,
        };
        let count = s.players.len();
        println!("[{id}] connected from {peer:?}  (total: {count})");
        // Send hello while holding the lock so it cannot race a `taken` for the snapshot.
        let Ok(hello_text) = serde_json::to_string(&hello) else {
            s.players.remove(&id);
            return;
        };
        if ws.send(Message::Text(hello_text.into())).is_err() {
            s.players.remove(&id);
            return;
        }
        id
    };

    'main: loop {
        // Drain outbound (relayed positions, coin events, etc.).
        loop {
            match rx.try_recv() {
                Ok(msg) => {
                    if ws.send(msg).is_err() {
                        break 'main;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break 'main,
            }
        }

        // Read one inbound frame (or time out and loop).
        match ws.read() {
            Ok(Message::Text(text)) => {
                if text.len() > MAX_JSON_MESSAGE_BYTES {
                    eprintln!("[{id}] dropped oversized message: {} bytes", text.len());
                    continue;
                }
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(msg) => handle_message(&server, id, msg),
                    Err(err) => eprintln!("[{id}] invalid JSON: {err}"),
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // read timeout — keep looping to service the outbound queue
            }
            Err(_) => break,
        }
    }

    cleanup(&server, id);
}

fn handle_message(server: &Shared, id: usize, msg: ClientMessage) {
    let mut s = server.lock().unwrap();
    match msg {
        ClientMessage::Position { x, y } => {
            s.broadcast(&ServerMessage::Position { id, x, y }, Some(id));
        }
        ClientMessage::Grab { coin } => {
            // The game is over — ignore late claims.
            if s.winner.is_some() {
                return;
            }
            // First claim wins: if the coin is already gone, this claim lost the race.
            if s.coins.remove(&coin).is_none() {
                return;
            }
            let score = {
                let Some(player) = s.players.get_mut(&id) else {
                    return;
                };
                player.score += 1;
                player.score
            };
            s.broadcast(
                &ServerMessage::Taken {
                    coin,
                    by: id,
                    score,
                },
                None,
            );

            // Keep the field full.
            let (new_id, x, y) = s.spawn_coin();
            s.broadcast(&ServerMessage::Coin { coin: new_id, x, y }, None);

            if score >= TARGET_SCORE {
                s.winner = Some(id);
                s.broadcast(&ServerMessage::Win { id }, None);
                println!("[{id}] wins with {score} coins");
            }
        }
    }
}

fn cleanup(server: &Shared, id: usize) {
    let mut s = server.lock().unwrap();
    s.players.remove(&id);
    s.broadcast(&ServerMessage::Bye { id }, None);
    let count = s.players.len();
    println!("[{id}] disconnected  (total: {count})");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_starts_with_a_full_coin_field() {
        let s = Server::new(12345);
        assert_eq!(s.coins.len(), COIN_COUNT);
    }

    #[test]
    fn coins_spawn_within_field_bounds() {
        let mut s = Server::new(0xABCDEF);
        for _ in 0..1000 {
            let (_, x, y) = s.spawn_coin();
            assert!(
                (FIELD_MIN_X..FIELD_MAX_X).contains(&x),
                "x out of bounds: {x}"
            );
            assert!(
                (FIELD_MIN_Y..FIELD_MAX_Y).contains(&y),
                "y out of bounds: {y}"
            );
        }
    }

    #[test]
    fn coin_ids_are_unique_and_monotonic() {
        let mut s = Server::new(7);
        let a = s.spawn_coin().0;
        let b = s.spawn_coin().0;
        assert!(b > a, "coin ids must be monotonic: {a} then {b}");
    }

    #[test]
    fn client_messages_parse() {
        let pos: ClientMessage =
            serde_json::from_str(r#"{"type":"pos","x":1.5,"y":-2.0}"#).unwrap();
        assert!(matches!(pos, ClientMessage::Position { x, y } if x == 1.5 && y == -2.0));
        let grab: ClientMessage = serde_json::from_str(r#"{"type":"grab","coin":42}"#).unwrap();
        assert!(matches!(grab, ClientMessage::Grab { coin: 42 }));
    }

    #[test]
    fn server_messages_serialize_with_type_tags() {
        let taken = serde_json::to_string(&ServerMessage::Taken {
            coin: 3,
            by: 7,
            score: 2,
        })
        .unwrap();
        assert_eq!(taken, r#"{"type":"taken","coin":3,"by":7,"score":2}"#);
    }
}
