# v5.0.0 breaking batch shipped: handle newtypes, SystemRegistrar, Arc<str> sprites, all deprecations removed — merged to main via PR #13

**Date:** 2026-06-11
**Status:** COMPLETED — the entire 2026-06-10 code-analysis round (70 verified findings) is now FULLY closed across two releases: v4.6.0 (PR #12, non-breaking) and **v5.0.0 (PR #13, merge commit `c34b6c1`, CI 4/4 green, 12 bisect-buildable commits)**. No analysis debt remains. rust-survivors is still pinned to 4.6.0 and now owes a *real* migration (first non-no-op bump).
**Bead(s):** none (`bd` not installed — `command not found`, same as seq 1/2; tracked via conversation + TaskCreate tool)
**Epic:** code-analysis remediation, round 2 (`docs/CODE_ANALYSIS_2026-06-10.md`)
**Chain:** `code-analysis-2` seq `3`
**Parent:** `HANDOFF_code-analysis-2_findings-sweep-merge_2026-06-11.md`
**Prior chain:** `HANDOFF_code-analysis-2_top10-fix-batch_2026-06-10.md` > `HANDOFF_code-analysis-2_findings-sweep-merge_2026-06-11.md` > this

---

## Stale References

All deliberate removals by this session (v5 batch), not drift — listed so greps against parent handoffs don't confuse the next session:

- `SystemMeta` — merged into `SystemConfig` (commit `d6a3bc3`); zero occurrences remain
- `DebugDrawQueue` / `DebugRect` — removed (`386b5b4`); `DebugDraw::rect_filled_z` is the replacement
- `World::register_reflect` — removed (`4ddf156`); `register_reflect_named` only
- `NetworkEvent::JsonParseError` — removed (`4ddf156`)
- `App::load_texture` — removed (`4ddf156`); `SpriteRenderer::load_texture` (internal, renderer/sprite/textures.rs) still exists and is unrelated
- `ParticleEmitter::for_burst` — removed (`4ddf156`); `burst()` canonical
- Deep paths `animation::player::{UvRect, BlendUv}`, `timeline::Lerp`, `prefab::topological_sort_entities`, the whole `components::*` migration facade — removed (`6d088d3`); root re-exports all survive
- `Sprite.texture: Option<String>` — now `Option<Arc<str>>` (`7903067`)
- `Scene::on_enter(.., &mut Vec<Box<dyn System>>)` — now `&mut SystemRegistrar` (`e4b6c39`)
- rapier `RigidBodyHandle`/`ColliderHandle` in public physics signatures — now engine `BodyHandle`/`ColliderHandle` (`340ba22`)
- `input::map::AxisBinding`, `input::touch::TouchPoint` deep paths — input submodules privatized (`92f06cf`); `engine::AxisBinding` / `engine::input::AxisBinding` work, `TouchPoint` is now `pub(crate)`

## Since Last Handoff

- Parent's "Where We're Going" #1 (**v5.0.0 breaking batch**) — **DONE in full, in one session**, including everything queued from both prior sessions' triage: #2 newtypes (with the wider blast radius seq 2 documented), #8 registrar, all removals, Sprite interning, visibility narrowings, SystemConfig/Meta merge, ShaderMaterial hash. Branch + PR + merge same day (v3 precedent followed exactly).
- Parent's open question "`DebugShape` (and `NetworkEvent`?) `#[non_exhaustive]`" — **answered by user: BOTH** (AskUserQuestion, recommendation accepted). Shipped in `d6a3bc3`.
- Parent's open question "migrate examples off `q.0 = true`?" — **answered: yes** (shipped `52c5f16`, 4 sites).
- Parent's open question "REFERENCE.html per-release or curated?" — **answered: hand-curated, updated in-batch** (confirmed no generation script exists; agent-updated in `92325fc`, backfilling 4.6.0 too).
- Parent's "Where We're Going" #3 (optional cleanups) — partially absorbed: REFERENCE.html done; **remote branch deletion + worktree-agent branch pruning still open** (now two merged branches to delete).
- Parent's #4 (fable5 subagent retest) — NOT done again; all 7 agents this session ran explicit `model: sonnet`, zero failures. Memory `new-model-subagent-incompat` still accurate.
- Parent's #5 (rust-survivors owes nothing) — **changed**: v5 is breaking, so the game now owes a real migration (Scene impl + physics handles + Sprite). Deliberately NOT done this session.
- Parent's risk "`exec_order` take/swap rests on systems not reaching App" — **checked during #8 design**: SystemRegistrar wraps the two Vecs, does NOT hand systems App access; the safety argument still holds.
- Parent's risk "PATTERNS.md ordering table is normative — update with SystemRegistrar or it teaches stale API" — **handled**: the "planned for v5" gap note was replaced with the shipped registrar pattern + settings_menu example reference, same session as the API change.
- New since parent: the unpushed seq-2 handoff commit (`3b9a664`, local main ahead 1) was pushed at session start before branching.

## Reference Documents

- `docs/CHANGELOG.md` — **`## 5.0.0` entry is the canonical migration guide** (per-item migrations; written this session).
- `docs/CODE_ANALYSIS_2026-06-10.md` — analysis report; its resolution header still describes v5 as "planned" (NOT updated this session — minor follow-up, see Where We're Going).
- `docs/PATTERNS.md` — "System ordering with labels" now includes the SystemRegistrar scene pattern; render-layer row references DebugDraw (not the removed queue).
- `CLAUDE.md` — doc v1.6.0, package 5.0.0; module map rows updated (SystemRegistrar, handle newtypes, non_exhaustive DebugShape, shim mentions dropped).
- `REFERENCE.html` — caught up to 4.6.0 + 5.0.0 (agent-updated, spot-checked; header says v5.0.0).
- Parent handoffs (seq 1: Top-10 batch + original v5 spec; seq 2: sweep + merge + widened #2 blast radius).
- Memory: `engine-current-state` (rewritten this session for v5), `new-model-subagent-incompat`, `ci-toolchain-pin`, `rust-survivors-engine-pin`, `subagent-usage-preference`, `conversation-language-korean`.

## The Goal

Ship the v5.0.0 breaking batch — the only remaining work from the 2026-06-10 full-codebase analysis — as a branch + PR with per-item bisect-buildable commits (v3/v4.6 precedent). Scope was fully specified across the two parent handoffs: analysis Top-10 #2 (rapier handle newtypes) and #8 (`on_enter` SystemRegistrar), removal of everything deprecated in 4.6.0 plus all path shims, `Sprite.texture` interning, and the sweep-triaged breaking items (visibility narrowings, SystemConfig/SystemMeta merge, ShaderMaterial source-hash caching). Three open questions rode along and were resolved by the user mid-session. End state: v5.0.0 merged to main, engine analysis-debt-free, with a written migration guide for the one real consumer (rust-survivors).

## Where We Are

- **`main` = `c34b6c1`** (merge commit of PR #13, pushed; 65 files, +763/−539). Branch `feat/v5-breaking-api` fully merged: 12 commits, every one verified bisect-buildable. Working tree clean. Remote branch NOT deleted (cleanup candidate, alongside `fix/analysis-top10` from seq 2).
- **PR #13** (https://github.com/ChunSam/skeleton-engine/pull/13) — body has phase tables, verification matrix, consumer-impact note. CI 4/4: Test (native) 7m0s, Build (WASM) 54s, Package dry-run 4m23s, Rustdoc 59s (run 27317648892).
- **Package version 5.0.0** (`Cargo.toml:3`), CLAUDE.md doc v1.6.0.
- **Final gates** (all `cargo +1.88.0`): fmt ✓, clippy `--all-targets -D warnings` ✓, wasm32 build ✓, test --all-targets **375/0** (unchanged from 4.6.0 — breaking batch added no behavior), doc `-D warnings` ✓, `wasm_smoke.sh` PASS (41,974-byte screenshot, eyeballed — HUD text, white player square, 6 coins, footer all correct, byte-identical size to the seq-2 frame; **meaningful because coin_race now runs through SystemRegistrar + Arc<str> paths**).
- **#2 newtypes shipped**: `BodyHandle(pub(crate) RapierBodyHandle)` / `ColliderHandle(pub(crate) RapierColliderHandle)` next to `JointHandle` in `src/physics/world.rs`, both with `pub fn raw()` escape hatch + `pub(crate) from_raw()`. Engine `ColliderHandle` deliberately shadows rapier's glob import (local defs beat globs); rapier's types aliased `Rapier*Handle` where needed. All 8 physics src files converted (world, body, system, body_factory, raycast, character_movement, joints, tile_collider); `RaycastHit.collider_handle` is the newtype; `rigid_body[_mut]`/`get_collider[_mut]` still return raw rapier refs (documented escape hatches). `one_way_colliders` stays raw internally.
- **#8 registrar shipped**: `SystemRegistrar<'a>` in `src/scene.rs` wrapping `(&mut Vec<Box<dyn System>>, &mut Vec<SystemConfig>)`, `pub(crate)` constructor (only `App::apply_scene_cmd` can build one), `add()` / `add_labeled()` push both vecs in lockstep — kills the `reconcile_meta()` default-backfill gap the onboarding read discovered. `reconcile_meta` kept as safety no-op + for the `App::add_system` path. `SceneCmd::Pop` `owned`-count bookkeeping unchanged. 8 example files + 2 test-internal Scene impls (`SceneA`/`SceneB` in app.rs) migrated.
- **Sprite interning shipped**: `Sprite.texture: Option<Arc<str>>`; constructors take `impl Into<Arc<str>>` (call sites with literals compile unchanged); serde `rc` feature added (`Cargo.toml:108`); RON wire format identical — 13 prefab tests including round-trips and back-compat encrypted loads prove it. Renderer batch keys: `SpriteRenderKind::Sprite{texture_key: Arc<str>}` / `::Material{texture_key: Option<Arc<str>>}` in `renderer/sprite/sort.rs`; the one real conversion is `Arc::from(h.path())` per handle-derived key at batch-key build; `sprite.texture.clone()` at sprite.rs:341/:499 are now refcount bumps.
- **SystemMeta gone** — `compute_order(&[SystemConfig])`; App field `system_meta: Vec<SystemConfig>` (name kept); schedule tests rewritten with SystemConfig literals (same-module pub(crate) fields).
- **ShaderMaterial**: `new(frag_source, params)` hashes once (DefaultHasher, same fn the renderer used); `frag_source` private behind `frag_source()`/`set_frag_source()` (re-hashes); `params` stays pub; renderer's per-frame 3-line DefaultHasher block in the `mat_ids` collection pass (sprite.rs ~471-474) replaced by `mat.source_hash()`; `seen_new_hashes` one-clone-per-new-hash logic untouched.
- **non_exhaustive**: `DebugShape` + `NetworkEvent`, doc notes added; in-crate matches unaffected; all 5 NetworkEvent-matching examples already had `_ =>` arms (pre-verified by grep, confirmed by gate).
- **Visibility narrowings**: `GpuLightData`/`LightingUniforms` + fields → pub(crate); `PostProcessRenderer.{target_view,width,height}` → pub(crate) (`PostProcessConfig` untouched — real API); `TouchState` 5 event fields private behind `began()/moved()/ended()/pinch_delta()/swipe()` accessors, `TouchPoint` → pub(crate) (no public method returns it); input submodules `mod` (private) with curated re-exports — `pub use map::{AxisBinding, InputMap}` added, lib.rs:83 repointed.
- **Removals all clean**: zero lingering callers verified by grep before each commit; mp_client's `#[allow(deprecated)]` + JsonParseError arm dropped; render step-2.5 drain + core_resources registration deleted with the queue.
- **Examples teach `ShouldQuit::quit()`** — basic.rs:34, platformer:161, settings_menu:499, scene_flow:321.
- **Docs shipped**: CHANGELOG 5.0.0 (3 sections: removed / API changes / changed, each with migration), PATTERNS.md (3 edits), CLAUDE.md (6 row updates + version header via anchored python), REFERENCE.html (agent: 13 fixes + 10 additions, zero stale names by grep, header v5.0.0).
- **Memory**: `engine-current-state` fully rewritten (v5 merged, rust-survivors migration owed, next candidates); MEMORY.md index line updated.
- **Task tracking**: used the session Task tools (8 tasks, all completed) instead of bd.

## What We Tried (Chronological)

1. **Onboarding per the user's 5-step protocol** (read handoff → beads → verification plan → key files + 2-3 unlisted adjacent → first action, then WAIT). `bd` not installed (exit 127). Found during baseline check: local main was **ahead 1** (seq-2 handoff commit `3b9a664` committed but never pushed — user's close instruction was "커밋" only) and two stale `worktree-agent-*` local branches exist. Adjacent reading (`src/lib.rs`, `src/input/touch.rs`, raw-handle grep) produced four discoveries the handoffs lacked: (a) #2 blast radius is **8** src files, not 6 (add `system.rs`, `tile_collider.rs`); platformer never *names* rapier types — uses factory-tuple inference + PhysicsBody literals (`platformer.rs:589,615`), predicting zero-edit migration (confirmed later); (b) `apply_scene_cmd`'s `reconcile_meta()` backfills `SystemMeta::default()` for scene-pushed systems (`app/scenes.rs:56-65`) — the *mechanism* of the #8 gap, so the registrar must push configs in lockstep and preserve `owned` counts; (c) `Sprite` derives Serialize/Deserialize + `Reflect::set_field("texture", String)` (`components.rs:195`) → serde `rc` feature constraint; (d) `SystemConfig`/`SystemMeta` are field-for-field identical → merge is mechanical.
2. **Go-ahead ("1") → setup**: pushed the stranded handoff commit; created 8 tasks; ran the 5-gate baseline in background → **GATE EXIT 0, 375/0**, bit-identical to seq-2 close.
3. **Open questions resolved while baseline ran** (AskUserQuestion, all three recommendations accepted): `#[non_exhaustive]` → **both** enums; example `q.0` migration → **yes**; REFERENCE.html → **update in-batch** (checked first: no generation script exists; hand-curated since Phase 39d per HANDOFF.md history).
4. **Plan presented as 4 phases / 14 commits**, user approved Phase 1 start ("페이즈1 시작해"). Commit convention verified against v3 batch: `feat(scope)!:`.
5. **Phase 1 (main session, sequential, 3 commits)**. Gotcha #1: removing the deprecated `NetworkEvent` variant made rustfmt fold the remaining struct variants to single-line → fmt fallout for commit 2's files was **amended into commit 2** so every commit stays fmt-clean for bisect. Gotcha #2: the shims in `player.rs` and `timeline.rs` doubled as those files' own imports — removal broke `UvRect`/`Lerp` resolution inside the very files; fixed with plain `use` lines. `prefab::topological_sort_entities` removal required repointing lib.rs's root re-export source from prefab → hierarchy group (root path `engine::topological_sort_entities` survives) + editor call site + prefab test. Per-commit gate: clippy --all-targets + test --all-targets each, full 5-gate at phase end (background) — all green, 375/0.
6. **Phase 2 (3 parallel Sonnet agents: S/V/M, disjoint whitelists)** after scouting greps proved: all 5 NetworkEvent-matching examples already had wildcards; TouchState's only example consumer is touch_demo.rs; ShaderMaterial has zero example users; lib.rs needs exactly 2 single-line edits (S: line 75 SystemMeta, V: line 83 AxisBinding). All three returned DONE. S found one unlisted SystemMeta user (an app.rs test building literals). M removed the now-dead `use std::hash::{Hash, Hasher}` import. Cross-agent noise handled per playbook (S/M saw V's joystick/touch mid-edit errors, correctly ignored).
7. **lib.rs bisect-split trick (new technique, used twice)**: two agents legitimately edit different single lines of lib.rs, but `git add` is per-file. To keep commits bisect-buildable: temporarily revert agent-2's line to its pre-change form, stage lib.rs with agent-1's commit (snapshot pairs old line with old module state → compiles), restore the line, stage with agent-2's commit. Verified by compiling the intermediate snapshot in a temp `git worktree` sharing the main target dir (`cargo check --lib --target-dir <main>/target` — deps cached, seconds not minutes).
8. **Phase 3 (3 parallel Sonnet agents: P3-A/B/C)** after scouting (Sprite literal constructors in 3 examples; serde features line; `3cddc1e` commit message for clone sites). All DONE. P3-A: `Rapier*Handle` alias pattern for the deliberate name shadow; platformer needed **zero edits** (inference prediction confirmed); crane dropped its rapier handle imports but keeps `rapier2d::na`/`vector` for the `rigid_body_mut` escape-hatch math. P3-B: registrar with `pub(crate)` constructor; settings_menu got the labeled demo (`UiSystem` after `LayoutSystem::LABEL` — a real constraint, not invented); 2 test-internal Scene impls migrated. P3-C: `SpriteRenderKind` key type swap; 348 lib tests green including all 13 prefab RON tests.
9. **Mid-flight false alarm + one real catch**: rust-analyzer diagnostics showed platformer E0308 handle mismatches — **stale** (captured mid-edit; full clippy clean). The **real** failure was the rustdoc gate: P3-B's `SystemRegistrar` doc used unresolvable intra-doc links (`[add]`, `[add_labeled]`, bare `[App::add_system_labeled]`) → `-D warnings` doc gate caught it; fixed by qualifying (`SystemRegistrar::add`, `crate::App::...`). Exactly the regression class the 5th gate exists for.
10. **Phase 3 commits with the lib.rs split trick again** (scene line this time), then **both intermediate snapshots verified standalone** (`cargo check --all-targets` in worktrees, background): `340ba22` ✓ `e4b6c39` ✓.
11. **Phase 4**: `q.0 = true` → `quit()` via python (assert count==1 per file) + clippy --examples → `52c5f16`. Version bump 5.0.0; CHANGELOG entry written with per-item migrations; PATTERNS.md gap-note → shipped-pattern replacement; CLAUDE.md row updates via anchored python (assert count==1 each). REFERENCE.html delegated to a background Sonnet agent in parallel with the final background 5-gate (HTML isn't compiled — no conflict). Agent fixed 13 stale spots (including a previously unknown stale `on_enter(app: &mut App)` example) and added 10 sections; spot-checked (v5.0.0 header, 4× SystemRegistrar, zero stale names, 81+/23− diff).
12. **wasm_smoke PASS** (41,974 B, eyeballed) — first smoke through the new registrar + Arc paths since coin_race was migrated.
13. **Release**: docs commit `12deda3` (REFERENCE excluded, agent still writing) → `92325fc` (REFERENCE) → push → **PR #13** → `gh pr checks --watch` background → CI 4/4 → AskUserQuestion (merge commit 추천 / squash / hold) → user picked merge commit → **merged, main `c34b6c1`**, local main ff-synced. Memory rewritten + index updated.

### Phase-1 per-commit edit specifics (for git-archaeology without re-reading diffs)

- **`386b5b4`** (4 files, +1/−63): resources.rs lines 25-52 (section header + both deprecated types) deleted, line 167 doc reworded "deprecated" → "pre-v5"; lib.rs 3-line export block deleted; app/render.rs step-2.5 block (26 lines incl. the `DrawRect` conversion loop) deleted, step 2.6 untouched; core_resources.rs 3-line registration deleted.
- **`4ddf156`** (6 files, +5/−60): ecs/world.rs `register_reflect` (28 lines incl. doc) deleted and `register_reflect_named`'s doc absorbed the registry description; reflect.rs trait doc repointed to `register_reflect_named::<T>("Name")`; network.rs variant + deprecation attr deleted (rustfmt then folded `Disconnected`/`MessageTooLarge`/`ReceiveQueueFull` to single-line — amended in); mp_client.rs `#[allow(deprecated)]` + comment + 3-line match arm deleted (existing `_ => {}` covers); app/assets.rs 7-line method deleted; particle.rs 8-line method deleted.
- **`6d088d3`** (9 files, +15/−41): player.rs shim block deleted + plain `use crate::renderer::uv::UvRect` added (its own code used the shim at lines 28/100/105!); animation/mod.rs re-export trimmed to `{AnimationClip, AnimationPlayer, BlendWeight}`; system.rs/blend_system.rs imports split between player and renderer::uv; timeline.rs shim deleted + `use crate::tween::{Easing, Lerp}`; prefab.rs shim + doc deleted, test call qualified; editor ui/mod.rs:692 `crate::prefab::` → `crate::hierarchy::`; lib.rs topo moved from prefab group to hierarchy group; components.rs facade block deleted.

### AskUserQuestion rounds (exact options, what was picked)

1. **non_exhaustive**: "Both (Recommended)" ✓ / DebugShape only / Neither — picked Both.
2. **Examples q.0**: "Migrate (Recommended)" ✓ / Leave as-is — picked Migrate.
3. **REFERENCE.html**: "Update in v5 batch (Recommended)" ✓ / Separate session / Leave it — picked in-batch.
4. **Merge** (post-CI): "Merge commit으로 머지 (추천)" ✓ / Squash / 보류 — picked merge commit. (Identical outcome to PR #8 and PR #12 — three-for-three on this question; consider defaulting with confirmation next time.)

## Key Decisions

- **Phase ordering: removals first** — clearing the deprecated surface before the big items means no later work maintains dead paths; small mechanical merges next (S/V/M); big entangled items (#2/#8/interning) after; polish + docs last. Worked exactly as planned, 12 commits, zero rework.
- **Keep `SystemConfig`, delete `SystemMeta`** (not the reverse): SystemConfig is the user-facing builder; its pub(crate) fields are the narrower (v5-appropriate) surface. `compute_order` signature changed rather than keeping a dead alias.
- **`SystemRegistrar` lives in `scene.rs` with a `pub(crate)` constructor** — only `apply_scene_cmd` can create one, so the lockstep invariant (systems.len() == configs.len()) can't be violated from outside. `reconcile_meta` kept as a safety net rather than removed.
- **Engine `ColliderHandle` deliberately shadows rapier's name** — local definitions beat glob imports in Rust; inside physics files rapier's types are aliased `RapierColliderHandle`/`RapierBodyHandle`. Rejected alternative (different name like `EngineColliderHandle`) — worse API for users, who never see the shadow.
- **`.raw()` escape hatch on both newtypes + raw rapier refs from `rigid_body[_mut]`/`get_collider[_mut]`** — per parent spec: forks must be able to drop to rapier; the newtype prevents *accidental* leakage, not deliberate use.
- **`params` stays pub on ShaderMaterial** — it's per-frame animation input and not part of the hash; only `frag_source` can desync the cache, so only it went private.
- **`Arc<str>` over interned-ID table for Sprite.texture** — serde stays derive-only (`rc` feature), RON format identical, no global interner state; `impl Into<Arc<str>>` keeps every constructor call site compiling. Rejected: u32-interning (needs a registry resource + serde custom impls; more invasive for the same per-frame win).
- **lib.rs hunk-splitting via temporary line revert** (not `git add -p`, which is interactive-only here; not "one agent owns lib.rs", which would serialize the waves) — and **verify the intermediate snapshot compiles in a worktree** rather than trusting the reasoning. Used twice, caught nothing, cost seconds (shared target dir).
- **Examples-already-have-wildcards check BEFORE non_exhaustive** — scouted instead of letting agents discover; turned a potential 5-file example edit into zero.
- **REFERENCE.html update in-batch via background agent in parallel with the final gate** — HTML isn't compiled, so the two background tasks can't conflict; agent quality controlled by grep-zero-stale-names + spot checks rather than full read.
- **Merge commit, not squash** (user choice, my recommendation, third time running): 12 engineered bisect-buildable commits; squash destroys exactly what the discipline bought.
- **rust-survivors migration deliberately NOT started** — breaking bump deserves its own session with the user; PR body + CHANGELOG carry the guide.

## Evidence & Data

### v5 commit log (`feat/v5-breaking-api`, oldest first — all on main via `c34b6c1`)

| Hash | Subject | Phase |
|---|---|---|
| `386b5b4` | feat(resources)!: remove deprecated DebugDrawQueue/DebugRect | 1 |
| `4ddf156` | feat(api)!: remove the four 4.6.0-deprecated APIs | 1 |
| `6d088d3` | feat(api)!: remove pre-v5 re-export shims and the components.rs facade | 1 |
| `d6a3bc3` | feat(ecs)!: merge SystemMeta into SystemConfig; non_exhaustive enums | 2 |
| `92f06cf` | feat(api)!: narrow internal visibility (lighting, post-process, input) | 2 |
| `a40e847` | feat(material)!: cache ShaderMaterial source hash at construction | 2 |
| `340ba22` | feat(physics)!: BodyHandle/ColliderHandle newtypes — rapier no longer leaks | 3 |
| `e4b6c39` | feat(scene)!: SystemRegistrar replaces raw Vec in Scene::on_enter | 3 |
| `7903067` | feat(components)!: Sprite.texture is Arc<str> — per-sprite key clones are pointer bumps | 3 |
| `52c5f16` | docs(examples): migrate ShouldQuit field writes to the quit() accessor | 4 |
| `12deda3` | docs(v5.0.0): version bump, changelog with per-item migration guide | 4 |
| `92325fc` | docs(reference): catch REFERENCE.html up to 4.6.0 + 5.0.0 APIs | 4 |

Then merge commit `c34b6c1` (PR #13 → main, 65 files, +763/−539).

### Agent usage (all `model: sonnet`, explicit per fable5 incompat — 7 agents, zero failures)

| Agent | Scope | Tokens | Tool uses | Duration | Outcome |
|---|---|---|---|---|---|
| S | SystemMeta merge + non_exhaustive | 45,962 | 37 | 208s | DONE (1 unlisted user found: app.rs test) |
| V | visibility narrowings ×4 | 67,997 | 33 | 205s | DONE (TouchPoint → pub(crate)) |
| M | ShaderMaterial source_hash | 31,004 | 19 | 116s | DONE |
| P3-A | physics handle newtypes | 80,440 | 66 | 439s | DONE (platformer zero edits) |
| P3-B | SystemRegistrar + 8 examples | 80,277 | 82 | 377s | DONE (1 doc-link bug, caught by doc gate) |
| P3-C | Sprite Arc<str> interning | 82,533 | 64 | 403s | DONE (13 prefab RON tests green) |
| REF | REFERENCE.html catch-up | 84,742 | 69 | 370s | DONE (13 fixes + 10 additions, background) |
| **Σ** | | **472,955** | 370 | 3 waves | 7/7 DONE |

### Verification matrix (`cargo +1.88.0`)

| Gate | Baseline (main pre-branch) | Phase 1 | Phase 2 | Phase 3 | Final |
|---|---|---|---|---|---|
| `fmt --check` | OK | OK | OK | OK | OK |
| `clippy --all-targets -- -D warnings` | clean | clean | clean | clean | clean |
| `build --target wasm32-unknown-unknown` | OK | OK | OK | OK | OK |
| `test --all-targets` | 375/0 | 375/0 | 375/0 | 375/0 | **375/0** |
| `RUSTDOCFLAGS="-D warnings" doc` | OK | OK | OK | FAIL→fixed→OK | OK |
| `./scripts/wasm_smoke.sh` | — | — | — | — | PASS (41,974 B, eyeballed) |

Bisect-buildability: intermediate snapshots `d6a3bc3`, `340ba22`, `e4b6c39` each compiled standalone in temp worktrees (shared target dir). The doc-gate FAIL was P3-B's intra-doc links in scene.rs — the only gate failure of the session.

### GitHub CI (PR #13, run 27317648892)

| Job | Result | Duration |
|---|---|---|
| Test (native) | pass | 7m0s |
| Build (WASM) | pass | 54s |
| Package dry-run | pass | 4m23s |
| Rustdoc | pass | 59s |

### Open questions resolved (AskUserQuestion, all recommendations accepted)

| Question | Answer | Shipped in |
|---|---|---|
| `#[non_exhaustive]` on DebugShape / NetworkEvent? | **Both** | `d6a3bc3` |
| Migrate examples off `q.0 = true`? | **Migrate** | `52c5f16` |
| REFERENCE.html tracking policy? | **Update in v5 batch** (hand-curated confirmed) | `92325fc` |
| Merge method for PR #13? | **Merge commit** (3rd time running) | `c34b6c1` |

### #2 blast radius (final, verified)

| Surface | Conversion |
|---|---|
| 8 src files (world, body, system, body_factory, raycast, character_movement, joints, tile_collider) | newtypes throughout |
| `RaycastHit.collider_handle` pub field | engine `ColliderHandle` |
| `examples/crane_wrecking_ball.rs` | dropped rapier handle imports; keeps `rapier2d::na`/`vector` for escape-hatch math |
| `examples/games/platformer/platformer.rs` | **zero edits** (inference through factory tuples + PhysicsBody literals) |
| `rigid_body[_mut]`, `get_collider[_mut]` returns | raw rapier refs KEPT (documented escape hatch) |
| internal `one_way_colliders`, system.rs scratch | raw rapier types kept, unwrap at boundary |

### Pre-dispatch scouting grep results (the facts each wave was built on)

| Scout | Result | Consequence |
|---|---|---|
| NetworkEvent matchers in examples | 5 files: mp_client, orbital_dodger, predict_shooter, coin_race, salvage_run — **ALL already have `_ =>` arms** | non_exhaustive needed zero example edits |
| TouchState field readers | src: only `pub(crate) on_touch_*` writers (joystick.rs tests, app/window.rs); examples: **touch_demo.rs only** (`.swipe` :36, `.pinch_delta` :72, `.began/.ended.clone()` :103-104) | V's example scope = 1 file |
| ShaderMaterial users | src/lib.rs, src/material.rs, src/renderer/sprite.rs — **zero examples** | M's scope = 2 files; doc example was the only literal-construction site |
| lighting/post_process field users outside own file | only `app/render.rs:352` (`&pr.target_view`) | pub(crate) suffices, no caller edits |
| deep `input::` paths | only `lib.rs:83` (`pub use input::map::AxisBinding`) | one-line lib.rs repoint |
| Sprite literal constructions (`texture: Some/None`) | src: prefab.rs, components.rs, particle.rs, renderer/ui.rs; examples: security_camera, split_screen, minimap | P3-C example scope = 3 files |
| `q.0 = true` quit sites | basic.rs:34, platformer:161 (`quit.0`), settings_menu:499, scene_flow:321 (`should_quit.0`) | 4-site python patch |
| serde features | `Cargo.toml:108` `features = ["derive"]` | add `"rc"` for Arc<str> Deserialize |
| `impl Scene for` sites | 8 example files + src/app.rs (tests) + src/scene.rs (docs) | P3-B's migration list |
| raw rapier handle type names | 8 src files + crane_wrecking_ball only; platformer uses pure inference | A's whitelist; platformer zero-edit prediction |

### P3-A per-file conversion map (verbatim from agent return)

| File | Change |
|---|---|
| `src/physics/world.rs` | newtypes + `raw()`/`from_raw()`; all method params/fields updated; `one_way_colliders` stays raw rapier internally |
| `src/physics/body.rs` | `PhysicsBody.rigid_body_handle: BodyHandle`, `.collider_handle: ColliderHandle` |
| `src/physics/world/body_factory.rs` | all `add_*` return `(BodyHandle, ColliderHandle)`; `remove_body` unwraps `.0` |
| `src/physics/world/raycast.rs` | `cast_ray` → `Option<(ColliderHandle, f32)>`; `cast_ray_with_normal` wraps into RaycastHit |
| `src/physics/world/character_movement.rs` | handle params newtyped; predicate closure keeps `rapier2d::prelude::ColliderHandle` (rapier API requires it) |
| `src/physics/world/joints.rs` | `add_*_joint(body1/body2: BodyHandle, ...)` |
| `src/physics/world/tile_collider.rs` | returns `Vec<(BodyHandle, ColliderHandle)>`; dead `rapier2d::prelude::*` import removed |
| `src/physics/system.rs` | scratch fields stay raw (private); `body_pairs: Vec<(Entity, BodyHandle)>`; `.0` unwrap at boundary |
| `src/physics/world/tests.rs` | `solids[0].0.0` to reach the raw handle for `rigid_body_set.get()` |
| `src/physics/mod.rs` + `src/lib.rs` | re-exports for both newtypes |
| `examples/crane_wrecking_ball.rs` | `use rapier2d::prelude::{vector, RigidBodyHandle}` dropped; `Dynamic.handle`/`Block.handle`/`CraneSystem.cart`/`spawn_visual` → newtypes; keeps `rapier2d::na` + `vector` for `rigid_body_mut` escape-hatch math |

### REFERENCE.html agent change list (verbatim, for future audit)

Fixes (A): version header v4.3.0→v5.0.0; quick-start `ShouldQuit.0`→`.quit()` + `load_texture`→`load_image`; App API table row; Scene `on_enter` signature + `systems.push`→`systems.add`; **FadeTransition example had a stale `on_enter(app: &mut App)` signature predating even the Vec form** (unknown until this audit); built-in resources table (DebugDrawQueue row → DebugDraw); physics intro paragraph + query example (`rapier2d::prelude::RigidBodyHandle` → `BodyHandle`) + accessor example import style; TouchState field access → accessors; register_reflect note; `Sprite.texture` literal `"minimap".to_string()` → `.into()` with `Option<Arc<str>>` annotation; NetworkEvent match arm cleanup + non_exhaustive note.

Additions (B): SystemRegistrar table (add/add_labeled); SceneChange is_pending/take; BodyHandle/ColliderHandle intro; DebugDraw rect_filled/rect_filled_z rows; ParticleEmitter::burst + ParticleBurst subsection; write_ron/read_ron subsection; NetworkConfig re-export + is_connected; NetworkEvent non_exhaustive note; built-in LABEL constants list; ShouldQuit quit/is_quitting.

Zero-stale-grep confirmed for: DebugDrawQueue, DebugRect, bare register_reflect, load_texture, for_burst, JsonParseError arm, rapier2d::prelude::RigidBodyHandle, SystemMeta, `Vec<Box<dyn System>>` param, `systems.push(`, `on_enter(app:`, direct touch field access.

### Background task choreography (what ran concurrently when)

| Time slot | Foreground | Background |
|---|---|---|
| Onboarding → plan | file reads, scouting | baseline 5-gate (`bujm3xqwc`) |
| Baseline gate running | AskUserQuestion (3 open questions) | — |
| Phase 1 end | commits | full 5-gate (`b6o1bjkvq`) |
| Phase 3 commits done | q.0 python patch | 2× worktree bisect checks (`b6fxtnvz2`) |
| Phase 4 docs | CHANGELOG/PATTERNS/CLAUDE.md writes | REFERENCE.html agent + final 5-gate (`b8yzgdwwo`) in parallel |
| Gate green | — | wasm_smoke (`bf6y9mt2x`) |
| PR created | — | `gh pr checks 13 --watch` (`b2pj2cjup`) |

### The one gate failure (verbatim, for the pattern library)

```
error: unresolved link to `add`
 --> src/scene.rs:6:25
6 | /// Systems added via [`add`] receive a default (no-constraint) [`SystemConfig`];
error: unresolved link to `add_labeled`   --> src/scene.rs:7:25
error: unresolved link to `App::add_system_labeled`  --> src/scene.rs:9:7
```
Fix: `[`SystemRegistrar::add`]` / `[`App::add_system_labeled`](crate::App::add_system_labeled)`. Bare method names in struct-level docs don't resolve; `App` isn't in scene.rs scope. P3-B's own clippy self-verify could not catch this — only the `RUSTDOCFLAGS="-D warnings" doc` gate compiles docs.

### Plan-as-approved vs as-shipped

| Plan | Shipped | Delta |
|---|---|---|
| Phase 1: 3 removal commits, main session | `386b5b4`/`4ddf156`/`6d088d3` | fmt fallout amended into commit 2; shim-as-own-import gotcha ×2 |
| Phase 2: agents S/V/M, 5 commits estimated | 3 commits (S items merged into one) | non_exhaustive folded into S's commit instead of separate |
| Phase 3: sequential or 2 agents, 3 commits | 3 parallel agents, 3 commits | parallelized fully (example sets proved disjoint) |
| Phase 4: 3 commits | 3 commits (`52c5f16`/`12deda3`/`92325fc`) | REFERENCE split from docs commit (agent timing) |
| Gates per phase boundary | held at every boundary | +2 unplanned worktree bisect verifications |

### Session command patterns that worked (reusable)

- **lib.rs split for two-agent single-line edits**: temp-revert agent-2's line → stage lib.rs with agent-1's files → commit → restore → stage with agent-2's. Verify the intermediate snapshot: `git worktree add /tmp/chk <hash> && cd /tmp/chk && cargo +1.88.0 check --all-targets --target-dir <main>/target` (seconds, deps cached).
- **Anchored python bulk edits with `assert count==1`** (q.0 migration, CLAUDE.md rows, Cargo.toml bump) — the assert catches drift/duplicates before writing.
- **Background pairing**: final 5-gate + REFERENCE.html agent ran concurrently (HTML not compiled → no conflict); `gh pr checks 13 --watch --interval 20` as background task.
- **Test-count summing**: `cargo test --all-targets 2>&1 | grep -E "^test result" | awk -F'[ ;]+' '{p+=$4; f+=$6} END {print p, f}'`.
- **Per-agent prompt rules carried from seq 2** (file whitelist / no repo-wide fmt — this session: no rustfmt at all, main session formats at gate / no git commit / ignore foreign-file errors / `cargo +1.88.0`) — 7/7 agents complied, zero corruption.

## Verbatim API shapes shipped (copy-paste reference)

The two newtypes (`src/physics/world.rs`, next to JointHandle):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyHandle(pub(crate) RapierBodyHandle);
impl BodyHandle {
    pub fn raw(self) -> RapierBodyHandle { self.0 }
    pub(crate) fn from_raw(h: RapierBodyHandle) -> Self { Self(h) }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColliderHandle(pub(crate) RapierColliderHandle);
// same raw()/from_raw() shape
```

The registrar (`src/scene.rs`):

```rust
pub struct SystemRegistrar<'a> {
    systems: &'a mut Vec<Box<dyn System>>,
    configs: &'a mut Vec<SystemConfig>,
}
impl<'a> SystemRegistrar<'a> {
    pub(crate) fn new(systems: .., configs: ..) -> Self      // App-only construction
    pub fn add(&mut self, system: impl System + 'static)      // SystemConfig::default()
    pub fn add_labeled(&mut self, system: impl System + 'static, config: SystemConfig)
}
```

The settings_menu labeled demo (the in-repo acceptance for #8):

```rust
fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
    systems.add(LayoutSystem);
    // UiSystem reads the layout geometry computed by LayoutSystem each frame — must run after it.
    systems.add_labeled(UiSystem, SystemConfig::new().after(LayoutSystem::LABEL));
}
```

Final `input/mod.rs` surface:

```rust
mod gamepad; mod map; mod state; mod touch;
pub use gamepad::{GamepadAxis, GamepadButton, GamepadState};
pub use map::{AxisBinding, InputMap};
pub use state::InputState;
pub use touch::TouchState;
```

ShaderMaterial (src/material.rs): `new(frag_source: impl Into<String>, params: [f32; 4])`, private `frag_source`/`source_hash`, getters `frag_source() -> &str` / `source_hash() -> u64`, `set_frag_source(impl Into<String>)` re-hashes; `params: [f32; 4]` pub.

## Code Analysis

- `BodyHandle`/`ColliderHandle`: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]`, tuple field `pub(crate)`, `pub fn raw(self) -> Rapier*Handle`, `pub(crate) fn from_raw`. Defined in `src/physics/world.rs` next to `JointHandle`. The engine name shadows rapier's glob-imported name inside physics files (legal: local items beat glob imports); alias `use rapier2d::prelude::ColliderHandle as RapierColliderHandle` where the raw type is needed (e.g. `move_character`'s predicate closure must take rapier's type — rapier API requirement).
- `SystemRegistrar<'a> { systems: &'a mut Vec<Box<dyn System>>, configs: &'a mut Vec<SystemConfig> }`; `new()` is `pub(crate)`; `add` pushes `SystemConfig::default()`, `add_labeled` pushes the given config. Constructed in both `Replace` and `Push` arms of `apply_scene_cmd`. The `exec_order` take/swap safety argument from seq 2 still holds — registrar hands scenes no App access.
- `Sprite.texture: Option<Arc<str>>`; `Reflect::fields` does `self.texture.as_deref().unwrap_or_default().to_string()` (clone only when Inspector reads); `set_field` maps empty string → None, else `Some(Arc::from(s))`. Batch keys in `renderer/sprite/sort.rs` `SpriteRenderKind`; image-handle arm converts via `Arc::from(h.path())` once per key build.
- `ShaderMaterial { frag_source: String (private), source_hash: u64 (private), params: [f32;4] (pub) }`; `hash_source` private helper (DefaultHasher — in-memory cache, run-stability irrelevant per analysis disposition); `set_frag_source` re-hashes atomically.
- `compute_order(metas: &[SystemConfig])` — same Kahn's algorithm, tie-break by insertion index; schedule tests construct SystemConfig literals (same-module pub(crate) access).
- `TouchState` accessors: `began()/ended() -> &[(u64, Vec2)]`, `moved() -> &[(u64, Vec2, Vec2)]`, `pinch_delta() -> f32`, `swipe() -> Option<Vec2>`; writers unchanged (`pub(crate) on_touch_*`). `input/mod.rs`: `mod gamepad; mod map; mod state; mod touch;` + `pub use gamepad::{GamepadAxis, GamepadButton, GamepadState}; pub use map::{AxisBinding, InputMap}; pub use state::InputState; pub use touch::TouchState;`.
- serde `rc` feature (`Cargo.toml:108`): enables `Deserialize` for `Arc<str>`; serialization format identical to `String` (RON/JSON) — backward/forward compatible wire format, proven by `prefab::tests` round-trips + `*_backcompat_encrypted_load`.
- Intra-doc-link rule worth remembering: links in struct docs must be path-qualified (`[`SystemRegistrar::add`]`, `[`App::x`](crate::App::x)`) — bare `[add]` fails `RUSTDOCFLAGS="-D warnings"`.

## Files Changed

### Phase 1 — removals
- `src/resources.rs` (deprecated pair deleted, doc reword), `src/lib.rs` (export dropped), `src/app/render.rs` (step-2.5 drain deleted), `src/app/core_resources.rs` (registration deleted)
- `src/ecs/world.rs` (register_reflect deleted, register_reflect_named doc absorbed), `src/reflect.rs` (doc), `src/network.rs` (variant deleted + fmt refold), `src/app/assets.rs` (load_texture deleted), `src/particle.rs` (for_burst deleted), `examples/mp_client.rs` (allow + arm dropped)
- `src/animation/player.rs` + `mod.rs` + `system.rs` + `blend_system.rs` (shim deleted, imports repointed to renderer::uv), `src/timeline.rs` (Lerp shim deleted, tween import), `src/prefab.rs` (shim deleted, test qualified), `src/app/editor/ui/mod.rs` (call repointed to hierarchy), `src/components.rs` (facade deleted), `src/lib.rs` (topo re-export prefab→hierarchy group)

### Phase 2 — small breaking
- `src/ecs/schedule.rs` (SystemMeta deleted, compute_order + tests), `src/ecs/mod.rs`, `src/app.rs` (field type + test), `src/app/schedule.rs`, `src/app/scenes.rs`, `src/resources.rs` + `src/network.rs` (non_exhaustive + doc), `src/lib.rs` (line 75)
- `src/renderer/lighting.rs`, `src/renderer/post_process.rs`, `src/input/{mod,touch,map}.rs`, `src/ui/joystick.rs`, `examples/touch_demo.rs`, `src/lib.rs` (line 83)
- `src/material.rs` (rewrite), `src/renderer/sprite.rs` (hash call + getter use)

### Phase 3 — big items
- `src/physics/` all 8 files + `src/physics/mod.rs` + `src/physics/world/tests.rs`, `src/lib.rs` (physics group), `examples/crane_wrecking_ball.rs`
- `src/scene.rs` (SystemRegistrar + trait + doc examples + post-gate link fix), `src/app/scenes.rs` (registrar wiring), `src/app.rs` (2 test Scene impls), `src/lib.rs` (scene group), 8 example files (coin_race, orbital_dodger, predict_shooter, salvage_run, scene_flow, settings_menu incl. labeled demo, loading_bar, mp_client)
- `Cargo.toml` (serde rc), `src/components.rs`, `src/renderer/sprite.rs` + `sprite/sort.rs` + `sprite/tests.rs`, `src/tilemap.rs`, `examples/{security_camera,split_screen,minimap}.rs`

### Phase 4 — polish + release
- `examples/{basic,platformer,settings_menu,scene_flow}` (quit()), `Cargo.toml`+`Cargo.lock` (5.0.0), `docs/CHANGELOG.md` (5.0.0 entry), `docs/PATTERNS.md` (3 edits), `CLAUDE.md` (v1.6.0 + 6 rows), `REFERENCE.html` (81+/23−)

### Memory
- `engine-current-state.md` (rewritten for v5), `MEMORY.md` (index line)

## User Feedback & Preferences (REQUIRED — never omit)

- Session opener repeated the **5-step onboarding narration protocol with explicit wait-for-go-ahead** — same shape as seq 2's opener. Honor it again if the next paste prompt repeats it.
- Go-aheads stayed maximally terse: "1" (start v5 batch), "페이즈1 시작해", "1" (proceed to docs commit during close-out), "4" (run /handoff). **Keep next-step lists numbered** — the user answers in one or two characters.
- AskUserQuestion batching worked well: 3 open questions answered in one interaction, **all recommendations accepted** (non_exhaustive both / migrate examples / REFERENCE in-batch). Merge question also accepted the recommendation (merge commit). Pattern: state the rationale in the option description and the user follows it.
- User let the entire 4-phase breaking batch run without intervention between phases after the single plan approval — the plan-once-then-execute shape fits.
- Standing preferences honored: Korean prose to user / English artifacts (`conversation-language-korean`, `doc-language-rule`), aggressive parallel Sonnet subagents (`subagent-usage-preference`), `+1.88.0` everywhere (`ci-toolchain-pin`), explicit `model:` on every Agent call (`new-model-subagent-incompat`).
- "/handoff 하고 커밋" close pattern from prior sessions — this session the user picked "4" from my numbered list where 4 = handoff; commit follows per the close flow.
- The user gave **zero mid-execution corrections** this session — every intervention point was a designed checkpoint (plan approval, 3 open questions, merge method, close-out step). Read: the checkpoint cadence is right; don't add more interrupts, don't remove the existing ones.
- Recommendation acceptance rate this session: **5/5** (3 open questions + merge method + option-1 batch start). Across seq 2 + 3: the user consistently accepts when the option description carries the rationale (bisect-buildable, precedent, cost). Keep writing the "why" into the option text itself.
- The user resumed an interrupted flow with a bare "resume" — treat terse single-word inputs as "continue exactly where you left off", not as new instructions.
- Phase-boundary status reports in Korean with commit-hash tables were never questioned — that report shape (table of hash → content → gate state) is the accepted format for this user.

### Complete user-input timeline (verbatim — calibrates next session's interaction model)

| # | User input | Meaning |
|---|---|---|
| 1 | paste prompt: "Read plans/handoffs/HANDOFF_…findings-sweep-merge… (seq 2) and continue from Where We're Going… narrate your onboarding… wait for my go-ahead" | 5-step onboarding protocol, third session running |
| 2 | "1" | start the v5 batch (option 1 of 4) |
| 3 | AskUserQuestion: Both / Migrate / Update-in-batch | 3 open questions, all recommendations |
| 4 | "페이즈1 시작해" | begin Phase 1 execution |
| 5 | "1" | proceed with close-out step 1 (docs commit) while REFERENCE agent ran |
| 6 | AskUserQuestion: "Merge commit으로 머지 (추천)" | merge PR #13 |
| 7 | "4" | run /handoff (option 4 of the next-candidates list) |
| 8 | "resume" | continue interrupted handoff flow |

Eight inputs total for a 12-commit major release — the interaction budget this user expects. Every input except the paste prompt was ≤6 characters or a click.

### PR #13 body structure (template for future breaking-batch PRs)

Sections: title with the four headline items → one-paragraph summary naming the analysis round + bisect-buildable claim + CHANGELOG pointer → four phase tables (commit hash → change) → verification matrix (6 gates incl. smoke + bisect line) → **Consumer impact** paragraph (rust-survivors pins by rev, names the migrations it will need). This is the third PR in the precedent line (#8 v3 → #12 v4.6 → #13 v5) — reuse the shape.

## Where We're Going

1. **rust-survivors v5 migration** (first real one): bump `crates/game/Cargo.toml:20` rev to `c34b6c1`'s full hash, then follow `docs/CHANGELOG.md` ## 5.0.0 — expect: Scene impls → SystemRegistrar signature (`systems.add(...)`), physics handle types if the game names them, Sprite struct literals → `.into()`, any TouchState field reads → accessors, any removed-API usage (game was zero-deprecated-usage at 4.6.0, so removals likely free). Gate: `+1.88.0` fmt / clippy `-D warnings` / `test -p game --lib` (baseline 200/200). Do it in a dedicated session; the game tree has the user's own uncommitted doc changes — leave them strictly alone (seq-2 rule).
2. **Feature candidates split from the sweep** (each example-driven per VISION loop): `StateMachineSystem` crossfade_duration; scripting Arrive/Wander bindings; AudioEffect release-envelope implementation.
3. **Cleanup batch (cheap)**: delete merged remote branches `fix/analysis-top10` + `feat/v5-breaking-api` (`git push origin --delete <branch>`); prune the two stale local agent branches observed at onboarding (`worktree-agent-a63f768707c3c60f1`, `worktree-agent-aa7005699e3e08697` — leftovers from earlier sessions' worktree agents; `git branch -D`); optionally update `docs/CODE_ANALYSIS_2026-06-10.md`'s resolution header to say the v5 batch SHIPPED (it still says "planned"). Local branch `docs/english-conversion` also exists — NOT ours, leave unless the user asks.
4. **Optional**: re-test fable5-as-subagent after Claude Code updates; if fixed, delete `new-model-subagent-incompat` and stop forcing `model:`.
5. **Optional**: `v3-breaking-batch` memory is stale (describes PR #8 as "ready"; it merged long ago) — candidate for deletion next time memory is touched.

### rust-survivors v5 migration checklist (pre-built for Where We're Going #1)

Derived from what v5 actually broke × what the game is known to use (it gates with `-D warnings`, had zero deprecated-API usage at 4.6.0):

| v5 change | Game impact prediction | What to do |
|---|---|---|
| Removals (DebugDrawQueue, register_reflect, load_texture, for_burst, JsonParseError) | **none** — game was zero-deprecated-usage at the 4.6.0 bump (clippy `-D warnings` proved it) | nothing |
| Shim/facade removals | low — only if game imports deep paths (`engine::components::WindowConfig` style); root re-exports unchanged | grep game for `animation::player::`, `timeline::Lerp`, `components::{Window,Game,Should,Viewport}` |
| `Scene::on_enter` → SystemRegistrar | **certain** — game has Scene impls | signature + `systems.push(Box::new(X))` → `systems.add(X)`; consider `add_labeled` where the game has ordering comments |
| Physics handle newtypes | likely-free if game only round-trips handles (platformer precedent); breaks if it names rapier handle types or stores them in fields with annotations | grep game for `RigidBodyHandle`/`ColliderHandle` imports |
| `Sprite.texture: Arc<str>` | only struct-literal constructions break | grep `texture: Some(`; add `.into()` |
| `SystemMeta` | only if game references it (unlikely — scene-level API) | grep `SystemMeta` |
| `ShaderMaterial::new` | only if game uses custom shaders | grep `ShaderMaterial` |
| `non_exhaustive` NetworkEvent/DebugShape | only if game matches them exhaustively | gate will tell; add `_ =>` |
| TouchState accessors / input privatization | desktop game — probably no touch; check `input::` deep paths | grep `\.swipe\|\.pinch_delta\|input::map::` |
| `ShouldQuit` | `.0` stays pub — zero impact | optional cosmetic migration to `.quit()` |

Procedure: bump rev → `cargo update -p skeleton-engine` → let `cargo +1.88.0 clippy --all-targets -- -D warnings` enumerate the breaks → fix per table → `test -p game --lib` (baseline **200/200**) → surgical commit (Cargo.toml + Cargo.lock only; the user's uncommitted doc edits in that tree are NOT ours to stage).

### wasm_smoke raw result (final)

```
ok  : server logged a client connection (wasm WebSocket path works)
ok  : screenshot is 41974 bytes (>= 15000) -> rendered a real frame
>>> WASM SMOKE: PASS (run + connect + non-blank render)
```
Eyeball items verified on /tmp/wasm_smoke.png: "You are Player #1 — first to 10 wins!" HUD, "▶ Player #1: 0/10" scoreboard, white player square, 6 gold coins, "WASD / Arrows to move · grab gold coins" footer. Size 41,974 B vs seq-2's 41,974/41,977 B — same-class frame. This was the first smoke through the migrated coin_race (SystemRegistrar scene + Arc<str> sprite keys), so it doubles as the #8/interning acceptance check on a real game.

### Session task-tracker final state (in-session Task tool, not bd)

| # | Task | State |
|---|---|---|
| 1 | 5-gate baseline on main | completed (375/0, exit 0) |
| 2 | branch + batch plan (+ 3 open questions) | completed |
| 3 | #2 handle newtypes | completed |
| 4 | #8 SystemRegistrar | completed |
| 5 | removals (deprecated + shims) | completed |
| 6 | Sprite Arc<str> | completed |
| 7 | narrowings + small breaking | completed |
| 8 | bump/CHANGELOG/docs/gate/PR | completed |

### CHANGELOG 5.0.0 entry structure (what the migration guide covers)

- **Breaking — removed**: 6 bullets (queue pair, register_reflect, JsonParseError, load_texture, for_burst, shims+facade) — each names its replacement.
- **Breaking — API changes**: 7 bullets (#2 newtypes w/ escape hatch + inference note, #8 registrar w/ before/after signatures, Arc<str> w/ literal-fix note, SystemMeta merge, ShaderMaterial::new, non_exhaustive ×2, visibility narrowings incl. TouchState accessor names).
- **Changed**: 1 bullet (examples teach quit()).
- Every bullet has an inline *Migration:* clause — the PR body links here as "the migration guide".

## Risks & Blockers

- **rust-survivors compiles against 4.6.0 until migrated** — no urgency (rev-pinned), but any engine session that assumes "game tracks latest" will be wrong until Where We're Going #1 happens.
- **`#[non_exhaustive]` + accessor narrowings raise the fork-migration cost of v5** beyond the headline items — the CHANGELOG guide covers all of it, but forks that matched `NetworkEvent` exhaustively or read `TouchState` fields will hit compile errors with less-obvious fixes than the renames.
- **`SystemRegistrar` holds `&mut` to both App vecs during `on_enter`** — if a future change lets scenes reach `App` (same risk seq 2 flagged for exec_order), the lockstep invariant and the take/swap both need revisiting together.
- **REFERENCE.html was agent-edited and spot-checked, not fully proofread** — greps confirm no stale API names and structure looks right, but a human-eye pass over the rendered HTML hasn't happened.
- **gh auth still session/keyring-bound** — worked all session (ChunSam); if 401 returns, device-flow-on-local-browser is the known path for this user's remote setup.

## Open Questions

- None blocking. All three questions carried from seq 1/2 were answered this session (non_exhaustive: both; example migration: yes; REFERENCE.html: in-batch hand-curated), and the merge method settled (merge commit, third consecutive time).
- Soft/cosmetic only: should `docs/CODE_ANALYSIS_2026-06-10.md`'s resolution header get a "v5 shipped" paragraph? Folded into Where We're Going #3.
- Process question for the user, low priority: with merge-commit chosen 3/3 times, should future batch PRs just default to it (announce instead of ask)?

## Quick Start for Next Session

```bash
# Current state
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3 main     # c34b6c1 = merge of PR #13 (v5.0.0); tree clean; everything pushed
gh pr view 13                  # the merged breaking-batch PR (gh authenticated as ChunSam)

# Canonical context
# - docs/CHANGELOG.md ## 5.0.0       <- THE migration guide (per-item)
# - docs/PATTERNS.md                  (SystemRegistrar pattern + ordering table)
# - CLAUDE.md v1.6.0                  (module map current)
# - this file + parents (seq 1: v5 spec origin; seq 2: sweep + widened #2 radius)

# If doing rust-survivors v5 migration (next action):
cd /Users/jkl/Projects/rust-survivors
git log --oneline -2           # f18c5b0 = 4.6.0 pin bump
# edit crates/game/Cargo.toml:20 rev -> $(cd ../skeleton-engine && git rev-parse c34b6c1)
# cargo update -p skeleton-engine, then fix compile errors per CHANGELOG guide
# gate: cargo +1.88.0 fmt --check && cargo +1.88.0 clippy --all-targets -- -D warnings \
#       && cargo +1.88.0 test -p game --lib    # baseline 200/200
# NOTE: the game tree has the user's own uncommitted doc changes — do not stage them

# Verify engine state (CI pin — memory ci-toolchain-pin)
cargo +1.88.0 fmt --check && cargo +1.88.0 clippy --all-targets -- -D warnings \
  && cargo +1.88.0 build --target wasm32-unknown-unknown \
  && cargo +1.88.0 test --all-targets \
  && RUSTDOCFLAGS="-D warnings" cargo +1.88.0 doc --no-deps
# Expect: 375 passed / 0 failed

# Next action
# rust-survivors v5 migration (Where We're Going #1) — or feature work (#2) if the user prefers.
```

## Session Closed
**Closed at:** 2026-06-11
**Commit:** see `session: v5-breaking-batch [code-analysis-2]` on main (handoff file only — all code/doc work was committed and merged via PR #13 during the session)
**Session status:** Handed off to next session
