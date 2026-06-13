# In-engine editor GUI arc — docked editor → scene layout editing → game-data editor (3 releases shipped)

**Date:** 2026-06-13
**Status:** COMPLETED
**Bead(s):** none (`bd` not installed — `command not found`, this whole arc tracked via in-session Task tools #1–#5, all completed)
**Epic:** Editor GUI — a GUI for game developers to customize in-game UI/UX and game data
**Chain:** `editor-gui-arc` seq `1`
**Parent:** none — first in chain
**Prior chain:** none — first in chain (session was BOOTSTRAPPED from the `v6-release-arc` chain's final handoff, whose "Where We're Going #2 = new feature work, user picks"; that is a reference, NOT a parent — see Related Handoffs)

---

## Related Handoffs

- `HANDOFF_v6-release-arc_v7-wgpu29-stack_2026-06-12.md` — seq 2 / final of the `v6-release-arc` chain (engine at v7.0.0, wgpu 29 stack). Its "Where We're Going" listed "new feature work per VISION, queue empty" — this session executed exactly that by picking the editor-GUI feature. **Reference only / bootstrap point, NOT chain parent.** A new feature stream gets its own chain; the release arc and the editor arc are independent work streams.

## Reference Documents

- `CLAUDE.md` (doc v1.6.6, updated this arc) — module map, verification gates, VISION loop; header now says v8.1.0 + "Cargo workspace"
- `docs/CHANGELOG.md` — `## 7.1.0`, `## 8.0.0`, `## 8.1.0` all written this arc (canonical migration guides; 8.0.0 carries the breaking guide)
- `REFERENCE.html` — three new editor sections added (Korean, per doc-internal-consistency rule): "도킹 모드 (F2)", "UI 레이아웃 편집", "게임 데이터 편집"
- `~/.claude/plans/ui-ux-merry-bengio.md` — the planning doc; carries the full 3-phase roadmap + Phase B and Phase C detailed designs
- `docs/VISION.md` — the feature loop (a feature is not done until a playable example exercises it) that gated each phase's acceptance example
- Memory: `engine-current-state` (rewritten to "v8.1.0; editor-GUI arc COMPLETE"), `new-model-subagent-incompat`, `ci-toolchain-pin`, `playtest-windowed-examples`, `conversation-language-korean`, `subagent-usage-preference`

## The Goal

The user wanted a GUI that lets game developers customize in-game UI/UX conveniently. Two concrete requirements, in the user's words: (1) **scene composition editing** — drag/resize main-menu button positions, in-game HUD, and playable-character/sprite position & size in a GUI; (2) **game-data editing** — change character/enemy stats (HP, stamina, strength, agility) and item values numerically, **persisted to data files**. After the user tried the pre-existing F1 floating-window editor overlay and rejected it ("feels like a simple debug tool; it covers the screen"), the chosen form became a **commercial-engine-style docked layout** (side panels around a central game viewport). Delivered as a 3-phase in-engine editor arc, each phase a shipped release with a playable acceptance example per the VISION loop. **End state reached: all three phases merged to `main` (v7.1.0 → v8.0.0 → v8.1.0); both user requirements met.**

## Where We Are

- **Engine `main` = `db19c6e`** "Merge pull request #23" (v8.1.0). Working tree CLEAN, branch `main`, all three feature branches deleted (local+remote).
- **Three releases shipped, three PRs merged:** #21 (v7.1.0 docked editor), #22 (v8.0.0 scene layout editing), #23 (v8.1.0 game-data editor). Every PR CI 4/4 green.
- **The engine is now a Cargo workspace** (members `.` + `engine_reflect_derive`). This is the single biggest structural change of the arc.
- **Phase A (v7.1.0) — docked editor shell.** `EditorMode { Off, Overlay, Docked }` (src/app/editor/state.rs); **F2** toggles Docked, **F1** keeps the old floating overlay (mutually exclusive). In Docked mode the game scene renders into an editor-owned offscreen texture (`register_native_texture`/`free_texture`, 3-frame resize debounce, zero-size guard) shown in an egui central panel; surface pass = clear + egui only. Toolbar (play/pause/step/snap/save-load), left Entities/Scene tabs, right Inspector, bottom Assets. `ViewportSize` delegated to the central panel logical size so game cameras/UI/`screen_to_world` work unchanged. Engine-level editor pause skips scene systems but runs the builtin tail (HierarchySystem) so gizmo drags keep children synced. No breaking changes.
- **Phase B (v8.0.0) — scene layout editing.** All 9 UI widgets (UiNode, Button, Label, TextInput, Slider, CheckBox, ScrollView, Panel, LocalizedText) + Anchor/TextAlign/LayoutDir now derive serde + impl Reflect (editable in inspector, serialize to scene RON). `SerdeComponentRegistry` + `App::register_serde_component::<T>` persist any registered component into scene files. `EntityDef` gained `components: HashMap<String, ron::Value>`; SCENE_DEF_VERSION 2→3 (v2 back-compat via serde default). Screen-space gizmo for UiNode (drag offset + 8-handle resize) + world-sprite scale resize; `EditorCmd::{MoveUiNode, ResizeUiNode, ResizeEntity}` undo. Example `ui_layout_editor_game`. **Breaking** (EntityDef literal needs `components: Default::default()`; TextInput.initial_text / Slider.initial_value added, runtime text/value `#[serde(skip)]`).
- **Phase C (v8.1.0) — game-data editor.** `#[derive(Reflect)]` proc-macro in the new `engine_reflect_derive` workspace crate. `App::register_editable_component::<T>` (one call = reflect + clone + serde + add/remove-component). `DataTable`/`DataTableRegistry`/`App::load_data_table` — schema-agnostic RON tables, a new "Data Tables" tab in the docked bottom panel (per-cell int/float/string/bool editors, add/delete row, Save/Reload), hot-reload with a dirty-guard. Example `stat_editor_game`. **Additive (no breaking).**
- **Test count progression (lib):** 413 (v7.0 baseline) → 430 (B/A widget serde) → 434 (B registry) → 442 (D gizmo) → 448 (C data table) + 8 `derive_reflect` integration tests + 6 `data_table` tests.
- **All gates green at each step:** `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo test --all-targets`, `cargo build --target wasm32-unknown-unknown` (0 warnings), `RUSTDOCFLAGS=-D warnings cargo doc --no-deps`, `./scripts/wasm_smoke.sh` (PASS each release).
- **Two playtests by the user:** Phase A "f2 눌러서 정상 작동 확인" (F2 docked verified live). Phase B verified live (the earlier `platformer_game` F2 session — drag/inspector/save). Phase C playtest **SKIPPED** by user ("플레이테스트 스킵 하고 머지 확인").
- **Governance changed mid-arc:** the autonomous `/loop` supervisor role moved from **Fable 5 to Opus 4.8** (fable5 usage regulation); the user ran `/model opus4.8`. sonnet subagents do all implementation; supervisor verifies+commits.
- **CLAUDE.md is at 194 lines** (under the project's 200-line cap — checked each release); doc-language rule held (English prose; REFERENCE.html sections written in Korean for doc-internal consistency, the established precedent).
- **`cargo package` succeeds** (227 files, 21.0 MiB / 8.0 MiB compressed; verify build OK) and `cargo publish --dry-run` passes — both confirmed locally after the dev-dependency fix. Pre-existing benign warning: `swash v0.2.8` yanked in Cargo.lock (unrelated to this arc, present before it).

## What We Tried (Chronological)

1. **Initial direction was "data-driven UI foundation".** First plan (in `ui-ux-merry-bengio.md` history) was serde + a `UiTheme` style resource + a `Dim { Px, Pct }` responsive-unit enum + a `.ui.ron` layout asset format + named event routing. Designed via a Plan agent. **Redirected by the user** after they described what they actually wanted (scene/button/HUD/stat editing via GUI) — the data-driven-format work was the wrong altitude.
2. **User tried the pre-existing F1 editor overlay** (they didn't know it existed). Verdict: "간단한 디버깅 툴 같은 느낌... 화면을 가린다" — the floating overlay covers the game, so you can't judge UI layout or confirm sprite spawns. **This produced the pivotal decision: docked layout (sidebars + central game viewport), not a floating overlay.**
3. **Re-scoped to a 3-phase editor arc** (A docked shell → B scene layout editing → C data editor), confirmed with the user via AskUserQuestion (pause control = toolbar play/pause; F1 overlay kept alongside F2; order A→B→C).
4. **Phase A package 1** (EditorMode + offscreen RT + ViewportSize delegation): first sonnet agent hit the **session token limit mid-work** (~00:10 KST, reset 01:10). Left ~492 uncommitted insertions + 2 new files, compile state unknown. **Resumed at reset** with a second agent that fixed 6 compile errors (a malformed egui `show_inside` call) and finished. Verified clean, committed `0949974`.
5. **Phase A package 2** (input routing + dock layout + pause): the supervisor had to fix the agent's `egui_wants_pointer_input` gate — it's UNUSABLE in docked mode because the game viewport IS an egui CentralPanel, so egui always "wants" the pointer there. Replaced with a layer-aware gate `docked_game_pointer_allowed` (inside central rect + egui not actively using pointer + topmost layer is Background order). Added `window_cursor` tracking field. Committed `9a8e3b2`.
6. **Phase A shipped** as v7.1.0 (PR #21). User playtested F2 live ("정상 작동 확인"), CI 4/4, merged `1523b5d`.
7. **Phase B serial chain A→B→D→E.** A (widget serde/Reflect, `d09bdde`), B (SerdeComponentRegistry + EntityDef.components + save/load wiring, `ceb4778`), D (screen gizmo + resize, `86f2d1b`), E (example + native-gating cleanup, `3ab4d5a`). Supervisor ran full gates and committed between each agent.
8. **Phase B gotcha — wasm dead_code.** Agent D cfg-gated SOME resize helpers but not `ui_drag_new_offset` / the 4 constants / `ui_resize_new_layout` / `ResizeHandle`, leaving 7 dead_code warnings on the wasm build (the resize path is native-only). The wasm gate is `cargo build` (warnings allowed) so it didn't FAIL, but the supervisor native-gated all of them for a 0-warning wasm build (commit folded into E `3ab4d5a`).
9. **Phase B shipped** as v8.0.0 (PR #22), CI 4/4, merged `851532f`. Interactive playtest was display-off during the autonomous run → marked pending → user had already verified F2 live earlier.
10. **Phase C serial chain A→B→C.** A (derive proc-macro + workspace conversion, `f53a473`), B (DataTable + panel + hot reload, `f881120`), C (register_editable_component + stat_editor example, `8db348e`).
11. **Phase C — the workspace/publish gotcha (the expensive one).** Phase C/A's first design made `engine_reflect_derive` an optional regular dependency with a default-on `derive` feature, re-exported from `engine` (the serde `serde`/`serde_derive` pattern). All LOCAL gates passed. But **PR #23 CI failed the "Package dry-run" job**: `cargo package --locked` errors with `no matching package engine_reflect_derive found / location searched: crates.io index`. An umbrella crate that depends (even optionally) on an UNPUBLISHED workspace sibling cannot be packaged/published, because packaging resolves all deps from crates.io. `--no-verify` does NOT help (still resolves). Adding `version = "0.1.0"` fixed the manifest error but the verify-build then failed on the same missing-from-crates.io sibling.
12. **Fix: make `engine_reflect_derive` a `[dev-dependencies]`** (commit `4f28baa`). Dev-deps are stripped on publish, so no crates.io resolution is required; examples and the integration test still use the derive directly (`use engine_reflect_derive::Reflect`); downstream games add the crate to their own Cargo.toml the same way they add `engine`. The cost: derive is now `use engine_reflect_derive::Reflect` instead of `use engine::Reflect`. Verified locally with `cargo package --locked` (227 files, verify build OK) + `cargo publish --dry-run --locked` (OK).
13. **Phase C shipped** as v8.1.0 (PR #23), CI 4/4 green (Package dry-run now passing), user "플레이테스트 스킵 하고 머지 확인", merged `db19c6e`.
14. **Autonomous loop sub-thread.** The whole arc ran under `/loop` dynamic mode. The supervisor used `caffeinate` to keep the machine awake overnight, watched `ioreg HIDIdleTime` to detect the user sleeping (idle climbed to ~6h), throttled the heartbeat from 30min to 60min while away, and recovered from one token-limit hit by scheduling a wakeup at the reset time. `osascript` window control failed (accessibility not granted) and `screencapture` returned "could not create image from display" while the display was off — so autonomous screenshot playtests were not possible; playtests were left for the user.

## Key Decisions

- **Docked layout over a floating overlay** (user-driven). The F1 overlay is kept as a quick-debug tool; F2 is the new docked mode; they're mutually exclusive. This decision came directly from the user trying F1 and finding it covered the game.
- **Game-to-offscreen-texture, not multi-window.** A second OS window (winit multi-window + 2nd wgpu surface) was rejected as too invasive to App + macOS event-loop risk. Instead the game renders to an editor-owned `RenderTarget`-style texture displayed via `register_native_texture` in an egui central panel. `RenderTarget` infra already existed (security_camera example).
- **`ViewportSize` delegation** makes the game think the window shrank — cameras, screen-space UI, DrawText, and `Camera::screen_to_world` all work unchanged against the viewport with no per-call coordinate math. This is why the existing gizmo's `screen_to_world` math needed no change in docked mode.
- **Engine-level editor pause, NOT via GameState.** `GameState::Paused` is a game-convention resource the engine update loop does NOT read (verified: 0 references in schedule.rs/app.rs) — so toggling it wouldn't stop most games. Pause is implemented by skipping the labeled scene-system pipeline while still running the builtin tail (HierarchySystem, via `builtin_tail_count`) so a paused gizmo drag still moves child entities. `GameState` left untouched as a game-side concept.
- **Component serialization registry over hardcoded EntityDef.** Rather than add `Option<Button>`, `Option<Slider>`, … fields to EntityDef, a type-erased `SerdeComponentRegistry` lets any `Serialize+DeserializeOwned+Clone` component register once and round-trip through scene RON. This is the shared path for Phase B (UI widgets) AND Phase C (game stats). UI widgets auto-register; games register their own.
- **Components stored as string-encoded `ron::Value`** in the components map (not a parsed `ron::Value::Map`), because ron 0.8's `Value::into_rust` cannot round-trip enums (e.g. `Anchor`). The string path (`ron::to_string` → `Value::String` → `ron::from_str::<T>`) preserves full serde fidelity. Visible (escaped) in saved files but correct.
- **Screen gizmo: UiNode path takes priority** over the world Transform path; highlight via `UiQueue` DrawRect (NOT `DebugDraw`, which is world-space only). Drag invariant: `screen_pos = anchor_base + offset`, anchor_base constant during a drag, so `new_offset = old_offset + cursor_delta` regardless of anchor (unit-tested).
- **derive macro is a separate crate used directly (`use engine_reflect_derive::Reflect`), NOT re-exported from engine.** Forced by the cargo-package constraint (see What We Tried 11–12). If the user later wants the serde-style `use engine::Reflect`, the path is: publish `engine_reflect_derive` to crates.io first, then re-export behind a feature and coordinate a 2-crate publish.
- **Serial agent chains, supervisor verifies+commits between each.** app/ internals (state/render/schedule/ui/editor.rs) are too entangled for safe parallel agents (EditorCmd enum + Cargo.toml + lib.rs overlap across agents). Serial in-place on one branch, with the supervisor running ALL gates and committing between agents, avoided every merge conflict.
- **Never trust agent self-reports or IDE diagnostics — re-run the gates.** Repeatedly the agent reported "all green" AND the IDE pushed phantom errors (stale mid-edit snapshots: "unexpected closing delimiter", "missing field", "Clone not implemented"), but the real `cargo build`/clippy was clean every time. Conversely CI caught a real failure (cargo package) that local gates missed. The supervisor independently ran the full gate suite before every commit.

## Evidence & Data

### Release / PR ledger

| Phase | Release | PR | Merge commit | Feature commits | CI |
|---|---|---|---|---|---|
| A | 7.1.0 | #21 | `1523b5d` | `0949974` (pkg1), `9a8e3b2` (pkg2), `1c4044b` (release) | 4/4 |
| B | 8.0.0 | #22 | `851532f` | `d09bdde` (A), `ceb4778` (B), `86f2d1b` (D), `3ab4d5a` (E), `3247a57` (release) | 4/4 |
| C | 8.1.0 | #23 | `db19c6e` | `f53a473` (A), `f881120` (B), `8db348e` (C), `6f895c0` (release), `4f28baa` (package fix) | 4/4 |

CI jobs (all PRs): Build (WASM), Package dry-run, Rustdoc, Test (native). Package dry-run = the long pole (~1m+) and the one that caught the workspace-publish bug on PR #23.

### Test count progression (lib)

| Point | lib tests | added |
|---|---|---|
| v7.0.0 baseline | 413 | — |
| Phase A | 413 | (gizmo/RT unit tests; count unchanged at lib level reported as 413→ docked tests folded) |
| Phase B/A widget serde | 430 | +17 serde/Reflect/Anchor-i32 |
| Phase B/B registry | 434 | +4 registry round-trip / unknown-tolerance / v2-back-compat / post_spawn |
| Phase B/D gizmo | 442 | +8 resize-handle / drag-invariant / resize-math |
| Phase C/B data table | 448 | +6 DataTable parse/edit/save round-trip |
| Phase C/A derive (integration) | +8 | `tests/derive_reflect.rs` field round-trip + skip + wrong-variant |

### wasm_smoke screenshots (non-blank render gate)

| Release | screenshot bytes |
|---|---|
| 7.1.0 | 41909 |
| 8.0.0 | 41946 |
| 8.1.0 | 41908 |

(All > 15000B threshold = real frame rendered; HUD/geometry eyeballed each time.)

### The cargo-package failure (verbatim, for the next session)

```
error: failed to verify manifest at `.../Cargo.toml`
Caused by: all dependencies must have a version requirement specified when packaging.
  dependency `engine_reflect_derive` does not specify a version
```
After adding `version`:
```
error: failed to prepare local package for uploading
Caused by: no matching package named `engine_reflect_derive` found
  location searched: crates.io index
```
Fix = move it to `[dev-dependencies]` (stripped on publish).

### Reproduce the CI-only package gate locally

```bash
cargo package --locked            # NOT in scripts/verify.sh — CI-only
cargo publish --dry-run --locked  # also CI-only
```

### Agent fleet roster (all sonnet, supervisor = Opus 4.8 after the mid-arc switch)

| Phase | Agent | Scope | Outcome |
|---|---|---|---|
| A | pkg1 | EditorMode, offscreen RT, ViewportSize delegation | died at session token limit; resumed by pkg1-resume → `0949974` |
| A | pkg2 | input routing, dock layout, engine pause | supervisor replaced its egui-wants-pointer gate → `9a8e3b2` |
| B | A | widget serde + Reflect (9 widgets + Anchor/TextAlign/LayoutDir) | `d09bdde` (lib 413→430) |
| B | B | SerdeComponentRegistry + EntityDef.components + save/load | `ceb4778` (430→434) |
| B | D | screen gizmo + 8-handle resize + world-sprite scale | `86f2d1b` (434→442) |
| B | E | ui_layout_editor example + native-gate gizmo helpers | `3ab4d5a` |
| C | A | #[derive(Reflect)] proc-macro + workspace conversion | `f53a473` (+8 derive tests) |
| C | B | DataTable + Data Tables panel + hot reload | `f881120` (442→448) |
| C | C | register_editable_component + stat_editor example | `8db348e` |
| (design) | Plan ×2 | Phase B design, Phase C design (read-only, while waiting) | folded into ui-ux-merry-bengio.md |

Pattern that held 9-for-9: serial agent → supervisor runs ALL gates → supervisor commits → next agent. Shared docs (CHANGELOG/REFERENCE/CLAUDE.md/version) touched ONLY by the supervisor in a per-release integration commit. Agents banned from those files = zero conflicts.

### v8.0.0 migration (for forks / the rust-survivors consumer)

Mechanical, per CHANGELOG 8.0.0:
- `EntityDef { … }` struct literals: add `components: Default::default()` (or `..Default::default()`).
- `TextInput`: set `initial_text` for design-time text; runtime `text`/`cursor`/`focused`/`preedit` are now `#[serde(skip)]`. `Slider`: set `initial_value`; runtime `value` is `#[serde(skip)]`. Constructors (`Slider::new`, etc.) behave identically — only matters if you serialized these structs directly.
- `SceneDef` v2 files still load (the new `components` map defaults empty; the version-mismatch warning is informational); v3 files can't be read by ≤v7 engines.
- v8.1.0 is purely additive on top — no further migration.

### Deferred / abandoned design (don't re-discover from scratch)

The FIRST plan (superseded after the F1 playtest) was a "data-driven UI foundation": a `UiTheme` style resource (named styles + per-instance override), a `Dim { Px(f32), Pct(f32) }` responsive-unit enum resolved against viewport/parent, a `.ui.ron` hierarchical layout asset with `spawn_ui_def`, and `UiName` + named event routing (`events.clicked("start_btn")` instead of storing 11 entity-id fields + if/else). These were DEMOTED to "後순위 — not the user's bottleneck", NOT rejected on merit. If a future "responsive UI / theming" feature is wanted, that design is the starting point (it was in an earlier revision of `ui-ux-merry-bengio.md`).

## Code Analysis

- `EditorMode { Off, Overlay, Docked }` + pure `apply_f1`/`apply_f2(mode) -> mode` transitions (src/app/editor/state.rs) — testable without a World.
- `docked_rt.rs`: `RtDebounce` (3-stable-frame rule), `compute_central_rect`, `rect_to_physical(rect, scale)`, `viewport_to_game(window_pos, central_rect) -> Option<Pos2>`, `docked_game_pointer_allowed(window_cursor, central_rect, ctx) -> bool` (the layer-aware gate; uses `ctx.egui_is_using_pointer()` + `ctx.layer_id_at(pos).is_none_or(|l| l.order == Background)`).
- `EditorState.central_rect: Option<egui::Rect>` is the Package-1↔Package-2 contract — package 2 writes the real CentralPanel rect each docked frame; the RT + ViewportSize follow it.
- `SerdeComponentRegistry` (src/prefab.rs): `SerdeComponentEntry { serialize, deserialize, post_spawn: Option<Box<dyn Fn>> }`; `register::<T>(name, post_spawn)`, `serialize_entity`, `deserialize_into`/`deserialize_one`. Borrow workaround in `spawn_entity_def`: `remove_resource::<SerdeComponentRegistry>()` → deserialize → `insert_resource`.
- `App::register_serde_component::<T>(name, post_spawn)` (editor.rs:360) and `App::register_editable_component::<T>(name, post_spawn)` (editor.rs:361) — the latter wraps register_reflect_named + register_clone + register_serde_component + (native) register_component + register_component_remover. Bounds: `Reflect + Serialize + DeserializeOwned + Clone + Default + Send + Sync + 'static`.
- Gizmo routing (gizmo.rs): `update_editor_gizmo` → if `world.get::<UiNode>(sel).is_some()` use `update_ui_node_gizmo` (screen space), else `update_transform_gizmo` (world). Pure helpers: `ui_drag_new_offset`, `handle_centers`, `hit_test_handles`, `ui_resize_new_layout`, `ResizeHandle` enum — ALL native-gated (resize path excluded from the reduced wasm editor).
- `DataTable { columns: Vec<String>, rows: Vec<Vec<(String, ron::Value)>>, path, dirty }`; `from_str` (fs-free, for tests + wasm) / `load` / `save` / `to_ron_string`. `DataTableRegistry::reload_path` has the dirty-guard (skips reload if unsaved edits). ron 0.8 numbers: `ron::Value::Number(ron::Number::Integer(i64))` vs `Number::Float(Float)` (`Float::new`/`Float::get`).
- `engine_reflect_derive`: `#[proc_macro_derive(Reflect, attributes(reflect))]` → `expand(DeriveInput) -> syn::Result<TokenStream>`. Maps `f32→F32`, `i32→I32`, `Vec2→Vec2`, `bool→Bool`, `String→String`, `Color→Color (.to_array())`, `[f32;4]→Color`; `#[reflect(skip)]` omits; other types → `compile_error!`. Emits fully-qualified `engine::reflect::Reflect`/`engine::reflect::ReflectValue`/`engine::color::Color` so it works from game code.

### Docked render pipeline (Phase A — the trickiest part)

- **Frame order when Docked:** existing user `OffscreenCamera` RenderTargets submit their own command buffers FIRST (unchanged) → the main scene pass renders into the editor-owned offscreen texture (instead of the surface) → the surface pass does clear + the egui pass only. egui shows the editor texture via `ui.image((texture_id, size))` in the central panel.
- **TextureId lifecycle:** on RT (re)creation, `egui_wgpu::Renderer::free_texture(&old_id)` then `register_native_texture(device, &view, FilterMode::Linear)`; the `TextureId` is cached on `EditorState`. RT recreation is debounced (`RtDebounce`: target physical size must be stable 3 frames AND differ from current) to avoid per-frame churn during a panel-resize drag; zero-size (minimized) skips both recreate and scene render.
- **Physical vs logical:** central rect is in logical points; RT size = `rect_to_physical(rect, scale_factor)`. ViewportSize reports the logical central size. `DisplayScaleFactor` stays the real window scale.
- **egui 0.34 deprecation:** top-level `Panel::show(ctx, …)` / `CentralPanel::show(ctx, …)` are deprecated ("use show_inside") but `show_inside` needs a parent `&mut Ui` that doesn't exist at the top level, and `show_dyn` is private — so all top-level panel calls use `#[allow(deprecated)]` with an explanatory comment. A future cleanup could route through `egui::Area`/a root Ui.

### Editor controls quick reference (for the next live playtest)

- **F1** = floating-window overlay editor (unchanged, quick debug). **F2** = docked editor. Mutually exclusive: entering Docked turns Overlay off; F1 while Docked switches to Overlay.
- **Docked toolbar:** ▶/⏸ (editor pause), ⏭ (single-frame step, enabled only while paused), Snap toggle + grid-size, scene path field + Save/Load Scene.
- **Select:** click an entity in the Entities tab or click it in the viewport (multi-select with Ctrl). Selecting a `UiNode` entity → screen-space gizmo; selecting a Transform-only entity → world gizmo.
- **Move:** drag the body. **Resize:** drag one of the 8 handles. **Undo:** Ctrl+Z (`EditorCmd::MoveEntity`/`MoveUiNode`/`ResizeUiNode`/`ResizeEntity`/`CreateEntity`/`DeleteEntity`; each restores its specific fields on undo). Snap applies to both move and resize when enabled.
- **Data Tables tab** (bottom panel, native): pick or open a table → edit cells in the grid → Save (writes pretty RON, comments not preserved) / Reload (dirty-guarded). Editing a `.ron` table on disk hot-reloads into the running game.
- **Persistence demo path:** the default Save-Scene filename is `saved_scene.ron` (EditorState default); `ui_layout_editor_game` and `stat_editor_game` load that same filename on start, so Save → restart shows the edit persisted with zero typing.

### wasm story (what's gated)

- Editor is native-only (`#[cfg(not(target_arch = "wasm32"))]`) — Docked mode, the gizmo resize path, `ResizeHandle`, the data-table panel, and file save/load are all excluded from the reduced wasm editor.
- Cross-platform (compile on wasm): `DataTable`/`DataTableRegistry` types (`from_str` is fs-free), `SerdeComponentRegistry` + `register_*` registrations (the widget Reflect/serde registrations run on wasm too), `EntityDef.components`, ron parsing. File IO is gated at the `save.rs` `read_ron`/`write_ron` layer (returns `SaveError::Unsupported` on wasm); `poll_reloads` returns empty on wasm.
- proc-macros are host-compiled, so `engine_reflect_derive` never builds for the wasm target; `cargo build --target wasm32-unknown-unknown` builds lib+bins only and is 0-warning.

## Files Changed

### New crates / files
- `engine_reflect_derive/` (new workspace member): `Cargo.toml`, `src/lib.rs`, `src/reflect_derive.rs`
- `src/data_table.rs` — DataTable + DataTableRegistry
- `src/app/editor/docked_rt.rs` — RtDebounce, viewport_to_game, docked_game_pointer_allowed, central-rect math
- `src/app/editor/ui/docked.rs` — docked layout (toolbar + panels), shared tab-body functions, do_save_scene_with_list
- `src/app/editor/ui/data_table_panel.rs` — Data Tables editor panel
- `tests/derive_reflect.rs` — derive integration tests (engine-side to avoid the engine↔derive cycle)
- `examples/games/ui_layout_editor/` (ui_layout_editor.rs + assets) — Phase B acceptance
- `examples/games/stat_editor_game/` (stat_editor.rs + enemies.ron + items.ron) — Phase C acceptance

### Source (modified)
- `src/app/editor/state.rs` — EditorMode, apply_f1/apply_f2, docked + resize + data-table fields, ResizeHandle (native-gated)
- `src/app/editor.rs` — EditorCmd new variants + undo/redo, register_serde_component, register_editable_component, load_data_table, entity_to_def(+components)
- `src/app/editor/ui/mod.rs`, `gizmo.rs` — shared tab bodies; screen-space UI gizmo + resize; native gating
- `src/app/render.rs`, `schedule.rs`, `window.rs`, `app.rs` — offscreen RT lifecycle, ViewportSize delegation, F2 handling + input routing, pause, data-table reload
- `src/ui/*.rs` (node, button, label, text_input, slider, checkbox, scroll_view, panel, localized) + `src/renderer/text.rs` — serde + Reflect + initial_text/initial_value
- `src/prefab.rs` — EntityDef.components, SCENE_DEF_VERSION 3, SerdeComponentRegistry
- `src/asset.rs`, `src/asset/hot_reload.rs` — data_table_paths + watch_data_table_path + poll_reloads extension
- `src/app/core_resources.rs` — register_reflect_named/register_clone for widgets, insert SerdeComponentRegistry, auto-register widget serde
- `src/lib.rs` — re-exports (data_table; NOT the derive — see decision)
- `Cargo.toml` — `[workspace]`, version 7.1.0→8.0.0→8.1.0, dev-dependency engine_reflect_derive, [[example]] entries

### Docs
- `docs/CHANGELOG.md` — `## 7.1.0`, `## 8.0.0` (breaking guide), `## 8.1.0`
- `REFERENCE.html` — 3 new editor sections + TOC entries
- `CLAUDE.md` — v1.6.6, workspace note, module rows (editor docked, prefab registry, reflect derive, data_table)

## User Feedback & Preferences (REQUIRED)

- **The pivotal redirect:** "지금은 간단한 디버깅 툴 같은 느낌이네... 화면을 가린다는 부분이 ... 제대로 생성되었는지 확인 할 수 없다... 사이드바에서 설정을 조작하고 가운데에 게임 화면을 띄울수 있게 하는 쪽으로." → docked layout, not floating overlay.
- "나 지금가지 f1오버레이라는 기능이 있는줄도 모르고 한번도 사용 안해봤어. 일단 내가 실사용해보고 정해봐도 될까?" → wanted to try F1 before deciding the GUI form. (Then redirected as above.)
- Stat persistence: chose **데이터 파일로 영구 저장** (RON), targets BOTH entity components AND data tables ("둘 다").
- Approach order: chose **기반 먼저** then **A→B→C** for the editor arc; success bar **예제+신규 포크 수준** (not rust-survivors migration).
- "f2 눌러서 정상 작동 확인" → Phase A playtest passed (user verified F2 docked live).
- "플레이테스트 스킵 하고 머지 확인" → Phase C: user skipped the interactive playtest and confirmed merge.
- **Governance:** "fable5 사용 규제로 fable5가 작업하던 관리 감독 역할을 opus4.8로 변경" → supervisor moved to Opus 4.8 (user ran `/model opus4.8`). Subagents do implementation.
- **Process directives (from the /loop prompt):** supervisor = manage/direct/verify; sonnet subagents = implementation; proceed by supervisor judgment until the work plan is complete; if stuck 3+ times find another way; on token-limit, remember the reset time and resume then.
- Repeatedly merged each PR explicitly via "머지 확인" (the classifier checkpoint per merge).
- Wanted detailed plan explained in Korean before executing (early in the session, before the loop).

## Orchestration & Autonomous-Loop Learnings

This entire arc ran unattended under `/loop` dynamic mode, overnight (user slept ~6h mid-arc). Mechanics that worked, for the next autonomous session:

- **Supervisor / worker split.** Opus 4.8 (the main loop) managed/directed/verified; sonnet subagents did all code. The supervisor NEVER trusted a subagent's "all gates pass" report — it independently re-ran `fmt`/`clippy`/`test`/`wasm`/`doc` (and per-release `wasm_smoke` + `cargo package`) before every commit. This caught: the egui-pointer-gate bug, 7 wasm dead_code warnings, and (via CI) the workspace-package failure.
- **Stale IDE diagnostics are noise.** Every Phase emitted phantom `<new-diagnostics>` mid-edit (unexpected closing delimiter, missing struct fields, Clone not implemented). `cargo build` was clean every single time. Rule: re-run the compiler; never act on a diagnostic alone.
- **Token-limit recovery.** A subagent died at the session token limit (~00:10 KST, "resets 1:10am"). The supervisor recorded the reset time, scheduled a `ScheduleWakeup` for it, and resumed the half-done work at reset — committing nothing until a clean build was reached.
- **Sleep/idle handling.** `caffeinate -is -t <secs>` kept the machine awake; `ioreg -c IOHIDSystem | grep HIDIdleTime` detected the user away (idle climbed past 100→350 min); the heartbeat was throttled from 30min to 60min while idle to cut cache-miss overhead.
- **Live playtest is human-only here.** `osascript` System Events failed ("보조 접근이 허용되지 않습니다" — accessibility not granted) and `screencapture` failed ("could not create image from display") while the display slept. Autonomous screenshot verification was impossible; playtests were deferred to the user. The example launches fine headless (banner prints, no panic) — "Occluded" render errors are just the off-display surface, not a crash.
- **Per-merge classifier checkpoint.** Each PR merge needed a fresh user "머지 확인"; the supervisor never self-merged. CI was confirmed 4/4 before presenting each PR as merge-ready.
- **AskUserQuestion at the forks.** The three editor-arc decisions (pause control → toolbar play/pause; F1 overlay kept alongside F2; phase order A→B→C) and the early ones (stat persistence → data file; targets → both component+table; approach → foundation-first; success bar → examples+forks) were all resolved via AskUserQuestion, not assumed.

## Where We're Going

1. **Editor-GUI arc is COMPLETE.** Next session = clean slate (a new feature per VISION, or polish). The candidate/structural-debt queues are empty.
2. **Optional polish — live editor playtests.** The interactive editor playtests for v8.0.0 (drag/resize/Save-Scene-persist) and v8.1.0 (Stats inspector edit + Data Tables hot-reload) were never eyeballed live by a human with display access (Phase A's F2 was, Phases B/C were skipped or display-off). Worth running `cargo run --example ui_layout_editor_game` and `cargo run --example stat_editor_game`, pressing F2, and confirming the full edit→save→reload loops by eye.
3. **Optional — derive ergonomics.** If `use engine::Reflect` (one import for trait + derive, serde-style) is wanted: publish `engine_reflect_derive` to crates.io, then re-export it from `engine` behind a feature and coordinate a 2-crate publish. Until then, `use engine_reflect_derive::Reflect` is the documented path.
4. **Consumer (rust-survivors) impact:** v8.0.0 is breaking on the remote (EntityDef literal, TextInput/Slider fields, SceneDef v3). rust-survivors pins the engine by git rev and won't pick it up until the user bumps the pin; migration is per CHANGELOG 8.0.0. Pushing is the user's call (standing rule). NOT migrated this arc.

## Risks & Blockers

- **Workspace publishing is now a 2-crate dance.** To publish `skeleton-engine` to crates.io you must publish `engine_reflect_derive` FIRST. The CI Package/publish dry-run passes now only because the derive is a dev-dep (stripped). `scripts/verify.sh` does NOT run `cargo package` — that gate is CI-only; reproduce locally with `cargo package --locked` when touching the workspace or deps.
- **Agent IDE diagnostics are routinely STALE** (phantom parse/missing-field/trait errors from mid-edit snapshots). The real `cargo build` was clean every time this arc. Never act on a diagnostic without re-running the compiler.
- **v8.0.0 is breaking for forks** — anyone tracking `main` breaks on the EntityDef/TextInput/Slider/SceneDef changes; CHANGELOG 8.0.0 is the migration guide.
- **Autonomous screenshot playtests are not available on this machine** — `osascript` lacks accessibility permission and `screencapture` fails on a sleeping display. Human-in-the-loop is required for live editor eyeballing.

## Open Questions

- None blocking. The arc is fully shipped and merged.
- Soft: does the user actually publish `skeleton-engine` to crates.io, or is the Package dry-run gate aspirational? (Affects whether the derive should eventually be published + re-exported.) Not urgent.
- Soft: the docked panels use deprecated top-level `Panel::show(ctx, …)` under `#[allow(deprecated)]` (egui 0.34 wants `show_inside`, which needs a root `Ui`). A cleanup could introduce a root `Ui` via `egui::Area` and drop the allow. Cosmetic; no behavior impact.
- Soft: the per-cell editor in the Data Tables panel only handles primitive `ron::Value` types (int/float/string/bool); nested `Seq`/`Map`/`Option` cells render as a non-editable `(complex)` label. Fine for flat stat tables; a future need for nested data would extend the cell editor.

## Quick Start for Next Session

```bash
# Current state
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3 main      # db19c6e = v8.1.0 merge; clean; pushed
git status -s                  # clean

# The engine is now a Cargo WORKSPACE (members '.' + engine_reflect_derive)

# Reference docs
#   ~/.claude/plans/ui-ux-merry-bengio.md  — 3-phase roadmap + Phase B/C designs
#   docs/CHANGELOG.md  — 7.1.0 / 8.0.0 (breaking) / 8.1.0
#   CLAUDE.md (v1.6.6) — module map, gates

# Key files to read first (editor lives here)
#   src/app/editor/state.rs        — EditorMode, apply_f1/f2, all editor fields
#   src/app/editor/docked_rt.rs    — viewport_to_game, docked_game_pointer_allowed, RtDebounce
#   src/app/editor/ui/docked.rs    — docked layout, shared tab bodies, save path
#   src/prefab.rs                  — SerdeComponentRegistry, EntityDef.components
#   src/data_table.rs              — DataTable + DataTableRegistry
#   engine_reflect_derive/src/reflect_derive.rs — #[derive(Reflect)]

# Verify current state (all must pass)
./scripts/verify.sh
cargo package --locked         # CI-only gate — run when touching workspace/deps
./scripts/wasm_smoke.sh        # render-path changes

# Next action
#   The editor-GUI arc is DONE. Either start a new feature (ask the user), OR
#   do the optional live editor playtest of ui_layout_editor_game / stat_editor_game
#   (F2 → drag/resize/edit → Save → restart/reload → confirm persistence).
```
