//! The dialogue data model: [`DialogueChoice`] (a branching option) and [`DialogueBox`] (a
//! speaker + typewriter text box), plus the world-level [`advance`] / [`choose`] control fns
//! that operate on a box and honor conditional choices.

use serde::{Deserialize, Serialize};

use crate::asset::{Handle, ImageAsset};
use crate::ecs::{Entity, Events, World};
use crate::locale::LocaleResource;

use super::{DialogueCond, DialogueEffect, DialogueEvent, DialogueVars};

/// One branching choice presented at a [`DialogueBox`] line: a label plus the line index the
/// conversation jumps to when the player selects it.
///
/// `text` is the display label; `key` is an optional localization key — when set,
/// [`DialogueBox::resolve`] fills `text` from the active [`LocaleResource`] (same as line keys).
/// `goto` is the target line index ([`DialogueBox::choose`] clamps an out-of-range value to the
/// end, finishing the conversation, rather than panicking).
///
/// # Gating
///
/// Three independent gate fields, ANDed together — the choice is shown when **all** of these
/// hold (see [`DialogueBox::visible_choices`]):
///
/// | Field | Passes when |
/// |---|---|
/// | [`cond`](Self::cond) | it is `None`, or its single condition evaluates true |
/// | [`cond_all`](Self::cond_all) | **every** listed condition evaluates true (empty = passes) |
/// | [`cond_any`](Self::cond_any) | **at least one** listed condition evaluates true (empty = passes) |
///
/// `cond` is the one-term sugar: it predates the other two and is exactly `cond_all` with a
/// single element. Ordinary two-term gates go in `cond_all` (`gold >= 10 && !has_lantern`) and
/// their negations in `cond_any` (`gold < 10 || has_lantern`); combining the fields expresses
/// a conjunction of a disjunction without nesting.
///
/// Why three flat fields rather than `All`/`Any` variants on [`DialogueCond`]: the conditions
/// are authored in RON, and **RON 0.8 cannot express that shape without breaking every existing
/// file**. An externally tagged enum turns today's `cond: (var: "gold", …)` into
/// `cond: Cmp((var: "gold", …))`, taxing the common single-term case to serve the rare compound
/// one; `#[serde(untagged)]` — the usual escape hatch — does not work at all here, and cannot
/// even round-trip its own output through RON. Measured, not assumed. Flat fields keep every
/// `.dlg.ron` written before this feature parsing byte-for-byte unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueChoice {
    /// Display label for this choice (resolved from `key` when localized).
    pub text: String,
    /// Optional localization key; when set, [`DialogueBox::resolve`] fills `text` from the locale.
    #[serde(default)]
    pub key: Option<String>,
    /// Line index the conversation jumps to when this choice is selected.
    pub goto: usize,
    /// Optional single-term gate: the choice is only shown when this condition passes against
    /// [`DialogueVars`] (see [`DialogueBox::visible_choices`]). `None` = this gate passes.
    #[serde(default)]
    pub cond: Option<DialogueCond>,
    /// Conjunctive gate: **every** condition here must pass. Empty = this gate passes.
    #[serde(default)]
    pub cond_all: Vec<DialogueCond>,
    /// Disjunctive gate: **at least one** condition here must pass. Empty = this gate passes.
    #[serde(default)]
    pub cond_any: Vec<DialogueCond>,
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
            cond_all: Vec::new(),
            cond_any: Vec::new(),
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
            cond_all: Vec::new(),
            cond_any: Vec::new(),
            effect: None,
        }
    }

    /// Gates this choice on `cond` (builder): it is only shown while the condition passes.
    pub fn when(mut self, cond: DialogueCond) -> Self {
        self.cond = Some(cond);
        self
    }

    /// Gates this choice on **every** condition in `conds` (builder) — a conjunction.
    ///
    /// ```
    /// # use engine::{DialogueChoice, DialogueCond, DialogueOp, DialogueValue, DialogueVars};
    /// // `gold >= 10 && !has_lantern` — the ordinary shop gate.
    /// let buy = DialogueChoice::new("Buy the lantern", 1).when_all([
    ///     DialogueCond::new("gold", DialogueOp::Ge, DialogueValue::Int(10)),
    ///     DialogueCond::new("has_lantern", DialogueOp::Eq, DialogueValue::Bool(false)),
    /// ]);
    /// let mut vars = DialogueVars::new();
    /// vars.set_int("gold", 10);
    /// assert!(!buy.is_unconditional());
    /// ```
    pub fn when_all(mut self, conds: impl IntoIterator<Item = DialogueCond>) -> Self {
        self.cond_all = conds.into_iter().collect();
        self
    }

    /// Gates this choice on **at least one** condition in `conds` (builder) — a disjunction.
    pub fn when_any(mut self, conds: impl IntoIterator<Item = DialogueCond>) -> Self {
        self.cond_any = conds.into_iter().collect();
        self
    }

    /// Attaches a side `effect` applied when this choice is taken (builder).
    pub fn then(mut self, effect: DialogueEffect) -> Self {
        self.effect = Some(effect);
        self
    }

    /// Whether this choice is currently available: every gate it carries passes.
    ///
    /// The three gates are ANDed — `cond` (single term), `cond_all` (conjunction) and `cond_any`
    /// (disjunction) — and an unset gate passes vacuously, so a choice with none is always
    /// available. See the table on [`DialogueChoice`].
    fn is_available(&self, vars: &DialogueVars) -> bool {
        self.cond.as_ref().is_none_or(|c| c.eval(vars))
            && self.cond_all.iter().all(|c| c.eval(vars))
            && (self.cond_any.is_empty() || self.cond_any.iter().any(|c| c.eval(vars)))
    }

    /// Whether this choice carries no gate at all and is therefore always shown — independent
    /// of any [`DialogueVars`]. Used by the plain (vars-unaware) API to avoid deadlocking on
    /// lines whose choices are entirely condition-gated.
    ///
    /// All three gate fields count: a choice gated only by `cond_all` / `cond_any` is *not*
    /// unconditional, and the vars-unaware API must not offer it.
    pub fn is_unconditional(&self) -> bool {
        self.cond.is_none() && self.cond_all.is_empty() && self.cond_any.is_empty()
    }
}

/// A dialogue / textbox: a sequence of lines, each revealed with a typewriter effect and advanced
/// one at a time. Attach it to an entity and add a [`DialogueSystem`](crate::DialogueSystem); call
/// [`advance`](Self::advance) from your own input handling to move through the conversation.
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
    /// resolved against a [`LocaleResource`] each frame by [`DialogueSystem`](crate::DialogueSystem)
    /// (or manually via [`resolve`](Self::resolve)). [`lines`](Self::lines)/[`speaker`](Self::speaker)
    /// start empty and are filled on the first resolve, so switching locale live retranslates the box.
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
    /// Whether this box has anything a locale could translate — any `line_keys`, or any choice
    /// carrying a `key`.
    ///
    /// Used by `DialogueSystem` to skip cloning the entire `LocaleResource` (which owns every
    /// translated string in the game) on frames where no box would use it.
    pub fn needs_locale_resolve(&self) -> bool {
        !self.line_keys.is_empty()
            || self
                .choices
                .iter()
                .any(|(_line, choices)| choices.iter().any(|c| c.key.is_some()))
    }

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

    /// Advances the typewriter by `dt`. Called by [`DialogueSystem`](crate::DialogueSystem); no
    /// effect once finished.
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
