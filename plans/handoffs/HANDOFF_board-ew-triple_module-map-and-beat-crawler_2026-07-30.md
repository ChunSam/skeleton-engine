# Shipped: the module map moved out of CLAUDE.md, and a second capstone whose turn clock is the music (v0.137.0 → v0.138.0)

**Date:** 2026-07-30
**Status:** COMPLETED (PRs #387–#391 all merged; `main @ b6135f2`, v0.138.0, clean + green)
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `board-ew-triple` seq `4`
**Parent:** `HANDOFF_board-ew-triple_audio-reactive-levels-spectrum_2026-07-30.md`
**Prior chain:** `HANDOFF_board-ew-triple_three-board-requests_2026-07-26.md` > `HANDOFF_board-ew-triple_atlas-bytes-and-wasm-proof_2026-07-29.md` > `HANDOFF_board-ew-triple_audio-reactive-levels-spectrum_2026-07-30.md` > this

> **Note on the chain tag.** `board-ew-triple` remains historical: EW-009/010/011 are closed and archived and no board request has driven work since seq 1. The tag continues because the session was opened by a paste prompt naming seq 3 and executed that plan verbatim. Read it as "the empty-board menu chain" from seq 2 onward. **This arc has now closed cleanly** — the CLAUDE.md cap is resolved and the second capstone shipped — so a fresh tag is reasonable for the next session.

---

## Since Last Handoff

Framed against the parent (seq 3) and its "Where We're Going":

- **Every one of the parent's five next-steps was executed, in order.** Board gate first on both channels → both empty → **ASK, do not self-pick** → user chose → build. Nothing was self-picked and no code was written before the answer.
- **The parent's step 4 — the CLAUDE.md 198/200 cap — is RESOLVED (#387),** and it is the second consecutive handoff that flagged it. The user chose "move the module map to `docs/MODULE_MAP.md`".
- **The parent's open question "Should the module map move out of CLAUDE.md?" is ANSWERED: yes.** But the *reasoning* changed en route — see Chunk 1; the parent (and seq 198 before it) had measured **lines**, and measuring **bytes** overturned the conclusion about which option was best.
- **The parent's step 5 (optional `/wrap` candidates 4 and 5) is DONE (#390).** It stopped being optional: Trap 4 fired again this session, making the copy-paste block the highest-value doc change available.
- **The parent's dominant risk — "verification tooling that lies" — recurred a THIRD time.** The background notification reported `exit code 0` while the `.exit` file held **1**. Same mechanism, now documented three sessions running. Reading the file caught it again.
- **The `cargo fmt` reflow trap also fired once**, exactly as `[[cargo-fmt-reflow-trap]]` memory warns.
- **The parent's risk "a capstone (3A) sprawling past one session" did NOT materialize.** It landed in one session, in two PRs (engine fix split out from the game).
- **A new risk appeared that no prior handoff predicted:** writing the capstone surfaced a **pre-existing engine bug affecting 20 shipped examples** (`WindowConfig` dropped by the scene reset). The VISION loop worked exactly as designed — the example was the acceptance test, and it failed.
- **`Render tests (lavapipe)` did not recur at 10m34s** (56–80 s across all five PRs this session). Still unexplained, now looking like one-off runner variance.

---

## Reference Documents

- `CLAUDE.md` — project conventions + the verify gate. **Radically smaller this session:** 198 lines / 95,075 B → **137 lines / 8,231 B**. No longer holds the module map.
- **`docs/MODULE_MAP.md`** — **NEW.** The 72-row "where do I read to find X" table, extracted from CLAUDE.md. **Grep it; do not read it whole.**
- **`docs/VERIFICATION.md`** — the six exit-code traps + three blind spots. Now carries the copy-paste background-gate block (155 → **178 lines**).
- **`docs/PATTERNS.md`** — architecture patterns + task recipes. Gained the scene-reset persistence pattern (378 → **434 lines**).
- `docs/CHANGELOG.md` — 0.137.1 and 0.138.0 entries are the migration notes.
- `docs/VISION.md` — "a feature is not done until a playable example exercises it in real play"; this session is the clearest instance yet of that rule paying out.
- `../dungeon-merchant/docs/engine-wishlist.md` — the shared board. Empty; last edited **2026-07-27**, before this session.

---

## The Goal

Execute the parent's plan: run the board gate, and — because both channels were empty — get an explicit direction from the user rather than self-picking. The user made two calls in one round-trip: **(i)** resolve the CLAUDE.md 200-line cap by moving the module map to `docs/MODULE_MAP.md`, and **(ii)** build a **second capstone game**, taking the recommended genre (a beat-driven dungeon crawler).

The capstone's purpose is specific and worth restating: the engine's surface has grown far faster than its *integration* coverage. Ten UI widgets, a game-feel toolkit, three procgen modes, FOV, dialogue, timeline, skeletal animation and now audio analysis are each exercised by a **single-purpose demo written by the same session that designed the feature**. `docs/VISION.md` says API awkwardness surfaces in real play; that had never actually been tested against composed usage. It was this session, and it immediately found a pre-existing engine bug.

A late follow-up request from the user then added a fifth PR: **document the deferred `Audio`-persistence risk** rather than fixing it, with explicit reopen triggers.

---

## Where We Are

- **`main @ b6135f2`, package v0.138.0, CLAUDE.md header v1.6.239, clean tree, all gates green.** Local `main` == `origin/main`.
- **Five PRs merged, serially, each 6/6 green:** **#387 `d946c05`** (docs), **#388 `50977ae` v0.137.1** (PATCH), **#389 `672973d` v0.138.0** (MINOR), **#390 `e192f9b`** (docs), **#391 `b6135f2`** (docs).
- **Lib tests 1338 → 1339 (+1)** — the `WindowConfig` regression test. **`ok` groups 151 → 152** (+1: the new example is a new target group).
- **`CLAUDE.md` is 137/200 lines and 8,231 bytes** — down from 198 lines / 95,075 B. That is a **−91% cut in per-session auto-loaded context**, and headroom went from 2 lines to 63.
- **`docs/MODULE_MAP.md` holds all 72 rows, verified byte-for-byte identical** to what was removed (`diff` of the extracted range against `git show 612ec20:CLAUDE.md` is empty).
- **`WindowConfig` now survives a scene reset** (one line in `App::new`), fixing a bug that silently affected **20 shipped example files**, including `roguelike`.
- **`examples/games/beat_crawler/beat_crawler.rs` (961 lines)** is the second capstone. `cargo run --example beat_crawler_game`.
- **The turn clock is `Audio::bands()` low-band energy.** Measured with the tones the game actually plays: **kick 4.00 vs blip 0.61, 6.5× apart**, threshold 1.20 sitting 3.3× below the kick and 2.0× above the blip.
- **`BEAT_CRAWLER_SELFTEST=1` passes**: 6 depths solvable across both generators, pathing approaches (7→5), kick detected, blip rejected. **No audio device is a SKIP, not a failure.**
- **The headless capture was eyeballed twice and found two real defects** — a HUD overlap, then the freeze-on-silence flaw. Fourth consecutive session in which eyeballing the PNG paid.
- **`docs/PATTERNS.md` gained "Surviving a scene reset"** — the auto-persisted list, the game's responsibilities, and the deferred `Audio` decision with reopen triggers.
- **`docs/VERIFICATION.md` gained the copy-paste background-gate block**, closing Traps 4 and 5 together, plus a "corroborate the counts" step.
- **Four local skills were repointed** (`ship`, `split-module`, `add-feature-example`, `add-ui-widget`) — all four told sessions to update "the CLAUDE.md module map", which no longer exists there.
- **Memory advanced seq 204 → 209**, plus a `local-tooling-skills` audit section. `engine-current-state.md` grew 35.8 KB → **~42 KB**.
- **The board was never touched.** Nothing this session served a board request.

---

## What We Tried (Chronological)

### Chunk 1 — Board gate, the byte measurement, and the two-part ask (early)

1. **Read the plan and handoff first**, per the paste prompt, before touching anything.
2. **Ran the board gate on both channels as the first real action.** `../dungeon-merchant/docs/engine-wishlist.md` → Active requests **empty**, next free **EW-012**; the `[Game]` comments contained no unfiled asks (the last, 2026-07-03, explicitly says they are *not* filing an EW). `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` → `_None._`. `git log -1` on the board file → **2026-07-27** (`539b610`), i.e. unchanged since before the parent session. **Verdict: empty on both channels, nothing preempts.**
3. **Verified the starting state and read the exit code from the file**, with an mtime freshness check: **exit 0**, 151 `ok` groups, 1338 lib tests — matching the parent's recorded baseline exactly.
4. **Measured `CLAUDE.md` in BYTES, not lines — and it overturned the plan's premise.** The plan (inheriting seq 198's measurement) framed the decision around line counts: 81 of 198 lines. Measuring bytes:

   | | lines | bytes |
   |---|---|---|
   | `CLAUDE.md` total | 198 | 94,293 |
   | of which the module map | 82 (41%) | **88,304 (94%)** |

   Longest single row: **8,012 characters** — one line. Median row: 488.

5. **That reframed which option was correct.** A line cap cannot see a row growing from 500 to 8,000 characters, and option (c) "merge related rows" — the option that buys line headroom — would have left the real cost untouched. It games the metric. Option (b) "raise the cap" treats the wrong number. Only extraction addresses the actual per-session cost, and **it makes navigation better, not worse**: grep for your topic (3 rows) instead of carrying all 72.
6. **Asked ONE `AskUserQuestion` in Korean carrying BOTH decisions**, with the reasoning rather than just labels, as the plan required. Reported the board verdict and the byte measurement first, explicitly flagging that the measurement changed my recommendation from what the plan assumed.
7. **The user took both recommendations**: "모듈 맵 분리 (추천)" and "2번째 캡스톤 게임 (추천)". That is now **five recommendation-takes out of five** across this and the parent session.

### Chunk 2 — Phase 2: extracting the module map (#387) (early-mid)

8. **Enumerated every reference to the module map across the repo before moving anything** — this is what caught the four local skills.
9. **Found `AGENTS.md` has its own separate, condensed ~20-row map** at 122 lines / 7.4 KB. **Deliberately left untouched** — it is a distinct artifact for Codex and is under no pressure.
10. **Performed the split with a script rather than editing 88 KB through the Edit tool** (which would have burned enormous context for zero benefit): header + `sed -n '74,145p'` for the rows, then `head -67` + pointer + `tail -n +146` for the new CLAUDE.md.
11. **Verified data integrity by diffing the extracted rows against `git show HEAD:CLAUDE.md`.** First attempt reported DIFFERS — **my comparison range was off by one** (the table separator sits at line 19, not 18). Re-ran with the correct range: **IDENTICAL, byte-for-byte, 72 rows.** Worth noting I did not accept the first result either way.
12. **Caught my own error in the new docs.** I had written "73 rows" everywhere, because `grep -c '^| '` counts the table *header* row too. The byte-identical diff showed **72** data rows. Corrected in CLAUDE.md, `docs/MODULE_MAP.md` and `FORKING.md`.
13. **Rewrote the Documentation-rules "Length" bullet** to record the byte measurement as a rule, so a future session neither re-derives it nor "fixes" the cap by merging rows.
14. **Updated `FORKING.md` (2 places)** and the **four local skills**, which all instructed sessions to update "the CLAUDE.md module map".
15. **Left `plans/**` and `docs/CHANGELOG.md` mentions alone** — they are dated historical records of past sessions and describe what was true then.
16. **Verify exit 0** (151 groups, 1338 tests) → PR #387 → `--auto --squash` → 6/6 green → merged `d946c05`.
17. **Recorded the measurement in the PR body, not just the commit**, so the next session inherits the numbers rather than re-deriving them — the plan asked for exactly this.

### Chunk 3 — Phase 3A: the capstone, and the engine bug it exposed (mid)

17. **Dogfooded the new map immediately** — `grep -in 'HitFlash\|FloatingText' docs/MODULE_MAP.md` returned 3 targeted rows instead of 88 KB. The workflow works.
18. **Verified every symbol I intended to import is re-exported from `src/lib.rs`** (28 symbols) before writing a line of the game.
19. **Confirmed two layout facts from source rather than assuming:** the sprite quad is `[-0.5, 0.5]`, so `Transform.position` is the **center** and `scale` is the size; and `Camera.position` is the viewport's **top-left**, so with the camera at the origin, world coordinates read like screen coordinates (+X right, +Y down) — the same orientation as the grid.
20. **Chose full world-space `Sprite` entities over `roguelike`'s immediate-mode `UiQueue` rects.** `HitFlash`, `FloatingText` and `Camera::shake` are entity/world-based, so an immediate-mode grid would have exercised almost none of the game-feel toolkit — the whole point of the capstone. One sprite per cell (640), recolored on move.
21. **Designed the rhythm so that `bands()` is load-bearing rather than decorative:** a 16-step pattern of 110 Hz kicks and 880 Hz blips, where only kicks are turns. **`levels()` provably cannot express this** — the blips are equally loud and each would fire a phantom turn. The discrimination is by *frequency*. The `PATTERN` const is read **only** by the audio scheduler; no gameplay code sees it.
22. **Made placement generator-agnostic via BFS distance from spawn, not the room list.** `generate_cellular_cave` records only a single 1×1 `Room`, so a room-based placement would silently degenerate on even depths. Stair = farthest reachable cell, which makes every level solvable *by construction* for either generator.
23. **First self-test run passed but with a thin margin: blip 0.923 against a 1.20 threshold (1.3×).** I did not accept that. The cause was that I was measuring with a **longer and louder tone than the game actually plays** (0.35 s at 0.9 vs the game's 0.05 s at 0.5).
24. **Fixed the test rather than the threshold.** Re-measured with the game's real parameters: **kick 4.000, blip 0.614 — 6.5× apart.** Measuring the wrong case is precisely how a threshold gets tuned against a situation that never occurs; the doc comment now says so.
25. **First headless capture — and it exposed THREE defects at once.** The PNG was 1280×720 on a grey-purple background with the health bar drawn on top of the depth readout.
26. **Isolated whether the size/colour problem was mine or the engine's, rather than guessing.** Ran `roguelike` headless: **also 1280×720**, against its configured 892×674. So it was pre-existing and engine-wide.
27. **Traced the mechanism:** `App::set_scene` → `reload_scene` rebuilds the `World`; `WindowConfig` (inserted before `run()`) is not registered persistent, so it is replaced by `WindowConfig::default()` on the **first** scene enter. Headless capture reads the size *after* the reset; `clear_color` is read per frame.
28. **Found the engine already knew — and had routed around it twice instead of fixing it.** `src/app.rs:80`: *"Stored here rather than read from `WindowConfig` because a scene transition (`World` reset) can revert `WindowConfig` to its default."* `src/app/schedule.rs:215`: *"stable across scene resets, unlike `WindowConfig`."* Both are workarounds for this exact bug.
29. **Quantified the blast radius before proposing a fix:** 20 example files set both a `WindowConfig` and a scene.
30. **Fixed the cause with one line** — `app.register_persistent::<WindowConfig>()` in `App::new`, the same treatment `TextMeasurer` gets one line above. Left the wasm workarounds in place; they remain correct.
31. **Proved the regression test is not vacuous.** Disabled the one-line fix → `test ... FAILED / assertion left == right failed: scene reset reverted WindowConfig to its default size`; restored → ok. A regression test that passes without the fix is worthless.
32. **Checked whether `Audio` needs the same treatment, and found the precedent.** `examples/games/settings_menu:992` already calls `register_persistent::<Audio>()`. So `Audio` persistence is established game-side responsibility, and I followed it rather than changing engine behaviour.
33. **Fixed the HUD overlap by deriving every row from one `HUD_TOP` constant** instead of independent magic numbers — the same class of fix (and the same root cause) as the `audio_reactive` label bug one session earlier.
34. **Second capture: correct 864×660 on the correct background, HUD clean — but `beats 0`.** I did not dismiss this as a headless artifact.
35. **That number exposed a real design flaw.** The turn clock keys off wall-clock audio, so *anything* that silences output — a muted OS, a busy device, an `AudioContext` that never unlocks on the web — would leave the dungeon **frozen forever with no explanation**. A player muting their OS would experience the game as broken.
36. **Added `BEAT_WATCHDOG` (1.2 s)**: after that long with kicks scheduled but none heard, fall back to the schedule and say so in the HUD (`schedule (nothing heard)` vs `bands() — listening`). This is a genuine robustness fix that also happens to make the game headless-drivable.
37. **Hit a false-green from my OWN command.** `cargo clippy … | grep -E "^(error|warning)" -A6 | head -30` reported `CLIPPY_EXIT=0` while the file had compile errors. Re-ran without pipes — the real exit *was* 0, and the "errors" were **stale rust-analyzer diagnostics captured mid-edit**. Both halves of that are worth remembering: my pipeline was unreliable, *and* the IDE diagnostics were wrong.
38. **Third capture confirmed the watchdog:** `beats 2`, `turn clock: schedule (nothing heard)`.

### Chunk 4 — Landing, Phase 4, and the user's risk-documentation request (late)

39. **Split the work into two PRs deliberately**, per the plan's "revert any engine-side change separately so the capstone can land without it": #388 (engine fix, PATCH v0.137.1) then #389 (capstone, MINOR v0.138.0).
40. **The `sed` version bump silently missed** because `Cargo.toml` uses padded alignment (`version     = "0.137.0"`). Caught by checking the output rather than assuming; fixed and re-verified against `Cargo.lock`.
41. **Verify gate went red once on the capstone: exit 1, `cargo fmt --check`** — the documented reflow trap (import list, an array literal, and a long `let`). Ran `cargo fmt`, re-verified: exit 0, **152 groups, 1339 tests**.
42. **The completion notification claimed `exit code 0` on that red run.** Trap 4, third occurrence. The `.exit` file said **1**.
43. **Phase 4 (`/wrap` candidates 5 and 4) landed as #390.** Candidate 5 stopped being optional given Trap 4 had just fired again; wrote the whole procedure as a copy-paste block closing Traps 4 *and* 5, plus a "corroborate the counts" step the doc did not have.
44. **User follow-up: "리스크 문서화 해서 남겨두고 다음에 문제가 생기면 작업하도록 해줘"** — document the risk, act only if it bites.
45. **Enumerated the auto-persisted list from source before writing the doc**, so it is accurate rather than remembered: `WindowConfig`, `SceneTransition` (registered inline in the `App` literal, not via a call), `TextMeasurer`, `InputScript`, the 7 RON registries, plus `DebugUi` carried by hand inside `reload_scene`.
46. **Wrote "Surviving a scene reset" into `docs/PATTERNS.md`** with the auto-list, the game's responsibilities, the scene-state-vs-session-state test, and a blockquoted "Known rough edge, deliberately not fixed" carrying the **reopen triggers**.
47. **Added a CLAUDE.md pointer** (it has headroom now) and recorded the deferral as a **STANDING item in memory** with the same triggers, so a future session sees it without reading the repo.
48. **Updated memory to seq 209** and appended a `local-tooling-skills` section recording the four skill edits — skills are gitignored, so memory is the only record.
49. **Filled the merge hash into memory only after #391 actually merged** — a handoff/memory line written before the merge records a stale tip, which is the mistake seq 2 and seq 3 both made. Used a `<seq209>` placeholder and substituted `b6135f2` afterwards.
50. **Amended the long-standing `set_scene` gotcha in memory** rather than adding a new one: it now reads "`set_scene` RESETS the world … — **`WindowConfig` is EXEMPT as of v0.137.1**, but `Audio` and everything else a game inserts is NOT." Editing the existing line keeps the two facts from drifting apart.

---

## Key Decisions

- **Ask, don't self-pick — again, and it again produced better work.** The board was empty and the plan's Phase 1 deliberately ends in a question. Both of the user's picks were the ones I recommended, but the recommendation itself changed *because I measured first* (see below).
- **Measure bytes, not lines — and say so when it overturns the inherited plan.** The plan, and seq 198 before it, framed the CLAUDE.md decision in lines. Bytes showed the map was 94% of the cost in 41% of the lines. I reported this to the user *as a change to the plan's premise* rather than quietly acting on it.
- **Reject "merge related rows" explicitly as metric-gaming.** It buys line headroom while leaving per-session cost identical, and makes already-dense rows worse. Naming that in the options was more useful than just ranking them.
- **Extraction improves navigation rather than degrading it.** The obvious objection is "now it's one hop away". The answer is that a session needs 2–3 rows, not 72, and a separate file is greppable — so the hop is cheaper than the carry.
- **Use world-space sprite entities for the capstone, not immediate-mode rects.** `roguelike`'s `UiQueue` approach is simpler but would have exercised none of `HitFlash`/`FloatingText`/`Camera::shake`, which is the reason the capstone exists.
- **Make `bands()` load-bearing, not decorative.** Kicks and blips at equal loudness mean `levels()` *cannot* substitute. This is the difference between a capstone that demonstrates a feature and one that depends on it.
- **Fix the measurement, not the threshold, when the margin looked thin.** The 1.3× margin was an artifact of testing a tone the game never plays. Widening the threshold would have hidden that.
- **Split the engine fix into its own PR (#388) before the capstone (#389).** The plan asked for this, and it is right: the fix stands on its own merits, is independently revertible, and benefits 20 examples regardless of whether the game lands.
- **Fix the cause, not add a third workaround.** The engine had already routed around the `WindowConfig` reset twice. A third would have been cheaper locally and wrong globally.
- **Prove the regression test fails without the fix.** Disabling the one line and watching it go red is the only thing that distinguishes a real regression test from a decorative one.
- **Leave `Audio` game-side, and write down *why* plus what would reopen it.** Auto-persisting whatever a game inserts would empty `register_persistent` of meaning. But the footgun is real, so the deferral carries explicit triggers rather than being silently dropped.
- **Treat `beats 0` in the capture as a finding, not an artifact.** It would have been easy to write off as "headless runs faster than audio". Following it produced the watchdog, which fixes a genuine player-facing failure.
- **Leave `AGENTS.md` alone.** It has its own small map and no pressure; changing it would have been scope creep.
- **Don't touch historical `plans/**` and CHANGELOG references to the old map location.** They are dated records of what was true.

---

## Evidence & Data

### Shipped: PR → version → commit → diffstat

| PR | Version | Bump | Commit | Files | +/− |
|---|---|---|---|---|---|
| #387 · module map extraction | — | docs-only | `d946c05` | 3 | +110 / −83 |
| #388 · `WindowConfig` persists | **0.137.1** | PATCH | `50977ae` | 5 | +54 / −3 |
| #389 · `beat_crawler` capstone | **0.138.0** | MINOR | `672973d` | 6 | +986 / −3 |
| #390 · verification block | — | docs-only | `e192f9b` | 2 | +30 |
| #391 · scene-reset risk | — | docs-only | `b6135f2` | 2 | +56 / −2 |

### CI behaviour (all 6/6 green, all five PRs)

| PR | Test (native) | Render (lavapipe) | WASM | Windows | Rustdoc | Package |
|---|---|---|---|---|---|---|
| #387 | 294s | 56s | 42s | 92s | 49s | 61s |
| #388 | 296s | 72s | 41s | 104s | 36s | 65s |
| #389 | 390s | 80s | 42s | 103s | 50s | 87s |
| #390 | 379s | 37s→ 64s | 37s | 85s | 35s | **156s** |
| #391 | 283s | 62s | 42s | 86s | 37s | 61s |

`Render tests (lavapipe)` stayed in its 56–80 s norm across all five — the parent's unexplained **10m34s** on #382 did not recur.

### The measurement that decided Phase 2

| | lines | bytes |
|---|---|---|
| `CLAUDE.md` before (`612ec20`) | 198 | **95,075** |
| of which the module map | 82 (41%) | **88,304 (94%)** |
| `CLAUDE.md` after #387 | 132 | 7,767 |
| `CLAUDE.md` after #391 (final) | **137** | **8,231** |

Row lengths inside the map: longest **8,012** chars, then 5,997 / 5,491 / 4,166 / 4,044; **median 488**. This is why a *line* cap could not constrain it.

| | before | after |
|---|---|---|
| per-session auto-loaded context | 95,075 B | **8,231 B** (−91%) |
| headroom under the unchanged 200-line cap | **2 lines** | **63 lines** |
| module-map rows | 82 lines inline | 72 rows in `docs/MODULE_MAP.md` |

### Verify-gate history (every run; exit codes read from the FILE, never the notification)

| # | Tree | Exit | Groups / tests | Cause |
|---|---|---|---|---|
| 0 | session start, clean `main` | **0** | 151 / 1338 | baseline, matches parent |
| 1 | module map extracted | **0** | 151 / 1338 | — |
| 2 | + `WindowConfig` fix + test | **0** | 151 / **1339** | +1 regression test |
| 3 | + capstone | **1** | — | `cargo fmt --check` reflow (3 sites) |
| 4 | after `cargo fmt` | **0** | **152** / 1339 | +1 group = the new example target |
| 5 | + verification docs | **0** | 152 / 1339 | — |
| 6 | + scene-reset risk docs | **0** | 152 / 1339 | — |

**One red run — and the background notification reported `exit code 0` for it.** Trap 4, third occurrence across three sessions.

### The `WindowConfig` bug — blast radius and proof

| Fact | Value |
|---|---|
| Example files setting both `WindowConfig` and a scene | **20** |
| `roguelike` configured window | 892 × 674 |
| `roguelike` actual headless capture | **1280 × 720** (the engine default) |
| `beat_crawler` capture before fix | **1280 × 720**, default grey background |
| `beat_crawler` capture after fix | **864 × 660**, configured dark background |
| Fix size | **1 line** in `App::new` |
| Test proven non-vacuous | yes — disabled the fix → `FAILED` |

Prior acknowledgements of the bug found in-tree (workarounds, not fixes):

```
src/app.rs:80        "…because a scene transition (`World` reset) can revert
                      `WindowConfig` to its default, whereas the canvas
                      attributes are stable."
src/app/schedule.rs  "Logical size = the authored canvas attributes captured in
        :215          finish_init (stable across scene resets, unlike WindowConfig)."
```

### Rhythm detection — the numbers that justify `bands()` over `levels()`

| Tone | duration | volume | low-band energy (4 of 16 bands) | vs threshold 1.20 |
|---|---|---|---|---|
| kick 110 Hz | 0.10 s | 0.9 | **4.000** | 3.3× above |
| blip 880 Hz | 0.05 s | 0.5 | **0.614** | 2.0× below |

**Separation 6.5×.** An earlier measurement using a longer/louder tone (0.35 s @ 0.9) gave blip **0.923** — only 1.3× of margin — which is what prompted re-measuring with the game's real parameters instead of adjusting the threshold.

### `beat_crawler` self-test output (full)

```
depth 1 (27 cells to the stair) ok
depth 2 (16 cells to the stair) ok
depth 3 (27 cells to the stair) ok
depth 4 (15 cells to the stair) ok
depth 5 (37 cells to the stair) ok
depth 6 (25 cells to the stair) ok
pathing approaches: 7 -> 5 ok
kick  low-band peak 4.000 (must reach 1.20)
blip  low-band peak 0.614 (must stay under 1.20)
PASS: levels solvable, enemies approach, kick 4.00 / blip 0.61 separated by 6.5x
      around threshold 1.20
```

Depths 1/3/5 are BSP dungeons, 2/4/6 are cellular caves — both generators exercised, all six solvable.

### Headless capture iterations (each one found something)

| # | Frame | Result | What it exposed |
|---|---|---|---|
| 1 | 20, 90 | 1280×720, grey bg, HUD overlapping | **3 defects**: `WindowConfig` lost (size + colour), health bar on top of the depth text |
| 2 | 120 | 864×660, correct bg, HUD clean, **`beats 0`** | the freeze-on-silence flaw |
| 3 | 200 | `beats 2`, `schedule (nothing heard)` | watchdog confirmed working |

### Auto-persisted resources (enumerated from source for #391)

| Type | Registered where |
|---|---|
| `WindowConfig` | `App::new` (NEW, v0.137.1) |
| `SceneTransition` | inline in the `App` struct literal, `app.rs:249` |
| `TextMeasurer` | `App::new`, `app.rs:296` |
| `InputScript` | `input_script.rs:602` |
| `DataTableRegistry`, `AnimationClipRegistry`, `ParticleConfigRegistry`, `DialogueRegistry`, `TriggerZoneRegistry`, `ZoneEffectRegistry`, `AnimEffectRegistry` | `app/editor/loading.rs` |
| `DebugUi` | **not** via the registry — removed and re-inserted by hand in `reload_scene` |

### Doc file sizes, before → after

| File | Before | After |
|---|---|---|
| `CLAUDE.md` | 198 lines / 95,075 B | **137 / 8,231 B** |
| `docs/MODULE_MAP.md` | — | **92 lines / ~89 KB (NEW)** |
| `docs/PATTERNS.md` | 378 | **434** |
| `docs/VERIFICATION.md` | 155 | **178** |
| `examples/games/beat_crawler/beat_crawler.rs` | — | **961 (NEW)** |

### Trap 4 occurrence history (the reason #390 exists)

| Session | Notification said | `.exit` file held | Real cause |
|---|---|---|---|
| 2026-07-29 | `exit code 0` | **1** | `cargo fmt` reflow |
| 2026-07-29 | `exit code 0` | **101** | 4 rustdoc errors |
| **2026-07-30 (this)** | `exit code 0` | **1** | `cargo fmt` reflow |

Documented before each of the last two. Prose alone has now failed three times — hence the copy-paste block.

### The direction question exactly as presented (for calibration)

One `AskUserQuestion`, two questions, Korean, each option carrying its trade-off rather than a label:

| Q | Options offered (recommendation first) | Chosen |
|---|---|---|
| CLAUDE.md cap | **모듈 맵 분리 (추천)** — cuts 88 KB (94% of the file) from every session; grep 3 rows instead of carrying 72; cost = one hop away · **캡을 250줄로 상향** — smallest change but treats the wrong number · **관련 행 병합** — buys 5–10 lines, context cost unchanged, "지표를 개선하는 게 아니라 지표를 속이는 쪽" | **분리** |
| Direction | **2번째 캡스톤 게임 (추천)** — beat-driven crawler; the only genre that composes the new features structurally rather than decoratively; largest and may overrun a session · **survivor에 audio-reactive 도입** — 2nd consumer, expect the `play_sfx` limitation immediately · **4번째 procgen 모드** — safest, lowest marginal value | **캡스톤** |

Both recommendations taken. The `AskUserQuestion` also carried the board verdict and the byte
measurement *before* the options, explicitly flagged as changing the plan's premise.

### `beat_crawler` internal constants (tuning surface)

| Constant | Value | Why |
|---|---|---|
| `COLS` × `ROWS` × `CELL` | 32 × 20 × 26.0 | 640 cell sprite entities; window 864 × 660 |
| `STEP_SECS` | 0.16 | 16 steps ≈ 2.6 s per bar |
| `PATTERN` | `K..b. K..bb K..b. Kb.b` | kicks at steps 0/4/8/12 → intervals 0.64/0.64/0.64/0.64 s |
| `KICK_HZ` / `BLIP_HZ` | 110 / 880 | 3 octaves apart, so the low bands separate them cleanly |
| `KICK_SECS` / `BLIP_SECS` | 0.10 / 0.05 | short enough that the meter decays between beats |
| `BANDS` / `LOW_BANDS` | 16 / 4 | the caller's choice; `bands()` resamples its internal 32 |
| `KICK_THRESHOLD` / `REARM_THRESHOLD` | 1.20 / 0.50 | measured 3.3× under the kick, 2.0× over the blip |
| `ON_BEAT_WINDOW` | 0.18 s | press-before-beat window for the ×2 bonus |
| **`BEAT_WATCHDOG`** | **1.2 s** | **scheduled-but-unheard kicks → fall back to the schedule** |
| `TORCH_RADIUS` | 7 | FOV radius |
| `PLAYER_MAX_HP` / `ENEMY_HP` | 6 / 2 | bump attack does 1, or 2 on beat |
| `MIN_ENEMY_DIST` | 6 | BFS steps from spawn — no monster in your face at level open |
| `ENEMIES_PER_DEPTH` | `[2,3,4,5]` | saturates at depth 4+ |
| `HUD_TOP` + `HUD_ROW_*` | derived from `GRID_OY + ROWS*CELL` | **the fix for the overlap bug** — every row is an offset from one value |

### How to drive `beat_crawler` without a window

| Trigger | Effect | Exit codes |
|---|---|---|
| `BEAT_CRAWLER_SELFTEST=1 cargo run --example beat_crawler_game` | solvability + pathing + kick/blip discrimination through a **real device** | `0` pass **or skipped (no device)** · `1` stair unreachable · `2` bad enemy placement · `3` pathing did not approach · `4` kick never detected · `5` blip false-fired |
| `ENGINE_CAPTURE=<frame>:<path>[,…]` | engine-level (v0.134.0), **no example code**; headless PNG per listed frame | — |
| no audio device | `Audio::new()` → `None` → schedule clock; self-test SKIPs after the level checks | `0` |

**A no-device box must SKIP, not fail** — the rhythm cannot be exercised there and that is not a
regression. Same rule the parent established for `audio_reactive`.

### The three `cargo fmt --check` reflow sites (#389, gate run 3)

```
beat_crawler.rs:48   the `use engine::{…}` import list re-wrapped
beat_crawler.rs:176  `for step in [IVec2::new(1,0), …]` → one element per line
beat_crawler.rs:239  `let want = ENEMIES_PER_DEPTH[…]` → split across two lines
```

All three are the documented `[[cargo-fmt-reflow-trap]]` shape: hand-wrapped long lines that
rustfmt re-flows. Fix is always `cargo fmt` then re-verify — never hand-editing to match.

### Memory sequence mapping (this session)

| Seq | PR | Kind | Subject |
|---|---|---|---|
| 205 | #387 | DOCS | module map → `docs/MODULE_MAP.md`; the byte measurement |
| 206 | #388 | CODE v0.137.1 | `WindowConfig` survives a scene reset |
| 207 | #389 | CODE v0.138.0 | `beat_crawler` capstone |
| 208 | #390 | DOCS | verification copy-paste block + web-sys footnote |
| 209 | #391 | DOCS | scene-reset persistence pattern + deferred `Audio` risk |

`engine-current-state.md` 35.8 KB → ~42 KB. **Trim the tail into `[[engine-history-archive]]`
next session** — the read cap (~76 KB) has been hit once before.

### Local branch state at session end

| Branch | State |
|---|---|
| `main` | in sync at `b6135f2` |
| `docs/module-map-extraction`, `fix/window-config-survives-scene-reset`, `feat/beat-crawler-capstone`, `docs/verification-block-and-websys-footnote`, `docs/scene-reset-persistence-risk` | merged, deleted remotely by squash-merge; local refs remain (harmless) |
| **`docs/handoff-dm-adoption-seq4`** | **still the one local branch with NO `origin/` counterpart — deliberately NOT deleted** (carried from seq 3; deletion is not provably lossless) |

---

## Code Analysis

- **`App::new`** now ends with two `register_persistent` calls: `TextMeasurer` (pre-existing) and `WindowConfig` (new). `SceneTransition` is registered differently — inline in the struct literal's `persistent_resources: vec![...]` — which is why a naive grep for `register_persistent::<` misses it.
- **`App::reload_scene`** (`src/app/scenes.rs`) extracts persistent resources into type-erased boxes, builds a fresh `World`, re-runs `insert_core_resources` (which inserts `WindowConfig::default()`), re-applies event initializers and world registrars, then **re-inserts the preserved resources LAST** so they overwrite the engine defaults. That ordering is what makes the one-line fix sufficient.
- **`Crawler::detect_beat(&mut self, world, dt) -> bool`** — consumes a `scheduled_kick` flag via `std::mem::take`, reads `bands()` into a `[f32; 16]`, sums `[..4]`, and uses an **armed flag** rather than a pure refractory window: re-arm only once energy falls below `REARM_THRESHOLD` (0.5). A bare threshold test would fire several times per kick because the meter decays over the smoothing release rather than snapping to zero.
- **`BEAT_WATCHDOG = 1.2`** — the fallback is *inside* the audio branch, so a device that exists but produces nothing still recovers. The no-device branch returns the schedule directly. `heard` drives the HUD text so the player can tell which clock is running.
- **`Crawler::repaint_fog`** rewrites all 640 cell sprite colours from the FOV, and **skips any entity currently carrying a `HitFlash`** (`world.get::<HitFlash>(e).is_some()`). `HitFlash` snaps `Sprite.color`, lerps it back to the colour captured on its first run, and removes itself — so a game that also wrote colour every frame would fight it and the flash would never restore correctly.
- **`make_level(depth, seed)`** picks the generator by depth parity, derives `PathGrid` → `FovMap` through the same one-line bridge `roguelike` uses, then does all placement through `bfs_distances`. Enemies use a *second* `Rng` seeded `seed ^ 0x5eed_c0de` so enemy placement is reproducible but not correlated with the map stream.
- **`bfs_distances(grid, start) -> Vec<i32>`** — 4-connected BFS, `-1` for unreachable. This one function provides solvability checking, stair placement, minimum enemy distance, and the self-test's assertions.
- **Player/enemy `Sprite` colours are set once at spawn, never per frame** — a deliberate constraint imposed by `HitFlash` ownership, and commented as such at the spawn site.
- **`Transform.position` is the quad centre** (`VERTICES` span `-0.5..0.5` in `renderer/sprite/geometry.rs`) and **`Camera.position` is the viewport top-left** — together these let the grid be laid out in world coordinates that read exactly like screen coordinates.
- **`ENGINE_CAPTURE` / `save_screenshot_headless` read the render size from `WindowConfig`** with `.unwrap_or((1280, 720))` at three sites in `src/app/headless.rs` — that `unwrap_or` is what turned the lost resource into a silently wrong capture size rather than an error.
- **`Crawler::tick_beat` is the whole turn**: increment beat count, set the flash timer, resolve the queued player step (with the on-beat bonus decided by `pending_age <= ON_BEAT_WINDOW`), then run every enemy, then mark the fog dirty. Input is *queued* on press and *resolved* on the beat, which is what makes the game rhythmic rather than real-time.
- **`Crawler::enemy_turn` gates on `fov.is_visible(enemy)`** — the same FOV that draws the fog. An enemy you cannot see does not act, so the AI can never chase you through a wall you have not seen it through. It also collects `occupied` cells up front and refuses to step onto the player or another monster, since `PathGrid` knows nothing about actors.
- **`descend` takes `&mut World` it does not use** (`let _ = world;`) — kept in the signature deliberately so the call site reads symmetrically with the other world-mutating beat actions, and so adding entity work there later needs no signature change.
- **`needs_spawn` is checked twice per frame** — once at the top and once after `tick_beat` — because a descent queued *inside* the beat must rebuild before anything draws. Without the second check the frame would paint fog over a level that no longer exists.
- **The cell grid is created once and only recoloured**; only actors are despawned and respawned on descent. `spawn_entities` is idempotent for cells (`if self.cells.is_empty()`), which is what lets it serve both first-run init and every later descent.
- **`Sprite.color.a` is used to hide the stair and out-of-sight monsters**, not despawn/respawn — so a `HitFlash` running on an enemy is never interrupted by visibility changes (the repaint skips flashing entities entirely).
- **`self_test()` deliberately does NOT go through `App`.** It builds `Level`s directly and drives `audio.update(0.016)` on a `thread::sleep(16ms)` cadence, reproducing a game's frame timing against real audio time. A headless `App` loop runs as fast as it can while the device advances on the wall clock, so any frame-count-based audio assertion would be timing noise. Same reasoning the parent recorded for `audio_reactive`'s self-test.

---

## Files Changed

### Source code
- `src/app.rs` — `register_persistent::<WindowConfig>()` in `App::new` with a comment naming both prior workarounds; new `window_config_survives_a_scene_reset` test in the existing `mod tests`.

### Examples (the acceptance test)
- `examples/games/beat_crawler/beat_crawler.rs` — **NEW (961)**. `Step`/`PATTERN` soundtrack, `Level`/`make_level`, `bfs_distances`, `Enemy`, `Crawler` system (input → soundtrack → detect → beat → fog → HUD), `CrawlScene`, `self_test()`, `main` with the `BEAT_CRAWLER_SELFTEST` branch.

### Docs
- **`docs/MODULE_MAP.md`** — **NEW (92)**. Header + all 72 rows, byte-identical to what left CLAUDE.md; plus a `beat_crawler` row added in #389.
- `CLAUDE.md` — module map replaced by a pointer; Documentation-rules byte measurement; Document-map row; Core-patterns scene-reset clause; header v1.6.235 → **v1.6.239**, package → v0.138.0.
- `docs/PATTERNS.md` — "Surviving a scene reset" section (+49) and the web-sys footnote (+7).
- `docs/VERIFICATION.md` — Trap 4 copy-paste block + corroboration step (+23).
- `FORKING.md` — repo-layout row and further-reading entry point at `docs/MODULE_MAP.md`.
- `docs/CHANGELOG.md` — 0.137.1 and 0.138.0 entries.
- `Cargo.toml` / `Cargo.lock` — 0.137.0 → 0.137.1 → 0.138.0; `[[example]] beat_crawler_game`.

### Local tooling (gitignored — memory is the only record)
- `.claude/skills/{ship,split-module,add-feature-example,add-ui-widget}/SKILL.md` — all four repointed from "CLAUDE.md module map" to `docs/MODULE_MAP.md`; `split-module` and `add-feature-example` also lost the now-inapplicable "keep CLAUDE.md ≤200 lines" instruction for map edits.

### Memory (not in any PR)
- `engine-current-state.md` — seq 205/206/207/208/209 prepended; menu line updated; the `set_scene` gotcha amended to note `WindowConfig` is now exempt; **new STANDING/DEFERRED block** for the `Audio` risk with reopen triggers. 35.8 KB → ~42 KB.
- `local-tooling-skills.md` — 2026-07-30 section recording the four skill edits.
- `MEMORY.md` — index hook refreshed twice.

---

## User Feedback & Preferences (REQUIRED)

- **The opening instruction was a paste prompt** demanding: read the plan and handoff, execute Phase 1, *"This is not optional and it is READ-ONLY"* for the board gate, read the exit code from the FILE, *"PHASE 1 ENDS IN A QUESTION, NOT CODE"*, and *"Do NOT self-pick the direction."* Followed literally; no code before the answer.
- The prompt also said: *"Give the recommendation reasoning, not just labels — the user took the recommendation three times out of three last session."* Honoured — each option carried its trade-off, and I flagged where my own recommendation had **changed** from the plan's assumption because of the byte measurement.
- **"모듈 맵 분리 (추천)"** and **"2번째 캡스톤 게임 (추천)"** — both recommendations taken, in one round-trip. **Now 5 for 5 across two sessions.** The framing (what breaks otherwise / what the cost actually is) is doing real work — keep it.
- **"리스크 문서화 해서 남겨두고 다음에 문제가 생기면 작업하도록 해줘"** — *document the risk and leave it; work on it if a problem comes up later.* Given after I flagged the `Audio`-persistence footgun unprompted at the end of my report. **Calibration: the user wants known-but-unproven risks recorded with a trigger, not fixed speculatively, and not silently dropped.** This is the same instinct as the standing anti-goal against building on speculation.
- **The user reads the closing report and acts on what it surfaces.** My final paragraph raised the `Audio` question as an open judgement call; the very next message was a decision about it. Ending a report with a genuine open question is productive here — but only when it is real, not decorative.
- **Merge authority remains standing-delegated** — squash on green CI, async auto-merge, no per-PR confirmation. Exercised on all five PRs.
- **Korean to the user, English in artifacts** — every report and question in Korean; code, comments, commit messages, PR bodies, CHANGELOG, docs and this handoff in English.
- **No mid-session course corrections.** After the single two-part answer, the session ran design → implement → verify → ship → PR → merge → memory unassisted through four PRs.
- **Scope discipline in both directions is expected.** Out-of-scope-but-necessary work was done *and reported as such* (the engine fix, split into its own PR; the four skill updates). Conversely, `AGENTS.md` and the historical `plans/**` references were deliberately left alone and that was stated.
- **Corrections are expected plain and immediate.** Corrected my own "73 rows" to 72 mid-session, and corrected my own false-green clippy pipeline, both without ceremony.

---

## Where We're Going

1. **Board gate FIRST, every session** — `../dungeon-merchant/docs/engine-wishlist.md` (next free **EW-012**; EW-001–011 closed and archived) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`). A newly filed request preempts everything below.
2. **If both are empty (likely): ASK — do NOT self-pick.** The menu, with the completed items removed:
   - **Adopt audio-reactive in `examples/games/survivor`** as a *third* consumer. `beat_crawler` is now a second consumer, but it was written by the same session that had just read the API. `survivor` is older code. **Expect to hit `play_sfx`'s unmeterable 16-voice round-robin immediately** — that is the interesting outcome, and the verdict (game moves to a named channel vs real API gap) is the deliverable.
   - **A fourth procgen mode** (drunkard's-walk / room-accretion / Voronoi) — still the safest and lowest marginal value; rooms/caves/mazes plus `roguelike` and now `beat_crawler` cover the area well.
   - **A third capstone or a deeper pass on `beat_crawler`** (meta-progression, items, a real soundtrack rather than synthesized tones) — only if the user wants to keep pulling this thread.
   - **The deferred `MapGenerator` trait** — still an anti-goal; its trigger has not fired.
3. **This handoff + plan land as their own `docs(handoff)` PR** (repo convention), chain `board-ew-triple` seq 4. Bump memory to **seq 210** after it merges — and note the recorded tip will be one commit stale, as it has been for every handoff in this chain.
4. **Consider starting a fresh chain tag.** `board-ew-triple` has been historical since seq 2 and the arc it was continuing (audio-reactive → capstone → cap resolution) has now closed. A new tag would be honest.
5. **`engine-current-state` memory is ~42 KB and the trim threshold is near.** The plan's risk note said trim around seq 210; we are at 209. **Trim the chain tail into `[[engine-history-archive]]` next session**, before it hits the ~76 KB read cap it has already hit once.

---

## Risks & Blockers

- **Verification tooling that lies is still the dominant risk, and it has now fired in three consecutive sessions.** #390 upgraded the mitigation from prose to a copy-paste block; whether that is enough is unproven. Always `rm -f` the exit file, run non-piped, read the file, check its mtime, and corroborate the counts (**152 groups / 1339 lib tests** at v0.138.0).
- **Your own shell pipelines can produce false greens too** — this session, `cargo clippy … | grep … | head -30` reported exit 0 independently of the gate. The rule is not just about `verify.sh`.
- **Stale rust-analyzer diagnostics repeatedly claimed compile errors that did not exist** (mid-edit captures). Trust `cargo build` / `cargo clippy` exit codes, never the IDE panel. This bit twice this session and once in the parent.
- **The verify gate still excludes examples for wasm.** `beat_crawler` is native-only by design and claims no web support, so nothing is owed there — but a future web claim needs `cargo build --example beat_crawler_game --target wasm32-unknown-unknown` plus a smoke.
- **`engine-current-state` memory is ~42 KB**, growing ~1.5 KB/seq against a ~76 KB read cap it has hit once already. Trim next session.
- **`Audio` is not auto-persisted** — a game that forgets `register_persistent::<Audio>()` loses its device silently on the first scene change. Documented with reopen triggers in `docs/PATTERNS.md`; deliberately not fixed (user ruling).
- **`core.fileMode = false` will hide the next mode change.** No new scripts this session, so the trap was not exercised — but any new `.sh` is a fresh chance to get it wrong.
- **`dungeon-merchant` has no CI/branch protection** — its board is discipline, not enforcement. `rust-survivors` is paused/deprecated.

---

## Open Questions

- **Should `Audio` be auto-persisted after all?** Deliberately deferred by user ruling. Reopen if a third game hits it, a bug report traces to it, or another engine-inserted config type turns out to be dropped the way `WindowConfig` was.
- **Are there other engine-inserted config resources silently reverting on scene reset?** `WindowConfig` was found by accident. Nobody has audited `insert_core_resources` against "which of these would a game set once and expect to keep". That audit is cheap and has not been done.
- **Should `bands()` offer a caller-selectable FFT size?** Still nothing has asked — and notably `beat_crawler` did **not** need it; 16 bands with the low 4 summed was ample. Weaker case than before.
- **Should `embedded_image` get a web harness?** Carried unanswered from seq 2 and seq 3.
- **Is the `add-facade-capability` skill worth writing?** Still deferred at n=2.
- **Why did `Render tests (lavapipe)` take 10m34s on #382?** Did not recur across five more PRs (56–80 s). Almost certainly runner variance; closeable.

---

## Quick Start for Next Session

```bash
# Nothing is dangling — all five PRs merged, main is clean at b6135f2 (v0.138.0).

cd ~/Projects/skeleton-engine
git log --oneline -5      # expect b6135f2 at the tip (or the handoff merge above it)
git status -s             # expect clean

# 1. READ THE BOARD FIRST — both channels (a new request preempts everything)
#   ../dungeon-merchant/docs/engine-wishlist.md        (next free EW-012; EW-001-011 all
#                                                       Verified + archived, NO open requests)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md   (currently _None._)
cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md && cd -
# ^ if later than 2026-07-27, the board MOVED — read it before anything else

# 2. Verify starting state — the copy-paste block #390 added, now in docs/VERIFICATION.md.
#    Read the exit code from the FILE and check its mtime. The notification lied again
#    this session (said 0, file held 1).
rm -f /tmp/v.exit /tmp/v.log
(./scripts/verify.sh > /tmp/v.log 2>&1; echo $? > /tmp/v.exit) &
until [ -f /tmp/v.exit ]; do sleep 20; done
echo "EXIT=$(cat /tmp/v.exit)"; ls -l /tmp/v.exit
echo "ok groups: $(grep -c 'test result: ok' /tmp/v.log)"
# expect 0 · 152 ok groups · 1339 lib tests

# 3. Re-prove this session's work
BEAT_CRAWLER_SELFTEST=1 cargo run --example beat_crawler_game
# expect: 6 depths ok, pathing approaches, kick 4.00 / blip 0.61, PASS, exit 0
ENGINE_CAPTURE=200:/tmp/beat.png cargo run --example beat_crawler_game   # 864x660, eyeball it

# 4. Key files
#   docs/MODULE_MAP.md      — GREP IT, don't read it whole (72 rows, ~89 KB)
#   CLAUDE.md               — now 137 lines; conventions + the gate only
#   docs/VERIFICATION.md    — 6 traps + the new copy-paste block
#   docs/PATTERNS.md        — "Surviving a scene reset" is the newest section
#   examples/games/beat_crawler/beat_crawler.rs   — the capstone

# 5. FIRST ACTION: board gate → if empty, ASK for direction. Do NOT self-pick.
#    Menu: adopt audio-reactive in `survivor` as a 3rd consumer (expect play_sfx's
#    unmeterable round-robin — that verdict IS the deliverable) / a 4th procgen mode /
#    a deeper pass on beat_crawler. Windowed capture remains OFF the menu.
#    ALSO: trim engine-current-state (~42 KB) into engine-history-archive.
```
