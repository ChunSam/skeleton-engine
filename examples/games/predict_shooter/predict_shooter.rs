//! Client-prediction shooter — client.
//!
//! ```text
//! # Terminal 1
//! cargo run --example predict_shooter_server
//!
//! # Terminals 2, 3, ...
//! cargo run --example predict_shooter
//! ```
//!
//! Demonstrates the three pillars of responsive game networking against the authoritative
//! `predict_shooter_server`:
//! - **client-side prediction** — your input moves you *immediately* (no round-trip lag);
//! - **server reconciliation** — each snapshot's per-client `ack` snaps you to the server's
//!   authoritative position and replays your un-acked inputs, so prediction never drifts;
//! - **remote interpolation** — other players and bullets are rendered ~`INTERP_DELAY` in the past,
//!   lerped between snapshots, so they move smoothly despite the low snapshot rate.
//!
//! The pure netcode (prediction/reconciliation/interpolation) lives in `client_net.rs` and is
//! unit-tested headlessly; this file wires it into ECS systems + rendering.
//!
//! Controls: WASD / arrows to move, Space to shoot; the bracket keys live-tune the
//! interpolation delay so its feel can be judged in real play.

use engine::{
    App, DrawText, Events, InputState, KeyCode, NetworkClient, NetworkEvent, NetworkSystem, Scene,
    Sprite, System, TextQueue, Transform, WindowConfig, World,
};
use glam::Vec2;
use std::collections::{HashMap, HashSet};

#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;

#[path = "client_net.rs"]
mod client_net;
#[path = "protocol.rs"]
mod protocol;

use client_net::{Interp, Prediction};
use protocol::*;

/// Default interpolation delay (seconds): render remote entities this far in the past so there are
/// always two snapshots to interpolate between at the ~33 ms snapshot interval. Live-tunable at
/// runtime with the bracket keys (see `ShooterClient::interp_delay`) — the one feel parameter
/// automated tests can't judge.
const INTERP_DELAY_DEFAULT: f64 = 0.1;
/// Live-tuning range + step for the interpolation delay (left bracket decreases, right increases).
const INTERP_DELAY_MIN: f64 = 0.0;
const INTERP_DELAY_MAX: f64 = 0.30;
const INTERP_DELAY_STEP: f64 = 0.01;
const PLAYER_SIZE: f32 = 2.0 * PLAYER_HALF;
const BULLET_SIZE: f32 = 10.0;
/// Cap input catch-up so a long stall (tab backgrounded) can't spiral.
const MAX_INPUT_STEPS_PER_FRAME: u32 = 5;

// ── Scene ─────────────────────────────────────────────────────────────────────

struct ShooterScene;

impl Scene for ShooterScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>) {
        world.insert_resource(NetworkClient::connect(&format!("ws://{SERVER_ADDR}")));
        systems.push(Box::new(NetworkSystem));
        systems.push(Box::new(ShooterClient::new()));
    }
}

// ── Client system ───────────────────────────────────────────────────────────────

struct ShooterClient {
    local_id: Option<usize>,
    local_entity: Option<engine::Entity>,
    /// Local-player prediction (created on `welcome`).
    prediction: Option<Prediction>,
    /// Remote players + bullets: the `id → Entity` lifecycle (engine::RemoteEntities).
    remote_players: engine::RemoteEntities<usize>,
    bullets: engine::RemoteEntities<usize>,
    /// Per-remote snapshot buffers for interpolation.
    player_interp: HashMap<usize, Interp>,
    bullet_interp: HashMap<usize, Interp>,
    client_time: f64,
    /// Interpolation delay (seconds), live-tunable with the bracket keys. Starts at
    /// `INTERP_DELAY_DEFAULT`.
    interp_delay: f64,
    input_accum: f32,
    status: String,
}

impl ShooterClient {
    fn new() -> Self {
        Self {
            local_id: None,
            local_entity: None,
            prediction: None,
            remote_players: engine::RemoteEntities::new(),
            bullets: engine::RemoteEntities::new(),
            player_interp: HashMap::new(),
            bullet_interp: HashMap::new(),
            client_time: 0.0,
            interp_delay: INTERP_DELAY_DEFAULT,
            input_accum: 0.0,
            status: format!("Connecting to {SERVER_ADDR} ..."),
        }
    }

    fn spawn_square(
        world: &mut World,
        pos: Vec2,
        size: f32,
        z: f32,
        color: [f32; 3],
    ) -> engine::Entity {
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: pos,
                scale: Vec2::splat(size),
                rotation: 0.0,
                z,
            },
        );
        world.add_component(e, Sprite::colored(color[0], color[1], color[2]));
        e
    }

    fn handle_message(&mut self, world: &mut World, text: &str) {
        if text.len() > MAX_JSON_MESSAGE_BYTES {
            return;
        }
        let msg = match serde_json::from_str::<ServerMsg>(text) {
            Ok(m) => m,
            Err(err) => {
                self.status = format!("Protocol error: {err}");
                return;
            }
        };
        match msg {
            ServerMsg::Welcome { id } => {
                self.local_id = Some(id);
                // Predict from the field centre; the first snapshot reconciles to the real spawn.
                let centre = Vec2::new(FIELD_W * 0.5, FIELD_H * 0.5);
                self.prediction = Some(Prediction::new(centre.x, centre.y));
                self.local_entity = Some(Self::spawn_square(
                    world,
                    centre,
                    PLAYER_SIZE,
                    1.0,
                    [1.0, 1.0, 1.0],
                ));
                self.status = format!("You are Player #{id} — WASD to move, Space to shoot");
            }
            ServerMsg::Snap {
                players,
                bullets,
                ack,
                ..
            } => {
                for p in &players {
                    if Some(p.id) == self.local_id {
                        if let Some(pred) = &mut self.prediction {
                            pred.reconcile(p.x, p.y, ack);
                        }
                    } else {
                        self.player_interp.entry(p.id).or_default().push(
                            self.client_time,
                            p.x,
                            p.y,
                        );
                        let color = remote_color(p.id);
                        let pos = Vec2::new(p.x, p.y);
                        self.remote_players.get_or_spawn(world, p.id, |w| {
                            Self::spawn_square(w, pos, PLAYER_SIZE, 0.0, color)
                        });
                    }
                }

                // Bullets: spawn/refresh from the snapshot, despawn any no longer present.
                let live: HashSet<usize> = bullets.iter().map(|b| b.id).collect();
                for b in &bullets {
                    self.bullet_interp
                        .entry(b.id)
                        .or_default()
                        .push(self.client_time, b.x, b.y);
                    let pos = Vec2::new(b.x, b.y);
                    self.bullets.get_or_spawn(world, b.id, |w| {
                        Self::spawn_square(w, pos, BULLET_SIZE, -1.0, [1.0, 0.85, 0.3])
                    });
                }
                let stale: Vec<usize> = self
                    .bullets
                    .iter()
                    .map(|(&id, _)| id)
                    .filter(|id| !live.contains(id))
                    .collect();
                for id in stale {
                    self.bullets.remove(world, &id);
                    self.bullet_interp.remove(&id);
                }
            }
            ServerMsg::Bye { id } => {
                self.remote_players.remove(world, &id);
                self.player_interp.remove(&id);
            }
        }
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
                "INTERP_DELAY {:.0} ms   ·   [ / ] to tune   ·   default {:.0} ms",
                self.interp_delay * 1000.0,
                INTERP_DELAY_DEFAULT * 1000.0
            ),
            Vec2::new(12.0, 34.0),
            14.0,
            [140, 220, 255, 220],
        ));
        tq.push(DrawText::new(
            "WASD / Arrows move · Space shoots · white = you, colors = rivals",
            Vec2::new(12.0, FIELD_H - 26.0),
            13.0,
            [170, 170, 170, 180],
        ));
    }
}

impl System for ShooterClient {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.client_time += dt as f64;

        // 0. Live-tune the interpolation delay — the one "feel" parameter automation can't judge.
        //    `[` shortens it (snappier, but risks a clamp-freeze on jitter); `]` lengthens it
        //    (smoother under jitter, but more visual lag). Each press steps by INTERP_DELAY_STEP.
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::BracketLeft) {
                self.interp_delay = (self.interp_delay - INTERP_DELAY_STEP).max(INTERP_DELAY_MIN);
            }
            if input.just_pressed(KeyCode::BracketRight) {
                self.interp_delay = (self.interp_delay + INTERP_DELAY_STEP).min(INTERP_DELAY_MAX);
            }
        }

        // 1. Drain network events.
        let events: Vec<NetworkEvent> = world
            .resource::<Events<NetworkEvent>>()
            .map(|bus| bus.read().to_vec())
            .unwrap_or_default();
        for ev in events {
            match ev {
                NetworkEvent::Connected => self.status = "Connected — waiting for ID...".into(),
                NetworkEvent::TextMessage(ref text) => self.handle_message(world, text),
                NetworkEvent::Disconnected { reason } => {
                    self.status = format!("Disconnected: {reason} (is the server running?)")
                }
                NetworkEvent::Error(e) => self.status = format!("Error: {e}"),
                _ => {}
            }
        }

        // 2. Fixed-step input: predict locally + send to the server.
        let (mx, my, fire) = read_input(world);
        self.input_accum += dt;
        let mut steps = 0;
        while self.input_accum >= FIXED_DT && steps < MAX_INPUT_STEPS_PER_FRAME {
            self.input_accum -= FIXED_DT;
            steps += 1;
            if let Some(pred) = &mut self.prediction {
                let seq = pred.predict(mx, my);
                if let Some(client) = world.resource::<NetworkClient>() {
                    if let Ok(text) = serde_json::to_string(&ClientMsg::Input { seq, mx, my, fire })
                    {
                        client.send_text(text);
                    }
                }
            }
        }

        // 3. Apply the predicted position to our avatar.
        if let (Some(e), Some(pred)) = (self.local_entity, &self.prediction) {
            if let Some(tr) = world.get_mut::<Transform>(e) {
                tr.position = Vec2::new(pred.x, pred.y);
            }
        }

        // 4. Render remote players + bullets at the interpolated past position.
        let rt = self.client_time - self.interp_delay;
        let player_updates: Vec<(engine::Entity, Vec2)> = self
            .remote_players
            .iter()
            .filter_map(|(id, e)| {
                self.player_interp
                    .get(id)
                    .and_then(|it| it.sample(rt))
                    .map(|(x, y)| (e, Vec2::new(x, y)))
            })
            .collect();
        let bullet_updates: Vec<(engine::Entity, Vec2)> = self
            .bullets
            .iter()
            .filter_map(|(id, e)| {
                self.bullet_interp
                    .get(id)
                    .and_then(|it| it.sample(rt))
                    .map(|(x, y)| (e, Vec2::new(x, y)))
            })
            .collect();
        for (e, pos) in player_updates.into_iter().chain(bullet_updates) {
            if let Some(tr) = world.get_mut::<Transform>(e) {
                tr.position = pos;
            }
        }

        // 5. HUD.
        self.draw_hud(world);
    }
}

fn read_input(world: &World) -> (f32, f32, bool) {
    let Some(input) = world.resource::<InputState>() else {
        return (0.0, 0.0, false);
    };
    let right = (input.is_pressed(KeyCode::KeyD) || input.is_pressed(KeyCode::ArrowRight)) as i32;
    let left = (input.is_pressed(KeyCode::KeyA) || input.is_pressed(KeyCode::ArrowLeft)) as i32;
    let down = (input.is_pressed(KeyCode::KeyS) || input.is_pressed(KeyCode::ArrowDown)) as i32;
    let up = (input.is_pressed(KeyCode::KeyW) || input.is_pressed(KeyCode::ArrowUp)) as i32;
    let fire = input.is_pressed(KeyCode::Space);
    ((right - left) as f32, (down - up) as f32, fire)
}

/// Maps a player ID to a stable 6-color palette (matches `coin_race`/`mp_client`).
fn remote_color(id: usize) -> [f32; 3] {
    const PALETTE: &[[f32; 3]] = &[
        [1.0, 0.35, 0.35],
        [0.35, 1.0, 0.45],
        [0.35, 0.55, 1.0],
        [1.0, 0.95, 0.3],
        [1.0, 0.4, 1.0],
        [0.3, 1.0, 0.95],
    ];
    PALETTE[id % PALETTE.len()]
}

/// Builds the app and runs it. Shared by the native `main` and the wasm entry point.
fn run() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Predict Shooter — client prediction".to_string(),
        width: FIELD_W as u32,
        height: FIELD_H as u32,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });
    app.register_event::<NetworkEvent>();
    app.set_scene(Box::new(ShooterScene));
    app.run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run();
}

/// WASM entry point (called from an `index.html`, mirroring `coin_race`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_predict_shooter() {
    run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
