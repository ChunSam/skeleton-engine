//! dialogue_style — restyling `DialogueSystem` via the `DialogueStyle` resource.
//!
//! The same `DialogueBox` is drawn two ways: the engine default (absent the resource) and a custom
//! `DialogueStyle` (different colors, font sizes, margins, and hint label) — restyling dialogue
//! WITHOUT forking the engine. Press T to toggle between them and watch the box change live.
//!
//! Run from the repo root:  `cargo run --example dialogue_style`
//! SPACE = advance · T = toggle style · ESC = quit.

use engine::{
    App, Color, DialogueBox, DialogueStyle, DialogueSystem, Entity, InputState, KeyCode,
    ShouldQuit, System, TextQueue, Vec2, WindowConfig, World,
};

/// A deliberately distinct style so the override is obvious at a glance: cyan speaker, larger body,
/// wider margin, a different advance hint. Every field defaults to the engine look, so we tweak a
/// few and spread the rest from `default()`.
fn custom_style() -> DialogueStyle {
    DialogueStyle {
        text_margin: 110.0,
        speaker_font_size: 28.0,
        speaker_color: Color::rgb(0.35, 0.9, 1.0),
        body_font_size: 24.0,
        body_color: Color::rgb(0.92, 0.96, 1.0),
        hint_label: "▶ SPACE".to_string(),
        hint_color: Color::rgb(1.0, 0.7, 0.4),
        ..Default::default()
    }
}

struct StyleDemo {
    box_entity: Entity,
    custom: bool,
}

impl System for StyleDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (advance, toggle, quit) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.just_pressed(KeyCode::Space),
                    i.just_pressed(KeyCode::KeyT),
                    i.just_pressed(KeyCode::Escape),
                )
            })
            .unwrap_or((false, false, false));

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // T: toggle the DialogueStyle resource on/off. Removing it falls back to the engine default;
        // inserting `custom_style()` restyles every active box the next frame.
        if toggle {
            self.custom = !self.custom;
            if self.custom {
                world.insert_resource(custom_style());
            } else {
                world.remove_resource::<DialogueStyle>();
            }
        }

        if advance {
            if let Some(d) = world.get_mut::<DialogueBox>(self.box_entity) {
                d.advance();
                if d.is_finished() {
                    // Loop the conversation so the demo never dead-ends.
                    *d = demo_box();
                }
            }
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(engine::DrawText::new(
                format!(
                    "SPACE: advance   T: toggle style ({})   ESC: quit",
                    if self.custom { "CUSTOM" } else { "default" }
                ),
                Vec2::new(60.0, 24.0),
                18.0,
                Color::rgb(0.6, 0.7, 0.85),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "style_demo"
    }
}

fn demo_box() -> DialogueBox {
    DialogueBox::new(
        "Stylist",
        [
            "This box is drawn by DialogueSystem.",
            "Press T — same box, restyled by a DialogueStyle resource.",
        ],
    )
    .with_chars_per_sec(34.0)
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "dialogue_style — DialogueStyle resource".to_string(),
        width: 900,
        height: 600,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    // Start with the custom style so the override shows immediately; T toggles back to default.
    app.world.insert_resource(custom_style());

    let box_entity = app.world.spawn();
    app.world.add_component(box_entity, demo_box());

    app.add_system(DialogueSystem);
    app.add_system(StyleDemo {
        box_entity,
        custom: true,
    });
    app.run();
}
