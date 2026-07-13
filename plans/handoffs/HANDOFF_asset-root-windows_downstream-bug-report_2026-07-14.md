# Downstream bug report served: Windows/DX12 build unbroken + engine-owned asset roots (v0.125.0, v0.126.0)

**Date:** 2026-07-14
**Status:** COMPLETED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `asset-root-windows` seq `1`
**Parent:** `none — first in chain`
**Prior chain:** none — first in chain

---

## Related Handoffs

- `plans/handoffs/HANDOFF_breadth-fov_rng-and-scene-transition_2026-07-11.md` — the **previous chain's close** (`breadth-fov` seq 4). It is NOT this chain's parent: it ended with the standing directive *"SHELF EXHAUSTED → next session reads the board FIRST; if empty, ASK for a NEW area."* This session did not need to ask, and did not self-pick — a **real downstream bug report** arrived and became the work. Read it only for the prior feature arc (FOV / procgen / Rng / SceneTransition), not for continuation context.

## Reference Documents

- `CLAUDE.md` — project conventions, module map, the verify-gate rules. **Updated this session** (new `asset_path` module-map row; header → v1.6.219 / package v0.126.0).
- `docs/VISION.md` — the feature+example loop ("a feature is not done until a playable example exercises it").
- `docs/CHANGELOG.md` — 0.125.0 and 0.126.0 entries written this session.
- **The request source (a second repo):** `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` and `../rust-survivors/docs/ENGINE_ASSET_LOADING_REQUEST.md`.

---

## The Goal

The user builds a **Windows release exe of `rust-survivors` to share with other people**. Doing that surfaced two engine defects, which the game repo filed as engine requests on 2026-07-13: the packaged exe rendered a **solid magenta window** (Request A), and before that the Windows release build **would not compile at all** (Request B). The session's job was to check those requests, decide *where* the fix belongs, and serve them.

The answer to "where" turned out to be the crux: **fix the engine, change nothing in the game.** Both defects were real, live engine bugs — not artifacts of the game's old engine pin — and every future consumer (dungeon-merchant, any fork) would hit them the day they packaged a build. The game already ships fine via a zip and its own workarounds, and it pins an engine version too old to receive the fix anyway.

End state: both requests are fixed and merged in the engine (v0.125.0, v0.126.0), the Windows build is now guarded by a **required** CI job, `rust-survivors` is untouched apart from its request docs being marked resolved.

---

## Where We Are

- **`main @ de95bbb`, package v0.126.0, CLAUDE.md header v1.6.219, clean tree, all gates green.**
- **PR #358 (v0.125.0) — MERGED** `1e1609e`. rodio 0.19 → 0.22.2; `src/audio/` migrated to the new API; new `Build (Windows / DX12)` CI job.
- **PR #359 (v0.126.0) — MERGED** `de95bbb`. New `src/asset_path.rs`; `resolve()` applied at 10 filesystem read sites; loud failures; example `packaged_assets`; regression test `tests/asset_root.rs`.
- **Branch protection updated:** required checks went **5 → 6**, adding `Build (Windows / DX12)`. `strict=true` preserved. (User approved via AskUserQuestion.)
- **The DX12 backend now compiles in CI for the first time ever** — the `windows` job passed in 4m33s on #358, 1m35s on #359 (cached).
- `cargo update -p rodio` printed the money line: **`Removing windows v0.54.0`**. The MSVC target now resolves exactly one `windows` version (0.62.2).
- **Verified end to end by hand:** built `target/debug/examples/packaged_assets`, ran it from `/`, and the sprite renders in colour. That exact launch used to produce a full magenta window.
- **The regression test was verified to actually fail without the fix** — reverting `resolve()` at one read site turns `tests/asset_root.rs` red. It is not a test that passes for free.
- Engine test count: **1241 lib tests** pass (was ~1229 at session start); `asset_path` contributes 12 unit tests; `tests/asset_root.rs` adds 1 integration test.
- **`rust-survivors` PR #27** (a pre-existing open PR from a prior session, `Adopt engine::KeyCode…`) was **merged** — it touched the same requests doc, so it had to land first. Game main is now `43b1abd`.
- **`rust-survivors` PR #28 — MERGED** `7794358` (CI `macOS game checks` pass 5m23s; that repo has **auto-merge disabled**, so it was merged by hand). Both 2026-07-13 requests are now under "Completed" with what-shipped notes, `ENGINE_ASSET_LOADING_REQUEST.md` carries a RESOLVED banner, and the game's **Open Requests section now reads `_None._`** — the request queue from that repo is empty.
- **Phase 3 (byte-sourced images / `App::load_image_bytes`) was deliberately NOT built.** See Key Decisions.
- Memory bumped: `engine-current-state` → seq 179; `rust-survivors-deprecated` rewritten (it is paused as a project but is a **live bug-report channel**).

---

## What We Tried (Chronological)

### Chunk 1 — Reading the request, and checking whether it was still true (early)

1. **Read the two requests.** `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` had two entries under "Open Requests", both dated 2026-07-13, both pointing at a fuller prompt doc `ENGINE_ASSET_LOADING_REQUEST.md`. Request A = asset loading vs. the working directory (the magenta screen). Request B = duplicate `windows` crate versions breaking DX12.

2. **Did NOT take the request at face value — cross-checked it against the current engine.** The requests were written against the game's pinned engine (`rev a3369ee` = **v7.0.0**, which predates the 0.x version reset; the engine is now 0.12x). So the first real question was: *are these still bugs on `main`?* Grepped for `set_asset_root` / `load_image_bytes` / `failed_assets` (none exist), checked `renderer/texture.rs:56` (still `warn!` + magenta fallback), checked the `rodio` version (still 0.19).

3. **Confirmed the `windows` conflict is LIVE on engine main v0.124.0**, not merely on the game's old pin:
   ```
   $ cargo tree --target x86_64-pc-windows-msvc -i windows
   error: specification `windows` is ambiguous
     windows@0.54.0
     windows@0.62.2
   ```
   `windows 0.54.0` ← `cpal 0.15.3` ← `rodio 0.19` ← skeleton-engine.
   `windows 0.62.2` ← `gilrs-core`, `gpu-allocator 0.28`, `wgpu-hal 29.0.3`.
   **Nuance that matters:** the engine's *lockfile* happened to have `gpu-allocator` resolved onto **0.62.2**, so `main` might have compiled on Windows by luck. But `gpu-allocator` declares `windows = ">=0.53, <=0.62"` — a range spanning both nodes — so any re-resolve (a `cargo update`, or a different dependency set, i.e. exactly the game's situation) can flip it onto 0.54 and break DX12. The engine was one `cargo update` away from the game's failure, and **no CI job would have noticed**.

4. **Found the game already ships fine.** Its recent commits (`3c5dd7e` resolve assets relative to the executable, `af31f35` Windows zip packaging, `e73773b` unify audio path resolution) mean `crates/game/src/survivor/asset_root.rs` already `set_current_dir`s to the resolved asset root, and `scripts/package_windows.ps1` produces a working zip (exe + `assets/` + licences + a SHA256 manifest). So the *sharing goal was already met*; what remained was the engine defect.

5. **Reported to the user** with a table of "what the request asked for" vs "current engine state", flagging that (a) the audio half of byte-sourced assets already exists (`AudioManager::play_bytes`), and (b) the memory said `rust-survivors` was deprecated, which now looked stale.

6. **User clarified the framing** (this is what unlocked the plan): the project *is* paused, but they build a Windows exe from it **to share with other people**, and hit the asset failure doing exactly that. Asked for a fix plan.

7. **Presented a 3-phase plan** (Phase 1 rodio/Windows, Phase 2 asset root + loud failures, Phase 3 byte-sourced images) with an explicit recommendation to skip Phase 3 and keep the zip. **User chose "Phase 1 → 2 순차".**

### Chunk 2 — Phase 1: the rodio bump (mid)

8. **Bumped `rodio` 0.19 → 0.22 and re-resolved.** `cargo update -p rodio` printed `Removing windows v0.54.0`, `Removing windows-core v0.54.0`, `Removing windows-result v0.1.2`. `cpal` went 0.15.3 → **0.17.3**, which is on `windows 0.62.2` — the same node as `wgpu-hal`. `grep -A1 'name = "windows"' Cargo.lock` then showed exactly one version.

9. **Read the rodio 0.22 API out of the vendored source, not from memory.** This was worth doing — 0.21/0.22 is a large rework and guessing would have cost several compile cycles. From `~/.cargo/registry/src/*/rodio-0.22.2/src/`:
   | 0.19 | 0.22 |
   |---|---|
   | `Sink` | **`Player`** — `Player::connect_new(&Mixer)`, **infallible** (was `Sink::try_new(&handle) -> Result`) |
   | `OutputStream` + `OutputStreamHandle` | one **`MixerDeviceSink`** via `DeviceSinkBuilder::open_default_sink() -> Result<MixerDeviceSink, DeviceSinkError>`; `.mixer() -> &Mixer` |
   | `Source::current_frame_len` | `Source::current_span_len` |
   | `channels() -> u16` | `channels() -> ChannelCount` = **`NonZero<u16>`** |
   | `sample_rate() -> u32` | `sample_rate() -> SampleRate` = **`NonZero<u32>`** |
   | samples `i16`/`f32` + `convert_samples()` | `Source: Iterator<Item = Sample>`, **`Sample = f32`**; `convert_samples` is GONE |
   | trait object `Box<dyn Source<Item = i16>>` | `Box<dyn Source + Send>` (`source_pointer_impl!(Source for Box<dyn Source + Send>)`) |
   | `SamplesBuffer::new(u16, u32, data)` | `SamplesBuffer::new(ChannelCount, SampleRate, impl Into<Vec<f32>>)` |

10. **The feature trap:** rodio 0.22 moved `cpal` behind a **`playback`** feature. Since the engine uses `default-features = false`, the bump needed `features = ["playback", "wav", "vorbis", "mp3"]` — without `playback` there is no output stream at all. Codec features still exist by the same names but are now **symphonia**-backed. MSRV of rodio 0.22.2 is 1.87 (engine is 1.95 — fine).

11. **Migrated 4 files** (`src/audio.rs`, `src/audio/{types,source,playback}.rs`). First build: **20 errors**, all in `src/audio/`, all the predicted API breakage. The f32-everywhere change was a net *simplification*: `append_decoded`'s effect chain had a nested branch tree doing `.convert_samples::<f32>().low_pass(hz).convert_samples::<i16>()` round-trips; it collapsed to a flat `speed → low_pass → fade_in` chain over `Box<dyn Source + Send>`. Net diff for the PR: **298 insertions, 327 deletions** across 9 files.

12. **Kept the CPU tone envelope on purpose.** rodio 0.22 *does* now have `Source::fade_out` (it didn't in 0.19, which is what the old comment blamed). Switching `play_tone`'s de-click envelope to source combinators would have changed the samples and broken the golden-value test `tone_envelope_ramps_to_zero_at_both_ends`. Left `enveloped_tone_samples` materializing into a `SamplesBuffer`; only fixed the now-false comment.

13. **Added the `Build (Windows / DX12)` CI job**, and made it assert the *invariant* rather than merely building: it counts `windows` versions on the MSVC target and fails with `::error::multiple 'windows' crate versions` if >1. Rationale in the job's own comment: a future dependency bump should fail with a readable message, not a wall of D3D12 type mismatches inside `wgpu-hal`.
    - **Tested the guard both ways locally**: `windows` → 1 version (passes); genuinely duplicated crates → `bitflags` = 2, `windows-sys` = 5, `getrandom` = 2 (would fail). Guarding only `windows` is correct — `windows-sys` duplication is normal and does not affect the `gpu-allocator`/`wgpu-hal` D3D12 type exchange.
    - Guard uses `|| true` on the `cargo tree -i` pipeline, because `cargo tree -i` **exits non-zero when the spec is ambiguous** — which is precisely the failure being tested for, so its status must be swallowed and the output judged instead.

14. **Judgment gate — the thing green CI could not check.** rodio 0.22 swaps the decoders to symphonia. If mp3 silently stopped decoding, the 2026-05-31 "engine owns its codec policy" promise would be voided *silently*. Discovered that **the engine repo ships ZERO audio assets** — only a hand-synthesized WAV header inside a test exercises `Decoder` at all. So: wrote a **throwaway** `tests/tmp_codec_smoke.rs`, pointed it at real files from `rust-survivors/assets/audio/` (10 mp3, 9 wav there), and confirmed both `rodio::Decoder::new` accepted them **and** `AudioManager::play` → `is_playing()` reported `true`. Then deleted the file.
    - **Known gap, left open and disclosed in the PR:** vorbis/`.ogg` is exercised by *nothing*. There is no `.ogg` anywhere in either repo, `afconvert` (the only encoder on this machine) cannot produce mp3/ogg, and the game's audio is licensed music that must not be committed into an MIT engine repo as a fixture.

15. **Shipped v0.125.0** (`/ship`: Cargo.toml + lock + CHANGELOG + CLAUDE.md header), opened PR #358. **Did NOT arm auto-merge** — deliberately. The new Windows job was not yet in branch protection's required checks, so auto-merge would have merged *without waiting for it*, defeating the entire point of the PR. Watched instead: **CI 6/6 green, `Build (Windows / DX12)` pass 4m33s**. Squash-merged.

16. **Then closed that hole:** asked the user, who approved, and added `Build (Windows / DX12)` to `main`'s required status checks via `gh api -X PATCH .../protection/required_status_checks` (5 → 6 contexts, `strict: true` preserved).

### Chunk 3 — Phase 2: asset roots (late)

17. **Mapped the filesystem read sites** before designing anything: `grep -rn "fs::read\|read_to_string" src/` → 10 asset read sites, plus `asset.rs:241`'s `canonicalize` (which is a **cache-key** concern, not a read).

18. **The load-bearing design decision fell out of that map:** every read site takes the path as an argument while the *caller* keeps that same string as the cache key (`SpriteRenderer::texture_cache`, `Handle::path()`, `read_cached_bytes`'s `HashMap<String, _>`). So `resolve()` could be inserted **purely at the read**, leaving identity untouched. This is not a stylistic choice — see Key Decisions.

19. **Wrote `src/asset_path.rs`**: candidate roots (macOS bundle `Contents/Resources` → exe dir → exe ancestors (`ANCESTOR_DEPTH` = 4) → cwd → cwd ancestors, deduped), `resolve(path)` returning the first candidate under which the file **exists**, `set_asset_root` / `asset_root` / `clear_asset_root`, `record_failure` / `asset_failures` / `clear_asset_failures`, `set_strict_assets` / `strict_assets`.

20. **Applied `resolve()` at all 10 read sites** via a scripted, anchored Python edit (one assert per anchor, printing `ok`/`SKIP` per file), then confirmed `grep -rn "std::fs::read(path)\|std::fs::read_to_string(path)" src/` returned nothing outside `asset_path.rs`.

21. **Reworked failure recording after noticing a duplicate.** `App::load_image` feeds **two** subsystems — `pending_textures` (renderer upload, fails in `renderer/texture.rs`) and `AssetServer` (CPU decode, fails in `asset/image_loading.rs`) — so one missing file produced **two** entries. Moved the "searched roots" suffix *into* `record_failure` (so both call sites get a uniform, useful message) and made it **dedup by path** (first wins).

22. **Example `packaged_assets`** (VISION rule: the example is the acceptance test). It loads a real texture by a **relative** path — an absolute path would sidestep the entire bug — plus one deliberately-missing texture, and its HUD prints the working directory, the roots searched, and the live `asset_failures()` panel. In `HEADLESS_SHOT` mode it **exits non-zero** if the real texture failed to resolve, so it doubles as a runnable check.
    - Guessed the API wrong twice on first write (`Transform::from_xy`, `Sprite::with_size`, missing `WindowConfig.clear_color` / `Transform.z`) — the real shapes are `Transform { position, scale, rotation, z }`, `Sprite { texture, color, image_handle }`, `world.spawn()` → `add_component`.

23. **Ran the acceptance test for real.** Built the example, then ran the binary from `/`:
    ```
    $ cd / && HEADLESS_SHOT=/tmp/pa_foreign.png .../target/debug/examples/packaged_assets
    OK: 'examples/assets/hex_tiles.png' resolved; failures = ["examples/assets/__deliberately_missing__.png"]
    exit=0
    ```
    Read the PNG back: hex tiles render in green/tan (**not magenta**), HUD shows `working dir: /` and the 5 roots searched, the missing texture reported **once**.

24. **Pinned it with `tests/asset_root.rs`** — moves the cwd to a temp dir, loads a relative asset, asserts no failure. Needs **no GPU** (the `AssetServer` decodes on the CPU), so it runs on the ordinary CI test job rather than the lavapipe render job.

25. **Verified the test isn't vacuous.** Temporarily reverted `resolve()` in `image_loading.rs` → test went **RED** with exactly the informative message the design intends (path + every root searched) → restored → green. A regression test that passes without the fix is worthless; this one was checked.

26. **wasm gate trip.** `cargo clippy --target wasm32 --lib -D warnings` failed with 3 × `never used`: `candidate_roots()` is a **stub** on wasm (no filesystem), so `macos_bundle_resources` / `push_unique` / `candidates_from` become dead there. Gated those three plus the whole `mod tests` with `cfg(not(target_arch = "wasm32"))`. (Note: `cargo fmt` had re-wrapped `candidates_from`'s signature across lines, so the first anchored edit missed — the classic fmt-reflow trap.)

### Chunk 4 — Phase 2's CI failure, and closing out (late)

27. **PR #359 went RED on CI** — `Test (native)` failed while `Build (Windows / DX12)`, WASM, Rustdoc, Package dry-run and Render tests all passed. My own test:
    ```
    ---- asset_path::tests::failures_are_recorded_and_clearable stdout ----
    thread panicked at src/asset_path.rs:402:9:
    assertion `left == right` failed
      left: 2
     right: 1
    ```
    **Cause:** the test asserted the **process-global** `FAILURES` list had `len() == 1`. Other tests in the same lib binary record into that list **concurrently**. It passed locally on timing and raced on CI.

28. **Fixing it surfaced a worse latent bug.** The neighbouring test called `set_asset_root()` — mutating the **process-global asset root that every other parallel test's `resolve()` reads**. Nothing had broken yet, but any parallel test loading an asset while that root was pinned would have failed *at a distance*, and it would have looked like a flake in an unrelated module.
    - **Fix (both problems at once):** extracted the pure `resolve_in(roots: &[PathBuf], path: &Path) -> PathBuf` and made `resolve()` a thin wrapper (`resolve_in(&candidate_roots(), path)`). Both root tests now drive `resolve_in` with an explicit temp-dir root list, so **no test mutates the global root at all**. The failure test now asserts only on its **own unique path** (`assets/__unit_test_recorded_failure__.png`), never on the list's length, and additionally asserts the message contains `searched:`.
    - Also added a second root test (`the_first_root_that_holds_the_file_wins_not_merely_the_first_root`) since `resolve_in` made it trivial.
    - Stress-checked: full lib suite green **3× in a row** (1241 tests).

29. **PROCESS SLIP — pushed a red gate.** Ran the re-verify as `./scripts/verify.sh > log 2>&1; echo "VERIFY_EXIT=$?"; git commit ...; git push` **in one Bash call**. `VERIFY_EXIT=101` (clippy `cloned_ref_to_slice_refs` on `&[dir.clone()]`) but `;` does not stop on failure, so the commit and push happened anyway. Caught it immediately from the printed exit code and fixed it in a follow-up commit (`std::slice::from_ref(&dir)`), then re-ran verify **as its own call** → 0.
    - The project rule in `CLAUDE.md` says *don't PIPE a gate's exit code*. **Chaining with `;` hides it just as effectively.** Recorded as a live gotcha in memory.

30. **CI re-ran green (6/6), auto-merge landed `de95bbb`** at 2026-07-13T23:09Z. Synced main, deleted the branch, re-confirmed on the merged tree that the example still resolves from `/`.

31. **Closing the loop in the game repo.** User asked to mark the requests resolved, run `/handoff`, and merge. Discovered `rust-survivors` had an **open PR #27** from a prior session (`Adopt engine::KeyCode…`, CLEAN) which **also edited `ENGINE_CHANGE_REQUESTS.md`** (it fixes a misplaced `## Completed Requests` heading). Merging my doc edit around it would conflict. Asked the user rather than unilaterally merging someone else's PR; **user chose "merge #27 first"**. Merged #27 (`43b1abd`), then wrote the resolved entries on a clean main and opened **PR #28**.

---

## Key Decisions

- **Fix the engine; change nothing in `rust-survivors`.** The game pins `rev a3369ee` = **v7.0.0**, which predates the engine's 0.x version reset — an engine fix cannot reach it without a migration across 100+ releases, which is not worth paying on a paused project. It already ships correctly today via its workarounds + the zip. Rejected: bumping the game's engine pin; patching the game further.

- **`resolve()` is applied ONLY at the filesystem read, never to an asset's identity.** Cache keys and `Handle::path()` stay exactly the caller's string. This is the single most important constraint in the change. Rewriting them to the resolved path would **reintroduce a bug this engine has already shipped once** (the 2026-05-29 "Unify Image Texture Cache Keys" request): a handle keyed by the canonical path while the GPU texture cache is keyed by the relative one, so every sprite silently renders **white**. Identity stays logical; resolution stays at the filesystem edge.

- **Candidate search, not a single configured root** — and **executable-derived candidates before the working directory**. Two reasons: a shipped build must never silently pick up a stray `assets/` from wherever the user launched it; and a dev build's exe (`target/debug/examples/x`) reaches the repo root through its ancestors, which is exactly why **not one of the 100+ existing examples needed a change**. An explicit `set_asset_root` short-circuits to a single candidate (a miss is then a miss, not a quiet fallback).

- **A miss returns the path UNCHANGED.** When no candidate holds the file, `resolve` returns the caller's original relative path, so the ensuing error names what the caller actually asked for. The searched roots are attached by `record_failure` instead.

- **The Windows CI job asserts the invariant, not just the build.** Counting `windows` versions on the MSVC target gives a future dependency bumper a readable failure instead of ~10 D3D12 type-mismatch errors inside `wgpu-hal`.

- **Did NOT arm auto-merge on #358.** The new Windows job wasn't a required check yet, so auto-merge would have merged without waiting for it. Watched and merged manually, *then* added it to branch protection. (Auto-merge WAS used on #359, once the job was required.)

- **Phase 3 (`App::load_image_bytes` / `include_bytes!` single-file distribution) deliberately NOT built.** The audio half already exists (`AudioManager::play_bytes`). But it would not have helped the reporter: `rust-survivors/assets` is **93 MB (56 MB audio, 33 MB textures)**, so embedding means a ~100 MB executable and a link-time hit. The zip from `scripts/package_windows.ps1` is the right way to hand that game to someone. Phase 3 remains a legitimate *engine* feature for small/jam games — just not this bug's fix.

- **Kept the CPU-materialized tone envelope** despite rodio 0.22 gaining `Source::fade_out`, because a golden-value test pins the sample stream.

- **Did not commit audio fixtures.** Verifying mp3/vorbis decode permanently would need binary fixtures; `afconvert` can't encode them, and the game's licensed music must not land in an MIT repo. Disclosed the vorbis gap in the PR instead of faking coverage.

---

## Evidence & Data

### The dependency conflict, before and after

| | Before (rodio 0.19) | After (rodio 0.22.2) |
|---|---|---|
| `cpal` | 0.15.3 | **0.17.3** |
| `windows` versions on msvc | **0.54.0 AND 0.62.2** | **0.62.2 only** |
| `windows 0.54` pulled by | `cpal 0.15.3` ← `rodio 0.19` | *(removed from graph)* |
| `windows 0.62.2` pulled by | `gilrs-core`, `gpu-allocator 0.28`, `wgpu-hal 29.0.3` | same + `cpal 0.17.3` |
| DX12 backend | can fail (`gpu-allocator` may bind 0.54) | one version, cannot mismatch |

`cargo update -p rodio` output: `Removing windows v0.54.0`, `Removing windows-core v0.54.0`, `Removing windows-result v0.1.2`.

### The CI guard, validated both ways (local, msvc target)

| crate | versions counted | guard verdict |
|---|---|---|
| `windows` | **1** (0.62.2) | pass |
| `bitflags` | 2 | would fail |
| `windows-sys` | 5 | would fail |
| `getrandom` | 2 | would fail |
| `base64` | 1 | pass |

(Only `windows` is guarded — `windows-sys` duplication is normal and irrelevant to the `gpu-allocator` ↔ `wgpu-hal` D3D12 type exchange.)

### CI results

| PR | Run | Test (native) | WASM | **Windows / DX12** | Render (lavapipe) | Rustdoc | Package dry-run |
|---|---|---|---|---|---|---|---|
| #358 | 1st | pass 6m11s | pass 49s | **pass 4m33s** | pass 1m44s | pass 45s | pass 1m24s |
| #359 | 1st | **FAIL 3m59s** | pass 48s | pass 1m37s | pass 1m20s | pass 36s | pass 1m19s |
| #359 | 2nd | pass 5m32s | pass 38s | **pass 1m35s** | pass 1m4s | pass 34s | pass 59s |

### The #359 CI failure (my test, verbatim)

```
---- asset_path::tests::failures_are_recorded_and_clearable stdout ----
thread 'asset_path::tests::failures_are_recorded_and_clearable' panicked at src/asset_path.rs:402:9:
assertion `left == right` failed
  left: 2
 right: 1
test result: FAILED. 1239 passed; 1 failed; 0 ignored
```

### The regression test, proven non-vacuous

With `resolve()` reverted in `image_loading.rs` (i.e. the OLD behavior):
```
test a_relative_asset_resolves_from_a_foreign_working_directory ... FAILED
'examples/assets/hex_tiles.png' failed to load from working dir /var/folders/.../T/:
[AssetFailure { path: "examples/assets/hex_tiles.png",
  error: "image file read failed: No such file or directory (os error 2)
          (searched: /Users/jkl/Projects/skeleton-engine/target/debug/deps,
                     /Users/jkl/Projects/skeleton-engine/target/debug,
                     /Users/jkl/Projects/skeleton-engine/target,
                     /Users/jkl/Projects/skeleton-engine,
                     /private/var/folders/.../T, ...)" }]
```
With the fix restored: `test result: ok. 1 passed`.

### The acceptance test (example run from a foreign cwd)

```
$ cd / && HEADLESS_SHOT=/tmp/pa_foreign.png .../target/debug/examples/packaged_assets
OK: 'examples/assets/hex_tiles.png' resolved; failures = ["examples/assets/__deliberately_missing__.png"]
exit=0
```
Screenshot confirms: `working dir: /`, hex tiles rendered in green/tan (**not magenta**), 5 roots listed, 1 failure reported (not 2 — dedup works).

### Verify-gate runs this session

| Run | Result | Cause of failure |
|---|---|---|
| Phase 1 post-migration | **0** | — |
| Phase 1 post-`/ship` | **0** | — |
| Phase 2 first full run | **101** | wasm clippy: 3 × `never used` (`macos_bundle_resources`, `push_unique`, `candidates_from`) |
| Phase 2 after cfg-gating | **0** | — |
| Phase 2 post-`/ship` | **0** | — |
| Post-CI-fix | **101** | clippy `cloned_ref_to_slice_refs` — **and this one got committed+pushed anyway** (see Risks) |
| Final | **0** | — |

### rust-survivors facts (why Phase 3 was skipped)

| | |
|---|---|
| `assets/` total | **93 MB** |
| — audio | 56 MB (10 × mp3, 9 × wav) |
| — textures | 33 MB |
| — fonts | 4.4 MB |
| Engine pin | `rev a3369ee` = **v7.0.0** (pre-0.x-reset) |
| Ships as | zip via `scripts/package_windows.ps1` (exe + assets + licences + SHA256 manifest) |
| Workarounds in place | `crates/game/src/survivor/asset_root.rs` (`set_current_dir`), `Cargo.lock` hand-pin of `gpu-allocator`'s `windows` edge → 0.62.2 |

### Commits landed

| Repo | Hash | Summary |
|---|---|---|
| engine | `1e1609e` | `fix(audio): bump rodio 0.19 → 0.22 to unbreak the Windows/DX12 build (v0.125.0) (#358)` |
| engine | `de95bbb` | `feat(assets): resolve relative asset paths against an engine-owned root (v0.126.0) (#359)` |
| rust-survivors | `43b1abd` | `Adopt engine::KeyCode, add content tests, and prune stale docs (#27)` — pre-existing PR, merged to unblock the doc edit |
| rust-survivors | `7794358` | `docs: mark both 2026-07-13 engine requests resolved in the engine (#28)` |

### Branch protection (engine `main`)

Before: `["Build (WASM)", "Test (native)", "Rustdoc", "Package dry-run", "Render tests (lavapipe)"]` (5)
After: **+ `"Build (Windows / DX12)"`** (6). `strict: true` preserved.

```bash
gh api -X PATCH repos/ChunSam/skeleton-engine/branches/main/protection/required_status_checks \
  -f 'contexts[]=Build (WASM)' -f 'contexts[]=Test (native)' -f 'contexts[]=Rustdoc' \
  -f 'contexts[]=Package dry-run' -f 'contexts[]=Render tests (lavapipe)' \
  -f 'contexts[]=Build (Windows / DX12)'
```

### rodio 0.22.2 feature surface (why `playback` had to be added)

`cargo info rodio@0.22.2` — MSRV **1.87** (engine is 1.95, fine):

```
default    = [playback, recording, flac, mp3, mp4, vorbis, wav, dither]
playback   = [dep:cpal]        <-- NEW gate: with default-features=false, cpal (and therefore
                                   any output stream at all) is OFF unless you ask for this
mp3        = [symphonia-mp3]   <-- codecs are symphonia-backed now, same feature names
vorbis     = [symphonia-ogg, symphonia-vorbis]
wav        = [symphonia-wav, symphonia-pcm]
```

Engine's declaration, before → after:
```toml
# before
rodio = { version = "0.19", default-features = false, features = ["wav", "vorbis", "mp3"] }
# after
rodio = { version = "0.22", default-features = false, features = ["playback", "wav", "vorbis", "mp3"] }
```

### The `append_decoded` effect chain, before → after (rodio's f32 change collapsing it)

This is the clearest illustration of why the migration was a net simplification. **Before** (samples were `i16` on the sink, `f32` for filters, so every stage round-tripped):

```rust
let effected: Box<dyn Source<Item = i16> + Send + 'static> = if let Some(eff) = effect {
    if (eff.pitch - 1.0).abs() > 0.001 {
        let s = source.speed(eff.pitch);
        if let Some(hz) = eff.low_pass_hz {
            let s = s.convert_samples::<f32>().low_pass(hz).convert_samples::<i16>();
            if eff.attack_secs > 0.001 { Box::new(s.fade_in(...)) } else { Box::new(s) }
        } else if eff.attack_secs > 0.001 {
            Box::new(s.convert_samples::<i16>().fade_in(...))
        } else { Box::new(s.convert_samples::<i16>()) }
    } else if let Some(hz) = eff.low_pass_hz {
        let s = source.convert_samples::<f32>().low_pass(hz).convert_samples::<i16>();
        if eff.attack_secs > 0.001 { Box::new(s.fade_in(...)) } else { Box::new(s) }
    } else if eff.attack_secs > 0.001 { Box::new(source.fade_in(...)) } else { Box::new(source) }
} else { Box::new(source) };
// ...and later: PannedSource::new(effected.convert_samples::<f32>(), pan)
```

**After** (`Sample = f32` everywhere, so the stages just compose; order preserved exactly: speed → low-pass → fade-in):

```rust
let effected: Box<dyn Source + Send + 'static> = if let Some(eff) = effect {
    let pitched: Box<dyn Source + Send + 'static> = if (eff.pitch - 1.0).abs() > 0.001 {
        Box::new(source.speed(eff.pitch))
    } else { Box::new(source) };
    let filtered: Box<dyn Source + Send + 'static> = match eff.low_pass_hz {
        Some(hz) => Box::new(pitched.low_pass(hz)),
        None => pitched,
    };
    if eff.attack_secs > 0.001 {
        Box::new(filtered.fade_in(Duration::from_secs_f32(eff.attack_secs)))
    } else { filtered }
} else { Box::new(source) };
// ...and later: PannedSource::new(effected, pan)   // no conversion
```

Side effect worth knowing: the pitch/low-pass chain **no longer quantizes through `i16` between stages** — one less lossy hop.

### The CI guard (the actual step)

```yaml
- name: One `windows` crate version on the MSVC target
  shell: bash
  run: |
    # `cargo tree -i` exits non-zero when the spec is ambiguous (i.e. exactly the
    # failure we are testing for), so swallow the status and judge by the output.
    versions=$(cargo tree --target x86_64-pc-windows-msvc -i windows 2>&1 \
      | grep -oE '^windows v[0-9.]+|windows@[0-9.]+' | sort -u || true)
    echo "$versions"
    if [ "$(printf '%s\n' "$versions" | grep -c .)" -ne 1 ]; then
      echo "::error::multiple 'windows' crate versions on the MSVC target — DX12 will not compile"
      exit 1
    fi
```

### Candidate-root resolution, worked through (this is the whole design in one table)

For a relative path `assets/hero.png`, candidates are tried in order and the **first one where the file exists** wins:

| Scenario | exe location | candidates (in order) | where `assets/` is found |
|---|---|---|---|
| macOS bundle | `Game.app/Contents/MacOS/game` | `Game.app/Contents/Resources`, `…/Contents/MacOS`, `…/Contents`, `Game.app`, *(then cwd + ancestors)* | **1st** — bundle `Resources` |
| Packaged zip / plain exe | `RustSurvivors/survivor.exe` | `RustSurvivors/`, its ancestors, *(then cwd + ancestors)* | **1st** — beside the exe. **Independent of the launch directory — this is the bug fix.** |
| `cargo run --example x` | `target/debug/examples/x` | `target/debug/examples`, `target/debug`, `target`, **`<repo root>`**, *(then cwd…)* | **4th** — the repo root, via exe ancestors. **This is why zero existing examples needed a change.** |
| `cargo test` (integration) | `target/debug/deps/<test>-<hash>` | `target/debug/deps`, `target/debug`, `target`, **`<repo root>`**, … | **4th** — which is what makes `tests/asset_root.rs` work from a moved cwd |
| `set_asset_root("/opt/content")` | *(irrelevant)* | `/opt/content` **only** | there, or it's a miss (no quiet fallback) |

Actual roots printed by the example when launched from `/`:
```
1. /Users/jkl/Projects/skeleton-engine/target/debug/examples
2. /Users/jkl/Projects/skeleton-engine/target/debug
3. /Users/jkl/Projects/skeleton-engine/target
4. /Users/jkl/Projects/skeleton-engine      <-- hit
5. /
```

### `asset_path` public API

| Item | Purpose |
|---|---|
| `resolve(path) -> PathBuf` | The resolution. Absolute → unchanged. Relative → first existing candidate, else **unchanged**. wasm → identity. |
| `candidate_roots() -> Vec<PathBuf>` | The search order (diagnostic). wasm → empty. |
| `set_asset_root(root)` / `App::set_asset_root` | Pin one explicit root (becomes the only candidate). |
| `asset_root() -> Option<PathBuf>` / `clear_asset_root()` | Read / unpin. |
| `asset_failures() -> Vec<AssetFailure>` / `App::asset_failures` | Everything that failed to load. `AssetFailure { path, error }` — `path` is the caller's string, `error` includes `(searched: …)`. |
| `clear_asset_failures()` | Drop the record (e.g. between scenes). |
| `set_strict_assets(bool)` / `App::set_strict_assets` / `strict_assets()` | Panic at the load instead of falling back. Off by default. |
| `ANCESTOR_DEPTH: usize = 4` | How far up from exe/cwd to walk. |
| `record_failure(path, error)` — `pub(crate)` | Logs `error!`, panics if strict, dedups by path, records. |

### The 12 `asset_path` unit tests (what they pin)

```
a_macos_bundle_resources_dir_is_searched_first
a_plain_executable_yields_no_bundle_candidate
executable_ancestors_reach_the_repo_root_of_a_dev_build      <-- why examples didn't change
executable_candidates_precede_working_directory_candidates   <-- why a stray assets/ can't win
candidates_are_deduplicated_when_exe_dir_and_cwd_overlap
an_explicit_root_is_the_only_candidate
candidates_are_empty_without_an_executable_or_working_directory
an_absolute_path_resolves_to_itself
a_relative_path_that_exists_nowhere_is_returned_unchanged
a_relative_path_resolves_against_a_root_that_is_not_the_working_directory  <-- via resolve_in, NOT the global
the_first_root_that_holds_the_file_wins_not_merely_the_first_root
a_failure_is_recorded_once_per_path_and_can_be_cleared       <-- asserts own path only, never len()
```

---

## Code Analysis

- **`src/asset_path.rs`** — process-global state in three statics: `EXPLICIT_ROOT: RwLock<Option<PathBuf>>`, `FAILURES: Mutex<Vec<AssetFailure>>`, `STRICT: RwLock<bool>`. Globals (not a World resource) because the renderer's texture loader has no access to `App`/`World`, and an asset root is inherently process-wide. **This globality is exactly what bit the tests** — see Risks.
- `pub const ANCESTOR_DEPTH: usize = 4` — a dev exe sits 3 levels below the repo root (`<root>/target/debug/examples/<name>`); 4 leaves headroom.
- `fn candidates_from(explicit: Option<&Path>, exe: Option<&Path>, cwd: Option<&Path>) -> Vec<PathBuf>` — **pure path arithmetic**, no filesystem. An explicit root short-circuits to `vec![root]`. This is what makes the ordering testable without a real layout.
- `fn resolve_in(roots: &[PathBuf], path: &Path) -> PathBuf` — the resolution itself, against an explicit root list. Extracted specifically so tests need not touch the global root. `pub fn resolve(path)` = `resolve_in(&candidate_roots(), path.as_ref())`.
- `pub(crate) fn record_failure(path: &str, error: impl Display)` — appends `(searched: …)`, logs `error!`, **panics if strict**, then **dedups by path** before pushing.
- **wasm**: `resolve` is identity, `candidate_roots()` returns `Vec::new()` (present on both targets so `renderer/texture.rs` needs no `cfg`). The three search helpers + `mod tests` are `cfg(not(target_arch = "wasm32"))`.
- **The 10 read sites** now wrapped in `resolve()`: `renderer/texture.rs` (`try_from_path_with_format`), `asset/image_loading.rs` (`decode_image_with_state`), `audio/playback.rs` (`read_cached_bytes`), `scripting/loading.rs`, `data_table.rs`, `ron_registry.rs`, `animation/clip_set.rs`, `dialogue/tree.rs`, `particle/config_set.rs`, `trigger_zone.rs`, `zone_effect.rs`, `anim_effect.rs`.
- **`src/audio/`**: `AudioManager` now holds `stream: MixerDeviceSink` (was `_stream: OutputStream` + `stream_handle: OutputStreamHandle`) and `sinks: HashMap<String, Player>`. `PannedSource<S: Source>` (was `S: Source<Item = f32>`); its `Iterator::Item = Sample`.
- `const TONE_RATE: SampleRate = match SampleRate::new(48_000) { Some(r) => r, None => unreachable!() };` and `const TONE_CHANNELS: ChannelCount = ChannelCount::MIN;` — the `NonZero` newtypes in const position.

- **`App::load_image` feeds TWO independent subsystems** — this is the single most important thing to understand before attempting **Phase 3**, and it is why a missing file used to be reported twice:
  ```rust
  pub fn load_image(&mut self, path: impl Into<String>) -> Handle<ImageAsset> {
      let path = path.into();
      self.pending_textures.push(path.clone());          // (1) renderer GPU upload queue,
                                                          //     read+decoded in renderer/texture.rs
      self.world.resource_mut::<AssetServer>()...
          .load_image(&path)                              // (2) AssetServer: CPU read+decode NOW
  }                                                       //     (asset/image_loading.rs), returns the Handle
  ```
  Both paths take **the same string**, and both use it as their key — `SpriteRenderer::texture_cache` on side (1), `AssetServer::path_to_id` / `Handle::path()` on side (2). A `Sprite` renders by preferring `image_handle.path()` and falling back to `texture`, so **the two keys must agree**. That agreement is exactly what the 2026-05-29 bug broke, and exactly what this session's "resolve at the read, never at the key" rule protects. **Phase 3 (`load_image_bytes(key, bytes)`) must register the decoded image under `key` on BOTH sides** — seed `AssetServer` *and* prime the renderer's texture cache — without ever sending `key` down a filesystem path. Design it before building it.

- **Component shapes needed to write an example** (discovered the hard way — I guessed wrong twice):
  ```rust
  WindowConfig { width: u32, height: u32, title: String, clear_color: [f64; 4] }  // has Default
  Transform    { position: Vec2, scale: Vec2, rotation: f32, z: f32 }             // no `from_xy`
  Sprite       { texture: Option<Arc<str>>, color: Color, image_handle: Option<Handle<ImageAsset>> }
  Sprite::textured(path) | Sprite::with_handle(h) | Sprite::textured_with_handle(path, Some(h))
  // spawn idiom: let e = app.world.spawn(); app.world.add_component(e, ...);  // NOT spawn((a, b))
  ```

- **The verify gate** (`./scripts/verify.sh`, in order): `cargo fmt --check` → `cargo clippy --all-targets -D warnings` → `cargo build --target wasm32-unknown-unknown` (lib+bins, **not** `--all-targets`) → `cargo clippy --target wasm32 --lib -D warnings` → `cargo test --all-targets` → `cargo test --doc` → `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`. The **wasm clippy** step is the one that caught this session's dead-code trip; a native-only build would have shipped it.

---

## Files Changed

### Source code (engine)
- `src/asset_path.rs` — **NEW.** Asset-root resolution, failure recording, strict mode.
- `src/lib.rs` — `pub mod asset_path;` + `pub use asset_path::AssetFailure;`
- `src/app/assets.rs` — `App::set_asset_root` / `App::set_strict_assets` / `App::asset_failures`.
- `src/renderer/texture.rs` — read via `resolve()`; the magenta-fallback branch now calls `record_failure` (was a bare `warn!`).
- `src/asset/image_loading.rs` — read via `resolve()`; both failure branches route through `record_failure`.
- `src/audio.rs`, `src/audio/types.rs`, `src/audio/source.rs`, `src/audio/playback.rs` — rodio 0.22 migration; `playback.rs` also reads via `resolve()`.
- `src/scripting/loading.rs`, `src/data_table.rs`, `src/ron_registry.rs`, `src/animation/clip_set.rs`, `src/dialogue/tree.rs`, `src/particle/config_set.rs`, `src/trigger_zone.rs`, `src/zone_effect.rs`, `src/anim_effect.rs` — read via `resolve()`.

### Tests
- `tests/asset_root.rs` — **NEW.** The regression test (foreign cwd → relative asset must still load). **Deliberately the ONLY test in that file.**
- `src/asset_path.rs` — 12 unit tests (candidate ordering, dedup, bundle detection, explicit root, resolution, failure recording).

### Examples
- `examples/packaged_assets.rs` — **NEW.** Asset-root + loud-failure demo; self-checking in `HEADLESS_SHOT` mode.

### Config / CI
- `Cargo.toml` — `rodio` 0.19 → 0.22 with `features = ["playback", "wav", "vorbis", "mp3"]`; version 0.124.0 → 0.125.0 → 0.126.0.
- `Cargo.lock` — re-resolved (`windows 0.54` gone; `cpal` 0.17.3).
- `.github/workflows/ci.yml` — new `windows` job (`Build (Windows / DX12)`).
- Branch protection (via `gh api`, not a file) — required checks 5 → 6.

### Docs
- `CLAUDE.md` — new `asset_path` module-map row; header → v1.6.219 / package v0.126.0.
- `docs/CHANGELOG.md` — 0.125.0 and 0.126.0 entries.

### rust-survivors (docs only)
- `docs/ENGINE_CHANGE_REQUESTS.md` — both 2026-07-13 requests moved to **Completed** with what-shipped notes; Open Requests now `_None._`
- `docs/ENGINE_ASSET_LOADING_REQUEST.md` — RESOLVED banner at the top.

---

## User Feedback & Preferences (REQUIRED)

- **"rust-suvivors/docs/ENGINE_CHANGE_REQUESTS.md 에 오늘자 요청사항 확인하고 나에게 알려줘"** — the session's opening ask. Note: the user pointed at a *second repo's* doc, not the usual dungeon-merchant EW board.
- **"유지보수 중단 프로젝트이지만, 다른 사람 공유를 위해 windows 환경에서 exe 릴리즈 빌드 진행하면서, asset이 따라오지 못해 게임 실행시 asset을 불러오지 못하는 문제를 겪으면서 작성한거야. 이에대한 수정 방안 있으면 계획 세워서 알려줘"** — **the key clarification.** rust-survivors is paused *as a project*, but it is still a real source of engine bug reports because the user actually distributes builds from it. Do not dismiss its requests as "deprecated project".
- Chose **"Phase 1 → 2 순차 (추천)"** from a 4-option plan — i.e. accepted the recommendation to skip Phase 3 (byte-sourced images) for now.
- Chose **"추가한다 (추천)"** — approved adding `Build (Windows / DX12)` to branch protection's required checks.
- **"엔진 해결 됌으로 정리하고 /handoff 하고 머지해줘"** — mark the game's requests resolved, write a handoff, merge.
- Chose **"#27 먼저 머지 후 정리"** — when told a *pre-existing* open PR on rust-survivors touched the same doc, the user opted to merge it first rather than stack or skip.
- **Standing (from memory, honoured this session):** merge authority is delegated (squash on green CI, no per-PR re-confirm) — but I still asked before touching **branch protection** and before merging **someone else's PR**, both of which are outside "merge my own work".
- **Standing:** user-facing reports in Korean; code, docs, commit messages, PR bodies in English.

---

## Where We're Going

1. **Land this handoff** as its own `docs(handoff)` PR (`docs/handoff-seq1-asset-root-windows`), per `/land-pr` handoff mode. No version bump. *(Everything else from this session is already merged — engine #358/#359, game #27/#28.)*
2. **After that: the shelf is genuinely empty again.** The `breadth-fov` chain's directive still stands — *read the board first* (`../dungeon-merchant/docs/engine-wishlist.md`, next free ID **EW-007**, currently empty) and **ask** for a direction if it is. Note there are now **two** request channels: dungeon-merchant's EW board and `rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md`.
3. **Candidate follow-ups this session created (none urgent):**
   - **Phase 3** — `App::load_image_bytes(key, bytes)` so `Sprite::textured(key)` resolves an embedded image exactly like a path. Completes Request A's third bullet and enables `include_bytes!` single-file builds for small/jam games. Audio side already done.
   - **Close the vorbis test gap** — nothing in the repo exercises `.ogg` decoding. Would need a small, license-clean fixture.
   - **Hot-reload under an asset root** — `resolve()` was applied at reads only; the notify watchers still register the caller's path (fine for dev-from-repo-root, which is the only hot-reload use case, but worth knowing).

---

## Risks & Blockers

- **`rust-survivors` has auto-merge DISABLED** (`gh pr merge --auto` returns `auto: false`) — unlike the engine repo. Any PR there must be merged by hand after watching its checks. (Bit this session on #28; merged manually.)
- **`asset_path`'s state is process-global**, and that is a live footgun for *tests*, not for games. Two tests already tripped on it in one session (one raced on the shared `FAILURES` list; one mutated the shared root). **Rule for future tests: never assert on the length of `asset_failures()`, and never call `set_asset_root()` in a unit test — use `resolve_in(roots, path)`.** `tests/asset_root.rs` may move the cwd only because it is the sole test in its own integration binary (each integration-test *file* is a separate process); **do not add a second test to that file.**
- **`;`-chaining a gate with a commit will push red code.** `CLAUDE.md` warns against *piping* an exit code; chaining hides it identically. Capture `VERIFY_EXIT=$?` and **branch on it** before committing, or run verify as its own tool call. (Bit this session; recorded in memory's live gotchas.)
- **Vorbis/`.ogg` decoding is untested** anywhere in the engine. The rodio 0.22 codec swap to symphonia was verified for **mp3 and wav only**. If a game reports broken `.ogg`, start there.
- The **two audio tests that need a real device** (`audio_system_drives_fade_out_to_stop_when_device_exists`, `audio_manager_clear_file_cache_when_device_exists`) passed here and on CI, but remain the usual suspects on a no-audio box.

## Open Questions

- **Is `Sprite::textured(key)` the right shape for Phase 3?** Byte-sourced images need a key that the renderer's texture cache resolves like a path but never reads from disk. The cleanest seam is probably registering the decoded image under `key` in `AssetServer` *and* seeding `SpriteRenderer::texture_cache` — but that touches the exact identity/cache-key machinery this session was careful not to disturb. Worth designing before building.
- Should `rust-survivors`'s request doc stay the second channel, or should future game-driven requests be funnelled into dungeon-merchant's EW board for one queue? (Only matters if the user files more from that repo.)

---

## Quick Start for Next Session

```bash
# Nothing is left dangling — engine #358/#359 and rust-survivors #27/#28 are all merged.
# Only this handoff itself needs landing.

# 1. Engine state
cd ~/Projects/skeleton-engine
git log --oneline -3     # expect de95bbb (v0.126.0) at or near the tip
git status -s            # expect clean

# 2. Read first
#   src/asset_path.rs                 — the new module (design + the global-state caveats)
#   tests/asset_root.rs               — the regression test (and why it's alone in its file)
#   examples/packaged_assets.rs       — the acceptance-test example
#   CLAUDE.md                         — module map row for asset_path; the verify-gate rules
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md  — the second request channel

# 3. Verify current state (read the exit code — do NOT pipe or `;`-chain it)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# 4. Re-prove the headline fix if you want to see it
cargo build --example packaged_assets
(cd / && HEADLESS_SHOT=/tmp/pa.png "$OLDPWD/target/debug/examples/packaged_assets")
# expect: OK: 'examples/assets/hex_tiles.png' resolved; exit 0

# 5. Next action
#   Land this handoff as its own docs(handoff) PR, then:
#   READ THE BOARD FIRST — ../dungeon-merchant/docs/engine-wishlist.md (EW-007, currently EMPTY).
#   If empty, ASK the user for a direction. The self-pick shelf is exhausted.
#   Standing candidates created this session: Phase 3 (App::load_image_bytes), the vorbis test gap.
```
