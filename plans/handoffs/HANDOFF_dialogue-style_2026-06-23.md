# DialogueStyle — restyle DialogueSystem without forking (v0.63.0)

**Date:** 2026-06-23
**Status:** COMPLETED
**Bead(s):** none (engine work tracked via the dungeon-merchant wishlist board + the untracked code-quality scan)
**Epic:** Code-quality findings backlog (`docs/CODE_QUALITY_FINDINGS_2026-06-23.md`)
**Chain:** `codequality-backlog` seq `2` (Task 2 of the user-ordered 1→2→3 run)
**Parent:** seq 1 = `HANDOFF_gpu-particles-hdr_2026-06-23.md` (GPU particles under HDR, v0.62.2)
**Next:** Task 3 = egui submission dedup (`frame.rs` + `docked.rs` → `egui_pass.rs`)

---

## The Goal

The scan's **P2 dialogue-style** finding: `DialogueSystem` embedded its presentation (layout positions, font sizes, colors for speaker / body / choices / hint / portrait, plus the no-`ViewportSize` fallback) as inline constants, so a game had to edit engine source to make a normal style change — conflicting with the fork-friendly goal. This task makes the style a game-overridable resource while keeping the default look byte-identical.

## Where We Are

- **main @ `caf4a0a`, package v0.63.0, CLAUDE.md header v1.6.134, clean + green.** PR #222 merged (squash).
- **New `DialogueStyle` resource** (`src/dialogue/mod.rs`, re-exported from `engine`): `fallback_viewport`, portrait `size`/`x`/`bottom_offset`/`text_gap`, `text_margin`, and per-element (speaker/body/choice/hint) `bottom_offset` + `font_size` + `color`, plus `body_height` (wrap bounds), `choice_indent`/`choice_line_step`, and `hint_label`/`hint_right_offset`. `#[derive(Clone, Debug, Serialize, Deserialize)]` with a manual `Default` whose values **equal the original literals**.
- **Vertical positions are offsets UP from the viewport bottom** (the original layout was `vh - N`), so the box stays anchored to the bottom on a taller window. The advance-hint x is an offset LEFT from the viewport right edge.
- **`DialogueSystem::run` reads the style** via `world.resource::<DialogueStyle>().cloned().unwrap_or_default()` (cloned once — it owns the `hint_label` String — to release the resource borrow before the queries), and draws from it instead of inline constants. **Absent the resource → `default()` → byte-identical to before.**
- **Example `dialogue_style`** (flat): the same `DialogueBox` drawn with a custom style vs. the default; `T` toggles the `DialogueStyle` resource on/off (`insert_resource` / `remove_resource`) live.
- Verify gate green (`VERIFY_EXIT=0`); CI #222 4/4 green.
- **2 new integration tests** + a **native smoke** (text-render path) — see Evidence.

## Public API surface (this change)

```rust
// Opt-in resource; insert to restyle, omit for the default look.
pub struct DialogueStyle {
    pub fallback_viewport: Vec2,
    pub portrait_size: f32, pub portrait_x: f32,
    pub portrait_bottom_offset: f32, pub portrait_text_gap: f32,
    pub text_margin: f32,
    pub speaker_bottom_offset: f32, pub speaker_font_size: f32, pub speaker_color: Color,
    pub body_bottom_offset: f32, pub body_font_size: f32, pub body_color: Color, pub body_height: f32,
    pub choice_bottom_offset: f32, pub choice_indent: f32,
    pub choice_font_size: f32, pub choice_color: Color, pub choice_line_step: f32,
    pub hint_right_offset: f32, pub hint_bottom_offset: f32,
    pub hint_font_size: f32, pub hint_color: Color, pub hint_label: String,
}
// Default == the previous hardcoded look. Re-exported as engine::DialogueStyle.
```

Additive feature → **MINOR** (v0.63.0).

## Key Decisions

- **Opt-in resource read via `unwrap_or_default()`, NOT auto-inserted.** A game that never touches it gets the exact previous look (the OFF path is byte-identical); a game that inserts a customized one restyles. No auto-insert, no breaking, no migration.
- **Offsets-from-bottom, not absolute y.** The originals were all `vh - N`, so the style stores `N` (the bottom offset). This keeps the box bottom-anchored and matches the prior behavior exactly. Same for the hint's right-offset.
- **Clone the style once per frame** (like the existing `DialogueVars` clone) rather than threading a borrow — it owns one `String` (`hint_label`); the alloc only happens while a dialogue box is active, negligible.
- **`hint_label` is a `String`, not `&'static str`** — a game can localize / change the advance hint, which `&'static str` would prevent.
- **A dedicated `dialogue_style` example with a live `T` toggle**, rather than restyling an existing demo — the toggle makes "default vs. custom" a single visible comparison and leaves the other dialogue examples unchanged.
- **Tested via `TextQueue::iter()`** — the integration tests run `DialogueSystem` and assert the emitted `DrawText` positions/sizes/colors, locking in both "default == original literals" and "custom overrides apply".

## Evidence & Data

| Hash | PR | Version | Summary |
|---|---|---|---|
| `caf4a0a` | #222 | v0.63.0 | `DialogueStyle` resource |

**Tests** (`src/dialogue/tests.rs`):
- `dialogue_system_default_style_matches_original_literals` — no `DialogueStyle`; speaker at `(60, 450)` size 22 gold, body at `(60, 482)` size 20 white (viewport 900×600 → `vh-150`, `vh-118`, `text_margin` 60).
- `dialogue_system_custom_style_overrides` — `DialogueStyle { text_margin: 110, speaker_font_size: 28, speaker_color: cyan, ..default() }` → speaker x 110, size 28, cyan.

**Native smoke** (`/tmp/dlg_custom.png`, `/tmp/dlg_default.png`): `dialogue_style` with the custom style → cyan "Stylist" (font 28), wider margin, orange "▶ SPACE" hint; `T` (key code 17) → reverts to default → gold "Stylist" (font 22), original margin, blue-gray "▼ space" hint. Clean log.

**Verify gate:** `VERIFY_EXIT=0` (fmt / clippy `--all-targets -D warnings` / wasm lib+bins build / `test --all-targets` / rustdoc). CI #222 4/4 green.

## Files Changed

- `src/dialogue/mod.rs` — `DialogueStyle` struct + `Default`; `DialogueSystem::run` reads it (style-driven text_x closure, portrait, speaker, body, choices, hint).
- `src/dialogue/tests.rs` — 2 integration tests (default-matches-literals + custom-overrides).
- `src/lib.rs` — re-export `DialogueStyle`.
- `examples/dialogue_style.rs` — new flat example (T toggles the resource).
- `CLAUDE.md` — dialogue module-map row (styling + the example); header v1.6.134.
- `Cargo.toml` / `Cargo.lock` (0.63.0), `docs/CHANGELOG.md` (0.63.0 entry).

## Risks & Blockers

- **CI is ubuntu-only** — the text-render layout is macOS-verified manually + by the `TextQueue` integration tests (which don't need a GPU). The visual smoke confirmed the actual glyph render.
- The style covers the **text + portrait** layout, not a background panel — `DialogueSystem` deliberately draws no box background (the game supplies box art), unchanged here.

## Reusable Gotchas & Recipes

- **`TextQueue::iter()` exposes `&DrawText` with public `text`/`position`/`size`/`color`** — so a render-emitting system can be unit-tested by running it against a `World` with `ViewportSize` + `TextQueue` and asserting the queued draws, no GPU needed. Used for both DialogueStyle tests.
- **macOS synthetic key** for the smoke: `osascript ... set frontmost of process "<bin>" to true` then `key code 17` (T). Window opened at ~(510,135); read back with `get position of window 1`, `screencapture -R`.
- **Style-as-resource pattern** (now: `FocusRingStyle`, `DialogueStyle`): a `#[derive(Default)]`-or-manual-`Default` resource whose default equals the prior literals, read via `unwrap_or_default()`. Byte-identical OFF path, additive, fork-friendly. The reusable recipe for "this thing was hardcoded, make it configurable without breaking".

## Quick Start for Next Session (Task 3)

```bash
git -C /Users/jkl/Projects/skeleton-engine log --oneline -3   # tip caf4a0a (v0.63.0)
# Task 3 = egui submission dedup. The two near-identical blocks:
#   src/app/render/frame.rs  (present_egui, ~L21-59) — has the paint-callback guard
#   src/app/render/docked.rs (~L58-86)               — no callback guard
# Plan: extract submit_egui(render, gpu, view, guard_callbacks) into src/app/egui_pass.rs;
#   make egui_render_pass + paint_jobs_contain_callbacks private (only submit_egui uses them);
#   drop the now-unused imports from frame.rs/docked.rs. Behavior-preserving (guard_callbacks=true
#   for present_egui, false for docked). RenderState = crate::app::render_state::RenderState,
#   GpuContext = crate::renderer::GpuContext.
```
