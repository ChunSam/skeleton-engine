//! `Switch` — a styled boolean toggle: a sliding track + knob, the switch-look alternative to a
//! [`CheckBox`].
//!
//! Attach a [`Switch`] next to a [`UiNode`]; clicking anywhere on the node flips it (like a
//! [`CheckBox`]), the knob slides between left (off) and right (on), and
//! [`UiEvent::SwitchToggled`] carries the new state. The switch is one focus stop: **Tab** focuses
//! it, **Enter/Space** toggle it, and **←/→** turn it off/on without the pointer.
//!
//! This demo wires four switches (Sound / Music / Fullscreen / a custom-styled Vsync) to a live HUD
//! readout and a change counter.
//!
//! - **Click** a switch — flip it
//! - **Tab** / **Shift+Tab** — move focus between the switches
//! - **Enter/Space** — toggle the focused switch; **←/→** — turn it off/on
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/ui_switch.png cargo run --example ui_switch`): captures the four
//! switches in a mix of on/off states. `HEADLESS_FRAMES=N` overrides the default 10 frames.
use engine::{
    App, Color, DrawText, Entity, Events, InputState, KeyCode, ShouldQuit, Switch, System,
    TextQueue, UiEvent, UiNode, UiSystem, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 620;
const WIN_H: u32 = 380;

struct Demo {
    switches: Vec<(Entity, &'static str)>,
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

        // Count toggles from the event bus (fires on every flip / actual state change).
        if let Some(events) = world.resource::<Events<UiEvent>>() {
            self.changes += events
                .read()
                .iter()
                .filter(|e| matches!(e, UiEvent::SwitchToggled(_, _)))
                .count();
        }

        let states: Vec<String> = self
            .switches
            .iter()
            .map(|(e, name)| {
                let on = world.get::<Switch>(*e).map(|s| s.on).unwrap_or(false);
                format!("{name}: {}", if on { "ON" } else { "off" })
            })
            .collect();

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Switch — boolean toggle (click, or Tab + Enter / \u{2190}\u{2192})",
                Vec2::new(28.0, 24.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                states.join("    "),
                Vec2::new(60.0, 288.0),
                16.0,
                Color::rgb(0.75, 0.9, 0.95),
            ));
            tq.push(DrawText::new(
                format!("changes: {}", self.changes),
                Vec2::new(60.0, 312.0),
                16.0,
                Color::rgb(0.7, 0.85, 0.78),
            ));
            tq.push(DrawText::new(
                "Tab: focus   Enter/Space: toggle   \u{2190}/\u{2192}: off/on   Esc: quit",
                Vec2::new(28.0, WIN_H as f32 - 26.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "ui_switch_demo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "ui_switch — boolean toggles".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.register_event::<UiEvent>();

    // Sound (on) / Music (off) / Fullscreen (off) — default style; each node leaves room for the
    // label drawn to the right of the track.
    let mut spawn = |y: f32, sw: Switch| {
        let e = app.world.spawn();
        app.world
            .add_component(e, UiNode::new(60.0, y, 240.0, 40.0));
        app.world.add_component(e, sw);
        e
    };
    let sound = spawn(84.0, Switch::new("Sound").with_on(true));
    let music = spawn(134.0, Switch::new("Music"));
    let fullscreen = spawn(184.0, Switch::new("Fullscreen"));

    // Vsync — a larger, custom green-on switch to show the styling surface.
    let vsync = app.world.spawn();
    app.world
        .add_component(vsync, UiNode::new(60.0, 234.0, 240.0, 40.0));
    app.world.add_component(
        vsync,
        Switch::new("Vsync")
            .with_size(60.0, 30.0)
            .with_colors(
                Color::rgba(0.24, 0.62, 0.40, 1.0),
                Color::rgba(0.20, 0.22, 0.24, 1.0),
                Color::rgba_u8(232, 240, 232, 255),
            )
            .with_text_color(Color::rgb(0.82, 0.9, 0.84)),
    );

    app.add_system(UiSystem::new());
    app.add_system(Demo {
        switches: vec![
            (sound, "Sound"),
            (music, "Music"),
            (fullscreen, "Fullscreen"),
            (vsync, "Vsync"),
        ],
        changes: 0,
    });

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        // Set a mix of states so the capture shows knobs on both sides.
        for (e, on) in [
            (sound, true),
            (music, true),
            (fullscreen, false),
            (vsync, true),
        ] {
            if let Some(s) = app.world.get_mut::<Switch>(e) {
                s.on = on;
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
