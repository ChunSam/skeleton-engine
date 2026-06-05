# Next dogfooding cycle: pick a never-in-a-game subsystem and prove it with a small playable example

**Date:** 2026-06-05
**Status:** PLANNED
**Bead(s):** none (`bd` not installed)
**Epic:** VISION feature+example loop — dogfood shipped-but-never-in-a-game subsystems
**Chain:** `blendtree1d-locomotion` seq `1` (paired with this session's handoff)
**Context:** See `HANDOFF_blendtree1d-locomotion_crossfade-blend-ime_2026-06-05.md` for the completed
BlendTree1D session (data, decisions, the proven loop). This plan defines the **next** cycle.

---

## Problem Statement

The BlendTree1D work is done, committed (`18a6b48`), and pushed; that closes candidate **I**. The VISION
loop continues: several shipped subsystems still have **no playable-game coverage**. The next session
picks one, proves it with a small playable example, and fixes/closes whatever gap it surfaces — the same
loop that this session used to close 3 engine issues. The remaining never-in-a-game candidates (per
`docs/NEXT_WORK.md`): `Timeline`/cutscene, **physics joints**, `RenderTarget`/`OffscreenCamera` in real
play, networking. Two were scouted this session (see Key Findings) so the next session can start fast.

## Key Findings (from this session's scouting)

- **Physics joints = a real public-API gap (recommended candidate).** `PhysicsWorld` holds
  `impulse_joint_set`/`multibody_joint_set` but they are **`pub(crate)`** (`src/physics/world.rs:143-144`)
  and there is **no public joint-creation API**. rapier2d's `ImpulseJointHandle` is re-exported
  (`src/physics/mod.rs:10`) but nothing lets a game build a joint. → high dogfooding value: the example
  forces a public joint API. → drives Phase 2 (joints).
- **`RenderTarget`/`OffscreenCamera` = API exists, no *playable game* (strong alternative).** Already used
  by the `minimap.rs` *demo* (`app.create_render_target("minimap",256,256)` + `OffscreenCamera`), re-exported
  from `engine::`. So the gap is "in real play", not "missing API" — lower engine-change risk, more
  example-led. → alternative for Phase 2.
- **The loop is proven and fast** (this session): `/grill-me` scope-lock → plan → implement → HTML-checklist
  playtest + screenshots → `verify.sh` + rust-survivors path-patch check → single commit/push. → drives all phases.
- **Render path is now blend-aware and additive** — `InstanceRaw` is 116B with `to_uv`/`blend`; any new render
  example inherits this with no action (weight=0 = identical). → Phase 2 (if RenderTarget chosen).
- **IME is now off by default** — any new example needing text input must insert `ImeConfig { allowed: true }`.
  Joint/render examples won't need it. → Phase 2/3 footnote.

## Anti-Goals (What NOT To Do)

- **Don't skip `/grill-me`.** Every candidate this user has shipped was scope-locked via grill first; they
  answer precisely and sometimes pick the *bigger* option (this session: true 2-UV blend over a doc-only fix).
  Lock scope before coding.
- **Don't pick networking first.** It needs the most infra (`mp_client`/`mp_server` exist but it's the least
  self-contained) — lowest value-per-effort of the remaining candidates.
- **Don't widen the engine surface beyond what the example needs.** This session kept the blend change scoped
  and used a gen-example PNG instead of adding a runtime image API. Same discipline next time.
- **Don't declare done on `verify.sh` alone.** CI can't run the windowed app; the bugs this session found
  (shader validity, centering, IME) were all native-only. The example's native run + user playtest is the gate.

## Plan

### Phase 1: Confirm green + lock the next candidate's scope

**Goal:** Start from a confirmed-green tip and a grill-locked scope for one candidate.

**Why this approach:** The work is committed but CI was `in_progress` at handoff; confirm it before building
on top. Candidate choice is the user's call (established pattern) — recommend physics joints, present
RenderTarget as the alternative, and `/grill-me` the chosen one.

- Confirm CI green on `18a6b48`: `gh run list --branch main --limit 3` (expect success; if red, fix the
  named job — all checks passed locally so a failure would be environmental).
- Confirm clean state: `git status -s` (clean), `git log --oneline -3` (`18a6b48` at tip), `./scripts/verify.sh`.
- Recommend **physics joints** (real API gap to close) to the user; offer `RenderTarget`/`OffscreenCamera`
  (playable game for an existing API) as the alternative. Let them choose.
- Run `/grill-me` on the chosen candidate to lock: example concept, the engine gap/API to add, the
  compat/wasm posture, the engine-fix bar, and the proof bar. Produce a grill decision packet.

**Files:** none (process phase).
**Validates with:** `gh run list` shows success on `18a6b48`; a written grill decision packet with `plan_allowed: true`.
**Rollback:** n/a (no code).

### Phase 2: Implement the engine gap + a small playable example

**Goal:** Close the API gap the candidate exposes and prove it with one playable example.

**Why this approach:** VISION — the example is the acceptance test; fix awkward API before release. Mirrors
exactly how this session added `BlendUv`/`ImeConfig` and the `blend_locomotion` demo.

If **physics joints** (recommended):
- Add a **public joint-creation API** on `PhysicsWorld` (e.g. `add_revolute_joint(body_a, body_b, anchor…)`,
  `add_fixed_joint(...)`, maybe `add_prismatic_joint(...)`) wrapping rapier2d's `ImpulseJointSet::insert`;
  return the `ImpulseJointHandle`. Keep the joint sets `pub(crate)`; expose only the safe builders +
  accessors (mirror the existing `PhysicsWorld` encapsulation pattern — see `docs/PATTERNS.md`).
- Re-export the needed types from `engine::`. Add unit tests in `src/physics/` (construct two bodies, add a
  joint, step, assert the constraint holds — e.g. distance/angle bound).
- Build a small playable example in `examples/` (e.g. a 2-link pendulum/rope bridge or a wrecking-ball /
  ragdoll contraption) under `examples/games/<name>/` if it has a win/lose loop, else top-level. Drive bodies
  with input; visualize with colored `Sprite`s (no art needed, like `skeletal_puppet`).

If **RenderTarget/OffscreenCamera** (alternative):
- The API exists; build a *playable game* that needs offscreen rendering in play — e.g. a security-camera
  stealth puzzle (monitor = a `RenderTarget` of an `OffscreenCamera` elsewhere in the level) or a
  mirror/portal puzzle. Surface and fix whatever the *game* (not a demo) needs that `minimap.rs` didn't.

- Keep changes additive (public API/behavior preserved); ensure `cargo build --target wasm32` (lib+bins)
  still builds (joints/render are native-capable; gate native-only deps if any, like the audio/gpu-particle examples).

**Files:** `src/physics/world.rs` (+joint builders) or the render path; `src/lib.rs` (re-exports);
`examples/...` (the playable example); `src/physics/...` tests; docs (CHANGELOG/NEXT_WORK/HANDOFF/CLAUDE.md).
**Validates with:** `./scripts/verify.sh` green after each change; new unit tests pass.
**Rollback:** revert the example + the new public API (additive, isolated) — the joint sets were already private.

### Phase 3: Playtest, cross-repo check, commit

**Goal:** Prove it in real play and land it.

**Why this approach:** Same proof bar that caught this session's centering + IME bugs (CI can't run the window).

- Native `cargo run --example <name>`; drive it; `screencapture` for a self-check.
- Build an HTML checklist (reuse `/tmp/blend_locomotion_test.html` as a template — grouped items, localStorage,
  markdown export); deliver it + launch the demo; get the user's per-item ✅/❌ sign-off.
- Fix anything surfaced (example-level or engine-level per the grill bar).
- `rust-survivors cargo check` against the tip via the `--config` path patch (see memory
  `rust-survivors-engine-pin`), then `git checkout -- Cargo.lock`.
- On full sign-off: single commit (`feat(...)` style) + push to `main` (repo norm); confirm CI.

**Files:** the example + any fix; docs.
**Validates with:** user playtest all-✅; verify.sh green; rust-survivors clean; CI green.
**Rollback:** `git revert` the feature commit (additive).

## Dependencies & Order

- Phase 1 → 2 → 3 strictly sequential (scope before code; code before playtest).
- Within Phase 2, the engine API gap (joints) must land before the example can use it.
- Phase 3's rust-survivors check only matters if Phase 2 touched a shared path (physics or render) — joints
  touch `PhysicsWorld`, which rust-survivors uses, so the check is required.

## Risks & Mitigations

- **Candidate choice may change at grill.** Likely (user's call). Mitigation: Phase 2 is written for both
  scouted candidates; if the user picks Timeline/networking instead, scout that subsystem's API surface first
  (grep `src/` + check for an existing demo) before planning the example.
- **Joints API design churn** (rapier2d joint params are verbose). Medium. Mitigation: start with one joint
  type (revolute — most visually obvious) end-to-end through the example, then add others only if the example
  needs them. Don't expose the raw rapier sets.
- **Shared-path regression** (physics or render). Low (additive). Mitigation: the rust-survivors path-patch
  check + verify.sh, every change; weight=0/no-joint paths stay identical.
- **CI red on `18a6b48`.** Low (local verify green). Mitigation: Phase 1 confirms before building on top.

## Success Criteria

- **Minimum viable:** one never-in-a-game subsystem gains a playable example that exercises it in real play;
  the engine gap it surfaces is closed (or documented if big); `verify.sh` + new tests + native playtest +
  rust-survivors check all green; committed/pushed.
- **Full success:** the example reads as a small *playable* artifact (a goal loop or a clearly interactive
  toy), the new public API is minimal and fork-friendly, and `docs/NEXT_WORK.md` trims the candidate from the
  remaining list (candidate **J**).

## Quick Start

```bash
# Restore full context
cat plans/handoffs/HANDOFF_blendtree1d-locomotion_crossfade-blend-ime_2026-06-05.md

# Verify starting state
git log --oneline -3                       # expect 18a6b48 at tip
git status -s                              # expect clean
gh run list --branch main --limit 3        # confirm CI green on 18a6b48
./scripts/verify.sh                        # 5 checks, expect exit 0

# Scout the recommended candidate (physics joints)
sed -n '140,160p' src/physics/world.rs     # impulse_joint_set/multibody_joint_set are pub(crate)
grep -rn "joint\|Joint" src/physics/        # current joint surface (rapier re-exports only)
sed -n '120,146p' docs/NEXT_WORK.md         # the remaining candidate list
cat examples/minimap.rs                     # RenderTarget/OffscreenCamera usage (the alternative)

# First action
#   1) Confirm CI green on 18a6b48.
#   2) Recommend physics joints to the user (RenderTarget as alternative); /grill-me the chosen one.
#   3) Phase 2: add the public joint-creation API on PhysicsWorld + a playable contraption example.
```
