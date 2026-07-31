# A deferred decision got decided, a real soundtrack exposed an engine bug and a dead system, and the clip counterpart shipped — four PRs in one session

**Date:** 2026-07-31
**Status:** COMPLETED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `audio-adoption` seq `3`
**Parent:** `HANDOFF_audio-adoption_metered-oneshot-and-config-audit_2026-07-31.md`
**Prior chain:** `HANDOFF_audio-adoption_survivor-audio-reactive-adoption_2026-07-30.md` (seq 1) > `HANDOFF_audio-adoption_metered-oneshot-and-config-audit_2026-07-31.md` (seq 2) > this

---

## Stale References

Identifiers named in the parent (seq 2) or in `docs/MODULE_MAP.md` that no longer exist **in `beat_crawler`**, all removed by PR #401:

- `REARM_THRESHOLD` — **gone from the codebase entirely** (0 files). Replaced by `KICK_COOLDOWN = 0.40`.
- `advance_soundtrack` — **gone** (0 files). Renamed to `advance_schedule`, and it no longer plays anything.
- `Crawler::armed` (the arm/re-arm latch field) — removed.
- `BEAT_CHANNEL`, `KICK_HZ`, `BLIP_HZ`, `KICK_SECS`, `BLIP_SECS`, `BEAT_BUS` — ⚠️ **these still grep-hit, but only in `examples/audio_reactive/audio_reactive.rs`, which has its own identically-named constants.** In `beat_crawler` they are gone; `BEAT_CHANNEL` became `BEAT_METER = Audio::MUSIC_CHANNEL`. A next session grepping these names will land in the wrong file.

Everything else the parent named still exists and is unchanged: `KILL_METER`, `play_metered_tone`, `drive_from_amplitude`, `KILL_PEAK_FULL`, `KILL_PEAK_ON`, `KILL_PEAK_OFF`, `AudioFeel`, `AudioFeelSystem`, `sum_levels`, `combine_voices`, `poly_voice_channel`, `next_poly_voice`, `POLY_VOICES`, `play_tone_poly`, `BEAT_WATCHDOG`, `ON_BEAT_WINDOW`.

**Signature changes to know about (both `pub(super)`/private, no public break):**
- `AudioManager::play_bytes_internal` gained a 5th parameter `meter: Option<(&str, usize)>`.
- `AudioManager::append_decoded` gained a 6th parameter `meter: Option<(&str, usize)>`.
- `WebAudio::play_sfx_to_opts` gained a 4th parameter `meter: Option<&str>`.

## Since Last Handoff

The parent's plan ran as written through Phase 2, then Phase 3 went somewhere the plan did not anticipate.

- **Phase 1 (board gate → ASK) ran exactly as written.** Both channels empty, board unmoved since 2026-07-27 (verified by `git log`, not by eyeballing). One Korean `AskUserQuestion` with two questions.
- **The parent's top-listed risk — "the `Audio` decision is now load-bearing and unowned" — was resolved, not inherited.** The user chose to persist it. `docs/PATTERNS.md` no longer contains a live unanswered question.
- **Phase 2 was mandatory and it happened first**, before any build work, exactly as the plan demanded in three places.
- **The parent's open question "Should `Audio` be auto-persisted?" is ANSWERED — yes, and it shipped** (v0.141.1, PR #399).
- **The parent's open question "Should `play_sfx_metered` exist?" is ANSWERED — yes, and it shipped** (v0.143.0, PR #402), but *not* from the plan's Phase 3B branch: the user asked for it in a second turn after the first four PRs were done.
- **Phase 3A was chosen for exactly the reason the plan gave — "the only remaining item where the engine could genuinely FAIL" — and it failed three times.** One of those failures was an engine bug (metering a loop), one was an arrangement problem, and one was a *pre-existing dead system* in the example that had nothing to do with the soundtrack.
- **A risk the parent listed materialized in a new form.** "Audio work is not covered by CI" was listed as a mitigation-needs-a-real-device note. It bit twice: the loop-meter bug is invisible to CI, and a headless capture *cannot even be used to check it*, which cost two separate investigations.
- **The parent's Phase 4 (engine change as its own PR) fired twice again**, so this session produced four PRs where the plan sketched two.
- **The verify gate went RED once (exit 101)** — the first red gate in three sessions. Caught because the exit code was read from the file.
- **Trajectory: the `audio-adoption` chain has now consumed its entire remaining menu except two items.** Seq 1 found the gap, seq 2 closed it for tones, seq 3 closed it for clips and hardened the metering path underneath both.

## Reference Documents

- `CLAUDE.md` — project conventions, the verify gate, pre-1.0 versioning (MINOR = any release, PATCH = bugfix)
- `docs/MODULE_MAP.md` — 72 rows. **Grep it, never read it whole** (~90 KB). Rows 79 (audio-reactive hooks) and 91 (beat_crawler) both rewritten this session
- `docs/VERIFICATION.md` — the 6 exit-code traps; carries the copy-paste gate block that caught this session's red gate
- `docs/PATTERNS.md` — "Surviving a scene reset", now written as DECIDED rather than pending
- `docs/CHANGELOG.md` — 0.141.1 / 0.141.2 / 0.142.0 / 0.143.0 entries written this session
- `examples/games/beat_crawler/assets/README.md` — **new**, the track's provenance + why PCM not Ogg + the per-layer measurement method

---

## The Goal

Execute the seq-2 plan: run the board gate, and if both channels are empty, **ask** — leading with the `Audio` auto-persistence *decision* (which the user owns) rather than a build item. Then resolve that decision in writing whichever way it went, because `docs/PATTERNS.md` was in the unacceptable state of documenting a fired reopen trigger with no resolution. Only then build one menu item.

The plan's recommended build item was `beat_crawler`'s real soundtrack, justified as **the only remaining item where the engine could genuinely fail and we would learn something** — the example was discriminating a 110 Hz kick from an 880 Hz blip at 4.00 vs 0.61, 6.5× apart, which is a test that cannot fail.

That justification turned out to be exactly right, three times over. After the four PRs landed, the user opened a second turn asking for `play_sfx_metered` — the plan's 3B branch, which I had flagged in my closing report as having risen in priority precisely because its tap sits in `append_decoded`, the function the loop-meter bug had just been found in.

## Where We Are

- **Everything is landed and merged. Tree clean, `main` @ `a00bad0`, package v0.143.0, CLAUDE.md header v1.6.247.** Nothing in flight, no open branches, no stashes.
- **Four PRs, all squash-merged, all 6/6 CI green:** #399 (04:01:14Z), #400 (04:33:59Z), #401 (04:42:05Z), #402 (06:33:33Z).
- **Lib tests 1356 → 1360** (+1 in #399, +1 in #400, 0 in #401, +2 in #402). `ok` groups 152 throughout, every run.
- **The verify gate ran 6 times, 5 green and 1 RED (exit 101).** Every exit code read from the file, never from a notification.
- **#399 / v0.141.1 — `Audio` is auto-persisted across a scene reset.** One line in `App::new`, one regression test, and the hand-rolled `register_persistent::<Audio>()` calls deleted from `settings_menu` and `beat_crawler`.
- **The test asserts the REGISTRATION, not a surviving instance.** `Audio::new()` opens a real output device and returns `None` on every CI runner, so an instance test is impossible; the plan ruled `#[ignore]` out as non-evidence. It was **confirmed to fail without the one-line fix** before shipping.
- **Blast radius was measured and stated in the PR, not assumed:** of the 10 examples calling `Audio::new()`, only `settings_menu` (8 scene ops) and `beat_crawler` (1) ever change scenes, and **both already registered by hand** — so zero examples changed behaviour.
- **#400 / v0.141.2 — metering a LOOPING sound went dead after its first pass.** A real engine bug, found by giving `beat_crawler` a looping track.
- **The audio kept playing the whole time.** `is_channel_playing` stayed `true` for the full 8 s while `levels()`/`bands()` read exactly `0.0000` from 2.6 s onward.
- **Cause confirmed against rodio's own source, not inferred:** `repeat_infinite()` calls `input.buffered()` and clones that buffer per pass, so anything *inside* it is polled exactly once. The analysis tap was inside.
- **Fix: `append_decoded` repeats first and taps second.** Tap now sits outside the buffer, still before pan and sink volume, so the documented pre-volume `AudioLevels` semantics are unchanged.
- **A second bug fell out of the same reorder:** `fade_in` was also baked into the buffer, so `crossfade_music` re-faded the track in at the top of **every loop** instead of once. Pan and fade now both sit outside the repeat.
- **#401 / v0.142.0 — `beat_crawler` runs its turn clock off a real mixed track.** `assets/soundtrack.wav` (2.560 s, 56,448 frames, 112,940 bytes) + `assets/soundtrack.py` that synthesizes it, CC0, plus `assets/README.md`.
- **PCM, not Ogg, deliberately** — the loop must be sample-exact, and a lossy encoder pads the stream, which `repeat_infinite` would then replay every pass, drifting the bar against the written schedule.
- **The first arrangement made the kick undetectable and NO threshold worked.** With the bass on C2/F2/G2 at gain 0.30, every low band sat pinned near full scale; a saturated band cannot show a transient. Fixed by moving the bass up an octave (C3/F3/G3) at gain 0.10 — what a real mix does for the same reason.
- **`LOW_BANDS` went 4 → 2**, established by measuring each layer in isolation (`soundtrack.py --only kick|bass|hat|lead`), not by reasoning: the kick owns bands 0–1 (which share FFT bins at this resolution and move together), the bass saturates 2–6.
- **`KICK_THRESHOLD` 1.2 → 1.6**, chosen mid-plateau: 1.45–1.95 all produce the correct count (28) and the correct spacing (0.640 s, sd ≈0.03) over 7 bars.
- **Arm/re-arm was replaced by a retrigger COOLDOWN (0.40 s) — the second independent confirmation of seq 1's `survivor` finding**, on a completely different signal class (a sustained musical mix rather than a kill stream).
- **⚠️ `AudioFacadeSystem` had never been running in `beat_crawler`.** It was registered on the `App` *before* `set_scene`, and `SceneCmd::Replace` swaps out the entire systems list — so the tick that feeds `Audio::update` was silently dropped, `bands()` returned `0.000` forever, and the game ran permanently on `BEAT_WATCHDOG`'s schedule fallback.
- **That bug PREDATES the soundtrack.** The example's headline feature — "the turn clock is the music" — was not actually running in the playable game, and the only symptom was the HUD reading "schedule (nothing heard)" instead of "listening". Now registered in `CrawlScene::on_enter` so the ordering is unforgeable.
- **Checked the other five `AudioFacadeSystem` examples — none of them use scenes**, so `beat_crawler` was the only one affected.
- **#402 / v0.143.0 — `Audio::play_sfx_metered`, the clip counterpart of `play_tone_metered`.** Came in exactly at the branch's stated stop condition: one facade method, one manager method (`play_sfx_poly`), one threaded parameter.
- **The one real difference from the tone path is where the tap goes in.** A tone is a `SamplesBuffer` `play_tone_poly` wraps itself; a clip must be decoded, and the decode → effects → repeat → tap → pan chain lives in `append_decoded`. The voice threads down as `meter: Option<(&str, usize)>` rather than being applied at the top, keeping one chain instead of forking it.
- **wasm needed no voice pool again, and this was VERIFIED not assumed** (the plan explicitly said to check, since `play_sfx_to_opts` returns an `Sfx` handle the tone path lacks): `play_sfx_to` already builds a fresh per-source gain node per call, so it is one `tap(meter, &gain)`. The `Sfx` handle is deliberately dropped.
- **⚠️ A headless `ENGINE_CAPTURE` run cannot photograph an audio meter.** Confirmed from `src/app/headless.rs:114` (`let dt = 1.0 / 60.0;`) — capture advances game time with a fixed dt as fast as the CPU allows while the audio thread publishes in real time, so the smoothing release drains in milliseconds of wall clock. Cost two separate investigations before being written down.
- **Memory advanced to seq 220** (`engine-current-state.md` 36,734 → ~48 KB region, `MEMORY.md` hook rewritten twice). **Trim is now OVERDUE** — the parent said "~seq 220" and we are at 220.

## What We Tried (Chronological)

### Early — the board gate and the decision (PRs #399)

1. **Board gate, both channels, read-only.** `awk` on the 53 KB wishlist rather than `cat`. Result: `*(none — all filed requests are closed; next free ID EW-012)*` and `_None._`. `git log -1 --date=short` on the wishlist returned **2026-07-27** — not later than the gate condition, so the board had not moved. No preemption.
2. **Baseline verify gate.** Exit **0** read from `/tmp/v.exit` (mtime 12:30), **152 ok groups**, **1356 lib tests** — matching the parent's recorded expectation exactly.
3. **Asked one `AskUserQuestion` with two questions, in Korean, leading with the decision.** Q1 = the `Audio` persistence decision, Q2 = the build menu. Carried the reasoning, and recommended a *combination* (Q1 persist + Q2 soundtrack) because the parent recorded that the user had asked for the packaging judgement, not just the ranking.
4. **The recommendation for Q1 rested on the doc's own words.** `docs/PATTERNS.md`'s closing rule already said *"session state (config, device handles, caches, cross-scene progress — must be registered)"* — a device handle is the textbook case. The audit's narrower line ("engine defines it and only reads it") was the only thing excluding `Audio`, and that line was a description of what had been audited, not a principle.
5. **Both recommendations taken.** Recommendation now taken **9 of 9**.
6. **Wrote the regression test, then deliberately broke the fix to watch it fail.** Commented out `app.register_persistent::<crate::audio_facade::Audio>();`, re-ran, got the assertion message verbatim, restored. This is seq 2's reusable procedure applied in the cheap order.
7. **Measured the blast radius before claiming zero.** Grepped all 10 examples calling `Audio::new()` and counted scene operations per file, rather than asserting "nothing changes".

### Mid — the soundtrack, and the engine bug it exposed (PR #400)

8. **Wrote `soundtrack.py`: kick + sustained bass + hats + lead, 16 steps × 0.16 s.** Deliberately put the bass *in* the detector's window so the test would be real. First render: peak 1.216 → gain 0.699, 112,940 bytes.
9. **Chose `play_music` + `Audio::MUSIC_CHANNEL` after checking the facade surface**, because there is no "play bytes on a named channel, non-positional, looping" method. `audio_wasm.rs:217` documents `MUSIC_CHANNEL` as the meterable music name on both backends, so this is the supported path — not an abuse of `play_at_on_channel`.
10. **First probe returned essentially zero low-band energy** (max 0.009 across 4 warm loops). Did **not** start tuning the detector — instrumented instead.
11. **Printed `levels()` + the full 16-band array per frame.** That showed a rich spectrum at t=0.00 decaying to exactly 0 by t≈2.6 s — i.e. one loop of a 2.56 s track. The signal was not weak; it was *stopping*.
12. **Formed the hypothesis that `repeat_infinite` buffers the tap away, and checked rodio's source rather than assuming.** `~/.cargo/registry/.../rodio-0.22.2/src/source/repeat.rs` confirmed `let input = input.buffered(); Repeat { inner: input.clone(), next: input }`.
13. **Ran the control that distinguishes "meter broken" from "playback broken":** printed `is_channel_playing` alongside the meter for 8 s. `playing=true` on every sample while `rms=0.0000` — proving the audio was looping audibly and only the meter was dead.
14. **Fixed by reordering, not by special-casing.** Repeat moved before the tap; the pan/fade tail collapsed from a 4-branch `if/else` tree into a linear `Option` match.
15. **Wrote the regression test to assert BOTH orders** — the fixed one publishes once per pass, and the old one publishes only the first pass, kept deliberately as a tripwire if rodio ever stops buffering `Repeat`.
16. **Re-ran the same 8 s probe after the fix:** `rms` 0.08–0.38 sustained through every pass, low-band 1.68–4.00. The meter no longer dies.

### Late-mid — the mix, the clock trap, and the dead system (PR #401)

17. **With metering fixed, the folded per-phase plot came out FLAT** — mean ≈3.0 at every one of 32 phase buckets, kicks invisible. Two separate causes, both real.
18. **Cause A: the bass was saturating the low bands.** Verified by rendering layers in isolation (`--only kick`, `--only bass`) and comparing per-band percentiles. Kick bands 0–3: p50 0.346, p95 1.000. Bass bands 0–1: p50 0.688; bands 2–4: p50 0.889–1.000, i.e. pinned.
19. **Rejected "just lower the bass gain" after doing the dB arithmetic.** `MIN_DB`/`MAX_DB` are −100/−30, a 70 dB window; getting the bass bands from ~0.76 down to ~0.30 needs about −32 dB, a factor of ~40 in amplitude, which would make the bass inaudible. The problem was *frequency placement*, not level.
20. **Moved the bass up an octave (C2/F2/G2 → C3/F3/G3) at gain 0.30 → 0.10.** Re-measured: bass bands 0–1 p50 0.408 (was 0.688), its energy now peaking in bands 5–6 where it belongs.
21. **Cause B — and this is the one that nearly produced a wrong conclusion: the measurement clock was wrong.** The probe paced itself with `t += 1.0/60.0`. `sleep` sleeps *at least* that long, so the accumulator ran slower than the music and every measured gap came out ~28% short.
22. **The tell was a suspiciously regular result that did not vary with the parameter.** Thresholds 1.30 / 1.50 / 1.70 and rise 0.20 / 0.35 all produced *identical* output: 39 fires, gap mean 0.458, sd 0.025. A detector whose output ignores its own threshold is measuring something else.
23. **Rewrote the probe to pace off `Instant` and timestamp off `Instant`.** The same detector then produced 28 fires at a 0.640 s mean gap — the ground truth. A correct detector had looked like it was over-firing by 40%.
24. **Swept three retrigger strategies over 7 warm bars** — arm/re-arm, cooldown, and cooldown+rise (spectral flux) — scoring by fire count, gap mean, gap sd, and fraction of gaps on the 0.640 s grid.
25. **Cooldown won outright and flux added nothing**, so the simplest correct detector shipped. Mapped the threshold plateau (1.35 → 1.95) to confirm 1.6 is not knife-edge.
26. **Headless capture of the game showed `low-band 0.00` and "schedule (nothing heard)".** Instrumented `detect_beat` behind `BC_DEBUG` rather than guessing: every band was exactly `0.0` on all 1201 frames while `is_channel_playing` was `true`.
27. **Traced it to system registration, not to audio.** `src/app/scenes.rs`'s own comment says *"Replace swaps out the entire systems list"* — and `beat_crawler` registered `AudioFacadeSystem` on the line *before* `set_scene`. `ch.bands` is only ever written inside the `update` path (`analysis.rs:530`), so with no tick the bands stay zero forever.
28. **Fixed structurally rather than by reordering two lines.** Moved the registration into `CrawlScene::on_enter`, so the system that feeds the turn clock is owned by the scene whose turn clock it is.
29. **Re-ran the instrumented capture: max low-band 2.00, and 9 of 1201 frames above `KICK_THRESHOLD`** — the game's own code path now reads a live meter that crosses the threshold.
30. **Attempted a windowed real-time playtest and abandoned it after two attempts.** `screencapture` returned an all-black PNG (screen-recording permission), and `osascript` could not find the window. Did not keep retrying — reported the limitation instead.
31. **Extended `BEAT_CRAWLER_SELFTEST` to assert the real question.** It no longer measures two tones; it runs the game's own detector against the mix and asserts count + spacing. Result: 16 kicks / 4 bars, mean gap 0.638 s vs a 0.640 s grid, 15/15 on-grid.

### Late — the clip counterpart (PR #402)

32. **User asked for `play_sfx_metered` directly** in a second turn, immediately after the closing report flagged it as having risen in priority.
33. **Read the tone path first to find the real difference**, per the plan's warning that "the tap redirection must reach `append_decoded`". Confirmed: `play_tone_poly` builds its own `SamplesBuffer` and calls `poly_tapped` inline, which a clip cannot do.
34. **Enumerated the byte path's call sites before touching anything** — `play_internal`, `play_bytes`, `play_bytes_internal`, `crossfade_bytes`, and `positional::play_bytes_at` (which funnels through `play_bytes`). Five sites, one shared chain, so threading a parameter was cheaper than forking.
35. **Rejected parsing the `__poly_` channel name inside `tapped()`** as a way to avoid the parameter — stringly-typed and fragile.
36. **Verified the wasm claim instead of inheriting it.** `play_sfx_to_opts` builds `gain`/`panner` synchronously and routes `source → panner → gain → dest`, so `tap(meter, &gain)` sits exactly where the tone path taps. Also handled the degraded path where node creation fails and there is nothing to tap.
37. **Wrote a real-device probe with a CONTROL, not just a result.** One clip, three clips 60 ms apart, and three clips 600 ms apart. The third is what proves per-voice staleness rather than accumulation.
38. **Added the `4` key to `examples/audio_facade`** and drove it headlessly via `ENGINE_INPUT` to confirm the key path fires (status line + legend rendered correctly in the capture).
39. **The example's meter readout showed 0.00 in the capture — and this time the cause was already known.** Rather than assume, confirmed it from `src/app/headless.rs:114`: `let dt = 1.0 / 60.0;`. Fixed dt + as-fast-as-possible frames means game time outruns real audio and the release drains between publishes.
40. **Wrote that trap into `Audio::levels`'s docs and module-map row 79**, because it had now cost two separate investigations in one session.
41. **The verify gate went RED (exit 101).** A `pub` method's doc linked `pub(crate)` `POLY_VOICES`, tripping `-D rustdoc::private_intra_doc_links`. Replaced the link with prose, re-ran the doc build alone to confirm, then re-ran the full gate.

## Key Decisions

- **Led the ask with the decision, not the build menu.** The parent's plan said to, and the reason held: Q1 was not "which should we build" but "a shipped doc states an unresolved question and it needs an owner". A build item can wait a session; a doc drifting into presenting a live question as settled cannot.
- **Recommended persisting `Audio` on the strength of the doc's own closing rule**, not on the audit's narrower line. The audit's line ("the engine defines the type and only reads it") is a *description of what was audited*; the general rule it sits under already names device handles as session state. Saying which of the two is the principle was the substance of the recommendation.
- **Asserted on the registration rather than on a surviving `Audio` instance.** The plan offered three fallbacks and ruled out `#[ignore]`. Chose the `TypeId`-in-`persistent_resources` assertion because it fails without the fix (verified) and the preservation mechanism it opts into is already covered by the eight round-trip tests beside it.
- **Fixed the loop-meter bug by reordering the chain, not by special-casing `repeat`.** Putting the tap outside the buffer preserves the documented pre-volume semantics exactly, simplifies the pan/fade tail, and incidentally corrects the re-fading loop. A "if repeat, also tap the outer source" special case would have left both the fade bug and a second tap path.
- **Kept the buggy composition order as an assertion in the regression test.** If rodio ever stops buffering `Repeat`, that assertion fires and tells the next reader the `append_decoded` comment is stale — cheaper than the comment silently rotting.
- **Treated the flat spectrum as two bugs, not one.** It would have been easy to tune the threshold until something worked; the arrangement problem and the clock problem had to be separated before any constant meant anything.
- **Moved the bass rather than lowering it, after doing the dB arithmetic.** −32 dB would have been needed to unsaturate the bands, which makes the bass inaudible. That is the difference between fixing a mix and deleting a layer.
- **Refused to over-tune the track until the test passed.** The stated goal was a mix where the low band is *shared but separable*, not one where the kick is trivially isolated. The bass still overlaps the detector's window through its harmonics, and that is recorded in `assets/README.md` as deliberate.
- **Chose the simplest correct detector.** Cooldown+rise (spectral flux) scored identically to plain cooldown at every working threshold, so the flux term was dropped rather than kept "for robustness".
- **Fixed the `AudioFacadeSystem` ordering structurally.** Moving the call two lines down would have worked and would have been re-breakable by anyone reordering `main`. Registering it in `CrawlScene::on_enter` makes the system belong to the scene whose clock it feeds.
- **Reported the dead system as predating the soundtrack** rather than folding it into the feature narrative. It means the example's headline claim was false for several releases, and that is worth saying plainly.
- **Split the work into four PRs**, per the #388 precedent: the `Audio` persistence fix, the engine loop-meter fix, the example, and the clip API each stand or revert independently.
- **Threaded `meter: Option<(&str, usize)>` down to `append_decoded` rather than forking a second decode path.** Five call sites gained a `None`; the alternative duplicated the decode → effects → repeat → tap → pan chain.
- **Dropped the wasm `Sfx` handle in `play_sfx_metered` deliberately.** A metered one-shot is fire-and-forget by definition, and returning the handle would imply a per-sound control the native ring cannot offer.
- **Documented the headless-capture trap on `Audio::levels` rather than in a session note.** It is a property of the API's observability, and the next person to hit it will be reading that doc.

## Evidence & Data

### The four PRs

| PR | Version | Merged (UTC) | Files | +/− | Lib tests | What |
|---|---|---|---:|---|---:|---|
| #399 | v0.141.1 | 04:01:14 | 9 | +98/−44 | 1356 → 1357 | `Audio` auto-persisted across a scene reset |
| #400 | v0.141.2 | 04:33:59 | 7 | +90/−33 | 1357 → 1358 | Metering survives every pass of a looped sound |
| #401 | v0.142.0 | 04:42:05 | 9 | +358/−100 | 1358 | `beat_crawler` on a real mixed track |
| #402 | v0.143.0 | 06:33:33 | 10 | +241/−24 | 1358 → 1360 | `play_sfx_metered`, the clip counterpart |

### Verify-gate history — every exit code read from the FILE

| # | When | Exit file mtime | Exit | ok groups | lib tests | Notes |
|---|---|---|---|---:|---:|---|
| 1 | Session start | 12:30 | **0** | 152 | 1356 | Matches the parent's recorded expectation exactly |
| 2 | Post-#399 (+ bump) | 12:51 | **0** | 152 | 1357 | +1 registration test |
| 3 | Post-#400 (+ bump) | 13:27 | **0** | 152 | 1358 | +1 tap-order test |
| 4 | Post-#401 (+ bump) | 13:34 | **0** | 152 | 1358 | Example only, no new lib tests |
| 5 | Post-#402 (+ bump) | 15:20 | **101** | 152 | 1360 (running) | 🔴 **RED** — private intra-doc link |
| 6 | Post-#402 doc fix | 15:24 | **0** | 152 | 1360 | +2 poly-ring tests |

**The red gate is the headline here.** `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` failed with:

```
error: public documentation for `play_sfx_poly` links to private item `super::analysis::POLY_VOICES`
   --> src/audio/playback.rs:274:26
    = note: `-D rustdoc::private-intra-doc-links` implied by `-D warnings`
```

Fmt, clippy, wasm, and all 1360 tests were green — only the doc step failed. **A `| tail` on the gate would have shown a wall of passing tests and hidden this.**

### The loop-meter bug — before and after, real device, same 8 s run

| t (s) | BEFORE: playing | BEFORE: rms | BEFORE: low4 | AFTER: low4 |
|---:|---|---:|---:|---:|
| 0.0 | true | 0.4073 | 3.744 | — (analyser filling) |
| 0.5 | true | 0.3319 | 3.430 | 2.809 |
| 1.0 | true | 0.1850 | 3.556 | 3.556 |
| 1.5 | true | 0.1862 | 2.984 | 3.162 |
| 2.0 | true | 0.0203 | 0.548 | 2.560 |
| 2.5 | true | 0.0006 | 0.016 | 3.167 |
| 3.0 | true | **0.0000** | **0.000** | 2.556 |
| 5.5 | true | **0.0000** | **0.000** | 4.000 |
| 8.0 | true | **0.0000** | **0.000** | 3.233 |

`playing=true` in **every** row of both columns. The track loops every 2.56 s; before the fix the meter dies exactly at the end of pass one.

### rodio's `Repeat` — the confirming source

```rust
pub fn repeat<I>(input: I) -> Repeat<I> {
    let input = input.buffered();
    Repeat { inner: input.clone(), next: input }
}
```

`Buffered` caches decoded samples; each pass clones the buffer and replays the cache. Anything wrapped *inside* is polled once.

### Per-layer band profiles — how `LOW_BANDS = 2` was established

**Kick alone** (bands 0–3 identical; they share FFT bins at this resolution):

| band | p25 | p50 | p75 | p95 | max |
|---:|---:|---:|---:|---:|---:|
| 0–3 | 0.152 | 0.346 | 0.702 | 1.000 | 1.000 |
| 5 | 0.115 | 0.256 | 0.583 | 0.889 | 1.000 |
| 8 | 0.073 | 0.165 | 0.336 | 0.646 | 0.816 |

**Bass alone, FIRST arrangement (C2/F2/G2 @ 0.30) — the failure:**

| band | p25 | p50 | p75 | p95 |
|---:|---:|---:|---:|---:|
| 0–1 | 0.627 | 0.688 | 0.757 | 0.877 |
| 2 | 0.781 | 0.889 | **1.000** | **1.000** |
| 3–4 | 0.889 | **1.000** | **1.000** | **1.000** |

**Bass alone, SECOND arrangement (C3/F3/G3 @ 0.10) — shipped:**

| band | p25 | p50 | p75 | p95 |
|---:|---:|---:|---:|---:|
| 0–1 | 0.338 | **0.408** | 0.521 | 0.889 |
| 4 | 0.601 | 0.747 | 0.882 | 0.907 |
| 5–6 | 0.766–0.820 | 0.866–0.889 | 0.935 | 1.000 |

The bass's energy moved off the kick's bands and up to 5–6. Band 0–1 p50 dropped 0.688 → 0.408.

### Full mix, `LOW_BANDS = 2`, real clock, 7 warm bars

```
min 0.17  p10 0.34  p25 0.53  p50 0.83  p75 1.39  p90 1.78  max 2.00
```

Ground truth: 4 kicks per 2.56 s bar = **28 kicks** at **0.640 s** spacing.

### Retrigger strategy comparison — the decisive table

| Strategy | Params | Fires (28 expected) | Gap mean | Gap sd | On-grid |
|---|---|---:|---:|---:|---|
| **arm/re-arm** (shipped before) | 1.20 / 0.50 | 31 | 0.592 | 0.162 | 15/30 |
| arm/re-arm | 1.40 / 1.00 | 38 | 0.480 | 0.167 | 18/37 |
| arm/re-arm | 1.60 / 1.20 | 31 | 0.576 | 0.155 | 27/30 |
| arm/re-arm | 1.80 / 1.40 | 33 | 0.540 | 0.188 | 26/32 |
| cooldown | 1.30 / 0.40 | 31 | 0.592 | 0.105 | 17/30 |
| **cooldown (shipped)** | **1.50 / 0.40** | **28** | **0.640** | **0.030** | **27/27** |
| cooldown | 1.70 / 0.40 | 28 | 0.640 | 0.028 | 27/27 |
| cooldown | 1.70 / 0.50 | 28 | 0.640 | 0.028 | 27/27 |
| cooldown + rise | 1.50 / 0.35 / 0.40 | 28 | 0.640 | 0.030 | 27/27 |

**Arm/re-arm never gets it right at any setting.** Cooldown+rise never beats plain cooldown, so the flux term was dropped.

### Threshold plateau map (cooldown 0.40 s) — proving 1.6 is not knife-edge

| Threshold | Fires | Gap mean | Gap sd | On-grid |
|---:|---:|---:|---:|---|
| 1.35 | 30 | 0.611 | 0.096 | 19/29 |
| 1.40 | 29 | 0.633 | 0.070 | 23/28 |
| **1.45** | **28** | 0.639 | 0.028 | 27/27 |
| 1.50 | 28 | 0.639 | 0.028 | 27/27 |
| **1.60 (shipped)** | **28** | **0.639** | **0.025** | **27/27** |
| 1.70 | 28 | 0.639 | 0.025 | 27/27 |
| 1.80 | 28 | 0.639 | 0.036 | 27/27 |
| **1.95** | **28** | 0.638 | 0.044 | 27/27 |

Plateau spans **1.45–1.95**; 2.00 is the saturation ceiling (2 bands × 1.0).

### The broken-clock measurement — what a wrong time base looks like

With `t += 1.0/60.0` pacing (WRONG):

| Threshold | Rise | Fires | Gap mean | Gap sd |
|---:|---:|---:|---:|---:|
| 1.30 | — | 39 | 0.458 | 0.025 |
| 1.50 | — | 39 | 0.458 | 0.025 |
| 1.70 | — | 39 | 0.458 | 0.025 |
| 1.30 | 0.20 | 39 | 0.458 | 0.025 |
| 1.50 | 0.35 | 39 | 0.458 | 0.025 |

**Identical output across every parameter** — that is the signature of a measurement that is not measuring what it claims. Real gap is 0.640 s; the accumulator reported 0.458 s, i.e. ~28% short, making a correct detector look 40% over-firing.

### `beat_crawler` in-game meter — before and after the `AudioFacadeSystem` fix

| | Frames sampled | Max low-band | Frames ≥ 1.6 | `is_channel_playing` |
|---|---:|---:|---:|---|
| Before (registered pre-`set_scene`) | 1201 | **0.000** | 0 | true |
| After (registered in `on_enter`) | 1201 | **2.00** | 9 | true |

Every band read exactly `0.0` before the fix, not "low" — the tell that nothing was ticking rather than that the signal was weak.

### `BEAT_CRAWLER_SELFTEST=1` — final, real device

```
depth 1 (27 cells to the stair) ok      depth 4 (15 cells to the stair) ok
depth 2 (16 cells to the stair) ok      depth 5 (37 cells to the stair) ok
depth 3 (27 cells to the stair) ok      depth 6 (25 cells to the stair) ok
pathing approaches: 7 -> 5 ok
kicks heard 16 (expected ~16), mean gap 0.638s (grid 0.640s), on-grid 15/15
PASS: levels solvable, enemies approach, 16 kicks found in a real mix at 0.638s spacing with threshold 1.60
```
Exit **0**.

### `play_sfx_metered` — real-device proof, with a control

| Case | Peak | Reads as |
|---|---:|---|
| Silent, before any play | 0.0000 | meter clean |
| 1 metered clip | **0.4929** | the tap redirection reaches the byte path |
| 3 clips, 60 ms apart | **0.9259** (**1.88×**) | they sum ⇒ they overlap, nothing cut |
| 3 clips, 600 ms apart | **0.4929** | identical to one voice ⇒ per-voice staleness works |

The 600 ms control is as important as the 60 ms result: without it, 1.88× could have been an accumulating counter rather than a sum of *sounding* voices.

### The track

| Property | Value |
|---|---|
| Length | 2.560 s (56,448 frames @ 22,050 Hz mono 16-bit) |
| Size | 112,940 bytes |
| Steps | 16 × 0.16 s; kicks on 0, 4, 8, 12 |
| First render | peak 1.216 → normalised gain 0.699 |
| Shipped render | peak 1.051 → normalised gain 0.809 |
| Layers | kick (150→48 Hz sweep), bass C3/C3/F3/G3 @ 0.10, hats (seeded noise, first-differenced), lead 440–880 Hz |

### Headless capture timing — why a meter cannot be photographed

- `src/app/headless.rs:114` — `let dt = 1.0 / 60.0;`, then `for frame in 0..=last { self.update(dt); ... }` as fast as the CPU allows.
- Measured: **2400 frames in 1.93 s wall clock** (including GPU init) ≈ 2000 fps.
- So 2400 frames = **40 s of game time in ~1.2 s of real time**.
- The analysis release is 0.15 s of *game* time ≈ 9 frames ≈ **4.5 ms of wall clock**, while the audio thread publishes every **~21 ms**. The meter is always decayed when the frame is photographed.
- Consequence: `beat_crawler` captures read "schedule (nothing heard)" and `audio_facade` captures read `impact meter: 0.00` **even though both are working correctly**.

### Board gate

| Channel | Open requests | Next free ID | Last modified | Moved? |
|---|---|---|---|---|
| `../dungeon-merchant/docs/engine-wishlist.md` | `*(none)*` | **EW-012** | 2026-07-27 (`539b610`) | **No** |
| `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` | `_None._` | n/a | — | No (paused) |

### The two questions asked, verbatim (kept for calibration — the parent did the same)

**Q1 header: `Audio 지속성`.** *"`Audio` 리소스를 엔진이 자동으로 persist 하도록 바꿀까요? (seq 213에서 재검토 트리거가 실제로 발화했습니다 — 7개 config 리소스가 씬 리셋 때 조용히 되돌아가고 있었고, 그 중 4개가 '게임이 insert한' 타입이었습니다. 바로 그 '게임이 넣었으니 게임 책임'이라는 논리가 Audio를 보류시켰던 근거였습니다.)"*

| Option | Framing given |
|---|---|
| **자동 persist 한다 (추천)** ← chosen | `App::new` 한 줄 + 회귀 테스트, 두 예제의 수동 호출 삭제. **추천 근거: `PATTERNS.md`의 최종 규칙이 이미 "device handle = session state, 반드시 등록"이라고 스스로 분류**하고 있고, `Audio`를 제외하는 건 감사가 그은 좁은 선뿐. 실패 모드는 무증상+치명적(`beat_crawler`는 턴 클럭이 `bands()`라 게임이 아예 멈춤). PATCH. |
| 게임 쪽 유지 — 문서를 '결정됨'으로 | 코드 변경 없음. `PATTERNS.md`를 '보류'가 아니라 '결정됨'으로 재작성: 트리거 발화 사실, 남은 구분(엔진이 매 프레임 *구동*, 값이 아니라 OS 디바이스 핸들 소유), 다음 재검토 조건. docs-only PR. |

**Q2 header: `다음 작업`.** Four options; the recommended one was chosen.

| Option | Framing given |
|---|---|
| **beat_crawler 실사운드트랙 (추천)** ← chosen | 지금은 kick 4.00 vs blip 0.61 — 6.5배 차이나는 합성 톤 둘. **즉 실패할 수 없는 테스트.** 실제 믹스야말로 `bands()`가 존재하는 이유이고, 저주파 검출기가 진짜로 틀릴 수 있는 첫 사례. 라이선스는 합성(CC0 레시피). 예상 난관: 밀도 높은 트랙에서 저역이 `REARM_THRESHOLD` 아래로 안 떨어져 arm/re-arm이 깨질 수 있음 — seq 1의 쿨다운 발견이 유력한 해법. |
| play_sfx_metered (클립 판) | 톤만 커버됐고 클립은 여전히 배타성 문제. 리다이렉션 모양은 동일하나 탭 지점이 `append_decoded` 안쪽이라 한 단계 깊음 → 정지 조건 부착. **3A보다 낮은 이유: 아무도 요청한 적 없고 대칭성만이 동기.** |
| 4번째 procgen 모드 | 세 번 검증된 패턴, 공짜 조합. 가장 안전하지만 배우는 게 가장 적음. |
| SURVIVOR_SELFTEST | 두 세션 연속 동작이 바뀌었는데 검증 수단을 두 번 다 지웠음. 순위가 낮은 이유는 새 기능이 없다는 것뿐. |

The user's second turn was a plain instruction with no question asked: **"play_sfx_metered 진행해줘"** — the option ranked *second* in Q2, chosen only after the first turn's closing report argued its priority had risen.

### What `docs/PATTERNS.md` now says (the decision record)

The "Known rough edge, still not fixed — but its reopen trigger has now fired" block was **replaced** by:

> **Decided 2026-07-31 (v0.141.1), after the trigger fired.** `Audio` had been left game-side on the argument that it is inserted *by the game* […] One of that deferral's stated triggers […] fired in the v0.139.1 audit above: **seven** did, and **four of the seven are game-inserted**. So *who inserts it* was never the distinction […]
>
> `Audio` is the one entry that line does not reach cleanly, because `AudioFacadeSystem` *drives* it every frame rather than reading it as config. It is registered anyway, on the rule stated below: it owns an **OS device handle**, which is session state by definition. […]
>
> **What would reopen this:** a game that genuinely wants its audio device torn down and rebuilt per scene. None has; both engine examples that use `Audio` across scenes were already hand-rolling the registration, which is what settled it. If one appears, the answer is an opt-out (`App` builder flag) rather than reverting to the footgun.

Also added to the auto-persisted table: `| Audio | an OS output device handle, not a value — losing it kills audio silently (v0.141.1; see below) |`, and the audit's line gained the sentence **"A type the *game* defines is still the game's job; that is what keeps the mechanism meaningful."**

### Anti-goals from the seq-2 plan — all honoured, and how

| Anti-goal | Status |
|---|---|
| Do NOT self-pick the next feature | Held — Phase 1 ended in a question, both answers recorded |
| Do NOT silently re-defer the `Audio` question, and do NOT unilaterally change its behaviour | Held — asked first, then implemented the answer |
| Do NOT re-run the `insert_core_resources` audit | Held — never opened; the 27-resource classification was read, not recomputed |
| Do NOT add `TimeScale` to the persistent set | Held — untouched |
| Do NOT migrate `survivor`'s bullet tone | Held — `survivor` not touched at all this session |
| Do NOT make `bands()` work for a metered one-shot | Held — still zeros, and `play_sfx_metered` inherits that limit explicitly |
| Do NOT implement windowed frame capture | Held — attempted a *playtest* screenshot (different thing), abandoned after 2 tries per the rabbit-hole rule |
| Do NOT add the `MapGenerator` trait | Held — untouched |
| Do NOT trust a `run_in_background` completion notification's exit code | Held — 6 gate runs, 6 file reads, and run 5 was red |

## Code Analysis

- **`AudioManager::append_decoded`** is the single funnel for every byte-sourced sound: `play_internal` (file path), `play_bytes` → `play_bytes_internal`, `crossfade_bytes`, and `positional::play_bytes_at` (via `play_bytes`). Five entry points, one chain. That is why threading one parameter beat forking a path.
- **Chain order after this session:** decode → effects (speed / low-pass / fade-in from `AudioEffect`) → **repeat** → **tap** → pan → sink. Before: decode → effects → tap → pan → fade → repeat (outermost).
- **`ch.bands` is written in exactly one place** — inside the update path at `src/audio/analysis.rs:530` (`channel.slot.read_bands(&mut raw_bands)`). So `bands()` returns zeros unless `Audio::update` runs, which is what made the dead `AudioFacadeSystem` present as "signal is zero" rather than "system is missing".
- **`AudioManager::poly_tapped` takes `&mut self`** (it lazily grows `channel.poly`), while `tapped` takes `&self`. `append_decoded` is already `&mut self`, so the match between them needed no borrow restructuring.
- **`poly_voice_channel(meter, voice)` → `__poly_{meter}_{voice}`**, and `next_poly_voice` rotates modulo `POLY_VOICES = 8` per meter name. Both are pure and `pub(crate)`, extracted in seq 2 precisely so CI can test them without a device.
- **`play_sfx_poly` does not call `stop_immediate` itself**, unlike `play_tone_poly` — `play_bytes_internal` already tears the channel down first, so ring-wrap cutting comes for free from the path that was already doing it.
- **`App::set_scene` = `apply_scene_cmd(SceneCmd::Replace(..))`**, and `src/app/scenes.rs` states in a comment that Replace "swaps out the entire systems list". Any `app.add_system(..)` before `set_scene` is discarded. Only `beat_crawler` was affected; the other five `AudioFacadeSystem` examples never call `set_scene`.
- **`App::new`'s `register_persistent` block** now ends with `crate::audio_facade::Audio` after the v0.139.1 config family. `register_persistent` on an absent resource is free because `reload_scene` skips types it does not find — which is what makes registering `Audio` cost nothing for a game that never creates one.
- **Spectrum band mapping:** 32 internal log-spaced bands over a 1024-point FFT, resampled to whatever length the caller passes. `MIN_DB`/`MAX_DB` = −100/−30 (Web Audio's `AnalyserNode` defaults). Requesting 16 bands means output slot *i* averages source bands `[2i, 2i+2)`; `LOW_BANDS = 2` therefore covers source bands 0–3, roughly the bottom ~45–190 Hz at typical device rates.
- **The 70 dB window is why saturation is easy:** audible musical content sits in the top ~20 dB of it, so any sustained layer in a band pins that band near 1.0. This is the physics behind "a saturated band cannot show a transient".
- **`Audio` has no facade method for "play bytes on a named non-positional channel"** — the surface is `play_sfx` (anonymous ring), `play_sfx_on_bus` (anonymous ring), `play_at_on_channel` (named but *looping positional*), `play_music` (the single `MUSIC_CHANNEL`), and now `play_sfx_metered`. That absence is why `beat_crawler` uses `play_music` for its track: `MUSIC_CHANNEL` is documented as meterable on both backends (`audio_wasm.rs:217`), so it is the supported path rather than a workaround.
- **`play_music` rides `MASTER_BUS` on native**, not the `"music"` bus — so `set_bus_volume("music", …)` does not scale it. `beat_crawler` does not touch bus volumes, so this is currently invisible there, but a game combining a metered soundtrack with a music-bus slider would notice.
- **`LevelSlot::read()` returns `(rms, peak, seq)`** and `combine_voices` treats "seq unchanged since the last call" as *not sounding*. That per-call staleness test is what makes the 600 ms control in the `play_sfx_metered` probe meaningful — and it is also why a >46 fps caller sees some frames with no new publish (masked at 60 fps by the 0.15 s release, catastrophic at the ~2000 fps of a headless capture).
- **`AnalysisChannel.poly` grows lazily inside `poly_tapped`** (`while channel.poly.len() <= voice { push(PolyVoice::default-ish) }`), so a meter name that has sounded once never allocates again, and a name that is never analyzed allocates nothing at all — `poly_tapped` returns the source untouched when `self.analysis.get_mut(meter)` is `None`.
- **`examples/audio_facade` builds every clip in-process** via `sine_wav(freq, secs)` (a hand-written 44-byte RIFF header + 16-bit PCM body), which is why adding the `4` demo needed **no new asset** — `impact: sine_wav(330.0, 0.30)` in `Default`.
- **The wasm tap has a degraded path with no meter:** `play_sfx_to_opts` falls back to `(None, None)` for `(gain, panner)` if node creation or wiring fails, and routes the bare source straight to `dest`. The tap is therefore `if let (Some(meter), Some(gain)) = …` — a failure loses per-source control *and* metering together, which matches the tone path's behaviour of returning early.
- **`beat_crawler`'s `since_beat` doubles as the cooldown guard.** It is already "seconds since the last turn" (reset on every beat, heard or watchdog-driven), so `KICK_COOLDOWN` needed no new field — the arm/re-arm `armed: bool` was deleted rather than replaced.

## Reusable Procedures

### Distinguishing "the meter is broken" from "the sound stopped"

Both look like `levels() == 0`. The discriminator is one line:

```rust
println!("playing={} rms={:.4}", audio.is_channel_playing(ch), audio.levels(ch).rms);
```

`is_channel_playing` reads the sink, not the tap. `playing=true` with `rms=0.0000` sustained over seconds means the audio graph is fine and the *measurement* is broken — which is what turned a suspected tuning problem into an engine bug in one run.

### Pacing a real-time audio measurement (do NOT use an accumulator)

```rust
let start = Instant::now();
let mut frame = 0u32;
loop {
    frame += 1;
    let target = start + Duration::from_secs_f32(DT * frame as f32);
    let now = Instant::now();
    if target > now { std::thread::sleep(target - now); }
    let now = Instant::now();
    let t = (now - start).as_secs_f32();      // timestamp from the CLOCK
    audio.update((now - prev).as_secs_f32()); // dt from the CLOCK
    prev = now;
}
```

`sleep(d)` sleeps *at least* `d`, so `t += DT` drifts slow against anything real-time. **The tell that you have this bug: results that do not change when you change the parameter.** Here every threshold and every flux setting produced identically 39 fires / 0.458 s / sd 0.025.

### Establishing which spectrum bands a layer occupies

Do not reason about it from note names — the dB window saturates aggressively. Render each layer in isolation and compare per-band percentiles:

```sh
python3 soundtrack.py /tmp/kick.wav --only kick
python3 soundtrack.py /tmp/bass.wav --only bass
# then play each and print p25/p50/p75/p95/max per band
```

A layer whose p50 is ≥ 0.9 in a band **owns** that band; including it in a detector sum adds a constant, not information.

### Splitting one working tree into two stacked PRs

When the engine fix and the example that found it are entangled in one tree:

```bash
git stash -u                       # everything, including untracked assets
git checkout main && git pull --ff-only
git checkout -b fix/engine-part
git stash pop
git add <only the engine files> && git commit      # PR A
git checkout -b feat/example-part                  # carries the remaining changes
git add -A && git commit                           # PR B
# after A squash-merges:
git fetch origin
git rebase --onto origin/main <A-branch-tip-sha> feat/example-part
git diff <pre-rebase-sha> <post-rebase-sha> --stat  # EMPTY = the verified tree is unchanged
```

That last `git diff` is the step worth keeping: it proves the rebase did not alter the tree you ran the gate against, so you do not have to re-run a 7-minute gate.

### Reading a red gate

`EXIT=101` with **152 ok groups and all tests passing** means the failure is after the test step. Grep the log for `^error` rather than scrolling:

```bash
grep -nE '^(error|warning: unused|test result: FAILED|failures:)' /tmp/v.log | head
```

Then fix and re-run **only that step** (`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps; echo $?`) before paying for the full gate again.

### Testing a fix that needs hardware CI does not have

`Audio::new()` returns `None` without a device, so "insert the resource, reset the scene, assert it survived" is untestable on CI. The plan named three fallbacks in preference order; the one that works is **assert on the registration, not on the instance**:

```rust
let app = App::new();
assert!(app.persistent_resources.contains(&TypeId::of::<crate::audio_facade::Audio>()),
        "Audio is not registered as persistent — a scene reset drops the output device …");
```

It fails without the fix (verified by commenting the line out and reading the panic), and the *mechanism* it opts into is already covered by the eight resource round-trip tests beside it. An `#[ignore]`d device test would have been strictly worse: it proves nothing and reads as coverage. The same reasoning produced the two `play_sfx_metered` tests — assert the pure helpers (`next_poly_voice`, `poly_voice_channel`) that were extracted for exactly this reason, and prove the hardware behaviour with a probe instead.

### Instrumenting a game's own code path rather than a probe binary

A standalone probe proves the API works; it does not prove the *game* uses it correctly. When the two disagree, put the probe inside the game behind an env var:

```rust
if std::env::var("BC_DEBUG").is_ok() {
    eprintln!("BCDBG low={:.3} all={:?} playing={}", low, &bands[..6], playing);
}
```

Then `awk` the output for max/threshold-crossing counts. This is what separated "the detector is mistuned" from "nothing is ticking `update`". Remove it with a guarded Python replacement and verify with an independent `grep`, never with the removal script's exit code.

## Files Changed

### Engine source

- `src/app.rs` — `App::new` registers `crate::audio_facade::Audio` as persistent (+ an 11-line comment explaining why it sits outside the audit's line); new test `audio_is_registered_to_survive_a_scene_reset`.
- `src/audio/playback.rs` — `append_decoded` reordered (repeat before tap; pan/fade tail collapsed to a linear `Option` match) and gained `meter: Option<(&str, usize)>`; `play_bytes_internal` gained the same parameter; **new `play_sfx_poly(meter, bytes, bus)`**.
- `src/audio/analysis.rs` — new tests `a_tap_outside_repeat_keeps_publishing_on_every_pass`, `a_tone_and_a_clip_under_one_meter_share_the_ring`, `a_voice_channel_is_private_and_never_the_meter_name`.
- `src/audio_facade.rs` — **new `play_sfx_metered(meter, bytes, bus)`**; `enable_analysis` docs no longer exclude clips; `levels` gained the headless-capture warning.
- `src/audio_wasm.rs` — **new `play_sfx_metered`**; `play_sfx_to_opts` gained `meter: Option<&str>` and taps the per-source gain.

### Examples

- `examples/games/beat_crawler/beat_crawler.rs` — real track via `play_music` on `Audio::MUSIC_CHANNEL`; `LOW_BANDS` 4→2; `KICK_THRESHOLD` 1.2→1.6; arm/re-arm → `KICK_COOLDOWN`; `advance_soundtrack` → `advance_schedule` (plays nothing); `AudioFacadeSystem` moved into `CrawlScene::on_enter`; selftest rewritten to assert grid alignment; module docs rewritten.
- `examples/games/beat_crawler/assets/soundtrack.py` — **new**, 138 lines, CC0 generator with `--only <layer>`.
- `examples/games/beat_crawler/assets/soundtrack.wav` — **new**, 112,940 bytes.
- `examples/games/beat_crawler/assets/README.md` — **new**, provenance + PCM-not-Ogg rationale + the C3-not-C2 measurement story.
- `examples/games/settings_menu/settings_menu.rs` — hand-rolled `register_persistent::<Audio>()` deleted; module doc corrected.
- `examples/audio_facade/audio_facade.rs` — key `4` fires 3 overlapping `play_sfx_metered`; `IMPACT_METER` const; `impact`/`impact_peak` state; live meter readout; `enable_analysis` at setup.

### Docs

- `docs/PATTERNS.md` — "Surviving a scene reset" rewritten as **DECIDED**; `Audio` added to the auto-persisted table; the next reopen trigger stated.
- `docs/MODULE_MAP.md` — row 31 (`register_persistent`), row 79 (audio-reactive hooks: tap order + the headless-capture trap + `play_sfx_metered`), row 91 (`beat_crawler`).
- `docs/CHANGELOG.md` — 0.141.1, 0.141.2, 0.142.0, 0.143.0.
- `CLAUDE.md` — header v1.6.243 → v1.6.247; the scene-reset pattern bullet rewritten.
- `Cargo.toml` / `Cargo.lock` — 0.141.0 → 0.143.0 across four bumps.

### Temporary (created and removed)

- `examples/_track_probe.rs` — rewritten 4 times (band profiler → loop diagnostic → strategy sweep → real-clock sweep). Removed.
- `examples/_sfx_metered_probe.rs` — the 1/3/3-spread measurement. Removed.
- `BC_DEBUG` instrumentation inside `detect_beat`. Removed, verified by grep.

## User Feedback & Preferences (REQUIRED)

- **The session opened with a highly-specified paste prompt** — board gate commands verbatim, the exact verify block, "PHASE 1 ENDS IN QUESTIONS, NOT CODE", "Do NOT self-pick", "Do NOT re-run the insert_core_resources audit", "Grep docs/MODULE_MAP.md, never read it whole". Every instruction was followed as written.
- **Q1 answer: "자동 persist 한다 (추천)"** — the recommendation.
- **Q2 answer: "beat_crawler 실사운드트랙 (추천)"** — the recommendation. **Recommendation now taken 9 of 9.**
- **The user then opened a SECOND turn with a direct instruction: "play_sfx_metered 진행해줘".** This is new in shape. The closing report of the first turn had said `play_sfx_metered`'s priority had risen because its tap sits in the very function the loop bug was found in — and the user picked exactly that, immediately, without being asked. Effectively a 10th recommendation taken, but expressed as an instruction rather than through `AskUserQuestion`.
- **The user did not ask for a status update or intervene at any point during the four PRs.** Merge authority is standing-delegated and was exercised four times without re-confirmation.
- **Standing (from `CLAUDE.md` / memory):** user-facing reports in **Korean**, everything else (code, docs, commit messages, PR bodies, subagent prompts) in **English**. Subagents always get an explicit `model`. Squash-merge on green CI; express merge as a direct instruction, never as an `AskUserQuestion` option.
- **"Read the exit code from the FILE — the notification lied in four straight sessions."** Followed on all six gate runs. It paid off on run 5, which was red.
- **The plan's anti-goal "Do NOT let this become a third capstone"** was honoured — `beat_crawler` gained a track and lost a broken detector, but no new game systems.
- **The plan's stop condition for `play_sfx_metered`** ("if it is not roughly one facade method plus one manager method plus the tap redirection, report why instead of expanding") was met exactly and reported as met.

## Where We're Going

1. **Board gate FIRST, every session.** `../dungeon-merchant/docs/engine-wishlist.md` (next free **EW-012**, unmoved since 2026-07-27) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`). Check whether it **moved** with `git log -1 --date=short`, not whether it looks empty.
2. **If both are empty: ASK — do not self-pick.** The remaining menu is now short, and one item is newly the strongest:
   - **A game example adopting `play_sfx_metered`** — the VISION acceptance test. `audio_facade` demonstrates the surface but is a demo, not a game. This is the direct analogue of v0.140.0 → v0.141.0.
   - **`SURVIVOR_SELFTEST=1`** — carried from seq 1 and seq 2, still unbuilt, and *more* pointed now: `beat_crawler` proved that an example's headline feature can be silently dead for releases with only a HUD string as the symptom.
   - **A fourth procgen mode** (drunkard's walk) — unchanged, still the lowest marginal value.
3. **`engine-current-state` memory trim is OVERDUE.** The parent said "~seq 220" and we are at seq 220; the file is ~48 KB against a ~76 KB working cap. Procedure is in `[[engine-history-archive]]`.
4. **This handoff + its plan land as their own `docs(handoff)` PR**, chain `audio-adoption` seq 3. Bump memory to seq 221 after it merges.

## Risks & Blockers

- **An example's headline feature can be dead for releases with almost no symptom.** `beat_crawler`'s turn clock was running on a watchdog fallback and the only sign was a HUD string nobody read. Any example whose feature degrades *gracefully* has this exposure — and `survivor` is the obvious next candidate, since it has no self-test at all.
- **`register_persistent` before/after `set_scene` is a live footgun for every example.** `beat_crawler` was the only one affected today only because the others do not use scenes. A new scene-based example that registers systems in `main` will hit it, and the failure is silent.
- **Audio remains outside CI entirely.** All four PRs' audio behaviour was proven on a local device; CI can only prove it compiles and that the arithmetic tests pass. There is no automated guard against a repeat of the loop-meter bug in a *different* path.
- **A headless capture cannot verify metering, and now two sessions have tried.** It is documented on `Audio::levels` and in module-map row 79, but the instinct to reach for `ENGINE_CAPTURE` is strong because it is the only headless verification the engine offers.
- **Real-time windowed playtest is not currently available** — `screencapture` returns black (screen-recording permission) and `osascript` could not locate the window. Real-time verification is therefore limited to headless loops paced off `Instant` (which is what the self-tests do).
- **The `soundtrack.wav` and `PATTERN` must be changed together.** `soundtrack.py` mirrors `PATTERN` by hand; nothing enforces it. A groove edit in one place silently desyncs the watchdog schedule from the audible track.
- **`cargo fmt` can still defeat a scripted edit.** Unchanged. Every probe removal this session was verified with an independent `grep`.
- **`dungeon-merchant` has no CI or branch protection** — its board is discipline, not enforcement. `rust-survivors` is paused but remains a real bug-report channel.

## Open Questions

- **Which game example should adopt `play_sfx_metered`?** `audio_facade` demonstrates it but is a demo. `beat_crawler` has an asset pipeline and currently has **no SFX at all** (attacks are silent), so a metered hit clip is a genuine addition rather than churn — but seq 2's finding #3 applies: metering only pays when the sound carries information the game does not already hold. In `beat_crawler`, damage varies (ON BEAT = 2 vs 1) and several hits can land in one beat, so a summed meter would carry "how much violence this beat" — defensible, but thin. Needs a decision, not a default.
- **Should `survivor` get `SURVIVOR_SELFTEST=1`?** Carried from seq 1 and seq 2 and now three sessions old. `beat_crawler` just demonstrated the exact failure it would prevent.
- **Should the engine warn when `add_system` is called before `set_scene`?** The silent drop cost this session an investigation and had cost `beat_crawler` several releases of a dead feature. A debug-build `log::warn!` when systems are discarded by a Replace is cheap. Not filed, not decided.
- **Should `bands()` ever work for a metered one-shot?** Still deliberately zeros. Unchanged from seq 2; nobody has a use case for the spectrum *of a one-shot*.
- **Should `embedded_image` get a web harness?** Carried unanswered from five prior sessions. Probably time to either do it or close it.
- **Is the `add-facade-capability` skill worth writing?** This session was its **n=4** (`play_sfx_metered` followed the same facade + native + wasm + policy-module shape as `play_tone_metered`). Still deferred.

## Quick Start for Next Session

```bash
# Restore full context
cat plans/handoffs/HANDOFF_audio-adoption_loop-meter-and-clip-metering_2026-07-31.md

# Nothing is dangling — #399/#400/#401/#402 all merged, main clean at a00bad0 (v0.143.0).
cd ~/Projects/skeleton-engine
git log --oneline -5      # expect a00bad0 at the tip (or the handoff merge above it)
git status -s             # expect clean

# 1. READ THE BOARD FIRST — both channels; a filed request preempts everything
awk '/^## Active requests/,/^## Done \/ archive/' ../dungeon-merchant/docs/engine-wishlist.md
grep -A3 '^## Open Requests' ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md
cd ../dungeon-merchant && git log -1 --date=short -- docs/engine-wishlist.md && cd -
# ^ later than 2026-07-27 = the board MOVED; read it before anything else

# 2. Verify starting state — the block docs/VERIFICATION.md carries.
#    Read the exit code from the FILE. It went RED (101) this session and the file is why we saw it.
rm -f /tmp/v.exit /tmp/v.log
(./scripts/verify.sh > /tmp/v.log 2>&1; echo $? > /tmp/v.exit) &
until [ -f /tmp/v.exit ]; do sleep 20; done
echo "EXIT=$(cat /tmp/v.exit)"; ls -l /tmp/v.exit
echo "ok groups: $(grep -c 'test result: ok' /tmp/v.log)"
# expect 0 · 152 ok groups · 1360 lib tests   (v0.143.0)

# 3. Key files
#   src/audio_facade.rs                     — play_sfx_metered + the levels() headless warning
#   src/audio/playback.rs                   — append_decoded (repeat BEFORE tap), play_sfx_poly
#   examples/games/beat_crawler/…           — the track, the cooldown detector, the SELFTEST precedent
#   examples/games/survivor/survivor.rs     — has NO self-test; the SURVIVOR_SELFTEST candidate
#   docs/MODULE_MAP.md                      — GREP IT (rows 79, 91 rewritten this session)

# 4. Real-device audio check (CI cannot do this)
BEAT_CRAWLER_SELFTEST=1 cargo run --release --example beat_crawler_game
# expect exit 0 and "16 kicks ... 0.638s ... on-grid 15/15"

# 5. FIRST CONCRETE ACTION
#    Read both board files and state the verdict. If empty, ask ONE AskUserQuestion (Korean):
#      - a GAME example adopting play_sfx_metered (the VISION acceptance test — recommended)
#      - SURVIVOR_SELFTEST=1 (now sharper: beat_crawler proved a headline feature can die silently)
#      - 4th procgen mode (drunkard's walk)
#    Do NOT self-pick.
```
