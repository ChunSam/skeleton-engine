# wasm AEAD save/load browser verification + acoustic audio confirmation (v0.27.0)

**Date:** 2026-06-18
**Status:** **COMPLETED — both remaining verification debts CLOSED.** wasm save: autonomous headless
smoke **7/7**; WebAudio acoustic: confirmed by the user in a real browser. Shipped as v0.27.0 on
branch `test/wasm-save-smoke` (PR pending push/open).
**Chain:** `engine-hardening` seq `28` · **Parent:** `HANDOFF_engine-hardening_wasm-audio-panning_2026-06-18.md` (seq 27)
**Origin:** The user asked "are there two untested things?" — yes: (1) the wasm AEAD save
`localStorage` browser round-trip (v0.22.0 deferred — compile-gated only), and (2) WebAudio
acoustic output (no audio capture). This closes BOTH.

---

## What shipped / what was verified

### (1) wasm AEAD save — autonomous browser verification (this release, v0.27.0)
The v0.22.0 cross-platform save path (hex ChaCha20-Poly1305 blob in `localStorage`) had never *run*
in a browser. Now it does, asserted headlessly:
- **Example `wasm_save`** (`examples/wasm_save/`, wasm-only; native `main` is a stub). Exercises the
  real localStorage backend: `save`→`exists`→`load` round-trip, **asserts the stored value is hex
  ciphertext not plaintext** (reads it back via `web_sys::Storage`), **AEAD tamper detection** (it
  overwrites the blob with garbage hex and asserts `load` then errors), and a
  `save_versioned`/`load_migrated` round-trip + `delete`. 7 checks; writes the verdict to the page
  title (`SAVE_CHECK: PASS (7/7)`).
- **`scripts/wasm_save_smoke.sh`** — headless Chrome, reads the title verdict over the DevTools
  `/json` endpoint (same pattern as `wasm_audio_smoke.sh`; no audio/GPU flags needed). **PASS (7/7).**
- **No engine code change** — example + tooling only. (The save API was already cross-platform.)

### (2) WebAudio acoustic output — confirmed by the user (closes the seq-24 P3-3B debt)
The user ran `web_audio` in a real browser, clicked "Start audio", and confirmed sound plays
correctly (master volume + looping music + suspend/resume). The lifecycle was already headless-
verified (12/12, seq 27); the acoustic half is now human-confirmed. **The one P3 debt flagged back
in seq 24 is closed.**

## Notes / gotchas
- The localStorage key on wasm is `path.to_string_lossy()` — the tamper test sets that exact key
  via `web_sys::Storage::set_item`, then calls `engine::load` and expects `Err` (the AEAD auth tag
  rejects it). Confirms tamper detection works through the real browser backend, not just natively.
- Headless Chrome supports `localStorage` with a `--user-data-dir`; no swiftshader/WebGL needed
  (the example has no canvas).
- web-sys `Storage` + `Window` features were already enabled (from the v0.15.0 plaintext path).

## Remaining (optional, NOT debt)
- **A2 stereo pan acoustics:** "sound works" confirmed volume/music; the *pan* specifically wasn't
  isolated by ear (the example's panned beep is stopped quickly by the lifecycle check). The panner
  node wiring is headless-verified. Could add a "left 1s → right 1s" sweep to the demo if a human
  wants to hear the pan — offered, not done.
- Feature backlog (seq-25 list): **C1 iso/hex tilemap** (biggest unmet demand), B1 TilemapAutotile
  mode unify, B2 UI Tab/focus, further wasm audio (named buses/crossfade), crates.io publish.

## Files changed
- New: `examples/wasm_save/wasm_save.rs`, `examples/wasm_save/web/{index.html,build.sh}`,
  `scripts/wasm_save_smoke.sh`. Modified: `Cargo.toml` (example entry + version), `Cargo.lock`,
  `docs/CHANGELOG.md`, `CLAUDE.md` (save module-map row + verification section).

## Verification
- `scripts/wasm_save_smoke.sh` → **PASS (7/7)**. Full `./scripts/verify.sh` green after the bump.

## Quick start
```bash
cd /Users/jkl/Projects/skeleton-engine
grep -m1 '^version' Cargo.toml        # 0.27.0
bash scripts/wasm_save_smoke.sh        # PASS (7/7) — needs Chrome + matching wasm-bindgen-cli
```
