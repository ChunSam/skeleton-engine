# Remaining code-analysis remediation: performance + subsystem robustness + additive API polish

**Date:** 2026-06-06
**Status:** PLANNED
**Bead(s):** none (bd unavailable)
**Epic:** code-analysis remediation (`docs/CODE_ANALYSIS.md`)
**Chain:** `code-analysis-fixes` seq `2`
**Context:** See `HANDOFF_code-analysis-fixes_v3-breaking-batch_2026-06-06.md` for full session data, the per-issue backlog table (file:line + fix), commit log, gotchas, and verification commands.

---

## Problem Statement

The full-codebase analysis (`docs/CODE_ANALYSIS.md`) found 30 issues; this chain has fixed 17 across merged PRs #7 (non-breaking) and #8 (v3.0.0 breaking — Color + PhysicsWorld-resource). **13 remain**: 8 MEDIUM (#7/#8/#12/#15/#18/#19/#20/#22) and 5 LOW (#23/#27/#28/#29/#30). All are non-breaking EXCEPT #28 (rapier `ImpulseJointHandle` newtype). This plan ships the 12 non-breaking ones as a single PR off v3.0.0 `main`, grouped by theme, and explicitly defers the one breaking item. See the **Remaining Backlog** table in the handoff for each item's file:line and fix sketch.

## Key Findings

- The remaining MEDIUM cluster is **performance** (#7 Rhai AST clone, #8 render-pass-per-batch, #18 A* no closed-set, #19 per-frame SpatialGrid deep-clone) — internal, no API change, high leverage. → **drives Phase 1**.
- A second cluster is **subsystem correctness/robustness** (#15 light radius ½-size, #20 audio update not driven + no SFX cache, #22 layer-mask negative fold, #23 dup-tag silent overwrite). → **drives Phase 2**.
- The rest is **additive API polish** (#27 `MouseButton` re-export, #12 coord docs/helpers, #30 Rhai limits, #29 `ReflectValue` `#[non_exhaustive]`+`I32`). → **drives Phase 3**.
- **#11 touched several of these files** (renderer, particle, ui, reflect) — line numbers in the backlog predate it; re-grep before editing. `ReflectValue` now has a `Color` variant (from #11) — confirm current variants before #29.
- **CI pins Rust 1.88.0** (rustfmt 1.8.0) — verify ONLY with `cargo +1.88.0`; local stable differs and fails CI (handoff Gotchas).
- **#19 may mildly change the `SpatialGrid` resource type** (`SpatialGrid` → `Arc<SpatialGrid>`) — check readers; if it breaks reader call-sites, either keep it internal (Arc inside SpatialGrid's buckets) or accept the small change.

## Anti-Goals (What NOT To Do)

- **Do NOT fold #28 (JointHandle newtype) into this PR** — it's breaking; hold for a future v3.x/own batch (would force a major bump on an otherwise non-breaking PR).
- **Do NOT run `cargo fmt` with the default toolchain** — use `cargo +1.88.0 fmt`. And avoid editing `src/app/editor/ui/mod.rs` unless necessary (re-triggers ~700-line rustfmt churn).
- **Do NOT trust rust-analyzer diagnostics or subagent self-reports** — re-verify with a real `cargo +1.88.0 build --all-targets` + full gate (handoff Gotchas).
- **Do NOT use bare `arr.into()` for colors** — simba `From<[T;N]>` ambiguity (E0283); use `Color::rgba(..)`/`Color::from(..)`.

## Plan

### Phase 1: Performance (non-API)

**Goal:** Remove the highest-leverage per-frame costs without changing any public signature.

**Why this approach:** All four are internal hot-path fixes the analysis flagged; none touch the API, so they're low-risk and independently testable.

- **#19 (start here — smallest):** `src/collision/grid.rs:~197` — stop `world.insert_resource(self.grid.clone())` deep-copying buckets each frame. Wrap in `Arc<SpatialGrid>` and insert an `Arc::clone` (refcount bump). Check `world.resource::<SpatialGrid>()` readers; if the type change ripples, prefer `Arc<SpatialGrid>` resource and update readers, OR move buckets behind `Arc` inside SpatialGrid. Also make `CollisionDebugSystem` (`debug.rs:~50`) read the mirrored resource instead of rebuilding a second grid.
- **#7:** `src/scripting/execution.rs` — store `Arc<rhai::AST>` on `ScriptAsset` (clone = refcount bump, not tree-clone); give `ScriptRunner` four owned reusable buffers `clear()`ed between entities instead of 4×`Arc<Mutex>` per entity per frame.
- **#8:** `src/renderer/sprite.rs` draw loop — open ONE `wgpu` render pass for the whole pre-sorted entry stream; issue `set_pipeline`/`set_bind_group`/`set_vertex_buffer`/`draw_indexed` per run within it (was a new `begin_render_pass` per texture-run and per material).
- **#18:** `src/pathfinding.rs:~173-201` — add a visited/closed set (or skip popped nodes whose stored g-score is stale); reuse the three scratch collections across calls (store on `PathGrid` or pass a scratch struct).
- Re-grep each file first (post-#11 line drift). Add/extend a unit test per fix where behavior is assertable (A* path correctness; grid mirror identity; particle/script unaffected).

**Files:** `src/collision/grid.rs`, `src/collision/debug.rs`, `src/scripting/execution.rs`, `src/renderer/sprite.rs`, `src/pathfinding.rs`.
**Validates with:** `cargo +1.88.0 fmt --check && clippy --all-targets -- -D warnings && test --all-targets` all green; existing tests still pass; new A*/grid tests pass. Sprite output unchanged visually (render is exercised by examples).
**Rollback:** each fix is file-local; revert the offending file. #8 is the riskiest (render correctness) — revert it alone if sprites/UI render wrong.

### Phase 2: Subsystem robustness (non-API)

**Goal:** Fix correctness/quality-of-life defects in lighting, audio, layer-masking, and prefab loading.

**Why this approach:** Each is a self-contained subsystem fix the analysis flagged; bundling them keeps the PR coherent and each is independently revertable.

- **#15:** `src/renderer/lighting.rs:~97,148` — CPU computes `radius_ndc = radius*zoom/viewport_w` but the shader compares in UV space (NDC/2), so lights render ~½ size. Make CPU and shader agree on one space (`2*radius/viewport_w`); add a test asserting a known world radius → expected UV falloff. (Native-only; lighting is `#[cfg(not(wasm32))]`.)
- **#20:** `src/audio/` — register a built-in `AudioSystem` (added by the app or documented) that ticks `AudioManager::update(dt)` so fades actually progress; add a decoded-bytes cache so `play()` doesn't re-read the file from disk per shot. Validate fades + cache with a unit test on `AudioManager`.
- **#22:** `src/renderer/sprite/sort.rs:~88-94` + `src/components.rs` — `RenderLayer(i32)` masking folds negatives to bit 0, so `RenderLayer(-1)` background leaks into `layer_mask: 1<<0`. Restrict masking to a documented non-negative range (warn on negative) or bias the i32 onto distinct bits. Add a test for the OffscreenCamera layer_mask exclusion.
- **#23:** `src/prefab.rs:~301-303` — `spawn_scene_def` silently overwrites duplicate tags. `log::warn!` on duplicate insert and/or validate in `SceneDef::load`. Add a test asserting the warning path / first-wins documented behavior.

**Files:** `src/renderer/lighting.rs`, `src/audio/*`, `src/renderer/sprite/sort.rs`, `src/components.rs`, `src/prefab.rs`.
**Validates with:** full `+1.88.0` gate green; new lighting/audio/layer-mask/prefab tests pass. If feasible per VISION, a tiny example tweak demonstrating #20 audio fades.
**Rollback:** per-subsystem; revert the file. #20 is additive (new system) — safe.

### Phase 3: Additive API polish

**Goal:** Close the additive ergonomic gaps (no breaking signatures).

**Why this approach:** All additive (re-exports, new helpers, richer enums, config) — safe to land in the same non-breaking PR. #28 (the one breaking item) is explicitly excluded.

- **#27:** `src/lib.rs` — `pub use winit::event::MouseButton;`; convert examples that import `engine::ecs::…`/`engine::renderer::…` internal paths to top-level imports; English-ify their Korean doc comments; fix `examples/gpu_particles.rs:~23` `.unwrap()`.
- **#12:** add a `DrawText::centered(...)` / anchor helper (TopLeft/Center) and document the screen-vs-world convention prominently (`src/camera.rs` / `src/renderer/text.rs` docs). The #6 example fixes already corrected loading_bar/minimap.
- **#30:** `src/scripting/api.rs:~9-12` — set conservative `max_string_size`/`max_array_size`/`max_map_size`/call-depth/expr-depth on the Rhai engine, exposed via a `ScriptingLimits` config (defaults sane for trusted-local).
- **#29:** `src/reflect.rs` — mark `ReflectValue` `#[non_exhaustive]` and add an `I32` variant. **First confirm current variants** (#11 added `Color`); update the inspector match arms.
- **Note / Anti-goal reminder:** **skip #28** (JointHandle newtype, breaking) — leave a one-line `// TODO(#28, breaking)` near `src/physics/mod.rs` if helpful.

**Files:** `src/lib.rs`, several `examples/*.rs`, `src/renderer/text.rs`, `src/camera.rs`, `src/scripting/api.rs`, `src/reflect.rs`, inspector match sites.
**Validates with:** full `+1.88.0` gate (incl. `build --target wasm32-unknown-unknown` — #27/#30 are wasm-relevant); doc gate `RUSTDOCFLAGS="-D warnings" cargo +1.88.0 doc --no-deps`. Examples compile via `--all-targets`.
**Rollback:** all additive; revert the specific file. `#[non_exhaustive]` on `ReflectValue` is the only mild-breaking bit — if a forker matches it exhaustively it'd warn; acceptable, but note in the PR.

## Dependencies & Order

- Phases are largely independent and could be separate commits in one PR; **do Phase 1 → 2 → 3** for review coherence (perf → correctness → polish).
- Within Phase 1, **#19 first** (smallest, sets the Arc-resource pattern), then #7, #18, #8 (#8 last — riskiest render change).
- #29 depends on confirming the current `ReflectValue` variants (post-#11) — check before editing.

## Risks & Mitigations

- **#8 render-pass change breaks rendering** (likely if instance-buffer offsets/bind-group order are mishandled). Mitigation: keep the per-run draw calls identical, only hoist `begin_render_pass`; verify by running an example (`security_camera`/`lit_dungeon`) — revert #8 alone if wrong.
- **#19 resource-type ripple** (`SpatialGrid` → `Arc<SpatialGrid>`). Likelihood: medium. Mitigation: grep readers first; if many, keep Arc internal to SpatialGrid instead.
- **rust-analyzer/subagent false signals** (handoff Gotchas). Mitigation: real `cargo +1.88.0 build --all-targets` + full gate before every commit.
- **Line-number drift from #11.** Mitigation: re-grep each target before editing.
- **#20 audio is native-only** (`#[cfg(not(wasm32))]`) — keep gating intact; wasm build must stay green.

## Success Criteria

- **Minimum viable:** Phase 1 + 2 landed (the 8 MEDIUM + #23), full `+1.88.0` gate green, CI 4/4 on the PR, merged. 12 → ~21+ analysis issues resolved.
- **Full success:** all 12 non-breaking remaining issues (Phases 1–3) merged; #28 documented as deferred. Only the single breaking item left → 29/30 effectively addressed.
- Test count grows (new tests for A*, grid mirror, lighting radius, audio fades, layer-mask, dup-tag); 0 failures; wasm build + doc gate green.
- No regression in the 296-test baseline or the examples.

## Quick Start

```bash
cd /Users/jkl/Projects/skeleton-engine

# Restore full context
cat plans/handoffs/HANDOFF_code-analysis-fixes_v3-breaking-batch_2026-06-06.md   # backlog table + gotchas
cat docs/CODE_ANALYSIS.md                                                        # the 30-issue report

# Confirm starting state (PINNED toolchain — never default stable)
grep '^version' Cargo.toml   # 3.0.0
cargo +1.88.0 fmt --check && cargo +1.88.0 clippy --all-targets -- -D warnings && cargo +1.88.0 test --all-targets

# Key files for Phase 1 (re-grep line numbers — #11 shifted them)
#   src/collision/grid.rs + debug.rs (#19), src/scripting/execution.rs (#7),
#   src/renderer/sprite.rs (#8), src/pathfinding.rs (#18)

# Branch + first concrete action: #19 Arc<SpatialGrid>
git checkout -b fix/analysis-perf
grep -n "insert_resource(self.grid" src/collision/grid.rs   # the per-frame deep clone to replace with Arc::clone
```
