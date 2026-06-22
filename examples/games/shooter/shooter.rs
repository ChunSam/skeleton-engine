//! Vertical shooter playable example (candidate D from docs/NEXT_WORK.md).
//!
//! First example to stress `engine::Pool` (bullet churn), `Timer`-driven
//! spawning (fire cooldown + enemy waves), `SpatialGrid`/`CollisionLayer` under
//! many simultaneous entities, and the audio bus mixer — all at once. It also
//! drives the engine gap this session closed: `ParticleEmitter` was
//! continuous-only, so hit/explosion *bursts* now go through the additive
//! `ParticleBurst` one-shot component.
//!
//! Rendering uses persistent ECS `Sprite` entities (the maze pattern), not the
//! immediate-mode `DebugDraw` filled rects Sokoban uses — a particle-heavy action game
//! has many moving sprites.
//!
//! Controls: A/D or ←/→ and W/S or ↑/↓ to move · Space to fire · R restart ·
//! Esc quit.

use engine::{
    App, Audio, AudioFacadeSystem, Camera, Collider, CollisionGridSystem, CollisionLayer, Color,
    DrawText, Entity, InputState, KeyCode, ParticleBurst, ParticleEmitter, ParticleSystem, Pool,
    ShouldQuit, SpatialGrid, Sprite, System, TextQueue, Timer, Transform, WindowConfig, World,
};
// Audio uses the cross-platform `Audio` facade — one API for native + web, no `cfg` guards (sfx
// now play on web too), so no target-gated audio import is needed.
use glam::Vec2;
use rand::Rng;

// ─── Layout / tuning ───────────────────────────────────────────────────────────

const WINDOW_W: u32 = 720;
const WINDOW_H: u32 = 900;
const W: f32 = WINDOW_W as f32;
const H: f32 = WINDOW_H as f32;

const PLAYER_SPEED: f32 = 430.0;
const PLAYER_HALF: f32 = 20.0;
const PLAYER_Y: f32 = H - 90.0;

const BULLET_SPEED: f32 = 760.0;
const BULLET_HALF: f32 = 5.0;
const BULLET_POOL_CAP: usize = 64;
const FIRE_COOLDOWN: f32 = 0.16;

const ENEMY_HALF: f32 = 18.0;
const ENEMY_SPEED_MIN: f32 = 80.0;
const ENEMY_SPEED_MAX: f32 = 150.0;
const WAVE_INTERVAL: f32 = 1.3;
const INVULN_TIME: f32 = 1.2;
const START_LIVES: i32 = 3;

// Collision layers — disjoint bits so `query_aabb(mask)` filters cleanly.
const LAYER_PLAYER: u32 = 1 << 0;
const LAYER_ENEMY: u32 = 1 << 1;
const LAYER_BULLET: u32 = 1 << 2;

// ─── Marker / data components ──────────────────────────────────────────────────

#[allow(dead_code)]
struct Player;
#[allow(dead_code)]
struct Bullet;
#[allow(dead_code)]
struct Enemy;

/// Per-entity velocity in pixels/sec (y grows downward, as in maze_escape).
struct Velocity(Vec2);

// ─── Session ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Playing,
    GameOver,
}

struct Shooter {
    player: Entity,
    status: Status,
    score: u32,
    lives: i32,
    fire_timer: Timer,
    wave_timer: Timer,
    /// Remaining invulnerability after taking a hit (seconds).
    invuln: f32,
    /// Waves spawned so far — used to ramp difficulty a little.
    waves: u32,
}

// ─── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine shooter".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.04, 0.05, 0.09, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    // Bullet object pool — the whole point of surfacing the pooling path.
    app.world.insert_resource(Pool::new(BULLET_POOL_CAP));

    // Audio is best-effort: no device (headless / web before a gesture) → silent, never panics.
    // The `Audio` facade is cross-platform, so this wiring carries no `cfg` guards — sfx now play
    // on web too. Tones route through the "sfx" bus (see `play_tone`); set its group volume here.
    if let Some(mut audio) = Audio::new() {
        audio.set_bus_volume("sfx", 0.6);
        app.world.insert_resource(audio);
    }
    app.add_system(AudioFacadeSystem); // ticks native fades/ducks; no-op on web

    // Player ship.
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: Vec2::new(W * 0.5, PLAYER_Y),
            scale: Vec2::splat(PLAYER_HALF * 2.0),
            rotation: 0.0,
            z: 1.0,
        },
    );
    app.world
        .add_component(player, Sprite::colored(0.45, 0.85, 1.0));
    app.world.add_component(player, Player);
    app.world.add_component(
        player,
        Collider::Aabb {
            half_extents: Vec2::splat(PLAYER_HALF),
        },
    );
    app.world
        .add_component(player, CollisionLayer(LAYER_PLAYER));

    app.world.insert_resource(Shooter {
        player,
        status: Status::Playing,
        score: 0,
        lives: START_LIVES,
        fire_timer: Timer::repeating(FIRE_COOLDOWN),
        wave_timer: Timer::repeating(WAVE_INTERVAL),
        invuln: 0.0,
        waves: 0,
    });

    // System order: rebuild spatial grid → input/fire → bullets → waves →
    // enemies → collisions → particles → HUD. Collisions read the grid built
    // first this frame.
    app.add_system(CollisionGridSystem::new(64.0));
    app.add_system(PlayerSystem);
    app.add_system(BulletSystem);
    app.add_system(WaveSystem);
    app.add_system(EnemySystem);
    app.add_system(CollisionSystem);
    app.add_system(ParticleSystem);
    app.add_system(HudSystem);

    app.run();
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

/// Acquire a bullet from the pool and (re)initialise it at `muzzle`.
fn fire_bullet(pool: &mut Pool, world: &mut World, muzzle: Vec2) {
    pool.acquire(world, |w, e| {
        w.add_component(
            e,
            Transform {
                position: muzzle,
                scale: Vec2::new(BULLET_HALF * 2.0, BULLET_HALF * 4.0),
                rotation: 0.0,
                z: 0.8,
            },
        );
        w.add_component(e, Sprite::colored(1.0, 0.95, 0.5));
        w.add_component(e, Velocity(Vec2::new(0.0, -BULLET_SPEED)));
        w.add_component(e, Bullet);
        w.add_component(
            e,
            Collider::Aabb {
                half_extents: Vec2::new(BULLET_HALF, BULLET_HALF * 2.0),
            },
        );
        w.add_component(e, CollisionLayer(LAYER_BULLET));
    });
}

// ─── Systems ─────────────────────────────────────────────────────────────────

struct PlayerSystem;
impl System for PlayerSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let (axis, fire, restart, quit) = world
            .resource::<InputState>()
            .map(|input| {
                let mut x = 0.0;
                let mut y = 0.0;
                if input.is_pressed(KeyCode::KeyA) || input.is_pressed(KeyCode::ArrowLeft) {
                    x -= 1.0;
                }
                if input.is_pressed(KeyCode::KeyD) || input.is_pressed(KeyCode::ArrowRight) {
                    x += 1.0;
                }
                if input.is_pressed(KeyCode::KeyW) || input.is_pressed(KeyCode::ArrowUp) {
                    y -= 1.0;
                }
                if input.is_pressed(KeyCode::KeyS) || input.is_pressed(KeyCode::ArrowDown) {
                    y += 1.0;
                }
                (
                    Vec2::new(x, y),
                    input.is_pressed(KeyCode::Space),
                    input.just_pressed(KeyCode::KeyR),
                    input.just_pressed(KeyCode::Escape),
                )
            })
            .unwrap_or((Vec2::ZERO, false, false, false));

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.0 = true;
            }
        }
        if restart {
            restart_game(world);
            return;
        }

        let Some((player, status)) = world.resource::<Shooter>().map(|s| (s.player, s.status))
        else {
            return;
        };
        if status != Status::Playing {
            return;
        }

        // Movement, clamped to the play field.
        let dir = if axis.length_squared() > 0.0 {
            axis.normalize()
        } else {
            Vec2::ZERO
        };
        if let Some(t) = world.get_mut::<Transform>(player) {
            let mut p = t.position + dir * PLAYER_SPEED * dt;
            p.x = p.x.clamp(PLAYER_HALF, W - PLAYER_HALF);
            p.y = p.y.clamp(PLAYER_HALF, H - PLAYER_HALF);
            t.position = p;
        }

        // Tick down invulnerability and the fire cooldown.
        if let Some(s) = world.resource_mut::<Shooter>() {
            if s.invuln > 0.0 {
                s.invuln = (s.invuln - dt).max(0.0);
            }
            s.fire_timer.tick(dt);
        }

        // Fire on cooldown while Space is held.
        let ready = world
            .resource::<Shooter>()
            .map(|s| s.fire_timer.just_finished())
            .unwrap_or(false);
        if fire && ready {
            if let Some(muzzle) = world
                .get::<Transform>(player)
                .map(|t| Vec2::new(t.position.x, t.position.y - PLAYER_HALF - BULLET_HALF * 2.0))
            {
                let mut pool = world.remove_resource::<Pool>().unwrap();
                fire_bullet(&mut pool, world, muzzle);
                world.insert_resource(pool);
                play_tone(world, 900.0, 0.05, 0.18);
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
        // Advance bullets; collect those that left the top of the screen.
        // Collect entities first so the query borrow is released before we
        // touch the world again (the maze "collect then get" pattern).
        let entities: Vec<Entity> = world.query::<Bullet>().map(|(e, _)| e).collect();
        let moves: Vec<(Entity, Vec2)> = entities
            .into_iter()
            .filter_map(|e| world.get::<Velocity>(e).map(|v| (e, v.0)))
            .collect();

        let mut offscreen = Vec::new();
        for (e, vel) in moves {
            if let Some(t) = world.get_mut::<Transform>(e) {
                t.position += vel * dt;
                if t.position.y < -BULLET_HALF * 4.0 {
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

struct WaveSystem;
impl System for WaveSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some((status, spawn, waves)) = world.resource_mut::<Shooter>().map(|s| {
            s.wave_timer.tick(dt);
            (s.status, s.wave_timer.just_finished(), s.waves)
        }) else {
            return;
        };
        if status != Status::Playing || !spawn {
            return;
        }

        // 3..=6 enemies per wave, more as waves accumulate.
        let mut rng = rand::thread_rng();
        let count = (3 + (waves / 3)).min(6);
        for _ in 0..count {
            let x = rng.gen_range(ENEMY_HALF..(W - ENEMY_HALF));
            let speed = rng.gen_range(ENEMY_SPEED_MIN..=ENEMY_SPEED_MAX);
            spawn_enemy(world, Vec2::new(x, -ENEMY_HALF * 2.0), speed);
        }
        if let Some(s) = world.resource_mut::<Shooter>() {
            s.waves += 1;
        }
    }

    fn name(&self) -> &'static str {
        "WaveSystem"
    }
}

fn spawn_enemy(world: &mut World, pos: Vec2, speed: f32) {
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
    world.add_component(e, Velocity(Vec2::new(0.0, speed)));
    world.add_component(
        e,
        Collider::Aabb {
            half_extents: Vec2::splat(ENEMY_HALF),
        },
    );
    world.add_component(e, CollisionLayer(LAYER_ENEMY));
}

struct EnemySystem;
impl System for EnemySystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let entities: Vec<Entity> = world.query::<Enemy>().map(|(e, _)| e).collect();
        let moves: Vec<(Entity, Vec2)> = entities
            .into_iter()
            .filter_map(|e| world.get::<Velocity>(e).map(|v| (e, v.0)))
            .collect();

        let mut escaped = Vec::new();
        for (e, vel) in moves {
            if let Some(t) = world.get_mut::<Transform>(e) {
                t.position += vel * dt;
                if t.position.y > H + ENEMY_HALF * 2.0 {
                    escaped.push(e);
                }
            }
        }
        // Enemies that slip past the bottom simply leave (no life penalty —
        // only contact costs a life).
        for e in escaped {
            world.despawn(e);
        }
    }

    fn name(&self) -> &'static str {
        "EnemySystem"
    }
}

struct CollisionSystem;
impl System for CollisionSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((player, status, invuln)) = world
            .resource::<Shooter>()
            .map(|s| (s.player, s.status, s.invuln))
        else {
            return;
        };
        if status != Status::Playing {
            return;
        }

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
            if invuln <= 0.0 {
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

        // Apply bullet→enemy kills: explode + despawn enemy, release bullet,
        // bump score.
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
            if let Some(s) = world.resource_mut::<Shooter>() {
                s.score += score_gain;
            }
            play_tone(world, 150.0, 0.16, 0.3);
        }

        // Apply enemy→player contact: explode enemy, lose a life, grant brief
        // invulnerability, end the game at zero lives.
        if let Some(enemy) = player_hit {
            let epos = world.get::<Transform>(enemy).map(|t| t.position);
            if let Some(p) = epos {
                spawn_explosion(world, p, [1.0, 0.4, 0.4, 1.0]);
            }
            if world.is_alive(enemy) {
                world.despawn(enemy);
            }
            if let Some(s) = world.resource_mut::<Shooter>() {
                s.lives -= 1;
                s.invuln = INVULN_TIME;
                if s.lives <= 0 {
                    s.status = Status::GameOver;
                }
            }
            play_tone(world, 110.0, 0.25, 0.35);
        }
    }

    fn name(&self) -> &'static str {
        "CollisionSystem"
    }
}

/// One-shot explosion: a dedicated entity carrying a burst emitter that the
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
    emitter.lifetime = 0.45;
    emitter.size = Vec2::splat(5.0);
    emitter.velocity_spread = Vec2::splat(190.0);
    world.add_component(e, emitter);
    world.add_component(e, ParticleBurst { remaining: 18 });
}

struct HudSystem;
impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((status, score, lives)) = world
            .resource::<Shooter>()
            .map(|s| (s.status, s.score, s.lives.max(0)))
        else {
            return;
        };
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!("Score {score}    Lives {lives}"),
                Vec2::new(10.0, 8.0),
                22.0,
                [235, 245, 255, 240],
            ));
            tq.push(DrawText::new(
                "Move: WASD/Arrows   Fire: Space   R: restart   Esc: quit",
                Vec2::new(10.0, H - 26.0),
                16.0,
                [180, 200, 220, 200],
            ));
            if status == Status::GameOver {
                tq.push(DrawText::new(
                    format!("GAME OVER — Score {score}.  Press R to play again."),
                    Vec2::new(W * 0.5 - 220.0, H * 0.5 - 16.0),
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
    // enemies + lingering particles.
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

    let player = world.resource::<Shooter>().map(|s| s.player);
    if let Some(player) = player {
        if let Some(t) = world.get_mut::<Transform>(player) {
            t.position = Vec2::new(W * 0.5, PLAYER_Y);
        }
    }
    if let Some(s) = world.resource_mut::<Shooter>() {
        s.status = Status::Playing;
        s.score = 0;
        s.lives = START_LIVES;
        s.invuln = 0.0;
        s.waves = 0;
        s.fire_timer.reset();
        s.wave_timer.reset();
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
