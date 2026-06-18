# wasm positional audio on a mixer bus (v0.36.0)

**Date:** 2026-06-18
**Status:** COMPLETED — merged + tagged, `main` clean & green
**Bead(s):** none (repo uses `plans/handoffs/`)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `36`
**Parent:** `HANDOFF_engine-hardening_wasm-audio-parity_2026-06-18.md` (seq 35)

> Single-method stretch item (user picked "positional play_at_on_bus" from the seq-35 stretch list).
> Tiny additive composition of the positional + bus paths shipped earlier in the wasm-audio arc.

---

## The Goal
After the wasm audio path reached native parity (seq 35), the user chose the smallest stretch
follow-up: let positional sounds ride a named mixer bus (so a positional SFX can be grouped/ducked).

## Where We Are
- `main` @ `a64ba75`, package **v0.36.0**, CLAUDE.md header **v1.6.85**, tree clean, CI green.
- **1 PR merged + tagged**: v0.36.0 (#127, merge `a64ba75`).
- Headless wasm audio smoke **35 → 38** (`scripts/wasm_audio_smoke.sh`, PASS).

## What We Did
- `WebAudio::play_at_on_bus(bytes, source, listener, max_dist, bus) -> Sfx` (`src/audio_wasm.rs`):
  `play_sfx_on_bus(bytes, bus)` then `Sfx::update_position(source, listener, max_dist)`. The positional
  vol/pan live on the per-source `Sfx` nodes (independent of the bus level); the bus's
  `set_bus_volume`/`duck_bus` scale the group downstream. Mirrors `play_at` exactly but via the bus
  input instead of master. Additive — nothing else changed.

## Evidence & Data
| seq | ver | PR | merge | item | smoke | verify |
|---|---|---|---|---|---|---|
| 36 | 0.36.0 | #127 | `a64ba75` | wasm `play_at_on_bus` | 38/38 | green |

## Files Changed
- `src/audio_wasm.rs` — `play_at_on_bus` + module-doc/link.
- `examples/web_audio/web_audio.rs` — +3 checks (playing, right→+pan through a bus, per-source volume
  = spatial value independent of bus volume).
- `scripts/wasm_audio_smoke.sh` — header; verdict now 38/38.
- `docs/CHANGELOG.md` (0.36.0), `CLAUDE.md` (header v1.6.85, audio row).

## Where We're Going (next session — all optional, none committed)
1. **wasm audio is done** (parity + this bus-positional convenience). Only auto-sidechain stays
   native-only (documented poor fit).
2. **crates.io publish** — deferred, irreversible, needs explicit go (also `engine_reflect_derive`).
3. **Remaining stretch items** (user picked only `play_at_on_bus` from the seq-35 list — these are
   still open): gamepad UI focus nav; flat-top hex; autotile across iso+hex; focus-ring styling.

## Risks & Blockers
- None. Tree clean, CI green, tag pushed. Auto-merge still disabled (manual wait-green-merge).

## Quick Start for Next Session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # a64ba75 (#127) …
grep -m1 '^version' Cargo.toml  # 0.36.0
./scripts/verify.sh             # green
bash scripts/wasm_audio_smoke.sh   # PASS (38/38)
```
