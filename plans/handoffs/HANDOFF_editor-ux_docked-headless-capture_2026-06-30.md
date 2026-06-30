# Docked-mode headless editor capture + prefab action toasts (v0.94.0 / v0.95.0, PRs #298 / #299)

**Date:** 2026-06-30
**Status:** COMPLETED (both shipped + merged)
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `editor-ux` seq `5`
**Parent:** `HANDOFF_editor-ux_session-summary_2026-06-30.md` (`editor-ux` seq 4 — session-close umbrella)

> Chain rationale: the prior session-summary named **docked-mode headless capture** the top infra gap —
> the overlay headless capture couldn't show the docked side panels, so docked-panel editor UI (the
> entity-list eye toggle, inspector, Data Tables) couldn't be visually verified or golden-image tested.
> This session built that, then immediately used it as license to continue editor-UX with a
> headless-verifiable increment (prefab action toasts). `editor-ux` seq 5.

## What shipped

### 1. Docked-mode headless editor capture (v0.94.0, PR #298) — seq 127

`App::screenshot_editor_docked_headless(frames, path)` / `screenshot_editor_docked_headless_rgba(frames)`
(`src/app/headless.rs`): enter `EditorMode::Docked`, drive egui with **no window**, and composite the
**full docked layout** (top toolbar / left entity list incl. the eye toggle / right inspector / bottom
Data Tables–Assets–Audio) **plus the game scene in the central viewport** onto the offscreen texture,
then read it back. Works with the monitor off / asleep / locked, and on CI lavapipe.

- **Why it was needed:** the overlay capture (`screenshot_editor_headless_rgba`, seq 124) leaves the
  editor mode `Off`, so it only draws the *unconditional* chrome (the shortcuts cheatsheet + action
  toasts) — the `Overlay`-gated EngineStats/Inspector windows don't draw, and the docked side panels
  were entirely invisible. So no docked-panel feature could be headless/golden-verified. **This unblocks
  that** — the clearest infra gap from the seq-4 umbrella.
- **Mechanism:** overlay + docked captures now share one private `editor_headless_capture(frames, docked)`
  driver; `screenshot_editor_headless_rgba` is a thin wrapper over it (`docked = false`). The docked
  path sets `self.editor.mode = EditorMode::Docked` before the `update → drive egui → render` loop.
- **Central viewport is a debounced offscreen RT.** `render()`'s existing `prepare_docked_scene_view`
  recreates the game-scene RT only after its size is stable for 3 frames (`RtDebounce`), then egui shows
  the texture the *following* frame. So **pass `frames >= 5`** for the scene to appear in the central
  panel; with fewer frames the side panels still render and the central panel shows the *(no game frame
  yet)* placeholder (graceful, not broken). During warm-up `present_docked_placeholder` no-ops in
  headless (no surface); the final frame's full path clears the surface, composites egui, and reads back.
- **Also new:** `App::editor_select_entity(entity)` (public) — sets the Inspector selection + sole
  multi-selection, the same state a list click produces. Closes the documented-but-missing "select an
  entity to populate the Inspector" gap that *both* capture docs reference.
- **Verified** headlessly **with the screen locked** (acceptance screenshot sent to the user): the docked
  editor renders fully, including the selected quad's move/resize/rotation gizmo and the `Hidden` quad's
  gap in the central scene.
- **Tests/example:** `tests/render.rs::editor_docked_renders_headless` (lavapipe-safe — asserts the
  docked left panel produces bright UI text; the shared `editor_render_or_skip` helper gained a `docked`
  flag, updating the 2 existing overlay call sites) + example `examples/editor_docked_headless_shot.rs`.

### 2. Prefab save/spawn action toasts (v0.95.0, PR #299) — seq 128

`App::save_selected_as_prefab` / `App::spawn_prefab` (`src/app/editor/prefab.rs`) now push a colour-coded
action toast (`ToastKind::Success` / `Error`) alongside the inline `prefab_status` label, mirroring
`do_save_scene_with_list`'s feedback. Completes the toast coverage of the editor's file actions
(delete/duplicate/paste/scene-save already toasted). Editor-only behavior; **no public API change**. Unit
test `prefab_save_and_spawn_push_toasts` (missing-file → Error, successful save → Success).

## New public API this session (all additive)

| Symbol | Seq | Location |
|---|---|---|
| `App::screenshot_editor_docked_headless[_rgba]` | 127 | `src/app/headless.rs` |
| `App::editor_select_entity(entity)` | 127 | `src/app/headless.rs` |
| (none — prefab toasts are internal) | 128 | `src/app/editor/prefab.rs` |

## Cross-cutting learnings

- **The full docked scene renders headlessly for free.** I expected to settle for a placeholder central
  panel (as the seq-4 umbrella predicted), but `render()`'s docked path already targets an offscreen RT
  and `present_docked_placeholder` no-ops cleanly with no surface — so with `frames >= 5` the real game
  scene + the selection gizmo composite into the central viewport. The only requirement was driving
  enough frames past the 3-frame `RtDebounce` warm-up.
- **In docked mode the scene's `ViewportSize` is the central-panel rect**, not the window
  (`schedule.rs` delegates it). So an example/test authoring scene content for the central viewport must
  use coordinates within that ~(window − side-panels) space, not window space — my first example draft
  put the quads off the right edge.
- **A render test asserting "docked rendered" should sample the LEFT panel strip.** That region is the
  part the *overlay* capture can never fill (it's scene-clear-color there), so a bright UI pixel in it
  specifically proves the docked layout composited — a cleaner differentiator than "any bright pixel".
- **`rustdoc -D warnings` rejects intra-doc links to `pub(in crate::app)` items.** Linking
  `` [`EditorMode::Docked`](crate::app::editor) `` failed `private_intra_doc_links`; made it plain text.
  (Recurring gate this chain — always run the doc gate.)
- **Ship the infra, then immediately use it.** Prefab toasts (seq 128) is exactly the kind of
  docked-panel UX increment the seq-127 capture now lets us verify — the infra paid off the same session.

## Verification (both features)

Each passed the full local gate (`fmt`, `clippy --all-targets -D warnings`, wasm `--lib` build,
`doc -D warnings`, `test --all-targets`, `test --doc`) and CI **5/5** (incl. the lavapipe `render` job).
Lib tests 1000 → **1001** (the prefab toast test; the docked render test lives in `tests/render.rs`,
counted separately). Doctests **85**. The 2 audio-device tests are skipped locally (no audio device on
this box — `--skip`; CI gates them).

## Where we are

- **main @ the #299 squash-merge** (package **v0.95.0**, CLAUDE.md header **v1.6.182**), tree clean.
- Memory `engine-current-state.md` at **seq 128**; seqs 121–122 were trimmed to the
  `[[engine-history-archive]]` pointer this session (file 156 KB → ~155 KB after re-adding 127/128).
- Board `../dungeon-merchant/docs/engine-wishlist.md` is **ACTIVE EMPTY** (next free ID EW-004) — checked
  at session start, unchanged.

## Where we're going (next session)

1. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004) — a real downstream
   request outranks self-picked polish.
2. **More editor-UX, now headless-verifiable end-to-end:** inline entity rename (double-click a list row),
   entity-list type/component icons, drag-to-reparent in the Scene tab. Each can now be captured +
   golden-tested via `screenshot_editor_docked_headless`.
3. **Golden-image tests for docked panels** — the capture makes byte-tolerant golden tests of specific
   docked panels feasible (entity list, Data Tables, inspector); consider a small golden harness if a
   regression slips a panel.
4. Otherwise **ASK the user for direction** when the board is empty.

## Risks & blockers / housekeeping

- **GUI playtests remain blocked (locked screen)** — headless is the verification path; docked capture is
  now part of it, so this is much less limiting than it was.
- Docked headless capture is **native-only** (offscreen read-back; the whole editor is native-only).
- Prefab toasts (like all toasts/visibility) are **not serde** — consistent with the sibling markers.
- No OS-gated-CI risk (editor compiles+tests on ubuntu; egui rendering is lavapipe-verified). Tree clean.

## Quick start for next session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -6            # tip = #299 prefab toasts; #298 docked capture
git status -s                   # clean

# Board FIRST
sed -n '53,56p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick

# Reproduce the docked editor capture (works with NO display / locked screen; frames>=5 for the scene)
HEADLESS_SHOT=/tmp/editor_docked.png cargo run --example editor_docked_headless_shot

# Verify (2 audio tests fail locally — env; read exit via echo $?, not ${PIPESTATUS[0]} in zsh)
cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Key files this session
#   src/app/headless.rs            — screenshot_editor_docked_headless[_rgba] + editor_select_entity + shared editor_headless_capture
#   src/app/editor/prefab.rs       — prefab save/spawn action toasts
#   tests/render.rs                — editor_docked_renders_headless (lavapipe) + editor_render_or_skip(docked) flag
#   examples/editor_docked_headless_shot.rs
```
