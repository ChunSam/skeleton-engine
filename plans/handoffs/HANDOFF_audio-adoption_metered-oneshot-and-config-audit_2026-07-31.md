# The measured API gap became an API, an unbuilt audit found seven bugs, and both landed — three PRs in one session

**Date:** 2026-07-31
**Status:** COMPLETED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `audio-adoption` seq `2`
**Parent:** `HANDOFF_audio-adoption_survivor-audio-reactive-adoption_2026-07-30.md`
**Prior chain:** `HANDOFF_audio-adoption_survivor-audio-reactive-adoption_2026-07-30.md` (seq 1) > this

---

## Stale References

Identifiers named in the parent (seq 1) that no longer exist, both renamed by this session's PR #397:

- `KILL_CHANNEL` — renamed to **`KILL_METER`** in `examples/games/survivor/survivor.rs`. It no longer names a channel, so the old name was actively misleading.
- `play_tone_named` — renamed to **`play_metered_tone`**, and its body changed from `Audio::play_tone_on_channel` to `Audio::play_tone_metered`.

Everything else the parent named still exists and is unchanged: `AudioFeel`, `AudioFeelSystem`, `drive_from_amplitude`, `HUD_TOP`, `KILL_VOL_BASE`, `KILL_VOL_MAX`, `KILL_VOL_PER_KILL`, `COMBO_DECAY`, `DRIVE_FLOOR`, `KILL_PEAK_ON`, `KILL_PEAK_OFF`, `SHAKE_MAX_PX`, `SHAKE_SECS`, `PULSE_MAX`, `FEEL_WATCHDOG`. **`KILL_VOL_MAX` still exists but its MEANING changed** — it is no longer the top of the metered range (see `KILL_PEAK_FULL` below).

## Since Last Handoff

The parent's plan (`PLAN_audio-adoption_survivor-audio-reactive-adoption_2026-07-30.md`) ran almost exactly as written, with two deliberate departures:

- **Phase 1 (board gate → ASK, do not self-pick) — ran as written.** Board empty on both channels, unmoved since 2026-07-27. Asked one Korean `AskUserQuestion` with four options and the reasoning.
- **The user did NOT pick an option — they asked whether any could be combined, and for a recommendation.** That is new: the previous three sessions were straight single-option picks. Recommended **2A + 2B as two separate PRs** (no file overlap; 2B parallelizable into a subagent; 2B carried unbuilt across two plans) and recommended *against* 2A+2C. The user took it. **Recommendation now taken 7 of 7.**
- **Phase 2A shipped, but NOT in the shape the plan preferred.** The plan preferred design (a), "a named channel with a `polyphonic` flag." Reading the code showed (a) rewrites the core of `playback.rs`. Presented the evidence and the user chose (b). See Key Decisions.
- **Phase 2B shipped, and its outcome was NOT the "none found" the plan flagged as a valid result.** Seven resources were being silently reverted.
- **Phase 3 (engine change as its own PR) fired twice**, producing three PRs total rather than the one or two the plan anticipated.
- **The parent's open question "Should the engine offer a meterable one-shot?" is ANSWERED — yes, and it shipped** (`Audio::play_tone_metered`, v0.140.0).
- **The parent's open question "Are there other engine-inserted config resources silently reverting?" is ANSWERED — yes, seven** (v0.139.1).
- **A risk the parent listed did NOT materialize:** "2A growing past one PR" was rated moderate-to-likely. It stayed within one facade method plus one playback method plus the analysis plumbing. The stop condition never had to fire.
- **A risk that DID materialize, unlisted:** adopting the new API in `survivor` broke that example's tuning, because a constant normalised against one voice's ceiling is invalid once the meter sums voices. Caught by measurement, not by the compiler.
- **Trajectory: the `audio-adoption` chain's arc has now closed.** Seq 1 found and named the gap; seq 2 closed it and adopted it. The measurement-backed menu item is spent.

## Reference Documents

- `CLAUDE.md` — project conventions, the verify gate, pre-1.0 versioning (MINOR = any release, PATCH = bugfix)
- `docs/MODULE_MAP.md` — 72 rows, "where do I read to find X". **Grep it, never read it whole** (~90 KB)
- `docs/VERIFICATION.md` — the 6 exit-code traps + 3 blind spots; carries the copy-paste gate block
- `docs/PATTERNS.md` — "Surviving a scene reset" (rewritten this session), shared-policy-for-cfg-split-backends
- `docs/CHANGELOG.md` — 0.139.1 / 0.140.0 / 0.141.0 entries written this session

---

## The Goal

Execute the seq-1 plan: run the board gate, and if both channels are empty, **ask** rather than self-pick. Then build whatever the user chose.

The plan's recommended item was unusual in being **measurement-backed rather than a hunch**: the seq-1 `survivor` adoption had proved that *meterability and overlap are mutually exclusive* — only a stable channel name can be metered, and a named-channel replay calls `stop_immediate` and cuts the sound already there. The plan wanted that closed, with an explicit internal gate: **agree the API shape with the user before building, and decide sum-vs-max meter semantics in writing.**

The user's answer widened the scope: combine the recommended item with the cheap, twice-deferred `insert_core_resources` audit. So the session had two independent deliverables plus, per the VISION rule, a real-play example proving the new API works.

## Where We Are

- **Everything is landed and merged. Tree clean, `main` @ `c2e1b82`, package v0.141.0, CLAUDE.md header v1.6.243.** Nothing in flight, no open branches.
- **Three PRs, all squash-merged, all 6/6 CI green:** #395 (01:30:22Z), #396 (01:43:38Z), #397 (01:56:44Z).
- **Lib tests 1339 → 1356** (+7 in #395, +10 in #396, 0 in #397). `ok` groups 152 throughout.
- **The verify gate ran 5 times this session, all exit 0, all read from the exit FILE** — session start, post-2B, post-2B-bump, post-2A, post-2A-example.
- **#395 / v0.139.1 — the scene-reset audit.** All 27 resources `insert_core_resources` inserts were classified. **Seven were session config with no persistence.** `FocusRingStyle`, `StickNavConfig`, `FrameConfig` (engine-inserted); `DesignResolution`, `WindowOptions`, `LightingConfig`, `DialogueStyle` (game-inserted).
- **Every one of the seven carries a regression test that was RUN BEFORE the fix and FAILED** — `24 passed; 7 failed` → `31 passed; 0 failed`. This converted the subagent's "reasoned from code paths, not observed" into observed.
- **The other 20 resources are correctly scene state, and that negative result is written into `docs/PATTERNS.md`** so the audit is not run a third time.
- **`TimeScale` is a deliberate exclusion** and is documented as the counter-example: config-shaped and engine-inserted, but games drive it moment-to-moment for hit-stop, so a frozen old scene leaking forward is the worse bug.
- **The audit moved the line.** v0.137.1's reasoning was "the engine inserts `WindowConfig`, so the engine persists it." Four of the seven are inserted by the *game* and fail identically. The line is now **"the engine defines the type and only reads it."**
- **⚠️ The audit FIRED the reopen trigger on the deferred `Audio` auto-persistence decision** — its own listed condition was "another engine-inserted config type turns out to be dropped the way `WindowConfig` was." Seven did. `Audio` was **deliberately left unchanged**; `docs/PATTERNS.md` records that the trigger fired and that the remaining argument is thinner. **This is now a live decision awaiting the user, not a dormant deferral.**
- **#396 / v0.140.0 — `Audio::play_tone_metered(meter, freq, dur, vol, bus)`**, a one-shot that overlaps itself *and* is metered. First genuinely new public API in this chain.
- **The plan's preferred design was overturned by reading the code.** Native `sinks: HashMap<String, Player>` is one-sink-per-channel and `stop_channel` / `is_playing` / `playback_state` / fades / effects all rest on that 1:1, so a `polyphonic` flag rewrites the core. Shipped the dedicated-method shape instead; **the channel model is untouched**.
- **Native rotates `POLY_VOICES = 8` sink channels private to the meter name** (`__poly_{meter}_{i}`) while pointing every voice's tap at the one meter entry.
- **wasm needed no voice pool at all** — `play_tone_to` already builds a fresh oscillator per call, so the only missing piece was `tap(meter, &gain)`.
- **Meter semantics = SUM of the sounding voices, clamped to 1.0**, decided in writing with the user. Not a free choice: Web Audio mixes multiple inputs into one `AnalyserNode`, so a native `max` would have made the backends disagree about *meaning*. Policy lives once in un-gated `audio_analysis::sum_levels`.
- **All 10 new tests are device-free** — `next_poly_voice` and `combine_voices` were extracted from their call sites precisely so CI (which has no audio device) can test the rotation and the summing.
- **#397 / v0.141.0 — `survivor` adopts it**, the VISION acceptance test, verified on a real device.
- **Measured: 22 of 25 kill-tone replays were cutting a still-sounding tone.** The parent's stated reason for putting the kill tone on a named channel ("it fires at most once per frame, so it can afford that") was wrong on the numbers.
- **Measured: the metered peak went 0.6000 → 1.0000**, with 0 → 61 frames of 301 reading above 0.60. That is the sum semantics visible end-to-end on hardware.
- **That range change broke the tuning, and had to be fixed.** `drive_from_amplitude` normalised against `KILL_VOL_MAX = 0.60` — one voice's ceiling — so 31% of sounding frames saturated at full shake. Re-basing on a new `KILL_PEAK_FULL = 1.0` brought that to 6%.
- **A second claim of the parent's was disproved by arithmetic:** the bullet tone was kept on the anonymous ring because it "fired every 0.14 s" and could not afford being cut — but the tone is **0.04 s** long, so consecutive bullet tones never overlap and a named channel would never have cut one.
- **Memory advanced to seq 215** (35,116 → 41,012 bytes), `MEMORY.md` hook rewritten (5,805 → 6,119 bytes). Trim still due ~seq 220.
- **Two temporary probes were added and removed**; removal confirmed by an independent `grep` returning 0, never by the removal script's own exit code.

## What We Tried (Chronological)

### Chunk 1 — Board gate, the ask, and a new kind of answer (early)

1. **Launched the verify gate in the background and read the board in parallel**, using the `docs/VERIFICATION.md` block verbatim (`rm -f` first, non-piped, wait on the file). The parent recorded that the completion notification has lied in four consecutive sessions.
2. **Read both board channels read-only.** Used `awk '/^## Active requests/,/^## Done \/ archive/'` on the 53 KB `dungeon-merchant` file rather than `cat`, per the parent's note that a raw `cat` overflows the tool result.
3. **Checked whether the board had MOVED**, which is the actual gate condition rather than "does it look empty": `git log -1 --date=short -- docs/engine-wishlist.md` → **2026-07-27 (`539b610`)**, not later. No preemption. `rust-survivors` → `_None._`, last touched 2026-07-14 (`7794358`).
4. **Read the gate result from `/tmp/v.exit`, not the notification.** `EXIT=0`, mtime 09:38, all 8 `[verify]` markers present including `all checks passed ✓`, 152 ok groups, 1339 lib tests — corroborated against the expected counts, which is the step `docs/VERIFICATION.md` added in #390.
5. **Created three tasks with `TaskCreate`**, one per plan phase, as the paste prompt instructed (no `bd` in this repo).
6. **Asked ONE Korean `AskUserQuestion`** with all four Phase-2 branches and the *reasoning* for each, leading with 2A and stating explicitly that it differs from the usual menu in being measurement-backed.
7. **The user did not pick an option.** They asked: *"위 선택지에 있는 작업들 같이 진행 할 수 있는게 있으면 한번에 작업 하면 좋겠는데 추천 하는 방향 있을까?"* — can any of these be combined, and what do you recommend? This is the first time in this chain the answer was not a straight pick.
8. **Recommended 2A + 2B as two separate PRs, with reasons and with explicit anti-recommendations.** For: zero file overlap (2A is `audio_facade.rs` / `audio/playback.rs` / `examples/games/survivor`, 2B is `app/core_resources.rs` / `app.rs` / `docs/PATTERNS.md`), so they cannot contaminate each other's verification; 2B is the only branch that parallelizes cleanly into a read-only subagent; 2B was carried unbuilt across two plans. Against: **2A+2C** because both touch the audio subsystem and re-tuning `beat_crawler`'s low-band detector while `playback.rs` is in flux moves the measurement baseline — and because 2A has an explicit stop condition that a same-subsystem sibling would defeat. Against **2D** as lowest marginal value.
9. **Launched the 2B audit as an `Explore` subagent with an explicit `model: sonnet`** (standing policy: always pass a model) and six numbered fact-finding questions, while reading `audio_facade.rs:80-330` myself for 2A. Told it to grep the module map, never read it whole, and told it explicitly that "no bugs found" is a valid and expected result — so it would not manufacture findings.

### Chunk 2 — 2B: verifying the subagent, proving the bug, and the trigger that fired (early-mid)

10. **Verified the subagent's crux claims myself rather than trusting them** (the parent recorded one overstatement from the seq-1 subagent). Read `src/app/core_resources.rs` — **27 `insert_resource` calls, lines 15–41**, exactly as reported, and confirmed from the import list that `DesignResolution` / `WindowOptions` / `LightingConfig` / `DialogueStyle` are genuinely *not* inserted.
11. **Found the fact that made the fix general, and the subagent had not drawn it out:** `reload_scene` (`scenes.rs:13-17`) collects preserved resources with `filter_map(|&tid| self.world.take_resource_erased(tid)...)`, so **registering a type the engine never inserts is free — absent types are simply skipped.** That is what lets the four game-inserted config types be covered by the same one-liner as the engine-inserted ones.
12. **Read the `WindowConfig` fix and its test** to reuse the shape: `app.register_persistent::<crate::resources::WindowConfig>();` at `app.rs:305`, test `window_config_survives_a_scene_reset` at `app.rs:404` — a plain `reload_scene()` unit test needing no window or GPU.
13. **Checked every candidate for engine-side mutation before classifying it as read-only config.** Grepped `resource_mut::<T>` / `insert_resource(T` across `src/` for all eight suspects: **no production path mutates any of them** — the only hits are doc examples and `#[cfg(test)]` blocks. This is what justified "the engine defines it and only reads it" as the line.
14. **Read the struct definitions** to confirm they are plain config and to write meaningful assertions: `DesignResolution{width,height}`, `WindowOptions{resizable,mode,lock_aspect,...}`, `LightingConfig{max_lights}`, `DialogueStyle{...14 fields}`, `FrameConfig{max_dt}` (default 0.1), `FocusRingStyle{color,thickness,corner_radius,enabled,pulse_hz,pulse_min_alpha}`, `StickNavConfig{activate,release}`.
15. **Wrote all seven regression tests BEFORE the fix and ran them.** Result: `test result: FAILED. 24 passed; 7 failed`. Every one failed. This is the step that turned the subagent's honest caveat — *"no example anywhere both sets one of these AND performs a scene transition afterward, so the actual silent-revert has not been observed, only reasoned"* — into an observation.
16. **Then added the seven `register_persistent` lines** to `App::new` after the `WindowConfig` one. Re-ran: `test result: ok. 31 passed; 0 failed`.
17. **Rewrote `docs/PATTERNS.md`'s "Surviving a scene reset"**: extended the auto-registered table with both new groups, added an audit subsection with the 20-resource negative result grouped into four categories, recorded the `TimeScale` exclusion with its reasoning, and stated the new line.
18. **Recorded that the `Audio` reopen trigger fired, without acting on it.** The deferred block named "another engine-inserted config type turns out to be dropped the way `WindowConfig` was" as a reopen condition; seven did. Rewrote the block to say so, to state precisely how the fix *weakened* the original argument (four of seven are game-inserted, so "who inserts it" was never the distinction), and to state what still separates `Audio` (the engine *drives* it every frame rather than reading it as config, and it owns a device handle rather than a value). Left the behaviour untouched on purpose.
19. **Extended module-map row 31 rather than adding a row**, per the house rule.
20. **Shipped as PATCH 0.139.1**, following the v0.137.1 precedent for the identical fix. Gate #2 (post-implementation) exit 0, 152 groups, 1346 tests; gate #3 (post-bump) identical.
21. **Landed #395 with async auto-merge** per standing delegated merge authority. 6/6 green, merged 01:30:22Z.

### Chunk 3 — 2A: the read that overturned the plan, and the implementation (mid)

22. **Read `src/audio/playback.rs` and `src/audio/analysis.rs` in full before designing anything.** The decisive finds: `play_tone` opens with `self.stop_immediate(channel)` at `:180` (the cut), `tapped(channel, source)` at `analysis.rs:385-394` looks the slot up **by channel name only**, and `LevelSlot::publish` is a plain relaxed store with a release-ordered seq bump.
23. **Concluded the plan's preferred shape (a) was the expensive one.** `sinks: HashMap<String, Player>` is one sink per channel, and `stop`/`stop_immediate`/`playback_state`/`is_playing`/`is_finished`/fades/effects all key on that. A `polyphonic` flag means a channel holds N sinks — a rewrite of the core model. Whereas the *meter* is already decoupled: nothing stops several sinks from being tapped into one analysis entry.
24. **Read the wasm backend and found the asymmetry that decided the semantics.** `audio_wasm.rs:368-375`'s `tap(channel, node)` connects a source's gain node to the channel's `AnalyserNode`; `play_tone_to` (`:525`) already builds a fresh oscillator per call. So wasm needs **no pool**, and because Web Audio mixes multiple inputs into one node, its metering semantics are **inherently sum**.
25. **Asked ONE Korean `AskUserQuestion` with two questions**, grounded in that reading: the API shape (recommending (b) and explaining why the plan's (a) was wrong), and the meter semantics (recommending sum, because the web graph sums whether we like it or not and a native `max` would make the backends disagree about meaning rather than rounding). The user took both recommendations.
26. **Wrote the policy first, in the un-gated module.** `audio_analysis::sum_levels` carries the decision, the reasoning, the native/wasm asymmetry, and the equivalent-not-identical caveat — satisfying `docs/PATTERNS.md`'s rule that a value both backends derive lives in ONE un-gated module.
27. **Rejected a tempting shortcut and recorded why.** Making `LevelSlot::publish` accumulate (so N voices sum into one slot) would have avoided per-voice slots entirely — but windows are ~10 ms and frames ~16 ms, so accumulate-and-drain conflates summing across *voices* with summing across *time*, and two windows of one voice would read as two voices. Per-voice slots are the honest structure.
28. **Implemented per-voice slots:** `PolyVoice { slot, last_seq }`, `AnalysisChannel.poly: Vec<PolyVoice>` (empty for an ordinary channel, so the existing path is byte-identical), `poly_tapped(meter, voice, source)` creating slots on demand, and `combine_voices(single, &mut poly)` applying the staleness test **per voice** before summing.
29. **Deliberately left spectrum OFF on voice slots and documented it.** `bands()` reads the channel's own slot; summing eight band arrays per window would put a per-voice FFT on the playback thread to serve an API whose use case is a soundtrack, not a one-shot.
30. **Extracted `next_poly_voice` and `combine_voices` as free functions specifically so CI can test them.** CI has no audio device, so anything reachable only through `AudioManager::new()` is untestable there. This is why the rotation (the thing that makes plays *not* cut each other) and the summing both have real coverage.
31. **Implemented the manager method `play_tone_poly`**, which owns the rotation, the naming, the bus assignment and the tap redirection, so the facade method is a one-liner and no new state lives in `Audio`.
32. **Implemented wasm as `play_tone_to_metered(..., meter: Option<&str>)`** with `play_tone_to` delegating with `None`, so the existing path is unchanged and the metered path differs by exactly one `self.tap(meter, &gain)`.
33. **De-linked three intra-doc references to private items** (`AudioManager::poly_tapped`, `sfx_voice_channel`, `POLY_VOICES`) before running the gate. CI runs `RUSTDOCFLAGS="-D warnings" cargo doc`, and a link to an undocumented private item is a broken-link warning.
34. **Updated the two engine docs the new API falsified.** `enable_analysis`'s "Which channels can be metered" and `play_tone_on_bus` both asserted the trade-off as unconditional; both now point at `play_tone_metered` and scope the remaining trade-off to clips.
35. **Caught my own error from PR #395.** While writing the 0.140.0 entry I noticed the 2B docs had dated the audit **v0.140.0** in five places while it actually shipped as **v0.139.1**. Fixed all five and logged it in the CHANGELOG's `### Fixed`.
36. **Gate #4** exit 0, 152 groups, 1356 tests. Landed #396, 6/6 green, merged 01:43:38Z.

### Chunk 4 — 2A's acceptance test: three measurements, one of which broke the tuning (late)

37. **Checked the plan's suggested acceptance test against the actual numbers before doing it.** The plan said to flip `survivor`'s **bullet** tone onto the new API. But the bullet tone is `play_tone(900.0, 0.04, 0.16)` fired on a `FIRE_COOLDOWN = 0.14` timer — **0.04 s of sound every 0.14 s, so consecutive bullet tones never overlap.** A named channel would never have cut one. The parent's justification for leaving it on the ring was wrong, and the plan inherited it.
38. **Identified the real target: the kill tone**, which is already on a named channel (`play_tone_on_channel`, dur 0.14) with kills landing at a comparable cadence — so it is where the cut actually happens.
39. **Measured it instead of asserting it.** Added a temporary `CUTPROBE` printing `audio.is_channel_playing(channel)` immediately before each replay, and ran the game headlessly with the parent's scripted input (`G` invulnerable, `B`×3 for +150 enemies, hold `ArrowRight` to fire forever) under `ENGINE_INPUT` + `ENGINE_CAPTURE`. Result: **25 replays, 22 of them cut a tone that was still sounding.**
40. **Confirmed a real audio device was present in every run** by grepping for rodio's `Dropping DeviceSink, audio playing through this sink will stop` teardown line, which only appears if a sink was actually playing. Without it a green capture proves nothing about audio.
41. **Captured a "before" peak distribution** with a `PEAKPROBE`: 301 frames, **max 0.6000, zero frames above 0.60** — pinned exactly at the single-voice ceiling `KILL_VOL_MAX`, matching the parent's recorded v3 number exactly.
42. **Migrated the kill tone** and renamed for accuracy: `KILL_CHANNEL` → `KILL_METER` (it no longer names a channel), `play_tone_named` → `play_metered_tone`, body → `Audio::play_tone_metered`.
43. **Re-measured: peak max 1.0000, with 61 of 301 frames above 0.60.** The sum working end-to-end on hardware — something the device-free unit tests cannot show.
44. **Noticed the consequence rather than celebrating the number.** `drive_from_amplitude` normalises `(amp - KILL_VOL_BASE) / (KILL_VOL_MAX - KILL_VOL_BASE)` — against **one voice's** ceiling. With sums arriving, that saturates. Computed the drive distribution offline from the captured peaks (drive is a pure function of peak, so no extra run was needed): **61 of 197 sounding frames (31%) pinned at full shake.**
45. **Added `KILL_PEAK_FULL = 1.0`** — the summed ceiling, where the engine clamps — and re-based the mapping. Saturation fell to **12 of 197 (6%)**, p50 drive 0.761 → 0.566, which puts a single kill mid-range and reserves the top for several kills landing together.
46. **Removed both probes and verified by independent `grep`, not by the removal script's exit code.** The parent recorded a new variant of the `cargo fmt` reflow trap where a deletion regex silently matched nothing while reporting success. `grep -c "eprintln\|PEAKPROBE\|CUTPROBE\|TEMP"` → **0**.
47. **Took a final capture and read the HUD** to confirm metering is live end to end: `combo 13.6   kill meter 0.39  → shake/pulse (audio-driven)` in **blue**, not the amber watchdog line.
48. **Corrected both stale claims in the docs** — the example's own module docs, and module-map row 79's finding (1), which had asserted "meterability and overlap are mutually exclusive" as a standing fact.
49. **Shipped as MINOR 0.141.0** following the seq-1 precedent (an example gaining a user-visible capability is not a bugfix). Gate #5 exit 0, 152 groups, 1356 tests. Landed #397, 6/6 green, merged 01:56:44Z.
50. **Synced main, deleted all three local branches, updated memory to seq 215.**

## Key Decisions

- **Recommended combining 2A + 2B rather than picking one.** The user asked for a combination; the honest answer was that only these two combine safely — they share no files, and 2B parallelizes into a subagent while 2A's design work happens in the main context. Explicitly recommended *against* 2A+2C, because two audio-subsystem changes in flight would move the measurement baseline and would defeat 2A's stop condition.
- **Overturned the plan's preferred API shape after reading the code, and said so to the user with the evidence** rather than silently substituting. The plan's argument for (a) — "it reuses the existing channel/bus/effect plumbing and does not add a second naming scheme" — is true but ignores that native holds one sink per channel and that six methods rest on that. Shape (b)'s cost (a meter name that is not addressable by `stop_channel` / `set_low_pass`) is real, and is documented on the method rather than hidden.
- **Chose SUM over MAX because wasm forces it, not because sum is prettier.** Multiple sources into one `AnalyserNode` are mixed by Web Audio. Matching that on native keeps the two backends agreeing about *meaning*; the residual difference (native sums per-voice measurements, the browser measures the summed signal) is documented with the same "equivalent, not identical" wording `bands()` already uses.
- **Gave each voice its own `LevelSlot` instead of sharing one.** Sharing would race — `publish` is a plain store, so a reader sees whichever voice published last, which is neither the sum nor the loudest. Pinned by `overlapping_voices_sum_into_one_reading`.
- **Applied the staleness test per voice, not per name.** Per name, a drained voice keeps contributing its last level for as long as any other voice still sounds, so the meter never falls between bursts. Pinned by `a_drained_voice_stops_contributing_while_others_still_sound`.
- **Rejected accumulate-in-the-slot** as a cheaper alternative to per-voice slots: it conflates summing across voices with summing across time, because several windows land per frame.
- **Extracted `next_poly_voice` / `combine_voices` purely for testability.** CI has no audio device, so logic reachable only through `AudioManager::new()` would ship untested. This is a deliberate structural concession, and it is why the branch has 10 real tests rather than 2.
- **Wrote the seven scene-reset tests BEFORE the fix.** The subagent's report was explicit that the failure mode was reasoned, not observed, in every case. Running the tests first is what made the claim observed — and it is the #388 precedent ("proven non-vacuous by disabling the fix") applied in the correct order.
- **Included the four game-inserted config types in the scene-reset fix**, which is broader than "engine-inserted resources" — justified because `reload_scene` skips absent types, so the registration is free, and because the failure is identical. This *changed the stated line* and that change is documented.
- **Excluded `TimeScale` and wrote down why.** It would have been easy to include for uniformity; a slowed or frozen old scene leaking into the next one is worse than losing a hit-stop.
- **Recorded that the `Audio` reopen trigger fired, but did not act on it.** The plan's anti-goals said not to reopen it; the doc's own trigger said this is exactly when to. Resolved by doing neither silently: the behaviour is unchanged, the trigger is recorded as fired, and the decision is surfaced to the user. Folding an `Audio` behaviour change into the audit that raised the question would have buried it.
- **Landed three separate PRs rather than one or two**, per the #388 precedent, so the engine API can stand or revert independently of both the audit and the example that exercises it.
- **Reported the parent's bullet-tone justification as wrong rather than quietly not doing it.** The plan named the bullet tone as the acceptance test; the arithmetic says it was never the problem. Saying so is more valuable than performing the migration the plan expected.

## Evidence & Data

### Board gate — both channels (Phase 1)

| Channel | Open requests | Next free ID | Last modified | Moved? |
|---|---|---|---|---|
| `../dungeon-merchant/docs/engine-wishlist.md` | `*(none — all filed requests are closed)*` | **EW-012** | 2026-07-27 (`539b610`) | **No** (gate condition: later than 2026-07-27) |
| `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` | `_None._` | n/a | 2026-07-14 (`7794358`) | No (paused/deprecated) |

### Verify-gate history — every run, exit code read from the FILE

| # | When | Exit file | Exit | ok groups | lib tests | Notes |
|---|---|---|---|---|---|---|
| 1 | Session start | `/tmp/v.exit` (mtime 09:38) | **0** | 152 | 1339 | All 8 `[verify]` markers; baseline v0.139.0 |
| 2 | Post-2B implementation | `/tmp/v2.exit` (mtime 10:19) | **0** | 152 | 1346 | +7 scene-reset tests |
| 3 | Post-2B version bump | `/tmp/v3.exit` (mtime 10:23) | **0** | 152 | 1346 | Required by `/ship` convention |
| 4 | Post-2A implementation + bump | `/tmp/v4.exit` (mtime 10:36) | **0** | 152 | 1356 | +10 metered-one-shot tests |
| 5 | Post-2A example + bump | `/tmp/v5.exit` (mtime 10:47) | **0** | 152 | 1356 | Example change, no new tests |

**The false-green notification did NOT bite this session** — the `docs/VERIFICATION.md` block was used from the first launch and every exit code came from the file. This is the fifth consecutive session the block has been needed.

### The scene-reset audit — 27 resources classified

| Group | Count | Members | Verdict |
|---|---:|---|---|
| **Session config — WAS BEING REVERTED (fixed)** | **7** | `FocusRingStyle`, `StickNavConfig`, `FrameConfig`, `DesignResolution`, `WindowOptions`, `LightingConfig`, `DialogueStyle` | Now `register_persistent` |
| Already persisted | 1 | `WindowConfig` | Fixed in v0.137.1 (#388) |
| Device / derived state | 6 | `InputState`, `GamepadState`, `TouchState`, `RealDt`, `ViewportSize`, `PendingResize` | Correctly reset — rewritten every frame |
| Per-frame draw queues | 4 | `TextQueue`, `UiQueue`, `UiImageQueue`, `DebugDraw` | Correctly reset — cleared every frame |
| Scene-scoped state | 8 | `GameState`, `Camera`, `UiFocus`, `SelectedEntity`, `ProfilerData`, `SceneChange`, `LoadProgress`, `ShouldQuit` | Correctly reset — several hold dead `Entity` ids |
| Rebuilt by another mechanism | 4 | `AssetServer`, `ScriptRegistry`, `SerdeComponentRegistry`, `PanickedSystems` | Correctly reset — path-keyed caches or re-registered by `reload_scene` |
| **Deliberate exclusion** | 1 | `TimeScale` | Live gameplay effect; must NOT leak across scenes |

Note the last four groups sum to 22 and overlap the 27 by counting `WindowConfig` and `TimeScale` separately; the shipped statement is "7 fixed, 20 correctly scene state."

### The seven regression tests — run BEFORE the fix

```
test result: FAILED. 24 passed; 7 failed; 0 ignored; 1315 filtered out     <- before
test result: ok.     31 passed; 0 failed; 0 ignored; 1315 filtered out     <- after
```

Failures, verbatim:

```
app::tests::design_resolution_survives_a_scene_reset
app::tests::dialogue_style_survives_a_scene_reset
app::tests::focus_ring_style_survives_a_scene_reset
app::tests::frame_config_survives_a_scene_reset
app::tests::lighting_config_survives_a_scene_reset
app::tests::stick_nav_config_survives_a_scene_reset
app::tests::window_options_survive_a_scene_reset

scene reset dropped LightingConfig                                    (app.rs:558)
scene reset dropped WindowOptions                                     (app.rs:539)
assertion `left == right` failed: scene reset reverted the game's stick deadzone tuning
  left: (0.6, 0.35)     <- StickNavConfig::default()
 right: (0.9, 0.2)      <- what the game set
```

### The kill tone was being cut — measured, 300-frame headless run

| Metric | Value |
|---|---|
| Kill-tone replays in the run | **25** |
| Replays where the previous tone was **still sounding** | **22** |
| Fraction cut | **88%** |
| Audio device present (rodio `DeviceSink` teardown line) | yes, both runs |

### Metered peak — before vs after the migration (same input script, 301 frames)

| Metric | before (`play_tone_on_channel`) | after (`play_tone_metered`) |
|---|---|---|
| frames sampled | 301 | 301 |
| frames with signal (>0.05) | 239 | 197 |
| peak **p50** | 0.3746 | **0.4385** |
| peak **p90** | 0.6000 | **0.8889** |
| peak **p99** | 0.6000 | **1.0000** |
| peak **max** | **0.6000** (pinned at `KILL_VOL_MAX`) | **1.0000** |
| frames reading **above 0.60** | **0 of 301** | **61 of 301** |

The `before` max of exactly 0.6000 reproduces the parent's recorded v3 number, which is what makes the comparison trustworthy.

### The tuning break the range change caused, and the fix

| Drive normalisation | p50 drive | p90 drive | Saturated frames (drive ≥ 0.999) |
|---|---|---|---|
| old top = `KILL_VOL_MAX` 0.60 | 0.761 | 1.000 | **61 / 197 (31%)** |
| new top = `KILL_PEAK_FULL` 1.00 | 0.566 | 0.914 | **12 / 197 (6%)** |

Computed offline from the captured peaks — `drive_from_amplitude` is a pure function of peak, so no extra run was needed.

### The new API's tests — all device-free

| Test | Module | What it pins |
|---|---|---|
| `consecutive_metered_one_shots_take_different_voices` | `audio/analysis.rs` | The core property: a replay does not reuse the sink, so it cannot cut |
| `voices_rotate_and_wrap_at_the_ring_size` | `audio/analysis.rs` | Bounded at 8 — voice 8 reuses slot 0, not a ninth sink |
| `each_name_rotates_independently` | `audio/analysis.rs` | Two meters do not share a counter |
| `overlapping_voices_sum_into_one_reading` | `audio/analysis.rs` | 0.2+0.1 → 0.3 rms, 0.3+0.25 → 0.55 peak |
| `a_drained_voice_stops_contributing_while_others_still_sound` | `audio/analysis.rs` | Per-voice staleness: 0.5 → 0.3 after one voice drains |
| `a_summed_reading_is_clamped_to_full_scale` | `audio/analysis.rs` | 8 voices at 0.6/0.9 → 1.0/1.0 |
| `an_ordinary_channel_is_unaffected_by_the_poly_path` | `audio/analysis.rs` | Empty poly ⇒ identity, so existing meters are byte-identical |
| `summing_voices_adds_them_rather_than_taking_the_loudest` | `audio_analysis.rs` | The decision itself: max would say 0.5, sum says 0.9 |
| `a_sum_past_full_scale_is_clamped` | `audio_analysis.rs` | 3×0.7 → 1.0 |
| `summing_one_voice_or_none_is_the_identity_or_silence` | `audio_analysis.rs` | Degenerate cases |

`cargo test --lib audio` → **103 passed; 0 failed**.

### Shipped — three PRs

| PR | Commit | Version | Bump | Diffstat | Merged (UTC) |
|---|---|---|---|---|---|
| #395 | `cf0763f` | 0.139.1 | PATCH | 7 files, +246 / −11 | 2026-07-31T01:30:22Z |
| #396 | `7872ec6` | 0.140.0 | MINOR | 12 files, +475 / −19 | 2026-07-31T01:43:38Z |
| #397 | `c2e1b82` | 0.141.0 | MINOR | 6 files, +68 / −28 | 2026-07-31T01:56:44Z |

All three: 6/6 required checks green (Test native, Render tests lavapipe, Build WASM, Build Windows/DX12, Rustdoc, Package dry-run). All three landed with `gh pr merge --auto --squash` per standing delegated merge authority.

### The three APIs for a tone, after this session

| | `play_tone` / `play_tone_on_bus` | `play_tone_on_channel` | **`play_tone_metered`** (new) |
|---|---|---|---|
| Overlaps itself | ✅ anonymous ring, 16 voices shared globally | ❌ `stop_immediate` on replay | ✅ 8 voices private to the meter name |
| Metered by `levels()` | ❌ no stable name | ✅ | ✅ |
| `stop_channel` / `set_low_pass` / `is_channel_playing` | ❌ | ✅ | ❌ (meter name is not a channel) |
| `set_effect` (pitch / low-pass / attack) | ❌ | ✅ | ❌ |
| `bands()` spectrum | ❌ | ✅ | ❌ (zeros — an FFT per voice) |
| Use when | fire and forget | sustained / re-armed / needs control | repeats faster than it decays, and you want to read it |

### HUD verdict strings (what a capture must show)

```
combo 13.6   kill meter 0.39  → shake/pulse (audio-driven)        [150,220,255] blue   <- metered, watchdog idle
combo  4.0   kill meter  --   → shake from combo (nothing audible) [230,190,130] amber  <- no device / analysis off
```

Final capture at frame 260 showed the **blue** line with `kill meter 0.39`, Kills 19, Enemies 152.

### The direction question exactly as presented (for calibration)

The parent recorded that *how* the menu is framed drives the answer, and asked that the pattern be reproduced. Four options, trade-offs spelled out, recommendation first and labelled:

| Option | Korean label | Core of the reasoning given |
|---|---|---|
| 1 | `미터링 가능한 원샷 API (추천)` | Meterability and overlap provably exclusive today (`stop_immediate` on named replay); **the only item backed by measurement rather than a hunch**; but it is a public API addition, so the shape and the sum-vs-max semantics must be agreed *before* building, and the branch has a stop condition |
| 2 | `core_resources 씬리셋 감사` | `WindowConfig` was found by accident after being worked around twice — that is not a search strategy; cheap and bounded; **a written "none found" is a valid result** that stops the next session re-running it |
| 3 | `beat_crawler 실제 사운드트랙` | Today it meters two synthesized tones 6.5× apart — a test that cannot fail; a real mix is `bands()`'s actual use case and the first place the low-band detector could genuinely fail; costs a licence-clean file |
| 4 | `4번째 맵 생성기 (drunkard's walk)` | ~80 lines, connected by construction, deterministic — but rooms/caves/mazes + `roguelike` + `beat_crawler` already cover this; a fourth repetition teaches least |

**The user answered none of them**, asking instead which could be combined. The recommendation given, and its anti-recommendations:

| Verdict | Combination | Reasoning given |
|---|---|---|
| **Recommended** | 2A + 2B, two separate PRs | Zero file overlap (`audio_facade.rs`/`audio/playback.rs`/`examples/games/survivor` vs `app/core_resources.rs`/`app.rs`/`docs/PATTERNS.md`), so neither can contaminate the other's verification and each reverts independently (#388 precedent); 2B is the only branch that parallelizes into a read-only subagent; 2B carried unbuilt across two plans and a "none found" would close it for good |
| **Advised against** | 2A + 2C | Both are audio-subsystem work: re-tuning `beat_crawler`'s low-band detector while `playback.rs` is in flux moves the measurement baseline, and 2A's explicit stop condition ("stop if it grows past one facade method plus one playback change") is defeated by a same-subsystem sibling |
| **Advised against** | + 2D | Fully independent and safe to add, but lowest marginal value — most tokens for least learning |

### The second question — the 2A design gate (asked before writing any 2A code)

The plan mandated agreeing the shape and the semantics in writing first. Both questions led with the recommendation and the evidence that produced it:

| Question | Options | Answer |
|---|---|---|
| API shape | (b) **dedicated method + per-name voice pool** *(recommended — reverses the plan)* vs (a) polyphonic flag on a named channel *(the plan's original)* | **(b)** |
| Meter semantics | **sum — the mixed signal** *(recommended)* vs max — the loudest voice | **sum** |

The shape question explicitly told the user the plan was being reversed and why: native holds one sink per channel, and `stop_channel` / `is_playing` / `playback_state` / fades / effects all rest on that 1:1, so (a) rewrites `playback.rs`'s core. The semantics question led with the fact that Web Audio mixes multiple inputs into one `AnalyserNode`, so max would make the backends disagree about meaning rather than rounding.

### Engine doc corrections forced by the new API

Two public docs asserted the trade-off as an unconditional fact and became false the moment `play_tone_metered` shipped:

| Location | Was | Now |
|---|---|---|
| `Audio::enable_analysis`, "Which channels can be metered" | "Moving a sound onto a named channel to meter it is a real trade-off, not a free rename: a replay on a named channel **cuts** the sound already there" | Scoped to the past tense, points at `play_tone_metered` for tones, and notes **clips (`play_sfx`) still face the original trade-off** |
| `Audio::play_tone_on_bus` | "cannot be metered … use `play_tone_on_channel` for that" | Distinguishes the two alternatives: `play_tone_metered` (overlaps *and* meters) vs `play_tone_on_channel` (metered, stoppable, filterable, but cuts on replay) |
| `examples/games/survivor` module docs, finding (1) | "Meterability and overlap are mutually exclusive today." | Records the measured 22/25 cut rate, the 0.6000 → 1.0000 range change, and that the range change forced re-basing the drive normalisation |
| `docs/MODULE_MAP.md` row 79, finding (1) | Same claim, asserted as standing fact | Marked **CLOSED in v0.140.0**, with the design, the sum policy, the limits, and the adoption evidence |

### Three intra-doc links removed before the gate

CI runs `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`, and a link to an item rustdoc does not document is a hard failure. All three targets are private:

| Link written | Target visibility | Changed to |
|---|---|---|
| `` [`AudioManager::poly_tapped`] `` | `pub(super)` | plain code span |
| `` [`sfx_voice_channel`](crate::audio_facade::sfx_voice_channel) `` | private `fn` | plain code span + prose pointer |
| `` [`POLY_VOICES`] `` | `pub(crate)` | plain code span |

### An error I made and caught myself

While writing the 0.140.0 CHANGELOG I noticed the 2B docs — already merged in #395 — dated the scene-reset audit **v0.140.0** in five places, when it shipped as **v0.139.1**:

| File | Occurrences | Text |
|---|---:|---|
| `docs/PATTERNS.md` | 3 | "the audit below (v0.140.0)", the audit heading, "fired in the v0.140.0 audit above" |
| `docs/MODULE_MAP.md` | 2 | "since v0.140.0 the config family", "The v0.140.0 audit classified" |

Fixed in #396 and logged under its `### Fixed`. Cause: I drafted the 2B docs expecting a MINOR bump, then correctly shipped PATCH per the v0.137.1 precedent without re-reading the prose.

### Board file size — a small discrepancy worth noting

The parent recorded `../dungeon-merchant/docs/engine-wishlist.md` as **64.4 KB**; it measured **53,374 bytes** this session, with `git log` showing no commit since 2026-07-27. The board content did not change, so the parent's figure was most likely the tool-result size rather than the file size. Practical consequence unchanged: **do not `cat` it** — extract with `awk '/^## Active requests/,/^## Done \/ archive/'`.

### Memory footprint

| File | Before | After | Δ |
|---|---|---|---|
| `engine-current-state.md` | 35,116 B | **41,012 B** | +5,896 (3 seq entries) |
| `MEMORY.md` | 5,805 B | 6,119 B | +314 (hook rewritten) |

Grows ~1.5–2 KB/seq against a ~76 KB read cap ⇒ **trim still due by ~seq 220**, unchanged from the parent's estimate.

### Subagent survey — cost and what needed correcting

One `Explore` agent, `model: sonnet`, 6 numbered questions, run in parallel with my own 2A code reading. **101,602 tokens / 59 tool calls / 506 s.**

| Claim returned | Verdict after my own check |
|---|---|
| 27 resources, exhaustive list with file:line | **Correct** — verified against `core_resources.rs:15-41` |
| `DesignResolution`/`WindowOptions`/`LightingConfig`/`DialogueStyle` are NOT inserted by the engine | **Correct** — confirmed from the import list too |
| `register_persistent` ground truth (4 named + 7 RON registries) | **Correct** |
| `WindowConfig` fix at `app.rs:305` + test at `:404` | **Correct** |
| Three suspects: `FocusRingStyle` (strong), `StickNavConfig` (moderate), `FrameConfig` (weak) | **Correct but under-called** — it rated them unconfirmed because no *example* both sets one and calls `set_scene`. A `reload_scene()` unit test is the confirmation, and all seven failed. |
| The four out-of-scope types "strictly out of Q1's scope" | **True but the wrong conclusion** — it did not notice that `reload_scene`'s `filter_map` makes registering an absent type free, so they are fixable by the same one-liner |

The pattern from seq 1 held: the subagent's *facts* were reliable, its *judgement about what to do with them* needed checking.

## Code Analysis

- **The anonymous ring, unchanged.** `SFX_VOICES = 16`; `sfx_voice_channel(seq, voices) -> "__facade_sfx_{seq % voices}"`; four callers — `play_sfx` (`audio_facade.rs:137`), `play_sfx_on_bus` (154), `play_tone` (176), `play_tone_on_bus` (198).
- **The new per-name ring.** `POLY_VOICES: u64 = 8` and `poly_voice_channel(meter, voice) -> "__poly_{meter}_{voice}"` in `src/audio/analysis.rs`; `next_poly_voice(&mut HashMap<String,u64>, meter) -> usize` does the rotation. Half the anonymous ring's size because these are *per name*, where the ring is shared by every fire-and-forget sound at once.
- **Why the channel model survived.** `AudioManager::play_tone_poly` (`playback.rs`) calls `assign_bus`, `stop_immediate` (only on wrap), `Player::connect_new`, `effective_volume`, and inserts into `self.sinks` under the **voice** channel name. Every one of those is the ordinary single-sink path; nothing in `playback.rs` learned about polyphony.
- **Where the meter diverges from the sink.** `poly_tapped(meter, voice, source)` looks the analysis entry up under `meter`, grows `channel.poly` to cover `voice`, and wraps the source in a `LevelTap` over that voice's own `Arc<LevelSlot>`. `tapped` (the single-voice path) is untouched.
- **`combine_voices(single, &mut [PolyVoice]) -> AudioLevels`** reads each voice's `(rms, peak, seq)`, treats `seq == last_seq` as drained, and hands the survivors to `sum_levels`. `tick_analysis` calls it only when `!channel.poly.is_empty()`, so an ordinary channel's arithmetic is unchanged.
- **`sum_levels(impl IntoIterator<Item = AudioLevels>) -> AudioLevels`** in un-gated `src/audio_analysis.rs` — adds `rms` and `peak` independently and clamps each to 1.0.
- **`LevelSlot::publish`** is `rms.store(Relaxed)`, `peak.store(Relaxed)`, `seq.fetch_add(Release)`. That plain store is exactly why voices cannot share a slot.
- **`LevelTap::next`** already clamps each publish with `.min(1.0)` (`analysis.rs:215`), so per-voice readings were bounded before the sum ever existed.
- **wasm's tap point.** `WebAudio::tap(channel, node)` (`audio_wasm.rs:371`) is `node.connect_with_audio_node(&state.node)`. `play_tone_to_metered` calls it after `gain` is connected to `dest`, matching where every other tap sits — after the sound's own gain, before bus/master.
- **`reload_scene`'s persistence mechanism** (`src/app/scenes.rs:11-46`): snapshot registered types out of the old world with `take_resource_erased` under a `filter_map`, rebuild `World::new()` + `insert_core_resources` + metadata + replayed `world_registrars`, re-insert `DebugUi` by hand, then re-insert the snapshot **last** so it beats engine defaults. The `filter_map` is what makes registering an absent type free.
- **`register_persistent<T>`** (`scenes.rs:4-9`) just pushes a `TypeId` if absent — no bound on `T` beyond `'static`, so a type the engine never inserts registers fine.
- **survivor's new constant:** `KILL_PEAK_FULL: f32 = 1.0` — the *summed* ceiling. `drive_from_amplitude` now normalises `(amp - KILL_VOL_BASE) / (KILL_PEAK_FULL - KILL_VOL_BASE)`, floored at `DRIVE_FLOOR = 0.35`.
- **survivor's fire path:** `FIRE_COOLDOWN = 0.14` on a `Timer::repeating`, one bullet and one `play_tone(900.0, 0.04, 0.16)` per `just_finished()`. 0.04 s of sound per 0.14 s — the arithmetic that disproved the parent's bullet-tone claim.

## Reusable Procedures

### Proving a silent-revert bug, in the order that makes the proof real

The subagent's audit was honest that its findings were *reasoned from code paths, not observed* — no shipped example both sets one of these resources and then performs a scene transition. The fix for that is not more reading:

1. Write the regression test **first**, one per suspect, modelled on `window_config_survives_a_scene_reset`: `App::new()` → `world.insert_resource(T { …distinctive values… })` → `app.reload_scene()` → assert the values survived. No window, no GPU, no example needed.
2. **Run them before touching the fix.** A test that has never failed proves nothing; `24 passed; 7 failed` is the evidence. This is #388's "proven non-vacuous by disabling the fix", done in the cheaper order — you never have to remember to disable anything.
3. Only then add the one-line fixes and re-run.

Assertion messages should name the *consequence*, not the field: `"scene reset re-enabled the focus ring a game had turned off"` reads better in a failure than `"assert_eq!(style.enabled, false)"`.

### Measuring whether a named audio channel is cutting itself

Claims like "this fires at most once per frame so it can afford a named channel" are cheap to make and were wrong here. To settle it:

1. Insert a probe immediately **before** the replay: `eprintln!("CUTPROBE still_playing={}", audio.is_channel_playing(channel));`. `is_channel_playing` is exactly "was there still audio queued" — i.e. "is this replay about to cut something".
2. Drive the game headlessly under `ENGINE_INPUT` + `ENGINE_CAPTURE` (capture *diverts* the run — it never also opens a window).
3. **Confirm a device existed**: rodio prints `Dropping DeviceSink, audio playing through this sink will stop` on teardown only if a sink was playing. Without that line the run proved nothing about audio.
4. `grep -c "CUTPROBE"` vs `grep -c "CUTPROBE still_playing=true"` gives the rate directly — here 25 and 22.

### Deriving a second distribution without a second run

`drive_from_amplitude` is a pure function of the metered peak, so once the peaks are captured the drive distribution for **any** candidate normalisation can be computed offline in a few lines of Python. That is how "old top 0.60 → 31% saturated, new top 1.00 → 6% saturated" was obtained from a single game run rather than three.

Generalises: probe the *rawest* value in the chain, then evaluate candidate mappings offline. Probing the derived value locks you into one mapping per run.

### Removing a temporary probe safely (unchanged from seq 1, and still necessary)

`cargo fmt` may reflow a one-line probe onto several lines between insertion and removal, so a deletion regex written against the original text matches nothing **and reports success**. Always finish with an independent `grep -c "eprintln\|<probe tag>" <file>` and require `0`. Used twice this session; both removals were verified this way, not by the removal script's exit code.

### Editing a doc or memory file with guarded replacements

Every prose edit in this session went through a Python block that asserts `s.count(old) == 1` before replacing, and that prints what it changed. Two of them **failed the assertion and were caught**: one because an earlier edit had already rewritten the target sentence, one because the parent's recorded string differed in capitalisation. Both would have been silent no-ops with `sed`. For unversioned files (memory), copy to the scratchpad first — this session backed up `engine-current-state.md` and `MEMORY.md` before editing.

### Stacking a second PR on an unmerged first one

The 2A work had to start before #395 merged, and the 2B docs/version files were in the way. What worked:

```bash
git checkout -b feat/second            # branched from the still-unmerged first branch
# … work, commit …
git fetch origin                       # after PR 1 squash-merges
git rebase --onto origin/main <first-branch-tip-sha> feat/second
```

`--onto` replays **only** the second branch's own commits onto the new `main`, dropping the first branch's commit (whose content `main` already has under a different hash from the squash). A plain `git rebase main` would try to re-apply it and conflict.

One gotcha hit along the way: `git checkout main` with uncommitted changes **carries them across** when they do not conflict, which silently moved in-progress work onto `main`. Harmless here (the branch was recreated from that point), but `git rebase` then refuses with `cannot rebase: You have unstaged changes`.

## Files Changed

### Source code — engine

- `src/app.rs` — seven `register_persistent` calls added to `App::new` after the `WindowConfig` one, with a comment block stating the audit, the line, and the `TimeScale` exclusion. Plus seven regression tests in `mod tests`.
- `src/audio_analysis.rs` — **new public `sum_levels`**, carrying the sum-vs-max decision, the native/wasm asymmetry and the clamp rationale. Plus three unit tests.
- `src/audio/analysis.rs` — `POLY_VOICES`, `poly_voice_channel`, `next_poly_voice`, `PolyVoice`, `AnalysisChannel.poly`, `combine_voices`, `AudioManager::poly_tapped`, and the poly branch in `tick_analysis`. Plus seven unit tests.
- `src/audio/playback.rs` — **new `AudioManager::play_tone_poly`**; `poly_seq: HashMap::new()` added to the constructor.
- `src/audio.rs` — `poly_seq: HashMap<String, u64>` field on `AudioManager`.
- `src/audio_facade.rs` — **new public `Audio::play_tone_metered`**; `enable_analysis` and `play_tone_on_bus` docs corrected (they asserted the trade-off as unconditional).
- `src/audio_wasm.rs` — **new `WebAudio::play_tone_metered`**; `play_tone_to` refactored to delegate to a new `play_tone_to_metered(..., meter: Option<&str>)` so the unmetered path is unchanged.

### Examples (the acceptance test)

- `examples/games/survivor/survivor.rs` — kill tone → `play_tone_metered`; `KILL_CHANNEL` → `KILL_METER`; `play_tone_named` → `play_metered_tone`; **new `KILL_PEAK_FULL`**; `drive_from_amplitude` re-based on it; module docs rewritten with the measured before/after and the transferable lesson.

### Docs

- `docs/PATTERNS.md` — "Surviving a scene reset" substantially rewritten: two new auto-registered rows, the audit subsection with the 20-resource negative result, the `TimeScale` exclusion, the new line, and the `Audio` block rewritten to record that its reopen trigger fired.
- `docs/MODULE_MAP.md` — row 31 (ECS/`register_persistent`) extended with the audit; row 79 (audio-reactive) extended twice — once for the new API and once for the adoption evidence. **No new rows**, per the house rule.
- `docs/CHANGELOG.md` — new 0.139.1, 0.140.0 and 0.141.0 sections.
- `CLAUDE.md` — header v1.6.240 → v1.6.243, package v0.139.0 → v0.141.0.

### Release paperwork

- `Cargo.toml` / `Cargo.lock` — 0.139.0 → 0.139.1 → 0.140.0 → 0.141.0, lock refreshed each time with `cargo update -p skeleton-engine`.

### Memory (not in any PR — not version-controlled)

- `engine-current-state.md` — seqs 213/214/215 prepended; header main hash/version/date updated; trim note annotated with the new size.
- `MEMORY.md` — the engine-current-state hook rewritten, including that the measurement-backed menu item is now spent and that the `Audio` question is live.

### Scratchpad (throwaway, not committed)

- `survivor_feel.ron` — the input script (G / B×3 / hold ArrowRight). **Still deliberately not committed**, same as seq 1; contents reproduced in Quick Start below.
- `BACKUP_engine-current-state.md`, `BACKUP_MEMORY.md` — pre-edit rollback copies.
- `before.log`, `before2.log`, `after.log`, `final.log` — probe output; `before.png`, `before2.png`, `after.png`, `final.png` — captures.

## User Feedback & Preferences (REQUIRED)

- **The session opened with a highly-specified paste prompt** covering the board gate, the exact verify block, the ask, and an explicit "PHASE 1 ENDS IN A QUESTION, NOT CODE" / "Do NOT self-pick". Every instruction was followed as written.
- **The user did NOT pick from the menu — they asked for a combination and a recommendation:** *"위 선택지에 있는 작업들 같이 진행 할 수 있는게 있으면 한번에 작업 하면 좋겠는데 추천 하는 방향 있을까?"* This is a shift in how direction is given: they want the ranking *and* the packaging judgement, not just the ranking.
- **Both recommendations in the follow-up question were taken** — API shape (b), and SUM semantics. **Recommendation now taken 7 of 7 across four sessions.** The framing (what breaks otherwise, what it actually costs) remains load-bearing.
- **The user accepted a recommendation that contradicted the plan they had approved the session before** — shape (b) over the plan's (a) — once the code evidence was laid out. Presenting the disagreement with evidence is welcome, not friction.
- **"Read the exit code from the FILE — the notification has now lied in FOUR straight sessions."** Followed from the first gate launch; five gates, five file reads.
- **"Grep `docs/MODULE_MAP.md` for topics — never read it whole (72 rows, ~89 KB)."** Followed, including in the subagent's prompt.
- **"Do NOT onboard or re-explore the codebase — the handoff has it."** No onboarding; went straight to the gate.
- **"Do NOT reopen the deferred Audio-persistence decision."** Honoured — the behaviour is unchanged. But the trigger genuinely fired, so it is *recorded* and surfaced rather than silently suppressed. The next session should treat this as an explicit question for the user.
- **"Windowed capture is OFF the menu: the game declined it in writing 2026-07-27."** Kept off.
- **Standing (from `CLAUDE.md` / memory):** user-facing reports in **Korean**, everything else (code, docs, subagent prompts, commit messages, PR bodies) in **English**. Subagents always get an **explicit `model`**. Merge authority is **standing-delegated** — squash-merge on green CI without re-confirming; express merge as a direct instruction, never as an `AskUserQuestion` option.

## Where We're Going

1. **Board gate FIRST, every session.** `../dungeon-merchant/docs/engine-wishlist.md` (next free **EW-012**, unmoved since 2026-07-27) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`). Check whether it **moved** with `git log -1 --date=short`, not whether it looks empty. A filed request preempts everything.
2. **If both are empty: ASK — do not self-pick.** But the menu has materially changed, and one item is now a *decision* rather than a build:
   - **Decide the `Audio` auto-persistence question.** Its reopen trigger has fired and is documented. This is the only menu item that is a decision the user owns rather than work the agent can rank.
   - **A real, licence-clean soundtrack for `beat_crawler`** — a mixed track rather than two synthesized tones 6.5× apart; the first case where the low-band detector could genuinely fail. CC0 synthesis recipe in `src/audio/fixtures/README.md`.
   - **A fourth procgen mode** (drunkard's walk) — still the safest and lowest marginal value.
   - **`play_sfx_metered`** — the clip counterpart of what shipped. `play_tone_metered` covers tones only; clips still face the original trade-off, and the redirection would be the same shape (`play_bytes_internal` → `append_decoded` → `tapped`). **Nothing has asked for it**, so it is a menu item, not a decision.
3. **This handoff + its plan land as their own `docs(handoff)` PR**, chain `audio-adoption` seq 2. Bump memory to **seq 216** after it merges — the recorded tip will be one commit stale, as it always is.
4. **Trim `engine-current-state` by ~seq 220** (41.0 KB now, ~2 KB/seq, ~76 KB cap). Procedure and guard assertions are in the parent's Chunk 2.

## Risks & Blockers

- **The `Audio` decision is now load-bearing and unowned.** `docs/PATTERNS.md` says the trigger fired and the argument is thinner. If the next session neither raises it nor closes it, the doc drifts into stating a live question as settled. **This is the top item to surface.**
- **Adopting a polyphonic meter invalidates single-voice normalisation, silently.** It surfaces as a feel regression, not a compile error — this session caught it only because the drive distribution was computed. Any *other* consumer that later adopts `play_tone_metered` must re-check its constants. Currently only `survivor` does.
- **Audio behaviour is still not covered by CI.** The 10 new tests are device-free by design, which is what makes them run — but they test the *combination arithmetic*, not that a real device produces overlapping voices. Only the headless `ENGINE_CAPTURE` run on a machine with a device proves that, and it is manual.
- **`survivor` still has no self-test mode.** Everything verified this session was verified by probes that were then removed. A regression in the metered path would be silent. (`beat_crawler` has `BEAT_CRAWLER_SELFTEST=1`; `survivor` has nothing.)
- **The input script is still not committed**, so reproducing the measurements means re-writing the `.ron`. Contents are in Quick Start.
- **`cargo fmt` can still silently defeat a scripted edit.** Mitigated this session by grepping after every probe removal; the trap itself is unchanged.
- **Stale rust-analyzer diagnostics fired again** (a phantom `E0107` in `beat_crawler.rs`, plus "file not included in any crates" for `audio_wasm.rs`) in code that compiled clean. Trust `cargo` exit codes only.
- **`dungeon-merchant` has no CI or branch protection** — its board is discipline, not enforcement. `rust-survivors` is paused/deprecated but remains a real bug-report channel.

## Open Questions

- **Should `Audio` be auto-persisted?** No longer deferred-and-dormant: its stated reopen trigger has fired, and the fix that fired it weakened the original argument by covering four game-inserted types. What still separates `Audio` is that the engine *drives* it every frame rather than reading it as config, and that it owns an OS device handle rather than a value. **This needs a decision, not another deferral.**
- **Should `play_sfx_metered` exist?** `play_tone_metered` covers tones; clips still face the original exclusivity. The redirection is the same shape. Nothing has asked for it yet.
- **Should `survivor` get a `SURVIVOR_SELFTEST=1` mode?** Carried from seq 1 and now more pointed: the example's behaviour changed twice in two sessions and both times the verification was thrown away. Needs a device, so CI would SKIP (the `beat_crawler` precedent).
- **Should `bands()` ever work for a metered one-shot?** Deliberately zeros today. Reopening means an FFT per voice on the playback thread; the trigger would be an actual use case for a *spectrum* of a one-shot, which nobody has.
- **Should `embedded_image` get a web harness?** Carried unanswered from four prior sessions. Probably time to either do it or close it.
- **Is the `add-facade-capability` skill worth writing?** Still deferred; this session would have been its **n=3** (`play_tone_metered` followed the same facade + native + wasm + policy-module shape).

## Quick Start for Next Session

```bash
# Restore full context
cat plans/handoffs/HANDOFF_audio-adoption_metered-oneshot-and-config-audit_2026-07-31.md

# Nothing is dangling — #395/#396/#397 all merged, main clean at c2e1b82 (v0.141.0).
cd ~/Projects/skeleton-engine
git log --oneline -4      # expect c2e1b82 at the tip (or the handoff merge above it)
git status -s             # expect clean

# 1. READ THE BOARD FIRST — both channels; a filed request preempts everything
#   ../dungeon-merchant/docs/engine-wishlist.md        (next free EW-012, empty since 2026-07-27)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md   (_None._)
cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md && cd -
# ^ later than 2026-07-27 = the board MOVED; read it before anything else

# 2. Verify starting state — the block in docs/VERIFICATION.md.
#    Read the exit code from the FILE. The notification has lied in FIVE straight sessions.
rm -f /tmp/v.exit /tmp/v.log
(./scripts/verify.sh > /tmp/v.log 2>&1; echo $? > /tmp/v.exit) &
until [ -f /tmp/v.exit ]; do sleep 20; done
echo "EXIT=$(cat /tmp/v.exit)"; ls -l /tmp/v.exit
echo "ok groups: $(grep -c 'test result: ok' /tmp/v.log)"
# expect 0 · 152 ok groups · 1356 lib tests   (v0.141.0)

# 3. Key files
#   docs/PATTERNS.md                      — "Surviving a scene reset": the audit + the FIRED Audio trigger
#   src/audio_analysis.rs                 — sum_levels (the un-gated sum-vs-max policy)
#   src/audio/analysis.rs                 — POLY_VOICES, poly_voice_channel, next_poly_voice, combine_voices, poly_tapped
#   src/audio/playback.rs                 — play_tone_poly
#   src/audio_facade.rs                   — Audio::play_tone_metered
#   examples/games/survivor/survivor.rs   — KILL_METER, play_metered_tone, KILL_PEAK_FULL
#   docs/MODULE_MAP.md                    — GREP IT; rows 31 and 79 carry this session

# 4. Reproduce the real-device measurements (needs an audio device; NOT committed)
cat > /tmp/survivor_feel.ron <<'RON'
(events: [(frame:5,action:KeyPress("KeyG")), (frame:10,action:KeyPress("KeyB")),
          (frame:12,action:KeyPress("KeyB")), (frame:14,action:KeyPress("KeyB")),
          (frame:20,action:KeyDown("ArrowRight"))])
RON
ENGINE_INPUT=/tmp/survivor_feel.ron ENGINE_CAPTURE=260:/tmp/s.png cargo run --example survivor_game
#    HUD row 2 must read blue: "combo N.N   kill meter 0.NN  → shake/pulse (audio-driven)"
#    Confirm a device existed: grep -c DeviceSink on stderr must be 1, or the run proved nothing.

# 5. FIRST CONCRETE ACTION
#    Read both board files and state the verdict. If empty, ask ONE AskUserQuestion (Korean).
#    LEAD WITH THE DECISION, not a build item: the Audio auto-persistence question's reopen
#    trigger fired this session and docs/PATTERNS.md records it as live. Then the build menu:
#      beat_crawler real soundtrack / play_sfx_metered / 4th procgen mode.
#    Do NOT self-pick.
```
