//! Coroutine sequencer demo.
//!
//! A single coroutine drives the whole scene:
//!
//!   1. Wait 1 s  — title screen
//!   2. Run       — spawn a cyan box
//!   3. RunFor 2s — slide the box across the screen (progress 0→1)
//!   4. Wait 0.5s — brief pause
//!   5. Run       — change box colour to orange
//!   6. Wait 0.5s — pause before loop
//!   7. Run       — restart the coroutine (loops the sequence)
//!
//! On-screen text always shows the current phase label, updated by the
//! coroutine closures themselves (via a `PhaseLabel` resource).
//!
//! Press Esc to quit.

use engine::{
    App, Coroutine, CoroutineRunner, CoroutineSystem, DrawText, Entity, InputState, KeyCode,
    ShouldQuit, Sprite, System, TextQueue, Transform, Vec2, WindowConfig, World,
};

const WIN_W: f32 = 800.0;
const WIN_H: f32 = 450.0;
const BOX_Y: f32 = WIN_H * 0.5 - 40.0;
const BOX_START_X: f32 = 80.0;
const BOX_END_X: f32 = WIN_W - 80.0;

// ── Shared resources ──────────────────────────────────────────────────────────

/// Tracks the label displayed in the HUD.
#[derive(Default)]
struct PhaseLabel(String);

/// Holds the spawned box entity so coroutine closures can mutate it.
#[derive(Default)]
struct BoxEntity(Option<Entity>);

// ── Quit system ───────────────────────────────────────────────────────────────

struct QuitSystem;

impl System for QuitSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let quit = world
            .resource::<InputState>()
            .map(|i| i.just_pressed(KeyCode::Escape))
            .unwrap_or(false);
        if quit {
            if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                sq.quit();
            }
        }
    }
}

// ── HUD system ────────────────────────────────────────────────────────────────

struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let phase = world
            .resource::<PhaseLabel>()
            .map(|p| p.0.clone())
            .unwrap_or_default();

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };

        tq.push(DrawText::new(
            "Coroutine demo — Esc to quit",
            Vec2::new(16.0, 14.0),
            18.0,
            [240, 240, 255, 220],
        ));

        tq.push(DrawText::new(
            format!("Phase: {}", phase),
            Vec2::new(16.0, 42.0),
            16.0,
            [200, 230, 200, 255],
        ));

        tq.push(DrawText::new(
            "Steps: wait 1s -> spawn -> slide 2s -> pause -> recolor -> loop",
            Vec2::new(16.0, WIN_H - 26.0),
            13.0,
            [170, 170, 190, 180],
        ));
    }
}

// ── Coroutine builder ─────────────────────────────────────────────────────────

/// Builds the scripted sequence coroutine.
fn build_sequence() -> Coroutine {
    Coroutine::new()
        // 1. Wait 1 s before doing anything.
        .run(|world| {
            if let Some(p) = world.resource_mut::<PhaseLabel>() {
                p.0 = "waiting 1s before spawn...".to_string();
            }
        })
        .wait(1.0)
        // 2. Spawn the box.
        .run(|world| {
            if let Some(p) = world.resource_mut::<PhaseLabel>() {
                p.0 = "spawning box".to_string();
            }
            // Despawn any existing box first.
            let existing = world.resource::<BoxEntity>().and_then(|b| b.0);
            if let Some(old) = existing {
                world.despawn(old);
            }
            let e = world.spawn();
            world.add_component(
                e,
                Transform {
                    position: Vec2::new(BOX_START_X, BOX_Y),
                    scale: Vec2::new(60.0, 60.0),
                    rotation: 0.0,
                    z: 0.5,
                },
            );
            world.add_component(e, Sprite::colored(0.2, 0.8, 0.9)); // cyan
            if let Some(be) = world.resource_mut::<BoxEntity>() {
                be.0 = Some(e);
            }
        })
        // 3. Slide across the screen over 2 s.
        .run(|world| {
            if let Some(p) = world.resource_mut::<PhaseLabel>() {
                p.0 = "sliding box across screen (2s)".to_string();
            }
        })
        .run_for(2.0, |world, t| {
            let entity = world.resource::<BoxEntity>().and_then(|b| b.0);
            if let Some(e) = entity {
                let x = BOX_START_X + (BOX_END_X - BOX_START_X) * t;
                if let Some(tr) = world.get_mut::<Transform>(e) {
                    tr.position.x = x;
                }
            }
        })
        // 4. Wait 0.5 s.
        .run(|world| {
            if let Some(p) = world.resource_mut::<PhaseLabel>() {
                p.0 = "pausing 0.5s...".to_string();
            }
        })
        .wait(0.5)
        // 5. Change colour to orange.
        .run(|world| {
            if let Some(p) = world.resource_mut::<PhaseLabel>() {
                p.0 = "recoloring box to orange".to_string();
            }
            let entity = world.resource::<BoxEntity>().and_then(|b| b.0);
            if let Some(e) = entity {
                if let Some(sp) = world.get_mut::<Sprite>(e) {
                    *sp = Sprite::colored(1.0, 0.5, 0.1);
                }
            }
        })
        // 6. Wait 0.5 s.
        .wait(0.5)
        // 7. Loop: re-start the sequence.
        .run(|world| {
            if let Some(p) = world.resource_mut::<PhaseLabel>() {
                p.0 = "restarting sequence...".to_string();
            }
            let next = build_sequence();
            if let Some(runner) = world.resource_mut::<CoroutineRunner>() {
                runner.start(next);
            }
        })
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();

    app.world.insert_resource(WindowConfig {
        title: "coroutine_demo — skeleton-engine".to_string(),
        width: WIN_W as u32,
        height: WIN_H as u32,
        clear_color: [0.07, 0.08, 0.12, 1.0],
    });

    // Insert shared resources.
    app.world.insert_resource(PhaseLabel::default());
    app.world.insert_resource(BoxEntity::default());
    app.world.insert_resource(CoroutineRunner::new());

    // Start the first iteration of the sequence.
    let seq = build_sequence();
    app.world
        .resource_mut::<CoroutineRunner>()
        .unwrap()
        .start(seq);

    // Register systems.
    app.add_system(QuitSystem);
    app.add_system(CoroutineSystem);
    app.add_system(HudSystem);

    app.run();
}
