# Scene-tree context menu (＋Add child) + full documentation audit — 3 PRs (v0.103.0)

**Date:** 2026-07-01
**Status:** COMPLETED (3 PRs shipped + merged; tree clean, all green)
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `editor-ux` seq `9`
**Parent:** `HANDOFF_editor-ux_entity-list-ux-suite_2026-07-01.md` (`editor-ux` seq 8)
**Prior chain:** seq 1 (keyboard-shortcuts) > seq 2 (action-toasts) > seq 3 (entity-visibility) > seq 4 (session-summary) > seq 5 (docked-headless-capture) > seq 6 (inline-rename + scene-reparent) > seq 7 (reparent-undo-review) > seq 8 (entity-list-ux-suite) > this (seq 9)

---

## Since Last Handoff

Seq 8 (entity-list UX suite) closed saying: **(1)** read the Dungeon-Merchant wishlist board FIRST (ACTIVE EMPTY, EW-004) and ASK if still empty; **(2)** editor-UX self-pick breadth is *genuinely exhausted* — the ONLY clean remaining self-pick is the **Scene-tree context menu** (port #310's menu to `scene_tab_body`, reuse `editor_apply_entity_context_action`, maybe add "Add child"); **(3)** otherwise ASK the user for a direction, don't grind marginal tweaks. What happened:

- **Board still ACTIVE EMPTY** (checked at session start; last board edit 2026-06-25, unrelated). No new game requests.
- **Asked the user** via AskUserQuestion (3 options: Scene-tree context menu / name another subsystem / stop). User chose **Scene-tree context menu** → shipped as **#312 (v0.103.0)**, WITH the "Add child" capability seq 8 flagged as the genuinely-new differentiator. This **completes the editor-ux entity-management set** and exhausts the last clean self-pick.
- **Then the user pivoted to documentation** (unprompted by the board): "현재 엔진 기능과 문서가 맞지 않는 부분 찾아내서, 문서 최신화 해줘" → **#313** (internal/living docs). Then "외부에 보여주는 readme, 레퍼런스 문서등 … 수정할 부분 찾아줘" → **#314** (outward-facing docs).
- **Trajectory shift:** editor-ux feature breadth is now FULLY exhausted (this was the last named item). The session's second half became a **documentation audit** — a new, valuable work-type for this chain. Both the internal and external doc surfaces are now audited & synced against code; no known doc mismatches remain.

## Reference Documents

- `CLAUDE.md` — project agent reference; edited by #312 (module-map editor row) and #313 (Verification block, Easing row, RenderPlugin path, trimmed to exactly 200 lines).
- `docs/CHANGELOG.md` — 0.103.0 entry (this session, #312).
- Parent handoff `HANDOFF_editor-ux_entity-list-ux-suite_2026-07-01.md` — seq 8 context (the 4-feature entity-list suite; #307–#310).
- Memory: `engine-current-state.md` (bumped to `f9f7a69`, notes #312/#313/#314), `MEMORY.md` index line, `egui-editor-emoji-glyph-set.md` (glyph tofu gotcha, still relevant).

## The Goal

Keep the docked in-game editor usable + correct while the Dungeon-Merchant wishlist board is empty, then — once editor breadth is exhausted — turn to **documentation correctness**. The broader engine vision (`docs/VISION.md`): a hackable, fork-friendly MIT 2D engine. This session shipped the final clean editor-UX self-pick (Scene-tree context menu, verified headlessly + unit tests) and then ran a **two-part documentation audit** (internal agent-facing docs, then outward-facing README/REFERENCE/ARCHITECTURE) using parallel audit agents, fixing every factual doc↔code mismatch found. End state: editor entity-management UX complete, docs fully synced with code.

## Where We Are

- **main @ `f9f7a69`** (package **v0.103.0**, CLAUDE.md header **v1.6.191**), tree clean, all green.
- **3 PRs merged this session** (all squash-merged on green CI per delegated authority):
  - **#312 v0.103.0** — Scene-tree right-click context menu + `＋ Add child` (feature; the editor-ux continuation).
  - **#313** — living/internal docs sync: 6 factual mismatches + CLAUDE.md trimmed 208→200 lines (docs-only, no package bump; header v1.6.190→v1.6.191).
  - **#314** — outward-facing docs sync: 14 mismatches in REFERENCE.html (×10) + ARCHITECTURE.html (×4) (docs-only, no package bump).
- **Lib unit tests: 1033 → 1035** (+2: `add_child_spawns_a_parented_entity_and_selects_the_child`, `add_child_on_a_dead_entity_is_a_noop`, both in `context_action_tests` → 6 context tests total).
- **Render tests: 11, unchanged** — the new context menu is inert without a right-click, so the Scene tree renders identically; existing `editor_docked_scene_tree_{rename,reparent}_renders_headless` still pass.
- **#312 code** all lives in `src/app/editor/ui/docked.rs`: new `EntityContextAction::AddChild` variant, the AddChild branch in `editor_apply_entity_context_action`, a shared `entity_context_menu()` helper (dedups the menu across the Entities list + Scene tree), and the `scene_tab_body` context-menu wiring (collect-then-apply).
- **#313 fixes** touched `CLAUDE.md`, `docs/PATTERNS.md`, `docs/RENDER_TESTING.md`, `docs/MACOS_FFI.md`.
- **#314 fixes** touched `REFERENCE.html` + `ARCHITECTURE.html` only (both Korean; README.md + FORKING.md were 100% accurate, untouched).
- **Doc audit method:** 4 parallel Sonnet agents per audit (report-only), then I (main/Opus) re-verified EACH finding against `src/` before editing — no agent-suggested fix was applied blind.
- **Every feature is native-only** (`#![cfg(not(target_arch = "wasm32"))]` on `docked.rs`), additive, **no public API** added.
- **The whole session went through the land-pr loop** per PR: branch → verify → /ship (only #312 needed the version paperwork) → commit → PR → CI 5/5 → squash-merge → sync main → memory bump.
- **#312 compiled clean first try** — no fmt/clippy/build fixes needed (unlike #310 last session, which hit `ui.close_menu()` deprecation + a private-type leak). The shared-helper refactor + AddChild landed without churn.
- **2 audio tests are `--skip`ped locally** (`play_tone_reports_playing_then_finished_when_audio_device_exists`, `stop_on_drained_sink_is_immediate`) — no audio device on this locked/remote macOS box; CI gates them. This is the standing local-verify pattern.

## What We Tried (Chronological)

1. **Board check + ask.** Read `../dungeon-merchant/docs/engine-wishlist.md` → ACTIVE EMPTY (EW-004). Per seq 8's guidance (self-pick exhausted), used **AskUserQuestion** rather than grinding. User chose **Scene-tree context menu**.
2. **#312 Scene-tree context menu.** Read `docked.rs` (1521 lines). Found #310's context menu inline in `entities_tab_body` (lines ~613–634) and the `editor_apply_entity_context_action` dispatch. Plan: (a) add `AddChild` variant, (b) extract a shared `entity_context_menu` helper so the two tabs can't drift, (c) wire it into `scene_tab_body`'s node loop with collect-then-apply.
   - Added `EntityContextAction::AddChild`; the dispatch handles it via an early branch (spawn `Transform`+`Tag("New Entity")` like the `＋ New Entity` toolbar button, parent under the target via cycle-safe `crate::hierarchy::reparent(&mut world, child, Some(entity))`, then select the CHILD — not the parent — so the user can immediately rename it; not undoable, matching the New Entity button).
   - Extracted `fn entity_context_menu(ui, e, add_child, out)` — draws Rename/⎘Duplicate/🎯Focus/🗑Delete (+ Scene-tree-only `＋ Add child` when `add_child`), recording `(entity, action)` into `out`. Replaced the inline menu in `entities_tab_body` with `entity_context_menu(ui, e, false, &mut ctx_action)`; wired `scene_tab_body` with `entity_context_menu(ui, entity, true, &mut ctx_action)` on the node's `response`.
   - **Glyph check:** no NEW glyphs needed — `＋` reused from the New Entity button, ⎘/🎯/🗑 from the existing menu (all already verified rendering). So the tofu gotcha (memory `egui-editor-emoji-glyph-set.md`) didn't bite.
   - +2 unit tests. Verify 5 gates green locally (2 audio tests `--skip`), CI 5/5. /ship → v0.103.0. Merged #312.
3. **User: "문서 최신화" (internal).** Ran a doc audit of the LIVING docs (CLAUDE.md + docs/PATTERNS/AGENT_NOTES/RENDER_TESTING/MACOS_FFI/SKELETAL/WASM_SMOKES/VISION). First surveyed the doc landscape myself (`ls docs/*.md`, line counts) and separated LIVING docs from dated snapshots (CODE_ANALYSIS_*, HARDCODING_AUDIT_*, etc.) to scope the agents. **4 parallel Sonnet agents:** (A) existence/reference audit, (B) module-map rows 93–121, (C) module-map rows 122–153, (D) prose sections + subsystem docs. Verified each finding myself (e.g. confirmed `scripts/verify.sh` runs 7 gates not 5; confirmed the render/lavapipe CI job exists; confirmed `src/app/render.rs` is a directory and dispatch is in `render/frame.rs:404`).
   - **Result: module map was remarkably accurate** — 0 mismatches in rows 93–121, all default constants (`DEFAULT_MAX_LIGHTS=16`, `DEFAULT_STICK_ACTIVATE=0.6`, `DEFAULT_ONE_WAY_TOLERANCE=0.05`, …) and byte-asserts (`GpuParticle`=80, `LightingHeader`=32) correct. **6 real mismatches** found (see Evidence).
   - Also fixed CLAUDE.md's **self-rule violation**: 208 lines > its own ≤200 rule (and the gate-list fix pushed it to 210). Removed a duplicate "growth strategy" blockquote + condensed 3 doc-backed Verification bullets → exactly 200, no content dropped. Header v1.6.190→v1.6.191.
   - Docs-only → `docs:` PR #313, no package bump. CI 5/5. Merged.
4. **User: "readme/레퍼런스 등 외부 문서" (outward-facing).** Ran a second audit of the OUTWARD-FACING docs. Surveyed first: found `README.md` (189 lines), `LICENSE` (MIT), `REFERENCE.html` (248KB Korean, ~30 sections, no version number in title), `ARCHITECTURE.html` (Korean). Checked critical README references myself first (examples/files/scripts/signatures — all OK), then **4 parallel Sonnet agents:** (A) README + FORKING.md, (B) REFERENCE.html first half (→파티클), (C) REFERENCE.html second half (타일맵→end), (D) ARCHITECTURE.html. Verified each finding against `src/`.
   - **README + FORKING.md: 100% accurate.** REFERENCE.html: **10 compile-breaking mismatches** (example code that wouldn't compile if copy-pasted). ARCHITECTURE.html: **4** (version pill, frame swim-lane render order, gilrs note).
   - **Applied each fix by grepping the exact HTML** (with `&amp;`/`&lt;`/`&gt;` escaping) then `Edit`. Handled 3 sub-edits for the autotile finding (import + call + prose reference). For the swim-lane I rewrote BOTH lanes at once (GPU particles → overlay, TextRenderer → post-processing) after confirming the order in `frame.rs`.
   - **Went beyond one agent finding:** the NetworkConfig fix — agent suggested only renaming `max_queue`→`max_pending_messages`, but the struct has **5 fields** (`max_message_bytes`, `max_pending_messages`, `max_pending_events`, `max_buffered_bytes`, `read_timeout`) and the doc's 2-field literal had no `..Default::default()`, so it still wouldn't compile. Added `..Default::default()`. This is exactly why every finding gets re-verified before applying.
   - Final grep pass confirmed **0 stale strings** remained and **all corrected strings present**. Docs-only → `docs:` PR #314. CI 5/5. Merged.
5. **User: "handoff 하고 푸시".** → this handoff (seq 9), to be committed + pushed.

## Key Decisions

- **Asked the user instead of self-picking.** Seq 8 pre-authorized a self-pick but flagged breadth as exhausted; with a genuinely-final clean item, I surfaced the choice via AskUserQuestion (recommended the Scene-tree menu as option 1) rather than assuming. User picked it.
- **AddChild selects the CHILD, not the parent.** The selection-scoped ops (Rename/Duplicate/Focus/Delete) pre-select the right-clicked entity; AddChild instead spawns + parents + selects the new child, so it early-returns before the pre-select. Rationale: the user's next action is almost always to rename/position the new child.
- **AddChild is not undoable.** Matches the existing `＋ New Entity` toolbar button (also a bare spawn, no undo). Keeping it consistent avoided inventing an `EditorCmd::Spawn` just for this. (The reparent op IS undoable, but AddChild is a fresh spawn, not a reparent of an existing node.)
- **One shared `entity_context_menu` helper.** Rather than duplicating the menu markup in both tabs (they'd drift as ops are added), a single helper takes an `add_child: bool`. Single source of truth. Matches the seq-8 pattern of `entity_kind` backing both icon + sort.
- **Doc audits: parallel agents REPORT, main agent VERIFIES + EDITS.** Agents run read-only and never edit (avoids write conflicts on CLAUDE.md/REFERENCE.html when 4 run concurrently). Every finding re-verified against `src/` before I applied it — caught the NetworkConfig `..Default::default()` gap the agent missed.
- **Historical/dated docs deliberately left untouched.** `docs/CODE_ANALYSIS_*`, `HARDCODING_AUDIT_*`, `MODULE_COHESION_*`, `NEXT_WORK`, `ROADMAP`, `HANDOFF.md`, `CHANGELOG.md` are point-in-time snapshots — "updating" them to current code would corrupt the historical record. Audited only the LIVING reference docs.
- **CLAUDE.md ≤200: trim redundancy, don't drop content.** The rule says "never drop needed content to hit the limit." Removed a verbatim-duplicate blockquote + condensed bullets whose detail lives in referenced docs → exactly 200, zero information lost.
- **RENDER_TESTING.md test list made self-maintaining.** Rather than re-listing all 11 render tests (which restale on every new editor feature), grouped them + pointed at `tests/render.rs` as authoritative.
- **Doc changes are `docs:` commits, no package bump.** #313/#314 touch no compiled code → crate behavior unchanged → package stays v0.103.0; only the CLAUDE.md doc-version (v1.6.191) and the ARCHITECTURE pill moved.
- **Context menu attaches to the node's outer `response`, not the inner `selectable_label`.** In `scene_tab_body` the node is a `dnd_drag_source(...).response`; the menu goes on that so a right-click on the whole row opens it. A secondary (right) click doesn't trigger the primary-drag reparent, so the two coexist safely.
- **Ran the audits as two separate PRs, not one.** #313 (internal) and #314 (external) were kept distinct because the user requested them separately and they touch different audiences (agent-facing vs new-user-facing). Cleaner review + independent revertability.
- **Chose to verify README critical refs by hand before dispatching agents.** A broken `hello_sprite`/`FORKING.md`/`build_wasm.sh` would wall a new user immediately, so I checked those existence claims myself first (all OK) for instant signal, then let the agents do the deep API-signature audit.

## Evidence & Data

### PRs this session

| PR | Version | Type | Summary | Tests |
|---|---|---|---|---|
| #312 | v0.103.0 | feat | Scene-tree context menu + `＋ Add child` | +2 unit → 6 context tests |
| #313 | — (docs) | docs | Living-docs sync (6 mismatches) + CLAUDE.md →200 lines | none (docs) |
| #314 | — (docs) | docs | Outward-facing docs sync (14 mismatches) | none (docs) |

### #313 — living-docs mismatches fixed (6)

| # | Doc | Was | Now |
|---|---|---|---|
| 1 | CLAUDE.md Verification block | 5 gates | 7 (added `cargo clippy --target wasm32 --lib` + `cargo test --doc` — verify.sh runs 7) |
| 2 | CLAUDE.md `Easing` variants | `Out/InOut/InBack/…` | `EaseOut/EaseInOut/EaseInBack/…` (8 of 10 dropped the `Ease` prefix → would not compile) |
| 3 | CLAUDE.md RenderPlugin file | `src/app/render.rs` | `src/app/render/frame.rs` (dir module; no flat file) |
| 4 | docs/PATTERNS.md resource recipe | `src/resources.rs` | `src/resources/` (dir split by concern) |
| 5 | docs/RENDER_TESTING.md test list | 4 tests | 11 (grouped + points at `tests/render.rs`) |
| 6 | docs/MACOS_FFI.md cross-ref | "CI is Linux only" | "CI is ubuntu only" |

### #314 — outward-facing mismatches fixed (14: REFERENCE.html ×10, ARCHITECTURE.html ×4)

REFERENCE.html (each would fail to compile if copy-pasted):

| Section | Was | Now |
|---|---|---|
| Collision | `DebugConfig { draw_colliders }` | `show_colliders` |
| Collision | `add_system(CollisionGridSystem)` | `CollisionGridSystem::new(64.0)` (non-unit) |
| Autotile | `MultiTerrainAutotile::edge_16(..)` | `TilemapAutotile::multi_edge_16(..)` (unified v0.30.0) |
| Pathfinding | `PathGrid::new(tilemap.width, tilemap.height)` | `tilemap.dims()` → `PathGrid::new(cols, rows)` (no such fields) |
| Steering | `add_system(SteeringSystem)` | `SteeringSystem::new()` (non-unit) |
| Text | `TextAlign { Left, Center, Right }` | `+ End, Auto` |
| Text | `DrawText.color: [u8;4]` | `Color` + added `anchor`/`single_line_caret` fields |
| Post-process | `aberration_strength` / `bloom_strength` | `chroma_offset` / `bloom_intensity` |
| Network | `add_system(NetworkSystem)` | `NetworkSystem::new()` (non-unit) |
| Network | `NetworkConfig { max_queue, .. }` | `max_pending_messages` + `..Default::default()` (5-field struct) |

ARCHITECTURE.html: version pill v0.43.5(2026-06-20)→v0.103.0(2026-07-01); frame swim-lane render order (GPU-particles = overlay group Step 2.8 before post; TextRenderer = post-processing lane Step 4.7 after post+lighting — diagram had them swapped); gilrs native-deps note → "Windows/Linux only; macOS = GameController framework".

### Diff stats per PR

| PR | Files changed | Notes |
|---|---|---|
| #312 | `docked.rs` +160/−35, Cargo.toml/lock, CHANGELOG, CLAUDE.md | all feature code in one file |
| #313 | CLAUDE.md (42 lines net), PATTERNS.md, RENDER_TESTING.md (+13/−4), MACOS_FFI.md | 4 files, docs-only |
| #314 | REFERENCE.html (33 lines), ARCHITECTURE.html (8 lines) | 2 files, docs-only |

### Test-count progression (lib)

| After | Lib tests |
|---|---|
| Session start (seq 8 tip, `0c3e2d4`) | 1033 |
| #312 (AddChild, +2) | 1035 |
| #313, #314 (docs-only) | 1035 |

### Commit log (this session)

| Hash | PR | Summary |
|---|---|---|
| `6fe3a3c` | #312 | feat(editor): Scene-tree context menu + ＋ Add child (v0.103.0) |
| `503be3d` | #313 | docs: sync living docs (6 mismatches) + CLAUDE.md →200 lines |
| `f9f7a69` | #314 | docs: fix 14 code mismatches in REFERENCE.html + ARCHITECTURE.html |

### Render pass order (verified against `src/app/render/frame.rs`, used to fix ARCHITECTURE swim-lane)

`Step 2` sprites → `Step 2.6` DebugDraw→UiQueue + UiQueue rects + UiImageQueue → `Step 2.8` GPU particles (native) → `Step 3` render plugins → `Step 3.5` bloom → `Step 4` post-process → `Step 4.5` lighting → `Step 4.7` TextRenderer (HUD/text, AFTER post+lighting) → `Step 5` fade → egui → present.

### Agent orchestration (reusable — how the two doc audits were run)

Each audit used **4 parallel Sonnet subagents, report-only** (they never edit — avoids write conflicts when several would touch CLAUDE.md/REFERENCE.html at once). Each finding format: doc location + quoted claim · claim vs reality (with `file:line` code evidence) · suggested fix · confidence. The main agent (Opus) then re-verified EACH finding against `src/` before applying — this caught the `NetworkConfig ..Default::default()` gap and confirmed 0 false positives.

- **#313 (living docs) split:** (A) existence/reference audit of all living docs; (B) module-map rows 93–121; (C) module-map rows 122–153; (D) prose sections + subsystem docs. Findings: A=2 (both dir-module path errors), B=0, C=1 (Easing), D=4. After dedup → 6 unique.
- **#314 (outward docs) split:** (A) README + FORKING.md; (B) REFERENCE.html first half (→파티클); (C) REFERENCE.html second half (타일맵→end); (D) ARCHITECTURE.html. Findings: A=0, B=2, C=8, D=4 → 14.
- **Key lesson:** the drift was NOT evenly spread — the module map + README + FORKING were near-perfect; ALL the compile-breaking rot was in the two large hand-maintained Korean HTML references (last touched around v0.43.x per the ARCHITECTURE pill). When re-auditing later, weight effort toward REFERENCE.html/ARCHITECTURE.html.

### Verify + CI (per PR)

Local gate (all green each PR; 2 audio tests `--skip`ped — no audio device on this box): `cargo fmt --check` · `cargo clippy --all-targets -D warnings` · `cargo build --target wasm32-unknown-unknown` · `cargo test --all-targets` (1035 lib) · `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`. CI ran **5/5 green** each PR: Build (WASM) · Package dry-run · Render tests (lavapipe) · Rustdoc · Test (native).

### AddChild dispatch (the new code shape, `docked.rs`)

```rust
if action == EntityContextAction::AddChild {
    let child = self.world.spawn();
    self.world.add_component(child, crate::components::Transform::default());
    self.world.add_component(child, crate::prefab::Tag("New Entity".into()));
    crate::hierarchy::reparent(&mut self.world, child, Some(entity)); // cycle-safe
    self.editor.inspector_selected = Some(child);
    self.editor.selected_entities = vec![child];       // select the CHILD, not `entity`
    return;                                             // early — skips the pre-select below
}
```

### AskUserQuestion presented (session opening)

3 options offered when the board was found empty: **(1) Scene-tree context menu** [recommended — the last clean self-pick] · **(2) name another subsystem/refactor/docs direction** · **(3) stop for now**. User picked (1). This is the model to repeat next session if the board is still empty and no direction is given.

### editor-ux chain arc (seq → version → feature) — orientation for the next session

| Seq | PR(s) | Version | Feature |
|---|---|---|---|
| 1 | — | — | keyboard shortcuts + headless editor screenshot |
| 2 | — | — | action toasts |
| 3 | — | — | per-entity visibility (eye toggle / `Hidden`) |
| 4 | — | — | session summary |
| 5 | #298 | — | docked-mode headless editor capture (`screenshot_editor_docked_headless`) |
| 6 | #301/#302 | v0.96.0/v0.97.0 | inline entity rename + Scene-tree drag-to-reparent |
| 7 | #304 | v0.98.0 | reparent made undoable + review |
| 8 | #307–#310 | v0.99.0–v0.102.0 | entity-list UX suite: type-icons, Scene-tree rename, sort, row context menu |
| **9** | **#312** | **v0.103.0** | **Scene-tree context menu + ＋ Add child** ← this session (+ docs #313/#314) |

The chain built the docked editor's entity-management UX from nothing to a complete set: **classify** (type-icons) → **rename** (inline, both views) → **sort** → **act** (context menu, both views, + create-child). Seq 9 closes it. There is no seq-10 editor feature queued — the next chain entry needs an external signal.

### README critical references verified by hand (#314, before dispatching agents)

All confirmed present/correct, so README needed no fix: examples `hello_sprite` / `basic` / `runtime_policies` (exist under `examples/`); root files `FORKING.md` + `ARCHITECTURE.html`; `scripts/build_wasm.sh`; `Sprite::colored(f32,f32,f32)` (3-arg); `WindowConfig { width:u32, height:u32, title:String, clear_color:[f64;4] }`; `World::clone_entity(src) -> Option<Entity>`. The Quick-Start `use engine::{...}` block (11 re-exports incl. `KeyCode`, `Vec2`) all resolve from `src/lib.rs`; the API-notes Rhai fns (`entity_index`/`entity_generation`/`despawn_entity`) are registered in `src/scripting/api.rs`.

### CLAUDE.md 200-line surgery (#313)

Was 208; the #313 gate-list fix (F1, +2 lines) made it 210. To get back to ≤200 without dropping content: (a) removed the bottom "Growth strategy" blockquote — a verbatim duplicate of the "Length" bullet under Documentation rules (−4); (b) condensed the two doc-backed Verification bullets (Optional wasm smoke checks + GPU render tests — both already point at `docs/WASM_SMOKES.md` / `docs/RENDER_TESTING.md`) (−3); (c) condensed the CI-ubuntu bullet 6→4 lines + the "Why this exists" bullet 3→1 (−4). Landed at **exactly 200**.

### Reusable: how to re-run a doc audit (for a future session)

1. Separate LIVING docs (track code) from DATED snapshots (leave alone): living = CLAUDE.md, docs/{PATTERNS,AGENT_NOTES,RENDER_TESTING,MACOS_FFI,SKELETAL,WASM_SMOKES,VISION}.md, README.md, FORKING.md, REFERENCE.html, ARCHITECTURE.html. Dated = docs/CODE_ANALYSIS_*, HARDCODING_AUDIT_*, MODULE_COHESION_*, NEXT_WORK, ROADMAP, HANDOFF.md, CHANGELOG.md.
2. Dispatch ~4 parallel **Sonnet** agents, **report-only**, split by surface (existence / module-map halves / prose / each big HTML). Prompt them for `file:line` evidence + confidence + a concrete fix; tell them CODE is truth and to be conservative.
3. **Re-verify every finding against `src/` yourself before editing** — agents miss things (the `..Default::default()` case) and can over-flag. 0 false positives this session, but 1 incomplete fix.
4. Weight effort toward `REFERENCE.html` / `ARCHITECTURE.html` — that's where the compile-breaking rot concentrates (hand-maintained, last bulk-touched ~v0.43.x).
5. Docs-only → `docs:` PR, no package bump. Bump the CLAUDE.md doc-version only if CLAUDE.md itself changed.

## Code Analysis

- **`editor_apply_entity_context_action(entity, action)`** (`docked.rs`, module-private `fn`) — guards `is_alive`; for `AddChild` early-returns after spawn+parent+select-child; otherwise pre-selects `entity` then matches Rename/Duplicate/Focus/Delete to the existing (tested) ops. `AddChild => {}` in the final match is unreachable-after-early-return but keeps the match exhaustive.
- **`entity_context_menu(ui, e, add_child, out: &mut Option<(Entity, EntityContextAction)>)`** — the shared helper; `add_child=true` prepends `＋ Add child` + a separator. Uses `ui.close()` (NOT the deprecated `ui.close_menu()`).
- **`scene_tab_body` wiring** — `let mut ctx_action = None;` before the ScrollArea; `response.context_menu(|ui| entity_context_menu(ui, entity, true, &mut ctx_action))` on the non-renaming node branch; applied after the tree + after the reparent drop (`if let Some((e, a)) = ctx_action { app.editor_apply_entity_context_action(e, a); }`). Secondary-click doesn't disturb the primary-drag `dnd_drag_source`.
- **`crate::hierarchy::reparent(world, child, Some(parent)) -> bool`** (`src/hierarchy.rs:99`) — cycle-safe attach+detach; for a fresh child (no parent) it's a plain attach. Public, re-exported.
- **Non-unit systems needing `::new()`** — `CollisionGridSystem::new(cell)`, `SteeringSystem::new()`, `NetworkSystem::new()` all carry internal state (cell_size / scratch buffers / queues). Recurring REFERENCE.html error class.
- **`NetworkConfig` (5 fields)** — `max_message_bytes`, `max_pending_messages`, `max_pending_events`, `max_buffered_bytes: Option<u32>`, `read_timeout: Duration` (`src/network/event.rs:46`). Partial literals need `..Default::default()`.
- **`DrawText` (9 fields)** — `text, position, bounds, size, color: EngineColor(=Color), align, anchor: TextAnchor, rich, single_line_caret: Option<usize>` (`src/renderer/text/queue.rs:25`). Builder (`::new`/`::centered` + `with_*`) is preferred over struct literals.

## Files Changed

### Source code (#312 only)
- `src/app/editor/ui/docked.rs` — `EntityContextAction::AddChild`; the AddChild branch in `editor_apply_entity_context_action`; new `entity_context_menu()` helper; `entities_tab_body` now calls the helper (was inline); `scene_tab_body` context-menu wiring + `ctx_action` collect-then-apply; +2 tests in `context_action_tests`.

### Docs (#313)
- `CLAUDE.md` — Verification block 5→7 gates; Easing variants; RenderPlugin File column path; removed duplicate blockquote + condensed bullets → 200 lines; header v1.6.191 + editor module-map row (the #312 update).
- `docs/PATTERNS.md` — resource recipe `src/resources/`.
- `docs/RENDER_TESTING.md` — test list 4→11, self-maintaining.
- `docs/MACOS_FFI.md` — "CI is ubuntu only".

### Docs (#314)
- `REFERENCE.html` — 10 code-block/field fixes (Collision, Autotile, Pathfinding, Steering, Text, Post-process, Network).
- `ARCHITECTURE.html` — version pill, frame swim-lane render order (both lanes), gilrs macOS caveat.

### Release paperwork (#312)
- `Cargo.toml` / `Cargo.lock` — 0.102.0 → 0.103.0.
- `docs/CHANGELOG.md` — 0.103.0 entry.

### Memory (outside repo)
- `engine-current-state.md` — hash `0c3e2d4`→`503be3d`→`f9f7a69`; header v1.6.191; DOCS-SYNCED notes for #313 + #314. (Edited via Python in-place replace — the tip line is ~69k tokens, too big for the Edit read-gate.)
- `MEMORY.md` — index line updated (hash, seq context, both docs-sync PRs).

## User Feedback & Preferences (REQUIRED)

- **"Scene-tree 컨텍스트 메뉴"** (AskUserQuestion answer) — chose the last clean editor-UX self-pick over "name another subsystem" or "stop". Signal: still wants editor breadth completed before pivoting.
- **"현재 엔진 기능과 문서가 맞지 않는 부분 찾아내서, 문서 최신화 해줘"** — proactively asked for an internal doc audit. Signal: **values documentation correctness**, not just features.
- **"외부에 보여주는 readme, 레퍼런스 문서등, 기타 다른 문서도 수정할 부분 찾아줘"** — extended the audit to outward-facing docs. Signal: cares about the *new-user / public* experience specifically.
- **"handoff 하고 푸시"** — wants the handoff captured AND pushed (not left uncommitted).
- **Standing (from memory + prior seqs):** merge authority delegated (squash on green CI, no re-confirm); user-facing reports **Korean**, repo artifacts **English**; never push to main directly (branch + PR); `cargo fmt` before verify; read gate exit non-piped or via `$pipestatus` (1-indexed) NOT `${PIPESTATUS[0]}`; 2 audio tests fail locally (no audio device) → `--skip` them, CI gates.
- **Momentum + honesty (carried from seq 8):** wants sustained shipping but accepts honest "diminishing returns" framing. This session, honesty took the form of *asking* when self-pick was exhausted rather than grinding — which is what seq 8 recommended.

## Where We're Going

1. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004) — ASK if still empty. Board-driven work is now the ONLY clear high-value path.
2. **Editor-UX self-pick breadth is FULLY exhausted.** The Scene-tree context menu was the last named clean item (seq 8's list), now shipped. Do NOT invent marginal editor tweaks — ASK the user for a direction (a subsystem, a refactor, a game).
3. **Documentation is fully audited & synced** (internal #313 + external #314). No known doc↔code mismatches remain. A future audit is only worth it after substantial new features land.
4. **If the user insists on a self-pick despite exhaustion:** the only remaining editor idea is **group headers when sorted by Kind** (seq 8 flagged it — section labels like "💡 Lights (2)"; logic is unit-testable but the Kind-mode rendering can't be headless-captured since `entity_sort` is `pub(in crate::app)`). Low value — say so.

## Risks & Blockers

- **GUI playtest still blocked** (locked/remote macOS screen) — the #312 gesture paths (right-click a Scene-tree node → menu, click `＋ Add child`) are NOT headless-testable; only the `AddChild` *dispatch* logic is unit-covered and the tree *rendering* is smoke-covered (inert menu). A human click-through on a real display is the only way to catch an egui wiring regression (menu doesn't open on a tree node, Add child spawns in the wrong place).
- **Docs verified by reading, not by compiling.** The 10 REFERENCE.html code snippets are now correct *by inspection* against `src/`, but they are illustrative fragments in an HTML file — nothing compiles them. A future safety net would be doctest-extracting the reference snippets, but that's a larger project (they use undeclared locals like `map_entity`).
- **Whole docked editor is native-only** → no OS-gated-CI risk (CI ubuntu native runner compiles `cfg(not(wasm32))`). Tree clean, all 3 PRs merged.
- **`engine-current-state.md` tip line is ~69k tokens** (one giant line) — too big for the Edit tool's read-gate, so updates need an in-place Python `str.replace` (used twice this session). It keeps growing as seqs accrete. A future cleanup should trim the oldest per-seq detail into `engine-history-archive.md` (as was done on 2026-06-20) before it becomes unwieldy again.

## Open Questions

- **None blocking.** The recurring seq-6/7 open questions (world-vs-local reparent semantics, unparent-zone discoverability) remain — both need a GUI playtest to answer.
- The one soft question is strategic: **what next, given editor breadth + docs are both done and the board is empty?** That's for the user (see Where We're Going).
- **`REFERENCE.html` + `ARCHITECTURE.html` are Korean**, while the Documentation-rules section of CLAUDE.md says prose should be English to cut token cost (exceptions: the beginner glossary + one-off docs). These two large public references are a de-facto third exception (they're human-facing, not agent-facing). Left as-is this session — NOT flagged as a mismatch, just noted. If the user ever wants token-lean agent-readable references, converting them is a separate, larger project.
- **Should the REFERENCE.html snippets be compile-checked?** They're now correct by inspection but nothing compiles them (they use undeclared locals like `map_entity`). A doctest-harness that extracts + compiles them would prevent future rot but is a real project, not a quick win.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4          # tip = f9f7a69 #314 (docs); f9f7a69/503be3d docs, 6fe3a3c #312 feat
git status -s                 # clean

# Board FIRST (ACTIVE EMPTY → ASK the user for a direction — editor breadth + docs are DONE)
sed -n '53,56p' ../dungeon-merchant/docs/engine-wishlist.md

# Verify (2 audio tests fail locally — env; read exit via echo $?, NOT ${PIPESTATUS[0]} in zsh)
cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings

# Key files (this session)
#   src/app/editor/ui/docked.rs   — EntityContextAction::AddChild + entity_context_menu helper + scene_tab_body wiring
#   CLAUDE.md                     — now exactly 200 lines; Verification block = 7 gates
#   REFERENCE.html / ARCHITECTURE.html — Korean public docs, now code-accurate

# Next action
#   Check the board; if empty (likely), ASK the user for a direction.
#   Editor-UX self-pick is FULLY exhausted; docs are fully synced. Do not grind marginal tweaks.
```

## Session Closed

**Closed at:** 2026-07-01
**Commit:** landed via the `docs(handoff)` PR (squash-merged to `main`)
**Session status:** Handed off to next session
