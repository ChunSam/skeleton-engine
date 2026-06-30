# Editor action toasts — transient colour-coded feedback popups shipped (v0.92.0, PR #293)

**Date:** 2026-06-30
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `editor-ux` seq `2`
**Parent:** `HANDOFF_editor-ux_keyboard-shortcuts-headless-editor-screenshot_2026-06-30.md` (`editor-ux` seq 1)

> Chain rationale: continuing the editor user-friendliness work. Seq 1 added keyboard shortcuts + a
> cheatsheet + the headless editor-screenshot capability. The user said "다음세션 진행"; the board was
> empty, so (per protocol) I offered editor-UX increments and the user chose **action toasts/feedback**.
> Now verifiable headlessly thanks to seq 1's screenshot path. `editor-ux` seq 2.

## Related Handoffs

- `HANDOFF_editor-ux_keyboard-shortcuts-headless-editor-screenshot_2026-06-30.md` — the parent (seq 1).
  Its **headless editor screenshot** (`App::screenshot_editor_headless`) is what verifies this feature
  visually on a locked screen + on CI.

## Reference Documents

- `CLAUDE.md` — editor row now lists action toasts. Header → **v1.6.179** / package **v0.92.0**.
- `docs/CHANGELOG.md` — the 0.92.0 entry.
- `~/.claude/.../memory/engine-current-state.md` — bumped to **seq 125** on this handoff PR's landing.
- `../dungeon-merchant/docs/engine-wishlist.md` — game↔engine board; **ACTIVE EMPTY** (EW-004 next).
- Key files: `src/app/editor/ui/toasts.rs` (the feature), `src/app/editor/state.rs` (Toast/ToastKind).

## The Goal

Editor actions wrote a status string into a panel where it's easy to miss. Add **transient
bottom-right toasts** (colour-coded: success/error/info) that auto-expire and fade, wired to the
common editor actions, so a save/delete/duplicate/paste outcome is immediately visible.

## Where We Are

- **main @ `4a4f63d`** (package **v0.92.0**, header **v1.6.179**), tree clean.
  _(This handoff lands as its own `docs(handoff)` PR; memory → seq 125 lands with it.)_
- **PR #293 merged** (squash `4a4f63d`, CI 5/5 green incl. the lavapipe render job).
- **New** `src/app/editor/ui/toasts.rs`: `App::push_editor_toast(msg, kind)` (internal, picks a
  `ToastKind`) + public `App::editor_toast` / `editor_toast_success` / `editor_toast_error`;
  `App::draw_editor_toasts(ctx, dt)` ages (`age += dt`, retain `age < ttl`) + draws a bottom-right
  `egui::Area` stack (`Align2::RIGHT_BOTTOM`, `Layout::bottom_up`), each toast an `egui::Frame::NONE`
  with a kind-coloured fill + label, fading alpha over its last ~0.6 s; queue capped at 5.
- **New types** `Toast { message, kind, age, ttl }` + `ToastKind { Info, Success, Error }` +
  `DEFAULT_TOAST_TTL` (2.6 s) in `state.rs`; `EditorState::toasts: Vec<Toast>`.
- **Drawn** from `update_editor_ui` (after the cheatsheet), so toasts show in both overlay + docked.
- **Wired** to actions: `editor_delete_selection` ("Deleted N", Success), `editor_duplicate_selection`
  ("Duplicated N", Success), `editor_paste_clipboard` ("Pasted N", Info), and `do_save_scene_with_list`
  (Success "Scene saved (N)" / Error "Save failed: …" — covers both the toolbar button and Ctrl+S).
  All localized EN/KO.
- **Tests**: unit `editor_toasts_push_and_cap_to_five`; render `tests/render.rs::editor_toast_renders_headless`
  (lavapipe-verified). Example `editor_headless_shot` now also pushes the 3 colour-coded toasts.

## What We Tried (Chronological)

1. **Board empty + "다음세션 진행"** → offered editor-UX increments; user chose **action toasts**.
2. **Built the toast system**: types in `state.rs`, behaviour in a new `ui/toasts.rs`, drawn from
   `update_editor_ui`. Public `editor_toast{,_success,_error}` so games/tests/headless can push;
   internal `push_editor_toast` for the wired actions.
3. **Wired** delete/duplicate/paste (shortcuts.rs) + save (docked.rs `do_save_scene_with_list`, so
   both the button and Ctrl+S toast).
4. **Visually verified headlessly** (seq-1 capability) on the locked screen: extended
   `editor_headless_shot` to push one of each kind; the screenshot showed red/green/neutral toasts
   bottom-right + the cheatsheet top-left. Sent to the user.
5. **Render test gotcha** (see below): the dark-green Success panel is *darker* than the dark clear
   color, so a mean-brightness assert failed; switched to a **max-luma in the bottom-right quadrant**
   assert (the light toast text far exceeds the clear color). Both render tests green on Metal.
6. **Verify** all gates green (999 lib + 2 render tests + doctest 84 + doc + wasm). **`/ship`** →
   v0.92.0. **`/land-pr`** → branch `feat/editor-toasts`, PR #293, CI 5/5, squash `4a4f63d`, synced.

## Key Decisions

- **Types in `state.rs`, behaviour in `ui/toasts.rs`.** The field's type lives with `EditorState`;
  the push/age/draw methods live in the ui layer. Avoids a `state → ui` dependency.
- **Public `editor_toast{,_success,_error}`; internal `push_editor_toast(kind)`.** Exposes colour-coded
  feedback to games/tests without making `ToastKind` public API (kept `pub(in crate::app)`). The
  example needs the public methods (it's an external crate).
- **Age + draw together in `draw_editor_toasts`**, called once per frame from `update_editor_ui` (in
  the `Some(ctx)` block) — so aging only advances when the editor egui is live, both modes.
- **Wire save in `do_save_scene_with_list`** (the shared helper), so the toast covers the toolbar 💾
  button AND the Ctrl+S shortcut with one change.
- **Queue capped at 5** (`drain` oldest) so a burst of actions can't grow it unbounded.
- **Versioning MINOR (v0.92.0)** — additive, native-only editor feature.

## Evidence & Data

| Hash | PR | Version | Seq | Summary |
|---|---|---|---|---|
| (squashed) `4a4f63d` | #293 | v0.92.0 | 125 | editor action toasts |

### New public API (additive / native-only)

| Symbol | Location |
|---|---|
| `App::editor_toast(msg)` / `editor_toast_success(msg)` / `editor_toast_error(msg)` | `src/app/editor/ui/toasts.rs` |

### Tests

`editor_toasts_push_and_cap_to_five` (8 pushed → 5 kept, oldest dropped) + `editor_toast_renders_headless`
(bottom-right max-luma far exceeds the clear color). `test --all-targets` = 999 lib + integration, 0
failed (2 audio skipped). `test --doc` = 84.

### CI (PR #293 — 5/5 green): Test (native), Render (lavapipe), Build (WASM), Rustdoc, Package dry-run.

### Visual (locked screen, no display)

`HEADLESS_SHOT=… cargo run --example editor_headless_shot` → red "Load failed: missing.ron", green
"Scene saved (3)", neutral "Pasted 2" stacked bottom-right + the cheatsheet top-left. Sent to the user.

## Gotchas & Discoveries

- **A render test for a coloured panel can't assume the panel is *brighter* than the background.** The
  Success toast fill (dark green 34,84,54) is *darker* than the dark clear color (≈63,69,85 in sRGB),
  so a mean-brightness assert failed. The toast's **light text** is the reliable signal — assert the
  **max luma** in the toast's quadrant exceeds the (uniform) clear color by a margin. Position- and
  panel-colour-independent.
- **`egui` 0.34 Frame API**: `egui::Frame::NONE` + `.fill()` + `.corner_radius(CornerRadius::same(n))`
  + `.inner_margin(Margin::symmetric(x, y))` (Margin is `i8`-based in 0.34). Matches existing usage.
- **The headless editor screenshot (seq 1) is now the standard way to verify editor UI** while the
  screen is locked — used here for the toast stack. Reuse it for future editor-UX features.
- **Environmental audio (standing):** 2 audio-device tests fail locally; `--skip`, CI gates audio.

## Files Changed (PR #293)

- `src/app/editor/ui/toasts.rs` (new) — toast push/age/draw + colours.
- `src/app/editor/state.rs` — `Toast`/`ToastKind`/`DEFAULT_TOAST_TTL` + `EditorState::toasts`.
- `src/app/editor/ui/mod.rs` — `mod toasts;` + `draw_editor_toasts` call.
- `src/app/editor/ui/shortcuts.rs` — toasts on delete/duplicate/paste.
- `src/app/editor/ui/docked.rs` — toast on scene save (Ok/Err).
- `src/app/editor/tests.rs` — `editor_toasts_push_and_cap_to_five`.
- `tests/render.rs` — `editor_toast_renders_headless`.
- `examples/editor_headless_shot.rs` — push the 3 colour-coded toasts.
- `CLAUDE.md` (row + header), `docs/CHANGELOG.md` (0.92.0), `Cargo.toml`/`Cargo.lock`.

## User Feedback & Preferences

- **"다음세션 진행"** = proceed with the next session's work; **board first, then pick/ask.**
- Continues the **editor user-friendliness** direction (toasts chosen from the offered increments).
- **Values headless verification** — the toast stack was confirmed via the headless screenshot on a
  locked screen and sent as the artifact.
- Korean for user-facing replies; English for code/docs/handoffs. Merge authority delegated.

## Where We're Going

The editor now has keyboard UX + a cheatsheet (seq 1) + action toasts (seq 2), all headless/CI
verifiable. Further editor-UX increments, driven by real-use friction:

1. **Entity-list QoL**: visibility toggle (eye icon), type icons, inline rename.
2. **Esc-to-deselect** (dropped twice as risky vs. example Esc-quit) — only with clear gating.
3. **More headless editor golden tests**: capture Inspector-with-selection / Data Tables / Tile Paint.
4. **Docked-mode headless capture** (seq 1 did overlay only) — to screenshot the toolbar + viewport.
5. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004) — a real game
   request outranks self-picked polish.

Otherwise: **ASK the user for direction** when the board is empty.

## Risks & Blockers

- **GUI playtests blocked while the screen is locked** — use the headless editor screenshot.
- **Toast colours are fixed** (no per-game theme) — fine for now; a `ToastStyle` resource is a future
  refinement if a game asks.
- No OS-gated-CI risk (editor compiles+tests on ubuntu; egui rendering lavapipe-verified). Clean.

## Open Questions

- Should toasts be **themeable** (colours/position/duration via a resource)? Deferred — defaults are
  sensible; add on a concrete request.
- Should prefab save/spawn also toast? Left for now (they have prominent-ish panel status); easy to add.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5            # main tip = this handoff PR; feature = #293 (4a4f63d)
git status -s                   # clean

# 1) Board FIRST
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick

# Memory: ~/.claude/.../memory/engine-current-state.md  (seq 125 tip)

# Key files
#   src/app/editor/ui/toasts.rs   — the toast system (push/age/draw)
#   src/app/editor/ui/shortcuts.rs — editor keyboard shortcuts (seq 1)
#   src/app/headless.rs           — headless editor screenshot (seq 1; verify editor UI w/o a display)

# Reproduce the editor screenshot (cheatsheet + toast stack; works with NO display / locked screen)
HEADLESS_SHOT=/tmp/editor.png cargo run --example editor_headless_shot

# Verify (2 audio tests fail locally — env; read exit via echo $?, not ${PIPESTATUS[0]} in zsh)
cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Next: read the wishlist; if empty, ASK. Editor-UX follow-ups (entity-list QoL, more headless golden
# tests, docked-mode capture) are available — drive by real-use friction or a game request.
```

## Session Closed

**Closed:** 2026-06-30
**Chain:** `editor-ux` seq 2 — continuation of seq 1.
**Code landed:** #293 (v0.92.0), main @ `4a4f63d`. This handoff lands as its own `docs(handoff)` PR; memory → seq 125 with it.
**Session status:** Handed off. The editor gained colour-coded action toasts, wired to save/delete/duplicate/paste and verified headlessly (locked screen + CI lavapipe). Next session starts from the wishlist board or asks for direction.
