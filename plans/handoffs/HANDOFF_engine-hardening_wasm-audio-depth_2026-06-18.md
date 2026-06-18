# wasm audio depth — named mixer buses (v0.32.0) + track-to-track crossfade (v0.33.0)

**Date:** 2026-06-18
**Status:** COMPLETED — both releases merged + tagged, `main` clean & green
**Bead(s):** none (repo uses `plans/handoffs/`, not beads)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `34`
**Parent:** `HANDOFF_engine-hardening_session-wrap_2026-06-18.md` (seq 33)

> Two-feature arc continuing the "further wasm audio" backlog item the seq-33 wrap left open
> (`⬜ further wasm audio (named buses / wasm crossfade)`). Both halves are now shipped.

---

## The Goal

skeleton-engine is a fork-friendly, MIT, genre-agnostic 2D engine ("the example is the acceptance
test"). The wasm audio path (`WebAudio`, `src/audio_wasm.rs`) was the browser counterpart to the
native rodio `AudioManager`, but lagged it on two mixer features: **named buses** and **track-to-track
crossfade** were native-only. This arc closed both, bringing the wasm path to mixer parity for the
common cases. The user picked "wasm 오디오 심화" off the seq-33 backlog and approved carrying each item
through merge ("진행 하고 테스트 완료되면 머지까지 진행"); split into two MINOR releases per the repo's
established per-item cadence (buses first, then crossfade off fresh main).

## Where We Are

- `main` @ `db1ea90`, package **v0.33.0**, CLAUDE.md header **v1.6.82**, tree clean, CI green.
- **2 PRs merged + tagged** this arc:
  - **v0.32.0** (#121, merge `b62b993`) — wasm named mixer buses.
  - **v0.33.0** (#122, merge `db1ea90`) — wasm track-to-track music crossfade.
- Both tags pushed (annotated, `vX.Y.Z — desc (#NN)` on the squash-merge commit).
- Verification: `./scripts/verify.sh` green for each; the **headless wasm audio smoke grew 12 → 19 →
  22** (`scripts/wasm_audio_smoke.sh`, PASS). No native unit tests added — the wasm `WebAudio` path
  can't be exercised by `cargo test` on the native target, so the browser smoke IS the test (plus the
  `wasm32` build gate). Acoustic output remains a human-only step (no audio capture).

## What We Did

### v0.32.0 — named mixer buses (#121)
A **bus is a named `GainNode`** wired `bus → master`. Web Audio is already a node graph, so this is
*more* natural than the native sink-volume-multiply model and needs **no per-frame `update()` tick**
(the native fade-skip-during-bus-change complexity simply doesn't exist here).
- `WebAudio::set_bus_volume(bus, v)` (clamped `0.0..=1.0`) / `bus_volume(bus)` (read-only, `1.0` if
  absent, does NOT create) / `bus_names()` (sorted) — native `AudioManager` mixer parity.
- `WebAudio::play_on_bus(bytes, bus)` (fire-and-forget) / `play_sfx_on_bus(bytes, bus) -> Sfx`
  (controllable) — route through a bus: `source → [panner → per-source gain →] bus gain → master`.
- Buses are **lazy-created** on first reference (`bus_gain` private helper); a **volume-only bus**
  (set without ever playing on it) persists in `bus_names` (matches native `collect_bus_names`).
- Refactor: factored `play_sfx_to(bytes, dest)` and generalized `decode_then_play` → a `dest:
  &GainNode` param so master vs bus is just a different destination node. `&GainNode` deref-coerces
  to `&AudioNode` in `connect_with_audio_node`, so master and buses share one code path.

### v0.33.0 — track-to-track music crossfade (#122)
- `WebAudio::crossfade_music(bytes, dur)` — old track fades **out** (then stops) while the new track
  fades **in**; nothing playing = plain fade-in.
- Music now routes through a **dedicated per-track `GainNode`** so its volume can be ramped
  independently. Storage changed from a bare source to `struct MusicChannel { source, gain }`.
- Fades are scheduled on the **Web Audio clock** via `AudioParam::set_value_at_time` +
  `linear_ramp_to_value_at_time` — so, unlike native `crossfade` (temp channel + per-frame `update`
  ticking `Fade`), there is **no `update()` tick and no temp channel**.
- Crux: the fade-in ramp must be scheduled at the source's **actual start time** (inside the async
  decode closure, anchored with `ctx.current_time()`), not at call time — otherwise a slow decode
  races the ramp. `start_music(bytes, fade_in: Option<f64>)` is the shared path for `play_music`
  (None) and `crossfade_music` (Some(dur)).
- `decode_then_play_to` was simplified back to one-shot-SFX only (dropped the `looping`/`is_music`
  params) since music now has its own `start_music` path.

## Key Decisions

- **Split into two MINOR releases** (buses, then crossfade) — matches the repo's per-item cadence and
  the squash-merge-diverges-base constraint (branch crossfade off fresh main only AFTER #121 merged).
- **Buses model = one GainNode per bus**, chosen per-play via `*_on_bus` variants, rather than
  mirroring native's persistent named *channels* + `assign_bus`. wasm `WebAudio` is fire-and-forget
  per sound; "pick the bus when you play" is the idiomatic Web Audio mapping.
- **Crossfade rides the audio clock**, not a JS timer or an engine system tick — the whole point of
  doing it in Web Audio instead of porting the native `Fade`/`update` machinery.
- **Smoke is the test** for the wasm path; no `#[cfg(test)]` units (they'd never run — native target).

## Files Changed
### Source
- `src/audio_wasm.rs` — buses (`buses` field + `bus_gain`/`set_bus_volume`/`bus_volume`/`bus_names` +
  `play_on_bus`/`play_sfx_on_bus` + `play_sfx_to`/`decode_then_play_to` refactor) [#121]; crossfade
  (`MusicChannel` struct + `crossfade_music`/`start_music`/`ramp_gain_to`/`stop_at`, music field type
  change, `play_music`/`stop_music` updated) [#122].
### Example + tooling
- `examples/web_audio/web_audio.rs` — drives + self-checks buses (7 checks) then crossfade (3 checks).
- `scripts/wasm_audio_smoke.sh` — header updated; verdict now PASS (22/22).
### Docs
- `docs/CHANGELOG.md` (0.32.0 + 0.33.0), `CLAUDE.md` (header v1.6.80→v1.6.82, audio module-map row).

## Evidence & Data
| seq | ver | PR | merge | item | smoke | verify |
|---|---|---|---|---|---|---|
| 34a | 0.32.0 | #121 | `b62b993` | wasm named mixer buses | 19/19 | green |
| 34b | 0.33.0 | #122 | `db1ea90` | wasm track-to-track crossfade | 22/22 | green |

## Code Analysis (cross-cutting facts worth keeping)
- **`WebAudio` graph** (`src/audio_wasm.rs`, wasm-only): every sound → (optional panner+per-source
  gain) → **master GainNode** → destination. Buses insert a named GainNode between sound and master.
  Music inserts a per-track GainNode (held in `MusicChannel`).
- **No new web-sys feature needed** — `AudioParam::{set_value_at_time, linear_ramp_to_value_at_time,
  value, set_value}`, `AudioContext::current_time`, `AudioBufferSourceNode::stop_with_when` all ride
  features already enabled by existing usage. `stop_with_when` is under the same `#[allow(deprecated)]`
  umbrella as the existing `stop()` calls (web-sys deprecates the `stop*` family with no replacement).
- **`#[allow(deprecated)]`** sits on `stop_music`, `Sfx::stop`, and the new `stop_at` helper.

## Where We're Going (next session — all optional, none committed)
1. **The "further wasm audio" backlog item is now fully done** (buses + crossfade). Remaining
   native-only audio: **ducking/sidechain** and **full positional audio** — could be ported next if
   wasm audio depth is still wanted, but no one has asked.
2. **crates.io publish** — still deferred; irreversible; needs explicit go. Publish
   `engine_reflect_derive` too so `cargo add` users get `#[derive(Reflect)]`.
3. **Stretch (unchanged from seq 33):** gamepad UI focus nav; flat-top hex variant; autotile across
   iso+hex; focus-ring styling knobs.

## Risks & Blockers
- None. Tree clean, CI green, both tags pushed.
- Auto-merge still disabled on the repo — merges done via the "wait green then `gh pr merge --squash`"
  background task (used for both #121 and #122).

## Gotchas (this arc)
1. **`| tail` masks the real exit code.** `./scripts/verify.sh | tail -12` reports `tail`'s exit (0),
   hiding a `cargo fmt --check` failure. Run `verify.sh > log 2>&1; echo $?` to get the true verdict.
   (A `check!` macro line in the example needed multi-line rustfmt formatting; `cargo fmt` fixed it.)
2. **Schedule the music fade-in ramp at the real start time** (`ctx.current_time()` inside the async
   decode closure), not at call time — else a slow `decode_audio_data` races the ramp.
3. **Smoke verdict count is dynamic** — the script matches the `AUDIO_CHECK: PASS` prefix, so adding
   checks (12→19→22) needs no script change; only the header comment was updated for accuracy.

## Quick Start for Next Session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4            # db1ea90 (#122) b62b993 (#121) …
grep -m1 '^version' Cargo.toml  # 0.33.0
./scripts/verify.sh             # green
bash scripts/wasm_audio_smoke.sh   # PASS (22/22) — needs Chrome + matching wasm-bindgen-cli
# Key file: src/audio_wasm.rs (WebAudio: buses + MusicChannel/crossfade_music)
```
