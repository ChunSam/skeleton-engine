//! `netplay_game` — the networked genre-game of the rebuilt examples tree.
//!
//! Phase 5 of `plans/2026-08-19-examples-rebuild-plan.md`, and the game that folds the four
//! deleted networked examples (`coin_race`, `predict_shooter`, `orbital_dodger`, `salvage_run`)
//! into one client + one authoritative server.
//!
//! ```text
//! cargo run --example netplay_server                      # terminal 1 — the authority
//! cargo run --example netplay_game                        # terminal 2 — a pilot
//! cargo run --example netplay_game                        # terminal 3 — a rival
//! NETPLAY_SELFTEST=1 cargo run --example netplay_game     # the acceptance test (headless)
//! ```
//!
//! # Why one game replaced four
//!
//! The four deleted games differed by *which* netcode technique they showed, not by genre — they
//! were all "squares move around a field". Merging them is only honest if the merge keeps every
//! technique, and the way to guarantee that is to put each one on a **different kind of object**,
//! so that dropping one is a visibly different game rather than a quietly missing code path:
//!
//! - **Your ship** is server-authoritative but **predicted** locally. It moves the instant you
//!   press a key; when the server disagrees, [`Prediction::reconcile`] replays the inputs the
//!   server has not seen yet instead of snapping. (`predict_shooter`)
//! - **Hazard drones** are server-owned and only ever **interpolated** — drawn where the server
//!   was [`INTERP_DELAY`] ago, so 12 Hz snapshots look like continuous motion. Collision reads the
//!   *drawn* position, not the newest snapshot. (`orbital_dodger`)
//! - **Salvage pickups** are **claimed, not taken**. Touching one sends a request and changes
//!   nothing locally; the pickup disappears when — and only when — the server says
//!   [`ServerMsg::Taken`]. (`coin_race`)
//! - **The set of objects you are told about at all** is an **area of interest** around your ship.
//!   Entities stream in as you fly, and stream out by simply not being mentioned again.
//!   (`salvage_run`)
//!
//! # The one thing to understand before reading the code
//!
//! Three different positions exist for the same object and confusing them is the entire genre of
//! netcode bug:
//!
//! | | what it is | who reads it |
//! |---|---|---|
//! | **authoritative** | where the server says it is | nobody, directly — it arrives in snapshots |
//! | **newest snapshot** | the last thing the server said | the interpolation buffer, as input |
//! | **displayed** | where it is *drawn*, [`INTERP_DELAY`] in the past | the renderer **and collision** |
//!
//! Collision using the newest snapshot while the eye sees the displayed position is the bug that
//! reads as "the hitboxes are bad" and looks perfectly fine in every screenshot. This game keeps
//! `displayed` explicitly, and check 4 measures that the two are far enough apart for it to
//! matter (they are — at 12 Hz, tens of pixels against a 16 px hazard).
//!
//! # Controls
//!
//! `WASD`/arrows fly · `-`/`=` shrink/grow the area of interest · `R` reconnect · `Esc` quit.

use engine::{
    App, Camera, DebugDraw, DrawText, Entity, Events, InputState, KeyCode, NetworkClient,
    NetworkEvent, NetworkSystem, RemoteEntities, ShouldQuit, SnapshotBuffer, Sprite, System,
    TextQueue, Transform, WindowConfig, World,
};
use glam::Vec2;
use std::collections::{HashMap, HashSet, VecDeque};

#[path = "protocol.rs"]
mod protocol;
use protocol::*;

// ── Client-side prediction ──────────────────────────────────────────────────────────────────────

/// One locally-applied input, retained until the server acks it so it can be replayed.
#[derive(Clone, Copy, Debug)]
struct PendingInput {
    seq: u32,
    mx: f32,
    my: f32,
}

/// Local-player prediction + server reconciliation.
///
/// [`predict`](Self::predict) applies an input immediately, so the ship moves on the frame you
/// pressed the key rather than one round trip later, and buffers it.
/// [`reconcile`](Self::reconcile) snaps to the server's authoritative position *as of the acked
/// input* and replays everything the server has not processed yet.
///
/// That replay is the whole difference between "prediction" and "lag compensation theatre". Snap
/// to the server position without replaying and the ship jerks backwards by however many inputs
/// are in flight, every single snapshot — the rubber-band. The unit tests below pin both halves,
/// but the wiring is what actually breaks: check 3 exists because a client can predict perfectly
/// and simply never call `reconcile`, which *feels* right in one window and silently disagrees
/// with the server forever.
struct Prediction {
    pos: Vec2,
    next_seq: u32,
    pending: VecDeque<PendingInput>,
}

impl Prediction {
    fn new(pos: Vec2) -> Self {
        Self {
            pos,
            next_seq: 1,
            pending: VecDeque::new(),
        }
    }

    /// Applies an input locally and records it for replay. Returns its `seq`.
    fn predict(&mut self, mx: f32, my: f32) -> u32 {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        let (x, y) = step_position(self.pos.x, self.pos.y, mx, my);
        self.pos = Vec2::new(x, y);
        self.pending.push_back(PendingInput { seq, mx, my });
        seq
    }

    /// Reconciles to the authoritative position acked at input `ack`: drops acked inputs, then
    /// replays the remaining ones from the server's position using the same [`step_position`] the
    /// server used. The result is where continuous local prediction *would* have reached had it
    /// started from the truth.
    fn reconcile(&mut self, server: Vec2, ack: u32) {
        while let Some(front) = self.pending.front() {
            if front.seq <= ack {
                self.pending.pop_front();
            } else {
                break;
            }
        }
        let mut p = server;
        for input in &self.pending {
            let (x, y) = step_position(p.x, p.y, input.mx, input.my);
            p = Vec2::new(x, y);
        }
        self.pos = p;
    }

    fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Forgets everything — used on reconnect, where seq numbering restarts and replaying inputs
    /// the new server never saw would corrupt the first reconciliation.
    fn reset(&mut self, pos: Vec2) {
        self.pos = pos;
        self.next_seq = 1;
        self.pending.clear();
    }
}

// ── World ───────────────────────────────────────────────────────────────────────────────────────

const WINDOW_W: u32 = VIEW_W as u32;
const WINDOW_H: u32 = VIEW_H as u32;

/// Where a ship starts, before the first snapshot corrects it. Matches the server's spawn, so the
/// first reconciliation is a no-op in the common case rather than a visible jump.
fn start_pos() -> Vec2 {
    Vec2::new(WORLD_W * 0.5, WORLD_H * 0.5)
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

/// A static checkerboard spanning the world, so the scrolling camera has visible texture and "the
/// world is much larger than the window" is legible at a glance.
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

/// Spawns the background and the local ship. Returns the ship entity.
///
/// Everything else in the world arrives over the network, which is the point — the selftest calls
/// *this* function rather than building its own field, so a harness cannot drift from the game.
fn build_world(world: &mut World) -> Entity {
    spawn_background(world);
    spawn_square(
        world,
        start_pos(),
        Vec2::splat(PLAYER_RADIUS * 2.0),
        1.0,
        [0.35, 0.85, 1.0],
    )
}

/// Cool cyan for salvage (collect me), warm amber for hazards (do not touch me), violet for other
/// pilots. Colour by kind, varied slightly by id so a cluster is readable as several objects.
fn entity_color(kind: Kind, id: u32) -> [f32; 3] {
    let jitter = (id % 5) as f32 * 0.04;
    match kind {
        Kind::Pickup => [0.20 + jitter, 0.80, 0.70 + jitter],
        Kind::Hazard => [0.95, 0.55 - jitter, 0.20 + jitter],
        Kind::Player => [0.72 + jitter, 0.45, 0.95],
    }
}

fn kind_radius(kind: Kind) -> f32 {
    match kind {
        Kind::Pickup => PICKUP_RADIUS,
        Kind::Hazard => HAZARD_RADIUS,
        Kind::Player => PLAYER_RADIUS,
    }
}

fn read_move(world: &World) -> (f32, f32) {
    let Some(input) = world.resource::<InputState>() else {
        return (0.0, 0.0);
    };
    let mut mx = 0.0;
    let mut my = 0.0;
    if input.is_pressed(KeyCode::KeyA) || input.is_pressed(KeyCode::ArrowLeft) {
        mx -= 1.0;
    }
    if input.is_pressed(KeyCode::KeyD) || input.is_pressed(KeyCode::ArrowRight) {
        mx += 1.0;
    }
    if input.is_pressed(KeyCode::KeyW) || input.is_pressed(KeyCode::ArrowUp) {
        my -= 1.0;
    }
    if input.is_pressed(KeyCode::KeyS) || input.is_pressed(KeyCode::ArrowDown) {
        my += 1.0;
    }
    (mx, my)
}

// ── The client ──────────────────────────────────────────────────────────────────────────────────

/// The whole game, as one system.
///
/// It holds three parallel maps keyed by [`NetId`], and their split is deliberate:
/// [`RemoteEntities`] owns the `id → Entity` lifecycle, `buffers` owns the position history the
/// renderer interpolates, and `last_seen` owns eviction. They must be kept in step by hand — which
/// is exactly the bookkeeping check 2's second half exists to catch, because dropping an entity
/// from one map and not the others leaks silently.
struct NetplayClient {
    // Identity + connection.
    player_id: u32,
    status: String,
    pickups_total: u32,

    // The local, predicted ship.
    ship: Entity,
    prediction: Prediction,
    /// Accumulates real time toward the next [`INPUT_DT`] input tick.
    input_accum: f32,
    /// The last authoritative position the server sent, for the HUD's divergence readout.
    server_pos: Vec2,
    /// Whether a snapshot has ever arrived — before that, "diverged by 0 px" would be a lie.
    have_authority: bool,

    // The streamed world.
    remotes: RemoteEntities<NetId>,
    buffers: HashMap<NetId, SnapshotBuffer<Vec2>>,
    last_seen: HashMap<NetId, f64>,
    /// Where each streamed entity is **drawn** this frame. Collision reads this, never the newest
    /// snapshot — see the table in the module docs.
    displayed: HashMap<NetId, Vec2>,
    /// The newest snapshot position, kept only so the HUD and check 4 can compare it against
    /// `displayed`. Nothing in the game logic may read this.
    newest: HashMap<NetId, Vec2>,

    // Claim/confirm.
    /// Pickups we have asked for and not yet heard a verdict on. Prevents re-asking every frame;
    /// it is emphatically **not** a local deletion.
    claim_sent: HashSet<u32>,
    /// Every pickup the server has announced as taken, by anyone.
    ///
    /// `Taken` is broadcast to all clients, so this is each client's independent record of what
    /// the world lost. Two clients' copies must agree, and the sum of their scores must equal its
    /// size — which is the invariant check 6 asserts, and the only thing that catches a server
    /// that granted one pickup twice.
    taken_seen: HashSet<u32>,
    score: u32,
    /// Hazards currently overlapping the ship, so a hit counts once on entry rather than per frame.
    touching: HashSet<u32>,
    hits: u32,

    // Presentation.
    aoi_radius: f32,
    clock: f64,
    hit_flash: f32,
}

impl NetplayClient {
    fn new(ship: Entity) -> Self {
        Self {
            player_id: 0,
            status: format!("Connecting to {} ...", server_addr()),
            pickups_total: PICKUP_COUNT,
            ship,
            prediction: Prediction::new(start_pos()),
            input_accum: 0.0,
            server_pos: start_pos(),
            have_authority: false,
            remotes: RemoteEntities::new(),
            buffers: HashMap::new(),
            last_seen: HashMap::new(),
            displayed: HashMap::new(),
            newest: HashMap::new(),
            claim_sent: HashSet::new(),
            taken_seen: HashSet::new(),
            score: 0,
            touching: HashSet::new(),
            hits: 0,
            aoi_radius: AOI_RADIUS_DEFAULT,
            clock: 0.0,
            hit_flash: 0.0,
        }
    }

    // ── Inbound ─────────────────────────────────────────────────────────────────────────────────

    fn handle_message(&mut self, world: &mut World, text: &str) {
        if text.len() > MAX_MESSAGE_BYTES {
            self.status = format!("Dropped an oversized message ({} bytes)", text.len());
            return;
        }
        match ron::from_str::<ServerMsg>(text) {
            Ok(ServerMsg::Welcome {
                player_id,
                pickups_total,
                ..
            }) => {
                self.player_id = player_id;
                self.pickups_total = pickups_total;
                self.status = format!("Pilot {player_id} — collect salvage, dodge the drones");
            }
            Ok(ServerMsg::Snap {
                ack,
                x,
                y,
                entities,
                ..
            }) => {
                self.server_pos = Vec2::new(x, y);
                self.have_authority = true;
                // Reconcile FIRST, then ingest: both read `self.clock`, and reconciliation moves
                // the ship the AOI is centred on. Ingesting first would stamp this snapshot's
                // entities against a stale ship position for one frame.
                self.prediction.reconcile(self.server_pos, ack);
                self.ingest(world, &entities);
            }
            Ok(ServerMsg::Taken { id, by, score }) => self.on_taken(world, id, by, score),
            Ok(ServerMsg::Left { player_id }) => {
                self.forget(world, (Kind::Player, player_id));
            }
            Err(err) => self.status = format!("Protocol error: {err}"),
        }
    }

    /// Spawns/refreshes the entities in one AOI snapshot, stamping each interpolation buffer at
    /// the current client clock and refreshing its last-seen time.
    ///
    /// Note what this does **not** do: it never removes anything. Removal is
    /// [`evict_stale`](Self::evict_stale) (the AOI left) or [`on_taken`](Self::on_taken) (the
    /// server said so), and keeping those separate is what makes each one assertable on its own.
    fn ingest(&mut self, world: &mut World, entities: &[EntityState]) {
        for ent in entities {
            let key = ent.net_id();
            let pos = Vec2::new(ent.x, ent.y);

            let color = entity_color(ent.kind, ent.id);
            let scale = Vec2::splat(kind_radius(ent.kind) * 2.0);
            let z = match ent.kind {
                Kind::Pickup => 0.0,
                Kind::Hazard => 0.5,
                Kind::Player => 0.9,
            };
            self.remotes
                .get_or_spawn(world, key, |w| spawn_square(w, pos, scale, z, color));

            self.buffers
                .entry(key)
                .or_insert_with(|| SnapshotBuffer::with_capacity(12))
                .push(self.clock, pos);
            self.last_seen.insert(key, self.clock);
            self.newest.insert(key, pos);
        }
    }

    /// The server granted a claim. **This is the only thing that removes a pickup.**
    ///
    /// A client that deleted the pickup on touch would never need this message, would look
    /// *better* in single player (instant, no round trip), and would be wrong the moment two
    /// pilots reach for the same salvage — which is what check 5 pins offline and check 6 pins
    /// against a real server with two clients.
    fn on_taken(&mut self, world: &mut World, id: u32, by: u32, score: u32) {
        self.forget(world, (Kind::Pickup, id));
        self.claim_sent.remove(&id);
        self.taken_seen.insert(id);
        if by == self.player_id {
            self.score = score;
        }
    }

    /// Drops one streamed entity from **every** map. One place, so the three cannot drift.
    fn forget(&mut self, world: &mut World, key: NetId) {
        self.remotes.remove(world, &key);
        self.buffers.remove(&key);
        self.last_seen.remove(&key);
        self.displayed.remove(&key);
        self.newest.remove(&key);
        if key.0 == Kind::Hazard {
            self.touching.remove(&key.1);
        }
    }

    /// Removal-by-omission: anything not mentioned for [`AOI_EVICT_SECS`] has left the area of
    /// interest, because the server never says so explicitly.
    ///
    /// The failure this guards is *flattering*: with eviction dead the world only ever gains
    /// entities, so the HUD's "streaming N" climbs impressively and every frame looks great while
    /// memory fills with objects that are nowhere near you.
    fn evict_stale(&mut self, world: &mut World) {
        let cutoff = self.clock - AOI_EVICT_SECS;
        let stale: Vec<NetId> = self
            .last_seen
            .iter()
            .filter(|(_, &seen)| seen < cutoff)
            .map(|(&key, _)| key)
            .collect();
        for key in stale {
            self.forget(world, key);
        }
    }

    // ── Per-frame ───────────────────────────────────────────────────────────────────────────────

    /// Samples every buffer at `clock - INTERP_DELAY` and writes the result to both the entity's
    /// `Transform` (what you see) and `displayed` (what collision reads). Those must be the same
    /// number, which is why one function writes both.
    fn apply_display(&mut self, world: &mut World) {
        let render_time = self.clock - INTERP_DELAY;
        for (&key, buffer) in self.buffers.iter() {
            let Some(pos) = buffer.sample(render_time) else {
                continue;
            };
            self.displayed.insert(key, pos);
            if let Some(entity) = self.remotes.get(&key) {
                if let Some(tr) = world.get_mut::<Transform>(entity) {
                    tr.position = pos;
                }
            }
        }
    }

    /// Accumulates real time and emits at most one input per [`INPUT_DT`], predicting each one.
    ///
    /// The accumulator is what keeps prediction reproducible: a client that predicted per *frame*
    /// would apply a different number of steps than the server applies inputs, and reconciliation
    /// would fight that difference forever. See rule 2 in `protocol.rs`.
    fn pump_input(&mut self, world: &mut World, dt: f32) {
        let (mx, my) = read_move(world);
        self.input_accum += dt;
        while self.input_accum >= INPUT_DT {
            self.input_accum -= INPUT_DT;
            let seq = self.prediction.predict(mx, my);
            self.send(
                world,
                &ClientMsg::Input {
                    seq,
                    mx,
                    my,
                    r: self.aoi_radius,
                },
            );
        }
        if let Some(tr) = world.get_mut::<Transform>(self.ship) {
            tr.position = self.prediction.pos;
        }
    }

    /// Asks for every pickup the ship is touching — by its **displayed** position, the one you can
    /// actually see. Removes nothing.
    fn pump_claims(&mut self, world: &mut World) {
        let ship = self.prediction.pos;
        let wanted: Vec<u32> = self
            .displayed
            .iter()
            .filter(|((kind, _), _)| *kind == Kind::Pickup)
            .filter(|((_, id), pos)| {
                !self.claim_sent.contains(id) && ship.distance(**pos) <= CLAIM_RADIUS
            })
            .map(|((_, id), _)| *id)
            .collect();
        for id in wanted {
            self.claim_sent.insert(id);
            self.send(world, &ClientMsg::Claim { id });
        }
    }

    /// Counts a hazard hit on *entry*, against the drawn position.
    ///
    /// Reading `newest` here instead would be the `orbital_dodger` bug: you would be hit by a
    /// drone that is visibly elsewhere, which presents as bad hitboxes and looks correct in every
    /// screenshot. Check 4 measures how far apart the two are.
    fn pump_hazards(&mut self) {
        let ship = self.prediction.pos;
        let reach = PLAYER_RADIUS + HAZARD_RADIUS;
        let mut now_touching = HashSet::new();
        for ((kind, id), pos) in self.displayed.iter() {
            if *kind != Kind::Hazard {
                continue;
            }
            if ship.distance(*pos) <= reach {
                now_touching.insert(*id);
                if !self.touching.contains(id) {
                    self.hits += 1;
                    self.hit_flash = 0.35;
                }
            }
        }
        self.touching = now_touching;
    }

    // ── Outbound ────────────────────────────────────────────────────────────────────────────────

    /// Sends a message if a socket exists. Offline (the selftest's device-free checks) this is a
    /// no-op, which is deliberate: every check that can run without a server should.
    fn send(&self, world: &World, msg: &ClientMsg) {
        let Some(client) = world.resource::<NetworkClient>() else {
            return;
        };
        if let Ok(text) = ron::to_string(msg) {
            client.send_text(text);
        }
    }

    // ── Lifecycle ───────────────────────────────────────────────────────────────────────────────

    /// Drops the whole streamed world. Called on disconnect: those entities describe a server we
    /// are no longer talking to, and leaving them on screen is worse than an empty field because
    /// they look live.
    fn drop_world(&mut self, world: &mut World) {
        self.remotes.clear(world);
        self.buffers.clear();
        self.last_seen.clear();
        self.displayed.clear();
        self.newest.clear();
        self.claim_sent.clear();
        self.taken_seen.clear();
        self.touching.clear();
        self.have_authority = false;
    }

    fn reconnect(&mut self, world: &mut World) {
        self.drop_world(world);
        self.prediction.reset(start_pos());
        self.score = 0;
        self.hits = 0;
        self.player_id = 0;
        let addr = server_addr();
        self.status = format!("Reconnecting to {addr} ...");
        world.insert_resource(NetworkClient::connect(&format!("ws://{addr}")));
        // The join goes out before the socket is open on purpose — see `ClientMsg::Join`.
        self.send(world, &ClientMsg::Join);
    }

    // ── HUD ─────────────────────────────────────────────────────────────────────────────────────

    fn draw_hud(&self, world: &mut World) {
        let ship = self.prediction.pos;

        if let Some(dd) = world.resource_mut::<DebugDraw>() {
            // The area of interest, as a ring you can grow and shrink.
            dd.circle(ship, self.aoi_radius, [90, 200, 255, 90]);
            // The world edge, so "the world is bigger than the window" is visible.
            dd.rect(Vec2::ZERO, Vec2::new(WORLD_W, WORLD_H), [70, 90, 130, 120]);
            if self.have_authority {
                // Where the server thinks you are. Normally invisible under the ship; it separates
                // visibly under load, which is prediction doing its job.
                dd.circle(self.server_pos, PLAYER_RADIUS + 4.0, [255, 230, 120, 130]);
            }
            if self.hit_flash > 0.0 {
                let a = (self.hit_flash / 0.35 * 200.0) as u8;
                dd.circle(ship, PLAYER_RADIUS * 2.2, [255, 90, 70, a]);
            }
        }

        let streaming = self.remotes.len();
        let pickups = self
            .displayed
            .keys()
            .filter(|(k, _)| *k == Kind::Pickup)
            .count();
        let hazards = self
            .displayed
            .keys()
            .filter(|(k, _)| *k == Kind::Hazard)
            .count();
        let pilots = self
            .displayed
            .keys()
            .filter(|(k, _)| *k == Kind::Player)
            .count();
        let diverged = if self.have_authority {
            ship.distance(self.server_pos)
        } else {
            0.0
        };

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        tq.push(DrawText::new(
            "WASD fly   -/= area of interest   R reconnect   Esc quit",
            Vec2::new(16.0, 14.0),
            17.0,
            [224, 236, 250, 230],
        ));
        tq.push(DrawText::new(
            format!(
                "salvage {}/{}   hits {}   streaming {streaming} ({pickups} salvage / {hazards} \
                 hazards / {pilots} pilots)   aoi {:.0}   unacked {}   diverged {diverged:.1} px",
                self.score,
                COLLECT_GOAL,
                self.hits,
                self.aoi_radius,
                self.prediction.pending_len(),
            ),
            Vec2::new(16.0, 38.0),
            17.0,
            [168, 208, 255, 220],
        ));
        tq.push(DrawText::new(
            self.status.clone(),
            Vec2::new(16.0, 62.0),
            16.0,
            [150, 180, 210, 200],
        ));
        if self.score >= COLLECT_GOAL {
            tq.push(DrawText::new(
                "hold complete — press R to fly another run",
                Vec2::new(16.0, 88.0),
                20.0,
                [140, 255, 190, 240],
            ));
        }
    }
}

impl System for NetplayClient {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.clock += dt as f64;
        self.hit_flash = (self.hit_flash - dt).max(0.0);

        // 0. Keys that are not movement. Read them all first so the InputState borrow is released
        //    before anything below mutates the world.
        let (aoi_dn, aoi_up, do_reconnect, do_quit) = match world.resource::<InputState>() {
            Some(input) => (
                input.just_pressed(KeyCode::Minus),
                input.just_pressed(KeyCode::Equal),
                input.just_pressed(KeyCode::KeyR),
                input.just_pressed(KeyCode::Escape),
            ),
            None => (false, false, false, false),
        };
        if aoi_dn {
            self.aoi_radius = (self.aoi_radius - AOI_RADIUS_STEP).max(AOI_RADIUS_MIN);
        }
        if aoi_up {
            self.aoi_radius = (self.aoi_radius + AOI_RADIUS_STEP).min(AOI_RADIUS_MAX);
        }
        if do_reconnect {
            self.reconnect(world);
        }
        if do_quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.0 = true;
            }
        }

        // 1. Drain the network. Copied out first — `handle_message` needs `&mut World`, and the
        //    bus lives in it.
        let events: Vec<NetworkEvent> = world
            .resource::<Events<NetworkEvent>>()
            .map(|bus| bus.read().to_vec())
            .unwrap_or_default();
        for ev in events {
            match ev {
                NetworkEvent::Connected => {
                    self.status = "Connected — waiting for the first snapshot".into();
                }
                NetworkEvent::TextMessage(ref text) => self.handle_message(world, text),
                NetworkEvent::Disconnected { reason } => {
                    self.status = format!("Disconnected: {reason} (is netplay_server running?)");
                    self.drop_world(world);
                }
                NetworkEvent::Error(e) => self.status = format!("Error: {e}"),
                NetworkEvent::MessageTooLarge { len, limit } => {
                    self.status = format!("Server sent {len} bytes, over the {limit} limit");
                }
                NetworkEvent::ReceiveQueueFull { dropped, capacity } => {
                    self.status =
                        format!("Dropped {dropped} events — the {capacity}-event queue filled");
                }
                _ => {}
            }
        }

        // 2. Predict + send input, then evict what the AOI stopped mentioning, then decide where
        //    everything is drawn. Order matters: eviction before display, or a just-evicted
        //    entity gets one last frame at a stale position.
        self.pump_input(world, dt);
        self.evict_stale(world);
        self.apply_display(world);

        // 3. Gameplay reads the *displayed* world.
        self.pump_claims(world);
        self.pump_hazards();

        // 4. Camera follows the predicted ship, clamped to the world.
        if let Some(cam) = world.resource_mut::<Camera>() {
            let half = Vec2::new(VIEW_W, VIEW_H) * 0.5;
            let top_left = self.prediction.pos - half;
            cam.position = Vec2::new(
                top_left.x.clamp(0.0, (WORLD_W - VIEW_W).max(0.0)),
                top_left.y.clamp(0.0, (WORLD_H - VIEW_H).max(0.0)),
            );
        }

        self.draw_hud(world);
    }

    fn name(&self) -> &'static str {
        "NetplayClient"
    }
}

// ── Wiring ──────────────────────────────────────────────────────────────────────────────────────

/// Builds the app **without** a socket. The selftest's device-free checks call this and drive the
/// same systems the game runs; [`run`] adds the connection on top.
fn build_app() -> App {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — netplay_game".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        ..Default::default()
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.register_event::<NetworkEvent>();

    let ship = build_world(&mut app.world);
    app.add_system(NetworkSystem::new());
    app.add_system(NetplayClient::new(ship));
    app
}

fn run() {
    let mut app = build_app();
    let addr = server_addr();
    app.world
        .insert_resource(NetworkClient::connect(&format!("ws://{addr}")));
    // Sent before the socket is open — ordinary code on both targets since v0.150.2. See
    // `ClientMsg::Join`.
    if let Some(client) = app.world.resource::<NetworkClient>() {
        if let Ok(text) = ron::to_string(&ClientMsg::Join) {
            client.send_text(text);
        }
    }
    println!("netplay_game: connecting to ws://{addr}");
    println!("  run `cargo run --example netplay_server` first, and a second client for company.");
    app.run();
}

// The engine reports trouble through `log`, which discards everything until a binary installs
// a logger. Every game installs the same one; the module explains what that buys and what it
// still does not cover in a browser. Gated to match the `main` below — this is the only game
// whose native entry point is itself gated, and an ungated `mod` here would be dead code on wasm.
#[cfg(not(target_arch = "wasm32"))]
#[path = "../shared/logging.rs"]
mod logging;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    logging::init();
    if std::env::var("NETPLAY_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }
    run();
}

/// The web build has no `main` of its own — `wasm-bindgen` calls [`run_netplay_game`].
#[cfg(target_arch = "wasm32")]
fn main() {}

// Only `web_check_netplay` uses this — gated so a native build does not compile a module it
// cannot reach.
#[cfg(target_arch = "wasm32")]
#[path = "../shared/web_check.rs"]
mod web_check;

/// Runs the client **and** the browser check, publishing its verdict to `document.title` for
/// `scripts/netplay_web_smoke.sh`.
///
/// # What this covers that nothing else does
///
/// This is the only automated check in the tree that runs the engine's **wasm WebSocket path**.
/// `src/network/wasm_impl.rs` is a completely separate implementation from the native
/// tungstenite client — different queueing, different overflow policy, different open semantics —
/// and since the 2026-08-19 deletion nothing has executed a line of it. The two native↔wasm
/// contracts recorded in `docs/MODULE_MAP.md` (a send before the socket opens, and the inbound
/// overflow policy) were both fixed in v0.150.2 *because* a browser smoke existed to catch them.
///
/// # Three things, in the order they can fail
///
/// 1. **`Connected` arrives** — the handshake really happened. A page that renders a perfectly good
///    empty world looks identical to one whose socket never opened.
/// 2. **A snapshot arrives and spawns entities** — the protocol round-trips through the browser's
///    socket, not just through a native one. `ClientMsg::Join` is sent *before* the socket opens,
///    which is ordinary code on both targets only because of the v0.150.2 fix; before it, that
///    packet vanished on the web alone.
/// 3. **The server's authority reaches the ship** — `have_authority` means a `Snap` carried a
///    position and ack, i.e. the full client→server→client loop closed in a browser.
///
/// ⚠️ **This does not assert that anything was drawn.** Reading pixels back out of a wgpu canvas
/// needs `preserveDrawingBuffer`, which changes how the surface is configured — so a check for it
/// would be measuring a different configuration than the one the game ships. Said plainly rather
/// than implied: the browser render path still has no pixel-level gate.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn web_check_netplay() {
    use web_check::{Step, WebCheck};

    let mut app = build_app();
    let addr = server_addr();
    app.world
        .insert_resource(NetworkClient::connect(&format!("ws://{addr}")));
    if let Some(client) = app.world.resource::<NetworkClient>() {
        if let Ok(text) = ron::to_string(&ClientMsg::Join) {
            client.send_text(text);
        }
    }

    // Generous: a CI runner has to start the native server, and the handshake plus one snapshot
    // interval is all this needs once it is up.
    const DEADLINE: f32 = 25.0;

    app.add_system(WebCheck::new("NETPLAY_CHECK", DEADLINE, move |world, _t| {
        // `NetplayClient` is a system, not a resource, so its state is not reachable from here.
        // Read the same evidence it reads instead — the event bus and the spawned entities — which
        // is stronger anyway: it asserts what actually arrived rather than what the client
        // remembers about it.
        let connected = world
            .resource::<Events<NetworkEvent>>()
            .map(|bus| {
                bus.read()
                    .iter()
                    .any(|e| matches!(e, NetworkEvent::Connected))
            })
            .unwrap_or(false);
        if connected {
            CONNECTED.with(|c| c.set(true));
        }
        let ever_connected = CONNECTED.with(|c| c.get());

        // Entities streamed in from a snapshot. The world starts with the background and the ship;
        // anything beyond that came over the socket.
        let sprites = world.query::<Sprite>().count();
        let streamed = sprites.saturating_sub(BASE_SPRITES.with(|b| b.get()));

        if ever_connected && streamed > 0 {
            return Step::pass(format!(
                "handshake completed and {streamed} entities streamed in over the browser socket"
            ));
        }
        Step::Waiting(format!(
            "connected {ever_connected}, {streamed} streamed entities (want connected plus at              least one)"
        ))
    }));

    // Count what the world holds before the socket delivers anything, so `streamed` means
    // "arrived over the network" rather than "exists".
    BASE_SPRITES.with(|b| b.set(app.world.query::<Sprite>().count()));
    app.run();
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Sticky: `Events` is drained every frame, so `Connected` is visible for exactly one frame and
    /// a probe that only looked at the current frame would miss it almost every time.
    static CONNECTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Sprites present before any snapshot — the background tiles plus the local ship.
    static BASE_SPRITES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Entry point for the wasm harness, called from `web/index.html`.
///
/// ⚠️ This must be `#[wasm_bindgen]`, not `#[no_mangle] pub extern "C"`. Both compile, and
/// `scripts/build_wasm_examples.sh` goes green either way — but only the former makes
/// `wasm-bindgen` emit a JS binding, so with the latter the page has nothing to import and the
/// game cannot start at all. Measured when this was wrong (it shipped that way in #494): the
/// generated `netplay_game.js` contained **zero** occurrences of this function's name.
///
/// That is the whole reason the `wasm-smokes` job exists. Compiling for wasm is not running on
/// wasm, and the build gate cannot tell the difference.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_netplay_game() {
    run();
}

// ── Acceptance test ─────────────────────────────────────────────────────────────────────────────
//
// `NETPLAY_SELFTEST=1 cargo run --example netplay_game`, and `scripts/selftests.sh`.
//
// Networking is the most screenshot-invisible subsystem in the tree, and — worse — its failures
// are *flattering*. A client that deletes a pickup the moment you touch it feels better than one
// that waits for the server: instant, no round trip. A client whose AOI eviction is dead shows a
// "streaming N" that climbs impressively. A client that never reconciles moves exactly as smoothly
// as one that does. Every one of those looks right in one window, in every frame, forever.
//
// So the checks below are built around a rule: **assert the thing the failure would make look
// better**, and assert both halves of it.
//
// Checks 1-5 need no server and run anywhere. Checks 6-7 spawn the real `netplay_server`.
//
// ⚠️ A missing server binary is a FAILURE here, not a skip. The deleted tree's networked selftests
// returned exit 0 when their server was absent, and those were the checks that covered the most —
// measured then: with the server hidden, the raw exit code was 0. `scripts/selftests.sh` also
// treats any non-audio SKIP as a failure, so this is belt and braces; the belt matters because
// running the binary directly bypasses the runner entirely.
//
// Exit codes: 0 pass · 1 a snapshot does not stream entities in · 2 removal-by-omission does not
// evict, or evicts what is still arriving · 3 prediction/reconciliation is not wired to the ship ·
// 4 the displayed position is not interpolated, or collision does not read it · 5 a claim deletes
// locally, or `Taken` does not · 6 two clients contesting one pickup do not agree with the server
// on what was taken · 7 the server does not tailor snapshots to the AOI, or shrinking it never
// drains · 8 the server binary is missing or unusable.

#[cfg(not(target_arch = "wasm32"))]
fn self_test() -> i32 {
    use engine::{InputAction, InputScript};
    use std::time::{Duration, Instant};

    /// Simulated frame time for the offline checks. Everything they exercise is driven by `dt`, so
    /// a fixed step reproduces real play exactly. The **server-backed** checks must not use it: the
    /// server ticks on a wall clock, so those are paced off `Instant`.
    const DT: f32 = 1.0 / 60.0;
    /// Frames between snapshots at the client's own snapshot rate.
    const SNAP_EVERY: u32 = 60 / SNAPSHOT_HZ;

    /// The game's world plus the buses `App` owns, minus the socket.
    ///
    /// This calls the game's own `build_world` and constructs the game's own systems, in the order
    /// `build_app` adds them — a harness that spawned its own ship, or ran the systems in a
    /// different order, would stop testing the game and start testing itself.
    fn harness() -> (App, NetworkSystem, NetplayClient) {
        let mut app = App::new();
        app.register_event::<NetworkEvent>();
        app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
        let ship = build_world(&mut app.world);
        (app, NetworkSystem::new(), NetplayClient::new(ship))
    }

    /// Hands the client a real protocol message through the real event bus — the same path a
    /// socket would deliver it on, encoded by the same serializer.
    fn feed(world: &mut World, msg: &ServerMsg) {
        let text = ron::to_string(msg).expect("encode ServerMsg");
        if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
            bus.send(NetworkEvent::TextMessage(text));
        }
    }

    /// One frame in the order `build_app` wires, then the end-of-frame flush `App` performs.
    fn tick(world: &mut World, net: &mut NetworkSystem, client: &mut NetplayClient, dt: f32) {
        net.run(world, dt);
        client.run(world, dt);
        if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
            bus.flush();
        }
    }

    fn at(kind: Kind, id: u32, pos: Vec2) -> EntityState {
        EntityState {
            id,
            kind,
            x: pos.x,
            y: pos.y,
        }
    }

    fn snap(tick: u32, ack: u32, ship: Vec2, entities: Vec<EntityState>) -> ServerMsg {
        ServerMsg::Snap {
            tick,
            ack,
            x: ship.x,
            y: ship.y,
            entities,
        }
    }

    // Somewhere well clear of the ship, so a pickup placed there is never claimed mid-check and a
    // hazard never registers a hit. Both would quietly delete what the check is counting.
    let far = |i: u32| start_pos() + Vec2::new(600.0 + i as f32 * 40.0, 0.0);

    // ── 1. A snapshot streams entities in, and Welcome establishes identity ────────────────────
    {
        let (mut app, mut net, mut client) = harness();
        feed(
            &mut app.world,
            &ServerMsg::Welcome {
                player_id: 7,
                world_w: WORLD_W,
                world_h: WORLD_H,
                pickups_total: PICKUP_COUNT,
            },
        );
        let want = vec![
            at(Kind::Pickup, 1, far(0)),
            at(Kind::Hazard, 2, far(1)),
            at(Kind::Player, 3, far(2)),
        ];
        feed(&mut app.world, &snap(1, 0, start_pos(), want.clone()));
        tick(&mut app.world, &mut net, &mut client, DT);

        if client.player_id != 7 || client.pickups_total != PICKUP_COUNT {
            eprintln!(
                "FAIL: Welcome did not establish identity — player_id {} (want 7), pickups_total \
                 {} (want {PICKUP_COUNT})",
                client.player_id, client.pickups_total
            );
            return 1;
        }

        // All three kinds, not just the first: `RemoteEntities` is keyed by `(Kind, id)` precisely
        // so pickup 1 and hazard 1 cannot collide, and a flat-id regression shows up here.
        let missing: Vec<NetId> = want
            .iter()
            .map(|e| e.net_id())
            .filter(|key| {
                client
                    .remotes
                    .get(key)
                    .map(|e| !app.world.is_alive(e))
                    .unwrap_or(true)
            })
            .collect();
        if !missing.is_empty() {
            eprintln!(
                "FAIL: a snapshot did not stream {} of 3 entities in — missing {missing:?}. The \
                 server sends the world only through snapshots; nothing else spawns anything.",
                missing.len()
            );
            return 1;
        }
        // And they are drawn where the snapshot put them.
        for e in &want {
            let key = e.net_id();
            let drawn = client.displayed.get(&key).copied().unwrap_or(Vec2::ZERO);
            if drawn.distance(Vec2::new(e.x, e.y)) > 1.0 {
                eprintln!(
                    "FAIL: {key:?} streamed in but is drawn at {drawn:?}, not the snapshot's \
                     {:?}",
                    Vec2::new(e.x, e.y)
                );
                return 1;
            }
        }
        println!("ok: a snapshot streams all three entity kinds in, at the positions it names");
    }

    // ── 2. Removal-by-omission — both halves ──────────────────────────────────────────────────
    //
    // The omitted entity must go, and the one still being mentioned must NEVER go — not even for
    // a single frame. Asserting only the first half passes on a client that evicts everything;
    // asserting only the second passes on a client that evicts nothing, which is the flattering
    // failure. So both are asserted, and the "stays" half is checked every frame rather than at
    // the end, because a momentary drop-and-respawn is invisible in a final count and is exactly
    // what a subtly wrong last-seen comparison produces.
    {
        let (mut app, mut net, mut client) = harness();
        let stays = at(Kind::Hazard, 10, far(0));
        let leaves = at(Kind::Hazard, 11, far(1));
        let stays_key = stays.net_id();
        let leaves_key = leaves.net_id();

        feed(
            &mut app.world,
            &snap(1, 0, start_pos(), vec![stays.clone(), leaves.clone()]),
        );
        tick(&mut app.world, &mut net, &mut client, DT);
        if !client.remotes.contains_key(&leaves_key) {
            eprintln!("FAIL: the entity that is about to leave never arrived");
            return 2;
        }

        // Keep mentioning `stays` only, for comfortably longer than the eviction timeout.
        let frames = (AOI_EVICT_SECS / DT as f64).ceil() as u32 + SNAP_EVERY * 2;
        let mut tick_no = 2;
        for f in 0..frames {
            if f % SNAP_EVERY == 0 {
                feed(
                    &mut app.world,
                    &snap(tick_no, 0, start_pos(), vec![stays.clone()]),
                );
                tick_no += 1;
            }
            tick(&mut app.world, &mut net, &mut client, DT);
            if !client.remotes.contains_key(&stays_key) {
                eprintln!(
                    "FAIL: the entity the server is still sending was evicted on frame {f} — the \
                     last-seen comparison is dropping entities that never left the AOI, which \
                     reads in play as objects flickering out of existence."
                );
                return 2;
            }
        }

        if client.remotes.contains_key(&leaves_key) {
            eprintln!(
                "FAIL: an entity unmentioned for {AOI_EVICT_SECS:.3} s was not evicted. The \
                 server never says an entity left — it stops including it — so this timeout is \
                 the ONLY thing that removes anything. With it dead the world only ever grows, \
                 and the HUD's 'streaming N' climbs impressively while nothing is near you."
            );
            return 2;
        }

        // The three parallel maps must agree. `RemoteEntities` owns the entity, `buffers` owns the
        // interpolation history, `last_seen` owns eviction, and they are kept in step by hand —
        // so a leak here is silent and unbounded, which is the worst shape a leak can have.
        let leaked: Vec<&str> = [
            ("buffers", client.buffers.contains_key(&leaves_key)),
            ("last_seen", client.last_seen.contains_key(&leaves_key)),
            ("displayed", client.displayed.contains_key(&leaves_key)),
            ("newest", client.newest.contains_key(&leaves_key)),
        ]
        .iter()
        .filter(|(_, leaked)| *leaked)
        .map(|(name, _)| *name)
        .collect();
        if !leaked.is_empty() {
            eprintln!(
                "FAIL: eviction despawned the entity but left it in {leaked:?}. Those maps are \
                 parallel to RemoteEntities and nothing keeps them in step but `forget`."
            );
            return 2;
        }
        println!(
            "ok: removal-by-omission evicts what stopped arriving (and never what is still \
             arriving), from all four maps"
        );
    }

    // ── 3. Prediction and reconciliation, wired to the actual ship ────────────────────────────
    //
    // `Prediction` is unit-tested above in isolation, and those tests would all still pass with
    // the client never calling `reconcile` — which is the sabotage that matters, because the game
    // still predicts, the ship still moves, it still *feels* right, and the server just quietly
    // disagrees about where you are. With one window there is nothing to compare against.
    //
    // So this check drives held input through the real `ENGINE_INPUT` path (`InputState` has no
    // public press setter, which is a feature: it forces the check through `read_move` rather
    // than a faked direction) and then hands the client a snapshot that disagrees ON PURPOSE.
    {
        let (mut app, mut net, mut client) = harness();
        let mut script = InputScript::new([(0, InputAction::KeyDown(KeyCode::KeyD))]);

        // Fly right for a while. Each INPUT_DT of held input is one predicted step.
        let frames = 30;
        for _ in 0..frames {
            script.apply(&mut app.world);
            tick(&mut app.world, &mut net, &mut client, DT);
        }

        let predicted = client.prediction.pos;
        if predicted.x <= start_pos().x + 1.0 {
            eprintln!(
                "FAIL: {frames} frames of held input moved the ship {:.2} px — prediction is not \
                 reaching the ship at all.",
                predicted.x - start_pos().x
            );
            return 3;
        }
        // The ship's Transform is what the renderer draws; predicting into a field nobody reads
        // would pass a weaker check.
        let drawn = app
            .world
            .get::<Transform>(client.ship)
            .map(|t| t.position)
            .unwrap_or(Vec2::ZERO);
        if drawn.distance(predicted) > 0.01 {
            eprintln!(
                "FAIL: the ship is predicted at {predicted:?} but drawn at {drawn:?} — the \
                 prediction never reaches the Transform."
            );
            return 3;
        }
        let pending_before = client.prediction.pending_len();
        if pending_before == 0 {
            eprintln!("FAIL: no inputs are pending — nothing was ever sent to be acked");
            return 3;
        }

        // Now the server disagrees: it has applied only the FIRST input, and it puts the ship
        // somewhere the client could not have predicted. A client that ignores the correction
        // stays where it was; one that snaps without replaying lands exactly on the server
        // position; only one that replays the unacked tail lands past it, by exactly those inputs.
        let server_pos = Vec2::new(start_pos().x - 200.0, start_pos().y + 120.0);
        feed(&mut app.world, &snap(2, 1, server_pos, vec![]));
        script.apply(&mut app.world);
        tick(&mut app.world, &mut net, &mut client, DT);

        let after = client.prediction.pos;
        if after.distance(predicted) < 1.0 {
            eprintln!(
                "FAIL: the ship is still at {after:?} after the server said {server_pos:?} — the \
                 correction was ignored. Prediction without reconciliation feels perfect in one \
                 window and disagrees with the server forever."
            );
            return 3;
        }
        if after.distance(server_pos) < 1.0 {
            eprintln!(
                "FAIL: the ship snapped exactly to the server's {server_pos:?} without replaying \
                 the {pending_before} unacked inputs — that is the rubber-band: every snapshot \
                 yanks you back by however many inputs are in flight."
            );
            return 3;
        }
        // Replaying the unacked tail from the server position must reproduce the ship exactly.
        let mut want = (server_pos.x, server_pos.y);
        for _ in 0..client.prediction.pending_len() {
            want = step_position(want.0, want.1, 1.0, 0.0);
        }
        if (after.x - want.0).abs() > 0.5 || (after.y - want.1).abs() > 0.5 {
            eprintln!(
                "FAIL: reconciled to {after:?}, but replaying the {} unacked inputs from the \
                 server's position gives ({:.2}, {:.2}).",
                client.prediction.pending_len(),
                want.0,
                want.1
            );
            return 3;
        }
        println!(
            "ok: input is predicted onto the ship, and a server correction replays the {} unacked \
             inputs instead of snapping ({:.0} px from where it was, {:.0} px past the server's \
             own position)",
            client.prediction.pending_len(),
            after.distance(predicted),
            after.distance(server_pos)
        );
    }

    // ── 4. The displayed position is interpolated, and collision reads it ─────────────────────
    //
    // Two halves, and the second is the one that earns the check. Interpolating for the renderer
    // while colliding against the newest snapshot means you are hit by a hazard that is visibly
    // somewhere else — it reads as bad hitboxes, and no frame looks wrong.
    //
    // ⚠️ A warm-up is part of the property: until the buffer holds more than INTERP_DELAY of
    // history there is nothing to interpolate *between*, so drawn == newest and any assertion
    // about the gap is vacuous. The loop below fills the buffer before measuring.
    {
        let (mut app, mut net, mut client) = harness();
        // Deliberately faster than any real hazard (which tops out at HAZARD_MAX_SPEED). The
        // WIRING under test — which map `pump_hazards` reads — is speed-independent; only its
        // *observability* is not. The two positions must separate by more than the collision
        // reach or "stand on the newest snapshot, expect no hit" cannot discriminate at all, and
        // the second half below would pass whichever map collision read.
        //
        // Measured, not assumed: at 300 px/s the gap is 17.5 px against a 30 px reach, so the
        // first draft of this check reported a hit in BOTH positions and could not tell them
        // apart. That is also the honest limit of the property in real play — at
        // HAZARD_MAX_SPEED the same bug is a ~9 px error, a wrong hitbox rather than an obviously
        // wrong one.
        const SPEED: f32 = 900.0; // px/s
        let origin = far(0);
        let key = (Kind::Hazard, 20);

        // Feed a hazard travelling right at a constant speed, one snapshot every SNAP_EVERY
        // frames, for long enough that the buffer is full of real history.
        let total_frames = SNAP_EVERY * 8;
        let mut tick_no = 1;
        for f in 0..total_frames {
            if f % SNAP_EVERY == 0 {
                let t = f as f32 * DT;
                let pos = origin + Vec2::new(SPEED * t, 0.0);
                feed(
                    &mut app.world,
                    &snap(tick_no, 0, start_pos(), vec![at(Kind::Hazard, 20, pos)]),
                );
                tick_no += 1;
            }
            tick(&mut app.world, &mut net, &mut client, DT);
        }

        let drawn = match client.displayed.get(&key) {
            Some(p) => *p,
            None => {
                eprintln!("FAIL: the hazard is not being displayed at all");
                return 4;
            }
        };
        let newest = client.newest.get(&key).copied().unwrap_or(Vec2::ZERO);
        let lag = newest.x - drawn.x;

        // Where the sender actually was, INTERP_DELAY ago. Comparing against the newest snapshot
        // instead is wrong and was the deleted `salvage_run`'s first-draft bug: that sample is
        // already up to one snapshot interval old, so the expected lag came out 37.5 px against a
        // measured 17.5 and only passed because the tolerance swallowed the difference.
        let rt = total_frames as f64 * DT as f64 - INTERP_DELAY;
        let want_x = origin.x + SPEED * (rt - DT as f64) as f32;
        let slack = SPEED * DT * 2.0; // where the snapshot cadence happens to land
        if (drawn.x - want_x).abs() > slack {
            eprintln!(
                "FAIL: the hazard is drawn at x={:.1}, but {INTERP_DELAY:.3} s ago the sender was \
                 at x={want_x:.1} (+/- {slack:.1}). The newest snapshot says x={:.1}; a client \
                 that snaps to it rather than interpolating draws exactly that.",
                drawn.x, newest.x
            );
            return 4;
        }
        if lag <= 0.0 {
            eprintln!(
                "FAIL: the drawn position is not behind the newest snapshot (lag {lag:.1} px) — \
                 nothing is being interpolated."
            );
            return 4;
        }
        // The gap must actually MATTER. If it is smaller than the hazard, colliding against the
        // wrong one of the two would be unobservable and the second half below would be vacuous.
        let reach = PLAYER_RADIUS + HAZARD_RADIUS;
        if lag <= reach {
            eprintln!(
                "FAIL: the interpolation lag is only {lag:.1} px against a {reach:.0} px collision \
                 reach — the ship would touch the hazard at BOTH positions, so the second half \
                 below cannot tell which map collision reads and would pass either way. Raise \
                 SPEED in this check, or lower SNAPSHOT_HZ / raise INTERP_DELAY in the protocol."
            );
            return 4;
        }

        // Second half: park the ship on the DRAWN position — a hit must register. Park it on the
        // NEWEST — it must not. Driving `pump_hazards` directly is deliberate: it is the collision
        // pass, and the ship position it reads is the prediction, which nothing else here moves.
        client.prediction.pos = drawn;
        client.touching.clear();
        let hits_before = client.hits;
        client.pump_hazards();
        if client.hits == hits_before {
            eprintln!(
                "FAIL: the ship is sitting exactly on the hazard's drawn position ({drawn:?}) and \
                 no hit registered — collision is not reading what is on screen."
            );
            return 4;
        }

        client.prediction.pos = newest;
        client.touching.clear();
        let hits_before = client.hits;
        client.pump_hazards();
        if client.hits != hits_before {
            eprintln!(
                "FAIL: the ship was hit while sitting on the hazard's NEWEST snapshot position \
                 ({newest:?}), {lag:.1} px from where the hazard is drawn ({drawn:?}). That is \
                 the bug that reads as bad hitboxes: you are killed by something visibly \
                 elsewhere, and every frame looks correct."
            );
            return 4;
        }
        println!(
            "ok: the hazard is drawn where the sender was {INTERP_DELAY:.3} s ago, {lag:.1} px \
             behind the newest snapshot, and collision fires on the drawn position and not the \
             newest one"
        );
    }

    // ── 5. A claim is a request; only the server removes a pickup ─────────────────────────────
    //
    // THE flattering failure. A client that deletes the pickup on touch is instant, needs no round
    // trip, and looks better in single player. It is wrong the moment two pilots reach for the
    // same salvage — and with one window there is nothing to reveal it.
    {
        let (mut app, mut net, mut client) = harness();
        feed(
            &mut app.world,
            &ServerMsg::Welcome {
                player_id: 4,
                world_w: WORLD_W,
                world_h: WORLD_H,
                pickups_total: PICKUP_COUNT,
            },
        );
        // A pickup exactly under the ship, so the claim path is guaranteed to fire.
        let key = (Kind::Pickup, 30);
        feed(
            &mut app.world,
            &snap(1, 0, start_pos(), vec![at(Kind::Pickup, 30, start_pos())]),
        );
        tick(&mut app.world, &mut net, &mut client, DT);
        tick(&mut app.world, &mut net, &mut client, DT);

        if !client.claim_sent.contains(&30) {
            eprintln!(
                "FAIL: the ship is on top of pickup 30 and never claimed it — the claim path is \
                 not reaching the server."
            );
            return 5;
        }
        // ⚠️ `NetworkClient` has no readable outbox, so "a claim was sent" is unobservable
        // offline. `claim_sent` is the closest honest observable, and the half below is what
        // makes it meaningful: the pickup must still be standing.
        if !client.remotes.contains_key(&key) {
            eprintln!(
                "FAIL: touching pickup 30 deleted it locally. A claim is a REQUEST — the server \
                 decides. Deleting on touch is faster, feels better, and gives two players the \
                 same pickup; their scoreboards then part company and neither is wrong locally."
            );
            return 5;
        }
        if client.score != 0 {
            eprintln!(
                "FAIL: the score moved to {} on a claim the server has not granted.",
                client.score
            );
            return 5;
        }

        // Now the server grants it — to somebody else. The pickup goes; our score does not move.
        feed(
            &mut app.world,
            &ServerMsg::Taken {
                id: 30,
                by: 99,
                score: 1,
            },
        );
        tick(&mut app.world, &mut net, &mut client, DT);
        if client.remotes.contains_key(&key) {
            eprintln!(
                "FAIL: the server said pickup 30 was taken and it is still here — `Taken` is the \
                 only thing that removes a pickup, so nothing else ever will."
            );
            return 5;
        }
        if client.score != 0 {
            eprintln!(
                "FAIL: another pilot took pickup 30 and our score became {} — the `by` field is \
                 being ignored.",
                client.score
            );
            return 5;
        }

        // And one granted to us does move the score, to the value the SERVER named — not a local
        // increment, which would drift the moment a Taken message is missed.
        let key2 = (Kind::Pickup, 31);
        feed(
            &mut app.world,
            &snap(2, 0, start_pos(), vec![at(Kind::Pickup, 31, start_pos())]),
        );
        tick(&mut app.world, &mut net, &mut client, DT);
        feed(
            &mut app.world,
            &ServerMsg::Taken {
                id: 31,
                by: 4,
                score: 6,
            },
        );
        tick(&mut app.world, &mut net, &mut client, DT);
        if client.remotes.contains_key(&key2) || client.score != 6 {
            eprintln!(
                "FAIL: our own grant left the pickup {} and the score at {} (want gone, 6 — the \
                 server's number, not a local +1).",
                if client.remotes.contains_key(&key2) {
                    "standing"
                } else {
                    "removed"
                },
                client.score
            );
            return 5;
        }
        println!(
            "ok: a claim leaves the pickup standing and the score still, and only the server's \
             `Taken` removes it — crediting exactly whom the server named, at the score it named"
        );
    }

    // ── 6-7. Against the real server ──────────────────────────────────────────────────────────
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let server_bin =
        exe_dir.map(|d| d.join(format!("netplay_server{}", std::env::consts::EXE_SUFFIX)));
    let Some(server_bin) = server_bin.filter(|p| p.exists()) else {
        // ⚠️ NOT a skip. See the note at the top of this test.
        eprintln!(
            "FAIL: netplay_server has not been built, so the two checks that cover the most — a \
             contested claim and live AOI streaming — cannot run. Build it with `cargo build \
             --example netplay_server`; `scripts/selftests.sh` does this for you, deriving the \
             list from Cargo.toml's `[[example]]` blocks. This is a FAILURE rather than a skip \
             because the deleted tree made it a skip and measured the result: with the server \
             hidden, the raw exit code was 0."
        );
        return 8;
    };

    // A port of our own. Binding :0 asks the OS for a free one; the listener is dropped
    // immediately so the child can take it. That is a race in principle and the right trade in
    // practice — the alternative is the hardcoded 9006, which collides with a server the user is
    // already running, or with a parallel CI job.
    let addr = match std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()) {
        Ok(a) => a.to_string(),
        Err(e) => {
            eprintln!("FAIL: could not reserve a local port for the server ({e})");
            return 8;
        }
    };

    // ⚠️ Keep the child's stderr. The port above is reserved by binding `:0` and dropping the
    // listener, so the child re-binds a port this process no longer holds — and `server.rs` meets
    // a lost race with `.expect("bind failed")`. With stderr discarded that panic went nowhere,
    // and the only surviving evidence was the 10 s timeout below, which reports a server that
    // "never bound" without saying it died trying. Two different failures, one message.
    //
    // A FILE, not a pipe. An unread pipe fills and blocks the writer, and the server logs every
    // accept error and failed WS handshake — so a pipe would trade this diagnosis for a deadlock
    // in the checks that follow. A file has no such ceiling and needs no reader thread.
    let err_path =
        std::env::temp_dir().join(format!("netplay_selftest_{}.err", std::process::id()));
    let err_file = match std::fs::File::create(&err_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "FAIL: could not create {} for the server's stderr: {e}",
                err_path.display()
            );
            return 8;
        }
    };
    let mut child = match std::process::Command::new(&server_bin)
        .env("NETPLAY_ADDR", &addr)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::from(err_file))
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FAIL: could not spawn {}: {e}", server_bin.display());
            std::fs::remove_file(&err_path).ok();
            return 8;
        }
    };

    // Kills the child and returns `code`, printing `msg` as the failure reason.
    fn finish(child: &mut std::process::Child, code: i32, msg: Option<String>) -> i32 {
        child.kill().ok();
        child.wait().ok();
        if let Some(msg) = msg {
            eprintln!("FAIL: {msg}");
        }
        code
    }

    // Whatever the server wrote to stderr, trimmed. Empty when it said nothing.
    let server_said = |path: &std::path::Path| -> String {
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .trim()
            .to_string()
    };

    // Wait for the child to bind before connecting. `NetworkClient::connect` dials once and does
    // NOT retry, so connecting first and hoping is not a slower path — it is a guaranteed failure.
    let bind_deadline = Instant::now() + Duration::from_secs(10);
    let mut bound = false;
    let mut died = None;
    while Instant::now() < bind_deadline {
        if std::net::TcpStream::connect(&addr).is_ok() {
            bound = true;
            break;
        }
        // ⚠️ A dead child will never bind, so waiting out the remaining deadline for it buys
        // nothing and costs the diagnosis: 10 s of silence reads as a slow machine, which is the
        // one thing it is not. This is the branch that separates "the port was taken" from "the
        // server is wedged", and they have opposite fixes.
        if let Ok(Some(status)) = child.try_wait() {
            died = Some(status);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if let Some(status) = died {
        let said = server_said(&err_path);
        std::fs::remove_file(&err_path).ok();
        eprintln!(
            "FAIL: netplay_server exited before it bound {addr} ({status}) — {}. The port is \
             reserved by binding :0 and dropping the listener, so anything that claimed it in \
             that gap leaves the server nothing to bind.",
            if said.is_empty() {
                "it wrote nothing to stderr".to_string()
            } else {
                format!("it said: {said}")
            },
        );
        return 8;
    }
    if !bound {
        let said = server_said(&err_path);
        std::fs::remove_file(&err_path).ok();
        return finish(
            &mut child,
            8,
            Some(format!(
                "netplay_server never bound {addr} within 10 s, and it is still running — a slow \
                 or wedged server rather than a failed bind.{}",
                if said.is_empty() {
                    String::new()
                } else {
                    format!(" It said: {said}")
                },
            )),
        );
    }
    // Bound, so the diagnostic window is over and every failure past here has its own message.
    // On Unix the child keeps writing into the unlinked inode, which costs nothing and is
    // reclaimed when it exits — this is what keeps a 60-run soak from leaving 60 files behind.
    std::fs::remove_file(&err_path).ok();

    /// A live client: the game's own world and systems, plus a real socket.
    struct Live {
        app: App,
        net: NetworkSystem,
        client: NetplayClient,
    }

    let connect = |addr: &str| -> Live {
        let (mut app, net, client) = harness();
        app.world
            .insert_resource(NetworkClient::connect(&format!("ws://{addr}")));
        if let Some(c) = app.world.resource::<NetworkClient>() {
            if let Ok(text) = ron::to_string(&ClientMsg::Join) {
                c.send_text(text);
            }
        }
        Live { app, net, client }
    };

    /// Runs real system chains for `secs` of **wall clock**, pacing off `Instant`.
    ///
    /// The server steps in real time, so an accumulator clock would drift against it — the trap
    /// `docs/VERIFICATION.md` records from `beat_crawler`, where `t += 1.0/60.0` made a correct
    /// detector look 40% over-firing.
    fn run_live(lives: &mut [&mut Live], secs: f64) {
        let start = Instant::now();
        let mut last = start;
        while start.elapsed().as_secs_f64() < secs {
            let now = Instant::now();
            let dt = (now - last).as_secs_f32();
            last = now;
            for live in lives.iter_mut() {
                live.net.run(&mut live.app.world, dt);
                live.client.run(&mut live.app.world, dt);
                if let Some(bus) = live.app.world.resource_mut::<Events<NetworkEvent>>() {
                    bus.flush();
                }
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    }

    // ── 6. Two clients, one pickup ────────────────────────────────────────────────────────────
    //
    // A contested pickup has no meaning with one player. Both pilots spawn on the world centre
    // (the server spawns everyone there, deliberately, so this is not a coin flip), fly at the
    // same salvage, and the assertion is the invariant rather than a score:
    //
    //     points awarded across both pilots == pickups the server actually removed
    //
    // ⚠️ Counting score *deltas* would be corrupted by any pickup that happens to be nearby;
    // the invariant holds however many get taken. That is what caught the sabotage in the deleted
    // `coin_race`: dropping the server's first-claim-wins guard left both boards *agreeing* on
    // 1-1, and only 2 points against 1 pickup revealed it.
    //
    // ⚠️ The ships genuinely FLY. The first draft of this check just assigned `prediction.pos`
    // and the server granted nothing at all — correctly, because a claim is validated against the
    // server's own authoritative position, which only client *inputs* move. The check caught its
    // own harness cheating, which is the shape every one of these is supposed to have.
    {
        // Steers every client toward `target` through the game's real input path, until all are
        // within `stop_dist` or `secs` runs out. Eight-direction, like a player: `read_move` reads
        // keys, so nothing here can express a direction the game itself cannot.
        /// What a flight cost. The check reports these rather than only whether it arrived,
        /// because the two margins that decide check 6 are runtime quantities and both flake
        /// fixes turned on them — see `APPROACH`.
        struct Flight {
            arrived: bool,
            /// Wall clock used, against the `secs` budget.
            elapsed: f64,
            /// The LONGEST single iteration. This is the client-side margin's whole story: the
            /// stop test runs BEFORE the frame moves the ship, so one iteration of travel is the
            /// overshoot, and `APPROACH - CLAIM_RADIUS` is all the room there is to absorb it.
            worst_iter: f64,
        }

        fn fly_to(lives: &mut [&mut Live], target: Vec2, stop_dist: f32, secs: f64) -> Flight {
            const DEAD: f32 = 6.0;
            let start = Instant::now();
            let mut last = start;
            let mut worst_iter = 0.0f64;
            while start.elapsed().as_secs_f64() < secs {
                let mut all_arrived = true;
                for live in lives.iter_mut() {
                    let d = target - live.client.prediction.pos;
                    let arrived = d.length() <= stop_dist;
                    all_arrived &= arrived;
                    let mut events = vec![
                        (0, InputAction::KeyUp(KeyCode::KeyA)),
                        (0, InputAction::KeyUp(KeyCode::KeyD)),
                        (0, InputAction::KeyUp(KeyCode::KeyW)),
                        (0, InputAction::KeyUp(KeyCode::KeyS)),
                    ];
                    if !arrived {
                        if d.x > DEAD {
                            events.push((0, InputAction::KeyDown(KeyCode::KeyD)));
                        } else if d.x < -DEAD {
                            events.push((0, InputAction::KeyDown(KeyCode::KeyA)));
                        }
                        if d.y > DEAD {
                            events.push((0, InputAction::KeyDown(KeyCode::KeyS)));
                        } else if d.y < -DEAD {
                            events.push((0, InputAction::KeyDown(KeyCode::KeyW)));
                        }
                    }
                    InputScript::new(events).apply(&mut live.app.world);
                }
                if all_arrived {
                    return Flight {
                        arrived: true,
                        elapsed: start.elapsed().as_secs_f64(),
                        worst_iter,
                    };
                }
                let now = Instant::now();
                let dt = (now - last).as_secs_f32();
                last = now;
                worst_iter = worst_iter.max(dt as f64);
                for live in lives.iter_mut() {
                    live.net.run(&mut live.app.world, dt);
                    live.client.run(&mut live.app.world, dt);
                    if let Some(bus) = live.app.world.resource_mut::<Events<NetworkEvent>>() {
                        bus.flush();
                    }
                }
                std::thread::sleep(Duration::from_millis(16));
            }
            Flight {
                arrived: false,
                elapsed: start.elapsed().as_secs_f64(),
                worst_iter,
            }
        }

        let mut a = connect(&addr);
        let mut b = connect(&addr);

        // Wait for both to receive a first snapshot. How long that takes is the cheapest health
        // signal this check has for the machine it is running on, so it is kept and reported.
        let authority_start = Instant::now();
        let deadline = authority_start + Duration::from_secs(10);
        while Instant::now() < deadline && (!a.client.have_authority || !b.client.have_authority) {
            run_live(&mut [&mut a, &mut b], 0.25);
        }
        let authority_wait = authority_start.elapsed().as_secs_f64();
        if !a.client.have_authority || !b.client.have_authority {
            return finish(
                &mut child,
                6,
                Some(format!(
                    "the two clients never both received a snapshot from {addr} in 10 s (a: {}, \
                     b: {})",
                    a.client.status, b.client.status
                )),
            );
        }

        // Contest the pickups nearest the spawn, so the flight is short and both pilots are
        // guaranteed to have them streaming.
        const ROUNDS: usize = 2;
        // This distance is squeezed from BOTH sides, and picking it by looking at one side is how
        // v0.155.1 traded one flake for the shape of another.
        //
        // *Below* it is `CLAIM_RADIUS` (24 px): dip inside that during the flight and
        // `Client::run`'s own `pump_claims` fires, which both claims early AND latches
        // `claim_sent`, so the staged push below becomes a no-op for that pilot. ⚠️ The stop test
        // runs BEFORE the frame moves the ship, so the margin has to cover a whole frame of
        // travel — and `read_move` does not normalise, so a diagonal closes at
        // `PLAYER_SPEED * sqrt(2)` ≈ 452 px/s. At APPROACH = 40 that is one 35 ms iteration, on
        // runners that sleep 16 ms per iteration and already lost a race to an 83 ms snapshot gap.
        //
        // *Above* it is the server's reach, `CLAIM_RADIUS + CLAIM_SLACK` = 120 px, measured from
        // the server's own copy of the ship — which lags the client's prediction by the input
        // round trip. Measured at APPROACH = 70 mid-flight: the server saw **76.5** and **96.7**
        // px, i.e. 23 px of margin on a legitimate claim.
        //
        // 70 with `SETTLE` below beats 40 on both counts. Stopping the ships and letting a few
        // snapshots land removes the lag term instead of paying for it out of the lower margin:
        // the server converges on ~70 px (50 px of reach to spare, against 23) while the client
        // keeps the full 46 px of travel room (~102 ms, against ~35).
        const APPROACH: f32 = 70.0;
        // Long enough for the server's copy to catch up with a stopped ship: SNAPSHOT_HZ is 12, so
        // this is ~4 snapshots. It is not a guess about scheduling — the ships are not moving, so
        // the only thing being waited on is the round trip.
        const SETTLE: f64 = 0.35;

        // ── The margins, kept rather than remembered ──────────────────────────────────────────
        //
        // v0.155.1 and v0.155.2 both turned on numbers nobody can see from the outside: the
        // server's copy sitting 76.5 px into a 120 px reach, and one 35 ms iteration against
        // 46 px of travel room. Each was measured by hand, once, mid-investigation — and then the
        // only place they survived was prose, which CLAUDE.md is explicit about: a number pinned
        // in prose goes stale, and a margin eroding on a slower runner is a red that arrives with
        // no warning at all. The worst of each across every round rides on the `ok:` line, so the
        // check reports how close it came instead of only that it got there.
        let mut worst_gap = 0.0f32; // server-side: its copy of the ship, against its reach
        let mut worst_iter = 0.0f64; // client-side: one frame of travel, against the room for it
        let mut worst_flight = 0.0f64; // against `fly_to`'s own 12 s budget
        let mut contested = 0usize;

        for _ in 0..ROUNDS {
            let here = a.client.prediction.pos;
            let target = a
                .client
                .displayed
                .iter()
                .filter(|((kind, _), _)| *kind == Kind::Pickup)
                .filter(|((_, id), _)| !a.client.taken_seen.contains(id))
                .min_by(|(_, p), (_, q)| {
                    here.distance(**p)
                        .partial_cmp(&here.distance(**q))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|((_, id), pos)| (*id, *pos));
            let Some((target_id, target)) = target else {
                break;
            };

            let flight = fly_to(&mut [&mut a, &mut b], target, APPROACH, 12.0);
            worst_flight = worst_flight.max(flight.elapsed);
            worst_iter = worst_iter.max(flight.worst_iter);
            if !flight.arrived {
                return finish(
                    &mut child,
                    6,
                    Some(format!(
                        "the pilots did not reach {target:?} within 12 s (a is {:.0} px away, b \
                         is {:.0} px; slowest iteration {:.0} ms). They fly at {PLAYER_SPEED} \
                         px/s through the real input path, so this is a movement or \
                         reconciliation failure, not a claim one.",
                        a.client.prediction.pos.distance(target),
                        b.client.prediction.pos.distance(target),
                        flight.worst_iter * 1000.0,
                    )),
                );
            }

            // Let the ships stand still until the server's copy catches up with them. Nothing is
            // moving, so this waits out the input round trip and nothing else — and it is what
            // buys the claim below its margin against the server's 120 px reach without spending
            // the client's margin against `CLAIM_RADIUS`. See `APPROACH`.
            run_live(&mut [&mut a, &mut b], SETTLE);

            // The margin `SETTLE` exists to buy, read at the only instant it decides anything:
            // `try_claim` measures from the server's copy of the ship, so this is the distance
            // the server is about to compare against its own reach. Worst of the two pilots.
            worst_gap = worst_gap
                .max(a.client.server_pos.distance(target))
                .max(b.client.server_pos.distance(target));

            // Both pilots are now within APPROACH px of the salvage — inside the server's
            // CLAIM_SLACK, so a claim from here is one the server should honour. Push both onto it
            // on the SAME frame; this is the contested moment, and with one pilot it cannot exist.
            a.client.prediction.pos = target;
            b.client.prediction.pos = target;

            // ⚠️ **Claim here, not on the next frame.** This line is the whole reason check 6 was
            // flaky (~15% of runs, 2026-08-24 → diagnosed 2026-08-25).
            //
            // The position above is written directly, so it is not backed by any input the server
            // will ever ack — and `run_live`'s frame drains the network FIRST, where `reconcile`
            // overwrites `prediction.pos` wholesale with "authoritative position + replayed
            // pending inputs". A snapshot arrives every `1/SNAPSHOT_HZ` (~83 ms) against a ~16 ms
            // frame, so roughly one teleport in five was erased before `pump_claims` ran three
            // steps later in the same frame. The pickup was then ~76 px away — far outside
            // `CLAIM_RADIUS` — so no claim was ever *sent*, and the failure read as "the server
            // granted none", which points at the server's distance validation and is the wrong
            // half of the system entirely.
            //
            // Verified by forcing the race: assigning `prediction.pos = server_pos` right here
            // fails 3 runs out of 3 with exactly the observed signature (exit 6, zero claims).
            //
            // Pumping claims explicitly makes the contested moment what the comment above already
            // claims it is — the same instant for both pilots, with no frame boundary in between.
            // It is the same method the frame calls three steps after reconciliation; the only
            // thing being chosen here is *when*.
            // ⚠️ **Assert the contest happened, rather than assuming the staging worked** — and
            // assert it on BOTH sides of the pump, because one side alone cannot see it.
            //
            // `pump_claims` skips any pickup already in `claim_sent`, and only a `Taken` clears
            // that set. So a single premature claim during the flight — a dip inside
            // `CLAIM_RADIUS` when a slow frame overshoots the stop distance — latches that pilot
            // out of this contest for good. One claimant is not a contest: the server's
            // first-claim-wins guard is never asked anything, and the invariant below then holds
            // for the wrong reason.
            //
            // ⚠️ **Checking only AFTER the pump does not catch it**, and the first draft of this
            // guard did exactly that and was proven useless by its own sabotage: a latched-out
            // pilot has the id in `claim_sent` too — that is what "latched" means — so the
            // post-condition is satisfied by the very failure it was written to detect. The
            // PRE-condition is the half that carries the information.
            let early = (
                a.client.claim_sent.contains(&target_id),
                b.client.claim_sent.contains(&target_id),
            );
            if early.0 || early.1 {
                return finish(
                    &mut child,
                    6,
                    Some(format!(
                        "pickup {target_id} was already claimed during the approach flight (a: {}, b: {}), so the staged moment cannot be contested — `pump_claims` skips what is already in `claim_sent`. The flight stops {APPROACH} px out, clear of the {CLAIM_RADIUS} px claim radius; a frame long enough to close that gap in one step lands here.",
                        early.0, early.1,
                    )),
                );
            }

            a.client.pump_claims(&mut a.app.world);
            b.client.pump_claims(&mut b.app.world);

            if !a.client.claim_sent.contains(&target_id)
                || !b.client.claim_sent.contains(&target_id)
            {
                return finish(
                    &mut child,
                    6,
                    Some(format!(
                        "the staged push did not make both pilots claim pickup {target_id} (a: {}, b: {}). The pre-check above passed, so neither had claimed it already and `pump_claims` measured a distance it should not have — it reads `prediction.pos`, which was just written onto the pickup.",
                        a.client.claim_sent.contains(&target_id),
                        b.client.claim_sent.contains(&target_id),
                    )),
                );
            }

            run_live(&mut [&mut a, &mut b], 1.0);
            contested += 1;
        }

        if contested == 0 {
            return finish(
                &mut child,
                6,
                Some("no pickups were streaming to the first pilot to contest".into()),
            );
        }
        run_live(&mut [&mut a, &mut b], 0.8);

        // `Taken` is broadcast to everyone, so each client saw every removal. Assert they AGREE
        // before counting: two clients disagreeing about what the world lost is its own bug, and
        // taking a union first would hide it.
        if a.client.taken_seen != b.client.taken_seen {
            return finish(
                &mut child,
                6,
                Some(format!(
                    "the two pilots disagree about what was taken: a saw {:?}, b saw {:?}. \
                     `Taken` is broadcast to every client precisely so they cannot.",
                    a.client.taken_seen, b.client.taken_seen
                )),
            );
        }
        let taken = a.client.taken_seen.clone();
        let points = a.client.score + b.client.score;
        if taken.is_empty() {
            // ⚠️ Separate "no claim was sent" from "the claim was refused" before blaming either.
            // These are opposite halves of the system and the message used to offer both, which
            // sent the 2026-08-25 investigation at the server's distance validation when the
            // client had never opened its mouth. `claim_sent` is the client's own record.
            let sent = a.client.claim_sent.len() + b.client.claim_sent.len();
            let detail = if sent == 0 {
                "neither pilot SENT a claim — this is a client-side problem. The predicted \
                 position is what `pump_claims` measures from, and something reset it between \
                 the push and the pump (`reconcile` overwrites it wholesale on every snapshot)."
                    .to_string()
            } else {
                format!(
                    "{sent} claim(s) were sent and none was granted — this is the server's \
                     `try_claim`, which measures from ITS copy of the ship, not the predicted \
                     one. Reach is {:.0} px and its copy was {worst_gap:.1} px out at the worst \
                     staged moment, so {}.",
                    CLAIM_RADIUS + CLAIM_SLACK,
                    if worst_gap > CLAIM_RADIUS + CLAIM_SLACK {
                        "the claim was legitimately out of range — `SETTLE` did not buy enough \
                         for this machine"
                    } else {
                        "distance is NOT the reason and the refusal is in the guard itself"
                    },
                )
            };
            return finish(
                &mut child,
                6,
                Some(format!(
                    "two pilots sat on {contested} pickup(s) together and the server granted \
                     none — scores {} / {}. {detail}",
                    a.client.score, b.client.score
                )),
            );
        }
        if points as usize != taken.len() {
            return finish(
                &mut child,
                6,
                Some(format!(
                    "{points} point(s) awarded across two pilots against {} pickup(s) actually \
                     removed. Exactly one claim per pickup may score: with first-claim-wins gone \
                     BOTH scoreboards agree and both look right — only counting points against \
                     pickups reveals it. (a={} b={}, taken={taken:?})",
                    taken.len(),
                    a.client.score,
                    b.client.score,
                )),
            );
        }
        // ⚠️ The margins are the half of this line that can warn. The verdict above only ever
        // says yes or no, and both flake fixes were about a number sliding toward zero long
        // before it flipped — `travel_room` is derived from the same constants the comment on
        // `APPROACH` reasons about, so it cannot drift away from them the way prose did.
        let travel_room = (APPROACH - CLAIM_RADIUS) / (PLAYER_SPEED * std::f32::consts::SQRT_2);
        println!(
            "ok: two pilots flew onto {contested} pickup(s) and claimed together — {points} \
             point(s) awarded against {} removed, and both agree on which (a={} b={}). Margins: \
             the server's copy {worst_gap:.1} px into its {:.0} px reach, slowest frame {:.0} ms \
             against {:.0} ms of travel room, flight {worst_flight:.1} s of 12, first snapshot \
             {authority_wait:.2} s",
            taken.len(),
            a.client.score,
            b.client.score,
            CLAIM_RADIUS + CLAIM_SLACK,
            worst_iter * 1000.0,
            travel_room * 1000.0,
        );
    }

    // ── 7. The server tailors snapshots to the AOI, and shrinking it drains ───────────────────
    {
        let mut c = connect(&addr);
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && !c.client.have_authority {
            run_live(&mut [&mut c], 0.25);
        }
        if !c.client.have_authority {
            return finish(
                &mut child,
                7,
                Some(format!("never received a snapshot ({})", c.client.status)),
            );
        }

        // Park in the middle, where an AOI change has entities to find.
        c.client.prediction.pos = Vec2::new(WORLD_W * 0.5, WORLD_H * 0.5);

        c.client.aoi_radius = AOI_RADIUS_MIN;
        run_live(&mut [&mut c], 1.2);
        let tight = c.client.remotes.len();

        c.client.aoi_radius = AOI_RADIUS_MAX;
        run_live(&mut [&mut c], 1.5);
        let wide = c.client.remotes.len();

        if wide <= tight {
            return finish(
                &mut child,
                7,
                Some(format!(
                    "widening the AOI from {AOI_RADIUS_MIN:.0} to {AOI_RADIUS_MAX:.0} streamed \
                     {wide} entities against {tight} — the server is not tailoring snapshots per \
                     client, it is broadcasting the world and the radius is decoration."
                )),
            );
        }

        // And it must DRAIN — the half that fails when eviction is dead. Growing alone would pass
        // on a client that never removes anything, which is the flattering failure again.
        c.client.aoi_radius = AOI_RADIUS_MIN;
        run_live(&mut [&mut c], 1.5);
        let drained = c.client.remotes.len();
        if drained >= wide {
            return finish(
                &mut child,
                7,
                Some(format!(
                    "shrinking the AOI back to {AOI_RADIUS_MIN:.0} left {drained} entities of the \
                     {wide} that had streamed in. Nothing announces a departure, so only the \
                     last-seen timeout can remove them."
                )),
            );
        }
        println!(
            "ok: the server tailors snapshots to the AOI — {tight} entities at r={AOI_RADIUS_MIN:.0}, \
             {wide} at r={AOI_RADIUS_MAX:.0}, draining back to {drained} when it shrinks"
        );
    }

    finish(&mut child, 0, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prediction_moves_immediately_and_matches_the_shared_step() {
        let mut p = Prediction::new(Vec2::new(100.0, 100.0));
        let seq = p.predict(1.0, 0.0);
        assert_eq!(seq, 1);
        let want = step_position(100.0, 100.0, 1.0, 0.0);
        assert!((p.pos.x - want.0).abs() < 1e-4 && (p.pos.y - want.1).abs() < 1e-4);
        assert_eq!(p.pending_len(), 1);
    }

    #[test]
    fn reconcile_drops_acked_and_replays_unacked() {
        let start = Vec2::new(200.0, 200.0);
        let cmds = [(1.0_f32, 0.0_f32), (0.0, 1.0), (1.0, 1.0)];
        let mut p = Prediction::new(start);
        for &(mx, my) in &cmds {
            p.predict(mx, my);
        }

        // The server has applied inputs 1..=2 — compute its authoritative position.
        let mut sv = (start.x, start.y);
        sv = step_position(sv.0, sv.1, cmds[0].0, cmds[0].1);
        sv = step_position(sv.0, sv.1, cmds[1].0, cmds[1].1);
        p.reconcile(Vec2::new(sv.0, sv.1), 2);
        assert_eq!(p.pending_len(), 1, "only the unacked input remains");

        // Reconciled == the full continuous chain from the start. This is the property that makes
        // it invisible in play: correcting to the truth must not undo what you have already done.
        let mut full = (start.x, start.y);
        for &(mx, my) in &cmds {
            full = step_position(full.0, full.1, mx, my);
        }
        assert!(
            (p.pos.x - full.0).abs() < 1e-3 && (p.pos.y - full.1).abs() < 1e-3,
            "reconciled {:?} != full chain {full:?}",
            p.pos
        );
    }

    #[test]
    fn reconcile_with_everything_acked_snaps_to_the_server() {
        let mut p = Prediction::new(Vec2::ZERO);
        p.predict(1.0, 0.0);
        p.predict(1.0, 0.0);
        p.reconcile(Vec2::new(500.0, 300.0), 2);
        assert_eq!(p.pending_len(), 0);
        assert_eq!(p.pos, Vec2::new(500.0, 300.0));
    }

    #[test]
    fn a_stale_ack_leaves_the_pending_queue_alone() {
        let mut p = Prediction::new(Vec2::ZERO);
        for _ in 0..3 {
            p.predict(1.0, 0.0);
        }
        p.reconcile(Vec2::ZERO, 0); // the server has acked nothing yet
        assert_eq!(p.pending_len(), 3);
        // All three replayed from the origin.
        let mut want = (0.0_f32, 0.0_f32);
        for _ in 0..3 {
            want = step_position(want.0, want.1, 1.0, 0.0);
        }
        assert!((p.pos.x - want.0).abs() < 1e-3);
    }

    #[test]
    fn reset_forgets_pending_inputs() {
        let mut p = Prediction::new(Vec2::ZERO);
        p.predict(1.0, 0.0);
        p.predict(1.0, 0.0);
        p.reset(start_pos());
        assert_eq!(p.pending_len(), 0);
        assert_eq!(p.pos, start_pos());
        // Sequence numbering restarts, so the new server's acks mean what they say.
        assert_eq!(p.predict(0.0, 0.0), 1);
    }

    #[test]
    fn entity_colors_are_distinct_per_kind() {
        // Salvage must not read as a hazard — the game is unplayable if they look alike.
        let p = entity_color(Kind::Pickup, 0);
        let h = entity_color(Kind::Hazard, 0);
        let diff: f32 = (0..3).map(|i| (p[i] - h[i]).abs()).sum();
        assert!(diff > 0.8, "pickup {p:?} and hazard {h:?} are too close");
    }
}
