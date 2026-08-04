//! Client-prediction shooter — authoritative fixed-tick server.
//!
//! ```text
//! # Terminal 1
//! cargo run --example predict_shooter_server
//!
//! # Terminals 2, 3, ...
//! cargo run --example predict_shooter
//! ```
//!
//! Unlike `coin_race_server` (event-driven, relays positions a client computes itself), this
//! server is the **authority over movement**: clients send seq-numbered input commands, the server
//! integrates them on a fixed tick, and broadcasts snapshots whose per-client `ack` tells each
//! client which of its inputs have been applied. That `ack` is what makes client-side prediction +
//! reconciliation possible. Bullets are server-spawned on a `fire` input and server-integrated;
//! the client interpolates them like any other remote entity.
//!
//! Protocol: see `protocol.rs` (shared with the client).

use std::collections::{HashMap, VecDeque};
use std::net::TcpListener;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tungstenite::{accept, Message};

#[path = "protocol.rs"]
mod protocol;
use protocol::*;

/// Dependency-free xorshift64 RNG — only used to scatter spawn positions.
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

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let unit = (self.next_u64() >> 11) as f32 / (1u64 << 53) as f32;
        lo + unit * (hi - lo)
    }
}

struct InputCmd {
    seq: u32,
    mx: f32,
    my: f32,
    fire: bool,
}

struct Player {
    x: f32,
    y: f32,
    /// Last non-zero movement direction (unit), used as the firing direction.
    last_dir: (f32, f32),
    /// Highest input `seq` applied so far — echoed to the client as `ack`.
    last_seq: u32,
    fire_cd: f32,
    inputs: VecDeque<InputCmd>,
    sender: mpsc::Sender<Message>,
}

struct Bullet {
    id: usize,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
}

struct Server {
    players: HashMap<usize, Player>,
    bullets: Vec<Bullet>,
    next_player_id: usize,
    next_bullet_id: usize,
    tick: u32,
    rng: Rng,
}

impl Server {
    fn new(seed: u64) -> Self {
        Self {
            players: HashMap::new(),
            bullets: Vec::new(),
            next_player_id: 1,
            next_bullet_id: 1,
            tick: 0,
            rng: Rng(seed | 1), // xorshift must never be seeded with 0
        }
    }

    /// Advances the authoritative simulation by one fixed step: applies each player's queued
    /// inputs (recording `last_seq`), spawns bullets on `fire`, then integrates and culls bullets.
    /// Pure (no networking) so it is unit-testable.
    fn step(&mut self) {
        let mut spawned: Vec<(f32, f32, f32, f32)> = Vec::new();
        for player in self.players.values_mut() {
            player.fire_cd = (player.fire_cd - FIXED_DT).max(0.0);
            while let Some(cmd) = player.inputs.pop_front() {
                let (nx, ny) = step_position(player.x, player.y, cmd.mx, cmd.my);
                player.x = nx;
                player.y = ny;
                // WebSocket delivery is ordered, so the last queued command has the highest seq;
                // assign directly. (Over an unordered transport, guard with `seq > last_seq`.)
                player.last_seq = cmd.seq;
                if cmd.mx != 0.0 || cmd.my != 0.0 {
                    let inv = 1.0 / (cmd.mx * cmd.mx + cmd.my * cmd.my).sqrt();
                    player.last_dir = (cmd.mx * inv, cmd.my * inv);
                }
                if cmd.fire && player.fire_cd <= 0.0 {
                    let (dx, dy) = player.last_dir;
                    spawned.push((player.x, player.y, dx * BULLET_SPEED, dy * BULLET_SPEED));
                    player.fire_cd = FIRE_COOLDOWN;
                }
            }
        }
        for (x, y, vx, vy) in spawned {
            let id = self.next_bullet_id;
            self.next_bullet_id += 1;
            self.bullets.push(Bullet {
                id,
                x,
                y,
                vx,
                vy,
                life: BULLET_LIFE,
            });
        }
        for b in self.bullets.iter_mut() {
            b.x += b.vx * FIXED_DT;
            b.y += b.vy * FIXED_DT;
            b.life -= FIXED_DT;
        }
        self.bullets.retain(|b| {
            b.life > 0.0
                && b.x > -50.0
                && b.x < FIELD_W + 50.0
                && b.y > -50.0
                && b.y < FIELD_H + 50.0
        });
        self.tick = self.tick.wrapping_add(1);
    }

    fn player_states(&self) -> Vec<PlayerState> {
        self.players
            .iter()
            .map(|(&id, p)| PlayerState { id, x: p.x, y: p.y })
            .collect()
    }

    fn bullet_states(&self) -> Vec<BulletState> {
        self.bullets
            .iter()
            .map(|b| BulletState {
                id: b.id,
                x: b.x,
                y: b.y,
            })
            .collect()
    }

    /// Sends a per-client snapshot (shared players/bullets, but each client's own `ack`).
    fn broadcast_snapshot(&self) {
        let players = self.player_states();
        let bullets = self.bullet_states();
        for player in self.players.values() {
            let snap = ServerMsg::Snap {
                tick: self.tick,
                players: players.clone(),
                bullets: bullets.clone(),
                ack: player.last_seq,
            };
            if let Ok(text) = serde_json::to_string(&snap) {
                let _ = player.sender.send(Message::Text(text.into()));
            }
        }
    }

    fn broadcast(&self, msg: &ServerMsg) {
        let Ok(text) = serde_json::to_string(msg) else {
            return;
        };
        let frame = Message::Text(text.into());
        for player in self.players.values() {
            let _ = player.sender.send(frame.clone());
        }
    }
}

type Shared = Arc<Mutex<Server>>;

fn main() {
    let addr = server_addr();
    let listener = TcpListener::bind(&addr).expect("bind failed");
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15);
    let server: Shared = Arc::new(Mutex::new(Server::new(seed)));

    println!("predict_shooter_server: authoritative fixed-tick server on ws://{addr}");
    println!(
        "  {SNAPSHOT_HZ} Hz snapshots · {MOVE_SPEED} px/s · run `predict_shooter` in 2+ windows"
    );
    println!();

    // Fixed-tick simulation thread: step at 60 Hz, snapshot at SNAPSHOT_HZ.
    {
        let server = server.clone();
        // Snapshot every Nth tick. Assumes the tick rate (1/FIXED_DT) is an integer multiple of
        // SNAPSHOT_HZ; for non-integer ratios the period rounds to the nearest whole tick.
        let snap_every = (1.0 / FIXED_DT / SNAPSHOT_HZ as f32).round().max(1.0) as u32;
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs_f32(FIXED_DT));
            let mut s = server.lock().unwrap();
            s.step();
            if s.tick.is_multiple_of(snap_every) {
                s.broadcast_snapshot();
            }
        });
    }

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
    ws.get_mut()
        .set_read_timeout(Some(Duration::from_millis(5)))
        .ok();

    let (tx, rx) = mpsc::channel::<Message>();

    let id = {
        let mut s = server.lock().unwrap();
        let id = s.next_player_id;
        s.next_player_id += 1;
        let x = s.rng.range(PLAYER_HALF, FIELD_W - PLAYER_HALF);
        let y = s.rng.range(PLAYER_HALF, FIELD_H - PLAYER_HALF);
        s.players.insert(
            id,
            Player {
                x,
                y,
                last_dir: (0.0, -1.0),
                last_seq: 0,
                fire_cd: 0.0,
                inputs: VecDeque::new(),
                sender: tx,
            },
        );
        let count = s.players.len();
        println!("[{id}] connected from {peer:?}  (total: {count})");
        id
    };

    // Send the welcome OUTSIDE the lock: unlike coin_race (event-driven), this server has a fixed
    // tick thread that needs the lock 60×/s, so blocking a network send under the lock would stall
    // the whole simulation on one slow client. Welcome carries only the id, so it can't race state.
    let Ok(text) = serde_json::to_string(&ServerMsg::Welcome { id }) else {
        cleanup(&server, id);
        return;
    };
    if ws.send(Message::Text(text.into())).is_err() {
        cleanup(&server, id);
        return;
    }

    'main: loop {
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

        match ws.read() {
            Ok(Message::Text(text)) => {
                if text.len() > MAX_JSON_MESSAGE_BYTES {
                    eprintln!("[{id}] dropped oversized message: {} bytes", text.len());
                    continue;
                }
                match serde_json::from_str::<ClientMsg>(&text) {
                    Ok(ClientMsg::Input { seq, mx, my, fire }) => {
                        let mut s = server.lock().unwrap();
                        if let Some(p) = s.players.get_mut(&id) {
                            p.inputs.push_back(InputCmd { seq, mx, my, fire });
                        }
                    }
                    Err(err) => eprintln!("[{id}] invalid JSON: {err}"),
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }

    cleanup(&server, id);
}

fn cleanup(server: &Shared, id: usize) {
    let mut s = server.lock().unwrap();
    s.players.remove(&id);
    s.broadcast(&ServerMsg::Bye { id });
    let count = s.players.len();
    println!("[{id}] disconnected  (total: {count})");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_player(s: &mut Server, x: f32, y: f32) -> usize {
        let (tx, _rx) = mpsc::channel();
        let id = s.next_player_id;
        s.next_player_id += 1;
        s.players.insert(
            id,
            Player {
                x,
                y,
                last_dir: (0.0, -1.0),
                last_seq: 0,
                fire_cd: 0.0,
                inputs: VecDeque::new(),
                sender: tx,
            },
        );
        id
    }

    #[test]
    fn step_applies_input_and_advances_ack() {
        let mut s = Server::new(1);
        let id = add_player(&mut s, 400.0, 300.0);
        s.players.get_mut(&id).unwrap().inputs.push_back(InputCmd {
            seq: 1,
            mx: 1.0,
            my: 0.0,
            fire: false,
        });
        s.step();
        let p = &s.players[&id];
        assert_eq!(p.last_seq, 1, "ack should advance to the applied input seq");
        let expected = 400.0 + MOVE_SPEED * FIXED_DT;
        assert!(
            (p.x - expected).abs() < 1e-3,
            "x={} expected={}",
            p.x,
            expected
        );
        assert_eq!(p.y, 300.0);
    }

    #[test]
    fn replaying_inputs_one_per_step_matches_step_position_chain() {
        // The client replays inputs with step_position; the server applies them in step().
        // Both must converge to the same position for reconciliation to work.
        let mut s = Server::new(2);
        let id = add_player(&mut s, 100.0, 100.0);
        let cmds = [(1.0_f32, 0.0_f32), (0.0, 1.0), (1.0, 1.0), (-1.0, 0.0)];
        for (i, &(mx, my)) in cmds.iter().enumerate() {
            s.players.get_mut(&id).unwrap().inputs.push_back(InputCmd {
                seq: i as u32 + 1,
                mx,
                my,
                fire: false,
            });
            s.step();
        }
        let server_pos = (s.players[&id].x, s.players[&id].y);

        let mut p = (100.0_f32, 100.0_f32);
        for &(mx, my) in cmds.iter() {
            p = step_position(p.0, p.1, mx, my);
        }
        assert!((server_pos.0 - p.0).abs() < 1e-3 && (server_pos.1 - p.1).abs() < 1e-3);
    }

    #[test]
    fn fire_spawns_a_bullet_that_expires() {
        let mut s = Server::new(3);
        let id = add_player(&mut s, 400.0, 300.0);
        // Establish a movement direction, then fire.
        s.players.get_mut(&id).unwrap().inputs.push_back(InputCmd {
            seq: 1,
            mx: 1.0,
            my: 0.0,
            fire: true,
        });
        s.step();
        assert_eq!(s.bullets.len(), 1, "a fire input should spawn one bullet");

        // After its lifetime, the bullet is culled.
        let steps = (BULLET_LIFE / FIXED_DT).ceil() as usize + 2;
        for _ in 0..steps {
            s.step();
        }
        assert!(
            s.bullets.is_empty(),
            "bullet should expire after its lifetime"
        );
    }

    #[test]
    fn fire_is_rate_limited_by_cooldown() {
        let mut s = Server::new(4);
        let id = add_player(&mut s, 400.0, 300.0);
        // Two fire inputs in the same tick → only one bullet (cooldown).
        for seq in 1..=2 {
            s.players.get_mut(&id).unwrap().inputs.push_back(InputCmd {
                seq,
                mx: 0.0,
                my: -1.0,
                fire: true,
            });
        }
        s.step();
        assert_eq!(
            s.bullets.len(),
            1,
            "cooldown should limit to one bullet per window"
        );
    }

    #[test]
    fn protocol_round_trips() {
        let input: ClientMsg =
            serde_json::from_str(r#"{"t":"in","seq":5,"mx":1.0,"my":-1.0,"fire":true}"#).unwrap();
        assert!(matches!(
            input,
            ClientMsg::Input {
                seq: 5,
                fire: true,
                ..
            }
        ));
        let snap = serde_json::to_string(&ServerMsg::Snap {
            tick: 9,
            players: vec![PlayerState {
                id: 1,
                x: 2.0,
                y: 3.0,
            }],
            bullets: vec![],
            ack: 7,
        })
        .unwrap();
        assert!(snap.contains(r#""t":"snap""#) && snap.contains(r#""ack":7"#));
    }

    #[test]
    fn movement_is_clamped_to_the_field() {
        let mut s = Server::new(5);
        let id = add_player(&mut s, FIELD_W - PLAYER_HALF, FIELD_H - PLAYER_HALF);
        // Push hard into the bottom-right corner for many ticks.
        for seq in 1..=120 {
            s.players.get_mut(&id).unwrap().inputs.push_back(InputCmd {
                seq,
                mx: 1.0,
                my: 1.0,
                fire: false,
            });
            s.step();
        }
        let p = &s.players[&id];
        assert!(p.x <= FIELD_W - PLAYER_HALF + 1e-3 && p.y <= FIELD_H - PLAYER_HALF + 1e-3);
        assert!(p.x >= PLAYER_HALF && p.y >= PLAYER_HALF);
    }

    #[test]
    fn snapshot_includes_all_players_with_their_positions() {
        let mut s = Server::new(6);
        let a = add_player(&mut s, 100.0, 100.0);
        let b = add_player(&mut s, 700.0, 500.0);
        let states = s.player_states();
        assert_eq!(states.len(), 2);
        let sa = states.iter().find(|p| p.id == a).unwrap();
        let sb = states.iter().find(|p| p.id == b).unwrap();
        assert_eq!((sa.x, sa.y), (100.0, 100.0));
        assert_eq!((sb.x, sb.y), (700.0, 500.0));
    }
}
