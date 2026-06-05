# Code-analysis remediation: PR #7 (non-breaking sweep) merged + v3.0.0 breaking batch (#11/#13) merged

**Date:** 2026-06-06
**Status:** COMPLETED (this session) — 17/30 analysis issues fixed across 2 merged PRs; main is now v3.0.0
**Bead(s):** none (bd unavailable)
**Epic:** code-analysis remediation (driven by `docs/CODE_ANALYSIS.md`)
**Chain:** `code-analysis-fixes` seq `2`
**Parent:** `HANDOFF_code-analysis-fixes_high-severity-bugs_2026-06-06.md` (seq 1)
**Prior chain:** `HANDOFF_code-analysis-fixes_high-severity-bugs_2026-06-06.md` > this

---

## Since Last Handoff

Seq 1 ended with the HIGH fixes applied-but-uncommitted on `main` and a planned "A = commit, B = MEDIUM #9/#10". What actually happened went much further:

- **A (commit) done as 2 commits on branch `fix/high-severity-bugs`** (not main directly): non-editor fixes + editor cluster. Commit 1 verified green in isolation. Opened **PR #7**, CI 4/4 green, **merged via rebase** to main (linear history preserved).
- **B (MEDIUM) expanded well beyond #9/#10:** did **#9, #10, #14 (query_mut), #16, #17, #21** — six MEDIUM items, all in PR #7.
- **Then escalated to the deferred breaking work:** the user chose to start the **v3.0.0 batch**. Branch `feat/v3-breaking-api`, bumped to 3.0.0, implemented **#13 (PhysicsWorld→resource)** and **#11 (Color newtype, 37 files)**. Opened **PR #8**, CI 4/4 green, **merged via rebase**.
- **Toolchain trap from seq 1 fully resolved & internalized:** every verification this session used `cargo +1.88.0` (CI's pinned rustfmt 1.8.0), never local stable. Saved as memory `ci-toolchain-pin`.
- **Two subagents used** (per CLAUDE.md "code after long conversation") for #13 and #11; both produced clean work but **reported success while rust-analyzer showed stale E0308/E0061 errors** — every time the real `cargo +1.88.0 build` was clean. Lesson: independently re-verify subagent output with a real build, ignore stale rust-analyzer diagnostics.

Net: seq 1's "fix the HIGH bugs" goal is fully shipped, plus 8 MEDIUM, plus the entire breaking batch. 17 of 30 analysis issues are resolved and on main.

## Reference Documents

- `docs/CODE_ANALYSIS.md` — **the 30-issue analysis report** (severity-ranked; the remaining-work backlog). On main.
- `HANDOFF_code-analysis-fixes_high-severity-bugs_2026-06-06.md` (seq 1) — the analysis run + HIGH-fix details + the full MEDIUM/LOW backlog table with file:line + the toolchain investigation. On main.
- `CLAUDE.md` — module map + the 5-command verification gate (note: it uses default toolchain; see ci-toolchain-pin caveat).
- `docs/VISION.md` — skeleton/forkable thesis; "feature not done until an example exercises it"; semver (breaking → major bump).
- Memory: `ci-toolchain-pin` (verify with `+1.88.0`), `v3-breaking-batch` (this batch — now done), `rust-survivors-engine-pin`, `subagent-usage-preference`, `doc-language-rule`.

## The Goal

Remediate the issues surfaced by the full-codebase analysis (`docs/CODE_ANALYSIS.md`) so the "skeleton" engine stays trustworthy for forkers (VISION priority 1). This session shipped the entire HIGH set + a large MEDIUM set as a non-breaking PR, then the two breaking API improvements (Color unification, PhysicsWorld-as-resource) as a deliberate v3.0.0 batch. The remaining ~13 issues (perf + polish LOW + a few additive-API items) are the next session's work.

## Where We Are

- **main HEAD = `d278f57`, version `3.0.0`, working tree clean.** Both PRs merged; no open PRs; feature branches deleted.
- **Session commit range:** `b754577..d278f57` = 9 commits (see Evidence).
- **PR #7 (merged, non-breaking):** `64fd9e9` fix(engine) HIGH #1/#4/#5/#6 + guards #24/#25/#26 + analysis doc; `c0a3ea5` fix(editor) #2/#3 + #1 test + mod.rs rustfmt normalization; `9157e8d` feat(save) #9; `839ac6f` feat(schedule) #10; `b73801f` fix #16/#17/#21; `3893f27` feat(ecs) #14.
- **PR #8 (merged, BREAKING, v3.0.0):** `1e7f91b` feat(physics)! #13; `13c70ae` feat(color) Color foundation; `d278f57` feat(color)! #11.
- **Test count:** lib went ~282 → **293** (PR #7 added 7 tests; Color added 4); aggregate `test --all-targets` = **296 passed, 0 failed**; **33 doctests**.
- **`Color` newtype** is the new single color representation (`src/color.rs`): `rgb/rgba/rgba_u8/hex`, `WHITE/BLACK/RED/GREEN/BLUE/TRANSPARENT`, `From<[f32;4]>/[f32;3]/[u8;4]`, `to_array/to_u8/to_rgb`. 37 files migrated.
- **`PhysicsWorld` is now a World resource;** `PhysicsSystem::new(pixels_per_unit)` (no physics arg); `run()` does remove→step→insert.
- **`World::query_mut::<T>()`** added (single-component mutable iteration).
- **Built-in `SystemLabel` consts** on AnimationSystem/StateMachineSystem/BlendTreeSystem/LayoutSystem/UiSystem.
- **`save`/`load`/`delete` wasm-gated** with `SaveError::Unsupported`; crypto native-only.
- **17/30 issues resolved.** Remaining 13: MEDIUM #7,#8,#12,#15,#18,#19,#20,#22 and LOW #23,#27,#28,#29,#30 (see seq-1 handoff's backlog table for file:line + fix sketch).
- **`docs/CODE_ANALYSIS.md` still lists the resolved items as open** — not updated to mark fixes (intentional: it's the as-of-analysis snapshot; git log is the source of truth).
- **rust-survivors not yet migrated to v3** — it pins engine by git rev and (grep-checked) doesn't reference physics or color APIs, so impact looks minimal; verify before bumping its pin.

## What We Tried (Chronological)

1. **Committed the seq-1 HIGH fixes as a split PR.** On main → branched `fix/high-severity-bugs`. Could not use `git add -p` (interactive, unsupported), so split by WHOLE FILES along compile boundaries: commit 1 = non-editor fixes + analysis doc (editor files at HEAD); commit 2 = editor cluster (app.rs + editor.rs + gizmo.rs + mod.rs) + #1 scene test. Verified commit 1 green **in isolation** by stashing the editor changes. Worked.
2. **Continued MEDIUM in PR #7:** #9 (save wasm-gate), #10 (SystemLabel consts), #16/#17/#21 (physics one-way handle, particle under-emit, checkbox release-toggle), #14 (query_mut). Each verified under +1.88.0 then committed.
3. **#9 clippy snag:** `return` inside `#[cfg]` blocks tripped `clippy::needless_return` (the cfg-removed remaining block is the tail). Fix: tail expressions, no `return`. (Local default stable did NOT catch this; `+1.88.0` did.)
4. **#14 query_mut design:** single-component `(Entity, &mut T)` via destructuring `let Archetype { entities, columns, .. } = arch` for disjoint borrows. Skipped `query2_mut` — two mutable columns from one `HashMap<TypeId, Vec<Box>>` needs unstable/unsafe `get_disjoint_mut`.
5. **PR #7 merged (rebase, --delete-branch).** CI 4/4 green (confirmed Package dry-run too, ~5min). main fast-forwarded.
6. **#11/#13 confirmed breaking → user chose "v3.0.0 batch".** Created `feat/v3-breaking-api`, bumped Cargo.toml to 3.0.0.
7. **#13 via subagent (sonnet):** move PhysicsWorld out of PhysicsSystem into a World resource; `run()` remove/insert; migrate platformer + crane. Subagent reported pass; **rust-analyzer showed E0061**; my real `cargo +1.88.0 build --all-targets` was **clean** (stale diagnostics). Independently verified all 5 checks green. Committed.
8. **#11 strategy refinement (key insight):** make color constructor/builder params `impl Into<Color>` so array call-sites keep compiling via `From` — then ONLY struct-literal `color:` fields break. Created `src/color.rs` myself (design-critical: Reflect/serde/render boundaries), verified (293 lib + 33 doc), committed as foundation.
9. **#11 migration via subagent (opus, omitted model for reliability):** 37 files (23 src + 14 examples). Subagent again reported pass while rust-analyzer showed a wall of E0308; real build = **0 errors**. Independently ran all 6 checks: green (296 tests). Committed.
10. **Color `.into()` ambiguity discovered:** simba (via rapier) impls `From<[T;N]>`, so bare `arr.into()` whose target isn't fixed → E0283. Hit it in my own Color test (`assert_eq!([..].into(), Color::GREEN)`); fixed with `Color::from(..)`. Documented in the commit + memory as a migration rule.
11. **PR #8 marked ready, CI watched with `gh pr checks 8 --watch`, merged on green (rebase).** main → d278f57, v3.0.0.

## Key Decisions

- **Split PR #7 by whole files, not `git add -p`** (interactive unsupported), ordered so each commit compiles (scenes.rs #1-fix in commit 1, its test in commit 2 — test passes because the fix already landed). Per-commit greenness verified for commit 1 via stash.
- **Accept the ~700-line rustfmt churn in `src/app/editor/ui/mod.rs`** (rustfmt 1.8.0 reprocessing a previously-skipped block once edited). It's the only CI-passing option once the file is touched; real change ~15 lines (`git diff -w`).
- **#14 = single-component query_mut only.** query2_mut deferred (needs unstable/unsafe). Additive, non-breaking.
- **#11 constructors take `impl Into<Color>`** to minimize example churn. Rejected: changing only field types (would break every array call-site).
- **#11 Reflect via existing `ReflectValue::Color([f32;4])` widget** (`to_array`/`Color::from` at the boundary) instead of 4×F32 fields — preserves the combined-color inspector UX. (Subagent deviation, accepted.)
- **`Color` is RGBA f32 with named fields** (serde struct form) — breaks Sprite/scene RON repr; acceptable for v3.0.0.
- **`WindowConfig.clear_color` left `[f64;4]`** (wgpu clear-color f64 space; Color has no f64 conv) — out of scope.
- **Rebase-merge both PRs** to keep main's linear history (matches the repo's existing style).
- **Independently re-verify all subagent work** with a real `cargo +1.88.0` build; treat rust-analyzer diagnostics as possibly-stale.

## Evidence & Data

### Session commits (`b754577..d278f57`)

| SHA | Commit | PR |
|-----|--------|----|
| `64fd9e9` | fix(engine): scene-transition leak, GPU buffer leak, anim hang, float guards (#1/#4/#5/#6/#24/#25/#26) | #7 |
| `c0a3ea5` | fix(editor): repair undo/redo, honor register_component (#2/#3 + #1 test) | #7 |
| `9157e8d` | feat(save): wasm-gate save/load + SaveError::Unsupported (#9) | #7 |
| `839ac6f` | feat(schedule): SystemLabel consts for built-ins (#10) | #7 |
| `b73801f` | fix: one-way handle leak, particle under-emit, checkbox toggle (#16/#17/#21) | #7 |
| `3893f27` | feat(ecs): World::query_mut (#14) | #7 |
| `1e7f91b` | feat(physics)!: PhysicsWorld → World resource (#13) | #8 |
| `13c70ae` | feat(color): add Color newtype (foundation) | #8 |
| `d278f57` | feat(color)!: migrate all color fields to Color (#11) | #8 |

### Issues resolved (17/30)

| Severity | Resolved | Remaining |
|----------|----------|-----------|
| HIGH (6) | #1 #2 #3 #4 #5 #6 | — |
| MEDIUM (16) | #9 #10 #11 #13 #14 #16 #17 #21 (8) | #7 #8 #12 #15 #18 #19 #20 #22 (8) |
| LOW (8) | #24 #25 #26 (3) | #23 #27 #28 #29 #30 (5) |

### Verification — every check under Rust 1.88.0 (CI's pinned toolchain)

Both PRs: `fmt --check` · `clippy --all-targets -D warnings` · `test --all-targets` (296) · `test --doc` (33) · `build --target wasm32-unknown-unknown` · `doc -D warnings`. CI on both PRs: **4/4 jobs green** (Test native ~2m28s, Package dry-run ~5m, Rustdoc ~37s, WASM ~24s).

### #11 migration scope: 37 files, +590/−315 (incl. PR #8 totals)

23 src (color.rs new; components/atlas/resources/particle/gpu_particle/timeline/prefab + renderer{ui,text,sprite,geometry,ui_primitives,lighting} + app/render + collision/debug + editor/gizmo + 7 ui widgets) + 14 examples.

### Subagent runs

| Task | Model | tokens | tool_uses | duration | outcome |
|------|-------|-------:|----------:|----------|---------|
| #13 PhysicsWorld resource | sonnet | ~71K | 38 | ~5.6m | clean (stale RA diagnostics) |
| #11 Color migration | opus | ~207K | 272 | ~28m | clean (stale RA diagnostics) |

## Code Analysis

- `Color` (`src/color.rs`): `struct Color { r,g,b,a: f32 }`, `const fn rgb/rgba`, `rgba_u8`, `hex(0xRRGGBB)`, consts, `to_array/to_u8/to_rgb`, `From<[f32;4]>/[f32;3]/[u8;4]` + `From<Color>` for each. Re-exported `engine::Color`.
- `PhysicsSystem` (`src/physics/system.rs`): no longer owns physics; `run()` `let Some(mut physics) = world.remove_resource::<PhysicsWorld>() else { return }; physics.step(dt); …; world.insert_resource(physics);`. Game systems needing physics + components simultaneously use the same remove/insert pattern (e.g. platformer `move_character`).
- `World::query_mut::<T>(&mut self) -> impl Iterator<Item=(Entity,&mut T)>` via archetype field destructuring (`src/ecs/world.rs`).
- `panicked_systems` (`HashSet<usize>`) cleared in `reload_scene`, pruned in `apply_scene_cmd` Pop (`src/app/scenes.rs`).
- `params_buffers` pruned by live `mat_ids` set at end of `SpriteRenderer::render` (`src/renderer/sprite.rs`).
- Built-in labels: `AnimationSystem::LABEL = "engine::animation"` etc. (`crate::ecs::schedule::SystemLabel = &'static str`).
- GPU/glyphon Pod structs (`InstanceRaw`, `GpuParticle`, lighting/fade uniforms) stay `[f32;_]`; Color→array conversion at call sites only.

## Files Changed (this session)

All committed & merged to main. New files: `src/color.rs`, `docs/CODE_ANALYSIS.md` (seq 1), `plans/handoffs/HANDOFF_code-analysis-fixes_high-severity-bugs_2026-06-06.md` (seq 1). See the commit table for the per-commit file groupings; `git show <sha> --stat` for details. This handoff + its PLAN are the only new uncommitted files (committed at session close).

## User Feedback & Preferences

- **"두가지 다 진행"** / **"a 진행 다음 b 진행"** / **"남은 선택 모두 진행"** / **"a 실행"** / **"CI 그린 확인하고 PR #8 머지"** — strong, terse "keep going / do everything / merge it" energy throughout. The user authorized each consequential step (commit, push, PR, merge) explicitly.
- **Korean for conversational summaries**, English for artifacts (doc-language rule). Honored.
- Chose **"v3.0.0 배치로 분리"** then **"착수"/"실행"** — wanted the breaking changes done as a deliberate v3 batch, and then to actually execute them.
- Earlier turned on **workflows + ultracode** and fixed a subscription/login issue — appetite for thorough multi-agent work, token cost not a constraint.
- Standing prefs (memory): subagents aggressively for parallel/heavy work (Sonnet preferred); verify before declaring done.

## Where We're Going

The remaining 13 analysis issues, grouped for the next session (see the PLAN file for phase detail). All non-breaking EXCEPT #28 (JointHandle newtype) and arguably #29 (ReflectValue `#[non_exhaustive]`).

- **Performance (non-API):** #7 `Arc<rhai::AST>` + reusable buffers; #8 single sprite render pass; #18 A* closed-set + scratch reuse; #19 `Arc<SpatialGrid>` (avoid per-frame deep clone) + CollisionDebugSystem reads the mirror.
- **Subsystem robustness (non-API):** #15 point-light radius unit fix + visual-size test; #20 built-in `AudioSystem` driving `update()` + SFX cache; #22 RenderLayer/layer_mask negative-fold fix; #23 `spawn_scene_def` duplicate-tag warn.
- **API polish (mostly additive):** #27 re-export `MouseButton` + fix example internal-path imports + Korean docs; #12 coordinate-convention docs + `DrawText::centered`/anchor helpers; #30 conservative Rhai string/array/recursion limits; #29 `ReflectValue` `#[non_exhaustive]` + `I32`; #28 wrap rapier `ImpulseJointHandle` in an engine `JointHandle` newtype (BREAKING — batch with a future v3.x or hold).

## Remaining Backlog (13 issues — feeds the PLAN)

From `docs/CODE_ANALYSIS.md` + seq-1's backlog table. **Line numbers predate the #11 Color
migration — re-grep before editing (several of these files were touched).** Severity per analysis.

### MEDIUM (8)

| # | Issue | Location (pre-#11) | Fix sketch | Breaking? |
|---|-------|--------------------|------------|:---:|
| #7 | Per-frame Rhai AST deep-clone + 4×`Arc<Mutex>` per scripted entity | `src/scripting/execution.rs:46,53-58` | `Arc<rhai::AST>` (clone = refcount bump) + per-`ScriptRunner` reusable buffers (drop the `Mutex`) | no |
| #8 | New render pass opened per texture-run AND per material | `src/renderer/sprite.rs` (the draw loop, was ~626-647/665-688) | begin ONE pass for the whole pre-sorted stream; `set_pipeline`/`set_bind_group`/`draw_indexed` per run | no |
| #12 | Screen-vs-world coord convention undocumented; flagship examples mis-use it | `src/renderer/text.rs`, `sprite/ui_primitives.rs`, `src/camera.rs` | anchor enum / `DrawText::centered` helper + prominent docs; the #6 fixes already centered loading_bar/minimap | partial |
| #15 | Point-light radius unit mismatch → lights render ~½ size | `src/renderer/lighting.rs:97,148` | make CPU `radius_ndc` and the shader agree on one space (`2*radius/viewport_w`); add a visual-size test | no |
| #18 | A* has no closed-set → duplicate heap entries, re-expansion, per-call re-alloc | `src/pathfinding.rs:173-201` | visited/closed set (or skip stale pops by g-score); reuse the 3 scratch collections across calls | no |
| #19 | `CollisionGridSystem` deep-clones two HashMaps every frame | `src/collision/grid.rs:197` | wrap in `Arc<SpatialGrid>`, store an `Arc::clone`; `CollisionDebugSystem` (`debug.rs:50`) should read the mirror not rebuild | **no** (resource type changes `SpatialGrid` → `Arc<SpatialGrid>` — check readers; may be mild breaking) |
| #20 | Audio `update()` not driven by any built-in system; no SFX caching | `src/audio/playback.rs:122,188` | register a built-in `AudioSystem` that ticks `update(dt)`; cache decoded bytes (or route via AssetServer) | no (additive) |
| #22 | `RenderLayer(i32)` vs `layer_mask(u32)` clamp folds negatives to bit 0 | `src/renderer/sprite/sort.rs:88-94`, `src/components.rs` | restrict masking to a documented non-negative range (warn on negative) or bias the i32 onto distinct bits | partial |

### LOW (5)

| # | Issue | Location | Fix sketch | Breaking? |
|---|-------|----------|------------|:---:|
| #23 | `spawn_scene_def` silently overwrites duplicate tags | `src/prefab.rs:301-303` | `log::warn!` on duplicate + validate in `SceneDef::load` | no |
| #27 | `MouseButton` not re-exported; examples use internal/winit paths + Korean docs; `gpu_particles.rs:23` `.unwrap()` | `src/lib.rs` | `pub use winit::event::MouseButton`; convert examples to top-level imports + English docs | no (additive) |
| #28 | rapier `ImpulseJointHandle` leaks through the public API | `src/physics/mod.rs:10` | wrap in an engine `JointHandle` newtype (like `CollisionGroups`) | **YES** — hold for a v3.x/own batch |
| #29 | `ReflectValue` closed enum (no `#[non_exhaustive]`, no `I32`) | `src/reflect.rs` | `#[non_exhaustive]` + add `I32`. NB: #11 added/used a `ReflectValue::Color` variant — confirm current variants first | mild |
| #30 | Incomplete Rhai resource limits (only `max_operations`) | `src/scripting/api.rs:9-12` | conservative max string/array/map size + call/expr-depth via a `ScriptingLimits` config | no (additive) |

## Detailed Files Changed (per commit, this session)

- `64fd9e9` (PR#7): `src/app/scenes.rs`, `src/renderer/sprite.rs`, `src/animation/system.rs`, `src/camera.rs`, `src/timeline.rs`, `src/timer.rs`, `examples/{loading_bar,minimap}.rs`, `docs/CODE_ANALYSIS.md` (new).
- `c0a3ea5` (PR#7): `src/app.rs` (editor fields + #1 test), `src/app/editor.rs`, `src/app/editor/ui/{gizmo,mod}.rs` (mod.rs = ~15 logic + ~680 rustfmt churn), `plans/handoffs/HANDOFF_...high-severity-bugs...md` (seq 1).
- `9157e8d` (PR#7): `src/save.rs`, `src/asset/script_loading.rs`.
- `839ac6f` (PR#7): `src/animation/{system,state_machine,blend_system}.rs`, `src/ui/{system,panel}.rs`, `src/app.rs` (label test).
- `b73801f` (PR#7): `src/physics/world/body_factory.rs`, `src/particle.rs`, `src/ui/system/checkbox_pass.rs`.
- `3893f27` (PR#7): `src/ecs/world.rs`, `src/ecs/world/tests.rs`.
- `1e7f91b` (PR#8): `Cargo.toml`+`Cargo.lock` (3.0.0), `src/physics/system.rs`, `examples/{games/platformer/platformer,crane_wrecking_ball}.rs`.
- `13c70ae` (PR#8): `src/color.rs` (new), `src/lib.rs`.
- `d278f57` (PR#8): 37 files (see Evidence) — the Color field migration.

## Risks & Blockers

- **rustfmt mismatch (live):** local default ≠ CI (1.8.0). ALWAYS `cargo +1.88.0`. `scripts/verify.sh`/CLAUDE.md use default — the trap (memory `ci-toolchain-pin`).
- **Subagent over-reporting:** both subagents claimed pass amid stale rust-analyzer errors. Mitigation: always re-run a real `cargo +1.88.0 build --all-targets` and the full gate independently before committing subagent work.
- **rust-survivors v3 migration:** main is now breaking v3.0.0. If its pin is bumped, Color + physics-setup changes apply (grep says it uses neither directly — verify). Test via `--config` path patch (memory `rust-survivors-engine-pin`).
- **#28 is breaking** — don't fold it into a non-breaking PR; needs a v3.x major or its own batch.
- **Editing `src/app/editor/ui/mod.rs` re-triggers the ~700-line rustfmt churn** — isolate real changes with `git diff -w`.

## Gotchas & Lessons (reusable, cost real time this session)

- **rust-analyzer diagnostics go stale after big batch edits.** Both subagents' edits triggered walls of E0061/E0308 in the diagnostics panel while the actual `cargo +1.88.0 build --all-targets` was 0 errors. Trust the compiler, not the inline diagnostics, right after large edit bursts.
- **Always verify subagent output yourself.** Both subagents reported "all pass" — true, but only confirmable by re-running the full `+1.88.0` gate. Treat subagent self-reports as unverified.
- **rustfmt pin (1.8.0 via Rust 1.88.0) ≠ local stable (1.9.0).** `cargo fmt` (default) reformats chains differently and can fail CI. Use `cargo +1.88.0 fmt`. Also: editing `src/app/editor/ui/mod.rs` re-triggers a ~700-line rustfmt normalization (it was committed in a state rustfmt left untouched).
- **`clippy::needless_return` fires on `return` inside `#[cfg]` blocks** — after cfg strips the other branch, the remaining block is the function tail, so `return` is "needless". Use tail expressions. Default stable clippy did NOT flag this; `+1.88.0` did.
- **`[u8;4]`/`[f32;4]` `.into()` is ambiguous in physics-linked crates** — simba/nalgebra (via rapier) impl `From<[T;N]>`, so a bare `arr.into()` whose target type isn't fixed → E0283. In field-init / `impl Into<Color>` params the target IS fixed (fine); elsewhere use `Color::from(arr)` / explicit constructors.
- **`git add -p` is unsupported here (interactive).** To split a commit, stage by whole files along compile boundaries and order commits so each compiles (put a fix before the test that asserts it).
- **Verify per-commit greenness by stashing later commits' files** (`git stash push -- <files>` → run gate → `git stash pop`).
- **Merge style:** both PRs used `gh pr merge N --rebase --delete-branch` (fall back to `--merge`) to keep main's linear history. `gh pr checks N --watch --interval 20` blocks until CI resolves and exits non-zero on failure — chain it with `&&` to merge only on green.

## Open Questions

- Should `docs/CODE_ANALYSIS.md` be updated to mark the 17 resolved items (or add a "resolved in PR #7/#8" column)? Currently it's the as-of snapshot.
- Bump rust-survivors' engine pin to v3 now, or wait? (Low risk per grep.)
- Hold #28 (breaking) for a future batch, or do all remaining non-breaking items as one PR and list #28 separately?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine

# Restore context
cat plans/handoffs/HANDOFF_code-analysis-fixes_v3-breaking-batch_2026-06-06.md   # this file
cat plans/handoffs/HANDOFF_code-analysis-fixes_high-severity-bugs_2026-06-06.md  # seq 1 (backlog table w/ file:line)
cat docs/CODE_ANALYSIS.md                                                        # the 30-issue report

git log --oneline b754577..HEAD     # this session's 9 commits
grep '^version' Cargo.toml           # 3.0.0

# Key files for the first perf phase (#7/#8/#18/#19)
#   src/scripting/execution.rs (#7), src/renderer/sprite.rs (#8),
#   src/pathfinding.rs (#18), src/collision/grid.rs (#19)

# Verify starting state — MUST use the pinned toolchain (NOT default stable)
cargo +1.88.0 fmt --check && cargo +1.88.0 clippy --all-targets -- -D warnings && cargo +1.88.0 test --all-targets

# First action: branch, then start Phase 1 (#19 Arc<SpatialGrid> is the smallest/cleanest win)
git checkout -b fix/analysis-perf
# edit src/collision/grid.rs: store Arc<SpatialGrid> in the resource instead of self.grid.clone()
```
