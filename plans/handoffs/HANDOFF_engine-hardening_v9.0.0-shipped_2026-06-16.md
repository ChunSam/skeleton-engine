# Engine-wide hardening batch (80-finding analysis) shipped as v9.0.0 — concurrent-session reconciliation pending

**Date:** 2026-06-16
**Status:** COMPLETED (all WU1–14 implemented, Gate6 green, v9.0.0 finalized & committed) — NOT pushed/merged; concurrent-session reconciliation pending
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine — engine hardening / robustness pass
**Chain:** `engine-hardening` seq `1`
**Parent:** `none — first in chain` (the prior interrupted session left no handoff file, only a RESUME-STATE note inside `plans/code-analysis-2026-06-16-progress.md` @ commit `613b9c6`)
**Prior chain:** none — first in chain

## Related Handoffs

- `HANDOFF_editor-tile-painting_rust-survivors-dropped_2026-06-16.md` (seq 5) — the paste-prompt that *started* this session pointed here, but the work pivoted to engine hardening (separate stream). Reference only, NOT a parent.
- `HANDOFF_editor-tile-painting_items1-5-batch_2026-06-16.md` (seq 4) — its grandparent; v8.24→8.27 feature batch. Reference only.

## ⚠️ COMMITS TO CHECK (the user's explicit ask: "확인해야 하는 커밋 정리")

Branch `chore/engine-hardening-2026-06-16` is **10 commits ahead of `origin/main`, unpushed, not on any remote.** Base = `42de46c` (= `main`). Read top→down (newest first).

| Commit | Author/source | What | Check? |
|---|---|---|---|
| `f4894ea` | **THIS session** | release: v9.0.0 (Cargo.toml/CLAUDE.md/CHANGELOG/REFERENCE) | review version-bump decision (9.0.0 for breaking) |
| `305d6d3` | **THIS session** | batch 4 — WU12 app-loop + WU14 build/CI + Gate6-green cleanup | normal review |
| `a6aefdb` | **THIS session** | batch 3-part2 — WU10 pathfinding/timeline + WU11 editor | normal review |
| `e96529d` | **⚠️ CONCURRENT SESSION** | "iteration-3 repaired baseline (692 tests)" — scripting WU9 dead-field removal | **RECONCILE** — a different session committed this on top of my `92e8141`; it overlaps the WU9 scripting fix I also made. Interleaved into my history. |
| `92e8141` | **THIS session** | batch 3-part1 — WU8 network / WU9 assets / WU10 save+behavior+steering | normal review |
| `7d99607` | **THIS session** | style: cargo fmt --all (latent formatting from batches 1–2) | trivial (fmt only) |
| `613b9c6` | prior interrupted session | docs(plan): RESUME-STATE note in progress doc | context only |
| `f5260ef` | prior interrupted session | batch 2 — WU5,6,7,13 (renderer/camera/ecs/input) | already gated by prior session |
| `0d6da75` | prior interrupted session | batch 1 — WU1–4 (animation/physics/audio/ui) | already gated by prior session |
| `93f2275` | prior interrupted session | docs: 80-finding audit + execution plan | context only |

**The one that matters most: `e96529d`.** It is NOT mine. A concurrent session (same git user `ChunSam`, same working tree) committed it ~the same minute as my `92e8141`. It contains only the WU9 scripting dead-field removal (`api.rs`/`context.rs`/`execution.rs`) — work I ALSO did. The two sessions raced on the iteration-3 repair. The user is aware ("다른세션에서 별도 작업 진행하고 있음 … 일단 작업 후 나중에 정리"). Decision deferred to the user.

## The Goal

Complete the **engine-wide hardening batch** — 80 findings from a 14-subsystem code analysis (`docs/CODE_ANALYSIS_2026-06-16.md`), executed as work units WU1–14 across 5 iterations on branch `chore/engine-hardening-2026-06-16`. A prior session hit a usage limit mid-Iteration-3, leaving a **non-compiling working tree**. This session resumed, repaired the tree, finished Iterations 3–5, and shipped the whole batch as **v9.0.0** (major bump because the batch includes breaking changes). The dominant theme of the findings is **fail-loud over fail-quiet** + fork-friendliness, matching the skeleton-engine VISION (a hackable, forkable 2D engine).

## Where We Are

- **`main` base = `42de46c`. Branch HEAD = `f4894ea` (v9.0.0). Clean tree. 10 commits unpushed; NOT merged to main.**
- **All WU1–14 implemented.** Iter 1 (WU1–4) + Iter 2 (WU5,6,7,13) by the prior session; **Iter 3 (WU8–11), Iter 4 (WU12, WU14), Iter 5 (finalize) by THIS session.**
- **Full Gate6 GREEN:** `cargo fmt --check` · `cargo clippy --all-targets -D warnings` · `cargo build --target wasm32-unknown-unknown` (lib+bins) · `cargo test --all-targets` · `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`.
- **Tests: lib 698 pass** (hardening started at 603 → **+95**) **+ 33 new integration tests** (`tests/{pathfinding,timeline,behavior,save}_smoke.rs`).
- **Resumed from a non-compiling tree.** The prior session's interrupted agents left WU8/9/10 partially wired. I salvaged it: particle `z` field was added to the struct but spawn-path plumbing (`spawn_particle` arity, snapshot tuple, `config_set.rs` literal) was incomplete → E0063/E0308; `NetworkSystem` gained a field, breaking 5 examples (E0423); a Selector test had wrong semantics.
- **WU8 Network (#29,30,31,64,65,66):** bounded receive queue + overflow marker; `NetworkConfig.max_buffered_bytes`; `disconnect()` close-flag side-channel; `RemoteEntities` is_alive re-spawn; warn-once for missing `Events<NetworkEvent>`. `NetworkSystem` is now a struct → I added `::new()` + `#[derive(Default)]` and updated 5 examples.
- **WU9 Assets/tilemap (#1,11,12,13,51,56):** blob_47 `VALID_MASKS` corrected (36 reachable masks no longer fall back to tile 0); Tilemap `generation` dirty-guard; removed-entity HashSet; DataTable extra-columns warn; `ParticleEmitter::z`/`with_z` (+ `EmitterDef` serde `z`); ScriptCommands dead `spawned_ids` removed.
- **WU10 Save/behavior/steering/path/timeline (#41,43,76,78,42):** `save_versioned_with_key`/`load_migrated_with_key`; Sequence/Selector `reset()` resets children on completion; steering scratch reuse; `find_path`/`find_path_diagonal` blocked-`start==goal` → None; `Track::set_value`/`set_easing`.
- **WU11 Editor (#23,24,61,62,42/44):** factory+remover for all 8 serde UI widgets; `SerdeComponentRegistry::component_names_for` (presence check, no RON serialize); pathfinding-overlay tiles-only snapshot (no full Tilemap clone); `add_component_selected` revalidated after scene reload; Timeline keyframe easing ComboBox.
- **WU12 App-loop (#8,9,10,28,48,49,63):** `Focused(false)`→`release_all()`; `step_frame_once` double-step guard; touch logical coords; panic frame-abort; `forward_reloads!` macro dedup; offscreen raw-`*const TextureView` → owned `TextureView` (unsafe removed); FadeTransition wasm warn-once.
- **WU14 Build/CI (#17,19,20,55):** `serde_json`→dev-deps; MSRV 1.92→1.95; wasm CI clippy step; 4 integration tests.
- **v9.0.0 finalized:** Cargo.toml `8.27.0→9.0.0`, CLAUDE.md header `v1.6.33 / v9.0.0 / MSRV 1.95`, comprehensive CHANGELOG entry, REFERENCE.html v9.0.0 hardening section.
- **Reproduce the green gate:** `./scripts/verify.sh` (runs the full Gate6) or run the steps individually per `CLAUDE.md`. Local Gate6 is green; **CI has NOT run** (branch unpushed) — and WU14 just added a `cargo clippy --target wasm32 --lib -D warnings` step to the wasm CI job, so the first remote run exercises a never-before-run lint pass.
- **Nothing pending in the working tree** (clean); all session work is in the 5 commits `7d99607`→`f4894ea` (plus the concurrent `e96529d` interleaved).
- **Task list (this session) all completed:** WU9-scripting, WU10-path/timeline, WU11-editor, WU12-app-loop, WU14-build/CI, Iter5-finalize.

## Since Last Handoff

(No formal parent handoff; comparing against the prior session's RESUME-STATE note in the progress doc.)

- The RESUME note predicted exactly the two compile errors I hit (NetworkSystem examples E0423; particle `z` tuple/literal). **Both fixed** — but via a DIFFERENT approach than the note suggested: the note proposed keeping `NetworkSystem` a unit struct with a `static AtomicBool` warn-flag; I instead kept the field + `::new()` (matches the codebase's `StateMachineSystem::new()` pattern, cleaner). Documented divergence.
- The note's "MISSING/NOT DONE" list (WU9-scripting, WU10-pathfinding+timeline, WU11) — **all completed this session.**
- **Trajectory shift the note did not anticipate:** a *concurrent* session is independently doing the same resume, and committed `e96529d`. This was discovered mid-session via reflog/`git show`. The user confirmed and chose "proceed now, reconcile later."

## Reference Documents

- `docs/CODE_ANALYSIS_2026-06-16.md` — the 80-finding audit (severity/file:line/effort/breaking table in Appendix A). Authoritative finding list.
- `plans/code-analysis-2026-06-16-progress.md` — execution plan + per-WU status + the prior session's RESUME-STATE note. **NOTE: I did NOT update its WU checkboxes** (high concurrent-session conflict risk) — CHANGELOG + commits are the authoritative record.
- `CLAUDE.md` — Gate6 "Verification" checklist + conventions.
- `docs/CHANGELOG.md` — the 9.0.0 entry is the full human-readable record of the batch.

## What We Tried (Chronological)

1. **Onboarding on the wrong chain.** Session started reading `editor-tile-painting` seq-5 handoff (serde-persistence next-action). Verified git == `42de46c`, `cargo test --lib` == 603. Asked the user which next-action; the AskUserQuestion was rejected ("clarify"), then the user pivoted entirely: **"limit으로 멈춘 작업 다시 진행."**
2. **Discovered the real work via `git status`.** Branch was actually `chore/engine-hardening-2026-06-16` with 3 hardening commits + 7 files of uncommitted Iteration-3 WIP. Read `docs/CODE_ANALYSIS_2026-06-16.md` + `plans/code-analysis-2026-06-16-progress.md` to understand the loop (Opus supervises, Sonnet implements; per-iteration gate = `test --lib` + `clippy --lib`).
3. **Gated the uncommitted tree → NON-COMPILING.** `cargo test --lib` failed: particle `z` half-wired (E0063 in `config_set.rs`, E0308 12-vs-13 tuple), `NetworkSystem` broke 5 examples (E0423).
4. **Repaired the compile.** Finished particle `z` plumbing (snapshot tuple binding, `spawn_particle(z)`, both call sites, `Transform{z}`, `EmitterDef` serde `z` + `config_set` literal) + a `with_z` test. `NetworkSystem::new()` + derive(Default) + sed-patched 5 examples. → compiles, 690 pass / 1 fail.
5. **Fixed the 1 failing test (wrong semantics, not a code bug).** `selector_resets_children_on_failure` used a `CountingNode` that returns Success — but a Selector *succeeds* on child success, never advancing to Failure. Generalized `CountingNode` with a terminal status + `new_failing()`. → 692 pass.
6. **Discovered latent fmt churn.** `cargo fmt` reformatted ~22 iter1/2 files (the per-iteration gate omitted `fmt --check`; `git stash` confirmed HEAD itself was fmt-dirty at `blend_tree.rs:60`). Split into a `style:` commit (`7d99607`, fmt-only files) + a `logic:` commit (`92e8141`, my files) to keep history clean.
7. **Discovered the concurrent session (`e96529d`).** While verifying my WU9-scripting edits, found a commit I didn't author on top of my HEAD. Investigated reflog/`git show` → confirmed external. Surfaced to the user via AskUserQuestion; user said proceed + reconcile later.
8. **Finished Iteration 3 (WU9-scripting, WU10-path/timeline directly; WU11 via Sonnet agent).** Committed `a6aefdb`. WU11 agent did all 5 findings correctly (verified names match serde keys, overlay predicate preserved, easing apply-order valid).
9. **Iteration 4 via 2 parallel Sonnet agents** (WU12 app-loop, WU14 build/CI) — disjoint files. Both returned correct work.
10. **Comprehensive Gate6 → 3 failures.** (a) `scene_replace_clears_panicked_systems` broke (WU12 frame-abort changed Counter 1→0). (b) wasm E0425 `web_sys::ErrorEvent` (WU8 needed the web-sys feature). (c) clippy `--all-targets` 6 lints (latent test-module lints never caught by lib-only gate + new). Fixed all + the doc gate (4 broken intra-doc links). Committed `305d6d3`.
11. **Finalized v9.0.0** (`f4894ea`) after the user chose "내가 마무리 진행."

## Key Decisions

- **v9.0.0 (major bump), not 8.28.0.** The batch has breaking changes (NetworkSystem unit→struct, GamepadButton/Axis `#[non_exhaustive]`, SerdeComponentEntry +field, SolidTiles::Only Vec→HashSet, MSRV 1.92→1.95). Precedent: `v8.0.0` was the prior breaking major (per the seq-4 handoff's "v8.0.0 BREAKING window"). So major = breaking → 9.0.0. **Rejected:** treating it as a minor bump.
- **`NetworkSystem` as a field-struct + `::new()`**, diverging from the prior session's RESUME-note suggestion (unit struct + `static AtomicBool`). Reason: matches the codebase's existing `StateMachineSystem::new()`/scratch-field idiom; warn-once-per-instance is the correct semantic.
- **Split style vs logic commits.** The repo-wide `cargo fmt` touched 22 previously-committed files; bundling that with new logic would obscure review. → dedicated `style:` commit first.
- **Did NOT update the progress-doc WU checkboxes.** It's the file most likely also being edited by the concurrent session; CHANGELOG + commits are the authoritative record. Avoids a tug-of-war.
- **Did NOT push or merge to main.** Standing rule "push is the user's"; plus the concurrent-session reconciliation must happen first.
- **Latent-lint cleanup folded into batch 4.** The per-iteration gate was lib-only (`test --lib` + `clippy --lib`); `--all-targets` + wasm + doc were deferred to the final gate, so iter1–3 accumulated test-module clippy lints + broken doc links + a wasm feature gap. Fixed all in the final gate rather than amending old commits.
- **Delegated WU11/12/14 to Sonnet sub-agents, did WU9-scripting/WU10 inline.** Small surgical fixes inline (faster than agent round-trip); larger multi-finding WUs delegated per the loop model + subagent-usage preference. Reviewed + gated every agent's output.
- **`#9` hot-reload shipped as a `macro_rules!` dedup, not a full `HotReloadable` trait** (the agent's "smaller safe version"; full trait deferred — noted in CHANGELOG).
- **REFERENCE.html: appended a v9.0.0 section, not a full rewrite.** Its top blockquote was already stale at "v5.0.0 기준" (not maintained per-release); a full refresh was out of scope.

## Evidence & Data

### Iteration → WU → version → commit → tests

| Iter | WUs | Done by | Commit(s) | Lib tests |
|---|---|---|---|---|
| 1 | WU1–4 (animation/physics/audio/ui) | prior session | `0d6da75` | 638 |
| 2 | WU5,6,7,13 (renderer/camera/ecs/input) | prior session | `f5260ef` | 677 |
| 3 | WU8–11 (network/assets/save/behavior/scripting/path/timeline/editor) | **this + concurrent `e96529d`** | `92e8141`, `a6aefdb` (+ `e96529d`) | 697 |
| 4 | WU12, WU14 (app-loop, build/CI) | **this** | `305d6d3` | 698 |
| 5 | finalize (v9.0.0) | **this** | `f4894ea` | 698 |

### Gate6 failures fixed in the final pass (this session)

| Failure | Cause | Fix |
|---|---|---|
| `scene_replace_clears_panicked_systems` FAILED | WU12 frame-abort: CountSystem no longer runs the frame PanicSystem panics → Counter 0 not 1 | test asserts 0 (scene A) and 2 (scene B) |
| wasm E0425 `web_sys::ErrorEvent` | WU8 `on_error` uses ErrorEvent; web-sys feature absent | added `"ErrorEvent"` to `web-sys` features in Cargo.toml |
| clippy --all-targets ×6 | latent test-module lints (lib-only gate missed) + new | for_kv_map (tilemap), field_reassign×3 (prefab+touch), unused import (gpu_particle), collapsible_match (window→`step_frame_once` helper) |
| doc -D warnings ×4 | broken intra-doc links | `has_component_typeid`/`World::despawn`×2 → code spans; `serialize_entity`→`Self::serialize_entity` |

### New tests added this session

- `particle::emitter_z_propagates_to_spawned_particles`
- `behavior::CountingNode::new_failing` + fixed `selector_resets_children_on_failure`
- `pathfinding::blocked_start_equals_goal_returns_none`
- `timeline::track_set_value_and_easing`
- WU11 agent: `prefab::component_names_for_{present_and_absent,all_present_sorted,none_present}`
- WU12 agent: `app::panic_aborts_remaining_systems_this_frame`
- WU14 agent: `tests/{pathfinding,timeline,behavior,save}_smoke.rs` (6+12+12+3 = 33)

### WU → findings coverage (all 80 confirmed findings, by work unit)

| WU | Subsystem | Findings addressed (audit #) | Iter |
|---|---|---|---|
| 1 | animation | 3,4,5,6,7,47 | 1 |
| 2 | physics | 32,33,67,68,69,70 | 1 |
| 3 | audio | 14,15,16,53,54,52 | 1 |
| 4 | ui | 44,45,46,79,80 | 1 |
| 5 | renderer-core | 34,35,36,37,71,72 | 2 |
| 6 | renderer-lighting/camera | 38,39,40,73,74,75 | 2 |
| 7 | ecs/reflect/prefab | 21,22,57,58,59,60 | 2 |
| 13 | input | 2,25,26,27,(63 shared) | 2 |
| 8 | network | 29,30,31,64,65,66 | 3 |
| 9 | assets/scripting/tilemap | 1,11,12,13,50,51,56 | 3 |
| 10 | save/behavior/path/timeline | 41,42,43,76,77,78 | 3 |
| 11 | editor | 23,24,61,62,(42/44 wiring) | 3 |
| 12 | app-loop | 8,9,10,28,48,49,63 | 4 |
| 14 | build/CI | 17,19,20,55 | 4 |

(Rejected/false-positive findings: 4 — see audit §9. A handful of LOW items may be partially covered or noted-only; CHANGELOG is the authoritative shipped list.)

### VISION / example-acceptance angle

No NEW example game was added (this is a hardening pass, not a feature). The changes are exercised by: the **33 new crate-boundary integration tests** + the existing examples that touch the changed paths (`dig_quest` for tile colliders, `rtl_text`, `timeline_cutscene`, `sm_crossfade`, the 5 network examples now using `NetworkSystem::new()`). A human eyeball of the editor panels + `cargo run --example` for the input/focus changes is worth doing with display access (the docked cursor-freeze keeps autonomous visual validation weak — recurring ceiling).

### How the batch was executed (loop / operating model)

From `plans/code-analysis-2026-06-16-progress.md`: **"Opus plans/reviews/supervises, Sonnet implements. No intermediate reports; full code review + tests only at the end; final report when clean. Agents EDIT + write inline `#[cfg(test)]` tests, do NOT run cargo (avoid lock contention); Opus runs the central gate per iteration. Agents must NOT edit `src/lib.rs` (report needed re-exports → Opus adds centrally)."**

This session honored that model: small surgical WUs (WU9-scripting, WU10) done inline by Opus; larger multi-finding WUs (WU11 editor, WU12 app-loop, WU14 build/CI) delegated to **Sonnet sub-agents** (explicit `model: sonnet` per `[[new-model-subagent-incompat]]`), with Opus running every gate + reviewing every diff. Agent track record this session: WU11 (5/5 findings correct), WU14 (4/4 correct), WU12 (7/7 but introduced the 3 gate failures that the final pass fixed — frame-abort test, web-sys feature, latent clippy). Sub-agents are reliable WITH precise prompts (exact file:line + patterns + "do NOT run cargo / do NOT edit lib.rs / report re-exports").

### Integration tests added (WU14, `tests/`)

- `pathfinding_smoke.rs` (6): adjacent path, blocked-only-route None, blocked goal None, `start==goal` single-elem, diagonal path, OOB `is_walkable` no-panic.
- `timeline_smoke.rs` (12): empty/clamp/lerp (f32+Vec2), out-of-order sort, remove/clear, Timeline state, `is_finished`, looping, `restart`, `set_time` re-sort.
- `behavior_smoke.rs` (12): Sequence all-succeed/abort/Running, Selector first-success/all-fail, Inverter, AlwaysSucceed, Blackboard typed round-trip + `set_path`/`get_path`.
- `save_smoke.rs` (3, `#![cfg(not(wasm32))]`): `write_ron`/`read_ron` round-trip, missing-file `SaveError::Io`, `exists`/`delete`. Uses `std::env::temp_dir()` (no new deps).

### Concurrent-session reconciliation recipe (concrete)

```bash
# Is the other session on a separate branch, or the same one?
git branch -a; git for-each-ref --sort=-committerdate refs/heads/ --format='%(refname:short) %(committerdate:iso) %(subject)' | head
# What did e96529d change vs my scripting edits? (should be identical / a subset)
git show e96529d -- src/scripting/
# Anything uncommitted from the other session in the shared tree right now?
git status -s
# The 10 commits to adjudicate:
git log --oneline 42de46c..HEAD
```

### Prior-session work units (WU1–7, WU13) — for completeness (from the progress doc)

These were done + committed by the prior interrupted session (`0d6da75`, `f5260ef`) before this session resumed. Not re-verified line-by-line this session beyond the green Gate6, but recorded so the v9.0.0 batch is fully documented:

- **WU1 Animation:** `columns==0` div-by-zero guard; OOB frame validate; `play(OOB)` guard + `is_finished()=false` fallback; `add_transition` dead-edge warn; skeletal `is_finished()` started-guard; `BlendTree1D` sort entries in `new()`.
- **WU2 Physics:** prismatic zero-axis guard; `contact_pairs` ordered-pair symmetry; 4 scratch Vecs → fields; `SpatialGrid::rebuild` no-collect; `SolidTiles::Only`→HashSet (additive ctor); `TilemapColliders` despawn-leak helper (native-gated).
- **WU3 Audio:** `set_bus_volume`/`set_volume` fade guard; `update_position` fade guard; `clear_file_cache()`; scratch reuse; wasm rustdoc note.
- **WU4 UI:** `Panel::direction` Reflect (`LayoutDir` to_i32/from_i32); text_input focus z-order + visible guard; `Slider` set_field `initial_value`→`value`; `LocalizationSystem` `TextInput.placeholder`.
- **WU5 Renderer-core:** atlas `texture_path_arc` + sprite scratch fields; glyphon prepare/render log + shaped-buffer cache; bloom `texel_size` uniform; gpu_particle ring-buffer base_slot partition.
- **WU6 Renderer-lighting/camera:** light cull viewport-center + frustum prefilter; camera shake in `screen_to_world`/`world_to_screen` + zoom/shake accessors + `zoom_to` guard; `RenderTarget` `clear_color`.
- **WU7 ECS/reflect/prefab:** `has_component<T>()`; `query_added/changed` empty guard; events doc fix; `serialize_entity` ron-err log; `spawn_entity_def` missing-registry log; inspector write-back TypeId-keyed.
- **WU13 Input:** gamepad axis in `just_pressed`/`just_released` + `axis_value()`; `#[non_exhaustive]` on GamepadButton/GamepadAxis; `release()` guard + `release_all()`; touch coord docs + `swipe_threshold` field.

### Concurrent-session forensics (how `e96529d` was diagnosed)

- First clue: after committing `92e8141`, `git status` showed scripting files CLEAN even though I'd edited them → `git diff HEAD src/scripting/` empty AND `spawned_ids` gone → my edits matched HEAD, but `92e8141` didn't contain them.
- `git log -- src/scripting/context.rs` → top commit `e96529d` "iteration-3 repaired baseline (692 tests)" — a message I never wrote.
- `git reflog` showed a LINEAR progression: …`92e8141` (HEAD@{1}, mine) → `e96529d` (HEAD@{0}, NOT mine). `git show -s e96529d` → author `ChunSam`, commit-date `08:49` (≈ right after my `92e8141`), touches ONLY the 3 scripting files.
- No `index.lock`, stable HEAD, no src files modified in the last 3 min → the concurrent process was momentarily DORMANT when checked. Later a "Wait for clippy" background task fired (the other session running its own gate) — confirming it's still alive.
- Conclusion: a parallel Claude session, same working tree + `.git`, racing the same resume. It selectively committed the scripting fix (its own iteration-3 baseline). My WU10/WU11 stayed uncommitted on top until I committed `a6aefdb`.

### Breaking changes (drive the v9.0.0 bump)

- `NetworkSystem` unit→struct: `app.add_system(NetworkSystem::new())`.
- `GamepadButton` / `GamepadAxis` `#[non_exhaustive]` (downstream match needs `_`).
- `SerdeComponentEntry` +`has_component` field (manual struct-literal construction breaks).
- `SolidTiles::Only` `Vec<u32>`→`HashSet<u32>` (additive `IntoIterator` ctor).
- MSRV `1.92`→`1.95`.

## Code Analysis

- **`NetworkSystem`** (`src/network.rs`): `#[derive(Default)] struct { warned_missing_events: bool }`; `new()`, `LABEL`. The receive-queue overflow marker (`push_event_bounded`) keeps `len ≤ capacity` and reserves the back slot for a `ReceiveQueueFull` marker.
- **`step_frame_once`** (`src/app/render.rs`, next to `step_frame`): `if !self.stepped_this_iteration { self.stepped_this_iteration = true; self.step_frame(event_loop); }`. Called by both the `Resized` (native-gated) and `RedrawRequested` arms; flag reset in `about_to_wait`.
- **Panic frame-abort** (`src/app/schedule.rs` ~line 350): `DisableSystemAndContinue` now records the panicked index AND `break`s the per-frame system loop (World may be half-mutated); `AbortAfterLog` re-panics.
- **Offscreen owned views** (`src/app/render.rs` ~525): `OffscreenRenderInfo` holds an owned `wgpu::TextureView` built via `rt.texture.create_view(&Default::default())` (zero-cost handle clone), decoupled from `render_targets` — raw `*const` + `unsafe` gone.
- **`SerdeComponentEntry::has_component`** (`src/prefab.rs:128`): `Box<dyn Fn(&World, Entity)->bool>` = `world.get::<T>(e).is_some()`; `component_names_for` filters by it, sorts, no RON serialize.
- **`Track<T>`** (`src/timeline.rs`): `set_value(i,v)->bool` / `set_easing(i,e)->bool` (no re-sort — value/easing don't affect ordering, unlike `set_time`).
- **`Easing`** (`src/tween.rs`): exactly 6 variants — Linear/EaseIn/EaseOut/EaseInOut/EaseInBack/EaseOutBack (the editor's `easing_variants()` ComboBox enumerates all 6).

## Reusable Engineering Gotchas (this session)

- **The per-iteration gate (`test --lib` + `clippy --lib`) silently misses a LOT.** It does NOT catch: `fmt --check` drift, `clippy --all-targets` test-module lints (for_kv_map, field_reassign_with_default, unused imports in `#[cfg(test)]` mods), broken intra-doc links (`doc -D warnings`), wasm-only compile errors (missing `web-sys` features), and example/integration-test breakage. Iterations 1–3 accumulated ~10 such latent issues that only surfaced at the final `--all-targets`/wasm/doc gate. **Run the FULL Gate6 at least once before declaring an iteration done**, or budget a cleanup pass.
- **`physics` is native-only** — any `crate::physics` ref in a non-gated module breaks the wasm lib build (E0433). (Carried gotcha, still true.)
- **`web-sys` types are feature-gated per-type.** Using `web_sys::ErrorEvent` requires `"ErrorEvent"` in the `web-sys` features list — the native build is fine; only the wasm build catches it (E0425).
- **`[dev-dependencies]` ARE available to examples + tests + benches.** Moving `serde_json` there does NOT break the examples that use it (rust-analyzer's E0433 flood was stale; `cargo` confirmed fine).
- **zsh does NOT word-split unquoted variables.** `git restore --staged $FILES` passed the whole string as one pathspec and failed. Pass paths as explicit args, or use an array / `${=VAR}`.
- **`cargo fmt` reformats the WHOLE workspace**, not just your files — if earlier commits were fmt-dirty (no `fmt --check` in their gate), a single `cargo fmt` produces churn across many committed files. Split that into a dedicated `style:` commit.
- **`cargo fmt` rewraps freshly-added test asserts** → a follow-up `Edit` then fails ("string not found"). Re-Read after `cargo fmt`. (Carried.)
- **rust-analyzer phantoms persist** (`expected ColliderHandle, found ColliderHandle` E0308; "file not in module tree" for `config_set.rs`; stale "missing field"). ALL cleared by `cargo check`. Trust the compiler, not the IDE snapshot. (Carried.)
- **Concurrent sessions share one working tree + `.git`.** `git add -A` by either session sweeps the other's uncommitted files into your commit. Always commit with EXPLICIT paths when a parallel session may be active, and re-check `git rev-parse HEAD` before each commit (HEAD can move under you).
- **A clippy `derivable_impls` lint** fired on WU8's manual `impl Default for NetworkSystem` (a single-bool field) → replace with `#[derive(Default)]`.
- **`collapsible_match`** fires when a `match` arm's body (after cfg-stripping) is a single `if` — the suggested match-guard collapse can be WRONG when a `#[cfg(wasm32)]` block also lives in the arm (the guard would skip it). Extract a helper method instead.

## Files Changed (this session)

### Source — Iter 3
- `src/particle/{mod.rs,config_set.rs}` (z plumbing), `src/network.rs` (+`new()`/derive), `src/scripting/{api,context,execution}.rs` (dead field — also in concurrent `e96529d`), `src/pathfinding.rs` (start==goal guard), `src/timeline.rs` (set_value/set_easing), `src/app/editor.rs` + `src/app/editor/ui/docked.rs` + `src/prefab.rs` (WU11), `src/behavior.rs` (reset + test fix), `src/input/state.rs` (temp allow, later removed by WU12), 5 examples (`NetworkSystem::new()`).

### Source — Iter 4
- `src/app/{window.rs,schedule.rs,render.rs}`, `src/app.rs`, `src/input/{state.rs,touch.rs}` (WU12); `src/{gpu_particle,tilemap,prefab,ecs/world,physics/world/tile_collider}.rs` (final-gate lint/doc fixes).

### Tests
- `tests/{pathfinding,timeline,behavior,save}_smoke.rs` (NEW, WU14).

### Config / docs
- `Cargo.toml` (serde_json→dev, MSRV 1.95, web-sys ErrorEvent, version 9.0.0), `Cargo.lock`, `.github/workflows/ci.yml` (wasm clippy), `CLAUDE.md` (header), `docs/CHANGELOG.md` (9.0.0), `REFERENCE.html` (v9.0.0 section).

## User Feedback & Preferences (REQUIRED — never omit)

- **"limit으로 멈춘 작업 다시 진행"** — resume the limit-stopped work (the engine-hardening batch), NOT the editor-tile-painting onboarding I'd started.
- **On the concurrent session: "다른세션에서 별도 작업 진행하고 있음. 이쪽 작업과 유관할 수 있으나 일단 작업 후 나중에 정리"** — a parallel session is doing related work; proceed now, reconcile later. (So I committed only my explicit files, never `git add -A` blindly when other changes might be present.)
- **Finalization: chose "내가 마무리 진행"** (the AskUserQuestion option = "Claude does the finalization") — so I did the v9.0.0 bump + CHANGELOG + REFERENCE.
- **This `/handoff` ask: "하고 확인해야하는 커밋 정리해서 알려줘"** — produce the handoff AND a clear list of commits to check (see the COMMITS TO CHECK table at top).
- **Standing:** Korean prose to the user; English code/docs/handoff. Subagents on Sonnet with explicit `model:`. Push/merge is the user's call. Never drop CLAUDE.md content to hit ≤200 lines.

## Where We're Going

1. **Reconcile the concurrent session FIRST.** Compare what the other session did vs this branch. `e96529d` (its scripting commit) is already interleaved into this history; check whether the other session also produced its own WU10/WU11/WU12/WU14/finalize commits elsewhere (another branch? uncommitted in the shared tree?). Decide which lineage is canonical, or merge. The two raced on iteration-3 repair.
2. **Then push / open PR / merge to main** (the user's call). Branch is 10 commits ahead of `origin/main`, unpushed. CHANGELOG/version already say 9.0.0.
3. **(optional) Update `plans/code-analysis-2026-06-16-progress.md`** WU checkboxes to DONE once the reconciliation settles (left untouched to avoid concurrent conflict).
4. **Deferred follow-ups (noted in CHANGELOG):** full `HotReloadable` trait (shipped as a macro dedup); REFERENCE.html full refresh (currently v5.0.0-era body + a v9.0.0 section); SM visual node-graph / timeline time-ruler (from the editor chain, unrelated).

## Risks & Blockers

- **Concurrent session shares this working tree + `.git`.** Inherent clobber risk: a `git add -A` by either session sweeps the other's uncommitted files; HEAD can move under you. Mitigated this session by explicit-path commits + frequent HEAD checks, but the two lineages still need human reconciliation. **This is the one real blocker to "done".**
- Branch unpushed/unmerged — no CI has run remotely (local Gate6 is green; CI mirrors it but adds a wasm clippy step WU14 just introduced — first remote run may surface env-specific issues).

## Open Questions

- How to reconcile the two sessions' work — which lineage is canonical, or merge both? (User: "나중에 정리".)
- Is v9.0.0 the right number, or does the project prefer to minimize the bump to 8.28.0 despite the breaking changes? (I chose 9.0.0 per the `v8.0.0`-was-breaking precedent; user endorsed via "내가 마무리 진행".)
- Should the deferred `HotReloadable` trait be done before v9.0.0 ships, or as v9.1.0?
- Were ALL 80 findings actually addressed, or are some LOW-severity ones noted-only? The WU→findings matrix maps coverage by intent, but a line-by-line audit against `docs/CODE_ANALYSIS_2026-06-16.md` Appendix A would confirm 100% (vs the ~80% the analysis itself predicted as "addressed"). Worth a verification pass before the v9.0.0 PR description claims "all 80".

## Quick Start for Next Session

```bash
# No beads. References: docs/CODE_ANALYSIS_2026-06-16.md, plans/code-analysis-2026-06-16-progress.md, CLAUDE.md (Gate6).
cd /Users/jkl/Projects/skeleton-engine
git -C . log --oneline -11               # expect f4894ea (v9.0.0) … down to 42de46c
git status -s                            # expect clean

# ⚠️ FIRST: reconcile the concurrent session. e96529d is NOT this session's commit.
git show -s --format='%an %ci %s' e96529d
git branch -a                            # is the other session on a different branch?
git log --oneline 42de46c..HEAD          # the 10 unpushed commits to review

# Verify the batch is green (THIS branch)
cargo test --lib                         # 698 pass
./scripts/verify.sh                      # full Gate6 (fmt, clippy --all-targets, wasm build, test --all-targets, doc)

# Key files to read first
#   docs/CHANGELOG.md (9.0.0 entry = the human record of the whole batch)
#   plans/code-analysis-2026-06-16-progress.md (WU plan + prior-session RESUME note; checkboxes NOT updated)
#   src/app/{schedule.rs (panic frame-abort), render.rs (step_frame_once + owned views), window.rs (Focused/double-step)}
#   src/network.rs (NetworkSystem::new), src/prefab.rs (component_names_for + has_component)

# Next action: reconcile the two sessions, then (user's call) push + merge v9.0.0 to main.
```
