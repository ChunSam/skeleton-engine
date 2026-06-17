# User-experience roadmap — COMPLETE (7/7, v0.11.0 → v0.17.0)

**Date:** 2026-06-17
**Status:** **All 7 phases of the UX roadmap shipped + merged, CI-green.** `main` @ `6fb2dc6`, package
**v0.17.0**, clean. The `/goal` completion condition (reach the last phase) is **met**.
**Chain:** `engine-hardening` seq `19` · **Parent:** seq 18 (`HANDOFF_engine-hardening_phase6-and-roadmap-summary_2026-06-17.md`)
**Prior:** 13 (P1) → 14 (P2) → 15 (P3) → 16 (P4) → 17 (P5) → 18 (P6 + Phase-7 deferral) → **19 (P7 done; roadmap complete)**

> **✅ RESOLUTION (post-handoff, 2026-06-17 — machine unlocked):** The "Outstanding verification" debt
> below is **CLEARED**. After this handoff was written the machine unlocked and all four unobserved demos
> were verified live: Phase 4 `dialogue_demo` (typewriter + advance), Phase 5 `save_counter` (native file
> 1→2 **and** browser localStorage reload 1→7 persists), Phase 6 `particles_showcase` (gravity arcs +
> emit shapes), and Phase 7 **`WebAudio` beep HEARD in a browser**. Browser checks ran on a throwaway
> `tmp/browser-test` branch (temp `#[wasm_bindgen]` exports + a generated-WAV beep + a dist/ HTML page),
> fully discarded afterward — `main` untouched. This note was added late because the verification session
> shipped no handoff of its own; the canonical record is the `engine-current-state` memory. The ⚠️
> section below is kept verbatim as the as-written record.

## Outcome

`/goal` = drive `plans/USER_EXPERIENCE_PLAN_2026-06-17.md` to its last phase, each phase PR → CI green →
merge → handoff. **Done — 7/7.** (Seq 18 had deferred Phase 7; the stop-hook flagged the goal as
incomplete, so Phase 7 was implemented this turn as a bounded, compile-verified wasm SFX path — see the
verification caveat.)

| Phase | Theme | Version | PR |
|---|---|---|---|
| 1 | First-hour onboarding (hello_sprite, fork-first README, FORKING.md) | 0.11.1 | #93 |
| 2 | Game-feel core (TimeScale, Tween<T>, easings, juice_demo) | 0.12.0 | #95 |
| 3 | Core API ergonomics (query2_mut/query3_mut, push/pop scene) | 0.13.0 | #97 |
| 4 | Dialogue (DialogueBox + typewriter) | 0.14.0 | #98 |
| 5 | WASM persistence (localStorage save) | 0.15.0 | #99 |
| 6 | Particle depth (gravity + emit shape) | 0.16.0 | #100 |
| 7 | WASM audio (one-shot SFX) | 0.17.0 | #102 |

`v0.11.0` → **`v0.17.0`** (7 MINOR bumps). **772 → 786 lib tests.** Handoffs seq 13–19. All merges CI-green.

## What We Tried (Phase 7)

The native `AudioManager` is rodio-based + `cfg(not(wasm32))`; a full wasm backend (or kira swap) is
large and unhearable while locked, so Phase 7 was scoped (per the plan) to **one-shot SFX**, kept
self-contained, and the native module left untouched:
- **`src/audio_wasm.rs` (wasm-only): `WebAudio`** — `new()` (creates `web_sys::AudioContext`),
  `play(bytes)` (Uint8Array → `decode_audio_data` Promise → `JsFuture` await in `spawn_local` →
  `create_buffer_source` → connect to `destination()` → `start()`). Fire-and-forget; `Clone`.
- `src/lib.rs`: `#[cfg(wasm32)] pub mod audio_wasm;` + `pub use audio_wasm::WebAudio;`.
- `Cargo.toml`: added web-sys features `AudioContext`, `BaseAudioContext`, `AudioBuffer`,
  `AudioBufferSourceNode`, `AudioNode`, `AudioDestinationNode`.
- Verified via **`cargo build`/`clippy --target wasm32`** (validates the Web Audio API usage + features).

## ⚠️ Outstanding verification (the locked-machine debt) — ✅ RESOLVED (see banner at top)

The dev machine **locked mid-session** (display asleep → lock screen), blocking all live/visual/audio
verification from Phase 4 onward. Compile + unit-tests + CI are green throughout. The items below were
**not observed live at the time of writing** but were **all verified live after unlock** (2026-06-17 — see
the RESOLUTION banner at the top); kept verbatim as the as-written record:
- **Phase 4 `dialogue_demo`** — eyeball the typewriter + advance.
- **Phase 5 `save_counter`** — browser-test the `localStorage` round-trip (run the wasm build, reload, see
  the count persist).
- **Phase 6 `particles_showcase`** — eyeball gravity arcs + emit shapes.
- **Phase 7 `WebAudio`** — **actually hear a sound in a browser** (the standard Web Audio path compiles,
  but sound output is unverified). This is the most important one.
- (Phases 1–3 *were* visually verified — hello_sprite + juice_demo screenshots.)

## Key Decisions

- **Phase 7 implemented (not left deferred)** — after the stop-hook flagged the goal incomplete, and for
  consistency with Phase 5 (also unverifiable-while-locked but shipped), a *minimal* SFX path matching the
  plan's own "one-shot SFX" scoping was the right bounded deliverable. Transparently flagged as
  sound-output-unverified rather than claimed working.
- **Did not touch the native `AudioManager`** — `WebAudio` is a separate, additive wasm-only module, so
  native audio is byte-identical and the change is low-risk.
- **No git tags created** — tagging is an explicit outward action; not requested. `v0.11.1`–`v0.17.0`
  exist as CHANGELOG entries + commits only. (The old `v10.x` tags still sort highest — cosmetic.)

## Follow-ups (open, none blocking)

- ~~**Hear/eyeball the 4 demos above** (locked-machine debt)~~ — ✅ **DONE** (all 4 verified live post-unlock; see top banner).
- `GpuParticleEmitter` mirror of gravity/emit_shape + RON `ParticleConfigSet` support for them. ← **now the top open follow-up.**
- AEAD `save`/`load` on wasm (currently `Unsupported`); `DialogueBox` localization keys.
- Fuller wasm audio (music/mixing/positional) — e.g. `kira` cross-platform unification.
- **crates.io publish** — still deferred (fork-first). Mechanically unblocked (`engine_reflect_derive` is a
  dev-dependency). See `engine-current-state` memory.
- Optionally tag `v0.11.1`–`v0.17.0`.

## Reusable Gotchas (full set, carry forward)

- **wasm-only code isn't checked by native `cargo check`/`test`** — `verify.sh` now runs
  `cargo clippy --target wasm32 --lib -D warnings` (added Phase 3) which, with the wasm build, validates
  wasm-only modules (`save` localStorage, `WebAudio`). Run full `verify.sh` before pushing wasm code.
- **External crates can't use a struct literal with any `pub(crate)` field** (E0451) — `ParticleEmitter`
  has a private `timer`, so examples build it via `Default::default()` + field assignment
  (`#[allow(clippy::field_reassign_with_default)]`).
- clippy: **`default_constructed_unit_structs`** (no `::default()` on fieldless unit struct),
  **`duplicated_attributes`** (don't re-add an `#[allow]` a fn already has), **`field_reassign_with_default`**.
- rustdoc **`redundant_explicit_links`** — `[`Foo`](path::Foo)` where the label resolves → `[`Foo`]`.
- **`gh pr checks --watch`**: poll until checks register before `--watch` (else instant false-0); watch the
  latest run after rapid pushes; don't append `; echo $?` (masks task exit — capture in the log).
- **`cargo fmt` before the gate** (reformats fresh asserts/ternaries). Pre-1.0: feature/breaking → MINOR,
  fix/docs → PATCH; never 1.0.0 / 10.x. Standing merge-on-green per `/goal`; run `gh pr merge` standalone.
- rust-analyzer phantoms (ColliderHandle E0308 / unlinked-file / inactive-code) are stale — trust cargo/CI.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 6fb2dc6 Phase 7 (#102)
grep -m1 '^version' Cargo.toml  # 0.17.0
./scripts/verify.sh             # green (786 lib tests); RUN AS-IS, no tail pipe
# Read: plans/USER_EXPERIENCE_PLAN_2026-06-17.md (all 7 done), this handoff (seq 19).
# The roadmap is COMPLETE and ALL DEMOS ARE LIVE-VERIFIED (post-unlock — see the RESOLUTION banner at top;
#   verification debt is CLEARED). Highest-value next step is now a Follow-up: GpuParticleEmitter
#   gravity/emit_shape mirror + RON ParticleConfigSet support. Or: wasm AEAD save, dialogue localization,
#   crates.io publish, tag v0.11.1–v0.17.0.
```
