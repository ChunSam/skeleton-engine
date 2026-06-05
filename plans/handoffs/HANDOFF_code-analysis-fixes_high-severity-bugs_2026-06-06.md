# Full-codebase analysis + HIGH-severity bug fix sweep (skeleton-engine v2.0.0)

**Date:** 2026-06-06
**Status:** IN PROGRESS — all HIGH fixes done & verified; uncommitted on `main`; next = commit (A) then start MEDIUM items (B)
**Bead(s):** none (bd unavailable)
**Epic:** none
**Chain:** `code-analysis-fixes` seq `1`
**Parent:** `none — first in chain`
**Prior chain:** none — first in chain

> This is a NEW work stream, not a continuation of the example-specific handoffs in
> `plans/handoffs/` (timeline-cutscene, lit-dungeon, etc.). Those are unrelated.

---

## Reference Documents

- `docs/CODE_ANALYSIS.md` — **the 30-issue analysis report this session produced** (severity-ranked; primary backlog source).
- `CLAUDE.md` — agent reference: module map, the 5-command verification gate, task recipes.
- `docs/VISION.md` — the "skeleton/forkable" thesis + "feature is not done until an example exercises it" loop; drives fix priority (priority 1 = forkable skeleton).
- `docs/PATTERNS.md` — core architecture patterns (ECS query collect-then-`get_mut`, render-layer separation, system ordering).
- `docs/HANDOFF.md` — per-phase dev history (separate from these `plans/handoffs/` files).
- `REFERENCE.html` / `ARCHITECTURE.html` — full public API + architecture (generated).
- Memory: `ci-toolchain-pin` (verify with `+1.88.0`), `rust-survivors-engine-pin` (downstream test via `--config` path patch), `subagent-usage-preference`, `doc-language-rule`.

## The Goal

The user asked for a full code review of the entire `skeleton-engine` engine, then to
(1) save the analysis report and (2) fix the HIGH-severity bugs it surfaced, summarizing
in Korean when done. Ultracode (xhigh + dynamic workflow orchestration) was ON for the
session, so the analysis used a 22-agent Workflow. The engine is a wgpu-based, MIT,
genre-agnostic 2D Rust game engine (~25K LOC engine + ~9K LOC examples, ~120 files). The
end state: a tracked analysis doc + a green-CI fix set covering every HIGH bug, so the
"skeleton" stays trustworthy for forkers (VISION priority 1).

## Where We Are

- **Analysis complete:** 22-agent Workflow (15 subsystem readers on Sonnet → 6 cross-cutting lenses on Opus → 1 synthesizer). 1.57M subagent tokens, 783 tool uses, ~17 min. Output saved to `docs/CODE_ANALYSIS.md` (untracked, `??`).
- **30 issues found:** 6 HIGH (#1–6), 16 MEDIUM (#7–22), 8 LOW (#23–30). The synthesizer adversarially **downgraded 4 overstated agent claims** (scripting `get_mut().unwrap()` is guarded by an earlier `get`; the audio fade "dead branch" is reachable; wasm save/load returns `Err` not panic; `last_dt` is set before `update`).
- **9 issues FIXED:** all 6 HIGH (#1–6) + 3 float/hang guards (#24 camera zoom=0, #25 timeline NaN, #26 Timer::repeating(0.0)).
- **5 regression tests ADDED**, all pass. Lib test count: **287 passed, 0 failed** (was ~282).
- **12 files modified + 1 new** (`docs/CODE_ANALYSIS.md`). Uncommitted on `main`.
- **Verification: ALL 5 CI checks pass under the EXACT CI toolchain (Rust 1.88.0 / rustfmt 1.8.0).** Local default stable is 1.9.0 and MUST NOT be used for fmt (see "What We Tried").
- **Memory written:** `~/.claude/.../memory/ci-toolchain-pin.md` + MEMORY.md pointer — records the 1.88.0 verification requirement.
- **NOT committed** (user hadn't asked at fix time). On `main` directly — must branch before committing.
- Specific fixes by file:
  - `src/app/scenes.rs:11-44` `reload_scene()` now `self.panicked_systems.clear()`; `apply_scene_cmd` `Pop` arm prunes `panicked_systems.retain(|&i| i < new_len)` + rebuilds the `PanickedSystems` resource display.
  - `src/app.rs` — 2 new native-only fields (`gizmo_drag_start_positions: Vec<(Entity, glam::Vec2)>`, `component_removers: HashMap<String, ComponentFactory>`) + init + the scene-transition test.
  - `src/app/editor.rs` — `EditorCmd::DeleteEntity` gained `entity: Option<Entity>`; `undo()`/`redo()` rewritten; `register_default_components` registers removers; new public `register_component_remover()`.
  - `src/app/editor/ui/gizmo.rs` — drag-start snapshots all selected entities' positions; release pushes a `MoveEntity` per moved entity.
  - `src/app/editor/ui/mod.rs` — `DeleteEntity { entity: None, ... }`; removal block dispatches through `component_removers` (hardcoded match deleted). **NOTE: this file has a ~696-line diff that is 95% rustfmt-1.8.0 reformatting, not logic — see Risks.**
  - `src/renderer/sprite.rs:454,~691` — `live_material_entities` set + `params_buffers.retain(...)` at end of `render()`.
  - `src/animation/system.rs:29,61` — `fps>0` guard, else `frame_dur = f32::INFINITY`.
  - `src/camera.rs` — `safe_zoom()` helper used in `screen_to_world`/`visible_rect`/`view_proj`.
  - `src/timeline.rs:112` — `rposition(...)` `.unwrap()` → `let Some(idx) = ... else { return Some(first) }`.
  - `src/timer.rs:tick` — repeating wrap uses `%=` (guarded for `duration<=0`).
  - `examples/loading_bar.rs`, `examples/minimap.rs` — off-screen HUDs repositioned via `ViewportSize` / `Camera::visible_rect`; minimap gained `MinimapTag` + `MinimapHudSystem`.

## What We Tried (Chronological)

1. **Inline scouting first** — `ls`/`wc`/`cat Cargo.toml`/`find src` to build the work-list (15 subsystem clusters + examples) and read `src/lib.rs` + `docs/VISION.md` before fanning out. Result: precise file lists for each agent. This "scout then orchestrate" hybrid is the right pattern.
2. **22-agent Workflow** — `analyze-skeleton-engine` script (persisted at `.../workflows/scripts/analyze-skeleton-engine-wf_b590b505-d99.js`). pipeline: 15 readers (sonnet, `SUBSYSTEM_SCHEMA`) → barrier → 6 lenses (opus, `LENS_SCHEMA`, seeded with compact phase-1 JSON) → synthesizer (opus, free-text MD). Returned a single report. Worked first try.
3. **Saved report** to `docs/CODE_ANALYSIS.md`, then re-confirmed each bug site by READING the actual code before editing (per the "verify recalled claims" rule — and the synthesizer itself had downgraded 4 false claims). Every fix below was confirmed against real code first.
4. **Applied 9 fixes + 5 tests** across 12 files. Editor fixes (#2/#3) needed new `App` fields + a struct-variant field (`DeleteEntity.entity`) rippling to 3 sites; verified brace balance and disjoint-field borrows compile.
5. **First `cargo fmt` (local stable) → ALARM:** `src/app/editor/ui/mod.rs` showed a **698-line diff** but my edit was ~15 lines. `git diff -w` showed ~148 real content lines (rustfmt collapsing multi-line method chains) + ~550 whitespace re-indent. Smelled like a rustfmt version mismatch creating CI-risky churn.
6. **Toolchain investigation (the expensive part — DO NOT re-derive):**
   - No `rust-toolchain.toml`. Local `rustfmt --version` = **1.9.0-stable (2026-04-14)**.
   - Read `.github/workflows/ci.yml`: every job uses `dtolnay/rust-toolchain@1.88.0`. CI's fmt/clippy = **Rust 1.88.0**, whose `rustfmt --version` = **1.8.0-stable (2025-06-23)**. 1.88.0 IS installed locally (`rustup toolchain list`).
   - Decisive tests: HEAD `mod.rs` is **clean under rustfmt 1.8.0** (rustfmt leaves a latently-misformatted block untouched — `else` body under-indented at line ~323, chains left multi-line). **But editing the file makes rustfmt reprocess the whole file → the 698-line canonical reformat.** The minimal-diff edited version **FAILS** `cargo +1.88.0 fmt --check`.
   - The other 11 files: diffs are proportional to my edits (no mass reformat) — local 1.9.0 and CI 1.8.0 agree on them; only `mod.rs` has the divergent chains.
7. **Decision: accept rustfmt-1.8.0 canonical reformat of `mod.rs`** (the ONLY CI-passing option) via `cargo +1.88.0 fmt`. Re-verified ALL checks under +1.88.0.
8. **Stash-based "pristine HEAD" tests were unreliable** mid-investigation (gave contradictory "clean" results); the reliable signal was running `cargo +1.88.0 fmt --check` with the actual file on disk. Don't trust `git stash` round-trips for fmt isolation.

## Key Decisions

- **Verify every agent finding against real code before fixing.** Rationale: the synthesizer already caught 4 false-positive agent claims; recalled/agent claims can be wrong. Every fix here was confirmed at its `file:line`.
- **Scope = 6 HIGH + 3 cheap float/hang guards (#24/#25/#26).** Rationale: user said "fix the HIGH bugs"; I had offered #1/#5/#24/#25/#26 as the "cleanest first batch", so I folded the 3 guards in. MEDIUM/other LOW deferred. Rejected: doing all 30 (too broad for one pass; MEDIUM #11 Color type / #13 PhysicsWorld-as-resource are API changes needing design).
- **#3 fix = remover-registry + non-breaking new method**, NOT changing `register_component`'s signature. Rationale: `register_component(name, factory)` is public and consumed by `rust-survivors`; changing it to generic `<T>` would break forks. Added `register_component_remover(name, remover)` instead and hid the "✕" button when no remover is registered. Rejected: generic `register_component<T>` (breaking).
- **#2c group-move = one `MoveEntity` per moved entity** (non-atomic undo), NOT a composite command. Rationale: avoids restructuring `EditorCmd` into a composite variant; strictly better than "others can't be undone at all". Trade-off documented: undoing a group move takes N Ctrl+Z.
- **#5 / animation = `frame_dur = f32::INFINITY` sentinel when `fps<=0`** instead of restructuring the loop. Rationale: `timer >= INF` is always false → no frame advance, no infinite `while`. Minimal and clean for both the crossfade and current-clip paths.
- **#26 / Timer = `%=` modulo wrap** (guarded `duration>0`), not constructor clamping. Rationale: `f32::EPSILON` clamp would NOT bound elapsed (tick subtracts once, not in a loop); `%=` bounds it AND fixes dt>duration catch-up. Existing `repeating_wraps` test still passes.
- **Accept the `mod.rs` 696-line reformat.** Rationale: it is rustfmt-1.8.0 canonical and the only way `cargo fmt --check` (CI) passes once the file is edited. Rejected: keeping a minimal `mod.rs` diff (fails CI fmt); keeping HEAD's skipped state (impossible once edited).
- **Always verify this repo with `cargo +1.88.0 ...`**, never default stable. Saved as memory.

## Evidence & Data

### The 30 issues (severity → status)

| Sev | Count | Fixed this session | Deferred |
|-----|------:|--------------------|----------|
| HIGH | 6 | #1, #2, #3, #4, #5, #6 (all) | — |
| MEDIUM | 16 | — | #7–#22 |
| LOW | 8 | #24, #25, #26 | #23, #27, #28, #29, #30 |

### HIGH bugs fixed (with confirmed locations)

| # | Bug | Fix location |
|---|-----|--------------|
| #1 | `panicked_systems` (`HashSet<usize>`, `app.rs:93`) never cleared across scene transitions → stale indices disable new scene's systems | `src/app/scenes.rs` reload clear + Pop prune |
| #2 | Editor undo/redo: CreateEntity redo `drop(cmd);return` (chain break), DeleteEntity redo despawns `*selected` not recorded target, group-move records only primary | `src/app/editor.rs`, `editor/ui/gizmo.rs` |
| #3 | Component removal hardcoded 4-type `match`, `_ => {}` → custom components can't be removed | `src/app/editor/ui/mod.rs` + `register_component_remover` |
| #4 | `params_buffers: HashMap<Entity,...>` (`sprite.rs:72`) insert-only → GPU leak | `src/renderer/sprite.rs` retain |
| #5 | `1.0/clip.fps` (`animation/system.rs:29,61`); `fps<0` → negative `frame_dur` → infinite `while` | `fps>0` guard |
| #6 | `loading_bar` bar at `(-200,-18)`, text `(-70,-60)`; `minimap` HUD world-fixed `(272,-172)` → off-screen | both examples |

### Verification (run UNDER Rust 1.88.0 — CI's pinned toolchain)

```
cargo +1.88.0 fmt --check                              → CLEAN ✓
cargo +1.88.0 clippy --all-targets -- -D warnings      → 0 warnings ✓ (16.8s)
cargo +1.88.0 test --all-targets                       → 287 passed / 0 failed ✓
cargo +1.88.0 build --target wasm32-unknown-unknown    → Finished ✓ (43s, target added)
RUSTDOCFLAGS="-D warnings" cargo +1.88.0 doc --no-deps  → Generated ✓
```

### New tests (5, all pass under 287-test lib suite)

| Test | File | Guards |
|------|------|--------|
| `scene_replace_clears_panicked_systems` | `src/app.rs` (tests) | #1 flagship |
| `zoom_zero_does_not_produce_nan` | `src/camera.rs` (tests) | #24 |
| `all_nan_keyframe_times_do_not_panic` | `src/timeline.rs` (tests) | #25 |
| `repeating_zero_duration_stays_bounded` | `src/timer.rs` (tests) | #26 |
| `repeating_catches_up_when_dt_exceeds_duration` | `src/timer.rs` (tests) | #26 |

### `git diff --stat` (12 files, +679 / −391; mod.rs dominated by fmt)

```
examples/loading_bar.rs    |  22 +-
examples/minimap.rs        |  49 +++-
src/animation/system.rs    |  17 +-
src/app.rs                 |  51 ++++
src/app/editor.rs          |  58 +++-
src/app/editor/ui/gizmo.rs |  49 +++-
src/app/editor/ui/mod.rs   | 696 ++++  (≈148 real via `git diff -w`, rest = rustfmt)
src/app/scenes.rs          |  21 ++
src/camera.rs              |  46 ++-
src/renderer/sprite.rs     |  12 +
src/timeline.rs            |  19 +-
src/timer.rs               |  30 +-
?? docs/CODE_ANALYSIS.md (new)
```

### Workflow run metrics (analysis phase)

| Metric | Value |
|--------|------:|
| Agents | 22 (15 subsystem readers + 6 lenses + 1 synth) |
| Subagent tokens | ~1,572,607 |
| Tool uses | 783 |
| Duration | ~1,001s (~17 min) |
| Reader model | Sonnet (per `subagent-usage-preference`) |
| Lens + synth model | Opus (reasoning-critical) |
| Run ID | `wf_b590b505-d99` |

### Re-running the analysis (if needed)

The 22-agent workflow script is persisted and re-runnable without re-sending it:
```
Workflow({scriptPath: ".../workflows/scripts/analyze-skeleton-engine-wf_b590b505-d99.js"})
```
Structure: `SUBSYSTEMS` array (15 clusters w/ file lists + focus) → `parallel()` readers (sonnet, `SUBSYSTEM_SCHEMA`) → barrier → `LENSES` array (6: architecture, api-ergonomics, correctness, performance, testing, security) seeded with compact phase-1 JSON (opus, `LENS_SCHEMA`) → synthesizer (opus). To re-scope, edit `SUBSYSTEMS`/`LENSES` and re-invoke with the same `scriptPath`.

### Toolchain facts (CI parity)

| Tool | Local default | CI (pinned) |
|------|---------------|-------------|
| rustc/cargo | stable | **1.88.0** (`dtolnay/rust-toolchain@1.88.0`) |
| rustfmt | 1.9.0-stable (2026-04-14) | **1.8.0-stable (2025-06-23)** |
| Effect | collapses multi-line chains in `mod.rs` | leaves HEAD `mod.rs` untouched (skip) |

## Code Analysis

- `App.panicked_systems: HashSet<usize>` (`app.rs:93`) is the skip set checked at `app/schedule.rs:186` and inserted at `:214`. `PanickedSystems` (display, `Vec<String>`) is a core resource (`core_resources.rs:34`), reset on `World::new()` in `reload_scene`.
- `App::apply_scene_cmd` (`scenes.rs:61`): `Replace` calls `systems.clear()` → `reload_scene()` (world reset, preserves `register_persistent` resources) → `on_enter`. `Push` appends (no clear needed). `Pop` truncates by owned-count.
- `SpriteRenderer::render(world, ...)` collects `mat_ids` from `world.query::<ShaderMaterial>()` BEFORE culling (`sprite.rs:454`) — the full live set, ideal for liveness pruning regardless of which offscreen target is rendering.
- `EditorHistory::undo/redo(&mut self, world, selected)` (`editor.rs`): commands are popped, matched, pushed to the opposite stack. The `DeleteEntity.entity` field is filled during `undo` (after the immutable match borrow ends, via `(respawned, &mut cmd)`).
- `Camera` is top-left anchored; `view_proj`/`screen_to_world`/`visible_rect` all divide by `zoom`. `safe_zoom()` = `if zoom.abs() < EPSILON { EPSILON } else { zoom }`.
- `Track::sample` already guarded NaN sample-time `t`, but not NaN keyframe *times*; `total_cmp` sorts NaN last, all-NaN makes every `kf.time <= t` false → `rposition` None.
- `register_component(name, factory)` / new `register_component_remover(name, remover)` both take `impl Fn(&mut World, Entity) + Send + Sync + 'static` (`ComponentFactory` alias). The remover map gates the editor "✕" button.

## Files Changed

### Source code (engine)
- `src/app/scenes.rs` — #1: clear/prune `panicked_systems` on scene transitions.
- `src/app.rs` — #2/#3: 2 native fields + inits; + scene-transition test.
- `src/app/editor.rs` — #2/#3: `DeleteEntity.entity`, undo/redo rewrite, removers, `register_component_remover`.
- `src/app/editor/ui/gizmo.rs` — #2c: per-entity drag-start snapshot + per-entity move recording.
- `src/app/editor/ui/mod.rs` — #2/#3 sites; **+ unavoidable rustfmt-1.8.0 reformat (≈550 ws + 148 chain lines)**.
- `src/renderer/sprite.rs` — #4: `params_buffers` liveness retain.
- `src/animation/system.rs` — #5: `fps>0` guard (both crossfade + current-clip paths).
- `src/camera.rs` — #24: `safe_zoom()` + test.
- `src/timeline.rs` — #25: `rposition` fallback + test.
- `src/timer.rs` — #26: `%=` wrap + 2 tests.

### Examples
- `examples/loading_bar.rs` — #6: center bar/text via `ViewportSize`.
- `examples/minimap.rs` — #6: `MinimapTag` + `MinimapHudSystem` pins HUD to camera's `visible_rect` top-right.

### Docs / data
- `docs/CODE_ANALYSIS.md` (NEW) — the full 30-issue analysis report (English).
- `~/.claude/projects/.../memory/ci-toolchain-pin.md` (NEW) + MEMORY.md pointer.

### Workflow artifact
- `.../workflows/scripts/analyze-skeleton-engine-wf_b590b505-d99.js` — the 22-agent analysis script (re-runnable via `Workflow({scriptPath})`).

## User Feedback & Preferences

- **"두가지 다 진행하고 완료되면 정리해서 알려줘. 정리는 한국어로 해줘"** — do BOTH (save report + fix HIGH bugs); final summary in **Korean** (artifacts stay English).
- **"코드 전체 살펴보고 분석해줘"** — wanted a full-codebase analysis (drove the 22-agent workflow under ultracode).
- Earlier this session the user fixed a Claude subscription/login issue (`/login`), turned **workflows ON** and **effort = ultracode** — signals appetite for thorough, multi-agent work and not minimizing token cost.
- Follow-up command: **`/handoff 하고 a 진행 다음 b 진행`** — create handoff, then proceed with A then B. Interpreted as **A = commit the current work, B = begin MEDIUM-severity items** (from the 3 options I offered: 1 commit / 2 MEDIUM / 3 playtest). CONFIRM scope before the consequential commit.
- Standing prefs (memory): subagents aggressively for parallel work (Sonnet); docs in English to cut tokens; the user will run windowed-example playtests themselves on macOS when delegated.

## Where We're Going

- **A — Commit (next, requested):** on `main` now → **branch first** (e.g. `fix/high-severity-bugs`). Recommend splitting to keep review clean: (1) `fix:` commit for the 11 logic-bearing files + tests + `docs/CODE_ANALYSIS.md`, (2) optionally isolate the `mod.rs` rustfmt churn or just include it with a clear body note. Commit message must end with the Co-Authored-By trailer. CONFIRM with user: branch name + whether to split.
- **B — Begin MEDIUM items (requested):** highest-leverage first. Suggested order tied to VISION priority 1 (forkability): **#9** wasm-gate `save`/`load`/`load_script` (typed error, no green-CI runtime failure) → **#10** publish `SystemLabel` consts for ordering-sensitive built-ins → **#13** mirror `PhysicsWorld` to a World resource → **#11** `Color` newtype (API change — design first) → **#14** `query_mut`. CONFIRM which subset; #11/#13/#14 are API-surface changes that may affect `rust-survivors`.
- After each MEDIUM batch: re-run the 5-check verification UNDER `cargo +1.88.0`.

## Deferred Backlog (MEDIUM + remaining LOW) — for Phase B

Full detail in `docs/CODE_ANALYSIS.md`. Inlined here so B is actionable without re-opening it.
Severity per the analysis; "API" = changes public surface (check `rust-survivors` impact).

### MEDIUM (#7–#22)

| # | Issue | Location | Fix sketch | API? |
|---|-------|----------|------------|:---:|
| #7 | Per-frame Rhai AST deep-clone + 4×`Arc<Mutex>` per scripted entity | `src/scripting/execution.rs:46,53-58` | `Arc<rhai::AST>` + per-runner reusable buffers (drop the `Mutex`) | no |
| #8 | New render pass opened per texture-run and per material | `src/renderer/sprite.rs:626-647,665-688` | one pass for the whole pre-sorted stream | no |
| #9 | `save`/`load`/`delete`/`load_script` call `std::fs` unconditionally → wasm runtime failure w/ green CI | `src/save.rs:77-130`, `src/asset/script_loading.rs:12-43` | `#[cfg(not(wasm32))]` gate or wasm stub returning typed `SaveError` | no |
| #10 | Built-in systems publish no `SystemLabel` — scheduler not dogfooded | `src/ecs/schedule.rs` | `pub const` labels for ordering-sensitive built-ins (StateMachine-after-Animation, Layout-before-Ui) | no (additive) |
| #11 | Color in 3 incompatible forms (`[f32;4]`/`[u8;4]`/`[f32;3]`) | Sprite/DrawText/PointLight/Button | `Color` newtype + `impl Into<Color>` everywhere | **yes** |
| #12 | Screen-vs-world coord convention undocumented; flagship examples get it wrong | `renderer/text.rs`, `sprite/ui_primitives.rs:91`, `camera.rs` | anchor enum / `DrawText::centered` + docs | partial |
| #13 | `PhysicsWorld` not mirrored to a World resource (asymmetric w/ SpatialGrid) | `src/physics/system.rs:24-33` | insert PhysicsWorld as a resource like SpatialGrid | **yes** |
| #14 | No mutable-iteration query — forces collect-then-`get_mut` everywhere | `src/ecs/world.rs:306-338` | add `query_mut`/`query2_mut` | **yes** (additive) |
| #15 | Point-light radius unit mismatch → lights ~½ size | `src/renderer/lighting.rs:97,148` | unify CPU/shader space; add visual-size test | no |
| #16 | One-way collider handle not dropped on `remove_body` → stale reuse | `src/physics/world/body_factory.rs:239` | remove handle(s) from `one_way_colliders` | no |
| #17 | Continuous emitter under-emits on slow frames (1 interval/frame) | `src/particle.rs:204-213` | `while timer >= interval` | no |
| #18 | A* has no closed-set; re-expands, re-allocs per call | `src/pathfinding.rs:173-201` | visited set + reusable scratch | no |
| #19 | `CollisionGridSystem` deep-clones 2 HashMaps every frame | `src/collision/grid.rs:197` | `Arc<SpatialGrid>`; debug system reads mirror not rebuild | no |
| #20 | Audio `update()` not driven by any built-in system; no SFX cache | `src/audio/playback.rs:122,188` | register `AudioSystem`; cache decoded bytes | no (additive) |
| #21 | CheckBox toggles on press while Button fires on release | `src/ui/system/checkbox_pass.rs:32` | toggle on release w/ press-inside+release-inside | no |
| #22 | `RenderLayer(i32)` vs `layer_mask(u32)` clamp folds negatives to bit 0 | `src/renderer/sprite/sort.rs:88-94`, `components.rs:298` | restrict mask to non-negative range or bias onto bits | partial |

### Remaining LOW (#23, #27–#30; #24/#25/#26 already fixed)

| # | Issue | Location | Fix sketch |
|---|-------|----------|------------|
| #23 | `spawn_scene_def` silently overwrites duplicate tags | `src/prefab.rs:301-303` | `log::warn!` / validate in `SceneDef::load` |
| #27 | `MouseButton` not re-exported; examples use internal paths + Korean docs; `gpu_particles.rs:23` `.unwrap()` | `src/lib.rs:48` | `pub use winit::event::MouseButton`; fix examples |
| #28 | rapier `ImpulseJointHandle` leaks through public API | `src/physics/mod.rs:10` | wrap in engine `JointHandle` newtype |
| #29 | `ReflectValue` closed enum (no `#[non_exhaustive]`, no `I32`) | `src/reflect.rs:8` | `#[non_exhaustive]` + add `I32` |
| #30 | Incomplete Rhai limits (only `max_operations`) | `src/scripting/api.rs:9-12` | string/array/map/recursion bounds via `ScriptingLimits` |

### 4 agent claims the synthesizer DOWNGRADED (do NOT re-chase as bugs)

- Scripting `get_mut().unwrap()` is guarded by an earlier `get` — not a panic.
- The audio fade "dead branch" is actually reachable and correct.
- wasm `save`/`load` returns `Err` (not a panic) — still worth gating (#9) for a typed signal, but not a crash.
- GPU particles: `last_dt` IS set before `update`, so they use the current frame's dt — correct.

## Risks & Blockers

- **rustfmt version mismatch (live trap):** local default = 1.9.0, CI = 1.8.0. NEVER run plain `cargo fmt` for final verification — use `cargo +1.88.0 fmt`. `scripts/verify.sh` and CLAUDE.md's verify block use the default toolchain (the trap). Memory `ci-toolchain-pin` records this.
- **`mod.rs` reformat churn:** any future edit to `src/app/editor/ui/mod.rs` triggers the same ~700-line rustfmt reprocessing. Isolate real changes with `git diff -w`.
- **API-change MEDIUM items** (#11 Color, #13 PhysicsWorld resource, #14 query_mut) can break `rust-survivors` (pins engine by git rev; test via `--config` path patch — memory `rust-survivors-engine-pin`). Check downstream impact before landing.
- Examples only compile in CI, never run — the off-screen fixes (#6) are visually unverified; a windowed playtest would confirm.

## Open Questions

- A/B interpretation: is **A = commit, B = MEDIUM** correct? Confirm.
- Commit shape: single `fix:` commit or split logic vs `mod.rs` fmt churn? Branch name?
- Which MEDIUM items for B, and how far (the API-changing ones need a design pass + downstream check)?
- Should `docs/CODE_ANALYSIS.md` get a HANDOFF/CHANGELOG cross-reference, and should the analysis doc mark the 9 fixed items as resolved?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine

# Restore context
cat plans/handoffs/HANDOFF_code-analysis-fixes_high-severity-bugs_2026-06-06.md
cat docs/CODE_ANALYSIS.md            # the 30-issue report (severity-ranked)

# Current state: 12 files modified + docs/CODE_ANALYSIS.md, uncommitted on `main`
git status -s
git diff -w --stat                   # real changes (mod.rs noise filtered)

# Key files to read first
#   src/app/scenes.rs, src/app/editor.rs, src/app/editor/ui/{gizmo,mod}.rs (HIGH fixes)
#   src/renderer/sprite.rs (#4), src/animation/system.rs (#5)

# Verify current state — MUST use the 1.88.0 toolchain (CI parity), NOT default stable
cargo +1.88.0 fmt --check
cargo +1.88.0 clippy --all-targets -- -D warnings
cargo +1.88.0 test --all-targets

# Next action (A): branch off main, then commit the fix set
git checkout -b fix/high-severity-bugs
# then commit (confirm split + message with user); end message with Co-Authored-By trailer

# Then (B): start MEDIUM items — recommend #9 (wasm-gate save/load) first
```
