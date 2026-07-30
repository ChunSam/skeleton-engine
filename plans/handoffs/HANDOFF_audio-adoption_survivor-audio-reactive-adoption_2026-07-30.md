# survivor adopts audio-reactive game feel — the first independent consumer, and the four findings it produced

**Date:** 2026-07-30
**Status:** COMPLETED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `audio-adoption` seq `1`
**Parent:** `none — first in chain (deliberate tag reset, see Related Handoffs)`
**Prior chain:** none — first in chain

---

## Related Handoffs

This session executed the plan in the **`board-ew-triple` seq-4** pair, but starts a **new chain tag** rather than continuing that one. That was the predecessor's own recommendation, not drift:

- `PLAN_board-ew-triple_module-map-and-beat-crawler_2026-07-30.md` — the plan this session executed (Phases 1/2/3A). Its Dependencies section said: *"`board-ew-triple` has been historical since seq 2 and the arc has closed. **Starting a fresh tag at seq 1 is reasonable and should be stated explicitly** in the next handoff rather than continuing out of habit."*
- `HANDOFF_board-ew-triple_module-map-and-beat-crawler_2026-07-30.md` — its paired session data (531 lines). Read it for the module-map extraction, the `WindowConfig` scene-reset fix, and the `beat_crawler` capstone.

**Why the tag changed:** `board-ew-triple` was named for three `dungeon-merchant` board requests (EW-009/010/011) that were closed and archived on 2026-07-27. Everything after its seq 2 was carried under that tag out of habit. The arc it named (audio-reactive → 2nd capstone → the CLAUDE.md size cap) is fully closed. This session opens a genuinely new one: **adoption of the audio-reactive API by code that did not design it**, which is a question with follow-on work.

**Plan-vs-reality against that predecessor:**

- Its Phase 1 (board gate → ASK, do not self-pick) — ran exactly as written. Board empty on both channels, user asked, recommendation taken.
- Its Phase 2 (trim the memory regardless of the answer) — done, 45,519 → 29,568 bytes.
- Its Phase 3A (adopt audio-reactive in `survivor`) — chosen by the user and shipped, but **its stated premise was wrong in two ways** (see What We Tried, Chunk 3). It predicted the session would "hit `play_sfx`'s unmeterable 16-voice round-robin immediately" and that we would "drive `HitFlash` intensity and `Camera::shake` amplitude from `levels().peak` **instead of fixed constants**." `survivor` never calls `play_sfx`, and it had no `HitFlash` and no `Camera::shake` at all — there were no constants to replace.
- Its open question *"is the missing `register_persistent::<Audio>()` a third instance of the deferred `Audio` risk?"* — **answered: no.** `survivor` has no scenes.

## Reference Documents

- `CLAUDE.md` — project conventions, the verify gate, versioning (pre-1.0)
- `docs/MODULE_MAP.md` — 72 rows, "where do I read to find X". **Grep it, never read it whole** (~89 KB)
- `docs/VERIFICATION.md` — the 6 exit-code traps + 3 blind spots; carries the copy-paste gate block
- `docs/PATTERNS.md` — architecture patterns; "Surviving a scene reset" + the deferred `Audio` decision and its reopen triggers
- `docs/CHANGELOG.md` — 0.139.0 entry written this session

---

## The Goal

Answer a question the engine could not answer about itself: **does the audio-reactive metering API (`Audio::enable_analysis` / `levels` / `bands`, shipped v0.136.0–v0.137.0) survive contact with code that did not design it?** Both existing consumers — `examples/audio_reactive` (v0.136.0) and `examples/games/beat_crawler` (v0.138.0) — were written by the same sessions that had just built or just read the API, so neither was evidence. `examples/games/survivor` is older, denser, single-file, and already used the `Audio` facade, making it the first genuinely independent consumer.

The deliverable was explicitly **the verdict, not just the code**: the predecessor plan said "the `play_sfx` verdict IS the deliverable... Do not paper over it." Secondary goal: trim the `engine-current-state` memory file before it hit the ~76 KB read cap it had already struck once on 2026-07-29.

## Where We Are

- **Everything is landed and merged. Tree clean, `main` @ `e7378e1`, package v0.139.0, CLAUDE.md header v1.6.240.** Nothing is in flight.
- **PR #393 merged at 2026-07-30T13:46:21Z**, squash, all **6/6 CI checks green**. Local branch `feat/survivor-audio-reactive-feel` deleted.
- **Board gate ran and was empty on both channels.** `dungeon-merchant` Active requests: `*(none — all filed requests are closed)*`, next free **EW-012**, board last touched **2026-07-27** (`539b6100`, unmoved). `rust-survivors`: `_None._`, last touched 2026-07-14.
- **The user chose 3A from a 4-option Korean `AskUserQuestion`** — the recommended option. That is **6 recommendations taken out of 6** across three sessions.
- **`examples/games/survivor/survivor.rs`** is the only behavioral change: +274 lines net across the whole PR, one example file.
- **New in survivor:** `KILL_CHANNEL` (`"kill"`), an `AudioFeel` resource, an `AudioFeelSystem` (registered after `CollisionSystem`, before `ParticleSystem`), a `play_tone_named` helper, a `drive_from_amplitude` mapping function, a second HUD row, and 10 tuning constants.
- **The kill tone moved** `play_tone_on_bus(150,0.14,0.28,"sfx")` → `play_tone_on_channel("kill",150,0.14,vol,"sfx")` where `vol = clamp(KILL_VOL_BASE + KILL_VOL_PER_KILL*combo, ≤ KILL_VOL_MAX)`.
- **The bullet tone (900 Hz, every 0.14 s) and the death tone (110 Hz) deliberately stay on the anonymous ring** — moving them would cost sound overlap for no gain.
- **Engine library changes are doc-comment only.** `src/audio_facade.rs`: `enable_analysis`'s "Which channels can be metered" and `play_tone_on_bus`. **No API change**, no new tests, test counts unchanged.
- **`docs/MODULE_MAP.md` audio-reactive row was EXTENDED, not added** (house rule: "extend an existing row before adding one").
- **Verify gate ran 3× this session, all exit 0**, all read from the exit *file*: start-of-session, post-implementation, post-version-bump. **152 ok groups / 1339 lib tests every time, unchanged** — correct for an example+docs change.
- **Memory trimmed 45,519 → 29,568 bytes** (−35%), seqs 197→185 moved verbatim into `[[engine-history-archive]]` (319,336 → 336,783 bytes). Reads back in a single `Read`. Memory advanced to **seq 211**.
- **Trap 4 (background notification reports the wrong exit code) fired again**, on the very first gate launch of the session — now **four consecutive sessions**. The `docs/VERIFICATION.md` copy-paste block worked; reading the file caught it.
- **A NEW variant of the `cargo fmt` reflow trap was found:** `cargo fmt` reflowed a one-line temporary `eprintln!` probe onto multiple lines, so the regex written to delete it silently matched nothing — the probe survived a removal that reported success.
- **One claim I made mid-session was wrong and I disproved it myself:** I asserted world sprites draw over `z: None` HUD text. Pixel sampling showed text glyphs interleaved *inside* the sprite band. The engine behaves exactly as `DrawText::z` documents.
- **`survivor` has no scenes at all** — zero `set_scene` / `SceneTransition` / `register_persistent`. Its missing `register_persistent::<Audio>()` is therefore **inert**, and this is **not** a third instance of the deferred `Audio` risk. The STANDING/DEFERRED decision was correctly left untouched.
- Four temporary probes were added and removed during the session; **`grep` confirms none remain** in the shipped file.

## What We Tried (Chronological)

### Chunk 1 — Board gate, the false-green notification, and the ask (early)

1. **Launched the verify gate in the background and read the board in parallel.** Within one tool round the harness reported `Background command "Launch verify gate" completed (exit code 0)`. That is **Trap 4**: the wrapper's trailing `echo $? > /tmp/v.exit` exits immediately, so the notification describes the wrapper, not `verify.sh`. Ignored it and waited on the file. (Fourth consecutive session this has fired.)
2. **Read both board channels read-only.** The `dungeon-merchant` board file is 64.4 KB — the raw `cat` overflowed the tool result and was persisted to a file. Re-extracted just the relevant sections with `awk '/^## Active requests/,/^## Done \/ archive/'` rather than re-reading the whole thing.
3. **Checked whether the board had MOVED**, which was the actual gate condition: `git log -1 --date=short -- docs/engine-wishlist.md` → **2026-07-27**, not later. Board unmoved; no preemption.
4. **Read the verify result from `/tmp/v.exit`, not the notification.** `EXIT=0`, mtime 13:13 (fresh — `rm -f` ran in the same command that launched it, so no stale-file risk), 8 `[verify]` step markers present including the final `all checks passed ✓`, 152 ok groups, 1339 lib tests. Corroborated the counts against the expected 152/1339, per the newly-added `docs/VERIFICATION.md` step.
5. **Sanity-checked that all gate steps actually ran**, not just the early ones — `grep -n '\[verify\]'` showed all 8, including `cargo test --doc` (line 2339) and `RUSTDOCFLAGS=-D warnings cargo doc` (line 2501). A gate that dies early can still exit 0 if the script is wrong; this is cheap insurance.
6. **Asked ONE `AskUserQuestion` in Korean** with all four Phase-3 branches and the *reasoning* for each, not just labels — the predecessor handoff recorded that the framing is load-bearing (the user had taken the recommendation 5/5). Stated explicitly that Phase 2 was not a choice and that windowed capture was off the menu (the game declined it in writing 2026-07-27).
7. **User picked the recommended option**, `survivor 오디오 채택`. 6 of 6.

### Chunk 2 — Phase 2: the memory trim (early-mid)

8. **Copied all three memory files to the scratchpad BEFORE editing.** They are not version-controlled; the plan called this out as the rollback. `BACKUP_engine-current-state.md` 45,519 B, `BACKUP_engine-history-archive.md` 319,336 B, `BACKUP_MEMORY.md` 5,385 B.
9. **Measured before cutting.** Per-line bytes showed the file is 18 lines but **line 9 alone is 40,428 of 45,519 bytes** (89%). The tip line is one enormous paragraph, so any edit had to be a surgical single-occurrence replace — which the file's own header instructs.
10. **Segmented the tip chain by `seq NNN` markers and measured each entry** to choose a cut point from data rather than by feel. Table in Evidence. Largest single entries: the seq-200/201 audio-reactive block, seq 207 (`beat_crawler`, 2,605 B), seq 185 (2,963 B), seq 193 (2,111 B).
11. **Computed the resulting file size for every candidate cut point** rather than guessing. Keeping 210→194 (the full `board-ew-triple` chain) landed at ~32.8 KB — under the 40 KB bar but outside the 25–30 KB target. Moving 197→185 landed at ~28.7 KB, inside it.
12. **Chose to move seqs 197→185 and keep 210→198.** Rationale: seq 198 created `docs/VERIFICATION.md`, which is referenced every session; seq 197's lesson (`core.fileMode`) was already preserved verbatim in the Live-gotchas line, so archiving its entry lost nothing live.
13. **Printed the verbatim text at both cut boundaries before touching anything**, because the marker strings had inconsistent spacing (`] ; prior =` vs `]; prior =`). This is what made the replace safe.
14. **Executed the move with asserted guards**, not just a single-occurrence assert: asserted the moved region *starts* at seq 197, *ends* with seq 185's closing bracket, contains seq 194/190/185, and does **NOT** contain `seq 198`, `BOARD (2026-07-29)`, or `STANDING / DEFERRED`. Those three negative assertions are what prevented a bad trim from eating live content.
15. **Rescued the still-live part of an archived entry.** Seq 193's `ENGINE_CAPTURE` / `ENGINE_INPUT` env entry points are used operationally almost every session, so before archiving that entry I promoted a 3-sentence version into the Live-gotchas line (~150 B instead of 2,111 B).
16. **One assertion failed — and it was my check string, not the file.** I asserted `"next free **EW-012**"` (lowercase n); the file says `"Next free **EW-012**"`. The write had already succeeded. Re-ran verification with corrected strings; 16/16 live markers present.
17. **Re-measured and corrected the recorded number.** The keep-compact note said 28,811 but the file was 29,568 after I added the gotcha + measurement note. Rewrote the note using same-width digit strings so the correction itself did not change the file size. Final: **29,568 bytes, verified equal to what the note claims.**
18. **Confirmed the file reads back in a single `Read` call** — the actual failure mode being prevented (it became unreadable on 2026-07-29).

### Chunk 3 — Phase 3A: survey, premise-checking, and the first implementation (mid)

19. **Spawned one `Explore` subagent (explicit `model: sonnet`, per standing policy) with 7 numbered fact-finding questions** while doing the memory trim myself — the trim is delicate and unversioned, the survey is parallelizable. Told it to grep `docs/MODULE_MAP.md`, never read it whole.
20. **Verified the subagent's crux claim myself rather than trusting it.** It reported that `play_tone_on_bus` shares the anonymous ring. `grep` on `next_sfx_channel` confirmed **four** callers: `play_sfx` (137), `play_sfx_on_bus` (154), `play_tone` (176), `play_tone_on_bus` (193), with `SFX_VOICES = 16`.
21. **Corrected one subagent overstatement.** It implied the docs did not mention the ring. In fact `play_tone`'s doc *does* (`audio_facade.rs:174-175`). The real gaps were narrower but real: `play_tone_on_bus` says nothing about the ring, and `enable_analysis`'s meterability list names only `play_sfx`.
22. **Checked the one assumption the whole design rested on: is `play_tone`'s `vol` argument metered?** Metering is documented as **pre-volume**, so if `vol` were applied at the sink it would be invisible to the meter and scaling it would be pointless. Read `playback.rs:178-217`: `sink.set_volume(effective_volume)` at 183 is separate, `.amplify(volume)` at 187 and `enveloped_tone_samples(freq, dur, volume)` at 209 bake `vol` into the samples, and `self.tapped(channel, source)` at 214 wraps the meter **after** that. The meter sees `vol`. Design held.
23. **Found the overlap trade-off in the same read.** `playback.rs:180` calls `self.stop_immediate(channel)` on every named-channel play — so a named-channel replay *cuts* the prior sound. That is the mechanism behind the "meterability vs overlap" finding, confirmed at the code level rather than inferred from docs.
24. **Discovered the predecessor plan's premise was wrong.** `survivor` calls `play_sfx` zero times (it uses `play_tone_on_bus`), has zero `HitFlash`, zero `Camera::shake` (only `Camera::new`), zero music, and zero scenes. The plan's "drive them from `levels().peak` instead of fixed constants" had no constants to replace — the effects had to be *added*.
25. **Confirmed the engine ticks the shake** before wiring anything: `Camera::update` advances `shake_timer`/`shake_duration` (`camera.rs:320-325`) and is called by the engine schedule at `src/app/schedule.rs:488`. No game-side system needed.
26. **Implemented v1:** named metered kill channel, `AudioFeel` + `AudioFeelSystem`, volume scaled by **kills-in-this-frame**, rising-edge arm/re-arm detection (mirroring `beat_crawler`), camera shake + player pulse, HUD row, watchdog, restart reset.
27. **Introduced `HUD_TOP`** and derived the new row's Y from it instead of adding another independent magic number — directly applying the recorded `beat_crawler` lesson (it shipped a capture with two HUD rows drawn on top of each other).

### Chunk 4 — Measurement-driven iteration: three findings that overturned the design (mid-late)

28. **Drove the game headlessly with scripted input**, no window, no OS automation: a scratchpad `.ron` pressing `G` (invulnerable), `B`×3 (+150 enemies), then holding `ArrowRight` to fire forever, run under `ENGINE_INPUT` + `ENGINE_CAPTURE`.
29. **Confirmed a real audio device was present in the headless run** — rodio printed `Dropping DeviceSink, audio playing through this sink will stop`, which only happens if a sink was actually playing. This is what makes headless capture a *behavioral* test for audio, not just a rendering one.
30. **First capture worked:** `kill meter 0.20 → shake/pulse (audio-driven)`, Kills 5, Enemies 160, watchdog untripped.
31. **I claimed a defect that did not exist, then disproved it.** Reading the thumbnail, enemy sprites appeared to cover the new HUD row. I cropped and magnified 3× — still looked covered. Rather than "fixing" it I sampled pixels, and found **67 light-blue TEXT pixels interleaved inside the pink band** (vs 456 pink). The text *is* on top, exactly as `DrawText::z`'s doc says (`queue.rs:43-52`); thin glyph strokes over saturated pink just read as covered at thumbnail scale. **No change made.** Cost: ~4 tool calls. Value: did not "fix" a non-bug in the renderer.
32. **Proved the watchdog non-vacuous the way #388 did** — temporarily replaced `enable_analysis(KILL_CHANNEL)` with `let _ = KILL_CHANNEL;`, re-captured, and confirmed the HUD flipped to the amber `kill meter -- → shake from kill count (nothing audible)`. Restored immediately.
33. **Measured the pulse instead of eyeballing it.** Player sprite core is exactly 32×32 px (1024 exact-colour pixels at rest); rendered colour is `(179,243,249)`. Across frames 170–195 the tolerance-box width moved **34 → 36 px**, spiking on kills and decaying between them. Real, but small.
34. **Measured the shake and found it invisible.** The player's world position is constant, so its screen position is a pure camera probe. Across 26 frames the centre was pinned at **409.5** — span 0.5 px. The shake was firing at ~1.5 px because `drive` was normalised against a `KILL_VOL_MAX` a real kill never reached.
35. **Stopped inferring and printed the actual numbers** — the seq-201 lesson ("I only found this by PRINTING THE ACTUAL BAND VALUES instead of theorizing from the screenshot"). Temporary `eprintln!` of `kills` and `peak`, 347 sampled frames.
36. **The measurement overturned the design.** Peak max was **0.2300 — exactly `KILL_VOL_BASE + KILL_VOL_PER_KILL×1`** — and **all 40 kill frames had `kills=1`**. `survivor` fires one bullet per `FIRE_COOLDOWN` and a bullet kills at most one enemy, so "kills in this frame" is a constant. The tone therefore had a single amplitude and metering it recovered **a constant**: a pure round-trip that told the game nothing it did not already hold. This became the session's most transferable finding.
37. **Re-keyed the tone to a decaying kill combo** (`COMBO_DECAY = 1.8`/s), added `DRIVE_FLOOR = 0.35` and a `drive_from_amplitude()` that maps the tone's **real** amplitude span (`KILL_VOL_BASE..KILL_VOL_MAX`) onto `FLOOR..1.0` rather than mapping `0..KILL_VOL_MAX`.
38. **Re-measured: the range became real.** Combo max 23.01, peak p50 **0.3330**, max **0.6000** (the ceiling), drive **0.35..1.00** → shake **2.4–7.0 px**, pulse **3.4–9.6 px**.
39. **Probed the shake directly** (`cam.shake_offset()` + `shake_remaining()`), since pixel measurement had become unreliable once 160 enemies buried the player (the cyan detector was latching onto 1–4 stray pixels). Result: the shake is genuine — amp 3.17 px, offset oscillating **−2.94..+3.15 x**, **−3.09..+3.17 y**, `rem` decaying 0.160 → 0.010, non-zero on 10/10 frames.
40. **But it had fired only ONCE in 300 frames.** Under a continuous kill stream the metered envelope never falls back below the re-arm threshold, so the arm/re-arm latch never re-arms — **the screen goes still exactly when the action is hottest.** This is the inverse of the trap `beat_crawler` documented, and it means that pattern does *not* transfer from a discrete beat clock to a sustained stream.
41. **Replaced arm/re-arm with a retrigger cooldown** (`since_shake >= SHAKE_SECS`), and re-measured: **25 fires in 300 frames** against a theoretical ceiling of ~29, amplitude 3.17–7.00 px, mean 4.89.
42. **A probe removal silently failed.** `cargo fmt` had reflowed the one-line `eprintln!` onto three lines, so the deletion regex matched nothing while the script reported success. Caught by `grep`-ing for `eprintln|TEMP|MEASURE|SHAKEPROBE` afterwards. **New variant of the known fmt-reflow trap**; the lesson is to verify probe removal by grep, never by the removal script's own exit.
43. **Fixed the engine docs the adoption exposed** — `enable_analysis`'s meterability list now names all four anonymous entry points and states the overlap trade-off; `play_tone_on_bus` now mentions the ring and points at `play_tone_on_channel`.
44. **Ran the gate, shipped, landed.** `/ship` paperwork (4 files in sync), gate re-run post-bump, branch → commit `c4e7a03` → PR #393 → auto-merge armed → 6/6 CI → merged `e7378e1` → main synced → branch deleted → memory bumped to seq 211.

## Key Decisions

- **Started a new chain tag `audio-adoption` at seq 1** rather than continuing `board-ew-triple` seq 5. The predecessor plan explicitly recommended it and the arc had closed. Recorded the linkage in Related Handoffs so nothing is lost.
- **Did NOT reopen the deferred `Audio` auto-persistence decision.** The plan said a missing `register_persistent::<Audio>()` in `survivor` would be a third instance and a reopen trigger. It is absent — but `survivor` never resets the `World`, so the risk cannot fire there. A trigger that cannot fire is not a trigger. Left the STANDING/DEFERRED block untouched.
- **Kept the bullet tone on the anonymous ring.** Making it meterable would cost overlap on a sound fired every 0.14 s. Meterability is a per-sound decision, not a global migration — and saying so in the docs is more valuable than converting everything.
- **Re-keyed the tone to a combo instead of declaring survivor a bad target.** When the measurement showed kills-per-frame was always 1, the honest options were "revert and report" or "give the tone real dynamic range". Chose the latter because a combo is genuinely better game feel *and* it makes the metering demonstrate something. Recorded the failed first keying in the code comments so nobody re-derives it.
- **Replaced arm/re-arm with a cooldown rather than tuning the thresholds.** Lowering `KILL_PEAK_OFF` would have been a fragile fix for a structural mismatch: the pattern assumes silence between events, and a kill stream has none.
- **Sized both effects from measurement, not from theory.** `PULSE_MAX` went 0.10 → 0.30 and `DRIVE_FLOOR` was introduced only after measuring that a lone kill produced ~1 px of camera movement.
- **Did not "fix" the HUD text z-order.** Investigated, found the engine correct, made no change, and recorded the disproof so the next session does not re-open it.
- **Landed the engine doc fix in the same PR** rather than splitting it. It is doc-comment-only with no API or behavior change, and it is only intelligible next to the adoption that found it. (Contrast #388, where a real one-line engine *fix* was deliberately split out.)
- **Shipped as MINOR 0.139.0, not PATCH.** No API change, but a shipped example gained a user-visible capability; the pre-1.0 rule reserves PATCH for bugfixes.
- **Used async auto-merge** (`gh pr merge --auto --squash`) per standing delegated merge authority — the change is example+docs, which green CI fully covers, and the audio behavior was already verified locally on a real device.

## Evidence & Data

### Board gate — both channels (Phase 1)

| Channel | Open requests | Next free ID | Last modified | Moved? |
|---|---|---|---|---|
| `../dungeon-merchant/docs/engine-wishlist.md` | `*(none — all filed requests are closed)*` | **EW-012** | 2026-07-27 (`539b6100`) | **No** (gate condition: later than 2026-07-27) |
| `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` | `_None._` | n/a | 2026-07-14 (`7794358`) | No (paused/deprecated) |

### Verify-gate history — every run, exit code read from the FILE

| # | When | Exit file | Exit | ok groups | lib tests | Notes |
|---|---|---|---|---|---|---|
| 1 | Session start | `/tmp/v.exit` (mtime 13:13) | **0** | 152 | 1339 | Notification claimed completion for the wrapper — Trap 4, 4th session running |
| 2 | Post-implementation | `/tmp/v2.exit` (mtime 22:32) | **0** | 152 | 1339 | All 8 `[verify]` markers present |
| 3 | Post-version-bump | `/tmp/v3.exit` (mtime 22:38) | **0** | 152 | 1339 | Required by the `/ship` skill |

Counts unchanged across all three — correct for an example + docs change, and the corroboration step `docs/VERIFICATION.md` added in #390.

### Memory tip-chain, per-seq bytes (the measurement that chose the cut)

| seq | bytes | seq | bytes | seq | bytes |
|---|---|---|---|---|---|
| 210 | 1,297 | 202 | 1,436 | 193 | 2,111 |
| 209 | 925 | 201 | *(see caveat)* | 192 | 1,079 |
| 208 | 1,012 | 200 | 5,609\* | 191 | 1,635 |
| 207 | 2,605 | 199 | 816 | 190 | 214 |
| 206 | 1,226 | 198 | 1,029 | 189 | 1,758 |
| 205 | 1,315 | 197 | 1,051 | 188 | 222 |
| 204 | 1,152 | 196 | 1,336 | 187 | 1,558 |
| 203 | 1,543 | 195 | 1,766 | 186 | 228 |
| | | 194 | 642 | 185 | 2,963 |

**Caveat, recorded honestly:** the segmenter split on literal `seq NNN` matches, and seq 201's entry contains the prose `"(PR1 = seq 200 levels)"`. That false match ended seq 201's span early and folded its remainder into the 5,609 B attributed to seq 200. **The trim itself was unaffected** — it used explicit boundary strings that were printed and eyeballed first, not these byte offsets. Tail (board status + STANDING blocks, kept): 3,668 B.

### Memory trim — before → after

| File | Before | After | Δ |
|---|---|---|---|
| `engine-current-state.md` | 45,519 B | **29,568 B** | −15,951 (−35%) |
| — of which line 9 (tip chain) | 40,428 B | 23,720 B | −16,708 |
| `engine-history-archive.md` | 319,336 B | 336,783 B | +17,447 (verbatim) |
| `MEMORY.md` | 5,385 B | ~5,564 B | hook updated |

Moved region: **16,947 B, seqs 197→185**. Kept chain: seqs 210→198. Target was 25–30 KB; landed at 29.6 KB. Grows ~1.5 KB/seq ⇒ **trim again by ~seq 220**.

### The measurement that overturned the design — v1 (kills-per-frame keying)

| Metric | Value |
|---|---|
| Sampled frames with signal | 347 |
| Kill frames | 40 |
| **Kills per kill-frame** | **1 in 40 of 40 — never more** |
| peak max | **0.2300** (= `0.16 + 0.07×1` exactly) |
| peak p50 / mean | 0.1615 / 0.1605 |
| peak-after-kill (n=40) | min 0.162, max 0.230, mean 0.217 |

Conclusion: the tone had exactly one amplitude, so the meter recovered a constant.

### The same measurement — v2 (combo keying)

| Metric | v1 | v2 |
|---|---|---|
| combo max | n/a (always 1) | **23.01** (p90 21.70) |
| peak max | 0.2300 | **0.6000** (ceiling reached) |
| peak p50 | 0.1615 | **0.3330** |
| peak p90 | 0.2300 | 0.6000 |
| mapped drive | ~0.20 fixed | **0.35 → 1.00** |
| camera shake | ~1.5 px | **2.4 – 7.0 px** |
| player pulse | ~1 px | **3.4 – 9.6 px** |

### Retrigger: arm/re-arm vs cooldown (same 300-frame run)

| Strategy | Fires in 300 frames | Amplitude | Verdict |
|---|---|---|---|
| Arm/re-arm on envelope (`beat_crawler`'s pattern) | **1** | 3.17 px | Fails — envelope never re-arms under a stream |
| Retrigger cooldown (`since_shake >= SHAKE_SECS`) | **25** (ceiling ≈29) | 3.17–7.00 px, mean 4.89 | Correct for a sustained level |

### Direct camera-shake probe (10 consecutive frames after one fire)

```
fire=true  drive=0.453 amp=3.17 off=( 0.00, 3.17) rem=0.160
fire=false               off=( 2.38, 1.30) rem=0.143
fire=false               off=( 3.15,-2.11) rem=0.127
fire=false               off=( 1.77,-3.02) rem=0.110
fire=false               off=(-0.81,-0.36) rem=0.093
fire=false               off=(-2.84, 2.73) rem=0.077
fire=false               off=(-2.94, 2.59) rem=0.060
fire=false               off=(-1.04,-0.62) rem=0.043
fire=false               off=( 1.57,-3.09) rem=0.027
fire=false               off=( 3.11,-1.91) rem=0.010
```
Non-zero on 10/10 frames, decaying to zero. The shake is real; only its *amplitude* was wrong in v1.

### Player-sprite pulse, measured per frame (v1, PULSE_MAX=0.30, base core 32 px)

| frames | box width |
|---|---|
| 171–174, 183–184, 186–187, 195 | 36 px (pulsed) |
| 176–182, 188–194 | 34 px (at rest) |
| 175 | 35 px (decaying) |

Rest box measures 34 px because the exact-colour core is 32 px plus ~1 px of antialiased edge per side (verified: 1024 exact-colour pixels = 32×32 at rest).

### The disproved claim — pixel classification inside the "covered" band

```
y=34: ...............PPPPPPPPPPPPPPPPPPPPPPPPPPPP.................
y=38: ...T....T..T....TTTPPPP.TTTPPPPPPP.TPPP.TTT.........T....TT.
y=42: T..T....T..T...P..TTP.TPPPPPPPPPPPT.PP.TPPP.........T..T....
y=46: ...............PPPPPPPPPPPPPPPPPPPPTPPPPPPP.................

x=195..225, y=30..50 → TEXT px = 67, PINK px = 456
```
`T` = HUD text blue, `P` = enemy sprite pink. Text pixels appear *inside* the sprite band ⇒ text renders on top. Claim withdrawn, no code changed.

### Temporary probes added and removed (all verified gone)

| # | Probe | Purpose | Removal |
|---|---|---|---|
| 1 | `enable_analysis` → `let _ = KILL_CHANNEL;` | Prove the watchdog fires | Restored by Edit |
| 2 | `eprintln!("MEASURE kills=… peak=…")` | Peak distribution, v1 | Regex, clean |
| 3 | `eprintln!("SHAKEPROBE …off=…rem=…")` | Prove the camera actually shakes | Explicit replace |
| 4 | `eprintln!("FIRE drive=… amp=…")` | Count fires after the cooldown fix | **Regex FAILED** (fmt reflow) → removed by Edit after `grep` caught it |

Final check: `grep -c "eprintln\|TEMP\|MEASURE\|SHAKEPROBE" examples/games/survivor/survivor.rs` → **0**.

### CI — PR #393, all six required checks

| Check | Result | Duration |
|---|---|---|
| Test (native) | pass | 6m59s |
| Build (Windows / DX12) | pass | 1m39s |
| Package dry-run | pass | 1m17s |
| Render tests (lavapipe) | pass | 1m14s |
| Build (WASM) | pass | 47s |
| Rustdoc | pass | 40s |

Auto-merge armed 13:39:24Z, merged 13:46:21Z as `e7378e1`. `lavapipe` at 1m14s — no recurrence of the 10m34s outlier seen once on #382 (six PRs ago), so that question is closeable.

### Shipped

| PR | Commit | Version | Diffstat |
|---|---|---|---|
| #393 | `e7378e1` (squash of `c4e7a03`) | 0.139.0 (MINOR) | 7 files, +274 net in `survivor.rs` |

### The four findings, stated once (the actual deliverable)

These are scattered through the code comments, the CHANGELOG and the module map. Collected here so the next session does not have to reassemble them:

1. **Meterability and overlap are mutually exclusive.** Only a stable channel name can be metered; a named-channel replay calls `stop_immediate` and *cuts* the sound already there, where the anonymous ring lets consecutive one-shots overlap. This is a **per-sound** decision, not a global migration.
2. **Arm/re-arm does not survive a continuous stream.** A rising-edge latch with a lower re-arm threshold — correct for `beat_crawler`'s kicks, which are separated by silence — fired **1 time in 300 frames** under a kill stream, because the smoothed envelope never falls back below the re-arm level. A retrigger **cooldown** fired **25 times** over the same run.
3. **Metering only pays when the sound carries information the game does not already hold.** Keyed to kills-per-frame the tone had exactly one amplitude (measured: 1 kill in 40 of 40 kill frames; peak pinned at 0.230), so reading it back recovered a constant — a round-trip. The win comes from audio the game does **not** author.
4. **A meter-driven effect needs a watchdog.** Independently rediscovered here after `BEAT_WATCHDOG`, which is what makes it a general rule rather than a quirk of a turn clock.

### The three consumers of the metering API, compared

| | `examples/audio_reactive` | `examples/games/beat_crawler` | `examples/games/survivor` (this session) |
|---|---|---|---|
| Shipped | v0.136.0 / v0.137.0 | v0.138.0 | **v0.139.0** |
| Written by | the session that **built** the API | the session that had just **read** it | a session touching **older, unrelated** code |
| What it meters | its own demo tones | a soundtrack **no gameplay code sees** | a tone the game itself authors |
| Uses | `levels` + `bands` | `bands` (low-band sum = turn clock) | `levels().peak` only |
| Independent evidence? | No | Weak | **Yes** |
| Detection pattern | continuous display | arm/re-arm on discrete kicks | **cooldown retrigger** on a stream |

The point of the comparison: the first two could not fail in an interesting way, because the metering was designed around them. The third could — and did, three times.

### Design iteration history — v1 → v2 → v3, and what forced each change

| Version | Intensity keyed to | Retrigger | Measured outcome | Why it changed |
|---|---|---|---|---|
| **v1** | kills **this frame** | arm/re-arm on envelope | peak pinned at **0.2300**; all 40 kill frames had `kills=1`; shake ~1.5 px; camera centre span **0.5 px** (invisible) | The knob had no dynamic range — metering recovered a constant |
| **v2** | decaying **combo** (`COMBO_DECAY=1.8`/s) + `DRIVE_FLOOR` + `drive_from_amplitude()` | arm/re-arm (unchanged) | peak p50 **0.3330**, max **0.6000**; drive 0.35–1.00; but shake fired **1×/300 frames** | Range fixed, but the stream never let the latch re-arm |
| **v3 (shipped)** | combo (unchanged) | **cooldown** `since_shake >= SHAKE_SECS` | **25 fires/300 frames**, amp 3.17–7.00 px (mean 4.89), pulse 3.4–9.6 px | Correct for a sustained level |

Each step was forced by a measurement, not by taste. v1→v2 came from printing `peak`; v2→v3 came from probing `cam.shake_offset()`.

### `survivor`'s audio call sites — before → after

| Site | Before | After | Metered? | Why |
|---|---|---|---|---|
| Bullet fired (`PlayerSystem`) | `play_tone(900, 0.04, 0.16)` via `play_tone_on_bus` | **unchanged** | No | Fires every 0.14 s — a named channel would cut each shot |
| Enemy killed (`CollisionSystem`) | `play_tone(150, 0.14, 0.28)` — fixed volume | `play_tone_named(KILL_CHANNEL, 150, 0.14, vol(combo))` | **Yes** | At most once per frame; re-triggering with a fresh amplitude is the intent |
| Player hit / game over (`CollisionSystem`) | `play_tone(110, 0.3, 0.35)` | **unchanged** | No | Fires exactly once per run, at game over — nothing to drive |

Note the death tone's timing was itself a finding during design: `survivor` is single-life, so "player hurt" and "run over" are the same event. Driving a `HitFlash` from it, as the predecessor plan suggested, would flash a player who is already dead.

### The direction question exactly as presented (for calibration)

The predecessor recorded that *how* the menu is framed drives the answer. Reproduced so the pattern is repeatable — four options, each with the trade-off spelled out, recommendation first and labelled:

| Option | Label | Core of the reasoning given |
|---|---|---|
| 1 (taken) | `survivor 오디오 채택 (추천)` | First consumer not written by the API's designer; `beat_crawler` is a weak second because the same session designed it; expect to hit the unmeterable round-robin immediately — **that verdict is the deliverable** |
| 2 | `core_resources 씬리셋 감사` | `WindowConfig` was found by accident after being worked around twice; a written "none found" is a valid result that stops the next session re-running it |
| 3 | `4번째 맵 생성기 (drunkard's walk)` | ~80 lines, connected by construction — but rooms/caves/mazes + `roguelike` + `beat_crawler` already cover it; repeating a proven pattern a fourth time teaches nothing |
| 4 | `beat_crawler 심화` | One addition only; a real soundtrack is the most interesting for `bands()` but needs a licence-clean file |

Also stated in the question, per the plan: Phase 2 is **not** a choice, and windowed capture is **off the menu** (declined in writing 2026-07-27).

### Subagent survey — what was asked, what came back, what I had to correct

One `Explore` agent, `model: sonnet`, 7 numbered questions, run in parallel with the memory trim.

| Claim returned | Verdict after my own check |
|---|---|
| `levels`/`bands`/`enable_analysis` signatures + `AudioLevels{rms,peak}` | **Correct** |
| `play_tone_on_bus` shares the anonymous ring with `play_sfx` | **Correct and load-bearing** — `grep next_sfx_channel` shows 4 callers |
| Docs do not warn about the ring | **Overstated.** `play_tone`'s doc *does* (`:174-175`); the real gaps were `play_tone_on_bus` and `enable_analysis`'s list |
| `survivor` has no scenes, no `register_persistent::<Audio>()` | **Correct** — and it makes the missing call inert |
| `survivor` has no `HitFlash`, no `Camera::shake` | **Correct** — which invalidated the plan's "instead of fixed constants" framing |
| Example name is `survivor_game`, no self-test mode | **Correct** |
| `beat_crawler`/`audio_reactive` call pattern | **Correct** |

Cost ~80K subagent tokens / 37 tool calls / 153 s, and it ran while the trim proceeded. The one overstatement is the reason the standing rule is to verify a subagent's crux claim before building on it.

### The shipped feedback loop (primary evidence — the core of the change)

```rust
// CollisionSystem, once per frame in which anything died:
let combo = { feel.pending_kills += score_gain; feel.combo += score_gain as f32; feel.combo };
let vol = (KILL_VOL_BASE + KILL_VOL_PER_KILL * combo).min(KILL_VOL_MAX);
play_tone_named(world, KILL_CHANNEL, 150.0, 0.14, vol);

// AudioFeelSystem, every frame, after AudioFacadeSystem has sampled the meters:
let peak = world.resource::<Audio>().map(|a| a.levels(KILL_CHANNEL).peak).unwrap_or(0.0);
feel.combo = (feel.combo - COMBO_DECAY * dt).max(0.0);
feel.since_shake += dt;
let ready = feel.since_shake >= SHAKE_SECS;
let (fire, drive) = if feel.metered {
    if ready && peak >= KILL_PEAK_ON { (true, drive_from_amplitude(peak)) } else { (false, 0.0) }
} else if kills > 0 && ready {                      // silent fallback: predict the amplitude
    (true, drive_from_amplitude((KILL_VOL_BASE + KILL_VOL_PER_KILL * feel.combo).min(KILL_VOL_MAX)))
} else { (false, 0.0) };
if fire { feel.since_shake = 0.0; cam.shake(SHAKE_MAX_PX * drive, SHAKE_SECS); }
let pulse = if feel.metered && peak > KILL_PEAK_OFF { drive_from_amplitude(peak) } else { 0.0 };
t.scale = Vec2::splat(PLAYER_HALF * 2.0 * (1.0 + PULSE_MAX * pulse));   // rebuilt from base, never accumulated
```

Two subtleties worth preserving: the pulse is **gated on `peak > KILL_PEAK_OFF`** because `drive_from_amplitude` floors at `DRIVE_FLOOR` and an ungated silent frame would leave the player permanently inflated; and the scale is **rebuilt from the constant base** each frame so repeated frames cannot drift.

### The two HUD strings (what a capture should show)

```
combo  4.0   kill meter 0.33  → shake/pulse (audio-driven)        [150,220,255] blue
combo  4.0   kill meter  --   → shake from combo (nothing audible) [230,190,130] amber
```
The amber line is the watchdog verdict. A capture on a machine with no device, or with `enable_analysis` disabled, must show amber; a capture with a device must show blue and a non-zero meter.

### Trap 4 occurrence history (the background-notification false green)

| Session | What the notification said | What the file held |
|---|---|---|
| 2026-07-29 (a) | exit 0 | **1** (`cargo fmt`) |
| 2026-07-29 (b) | exit 0 | **101** (rustdoc) |
| 2026-07-30 (predecessor) | exit 0 | **1** (`cargo fmt` reflow) |
| **2026-07-30 (this session)** | "completed (exit code 0)" within one round | gate still running; file did not exist yet |

Four consecutive sessions. The mitigation (`docs/VERIFICATION.md` copy-paste block, added #390) held both times it was used here. Note this session's variant is the *cheapest* one to fall for: the notification arrives so fast that it looks like a warm-cache success.

### The memory-trim guard assertions (reusable — this is the procedure, not just the result)

```python
assert txt.count(A) == 1 and txt.count(B) == 1     # both boundaries unique
assert a < b                                        # ordering sane
assert moved.rstrip().endswith("arc is served).**]")# ends at a real entry boundary
assert "seq 185 TEST #366" in moved and "seq 194 DOCS #375" in moved
assert "seq 198 DOCS #379" not in moved             # NEGATIVE: kept entry not swallowed
assert "BOARD (2026-07-29)" not in moved            # NEGATIVE: board status preserved
assert "STANDING / DEFERRED" not in moved           # NEGATIVE: Audio decision preserved
```
The three **negative** assertions are what make this safe. A single-occurrence positive assert only proves you found *a* boundary; it does not prove you did not swallow live content past it. Afterwards, 16 live markers were re-checked by substring and the file was confirmed to read back in one `Read`.

## Reusable Procedures

### Verifying an audio-driven effect headlessly (no window, no OS automation)

1. Write an `InputScript` `.ron` that forces the state you need (`KeyPress`/`KeyDown`; keys are winit `KeyCode` variant names — `"KeyG"`, `"ArrowRight"`).
2. Run `ENGINE_INPUT=<script>.ron ENGINE_CAPTURE=<frame>:<png>[,…] cargo run --example <name>`. Capture **diverts** the run — it never also opens a window.
3. **Confirm a device actually existed**: rodio prints `Dropping DeviceSink, audio playing through this sink will stop` on teardown only if a sink was playing. Without this line, a green capture proves nothing about audio.
4. Read the HUD from the PNG for the qualitative verdict; for anything quantitative, **add a temporary `eprintln!` and read the numbers** — do not infer values from pixel sizes (see below).
5. Prove the failure path too, by disabling the input (here: replacing `enable_analysis` with `let _ = …`) and re-capturing. A watchdog nobody has seen fire is not a watchdog.

### Removing a temporary probe safely

`cargo fmt` may reflow a one-line probe onto several lines between insertion and removal, so a deletion regex written against the original text matches nothing **and reports success**. Always finish with an independent `grep -c "eprintln\|TEMP\|<probe tag>" <file>` and require `0`. This session's probe #4 survived its own removal script exactly this way.

### When pixel measurement stops being trustworthy

Measuring a sprite's bounding box is reliable only while the sprite is unoccluded. At 160 enemies the cyan player detector was latching onto 1–4 stray pixels and reported an 18 px "shake" that did not exist. Prefer probing engine state directly (`cam.shake_offset()`, `shake_remaining()`) once the scene is crowded, and treat a sudden drop in detected sprite size as the tell that the measurement — not the game — has broken.

## Code Analysis

- **The anonymous voice ring.** `const SFX_VOICES: u64 = 16`; `sfx_voice_channel(seq, voices) -> format!("__facade_sfx_{}", seq % voices)`; `next_sfx_channel(&mut self)` bumps `self.sfx_seq`. Called by **four** public methods: `play_sfx` (`audio_facade.rs:137`), `play_sfx_on_bus` (154), `play_tone` (176), `play_tone_on_bus` (193). None is meterable.
- **The meterable alternatives.** `play_tone_on_channel(&mut self, channel: &str, freq: f32, dur: f32, vol: f32, bus: &str)` (212) and `play_at_on_channel` (289), plus `Audio::MUSIC_CHANNEL` (446). Note `play_tone_on_channel` takes **both** a channel and a bus, so migrating off `play_tone_on_bus` preserves bus routing and costs exactly one argument.
- **Why a named replay cuts.** `AudioManager::play_tone` (`playback.rs:178`) opens with `self.stop_immediate(channel)`. That is the whole overlap trade-off, in one line.
- **Why the meter sees `vol`.** Volume reaches the sink separately (`sink.set_volume(self.effective_volume(channel))`, :183) while `vol` is baked into the source — `.amplify(volume)` (:187) on the effects path, `enveloped_tone_samples(freq, duration_secs, volume)` (:209) on the no-effect path — and `self.tapped(channel, source)` (:214) wraps the meter *after* both. Pre-volume metering excludes bus/duck/master but **not** the per-sound amplitude.
- **`AudioLevels { rms, peak }`** (`audio_analysis.rs:50-58`), both 0..=1, smoothed engine-side (instant attack + timed release, `DEFAULT_ANALYSIS_SMOOTHING` = 0.15 s). Callers do **not** smooth.
- **`Camera::shake(strength_px, duration_secs)`** (`camera.rs:236`) sets `shake_timer = 0.0`. **Calling it every frame pins `shake_timer` at 0**, and `shake_offset()` (:260) computes `ox = sin(t*1.7)*s, oy = cos(t*2.3)*s` with `t = shake_timer*30` — so a per-frame call yields a *constant* offset `(0, s)`, not a shake. This is why the retrigger must be event- or cooldown-gated, never continuous. Ticked by `cam.update(dt, follow)` at `src/app/schedule.rs:488`.
- **`DrawText::z: Option<f32>`** (`renderer/text/queue.rs:52`) — `None` = the final on-top text pass, drawn over UI rects/images and after post-processing, "right for HUD readouts". Verified empirically this session: it also draws over world sprites.
- **survivor's shape:** single file, 983 lines before / ~1,150 after. Sprite z-values in use: 0.5 (thruster), 0.8 (bullets), 1.0 (player/enemies), 2.0 (explosions). Player base scale `Vec2::splat(PLAYER_HALF * 2.0)` = 32 px; rendered colour `(179,243,249)`.
- **New constants shipped:** `KILL_CHANNEL="kill"`, `KILL_VOL_BASE=0.16`, `KILL_VOL_PER_KILL=0.07`, `KILL_VOL_MAX=0.60`, `COMBO_DECAY=1.8`, `DRIVE_FLOOR=0.35`, `KILL_PEAK_ON=0.10`, `KILL_PEAK_OFF=0.05`, `SHAKE_MAX_PX=7.0`, `SHAKE_SECS=0.16`, `PULSE_MAX=0.30`, `FEEL_WATCHDOG=0.6`, `HUD_TOP=8.0`.
- **`drive_from_amplitude(amp)`** maps the tone's *real* span onto the drive: `t = clamp((amp - KILL_VOL_BASE)/(KILL_VOL_MAX - KILL_VOL_BASE), 0, 1); DRIVE_FLOOR + (1-DRIVE_FLOOR)*t`. Because it floors at 0.35, the **pulse must be gated** on `peak > KILL_PEAK_OFF` or a silent frame leaves the player permanently inflated.

## Files Changed

### Source code
- `src/audio_facade.rs` — **doc comments only, no API change.** `enable_analysis`'s "Which channels can be metered" now names all four anonymous entry points and states the meterability-vs-overlap trade-off; `play_tone_on_bus` now documents that it rides the shared ring and cannot be metered.

### Examples (the acceptance test)
- `examples/games/survivor/survivor.rs` — the whole behavioral change. Module docs (four findings), the audio-reactive tuning constant block, `AudioFeel` resource, `AudioFeelSystem`, `drive_from_amplitude`, `play_tone_named`, kill tone → named metered channel keyed to a combo, camera shake + player pulse, second HUD row, `HUD_TOP`, watchdog + silent fallback, restart reset preserving `metered`.

### Docs
- `docs/CHANGELOG.md` — new 0.139.0 section: summary, `### Changed`, `### Fixed`, and a four-bullet "Notes for anyone building on it".
- `docs/MODULE_MAP.md` — audio-reactive row **extended** with the corrected meterability list and all four adoption findings.
- `CLAUDE.md` — header line 3 → `v1.6.240` / package `v0.139.0`.

### Release paperwork
- `Cargo.toml` — `version = "0.139.0"`.
- `Cargo.lock` — refreshed via `cargo update -p skeleton-engine` (Locking 0 packages).

### Memory (not in any PR — not version-controlled)
- `engine-current-state.md` — trimmed 45,519 → 29,568 B; seq 211 prepended; menu line marks the survivor adoption done; ENGINE_CAPTURE/ENGINE_INPUT gotcha added; measurement note corrected.
- `engine-history-archive.md` — new dated section, seqs 197→185 verbatim.
- `MEMORY.md` — both the current-state hook and the archive hook updated.

### Scratchpad (throwaway, not committed)
- `survivor_feel.ron` — the input script (G / B×3 / hold ArrowRight). **Deliberately not committed** — kept the change to one file.
- `BACKUP_engine-current-state.md`, `BACKUP_engine-history-archive.md`, `BACKUP_MEMORY.md` — pre-trim rollback copies.
- `feel_*.png`, `p_*.png`, `f_*.png`, `nowd_180.png`, `crop_*.png`, `measure*.txt`, `shake.txt`, `fire.txt` — capture and measurement artifacts.

## User Feedback & Preferences (REQUIRED)

- **Direction chosen from the menu: "survivor 오디오 채택 (추천)"** — the recommended option. **Six of six** recommendations taken across three sessions. The framing (what breaks otherwise, what it actually costs) continues to be load-bearing; keep presenting reasoning, not labels.
- **The session opened with an explicit, highly-specified prompt** covering the board gate, the exact verify copy-paste block, the ask, and the trim — including "PHASE 1 ENDS IN A QUESTION, NOT CODE" and "Do NOT self-pick". Every instruction was followed as written.
- **"Read the exit code from the FILE — the notification has lied in three straight sessions, including one where it said 0 on a run that was actually 1."** It lied again immediately. Treat the notification as noise, permanently.
- **"COPY BOTH MEMORY FILES TO THE SCRATCHPAD FIRST — they are not version-controlled."** Done before any edit.
- **"Do NOT reopen the deferred Audio-persistence decision unless one of the triggers in `docs/PATTERNS.md` has actually fired."** Checked; it has not; left closed.
- **"Windowed capture is OFF the menu: the game declined it in writing 2026-07-27."** Kept off the menu and said so in the question.
- **"Grep `docs/MODULE_MAP.md` for topics — never read it whole (72 rows, ~89 KB)."** Followed, including in the subagent's prompt.
- **"Do NOT onboard or re-explore the codebase — the handoff has it."** No onboarding was done; went straight to the gate.
- **Standing (from CLAUDE.md / memory):** user-facing reports in **Korean**, everything else (code, docs, subagent prompts) in **English**. Subagents always get an **explicit `model`**. Merge authority is **standing-delegated** — squash-merge on green CI without re-confirming.

## Where We're Going

1. **Board gate FIRST, every session** — `../dungeon-merchant/docs/engine-wishlist.md` (next free **EW-012**, empty since 2026-07-27) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`). A filed request preempts everything. Check whether the board **moved** (`git log -1 --date=short -- docs/engine-wishlist.md`), not just whether it looks empty.
2. **If both are empty: ASK — do not self-pick.** The menu, with completed items removed:
   - **A meterable one-shot API** (the gap this session *named* but did not close): today meterability and overlap are mutually exclusive. A `play_sfx_metered`-style API — a named channel that allocates a fresh voice per trigger instead of cutting — would remove the trade-off. This is the first genuinely API-shaped request the adoption produced, and it now has evidence behind it.
   - **A third capstone, or a real soundtrack for `beat_crawler`** — a mixed track rather than isolated tones is the actual use case for `bands()`; needs a licence-clean file (the CC0 synthesis recipe in `src/audio/fixtures/README.md` solved exactly this for the codec tests).
   - **Audit `src/app/core_resources.rs`** for more `WindowConfig`-class scene-reset-fragile config — still unbuilt, still cheap, and a written "none found" is still a valid result. This was branch 3B of the predecessor plan.
   - **A fourth procgen mode** (drunkard's walk) — still the safest and lowest marginal value.
3. **This handoff + its plan land as their own `docs(handoff)` PR**, chain `audio-adoption` seq 1. Bump memory to **seq 212** after it merges — and note the recorded tip will be one commit stale, as it has been for every handoff in every chain.
4. **Trim `engine-current-state` again by ~seq 220** (29.6 KB now, ~1.5 KB/seq, ~76 KB cap). The procedure and its guard assertions are in this session's Chunk 2.

## Risks & Blockers

- **The background-notification exit code is still unreliable — four consecutive sessions.** The `docs/VERIFICATION.md` copy-paste block works; the failure mode is forgetting to use it. Always `rm -f` first, run non-piped, wait on the file, read it, check mtime, corroborate 152/1339.
- **`cargo fmt` can silently defeat a scripted edit.** New this session: fmt reflowed a one-line probe, so the deletion regex matched nothing and reported success. Any script that removes code it inserted must be verified by an independent `grep`, not by its own exit code.
- **Pixel-based verification degrades under load.** The cyan player detector was reliable at 5 enemies and useless at 160 (occlusion reduced the sprite to 1–4 visible pixels, producing 18 px of phantom "shake"). Probe the state directly when the scene is crowded.
- **Audio behaviour is not covered by CI.** Green CI proves it builds and the tests pass; it cannot prove the meter reads anything. Headless `ENGINE_CAPTURE` on a machine with a real device is the only automated check, and it is manual. Anything touching metering needs that run.
- **`survivor` has no self-test mode.** Unlike `beat_crawler` (`BEAT_CRAWLER_SELFTEST=1`), the audio-reactive path has no assertion that runs unattended — it was verified by capture + probes, both of which were removed. A regression would be silent.
- **The input script is not committed.** Reproducing this session's verification requires re-writing the `.ron` (contents are in Chunk 4 / Files Changed).
- **`dungeon-merchant` has no CI or branch protection** — its board is discipline, not enforcement. `rust-survivors` is paused/deprecated but remains a real bug-report channel.

## Open Questions

- **Should the engine offer a meterable one-shot** — a named channel that allocates a fresh voice per trigger rather than cutting? This session turned a vague "is `play_sfx` a gap?" into a specific, evidence-backed design question. Nothing has *asked* for it yet, so it stays a menu item, not a decision.
- **Should `survivor` get a `SURVIVOR_SELFTEST=1` mode** asserting that the meter moves and the watchdog engages? It would make the adoption regression-proof, but needs a device and would be skipped on CI (the `beat_crawler` precedent: no device = SKIP, not fail).
- **Should `Audio` be auto-persisted?** Still deliberately deferred. `survivor` did **not** add a third instance. Reopen only if a third game actually hits it, a bug traces to it, or another *engine-inserted* config type turns out to be dropped the way `WindowConfig` was.
- **Are there other engine-inserted config resources silently reverting on scene reset?** `insert_core_resources` has still never been audited. Carried unanswered from the predecessor.
- **Should `embedded_image` get a web harness?** Carried unanswered from three prior sessions. Probably time to either do it or close it.
- **Is the `add-facade-capability` skill worth writing?** Still deferred at n=2.

## Quick Start for Next Session

```bash
# Restore full context
cat plans/handoffs/HANDOFF_audio-adoption_survivor-audio-reactive-adoption_2026-07-30.md

# Nothing is dangling — PR #393 merged, main clean at e7378e1 (v0.139.0).
cd ~/Projects/skeleton-engine
git log --oneline -3      # expect e7378e1 at the tip (or the handoff merge above it)
git status -s             # expect clean

# 1. READ THE BOARD FIRST — both channels; a filed request preempts everything
#   ../dungeon-merchant/docs/engine-wishlist.md        (next free EW-012, empty since 2026-07-27)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md   (_None._)
cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md && cd -
# ^ later than 2026-07-27 = the board MOVED; read it before anything else

# 2. Verify starting state — the block in docs/VERIFICATION.md.
#    Read the exit code from the FILE. The notification has now lied in FOUR straight sessions.
rm -f /tmp/v.exit /tmp/v.log
(./scripts/verify.sh > /tmp/v.log 2>&1; echo $? > /tmp/v.exit) &
until [ -f /tmp/v.exit ]; do sleep 20; done
echo "EXIT=$(cat /tmp/v.exit)"; ls -l /tmp/v.exit
echo "ok groups: $(grep -c 'test result: ok' /tmp/v.log)"
# expect 0 · 152 ok groups · 1339 lib tests

# 3. Key files if the next work is audio-adoption follow-on
#   examples/games/survivor/survivor.rs   — this session's adoption + its 4 findings in the module docs
#   src/audio_facade.rs                   — the ring (l.88-205), play_tone_on_channel (212), enable_analysis (~465)
#   src/audio/playback.rs                 — stop_immediate (180), amplify (187), tapped (214)
#   docs/MODULE_MAP.md                    — GREP IT; audio-reactive row carries the findings
#   src/app/core_resources.rs             — the unbuilt 3B audit subject

# 4. Reproduce this session's headless verification (needs a real audio device)
#    Write the input script first — it was NOT committed:
#      (events: [(frame:5,action:KeyPress("KeyG")), (frame:10,action:KeyPress("KeyB")),
#                (frame:12,action:KeyPress("KeyB")), (frame:14,action:KeyPress("KeyB")),
#                (frame:20,action:KeyDown("ArrowRight"))])
ENGINE_INPUT=<script.ron> ENGINE_CAPTURE=180:/tmp/s.png cargo run --example survivor_game
#    HUD row 2 should read: "combo N.N   kill meter 0.NN  → shake/pulse (audio-driven)"

# 5. FIRST CONCRETE ACTION
#    Read both board files and state the verdict. If empty, ask ONE AskUserQuestion (Korean)
#    with reasoning: a meterable one-shot API (the gap this session named, now evidence-backed) /
#    a real soundtrack for beat_crawler / audit insert_core_resources for WindowConfig-class bugs /
#    a 4th procgen mode. Do NOT self-pick.
```
