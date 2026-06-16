# VISION feature batch — 5 features shipped (v10.3.0 → v10.7.0) + main branch protection

**Date:** 2026-06-17
**Status:** COMPLETED — 5 features shipped as 5 CI-green squash-merged PRs + 1 doc hotfix; `main` @ `3f43ab3`, v10.7.0, clean, CI-green, 772 lib tests. `main` branch protection now enforced (CI-gated merges). Goal `/goal` satisfied.
**Bead(s):** none (beads unavailable in this repo — `bd` not installed)
**Epic:** skeleton-engine — engine hardening / architecture
**Chain:** `engine-hardening` seq `11`
**Parent:** `HANDOFF_engine-hardening_vision-features-render-split_2026-06-17.md` (seq 10)
**Prior chain:** seq 1 (v9.0.0) → … → 9 (v10 breaking pass) → 10 (shader_material/parallax/render-split, v10.2.1) → **this (11)**

---

## Since Last Handoff

Seq 10 ended with the "clean additive backlog exhausted" note and left, as options for a fresh session: (1) decide crates.io publish, (2) backfill v5–v9 git tags, (3) **pick the next VISION feature** from five 0-hit candidates it listed (nine-slice / TweenSequence / coroutine / animated-tiles / music-crossfade). This session did **all five** of those candidates in one batch.

- **All 5 VISION candidates shipped** (not just one) — the user `/goal`'d "1번부터 5번까지 진행" after I listed them difficulty-ordered.
- **Merge authority was re-granted** for this run via the `/goal` (seq 10 flagged it as loop-scoped; it was re-confirmed, scoped again to this run only).
- **crates.io: still NOT published; v10.3.0–v10.7.0 NOT tagged** — unchanged from seq 10's deferral (left for an explicit release step).
- **New since seq 10:** main now has **branch protection** (required status checks, enforce_admins=true) — a *mechanical* guard added at the user's request after a process failure this session (see below). Seq 10 had no branch protection, which is how a red PR could be merged.
- Trajectory: still on the VISION feature→example loop. One real process incident (a red PR merged) was caught, hotfixed, and structurally prevented going forward.

## Reference Documents

- `CLAUDE.md` — agent quick reference (bumped to v1.6.57 / package v10.7.0 this session; +1 module-map row each for Coroutine, NineSlice; tween/audio/tilemap rows extended)
- `docs/VISION.md` — the forkable-skeleton vision + "a feature isn't done until a playable example exercises it" bar (drove all 5)
- `plans/VISION_FEATURES_BATCH_2026-06-17.md` — THIS session's saved plan (difficulty order, execution model, outcomes, lessons). Committed to main in the session commit.
- `docs/CHANGELOG.md` — new `## 10.3.0`–`## 10.7.0` sections

## The Goal

Keep extending the `skeleton-engine` (forkable, MIT, genre-agnostic 2D wgpu engine) along the VISION loop without regressions. Concrete `/goal` for this session: save a plan, then implement candidates 1→5 in difficulty order, each as a CI-green squash-merged PR with a playable/audible example (the acceptance bar) and a minor version bump; parallelize implementation with subagents; visual tests via screencapture (leave a TODO if the monitor is off → black frame); merge when all CI is green, then end. (Monitor stayed on the whole run, so no visual-TODO fallbacks were needed.)

## Where We Are

- **`main` @ `3f43ab3`**, package **v10.7.0** (was v10.2.1 at session start), clean working tree.
- **772 lib tests** pass (was 732; +40 = tween 9 + audio 3 + coroutine 11 + animated-tiles 11 + nine-slice 6). Doctests 63 pass / 33 ignored. Full Gate6 green (verified locally on main, authoritative exit 0).
- **5 features shipped + squash-merged**, all branches deleted, agent worktrees removed:
  - **#83** v10.3.0 — `TweenSequence` (chained tweens) — `src/tween.rs` + `examples/tween_sequence.rs`
  - **#84** v10.4.0 — `AudioManager::crossfade` (track-to-track) — `src/audio/playback.rs` + `examples/music_crossfade.rs` (native-only)
  - **#85** v10.5.0 — `Coroutine`/`CoroutineRunner`/`CoroutineSystem` — `src/coroutine.rs` + `examples/coroutine_demo.rs`
  - **#87** v10.6.0 — animated tiles — `src/tilemap/animation.rs` + `system.rs` integration + `examples/animated_tiles.rs`
  - **#88** v10.7.0 — `NineSlice` 9-patch — `src/nine_slice.rs` + 9-quad branch in `src/renderer/sprite.rs` + `examples/nine_slice.rs`
- **#86** v(no bump) — doc hotfix for #85's coroutine module doc (private-intra-doc-link). Restored main to green.
- **New public API** at crate root: `engine::{TweenSequence, Coroutine, CoroutineRunner, CoroutineSystem, TileAnimation, TileAnimationSet, AnimatedTileCell, AnimatedTileSystem, NineSlice}` + `AudioManager::crossfade`.
- **Visual playtests PASSED** (screencapture) for tween, coroutine, animated-tiles (2 frames showing cycling), nine-slice (corners fixed + rotation rigid + naive-stretch contrast). Audio verified via CI-safe scheduling tests + a clean local run (sound not screencapturable).
- **`main` branch protection ENABLED** (NEW): `required_status_checks` = `Build (WASM)`, `Test (native)`, `Rustdoc`, `Package dry-run`; `strict=true`; `enforce_admins=true`; no required reviews. Red CI ⇒ merge blocked for everyone incl. admins/automation; direct pushes to main are now blocked (PR-only).
- CLAUDE.md is 188 lines (≤200 cap OK).
- The `engine-current-state` + `MEMORY.md` auto-memories were updated to v10.7.0 this session (were stale at v10.2.1).

## What We Tried (Chronological)

1. **Onboarding + Gate6 baseline.** Read parent (seq 10) + VISION + CHANGELOG. Ran `./scripts/verify.sh` → green @ `5410e74`, v10.2.1, 732 tests. Listed the 5 candidates difficulty-ordered; user `/goal`'d "1번부터 5번까지 진행 … 동시 진행 … ci 끝나면 머지하고 종료".
2. **Saved the plan** → `plans/VISION_FEATURES_BATCH_2026-06-17.md` (execution model: 5 separate minor-bump PRs; parallel impl via worktree agents for 1–4; opus-by-hand nine-slice; serial integration; merge gate = CI; visual = best-effort with TODO fallback).
3. **Probed monitor** via `screencapture` + Read the image → ON (live desktop). So real playtests viable.
4. **Launched 4 background Sonnet agents** (`isolation: worktree`, `model: sonnet`, `run_in_background`) for features 1–4 (tween/audio/coroutine/animated-tiles), each told to touch only its own files + `src/lib.rs`, NOT version/CHANGELOG/CLAUDE, and to run fmt/clippy/test/build + commit to a `feat/*` branch. Each returned branch + commit + API summary.
5. **Hand-built nine-slice (#5)** concurrently (opus, render path — handoff warns agents struggle there). Read `src/renderer/sprite.rs` instance loop, `geometry.rs` (InstanceRaw), `components.rs` Transform. Designed `NineSlice` + a pure `nine_slice_subquads()` geometry fn (unit-testable) + an additive 9-quad branch in the sprite loop that `continue`s before the normal single-quad path (existing sprites byte-identical). **Caught + fixed a vertical-orientation bug** before the example (world +y is screen-down, uv v=0 at sprite top → TOP border must map to low-v/screen-top). Built lib, built example, **visual playtest PASS**, 6 unit tests, fmt+clippy clean (fixed a `manual_range_contains` clippy nit). Committed to `feat/nine-slice`.
6. **Serial integration, version order.** For each feature: branch `pr/<name>` off latest main → `git merge --no-ff feat/<name>` → bump Cargo.toml + Cargo.lock + docs/CHANGELOG.md + CLAUDE.md (+module-map row) → commit → push → `gh pr create` → CI + local Gate6 → squash-merge → sync local main → next.
7. **#83 tween** integrated first. Local Gate6 green; **discovered the playtest binary wasn't built** — `tween_sequence` window never appeared (System Events couldn't find the process). Root cause: `./target/debug/examples/tween_sequence` **did not exist** — Gate6's clippy/test `--all-targets` only CHECK, they don't leave a runnable example binary; my earlier "CLEAN" log scans were false negatives on a "no such file" error. Fixed by `cargo build --example tween_sequence` first → window appeared, HUD showed live segment/fraction → PASS. Merged.
8. **#84 audio** integrated. No visual (audio); ran the example ~5s (alive, log clean) + relied on 3 CI-safe scheduling tests. Merged.
9. **#85 coroutine** integrated and **merged RED** — see Key Decisions / the incident. CI Rustdoc actually FAILED (private-intra-doc-link) but I merged because my `gh pr checks --watch | tail` masked gh's exit code (pipeline exit = `tail` = 0), so the "exit 0" notification was misleading. main went red on Rustdoc.
10. **Front-loaded #3/#4 visual playtests** on a throwaway `scratch/playtest` branch (merged coroutine + animated-tiles, built both examples, screencaptured) while #83's CI ran — coroutine demo (box spawn→slide→recolor) PASS; animated tiles (2 frames 1s apart, water/lava shades cycle, ground static) PASS. Discarded scratch.
11. **Caught the red main** when verifying #85's local Gate6 output — read it and saw `error: could not document skeleton-engine` (the `| tail -5` had captured the failure line). Investigated: CI uses `RUSTDOCFLAGS="-D warnings"` (ci.yml:116) and the global env has no RUSTDOCFLAGS; reproduced `private_intra_doc_links` at `coroutine.rs:3` (module doc linked the private `CoroutineStep`). Confirmed via `gh run view --json jobs` that on the main re-run only **Rustdoc** fails (the #85 PR's "Test fail" was a transient — main re-run shows Test=success).
12. **Hotfix #86** — dedicated PR from main: unlink `CoroutineStep` (keep backticks). Verified rustdoc `-D warnings` exit 0 (unmasked). Watched CI **unmasked** → all 4 green → merged → main restored.
13. **Recreated #4 on fixed main** (the earlier `pr/animated-tiles` was cut from the broken main, so it carried the bad coroutine.rs). Bumped 10.6.0, full Gate6 + CI both green (authoritative exits) → merged.
14. **Pre-fixed nine-slice doc on its branch** before integrating #5 (I'd skipped the doc gate on nine-slice). Found + fixed TWO doc bugs: the module-doc example used `App::new(Default::default())` (real API is no-arg) and `app.world_mut()` (real API is the `app.world` field) → would fail `cargo test --doc`; and a bare `[Sprite]` intra-doc link (Sprite not imported in the module) → `broken_intra_doc_links`. Verified doctest compiles + rustdoc `-D warnings` exit 0. Committed to `feat/nine-slice`.
15. **#88 nine-slice** integrated: bump 10.7.0 + NineSlice module-map row; full Gate6 + CI both green (Rustdoc passed — fixes worked) → merged. All 5 done.
16. **Final main Gate6** (authoritative) → green, 772 tests. **Cleanup:** removed 4 agent worktrees (`git worktree remove --force`), deleted merged feat/pr/worktree-agent branches, pruned. **Updated memory** (engine-current-state → v10.7.0 + new gotchas; MEMORY.md index). **Committed the plan doc** to main as `session: vision-features-batch …` (3f43ab3), pushed, watched the main CI → success.
17. **Branch protection** (post-completion, user asked "ci 그린 아니면 머지 안되도록 깃허브 설정 바꿀 수 있어?"). Verified gh admin + main unprotected; presented strictness options; user chose "엄격 (관리자 포함)"; applied via `gh api PUT …/branches/main/protection`; verified.

## Key Decisions

- **Parallel implementation, serial integration.** Implementation parallelized (4 background Sonnet worktree agents for the independent pure-/contained features 1–4; opus-by-hand for the render-path nine-slice). Merges are inherently serial because each PR bumps the same version lines (Cargo.toml/Cargo.lock/CHANGELOG/CLAUDE) — stacked/parallel PRs would collide. So: branch off latest main each time, 3-way merge the feature branch, bump, gate, merge, sync, repeat.
- **Agents do NOT bump version/CHANGELOG/CLAUDE** — those are integration-owned by opus, to keep the version sequence conflict-free and let agents touch only feature-distinct files (+ lib.rs).
- **Worktree isolation for agents** (not main-tree sequential) — true parallelism needs separate checkouts (else concurrent writes to shared lib.rs corrupt). Cost: 4 extra target dirs; acceptable.
- **nine-slice by hand, not an agent** — render path is borrow-hostile + visually-silent on regressions (seq 9/10 warnings). Done as an additive branch so existing sprites are byte-identical.
- **#85 doc fix as a DEDICATED hotfix PR (#86), not folded into #4** — main was red; a separate, properly-attributed hotfix restores main cleanly and decouples from the feature PR. Cost one extra CI cycle; correctness over speed.
- **No version bump for the hotfix** — doc-only correction to the just-merged 10.5.0, which was never tagged/published, so 10.5.0-corrected is fine.
- **Overlap local Gate6 with CI** (push → CI + local verify in parallel → merge only when both green) to cut per-feature wall-clock while keeping the full local bar.
- **Branch protection: enforce_admins=true** (user's "엄격" choice) — the only setting that actually prevents an automated admin from merging red (false would leave the bot able to bypass). Accepts the trade-off: direct pushes to main now blocked (PR-only), no admin emergency override.
- **Did NOT publish crates.io / did NOT tag 10.3.0–10.7.0** — same conservative deferral as seq 10; needs explicit user go.

## Evidence & Data

### PRs shipped (all squash-merged, branches deleted)

| PR | Version | Title | Tests added | Visual |
|---|---|---|---|---|
| #83 | 10.3.0 | TweenSequence chained tweens | 9 | ✅ square eased path + HUD |
| #84 | 10.4.0 | AudioManager::crossfade | 3 (CI-safe) | n/a (audio); clean run |
| #85 | 10.5.0 | Coroutine sequencer | 11 | ✅ spawn→slide→recolor |
| #86 | — | fix coroutine doc link (hotfix) | 0 | n/a |
| #87 | 10.6.0 | animated tiles | 11 | ✅ 2 frames, tiles cycle |
| #88 | 10.7.0 | NineSlice 9-patch | 6 | ✅ corners fixed + rotation |

### main commit log (this session)

```
3f43ab3 session: vision-features-batch … (v10.3.0–v10.7.0) [engine-hardening]
457e4fb feat(nine-slice): … (v10.7.0) (#88)
42c7a6b feat(tilemap): animated tiles (v10.6.0) (#87)
e51f46f fix(coroutine): unlink private CoroutineStep in module doc (#86)
beec2f1 feat(coroutine): … (v10.5.0) (#85)
76dd7f7 feat(audio): … (v10.4.0) (#84)
b818163 feat(tween): … (v10.3.0) (#83)
2aa9cdd session: vision-features-render-split [seq 10 close]
```

### Background Sonnet agent metrics

| Feature | Tokens | Tool uses | Duration | Result |
|---|---|---|---|---|
| #1 tween | 59,890 | 21 | ~273s | clean (ran fmt/clippy/test/build) |
| #2 audio | 82,913 | 30 | ~393s | clean (ALSO ran doc — clean) |
| #3 coroutine | 69,947 | 33 | ~372s | **skipped doc gate → doc lint slipped** |
| #4 animated-tiles | 131,779 | 84 | ~886s | clean (ran full Gate6 incl. doc + wasm) |

### The #85 red-merge incident (root cause, for the record)

- **Symptom:** PR #85 merged with CI Rustdoc + Test both showing fail; main went red on Rustdoc.
- **Cause 1 — agent gap:** the coroutine agent ran fmt/clippy/`test --all-targets`/build but NOT `RUSTDOCFLAGS="-D warnings" cargo doc` nor `cargo test --doc`. Its module doc `//! …[`CoroutineStep`]…` linked the *private* `CoroutineStep` → `private_intra_doc_links` (an error under `-D warnings`, which CI uses).
- **Cause 2 — my exit-masking:** both `./scripts/verify.sh | tail -N` and `gh pr checks <n> --watch | tail -N` make the pipeline exit code = `tail`'s (0), hiding the real failure. The "exit 0" task notification was `tail`'s, not verify.sh's / gh's. Caught only because I *read* the tail'd output and saw `error: could not document`.
- **Fix:** hotfix #86 (unlink). Process fix: run gate commands as `cmd > log 2>&1` (no trailing pipe/echo) so the background task's own exit code is authoritative; double-check `gh pr checks <n>` before merge. Mechanical fix: branch protection (now red can't merge at all).

### `cargo test --doc` vs `--all-targets` (the other trap)

`cargo test --all-targets` SKIPS doctests; CI runs `cargo test --doc` as a separate step (ci.yml:53-54). So a doc example that doesn't compile passes `--all-targets` but fails CI. nine-slice's example had `App::new(Default::default())` (real: `App::new()`) + `app.world_mut()` (real: `app.world` field) — both caught at integration before pushing (no red main).

### Final verification

`./scripts/verify.sh` on main (authoritative, exit 0): `all checks passed ✓`; lib `test result: ok. 772 passed; 0 failed`. Final main CI run (session commit) = `completed / success`.

## Code Analysis

- **`TweenSequence`** (`src/tween.rs`): a `Vec<Tween>` of segments played back-to-back. Builder `new`/`then(start,end,dur,easing)`/`push(Tween)`/`looping(bool)`; runtime `tick(dt)->f32` (carries leftover `dt` across segment boundaries so a big frame advances multiple segments), `value`/`finished`/`fraction` (segment-count fraction)/`reset`/`current_segment`/`segment_count`. f32-only by design (matches `Tween`; generic-over-Lerp out of scope). Agent added `current_segment`/`segment_count` because the example HUD needed them.
- **`AudioManager::crossfade(channel, new_path, repeat, dur)`** (`src/audio/playback.rs`): relocates the channel's current sink to an internal `"{channel}__xfade"` temp channel + schedules a `stop_when_done` fade-out there, then `play_fade_in`s the new track on `channel` → they overlap. Degrades to `play_fade_in` if nothing is playing. The temp sink is torn down by the existing `update()` when its fade completes. Reuses the `Fade` infra entirely; no new public type. Native-only (`AudioManager` is `cfg(not(wasm32))`).
- **`Coroutine`** (`src/coroutine.rs`): private `CoroutineStep` enum (`Wait`/`Run(Box<dyn FnMut(&mut World)+Send+Sync>)`/`RunFor{dur,elapsed,f}`); builder `new`/`wait`/`run`/`run_for(dur, |w,t|)` (t in 0..=1, final call at t==1.0). `CoroutineRunner` World resource (`start`/`active_count`). **`CoroutineSystem::run` uses the remove→tick→reinsert pattern**: `world.remove_resource::<CoroutineRunner>()`, tick all (closures get a free `&mut World`, no alias), `retain` drops finished, `world.insert_resource(runner)`. Closures must NOT re-enter the runner. Private type aliases `RunCb`/`RunForCb` silence clippy `type_complexity`.
- **animated tiles** (`src/tilemap/animation.rs` + `system.rs`): DECOUPLED tag-at-spawn. `TileAnimation{frames:Vec<u32>, frame_time}` + `frame_at(elapsed)`; `TileAnimationSet` (component on the tilemap entity, value→animation map); `AnimatedTileCell` (per-tile-entity tag with PRECOMPUTED frame UVs + phase); `AnimatedTileSystem` cycles tagged cells' `UvRect` each frame. `TilemapSystem` tags animated cells at spawn (via `make_anim_cell`) + refreshes on value change; per-frame cycling is render-only and does NOT bump `Tilemap::generation`, so the non-animated generation-cache fast path is fully preserved (zero extra work for maps with no `TileAnimationSet`).
- **`NineSlice`** (`src/nine_slice.rs`): `border:[f32;4]` (world-px) + `uv_border:[f32;4]` (UV fraction), both `[left,right,top,bottom]` (consts `LEFT`/`RIGHT`/`TOP`/`BOTTOM`). Ctors `new`/`uniform(px,frac)`. `nine_slice_subquads(size, uv, &ns) -> Vec<SubQuad{local_center,size,uv}>` (pure, unit-tested; partitions position+UV into a 3×3 that tiles exactly; skips zero-size quads). **Render integration** in `src/renderer/sprite.rs`: an additive branch in the `query::<Sprite>()` loop — when a `NineSlice` is present, build 9 `InstanceRaw::single` (each model = `Mat4::from_scale_rotation_translation(subquad_size, rot, pos + Vec2::from_angle(rot).rotate(local_center))`) and `continue` (skips the single-quad path). Rotates rigidly. Not for `AtlasSprite`/`ShaderMaterial`. **Orientation:** world +y is screen-down and the unit quad puts uv v=0 at its lowest local y (screen top), so row index 0 (lowest y / screen top) uses the TOP border + lowest v — a subtle, visually-silent mapping that was fixed before the example.
- **App/render API facts (used in examples + docs):** `App::new()` takes no args; `app.world` is a public field (no `world_mut()`); `Transform::new(pos, scale, rot)`, scale = total world size; `app.load_image(path)->Handle` synchronous; `DrawText::new(text, Vec2, size, [u8;4])` via the `TextQueue` resource; camera at origin ⇒ world coords == screen pixels (top-left origin).

## Files Changed

### Source code
- `src/tween.rs` — +`TweenSequence` + 9 tests (#83)
- `src/audio/playback.rs` — +`crossfade`; `src/audio/tests.rs` — +3 CI-safe tests (#84)
- `src/coroutine.rs` — NEW module + 11 tests; doc-link fix (#85/#86)
- `src/tilemap/animation.rs` — NEW; `src/tilemap/system.rs` — tag-at-spawn integration; `src/tilemap/mod.rs` — re-exports (#87)
- `src/nine_slice.rs` — NEW + 6 tests + 2 doc fixes; `src/renderer/sprite.rs` — 9-quad additive branch + import (#88)
- `src/lib.rs` — `pub mod`/`pub use` for coroutine, nine_slice; tilemap + tween re-export lines extended

### Examples
- `examples/tween_sequence.rs`, `examples/music_crossfade.rs`, `examples/coroutine_demo.rs`, `examples/animated_tiles.rs`, `examples/nine_slice.rs` — all NEW top-level (cargo auto-discovers; no Cargo.toml entry)

### Docs / meta
- `Cargo.toml` / `Cargo.lock` — 10.2.1 → 10.7.0 (five bumps)
- `docs/CHANGELOG.md` — `## 10.3.0`–`## 10.7.0`
- `CLAUDE.md` — header v1.6.52→v1.6.57; +Coroutine row, +NineSlice row; tween/audio/tilemap rows extended
- `plans/VISION_FEATURES_BATCH_2026-06-17.md` — NEW (the saved plan; committed in 3f43ab3)

### Repo config (GitHub, not a file)
- `main` branch protection — required status checks (4 CI contexts) + strict + enforce_admins=true

## User Feedback & Preferences (REQUIRED)

- **`/goal` (the driving directive):** "plan을 저장하고 1번부터 5번까지 진행. 화면이 보여야 하는 테스트가 있는데 만약 모니터가 꺼져있어서 검은 화면만 확인 된다면 todo로 남기고, 이후 내가 모니터 열어주면 다시 테스트. 서브에이전트 사용해서 동시 진행 가능한 작업 동시 진행하고, 전체 작업 ci 끝나면 머지하고 종료." → merge authority granted (this run only); parallelism wanted; visual tests with monitor-off TODO fallback.
- **Asked for the recommended order first**, difficulty-ordered ("추가 되는 기능들만 작업 난이도 순으로 정리해서 리스트 만들어 줘") before greenlighting.
- **Praised honesty:** "너의 솔직함이 앞으로 있을 실수를 막았어 훌륭해" — explicitly values faithful reporting of the red-merge incident over papering over it. Carry this: report failures plainly with evidence.
- **Requested a mechanical guard:** "ci 그린 아니면 머지 안되도록 깃허브 설정 바꿀 수 있어?" → chose **"엄격 (관리자 포함)"** (enforce_admins=true) when offered strictness options.
- **Standing (carried):** Korean prose to the user / English code + docs + handoff; Sonnet subagents with explicit `model: sonnet` ([[new-model-subagent-incompat]]); no breaking changes without sign-off; never tag/publish unprompted; **merge authority re-confirm each session** (it was `/goal`-scoped here).

## Where We're Going

(No work in flight — batch complete. Options for a fresh session.)
1. **Decide crates.io publish** — still never done; irreversible first-ever name claim; needs explicit go. Otherwise stay GitHub-only.
2. **Tag + GitHub Release v10.3.0–v10.7.0** (and/or a single v10.7.0) — none of this batch is tagged; seq 10 tagged its three. Cheap, on request.
3. **Next VISION feature** — the seq-10 candidate list is now fully consumed; pick a new one (e.g. particle trails, tilemap layers/parallax-tilemap, gamepad rumble, screen-shake presets, save-slot UI, dialogue/textbox system) and run the feature→example loop.
4. **Backfill v5–v9 git tags** (optional, from seq 10).

## Risks & Blockers

- **main is now PR-only (branch protection, enforce_admins=true).** Direct `git push origin main` is BLOCKED — all changes, even docs, must go through a PR whose 4 CI checks pass. `strict=true` means PR branches must be up-to-date with main before merging (rebase if main moved). No admin emergency override. To relax/remove: `gh api -X DELETE repos/ChunSam/skeleton-engine/branches/main/protection` (or PUT with enforce_admins=false).
- **Required-check names are coupled to CI job names.** If `.github/workflows/ci.yml` job names change (`Build (WASM)`/`Test (native)`/`Rustdoc`/`Package dry-run`), update the branch-protection contexts too or merges wait forever on a check that never reports.
- **Render path still has no GPU/CI test** — any future `src/renderer/sprite.rs` / `render.rs` change needs the manual screencapture playtest (recipe in Quick Start). nine-slice's additive branch is the only new render surface; existing sprites untouched.
- **Audio + nine-slice + animated-tiles correctness beyond unit tests relies on eyeball/ear on this macOS box** — headless can't reproduce.

## Open Questions

- Publish to crates.io? (default: no — explicit go needed)
- Tag/Release v10.3.0–v10.7.0? (not done; on request)
- Which VISION feature next? (candidate list exhausted; user picks or I propose)

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -8                 # 3f43ab3 session; 457e4fb/#88 … b818163/#83
grep -m1 '^version' Cargo.toml       # 10.7.0
git status -s                        # clean
./scripts/verify.sh                  # main green (772 lib tests); RUN AS-IS, do NOT pipe to tail

# Read first
#   THIS handoff (seq 11)
#   plans/VISION_FEATURES_BATCH_2026-06-17.md  (plan + lessons)
#   docs/CHANGELOG.md → ## 10.7.0 … 10.3.0
#   parent: HANDOFF_engine-hardening_vision-features-render-split_2026-06-17.md (seq 10)

# Key new files to read
#   src/nine_slice.rs · src/coroutine.rs · src/tilemap/animation.rs
#   src/tween.rs (TweenSequence) · src/audio/playback.rs (crossfade)
#   src/renderer/sprite.rs (the 9-quad branch in the query::<Sprite>() loop)

# PROCESS GUARDS (learned the hard way this session):
#   - NEVER pipe a gate command to tail/head: `verify.sh | tail` and
#     `gh pr checks --watch | tail` make the exit code = tail's (0) → masks failures.
#     Run `cmd > /tmp/x.log 2>&1` (no pipe) so the bg-task exit is authoritative;
#     also eyeball `gh pr checks <n>` before merging.
#   - Agent Gate6 MUST include the doc gate: RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
#     AND cargo test --doc (--all-targets skips doctests). A doc example must COMPILE
#     (App::new() no-arg; app.world is a field, no world_mut()).
#   - `cargo build --example <name>` BEFORE any screencapture playtest (Gate6 doesn't
#     leave runnable example binaries → you'd capture a missing-file no-op).
#   - main is PR-ONLY now (branch protection). No direct push; open a PR, let CI gate it.

# Visual playtest recipe (macOS, monitor must be ON; else TODO + retest later):
#   EX=nine_slice; cargo build --example "$EX"
#   ( ./target/debug/examples/"$EX" > /tmp/s.log 2>&1 & echo $! > /tmp/s.pid )
#   sleep 7; osascript -e "tell application \"System Events\" to set frontmost of (first process whose name is \"$EX\") to true"
#   sleep 1.5; screencapture -x /tmp/s.png; kill "$(cat /tmp/s.pid)"; pkill -f "examples/$EX"
#   grep -aiE 'panic|validation|error|wgpu|no such' /tmp/s.log   # then Read /tmp/s.png

# Next action (pick one — no work in flight):
#   (a) Tag + GitHub Release v10.3.0–v10.7.0 (cheap; seq 10 tagged its batch).
#   (b) Publish to crates.io — ONLY on explicit user go (irreversible first publish).
#   (c) Propose/scope the next VISION feature (seq-10 candidate list now exhausted).
```

## Reusable Gotchas (HIGH VALUE — read before next session)

- **NEVER pipe a gate command to `tail`/`head`.** `./scripts/verify.sh | tail -N` and `gh pr checks <n> --watch | tail -N` set the pipeline exit code to `tail`'s (always 0), masking real failures — this directly caused the red #85 merge. Run `cmd > /tmp/x.log 2>&1` with NO trailing pipe/echo so the background task's own exit code is authoritative; then read the log AND eyeball `gh pr checks <n>` before merging. For push-to-main runs (no PR), `gh run watch <id> --exit-status`. ([[ci-toolchain-pin]] already noted "pipefail in gate pipes" — heed it.)
- **Agent Gate6 MUST include the doc gate.** A Sonnet agent that runs only fmt/clippy/`test --all-targets`/build will MISS doc lints — `--all-targets` skips doctests, and clippy doesn't run rustdoc. Require agents to also run `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` + `cargo test --doc`, OR run them yourself at integration before pushing. (coroutine slipped a `private_intra_doc_links`; nine-slice would have slipped a broken `[Sprite]` link + a non-compiling doc example — both caught at integration.)
- **`cargo build --example <name>` BEFORE any screencapture playtest.** Gate6's clippy/test only CHECK; they leave NO runnable `target/debug/examples/<name>` binary. Launching a missing binary = blank capture + a log whose "no such file" doesn't match `panic|error|wgpu` greps → false "CLEAN". Confirm `ls target/debug/examples/<name>` exists.
- **Doc examples must COMPILE** (CI runs `cargo test --doc`): `App::new()` takes no args; `app.world` is a public field (NO `world_mut()`); module-doc intra-links to `pub(crate)`/private items fail under `-D warnings` (`private_intra_doc_links`) — use an explicit `(crate::path)` or drop the link.
- **Worktree-agent integration mechanics:** agents commit to `feat/*` branches inside `.claude/worktrees/agent-*`. From the main tree you CAN'T `git checkout feat/<name>` (checked out in the worktree) but you CAN `git merge feat/<name>` into a `pr/<name>` branch (merge reads the ref). lib.rs additions auto-merged all 5 times (different regions). Cleanup at the end: `git worktree remove --force <path>` for each, delete merged `feat/*`/`pr/*`/`worktree-agent-*` branches, `git worktree prune`.
- **Stale rust-analyzer diagnostics flood from concurrent worktrees** (ColliderHandle "E0308 expected X found X", unlinked-file, inactive-cfg). Pure noise — trust `cargo`/CI, not the diagnostic blob.
- **macOS playtest focus is finicky:** `osascript … set frontmost … to true` can fail silently; if System Events errors `-1728` ("process can't be got") the example isn't a GUI process → it didn't build/open (usually the missing-binary trap above), not a focus issue.
- **`gh pr merge --squash --delete-branch` deletes only the REMOTE branch** — local `pr/*` branches persist; clean them up manually.

## Non-obvious Technical Findings

- **Additive render branch = byte-identical existing sprites.** The nine-slice 9-quad path only runs when a `NineSlice` component is present and `continue`s before the normal single-quad push — so all existing sprite rendering is untouched (same principle as the `geometry.rs` blend comment: `blend==0` ⇒ identical output). This is the safe way to extend the borrow-hostile render path without a full GPU regression risk.
- **nine-slice vertical orientation is visually-silent if wrong.** World +y is screen-down; the unit quad maps uv `v=0` to its lowest local y (= screen top). So the partition's row index 0 (lowest y / screen top) must use the TOP border and the LOWEST v. Uniform borders hide a wrong mapping entirely — only an asymmetric border reveals it. Fixed before writing the example; verified by the playtest (corners crisp, no flip).
- **animated-tiles phase stagger:** the spawn-time phase offset `((row+col) * frame_time * 0.37) % total_time` desyncs neighboring animated cells so water/lava doesn't flash in unison. Frame index = `(elapsed + phase) / frame_time mod frames.len()`; unit-tested with `frame_time=0.25` (exact in f32) to avoid modulo rounding flake.
- **coroutine closures carry `Send + Sync + 'static`** (agent's choice; World resources don't strictly require it but it's harmless + future-proofs threaded scheduling). The remove→tick→reinsert pattern is the ONLY way to pass `&mut World` into closures stored in a World resource without a borrow conflict.
- **`AudioManager::crossfade` reuses, not reinvents.** Overlap is achieved by moving the live sink to a temp channel `"{ch}__xfade"` + a `stop_when_done` fade-out there, then `play_fade_in` on the real channel. `update()` already tears down stop-when-done sinks → no new lifecycle code. CI has no audio device, so tests assert only the `fades`/`sinks` map STATE (skip cleanly if `AudioManager::new()` returns None headless).

## Evidence & Data (continued)

### `main` branch protection (applied this session)

```json
PUT /repos/ChunSam/skeleton-engine/branches/main/protection
{
  "required_status_checks": { "strict": true,
    "contexts": ["Build (WASM)", "Test (native)", "Rustdoc", "Package dry-run"] },
  "enforce_admins": true,
  "required_pull_request_reviews": null,
  "restrictions": null
}
```
Verified read-back: `strict=true | enforce_admins=true | checks=Build (WASM), Test (native), Rustdoc, Package dry-run`. Remove with `gh api -X DELETE …/branches/main/protection`.

### CI job timings (approx, from `gh pr checks`)

| Job | Typical |
|---|---|
| Build (WASM) | ~37s |
| Rustdoc | ~45–52s |
| Package dry-run | ~1m–2m45s |
| Test (native) | ~3m49s–8m16s (fmt+clippy+test --all-targets+test --doc+build --release) |

### Visual playtest matrix (all PASS; monitor was ON throughout)

| Feature | What was confirmed |
|---|---|
| nine_slice | orange corners stay 16px on wide/tall/small/large panels; rotating panel rigid; naive-stretch row smears (contrast) |
| tween_sequence | square animates the eased multi-leg path; HUD shows live `seg 1/4 [EaseOut] frac 0.32` |
| coroutine_demo | scripted scene: box spawned, recolored orange, "restarting sequence…" phase label |
| animated_tiles | two frames 1s apart — top water/lava cells cycle shades, bottom ground rows static |
| music_crossfade | (audio, no capture) process alive 5s, log clean; 3 scheduling tests |

### New public API — exact signatures (crate-root re-exports)

```rust
// src/tween.rs
pub struct TweenSequence { /* private */ }
impl TweenSequence {
    pub fn new() -> Self;
    pub fn then(self, start: f32, end: f32, duration: f32, easing: Easing) -> Self;
    pub fn push(self, tween: Tween) -> Self;
    pub fn looping(self, enabled: bool) -> Self;
    pub fn tick(&mut self, dt: f32) -> f32;       // carries leftover dt across segments
    pub fn value(&self) -> f32;  pub fn finished(&self) -> bool;  pub fn fraction(&self) -> f32;
    pub fn reset(&mut self);  pub fn current_segment(&self) -> usize;  pub fn segment_count(&self) -> usize;
}

// src/audio/playback.rs (AudioManager, native-only)
pub fn crossfade(&mut self, channel: &str, new_path: &str, repeat: bool, dur: f32);

// src/coroutine.rs
pub struct Coroutine; // new/wait(secs)/run(|&mut World|)/run_for(dur, |&mut World, f32|)
pub struct CoroutineRunner; // new/start(Coroutine)/active_count() -> usize  [World resource]
pub struct CoroutineSystem;  // impl System; remove→tick→reinsert the runner

// src/tilemap/animation.rs
pub struct TileAnimation { pub frames: Vec<u32>, pub frame_time: f32 } // new/total_time/frame_at(elapsed)->usize
pub struct TileAnimationSet;  // new/insert(value, anim)/get/contains/remove/iter  [component on tilemap entity]
pub struct AnimatedTileCell { pub frame_uvs: Vec<UvRect>, pub frame_time: f32, pub phase: f32, pub elapsed: f32 }
pub struct AnimatedTileSystem; // impl System; render-only, does not bump generation

// src/nine_slice.rs
pub struct NineSlice { pub border: [f32;4], pub uv_border: [f32;4] } // [left,right,top,bottom]
impl NineSlice { pub fn new(border:[f32;4], uv_border:[f32;4]) -> Self; pub fn uniform(px:f32, frac:f32) -> Self; }
pub const LEFT/RIGHT/TOP/BOTTOM: usize;
```

## Session Closed

**Closed at:** 2026-06-17 (KST evening)
**Commit:** feature work + branch protection landed at main `3f43ab3` (v10.7.0); this handoff merged to main via its own PR (main is now PR-only).
**Session status:** Handed off to next session (seq 11). VISION feature batch COMPLETE (5 features v10.3.0→v10.7.0, all CI-green merged); main is now branch-protected (required CI checks + enforce_admins). No work in flight; no wakeup armed. Merge authority was `/goal`-scoped to this run — re-confirm next session.
