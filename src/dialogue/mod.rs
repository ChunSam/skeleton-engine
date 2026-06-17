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
use crate::ecs::{Entity, Events, System, World};
use crate::locale::LocaleResource;
use crate::renderer::{DrawText, TextQueue};
use crate::resources::ViewportSize;

mod tree;
mod vars;
pub use tree::{DialogueRegistry, DialogueTree};
pub use vars::{
    DialogueCond, DialogueEffect, DialogueEvent, DialogueOp, DialogueValue, DialogueVars,
};

/// One branching choice presented at a [`DialogueBox`] line: a label plus the line index the
/// conversation jumps to when the player selects it.
///
/// `text` is the display label; `key` is an optional localization key — when set,
/// [`DialogueBox::resolve`] fills `text` from the active [`LocaleResource`] (same as line keys).
/// `goto` is the target line index ([`DialogueBox::choose`] clamps an out-of-range value to the
/// end, finishing the conversation, rather than panicking).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueChoice {
    /// Display label for this choice (resolved from `key` when localized).
    pub text: String,
    /// Optional localization key; when set, [`DialogueBox::resolve`] fills `text` from the locale.
    #[serde(default)]
    pub key: Option<String>,
    /// Line index the conversation jumps to when this choice is selected.
    pub goto: usize,
    /// Optional gate: the choice is only shown when this condition passes against
    /// [`DialogueVars`] (see [`DialogueBox::visible_choices`]). `None` = always shown.
    #[serde(default)]
    pub cond: Option<DialogueCond>,
    /// Optional side effect applied when the choice is taken (set a var / emit an event) via
    /// [`dialogue::choose`](crate::dialogue::choose). `None` = no effect.
    #[serde(default)]
    pub effect: Option<DialogueEffect>,
}

impl DialogueChoice {
    /// A literal-text choice that jumps to line `goto` when chosen.
    pub fn new(text: impl Into<String>, goto: usize) -> Self {
        Self {
            text: text.into(),
            key: None,
            goto,
            cond: None,
            effect: None,
        }
    }

    /// A localized choice: `key` is resolved into `text` via the active [`LocaleResource`].
    pub fn localized(key: impl Into<String>, goto: usize) -> Self {
        Self {
            text: String::new(),
            key: Some(key.into()),
            goto,
            cond: None,
            effect: None,
        }
    }

    /// Gates this choice on `cond` (builder): it is only shown while the condition passes.
    pub fn when(mut self, cond: DialogueCond) -> Self {
        self.cond = Some(cond);
        self
    }

    /// Attaches a side `effect` applied when this choice is taken (builder).
    pub fn then(mut self, effect: DialogueEffect) -> Self {
        self.effect = Some(effect);
        self
    }

    /// Whether this choice is currently available (its `cond` passes, or it has none).
    fn is_available(&self, vars: &DialogueVars) -> bool {
        self.cond.as_ref().is_none_or(|c| c.eval(vars))
    }
}

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
    /// Localization keys for each line. When non-empty, [`resolve`](Self::resolve) fills
    /// [`lines`](Self::lines) by looking each key up in a [`LocaleResource`] — empty = literal
    /// mode (use `lines` directly, byte-identical to pre-localization behavior).
    #[serde(default)]
    pub line_keys: Vec<String>,
    /// Localization key for the speaker name, resolved into [`speaker`](Self::speaker). `None` =
    /// literal speaker (use `speaker` directly).
    #[serde(default)]
    pub speaker_key: Option<String>,
    /// Branching choices keyed by line index: each `(line, choices)` entry shows `choices` once
    /// line `line` is fully revealed. Empty = linear dialogue (byte-identical prior behavior).
    /// See [`pending_choices`](Self::pending_choices) / [`choose`](Self::choose).
    #[serde(default)]
    pub choices: Vec<(usize, Vec<DialogueChoice>)>,
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
            line_keys: Vec::new(),
            speaker_key: None,
            choices: Vec::new(),
            elapsed: 0.0,
            full: false,
            portrait: None,
        }
    }

    /// Creates a *localized* dialogue box: `speaker_key` + `line_keys` are translation keys
    /// resolved against a [`LocaleResource`] each frame by [`DialogueSystem`] (or manually via
    /// [`resolve`](Self::resolve)). [`lines`](Self::lines)/[`speaker`](Self::speaker) start empty
    /// and are filled on the first resolve, so switching locale live retranslates the box.
    ///
    /// ```
    /// use engine::DialogueBox;
    /// let d = DialogueBox::localized("npc.guide", ["intro.welcome", "intro.continue"]);
    /// assert_eq!(d.line_keys.len(), 2);
    /// assert_eq!(d.speaker_key.as_deref(), Some("npc.guide"));
    /// ```
    pub fn localized<S: Into<String>>(
        speaker_key: impl Into<String>,
        line_keys: impl IntoIterator<Item = S>,
    ) -> Self {
        Self {
            speaker: String::new(),
            lines: Vec::new(),
            current: 0,
            chars_per_sec: 28.0,
            line_keys: line_keys.into_iter().map(Into::into).collect(),
            speaker_key: Some(speaker_key.into()),
            choices: Vec::new(),
            elapsed: 0.0,
            full: false,
            portrait: None,
        }
    }

    /// Resolves localization keys against `locale`, filling [`lines`](Self::lines) from
    /// [`line_keys`](Self::line_keys) and [`speaker`](Self::speaker) from
    /// [`speaker_key`](Self::speaker_key). A no-op when `line_keys` is empty (literal mode), so a
    /// non-localized box is untouched. Deliberately does **not** touch `current`/`elapsed`/reveal
    /// state, so it is safe to call every frame — the typewriter keeps its progress across a
    /// live locale switch.
    pub fn resolve(&mut self, locale: &LocaleResource) {
        // Localized choices resolve independently of line keys: a literal-line box may still
        // present localized choices (and vice versa).
        for (_line, choices) in &mut self.choices {
            for choice in choices {
                if let Some(key) = &choice.key {
                    choice.text = locale.t(key).to_string();
                }
            }
        }
        if self.line_keys.is_empty() {
            return;
        }
        self.lines = self
            .line_keys
            .iter()
            .map(|k| locale.t(k).to_string())
            .collect();
        if let Some(key) = &self.speaker_key {
            self.speaker = locale.t(key).to_string();
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

    /// Attaches branching `choices` shown once line `line` is fully revealed (builder, chainable).
    /// Selecting choice `i` jumps the conversation to `choices[i].goto` (see [`choose`](Self::choose)).
    pub fn with_choices(
        mut self,
        line: usize,
        choices: impl IntoIterator<Item = DialogueChoice>,
    ) -> Self {
        self.choices.push((line, choices.into_iter().collect()));
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
    ///
    /// When the current line has [`pending_choices`](Self::pending_choices), `advance` is a no-op —
    /// the game must resolve the branch with [`choose`](Self::choose) so a plain advance can't skip
    /// a decision. (The first press still completes the reveal, *then* the choices appear.)
    pub fn advance(&mut self) {
        if self.is_finished() {
            return;
        }
        if self.pending_choices().is_some() {
            return; // a decision is pending — the game must call `choose`
        }
        if !self.line_fully_revealed() {
            self.full = true;
        } else {
            self.current += 1;
            self.elapsed = 0.0;
            self.full = false;
        }
    }

    /// The choices to present right now, or `None` when none apply. `Some` only when the current
    /// line is fully revealed *and* has a non-empty choice list — i.e. the moment the player must
    /// pick a branch. While the line is still typing out, this is `None` (so [`advance`](Self::advance)
    /// completes the reveal first).
    pub fn pending_choices(&self) -> Option<&[DialogueChoice]> {
        if self.is_finished() || !self.line_fully_revealed() {
            return None;
        }
        self.choices
            .iter()
            .find(|(line, cs)| *line == self.current && !cs.is_empty())
            .map(|(_, cs)| cs.as_slice())
    }

    /// Selects pending choice `i`, jumping the conversation to that choice's `goto` line. A no-op
    /// when no choices are pending or `i` is out of range. An out-of-range `goto` is clamped to the
    /// end (finishing the conversation) rather than panicking.
    pub fn choose(&mut self, i: usize) {
        let goto = match self.pending_choices().and_then(|cs| cs.get(i)) {
            Some(choice) => choice.goto,
            None => return,
        };
        self.current = goto.min(self.lines.len());
        self.elapsed = 0.0;
        self.full = false;
    }

    /// The choices visible right now given `vars`: the [`pending_choices`](Self::pending_choices)
    /// whose [`cond`](DialogueChoice::cond) passes (or have none). Empty when no decision is
    /// pending. The index into this slice is what [`choose_visible`](Self::choose_visible) /
    /// [`dialogue::choose`](crate::dialogue::choose) expect.
    pub fn visible_choices(&self, vars: &DialogueVars) -> Vec<&DialogueChoice> {
        match self.pending_choices() {
            Some(cs) => cs.iter().filter(|c| c.is_available(vars)).collect(),
            None => Vec::new(),
        }
    }

    /// Whether a decision is pending given `vars` — the current line is revealed and at least
    /// one choice is visible. A line whose every choice is gated out is *not* a decision.
    pub fn is_choosing(&self, vars: &DialogueVars) -> bool {
        !self.visible_choices(vars).is_empty()
    }

    /// Like [`advance`](Self::advance) but [`DialogueVars`]-aware: blocks only while
    /// [`is_choosing`](Self::is_choosing) is true, so a line whose choices are all gated out
    /// advances normally. Prefer [`dialogue::advance`](crate::dialogue::advance) from game code
    /// (it clones the resource for you).
    pub fn advance_with(&mut self, vars: &DialogueVars) {
        if self.is_finished() {
            return;
        }
        if self.is_choosing(vars) {
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

    /// Selects the `i`-th *visible* choice (see [`visible_choices`](Self::visible_choices)),
    /// jumping to its `goto` line and returning its [`effect`](DialogueChoice::effect) for the
    /// caller to apply. Returns `None` (a no-op) when `i` is out of range. An out-of-range
    /// `goto` clamps to the end. Prefer [`dialogue::choose`](crate::dialogue::choose), which
    /// also applies the effect.
    pub fn choose_visible(&mut self, i: usize, vars: &DialogueVars) -> Option<DialogueEffect> {
        let (goto, effect) = {
            let visible = self.visible_choices(vars);
            let choice = visible.get(i)?;
            (choice.goto, choice.effect.clone())
        };
        self.current = goto.min(self.lines.len());
        self.elapsed = 0.0;
        self.full = false;
        effect
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

/// Advances the [`DialogueBox`] on `entity`, honoring conditional choices (a no-op while a
/// cond-passing choice is pending). Reads the [`DialogueVars`] resource (or an empty default)
/// to decide. The game-code counterpart to [`DialogueBox::advance`] for gated dialogue — call
/// it on your advance key.
pub fn advance(world: &mut World, entity: Entity) {
    let vars = world
        .resource::<DialogueVars>()
        .cloned()
        .unwrap_or_default();
    if let Some(d) = world.get_mut::<DialogueBox>(entity) {
        d.advance_with(&vars);
    }
}

/// Selects the `visible_index`-th visible choice on `entity`'s [`DialogueBox`], jumping to its
/// target line and applying its [`effect`](DialogueChoice::effect): a `SetVar` writes the
/// [`DialogueVars`] resource (inserting it if absent); an `EmitEvent` sends a [`DialogueEvent`]
/// to `Events<DialogueEvent>` (register the bus with `App::register_event::<DialogueEvent>()`).
pub fn choose(world: &mut World, entity: Entity, visible_index: usize) {
    let vars = world
        .resource::<DialogueVars>()
        .cloned()
        .unwrap_or_default();
    let effect = match world.get_mut::<DialogueBox>(entity) {
        Some(d) => d.choose_visible(visible_index, &vars),
        None => return,
    };
    match effect {
        Some(DialogueEffect::SetVar { key, value }) => {
            if world.resource::<DialogueVars>().is_none() {
                world.insert_resource(DialogueVars::default());
            }
            if let Some(v) = world.resource_mut::<DialogueVars>() {
                v.set(key, value);
            }
        }
        Some(DialogueEffect::EmitEvent { name }) => {
            if let Some(ev) = world.resource_mut::<Events<DialogueEvent>>() {
                ev.send(DialogueEvent { name, entity });
            } else {
                log::debug!(
                    "dialogue::choose: EmitEvent('{name}') dropped — register the bus with \
                     App::register_event::<DialogueEvent>()"
                );
            }
        }
        None => {}
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
        // 0. Resolve localized boxes against the current locale before ticking, so a live
        //    `set_locale` retranslates them. `resolve` needs both `&mut DialogueBox` (query_mut)
        //    and `&LocaleResource`, so clone the resource first to release the world borrow. Only
        //    runs when a `LocaleResource` exists, and `resolve` is a no-op for literal boxes.
        if let Some(locale) = world.resource::<LocaleResource>().cloned() {
            for (_e, d) in world.query_mut::<DialogueBox>() {
                d.resolve(&locale);
            }
        }

        // 1. Advance every dialogue box's typewriter (Phase-3 mutable query — no collect).
        for (_e, d) in world.query_mut::<DialogueBox>() {
            d.tick(dt);
        }

        // 2. Gather what to draw for active boxes (releases the query borrow).
        let (vw, vh) = world
            .resource::<ViewportSize>()
            .map(|v| (v.width, v.height))
            .unwrap_or((1280.0, 720.0));
        // Clone DialogueVars (or empty) to filter conditional choices without holding a
        // resource borrow across the query below.
        let vars = world
            .resource::<DialogueVars>()
            .cloned()
            .unwrap_or_default();
        let items: Vec<(String, String, bool, Vec<String>)> = world
            .query::<DialogueBox>()
            .filter(|(_, d)| !d.is_finished())
            .map(|(_, d)| {
                // Visible (cond-passing) choice labels, already resolved into `text` by step 0.
                let choices: Vec<String> = d
                    .visible_choices(&vars)
                    .iter()
                    .map(|c| c.text.clone())
                    .collect();
                (
                    d.speaker.clone(),
                    d.visible_text().to_string(),
                    d.line_fully_revealed(),
                    choices,
                )
            })
            .collect();
        if items.is_empty() {
            return;
        }

        // 3. Render near the bottom of the screen.
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            let x0 = 60.0;
            for (speaker, body, full, choices) in items {
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
                if !choices.is_empty() {
                    // A decision is pending → numbered choice list (replaces the ▼ hint).
                    let mut y = vh - 86.0;
                    for (i, label) in choices.iter().enumerate() {
                        tq.push(DrawText::new(
                            format!("{}. {}", i + 1, label),
                            crate::Vec2::new(x0 + 16.0, y),
                            18.0,
                            crate::Color::rgb(0.85, 0.92, 1.0),
                        ));
                        y += 26.0;
                    }
                } else if full {
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

    // --- Phase 1: localization -------------------------------------------------------------

    const LOCALE_SAMPLE: &str = r#"
(
    default_locale: "en",
    locales: {
        "en": ( translations: {
            "npc.guide": "Guide",
            "intro.welcome": "Welcome, traveler.",
            "intro.continue": "Press space to continue.",
            "choice.left": "Go left",
            "choice.right": "Go right",
        } ),
        "ko": ( translations: {
            "npc.guide": "안내자",
            "intro.welcome": "어서 오세요, 여행자여.",
            "intro.continue": "계속하려면 스페이스를 누르세요.",
            "choice.left": "왼쪽으로",
            "choice.right": "오른쪽으로",
        } ),
    },
)
"#;

    #[test]
    fn localized_box_resolves_then_reresolves_on_locale_switch() {
        let mut locale = LocaleResource::from_ron_str(LOCALE_SAMPLE).unwrap();
        let mut d = DialogueBox::localized("npc.guide", ["intro.welcome", "intro.continue"]);
        // Keys are stored separately; display text starts empty until the first resolve.
        assert!(d.lines.is_empty());
        assert!(d.speaker.is_empty());

        d.resolve(&locale);
        assert_eq!(d.speaker, "Guide");
        assert_eq!(
            d.lines,
            vec!["Welcome, traveler.", "Press space to continue."]
        );

        assert!(locale.set_locale("ko"));
        d.resolve(&locale);
        assert_eq!(d.speaker, "안내자");
        assert_eq!(d.lines[0], "어서 오세요, 여행자여.");
    }

    #[test]
    fn resolve_is_noop_for_literal_box() {
        let locale = LocaleResource::from_ron_str(LOCALE_SAMPLE).unwrap();
        let mut d = DialogueBox::new("NPC", ["literal one", "literal two"]);
        d.resolve(&locale); // no line_keys → literal mode, unchanged
        assert_eq!(d.speaker, "NPC");
        assert_eq!(d.lines, vec!["literal one", "literal two"]);
    }

    #[test]
    fn resolve_preserves_typewriter_progress() {
        let locale = LocaleResource::from_ron_str(LOCALE_SAMPLE).unwrap();
        let mut d = DialogueBox::localized("npc.guide", ["intro.welcome", "intro.continue"])
            .with_chars_per_sec(10.0);
        d.resolve(&locale);
        d.tick(0.3); // 3 chars into "Welcome, traveler."
        assert_eq!(d.visible_text(), "Wel");
        assert!(!d.line_fully_revealed());

        d.resolve(&locale); // re-resolve must NOT touch current/elapsed/full
        assert_eq!(d.current, 0);
        assert_eq!(
            d.visible_text(),
            "Wel",
            "resolve must preserve reveal progress"
        );
        assert!(!d.line_fully_revealed());
    }

    #[test]
    fn serde_roundtrip_and_legacy_back_compat() {
        // A localized box round-trips its keys.
        let d = DialogueBox::localized("npc.guide", ["a.key", "b.key"]);
        let ron = ron::to_string(&d).unwrap();
        let back: DialogueBox = ron::from_str(&ron).unwrap();
        assert_eq!(back.line_keys, vec!["a.key", "b.key"]);
        assert_eq!(back.speaker_key.as_deref(), Some("npc.guide"));

        // A pre-localization scene (no line_keys/speaker_key) still loads via serde defaults.
        let legacy = r#"(speaker:"NPC",lines:["one","two"],current:0,chars_per_sec:28.0,elapsed:0.0,full:false)"#;
        let d2: DialogueBox = ron::from_str(legacy).unwrap();
        assert_eq!(d2.lines, vec!["one", "two"]);
        assert!(d2.line_keys.is_empty());
        assert_eq!(d2.speaker_key, None);
        assert!(d2.choices.is_empty());
    }

    // --- Phase 2: branching choices --------------------------------------------------------

    #[test]
    fn advance_is_blocked_while_choices_pending_then_choose_branches() {
        let mut d = DialogueBox::new("NPC", ["Pick a door.", "Left room.", "Right room."])
            .with_chars_per_sec(0.0) // instant reveal
            .with_choices(
                0,
                [
                    DialogueChoice::new("Left", 1),
                    DialogueChoice::new("Right", 2),
                ],
            );
        // Line 0 is instantly revealed and has choices → a decision is pending.
        assert_eq!(d.pending_choices().map(<[_]>::len), Some(2));

        // A plain advance must NOT skip the decision.
        d.advance();
        assert_eq!(d.current, 0, "advance is a no-op while choices are pending");

        // choose(0) jumps to the first choice's goto.
        d.choose(0);
        assert_eq!(d.current, 1);
        assert!(d.pending_choices().is_none());
    }

    #[test]
    fn choose_lands_on_distinct_targets() {
        let make = || {
            DialogueBox::new("NPC", ["Q", "A", "B"])
                .with_chars_per_sec(0.0)
                .with_choices(
                    0,
                    [DialogueChoice::new("a", 1), DialogueChoice::new("b", 2)],
                )
        };
        let mut left = make();
        left.choose(0);
        assert_eq!(left.current, 1);

        let mut right = make();
        right.choose(1);
        assert_eq!(right.current, 2);
    }

    #[test]
    fn out_of_range_goto_finishes_safely() {
        let mut d = DialogueBox::new("NPC", ["only"])
            .with_chars_per_sec(0.0)
            .with_choices(0, [DialogueChoice::new("leave", 99)]);
        d.choose(0); // goto 99 clamps to lines.len() (==1) → finished, no panic
        assert!(d.is_finished());
        assert_eq!(d.current, 1);
    }

    #[test]
    fn out_of_range_choice_index_is_noop() {
        let mut d = DialogueBox::new("NPC", ["Q", "A"])
            .with_chars_per_sec(0.0)
            .with_choices(0, [DialogueChoice::new("a", 1)]);
        d.choose(5); // no such choice → no-op
        assert_eq!(d.current, 0);
        assert!(d.pending_choices().is_some());
    }

    #[test]
    fn first_advance_completes_reveal_then_choices_appear() {
        let mut d = DialogueBox::new("NPC", ["Pick", "X", "Y"])
            .with_chars_per_sec(10.0)
            .with_choices(
                0,
                [DialogueChoice::new("x", 1), DialogueChoice::new("y", 2)],
            );
        d.tick(0.1); // mid-reveal → not fully shown yet, so no choices pending
        assert!(d.pending_choices().is_none());
        d.advance(); // first press completes the reveal
        assert!(d.line_fully_revealed());
        assert!(
            d.pending_choices().is_some(),
            "choices appear once the line is fully revealed"
        );
        d.advance(); // now a decision is pending → no-op
        assert_eq!(d.current, 0);
    }

    #[test]
    fn empty_choice_list_for_a_line_is_not_pending() {
        let mut d = DialogueBox::new("NPC", ["line"])
            .with_chars_per_sec(0.0)
            .with_choices(0, []); // explicitly empty → treated as no choices
        assert!(d.pending_choices().is_none());
        d.advance(); // behaves like a normal linear line
        assert!(d.is_finished());
    }

    #[test]
    fn localized_choices_resolve_text() {
        let mut locale = LocaleResource::from_ron_str(LOCALE_SAMPLE).unwrap();
        let mut d = DialogueBox::localized("npc.guide", ["intro.welcome"])
            .with_chars_per_sec(0.0)
            .with_choices(
                0,
                [
                    DialogueChoice::localized("choice.left", 0),
                    DialogueChoice::localized("choice.right", 0),
                ],
            );
        d.resolve(&locale);
        let cs = d.pending_choices().expect("choices pending after resolve");
        assert_eq!(cs[0].text, "Go left");
        assert_eq!(cs[1].text, "Go right");

        assert!(locale.set_locale("ko"));
        d.resolve(&locale);
        let cs = d.pending_choices().unwrap();
        assert_eq!(cs[0].text, "왼쪽으로");
        assert_eq!(cs[1].text, "오른쪽으로");
    }

    #[test]
    fn serde_roundtrip_with_choices() {
        let d = DialogueBox::new("NPC", ["Q", "A", "B"]).with_choices(
            0,
            [
                DialogueChoice::new("a", 1),
                DialogueChoice::localized("k", 2),
            ],
        );
        let ron = ron::to_string(&d).unwrap();
        let back: DialogueBox = ron::from_str(&ron).unwrap();
        assert_eq!(back.choices.len(), 1);
        assert_eq!(back.choices[0].0, 0);
        assert_eq!(back.choices[0].1[0], DialogueChoice::new("a", 1));
        assert_eq!(back.choices[0].1[1].goto, 2);
        assert_eq!(back.choices[0].1[1].key.as_deref(), Some("k"));
    }

    // --- P1 1b/1c: conditional choices + effect hooks --------------------------------------

    use crate::ecs::{Events, World};

    #[test]
    fn conditional_choice_hidden_until_var_set() {
        let mut d = DialogueBox::new("NPC", ["Pick", "secret", "normal"])
            .with_chars_per_sec(0.0)
            .with_choices(
                0,
                [
                    DialogueChoice::new("secret", 1).when(DialogueCond::new(
                        "vip",
                        DialogueOp::Eq,
                        DialogueValue::Bool(true),
                    )),
                    DialogueChoice::new("normal", 2),
                ],
            );
        let mut vars = DialogueVars::new();
        // vip unset → only the ungated choice is visible.
        assert_eq!(d.visible_choices(&vars).len(), 1);
        assert_eq!(d.visible_choices(&vars)[0].text, "normal");

        vars.set_bool("vip", true);
        assert_eq!(d.visible_choices(&vars).len(), 2);
        // visible index 0 is now the secret branch → jumps to line 1.
        assert!(d.choose_visible(0, &vars).is_none());
        assert_eq!(d.current, 1);
    }

    #[test]
    fn advance_with_skips_line_whose_choices_are_all_gated_out() {
        let mut d = DialogueBox::new("NPC", ["gate", "after"])
            .with_chars_per_sec(0.0)
            .with_choices(
                0,
                [DialogueChoice::new("locked", 1).when(DialogueCond::new(
                    "key",
                    DialogueOp::Eq,
                    DialogueValue::Bool(true),
                ))],
            );
        let vars = DialogueVars::new(); // key unset → the only choice is gated out
        assert!(!d.is_choosing(&vars));
        d.advance_with(&vars); // line is full (cps 0) → advances past it like a normal line
        assert_eq!(d.current, 1);
    }

    #[test]
    fn choose_visible_returns_effect() {
        let mut d = DialogueBox::new("NPC", ["q", "a"])
            .with_chars_per_sec(0.0)
            .with_choices(
                0,
                [DialogueChoice::new("take", 1).then(DialogueEffect::SetVar {
                    key: "got".into(),
                    value: DialogueValue::Bool(true),
                })],
            );
        let vars = DialogueVars::new();
        let effect = d.choose_visible(0, &vars).expect("effect returned");
        assert_eq!(
            effect,
            DialogueEffect::SetVar {
                key: "got".into(),
                value: DialogueValue::Bool(true)
            }
        );
        assert_eq!(d.current, 1);
    }

    #[test]
    fn world_choose_applies_setvar() {
        let mut world = World::new();
        world.insert_resource(Events::<DialogueEvent>::default());
        let e = world.spawn();
        world.add_component(
            e,
            DialogueBox::new("NPC", ["q", "buy", "leave"])
                .with_chars_per_sec(0.0)
                .with_choices(
                    0,
                    [DialogueChoice::new("buy", 1).then(DialogueEffect::SetVar {
                        key: "bought".into(),
                        value: DialogueValue::Bool(true),
                    })],
                ),
        );
        super::choose(&mut world, e, 0);
        assert_eq!(world.get::<DialogueBox>(e).unwrap().current, 1);
        assert_eq!(
            world.resource::<DialogueVars>().unwrap().get_bool("bought"),
            Some(true)
        );
    }

    #[test]
    fn world_choose_emits_event() {
        let mut world = World::new();
        world.insert_resource(Events::<DialogueEvent>::default());
        let e = world.spawn();
        world.add_component(
            e,
            DialogueBox::new("NPC", ["q", "x"])
                .with_chars_per_sec(0.0)
                .with_choices(
                    0,
                    [DialogueChoice::new("go", 1)
                        .then(DialogueEffect::EmitEvent { name: "door".into() })],
                ),
        );
        super::choose(&mut world, e, 0);
        let evs = world.resource::<Events<DialogueEvent>>().unwrap().read();
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].name, "door");
        assert_eq!(evs[0].entity, e);
    }

    #[test]
    fn world_advance_blocks_on_visible_choice() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            DialogueBox::new("NPC", ["pick", "a", "b"])
                .with_chars_per_sec(0.0)
                .with_choices(
                    0,
                    [
                        DialogueChoice::new("a", 1),
                        DialogueChoice::new("b", 2),
                    ],
                ),
        );
        super::advance(&mut world, e); // a decision is pending → no-op
        assert_eq!(world.get::<DialogueBox>(e).unwrap().current, 0);
    }
}
