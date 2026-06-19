# Gamepad left analog stick → UI focus navigation (v0.41.0, shipped); macOS gamepad-input limitation diagnosed

**Date:** 2026-06-19
**Status:** COMPLETED — PR #142 merged + green; `main` clean. (Hardware gamepad test deliberately deferred.)
**Bead(s):** none (repo uses `plans/handoffs/`, not beads)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `42`
**Parent:** `HANDOFF_engine-hardening_deferred-cleanups_2026-06-19.md` (seq 41)
**Prior chain:** seq 37 `stretch-trio` > 38 `session-wrap-2` > 39 `visual-audio-verify` > 40 `review-fixes` > 41 `deferred-cleanups` > **42 this (gamepad-stick-ui-nav)**

> This session implemented a **feature** from seq-41's "Where We're Going" item 2 (the seq-38/39
> follow-up "gamepad analog-stick nav, UI focus currently D-pad only"): the UI focus pass now reads
> the **left analog stick** in addition to the D-pad. Shipped as v0.41.0 (PR #142). The real-hardware
> test could NOT be completed — diagnosed a macOS/gilrs/Xbox input limitation (not a bug) and the user
> deferred hardware verification to a future per-OS input-optimization pass.

---

## Since Last Handoff (vs seq-41's "Where We're Going")

Seq-41 left three buckets: (1) crates.io publish, (2) seq-38/39 feature follow-ups [64-tile hex atlas,
**gamepad analog-stick nav**, focus-ring styling], (3) user-only gamepad hardware test.
- **User picked bucket (2) → gamepad analog-stick nav** (via the onboarding AskUserQuestion).
- Shipped it as **v0.41.0** (PR #142, `2dc5a48`). The seq-41 cleanup tail stays fully drained.
- **crates.io still untouched** (bucket 1, unchanged since seq 33). The other two seq-38/39 follow-ups
  (64-tile hex atlas, focus-ring styling) remain open.
- **Bucket (3) materialized as a blocker, not a task:** attempting the hardware test surfaced that
  gilrs receives NO input from a Bluetooth Xbox pad on this macOS box (env limitation). The user
  re-scoped hardware verification into a future per-OS optimization effort.
- The seq-41 gotcha **"R1: a stale verify-exit file is a trap"** recurred and was caught again (see
  Gotchas) — the rule held its value.

## Reference Documents

- `CLAUDE.md` — conventions (now **v1.6.92**, package **v0.41.0**). R1 verify-exit rule (seq 40); module
  map row 122 (UI focus) updated this session.
- Parent `HANDOFF_engine-hardening_deferred-cleanups_2026-06-19.md` (seq 41).
- `docs/VISION.md` — "a feature is not done until an example exercises it"; here the example is `ui_focus`.
- Memory `gilrs-macos-xbox-no-input.md` (written this session) — the macOS gamepad-input limitation.

## The Goal

skeleton-engine is a fork-friendly, MIT, genre-agnostic 2D engine; the engine-hardening arc drains a
post-roadmap backlog. This segment's goal: **make the UI focus system navigable by the left analog
stick** (it was D-pad + keyboard only), shipping it as a clean MINOR release with the example as the
acceptance test — while honestly handling the fact that the analog axis needs edge-detection the D-pad
gets for free, and that real-pad confirmation may not be possible in this dev environment.

## Where We Are

- `main` @ **`2dc5a48`** (PR #142, v0.41.0), package **v0.41.0**, CLAUDE.md header **v1.6.92**, tree clean,
  CI green (4/4: Build WASM · Package dry-run · Rustdoc · Test native 5m48s).
- **Feature:** `UiSystem`'s focus pass folds the **first connected pad's left analog stick** into its
  per-frame `InputSnapshot` next to the existing D-pad. Stick **Up/Down** cycle focus (Up = reverse,
  like Shift+Tab); stick **Left/Right** nudge a focused `Slider`. South (A) still activates.
- **Edge detection:** new `StickNav` (`src/ui/system/state.rs`) — per-axis latched zone (`i8`: −1/0/+1)
  with hysteresis (`STICK_ACTIVATE = 0.6`, `STICK_RELEASE = 0.35`). One push = one focus step; a held
  stick does NOT auto-repeat (it must fall back into the neutral band before firing again).
- `StickNav` lives on `UiSystem` (held across frames, like the scratch buffers); `InputSnapshot::from_world`
  now takes `&mut StickNav` (its only caller is `UiSystem::run`, `src/ui/system.rs:55`).
- **Axis signs match the engine's existing convention** (the `survivor` example's `AxisBinding`s):
  **up = −Y, down = +Y, right = +X** (NOT the "+Y = up" I initially assumed — see What We Tried #3).
- **Test helper:** `#[cfg(test)] GamepadState::test_axis(pad, axis, value)` (`src/input/gamepad.rs`),
  mirroring `test_press`, lets non-`gilrs` tests drive analog input.
- **Tests:** 8 new (4 `StickNav` hysteresis unit tests in `state.rs` + 4 focus-pass integration tests
  in `focus_pass.rs`: down-advances-and-wraps, up-reverses, held-no-repeat, stick-right-nudges-slider).
  **Lib tests 870 → 878.** Verify gate green at every stage (baseline 0/870 → feature 0/878 → ship 0/878).
- **No public API change** — `StickNav` is `pub(super)`; module map row updated but no exported symbol added.
- **Example `ui_focus`** doc comment + on-screen instruction text updated to advertise the left stick
  (font 17→15 to fit the longer line in the 760px window). No code change to the example (it works
  transparently — the engine owns the fold-in).
- **HARDWARE UNVERIFIED:** the analog-stick path was never confirmed against a real pad — gilrs delivers
  no input on this macOS setup (see Evidence). Logic is verified only by tests + survivor sign-match.

## What We Tried (Chronological)

1. **Onboarding + baseline.** Read seq-41 handoff + the 3 files it touched (`audio_wasm.rs` `play_at_to`,
   `focus_pass.rs` `is_focusable`, `autotile.rs` `hex_mask_from_offsets`) + adjacent (`audio_spatial.rs`,
   `ui/system/state.rs` `node_layout`). Ran the verify gate R1-safe (fresh `/tmp/verify_seq41_exit.txt`):
   **0 / 870**. Confirmed parent identifiers all still present (no stale refs).
2. **Designed `StickNav` edge detection.** The core problem: D-pad uses `GamepadState::just_pressed`
   (edge-detected for free); an analog axis is continuous, so pushing-and-holding would auto-repeat
   every frame. Solution: a per-axis latched zone with hysteresis stored on `UiSystem` (persists across
   frames), fed via `from_world(world, &mut StickNav)`. Activate 0.6 / release 0.35.
3. **Got the Y sign WRONG initially, then corrected via cross-check.** First wrote `up = sy > 0`
   (assuming gilrs "+Y = up"). Before the gate, grepped existing stick usage and found
   `examples/games/survivor/survivor.rs`: `MoveUp → AxisBinding::negative(LeftStickY)`,
   `MoveDown → AxisBinding::positive(LeftStickY)`. So **this codebase's convention is up = −Y, down = +Y.**
   Flipped the code (`up = sy < 0; down = sy > 0`) AND the integration tests AND renamed the misleading
   unit test (`stick_axis_signs_match_gilrs` → `stick_zone_carries_value_sign`).
4. **First full gate FAILED (exit file = 1).** The background task notification said "exit code 0" but
   `/tmp/verify_stick_exit.txt` held **1** — `cargo fmt --check` diffs on my long assert lines. (The
   wrapper's "exit 0" is just the trailing `echo`; the file is authoritative — exactly the seq-40 R1
   lesson.) Ran `cargo fmt`, re-ran: green.
5. **Feature gate green: 0 / 878.** All 8 new tests pass; fmt/clippy(native+wasm)/wasm-build/doc clean.
6. **Hardware test attempt → launched `ui_focus`.** `caffeinate -dimsu cargo run --example ui_focus`
   backgrounded; osascript front + `screencapture` confirmed the window renders (5 widgets + focus readout).
7. **User reports "조작이 안돼" with an Xbox pad connected.** Investigated the gilrs plumbing:
   `poll_gilrs` (`src/app/window.rs:728`) IS called every frame from `about_to_wait` (`:499`, with
   `ControlFlow::WaitUntil`). So the plumbing exists.
8. **Raw-gilrs probe** (`examples/gamepad_probe.rs`, throwaway): `gilrs.gamepads()` enumerated **0** at
   `Gilrs::new()` time, but a **`Connected` event WAS emitted** during polling. So gilrs sees the pad.
9. **Engine-level probe** (`examples/gamepad_state_probe.rs`, throwaway, used the full `App`):
   `any_connected = true, primary = Some(0)` — the engine registers the pad — but **`LX=0.00 LY=0.00
   South=false` across 427 frames while the user actively mashed the stick + A.** Input never arrives.
10. **`ioreg -r -c IOHIDDevice` → root cause.** The pad shows as `XboxSeriesXGamepad` /
    `AppleUserHIDDevice` with `IOUserServerName = com.apple.gamecontroller.driver.XboxGamepad`. macOS's
    **GameController framework claims the input**; the IOKit-HID gilrs client gets enumeration (→ Connected)
    but no input reports. Environment limitation, not a feature/engine bug — `survivor`'s gamepad fails
    identically here.
11. **User deferred hardware test** ("게임패드 테스트는 이후에 os별 최적화하면서 같이 진행하자"). Removed both
    throwaway probes, recorded the limitation in memory, ran `/ship` v0.41.0, committed (`bfa074b`),
    pushed, opened PR #142, watched CI (4/4 green), squash-merged → `main @ 2dc5a48`.

## Key Decisions

- **Edge detection via hysteresis (0.6/0.35), not a single threshold.** A single threshold would
  chatter on a stick resting near the edge; the activate>release band guarantees one step per deliberate
  push. State is per-axis `i8` zone latched on `UiSystem`. Rejected: a World resource (adds a globally
  visible resource + auto-insertion for an internal detail) — threading `&mut StickNav` through the one
  caller is cleaner and keeps it `pub(super)`.
- **Match the engine's existing axis-sign convention (`survivor`), not "textbook gilrs."** The ground
  truth for the engine is its own working bindings, so a user's pad behaves consistently across
  `survivor` and the UI. up=−Y/down=+Y/right=+X.
- **Honest caveat: unit tests can't catch a wrong sign.** Test + code flip together, so all 8 pass
  either way. Sign correctness rests on (a) matching survivor and (b) the deferred hardware test. This
  was surfaced to the user explicitly (the user values honest gap-naming).
- **MINOR bump v0.41.0** (additive feature, 0.x cadence) — `/ship`'s four-edit set + module-map row
  (row 122). No tag (none requested).
- **Ship despite no hardware verification.** Diagnosed the blocker as environmental (gilrs can't read
  the GC-framework-claimed pad on macOS), confirmed it's not a regression (survivor hits the same wall),
  and the user chose to ship on logic-level verification + defer the real test. Recorded in memory so a
  future per-OS session resumes without re-diagnosing.
- **Probes were throwaway, removed before commit.** `examples/gamepad_probe.rs` (raw gilrs) and
  `examples/gamepad_state_probe.rs` (engine-level) were diagnostic only; deleted so the PR is just the feature.

## Evidence & Data

### Verify gates (R1-safe, exit read from file not wrapper)
| stage | file | exit | lib tests |
|---|---|---|---|
| onboarding baseline | `/tmp/verify_seq41_exit.txt` | 0 | 870 |
| feature, 1st run (FAILED) | `/tmp/verify_stick_exit.txt` | **1** | — (fmt diff, tests never ran) |
| feature, after `cargo fmt` | `/tmp/verify_stick2_exit.txt` | 0 | 878 |
| after `/ship` v0.41.0 | `/tmp/verify_ship41_exit.txt` | 0 | 878 |

### Gamepad probe results (the diagnosis)
| probe | observation |
|---|---|
| raw gilrs `gamepads()` at `new()` | **0** enumerated |
| raw gilrs `next_event()` | emits a `Connected` event (deferred) |
| engine `GamepadState` | `any_connected=true, primary=Some(0)` |
| engine left-stick + South over 427 frames of active input | `LX=0.00 LY=0.00 South=false` (never changed) |
| `ioreg -r -c IOHIDDevice` | `XboxSeriesXGamepad` / `AppleUserHIDDevice` / `IOUserServerName = com.apple.gamecontroller.driver.XboxGamepad` |

### Ship / merge
| item | value |
|---|---|
| commit | `bfa074b` → squashed to `2dc5a48` (#142) |
| PR | #142, 9 files, +293/−23 |
| CI | Build WASM 47s · Package dry-run 1m5s · Rustdoc 34s · Test native 5m48s — all pass |

### StickNav unit-test expectations (the hysteresis contract, ground truth)
```
update(0,-0.8) → (0,-1)   first push fires
update(0,-0.8) → (0, 0)   held: no repeat
update(0,-0.5) → (0, 0)   in band (0.35..0.6): no fire
update(0, 0.0) → (0, 0)   neutral: resets latch
update(0,-0.8) → (0,-1)   push again fires
update(0.9,0)/(-0.9,0) → (1,0)/(-1,0)   slam to opposite without neutral still fires
```

## Code Analysis

- **`StickNav::step_axis`** (`state.rs`): `zone = if v>=0.6 {1} else if v<=-0.6 {-1} else if v.abs()<=0.35
  {0} else {*latched}` (the `else` is the hysteresis band — hold prior zone). `fired = (zone!=0 &&
  zone!=*latched) ? zone : 0`; then `*latched = zone`. Returns the per-axis fired step.
- **`InputSnapshot::from_world`** (`state.rs`) gamepad fold-in: `let (sx,sy)=stick.update(axis(LeftStickX),
  axis(LeftStickY)); up = DPadUp || sy<0; down = DPadDown || sy>0; if down||up {tab=true; shift|=up};
  nav_left |= DPadLeft || sx<0; nav_right |= DPadRight || sx>0; activate |= South`.
- **gilrs pump** (`src/app/window.rs:728 poll_gilrs`, called from `about_to_wait:499`): drains
  `gilrs.next_event()` inside `catch_unwind` (a flaky controller disables gamepad, doesn't crash), then
  `GamepadState::process_event` per event. `process_event` registers a slot ONLY on `EventType::Connected`
  — it does **not** enumerate `gilrs.gamepads()`. (This was NOT the bug — gilrs deferred-emits Connected —
  but it's a fragile pattern worth noting for the per-OS work.)
- **`GamepadState::axis`** returns raw −1..1, no deadzone (deadzone is applied per-feature; `StickNav`'s
  release threshold is the UI deadzone).
- Slider nudge step = `(max-min) * SLIDER_STEP_FRAC` where `SLIDER_STEP_FRAC = 0.05` (`focus_pass.rs`).
- `collect_focusables` sorts by `Entity::index`; `is_focusable` binary-searches that (seq-41's B).

## Files Changed (PR #142)

### Source
- `src/ui/system/state.rs` — `StickNav` + `step_axis` + `STICK_ACTIVATE`/`STICK_RELEASE`; `from_world`
  takes `&mut StickNav`; gamepad fold-in reads the left stick; +4 unit tests.
- `src/ui/system.rs` — `UiSystem.stick_nav: StickNav` field; passes `&mut self.stick_nav` to `from_world`.
- `src/input/gamepad.rs` — `#[cfg(test)] test_axis` helper.
### Tests
- `src/ui/system/focus_pass.rs` — `set_stick` helper + 4 integration tests; `GamepadAxis`/`Slider` imports.
### Example & docs
- `examples/ui_focus.rs` — doc + on-screen text advertise the left stick.
- `CLAUDE.md` — module map row 122 (UI focus) + header v1.6.92 / package v0.41.0.
- `docs/CHANGELOG.md` — 0.41.0 entry (incl. the hardware-deferred note).
- `Cargo.toml` / `Cargo.lock` — v0.41.0.
### Throwaway (created + deleted this session, NOT in the PR)
- `examples/gamepad_probe.rs`, `examples/gamepad_state_probe.rs` — gilrs diagnostics.

## User Feedback & Preferences

- **"게임패드 아날로그 nav"** — chose this seq-38/39 follow-up from the onboarding question.
- **"계획 진행하고 테스트 할 때 알려줘"** — proceed with the plan; notify when it's time to (hardware) test.
- **"테스트 해보게 게임 실행해줘"** — wanted me to launch the example so they could test with their pad.
- **"xbox 게임 패드를 연결 해 놓았는데, 게임에서 조작이 안돼"** — the bug report that triggered the diagnosis.
- **"mac os 정책으로 현재 유선연걸인데도 블루투스로 잡는 것 같아. 게임패드 테스트는 이후에 os(windows, mac)별
  최적화하면서 같이 진행하자"** — confirmed the env diagnosis; **deferred gamepad hardware testing to a
  future per-OS optimization pass**; ship on logic verification now.
- **"/handoff 푸시"** — close with a handoff, committed/pushed.
- Standing: Korean for user-facing reports/questions; merge standing-delegated as a DIRECT instruction
  (never an AskUserQuestion option — Korean classifier misread); honest gap-naming valued.

## Where We're Going

1. **Per-OS (Windows/Mac) input optimization** — the user's named next effort. Includes: (a) a gilrs
   **GameController-framework backend** (or alternative) so modern Xbox/PS5 pads deliver input on macOS,
   and (b) **the deferred real-hardware test of THIS feature** — confirm the analog-stick Y sign on a
   pad that actually reports input (logic matches `survivor`, but unconfirmed). See `gilrs-macos-xbox-no-input` memory.
2. **crates.io publish** — the one persistent untouched backlog item (irreversible, needs explicit go;
   publish `engine_reflect_derive` too). Package dry-run CI passes on every PR.
3. **Remaining seq-38/39 follow-ups (open):** 64-tile hex atlas asset, focus-ring styling (the
   `RING_COLOR`/`RING_THICKNESS` constants in `focus_pass.rs` are hardcoded).

## Risks & Blockers

- **Gamepad input is unverifiable on this macOS box** with a BT Xbox pad (GameController framework
  claims it). Not blocking the merge (env, not code), but the feature's runtime correctness — especially
  the Y sign — is **unconfirmed on hardware**. A working pad (wired generic HID, or another OS) or the
  per-OS backend work is needed to verify.
- **crates.io is irreversible** — do not publish without an explicit user go.
- The seq-40/41 merge-classifier issue (Korean AskUserQuestion misread) did NOT recur — direct-instruction
  merge held.

## Open Questions

- **Does the analog-stick Y sign behave correctly on a real, input-delivering pad?** Logic matches
  `survivor`'s `AxisBinding` convention (up=−Y), but no hardware confirmed it this session. (Resolve
  during the per-OS work.)
- Is gilrs 0.11 the right gamepad backend for macOS at all, or should the per-OS pass adopt the
  GameController framework directly? (Open for the future effort.)

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 2dc5a48 (#142 v0.41.0) … 9f05f93 (#141) … 9ff2806 (#140)
grep -m1 '^version' Cargo.toml  # 0.41.0
./scripts/verify.sh > /tmp/v.log 2>&1; echo $?   # 0  (R1: read the exit, don't pipe; use a FRESH path)

# Key files for this feature:
#   src/ui/system/state.rs       (StickNav + from_world fold-in + 4 unit tests)
#   src/ui/system/focus_pass.rs  (set_stick helper + 4 integration tests)
#   src/input/gamepad.rs         (test_axis; poll path is src/app/window.rs:728 poll_gilrs)
#   examples/games/survivor/survivor.rs  (the axis-sign convention: MoveUp = negative LeftStickY)

# Memory to recall before per-OS gamepad work:
#   memory/gilrs-macos-xbox-no-input.md  (why hardware test is deferred; the GC-framework claim)

# Next action (pick one — nothing is required; the feature is shipped & green):
#   (A) per-OS input optimization + the deferred hardware test of the analog-stick Y sign, OR
#   (B) crates.io publish (explicit go required), OR
#   (C) another seq-38/39 follow-up (64-tile hex atlas / focus-ring styling).
```

---

## Cross-cutting gotchas (expensive-to-rediscover)

1. **R1 recurred and was caught.** The background-task notification prints "exit code 0" — that's the
   *wrapper's* trailing `echo`, NOT the gate's verdict. The gate's exit lived in the file as **1** (an
   `fmt --check` diff). Always read the exit *file* (and `rm` it / use a fresh path first so you don't
   read a prior run's value). Bit me once this session (caught), as designed by the seq-40 rule.
2. **Analog axis sign = the engine's `survivor` convention, NOT textbook gilrs.** `survivor` binds
   `MoveUp → negative(LeftStickY)`, so in this codebase **up = −Y, down = +Y** (the opposite of the
   "+Y = up" I first coded). Grep `LeftStickY` in `examples/` before assuming a sign.
3. **Unit tests cannot catch a wrong analog sign** — flipping the code and the test together keeps them
   green. Sign correctness is only provable by matching the engine convention + a real-hardware test.
   Don't read "8 tests pass" as "sign confirmed."
4. **gilrs on macOS: `gamepads()` is empty at `Gilrs::new()`, but a `Connected` event arrives shortly
   after** (deferred). So relying on the event (as the engine does) DOES register pre-connected pads —
   that part works. The failure is one layer deeper (input never flows; GameController framework claims it).
5. **gilrs + BT Xbox + macOS = enumeration only, zero input.** `any_connected=true` but every axis/button
   reads neutral. `ioreg` shows `IOUserServerName = com.apple.gamecontroller.driver.XboxGamepad`. This is
   environmental; do not chase it as a feature bug. (Recorded in memory `gilrs-macos-xbox-no-input`.)
6. **Test-helper subtleties for analog UI tests:** (a) reuse the SAME `UiSystem` instance across frames
   (the `StickNav` zone state lives on it — a fresh `UiSystem::new()` each frame resets the latch and
   breaks held-no-repeat tests); (b) `set_stick` mutates the existing `GamepadState` (axes persist across
   frames, mirroring the real input flush which clears just-pressed but keeps axes).

## Process / versioning notes

- 0.x cadence: additive feature = MINOR → **v0.41.0**. `/ship` four-edit set (Cargo.toml + lock +
  CHANGELOG + CLAUDE.md header) + module-map row 122. No tag (none requested).
- VISION loop honored: the feature's acceptance test is the `ui_focus` example (updated to advertise the
  stick). It builds + runs + renders (screenshotted); only the *gamepad input* couldn't be exercised here.
- Diagnostic examples (`gamepad_probe`, `gamepad_state_probe`) were created to isolate raw-gilrs vs
  engine-`GamepadState` vs OS, then deleted — the PR is the feature only. Re-create them from this
  handoff's probe descriptions if the per-OS work needs them again.
- Squash-merge via plain `gh pr merge --squash --delete-branch` after CI 4/4 green — no classifier block
  (merge driven by direct instruction, never a Korean AskUserQuestion option).

---

## Session Closed
**Closed at:** 2026-06-19 14:33 KST
**Commit:** feature `2dc5a48` (#142, v0.41.0); this handoff committed separately + pushed.
**Session status:** Handed off to next session
