# Settings UI example — QA-driven engine UI/input/audio fixes (v1.2.0, ready to merge)

**Date:** 2026-06-02
**Status:** COMPLETE — PR #4 open, MERGEABLE/CLEAN; user asked to commit + merge
**Epic:** skeleton-engine playable-examples dogfooding loop
**Chain:** `settings-ui` seq `2`
**Parent:** `plans/handoffs/HANDOFF_settings-ui_2026-06-02.md` (seq 1 — full feature + round-by-round detail)
**Branch:** `feat/example-settings-ui` → base `main`
**PR:** #4 "feat(ui): LocalizedText + settings_menu_game example (v1.2.0)"

## Since Last Handoff (seq 1 → seq 2)

Seq 1 shipped the feature (`settings_menu_game` + `LocalizedText`) and bumped to v1.2.0, leaving
"interactive play left for user confirmation." This session **was that confirmation** — six rounds
of macOS interactive QA, each surfacing real **engine** bugs (the example was the magnifying glass).
All are fixed and committed; the parent file already has the round-by-round log appended. This seq-2
file is the **closing record**: final state, what was learned (so the next session doesn't redo the
latency hunt), and the deferred follow-ups.

## The Goal

Validate the UI-depth + localization + audio-bus cluster via a real playable example, fix every
engine issue it surfaces, and land it. Vision = dogfooding loop ("a feature isn't done until a small
playable example exercises it; fix the API if it feels wrong"). Direction/scope were grilled up front
(plan `~/.claude/plans/mossy-gathering-perlis.md`).

## Where We Are

- Working tree **clean**, on `feat/example-settings-ui`, pushed. **8 commits** ahead of `main`.
- **PR #4: OPEN, mergeable=MERGEABLE, mergeStateStatus=CLEAN.** Ready to merge.
- v1.2.0 in `Cargo.toml` + `Cargo.lock`. `cargo test --lib` = **260 passed**. fmt + clippy
  (`-D warnings`, native lib+example AND wasm) clean. `rust-survivors` rebuilds clean (additive).
- Example builds native + wasm (`settings_menu_game`).

### Commits on the branch (oldest → newest)

| Hash | Summary |
|---|---|
| `0270592` | feat(ui): LocalizedText + settings_menu_game example (v1.2.0) |
| `8cfe3c6` | fix(input): store cursor in logical pixels for HiDPI hit-testing |
| `fa5273b` | fix: continuous loop (ControlFlow::Poll) + TextInput arrow/Home/End/Delete + example polish |
| `1f48285` | fix: caret position, IME compose (set_ime_allowed), audio bus/effect on tones, click freshness |
| `1562645` | fix(ui): hit-test clicks at the press/release cursor; (interim) steady caret; audible muffle |
| `a8dc46a` | fix: lower frame latency (frame_latency=1); blinking non-shifting caret; correct muffle wording |
| `7de8f24` | fix: remove frame-gap latency (revert update→about_to_wait); audible low-pass demo |
| `adba82a` | perf: keep AutoVsync default; document deferred macOS latency + caret gaps |

## What Shipped — feature

- **`examples/games/settings_menu/settings_menu.rs`** (`settings_menu_game`): Title → Settings →
  Dialogue. First playable-game coverage of `TextInput`, `Slider`×2, `CheckBox`×2, `ScrollView`,
  `Panel`/`LayoutSystem`, rich/multiline `Label`, `LocaleResource` (EN/KO/ES) + `LocalizedText`,
  `AudioManager` buses (`music`/`sfx`) + `AudioEffect` low-pass. `Settings`/locale/`AudioManager`
  persist across `SceneCmd::Replace` via `App::register_persistent`. Registered in `Cargo.toml`.
- **`LocalizedText` + `LocalizationSystem`** (`src/ui/localized.rs`, re-exported from crate root):
  bind a translation key to a `Label`/`Button`/`CheckBox`; one `set_locale()` retranslates the whole
  UI. 3 unit tests.

## What Shipped — engine bug fixes (dogfooding yield)

Each was a pre-existing engine bug the click-heavy example exposed:

| Fix | Root cause | Site |
|---|---|---|
| HiDPI click offset | cursor stored in **physical** px; UI/`screen_to_world` use **logical** px | `app.rs` CursorMoved/touch ÷ `scale_factor` |
| `TextInput` caret nav | no arrow/Home/End/Delete handling | `TextInput::move_left/right/home/end/delete_forward` + `UiSystem` |
| Korean = separated jamo | `set_ime_allowed` never called → no IME compose | `app.rs::finish_init` → `window.set_ime_allowed(true)` |
| Volume/effect silent on tones | `play_tone` ignored `effective_volume` + channel `AudioEffect` | `audio.rs::play_tone` mirrors `play_internal` |
| **Click on wrong widget after move** | `InputState` keeps only the latest cursor; press + same-frame move hit-tested the click at the moved-to spot | `InputState` records `mouse_press_cursor`/`mouse_release_cursor`; `UiSystem` hit-tests clicks/toggles/drag-starts/text-focus against them |
| Caret at wrong position | caret `\|` always appended at string end | `TextInput::display_with_caret` inserts caret+preedit at byte cursor |
| Caret blink shifted text | blink added/removed the glyph | reserve caret slot (space when off, `\|` when on) |
| Input latency | `frame_latency=2`; update ran in `about_to_wait` (frame gap) | `frame_latency=1`; update+render back together in `RedrawRequested` |

## What We Tried — the click/latency investigation (don't redo this)

The mouse issues took several rounds; the dead-ends are the expensive part:

1. **HiDPI offset (round 1)** — real, fixed by ÷scale. Clicks then landed on the cursor. ✓
2. **`ControlFlow::Poll` (round 2)** — added because the loop defaulted to `Wait`; did **not** fix the
   remaining "click drops after a move." Kept (continuous loop is still correct), but it was not the cause.
3. **`update()` → `about_to_wait` (round 3)** — hypothesis: stale cursor because update ran before the
   event drain. Did **not** fix the drop, and **added a frame of latency** (update at iteration N,
   render at N+1). **Reverted in round 6.**
4. **press/release-cursor capture (round 4)** — THE fix. `InputState` collapses the cursor to one
   latest value, so a press + a same-frame move (winit batches them) evaluated the click at the
   moved-to position. Recording the cursor at the press and release *events* and hit-testing clicks
   against those fixed it. Two regression tests in `src/ui/system.rs`.
5. **`frame_latency=1` (round 5)** + **revert about_to_wait (round 6)** — cut latency frames.
6. **`AutoNoVsync` test (round 7, this session)** — user tested; only **marginally** better and uncaps
   the frame rate (battery/thermal cost engine-wide). **Not adopted** — reverted to `AutoVsync`.

**Conclusion:** clicks are now positionally correct and feel correct; the residual "half-beat" is
macOS/winit present + modal-drag behavior, deferred (see below).

## Key Decisions

- `LocalizedText` targets all three text widgets (Label/Button/CheckBox) — full retranslate, no
  manual rebuild. Additive, non-breaking minor (v1.2.0).
- Locales EN/KO/ES via system-font fallback (no bundled font) — Korean renders on macOS; absent on
  Linux CI/wasm (documented). Arabic/RTL omitted (RTL not wired).
- Audio target-gated `cfg(not(wasm32))` (shooter/survivor pattern); two flat buses `music`/`sfx`.
- "Muffle" (M) is a **low-pass**, not a mute — terminology corrected in all locales after a KO
  translation mixup ("음소거"=mute). Music is a **low+high two-tone** so the low-pass audibly removes
  the high tone (a sine alone only attenuates). **User-confirmed working.**
- Caret blinks (visibility) with a reserved slot (minimal shift) — full stability needs a renderer
  overlay (deferred).
- Present mode stays `AutoVsync` + `frame_latency=1` — efficient, no tearing; latency deferred.
- Fullscreen checkbox is a stored **preference** only (no OS fullscreen path); label says so.

## Evidence & Data

- Tests: `cargo test --lib` = **260 passed** (started at 253; +`LocalizedText` ×3, +`display_with_caret`,
  +`cursor_movement_is_utf8_safe`, +2 click regression tests).
- New click regression tests (`src/ui/system.rs`): `click_uses_press_cursor_not_the_moved_cursor`
  (press off-button → move on → release ⇒ **no** click), `click_fires_when_press_and_release_on_button_then_cursor_leaves`.
- PR #4 state: `state=OPEN mergeable=MERGEABLE mergeStateStatus=CLEAN`.
- Runtime log noise on macOS (benign): `TSM AdjustCapsLockLED…`, `error messaging the mach port for
  IMKCFRunLoopWakeUpReliable` — cosmetic IME/system messages, not crashes.

## Known Gaps / Deferred (next-session candidates)

1. **macOS residual input latency** — click registers a beat late; window content lags during a live
   OS window drag (winit enters a modal event loop). `AutoNoVsync` only helped marginally. A
   macOS/winit optimization (e.g. frame limiter + low-latency present, or pacing investigation).
2. **Stable caret** — inline `\|`/space slot still shifts sub-pixel on blink. Proper fix: renderer
   measures the glyph x and draws a caret quad overlay (text never contains the caret). Needs a
   `DrawText` cursor field + caret-draw path; the glyphon text renderer has no quad pipeline today.
3. **`TextInput` horizontal scroll** — single-line, no scroll: long text clips at the field edge;
   IME at `max_len` shows an uncommittable preedit. Fine for short fields; scrolling field is future work.
4. **Real fullscreen** — no window fullscreen-request path; the checkbox only stores a preference.

Broader dogfooding candidates (from `docs/NEXT_WORK.md`, none scheduled): 2D lighting
(`PointLight`/normal-map), `BlendTree1D`, `Timeline`/cutscene, `PostProcessConfig`, physics joints,
`RenderTarget` in real play, networking.

## Files Changed (this branch)

- **New:** `examples/games/settings_menu/settings_menu.rs`; `src/ui/localized.rs`;
  `plans/handoffs/HANDOFF_settings-ui_2026-06-02.md` (seq 1) + this file.
- **Engine:** `src/ui/system.rs` (press/release-cursor hit-test, caret render, nav keys),
  `src/ui/text_input.rs` (cursor nav + `display_with_caret`), `src/ui/mod.rs` + `src/lib.rs`
  (re-exports), `src/input/state.rs` (press/release cursor), `src/audio.rs` (`play_tone` volume+effect),
  `src/app.rs` (HiDPI cursor ÷scale, `set_ime_allowed`, `ControlFlow::Poll`, loop revert),
  `src/renderer/context.rs` (`frame_latency=1`).
- **Build/docs:** `Cargo.toml` (example entry + v1.2.0), `Cargo.lock`, `docs/CHANGELOG.md` (1.2.0
  Added/Fixed/Known-gaps), `docs/HANDOFF.md`, `docs/NEXT_WORK.md`, `CLAUDE.md`.

## User Feedback & Preferences

- QA's the example by *playing* it on macOS, reports precisely (per-symptom), expects polish to feel
  right — drove six fix rounds.
- Pragmatic about scope: agreed to defer macOS/renderer-deep items ("나중으로 미룸").
- Caught the muffle/mute terminology issue themselves (KO "음소거"=mute vs low-pass) — sharp on detail.
- Decisive: "이번 작업은 마무리. /handoff 하고 커밋 후 머지해줘."
- Prefers Korean conversation; English docs (project doc-language rule).

## Where We're Going

- **Now:** commit this handoff, **merge PR #4** (merge commit, mirroring #3), delete the branch,
  sync `main`. v1.2.0 lands.
- **Next session:** await a new direction. If continuing engine work, the deferred items above
  (macOS latency, overlay caret, TextInput h-scroll, fullscreen) or a fresh `NEXT_WORK` dogfooding
  candidate (lighting is the most visually striking untested subsystem).

## Risks & Blockers

- None blocking the merge — CI is the gate (WASM / Package dry-run / Rustdoc / Test native), all
  expected green from local verification.

## Quick Start for Next Session

```bash
# v1.2.0 merged (assuming PR #4 landed). On main.
git switch main && git pull
cargo test --lib                 # expect 260 passed
cargo run --example settings_menu_game   # the UI/i18n/audio slice

# Deferred follow-ups (see Known Gaps): macOS input latency, overlay caret,
# TextInput horizontal scroll, real fullscreen. None scheduled.
# Reference: docs/NEXT_WORK.md, docs/VISION.md, plans/handoffs/HANDOFF_settings-ui_2026-06-02.md
```
