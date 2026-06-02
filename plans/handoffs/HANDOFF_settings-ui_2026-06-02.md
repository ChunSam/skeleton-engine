# Settings + Dialogue UI example + `LocalizedText` (v1.2.0)

**Date:** 2026-06-02
**Status:** IMPLEMENTED — pending PR + user interactive confirmation
**Epic:** skeleton-engine playable-examples dogfooding loop
**Chain:** `settings-ui` seq `1`
**Branch:** `feat/example-settings-ui`

## The Goal

Close the densest "shipped but never played" gap (UI depth + localization + audio buses) with one
new playable example, and apply the single additive engine fix the example surfaces — per the
vision's dogfooding core loop. Direction + all scope decisions were locked via a grill session
(plan: `~/.claude/plans/mossy-gathering-perlis.md`).

## What Shipped

- **`examples/games/settings_menu/settings_menu.rs`** (`settings_menu_game`) — Title → Settings →
  Dialogue. First playable-game use of `TextInput`, `Slider`×2, `CheckBox`×2, `ScrollView`,
  `Panel`/`LayoutSystem`, rich/multiline `Label`, `LocaleResource` (EN/KO/ES), `AudioManager`
  buses (`music`/`sfx`) + `AudioEffect` low-pass (`M` muffles). `Settings` + locale + `AudioManager`
  persist across `SceneCmd::Replace` via `App::register_persistent`. Registered in `Cargo.toml`.
- **Engine fix — `src/ui/localized.rs`:** `LocalizedText { key }` component + `LocalizationSystem`
  resolving `LocaleResource::t(key)` into `Label.text` / `Button.label` / `CheckBox.label` each
  frame. Re-exported in `src/ui/mod.rs` + `src/lib.rs`. 3 unit tests. One `set_locale` call now
  retranslates the whole UI — no manual per-widget rebuild.
- **Version:** `Cargo.toml` 1.1.0 → **1.2.0** (Cargo.lock refreshed). CHANGELOG 1.2.0 section,
  NEXT_WORK coverage-follow-up row, HANDOFF.md session entry, CLAUDE.md version + module-map row.

## Key Decisions (locked in grill)

- Add `LocalizedText` (targets Label+Button+CheckBox) as additive minor API — over document-only.
- Full 3-scene example (vs Settings-only).
- System-font fallback, locales EN/KO/ES, no bundled font.
- Audio = bus volume + `AudioEffect` low-pass (positional audio judged contrived for a menu).
- Ship via feature branch + PR + v1.2.0 (mirrors the #3 merge).

## Documented Gaps (surfaced, NOT fixed — out of scope)

- **Runtime per-locale font switching unsupported:** `TextRenderer` font is fixed at init via the
  `FontData` resource (`src/app.rs:2923`); `LocaleData.font` is dead. Non-Latin renders only via
  native system-font fallback → Korean shows on macOS but **not on Linux CI / wasm** (no system
  fonts). This is expected; the example degrades to glyph boxes there, builds stay green.
- **RTL dead metadata:** `LocaleData.direction` / `TextDirection::RightToLeft` is not wired into the
  renderer (only `TextAlign::Right` is mapped, `src/renderer/text.rs:75`). No RTL locale ships, so
  wiring it would add unexercised code — deferred to a future RTL-focused example.

## Verification

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --lib --example settings_menu_game -- -D warnings` | 0 warnings |
| `cargo test --lib` | 256 passed (253 + 3 new) |
| `cargo build --target wasm32-unknown-unknown --lib` and `--example settings_menu_game` | build |
| `rust-survivors` rebuild | clean (additive) |
| Native interactive run | **left for user** (GUI not observable here) |

## QA fix rounds (engine bugs the example surfaced)

Interactive QA on macOS drove three rounds of engine fixes (all on this branch, all in CHANGELOG
1.2.0 **Fixed**):

- **Round 1 — HiDPI cursor offset.** Cursor was stored in physical px while UI/`screen_to_world`
  use logical px → clicks landed offset on Retina. Divide `CursorMoved`/touch by scale factor.
- **Round 2 — responsiveness + text caret nav.** Set `ControlFlow::Poll`; add `TextInput`
  arrow/Home/End/Delete cursor movement; example polish (name caret at end, audible volume, scroll).
- **Round 3 — four confirmed root causes:**
  - Caret rendered at end of string (ignored `ti.cursor`) → `TextInput::display_with_caret` inserts
    the caret (and IME preedit) at the real byte cursor; unit-tested.
  - Korean typed as separated jamo → `window.set_ime_allowed(true)` in `finish_init` (the
    `Ime::Preedit/Commit` handlers already existed).
  - Volume sliders / M-muffle inaudible → `AudioManager::play_tone` now applies `effective_volume`
    + channel `AudioEffect` like `play_internal`.
  - First click after a mouse move dropped (stale cursor) → moved `update()` from `RedrawRequested`
    into `about_to_wait` so input is fully drained before systems read it; `render()` stays in
    `RedrawRequested`.
  - Fullscreen checkbox relabeled "(preference)" — no OS fullscreen path yet (documented gap).

Round-3 verification: fmt clean; clippy `-D warnings` clean; `cargo test --lib` = **258 passed**;
wasm lib + example build; `rust-survivors` rebuilds clean. **Residual to re-evaluate with user:** if
a half-beat input latency remains it is AutoVsync buffering (vsync-disable deferred by user).

## Where We're Going

- Open PR on `feat/example-settings-ui`; merge after CI green (WASM / Package dry-run / Rustdoc /
  Test native), mirroring #3. User to confirm the interactive run (caret tracks cursor; Korean
  composes; sliders change loudness; M muffles; first click after a move registers).
- Remaining never-in-a-game subsystems for later cycles (none scheduled): 2D lighting, `BlendTree1D`,
  `Timeline`/cutscene, `PostProcessConfig`, physics joints, `RenderTarget` in real play, networking.
