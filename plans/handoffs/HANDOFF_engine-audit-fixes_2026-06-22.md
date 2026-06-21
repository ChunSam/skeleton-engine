# HANDOFF — Engine-wide audit + priority fixes

**Chain:** standalone-4365aa4a
**Seq:** 1 (new chain — not a continuation)
**Parent:** none
**Date:** 2026-06-22
**Branch:** main
**Status:** Implementation COMPLETE + verified; **UNCOMMITTED** working tree (user asked for /handoff + report, not commit/ship)
**Auto:** false
**Package:** skeleton-engine v0.47.1 (no version bump done — see "Shipping" below)

---

## Goal

User request (Korean): *"엔진 전체 코드 점검. 하드코딩 된 부분 있는지, 수정 해야하는 코드나 구조 있는지 확인"* — audit the whole engine for hardcoded values and code/structure that needs fixing. Then (via `/goal`): *"권장 처리 순서대로 작업 계획 세워서 진행. 전체 작업 완료 후 /handoff 하고 완료보고 진행"* — work the recommended priority order to completion, then handoff + report.

Two-part session: (1) a 7-subagent parallel audit of `src/` (195 files, 57.5k LOC), then (2) implementing the resulting fixes in the priority order I recommended (Tier 1 → 4), deferring the larger/breaking Tier 4/5 refactors.

---

## Where We Are

All planned fix phases (1–4) are implemented and pass the full CI-equivalent gate locally:

```
./scripts/verify.sh  →  VERIFY_EXIT=0   (fmt --check + clippy --all-targets -D warnings
                                          + wasm32 lib/bins build + test --all-targets
                                          + RUSTDOCFLAGS=-D warnings cargo doc --no-deps)
```

- **29 files changed, +378 / −39.** ~8 new regression tests, all green (full lib suite 900→908+ tests pass).
- Working tree is **dirty on `main`** — nothing committed. Per the project git rule (never commit to protected default branch; branch + PR), the next session should branch before committing. `/ship` (version/CHANGELOG) and merge are NOT done.

### Baseline observations from the audit (context for future work)
- Codebase is clean: exactly **1** TODO marker in all of `src/` (`ui/system/button_pass.rs:26`, a real layered-UI pointer-consumption gap — left as-is, documented).
- `src/ecs/world.rs` has **56 non-test `unwrap()`** (inline test mod is only the last line `mod tests;`) — all real impl code, relies on internal invariants. Not touched this pass (high blast radius); flagged for a future dedicated review.
- `[profile.release] panic = "abort"` — any unwrap/expect that fires in a release game **aborts the whole process** (no unwind). This is WHY the unwrap/index-guard fixes below matter; it was the weighting lens for the whole audit.

---

## What We Did (by phase, with file:line)

### Phase 1 — Tier 1 logic bugs (6), each with a regression test
1. `src/tween.rs:307` `TweenSequence::tick` — a **looping** sequence whose segments are *all* zero-duration consumed no `dt` per boundary → infinite loop (the `dt <= 0.0` exit never reached). Added a `zero_crossings` counter; any positive-duration segment resets it, so legit multi-loop fast-forward still works; a full zero-duration ring breaks out. Test `looping_zero_duration_does_not_hang`.
2. `src/animation/state_machine.rs:402` `evaluate` — a transition with **empty `conditions`** auto-fired every frame (`[].iter().all() == true`). Now `conditions.is_empty()` → `continue` (skip). Verified safe: the editor's "add transition" button (`state_machine_panel.rs:468`) creates condition-less placeholders, so inert-until-authored is the correct semantics; no test/example relied on the old behavior. Test `empty_conditions_never_fire`.
3. `src/skeletal.rs:144` `SkeletalAnimator::play` — OOB clip index silently froze the animator. Added bounds guard + `log::warn!` mirroring the existing `AnimationPlayer::play`. Test `play_out_of_bounds_is_ignored`.
4. `src/pathfinding.rs:233` `find_path` (cardinal) used plain `g_current + 1` while `find_path_diagonal` used `saturating_add`. Changed to `saturating_add(1)` for parity (overflow unreachable in practice; removes the maintenance trap).
5. `src/steering.rs:295` Wander zeroed `timer`, discarding leftover dt → an extra full-interval gap on slow frames. Now `timer -= change_interval` (carry), guarded with `if change_interval > 0.0 { ... } else { timer = 0.0 }` against a zero interval.
6. `src/hierarchy.rs:54` `attach(child, parent)` allowed `child == parent` (corrupt `Parent(self)`+`Children([self])`). Added an early-return guard + warn. Test `self_attach_is_ignored`.

### Phase 2 — Tier 2 drift guards (Rust↔WGSL + hot-path panics)
- **size-assertion tests** (only `LightingUniforms` had one before): `GpuParticle == 80` (`renderer/gpu_particle.rs` tests mod), `InstanceRaw == 116` + `UiInstanceRaw == 112` (`renderer/sprite/geometry.rs` tests mod). A field add/reorder that changes the stride now fails the build instead of silently corrupting rendering.
- `src/renderer/gpu_particle.rs` — new `const COMPUTE_WORKGROUP_SIZE: u32 = 64` is the single source: it drives the `div_ceil` dispatch AND is substituted into the WGSL `@workgroup_size(64)` at shader-load via `.replace(...)`. WGSL keeps `64` as a valid standalone fallback (comment added in `shaders/gpu_particle_compute.wgsl`).
- `src/renderer/lighting.rs` — `MAX_LIGHTS` const now drives (a) the Rust `LightingUniforms.lights: [GpuLightData; MAX_LIGHTS]`, and (b) the inline WGSL `array<GpuLight, MAX_LIGHTS>` — the WGSL carries a `MAX_LIGHTS` token (invalid alone) replaced with the number at shader-load (`LIGHTING_SHADER.replace("MAX_LIGHTS", ...)`). Also `cached_bind_group.as_ref().unwrap()` → `.expect("...set in the branch above")` (provably safe; just documents intent).
- `src/renderer/sprite/draw.rs:92,97` — panicking `custom_pipelines[hash]` / `params_buffers[entity]` HashMap indexing in the Material draw arm → guarded `.get()` with `let (Some, Some) = (...) else { debug_assert!(false, ...); continue; }`. Moved the `i += 1` to the top of the arm so `continue` is correct.

### Phase 3 — Tier 3 fork-friendliness (ALL additive / non-breaking)
**Key constraint discovered:** `WindowConfig` is built by **full struct literal in ~70 example files** (only 1 uses `..Default::default()`), so adding a field there would be a 70-site breaking change → AVOIDED. Used optional resources / new methods instead.
- `src/lib.rs` — re-exported `ScriptingLimits` (was unreachable from crate root; `ScriptingSystem::with_limits(ScriptingLimits)` already exists, so tuning Rhai op/memory/depth limits is now possible without `use engine::scripting::...`).
- `src/resources.rs` — `pub const DEFAULT_CANVAS_ID: &str = "game-canvas"` (re-exported via lib). Replaced 4 hardcoded `"game-canvas"` lookups: `app/window.rs` ×3 + `renderer/context.rs` ×1. One edit point for forks embedding under a different id.
- `src/gpu_particle.rs` — new optional `GpuParticleConfig { capacity: u32 }` resource (`Default` = 4096). `app/render/frame.rs` reads `world.resource::<GpuParticleConfig>().copied().unwrap_or_default().capacity` instead of the hardcoded `4096`. Insert the resource pre-first-frame to override.
- `src/physics/character.rs` — `CharacterController.drop_duration: f32` pub field + `with_drop_duration` builder; `request_drop` now uses `self.drop_duration` (was `Self::DROP_DURATION`). `DROP_DURATION` const promoted `pub(crate)` → `pub` (documents default, intra-doc-linked). Safe: the struct has private fields so all construction goes through `new()/default()/builders`.
- `src/physics/world.rs` — `set_collider_friction` / `set_collider_restitution` convenience methods (mirror `set_collision_groups`; `get_collider_mut` raw escape hatch already existed). `src/physics/mod.rs` — `pub const DEFAULT_FRICTION = 0.3` / `DEFAULT_RESTITUTION = 0.0`; `body_factory.rs` uses them (4 literals removed).
- `src/physics/world/joints.rs` — new `add_spring_joint(.., stiffness, damping)`; `add_distance_joint` now delegates with named `DISTANCE_JOINT_STIFFNESS = 1000.0` / `DISTANCE_JOINT_DAMPING = 10.0` consts (was bare `SpringJointBuilder::new(rest, 1000.0, 10.0)`).
- Tests added: `set_collider_friction_and_restitution_update_collider`, `character_controller_drop_duration_is_configurable`, `add_spring_joint_creates_with_custom_constants`.

### Phase 4 — Tier 4 cheap real bugs
- `src/dialogue/tree.rs:261` `DialogueRegistry::reload_path` — loaded via the **caller's** `path` string, which can canonicalize-equal the registered path but differ as a string (trailing slash, rel vs abs) and fail to open. Now captures and loads from the **stored** registered path (matches `ParticleConfigRegistry`).
- `src/input/gamepad_macos.rs` — added file-level `#![cfg(target_os = "macos")]` self-contained gate. CI is ubuntu-only so this file is never compiled there; the inner attr prevents accidental Linux/CI compilation if an include path is ever added.
- `src/tilemap/system.rs:147` — `world.get::<Tilemap>(map_entity).unwrap()` (entity collected earlier, could be despawned mid-frame by a script/coroutine) → `let Some(tm) = ... else { continue; }`.
- `src/audio/ducking.rs:214` + `src/audio/playback.rs:204` — collect-then-mutate `get_mut(key).unwrap()` → `let Some(...) else { continue; }`. Behavior-preserving in the normal (key-present) case; robust against future refactors that remove an entry mid-loop.

---

## Key Decisions & Rejected Alternatives

- **No workflow tool used.** The user did not opt into multi-agent orchestration; the audit used 7 plain `Agent` subagents (sonnet, explicit model per the new-model-subagent-incompat rule), one per subsystem group. That was within the normal subagent budget.
- **Scope cut at Tier 1–4 + cheap Tier-4 bugs.** Tier 4/5 large refactors were deliberately deferred (see below) — they carry real regression risk across 61 example "acceptance tests" and belong in the feature+example loop or a dedicated `/split-module`, not a quick audit-fix pass. Stated to the user; they did not object.
- **`WindowConfig` field NOT added** for GPU particle capacity → used an optional resource instead (70 full-literal call sites would break). This is the single most important non-obvious constraint for anyone extending `WindowConfig`.
- **`material.rs` `DefaultHasher`** finding (flagged by the renderer auditor) was DOWNGRADED and NOT changed: it's a runtime-only pipeline cache (never persisted), so "unstable across Rust versions" is a non-issue, and collision probability for distinct WGSL sources is astronomically low.
- **`lighting.rs:470` unwrap** was provably safe (the `if is_none()` branch always sets it); changed to `.expect()` for documentation only, not restructured (restructuring = risk for zero gain).
- **`particle/mod.rs:317` unwrap** (flagged HIGH by an auditor) was judged a false-positive for actual panic (no despawn happens inside that loop) and left alone.
- Fixes that touched **public API were kept strictly additive** — no example file needed editing, which kept the diff to `src/` only and the gate green.

---

## Evidence & Verification

- Full gate: `./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?` → `0`. (Read the exit code, never piped — per project rule.)
- Targeted runs during work: `cargo test --lib` (900 pass pre-Phase-3 additions), `cargo test --lib renderer::` (size asserts green: `gpu_particle_size_is_stable`, `instance_raw_size_is_stable`, `ui_instance_raw_size_is_stable`), `cargo test --lib physics::` (47 pass incl. 3 new).
- `cargo fmt` auto-reformatted two long assert lines (`skeletal.rs`, `physics/world/tests.rs`) — expected, included in the diff.
- Diffstat: 29 files, +378/−39 (see `git diff --stat`).

---

## Where We're Going — DEFERRED follow-ups (priority order)

These were flagged in the audit but NOT implemented. Each is a self-contained next task:

1. **Registry/hot-reload boilerplate → generic `RonRegistry<V>`.** `ParticleConfigRegistry` / `DialogueRegistry` / `AnimationClipRegistry` / `DataTableRegistry` each re-implement `HashMap<name,V>` + `HashMap<name,path>` + `load` + canonical-path `reload_path` + `impl HotReloadable`. ~80% is copy-paste; the divergence already caused the `dialogue reload_path` bug fixed this session. A blanket `HotReloadable` would prevent future drift. (Behavior-preserving refactor → `/split-module`-style.)
2. **`ecs/schedule.rs` Kahn topo-sort is O(n²)** (`available.iter().min()` + `retain` + `order.contains`). Fine for small system counts; replace ready-set with `BinaryHeap<Reverse<usize>>` + a `HashSet` for the membership check. Watch determinism: current `min()` makes order deterministic — preserve that (min-heap does).
3. **God-files:** `app/editor/ui/docked.rs` (1233 lines), `gizmo.rs` (1183). Extract `particle_tuner_grid` / `point_light_grid` into dedicated files — the pattern is already established (`audio_panel.rs`, `data_table_panel.rs`). `/split-module`.
4. **Editor theming constants module.** Pervasive inline magic numbers across `docked.rs`/`gizmo.rs`/`mod.rs`/`slider.rs`/`checkbox.rs`: panel sizes, gizmo handle sizes, Z-offsets (`999`/`1000`), colors, font sizes, drag ranges. No central place to retheme.
5. **`audio.rs` (native) vs `audio_wasm.rs` shared `AudioSurface` trait** — different feature surfaces force cfg-guards at every cross-platform call site. Define a minimal common trait (`play`/`stop`/`set_volume`/bus ops).
6. **`renderer/texture.rs:130` `Rgba8UnormSrgb` hardcoded** — blocks HDR/linear sprite workflows. Parameterize the format (breaking to `from_rgba`; add a `from_rgba_with_format` + keep `from_rgba` as the srgb wrapper). Best as a feature+example task.
7. **Tier 5 remainder:** `SpatialGrid::candidates_in_aabb` allocates a `HashSet` per query → scratch buffer (SteeringSystem pattern); `pathfinding` reconstruct-path loop dup → `reconstruct_path` helper; many named-constant extractions; editor asset-browser renders a `"[ ]"` stub (no thumbnails) — below the bar of other panels.
8. **`ecs/world.rs` 56 real unwraps** — dedicated invariant-hardening review (high blast radius; left untouched this pass).

---

## Open Questions

- **Ship or not?** Changes are uncommitted on `main`. Does the user want these landed as a PR (`/land-pr` → branch + `/ship` version bump + CHANGELOG + squash-merge; merge authority is standing-delegated)? If yes: this is a `fix:` + `feat:` mix; suggest a MINOR bump (new public API: `GpuParticleConfig`, `ScriptingLimits` re-export, `DEFAULT_CANVAS_ID`, `set_collider_friction/restitution`, `add_spring_joint`, `CharacterController::with_drop_duration`, `DEFAULT_FRICTION/RESTITUTION`). Pre-1.0 MINOR = any release.
- How far into the deferred list does the user want to go next? (Items 1–2 are the highest value/lowest risk.)

---

## New public API surface (for CHANGELOG when shipping)

- `engine::ScriptingLimits` (re-export)
- `engine::DEFAULT_CANVAS_ID: &str`
- `engine::GpuParticleConfig { capacity: u32 }` (optional resource; native-only)
- `engine::physics::DEFAULT_FRICTION` / `DEFAULT_RESTITUTION: f32`
- `PhysicsWorld::set_collider_friction(ColliderHandle, f32) -> bool`
- `PhysicsWorld::set_collider_restitution(ColliderHandle, f32) -> bool`
- `PhysicsWorld::add_spring_joint(b1, b2, anchor1, anchor2, rest_length, stiffness, damping) -> JointHandle`
- `CharacterController.drop_duration: f32` + `with_drop_duration(f32)`; `CharacterController::DROP_DURATION` now `pub`
- Internal: `COMPUTE_WORKGROUP_SIZE` (renderer/gpu_particle.rs), `DISTANCE_JOINT_STIFFNESS/DAMPING` (joints.rs) — module-private consts.

---

## Pointers

- Full per-tier audit report (every finding with file:line + severity) is in the conversation history (the 7 subagent results + the consolidated Korean report).
- Module map / verification rules: `CLAUDE.md`. Patterns: `docs/PATTERNS.md`. Dev history: `docs/HANDOFF.md`.
- Verify: `./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?` (authoritative; never pipe the gate).

---

## Audit methodology (how the findings were produced)

7 parallel `Agent` subagents (sonnet, explicit model), each scoped to a non-overlapping subsystem group, each given the same rubric: (A) hardcoded values that should be configurable, (B) code that needs fixing (panics/swallowed errors/dup/footguns), (C) structural issues. Each returned `file:line — SEVERITY — CATEGORY — problem — fix`, grouped by severity, plus a themes summary.

| Subagent scope | Notable yield |
|---|---|
| Core ECS + App (`app.rs`, `app/` ex-editor, `ecs/`, `scene.rs`, `resources.rs`, `hierarchy.rs`, `pool.rs`) | hierarchy self-cycle; Kahn O(n²); world.rs invariant unwraps; crash.log path |
| Renderer (`renderer/`, `material.rs`, `nine_slice.rs`, `gpu_particle.rs`, `atlas.rs`, `color.rs`) | size-assert gaps; workgroup/MAX_LIGHTS drift; draw.rs index panics; texture format |
| Physics/collision/AI (`physics/`, `collision/`, `pathfinding.rs`, `steering.rs`, `behavior.rs`) | hardcoded materials/spring; find_path overflow parity; wander dt; SpatialGrid alloc |
| Animation/time (`animation/`, `skeletal.rs`, `tween.rs`, `timer.rs`, `timeline.rs`, `coroutine.rs`) | tween infinite loop; empty SM conditions; skeletal play bounds |
| Audio/save/net/script (`audio*`, `save*`, `network*`, `scripting*`) | ducking/playback unwraps; ScriptingLimits unreachable; net duplication |
| Editor + UI (`app/editor/`, `debug_ui.rs`, `ui/`) | data_table/state_machine panel `.expect`; pervasive layout magic numbers; asset-browser stub |
| Assets/tilemap/data (`asset*`, `tilemap/`, `particle/`, `dialogue/`, `data_table.rs`, `prefab*`, `reflect.rs`, `input/`, `camera.rs`, `parallax.rs`, `locale.rs`) | registry boilerplate; dialogue reload_path bug; gamepad_macos cfg gate; tilemap unwrap |

Cross-cutting greps I ran directly (complementary, not delegated): TODO/FIXME census (1 total), per-file unwrap/expect/panic counts, inline-`#[cfg(test)]` map (to discount test unwraps), `Cargo.toml` review (found `panic = "abort"`).

## Files changed, by phase

- **P1 (logic):** `tween.rs`, `animation/state_machine.rs` (+ `state_machine/tests.rs`), `skeletal.rs`, `pathfinding.rs`, `steering.rs`, `hierarchy.rs`.
- **P2 (drift):** `renderer/gpu_particle.rs`, `renderer/sprite/geometry.rs`, `renderer/sprite/draw.rs`, `renderer/lighting.rs`, `renderer/shaders/gpu_particle_compute.wgsl`.
- **P3 (fork-friendliness):** `lib.rs`, `resources.rs`, `app/window.rs`, `renderer/context.rs`, `gpu_particle.rs`, `app/render/frame.rs`, `physics/character.rs`, `physics/world.rs`, `physics/mod.rs`, `physics/world/body_factory.rs`, `physics/world/joints.rs` (+ `physics/world/tests.rs`).
- **P4 (cheap bugs):** `dialogue/tree.rs`, `input/gamepad_macos.rs`, `tilemap/system.rs`, `audio/ducking.rs`, `audio/playback.rs`.

## How to resume

1. **If shipping:** branch off `main` (don't commit to it). `git checkout -b fix/audit-tier1-4`. Then `/ship` for the MINOR bump + CHANGELOG (use the "New public API surface" list above), commit, PR, watch CI, squash-merge, bump the `engine-current-state` memory seq.
2. **If continuing the deferred list:** start with item 1 (registry generic) or item 2 (scheduler) — both are behavior-preserving and `/split-module`-shaped (tests stay green, ideally untouched). Item 2 caution: preserve deterministic ready-set ordering (min-heap).
3. **Sanity before any further edit:** `./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?` should already be `0` on the current dirty tree.
