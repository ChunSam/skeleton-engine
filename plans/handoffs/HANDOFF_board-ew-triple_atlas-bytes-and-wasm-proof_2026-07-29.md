# Shipped: byte-source atlas parity, then the three follow-ups that verifying it uncovered (v0.135.0 → v0.135.2)

**Date:** 2026-07-29
**Status:** COMPLETED (PRs #376, #377, #378, #379 all merged; `main @ af4573a`, v0.135.2, clean + green)
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `board-ew-triple` seq `2`
**Parent:** `HANDOFF_board-ew-triple_three-board-requests_2026-07-26.md`
**Prior chain:** `HANDOFF_board-ew-triple_three-board-requests_2026-07-26.md` > this

---

## Since Last Handoff

Framed against the parent (seq 1) and its plan, `PLAN_board-ew-triple_three-board-requests_2026-07-26.md`:

- **The parent's central prediction held, and its Phase 1 worked exactly as designed.** It said the board would most likely be empty and the session MUST ask rather than self-pick. The board *was* empty; the session asked; the user chose `load_atlas_bytes` from the four-option menu. No code was written before that answer.
- **The parent's biggest open question got a definitive answer — and it was "no".** "Does the game actually need windowed capture?" The game replied on the board 2026-07-27: *"Headless-only capture is fine for us; no need for the windowed follow-up."* **Phase 2 of the parent plan is therefore dead**, not deferred. The `COPY_SRC` surface change stays unmade.
- **All three shipped requests came back `Verified`, none with a problem report.** Phase 4 (game-report triage) never triggered. EW-009/010/011 are `- [x]` and archived; next free ID is **EW-012**.
- **Phase 3 (`load_atlas_bytes`) shipped as specified** — the parent plan's file list, identity invariant and test list were all accurate. Nothing in it had to be re-planned.
- **The parent's CLAUDE.md risk materialized within the same session.** It flagged "CLAUDE.md is ~208 lines, over the 200-line soft cap… both Phase 2 and Phase 3 add module-map detail." Phase 3 did exactly that; #379 fixed it.
- **A risk the parent did NOT anticipate cost the most time: verification tooling lying about its own results.** Two separate traps (a stale `.exit` file, `core.fileMode = false`) each produced a confident-but-false green. Both are now written down.
- **The parent's "background task notification lies about exit codes" warning held** — and a *new variant* of it bit anyway (stale file, not wrong command).
- **CI runner backlog did NOT recur.** All four PRs went 6/6 green in 4–6 minutes for `Test (native)`, versus ~20 min stalls on two of three PRs last session.

## Reference Documents

- `CLAUDE.md` — project conventions, module map, verify gate. Edited this session (atlas row + the Verification rewrite).
- **`docs/VERIFICATION.md` — NEW this session.** The verify gate's six exit-code traps and three blind spots. Read it before trusting any gate result.
- `docs/CHANGELOG.md` — 0.135.0 / 0.135.1 / 0.135.2 entries are the migration notes.
- `docs/VISION.md` — "a feature is not done until a playable example exercises it", which drove both examples.
- `../dungeon-merchant/docs/engine-wishlist.md` — the shared board (empty; all EWs archived).

---

## The Goal

Execute the parent plan: run the board gate, and if the board is empty, get an explicit direction from the user rather than self-picking. The board was empty and the user chose **`load_atlas_bytes`** — byte-source texture atlas parity, the last gap in the `include_bytes!` single-file/jam story that `load_image_bytes` started. That shipped as v0.135.0.

The session then continued into three follow-ups that the work itself surfaced, each chosen by the user from a presented list: making the wasm claim in #376 actually *true* rather than compile-checked (v0.135.1), fixing the executable bit across every bundled shell script after discovering `core.fileMode = false` had hidden it (v0.135.2), and bringing CLAUDE.md back under its own 200-line rule (docs-only).

The through-line of the last three PRs is the same: **each one exists because verifying the previous one revealed that the verification was weaker than it looked.**

## Where We Are

- **`main @ af4573a`, package v0.135.2, CLAUDE.md header v1.6.232, clean tree, all gates green.** Local `main` == `origin/main`.
- **Four PRs merged, serially, each with its own bump:** **#376 `aa47d7e` v0.135.0** (MINOR), **#377 `660a9e5` v0.135.1** (PATCH), **#378 `d717e9d` v0.135.2** (PATCH), **#379 `af4573a`** (docs-only, no package bump).
- **Lib tests 1294 → 1297 (+3)**, all in `src/asset/tests.rs`. All passed on their first run.
- **The board is empty on both channels.** `dungeon-merchant` has zero active requests (next free **EW-012**); `rust-survivors` is `_None._`. Re-checked mid-session after the first merge — still empty.
- **EW-009/010/011 are all `Verified` and archived** by the game on 2026-07-27, with no problem reports.
- **`load_atlas_bytes` (v0.135.0)** — `App::load_atlas_bytes(key, bytes, cols, rows)` (`src/app/assets.rs`) + `AssetServer::load_atlas_bytes` (`src/asset/atlas_loading.rs`). Purely additive; no existing type or signature changed.
- **The identity invariant is the whole point and it is test-pinned.** `key` is verbatim on three sides: atlas cache key (`atlas_path_to_id`), `Handle::path()`, and the renderer's texture key — the last because the underlying image is registered through `load_image_bytes` under the *same* string. `load_atlas_bytes_keys_the_atlas_image_and_render_key_identically` asserts `image_assets_for_gpu()` (what actually feeds the renderer's cache) carries that key.
- **Grid maths are shared, not duplicated** — `uv_rect` is the same code for both sources, A/B-tested against `load_atlas` on a real temp file across all 12 tiles.
- **`examples/embedded_atlas/embedded_atlas.rs`** renders all 12 tiles of an embedded 4×3, 64px-cell sheet (`examples/assets/blend_locomotion.png`, 256×192 — the sheet `blend_locomotion` loads *by path*), each labelled with its atlas index, plus a deliberately corrupt embed in an `asset_failures()` panel.
- **The example ships to the web** (`examples/embedded_atlas/web/`) with a `#[wasm_bindgen]` entry point beside a shared `build_app()`, and moved into its own directory with a `[[example]]` entry — the layout every web-shipped example uses.
- **`scripts/embedded_atlas_smoke.sh` asserts BOTH halves of the feature's claim** — no image file is served beside the page, *and* the frame is non-blank. Either alone is weak; together they mean the atlas rendered and cannot have come from a file.
- **`examples/embedded_image.rs` had been unbuildable for wasm since it was added** (E0599: called native-only `save_screenshot_headless` unconditionally). Fixed in #377 by the same `cfg`-gated-acceptance-test shape.
- **All 31 bundled shell scripts are now `100755` in git** (26 were `100644`). Three smokes — `centered_text_smoke.sh`, `game_feel_web_smoke.sh`, `hdr_web_smoke.sh` — plus my own new `embedded_atlas_smoke.sh` died at "Permission denied" before doing any work.
- **`CLAUDE.md` is 194 lines** (was 207), back under its own ≤200 rule, with 6 lines of headroom. The Verification section's rationale moved to **`docs/VERIFICATION.md`** (164 lines).
- **`engine-current-state` memory was trimmed** from 76 KB (over the 25k-token Read/Edit cap — it could not be read) to 21 KB by archiving tip-line seqs 184→173. It is now 25 KB after three seq bumps.
- **Memory advanced seq 195 → 198** with a new `core.fileMode` gotcha and a stale-`.exit` gotcha in the live-gotchas line.

## What We Tried (Chronological)

### Chunk 1 — Board gate and the first false green (early)

1. **Read the parent plan and handoff, created a task per phase.** The plan's Phase 1 was explicitly "ends in a QUESTION, not code", which is what shaped the whole opening.
2. **Ran the board gate on both channels before anything else.** `../dungeon-merchant/docs/engine-wishlist.md` → Active requests **empty**, all of EW-009/010/011 moved to Done/archive with `Verified` on 2026-07-27. `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` → `_None._`.
3. **Read the game's verification comments in full rather than just the status field.** That is where the windowed-capture refusal lives, and it is the single most decision-relevant fact of the session. It also revealed the game had built `scripts/verify/` on EW-011 and used *that* to verify EW-009 — one board request verifying another.
4. **Kicked `./scripts/verify.sh` off in the background, then hit the session's first trap.** I wrote an `until [ -f /tmp/verify_start.exit ]; do sleep 5; done` waiter. It returned **instantly** with `VERIFY_EXIT=0`. That was a **stale file from 7월 26 — the previous session's leftover.** Caught it because the log's tail showed `cargo test` still mid-run and only 15 `test result: ok` groups where ~149 were expected.
5. **Diagnosed it properly instead of re-running blind:** `ls -la` showed the `.exit` file dated 3 days earlier while the `.log` was seconds old, and `pgrep -fl` showed `verify.sh` (pid 17557) and `cargo test --all-targets` (pid 17909) alive. Re-waited on the **PID** (`while kill -0 17557`), then read the real code: **exit 0, 149 groups, `all checks passed ✓`**.
6. **Re-proved the parent's three features:** `text_measure` 11 / `data_table` 16 / `input_script` 13, total 1294. Both examples exited 0 with the expected output strings.
7. **Asked via `AskUserQuestion`, in Korean, with the parent plan's four-option menu** — and deliberately demoted option (1) windowed capture, because the game had just declined it. Recommended `load_atlas_bytes` as the smallest coherent gap. **User picked `load_atlas_bytes`.**

### Chunk 2 — `load_atlas_bytes` (#376, v0.135.0)

8. **Read the exact template before writing anything.** `AssetServer::load_image_bytes` (`src/asset/image_loading.rs:87`) and `AssetServer::load_atlas` (`src/asset/atlas_loading.rs:11`), plus the `AssetServer` fields (`path_to_id`, `atlases`, `atlas_path_to_id`, all `HashMap<Arc<str>, AssetId>`). The plan said to read the CLAUDE.md identity row before writing a line; the code confirmed why.
9. **Wrote the identity test first**, per the plan's mitigation for "re-breaking the texture-cache identity invariant (silent at compile time)".
10. **Implemented `load_atlas_bytes` by delegating the image to `load_image_bytes(&*key, …)`** — the same verbatim string — so `atlas.texture_path()` and the renderer's upload key are one string by construction rather than by convention.
11. **Removed a redundant `#[cfg(test)]`** on the `png_bytes` test helper: `mod tests` is already `#[cfg(test)]` at `src/asset.rs:316`.
12. **3 tests passed on the first run.** Included an A/B against a real temp-file `load_atlas` across all 12 tiles, which also asserts the two atlases' texture paths *differ* (one canonicalized, one verbatim) — so the test fails if the byte path ever starts borrowing the path form's identity.
13. **Wrote `examples/embedded_atlas.rs`** using `blend_locomotion.png` (documented 4×3, 64px cells by `examples/gen_blend_sheet.rs`), with a `cfg`-gated headless acceptance test so the example would build for wasm — deliberately *better* than `embedded_image`, which I had just confirmed does not.
14. **The first headless capture caught a real layout bug.** Exit code was 0 and the self-check passed, but the PNG showed only 4 of 12 tiles, clipped at the top-left. **Cause: I laid the grid out centered on the world origin, but `Camera::default()` is `position: Vec2::ZERO, zoom: 1.0`, and `world_to_screen(w) = (w - position) * zoom` — the identity.** World units are top-left-origin screen pixels. Fixed with an absolute `GRID_ORIGIN: Vec2 = Vec2::new(60.0, 110.0)`.
15. **Second capture correct** — 12 tiles, three rows (blue idle / green walk / orange run), labelled 0–11, corrupt embed in the failures panel.
16. **Extended the existing `TextureAtlas` module-map row rather than adding a new one**, per the plan's explicit CLAUDE.md mitigation.
17. **verify exit 0** (150 `ok` groups, 1297 lib tests) → `/ship` 0.135.0 → **re-verify exit 0** → PR #376 → `--auto --squash` → 6/6 green → merged `aa47d7e`.
18. **Hit the memory-file cap while bumping seq 195.** `engine-current-state.md` was 76 KB / ~33k tokens — `Read` refused it outright, even with `offset`/`limit`. Bumped via an asserted single-occurrence Python `str.replace` (the file's own documented method), then **trimmed tip-line seqs 184→173 into `engine-history-archive`**: 75,452 → 21,042 chars.

18b. **Chose `blend_locomotion.png` as the embedded sheet on evidence, not convenience.**
    `examples/gen_blend_sheet.rs` documents it as "a 4×3 grid (64px cells)" and two examples
    (`blend_locomotion`, `sm_crossfade`) already load it **by path** — so the new example is a
    direct byte-source counterpart to a known-good path load, and its three colour-coded rows
    (blue idle / green walk / orange run) make a row/column mix-up visible at a glance.
18c. **Flat examples needed no `Cargo.toml` entry** at this point — verified rather than assumed
    (`grep -n embedded_image Cargo.toml` → nothing; flat `examples/*.rs` are auto-discovered).
    This was *reversed* one PR later when the web harness forced the nested layout.

### Chunk 3 — Making the wasm claim true (#377, v0.135.1)

19. **User asked for a work list.** Presented four categories (A: session-surfaced gaps, B: doc hygiene, C: engine breadth, D: parent's open questions), recommended A1+A2 as one coherent change. **User picked A1+A2.**
20. **A1 was small:** moved `embedded_image`'s headless block into a `#[cfg(not(target_arch = "wasm32"))] fn run_acceptance_test`, dropping `AssetServer` from the top-level import so wasm has no unused-import warning. Both targets build; native test still exits 0.
21. **A2 required a layout change I did not anticipate.** Web-shipped examples live in their own directory with an explicit `[[example]]` entry (`examples/centered_text/centered_text.rs`, not `examples/centered_text.rs` — I initially grepped the wrong path and got "No such file"). So `git mv` into `examples/embedded_atlas/`, add the Cargo.toml entry, and fix `include_bytes!` — switched to the `concat!(env!("CARGO_MANIFEST_DIR"), …)` form used by `centered_text`, which survives the move.
22. **Wasm build emitted `unused_mut`** on `let (mut app, _sheet)` in the wasm entry — `App::run` consumes `self` on wasm. Dropped the `mut`; the native `main` still needs it for `run_acceptance_test(&mut app, …)`.
23. **Confirmed `pkg/` is gitignored** before building a 13 MB wasm bundle into the repo; only `build.sh` + `index.html` are tracked, matching `centered_text`.
24. **The browser check showed a BLANK CANVAS** — module loaded, button read "Running", nothing drawn.
25. **Did not assume it was my bug.** Checked console (empty), network (empty — both trackers arm on first call and need a reload), then used `javascript_tool`: **WebGPU adapter OK, WebGL2 true, and the canvas had been resized 960×540 → 1920×1080**, which is the engine's own DPR path in `src/app/window.rs:755`. So the engine was alive and had initialized.
26. **Ran the decisive A/B: loaded the already-shipped `centered_text` web demo the same way — also blank.** That moved the fault off my example and onto the environment.
27. **Stopped ad-hoc browser poking and used the project's own tool.** `scripts/centered_text_smoke.sh` exists precisely to answer "does the web render path work". Running it revealed **two** things: the repo's scripts are not executable (`permission denied: scripts/…`, worked around with `bash`), and then **exit 1 — `build.sh: Permission denied` at line 69**.
28. **`chmod +x` on `centered_text/web/build.sh` → smoke PASS, 147,821-byte screenshot.** Baseline established: the headless web render path works on this machine. The blank interactive canvas is an artifact of the automation tab.
29. **Wrote `scripts/embedded_atlas_smoke.sh` with a dual assertion**, deliberately stronger than the byte-size check it was modeled on: it also asserts no `*.png|jpg|jpeg|gif|webp` exists under `web/`, *before* rendering, so a stray asset can never be masked by a frame that happened to draw.
30. **Smoke PASS (102,027 bytes) and the screenshot eyeballed** — 12 tiles, identical to native, with `working dir: <unknown>` (wasm has no `current_dir`), which quietly reinforces the point.
31. **Reverted the `centered_text` chmod out of PR A** to keep it one coherent change, and flagged the permission bug in the PR body as a follow-up.
32. **PATCH bump (0.135.1), not MINOR** — no library code changed at all, only examples and tooling.

### Chunk 4a — The executable bit, and a bug I had just shipped (#378, v0.135.2)

33. **`chmod +x` on all the `build.sh` files produced an EMPTY `git status`.** That is the tell: **`core.fileMode = false`** in this repo — git ignores filesystem mode changes entirely.
34. **`git ls-files -s` told the real story, and it was worse than the local view.** `examples/games/coin_race/web/build.sh` is `100644` **in git** while showing `755` on disk. So my earlier observation that "6 of 14 are already executable" was a **local-filesystem artifact**; in the repository all 14 were 644.
35. **Including one I had shipped an hour earlier.** `examples/embedded_atlas/web/build.sh` went into #377 as **644** — because my `chmod +x` was invisible to git. The smoke I had just added was broken for anyone cloning the repo. This is exactly why the fix was widened rather than limited to three files.
36. **Corrected my own overstatement.** I had told the user four smokes were broken. Re-checking the actual invocations: `bloom_web_smoke.sh` and `render_format_query_smoke.sh` use `bash "$WEB_DIR/build.sh"` and are unaffected; only `centered_text`, `game_feel` and `hdr_web` exec it directly. Reproduced all three failures rather than asserting them.
37. **Full scope measured: 26 of 31 shell scripts were `100644`.** Only `verify.sh`, `wasm_audio_smoke.sh`, `wasm_save_smoke.sh` and two `build.sh` had the bit. Every one of them documents its usage as `scripts/foo.sh …`, which needs it.
38. **`git update-index --chmod=+x` fixed the index — and direct execution still failed.** It changes what is committed, not the working tree. Had to `chmod` the working tree too before the local check meant anything. Nearly reported "fixed" off the index change alone.
39. **Missed `examples/wasm/build.sh`** (it is not under a `web/` directory, so my glob skipped it) — caught by the fresh-clone check reporting exactly **1** remaining non-executable `.sh`. Added it and corrected the CHANGELOG counts 30→31 and 25→26 by amend.
40. **Verified by fresh clone**, which is the condition that was actually broken: cloned the branch into a temp dir, confirmed `100755` on disk and that `./scripts/centered_text_smoke.sh` starts building.

### Chunk 4b — CLAUDE.md back under its cap (#379, docs-only)

41. **Measured before acting, and the measurement killed my own earlier proposal.** I had told the user the fix was "move module-map detail into `docs/*.md`". Section line counts: Module map **80**, Verification **47**, Project direction 16, Core patterns 15, Documentation rules 14, Document map 10, header 9. **The module map is a table — one line per row regardless of row length.** A 2,000-character row is still one line, so compressing row *text* buys exactly zero lines. Only prose sections can shrink.
42. **Extracted the Verification section's rationale to `docs/VERIFICATION.md`** — six exit-code traps and three blind spots — keeping inline what an agent needs every session: the seven-command gate, the exit-code rule, and one-line blind-spot statements.
43. **Seeded the new doc with this session's two undocumented traps** (stale `.exit`, `core.fileMode`) and with "compiling for wasm is not running on wasm", which is precisely what made the v0.135.0 web claim weaker than it read.
44. **Compressed `Documentation rules` and `Project direction` prose** without dropping a rule, and recorded the measurement itself: *"The module map is the growth driver — extend an existing row before adding one."*
45. **207 → 194 lines.** Caught a dangling reference on the way: the gate's `# lib+bins — see wasm gotcha` comment pointed at a bullet that no longer existed; retargeted to `# lib+bins only — NOT --all-targets`. Verified all 10 referenced `docs/*.md` paths resolve.
46. **Docs-only, so no package bump** — just the CLAUDE.md doc-version `v1.6.231` → `v1.6.232`, per the `/ship` rule.

### Chunk 5 — Bookkeeping that recurred every PR

47. **Memory was bumped after every merge**, not batched: seq 195 (#376), seq 197 (#378), seq 198
    (#379). Each used an **asserted single-occurrence Python `str.replace`** — the method the
    memory file itself prescribes — so a silently-missed or double-applied edit is impossible.
    Every run printed its before/after char count.
48. **The `MEMORY.md` index hook was refreshed each time** to keep `main @ <hash>` / version /
    header pointers truthful; a stale hook is worse than none because it reads as current.
49. **The board needed no update.** Nothing this session served a board request — `load_atlas_bytes`
    came from the empty-board menu — so there was no `[Engine]` thread comment to write. Resisted
    the pull to post a "notice" the game has no use for (they render exclusively through
    `UiQueue`/`TextQueue` with zero `Sprite`/`Transform` entities, per their own 2026-07-03 note,
    so an atlas API is not applicable to them).
50. **Task list kept in sync with reality** — Phase 2 (windowed capture) and Phase 4 (game-report
    triage) were *deleted*, not left pending, once the board made them moot.

## Key Decisions

- **Demote windowed capture in the direction menu because the game declined it.** The parent plan listed it first and said "recommend (1) only if the game has asked for it". The game explicitly said the opposite, so the `COPY_SRC` surface change — which touches every app on every backend and is disallowed on WebGL2 — stays unmade. This is a closed question, not a deferred one.
- **Delegate the atlas's image to `load_image_bytes` under the same key, rather than duplicating its logic.** This makes the three-way identity structural: there is no second place where a key could be canonicalized differently. Same reasoning the parent used for `shape_text`/`font_blobs` in EW-009.
- **Write the identity test before the implementation.** The failure mode (white sprites) is silent at compile time and only visible at render; the plan named it as the top risk. `image_assets_for_gpu()` — the function that actually populates the renderer cache — is what the test asserts on, not a proxy.
- **A/B the grid maths against a real temp-file `load_atlas` rather than against hand-written `UvRect::from_grid` values.** Hand-written expectations would pass even if both paths drifted together; the A/B fails if the byte path grows its own maths.
- **Make the new example wasm-buildable even though its sibling is not.** `cfg`-gating the headless block costs ~6 lines and makes the feature's central claim — single-file wasm builds — demonstrable rather than asserted. This decision is what later exposed that `embedded_image` had been broken all along.
- **Trust the headless smoke over the interactive browser tab.** When the automation tab showed a blank canvas, the tempting conclusion was "my example is broken". The A/B against the already-shipped `centered_text` demo (also blank there, passes headless) showed the tab is the unreliable instrument. Verification rests on the smoke.
- **Give the new smoke a structural assertion, not just a byte-size check.** A non-blank frame alone could come from a fetch; an empty directory alone proves nothing if nothing drew. Asserting both — and checking the directory *before* rendering — is what makes "the sheet came from the binary" actually verified.
- **Widen the executable-bit fix from 3 files to 31.** Fixing only the three smokes I had reproduced would have left my own newly-shipped `embedded_atlas_smoke.sh` broken, plus every other script whose documented usage does not work. Same defect, one uniform fix.
- **Split the exec-bit fix into its own PR.** It touches `bloom`, `game_feel`, `hdr_render_target` and a dozen unrelated examples — genuinely independent of the atlas work. Landed serially to avoid the release-paperwork conflict the parent handoff documented.
- **PATCH for #377 and #378, docs-only for #379.** No library code changed in any of them; the pre-1.0 rule reserves MINOR for a release with an actual API change.
- **Do not restructure the module map unilaterally.** Moving it out of CLAUDE.md would buy ~80 lines of headroom but change how every future session navigates the codebase. Flagged to the user as a decision rather than made.

## Evidence & Data

### Shipped: PR → version → commit → diffstat

| PR | Version | Bump | Commit | Files | +/− | Merged (UTC) |
|---|---|---|---|---|---|---|
| #376 · `load_atlas_bytes` | **0.135.0** | MINOR | `aa47d7e` | 8 | +518 / −4 | 2026-07-29T01:45:31Z |
| #377 · web harness + `embedded_image` fix | **0.135.1** | PATCH | `660a9e5` | 9 | +384 / −62 | 2026-07-29T02:26:21Z |
| #378 · executable bit | **0.135.2** | PATCH | `d717e9d` | 30 | +11 / −3 | 2026-07-29T02:42:36Z |
| #379 · CLAUDE.md + `docs/VERIFICATION.md` | — | docs-only | `af4573a` | 2 | +164 / −41 | 2026-07-29T03:35:56Z |

### CI behaviour (the parent's runner-backlog risk did NOT recur)

| PR | `Test (native)` | Other 5 checks | Result |
|---|---|---|---|
| #376 | 5m17s | WASM 45s · Windows 1m44s · Package 1m16s · Render 1m27s · Rustdoc 42s | 6/6 green |
| #377 | 5m57s | WASM 40s · Windows 2m1s · Package 1m14s · Render 1m20s · Rustdoc 54s | 6/6 green |
| #378 | 5m45s | WASM 47s · Windows 1m46s · Package 1m16s · Render 1m1s · Rustdoc 37s | 6/6 green |
| #379 | 4m11s | WASM 39s · Windows 1m24s · Package 1m5s · Render 1m2s · Rustdoc 50s | 6/6 green |

Last session two of three PRs sat `pending 0s` for ~20 min. This session: none. Do not treat the backlog as a standing condition.

### Verify-gate history (every run, real exit codes read non-piped)

| # | Tree | Exit | Note |
|---|---|---|---|
| 0 | session start, clean `main` | **0** | 149 `ok` groups — but see the stale-file trap below |
| 1 | `load_atlas_bytes` | **0** | 150 `ok` groups, 1297 lib tests |
| 2 | + 0.135.0 bump | **0** | lock + doc re-checked |
| 3 | A1+A2 | **0** | — |
| 4 | + 0.135.2 exec bit | **0** | — |
| 5 | + amend (`examples/wasm/build.sh`) | **0** | — |
| 6 | CLAUDE.md docs change | **0** | — |

**Zero red runs this session** — unusual; last session had three, each at a different step.

### The stale-`.exit` trap, in numbers

| Observation | Value |
|---|---|
| `/tmp/verify_start.exit` mtime | **7월 26 13:19** (3 days old — previous session) |
| `/tmp/verify_start.log` mtime | 7월 29 10:12 (seconds old) |
| What the waiter reported | `VERIFY_EXIT=0` — **instantly** |
| What was actually running | pid 17557 `verify.sh`, pid 17909 `cargo test --all-targets` |
| `test result: ok` groups at that moment | **15** (of an eventual 149) |
| Real result, after waiting on the PID | exit **0**, 149 groups |

The false green happened to match the true result — which is exactly what makes it dangerous.

### The executable-bit problem, as recorded in git vs on disk

| | On disk (`ls -l`) | In git (`git ls-files -s`) |
|---|---|---|
| `examples/games/coin_race/web/build.sh` | `755` | **`100644`** |
| `examples/embedded_atlas/web/build.sh` (shipped in #377) | `755` | **`100644`** |
| `build.sh` files appearing executable | 6 of 14 | **0 of 14** |
| All `.sh` files executable | — | 5 of 31 → **31 of 31** |

`core.fileMode = false` is why the two columns disagree. **`ls -l` is not evidence in this repo.**

### Which smokes were actually broken (my first count was wrong)

| Script | Invocation | Broken? |
|---|---|---|
| `centered_text_smoke.sh` | `"$WEB_DIR/build.sh"` | **yes** |
| `game_feel_web_smoke.sh` | `"$WEB_DIR/build.sh"` | **yes** |
| `hdr_web_smoke.sh` | `"$WEB_DIR/build.sh"` | **yes** |
| `embedded_atlas_smoke.sh` (new, #377) | `"$WEB_DIR/build.sh"` | **yes** |
| `bloom_web_smoke.sh` | `bash "$WEB_DIR/build.sh"` | no |
| `render_format_query_smoke.sh` | `bash "$WEB_DIR/build.sh"` | no |

Plus all 26 scripts whose own docs say `scripts/foo.sh …`, which fails without the bit regardless of build.sh.

### CLAUDE.md line budget — before and after

| Section | Before | After |
|---|---|---|
| header | 9 | 9 |
| Conversation language | 8 | 8 |
| **Verification** | **47** | **36** |
| Project direction | 16 | 14 |
| **Module map** | **80** | **80** |
| Core patterns & task recipes | 15 | 15 |
| Agent working notes | 7 | 7 |
| Documentation rules | 14 | 13 |
| Document map | 10 | 11 |
| **TOTAL** | **207** | **194** |

**The module map did not move — and could not.** It is 80 lines because it has ~72 rows, not because the rows are long.

### Test counts

| Point | Lib tests | Added |
|---|---|---|
| Session start (v0.134.0) | 1294 | — |
| After `load_atlas_bytes` (v0.135.0) | **1297** | +3 `src/asset/tests.rs` |
| v0.135.1, v0.135.2, #379 | 1297 | none (examples/tooling/docs only) |

### Render smoke results

| Smoke | Screenshot | Verdict |
|---|---|---|
| `centered_text_smoke.sh` (baseline, after chmod) | 147,821 bytes | PASS — environment is fine |
| `embedded_atlas_smoke.sh` (new) | 102,027 bytes | PASS — plus "no image served" assertion |

### The example bug the first capture caught (again — 2nd session running)

| | Value |
|---|---|
| Symptom | 4 of 12 tiles visible, clipped at the top-left corner |
| Cause | Grid centered on the world origin |
| Why that fails | `Camera::default()` = `position: ZERO, zoom: 1.0`; `world_to_screen(w) = (w − position) * zoom` → **the identity**. World units are top-left-origin screen pixels. |
| Fix | `GRID_ORIGIN: Vec2 = Vec2::new(60.0, 110.0)`, absolute |
| Precedent | Last session's first `scripted_capture` shot caught an unlayered-`DrawText` z-order bug |

### The three new tests (`src/asset/tests.rs`)

| Test | What fails it |
|---|---|
| `load_atlas_bytes_keys_the_atlas_image_and_render_key_identically` | any divergence between handle path, `texture_path()`, and `image_assets_for_gpu()`'s key |
| `load_atlas_bytes_tiles_the_grid_exactly_like_a_path_loaded_atlas` | the byte path growing its own grid maths (A/B over all 12 tiles vs a real temp file) |
| `load_atlas_bytes_reports_a_corrupt_embed_as_a_failure` | a bad `include_bytes!` being swallowed instead of reaching `asset_failures()` |

### Public API added (the whole surface)

| Item | Signature |
|---|---|
| `App::load_atlas_bytes` | `(&mut self, key: impl Into<String>, bytes: &[u8], cols: u32, rows: u32) -> Handle<TextureAtlas>` |
| `AssetServer::load_atlas_bytes` | same shape; the delegate that owns the identity |

Nothing else. No existing type or signature changed in any of the four PRs.

### The game's verification of the parent's three requests (from the board, 2026-07-27)

| Request | How the game verified it |
|---|---|
| EW-009 (text measurement) | Measured the **captured pixels**: 사슬갑옷/200g → panel 74px (name ink 52, value ink 30, padding 11/11). Every case equals `max(name_ink, value_ink) + 22` **to the pixel**. "200g" measures 30px where the `chars × 15` heuristic assumed 60. |
| EW-010 (DataTable union) | Own regression test `an_optional_column_on_a_later_row_survives_parsing`; deleted header comments in **13 RON files**, 2 in-code comments, a CLAUDE.md 함정 rule and an AGENTS.md rule. |
| EW-011 (headless capture) | Built `scripts/verify/`: `screens.ron` drives Title → City → 집 → 상점 → Pack → Fight and captures all six in ONE pass; `tooltip.ron` captures four hover tooltips — **and that is what verified EW-009.** Retired the entire osascript/`screencapture -R`/CGEvent cookbook. |

Game's one FYI (not an engine bug): a capture run is a **real session**, so it reads and autosaves the save slot; their runner stashes the slot aside and restores it, which also makes every capture start from a deterministic new game.

### The direction menu as presented (Phase 1's output — the decision point)

The parent plan specified a four-option menu. What was actually presented, with the parent's
recommendation logic applied to the new board facts:

| # | Option | Framing given |
|---|---|---|
| 1 | **`load_atlas_bytes`** ← **chosen** | "smallest coherent gap"; purely additive; exact template exists (`load_image_bytes`) incl. the identity invariant |
| 2 | Windowed frame capture | **demoted** — parent said "recommend only if the game has asked"; the game had just declined it in writing |
| 3 | 4th procgen mode / `MapGenerator` trait | optional breadth; trait is an explicit anti-goal, trigger not fired |
| 4 | Audio-reactive hooks / 2nd capstone | fresh areas; need design discussion first |

### New-file line counts

| File | Lines |
|---|---|
| `docs/VERIFICATION.md` | 164 |
| `scripts/embedded_atlas_smoke.sh` | ~150 |
| `examples/embedded_atlas/embedded_atlas.rs` | ~290 |
| `examples/embedded_atlas/web/index.html` | 70 |
| `examples/embedded_atlas/web/build.sh` | 33 |

### wasm bundle (not committed — `pkg/` is gitignored)

| Artifact | Size |
|---|---|
| `embedded_atlas_bg.wasm` | 13 MB (dev-profile release build) |
| `embedded_atlas.js` | 146 KB |
| `wasm-bindgen` CLI vs `Cargo.lock` | **0.2.122 == 0.2.122** (must match, or bindings fail at runtime) |

### CHANGELOG section headings used (house format)

| Version | Sections |
|---|---|
| 0.135.0 | `### Added` |
| 0.135.1 | `### Fixed`, `### Added` |
| 0.135.2 | `### Fixed` |

Each opens with a bold one-line summary plus 2–5 sentences of context, and states "Purely
additive: no existing type or signature changed" / "no library code changed" where true.

### Reusable procedure — verifying a web-example claim

This session's mechanics, worth repeating whenever an example claims web support:

1. `cargo build --example <name> --target wasm32-unknown-unknown` — necessary, **not sufficient**.
   The verify gate never does this (lib+bins only).
2. Scaffold `examples/<name>/web/{build.sh,index.html}` from the nearest sibling; move the
   example into `examples/<name>/` with a `[[example]]` entry; switch `include_bytes!` to the
   `concat!(env!("CARGO_MANIFEST_DIR"), …)` form so it survives the move.
3. Add a `#[wasm_bindgen]` entry beside a shared `build_app()` so native and web run the same code.
4. Run `build.sh` — confirms the bundle generates and the CLI/crate versions agree.
5. **Write a smoke script and trust it over any interactive browser.** Model it on
   `scripts/centered_text_smoke.sh`; add a *structural* assertion specific to the claim where one
   exists (for an embedded asset: no image file in the served directory).
6. **Eyeball the saved screenshot.** A byte-size check proves something drew, not that it drew
   correctly — it would not have caught this session's off-screen grid.

### Reusable procedure — changing a file mode in this repo

`core.fileMode = false` makes the obvious approach silently no-op:

1. `git ls-files -s <paths>` — read the **recorded** mode. `ls -l` is not evidence here.
2. `git update-index --chmod=+x <paths>` — changes what is committed.
3. `chmod +x <paths>` — *also* needed, or your own local run still fails and you cannot verify.
4. Re-check with `git ls-files -s`; confirm the count matches what you intended to change.
5. **Verify by fresh clone** (`git clone --branch <b> . /tmp/x`) — that is the condition that
   was broken, and the only check that cannot be fooled by local state.

### The bundled smoke's dual assertion (primary artifact)

```bash
# Assertion 1 — structural, and it runs BEFORE the render so a stray asset
# can never be masked by a frame that happened to draw.
strays="$(find "$WEB_DIR" \( -name '*.png' -o -name '*.jpg' … \) -print)"
[[ -n "$strays" ]] && { echo "FAIL: an image file is served beside the page"; exit 1; }

# Assertion 2 — the frame actually drew.
(( bytes >= MIN_PNG_BYTES ))   # blank ~4KB; a real frame is well above 15000
```

### The identity invariant, as written into the code

The doc comment is deliberately explicit about *why*, because the failure is silent:

```rust
// The image goes in under the SAME verbatim key, so `atlas.texture_path()` and the
// renderer's texture-cache key are one string. Diverging them is the 2026-05-29
// white-sprite bug.
let img_handle = self.load_image_bytes(&*key, bytes);
```

And the test says what would otherwise be untestable folklore:

```rust
// 3. The underlying image decoded, and is uploaded to the GPU under that same string —
//    `image_assets_for_gpu` is exactly what feeds the renderer's texture cache.
assert!(server.image_assets_for_gpu().iter().any(|(k, _)| k == key));
```

### Environment gotchas hit while running the smokes

| Symptom | Cause | Workaround |
|---|---|---|
| `(eval):1: command not found: timeout` | macOS ships no GNU `timeout` | drop it, or `gtimeout` from coreutils |
| `(eval):1: permission denied: scripts/x.sh` | the exec-bit bug (fixed in #378) | `bash scripts/x.sh` at the time |
| `FAIL: port N already in use` | orphaned `http.server` from an earlier run | the smokes guard the port on purpose; `pkill -f "http.server <port>"` |

### The `include`-list question, resolved by reading rather than assuming

Moving the example to `examples/embedded_atlas/embedded_atlas.rs` takes it out of the package
`include` list's `examples/*.rs` glob. That looked like a `cargo package` hazard — until the
existing web examples showed the same shape (`centered_text`, `game_feel`, `web_audio`,
`wasm_save` are all nested and none is covered) with **Package dry-run green on every PR**.
`cargo package`'s verify build compiles lib+bins, not examples, so an example's
`include_bytes!` of a non-packaged asset is never read. Confirmed empirically: #377's Package
dry-run passed in 1m14s.

## Code Analysis

- **`AssetServer::load_atlas_bytes(key, bytes, cols, rows)`** (`src/asset/atlas_loading.rs`) — `let key: Arc<str> = Arc::from(key.into())`, cache hit on `atlas_path_to_id`, else `self.load_image_bytes(&*key, bytes)` for the image handle, `alloc_id()`, insert into `atlases` + `atlas_path_to_id`. The `&*key` reborrow is what keeps both registrations on one string.
- **The two `Arc<str>` allocations are separate but content-equal** — `load_image_bytes` builds its own from the `&str`. This mirrors the path form exactly (`load_atlas` calls `load_image(path)` which re-derives `asset_key`), so nothing changed about how lookups behave.
- **`TextureAtlas::texture_path()`** returns `self.handle.path()` — the *image* handle's path. That is why registering the image under the caller's verbatim key is load-bearing rather than cosmetic.
- **`image_assets_for_gpu()`** (`src/asset/image_loading.rs:157`, `pub(crate)`) iterates `path_to_id` and yields `(String, ImageAsset)`. `App::upload_asset_server_images_to_gpu` consumes it each frame and skips keys the renderer already has — this is the seam byte-sourced assets ride instead of `pending_textures`.
- **`Camera::world_to_screen(w) = (w - self.position - self.shake_offset()) * self.zoom`** (`src/camera.rs:163`), and `Camera::default()` is `position: Vec2::ZERO, zoom: 1.0`. Hence the identity, and hence the example's layout bug.
- **`App::run` differs by target** — it consumes `self` on wasm (a `mut` binding warns) while the native path needs `&mut` for the acceptance test. That asymmetry is why `build_app()` returns the app rather than running it.
- **Web-shipped example layout** — `examples/<name>/<name>.rs` + `[[example]] path = …` in Cargo.toml + `examples/<name>/web/{build.sh,index.html}`, with `pkg/` gitignored. `examples/*.rs` in the package `include` list does **not** cover the nested form; that matches every existing web example and Package dry-run stays green (examples are not compiled by the verify build).
- **`git update-index --chmod=+x` vs `chmod +x`** — the former changes the index (what is committed), the latter the working tree. Under `core.fileMode = false` only the former is durable, and only the latter makes a local run work. Both are needed while working; only the former matters to other clones.

## Files Changed

### Source code
- `src/asset/atlas_loading.rs` — **`AssetServer::load_atlas_bytes`** (+~50 incl. doc comment).
- `src/app/assets.rs` — **`App::load_atlas_bytes`** with a `rust,no_run` doctest.

### Tests
- `src/asset/tests.rs` — `png_bytes` helper + 3 tests (identity / path-vs-bytes A/B / corrupt embed).

### Examples (the acceptance tests)
- `examples/embedded_atlas/embedded_atlas.rs` — **NEW** (~290). Renders 12 tiles of an embedded 4×3 sheet; `build_app()` shared by native and wasm entry points; `cfg`-gated headless acceptance test.
- `examples/embedded_atlas/web/build.sh`, `examples/embedded_atlas/web/index.html` — **NEW**. wasm-bindgen harness; `?autostart=1` for the smoke.
- `examples/embedded_image.rs` — headless block moved into a `cfg(not(wasm32))` `run_acceptance_test`; `AssetServer` dropped from the top-level import. **Now builds for wasm.**

### Scripts
- `scripts/embedded_atlas_smoke.sh` — **NEW** (~150). Dual-assertion headless-Chrome render smoke.
- 31 `.sh` files — `100644` → `100755` (26 changed).

### Docs / release
- `docs/VERIFICATION.md` — **NEW** (164). Six exit-code traps, three blind spots, the smoke table.
- `CLAUDE.md` — atlas module-map row extended twice; Verification section rewritten; `Documentation rules` + `Project direction` compressed; Document map row added. 207 → **194** lines. Header v1.6.228 → **v1.6.232**.
- `docs/CHANGELOG.md` — 0.135.0, 0.135.1, 0.135.2 entries.
- `Cargo.toml` / `Cargo.lock` — 0.134.0 → 0.135.0 → 0.135.1 → 0.135.2; `[[example]] embedded_atlas` entry added.

### Memory (not in any PR)
- `engine-current-state.md` — seq 195/197/198 prepends, a rewritten BOARD paragraph, two new live gotchas, and a **76 KB → 21 KB trim** (seqs 184→173 archived).
- `engine-history-archive.md` — received the trimmed chain tail (260 KB → 315 KB).
- `MEMORY.md` — index hook refreshed twice.

## User Feedback & Preferences (REQUIRED)

- **The opening instruction was a paste prompt** telling me to execute the parent plan from Phase 1 and that "Phase 1 ENDS IN A QUESTION… Do NOT self-pick." Followed literally.
- **"load_atlas_bytes (추천)"** — the user's pick from the four-option menu. They took the recommendation, which suggests the recommendation framing (why it is the smallest coherent gap) is worth keeping.
- **"다음 작업 이어서 진행 해자. 작업 리스트 보여줘"** — *show me the work list, then continue.* The user wants to see options before work proceeds, not a self-selected next task. Presenting a categorized table (A/B/C/D) with a recommendation worked; they picked from it immediately.
- **"A1+A2 — byte-source wasm 스토리 완성 (추천)"** — again took the recommended option.
- **"b 이어서 진행해"** — terse. Referred to B1 (the CLAUDE.md trim) from the presented list. Short instructions reference the list by label; keep list labels stable across messages.
- **"b2 까지 진행해"** — *proceed through B2*, i.e. this handoff. Same pattern.
- **Board-first is standing.** Read both channels before anything else; when both are empty, **ASK — do not self-pick.** Exercised twice this session (once at the start, once mid-session after the first merge).
- **Merge authority is standing-delegated** — squash on green CI, async auto-merge, no per-PR confirmation. Exercised on all four PRs.
- **Korean to the user, English in artifacts** — every status report and question in Korean; code, comments, commit messages, PR bodies, CHANGELOG, docs and this handoff in English.
- **No mid-session course corrections.** After each go-ahead the session ran end-to-end (design → implement → verify → ship → PR → merge → memory) with no further input. Calibration: once scope is approved, proceed autonomously through the whole loop.
- **Scope discipline in both directions is expected.** Out-of-scope-but-necessary work (widening the exec-bit fix from 3 files to 31; trimming the memory file) was done and *reported as such*, not silently included. Conversely the module-map restructure was flagged as the user's call rather than made.
- **Corrections are expected to be plain and immediate.** Two overstatements were corrected mid-flight (four broken smokes → three; "environment is fine" → proven by A/B) without ceremony.

## Where We're Going

1. **Board gate FIRST, every session** — `../dungeon-merchant/docs/engine-wishlist.md` (next free **EW-012**; EW-001–011 all closed and archived) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`). A newly-filed request preempts everything below.
2. **If both are empty (likely): ASK — do NOT self-pick.** The menu, with windowed capture now **removed** (the game declined it):
   - **A fourth procgen mode** (drunkard's-walk / room-accretion / Voronoi) over the shared `DungeonMap` — composes with `to_path_grid` / `to_tilemap_tiles` / `FovMap` for free, and the example pattern is proven three times.
   - **Audio-reactive hooks** — expose a playing sound's amplitude/frequency to game logic and visuals. Native (rodio) and wasm (WebAudio `AnalyserNode`) differ, so the `Audio` facade needs an intersection API designed first.
   - **A second capstone game** — the largest scope; wants design discussion before implementation.
   - **The deferred `MapGenerator` trait** — still an anti-goal. Its trigger (something wanting to swap generators at runtime) has not fired.
3. **This handoff lands as its own `docs(handoff)` PR** (repo convention), chain `board-ew-triple` seq 2. Bump memory to **seq 199** after it merges so the recorded `main @ <hash>` points at the handoff merge.
4. **If CLAUDE.md needs headroom again**, the decision to put to the user is whether the module map (80 of 194 lines) moves to `docs/MODULE_MAP.md`. Do not make that call unilaterally — it changes how every session navigates the codebase.

## Risks & Blockers

- **Verification tooling that lies is the dominant risk in this repo.** Three distinct mechanisms have now bitten: a piped exit code, a background task's notification, and a stale `.exit` file. All are in `docs/VERIFICATION.md`. Read a gate's result from a non-piped command whose freshness you have checked.
- **`core.fileMode = false` will hide the next mode change too.** Anything about file permissions must be verified with `git ls-files -s`, never `ls -l`, and set with `git update-index --chmod=+x`.
- **The wasm gate still excludes examples.** After touching an example's `cfg(target_arch = "wasm32")` path — or adding one that claims web support — build it explicitly. `embedded_image` was broken from the day it was added and nothing noticed.
- **The Claude-in-Chrome interactive tab renders these wasm demos blank.** Not diagnosed further; the already-shipped `centered_text` demo behaves identically there while passing its headless smoke. Use the smoke scripts, not the tab, for web verification.
- **CLAUDE.md has 6 lines of headroom** and the module map grows ~1 line per feature. It will breach again within a few features.
- **`engine-current-state` memory grows ~1 KB per seq** and is at 25 KB. The Read/Edit cap is ~25k tokens (~76 KB); trim the chain tail into `engine-history-archive` well before that.
- **2 audio tests can fail on a no-audio box** (carried gotcha) — not hit this session.
- **`dungeon-merchant` has no CI/branch protection** — its board is discipline, not enforcement. `rust-survivors` is paused/deprecated; do not chase compatibility.

## Open Questions

- **Should the module map move out of CLAUDE.md?** It is 80 of 194 lines and the sole growth driver. Moving it buys real headroom but changes the primary navigation surface. Flagged to the user, not decided.
- **Why do the wasm example demos render blank in the Claude-in-Chrome tab?** WebGPU adapter and WebGL2 are both available and the engine's DPR canvas resize runs, so it is not a missing-backend problem. Headless Chrome renders the same page correctly. Not worth chasing unless someone needs the interactive tab.
- **Should `embedded_image` also get a web harness?** It now builds for wasm but has no `web/` directory. Symmetry argues yes; nothing has asked for it.
- **Do the remaining web examples' smokes actually pass?** `bloom`, `game_feel`, `hdr`, `render_format_query`, `audio_facade`, `positional_audio` were unblocked by #378 but only `centered_text` and `embedded_atlas` were run to completion this session.
- **Is `MIN_PNG_BYTES = 15000` the right blank-frame threshold for every window size?** It was inherited from `centered_text` (960×540) and used unchanged at 820×470. It passed with 102 KB, so there is wide margin, but a much smaller demo could false-fail.

## Quick Start for Next Session

```bash
# Nothing is dangling — all four PRs merged, main is clean at af4573a (v0.135.2).

cd ~/Projects/skeleton-engine
git log --oneline -5      # expect af4573a at the tip
git status -s             # expect clean

# 1. READ THE BOARD FIRST — both channels (a new request preempts everything)
#   ../dungeon-merchant/docs/engine-wishlist.md        (next free EW-012; EW-001-011 all
#                                                       Verified + archived, NO open requests)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md   (currently _None._)

# 2. Verify starting state — read the exit code from a NON-PIPED command,
#    and check the file is FRESH (a stale .exit from a prior session reads as an instant pass)
rm -f /tmp/verify.exit
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# 3. Re-prove this session's feature
cargo test --lib load_atlas_bytes          # expect 3 passed
cargo test --lib                           # expect 1297 passed
HEADLESS_SHOT=/tmp/ea.png cargo run --example embedded_atlas
# expect: "OK: 256x192 4x3 atlas decoded from include_bytes! ..." ; exit 0
scripts/embedded_atlas_smoke.sh            # optional; needs Chrome + wasm-bindgen-cli

# 4. Key files to read first
#   docs/VERIFICATION.md                          — the gate's traps and blind spots (NEW)
#   src/asset/atlas_loading.rs                    — load_atlas_bytes + the identity comment
#   src/asset/tests.rs                            — the 3 tests, incl. the path-vs-bytes A/B
#   examples/embedded_atlas/embedded_atlas.rs     — build_app() + the dual-target entry points
#   scripts/embedded_atlas_smoke.sh               — the dual-assertion smoke pattern

# 5. FIRST ACTION: board gate → if empty, ASK for direction. Do NOT self-pick.
#    Menu: a 4th procgen mode, audio-reactive hooks, a 2nd capstone game.
#    Windowed capture is OFF the menu — the game declined it on 2026-07-27.
```
