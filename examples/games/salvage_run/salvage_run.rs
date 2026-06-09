//! Salvage Run — client.
//!
//! ```text
//! # Terminal 1
//! cargo run --example salvage_run_server
//!
//! # Terminal 2
//! cargo run --example salvage_run
//! ```
//!
//! An **area-of-interest (AOI) streaming** networked example: pilot a ship across a world far larger
//! than the window, collecting drifting salvage while dodging roaming drones. The world's ~120
//! entities are wholly server-authoritative, but the server streams you **only** the ones within an
//! interest radius of your ship — so as you roam, entities continuously stream in and out. The
//! client interpolates each streamed entity's motion with `engine::SnapshotBuffer<Vec2>` (its third
//! call site) and evicts entities that fall out of the AOI by a last-seen timeout, because the
//! server signals departure only by *omission* (it stops sending them).
//!
//! This is the engine's first example to stress interest management / staleness churn (open
//! questions #4/#5 in `docs/REMOTE_ENTITIES_DESIGN.md`) and the first to call
//! `RemoteEntities::clear` on disconnect (#7). Two entity kinds are kept as two `RemoteEntities`
//! maps — the "N maps" baseline for #4.
//!
//! Controls: WASD / arrows roam · `-` / `=` shrink/grow the AOI (watch entities stream in/out) ·
//! `[` / `]` tune the interpolation delay · `R` reset · `Esc` quit.

use engine::Camera;
use engine::{
    App, DrawText, Entity, Events, InputState, KeyCode, NetworkClient, NetworkEvent, NetworkSystem,
    Scene, ShouldQuit, SnapshotBuffer, Sprite, System, TextQueue, Transform, WindowConfig, World,
};
use glam::Vec2;
use std::collections::{HashMap, HashSet};
use std::f32::consts::TAU;

#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;

#[path = "protocol.rs"]
mod protocol;
use protocol::*;

/// Interpolation delay (seconds): render streamed entities this far in the past so there are always
/// two snapshots to interpolate between. The snapshot interval is ~83 ms (12 Hz); 125 ms leaves
/// margin for jitter. Live-tunable with the bracket keys.
const INTERP_DELAY_DEFAULT: f64 = 0.125;
const INTERP_DELAY_MIN: f64 = 0.0;
const INTERP_DELAY_MAX: f64 = 0.40;
const INTERP_DELAY_STEP: f64 = 0.02;

/// Evict a streamed entity this long after its last snapshot. = three snapshot intervals (~0.25 s),
/// so an in-AOI entity that merely misses a snapshot isn't evicted, but one that leaves the AOI
/// (the server stops sending it) is dropped promptly — this timeout *is* the eviction hysteresis.
const STALE_TIMEOUT: f64 = 3.0 / SNAPSHOT_HZ as f64;

/// How long between `Move` reports to the server (throttle continuous 60 fps movement).
const MOVE_SEND_INTERVAL: f32 = 1.0 / MOVE_SEND_HZ;

/// Number of dots drawn around the AOI boundary ring.
const AOI_RING_DOTS: usize = 28;

/// Player spawn / respawn point: a sparse spot near the left edge, so the field of salvage is out in
/// the world and you have to actively roam (streaming entities in) to collect it — rather than
/// idling in the dense centre.
fn start_pos() -> Vec2 {
    Vec2::new(WORLD_W * 0.12, WORLD_H * 0.5)
}

// ── Scene ───────────────────────────────────────────────────────────────────────

struct SalvageScene;

impl Scene for SalvageScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>) {
        world.insert_resource(NetworkClient::connect(&format!("ws://{SERVER_ADDR}")));

        // Camera: scroll across the large world, clamped to its bounds (App auto-clamps each frame).
        if let Some(cam) = world.resource_mut::<Camera>() {
            cam.bounds = Some((Vec2::ZERO, Vec2::new(WORLD_W, WORLD_H)));
            cam.follow_entity = None;
            cam.zoom = 1.0;
        } else {
            let mut cam = Camera::new(Vec2::ZERO, 1.0);
            cam.bounds = Some((Vec2::ZERO, Vec2::new(WORLD_W, WORLD_H)));
            world.insert_resource(cam);
        }

        // Static background tile grid so the camera's motion is visible against the empty world.
        spawn_background(world);

        // The local player — white, on top. Starts at world centre.
        let player = spawn_square(
            world,
            start_pos(),
            Vec2::splat(2.0 * PLAYER_RADIUS),
            1.0,
            [1.0, 1.0, 1.0],
        );

        // The AOI boundary ring (dim dots), repositioned each frame around the player.
        let ring: Vec<Entity> = (0..AOI_RING_DOTS)
            .map(|_| {
                spawn_square(
                    world,
                    start_pos(),
                    Vec2::splat(5.0),
                    -1.0,
                    [0.35, 0.5, 0.65],
                )
            })
            .collect();

        systems.push(Box::new(NetworkSystem));
        systems.push(Box::new(SalvageClient::new(player, ring)));
    }
}

// ── Client system ─────────────────────────────────────────────────────────────────

struct SalvageClient {
    player: Entity,
    player_pos: Vec2,
    aoi_ring: Vec<Entity>,
    /// Two entity kinds kept as two `RemoteEntities` maps — the #4 "N maps" baseline.
    salvage: engine::RemoteEntities<usize>,
    drones: engine::RemoteEntities<usize>,
    /// Per-entity interpolation buffers (`engine::SnapshotBuffer<Vec2>`), keyed by the same ids.
    salvage_buf: HashMap<usize, SnapshotBuffer<Vec2>>,
    drone_buf: HashMap<usize, SnapshotBuffer<Vec2>>,
    /// id → client time of the last snapshot that contained it (drives staleness eviction, #5).
    last_seen: HashMap<usize, f64>,
    /// Salvage already collected — suppressed from re-streaming so it isn't re-counted.
    collected: HashSet<usize>,
    client_time: f64,
    interp_delay: f64,
    aoi_radius: f32,
    move_send_accum: f32,
    score: u32,
    won: bool,
    total: usize,
    status: String,
}

impl SalvageClient {
    fn new(player: Entity, aoi_ring: Vec<Entity>) -> Self {
        Self {
            player,
            player_pos: start_pos(),
            aoi_ring,
            salvage: engine::RemoteEntities::new(),
            drones: engine::RemoteEntities::new(),
            salvage_buf: HashMap::new(),
            drone_buf: HashMap::new(),
            last_seen: HashMap::new(),
            collected: HashSet::new(),
            client_time: 0.0,
            interp_delay: INTERP_DELAY_DEFAULT,
            aoi_radius: AOI_RADIUS_DEFAULT,
            move_send_accum: 0.0,
            score: 0,
            won: false,
            total: ENTITY_COUNT,
            status: format!("Connecting to {SERVER_ADDR} ..."),
        }
    }

    fn handle_message(&mut self, world: &mut World, text: &str) {
        if text.len() > MAX_JSON_MESSAGE_BYTES {
            return;
        }
        match serde_json::from_str::<ServerMsg>(text) {
            Ok(ServerMsg::Welcome { total, .. }) => self.total = total,
            Ok(ServerMsg::Snap { entities, .. }) => self.ingest(world, &entities),
            Err(err) => self.status = format!("Protocol error: {err}"),
        }
    }

    /// Spawn/refresh streamed entities from an AOI snapshot, stamping each interpolation buffer at
    /// `client_time` and refreshing its last-seen clock.
    fn ingest(&mut self, world: &mut World, entities: &[EntityState]) {
        for ent in entities {
            let pos = Vec2::new(ent.x, ent.y);
            match ent.kind {
                Kind::Salvage => {
                    if self.collected.contains(&ent.id) {
                        continue; // already grabbed — don't let the server re-stream it back
                    }
                    self.last_seen.insert(ent.id, self.client_time);
                    self.salvage_buf
                        .entry(ent.id)
                        .or_default()
                        .push(self.client_time, pos);
                    let color = salvage_color(ent.id);
                    self.salvage.get_or_spawn(world, ent.id, |w| {
                        spawn_square(w, pos, Vec2::splat(2.0 * SALVAGE_RADIUS), 0.0, color)
                    });
                }
                Kind::Drone => {
                    self.last_seen.insert(ent.id, self.client_time);
                    self.drone_buf
                        .entry(ent.id)
                        .or_default()
                        .push(self.client_time, pos);
                    let color = drone_color(ent.id);
                    self.drones.get_or_spawn(world, ent.id, |w| {
                        spawn_square(w, pos, Vec2::splat(2.0 * DRONE_RADIUS), 0.0, color)
                    });
                }
            }
        }
    }

    /// Drop entities not seen for `STALE_TIMEOUT` — they left the AOI and the server stopped sending
    /// them (removal-by-omission). Probing both maps to find an id's kind is the #4 "N maps" wart.
    fn evict_stale(&mut self, world: &mut World) {
        let cutoff = self.client_time - STALE_TIMEOUT;
        let stale: Vec<usize> = self
            .last_seen
            .iter()
            .filter(|(_, &t)| t < cutoff)
            .map(|(&id, _)| id)
            .collect();
        for id in stale {
            if self.salvage.contains_key(&id) {
                self.salvage.remove(world, &id);
                self.salvage_buf.remove(&id);
            } else if self.drones.contains_key(&id) {
                self.drones.remove(world, &id);
                self.drone_buf.remove(&id);
            }
            self.last_seen.remove(&id);
        }
    }

    /// Drop every streamed entity + its buffers (on disconnect). The first example to exercise
    /// `RemoteEntities::clear` — note it must also clear the *parallel* maps the engine doesn't own.
    fn disconnect_cleanup(&mut self, world: &mut World) {
        self.salvage.clear(world);
        self.drones.clear(world);
        self.salvage_buf.clear();
        self.drone_buf.clear();
        self.last_seen.clear();
    }

    fn reset(&mut self, world: &mut World) {
        self.player_pos = start_pos();
        self.score = 0;
        self.won = false;
        self.collected.clear();
        if let Some(tr) = world.get_mut::<Transform>(self.player) {
            tr.position = self.player_pos;
        }
    }

    fn send_move(&self, world: &World) {
        if let Some(client) = world.resource::<NetworkClient>() {
            if let Ok(text) = serde_json::to_string(&ClientMsg::Move {
                x: self.player_pos.x,
                y: self.player_pos.y,
                r: self.aoi_radius,
            }) {
                client.send_text(text);
            }
        }
    }

    fn draw_hud(&self, world: &mut World) {
        let streaming = self.salvage.len() + self.drones.len();
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
            format!("streaming {streaming} / {} entities (AOI)", self.total),
            Vec2::new(12.0, 34.0),
            15.0,
            [140, 220, 255, 230],
        ));
        tq.push(DrawText::new(
            format!(
                "AOI radius {:.0}  (- / =)   ·   collected {}/{}   ·   interp {:.0} ms  [ ]",
                self.aoi_radius,
                self.score,
                COLLECT_GOAL,
                self.interp_delay * 1000.0
            ),
            Vec2::new(12.0, 56.0),
            14.0,
            [200, 200, 200, 210],
        ));
        tq.push(DrawText::new(
            "WASD / Arrows roam · - / = resize AOI · [ ] interp delay · R reset · Esc quit",
            Vec2::new(12.0, VIEW_H - 26.0),
            13.0,
            [170, 170, 170, 180],
        ));
    }
}

impl System for SalvageClient {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.client_time += dt as f64;

        // 0. Toggle / tune / reset / quit keys. Read all key states first (releasing the
        //    InputState borrow) so the actions below can mutate the world.
        let mut aoi_changed = false;
        let (interp_dn, interp_up, aoi_dn, aoi_up, do_reset, do_quit) =
            match world.resource::<InputState>() {
                Some(input) => (
                    input.just_pressed(KeyCode::BracketLeft),
                    input.just_pressed(KeyCode::BracketRight),
                    input.just_pressed(KeyCode::Minus),
                    input.just_pressed(KeyCode::Equal),
                    input.just_pressed(KeyCode::KeyR),
                    input.just_pressed(KeyCode::Escape),
                ),
                None => (false, false, false, false, false, false),
            };
        if interp_dn {
            self.interp_delay = (self.interp_delay - INTERP_DELAY_STEP).max(INTERP_DELAY_MIN);
        }
        if interp_up {
            self.interp_delay = (self.interp_delay + INTERP_DELAY_STEP).min(INTERP_DELAY_MAX);
        }
        if aoi_dn {
            self.aoi_radius = (self.aoi_radius - AOI_RADIUS_STEP).max(AOI_RADIUS_MIN);
            aoi_changed = true;
        }
        if aoi_up {
            self.aoi_radius = (self.aoi_radius + AOI_RADIUS_STEP).min(AOI_RADIUS_MAX);
            aoi_changed = true;
        }
        if do_reset {
            self.reset(world);
        }
        if do_quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.0 = true;
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
                    self.status = "Connected — collect salvage, dodge the drones!".into()
                }
                NetworkEvent::TextMessage(ref text) => self.handle_message(world, text),
                NetworkEvent::Disconnected { reason } => {
                    self.status = format!("Disconnected: {reason} (is the server running?)");
                    self.disconnect_cleanup(world);
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
                .clamp(PLAYER_RADIUS, WORLD_W - PLAYER_RADIUS);
            self.player_pos.y = self
                .player_pos
                .y
                .clamp(PLAYER_RADIUS, WORLD_H - PLAYER_RADIUS);
            if let Some(tr) = world.get_mut::<Transform>(self.player) {
                tr.position = self.player_pos;
            }
        }

        // 3. Report our AOI centre to the server (throttled; immediate on an AOI resize so it feels
        //    instant).
        self.move_send_accum += dt;
        if aoi_changed || self.move_send_accum >= MOVE_SEND_INTERVAL {
            self.move_send_accum = 0.0;
            self.send_move(world);
        }

        // 4. Evict entities that left the AOI (server stopped sending them).
        self.evict_stale(world);

        // 5. Sample + apply transforms (collect-then-apply: read self+buffers first, then mutate).
        let rt = if self.interp_delay > 0.0 {
            self.client_time - self.interp_delay
        } else {
            self.client_time
        };
        let mut updates: Vec<(usize, Entity, Vec2, Kind)> = Vec::new();
        for (id, e) in self.salvage.iter() {
            if let Some(pos) = self.salvage_buf.get(id).and_then(|b| b.sample(rt)) {
                updates.push((*id, e, pos, Kind::Salvage));
            }
        }
        for (id, e) in self.drones.iter() {
            if let Some(pos) = self.drone_buf.get(id).and_then(|b| b.sample(rt)) {
                updates.push((*id, e, pos, Kind::Drone));
            }
        }
        for &(_, e, pos, _) in &updates {
            if let Some(tr) = world.get_mut::<Transform>(e) {
                tr.position = pos;
            }
        }

        // 6. Collision / collect / win (against the displayed interpolated positions).
        if !self.won {
            let mut collected: Vec<usize> = Vec::new();
            let mut hit_drone = false;
            for &(id, _, pos, kind) in &updates {
                let hit_r = PLAYER_RADIUS + kind_radius(kind);
                if self.player_pos.distance(pos) < hit_r {
                    match kind {
                        Kind::Salvage => collected.push(id),
                        Kind::Drone => hit_drone = true,
                    }
                }
            }
            for id in collected {
                self.score += 1;
                self.collected.insert(id);
                self.salvage.remove(world, &id);
                self.salvage_buf.remove(&id);
                self.last_seen.remove(&id);
            }
            if hit_drone {
                self.player_pos = start_pos();
                self.status = "Hit a drone — back to the start!".into();
                if let Some(tr) = world.get_mut::<Transform>(self.player) {
                    tr.position = self.player_pos;
                }
            }
            if self.score >= COLLECT_GOAL {
                self.won = true;
                self.status = format!(
                    "Salvage run complete — {} collected! Press R to run again.",
                    self.score
                );
            }
        }

        // 7. Camera follows the player (App clamps it to the world bounds after this system).
        if let Some(cam) = world.resource_mut::<Camera>() {
            cam.position = self.player_pos - Vec2::new(VIEW_W, VIEW_H) * 0.5;
        }

        // 8. Reposition the AOI boundary ring around the player (circular — matches the server's
        //    squared-distance filter, so it shows exactly where entities start streaming).
        for (i, &dot) in self.aoi_ring.iter().enumerate() {
            let theta = (i as f32) / (AOI_RING_DOTS as f32) * TAU;
            let p = self.player_pos + Vec2::new(theta.cos(), theta.sin()) * self.aoi_radius;
            if let Some(tr) = world.get_mut::<Transform>(dot) {
                tr.position = p;
            }
        }

        // 9. HUD.
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

fn kind_radius(kind: Kind) -> f32 {
    match kind {
        Kind::Salvage => SALVAGE_RADIUS,
        Kind::Drone => DRONE_RADIUS,
    }
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

/// A static checkerboard tile grid spanning the world, so the scrolling camera has visible texture.
fn spawn_background(world: &mut World) {
    const TILE: f32 = 240.0;
    let cols = (WORLD_W / TILE).ceil() as i32;
    let rows = (WORLD_H / TILE).ceil() as i32;
    for gy in 0..rows {
        for gx in 0..cols {
            let shade = if (gx + gy) % 2 == 0 {
                [0.10, 0.11, 0.15]
            } else {
                [0.08, 0.09, 0.12]
            };
            let pos = Vec2::new(gx as f32 * TILE + TILE * 0.5, gy as f32 * TILE + TILE * 0.5);
            spawn_square(world, pos, Vec2::splat(TILE), -2.0, shade);
        }
    }
}

/// Salvage = cool cyan/teal palette (collectible); reads distinctly from the warm drones.
fn salvage_color(id: usize) -> [f32; 3] {
    const PALETTE: &[[f32; 3]] = &[
        [0.35, 0.85, 0.80],
        [0.40, 0.80, 0.95],
        [0.55, 0.90, 0.70],
        [0.45, 0.75, 0.90],
    ];
    PALETTE[id % PALETTE.len()]
}

/// Drones = warm "danger" palette (avoid).
fn drone_color(id: usize) -> [f32; 3] {
    const PALETTE: &[[f32; 3]] = &[[1.0, 0.45, 0.35], [1.0, 0.55, 0.25], [0.95, 0.35, 0.45]];
    PALETTE[id % PALETTE.len()]
}

/// Builds the app and runs it. Shared by the native `main` and the wasm entry point.
fn run() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Salvage Run — AOI streaming".to_string(),
        width: VIEW_W as u32,
        height: VIEW_H as u32,
        clear_color: [0.04, 0.05, 0.08, 1.0],
    });
    app.register_event::<NetworkEvent>();
    app.set_scene(Box::new(SalvageScene));
    app.run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run();
}

/// WASM entry point (called from an `index.html`, mirroring the other networked examples).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_salvage_run() {
    run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
