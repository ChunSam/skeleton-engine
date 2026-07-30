# Next: board gate, then one branch — with a real API request now on the table for the first time in a while

**Date:** 2026-07-30
**Status:** PLANNED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `audio-adoption` seq `1`
**Context:** See `HANDOFF_audio-adoption_survivor-audio-reactive-adoption_2026-07-30.md` for session data, all measurements, the four findings and the verify-gate history.

---

## Problem Statement

The audio-reactive API now has an independent consumer and survived — but not unchanged. Adopting it in `examples/games/survivor` (v0.139.0, PR #393, merged `e7378e1`) took **three design revisions**, each forced by a measurement, and produced a **named, evidence-backed API gap**: today *meterability and overlap are mutually exclusive*, because only a stable channel name can be metered and a named-channel replay calls `stop_immediate` and cuts the sound already there. That is the first genuinely API-shaped request to come out of this area in several sessions, and unlike the usual menu items it has data behind it rather than a hunch.

Meanwhile **both board channels are empty again** (next free **EW-012**; `rust-survivors` `_None._`; board unmoved since 2026-07-27), so there is once more no externally-driven work, and the standing rule is to ASK rather than self-pick. The two housekeeping items that had deadlines are both discharged: the `CLAUDE.md` size cap (seq 205) and the memory trim (this session, 45,519 → 29,568 bytes; next trim due ~seq 220, not now).

See Evidence & Data in the handoff for the numbers behind all of this.

## Key Findings

- **Both board channels are empty and the board has not moved since 2026-07-27** (verified by `git log`, not by eyeballing the file). → drives Phase 1: ASK, do not self-pick.
- **The user has now taken the recommendation 6 times out of 6** across three sessions. The framing — what breaks otherwise, what it actually costs — is load-bearing, not decoration. → drives Phase 1's presentation.
- **A real API gap is now named and measured**: meterability vs overlap. `play_sfx` / `play_sfx_on_bus` / `play_tone` / `play_tone_on_bus` all share one ring of 16 anonymous voices; the meterable alternatives cut on replay. → drives Phase 2A.
- **`arm/re-arm` does not transfer from a discrete beat clock to a continuous stream** — 1 fire in 300 frames vs 25 with a cooldown. A pattern the engine's own docs had implicitly generalised turned out to be conditional. → drives the caution in 2A and 2C.
- **Metering audio the game itself authored is a round-trip** — measured, not argued: keyed to kills-per-frame the tone had exactly one amplitude (1 kill in 40/40 kill frames, peak pinned at 0.230). → drives 2C's framing (a real soundtrack is the honest use case).
- **`survivor` has no scenes**, so its missing `register_persistent::<Audio>()` is inert and is **not** a third instance of the deferred `Audio` risk. → the deferred decision stays closed; see Anti-Goals.
- **`insert_core_resources` has still never been audited** against "which of these would a game set once and expect to keep". `WindowConfig` was found by accident, twice worked around. → drives Phase 2B, carried unbuilt from the predecessor plan.
- **The adoption is regression-silent**: `survivor` has no `SELFTEST` mode, and the probes that verified it were removed. → drives the optional hardening noted in 2A/2C.

## Anti-Goals (What NOT To Do)

- **Do NOT self-pick the next feature.** The board is empty; Phase 1 ends in a question, not code. This shape has now worked four sessions running.
- **Do NOT reopen the deferred `Audio` auto-persistence decision.** `survivor` did not add a third instance — it has no scenes, so the risk cannot fire there. Reopen only on a real trigger from `docs/PATTERNS.md`.
- **Do NOT implement windowed frame capture.** The game declined in writing 2026-07-27. Closed, not deferred.
- **Do NOT add the `MapGenerator` trait.** Its trigger (wanting to swap generators at runtime) still has not fired; `beat_crawler` swapping by depth parity did not need it.
- **Do NOT widen `bands()` with a caller-selectable FFT size.** Two capstones have now not needed it.
- **Do NOT migrate the rest of `survivor`'s tones to named channels.** Measured and deliberate: the bullet tone fires every 0.14 s and would be cut on every shot. Meterability is a per-sound decision.
- **Do NOT "fix" HUD text z-order.** Investigated and disproved this session with pixel sampling — the engine behaves as `DrawText::z` documents.
- **Do NOT trust a `run_in_background` completion notification's exit code.** It has now lied in four consecutive sessions. Use the block in `docs/VERIFICATION.md`.
- **Do NOT re-sweep the 14 smokes.** Done seq 202, 14/14. Re-run individually only after touching the render path, the asset path, or a `web/build.sh`.

## Plan

### Phase 1: Board gate, then ask for direction

**Goal:** Establish whether external work exists and, if not, get an explicit direction rather than inventing one.

**Why this approach:** The board has preempted self-picked work every time it was non-empty, and the ask has produced better choices than my own ranking for four sessions running.

- Read `../dungeon-merchant/docs/engine-wishlist.md` — **Active requests** (expected: empty, next free **EW-012**) and skim the `[Game]` comments for anything actionable that was never filed.
- Read `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — **Open Requests** (expected `_None._`). Paused/deprecated; do not chase compatibility, but it is still a real bug-report channel.
- Run `cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md`. Later than **2026-07-27** means the board moved — read it before anything else. **A filed request preempts every phase below.**
- Verify the starting state with the copy-paste block in `docs/VERIFICATION.md`. Read the exit code **from the file**; expect **0 · 152 ok groups · 1339 lib tests** at v0.139.0.
- If a request exists: triage by priority, acknowledge on the board with an append-only `[Engine]` line and an absolute date, and defer Phase 2.
- If both are empty: ask **one** `AskUserQuestion` (Korean) carrying the Phase 2 menu **with the trade-off reasoning, not just labels**. Lead with 2A and say why it is different from the usual menu — it is the first item in several sessions backed by measurement rather than a hunch.

**Files:** none (read-only)
**Validates with:** both board files read; a stated verdict ("empty on both channels, next free EW-012, unmoved since 2026-07-27") and either a filed request or the user's recorded choice.
**Rollback:** n/a — no mutations.

### Phase 2: Execute the chosen menu item

Four mutually exclusive branches. **Only the one the user picks in Phase 1 gets built.**

#### 2A — A meterable one-shot (recommended)

**Goal:** Remove the meterability-vs-overlap trade-off that the `survivor` adoption proved is real.

**Why this approach:** It is the only menu item that comes from measurement rather than speculation. The constraint was hit by an actual consumer, documented in three places (the example's module docs, `enable_analysis`, the module map), and worked around rather than solved. Every other branch is a nice-to-have.

- Read `src/audio_facade.rs:88-230` (the ring: `SFX_VOICES`, `sfx_voice_channel`, `next_sfx_channel`, the four callers) and `src/audio/playback.rs:178-217` (`stop_immediate` at :180 is the cut; `tapped` at :214 is the meter insertion point).
- **Design decision to settle first, and it is the whole PR:** a meterable one-shot needs a stable *identity* for metering while allocating a *fresh voice* per trigger. Two candidate shapes — (a) a named channel with a `polyphonic` flag that appends instead of calling `stop_immediate`, its meter summing the live voices; (b) a new `play_sfx_metered(name, bytes) -> ()` that owns a small per-name voice pool. **Prefer (a)**: it reuses the existing channel/bus/effect plumbing and does not add a second naming scheme. Confirm with the user before building — this is a public API addition.
- Metering semantics must be stated explicitly, because they are not obvious once voices overlap: does `levels(name)` report the **loudest live voice** or the **sum**? Sum is the honest answer for "how big was that", but it can exceed 1.0 — decide and document, and pin it with a test.
- **Watch the native/wasm asymmetry.** Per `docs/PATTERNS.md`, any derived value both backends compute must live in ONE un-gated module. If the voice-pool policy differs between rodio and Web Audio, the *policy* still goes in `src/audio_analysis.rs`.
- Tests: device-free unit tests in the style of the existing `audio_analysis` ones (a synthesized signal with a known RMS). Prove overlap is preserved (two triggers → both audible) and that the meter responds to the second trigger without the first being cut.
- **The example is the acceptance test** (VISION rule): the cheapest real one is to flip `survivor`'s bullet tone onto the new API and confirm it is both meterable and still overlapping — which directly retires a documented workaround.
- If the design turns out to be more invasive than one facade method plus a playback change, **stop and report** rather than growing the PR. A written "here is why it is not one line" is a valid outcome.

**Files:** `src/audio_facade.rs`, `src/audio/playback.rs`, possibly `src/audio_analysis.rs` and `src/audio_wasm.rs`, `examples/games/survivor/survivor.rs`, `docs/MODULE_MAP.md`, `docs/CHANGELOG.md`
**Validates with:** a test proving two overlapping triggers on one metered name; `./scripts/verify.sh` exit 0; lib tests **>1339** (this branch must add tests); a headless capture on a real device showing the meter respond.
**Rollback:** additive — remove the new method, its tests, and revert `survivor`'s bullet tone to `play_tone`.

#### 2B — Audit `insert_core_resources` for other scene-reset-fragile config

**Goal:** Find out whether `WindowConfig` was the only engine-inserted resource silently reverting on a scene change.

**Why this approach:** It was found **by accident** while writing a capstone, after the engine had documented and worked around it twice. That is not a search strategy. Cheap, bounded, evidence-driven — and carried unbuilt from the predecessor plan.

- Read `src/app/core_resources.rs` and list every resource `insert_core_resources` inserts.
- Classify each: **scene state** (correctly dies with the scene) vs **session/config state** (a game sets it once and expects it to persist). `docs/PATTERNS.md` "Surviving a scene reset" defines the test.
- Cross-check the survivors list in that doc (`WindowConfig`, `SceneTransition`, `TextMeasurer`, `InputScript`, the 7 RON registries, `DebugUi` by hand).
- Prime suspects, by the logic that made `WindowConfig` wrong: `DesignResolution`, `WindowOptions`, `FrameConfig`, `LightingConfig`, `FocusRingStyle`, `StickNavConfig`, `DialogueStyle`, `TimeScale`.
- For each genuine finding: one line in `App::new` + a regression test **proven non-vacuous by disabling the fix**, exactly as #388 did. Land as one PR if uniform, one per resource if they need argument.
- If the answer is "none", **say so and write it into `docs/PATTERNS.md`** — a negative result stops the next session re-running the audit. This is an explicitly valid outcome, not a failure.

**Files:** `src/app.rs`, `src/app/core_resources.rs`, `docs/PATTERNS.md`, `docs/CHANGELOG.md`
**Validates with:** every inserted resource classified in writing; any fix carries a test that fails without it; `./scripts/verify.sh` exit 0.
**Rollback:** each fix is one line + one test; revert individually.

#### 2C — A real soundtrack for `beat_crawler`

**Goal:** Meter a mixed track rather than isolated synthesized tones — the actual use case `bands()` exists for.

**Why this approach:** This session proved that metering audio the game itself authors is a round-trip. `beat_crawler` already meters audio no gameplay code sees, which is the right shape, but its "soundtrack" is two synthesized tones chosen to be trivially separable (kick 4.00 vs blip 0.61, 6.5× apart). A real track is the first case where the low-band detector could genuinely fail.

- Needs a **licence-clean** audio file. The CC0 synthesis recipe in `src/audio/fixtures/README.md` solved exactly this for the codec tests — synthesize from scratch rather than sourcing.
- **Expect the kick detector to need retuning**, and expect the arm/re-arm pattern to come under pressure: a dense track's low bands may never fall below `REARM_THRESHOLD`. This session's cooldown finding is the likely fix, and confirming that would generalise it a second time.
- Keep `BEAT_CRAWLER_SELFTEST=1` green and extend it — the kick-heard/blip-rejected assertion needs a real-track analogue.
- Do **not** let this become "a third capstone". One addition, kept small.

**Files:** `examples/games/beat_crawler/beat_crawler.rs`, `examples/assets/` (or a synthesized fixture), `docs/CHANGELOG.md`
**Validates with:** `BEAT_CRAWLER_SELFTEST=1` exits 0; the turn clock tracks the real track; `./scripts/verify.sh` exit 0.
**Rollback:** revert the track + threshold constants; the tone scheduler is still there.

#### 2D — A fourth procgen mode (safest, lowest marginal value)

**Goal:** Add one more generator over the shared `DungeonMap`.

**Why this approach:** Proven three times and composes with `to_path_grid`/`to_tilemap_tiles`/`FovMap` for free. Ranks last: rooms/caves/mazes plus `roguelike` and `beat_crawler` already cover this area, so a fourth teaches little.

- **Drunkard's walk** remains the best candidate: ~80 lines, **connected by construction**, deterministic from `Rng`.
- Mirror `generate_cellular_cave`'s signature exactly: `generate_drunkard_walk(w, h, seed, &DrunkardParams) -> DungeonMap`; params for target floor fraction + max steps; cap cells at `MAX_PATH_GRID_CELLS`.
- Record a 1×1 spawn `Room` at the walk's start so `first_room_center` works — `beat_crawler` proved why that matters (room-based placement degenerates when a generator records one room).
- Tests: determinism, connectivity (single flood-filled region), floor-fraction within tolerance, over-cap → empty + `error!`.
- Example `drunkard_walk` following `maze_generation`'s shape (WASD, R regenerate, headless connectivity self-check).

**Files:** `src/mapgen.rs`, `src/lib.rs`, `examples/drunkard_walk.rs`, `docs/MODULE_MAP.md`, `docs/CHANGELOG.md`
**Validates with:** 4+ unit tests; headless self-check asserts a single connected region; `./scripts/verify.sh` exit 0.
**Rollback:** purely additive — remove the function, the re-export and the example.

### Phase 3 (conditional): land an engine change as its own PR

**Goal:** Keep a public-API or engine-behavior change separable from the example that motivated it.

**Why this approach:** #388 established it — the `WindowConfig` fix landed separately from the capstone that found it, so it could stand or revert on its own. **2A and 2B are the only branches that can produce an engine change**; 2C and 2D are additive example/module work and do not need this.

- If 2A yields a new public method: land the engine addition + its tests as PR 1, and the `survivor` adoption of it as PR 2.
- If 2B yields fixes: one PR if uniform, one per resource if each needs argument. Every fix carries a test **proven to fail without it**.
- Version per `CLAUDE.md`: MINOR for an additive API, PATCH for a pure bugfix. Use `/ship` for the four-file paperwork, `/land-pr` for the branch→CI→merge loop.

**Files:** as per the branch
**Validates with:** each PR green independently; `./scripts/verify.sh` exit 0 before each push.
**Rollback:** revert the engine PR without touching the example PR.

## Dependencies & Order

- **Phase 1 gates everything.** A filed board request preempts Phase 2 entirely.
- **Phase 2's branches are mutually exclusive. Do not build two.**
- **Phase 3 only fires if Phase 2 chose 2A or 2B** and that branch actually produced an engine change.
- **2A has an internal gate**: the API shape must be agreed with the user before implementation, because it is a public addition. Do not build shape (a) or (b) without confirming.
- **No mandatory housekeeping this time.** Unlike the last two plans, both deadline items are discharged — the memory file is at 29.6 KB against a ~76 KB cap and does not need trimming until ~seq 220.

## Risks & Mitigations

- **Self-picking instead of asking.** Likely if the next session reads "execute Phase 1" as "start coding". Mitigation: Phase 1 has no files and no code; its deliverable is a recorded answer.
- **A false-green gate.** Has now fired in **four consecutive sessions**, most recently as a notification arriving within one tool round of launching. Mitigation: the `docs/VERIFICATION.md` block — `rm -f` first, run non-piped, wait on the file, read it, check mtime, corroborate **152 groups / 1339 lib tests** at v0.139.0.
- **2A growing past one PR.** Moderate-to-likely: polyphonic metering touches the voice model on two backends. Mitigation: the branch has an explicit stop condition — if it is not roughly one facade method plus a playback change, report why instead of expanding.
- **2A's meter semantics being decided implicitly.** Sum-vs-max is easy to leave to whatever the first implementation does. Mitigation: decide it in writing, document it on the method, and pin it with a test before wiring any example.
- **`cargo fmt` reflow defeating a scripted edit.** Bit this session on a probe removal that reported success while the probe survived. Mitigation: run `cargo fmt` before the first verify, and verify any scripted deletion with an independent `grep`, never with the script's own exit code.
- **Audio work is not covered by CI.** Green CI cannot prove a meter reads anything. Mitigation: any metering change needs a headless `ENGINE_CAPTURE` run on a machine with a real device — confirm the rodio `DeviceSink` teardown line appears, or the run proved nothing.
- **Stale rust-analyzer diagnostics claiming compile errors.** Fired again this session (a phantom `E0107` in `beat_crawler`, in code that had just passed the full gate). Mitigation: trust `cargo build` / `cargo clippy` exit codes only.

## Success Criteria

- **Minimum viable:** the board gate is run and stated on both channels; the direction question is asked (or a filed request is triaged); nothing is self-picked.
- The chosen Phase 2 branch ships with `./scripts/verify.sh` exit **0**, `ok` groups **≥152**, and lib tests **≥1339** — never fewer.
- **If 2A:** lib tests **>1339** (this branch must add them); a test proving two overlapping triggers on one metered name; the sum-vs-max semantics documented on the method; `survivor`'s bullet-tone workaround retired or a written reason why not.
- **If 2B:** every resource `insert_core_resources` inserts is classified **in writing** — including a written "none found" if that is the answer — and any fix carries a test proven to fail without it.
- **If 2C:** a licence-clean track, `BEAT_CRAWLER_SELFTEST=1` still exit 0, and a written verdict on whether arm/re-arm survives a real track or needs this session's cooldown.
- **If 2D:** 4+ unit tests including a single-connected-region assertion, plus a playable example.
- Memory advanced to at least **seq 212** with the recorded `main @ <hash>`, and `MEMORY.md`'s index hook refreshed. Note the recorded tip will be one commit stale if the handoff lands as its own PR, as it always has.

## Quick Start

```bash
# Restore full context
cat plans/handoffs/HANDOFF_audio-adoption_survivor-audio-reactive-adoption_2026-07-30.md

# 1. BOARD FIRST — this is Phase 1 and it is not optional
cat ../dungeon-merchant/docs/engine-wishlist.md          # Active requests; next free EW-012
cat ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md     # expect _None._
cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md && cd -
# ^ later than 2026-07-27 = the board MOVED; read it before anything else

# 2. Verify starting state — the block docs/VERIFICATION.md carries.
#    Read the exit code from the FILE. The notification has lied in FOUR straight sessions.
rm -f /tmp/v.exit /tmp/v.log
(./scripts/verify.sh > /tmp/v.log 2>&1; echo $? > /tmp/v.exit) &
until [ -f /tmp/v.exit ]; do sleep 20; done
echo "EXIT=$(cat /tmp/v.exit)"; ls -l /tmp/v.exit
echo "ok groups: $(grep -c 'test result: ok' /tmp/v.log)"
# expect 0 · 152 ok groups · 1339 lib tests   (v0.139.0)

# 3. Key files for the phases
#   src/audio_facade.rs           — the ring (88-205), play_tone_on_channel (212), enable_analysis (~465)  [2A]
#   src/audio/playback.rs         — stop_immediate (180) = the cut, tapped (214) = the meter tap           [2A]
#   src/app/core_resources.rs     — the audit subject                                                     [2B]
#   docs/PATTERNS.md              — "Surviving a scene reset" (2B's classification test)
#   examples/games/survivor/survivor.rs — this session's adoption + the 4 findings in its module docs
#   docs/MODULE_MAP.md            — GREP IT, never read it whole (72 rows, ~89 KB)

# 4. FIRST CONCRETE ACTION
#    Read both board files and state the verdict. If empty, ask ONE AskUserQuestion (Korean)
#    with the reasoning, leading with 2A and saying why it differs from the usual menu:
#      a meterable one-shot API (RECOMMENDED — the first item in several sessions backed by
#        measurement, not a hunch: meterability and overlap are provably exclusive today) /
#      audit insert_core_resources for more WindowConfig-class scene-reset bugs /
#      a real licence-clean soundtrack for beat_crawler /
#      a 4th procgen mode (drunkard's walk)
#    Do NOT self-pick. Phase 1 ends in a question, not code.
```
