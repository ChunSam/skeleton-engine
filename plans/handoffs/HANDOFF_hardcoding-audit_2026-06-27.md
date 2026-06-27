# Hardcoding audit Tier-2: 3 PRs (one-way landing skin, solver iterations, frame dt cap)

**Date:** 2026-06-27
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `hardcoding-audit` seq `2`
**Parent:** `HANDOFF_hardcoding-audit_2026-06-26.md` (seq 1 — the Tier-1 fixes + the audit findings record this continues).
**Related handoffs:** `HANDOFF_hardcoding-audit_2026-06-26.md` (seq 1).

---

## Since Last Handoff

Seq-1 (2026-06-26) landed all four Tier-1 fixes (#248–#251, v0.68.5→v0.69.1) and committed
`docs/HARDCODING_AUDIT_2026-06-26.md` (the found→fixed record with a Tier-2/Tier-3 remainder
catalog). Between seq-1 and this session, two Tier-2 items shipped on their own: **#253**
(v0.70.0, `RenderTarget::with_filter`) and **#254** (v0.71.0, `ParticleEmitter::max_per_frame`).

This session (seq-2): the user said "핸드오프 확인하고 tier2 진행". Board was re-checked
(`../dungeon-merchant/docs/engine-wishlist.md` still ACTIVE EMPTY, EW-004 next), so this is
Tier-2-driven, not board-driven. Asked the user which Tier-2 bundle; they chose the
**physics/timing bundle (3 items, serial PRs)**.

## Reference Documents

- `docs/HARDCODING_AUDIT_2026-06-26.md` — the audit record. **Updated this session**: added a
  "Resolution log — Tier 2 (shipped)" table (#253–#257) and trimmed the still-open Tier-2 list.
- `CLAUDE.md` — module-map rows updated for all three features (physics character / PhysicsWorld /
  resources-time). Header now **v1.6.157**, package **v0.74.0**.
- `docs/CHANGELOG.md` — 0.72.0 / 0.73.0 / 0.74.0 entries.
- `~/.claude/.../memory/engine-current-state.md` — LIVE per-seq state (bump for this session).

## The Goal

Continue the hardcoding audit by landing the physics/timing Tier-2 knobs as separate
additive, default-preserving PRs, each with a VISION example. Acceptance: no behavior change
for non-opt-in callers (guarded by a default-value test), verify gate green, CI green.

## Where We Are

- **main @ `f5d5dd1`** (v0.74.0), tree clean except this handoff + the audit-doc edit (the
  wrap docs, landing as one `docs(handoff)` PR). No open PRs after the wrap merges.
- **All three Tier-2 PRs MERGED** (squash, branches deleted, CI 4/4 each):
  - **#255** (`ccfc3c1`, v0.72.0) — `CharacterController::one_way_tolerance` (+ `with_one_way_tolerance`, `DEFAULT_ONE_WAY_TOLERANCE`). Example `one_way_tolerance`. **Native-playtested** (lands on / drops through a one-way platform at PPU 24).
  - **#256** (`974232c`, v0.73.0) — `PhysicsWorld::set_solver_iterations` (+ `with_integration_params`, `integration_params()`). Example `solver_iterations`. Verified by a **deterministic behavioral test** (the screen had locked, so no screenshot).
  - **#257** (`f5d5dd1`, v0.74.0) — `FrameConfig { max_dt }` resource (+ `cap`). Example `frame_dt_cap`. Pure-function logic fully unit-tested (display asleep, no screenshot).
- Board remains **ACTIVE EMPTY** (EW-004 next).

## What We Tried (Chronological)

1. Read both handoffs + the audit doc + board. Asked the user the Tier-2 bundle; they chose physics/timing.
2. **PR1 (#255) one-way tolerance.** `const ONE_WAY_TOLERANCE = 0.05` in `character_movement.rs` → `CharacterController::one_way_tolerance` field read fresh each frame. Deterministic threshold test (`one_way_tolerance_widens_the_landing_skin`): a character started slightly penetrated below a one-way platform slips through at tol 0.05 but is caught at 0.2. Example at PPU 24 scales the skin to its PPU. Native-screenshotted (rest-on-top + drop-through). verify→ship→PR→CI 4/4→merge.
3. **PR2 (#256) solver iterations.** Added `set_solver_iterations` / `with_integration_params` / `integration_params()`. **Spent real effort finding an HONEST example regime** (see Key Decisions): plain box stacks are iteration-robust in rapier 0.22; the visible regime is loaded joints. Example = two heavy-ended hanging revolute chains (2 vs 16 iters); behavioral test `solver_iterations_stiffen_a_joint_chain` (stretch 0.84 @ 2 vs 0.013 @ 16). The dev screen locked → no screenshot; the test pins the displayed quantity. verify (1 clippy fix: `field_reassign_with_default` → struct-update syntax)→ship→PR→CI→merge.
4. **PR3 (#257) frame dt cap.** `.min(0.1)` in `step_frame` → `FrameConfig::cap`, resource auto-inserted in `core_resources`. Example `frame_dt_cap` (capped-vs-raw markers, self-hitching). Default+clamp unit tests. Display asleep → no screenshot; cap is a pure function fully tested. verify→ship→PR→CI→merge.
5. **Wrap.** Updated the audit doc Tier-2 resolution log, this handoff, memory.

## Key Decisions

- **PR2's example took empirical work to make HONEST.** I measured rapier 0.22's solver before
  writing the example. Findings: a plain N-box stack is fine at the default 4 iterations and only
  *collapses* at 1 (and very-high counts can even destabilize n=50). Locking rotation removes the
  iteration sensitivity entirely (it's elastic contact softness, not iteration-fixable). The one
  clean, monotonic, stable regime where **raising** iterations helps is **loaded joints** — a
  heavy-ended hanging revolute chain stretches 0.84 links at 2 iters vs 0.013 at 16. So the example
  is joint chains, not box stacks, and the doc says so plainly. (Do NOT "fix" this into a stack demo
  — a stack demo would be dishonest for rapier 0.22.)
- **Behavioral test thresholds are generous + commented rapier-version-sensitive.** `low > 0.4`,
  `high < 0.15`, `low > high*3` around measured 0.84 / 0.013 — absorbs minor solver drift; a rapier
  bump that breaks it correctly flags that the example's premise changed.
- **Match verification to what's checkable.** PR1 I screenshotted (screen was unlocked). PR2/PR3 the
  Mac display locked/slept mid-session and can't be unlocked headlessly — but both effects are
  CI-verifiable (PR2 behavioral test measures the HUD quantity; PR3 cap is a pure function with
  default+clamp tests), so I merged on green CI per delegated authority. No audio-style human gate
  was needed (unlike seq-1 #251).
- **`one_way_tolerance` read fresh every frame** (like `max_slope_angle`), so direct field
  assignment works, not just the builder.
- **`FrameConfig` default reproduces the old 0.1 cap**, auto-inserted + read with `unwrap_or_default`,
  so hand-built test worlds stay byte-identical (the PATTERNS default-preserving-resource recipe).
- **MINOR bumps** (additive public API): 0.72.0 / 0.73.0 / 0.74.0. **Serial PRs** (version-paperwork
  files conflict if parallel) — merge + sync main before each next branch.

## Evidence & Data

### Commits landed (main)
| Hash | PR | Bump | Summary |
|---|---|---|---|
| `ccfc3c1` | #255 | 0.71→0.72.0 | CharacterController::one_way_tolerance |
| `974232c` | #256 | 0.72→0.73.0 | PhysicsWorld::set_solver_iterations |
| `f5d5dd1` | #257 | 0.73→0.74.0 | FrameConfig::max_dt |

### CI (all 4/4 SUCCESS, squash-merged)
| PR | Test (native) | WASM | Rustdoc | Package |
|---|---|---|---|---|
| #255 | 4m15s | pass | pass | pass |
| #256 | 4m57s | pass | pass | pass |
| #257 | 5m10s | pass | pass | pass |

### Solver-iteration measurements (rapier 0.22, why the example is joints not stacks)
- 14-box plain stack: iters 1 → collapse (sink 20, jitter 100); iters 2–16 → stable (~0.18). 16@n=50 even destabilizes.
- High mass ratio (1000:1 unlocked capstone): iters 4 → crush 5.86; iters 8+ → 0.6 (clean but chaotic visual).
- **Heavy-ended hanging revolute chain (the chosen example):** stretch — 2 iters 0.84, 4 iters 0.21, 16 iters 0.013. Monotonic, stable, no explosion.

### Tests added
- `one_way_tolerance_widens_the_landing_skin`, `default_one_way_tolerance_matches_historical_constant`, `with_one_way_tolerance_sets_field` (#255).
- `set_solver_iterations_sets_and_clamps`, `with_integration_params_overrides_full_struct`, `solver_iterations_stiffen_a_joint_chain` (#256).
- `frame_config_default_matches_historical_cap`, `cap_clamps_long_frames_but_passes_short_ones` (#257).

## Gotchas & Discoveries

- **rapier 0.22's TGS-soft solver is iteration-robust for plain stacks.** Raising
  `num_solver_iterations` above the default 4 shows NO benefit on box stacks (and very high counts
  can destabilize). Demonstrable benefit is in joints under load / extreme mass ratios. Don't assume
  "more iterations = more stable" universally for this rapier version.
- **`IntegrationParameters` is NOT `#[non_exhaustive]`** (rapier 0.22) → struct-update syntax
  `IntegrationParameters { num_solver_iterations: …, ..Default::default() }` works and satisfies
  clippy `field_reassign_with_default` (which rejects `let mut p = default(); p.field = …`).
- **`num_solver_iterations` is `NonZeroUsize`** → `set_solver_iterations` clamps to `.max(1)` (0 panics).
- **The backgrounded-`verify.sh > log; echo $?` gotcha bit again.** A background task's reported
  "exit code 0" is the trailing `echo`'s, NOT verify.sh's — it masked a real clippy failure on PR2
  until I grepped the LOG for "all checks passed". **Always read the log, never trust the task exit
  on a `…; echo` command.** (This is in memory + the seq-1 handoff; still easy to fall into.)
- **macOS display lock/sleep blocks the screenshot playtest** and can't be undone headlessly. When
  it happens, lean on deterministic tests that measure the same quantity the demo shows; only block
  a merge for a human gate when the effect is genuinely CI-unverifiable (audio, OS-runtime).
- **`step_frame` is cross-platform** (`web_time::Instant` on wasm), so the dt cap — and `FrameConfig`
  — applies to both native and wasm; no cfg gating needed.

## Files Changed (per PR; all merged)

- **#255**: `src/physics/character.rs` (field+const+builder+tests), `src/physics/world/character_movement.rs` (use field), `src/physics/world/tests.rs` (threshold test), `examples/one_way_tolerance.rs` (new), CLAUDE.md row.
- **#256**: `src/physics/world.rs` (3 methods), `src/physics/world/tests.rs` (3 tests), `examples/solver_iterations.rs` (new), CLAUDE.md row.
- **#257**: `src/resources/time.rs` (`FrameConfig`+tests), `src/resources/mod.rs` + `src/lib.rs` (re-export), `src/app/core_resources.rs` (auto-insert), `src/app/render/frame.rs` (use `cap`), `examples/frame_dt_cap.rs` (new), CLAUDE.md row.
- **Wrap (this PR)**: `docs/HARDCODING_AUDIT_2026-06-26.md` (Tier-2 resolution log), this handoff.

## User Feedback & Preferences

- Opener: "핸드오프 확인하고 tier2 진행" → read handoff, then proceed with Tier 2.
- On the bundle question (AskUserQuestion): chose **"물리/타이밍 묶음 3건"** (one-way tolerance / solver iters / FrameConfig, serial PRs).
- Standing prefs honored: user-facing reports **Korean**, code/docs **English**; merge authority delegated (squash on green CI) EXCEPT OS/GPU/AV-unverifiable; `cargo fmt` before verify; never trust a masked gate exit; explicit `model` on any subagent (none spawned this session — all hand-done).

## Where We're Going

1. **This handoff + the audit-doc edit land as one `docs(handoff)` PR** (`docs/handoff-hardcoding-tier2`, no package bump).
2. **Next session: read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004 next). New EW → VISION loop; still empty → ASK.
3. **If more hardcoding work:** `docs/HARDCODING_AUDIT_2026-06-26.md` "Open — Tier 2 (remaining)" has the rest. Easiest next: `desired_maximum_frame_latency` (`WindowOptions` field), `STICK_ACTIVATE/RELEASE` deadzone (`UiConfig`), `SLIDER_STEP_FRAC` (per-`Slider`), editor `APP_ID`. Hardest: `MAX_LIGHTS` configurable (dynamic WGSL uniform array). `MAX_GAMEPADS` is easy code but hard to verify (needs >4 physical pads). Tier 3 is naming/dedup polish.

## Risks & Blockers

- **None blocking.** Tree clean (wrap docs only), all three merges green, no open PRs.
- PR2/PR3 had **no visual playtest** (Mac display locked/slept). Both are CI-verifiable via tests that
  measure the demonstrated quantity, so this is low-risk; but a future edit to the `solver_iterations`
  or `frame_dt_cap` *visual* presentation should be screenshot-checked when a display is available.
- The `solver_iterations` behavioral test is rapier-0.22-specific (generous thresholds + comment) — a
  rapier bump must re-verify it.

## Quick Start for Next Session

```bash
cat ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY? EW-004 next → ASK if empty
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -6        # tip = this seq-2 handoff merge, above f5d5dd1 (#257)
git status -s               # clean
# More Tier-2? docs/HARDCODING_AUDIT_2026-06-26.md "Open — Tier 2 (remaining)".
cargo fmt && ./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"
#   (BACKGROUNDED verify must NOT append `; echo` — read the LOG for "all checks passed", the task exit lies.)
```

---

## Session Closed

**Closed at:** 2026-06-27 (KST)
**Commit:** lands via a `docs(handoff)` PR (this file + the audit-doc Tier-2 resolution log).
**Session status:** Handed off — 3 Tier-2 PRs (#255–#257) merged to `main` (v0.71.0 → v0.74.0); audit Tier-2 log updated; memory bumped. This handoff is the session record.
