//! `TabBar` — equal-width tab headers, one active; content switching is the game's job.
//!
//! Attach a [`TabBar`] next to a [`UiNode`]; the node's rect divides into equal-width headers.
//! Click a header (or focus the bar and press **←/→**) to change the active tab —
//! [`UiEvent::TabChanged`] fires only when it actually changed. The bar renders **headers only**;
//! this demo shows the intended wiring: each tab owns a group of widget entities and a tiny
//! system toggles the groups' [`UiNode::visible`] to match the active tab (hidden widgets are
//! also skipped by Tab-focus cycling, so keyboard nav never lands on an inactive tab's content).
//!
//! - **Click** a tab header — switch tabs
//! - **Tab** / **Shift+Tab** — move focus (the bar, then the active tab's widgets)
//! - **←/→** — step the active tab while the bar is focused
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/ui_tabs.png cargo run --example ui_tabs`): captures the Stats
//! tab active. `HEADLESS_FRAMES=N` overrides the default 10 frames.
use engine::{
    App, CheckBox, Color, DrawText, Entity, Events, InputState, KeyCode, ProgressBar, ScrollView,
    ShouldQuit, Slider, System, TabBar, TextQueue, UiEvent, UiNode, UiSystem, Vec2, WindowConfig,
    World,
};

const WIN_W: u32 = 720;
const WIN_H: u32 = 420;

struct Demo {
    bar: Entity,
    /// Widget entities per tab, index-aligned with the bar's `tabs`.
    groups: Vec<Vec<Entity>>,
    switches: usize,
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

        // Count tab switches from the event bus (fires only on an actual change).
        if let Some(events) = world.resource::<Events<UiEvent>>() {
            self.switches += events
                .read()
                .iter()
                .filter(|e| matches!(e, UiEvent::TabChanged(_, _)))
                .count();
        }

        // The whole "tab container": show the active tab's widgets, hide the rest.
        let (active, title) = world
            .get::<TabBar>(self.bar)
            .map(|tb| {
                (
                    tb.selected_index(),
                    tb.selected_tab().unwrap_or("").to_owned(),
                )
            })
            .unwrap_or((0, String::new()));
        for (i, group) in self.groups.iter().enumerate() {
            for &e in group {
                if let Some(node) = world.get_mut::<UiNode>(e) {
                    node.visible = i == active;
                }
            }
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "TabBar — headers switch per-tab widget groups (click / Tab + arrows)",
                Vec2::new(28.0, 24.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            if active == 0 {
                tq.push(DrawText::new(
                    "HP",
                    Vec2::new(60.0, 156.0),
                    15.0,
                    Color::rgb(0.78, 0.82, 0.9),
                ));
                tq.push(DrawText::new(
                    "MP",
                    Vec2::new(60.0, 206.0),
                    15.0,
                    Color::rgb(0.78, 0.82, 0.9),
                ));
            }
            tq.push(DrawText::new(
                format!("Active: {title}   switches: {}", self.switches),
                Vec2::new(60.0, 350.0),
                16.0,
                Color::rgb(0.75, 0.9, 0.95),
            ));
            tq.push(DrawText::new(
                "Tab: focus   \u{2190}/\u{2192}: step tab (bar focused)   Esc: quit",
                Vec2::new(28.0, WIN_H as f32 - 28.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "ui_tabs_demo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "ui_tabs — tab bar + per-tab content".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.register_event::<UiEvent>();

    let bar = app.world.spawn();
    app.world
        .add_component(bar, UiNode::new(60.0, 80.0, 460.0, 34.0));
    app.world.add_component(
        bar,
        TabBar::new(["Stats", "Inventory", "Options"]).with_corner_radius(6.0),
    );

    // ── Tab 0: Stats — two read-only gauges ───────────────────────────────────
    let bg = Color::rgba(0.14, 0.15, 0.19, 1.0);
    let hp = app.world.spawn();
    app.world
        .add_component(hp, UiNode::new(100.0, 150.0, 300.0, 24.0));
    app.world.add_component(
        hp,
        ProgressBar::new(0.72)
            .with_colors(Color::rgb(0.35, 0.82, 0.42), bg)
            .with_corner_radius(6.0),
    );
    let mp = app.world.spawn();
    app.world
        .add_component(mp, UiNode::new(100.0, 200.0, 300.0, 24.0));
    app.world.add_component(
        mp,
        ProgressBar::new(0.40)
            .with_colors(Color::rgb(0.30, 0.66, 0.95), bg)
            .with_corner_radius(6.0),
    );

    // ── Tab 1: Inventory — a scroll list ──────────────────────────────────────
    let inv = app.world.spawn();
    app.world
        .add_component(inv, UiNode::new(60.0, 140.0, 300.0, 160.0));
    app.world.add_component(
        inv,
        ScrollView::new().with_items(
            (1..=14)
                .map(|i| format!("Item {i:02} — rusty artifact"))
                .collect::<Vec<_>>(),
        ),
    );

    // ── Tab 2: Options — a checkbox + a slider ────────────────────────────────
    let fullscreen = app.world.spawn();
    app.world
        .add_component(fullscreen, UiNode::new(60.0, 150.0, 240.0, 26.0));
    app.world
        .add_component(fullscreen, CheckBox::new("Fullscreen").with_checked(true));
    let volume = app.world.spawn();
    app.world
        .add_component(volume, UiNode::new(60.0, 200.0, 260.0, 22.0));
    app.world
        .add_component(volume, Slider::new(0.0, 100.0, 65.0));

    app.add_system(UiSystem::new());
    app.add_system(Demo {
        bar,
        groups: vec![vec![hp, mp], vec![inv], vec![fullscreen, volume]],
        switches: 0,
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
