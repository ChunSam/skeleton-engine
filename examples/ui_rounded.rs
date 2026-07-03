//! ui_rounded — rounded-corner UI primitives + a rounded keyboard-focus ring.
//!
//! `DrawRect` gained two optional, default-off knobs rendered by a dedicated SDF UI shader:
//! - **`with_corner_radius(r)`** rounds the rect's corners (0.0 = sharp, byte-identical to before),
//! - **`with_border(w)`** draws only an inset outline ring of width `w` instead of a filled rect.
//!
//! `FocusRingStyle::corner_radius` uses the same machinery so the engine's keyboard-focus ring can
//! be rounded too. This example shows all of it in one window:
//! - a rounded "card" panel behind a column of focusable widgets,
//! - a showcase column demonstrating the three fill modes (sharp fill / rounded fill / rounded
//!   outline),
//! - **rounded buttons** — `Button` styled through its `with_*` builders
//!   (`with_corner_radius` / `with_colors` / `with_text_color` / `with_font_size`, the same
//!   convention as `TabBar`/`Dropdown`/`RadioGroup`), and
//! - a **rounded** focus ring you can Tab around the widgets to see.
//!
//! Run from the repo root:  `cargo run --example ui_rounded`
//! Tab/Shift+Tab = move focus · Enter/Space = activate · <-/-> = slider · ESC = quit.
//!
//! Headless (`HEADLESS_SHOT=/tmp/ui_rounded.png cargo run --example ui_rounded`): renders the
//! window to a PNG with no display; `HEADLESS_FRAMES=N` overrides the default 10 warm-up frames.

use engine::{
    App, Button, CheckBox, Color, DrawRect, DrawText, Events, FocusRingStyle, InputState, KeyCode,
    ShouldQuit, Slider, System, TextInput, TextQueue, UiEvent, UiFocus, UiNode, UiQueue, UiSystem,
    Vec2, WindowConfig, World,
};

const CARD_BG: Color = Color::rgba(0.13, 0.15, 0.21, 1.0);
const SWATCH: Color = Color::rgba(0.32, 0.55, 0.92, 1.0);
const RING: Color = Color::rgba(0.45, 0.9, 0.6, 1.0);

/// Draws the static rounded showcase (a card panel + three fill-mode swatches with labels) every
/// frame — the `UiQueue` is drained each frame, so these are re-pushed — and reports focus state.
struct RoundedDemo {
    last: String,
}

impl System for RoundedDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if world
            .resource::<InputState>()
            .is_some_and(|i| i.just_pressed(KeyCode::Escape))
        {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        if let Some(events) = world.resource::<Events<UiEvent>>() {
            for ev in events.read() {
                self.last = match ev {
                    UiEvent::ButtonClicked(e) => format!("ButtonClicked({})", e.index()),
                    UiEvent::CheckBoxToggled(e, v) => {
                        format!("CheckBoxToggled({}, {v})", e.index())
                    }
                    UiEvent::SliderChanged(e, v) => format!("SliderChanged({}, {v:.0})", e.index()),
                    other => format!("{other:?}"),
                };
            }
        }

        let focused = world
            .resource::<UiFocus>()
            .and_then(|f| f.entity)
            .map(|e| format!("entity {}", e.index()))
            .unwrap_or_else(|| "none".to_string());

        // ── Rounded primitives (drawn behind / beside the widgets) ──────────────
        if let Some(ui) = world.resource_mut::<UiQueue>() {
            // Card panel behind the widget column — a rounded *filled* rect (z behind widgets).
            ui.push(
                DrawRect::new(36.0, 64.0, 320.0, 396.0, CARD_BG)
                    .with_corner_radius(18.0)
                    .with_z(-0.5),
            );

            // Showcase column: the three fill modes, top to bottom.
            // (a) sharp fill — corner_radius 0.0 takes the shader fast path (identical to before).
            ui.push(DrawRect::new(440.0, 110.0, 330.0, 60.0, SWATCH).with_z(0.2));
            // (b) rounded fill — a filled rect with rounded corners.
            ui.push(
                DrawRect::new(440.0, 210.0, 330.0, 60.0, SWATCH)
                    .with_corner_radius(18.0)
                    .with_z(0.2),
            );
            // (c) rounded outline — only a `border`-wide inset ring is drawn.
            ui.push(
                DrawRect::new(440.0, 310.0, 330.0, 60.0, RING)
                    .with_corner_radius(18.0)
                    .with_border(6.0)
                    .with_z(0.2),
            );
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Tab/Shift+Tab: focus (rounded ring)   Enter/Space: activate   <-/->: slider   ESC: quit",
                Vec2::new(36.0, 24.0),
                15.0,
                Color::rgb(0.7, 0.78, 0.92),
            ));
            tq.push(DrawText::new(
                "sharp fill (corner_radius 0)",
                Vec2::new(444.0, 90.0),
                14.0,
                Color::rgb(0.8, 0.85, 0.95),
            ));
            tq.push(DrawText::new(
                "rounded fill (corner_radius 18)",
                Vec2::new(444.0, 190.0),
                14.0,
                Color::rgb(0.8, 0.85, 0.95),
            ));
            tq.push(DrawText::new(
                "rounded outline (corner_radius 18, border 6)",
                Vec2::new(444.0, 290.0),
                14.0,
                Color::rgb(0.8, 0.85, 0.95),
            ));
            tq.push(DrawText::new(
                format!("focused: {focused}        last event: {}", self.last),
                Vec2::new(36.0, 476.0),
                18.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "rounded_demo"
    }
}

fn spawn_widget<C: Send + Sync + 'static>(app: &mut App, y: f32, w: f32, h: f32, widget: C) {
    let e = app.world.spawn();
    app.world.add_component(e, UiNode::new(60.0, y, w, h));
    app.world.add_component(e, widget);
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "ui_rounded — rounded UI primitives + rounded focus ring".to_string(),
        width: 820,
        height: 520,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.register_event::<UiEvent>();

    // A rounded focus ring: same `FocusRingStyle` resource as `ui_focus`, now with a corner radius.
    // `corner_radius = 0.0` (the default) keeps the historical sharp four-bar ring.
    app.world.insert_resource(FocusRingStyle {
        color: Color::rgb(0.3, 0.9, 1.0),
        thickness: 4.0,
        corner_radius: 10.0,
        pulse_hz: 1.2,
        pulse_min_alpha: 0.4,
        ..Default::default()
    });

    // A column of focusable widgets (Tab order = spawn/entity order), sitting on the rounded card.
    // The buttons use the EW-005 builder surface: rounded corners + chained style, matching the
    // newer widgets' `with_*` convention (TabBar/Dropdown/RadioGroup).
    spawn_widget(
        &mut app,
        96.0,
        260.0,
        48.0,
        Button::new("Play")
            .with_corner_radius(12.0)
            .with_font_size(20.0)
            .with_text_color(Color::rgba_u8(235, 240, 250, 255)),
    );
    spawn_widget(&mut app, 168.0, 280.0, 36.0, CheckBox::new("Fullscreen"));
    spawn_widget(&mut app, 232.0, 280.0, 28.0, Slider::new(0.0, 100.0, 50.0));
    spawn_widget(&mut app, 296.0, 280.0, 40.0, TextInput::new("your name…"));
    spawn_widget(
        &mut app,
        372.0,
        260.0,
        48.0,
        Button::new("Quit")
            .with_corner_radius(12.0)
            .with_colors(
                Color::rgba(0.30, 0.14, 0.16, 1.0),
                Color::rgba(0.44, 0.20, 0.22, 1.0),
                Color::rgba(0.22, 0.10, 0.12, 1.0),
            )
            .with_text_color(Color::rgba_u8(245, 205, 205, 255)),
    );

    app.add_system(UiSystem::new());
    app.add_system(RoundedDemo {
        last: "(none)".to_string(),
    });

    // `HEADLESS_SHOT=path` → render to a PNG with no window and exit.
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
