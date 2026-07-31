# Next: board gate, then close the one question this session left open — and only then build

**Date:** 2026-07-31
**Status:** PLANNED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `audio-adoption` seq `2`
**Context:** See `HANDOFF_audio-adoption_metered-oneshot-and-config-audit_2026-07-31.md` for session data, all measurements, the three PRs and the verify-gate history.

---

## Problem Statement

This session closed the `audio-adoption` arc: the gap seq 1 measured became `Audio::play_tone_metered` (v0.140.0), and `survivor` adopted it (v0.141.0) with real-device proof — 22 of 25 kill-tone replays had been cutting a still-sounding tone, and the metered peak went 0.6000 → 1.0000 once voices could overlap. Alongside it, the twice-deferred `insert_core_resources` audit found **seven** config resources silently reverting on a scene reset (v0.139.1), each proven by a test that failed before its fix.

That audit had a side effect the next session must not inherit quietly. **It fired the reopen trigger on the deferred `Audio` auto-persistence decision** — the trigger's own wording was "another engine-inserted config type turns out to be dropped the way `WindowConfig` was", and seven were. Worse for the original argument, four of the seven are inserted by the *game*, so "the game inserted it, so it is the game's job" — the exact reasoning that deferred `Audio` — is no longer the line. `docs/PATTERNS.md` now records the trigger as fired and the remaining argument as thinner. **A shipped doc currently states a live, unowned question.**

Meanwhile both board channels are empty again (next free **EW-012**, unmoved since 2026-07-27), so there is still no externally-driven work and the standing rule is to ASK. See Evidence & Data in the handoff for the numbers behind all of this.

## Key Findings

- **The `Audio` reopen trigger fired and is documented as fired.** Left alone, `docs/PATTERNS.md` drifts into presenting an open question as settled. → drives Phase 2, which is **mandatory this time**.
- **"Who inserts it" was never the distinction.** Four of the seven fixed resources are game-inserted; the line is now "the engine defines the type and only reads it". That is precisely the argument that had kept `Audio` deferred. → drives Phase 2's framing.
- **What still separates `Audio`:** the engine *drives* it every frame (`AudioFacadeSystem`) rather than reading it as config, and it owns an OS device handle rather than a value. Real, but thinner than what it replaced. → drives Phase 2's options.
- **Both board channels are empty and the board has not moved since 2026-07-27** (verified by `git log`, not by eyeballing). → drives Phase 1: ASK, do not self-pick.
- **The user has now taken the recommendation 7 of 7**, and this session they asked for the *packaging* judgement too ("can any of these be combined?"), not just the ranking. → drives Phase 1's presentation: recommend a combination, not just an order.
- **`play_tone_metered` covers tones only.** Clips (`play_sfx` / `play_sfx_on_bus`) still face the original exclusivity, and the redirection would be the same shape. Nothing has asked for it. → drives Phase 3B.
- **Adopting a polyphonic meter silently invalidates single-voice normalisation.** It surfaced as 31% of frames pinned at full shake, caught only because the drive distribution was computed. → drives the caution in 3B and any future adoption.
- **`survivor`'s behaviour changed in two consecutive sessions and both times the verification was thrown away.** It still has no self-test mode. → drives Phase 3D.

## Anti-Goals (What NOT To Do)

- **Do NOT self-pick the next feature.** The board is empty; Phase 1 ends in a question. This shape has now worked five sessions running.
- **Do NOT silently re-defer the `Audio` question, and do NOT unilaterally change `Audio`'s behaviour either.** Both are wrong. The doc says the trigger fired; the resolution is the user's, and it must be recorded either way.
- **Do NOT re-run the `insert_core_resources` audit.** All 27 are classified in writing in `docs/PATTERNS.md`, including the 20 that are correctly scene state. That negative result exists precisely to stop a third pass.
- **Do NOT add `TimeScale` to the persistent set.** Deliberate exclusion, documented: a frozen or slowed *old* scene leaking forward is worse than losing a hit-stop.
- **Do NOT migrate `survivor`'s bullet tone to anything.** Settled by arithmetic: 0.04 s of tone every 0.14 s never overlaps, so it was never being cut and there is nothing to meter.
- **Do NOT make `bands()` work for a metered one-shot.** Deliberately zeros; reopening costs an FFT per voice on the playback thread, and nobody has a use case for the spectrum *of a one-shot*.
- **Do NOT implement windowed frame capture.** The game declined in writing 2026-07-27. Closed, not deferred.
- **Do NOT add the `MapGenerator` trait.** Its trigger (swapping generators at runtime) still has not fired.
- **Do NOT trust a `run_in_background` completion notification's exit code.** Five consecutive sessions. Use the block in `docs/VERIFICATION.md`.

## Plan

### Phase 1: Board gate, then ask — leading with the decision, not a build item

**Goal:** Establish whether external work exists and, if not, get an explicit direction — with the `Audio` question raised first, because it is a decision the user owns rather than work the agent can rank.

**Why this approach:** The board has preempted self-picked work every time it was non-empty, and the ask has produced better choices than my own ranking for five sessions. The `Audio` item is different in kind from the rest of the menu: it is not "which should we build" but "a doc currently states an unresolved question, and it needs an owner's answer".

- Read `../dungeon-merchant/docs/engine-wishlist.md` — **Active requests** (expected: empty, next free **EW-012**). Extract with `awk '/^## Active requests/,/^## Done \/ archive/'`; do **not** `cat` the 53 KB file.
- Read `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — **Open Requests** (expected `_None._`).
- Run `cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md`. Later than **2026-07-27** means the board moved — read it before anything else. **A filed request preempts every phase below.**
- Verify the starting state with the copy-paste block in `docs/VERIFICATION.md`. Read the exit code **from the file**; expect **0 · 152 ok groups · 1356 lib tests** at v0.141.0.
- If a request exists: triage, acknowledge on the board with an append-only `[Engine]` line and an absolute date, and defer Phases 2–3.
- If both are empty: ask **one** `AskUserQuestion` (Korean). **Question 1 is the `Audio` decision** (Phase 2's options, below). **Question 2 is the build menu** (Phase 3's branches, with reasoning). Two questions in one call — the user answered a two-question call cleanly this session.
- Carry the reasoning, not just labels, and recommend a **combination** where one is defensible — this session the user explicitly asked for that judgement.

**Files:** none (read-only)
**Validates with:** both board files read; a stated verdict ("empty on both channels, next free EW-012, unmoved since 2026-07-27"); either a filed request or the user's two recorded answers.
**Rollback:** n/a — no mutations.

### Phase 2: Resolve the `Audio` auto-persistence question (MANDATORY, whichever way it goes)

**Goal:** Get `docs/PATTERNS.md` out of the state where it documents a fired trigger with no resolution.

**Why this approach:** Unlike the last two plans, there is a housekeeping item with a real deadline here — not a size cap, but a correctness one. The doc says the trigger fired and the argument is thinner. Every session that reads it and moves on makes it likelier the next agent treats a live question as settled. Both outcomes are small; leaving it open is the only expensive option.

Two outcomes, and **the work is bounded either way**:

- **If the user says persist it:** one line in `App::new` — `app.register_persistent::<crate::audio_facade::Audio>();` — plus a regression test in `src/app.rs`'s `mod tests` modelled exactly on `window_config_survives_a_scene_reset`. ⚠️ **`Audio::new()` needs a device**, so the test cannot construct a real one on CI. Options, in order of preference: (a) assert on a stand-in type registered the same way if `Audio` cannot be built headlessly; (b) `#[ignore]` with a comment; (c) test only that the `TypeId` is in `persistent_resources`. Pick (a) or (c) — an ignored test is not evidence. Then **remove** `register_persistent::<Audio>()` from `examples/games/settings_menu` and `examples/games/beat_crawler`, which currently do it by hand, and update `docs/PATTERNS.md` + the `beat_crawler` module-map row. Ship as **PATCH** (bugfix, v0.137.1 precedent).
- **If the user says leave it game-side:** no code. Rewrite the `docs/PATTERNS.md` block so it reads as **decided**, not as pending — state the fired trigger, state the surviving distinction (the engine *drives* `Audio` per frame and it owns a device handle, not a value), and state what would reopen it *next* (a third game hitting it, or a bug report tracing to it). Ship as a docs-only PR, no bump.
- Either way, update the `[[engine-current-state]]` memory: the STANDING/DEFERRED block currently describes a deferral whose trigger has fired.

**Files:** `docs/PATTERNS.md` always; `src/app.rs`, `examples/games/settings_menu/settings_menu.rs`, `examples/games/beat_crawler/beat_crawler.rs`, `docs/MODULE_MAP.md`, `docs/CHANGELOG.md` only in the "persist it" branch.
**Validates with:** `docs/PATTERNS.md` contains no unresolved question; if code changed, a test that fails without the fix and `./scripts/verify.sh` exit 0 with lib tests ≥ 1356.
**Rollback:** the code branch is one line + one test + two example deletions; the docs branch is a single block revert.

### Phase 3: Build the chosen menu item

Four mutually exclusive branches. **Only the one the user picks gets built.**

#### 3A — A real soundtrack for `beat_crawler` (recommended)

**Goal:** Meter a mixed track rather than two synthesized tones chosen to be trivially separable.

**Why this approach:** It is the only remaining item where the engine could genuinely *fail* and we would learn something. `beat_crawler` discriminates kick 4.00 from blip 0.61 — 6.5× apart — which is a test that cannot fail. A real mix is what `bands()` exists for, and it is the first case where the low-band detector could be wrong.

- Needs a **licence-clean** file. The CC0 synthesis recipe in `src/audio/fixtures/README.md` solved exactly this for the codec tests — **synthesize, do not source.**
- **Expect the kick detector to need retuning**, and expect the arm/re-arm pattern to come under pressure: a dense track's low bands may never fall below `REARM_THRESHOLD`. Seq 1's cooldown finding is the likely fix, and confirming it would generalise that finding a second time.
- Keep `BEAT_CRAWLER_SELFTEST=1` green and extend it — the kick-heard/blip-rejected assertion needs a real-track analogue.
- Do **not** let this become a third capstone. One addition, kept small.

**Files:** `examples/games/beat_crawler/beat_crawler.rs`, a synthesized fixture, `docs/CHANGELOG.md`
**Validates with:** `BEAT_CRAWLER_SELFTEST=1` exits 0; the turn clock tracks the real track; `./scripts/verify.sh` exit 0.
**Rollback:** revert the track + threshold constants; the tone scheduler is untouched.

#### 3B — `play_sfx_metered`, the clip counterpart

**Goal:** Extend the metered one-shot from tones to clips, which is the more common one-shot.

**Why this approach:** `play_tone_metered` closed the gap for tones only, and `enable_analysis`'s docs now say so explicitly. The redirection is the same shape and the plumbing exists. Ranks below 3A because **nothing has asked for it** — 3A is driven by a real weakness, this by symmetry.

- Mirror `play_tone_poly`: rotate `POLY_VOICES` voices under `poly_voice_channel(meter, i)` via `next_poly_voice`, `assign_bus`, `stop_immediate` on wrap, then `play_bytes_internal` — but the tap redirection must reach `append_decoded`, which currently calls `self.tapped(channel, effected)` at `playback.rs:413`. That is the one real difference from the tone path and the place this could grow.
- wasm: `play_sfx_to` builds per-source nodes already, so like the tone path it should need only a `tap(meter, node)` — **verify that before assuming it**, since `play_sfx_to_opts` returns an `Sfx` handle the tone path does not have.
- Reuse `sum_levels` and `combine_voices` unchanged. **No new policy** — if the design seems to need one, stop.
- Tests: device-free, in the style of the ten that shipped. At minimum, that the byte path taps under the meter name and not the voice name.
- **Same stop condition as its predecessor:** if it is not roughly one facade method plus one manager method plus the tap redirection, report why instead of expanding.

**Files:** `src/audio_facade.rs`, `src/audio/playback.rs`, `src/audio/analysis.rs`, `src/audio_wasm.rs`, `docs/MODULE_MAP.md`, `docs/CHANGELOG.md`
**Validates with:** device-free tests proving the redirection; `./scripts/verify.sh` exit 0; lib tests **> 1356**.
**Rollback:** additive — remove the method and its tests.

#### 3C — A fourth procgen mode (drunkard's walk)

**Goal:** Add one more generator over the shared `DungeonMap`.

**Why this approach:** Proven three times, composes with `to_path_grid` / `to_tilemap_tiles` / `FovMap` for free. Ranks last: rooms/caves/mazes plus `roguelike` and `beat_crawler` already cover this area, so a fourth teaches least.

- Mirror `generate_cellular_cave`'s signature exactly: `generate_drunkard_walk(w, h, seed, &DrunkardParams) -> DungeonMap`; params for target floor fraction + max steps; cap cells at `MAX_PATH_GRID_CELLS`.
- Record a 1×1 spawn `Room` at the walk's start so `first_room_center` works — `beat_crawler` proved why (room-based placement degenerates when a generator records one room).
- Tests: determinism, connectivity (single flood-filled region), floor fraction within tolerance, over-cap → empty + `error!`.
- Example `drunkard_walk` following `maze_generation`'s shape (WASD, R regenerate, headless connectivity self-check).

**Files:** `src/mapgen.rs`, `src/lib.rs`, `examples/drunkard_walk.rs`, `docs/MODULE_MAP.md`, `docs/CHANGELOG.md`
**Validates with:** 4+ unit tests incl. a single-connected-region assertion; `./scripts/verify.sh` exit 0.
**Rollback:** purely additive.

#### 3D — `SURVIVOR_SELFTEST=1`

**Goal:** Make `survivor`'s audio-reactive path regression-proof.

**Why this approach:** Its behaviour changed in two consecutive sessions and both times the verification (probes, captures) was deleted afterwards. A regression would be silent. Ranks low only because it adds no capability.

- Follow the `BEAT_CRAWLER_SELFTEST` precedent exactly, including **no device = SKIP, not fail**, which is what lets it exist without breaking CI.
- Assert what this session measured by hand: the meter moves above `KILL_PEAK_OFF` while kills land; the watchdog engages when `enable_analysis` is skipped; the drive does not sit pinned at 1.0.
- Reuse the input-script approach from the handoff's Quick Start rather than driving the game by hand.

**Files:** `examples/games/survivor/survivor.rs`, `docs/CHANGELOG.md`
**Validates with:** `SURVIVOR_SELFTEST=1` exits 0 with a device and SKIPs without one; `./scripts/verify.sh` exit 0.
**Rollback:** additive — remove the mode.

### Phase 4 (conditional): land an engine change as its own PR

**Goal:** Keep a public-API or engine-behaviour change separable from the example that motivated it.

**Why this approach:** #388 established it and this session used it twice, producing three independently-revertible PRs. **Phase 2's code branch and Phase 3B are the only ones that can produce an engine change**; 3A, 3C and 3D are additive example/module work.

- Version per `CLAUDE.md`: MINOR for an additive API, PATCH for a pure bugfix. Use `/ship` for the four-file paperwork, `/land-pr` for the branch → CI → merge loop.
- **If two PRs stack**, branch the second from the first and rebase with `git rebase --onto origin/main <first-tip-sha> <second-branch>` after the first merges — a plain `git rebase main` re-applies the squashed commit and conflicts. Procedure in the handoff.

**Files:** as per the branch
**Validates with:** each PR green independently; `./scripts/verify.sh` exit 0 before each push.
**Rollback:** revert the engine PR without touching the example PR.

## Dependencies & Order

- **Phase 1 gates everything.** A filed board request preempts Phases 2–3 entirely.
- **Phase 2 is mandatory and runs before Phase 3**, regardless of which build item is chosen — it is small either way and the doc should not stay in its current state for another session.
- **Phase 3's branches are mutually exclusive. Do not build two** unless the user asks to combine, as they did this session; if they do, prefer pairs with **no file overlap** (3A + 3C is the only clean pair; 3B and 3D both touch audio-adjacent code that 3A also touches).
- **Phase 4 only fires if Phase 2 took its code branch or Phase 3 chose 3B.**

## Risks & Mitigations

- **Self-picking instead of asking.** Likely if the next session reads "execute Phase 1" as "start coding". Mitigation: Phase 1 has no files and no code; its deliverable is two recorded answers.
- **Treating Phase 2 as optional.** The most likely failure of this plan — it is bookkeeping-shaped and easy to skip past. Mitigation: it is listed as mandatory in three places, and its success criterion is a doc containing no unresolved question.
- **A false-green gate.** Fired in four sessions; did not fire this session because the `docs/VERIFICATION.md` block was used from the first launch. Mitigation: keep using it — `rm -f` first, run non-piped, wait on the file, read it, check mtime, corroborate **152 groups / 1356 lib tests** at v0.141.0.
- **3B growing past its stop condition.** Moderate: the byte path's tap sits inside `append_decoded`, one level deeper than the tone path's. Mitigation: the branch has an explicit stop condition; a written "here is why it is not the same shape" is a valid outcome.
- **A new metered-one-shot consumer inheriting the normalisation bug.** Any code that normalises against a single voice's maximum is wrong once voices sum, and it shows as a feel regression rather than an error. Mitigation: it is documented in `survivor`'s module docs and module-map row 79; re-read those before adopting the API anywhere else.
- **`Audio::new()` needing a device blocks a clean Phase 2 test.** Likely if the persist branch is chosen. Mitigation: the phase names three fallbacks in preference order and rules out `#[ignore]` as non-evidence.
- **`cargo fmt` reflow defeating a scripted edit.** Unchanged. Mitigation: run `cargo fmt` before the first verify, and verify any scripted deletion with an independent `grep`, never with the script's own exit code.
- **Audio work is not covered by CI.** Green CI cannot prove a meter reads anything. Mitigation: anything touching metering needs a headless `ENGINE_CAPTURE` run on a machine with a real device — confirm the rodio `DeviceSink` teardown line appears, or the run proved nothing.

## Success Criteria

- **Minimum viable:** the board gate is run and stated on both channels; both questions are asked (or a filed request is triaged); nothing is self-picked.
- **Phase 2 is resolved and `docs/PATTERNS.md` no longer contains a live unanswered question** — this is the one non-negotiable outcome of the session.
- The chosen Phase 3 branch ships with `./scripts/verify.sh` exit **0**, `ok` groups **≥ 152**, and lib tests **≥ 1356** — never fewer.
- **If Phase 2 took the code branch:** the test fails without the one-line fix, and both examples' hand-rolled `register_persistent::<Audio>()` calls are removed.
- **If 3A:** a licence-clean synthesized track, `BEAT_CRAWLER_SELFTEST=1` still exit 0, and a written verdict on whether arm/re-arm survives a real track or needs seq 1's cooldown.
- **If 3B:** lib tests **> 1356**, and either a working `play_sfx_metered` or a written explanation of why the byte path is not the same shape.
- **If 3C:** 4+ unit tests including a single-connected-region assertion, plus a playable example.
- **If 3D:** `SURVIVOR_SELFTEST=1` exits 0 with a device and SKIPs without one.
- Memory advanced to at least **seq 216** with the recorded `main @ <hash>`, `MEMORY.md`'s hook refreshed, and the STANDING/DEFERRED `Audio` block updated to match Phase 2's outcome. The recorded tip will be one commit stale if the handoff lands as its own PR, as it always has.

## Quick Start

```bash
# Restore full context
cat plans/handoffs/HANDOFF_audio-adoption_metered-oneshot-and-config-audit_2026-07-31.md

# 1. BOARD FIRST — this is Phase 1 and it is not optional
awk '/^## Active requests/,/^## Done \/ archive/' ../dungeon-merchant/docs/engine-wishlist.md
grep -A3 '^## Open Requests' ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md
cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md && cd -
# ^ later than 2026-07-27 = the board MOVED; read it before anything else

# 2. Verify starting state — the block docs/VERIFICATION.md carries.
#    Read the exit code from the FILE. The notification has lied in FIVE straight sessions.
rm -f /tmp/v.exit /tmp/v.log
(./scripts/verify.sh > /tmp/v.log 2>&1; echo $? > /tmp/v.exit) &
until [ -f /tmp/v.exit ]; do sleep 20; done
echo "EXIT=$(cat /tmp/v.exit)"; ls -l /tmp/v.exit
echo "ok groups: $(grep -c 'test result: ok' /tmp/v.log)"
# expect 0 · 152 ok groups · 1356 lib tests   (v0.141.0)

# 3. Key files for the phases
#   docs/PATTERNS.md                       — "Surviving a scene reset": the FIRED Audio trigger   [Phase 2]
#   src/app.rs                             — App::new's register_persistent block + its tests     [Phase 2]
#   examples/games/beat_crawler/…          — the turn clock + BEAT_CRAWLER_SELFTEST precedent     [3A/3D]
#   src/audio/playback.rs                  — play_tone_poly (the shape to mirror), append_decoded [3B]
#   src/audio/analysis.rs                  — POLY_VOICES, next_poly_voice, combine_voices         [3B]
#   src/audio_analysis.rs                  — sum_levels (do NOT add a second policy)              [3B]
#   docs/MODULE_MAP.md                     — GREP IT, never read it whole (72 rows, ~90 KB)

# 4. FIRST CONCRETE ACTION
#    Read both board files and state the verdict. If empty, ask ONE AskUserQuestion (Korean)
#    with TWO questions:
#      Q1 (decision, lead with it): should `Audio` be auto-persisted? Its reopen trigger FIRED
#         this session — 7 config resources were being dropped, and 4 of them are game-inserted,
#         which is exactly the argument that had kept Audio deferred. Options: persist it
#         (one line + a test + remove the two examples' hand-rolled calls) / keep it game-side
#         and rewrite the doc as DECIDED rather than pending.
#      Q2 (build menu, with reasoning): beat_crawler real soundtrack (recommended — the only item
#         where the engine could genuinely fail) / play_sfx_metered / 4th procgen mode /
#         SURVIVOR_SELFTEST.
#    Do NOT self-pick. Phase 1 ends in a question, not code.
```
