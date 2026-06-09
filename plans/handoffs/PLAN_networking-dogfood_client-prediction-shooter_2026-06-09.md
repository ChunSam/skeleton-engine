# Client-prediction networked shooter (3rd networked example) → richer RemoteEntities

**Date:** 2026-06-09
**Status:** PLANNED (executing the headless-verifiable parts; GUI/feel validation deferred — monitor off)
**Bead(s):** none (`bd` unavailable)
**Epic:** VISION feature+example loop — networking depth + the Phase-2 precondition
**Chain:** `networking-dogfood` seq `4` (continues seq 3)
**Context:** seq-3 Phase 2 shipped the *minimal* `RemoteEntities<K>` and deferred the *richer* version (interpolation/prediction/reconciliation) until a **3rd distinct networked example** reveals the shape (see `docs/REMOTE_ENTITIES_DESIGN.md`). The user chose that 3rd example: a **client-prediction shooter**. This both deepens networking coverage (the engine currently has only relay + simple authoritative) and provides the design signal.

---

## Problem Statement

Build a top-down networked shooter that demonstrates the three pillars of responsive game networking — **client-side prediction**, **server reconciliation**, and **remote-entity interpolation** — against an authoritative server. Then make an informed decision on whether/how to extend `RemoteEntities` (the seq-3 Phase 2 richer goal).

## Hard constraint this session: monitor OFF → feel-validation deferred

Prediction/reconciliation/interpolation correctness is partly *subjective* (responsiveness, rubber-banding, jitter) and needs interactive real-play on a real GPU. With the monitor off, this session does only what is **headless-verifiable**:
- server logic via a `tungstenite` probe + unit tests (deterministic);
- client **compiles** (native + wasm) + headless connect + non-blank render (`wasm_smoke`-style);
- the **pure logic** of prediction/reconciliation/interpolation as unit tests (replay determinism, interpolation math).

Explicitly DEFERRED to a monitor-on session: the interactive feel (input responsiveness, convergence-without-drift, smooth remote motion) and any release. **Do not release or declare the example "done" until real-play passes.**

## Design (protocol + model)

Determinism rule: client and server integrate movement with the **same formula and fixed timestep** so the client can replay inputs exactly. `pos += dir.normalized() * SPEED * FIXED_DT`, `FIXED_DT = 1/60`, shared `SPEED`.

**Client → Server**
- `{"t":"in","seq":<u32>,"mx":<f32>,"my":<f32>,"fire":<bool>}` — one input command per client tick. `seq` is monotonic; `mx,my` ∈ [-1,1] movement intent.

**Server → Client**
- `{"t":"welcome","id":<usize>}` on connect.
- `{"t":"snap","tick":<u32>,"players":[{"id":<usize>,"x":<f32>,"y":<f32>}],"bullets":[{"id":<usize>,"x":<f32>,"y":<f32>}],"ack":<u32>}` — periodic snapshot; `ack` = the last input `seq` the server has applied **for the receiving client** (per-client ack).
- `{"t":"bye","id":<usize>}` on disconnect.

**Server model:** fixed-tick loop. Per client: a queue of received input commands; each tick apply the next pending command(s), record `last_seq`. Bullets: on a `fire` command spawn a bullet (pos+velocity from the firing player), integrate each tick, despawn on bounds/lifetime. Broadcast a snapshot every N ticks (e.g. 30 Hz). Authoritative.

**Client model:**
- *Local player (predicted):* each tick, sample input → command{seq, mx, my, fire} → apply locally immediately → push to `pending` → send. On `snap`: set local pos = server's pos for self, drop `pending` with `seq <= ack`, **replay** remaining `pending` → corrected predicted pos.
- *Remote players + bullets (interpolated):* buffer the last ≥2 snapshots; render at `now - INTERP_DELAY` (≈100 ms) by lerping each remote entity between the two bracketing snapshots. This is where `RemoteEntities` is stressed (spawn/despawn by id **plus** a per-entity position history).

## Plan

### Phase A — protocol + authoritative tick server  *(headless-verifiable now)*
**Goal:** `predict_shooter_server` ([[example]] binary, modeled on `coin_race/server.rs`: raw `tungstenite`, dependency-free RNG) running a fixed-tick authoritative sim with per-client seq'd inputs + bullets + snapshot broadcast.
- Protocol types (serde) shared via a small module in the example dir (client + server both include).
- Per-client input queue + `last_seq`; fixed-tick integration; bullet spawn/expire; 30 Hz snapshot with per-client `ack`.
- **Verify (headless):** unit tests for the sim step (deterministic movement, ack advances, bullet lifetime); a throwaway `tungstenite` probe (like coin_race) asserting welcome→input→snap(ack increments)→bye. `+1.88.0` gate.

### Phase B — client prediction + reconciliation  *(code now; FEEL deferred)*
**Goal:** local player predicts immediately and reconciles to server state without drift.
- Fixed-tick input sampling (seq), local apply, `pending` buffer, send; on `snap` snap-to-ack + replay pending.
- **Verify now (headless):** unit-test the reconciliation function — given a server pos at `ack` + a `pending` list, replaying yields the same pos as continuous local simulation (determinism); client compiles native+wasm; headless connect + non-blank frame.
- **Deferred (monitor on):** input feels responsive (no perceived lag) AND no rubber-banding/drift under simulated latency.

### Phase C — remote interpolation  *(code now; FEEL deferred)*
**Goal:** remote players + bullets move smoothly despite the 30 Hz snapshot rate.
- Per-remote-entity snapshot buffer; render at `now - INTERP_DELAY` lerping between two snapshots; spawn/despawn by id.
- **Verify now (headless):** unit-test the interpolation math (lerp between two timestamped samples at a given render time; clamp at ends); compiles; headless render non-blank.
- **Deferred (monitor on):** remote motion is smooth (no jitter/teleport) and acceptably delayed.

### Phase D — decide + (maybe) extend RemoteEntities; verify; release  *(decision now; release deferred)*
**Goal:** with three call sites (`mp_client` snap, `coin_race` snap, shooter interpolated), decide whether the interpolation buffer belongs in `RemoteEntities` (or a companion `InterpolatedRemoteEntities`) or stays example-side.
- If a clean shared shape emerges → extend/add the helper, migrate, update `docs/REMOTE_ENTITIES_DESIGN.md`. If not → record that the example-side interpolation is game-specific and keep the minimal helper (a legitimate outcome — the deferral may have been correct).
- **Verify now:** `+1.88.0` gate + unit tests; `wasm_smoke`-style headless for the shooter if it targets wasm.
- **Deferred (monitor on):** real-play of all affected examples; then CHANGELOG + HANDOFF + NEXT_WORK + **release** (minor — new example, possibly new helper API).

## Anti-Goals
- **Do NOT release or mark the example done without real-play.** Prediction/interpolation correctness is not provable headless.
- **Do NOT over-build the shooter.** Movement prediction is the core; bullets are server-authoritative + interpolated (not predicted). No hit-detection netcode beyond server-side bullet/player overlap. No lag-compensation/rewind (note as a future item).
- **Do NOT extend `RemoteEntities` speculatively.** Extend only if the 3rd call site makes the shared shape clear; otherwise keep it minimal and say so.
- **Do NOT change the existing minimal `RemoteEntities` API** in a breaking way; additive only.

## Dependencies & Order
- A → B → C (client needs the server + protocol). D last.
- Phases A–C code + headless verification proceed now; all *feel* validation and the release wait for a monitor-on session.

## Risks & Mitigations
- **Prediction/reconciliation drift** (high difficulty). Mitigation: shared deterministic integration constants; unit-test replay-equals-continuous; defer feel to real-play.
- **Building blind (monitor off)** — subtle netcode bugs won't show headless. Mitigation: maximize unit-test coverage of pure logic; treat the headless smoke as "runs + connects" only; gate the release on real-play.
- **RemoteEntities extension premature** — mitigation: Phase D may legitimately conclude "keep minimal".

## Success Criteria
- **This session (headless):** server + client compile (native+wasm); server sim + reconciliation + interpolation **logic unit-tested**; probe confirms the protocol; headless connect + non-blank render; `+1.88.0` gate green. A clear Phase-D recommendation written.
- **Next (monitor-on):** real-play confirms responsive prediction, no drift, smooth interpolation; then release.

## Quick Start
```bash
cd /Users/jkl/Projects/skeleton-engine
# Phase A first — model the server on the existing one:
sed -n '1,60p' examples/games/coin_race/server.rs
./scripts/verify.sh   # baseline (or +1.88.0); 313 lib
# new example dir (proposed): examples/games/predict_shooter/  (server.rs + predict_shooter.rs + protocol)
```
