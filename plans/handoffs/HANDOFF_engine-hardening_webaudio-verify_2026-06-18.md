# WebAudio runtime verification + first example (v0.24.0)

**Date:** 2026-06-18
**Status:** **COMPLETED — implemented, verified, CI-equivalent gate green; shipped as v0.24.0 on branch `feat/web-audio-verify` (PR open, merge pending user approval).**
**Chain:** `engine-hardening` seq `25` · **Parent:** `HANDOFF_engine-hardening_p3-wasm-parity_2026-06-18.md` (seq 24)
**Context:** seq 24 closed the P1/P2/P3 roadmap and flagged ONE open item — *3B WebAudio runtime
(music/volume) needs a browser + ears.* This session closes the autonomously-verifiable part of
that debt and fills the missing example.

---

## What shipped

### The verification debt, closed (as far as a script can)
`WebAudio` (v0.23.0) shipped compile-checked only — nothing ever *ran* it in a browser, and it had
no example (a VISION-loop gap: "a feature is not done until an example exercises it"). Now:

- **Example `web_audio`** (`examples/web_audio/`, wasm-only; registered in `Cargo.toml`,
  native build is a stub `main` that prints build instructions). It generates an **in-memory sine
  WAV** (no audio asset needed) and drives the whole `WebAudio` surface in sequence — `new` →
  volume default/set/clamp → `resume` → `play_music` (looping) → `suspend` → `resume` — writing a
  ✅/❌ line per step into `#status` and a verdict into the **document title**
  (`AUDIO_CHECK: PASS (9/9)` / `...FAIL: <step>`). Leaves music playing at the end so a human can
  hear it. Web scaffolding: `examples/web_audio/web/{index.html,build.sh}`.
- **Harness `scripts/wasm_audio_smoke.sh`** — optional local (non-CI) check. Builds the example to
  wasm, serves it, runs headless Chrome, and asserts the lifecycle by reading the title verdict
  **live over Chrome's DevTools `/json` endpoint**. **4 consecutive runs → PASS (9/9).**

### Two new public accessors (genuinely useful, not just for the test)
- `WebAudio::is_running()` — context unlocked & not suspended (drives a "tap to enable sound"
  prompt / paused indicator).
- `WebAudio::is_music_playing()` — music channel occupied (music on/off UI), and it makes the
  async `play_music` decode observable to the harness.

### Paperwork
- `Cargo.toml` web-sys feature `AudioContextState` (for `ctx.state()` in `is_running`).
- v0.24.0: Cargo.toml/Cargo.lock/CHANGELOG/CLAUDE.md header (v1.6.73), module-map audio row +
  verification section updated.

## The one real gotcha (design decision worth keeping)
**Do NOT use `--virtual-time-budget` to verify AudioContext state transitions.** First cut used
virtual time + `--dump-dom`; the final `resume()`-after-`suspend()` check **flaked** (passed once,
failed once). Cause: virtual time fast-forwards the JS timer clock, but `AudioContext`
suspend/resume happen on the browser's **real-time audio thread** — the virtual clock races ahead
of them. Fix: run Chrome in **real time** and poll the page **title** over the DevTools endpoint
(`--remote-debugging-port` → `GET /json` lists each tab's live title). Reliable across 4 runs.
Secondary lesson (already known from `wasm_smoke.sh`): SwiftShader headless Chrome hangs on *exit*,
so launch it backgrounded and reap it after the verdict appears — don't wait on it.

## Verification & limits
- **Automated (this session):** full `./scripts/verify.sh` green (fmt / clippy --all-targets /
  wasm lib+bins build / test --all-targets [841 lib + 65 doctests] / rustdoc -D warnings) **after
  the version bump**; `scripts/wasm_audio_smoke.sh` PASS (9/9) ×4; `bash -n` on the script.
- **Remaining human step:** actually *hearing* the 440 Hz tone. No audio capture exists in this
  flow, so acoustic output cannot be autonomously asserted —
  `python3 -m http.server 8080 --directory examples/web_audio/web && open http://localhost:8080`,
  click "Start audio", listen. This is the same human-only residue noted in seq 24.

## Files changed
- `src/audio_wasm.rs` (+`is_running`/`is_music_playing`), `Cargo.toml` (web-sys feature + example
  entry + version), `Cargo.lock`, `docs/CHANGELOG.md`, `CLAUDE.md`.
- New: `examples/web_audio/web_audio.rs`, `examples/web_audio/web/{index.html,build.sh}`,
  `scripts/wasm_audio_smoke.sh`.

## Risks & Blockers
- None blocking. Additive change, no public API break, gate green.
- PR is **open, not merged** — merge authority confirmed per-session; awaiting user go.

## Where to go next (all optional — roadmap already complete)
- Merge this PR (after review), optionally tag `v0.24.0`.
- Deferred from seq 24, still open: P1 per-line dialogue portraits (needs a dialogue image-render
  path); fuller wasm audio (per-source handles, `StereoPannerNode` pan, buses); crates.io publish
  (irreversible, explicit go needed).

## Quick start
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3
grep -m1 '^version' Cargo.toml          # 0.24.0
./scripts/verify.sh                      # green
./scripts/wasm_audio_smoke.sh            # PASS (9/9)  — needs Chrome + matching wasm-bindgen-cli
# hear it:
python3 -m http.server 8080 --directory examples/web_audio/web && open http://localhost:8080
```
