# P3 shipped — wasm parity (AEAD save/load + WebAudio depth) → ROADMAP COMPLETE

**Date:** 2026-06-18
**Status:** **COMPLETED — both axes shipped + merged, CI-green.** `main` @ `0622cff`, package **v0.23.0**, clean tree.
PRs **#109** (3A, v0.22.0) + **#110** (3B, v0.23.0) squash-merged + branches deleted.
**Chain:** `engine-hardening` seq `24` · **Parent:** `HANDOFF_engine-hardening_p2-particle-ron-gpu_2026-06-18.md` (seq 23)
**Roadmap:** P3 of `PLAN_engine-hardening_p1p2p3-roadmap_2026-06-18.md` — **P1✅ P2✅ P3✅ → the whole P1/P2/P3 roadmap is DONE.**
**Goal directive:** "ship roadmap P1→P2→P3, each test+merge+handoff, stop after P3." → **MET.** The `/goal` Stop condition is now satisfied.

---

## What shipped

### 3A — wasm AEAD save/load parity (v0.22.0, PR #109)
`save` / `load` / `save_versioned` / `load_migrated` previously returned `SaveError::Unsupported`
on wasm; only `write_ron`/`read_ron`/`exists`/`delete` (plaintext localStorage) worked. Now the
ChaCha20-Poly1305 AEAD core (`src/save.rs`) is **cross-platform**:
- The only per-target difference is the storage backend — a file on native, a **hex-encoded** blob
  in `localStorage` (keyed by the path string) on wasm. `save_with_key`/`load_with_key` branch on
  storage; the encrypt/decrypt/envelope/migration logic is shared.
- Nonce RNG `rand::thread_rng()` → `rand::rngs::OsRng` (works on wasm via `getrandom`'s `js`
  backend; `thread_rng` isn't wired there).
- `SaveError::Unsupported` re-scoped to "storage unavailable / future save version".
- **Resolves the OPEN QUESTION** (author had deliberately left wasm AEAD `Unsupported` because
  browser localStorage is inspectable): the gap is closed with the caveat made **explicit** —
  `localStorage` is user-inspectable, so the binary-embedded key is tamper-detection + obfuscation,
  not secrecy, the **same trust model as the native file** (documented on `save_with_key`).
- Example `save_encrypted` (NEW): encrypted launch counter, file natively / hex localStorage on
  web. **Playtested windowed** — count persists 1→2 across runs; the saved file is `R2DAEAD01`
  magic + ciphertext (not plaintext). 19 native save tests pass unchanged.

### 3B — WebAudio depth (v0.23.0, PR #110)
`WebAudio` (`src/audio_wasm.rs`) grew from one-shot SFX (`new`/`play`) into a small mixer:
- `set_volume`/`volume` — a master `GainNode` all playback routes through.
- `play_music`/`stop_music` — a single looping music channel (stops current, starts new looping).
- `suspend`/`resume` — pause/resume all audio (also the browser user-gesture unlock).
- Added web-sys features `GainNode`, `AudioParam`. Native rodio `AudioManager` unchanged;
  per-source mixing/crossfade/buses/ducking/positional stay native-only.

## Gotchas hit (both fixed)
1. **3A — `gpu`-style cfg trap avoided:** `GpuParticleEmitter` lesson from P2 reapplied; here the
   AEAD deps (`chacha20poly1305`, `rand`) were already *unconditional* `[dependencies]`, so making
   the code cross-platform "just worked" once `thread_rng`→`OsRng` and hex+localStorage were wired.
2. **3B — web-sys deprecates `AudioBufferSourceNode::stop*`** (no non-deprecated stop binding is
   exposed; `start()` is fine). Used `#[allow(deprecated)]` scoped to `stop_music` — the call is
   correct, the deprecation is a blanket web-sys binding flag.

## Verification & limits
- **3A:** native playtest (count persists, file encrypted) + 19 native tests + full verify
  (incl. wasm build + wasm clippy — the AEAD path compiles for wasm32). Browser run-time of the
  localStorage path is **deferred** (compile-gated; mirrors the v0.15.0 `write_ron` localStorage
  path which *was* browser-verified).
- **3B:** **No autonomous runtime verification is possible** — there's no audio capture in this
  flow. Verified by: wasm build + wasm clippy green (the API compiles for wasm32) + a compile-`ignore`
  doc example. Hearing the music/volume is a browser/human step, exactly as the v0.17.0 WebAudio
  one-shot was originally checked (throwaway-branch browser test). **This is the one open
  verification debt for P3.**

## CI
- PR #109: Test (native) 7m2s · WASM 44s · Rustdoc 34s · Package 1m9s — all green.
- PR #110: Test (native) 4m52s · WASM 1m39s · Rustdoc 51s · Package 1m0s — all green.

## Files changed
- 3A: `src/save.rs` (cross-platform AEAD + hex helpers), `examples/save_encrypted.rs` (new),
  release paperwork (0.22.0).
- 3B: `src/audio_wasm.rs` (WebAudio rewrite), `Cargo.toml` (web-sys GainNode/AudioParam),
  release paperwork (0.23.0).

## Risks & Blockers
- None blocking. Tree clean, main green, roadmap complete.
- **One verification debt:** 3B WebAudio runtime (music/volume) needs a browser + ears. Quick to
  do in a display session: build a wasm page that calls `play_music` on a user gesture and listen.

## Roadmap retrospective (P1→P3, this session)
| Stage | Release | PR | What |
|---|---|---|---|
| P1 | 0.20.0 | #107 | Data-driven dialogue — RON `DialogueTree`/`load_dialogue` + conditional choices (`DialogueVars`/`cond`) + event/effect hooks (`dialogue::choose`→`Events`), example `dialogue_quest`. (1d per-line portrait deferred — renderer is text-only.) |
| P2 | 0.21.0 | #108 | `ParticleConfigSet::gpu_emitter` — RON→`GpuParticleEmitter` (native-only), example `gpu_particles` loads RON. |
| P3 | 0.22.0 / 0.23.0 | #109 / #110 | wasm parity — AEAD save/load (3A) + WebAudio depth (3B). |

## Where to go next (none required — goal met)
- **Verification debt:** browser-verify 3B audio (above).
- **Deferred features:** P1's per-line portraits (needs a dialogue image-render path); fuller wasm
  audio (per-source handles, pan via `StereoPannerNode`, buses) — `kira` not recommended (large swap).
- **crates.io publish** (still deferred; irreversible; needs explicit go) + optional tags
  `v0.20.0`–`v0.23.0`.

## Quick start
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5            # 0622cff (#110) … 7750da0 (#108) … c568f70 (#107)
grep -m1 '^version' Cargo.toml  # 0.23.0
./scripts/verify.sh             # green
cargo run --example save_encrypted   # encrypted save persists across runs
cargo run --example dialogue_quest   # P1; cargo run --example gpu_particles  # P2
```
