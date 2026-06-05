# Next dogfooding cycle: prove a shipped-but-never-in-a-game subsystem with a small playable example

**Date:** 2026-06-05
**Status:** PLANNED
**Bead(s):** none (`bd` not installed)
**Epic:** VISION feature+example loop — dogfood shipped-but-never-in-a-game subsystems
**Chain:** `physics-joints` seq `1` (paired with this session's handoff)
**Context:** See `HANDOFF_physics-joints_crane-rotation-sync_2026-06-05.md` for the completed physics-joints
session (data, decisions, the proven loop). This plan defines the **next** cycle.

---

## Problem Statement

The physics-joints work is done, committed (`338eb08`), pushed, and CI-green; that closes candidate **J**.
The VISION loop continues: a few shipped subsystems still have **no playable-game coverage**. The next
session picks one, proves it with a small playable example, and fixes/closes whatever gap it surfaces —
the same loop that has closed one candidate per session (H lighting, I blend, J joints). The remaining
never-in-a-game candidates (per `docs/NEXT_WORK.md`): **`RenderTarget`/`OffscreenCamera` in real play**,
`Timeline`/cutscene, networking. See **Evidence & Data** + **Where We're Going** in the handoff for the
full state and the per-candidate notes.

## Key Findings (from this session + the standing candidate list)

- **`RenderTarget`/`OffscreenCamera` = API exists, no *playable game* (recommended candidate).** Already
  used by the `minimap.rs` **demo** (`app.create_render_target("minimap",256,256)` + `OffscreenCamera`),
  re-exported from `engine::`. So the gap is "in real play", not "missing API" — lower engine-change
  risk, more example-led. → drives Phase 2 (render-to-texture game).
- **VERIFY THE API SURFACE BEFORE PLANNING ENGINE WORK** (this session's hard lesson). The blendtree1d
  plan recommended joints as a "missing public API" — but `add_*_joint` already existed in
  `src/physics/world/joints.rs`; the scout read the struct fields and missed the `mod joints;` submodule.
  Re-framing cost was small but real. For RenderTarget, read `minimap.rs` + `src/renderer/` first to learn
  the *actual* surface, then scope the example to what a real *game* needs that the demo didn't. → Phase 1.
- **The loop is proven and fast** (3 sessions): confirm green → recommend + `/grill-me` scope-lock → plan
  → implement engine + playable example → HTML-checklist playtest + screenshot → verify.sh + rust-survivors
  path-patch check → single commit/push → confirm CI. → drives all phases.
- **`PhysicsSystem` now syncs rotation; the render path is blend-aware + additive.** Any new example
  inherits both with no action. A render-to-texture game is cross-platform-capable (RenderTarget works on
  wasm, unlike physics/lighting) — but confirm in the grill. → Phase 2 (if RenderTarget chosen).
- **IME is off by default; `ImeConfig { allowed: true }` needed for text input.** A render/Timeline
  example likely won't need it. → Phase 2/3 footnote.

## Anti-Goals (What NOT To Do)

- **Don't skip `/grill-me`.** Every shipped candidate was scope-locked via grill first. Lock scope before
  coding. The user sometimes takes every "Recommended" (this session) and sometimes the more ambitious
  option (blend session) — present bounded trade-offs, don't assume.
- **Don't re-scout from struct fields.** Read the actual module + the existing demo to confirm what the
  API does and doesn't do, before claiming a "gap". (The joints mis-scout is the cautionary tale.)
- **Don't pick networking first.** It needs the most infra (`mp_client`/`mp_server` exist but it's the
  least self-contained) — lowest value-per-effort of the remaining candidates.
- **Don't widen the engine surface beyond what the example needs.** This session kept the change to one
  rotation-sync line and used colored sprites (no art). Same discipline next time — don't speculatively
  add render-target features (multi-target, formats) the example doesn't exercise.
- **Don't declare done on `verify.sh` alone.** CI can't run the windowed app; the bugs prior sessions
  found (shader validity, centering, IME, the rotation gap) were all native-only. The example's native
  run + user playtest is the gate.

## Plan

### Phase 1: Confirm green + lock the next candidate's scope

**Goal:** Start from a confirmed-green tip and a grill-locked scope for one candidate.

**Why this approach:** Work is committed and CI was green at handoff; confirm before building on top.
Candidate choice is the user's call (established pattern) — recommend RenderTarget, offer Timeline as the
alternative, and `/grill-me` the chosen one.

- Confirm CI green on `338eb08`: `gh run list --branch main --limit 3` (expect success; if red, fix the
  named job — all checks passed locally so a failure would be environmental).
- Confirm clean state: `git status -s` (clean), `git log --oneline -3` (`338eb08` + this session's
  `session:` commit at tip), `./scripts/verify.sh` (exit 0).
- **Read the real API surface first** (the anti-mis-scout step): `cat examples/minimap.rs`;
  `grep -rn "RenderTarget\|OffscreenCamera\|create_render_target\|layer_mask" src/renderer/ src/app/`;
  note what the demo does (offscreen camera → texture) and what it does NOT (sample the target as a
  sprite/UI texture in a gameplay context? multiple targets? resize?). That delta is the example's job.
- Recommend **`RenderTarget`/`OffscreenCamera` in real play** to the user; offer `Timeline`/cutscene as
  the alternative. Let them choose.
- Run `/grill-me` on the chosen candidate to lock: example concept, the engine gap/API to add (if any),
  the compat/wasm posture, the engine-fix bar, and the proof bar. Produce a grill decision packet.

**Files:** none (process phase).
**Validates with:** `gh run list` shows success on `338eb08`; a written grill decision packet with
`plan_allowed: true`.
**Rollback:** n/a (no code).

### Phase 2: Implement the engine gap + a small playable example

**Goal:** Close the gap the candidate exposes (if any) and prove it with one playable example.

**Why this approach:** VISION — the example is the acceptance test; fix awkward API before release.
Mirrors how this session added the rotation sync + the crane example, and the blend session before it.

If **RenderTarget/OffscreenCamera** (recommended):
- The API exists; build a *playable game* that needs offscreen rendering in play — e.g. a security-camera
  stealth puzzle (a monitor shows a `RenderTarget` of an `OffscreenCamera` watching another room) or a
  mirror/portal puzzle. Surface and fix whatever the *game* (not a demo) needs that `minimap.rs` didn't —
  likely candidates: sampling the render target as a `Sprite`/UI texture in the gameplay layer, target
  resize on viewport change, `layer_mask` selection, multiple simultaneous targets.
- Keep colored sprites / procedural assets (no art pipeline), matching `crane_wrecking_ball` /
  `skeletal_puppet`.

If **Timeline/cutscene** (alternative):
- `Timeline`/`Track`/`TimelineSystem` ship with unit tests but no example. Build a small cutscene/intro
  that drives entity `Transform`s (and maybe camera/fades) along tracks, triggered in play. Surface
  whatever the example needs (e.g. binding tracks to components, an on-finish hook, skip/seek control).

- Keep changes additive (public API/behavior preserved); ensure `cargo build --target wasm32` (lib+bins)
  still builds. RenderTarget/Timeline are cross-platform; a render example *can* be wasm-functional
  (confirm in grill) unlike physics/lighting. Gate native-only deps only if the example pulls any.
- Add unit tests for any new engine surface; reuse the wrap-a-system / `world.get_mut` patterns from the
  handoff's Code Analysis.

**Files:** `src/renderer/...` or `src/app/...` (if a render gap) or `src/timeline.rs`; `src/lib.rs`
(re-exports if new types); `examples/...` (the playable example); tests; docs
(CHANGELOG/NEXT_WORK/HANDOFF/CLAUDE.md).
**Validates with:** `./scripts/verify.sh` green after each change; new unit tests pass; native run renders.
**Rollback:** revert the example + any new public API (additive, isolated).

### Phase 3: Playtest, cross-repo check, commit

**Goal:** Prove it in real play and land it.

**Why this approach:** Same proof bar that caught this session's rotation gap (CI can't run the window).

- Native `cargo run --example <name>`; drive it; `screencapture` for a self-check.
- Build an HTML checklist (reuse `/tmp/crane_wrecking_ball_test.html` as a template — grouped items,
  localStorage, markdown export); deliver it + launch the demo; get the user's per-item ✅/❌ sign-off.
- Fix anything surfaced (example-level or engine-level per the grill bar).
- `rust-survivors` `cargo check` against the tip via the `--config` path patch (see memory
  `rust-survivors-engine-pin`), then `git checkout -- Cargo.lock`. **Only strictly required if Phase 2
  touched a shared path** rust-survivors uses — it uses 0 RenderTarget/Timeline and 0 PhysicsSystem
  (raw `PhysicsWorld` only), so a render/Timeline example is likely behaviorally inert there; still run
  the compile check for API compatibility.
- On full sign-off: single commit (`feat(...)` style) + push to `main` (repo norm; the auto-mode
  classifier may prompt on push-to-main — the user's standing norm is direct-to-main); confirm CI.

**Files:** the example + any fix; docs.
**Validates with:** user playtest all-✅; verify.sh green; rust-survivors clean; CI green.
**Rollback:** `git revert` the feature commit (additive).

## Dependencies & Order

- Phase 1 → 2 → 3 strictly sequential (scope before code; code before playtest).
- Within Phase 2, any engine gap must land before the example can use it.
- Phase 3's rust-survivors check is a compile-compat formality unless Phase 2 touched a path
  rust-survivors uses (it doesn't use RenderTarget/Timeline/PhysicsSystem) — run it anyway, it's cheap.

## Risks & Mitigations

- **Candidate choice may change at grill.** Likely (user's call). Mitigation: Phase 2 is written for both
  scouted candidates; if the user picks networking instead, scout that subsystem's API surface first
  (grep `src/network/` + the `mp_*` examples) before planning.
- **RenderTarget gap may be bigger than a "small fix"** (e.g. sampling a target as a gameplay texture may
  need real renderer plumbing). Medium. Mitigation: in the grill, decide the engine-fix bar explicitly —
  if the gap is large, document it as a known limit (like lighting's occlusion) and ship the example
  against what works, rather than a deep renderer refactor.
- **Re-scouting a non-existent gap** (the joints lesson). Low now that it's called out. Mitigation: Phase
  1 reads the module + the demo before claiming a gap.
- **CI red on `338eb08`.** Low (CI already green at handoff). Mitigation: Phase 1 confirms before building.

## Success Criteria

- **Minimum viable:** one never-in-a-game subsystem gains a playable example that exercises it in real
  play; the engine gap it surfaces is closed (or documented if big); `verify.sh` + new tests + native
  playtest + rust-survivors check all green; committed/pushed/CI-green.
- **Full success:** the example reads as a small *playable* artifact (a goal loop or clearly interactive
  toy), any new public API is minimal and fork-friendly, and `docs/NEXT_WORK.md` trims the candidate from
  the remaining list (candidate **K**).

## Quick Start

```bash
# Restore full context
cat plans/handoffs/HANDOFF_physics-joints_crane-rotation-sync_2026-06-05.md

# Verify starting state
git log --oneline -3                       # expect 338eb08 + this session's commit at tip
git status -s                              # expect clean
gh run list --branch main --limit 3        # confirm CI green on 338eb08
./scripts/verify.sh                        # 5 checks, expect exit 0

# Scout the recommended candidate (RenderTarget/OffscreenCamera) — READ THE REAL SURFACE FIRST
cat examples/minimap.rs                     # the existing demo (OffscreenCamera → RenderTarget)
grep -rn "RenderTarget\|OffscreenCamera\|create_render_target\|layer_mask" src/renderer/ src/app/
sed -n '148,170p' docs/NEXT_WORK.md         # remaining candidates + candidate J entry
cat docs/VISION.md                          # the feature+example loop

# First action
#   1) Confirm CI green on 338eb08.
#   2) Recommend RenderTarget/OffscreenCamera in real play (Timeline as alternative); /grill-me the chosen one.
#   3) Phase 2: build a render-to-texture game (security-camera / portal puzzle) + fix what real play needs.
```
