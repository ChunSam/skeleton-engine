//! GPU compute-shader particle demo.
//!
//! `cargo run --example gpu_particles`
//!
//! Spawns a `GpuParticleEmitter` at the mouse-click position, **built from a RON config**
//! (`gpu_particles.ron`) via `ParticleConfigSet::gpu_emitter` — the RON→GPU counterpart to the
//! CPU `emitter()` path. Each emitter rises, arcs back down under `gravity`, and scatters its
//! spawn over a `Circle` emit shape. Edit `gpu_particles.ron` while running to hot-reload.
//! - Space: toggle emission for the current emitters
//! - R: remove all emitters
//! - H: toggle HDR post-process (ACES tonemap) — confirms GPU particles render under HDR post
use engine::{
    App, DrawText, GpuParticleEmitter, InputState, KeyCode, MouseButton, ParticleConfigRegistry,
    PostProcessConfig, System, TextQueue, Tonemap, Transform, WindowConfig, World,
};
use glam::Vec2;

struct GpuParticleDemo {
    emitter_count: usize,
}

impl System for GpuParticleDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (mouse_pos, left_just_pressed, space_just_pressed, r_just_pressed, h_just_pressed) = {
            let Some(input) = world.resource::<InputState>() else {
                return;
            };
            (
                input.cursor(),
                input.mouse_just_pressed(MouseButton::Left),
                input.just_pressed(KeyCode::Space),
                input.just_pressed(KeyCode::KeyR),
                input.just_pressed(KeyCode::KeyH),
            )
        };

        // H: toggle HDR post-process (ACES tonemap). With HDR on, the scene renders into an
        // Rgba16Float intermediate — the particles must still appear, confirming they are
        // format-matched to the HDR target (previously they were skipped under HDR post).
        if h_just_pressed {
            let cfg = world.resource_mut::<PostProcessConfig>();
            if let Some(cfg) = cfg {
                let on = !cfg.hdr;
                cfg.enabled = on;
                cfg.hdr = on;
                cfg.tonemap = if on {
                    Tonemap::AcesFilmic
                } else {
                    Tonemap::None
                };
            }
        }

        // Mouse click → spawn a new emitter, built from the RON config via the GPU path
        // (gravity + Circle scatter come straight from gpu_particles.ron). Read the registry
        // first to release its borrow before spawning.
        if left_just_pressed {
            let emitter = world
                .resource::<ParticleConfigRegistry>()
                .and_then(|r| r.get("vfx"))
                .and_then(|set| set.gpu_emitter("spark"));
            if let Some(emitter) = emitter {
                let e = world.spawn();
                world.add_component(
                    e,
                    Transform {
                        position: mouse_pos,
                        scale: Vec2::ONE,
                        ..Default::default()
                    },
                );
                world.add_component(e, emitter);
                self.emitter_count += 1;
            }
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
        let hdr_on = world.resource::<PostProcessConfig>().is_some_and(|c| c.hdr);
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "GPU Particle Demo — LClick: spawn  Space: toggle  R: clear  H: HDR post",
                Vec2::new(10.0, 10.0),
                18.0,
                [220, 220, 220, 230],
            ));
            tq.push(DrawText::new(
                format!(
                    "Emitters: {}  (4096 particle slots total)   HDR post: {}",
                    self.emitter_count,
                    if hdr_on { "ON (ACES)" } else { "off" }
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
    // HDR post-process config (off by default; press H to toggle it on with ACES tonemap).
    app.world.insert_resource(PostProcessConfig::default());
    app.load_particle_configs("vfx", "examples/gpu_particles.ron");

    // Spawn one emitter at screen center from the RON config, so the demo shows the RON→GPU
    // path immediately (no click needed); more spawn on left-click.
    let mut count = 0;
    if let Some(emitter) = app
        .world
        .resource::<ParticleConfigRegistry>()
        .and_then(|r| r.get("vfx"))
        .and_then(|set| set.gpu_emitter("spark"))
    {
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: Vec2::new(640.0, 360.0),
                scale: Vec2::ONE,
                ..Default::default()
            },
        );
        app.world.add_component(e, emitter);
        count = 1;
    }

    app.add_system(GpuParticleDemo {
        emitter_count: count,
    });
    app.run();
}
