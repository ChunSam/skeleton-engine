# Editor-GUI arc seq 2 — live playtest → game-data-editor bug + 3 review sweeps → 11 fixes shipped (v8.1.2/8.1.3/8.1.4)

**Date:** 2026-06-14
**Status:** COMPLETED (arc converged; all 3 PRs merged to `main`)
**Bead(s):** none (`bd` not installed — `command not found`; tracked in-session)
**Epic:** Editor GUI — a GUI for game developers to customize in-game UI/UX and game data
**Chain:** `editor-gui-arc` seq `2`
**Parent:** `HANDOFF_editor-gui-arc_docked-editor-to-data-editor_2026-06-13.md` (seq 1)
**Prior chain:** `HANDOFF_editor-gui-arc_docked-editor-to-data-editor_2026-06-13.md` > this

---

## Since Last Handoff

Parent (seq 1) shipped the 3-phase editor GUI (v7.1.0→v8.1.0) and listed under "Where We're Going": **#1 arc complete / clean slate**, **#2 optional polish — live editor playtests of v8.0/v8.1 (never eyeballed by a human; B/C were skipped at merge)**. This session executed exactly #2 — and the playtest immediately found that the **headline Phase C feature was broken in real play**, which turned into a full bug-fix arc.

- Parent's open worry materialized: the skipped Phase B/C playtests hid a real bug. The `stat_editor_game` game-data editor (the v8.1.0 headline) did **not** work under its documented usage — `Stats` was absent from the Inspector, not serialized, and the Data Tables panel was empty.
- Parent said "candidate/structural-debt queues are empty." Still true for *new features*, but this session opened (and closed) a **reliability** queue: 1 game-data bug + 10 editor-correctness bugs found by adversarial review sweeps, all fixed.
- Trajectory: parent BUILT the editor; this session HARDENED it. Both editor reqs (scene/UI layout editing; game-data editing) are now verified working with all found bugs fixed.
- Parent's process notes held: live playtest is human/GUI-bound; this session got it working via synthetic input (with caveats, see Learnings). The "never trust agent self-reports — re-run gates" rule caught a (false) stale diagnostic again.

## Reference Documents

- `CLAUDE.md` (now header v8.1.4) — module map, verification gates (`./scripts/verify.sh`), VISION loop.
- `docs/CHANGELOG.md` — `## 8.1.2`, `## 8.1.3`, `## 8.1.4` all written this session (canonical fix descriptions).
- `docs/VISION.md` — "a feature is not done until a small playable example exercises it in real play"; the example IS the acceptance test (this is what caught the bug).
- Memory: `engine-current-state` (rewritten to v8.1.4 / arc COMPLETE), `new-model-subagent-incompat`, `playtest-windowed-examples`, `conversation-language-korean`, `subagent-usage-preference`, `ci-toolchain-pin`.

## The Goal

Verify the just-shipped in-engine editor GUI actually works for its two stated purposes (drag/resize scene & UI layout; edit character/enemy/item data persisted to files), and fix every bug found. The session began as "report whether the requested GUI features were implemented well" and escalated, under an autonomous `/loop`, to "fix ALL bugs" — driven to convergence via repeated adversarial review sweeps. End state: a hardened editor where both requirements work, all found bugs fixed, shipped as three patch releases, fully merged.

## Where We Are

- **`main` = `86fc5e6`** (Merge #27), version **8.1.4**, working tree CLEAN. All work merged; no branches outstanding (3 feature branches deleted local+remote).
- **Three PRs merged:** #25 (v8.1.2, registration replay), #26 (v8.1.3, 6 editor-reliability), #27 (v8.1.4, 4 gizmo/inspector/mouse/parent). Each CI 4/4 green before merge; user gave explicit "머지 확인" per PR.
- **Bug class root cause (v8.1.2):** `App::set_scene` → `SceneCmd::Replace` → `reload_scene()` does `self.world = World::new()` and re-applies ONLY built-ins (`insert_core_resources` reflect/clone/serde for Transform/Sprite/Tag/UI widgets, `event_initializers`, `register_core_component_metadata`, `register_persistent` resources). Game registrations done BEFORE `set_scene` lived on the discarded world → silently lost.
- **v8.1.2 fix:** `App.world_registrars: Vec<EventHook>` records game registrations as replay thunks, re-run at the end of `reload_scene` (mirrors `event_initializers`); `load_data_table` marks `DataTableRegistry` `register_persistent`; `SerdeComponentEntry.post_spawn` changed `Box`→`Arc` so it can be replayed (no public sig change).
- **v8.1.3 fixes (6):** `EditorCmd::DeleteEntity` stores full `EntityDef` (undo via `spawn_entity_def`, restores ALL components incl. game `Stats`); `EditorCmd::CreateEntity` gained `def: Option<EntityDef>` so Duplicate/Paste are undoable; `do_load_scene` despawns ALL entities (was Transform-only → stale `UiNode` entities); `DataTableRegistry::reload_path` returns `ReloadOutcome` so the panel reports accurately; Save warns + counts dropped untagged-parent links.
- **v8.1.4 fixes (4):** `ui_resize_new_layout` applies anchor-base compensation (Center/non-TopLeft widgets no longer slide on resize); Inspector write-back matched by component NAME + guarded to the captured entity (was positional index → mismatch on mid-frame archetype/selection change); docked mouse-release no longer double-fires (`!allowed` guard); `entity_to_def` captures the parent link (was hardcoded `parent: None` → Undo/Duplicate of a child re-spawned it as root).
- **Test count:** 448 (start) → 450 (v8.1.2, +2) → 455 (v8.1.3, +5) → 456 (v8.1.4 sweep-2, +1) → 457 (v8.1.4 parent fix, +1). Plus new integration test `tests/editable_component_scene_replace.rs` (real `#[derive(Reflect)]` flow).
- **All gates green at each step:** `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo build --target wasm32-unknown-unknown` (0 warnings), `cargo test --all-targets`, `RUSTDOCFLAGS=-D warnings cargo doc`, `cargo package --locked` (CI-only gate — workspace publish unaffected).
- **Live playtest (GUI, automated):** game-data editor full loop verified (Stats inspectable+editable seeded `hp 30` from `enemies.ron`, Save→`saved_scene.ron` Stats=3, Data Tables panel shows enemies/items + cell-edit→Save→Reload round-trip); ui_layout editor full loop verified in the FIRST playtest (drag→Save→restart restores). HTML report at `/tmp/editor_playtest/report.html`.
- **Convergence:** 3 adversarial review sweeps, findings declined **6 → 4 → 1**; sweep-3 declared all remaining areas CONVERGED/CLEAN except the 1 parent-link bug (fixed). Stopped the loop on convergence (4th sweep judged near-certain-dry, not worth the cost).
- **Starting baseline:** main was at v8.1.1 (`efae8d4`) when the session began — v8.1.1 = macOS `WaitUntil` frame pacing (PR #24, already merged before this session); memory `engine-current-state` was stale at v7.0.0 and was corrected first thing.
- **Cargo workspace** (members `.` + `engine_reflect_derive`). `engine_reflect_derive` is a `[dev-dependencies]` (so `cargo package` works); the derive is `use engine_reflect_derive::Reflect` (NOT re-exported from `engine` — inherited constraint). The new integration test + the stat_editor example both use it directly.
- **rust-survivors consumer NOT touched this session.** v8.x is breaking from the game's pinned engine rev; the game pins by git rev and won't auto-update. Migration is the user's call (standing rule).
- **No `git tag`s created** — engine release tags lapsed after v4.3.0; don't tag unprompted (standing rule). v8.1.2/8.1.3/8.1.4 are version-bumped + CHANGELOG'd but not git-tagged.
- **Two earlier debugging files were created then removed** during the stale-artifact investigation: `tests/editable_repro.rs` (the diagnostic repro — deleted after converting to the clean integration test) and a temporary `eprintln` in `reload_scene` (reverted). Neither is in the final tree.

## What We Tried (Chronological)

1. **State report.** User asked "다음 예정된 작업?" — found memory (`engine-current-state`) stale at v7.0.0; actual main was v8.1.1. Confirmed editor-GUI arc + code-analysis-2 confirmed-8 fixes all merged; queues empty. Reported optional follow-ups (live playtest, derive ergonomics, rust-survivors v8 migration).
2. **Memory refresh + feature-verification report.** Updated memory v8.1.1; audited GUI features by code+tests: `cargo test --all-targets` = 448+ pass; confirmed all code paths exist. Reported both editor reqs as code/test-verified, live-playtest pending.
3. **Built a synthetic-input playtest harness.** Probed capability (idle 26s, screencapture OK, osascript window control OK — accessibility granted this time, unlike the parent's autonomous run). Compiled `/tmp/se_input` (Swift CGEvent helper: `key`/`click`/`move`/`drag`, later `ctrlz`). Drove examples via `caffeinate nohup` launch + osascript position + screencapture.
4. **ui_layout playtest — FULL PASS.** Default menu renders; F2 docked editor (all panels); select entity → Button+UiNode inspector + 8-handle gizmo; drag Start button → Save (`saved_scene.ron`, `UiNode offset:(-242,-176)`) → restart restores. End-to-end loop confirmed by eye.
5. **stat_editor playtest — FAIL (found the bug).** F2 → select Goblin → Inspector shows only Transform/Sprite/Tag, **no `Stats`**. Save → `grep Stats saved_scene.ron` = 0. Data Tables panel EMPTY. Traced root cause in code (reload_scene world reset drops game registrations). Wrote HTML report flipping Req2 to FAIL.
6. **`/loop` autonomous fix (governance: opus supervises, sonnet implements, 3× failure → stop, else opus judges).** Sonnet agent #1 implemented the v8.1.2 `world_registrars` replay + `register_persistent` + Arc post_spawn + 2 unit tests. Opus independently re-ran verify.sh (450 tests green).
7. **v8.1.2 re-playtest — initially looked still-broken (FALSE).** After the fix, the stat_editor GUI still showed no Stats. **Root cause of the false failure: cargo stale artifacts** — the non-tracked `target/debug/examples/stat_editor_game` binary was NOT rebuilt on engine source change (mtime predated the fix; `cargo build --example` reported "Finished 0.16s" without relinking). Wrote `tests/editable_repro.rs` mirroring the real flow → reproduced "0 comps" → instrumented `reload_scene` with eprintln → integration test showed NO debug line (stale lib) while lib unit test showed `world_registrars.len()=2` (works). `cargo clean -p skeleton-engine` → fresh build → `[3 after set_scene] has Stats? true`. **The fix was correct from the start; the harness was contaminated by stale binaries.**
8. **v8.1.2 verified + shipped.** Converted the repro into a clean integration test `tests/editable_component_scene_replace.rs` (real derive macro + on_enter spawn). Re-playtest on fresh binary: Stats in inspector (hp30/stamina100/str5/agi8), Save Stats=3, Data Tables enemies/items, cell-edit→Save→Reload round-trip. CHANGELOG 8.1.2, PR #25, CI 4/4, merged.
9. **Review sweep 1 (sonnet) → v8.1.3.** Adversarial hunt of editor + scene-reset surface returned 8 findings; opus verified each in code. Dismissed Track-1 #1/#2 (register_* are App methods, not callable from `on_enter` → unbounded-growth premise unreachable). Confirmed 6 editor-correctness bugs. Sonnet agent #2 fixed them (+5 tests). Opus verified (455 tests), CHANGELOG 8.1.3, PR #26, CI 4/4, merged.
10. **Review sweep 2 (sonnet) → v8.1.4 (part 1).** Second-angle sweep (gizmo math, RT lifecycle, Reflect write-back, serde round-trip, editor pause) returned 4 findings (1 Med + 3 Low). Opus verified all. Sonnet agent #3 fixed them (+1 test, 456). Note: a transient stale rust-analyzer `TypeId` diagnostic looked like a compile error — `cargo check`/verify.sh compiled clean (the code uses fully-qualified `std::any::TypeId`). CHANGELOG 8.1.4, PR #27, CI 4/4.
11. **Review sweep 3 (convergence, sonnet) → v8.1.4 (part 2).** Targeted the fixes' own correctness + `spawn_entity_def` round-trip + multi-select. Result: **CONVERGED** — 1 real bug only (`entity_to_def` hardcoded `parent: None`). Opus fixed directly (3-line + 1 test, 457) — too trivial for a 7th agent. Added to PR #27 (b26ba88), re-CI 4/4, merged. Declared convergence; stopped the loop.

## Key Decisions

- **`world_registrars` mirrors `event_initializers`, not a World-internal change.** Recording replay thunks on `App` and re-running them in `reload_scene` is the established pattern (events, persistent resources). Avoided touching `World::new()`/registry internals. `component_factories`/`component_removers` were left alone — they live on `App`, not the World, so they already survive resets (verified, not assumed).
- **`SerdeComponentEntry.post_spawn` Box→Arc** so the registration can be replayed (a `Box<dyn Fn>` can't be cloned for each replay). `Arc::from(box)` works for `?Sized`. No public signature change.
- **Bundle related fixes per unreleased version; new PR off `main` per merged version.** v8.1.3 sweep-1 fixes built on the v8.1.2 branch (because DeleteEntity-undo of game components depends on the v8.1.2 serde-survives-reset fix). After v8.1.2 merged, v8.1.3 went as its own PR off main; same for v8.1.4. Kept each PR scoped + independently CI-green for the user's per-PR merge gate.
- **`entity_to_def` is now the single capture path for undo/duplicate/paste/delete-undo** — so it MUST capture full entity state (components via serde registry, AND the parent link). The parent-link gap (sweep 3) was a direct consequence of #7's fix routing delete-undo through `entity_to_def`.
- **Anchor-base resize compensation via a shared `anchor_base` helper** (used by both `UiNode::screen_pos` and the gizmo) to prevent formula drift. TopLeft anchor has constant base → zero compensation → existing behaviour provably unchanged.
- **Stopped at convergence after 3 sweeps (findings 6→4→1).** A 4th sweep was judged near-certain-dry and not worth ~120k tokens; offered it to the user as optional rather than spending unprompted.
- **Never self-merge.** Each PR waited for the user's explicit "머지 확인" (the merge classifier checkpoint), even under autonomous `/loop`.

## Evidence & Data

### PR / commit ledger

| Release | PR | Feature commit(s) | Merge | CI | Δtests |
|---|---|---|---|---|---|
| 8.1.2 | #25 | `ad8ebc6` | `f14adb3` | 4/4 | 448→450 |
| 8.1.3 | #26 | `e6f1439` | `fbc681b` | 4/4 | 450→455 |
| 8.1.4 | #27 | `226e8fa` (gizmo/inspector/mouse), `b26ba88` (parent) | `86fc5e6` | 4/4 | 455→457 |

### Bugs fixed (11 total)

| # | Release | Bug | Severity |
|---|---|---|---|
| 1 | 8.1.2 | `register_editable_component`/`load_data_table` lost on `set_scene` world reset → Stats absent from Inspector + scene RON, Data Tables empty | High (headline feature dead) |
| 2 | 8.1.3 | DeleteEntity undo dropped non-core components (game `Stats`) | Medium (data loss) |
| 3 | 8.1.3 | Duplicate (⎘) not undoable | Medium |
| 4 | 8.1.3 | Paste (Ctrl+V) not undoable | Medium |
| 5 | 8.1.3 | Load Scene left stale `UiNode`-only entities → duplicates | Medium |
| 6 | 8.1.3 | Data Tables "Reload" reported success even when skipped (dirty-guard) | Low |
| 7 | 8.1.3 | Save dropped untagged-parent hierarchy links silently | Medium (edge) |
| 8 | 8.1.4 | `ui_resize_new_layout` slid non-TopLeft-anchored widgets | Medium |
| 9 | 8.1.4 | Inspector write-back paired fields by positional index → mismatch on mid-frame archetype/selection change | Low |
| 10 | 8.1.4 | Docked mouse-release double-fired / released without press | Low |
| 11 | 8.1.4 | `entity_to_def` hardcoded `parent: None` → Undo/Duplicate of child re-spawned as root | Medium |

### Review-sweep convergence

| Sweep | Angle | Findings (real) | Dismissed |
|---|---|---|---|
| 1 | editor + scene-reset registration class | 6 | Track-1 unbounded-growth (unreachable premise); by-design resources (`register_persistent`) |
| 2 | gizmo math, RT lifecycle, Reflect write-back, serde round-trip, editor pause | 4 (1 Med, 3 Low) | RtDebounce, texture pairing, viewport coords, hit-test, undo logic — all CLEAN |
| 3 | fixes' own correctness, `spawn_entity_def` round-trip, multi-select, `History<T>` | 1 (Med) | everything else CONVERGED |

### Playtest test-item matrix (from `report.html`)

`ui_layout_editor_game` (Req1 — scene/UI layout) — ALL PASS:

| # | Item | Result | Evidence |
|---|---|---|---|
| A1 | Default menu renders (title + 3 buttons + slider + checkbox) | PASS | shot 01 |
| A2 | F2 docked editor (toolbar / Entities·Scene / Inspector / Assets·Data Tables) | PASS | 03 |
| A3 | Select entity → Inspector Button+UiNode + add/remove | PASS | 05 |
| A4 | Screen-space 8-handle gizmo | PASS | 05 |
| A5 | Drag-move widget (gizmo follows; others fixed) | PASS | 06 |
| A6 | Save Scene → `saved_scene.ron` with moved `offset:(-242,-176)` | PASS | disk RON |
| A7 | Restart → moved layout restored | PASS | 07 |

`stat_editor_game` (Req2 — game-data) — FAIL on v8.1.1, all FIXED after v8.1.2/8.1.4:

| # | Item | v8.1.1 | After fix | Evidence |
|---|---|---|---|---|
| B1 | Enemy sprites render | PARTIAL (top-skew, window-size artifact) | — | 08 |
| B2 | F2 docked + named entities (Goblin/Orc/Skeleton) | PASS | PASS | 09 |
| B3 | Select → Inspector shows editable `Stats` | FAIL | **PASS** (hp30/stam100/str5/agi8) | 10 → 14,15 |
| B4 | Edit Stats → HP HUD updates | BLOCKED | PASS* (DragValue editable; HUD pixels not eyeballed due to B1) | 15 |
| B5 | Save → `Stats` serialized | FAIL | **PASS** (Stats=3, hp:30.0) | RON |
| B6 | Data Tables panel shows enemies/items + cell-edit/Save/Reload | FAIL | **PASS** (round-trip) | 11 → 16,17–20 |
| B7 | Table seeding / hot-reload | (fallback to defaults) | PASS (Goblin seeded hp30 from enemies.ron) | 15 |

### Playtest evidence (`/tmp/editor_playtest/`, not in repo)

- `report.html` — HTML test checklist + 24 screenshots, FAIL→FIXED for B3/B5/B6.
- Screenshots `01`–`24`: ui_layout (01–07 PASS), stat_editor broken (08–11), stat_editor fixed (12–16), delete/undo attempt (22–24), data-table edit (17–20), component-add (21).
- `enemies.ron` after editor cell-edit (Goblin hp 30→50, map-format RON) — confirmed Save writes disk + Reload re-parses (round-trip). Note: Save rewrites tuple-RON `(name:"Goblin",hp:30)` as map-RON `{"hp":50,...}` (alpha-sorted keys; "writes pretty RON, comments not preserved") — `DataTable::load` parses BOTH, so the round-trip is sound. NOT a bug.

### Rejected / by-design (verified — do NOT re-investigate)

These were flagged by the review sweeps and confirmed NOT bugs (or intentional). Recording so the next session/sweep doesn't re-chase them:

| Candidate | Verdict |
|---|---|
| `world_registrars` "unbounded growth if called in `on_enter`" | Unreachable — `register_editable_component`/`register_serde_component` are `App` methods; `on_enter` gets `&mut World` only, not `&mut App`. Setup is once-per-type. No growth. |
| `LocaleResource`/`AmbientLight`/`PostProcessConfig`/`FadeTransition`/`PhysicsWorld`/`AudioManager` lost on scene reset | By-design — per-scene World resources; cross-scene needs `register_persistent::<T>()` (the `settings_menu` example demonstrates). |
| `register_component`/`register_component_remover` not in `world_registrars` | Stored on `App` (not World) → survive reset. No issue. |
| Double-registration on reset (reflect/clone/serde) | `HashMap::insert` is idempotent overwrite. No duplicate entries/serialization. |
| `Events<E>` lost on reset | Correctly replayed via `event_initializers`. |
| `RtDebounce` off-by-one / texture leak / use-after-free / zero-size panic | All CLEAN (3-stable-frame logic correct; register/free paired; zero-size guarded). |
| `ui_drag_new_offset` move math, `hit_test_handles` corner disambiguation | CLEAN (algebraically equivalent; distinct corners at `HANDLE_HIT_RADIUS=8.0`). |
| Editor pause skips a needed system / single-frame step double-run | CLEAN (`HierarchySystem` runs in the builtin tail while paused; gizmo-moved parents propagate). |
| Prefab serde enum round-trip (`Anchor`/`TextAlign`/`LayoutDir`), duplicate-Tag parent (first-wins), `SCENE_DEF_VERSION` v2→v3 back-compat | All CLEAN/tested (enums stored as RON-string `ron::Value::String`, NOT `Map`). |
| Multi-select Delete/Duplicate acts only on `inspector_selected` | Intentional — no bulk-edit feature exists; `selected_entities` self-cleans via `retain(is_alive)`. |
| `History<T>` generic undo (`src/history.rs`) | CLEAN (snapshot swap correct; `record` clears redo). |

## Code Analysis

- `App.world_registrars: Vec<EventHook>` where `EventHook = Box<dyn Fn(&mut World)>` (src/app.rs ~192). Replayed in `reload_scene` (src/app/scenes.rs) after `register_core_component_metadata`, before preserved-resource re-insert — so the fresh `SerdeComponentRegistry` exists when serde thunks run. `std::mem::take`/restore idiom; thunks pushed only at registration time, never during replay → no growth.
- `register_serde_component` converts `Box`→`Arc` (`post_spawn.map(Arc::from)`), calls `do_register_serde_component::<T>(world, name, ps)` once for the live world + captures `(T, name, ps.clone())` into a replay thunk. `register_editable_component` additionally pushes a reflect+clone thunk.
- `DataTableRegistry::reload_path` now returns `enum ReloadOutcome { Reloaded, SkippedDirty, NotFound, Err }` (src/data_table.rs); the dirty-guard returns `SkippedDirty`; the hot-reload caller (schedule.rs) discards the return.
- `EditorCmd::DeleteEntity { entity: Option<Entity>, def: EntityDef }` (was tag/transform/sprite); `EditorCmd::CreateEntity { entity, def: Option<EntityDef> }`. Undo/redo preserve the `respawned`-id rebinding so redo despawns the exact recreated entity.
- `anchor_base(anchor, size, vw, vh) -> Vec2` (src/app/editor/ui/gizmo.rs, shared with `UiNode::screen_pos` in src/ui/node.rs). Resize compensation: `offset += anchor_base(start_size) - anchor_base(new_size)`.
- `entity_to_def` (src/app/editor.rs ~212): now resolves `world.get::<crate::hierarchy::Parent>(e).and_then(|p| world.get::<Tag>(p.0).map(|t| t.0.clone()))` for `parent`. `Parent(pub Entity)` in src/hierarchy.rs.
- **`reload_scene` exact order (src/app/scenes.rs)** — get this wrong and serde replay breaks: (1) snapshot `persistent_resources` + `DebugUi`; (2) `self.world = World::new()`; (3) `insert_core_resources` (inserts `SerdeComponentRegistry`, auto-registers widget serde, built-in reflect/clone); (4) replay `event_initializers`; (5) `register_core_component_metadata`; (6) **replay `world_registrars`** (game reflect/clone/serde — needs the registry from step 3 to exist); (7) re-insert `DebugUi`; (8) re-insert preserved resources LAST (so they win over defaults). `apply_scene_cmd(Replace)` calls `reload_scene()` BEFORE `new_scene.on_enter()` — so game registrations are live when on_enter spawns.
- **EditorCmd undo/redo respawned-id rebind (delicate — preserve):** undo of `DeleteEntity`/redo of `CreateEntity` spawns a NEW entity id; the code records that id back into the cmd (`respawned` var in `undo`, and `self.undo.push(CreateEntity{entity: e, def: def.clone()})` in `redo`) so the inverse op despawns the EXACT recreated entity, not the current selection. Any change to create/delete undo must keep this rebinding.
- **`ui_resize_new_layout` (src/app/editor/ui/gizmo.rs):** per-handle offset/size deltas, then `MIN_UI_SIZE` clamp (top/left handles push offset back by the excess so the opposite edge stays put), then the new anchor-base compensation. `ResizeHandle` enum order: TL,T,TR,L,R,BL,B,BR. `HANDLE_HIT_RADIUS = 8.0`.
- **Inspector write-back (src/app/editor/ui/mod.rs):** `comp_fields: Vec<(&'static str type_name, Vec<(&'static str field, ReflectValue)>)>` built ~line 144; `comp_fields_entity` captures which entity it was built for; write-back (~530) guards `comp_fields_entity == Some(sel)` then resolves name→TypeId via `reflect_registered_types()` ∩ `reflected_components(sel)`. `ReflectValue` variants: F32/I32/Vec2/Bool/String/Color (`#[non_exhaustive]`).
- **CI jobs (all 3 PRs, 4/4 each):** Build (WASM), Package dry-run (the long pole ~1m+, the one that caught the v8.1.0 workspace-publish bug), Rustdoc, Test (native). `verify.sh` runs everything EXCEPT Package dry-run (that's CI-only — run `cargo package --locked` locally when touching workspace/deps).

## Orchestration & Playtest Harness (reusable for next GUI session)

This is the first session where automated GUI playtest WORKED on this machine (parent's autonomous run had accessibility denied + display asleep). The reusable recipe:

- **Launch:** `nohup caffeinate -dimsu target/debug/examples/<name> >/tmp/log.txt 2>&1 &` (caffeinate keeps display/system awake; nohup detaches).
- **Wait without shell `sleep`** (foreground sleep is blocked): use `osascript -e 'delay 4'`.
- **Find + position window:** `osascript` `tell application "System Events" ... set position of window 1 of (first process whose name is "<name>") to {120,70}; set size to {1500,950}`. Window control needs Accessibility permission (granted this session). Position at a known origin so screenshot region maps 1:1.
- **Screenshot a region:** `screencapture -x -R 120,70,1500,950 shot.png`. Coordinate mapping: `screen_pt = (window_origin) + image_pt`, where image_pt is in the captured logical-point space. (The Read-tool sometimes scales the displayed image — a `[Image: original WxH, displayed at ..., multiply by K]` note means `image_pt = displayed × K / 2` since the PNG is 2× Retina.)
- **Synthetic input:** `/tmp/se_input` (Swift CGEvent helper). `key <code>` (F2=120), `click x y`, `drag x1 y1 x2 y2`, `ctrlz`. **WORKS for winit-level plain keys (F2) + mouse. Does NOT work for egui-internal modifier shortcuts (Ctrl+Z)** — neither CGEvent `.maskControl` flags nor `osascript keystroke "z" using control down` delivered the modifier to egui. Verify modifier-shortcut features via unit tests.
- **CRITICAL pre-playtest step after ANY engine change:** `cargo clean -p skeleton-engine`, then `cargo build --example <name>`, then assert `stat -f "%Sm" -t "%H:%M:%S" target/debug/examples/<name>` is NEWER than the source. Otherwise the stale non-tracked example binary silently runs OLD code and yields false results (this burned ~3 cycles).
- **Cleanup:** back up tracked assets (`enemies.ron`) before editor-Save tests; `git checkout --` to restore. `saved_scene.ron` is gitignored (safe). `pkill -f <name>` to stop.

## Autonomous /loop Orchestration (governance held)

- **opus supervises, sonnet implements** (6 sonnet agents total: 2 review sweeps that found bugs + 1 convergence sweep + 3 fix agents... precisely: fix #25, review-sweep-1, fix #26, review-sweep-2, fix #27-part1, convergence-sweep-3; the #27 parent-fix was done by opus directly — 3 lines, too trivial for a 7th agent).
- **opus NEVER trusted agent self-reports** — independently re-ran `verify.sh` before every commit. This caught nothing false in the fixes themselves but reaffirmed the discipline (and a stale-diagnostic false alarm was dismissed by `cargo check`).
- **Loop-until-dry:** kept sweeping until convergence (6→4→1 findings). Stopped when sweep-3 declared CLEAN-except-one; judged a 4th sweep near-certain-dry and not worth the token cost (offered to user instead).
- **Per-PR merge gate:** never self-merged; each PR presented CI-green and waited for the user's "머지 확인".
- **`/loop` dynamic mode:** each turn launched a background sonnet agent (harness-tracked → auto-wakes on completion) + a long-fallback `ScheduleWakeup` (1800s). Stopped the loop on convergence (omitted ScheduleWakeup; sent a PushNotification, suppressed since terminal focused).
- **Merge-while-agent-running (used twice — #25 and #26):** the user sent "머지 확인" while a sonnet agent was mid-edit on the SAME branch (working tree had broken/uncommitted WIP). Safe because GitHub merge operates on the PUSHED commit (`gh pr view <n> --json headRefOid` confirmed = the clean CI-green commit), NOT the local working tree. So: verify `headRefOid` == the pushed clean commit + CI 4/4, then `gh pr merge <n> --merge` (NO `--delete-branch` while local WIP exists on that branch). The agent's uncommitted WIP is untouched; after it completes, move it to a new branch off the (now-updated) main. The next version then PRs cleanly off main with just the new commits.
- **Version strategy across stacked work:** fold fixes into the SAME unreleased version while its PR is open (expand the CHANGELOG section, keep the version); once a PR merges, the next batch bumps to the next patch on a fresh branch off main. v8.1.3 sweep-1 (6 fixes) was one version; v8.1.4 absorbed both sweep-2 (4 fixes) AND the convergence fix (1) because #27 hadn't merged between them.

## Files Changed (all merged to `main`)

### Source
- `src/app.rs` — `world_registrars` field + init.
- `src/app/scenes.rs` — replay `world_registrars` in `reload_scene`.
- `src/app/editor.rs` — `register_editable_component`/`register_serde_component` replay thunks + `do_register_serde_component` helper; `EditorCmd::{DeleteEntity, CreateEntity}` def fields + undo/redo; `entity_to_def` parent capture; `load_data_table` `register_persistent`.
- `src/prefab.rs` — `SerdeComponentEntry.post_spawn` Box→Arc, `register_arc`.
- `src/data_table.rs` — `ReloadOutcome` return from `reload_path`.
- `src/app/editor/ui/docked.rs` — Delete captures def; Duplicate pushes CreateEntity{def}; `do_load_scene` despawns all; Save untagged-parent warn/count.
- `src/app/editor/ui/shortcuts.rs` — Paste pushes CreateEntity{def} for undo.
- `src/app/editor/ui/data_table_panel.rs` — accurate reload status from `ReloadOutcome`.
- `src/app/editor/ui/mod.rs` — Inspector write-back by component name + entity guard.
- `src/app/editor/ui/gizmo.rs` — `anchor_base` helper + resize compensation; signature gains anchor+viewport.
- `src/ui/node.rs` — `screen_pos` uses shared `anchor_base`.
- `src/app/window.rs` — docked mouse-release `!allowed` guard.

### Tests
- `tests/editable_component_scene_replace.rs` (NEW) — real derive `Reflect` + `register_editable_component` + `load_data_table` + on_enter spawn → set_scene survival (2 integration tests).
- `src/app.rs` + `src/app/editor.rs` test modules — `editable_component_survives_scene_replace`, `data_table_registry_survives_scene_replace`, `delete_undo_restores_full_def`, `create_entity_with_def_undo_redo`, `create_entity_no_def_undo_redo`, `resize_center_anchor_fixed_corner_preserved`, `entity_to_def_captures_parent_tag`; `src/data_table.rs` — `reload_path_skipped_when_dirty`, `reload_path_not_found`.

### Docs
- `docs/CHANGELOG.md` — `## 8.1.2`, `## 8.1.3`, `## 8.1.4`.
- `CLAUDE.md` — header v8.1.0→8.1.4.
- `Cargo.toml`/`Cargo.lock` — 8.1.1→8.1.4.

### Artifacts (NOT in repo — `/tmp`, ephemeral)
- `/tmp/editor_playtest/report.html` + screenshots 01–24.
- `/tmp/se_input` + `/tmp/se_input.swift` — CGEvent synthetic-input helper (rebuildable: `swiftc -O /tmp/se_input.swift -o /tmp/se_input`).

## User Feedback & Preferences (REQUIRED)

- "메모리 갱신하고, 내가 요청한 gui기능들 구현 잘 됐는지 정리해서 보고해줘" — wants memory kept current + a verification report on the GUI features they requested.
- "메모리 v8.1.1로 갱신하고 에디터 플레이테스트 진행해줘. **테스트 항목 html 로 만들어서 실행해줘**" — explicitly wanted a runnable HTML test-item checklist (delivered: `report.html`, opened in browser + sent as file).
- `/loop` directive (governance, verbatim): "**버그 모두 수정까지 진행. opus가 감독하고 sonnet 시켜서 실무 진행해. 3회이상 실패지점은 나에게 보고하고 정지. 이외의 판단은 opus한테 맡김.**" → opus supervises/verifies/commits; sonnet does implementation; report+stop only on a 3×-failure point; otherwise opus decides autonomously.
- "머지 확인" × 3 — the user gates EVERY merge with an explicit confirmation (never self-merge). Sent it once per PR after each was presented CI-green.
- Conversation in Korean; code/paths/file-docs in English (standing preference, [[conversation-language-korean]]).
- Implicit: values thoroughness ("모두" = ALL bugs) — drove the loop-until-dry multi-sweep approach.

## Where We're Going

1. **Editor bug-fix arc is COMPLETE and merged.** `main` @ v8.1.4, clean, queues empty. Next session = clean slate.
2. **Optional — 4th confirmation sweep.** Sweep 3 converged (1 finding, rest CLEAN); a 4th is near-certain-dry. Only run if the user wants absolute certainty.
3. **Optional — extend the hunt to other subsystems** (physics / renderer / networking) — this session deliberately scoped to the editor-GUI area (the recently-shipped, under-playtested surface). The user offered this as a possible direction.
4. **rust-survivors consumer not migrated.** v8.x is breaking from the game's pinned rev; the game pins engine by git rev and won't auto-pick-up. Migration per CHANGELOG 8.0.0 is the user's call (standing rule — not done this arc).
5. **New feature per VISION** — the default next move; user picks the genre/subsystem.

## Risks & Blockers

- **Synthetic Ctrl+Z does not reach egui.** F2 (winit-level plain key) works via synthetic CGEvent; Ctrl+Z (egui-internal `modifiers.ctrl` shortcut) does NOT — neither CGEvent flags nor `osascript keystroke ... using control down` delivered the modifier to egui. Undo/redo were verified via unit tests, not live keyboard. A human can eyeball Ctrl+Z manually.
- **cargo stale-artifact trap.** The non-tracked `target/debug/examples/<name>` binary is NOT reliably rebuilt on engine-source change (`cargo build --example` may report "Finished" without relinking). ALWAYS `cargo clean -p skeleton-engine` before trusting a GUI re-playtest after an engine change, and verify `binary mtime > source mtime`. lib unit tests (cfg-test build) were unaffected = the reliable signal.
- **Stale rust-analyzer diagnostics.** A phantom `cannot find type TypeId` error appeared mid-edit; `cargo check` compiled clean. Trust the compiler, never the IDE diagnostic snapshot.
- **Workspace publish is a 2-crate dance** (inherited): `engine_reflect_derive` is a dev-dependency so `cargo package` works; `verify.sh` does NOT run `cargo package` (CI-only) — reproduce with `cargo package --locked` when touching workspace/deps.

## Open Questions

- Does the user want a 4th confirmation sweep, or to extend bug-hunting to non-editor subsystems? (Offered at session close; awaiting direction.)
- rust-survivors v8.x migration timing — user's call.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3        # 86fc5e6 = #27 merge (v8.1.4); main clean
grep '^version' Cargo.toml   # 8.1.4

# Prior context
#   plans/handoffs/HANDOFF_editor-gui-arc_playtest-bugfix-sweep_2026-06-14.md  (this file)
#   plans/handoffs/HANDOFF_editor-gui-arc_docked-editor-to-data-editor_2026-06-13.md  (parent, the build)
#   docs/CHANGELOG.md  — 8.1.2 / 8.1.3 / 8.1.4

# Key files (editor lives here)
#   src/app/scenes.rs            — reload_scene world_registrars replay
#   src/app/editor.rs            — register_editable_component, EditorCmd undo/redo, entity_to_def
#   src/app/editor/ui/gizmo.rs   — anchor_base + resize
#   src/app/editor/ui/mod.rs     — Inspector write-back
#   src/data_table.rs            — ReloadOutcome
#   tests/editable_component_scene_replace.rs — the real-flow regression test

# Playtest harness (rebuild if needed)
#   swiftc -O /tmp/se_input.swift -o /tmp/se_input   # CGEvent helper
#   GUI re-playtest after engine change: cargo clean -p skeleton-engine FIRST, then build --example

# Verify current state
./scripts/verify.sh             # fmt, clippy -D, wasm build, test --all-targets (457 lib), doc
cargo package --locked          # CI-only gate (workspace/deps)

# Optional human eyeball (what the synthetic harness could NOT drive): Ctrl+Z undo.
#   cargo run --example stat_editor_game   # F2; select Goblin; 🗑 Delete; Ctrl+Z;
#   reselect Goblin → Stats must be back (hp 30). Then resize a Center-anchored
#   ui_layout_editor_game menu button — it must NOT slide (v8.1.4 anchor fix).

# Next action
#   Editor arc DONE + merged. Ask the user: new feature (VISION), extend bug-hunt to
#   physics/render/net, or 4th confirmation sweep. Default = new feature, user picks.
#   If extending the hunt: the "Rejected / by-design" table above scopes what NOT to re-check.
```

## Notes for the Next Session

- The editor (F1 overlay / F2 docked) is now well-hardened — 3 sweeps converged. If a NEW editor bug surfaces, check first whether it's a fresh code path or a regression of one of the 11 fixed bugs (all have regression tests; run `cargo test --lib editor_cmd_tests` + `cargo test --test editable_component_scene_replace`).
- If extending bug-hunting to physics/render/networking, those subsystems each have playable examples (per VISION) that can be playtested with the `/tmp/se_input` harness recipe above — but remember the `cargo clean -p` rule and that egui modifier shortcuts can't be driven synthetically.
- The user works in Korean (prose) / English (code+docs). They gate every merge with "머지 확인" and value thoroughness ("모두"). Under `/loop` they delegate implementation to sonnet and want opus to supervise + decide autonomously, reporting only on a 3×-failure point.

---

## Session Closed
**Closed at:** 2026-06-14 13:16 UTC
**Commit:** this `session: playtest-bugfix-sweep [editor-gui-arc]` commit (see `git log`)
**Session status:** Handed off to next session (chain `editor-gui-arc` seq 2; all engine work merged to `main` @ v8.1.4)
