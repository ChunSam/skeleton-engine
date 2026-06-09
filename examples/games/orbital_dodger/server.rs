//! Orbital Dodger — authoritative hazard server.
//!
//! ```text
//! # Terminal 1
//! cargo run --example orbital_dodger_server
//!
//! # Terminal 2
//! cargo run --example orbital_dodger
//! ```
//!
//! A deliberately simple broadcast server: it owns a field of drifting, spinning hazards, steps
//! them on a fixed 60 Hz tick, and broadcasts a snapshot of every hazard's position + angle at a
//! **low 10 Hz** rate. It never receives anything from clients — the player is simulated entirely
//! client-side, so this server's only job is to be the low-rate, authoritative source of remote
//! motion that the client must *interpolate* to display smoothly.
//!
//! Contrast with `predict_shooter_server` (movement authority + per-client `ack` for
//! reconciliation) and `coin_race_server` (authoritative shared resource): this one has no client
//! input at all, which is exactly what isolates interpolation from prediction.
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

/// Dependency-free xorshift64 RNG — only used to scatter hazard spawns/velocities.
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

    /// Returns `+1.0` or `-1.0` with even odds.
    fn sign(&mut self) -> f32 {
        if self.next_u64() & 1 == 0 {
            1.0
        } else {
            -1.0
        }
    }
}

struct Hazard {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    /// Spin angle (radians), grown **unbounded** on purpose: the client lerps angle as a plain
    /// `f32`, so wrapping it at `TAU` would make the interpolation spin backwards across the seam.
    /// Each snapshot step is small (`spin · 100 ms`), and `f32` precision is ample for a demo.
    angle: f32,
    spin: f32,
}

struct Server {
    hazards: Vec<Hazard>,
    clients: HashMap<usize, mpsc::Sender<Message>>,
    next_client_id: usize,
    tick: u32,
}

impl Server {
    fn new(seed: u64) -> Self {
        let mut rng = Rng(seed | 1); // xorshift must never be seeded with 0
        let hazards = (0..HAZARD_COUNT)
            .map(|_| {
                let speed = rng.range(HAZARD_MIN_SPEED, HAZARD_MAX_SPEED);
                let heading = rng.range(0.0, TAU);
                Hazard {
                    // Spawn away from the left start strip so the player isn't spawn-killed.
                    x: rng.range(FIELD_W * 0.28, FIELD_W - HAZARD_RADIUS),
                    y: rng.range(HAZARD_RADIUS, FIELD_H - HAZARD_RADIUS),
                    vx: speed * heading.cos(),
                    vy: speed * heading.sin(),
                    angle: rng.range(0.0, TAU),
                    spin: rng.range(HAZARD_MIN_SPIN, HAZARD_MAX_SPIN) * rng.sign(),
                }
            })
            .collect();
        Self {
            hazards,
            clients: HashMap::new(),
            next_client_id: 1,
            tick: 0,
        }
    }

    /// Advances the hazard field one fixed step: drift, bounce off the walls, spin. Pure (no
    /// networking) so it is unit-testable.
    fn step(&mut self) {
        for h in self.hazards.iter_mut() {
            h.x += h.vx * FIXED_DT;
            h.y += h.vy * FIXED_DT;
            // Reflect off the walls, snapping back inside so a hazard can't tunnel out.
            if h.x < HAZARD_RADIUS {
                h.x = HAZARD_RADIUS;
                h.vx = h.vx.abs();
            } else if h.x > FIELD_W - HAZARD_RADIUS {
                h.x = FIELD_W - HAZARD_RADIUS;
                h.vx = -h.vx.abs();
            }
            if h.y < HAZARD_RADIUS {
                h.y = HAZARD_RADIUS;
                h.vy = h.vy.abs();
            } else if h.y > FIELD_H - HAZARD_RADIUS {
                h.y = FIELD_H - HAZARD_RADIUS;
                h.vy = -h.vy.abs();
            }
            h.angle += h.spin * FIXED_DT;
        }
        self.tick = self.tick.wrapping_add(1);
    }

    fn hazard_states(&self) -> Vec<BodyState> {
        self.hazards
            .iter()
            .enumerate()
            .map(|(id, h)| BodyState {
                id,
                x: h.x,
                y: h.y,
                angle: h.angle,
            })
            .collect()
    }

    fn broadcast_snapshot(&self) {
        let Ok(text) = serde_json::to_string(&ServerMsg::Snap {
            tick: self.tick,
            hazards: self.hazard_states(),
        }) else {
            return;
        };
        let frame = Message::Text(text.into());
        for sender in self.clients.values() {
            let _ = sender.send(frame.clone());
        }
    }
}

type Shared = Arc<Mutex<Server>>;

fn main() {
    let listener = TcpListener::bind(SERVER_ADDR).expect("bind failed");
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15);
    let server: Shared = Arc::new(Mutex::new(Server::new(seed)));

    println!("orbital_dodger_server: broadcast hazard server on ws://{SERVER_ADDR}");
    println!(
        "  {HAZARD_COUNT} hazards · {SNAPSHOT_HZ} Hz snapshots (sim {:.0} Hz) · run `orbital_dodger`",
        1.0 / FIXED_DT
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

    // Register the client and grab the current hazard set for its Hello (cloned under the lock,
    // sent outside it — the 60 Hz tick thread needs the lock 60×/s, so we never block a network
    // send while holding it).
    let (id, hello) = {
        let mut s = server.lock().unwrap();
        let id = s.next_client_id;
        s.next_client_id += 1;
        s.clients.insert(id, tx);
        let hello = ServerMsg::Hello {
            hazards: s.hazard_states(),
        };
        let count = s.clients.len();
        println!("[{id}] connected from {peer:?}  (total: {count})");
        (id, hello)
    };

    let Ok(text) = serde_json::to_string(&hello) else {
        cleanup(&server, id);
        return;
    };
    if ws.send(Message::Text(text.into())).is_err() {
        cleanup(&server, id);
        return;
    }

    // The client sends nothing; this loop just forwards queued snapshots and watches for the
    // socket closing.
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
    fn new_server_spawns_the_full_hazard_set() {
        let s = Server::new(1);
        let states = s.hazard_states();
        assert_eq!(states.len(), HAZARD_COUNT);
        // ids are the stable hazard indices.
        for (i, body) in states.iter().enumerate() {
            assert_eq!(body.id, i);
        }
    }

    #[test]
    fn hazards_stay_inside_the_field_after_many_steps() {
        let mut s = Server::new(2);
        for _ in 0..2000 {
            s.step();
        }
        for h in &s.hazards {
            assert!(
                h.x >= HAZARD_RADIUS - 1e-3 && h.x <= FIELD_W - HAZARD_RADIUS + 1e-3,
                "hazard x={} escaped the field",
                h.x
            );
            assert!(
                h.y >= HAZARD_RADIUS - 1e-3 && h.y <= FIELD_H - HAZARD_RADIUS + 1e-3,
                "hazard y={} escaped the field",
                h.y
            );
        }
    }

    #[test]
    fn step_advances_spin_angle() {
        let mut s = Server::new(3);
        let before: Vec<f32> = s.hazards.iter().map(|h| h.angle).collect();
        s.step();
        for (h, &a0) in s.hazards.iter().zip(&before) {
            let expected = a0 + h.spin * FIXED_DT;
            assert!((h.angle - expected).abs() < 1e-4);
            assert_ne!(
                h.angle, a0,
                "a spinning hazard's angle must change each step"
            );
        }
    }

    #[test]
    fn protocol_round_trips() {
        let hello = serde_json::to_string(&ServerMsg::Hello {
            hazards: vec![BodyState {
                id: 0,
                x: 1.0,
                y: 2.0,
                angle: 0.5,
            }],
        })
        .unwrap();
        assert!(hello.contains(r#""t":"hello""#));
        let parsed: ServerMsg =
            serde_json::from_str(r#"{"t":"snap","tick":4,"hazards":[]}"#).unwrap();
        assert!(matches!(parsed, ServerMsg::Snap { tick: 4, .. }));
    }
}
