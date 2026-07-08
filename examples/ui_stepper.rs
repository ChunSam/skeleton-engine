//! `Stepper` — a numeric spinner: a value in `min..=max` stepped by flanking `-`/`+` buttons.
//!
//! Attach a [`Stepper`] next to a [`UiNode`]; the node's rect is split into a `-` button, a centered
//! value label, and a `+` button. Click a button to move the value by `step` (clamped to the
//! bounds) — [`UiEvent::StepperChanged`] fires only when the value actually changed. The stepper is
//! one focus stop: **Tab** focuses it, **←/→** step the value without the pointer.
//!
//! This demo wires three steppers (volume / difficulty / a custom-styled zoom) to a live HUD
//! readout and a change counter.
//!
//! - **Click** `-` / `+` — step the value
//! - **Tab** / **Shift+Tab** — move focus between the steppers
//! - **←/→** — step the focused stepper
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/ui_stepper.png cargo run --example ui_stepper`): captures the three
//! steppers with non-default values. `HEADLESS_FRAMES=N` overrides the default 10 frames.
use engine::{
    App, Color, DrawText, Entity, Events, InputState, KeyCode, ShouldQuit, Stepper, System,
    TextQueue, UiEvent, UiNode, UiSystem, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 680;
const WIN_H: u32 = 400;

struct Demo {
    volume: Entity,
    difficulty: Entity,
    zoom: Entity,
    changes: usize,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if world
            .resource::<InputState>()
            .is_some_and(|i| i.just_pressed(KeyCode::Escape))
        {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // Count value changes from the event bus (fires only on an actual change).
        if let Some(events) = world.resource::<Events<UiEvent>>() {
            self.changes += events
                .read()
                .iter()
                .filter(|e| matches!(e, UiEvent::StepperChanged(_, _)))
                .count();
        }

        let label = |e: Entity, world: &World| {
            world
                .get::<Stepper>(e)
                .map(Stepper::label)
                .unwrap_or_default()
        };
        let volume = label(self.volume, world);
        let difficulty = label(self.difficulty, world);
        let zoom = label(self.zoom, world);

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Stepper — numeric spinner (click -/+ or Tab + arrows)",
                Vec2::new(28.0, 24.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            for (text, y) in [("Volume", 92.0), ("Difficulty", 152.0), ("Zoom", 212.0)] {
                tq.push(DrawText::new(
                    text,
                    Vec2::new(60.0, y + 12.0),
                    16.0,
                    Color::rgb(0.78, 0.82, 0.9),
                ));
            }
            tq.push(DrawText::new(
                format!(
                    "Volume: {volume}   Difficulty: {difficulty}   Zoom: {zoom}   changes: {}",
                    self.changes
                ),
                Vec2::new(60.0, 292.0),
                16.0,
                Color::rgb(0.75, 0.9, 0.95),
            ));
            tq.push(DrawText::new(
                "Tab: focus   \u{2190}/\u{2192}: step   click -/+   Esc: quit",
                Vec2::new(28.0, WIN_H as f32 - 26.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "ui_stepper_demo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "ui_stepper — numeric spinners".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.register_event::<UiEvent>();

    // Volume: 0..100 in steps of 5 (integer display).
    let volume = app.world.spawn();
    app.world
        .add_component(volume, UiNode::new(220.0, 90.0, 200.0, 40.0));
    app.world
        .add_component(volume, Stepper::new(0.0, 100.0, 50.0).with_step(5.0));

    // Difficulty: 1..5 in steps of 1.
    let difficulty = app.world.spawn();
    app.world
        .add_component(difficulty, UiNode::new(220.0, 150.0, 200.0, 40.0));
    app.world
        .add_component(difficulty, Stepper::new(1.0, 5.0, 3.0).with_step(1.0));

    // Zoom: 0.5..3.0 in steps of 0.25, two decimals, custom colors.
    let zoom = app.world.spawn();
    app.world
        .add_component(zoom, UiNode::new(220.0, 210.0, 200.0, 40.0));
    app.world.add_component(
        zoom,
        Stepper::new(0.5, 3.0, 1.0)
            .with_step(0.25)
            .with_decimals(2)
            .with_colors(
                Color::rgba(0.09, 0.13, 0.12, 1.0),
                Color::rgba(0.20, 0.34, 0.26, 1.0),
                Color::rgba(0.30, 0.52, 0.38, 1.0),
                Color::rgba_u8(210, 222, 212, 255),
            )
            .with_corner_radius(10.0),
    );

    app.add_system(UiSystem::new());
    app.add_system(Demo {
        volume,
        difficulty,
        zoom,
        changes: 0,
    });

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        // Set non-default values so the capture shows varied labels across the three steppers.
        for (e, v) in [(volume, 70.0), (difficulty, 5.0), (zoom, 2.25)] {
            if let Some(s) = app.world.get_mut::<Stepper>(e) {
                s.value = v;
            }
        }
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
