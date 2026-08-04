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
//! unit-tested headlessly; this file wires it into ECS systems + rendering. The **wiring** is what
//! `PREDICT_SHOOTER_SELFTEST=1` covers — see [`self_test`] — because a dead reconcile call still
//! looks and feels like a correctly predicted client.
//!
//! Controls: WASD / arrows to move, Space to shoot; the bracket keys live-tune the
//! interpolation delay so its feel can be judged in real play.

use engine::{
    App, DrawText, Events, InputState, KeyCode, NetworkClient, NetworkEvent, NetworkSystem, Scene,
    SnapshotBuffer, Sprite, System, SystemRegistrar, TextQueue, Transform, WindowConfig, World,
};
use glam::Vec2;
use std::collections::{HashMap, HashSet};

#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;

#[path = "client_net.rs"]
mod client_net;
#[path = "protocol.rs"]
mod protocol;

use client_net::Prediction;
use protocol::*;

/// Default interpolation delay (seconds): render remote entities this far in the past so there are
/// snapshots to interpolate between at the ~33 ms snapshot interval. 60 ms (≈2× the snapshot interval)
/// was chosen by real-play feel testing — below ~40 ms bullets ghost/trail badly, and above ~70 ms a
/// bullet lingers at the shooter's old position when moving and firing. Live-tunable at runtime with
/// the bracket keys (see `ShooterClient::interp_delay`) — the one feel parameter automated tests can't judge.
const INTERP_DELAY_DEFAULT: f64 = 0.06;
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
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        world.insert_resource(NetworkClient::connect(&format!("ws://{SERVER_ADDR}")));
        systems.add(NetworkSystem::new());
        systems.add(ShooterClient::new());
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
    /// Per-remote snapshot buffers for interpolation (the promoted `engine::SnapshotBuffer<Vec2>`).
    player_interp: HashMap<usize, SnapshotBuffer<Vec2>>,
    bullet_interp: HashMap<usize, SnapshotBuffer<Vec2>>,
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
                        self.player_interp
                            .entry(p.id)
                            .or_default()
                            .push(self.client_time, Vec2::new(p.x, p.y));
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
                        .push(self.client_time, Vec2::new(b.x, b.y));
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
                "INTERP_DELAY {:.0} ms   ·   [ -10ms   ] +10ms   ·   default {:.0} ms",
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
            .filter_map(|(id, e)| self.player_interp.get(id)?.sample(rt).map(|pos| (e, pos)))
            .collect();
        let bullet_updates: Vec<(engine::Entity, Vec2)> = self
            .bullets
            .iter()
            .filter_map(|(id, e)| self.bullet_interp.get(id)?.sample(rt).map(|pos| (e, pos)))
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
    // `PREDICT_SHOOTER_SELFTEST=1` runs the headless acceptance test instead of opening a window.
    if std::env::var("PREDICT_SHOOTER_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }
    run();
}

// ── Acceptance test ───────────────────────────────────────────────────────────────────────────

/// `PREDICT_SHOOTER_SELFTEST=1 cargo run --example predict_shooter` — asserts the three pillars this
/// example exists to show, none of which a screenshot can see.
///
/// Reconciliation is the load-bearing invisible one, and its failure *flatters*. With the
/// `pred.reconcile(...)` call never reached, input still moves you the instant you press a key and
/// the motion is still perfectly smooth — prediction alone is what makes it feel good. The only
/// symptom is that the server quietly disagrees about where you are: no message says so, nothing
/// renders differently, and in a single-window playtest there is nothing to compare against. That is
/// the `beat_crawler` shape, where a headline feature was not running for several releases.
///
/// `client_net.rs` unit-tests `Prediction` in isolation, so what this closes is the **ECS wiring** —
/// the snapshot handler reaching `reconcile`, and the predicted position reaching the avatar's
/// `Transform`. Each is one line, and a dead one costs nothing visible.
///
/// Checks 1-5 need no server and run anywhere. Checks 6-7 spawn the real `predict_shooter_server` on
/// a port of their own and **SKIP with exit 0** if that binary was never built — the same rule
/// `SALVAGE_RUN_SELFTEST` uses for its server and `BEAT_CRAWLER_SELFTEST` for a missing audio device.
///
/// Exit codes: `0` pass (the live checks may have skipped) · `1` `welcome` does not wire up the local
/// player · `2` input is not predicted locally, or the prediction never reaches the avatar ·
/// `3` an authoritative correction does not reconcile · `4` reconciliation does not replay the
/// un-acked inputs · `5` remote players are snapped rather than interpolated · `6` against a real
/// server, prediction never converges on the authoritative position · `7` a `fire` input never
/// round-trips into a server-spawned bullet.
#[cfg(not(target_arch = "wasm32"))]
fn self_test() -> i32 {
    use engine::{InputAction, InputScript};
    use std::time::{Duration, Instant};

    /// Simulated frame time for the offline checks, deliberately equal to `FIXED_DT` so each tick
    /// feeds the client's fixed-step input loop exactly one input — the sequence numbers the
    /// reconcile checks reason about are then just the tick count. The **server-backed** checks
    /// below must not use it: the server ticks in wall-clock time, so those are paced off `Instant`.
    const DT: f32 = FIXED_DT;
    /// One input's worth of movement along one axis, at full stick.
    const STEP: f32 = MOVE_SPEED * FIXED_DT;
    /// Snapshot cadence in frames — the shared `SNAPSHOT_HZ`, expressed in ticks.
    const SNAP_EVERY: usize = (60 / SNAPSHOT_HZ) as usize;

    let centre = Vec2::new(FIELD_W * 0.5, FIELD_H * 0.5);

    /// The scene's systems, minus the socket. `ShooterClient::new` is the game's own constructor —
    /// a harness that assembled its own client state would stop testing the scene.
    fn harness() -> (App, NetworkSystem, ShooterClient) {
        let mut app = App::new();
        app.register_event::<NetworkEvent>();
        (app, NetworkSystem::new(), ShooterClient::new())
    }

    /// Hand the client a real protocol message, through the real event bus.
    fn feed(world: &mut World, msg: &ServerMsg) {
        let text = serde_json::to_string(msg).expect("encode ServerMsg");
        if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
            bus.send(NetworkEvent::TextMessage(text));
        }
    }

    /// One frame in the scene's order: scripted input where `App` applies it, then the two systems
    /// `ShooterScene::on_enter` registers, then the end-of-frame flush `App` performs.
    fn tick(
        world: &mut World,
        net: &mut NetworkSystem,
        client: &mut ShooterClient,
        script: Option<&mut InputScript>,
        dt: f32,
    ) {
        if let Some(script) = script {
            script.apply(world);
        }
        net.run(world, dt);
        client.run(world, dt);
        if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
            bus.flush();
        }
    }

    /// A script holding one key down from the first frame. `InputState` has no public press setter,
    /// so `InputScript` — the engine's own `ENGINE_INPUT` replay path — is how a headless run
    /// synthesizes held input, and it drives the real `read_input`.
    fn hold(key: KeyCode) -> InputScript {
        InputScript::new([(0, InputAction::KeyDown(key))])
    }

    fn release(key: KeyCode) -> InputScript {
        InputScript::new([(0, InputAction::KeyUp(key))])
    }

    fn snap(tick: u32, players: Vec<PlayerState>, ack: u32) -> ServerMsg {
        ServerMsg::Snap {
            tick,
            players,
            bullets: vec![],
            ack,
        }
    }

    fn player(id: usize, pos: Vec2) -> PlayerState {
        PlayerState {
            id,
            x: pos.x,
            y: pos.y,
        }
    }

    fn pred_pos(client: &ShooterClient) -> Vec2 {
        client
            .prediction
            .as_ref()
            .map(|p| Vec2::new(p.x, p.y))
            .unwrap_or_default()
    }

    fn pending(client: &ShooterClient) -> usize {
        client
            .prediction
            .as_ref()
            .map(|p| p.pending_len())
            .unwrap_or(0)
    }

    fn avatar_pos(world: &World, client: &ShooterClient) -> Option<Vec2> {
        client
            .local_entity
            .and_then(|e| world.get::<Transform>(e))
            .map(|t| t.position)
    }

    // ── 1. `welcome` wires up the local player ────────────────────────────────────────────────
    //
    // Everything downstream is guarded on `if let Some(pred) = &mut self.prediction`, so a welcome
    // that does not land turns the whole example into a silent no-op that still opens a window and
    // still draws a HUD.
    {
        let (mut app, mut net, mut client) = harness();
        feed(&mut app.world, &ServerMsg::Welcome { id: 7 });
        tick(&mut app.world, &mut net, &mut client, None, DT);

        let spawned = avatar_pos(&app.world, &client);
        if client.local_id != Some(7)
            || client.prediction.is_none()
            || spawned.is_none_or(|p| (p - centre).length() > 0.5)
        {
            eprintln!(
                "FAIL: `welcome` did not wire up the local player — local_id {:?} (want Some(7)), \
                 prediction {}, avatar at {spawned:?} (want the field centre {centre:?}). Every \
                 prediction and reconciliation path below is guarded on `self.prediction` being \
                 Some, so this failing makes the rest of the example a no-op that still renders.",
                client.local_id,
                if client.prediction.is_some() {
                    "created"
                } else {
                    "MISSING"
                },
            );
            return 1;
        }
        println!("welcome ok: player #7, prediction seeded at the field centre {centre:?}");
    }

    // ── 2. Held input is predicted immediately, and the prediction reaches the avatar ──────────
    //
    // The first pillar. `client_net.rs` proves `Prediction::predict` integrates correctly; nothing
    // proved the client *calls* it from real input, or that step 3 copies the result onto the
    // avatar's `Transform`. Drop that copy and the player sits at the centre while a perfectly
    // correct prediction advances behind it — rivals, bullets and HUD all still working.
    {
        const FRAMES: usize = 30;
        let (mut app, mut net, mut client) = harness();
        let mut script = hold(KeyCode::KeyD);
        feed(&mut app.world, &ServerMsg::Welcome { id: 1 });
        for _ in 0..FRAMES {
            tick(&mut app.world, &mut net, &mut client, Some(&mut script), DT);
        }

        let want = centre + Vec2::new(STEP * FRAMES as f32, 0.0);
        let pred = pred_pos(&client);
        let drawn = avatar_pos(&app.world, &client).unwrap_or_default();
        if (pred - want).length() > 0.5
            || (drawn - pred).length() > 0.5
            || pending(&client) != FRAMES
        {
            eprintln!(
                "FAIL: input is not predicted onto the avatar — after {FRAMES} frames holding D the \
                 prediction is at {pred:?} and the avatar at {drawn:?}, want both at {want:?} with \
                 {FRAMES} inputs buffered for replay (got {}). A client that predicts but never \
                 writes the Transform leaves the player frozen at the centre; one that never \
                 predicts feels laggy in play but is identical in a still frame.",
                pending(&client)
            );
            return 2;
        }
        println!(
            "prediction ok: {FRAMES} held inputs moved the avatar {:.1} px with no round trip, {} \
             buffered for replay",
            (drawn - centre).length(),
            pending(&client)
        );
    }

    // ── 3. An authoritative correction reconciles ─────────────────────────────────────────────
    //
    // The headline, and what the whole example is exposed to. The server never announces a
    // correction — it just states where you are, in a snapshot that also carries everyone else's
    // position. With `reconcile` unreached the client keeps its own prediction forever, which looks
    // exactly like a correctly-predicted client, because it *is* one: just one the server disagrees
    // with. The key is released first so the tick carrying the snapshot predicts a zero-move input
    // and the reconciled position is the server's exactly.
    {
        const FRAMES: usize = 30;
        let (mut app, mut net, mut client) = harness();
        let mut script = hold(KeyCode::KeyD);
        feed(&mut app.world, &ServerMsg::Welcome { id: 1 });
        for _ in 0..FRAMES {
            tick(&mut app.world, &mut net, &mut client, Some(&mut script), DT);
        }
        let predicted = pred_pos(&client);

        let mut up = release(KeyCode::KeyD);
        tick(&mut app.world, &mut net, &mut client, Some(&mut up), DT);
        // What the client believes is outstanding; acking it drains the replay queue, so the
        // correction lands as a pure snap and check 4 owns the replay half.
        let sent = pending(&client) as u32;

        let authoritative = Vec2::new(centre.x - 120.0, centre.y + 90.0);
        feed(
            &mut app.world,
            &snap(1, vec![player(1, authoritative)], sent),
        );
        tick(&mut app.world, &mut net, &mut client, None, DT);

        let pred = pred_pos(&client);
        let drawn = avatar_pos(&app.world, &client).unwrap_or_default();
        if (pred - authoritative).length() > 0.5 || (drawn - authoritative).length() > 0.5 {
            eprintln!(
                "FAIL: an authoritative correction did not reconcile — the server placed player 1 at \
                 {authoritative:?} with all {sent} inputs acked, but the client is at {pred:?} \
                 (avatar {drawn:?}). It had predicted {predicted:?}; a client whose reconcile call \
                 is never reached stays exactly there, and nothing on screen ever says the server \
                 disagrees."
            );
            return 3;
        }
        println!(
            "reconciliation ok: a correction moved the client {:.1} px off its own prediction onto \
             the server's position",
            (predicted - authoritative).length()
        );
    }

    // ── 4. Reconciliation replays the un-acked inputs ─────────────────────────────────────────
    //
    // What separates reconciliation from snapping. The server's snapshot is always in the past — it
    // has not yet seen the inputs still in flight — so a client that simply adopts the acked
    // position rubber-bands backwards on every snapshot, throwing away the responsiveness
    // prediction just bought. The correction carries a Y lift the client never predicted, so this
    // fails on *both* halves: a naive snap misses the replay, a dead reconcile misses the lift.
    {
        const FRAMES: usize = 30;
        const ACKED: u32 = 10;
        const LIFT: f32 = 50.0;
        let (mut app, mut net, mut client) = harness();
        let mut script = hold(KeyCode::KeyD);
        feed(&mut app.world, &ServerMsg::Welcome { id: 1 });
        for _ in 0..FRAMES {
            tick(&mut app.world, &mut net, &mut client, Some(&mut script), DT);
        }
        let mut up = release(KeyCode::KeyD);
        tick(&mut app.world, &mut net, &mut client, Some(&mut up), DT);

        // Where the server is having applied only the first ACKED inputs, plus a correction on an
        // axis the client never moved along.
        let acked_pos = Vec2::new(centre.x + STEP * ACKED as f32, centre.y + LIFT);
        feed(&mut app.world, &snap(1, vec![player(1, acked_pos)], ACKED));
        tick(&mut app.world, &mut net, &mut client, None, DT);

        // The acked position plus a replay of the un-acked inputs, which are all +x.
        let want = Vec2::new(centre.x + STEP * FRAMES as f32, centre.y + LIFT);
        let pred = pred_pos(&client);
        if (pred - want).length() > 0.5 {
            eprintln!(
                "FAIL: reconciliation did not replay the un-acked inputs — the server acked {ACKED} \
                 of {FRAMES} inputs at {acked_pos:?} and the client settled at {pred:?}, want \
                 {want:?}. Snapping to the acked position alone lands on {acked_pos:?}, which is the \
                 rubber-band; never reconciling at all lands on {:?}, missing the {LIFT} px \
                 correction the client could not have predicted for itself.",
                Vec2::new(centre.x + STEP * FRAMES as f32, centre.y)
            );
            return 4;
        }
        println!(
            "replay ok: acking {ACKED} of {FRAMES} inputs left the client {:.1} px ahead of the \
             server's position while still taking its {LIFT} px correction",
            STEP * (FRAMES as f32 - ACKED as f32)
        );
    }

    // ── 5. Remote players are interpolated, not snapped ───────────────────────────────────────
    //
    // The third pillar, and the reason the server snapshots at SNAPSHOT_HZ well below its 60 Hz sim.
    // A client that renders the newest sample judders at the snapshot rate — invisible in any single
    // frame, and the bracket keys that live-tune the delay are currently the only way anyone would
    // find out.
    {
        const REMOTE: usize = 2;
        const SPEED: f32 = 300.0; // px/s along +x
        const FRAMES: usize = 30;
        let (mut app, mut net, mut client) = harness();
        let origin = Vec2::new(120.0, 480.0);
        let mut newest = origin;

        feed(&mut app.world, &ServerMsg::Welcome { id: 1 });
        for f in 0..FRAMES {
            if f % SNAP_EVERY == 0 {
                newest = origin + Vec2::new(SPEED * (f as f32 * DT), 0.0);
                feed(
                    &mut app.world,
                    &snap(1 + f as u32, vec![player(REMOTE, newest)], 0),
                );
            }
            tick(&mut app.world, &mut net, &mut client, None, DT);
        }

        let drawn = client
            .remote_players
            .get(&REMOTE)
            .and_then(|e| app.world.get::<Transform>(e))
            .map(|t| t.position)
            .unwrap_or_default();
        // Where the *sender* was `interp_delay` ago, stated in the trajectory this check authored
        // rather than re-derived from the engine's algorithm. `client_time` advances before events
        // are read, so the snapshot fed before tick `f` is stamped `(f + 1) * DT` while carrying
        // `x = origin + SPEED * f * DT`; over the sampled span the sender is therefore at
        // `x(t) = origin.x + SPEED * (t - DT)`.
        //
        // Comparing against the newest *snapshot* instead would be wrong — that sample is already up
        // to one snapshot interval old. It is the mistake `SALVAGE_RUN_SELFTEST` records costing it
        // a 17.5 px measurement judged against a 37.5 px expectation.
        let rt = FRAMES as f64 * DT as f64 - INTERP_DELAY_DEFAULT;
        let want = origin.x + SPEED * (rt - DT as f64) as f32;
        let slack = SPEED * DT; // one tick of motion, for where the cadence lands
        let lag = newest.x - drawn.x;
        if (drawn.x - want).abs() > slack || lag <= 0.0 {
            eprintln!(
                "FAIL: a remote player is not interpolated — drawn at x={:.1}, but \
                 {INTERP_DELAY_DEFAULT} s ago the sender was at x={want:.1} (+/- {slack:.1}). The \
                 newest snapshot said x={:.1}; a client that snaps to it rather than interpolating \
                 draws exactly that, for a lag of 0 (measured {lag:.1}).",
                drawn.x, newest.x
            );
            return 5;
        }
        println!(
            "interpolation ok: remote drawn at x={:.1} where the sender was x={want:.1} \
             {INTERP_DELAY_DEFAULT} s ago, {lag:.1} px behind the newest snapshot",
            drawn.x
        );
    }

    // ── 6-7. Against the real server ──────────────────────────────────────────────────────────
    let server_bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .map(|d| {
            d.join(format!(
                "predict_shooter_server{}",
                std::env::consts::EXE_SUFFIX
            ))
        });
    let Some(server_bin) = server_bin.filter(|p| p.exists()) else {
        println!(
            "SKIP: predict_shooter_server has not been built, so the live checks (6-7) did not run. \
             `cargo build --example predict_shooter_server` to include them."
        );
        println!("PASS: predict_shooter (offline checks only)");
        return 0;
    };

    // A port of our own. Binding :0 asks the OS for a free one; the listener is dropped immediately
    // so the child can take it. That is a race in principle and the right trade in practice — the
    // alternative is the hardcoded 9003, which collides with a server the user is already running.
    let addr = match std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()) {
        Ok(a) => a.to_string(),
        Err(e) => {
            eprintln!("SKIP: could not reserve a local port ({e})");
            println!("PASS: predict_shooter (offline checks only)");
            return 0;
        }
    };

    let mut child = match std::process::Command::new(&server_bin)
        .env("PREDICT_SHOOTER_ADDR", &addr)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("SKIP: could not spawn {}: {e}", server_bin.display());
            println!("PASS: predict_shooter (offline checks only)");
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
        eprintln!("SKIP: predict_shooter_server never bound {addr} within 10 s");
        println!("PASS: predict_shooter (offline checks only)");
        return 0;
    }

    /// The most recent authoritative position the wire carried for `id`, read off the event bus
    /// before the client consumes it. Observing the protocol is not reimplementing the client:
    /// `reconcile` is precisely what these checks are about, so the expected value cannot be
    /// derived from it.
    fn observe(world: &World, id: usize) -> Option<Vec2> {
        let bus = world.resource::<Events<NetworkEvent>>()?;
        let mut latest = None;
        for ev in bus.read() {
            let NetworkEvent::TextMessage(text) = ev else {
                continue;
            };
            let Ok(ServerMsg::Snap { players, .. }) = serde_json::from_str::<ServerMsg>(text)
            else {
                continue;
            };
            if let Some(p) = players.iter().find(|p| p.id == id) {
                latest = Some(Vec2::new(p.x, p.y));
            }
        }
        latest
    }

    /// Run the real system chain for `secs` of **wall clock**, paced off `Instant`.
    ///
    /// The server steps in real time, so an accumulator clock would drift against it — the trap
    /// `beat_crawler` hit, where `t += 1.0/60.0` made a correct detector look 40% over-firing.
    /// Returns the last authoritative position seen for `watch`, if a snapshot carried one.
    fn run_live(
        world: &mut World,
        net: &mut NetworkSystem,
        client: &mut ShooterClient,
        script: &mut InputScript,
        secs: f64,
        watch: Option<usize>,
    ) -> Option<Vec2> {
        let start = Instant::now();
        let mut last = start;
        let mut authoritative = None;
        while start.elapsed().as_secs_f64() < secs {
            let now = Instant::now();
            let dt = (now - last).as_secs_f32();
            last = now;
            script.apply(world);
            net.run(world, dt);
            if let Some(id) = watch {
                if let Some(pos) = observe(world, id) {
                    authoritative = Some(pos);
                }
            }
            client.run(world, dt);
            if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
                bus.flush();
            }
            std::thread::sleep(Duration::from_millis(16));
        }
        authoritative
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
    let mut client = ShooterClient::new();
    let mut net = NetworkSystem::new();
    app.world
        .insert_resource(NetworkClient::connect(&format!("ws://{addr}")));

    let mut idle = InputScript::new([]);
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < connect_deadline && client.local_id.is_none() {
        run_live(&mut app.world, &mut net, &mut client, &mut idle, 0.25, None);
    }
    let Some(local_id) = client.local_id else {
        return finish(
            &mut child,
            6,
            Some(format!(
                "never received a welcome from {addr} in 10 s (status: {})",
                client.status
            )),
        );
    };

    // ── 6. Prediction converges on the server's authority ─────────────────────────────────────
    //
    // The client seeds its prediction at the field centre while the server spawns each player at a
    // *random* position — `handle_message` says so: "the first snapshot reconciles to the real
    // spawn". That deliberate initial disagreement is what makes this load-bearing. A client that
    // never reconciles stays anchored to the centre forever, moving smoothly and responsively while
    // the server thinks it is somewhere else entirely, and with one window there is nothing to
    // compare it against.
    let Some(spawn) = run_live(
        &mut app.world,
        &mut net,
        &mut client,
        &mut idle,
        0.5,
        Some(local_id),
    ) else {
        return finish(
            &mut child,
            6,
            Some(format!(
                "the server sent no snapshot carrying player {local_id} within 0.5 s of the welcome"
            )),
        );
    };

    // Drive away from the nearer wall, so 1.2 s of movement is never clamped short of the distance
    // this check asserts. The spawn is random, so picking a fixed direction would flake.
    let start_pos = pred_pos(&client);
    let (key, key_name) = if start_pos.x > centre.x {
        (KeyCode::KeyA, "A")
    } else {
        (KeyCode::KeyD, "D")
    };
    let mut drive = hold(key);
    run_live(&mut app.world, &mut net, &mut client, &mut drive, 1.2, None);

    // Settle with nothing held: with no movement left to replay, reconciliation should leave the
    // client exactly where the server's last word put it.
    let mut up = release(key);
    let settled = run_live(
        &mut app.world,
        &mut net,
        &mut client,
        &mut up,
        0.8,
        Some(local_id),
    );
    let Some(authoritative) = settled else {
        return finish(
            &mut child,
            6,
            Some(format!(
                "the server stopped sending snapshots carrying player {local_id} during the settle \
                 window"
            )),
        );
    };

    let pred = pred_pos(&client);
    let gap = (pred - authoritative).length();
    let travelled = (pred - start_pos).length();
    if gap > 2.0 || travelled < 150.0 {
        return finish(
            &mut child,
            6,
            Some(format!(
                "prediction did not converge on the server — the client is at {pred:?}, the \
                 server's last word was {authoritative:?} ({gap:.1} px apart), and it travelled \
                 {travelled:.1} px from {start_pos:?} while holding {key_name} for 1.2 s. The \
                 prediction is seeded at the field centre and the server spawned this player at \
                 {spawn:?}, so a client that never reconciles simply keeps its own answer and \
                 disagrees forever."
            )),
        );
    }
    println!(
        "convergence ok: spawned at {spawn:?}, {travelled:.1} px of predicted movement settled onto \
         the server's {authoritative:?}, {gap:.2} px apart"
    );

    // ── 7. A `fire` input round-trips into a server-spawned bullet ────────────────────────────
    //
    // Bullets are server authority end to end — the client never spawns one locally, it only
    // receives them in snapshots. So this is the check that closes the loop: the `fire` field leaves
    // the client, the server acts on it, and the result streams back and spawns. Sampled for its
    // peak rather than read at the end, because a bullet's life is short and the player may be
    // firing toward a nearby wall.
    let mut fire = hold(KeyCode::Space);
    let mut seen = 0usize;
    for _ in 0..8 {
        run_live(&mut app.world, &mut net, &mut client, &mut fire, 0.15, None);
        seen = seen.max(client.bullets.len());
    }
    if seen == 0 {
        return finish(
            &mut child,
            7,
            Some(format!(
                "holding fire for 1.2 s produced no bullets — every bullet is server-spawned and \
                 server-integrated, so an empty client means the `fire` field never reached the \
                 server. At a {FIRE_COOLDOWN} s cooldown roughly {} were due.",
                (1.2 / FIRE_COOLDOWN) as u32
            )),
        );
    }
    println!("fire ok: up to {seen} server-spawned bullets streamed back while holding Space");

    finish(&mut child, 0, None);
    println!("PASS: predict_shooter");
    0
}

/// WASM entry point (called from an `index.html`, mirroring `coin_race`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_predict_shooter() {
    run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
