# Next dogfooding cycle: prove a shipped-but-never-in-a-game subsystem with a small playable example

**Date:** 2026-06-05
**Status:** PLANNED
**Bead(s):** none (`bd` not installed)
**Epic:** VISION feature+example loop — dogfood shipped-but-never-in-a-game subsystems
**Chain:** `rendertarget-offscreen` seq `1` (paired with this session's handoff)
**Context:** See `HANDOFF_rendertarget-offscreen_security-camera-disjoint-fix_2026-06-05.md` for the
completed RenderTarget session (data, decisions, the proven loop). This plan defines the **next** cycle.

---

## Problem Statement

The RenderTarget/OffscreenCamera work is done, committed (`cbbdfbd`), pushed, and CI-green; that closes
candidate **K**. The VISION loop continues: a couple of shipped subsystems still have **no playable-game
coverage**. The next session picks one, proves it with a small playable example, and fixes/closes whatever
gap it surfaces — the same loop that has closed one candidate per session (H lighting, I blend, J joints,
K render-target). The remaining never-in-a-game candidates (per `docs/NEXT_WORK.md`): **`Timeline`/cutscene
(recommended)** and **networking**. See **Where We're Going** + **Evidence & Data** in the handoff for the
full state and the per-candidate notes.

## Key Findings (from this session + the standing candidate list)

- **`Timeline` = ships with unit tests, no example (recommended candidate).** `src/timeline.rs` exports
  `Timeline`, `Track`, `Keyframe`, `Lerp`, `TimelineSystem` (re-exported from `engine::`), with unit tests
  but **zero example/game usage** (grep `examples/` for `Timeline` = none). Self-contained, cross-platform,
  no new deps — lower risk than networking. → drives Phase 2 (a small cutscene/intro driven by tracks).
- **VERIFY THE API SURFACE BEFORE PLANNING ENGINE WORK** (the chain's standing lesson, paid off twice).
  The blendtree1d plan mis-scouted joints as "missing API"; the physics-joints plan correctly flagged
  RenderTarget as "API exists, no game" and a real bug surfaced. For Timeline, **read `src/timeline.rs`
  first** (what `Track`/`Keyframe`/`Lerp`/`TimelineSystem` actually do, what they bind to) before claiming
  a gap, then scope the example to what a real cutscene needs that the unit tests don't. → Phase 1.
- **The loop is proven and fast** (4 sessions): confirm green → recommend + `/grill-me` scope-lock → plan
  → implement engine + playable example → HTML-checklist playtest + screenshot → verify.sh + rust-survivors
  path-patch check → single commit/push → confirm CI. → drives all phases.
- **The agent can run the playtest itself** when delegated (this session). See memory
  `playtest-windowed-examples` (osascript window bounds by PID + `caffeinate` + `screencapture -R` +
  synthetic `key down/up`). Still deliver the HTML checklist + screenshot. → Phase 3.
- **Diagnose render/engine bugs by evidence, not theory** (decisive this session): a recolor test + an
  `eprintln` of the actual values pinned the offscreen-render bug fast. → Phase 2 if a bug surfaces.
- **`Camera` is top-left anchored, Y-down** (`src/camera.rs`): a camera at `(0,0)` shows world
  `[0,W]×[0,H]`, +Y down. A Timeline example that moves `Transform`s/camera must use this convention (the
  RenderTarget example's first draft rendered off-screen by assuming center-origin/Y-up). → Phase 2 footnote.

## Anti-Goals (What NOT To Do)

- **Don't skip `/grill-me`.** Every shipped candidate was scope-locked via grill first. Lock scope (example
  concept, the engine gap/API if any, compat/wasm posture, engine-fix bar, proof bar) before coding.
- **Don't re-scout from re-exports alone.** Read `src/timeline.rs` (the module + its tests) to confirm what
  the API does and doesn't do before claiming a "gap". (The joints mis-scout is the cautionary tale; the
  RenderTarget "API complete, real bug downstream" is the positive counter-example.)
- **Don't pick networking first.** It needs the most infra (`mp_client`/`mp_server` exist but it's the
  least self-contained) — lowest value-per-effort of the remaining candidates.
- **Don't widen the engine surface beyond what the example needs.** This session kept the change to one
  offscreen-submit fix (+18 net) and used colored sprites (no art). Same discipline — don't speculatively
  add Timeline features (easing curves, event tracks, blending) the example doesn't exercise.
- **Don't declare done on `verify.sh` alone.** CI can't run the windowed app; this session's bug (offscreen
  rendering the main camera) and the RenderTarget coordinate gotcha were both native-only. The example's
  native run + playtest is the gate.

## Plan

### Phase 1: Confirm green + lock the next candidate's scope

**Goal:** Start from a confirmed-green tip and a grill-locked scope for one candidate.

**Why this approach:** Work is committed and CI was green at handoff; confirm before building on top.
Candidate choice is the user's call (established pattern) — recommend Timeline, offer networking as the
alternative, and `/grill-me` the chosen one.

- Confirm CI green on `cbbdfbd`: `gh run list --branch main --limit 3` (expect success on run
  `27012114491`; if red, fix the named job — all checks passed locally so a failure would be environmental).
- Confirm clean state: `git status -s` (clean), `git log --oneline -3` (`cbbdfbd` + this session's
  `session:` commit at tip), `./scripts/verify.sh` (exit 0).
- **Read the real API surface first** (the anti-mis-scout step): `sed -n '1,200p' src/timeline.rs`;
  `grep -rln "Timeline\|TimelineSystem" examples/` (confirm no example); note what `Track`/`Keyframe`/`Lerp`
  do, what a `Timeline` binds to (a single value? a component field? a `Transform`?), whether it loops,
  has an on-finish hook, or supports seek/skip. That delta is the example's job.
- Recommend **`Timeline`/cutscene** to the user; offer **networking** as the alternative. Let them choose.
  If they pick networking instead, scout `src/network/` + the `mp_*` examples' API surface first.
- Run `/grill-me` on the chosen candidate to lock: example concept (e.g. an intro/cutscene that drives
  entity `Transform`s + camera + a fade along tracks, triggered in play), the engine gap/API to add (if
  any), the compat/wasm posture, the engine-fix bar, and the proof bar. Produce a grill decision packet.

**Files:** none (process phase).
**Validates with:** `gh run list` shows success on `cbbdfbd`; a written grill decision packet with
`plan_allowed: true`.
**Rollback:** n/a (no code).

### Phase 2: Implement the engine gap (if any) + a small playable example

**Goal:** Close the gap the candidate exposes (if any) and prove it with one playable example.

**Why this approach:** VISION — the example is the acceptance test; fix awkward API/bugs before release.
Mirrors how this session added the security_camera example + the offscreen-render fix, and the crane
session before it.

If **Timeline/cutscene** (recommended):
- `Timeline`/`Track`/`TimelineSystem` ship with unit tests but no example. Build a small playable cutscene
  or scripted intro that drives entity `Transform`s (and maybe camera position/zoom and a fade) along
  tracks, triggered in real play (e.g. press a key to play the cutscene, with a skip). Surface and fix
  whatever the *example* (not a unit test) needs that the API didn't provide cleanly — likely candidates:
  binding a `Track` to a component field ergonomically, an on-finish hook/callback, a skip/seek control, or
  looping. Decide the engine-fix bar in the grill (fix if small; document if big — like RenderTarget's
  deferred items).
- Use the **top-left/Y-down** camera convention (`src/camera.rs`) — don't repeat the RenderTarget draft's
  center-origin mistake. Colored sprites / procedural assets (no art), matching `security_camera`/`crane`.

If **networking** (alternative):
- `mp_client`/`mp_server` exist but there's no small *playable* multiplayer toy. Scope something minimal
  (two clients moving dots in a shared room) and surface whatever the example needs. Higher infra cost —
  scope tightly.

- Keep changes additive (public API/behavior preserved); ensure `cargo build --target wasm32` (lib+bins)
  still builds. Timeline is cross-platform; a Timeline example can be wasm-functional (confirm in grill).
- Add unit tests for any NEW engine surface (a new hook/binding). If the fix is GPU/windowed-only (like the
  offscreen-submit fix), it may not be unit-testable — validate via the native run + playtest instead, and
  say so. Reuse the wrap-a-system / `world.get_mut` patterns from the handoff's Code Analysis.

**Files:** `src/timeline.rs` (+ `src/lib.rs` re-exports if new types) or `src/network/...`; `examples/...`
(the playable example); tests; docs (CHANGELOG/NEXT_WORK/HANDOFF/CLAUDE.md).
**Validates with:** `./scripts/verify.sh` green after each change; new unit tests pass; native run renders
the cutscene end-to-end.
**Rollback:** revert the example + any new public API (additive, isolated).

### Phase 3: Playtest, cross-repo check, commit

**Goal:** Prove it in real play and land it.

**Why this approach:** Same proof bar that caught this session's offscreen-render bug (CI can't run the
window).

- Native `cargo run --example <name>`; drive it; `screencapture` for a self-check. The agent can run the
  full checklist itself if the user delegates ("체크리스트 실행해줘") — see memory `playtest-windowed-examples`.
- Build an HTML checklist (reuse `/tmp/security_camera_test.html` as a template — grouped items,
  localStorage, markdown export); deliver it + a screenshot; get the user's per-item ✅/❌ sign-off (or run
  it yourself if delegated).
- Fix anything surfaced (example-level or engine-level per the grill bar).
- `rust-survivors` `cargo check` against the tip via the `--config` path patch (see memory
  `rust-survivors-engine-pin`), then `git checkout -- Cargo.lock`. Only strictly required if Phase 2
  touched a shared path rust-survivors uses — Timeline is likely behaviorally inert there; still run the
  compile check for API compatibility.
- On full sign-off: single commit (`feat(...)` style) + push to `main` (repo norm; direct-to-main is the
  user's standing norm); confirm CI.

**Files:** the example + any fix; docs.
**Validates with:** playtest all-✅; verify.sh green; rust-survivors clean; CI green.
**Rollback:** `git revert` the feature commit (additive).

## Dependencies & Order

- Phase 1 → 2 → 3 strictly sequential (scope before code; code before playtest).
- Within Phase 2, any engine gap must land before the example can use it.
- Phase 3's rust-survivors check is a compile-compat formality unless Phase 2 touched a path
  rust-survivors uses (it uses no Timeline/networking) — run it anyway, it's cheap.

## Risks & Mitigations

- **Candidate choice may change at grill.** Likely (user's call). Mitigation: Phase 2 is written for both
  Timeline and networking; if the user picks networking, scout `src/network/` + `mp_*` examples first.
- **Timeline may need a bigger engine addition than a "small fix"** (e.g. an on-finish hook or component-
  field binding may need real plumbing). Medium. Mitigation: decide the engine-fix bar explicitly in the
  grill — if the gap is large, document it as a known limit (like RenderTarget's deferred items) and ship
  the example against what works.
- **Re-scouting a non-existent gap** (the joints lesson). Low now that it's called out twice. Mitigation:
  Phase 1 reads `src/timeline.rs` + its tests before claiming a gap.
- **CI red on `cbbdfbd`.** Low (CI already green, run `27012114491`). Mitigation: Phase 1 confirms before
  building.

## Success Criteria

- **Minimum viable:** one never-in-a-game subsystem gains a playable example that exercises it in real
  play; the engine gap it surfaces is closed (or documented if big); `verify.sh` + new tests + native
  playtest + rust-survivors check all green; committed/pushed/CI-green.
- **Full success:** the example reads as a small *playable* artifact (a triggerable cutscene with skip, or
  a clearly interactive toy), any new public API is minimal and fork-friendly, and `docs/NEXT_WORK.md`
  trims the candidate from the remaining list (candidate **L**).

## Quick Start

```bash
# Restore full context
cat plans/handoffs/HANDOFF_rendertarget-offscreen_security-camera-disjoint-fix_2026-06-05.md

# Verify starting state
git log --oneline -3                       # expect cbbdfbd + this session's commit at tip
git status -s                              # expect clean
gh run list --branch main --limit 3        # confirm CI green on cbbdfbd (run 27012114491)
./scripts/verify.sh                        # 5 checks, expect exit 0

# Scout the recommended candidate (Timeline) — READ THE REAL SURFACE FIRST
sed -n '1,200p' src/timeline.rs            # Timeline/Track/Keyframe/Lerp/TimelineSystem + unit tests
grep -rln "Timeline" examples/             # confirm there is NO Timeline example
sed -n '168,200p' docs/NEXT_WORK.md        # remaining candidates + candidate K entry
cat docs/VISION.md                         # the feature+example loop

# First action
#   1) Confirm CI green on cbbdfbd.
#   2) Recommend Timeline/cutscene (networking as alternative); /grill-me the chosen one.
#   3) Phase 2: build a small triggerable cutscene example + fix what real play needs (per the grill bar).
```
