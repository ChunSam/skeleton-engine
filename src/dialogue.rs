//! Dialogue box primitive — a speaker + typewriter text box for RPG / visual-novel / narrative
//! games.
//!
//! Every narrative 2D game re-implements the same thing: a box that reveals a line of text one
//! character at a time and advances on a key press. [`DialogueBox`] is that, as a reusable
//! component; [`DialogueSystem`] ticks the typewriter and renders the box (screen-space text via
//! the [`TextQueue`]). The game decides *when* to advance by calling
//! [`DialogueBox::advance`] (e.g. on a Space press) — the system stays input-agnostic.
//!
//! ```no_run
//! use engine::{App, DialogueBox, DialogueSystem};
//! let mut app = App::new();
//! let e = app.world.spawn();
//! app.world.add_component(
//!     e,
//!     DialogueBox::new("Guide", ["Welcome, traveler.", "Press space to continue."])
//!         .with_chars_per_sec(28.0),
//! );
//! app.add_system(DialogueSystem);
//! // in your own system: on Space, `world.get_mut::<DialogueBox>(e).unwrap().advance();`
//! ```

use serde::{Deserialize, Serialize};

use crate::asset::{Handle, ImageAsset};
use crate::ecs::{System, World};
use crate::renderer::{DrawText, TextQueue};
use crate::resources::ViewportSize;

/// A dialogue / textbox: a sequence of lines, each revealed with a typewriter effect and advanced
/// one at a time. Attach it to an entity and add a [`DialogueSystem`]; call [`advance`](Self::advance)
/// from your own input handling to move through the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueBox {
    /// Speaker name shown above the body (empty = no speaker line).
    pub speaker: String,
    /// The conversation lines, shown one at a time.
    pub lines: Vec<String>,
    /// Index of the line currently showing. `>= lines.len()` once the conversation is finished.
    pub current: usize,
    /// Typewriter speed in characters per second. `<= 0.0` reveals each line instantly.
    pub chars_per_sec: f32,
    /// Seconds the current line has been revealing.
    elapsed: f32,
    /// Whether the current line was force-completed (first [`advance`](Self::advance) press).
    full: bool,
    /// Optional speaker portrait (runtime-only; not serialized).
    #[serde(skip)]
    pub portrait: Option<Handle<ImageAsset>>,
}

impl DialogueBox {
    /// Creates a dialogue box for `speaker` with the given `lines` (default 28 chars/sec).
    pub fn new<S: Into<String>>(
        speaker: impl Into<String>,
        lines: impl IntoIterator<Item = S>,
    ) -> Self {
        Self {
            speaker: speaker.into(),
            lines: lines.into_iter().map(Into::into).collect(),
            current: 0,
            chars_per_sec: 28.0,
            elapsed: 0.0,
            full: false,
            portrait: None,
        }
    }

    /// Sets the typewriter speed (builder). `<= 0.0` = instant reveal.
    pub fn with_chars_per_sec(mut self, cps: f32) -> Self {
        self.chars_per_sec = cps;
        self
    }

    /// Sets a speaker portrait handle (builder).
    pub fn with_portrait(mut self, handle: Handle<ImageAsset>) -> Self {
        self.portrait = Some(handle);
        self
    }

    /// Advances the typewriter by `dt`. Called by [`DialogueSystem`]; no effect once finished.
    pub fn tick(&mut self, dt: f32) {
        if !self.is_finished() {
            self.elapsed += dt;
        }
    }

    /// The current line's full text, or `None` once the conversation is finished.
    pub fn current_line(&self) -> Option<&str> {
        self.lines.get(self.current).map(|s| s.as_str())
    }

    /// The portion of the current line revealed so far (the whole line when finished revealing).
    pub fn visible_text(&self) -> &str {
        let line = match self.current_line() {
            Some(l) => l,
            None => return "",
        };
        if self.full || self.chars_per_sec <= 0.0 {
            return line;
        }
        let n = (self.elapsed * self.chars_per_sec) as usize;
        match line.char_indices().nth(n) {
            Some((byte, _)) => &line[..byte],
            None => line, // revealed past the end
        }
    }

    /// Whether the current line is fully revealed (or there is no line left).
    pub fn line_fully_revealed(&self) -> bool {
        match self.current_line() {
            None => true,
            Some(line) => {
                self.full
                    || self.chars_per_sec <= 0.0
                    || (self.elapsed * self.chars_per_sec) as usize >= line.chars().count()
            }
        }
    }

    /// Advances the conversation: the first press completes the current line's reveal; a press once
    /// the line is fully shown moves to the next line (or finishes after the last one).
    pub fn advance(&mut self) {
        if self.is_finished() {
            return;
        }
        if !self.line_fully_revealed() {
            self.full = true;
        } else {
            self.current += 1;
            self.elapsed = 0.0;
            self.full = false;
        }
    }

    /// Whether the conversation has run past its last line.
    pub fn is_finished(&self) -> bool {
        self.current >= self.lines.len()
    }

    /// Restarts the conversation from the first line.
    pub fn reset(&mut self) {
        self.current = 0;
        self.elapsed = 0.0;
        self.full = false;
    }
}

/// Ticks every [`DialogueBox`]'s typewriter and renders the active box (speaker + revealed text +
/// an advance hint) as screen-space text near the bottom of the viewport.
///
/// Input-agnostic: the game advances a box by calling [`DialogueBox::advance`]. Rendering is
/// text-only (no background panel) so it composes with whatever box art the game draws.
pub struct DialogueSystem;

impl System for DialogueSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // 1. Advance every dialogue box's typewriter (Phase-3 mutable query — no collect).
        for (_e, d) in world.query_mut::<DialogueBox>() {
            d.tick(dt);
        }

        // 2. Gather what to draw for active boxes (releases the query borrow).
        let (vw, vh) = world
            .resource::<ViewportSize>()
            .map(|v| (v.width, v.height))
            .unwrap_or((1280.0, 720.0));
        let items: Vec<(String, String, bool)> = world
            .query::<DialogueBox>()
            .filter(|(_, d)| !d.is_finished())
            .map(|(_, d)| {
                (
                    d.speaker.clone(),
                    d.visible_text().to_string(),
                    d.line_fully_revealed(),
                )
            })
            .collect();
        if items.is_empty() {
            return;
        }

        // 3. Render near the bottom of the screen.
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            let x0 = 60.0;
            for (speaker, body, full) in items {
                if !speaker.is_empty() {
                    tq.push(DrawText::new(
                        speaker,
                        crate::Vec2::new(x0, vh - 150.0),
                        22.0,
                        crate::Color::rgb(1.0, 0.85, 0.35),
                    ));
                }
                tq.push(
                    DrawText::new(
                        body,
                        crate::Vec2::new(x0, vh - 118.0),
                        20.0,
                        crate::Color::WHITE,
                    )
                    .with_bounds(crate::Vec2::new(vw - 2.0 * x0, 90.0)),
                );
                if full {
                    tq.push(DrawText::new(
                        "▼ space",
                        crate::Vec2::new(vw - 150.0, vh - 42.0),
                        16.0,
                        crate::Color::rgb(0.6, 0.7, 0.85),
                    ));
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "dialogue_system"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typewriter_reveals_over_time() {
        let mut d = DialogueBox::new("", ["hello"]).with_chars_per_sec(10.0);
        assert_eq!(d.visible_text(), "");
        d.tick(0.25); // 2.5 chars
        assert_eq!(d.visible_text(), "he");
        d.tick(0.30); // 5.5 chars → full
        assert_eq!(d.visible_text(), "hello");
        assert!(d.line_fully_revealed());
    }

    #[test]
    fn instant_when_cps_zero() {
        let d = DialogueBox::new("", ["instant"]).with_chars_per_sec(0.0);
        assert_eq!(d.visible_text(), "instant");
        assert!(d.line_fully_revealed());
    }

    #[test]
    fn advance_completes_then_moves_to_next_line() {
        let mut d = DialogueBox::new("NPC", ["one", "two"]).with_chars_per_sec(10.0);
        d.tick(0.1); // mid-reveal of "one"
        assert!(!d.line_fully_revealed());
        d.advance(); // first press → complete the line
        assert_eq!(d.visible_text(), "one");
        assert_eq!(d.current, 0);
        d.advance(); // second press → next line
        assert_eq!(d.current, 1);
        assert_eq!(d.visible_text(), "");
    }

    #[test]
    fn finishes_after_last_line() {
        let mut d = DialogueBox::new("", ["only"]).with_chars_per_sec(0.0);
        assert!(!d.is_finished());
        d.advance(); // line is already full (cps 0) → advances past the end
        assert!(d.is_finished());
        assert_eq!(d.visible_text(), "");
        d.advance(); // no-op when finished
        assert!(d.is_finished());
    }

    #[test]
    fn utf8_safe_reveal() {
        let mut d = DialogueBox::new("", ["héllo"]).with_chars_per_sec(10.0);
        d.tick(0.25); // 2 chars: "hé" (must not split the multibyte 'é')
        assert_eq!(d.visible_text(), "hé");
    }
}
