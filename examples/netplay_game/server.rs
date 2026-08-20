//! Netplay Salvage — the authoritative server.
//!
//! ```text
//! # Terminal 1
//! cargo run --example netplay_server
//!
//! # Terminal 2 (and 3, and 4 — it is a multiplayer world)
//! cargo run --example netplay_game
//! ```
//!
//! This is the *authority*: it owns every position that matters and every decision that can be
//! contested. Clients predict, interpolate and ask; none of that is believed here.
//!
//! # What "authoritative" means concretely, in three places
//!
//! - **Ships.** A client's ship moves only when [`Server::apply_input`] applies its input, using
//!   the same [`step_position`] the client predicted with. The client's own idea of where it is
//!   never crosses the wire — only its *inputs* do — so a client that edits its position edits
//!   nothing.
//! - **Pickups.** A claim is a request. [`Server::try_claim`] is first-come-first-served and
//!   validates distance against the server's own positions, then the removal is announced to
//!   everyone as [`ServerMsg::Taken`]. Two clients touching one pickup on the same frame is the
//!   case that matters, and exactly one of them scores.
//! - **The set of entities you can see at all.** [`Server::entities_within`] tailors every
//!   snapshot to that client's AOI, so a client cannot learn about what it should not see by
//!   reading packets it was never sent.
//!
//! # Removal-by-omission
//!
//! The server never tells a client an entity *left* its AOI. It simply stops including it. That is
//! what makes interest management scale — the cost of a departure is zero bytes — and it is why
//! the client needs a last-seen timeout rather than a delete message. There is deliberately no
//! `Despawn` variant in the protocol; adding one would make the client's eviction path dead code
//! and quietly delete the property this game exists to demonstrate.
//!
//! Protocol: see `protocol.rs` (shared with the client).
//!
//! NATIVE_ONLY: it is a TCP server — tungstenite and std::net are native-only by construction
//!
//! (That line is read by `scripts/build_wasm_examples.sh`, which checks it **both ways**: an
//! undeclared wasm failure fails, and a declaration on something that does build also fails. This
//! one is not a limitation to lift — a listening socket is not a thing a browser tab can be. The
//! *client* has no such line and does build for the web, which is the half that matters.)

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::net::TcpListener;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tungstenite::{accept, Message};

#[path = "protocol.rs"]
mod protocol;
use protocol::*;

/// Dependency-free xorshift64 RNG — only used to scatter spawns and velocities.
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

/// One salvage pickup. Its index in `Server.pickups` is its stable wire `id`.
///
/// A taken pickup is **kept** in the vector with `taken = true` rather than removed, so ids stay
/// stable for the life of the world. Compacting the vector would renumber every pickup after the
/// one that was taken, and a client holding an in-flight snapshot would then apply positions to
/// the wrong objects — a bug that only appears under contention, which is the worst kind.
struct Pickup {
    x: f32,
    y: f32,
    taken: bool,
}

/// A drifting hazard. Server-owned, never claimable, and the reason clients need interpolation.
struct Hazard {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

/// A connected client's ship plus the bookkeeping its snapshots need.
struct Player {
    x: f32,
    y: f32,
    /// AOI radius this client asked for, already clamped.
    r: f32,
    /// The last input seq applied. Echoed as `Snap.ack`; reconciliation replays from here.
    ack: u32,
    score: u32,
    sender: mpsc::Sender<Message>,
}

struct Server {
    pickups: Vec<Pickup>,
    hazards: Vec<Hazard>,
    players: HashMap<u32, Player>,
    next_player_id: u32,
    tick: u32,
}

impl Server {
    fn new(seed: u64) -> Self {
        let mut rng = Rng(seed | 1); // xorshift must never be seeded with 0

        let mut pickups = Vec::with_capacity(PICKUP_COUNT as usize);
        for _ in 0..PICKUP_COUNT {
            pickups.push(Pickup {
                x: rng.range(PICKUP_RADIUS, WORLD_W - PICKUP_RADIUS),
                y: rng.range(PICKUP_RADIUS, WORLD_H - PICKUP_RADIUS),
                taken: false,
            });
        }

        let mut hazards = Vec::with_capacity(HAZARD_COUNT as usize);
        for _ in 0..HAZARD_COUNT {
            let speed = rng.range(HAZARD_MIN_SPEED, HAZARD_MAX_SPEED);
            let heading = rng.range(0.0, TAU);
            hazards.push(Hazard {
                x: rng.range(HAZARD_RADIUS, WORLD_W - HAZARD_RADIUS),
                y: rng.range(HAZARD_RADIUS, WORLD_H - HAZARD_RADIUS),
                vx: speed * heading.cos(),
                vy: speed * heading.sin(),
            });
        }

        Self {
            pickups,
            hazards,
            players: HashMap::new(),
            next_player_id: 1,
            tick: 0,
        }
    }

    /// Advances every hazard one fixed step: drift + wall-bounce. Players are **not** stepped here
    /// — they advance per input, in [`apply_input`](Self::apply_input). Pure (no networking), so
    /// it is unit-testable.
    fn step(&mut self) {
        for h in self.hazards.iter_mut() {
            h.x += h.vx * FIXED_DT;
            h.y += h.vy * FIXED_DT;
            // Reflect off the walls, snapping back inside so a hazard cannot tunnel out.
            if h.x < HAZARD_RADIUS {
                h.x = HAZARD_RADIUS;
                h.vx = h.vx.abs();
            } else if h.x > WORLD_W - HAZARD_RADIUS {
                h.x = WORLD_W - HAZARD_RADIUS;
                h.vx = -h.vx.abs();
            }
            if h.y < HAZARD_RADIUS {
                h.y = HAZARD_RADIUS;
                h.vy = h.vy.abs();
            } else if h.y > WORLD_H - HAZARD_RADIUS {
                h.y = WORLD_H - HAZARD_RADIUS;
                h.vy = -h.vy.abs();
            }
        }
        self.tick = self.tick.wrapping_add(1);
    }

    /// Applies one client input: the ship advances by exactly [`step_position`], and `seq` becomes
    /// the ack the next snapshot carries.
    ///
    /// Out-of-order or replayed inputs are ignored (`seq <= ack`). Without that guard a client
    /// could resend an old input to move twice, and — more mundanely — a duplicated packet would
    /// desync the very reconciliation this ack exists to drive.
    fn apply_input(&mut self, id: u32, seq: u32, mx: f32, my: f32, r: f32) {
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        if seq <= p.ack {
            return;
        }
        let (nx, ny) = step_position(p.x, p.y, mx, my);
        p.x = nx;
        p.y = ny;
        p.ack = seq;
        p.r = r.clamp(AOI_RADIUS_MIN, AOI_RADIUS_MAX);
    }

    /// Resolves a claim on pickup `pid` by player `id`. Returns `(pickup_id, claimer, new_score)`
    /// when the claim is granted, `None` when it is refused.
    ///
    /// Refused for three reasons, and all three matter:
    /// - the pickup is already taken — **first claim wins**. This is the guard whose removal left
    ///   two clients' scoreboards *agreeing* on 1-1 in the deleted `coin_race`, and which was
    ///   caught only by counting 2 points against 1 pickup.
    /// - the id is out of range — a malformed or hostile client.
    /// - the ship is nowhere near it. The margin is generous ([`CLAIM_SLACK`]) because the client
    ///   legitimately acts on an interpolated position up to [`INTERP_DELAY`] old; it is not a
    ///   trust boundary so much as a sanity bound, and it is the difference between "latency" and
    ///   "claiming the whole map from spawn".
    fn try_claim(&mut self, id: u32, pid: u32) -> Option<(u32, u32, u32)> {
        let (px, py) = {
            let p = self.players.get(&id)?;
            (p.x, p.y)
        };
        let pickup = self.pickups.get(pid as usize)?;
        if pickup.taken {
            return None;
        }
        let (dx, dy) = (pickup.x - px, pickup.y - py);
        let reach = CLAIM_RADIUS + CLAIM_SLACK;
        if dx * dx + dy * dy > reach * reach {
            return None;
        }
        self.pickups[pid as usize].taken = true;
        let p = self.players.get_mut(&id)?;
        p.score += 1;
        Some((pid, id, p.score))
    }

    /// The AOI filter: every live entity within `r` of `(cx, cy)`, by squared distance (no sqrt).
    ///
    /// `viewer` is excluded from the `Player` results — a client renders its own ship from its
    /// *predicted* position, and streaming it back would give that ship two sources of truth: the
    /// prediction and a 12 Hz interpolated copy, visibly fighting each other.
    ///
    /// Taken pickups are omitted. That is not a second removal mechanism duplicating
    /// [`ServerMsg::Taken`] — it is the same one. `Taken` tells clients *why* a pickup vanished so
    /// they can score it; omission is what keeps it gone for a client that was out of range when
    /// it happened and never received that message at all.
    fn entities_within(&self, cx: f32, cy: f32, r: f32, viewer: u32) -> Vec<EntityState> {
        let r2 = r * r;
        let near = |x: f32, y: f32| {
            let (dx, dy) = (x - cx, y - cy);
            dx * dx + dy * dy <= r2
        };
        let mut out = Vec::new();

        for (i, p) in self.pickups.iter().enumerate() {
            if !p.taken && near(p.x, p.y) {
                out.push(EntityState {
                    id: i as u32,
                    kind: Kind::Pickup,
                    x: p.x,
                    y: p.y,
                });
            }
        }
        for (i, h) in self.hazards.iter().enumerate() {
            if near(h.x, h.y) {
                out.push(EntityState {
                    id: i as u32,
                    kind: Kind::Hazard,
                    x: h.x,
                    y: h.y,
                });
            }
        }
        for (&id, p) in self.players.iter() {
            if id != viewer && near(p.x, p.y) {
                out.push(EntityState {
                    id,
                    kind: Kind::Player,
                    x: p.x,
                    y: p.y,
                });
            }
        }
        out
    }

    /// Sends each client its own AOI-filtered snapshot, carrying that client's authoritative ship
    /// position and ack. Unlike a broadcast, **both** the entity set and the header differ per
    /// client.
    fn send_snapshots(&self) {
        for (&id, p) in self.players.iter() {
            let snap = ServerMsg::Snap {
                tick: self.tick,
                ack: p.ack,
                x: p.x,
                y: p.y,
                entities: self.entities_within(p.x, p.y, p.r, id),
            };
            let Ok(text) = ron::to_string(&snap) else {
                continue;
            };
            let _ = p.sender.send(Message::Text(text.into()));
        }
    }

    /// Sends one message to every client. Used for the events every client must agree on —
    /// `Taken` and `Left` — regardless of AOI: a pickup taken across the map must still disappear
    /// from a client that is holding a stale copy of it.
    fn broadcast(&self, msg: &ServerMsg) {
        let Ok(text) = ron::to_string(msg) else {
            return;
        };
        for p in self.players.values() {
            let _ = p.sender.send(Message::Text(text.clone().into()));
        }
    }

    fn live_pickups(&self) -> usize {
        self.pickups.iter().filter(|p| !p.taken).count()
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

    println!("netplay_server: authoritative salvage world on ws://{addr}");
    println!(
        "  {PICKUP_COUNT} pickups + {HAZARD_COUNT} hazards in a {WORLD_W:.0}x{WORLD_H:.0} world · \
         {SNAPSHOT_HZ} Hz per-client AOI snapshots · run `netplay_game`"
    );
    println!();

    // Fixed-tick simulation thread: step at 60 Hz, snapshot at SNAPSHOT_HZ.
    {
        let server = server.clone();
        // Snapshot every Nth tick (the tick rate is an integer multiple of SNAPSHOT_HZ).
        let snap_every = (1.0 / FIXED_DT / SNAPSHOT_HZ as f32).round().max(1.0) as u32;
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs_f32(FIXED_DT));
            let mut s = server.lock().unwrap();
            s.step();
            if s.tick.is_multiple_of(snap_every) {
                s.send_snapshots();
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

    // Register the player and build its Welcome. Built under the lock, sent outside it — the 60 Hz
    // tick thread needs the lock 60×/s, so a network send must never happen while holding it.
    let (id, welcome) = {
        let mut s = server.lock().unwrap();
        let id = s.next_player_id;
        s.next_player_id += 1;
        // Spawn in the middle. Deterministic on purpose: the selftest's contested-claim check puts
        // two clients on one pickup, and a random spawn would make "both are in reach" a coin flip.
        s.players.insert(
            id,
            Player {
                x: WORLD_W * 0.5,
                y: WORLD_H * 0.5,
                r: AOI_RADIUS_DEFAULT,
                ack: 0,
                score: 0,
                sender: tx,
            },
        );
        let welcome = ServerMsg::Welcome {
            player_id: id,
            world_w: WORLD_W,
            world_h: WORLD_H,
            pickups_total: PICKUP_COUNT,
        };
        let count = s.players.len();
        println!("[{id}] connected from {peer:?}  (players: {count})");
        (id, welcome)
    };

    let Ok(text) = ron::to_string(&welcome) else {
        cleanup(&server, id);
        return;
    };
    if ws.send(Message::Text(text.into())).is_err() {
        cleanup(&server, id);
        return;
    }

    // Forward queued snapshots and read the client's inputs / claims until the socket closes.
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
                if text.len() > MAX_MESSAGE_BYTES {
                    continue;
                }
                let Ok(msg) = ron::from_str::<ClientMsg>(&text) else {
                    continue;
                };
                match msg {
                    ClientMsg::Join => {}
                    ClientMsg::Input { seq, mx, my, r } => {
                        let mut s = server.lock().unwrap();
                        s.apply_input(id, seq, mx, my, r);
                    }
                    ClientMsg::Claim { id: pid } => {
                        // Resolve and announce under **one** lock. Two scopes would leave a window
                        // where the pickup is already taken but nobody has been told, and a second
                        // claim arriving in it would be refused with no `Taken` yet in flight —
                        // the loser's pickup would sit there until the AOI omission caught it.
                        let mut s = server.lock().unwrap();
                        if let Some((pickup, by, score)) = s.try_claim(id, pid) {
                            s.broadcast(&ServerMsg::Taken {
                                id: pickup,
                                by,
                                score,
                            });
                            println!(
                                "[{by}] took pickup {pickup} (score {score}, {} left)",
                                s.live_pickups()
                            );
                        }
                    }
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

fn cleanup(server: &Shared, id: u32) {
    let mut s = server.lock().unwrap();
    s.players.remove(&id);
    println!("[{id}] disconnected  (players: {})", s.players.len());
    // Tell everyone the ship is gone. AOI eviction would eventually do it for clients that could
    // see them, but a player who disconnects outside everyone's radius would otherwise stay
    // tracked on any client still holding a stale copy. The removal happens first, so the leaver
    // is not sent its own `Left`.
    s.broadcast(&ServerMsg::Left { player_id: id });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A server with one player parked at a known spot, for the claim tests.
    fn with_player(seed: u64, x: f32, y: f32) -> (Server, u32, mpsc::Receiver<Message>) {
        let mut s = Server::new(seed);
        let (tx, rx) = mpsc::channel();
        let id = s.next_player_id;
        s.next_player_id += 1;
        s.players.insert(
            id,
            Player {
                x,
                y,
                r: AOI_RADIUS_DEFAULT,
                ack: 0,
                score: 0,
                sender: tx,
            },
        );
        (s, id, rx)
    }

    #[test]
    fn new_server_spawns_the_full_field_inside_the_world() {
        let s = Server::new(1);
        assert_eq!(s.pickups.len(), PICKUP_COUNT as usize);
        assert_eq!(s.hazards.len(), HAZARD_COUNT as usize);
        for p in &s.pickups {
            assert!(p.x >= PICKUP_RADIUS && p.x <= WORLD_W - PICKUP_RADIUS);
            assert!(p.y >= PICKUP_RADIUS && p.y <= WORLD_H - PICKUP_RADIUS);
            assert!(!p.taken);
        }
        for h in &s.hazards {
            assert!(h.x >= HAZARD_RADIUS && h.x <= WORLD_W - HAZARD_RADIUS);
            assert!(h.y >= HAZARD_RADIUS && h.y <= WORLD_H - HAZARD_RADIUS);
        }
    }

    #[test]
    fn hazards_stay_inside_the_world_after_many_steps() {
        let mut s = Server::new(2);
        for _ in 0..2000 {
            s.step();
        }
        for h in &s.hazards {
            assert!(
                h.x >= HAZARD_RADIUS - 1e-3 && h.x <= WORLD_W - HAZARD_RADIUS + 1e-3,
                "hazard x={} escaped the world",
                h.x
            );
            assert!(
                h.y >= HAZARD_RADIUS - 1e-3 && h.y <= WORLD_H - HAZARD_RADIUS + 1e-3,
                "hazard y={} escaped the world",
                h.y
            );
        }
    }

    #[test]
    fn stepping_does_not_move_players() {
        // Player motion is per-input, never per tick. If `step` moved ships, a client that stopped
        // sending input would drift and reconciliation would fight a phantom.
        let (mut s, id, _rx) = with_player(3, 100.0, 200.0);
        for _ in 0..600 {
            s.step();
        }
        let p = &s.players[&id];
        assert_eq!((p.x, p.y), (100.0, 200.0));
    }

    #[test]
    fn apply_input_matches_the_shared_step_and_advances_the_ack() {
        let (mut s, id, _rx) = with_player(4, 500.0, 500.0);
        s.apply_input(id, 1, 1.0, 0.0, AOI_RADIUS_DEFAULT);
        let want = step_position(500.0, 500.0, 1.0, 0.0);
        let p = &s.players[&id];
        assert!((p.x - want.0).abs() < 1e-4 && (p.y - want.1).abs() < 1e-4);
        assert_eq!(p.ack, 1);
    }

    #[test]
    fn a_replayed_input_is_ignored() {
        let (mut s, id, _rx) = with_player(5, 500.0, 500.0);
        s.apply_input(id, 1, 1.0, 0.0, AOI_RADIUS_DEFAULT);
        let after_first = (s.players[&id].x, s.players[&id].y);
        s.apply_input(id, 1, 1.0, 0.0, AOI_RADIUS_DEFAULT); // same seq again
        assert_eq!((s.players[&id].x, s.players[&id].y), after_first);
        s.apply_input(id, 0, 1.0, 0.0, AOI_RADIUS_DEFAULT); // an older one
        assert_eq!((s.players[&id].x, s.players[&id].y), after_first);
    }

    #[test]
    fn apply_input_clamps_the_requested_aoi_radius() {
        let (mut s, id, _rx) = with_player(6, 500.0, 500.0);
        s.apply_input(id, 1, 0.0, 0.0, 1e6);
        assert_eq!(s.players[&id].r, AOI_RADIUS_MAX);
        s.apply_input(id, 2, 0.0, 0.0, -1.0);
        assert_eq!(s.players[&id].r, AOI_RADIUS_MIN);
    }

    #[test]
    fn first_claim_wins_and_the_second_gets_nothing() {
        // The failure this pins is the *flattering* one: with the guard removed both players score
        // and both scoreboards agree, so only counting points against pickups reveals it.
        let mut s = Server::new(7);
        let (tx_a, _ra) = mpsc::channel();
        let (tx_b, _rb) = mpsc::channel();
        let target = 0u32;
        let (px, py) = (s.pickups[target as usize].x, s.pickups[target as usize].y);
        for (id, tx) in [(1u32, tx_a), (2u32, tx_b)] {
            s.players.insert(
                id,
                Player {
                    x: px,
                    y: py,
                    r: AOI_RADIUS_DEFAULT,
                    ack: 0,
                    score: 0,
                    sender: tx,
                },
            );
        }
        let first = s.try_claim(1, target);
        let second = s.try_claim(2, target);
        assert!(matches!(first, Some((0, 1, 1))), "first claim must win");
        assert!(second.is_none(), "the second claim on one pickup must fail");
        assert_eq!(s.players[&1].score, 1);
        assert_eq!(s.players[&2].score, 0);
        // The invariant the live check asserts, in miniature: points awarded == pickups removed.
        let points: u32 = s.players.values().map(|p| p.score).sum();
        assert_eq!(points as usize, PICKUP_COUNT as usize - s.live_pickups());
    }

    #[test]
    fn a_claim_from_across_the_world_is_refused() {
        let mut s = Server::new(8);
        let (tx, _rx) = mpsc::channel();
        s.players.insert(
            1,
            Player {
                x: 0.0,
                y: 0.0,
                r: AOI_RADIUS_DEFAULT,
                ack: 0,
                score: 0,
                sender: tx,
            },
        );
        // Find a pickup that is genuinely far from the corner, so the test is not seed-dependent.
        let far = s
            .pickups
            .iter()
            .position(|p| p.x * p.x + p.y * p.y > (WORLD_W * 0.5).powi(2))
            .expect("some pickup is far from the origin");
        assert!(s.try_claim(1, far as u32).is_none());
        assert!(!s.pickups[far].taken);
        assert_eq!(s.players[&1].score, 0);
    }

    #[test]
    fn a_claim_on_a_nonexistent_pickup_is_refused() {
        let (mut s, id, _rx) = with_player(9, 500.0, 500.0);
        assert!(s.try_claim(id, PICKUP_COUNT + 100).is_none());
    }

    #[test]
    fn aoi_filter_includes_only_entities_within_radius_and_is_monotonic() {
        let (s, id, _rx) = with_player(10, WORLD_W * 0.5, WORLD_H * 0.5);
        let (cx, cy) = (WORLD_W * 0.5, WORLD_H * 0.5);
        let small = s.entities_within(cx, cy, 400.0, id);
        for e in &small {
            let (dx, dy) = (e.x - cx, e.y - cy);
            assert!(dx * dx + dy * dy <= 400.0 * 400.0, "an entity leaked in");
        }
        // Monotonic in r — the live-resize invariant: a larger radius is a superset.
        let big = s.entities_within(cx, cy, 900.0, id);
        assert!(big.len() >= small.len());
        let small_ids: std::collections::HashSet<NetId> =
            small.iter().map(|e| e.net_id()).collect();
        let big_ids: std::collections::HashSet<NetId> = big.iter().map(|e| e.net_id()).collect();
        assert!(small_ids.is_subset(&big_ids));
    }

    #[test]
    fn the_full_world_radius_streams_everything_but_the_viewer() {
        let (mut s, id, _rx) = with_player(11, WORLD_W * 0.5, WORLD_H * 0.5);
        let (tx, _rx2) = mpsc::channel();
        s.players.insert(
            99,
            Player {
                x: WORLD_W * 0.5,
                y: WORLD_H * 0.5,
                r: AOI_RADIUS_DEFAULT,
                ack: 0,
                score: 0,
                sender: tx,
            },
        );
        let all = s.entities_within(WORLD_W * 0.5, WORLD_H * 0.5, WORLD_W + WORLD_H, id);
        // Every pickup + every hazard + the *other* player, and not the viewer.
        assert_eq!(
            all.len(),
            (PICKUP_COUNT + HAZARD_COUNT) as usize + 1,
            "expected the whole field plus one remote ship"
        );
        assert!(
            !all.iter().any(|e| e.net_id() == (Kind::Player, id)),
            "the viewer must not be streamed its own ship — it renders that from prediction"
        );
        assert!(all.iter().any(|e| e.net_id() == (Kind::Player, 99)));
    }

    #[test]
    fn a_taken_pickup_stops_being_streamed() {
        let mut s = Server::new(12);
        let (tx, _rx) = mpsc::channel();
        let target = 0u32;
        let (px, py) = (s.pickups[0].x, s.pickups[0].y);
        s.players.insert(
            1,
            Player {
                x: px,
                y: py,
                r: AOI_RADIUS_DEFAULT,
                ack: 0,
                score: 0,
                sender: tx,
            },
        );
        let before = s.entities_within(px, py, WORLD_W + WORLD_H, 1);
        assert!(before.iter().any(|e| e.net_id() == (Kind::Pickup, target)));
        s.try_claim(1, target).expect("the claim should be granted");
        let after = s.entities_within(px, py, WORLD_W + WORLD_H, 1);
        assert!(
            !after.iter().any(|e| e.net_id() == (Kind::Pickup, target)),
            "a taken pickup must not be streamed — a client that missed the Taken message would \
             otherwise keep it forever"
        );
    }

    #[test]
    fn taking_a_pickup_does_not_renumber_the_others() {
        // Ids are indices into a vector that is never compacted; if a take renumbered them, an
        // in-flight snapshot would land on the wrong objects.
        let mut s = Server::new(13);
        let (tx, _rx) = mpsc::channel();
        let (px, py) = (s.pickups[0].x, s.pickups[0].y);
        s.players.insert(
            1,
            Player {
                x: px,
                y: py,
                r: AOI_RADIUS_DEFAULT,
                ack: 0,
                score: 0,
                sender: tx,
            },
        );
        let before: Vec<(f32, f32)> = s.pickups.iter().map(|p| (p.x, p.y)).collect();
        s.try_claim(1, 0).expect("granted");
        let after: Vec<(f32, f32)> = s.pickups.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(before, after, "pickup positions must not shift on a take");
        assert_eq!(s.pickups.len(), PICKUP_COUNT as usize);
    }
}
