# Phase D done — predict_shooter real-play verified, RemoteEntities decision, web harness; chain A–D complete

**Date:** 2026-06-09
**Status:** COMPLETED — networking-dogfood seq-4 Phase D executed: real-play verified, design decision made + documented, web harness added. Chain Phases A–D complete. No release (no public API change).
**Bead(s):** none (`bd` unavailable)
**Epic:** VISION feature+example loop — networking depth + the RemoteEntities-richness decision
**Chain:** `networking-dogfood` seq `5`
**Parent:** `HANDOFF_networking-dogfood_client-prediction-shooter_2026-06-09.md` (seq 4)
**Prior chain:** `coin-race-example` (1) > `wasm-coin-race-v4.1` (2) > `deferred-polish` (3) > `client-prediction-shooter` (4) > this (5)
**Paired plan:** `PLAN_networking-dogfood_client-prediction-shooter_2026-06-09.md` (Phase D was the open part)

---

## Since Last Handoff

Seq 4 built the predict_shooter server (Phase A) + client (Phase B/C) and **deferred Phase D** (interactive feel validation + the RemoteEntities-promotion decision + web harness + release) to a monitor-on session, because the monitor was off. This session is that monitor-on session — it executed Phase D end to end:

- **Phase D feel/real-play (the deferred test) → done.** Verified the full loop in real play with 2 native clients.
- **RemoteEntities promotion decision → made + documented.** Outcome: keep the v4.3.0 minimal API; interpolation stays separate. (Answered design-doc open questions #1/#2.)
- **Web harness → added + headless-verified.** The shooter now ships to the browser too.
- **Release → determined unnecessary** (no public API change), so seq-4's planned "release" step is moot.
- **Discovered + handled a baseline drift:** local stable (rustc 1.95.0) `./scripts/verify.sh` now fails on a NEW clippy lint (`manual_is_multiple_of`); the 1.88.0 CI pin is green. Fixed the one occurrence in my server code; the pre-existing-example occurrences remain (separate small cleanup).

## Reference Documents

- `PLAN_networking-dogfood_client-prediction-shooter_2026-06-09.md` — the seq-4 plan; Phase D was its open part.
- `HANDOFF_networking-dogfood_client-prediction-shooter_2026-06-09.md` (seq 4) — full server/client build context, protocol, code analysis.
- `docs/REMOTE_ENTITIES_DESIGN.md` — now has the **"Decision after the 3rd example"** section (the conclusion below).
- `docs/NEXT_WORK.md` — deferred-item-2 updated to "keep minimal"; Phase-3 audit (Camera bounds, InputMap gamepad) still listed.
- `CLAUDE.md` — `+1.88.0` gate (memory `ci-toolchain-pin`); the local-stable-drift is exactly why that pin exists.
- Memory: `engine-v4.1-wasm-state` (v4.3.0 + seq-4 state), `playtest-windowed-examples` (the macOS osascript/caffeinate/screencapture playtest technique used here), `conversation-language-korean`, `doc-language-rule`.

## The Goal

Close out the client-prediction shooter (the 3rd distinct networked example): prove the prediction/reconciliation/interpolation loop works in real play, decide whether `RemoteEntities` should gain a richer (interpolation-aware) API, and ship the example to the browser. End state: chain Phases A–D complete; an informed, documented helper decision; shooter playable native + web.

## Where We Are

**Branch `main`, all pushed to `origin/main`. Tree clean. Engine v4.3.0 (unchanged — no release).**

- **Real-play verified** (monitor on): `predict_shooter_server` + 2 native `predict_shooter` clients. Both connect (`[N] connected` ×2). Each client renders its own white avatar + the other as a colored rival (`remote_color`: #1↔green, #2↔blue) at **mutually consistent positions**. Driving one client's input moved its predicted avatar AND the other client's interpolated view of it to the matching spot → input → predict → server → snapshot → remote render confirmed end-to-end.
- **Web harness** `examples/games/predict_shooter/web/` (`build.sh` + `index.html`, mirroring `coin_race/web/`, port 9003, entry `run_predict_shooter`). Headless wasm smoke: the tab loaded the wasm, connected, and rendered the HUD + avatar at DPR=2 (crisp). `pkg/` is gitignored.
- **RemoteEntities decision** recorded in `docs/REMOTE_ENTITIES_DESIGN.md` + `NEXT_WORK.md`: the minimal v4.3.0 API is unchanged.
- **Lint fix:** `server.rs` `s.tick % snap_every == 0` → `s.tick.is_multiple_of(snap_every)` (stable since 1.87 → compiles on the 1.88 pin too).
- **Gate (`+1.88.0`):** green — 313 lib + `predict_shooter_server` 7 + `client_net` 6.
- **Commits this session:** `986436a` (decision + lint), `bc7652e` (web harness + Phase-D docs).

## What We Tried (Chronological)

1. **Onboarded** from the seq-4 handoff + plan + `REMOTE_ENTITIES_DESIGN.md` + `client_net.rs` + `coin_race/web/` template. Ran the baseline.
2. **Baseline drift found:** `./scripts/verify.sh` (local stable **rustc 1.95.0**) failed to clippy-compile `predict_shooter_server` — new lint `clippy::manual_is_multiple_of` on `% snap_every == 0`. The `+1.88.0` gate (CI pin) passes (the lint didn't exist in 1.88). Confirmed `+1.88.0 test` of both examples passes (6 + 7).
3. **Scoped the lint:** 2 sites — mine (`server.rs:224`) + pre-existing `(i+j) % 2 == 0` in `minimap.rs`/`touch_demo.rs`/`split_screen.rs`. Fixed only my new code (`is_multiple_of`); left the pre-existing as a separate stable-lint item (CI 1.88 green; out of scope for "run the deferred test").
4. **Real-play stage 1 (static):** `caffeinate -u`, launched server + 2 clients, positioned windows side-by-side via osascript, `screencapture`. Result: both connect + see each other consistently. Strong end-to-end proof (connection, snapshot, RemoteEntities remote-spawn, render).
5. **Real-play stage 2 (driven):** focused a client (osascript focus-by-window-position actually errored, so input went to the frontmost = Player #2) and sent a System Events keystroke burst (WASD + Space). Result: #2's white avatar moved down-right (prediction) and #1's window showed the blue rival (#2) at the matching new position (interpolation/sync). Movement was small — macOS synthetic keys without `cliclick` register only a few "held" frames; bullets too short-lived (1.2s) to catch in a static shot. Loop proven; subjective feel left to the user.
6. **Decision** written into `REMOTE_ENTITIES_DESIGN.md` + `NEXT_WORK.md`; `is_multiple_of` fix; committed `986436a` (verified on both toolchains + 7 server tests).
7. **Web harness:** wrote `web/build.sh` + `web/index.html`; ran build.sh (release wasm + wasm-bindgen → `pkg/`, 8.3 MB). Headless smoke (server 9003 + http + headless Chrome DPR=2): tab connected + rendered. Committed `bc7652e` (+ HANDOFF/NEXT_WORK docs).

## Key Decisions

- **Keep `RemoteEntities` minimal; do NOT fold interpolation in.** The shooter shows interpolation (`client_net::Interp`) is *orthogonal* to the id→Entity lifecycle — they compose as parallel maps (`remote_players: RemoteEntities` + `player_interp: HashMap<id, Interp>`). The two snap-only examples (mp_client, coin_race) don't interpolate; coupling would force the concept on them. The deferral (seq 3) was correct; the 3rd example confirms the separation.
- **Don't promote `Interp`/`Prediction` to public helpers yet** — single call site; same discipline that deferred RemoteEntities richness. Revisit if a 2nd interpolating example appears (`engine::Interp`/`SnapshotBuffer<T>`).
- **No release** — the decision is "no public API change", so the shooter is just a new example (+ a clippy fix). Nothing to version/tag.
- **Fix only my lint occurrence, not the pre-existing ones** — the user's ask was the real-play test; CI (1.88) is green; engine-wide stable-lint cleanup is a separate, optional item.
- **Real-play over native (not the web harness first)** — most direct "run the deferred test"; used the `playtest-windowed-examples` osascript/screencapture technique.

## Evidence & Data

### Real-play captures (sent to the user)
- `predict_shooter_2clients.png` — 2 windows; #1 sees white(self)+blue(#2), #2 sees white(self)+green(#1), positions mirror-consistent. Server log: `connected` ×2.
- `predict_shooter_moved.png` — after driving #2: #2's white moved down-right; #1's blue rival moved to the matching spot.
- Headless wasm: `web/` tab → "You are Player #1 …" HUD + white avatar at DPR=2 (1600×1200 shot), server logged a connection.

### Commits this session
| Hash | Summary |
|---|---|
| `986436a` | docs+fix: Phase D — real-play verified; keep RemoteEntities minimal (+ is_multiple_of) |
| `bc7652e` | feat(examples): ship predict_shooter to the browser (web harness) + Phase D docs |

### Toolchain / lint
- `rustc` stable = **1.95.0**; CI pin = **1.88.0**. New lint `clippy::manual_is_multiple_of` (stable ≥1.95) flags `x % n == 0`. `u32::is_multiple_of` is stable since **1.87** → safe on the 1.88 pin.
- Remaining `manual_is_multiple_of` hits (verify.sh, stable only): `examples/{minimap,touch_demo,split_screen}.rs` — `(i + j) % 2 == 0` checkerboard shading. Pre-existing, unrelated.

### Test counts (unchanged this session)
313 lib · predict_shooter_server 7 · client_net 6.

## Code Analysis

- `client_net::Interp { samples: VecDeque<Sample{t,x,y}> }` and `client_net::Prediction` are example-local and *orthogonal* to `engine::RemoteEntities` — the shooter holds `RemoteEntities` (lifecycle) and per-id `Interp` (position history) as separate maps. This is the structural evidence for the "keep separate" decision.
- `engine::RemoteEntities<K>` (`src/network.rs`) unchanged at v4.3.0.
- `examples/games/predict_shooter/server.rs` — only change: `is_multiple_of`. Sim logic identical.

## Files Changed (this session)

### Examples
- **NEW** `examples/games/predict_shooter/web/{build.sh, index.html}` — browser harness (port 9003, `run_predict_shooter`). `pkg/` gitignored.
- `examples/games/predict_shooter/server.rs` — `% == 0` → `is_multiple_of` (stable-lint fix).

### Docs
- `docs/REMOTE_ENTITIES_DESIGN.md` — "Decision after the 3rd example" section (keep minimal; interpolation orthogonal; not promoted).
- `docs/NEXT_WORK.md` — deferred-item-2 → "keep minimal" + shooter ships native+web.
- `docs/HANDOFF.md` — dev-history row for the shooter.

## User Feedback & Preferences (this session)

- **"모니터 진짜 켜져있음. 지금 다시 완료못한 테스트 진행해"** — monitor confirmed on; run the deferred (real-play) test now.
- **"웹 하니스 추가 후 종료"** — finish Phase D by adding the web harness (chose this over "done now" / "play myself first").
- **"handoff 하고 커밋 푸쉬 하고 세션 클리어"** — wrap with a handoff + commit/push + close.
- **Standing prefs (memory):** conversation in Korean / artifacts English; `cargo +1.88.0` gate; release decisions are the user's; commit-to-main + push; subjective GUI feel they judge themselves.

## Where We're Going

The networking-dogfood chain (Phases A–D) is **complete**. No scheduled next step. Candidate follow-ups (none urgent):
1. **Subjective feel pass (user-driven):** play `predict_shooter` live (server + 2 windows, WASD/Space) to judge responsiveness / no-drift / interpolation smoothness; tune `INTERP_DELAY` (currently 0.1s) if needed. Loop correctness is already verified.
2. **Stable-lint cleanup:** `(i+j) % 2 == 0` → `.is_multiple_of(2)` in `minimap.rs`/`touch_demo.rs`/`split_screen.rs` so `./scripts/verify.sh` is green under local stable 1.95. Small, mechanical. (CI 1.88 unaffected.)
3. **Phase-3 audit gaps (each ~1 session):** Camera world-bounds clamping (`Camera.bounds`); `InputMap` gamepad binding. Both small, broadly useful, validated inside an existing example.
4. **2nd interpolating networked example** would unlock promoting `Interp` → `engine::Interp`/`SnapshotBuffer` (and revisit RemoteEntities open questions #3–#7).

## Risks & Blockers

- **Subjective feel unverified by automation** — synthetic input can't stress smoothness/rubber-banding; needs a human play pass. Loop correctness is unit-tested + real-play-verified.
- **`./scripts/verify.sh` red under local stable 1.95** (pre-existing-example `% 2 == 0` lint). CI pin 1.88.0 is green — use `cargo +1.88.0` for the gate, or do the small cleanup (#2 above).
- **Server has no hit detection** (deliberate scope cut) — bullets fly + expire only.

## Open Questions

- `INTERP_DELAY = 0.1s` optimal? (tune in a live feel pass.)
- Do the pre-existing `% 2 == 0` lints get cleaned up now or left until a stable-toolchain bump? (CI is green either way.)

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -6 && git status -s            # clean; v4.3.0; bc7652e tip
cargo +1.88.0 test --example predict_shooter         # client_net 6
cargo +1.88.0 test --example predict_shooter_server  # server 7
# (./scripts/verify.sh is RED under local stable 1.95 — pre-existing % 2 == 0 lint; use +1.88.0)

# Play it (subjective feel — the one thing automation couldn't judge):
cargo run --example predict_shooter_server   # terminal 1
cargo run --example predict_shooter          # terminals 2,3 — WASD/arrows move, Space shoot
# Browser: examples/games/predict_shooter/web/build.sh, then python3 -m http.server --directory web

# Key files: examples/games/predict_shooter/{protocol,server,client_net,predict_shooter}.rs + web/
#            src/network.rs (RemoteEntities)   docs/REMOTE_ENTITIES_DESIGN.md (the decision)

# Next action: optional — stable-lint cleanup (minimap/touch_demo/split_screen `% 2 == 0`),
#   or a Phase-3 gap (Camera world-bounds clamp / InputMap gamepad). Chain A-D is done.
```

## Session Status
Phase D complete: predict_shooter real-play-verified (native, 2 clients) + headless wasm; RemoteEntities decision made (keep minimal, interpolation orthogonal) + documented; web harness added; stable-lint fix in my server code. No release (no API change). networking-dogfood chain Phases A–D complete. All pushed to `origin/main`; tree clean.

## Session Closed
**Closed at:** 2026-06-09 14:01 KST
**Commit:** `bc7652e` (Phase D work) + this handoff — all on `origin/main`
**Session status:** Closed. networking-dogfood chain Phases A–D complete; no scheduled next step. Optional follow-ups: stable-lint cleanup, Camera world-bounds clamp / InputMap gamepad (Phase-3 gaps), a live feel pass.
