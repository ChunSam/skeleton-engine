//! Orbital Dodger — client.
//!
//! ```text
//! # Terminal 1
//! cargo run --example orbital_dodger_server
//!
//! # Terminal 2
//! cargo run --example orbital_dodger
//! ```
//!
//! An **interpolation-only** networked example: cross the field to the green vault while dodging
//! the server's drifting, spinning hazards. The hazards are wholly server-authoritative and arrive
//! at a low 10 Hz; the local player is simulated entirely client-side and never round-trips. There
//! is no prediction or reconciliation here — the only netcode is *interpolation*, which keeps the
//! hazards moving smoothly between the sparse snapshots.
//!
//! This is the second example to use snapshot interpolation, which is why the buffer that was once
//! `predict_shooter`'s private `Interp` is now the public, generic [`engine::SnapshotBuffer<T>`]:
//! here each hazard needs **two** interpolated channels — its position ([`SnapshotBuffer<Vec2>`])
//! and its spin angle ([`SnapshotBuffer<f32>`]) — which is exactly what justified making the helper
//! generic over any [`engine::Lerp`] value instead of hardcoding `(x, y)`.
//!
//! Press `I` to toggle interpolation off and watch the hazards snap between 10 Hz updates — the
//! judder is the problem interpolation solves, made visible.
//!
//! Controls: WASD / arrows move · reach the green vault · `I` toggles interpolation ·
//! `[` / `]` tune the interpolation delay · `R` restart · `Esc` quit.

use engine::{
    App, DrawText, Entity, Events, InputState, KeyCode, NetworkClient, NetworkEvent, NetworkSystem,
    Scene, ShouldQuit, SnapshotBuffer, Sprite, System, TextQueue, Transform, WindowConfig, World,
};
use glam::Vec2;
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;

#[path = "protocol.rs"]
mod protocol;
use protocol::*;

/// Default interpolation delay (seconds): render hazards this far in the past so there are always
/// two snapshots to interpolate between. The snapshot interval is 100 ms (10 Hz), so the delay must
/// be at least one interval; 120 ms leaves a little margin for jitter. Live-tunable with the bracket
/// keys so its feel can be judged in real play.
const INTERP_DELAY_DEFAULT: f64 = 0.12;
const INTERP_DELAY_MIN: f64 = 0.0;
const INTERP_DELAY_MAX: f64 = 0.40;
const INTERP_DELAY_STEP: f64 = 0.02;

const PLAYER_SIZE: f32 = 2.0 * PLAYER_RADIUS;
const HAZARD_SIZE: f32 = 2.0 * HAZARD_RADIUS;
/// Width of the green "vault" strip at the right edge (from `GOAL_X` to the wall).
const GOAL_BAR_W: f32 = FIELD_W - GOAL_X;

/// Player spawn point: the left edge, vertically centred.
fn start_pos() -> Vec2 {
    Vec2::new(PLAYER_RADIUS + 10.0, FIELD_H * 0.5)
}

// ── Scene ───────────────────────────────────────────────────────────────────────

struct DodgerScene;

impl Scene for DodgerScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>) {
        world.insert_resource(NetworkClient::connect(&format!("ws://{SERVER_ADDR}")));

        // The vault door (win zone) — a dim green strip behind everything.
        spawn_square(
            world,
            Vec2::new(GOAL_X + GOAL_BAR_W * 0.5, FIELD_H * 0.5),
            Vec2::new(GOAL_BAR_W, FIELD_H),
            -3.0,
            [0.15, 0.5, 0.22],
        );
        // The local player — white, on top.
        let player = spawn_square(
            world,
            start_pos(),
            Vec2::splat(PLAYER_SIZE),
            1.0,
            [1.0, 1.0, 1.0],
        );

        systems.push(Box::new(NetworkSystem));
        systems.push(Box::new(DodgerClient::new(player)));
    }
}

// ── Client system ─────────────────────────────────────────────────────────────────

struct DodgerClient {
    player: Entity,
    player_pos: Vec2,
    /// Hazards: the `id → Entity` lifecycle (engine::RemoteEntities).
    hazards: engine::RemoteEntities<usize>,
    /// Per-hazard interpolation buffers — position and spin angle, both `engine::SnapshotBuffer`.
    hazard_pos: HashMap<usize, SnapshotBuffer<Vec2>>,
    hazard_rot: HashMap<usize, SnapshotBuffer<f32>>,
    client_time: f64,
    /// Interpolation delay (seconds), live-tunable with the bracket keys.
    interp_delay: f64,
    /// When `false`, render the latest snapshot raw (no interpolation) so the 10 Hz judder shows.
    interp_enabled: bool,
    /// Seconds survived in the current run (resets on catch / restart).
    run_time: f32,
    best: Option<f32>,
    deaths: u32,
    won: bool,
    status: String,
}

impl DodgerClient {
    fn new(player: Entity) -> Self {
        Self {
            player,
            player_pos: start_pos(),
            hazards: engine::RemoteEntities::new(),
            hazard_pos: HashMap::new(),
            hazard_rot: HashMap::new(),
            client_time: 0.0,
            interp_delay: INTERP_DELAY_DEFAULT,
            interp_enabled: true,
            run_time: 0.0,
            best: None,
            deaths: 0,
            won: false,
            status: format!("Connecting to {SERVER_ADDR} ..."),
        }
    }

    /// Spawn/refresh hazards from a snapshot, stamping each interpolation buffer at `client_time`.
    fn ingest(&mut self, world: &mut World, hazards: &[BodyState]) {
        for h in hazards {
            self.hazard_pos
                .entry(h.id)
                .or_default()
                .push(self.client_time, Vec2::new(h.x, h.y));
            self.hazard_rot
                .entry(h.id)
                .or_default()
                .push(self.client_time, h.angle);
            let pos = Vec2::new(h.x, h.y);
            let color = hazard_color(h.id);
            self.hazards.get_or_spawn(world, h.id, |w| {
                spawn_square(w, pos, Vec2::splat(HAZARD_SIZE), 0.0, color)
            });
        }
    }

    fn handle_message(&mut self, world: &mut World, text: &str) {
        if text.len() > MAX_JSON_MESSAGE_BYTES {
            return;
        }
        match serde_json::from_str::<ServerMsg>(text) {
            Ok(ServerMsg::Hello { hazards }) | Ok(ServerMsg::Snap { hazards, .. }) => {
                self.ingest(world, &hazards);
            }
            Err(err) => self.status = format!("Protocol error: {err}"),
        }
    }

    /// Reset the player to the start for a fresh run (on catch or restart).
    fn respawn(&mut self) {
        self.player_pos = start_pos();
        self.run_time = 0.0;
        self.won = false;
    }

    fn draw_hud(&self, world: &mut World) {
        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        tq.push(DrawText::new(
            &self.status,
            Vec2::new(12.0, 10.0),
            18.0,
            [255, 255, 255, 220],
        ));
        tq.push(DrawText::new(
            format!(
                "Interpolation {}  (I)   ·   delay {:.0} ms  [ -20ms   ] +20ms   ·   {} Hz snapshots",
                if self.interp_enabled { "ON " } else { "OFF" },
                self.interp_delay * 1000.0,
                SNAPSHOT_HZ
            ),
            Vec2::new(12.0, 34.0),
            14.0,
            if self.interp_enabled {
                [140, 220, 255, 220]
            } else {
                [255, 180, 120, 230]
            },
        ));
        let best = match self.best {
            Some(b) => format!("{b:.1}s"),
            None => "—".to_string(),
        };
        tq.push(DrawText::new(
            format!(
                "time {:.1}s   ·   best {best}   ·   caught {}",
                self.run_time, self.deaths
            ),
            Vec2::new(12.0, 56.0),
            14.0,
            [200, 200, 200, 210],
        ));
        tq.push(DrawText::new(
            "WASD / Arrows move · reach the green vault · I toggles interpolation · R restart · Esc quit",
            Vec2::new(12.0, FIELD_H - 26.0),
            13.0,
            [170, 170, 170, 180],
        ));
    }
}

impl System for DodgerClient {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.client_time += dt as f64;

        // 0. Toggle / tune / restart / quit keys.
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::KeyI) {
                self.interp_enabled = !self.interp_enabled;
            }
            if input.just_pressed(KeyCode::BracketLeft) {
                self.interp_delay = (self.interp_delay - INTERP_DELAY_STEP).max(INTERP_DELAY_MIN);
            }
            if input.just_pressed(KeyCode::BracketRight) {
                self.interp_delay = (self.interp_delay + INTERP_DELAY_STEP).min(INTERP_DELAY_MAX);
            }
            if input.just_pressed(KeyCode::KeyR) {
                self.respawn();
            }
            if input.just_pressed(KeyCode::Escape) {
                if let Some(q) = world.resource_mut::<ShouldQuit>() {
                    q.0 = true;
                }
            }
        }

        // 1. Drain network events.
        let events: Vec<NetworkEvent> = world
            .resource::<Events<NetworkEvent>>()
            .map(|bus| bus.read().to_vec())
            .unwrap_or_default();
        for ev in events {
            match ev {
                NetworkEvent::Connected => {
                    self.status = "Connected — dodge to the green vault!".into()
                }
                NetworkEvent::TextMessage(ref text) => self.handle_message(world, text),
                NetworkEvent::Disconnected { reason } => {
                    self.status = format!("Disconnected: {reason} (is the server running?)")
                }
                NetworkEvent::Error(e) => self.status = format!("Error: {e}"),
                _ => {}
            }
        }

        // 2. Local player movement (frozen once you've won — press R to run again).
        if !self.won {
            let (mx, my) = read_move(world);
            let len_sq = mx * mx + my * my;
            let dir = if len_sq > 1.0 {
                Vec2::new(mx, my) / len_sq.sqrt()
            } else {
                Vec2::new(mx, my)
            };
            self.player_pos += dir * PLAYER_SPEED * dt;
            self.player_pos.x = self
                .player_pos
                .x
                .clamp(PLAYER_RADIUS, FIELD_W - PLAYER_RADIUS);
            self.player_pos.y = self
                .player_pos
                .y
                .clamp(PLAYER_RADIUS, FIELD_H - PLAYER_RADIUS);
            self.run_time += dt;
            if let Some(tr) = world.get_mut::<Transform>(self.player) {
                tr.position = self.player_pos;
            }
        }

        // 3. Render hazards at the interpolated past time (or the latest snapshot raw when
        //    interpolation is toggled off — `client_time` clamps to the most recent sample, so it
        //    holds-then-jumps at 10 Hz, exactly the judder interpolation removes).
        let rt = if self.interp_enabled {
            self.client_time - self.interp_delay
        } else {
            self.client_time
        };
        let updates: Vec<(Entity, Vec2, f32)> = self
            .hazards
            .iter()
            .filter_map(|(id, e)| {
                let pos = self.hazard_pos.get(id)?.sample(rt)?;
                let angle = self
                    .hazard_rot
                    .get(id)
                    .and_then(|b| b.sample(rt))
                    .unwrap_or(0.0);
                Some((e, pos, angle))
            })
            .collect();
        for &(e, pos, angle) in &updates {
            if let Some(tr) = world.get_mut::<Transform>(e) {
                tr.position = pos;
                tr.rotation = angle;
            }
        }

        // 4. Collision (player vs each hazard at its displayed position) + win check.
        if !self.won {
            let hit_radius = PLAYER_RADIUS + HAZARD_RADIUS;
            let caught = updates
                .iter()
                .any(|&(_, pos, _)| self.player_pos.distance(pos) < hit_radius);
            if caught {
                self.deaths += 1;
                self.respawn();
                self.status = "Caught! Back to the start.".into();
                if let Some(tr) = world.get_mut::<Transform>(self.player) {
                    tr.position = self.player_pos;
                }
            } else if self.player_pos.x + PLAYER_RADIUS >= GOAL_X {
                self.won = true;
                let t = self.run_time;
                self.best = Some(self.best.map_or(t, |b| b.min(t)));
                self.status = format!("You reached the vault in {t:.1}s!  Press R to run again.");
            }
        }

        // 5. HUD.
        self.draw_hud(world);
    }
}

fn read_move(world: &World) -> (f32, f32) {
    let Some(input) = world.resource::<InputState>() else {
        return (0.0, 0.0);
    };
    let right = (input.is_pressed(KeyCode::KeyD) || input.is_pressed(KeyCode::ArrowRight)) as i32;
    let left = (input.is_pressed(KeyCode::KeyA) || input.is_pressed(KeyCode::ArrowLeft)) as i32;
    let down = (input.is_pressed(KeyCode::KeyS) || input.is_pressed(KeyCode::ArrowDown)) as i32;
    let up = (input.is_pressed(KeyCode::KeyW) || input.is_pressed(KeyCode::ArrowUp)) as i32;
    ((right - left) as f32, (down - up) as f32)
}

fn spawn_square(world: &mut World, pos: Vec2, scale: Vec2, z: f32, color: [f32; 3]) -> Entity {
    let e = world.spawn();
    world.add_component(
        e,
        Transform {
            position: pos,
            scale,
            rotation: 0.0,
            z,
        },
    );
    world.add_component(e, Sprite::colored(color[0], color[1], color[2]));
    e
}

/// Maps a hazard id to a warm "danger" palette so hazards read distinctly from the white player
/// and the green vault.
fn hazard_color(id: usize) -> [f32; 3] {
    const PALETTE: &[[f32; 3]] = &[
        [1.0, 0.35, 0.30],
        [1.0, 0.55, 0.25],
        [1.0, 0.40, 0.55],
        [0.95, 0.30, 0.30],
        [1.0, 0.70, 0.30],
        [0.90, 0.45, 0.70],
        [1.0, 0.50, 0.40],
    ];
    PALETTE[id % PALETTE.len()]
}

/// Builds the app and runs it. Shared by the native `main` and the wasm entry point.
fn run() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Orbital Dodger — snapshot interpolation".to_string(),
        width: FIELD_W as u32,
        height: FIELD_H as u32,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });
    app.register_event::<NetworkEvent>();
    app.set_scene(Box::new(DodgerScene));
    app.run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run();
}

/// WASM entry point (called from an `index.html`, mirroring `predict_shooter`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_orbital_dodger() {
    run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
