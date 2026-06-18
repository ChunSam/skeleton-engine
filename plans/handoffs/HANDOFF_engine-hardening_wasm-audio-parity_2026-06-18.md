# wasm audio parity — bus ducking (v0.34.0) + 2D positional (v0.35.0)

**Date:** 2026-06-18
**Status:** COMPLETED — both releases merged + tagged, `main` clean & green
**Bead(s):** none (repo uses `plans/handoffs/`, not beads)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `35`
**Parent:** `HANDOFF_engine-hardening_wasm-audio-depth_2026-06-18.md` (seq 34)

> Two-feature arc porting the **remaining native-only audio** to the wasm `WebAudio` path — the
> seq-34 "Where We're Going" item #1. After this, the wasm mixer reaches native parity for the
> common cases; only **automatic sidechain** stays native-only (by design, see below).

---

## The Goal

skeleton-engine is a fork-friendly, MIT, genre-agnostic 2D engine. The native rodio `AudioManager`
had three mixer features the wasm `WebAudio` lacked after seq 34 (buses + crossfade): **bus
ducking**, **automatic sidechain**, and **2D positional** audio. The user picked the "remaining
native-only audio" backlog item ("1번 진행", with standing "테스트 완료되면 머지까지 진행" approval).
Shipped ducking + positional as two MINOR releases; **deliberately did NOT port automatic sidechain**
(rationale below).

## Where We Are

- `main` @ `5abbcae`, package **v0.35.0**, CLAUDE.md header **v1.6.84**, tree clean, CI green.
- **2 PRs merged + tagged**:
  - **v0.34.0** (#124, merge `69f3b9f`) — wasm bus ducking.
  - **v0.35.0** (#125, merge `5abbcae`) — wasm 2D positional audio.
- Headless wasm audio smoke grew **28 → 35** (`scripts/wasm_audio_smoke.sh`, PASS). Same as the
  whole wasm-audio arc: the smoke is the test (no native units — wasm path); acoustic = human-only.

## What We Did

### v0.34.0 — bus ducking (#124)
- `WebAudio::duck_bus(bus, gain, attack_secs)` / `release_bus(bus, release_secs)` / `bus_duck(bus)`.
- **Buses became a two-gain chain** `duck → volume → master` (`struct Bus { volume, duck }`, was a
  single `GainNode`). Sounds connect to `duck` (the input via `bus_input`); `set_bus_volume` drives
  `volume`; ducking ramps `duck` — so the duck multiplier is independent of bus volume (native
  parity). `bus_entry` lazy-creates+wires the chain; `bus_input` returns the duck node.
- Ramps reuse `ramp_gain_to`, on the audio clock — no per-frame `update()`.
- **`set_sidechain` NOT ported** — needs continuous "is the trigger bus playing?" evaluation every
  frame; doesn't fit Web Audio's fire-and-forget model, and music isn't bus-routed on wasm. Manual
  `duck_bus`/`release_bus` (driven from game logic) covers the practical need. Documented in the
  module doc + CHANGELOG "Not ported" section.

### v0.35.0 — 2D positional (#125)
- `WebAudio::play_at(bytes, source, listener, max_dist) -> Sfx` — `play_sfx` then apply spatial
  vol+pan via `Sfx::update_position`. Routes to master.
- `Sfx::update_position(source, listener, max_dist)` — recompute + apply (track a moving source).
- `Sfx::volume()` / `Sfx::pan()` getters.
- Private free fn `spatial_params(source, listener, max_dist) -> (vol, pan)` (mirrors native:
  `vol = (1 - clamp(dist/max)).max(0)`, `pan = clamp(dx/max, -1, 1)`). Added `use glam::Vec2;`.
- Built entirely on the existing `Sfx` per-source gain + stereo panner — additive.

## Key Decisions

- **Two MINOR releases** (ducking, then positional), per the session's per-item cadence + the
  squash-merge-diverges-base constraint (branch positional off fresh main only AFTER #124 merged).
- **Drop automatic sidechain on wasm** — honest scope cut. It's the one native audio feature whose
  model (per-frame trigger eval) is fundamentally a poor fit for Web Audio. Surfaced explicitly to
  the user rather than shipping fragile `onended`-counting code.
- **Ducking is instant when `dur <= 0`** (`set_value`), ramped otherwise — both a sensible API
  detail (0s attack = instant) AND the key to headless testability (see gotcha #1).

## Evidence & Data
| seq | ver | PR | merge | item | smoke | verify |
|---|---|---|---|---|---|---|
| 35a | 0.34.0 | #124 | `69f3b9f` | wasm bus ducking | 28/28 | green |
| 35b | 0.35.0 | #125 | `5abbcae` | wasm 2D positional | 35/35 | green |

## Files Changed
### Source
- `src/audio_wasm.rs` — `Bus { volume, duck }` + `bus_entry`/`bus_input` + `duck_bus`/`release_bus`/
  `bus_duck` + `ramp_gain_to` instant-when-`dur<=0` branch (#124); `play_at` + `spatial_params` +
  `Sfx::update_position`/`volume`/`pan` + `use glam::Vec2` (#125).
### Example + tooling
- `examples/web_audio/web_audio.rs` — +6 ducking checks (#124), +7 positional checks (#125).
- `scripts/wasm_audio_smoke.sh` — header updated; verdict now PASS (35/35).
### Docs
- `docs/CHANGELOG.md` (0.34.0 + 0.35.0), `CLAUDE.md` (header v1.6.82→v1.6.84, audio module-map row).

## Code Analysis (cross-cutting facts worth keeping)
- **`WebAudio` graph now**: sound → (optional panner + per-source gain) → [bus `duck` → bus
  `volume` →] master → destination; music → per-track gain → master. Buses are `Bus { volume, duck }`.
- **`ramp_gain_to(gain, target, dur)`** is the one ramp helper (crossfade + ducking). `dur <= 0`:
  `cancel_scheduled_values(0) + set_value(target)` (instant, readable). `dur > 0`:
  `set_value_at_time(current, now) + linear_ramp_to_value_at_time(target, now+dur)`.
- **`spatial_params`** is a private free fn shared by `play_at` + `Sfx::update_position`.
- No new web-sys feature needed (`cancel_scheduled_values` rides the existing `AudioParam`; `glam`
  is a direct dep, `Vec2` also re-exported at `engine::Vec2`, lib.rs:56).

## Where We're Going (next session — all optional, none committed)
1. **wasm audio is effectively done.** Only **automatic sidechain** remains native-only (documented
   poor-fit). If ever wanted: an `onended`-counting active-source tracker per bus could drive it, but
   it's fragile and music isn't bus-routed — likely not worth it.
2. **crates.io publish** — still deferred; irreversible; needs explicit go (also publish
   `engine_reflect_derive`).
3. **Stretch (unchanged):** gamepad UI focus nav; flat-top hex; autotile across iso+hex; focus-ring
   styling; positional `play_at_on_bus` (route positional sounds through a bus).

## Risks & Blockers
- None. Tree clean, CI green, both tags pushed. Auto-merge still disabled (manual wait-green-merge).

## Gotchas (this arc)
1. **Headless SwiftShader does NOT advance `AudioParam` automation** — a scheduled `linearRamp`'s
   live value is computed on the audio render thread, which doesn't run headless, so reading
   `gain.value()` during a ramp returns the anchor, not the ramped value. (The first ducking smoke
   FAILED on exactly this.) Direct `set_value` (`value=`) IS reflected immediately. Fix: ducking is
   instant for `dur<=0` (set_value), and the smoke uses `dur=0.0` for deterministic reads. The
   smooth ramp, like acoustic output, is a by-ear/real-browser check. **This is why positional
   (set_volume/set_pan = set_value) is fully headless-verifiable but a duck ramp is not.**
2. **`./scripts/verify.sh | tail` masks the real exit code** (reports tail's 0) — run
   `verify.sh > log 2>&1; echo $?`. (Same lesson as seq 34; reconfirmed.)

## Quick Start for Next Session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4            # 5abbcae (#125) 69f3b9f (#124) …
grep -m1 '^version' Cargo.toml  # 0.35.0
./scripts/verify.sh             # green
bash scripts/wasm_audio_smoke.sh   # PASS (35/35) — needs Chrome + matching wasm-bindgen-cli
# Key file: src/audio_wasm.rs (WebAudio: Bus{volume,duck} ducking; play_at/spatial_params; Sfx)
```
