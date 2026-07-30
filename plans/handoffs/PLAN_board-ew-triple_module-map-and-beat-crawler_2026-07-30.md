# Next: board gate, trim the memory before it hits its cap, then one item from the empty-board menu

**Date:** 2026-07-30
**Status:** PLANNED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `board-ew-triple` seq `4`
**Context:** See `HANDOFF_board-ew-triple_module-map-and-beat-crawler_2026-07-30.md` for session data, all measurements, the five merged PRs and the verify-gate history.

---

## Problem Statement

The two things the last three handoffs kept flagging are now **done**: the `CLAUDE.md` 200-line cap is resolved (the module map moved to `docs/MODULE_MAP.md`, cutting per-session auto-loaded context from 95,075 to 8,231 bytes), and a second capstone shipped (`beat_crawler`, v0.138.0). **Nothing is in flight and both board channels are empty** (next free EW-012; `rust-survivors` `_None._`), so again there is no externally-driven work.

Two things constrain what comes next. First, **`engine-current-state` memory is ~42 KB and growing ~1.5 KB per seq against a ~76 KB read cap it has already hit once** — the prior plan said trim around seq 210 and we are at 209. That is housekeeping with a deadline, not a preference. Second, this session found a **pre-existing engine bug by accident** (`WindowConfig` silently reverting on scene reset, affecting 20 shipped examples, worked around twice in-tree rather than fixed) — which raises an open question nobody has answered: *are there others?* `insert_core_resources` has never been audited against "which of these would a game set once and expect to keep".

See Evidence & Data in the handoff for the numbers behind all of this.

## Key Findings

- **Both board channels are empty and were re-checked at session start** (board file last edited 2026-07-27). → drives Phase 1: ASK, do not self-pick.
- **The user has now taken the recommendation 5 times out of 5** across two sessions, including one where the recommendation *changed* mid-report because a new measurement overturned the plan's premise. The framing (what breaks otherwise, what the cost actually is) is load-bearing. → drives Phase 1's presentation.
- **`engine-current-state` is ~42 KB / seq 209**; the read cap is ~76 KB and was hit once on 2026-07-29. → drives Phase 2, which has a real deadline.
- **`WindowConfig` was found by accident, not by audit**, and the engine had documented the bug twice while working around it. Nothing says it is the only such resource. → drives Phase 3B.
- **`beat_crawler` is a second consumer of `Audio::bands()`, but was written by the session that had just designed the API.** `survivor` is older code and would be the first genuinely independent consumer. → drives Phase 3A.
- **Trap 4 (background notification reports the wrong exit code) fired a THIRD time**, in three consecutive sessions, despite being documented before each. #390 upgraded the mitigation from prose to a copy-paste block; that is unproven. → drives the verify discipline in every phase.
- **`bands()` did not need a caller-selectable FFT size** — 16 bands with the low 4 summed was ample for real gameplay. → weakens that open question further; leave it closed.
- **The `board-ew-triple` tag has been historical since seq 2** and the arc it was carrying has now closed. → drives the chain-tag note in Dependencies.

## Anti-Goals (What NOT To Do)

- **Do NOT self-pick the next feature.** The board is empty; the standing rule is to ASK. Phase 1 ends in a question, not code — the same shape as seq 3's plan, which worked.
- **Do NOT auto-persist `Audio` on speculation.** The user ruled on 2026-07-30: document the risk, act only if it bites. `docs/PATTERNS.md` carries the reopen triggers. Reopening it without one of those triggers firing contradicts an explicit decision.
- **Do NOT implement windowed frame capture.** The game declined in writing 2026-07-27. Closed, not deferred.
- **Do NOT add the `MapGenerator` trait.** Still an anti-goal; its trigger (wanting to swap generators at runtime) has not fired — and `beat_crawler` swapping generators by depth parity did **not** need it.
- **Do NOT widen `bands()` with a caller-selectable FFT size.** The capstone is now real evidence that 16 bands suffice.
- **Do NOT re-sweep the 14 smokes.** Done seq 3, 14/14. Re-run individually only after touching the render path, the asset path, or a `web/build.sh`.
- **Do NOT trust a `run_in_background` completion notification's exit code.** It has now lied in three consecutive sessions. Use the block in `docs/VERIFICATION.md`.
- **Do NOT merge module-map rows to save lines.** `CLAUDE.md`'s Documentation rules now record why that games the metric.

## Plan

### Phase 1: Board gate, then ask for direction

**Goal:** Establish whether external work exists and, if not, get an explicit direction rather than inventing one.

**Why this approach:** The board has preempted self-picked work every time it was non-empty, and the ask has produced better choices than my own ranking four sessions running.

- Read `../dungeon-merchant/docs/engine-wishlist.md` — **Active requests** (expected: empty, next free **EW-012**) and skim the `[Game]` comments for anything actionable that was not filed.
- Read `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — **Open Requests** (expected `_None._`). Paused/deprecated; do not chase compatibility.
- Run `cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md`. Later than **2026-07-27** means the board moved — read it before anything else. **A filed request preempts every phase below.**
- If a request exists: triage by priority, acknowledge on the board with an append-only `[Engine]` line and an absolute date, and defer Phases 2–3.
- If both are empty: ask **one** `AskUserQuestion` (Korean) carrying the direction menu from Phase 3, **with the trade-off reasoning, not just labels**.
- Phase 2 does **not** need to be in that question — it is housekeeping with a deadline, not a choice. State that it is being done and do it.

**Files:** none (read-only)
**Validates with:** both board files read; a stated verdict ("empty on both channels, next free EW-012") and either a filed request or the user's recorded choice.
**Rollback:** n/a — no mutations.

### Phase 2: Trim `engine-current-state` memory (do this regardless of the answer)

**Goal:** Get the memory file back under its safe size before it becomes unreadable mid-session.

**Why this approach:** It is ~42 KB against a ~76 KB Read/Edit cap and grows ~1.5 KB per seq. It hit that ceiling once already (2026-07-29) and **could not be read**, which is the worst possible failure mode for the file that tells a session where it is. The prior plan set the trigger at seq 210; we are at 209.

- Read `MEMORY.md` and `engine-current-state.md` first — the tip line is one enormous paragraph, so edits must be surgical single-occurrence `str.replace` (the file's own header says so).
- Cut the **chain tail** into `[[engine-history-archive]]`: keep the current arc (roughly seq 195+, i.e. `board-ew-triple` from seq 1) plus one prior session; move everything older.
- Follow the precedent of the two prior trims (2026-07-02 and 2026-07-29) — the archive gets the moved text verbatim under a dated heading, and the current-state file gets a one-line "prior = seqs X→Y moved to `[[engine-history-archive]]` on {date}" marker in the tip chain.
- Do **not** drop live gotchas, the STANDING/DEFERRED `Audio` block, or the board status line — those are current, not history.
- Re-measure afterwards and record the new size in the file's own keep-compact note.

**Files:** `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md`, `engine-history-archive.md`, `MEMORY.md`
**Validates with:** `wc -c engine-current-state.md` comfortably under 40 KB (target ~25–30 KB, matching the post-trim size after the 2026-07-29 pass); the file reads back in one `Read` call; no live gotcha lost.
**Rollback:** memory files are not version-controlled — **copy both files to the scratchpad before editing** so a bad trim is restorable.

### Phase 3: Execute the chosen menu item

Four mutually exclusive branches. **Only the one the user picks in Phase 1 gets built.**

#### 3A — Adopt audio-reactive in `examples/games/survivor` (recommended)

**Goal:** Stress `levels()`/`bands()` from code that was not written by the API's designer.

**Why this approach:** `beat_crawler` is a second consumer, but it was built in the same session that read the API cold — the same weakness the parent flagged about `audio_reactive`. `survivor` is older, denser, and already uses the `Audio` facade, so adoption needs no new plumbing.

- Meter `survivor`'s sfx bus or a named channel; drive `HitFlash` intensity and `Camera::shake` amplitude from `levels().peak` instead of fixed constants.
- **Expect to hit `play_sfx`'s limitation immediately** — it round-robins 16 anonymous voices on native, so it *cannot* be metered. **That verdict is the deliverable**: either the game moves its feedback sounds to a named channel, or this is a real API gap (a meterable one-shot). Report which, with evidence. Do not paper over it.
- Add `register_persistent::<Audio>()` if `survivor` uses scenes and does not already have it — and if it is missing, that is a **third** instance of the deferred risk and a trigger to reopen it (see `docs/PATTERNS.md`).
- Keep the change small and reversible; if it does not improve feel, say so and revert rather than shipping churn.

**Files:** `examples/games/survivor/survivor.rs`, possibly `src/audio_facade.rs` if a gap is confirmed, `docs/CHANGELOG.md`
**Validates with:** the game runs and the effect is visibly audio-driven; `./scripts/verify.sh` exit 0; a written verdict on `play_sfx`.
**Rollback:** revert the game file; any engine addition ships as its own commit.

#### 3B — Audit `insert_core_resources` for other scene-reset-fragile config

**Goal:** Find out whether `WindowConfig` was the only engine-inserted resource silently reverting on a scene change.

**Why this approach:** It was found **by accident** while writing a capstone, after the engine had documented and worked around it twice. That is not a search strategy. The audit is cheap, bounded, and directly evidence-driven.

- Read `src/app/core_resources.rs` and list every resource `insert_core_resources` inserts.
- For each, classify: **scene state** (correctly dies with the scene) vs **session/config state** (a game sets it once and expects it to persist). The `docs/PATTERNS.md` "Surviving a scene reset" section defines this test.
- Cross-check the survivors list in that doc (`WindowConfig`, `SceneTransition`, `TextMeasurer`, `InputScript`, the 7 RON registries, `DebugUi` by hand).
- Prime suspects to check first, by the same logic that made `WindowConfig` wrong: `DesignResolution`, `WindowOptions`, `FrameConfig`, `LightingConfig`, `FocusRingStyle`, `StickNavConfig`, `DialogueStyle`, `TimeScale` — all opt-in config a game inserts once.
- For each genuine finding: one line in `App::new` + a regression test **proven non-vacuous by disabling the fix**, exactly as #388 did. Land as one PR if the fixes are uniform, or one per resource if they need argument.
- If the answer is "none", **say so and write it into the doc** — a negative result stops the next session re-running the audit.

**Files:** `src/app.rs`, `src/app/core_resources.rs`, `docs/PATTERNS.md`, `docs/CHANGELOG.md`
**Validates with:** every inserted resource classified in writing; any fix carries a test that fails without it; `./scripts/verify.sh` exit 0.
**Rollback:** each fix is one line + one test; revert individually.

#### 3C — A fourth procgen mode (safest, lowest marginal value)

**Goal:** Add one more generator over the shared `DungeonMap`.

**Why this approach:** Proven three times and composes with `to_path_grid`/`to_tilemap_tiles`/`FovMap` for free. Ranks last: rooms/caves/mazes plus `roguelike` **and now `beat_crawler`** already cover this area well.

- **Drunkard's walk** remains the best candidate: ~80 lines, **connected by construction**, deterministic from `Rng`.
- Mirror `generate_cellular_cave`'s signature exactly: `generate_drunkard_walk(w, h, seed, &DrunkardParams) -> DungeonMap`; params for target floor fraction + max steps; cap cells at `MAX_PATH_GRID_CELLS`.
- Record a 1×1 spawn `Room` at the walk's start so `first_room_center` works — and note `beat_crawler` proved why that matters (room-based placement degenerates when a generator records one room).
- Tests: determinism, connectivity (single flood-filled region), floor-fraction within tolerance, over-cap → empty + `error!`.
- Example `drunkard_walk` following `maze_generation`'s shape (WASD, R regenerate, headless connectivity self-check).

**Files:** `src/mapgen.rs`, `src/lib.rs`, `examples/drunkard_walk.rs`, `docs/MODULE_MAP.md`, `docs/CHANGELOG.md`
**Validates with:** 4+ unit tests; headless self-check asserts a single connected region; `./scripts/verify.sh` exit 0.
**Rollback:** purely additive — remove the function, the re-export and the example.

#### 3D — A deeper pass on `beat_crawler`

**Goal:** Take the capstone from "one playable loop" to something with progression.

**Why this approach:** Only if the user wants to keep pulling this thread. The loop is deliberately minimal (no items, no meta-progression, no save) and each addition would exercise another engine subsystem — `save`/`load_migrated`, `WeightedTable` loot, `DataTable` enemy stats, a real soundtrack instead of synthesized tones.

- Pick **one** addition, not several. `WeightedTable` loot drops or `DataTable`-driven enemy stats are the cheapest that still exercise something new.
- A real soundtrack would be the most interesting for `bands()` (a mixed track rather than isolated tones is the actual use case) but needs a **license-clean** audio file — see the CC0 synthesis recipe in `src/audio/fixtures/README.md`, which solved exactly this problem for the codec tests.
- Keep the acceptance test green and extend it rather than replacing it.

**Files:** `examples/games/beat_crawler/beat_crawler.rs`, possibly `examples/assets/`, `docs/CHANGELOG.md`
**Validates with:** `BEAT_CRAWLER_SELFTEST=1` still exits 0; headless capture eyeballed; `./scripts/verify.sh` exit 0.
**Rollback:** the game is one file plus a `[[example]]` entry.

## Dependencies & Order

- **Phase 1 gates everything.** A filed board request preempts Phases 2–3.
- **Phase 2 is independent of the answer and should be done regardless** — it can run while waiting for the direction, and it is the one item with a deadline.
- **Phase 3's branches are mutually exclusive. Do not build two.**
- **3B is the only branch that could produce an engine fix**, so it is the only one where the "land the engine change as its own PR" rule from seq 3 applies again.
- **Chain tag:** `board-ew-triple` has been historical since seq 2 and the arc has closed. **Starting a fresh tag at seq 1 is reasonable and should be stated explicitly** in the next handoff rather than continuing out of habit.

## Risks & Mitigations

- **Self-picking instead of asking.** Likely if the next session reads "execute Phase 1" as "start coding". Mitigation: Phase 1 has no files and no code; its deliverable is a recorded answer.
- **A false-green gate.** Fired in **three consecutive sessions** in the same documented mechanism. Mitigation: use the copy-paste block now in `docs/VERIFICATION.md` — `rm -f` first, run non-piped, read the file, check mtime — and corroborate with the counts (**152 ok groups / 1339 lib tests** at v0.138.0). Note your *own* pipelines can lie too; that happened this session.
- **A bad memory trim losing live context.** Moderate — the file is not version-controlled. Mitigation: copy both memory files to the scratchpad before editing, and diff the gotchas/STANDING blocks after.
- **`cargo fmt` reflow reddening the first gate run.** Near-certain on any new `.rs`. Mitigation: run `cargo fmt` *before* the first verify, not after it fails.
- **Stale rust-analyzer diagnostics claiming compile errors.** Fired twice this session. Mitigation: trust `cargo build`/`cargo clippy` exit codes only.
- **3A discovering `play_sfx` is a real API gap** mid-session, turning a small adoption into an engine change. Mitigation: that outcome is *expected and wanted* — land the verdict as a written finding first, and only then decide whether to build; do not silently expand scope.
- **An example broken for wasm without anyone noticing.** The gate builds lib+bins only. Mitigation: if new work claims web support, build the example for wasm32 explicitly and run its smoke.

## Success Criteria

- **Minimum viable:** the board gate is run and stated on both channels; the direction question is asked (or a filed request is triaged); nothing is self-picked. Phase 2 is done regardless.
- **`engine-current-state.md` is comfortably under 40 KB** (target ~25–30 KB) and reads back in a single `Read`, with no live gotcha, the STANDING `Audio` block, or the board status lost.
- The chosen Phase 3 branch ships with `./scripts/verify.sh` exit **0**, lib tests **≥1339** (never fewer), and `ok` groups **≥152**.
- If 3A: a written verdict on whether `play_sfx`'s unmeterable round-robin is a real API gap or acceptable, with evidence.
- If 3B: every resource `insert_core_resources` inserts is classified in writing — **including a written "none found" if that is the answer** — and any fix carries a test proven to fail without it.
- Memory advanced to at least **seq 210** with the recorded `main @ <hash>`, and `MEMORY.md`'s index hook refreshed.

## Quick Start

```bash
# Restore full context
cat plans/handoffs/HANDOFF_board-ew-triple_module-map-and-beat-crawler_2026-07-30.md

# 1. BOARD FIRST — this is Phase 1 and it is not optional
cat ../dungeon-merchant/docs/engine-wishlist.md          # Active requests; next free EW-012
cat ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md     # expect _None._
cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md && cd -
# ^ later than 2026-07-27 = the board MOVED; read it before anything else

# 2. Verify starting state — the block docs/VERIFICATION.md now carries.
#    Read the exit code from the FILE. The notification lied in three straight sessions.
rm -f /tmp/v.exit /tmp/v.log
(./scripts/verify.sh > /tmp/v.log 2>&1; echo $? > /tmp/v.exit) &
until [ -f /tmp/v.exit ]; do sleep 20; done
echo "EXIT=$(cat /tmp/v.exit)"; ls -l /tmp/v.exit
echo "ok groups: $(grep -c 'test result: ok' /tmp/v.log)"
# expect 0 · 152 ok groups · 1339 lib tests

# 3. Key files for the phases
#   docs/MODULE_MAP.md            — GREP IT, never read it whole (72 rows, ~89 KB)
#   docs/PATTERNS.md              — "Surviving a scene reset" (3B's classification test)
#   src/app/core_resources.rs     — the audit subject (3B)
#   examples/games/survivor/survivor.rs  — the adoption target (3A)
#   src/mapgen.rs                 — 3 generators to mirror (3C)

# 4. FIRST CONCRETE ACTION
#    Read both board files and state the verdict. If empty, ask ONE AskUserQuestion (Korean)
#    with the Phase 3 menu AND the reasoning: adopt audio-reactive in survivor (recommended —
#    first consumer not written by the API's designer; the play_sfx verdict is the deliverable) /
#    audit insert_core_resources for more WindowConfig-class bugs / a 4th procgen mode /
#    a deeper pass on beat_crawler. Say that Phase 2 (memory trim) is being done regardless.
#    Windowed capture is OFF the menu. Do NOT self-pick.
```
