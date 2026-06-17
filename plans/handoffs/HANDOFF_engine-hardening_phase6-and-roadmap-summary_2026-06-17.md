# User-experience roadmap — Phase 6 shipped; Phase 7 deferred (roadmap 6/7 complete)

**Date:** 2026-06-17
**Status:** Phases 1–6 of the UX roadmap **DONE + merged** (v0.11.1 → v0.16.0, all CI-green). **Phase 7
(WASM audio — the explicit "stretch" item) deliberately DEFERRED** — rationale below. `main` @ `0400493`,
clean, CI green.
**Chain:** `engine-hardening` seq `18` · **Parent:** seq 17 (`HANDOFF_engine-hardening_phase5-wasm-save_2026-06-17.md`)
**Prior:** 13 (P1) → 14 (P2) → 15 (P3) → 16 (P4) → 17 (P5) → **18 (P6 + summary)**

## The goal & how it ended

User `/goal`: "drive `plans/USER_EXPERIENCE_PLAN_2026-06-17.md` to its last phase (7); each phase PR →
CI green → merge → handoff." I shipped **6 of 7 phases** this session, all merged on green with standing
merge authority. **Phase 7 is deferred** (not done) — see "Why Phase 7 is deferred".

## Roadmap result (from `plans/USER_EXPERIENCE_PLAN_2026-06-17.md`)

| Phase | Theme | Version | PR | Status |
|---|---|---|---|---|
| 1 | First-hour onboarding (hello_sprite, fork-first README, FORKING.md) | 0.11.1 | #93 | ✅ merged |
| 2 | Game-feel core (TimeScale, Tween<T>, easings, juice_demo) | 0.12.0 | #95 | ✅ merged |
| 3 | Core API ergonomics (query2_mut/query3_mut, push/pop scene) | 0.13.0 | #97 | ✅ merged |
| 4 | Dialogue primitive (DialogueBox + typewriter) | 0.14.0 | #98 | ✅ merged |
| 5 | WASM persistence (localStorage save) | 0.15.0 | #99 | ✅ merged |
| 6 | Particle depth (gravity + emit shape) | 0.16.0 | #100 | ✅ merged |
| 7 | WASM audio (stretch) | — | — | ⏸ **DEFERRED** |

Version went `0.11.0` → **`0.16.0`** across the session (6 MINOR bumps, one per phase). 772 → **786 lib tests**.

## Since Last Handoff (seq 17)

Seq 17 said Phase 6 next. Merged #99 (Phase 5) + shipped Phase 6 as **#100** (v0.16.0). Then assessed
Phase 7 and deferred it (below).

## What We Tried (Phase 6)

`ParticleEmitter` gained **`gravity: Vec2`** and **`emit_shape: EmitShape`** (`Point`/`Circle`/`Ring`/`Box`),
plus builders `with_gravity`/`with_emit_shape`; `EmitShape` re-exported.
- `Particle` gained a per-particle `gravity` (copied at spawn); `ParticleSystem::run` now integrates
  `new_vel = velocity + gravity*dt`, moves by `new_vel`, and **writes the velocity back** to the particle
  (previously particles moved at constant velocity, no writeback).
- Spawn applies `emit_shape.sample_offset(rng)` as a position offset (both continuous + burst paths).
- **Additive with no-op defaults:** `gravity = ZERO` + `emit_shape = Point` reproduce the prior behavior
  byte-identically (unit-tested: `zero_gravity_is_constant_velocity`).
- 3 unit tests (gravity integration, zero-gravity no-op, emit-shape sample bounds). `particles_showcase`
  example (fountain / buoyant fire / box sparks).
- **Construction-site lesson:** most `ParticleEmitter` literals use `..Default::default()` (unaffected by
  new fields); only `Default`, `burst()`, and `config_set.rs` are full literals (updated). **External code
  can't use a `ParticleEmitter { .. }` literal at all** (the `timer` field is `pub(crate)`) — the example
  builds via `Default::default()` + field assignment (with `#[allow(clippy::field_reassign_with_default)]`).

## Why Phase 7 (WASM audio) is deferred — the key decision

- **It's not additive — it's a whole new backend.** The entire `audio` module is `#[cfg(not(target_arch =
  "wasm32"))]` (`src/lib.rs:5`, `src/audio.rs`) and built on **rodio** (native). Wasm audio needs either a
  parallel **web-sys `AudioContext`** implementation (async `decodeAudioData` → buffer source + gain nodes,
  bridged into the engine's synchronous `play()`-returns-a-handle API — awkward) or swapping rodio for a
  cross-platform crate like **`kira`** (large, touches all of `audio/`). Hundreds of lines either way.
- **It is unverifiable right now.** Audio has no meaningful unit test (you can't assert "sound played"), and
  the dev machine is **locked** (no browser + no way to hear it). Phases 4–6 were shippable blind because
  their *logic* was unit-testable; Phase 7's actual function (does a sound play in the browser?) would be
  **100% unverified — only "it compiles."** A compiles-but-silent audio backend is worse than none.
- **It's the explicit "stretch" item** in the plan.
- **Decision:** do Phase 7 in a session where (a) the machine is unlocked so audio can be heard via
  `wasm_smoke`/manual browser test, and (b) a backend choice (web-sys AudioContext vs kira) is made first.
  Blind-shipping it violates this session's CI-green-AND-verified bar.

## Environment note (carried all session)

The dev machine **locked mid-session** (display asleep → lock screen). Live verification was blocked for
Phases 4–6: `dialogue_demo`/`save_counter`/`particles_showcase` were **not eyeballed live**, and the wasm
`localStorage` round-trip wasn't browser-tested. All three rely on unit tests + CI compile/clippy + render/
IO paths proven earlier in the session (Phases 1–3 *were* visually verified — hello_sprite, juice_demo).
**When unlocked, eyeball: `dialogue_demo`, `particles_showcase`; browser-test `save_counter` (localStorage).**

## Evidence & Data

All 8 PRs this session (incl. 2 handoff PRs) merged CI-green: #93, #95, #96(handoff), #97, #98, #99, #100,
plus seq 15/16/17 handoffs bundled into #98/#99/#100. main `0e44a24`→`7f52c92`→…→`0400493`.

## Reusable Gotchas (full set, carry forward)

- **wasm clippy** (`cargo clippy --target wasm32 --lib -D warnings`) is now in `verify.sh` — the wasm
  *build* only warns on wasm-only unused imports; clippy errors. Run full `verify.sh` before pushing.
- **clippy `default_constructed_unit_structs`** (no `::default()` on a fieldless unit struct);
  **`duplicated_attributes`** (don't re-add an `#[allow]` a fn already has); **`field_reassign_with_default`**
  (Default + field sets — needed when a `pub(crate)` field blocks external struct literals; `#[allow]` it).
- **rustdoc `redundant_explicit_links`** (`[`Foo`](path::Foo)` where the label resolves → `[`Foo`]`); bare
  `[`x`]` that doesn't resolve also errors (qualify it).
- **`gh pr checks --watch`**: poll `gh pr checks <n>` until checks register before `--watch` (else instant
  false-0); after two quick pushes, watch the *latest* run; don't append `; echo $?` (masks task exit).
- **`cargo fmt` before the gate** (reformats fresh asserts/ternaries). **External crates can't use a struct
  literal if any field is `pub(crate)`** (E0451) — use `Default` + field assignment.
- **Pre-1.0 (0.x):** feature/breaking → MINOR, fix/docs → PATCH; never 1.0.0 / 10.x. Standing merge-on-green
  per `/goal`; run `gh pr merge` standalone (auto-mode classifier once gated a bundled merge command).
- **rust-analyzer phantoms** (ColliderHandle E0308, unlinked-file, inactive-code) are stale — trust cargo/CI.

## Where We're Going

1. **Phase 7 — WASM audio (stretch, the only open phase):** pick a backend (web-sys `AudioContext` for a
   minimal one-shot SFX path, or `kira` for a full cross-platform replacement); un-gate `audio` for wasm;
   bridge async decode into the sync API; verify by **hearing it** in a browser (needs unlocked machine).
   Scope can start at one-shot SFX only.
2. **Deferred follow-ups surfaced this session:** `GpuParticleEmitter` mirror of gravity/emit_shape + RON
   `ParticleConfigSet` support for them; AEAD `save`/`load` on wasm (currently `Unsupported`); localization
   keys in `DialogueBox`; **live eyeball of dialogue_demo/particles_showcase + browser-test save_counter**
   once unlocked.
3. **crates.io publish** — still deferred (fork-first; the `engine_reflect_derive` path-dep is already a
   dev-dependency so publish is mechanically unblocked — see `engine-current-state` memory).

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 0400493 Phase 6 (#100)
grep -m1 '^version' Cargo.toml  # 0.16.0
./scripts/verify.sh             # green (786 lib tests); RUN AS-IS, no tail pipe
# Read: plans/USER_EXPERIENCE_PLAN_2026-06-17.md, this handoff (seq 18).
# Roadmap 6/7 done. ONLY Phase 7 (WASM audio, stretch) remains — needs a backend choice + an
#   UNLOCKED machine to verify by ear. Don't blind-ship it.
# Also (when unlocked): eyeball dialogue_demo + particles_showcase; browser-test save_counter.
```
