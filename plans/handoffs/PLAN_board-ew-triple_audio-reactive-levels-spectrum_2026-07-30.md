# Next: resolve the CLAUDE.md cap, then take one item from the empty-board menu

**Date:** 2026-07-30
**Status:** PLANNED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `board-ew-triple` seq `3`
**Context:** See `HANDOFF_board-ew-triple_audio-reactive-levels-spectrum_2026-07-30.md` for session data, all measurements, the four merged PRs and the verify-gate history.

---

## Problem Statement

Audio-reactive is complete (v0.136.0 `Audio::levels`, v0.137.0 `Audio::bands`), the parent's open question about the web smokes is answered (14/14 pass), and two architectural findings landed in `docs/PATTERNS.md`. **Nothing is in flight and both board channels are empty** (next free EW-012; `rust-survivors` `_None._`), so there is no externally-driven work.

Two things nevertheless constrain whatever comes next. First, **`CLAUDE.md` is at 198/200 lines** and the module map grows ~1 line per feature — so the *next* feature that adds a module-map row breaches the project's own documented cap. That decision (move the 80-line module map to `docs/MODULE_MAP.md`?) changes how every future session navigates the codebase and is explicitly the user's call, flagged in two consecutive handoffs now. Second, the engine's surface has grown far faster than its integration coverage: 10 UI widgets, a game-feel toolkit, 3 procgen modes, FOV, dialogue, timeline, skeletal animation and now audio analysis are exercised almost entirely by **single-purpose demos**, which is precisely where VISION says API awkwardness hides.

See Evidence & Data in the handoff for the numbers behind all of this.

## Key Findings

- **Both board channels are empty and were re-checked at session end** (board file last edited 2026-07-27, before the session). → drives Phase 1: the next session must ASK, not self-pick.
- **The user enforced "board first, then ask" twice this session** and picked an option I had not ranked first. Following the rule produced better work than following my ranking. → drives Phase 1.
- **`CLAUDE.md` hit exactly 200/200 during #385** and had to be tightened back to 198. The parent handoff predicted this ("will breach within a few features"); it happened the same day. → drives Phase 2, and makes Phase 2 a **blocker** for any feature adding a module-map row.
- **All 14 smokes pass, but only 9 assert anything specific** — 5 are byte-size-only and now named in `docs/VERIFICATION.md`. The verification debt is closed; do not re-sweep unless the render/asset path changes. → removes work from the menu.
- **Audio-reactive has exactly one consumer** (`examples/audio_reactive`). VISION's "fix the API before release if it feels awkward while writing the example" has only been tested against the example written *by* the same session that designed the API. → drives Phase 3B.
- **Trap 4 (background notification reports the wrong exit code) fired twice in one session** despite being documented 12 h earlier. Three red gate runs, three distinct causes, and the notification lied about two of them. → drives Phase 4 and the verify discipline in every phase.
- **`ship-wasm-example` had a reproducible defect** (`chmod +x` only, invisible under `core.fileMode = false`) that would have re-shipped #378's bug; fixed this session. Any *new* script is still a fresh chance to get this wrong. → drives the Rollback/validation notes.
- **The engine's newest capabilities compose unusually well** — `bands()` (beat), `mapgen` (3 generators), `FovMap`, `Rng`, the widget suite — but nothing composes them together. → drives Phase 3A's genre recommendation.

## Anti-Goals (What NOT To Do)

- **Do NOT self-pick the next feature.** The board is empty; the standing rule is to ASK. This was exercised twice this session and once in the parent. Phase 1 ends in a question, not in code — the same shape as `PLAN_board-ew-triple_three-board-requests_2026-07-26.md` Phase 1.
- **Do NOT implement windowed frame capture.** The game declined it in writing on 2026-07-27 ("Headless-only capture is fine for us"). It needs `COPY_SRC` on the surface — a change affecting every app on every backend, disallowed on WebGL2 — for zero requested benefit. **Closed, not deferred.**
- **Do NOT add the `MapGenerator` trait.** Still an explicit anti-goal; its trigger (something wanting to swap generators at runtime) has not fired.
- **Do NOT re-sweep the 14 smokes.** Done this session, 14/14. Re-run individually only after touching the render path, the asset path, or a `web/build.sh`.
- **Do NOT move the module map without asking.** It buys ~80 lines but changes the primary navigation surface for every session. Present it as a decision (Phase 2), do not execute it unilaterally.
- **Do NOT trust a `run_in_background` completion notification's exit code.** Read the `.exit` file and check its mtime. This is not theoretical — it misreported twice this session.
- **Do NOT widen `bands()` with a caller-selectable FFT size** on speculation. The 43 Hz/bin resolution is documented; nothing has asked for more.

## Plan

### Phase 1: Board gate, then ask for direction

**Goal:** Establish whether external work exists and, if not, get an explicit direction from the user instead of inventing one.

**Why this approach:** The board has preempted self-picked work every time it was non-empty, and the user has twice corrected toward "ask". The parent chain's plan used this exact shape and it worked both times.

- Read `../dungeon-merchant/docs/engine-wishlist.md` — check **Active requests** (expected: empty, next free **EW-012**) and skim the `[Game]` verification comments on EW-009/010/011 for anything actionable that was not filed as a request.
- Read `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — **Open Requests** (expected `_None._`). This project is paused/deprecated; do not chase compatibility.
- Also run `cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md` to see whether the board moved since **2026-07-27**. A newly filed request **preempts every phase below**.
- If a request exists: triage it by priority (`P0` blocker → `P3` polish), acknowledge it on the board with an `[Engine]` line (append-only, absolute date), and treat Phases 2–4 as deferred.
- If both are empty: run Phase 2's *decision* question and the direction question **together** in one `AskUserQuestion` (Korean), so the user makes both calls in one round-trip rather than two.
- Present the menu with the recommendation reasoning, not just labels — three-for-three uptake this session says the framing is load-bearing.

**Files:** none (read-only)
**Validates with:** both board files read; a stated verdict ("empty on both channels, next free EW-012") and either a filed request or the user's choice recorded.
**Rollback:** n/a — no mutations.

### Phase 2: Resolve the CLAUDE.md 200-line cap (BLOCKS Phase 3)

**Goal:** Create headroom before a feature needs it, so the next module-map row does not force an ad-hoc squeeze mid-PR.

**Why this approach:** `CLAUDE.md` is at 198/200 and #385's first draft hit exactly 200. The prior session measured *why* the usual fix does not work: the module map is **80 of 198 lines because it has ~72 rows**, one per topic — a 2000-character row is still one line, so compressing row *text* buys nothing. Only prose sections shrink, and they have already been compressed twice (#379, #385). The remaining options are structural.

- Present the decision to the user (folded into Phase 1's question round):
  - **(a) Move the module map to `docs/MODULE_MAP.md`** and leave a one-line pointer. Buys ~79 lines. Cost: every session's primary navigation surface moves one hop away; `CLAUDE.md` stops being self-sufficient for "where do I find X".
  - **(b) Raise the cap** to 250 in the Documentation rules. Cost: the cap exists to bound per-session context; raising it is a real (if small) tax on every session.
  - **(c) Merge related module-map rows** (e.g. fold the three audio rows into one, the UI-widget rows into one). Buys ~5-10 lines without moving anything. Cost: rows become even denser, and density is already high.
- Whichever is chosen, execute it as a **docs-only PR with no package bump**, bumping only the `CLAUDE.md` doc version (v1.6.235 → v1.6.236) per the `/ship` rule.
- If (a): verify every `docs/*.md` cross-reference still resolves, and add `docs/MODULE_MAP.md` to the Document map table.
- Re-measure section line counts afterwards and record them in the PR body, so the next session inherits the measurement rather than re-deriving it.

**Files:** `CLAUDE.md`; possibly new `docs/MODULE_MAP.md`
**Validates with:** `wc -l CLAUDE.md` under the (possibly new) cap with ≥15 lines of headroom; `./scripts/verify.sh` exit **0** read from a non-piped command; all referenced doc paths resolve.
**Rollback:** single docs-only commit — `git revert` restores the inline map. No code depends on it.

### Phase 3: Execute the chosen menu item

Three mutually exclusive branches. **Only the one the user picks in Phase 1 gets built.**

#### 3A — A second capstone game (largest value; recommended if the user wants substance)

**Goal:** Exercise the engine's grown surface in *real play*, which is the only place VISION expects API awkwardness to surface.

**Why this approach:** 20 entries in `examples/games/` but the newest capabilities (audio analysis, the 3 procgen modes, `FovMap`, the widget suite) are each exercised by a single-purpose demo written alongside the feature. A capstone is the first test of them *composed*.

- **Recommended genre — a beat-driven dungeon crawler** (the *Crypt of the NecroDancer* shape). It is the one genre that composes the newest features non-trivially rather than decoratively: `Audio::bands()` low-band energy drives the beat clock, `generate_bsp_dungeon`/`generate_cellular_cave` build levels from a stored `Rng` seed, `FovMap::from_path_grid(&map.to_path_grid())` gives fog-of-war, `find_path` moves enemies on the beat, and the widget suite plus `FloatingText`/`HitFlash`/`Camera::shake` carry the HUD and feedback. **Flag this as a recommendation, not a decision** — genre is the user's call.
- Scope it to one playable loop: descend → move on the beat → fight → find the stair → next level. No meta-progression, no save.
- Put it at `examples/games/<name>/<name>.rs` with an explicit `[[example]]` entry (nested layout, as every capstone uses).
- Drive the beat from `bands()` low-half energy with a threshold + refractory window — reuse the edge-trigger shape from `examples/audio_reactive`'s kick detector (`flash` timer gating, so one hit counts once).
- **Expect to find API friction and fix it before shipping** — that is the point. Log each friction item in the PR body even when the fix is game-side, so the next reader can tell "engine gap" from "game code".
- Add a headless self-check (`ENGINE_CAPTURE` needs no code) and eyeball the PNG — three consecutive sessions have caught a real layout bug that way.

**Files:** `examples/games/<name>/`, `Cargo.toml` (`[[example]]`), `CLAUDE.md` module-map row (needs Phase 2 done first), `docs/CHANGELOG.md`
**Validates with:** the loop is playable natively; headless capture eyeballed; `./scripts/verify.sh` exit 0; any engine change carries its own unit test.
**Rollback:** the game is additive — delete the directory and the `[[example]]` entry. Revert any engine-side change separately so the capstone can land without it.

#### 3B — Adopt audio-reactive as a second consumer (smaller, sharpens the new API)

**Goal:** Stress `levels()`/`bands()` from code that was *not* written by their designer.

**Why this approach:** The API has exactly one consumer and it is the example built in the same session. `examples/games/survivor` and `shooter` already use the `Audio` facade, so adoption needs no new audio plumbing.

- Pick `survivor` (denser feedback loop than `shooter`). Meter its sfx bus or a named channel; drive `HitFlash` intensity and `Camera::shake` amplitude from `levels().peak` instead of fixed constants.
- **Expect to hit the `play_sfx` limitation immediately** — it round-robins 16 anonymous voices on native, so it *cannot* be metered. That is the interesting outcome: either the game moves its feedback sounds to a named channel, or this surfaces a real API gap (a meterable one-shot). **Do not paper over it; report which.**
- Keep the change small and reversible; if it does not improve feel, say so and revert rather than shipping churn.

**Files:** `examples/games/survivor/survivor.rs`, possibly `src/audio_facade.rs` if a gap is confirmed, `docs/CHANGELOG.md`
**Validates with:** the game runs and the effect is visibly audio-driven; `./scripts/verify.sh` exit 0; a written verdict on the `play_sfx` limitation.
**Rollback:** revert the game file; any engine addition ships as its own commit.

#### 3C — A fourth procgen mode (safest, lowest marginal value)

**Goal:** Add one more generator over the shared `DungeonMap`.

**Why this approach:** The pattern is proven three times and composes with `to_path_grid`/`to_tilemap_tiles`/`FovMap` for free. Recommended only if the user wants low risk — rooms/caves/mazes plus the `roguelike` capstone already cover the area, which is why it ranks last.

- **Drunkard's walk** is the best of the three candidates: ~80 lines, **connected by construction** (the walk itself is connected, so no keep-largest pass unlike the cave), and deterministic from `Rng`.
- Mirror `generate_cellular_cave`'s signature exactly: `generate_drunkard_walk(w, h, seed, &DrunkardParams) -> DungeonMap`, params for target floor fraction + max steps, cell count capped at `MAX_PATH_GRID_CELLS`.
- Record a 1×1 spawn `Room` at the walk's start so `first_room_center` works, as the cave and maze generators do.
- Tests: determinism (same seed → identical map), connectivity (single flood-filled region), the floor-fraction target within tolerance, over-cap → empty + `error!`.
- Example `drunkard_walk` following `maze_generation`'s shape (WASD walk, R regenerate, headless connectivity self-check).

**Files:** `src/mapgen.rs`, `src/lib.rs`, `examples/drunkard_walk.rs`, `CLAUDE.md` (needs Phase 2), `docs/CHANGELOG.md`
**Validates with:** 4+ unit tests pass; headless self-check asserts a single connected region; `./scripts/verify.sh` exit 0.
**Rollback:** purely additive — remove the function, the re-export and the example.

### Phase 4: Cheap verification-hygiene leftovers (optional, any time)

**Goal:** Apply the two `/wrap` candidates that were proposed and not taken.

**Why this approach:** Both are small and both address things that actually bit. Deliberately last because neither blocks anything.

- **Candidate 5 (recommended first):** add a copy-paste-ready block to `docs/VERIFICATION.md` Trap 4 — `rm -f` the exit file, run non-piped, wait on the PID, read the file, check its mtime. The procedure worked both times it was needed this session; the cost is re-assembling it by hand each time. Optionally instead: `scripts/verify_bg.sh` wrapping the whole dance (remember `git update-index --chmod=+x`).
- **Candidate 4:** one-line footnote in `docs/PATTERNS.md` — using a new `web_sys` type requires adding it to the `Cargo.toml` web-sys `features` list. Low severity (the wasm lib build catches it) but rare enough to be forgotten: `git log -S'web-sys' -- Cargo.toml` shows only **two** such edits ever.

**Files:** `docs/VERIFICATION.md`, `docs/PATTERNS.md`, possibly `scripts/verify_bg.sh`
**Validates with:** `./scripts/verify.sh` exit 0; if a script is added, `git ls-files -s` shows `100755`.
**Rollback:** docs-only; revert the commit.

## Dependencies & Order

- **Phase 1 gates everything.** A filed board request preempts Phases 2–4 entirely.
- **Phase 2 blocks Phase 3A and 3C** — both add a `CLAUDE.md` module-map row, and there are 2 lines of headroom. 3B may not need a row (adoption, not new API), so 3B can precede Phase 2.
- **Phase 2's decision should be asked together with Phase 1's** direction question — one round-trip, two answers.
- **Phase 4 is independent** and can run at any point, including as filler while waiting on a decision.
- Within Phase 3, the branches are **mutually exclusive**. Do not build two.

## Risks & Mitigations

- **Self-picking instead of asking.** Likely if the next session reads "execute Phase 1" as "start coding" — the paste prompt is deliberately worded against this. Mitigation: Phase 1 has no files and no code; its deliverable is a recorded answer.
- **A false-green gate.** Demonstrated **twice this session** in the same documented mechanism. Mitigation: `rm -f` the exit file first, read it (never the notification), check its mtime, and corroborate with the `ok`-group count (expect **151**) and lib-test count (expect **1338** at baseline).
- **A new script shipping non-executable.** The skill is fixed but the trap is per-file. Mitigation: `chmod +x` **and** `git update-index --chmod=+x`, verify with `git ls-files -s '*.sh'` (expect all `100755`; currently 33/33).
- **A capstone (3A) sprawling past one session.** Moderate-to-likely — it is the largest item on the menu. Mitigation: scope to a single playable loop up front, land the engine-side fixes as separate PRs as they are found, and write a handoff before context runs low rather than after.
- **An example broken for wasm without anyone noticing.** The gate builds lib+bins only. Mitigation: if the new work claims web support, `cargo build --example <name> --target wasm32-unknown-unknown` explicitly, and run its smoke — `embedded_image` was broken from the day it was added because nothing ran it.
- **`engine-current-state` memory hitting the read cap.** It is 34.2 KB and grows ~1.5 KB/seq; the cap is ~76 KB and it has been hit once already. Mitigation: trim the chain tail into `engine-history-archive` around seq 210, not at the ceiling.

## Success Criteria

- **Minimum viable:** the board gate is run and stated on both channels; the direction question is asked (or a filed request is triaged); nothing is self-picked. Phase 2's decision is put to the user with the (a)/(b)/(c) trade-offs.
- `CLAUDE.md` ends with **≥15 lines of headroom** under its cap (from 2 today), and the measurement is recorded in the PR body.
- The chosen Phase 3 branch ships with `./scripts/verify.sh` exit **0**, lib tests **≥1338** (never fewer), and — if it touches an example claiming web support — an explicit wasm example build plus its smoke.
- If 3A: a playable loop, a headless capture that was **eyeballed**, and every API friction item written down (engine gap vs game code, explicitly labelled).
- If 3B: a written verdict on whether `play_sfx`'s unmeterable round-robin is a real API gap or acceptable.
- Memory advanced to at least **seq 204** with the recorded `main @ <hash>`, and `MEMORY.md`'s index hook refreshed.

## Quick Start

```bash
# Restore full context
cat plans/handoffs/HANDOFF_board-ew-triple_audio-reactive-levels-spectrum_2026-07-30.md

# 1. BOARD FIRST — this is Phase 1 and it is not optional
cat ../dungeon-merchant/docs/engine-wishlist.md          # Active requests; next free EW-012
cat ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md     # expect _None._
cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md && cd -
# ^ if the date is later than 2026-07-27, the board moved — read it before anything else

# 2. Key files for the phases
#   CLAUDE.md                     — 198/200 lines; the Phase 2 subject
#   docs/VERIFICATION.md          — 6 traps + 3 blind spots + smoke classification
#   docs/PATTERNS.md              — the 2 new patterns end "Core architecture patterns"
#   src/audio_analysis.rs         — the shared-policy pattern in practice (3A/3B)
#   src/mapgen.rs                 — 3 generators to mirror (3C)

# 3. Verify starting state — read the FILE, not any notification
rm -f /tmp/v.exit /tmp/v.log
./scripts/verify.sh > /tmp/v.log 2>&1; echo "VERIFY_EXIT=$?"
# expect 0 · 151 ok groups · 1338 lib tests

# 4. FIRST CONCRETE ACTION
#    Read both board files, state the verdict, then ask ONE AskUserQuestion (Korean) carrying
#    BOTH decisions: (i) the CLAUDE.md cap — move the module map / raise the cap / merge rows,
#    and (ii) the direction — 2nd capstone (beat-driven crawler recommended) / adopt
#    audio-reactive in survivor / 4th procgen mode (drunkard's walk).
#    Windowed capture is OFF the menu. Do NOT self-pick.
```
