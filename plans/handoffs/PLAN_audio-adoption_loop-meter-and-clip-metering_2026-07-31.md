# Next: board gate, clear the overdue memory trim, decide the silent-system-drop question — then give the new API its game

**Date:** 2026-07-31
**Status:** PLANNED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `audio-adoption` seq `3`
**Context:** See `HANDOFF_audio-adoption_loop-meter-and-clip-metering_2026-07-31.md` for session data, all measurements, the four PRs and the verify-gate history (including the red one).

---

## Problem Statement

This session shipped four PRs and closed two of the chain's long-standing questions: `Audio` is auto-persisted (v0.141.1) and `play_sfx_metered` exists (v0.143.0). It also found two bugs nobody was looking for — metering a **looping** sound died after one pass (v0.141.2), and `beat_crawler`'s `AudioFacadeSystem` **had never been running**, so the example's headline feature ran on a watchdog fallback for several releases with a HUD string as its only symptom.

Three things are now outstanding. **`play_sfx_metered` has no game.** Its only exerciser is `examples/audio_facade`, a surface demo — and `CLAUDE.md`'s VISION rule is explicit that a feature is not done until a small playable example game exercises it in real play. That is exactly the v0.140.0 → v0.141.0 shape, one release behind.

**The memory trim is overdue.** The parent said "~seq 220" and we are at seq 220; `engine-current-state.md` is ~48 KB against the ~76 KB working cap that forced the last two trims.

**And a silent-failure class is now proven, not theoretical.** `app.add_system(X)` before `set_scene` is discarded without a word. It cost this session an investigation and cost `beat_crawler` several releases. Whether the engine should guard it is a design call with a real trade-off.

Both board channels are empty (next free **EW-012**, unmoved since 2026-07-27), so there is no externally-driven work and the standing rule is to ASK. See Evidence & Data in the handoff for the numbers behind all of this.

## Key Findings

- **`play_sfx_metered` shipped without a game example.** `audio_facade` proves the surface, not real play. VISION says the example is the acceptance test. → drives Phase 4A.
- **`beat_crawler` currently has NO sound effects at all** — attacks are silent. A metered hit clip is a genuine addition rather than churn, and it already has an asset pipeline (`assets/soundtrack.py`). → drives Phase 4A's target choice.
- **⚠️ But metering may not pay there, and that is a legitimate outcome.** Seq 1's finding #3: metering only pays when the sound carries information the game does not already hold. If the meter recovers a constant, saying so is the deliverable. → drives Phase 4A's stop condition.
- **A system registered before `set_scene` is silently dropped.** `SceneCmd::Replace` swaps the whole systems list; `src/app/scenes.rs` says so in a comment, and nothing enforces it. Proven expensive this session. → drives Phase 3.
- **`survivor` still has no self-test**, three sessions running — and `beat_crawler` just demonstrated the exact failure mode it would prevent. → drives Phase 4B.
- **The verify gate went RED (exit 101) on a rustdoc lint** while fmt, clippy, wasm and all 1360 tests were green. A `| tail` would have hidden it. → drives the gate discipline in every phase.
- **A headless `ENGINE_CAPTURE` run cannot verify metering** (fixed `1/60` dt outruns real audio). Documented on `Audio::levels` and module-map row 79 after costing two investigations. → drives Phase 4's validation method.
- **The user took the recommendation 9 of 9 via `AskUserQuestion`, then picked the *second*-ranked item as a direct instruction** once the closing report argued its priority had risen. Framing moves the answer more than ranking does. → drives Phase 1's presentation.

## Anti-Goals (What NOT To Do)

- **Do NOT self-pick the next feature.** Both channels are empty; Phase 1 ends in questions. This shape has now worked six sessions running.
- **Do NOT re-run the `insert_core_resources` audit.** All 27 are classified in writing in `docs/PATTERNS.md`, including the 20 that are correctly scene state.
- **Do NOT reopen the `Audio` auto-persistence decision.** It is DECIDED (v0.141.1) and its *next* trigger is written down: a game that wants a per-scene device teardown, which would get an opt-out flag, not a revert.
- **Do NOT verify any metering change from a headless `ENGINE_CAPTURE` PNG.** It cannot work — fixed dt outruns real audio. Use a real-time loop paced off `Instant` (the self-tests do this).
- **Do NOT re-derive the beat detector's constants.** `LOW_BANDS = 2`, `KICK_THRESHOLD = 1.6` (plateau 1.45–1.95), `KICK_COOLDOWN = 0.40` are measured over 7 bars and recorded with their sweep tables.
- **Do NOT let `beat_crawler` become a third capstone.** It gained a track this session and lost a broken detector. One small addition, or none.
- **Do NOT migrate `survivor`'s bullet tone** to anything. Settled by arithmetic two sessions ago: 0.04 s of tone every 0.14 s never overlaps.
- **Do NOT attempt a windowed screenshot playtest.** Tried twice this session — `screencapture` returns black (permissions) and `osascript` cannot find the window. Closed, not deferred.
- **Do NOT trust a `run_in_background` completion notification's exit code.** Six sessions. Use the block in `docs/VERIFICATION.md`.

## Plan

### Phase 1: Board gate, then ask — one decision and one build item

**Goal:** Establish whether external work exists and, if not, get an explicit direction.

**Why this approach:** The board has preempted self-picked work every time it was non-empty, and asking has produced better choices than my own ranking for six sessions. Phase 3's question is different in kind from the build menu — it is an engine-behaviour trade-off (safety vs. log noise) that the user owns.

- Read `../dungeon-merchant/docs/engine-wishlist.md` — **Active requests** (expected: empty, next free **EW-012**). Extract with `awk '/^## Active requests/,/^## Done \/ archive/'`; do **not** `cat` the 53 KB file.
- Read `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — **Open Requests** (expected `_None._`).
- Run `cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md`. Later than **2026-07-27** means the board moved — read it before anything else. **A filed request preempts every phase below.**
- Verify the starting state with the copy-paste block in `docs/VERIFICATION.md`. Read the exit code **from the file**; expect **0 · 152 ok groups · 1360 lib tests** at v0.143.0.
- If a request exists: triage, acknowledge on the board with an append-only `[Engine]` line and an absolute date, and defer Phases 3–4.
- If both are empty: ask **one** `AskUserQuestion` (Korean) with **two** questions. **Q1 = the silent-system-drop decision** (Phase 3's options). **Q2 = the build menu** (Phase 4's branches, with reasoning).
- Carry the reasoning, not just labels, and recommend explicitly — the handoff records that framing moves the answer more than ranking does.

**Files:** none (read-only)
**Validates with:** both board files read; a stated verdict; either a filed request or the user's two recorded answers.
**Rollback:** n/a — no mutations.

### Phase 2: Trim `engine-current-state` memory (MANDATORY, do it before building)

**Goal:** Get the memory file back under its working size before it hits the Edit-tool cap mid-session.

**Why this approach:** The parent set the trigger at "~seq 220" and we are at seq 220. Both previous trims happened *after* the file had already become painful to edit (76 KB → 45.5 KB in 2026-07-30). Doing it first, while context is fresh, costs minutes; doing it mid-build costs a failed Edit and a recovery.

- Read the trim procedure from `[[engine-history-archive]]`'s body — it records the exact shape of the last three trims (tip-line tail seqs move out, body bullets move out, a one-line pointer stays).
- Move the **oldest tip-line seq entries** (currently reaching back to ~seq 186) into `engine-history-archive.md`, targeting roughly seqs 186–200, and leave the archive's pointer line updated with the new range and reason.
- **Guard every replacement** with a Python `assert s.count(old) == 1` — this is the procedure that caught two silent no-ops in the parent session. Back both files up to the scratchpad first; they are not version-controlled.
- Target: `engine-current-state.md` back under ~35 KB. Verify with `wc -c` before and after, and re-read the tip line to confirm it still parses as one coherent chain.
- Update `MEMORY.md`'s pointer for `[[engine-history-archive]]` with the new trimmed range.

**Files:** `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md`, `engine-history-archive.md`, `MEMORY.md`
**Validates with:** `wc -c` shows a real reduction; the tip line still reads seq 220 → … contiguously; no seq is lost between the two files.
**Rollback:** restore from the scratchpad backups.

### Phase 3: Resolve the silent-system-drop question

**Goal:** Decide whether the engine should tell a game that `set_scene` just discarded its systems.

**Why this approach:** This is not speculative any more. `beat_crawler` registered `AudioFacadeSystem` one line too early and its headline feature — the whole premise of the example — was dead for several releases, with a HUD string as the only symptom. The trade-off is real in both directions, which is why it is a question rather than a build item.

Two outcomes, both bounded:

- **If the user says warn:** in `apply_scene_cmd`'s Replace path, count the systems being discarded that are **not** builtins and, when that count is non-zero, `log::warn!` once naming the count and pointing at the fix ("register scene systems in `Scene::on_enter`, or add them after `set_scene`"). Gate it to `debug_assertions` so a release build pays nothing. Add a test that constructs an `App`, adds a system, calls `set_scene`, and asserts the system count dropped — the *observable* fact, since capturing the log is not worth the plumbing. Ship as **PATCH**. ⚠️ Check first whether any shipped example legitimately registers before `set_scene` and would now warn — `scene_flow` and `settings_menu` are the scene-using examples to check.
- **If the user says leave it:** no code. Add the trap to `docs/PATTERNS.md`'s scene section as a named footgun with the `beat_crawler` case as its evidence, and to the "Add a system" task recipe. Ship as a docs-only PR, no bump.
- Either way, record the outcome in the `[[engine-current-state]]` memory so the next session does not re-litigate it.

**Files:** `docs/PATTERNS.md` always; `src/app/scenes.rs`, `src/app.rs` (test), `docs/CHANGELOG.md`, `CLAUDE.md` only in the warn branch.
**Validates with:** if code changed, a test that fails without the fix and `./scripts/verify.sh` exit 0 with lib tests ≥ 1360; if docs only, the trap is findable by grepping `docs/PATTERNS.md` for `set_scene`.
**Rollback:** the code branch is one guarded `log::warn!` plus one test; the docs branch is a block revert.

### Phase 4: Build the chosen menu item

Three mutually exclusive branches. **Only the one the user picks gets built.**

#### 4A — Give `play_sfx_metered` its game: `beat_crawler` hit sounds (recommended)

**Goal:** Satisfy the VISION acceptance test for v0.143.0 — a playable game exercising the new API in real play.

**Why this approach:** v0.143.0's only exerciser is a surface demo. `beat_crawler` is the right target for three reasons: it currently has **no SFX at all** (so this is an addition, not churn), it already has a CC0 asset pipeline, and its combat genuinely produces **overlapping** hits — the player attacks on the beat while enemies attack back on the same beat, which is exactly the case a metered one-shot exists for.

- Add a short percussive hit clip to `assets/`, generated by extending `soundtrack.py` with a `--hit` mode (or a sibling function) so the CC0 provenance story stays one script. Keep it PCM and short (~0.15–0.25 s).
- Fire it with `play_sfx_metered(HIT_METER, HIT_CLIP, "sfx")` on **both** the player's attack and each enemy's attack, so several can land in one beat and overlap.
- `enable_analysis(HIT_METER)` at setup, **before the first play** — the meter is wired in as a sound starts.
- Drive one existing effect from the summed meter rather than adding a new one: `Camera::shake` magnitude or the `flash` intensity, keyed to "how much violence landed this beat".
- ⚠️ **The polyphonic normalisation trap applies.** Any constant normalised against a single voice's maximum is wrong once voices sum, and it shows as a feel regression rather than an error. Base the mapping on the summed ceiling, as `survivor`'s `KILL_PEAK_FULL` does.
- ⚠️ **STOP CONDITION — and this outcome is a legitimate deliverable.** Seq 1's finding #3: metering only pays when the sound carries information the game does not already hold. Measure the drive distribution on a real device; **if the meter recovers a near-constant** (because the game already knows how many hits landed), say so in writing, revert to a plain `play_sfx`, and report that `beat_crawler` was the wrong host. Do not tune around it.
- Keep `BEAT_CRAWLER_SELFTEST=1` green and extend it with a hit-audio assertion **only if** the measurement justified keeping the meter.
- Verify in **real time**, never from a capture — a headless loop paced off `Instant`, like the existing self-test.

**Files:** `examples/games/beat_crawler/beat_crawler.rs`, `examples/games/beat_crawler/assets/soundtrack.py` + a hit `.wav` + `assets/README.md`, `docs/MODULE_MAP.md` row 91, `docs/CHANGELOG.md`
**Validates with:** `BEAT_CRAWLER_SELFTEST=1` exits 0; a written drive distribution from a real device showing the meter spans a range rather than a constant; `./scripts/verify.sh` exit 0, lib tests ≥ 1360.
**Rollback:** additive — remove the clip, the meter and the drive; the turn clock is untouched.

#### 4B — `SURVIVOR_SELFTEST=1`

**Goal:** Make `survivor`'s audio-reactive path regression-proof.

**Why this approach:** Carried unbuilt from seq 1 and seq 2, and **materially sharper now**: `beat_crawler` just proved an example's headline feature can be dead for releases with only a HUD string as the symptom. `survivor` has the same exposure and no self-test at all.

- Follow the `BEAT_CRAWLER_SELFTEST` precedent exactly, including **no device = SKIP, not fail**, which is what lets it exist without breaking CI.
- Pace off `Instant`, not an accumulator — the handoff records that `t += 1.0/60.0` made a correct detector look 40% wrong.
- Assert what seq 2 measured by hand: the metered peak moves above `KILL_PEAK_OFF` while kills land; the watchdog engages when `enable_analysis` is skipped; the drive does not sit pinned at 1.0.
- Reuse the input-script approach (`ENGINE_INPUT`) rather than driving the game by hand.
- Also assert the wiring lesson from this session: that the systems `survivor` needs are actually registered after any scene setup.

**Files:** `examples/games/survivor/survivor.rs`, `docs/CHANGELOG.md`, `docs/MODULE_MAP.md` row 79
**Validates with:** `SURVIVOR_SELFTEST=1` exits 0 with a device and SKIPs without one; `./scripts/verify.sh` exit 0.
**Rollback:** additive — remove the mode.

#### 4C — A fourth procgen mode (drunkard's walk)

**Goal:** Add one more generator over the shared `DungeonMap`.

**Why this approach:** Proven three times, composes with `to_path_grid` / `to_tilemap_tiles` / `FovMap` for free. Ranks last: rooms/caves/mazes plus `roguelike` and `beat_crawler` already cover this area, so a fourth teaches least.

- Mirror `generate_cellular_cave`'s signature exactly: `generate_drunkard_walk(w, h, seed, &DrunkardParams) -> DungeonMap`; params for target floor fraction + max steps; cap cells at `MAX_PATH_GRID_CELLS`.
- Record a 1×1 spawn `Room` at the walk's start so `first_room_center` works — `beat_crawler` proved why (room-based placement degenerates when a generator records one room).
- Tests: determinism, connectivity (single flood-filled region), floor fraction within tolerance, over-cap → empty + `error!`.
- Example `drunkard_walk` following `maze_generation`'s shape (WASD, R regenerate, headless connectivity self-check).

**Files:** `src/mapgen.rs`, `src/lib.rs`, `examples/drunkard_walk.rs`, `docs/MODULE_MAP.md`, `docs/CHANGELOG.md`
**Validates with:** 4+ unit tests incl. a single-connected-region assertion; `./scripts/verify.sh` exit 0.
**Rollback:** purely additive.

### Phase 5 (conditional): land each engine change as its own PR

**Goal:** Keep public-API or engine-behaviour changes separable from the examples that motivate them.

**Why this approach:** #388 established it and this session used it twice more, producing four independently-revertible PRs. **Phase 3's warn branch is the only one here that can produce an engine change**; 4A, 4B and 4C are additive example work.

- Version per `CLAUDE.md`: MINOR for an additive API, PATCH for a pure bugfix. Use `/ship` for the four-file paperwork, `/land-pr` for the branch → CI → merge loop.
- **If two PRs stack,** split with the stash → branch → rebase procedure in the handoff's Reusable Procedures, and finish with `git diff <pre> <post> --stat` returning empty to prove the rebased tree still matches the one the gate passed on.
- Default to async auto-merge (`gh pr merge <n> --auto --squash`) — but **not** for anything whose behaviour only a real device can confirm. Get that confirmation before arming.

**Files:** as per the branch
**Validates with:** each PR green independently; `./scripts/verify.sh` exit 0 before each push.
**Rollback:** revert the engine PR without touching the example PR.

## Dependencies & Order

- **Phase 1 gates everything.** A filed board request preempts Phases 2–4 entirely.
- **Phase 2 (memory trim) runs before any build work**, regardless of Q2's answer. It is small, it is overdue, and doing it mid-build is how the last two trims became painful.
- **Phase 3 runs before Phase 4** — it is bounded either way, and if it takes the warn branch it may change what a Phase-4 example needs to do.
- **Phase 4's branches are mutually exclusive. Do not build two** unless the user asks to combine. 4A and 4C is the only clean pair (no file overlap); 4B touches audio-adjacent code that 4A also touches.
- **Phase 5 only fires if Phase 3 took its code branch.**

## Risks & Mitigations

- **4A's meter turning out to recover a constant.** Moderate — the game already knows how many hits landed. Mitigation: this is written into 4A as a *legitimate outcome*, not a failure; measure the distribution before committing to the design, and report a revert-to-`play_sfx` as the finding.
- **Self-picking instead of asking.** Likely if the next session reads "execute Phase 1" as "start coding". Mitigation: Phase 1 has no files and no code; its deliverable is two recorded answers.
- **Treating Phase 2 as optional.** The most likely failure of this plan — it is housekeeping-shaped and easy to skip. Mitigation: it is listed as mandatory, it has a fired trigger, and its success criterion is a `wc -c` number.
- **Verifying audio from a headless capture.** Two sessions have now tried. Mitigation: it is documented on `Audio::levels` and module-map row 79, and named in this plan's anti-goals — pace off `Instant` instead.
- **A false-green gate.** Did not fire this session because the `docs/VERIFICATION.md` block was used from the first launch — and run 5 was genuinely red, which the file caught. Mitigation: keep using the block; corroborate **152 groups / 1360 lib tests** at v0.143.0.
- **A red gate after the test step.** Happened this session (rustdoc, exit 101) with everything else green. Mitigation: `grep -nE '^error' /tmp/v.log` rather than scrolling, and re-run only the failing step before paying for the full gate.
- **The `soundtrack.wav` / `PATTERN` desync.** `soundtrack.py` mirrors `PATTERN` by hand and nothing enforces it. Mitigation: if 4A touches the generator, re-run `BEAT_CRAWLER_SELFTEST` — an off-grid result is the symptom.
- **`cargo fmt` reflow defeating a scripted edit.** Unchanged. Mitigation: run `cargo fmt` before the first verify, and verify any scripted deletion with an independent `grep`, never with the script's own exit code.

## Success Criteria

- **Minimum viable:** the board gate is run and stated on both channels; both questions are asked (or a filed request is triaged); nothing is self-picked.
- **Phase 2 is done and `engine-current-state.md` is measurably smaller** (`wc -c` before/after), with no seq lost between it and the archive — this is the non-negotiable housekeeping outcome.
- **Phase 3 is resolved and recorded**, so the silent-system-drop trap is either guarded in code or named in `docs/PATTERNS.md` — not left as a session anecdote.
- The chosen Phase 4 branch ships with `./scripts/verify.sh` exit **0**, `ok` groups **≥ 152**, and lib tests **≥ 1360** — never fewer.
- **If 4A:** a real-device drive distribution in writing, and either a metered hit that spans a range or an explicit finding that it recovers a constant plus a revert to `play_sfx`. `BEAT_CRAWLER_SELFTEST=1` still exits 0 either way.
- **If 4B:** `SURVIVOR_SELFTEST=1` exits 0 with a device and SKIPs without one.
- **If 4C:** 4+ unit tests including a single-connected-region assertion, plus a playable example.
- Memory advanced to at least **seq 221** with the recorded `main @ <hash>`, `MEMORY.md`'s hook refreshed, and Phase 3's outcome recorded. The recorded tip will be one commit stale if the handoff lands as its own PR, as it always has.

## Quick Start

```bash
# Restore full context
cat plans/handoffs/HANDOFF_audio-adoption_loop-meter-and-clip-metering_2026-07-31.md

# 1. BOARD FIRST — this is Phase 1 and it is not optional
awk '/^## Active requests/,/^## Done \/ archive/' ../dungeon-merchant/docs/engine-wishlist.md
grep -A3 '^## Open Requests' ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md
cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md && cd -
# ^ later than 2026-07-27 = the board MOVED; read it before anything else

# 2. Verify starting state — the block docs/VERIFICATION.md carries.
#    Read the exit code from the FILE. It went RED (101) last session and the file is why we saw it.
rm -f /tmp/v.exit /tmp/v.log
(./scripts/verify.sh > /tmp/v.log 2>&1; echo $? > /tmp/v.exit) &
until [ -f /tmp/v.exit ]; do sleep 20; done
echo "EXIT=$(cat /tmp/v.exit)"; ls -l /tmp/v.exit
echo "ok groups: $(grep -c 'test result: ok' /tmp/v.log)"
# expect 0 · 152 ok groups · 1360 lib tests   (v0.143.0)

# 3. Key files for the phases
#   ~/.claude/.../memory/engine-current-state.md  — the OVERDUE trim                      [Phase 2]
#   src/app/scenes.rs                             — apply_scene_cmd / Replace drops systems [Phase 3]
#   examples/games/beat_crawler/beat_crawler.rs   — no SFX today; CrawlScene::on_enter      [4A]
#   examples/games/beat_crawler/assets/soundtrack.py — the CC0 generator to extend          [4A]
#   src/audio_facade.rs                           — play_sfx_metered + the levels() warning [4A]
#   examples/games/survivor/survivor.rs           — has NO self-test                        [4B]
#   docs/MODULE_MAP.md                            — GREP IT (rows 79, 91 rewritten)

# 4. Real-device sanity check (CI cannot do this; a capture CANNOT verify a meter)
BEAT_CRAWLER_SELFTEST=1 cargo run --release --example beat_crawler_game
# expect exit 0 and "16 kicks ... 0.638s ... on-grid 15/15"

# 5. FIRST CONCRETE ACTION
#    Read both board files and state the verdict. If empty, ask ONE AskUserQuestion (Korean)
#    with TWO questions:
#      Q1 (decision): should `App` warn when `set_scene` discards systems registered before it?
#         Its trigger fired — beat_crawler's headline feature was dead for releases with only a
#         HUD string as the symptom. Options: debug-build `log::warn!` + a test / document the
#         footgun in docs/PATTERNS.md and leave the behaviour alone.
#      Q2 (build menu, with reasoning): a GAME example for play_sfx_metered — beat_crawler hit
#         sounds (RECOMMENDED: v0.143.0 has no game exerciser, VISION says the example IS the
#         acceptance test, and beat_crawler has no SFX at all today) / SURVIVOR_SELFTEST=1 /
#         4th procgen mode.
#    Do NOT self-pick. Phase 1 ends in a question, not code.
```
