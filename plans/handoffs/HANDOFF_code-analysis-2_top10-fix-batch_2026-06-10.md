# 2nd full-codebase analysis (multi-agent) + v4.6.0 non-breaking Top-10 fix batch

**Date:** 2026-06-10
**Status:** COMPLETED — 2nd full code analysis (architecture + quality focus, 24 agents, ~2.03M subagent tokens) produced `docs/CODE_ANALYSIS_2026-06-10.md` (70 verified findings); then a **v4.6.0 non-breaking fix batch** closed Top-10 items #1/#3/#4/#5/#6/#7/#9(+P3 follow-up)/#10 on branch `fix/analysis-top10` (10 commits). Full `+1.88.0` 5-command gate green, lib tests 339→346, all-targets 373/0, `wasm_smoke.sh` PASS. Breaking items #2/#8 deferred to a planned v5.0.0 batch. Branch committed + pushed this session (close-out).
**Bead(s):** none (`bd` empty — tracked with TaskCreate, tasks #1–#9 all completed)
**Epic:** code-analysis remediation, round 2 (`docs/CODE_ANALYSIS_2026-06-10.md`)
**Chain:** `code-analysis-2` seq `1`
**Parent:** none — first in chain
**Prior chain:** none — first in chain

---

## Related Handoffs

- `HANDOFF_code-analysis-fixes_rust-survivors-v4-verify_2026-06-08.md` (chain `code-analysis-fixes` seq 4, **closed**) — the FIRST analysis round (2026-06-05, bug-focused, 30/30 resolved → v4.0.0). Separate work stream; this session is a brand-new analysis with different focus (architecture/quality, explicitly NOT bugs). Its report `docs/CODE_ANALYSIS.md` must stay intact (see Key Decisions — it was accidentally overwritten this session and restored).
- `HANDOFF_networking-dogfood_salvage-run-aoi_2026-06-10.md` (chain `networking-dogfood` seq 9) — state of `main` (`8dd042c`/`370b0bb`) this branch forked from; v4.5.0, salvage_run example.

## Reference Documents

- `docs/CODE_ANALYSIS_2026-06-10.md` — THIS round's analysis report (committed, in Korean, with a resolution-status header listing what v4.6.0 fixed and what moved to v5). Canonical for finding details + file:line evidence.
- `docs/CODE_ANALYSIS.md` — the 2026-06-05 bug-focused report (all 30 items resolved; historical snapshot — do not edit).
- `docs/CHANGELOG.md` — new `## 4.6.0` entry documents the whole batch (Added/Changed/Deprecated).
- `CLAUDE.md` — bumped to doc v1.5.0 / engine v4.6.0; module-map rows updated for `renderer/uv.rs`, `tween::Lerp`, `save::write_ron/read_ron`, `DebugDraw::rect_filled_z`.
- Memory files: `new-model-subagent-incompat` (NEW this session), `engine-current-state` (updated twice), `ci-toolchain-pin`, `subagent-usage-preference`, `conversation-language-korean`.

## The Goal

User opened with "새로운 모델 나온 기념으로 전체 코드 분석 맡기고 싶어" — celebrate the new model (claude-fable-5) by running a full-codebase analysis. Via AskUserQuestion the scope was fixed to **architecture health + code quality/simplification** (bug-hunting and perf-hotspot options explicitly declined — the 06-05 round already did bugs), executed as a **multi-agent workflow** (user picked the recommended option knowing the token cost). After the report landed, the user said "Top10 수정 계획 세우고 진행 해줘" — plan and execute the Top-10 fixes. The batch was split by semver impact; the user chose **Batch A (non-breaking) only, on a branch with per-item commits**. End state: v4.6.0 ready on `fix/analysis-top10`, with the breaking remainder specified for a future v5.0.0 batch.

## Where We Are

- **Branch `fix/analysis-top10`** forked from `main` @ `370b0bb` (v4.5.0): **10 commits** (`0a30637..7f12374`), tree clean, being pushed at session close.
- **Engine version 4.5.0 → 4.6.0** (`Cargo.toml` + `Cargo.lock`). All changes non-breaking by design (verified: every moved type kept its old path via `pub use` shims; every changed struct had only private/`pub(crate)` fields).
- **Analysis report** `docs/CODE_ANALYSIS_2026-06-10.md` (~35KB, Korean prose / English identifiers) committed with resolution-status header. 70 surviving findings (confirmed+downgraded), 3 refuted, across 10 module groups + 3 cross-cutting lenses.
- **#1 DONE** — `save::write_ron` / `save::read_ron` (plaintext pretty RON, parent-dir creation, wasm → `Unsupported`); `SceneDef::{save,load}` + `Prefab::{save,load}` rerouted to them, signatures unchanged. `read_ron` sniffs the `R2DAEAD01` AEAD magic and falls back to `decrypt_save_bytes` + `SaveKey::DEFAULT` so pre-4.6 encrypted files still load. 6 new tests (round-trip, human-readable assertion, encrypted-back-compat ×2 types).
- **#4 DONE** — `LightingRenderer` caches its bind group (`cached_bind_group: Option<wgpu::BindGroup>` + `cached_scene_view_ptr: usize` pointer-identity check); `run_pass` now `&mut self`; `resize()` clears the cache; `reconfigure()` is `*self = Self::new(..)` so it resets implicitly. I adversarially reviewed the ABA risk myself before accepting (see What We Tried).
- **#9 DONE (partial by design) + P3 follow-up DONE** — sprite renderer no longer clones WGSL `frag_source` per entity per frame. Final state after the P3 fix: collection pass clones **nothing**; the clone decision happens **after layer/cull filters** with a frame-local `seen_new_hashes: HashSet<u64>` — exactly one clone per *new* pipeline hash per render call, carried by the first *surviving* entity. Per-sprite texture-key `String` clones remain (need `Sprite.texture` type change = breaking, deferred to v5).
- **#10 DONE** — native `NetworkClient::is_connected()` (parity with wasm at `network.rs:480`), backed by `Arc<AtomicBool>` (Acquire/Release); the socket thread sets it true at handshake and clears it on **every** exit path (send error, local Close, channel disconnect, remote Close frame, io/protocol error). New honest test `native_client_is_not_connected_before_handshake` (connects to `ws://127.0.0.1:1`).
- **#5 DONE** — 17 editor-only fields extracted from `App` into `EditorState` (`src/app/editor/state.rs`, `pub(super)` re-export). 5 cross-platform fields (`inspector_selected`, `gizmo_dragging`, `gizmo_drag_offset`, `editor_save_status`, `inspector_tab`) + 12 native-only (`selected_entities`, `copy_clipboard`, `editor_save_path`, `editor_load_status`, `cmd_history`, `gizmo_drag_start_pos`, `gizmo_drag_start_positions`, `component_factories`, `component_removers`, `add_component_selected`, `snap_enabled`, `snap_size`). Runtime fields deliberately NOT moved: `lighting_renderer`, `fade_renderer`, both `*_texture_for_lighting`, `gpu_particle_renderer`, `gilrs`.
- **#3 DONE** — `UvRect` + `BlendUv` moved to new `src/renderer/uv.rs` (canonical home); `animation::player` keeps a `pub use` shim; 5 import sites repointed (`atlas.rs`, `tilemap.rs`, `renderer/ui.rs`, `renderer/sprite.rs`, `components.rs`); root re-exports moved from the `animation` group to the `renderer` group in `lib.rs` (paths unchanged for consumers). `BlendWeight` stays in animation (it IS an animation concept).
- **#7 DONE** — `Lerp` trait + 4 impls (`f32`, `glam::Vec2`, `[f32;4]`, `Color`) moved `timeline.rs` → `tween.rs`; shim left in timeline; `network.rs` imports from tween; root re-export moved to the tween group.
- **#6 DONE (redesigned)** — the report's recommendation was wrong in one place (see What We Tried): `DebugRect` is **filled + z-ordered**, `DebugShape::Rect` is an **outline**. Fix: `DebugDraw` gained `rect_filled(min,max,color)` / `rect_filled_z(min,max,color,z)` backed by a new `pub(crate) FilledRect` vec field (field addition is non-breaking because all `DebugDraw` fields are `pub(crate)`); `DebugDrawQueue`/`DebugRect` are `#[deprecated(since="4.6.0")]` but still registered + drained (render step 2.5, `#[allow(deprecated)]`) until v5. Migrated producers: `CollisionDebugSystem`, editor gizmo selection highlight, `sokoban` example RenderSystem.
- **Verification (all green, on the CI pin)**: `cargo +1.88.0 fmt --check` / `clippy --all-targets -- -D warnings` / `build --target wasm32-unknown-unknown` / `test --all-targets` / `RUSTDOCFLAGS="-D warnings" doc --no-deps`. Lib tests **339 → 346** (+6 save/prefab, +1 network); `--all-targets` total **373 passed / 0 failed**.
- **`./scripts/wasm_smoke.sh` PASS** — coin_race built to wasm, served, rendered headless (DPR=2): server logged client connection, screenshot 41,977 bytes (≥15,000 threshold), and I eyeballed `/tmp/wasm_smoke.png` — HUD text, player square, coins, footer all correct. This empirically clears the sprite-renderer + UvRect-move changes for wasm.
- **Memory updated**: `engine-current-state` now records the branch, the per-item fix list, and the v5 plan; `new-model-subagent-incompat` records the fable5 subagent failure + workaround.
- **NOT done / explicitly deferred to v5.0.0**: #2 rapier handle newtypes, #8 `on_enter` SystemRegistrar, removal of deprecated `DebugDrawQueue`/`DebugRect`, removal of the `pub use` shims, `Sprite.texture` → `Arc<str>`/interning.
- **Branch not merged to main**; rust-survivors pin (v4.4.0 by rev) is unaffected until deliberately bumped.

## What We Tried (Chronological)

1. **Workflow run 1 — total instant failure (fable5 harness incompat).** Designed a 3-phase workflow (10 module-group analyzers + 3 cross-cutting lenses → per-group adversarial verifier → Korean synthesis report). Launched with default model inheritance. **All 14 agents died in ~1.7s with `API Error: 400 "thinking.type.disabled" is not supported for this model`** — the subagent harness disables thinking; claude-fable-5 only accepts adaptive/enabled thinking, so the API rejects before the model runs. 0 tokens wasted. Lesson memorialized in `new-model-subagent-incompat`: **always pass explicit `model:` on `agent()`/Agent calls while the main session runs fable5**.
2. **Workflow run 2 — model override fix.** Edited the persisted script file (4 `Edit` calls) adding `model: 'sonnet'` to all analyze/verify/cross agents and `model: 'opus'` to synthesis, relaunched via `scriptPath`. All 23 fan-out agents completed (1,474,252 tokens, 1,123 tool uses, ~10.7 min)…
3. **…then the script threw at synthesis: `undefined is not an object (evaluating 'args.reportPath')`.** My error: relaunching with `scriptPath` does NOT re-send `args` — I had passed `{date, reportPath}` only on run 1. Recovery: `Workflow({scriptPath, resumeFromRunId: "wf_b70f5a06-9b9", args: {…}})` — the journal cache restored all 23 completed agents **for free** (same prompt+opts → cache hit), only the Opus synthesis ran live (556,045 tokens). **Lesson: `args` must be passed on EVERY Workflow invocation, including resumes.**
4. **Synthesis agent path deviation — overwrote a tracked file.** The Opus synthesizer wrote to `docs/CODE_ANALYSIS.md` (the 06-05 bug report, tracked, with the 30/30 resolution history) instead of the instructed `CODE_ANALYSIS_2026-06-10.md`. Caught it because `git status` showed ` M` (modified) instead of `??` (untracked). Recovery: `mv docs/CODE_ANALYSIS.md docs/CODE_ANALYSIS_2026-06-10.md && git checkout -- docs/CODE_ANALYSIS.md`. Nothing lost.
5. **Pre-plan scouting (cheap greps before committing to the batch split).** Verified the facts the plan hinged on: all modules are `pub mod` in `lib.rs` (→ moving types breaks deep paths unless shimmed → shims chosen); `Sprite.texture` is `pub Option<String>` (→ #9 must stay internal); `App` editor fields are all private (`grep -c 'pub '` over the struct span = 0 → #5 is non-breaking); no `.ron` files exist on disk in examples/assets (→ #1 has no in-repo migration burden); `DebugRect` is consumed by sokoban as its whole renderer (→ #6 can't just delete).
6. **Wave 1 — 4 parallel Sonnet agents on disjoint file sets** (#1 save/prefab, #4 lighting+render.rs, #9 sprite.rs, #10 network.rs). All four returned green self-verifications. I then ran the combined gate myself (fmt/clippy/lib tests 346) before committing 4 per-item commits. Disjoint-file partitioning meant zero merge conflicts.
7. **A2's pointer-identity cache — adversarial review before accepting.** Pointer-compare caching has an ABA hazard: the `TextureView` lives *inline* in `App`'s `Option<(Texture, TextureView, …)>` field, so replacing the tuple contents keeps the SAME address — a pure pointer check would wrongly say "unchanged". Traced every recreation path in `render.rs:85-208`: (a) dims/format change → the `match` always also hits `lr.resize()` (clears cache) or `lr.reconfigure()` (`*self = Self::new(..)` → cache reset — verified by reading the impl); (b) post-process toggle alternates between two *different* App fields → different addresses → cache rebuilds; (c) `AmbientLight` removed/re-added → whole renderer dropped/rebuilt. No hole. Accepted.
8. **Wave 1.5 — A5 agent (EditorState, app.rs + app/editor/**) in parallel with me doing #3/#7 in main session.** Safe because file sets are disjoint (A5: app/*; me: renderer/animation/timeline/tween/network/lib). During A5's run the IDE streamed transient E0609 "no field" diagnostics from its mid-refactor states — correctly ignored; its final gate (native + wasm check + clippy + 9/9 app tests) was clean.
9. **#3/#7 commit granularity decision.** Both moves touch `lib.rs`; committing them separately would leave an intermediate tree that doesn't compile (lib.rs would re-export `tween::Lerp` before tween defines it). Combined into ONE commit (`dcc54e2`) to keep every commit bisect-buildable.
10. **#6 — found the report wrong, redesigned the fix.** The report (and its verifier) recommended migrating producers to `DebugShape::Rect`. Reading the actual drain code showed `DebugDrawQueue` → **filled** `DrawRect` **with z**, while `DebugShape::Rect` renders an **outline with fixed Z**; sokoban uses the old API as a poor-man's immediate-mode filled-rect renderer (floor/walls/goals/boxes/player at z 0.0–0.4). Migrating per the report = visual regression. Also `DebugShape` is a non-`#[non_exhaustive]` pub enum → adding a variant is formally semver-breaking. Chosen design: new `pub(crate) FilledRect` storage + `rect_filled/_z` methods on `DebugDraw` (struct has only `pub(crate)` fields → field addition non-breaking), old pair deprecated-but-functional.
11. **Final gate + wasm smoke.** `+1.88.0` 5-command gate (per `ci-toolchain-pin`: local stable rustfmt can differ from CI) all green; `wasm_smoke.sh` PASS with screenshot eyeball. Version/docs commit `50a85e6`.
12. **Per-item commits from a multi-agent working tree.** With 4 agents mutating one tree, per-item commits were preserved by **partitioning file sets per agent up front** and staging per set (`git add <agent's files> && git commit`). The one collision risk (A2 needing `src/app/render.rs` while A5 would too) was solved by scheduling: A2 in wave 1 (committed), A5 alone in wave 1.5. Worktree isolation was considered and rejected — merge overhead outweighs it when file sets can be made disjoint by planning.
13. **P3 follow-up (handed in from another agent's review).** Reported flaw in commit `3cddc1e`: collection decided clone-vs-skip from `custom_pipelines.contains_key` alone, so N entities sharing one NEW material hash cloned the WGSL N times on their first frame. The handed-in suggestion (frame-local `HashSet` dedup **at query time**) had its own flaw I caught: the single source-carrying entity could then be dropped by the layer-mask/cull filters, leaving the pipeline uncompiled — *permanently*, if that entity is always culled (e.g. lives on a never-rendered layer while a duplicate is visible). Final fix: collection carries `(Entity, u64, [f32;4])` with **zero clones**; the clone decision moved into the entry-build loop **after all filters**, using `!contains_key && seen_new_hashes.insert(hash)` and an on-demand `world.get::<ShaderMaterial>(entity)`. Compile flow (`!frag_source.is_empty() && !contains_key` pass before draw) unchanged. Verified: fmt OK, clippy `--all-targets` clean, **373/0** all-targets tests. Commit `7f12374`.

## Key Decisions

- **Batch split by semver, batch A only today** (user choice from 3 options): 8 non-breaking items now as v4.6.0; #2 (rapier handle newtypes) + #8 (`on_enter` registrar) + all removals → planned v5.0.0 batch, following the repo's v3-breaking-batch branch+PR precedent.
- **Branch + per-item commits** (user choice): `fix/analysis-top10`, 10 commits, push/PR deferred to session close (this handoff's close-out does commit+push of the branch — user: "하고 커밋 푸쉬").
- **`pub use` shims make type moves non-breaking** — old deep paths (`animation::player::UvRect`, `timeline::Lerp`) still compile; root re-exports unchanged. Shims are explicitly scheduled for removal in v5.
- **Fan-out on Sonnet, synthesis on Opus** — Sonnet per `subagent-usage-preference` memory (cost), Opus for the single high-stakes report writer; forced anyway by the fable5 subagent incompat (see memory `new-model-subagent-incompat`).
- **Encrypted-load fallback instead of a clean format break** (#1): `read_ron` sniffs `R2DAEAD01` and decrypts — keeps any user's pre-4.6 scene/prefab files loading with zero migration. Encrypted `save`/`load` remain the player-save path (correct use of AEAD).
- **#6 via field addition, not enum variant** — `DebugShape` lacks `#[non_exhaustive]` (unlike `ReflectValue`, the repo's own precedent), so a new variant breaks exhaustive matchers. `FilledRect` in a `pub(crate)`-field struct sidesteps it entirely. Deprecate-don't-delete keeps external users compiling (with warnings) until v5.
- **P3 clone decision placed after culling, not at query time** — robustness over micro-elegance: the first *surviving* entity must carry the source or a culled carrier starves the pipeline compile.
- **Report committed to the repo** (was untracked) — follows the `docs/CODE_ANALYSIS.md` precedent; got a resolution-status header exactly like the 06-05 report's, so a future reader knows what's fixed without diffing.
- **The 06-05 report is sacrosanct** — restored after the synthesis agent clobbered it; it holds the historical 30/30 resolution map.
- **New chain (`code-analysis-2`), not a continuation** — the `code-analysis-fixes` chain (seq 4) declared itself closed ("nothing in this chain remains"); this is a different analysis round with different focus.
- **Ignore parallel agents' transient IDE diagnostics** — while A5 refactored `app.rs`, rust-analyzer streamed dozens of E0609 "no field" errors into my context from its half-done states. Reacting to them would have been wrong twice over (not my files; not final states). The agent's own gate is the only signal that matters. Same applies to a one-off `E0308` that appeared mid-wave-1 from A3's in-flight edits.
- **AskUserQuestion at the two real forks only** — (a) analysis focus + execution method, (b) batch scope + commit policy. Everything else (workflow design, #6 redesign, P3 fix shape) was decided unilaterally and reported, per the user's plan-and-go style.

## Evidence & Data

### Repo scope (measured at session start)

- `src/`: **130 `.rs` files, 27,325 lines**; `examples/`: 37 `.rs` files. Largest module dirs: renderer 4,373 / app 3,044 / ecs 2,087 / ui 2,073 / physics 1,557 / input 1,237 lines. Largest single files: `network.rs` 918, `prefab.rs` 657, `timeline.rs` 598, `behavior.rs` 580, `resources.rs` 565.

### Workflow architecture (REUSABLE for a 3rd analysis round)

- **Script persisted at** `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/276dcc9b-6ac7-4513-a9cd-7dd8cb31cf63/workflows/scripts/full-code-analysis-wf_972f001a-837.js` — invoke with `Workflow({scriptPath, args: {date, reportPath}})`. **`args` is required on EVERY invocation including resume** (the run-2 failure).
- Shape: `phase Analyze` = 10 module-group agents (pipeline) + 3 cross-cutting lens agents (parallel), run concurrently via `Promise.all`; each group's verifier fires as soon as its analyzer finishes (pipeline, no barrier); `phase Synthesize` = 1 Opus agent that Writes the Korean report and returns a compact summary. Schemas force StructuredOutput: FINDING = {title (unique join key), severity, files (file:line), detail, recommendation}; verifier echoes titles exactly with verdict ∈ {confirmed, downgraded, refuted}.
- Module-group partition (line-balanced, ~2-4.4k each):

| Group | Contents |
|---|---|
| renderer | `src/renderer/` (whole) |
| core-app | `src/app/`, `app.rs`, `lib.rs`, `resources.rs` |
| ecs | `src/ecs/`, `scene.rs`, `hierarchy.rs`, `pool.rs`, `history.rs` |
| ui | `src/ui/`, `locale.rs` |
| physics-collision | `src/physics/`, `src/collision/` |
| input-camera | `src/input/`, `camera.rs`, `components.rs`, `color.rs`, `material.rs` |
| animation | `src/animation/`, `skeletal.rs`, `timeline.rs`, `tween.rs`, `timer.rs`, `particle.rs`, `gpu_particle.rs` |
| assets-audio | `src/asset/`+`asset.rs`, `atlas.rs`, `tilemap.rs`, `src/audio/`+`audio.rs` |
| net-save-prefab | `network.rs`, `save.rs`, `prefab.rs` |
| ai-scripting | `behavior.rs`, `steering.rs`, `pathfinding.rs`, `src/scripting/`+`scripting.rs`, `reflect.rs`, `debug_ui.rs` |

- Cross-cutting lenses: `api-consistency` (lib.rs surface + consumer view via 2-3 example games), `coupling` (`use crate::` dependency map, fork-removability), `system-assembly` (App orchestration, implicit ordering deps vs PATTERNS.md).
- All agent prompts embed: VISION.md/PATTERNS.md must-read, "deviations from patterns are findings, the patterns themselves are not", "codebase passes clippy -D warnings — no style nits", "OUT OF SCOPE: bugs/tests/security".
- **Live monitoring trick**: workflow background tasks are NOT in TaskGet/TaskList (returns "Task not found"); progress was read from the journal — `grep -c '"type":"started"' / '"type":"result"'` on `…/subagents/workflows/<run>/journal.jsonl`, and agent count via `ls agent-*.jsonl | wc -l`.

### Workflow runs (analysis phase)

| Run | ID | Agents | Subagent tokens | Tool uses | Duration | Outcome |
|---|---|---|---|---|---|---|
| 1 | `wf_972f001a-837` | 14 launched | 0 | 0 | 1.7s | ALL failed — fable5 + `thinking.type.disabled` 400 |
| 2 | `wf_b70f5a06-9b9` | 23 | 1,474,252 | 1,123 | 639.6s | analyze+verify done; script threw at synthesis (`args` undefined — my relaunch omission) |
| resume | same run ID | 24 (23 cached) | 556,045 | 242 | 615.1s | synthesis only ran live; report written (to the wrong path — recovered) |

Total analysis cost ≈ **2.03M subagent tokens**. Resume cache restored 23 agents at zero token cost.

### Analysis results

- **70 surviving findings** (confirmed + downgraded), **3 fully refuted**, several downgraded with verifier notes.
- Per-module health (H/M/L of surviving findings): renderer 1/3/5 · core-app 0/4/5 · ecs 0/3/5 · ui+locale 0/1/4 · physics-collision 1/2/4 · input-camera 0/2/5 · animation+ 0/3/4 · assets-audio 0/2/4 · net-save-prefab 1/1/4 · ai-scripting 0/1/4.
- Report structure (7 sections): 종합 평가 / Top 10 우선순위 권고 / 아키텍처 (3.1 layer leaks as the recurring root cause, 3.2 design-time vs runtime save confusion, 3.3 editor-in-App, 3.4 scheduler unreachable from main registration path, 3.5 ECS upward deps ×2, 3.6 small boundary leaks) / 코드 품질 (4.1 per-frame allocs as ONE repeated pattern, 4.2 duplication, 4.3 vestigial APIs, 4.4 API consistency) / 모듈별 건강 요약 / 기각된 주장들 / 부록 (all 70).
- Synthesis verdict (one line): healthy against the "forkable skeleton" goal — cycle-free module DAG, opt-in physics/audio/network slabs, solid ECS core; weaknesses cluster into 3 recurring themes (type-home layer leaks; AEAD on design-time assets; editor-state entanglement), all local with cheap fixes.
- Verifier kills worth remembering (details in report §6): "exec_order needlessly cloned per frame" → REFUTED (the `catch_unwind(AssertUnwindSafe(..))` mutable capture makes the clone necessary; the suggested `for idx in 0..len()` fix doesn't compile for the same reason); "TilemapAtlas duplicates UV math" → REFUTED (both delegate to `UvRect::from_grid`; remaining String-vs-Handle difference is a deliberate decoupling choice); "LocalizationSystem clones unconditionally" → REFUTED (clones are inside `if let Some`, once per held component).
- Notable downgrades (real but overstated): axis-binding missing from `just_pressed_with_gamepad` (documented limitation with workaround at `map.rs:228-232` — not a silent trap); `Reflect::fields()` called before `is_enabled()` gate (the value IS consumed unconditionally later at `mod.rs:746-757`, so early-return impossible — claim half-right); bulk-downgraded as "real but skeleton-philosophy-acceptable": reflect coupling, `query_added` allocs, `AnimationPlayer` pub fields, `AssetServer` extensibility ("forking IS the intended extension path"), pathfinding↔Tilemap coupling (`new`+`set_walkable` escape hatch exists).

### Top-10 disposition

| # | Finding (sev after verify) | Disposition | Commit |
|---|---|---|---|
| 1 | Prefab/scene AEAD-encrypted, not human-editable (high) | **FIXED** — `write_ron`/`read_ron` + magic fallback | `0a30637` |
| 2 | rapier handle types leak through public API (high) | **DEFERRED → v5** (breaking: `BodyHandle`/`ColliderHandle` newtypes, follow `JointHandle` pattern) | — |
| 3 | `UvRect`/`BlendUv` live in animation, used by 6 modules (high) | **FIXED** — moved to `renderer::uv` + shims | `dcc54e2` |
| 4 | Per-frame bind-group creation in lighting pass (high) | **FIXED** — cached, ptr-identity + resize/reconfigure invalidation | `5245f4c` |
| 5 | ~17 editor fields mixed into runtime `App` (medium) | **FIXED** — `EditorState` extraction | `72b18ef` |
| 6 | Dual debug-draw APIs, merge unfinished (medium) | **FIXED (redesigned)** — `rect_filled_z` + deprecation | `fe40587` |
| 7 | `Lerp` defined in timeline, imported by network (medium) | **FIXED** — moved to `tween` + shim | `dcc54e2` |
| 8 | `Scene::on_enter` can't express labeled system order (medium) | **DEFERRED → v5** (breaking: `SystemRegistrar` wrapper) | — |
| 9 | Per-frame String/WGSL allocs in sprite render (medium) | **PARTIAL** — WGSL clone fixed (+P3 follow-up: once per hash); texture-key Strings need API break → v5 | `3cddc1e`, `7f12374` |
| 10 | `is_connected` wasm-only, missing on native (high) | **FIXED** — `AtomicBool`-backed parity | `1978b70` |

### Commit log (branch `fix/analysis-top10`, oldest first)

| Hash | Subject |
|---|---|
| `0a30637` | fix(prefab): plaintext RON for design-time scene/prefab files (analysis #1) |
| `5245f4c` | perf(lighting): cache lighting bind group instead of per-frame create (analysis #4) |
| `3cddc1e` | perf(sprite): stop cloning WGSL frag_source per frame (analysis #9, partial) |
| `1978b70` | fix(network): add is_connected() to native NetworkClient (analysis #10) |
| `72b18ef` | refactor(app): extract editor-only state into EditorState (analysis #5) |
| `dcc54e2` | refactor(arch): rehome UvRect/BlendUv to renderer, Lerp to tween (analysis #3, #7) |
| `fe40587` | refactor(debug): unify debug drawing on DebugDraw, deprecate DebugRect queue (analysis #6) |
| `50a85e6` | docs(v4.6.0): changelog, module map, analysis report for the Top-10 fix batch |
| `7f12374` | perf(sprite): clone new-material WGSL source once per hash, not per entity (analysis #9 follow-up) |

(`main` is at `370b0bb`; fork point. 9 listed + the docs commit = 10 total incl. version bump inside `50a85e6`.)

### Fix-wave agent usage (Sonnet unless noted)

| Agent | Item | Tokens | Tool uses | Duration | Self-verify result |
|---|---|---|---|---|---|
| A1 | #1 save/prefab | 43,530 | 19 | 148s | 22 passed (save::+prefab:: filter) |
| A2 | #4 lighting | 37,033 | 17 | 120s | check+clippy clean, lighting 5/5 |
| A3 | #9 sprite | 43,058 | 13 | 114s | check clean, sprite 12/12, clippy clean |
| A4 | #10 network | 34,634 | 7 | 109s | network 12/12 incl. new test |
| A5 | #5 EditorState | 65,982 | 35 | 217s | native+wasm check, clippy, app 9/9 |

### Native `is_connected` flag lifecycle (#10, A4's audit)

| Socket-thread event | ~network.rs region | Flag |
|---|---|---|
| `tungstenite::connect` Err | ~130 | stays false (never set) |
| Handshake OK, before `Connected` event | ~158 | `store(true)` |
| Binary/Text send error | ~172, ~181 | `store(false)` |
| `OutMsg::Close` (local disconnect) | ~190 | `store(false)` |
| Outbound channel `Disconnected` | ~197 | `store(false)` |
| Remote `Close` frame | ~244 | `store(false)` |
| I/O or protocol error | ~259 | `store(false)` |

### New tests added this session (+7 lib)

- `save::tests::write_ron_read_ron_roundtrip` — file is valid UTF-8 containing field names (`"hi_score"`, `"sfx"`)
- `save::tests::read_ron_backcompat_encrypted_file` — old `save()` output loads via `read_ron`
- `prefab::tests::scene_def_file_is_human_readable`, `prefab_file_is_human_readable` — plain-text assertions
- `prefab::tests::scene_def_backcompat_encrypted_load`, `prefab_backcompat_encrypted_load`
- `network::…::native_client_is_not_connected_before_handshake` — false before AND 200ms after a failed connect (`ws://127.0.0.1:1`)

### Debug-draw migration specifics (#6)

| Producer | Old | New call | Color / z preserved |
|---|---|---|---|
| `CollisionDebugSystem` | `DebugDrawQueue.items.extend` | `dbg.rect_filled_z(min, max, …)` per AABB | `rgba(0,1,0.2,0.25)`, z=999 |
| Editor gizmo highlight (`gizmo.rs`) | `dq.items.push(DebugRect{..})` | `dbg.rect_filled_z(..)` | `rgba(0.2,0.85,1.0,0.65)`, z=tr.z+999 |
| sokoban `RenderSystem` | `Vec<DebugRect>` + queue extend | `Vec<(Vec2,Vec2,Color,f32)>` + `rect_filled_z` loop | floor/walls/goals/boxes/player at z 0.0–0.4 |

Render drain: step 2.5 (deprecated queue → `DrawRect::with_z`, kept under `#[allow(deprecated)]` with a "until v5" comment) then step 2.6 (DebugDraw `shapes` via `debug_shape_to_draw_rects` + new `filled_rects` direct conversion). `lib.rs` exports the deprecated pair on a separate `#[allow(deprecated)] pub use` line; `core_resources.rs` still registers the queue.

### Verification matrix (final, on `cargo +1.88.0`)

| Gate | Result |
|---|---|
| `fmt --check` | OK |
| `clippy --all-targets -- -D warnings` | clean |
| `build --target wasm32-unknown-unknown` (lib+bins) | OK |
| `test --all-targets` | 373 passed / 0 failed (lib: 346, was 339) |
| `RUSTDOCFLAGS="-D warnings" doc --no-deps` | OK |
| `./scripts/wasm_smoke.sh` | PASS — connect + non-blank render, screenshot 41,977 B, eyeballed OK |

Intermediate gates also ran clean at each commit boundary (wave 1: plain-stable clippy `--lib` + 346 lib tests before the 4 commits; wave 1.5: clippy `--all-targets` + wasm `--lib` check before `72b18ef`/`dcc54e2`; #6: clippy `--all-targets` + `--all-targets` tests before `fe40587`).

### Memory state after this session

- `new-model-subagent-incompat` (NEW, type project) — fable5-as-subagent 400s on `thinking.type.disabled`; always set explicit `model:`; delete when harness fixed.
- `engine-current-state` (UPDATED) — now records: v4.6.0 batch on `fix/analysis-top10` (10 commits, itemized), lib tests 339→346, report committed with status header, the 06-05 report overwrite+restore incident, and the full v5.0.0 breaking-batch spec.
- `MEMORY.md` index has the new pointer line.

## Code Analysis

- `save.rs`: AEAD format magic is `R2DAEAD01` (`SAVE_MAGIC`); `read_ron` decides plaintext-vs-encrypted by sniffing it. `write_ron` creates parent dirs; both are `Unsupported` on wasm.
- `LightingRenderer` cache invalidation contract: `resize()` early-returns on same dims, otherwise recreates `normal_view` AND nulls `cached_bind_group`/`cached_scene_view_ptr`; `reconfigure()` rebuilds whole self. `run_pass(&mut self, device, encoder, scene_view, output_view)`. The cached ptr is `scene_view as *const wgpu::TextureView as usize`.
- Sprite material flow (post-P3): query collects `(Entity, hash:u64, params:[f32;4])` (hash = `DefaultHasher` over `frag_source`); entry loop filters (layer mask → visibility/LOD cull) THEN decides source carry via `seen_new_hashes`; a later pass compiles `!frag_source.is_empty() && !custom_pipelines.contains_key(hash)`; draw looks up pipelines by hash only.
- `NetworkClient` (native) connection flag: `connected: Arc<AtomicBool>`, `is_connected()` loads Acquire; thread stores Release at: post-handshake true; send-error/local-Close/channel-Disconnected/remote-Close/io-error false.
- `EditorState` lives at `src/app/editor/state.rs`, surfaced `pub(super) use state::EditorState` in `app/editor.rs`; `App.editor: EditorState` is cfg-split (struct exists on both targets; native-only fields cfg-gated inside).
- `DebugDraw` now: `shapes: Vec<DebugShape>` + `filled_rects: Vec<FilledRect>` (both `pub(crate)`); `clear()` clears both; render step 2.6 drains both, step 2.5 drains the deprecated queue under `#[allow(deprecated)]`.
- `renderer/uv.rs` holds `UvRect` (with `FULL`, `new`, `from_grid`, `from_pixels`, `flipped_x/y`) + `BlendUv` + the `uv_tests` module (moved verbatim).
- `tween.rs` now owns `pub trait Lerp: Clone { fn lerp(a,b,t) }` + impls for `f32`/`Vec2`/`[f32;4]`/`Color`.

## Files Changed

### Source (engine)
- `src/save.rs` — `write_ron`/`read_ron` + tests
- `src/prefab.rs` — SceneDef/Prefab save/load → plaintext path; tests
- `src/renderer/lighting.rs` — bind-group cache
- `src/renderer/sprite.rs` — WGSL clone elimination (twice: `3cddc1e` then `7f12374`)
- `src/network.rs` — native `is_connected` (+ `Lerp` import → tween)
- `src/app.rs`, `src/app/editor.rs`, `src/app/editor/state.rs` (NEW), `src/app/editor/ui/{mod,gizmo,shortcuts}.rs`, `src/app/scenes.rs` — EditorState
- `src/renderer/uv.rs` (NEW), `src/renderer/mod.rs`, `src/animation/player.rs`, `src/atlas.rs`, `src/tilemap.rs`, `src/renderer/ui.rs`, `src/components.rs` — UvRect/BlendUv move
- `src/timeline.rs`, `src/tween.rs` — Lerp move
- `src/resources.rs` — deprecations + `FilledRect` + `rect_filled/_z`
- `src/collision/debug.rs`, `src/app/render.rs`, `src/app/core_resources.rs`, `src/lib.rs` — debug-draw unification + re-export reshuffles

### Examples
- `examples/games/sokoban/sokoban.rs` — RenderSystem → `DebugDraw::rect_filled_z`
- `examples/games/shooter/shooter.rs` — doc-comment reference updated

### Docs & config
- `Cargo.toml`/`Cargo.lock` — 4.6.0
- `docs/CHANGELOG.md` — 4.6.0 entry
- `CLAUDE.md` — v1.5.0 header + 4 module-map rows
- `docs/CODE_ANALYSIS_2026-06-10.md` — NEW (committed analysis report + resolution header)
- `plans/handoffs/HANDOFF_code-analysis-2_top10-fix-batch_2026-06-10.md` — this file

## User Feedback & Preferences (REQUIRED — never omit)

- Session opener: "새로운 모델 나온 기념으로 전체 코드 분석 맡기고 싶어" — celebratory framing; wanted the NEW model (fable5) doing the work. Accepted the Sonnet-fanout/Opus-synthesis fallback once the harness incompat was explained, but the spirit was "new model analyzes my engine".
- Scope choice (AskUserQuestion): **architecture health + quality/simplification**; did NOT select bug audit or perf-hotspot options. Chose the multi-agent workflow knowing the stated cost estimate (수십만~100만 tokens; actuals ran ~2M — no complaint registered).
- Asked two precision questions mid-run ("새 모델은 fable5를 의미하는거지? 서브에이전트로 못돌린다는 의미?", "현재 작업은 모두 종료된 상태?") — wants exact understanding of infra failures and live status; answer with specifics, not reassurance.
- "Top10 수정 계획 세우고 진행 해줘" — plan AND execute in one request; no pause between.
- Batch scope: picked "배치 A만 진행" (recommended) — comfortable deferring breaking work; consistent with the repo's prior v3-breaking-batch-on-a-branch pattern.
- Commit policy: "브랜치 + 항목별 커밋" — per-item commits on a fix branch; push/PR on request only (then explicitly requested at close: "/handoff 하고 커밋 푸쉬").
- P3 handoff message: precise spec format (대상/문제/수정 목표/검증) handed from another agent; "남은 작업과 같이 묶어서 작업해줘" — fold follow-ups into the existing batch rather than opening new streams.
- Standing preferences honored: Korean conversation prose / English repo docs (`conversation-language-korean`, `doc-language-rule`); aggressive subagent use on Sonnet (`subagent-usage-preference`); `+1.88.0` gate (`ci-toolchain-pin`).

## Where We're Going

1. **Merge decision** — `fix/analysis-top10` is pushed but unmerged. Either PR → review → merge, or direct merge to `main` (repo has used both; v3 batch used PR #8). Ask the user which.
2. **v5.0.0 breaking batch** (next major work item, fully specified): #2 rapier `BodyHandle`/`ColliderHandle` newtypes (mirror existing `JointHandle` — propagate through factories, `PhysicsBody` pub fields, `cast_ray`, `move_character`, joints; keep raw accessors as documented escape hatch); #8 `on_enter` `SystemRegistrar` (replace raw `&mut Vec<Box<dyn System>>` param; enables labeled ordering from scenes; also convert one canonical example to `add_system_labeled`); remove `DebugDrawQueue`/`DebugRect`; remove `animation::player` + `timeline` shims; `Sprite.texture` → `Arc<str>` or interned IDs (kills the remaining per-sprite per-frame String clones, sites listed in `3cddc1e`'s message).
3. **rust-survivors pin bump** to v4.6.0 after merge (current pin v4.4.0 by rev — unaffected until bumped; batch is non-breaking so the bump should be a no-op migration; verify with the game's own gate per `rust-survivors-engine-pin`).
4. Optional: sweep the remaining 60 non-Top-10 findings (mostly low) from report §7 — many are one-liners; could be a cleanup session.
5. Optional: re-test fable5 subagent support after Claude Code updates; delete `new-model-subagent-incompat` memory when fixed.

## Risks & Blockers

- **Deprecation warnings for external users** — any fork using `DebugDrawQueue`/`DebugRect` compiles with warnings (intended pressure, but `-D warnings` forks will break — they can `#[allow(deprecated)]` or migrate).
- **`read_ron` fallback key assumption** — encrypted-file back-compat uses `SaveKey::DEFAULT`; scene/prefab files written pre-4.6 with a *custom* key (if any user did that via `save_with_key` manually) won't fall back. Considered acceptable: the engine's own SceneDef/Prefab API never took a key.
- **Pointer-identity cache** (#4) is correct for all *current* recreation paths (audited); a future new path that swaps the intermediate texture without calling `resize`/`reconfigure` would silently bind a stale view. The comment in `lighting.rs` documents the contract.
- **fable5 subagent incompat persists** — until the harness stops sending `thinking.type.disabled`, every `agent()`/Agent call from a fable5 main session MUST set `model:` explicitly or it 400s instantly.

## Open Questions

- Merge route for `fix/analysis-top10`: PR (repo precedent for batches) or direct merge? → user call next session.
- Should the v5 batch also make `DebugShape` `#[non_exhaustive]` while it's already breaking? (Cheap to add, future-proofs variant additions; `ReflectValue` precedent.)
- `seen_new_hashes` is per-`render()`-call — with multi-camera (multiple render calls per frame), a new material visible to two cameras clones once per camera on its first frame. One-frame cost, judged not worth threading state across calls. Revisit only if material churn becomes real.

## Quick Start for Next Session

```bash
# Current state
cd /Users/jkl/Projects/skeleton-engine
git log --oneline main..fix/analysis-top10   # the 10-commit batch
git status                                    # should be clean

# Canonical context
# - docs/CODE_ANALYSIS_2026-06-10.md  (findings + resolution header)
# - docs/CHANGELOG.md                 (4.6.0 entry = batch summary)
# - this handoff

# Key files if continuing into the v5 batch
# - src/physics/world/joints.rs       (JointHandle newtype pattern to copy for #2)
# - src/scene.rs + src/app/scenes.rs + src/ecs/schedule.rs  (#8 on_enter registrar)
# - src/renderer/sprite.rs            (texture-key String sites for Arc<str> change)

# Verify (CI pin — see memory ci-toolchain-pin)
cargo +1.88.0 fmt --check && cargo +1.88.0 clippy --all-targets -- -D warnings \
  && cargo +1.88.0 build --target wasm32-unknown-unknown \
  && cargo +1.88.0 test --all-targets \
  && RUSTDOCFLAGS="-D warnings" cargo +1.88.0 doc --no-deps

# Next action
# Ask the user: merge fix/analysis-top10 via PR or direct? Then rust-survivors pin bump.
```

## Session Closed
**Closed at:** 2026-06-10
**Commit:** branch tip of `fix/analysis-top10` (session commit `session: top10-fix-batch [code-analysis-2]`; last code commit `7f12374`), pushed to origin
**Session status:** Handed off to next session
