//! script_steering — demonstrates Arrive and Wander scripting bindings (Phase 38e).
//!
//! # What to observe
//! - **Cyan square (Arrive)**: follows your mouse cursor, decelerating smoothly as it
//!   closes in and stopping within 12 px. The target is written to the entity's
//!   Blackboard each frame by `TargetWriterSystem`; the Rhai script reads it and calls
//!   `arrive_at(...)`.
//! - **Yellow square (Wander)**: roams around autonomously, changing direction every
//!   1.5 seconds. The Rhai script calls `wander(90.0, 1.5)` from `on_update`.
//!
//! # Run
//! ```
//! cargo run --example script_steering_game
//! ```
//! Press **Escape** to quit.

use engine::steering::SteeringSystem;
use engine::{
    App, Blackboard, Camera, DrawText, Entity, InputState, KeyCode, ScriptRunner, ScriptingSystem,
    ShouldQuit, Sprite, System, TextQueue, Transform, Vec2, WindowConfig, World,
};

// ─── Marker components ────────────────────────────────────────────────────────

/// Marks the arrive-steering entity so TargetWriterSystem can find it.
#[derive(Clone)]
struct ArriveAgent;

/// Marks the wander-steering entity.
#[derive(Clone)]
struct WanderAgent;

// ─── TargetWriterSystem ───────────────────────────────────────────────────────

/// Converts the screen-space cursor position to world space and writes it into
/// the `ArriveAgent`'s `Blackboard` every frame so the Rhai script can read it
/// via `bb_get_float("target_x")` / `bb_get_float("target_y")`.
struct TargetWriterSystem;

impl System for TargetWriterSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // Resolve cursor → world position.
        let cursor = world
            .resource::<InputState>()
            .map(|i| i.cursor())
            .unwrap_or(Vec2::ZERO);
        let world_pos = world
            .resource::<Camera>()
            .map(|cam| cam.screen_to_world(cursor))
            .unwrap_or(cursor);

        // Write into every ArriveAgent's Blackboard.
        let agents: Vec<Entity> = world.query::<ArriveAgent>().map(|(e, _)| e).collect();
        for entity in agents {
            if world.get::<Blackboard>(entity).is_none() {
                world.add_component(entity, Blackboard::new());
            }
            if let Some(bb) = world.get_mut::<Blackboard>(entity) {
                bb.set_float("target_x", world_pos.x);
                bb.set_float("target_y", world_pos.y);
            }
        }
    }
}

// ─── QuitSystem ───────────────────────────────────────────────────────────────

struct QuitSystem;

impl System for QuitSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let pressed = world
            .resource::<InputState>()
            .map(|i| i.just_pressed(KeyCode::Escape))
            .unwrap_or(false);
        if pressed {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
    }
}

// ─── LabelSystem ─────────────────────────────────────────────────────────────

/// Draws informational labels every frame.
struct LabelSystem;

impl System for LabelSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let arrive_pos = world
            .query::<ArriveAgent>()
            .find_map(|(e, _)| world.get::<Transform>(e).map(|t| t.position));
        let wander_pos = world
            .query::<WanderAgent>()
            .find_map(|(e, _)| world.get::<Transform>(e).map(|t| t.position));

        let white = [235u8, 235, 245, 255];
        let dim = [180u8, 180, 200, 220];
        let small = [160u8, 200, 160, 220];

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Arrive (cyan): move mouse to guide",
                Vec2::new(10.0, 10.0),
                18.0,
                white,
            ));
            tq.push(DrawText::new(
                "Wander (yellow): autonomous roaming",
                Vec2::new(10.0, 32.0),
                18.0,
                white,
            ));
            tq.push(DrawText::new(
                "Press Escape to quit",
                Vec2::new(10.0, 54.0),
                16.0,
                dim,
            ));

            // Live positions
            if let Some(p) = arrive_pos {
                tq.push(DrawText::new(
                    format!("Arrive pos: ({:.0}, {:.0})", p.x, p.y),
                    Vec2::new(10.0, 78.0),
                    14.0,
                    small,
                ));
            }
            if let Some(p) = wander_pos {
                tq.push(DrawText::new(
                    format!("Wander pos: ({:.0}, {:.0})", p.x, p.y),
                    Vec2::new(10.0, 96.0),
                    14.0,
                    small,
                ));
            }
        }
    }
}

// ─── main ─────────────────────────────────────────────────────────────────────

fn main() {
    env_logger::init();

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "script_steering — arrive & wander scripting demo".to_string(),
        width: 800,
        height: 600,
        clear_color: [0.05, 0.05, 0.08, 1.0],
    });

    // ── Load scripts ──────────────────────────────────────────────────────────
    // Paths are relative to the workspace root at runtime.
    let arrive_script = app.load_script("examples/games/script_steering/assets/arrive_agent.rhai");
    let wander_script = app.load_script("examples/games/script_steering/assets/wander_agent.rhai");

    // ── Arrive agent (cyan square) ────────────────────────────────────────────
    {
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: Vec2::new(200.0, 300.0),
                scale: Vec2::splat(32.0),
                ..Default::default()
            },
        );
        app.world.add_component(e, Sprite::colored(0.0, 0.85, 0.9));
        app.world.add_component(e, ArriveAgent);
        app.world.add_component(e, Blackboard::new());
        app.world.add_component(e, ScriptRunner::new(arrive_script));
    }

    // ── Wander agent (yellow square) ──────────────────────────────────────────
    {
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: Vec2::new(600.0, 300.0),
                scale: Vec2::splat(32.0),
                ..Default::default()
            },
        );
        app.world.add_component(e, Sprite::colored(0.9, 0.85, 0.0));
        app.world.add_component(e, WanderAgent);
        app.world.add_component(e, ScriptRunner::new(wander_script));
    }

    // ── Systems ───────────────────────────────────────────────────────────────
    // Order: write blackboard targets → run scripts → run steering physics
    app.add_system(QuitSystem);
    app.add_system(TargetWriterSystem);
    app.add_system(ScriptingSystem::new());
    app.add_system(SteeringSystem::default());
    app.add_system(LabelSystem);

    app.run();
}
