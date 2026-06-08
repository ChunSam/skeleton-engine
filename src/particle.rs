use glam::Vec2;
use rand::Rng;

use crate::color::Color;
use crate::components::{Sprite, Transform};
use crate::ecs::{Entity, System, World};

type ParticleUpdate = (Entity, f32, f32, Vec2, Color, Color);
type EmitterSnapshot = (
    Entity,
    Vec2,
    bool,
    f32,
    f32,
    Vec2,
    Vec2,
    Color,
    Color,
    Vec2,
    Option<String>,
);
// (entity, pos, lifetime, velocity_spread, color_start, color_end, size, texture)
type BurstSnapshot = (Entity, Vec2, f32, Vec2, Color, Color, Vec2, Option<String>);

// ─── Components ───────────────────────────────────────────────────────────────

/// Emitter component that spawns particles.
///
/// Attach to an entity together with `Transform`; `ParticleSystem` will create particles.
pub struct ParticleEmitter {
    /// Particles spawned per second.
    pub spawn_rate: f32,
    /// Particle lifetime in seconds.
    pub lifetime: f32,
    /// Base velocity (pixels/second).
    pub velocity: Vec2,
    /// Random range added to velocity (±per axis).
    pub velocity_spread: Vec2,
    /// Color at spawn (RGBA).
    pub color_start: Color,
    /// Color at death (RGBA) — interpolated over lifetime.
    pub color_end: Color,
    /// Particle size in pixels.
    pub size: Vec2,
    /// Texture path. None means a solid-color rectangle.
    pub texture: Option<String>,
    /// Set to false to stop emitting.
    pub emit: bool,
    /// Internal timer (no need to modify directly).
    pub(crate) timer: f32,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            spawn_rate: 20.0,
            lifetime: 1.0,
            velocity: Vec2::new(0.0, -50.0),
            velocity_spread: Vec2::new(20.0, 10.0),
            color_start: Color::WHITE,
            color_end: Color::rgba(1.0, 1.0, 1.0, 0.0),
            size: Vec2::splat(8.0),
            texture: None,
            emit: true,
            timer: 0.0,
        }
    }
}

impl ParticleEmitter {
    /// Emitter preset for one-shot bursts (explosions, hit effects).
    ///
    /// Disables continuous emission (`emit = false`, `spawn_rate = 0`) and fills
    /// in defaults suited for short-lived radial spread. Must be attached to the
    /// same entity as [`ParticleBurst`] — [`ParticleSystem`] will emit all
    /// `remaining` particles at once on the next tick, then despawn the entity.
    ///
    /// This is a purely additive API and does not affect continuous emitter behavior.
    pub fn for_burst() -> Self {
        Self {
            spawn_rate: 0.0,
            lifetime: 0.5,
            velocity: Vec2::ZERO,
            velocity_spread: Vec2::splat(160.0),
            color_start: Color::rgb(1.0, 0.85, 0.35),
            color_end: Color::rgba(1.0, 0.25, 0.1, 0.0),
            size: Vec2::splat(6.0),
            texture: None,
            emit: false,
            timer: 0.0,
        }
    }
}

/// Active particle component.
pub struct Particle {
    pub lifetime: f32,
    pub age: f32,
    pub velocity: Vec2,
    pub color_start: Color,
    pub color_end: Color,
}

/// One-shot particle burst marker.
///
/// Attach alongside [`ParticleEmitter`] on an entity; [`ParticleSystem`] will
/// emit all `remaining` particles radially at once on the next tick, then
/// **despawn the entity.** Use on a dedicated single-use entity for explosion or
/// hit effects.
///
/// Independent of continuous emission (`ParticleEmitter::emit`) — it is a
/// purely additive component that does not change existing emitter behavior.
/// Emission speed (pixels/second) uses the emitter's `velocity_spread` magnitude
/// as the radius.
pub struct ParticleBurst {
    /// Number of particles to emit in this burst.
    pub remaining: u32,
}

// ─── System ───────────────────────────────────────────────────────────────────

pub struct ParticleSystem;

impl System for ParticleSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // 1. Move and update color of existing particles; collect expired ones.
        let updates: Vec<ParticleUpdate> = world
            .query::<Particle>()
            .map(|(e, p)| (e, p.age, p.lifetime, p.velocity, p.color_start, p.color_end))
            .collect();

        let mut to_despawn = Vec::new();
        for (entity, age, lifetime, velocity, color_start, color_end) in updates {
            let new_age = age + dt;
            if new_age >= lifetime {
                to_despawn.push(entity);
                continue;
            }
            if let Some(tr) = world.get_mut::<Transform>(entity) {
                tr.position += velocity * dt;
            }
            let t = new_age / lifetime;
            let lerped = Color::rgba(
                color_start.r + (color_end.r - color_start.r) * t,
                color_start.g + (color_end.g - color_start.g) * t,
                color_start.b + (color_end.b - color_start.b) * t,
                color_start.a + (color_end.a - color_start.a) * t,
            );
            if let Some(sp) = world.get_mut::<Sprite>(entity) {
                sp.color = lerped;
            }
            if let Some(p) = world.get_mut::<Particle>(entity) {
                p.age = new_age;
            }
        }
        for e in to_despawn {
            world.despawn(e);
        }

        // 2. Emit new particles from emitters.
        let emitter_data: Vec<EmitterSnapshot> = world
            .query2::<Transform, ParticleEmitter>()
            .map(|(e, tr, em)| {
                (
                    e,
                    tr.position,
                    em.emit,
                    em.spawn_rate,
                    em.lifetime,
                    em.velocity,
                    em.velocity_spread,
                    em.color_start,
                    em.color_end,
                    em.size,
                    em.texture.clone(),
                )
            })
            .collect();

        let mut rng = rand::thread_rng();
        for (
            emitter_entity,
            pos,
            emit,
            spawn_rate,
            lifetime,
            velocity,
            spread,
            color_start,
            color_end,
            size,
            texture,
        ) in emitter_data
        {
            if !emit || spawn_rate <= 0.0 {
                continue;
            }
            let spawn_count = {
                let em = world.get_mut::<ParticleEmitter>(emitter_entity).unwrap();
                em.timer += dt;
                let interval = 1.0 / spawn_rate;
                // On a slow frame where timer exceeds interval multiple times, spawn that many.
                // (Previously only one was spawned per frame, making density framerate-dependent.)
                let mut count = 0u32;
                while em.timer >= interval {
                    em.timer -= interval;
                    count += 1;
                }
                // Runaway guard: at most 64 per frame (handles very large spawn_rate + long dt).
                count.min(64)
            };

            for _ in 0..spawn_count {
                let actual_velocity = Vec2::new(
                    velocity.x + rng.gen_range(-spread.x..=spread.x),
                    velocity.y + rng.gen_range(-spread.y..=spread.y),
                );

                spawn_particle(
                    world,
                    pos,
                    size,
                    &texture,
                    actual_velocity,
                    lifetime,
                    color_start,
                    color_end,
                );
            }
        }

        // 3. One-shot burst emission (ParticleEmitter + ParticleBurst). Independent of
        //    continuous emission; despawns the emitter entity after firing.
        let burst_emitters: Vec<BurstSnapshot> = world
            .query2::<Transform, ParticleEmitter>()
            .map(|(e, tr, em)| {
                (
                    e,
                    tr.position,
                    em.lifetime,
                    em.velocity_spread,
                    em.color_start,
                    em.color_end,
                    em.size,
                    em.texture.clone(),
                )
            })
            .collect();
        for (entity, pos, lifetime, spread, color_start, color_end, size, texture) in burst_emitters
        {
            let Some(remaining) = world.get::<ParticleBurst>(entity).map(|b| b.remaining) else {
                continue;
            };
            let radius = spread.max_element().max(1.0);
            for _ in 0..remaining {
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                let speed = rng.gen_range(0.2..=1.0) * radius;
                let vel = Vec2::new(angle.cos(), angle.sin()) * speed;
                spawn_particle(
                    world,
                    pos,
                    size,
                    &texture,
                    vel,
                    lifetime,
                    color_start,
                    color_end,
                );
            }
            world.despawn(entity);
        }
    }
}

/// Spawns a single particle entity (shared by continuous emission and burst).
#[allow(clippy::too_many_arguments)]
fn spawn_particle(
    world: &mut World,
    pos: Vec2,
    size: Vec2,
    texture: &Option<String>,
    velocity: Vec2,
    lifetime: f32,
    color_start: Color,
    color_end: Color,
) {
    let pe = world.spawn();
    world.add_component(
        pe,
        Transform {
            position: pos,
            scale: size,
            rotation: 0.0,
            z: 0.0,
        },
    );
    let sprite = match texture {
        Some(path) => Sprite::textured(path.as_str()),
        None => Sprite {
            texture: None,
            color: color_start,
            image_handle: None,
        },
    };
    world.add_component(pe, sprite);
    world.add_component(
        pe,
        Particle {
            lifetime,
            age: 0.0,
            velocity,
            color_start,
            color_end,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;

    #[test]
    fn burst_emits_count_then_retires_emitter() {
        let mut world = World::new();
        let emitter = world.spawn();
        world.add_component(emitter, Transform::default());
        world.add_component(emitter, ParticleEmitter::for_burst());
        world.add_component(emitter, ParticleBurst { remaining: 8 });

        ParticleSystem.run(&mut world, 0.016);

        // Exactly 8 particles are emitted in one tick.
        assert_eq!(world.query::<Particle>().count(), 8);
        // The emitter entity is despawned after the burst (one-shot).
        assert!(!world.is_alive(emitter));
    }

    #[test]
    fn continuous_emitter_unaffected_by_burst_path() {
        let mut world = World::new();
        let emitter = world.spawn();
        world.add_component(emitter, Transform::default());
        // Continuous emitter: emit=true, high spawn_rate. No ParticleBurst.
        world.add_component(
            emitter,
            ParticleEmitter {
                spawn_rate: 100.0,
                emit: true,
                ..ParticleEmitter::default()
            },
        );

        ParticleSystem.run(&mut world, 0.05);

        // Continuous emitter produces dt(0.05) / interval(1/100 = 0.01) = 5 particles
        // and survives (not despawned). (Previously only 1 per frame → under-emit.)
        assert_eq!(world.query::<Particle>().count(), 5);
        assert!(world.is_alive(emitter));
    }
}
