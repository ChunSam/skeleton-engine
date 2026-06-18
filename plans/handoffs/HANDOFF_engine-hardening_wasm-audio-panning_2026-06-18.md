# WebAudio controllable per-source SFX + stereo pan (v0.26.0)

**Date:** 2026-06-18
**Status:** **COMPLETED — implemented, headless-smoke 12/12, CI-equivalent gate green; shipped as v0.26.0 on branch `feat/wasm-audio-panning` (PR pending push/open).**
**Chain:** `engine-hardening` seq `27` · **Parent:** `HANDOFF_engine-hardening_dialogue-portrait_2026-06-18.md` (seq 26)
**Origin:** A-tier item **A2** from the seq-25 feature list (user asked to run it in parallel with the
#113 merge). A step toward native↔wasm audio parity, building on the seq-25 WebAudio foundation.

---

## What shipped
- **`WebAudio::play_sfx(bytes) -> Sfx`** (`src/audio_wasm.rs`) — a *controllable* one-shot SFX
  (the old `play` is fire-and-forget with no handle). Routes `source → StereoPannerNode →
  per-source GainNode → master`. The panner + gain are created **synchronously**, so the returned
  handle's `set_pan`/`set_volume` apply **before** the async decode finishes.
- **`Sfx` handle** (re-exported at crate root `src/lib.rs`): `set_volume(0..1, per-source)`,
  `set_pan(-1 left .. 1 right)`, `is_playing`, `stop`. `#[derive(Clone)]` — a clone controls the
  same sound. Robust fallback: if the per-source nodes can't be created, `gain`/`panner` are `None`,
  volume/pan become no-ops, and the source routes straight to master so it still plays.
- web-sys feature `StereoPannerNode`. `play`/`play_music`/master volume/suspend/resume unchanged.
- Example `web_audio` + `scripts/wasm_audio_smoke.sh` now also drive `play_sfx` (pan/volume/stop) →
  headless lifecycle check is **12/12** (was 9/9).
- **Fixed:** `examples/web_audio/web/build.sh` + `scripts/wasm_audio_smoke.sh` marked executable in
  git (`git update-index --chmod=+x`; they were 0644 like coin_race's), and the smoke script now
  calls `bash "$WEB_DIR/build.sh"` so a missing exec bit on a fresh checkout can't break it.

## Implementation notes / gotchas
- **Async node creation is the crux:** a buffer source only exists after `decode_audio_data`
  resolves, so the *handle* can't hold the source at return time. Solution: create the
  gain+panner **now** (they're independent of the buffer) and wire `panner→gain→master`
  synchronously; store the source into an `Rc<RefCell<Option<…>>>` slot inside the spawned decode
  task. The handle holds the slot, so `is_playing`/`stop` see the source once it starts.
- `connect_with_audio_node` takes `&AudioNode`; `&GainNode`/`&StereoPannerNode` coerce via Deref —
  no cast needed (same as the existing master wiring).
- `Sfx::stop` uses the deprecated `AudioBufferSourceNode::stop` (no non-deprecated binding) →
  `#[allow(deprecated)]`, same as `WebAudio::stop_music`.
- `is_playing` is NOT auto-cleared when a non-looping clip ends naturally (no `onended` wired) —
  same semantics as `is_music_playing`; documented.

## Verification
- `scripts/wasm_audio_smoke.sh` → **PASS (12/12)** (9 original lifecycle + 3 new: play_sfx started,
  set_pan/set_volume applied without dropping the sound, stop stopped it). Reads the verdict from
  the page title over Chrome DevTools (real time — see seq-25 gotcha about virtual-time + audio).
- Full `./scripts/verify.sh` green after the version bump.
- **Human-only residue (unchanged from seq 25):** actually *hearing* the pan/volume — no audio
  capture in the flow. `bash examples/web_audio/web/build.sh && python3 -m http.server 8080
  --directory examples/web_audio/web && open …`, click "Start audio", listen (left-panned beep).

## Files changed
- `src/audio_wasm.rs` (play_sfx + Sfx + doc), `src/lib.rs` (export Sfx), `Cargo.toml`
  (StereoPannerNode + version), `Cargo.lock`, `examples/web_audio/web_audio.rs` (SFX checks),
  `scripts/wasm_audio_smoke.sh` (bash invoke), build.sh/smoke exec bits, `docs/CHANGELOG.md`,
  `CLAUDE.md`.

## Risks & Blockers
- None. Additive, gate green, smoke 12/12. Acoustic confirmation is the only human step.

## Where to go next (seq-25 feature list, optional)
- **C1 isometric/hex tilemap** (biggest unmet demand).
- **B1** `TilemapAutotile { mode }` unification; **B2** UI Tab/focus navigation.
- Further wasm audio: per-source named buses, crossfade on wasm (kira NOT recommended — large swap).
- crates.io publish (irreversible, explicit go).

## Quick start
```bash
cd /Users/jkl/Projects/skeleton-engine
grep -m1 '^version' Cargo.toml        # 0.26.0
./scripts/verify.sh                    # green
bash scripts/wasm_audio_smoke.sh       # PASS (12/12)  — needs Chrome + matching wasm-bindgen-cli
```
