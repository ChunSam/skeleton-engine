//! dialogue_demo — the `DialogueBox` primitive in real play.
//!
//! A short multi-speaker conversation with a typewriter reveal. `DialogueSystem` (engine) ticks
//! the typewriter and draws the active line; this example's own system feeds input: SPACE either
//! completes the current line's reveal or advances to the next line / speaker, and loads the next
//! conversation segment when one finishes.
//!
//! Run from the repo root:  `cargo run --example dialogue_demo`
//! SPACE = advance · ESC = quit.

use engine::{
    App, Color, DialogueBox, DialogueSystem, Entity, InputState, KeyCode, ShouldQuit, System,
    TextQueue, Vec2, ViewportSize, WindowConfig, World,
};

/// Drives the conversation: feeds SPACE into the box and swaps in the next speaker's lines when
/// the current segment finishes.
struct ConversationSystem {
    box_entity: Entity,
    /// (speaker, lines) segments, played in order.
    script: Vec<(&'static str, Vec<&'static str>)>,
    seg: usize,
}

impl ConversationSystem {
    fn segment_box(&self, i: usize) -> DialogueBox {
        let (speaker, lines) = &self.script[i];
        DialogueBox::new(*speaker, lines.iter().copied()).with_chars_per_sec(32.0)
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
            // Advance the box; if that finished the segment, load the next one.
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

        // HUD: instructions, and an end note once the whole script is done.
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
                    Vec2::new(60.0, vh - 118.0),
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
        title: "dialogue_demo — DialogueBox + typewriter".to_string(),
        width: 900,
        height: 600,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    let script: Vec<(&'static str, Vec<&'static str>)> = vec![
        (
            "Guide",
            vec![
                "Welcome to the skeleton engine.",
                "This whole box is one DialogueBox component.",
            ],
        ),
        (
            "Traveler",
            vec!["So the text types itself out?", "And SPACE moves us along?"],
        ),
        (
            "Guide",
            vec![
                "Exactly — press SPACE once to finish a line,",
                "again to advance. Fork it and make it yours.",
            ],
        ),
    ];

    let box_entity = app.world.spawn();
    app.world.add_component(
        box_entity,
        DialogueBox::new(script[0].0, script[0].1.iter().copied()).with_chars_per_sec(32.0),
    );

    app.add_system(DialogueSystem);
    app.add_system(ConversationSystem {
        box_entity,
        script,
        seg: 0,
    });
    app.run();
}
