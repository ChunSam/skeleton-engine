//! ui_focus — keyboard focus navigation for UI widgets (`UiFocus` + `UiSystem`'s focus pass).
//!
//! A column of widgets (button, checkbox, slider, text field, button). `UiSystem` now cycles
//! keyboard focus across them:
//! - **Tab / Shift+Tab** (or a gamepad **D-pad** / **left stick** Down / Up) move focus (a ring is
//!   drawn around the focused widget),
//! - **Enter / Space** (or gamepad **A**) activate the focused button (click) or checkbox (toggle),
//! - **Left / Right** (or gamepad **D-pad** / **left stick** Left / Right) nudge a focused slider,
//! - clicking a widget also focuses it, and a focused text field receives typed characters.
//!
//! The engine owns all of that (the focus pass folds the first connected gamepad in alongside the
//! keyboard — the left stick is edge-detected, so one push = one step); this example only spawns
//! widgets and prints what happened.
//!
//! Run from the repo root:  `cargo run --example ui_focus`
//! Tab/Shift+Tab, D-pad or left stick = move focus · Enter/Space or A = activate · arrows, D-pad or stick = slider · ESC = quit.

use engine::{
    App, Button, CheckBox, Color, DrawText, Events, InputState, KeyCode, ShouldQuit, Slider,
    System, TextInput, TextQueue, UiEvent, UiFocus, UiNode, UiSystem, Vec2, WindowConfig, World,
};

/// Spawns the widgets and reports focus + the latest UI event as on-screen text.
struct FocusDemo {
    last: String,
}

impl System for FocusDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if world
            .resource::<InputState>()
            .is_some_and(|i| i.just_pressed(KeyCode::Escape))
        {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // Report UI events emitted by the focus pass (and mouse).
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

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Tab/Shift+Tab, D-pad or L-stick: focus   Enter/Space or A: activate   <-/->, D-pad or stick: slider   ESC: quit",
                Vec2::new(60.0, 24.0),
                15.0,
                Color::rgb(0.7, 0.78, 0.92),
            ));
            tq.push(DrawText::new(
                format!("focused: {focused}        last event: {}", self.last),
                Vec2::new(60.0, 470.0),
                18.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "focus_demo"
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
        title: "ui_focus — keyboard focus navigation".to_string(),
        width: 760,
        height: 520,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.register_event::<UiEvent>();

    // A column of focusable widgets (Tab order = spawn/entity order).
    spawn_widget(&mut app, 80.0, 220.0, 48.0, Button::new("Play"));
    spawn_widget(&mut app, 150.0, 260.0, 36.0, CheckBox::new("Fullscreen"));
    spawn_widget(&mut app, 215.0, 320.0, 28.0, Slider::new(0.0, 100.0, 50.0));
    spawn_widget(&mut app, 275.0, 320.0, 40.0, TextInput::new("your name…"));
    spawn_widget(&mut app, 345.0, 220.0, 48.0, Button::new("Quit"));

    app.add_system(UiSystem::new());
    app.add_system(FocusDemo {
        last: "(none)".to_string(),
    });
    app.run();
}
