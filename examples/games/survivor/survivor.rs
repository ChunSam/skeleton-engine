//! Top-down twin-stick survival example (candidate F from docs/NEXT_WORK.md).
//!
//! Where the shooter (candidate D) first stressed `engine::Pool`, `SpatialGrid`
//! and `Timer`, this example adds the *depth* surfaces: **steering under many
//! simultaneous entities** (`engine::Seek` + `SteeringSystem` driving ~100-200
//! seekers), the **native-only GPU particle path** (`GpuParticleEmitter`, a
//! player thruster) that the CPU `ParticleSystem` never touches, and **perf
//! visibility** via `ProfilerData.frame_ms` in the HUD.
//!
//! It reuses the shooter's proven shape wholesale: persistent ECS `Sprite`
//! entities, `CollisionGridSystem` rebuilding `SpatialGrid` each frame, disjoint
//! `CollisionLayer` bits + `query_aabb`, bullets routed through `engine::Pool`
//! with full deactivate-on-release, CPU `ParticleBurst` death explosions, and
//! target-gated best-effort audio. New: 8-way *aimed* fire (twin-stick), seeker
//! enemies, the GPU thruster, and the perf HUD.
//!
//! Controls: WASD/move · Arrow keys aim **and** fire (hold) · R restart · Esc quit.
//! Perf-debug keys: `G` toggles invulnerability and `B` spawns +50 enemies, so the
//! ~100-200 enemy perf target (and the HUD `frame_ms`/`steer` readouts) can be
//! reached on demand — single-life play tends to end before the screen fills. `B`
//! can push past the natural cap toward `MAX_ENEMIES` for stress-testing.

use engine::AxisBinding;
use engine::{
    App, Audio, AudioFacadeSystem, Camera, Collider, CollisionGridSystem, CollisionLayer, Color,
    DrawText, Entity, GamepadAxis, GamepadButton, GamepadState, InputMap, InputState, KeyCode,
    ParticleBurst, ParticleEmitter, ParticleSystem, Pool, ProfilerData, Seek, ShouldQuit,
    SpatialGrid, Sprite, SteeringSystem, SteeringVelocity, System, TextQueue, Timer, Transform,
    WindowConfig, World,
};
// Audio uses the cross-platform `Audio` facade (above) — one API for native + web, no `cfg`
// guards. `GpuParticleEmitter` is still native-only (`cfg(not(wasm32))`), so the thruster wiring
// stays target-gated; on wasm the thruster simply isn't created.
#[cfg(not(target_arch = "wasm32"))]
use engine::GpuParticleEmitter;
use glam::Vec2;
use rand::Rng;

// ─── Layout / tuning ───────────────────────────────────────────────────────────

const WINDOW_W: u32 = 820;
const WINDOW_H: u32 = 820;
const W: f32 = WINDOW_W as f32;
const H: f32 = WINDOW_H as f32;

const PLAYER_SPEED: f32 = 300.0;
const PLAYER_HALF: f32 = 16.0;

const BULLET_SPEED: f32 = 620.0;
const BULLET_HALF: f32 = 5.0;
const BULLET_POOL_CAP: usize = 96;
const FIRE_COOLDOWN: f32 = 0.14;
/// Bullets are released once they leave the arena by this margin.
const BULLET_MARGIN: f32 = 24.0;

const ENEMY_HALF: f32 = 14.0;
const ENEMY_SPEED_MIN: f32 = 90.0;
const ENEMY_SPEED_MAX: f32 = 140.0;
/// Natural-spawn steady-state cap — the locked ~100-200 perf/balance target that
/// `SpawnSystem` ramps toward in normal play.
const NATURAL_CAP: usize = 200;
/// Absolute hard cap (debug `B` burst headroom for stress-testing the O(N) steering
/// + collision + render paths beyond the natural target).
const MAX_ENEMIES: usize = 600;

/// Brief spawn-grace invulnerability after (re)start so an enemy spawning on the
/// player doesn't instantly end a single-life run.
const START_GRACE: f32 = 1.0;

// Collision layers — disjoint bits so `query_aabb(mask)` filters cleanly.
const LAYER_PLAYER: u32 = 1 << 0;
const LAYER_ENEMY: u32 = 1 << 1;
const LAYER_BULLET: u32 = 1 << 2;

// ─── Input actions ────────────────────────────────────────────────────────────

/// All gameplay actions understood by `PlayerSystem`.
///
/// The `InputMap<Action>` resource is pre-loaded with keyboard bindings (WASD
/// move, arrow-key aim, R restart, Esc quit) **and** optional gamepad bindings
/// (left stick move, right stick aim, South button restart) so a connected
/// controller drives the same actions without any if-gamepad branches in game
/// systems.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Action {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    AimLeft,
    AimRight,
    AimUp,
    AimDown,
    Restart,
    Quit,
    ToggleGod,
    SpawnBurst,
}

fn build_input_map() -> InputMap<Action> {
    let mut m = InputMap::new();

    // Movement — WASD + left stick
    m.bind(Action::MoveLeft, KeyCode::KeyA);
    m.bind_gamepad_button(Action::MoveLeft, GamepadButton::DPadLeft);
    m.bind_gamepad_axis(
        Action::MoveLeft,
        AxisBinding::negative(GamepadAxis::LeftStickX, 0.25),
    );

    m.bind(Action::MoveRight, KeyCode::KeyD);
    m.bind_gamepad_button(Action::MoveRight, GamepadButton::DPadRight);
    m.bind_gamepad_axis(
        Action::MoveRight,
        AxisBinding::positive(GamepadAxis::LeftStickX, 0.25),
    );

    m.bind(Action::MoveUp, KeyCode::KeyW);
    m.bind_gamepad_button(Action::MoveUp, GamepadButton::DPadUp);
    m.bind_gamepad_axis(
        Action::MoveUp,
        AxisBinding::negative(GamepadAxis::LeftStickY, 0.25),
    );

    m.bind(Action::MoveDown, KeyCode::KeyS);
    m.bind_gamepad_button(Action::MoveDown, GamepadButton::DPadDown);
    m.bind_gamepad_axis(
        Action::MoveDown,
        AxisBinding::positive(GamepadAxis::LeftStickY, 0.25),
    );

    // Aim / fire — Arrow keys + right stick
    m.bind(Action::AimLeft, KeyCode::ArrowLeft);
    m.bind_gamepad_axis(
        Action::AimLeft,
        AxisBinding::negative(GamepadAxis::RightStickX, 0.3),
    );

    m.bind(Action::AimRight, KeyCode::ArrowRight);
    m.bind_gamepad_axis(
        Action::AimRight,
        AxisBinding::positive(GamepadAxis::RightStickX, 0.3),
    );

    m.bind(Action::AimUp, KeyCode::ArrowUp);
    m.bind_gamepad_axis(
        Action::AimUp,
        AxisBinding::negative(GamepadAxis::RightStickY, 0.3),
    );

    m.bind(Action::AimDown, KeyCode::ArrowDown);
    m.bind_gamepad_axis(
        Action::AimDown,
        AxisBinding::positive(GamepadAxis::RightStickY, 0.3),
    );

    // Meta actions
    m.bind(Action::Restart, KeyCode::KeyR);
    m.bind_gamepad_button(Action::Restart, GamepadButton::Start);

    m.bind(Action::Quit, KeyCode::Escape);
    m.bind_gamepad_button(Action::Quit, GamepadButton::Select);

    m.bind(Action::ToggleGod, KeyCode::KeyG);
    m.bind(Action::SpawnBurst, KeyCode::KeyB);

    m
}

// ─── Marker / data components ──────────────────────────────────────────────────

#[allow(dead_code)]
struct Player;
#[allow(dead_code)]
struct Bullet;
#[allow(dead_code)]
struct Enemy;

/// Per-bullet velocity in pixels/sec. Enemies instead steer via `SteeringVelocity`.
struct Velocity(Vec2);

// ─── Session ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Playing,
    GameOver,
}

struct Survivor {
    player: Entity,
    /// Persistent GPU-particle thruster entity, position-synced to the player.
    thruster: Entity,
    status: Status,
    kills: u32,
    /// Seconds survived this run (the survival score).
    elapsed: f32,
    /// Remaining spawn-grace invulnerability (seconds).
    grace: f32,
    fire_timer: Timer,
    spawn_timer: Timer,
    /// Last movement direction this frame (drives the thruster); zero if still.
    move_dir: Vec2,
    /// Debug invulnerability (`G`) — lets the player survive so the ~100-200
    /// enemy perf target (and the HUD `frame_ms`) can actually be observed,
    /// which single-life play makes hard to reach.
    god: bool,
}

// ─── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine survivor".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.03, 0.04, 0.07, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    // Keyboard + gamepad action map (additive bindings).
    app.world.insert_resource(build_input_map());

    // Bullet object pool (same churn path the shooter exercised).
    app.world.insert_resource(Pool::new(BULLET_POOL_CAP));

    // Audio is best-effort: no device (headless / web before a gesture) → silent, never panics.
    // The `Audio` facade is cross-platform, so this wiring carries no `cfg` guards — sfx now play
    // on web too. Tones route through the "sfx" bus (see `play_tone`); set its group volume here.
    if let Some(mut audio) = Audio::new() {
        audio.set_bus_volume("sfx", 0.6);
        app.world.insert_resource(audio);
    }
    app.add_system(AudioFacadeSystem); // ticks native fades/ducks; no-op on web

    // Player ship at arena center.
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: Vec2::new(W * 0.5, H * 0.5),
            scale: Vec2::splat(PLAYER_HALF * 2.0),
            rotation: 0.0,
            z: 1.0,
        },
    );
    app.world
        .add_component(player, Sprite::colored(0.45, 0.9, 0.95));
    app.world.add_component(player, Player);
    app.world.add_component(
        player,
        Collider::Aabb {
            half_extents: Vec2::splat(PLAYER_HALF),
        },
    );
    app.world
        .add_component(player, CollisionLayer(LAYER_PLAYER));

    // Persistent GPU-particle thruster. The component compiles on both targets;
    // only the renderer is wasm-gated inside the engine, so on wasm this simply
    // renders nothing (no cfg needed here).
    let thruster = app.world.spawn();
    app.world.add_component(
        thruster,
        Transform {
            position: Vec2::new(W * 0.5, H * 0.5),
            scale: Vec2::ONE,
            rotation: 0.0,
            z: 0.5,
        },
    );
    // `GpuParticleEmitter` has private internal fields, so build via `default()`
    // + assignment (the gpu_particles demo pattern), not a struct literal.
    // Native-only — the type is wasm-absent, so on wasm the thruster entity exists
    // (Transform only) but carries no emitter and renders nothing.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut thruster_emitter = GpuParticleEmitter::default();
        thruster_emitter.spawn_rate = 140.0;
        thruster_emitter.lifetime = 0.5;
        thruster_emitter.velocity = Vec2::ZERO;
        thruster_emitter.velocity_spread = Vec2::splat(36.0);
        thruster_emitter.color_start = Color::rgba(0.5, 0.85, 1.0, 0.9);
        thruster_emitter.color_end = Color::rgba(0.2, 0.4, 0.9, 0.0);
        thruster_emitter.size = 5.0;
        thruster_emitter.emit = false; // toggled on only while moving
        app.world.add_component(thruster, thruster_emitter);
    }

    app.world.insert_resource(Survivor {
        player,
        thruster,
        status: Status::Playing,
        kills: 0,
        elapsed: 0.0,
        grace: START_GRACE,
        fire_timer: Timer::repeating(FIRE_COOLDOWN),
        spawn_timer: Timer::repeating(spawn_interval(0.0)),
        move_dir: Vec2::ZERO,
        god: false,
    });

    // System order: rebuild grid → input/move/fire → bullets → spawn → aim seek →
    // engine steering (enemies move) → thruster sync → collisions → particles →
    // HUD. `SeekSystem` must precede the engine `SteeringSystem`; `CollisionSystem`
    // reads the grid built first this frame.
    app.add_system(CollisionGridSystem::new(64.0));
    app.add_system(PlayerSystem);
    app.add_system(BulletSystem);
    app.add_system(SpawnSystem);
    app.add_system(SeekSystem);
    app.add_system(SteeringSystem::default());
    app.add_system(ThrusterSystem);
    app.add_system(CollisionSystem);
    app.add_system(ParticleSystem);
    app.add_system(HudSystem);

    app.run();
}

/// Spawn cadence shrinks as the run goes on (difficulty ramp), floored so it
/// can't outrun the `MAX_ENEMIES` cap.
fn spawn_interval(elapsed: f32) -> f32 {
    (0.55 - elapsed * 0.006).max(0.18)
}

// ─── Pool helpers ──────────────────────────────────────────────────────────────

/// Strip gameplay components so a released bullet is invisible to both the
/// renderer (`Transform`+`Sprite`) and the collision grid (`Collider`+
/// `CollisionLayer`). `Transform` is left to be overwritten on reuse.
fn deactivate_bullet(world: &mut World, e: Entity) {
    world.remove_component::<Sprite>(e);
    world.remove_component::<Collider>(e);
    world.remove_component::<CollisionLayer>(e);
    world.remove_component::<Velocity>(e);
    world.remove_component::<Bullet>(e);
}

/// Acquire a bullet from the pool and (re)initialise it at `muzzle`, travelling
/// along the aimed direction `dir` (unit vector).
fn fire_bullet(pool: &mut Pool, world: &mut World, muzzle: Vec2, dir: Vec2) {
    pool.acquire(world, |w, e| {
        w.add_component(
            e,
            Transform {
                position: muzzle,
                scale: Vec2::splat(BULLET_HALF * 2.0),
                rotation: 0.0,
                z: 0.8,
            },
        );
        w.add_component(e, Sprite::colored(1.0, 0.95, 0.5));
        w.add_component(e, Velocity(dir * BULLET_SPEED));
        w.add_component(e, Bullet);
        w.add_component(
            e,
            Collider::Aabb {
                half_extents: Vec2::splat(BULLET_HALF),
            },
        );
        w.add_component(e, CollisionLayer(LAYER_BULLET));
    });
}

// ─── Systems ─────────────────────────────────────────────────────────────────

struct PlayerSystem;
impl System for PlayerSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // Collect input from keyboard + gamepad (slot 0) via the action map.
        // All three resources are read-only; we snapshot what we need before
        // any mutable access.  The `resource()` method takes `&self` so two
        // distinct resource types can be borrowed at once through the same
        // `&World` borrow.
        let (move_axis, aim, restart, quit, toggle_god, burst) = {
            let r = world.resource::<InputState>();
            let g = world.resource::<GamepadState>();
            let m = world.resource::<InputMap<Action>>();
            match (r, g, m) {
                (Some(input), Some(gs), Some(map)) => {
                    let pad = gs.primary().unwrap_or(0);

                    let mut mv = Vec2::ZERO;
                    if map.is_pressed_with_gamepad(&Action::MoveLeft, input, gs, pad) {
                        mv.x -= 1.0;
                    }
                    if map.is_pressed_with_gamepad(&Action::MoveRight, input, gs, pad) {
                        mv.x += 1.0;
                    }
                    if map.is_pressed_with_gamepad(&Action::MoveUp, input, gs, pad) {
                        mv.y -= 1.0;
                    }
                    if map.is_pressed_with_gamepad(&Action::MoveDown, input, gs, pad) {
                        mv.y += 1.0;
                    }

                    // Aim stick: Arrow keys or right stick (hold = fire).
                    let mut aim = Vec2::ZERO;
                    if map.is_pressed_with_gamepad(&Action::AimLeft, input, gs, pad) {
                        aim.x -= 1.0;
                    }
                    if map.is_pressed_with_gamepad(&Action::AimRight, input, gs, pad) {
                        aim.x += 1.0;
                    }
                    if map.is_pressed_with_gamepad(&Action::AimUp, input, gs, pad) {
                        aim.y -= 1.0;
                    }
                    if map.is_pressed_with_gamepad(&Action::AimDown, input, gs, pad) {
                        aim.y += 1.0;
                    }

                    (
                        mv,
                        aim,
                        map.just_pressed_with_gamepad(&Action::Restart, input, gs, pad),
                        map.just_pressed_with_gamepad(&Action::Quit, input, gs, pad),
                        map.just_pressed(&Action::ToggleGod, input),
                        map.just_pressed(&Action::SpawnBurst, input),
                    )
                }
                _ => (Vec2::ZERO, Vec2::ZERO, false, false, false, false),
            }
        };

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.0 = true;
            }
        }
        if restart {
            restart_game(world);
            return;
        }
        // Debug perf affordances: G toggles invulnerability, B spawns a burst —
        // both exist so the ~100-200 enemy perf target can be reached for the
        // `frame_ms` observation (single-life play tends to end first).
        if toggle_god {
            if let Some(s) = world.resource_mut::<Survivor>() {
                s.god = !s.god;
            }
        }
        if burst {
            spawn_burst(world, 50);
        }

        let Some((player, status)) = world.resource::<Survivor>().map(|s| (s.player, s.status))
        else {
            return;
        };
        if status != Status::Playing {
            // Freeze the thruster when not playing.
            if let Some(s) = world.resource_mut::<Survivor>() {
                s.move_dir = Vec2::ZERO;
            }
            return;
        }

        // Movement, clamped to the arena.
        let dir = if move_axis.length_squared() > 0.0 {
            move_axis.normalize()
        } else {
            Vec2::ZERO
        };
        if let Some(t) = world.get_mut::<Transform>(player) {
            let mut p = t.position + dir * PLAYER_SPEED * dt;
            p.x = p.x.clamp(PLAYER_HALF, W - PLAYER_HALF);
            p.y = p.y.clamp(PLAYER_HALF, H - PLAYER_HALF);
            t.position = p;
        }

        // Advance survival clock, grace, fire cooldown; record move dir for the
        // thruster.
        if let Some(s) = world.resource_mut::<Survivor>() {
            s.elapsed += dt;
            if s.grace > 0.0 {
                s.grace = (s.grace - dt).max(0.0);
            }
            s.fire_timer.tick(dt);
            s.move_dir = dir;
        }

        // Fire along the aim direction while an arrow is held, gated by cooldown.
        let ready = world
            .resource::<Survivor>()
            .map(|s| s.fire_timer.just_finished())
            .unwrap_or(false);
        if aim.length_squared() > 0.0 && ready {
            let aim_dir = aim.normalize();
            if let Some(muzzle) = world
                .get::<Transform>(player)
                .map(|t| t.position + aim_dir * (PLAYER_HALF + BULLET_HALF * 2.0))
            {
                let mut pool = world.remove_resource::<Pool>().unwrap();
                fire_bullet(&mut pool, world, muzzle, aim_dir);
                world.insert_resource(pool);
                play_tone(world, 900.0, 0.04, 0.16);
            }
        }
    }

    fn name(&self) -> &'static str {
        "PlayerSystem"
    }
}

struct BulletSystem;
impl System for BulletSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // Advance bullets; collect those that left the arena. Collect entities
        // first so the query borrow is released before we touch the world again
        // (the maze "collect then get" pattern).
        let entities: Vec<Entity> = world.query::<Bullet>().map(|(e, _)| e).collect();
        let moves: Vec<(Entity, Vec2)> = entities
            .into_iter()
            .filter_map(|e| world.get::<Velocity>(e).map(|v| (e, v.0)))
            .collect();

        let mut offscreen = Vec::new();
        for (e, vel) in moves {
            if let Some(t) = world.get_mut::<Transform>(e) {
                t.position += vel * dt;
                let p = t.position;
                if p.x < -BULLET_MARGIN
                    || p.x > W + BULLET_MARGIN
                    || p.y < -BULLET_MARGIN
                    || p.y > H + BULLET_MARGIN
                {
                    offscreen.push(e);
                }
            }
        }
        if offscreen.is_empty() {
            return;
        }
        let mut pool = world.remove_resource::<Pool>().unwrap();
        for e in offscreen {
            deactivate_bullet(world, e);
            pool.release(e, world);
        }
        world.insert_resource(pool);
    }

    fn name(&self) -> &'static str {
        "BulletSystem"
    }
}

struct SpawnSystem;
impl System for SpawnSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // Only advance the spawn timer while actually playing (keeps it in step
        // with `elapsed`/`fire_timer`, which are also Playing-gated).
        let Some(status) = world.resource::<Survivor>().map(|s| s.status) else {
            return;
        };
        if status != Status::Playing {
            return;
        }
        let (fired, elapsed) = world
            .resource_mut::<Survivor>()
            .map(|s| {
                s.spawn_timer.tick(dt);
                (s.spawn_timer.just_finished(), s.elapsed)
            })
            .unwrap();
        if !fired {
            return;
        }

        // Keep the interval in step with the difficulty ramp.
        if let Some(s) = world.resource_mut::<Survivor>() {
            s.spawn_timer = Timer::repeating(spawn_interval(elapsed));
        }

        // Respect the natural steady-state cap (the perf/balance target). Spawn a
        // small batch that grows slowly with survival time. (Debug `B` can push
        // past this toward MAX_ENEMIES for stress-testing.)
        let alive = world.query::<Enemy>().count();
        if alive >= NATURAL_CAP {
            return;
        }
        let batch = (3 + (elapsed as usize / 12))
            .min(8)
            .min(NATURAL_CAP - alive);

        let player_pos = world
            .resource::<Survivor>()
            .and_then(|s| world.get::<Transform>(s.player).map(|t| t.position))
            .unwrap_or(Vec2::new(W * 0.5, H * 0.5));

        let mut rng = rand::thread_rng();
        for _ in 0..batch {
            let pos = edge_spawn(&mut rng);
            let speed = rng.gen_range(ENEMY_SPEED_MIN..=ENEMY_SPEED_MAX);
            spawn_enemy(world, pos, player_pos, speed);
        }
    }

    fn name(&self) -> &'static str {
        "SpawnSystem"
    }
}

/// Pick a random point just outside one of the four arena edges.
fn edge_spawn(rng: &mut impl Rng) -> Vec2 {
    let m = ENEMY_HALF * 2.0;
    match rng.gen_range(0..4) {
        0 => Vec2::new(rng.gen_range(0.0..W), -m),    // top
        1 => Vec2::new(rng.gen_range(0.0..W), H + m), // bottom
        2 => Vec2::new(-m, rng.gen_range(0.0..H)),    // left
        _ => Vec2::new(W + m, rng.gen_range(0.0..H)), // right
    }
}

fn spawn_enemy(world: &mut World, pos: Vec2, target: Vec2, speed: f32) {
    let e = world.spawn();
    world.add_component(
        e,
        Transform {
            position: pos,
            scale: Vec2::splat(ENEMY_HALF * 2.0),
            rotation: 0.0,
            z: 1.0,
        },
    );
    world.add_component(e, Sprite::colored(1.0, 0.42, 0.42));
    world.add_component(e, Enemy);
    // Steering: the engine `SteeringSystem` reads `Seek` + moves the Transform
    // via `SteeringVelocity`. `SeekSystem` retargets `Seek.target` each frame.
    world.add_component(
        e,
        Seek {
            target,
            max_speed: speed,
        },
    );
    world.add_component(
        e,
        SteeringVelocity {
            velocity: Vec2::ZERO,
            max_speed: speed,
        },
    );
    world.add_component(
        e,
        Collider::Aabb {
            half_extents: Vec2::splat(ENEMY_HALF),
        },
    );
    world.add_component(e, CollisionLayer(LAYER_ENEMY));
}

/// Debug helper (`B`): instantly spawn up to `count` seekers from the arena edges,
/// respecting `MAX_ENEMIES`, so the perf target can be reached on demand.
fn spawn_burst(world: &mut World, count: usize) {
    let alive = world.query::<Enemy>().count();
    let room = MAX_ENEMIES.saturating_sub(alive);
    if room == 0 {
        return;
    }
    let player_pos = world
        .resource::<Survivor>()
        .and_then(|s| world.get::<Transform>(s.player).map(|t| t.position))
        .unwrap_or(Vec2::new(W * 0.5, H * 0.5));
    let mut rng = rand::thread_rng();
    for _ in 0..count.min(room) {
        let pos = edge_spawn(&mut rng);
        let speed = rng.gen_range(ENEMY_SPEED_MIN..=ENEMY_SPEED_MAX);
        spawn_enemy(world, pos, player_pos, speed);
    }
}

/// Retarget every seeker at the player's current position, just before the
/// engine `SteeringSystem` consumes `Seek` to move them.
struct SeekSystem;
impl System for SeekSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(player_pos) = world
            .resource::<Survivor>()
            .and_then(|s| world.get::<Transform>(s.player).map(|t| t.position))
        else {
            return;
        };
        let enemies: Vec<Entity> = world.query::<Enemy>().map(|(e, _)| e).collect();
        for e in enemies {
            if let Some(seek) = world.get_mut::<Seek>(e) {
                seek.target = player_pos;
            }
        }
    }

    fn name(&self) -> &'static str {
        "SeekSystem"
    }
}

/// Sync the GPU thruster emitter to the player and emit only while moving.
struct ThrusterSystem;
impl System for ThrusterSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((player, thruster, move_dir)) = world
            .resource::<Survivor>()
            .map(|s| (s.player, s.thruster, s.move_dir))
        else {
            return;
        };
        let Some(ppos) = world.get::<Transform>(player).map(|t| t.position) else {
            return;
        };
        let moving = move_dir.length_squared() > 0.0;
        // Place the plume just behind the player and push particles backward.
        let back = if moving { -move_dir } else { Vec2::ZERO };
        if let Some(t) = world.get_mut::<Transform>(thruster) {
            t.position = ppos + back * PLAYER_HALF;
        }
        update_thruster_emit(world, thruster, moving, back);
    }

    fn name(&self) -> &'static str {
        "ThrusterSystem"
    }
}

/// Toggle the GPU thruster emitter (native-only; wasm has no `GpuParticleEmitter`).
#[cfg(not(target_arch = "wasm32"))]
fn update_thruster_emit(world: &mut World, thruster: Entity, moving: bool, back: Vec2) {
    if let Some(em) = world.get_mut::<GpuParticleEmitter>(thruster) {
        em.emit = moving;
        em.velocity = back * 90.0;
    }
}

/// wasm has no GPU particles; thruster emission is a no-op there.
#[cfg(target_arch = "wasm32")]
fn update_thruster_emit(_world: &mut World, _thruster: Entity, _moving: bool, _back: Vec2) {}

struct CollisionSystem;
impl System for CollisionSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((player, status, grace, god)) = world
            .resource::<Survivor>()
            .map(|s| (s.player, s.status, s.grace, s.god))
        else {
            return;
        };
        if status != Status::Playing {
            return;
        }
        // Effective invulnerability = spawn-grace OR debug god mode.
        let invuln = grace > 0.0 || god;

        // Snapshot bullet + player positions while only reading the world.
        let bullet_entities: Vec<Entity> = world.query::<Bullet>().map(|(e, _)| e).collect();
        let bullets: Vec<(Entity, Vec2)> = bullet_entities
            .into_iter()
            .filter_map(|e| world.get::<Transform>(e).map(|t| (e, t.position)))
            .collect();
        let player_pos = world.get::<Transform>(player).map(|t| t.position);

        // Resolve hits against the grid built this frame (immutable borrow).
        let mut bullet_kills: Vec<(Entity, Entity)> = Vec::new(); // (bullet, enemy)
        let mut player_hit: Option<Entity> = None;
        if let Some(grid) = world.resource::<SpatialGrid>() {
            let enemy_mask = CollisionLayer(LAYER_ENEMY);
            let mut claimed: std::collections::HashSet<Entity> = std::collections::HashSet::new();
            let mut spent: std::collections::HashSet<Entity> = std::collections::HashSet::new();
            for (bullet, pos) in &bullets {
                // A bullet kills at most one enemy and is released once. Without
                // `spent`, a single bullet overlapping two adjacent enemies in
                // one frame would be pushed (and later released) twice.
                if spent.contains(bullet) {
                    continue;
                }
                let min = *pos - Vec2::splat(BULLET_HALF * 2.0);
                let max = *pos + Vec2::splat(BULLET_HALF * 2.0);
                for enemy in grid.query_aabb(min, max, enemy_mask) {
                    if claimed.insert(enemy) {
                        bullet_kills.push((*bullet, enemy));
                        spent.insert(*bullet);
                        break; // one bullet, one enemy
                    }
                }
            }
            if !invuln {
                if let Some(pp) = player_pos {
                    let min = pp - Vec2::splat(PLAYER_HALF);
                    let max = pp + Vec2::splat(PLAYER_HALF);
                    player_hit = grid
                        .query_aabb(min, max, enemy_mask)
                        .into_iter()
                        .find(|e| !claimed.contains(e));
                }
            }
        }

        // Apply bullet→enemy kills: explode + despawn enemy, release bullet, score.
        if !bullet_kills.is_empty() {
            let mut pool = world.remove_resource::<Pool>().unwrap();
            let mut score_gain = 0u32;
            for (bullet, enemy) in bullet_kills {
                let epos = world.get::<Transform>(enemy).map(|t| t.position);
                if let Some(p) = epos {
                    spawn_explosion(world, p, [1.0, 0.7, 0.3, 1.0]);
                }
                if world.is_alive(enemy) {
                    world.despawn(enemy);
                }
                if world.is_alive(bullet) {
                    deactivate_bullet(world, bullet);
                    pool.release(bullet, world);
                }
                score_gain += 1;
            }
            world.insert_resource(pool);
            if let Some(s) = world.resource_mut::<Survivor>() {
                s.kills += score_gain;
            }
            play_tone(world, 150.0, 0.14, 0.28);
        }

        // Apply enemy→player contact: single life — explode and end the run.
        if let Some(enemy) = player_hit {
            let epos = world.get::<Transform>(enemy).map(|t| t.position);
            if let Some(p) = epos {
                spawn_explosion(world, p, [1.0, 0.4, 0.4, 1.0]);
            }
            if let Some(pp) = player_pos {
                spawn_explosion(world, pp, [0.5, 0.9, 1.0, 1.0]);
            }
            if world.is_alive(enemy) {
                world.despawn(enemy);
            }
            if let Some(s) = world.resource_mut::<Survivor>() {
                s.status = Status::GameOver;
            }
            play_tone(world, 110.0, 0.3, 0.35);
        }
    }

    fn name(&self) -> &'static str {
        "CollisionSystem"
    }
}

/// One-shot explosion: a dedicated entity carrying a CPU burst emitter that the
/// engine's `ParticleSystem` drains and despawns next tick.
fn spawn_explosion(world: &mut World, pos: Vec2, color: impl Into<Color>) {
    let e = world.spawn();
    world.add_component(
        e,
        Transform {
            position: pos,
            scale: Vec2::ONE,
            rotation: 0.0,
            z: 2.0,
        },
    );
    let mut emitter = ParticleEmitter::burst();
    emitter.color_start = color.into();
    emitter.lifetime = 0.4;
    emitter.size = Vec2::splat(4.0);
    emitter.velocity_spread = Vec2::splat(180.0);
    world.add_component(e, emitter);
    world.add_component(e, ParticleBurst { remaining: 16 });
}

struct HudSystem;
impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((status, kills, elapsed, god)) = world
            .resource::<Survivor>()
            .map(|s| (s.status, s.kills, s.elapsed, s.god))
        else {
            return;
        };
        let enemies = world.query::<Enemy>().count();
        // frame_ms is the vsync-capped frame delta (present mode is AutoVsync), so
        // it saturates near ~16.7ms at 60fps. `steer_ms` is the SteeringSystem's
        // own avg CPU cost from ProfilerData — the honest "does steering bite?"
        // number under many seekers.
        let (frame_ms, steer_ms) = world
            .resource::<ProfilerData>()
            .map(|p| {
                let steer = p
                    .systems
                    .iter()
                    .find(|s| s.name == "SteeringSystem")
                    .map(|s| s.avg_us / 1000.0)
                    .unwrap_or(0.0);
                (p.frame_ms, steer)
            })
            .unwrap_or((0.0, 0.0));

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            let god_tag = if god { "   [GOD]" } else { "" };
            tq.push(DrawText::new(
                format!(
                    "Time {elapsed:5.1}s   Kills {kills}   Enemies {enemies}   frame {frame_ms:4.1}ms   steer {steer_ms:4.2}ms{god_tag}"
                ),
                Vec2::new(10.0, 8.0),
                20.0,
                [235, 245, 255, 240],
            ));
            tq.push(DrawText::new(
                "Move: WASD   Aim/Fire: Arrows (hold)   R: restart   Esc: quit",
                Vec2::new(10.0, H - 44.0),
                16.0,
                [180, 200, 220, 200],
            ));
            tq.push(DrawText::new(
                "Perf debug — G: toggle invulnerability   B: spawn +50 enemies",
                Vec2::new(10.0, H - 24.0),
                16.0,
                [150, 175, 150, 190],
            ));
            if status == Status::GameOver {
                tq.push(DrawText::new(
                    format!("GAME OVER — survived {elapsed:.1}s, {kills} kills.  Press R."),
                    Vec2::new(W * 0.5 - 230.0, H * 0.5 - 16.0),
                    26.0,
                    [255, 150, 150, 255],
                ));
            }
        }
    }

    fn name(&self) -> &'static str {
        "HudSystem"
    }
}

// ─── Restart ───────────────────────────────────────────────────────────────────

fn restart_game(world: &mut World) {
    // Release every active bullet back to the pool (no leaks) and despawn all
    // enemies + lingering burst entities.
    let bullets: Vec<Entity> = world.query::<Bullet>().map(|(e, _)| e).collect();
    if !bullets.is_empty() {
        let mut pool = world.remove_resource::<Pool>().unwrap();
        for e in bullets {
            deactivate_bullet(world, e);
            pool.release(e, world);
        }
        world.insert_resource(pool);
    }
    let enemies: Vec<Entity> = world.query::<Enemy>().map(|(e, _)| e).collect();
    for e in enemies {
        world.despawn(e);
    }

    let player = world.resource::<Survivor>().map(|s| s.player);
    if let Some(player) = player {
        if let Some(t) = world.get_mut::<Transform>(player) {
            t.position = Vec2::new(W * 0.5, H * 0.5);
        }
    }
    if let Some(s) = world.resource_mut::<Survivor>() {
        s.status = Status::Playing;
        s.kills = 0;
        s.elapsed = 0.0;
        s.grace = START_GRACE;
        s.move_dir = Vec2::ZERO;
        s.fire_timer.reset();
        s.spawn_timer = Timer::repeating(spawn_interval(0.0));
    }

    // Pool sanity: every pooled bullet was returned, never leaked.
    if let Some(pool) = world.resource::<Pool>() {
        debug_assert!(pool.available_count() <= pool.capacity());
    }
}

// ─── Audio (best-effort) ───────────────────────────────────────────────────────

/// Best-effort sfx via the cross-platform [`Audio`] facade — one path for native + web, no `cfg`
/// split. Silent if no audio device / no resource. Tones ride the "sfx" mixer bus.
fn play_tone(world: &mut World, freq: f32, dur: f32, vol: f32) {
    if let Some(audio) = world.resource_mut::<Audio>() {
        audio.play_tone_on_bus(freq, dur, vol, "sfx");
    }
}
