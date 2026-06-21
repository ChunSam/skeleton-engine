use glam::Vec2;
use rand::Rng;

use crate::color::Color;
use crate::components::Transform;
use crate::ecs::{Entity, World};
use crate::particle::EmitShape;
use crate::renderer::gpu_particle::GpuParticle;

/// Optional resource overriding the GPU-particle ring-buffer capacity.
///
/// Insert before the first frame to size the GPU particle buffer for your game. When absent,
/// the engine falls back to [`GpuParticleConfig::default`] (4096). Native only — the buffer is
/// allocated once when the first [`GpuParticleEmitter`] appears, so set this during setup.
#[derive(Debug, Clone, Copy)]
pub struct GpuParticleConfig {
    /// Maximum number of GPU particles alive simultaneously.
    pub capacity: u32,
}

impl Default for GpuParticleConfig {
    fn default() -> Self {
        Self { capacity: 4096 }
    }
}

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
    /// Constant acceleration applied to each particle every frame (pixels/s²). `ZERO` = none.
    /// Mirrors [`crate::ParticleEmitter::gravity`]; integrated per-particle in the compute shader.
    pub gravity: Vec2,
    /// Shape the spawn position is scattered over (offset from the emitter). Default `Point`.
    /// Mirrors [`crate::ParticleEmitter::emit_shape`]; sampled on the CPU at emission time.
    pub emit_shape: EmitShape,
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
            gravity: Vec2::ZERO,
            emit_shape: EmitShape::Point,
            emit: true,
            timer: 0.0,
            next_slot: 0,
        }
    }
}

impl GpuParticleEmitter {
    /// Sets the constant per-particle acceleration (builder style). e.g. falling sparks /
    /// rising smoke.
    pub fn with_gravity(mut self, gravity: Vec2) -> Self {
        self.gravity = gravity;
        self
    }

    /// Sets the spawn scatter shape (builder style).
    pub fn with_emit_shape(mut self, shape: EmitShape) -> Self {
        self.emit_shape = shape;
        self
    }
}

/// Processes GPU particle emitters and collects new particle data.
///
/// `frame_cursor` is a shared ring-buffer write cursor that advances across **all**
/// emitters this frame, ensuring each emitter receives a disjoint slot range in the
/// shared 4096-slot particle buffer.  Without this, emitters that each start at
/// `next_slot = 0` would overwrite each other's particles.
///
/// Used alongside `GpuParticleRenderer::upload_particles` in the `App` render loop.
pub(crate) fn collect_new_particles(
    world: &mut World,
    capacity: u32,
    dt: f32,
    frame_cursor: &mut u32,
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

            // Scatter the spawn position over the emit shape (offset from the emitter).
            // `Point` (the default) yields `ZERO` → byte-identical to before.
            let spawn = pos + emitter.emit_shape.sample_offset(&mut rng);

            let particle = GpuParticle {
                pos: [spawn.x, spawn.y],
                vel: [vx, vy],
                life: emitter.lifetime,
                max_life: emitter.lifetime,
                size: emitter.size,
                _pad: 0.0,
                color_start: emitter.color_start.to_array(),
                color_end: emitter.color_end.to_array(),
                gravity: [emitter.gravity.x, emitter.gravity.y],
                _pad2: [0.0, 0.0],
            };

            // Use the shared frame cursor so every emitter gets disjoint slots.
            let slot = *frame_cursor % capacity;
            *frame_cursor = frame_cursor.wrapping_add(1);
            // Keep the per-emitter counter in sync (for display/debug purposes only).
            emitter.next_slot = emitter.next_slot.wrapping_add(1);
            result.push((slot, particle));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    /// Two emitters must receive disjoint slots when the shared frame cursor is used.
    #[test]
    fn disjoint_slots_across_emitters() {
        let capacity = 16u32;
        let mut cursor = 0u32;

        // Simulate emitter A emitting 4 particles.
        let slots_a: Vec<u32> = (0..4)
            .map(|_| {
                let s = cursor % capacity;
                cursor = cursor.wrapping_add(1);
                s
            })
            .collect();

        // Simulate emitter B emitting 4 particles.
        let slots_b: Vec<u32> = (0..4)
            .map(|_| {
                let s = cursor % capacity;
                cursor = cursor.wrapping_add(1);
                s
            })
            .collect();

        // No slot in A should appear in B.
        for s in &slots_a {
            assert!(
                !slots_b.contains(s),
                "slot {s} appears in both emitter A and emitter B (collision!)"
            );
        }

        // All 8 slots are unique.
        let mut all: Vec<u32> = slots_a.iter().chain(slots_b.iter()).copied().collect();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), 8, "expected 8 distinct slots, got {}", all.len());
    }
}
