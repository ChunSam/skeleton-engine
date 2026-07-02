//! `RadioGroup` — mutually exclusive options, one entity per group.
//!
//! Attach a [`RadioGroup`] next to a [`UiNode`]; the node's rect divides into option rows (ring +
//! label, the selected row's ring filled by a dot). Click a row to select it —
//! [`UiEvent::RadioChanged`] fires only when the selection actually changed. The group is one
//! focus stop: **Tab** focuses it, **←/→** steps the selection without the pointer.
//!
//! This demo wires two groups (difficulty + music) to a live HUD readout and a change counter.
//!
//! - **Click** an option row — select it
//! - **Tab** / **Shift+Tab** — move focus between the two groups
//! - **←/→** — step the focused group's selection
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/ui_radio.png cargo run --example ui_radio`): captures the two
//! groups with non-default selections. `HEADLESS_FRAMES=N` overrides the default 10 frames.
use engine::{
    App, Color, DrawText, Entity, Events, InputState, KeyCode, RadioGroup, ShouldQuit, System,
    TextQueue, UiEvent, UiNode, UiSystem, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 720;
const WIN_H: u32 = 420;

struct Demo {
    difficulty: Entity,
    music: Entity,
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

        // Count selection changes from the event bus (fires only on an actual change).
        if let Some(events) = world.resource::<Events<UiEvent>>() {
            self.changes += events
                .read()
                .iter()
                .filter(|e| matches!(e, UiEvent::RadioChanged(_, _)))
                .count();
        }

        let picked = |e: Entity, world: &World| {
            world
                .get::<RadioGroup>(e)
                .and_then(|rg| rg.selected_item().map(str::to_owned))
                .unwrap_or_default()
        };
        let difficulty = picked(self.difficulty, world);
        let music = picked(self.music, world);

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "RadioGroup — mutually exclusive options (click / Tab + arrows)",
                Vec2::new(28.0, 24.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                "Difficulty",
                Vec2::new(60.0, 80.0),
                16.0,
                Color::rgb(0.78, 0.82, 0.9),
            ));
            tq.push(DrawText::new(
                "Music",
                Vec2::new(400.0, 80.0),
                16.0,
                Color::rgb(0.78, 0.82, 0.9),
            ));
            tq.push(DrawText::new(
                format!("Picked: {difficulty} / {music}   changes: {}", self.changes),
                Vec2::new(60.0, 300.0),
                16.0,
                Color::rgb(0.75, 0.9, 0.95),
            ));
            tq.push(DrawText::new(
                "Tab: focus group   \u{2190}/\u{2192}: step selection   Esc: quit",
                Vec2::new(28.0, WIN_H as f32 - 28.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "ui_radio_demo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "ui_radio — radio groups".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.register_event::<UiEvent>();

    // Difficulty: 4 default-styled options, 30 px rows (120-tall node / 4).
    let difficulty = app.world.spawn();
    app.world
        .add_component(difficulty, UiNode::new(60.0, 100.0, 240.0, 120.0));
    app.world.add_component(
        difficulty,
        RadioGroup::new(["Easy", "Normal", "Hard", "Nightmare"]).with_selected(1),
    );

    // Music: custom colors + bigger circles, fixed 34 px rows.
    let music = app.world.spawn();
    app.world
        .add_component(music, UiNode::new(400.0, 100.0, 240.0, 102.0));
    app.world.add_component(
        music,
        RadioGroup::new(["Off", "Chiptune", "Orchestral"])
            .with_selected(2)
            .with_colors(Color::rgb(0.55, 0.75, 0.6), Color::rgb(0.95, 0.78, 0.30))
            .with_circle_size(22.0)
            .with_item_height(34.0),
    );

    app.add_system(UiSystem::new());
    app.add_system(Demo {
        difficulty,
        music,
        changes: 0,
    });

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
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
