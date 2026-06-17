//! GPU compute-shader particle demo (Phase 56a).
//!
//! `cargo run --example gpu_particles`
//!
//! Spawns a `GpuParticleEmitter` at the mouse-click position. Each emitter rises, then
//! arcs back down under `gravity`, and scatters its spawn over a `Circle` emit shape —
//! the GPU mirror of the CPU `ParticleEmitter` gravity/emit-shape support.
//! - Space: toggle emission for the current emitters
//! - R: remove all emitters
use engine::{
    App, Color, DrawText, EmitShape, GpuParticleEmitter, InputState, KeyCode, MouseButton, System,
    TextQueue, Transform, WindowConfig, World,
};
use glam::Vec2;

struct GpuParticleDemo {
    emitter_count: usize,
}

impl System for GpuParticleDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (mouse_pos, left_just_pressed, space_just_pressed, r_just_pressed) = {
            let Some(input) = world.resource::<InputState>() else {
                return;
            };
            (
                input.cursor(),
                input.mouse_just_pressed(MouseButton::Left),
                input.just_pressed(KeyCode::Space),
                input.just_pressed(KeyCode::KeyR),
            )
        };

        // Mouse click → spawn a new emitter
        if left_just_pressed {
            let e = world.spawn();
            world.add_component(
                e,
                Transform {
                    position: mouse_pos,
                    scale: Vec2::ONE,
                    ..Default::default()
                },
            );
            let mut emitter = GpuParticleEmitter::default();
            emitter.spawn_rate = 120.0;
            emitter.lifetime = 2.0;
            emitter.velocity = Vec2::new(0.0, -80.0);
            emitter.velocity_spread = Vec2::new(40.0, 30.0);
            emitter.color_start = Color::rgba(1.0, 0.7, 0.1, 1.0);
            emitter.color_end = Color::rgba(1.0, 0.1, 0.0, 0.0);
            emitter.size = 6.0;
            // Sparks rise, then fall back under gravity (Y is down), spawning over a small disc.
            emitter.gravity = Vec2::new(0.0, 140.0);
            emitter.emit_shape = EmitShape::Circle { radius: 14.0 };
            emitter.emit = true;
            world.add_component(e, emitter);
            self.emitter_count += 1;
        }

        // Space: toggle all emitters
        if space_just_pressed {
            let entities: Vec<_> = world
                .query::<GpuParticleEmitter>()
                .map(|(e, _)| e)
                .collect();
            for e in entities {
                if let Some(em) = world.get_mut::<GpuParticleEmitter>(e) {
                    em.emit = !em.emit;
                }
            }
        }

        // R: remove all emitters
        if r_just_pressed {
            let entities: Vec<_> = world
                .query::<GpuParticleEmitter>()
                .map(|(e, _)| e)
                .collect();
            for e in entities {
                world.despawn(e);
            }
            self.emitter_count = 0;
        }

        // HUD
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "GPU Particle Demo — LClick: spawn  Space: toggle  R: clear",
                Vec2::new(10.0, 10.0),
                18.0,
                [220, 220, 220, 230],
            ));
            tq.push(DrawText::new(
                format!(
                    "Emitters: {}  (4096 particle slots total)",
                    self.emitter_count
                ),
                Vec2::new(10.0, 36.0),
                16.0,
                [160, 200, 255, 200],
            ));
        }
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "GPU Particle Demo".to_string(),
        width: 1280,
        height: 720,
        clear_color: [0.03, 0.03, 0.08, 1.0],
    });
    app.add_system(GpuParticleDemo { emitter_count: 0 });
    app.run();
}
