# Cohesion-review follow-ups — v9.4.0 safe-additive batch (PR #67)

**Date:** 2026-06-16
**Status:** COMPLETED
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine — engine hardening / architecture
**Chain:** `engine-hardening` seq `5`
**Parent:** `HANDOFF_engine-hardening_module-cohesion-review_2026-06-16.md` (seq 4)
**Prior chain:** `..._v9.0.0-shipped_` (1) > `..._v9.0.0-merged_` (2) > `..._post-v9-priority-loop_` (3) > `..._module-cohesion-review_` (4) > this (5)

---

## Since Last Handoff

Seq 4 delivered the module-cohesion review (`docs/MODULE_COHESION_REVIEW_2026-06-16.md`, PR #66)
with a 7-item prioritized action list (1–4 safe-additive, 5–7 breaking→v10). The user re-ran
`/loop` ("재실행") → greenlight to execute the action list. This seq shipped the **safe-additive
subset (items 1, 2, 4)** as **PR #67 / v9.4.0**. Stopped at the line where remaining work becomes
lower-value reorg (item 3) or breaking/architectural (items 5–7).

## Reference Documents

- `docs/MODULE_COHESION_REVIEW_2026-06-16.md` — the review + the full prioritized action list (read for items 3, 5–7).
- `CLAUDE.md` (module map, Gate6) · `docs/VISION.md`.

## The Goal

Execute the cohesion review's **safe, non-breaking, behavior-identical** action items so the
findings turn into shipped improvements, without pre-empting the breaking/architectural decisions
(which need a v10 design pass + user sign-off).

## Where We Are

- **`main` = `ca5a636` (PR #67), v9.4.0, CLAUDE.md header v1.6.37, clean tree.** 721 lib tests (716 → +5). CI 4/4 green; full local Gate6 green.
- **Shipped in #67 (all behavior-identical or purely additive):**
  - **Perf (audit #72):** `SpriteRenderer` reuses `atlas_entries` + `live_material_entities` + `seen_new_hashes` as scratch fields (cleared per `render()`) instead of per-frame allocation.
  - **DRY:** `Tilemap::compute_tile_mask`/`compute_tile_mask_typed` delegate to one `compute_mask_raw(filled: impl Fn(i32,i32)->bool)` — ~50 lines of copy-pasted Blob8 logic removed, bit-identical; `AssetLoadError` shared enum, `ClipSetError`/`ParticleConfigError` now `pub type` aliases (public + variant names unchanged → non-breaking).
  - **Additive API:** `JointHandle::raw()` + `pub(crate) from_raw()` (matches `BodyHandle`/`ColliderHandle`); `engine::AssetLoadError` re-exported (the one `src/lib.rs` edit, done by the supervisor).
  - **Docs:** `src/animation` module-level "System registration order" doc; `register_editable_component` native-only note.
- **Deliberately SKIPPED (reported by the agent, confirmed correct):** `UiSystem`/`SteeringSystem` scratch fields (both UNIT structs → adding fields breaks `add_system(UiSystem)` = semver-breaking); `state_machine evaluate()` return-type refactor (fiddly borrow); `tilemap` cached-grid restructure, `collision` candidates scratch (`&self`), `query_added/changed` index inversion (structural); `body_factory remove_body` `.to_vec()` (borrow-fight); module-home moves + `Wander::direction_for` (item 3 / may break literals).
- **One cosmetic note:** `ParticleConfigError`'s Display string changed `"RON parse error:"`→`"RON error:"` (unified with `ClipSetError`); error strings aren't a semver contract.

## What We Tried (Chronological)

1. **Greenlight + scope-check.** User re-looped after the cohesion review. Verified which Theme-2 perf sites are non-breaking: `SpriteRenderer` (internal struct, `&mut self`) = safe; `UiSystem`/`SteeringSystem` = unit structs → breaking → skip; `evaluate()` private but fiddly.
2. **One Sonnet agent** for the combined safe-additive batch (items 1 perf + 2 DRY + 4 API/docs), with an explicit "SKIP + report if breaking/risky" rule and behavior-equivalence test requirement.
3. **Agent returned clean** (721 tests, clippy --all-targets clean, wasm pass, fmt clean); skipped exactly the breaking/risky items as instructed.
4. **Supervisor:** added the `AssetLoadError` re-export to `src/lib.rs`, ran full Gate6 (green), bumped v9.4.0 + CHANGELOG + CLAUDE header, PR #67, watched CI, squash-merged (ff-only sync — no divergence this time).

## Key Decisions

- **Shipped only the provably-safe subset; stopped at the reorg/breaking line.** Items 1/2/4 are clearly-valuable + Gate6-verifiable. Item 3 (module moves, panel extraction) is large mechanical reorg with lower product value; items 5–7 are breaking/architectural. Both warrant user engagement with the review rather than auto-grinding.
- **`UiSystem`/`SteeringSystem` scratch = breaking, deferred.** Both are unit structs (`pub struct X;`); adding fields breaks `add_system(X)`. The same reason #76 was flagged. → v10 (or a `Default`-based deprecation).
- **Type aliases keep `ClipSetError`/`ParticleConfigError` non-breaking** while sharing one impl — public names + variant names preserved, so user `match` arms still compile.
- **Pushed the seq-5 handoff straight to main** (learned from the seq-4 gotcha where an unpushed local-main handoff commit got bundled into a docs PR's squash).

## Evidence & Data

| Item | Status | Where |
|---|---|---|
| 1 Perf (SpriteRenderer scratch) | DONE | `src/renderer/sprite.rs` (#72) |
| 2 DRY (compute_mask_raw) | DONE | `src/tilemap.rs` |
| 2 DRY (AssetLoadError) | DONE | `src/asset.rs` + clip_set/config_set aliases |
| 4 API (JointHandle::raw) | DONE | `src/physics/world.rs` |
| 4 Docs (anim order, editable wasm note) | DONE | `src/animation/mod.rs`, `src/app/editor.rs` |
| 1 (UiSystem/Steering scratch) | SKIP (breaking) | unit structs |
| 3 module moves / 5–7 architectural | NOT STARTED | → next |

Tests: 716 → **721** (+5: scratch-reuse equivalence, 3× `compute_mask_raw` equivalence/reference, `JointHandle::raw` round-trip). New public symbols: `AssetLoadError`, `JointHandle::raw`.

PRs this session (full arc): #61 (v9.1) → #62 (v9.2) → #63 (v9.3) → #64 (coverage ledger) → #65 (REFERENCE) → #66 (cohesion review) → #67 (v9.4 cohesion follow-ups). main `ca5a636`.

## Files Changed (PR #67)

`src/renderer/sprite.rs` (+tests.rs), `src/tilemap.rs`, `src/asset.rs`, `src/animation/clip_set.rs`, `src/particle/config_set.rs`, `src/physics/world.rs` (+world/joints.rs test), `src/animation/mod.rs`, `src/app/editor.rs`, `src/lib.rs` (re-export), `Cargo.toml` + `docs/CHANGELOG.md` + `CLAUDE.md` (version).

## User Feedback & Preferences (REQUIRED)

- **"재실행"** — continue the loop = execute the cohesion review's action list autonomously.
- **Earlier this session:** chose the cohesion review over shipping marginal work; Korean prose / English artifacts; subagents on Sonnet w/ explicit `model:`; merge authority for the loop; never tag unprompted.

## Where We're Going

The clearly-valuable safe-additive subset is shipped. Remaining (none blocking), user picks:
1. **Cohesion item 3 — module-home moves** (additive-via-re-export, Gate6-verifiable but invasive): `SerdeComponentRegistry`→`serde_registry.rs`; `CameraUniform` dedup (trivial); `ScriptAsset`→decouple `asset.rs` from rhai (**design change**, not a plain move — AssetServer storage needs rethinking); extract SM/Timeline panels from the 2003-line `docked.rs`→`ui/state_machine_panel.rs`+`ui/timeline_panel.rs` (mechanical but large — pairs with the editor visual-editor work). Lower product value (no behavior change) → confirm the user wants the reorg.
2. **v10 architecture pass (cohesion items 5–7) — breaking, needs sign-off:** `RenderState` extraction from `App`; split `render()`(839)/`update()`(386); `SpriteRenderer`→`MaterialRenderer`+`TextureCache`; split `tilemap.rs`; `pub(crate)` the wgpu/rapier public fields + accessors; new `RenderPlugin` + `register_inspector_panel` extension hooks; `UiSystem`/`SteeringSystem` scratch.
3. **Deferred features (unchanged):** per-locale fonts (#5, display session — spec in seq 3); SM/Timeline visual editors (display session); v9.x git tag (policy).

## Risks & Blockers

- **None blocking.** main green + clean at `ca5a636`.
- The skipped `ParticleConfigError` Display-string change is cosmetic (noted); no public-API break.

## Open Questions

- Do item 3 module moves now (autonomous, low-risk-but-low-value reorg), or fold into the v10 pass?
- Schedule the v10 architecture pass (items 5–7), or leave the cohesion review as a roadmap?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4   # ca5a636 (#67) → 64286b6 (seq-4 handoff) → c6804e7 (#66) → ...
grep -m1 '^version' Cargo.toml   # 9.4.0
git status -s          # clean

# Read: docs/MODULE_COHESION_REVIEW_2026-06-16.md (action list) + this handoff (seq 5) + parent (seq 4)
./scripts/verify.sh    # confirm main green

# Next action — pick ONE:
#   (a) Cohesion item 3 (module moves) — start with the trivially-safe ones: SerdeComponentRegistry
#       -> serde_registry.rs (re-export), CameraUniform dedup. (ScriptAsset decouple = design change.)
#   (b) Scope a v10 architecture pass (items 5-7, breaking) with the user.
#   (c) A deferred feature needing a display session (per-locale fonts / visual editors).
#   For code work: implement -> Gate6 -> unit-test through real handler -> PR -> merge
#   (re-confirm merge authority in a NEW session).
```

## Session Closed
**Closed at:** 2026-06-16
**Commit:** the `session: cohesion-followups-v9.4 handoff [engine-hardening]` commit on `main`
**Session status:** Handed off — v9.4.0 shipped; next = cohesion item 3 (module moves) or a v10 architecture pass
