# Entity-list UX suite — type-icons + Scene-tree rename + sort + context menu (4 features, v0.99.0→v0.102.0)

**Date:** 2026-07-01
**Status:** COMPLETED (4 features shipped + merged; tree clean, all green)
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `editor-ux` seq `8`
**Parent:** `HANDOFF_editor-ux_reparent-undo-review_2026-07-01.md` (`editor-ux` seq 7)
**Prior chain:** seq 1 (keyboard-shortcuts) > seq 2 (action-toasts) > seq 3 (entity-visibility) > seq 4 (session-summary) > seq 5 (docked-headless-capture) > seq 6 (inline-rename + scene-reparent) > seq 7 (reparent-undo-review) > this (seq 8)

> **Numbering note (read this to avoid confusion):** this is **handoff-chain** `editor-ux` seq **8**. The
> 4 features it covers were tagged **"editor-ux chain seq 8/9/10/11"** in their *PR/commit descriptions* —
> that was a separate *per-feature* counter I used this session, NOT the handoff-chain seq. The authoritative
> per-feature counter is the **memory global seq: 132/133/134/135** (see `engine-current-state.md`). So:
> memory seq 132–135 = the 4 features; handoff-chain seq 8 = this doc. Next session: don't double-count.

---

## Since Last Handoff

Seq 7 (reparent-undo review) closed with a plan: **(1)** read the Dungeon-Merchant wishlist board FIRST
(it was ACTIVE EMPTY, EW-004) and ASK if still empty; **(2)** if empty, self-pick a remaining editor-UX
candidate — it named **entity-list type/component icons** (lowest-risk) and **Scene-tab inline rename**;
**(3)** golden-image tests for docked panels as a standing suggestion. What actually happened:

- **Board still ACTIVE EMPTY** (checked at session start AND re-checked after seq 10; last board edit
  2026-06-25, an unrelated usertest-filename commit). No new game requests.
- Shipped **both** named candidates **plus two more** self-picks, all merged: type-icons (#307), Scene-tree
  rename (#308), entity-list **sort** (#309), row **context menu** (#310). 4 features, v0.99.0 → v0.102.0.
- **Golden-image tests: deliberately NOT done** — investigated and rejected as conflicting with the repo's
  own testing philosophy (see Key Decisions). This resolves seq 5's standing suggestion as "won't do (pixel-exact)".
- **Trajectory:** self-pick breadth on the entity-list/Scene views is now **genuinely exhausted**. The four
  features form a coherent set (classify → rename → sort → act). Next real work needs an external signal
  (the game board) or a user-specified direction. The user explicitly said "계속해" twice, overriding a
  mid-session recommendation to stop — hence 4 features instead of the 2 planned.

## Reference Documents

- `CLAUDE.md` — the editor module-map row (line 101) was updated by every one of the 4 PRs; it now documents
  sort / type-icon / inline-rename (both views) / context-menu.
- `docs/CHANGELOG.md` — 0.99.0 / 0.100.0 / 0.101.0 / 0.102.0 entries (this session).
- Parent handoff `HANDOFF_editor-ux_reparent-undo-review_2026-07-01.md` — seq 7 context.
- Memory: `engine-current-state.md` (bumped to seq 135) + new `egui-editor-emoji-glyph-set.md` (the tofu gotcha).

## The Goal

Keep the docked in-game editor usable and correct, breadth-expanding its entity-management UX while the
Dungeon-Merchant wishlist board is empty (self-pick from the seq-6/7 next-steps). The broader engine vision
(`docs/VISION.md`): a hackable, fork-friendly MIT 2D engine. Editor-UX polish is verified **headlessly**
(via `App::screenshot_editor_docked_headless`) + unit tests, not via a playable example (editor tooling is
the exception to the "example is the acceptance test" rule — the headless capture IS the acceptance test).

## Where We Are

- **main @ `2c8b100`** (package **v0.102.0**, CLAUDE.md header **v1.6.189**), tree clean, all green.
- **4 PRs merged this session** (all squash-merged on green CI per delegated merge authority):
  - **#307 v0.99.0** — per-row **type-icon** in the docked Entities list AND Scene tree.
  - **#308 v0.100.0** — inline entity **rename** from the docked Scene tree (extends #301's Entities-list rename).
  - **#309 v0.101.0** — **sort** the Entities list by name or kind.
  - **#310 v0.102.0** — right-click **context menu** on the Entities list (Rename/Duplicate/Focus/Delete).
- **Lib unit tests: 1020 → 1033** (+13): +6 icon (`icon_tests`), +3 sort (in `icon_tests`), +4 context
  (`context_action_tests`). All in `src/app/editor/ui/docked.rs`.
- **Integration render test: +1** — `editor_docked_scene_tree_rename_renders_headless` in `tests/render.rs`
  (render suite 10 → 11, passes on local GPU + CI lavapipe).
- **New shared classifier**: `entity_kind(world, e) -> EntityKind` (`docked.rs`) — a single priority ladder
  backing BOTH the type-icon (`EntityKind::icon()`) and the Kind-sort (variant order), so they can't drift.
- **New memory**: `egui-editor-emoji-glyph-set.md` (reference) — which emoji render vs □ tofu in egui's
  bundled font, and how to verify (headless capture + PNG visual inspection).
- Every feature is **native-only** (`#![cfg(not(target_arch = "wasm32"))]` on `docked.rs`), additive, and
  adds **no public API** (all helpers are `pub(in crate::app)` or module-private).
- Each of the 4 features went through the full **land-pr loop**: branch → verify (5 gates) → /ship (version
  paperwork) → commit → PR → CI (5/5) → squash-merge → sync main → memory seq-bump.

## What We Tried (Chronological)

1. **Board check + self-pick.** Read `../dungeon-merchant/docs/engine-wishlist.md` → ACTIVE EMPTY. User's
   opening prompt pre-authorized a self-pick from the two seq-7 candidates; picked the lowest-risk
   **type-icons** first.
2. **#307 type-icons.** Added `entity_type_icon(world, e) -> &'static str` in `docked.rs`, a priority-ladder
   `world.get` scan → one emoji per kind, drawn before each row label (Entities list, between the eye toggle
   and the label; Scene tree, injected into the node's display string). **Critical discovery (the session's
   biggest gotcha):** two of my first-choice glyphs — 🧱 (tilemap) and ◇ (transform-only, a BMP geometric
   shape) — rendered as **□ tofu** (missing-glyph box) in egui's bundled emoji font. Found this by rendering
   the docked editor headless to a PNG (`screenshot_editor_docked_headless`), cropping+upscaling the icon
   column with PIL, and **Read-ing the PNG** (the Read tool renders images, so a □ box is visible). Ran a
   second probe rendering ~14 candidate glyphs as entity Tag labels to find which render → swapped to
   **🗺** (map) and **🔹** (small blue diamond), both verified rendering. 6 unit tests on the mapping+priority.
3. **#308 Scene-tree rename.** Extended #301's inline rename (Entities list) to the Scene tab. In
   `scene_tab_body`, a renaming node draws a focused text box (bound to the **shared** `entity_rename` buffer)
   instead of the draggable label — and is deliberately **NOT** wrapped in the node's `dnd_drag_source`, so
   typing/dragging in the field never starts a reparent DnD. Double-click a node → `editor_begin_rename`.
   Verified via a scratch example capture (text box renders in the tree, icon prefix preserved). Added render
   test `editor_docked_scene_tree_rename_renders_headless`.
4. **User: "계속해".** (After I reported seq 9 and recommended stopping.) → picked next self-pick.
5. **#309 entity-list sort.** Added `EntitySortMode` (Insertion/Name/Kind) in `state.rs` + `EditorState::entity_sort`
   (transient) + a `Sort:` toggle row in the Entities tab + `sorted_entity_list(list, mode, world, tag_map)`
   returning a **display-only sorted copy** (world + scene-save order untouched). **Refactored** `entity_type_icon`
   onto a shared `entity_kind -> EntityKind` classifier so the Kind sort (variant order) and the icon can't
   drift. 3 unit tests. clippy caught `sort_by` → `sort_by_key` (fixed; Kind sort became a clean tuple key
   `(EntityKind, name)`). Verified the `Sort:` row renders (Korean labels 기본/이름/종류) via a headless capture.
6. **User: "계속해" (again).** → next self-pick.
7. **#310 context menu.** Right-click an Entities row → Rename/Duplicate/Focus/Delete. `EntityContextAction`
   enum + `App::editor_apply_entity_context_action(entity, action)` which **selects the clicked row first**
   (so duplicate/delete/focus act on it, not the prior selection) then runs the existing op. **collect-then-apply**:
   the egui menu closure only records `(entity, action)` into a local `Option`; applied after the ScrollArea
   (so the closure never mutates `App` mid-iteration — the same pattern `scene_tab_body`'s drop uses).
   Fixed two build issues: `ui.close_menu()` deprecated → `ui.close()`; the private `EntityContextAction`
   leaked through a `pub(in crate::app)` method → made the method module-private (its only callers live in
   `docked.rs`). 4 dispatch unit tests.
8. **User: `/handoff 하고 머지해줘`.** → this handoff (seq 8), to be landed as a `docs(handoff)` PR.

## Key Decisions

- **Golden-image tests for docked panels: rejected.** `tests/render.rs`'s own header states a macOS-Metal
  render can NEVER byte-match ubuntu-lavapipe, and a lavapipe golden drifts with the runner's Mesa version —
  so the repo deliberately uses **renderer-tolerant relative assertions**, never pixel-exact goldens.
  Introducing goldens would fight this and produce flaky tests. Resolved seq-5's standing suggestion as "no".
- **Glyphs verified by headless PNG + Read, not by a brightness test.** A brightness/luma probe (what the
  existing docked render tests use) can't tell a real glyph from a □ tofu box (both are "bright"). The ONLY
  reliable check is rendering to a PNG and visually inspecting it. This caught 🧱/◇ before merge. Captured
  as a permanent memory (`egui-editor-emoji-glyph-set.md`).
- **One classifier for icon + sort.** Rather than two parallel priority ladders (icon strings + sort ranks),
  `entity_type_icon` delegates to `entity_kind -> EntityKind` and `EntityKind::icon()` maps to the glyph;
  the enum variant order backs the Kind sort. Single source of truth → they can never disagree.
- **Sort is display-only.** `sorted_entity_list` sorts a *copy* inside the tab body; the raw `entity_list`
  (used by `do_save_scene_with_list` → `topological_sort_entities`) is untouched. Default = Insertion =
  byte-identical to before. Chose a **stable** sort so equal keys keep insertion order.
- **Context-menu = collect-then-apply.** The menu closure writing to a local `Option<(Entity, action)>`
  (applied after the list) sidesteps the nested-closure double-`&mut App` borrow problem entirely — matches
  the existing `scene_tab_body` drop pattern. Avoided inventing a keyboard rename shortcut (F2 is already the
  docked-mode toggle; a bare letter felt arbitrary) — a right-click menu is the conventional, discoverable choice.
- **Scene-tree rename box is NOT a drag source.** Wrapping the text box in `dnd_drag_source` would make a
  drag-in-field start a reparent. Drawing it bare (only non-renaming nodes are drag sources) avoids that.
- **Kept sort + context-menu to the Entities (flat) list only** (not the Scene tree) to limit risk/scope.
  Sort on a hierarchy is ill-defined; a Scene-tree context menu is a clean follow-up (reuse the same dispatch).
- **Merges without asking, per standing delegated authority** (squash on green CI). User-facing reports Korean,
  repo artifacts (commits/PRs/handoffs) English.

## Evidence & Data

### PRs this session

| PR | Version | Memory seq | Feature | Tests added |
|---|---|---|---|---|
| #307 | v0.99.0 | 132 | Per-row type-icon (Entities list + Scene tree) | +6 unit (`icon_tests`) |
| #308 | v0.100.0 | 133 | Inline rename from the Scene tree | +1 render (`..scene_tree_rename..`) |
| #309 | v0.101.0 | 134 | Sort Entities list (Default/Name/Kind) | +3 unit (sort, in `icon_tests`) |
| #310 | v0.102.0 | 135 | Row context menu (Rename/Dup/Focus/Delete) | +4 unit (`context_action_tests`) |

### Test-count progression (lib)

| After | Lib tests |
|---|---|
| Session start (seq 7 tip) | 1020 |
| #307 (icons) | 1026 |
| #309 (sort, +3; #308 added a render test not a lib test) | 1029 |
| #310 (context) | 1033 |

### Glyph render verdicts (egui bundled emoji font) — the tofu gotcha

| Glyph | Codepoint | Renders? | Used for |
|---|---|---|---|
| 💡 ✨ 🎥 🎬 🔘 🖼 · | — | ✅ | light / particles / camera / animation / UI / sprite / bare |
| 🗺 | U+1F5FA | ✅ | tilemap (replacement) |
| 🔹 | U+1F539 | ✅ | transform-only (replacement) |
| 🧱 | U+1F9F1 | ❌ □ tofu | (tilemap, rejected) |
| ◇ | U+25C7 | ❌ □ tofu | (transform, rejected — BMP geometric shapes aren't in the font) |
| 🧩 🗂 | U+1F9E9 / U+1F5C2 | ❌ □ tofu | (probed, rejected) |
| 🏁 📍 📌 🔸 🎯 | — | ✅ | (probed alternatives, verified rendering) |
| 🎯 ⎘ 🗑 | — | ✅ | context-menu Focus/Duplicate/Delete (Rename = plain text) |

### Kind classification / sort order (EntityKind variant order = group order)

`Light(0) < Tilemap(1) < Particles(2) < Camera(3) < Animation(4) < Ui(5) < Sprite(6) < Transform(7) < Bare(8)`
— first-match priority (a light that also has a sprite classifies as Light). Kind sort groups by this rank,
then case-insensitive name. `sort_kind_groups_by_entity_kind_then_name` asserts `[light, sprite, xform, bare]`.

### Local verify gate (run per feature, all green)

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo build --target wasm32-unknown-unknown`
(lib+bins) · `cargo test --all-targets -- --skip play_tone_… --skip stop_on_drained_sink…` (2 env-only audio
tests skipped locally) · `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`. CI ran 5/5 green each time
(Build WASM / Package dry-run / Render lavapipe / Rustdoc / Test native).

## Code Analysis

- **`entity_kind(world, e) -> EntityKind`** (`src/app/editor/ui/docked.rs`) — the priority ladder (a chain of
  `world.get::<T>().is_some()`), first match wins. `EntityKind` derives `Ord` (variant order = group order for
  the Kind sort). `EntityKind::icon(self) -> &'static str` maps each variant to its verified glyph.
  `entity_type_icon` is now a one-line delegate.
- **`sorted_entity_list(entity_list, mode, world, tag_map) -> Vec<Entity>`** — `entity_list.to_vec()` then
  `sort_by_key`: Name = `name_key(e)` (lowercased label); Kind = `(entity_kind(world,e), name_key(e))` tuple.
  Stable. `name_key` closes over `tag_map` via `entity_label`.
- **`App::editor_apply_entity_context_action(entity, action)`** — module-private (`fn`, not `pub(in crate::app)`,
  so the private `EntityContextAction` doesn't leak). Guards `is_alive`; sets `inspector_selected` +
  `selected_entities = vec![entity]`; matches to `editor_begin_rename` / `editor_duplicate_selection` /
  `editor_focus_camera_on_selection` / `editor_delete_selection` (all pre-existing, tested).
- **`EntitySortMode`** (`src/app/editor/state.rs`) — `#[derive(Default)]` enum, `#[default] Insertion`.
  Re-exported at `crate::app::editor::EntitySortMode` (added to the `pub(super) use state::{…}` line in
  `src/app/editor.rs`). `EditorState::entity_sort` field added to BOTH the `#[derive(Default)]` struct AND
  the explicit `EditorState::new()` constructor (the struct has both — the `new()` lists every field, so a
  new field must be added there too or E0063).
- **Scene-tree rename branch** (`scene_tab_body`): `let renaming = matches!(&app.editor.entity_rename, Some(r) if r.entity == entity);` → if renaming, draw indent-label + bare text box; else the `dnd_drag_source` node
  with `.double_clicked()` → `editor_begin_rename`.
- **Context-menu wiring** (`entities_tab_body`): `let mut ctx_action: Option<(Entity, EntityContextAction)> = None;`
  before the ScrollArea; `resp.context_menu(|ui| { if ui.button(..).clicked() { ctx_action = Some(..); ui.close(); } })`;
  after the ScrollArea, `if let Some((e, a)) = ctx_action { app.editor_apply_entity_context_action(e, a); }`.

## Files Changed

### Source code
- `src/app/editor/ui/docked.rs` — the bulk of all 4 features: `EntityKind` + `entity_kind` + `EntityKind::icon`
  + `entity_type_icon` (delegate); `sorted_entity_list`; `EntityContextAction` + `editor_apply_entity_context_action`;
  the `Sort:` toggle row, the type-icon draw, the context-menu wiring in `entities_tab_body`; the type-icon +
  rename branch in `scene_tab_body`. Plus test modules `icon_tests` (6 icon + 3 sort) and `context_action_tests` (4).
- `src/app/editor/state.rs` — `EntitySortMode` enum + `EditorState::entity_sort` field (struct + `new()`).
- `src/app/editor.rs` — added `EntitySortMode` to the `pub(super) use state::{…}` re-export.

### Tests
- `tests/render.rs` — `editor_docked_scene_tree_rename_renders_headless` (nest a node, show Scene tab, begin
  rename, capture, assert bright left strip).

### Docs
- `CLAUDE.md` — editor module-map row (line 101) + header version (v1.6.185 → v1.6.189).
- `docs/CHANGELOG.md` — 0.99.0 / 0.100.0 / 0.101.0 / 0.102.0 entries.
- `Cargo.toml` / `Cargo.lock` — 0.98.0 → 0.102.0 (four bumps).

### Memory (outside repo)
- `~/.claude/.../memory/engine-current-state.md` — bumped to seq 135 (main hash, version, tip summary).
- `~/.claude/.../memory/MEMORY.md` — index line updated + new pointer for the glyph-set note.
- `~/.claude/.../memory/egui-editor-emoji-glyph-set.md` — NEW reference memory (tofu gotcha + verify method).

## User Feedback & Preferences

- **Opening prompt** — pre-authorized a self-pick from two named candidates ("entity-list type/component icons
  (lowest-risk) or Scene-tab inline rename"); "ask if still empty" re the board. Signal: proceed on the board
  being empty, don't stall.
- **"계속 진행해" / "계속해" (×2)** — after each feature I reported + sometimes recommended stopping; the user
  overrode and said keep going. Signal: **this user wants sustained shipping momentum**; don't stop at one
  feature or over-ask. But also — they value honesty (I flagged diminishing returns each time and they accepted
  the framing while still saying continue).
- **"/handoff 하고 머지해줘"** — wants the handoff captured AND landed as a merged PR, not left uncommitted.
- **Standing (from memory):** merge authority delegated (squash on green CI, no per-session re-confirm);
  user-facing reports Korean, repo artifacts English; never push to main directly; `cargo fmt` before verify;
  read gate exit non-piped or via `$pipestatus` (1-indexed), NOT `${PIPESTATUS[0]}`; 2 audio tests fail locally
  (no audio device) → `--skip` them, CI gates.

## Where We're Going

1. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004) — ASK if still empty.
   Board-driven work is now the highest-value path; self-pick breadth is exhausted.
2. **If self-picking anyway (low value — say so to the user):** the only clean remaining editor-UX items are
   - **Scene-tree context menu** — port #310's menu to `scene_tab_body` nodes, reusing
     `editor_apply_entity_context_action` (+ maybe an "Add child" that spawns a parented entity, a genuinely
     new capability). Low-risk. The most defensible remaining self-pick.
   - **Group headers when sorted by Kind** — section labels ("💡 Lights (2)") in the Entities list; logic is
     unit-testable but the Kind-mode rendering can't be headless-captured (the sort field is private, can't be
     driven from an example/integration test).
3. Otherwise **ASK the user** for a direction (a specific subsystem, a refactor, docs) — don't grind marginal
   editor tweaks without a signal.

## Risks & Blockers

- **GUI playtest still blocked** (locked/remote macOS screen) — the *gesture* paths (right-click → menu,
  double-click → rename in the tree, click a sort toggle) are not headless-testable; only the dispatch/sort/
  classify **logic** is unit-covered and the panels' *rendering* is smoke-covered. A human click-through on a
  real display is the only way to catch an egui wiring regression (menu doesn't open, toggle doesn't re-sort).
- **The context menu popup can't be headless-captured** (needs a click to open) — its verification is the 4
  dispatch unit tests + "the row still renders". Accepted as low-risk (wires only tested ops).
- **Kind-sort visual can't be headless-verified** — `entity_sort` is `pub(in crate::app)` (private to examples
  and the integration test crate), so no scratch example can set it to Kind and capture the grouped list. Sort
  *logic* is unit-tested; the *rendering* of a non-default sort is unverified visually.
- Whole docked editor is native-only → **no OS-gated-CI risk** (CI's ubuntu native runner compiles
  `cfg(not(wasm32))`). Tree clean, all 4 PRs merged.

## Open Questions

- None blocking. (Seq-6/7 open questions — world-vs-local reparent semantics, unparent-zone discoverability —
  remain; both need a GUI playtest to answer.)

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -6          # tip = 2c8b100 #310 context-menu (v0.102.0)
git status -s                 # clean

# Board FIRST (ACTIVE EMPTY → ASK / self-pick a remaining item — but say it's low value)
sed -n '53,56p' ../dungeon-merchant/docs/engine-wishlist.md

# Verify (2 audio tests fail locally — env; read exit via echo $?, NOT ${PIPESTATUS[0]} in zsh)
cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings

# Key files (all 4 features live here)
#   src/app/editor/ui/docked.rs   — entity_kind/entity_type_icon/sorted_entity_list/editor_apply_entity_context_action
#                                    + entities_tab_body (sort row, icon, context menu) + scene_tab_body (icon, rename)
#   src/app/editor/state.rs       — EntitySortMode + EditorState::entity_sort
#   tests/render.rs               — editor_docked_scene_tree_rename_renders_headless

# Gotcha before adding ANY editor glyph: verify it renders (not □ tofu) via a headless PNG.
#   See memory egui-editor-emoji-glyph-set.md. A brightness test CANNOT catch tofu.

# Next action
#   Check the board; if empty, ASK the user for a direction (self-pick breadth is exhausted).
#   Lowest-value-but-clean self-pick if they insist: Scene-tree context menu (reuse editor_apply_entity_context_action).
```
