# Dialogue depth — localization + branching choices (next epic)

**Date:** 2026-06-17
**Status:** PLANNED
**Bead(s):** none (beads unavailable in this environment)
**Epic:** Narrative dialogue — make the Phase-4 `DialogueBox` production-grade
**Chain:** `engine-hardening` seq `20` (paired with `HANDOFF_engine-hardening_gpu-ron-particle-depth_2026-06-17.md`)
**Context:** See that handoff for this session's data. This plan defines the NEXT session's work.

> **Note on epic choice:** "DialogueBox localization keys" is a named open follow-up (seq-19), and the
> recently-shipped `DialogueBox` (v0.14.0) is linear-only — branching choices are the obvious next need.
> Chosen over the other open follow-ups (wasm AEAD save, fuller wasm audio, release tags, crates.io) as
> the most VISION-aligned (a feature with a clear playable example) and self-contained. **If you'd rather
> target a different follow-up, swap this plan — the handoff data stands regardless.**

---

## Problem Statement

`DialogueBox` (`src/dialogue.rs`, v0.14.0) is a linear typewriter: a `Vec<String>` of literal lines,
advanced one at a time. Two gaps keep it from real RPG/visual-novel use: (1) **no localization** — lines
are hard-coded strings, so a game can't drive them from the existing `LocaleResource`/`t(key)` i18n that
the rest of the UI uses; (2) **no branching** — every conversation is a straight line, with no player
choices that change the path. Both are additive extensions to the existing component + `DialogueSystem`.
See the handoff's "Phase-6 baseline" + this session's reuse pattern for how the engine adds such mirrors
without breaking existing call sites.

## Key Findings (driving this plan)

- `DialogueBox` fields: `speaker: String`, `lines: Vec<String>`, `current: usize`, `chars_per_sec`,
  private `elapsed`/`full`, `portrait`. Serde-derived. Built via `DialogueBox::new(speaker, lines)`.
  → Phase 1 stores keys alongside `lines`; Phase 2 adds choices keyed by line index.
- `DialogueSystem` (a unit struct) ticks every box's typewriter via `query_mut`, then renders the active
  box's speaker/body/▼-hint as **screen-space `TextQueue` text** near the bottom; it is **input-agnostic**
  (the game calls `advance()`). → both phases extend the same render+tick loop; Phase 2 adds a choice
  render branch and a `choose(i)` call the game invokes.
- Localization already exists: `LocaleResource::new(default)` / `set_locale(loc)` / `t(key) -> &str`
  (`src/locale.rs`), and `LocalizedText`/`LocalizationSystem` resolve keys into UI widgets each frame
  (`src/ui/localized.rs`). → Phase 1 resolves dialogue keys the same way, per-frame, so live locale
  switches work and no new i18n machinery is needed.
- `advance()` semantics: 1st press completes the reveal, 2nd moves to the next line / finishes. The
  typewriter state lives in `current`/`elapsed`; re-resolving `lines` text from keys does NOT touch those
  indices → safe to re-resolve every frame. → Phase 1 is low-risk.
- `EmitShape`/`EmitShapeDef` mirror + `with_*` builder pattern from THIS session is the template for
  additive, default-preserving fields → reuse it for `choices`/key fields (all `#[serde(default)]`).

## Anti-Goals (do NOT do)

- **Do NOT store locale KEYS in the `lines` field.** The typewriter reveals `lines[current]` char-by-char;
  if that holds a key the player sees the key animate. Keep `lines` = resolved DISPLAY text; store keys
  separately and resolve into `lines`.
- **Do NOT make `DialogueSystem` read input or own the locale.** It stays input-agnostic — the game calls
  `advance()`/`choose(i)` and owns the `LocaleResource`. The system only *reads* the resource to resolve.
- **Do NOT build a scripting DSL / RON dialogue-tree loader this epic.** Keep branching in-code
  (`Vec<DialogueChoice>` with `goto` indices). A data-driven `.dlg` format is a later epic.
- **Do NOT reset typewriter state on locale change or resolve** (would restart the line every frame).

## Plan

### Phase 1: Localized dialogue lines

**Goal:** A `DialogueBox` can be built from locale keys and renders the active locale's text, live-updating
on `set_locale`.

**Why this approach:** Reuses the existing `LocaleResource::t` path (same as `LocalizedText`), so it is
purely additive and needs no new i18n infra; per-frame re-resolve is cheap and gives free live switching.

- Add `pub line_keys: Vec<String>` + `pub speaker_key: Option<String>` to `DialogueBox` (both
  `#[serde(default)]`; empty/None = literal mode = byte-identical prior behavior).
- Add constructor `DialogueBox::localized(speaker_key: impl Into<String>, line_keys: IntoIterator<Item=S>)`
  — fills `line_keys`/`speaker_key`, leaves `lines` empty (resolved on first tick), sets a sane default
  `chars_per_sec`.
- Add `fn resolve(&mut self, locale: &LocaleResource)`: if `!line_keys.is_empty()`, set
  `self.lines = line_keys.iter().map(|k| locale.t(k).to_string()).collect()` and
  `self.speaker = speaker_key.as_ref().map(|k| locale.t(k).to_string()).unwrap_or_default()`. Must NOT
  touch `current`/`elapsed`/`full`.
- In `DialogueSystem::run`, BEFORE the tick loop: if `world.resource::<LocaleResource>()` exists, call
  `resolve` on each box (collect the resource read first to avoid borrow conflict with `query_mut` — see
  the existing "gather then mutate" shape in the same file).
- Edge cases: missing key → `t` already returns the key itself (LocaleResource behavior — confirm) so it
  degrades visibly, not a panic; a line longer in another locale mid-reveal just reveals more chars (fine;
  RTL/multi-script already handled by the text renderer).

**Files:** `src/dialogue.rs` (fields, `localized`, `resolve`, system resolve-step), `src/lib.rs` (no new
export needed — `DialogueBox` already public).
**Validates with:** unit tests — `localized` box resolves "en" then `set_locale("ko")` re-resolves;
literal box (no keys) is unchanged; `resolve` preserves `current`/typewriter progress. `cargo test --lib dialogue`.
**Rollback:** the two fields + `resolve` are additive; delete them and the literal path is untouched.

### Phase 2: Branching choices

**Goal:** A line can present 2+ choices; selecting one jumps the conversation to a target line.

**Why this approach:** Keeps `DialogueSystem` input-agnostic (game calls `choose`), mirrors `advance`'s
two-stage feel, and uses plain `goto` indices (no DSL) so it stays in-code and testable.

- Add `pub struct DialogueChoice { pub text: String, pub key: Option<String>, pub goto: usize }`
  (serde-derived; `key` localizes like Phase 1, `goto` = target line index).
- Add `pub choices: Vec<(usize, Vec<DialogueChoice>)>` to `DialogueBox` (`#[serde(default)]`) — choices
  shown when line `usize` is fully revealed. Add `fn pending_choices(&self) -> Option<&[DialogueChoice]>`
  (Some when `current` has choices AND `line_fully_revealed()`).
- Add `fn choose(&mut self, i: usize)`: if `pending_choices` is Some and `i` in range, set
  `current = choices[i].goto`, reset `elapsed`/`full`; else no-op.
- In `advance()`: if `pending_choices().is_some()`, do NOTHING (force the game to `choose`) — so a plain
  advance can't skip a decision.
- In `DialogueSystem::run` render step: when `pending_choices` is Some, render the choices as a numbered
  list (resolve `key` via `LocaleResource` if present, like Phase 1) instead of the ▼ hint.
- Edge: `goto` out of range → clamp to `lines.len()` (finishes the conversation) rather than panic; an
  empty choices vec for a line is treated as no choices.

**Files:** `src/dialogue.rs` (struct, field, methods, render branch), `src/lib.rs` (`pub use … DialogueChoice`).
**Validates with:** unit tests — a 2-choice line: `advance` is a no-op while choices pending; `choose(0)`
vs `choose(1)` land on different `goto`; out-of-range `goto` finishes safely. `cargo test --lib dialogue`.
**Rollback:** all additive; remove `choices`/`DialogueChoice` and linear dialogue is unchanged.

### Phase 3: Playable example (the acceptance test)

**Goal:** Prove both phases in real play — the VISION rule (a feature isn't done without a playable example).

**Why this approach:** The engine's acceptance criterion; surfaces API awkwardness before release.

- New `examples/dialogue_branching.rs` (or extend `dialogue_demo`): a short branching, **localized**
  conversation — a key (e.g. number keys `1`/`2`) selects choices, `Space` advances, a key (e.g. `L`)
  toggles `LocaleResource` between "en" and "ko" live, `Esc` quits.
- Provide a tiny in-code `LocaleResource` with en + ko strings (mirror the existing locale test fixtures —
  Korean DATA is allowed as fixtures, see the repo's i18n examples).
- At least one choice must change which subsequent lines show (visible branch). Show the ▼ hint vs the
  numbered choice list correctly.
- Playtest it (windowed): per the handoff's gotchas, dialogue renders via `TextQueue`/`DrawText` so a
  static screencapture is sufficient (no GPU/run-loop sparsity issue like the particle playtest). Verify
  typewriter, choice selection branches, and the live `L` locale toggle.

**Files:** `examples/dialogue_branching.rs` (new); maybe a `_keys.ron`/in-code locale table.
**Validates with:** `cargo build --example dialogue_branching` + a windowed playtest screenshot showing a
choice list and (after toggling) the ko text. Then full `./scripts/verify.sh`.
**Rollback:** delete the example; engine code from Phases 1–2 stands alone.

## Dependencies & Order

- Phase 1 → Phase 2 → Phase 3 are **sequential** (Phase 2's choice render reuses Phase 1's key-resolve;
  Phase 3 exercises both). Not parallelizable.
- Each phase is its own PR + CHANGELOG entry (one MINOR bump per phase, or batch as one `0.19.0` — decide
  at ship time per the pre-1.0 cadence). main is branch-protected → PR-only, squash-merge; re-confirm
  merge authority at session start.

## Risks & Mitigations

- **Borrow conflict resolving locale during `query_mut`** (likely — same file already works around it):
  read `LocaleResource` / collect entities first, then mutate. Mitigation: follow the existing
  "gather then mutate" shape in `DialogueSystem::run`.
- **Serde back-compat for existing scenes** (low — fields are `#[serde(default)]`): old `DialogueBox` RON
  with no `line_keys`/`choices` must still load. Mitigation: a round-trip serde unit test.
- **API awkwardness surfaces in Phase 3** (medium — that's the point): if `choose`/`pending_choices` feels
  wrong while writing the example, fix the API before shipping (VISION core loop).
- **Locale `t` on a missing key** — confirm it returns the key (not panic/empty) before relying on it.

## Success Criteria

- **Minimum:** Phases 1–2 merged, additive, all existing dialogue tests + new tests green; literal
  `DialogueBox` behavior byte-identical (a no-keys/no-choices box renders exactly as today).
- **Full:** `dialogue_branching` example playtested — typewriter + a working branch + live en↔ko toggle,
  with `./scripts/verify.sh` green and a version bump shipped via `/ship`.
- Baseline to beat: current `dialogue` unit tests pass (`cargo test --lib dialogue`); 789 lib tests total.

## Quick Start

```bash
cd /Users/jkl/Projects/skeleton-engine
cat plans/handoffs/HANDOFF_engine-hardening_gpu-ron-particle-depth_2026-06-17.md   # session data
# /memory-recall dialogue localization   # if OV available

# Key source files for Phase 1
sed -n '1,220p' src/dialogue.rs           # DialogueBox + DialogueSystem
sed -n '1,130p' src/locale.rs             # LocaleResource::t / set_locale
sed -n '1,90p'  src/ui/localized.rs       # the resolve-each-frame pattern to mirror
cat examples/dialogue_demo.rs             # the existing linear example to extend

# Verify starting state
./scripts/verify.sh                       # green (789 lib tests) before changing anything

# First concrete action (Phase 1)
# In src/dialogue.rs: add `line_keys: Vec<String>` + `speaker_key: Option<String>`
# (both #[serde(default)]) to DialogueBox, a `localized(..)` ctor, and a `resolve(&LocaleResource)`
# that fills `lines`/`speaker` WITHOUT touching `current`/`elapsed`.
```
