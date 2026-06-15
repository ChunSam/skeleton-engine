//! Data-driven particles demo.
//!
//! The particle emitters are defined entirely in `assets/particles.ron` — NOT in code.
//! `App::load_particle_configs` loads them into a `ParticleConfigRegistry` (hot-reloaded via
//! the asset watcher). Press `1`/`2`/`3` to switch emitters by name (fire / fountain / smoke).
//! **Edit `particles.ron`** (a `spawn_rate`, a `color_start`, a `velocity`) while it runs and the
//! effect updates live — a re-sync system swaps the `ParticleEmitter` when the loaded config changes.
//!
//! Engine convention: camera position `(0,0)` = viewport TOP-LEFT (Y down); placed at positive coords.

use engine::{
    App, Camera, DrawText, Entity, InputState, KeyCode, ParticleConfigRegistry, ParticleEmitter,
    ParticleSystem, System, TextQueue, Transform, WindowConfig, World,
};
use glam::Vec2;

const CONFIGS: &str = "examples/games/data_particles/assets/particles.ron";
const WIN_W: u32 = 520;
const WIN_H: u32 = 460;

struct DemoState {
    emitter_entity: Entity,
    names: Vec<String>,
    current: usize,
    sig: u64,
}

/// Cheap signature over an emitter's config so we can detect a hot-reload.
fn emitter_sig(e: &ParticleEmitter) -> u64 {
    let c = e.color_start;
    [
        e.spawn_rate * 10.0,
        e.lifetime * 100.0,
        e.size.x,
        e.velocity_spread.x,
        (c.r + c.g + c.b + c.a) * 100.0,
    ]
    .iter()
    .fold(0u64, |acc, v| acc.wrapping_mul(131).wrapping_add(*v as u64))
}

fn fx_names(world: &World) -> Vec<String> {
    world
        .resource::<ParticleConfigRegistry>()
        .and_then(|r| r.get("fx"))
        .map(|s| s.names().map(|n| n.to_string()).collect())
        .unwrap_or_default()
}

fn fx_emitter(world: &World, name: &str) -> Option<ParticleEmitter> {
    world
        .resource::<ParticleConfigRegistry>()?
        .get("fx")?
        .emitter(name)
}

struct DataParticleSystem;

impl System for DataParticleSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (k1, k2, k3) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.just_pressed(KeyCode::Digit1),
                    i.just_pressed(KeyCode::Digit2),
                    i.just_pressed(KeyCode::Digit3),
                )
            })
            .unwrap_or((false, false, false));

        let (entity, current, sig) = match world.resource::<DemoState>() {
            Some(s) => (s.emitter_entity, s.current, s.sig),
            None => return,
        };
        let names = fx_names(world);
        if names.is_empty() {
            return;
        }

        // Target emitter index: a key press, else keep the current one.
        let mut idx = current.min(names.len() - 1);
        if k1 {
            idx = 0;
        } else if k2 && names.len() > 1 {
            idx = 1;
        } else if k3 && names.len() > 2 {
            idx = 2;
        }
        let switched = idx != current;

        // Detect a hot-reload of the currently-selected emitter (config values changed).
        let live = fx_emitter(world, &names[idx]);
        let new_sig = live.as_ref().map(emitter_sig).unwrap_or(0);
        let reloaded = !switched && new_sig != sig;

        // Replace the ParticleEmitter ONLY on a switch or hot-reload (not every frame —
        // that would reset the emit timer and never spawn particles).
        if switched || reloaded {
            if let Some(em) = live {
                if let Some(slot) = world.get_mut::<ParticleEmitter>(entity) {
                    *slot = em;
                }
                if let Some(s) = world.resource_mut::<DemoState>() {
                    s.current = idx;
                    s.names = names.clone();
                    s.sig = new_sig;
                }
            }
        }

        // HUD.
        let cur = names.get(idx).cloned().unwrap_or_default();
        let (rate, life) = world
            .get::<ParticleEmitter>(entity)
            .map(|e| (e.spawn_rate, e.lifetime))
            .unwrap_or((0.0, 0.0));
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Data-driven particles (emitters from RON)",
                Vec2::new(12.0, 10.0),
                18.0,
                [255, 230, 120, 255],
            ));
            tq.push(DrawText::new(
                format!("emitter: {cur}   spawn_rate: {rate}   lifetime: {life}"),
                Vec2::new(12.0, 36.0),
                15.0,
                [180, 255, 180, 255],
            ));
            tq.push(DrawText::new(
                "1: fire  2: fountain  3: smoke    edit assets/particles.ron to hot-reload",
                Vec2::new(12.0, WIN_H as f32 - 26.0),
                13.0,
                [180, 200, 230, 230],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "DataParticleSystem"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Data-Driven Particles".to_string(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.04, 0.04, 0.06, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    app.load_particle_configs("fx", CONFIGS);

    let names = fx_names(&app.world);
    let first = names.first().cloned().unwrap_or_default();
    let emitter = fx_emitter(&app.world, &first).expect("particles.ron failed to load");
    let sig = emitter_sig(&emitter);

    // Emitter entity sits low-centre so particles rise into view.
    let entity = app.world.spawn();
    app.world.add_component(
        entity,
        Transform {
            position: Vec2::new(WIN_W as f32 * 0.5, WIN_H as f32 * 0.66),
            ..Default::default()
        },
    );
    app.world.add_component(entity, emitter);

    app.world.insert_resource(DemoState {
        emitter_entity: entity,
        names,
        current: 0,
        sig,
    });

    app.add_system(ParticleSystem);
    app.add_system(DataParticleSystem);
    app.run();
}
