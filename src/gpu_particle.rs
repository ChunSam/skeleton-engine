use glam::Vec2;
use rand::Rng;

use crate::color::Color;
use crate::components::Transform;
use crate::ecs::{Entity, World};
use crate::renderer::gpu_particle::GpuParticle;

/// Particle emitter component updated via a GPU compute shader.
///
/// Native only (use CPU `ParticleEmitter` for WASM).
///
/// # Example
/// ```rust,no_run
/// # use engine::{App, GpuParticleEmitter, Transform};
/// # use glam::Vec2;
/// # let mut app = App::new();
/// # let world = &mut app.world;
/// # let entity = world.spawn();
/// world.add_component(entity, Transform { position: Vec2::ZERO, ..Default::default() });
/// let mut emitter = GpuParticleEmitter::default();
/// emitter.spawn_rate = 100.0;
/// emitter.lifetime = 2.0;
/// emitter.velocity = Vec2::new(0.0, 80.0);
/// emitter.velocity_spread = Vec2::new(30.0, 20.0);
/// emitter.color_start = engine::Color::rgb(1.0, 0.5, 0.0);
/// emitter.color_end = engine::Color::rgba(1.0, 0.0, 0.0, 0.0);
/// emitter.size = 6.0;
/// emitter.emit = true;
/// world.add_component(entity, emitter);
/// ```
pub struct GpuParticleEmitter {
    /// Particles emitted per second.
    pub spawn_rate: f32,
    /// Particle lifetime (seconds).
    pub lifetime: f32,
    /// Base velocity (pixels/second).
    pub velocity: Vec2,
    /// Random velocity spread (±per axis).
    pub velocity_spread: Vec2,
    /// Start color (RGBA).
    pub color_start: Color,
    /// End color (RGBA).
    pub color_end: Color,
    /// Particle size (pixels).
    pub size: f32,
    /// When false, emission is paused.
    pub emit: bool,
    /// Internal emission timer.
    pub(crate) timer: f32,
    /// Next ring-buffer slot to emit into.
    pub(crate) next_slot: u32,
}

impl Default for GpuParticleEmitter {
    fn default() -> Self {
        Self {
            spawn_rate: 50.0,
            lifetime: 1.5,
            velocity: Vec2::new(0.0, 60.0),
            velocity_spread: Vec2::new(20.0, 10.0),
            color_start: Color::rgb(1.0, 0.8, 0.2),
            color_end: Color::rgba(1.0, 0.2, 0.0, 0.0),
            size: 5.0,
            emit: true,
            timer: 0.0,
            next_slot: 0,
        }
    }
}

/// Processes GPU particle emitters and collects new particle data.
///
/// Used alongside `GpuParticleRenderer::upload_particles` in the `App` render loop.
pub(crate) fn collect_new_particles(
    world: &mut World,
    capacity: u32,
    dt: f32,
) -> Vec<(u32, GpuParticle)> {
    let mut rng = rand::thread_rng();
    let mut result: Vec<(u32, GpuParticle)> = Vec::new();

    let emitter_entities: Vec<Entity> = world
        .query::<GpuParticleEmitter>()
        .map(|(e, _)| e)
        .collect();

    for entity in emitter_entities {
        let pos = world
            .get::<Transform>(entity)
            .map(|t| t.position)
            .unwrap_or(Vec2::ZERO);

        let emitter = match world.get_mut::<GpuParticleEmitter>(entity) {
            Some(e) => e,
            None => continue,
        };

        if !emitter.emit {
            continue;
        }

        emitter.timer += dt;
        let interval = 1.0 / emitter.spawn_rate.max(0.001);

        while emitter.timer >= interval {
            emitter.timer -= interval;

            let vx = emitter.velocity.x
                + rng.gen_range(-emitter.velocity_spread.x..=emitter.velocity_spread.x);
            let vy = emitter.velocity.y
                + rng.gen_range(-emitter.velocity_spread.y..=emitter.velocity_spread.y);

            let particle = GpuParticle {
                pos: [pos.x, pos.y],
                vel: [vx, vy],
                life: emitter.lifetime,
                max_life: emitter.lifetime,
                size: emitter.size,
                _pad: 0.0,
                color_start: emitter.color_start.to_array(),
                color_end: emitter.color_end.to_array(),
            };

            let slot = emitter.next_slot % capacity;
            emitter.next_slot = emitter.next_slot.wrapping_add(1);
            result.push((slot, particle));
        }
    }

    result
}
