# Editor entity-list UX: inline rename + Scene-tree drag-to-reparent (v0.96.0 / v0.97.0, PRs #301 / #302)

**Date:** 2026-07-01
**Status:** COMPLETED (both shipped + merged)
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `editor-ux` seq `6`
**Parent:** `HANDOFF_editor-ux_docked-headless-capture_2026-06-30.md` (`editor-ux` seq 5)
**Prior chain:** `HANDOFF_editor-ux_keyboard-shortcuts-headless-editor-screenshot_2026-06-30.md` (seq 1) > `..._action-toasts_..` (seq 2) > `..._entity-visibility_..` (seq 3) > `..._session-summary_..` (seq 4) > `..._docked-headless-capture_..` (seq 5) > this (seq 6)

> Chain rationale: the parent (seq 5) built docked-mode headless editor capture and listed the next
> editor-UX increments as **"inline entity rename (double-click a list row), entity-list type/component
> icons, drag-to-reparent in the Scene tab — each can now be captured + golden-tested via
> `screenshot_editor_docked_headless`."** This session did the **first and third** of those three,
> back to back, each landing as its own merged PR. `editor-ux` seq 6.

---

## Since Last Handoff

- Parent's plan was: **board FIRST** (ACTIVE EMPTY, EW-004), then more headless-verifiable editor-UX
  (named: inline rename / entity-list icons / drag-to-reparent). Reality: checked the board (still
  ACTIVE EMPTY), asked the user for direction, user chose "continue editor-UX", then drove two of the
  three named candidates to merge.
- Parent's prediction that the docked headless capture "now lets us verify" these increments **held**:
  both features ship with a `tests/render.rs` docked render test (`editor_docked_inline_rename_renders_headless`,
  `editor_docked_scene_tree_reparent_renders_headless`) that composite the in-edit / re-nested panel
  headlessly on CI lavapipe. The seq-5 infra investment paid off exactly as intended.
- New public-API helper added this session to make the **Scene** tab headless-capturable
  (`App::editor_show_scene_tree`), mirroring the seq-5 `editor_select_entity` pattern (the default
  docked tab is Entities; nothing previously opened the Scene tree with no window).
- Trajectory unchanged: still breadth-expanding the docked editor's usability, board-empty so
  self-picking from the parent's own next-steps list. Two of three candidates done; one remains.

## Reference Documents

- `CLAUDE.md` — agent quick reference; the editor module-map row was updated with both features.
- `docs/CHANGELOG.md` — 0.96.0 + 0.97.0 entries written this session.
- `../dungeon-merchant/docs/engine-wishlist.md` — the Game⇄Engine board; **ACTIVE EMPTY (EW-004)**,
  checked at session start, unchanged.

## The Goal

Keep making the in-game docked editor genuinely usable for hand-authoring scenes — the VISION "the
example is the acceptance test" loop, here applied to the editor itself. The board is empty (no
downstream Dungeon-Merchant request outstanding), so work is self-picked from the parent handoff's
editor-UX next-steps. This session targets two entity-list ergonomics gaps: you couldn't rename an
entity in place (had to select it, then use a separate "Name:" field), and you couldn't re-parent
entities at all from the editor (the Scene tree was read-only — reparenting needed code or hand-edited
RON). Each increment must be headless-verifiable (no display on this locked/remote box) and ship as a
small, merged, version-bumped PR.

## Where We Are

- **main @ `a0b516b`** (package **v0.97.0**, CLAUDE.md header **v1.6.184**), tree **clean**, all green.
- **PR #301 (v0.96.0) MERGED** — inline entity rename in the docked **Entities** list.
- **PR #302 (v0.97.0) MERGED** — Scene-tree drag-to-reparent + cycle-safe `hierarchy::reparent`.
- **Inline rename:** double-click an Entities-list row → its label becomes a focused text box; Enter
  or click-away commits `Tag(buffer)`, Escape cancels; a blank/whitespace name or a despawned entity
  cancels instead of overwriting. Non-renaming rows gained a "double-click to rename" hover hint.
- **Drag-to-reparent:** drag a Scene-tree node onto another → re-parent under it; drag onto the bottom
  "⤴ unparent" zone → detach to a root. Cycle-safe: dropping onto self or a descendant, or a no-op
  move, leaves the graph untouched.
- New public API: `App::editor_begin_rename` (#301); `hierarchy::reparent`, `App::editor_reparent`,
  `App::editor_show_scene_tree` (#302). Internal: `App::editor_commit_rename`/`editor_cancel_rename`,
  `EntityRename`, `EditorState::entity_rename`, `DragEntity(Entity)` egui DnD payload.
- **Lib tests 1001 → 1008 (#301, +7) → 1017 (#302, +9).** Doctests **85**. The docked render tests
  live in `tests/render.rs` (counted separately), both pass on CI lavapipe + on the local real GPU.
- The 2 audio-device tests are skipped locally (no audio device on this locked/remote box —
  `--skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate`);
  CI gates them.
- Memory `engine-current-state.md` bumped to **seq 130** (header pointer + SEQ 129/130 entries; SEQ
  123/124 trimmed to `[[engine-history-archive]]` pointers to keep the file ~154 KB); MEMORY.md index
  line updated. `seq` here = the global engine seq counter, distinct from this handoff's `editor-ux`
  chain seq 6.

## What We Tried (Chronological)

1. **Session start (`/remote-control`):** read parent handoff + the engine-wishlist board. Board
   ACTIVE EMPTY (EW-004) → per the memory protocol, asked the user for direction rather than
   self-picking blindly. User chose "continue editor-UX".
2. **Surveyed the editor entity-list code** (Explore agent) to choose among the 3 candidates: found the
   docked left panel has two tabs sharing one panel (`entities_tab_body` = flat list / `scene_tab_body`
   = indented tree), rows are read-only `selectable_label`s, no double-click handling anywhere, Tag
   editing exists only as a separate "Name:" field (`tag_name_editor`), and the hierarchy data layer
   (`Parent`/`Children`/`attach`) exists but is unsurfaced for editing. Picked **inline rename** first
   (smallest, write-path already exists, fully headless-verifiable).
3. **Inline rename impl (#301):** added transient `EntityRename {entity, buffer, focus_pending}` +
   `EditorState::entity_rename`; factored begin/commit/cancel into `App` methods so the egui closure
   stays thin and the logic is unit-testable without egui. Wired `entities_tab_body`: a row
   `.double_clicked()` → `editor_begin_rename`; while renaming, draw `text_edit_singleline` bound to
   the buffer, `request_focus()` once via `focus_pending`, Escape→cancel / `lost_focus()`→commit.
4. **Inline rename verify:** quick `cargo build --lib` (clean), then full gate. fmt reflowed a test
   assert (ran `cargo fmt` before `--check`, per the known reflow trap). All gates 0; lib 1001→1008.
5. **Shipped #301 via /ship + /land-pr:** v0.96.0 paperwork (Cargo.toml/lock/CHANGELOG/CLAUDE.md
   header), branch `feat/editor-inline-rename`, PR #301, CI **5/5**, squash-merge, sync main, memory
   seq 129.
6. **User: "다음 후보 진행"** (proceed with the next candidate). Chose **drag-to-reparent** (highest
   structural value of the remaining two; the editor literally couldn't reparent before).
7. **Confirmed prerequisites:** egui is **0.34.3** (DnD API `dnd_drag_source`/`dnd_drop_zone`/
   `dnd_release_payload` available). `hierarchy::attach` guards only self-attach, NOT cycles → a
   reparent needs descendant-cycle prevention. `hierarchy` is a `pub mod` with `attach`/`detach`
   re-exported → a new `reparent` slots in naturally and is reachable from `tests/render.rs`.
8. **Reparent impl (#302):** added `hierarchy::reparent(world, child, new_parent: Option<Entity>) ->
   bool` (detach + cycle-checked attach via private `is_ancestor` with a visited-set guard) +
   re-export; `App::editor_reparent` (wraps it + success toast) + `App::editor_show_scene_tree`;
   wired `scene_tab_body` with egui DnD. Hit one clippy stop: the inner `selectable_label` return is
   unused (click comes from the OR'd outer `dnd_drag_source` response) → silenced with `let _ =`.
9. **Reparent verify:** full gate, lib 1008→1017 (+9), render test passed on real GPU. Shipped #302
   via the same /ship + /land-pr loop: v0.97.0, branch `feat/editor-scene-reparent`, PR #302, CI
   **5/5**, squash-merge, sync main, memory seq 130.

## Key Decisions

- **Inline rename = Entities tab only (this session).** It's the default docked tab, so the headless
  capture verifies it directly; Scene-tab inline rename is left as a follow-up. (Drag-to-reparent
  then went into the Scene tab — so both list tabs gained one write feature, but not symmetrically.)
- **`hierarchy::reparent` is a public ENGINE API, not editor-only.** Safe reparent-with-cycle-prevention
  is genuinely useful to any game doing runtime hierarchy edits, and it makes `attach`'s cycle gap
  explicit. `attach`/`detach` were left unchanged (lower-level primitives; changing `attach`'s
  semantics could surprise the skeletal builder and other callers). `reparent` adds the safety on top.
- **Reparent keeps the child's LOCAL Transform** (world position shifts to be relative to the new
  parent), matching `attach`. World-position-preserving reparent (recompute local from the new parent's
  world transform) is a nicety the skeleton's `attach` doesn't do either — deferred, documented.
- **No undo for either feature.** Matches the sibling list-panel QoL ops (the eye/visibility toggle and
  the existing "Name:" field also don't record undo). The gizmo move DOES record undo, so reparent-undo
  is a reasonable follow-up — noted, not done, to keep each PR tight.
- **Logic factored into testable `App` methods.** `editor_begin/commit/cancel_rename` and
  `editor_reparent` hold the behavior; the egui closures just call them. This is what makes the core
  unit-testable without an egui context (the egui *interaction* itself is not headless-testable; the
  logic is). Same pattern the seq-5 `editor_select_entity` established.
- **`hierarchy::reparent` returns `bool`** (changed/not) so the editor layer can decide whether to
  toast — a rejected (cycle/no-op) drop shows no toast; a real move shows a success toast.
- **Cycle check via `is_ancestor(world, child, p)`** (walk Parent chain up from the drop target `p`;
  if it reaches `child`, then `p` is in `child`'s subtree → reject), with a `visited` set so a
  pre-existing cyclic graph can't infinite-loop the walk.

## Evidence & Data

### Commits this session

| Hash | PR | Version | Summary |
|---|---|---|---|
| `f3cca03` | #301 | v0.96.0 | feat(editor): inline entity rename in the entity list |
| `a0b516b` | #302 | v0.97.0 | feat(editor): Scene-tree drag-to-reparent + cycle-safe hierarchy::reparent |

(Both squash-merged to `main`; feature branches `feat/editor-inline-rename` + `feat/editor-scene-reparent` auto-deleted.)

### Test counts

| Gate | #301 (v0.96.0) | #302 (v0.97.0) |
|---|---|---|
| Lib unit tests | 1001 → **1008** (+7 rename) | 1008 → **1017** (+9: 6 `hierarchy::reparent` + 3 `editor_reparent`) |
| Doctests | 85 | 85 |
| New render test | `editor_docked_inline_rename_renders_headless` | `editor_docked_scene_tree_reparent_renders_headless` |
| CI jobs | 5/5 (Test native 5m4s, Render lavapipe, WASM, Rustdoc, Package) | 5/5 (Test native 4m32s, Render lavapipe, WASM, Rustdoc, Package) |

### Local verify gate (both PRs, on this box)

All of `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo clippy --target
wasm32-unknown-unknown --lib -D warnings`, `cargo build --target wasm32-unknown-unknown`,
`cargo test --all-targets` (with the 2 audio skips), `cargo test --doc`, `RUSTDOCFLAGS=-D warnings
cargo doc --no-deps` returned exit **0**. (verify.sh itself is NOT used directly — it runs the 2
audio-device tests with no skip and fails on this box; the gates are run individually with the skips,
which is verify.sh-equivalent minus those 2 CI-gated tests.)

### New public API (all additive)

| Symbol | PR | Location |
|---|---|---|
| `App::editor_begin_rename(entity)` | #301 | `src/app/editor/ui/rename.rs` |
| `hierarchy::reparent(world, child, new_parent: Option<Entity>) -> bool` | #302 | `src/hierarchy.rs` (re-exported at crate root) |
| `App::editor_reparent(child, new_parent)` | #302 | `src/app/editor/ui/reparent.rs` |
| `App::editor_show_scene_tree()` | #302 | `src/app/headless.rs` |

## Code Analysis

- **`scene_tab_body` DnD pattern (egui 0.34):** `ui.dnd_drag_source(Id::new(("scene_dnd", entity)),
  DragEntity(entity), |ui| { let _ = ui.selectable_label(is_selected, &label); }).response` — egui's
  `dnd_drag_source` OR's the inner widget's response into the returned response, so `.clicked()` on the
  outer response still selects (the inner `selectable_label` is drawn only for the highlight, its
  return deliberately discarded). Each node then doubles as a drop target via
  `response.dnd_release_payload::<DragEntity>()` (fires on the frame a drag is released over its rect).
  A bottom `ui.dnd_drop_zone::<DragEntity, _>(frame, ...)` is the root/unparent target. Drops are
  collected into a local `dropped: Option<(Entity, Option<Entity>)>` and applied after the scroll area
  via `app.editor_reparent(child, new_parent)` (deferred, like the existing `clicked_entity` pattern,
  to avoid a mutable-borrow clash inside the closure).
- **`DragEntity(Entity)`** — the DnD payload; `'static + Send + Sync` (a plain `Entity` id), as
  `dnd_drag_source` requires. Defined module-scoped in `docked.rs`, native-gated.
- **`hierarchy::reparent` short-circuits** before mutating: `Some(p)` with `p == child` or `current ==
  Some(p)` → `false`; `is_ancestor(world, child, p)` → `false` + a `log::warn!`; else `detach` then
  `attach`. `None` with no current parent → `false`; else `detach`.
- **`EntityRename.focus_pending`** is the one-shot focus flag: the row's text box calls
  `request_focus()` only while it's true, then clears it — re-requesting every frame would trap focus
  and block click-away commits.
- **Stale-rename cleanup:** `update_editor_ui` drops `entity_rename` if its entity is no longer alive
  (added beside the existing dead-entity `selected_entities` retain), so a despawn mid-edit can't leave
  a stale text box.
- **`inspector_tab == 2` is the Scene tab** (0 = Entities); `editor_show_scene_tree()` just sets it.
  The field is `pub(in crate::app)`, so external test/example crates need the public method to switch.

## Files Changed

### Source code (#301 inline rename)
- `src/app/editor/state.rs` — `EntityRename` struct + `EditorState::entity_rename` field + `new()` init.
- `src/app/editor/ui/rename.rs` — **NEW** module: `editor_begin_rename` (public) / `editor_commit_rename` / `editor_cancel_rename` (internal).
- `src/app/editor/ui/mod.rs` — `mod rename;` + stale-rename cleanup in `update_editor_ui`.
- `src/app/editor/ui/docked.rs` — `entities_tab_body`: double-click → text box, "double-click to rename" hover.

### Source code (#302 drag-to-reparent)
- `src/hierarchy.rs` — `reparent` (public) + private `is_ancestor`.
- `src/lib.rs` — re-export `reparent` next to `attach`/`detach`.
- `src/app/editor/ui/reparent.rs` — **NEW** module: `App::editor_reparent` (public, + toast).
- `src/app/editor/ui/mod.rs` — `mod reparent;`.
- `src/app/headless.rs` — `App::editor_show_scene_tree()` (public).
- `src/app/editor/ui/docked.rs` — `scene_tab_body`: egui DnD drag source + drop targets + bottom unparent zone; `DragEntity(Entity)` payload.

### Tests
- `src/app/editor/ui/rename.rs` — 7 unit tests (buffer seed, trim, blank-cancel, despawn-noop, cancel-discard, untagged-add).
- `src/hierarchy.rs` — 6 `reparent` unit tests (move-between-parents, to-root, reject-self, reject-descendant-cycle, same-parent-noop, root-to-none-noop).
- `src/app/editor/ui/reparent.rs` — 3 `editor_reparent` unit tests (move+toast, reject-cycle-no-toast, to-root).
- `tests/render.rs` — `editor_docked_inline_rename_renders_headless`, `editor_docked_scene_tree_reparent_renders_headless` (both lavapipe-safe).

### Docs / release
- `Cargo.toml` + `Cargo.lock` — v0.96.0 then v0.97.0.
- `docs/CHANGELOG.md` — 0.96.0 + 0.97.0 entries.
- `CLAUDE.md` — header version refs + editor module-map row (inline rename + drag-to-reparent + `hierarchy::reparent`).

## User Feedback & Preferences

- **"핸드오프 확인하고 다음 작업 알려줘"** — start each session by reading the handoff + board, then
  surface the next action. (Board empty → ask, don't blind-pick.)
- **Chose "continue editor-UX"** from the 4-option AskUserQuestion when the board was ACTIVE EMPTY.
- **"다음 후보 진행"** — after #301 merged, told me to proceed with the next candidate without
  re-confirming the specific one → I self-picked the highest-value (drag-to-reparent) and executed.
  Signal: the user delegates the within-area choice; keep momentum, pick well, report the pick.
- **`/handoff 하고 머지`** — wants the handoff doc itself landed as a merged PR (the docs(handoff)
  cadence), not just written.
- Standing context (from memory): merge authority is delegated (squash on green CI, no per-session
  re-confirm); user-facing reports in Korean, artifacts/code in English; use subagents for parallel
  work with explicit `model`.

## Where We're Going

1. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004) — a real
   downstream request outranks self-picked polish. Ask the user if it's still empty.
2. **The one remaining seq-5 editor-UX candidate: entity-list type/component icons** — a small per-row
   glyph hinting the entity's nature (sprite / tilemap / light / animation / generic), headless-
   verifiable via the docked capture. Lowest-risk of what's left.
3. **Scene-tab inline rename** — extend #301's rename to the Scene tree (this session did Entities tab
   only); reuses `editor_begin/commit/cancel_rename` wholesale, just add the double-click + text box in
   `scene_tab_body` (now also a DnD source, so mind the interaction order).
4. **Reparent undo** — add an `EditorCmd` variant so drag-to-reparent is undoable (currently no-undo,
   matching the eye-toggle/rename QoL ops); the most consequential of the no-undo ops.
5. **Golden-image tests for docked panels** (parent's standing suggestion) — the headless capture makes
   byte-tolerant golden tests of specific panels feasible if a regression slips one.
6. Otherwise **ASK the user for direction** when the board is empty.

## Risks & Blockers

- **GUI playtests remain blocked (locked/remote screen).** Headless is the verification path; both
  features have docked render tests, but the egui **drag interaction itself is not headless-testable** —
  only the reparent *logic* (cycle prevention, `Children` maintenance) is unit-covered. A human drag-
  drop smoke on a real display would be the only way to catch an egui-wiring regression in the gesture.
- The whole docked editor is **native-only** (`cfg(not(wasm32))`); none of this compiles to wasm.
- No OS-gated-CI risk (editor compiles + egui renders under ubuntu lavapipe). Tree clean, both merged.
- The 2 audio-device tests fail locally (no device) — always run the test gate with the 2 `--skip`s, or
  read CI; never trust a raw `verify.sh` exit on this box.

## Open Questions

- Should reparent preserve **world** position (recompute local from the new parent) instead of keeping
  local? Current = local (matches `attach`); a user dragging in the editor might expect world-stable.
  Revisit if it feels wrong in a real playtest.
- Is the bottom "⤴ unparent" drop zone discoverable enough, or should unparent also be a right-click /
  a dedicated toolbar action? Unverified without a GUI playtest.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4          # tip = a0b516b #302 reparent; f3cca03 #301 rename
git status -s                 # clean

# Board FIRST (ACTIVE EMPTY → ASK / self-pick the remaining editor-UX candidate)
sed -n '53,56p' ../dungeon-merchant/docs/engine-wishlist.md

# Reproduce the two new headless captures (work with NO display / locked screen; frames>=5 for the scene)
HEADLESS_SHOT=/tmp/rename.png   cargo run --example editor_docked_headless_shot   # (rename via App::editor_begin_rename in code)
# (no dedicated reparent example — the render test drives it: tests/render.rs::editor_docked_scene_tree_reparent_renders_headless)

# Verify (2 audio tests fail locally — env; read exit via echo $?, not ${PIPESTATUS[0]} in zsh)
cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Key files this session
#   src/app/editor/ui/rename.rs    — editor_begin/commit/cancel_rename (#301)
#   src/app/editor/ui/reparent.rs  — App::editor_reparent (#302)
#   src/hierarchy.rs               — reparent + is_ancestor (#302)
#   src/app/editor/ui/docked.rs    — entities_tab_body (rename) + scene_tab_body (DnD reparent)
#   src/app/headless.rs            — editor_select_entity / editor_show_scene_tree
#   tests/render.rs                — editor_docked_inline_rename_renders_headless + ..._scene_tree_reparent_renders_headless

# Next action
#   Check the board; if empty, pick the last seq-5 editor-UX candidate (entity-list type/component icons)
#   or extend inline rename to the Scene tab — both headless-verifiable via screenshot_editor_docked_headless.
```
