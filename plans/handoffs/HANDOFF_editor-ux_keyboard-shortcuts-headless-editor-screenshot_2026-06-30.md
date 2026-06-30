# Editor keyboard-UX (shortcuts + cheatsheet) + a headless EDITOR screenshot capability to verify it without a display — shipped (v0.91.0, PR #291)

**Date:** 2026-06-30
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `editor-ux` seq `1` (NEW chain — topic pivot from feature-breadth to editor UX)
**Parent:** `HANDOFF_breadth-features_effect-offset_2026-06-30.md` (`breadth-features` seq 8; same session, earlier)
**Prior chains:** `breadth-features` (10 features) → `hardcoding-audit` (closed)

> Chain rationale: after the effect-offset work (breadth-features seq 8) landed and the board was
> empty, the user asked to **"make the editor more user-friendly"** — a new direction. They chose
> "keyboard UX + shortcuts cheatsheet". When the locked screen blocked a visual playtest, the user
> asked why the recently-added **headless screenshot** couldn't verify it; that led to building a
> **headless EDITOR screenshot** capability. New chain `editor-ux`, seq 1.

## Related Handoffs

- `HANDOFF_breadth-features_effect-offset_2026-06-30.md` — the immediately-prior work this session
  (seq 8 / seq 123); same session, different topic.
- `HANDOFF_headless-screenshot_2026-06-28.md` — added `App::screenshot_headless` (game render only).
  This session **extends** that idea to the egui editor overlay (the key discovery: plain headless
  can't capture egui because the editor egui is driven by the windowed `egui_winit` loop).

## Reference Documents

- `CLAUDE.md` — the headless-screenshot row now documents the editor variant; the editor row lists
  the keyboard shortcuts + cheatsheet. Header → **v1.6.178** / package **v0.91.0**.
- `docs/CHANGELOG.md` — the 0.91.0 entry (two Added sections).
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — bumped to
  **seq 124** on this handoff PR's landing.
- `../dungeon-merchant/docs/engine-wishlist.md` — game↔engine board; **ACTIVE EMPTY** (EW-004 next).
- Key files: `src/app/editor/ui/shortcuts.rs` (shortcuts + cheatsheet), `src/app/headless.rs`
  (headless editor screenshot), `examples/editor_headless_shot.rs`, `tests/render.rs`.

## The Goal

Make the in-game editor more user-friendly (the user's request). Chosen scope: standard keyboard
shortcuts + a discoverable cheatsheet. Then — because the screen was locked and the egui editor UI
can't be headless-captured by the existing path — build a **headless editor screenshot** so the
cheatsheet (and any editor UI) is visually verifiable with no display, including on CI.

## Where We Are

- **main @ `f5f6b4c`** (package **v0.91.0**, CLAUDE.md header **v1.6.178**), tree **clean**.
  _(This handoff lands as its own `docs(handoff)` PR; memory → seq 124 lands with it.)_
- **PR #291 merged** (squash `f5f6b4c`, CI **5/5** green incl. the lavapipe render job): `feat(editor):
  keyboard-UX shortcuts + cheatsheet, verified via a headless editor screenshot (v0.91.0)`.

### Feature 1 — editor keyboard UX (`src/app/editor/ui/shortcuts.rs`)
- New shortcuts: **Ctrl+S** save scene, **Ctrl+D** duplicate selection, **Delete**/**Backspace**
  delete selection, **F** focus camera on the selection, **?** toggle the cheatsheet (with the
  existing Ctrl+Z/Shift+Z/C/V). Bare single-key shortcuts are gated on `ctx.egui_wants_keyboard_input()`
  so typing in a TextEdit never fires them (Ctrl-combos stay ungated, matching the existing ones).
- Selection-scoped ops factored into `App::editor_delete_selection` / `editor_duplicate_selection`
  (multi-select aware, each recorded for undo) / `editor_focus_camera_on_selection` (centers the
  Camera: `position = entity_pos - viewport/2 / zoom`). Undo/redo/paste extracted into helpers.
- A **cheatsheet** window (`EditorState::show_shortcuts`), drawn by
  `App::draw_editor_shortcuts_window` (an `egui::Window` + `Grid` of key→action rows, localized
  EN/KO), toggled by `?` or a new **`? Keys`** toolbar `selectable_label`. Drawn in both overlay and
  docked modes (before the `is_enabled()` gate, so it shows regardless).

### Feature 2 — headless EDITOR screenshot (`src/app/headless.rs`)
- `App::screenshot_editor_headless(frames, path)` / `screenshot_editor_headless_rgba(frames)` render
  the egui **editor overlay** to the offscreen texture with **no window**. The trick: drive egui
  manually — synthesize a `RawInput` (screen_rect = viewport, ROOT viewport ppp=1) → `ctx.begin_pass`
  → `App::update_editor_ui(&Some(ctx), dt)` → `ctx.end_pass()` → `tessellate` → stash in
  `render.egui_output`; the normal `render()`'s egui pass (`present_egui`/`submit_egui`) then
  composites it onto the offscreen `final_view`. Requires a fresh `DebugUi`/egui `Context` (with the
  Korean fallback fonts from `DebugUi::new_with_ctx`) + an `egui_wgpu::Renderer` matched to the
  offscreen format (`new_headless` = `Rgba8UnormSrgb`). Overlay mode (not docked). Native-only.
- `App::set_editor_shortcuts_visible(bool)` — public; opens the cheatsheet programmatically.
- Example `editor_headless_shot` (opens the cheatsheet + captures) and render test
  `tests/render.rs::editor_overlay_renders_headless` (asserts the editor overlay produces bright UI
  pixels over a dark scene; position-independent max-luma scan) — **runs on the GPU-less CI runner
  via lavapipe**, so editor UI is now CI-verifiable.

## What We Tried (Chronological)

1. **User: "make the editor more user-friendly."** Surveyed the editor: the only shortcuts were
   Ctrl+Z/Shift+Z/C/V + F1/F2; **no cheatsheet, no Delete/Ctrl+S/Ctrl+D/focus**. Proposed options;
   user chose **keyboard UX + cheatsheet**.
2. **Built the keyboard UX.** Found delete/duplicate logic was inline in the docked button handlers;
   factored selection-scoped `App` methods (reused by the keys), bumped `do_save_scene` to
   `pub(in crate::app)` for Ctrl+S, added `show_shortcuts` to `EditorState`, the cheatsheet window,
   the `? Keys` toolbar button. 3 new unit tests (delete/duplicate/focus) — all green.
3. **Hit two compile gotchas:** `wants_keyboard_input()` is deprecated → the codebase already uses the
   renamed `egui_wants_keyboard_input()` (window.rs) — used that. A wasm-version note: glam serde is
   on but the codebase uses tuple mirrors (carried from prior seq, not relevant here).
4. **Tried a GUI playtest for the visual — BLOCKED: the screen is locked.** Launched `sprite_flip`,
   injected F2/? via osascript, screencaptured → got only the macOS lock screen. Confirmed the
   standing "locked/remote macOS" condition: no visible window, key injection lands nowhere.
5. **User asked: "didn't I add headless screenshot to test without a display — can't you use it?"**
   Investigated: `screenshot_headless` calls `update`+`render`, but the editor egui is built only when
   `begin_egui_frame` returns a ctx, which **requires a window + egui_winit state** (both absent
   headless) → editor never renders headlessly. Reported this precisely; the user chose **"build the
   headless editor screenshot first."**
6. **Built the headless editor screenshot** (drive egui manually, see Where We Are). First run
   **worked on the locked screen** — the cheatsheet rendered correctly with Korean localization (no
   tofu). Sent the screenshot as the acceptance artifact.
7. **Added a render test** (`editor_overlay_renders_headless`) via a parallel `editor_render_or_skip`
   helper, so the headless editor path is CI-verified on lavapipe.
8. **Earlier in the same session** (separate, already-landed): the editor-i18n candidate was found
   already-done (v0.46.0 #174) and the effect-offset feature shipped (seq 123, #289/#290). See that
   handoff.
9. **Verify (gate-by-gate).** fmt/clippy(native+wasm)/wasm-build/`test --all-targets` (998 lib + the
   render test; 2 audio skipped)/`test --doc` (84)/`doc -D warnings` — all green.
10. **`/ship`** → v0.91.0 (MINOR) + **`/land-pr`** → branch `feat/editor-keyboard-ux`, PR #291, CI 5/5
    (Render job log: `adapter=llvmpipe backend=Vulkan` + `editor_overlay_renders_headless ... ok`),
    squash `f5f6b4c`, synced main.

## Key Decisions

- **Drive egui manually for the headless editor, rather than faking a window.** `egui_winit::State`
  needs a real window; synthesizing `RawInput` directly is cleaner and window-free. The ROOT viewport
  must advertise `native_pixels_per_point = Some(1.0)` or layout/tessellation misbehaves.
- **Overlay mode, not docked, for the headless editor.** The docked path composites the game scene
  into an egui `ImageButton` (window-oriented, complex); overlay just draws egui windows over the
  scene — far safer headlessly and enough to verify any panel/window.
- **Reuse the existing render path's egui pass.** Instead of a bespoke egui render, stash the
  tessellated jobs in `render.egui_output` and let `present_egui`/`submit_egui` (already called by
  `render()`) composite them — so the headless path shares the windowed egui rendering exactly. Only
  needs an offscreen-format `egui_wgpu::Renderer` set up (the windowed one is surface-format).
- **Bare single-key shortcuts gated on `egui_wants_keyboard_input()`; Ctrl-combos ungated.** Matches
  the existing Ctrl+Z/C/V (which don't gate) and prevents Delete/F/? from firing while typing a path.
- **Selection-scoped ops as multi-select `App` methods**, reused by the keys; the inline button
  handlers were left behavior-preserving (single-entity) to avoid a behavior change to the buttons.
- **A render test with a position-independent max-luma assertion.** The editor window's exact egui
  placement isn't guaranteed, so the test scans for the brightest pixel (editor text/panel over a
  dark clear) rather than sampling a fixed region — renderer-tolerant, like the rest of `render.rs`.
- **One PR for both features.** They were co-developed (the headless shot was built to verify the
  cheatsheet; the example serves both); the coherent theme is "user-friendly editor + headless
  verification." Versioned MINOR (two additive features, native-only).

## Evidence & Data

### Commit / PR

| Hash | PR | Version | Seq | Summary |
|---|---|---|---|---|
| (squashed) `f5f6b4c` | #291 | v0.91.0 | 124 | editor keyboard UX + cheatsheet + headless editor screenshot |

### New public API (all additive / native-only)

| Symbol | Kind | Location |
|---|---|---|
| `App::screenshot_editor_headless(frames, path)` | method | `src/app/headless.rs` |
| `App::screenshot_editor_headless_rgba(frames)` | method | `src/app/headless.rs` |
| `App::set_editor_shortcuts_visible(bool)` | method | `src/app/headless.rs` |
| (internal) `App::editor_{delete,duplicate}_selection`, `editor_focus_camera_on_selection`, `draw_editor_shortcuts_window` | `pub(in crate::app)` | `src/app/editor/ui/shortcuts.rs` |
| `EditorState::show_shortcuts` | field | `src/app/editor/state.rs` |

### Tests

3 new unit tests (`editor_delete_selection_despawns_all_and_clears_with_undo`,
`editor_duplicate_selection_clones_offset_and_selects`, `editor_focus_camera_centers_on_selection`)
+ 1 render test (`editor_overlay_renders_headless`). `test --all-targets` = 998 lib + integration,
0 failed (2 audio skipped). `test --doc` = 84.

### CI (PR #291 — 5-job matrix, all green)

| Job | Result |
|---|---|
| Test (native) | pass (`editor_overlay_renders_headless ... ok`) |
| Render tests (lavapipe) | pass (`adapter=llvmpipe backend=Vulkan`) |
| Build (WASM) | pass |
| Rustdoc | pass |
| Package dry-run | pass |

### Visual verification (locked screen, no display)

`HEADLESS_SHOT=/tmp/editor_headless.png cargo run --example editor_headless_shot` produced a correct
cheatsheet render — title "⌨ 키보드 단축키", Key/Action columns, all 10 shortcuts, Korean localized,
no tofu — **with the screen locked**. Sent to the user as the acceptance artifact.

## Gotchas & Discoveries

- **The egui editor overlay is invisible to a plain headless screenshot.** `begin_egui_frame` returns
  `None` without a window + `egui_winit::State`, so `update()` never builds the editor egui. To
  capture it headlessly you must drive egui yourself (synth `RawInput` → build UI → tessellate →
  feed `render.egui_output`) and set up an offscreen-format `egui_wgpu::Renderer`. This is the core
  reusable discovery — it makes ALL editor UI headless/CI-verifiable, not just the cheatsheet.
- **`egui::Context::wants_keyboard_input()` is deprecated → `egui_wants_keyboard_input()`** (egui
  0.34). The codebase already used the new name in `window.rs`; grep before adding a deprecated call.
- **Synthesized egui `RawInput` needs the ROOT viewport's `native_pixels_per_point`** set, or
  tessellation/layout uses a wrong ppp.
- **The screen is LOCKED on this macOS box** — GUI playtests (window + key injection + screencapture)
  return only the lock screen. Headless GPU rendering still works (no display needed). Prefer headless
  capture over GUI automation here.
- **Editor is native-only but compiles + unit-tests on ubuntu CI** — so the keyboard-UX logic is
  CI-gated; only the egui *rendering* needed the new headless path (now lavapipe-verified). The editor
  is NOT an OS-gated-CI risk.
- **Environmental audio (standing):** 2 audio-device tests fail locally; `--skip` them, CI gates audio.
- **zsh `${PIPESTATUS[0]}` is empty** (carried) — read exit via `echo $?` / `$pipestatus[1]`.

## Files Changed (PR #291)

- `src/app/editor/ui/shortcuts.rs` — shortcuts (new keys + helpers) + the cheatsheet window.
- `src/app/editor/state.rs` — `show_shortcuts` field + default.
- `src/app/editor/ui/docked.rs` — `? Keys` toolbar button; `do_save_scene` → `pub(in crate::app)`.
- `src/app/editor/ui/mod.rs` — call `draw_editor_shortcuts_window` after the shortcut dispatch.
- `src/app/editor/tests.rs` — 3 new editor unit tests.
- `src/app/headless.rs` — `screenshot_editor_headless[_rgba]`, `set_editor_shortcuts_visible`, the
  manual egui driver `drive_editor_egui_headless`.
- `examples/editor_headless_shot.rs` — opens the cheatsheet + captures headlessly.
- `tests/render.rs` — `editor_render_or_skip` helper + `editor_overlay_renders_headless` render test.
- `CLAUDE.md` (rows + header), `docs/CHANGELOG.md` (0.91.0), `Cargo.toml`/`Cargo.lock`.

## User Feedback & Preferences

- **"Make the editor more user-friendly"** — an open editor-UX direction; the specific feature (keyboard
  UX) was the user's pick from concrete options.
- **The user pushed for headless testability** — "can't you test without a display using the headless
  feature?" drove building the headless editor screenshot. Values **verification infrastructure**, not
  just the feature.
- **Honesty about the locked screen** — reported the GUI-playtest block plainly, then found a path
  (headless) that works on a locked screen.
- **Values the acceptance artifact** — the headless cheatsheet screenshot was sent as proof.
- **Korean for user-facing replies; English for code/docs/commits/handoffs.** Merge authority delegated
  (squash on green CI; #291 landed without asking).

## Where We're Going

The editor now has discoverable keyboard UX **and** is headless/CI-verifiable. Natural follow-ups
(editor-UX chain), driven by use:

1. **More editor UX**: action toasts/feedback (save/delete confirmations), entity-list QoL (visibility
   toggle, type icons, inline rename), Esc-to-deselect (dropped this round as risky vs. example
   Esc-quit), gizmo/snap polish. Pick by what feels rough in real use.
2. **Headless editor screenshots for golden tests**: the new path could capture more editor panels
   (Inspector with a selection, Data Tables, Tile Paint) for CI golden-image coverage — the capability
   is now there; add tests as editor features change.
3. **Docked-mode headless capture**: this round did overlay mode only; docked (with the composited
   game texture + toolbar) would need the docked render path wired headlessly — more involved.
4. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004) — a real game
   request outranks self-picked editor polish.

Otherwise: **ASK the user for direction** when the board is empty.

## Risks & Blockers

- **GUI playtests are blocked while the screen is locked** — use the headless editor screenshot for
  editor-UI verification instead (works locked).
- **Headless editor capture is overlay-mode only** — docked-mode (toolbar + composited viewport) isn't
  captured yet; the `? Keys` *toolbar button* therefore isn't in the headless shot (the cheatsheet
  *window* is). Verify the toolbar button manually or extend to docked capture later.
- **`?` key detection is Shift+`/`** — fine on a US layout; other layouts may differ, but the `? Keys`
  toolbar button is a layout-independent fallback toggle.
- No OS-gated-CI risk (editor compiles+tests on ubuntu; egui rendering now lavapipe-verified). Clean.

## Open Questions

- Should the headless editor capture also support **docked mode** (to screenshot the toolbar + the
  composited game viewport)? Deferred — overlay covers windows/panels; docked is more wiring.
- Should `Esc` deselect in the editor? Dropped this round (risk of clashing with examples' Esc-quit);
  revisit if a clear, conflict-free gating emerges.
- Which **editor-UX** polish next (toasts, entity-list QoL, …)? Drive by real-use friction or a game
  request, not speculation.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5            # main tip = this handoff PR; feature = #291 (f5f6b4c)
git status -s                   # clean

# 1) Read the game↔engine board FIRST
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick

# Memory: ~/.claude/.../memory/engine-current-state.md  (seq 124 tip)

# Key files
#   src/app/editor/ui/shortcuts.rs  — editor shortcuts + the cheatsheet window
#   src/app/headless.rs             — headless EDITOR screenshot (drive egui w/o a window)
#   examples/editor_headless_shot.rs — capture the editor overlay headlessly
#   tests/render.rs::editor_overlay_renders_headless — the lavapipe-verified editor render test

# Reproduce the editor screenshot (works with NO display / locked screen)
HEADLESS_SHOT=/tmp/editor_headless.png cargo run --example editor_headless_shot

# Verify (2 audio-device tests fail locally — env; read exit via echo $?, not ${PIPESTATUS[0]} in zsh)
cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Next action: read the wishlist; if empty, ASK. Editor-UX follow-ups (toasts / entity-list QoL /
# more headless editor golden tests) are available but should be driven by real-use friction.
```

## Session Closed

**Closed:** 2026-06-30
**Chain:** `editor-ux` seq 1 — pivot from `breadth-features` (parent = the effect-offset handoff, seq 8).
**Code landed:** #291 (v0.91.0), main @ `f5f6b4c`. This handoff lands as its own `docs(handoff)` PR; memory → seq 124 with it.
**Session status:** Handed off. The editor gained keyboard UX + a cheatsheet, and — answering the user's testability push — a headless editor-screenshot path that makes editor UI verifiable with no display and on CI (lavapipe). Next session starts from the wishlist board or asks for direction.
