# VISION features batch — TweenSequence → nine-slice (v10.3.0 → v10.7.0)

**Date:** 2026-06-17
**Chain:** `engine-hardening` seq 11 (continues `HANDOFF_engine-hardening_vision-features-render-split_2026-06-17.md`, seq 10)
**Baseline:** `main` @ `5410e74`, v10.2.1, Gate6-green, 732 lib tests (verified this session).
**Authority:** merge authority granted for THIS session ("전체 작업 ci 끝나면 머지하고 종료").

## Goal

Implement, in difficulty order, the 5 additive VISION features the prior session left as
candidates. Each is one CI-green, squash-merged PR with a playable/audible example
(the VISION acceptance bar) and a minor version bump.

## Execution model

- **5 separate minor-bump PRs** (keeps the engine's one-PR-per-feature convention + per-feature CI).
- **Parallel implementation:** the agent-safe features (1–4) are implemented concurrently by
  background Sonnet subagents in **isolated git worktrees**, each on its own `feat/*` branch.
  Each agent touches ONLY its own source/example/test files **and** `src/lib.rs` (pub mod +
  re-export). Agents do **NOT** bump `Cargo.toml`/`Cargo.lock`/`docs/CHANGELOG.md`/`CLAUDE.md`
  — those are integration-owned (opus) to keep the version sequence conflict-free.
- **Render-path feature (5, nine-slice) is done by opus by hand** — the render path has no
  CI/GPU test (handoff mandates manual care + screencapture playtest).
- **Serial integration & merge (opus):** process branches in difficulty order — rebase onto
  latest main, add version + CHANGELOG + CLAUDE bump, run full Gate6, run the visual/audio
  playtest, open PR, watch CI, squash-merge. Each next feature rebases on the freshly merged main.

## Verification gates

- **Merge gate = CI-green** (`./scripts/verify.sh` locally + GitHub CI). Every PR must pass
  fmt / clippy --all-targets / wasm lib+bins build / test --all-targets / rustdoc -D warnings.
- **Visual confirmation = best-effort.** Features needing on-screen confirmation get a macOS
  screencapture playtest. **If the capture is black (monitor off/asleep), the merge still
  proceeds on CI-green + code review, and a TODO is filed below to re-run the visual check
  when the user re-opens the monitor.** (Per user instruction.)
- Every feature has a deterministic, unit-testable core so CI alone is a strong gate.

## The 5 features (difficulty order)

| # | Feature | Version | New public API (sketch) | Files | Verify | Owner |
|---|---------|---------|--------------------------|-------|--------|-------|
| 1 | TweenSequence (chained tweens) | 10.3.0 | `TweenSequence` (chain of `Tween` segments, per-segment easing, `then`, loop/pingpong) | `src/tween.rs` (or `src/tween/`), `examples/tween_sequence.rs` | unit tests + visual | Sonnet agent |
| 2 | Music-track crossfade | 10.4.0 | `AudioManager::crossfade(channel, new_path, dur)` (fade-out old + fade-in new, reusing `Fade` infra) | `src/audio/playback.rs`, `src/audio.rs`, `examples/music_crossfade.rs` | unit tests + (audio = listen → TODO) | Sonnet agent |
| 3 | Coroutine / timed-action sequencer | 10.5.0 | `Coroutine` (steps: `Wait` / `Run(FnMut(&mut World))` / `RunFor{dur,f}`), `CoroutineRunner` resource, `CoroutineSystem` | `src/coroutine.rs`, `examples/coroutine_demo.rs` | unit tests + visual | Sonnet agent |
| 4 | Animated tiles | 10.6.0 | `AnimatedTile`/`TileAnimation` (per-tile-id frame seq + fps); `TilemapSystem` cycles animated cells' `UvRect` each frame (bypass generation cache for animated cells) | `src/tilemap/`, `examples/animated_tiles.rs` | unit tests + visual | Sonnet agent |
| 5 | Nine-slice / 9-patch | 10.7.0 | `NineSlice`/`NinePatch` component (border insets) → renderer emits 9 sub-quads | `src/renderer/sprite/geometry.rs`, component, `examples/nine_slice.rs` | unit tests + visual (REQUIRED) | opus by hand |

## Pending visual verification (re-run when monitor is back on)

> Filled in during the run if a screencapture comes back black (monitor off).

- (none yet)

## Status log — COMPLETE ✅ (main @ v10.7.0)

- [x] 1 TweenSequence (10.3.0) — PR #83 merged
- [x] 2 Music crossfade (10.4.0) — PR #84 merged
- [x] 3 Coroutine (10.5.0) — PR #85 merged; doc hotfix PR #86
- [x] 4 Animated tiles (10.6.0) — PR #87 merged
- [x] 5 Nine-slice (10.7.0) — PR #88 merged

## Outcomes

- All 5 VISION features shipped as 5 CI-green squash-merged PRs (#83/#84/#85/#87/#88) + 1
  hotfix (#86). `main` @ `457e4fb`, v10.7.0, clean. Each has a playable/audible example + unit
  tests; nine-slice/coroutine/animated-tiles/tween visually playtested (screencaptures), audio
  verified by CI-safe scheduling tests + a clean local run.
- Implementation parallelized: features 1–4 built concurrently by background Sonnet agents in
  isolated git worktrees; nine-slice (render path) hand-built by opus. Integration + merges
  serial (version-bump ordering).
- New public API: `engine::{TweenSequence, Coroutine, CoroutineRunner, CoroutineSystem,
  TileAnimation, TileAnimationSet, AnimatedTileCell, AnimatedTileSystem, NineSlice}` +
  `AudioManager::crossfade`.
- No crates.io publish (unchanged from seq 10; still needs explicit go). No git tags pushed for
  10.3.0–10.7.0 this run (left for an explicit release step like seq 10's).

## Lessons (carry forward)

- **Agent Gate6 must include the doc step.** The coroutine agent ran fmt/clippy/test/build but
  NOT `RUSTDOCFLAGS="-D warnings" cargo doc`, so a `private_intra_doc_links` (module doc linked the
  private `CoroutineStep`) slipped through and turned main's Rustdoc CI red after #85. Fixed by
  hotfix #86. Always have agents run the full Gate6 incl. doc, OR run `cargo doc -D warnings` at
  integration before pushing.
- **Don't mask exit codes in gate pipes** (the [[ci-toolchain-pin]] pipefail gotcha). `verify.sh
  | tail` and `gh pr checks --watch | tail` make the pipeline exit code = `tail`'s (0), hiding a
  real failure — that's how the red #85 got merged. Run `verify.sh > log 2>&1` and `gh pr checks
  --watch --fail-fast > log 2>&1` with NO trailing pipe so the background task's own exit code is
  authoritative; still double-check `gh pr checks <n>` before merging.
- **`cargo test --all-targets` skips doctests; CI runs `cargo test --doc` separately.** A doc
  example that doesn't compile (e.g. wrong `App::new(...)` arity, non-existent `world_mut()`) fails
  CI even though `--all-targets` is green. nine-slice's module-doc example had both bugs + a bare
  `[Sprite]` intra-doc link; all caught at integration before pushing (no red main).
- **Gate6 does not leave runnable example binaries** (clippy/test only check). Always
  `cargo build --example <name>` before a screencapture playtest, else you capture a missing-file
  no-op (false "CLEAN" log).

## Standing constraints (from memory + handoff)

- Korean prose to user / English code + docs + handoff.
- Sonnet subagents with explicit `model: sonnet` (claude-fable-5 dies as a subagent).
- No new dependencies; no breaking changes.
- Version-bump policy: new public feature + example → minor. Each PR bumps Cargo.toml +
  Cargo.lock + docs/CHANGELOG.md (new `##` at top) + CLAUDE.md header (+ module-map row).
- Render path: no CI/GPU test — visual playtest is the only safety net for `render.rs`/geometry.
