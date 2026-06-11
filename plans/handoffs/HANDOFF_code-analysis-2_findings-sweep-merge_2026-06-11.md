# Non-Top-10 findings sweep (~30 items) + PR #12 merged to main + rust-survivors pin bump to v4.6.0

**Date:** 2026-06-11
**Status:** COMPLETED — the entire 2026-06-10 analysis round is now CLOSED for non-breaking work: ~30 appendix findings fixed in 10 commits on `fix/analysis-top10`, branch merged to `main` via **PR #12** (merge commit `59c0845`, CI 4/4 green), and **rust-survivors pinned to v4.6.0** (`f18c5b0`, game gate green, true no-op migration). Only the fully-specified v5.0.0 breaking batch remains.
**Bead(s):** none (`bd` not installed in this environment — `command not found`; tracked via conversation)
**Epic:** code-analysis remediation, round 2 (`docs/CODE_ANALYSIS_2026-06-10.md`)
**Chain:** `code-analysis-2` seq `2`
**Parent:** `HANDOFF_code-analysis-2_top10-fix-batch_2026-06-10.md`
**Prior chain:** `HANDOFF_code-analysis-2_top10-fix-batch_2026-06-10.md` > this

---

## Since Last Handoff

- Parent's "Where We're Going" #1 (merge decision) — **resolved differently than planned**: user picked the PR route, but `gh` was unauthenticated (401, both gh CLI and GitHub MCP) and the user was on a **remote connection** (couldn't do interactive login at first), so the PR was deferred mid-session… then un-deferred: the user ran `! gh auth login` (device flow, code `4A1B-EEC3`) later in the session, auth succeeded, and PR #12 was created AND merged the same session.
- Parent's #4 (optional sweep of the remaining 60 non-Top-10 findings) — **became the session's main work** (user picked it from a 3-option AskUserQuestion over starting the v5 batch or the fable5 retest). ~30 items fixed, the rest explicitly dispositioned (v5 / feature-work / wontfix).
- Parent's #3 (rust-survivors pin bump after merge) — **DONE**: 4.4.0 → 4.6.0 (`59c0845`), commit `f18c5b0` pushed, game gate green (fmt / clippy `-D warnings` / 200 lib tests), zero deprecated-API usage in the game.
- Parent's #2 (v5.0.0 breaking batch) — NOT started (correct per plan); its scope GREW with sweep-triage deferrals (see Where We're Going).
- Parent's open question "merge route: PR or direct?" — answered: **PR with merge commit** (preserves the 20 bisect-buildable per-item commits; v3-batch PR #8 precedent).
- Parent's open question "`DebugShape` `#[non_exhaustive]` in v5?" — still open, carried forward.
- Parent's risk "fable5 subagent incompat persists" — confirmed still true; all 8 sweep agents ran with explicit `model: sonnet` per `new-model-subagent-incompat` memory; zero failures.
- Parent's risk "deprecation warnings for external users" — extended: this session added **4 more deprecations** (`register_reflect`, `NetworkEvent::JsonParseError`, `App::load_texture`, `ParticleEmitter::for_burst`), all removal-planned for v5.

## Reference Documents

- `docs/CODE_ANALYSIS_2026-06-10.md` — analysis report; resolution header now has a second paragraph covering the sweep (what was fixed / deferred to v5 / split off as features / wontfixed).
- `docs/CHANGELOG.md` — `## 4.6.0` entry extended with sweep items (new Added bullets, "Findings-sweep cleanups" Changed bullet, new **Fixed** section, expanded Deprecated section).
- `docs/PATTERNS.md` — gained three sections this session: "Per-frame scratch buffers (allocation convention)", "System ordering with labels" (+ constraints table), "Add a custom asset type (fork extension point)"; stale `Box::new(...)` examples fixed.
- `CLAUDE.md` — doc v1.5.1 (particle row: `for_burst()` → `burst()`).
- Parent handoff — full Top-10 batch context, workflow architecture for a 3rd analysis round, v5 spec.
- Memory: `engine-current-state` (updated 3× this session), `new-model-subagent-incompat`, `ci-toolchain-pin`, `rust-survivors-engine-pin`, `subagent-usage-preference`, `conversation-language-korean`.

## The Goal

Close out the 2026-06-10 full-codebase analysis round (70 verified findings) without touching public-API compatibility. The parent session shipped the Top-10 batch (10 commits); this session's mandate grew through three user choices: (1) sweep the ~60 remaining appendix findings — fix everything non-breaking, disposition the rest; (2) once auth was solved mid-session, push + PR + merge the whole 20-commit branch to `main`; (3) bump rust-survivors to the merged v4.6.0 and prove the batch is genuinely non-breaking against a real consumer. All three are done; the engine's only remaining analysis debt is the v5.0.0 breaking batch.

## Where We Are

- **`main` = `59c0845`** (merge commit of PR #12, pushed). Branch `fix/analysis-top10` fully merged: 10 parent-session commits + 10 sweep commits, every one bisect-buildable. Working tree clean. Remote branch NOT deleted (optional cleanup).
- **PR #12** (https://github.com/ChunSam/skeleton-engine/pull/12) — body covers both phases + verification matrix; **lib-test count in the body was corrected post-creation** (I wrote 353 unverified; actual `cargo +1.88.0 test --lib` = **348**; fixed via `gh pr view 12 --json body | sed | gh pr edit --body-file`).
- **GitHub Actions CI on the PR: 4/4 pass** — Test (native) 3m37s, Build (WASM) 1m32s, Package dry-run 8m37s, Rustdoc 43s (run 27312728285).
- **Sweep scope**: 27 findings dispatched to 8 Sonnet agents in 2 waves (25 done, 2 honest skips), plus a main-session wave: LABEL constants ×14, platformer labeled-ordering demo, 3 PATTERNS.md sections, CHANGELOG/report/CLAUDE.md docs.
- **Test counts**: all-targets **373 → 375** (+2 new `physics::character` tests from the max_slope_angle fix); lib **346 → 348**. Zero failures at every gate.
- **Behavior fixes shipped (2)**: animation main-clip frame advance now `while`-catches-up on large dt (was `if`, max 1 frame/tick; crossfade path already correct); `CharacterController::max_slope_angle` direct field assignment now syncs into rapier's `inner` controller at `move_character` time (was silent desync unless `with_max_slope_deg` was used).
- **Per-frame allocation pass**: text queue drained via `std::mem::take` (was full Vec clone); `PhysicsSystem` event-diff got a generic private `diff_pairs` helper + 4 per-frame temporaries promoted to private scratch fields (`col_map`, `current_contacts`, `current_intersections`, `body_pairs` — clear+extend); single-pass particle emitters (burst data folded into `EmitterSnapshot`, was double scan + 2 Vecs); single-pass panel layout (local `PanelSnapshot` captures layout+background data in one `query2`); editor-UI `entity_list`/`tag_map`/`selected_comp_names`/scene-graph allocs moved behind `is_enabled()` (`comp_fields` stays unconditional — consumed at ~`mod.rs:752` for inspector write-back, verified constraint from analysis §6); `exec_order` take/swap-back in `app/schedule.rs` (clone was borrow-forced; take+restore avoids the alloc).
- **O(1) `despawn`**: new private `entities_row: HashMap<Entity, usize>` on `World`, maintained in `spawn()`/`despawn()` with swap_remove index patching (was linear `iter().position()` scan).
- **`topological_sort_entities` rehomed** `prefab.rs` → `hierarchy.rs`; `pub use crate::hierarchy::topological_sort_entities;` shim left in prefab so `lib.rs:95` root re-export and the editor call site (`app/editor/ui/mod.rs:709`) compile unchanged.
- **LABEL constants on all 14 remaining built-in systems** (joining the 5 that had them — UiSystem, LayoutSystem, AnimationSystem, StateMachineSystem, BlendTreeSystem): `engine::physics`, `engine::collision_grid`, `engine::collision_debug`, `engine::network`, `engine::particle`, `engine::tilemap`, `engine::audio`, `engine::skeletal_animation`, `engine::hierarchy`, `engine::steering`, `engine::behavior`, `engine::localization`, `engine::scripting`, `engine::timeline`. Inserted via a python script anchored on `^impl (crate::ecs::)?System for <Type>` with per-type doc comments (ordering hints where a real constraint exists).
- **platformer example** now registers `AnimationSystem`/`StateMachineSystem` via `add_system_labeled` + `.after(AnimationSystem::LABEL)` — first in-repo demonstration of the labeled API (the report flagged "LABEL never demonstrated").
- **New API surface (all additive)**: `SceneChange::take`/`is_pending`, `ShouldQuit::quit`/`is_quitting` (`.0` stays pub; examples NOT migrated, deliberate), `ParticleEmitter::burst` (canonical; `for_burst` deprecated + delegates; internal test caller and survivor/shooter examples migrated), `NetworkConfig` root re-export, `DrawImage::colored` doc (constructor turned out to already exist — agent added the missing doc only).
- **4 new deprecations** (since="4.6.0", removal v5): `World::register_reflect` (stores empty type name → breaks Inspector), `NetworkEvent::JsonParseError` (never emitted; `#[allow(deprecated)]` added at the one match site, `examples/mp_client.rs:103`), `App::load_texture` (vestigial vs `load_image`; NOTE: `SpriteRenderer::load_texture` in `renderer/sprite/textures.rs` is an unrelated internal method, untouched), `ParticleEmitter::for_burst`.
- **Structural dedup**: `App::new()` single struct literal with `#[cfg]`-gated field initializers (was ~90-line duplicated native/wasm literal; 6 native-only fields cfg-gated inline, `EditorState` init split into cfg'd `let` bindings); `AssetServer::new()` single literal via `(Option<Watcher>, Option<Receiver>)` match (Err arm's spurious second channel pair gone); editor Tag-name-editor and Ctrl+click multi-select blocks → private helpers (`tag_name_editor`, `apply_multiselect`); input `bind`/`bind_gamepad_button`/`bind_gamepad_axis` → `bindings_for` helper; fullscreen-quad vertex shader → shared `src/renderer/shaders/fullscreen_quad.wgsl` (fade + lighting `concat!(include_str!(..), fragment)`).
- **Dead code removed**: private `play_streaming` (audio/playback.rs, 34 lines + `#[allow(dead_code)]` + stale doc reference in audio.rs); dead `shape_type` binding (character_movement.rs).
- **Docs-only fixes**: SAFETY comment on the `egui_pass.rs` transmute (investigated — sound, no dangling ref; `er`/`rpass` don't escape), wasm no-op notes on fades (app.rs field + resources.rs FadeTransition), `CollisionGroups` vs `CollisionLayer` distinction, CollisionDebugSystem ordering, LocaleResource↔LocalizationSystem bridge, `query_added`/`query_changed` alloc trade-off, components.rs migration-facade marker, scripting Seek/Flee-only note, `AudioEffect::release_secs` "not yet applied" honesty note, move_entity clone rationale comments, `spawned_ids`/`BbEntry` dead-data docs.
- **`./scripts/wasm_smoke.sh` PASS** post-sweep: client connect logged, screenshot 41,974 bytes (≥15,000), eyeballed `/tmp/wasm_smoke.png` — HUD text, player square, 6 coins, footer all correct (essentially identical to parent session's 41,977-byte frame).
- **rust-survivors**: `crates/game/Cargo.toml` rev `f62df900b50c3d3a72f4fd951d77db5271eb96d8` → `59c0845a658587cfb7b7271b592e1284a5e65e1e`; `cargo update -p skeleton-engine` (4.4.0→4.6.0, 23 deps unchanged); gate `+1.88.0` fmt ✓ / clippy `--all-targets -D warnings` ✓ / `test -p game --lib` **200/200** (same baseline as the 4.4.0 bump); commit `f18c5b0` pushed. **The game tree had (and still has) pre-existing uncommitted doc changes** (AGENTS.md, CLAUDE.md, survivor README, deleted docs/ENGINE_FOLLOWUPS.md, IMAGE_*/NEXT_WORK_PLAN docs) — NOT mine, deliberately left unstaged and untouched.
- **Memory**: `engine-current-state` updated 3× (sweep done → merged → pin bumped); MEMORY.md index line updated twice to match.

## What We Tried (Chronological)

1. **Onboarding per the user's 5-step narration protocol** (read handoff → claim beads → state verification plan → read key files + 2-3 unlisted adjacent → propose first action, then WAIT). `bd` turned out not installed (`command not found`) — consistent with parent ("bd empty"). Baseline verified bit-identical to parent close (branch in sync, 10 commits ahead of `main`@`370b0bb`), so the 5-gate re-run was deliberately deferred to pre-merge. Found during adjacent reading: **#2's blast radius is wider than the parent lists** — beyond factories/raycast/joints, `set_one_way`/`is_one_way`/`has_contact`/`rigid_body[_mut]`/`get_collider[_mut]`/`set_collision_groups`/`collision_groups` all take raw rapier handles (world.rs:191-271), `RaycastHit.collider_handle` is a pub field (world.rs:98), and grep shows 6 src files + 2 examples (`crane_wrecking_ball`, `platformer`) touch raw handles. Recorded for the v5 batch.
2. **gh auth probe**: `gh pr list` → HTTP 401; GitHub MCP `get_me` → also 401 (checked deliberately before asking the user for credentials — would have been the zero-info path). Offered device-flow-via-`!` (recommended, token never enters transcript) vs PAT (with explicit transcript-exposure + revoke-after warning). User: remote connection → deferred PR entirely.
3. **Sweep triage** (read the full report §7 appendix, 70 findings). Bucketed the ~60 non-Top-10: **IN ~30** (non-breaking fixes/docs), **OUT-breaking→v5** (pub-visibility narrowings: GpuLightData/LightingUniforms, PostProcessRenderer.target_view/normal_view, TouchState fields, input submodule `pub`; SystemConfig/SystemMeta merge; ShaderMaterial source_hash caching; Sprite texture keys), **OUT-features** (StateMachine `crossfade_duration`, scripting Arrive/Wander bindings, AudioEffect release-envelope impl), **OUT-judged** (DefaultHasher stability — in-memory cache, irrelevant; World↔Reflect decoupling — opt-in, design-change scale).
4. **Pre-plan scouting greps** (parent's precedent — verify facts the plan hinges on): `ShaderMaterial` has pub fields AND a documented struct-literal construction example in its own doc comment → adding a private `source_hash` field breaks literal construction → **breaking, moved to v5** (this killed the "cache the hash" sweep item). `CharacterController.max_slope_angle` is a pub field → getter conversion breaking → invented the non-breaking lazy-sync fix instead. `PhysicsSystem` already has private fields → scratch promotion safe. `topological_sort_entities` is root-re-exported (lib.rs:95) + used by editor UI and prefab tests → move needs a shim. `play_streaming` is private → deletable. `for_burst` callers: particle.rs:328 test + survivor.rs:860 + shooter.rs:538. `App::load_texture` (app/assets.rs:22) has zero callers; the same-named `SpriteRenderer::load_texture` is unrelated.
5. **Wave 1 — 6 parallel Sonnet agents on disjoint file sets** (R renderer / E ecs+scripting / P physics / U ui / A animation+particle / F assets+audio), every prompt carrying: explicit file whitelist, non-breaking constraint, "no repo-wide cargo fmt" (an agent reformatting another agent's mid-edit file would corrupt its Edit-tool state), "no git commit", "ignore transient compile errors in files you don't own", repo patterns (mem::take precedent at app/render.rs:459, shim precedent, borrow workaround), and targeted self-verify commands. All six returned; 25/27 items done.
6. **Honest skips from wave 1 (both correct calls)**: E found `move_entity`'s two TypeId-Vec clones are **borrow-checker-forced** (both need `&type_set` keys while the same `Vec<Archetype>` element's `columns` is `&mut` — `split_at_mut` can't split one element) → rationale comments added, matching the parent's refuted "exec_order clone" pattern. U declined the panel-background→UiSystem-pass move (draw-order preservation not provable cheaply; `z − 0.01` under-children invariant documented on the bypass instead) — the prompt explicitly sanctioned this fallback.
7. **Partial from wave 1**: E confirmed `spawned_ids` is write-only (written in `scripting/api.rs`, cleared in `scripting/execution.rs`, never read) but those files were outside its whitelist → documented as dead storage w/ TODO instead of removing. `BbEntry.key` turned out NOT redundant (read in the `bb_buf` path, `execution.rs:154-156`) — the report's claim was half-right; documented the dual-context constraint.
8. **Cross-agent interference, handled as designed**: U, A, F all hit transient compile errors from P's in-flight `diff_pairs` refactor (`physics/system.rs:140,172` signature mismatches) during their self-verify runs; all three correctly reported them as foreign-file noise and verified their own files only. P's final gate was clean. Same playbook as the parent session's E0609 lesson.
9. **Wave-1 combined gate then 6 thematic commits staged per agent file-set** (`git add <set> && git commit`, parent's partitioning recipe): fmt ✓ → clippy `--all-targets` ✓ → **375/0** all-targets → wasm build ✓ → doc ✓ → commits `b073dc2`/`e6ba04b`/`070c07e`/`0b890d8`/`4548a98`/`fafa404`.
10. **Wave 2 — 2 parallel agents** (I app/editor 7/7; N api-surface 10/10). N deviated once, justifiably: deprecating `JsonParseError` required `#[allow(deprecated)]` in `examples/mp_client.rs` (outside its whitelist; the only external match site) — accepted and committed. N also discovered `DrawImage::colored` already existed (report stale) → doc-only. **One gate failure I fixed myself**: N's `send_text` len-precapture used `"... ({} bytes)", len` → 1.88.0 clippy `uninlined_format_args` under `-D warnings`; inlined to `{len}` (network.rs:315). The IDE's lingering mp_client deprecation diagnostics after N's allow were stale — the clippy `--all-targets` pass (which compiles examples) is the real signal. Commits `7378f0e`/`74a02ed`.
11. **Wave 3 — main session.** LABEL×14 via anchored-regex python insertion (assert exactly 1 match per type — would have caught doc-comment false positives like `debug_ui.rs:10`); platformer conversion (`SystemConfig` import + 2 labeled registrations with explanatory comment); PATTERNS.md 3 new sections + stale `Box::new` fixes (the recipes claimed `add_system(Box::new(...))`; actual API takes unboxed); CHANGELOG (incl. new **Fixed** section), report header sweep paragraph (Korean, matching report language), CLAUDE.md v1.5.1. Full 5-gate green; commits `7e00df1`/`21a3f92`. Then `wasm_smoke.sh` PASS + screenshot eyeball.
12. **Auth un-blocked mid-session**: user ran `! gh auth login` (background task) → device code `4A1B-EEC3` surfaced from the task output file → relayed to user with "browser can be your LOCAL machine even on remote connection". The background process itself later died with `context deadline exceeded`, but auth had completed through it/another path — `gh auth status` showed ChunSam with repo scope, so the failure notification was correctly dismissed as stale.
13. **Push + PR + self-caught error**: pushed `1fae1df..21a3f92`; `gh pr create` → **PR #12**. Immediately re-verified the numbers I'd put in the body: `cargo +1.88.0 test --lib` = **348**, but I'd written "lib 339 → 353" (un-verified arithmetic) → fixed with `gh pr edit 12 --body-file`. Lesson: never write a metric into an artifact without running the command that produces it.
14. **CI watch → merge**: `gh pr checks 12 --watch --interval 20` as a background task; 4/4 pass. AskUserQuestion (merge commit recommended vs squash vs hold) → user picked **merge commit** (20 bisect-buildable commits preserved; squash would destroy exactly what the per-item commit discipline bought). `gh pr merge 12 --merge` → `main` = `59c0845`; local main ff-synced.
15. **rust-survivors bump**: rev edit via python (full 40-char hash from `git rev-parse 59c0845`), `cargo update -p skeleton-engine`, game gate (fmt/clippy/200 lib tests — clippy `-D warnings` passing proves zero deprecated-API usage, so the 4 new deprecations cost the game nothing), surgical `git add crates/game/Cargo.toml Cargo.lock` (user's unrelated uncommitted doc changes left strictly alone), commit `f18c5b0`, push.

## Key Decisions

- **Sweep over v5-batch-start** (user choice from 3-option AskUserQuestion; sweep was my recommendation): non-breaking work could land on the existing unmerged branch and ride the same PR; starting v5 on top of an unmerged base would stack branches.
- **Sweep commits onto the SAME `fix/analysis-top10` branch** — folded into one PR with the Top-10 batch; option text said exactly this and the user picked it.
- **Triage discipline: pub-visibility narrowing = breaking here.** Because every module is `pub mod` in lib.rs (parent's finding), even non-re-exported `pub` items are reachable via deep paths → narrowings deferred to v5 wholesale rather than judged item-by-item.
- **`ShaderMaterial` hash caching killed by literal-construction check** — its own doc comment shows `ShaderMaterial { frag_source, params }` construction; a private field addition breaks that. Per-frame hash of the WGSL source stays for now; v5 fixes it together with the texture-key interning.
- **`max_slope_angle`: sync-at-use instead of getter** — pub field can't be removed (breaking); making `move_character` write the field into `inner.max_slope_climb_angle`/`min_slope_slide_angle` makes the field authoritative with zero API change. 2 new tests pin the lazy-sync semantics (inner intentionally stale until `move_character`).
- **Deprecate-don't-remove for all 4 vestigial APIs** — same policy as parent's DebugRect decision; forks with `-D warnings` can `#[allow]` or migrate; removals batched for v5.
- **LABEL naming `engine::<snake_case>`** — follows the 5 pre-existing constants exactly; ordering doc comments only where a real constraint exists (collision_grid, collision_debug, network, hierarchy, localization) to avoid inventing constraints.
- **Merge commit, not squash** (user choice, my recommendation): per-item commits were engineered bisect-buildable across both sessions; squash erases that. v3 batch (PR #8) precedent.
- **PR body metric self-correction over silent confidence** — when the 353 figure didn't trace to a command output, ran the command and edited the published PR rather than letting a wrong number stand in a permanent artifact.
- **Agents never run repo-wide `cargo fmt`** (new rule this session, encoded in every prompt): fmt rewrites other agents' mid-edit files → their Edit-tool old_string anchors go stale → corruption. Main session runs fmt once at gate time instead.
- **rust-survivors commit kept surgical** — only Cargo.toml + Cargo.lock staged; the user's pre-existing uncommitted doc edits in that repo are someone's in-flight work, not mine to bundle.

## Evidence & Data

### Sweep commit log (`fix/analysis-top10`, sweep portion, oldest first)

| Hash | Subject |
|---|---|
| `b073dc2` | perf(renderer): drain text queue via mem::take, dedup fullscreen-quad shader |
| `e6ba04b` | refactor(ecs): rehome topological_sort_entities, O(1) despawn, deprecate register_reflect |
| `070c07e` | perf(physics): diff_pairs helper + scratch-buffer reuse; fix max_slope_angle desync |
| `0b890d8` | perf(ui): single-pass panel layout, hoist TextInput get_mut out of char loop |
| `4548a98` | fix(animation): while-loop frame catch-up; perf(particle): single-pass emitters; ParticleEmitter::burst |
| `fafa404` | refactor(assets): single AssetServer::new construction path, remove dead play_streaming |
| `7378f0e` | refactor(app): dedupe editor UI blocks + App::new literal, gate editor allocs, exec_order take/swap |
| `74a02ed` | refactor(api): surface-consistency batch — ShouldQuit accessors, NetworkConfig re-export, deprecations |
| `7e00df1` | feat(schedule): LABEL constants on all built-in systems, labeled ordering demo + docs |
| `21a3f92` | docs(v4.6.0): changelog + analysis-report status for the findings sweep |

Then: merge commit `59c0845` (PR #12 → main). rust-survivors: `f18c5b0` (pin 4.4.0 → 4.6.0).

### Agent usage (all Sonnet, explicit `model:` per fable5 incompat)

| Agent | Scope | Items | Tokens | Tool uses | Duration | Outcome |
|---|---|---|---|---|---|---|
| R | renderer (text/fade/lighting/shaders) | 2 | 59,321 | 39 | 268s | 2/2 done |
| E | ecs/scene/scripting | 8 | 57,630 | 50 | 317s | 6 done, 1 borrow-forced skip, 1 partial(doc) |
| P | physics + collision docs | 5 | 44,409 | 24 | 224s | 5/5 done (+2 tests) |
| U | ui + locale | 5 | 44,672 | 28 | 174s | 4 done, 1 conservative skip(doc) |
| A | animation/particle + 2 examples | 4 | 36,122 | 26 | 207s | 4/4 done |
| F | assets/audio | 3 | 29,637 | 28 | 194s | 3/3 done |
| I | app/editor (wave 2) | 7 | 65,298 | 41 | 282s | 7/7 done |
| N | API surface (wave 2) | 10 | 42,615 | 44 | 195s | 10/10 done (1 lint fixed by me after) |
| **Σ** | | **44 dispatched / ~30 distinct findings** | **379,704** | 280 | 2 waves | 25+17 item-level done |

### Verification matrix (final, `cargo +1.88.0` unless noted)

| Gate | Wave 1 | Wave 2 | Wave 3 (final) |
|---|---|---|---|
| `fmt --check` | OK | OK | OK |
| `clippy --all-targets -- -D warnings` | clean | clean after 1 fix (`uninlined_format_args`, network.rs:315) | clean |
| `build --target wasm32-unknown-unknown` | OK | OK | OK |
| `test --all-targets` | 375/0 | 375/0 | 375/0 |
| `RUSTDOCFLAGS="-D warnings" doc --no-deps` | OK | — | OK |
| `./scripts/wasm_smoke.sh` | — | — | PASS (41,974 B screenshot, eyeballed) |

### GitHub CI (PR #12, run 27312728285)

| Job | Result | Duration |
|---|---|---|
| Test (native) | pass | 3m37s |
| Build (WASM) | pass | 1m32s |
| Package dry-run | pass | 8m37s |
| Rustdoc | pass | 43s |

### rust-survivors pin-bump gate (`+1.88.0`)

| Check | Result |
|---|---|
| `cargo update -p skeleton-engine` | 4.4.0 → 4.6.0, 23 deps unchanged |
| `fmt --check` | OK |
| `clippy --all-targets -- -D warnings` | clean ⇒ **zero deprecated-API usage in game** |
| `test -p game --lib` | **200/200** (identical to 4.4.0-bump baseline) |

### Sweep disposition summary

| Bucket | Count (approx) | Contents |
|---|---|---|
| Fixed (code) | ~22 | per-frame allocs, dedups, O(1) despawn, rehoming, behavior fixes, LABEL×14, API additions |
| Fixed (docs-only) | ~8 | SAFETY comment, wasm no-op notes, naming distinctions, ordering docs, honesty notes |
| Deprecated (removal v5) | 4 | `register_reflect`, `JsonParseError`, `App::load_texture`, `for_burst` |
| Deferred → v5 (breaking) | ~7 | visibility narrowings ×4 groups, SystemConfig/Meta merge, ShaderMaterial source_hash, TouchState accessors |
| Split off as features | 3 | StateMachine crossfade, scripting Arrive/Wander, AudioEffect release impl |
| Wontfix/judged | 2 | DefaultHasher stability, World↔Reflect decoupling |
| Honest skips (documented in code) | 2 | move_entity clones (borrow-forced), panel-background pass move |

### New LABEL constants (full list)

`engine::physics`, `engine::collision_grid`, `engine::collision_debug`, `engine::network`, `engine::particle`, `engine::tilemap`, `engine::audio`, `engine::skeletal_animation`, `engine::hierarchy`, `engine::steering`, `engine::behavior`, `engine::localization`, `engine::scripting`, `engine::timeline` (pre-existing: `engine::ui`, `engine::ui_layout`, `engine::animation`, `engine::animation_state_machine`, `engine::blend_tree`).

### Item-level sweep map (what landed where)

| Finding (report §7) | Fix shape | File(s) | Agent |
|---|---|---|---|
| Text queue Vec cloned per frame | `std::mem::take` (lifecycle verified: queue doc says cleared-after-render) | renderer/text.rs:269-275 | R |
| Fullscreen vtx shader duplicated | shared `fullscreen_quad.wgsl` + `concat!(include_str!, frag)` | renderer/fade.rs, lighting.rs, shaders/ (NEW) | R |
| topological_sort_entities in prefab | moved to hierarchy.rs + `pub use` shim | hierarchy.rs, prefab.rs | E |
| register_reflect dead API | `#[deprecated(since="4.6.0")]` + doc | ecs/world.rs | E |
| SceneChange asymmetric visibility | `take()` + `is_pending()` added | scene.rs | E |
| despawn linear scan | `entities_row: HashMap<Entity,usize>` O(1) | ecs/world.rs | E |
| move_entity TypeId Vec clones | SKIP — borrow-forced (same-element &/&mut); comments | ecs/world.rs:773,795 | E |
| query_added/changed alloc | doc trade-off note | ecs/world.rs:638-663 | E |
| spawned_ids write-only / BbEntry key | doc'd (removal needs api.rs/execution.rs — outside set; key IS read in bb_buf path) | scripting/context.rs | E |
| scripting Seek/Flee only | doc note (fork extension point) | scripting.rs | E |
| PhysicsSystem event-diff duplication + per-frame allocs | `diff_pairs` free fn + 4 scratch fields | physics/system.rs:75-161 | P |
| dead shape_type binding | removed | physics/world/character_movement.rs:30-40 | P |
| max_slope_angle mirror desync | sync-at-use in move_character + field doc + 2 tests | physics/character.rs, character_movement.rs | P |
| CollisionGroups vs CollisionLayer confusion | doc distinction paragraph | physics/world.rs | P |
| CollisionDebugSystem ordering | "# System ordering" doc | collision/debug.rs | P |
| Panel set queried twice | single pass, local `PanelSnapshot` | ui/panel.rs:74-118 | U |
| Panel bg bypasses UiOutput | SKIP (conservative) — bypass doc'd, z−0.01 invariant stated | ui/panel.rs | U |
| text_input get_mut per char | hoisted; events buffered then appended | ui/system/text_input_pass.rs:92-117 | U |
| Localization split non-obvious | bridging docs both sides | ui/localized.rs, locale.rs | U |
| ViewportSize manual deref | `.copied()` ×2 | ui/system/state.rs, ui/panel.rs | U |
| Main-clip `if` vs crossfade `while` | `while` + non-looping break guard (timer zeroed) | animation/system.rs:86-101 | A |
| frame_dur duplicated | private `frame_dur(fps)` helper | animation/system.rs:14-21 | A |
| ParticleSystem double scan | single pass; burst folded into `EmitterSnapshot` (`has_burst`, `burst_remaining`) | particle.rs | A |
| for_burst naming outlier | `burst()` canonical + deprecation + 3 callers migrated | particle.rs:79, survivor.rs:860, shooter.rs:538 | A |
| AssetServer::new triple literal + Err-arm channel waste | single literal via match tuple | asset.rs:199-218 | F |
| play_streaming dead | deleted (private; +unused imports, stale audio.rs doc ref) | audio/playback.rs:297-330 | F |
| release_secs unread | honest "not yet applied" doc | audio/types.rs:16 | F |
| Tag name-editor ×2 / Ctrl+click ×2 | `tag_name_editor` / `apply_multiselect` helpers (cfg-gated) | app/editor/ui/mod.rs | I |
| update_editor_ui unconditional allocs | 6 collections moved behind `is_enabled()`; comp_fields stays (write-back at ~:752) | app/editor/ui/mod.rs:19-51 | I |
| egui_pass transmute no SAFETY | investigated sound; SAFETY comment | app/egui_pass.rs:29-33 | I |
| exec_order clone per frame | take → iterate → restore (+ safety comment) | app/schedule.rs:201 | I |
| App::new dual literal | single literal, cfg-gated initializers ×6 + cfg'd EditorState binding | app.rs:235-326 | I |
| FadeTransition wasm no-op | docs (field + type) | app.rs, resources.rs | I+N |
| NetworkConfig not at root | re-export added | lib.rs | N |
| JsonParseError never emitted | variant deprecated + `#[allow]` at only match site | network.rs:24, examples/mp_client.rs:103 | N |
| send_text String clone | len precapture (native + wasm) | network.rs:276-284 | N |
| ShouldQuit `.0` leak | `quit()`/`is_quitting()` (examples NOT migrated) | resources.rs | N |
| DrawImage::colored missing | already existed (stale report) → doc added | renderer/ui.rs | N |
| load_texture vestigial | App method deprecated (renderer's same-name method untouched) | app/assets.rs:22 | N |
| bind ×3 or_insert_with dup | `bindings_for` helper | input/map.rs:118-170 | N |
| Camera inline path / components facade | `use` import / facade doc | camera.rs:48, components.rs:309-312 | N |
| LABELs missing on most systems | 14 constants + ordering hints | 14 files (see list below) | main |
| LABEL never demonstrated | platformer labeled registration + comment | examples/games/platformer/platformer.rs | main |
| take/swap convention unwritten | PATTERNS.md "Per-frame scratch buffers" | docs/PATTERNS.md | main |
| AssetServer extensibility | PATTERNS.md "Add a custom asset type" recipe | docs/PATTERNS.md | main |

### Auth/PR/merge timeline

| Event | Detail |
|---|---|
| gh CLI + GitHub MCP both 401 | probed before asking user for anything |
| PR deferred | user on remote connection, couldn't browser-auth at that moment |
| `! gh auth login` (background) | device code `4A1B-EEC3`; bg process later `context deadline exceeded` (stale failure — auth had succeeded) |
| `gh auth status` | ChunSam, keyring, scopes gist/read:org/repo/workflow |
| push | `1fae1df..21a3f92` |
| PR #12 created | body metric corrected 353→348 post-hoc |
| CI 4/4 | watched via `gh pr checks --watch` bg task |
| merged | `gh pr merge 12 --merge` → `59c0845` |

## Code Analysis

- `World.entities_row: HashMap<Entity, usize>` (private) — row index into the entities Vec; `despawn` does `entities_row.remove(&entity)` → `swap_remove` → patch the swapped entity's row. ECS suite (48 tests) green.
- `PhysicsSystem` scratch fields: `col_map`, `current_contacts`, `current_intersections`, `body_pairs` — private, `clear()`+`extend()` per frame; `diff_pairs(previous, current, col_map, started, stopped)` is a free function taking `&mut Vec<(Entity,Entity)>` outputs to avoid double-borrow.
- `move_character` now begins by writing `controller.max_slope_angle` → `inner.max_slope_climb_angle`/`min_slope_slide_angle` (field is authoritative; inner intentionally stale between calls — 2 tests pin this).
- `exec_order` take/swap safety argument: `add_system`/`add_system_labeled` are the only `schedule_dirty` setters and are not callable from inside a running system (systems get `&mut World`, not `&mut App`) → restored Vec can't clobber a mid-loop recompute.
- Fullscreen shader sharing: `src/renderer/shaders/fullscreen_quad.wgsl` holds `VOut` + `vs_main`; fade's fragment takes no args (extra `uv` interpolant legally discarded per WGSL spec); lighting's fragment consumes `in.uv`. Both native-only; wasm unaffected.
- `AssetServer::new()` native arm now computes `(Option<RecommendedWatcher>, Option<Receiver<PathBuf>>)` in one match, single `Self{..}` literal after.
- `App::new()` single literal: 6 native-only field initializers cfg-gated inline (`lighting_renderer`, `fade_renderer`, `scene_texture_for_lighting`, `post_texture_for_lighting`, `gpu_particle_renderer`, `gilrs`); `EditorState::new()` is native-only so its binding is cfg-split before the literal.
- `egui_pass.rs:29-33` transmute verdict: lifetime extension where both `er` and `rpass` outlive the call and don't escape — sound; SAFETY comment documents the invariant.
- LABEL insertion pattern: separate inherent `impl <Type> { pub const LABEL: crate::ecs::schedule::SystemLabel = "..."; }` block immediately before the `impl System for` block (multiple inherent impls are valid; matches ui/system.rs style).
- rust-survivors consumes the engine as `engine = { package = "skeleton-engine", git = "https://github.com/ChunSam/skeleton-engine", rev = "<full-40-char>" }` in `crates/game/Cargo.toml:20`.

### Process/CLI patterns that worked (reusable)

- **Multi-agent same-tree concurrency rules** (encode in EVERY agent prompt): explicit file whitelist; "do NOT run repo-wide `cargo fmt`" (would rewrite other agents' mid-edit files and break their Edit anchors); "do NOT git commit"; "IGNORE transient compile errors in files you don't own". Concurrent `cargo` invocations serialize on the target-dir lock — slow but safe.
- **Per-agent thematic commits from one tree**: gate once (fmt → clippy --all-targets → wasm → test → doc), then `git add <agent's file set> && git commit` per agent. Worked twice now (parent: 4+1 agents; this session: 6+2).
- **Anchored-regex bulk insertion** (LABEL×14): python script matching `^impl (crate::ecs::)?System for <Type> \{` with `assert len(matches)==1` per file — the assert is what makes it safe (catches doc-comment false positives like debug_ui.rs:10's commented impl).
- **Background watch patterns**: `gh pr checks <n> --watch --interval 20 > log 2>&1` as a `run_in_background` Bash → notification on completion; user-side `! gh auth login` runs as a background task whose output file (`/private/tmp/claude-501/.../tasks/<id>.output`) can be Read mid-flight to extract the device code.
- **PR body edit after publish**: `gh pr view 12 --json body -q .body | sed 's/old/new/' > /tmp/body.md && gh pr edit 12 --body-file /tmp/body.md`.
- **Test-count summing across targets**: `cargo test --all-targets 2>&1 | grep -E "^test result" | awk -F'[ ;]+' '{p+=$4; f+=$6} END {print p, f}'` (individual `test result:` lines are per-target; the lib line alone under-reports).

## Files Changed

### skeleton-engine — source (sweep)
- `src/renderer/text.rs` (mem::take), `src/renderer/fade.rs` + `src/renderer/lighting.rs` + `src/renderer/shaders/fullscreen_quad.wgsl` (NEW — shared vertex stage)
- `src/ecs/world.rs` (entities_row O(1) despawn, register_reflect deprecation, query_added/changed docs, move_entity comments), `src/hierarchy.rs` (topo-sort home + LABEL), `src/prefab.rs` (shim), `src/scene.rs` (take/is_pending), `src/scripting/context.rs` + `src/scripting.rs` (dead-data docs, Seek/Flee note)
- `src/physics/system.rs` (diff_pairs + scratch + LABEL), `src/physics/character.rs` (max_slope sync + 2 tests), `src/physics/world/character_movement.rs` (dead binding, sync write), `src/physics/world.rs` (CollisionGroups doc), `src/collision/debug.rs` (ordering doc + LABEL), `src/collision/grid.rs` (LABEL)
- `src/ui/panel.rs` (single-pass + .copied()), `src/ui/system/state.rs` (.copied()), `src/ui/system/text_input_pass.rs` (get_mut hoist), `src/ui/localized.rs` (bridge doc + LABEL), `src/locale.rs` (bridge doc)
- `src/animation/system.rs` (while catch-up + frame_dur helper), `src/particle.rs` (single-pass + burst()/for_burst deprecation + LABEL)
- `src/asset.rs` (single-literal new), `src/audio/playback.rs` (play_streaming removed), `src/audio/types.rs` (release_secs doc), `src/audio.rs` (stale doc + LABEL)
- `src/app.rs` (single-literal new, fade doc), `src/app/schedule.rs` (exec_order take/swap), `src/app/editor/ui/mod.rs` (2 helpers + gated allocs), `src/app/egui_pass.rs` (SAFETY)
- `src/network.rs` (JsonParseError deprecation, send_text len, LABEL, {len} lint fix), `src/lib.rs` (NetworkConfig), `src/resources.rs` (ShouldQuit methods, FadeTransition doc), `src/renderer/ui.rs` (DrawImage::colored doc), `src/app/assets.rs` (load_texture deprecation), `src/input/map.rs` (bindings_for), `src/camera.rs` (use import), `src/components.rs` (facade doc)
- `src/skeletal.rs`, `src/steering.rs`, `src/behavior.rs`, `src/tilemap.rs`, `src/timeline.rs`, `src/scripting/execution.rs` (LABEL each)

### skeleton-engine — examples & docs
- `examples/games/platformer/platformer.rs` (labeled registration demo), `examples/games/survivor/survivor.rs` + `examples/games/shooter/shooter.rs` (burst migration), `examples/mp_client.rs` (#[allow(deprecated)])
- `docs/PATTERNS.md` (3 new sections + Box::new fixes), `docs/CHANGELOG.md` (4.6.0 extended + Fixed section), `docs/CODE_ANALYSIS_2026-06-10.md` (sweep status paragraph), `CLAUDE.md` (v1.5.1, burst row)
- `plans/handoffs/HANDOFF_code-analysis-2_findings-sweep-merge_2026-06-11.md` — this file

### rust-survivors
- `crates/game/Cargo.toml` + `Cargo.lock` — pin → v4.6.0 `59c0845` (commit `f18c5b0`, pushed; user's unrelated uncommitted doc changes left untouched)

### Memory
- `engine-current-state.md` (3 updates: sweep → merged → pin bumped), `MEMORY.md` (index line, 2 updates)

## User Feedback & Preferences (REQUIRED — never omit)

- Session opener prescribed a **5-step onboarding narration protocol** (summarize handoff / claim beads / state verification plan / read key files + 2-3 unlisted adjacent files / propose first action) **and an explicit wait-for-go-ahead**. Honor this exact shape if the next session's paste prompt repeats it.
- "내가 답변 해줘야 하는 부분만 한글로 다시 보여줘" — after a long status report, the user wants ONLY the decision points re-presented, in Korean. Keep reports skimmable; isolate asks.
- "1번 진행하는데 내가 정보 알려주면 대신 진핸 가능해?" — willing to hand over credentials/info for delegation; answer honestly about transcript-exposure risks (the PAT warning was given and the user chose device flow instead).
- "원격으로 접속 중이라 직접하기는 힘들고" — user is on a **remote connection**; interactive local steps are friction. Device-flow trick that worked: the browser step can run on the user's LOCAL machine while the CLI runs remote.
- "이번 pr은 다음에 같이 하는걸로 미루고 다른 작업 있으면 알려줘" — when blocked, the user wants alternatives surfaced immediately, not idle waiting.
- Sweep choice: picked the recommended "잔여 findings 정리 스윕" (did NOT multi-select the fable5 retest — it remains undone).
- "2번 진행해줘" / "인증 완료했어 진행해" / "1번 진행" — terse numbered go-aheads referencing MY numbered lists; keep next-step lists numbered so the user can answer in two words.
- Merge: picked "merge commit으로 머지 (추천)" — accepts recommendations when the rationale (bisect-buildable commits) is stated.
- "/handoff 하고 커밋" — same close pattern as parent session (there it was "하고 커밋 푸쉬"; this time only 커밋 stated).
- Standing preferences honored all session: Korean prose to user / English repo artifacts (`conversation-language-korean`, `doc-language-rule`), aggressive Sonnet subagents (`subagent-usage-preference`), `+1.88.0` gates (`ci-toolchain-pin`), explicit `model:` on every agent (`new-model-subagent-incompat`).

## Where We're Going

1. **v5.0.0 breaking batch** — the only remaining analysis work; fully specified across two handoffs. From parent: #2 rapier `BodyHandle`/`ColliderHandle` newtypes (NOTE this session's finding: blast radius includes `set_one_way`/`is_one_way`/`has_contact`/`rigid_body[_mut]`/`get_collider[_mut]`/`set_collision_groups`/`collision_groups`, `RaycastHit.collider_handle` pub field, 6 src files + crane_wrecking_ball & platformer examples); #8 `on_enter` `SystemRegistrar` (now even more valuable — LABELs exist engine-wide but scenes still can't use them; PATTERNS.md explicitly notes this gap); remove `DebugDrawQueue`/`DebugRect` + the 4 new deprecations + `animation::player`/`timeline`/`prefab` shims; `Sprite.texture` → `Arc<str>`/interning. From this session's triage: visibility narrowings (GpuLightData, LightingUniforms, PostProcessRenderer.target_view/normal_view, TouchState fields, input submodule pub), SystemConfig/SystemMeta merge, `ShaderMaterial` cached `source_hash` (+ consider `#[non_exhaustive]` on `DebugShape` and on `NetworkEvent` while breaking). Follow the v3 precedent: branch + PR.
2. **Feature candidates split out of the sweep** (separate, example-driven per VISION loop): `StateMachineSystem` crossfade_duration; scripting Arrive/Wander bindings; AudioEffect release-envelope implementation.
3. **Optional cleanups**: delete merged remote branch `fix/analysis-top10`; archive the `code-analysis-fixes` chain handoffs (that chain closed itself in seq 4); REFERENCE.html was NOT updated for the new 4.6.0 APIs (burst, take/is_pending, quit/is_quitting, LABELs) — decide whether it tracks releases or gets regenerated.
4. **Optional**: re-test fable5-as-subagent after Claude Code updates; if fixed, delete `new-model-subagent-incompat` memory and stop forcing `model:`.
5. **rust-survivors**: no engine follow-up owed; the game tree's uncommitted doc changes are the user's own in-flight work (do not touch/commit them from an engine session).

## Risks & Blockers

- **4 new deprecations compound the fork-warning surface** (now 6 deprecated items total incl. DebugRect pair) — `-D warnings` forks break until they `#[allow]` or migrate; all removals land together in v5.
- **`seen_new_hashes` multi-camera caveat** from parent still stands (one clone per camera per new material on first frame) — unchanged this session, revisit only if material churn becomes real.
- **`exec_order` take/swap** rests on "systems can't reach `App` to set `schedule_dirty` mid-loop" — if a future API hands systems App access (e.g., the v5 `SystemRegistrar` work drifting that way), revisit the restore-after-loop.
- **PATTERNS.md ordering table is now normative** — if v5's SystemRegistrar changes how labels attach, update the table + platformer example together or they'll teach stale API.
- **gh auth is session/keyring-bound** — worked at close; if a future session hits 401 again, the device-flow-on-local-browser trick is the known path for this user's remote setup.

## Open Questions

- `DebugShape` (and `NetworkEvent`?) `#[non_exhaustive]` in the v5 batch — cheap while already breaking; `ReflectValue` precedent. (Carried from parent, still undecided.)
- Does REFERENCE.html get regenerated per release or manually curated? (It now lags 4.6.0's additive API.)
- Should the v5 batch also migrate examples off `q.0 = true` to `ShouldQuit::quit()` (left unmigrated deliberately this session to keep N's diff surgical)?

## Quick Start for Next Session

```bash
# Current state
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3 main          # 59c0845 merge of PR #12; tree clean; everything pushed
gh pr view 12                       # the merged batch PR (gh is authenticated as ChunSam)

# Canonical context
# - docs/CODE_ANALYSIS_2026-06-10.md   (resolution header = full disposition map)
# - docs/CHANGELOG.md                  (4.6.0 = both phases)
# - docs/PATTERNS.md                   (new: labels table, scratch-buffer convention, asset recipe)
# - parent handoff (Top-10 batch + v5 spec) + this file

# Key files if starting the v5 batch
# - src/physics/world.rs + body.rs + world/{body_factory,raycast,joints,character_movement}.rs  (#2 newtypes; blast radius in this handoff)
# - src/scene.rs + src/app/scenes.rs + src/ecs/schedule.rs   (#8 SystemRegistrar; LABELs now exist engine-wide)
# - src/resources.rs (DebugDrawQueue/DebugRect removal), src/material.rs (source_hash), src/components.rs (Sprite.texture)

# Verify (CI pin — memory ci-toolchain-pin)
cargo +1.88.0 fmt --check && cargo +1.88.0 clippy --all-targets -- -D warnings \
  && cargo +1.88.0 build --target wasm32-unknown-unknown \
  && cargo +1.88.0 test --all-targets \
  && RUSTDOCFLAGS="-D warnings" cargo +1.88.0 doc --no-deps
# Expect: 375 passed / 0 failed

# Next action
# v5.0.0 breaking batch (branch + PR, v3 precedent) — see Where We're Going #1 for full scope.
```

## Session Closed
**Closed at:** 2026-06-11
**Commit:** see `session: findings-sweep-merge [code-analysis-2]` on main (handoff file only — all code/doc work was committed and merged via PR #12 during the session)
**Session status:** Handed off to next session
