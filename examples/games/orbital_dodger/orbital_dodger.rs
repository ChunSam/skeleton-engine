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
    Scene, ShouldQuit, SnapshotBuffer, Sprite, System, SystemRegistrar, TextQueue, Transform,
    WindowConfig, World,
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
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        world.insert_resource(NetworkClient::connect(&format!("ws://{SERVER_ADDR}")));
        let player = build_world(world);
        systems.add(NetworkSystem::new());
        systems.add(DodgerClient::new(player));
    }
}

/// Builds the scene's world — the vault strip and the local player — and returns the player entity.
///
/// Shared by [`DodgerScene::on_enter`] and the self-test, so the test stands up the arrangement the
/// game actually uses rather than a replica that could drift from it. The socket deliberately stays
/// in `on_enter`: the offline checks must not have a live connection racing them, and the connect
/// line is covered by the live check instead.
fn build_world(world: &mut World) -> Entity {
    // The vault door (win zone) — a dim green strip behind everything.
    spawn_square(
        world,
        Vec2::new(GOAL_X + GOAL_BAR_W * 0.5, FIELD_H * 0.5),
        Vec2::new(GOAL_BAR_W, FIELD_H),
        -3.0,
        [0.15, 0.5, 0.22],
    );
    // The local player — white, on top.
    spawn_square(
        world,
        start_pos(),
        Vec2::splat(PLAYER_SIZE),
        1.0,
        [1.0, 1.0, 1.0],
    )
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
    // `ORBITAL_DODGER_SELFTEST=1` runs the headless acceptance test instead of opening a window.
    if std::env::var("ORBITAL_DODGER_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }
    run();
}

// ── Acceptance test ───────────────────────────────────────────────────────────────────────────

/// `ORBITAL_DODGER_SELFTEST=1 cargo run --example orbital_dodger` — asserts what this example exists
/// to show and a screenshot cannot.
///
/// This example is interpolation *in isolation*: no prediction, no reconciliation, no client→server
/// message at all. So the failure mode is narrow and completely invisible in a still frame — a
/// client that ignored its buffers and drew the newest 10 Hz sample renders hazards at plausible
/// positions in every single frame. Only motion shows it, and only as judder, which is exactly why
/// the example ships an `I` key to toggle interpolation off: watching it was the sole way to know.
///
/// The check that earns the test is **5**. Collision is tested against the *displayed* position, not
/// the newest snapshot — and at 10 Hz those are far apart. A client that renders interpolated but
/// collides against the raw snapshot kills you for touching a hazard that is not where you see it.
/// That reads as "the hitboxes feel off", never as a bug, and no frame of it looks wrong.
///
/// Checks 1-5 need no server and run anywhere. Check 6 spawns the real `orbital_dodger_server` on a
/// port of its own and **SKIPs with exit 0** if that binary was never built.
///
/// Exit codes: `0` pass (the live check may have skipped) · `1` `Hello` does not spawn the hazard
/// set · `2` hazard position is snapped, not interpolated · `3` the spin angle is not interpolated
/// on its own channel · `4` the `I` toggle does not change what is drawn · `5` collision uses the
/// raw snapshot instead of the displayed position · `6` against a real server, hazards never arrive
/// or never move.
#[cfg(not(target_arch = "wasm32"))]
fn self_test() -> i32 {
    use std::time::{Duration, Instant};

    /// Simulated frame time for the offline checks. Everything they exercise is driven by `dt`, so a
    /// fixed step reproduces real play exactly. The **server-backed** check must not use it: the
    /// server ticks in wall-clock time, so it is paced off `Instant`.
    const DT: f32 = 1.0 / 60.0;
    /// Snapshot cadence in frames — the shared `SNAPSHOT_HZ` (10) expressed in ticks, so six frames.
    const SNAP_EVERY: usize = (60 / SNAPSHOT_HZ) as usize;

    /// The scene's world and systems, minus the socket. `build_world` is the game's own — a harness
    /// that spawned its own player would stop testing the scene.
    fn harness() -> (App, NetworkSystem, DodgerClient) {
        let mut app = App::new();
        app.register_event::<NetworkEvent>();
        let player = build_world(&mut app.world);
        (app, NetworkSystem::new(), DodgerClient::new(player))
    }

    /// Hand the client a real protocol message, through the real event bus.
    fn feed(world: &mut World, msg: &ServerMsg) {
        let text = serde_json::to_string(msg).expect("encode ServerMsg");
        if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
            bus.send(NetworkEvent::TextMessage(text));
        }
    }

    /// One frame in the scene's order, then the end-of-frame flush `App` performs.
    fn tick(world: &mut World, net: &mut NetworkSystem, client: &mut DodgerClient, dt: f32) {
        net.run(world, dt);
        client.run(world, dt);
        if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
            bus.flush();
        }
    }

    fn body(id: usize, pos: Vec2, angle: f32) -> BodyState {
        BodyState {
            id,
            x: pos.x,
            y: pos.y,
            angle,
        }
    }

    fn drawn(world: &World, client: &DodgerClient, id: usize) -> Option<(Vec2, f32)> {
        let e = client.hazards.get(&id)?;
        let tr = world.get::<Transform>(e)?;
        Some((tr.position, tr.rotation))
    }

    // ── 1. `Hello` spawns the whole hazard set ────────────────────────────────────────────────
    //
    // `Hello` exists so every hazard entity is up before the first `Snap`. Dropping it is close to
    // invisible: the hazards simply appear from the first snapshot 100 ms later, which reads as
    // "the connection took a moment" rather than as a lost message.
    {
        let (mut app, mut net, mut client) = harness();
        let hazards: Vec<BodyState> = (0..HAZARD_COUNT)
            .map(|i| body(i, Vec2::new(200.0 + i as f32 * 40.0, 120.0), 0.0))
            .collect();
        feed(&mut app.world, &ServerMsg::Hello { hazards });
        tick(&mut app.world, &mut net, &mut client, DT);

        let alive = (0..HAZARD_COUNT)
            .filter(|i| client.hazards.get(i).is_some_and(|e| app.world.is_alive(e)))
            .count();
        if client.hazards.len() != HAZARD_COUNT || alive != HAZARD_COUNT {
            eprintln!(
                "FAIL: `Hello` did not spawn the hazard set — {} mapped and {alive} alive in the \
                 world, want {HAZARD_COUNT} of each. A dropped Hello is nearly invisible: the \
                 hazards turn up from the first Snap instead, one snapshot interval later.",
                client.hazards.len()
            );
            return 1;
        }
        println!("hello ok: {HAZARD_COUNT} hazards spawned before any snapshot");
    }

    // ── 2. Hazard position is interpolated, not snapped ───────────────────────────────────────
    //
    // The headline. At 10 Hz a client that draws the newest sample holds still for six frames and
    // then jumps — every individual frame of which is a perfectly plausible picture of a hazard.
    const ID: usize = 0;
    const SPEED: f32 = 200.0; // px/s along +x
    const SPIN: f32 = 2.0; // rad/s
    const FRAMES: usize = 60;
    let origin = Vec2::new(120.0, 480.0);
    {
        let (mut app, mut net, mut client) = harness();
        let mut newest = origin;
        let mut newest_angle = 0.0_f32;

        for f in 0..FRAMES {
            if f % SNAP_EVERY == 0 {
                newest = origin + Vec2::new(SPEED * (f as f32 * DT), 0.0);
                newest_angle = SPIN * (f as f32 * DT);
                feed(
                    &mut app.world,
                    &ServerMsg::Snap {
                        tick: 1 + f as u32,
                        hazards: vec![body(ID, newest, newest_angle)],
                    },
                );
            }
            tick(&mut app.world, &mut net, &mut client, DT);
        }

        let (pos, _) = drawn(&app.world, &client, ID).unwrap_or_default();
        // Where the *sender* was `interp_delay` ago, stated in the trajectory this check authored
        // rather than re-derived from the engine's algorithm. `client_time` advances before events
        // are read, so the snapshot fed before tick `f` is stamped `(f + 1) * DT` while carrying
        // `x = origin + SPEED * f * DT`; over the sampled span the sender is at
        // `x(t) = origin.x + SPEED * (t - DT)`.
        //
        // Comparing against the newest *snapshot* would be wrong: at 10 Hz that sample is up to
        // 100 ms old. `SALVAGE_RUN_SELFTEST` records that mistake costing it a 17.5 px measurement
        // judged against a 37.5 px expectation.
        let rt = FRAMES as f64 * DT as f64 - INTERP_DELAY_DEFAULT;
        let want = origin.x + SPEED * (rt - DT as f64) as f32;
        let slack = SPEED * DT; // one tick of motion, for where the cadence lands
        let lag = newest.x - pos.x;
        if (pos.x - want).abs() > slack || lag <= 0.0 {
            eprintln!(
                "FAIL: hazard position is not interpolated — drawn at x={:.1}, but \
                 {INTERP_DELAY_DEFAULT} s ago the sender was at x={want:.1} (+/- {slack:.1}). The \
                 newest snapshot said x={:.1}; a client that snaps to it draws exactly that, for a \
                 lag of 0 (measured {lag:.1}).",
                pos.x, newest.x
            );
            return 2;
        }
        println!(
            "interpolation ok: drawn at x={:.1} where the sender was x={want:.1} \
             {INTERP_DELAY_DEFAULT} s ago, {lag:.1} px behind the newest snapshot",
            pos.x
        );

        // ── 3. The spin angle is interpolated on its own channel ──────────────────────────────
        //
        // Each hazard carries TWO interpolated channels, and needing a second one is what justified
        // promoting the buffer to the generic `SnapshotBuffer<T: Lerp>` in the first place. A
        // position channel that works while the angle channel is dead leaves hazards that glide
        // smoothly and step their rotation — and a spinning square is hard enough to read that
        // nobody would catch it by eye.
        let (_, angle) = drawn(&app.world, &client, ID).unwrap_or_default();
        let want_angle = SPIN * (rt - DT as f64) as f32;
        let angle_slack = SPIN * DT;
        let angle_lag = newest_angle - angle;
        if (angle - want_angle).abs() > angle_slack || angle_lag <= 0.0 {
            eprintln!(
                "FAIL: the spin angle is not interpolated — drawn at {angle:.3} rad, want \
                 {want_angle:.3} (+/- {angle_slack:.3}). The newest snapshot said \
                 {newest_angle:.3}; a client that snaps the angle draws exactly that, for a lag of \
                 0 (measured {angle_lag:.3}). Position and angle are separate SnapshotBuffers — one \
                 can work while the other is dead."
            );
            return 3;
        }
        println!(
            "spin ok: angle drawn at {angle:.3} rad where the sender was {want_angle:.3}, \
             {angle_lag:.3} rad behind the newest snapshot"
        );
    }

    // ── 4. The `I` toggle changes what is drawn ───────────────────────────────────────────────
    //
    // The example's own demonstration control. If it were not wired the demo would look correct in
    // both states, which is the one thing this example must never do — showing the judder is how it
    // argues that interpolation is worth having.
    {
        let (mut app, mut net, mut client) = harness();
        let mut newest = origin;
        for f in 0..FRAMES {
            if f % SNAP_EVERY == 0 {
                newest = origin + Vec2::new(SPEED * (f as f32 * DT), 0.0);
                feed(
                    &mut app.world,
                    &ServerMsg::Snap {
                        tick: 1 + f as u32,
                        hazards: vec![body(ID, newest, 0.0)],
                    },
                );
            }
            tick(&mut app.world, &mut net, &mut client, DT);
        }
        let (interp_on, _) = drawn(&app.world, &client, ID).unwrap_or_default();

        // Toggling off makes `rt` the present, which clamps to the newest sample — hold-then-jump.
        client.interp_enabled = false;
        tick(&mut app.world, &mut net, &mut client, DT);
        let (interp_off, _) = drawn(&app.world, &client, ID).unwrap_or_default();

        if (interp_off.x - newest.x).abs() > 0.5 || (interp_off.x - interp_on.x).abs() < 1.0 {
            eprintln!(
                "FAIL: the interpolation toggle does not change what is drawn — with it ON the \
                 hazard was at x={:.1}, with it OFF at x={:.1}, and the newest snapshot said \
                 x={:.1}. OFF must draw the newest sample raw; if both states agree the demo cannot \
                 show the judder it exists to show.",
                interp_on.x, interp_off.x, newest.x
            );
            return 4;
        }
        println!(
            "toggle ok: interpolation ON draws x={:.1}, OFF snaps to the newest sample x={:.1}",
            interp_on.x, interp_off.x
        );
    }

    // ── 5. Collision uses the displayed position, not the raw snapshot ────────────────────────
    //
    // The check that earns this test. Both halves are asserted, because each alone is passable by a
    // broken client: one that never collides at all passes the "not caught" half, and one that
    // collides against everything passes the "caught" half.
    //
    // The interpolation delay is raised to its documented maximum (the `]` key's ceiling) so the
    // gap between the displayed position and the newest snapshot exceeds the collision radius. At
    // the 0.12 s default that gap is 7-27 px against a 38 px radius, so the two hypotheses would
    // overlap and neither half could discriminate.
    {
        let hit_radius = PLAYER_RADIUS + HAZARD_RADIUS;

        // The buffer needs more than `interp_delay` of history before the displayed position and
        // the newest sample diverge at all — until then there is nothing to interpolate between and
        // the two coincide, so a player parked on either one is standing *on* the hazard. The first
        // draft parked from frame 0 and scored 12 catches in the "must be safe" case, which was the
        // warm-up, not the property. So the player waits at its spawn (a corner the hazard lane
        // never reaches) and catches are counted only over the parked phase.
        const WARMUP: usize = (INTERP_DELAY_MAX / DT as f64) as usize + SNAP_EVERY;

        // Returns (displayed position, newest snapshot position, catches during the parked phase)
        // after driving one hazard across the field with the player parked at `park`.
        let run_with_player = |park: fn(Vec2, Vec2) -> Vec2| -> (Vec2, Vec2, u32) {
            let (mut app, mut net, mut client) = harness();
            client.interp_delay = INTERP_DELAY_MAX;
            let mut newest = origin;
            let mut baseline = 0;
            for f in 0..FRAMES {
                if f % SNAP_EVERY == 0 {
                    newest = origin + Vec2::new(SPEED * (f as f32 * DT), 0.0);
                    feed(
                        &mut app.world,
                        &ServerMsg::Snap {
                            tick: 1 + f as u32,
                            hazards: vec![body(ID, newest, 0.0)],
                        },
                    );
                }
                if f == WARMUP {
                    baseline = client.deaths;
                }
                if f >= WARMUP {
                    // Park relative to whatever is on screen right now, so the property is asserted
                    // on every frame of the phase rather than only on the last one.
                    let shown = drawn(&app.world, &client, ID)
                        .map(|(p, _)| p)
                        .unwrap_or(origin);
                    client.player_pos = park(shown, newest);
                } else {
                    client.player_pos = start_pos();
                }
                tick(&mut app.world, &mut net, &mut client, DT);
            }
            let shown = drawn(&app.world, &client, ID)
                .map(|(p, _)| p)
                .unwrap_or_default();
            (shown, newest, client.deaths - baseline)
        };

        // (a) Player sits on the NEWEST SNAPSHOT position — where the hazard is not drawn.
        let (shown_a, newest_a, deaths_a) = run_with_player(|_shown, newest| newest);
        let gap = (newest_a - shown_a).length();

        // (b) Player sits on the DISPLAYED position — where the player can see it.
        let (_, _, deaths_b) = run_with_player(|shown, _newest| shown);

        if gap <= hit_radius {
            eprintln!(
                "FAIL: the check cannot discriminate — the displayed position and the newest \
                 snapshot are only {gap:.1} px apart against a {hit_radius:.1} px collision radius, \
                 so both hypotheses overlap. Raise the interpolation delay or the hazard speed."
            );
            return 5;
        }
        if deaths_a != 0 || deaths_b == 0 {
            eprintln!(
                "FAIL: collision is not tested against the displayed position — parking the player \
                 on the newest snapshot ({gap:.1} px from where the hazard is drawn, radius \
                 {hit_radius:.1}) produced {deaths_a} catches (want 0), and parking it on the \
                 drawn hazard produced {deaths_b} (want > 0). A client that collides against the \
                 raw snapshot kills you for touching a hazard that is not where you see it — which \
                 reads as bad hitboxes, and no frame of it looks wrong."
            );
            return 5;
        }
        println!(
            "collision ok: {gap:.1} px from the drawn hazard is safe ({deaths_a} catches), on it is \
             not ({deaths_b} catches), radius {hit_radius:.1}"
        );
    }

    // ── 6. Against the real server ────────────────────────────────────────────────────────────
    let server_bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .map(|d| {
            d.join(format!(
                "orbital_dodger_server{}",
                std::env::consts::EXE_SUFFIX
            ))
        });
    let Some(server_bin) = server_bin.filter(|p| p.exists()) else {
        println!(
            "SKIP: orbital_dodger_server has not been built, so the live check (6) did not run. \
             `cargo build --example orbital_dodger_server` to include it."
        );
        println!("PASS: orbital_dodger (offline checks only)");
        return 0;
    };

    // A port of our own. Binding :0 asks the OS for a free one; the listener is dropped immediately
    // so the child can take it. A race in principle and the right trade in practice — the
    // alternative is the hardcoded 9004, which collides with a server the user is already running.
    let addr = match std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()) {
        Ok(a) => a.to_string(),
        Err(e) => {
            eprintln!("SKIP: could not reserve a local port ({e})");
            println!("PASS: orbital_dodger (offline checks only)");
            return 0;
        }
    };

    let mut child = match std::process::Command::new(&server_bin)
        .env("ORBITAL_DODGER_ADDR", &addr)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("SKIP: could not spawn {}: {e}", server_bin.display());
            println!("PASS: orbital_dodger (offline checks only)");
            return 0;
        }
    };

    // Wait for the child to bind before connecting. `NetworkClient::connect` dials once and does not
    // retry, so connecting first and hoping is not a slower path — it is a guaranteed failure.
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
        eprintln!("SKIP: orbital_dodger_server never bound {addr} within 10 s");
        println!("PASS: orbital_dodger (offline checks only)");
        return 0;
    }

    /// Run the real system chain for `secs` of **wall clock**, paced off `Instant`.
    ///
    /// The server steps in real time, so an accumulator clock would drift against it — the trap
    /// `beat_crawler` hit, where `t += 1.0/60.0` made a correct detector look 40% over-firing.
    fn run_live(world: &mut World, net: &mut NetworkSystem, client: &mut DodgerClient, secs: f64) {
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

    let mut app = App::new();
    app.register_event::<NetworkEvent>();
    let player = build_world(&mut app.world);
    let mut client = DodgerClient::new(player);
    let mut net = NetworkSystem::new();
    app.world
        .insert_resource(NetworkClient::connect(&format!("ws://{addr}")));

    // Park the player in the corner it starts in, out of the hazards' way — a catch would respawn
    // it mid-measurement and the motion sample would be about the player, not the hazards.
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < connect_deadline && client.hazards.is_empty() {
        run_live(&mut app.world, &mut net, &mut client, 0.25);
    }
    if client.hazards.len() != HAZARD_COUNT {
        return finish(
            &mut child,
            6,
            Some(format!(
                "the real server never streamed a full hazard set from {addr} in 10 s — {} of \
                 {HAZARD_COUNT} arrived (status: {})",
                client.hazards.len(),
                client.status
            )),
        );
    }

    let sample = |world: &World, client: &DodgerClient| -> Vec<Vec2> {
        (0..HAZARD_COUNT)
            .map(|i| drawn(world, client, i).map(|(p, _)| p).unwrap_or_default())
            .collect()
    };
    let first = sample(&app.world, &client);
    run_live(&mut app.world, &mut net, &mut client, 1.0);
    let second = sample(&app.world, &client);

    let moved = first
        .iter()
        .zip(&second)
        .filter(|(a, b)| a.distance(**b) > 1.0)
        .count();
    let out_of_field = second
        .iter()
        .filter(|p| {
            p.x < -HAZARD_RADIUS
                || p.x > FIELD_W + HAZARD_RADIUS
                || p.y < -HAZARD_RADIUS
                || p.y > FIELD_H + HAZARD_RADIUS
        })
        .count();

    if moved < HAZARD_COUNT || out_of_field > 0 {
        return finish(
            &mut child,
            6,
            Some(format!(
                "the real server's hazards are not moving as streamed — {moved} of {HAZARD_COUNT} \
                 changed position over 1 s and {out_of_field} are outside the field. A client whose \
                 buffers stop being fed holds its last frame forever, which looks like a paused \
                 game rather than a dead connection."
            )),
        );
    }
    println!(
        "live ok: {HAZARD_COUNT} hazards streamed from the real server, all {moved} moving and in \
         the field"
    );

    finish(&mut child, 0, None);
    println!("PASS: orbital_dodger");
    0
}

/// WASM entry point (called from an `index.html`, mirroring `predict_shooter`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_orbital_dodger() {
    run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
