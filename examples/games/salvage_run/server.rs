//! Salvage Run — authoritative AOI-streaming server.
//!
//! ```text
//! # Terminal 1
//! cargo run --example salvage_run_server
//!
//! # Terminal 2
//! cargo run --example salvage_run
//! ```
//!
//! The server owns a large world full of drifting entities of two kinds (slow salvage, roaming
//! drones), steps them on a fixed 60 Hz tick, and — unlike `orbital_dodger`'s broadcast server —
//! sends each client a **different** snapshot: only the entities within that client's
//! area-of-interest (AOI), a radius around the position the client last reported via
//! `ClientMsg::Move`. As a client roams, entities continuously stream in and out of its snapshots.
//! The server never tells the client an entity *left* — it simply stops including it, so the client
//! infers eviction by last-seen timeout. That removal-by-omission is the whole point of this
//! example (interest management), and is what stresses `docs/REMOTE_ENTITIES_DESIGN.md`'s open
//! questions about staleness (#5) and typed entities (#4).
//!
//! Protocol: see `protocol.rs` (shared with the client).

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

/// Dependency-free xorshift64 RNG — only used to scatter entity spawns/velocities.
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

/// Collision / spawn-inset radius for a kind (matches the client's rendered half-extent).
fn kind_radius(kind: Kind) -> f32 {
    match kind {
        Kind::Salvage => SALVAGE_RADIUS,
        Kind::Drone => DRONE_RADIUS,
    }
}

/// One server-authoritative body. Its index in `Server.entities` is its stable wire `id`.
struct Entity {
    kind: Kind,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

/// A connected client: the AOI centre + radius it last reported, plus its outbound channel.
struct Client {
    x: f32,
    y: f32,
    r: f32,
    sender: mpsc::Sender<Message>,
}

struct Server {
    entities: Vec<Entity>,
    clients: HashMap<usize, Client>,
    next_client_id: usize,
    tick: u32,
}

impl Server {
    fn new(seed: u64) -> Self {
        let mut rng = Rng(seed | 1); // xorshift must never be seeded with 0
        let mut entities = Vec::with_capacity(ENTITY_COUNT);
        for i in 0..ENTITY_COUNT {
            let kind = if i < SALVAGE_COUNT {
                Kind::Salvage
            } else {
                Kind::Drone
            };
            let (lo, hi) = match kind {
                Kind::Salvage => (SALVAGE_MIN_SPEED, SALVAGE_MAX_SPEED),
                Kind::Drone => (DRONE_MIN_SPEED, DRONE_MAX_SPEED),
            };
            let speed = rng.range(lo, hi);
            let heading = rng.range(0.0, TAU);
            let r = kind_radius(kind);
            entities.push(Entity {
                kind,
                x: rng.range(r, WORLD_W - r),
                y: rng.range(r, WORLD_H - r),
                vx: speed * heading.cos(),
                vy: speed * heading.sin(),
            });
        }
        Self {
            entities,
            clients: HashMap::new(),
            next_client_id: 1,
            tick: 0,
        }
    }

    /// Advances every entity one fixed step: drift + wall-bounce. Pure (no networking) so it is
    /// unit-testable.
    fn step(&mut self) {
        for e in self.entities.iter_mut() {
            let r = kind_radius(e.kind);
            e.x += e.vx * FIXED_DT;
            e.y += e.vy * FIXED_DT;
            // Reflect off the walls, snapping back inside so an entity can't tunnel out.
            if e.x < r {
                e.x = r;
                e.vx = e.vx.abs();
            } else if e.x > WORLD_W - r {
                e.x = WORLD_W - r;
                e.vx = -e.vx.abs();
            }
            if e.y < r {
                e.y = r;
                e.vy = e.vy.abs();
            } else if e.y > WORLD_H - r {
                e.y = WORLD_H - r;
                e.vy = -e.vy.abs();
            }
        }
        self.tick = self.tick.wrapping_add(1);
    }

    /// The AOI filter: every entity within `r` of `(cx, cy)`, by squared distance (no sqrt). Circular
    /// so the streamed set matches the radius ring the client draws.
    fn entities_within(&self, cx: f32, cy: f32, r: f32) -> Vec<EntityState> {
        let r2 = r * r;
        self.entities
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                let dx = e.x - cx;
                let dy = e.y - cy;
                dx * dx + dy * dy <= r2
            })
            .map(|(id, e)| EntityState {
                id,
                kind: e.kind,
                x: e.x,
                y: e.y,
            })
            .collect()
    }

    /// Send each client its own AOI-filtered snapshot. Unlike a broadcast, the *entity set itself*
    /// differs per client (each sees only what is near it).
    fn broadcast_snapshot(&self) {
        for client in self.clients.values() {
            let snap = ServerMsg::Snap {
                tick: self.tick,
                entities: self.entities_within(client.x, client.y, client.r),
            };
            let Ok(text) = serde_json::to_string(&snap) else {
                continue;
            };
            let _ = client.sender.send(Message::Text(text.into()));
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

    println!("salvage_run_server: AOI-streaming world on ws://{addr}");
    println!(
        "  {ENTITY_COUNT} entities ({SALVAGE_COUNT} salvage + {DRONE_COUNT} drones) in a {WORLD_W:.0}x{WORLD_H:.0} world · {SNAPSHOT_HZ} Hz per-client AOI snapshots · run `salvage_run`"
    );
    println!();

    // Fixed-tick simulation thread: step at 60 Hz, snapshot at SNAPSHOT_HZ.
    {
        let server = server.clone();
        // Snapshot every Nth tick (assumes the tick rate is an integer multiple of SNAPSHOT_HZ).
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

    // Register the client (AOI centre defaults to the world middle until its first Move) and build
    // its Welcome. Cloned/built under the lock, sent outside it — the 60 Hz tick thread needs the
    // lock 60×/s, so we never block a network send while holding it.
    let (id, welcome) = {
        let mut s = server.lock().unwrap();
        let id = s.next_client_id;
        s.next_client_id += 1;
        s.clients.insert(
            id,
            Client {
                x: WORLD_W * 0.5,
                y: WORLD_H * 0.5,
                r: AOI_RADIUS_DEFAULT,
                sender: tx,
            },
        );
        let welcome = ServerMsg::Welcome {
            world_w: WORLD_W,
            world_h: WORLD_H,
            total: ENTITY_COUNT,
        };
        let count = s.clients.len();
        println!("[{id}] connected from {peer:?}  (total: {count})");
        (id, welcome)
    };

    let Ok(text) = serde_json::to_string(&welcome) else {
        cleanup(&server, id);
        return;
    };
    if ws.send(Message::Text(text.into())).is_err() {
        cleanup(&server, id);
        return;
    }

    // Forward queued snapshots and watch for the client's Move updates / the socket closing.
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
                    continue;
                }
                if let Ok(ClientMsg::Move { x, y, r }) = serde_json::from_str::<ClientMsg>(&text) {
                    let mut s = server.lock().unwrap();
                    if let Some(c) = s.clients.get_mut(&id) {
                        c.x = x;
                        c.y = y;
                        c.r = r.clamp(AOI_RADIUS_MIN, AOI_RADIUS_MAX);
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

fn cleanup(server: &Shared, id: usize) {
    let mut s = server.lock().unwrap();
    s.clients.remove(&id);
    let count = s.clients.len();
    println!("[{id}] disconnected  (total: {count})");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_server_spawns_the_full_typed_field() {
        let s = Server::new(1);
        assert_eq!(s.entities.len(), ENTITY_COUNT);
        for (i, e) in s.entities.iter().enumerate() {
            let expected = if i < SALVAGE_COUNT {
                Kind::Salvage
            } else {
                Kind::Drone
            };
            assert_eq!(e.kind, expected, "entity {i} has the wrong kind");
        }
        // ids are the stable entity indices: a full-world snapshot lists 0..ENTITY_COUNT.
        let all = s.entities_within(WORLD_W * 0.5, WORLD_H * 0.5, WORLD_W + WORLD_H);
        assert_eq!(all.len(), ENTITY_COUNT);
        for (i, body) in all.iter().enumerate() {
            assert_eq!(body.id, i);
        }
    }

    #[test]
    fn entities_stay_inside_world_after_many_steps() {
        let mut s = Server::new(2);
        for _ in 0..2000 {
            s.step();
        }
        for e in &s.entities {
            let r = kind_radius(e.kind);
            assert!(
                e.x >= r - 1e-3 && e.x <= WORLD_W - r + 1e-3,
                "entity x={} escaped the world",
                e.x
            );
            assert!(
                e.y >= r - 1e-3 && e.y <= WORLD_H - r + 1e-3,
                "entity y={} escaped the world",
                e.y
            );
        }
    }

    #[test]
    fn aoi_filter_includes_only_entities_within_radius() {
        let s = Server::new(3);
        let (cx, cy, r) = (WORLD_W * 0.5, WORLD_H * 0.5, 400.0);
        let got = s.entities_within(cx, cy, r);
        // Cross-check against a brute-force squared-distance count.
        let want = s
            .entities
            .iter()
            .filter(|e| {
                let (dx, dy) = (e.x - cx, e.y - cy);
                dx * dx + dy * dy <= r * r
            })
            .count();
        assert_eq!(got.len(), want);
        for body in &got {
            let (dx, dy) = (body.x - cx, body.y - cy);
            assert!(
                dx * dx + dy * dy <= r * r,
                "entity outside the AOI leaked in"
            );
        }
        // Monotonic in r — the live-resize invariant: a larger radius is a superset.
        let bigger = s.entities_within(cx, cy, r + 300.0);
        assert!(bigger.len() >= got.len());
        let small_ids: std::collections::HashSet<usize> = got.iter().map(|b| b.id).collect();
        let big_ids: std::collections::HashSet<usize> = bigger.iter().map(|b| b.id).collect();
        assert!(small_ids.is_subset(&big_ids));
    }

    #[test]
    fn aoi_filter_radius_extremes() {
        let s = Server::new(4);
        // A tiny radius from a corner sees almost nothing; the full world diagonal sees everything.
        let tiny = s.entities_within(0.0, 0.0, 1.0);
        assert!(tiny.len() < ENTITY_COUNT);
        let huge = s.entities_within(WORLD_W * 0.5, WORLD_H * 0.5, WORLD_W + WORLD_H);
        assert_eq!(huge.len(), ENTITY_COUNT);
    }

    #[test]
    fn protocol_round_trips() {
        let welcome = serde_json::to_string(&ServerMsg::Welcome {
            world_w: WORLD_W,
            world_h: WORLD_H,
            total: ENTITY_COUNT,
        })
        .unwrap();
        assert!(welcome.contains(r#""t":"welcome""#));

        let parsed: ServerMsg = serde_json::from_str(
            r#"{"t":"snap","tick":7,"entities":[{"id":3,"kind":"Drone","x":1.0,"y":2.0}]}"#,
        )
        .unwrap();
        match parsed {
            ServerMsg::Snap { tick, entities } => {
                assert_eq!(tick, 7);
                assert_eq!(entities[0].kind, Kind::Drone);
                assert_eq!(entities[0].id, 3);
            }
            _ => panic!("expected Snap"),
        }

        let mv: ClientMsg =
            serde_json::from_str(r#"{"t":"move","x":10.0,"y":20.0,"r":500.0}"#).unwrap();
        assert!(matches!(
            mv,
            ClientMsg::Move {
                x: 10.0,
                y: 20.0,
                r: 500.0
            }
        ));
    }
}
