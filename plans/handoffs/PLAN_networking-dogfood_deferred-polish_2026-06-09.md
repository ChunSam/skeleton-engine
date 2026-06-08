# Deferred engine-polish follow-ups (wasm crispness + remote-entity helper + new-breadth audit)

**Date:** 2026-06-09
**Status:** PLANNED
**Bead(s):** none (`bd` unavailable in this project)
**Epic:** VISION feature+example loop — post-breadth polish / dogfooding
**Chain:** `networking-dogfood` seq `3`
**Context:** See `HANDOFF_networking-dogfood_deferred-polish_2026-06-09.md` for the full session that produced this plan. Seq 2 finalized the wasm/v4.1.0 release, added `scripts/wasm_smoke.sh`, and made the engine source fully English; this plan captures the four follow-ups that were deliberately deferred (most need a monitor-on / real-GPU session).

---

## Problem Statement

The breadth example program is complete (every engine subsystem now has a playable example, candidates A–M), the wasm path is hardened, and the source is fully English. What remains are **non-breadth** follow-ups that were deferred because they either (a) need real-GPU / GUI verification that a monitor-off session can't do, (b) are blocked on more signal, or (c) are open-ended. This plan sequences them so a future session can execute without re-discovering the context.

The single most concrete, ready item is **wasm Retina crispness**. The rest are progressively more speculative.

## Key Findings (from seq 2)

- The wasm surface lives at the canvas *logical* (CSS) size by design — `finish_init` locks the canvas CSS box to the drawing buffer (`window.rs:314-329`), and the `Resized` handler caps the buffer at the canvas DOM size to stay under the WebGL2 2048 texture limit (`window.rs:89-109`). This is correct but **soft on Retina** (no DPR scaling of the backing buffer). → drives Phase 1.
- All three of seq 1/2's wasm bugs lived in exactly this surface-sizing path (HiDPI viewport in `schedule.rs`, font in `renderer/mod.rs`+`window.rs`, canvas stretch in `window.rs finish_init`). **Headless SwiftShader masked every one of them** until a real browser render. → Phase 1 must verify on a real GPU, not just headless.
- `scripts/wasm_smoke.sh` exists and guards the catastrophic class (no-run / no-connect / blank). It will serve as the **regression guard** while editing the surface path, but it cannot confirm sharpness. → Phase 1 verification = smoke (regression) + real-GPU eyeball (sharpness).
- Remote-entity bookkeeping (`HashMap<id, Entity>`) is reimplemented inline in both `mp_client` and `coin_race`; two examples is not enough to fix the abstraction shape. → Phase 2 is blocked on a 3rd distinct networked example.
- Breadth is complete; there is no scheduled "next feature." → Phase 3 is an audit, not a build.

## Anti-Goals (What NOT To Do)

- **Do NOT raise the WebGL2 texture cap or remove the 2048 clamp** — it exists for a reason; the DPR-aware buffer must clamp `canvas_css_px × DPR` to ≤2048 per axis and fall back gracefully.
- **Do NOT "fix" crispness by changing the CSS display size** — display stays at logical (CSS) px; only the *drawing buffer* gets DPR-scaled. (Reversing this is how bug #3 happened.)
- **Do NOT verify Phase 1 on headless SwiftShader alone** — it renders sprites but masks geometry/sharpness issues and skips real glyph rasterization. Real-GPU eyeball is mandatory for the crispness claim.
- **Do NOT build the remote-entity helper from the current two examples** — deferred three sessions running; needs a 3rd *distinct* networked example first (Phase 2 precondition).
- **Do NOT invent a "new feature" without an example** — VISION's bar is "a feature is not done until a small playable example exercises it in real play." Phase 3 picks a feature *and* its example together, or does nothing.
- **Do NOT touch the `rust-survivors` repo without the maintainer's go** — Phase 4 is their call.

## Plan

### Phase 1: wasm Retina crispness (DPR-aware backing buffer) — PRIMARY

**Goal:** wasm renders crisply on Retina by making the drawing buffer DPR-aware (clamped to the WebGL2 2048 limit), while the canvas CSS display size stays at logical px.

**Why this approach:** Today the buffer = CSS px (≤2048) and the camera/viewport math treats the surface as logical (the seq-1 HiDPI fix forced `scale_factor = 1.0` on wasm). Scaling the buffer to `CSS × DPR` (capped 2048) gives 1:1 device-pixel rendering; the viewport/camera must then read the *logical* size (CSS px), not the raw buffer, so world coordinates stay logical. This is the inverse subtlety of bug #1 — get it wrong and Retina re-breaks.

- Make the `Resized` handler (`window.rs:89`) set the surface/`gpu.config` to `min(canvas_css_px × devicePixelRatio, 2048)` per axis; keep the canvas CSS box at logical px (don't relock it to the now-larger buffer — adjust `finish_init`'s canvas-size lock accordingly).
- Re-derive `ViewportSize`/`DisplayScaleFactor` (`schedule.rs:106-130`) so the *world* viewport stays logical (CSS px) on wasm while the GPU renders at the DPR-scaled buffer — i.e. wasm now has a real scale factor again, but applied as a render-resolution multiplier, not a viewport divisor. Unit-test the math where possible (logical viewport invariant under DPR change).
- Handle the 2048 clamp gracefully: above the cap, fall back to logical-size buffer (current behavior) and `log` once.

**Files:** `src/app/window.rs` (Resized + finish_init), `src/app/schedule.rs` (viewport/scale derivation), possibly `src/renderer/context.rs` (initial surface size).
**Validates with:** `cargo +1.88.0` full gate; `scripts/wasm_smoke.sh` stays green (regression guard); **real-GPU browser eyeball on Retina** (monitor on) — text + sprites sharp at DPR=2, layout unchanged vs the logical-size render; deterministic headless DPR=1-vs-2 repro shows no off-screen/clipping regression.
**Rollback:** revert the surface-sizing edits; the logical-size render is the safe baseline.
**Note:** this is the "Bigger change" flagged in seq 2. Do it in a monitor-on session.

### Phase 2: Reusable remote-entity helper — BLOCKED (needs a 3rd networked example)

**Goal:** Extract the repeated `HashMap<network_id, Entity>` spawn/update/despawn bookkeeping into a reusable engine helper — *only once a 3rd distinct networked example confirms the shape*.

**Why this approach:** `mp_client` and `coin_race` both do it inline, but they're too similar to reveal the right abstraction (deferred across seq 1 and 2 for exactly this reason). The precondition is a 3rd networked example with a *different* shape (e.g. client-side prediction, or entity-typed spawns, or interest management).

- **Precondition:** build a 3rd distinct networked example first (this is itself a breadth-ish task with a playable-example bar). Capture what its remote-entity bookkeeping needs that the other two didn't.
- Then design the helper against all three call sites; add it to the engine, migrate the three examples, keep behavior identical.

**Files:** TBD (new `src/network/*` helper + example migrations) — do not scaffold until the precondition is met.
**Validates with:** all three networked examples compile + run unchanged on the helper; unit tests for the bookkeeping; `cargo +1.88.0` gate.
**Rollback:** the inline bookkeeping is the baseline; don't migrate until the helper proves itself on all three.

### Phase 3: New breadth feature exploration — OPEN-ENDED (audit, not build)

**Goal:** Decide whether there is a genuinely new engine capability worth adding now that subsystem-breadth coverage is complete.

**Why this approach:** `docs/NEXT_WORK.md` records "every engine subsystem now has at least one playable example" — so there is no scheduled next feature. This phase is an *audit* that either surfaces a real new capability (paired with a playable example, per VISION) or concludes "nothing now" and stays closed.

- Re-read `docs/VISION.md` priorities. Survey what forkers of a genre-agnostic 2D skeleton commonly need that this engine lacks (candidates seen in passing, none scheduled: tilemap autotiling, particle-from-config assets, a 2nd networked shape from Phase 2, audio ducking/sidechain, a UI focus/tab-nav pass, asset-pack/atlas tooling).
- If a candidate clears the bar (genuinely useful + provable by a small playable example), spin it into its own PLAN; otherwise record "no new feature scheduled" and stop.

**Files:** none until a candidate is chosen.
**Validates with:** a new PLAN exists for the chosen feature, or `NEXT_WORK.md` records the audit conclusion.
**Rollback:** n/a (planning).

### Phase 4: rust-survivors WIP docs cleanup — OPTIONAL (separate repo, maintainer's call)

**Goal:** Organize/commit the ~20 uncommitted WIP doc changes sitting in the `rust-survivors` repo.

**Why this approach:** They are unrelated to engine work and live in a separate, independently-developed repo; only the maintainer should decide their fate.

- With the maintainer's go: review the ~20 changed/deleted docs, group into coherent commits (or discard), and land on `rust-survivors` `main`.

**Files:** `rust-survivors` repo only — never mixed into a `skeleton-engine` commit.
**Validates with:** `git -C ../rust-survivors status` clean (or intentionally-WIP), the game still builds/runs.
**Rollback:** the docs are already uncommitted; `git restore` to undo.

## Dependencies & Order

- **Phase 1 is the only ready, high-value item** — do it first, in a monitor-on session.
- **Phase 2 is blocked on its own precondition** (a 3rd networked example) — not actionable until that exists.
- **Phase 3 is an audit** — can run any time; cheap; gates whether more breadth work exists.
- **Phase 4 is independent** and the maintainer's call.
- No two phases touch the same files; the only hard ordering is Phase 2's precondition.

## Risks & Mitigations

- **Phase 1 re-breaks Retina rendering** (medium — it's the 3-bug-prone surface path). Mitigation: `wasm_smoke.sh` as a regression guard + mandatory real-GPU eyeball + the logical-size render as a one-revert rollback.
- **Headless verification gives false confidence for Phase 1** (high — SwiftShader masked all 3 prior bugs). Mitigation: treat headless as regression-only; require a real-GPU Retina render before declaring the crispness done.
- **Phase 2 builds a premature abstraction** (the exact reason it's deferred). Mitigation: hard precondition of a 3rd distinct example; design against all three call sites.
- **Phase 3 invents scope** (feature without a real need). Mitigation: VISION's playable-example bar; "no new feature" is an acceptable outcome.

## Success Criteria

- **Minimum:** Phase 1 ships — wasm renders crisply on Retina (real-GPU verified), `wasm_smoke.sh` + full `+1.88.0` gate green, no layout/clipping regression vs the logical-size render.
- Phase 2 stays explicitly deferred until a 3rd networked example exists (no premature helper).
- Phase 3 ends with either a new PLAN or a recorded "no new feature" conclusion.
- Phase 4 only touched with the maintainer's go.

## Quick Start

```bash
cd /Users/jkl/Projects/skeleton-engine
cat plans/handoffs/HANDOFF_networking-dogfood_deferred-polish_2026-06-09.md   # full context
./scripts/verify.sh   # baseline (or the 5 cmds with cargo +1.88.0); 311 lib + 5 server

# Phase 1 (monitor ON — real-GPU eyeball required):
#   edit src/app/window.rs (Resized + finish_init) + src/app/schedule.rs (viewport/scale)
#   regression guard:  scripts/wasm_smoke.sh
#   sharpness proof:    real browser on Retina (the seq-2 build path below), eyeball DPR=2
cargo run --example coin_race_server                                  # terminal 1
examples/games/coin_race/web/build.sh                                 # build wasm
python3 -m http.server 8080 --directory examples/games/coin_race/web  # terminal 2
open http://localhost:8080
```
