# Client-prediction shooter (server + client built, headless-verified) + v4.2.0/v4.3.0 released + headless polish batch

**Date:** 2026-06-09
**Status:** IN PROGRESS — predict_shooter server (Phase A) + client (Phase B/C) built and headless-verified; **interactive feel + web harness + release are deferred (monitor OFF)**. Earlier this session: v4.2.0 + v4.3.0 shipped.
**Bead(s):** none (`bd` unavailable in this project)
**Epic:** VISION feature+example loop — networking depth + the Phase-2 (richer RemoteEntities) precondition
**Chain:** `networking-dogfood` seq `4`
**Parent:** `HANDOFF_networking-dogfood_deferred-polish_2026-06-09.md` (seq 3)
**Prior chain:** `coin-race-example_2026-06-08` (1) > `wasm-coin-race-v4.1_2026-06-08` (2) > `deferred-polish_2026-06-09` (3) > this (4)
**Paired plan:** `PLAN_networking-dogfood_client-prediction-shooter_2026-06-09.md`

---

## Since Last Handoff

Seq 3 (`deferred-polish`) was written as a docs/planning deliverable and laid out four deferred follow-ups (Phase 1 wasm Retina crispness, Phase 2 reusable remote-entity helper, Phase 3 new-breadth audit, Phase 4 rust-survivors docs). This session then **executed** them and went further:

- **Phase 1 → shipped as v4.2.0.** wasm Retina crispness implemented + real-GPU verified (monitor was on at the time), released.
- **Phase 2 (minimal) → shipped as v4.3.0.** `engine::RemoteEntities<K>` added, two examples migrated, released. The *richer* version stayed deferred, needing a 3rd networked example.
- **Phase 2 follow-on chosen by the user → the 3rd networked example = a client-prediction shooter (this seq 4).** Built server (Phase A) + client (Phase B/C).
- **Phase 3 audit → done** (recorded in NEXT_WORK; conclusion below).
- **Mid-session the monitor went OFF**, so all GUI/feel validation + the predict_shooter release are now deferred. The user explicitly asked to do only headless-verifiable work ("화면 안켜져 있어도 가능한 작업", "테스트는 나중에").

## Reference Documents

- `docs/VISION.md` — the feature+example loop ("a feature isn't done until a small playable example exercises it in real play"; "fix only the gap the example hits").
- `PLAN_networking-dogfood_client-prediction-shooter_2026-06-09.md` — the seq-4 plan (Phases A–D, with GUI/feel marked deferred).
- `docs/REMOTE_ENTITIES_DESIGN.md` — the deferred "richer RemoteEntities" design notes + open questions + 3rd-example criterion. **The shooter is that 3rd example; its `client_net::Interp` is the candidate interpolation buffer to promote.**
- `docs/NEXT_WORK.md` — has the Phase-3 audit conclusion + the deferred-follow-ups list.
- `docs/CHANGELOG.md` — 4.2.0 + 4.3.0 entries added this session.
- `CLAUDE.md` — conventions; `+1.88.0` gate (memory `ci-toolchain-pin`); CLAUDE.md ≤200 lines.
- Memory: `engine-v4.1-wasm-state` (updated to v4.3.0), `conversation-language-korean`, `doc-language-rule`, `subagent-usage-preference`, `playtest-windowed-examples`, `rust-survivors-engine-pin`.

## The Goal

Deepen the engine's networking coverage (it had only relay + simple authoritative) with a **client-prediction shooter** — the canonical demo of prediction + server reconciliation + remote interpolation — which doubles as the 3rd distinct networked example needed to decide whether `RemoteEntities` should gain a richer (interpolation-aware) API. End state: a playable, feel-correct networked shooter; an informed decision on the helper; released.

## Where We Are

**Branch `main`, all pushed to `origin/main`.** Engine package **v4.3.0**, tags v4.0.0/v4.1.0/v4.2.0/v4.3.0.

- **predict_shooter server (Phase A) DONE** — `examples/games/predict_shooter/server.rs` (`predict_shooter_server` [[example]]): fixed-tick (60 Hz) authoritative WebSocket server (raw `tungstenite`, xorshift), per-client seq'd input queue, movement integration, bullets (spawn on fire / integrate / cull), 30 Hz snapshots with **per-client `ack`** (last applied input seq). **7 unit tests** + a (throwaway, since deleted) tungstenite probe confirmed `welcome → 3 inputs → snapshot ack=3`.
- **predict_shooter client (Phase B/C) DONE (code + headless)** — `examples/games/predict_shooter/predict_shooter.rs` (`predict_shooter` [[example]], dual-target native+wasm) + `client_net.rs` (pure netcode core).
  - `client_net::Prediction` — local apply immediately + buffer + `reconcile(server_pos, ack)` = drop acked + replay unacked via `step_position`.
  - `client_net::Interp` — per-remote snapshot buffer; `sample(render_time)` lerps between two stamps (renders `INTERP_DELAY=0.1s` in the past); clamps at ends; ignores out-of-order stamps.
  - **6 unit tests** for the core (prediction==step_position, reconcile drops+replays==continuous chain, all-acked snaps to server, interp lerp/clamp/empty/out-of-order).
  - Wires both into ECS; remote players + bullets use `engine::RemoteEntities<usize>` for the id→Entity lifecycle (bullets despawned when absent from a snapshot).
- **`protocol.rs`** — shared (via `#[path]` in both server + client): deterministic `step_position` (the single source of truth for client/server convergence), constants (`FIXED_DT=1/60`, `MOVE_SPEED=240`, `FIELD_W/H=800/600`, `SNAPSHOT_HZ=30`, `BULLET_*`, `FIRE_COOLDOWN`), wire types (`ClientMsg::Input`, `ServerMsg::{Welcome,Snap,Bye}` — serde tag `"t"`).
- **v4.2.0 (`dce44ae`+`3bbb813`)** — crisp wasm Retina rendering: drawing buffer = logical × DPR (uniform, capped 2048), CSS box stays logical; `WASM_LOGICAL_SIZE` thread-local (NOT WindowConfig — see trap below). Real-GPU verified.
- **v4.3.0 (`9dce3c0`+`499675e`)** — `engine::RemoteEntities<K>` (get_or_spawn/get/contains_key/remove(despawn)/clear/len/is_empty/iter); `mp_client`+`coin_race` migrated.
- **Gate (`cargo +1.88.0`):** fmt / clippy --all-targets / clippy --target wasm32 (lib + example) / build --target wasm32 (lib+bins) / test --all-targets / doc — all green. **313 lib tests + 6 client_net + 7 predict_shooter_server**, native + wasm build.
- **Working tree clean. No stray processes.**

## What We Tried (Chronological)

1. **wasm Retina crispness (v4.2.0).** Re-enabled the native HiDPI model on wasm: buffer = logical×DPR (uniform clamp ≤2048), CSS = logical, `ViewportSize` stays logical, `DisplayScaleFactor = buffer/logical` (so glyphon text renders at device res). **Trap hit:** first attempt used `WindowConfig` as the logical source → coin_race rendered a 1280 viewport in an 800 canvas. Root cause (found via a `web_sys::console::log_1` debug dump): **a scene `Replace` resets the World and reverts `WindowConfig` to its 1280×720 default**, so it's unreliable. Fix: read logical from the **authored `<canvas>` width/height attributes**, stored in a `WASM_LOGICAL_SIZE` thread-local in `app.rs`. Verified on a real-GPU Retina browser (Chrome Metal/ANGLE) + headless DPR=2 (coin_race 800→1600, run_demo 1280→2048 clamp path, aspect preserved). Released.
2. **RemoteEntities minimal helper (v4.3.0).** Extracted the `HashMap<id,Entity>` lifecycle into `engine::RemoteEntities<K>`; migrated mp_client + coin_race (players + coins). Caught: nothing (clean). Left the richer version deferred + wrote `docs/REMOTE_ENTITIES_DESIGN.md`. Released.
3. **Chose the 3rd networked example.** Asked the user; they picked **client-prediction shooter** (over interest-managed many-entity / binary-protocol). Wrote the seq-4 PLAN.
4. **Monitor went OFF.** Re-scoped to headless-only; deferred feel + release.
5. **Phase A — server.** Built the fixed-tick authoritative server + shared protocol. Modeled on `coin_race/server.rs` but added a dedicated tick thread (coin_race is event-driven). 5 sim unit tests; verified end-to-end with a throwaway tungstenite probe (`PROBE_OK id=1 ack=3`). Committed `75c68e9`.
6. **Phase B/C — client.** Wrote `client_net.rs` (Prediction + Interp, pure + 6 tests) and `predict_shooter.rs` (windowed client wiring). Native + wasm build green.
7. **Headless batch #1–5 (user asked for all five).** Launched 3 parallel Sonnet subagents (review / breadth-audit / REFERENCE.html-audit) while building the client.
   - **#2 review** found **1 real bug**: server sent `Welcome` *under the mutex lock* → a slow client could stall the 60 Hz tick thread (worse than coin_race because of the tick thread). Fixed: send Welcome **outside** the lock. RemoteEntities + the wasm Resized handler reviewed clean. The "scale-factor axis asymmetry" nit doesn't apply (my clamp is *uniform*; only a sub-pixel rounding artifact on extreme aspect ratios).
   - **#5** added 2 server edge-case tests (field clamp, multi-player snapshot) + invariant comments (ordered-delivery seq, snapshot-interval assumption).
   - **#3 audit** conclusion recorded in NEXT_WORK.
   - **#4** REFERENCE.html updated.
   - Committed `103662d` (code) + `4c95072` (docs).

## Key Decisions

- **wasm logical size = authored `<canvas>` attributes, not `WindowConfig`** — WindowConfig is reverted by scene resets; the canvas attributes are stable. (Stored in `WASM_LOGICAL_SIZE` thread-local.)
- **RemoteEntities stays minimal until the 3rd example proves the richer shape** — the shooter is that example; its `Interp` buffer is the concrete promotion candidate (Phase D decides).
- **Server determinism via a shared `step_position`** — client prediction only converges if it replays with the exact server formula + fixed timestep; so `step_position` lives in the shared `protocol.rs`.
- **Bullets are server-authoritative + interpolated, NOT predicted** — keeps the client tractable (prediction is for the local player's movement only); bullets exercise RemoteEntities spawn/despawn + interpolation heavily.
- **Send `Welcome` outside the mutex** (from #2 review) — the fixed-tick thread needs the lock 60×/s; a network send under the lock would freeze the sim on one slow client. Welcome carries only the id, so no state race (unlike coin_race's hello, which is deliberately under-lock).
- **Headless scope under monitor-off** — build + unit-test the netcode *math*; defer the interactive *feel* + web harness + release. Prediction/interpolation structure (tested) is low-rework-risk; only tuning params (`INTERP_DELAY`, smoothing) need playtest.
- **#4 REFERENCE.html scoped to the new networking API** — corrected the subagent's NetworkEvent draft (it had wrong variant names) to the real API; left JointHandle (v4.0.0) + wasm-render notes as a known remaining gap rather than risk inaccurate drafts.

## Evidence & Data

### Commits this session (chronological)
| Hash | Summary |
|---|---|
| `dce44ae` | feat(wasm): render at device resolution for crisp Retina output |
| `a44b444` | docs: mark wasm Retina crispness (seq-3 Phase 1) done |
| `3bbb813` | release: v4.2.0 — crisp wasm Retina rendering |
| `9dce3c0` | feat(net): add RemoteEntities<K> remote-entity lifecycle helper |
| `499675e` | release: v4.3.0 — RemoteEntities helper |
| `75c68e9` | feat(examples): predict_shooter authoritative server + protocol (Phase A) |
| `103662d` | feat(examples): predict_shooter client — prediction/reconciliation/interp (Phase B/C) |
| `4c95072` | docs: Phase 3 breadth audit + REFERENCE.html networking/RemoteEntities |

(Earlier same session, seq-3 docs/planning: `c176f24`, `bb98d62`, `da96b75`, `0e9f067` — repo-wide Korean→English + wasm_smoke + seq-3 plan.)

### Test counts
- Lib: **313** (was 311 pre-session; +2 RemoteEntities).
- `predict_shooter_server`: **7** (deterministic step/ack, replay==step_position chain, bullet spawn/expire, fire cooldown, protocol round-trip, field clamp, multi-player snapshot).
- `client_net` (via `--example predict_shooter`): **6** (prediction immediate==step_position, reconcile drops+replays==continuous chain, all-acked snaps to server, interp lerp, interp clamp+empty, interp out-of-order).

### #2 review findings (resolved)
| # | Severity | Finding | Resolution |
|---|---|---|---|
| 1 | **bug** | server Welcome sent under mutex lock → stalls tick thread on a slow client | fixed (send outside lock) |
| 2 | risk | fire_cd decays once/tick vs multiple inputs/tick | comment added (correct under 1 input/tick) |
| 3 | nit | snap_every assumes integer tick:snapshot ratio | comment added |
| 4 | nit | last_seq overwrite assumes ordered delivery | comment added (TCP/WS ordered) |
| 5 | risk | mutex poisoning if a thread panics under lock | latent (no panic path under lock); not fixed |
| 7 | nit | scale_factor width-axis only | N/A — clamp is uniform; sub-pixel rounding only |
| — | clean | RemoteEntities; wasm Resized handler | — |

### #3 Phase-3 audit conclusion
Breadth is genuinely complete (every subsystem has a playable example). No compelling *new subsystem*. Two small, broadly-useful **API gaps** worth one session each (recorded in NEXT_WORK):
- **Camera world-bounds clamping** — `Camera` lacks `bounds: Option<Rect>`; every scrolling game reinvents the ~10-line clamp (cf. `lit_dungeon` `CameraFollowSystem`). Very low effort + unit test.
- **`InputMap` gamepad binding** — keyboard-only; `GamepadState` is separate; gamepad games dispatch by hand. Additive `bind_gamepad`/axis.
Rejected as higher-effort/narrower/deferred: tilemap autotiling, runtime tilemap mutation, save migration, data-driven anim/particle assets, diagonal pathfinding, RTL/per-locale fonts, audio ducking.

### #4 REFERENCE.html
Version header bumped **v2.0.0 → v4.3.0**; new **Networking** section added (NetworkClient connect/send, NetworkEvent read with correct variants, RemoteEntities + method table, "deliberately minimal" note, example pointers) + TOC/sidebar entries. **Still missing (known gap):** JointHandle/joint methods (v4.0.0), wasm default-font + Retina notes (v4.1/4.2).

### Protocol (the wire contract)
- C→S: `{"t":"in","seq":<u32>,"mx":<f32>,"my":<f32>,"fire":<bool>}`
- S→C: `{"t":"welcome","id":<usize>}` · `{"t":"snap","tick":<u32>,"players":[{id,x,y}],"bullets":[{id,x,y}],"ack":<u32>}` · `{"t":"bye","id":<usize>}`
- `ack` = last input seq the server applied **for the receiving client** → drives reconciliation.

## Code Analysis

- `protocol::step_position(x,y,mx,my) -> (f32,f32)` — normalizes diagonal (len>1), `pos += dir*MOVE_SPEED*FIXED_DT`, clamps to `[PLAYER_HALF, FIELD-PLAYER_HALF]`. **The shared determinism anchor.**
- `Prediction { x, y, next_seq, pending: VecDeque<PendingInput> }`: `predict(mx,my)->seq` (apply+buffer), `reconcile(sx,sy,ack)` (drop `seq<=ack`, replay rest from server pos). `next_seq` uses `wrapping_add` (wraps after ~2 yrs @60Hz; demo-acceptable).
- `Interp { samples: VecDeque<Sample{t,x,y}> }`: `push(t,x,y)` (monotonic, caps 8), `sample(rt)->Option<(f32,f32)>` (clamp at ends, lerp between bracketing samples). **This is the candidate to promote into a richer RemoteEntities (Phase D).**
- Server tick thread: `sleep(FIXED_DT)`→`lock`→`step()`→every `snap_every=2` ticks `broadcast_snapshot()` (per-client `ack`). Per-client read thread pushes `InputCmd` into the player's queue under the same lock.
- wasm DPR model (`app/window.rs finish_init` + `Resized`, `app/schedule.rs`, `app.rs`): `scale = dpr.min(2048/lw).min(2048/lh)` (uniform), `buf = logical*scale`, CSS = logical, `ViewportSize = config/scale = logical`, `DisplayScaleFactor = scale`. Logical from `WASM_LOGICAL_SIZE`.
- `engine::RemoteEntities<K: Eq+Hash>` in `src/network.rs` — pure lifecycle map; `remove`/`clear` despawn via `World`.

## Files Changed

### Source (engine lib)
- `src/app/window.rs`, `src/app/schedule.rs`, `src/app.rs` — wasm DPR-aware crispness (v4.2.0).
- `src/network.rs` — `RemoteEntities<K>` + tests (v4.3.0); `src/lib.rs` re-export.
- ~110 `src/` files earlier this session — Korean→English (committed before this handoff's window, in `0e9f067`).

### Examples
- **NEW** `examples/games/predict_shooter/{protocol.rs, server.rs, client_net.rs, predict_shooter.rs}` — the shooter (server + shared protocol + client netcode core + client).
- `examples/mp_client.rs`, `examples/games/coin_race/coin_race.rs` — migrated to RemoteEntities (v4.3.0).
- `Cargo.toml` — `predict_shooter_server` + `predict_shooter` [[example]] entries; version 4.1.0→4.2.0→4.3.0.

### Docs
- **NEW** `docs/REMOTE_ENTITIES_DESIGN.md`, `PLAN_networking-dogfood_client-prediction-shooter_2026-06-09.md`.
- `docs/NEXT_WORK.md` (Phase-1 done, Phase-2 minimal done, Phase-3 audit), `docs/HANDOFF.md` (dev-history rows), `docs/CHANGELOG.md` (4.2.0/4.3.0), `REFERENCE.html` (version + networking), `CLAUDE.md` (version + wasm_smoke ref).

## User Feedback & Preferences (this session)

- **"계획 실행해" / "1번에서 5번까지 계획세워서 진행"** — wants me to plan then execute, decisively; comfortable with large batches.
- **"모니터 꺼져있음 테스트는 나중에"** + **"화면 안켜져 있어도 가능한 작업들은 뭐가 있어?"** — defer all GUI/feel validation when the monitor is off; do headless-verifiable work now. (Twice.)
- **Tag/release decisions are the user's** — chose "move v4.1.0 tag", "4.2.0 발행", "4.3.0 발행". Confirm before pushed-tag moves/releases.
- **"일단 a로 진행하고 나중에 한번 더 깊게 고민할 수 있게 내용 남겨놔줘"** — prefers the minimal/safe option now + leave design notes for a future deep-dive (→ `REMOTE_ENTITIES_DESIGN.md`).
- **Chose client-prediction shooter** as the 3rd networked example.
- **Standing prefs (memory):** conversation in **Korean**, artifacts in **English**; `cargo +1.88.0` gate; verify before declaring done; subagents (Sonnet) for parallel work; commit-to-main + push for landing.

## Where We're Going

**Phase D — requires the monitor ON (real-play feel):**
1. **Web harness for predict_shooter** — `examples/games/predict_shooter/web/` (index.html calling `run_predict_shooter` + a `build.sh`), mirroring `coin_race/web/`. Then a headless wasm smoke (optional) + the real-play.
2. **Real-play feel validation** — run the server + 2 client windows: confirm (a) local movement feels instant (prediction, no input lag), (b) no rubber-banding/drift (reconciliation), (c) remote players + bullets move smoothly (interpolation), tuning `INTERP_DELAY` if needed.
3. **Phase D decision — promote `client_net::Interp` into RemoteEntities?** Now that a 3rd example exists, answer the open questions in `docs/REMOTE_ENTITIES_DESIGN.md` (interpolation buffer shape). Either add an `InterpolatedRemoteEntities` / extend `RemoteEntities`, migrate, OR conclude interpolation stays game-specific (record it).
4. **Release** the shooter (+ any helper change) — minor version, CHANGELOG/HANDOFF/NEXT_WORK, tag.

**Independent, headless-doable any time:** the two API gaps from the Phase-3 audit (Camera world-bounds clamp; InputMap gamepad) — each a small session, validated inside an existing example.

## Risks & Blockers

- **Building netcode blind (monitor off)** — prediction/interpolation correctness is partly subjective; the *math* is unit-tested but rubber-banding/jitter only show in real play. Don't release before playtest.
- **`predict_shooter` has no web harness yet** — can't headless-smoke the client (needs index.html+build, like coin_race). Native client opens a window (not headless on a monitor-off Mac).
- **Server has no bullet/player hit detection** (deliberate scope cut) — bullets fly + expire only; note as future if the shooter wants scoring.
- **Mutex poisoning** (review #5) — latent; no panic path under the lock currently.

## Open Questions

- Does `client_net::Interp` generalize into a public `RemoteEntities` interpolation API, or stay game-side? (Phase D, answered by the real-play + the design-doc questions.)
- `INTERP_DELAY=0.1s` — right value? (tune in real-play.)
- Promote the shooter to wasm/browser (web harness) as part of Phase D, or native-only first?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
cat plans/handoffs/PLAN_networking-dogfood_client-prediction-shooter_2026-06-09.md   # the plan (Phase D)
cat docs/REMOTE_ENTITIES_DESIGN.md   # the helper-promotion decision context
git log --oneline -8 && git tag | grep v4   # v4.3.0 is current

# Verify current state (headless)
./scripts/verify.sh                          # or the 5 cmds with cargo +1.88.0; 313 lib
cargo +1.88.0 test --example predict_shooter         # client_net: 6
cargo +1.88.0 test --example predict_shooter_server  # server: 7

# Key files
#   examples/games/predict_shooter/{protocol.rs, server.rs, client_net.rs, predict_shooter.rs}
#   src/network.rs (RemoteEntities)   docs/REMOTE_ENTITIES_DESIGN.md

# Next action (MONITOR ON): add examples/games/predict_shooter/web/ (mirror coin_race/web/),
#   then run the server + 2 clients and validate prediction feel / no-drift / smooth interpolation:
cargo run --example predict_shooter_server      # terminal 1
cargo run --example predict_shooter             # terminals 2, 3 (native windows)
```

## Session Status
predict_shooter server (Phase A) + client (Phase B/C) built and headless-verified (13 example tests + full `+1.88.0` gate, native+wasm); v4.2.0 + v4.3.0 released; headless polish batch (#1–5) done. All pushed to `origin/main`; tree clean. Phase D (feel validation + helper-promotion decision + release) deferred to a monitor-on session.

## Session Closed
**Closed at:** 2026-06-09 12:27 KST
**Commit:** `5e9df1b` (handoff) — session work in `dce44ae`..`5e9df1b`, all on `origin/main`
**Session status:** Handed off — resume at **Phase D (monitor ON)**: `predict_shooter/web/` harness → real-play feel → `Interp`→RemoteEntities decision → release.
