# INTERP_DELAY settled (60 ms) + gilrs controller-crash fix (v4.3.1) + macOS gamepad limitation + REFERENCE.html for seq-6 APIs

**Date:** 2026-06-09
**Status:** COMPLETED — the seq-6 open items (subjective INTERP_DELAY pick + the deferred REFERENCE.html + GUI smokes) were executed, AND the controller test surfaced a real engine crash that was fixed. All committed + **pushed to `origin/main`** (`cce718f..7df2424`). Engine **bumped 4.3.0 → 4.3.1** (the gilrs crash fix is a real bug fix).
**Bead(s):** none (`bd` unavailable — tracked with TaskCreate this session)
**Epic:** VISION feature+example loop — networking feel + breadth-audit polish
**Chain:** `networking-dogfood` seq `7`
**Parent:** `HANDOFF_networking-dogfood_phase3-polish_2026-06-09.md` (seq 6)
**Prior chain:** `coin-race-example` (1) > `wasm-coin-race-v4.1` (2) > `deferred-polish` (3) > `client-prediction-shooter` (4) > `phase-d-realplay` (5) > `phase3-polish` (6) > this (7)

---

## Stale References

Parent (seq 6) identifiers that changed this session — next session beware:

- **`INTERP_DELAY_DEFAULT`** (`predict_shooter.rs`) — value changed **0.1 → 0.06** (100 ms → 60 ms). Still the same const name; doc comment now records the 60 ms feel rationale. `INTERP_DELAY_MIN/MAX/STEP` and the runtime `ShooterClient.interp_delay` tuner are unchanged.
- **predict_shooter HUD string** — `"... [ / ] to tune ..."` → `"... [ -10ms   ] +10ms ..."` (the `[ / ]` was misread by the user as the slash key; the actual keys are `BracketLeft`/`BracketRight`, unchanged).
- **gilrs dependency** — `Cargo.toml` `gilrs = "0.10"` → `"0.11"` (resolves to **0.11.2**; gilrs-core 0.5.15 → 0.6.8). New transitive deps: `objc2-io-kit`, `objc2-core-foundation`.
- **`App::poll_gilrs`** (`src/app/window.rs:447`) — now wraps the gilrs poll in `catch_unwind`; on a gilrs panic it logs once and sets `self.gilrs = None` (disables gamepad for the session).
- **Engine version** — `4.3.0` → **`4.3.1`** (`Cargo.toml` + `Cargo.lock`).
- All other seq-6 identifiers (`Camera::bounds`/`clamp_to_bounds`, `InputMap` gamepad API, `AxisBinding`) unchanged.

## Since Last Handoff

Parent (seq 6) closed the networking-dogfood Phase-3 polish and left **one subjective open item** (pick `INTERP_DELAY` by feel) plus optional follow-ups (lit_dungeon GUI smoke, survivor gamepad test needing a controller, REFERENCE.html update for 4 new public APIs). This session, the user chose to do **the objective work first (REFERENCE.html + lit_dungeon smoke)**, then settle INTERP_DELAY by playing, then attempt the survivor gamepad test.

- **REFERENCE.html (deferred in seq 6) → done.** All 4 seq-6 public APIs documented + a macOS gamepad-limitation note.
- **lit_dungeon Camera::bounds → GUI-verified** by the agent (osascript playtest; `playtest-windowed-examples` memory).
- **INTERP_DELAY → settled at 60 ms** by the user playing predict_shooter (2 windows); baked as `INTERP_DELAY_DEFAULT`.
- **survivor gamepad (B) → BLOCKED, then root-caused + documented.** Connecting a controller for this test **surfaced a real crash** (gilrs panic) that the parent never hit (parent's gamepad work was keyboard/unit-only). Fixed the crash; then discovered the controller still can't be read on macOS (GameController exclusive ownership). B is "code correct, macOS live-validation impossible via gilrs."
- Parent's open question "optimal INTERP_DELAY?" is now **answered: 60 ms.**

## Reference Documents

- `docs/NEXT_WORK.md` — added the **seq-7 section** (INTERP_DELAY 60 ms, gilrs fix, macOS gamepad limit) + a macOS note appended to the InputMap-gamepad bullet.
- `REFERENCE.html` — now documents the seq-6 APIs (was `grep`=0) + the macOS gamepad blockquote.
- Memory: `playtest-windowed-examples` (osascript GUI test technique — used for lit_dungeon + predict_shooter), `ci-toolchain-pin` (`+1.88.0` gate), `conversation-language-korean`, `doc-language-rule`, `subagent-usage-preference`, `rust-survivors-engine-pin`, `engine-current-state`.

## The Goal

Close out the seq-6 leftovers: (a) document the 4 new public APIs in `REFERENCE.html`; (b) GUI-smoke `Camera::bounds` in lit_dungeon; (c) settle the subjective `INTERP_DELAY` value by real play and bake it; (d) validate the survivor gamepad path with a real controller. End state: engine green on the `+1.88.0` gate, changes committed + pushed, version bumped only if a real fix warrants it.

## Where We Are

- **Branch `main`, pushed to `origin/main` at `7df2424`** (was `cce718f`). Tree clean. Engine **v4.3.1**.
- **3 commits this session** (`8edbd91`, `03bea6f`, `7df2424`): docs(reference), fix(input gilrs), tune(predict_shooter).
- **Full `+1.88.0` gate GREEN on the final tree:** fmt · clippy `--all-targets -D warnings` · wasm lib+bins build · `test --all-targets` (**333 lib tests**, 0 failed) · `RUSTDOCFLAGS=-D warnings` doc.
- **(a) REFERENCE.html** — added "월드 바운드 클램핑" h3 under 카메라 (`Camera::bounds` + `clamp_to_bounds`), "게임패드" h3 under 입력 (`GamepadState` basics: `primary`/`is_pressed`/`just_pressed`/`axis`/`any_connected`), "InputMap 게임패드 바인딩" h3 (`bind_gamepad_button`/`bind_gamepad_axis`/`AxisBinding::positive|negative`/`*_with_gamepad`/`keys_for`) with a method table + axis-edge limitation note, and a **macOS limitation blockquote**. +56 lines initially, +1 blockquote later. Korean prose + English code, matching the file. Tag-balance verified.
- **(b) lit_dungeon** — `Camera::bounds` verified in real play. Player spawns at world (96,96) → renders top-left (camera clamped at origin); after walking S+D to the far corner → renders bottom-right (clamped at max (160,168)). Off-center-at-edges = clamp working. Screenshots `/tmp/litd_spawn.png`, `/tmp/litd_corner2.png`.
- **(c) INTERP_DELAY** — user played, settled on **60 ms**. Baked `INTERP_DELAY_DEFAULT = 0.06` + rationale comment; HUD label clarified.
- **(d) survivor gamepad** — see "What We Tried" #5–#8. Crash fixed; live validation blocked by macOS platform limitation; documented.

## What We Tried (Chronological)

1. **Onboarded + baseline.** Read the seq-6 handoff + key files (`camera.rs`, `input/map.rs`, `app/schedule.rs`, `predict_shooter.rs`, `client_net.rs`, `protocol.rs`, `NEXT_WORK.md`). Confirmed git == origin == `cce718f`, clean. Ran baseline `+1.88.0` gate: `test --lib` = **333 pass**, clippy clean. Verified the timing model independently: 60 Hz sim, 30 Hz snapshots (33.3 ms), `INTERP_DELAY_DEFAULT` was 100 ms ≈ 3× interval; `Interp::sample()` clamps on overrun.
2. **REFERENCE.html.** Discovered the file is **all-Korean prose + English code**; gamepad was **entirely undocumented**. Read `src/input/gamepad.rs` for the exact `GamepadState` API + enum variants. Added 3 h3 sections (camera bounds, gamepad, InputMap gamepad) matching the file's style. No TOC change needed (h3 subsections aren't in the TOC). Verified HTML tag balance via a python script.
3. **lit_dungeon GUI smoke.** Example name is **`lit_dungeon_game`** (not `lit_dungeon`). Level 960×768 > viewport 800×600 (bounds clamp is meaningful). Player movement is normalized diagonal @ 235 px/s; collision slides. Launched via the prebuilt binary, osascript window bounds, `caffeinate -u` + `screencapture`. Torch fuel = 13 s; first attempt timed out (torch died → GAME OVER), restarted with `R` and drove to the corner within the fuel window. Confirmed clamp by the off-center player position.
4. **INTERP_DELAY playtest setup.** Launched `predict_shooter_server` + 2 `predict_shooter` clients (direct binaries → 2 PIDs), tiled side-by-side. **zsh gotcha:** `set -- $CLIENTS` does NOT word-split in zsh; used `clients=(${(f)"$(pgrep ...)"})` instead. Both clients connected (server log `total: 2`). User played and **settled on 60 ms** (below 40 ms = bullet ghosting/trail; above 70 ms = bullet lingers at the shooter's old position when moving-and-firing).
5. **Bracket-key confusion.** User: "I don't know how to change the value with `/`." The HUD `[ / ]` was misread as the slash key; the keys are `[`/`]` (BracketLeft/Right). Clarified, and folded a HUD-wording fix into the bake (`[ -10ms   ] +10ms`).
6. **Baked 60 ms** + rationale comment. Closed the predict_shooter windows.
7. **survivor gamepad → CRASH discovered.** Launched `survivor_game` with the controller connected → both predict_shooter clients had ALSO crashed earlier the same way. Panic: `gilrs-0.10.10/src/gamepad.rs:458 'called Option::unwrap() on a None value'`. Backtrace: `poll_gilrs` (window.rs:451) → `gilrs.next_event()` → `next_event_priv` (gamepad.rs:278, processing `RawEventType::AxisValueChanged`) → `Gilrs::gamepad(id).unwrap()` (gamepad.rs:458) → panic. lit_dungeon ran fine earlier because **no controller → no gilrs events → `poll_gilrs` early-returns** (`events.is_empty()`). The user had connected the controller for this test.
8. **Fixed the crash, then root-caused the no-input.** (a) Added a `catch_unwind` guard in `poll_gilrs`; (b) upgraded gilrs 0.10 → 0.11.2. Verified: predict_shooter survived 7 s with the controller connected (no panic). Full gate green. **But** the controller still produced no input. Wrote a temporary `examples/gilrs_probe.rs` (gilrs-core self-pumps a CFRunLoop thread, so headless works); ran it **5×**. Result: `Connected` event fires but **0 input events**, even while the user actively moved sticks. `ioreg` revealed the cause: the pad is **exclusively owned by `com.apple.gamecontroller.driver.XboxGamepad`** (`UsbExclusiveOwner=XboxUSBDevice`). Deleted the probe. Documented the limitation.
9. **Wrap-up.** Bumped version 4.3.0 → 4.3.1, updated NEXT_WORK, made 3 commits, pushed, wrote this handoff.

## Key Decisions

- **INTERP_DELAY = 60 ms** (user's subjective call, matched the agent's ~66 ms = 2× snapshot-interval estimate). Kept the live `[`/`]` tuner — only the default changed.
- **Keep gilrs at 0.11.2 + the guard (not revert to 0.10.10).** 0.11.2 fixes the crash (real robustness win) and is newer; 0.10.10 detected the G7 Pro (BT) but crashed. Neither makes Xbox/PS pads work on macOS (that's the GameController limitation, version-independent). The unwrap path still exists in 0.11.2, so the guard is a durable safety net for other misbehaving controllers.
- **`catch_unwind` + disable-on-panic** (not catch-and-continue). Mirrors `schedule.rs` per-system isolation. On a gilrs panic, disable gamepad for the session rather than re-panicking every frame.
- **Patch bump 4.3.0 → 4.3.1.** The gilrs crash fix is a genuine bug fix; the other changes (docs, example tuning) are non-version-worthy but ride along. (User chose this.)
- **3 logical commits, commit-to-main + push** (user's standing workflow + explicit choice this session).
- **REFERENCE.html new sections in Korean prose + English code** — match the existing all-Korean manual; the doc-language English rule targets agent-facing docs, not this user manual. (Flagged to the user; accepted implicitly.)
- **survivor B = "code correct, macOS-blocked + documented"**, not "failed." The blocker is platform/library, outside engine code.
- **Did NOT add a GCController backend.** That's a large separate effort; recorded as a future option.

## Evidence & Data

### Commits this session
| Hash | Summary |
|---|---|
| `8edbd91` | docs(reference): document seq-6 gamepad/camera APIs + macOS gamepad limit |
| `03bea6f` | fix(input): isolate gilrs panic + upgrade gilrs 0.10->0.11.2 (v4.3.1) |
| `7df2424` | tune(predict_shooter): default INTERP_DELAY to 60ms + clearer tuner HUD |

### gilrs crash (the controller test surfaced it)
| Fact | Value |
|---|---|
| Panic | `called Option::unwrap() on a None value` |
| Site | `gilrs-0.10.10/src/gamepad.rs:458` (`Gilrs::gamepad(id)` → `self.inner.gamepad(id.0).unwrap()`) |
| Trigger | `next_event_priv` (gamepad.rs:278) processing `RawEventType::AxisValueChanged` |
| Our call | `poll_gilrs` (`src/app/window.rs:451`) — documented `gilrs.next_event()` loop |
| Why latent | no controller → no events → `poll_gilrs` returns at `events.is_empty()` |
| Fix | catch_unwind guard + gilrs 0.10.10 → 0.11.2 (gilrs-core 0.5.15 → 0.6.8) |
| Verified | predict_shooter survived 7 s with controller connected, no panic |

### macOS gamepad limitation (5 probe runs + ioreg)
| Fact | Value |
|---|---|
| Controllers tried | GameSir-G7 Pro (BT, VID 0x3537), Xbox Series pad (VID 0x045E/1118, PID 0x0B12/2834) |
| Probe result (×5) | `(none reported)` at startup, then a single `Connected` event, **0 input events** while moving |
| ioreg evidence | `"UsbExclusiveOwner" = "XboxUSBDevice"`, driver `com.apple.gamecontroller.driver.XboxGamepad`, classes `XboxSeriesXGamepad`/`XboxWirelessGamepad` |
| Root cause | macOS GameController framework exclusively owns Xbox/PS pads; gilrs uses IOKit HID → enumerates (`Connected`) but gets no input |
| Works where | Linux, Windows, and generic-HID (DInput) pads on macOS |

### INTERP_DELAY feel (user's findings)
| Value | Observation |
|---|---|
| < ~40 ms | bullets ghost/trail badly |
| **60 ms** | **sweet spot (chosen)** ≈ 2× the 33 ms snapshot interval |
| > ~70 ms | a bullet lingers at the shooter's old position when moving and firing |
| (old default) | 100 ms |

### Test counts
- **333 lib tests, 0 failed** — unchanged from seq-6 baseline (the guard isn't unit-tested; consistent with `schedule.rs` panic-isolation not being unit-tested).

### GUI playtest captures
- `/tmp/litd_spawn.png` (player top-left = camera clamped at origin), `/tmp/litd_corner2.png` (player bottom-right = clamped at max). `/tmp/ps_setup.png` (2 predict_shooter windows, both connected, HUD shows INTERP_DELAY 100 ms before the bake). Probe logs `/tmp/gilrs_probe{,2..5}.log`.

## Code Analysis

- **`poll_gilrs` (`src/app/window.rs:447`):** collects events inside `catch_unwind(AssertUnwindSafe(|| { while let Some(e)=gilrs.next_event() {evs.push(e)} evs }))`; on `Err`, sets `gilrs_panicked=true`, logs `log::error!`, sets `self.gilrs=None`, returns. The `gilrs_panicked` flag avoids a borrow conflict (the `if let Some(gilrs)=&mut self.gilrs` borrow must end before `self.gilrs=None`).
- **predict_shooter tuner:** `INTERP_DELAY_DEFAULT=0.06`; `MIN=0.0`/`MAX=0.30`/`STEP=0.01` unchanged; `[`/`]` via `KeyCode::BracketLeft/BracketRight` (`just_pressed`); HUD `format!("INTERP_DELAY {:.0} ms   ·   [ -10ms   ] +10ms   ·   default {:.0} ms", ...)`.
- **gilrs 0.11.2 API compat:** zero changes needed to `src/input/gamepad.rs` — `Gilrs`/`Event`/`EventType`/`Button`/`Axis`/`GamepadId` are identical to 0.10. The fix is entirely in gilrs-core 0.6.8's macOS backend (objc2-io-kit). The `unwrap()` at gamepad.rs:290/473 still exists in 0.11.2 (source-identical), confirming the fix is backend-level, not a code-path change → the guard stays warranted.
- **gilrs-core 0.6.8 macOS:** spawns its own `std::thread` + `CFRunLoop::run()` (platform/macos/gamepad.rs:88) → headless probes work without a Cocoa app.

## Files Changed

### Source / config
- `src/app/window.rs` — `poll_gilrs` catch_unwind guard.
- `Cargo.toml` — `gilrs = "0.11"`; version `4.3.0` → `4.3.1`.
- `Cargo.lock` — gilrs 0.10.10 → 0.11.2 (+ gilrs-core 0.6.8, objc2-io-kit, objc2-core-foundation, inotify/nix bumps); skeleton-engine 4.3.1.
- `examples/games/predict_shooter/predict_shooter.rs` — `INTERP_DELAY_DEFAULT = 0.06` + rationale comment + HUD label.

### Docs
- `REFERENCE.html` — 카메라 world-bounds h3, 입력 게임패드 + InputMap-gamepad h3s + table, macOS gamepad blockquote.
- `docs/NEXT_WORK.md` — seq-7 section + macOS note on the InputMap-gamepad bullet.

### Temporary (created + deleted)
- `examples/gilrs_probe.rs` — diagnostic probe; deleted before committing.

## User Feedback & Preferences (this session)

- Chose to do **objective work first (REFERENCE.html + lit_dungeon smoke)**, then INTERP_DELAY, then gamepad.
- **INTERP_DELAY = 60 ms** with clear reasoning (ghost/trail vs lingering bullet).
- Misread the `[ / ]` HUD as the slash key → confirms terse symbol labels are ambiguous; prefer explicit labels.
- For the gamepad crash: chose **"guard + upgrade both"** and **"fix gilrs first, then test"**; persisted through 5 probe runs + a controller swap (G7 Pro → Xbox) trying to get input.
- Wrap: **3 logical commits + push**, **4.3.1 patch bump**, **write handoff** — all "recommended" options.
- **Standing prefs (memory):** Korean conversation / English artifacts; gate with `cargo +1.88.0`; subjective GUI feel is the user's call; commit-to-main + push; release/version decisions are the user's.

## Where We're Going

1. **survivor gamepad live-validation (when possible).** Needs one of: a Linux/Windows machine, OR a generic-HID/DInput-mode controller (some 8BitDo/GameSir non-Xbox pads) that macOS doesn't claim with the GameController driver. The code is ready; only the macOS+Xbox/PS combination is blocked.
2. **(Optional, large) native macOS GCController backend** — would let the engine read Xbox/PS pads on macOS where gilrs can't. Separate session; gilrs doesn't do this.
3. **(Optional) `rust-survivors` engine-pin bump** — picks up gilrs 0.11.2 (transitive; the game doesn't use gilrs directly) + the seq-6 `A: Clone` widening. Verify it builds on the next pin (memory `rust-survivors-engine-pin`: pinned by git rev; use `--config` path patch to test unpushed).
4. **(Optional) 2nd interpolating networked example** — would unlock promoting `client_net::Interp` (see `docs/REMOTE_ENTITIES_DESIGN.md`); still deferred.

## Risks & Blockers

- **gilrs cannot read Xbox/PS controllers on macOS** (GameController exclusive ownership) — fundamental, documented; not fixable without a GCController backend. The `catch_unwind` guard prevents a crash; it does not enable input.
- **The catch_unwind guard disables gamepad for the whole session on the first gilrs panic** — intentional (avoids per-frame re-panic) but means one bad controller event kills gamepad until restart.
- **`rust-survivors` not re-verified** against gilrs 0.11.2 this session — flagged for its next pin bump.
- **`predict_shooter_server` still has no hit detection** (seq-4 scope cut) — bullets fly + expire only.
- Gate with `cargo +1.88.0` for CI parity (local stable 1.95 diverges — memory `ci-toolchain-pin`).

## Open Questions

- **Does gilrs 0.11.2 read a *generic-HID/DInput* pad on macOS?** Untested (no such pad available this session). The probe is the tool (re-create `examples/gilrs_probe.rs` from this handoff if needed).
- **Is a GCController backend worth it for the skeleton engine?** Big effort; only matters for macOS Xbox/PS pad support.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -6 && git status -s     # clean; main @ 7df2424 == origin/main; v4.3.1
# bd is UNAVAILABLE — track with TaskCreate.

# Verify (gate with the CI pin, NOT plain cargo — local stable 1.95 diverges):
cargo +1.88.0 test --lib                   # 333 pass
cargo +1.88.0 clippy --all-targets -- -D warnings
./scripts/verify.sh                        # green on local stable too

# Key files this session:
#   src/app/window.rs (poll_gilrs catch_unwind guard)
#   Cargo.toml/Cargo.lock (gilrs 0.11.2, v4.3.1)
#   examples/games/predict_shooter/predict_shooter.rs (INTERP_DELAY_DEFAULT=0.06 + HUD)
#   REFERENCE.html (seq-6 APIs + macOS gamepad note)   docs/NEXT_WORK.md (seq-7 section)

# Gamepad on macOS (why survivor B is blocked):
#   gilrs (IOKit HID) can't read Xbox/PS pads — macOS GameController owns them exclusively
#   (ioreg: UsbExclusiveOwner=XboxUSBDevice). Connected event fires, 0 input. Works on
#   Linux/Windows or with a generic-HID (DInput) pad. To re-diagnose, recreate a gilrs probe:
#   a console binary using gilrs::Gilrs (gilrs-core self-pumps a CFRunLoop thread on macOS).

# Next action (optional, none scheduled): survivor gamepad live-validation needs a
#   gilrs-readable controller (Linux/Windows or a DInput-mode pad), OR a GCController backend.
```

## Session Closed
**Closed at:** 2026-06-09 22:47 KST
**Commit:** `7df2424` (3 commits, pushed to `origin/main`) + this handoff
**Session status:** Handed off. networking-dogfood seq 7 complete — INTERP_DELAY settled (60 ms), gilrs controller-crash fixed (guard + 0.11.2, v4.3.1), macOS gamepad limitation documented, REFERENCE.html updated, lit_dungeon Camera::bounds GUI-verified. survivor gamepad live-validation blocked by macOS platform limitation (code is correct).
