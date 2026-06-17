# User-experience roadmap — Phase 5 shipped (WASM persistence)

**Date:** 2026-06-17
**Status:** Phase 5 MERGED (PR #99, v0.15.0, CI-green). `main` @ `a035571`, clean. Driving the roadmap
per `/goal` (each phase: PR → CI green → merge → handoff). **Phase 6 (particle depth) next; then 7 (WASM audio, stretch) = last.**
**Chain:** `engine-hardening` seq `17` · **Parent:** seq 16 (`HANDOFF_engine-hardening_phase4-dialogue_2026-06-17.md`)
**Prior:** 13 (P1 v0.11.1) → 14 (P2 v0.12.0) → 15 (P3 v0.13.0) → 16 (P4 v0.14.0) → **17 (P5 v0.15.0)**

## The standing goal & environment

`/goal`: drive `plans/USER_EXPERIENCE_PLAN_2026-06-17.md` to its last phase (7); per phase implement →
PR → **CI green → merge** (standing merge authority on green) → handoff (bundled into the next phase's PR).
**⚠️ The dev machine is LOCKED** (display asleep) → live screencapture + browser wasm-smoke playtests are
blocked. Phases 5–7 rely on unit tests + CI compile/clippy; live verification deferred until unlocked.

## Since Last Handoff (seq 16)

Seq 16 said Phase 5 (WASM save) next. Merged #98 (Phase 4) + shipped Phase 5 as **#99** (v0.15.0).

## Where We Are

- **`main` @ `a035571`**, package **v0.15.0**, CLAUDE.md header **v1.6.63**, clean, only `main`.
- **783 lib tests** (Phase 5 added no native lib tests — the new code is wasm-only). Full `verify.sh` green.
- Roadmap: **Phases 1–5 DONE + merged**; Phase 6 next; Phase 7 (stretch) last.

## What We Tried (Phase 5)

1. **`src/save.rs` wasm branches → `localStorage`.** Added a `#[cfg(target_arch = "wasm32")] mod
   wasm_storage` (web-sys `Storage`): `get`/`set`/`remove` via `window().local_storage()`. Routed the
   wasm branches of `write_ron` (serialize → `set`), `read_ron` (`get` → `ron::from_str`; absent →
   `Io(NotFound)`), `exists` (`get`.is_some()), `delete` (`remove`) through it. Keyed by
   `path.to_string_lossy()`. Native path untouched.
2. **`Cargo.toml`** — added `"Storage"` to the wasm `web-sys` features.
3. **Kept AEAD `save`/`load`/`save_versioned` `Unsupported` on wasm** — documented (hardcoded-key crypto
   in an inspectable browser store adds little; binary ciphertext would need base64 in a string store).
4. **`examples/save_counter.rs`** — cross-platform launch counter via `write_ron`/`read_ron` (file on
   native, localStorage on web). Compile-verified; not run live (locked).
5. Docs: CHANGELOG `## 0.15.0`, CLAUDE.md save row.
6. Verified the wasm code compiles + lints (`cargo build`/`clippy --target wasm32 --lib -D warnings`),
   then full `verify.sh` green.

## Key Decisions

- **Plaintext family only on wasm** (`write_ron`/`read_ron`/`exists`/`delete`); AEAD stays native-only.
  Covers the common browser need (settings/score) cleanly without un-gating crypto or base64 encoding.
- **`read_ron` absent key → `Io(NotFound)`** so `unwrap_or`/`load_or_default` patterns are identical
  cross-platform.
- **No blind edit of `coin_race`** — added a small self-contained `save_counter` example instead (lower
  risk than threading persistence through the networked game I can't playtest).

## Evidence & Data

| PR | main after | Title | CI |
|---|---|---|---|
| #98 | `c184987` | Phase 4 dialogue (v0.14.0) | merged |
| #99 | `a035571` | Phase 5 WASM save (v0.15.0) | 4/4 |

Files (#99): `src/save.rs` (wasm branches + `wasm_storage`), `Cargo.toml` (web-sys `Storage` + version),
`Cargo.lock`, `examples/save_counter.rs` (new), `docs/CHANGELOG.md`, `CLAUDE.md`. (seq-16 handoff rode along.)

## Code Analysis (Phase 6 anchors — particle depth)

- **`src/particle/mod.rs`**: `ParticleEmitter` (struct ~L40) fields `spawn_rate`, `lifetime`, `velocity`,
  `velocity_spread`, `color_start/end`, `size`, `texture` (+ a `Default` impl ~L70). `Particle` (struct
  ~L118) fields incl. `age`, `lifetime`, `velocity`, `color_start/end`. `ParticleSystem::run` (~L152):
  ages particles, `tr.position += velocity * dt`, lerps color by `age/lifetime`, spawns new particles
  with `actual_velocity = velocity ± spread` (radial speed path ~L289 uses `velocity_spread` magnitude).
- **Phase 6 plan:** add `gravity: Vec2`, `angular_velocity: f32`, `angular_spread: f32`, `emit_shape`
  enum (Point/Circle/Ring/Box) to `ParticleEmitter`; store per-particle gravity + angular_velocity on
  `Particle`; in `run`, `velocity += gravity*dt` then `position += velocity*dt`, and `tr.rotation +=
  angular_velocity*dt`; offset spawn position by an `emit_shape` sample. Update the `Default` impl + ALL
  in-repo `ParticleEmitter { .. }` construction sites (new fields break struct literals without
  `..Default::default()` — grep them). Mirror onto `GpuParticleEmitter` only if low-risk (separate
  native GPU path) — else defer with a note. **Unit-test the math** (gravity accumulation, shape-sample
  bounds, angular advance — these are native-testable, unlike the visuals). Example `particles_showcase`
  (compile-verified; visual deferred while locked). Also `ParticleConfigSet` RON (config_set.rs) may need
  the new fields if it mirrors the emitter — check.

## Where We're Going

- **Phase 6 — particle depth (0.16.0):** see anchors above.
- **Phase 7 — WASM audio (stretch, last):** wasm SFX path in `audio` (currently `cfg(not(wasm32))`);
  scope to one-shot SFX; browser verification deferred (locked).

## Risks & Blockers

- **Locked machine** → no live particle/audio verification for 6/7. Lean on unit tests + CI.
- **`ParticleEmitter` field additions** can break struct-literal construction sites + `ParticleConfigSet`
  RON parsing — find and update all (or guard with `#[serde(default)]` on the config struct).
- **main PR-only**; standing merge-on-green per `/goal`; run `gh pr merge` standalone (classifier).

## Reusable Gotchas (carry forward)

- **wasm-only code isn't checked by native `cargo check`/`test`** — use `cargo build/clippy --target
  wasm32` (now in `verify.sh`) for the wasm save/audio paths.
- **clippy `default_constructed_unit_structs`** (no `::default()` on a fieldless unit struct);
  **rustdoc `redundant_explicit_links`** (`[`Foo`](path::Foo)` where label resolves → use `[`Foo`]`).
- **`gh pr checks --watch`**: poll until checks register (else instant false-0); watch the latest run.
- **`cargo fmt` before the gate.** Pre-1.0: feature→MINOR, fix/docs→PATCH; never 1.0.0 / 10.x.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # a035571 Phase 5 (#99)
grep -m1 '^version' Cargo.toml  # 0.15.0
./scripts/verify.sh             # green; RUN AS-IS, no tail pipe
# Read: plans/USER_EXPERIENCE_PLAN_2026-06-17.md (Phase 6 next), this handoff (seq 17).
# Goal: phases 6→7, each PR → CI green → merge → handoff. Standing merge authority on green.
# NEXT: Phase 6 — ParticleEmitter gravity/angular/emit-shape (+ particles_showcase) → 0.16.0.
#   Update ALL ParticleEmitter construction sites + ParticleConfigSet RON; unit-test the math; GPU mirror optional.
```
