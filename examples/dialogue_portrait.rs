//! dialogue_portrait — per-line speaker portraits in a `DialogueBox`.
//!
//! Same typewriter conversation as `dialogue_demo`, but each segment sets a `portrait` image via
//! [`DialogueBox::with_portrait`]. `DialogueSystem` (engine) now draws that portrait to the left of
//! the text and shifts the text right to clear it — so the face on screen changes as the speaker
//! does. A box with no portrait renders exactly as before (the last segment omits one to show the
//! fallback).
//!
//! Run from the repo root:  `cargo run --example dialogue_portrait`
//! SPACE = advance · ESC = quit.

use engine::{
    App, Color, DialogueBox, DialogueSystem, Entity, Handle, ImageAsset, InputState, KeyCode,
    ShouldQuit, System, TextQueue, Vec2, ViewportSize, WindowConfig, World,
};

/// One conversation beat: who speaks, their portrait (or none), and their lines.
struct Segment {
    speaker: &'static str,
    portrait: Option<Handle<ImageAsset>>,
    lines: Vec<&'static str>,
}

/// Drives the conversation: feeds SPACE into the box and swaps in the next speaker (and portrait)
/// when the current segment finishes.
struct ConversationSystem {
    box_entity: Entity,
    script: Vec<Segment>,
    seg: usize,
}

impl ConversationSystem {
    fn segment_box(&self, i: usize) -> DialogueBox {
        let s = &self.script[i];
        let mut b = DialogueBox::new(s.speaker, s.lines.iter().copied()).with_chars_per_sec(32.0);
        if let Some(p) = &s.portrait {
            b = b.with_portrait(p.clone());
        }
        b
    }
}

impl System for ConversationSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (mut advance, mut quit) = (false, false);
        if let Some(input) = world.resource::<InputState>() {
            advance = input.just_pressed(KeyCode::Space);
            quit = input.just_pressed(KeyCode::Escape);
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        if advance {
            let finished = if let Some(d) = world.get_mut::<DialogueBox>(self.box_entity) {
                d.advance();
                d.is_finished()
            } else {
                false
            };
            if finished && self.seg + 1 < self.script.len() {
                self.seg += 1;
                let next = self.segment_box(self.seg);
                if let Some(d) = world.get_mut::<DialogueBox>(self.box_entity) {
                    *d = next;
                }
            }
        }

        // HUD: instructions + an end note once the whole script is done.
        let over = world
            .get::<DialogueBox>(self.box_entity)
            .map(|d| d.is_finished())
            .unwrap_or(true)
            && self.seg + 1 >= self.script.len();
        let vh = world
            .resource::<ViewportSize>()
            .map(|v| v.height)
            .unwrap_or(600.0);
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(engine::DrawText::new(
                "SPACE: advance     ESC: quit",
                Vec2::new(60.0, 24.0),
                18.0,
                Color::rgb(0.6, 0.7, 0.85),
            ));
            if over {
                tq.push(engine::DrawText::new(
                    "[ conversation over — ESC to quit ]",
                    Vec2::new(60.0, vh - 200.0),
                    20.0,
                    Color::rgb(0.7, 0.7, 0.75),
                ));
            }
        }
    }

    fn name(&self) -> &'static str {
        "conversation_system"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "dialogue_portrait — per-speaker portraits".to_string(),
        width: 900,
        height: 600,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    // Asset paths are relative to the repo root (see hello_sprite). load_image returns a handle
    // immediately; the GPU upload happens on app start.
    let sage = app.load_image("examples/assets/portrait_sage.png");
    let knight = app.load_image("examples/assets/portrait_knight.png");

    let script = vec![
        Segment {
            speaker: "Sage",
            portrait: Some(sage.clone()),
            lines: vec![
                "A portrait rides along with each line now.",
                "Watch the face change when the speaker does.",
            ],
        },
        Segment {
            speaker: "Knight",
            portrait: Some(knight.clone()),
            lines: vec![
                "So my visage shows while I speak?",
                "Set it with DialogueBox::with_portrait.",
            ],
        },
        Segment {
            speaker: "Sage",
            portrait: Some(sage),
            lines: vec!["Indeed — and a line without one…"],
        },
        Segment {
            speaker: "Narrator",
            portrait: None,
            lines: vec!["…falls back to the original text-only layout."],
        },
    ];

    let box_entity = app.world.spawn();

    let convo = ConversationSystem {
        box_entity,
        script,
        seg: 0,
    };
    app.world.add_component(box_entity, convo.segment_box(0));

    app.add_system(DialogueSystem);
    app.add_system(convo);
    app.run();
}
