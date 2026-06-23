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
use crate::renderer::{DrawImage, DrawText, TextQueue, UiImageQueue};
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

    /// Whether this choice has no condition (`cond` is `None`) and is therefore always
    /// shown — independent of any [`DialogueVars`]. Used by the plain (vars-unaware) API
    /// to avoid deadlocking on lines whose choices are entirely condition-gated.
    pub fn is_unconditional(&self) -> bool {
        self.cond.is_none()
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
        if self.full || !self.chars_per_sec.is_finite() || self.chars_per_sec <= 0.0 {
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
                    || !self.chars_per_sec.is_finite()
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

    /// All choices registered for the current line (regardless of conditions), or `None` when the
    /// line is not yet fully revealed, there are no choices, or the conversation is finished.
    /// Used internally by the vars-aware API which re-filters by `is_available`.
    fn pending_choices_raw(&self) -> Option<&[DialogueChoice]> {
        if self.is_finished() || !self.line_fully_revealed() {
            return None;
        }
        self.choices
            .iter()
            .find(|(line, cs)| *line == self.current && !cs.is_empty())
            .map(|(_, cs)| cs.as_slice())
    }

    /// The unconditional choices to present right now via the plain (vars-unaware) API, or `None`
    /// when none apply. `Some` only when the current line is fully revealed *and* has at least one
    /// choice with no [`cond`](DialogueChoice::cond) — i.e. the moment the player must pick an
    /// unconditional branch. Conditional-only choices are invisible to this method (so
    /// [`advance`](Self::advance) is not deadlocked by a line whose every choice is gated).
    ///
    /// Use [`visible_choices`](Self::visible_choices) / [`advance_with`](Self::advance_with) /
    /// [`dialogue::advance`](crate::dialogue::advance) when you need condition-aware filtering.
    pub fn pending_choices(&self) -> Option<Vec<&DialogueChoice>> {
        let cs = self.pending_choices_raw()?;
        let unconditional: Vec<&DialogueChoice> =
            cs.iter().filter(|c| c.is_unconditional()).collect();
        if unconditional.is_empty() {
            None
        } else {
            Some(unconditional)
        }
    }

    /// Selects unconditional pending choice `i`, jumping the conversation to that choice's `goto`
    /// line. A no-op when no unconditional choices are pending or `i` is out of range. An
    /// out-of-range `goto` is clamped to the end (finishing the conversation) rather than panicking.
    ///
    /// For condition-gated choices use [`choose_visible`](Self::choose_visible) /
    /// [`dialogue::choose`](crate::dialogue::choose).
    pub fn choose(&mut self, i: usize) {
        let goto = match self.pending_choices().and_then(|cs| cs.into_iter().nth(i)) {
            Some(choice) => choice.goto,
            None => return,
        };
        self.current = goto.min(self.lines.len());
        self.elapsed = 0.0;
        self.full = false;
    }

    /// The choices visible right now given `vars`: all choices for the current line
    /// whose [`cond`](DialogueChoice::cond) passes (or have none). Empty when no decision is
    /// pending. The index into this slice is what [`choose_visible`](Self::choose_visible) /
    /// [`dialogue::choose`](crate::dialogue::choose) expect.
    pub fn visible_choices(&self, vars: &DialogueVars) -> Vec<&DialogueChoice> {
        match self.pending_choices_raw() {
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

/// Visual style for [`DialogueSystem`]: the layout positions, font sizes, and colors of the
/// speaker, body, choice list, advance hint, and portrait. Insert a customized one as a resource to
/// restyle dialogue **without forking the engine** — absent, the system uses
/// [`DialogueStyle::default`], which reproduces the previous hardcoded look exactly, so existing
/// games are unchanged.
///
/// Vertical positions are **offsets up from the viewport bottom** (a taller window keeps the box
/// anchored to the bottom), matching the original `vh - N` layout. The advance-hint x is an offset
/// **left** from the viewport right edge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DialogueStyle {
    /// Fallback viewport `(width, height)` when no [`ViewportSize`] resource is present.
    pub fallback_viewport: crate::Vec2,
    /// Square speaker-portrait edge length (px).
    pub portrait_size: f32,
    /// Portrait left x (px).
    pub portrait_x: f32,
    /// Portrait top y, as an offset up from the viewport bottom (px).
    pub portrait_bottom_offset: f32,
    /// Gap between the portrait's right edge and the text column, when a portrait is shown (px).
    pub portrait_text_gap: f32,
    /// Left text margin when there is no portrait (px).
    pub text_margin: f32,
    /// Speaker label bottom offset (px up from bottom).
    pub speaker_bottom_offset: f32,
    /// Speaker label font size.
    pub speaker_font_size: f32,
    /// Speaker label color.
    pub speaker_color: crate::Color,
    /// Body-text bottom offset (px up from bottom).
    pub body_bottom_offset: f32,
    /// Body-text font size.
    pub body_font_size: f32,
    /// Body-text color.
    pub body_color: crate::Color,
    /// Body-text wrap-bounds height (px).
    pub body_height: f32,
    /// Choice-list first-row bottom offset (px up from bottom).
    pub choice_bottom_offset: f32,
    /// Choice-row x indent from the text column (px).
    pub choice_indent: f32,
    /// Choice-row font size.
    pub choice_font_size: f32,
    /// Choice-row color.
    pub choice_color: crate::Color,
    /// Per-choice-row vertical step (px).
    pub choice_line_step: f32,
    /// Advance-hint x as an offset left from the viewport right edge (px).
    pub hint_right_offset: f32,
    /// Advance-hint bottom offset (px up from bottom).
    pub hint_bottom_offset: f32,
    /// Advance-hint font size.
    pub hint_font_size: f32,
    /// Advance-hint color.
    pub hint_color: crate::Color,
    /// Advance-hint label (shown once a line is fully revealed and no choice is pending).
    pub hint_label: String,
}

impl Default for DialogueStyle {
    fn default() -> Self {
        Self {
            fallback_viewport: crate::Vec2::new(1280.0, 720.0),
            portrait_size: 96.0,
            portrait_x: 48.0,
            portrait_bottom_offset: 162.0,
            portrait_text_gap: 18.0,
            text_margin: 60.0,
            speaker_bottom_offset: 150.0,
            speaker_font_size: 22.0,
            speaker_color: crate::Color::rgb(1.0, 0.85, 0.35),
            body_bottom_offset: 118.0,
            body_font_size: 20.0,
            body_color: crate::Color::WHITE,
            body_height: 90.0,
            choice_bottom_offset: 86.0,
            choice_indent: 16.0,
            choice_font_size: 18.0,
            choice_color: crate::Color::rgb(0.85, 0.92, 1.0),
            choice_line_step: 26.0,
            hint_right_offset: 150.0,
            hint_bottom_offset: 42.0,
            hint_font_size: 16.0,
            hint_color: crate::Color::rgb(0.6, 0.7, 0.85),
            hint_label: "▼ space".to_string(),
        }
    }
}

/// What [`DialogueSystem`] gathers per active box before drawing — pulled out of the query borrow
/// so the text + image queues can be written afterward.
struct DrawItem {
    speaker: String,
    body: String,
    full: bool,
    choices: Vec<String>,
    portrait: Option<Handle<ImageAsset>>,
}

/// Ticks every [`DialogueBox`]'s typewriter and renders the active box (speaker + revealed text +
/// an advance hint) as screen-space text near the bottom of the viewport. When a box sets a
/// [`portrait`](DialogueBox::portrait) (e.g. via [`with_portrait`](DialogueBox::with_portrait)),
/// the speaker's image is drawn to the left through the UI image queue and the text shifts right
/// to clear it.
///
/// Input-agnostic: the game advances a box by calling [`DialogueBox::advance`]. Rendering is
/// text + optional portrait (no background panel) so it composes with whatever box art the game
/// draws.
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
        // Style is a game-overridable resource; absent, `default()` reproduces the original look.
        // Cloned once (it owns the hint label) to release the resource borrow before the queries.
        let style = world
            .resource::<DialogueStyle>()
            .cloned()
            .unwrap_or_default();
        let (vw, vh) = world
            .resource::<ViewportSize>()
            .map(|v| (v.width, v.height))
            .unwrap_or((style.fallback_viewport.x, style.fallback_viewport.y));
        // Clone DialogueVars (or empty) to filter conditional choices without holding a
        // resource borrow across the query below.
        let vars = world
            .resource::<DialogueVars>()
            .cloned()
            .unwrap_or_default();
        let items: Vec<DrawItem> = world
            .query::<DialogueBox>()
            .filter(|(_, d)| !d.is_finished())
            .map(|(_, d)| DrawItem {
                speaker: d.speaker.clone(),
                body: d.visible_text().to_string(),
                full: d.line_fully_revealed(),
                // Visible (cond-passing) choice labels, already resolved into `text` by step 0.
                choices: d
                    .visible_choices(&vars)
                    .iter()
                    .map(|c| c.text.clone())
                    .collect(),
                portrait: d.portrait.clone(),
            })
            .collect();
        if items.is_empty() {
            return;
        }

        // 3. Render near the bottom of the screen, driven by `style`.
        //
        // A box with a `portrait` draws a square image on the left and shifts its text right to
        // clear it; a box without one keeps the no-portrait left margin.
        let text_x = |has_portrait: bool| {
            if has_portrait {
                style.portrait_x + style.portrait_size + style.portrait_text_gap
            } else {
                style.text_margin
            }
        };

        // 3a. Portraits — screen-space images to the left of the text, drawn through the same UI
        //     image queue the renderer presents over the scene. Borrow `&items` here; the text
        //     loop below consumes `items`.
        if let Some(iq) = world.resource_mut::<UiImageQueue>() {
            for item in &items {
                if let Some(handle) = &item.portrait {
                    iq.push(DrawImage::with_handle(
                        style.portrait_x,
                        vh - style.portrait_bottom_offset,
                        style.portrait_size,
                        style.portrait_size,
                        handle.clone(),
                    ));
                }
            }
        }

        // 3b. Speaker + body + choices/hint text.
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            for item in items {
                let DrawItem {
                    speaker,
                    body,
                    full,
                    choices,
                    portrait,
                } = item;
                let x0 = text_x(portrait.is_some());
                if !speaker.is_empty() {
                    tq.push(DrawText::new(
                        speaker,
                        crate::Vec2::new(x0, vh - style.speaker_bottom_offset),
                        style.speaker_font_size,
                        style.speaker_color,
                    ));
                }
                tq.push(
                    DrawText::new(
                        body,
                        crate::Vec2::new(x0, vh - style.body_bottom_offset),
                        style.body_font_size,
                        style.body_color,
                    )
                    .with_bounds(crate::Vec2::new(vw - 2.0 * x0, style.body_height)),
                );
                if !choices.is_empty() {
                    // A decision is pending → numbered choice list (replaces the advance hint).
                    let mut y = vh - style.choice_bottom_offset;
                    for (i, label) in choices.iter().enumerate() {
                        tq.push(DrawText::new(
                            format!("{}. {}", i + 1, label),
                            crate::Vec2::new(x0 + style.choice_indent, y),
                            style.choice_font_size,
                            style.choice_color,
                        ));
                        y += style.choice_line_step;
                    }
                } else if full {
                    tq.push(DrawText::new(
                        style.hint_label.clone(),
                        crate::Vec2::new(
                            vw - style.hint_right_offset,
                            vh - style.hint_bottom_offset,
                        ),
                        style.hint_font_size,
                        style.hint_color,
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
mod tests;
