# Module-home reorganization — v9.4.1 (PR #68); cohesion safe-subset exhausted

**Date:** 2026-06-16
**Status:** COMPLETED
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine — engine hardening / architecture
**Chain:** `engine-hardening` seq `6`
**Parent:** `HANDOFF_engine-hardening_cohesion-followups-v9.4_2026-06-16.md` (seq 5)
**Prior chain:** seq 1 > 2 > 3 (priority loop) > 4 (cohesion review) > 5 (v9.4.0 follow-ups) > this (6)

---

## Since Last Handoff

Seq 5 shipped v9.4.0 (cohesion review items 1/2/4) and named **item 3 (module-home moves)** as the
next autonomous batch. The user re-ran `/loop` ("재실행") → greenlight. This seq shipped item 3's
**pure-relocation subset** as **PR #68 / v9.4.1**. With it, **the cohesion review's entire SAFE
autonomous subset (items 1–4) is now exhausted** — everything left is breaking/architectural (→ v10,
needs user sign-off) or a design change.

## Reference Documents

- `docs/MODULE_COHESION_REVIEW_2026-06-16.md` — the review + action list (items 5–7 remain).
- `CLAUDE.md` (module map — updated for the moves) · `docs/VISION.md`.

## The Goal

Execute item 3's mechanical module-home moves (pure relocation, zero behavior change, public API
preserved) to split the two editor god-files + decouple the serde registry — improving cohesion +
fork-friendliness without any breaking change.

## Where We Are

- **`main` = `b6106f9` (PR #68), v9.4.1, CLAUDE.md header v1.6.38, clean tree.** 721 lib tests (unchanged — moves preserve tests). CI 4/4 green; full local Gate6 green (independently re-verified).
- **Shipped in #68 (pure relocation, no API change):**
  - `SerdeComponentRegistry` + `SerdeComponentEntry` → new `src/serde_registry.rs`, re-exported from `prefab.rs` (so `engine::SerdeComponentRegistry` and `crate::prefab::SerdeComponentRegistry` both still resolve). `lib.rs` gained `pub mod serde_registry;`.
  - State-Machine panel (+ `cond_summary`/`param_display`) → `src/app/editor/ui/state_machine_panel.rs`; Timeline panel (+ `timeline_track_ui`/`easing_variants`) → `src/app/editor/ui/timeline_panel.rs`. **`docked.rs` 2003 → 1259 lines.**
  - Tile-paint methods + free fns + tests → `src/app/editor/ui/tile_paint.rs`. **`gizmo.rs` 1869 → 1183 lines.**
- **Skipped (reported):** `CameraUniform` dedup — the two defs (`gpu_particle.rs` private vs `sprite/geometry.rs` `pub(super)`) differ in field visibility; the agent conservatively skipped rather than reconcile. Trivial to finish later (unify to `pub(crate)`).
- **Verification caught a false "green" claim:** the agent reported all gates pass, but the harness diagnostics showed rustc E0624/E0425 (visibility) errors. I **re-ran the full Gate6 myself** → genuinely green (the errors were mid-edit snapshots the agent resolved before finishing). Lesson: verify the build independently after large mechanical moves; don't trust the agent's "green" alone.

## Key Decisions

- **Pure relocation only; preserve every public path via re-export.** No logic change ⇒ Gate6 (compile + tests + clippy + wasm + doc) is a complete safety net for moves.
- **Stopped after item 3 — the safe autonomous subset is done.** Items 5–7 are breaking/architectural (semver) and need a v10 design decision + user sign-off; `ScriptAsset` decouple is a design change (AssetServer storage), not a move. Continuing autonomously would mean breaking changes without the user's call.
- **Independent Gate6 re-verification** (not trusting the agent's claim) — the diagnostics flagged real-looking errors; confirming green myself was the correct gate before merge.

## Evidence & Data

| Move | Status | Result |
|---|---|---|
| A `CameraUniform` dedup | SKIPPED | defs differ in field visibility (trivial to finish later) |
| B `SerdeComponentRegistry` → `serde_registry.rs` | DONE | re-exported from prefab; `engine::` path unchanged |
| C SM panel → `state_machine_panel.rs` | DONE | docked.rs −≈434 lines |
| D Timeline panel → `timeline_panel.rs` | DONE | docked.rs total 2003 → 1259 |
| E tile-paint → `tile_paint.rs` | DONE | gizmo.rs 1869 → 1183 |

Tests unchanged at **721** (moves preserve tests). Diff: docked.rs −748, gizmo.rs −686, prefab.rs −186 (relocated); 4 new files; lib.rs/ui-mod.rs `mod` decls.

Session arc (8 PRs): #61(v9.1) → #62(v9.2) → #63(v9.3) → #64(coverage) → #65(REFERENCE) → #66(cohesion review) → #67(v9.4.0) → #68(v9.4.1). main `b6106f9`.

## Files Changed (PR #68)

New: `src/serde_registry.rs`, `src/app/editor/ui/{state_machine_panel,timeline_panel,tile_paint}.rs`. Modified: `src/prefab.rs` (re-export), `src/lib.rs` (`pub mod serde_registry;`), `src/app/editor/ui/{docked,gizmo,mod}.rs`, `Cargo.toml`/`docs/CHANGELOG.md`/`CLAUDE.md` (version + module-map row).

## User Feedback & Preferences (REQUIRED)

- **"재실행" (×3 this session)** — keep executing the planned backlog autonomously, batch → merge → handoff → report each time.
- **Standing:** Korean prose / English artifacts; subagents on Sonnet w/ explicit `model:`; merge authority for the loop; never tag unprompted; rust-survivors dropped.

## Where We're Going

**The safe autonomous backlog is exhausted.** Everything remaining needs a user decision:
1. **v10 architecture pass (cohesion items 5–7) — BREAKING, needs sign-off.** `RenderState` extraction from `App`; split `render()`(839)/`update()`(386); `SpriteRenderer`→`MaterialRenderer`+`TextureCache`; split `tilemap.rs`; `pub(crate)` the leaking wgpu/rapier fields + accessors; new `RenderPlugin` + `register_inspector_panel` extension hooks; `UiSystem`/`SteeringSystem` scratch (both unit structs → breaking). Scope this as a deliberate v10 design pass.
2. **`ScriptAsset` → rhai decouple** — a design change (rethink how `AssetServer` stores scripts: erased `Box<dyn Any>` or a trait); not a mechanical move.
3. **Trivial leftover:** finish the `CameraUniform` dedup (unify the two defs to `pub(crate)`).
4. **Display-session features (unchanged):** per-locale fonts (#5, spec in seq 3); SM/Timeline visual editors; v9.x git tag (policy).

## Risks & Blockers

- **None blocking.** main green + clean at `b6106f9`.
- **Process note:** after big mechanical moves, the agent's self-reported "green" disagreed with harness diagnostics — always re-run Gate6 independently before merge (done here; was genuinely green).

## Open Questions

- Greenlight a v10 breaking architecture pass (items 5–7), or leave the cohesion review as a roadmap and pivot to features (per-locale fonts / a new VISION feature)?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4   # b6106f9 (#68) → b071e01 (seq-5 handoff) → ca5a636 (#67) → ...
grep -m1 '^version' Cargo.toml   # 9.4.1
git status -s          # clean

# Read: docs/MODULE_COHESION_REVIEW_2026-06-16.md (items 5-7) + this handoff (seq 6) + seq 4/5
./scripts/verify.sh    # confirm main green

# Next action — needs a DECISION (safe autonomous backlog is exhausted):
#   (a) Scope a v10 breaking architecture pass (cohesion items 5-7) WITH the user.
#   (b) Pivot to a feature: per-locale fonts (#5, display session) or a new VISION feature+example.
#   (c) Trivial cleanup: CameraUniform dedup.
#   For breaking work: do NOT proceed without explicit user sign-off on the v10 window.
```

## Session Closed
**Closed at:** 2026-06-16
**Commit:** the `session: module-moves-v9.4.1 handoff [engine-hardening]` commit on `main`
**Session status:** Handed off — v9.4.1 shipped; cohesion safe-subset exhausted; next step (v10 breaking pass) needs user sign-off
