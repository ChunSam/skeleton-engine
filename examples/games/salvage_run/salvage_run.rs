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
    Scene, ShouldQuit, SnapshotBuffer, Sprite, System, SystemRegistrar, TextQueue, Transform,
    WindowConfig, World,
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
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        world.insert_resource(NetworkClient::connect(&format!("ws://{}", server_addr())));

        let (player, ring) = build_world(world);

        systems.add(NetworkSystem::new());
        systems.add(SalvageClient::new(player, ring));
    }
}

/// Everything the scene puts in the `World` except the socket: camera bounds, the background grid,
/// the player, and the AOI ring. Returns what `SalvageClient::new` needs.
///
/// Split out of `on_enter` so `self_test` can stand up the *same* world. The socket stays behind in
/// `on_enter` deliberately — the device-free checks drive the client with injected protocol
/// messages and must not have a live connection racing them, and the connect line is covered by the
/// server-backed checks instead.
fn build_world(world: &mut World) -> (Entity, Vec<Entity>) {
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

    (player, ring)
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
            status: format!("Connecting to {} ...", server_addr()),
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
    // `SALVAGE_RUN_SELFTEST=1` runs the headless acceptance test instead of opening a window.
    if std::env::var("SALVAGE_RUN_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }
    run();
}

// ── Acceptance test ───────────────────────────────────────────────────────────────────────────

/// `SALVAGE_RUN_SELFTEST=1 cargo run --example salvage_run` — asserts what this example exists to
/// show and a screenshot cannot.
///
/// Interest management is the most screenshot-invisible feature in the tree. A frame of this game
/// shows some squares near a ship; whether the ones that left the AOI were **evicted** or are
/// piling up forever off-camera looks identical, and the failure is worse than invisible — it is
/// *flattering*. With eviction dead the world only ever gains entities, so the HUD's
/// "streaming N / total" climbs toward the total and the game looks like it is streaming
/// beautifully. Removal is signalled only by omission, so there is no message whose absence
/// anything would notice.
///
/// Checks 1-4 need no server and run anywhere. Checks 5-6 spawn the real `salvage_run_server` on a
/// port of their own and **SKIP with exit 0** if that binary was never built — the same rule
/// `BEAT_CRAWLER_SELFTEST` uses for a missing audio device.
///
/// Exit codes: `0` pass (server checks may have skipped) · `1` a snapshot does not stream entities
/// in · `2` removal-by-omission does not evict, or evicts entities that are still arriving ·
/// `3` a disconnect leaves streamed entities or the parallel maps behind · `4` streamed motion is
/// not interpolated · `5` the server does not tailor its snapshot to the AOI radius · `6` against a
/// real server, shrinking the AOI never drains what it streamed.
#[cfg(not(target_arch = "wasm32"))]
fn self_test() -> i32 {
    use std::time::{Duration, Instant};

    /// Simulated frame time for the offline checks. Everything they exercise is driven by `dt`, so a
    /// fixed step reproduces real play exactly. The **server-backed** checks below must not use it:
    /// the server ticks in wall-clock time, so those are paced off `Instant`.
    const DT: f32 = 1.0 / 60.0;
    /// Snapshot cadence in frames — the client's own `SNAPSHOT_HZ`, expressed in ticks.
    const SNAP_EVERY: usize = (60 / SNAPSHOT_HZ) as usize;

    /// The scene's world, minus the socket, plus the buses `App` owns. `build_world` is the
    /// game's — a harness that spawned its own player and ring would stop testing the scene.
    fn harness() -> (App, SalvageClient) {
        let mut app = App::new();
        app.register_event::<NetworkEvent>();
        let (player, ring) = build_world(&mut app.world);
        (app, SalvageClient::new(player, ring))
    }

    /// Hand the client a real protocol message, through the real event bus.
    fn feed(world: &mut World, msg: &ServerMsg) {
        let text = serde_json::to_string(msg).expect("encode ServerMsg");
        if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
            bus.send(NetworkEvent::TextMessage(text));
        }
    }

    /// One frame in the scene's order, then the end-of-frame flush `App` performs.
    fn tick(world: &mut World, net: &mut NetworkSystem, client: &mut SalvageClient, dt: f32) {
        net.run(world, dt);
        client.run(world, dt);
        if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
            bus.flush();
        }
    }

    fn snap(tick: u32, entities: Vec<EntityState>) -> ServerMsg {
        ServerMsg::Snap { tick, entities }
    }

    fn at(id: usize, kind: Kind, pos: Vec2) -> EntityState {
        EntityState {
            id,
            kind,
            x: pos.x,
            y: pos.y,
        }
    }

    let streaming = |c: &SalvageClient| c.salvage.len() + c.drones.len();

    // Positions are placed on the player so nothing is collected or collides mid-check; the
    // collision pass runs against the interpolated positions and would quietly delete salvage.
    let far = |i: usize| start_pos() + Vec2::new(600.0 + i as f32 * 40.0, 0.0);

    // ── 1. A snapshot streams entities in ─────────────────────────────────────────────────────
    {
        let (mut app, mut client) = harness();
        let mut net = NetworkSystem::new();
        let want: Vec<EntityState> = vec![
            at(1, Kind::Salvage, far(0)),
            at(2, Kind::Salvage, far(1)),
            at(SALVAGE_COUNT + 1, Kind::Drone, far(2)),
        ];
        feed(&mut app.world, &snap(1, want.clone()));
        tick(&mut app.world, &mut net, &mut client, DT);

        let alive = want
            .iter()
            .filter(|e| {
                match e.kind {
                    Kind::Salvage => client.salvage.get(&e.id),
                    Kind::Drone => client.drones.get(&e.id),
                }
                .is_some_and(|ent| app.world.is_alive(ent))
            })
            .count();
        if client.salvage.len() != 2 || client.drones.len() != 1 || alive != want.len() {
            eprintln!(
                "FAIL: a snapshot did not stream its entities in — {} salvage (want 2), {} drones \
                 (want 1), {alive} of {} spawned into the world",
                client.salvage.len(),
                client.drones.len(),
                want.len()
            );
            return 1;
        }
        println!(
            "stream-in ok: {} entities from one snapshot",
            streaming(&client)
        );
    }

    // ── 2. Removal-by-omission evicts — and only what stopped arriving ────────────────────────
    //
    // The headline, and the check the whole example is exposed to. The server never says an entity
    // left; it stops including it. Both halves are asserted, because each alone is passable by a
    // broken client: one that never evicts keeps everything, and one that evicts on every frame
    // keeps nothing — and *that* one would also make the "gone" half green.
    {
        let (mut app, mut client) = harness();
        let mut net = NetworkSystem::new();
        let kept: Vec<usize> = vec![1, 2];
        let dropped: Vec<usize> = vec![3, 4, SALVAGE_COUNT + 1];

        let all: Vec<EntityState> = kept
            .iter()
            .chain(dropped.iter())
            .enumerate()
            .map(|(i, &id)| {
                let kind = if id < SALVAGE_COUNT {
                    Kind::Salvage
                } else {
                    Kind::Drone
                };
                at(id, kind, far(i))
            })
            .collect();
        feed(&mut app.world, &snap(1, all));
        tick(&mut app.world, &mut net, &mut client, DT);

        // Handles captured before eviction, so "gone" can mean despawned and not merely unmapped.
        let handles: Vec<(usize, Entity)> = dropped
            .iter()
            .filter_map(|id| {
                client
                    .salvage
                    .get(id)
                    .or_else(|| client.drones.get(id))
                    .map(|e| (*id, e))
            })
            .collect();
        if handles.len() != dropped.len() {
            eprintln!("FAIL: setup for the eviction check did not stream all of its entities");
            return 2;
        }

        // Keep sending only `kept`, at the real snapshot cadence, for twice the stale timeout.
        //
        // `kept` is watched **every frame**, not just at the end. A hysteresis window shorter than
        // the snapshot interval evicts a still-arriving entity in the gap between snapshots and then
        // re-spawns it on the next one, so an end-state assertion sees nothing wrong — which is
        // exactly what the first draft did, and it passed a `STALE_TIMEOUT` sabotaged to 0.05 s.
        let frames = (STALE_TIMEOUT * 2.0 * 60.0).ceil() as usize;
        let mut kept_flickered: Vec<usize> = Vec::new();
        for f in 0..frames {
            if f % SNAP_EVERY == 0 {
                let live: Vec<EntityState> = kept
                    .iter()
                    .enumerate()
                    .map(|(i, &id)| at(id, Kind::Salvage, far(i)))
                    .collect();
                feed(&mut app.world, &snap(2 + f as u32, live));
            }
            tick(&mut app.world, &mut net, &mut client, DT);
            for id in &kept {
                if !client.salvage.contains_key(id) && !kept_flickered.contains(id) {
                    kept_flickered.push(*id);
                }
            }
        }

        let still_mapped: Vec<usize> = dropped
            .iter()
            .copied()
            .filter(|id| client.salvage.contains_key(id) || client.drones.contains_key(id))
            .collect();
        let still_alive: Vec<usize> = handles
            .iter()
            .filter(|(_, e)| app.world.is_alive(*e))
            .map(|(id, _)| *id)
            .collect();
        if !still_mapped.is_empty() || !still_alive.is_empty() || !kept_flickered.is_empty() {
            eprintln!(
                "FAIL: removal-by-omission is not working — omitted ids still mapped {still_mapped:?}, \
                 still alive in the world {still_alive:?}; ids that kept arriving but were evicted \
                 at some point anyway {kept_flickered:?}. A client that never evicts only ever gains \
                 entities, which reads as healthy streaming; one that evicts too eagerly makes \
                 in-AOI entities flicker between snapshots."
            );
            return 2;
        }
        println!(
            "eviction ok: {} omitted entities dropped after {:.2} s, {} still-arriving kept for all \
             {frames} frames",
            dropped.len(),
            STALE_TIMEOUT,
            kept.len()
        );
    }

    // ── 3. A disconnect clears the streamed world, parallel maps included ──────────────────────
    //
    // `RemoteEntities::clear` does not own `salvage_buf` / `drone_buf` / `last_seen` — the example's
    // own comment says so. Forgetting one leaks silently: nothing renders from those maps, so the
    // only symptom is memory.
    {
        let (mut app, mut client) = harness();
        let mut net = NetworkSystem::new();
        let ids: Vec<usize> = vec![5, 6, SALVAGE_COUNT + 2];
        let entities: Vec<EntityState> = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                let kind = if id < SALVAGE_COUNT {
                    Kind::Salvage
                } else {
                    Kind::Drone
                };
                at(id, kind, far(i))
            })
            .collect();
        feed(&mut app.world, &snap(1, entities));
        tick(&mut app.world, &mut net, &mut client, DT);
        let handles: Vec<Entity> = client
            .salvage
            .iter()
            .map(|(_, e)| e)
            .chain(client.drones.iter().map(|(_, e)| e))
            .collect();
        if handles.len() != ids.len() {
            eprintln!("FAIL: setup for the disconnect check did not stream all of its entities");
            return 3;
        }

        if let Some(bus) = app.world.resource_mut::<Events<NetworkEvent>>() {
            bus.send(NetworkEvent::Disconnected {
                reason: "self-test".into(),
            });
        }
        tick(&mut app.world, &mut net, &mut client, DT);

        let leaked = handles.iter().filter(|e| app.world.is_alive(**e)).count();
        if streaming(&client) != 0
            || !client.salvage_buf.is_empty()
            || !client.drone_buf.is_empty()
            || !client.last_seen.is_empty()
            || leaked != 0
        {
            eprintln!(
                "FAIL: a disconnect did not clear the streamed world — {} mapped, {} salvage bufs, \
                 {} drone bufs, {} last-seen, {} entities still alive. The parallel maps are not \
                 owned by RemoteEntities::clear and leak in silence.",
                streaming(&client),
                client.salvage_buf.len(),
                client.drone_buf.len(),
                client.last_seen.len(),
                handles.iter().filter(|e| app.world.is_alive(**e)).count()
            );
            return 3;
        }
        println!(
            "disconnect ok: {} entities and every parallel map cleared",
            ids.len()
        );
    }

    // ── 4. Streamed motion is interpolated, not snapped ────────────────────────────────────────
    //
    // The whole reason `SnapshotBuffer` exists. At 12 Hz a client that renders the newest sample
    // instead of interpolating judders — invisible in any single frame, which is why `I`-toggling it
    // in real play is currently the only way anyone would know.
    {
        let (mut app, mut client) = harness();
        let mut net = NetworkSystem::new();
        const ID: usize = 7;
        const SPEED: f32 = 300.0; // px/s along +x
        const FRAMES: usize = 30;
        let origin = far(0);
        let mut newest = origin;

        // Half a second of linear motion at the real snapshot cadence.
        for f in 0..FRAMES {
            if f % SNAP_EVERY == 0 {
                newest = origin + Vec2::new(SPEED * (f as f32 * DT), 0.0);
                feed(
                    &mut app.world,
                    &snap(1 + f as u32, vec![at(ID, Kind::Salvage, newest)]),
                );
            }
            tick(&mut app.world, &mut net, &mut client, DT);
        }

        let drawn = client
            .salvage
            .get(&ID)
            .and_then(|e| app.world.get::<Transform>(e))
            .map(|t| t.position)
            .unwrap_or_default();
        // Where the *sender* was `interp_delay` ago. Stated in the trajectory this test authored,
        // not in the engine's algorithm — the assertion is the property, not a re-derivation.
        //
        // `client_time` advances before events are read, so the snapshot fed on frame `f` is
        // stamped `(f + 1) * DT` while carrying `x = origin + SPEED * f * DT`; over the sampled span
        // the sender's position is therefore `x(t) = origin + SPEED * (t - DT)`. After `FRAMES`
        // ticks the client renders `rt = FRAMES * DT - interp_delay`.
        //
        // Comparing against the newest *snapshot* instead would be wrong and was the first draft's
        // bug: that sample is already up to one snapshot interval old, so the expected lag came out
        // 37.5 px against a measured 17.5 and only passed because the tolerance was loose enough to
        // swallow the difference.
        let rt = FRAMES as f64 * DT as f64 - INTERP_DELAY_DEFAULT;
        let want = origin.x + SPEED * (rt - DT as f64) as f32;
        let slack = SPEED * DT; // one tick of motion, for where the cadence lands
        let lag = newest.x - drawn.x;
        if (drawn.x - want).abs() > slack || lag <= 0.0 {
            eprintln!(
                "FAIL: streamed motion is not interpolated — drawn at x={:.1}, but \
                 {INTERP_DELAY_DEFAULT} s ago the sender was at x={want:.1} (+/- {slack:.1}). The \
                 newest snapshot said x={:.1}; a client that snaps to it rather than interpolating \
                 draws exactly that, for a lag of 0 (measured {lag:.1}).",
                drawn.x, newest.x
            );
            return 4;
        }
        println!(
            "interpolation ok: drawn x={:.1} where the sender was x={want:.1} \
             {INTERP_DELAY_DEFAULT} s ago, {lag:.1} px behind the newest snapshot",
            drawn.x
        );
    }

    // ── 5-6. Against the real server ──────────────────────────────────────────────────────────
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let server_bin = exe_dir.map(|d| {
        d.join(format!(
            "salvage_run_server{}",
            std::env::consts::EXE_SUFFIX
        ))
    });
    let Some(server_bin) = server_bin.filter(|p| p.exists()) else {
        println!(
            "SKIP: salvage_run_server has not been built, so the AOI checks (5-6) did not run. \
             `cargo build --example salvage_run_server` to include them."
        );
        println!("PASS: salvage_run (offline checks only)");
        return 0;
    };

    // A port of our own. Binding :0 asks the OS for a free one; the listener is dropped immediately
    // so the child can take it. That is a race in principle and the right trade in practice — the
    // alternative is the hardcoded 9005, which collides with a server the user is already running.
    let addr = match std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()) {
        Ok(a) => a.to_string(),
        Err(e) => {
            eprintln!("SKIP: could not reserve a local port ({e})");
            println!("PASS: salvage_run (offline checks only)");
            return 0;
        }
    };

    let mut child = match std::process::Command::new(&server_bin)
        .env("SALVAGE_RUN_ADDR", &addr)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("SKIP: could not spawn {}: {e}", server_bin.display());
            println!("PASS: salvage_run (offline checks only)");
            return 0;
        }
    };

    // Wait for the child to bind before connecting. `NetworkClient::connect` dials once and does not
    // retry, so connecting first and hoping is not a slower path — it is a guaranteed failure, and
    // it is how the first draft of this check spent its whole 10 s deadline reporting
    // "Unable to connect" against a server that was up the entire time.
    let bind_deadline = Instant::now() + Duration::from_secs(10);
    let mut bound = false;
    while Instant::now() < bind_deadline {
        if std::net::TcpStream::connect(&addr).is_ok() {
            bound = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if !bound {
        child.kill().ok();
        child.wait().ok();
        eprintln!("SKIP: salvage_run_server never bound {addr} within 10 s");
        println!("PASS: salvage_run (offline checks only)");
        return 0;
    }

    let mut app = App::new();
    app.register_event::<NetworkEvent>();
    let (player, ring) = build_world(&mut app.world);
    let mut client = SalvageClient::new(player, ring);
    let mut net = NetworkSystem::new();
    app.world
        .insert_resource(NetworkClient::connect(&format!("ws://{addr}")));

    /// Run the real system chain for `secs` of **wall clock**, pacing off `Instant`.
    ///
    /// The server steps in real time, so an accumulator clock would drift against it — the trap
    /// `beat_crawler` hit, where `t += 1.0/60.0` made a correct detector look 40% over-firing.
    fn run_live(world: &mut World, net: &mut NetworkSystem, client: &mut SalvageClient, secs: f64) {
        let start = Instant::now();
        let mut last = start;
        while start.elapsed().as_secs_f64() < secs {
            let now = Instant::now();
            let dt = (now - last).as_secs_f32();
            last = now;
            net.run(world, dt);
            client.run(world, dt);
            if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
                bus.flush();
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    }

    let finish = |child: &mut std::process::Child, code: i32, msg: Option<String>| -> i32 {
        child.kill().ok();
        child.wait().ok();
        if let Some(msg) = msg {
            eprintln!("FAIL: {msg}");
        }
        code
    };

    // Connect, with a deadline. The child needs a moment to bind before the socket will take us.
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < connect_deadline && streaming(&client) == 0 {
        run_live(&mut app.world, &mut net, &mut client, 0.25);
    }
    if streaming(&client) == 0 {
        return finish(
            &mut child,
            5,
            Some(format!(
                "never received a snapshot from {addr} in 10 s (status: {})",
                client.status
            )),
        );
    }

    // Park in the dense middle of the world, where an AOI change has entities to find.
    client.player_pos = Vec2::new(WORLD_W * 0.5, WORLD_H * 0.5);

    client.aoi_radius = AOI_RADIUS_MIN;
    run_live(&mut app.world, &mut net, &mut client, 1.5);
    let narrow = streaming(&client);

    client.aoi_radius = AOI_RADIUS_MAX;
    run_live(&mut app.world, &mut net, &mut client, 1.5);
    let wide = streaming(&client);

    if wide <= narrow || narrow >= ENTITY_COUNT {
        return finish(
            &mut child,
            5,
            Some(format!(
                "the server is not tailoring its snapshot to the AOI — radius {AOI_RADIUS_MIN} \
                 streamed {narrow} entities and radius {AOI_RADIUS_MAX} streamed {wide}, out of \
                 {ENTITY_COUNT} in the world. A server that ignored the radius would send the same \
                 count both times, and the whole point of this example would be decorative."
            )),
        );
    }
    println!("AOI ok: radius {AOI_RADIUS_MIN} streams {narrow}, radius {AOI_RADIUS_MAX} streams {wide} of {ENTITY_COUNT}");

    // ── 6. Shrinking the AOI drains what it streamed ──────────────────────────────────────────
    //
    // Check 5 passes with eviction entirely dead: the count only has to *grow*. This one is the
    // failure that flatters — with no evictor the world keeps every entity the wide radius ever
    // streamed, and the HUD climbs toward the total looking like healthy streaming.
    client.aoi_radius = AOI_RADIUS_MIN;
    run_live(&mut app.world, &mut net, &mut client, 2.0);
    let drained = streaming(&client);

    if drained >= wide {
        return finish(
            &mut child,
            6,
            Some(format!(
                "shrinking the AOI back to {AOI_RADIUS_MIN} did not drain the streamed world — \
                 {wide} entities at the wide radius, {drained} after shrinking (it first held \
                 {narrow} at this radius). Entities the server stopped sending are piling up."
            )),
        );
    }
    println!("drain ok: {wide} at the wide radius fell back to {drained} after shrinking");

    finish(&mut child, 0, None);
    println!("PASS: salvage_run");
    0
}

/// WASM entry point (called from an `index.html`, mirroring the other networked examples).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_salvage_run() {
    run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
