# `engine::SnapshotBuffer<T: Lerp>` promoted + Orbital Dodger (2nd interpolating example) + predict_shooter migrated (v4.4.0)

**Date:** 2026-06-09
**Status:** COMPLETED — the seq-7 "Where We're Going" item #4 (a 2nd interpolating networked example) was built, and it triggered the deferred promotion of `client_net::Interp` → public, generic `engine::SnapshotBuffer<T: Lerp>`. predict_shooter migrated onto it. All committed + **pushed to `origin/main`** (`2195b87..f62df90`, 3 commits). Engine **bumped 4.3.1 → 4.4.0** (additive public API). Tree clean. Full `+1.88.0` gate green (**339 lib tests**).
**Bead(s):** none (`bd` unavailable — tracked with TaskCreate this session)
**Epic:** VISION feature+example loop — networking dogfood (interpolation as a first-class, reusable concern)
**Chain:** `networking-dogfood` seq `8`
**Parent:** `HANDOFF_networking-dogfood_interp-gilrs-fix_2026-06-09.md` (seq 7)
**Prior chain:** `coin-race-example` (1) > `wasm-coin-race-v4.1` (2) > `deferred-polish` (3) > `client-prediction-shooter` (4) > `phase-d-realplay` (5) > `phase3-polish` (6) > `interp-gilrs-fix` (7) > this (8)

---

## Stale References

Parent (seq 7) identifiers that changed this session — next session beware:

- **`client_net::Interp`** (in `examples/games/predict_shooter/client_net.rs`) — **DELETED**. Promoted to `engine::SnapshotBuffer<T: Lerp>` in `src/network.rs`. predict_shooter now uses `engine::SnapshotBuffer<Vec2>`. The example's `Sample` struct + the 3 `interp_*` unit tests were removed with it.
- **`Interp::sample` return type** — was `Option<(f32, f32)>`; the generic `SnapshotBuffer::sample` returns `Option<T>` (so `Option<Vec2>` at the predict_shooter call site).
- **Engine version** — `4.3.1` → **`4.4.0`** (`Cargo.toml` + `Cargo.lock`).
- **`predict_shooter`'s `Prediction`** — UNCHANGED, still example-local (deliberate; it's a local concern, not remote-entity state).
- All seq-7 identifiers (`INTERP_DELAY_DEFAULT = 0.06`, `poll_gilrs` catch_unwind guard, gilrs 0.11.2, `Camera::bounds`, `InputMap` gamepad API) UNCHANGED.

## Since Last Handoff

Parent (seq 7) left four optional, environment-gated "Where We're Going" items: (1) survivor gamepad live-validation (macOS-blocked), (2) a native macOS GCController backend (large), (3) a rust-survivors engine-pin bump, (4) **a 2nd interpolating networked example** (would unlock promoting `client_net::Interp`). The user chose **#4**.

- **#4 done + the promotion it unlocked done.** Built `orbital_dodger` (interpolation-only) AND promoted `Interp` → `engine::SnapshotBuffer<T: Lerp>`, migrating predict_shooter. This closes `docs/REMOTE_ENTITIES_DESIGN.md` open question **#1 (interpolation)**.
- **Design-doc trigger fired exactly as written.** seq-3's `REMOTE_ENTITIES_DESIGN.md` said: "If a *second* interpolating example appears, `engine::Interp` / `SnapshotBuffer<T>` is a clean additive helper to extract then." This session is that second example.
- **Better-than-planned shape.** The plan considered a hardcoded copy; instead the buffer is **generic over the existing `Lerp` trait** (`src/timeline.rs`), because orbital_dodger needs to interpolate a non-`Vec2` channel (spin angle, `f32`) — strong evidence the generic shape is right, not over-fit.
- **Items #1–#3 still open/untouched** (gamepad live-validation, GCController backend, rust-survivors pin) — none chosen this session.

## Reference Documents

- `docs/REMOTE_ENTITIES_DESIGN.md` — the design rationale; now has a "SnapshotBuffer promoted" section closing open Q#1. #3–#7 still open.
- `docs/VISION.md` — the feature+playable-example loop (the example IS the acceptance test).
- `docs/NEXT_WORK.md` — added the **seq-8** section.
- `CLAUDE.md` — module map (network row updated); verification gate; `playtest-windowed-examples` + `ci-toolchain-pin` memory pointers.
- `plans/handoffs/PLAN_networking-dogfood_deferred-polish_2026-06-09.md` (seq-3) — where the `RemoteEntities`/interpolation deferral originated; `NEXT_WORK.md` "Deferred follow-ups #2" tracked "`engine::SnapshotBuffer<T>` is a clean additive helper to extract" — this session executed it.
- Templates mirrored: `examples/games/predict_shooter/{server,protocol,client_net,predict_shooter}.rs` + `web/` (the structural blueprint for the new example's server/protocol/wasm-harness).

## The Goal

Close the loop the seq-3 design doc opened: a *second* interpolating networked example is the named trigger to promote predict_shooter's private snapshot-interpolation buffer (`Interp`) into a public, reusable engine helper. Build that example (Orbital Dodger — interpolation-only, distinct from predict_shooter's prediction-heavy design), extract `engine::SnapshotBuffer<T: Lerp>` reusing the engine's existing `Lerp` trait, and migrate predict_shooter onto it. End state: a fork-friendly generic helper proven by two real call sites (one needing a non-`Vec2` channel), the example green on the `+1.88.0` gate, version bumped additively, committed + pushed.

## Where We Are

- **Branch `main`, pushed to `origin/main` at `f62df90`** (was `2195b87`). Tree clean. Engine **v4.4.0**.
- **3 commits this session** (`29f2b08`, `8538f0e`, `f62df90`): feat(network) SnapshotBuffer+migration · example orbital_dodger+manifest/version · docs(v4.4.0).
- **Full `+1.88.0` gate GREEN:** fmt · clippy `--all-targets -D warnings` · wasm lib+bins build · `orbital_dodger` wasm build · `test --all-targets` (**339 lib tests**, was 333; +6 SnapshotBuffer) · `RUSTDOCFLAGS=-D warnings` doc. `./scripts/verify.sh` green on local stable too.
- **`engine::SnapshotBuffer<T: Lerp>`** lives in `src/network.rs` (peer to `RemoteEntities`, cross-platform, no cfg gate). Reuses `crate::timeline::Lerp`. API: `new()`, `with_capacity(n)` (clamped ≥2), `push(t, value)`, `sample(rt) -> Option<T>`, `is_empty()`, `len()`. Private `Snapshot<T> { t, value }` (derives `Clone`, not `Copy`). `DEFAULT_SNAPSHOT_CAPACITY = 8`. Re-exported at `src/lib.rs:85`.
- **6 unit tests + 1 doctest** for SnapshotBuffer (`src/network.rs` tests mod): lerp-between-two (f32), clamp-outside+empty, ignore-out-of-order/dup, Vec2 interpolation, capacity-trim, capacity-floor-is-2. Doctest uses `f32` (avoids a glam import in the doc).
- **`orbital_dodger` example** (`examples/games/orbital_dodger/`): `protocol.rs`, `server.rs`, `orbital_dodger.rs`, `web/{index.html,build.sh}`. Two `[[example]]` entries: `orbital_dodger` + `orbital_dodger_server`. Port **9004** (coin_race=9002, predict_shooter=9003).
- **Server** (`server.rs`): broadcast-only, no client input. `HAZARD_COUNT = 7` drifting+spinning hazards; 60 Hz sim thread; **10 Hz** snapshot broadcast (`snap_every = 6`). Dependency-free xorshift `Rng` (+ `sign()`). Hazard `angle` grown **unbounded** (client lerps it as plain f32; wrapping at TAU would spin backwards across the seam). 4 unit tests (full hazard set, in-bounds after 2000 steps, spin advances, protocol round-trip).
- **Client** (`orbital_dodger.rs`): `DodgerClient` system with `hazards: RemoteEntities<usize>`, `hazard_pos: HashMap<usize, SnapshotBuffer<Vec2>>`, `hazard_rot: HashMap<usize, SnapshotBuffer<f32>>` — **two interpolation channels per hazard**. Local player is pure client-side (clamped to field), never round-trips. Win = reach the green vault (`x + PLAYER_RADIUS >= GOAL_X`); lose = circle-vs-circle collision → respawn + `deaths++`. `INTERP_DELAY_DEFAULT = 0.12` (s), `[`/`]` tune (step 0.02), `I` toggles interpolation, `R` restart, `Esc` quit.
- **The interp-on/off trick:** `rt = interp_enabled ? client_time - interp_delay : client_time`. Sampling at `client_time` (no delay) clamps to the latest snapshot → it holds-then-jumps at 10 Hz = the raw judder. Same `sample()` call, different `rt`.
- **predict_shooter migrated:** `player_interp`/`bullet_interp` are now `HashMap<usize, SnapshotBuffer<Vec2>>`; `push(t, Vec2::new(x,y))`; `sample(rt) -> Option<Vec2>`. `Interp` struct + its 3 tests deleted from `client_net.rs`; module doc updated; `Prediction` kept. Behavior-identical (Vec2 lerp == per-component lerp). 3 Prediction + 7 server tests still pass.
- **GUI playtest passed** (osascript per `playtest-windowed-examples`): orbital_dodger connects → hazards spawn → **position AND rotation both interpolate** (captured frame shows hazards rotated at distinct angles = `SnapshotBuffer<f32>` proof) → player movement → collision → respawn ("Caught! Back to the start."). predict_shooter regression smoke: connects, spawns local player, INTERP_DELAY 60 ms HUD intact.
- **Docs:** REFERENCE.html (new `SnapshotBuffer` h3, Korean prose + English code, tags balanced), CHANGELOG (4.4.0 + backfilled the missing 4.3.1 gilrs note), NEXT_WORK (seq 8), REMOTE_ENTITIES_DESIGN (Q#1 closed), CLAUDE.md module map.

## What We Tried (Chronological)

1. **Onboarded from the seq-7 handoff.** Read it fully, verified `HEAD == origin/main == 2195b87` (the seq-7 handoff's `7df2424` is the commit before its own session-doc commit — normal), tree clean. Read key files (`window.rs` poll_gilrs, `Cargo.toml`, `gamepad.rs`) + adjacent (`schedule.rs` isolation, `input/map.rs` gamepad resolution, `survivor.rs` bindings). Baseline gate: `test --lib` = **333 pass**. Narrated onboarding, asked direction.
2. **User chose the 2nd interpolating example** (over rust-survivors pin / GCController backend). Entered plan mode.
3. **Researched the promotion + example scaffolding** (2 parallel Explore agents): (a) mapped predict_shooter's full structure (server fixed-tick + snapshot broadcast, `#[path]` mod includes, wasm entry, web harness); (b) mapped `src/network.rs` (RemoteEntities is cross-platform, un-gated; `Lerp` at `timeline.rs:20` with f32/Vec2/[f32;4]/Color impls, re-exported `lib.rs:120`; no `SnapshotBuffer` name collision; placing it in network.rs is clean). Verified `Transform.rotation: f32` exists + renderer reads it; examples import `Vec2` via `use glam::Vec2`.
4. **Asked the user two product decisions** (AskUserQuestion): example concept → **Orbital Dodger** (interpolation-only, spinning hazards, single window); API scope → **build example + promote SnapshotBuffer + migrate** (the full payoff). Wrote the plan, ExitPlanMode → approved.
5. **Part 1 — engine helper.** Added `use crate::timeline::Lerp` + `use std::collections::VecDeque` to network.rs; inserted `SnapshotBuffer<T: Lerp>` after `RemoteEntities`; 6 tests; re-export. `test --lib snapshot_buffer` = **6 pass**; doctest passes. Confirmed only `src/{lib,network}.rs` changed (the rust-analyzer "stray file" warnings — `gilrs_probe.rs`, `probe.rs`, `camera.rs` unlinked — are phantom cache; `find` confirms they don't exist).
6. **Part 2 — example.** Wrote protocol.rs / server.rs / orbital_dodger.rs / web/. Two `[[example]]` entries. Built both binaries (clean), `test --example orbital_dodger_server` = **4 pass**, built client to wasm (clean).
7. **Part 3 — migration.** Removed `Interp` + `Sample` + module-doc bullet from client_net.rs; updated predict_shooter.rs imports/field-types/push/sample. First `test` build failed: the 3 `interp_*` tests still referenced `Interp` (the struct removal didn't touch the test mod). Removed them. Re-test = **3 Prediction + 7 server pass**. Confirmed predict_shooter still builds to wasm.
8. **Part 4 — docs + version.** REFERENCE.html SnapshotBuffer h3 (verified tag balance with a python script — all balanced); CHANGELOG 4.4.0 + backfilled 4.3.1; NEXT_WORK seq 8; REMOTE_ENTITIES_DESIGN Q#1 closed; CLAUDE.md row; version 4.3.1 → 4.4.0.
9. **Full gate.** `fmt --check` flagged formatting (multi-line clamp/assert) → ran `cargo +1.88.0 fmt` (CI-pinned rustfmt, per `ci-toolchain-pin`). clippy clean. wasm clean. `test --all-targets` = **339 lib**. `doc` failed once: `new()`'s doc linked the private `DEFAULT_SNAPSHOT_CAPACITY` → rewrote to link the public `with_capacity`. `verify.sh` green.
10. **GUI playtest** (background osascript script): positioned the window, captured interp-on, sent a 120× right-arrow burst (drove the player into the field), captured "moved" (hazards animated to new positions), toggled `I`, captured interp-off (HUD "Caught! Back to the start." + rotated hazards). predict_shooter regression smoke captured ("You are Player #1", INTERP_DELAY 60 ms).
11. **Wrap-up.** 3 logical commits, pushed `2195b87..f62df90`, updated `engine-current-state` memory + MEMORY.md index, wrote this handoff.

## Key Decisions

- **Generic `SnapshotBuffer<T: Lerp>`, not a hardcoded `(x,y)` copy.** orbital_dodger needs a *second* channel (spin angle, `f32`) beyond position — so the generic shape isn't speculative, it's forced by a real call site. Reuses the engine's existing `Lerp` trait (`src/timeline.rs`) rather than inventing a new abstraction. (Rejected: copy `Interp` verbatim into the new example — would've left two divergent buffers and no engine win.)
- **Interpolation split OUT of `RemoteEntities`, not folded in.** Confirms the seq-3/seq-4 finding: lifecycle (`id→Entity`) and value-interpolation are orthogonal, composed as parallel maps. Folding interpolation into `RemoteEntities` would force the concept + cost onto the two snap-only call sites (`mp_client`, `coin_race`). (Rejected: a richer `RemoteEntities` with a built-in buffer.)
- **Orbital Dodger is interpolation-ONLY (no prediction).** Proves interpolation stands alone, orthogonal to `Prediction` — the cleanest possible demonstration and a genuinely *distinct* example from predict_shooter. (Rejected concepts: Ghost Race — similar mechanics, fewer entity kinds; Network Air-Hockey — needs prediction for paddles + 2 windows, overlaps predict_shooter.)
- **Server is broadcast-only (no `ClientMsg`).** The player never round-trips; the server's sole job is being the low-rate authoritative source. Simpler than predict_shooter (no per-client `ack`/input). Documented in the protocol module doc.
- **Hazard `angle` grown unbounded on the server.** Client lerps angle as a plain `f32`; wrapping at TAU would make interpolation spin backwards across the 6.28→0 seam. Each 100 ms step is tiny and f32 precision is ample for a demo. (Rejected: `rem_euclid(TAU)` wrap — introduces the seam discontinuity.)
- **`Prediction` stays example-local.** Still one call site; a *local* concern (input replay vs. server correction), not remote-entity bookkeeping. A future `engine::Prediction` only if a 2nd prediction example appears — same single-call-site discipline.
- **`with_capacity` clamped to ≥2.** A buffer of <2 can never interpolate (needs a pair). Additive, fork-friendly; kept the rest of the API minimal (no speculative typed/staleness features — those await design-doc #3–#7).
- **Backfilled the missing 4.3.1 CHANGELOG entry.** seq-7 bumped the version but never logged it; added the gilrs-fix note while touching the changelog for 4.4.0.
- **Minor bump 4.3.1 → 4.4.0** (new public type = additive API surface; semver-after-1.0). 3 logical, individually-bisectable commits + push (user's standing workflow + explicit choice).

## Evidence & Data

### Commits this session
| Hash | Summary |
|---|---|
| `29f2b08` | feat(network): promote SnapshotBuffer<T: Lerp> + migrate predict_shooter |
| `8538f0e` | example: orbital_dodger — interpolation-only networked game (v4.4.0) |
| `f62df90` | docs(v4.4.0): SnapshotBuffer + orbital_dodger; close interpolation question |

Each commit builds on its own (bisectable): #1 = engine helper + migrated predict_shooter (no orbital_dodger refs); #2 = example + `[[example]]` entries + version; #3 = docs only.

### `SnapshotBuffer<T: Lerp>` public API
| Method | Behavior |
|---|---|
| `new()` / `Default` | empty, capacity 8 (`DEFAULT_SNAPSHOT_CAPACITY`) |
| `with_capacity(n)` | empty, retains most-recent `n` (clamped `.max(2)`) |
| `push(t: f64, value: T)` | record at client time `t`; ignore if `t <= back.t`; trim to capacity |
| `sample(rt: f64) -> Option<T>` | `None` if empty; clamp to first/last outside range; else `T::lerp(a, b, f)` |
| `is_empty()` / `len()` | inspection |

### Test counts
| Suite | Count | Note |
|---|---|---|
| lib (`src/`) | **339** | was 333; +6 SnapshotBuffer (`network::tests`) |
| orbital_dodger_server | 4 | hazard set, in-bounds@2000 steps, spin advance, protocol |
| predict_shooter (client_net) | 3 | Prediction only (interp tests moved to engine) |
| predict_shooter_server | 7 | unchanged |
| SnapshotBuffer doctest | 1 | `src/network.rs` line ~668 (f32) |

### Gate results (all green, `cargo +1.88.0`)
| Check | Result |
|---|---|
| `fmt --check` | OK (after `cargo +1.88.0 fmt`) |
| `clippy --all-targets -D warnings` | clean |
| `build --target wasm32` (lib+bins) | OK |
| `build --example orbital_dodger --target wasm32` | OK |
| `test --all-targets` | 339 lib + examples, 0 failed |
| `RUSTDOCFLAGS=-D warnings doc --no-deps` | OK (after the private-link fix) |
| `./scripts/verify.sh` | all checks passed ✓ (local stable) |

### orbital_dodger constants (`protocol.rs`)
| Const | Value |
|---|---|
| `SERVER_ADDR` | `127.0.0.1:9004` |
| `FIXED_DT` / `SNAPSHOT_HZ` | `1/60` / `10` (snap_every = 6) |
| `FIELD_W` / `FIELD_H` | `800` / `600` |
| `HAZARD_COUNT` / `HAZARD_RADIUS` | `7` / `26.0` |
| `HAZARD_MIN/MAX_SPEED` | `80.0` / `190.0` px/s |
| `HAZARD_MIN/MAX_SPIN` | `1.2` / `3.4` rad/s (sign randomized) |
| `PLAYER_RADIUS` / `PLAYER_SPEED` | `12.0` / `260.0` px/s |
| `GOAL_X` | `FIELD_W - 48.0` |

### Client interp tuning (`orbital_dodger.rs`)
| Const | Value | Note |
|---|---|---|
| `INTERP_DELAY_DEFAULT` | `0.12` s | ≥1 snapshot interval (100 ms) + margin |
| `INTERP_DELAY_MIN/MAX/STEP` | `0.0` / `0.40` / `0.02` | `[`/`]` tune |

### GUI playtest captures
- `/tmp/od_on.png` — HUD "Connected — dodge to the green vault!", warm hazard squares scattered at real positions.
- `/tmp/od_moved.png` — hazards animated to new positions (server stepping + client interpolating over time).
- `/tmp/od_off.png` — **hazards visibly rotated at distinct angles** (`SnapshotBuffer<f32>` proof) + HUD "Caught! Back to the start." (movement→collision→respawn proven).
- `/tmp/ps_smoke.png` — predict_shooter "You are Player #1 — WASD to move", INTERP_DELAY 60 ms tuner intact (no regression).
- Server log `/tmp/od_server.log`, client logs `/tmp/od_client.log` (no warnings/errors at RUST_LOG=warn).

### Primary-evidence code (the parts expensive to re-derive)

`SnapshotBuffer::sample` (`src/network.rs`) — the generic interpolation core:

```rust
pub fn sample(&self, rt: f64) -> Option<T> {
    let front = self.samples.front()?;
    if rt <= front.t { return Some(front.value.clone()); }   // clamp to first
    let back = self.samples.back()?;
    if rt >= back.t { return Some(back.value.clone()); }      // clamp to last
    for i in 0..self.samples.len() - 1 {
        let a = &self.samples[i];
        let b = &self.samples[i + 1];
        if a.t <= rt && rt <= b.t {
            let span = b.t - a.t;
            let f = if span > 0.0 { ((rt - a.t) / span) as f32 } else { 0.0 };
            return Some(T::lerp(&a.value, &b.value, f));      // generic lerp
        }
    }
    Some(back.value.clone())
}
```

orbital_dodger's **two-channel** ingest + render (the thing that justified the generic `T`):

```rust
// ingest (per snapshot): position AND spin angle, both stamped at client_time
self.hazard_pos.entry(h.id).or_default().push(self.client_time, Vec2::new(h.x, h.y));
self.hazard_rot.entry(h.id).or_default().push(self.client_time, h.angle);

// render: interp ON → past time; OFF → latest (clamps → holds-then-jumps = 10 Hz judder)
let rt = if self.interp_enabled { self.client_time - self.interp_delay } else { self.client_time };
let pos   = self.hazard_pos.get(id)?.sample(rt)?;                       // SnapshotBuffer<Vec2>
let angle = self.hazard_rot.get(id).and_then(|b| b.sample(rt)).unwrap_or(0.0); // SnapshotBuffer<f32>
// tr.position = pos; tr.rotation = angle;
```

orbital_dodger `ServerMsg` wire protocol (broadcast-only, no `ClientMsg`):

```rust
pub struct BodyState { pub id: usize, pub x: f32, pub y: f32, pub angle: f32 }
#[serde(tag = "t")]
pub enum ServerMsg {
    #[serde(rename = "hello")] Hello { hazards: Vec<BodyState> }, // bulk on connect
    #[serde(rename = "snap")]  Snap  { tick: u32, hazards: Vec<BodyState> }, // 10 Hz
}
```

## Code Analysis

- **`SnapshotBuffer::sample` (`src/network.rs`):** front-clamp (`rt <= front.t` → first), back-clamp (`rt >= back.t` → last), else linear scan for the bracketing pair and `T::lerp(&a.value, &b.value, ((rt-a.t)/span) as f32)`. Identical math to the old `Interp::sample`, value-generic. `span > 0.0` guard avoids div-by-zero (push() already rejects equal stamps, so this is belt-and-suspenders).
- **`Snapshot<T>` derives `Clone` not `Copy`** — `T: Lerp` only guarantees `Clone`. `sample` returns `front.value.clone()` / `T::lerp(...)`.
- **`Lerp` trait (`src/timeline.rs:20`):** `pub trait Lerp: Clone { fn lerp(a: &Self, b: &Self, t: f32) -> Self; }`. Impls: `f32`, `glam::Vec2` (`a.lerp(*b, t)`), `[f32;4]`, `crate::color::Color`. Un-gated, re-exported `lib.rs:120`. Adding `SnapshotBuffer` to network.rs needed only `use crate::timeline::Lerp` — no cfg conflict (both un-gated).
- **predict_shooter migration is behavior-identical:** `Vec2::lerp` is component-wise, so `SnapshotBuffer<Vec2>` produces the same numbers the old `(x,y)` `Interp` did. No netcode change; pure storage-type refactor.
- **orbital_dodger server reuses the predict_shooter server skeleton:** `Arc<Mutex<Server>>`, 60 Hz `thread::sleep(FIXED_DT)` sim thread, `s.tick.is_multiple_of(snap_every)` broadcast gate (`is_multiple_of` is stable on 1.88), per-client `mpsc::Sender<Message>`, Hello sent OUTSIDE the lock (the tick thread needs it 60×/s). Simpler: no players/inputs/ack — just `clients: HashMap<usize, Sender>` and a fixed hazard `Vec`.
- **`Transform.rotation: f32`** (radians, Z axis; `src/components.rs:16`) is read by the sprite renderer (`is_visible(...)` + instance transform) — so writing the sampled `f32` angle to `tr.rotation` actually spins the hazard sprite.
- **Client borrow pattern (collect-then-apply):** `DodgerClient::run` collects `Vec<(Entity, Vec2, f32)>` from the immutable `self.hazards.iter()` + `self.hazard_pos/rot` reads FIRST, then loops `world.get_mut::<Transform>(e)` to write — can't hold an immutable `self` borrow across `world` mutation. Mirrors predict_shooter's `player_updates`/`bullet_updates` collect step.
- **Collision uses the *displayed* hazard positions** (the same interpolated-or-raw `Vec2` written to the transform that frame), so what the player sees is what can hit them — fair regardless of the `I` toggle. Circle-vs-circle: `player_pos.distance(pos) < PLAYER_RADIUS + HAZARD_RADIUS`.
- **Doctest gotcha:** `new()`'s doc first linked `[`DEFAULT_SNAPSHOT_CAPACITY`]` (a private const) → `RUSTDOCFLAGS=-D warnings cargo doc` fails (`private_intra_doc_links`). Fixed by linking the public `[`with_capacity`](Self::with_capacity)` instead. Watch this on any future doc linking a private item.
- **Phantom rust-analyzer diagnostics:** throughout the session rust-analyzer flagged `inactive-code` (wasm `#[cfg]` blocks) and `unlinked-file` warnings for files like `gilrs_probe.rs` / `probe.rs` / `camera.rs` / `schedule.rs`. These are stale editor cache — `find` confirms the probe files don't exist and `git status` stayed clean. Trust `cargo`, not these.

## Files Changed

### Source / config
- `src/network.rs` — added `SnapshotBuffer<T: Lerp>` (+ private `Snapshot<T>`, `DEFAULT_SNAPSHOT_CAPACITY`) + 6 unit tests + doctest; `use crate::timeline::Lerp` + `use std::collections::VecDeque`. (+182 lines)
- `src/lib.rs` — `pub use network::{... SnapshotBuffer}` (line 85).
- `Cargo.toml` — version `4.3.1` → `4.4.0`; two `[[example]]` entries (`orbital_dodger`, `orbital_dodger_server`).
- `Cargo.lock` — skeleton-engine `4.4.0`.

### New example (`examples/games/orbital_dodger/`)
- `protocol.rs` — `BodyState`, `ServerMsg::{Hello, Snap}`, consts; no `ClientMsg` (broadcast-only).
- `server.rs` — broadcast hazard server (xorshift Rng + `sign()`, 60 Hz sim / 10 Hz snapshot, wall-bounce, unbounded spin) + 4 tests.
- `orbital_dodger.rs` — `DodgerScene` + `DodgerClient` (two `SnapshotBuffer` channels, local player, I-toggle, win/lose), wasm entry `run_orbital_dodger`.
- `web/index.html` + `web/build.sh` (exec bit set) — wasm harness, mirrors predict_shooter.

### predict_shooter migration
- `examples/games/predict_shooter/client_net.rs` — deleted `Interp` + `Sample` + 3 interp tests; module doc updated; kept `Prediction`. (−102 lines)
- `examples/games/predict_shooter/predict_shooter.rs` — `use engine::SnapshotBuffer`; maps → `SnapshotBuffer<Vec2>`; `push(t, Vec2::new(x,y))`; `sample` → `Option<Vec2>`.

### Docs
- `REFERENCE.html` — `SnapshotBuffer<T: Lerp>` h3 (table + tuning blockquote + examples) under 네트워크; RemoteEntities deferral note refreshed.
- `docs/CHANGELOG.md` — `## 4.4.0` + backfilled `## 4.3.1` (gilrs fix).
- `docs/NEXT_WORK.md` — "Networking-dogfood seq 8" section.
- `docs/REMOTE_ENTITIES_DESIGN.md` — "SnapshotBuffer promoted" section (Q#1 closed); pointers updated.
- `CLAUDE.md` — module-map network row.

### Memory
- `engine-current-state.md` → v4.4.0 / seq 8; `MEMORY.md` index line updated.

## User Feedback & Preferences (this session)

- **Chose the 2nd interpolating example** over the other seq-7 options (rust-survivors pin, GCController backend).
- **Chose Orbital Dodger** (interpolation-only) as the concept over Ghost Race / Network Air-Hockey.
- **Chose "build example + promote SnapshotBuffer + migrate both"** (the full payoff) over example-only/defer.
- **Chose 3 commits + push** (matched standing commit-to-main + push workflow) and **handoff + memory** at wrap.
- **Standing prefs (memory):** Korean conversation / English artifacts; gate with `cargo +1.88.0` (not plain cargo); subjective GUI feel (interp_delay, smoothness) is the user's call; release/version decisions are the user's; use subagents for parallel work.
- Followed the requested onboarding ritual (narrate understanding → state verification → read key + adjacent files → propose first action → wait for go-ahead) before executing.

## Where We're Going

Optional, none scheduled (the chosen work stream is complete):

1. **Settle `INTERP_DELAY_DEFAULT` for orbital_dodger by feel** — currently 0.12 s (picked analytically as ~1.2× the 100 ms snapshot interval). The `[`/`]` tuner + `I` toggle are in place; a real-play pass (the user's subjective call, like predict_shooter's 60 ms) could refine it.
2. **rust-survivors engine-pin bump** — to v4.4.0 (picks up gilrs 0.11.2 transitively + SnapshotBuffer, neither of which the game uses directly). Verify it builds on the next pin (memory `rust-survivors-engine-pin`: the game pins the engine by git rev). To test against this now-pushed `f62df90` before bumping the pin, from the rust-survivors repo:
   `cargo build --config 'patch.”https://github.com/ChunSam/skeleton-engine”.skeleton-engine.git=”/Users/jkl/Projects/skeleton-engine”'` (or set the pin rev to `f62df90`). Expect a clean build — the engine change is additive; the public API the game uses is unchanged.
3. **Promote `Prediction`** — only if a *second* prediction example appears (current discipline). Likewise the still-open `RemoteEntities` design questions #3–#7 (per-entity update callbacks, typed entities, staleness, binary protocol, disconnect policy) await examples that stress them.
4. **survivor gamepad live-validation / macOS GCController backend** — seq-7's still-open, environment-gated items (macOS can't read Xbox/PS pads via gilrs).

## Risks & Blockers

- **orbital_dodger server is native-only** (pulls `tungstenite`) — do NOT gate it on `wasm32 --all-targets` (the documented WASM gotcha). The *client* example builds on wasm (verified). CI's lib+bins wasm gate is the real gate.
- **interp_delay default unvalidated by subjective feel** — chosen analytically; smoothness-vs-lag is a feel parameter (low risk; the mechanism is unit-tested + GUI-confirmed working).
- **rust-survivors not re-verified** against v4.4.0 — flag for its next pin bump.
- **orbital_dodger `web/pkg/` is NOT built/committed** (intentional — `pkg/` is gitignored). To ship the example to the browser, run `examples/games/orbital_dodger/web/build.sh` (needs a `wasm-bindgen-cli` matching the `wasm-bindgen` crate in Cargo.lock, then serve over http). The wasm *compile* is gated (`build --example orbital_dodger --target wasm32`, verified green); the browser render was NOT eyeballed this session (no live render check run — optional `scripts/wasm_smoke.sh` covers coin_race only).
- Gate with `cargo +1.88.0` for CI parity (local stable diverges — memory `ci-toolchain-pin`).

## Open Questions

- **Optimal orbital_dodger `interp_delay`?** 0.12 s analytically; a feel pass could tune it.
- **Should `SnapshotBuffer` ever own a "latest()" or extrapolation mode?** Not needed by either example (the `rt = client_time` trick already gives raw-latest). Add only if an example demands it.
- Design-doc **#3–#7** remain open (none stressed yet).

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5 && git status -s     # clean; main @ f62df90 == origin/main; v4.4.0
# bd is UNAVAILABLE — track with TaskCreate.

# Verify (gate with the CI pin, NOT plain cargo — local stable diverges):
cargo +1.88.0 test --lib                   # 339 pass
./scripts/verify.sh                        # green on local stable too

# Key files this session:
#   src/network.rs (SnapshotBuffer<T: Lerp> + tests; peer to RemoteEntities)
#   src/lib.rs (re-export, line 85)
#   examples/games/orbital_dodger/{protocol,server,orbital_dodger}.rs + web/
#   examples/games/predict_shooter/{client_net,predict_shooter}.rs (migrated; Interp gone)
#   docs/REMOTE_ENTITIES_DESIGN.md (Q#1 closed; #3-#7 open)

# Run the example (interpolation made visible):
#   T1: cargo run --example orbital_dodger_server
#   T2: cargo run --example orbital_dodger      # WASD to the green vault; I toggles interp (watch the judder)

# Next action (optional, none scheduled): settle orbital_dodger interp_delay by feel,
#   OR bump rust-survivors' engine pin to v4.4.0 and verify it builds.
```

## Session Closed
**Closed at:** 2026-06-09 23:57 KST
**Commit:** `f62df90` (3 feature commits, pushed to `origin/main`) + this handoff doc
**Session status:** Handed off to next session. networking-dogfood seq 8 complete — `engine::SnapshotBuffer<T: Lerp>` promoted (reusing `Lerp`), `orbital_dodger` interpolation-only example shipped (native + web), `predict_shooter` migrated, design-doc open question #1 closed, engine v4.4.0. Full `+1.88.0` gate green (339 lib tests); GUI playtest confirmed position + rotation interpolation.
