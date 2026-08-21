//! `survivor_game` — the top-down action + shooter genre-game of the rebuilt examples tree.
//!
//! Phase 3 of `plans/2026-08-19-examples-rebuild-plan.md`, and the one game that puts the engine
//! under **count** pressure. `platformer_game` owns motion and `rpg_quest_game` owns state that
//! must survive; this one owns *N*. Pool churn, spatial-grid rebuilds, steering for a crowd and the
//! 16-light cap are all N-dependent, and **none of them fail at N=3** — which is why every other
//! game in the tree can be correct while these are broken.
//!
//! ```text
//! cargo run --example survivor_game                       # play it
//! SURVIVOR_SELFTEST=1 cargo run --example survivor_game    # the acceptance test (headless)
//! ```
//!
//! It covers both of VISION's "shooter" and "top-down action" genres deliberately — the decision is
//! recorded in `docs/VISION.md` and the rebuild plan. The two exist to stress the same thing.
//!
//! # What this game runs that nothing else does
//!
//! v0.153.2 changed four render paths with no game to exercise them, and this is the first one that
//! does:
//!
//! | Path | How this game reaches it |
//! |---|---|
//! | GPU-particle **idle gate** | the player's thruster emitter is switched **off between waves**, so emitter count hits zero while its particles are still alive — the exact case the gate must not kill early |
//! | GPU-particle **capacity rebuild** | `GpuParticleConfig::capacity` is raised once at wave 3, on purpose, to see the documented "particles in flight are discarded" behaviour |
//! | **Shaping-cache key** | every kill spawns a `FloatingText` whose position changes every frame — the case the cache was built for and, before v0.153.2, missed every time |
//! | **UI-primitive path** | HUD text + floating text + an optional `DebugDraw` overlay + a `RenderTarget` minimap, all in one frame |
//!
//! # The 16-light cap is pressed on purpose
//!
//! Every enemy carries a small `PointLight`, so a live wave is well past `DEFAULT_MAX_LIGHTS`.
//! ⚠️ **The "nearest-to-camera light survives the cull" half is NOT checked here** — the selection
//! (`select_nearest_lights`) is private to the renderer and only its *pixels* are observable, so
//! that check lives in `tests/render.rs`, which runs under a GPU. The selftest below asserts only
//! what it can see from the `World`: that the cap is genuinely exceeded.

use engine::{
    AmbientLight, App, Camera, Collider, CollisionGridSystem, CollisionLayer, Color, DebugDraw,
    DrawText, Entity, FloatingText, FloatingTextSystem, HitFlash, HitFlashSystem, InputState,
    KeyCode, LightingConfig, ParticleBurst, ParticleEmitter, ParticleSystem, PointLight, Pool,
    PostProcessConfig, ProfilerData, Rng, Seek, ShouldQuit, SpatialGrid, Sprite, SpriteTrail,
    SpriteTrailSystem, SteeringSystem, SteeringVelocity, System, SystemConfig, TextQueue, Timer,
    Tonemap, Transform, Wander, WindowConfig, World, YSort, YSortSystem,
};
use glam::Vec2;

// ── Arena ───────────────────────────────────────────────────────────────────────────────────────

const WINDOW_W: u32 = 1024;
const WINDOW_H: u32 = 640;
/// The playfield, in world pixels. Enemies spawn on its edge and the player is clamped inside it.
const ARENA: Vec2 = Vec2::new(1024.0, 640.0);
const ARENA_MARGIN: f32 = 24.0;

const PLAYER_SPEED: f32 = 230.0;
const PLAYER_SIZE: Vec2 = Vec2::new(26.0, 26.0);
const PLAYER_MAX_HP: f32 = 100.0;

const ENEMY_SIZE: Vec2 = Vec2::new(22.0, 22.0);
const ENEMY_HP: f32 = 2.0;
const ENEMY_TOUCH_DPS: f32 = 26.0;

const BULLET_SIZE: Vec2 = Vec2::new(8.0, 8.0);
const BULLET_SPEED: f32 = 460.0;
const BULLET_LIFE: f32 = 1.4;
/// Pool capacity. Deliberately smaller than the number of shots a run fires, so the pool must
/// actually recycle — a capacity nobody reaches never proves recycling works.
const BULLET_POOL: usize = 48;
const FIRE_INTERVAL: f32 = 0.16;

const GRID_CELL: f32 = 64.0;
const LAYER_ENEMY: CollisionLayer = CollisionLayer(1 << 1);
const LAYER_PLAYER: CollisionLayer = CollisionLayer(1 << 0);

/// Wave pacing. Each wave adds enemies, so the light count and the grid load climb with it.
const WAVE_INTERVAL: f32 = 6.0;
const WAVE_BASE_ENEMIES: u32 = 10;
const WAVE_GROWTH: u32 = 6;
/// The thruster emitter is off for this long at the start of each wave — the window in which the
/// GPU-particle idle gate has zero emitters but live particles.
const THRUSTER_PAUSE: f32 = 1.2;

/// Default seed. `SURVIVOR_SEED=<n>` overrides it, which is what makes a run reproducible: same
/// seed, same waves, same positions, frame for frame.
const DEFAULT_SEED: u64 = 0x5EED_1234;

// ── Components ──────────────────────────────────────────────────────────────────────────────────

struct Player;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EnemyKind {
    /// Straight-line chaser — `Seek`.
    Chaser,
    /// Drifts on its own — `Wander`. Present so the crowd is not one behaviour wearing N hats.
    Drifter,
}

struct Enemy {
    hp: f32,
    kind: EnemyKind,
}

struct Bullet {
    velocity: Vec2,
    life: f32,
}

// ── Session ─────────────────────────────────────────────────────────────────────────────────────

struct Survivor {
    player: Entity,
    /// The GPU thruster emitter, despawned and respawned around each wave.
    thruster: Option<Entity>,
    wave: u32,
    kills: u32,
    shots: u32,
    health: f32,
    elapsed: f32,
    wave_timer: Timer,
    fire_timer: Timer,
    thruster_off_for: f32,
    rng: Rng,
    seed: u64,
    /// Set once the capacity rebuild has been triggered, so it happens exactly once.
    capacity_raised: bool,
    /// Debug overlay toggle (`Tab`) — the frame shape the UI-primitive allocation row needs.
    debug_overlay: bool,
    /// Peak metered level seen this run; the audio check reads it.
    audio_peak: f32,
    /// Spawn stream: `(x, y, kind)` in spawn order, as issued by the seeded `Rng`.
    ///
    /// Check 5 fingerprints **this**, not the enemies' current transforms. Measured why: a
    /// fingerprint taken from live positions still diverged between two seeds after the spawn
    /// point was sabotaged to a constant, because the *speeds* are drawn from the same stream and
    /// the crowd had moved. Positions after movement answer "is the simulation deterministic",
    /// which is a different and much easier question than "does the seed reach the spawner".
    ///
    /// Bounded: a game that ran for an hour would otherwise accumulate one entry per enemy for the
    /// whole session. `SPAWN_LOG_CAP` is far past anything a check needs.
    spawn_log: Vec<(i32, i32, EnemyKind)>,
}

/// Upper bound on `Survivor::spawn_log` — a diagnostic buffer, not a history.
const SPAWN_LOG_CAP: usize = 512;

impl Survivor {
    fn new(player: Entity, seed: u64) -> Self {
        Self {
            player,
            thruster: None,
            wave: 0,
            kills: 0,
            shots: 0,
            health: PLAYER_MAX_HP,
            elapsed: 0.0,
            // Fires immediately on the first tick, so wave 1 exists from frame 1.
            wave_timer: Timer::repeating(WAVE_INTERVAL),
            fire_timer: Timer::repeating(FIRE_INTERVAL),
            thruster_off_for: 0.0,
            rng: Rng::new(seed),
            seed,
            capacity_raised: false,
            debug_overlay: false,
            audio_peak: 0.0,
            spawn_log: Vec::new(),
        }
    }
}

fn seed_from_env() -> u64 {
    std::env::var("SURVIVOR_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SEED)
}

// ── Spawning ────────────────────────────────────────────────────────────────────────────────────

/// A point on the arena edge, from the run's seeded `Rng`.
///
/// Seeded rather than random so a run replays exactly: check 5 spawns two runs from one seed and
/// requires identical positions, then a third from a different seed and requires they diverge.
fn edge_spawn(rng: &mut Rng) -> Vec2 {
    let t = rng.f32_unit();
    match rng.range(0, 4) {
        0 => Vec2::new(t * ARENA.x, ARENA_MARGIN),
        1 => Vec2::new(t * ARENA.x, ARENA.y - ARENA_MARGIN),
        2 => Vec2::new(ARENA_MARGIN, t * ARENA.y),
        _ => Vec2::new(ARENA.x - ARENA_MARGIN, t * ARENA.y),
    }
}

/// Spawns one enemy. Every enemy carries a `PointLight`, which is how a live wave gets past the
/// 16-light cap without a contrived "16 lamps in a row" scene.
fn spawn_enemy(world: &mut World, pos: Vec2, kind: EnemyKind, target: Vec2, speed: f32) -> Entity {
    let e = world.spawn();
    world.add_component(
        e,
        Transform {
            position: pos,
            scale: ENEMY_SIZE,
            z: 0.3,
            ..Default::default()
        },
    );
    let tint = match kind {
        EnemyKind::Chaser => Color::rgb(0.92, 0.34, 0.38),
        EnemyKind::Drifter => Color::rgb(0.85, 0.55, 0.95),
    };
    world.add_component(e, Sprite::colored(tint.r, tint.g, tint.b));
    world.add_component(e, Enemy { hp: ENEMY_HP, kind });
    world.add_component(
        e,
        Collider::Aabb {
            half_extents: ENEMY_SIZE * 0.5,
        },
    );
    world.add_component(e, LAYER_ENEMY);
    world.add_component(e, YSort::default());
    world.add_component(
        e,
        SteeringVelocity {
            velocity: Vec2::ZERO,
            max_speed: speed,
        },
    );
    match kind {
        EnemyKind::Chaser => world.add_component(
            e,
            Seek {
                target,
                max_speed: speed,
            },
        ),
        EnemyKind::Drifter => world.add_component(
            e,
            Wander::new(speed * 0.75, 0.9).with_direction_fn(|idx, _prev| {
                // Deterministic per-entity direction: `Wander`'s hook is a plain `fn`, so the run
                // stays reproducible without threading the game's `Rng` through the engine.
                let a = idx as f32 * 2.399_963;
                Vec2::new(a.cos(), a.sin())
            }),
        ),
    }
    world.add_component(
        e,
        PointLight {
            color: tint,
            radius: 90.0,
            intensity: 0.8,
            light_height: 0.35,
        },
    );
    e
}

// ── The shield: a ShaderMaterial that is Hidden most of the time ────────────────────────────────
//
// Small on purpose. It exists because the "a `Hidden` `ShaderMaterial` keeps its GPU buffers"
// path (fixed in v0.153.2) has no other consumer in the tree — before this game, hiding a material
// sprite destroyed its params buffer and bind group and rebuilt them on the next reveal. Blinking
// it every time the player is hit is the shape that used to pay for that.

const SHIELD_SHADER: &str = r#"
@group(1) @binding(0) var t_sprite: texture_2d<f32>;
@group(1) @binding(1) var s_sprite: sampler;
@group(2) @binding(0) var<uniform> params: vec4<f32>;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv:    vec2<f32>,
    @location(1) color: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // A ring that pulses with params.x (time) and fades with params.y (intensity).
    let d = distance(in.uv, vec2<f32>(0.5, 0.5));
    let ring = smoothstep(0.50, 0.44, d) - smoothstep(0.44, 0.36, d);
    let pulse = 0.65 + 0.35 * sin(params.x * 12.0);
    return vec4<f32>(0.45, 0.85, 1.0, ring * pulse * params.y);
}
"#;

/// Marker for the shield sprite so the combat system can find it without a session field.
struct Shield;

// ── Systems ─────────────────────────────────────────────────────────────────────────────────────

const L_WAVE: &str = "game::wave";
const L_PLAYER: &str = "game::player";
const L_RETARGET: &str = "game::retarget";
const L_BULLETS: &str = "game::bullets";
const L_COMBAT: &str = "game::combat";
const L_CAMERA: &str = "game::camera";

/// Drives wave pacing, seeded spawning, and the two GPU-particle states this game exists to reach.
struct WaveSystem;

impl System for WaveSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some(player_pos) = world
            .resource::<Survivor>()
            .and_then(|s| world.get::<Transform>(s.player))
            .map(|t| t.position)
        else {
            return;
        };

        let (start_wave, wave, seed_bump) = {
            let Some(s) = world.resource_mut::<Survivor>() else {
                return;
            };
            s.elapsed += dt;
            s.wave_timer.tick(dt);
            let start = s.wave_timer.just_finished() || s.wave == 0;
            if start {
                s.wave += 1;
                // The thruster goes quiet at the start of every wave: emitters drop to zero while
                // the particles already in flight keep living. That window is the idle gate.
                s.thruster_off_for = THRUSTER_PAUSE;
            }
            (start, s.wave, s.wave.wrapping_mul(0x9E37_79B9))
        };

        if start_wave {
            let count = WAVE_BASE_ENEMIES + (wave.saturating_sub(1)) * WAVE_GROWTH;
            let mut spawns = Vec::with_capacity(count as usize);
            if let Some(s) = world.resource_mut::<Survivor>() {
                let _ = seed_bump;
                for i in 0..count {
                    let pos = edge_spawn(&mut s.rng);
                    // Every fourth one drifts; the rest chase. Derived from the same seeded stream,
                    // so the mix replays exactly too.
                    let kind = if i % 4 == 3 {
                        EnemyKind::Drifter
                    } else {
                        EnemyKind::Chaser
                    };
                    let speed = 52.0 + s.rng.range_f32(0.0, 26.0) + wave as f32 * 4.0;
                    if s.spawn_log.len() < SPAWN_LOG_CAP {
                        s.spawn_log
                            .push((pos.x.round() as i32, pos.y.round() as i32, kind));
                    }
                    spawns.push((pos, kind, speed));
                }
            }
            for (pos, kind, speed) in spawns {
                spawn_enemy(world, pos, kind, player_pos, speed);
            }
        }

        // ── The two GPU-particle states ────────────────────────────────────────────────────────
        let (off_for, thruster, wave_now, raised) = {
            let Some(s) = world.resource_mut::<Survivor>() else {
                return;
            };
            s.thruster_off_for = (s.thruster_off_for - dt).max(0.0);
            (s.thruster_off_for, s.thruster, s.wave, s.capacity_raised)
        };

        if off_for > 0.0 {
            // Despawn rather than `emit = false`: the gate must see *no emitter at all*, which is
            // the case that used to keep dispatching compute work for the rest of the process.
            if let Some(t) = thruster {
                world.despawn(t);
                if let Some(s) = world.resource_mut::<Survivor>() {
                    s.thruster = None;
                }
            }
        } else if thruster.is_none() {
            let t = spawn_thruster(world, player_pos);
            if let Some(s) = world.resource_mut::<Survivor>() {
                s.thruster = Some(t);
            }
        }

        // Raise the GPU particle capacity exactly once, at wave 3. Documented to rebuild the
        // renderer and discard particles in flight — this game is where that is first seen.
        if wave_now >= 3 && !raised {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(cfg) = world.resource_mut::<engine::GpuParticleConfig>() {
                cfg.capacity = 8192;
            }
            if let Some(s) = world.resource_mut::<Survivor>() {
                s.capacity_raised = true;
            }
        }
    }

    fn name(&self) -> &'static str {
        "WaveSystem"
    }
}

/// The player's GPU-particle thruster. Native-only: `GpuParticleEmitter` does not exist on wasm.
fn spawn_thruster(world: &mut World, pos: Vec2) -> Entity {
    let e = world.spawn();
    world.add_component(
        e,
        Transform {
            position: pos,
            scale: Vec2::splat(6.0),
            z: 0.1,
            ..Default::default()
        },
    );
    // Built field-by-field rather than with `..Default::default()`: the struct carries private
    // runtime fields (`timer`, `next_slot`), so functional-update syntax is a privacy error from
    // outside the crate. Same for `ParticleEmitter` below.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut emitter = engine::GpuParticleEmitter::default();
        emitter.spawn_rate = 90.0;
        emitter.lifetime = 0.7;
        emitter.velocity = Vec2::new(0.0, 40.0);
        emitter.velocity_spread = Vec2::splat(50.0);
        emitter.size = 5.0;
        emitter.color_start = Color::rgba(0.55, 0.85, 1.0, 0.9);
        emitter.color_end = Color::rgba(0.2, 0.35, 0.8, 0.0);
        world.add_component(e, emitter);
    }
    e
}

/// Moves the player, keeps the thruster and shield with them, and fires from the bullet pool.
struct PlayerSystem;

impl System for PlayerSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let (axis, toggle_debug, quit) = world
            .resource::<InputState>()
            .map(|i| {
                let mut a = Vec2::ZERO;
                if i.is_pressed(KeyCode::KeyA) || i.is_pressed(KeyCode::ArrowLeft) {
                    a.x -= 1.0;
                }
                if i.is_pressed(KeyCode::KeyD) || i.is_pressed(KeyCode::ArrowRight) {
                    a.x += 1.0;
                }
                if i.is_pressed(KeyCode::KeyW) || i.is_pressed(KeyCode::ArrowUp) {
                    a.y -= 1.0;
                }
                if i.is_pressed(KeyCode::KeyS) || i.is_pressed(KeyCode::ArrowDown) {
                    a.y += 1.0;
                }
                (
                    a,
                    i.just_pressed(KeyCode::Tab),
                    i.just_pressed(KeyCode::Escape),
                )
            })
            .unwrap_or((Vec2::ZERO, false, false));

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if toggle_debug {
            if let Some(s) = world.resource_mut::<Survivor>() {
                s.debug_overlay = !s.debug_overlay;
            }
        }

        let Some(player) = world.resource::<Survivor>().map(|s| s.player) else {
            return;
        };
        let mut pos = world
            .get::<Transform>(player)
            .map(|t| t.position)
            .unwrap_or_default();
        if axis != Vec2::ZERO {
            pos += axis.normalize_or_zero() * PLAYER_SPEED * dt;
            pos.x = pos.x.clamp(ARENA_MARGIN, ARENA.x - ARENA_MARGIN);
            pos.y = pos.y.clamp(ARENA_MARGIN, ARENA.y - ARENA_MARGIN);
            if let Some(t) = world.get_mut::<Transform>(player) {
                t.position = pos;
            }
        }

        // The thruster and the shield ride along.
        let thruster = world.resource::<Survivor>().and_then(|s| s.thruster);
        if let Some(t) = thruster {
            if let Some(tr) = world.get_mut::<Transform>(t) {
                tr.position = pos + Vec2::new(0.0, 12.0);
            }
        }
        let shields: Vec<Entity> = world.query::<Shield>().map(|(e, _)| e).collect();
        for shield in shields {
            if let Some(tr) = world.get_mut::<Transform>(shield) {
                tr.position = pos;
            }
        }

        // ── Auto-fire at the nearest enemy, from the pool ──────────────────────────────────────
        let fire = {
            let Some(s) = world.resource_mut::<Survivor>() else {
                return;
            };
            s.fire_timer.tick(dt);
            s.fire_timer.just_finished()
        };
        if !fire {
            return;
        }
        let nearest = world
            .query::<Enemy>()
            .filter_map(|(e, _)| world.get::<Transform>(e).map(|t| (e, t.position)))
            .min_by(|a, b| {
                a.1.distance_squared(pos)
                    .total_cmp(&b.1.distance_squared(pos))
            });
        let Some((_, target)) = nearest else {
            return;
        };
        let dir = (target - pos).normalize_or_zero();
        if dir == Vec2::ZERO {
            return;
        }
        fire_bullet(world, pos, dir);
    }

    fn name(&self) -> &'static str {
        "PlayerSystem"
    }
}

/// Acquires one bullet from the pool. The pool — not `spawn` — is the point: check 1 asserts every
/// acquired slot comes back, and a leak here starves the gun silently rather than crashing.
///
/// ⚠️ **`Pool` parks an entity; it does not strip it.** `release` adds a `Pooled` marker and keeps
/// the entity alive with **every component it had** — and nothing in the renderer skips `Pooled`.
/// So a parked bullet keeps its `Sprite` (drawn, sitting where it died), keeps its `Bullet`
/// (still ticked by `BulletSystem`, and still counted by anything asking "how many bullets are
/// live?"), and keeps its `Collider` (still in the grid, still hit-testable). Measured: check 1
/// first reported 8 live bullets and 8/48 free slots at rest, because parked ones were being
/// re-expired and re-released forever. Reactivation is `acquire`'s job, so deactivation has to be
/// the game's: this pair removes exactly what `setup` puts back.
fn fire_bullet(world: &mut World, from: Vec2, dir: Vec2) {
    let Some(mut pool) = world.remove_resource::<Pool>() else {
        return;
    };
    pool.acquire(world, |world, e| {
        world.remove_component::<engine::Hidden>(e);
        world.add_component(
            e,
            Transform {
                position: from,
                scale: BULLET_SIZE,
                z: 0.4,
                ..Default::default()
            },
        );
        world.add_component(e, Sprite::colored(1.0, 0.95, 0.6));
        world.add_component(
            e,
            Bullet {
                velocity: dir * BULLET_SPEED,
                life: BULLET_LIFE,
            },
        );
        world.add_component(
            e,
            Collider::Aabb {
                half_extents: BULLET_SIZE * 0.5,
            },
        );
        world.add_component(e, LAYER_PLAYER);
        world.add_component(e, SpriteTrail::new(0.02, 0.12).with_start_alpha(0.5));
    });
    world.insert_resource(pool);
    if let Some(s) = world.resource_mut::<Survivor>() {
        s.shots += 1;
    }
}

/// Returns a bullet to the pool, and *deactivates* it — see the note on `fire_bullet`.
///
/// Order matters: strip first, park second. `Pool::release` refuses a double release by checking
/// for `Pooled`, and removing `Bullet` here is what stops `BulletSystem` from finding a parked
/// entity and trying to release it again on the next frame.
fn release_bullet(world: &mut World, bullet: Entity) {
    let Some(mut pool) = world.remove_resource::<Pool>() else {
        return;
    };
    world.remove_component::<Bullet>(bullet);
    world.remove_component::<Collider>(bullet);
    world.remove_component::<SpriteTrail>(bullet);
    world.add_component(bullet, engine::Hidden);
    pool.release(bullet, world);
    world.insert_resource(pool);
}

/// Points every chaser at the player each frame. `Seek::target` is a position, not a handle, so a
/// moving target has to be re-aimed — a chase that "works" against a stationary player is the
/// classic false pass here.
struct RetargetSystem;

impl System for RetargetSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(pos) = world
            .resource::<Survivor>()
            .and_then(|s| world.get::<Transform>(s.player))
            .map(|t| t.position)
        else {
            return;
        };
        for (_e, seek) in world.query_mut::<Seek>() {
            seek.target = pos;
        }
    }

    fn name(&self) -> &'static str {
        "RetargetSystem"
    }
}

/// Advances bullets and expires them back into the pool.
struct BulletSystem;

impl System for BulletSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let mut expired = Vec::new();
        for (e, bullet, transform) in world.query2_mut::<Bullet, Transform>() {
            bullet.life -= dt;
            transform.position += bullet.velocity * dt;
            let p = transform.position;
            let out = p.x < 0.0 || p.y < 0.0 || p.x > ARENA.x || p.y > ARENA.y;
            if bullet.life <= 0.0 || out {
                expired.push(e);
            }
        }
        for e in expired {
            release_bullet(world, e);
        }
    }

    fn name(&self) -> &'static str {
        "BulletSystem"
    }
}

/// Bullet↔enemy and enemy↔player, both through the `SpatialGrid`.
///
/// The grid is the reason this is one system rather than two nested loops: at wave 5 there are
/// ~40 enemies and up to 48 bullets, and the N² version is the shape that only starts hurting at
/// the counts this game reaches.
struct CombatSystem;

impl System for CombatSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some((player, _)) = world.resource::<Survivor>().map(|s| (s.player, s.wave)) else {
            return;
        };
        let player_pos = world
            .get::<Transform>(player)
            .map(|t| t.position)
            .unwrap_or_default();

        // ── Bullets hit enemies ────────────────────────────────────────────────────────────────
        let bullets: Vec<(Entity, Vec2)> = world
            .query::<Bullet>()
            .filter_map(|(e, _)| world.get::<Transform>(e).map(|t| (e, t.position)))
            .collect();

        let mut hits: Vec<(Entity, Entity, Vec2)> = Vec::new();
        if let Some(grid) = world.resource::<SpatialGrid>() {
            for (bullet, pos) in &bullets {
                let near = grid.query_radius(*pos, 18.0, LAYER_ENEMY);
                if let Some(&enemy) = near.first() {
                    hits.push((*bullet, enemy, *pos));
                }
            }
        }

        let mut killed: Vec<(Entity, Vec2)> = Vec::new();
        for (bullet, enemy, at) in hits {
            // A bullet that already hit something this frame is gone; skip the second claim.
            if world.get::<Bullet>(bullet).is_none() {
                continue;
            }
            release_bullet(world, bullet);
            let dead = match world.get_mut::<Enemy>(enemy) {
                Some(e) => {
                    e.hp -= 1.0;
                    e.hp <= 0.0
                }
                None => continue,
            };
            if dead {
                let pos = world
                    .get::<Transform>(enemy)
                    .map(|t| t.position)
                    .unwrap_or(at);
                world.despawn(enemy);
                killed.push((enemy, pos));
            } else if world.get::<HitFlash>(enemy).is_none() {
                world.add_component(enemy, HitFlash::white(0.08));
            }
        }

        for (_enemy, pos) in &killed {
            spawn_kill_effects(world, *pos);
        }
        if !killed.is_empty() {
            let n = killed.len() as u32;
            if let Some(s) = world.resource_mut::<Survivor>() {
                s.kills += n;
            }
            kill_sound(world, n);
        }

        // ── Enemies touch the player ───────────────────────────────────────────────────────────
        let touching = world
            .resource::<SpatialGrid>()
            .map(|g| g.query_radius(player_pos, 22.0, LAYER_ENEMY).len())
            .unwrap_or(0);
        if touching > 0 {
            let dmg = ENEMY_TOUCH_DPS * dt * touching as f32;
            let dead = {
                let Some(s) = world.resource_mut::<Survivor>() else {
                    return;
                };
                s.health = (s.health - dmg).max(0.0);
                s.health <= 0.0
            };
            show_shield(world, true);
            if dead {
                respawn_player(world);
            }
        } else {
            show_shield(world, false);
        }
    }

    fn name(&self) -> &'static str {
        "CombatSystem"
    }
}

/// A kill's three visible consequences: a CPU particle burst, a floating damage number, and a
/// short-lived light. The floating text is the one that matters beyond the game — it **moves every
/// frame**, which is the text-shaping case v0.153.2 fixed and nothing else in the tree produces.
fn spawn_kill_effects(world: &mut World, pos: Vec2) {
    let burst = world.spawn();
    world.add_component(
        burst,
        Transform {
            position: pos,
            ..Default::default()
        },
    );
    let mut emitter = ParticleEmitter::burst();
    emitter.z = 0.6;
    world.add_component(burst, emitter);
    world.add_component(burst, ParticleBurst { remaining: 18 });

    engine::spawn_floating_text(
        world,
        pos,
        FloatingText::colored("+1", Color::rgb(1.0, 0.92, 0.45))
            .with_velocity(Vec2::new(0.0, -46.0))
            .with_lifetime(0.9)
            .with_size(20.0)
            .with_z(0.9),
    );
}

/// A metered tone per kill, ducking the music bus underneath it.
///
/// `play_tone_metered` rather than `play_tone_on_channel`: overlapping kills must **sum** on one
/// meter rather than cutting each other, which is the v0.140.0 lesson the deleted survivor paid
/// for. The level it produces is what check 3 reads.
fn kill_sound(world: &mut World, kills: u32) {
    let Some(audio) = world.resource_mut::<engine::Audio>() else {
        return;
    };
    let vol = (0.35 + 0.12 * kills as f32).min(0.9);
    audio.play_tone_metered(KILL_METER, 660.0, 0.09, vol, "sfx");
    audio.duck_bus("music", 0.55, 0.03);
    audio.release_bus("music", 0.5);
}

/// The meter name the kill tones share. Not a channel — see `play_tone_metered`'s docs.
const KILL_METER: &str = "kill";

/// Shows or hides the shield. Hiding is a `Hidden` component rather than a despawn precisely so the
/// `ShaderMaterial`'s GPU buffers survive it (v0.153.2); a despawn would prove nothing about that.
fn show_shield(world: &mut World, visible: bool) {
    let shields: Vec<Entity> = world.query::<Shield>().map(|(e, _)| e).collect();
    for shield in shields {
        let hidden = world.get::<engine::Hidden>(shield).is_some();
        if visible && hidden {
            world.remove_component::<engine::Hidden>(shield);
        } else if !visible && !hidden {
            world.add_component(shield, engine::Hidden);
        }
    }
}

fn respawn_player(world: &mut World) {
    let Some(player) = world.resource::<Survivor>().map(|s| s.player) else {
        return;
    };
    if let Some(t) = world.get_mut::<Transform>(player) {
        t.position = ARENA * 0.5;
    }
    if let Some(s) = world.resource_mut::<Survivor>() {
        s.health = PLAYER_MAX_HP;
    }
    // Clear the field rather than let the player die into a wall of enemies.
    let enemies: Vec<Entity> = world.query::<Enemy>().map(|(e, _)| e).collect();
    for e in enemies {
        world.despawn(e);
    }
}

/// Keeps the camera and the minimap's offscreen camera on the player.
struct CameraSystem;

impl System for CameraSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(pos) = world
            .resource::<Survivor>()
            .and_then(|s| world.get::<Transform>(s.player))
            .map(|t| t.position)
        else {
            return;
        };
        if let Some(cam) = world.resource_mut::<Camera>() {
            cam.position = pos - Vec2::new(WINDOW_W as f32, WINDOW_H as f32) * 0.5;
        }
        // The minimap looks at the whole arena from above, zoomed out — a second camera on a second
        // target, which is the configuration the pipeline-cache-keyed-by-format trap hides in.
        let cams: Vec<Entity> = world
            .query::<engine::OffscreenCamera>()
            .map(|(e, _)| e)
            .collect();
        for e in cams {
            if let Some(oc) = world.get_mut::<engine::OffscreenCamera>(e) {
                oc.camera.position = Vec2::ZERO;
            }
        }

        // The display sprite lives in world space, so it has to be re-pinned to the viewport's
        // bottom-right corner every frame as the camera follows the player.
        let cam_pos = world
            .resource::<Camera>()
            .map(|c| c.position)
            .unwrap_or_default();
        let corner = cam_pos + Vec2::new(WINDOW_W as f32, WINDOW_H as f32)
            - Vec2::new(MINIMAP_SIZE.0 as f32, MINIMAP_SIZE.1 as f32) * 0.5
            - Vec2::splat(16.0);
        let views: Vec<Entity> = world.query::<MinimapView>().map(|(e, _)| e).collect();
        for v in views {
            if let Some(t) = world.get_mut::<Transform>(v) {
                t.position = corner;
            }
        }
    }

    fn name(&self) -> &'static str {
        "CameraSystem"
    }
}

/// HUD + the optional debug overlay. Together with the floating text this is the frame shape the
/// UI-primitive allocation row needs: text, primitives and a render target in one frame.
struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((wave, kills, shots, health, debug, elapsed, seed)) =
            world.resource::<Survivor>().map(|s| {
                (
                    s.wave,
                    s.kills,
                    s.shots,
                    s.health,
                    s.debug_overlay,
                    s.elapsed,
                    s.seed,
                )
            })
        else {
            return;
        };
        let enemies = world.query::<Enemy>().count();
        let lights = world.query::<PointLight>().count();
        let live_bullets = world.query::<Bullet>().count();
        let free = world
            .resource::<Pool>()
            .map(|p| p.available_count())
            .unwrap_or(0);
        let frame_ms = world
            .resource::<ProfilerData>()
            .map(|p| p.frame_ms)
            .unwrap_or(0.0);

        if debug {
            // Spawn ring + arena bounds. `DebugDraw` is immediate-mode data the App converts at the
            // render stage, so a system may push freely without touching the renderer.
            if let Some(dd) = world.resource_mut::<DebugDraw>() {
                dd.rect(
                    Vec2::new(ARENA_MARGIN, ARENA_MARGIN),
                    ARENA - Vec2::splat(ARENA_MARGIN * 2.0),
                    [90, 200, 255, 140],
                );
            }
        }

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        tq.push(DrawText::new(
            "WASD move   auto-fire   Tab debug overlay   Esc quit",
            Vec2::new(16.0, 14.0),
            17.0,
            [224, 236, 250, 230],
        ));
        tq.push(DrawText::new(
            format!(
                "wave {wave}   kills {kills}   shots {shots}   hp {health:.0}   enemies {enemies}  \
                 lights {lights}   bullets {live_bullets} live / {free} free   {frame_ms:.1} ms   \
                 seed {seed:#x}"
            ),
            Vec2::new(16.0, 38.0),
            17.0,
            [168, 208, 255, 220],
        ));
        if elapsed < 3.0 {
            tq.push(DrawText::new(
                "every enemy carries a light — the 16-light cap is being pressed on purpose",
                Vec2::new(16.0, 62.0),
                15.0,
                [150, 180, 210, 190],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "HudSystem"
    }
}

// ── Setup ───────────────────────────────────────────────────────────────────────────────────────

const MINIMAP: &str = "minimap";
const MINIMAP_SIZE: (u32, u32) = (192, 120);

/// Marker for the sprite that displays the minimap render target.
struct MinimapView;

fn build_app() -> App {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — survivor_game".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.04, 0.05, 0.08, 1.0],
    });

    // Dark ambient so the per-enemy lights actually read; the cap stays at its default so the game
    // presses it rather than raising it out of the way.
    app.world.insert_resource(AmbientLight {
        color: Color::WHITE,
        intensity: 0.22,
    });
    app.world.insert_resource(LightingConfig::default());
    app.world.insert_resource(PostProcessConfig {
        enabled: true,
        bloom: true,
        bloom_threshold: 0.62,
        bloom_intensity: 1.15,
        tonemap: Tonemap::AcesFilmic,
        vignette_strength: 0.35,
        ..Default::default()
    });

    // Audio is optional: a box with no device still plays the game, and the selftest's audio check
    // skips rather than fails there.
    if let Some(audio) = engine::Audio::new() {
        let mut audio = audio;
        audio.enable_analysis(KILL_METER);
        audio.set_bus_volume("sfx", 0.8);
        audio.set_bus_volume("music", 0.5);
        app.world.insert_resource(audio);
    }

    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: ARENA * 0.5,
            scale: PLAYER_SIZE,
            z: 0.5,
            ..Default::default()
        },
    );
    app.world
        .add_component(player, Sprite::colored(0.55, 0.85, 1.0));
    app.world.add_component(player, Player);
    app.world.add_component(player, YSort::default());
    app.world.add_component(
        player,
        Collider::Aabb {
            half_extents: PLAYER_SIZE * 0.5,
        },
    );
    app.world.add_component(player, LAYER_PLAYER);
    app.world.add_component(
        player,
        PointLight {
            color: Color::rgb(0.7, 0.9, 1.0),
            radius: 240.0,
            intensity: 1.4,
            light_height: 0.5,
        },
    );

    // The shield starts hidden; `CombatSystem` reveals it while the player is being touched.
    let shield = app.world.spawn();
    app.world.add_component(
        shield,
        Transform {
            position: ARENA * 0.5,
            scale: Vec2::splat(64.0),
            z: 0.55,
            ..Default::default()
        },
    );
    app.world
        .add_component(shield, Sprite::colored(1.0, 1.0, 1.0));
    app.world.add_component(
        shield,
        engine::ShaderMaterial::new(SHIELD_SHADER, [0.0, 1.0, 0.0, 0.0]),
    );
    app.world.add_component(shield, Shield);
    app.world.add_component(shield, engine::Hidden);

    app.world.insert_resource(Pool::new(BULLET_POOL));
    app.world
        .insert_resource(Survivor::new(player, seed_from_env()));
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    // The minimap: a second camera drawing the arena into its own target. Present because the
    // pipeline-cache-keyed-by-target-format trap is invisible without one.
    app.create_render_target(MINIMAP, MINIMAP_SIZE.0, MINIMAP_SIZE.1);
    let minimap = app.world.spawn();
    let mut mini_cam = Camera::new(Vec2::ZERO, 0.18);
    mini_cam.lerp_factor = 0.0;
    app.world.add_component(
        minimap,
        engine::OffscreenCamera {
            target: MINIMAP.to_string(),
            camera: mini_cam,
            // Layer 0 only. The display sprite below sits on layer 1, so the minimap does not
            // render a picture of itself in its own corner.
            layer_mask: 1 << 0,
        },
    );

    // Displaying it is a `Sprite` whose texture key is the target's name — the renderer checks its
    // render-target cache before its texture cache. Without this the offscreen pass still runs (and
    // still exercises the pipeline-cache-by-target-format trap), but nothing samples the result, so
    // half the path would be untested and the feature invisible.
    // A backdrop, or the minimap reads as scattered noise over the black arena. Same layer as the
    // view so it is excluded from the offscreen pass too.
    let minimap_back = app.world.spawn();
    app.world.add_component(
        minimap_back,
        Transform {
            position: Vec2::ZERO,
            scale: Vec2::new(MINIMAP_SIZE.0 as f32 + 8.0, MINIMAP_SIZE.1 as f32 + 8.0),
            z: 0.94,
            ..Default::default()
        },
    );
    app.world
        .add_component(minimap_back, Sprite::colored(0.13, 0.15, 0.22));
    app.world
        .add_component(minimap_back, engine::RenderLayer(1));
    app.world.add_component(minimap_back, MinimapView);

    let minimap_view = app.world.spawn();
    app.world.add_component(
        minimap_view,
        Transform {
            position: Vec2::ZERO, // placed against the camera every frame by `CameraSystem`
            scale: Vec2::new(MINIMAP_SIZE.0 as f32, MINIMAP_SIZE.1 as f32),
            z: 0.95,
            ..Default::default()
        },
    );
    app.world
        .add_component(minimap_view, Sprite::textured(MINIMAP));
    app.world
        .add_component(minimap_view, engine::RenderLayer(1));
    app.world.add_component(minimap_view, MinimapView);

    // ── Schedule ───────────────────────────────────────────────────────────────────────────────
    app.add_system_labeled(WaveSystem, SystemConfig::new().label(L_WAVE));
    app.add_system_labeled(
        PlayerSystem,
        SystemConfig::new().label(L_PLAYER).after(L_WAVE),
    );
    app.add_system_labeled(
        RetargetSystem,
        SystemConfig::new().label(L_RETARGET).after(L_PLAYER),
    );
    app.add_system_labeled(
        SteeringSystem::new(),
        SystemConfig::new()
            .label(SteeringSystem::LABEL)
            .after(L_RETARGET),
    );
    app.add_system_labeled(
        BulletSystem,
        SystemConfig::new()
            .label(L_BULLETS)
            .after(SteeringSystem::LABEL),
    );
    // The grid must be rebuilt from post-movement transforms, and combat reads the grid.
    app.add_system_labeled(
        CollisionGridSystem::new(GRID_CELL),
        SystemConfig::new()
            .label(CollisionGridSystem::LABEL)
            .after(L_BULLETS),
    );
    app.add_system_labeled(
        CombatSystem,
        SystemConfig::new()
            .label(L_COMBAT)
            .after(CollisionGridSystem::LABEL),
    );
    app.add_system_labeled(
        CameraSystem,
        SystemConfig::new().label(L_CAMERA).after(L_COMBAT),
    );
    app.add_system_labeled(
        ParticleSystem,
        SystemConfig::new().label(ParticleSystem::LABEL),
    );
    app.add_system(FloatingTextSystem);
    app.add_system(SpriteTrailSystem);
    app.add_system(HitFlashSystem);
    app.add_system(YSortSystem);
    app.add_system(engine::AudioFacadeSystem);
    app.add_system(HudSystem);
    app
}

// Only the wasm entry points use this. Gated so a native build does not compile a module it
// cannot reach — and so `cargo clippy --all-targets -D warnings` has nothing dead to complain
// about, which is how the two constants above were caught.
#[cfg(target_arch = "wasm32")]
#[path = "../shared/web_check.rs"]
mod web_check;

fn main() {
    if std::env::var("SURVIVOR_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }
    build_app().run();
}

// ── The web build ───────────────────────────────────────────────────────────────────────────────

/// The meter the browser audio check drives. Separate from [`KILL_METER`] on purpose: kills fire on
/// their own schedule, so measuring them would make the check's result depend on whether an enemy
/// happened to die inside the window.
#[cfg(target_arch = "wasm32")]
const WEB_METER: &str = "webcheck";

/// A deliberately **low** tone. The band assertion below is "the spectrum leans low", which is only
/// meaningful against a signal that actually is low — 110 Hz is the frequency the deleted
/// `audio_reactive` browser smoke used when it measured `low=9.41 / high=0.00`, and reusing it
/// keeps this comparable to the number that was lost.
#[cfg(target_arch = "wasm32")]
const WEB_TONE_HZ: f32 = 110.0;

/// Plays the game in a browser. Called from `web/index.html`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_survivor_game() {
    build_app().run();
}

/// Runs the game **and** the Web Audio check, publishing its verdict to `document.title` for
/// `scripts/survivor_audio_web_smoke.sh`.
///
/// # Why this is the first browser smoke rebuilt
///
/// Deleting the examples tree on 2026-08-19 removed a lot of aspiration and exactly one *working
/// measurement*: Web Audio genuinely gated from v0.143.17 to v0.153.0. Native rodio/ALSA cannot be
/// tested in CI (five runs, v0.143.10 — the table is in `docs/VERIFICATION.md`), so the browser
/// half was the only automated audio evidence the repo ever had, and since the deletion there has
/// been none of any kind.
///
/// # The two halves, and why one is not enough
///
/// - **A live level.** `Audio::levels(WEB_METER).rms > 0` says sound reached the meter.
/// - **A low-biased spectrum.** `Audio::bands` must lean toward the low end for a 110 Hz tone.
///
/// The level alone would pass on a backend that reports a plausible number without analysing
/// anything, and the spectrum alone would pass on one that fills a fixed curve. Together they say
/// the analyser is looking at *this* signal. That is the property the deleted smoke measured, and
/// it is why it caught things a "did the page load" check never would.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn web_check_survivor() {
    use web_check::{Step, WebCheck};

    let mut app = build_app();

    // Enough time for the AudioContext to unlock and for a few analysis windows to publish. The
    // deadline is generous because a CI runner under load is slower than a desktop, and the cost of
    // being generous is only paid when something is already broken.
    const DEADLINE: f32 = 20.0;
    // Let the app warm up before making noise — on wasm the adapter resolves asynchronously and
    // the first frames run with no GPU at all.
    const FIRE_AT: f32 = 1.0;

    let mut fired = false;
    let mut best_rms = 0.0_f32;
    let mut bands = [0.0_f32; 8];

    app.add_system(WebCheck::new("AUDIO_CHECK", DEADLINE, move |world, t| {
        let Some(audio) = world.resource_mut::<engine::Audio>() else {
            // `Audio::new()` returned None — in a browser that is a real failure, not the
            // "no device" skip the native selftest allows. A tab always has Web Audio.
            return Step::fail("no Audio resource — Audio::new() failed in the browser");
        };

        if !fired {
            if t < FIRE_AT {
                return Step::Pending;
            }
            audio.enable_analysis(WEB_METER);
            audio.enable_spectrum(WEB_METER);
            audio.play_tone_metered(WEB_METER, WEB_TONE_HZ, 6.0, 0.9, "sfx");
            fired = true;
            return Step::Pending;
        }

        best_rms = best_rms.max(audio.levels(WEB_METER).rms);
        let mut now = [0.0_f32; 8];
        audio.bands(WEB_METER, &mut now);
        // Keep the strongest spectrum seen rather than the newest: the bands are smoothed and the
        // tone is finite, so sampling only the last frame can land after it has decayed.
        if now.iter().sum::<f32>() > bands.iter().sum::<f32>() {
            bands = now;
        }

        let low = bands[0] + bands[1] + bands[2];
        let high = bands[5] + bands[6] + bands[7];

        if best_rms > 0.0 && low > 0.0 && low > high * 2.0 {
            return Step::pass(format!(
                "rms {best_rms:.4}, low {low:.3} vs high {high:.3} on a {WEB_TONE_HZ:.0} Hz tone"
            ));
        }
        // Report what is currently visible, so a timeout says WHICH half never arrived. Without
        // this every sabotage of this check produces the same 'no verdict' message, and a matrix
        // where every row fails identically has verified the deadline rather than the assertions —
        // the trap in docs/VERIFICATION.md § A sabotage that fails the wrong check.
        Step::Waiting(format!(
            "rms {best_rms:.4} (want above 0) and low {low:.3} vs high {high:.3} (want low at \
             least 2x high)"
        ))
    }));

    app.run();
}

// ── Acceptance test ─────────────────────────────────────────────────────────────────────────────
//
// `SURVIVOR_SELFTEST=1 cargo run --example survivor_game`, and `scripts/selftests.sh` in the gate.
//
// Everything here is **seeded and invariant-based**. The counts this game runs at are the point, so
// a check that pins a number ("42 enemies at wave 3") would break on any tuning change while
// telling you nothing; each check below asserts a property that holds at every N.
//
// Exit codes: 0 pass · 1 the bullet pool leaks · 2 the light cap is not actually pressed · 3 the
// kill tone never reaches the meter · 4 chasers do not close distance · 5 a seeded run is not
// reproducible, or two seeds do not diverge · 6 the GPU-particle idle window never happens ·
// 7 floating text leaks or does not move.

const DT: f32 = 1.0 / 60.0;

fn session(app: &App) -> &Survivor {
    app.world
        .resource::<Survivor>()
        .expect("Survivor resource missing")
}

fn step(app: &mut App, frames: u32) {
    for _ in 0..frames {
        app.step_headless(DT);
    }
}

fn player_pos(app: &App) -> Vec2 {
    let p = session(app).player;
    app.world
        .get::<Transform>(p)
        .map(|t| t.position)
        .unwrap_or_default()
}

/// The run's spawn stream — position and kind at the moment each enemy was drawn from the seeded
/// `Rng`. See `Survivor::spawn_log` for why this is not read off the live transforms.
fn spawn_fingerprint(app: &App) -> Vec<(i32, i32, EnemyKind)> {
    session(app).spawn_log.clone()
}

fn self_test() -> i32 {
    // ── 1. The bullet pool gets every slot back ────────────────────────────────────────────────
    //
    // The invariant, not a number. Two things make the obvious formulation wrong:
    //
    // - **`Pool::new(n)` does not preallocate.** `n` is a cap; the pool starts empty and spawns on
    //   demand, so "free == capacity at rest" is false for a healthy pool. Measured: this game's
    //   working set is ~7 bullets because they die fast, and the first version of this check failed
    //   at `7/48` against a pool that was behaving perfectly.
    // - A background wave keeps spawning, so any absolute count moves on its own.
    //
    // What cannot move is that a slot acquired comes back and is *reused*. So: run to quiescence,
    // record the resting set, fire another full cycle, and require the resting set to be the same.
    // A leak makes `acquire` spawn fresh entities on the second cycle and the number grows.
    {
        let mut app = build_app();
        step(&mut app, 4);

        // Fire, then silence the gun and let everything in flight expire.
        let quiesce = |app: &mut App| -> (usize, usize) {
            if let Some(s) = app.world.resource_mut::<Survivor>() {
                s.fire_timer = Timer::repeating(1_000.0);
            }
            step(app, (BULLET_LIFE / DT) as u32 + 30);
            let free = app
                .world
                .resource::<Pool>()
                .map(|p| p.available_count())
                .unwrap_or(0);
            (free, app.world.query::<Bullet>().count())
        };
        let resume = |app: &mut App| {
            if let Some(s) = app.world.resource_mut::<Survivor>() {
                s.fire_timer = Timer::repeating(FIRE_INTERVAL);
            }
        };

        step(&mut app, 300);
        let (free_a, live_a) = quiesce(&mut app);
        let shots_a = session(&app).shots;

        resume(&mut app);
        step(&mut app, 300);
        let (free_b, live_b) = quiesce(&mut app);
        let shots_b = session(&app).shots;

        let recycled = shots_b > free_b as u32 * 2;
        if live_a != 0 || live_b != 0 || free_b != free_a || free_a == 0 || !recycled {
            eprintln!(
                "FAIL: the bullet pool leaked — resting free slots {free_a} then {free_b} (want \
                 the same, and non-zero), live bullets at rest {live_a} then {live_b} (want 0), \
                 {shots_a} then {shots_b} shots through a resting set of {free_b} (want many more \
                 shots than slots, or nothing was recycled)"
            );
            return 1;
        }
        println!(
            "ok: the bullet pool recycles and loses nothing — {shots_b} shots through a resting \
             set of {free_b} slots, identical across two full cycles, 0 live at rest"
        );
    }

    // ── 2. The 16-light cap is genuinely pressed ───────────────────────────────────────────────
    //
    // ⚠️ This asserts the *setup*, not the cull. Which lights survive is decided by a private
    // renderer function and is only observable in pixels, so that half lives in
    // `tests/render.rs::nearest_light_survives_the_cap`, which runs under a GPU. Without this
    // check, though, that one could pass against a scene that never exceeded the cap at all.
    {
        let mut app = build_app();
        // Silence the gun and keep topping the player up: otherwise the crowd is thinned by kills
        // and by the death-clear, and the count never builds. Measured — the first version read 4
        // lights after two waves because the field had been wiped twice.
        if let Some(s) = app.world.resource_mut::<Survivor>() {
            s.fire_timer = Timer::repeating(1_000.0);
        }
        // Past the second wave (one every 6 s at 60 fps).
        for _ in 0..400 {
            app.step_headless(DT);
            if let Some(s) = app.world.resource_mut::<Survivor>() {
                s.health = PLAYER_MAX_HP;
            }
        }
        let lights = app.world.query::<PointLight>().count();
        let cap = app
            .world
            .resource::<LightingConfig>()
            .map(|c| c.max_lights)
            .unwrap_or(0);
        if cap == 0 || lights <= cap {
            eprintln!(
                "FAIL: the light cap is not being pressed — {lights} PointLights against a cap of \
                 {cap}. The render-side cull check has nothing to cull."
            );
            return 2;
        }
        println!(
            "ok: {lights} lights against a cap of {cap} — the cull path is under real pressure"
        );
    }

    // ── 3. A kill tone reaches the meter ───────────────────────────────────────────────────────
    //
    // ⚠️ **Paced off a wall clock, and it has to be.** The meter is published by the audio thread in
    // real time while a headless loop advances at a fixed 1/60 dt as fast as the CPU allows — so a
    // frame-counted loop reads 0.0 while the tone is playing correctly. This is the trap in
    // `CLAUDE.md` and `docs/MODULE_MAP.md`'s audio row, and it cost two separate investigations
    // before it was written down.
    //
    // The device probe is a throwaway `Audio::new()` taken **before the app is built**: sampling
    // "no device" from the thing under test would let a real metering failure forge a skip.
    {
        use std::time::{Duration, Instant};

        let has_device = engine::Audio::new().is_some();
        if !has_device {
            println!("SKIP: no audio device — the kill-tone meter cannot be observed");
        } else {
            let mut app = build_app();
            step(&mut app, 30);
            // Fire the tone through the same path a kill uses.
            kill_sound(&mut app.world, 3);

            let deadline = Instant::now() + Duration::from_secs(2);
            let mut peak = 0.0_f32;
            while Instant::now() < deadline {
                app.step_headless(DT);
                if let Some(audio) = app.world.resource::<engine::Audio>() {
                    peak = peak.max(audio.levels(KILL_METER).rms);
                }
                if peak > 0.0 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(4));
            }
            if let Some(s) = app.world.resource_mut::<Survivor>() {
                s.audio_peak = peak;
            }
            if peak <= 0.0 {
                eprintln!(
                    "FAIL: a kill tone never reached the meter — rms stayed {peak:.4} over 2 s of \
                     wall clock with a device present. Check `enable_analysis` runs before the \
                     first play, and that the loop is paced off `Instant` rather than frames."
                );
                return 3;
            }
            println!(
                "ok: a kill tone reaches the metered channel (rms {peak:.4} on a real device)"
            );
        }
    }

    // ── 4. Chasers close on where the player *is now* ──────────────────────────────────────────
    //
    // ⚠️ **The geometry is the check**, and it took three tries to make it discriminate. Earlier
    // versions passed with `RetargetSystem` disabled:
    //
    // 1. Player parked where they spawned — the stale target already *was* the right answer.
    // 2. Player jumped corner-to-corner — enemies spawn on the edge and their stale target is the
    //    centre, which lies between them and the new corner, so they closed on it by accident.
    //
    // What discriminates: let the crowd **arrive** at the stale target first, then step away.
    // Standing still becomes the visible consequence, and the distance only falls if something
    // re-aimed them. The cohort is also pinned to the enemies alive at the start — later waves
    // spawn on the far edge and would drag the mean back up for reasons unrelated to steering.
    {
        let mut app = build_app();
        step(&mut app, 10);
        if let Some(s) = app.world.resource_mut::<Survivor>() {
            s.fire_timer = Timer::repeating(1_000.0); // no kills during the measurement
        }
        let player = session(&app).player;
        let cohort: Vec<Entity> = app
            .world
            .query::<Enemy>()
            .filter(|(_, e)| e.kind == EnemyKind::Chaser)
            .map(|(e, _)| e)
            .collect();
        if cohort.is_empty() {
            eprintln!("FAIL: no chasers spawned at all — wave 1 did not run");
            return 4;
        }
        let cohort_distance = |app: &App, to: Vec2| -> f32 {
            let ds: Vec<f32> = cohort
                .iter()
                .filter_map(|e| {
                    app.world
                        .get::<Transform>(*e)
                        .map(|t| t.position.distance(to))
                })
                .collect();
            if ds.is_empty() {
                return f32::MAX;
            }
            ds.iter().sum::<f32>() / ds.len() as f32
        };
        let hold = |app: &mut App, at: Vec2, frames: u32| {
            for _ in 0..frames {
                if let Some(t) = app.world.get_mut::<Transform>(player) {
                    t.position = at;
                }
                app.step_headless(DT);
                if let Some(s) = app.world.resource_mut::<Survivor>() {
                    s.health = PLAYER_MAX_HP; // no death-clear either
                }
            }
            if let Some(t) = app.world.get_mut::<Transform>(player) {
                t.position = at;
            }
        };

        // Phase 1: hold at the centre — where the cohort's spawn-time target already points — until
        // they have crossed the arena and bunched up. At ~55-80 px/s over ~400 px that is seconds,
        // not frames, which is why this is 600 and not 60.
        let centre = ARENA * 0.5;
        hold(&mut app, centre, 600);
        let bunched = cohort_distance(&app, centre);

        // Phase 2: step away. A stale target leaves them sitting in the middle.
        let away = Vec2::new(ARENA.x - ARENA_MARGIN * 2.0, ARENA.y - ARENA_MARGIN * 2.0);
        if let Some(t) = app.world.get_mut::<Transform>(player) {
            t.position = away;
        }
        let before = cohort_distance(&app, away);
        hold(&mut app, away, 90);
        let after = cohort_distance(&app, away);

        // Control: without convergence, phase 2 proves nothing — say so rather than pass.
        if bunched > 140.0 {
            eprintln!(
                "FAIL: the cohort never converged on the parked player — mean distance \
                 {bunched:.1} px after 600 frames. Phase 2 cannot discriminate from here, so this \
                 check would pass vacuously."
            );
            return 4;
        }
        if before - after < 40.0 {
            eprintln!(
                "FAIL: chasers did not follow the player after they moved — mean distance to the \
                 new position {before:.1} -> {after:.1} px in 90 frames ({:.1} px of approach, \
                 want more than 40). A stale `Seek::target` leaves them standing where the player \
                 used to be.",
                before - after
            );
            return 4;
        }
        println!(
            "ok: chasers re-aim and follow — the wave-1 cohort bunched to {bunched:.1} px around \
             the parked player, then closed {before:.1} -> {after:.1} px after they stepped away"
        );
    }

    // ── 5. A seed replays exactly, and a different seed does not ───────────────────────────────
    //
    // Both halves. "Same seed, same run" alone passes on a game that ignores the seed entirely and
    // spawns everything at one fixed point — which is why the divergence half is here.
    {
        // The gun is silenced and the player kept alive, so the fingerprint is the **spawn stream**
        // rather than whoever happened to survive: attrition is deterministic too, but folding it
        // in would make the check fail for combat reasons and report them as a seeding bug.
        let run = |seed: u64| -> Vec<(i32, i32, EnemyKind)> {
            let mut app = build_app();
            if let Some(s) = app.world.resource_mut::<Survivor>() {
                *s = Survivor::new(s.player, seed);
                s.fire_timer = Timer::repeating(1_000.0);
            }
            for _ in 0..400 {
                app.step_headless(DT);
                if let Some(s) = app.world.resource_mut::<Survivor>() {
                    s.health = PLAYER_MAX_HP;
                }
            }
            spawn_fingerprint(&app)
        };
        let a = run(0xA11CE);
        let b = run(0xA11CE);
        let c = run(0xB0B);
        if a.is_empty() || a != b {
            eprintln!(
                "FAIL: one seed did not replay — {} spawns vs {}, first difference at {:?}",
                a.len(),
                b.len(),
                a.iter().zip(b.iter()).position(|(x, y)| x != y)
            );
            return 5;
        }
        if a == c {
            eprintln!(
                "FAIL: two different seeds produced identical spawns ({} of them) — the seed is \
                 not reaching the spawner",
                a.len()
            );
            return 5;
        }
        let kinds = a
            .iter()
            .filter(|(_, _, k)| *k == EnemyKind::Drifter)
            .count();
        println!(
            "ok: a seed replays exactly ({} spawns, {kinds} of them drifters) and a different seed \
             diverges",
            a.len()
        );
    }

    // ── 6. The GPU-particle idle window actually happens ───────────────────────────────────────
    //
    // ⚠️ Asserts the *state*, not the gate. Whether the renderer skips its dispatch is GPU-side and
    // invisible from here; what this guarantees is that the state the gate has to handle — zero
    // emitters while particles are still in flight — is reached by simply playing the game, so the
    // path is exercised rather than merely present.
    {
        let mut app = build_app();
        let mut saw_emitter = false;
        let mut saw_none_after = false;
        for _ in 0..500 {
            app.step_headless(DT);
            let has = session(&app).thruster.is_some();
            if has {
                saw_emitter = true;
            } else if saw_emitter {
                saw_none_after = true;
            }
        }
        if !saw_emitter || !saw_none_after {
            eprintln!(
                "FAIL: the GPU-particle idle window never happened — emitter seen: {saw_emitter}, \
                 gone again afterwards: {saw_none_after}. The thruster is supposed to despawn at \
                 the start of every wave."
            );
            return 6;
        }
        println!("ok: the thruster emitter despawns and respawns, so the idle gate is exercised");
    }

    // ── 7. Floating text moves, then goes away ─────────────────────────────────────────────────
    //
    // The movement half is what makes this game the shaping cache's first real consumer: a damage
    // number whose position changes every frame is the case the cache key used to miss. The
    // disappearance half is a leak check — a `FloatingText` that never expires accumulates one
    // entity per kill for the rest of the run.
    {
        let mut app = build_app();
        step(&mut app, 20);
        let at = player_pos(&app) + Vec2::new(40.0, 0.0);
        spawn_kill_effects(&mut app.world, at);

        let first = app
            .world
            .query::<FloatingText>()
            .next()
            .map(|(e, _)| e)
            .expect("a kill spawns a floating text");
        let y0 = app
            .world
            .get::<Transform>(first)
            .map(|t| t.position.y)
            .unwrap_or_default();
        step(&mut app, 12);
        let y1 = app
            .world
            .get::<Transform>(first)
            .map(|t| t.position.y)
            .unwrap_or(y0);
        let moved = (y1 - y0).abs();

        // Well past its 0.9 s lifetime, with the gun silenced so no new ones appear.
        if let Some(s) = app.world.resource_mut::<Survivor>() {
            s.fire_timer = Timer::repeating(1_000.0);
        }
        step(&mut app, 120);
        let left = app.world.query::<FloatingText>().count();

        if moved < 4.0 || left != 0 {
            eprintln!(
                "FAIL: floating text did not move ({moved:.1} px in 12 frames, want more than 4) \
                 or leaked ({left} still alive after 2 s, want 0)"
            );
            return 7;
        }
        println!(
            "ok: a kill's floating text moves {moved:.1} px in 12 frames and expires cleanly \
             ({left} left)"
        );
    }

    0
}
