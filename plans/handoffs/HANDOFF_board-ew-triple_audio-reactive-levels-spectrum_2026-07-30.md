# Shipped: audio-reactive hooks end to end (levels + spectrum), then closed the web-smoke verification debt (v0.136.0 → v0.137.0)

**Date:** 2026-07-30 (work window: 2026-07-29, all four PRs merged that day)
**Status:** COMPLETED (PRs #382–#385 all merged; `main @ aef39e1`, v0.137.0, clean + green)
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `board-ew-triple` seq `3`
**Parent:** `HANDOFF_board-ew-triple_atlas-bytes-and-wasm-proof_2026-07-29.md`
**Prior chain:** `HANDOFF_board-ew-triple_three-board-requests_2026-07-26.md` > `HANDOFF_board-ew-triple_atlas-bytes-and-wasm-proof_2026-07-29.md` > this

> **Note on the chain tag.** `board-ew-triple` is now historical: EW-009/010/011 are closed and archived,
> and no board request has driven work since seq 1. The chain continues under this tag because the
> session was opened by a paste prompt naming seq 2 and executed that handoff's "Where We're Going"
> verbatim. Read the tag as "the empty-board menu chain" from seq 2 onward. A future session may
> reasonably start a fresh tag (e.g. `audio-reactive`) once this arc closes.

---

## Reference Documents

- `CLAUDE.md` — project conventions, module map, verify gate. Edited twice this session (audio row extended twice; Core-patterns summary extended). 195 → **198 lines** (cap 200).
- **`docs/VERIFICATION.md`** — the verify gate's six exit-code traps and three blind spots. **Read it before trusting any gate result.** Edited twice this session (new smoke row; the byte-size-vs-assertion classification). 164 → **155 lines** after the prior session's split, now 155.
- **`docs/PATTERNS.md`** — architecture patterns + task recipes. **Two new patterns added this session** (see #385). 332 → **378 lines**.
- `docs/WASM_SMOKES.md` — the full smoke list and how to add one. Gained the `audio_reactive` entry.
- `docs/CHANGELOG.md` — 0.136.0 / 0.137.0 entries are the migration notes.
- `docs/VISION.md` — "a feature is not done until a playable example exercises it in real play", which drove the example and both of its self-checks.
- `../dungeon-merchant/docs/engine-wishlist.md` — the shared board. Empty; last edited 2026-07-27, before this session.
- `.claude/proposals/2026-07-29.md` — this session's `/wrap` skill/rule proposal (111 lines + an appended "실행 결과" section). Gitignored.

---

## Since Last Handoff

Framed against the parent (seq 2) and its "Where We're Going":

- **The parent's step 1 and 2 executed exactly as written.** Board gate first on both channels → both empty → **ASK, do not self-pick**. The user was asked in Korean with the parent's menu, minus windowed capture. They picked **audio-reactive hooks**. No code was written before that answer.
- **The parent's recorded tip was already stale by one commit.** It said `main @ e481873`; `origin/main` was actually **`a42aa1a`** because **PR #381 merged after the handoff was written**. Local `main` was behind and the session opened sitting on the leftover branch `docs/handoff-seq2-session-closed` (content identical to `origin/main`). Caught in the first three minutes by comparing `git rev-parse HEAD origin/main`. **Lesson for the next handoff: a handoff that lands as its own PR cannot record its own merge hash — expect the recorded tip to be one commit stale.**
- **The parent's open question "Do the remaining web examples' smokes actually pass?" is now ANSWERED: yes, all of them.** All 14 swept, 14/14 pass. See Evidence.
- **The parent's CLAUDE.md headroom warning materialized the same day.** It said "6 lines of headroom and the module map grows ~1 line per feature… will breach again within a few features." The first draft of the #385 CLAUDE.md edit hit **exactly 200/200**; the added prose had to be tightened back to 198.
- **The parent's dominant risk — "verification tooling that lies" — recurred twice, in the same mechanism it already documented.** Trap 4 (a `run_in_background` notification reports the trailing command, not the gate) fired **twice in one session**: the notification said `exit code 0` while the `.exit` file held **1** (fmt) and then **101** (rustdoc). The doc existed and did not prevent it; reading the file did catch it both times.
- **The parent's "windowed capture is dead, not deferred" holds and was re-litigated once, at the user's request.** They asked *why* the game declined. Answered from the board thread rather than from memory (see Chunk 1).
- **Priorities did not shift.** The parent left an open menu; the session took one item (audio-reactive), completed it in two PRs as designed, then spent the remainder on the parent's own open question (smokes) and on `/wrap` follow-through.

---

## The Goal

Execute the parent handoff's plan: run the board gate, and — because it was empty — get an explicit direction from the user instead of self-picking. The user chose **audio-reactive hooks**: expose what a playing sound is *doing* (amplitude, then frequency spectrum) to game logic and visuals, which the engine previously had **no** way to provide at all. This is genre-agnostic capability in the VISION's sense — music visualizers, beat-reactive spawns, mouth flaps, audio-driven VFX — and it lands in the one subsystem where native and wasm share almost no code, so the cross-platform story had to be designed rather than assumed.

The session then continued into two follow-ups the user selected: closing the parent's open question about whether the web smokes actually pass, and (via `/wrap`) promoting two of the session's architectural findings into `docs/PATTERNS.md` plus auditing the local skills.

---

## Where We Are

- **`main @ aef39e1`, package v0.137.0, CLAUDE.md header v1.6.235, clean tree, all gates green.** Local `main` == `origin/main`.
- **Four PRs merged, serially, each 6/6 green:** **#382 `2f22a4f` v0.136.0** (MINOR), **#383 `2a10a4b` v0.137.0** (MINOR), **#384 `66715e5`** (docs-only), **#385 `aef39e1`** (docs-only).
- **Lib tests 1297 → 1338 (+41).** Audio suite alone: 93 tests. All new tests are **device-free** (no audio hardware needed) so they run on CI.
- **`Audio::levels(channel) -> AudioLevels { rms, peak }`** (v0.136.0) and **`Audio::bands(channel, &mut [f32])`** (v0.137.0) are the whole public surface, mirrored on `AudioManager` (native) and `WebAudio` (wasm). Plus `enable_analysis`/`disable_analysis`/`is_analysis_enabled`/`set_analysis_smoothing`/`analysis_smoothing`, `enable_spectrum`/`disable_spectrum`, and the const `Audio::MUSIC_CHANNEL`.
- **Three new source files:** `src/audio_analysis.rs` (347 lines, un-gated shared policy), `src/audio/analysis.rs` (535, native tap + manager surface), `src/audio/spectrum.rs` (268, hand-written FFT).
- **Measurement point is PRE-VOLUME** — after a sound's own effects, before channel/bus/duck/master gain. **On native this is structural, not enforced:** volume lives on the rodio *sink* (`sink.set_volume(effective_volume(channel))`, `playback.rs:181`), never in the source chain, so any tap in that chain is pre-volume by construction.
- **Analysis is opt-in and free when off.** `tapped()` returns the same `Box` untouched for an unanalyzed channel, so its source chain is byte-identical to before. Spectrum is a *second*, separate opt-in because it costs an FFT per window; a levels-only channel runs no transform (test-pinned).
- **`Audio::update` is no longer a no-op on wasm.** It now samples the meters. `AudioFacadeSystem` already called it every frame, so no game code changes.
- **The facade gained 7 methods in #382 with ZERO `cfg` lines**, and one existing `cfg` pair was *deleted*. This is measurable, not rhetorical — see Evidence.
- **Cross-platform parity is engineered:** `MIN_DB`/`MAX_DB` are pinned to Web Audio's `AnalyserNode` defaults (−100/−30 dB) **and set explicitly on the node**; both backends fold FFT bins through the shared `log_band_range`.
- **Parity is demonstrated, not asserted:** native rms **0.619** vs web **0.631** on the same tone (theory 0.9/√2 = 0.636); spectrum native low-half **11.18** / high **0.25** vs web low **9.08** / high **0.00**.
- **`examples/audio_reactive/`** (644 lines) is the acceptance test: rms pulse + peak kick-flash + two meters + a 28-bar spectrum analyzer. `M` mutes and the pulse keeps going (the pre-volume demo); `S` cycles release. Native + web from one `build_app()`.
- **`scripts/audio_reactive_smoke.sh`** (139 lines) runs it in headless Chrome and reads a verdict from `document.title`, asserting both a live level **and** a low-biased spectrum.
- **All 14 smoke scripts swept: 14/14 pass.** 4 native + 10 web. Nothing was broken.
- **Only 9 of the 14 assert something specific; 5 are byte-size-only** and are now named in `docs/VERIFICATION.md` with what only a screenshot can tell you.
- **`docs/PATTERNS.md` gained two patterns** (#385): shared policy for cfg-split backends; real-time audio-thread producers.
- **`.claude/skills/ship-wasm-example/SKILL.md` had a reproducible defect and is fixed** — it told you to `chmod +x`, which records nothing under this repo's `core.fileMode = false`, i.e. it would recreate the bug #378 fixed across 31 files. Three other skills gained a `docs/VERIFICATION.md` pointer.
- **Memory advanced seq 199 → 203**, plus an audit section appended to `local-tooling-skills`.
- **The board was never touched.** Nothing this session served a board request, and the game renders exclusively through `UiQueue`/`TextQueue` with zero `Sprite`/`Transform` entities (their own 2026-07-03 note), so an audio-analysis API has no applicability to them. Resisted posting a notice they have no use for.

---

## What We Tried (Chronological)

### Chunk 1 — Onboarding, board gate, and the direction ask (early)

1. **Read the parent handoff and `docs/VERIFICATION.md` before touching any gate**, per the paste prompt's instruction. Confirmed the doc documents **six** exit-code traps and **three** blind spots.
2. **Corrected the user's premise, plainly and once.** Their prompt said "three documented ways a gate reports a confident-but-false green"; the doc has six traps (pipe status / zsh `$pipestatus` 1-indexing / `;` not short-circuiting / background-notification / stale `.exit` / `core.fileMode`) plus three separate blind spots. Stated the real count without belaboring it.
3. **Ran the board gate on both channels before anything else.** `../dungeon-merchant/docs/engine-wishlist.md` → Active requests **empty**, next free **EW-012**. `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` → `_None._`. Re-checked at the end of the session via `git log` on the board file: last edit **2026-07-27**, i.e. before this session — so nothing arrived mid-session.
4. **Found the state discrepancy immediately.** Parent recorded `main @ e481873`; `origin/main` was `a42aa1a` (PR #381, the parent's own "session closed" correction). Local `main` stale, sitting on branch `docs/handoff-seq2-session-closed`. Proved the content was identical with `git diff --stat origin/main HEAD` → empty. Synced with `git merge --ff-only`.
5. **Reported one local branch with no remote counterpart** (`docs/handoff-dm-adoption-seq4`) and deliberately did **not** delete it — unlike the other five leftovers, it has no `origin/` ref, so deleting could lose work.
6. **Read the listed key files plus three adjacent ones**, chosen to inform the menu rather than at random: `src/mapgen.rs` (1140 lines, 3 generators over a shared `DungeonMap`), `src/audio_facade.rs` (504 lines, 25 methods, **zero** amplitude/spectrum surface — the gap), `examples/games/` (20 capstones).
7. **Baseline verify gate: exit 0**, 150 `ok` groups, 1297 lib tests, `all checks passed ✓`. Read from the `.exit` file with an mtime freshness check (16 s old), not from the notification.
8. **Asked via `AskUserQuestion` in Korean**, with the parent's menu and windowed capture removed. Added a fourth option the parent had not listed (the web-smoke debt) because it was cheap and documented as an open question.
9. **The user asked a sub-question instead of choosing: "윈도우 캡쳐 거정 사유 알려줘."** Answered from the board thread rather than memory: the engine *offered* it at `engine-wishlist.md:80` ("Say so if a windowed capture is genuinely needed"), and the game declined in one sentence at `:82` ("Headless-only capture is fine for us"). Made clear they gave **no elaborate reason**, then showed why it is nonetheless coherent — their own EW-011 acceptance criteria (`:73`) asked for "no OS automation permissions", never for a window, and windowed readback would reintroduce exactly the display/permission dependency they wanted gone (plus needing `COPY_SRC` on the surface, disallowed on WebGL2).
10. **Re-asked the menu; the user picked audio-reactive hooks.**

### Chunk 2 — Design, then PR1: `Audio::levels` (#382, v0.136.0) (early-mid)

11. **Researched all three layers before proposing anything.** Found the insertion points already existed: `PannedSource` (`src/audio/source.rs`) is a pass-through `Source` that sees every sample in `next()` — the exact shape needed; `AnalyserNode` drops into the wasm graph; and `Audio::update(dt)` is **already called every frame** by `AudioFacadeSystem`, giving a free per-frame hook.
12. **Identified the three asymmetries that constrain any intersection API**, and put them to the user rather than guessing: (a) rodio's `MixerDeviceSink` exposes **no master-mix tap**, so per-channel is the only tractable granularity, while wasm can tap anywhere; (b) `Audio::play_sfx` **round-robins 16 anonymous voices** on native (`sfx_seq`), so a one-shot has no stable name to meter; (c) a spectrum is **free on wasm and absent on native**.
13. **THE KEY STRUCTURAL FINDING, and it decided the design:** on native, volume is **not in the source chain at all**. `playback.rs:181` does `sink.set_volume(self.effective_volume(channel))` — channel × bus volume live on the `Player`/sink. The `.amplify(volume)` at `:185` is the *tone's own amplitude argument*, not the mixer stage (the doc comment says so explicitly). Therefore **any** tap in the source chain is pre-volume by construction, and bus/duck/master are excluded for free. The user's chosen semantics fell out of the architecture instead of needing enforcement.
14. **Presented a written design and asked exactly two questions** — the ones where different answers meant materially different work: scope (2-stage vs all-at-once vs amplitude-only vs wasm-only spectrum) and measurement point (pre- vs post-volume vs both). **User took both recommendations: 2-stage PR, pre-volume.**
15. **Wrote `src/audio_analysis.rs` first** (shared, un-gated) — `AudioLevels`, `DEFAULT_ANALYSIS_SMOOTHING` (0.15 s), `MUSIC_CHANNEL`, and `smooth_toward` (**instant attack, timed release**). Explicitly modeled on `audio_spatial`, whose own header already said *"a single canonical implementation so the two builds can't drift."*
16. **Wrote the native tap** (`src/audio/analysis.rs`): `LevelTap<S: Source>` publishing RMS/peak per 1024-sample window into a lock-free `LevelSlot` of atomics (`f32::to_bits()` → `AtomicU32`), because the tap runs on **rodio's playback thread** where a mutex is a dropout.
17. **Designed for the freeze failure mode up front.** The tap only runs while rodio pulls samples, so a stopped channel would hold its last value forever and the meter would read as a **stuck bar at full height**. Added a monotonic **sequence counter**: unchanged since the last tick ⇒ not producing ⇒ decay toward zero. Then discovered wasm needs none of this — an `AnalyserNode` reads the live graph, so silence decays on its own. Recorded the asymmetry.
18. **Two compile errors in the first test write:** `SamplesBuffer::new(1, 48_000, …)` fails — rodio 0.22 wants `NonZero<u16>`/`NonZero<u32>`. Fixed with `ChannelCount::MIN` and a `const SampleRate::new(48_000)` match, copying the existing `TONE_CHANNELS`/`TONE_RATE` pattern in `playback.rs`.
19. **13 unit tests passed first run**, including the one that pins the math is genuinely RMS: **a unit sine measures 1/√2 ≈ 0.707**, which distinguishes RMS from a rectified average (2/π ≈ 0.637) or the peak (1.0).
20. **`web_sys::AnalyserNode` did not compile for wasm** — `E0425: cannot find type`. web-sys is feature-gated per DOM type; had to add `"AnalyserNode"` to `Cargo.toml`. **Caught only by `cargo build --target wasm32-unknown-unknown`**, which is in the gate for lib — so the gate did its job here.
21. **Facade forwarding needed no `cfg` at all.** Because both backends were given identical method names and signatures, all 7 methods are a bare `self.inner.x(...)`. Additionally the pre-existing `cfg` pair in `update()` was **deleted**, since wasm now has real work to do there.
22. **Wrote the example, then photographed it with the engine's own `ENGINE_CAPTURE`** — zero example code needed, since v0.134.0 shipped env-driven capture. **The first capture caught a real layout bug:** the rms/peak labels were drawn *under* their bars, because label Y and bar Y were independent magic numbers. Fixed by deriving label Y from bar Y (`LABEL_DY`), so it cannot drift again. **Third consecutive session in which eyeballing the headless PNG found a bug.**
23. **Set the exec bit correctly the first time**, having read #378's lesson: `chmod +x` *and* `git add` *and* `git update-index --chmod=+x`, verified with `git ls-files -s` → `100755`. Then swept all `.sh`: **33/33 at 100755**.
24. **The verify gate went red twice, and the notification lied both times.**
    - Run 1: notification "exit code 0", file **1** — `cargo fmt --check` reflowed a hand-wrapped `assert!` and a `if let Some(state) = analysers.borrow().get(…)` chain. This is the documented `cargo fmt` reflow trap; ran `cargo fmt` and re-verified.
    - Run 2: notification "exit code 0", file **101** — four rustdoc errors: `crate::audio_wasm::WebAudio` unresolved (cfg'd out on native), `crate::audio_spatial` is `pub(crate)` so linking to it from public docs is an error, and `smooth_toward` likewise ×2. Fixed by de-linking to plain backticks.
25. **Ran the three blind-spot checks explicitly** because the gate covers none of them: wasm *example* build (gate is lib+bins), native end-to-end through a real device (rms 0.619 rise / 0.0096 decay), and the browser smoke (rms 0.625).
26. **`/ship` 0.136.0 → final verify exit 0 (151 groups, 1316 tests) → PR #382 → `--auto --squash` → 6/6 green → merged `2f22a4f`.** Noted `Render tests (lavapipe)` took **10m34s**, well above its usual ~1m20s, but it passed.

### Chunk 3 — PR2: `Audio::bands` spectrum (#383, v0.137.0) (mid)

27. **Hand-wrote the FFT rather than taking a dependency.** rodio does not analyze and nothing in the tree provides an FFT; `src/audio/spectrum.rs` is an iterative radix-2 Cooley–Tukey with a Hann window. Justified in the module header by house precedent (SplitMix64 in `Rng`, shadowcasting in `FovMap`) **and** by the fact that an FFT is checkable *exactly*.
28. **Pinned the transform by mathematics, not a golden file.** 8 tests: energy lands in the bin matching a known cosine; the peak follows across bins 8/32/100/255; DC lands in bin 0 alone; **Parseval's theorem** (time-domain energy == frequency energy / N, rel. error < 1e-3) which catches scaling or dropped terms even when the peak bin is right; a non-power-of-two length is left **untouched** rather than transformed wrongly; Hann tapers to ~0 at both ends; a low tone peaks below a high tone; silence produces no bands.
29. **Downmixed to mono for the transform.** An FFT over raw *interleaved* stereo alternates L and R samples and is meaningless. Added a per-frame accumulator (`frame_acc`/`frame_ch`/`channels`) feeding a separate 1024-**frame** spectrum buffer, independent of the interleaved 1024-**sample** level window.
30. **Accepted running the FFT on the audio thread, with the arithmetic to justify it:** ~10k flops per 1024 frames ≈ 21 ms at 48 kHz ≈ 0.05% of a core, and it allocates nothing (scratch buffers owned by the tap, allocated in the constructor on the game thread). The alternative — publishing raw samples through a triple buffer — needs `unsafe`.
31. **Made spectrum a separate opt-in** (`enable_spectrum`) gated by an `AtomicBool` the tap reads per sample, so a channel that only wants `levels` pays for no transform. Test-pinned: a levels-only channel's `bands()` fills zeros.
32. **Extracted `log_band_range` to the shared module specifically so the two backends cannot disagree about what "band 7" means** — native folds complex magnitudes from its own FFT, wasm folds bytes from the browser's, and without one definition the same band index would cover different frequencies. Refactored the native fold to call it.
33. **Pinned `MIN_DB`/`MAX_DB` to Web Audio's `AnalyserNode` defaults (−100/−30) and set them explicitly on the node.** Matching makes the numbers comparable; re-setting explicitly means a browser changing its defaults cannot silently desync the platforms. A test asserts the two constants, with a comment that "tuning" them breaks cross-platform meaning.
34. **`normalized_db` triggered a wasm-only `dead_code` warning** (the browser does that conversion itself), which the wasm clippy gate would red. Fixed with `#[cfg(not(target_arch = "wasm32"))]` and a doc note that this is *the one part of the spectrum path the backends genuinely do not share* — **not** with a blanket `allow`.
35. **The first spectrum capture looked wrong and I did not theorize — I printed the values.** The lowest bars all moved together. A temporary debug print showed bands 0–6 at **exactly 1.0**. Cause: 1024 points at 44.1 kHz is ~43 Hz per bin, so a 110 Hz tone sits in bins 2–3, and `edge(b) = 512^(b/32)` maps bands 0–3 all to bin 1..2 and 4–6 to bins 2..4. **Physics, not a bug.** Documented on `bands()` under a "Resolution" heading instead of leaving it as a surprise.
36. **That investigation changed the test.** The original assertion was `loudest_band < SPECTRUM_BARS/2` — but with 7 tied bands, which one `max_by` returns is an implementation detail (Rust returns the *last* maximum). Replaced with **low-half vs high-half total energy** (`low > high * 3.0`), which is tie-independent. Applied the same change to the wasm self-check.
37. **rustdoc went red a third time**, exit **101**: `redundant explicit link target` — my own fix from step 24 wrote `[`DEFAULT_ANALYSIS_SMOOTHING`](crate::DEFAULT_ANALYSIS_SMOOTHING)`, where the label already resolves. Fixed with a reference-style link definition.
38. **Caught a doc/code mismatch I had written myself:** `spectrum_into`'s doc said each band "averages the FFT bins", but the code takes the **peak** (deliberately — a narrow tone in a wide high band would be averaged into invisibility). Corrected the doc.
39. **Final verify exit 0, 151 groups, 1338 tests.** Re-ran all three blind-spot checks on the settled tree: wasm example build 0; native low 11.18 / high 0.25, peak band 6/28; browser low 9.08 / high 0.00.
40. **PR #383 → 6/6 green → merged `2a10a4b`.** Render tests back to 1m14s.

### Chunk 4 — Smoke sweep (#384), `/wrap`, patterns (#385), skill audit (late)

41. **The user asked for a recommendation ("다음 추천작업 알려줘") rather than picking.** Re-ran the board gate first (still empty; board file last edited 2026-07-27), then recommended the **web-smoke sweep** over feature work, with the argument stated in evidence: 11 of 14 had never been completed since #378 unblocked them, and the last time anyone checked, 26 of 31 scripts were broken.
42. **Swept all 14 sequentially, recording real exit codes to a file.** 4 native first (fast, no Chrome), then 10 web. **14/14 pass, 0 failures.**
43. **Did not accept the green at face value.** Classified each script: 9 assert something specific (a `*_CHECK: PASS` page verdict, a lit-pixel ratio, a reported asset failure), **5 are byte-size-only**. Eyeballed the four surviving screenshots and re-ran `hdr_web` with `SMOKE_KEEP=1` because its own comment asks for an eyeball.
44. **The eyeball found real information a byte count cannot carry:** `hdr_web` shows the HDR panel keeping core (white) and mid (yellow) **distinct** while LDR collapses both to one flat grey block — the entire claim of the feature. `wasm`/coin_race shows the HUD reading **"Player #1"**, i.e. the WebSocket handshake actually completed rather than the page merely rendering. `centered_text` shows each label's center on its guide at the off-center x=192/768 that were the original EW-001 bug.
45. **Landed that classification as #384** (docs-only, no bump since CLAUDE.md was untouched) → 6/6 green → `66715e5`.
46. **Ran `/wrap`.** Read the 12h log (which spans **two sessions** — #379/#380/#381 belong to the parent), wrote `.claude/proposals/2026-07-29.md` (111 lines) with 5 candidates, and verified two claims with git rather than asserting them: `comm -12` showed #382 and #383 touched **exactly the same 14 files**, and `git log -S'web-sys' -- Cargo.toml` showed today was only the **second** such edit ever.
47. **The user said "2번 3번 추가"** — adopt candidates 2 and 3. Added both to `docs/PATTERNS.md` (+46 lines) and extended CLAUDE.md's Core-patterns summary.
48. **CLAUDE.md's first draft hit exactly 200/200.** Compressed my own addition (prose is the only section type that can shrink, per #379's measurement) → **198/200**. Bumped the doc version v1.6.234 → v1.6.235 since CLAUDE.md was edited but the package was not.
49. **The user also asked "사용되지 않거나 수정 할 만한 스킬은 없어?"** — and this found the session's sharpest defect. **`ship-wasm-example` line 109 said only `chmod +x`.** Under `core.fileMode = false` that records nothing in git, so the skill would recreate exactly the bug #378 fixed across 31 files — and which v0.135.1 had actually shipped (a 644 `build.sh` *and* a 644 smoke script). **Worse: step 5 verified with `bash …/build.sh`, which runs a 644 file happily and so masked the defect the skill itself created.**
50. **Fixed it:** a ⚠️ block with the `chmod` + `git add` + `git update-index --chmod=+x` + `git ls-files -s` sequence and the #377/#378 case history; step 5 changed to a **direct** `./…/build.sh` invocation; added a wasm-bindgen CLI-vs-`Cargo.lock` version check.
51. **Second skill finding:** `add-feature-example`, `split-module` and `ship` all inline the verify command list, but **none of the 6 skills referenced `docs/VERIFICATION.md`** — created 12 h earlier precisely to hold how to *read* a gate result, which is what failed twice today. Added the non-piped exit-code rule + the background-notification trap + a pointer to all three.
52. **Landed the patterns as #385** → verify exit 0 → 6/6 green → `aef39e1`.
53. **Appended an "실행 결과" section to the proposal file** so the proposal and what actually shipped cannot drift, and recorded the skill audit in the `local-tooling-skills` memory (skills are gitignored, so memory is the only record).

---

## Key Decisions

- **Ask, don't self-pick, even though the menu was already written.** The parent said the board would likely be empty and to ask. It was, and the user's pick (audio-reactive) was **not** the option I would have ranked first for risk. Following the rule produced better work than following my own ranking.
- **Answer "why was windowed capture declined" from the board file, not from memory.** Quoted the engine's offer (`:80`) and the game's one-sentence refusal (`:82`), and was explicit that they gave no detailed reason — then supplied the coherence argument from their own acceptance criteria. Distinguishing "what they said" from "why it makes sense" mattered because the second part is my inference.
- **Put the two design questions to the user instead of picking defaults.** Scope (2-stage vs one PR) and measurement point (pre- vs post-volume) both change what the feature *is*. The pre-volume choice in particular is a product decision — it determines whether a visualizer keeps working at volume 0.
- **Measure pre-volume, and let the architecture enforce it.** Rather than adding a rule ("don't tap after the volume stage"), the tap sits in the source chain, where volume structurally is not. There is no way to get it wrong later.
- **Extract the *policy* to an un-gated module, not the implementation.** `smooth_toward`, `log_band_range`, `MIN_DB`/`MAX_DB` are shared; the tap and the `AnalyserNode` are not. Copying `audio_spatial`'s stated reasoning rather than inventing a structure.
- **Match the other platform's constant, then set it explicitly.** `MIN_DB`/`MAX_DB` are the browser's own defaults — relying on the default would have worked *today* and desynced silently on a browser change.
- **Give both backends identical method names before writing either implementation.** This is what makes the facade `cfg`-free; a naming mismatch would have bred `cfg` in the facade instead. Measured: 7 methods, 0 `cfg` lines, 1 pair deleted.
- **Hand-write the FFT.** ~90 lines and no dependency, versus pulling a DSP crate for one 1024-point transform. Defensible *specifically* because an FFT is exactly testable — the decision would be different for something only checkable by eye.
- **Run the FFT on the audio thread rather than reach for `unsafe`.** The alternative (publishing raw samples via a lock-free triple buffer) needs `UnsafeCell` + a hand-rolled `Sync`. Bounded arithmetic (0.05% of a core) beat introducing unsafe code into an audio path.
- **Make spectrum a second opt-in.** Preserves the "free when off" property that levels-only users get, at the cost of one more API call.
- **Assert half-vs-half energy, not argmax.** Once the tie was understood, asserting on which tied band wins would have been testing an implementation detail — i.e. testing noise.
- **Document the frequency resolution instead of hiding it.** The lowest bands genuinely move together; a future reader would file it as a bug otherwise.
- **`cfg`-gate `normalized_db` rather than `#[allow(dead_code)]`.** The `/ship` skill's own guardrail says never silence a gate failure; the cfg is also *more* truthful, since it marks the one genuinely unshared step.
- **Recommend maintenance over features when asked for a recommendation.** The smoke sweep was the honest answer given 11 never-completed scripts and a 26/31 prior failure rate, even though the user had been picking feature work.
- **Do not delete `docs/handoff-dm-adoption-seq4`.** It is the one local branch with no `origin/` counterpart, so deletion is not obviously lossless.
- **Do not touch the board.** Nothing served a request, and the game has no use for an audio-analysis API (they render entirely through `UiQueue`/`TextQueue`). Posting a notice would be noise.

---

## Evidence & Data

### Shipped: PR → version → commit → diffstat

| PR | Version | Bump | Commit | Files | +/− |
|---|---|---|---|---|---|
| #382 · `Audio::levels` | **0.136.0** | MINOR | `2f22a4f` | 18 | +1662 / −19 |
| #383 · `Audio::bands` | **0.137.0** | MINOR | `2a10a4b` | 15 | +1004 / −36 |
| #384 · smoke classification | — | docs-only | `66715e5` | 1 | +18 |
| #385 · two PATTERNS rules | — | docs-only | `aef39e1` | 2 | +51 / −2 |

### CI behaviour (all 6/6 green)

| PR | `Test (native)` | WASM | Windows | Package | Render | Rustdoc |
|---|---|---|---|---|---|---|
| #382 | 5m29s | 1m40s | 1m57s | 1m16s | **10m34s** | 38s |
| #383 | 5m32s | 48s | 1m57s | 1m6s | 1m14s | 40s |
| #384 | 4m40s | 43s | 1m45s | 58s | 1m21s | 45s |
| #385 | 4m19s | 46s | 1m28s | 1m14s | 1m2s | 34s |

`Render tests (lavapipe)` on #382 took **10m34s** against a ~1m20s norm and then returned to normal. Unexplained, passed, not chased.

### Verify-gate history (every run, exit codes read from the file, never the notification)

| # | Tree | Exit | Cause / note |
|---|---|---|---|
| 0 | session start, clean `main` | **0** | 150 groups, 1297 tests — baseline |
| 1 | `Audio::levels` + example | **1** | `cargo fmt --check` reflow (multi-line `assert!`, `if let` chain) |
| 2 | after `cargo fmt` | **101** | 4 rustdoc errors: 1 unresolved (`crate::audio_wasm::WebAudio`, cfg'd out natively) + 3 links to `pub(crate)` items |
| 3 | after de-linking | **101** | `redundant explicit link target` — introduced by my own fix in run 2 |
| 4 | after reference-style link | **0** | 151 groups, 1316 tests |
| 5 | + 0.137.0 spectrum | **0** | 151 groups, **1338** tests |
| 6 | + PATTERNS docs | **0** | 151 groups |

**Three red runs, three different causes, and the background notification reported `exit code 0` for two of them.** Trap 4 reproduced twice in one session.

### Cross-platform parity — measured, not asserted

| Quantity | native | web | theory / note |
|---|---|---|---|
| `levels().rms`, 0.9-amplitude 110 Hz sine | **0.619** | **0.631** | 0.9/√2 = 0.636; envelope ramps account for the rest. 2% apart. |
| `levels().rms` after the tone ends | **0.0096** | — | decayed, not frozen |
| spectrum low-half energy (bands 0–13 of 28) | **11.18** | **9.08** | both strongly low-biased |
| spectrum high-half energy (bands 14–27) | **0.25** | **0.00** | |
| peak band, 110 Hz | **6 / 28** | — | bands 0–6 tie at 1.0 (see resolution) |

Two entirely independent implementations — a rodio `Source` tap and a Web Audio `AnalyserNode` — agreeing within 2% is the substance of the cross-platform claim.

### The band values that explained the "bug that wasn't"

Printed at the loudest moment of a 110 Hz tone, 28 bands:

```
[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.955, 0.794, 0.638, 0.536, 0.434, 0.352,
 0.274, 0.130, 0.064, 0.017, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0097, 0.0097, 0.0, 0.0, 0.0]
```

Bands 0–6 are **exactly 1.0** because `edge(b) = 512^(b/32)` truncates to bin 1 for b=0..3 and bins 2–3 for b=4..6 — and at 1024 points / 44.1 kHz (~43 Hz per bin) a 110 Hz tone lives in bins 2–3. The display was telling the truth.

### Test counts

| Point | Lib tests | Added | Where |
|---|---|---|---|
| Session start (v0.135.2) | 1297 | — | — |
| After `Audio::levels` (v0.136.0) | **1316** | +19 | `audio_analysis` 6, `audio/analysis` 7, `audio/tests` 6 |
| After `Audio::bands` (v0.137.0) | **1338** | +22 | `audio_analysis` +9, `audio/spectrum` 8, `audio/tests` +5 |
| Audio suite alone (final) | **93** | — | `cargo test --lib audio` |

All device-free except the pre-existing `_when_device_exists` guarded set, which returns early with no device.

### The 14-smoke sweep — full result

| Script | Kind | Exit |
|---|---|---|
| `headless_screenshot_smoke.sh` | assertion (non-blank PNG) | 0 |
| `hot_reload_smoke.sh` | assertion (`common`→`legendary` under a foreign root) | 0 |
| `lighting_cap_smoke.sh` | assertion (cap 16 → 210402 px vs cap 40 → 487954 px, >1.5×) | 0 |
| `packaged_assets_smoke.sh` | assertion (2 rows resolved from `/`, 2 missing files reported) | 0 |
| `bloom_web_smoke.sh` | verdict `BLOOM_WEB_CHECK: PASS (1/1)` | 0 |
| `render_format_query_smoke.sh` | verdict `PASS (2/2)` | 0 |
| `wasm_audio_smoke.sh` | verdict `AUDIO_CHECK: PASS (38/38)` | 0 |
| `wasm_save_smoke.sh` | verdict `SAVE_CHECK: PASS (7/7)` | 0 |
| `audio_reactive_smoke.sh` | verdict `PASS rms=0.625 bands low=9.13 high=0.00` | 0 |
| `centered_text_smoke.sh` | **byte-size only** (147821 B) | 0 |
| `game_feel_web_smoke.sh` | **byte-size only** (66611 B) | 0 |
| `hdr_web_smoke.sh` | **byte-size only** (40263 B) | 0 |
| `wasm_smoke.sh` (coin_race) | **byte-size only** (41913 B) | 0 |
| `embedded_atlas_smoke.sh` | byte-size + structural (no image served) | 0 |

**14/14 pass, 0 failures.** 9 assert something specific; 5 are byte-size-only.

### What the eyeball added, per byte-size-only smoke

| Smoke | What the screenshot proved that the byte count could not |
|---|---|
| `hdr_web` | HDR keeps core (white) vs mid (yellow) **distinct**; LDR collapses both to one flat grey block — the whole feature claim |
| `wasm` (coin_race) | HUD reads **"You are Player #1"** ⇒ the WebSocket handshake completed; 6 coins + player render |
| `centered_text` | every label's center sits on its guide at **x=192/480/768**, incl. the off-center cases that were EW-001 |
| `game_feel` | player + 3 dummies + platform gap + HUD + legend all present |

### Session-wide file inventory

| File | Lines | Status |
|---|---|---|
| `src/audio_analysis.rs` | 347 | NEW — un-gated shared policy |
| `src/audio/analysis.rs` | 535 | NEW — native tap + `AudioManager` surface |
| `src/audio/spectrum.rs` | 268 | NEW — Hann + radix-2 FFT + band fold |
| `examples/audio_reactive/audio_reactive.rs` | 644 | NEW — the acceptance test |
| `scripts/audio_reactive_smoke.sh` | 139 | NEW — browser verdict smoke |
| `examples/audio_reactive/web/index.html` | 70 | NEW |
| `examples/audio_reactive/web/build.sh` | 33 | NEW (`100755` in git) |
| `docs/PATTERNS.md` | 378 | 332 → 378 (+46) |
| `docs/VERIFICATION.md` | 155 | +18 (smoke classification) |
| `CLAUDE.md` | 198 | 195 → 200 → **198** (tightened) |

### Public API added (the entire surface, both PRs)

| Item | Signature |
|---|---|
| `AudioLevels` | `pub struct { pub rms: f32, pub peak: f32 }` + `SILENT`, `is_silent()` |
| `DEFAULT_ANALYSIS_SMOOTHING` | `pub const f32 = 0.15` |
| `Audio::MUSIC_CHANNEL` | `pub const &'static str` (= `"__facade_music"`) |
| `enable_analysis` / `disable_analysis` | `(&mut self, channel: &str)` |
| `is_analysis_enabled` | `(&self, channel: &str) -> bool` |
| `levels` | `(&self, channel: &str) -> AudioLevels` |
| `set_analysis_smoothing` / `analysis_smoothing` | `(&mut self, f32)` / `(&self) -> f32` |
| `enable_spectrum` / `disable_spectrum` | `(&mut self, channel: &str)` |
| `bands` | `(&self, channel: &str, out: &mut [f32])` |

Mirrored on `AudioManager` (native) and `WebAudio` (wasm). **No existing type or signature changed in any of the four PRs.**

### Internal constants worth knowing

| Constant | Value | Where | Why |
|---|---|---|---|
| `ANALYSIS_WINDOW` | 1024 samples | `audio/analysis.rs` | ~10 ms at 48 kHz stereo — several updates per rendered frame |
| `SPECTRUM_FFT_SIZE` | 1024 frames | `audio/analysis.rs` | power of two (radix-2); ~21 ms |
| `SPECTRUM_BANDS` | 32 | `audio_analysis.rs` | internal resolution; `bands()` resamples to caller length |
| `MIN_DB` / `MAX_DB` | −100 / −30 | `audio_analysis.rs` | **Web Audio `AnalyserNode` defaults** — the comparability lever |
| `ANALYSER_FFT_SIZE` | 1024 | `audio_wasm.rs` | matches the native window |
| `DEFAULT_ANALYSIS_SMOOTHING` | 0.15 s | `audio_analysis.rs` | falls visibly without flicker at 60 fps |

### Session timeline (merge times, UTC+9 as recorded by `git log`)

| Time | Commit | What |
|---|---|---|
| (prior session) 12:35 | `af4573a` | #379 CLAUDE.md 207→194 + `docs/VERIFICATION.md` created |
| (prior session) 13:00 / 13:33 | `e481873` / `a42aa1a` | #380 / #381 seq-2 handoff + its own tip correction |
| **17:59** | `2f22a4f` | **#382 v0.136.0 `Audio::levels`** |
| **19:24** | `2a10a4b` | **#383 v0.137.0 `Audio::bands`** |
| **23:36** | `66715e5` | **#384 smoke classification (docs)** |
| ~23:5x | `aef39e1` | **#385 two PATTERNS rules (docs)** |

The `af4573a`→`a42aa1a` block belongs to the **parent** session; the 12-hour `/wrap` window spanned both, which is why the proposal file states the split explicitly.

### The example's headless interfaces (how to drive it without a window)

| Trigger | Effect | Exit codes |
|---|---|---|
| `AUDIO_REACTIVE_SELFTEST=1 cargo run --example audio_reactive` | native acceptance test through a **real device**; no window | `0` pass **or skipped (no device)** · `1` meter never rose · `2` never decayed · `3` spectrum empty · `4` spectrum not low-biased |
| `ENGINE_CAPTURE=<frame>:<path>[,…]` | engine-level (v0.134.0), **no example code**; runs headless and writes a PNG per listed frame | — |
| `?autostart=1` on the web page | starts without the human Start click; what the smoke uses | — |
| wasm `WebSelfCheck` system | stamps `AR_CHECK: PASS rms=<n> bands low=<n> high=<n>` into `document.title` | verdict string |

**A no-device box must SKIP, not fail** — `Audio::new()` returning `None` prints `SKIP: no audio device available` and exits **0**, because the feature cannot be exercised there and that is not a regression.

### Smoke infrastructure specifics

| Item | Value | Why |
|---|---|---|
| static-file port | `8087` (`SMOKE_PORT`) | distinct from every existing smoke's port |
| DevTools port | `9224` (`SMOKE_DBG`) | `wasm_audio_smoke.sh` already uses 9223 |
| Chrome flags | `--headless=new --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader --autoplay-policy=no-user-gesture-required --remote-debugging-port` | SwiftShader = WebGL without a GPU; the autoplay flag lets `resume()` unlock the `AudioContext` with no click |
| port guard | refuses to run if `$PORT` is already listening | an orphaned `http.server` from a prior run would serve a **different page** and the verdict poll could read a stale title |
| `python3 -m http.server --directory` | not a `( cd && python3 )` subshell | so `$!` is the python process itself; a subshell's child would survive `kill` and orphan onto the port |
| verdict poll | up to 60 × 0.5 s against `/json` | Chrome under SwiftShader often hangs on *exit* after the work is done, so it is reaped rather than waited on |

**Known cosmetic noise, deliberately not fixed:** the cleanup trap prints `Terminated: 15` lines for the backgrounded `http.server`/Chrome on exit. Every existing smoke does the same, so diverging would be inconsistent for zero benefit; the exit code is unaffected.

### Environment gotchas hit this session

| Symptom | Cause | Resolution |
|---|---|---|
| `SamplesBuffer::new(1, 48_000, …)` → `E0308` | rodio 0.22 wants `NonZero<u16>` / `NonZero<u32>` | `ChannelCount::MIN` + `const SampleRate::new(48_000)` match, copying `TONE_CHANNELS`/`TONE_RATE` |
| `web_sys::AnalyserNode` → `E0425` | web-sys is feature-gated per DOM type | add `"AnalyserNode"` to the `Cargo.toml` web-sys `features` list |
| wasm-only `dead_code` on `normalized_db` | the browser does that conversion itself | `#[cfg(not(target_arch = "wasm32"))]`, **not** `#[allow]` |
| rustdoc `unresolved link` ×1 + `private item` ×3 | `crate::audio_wasm` is cfg'd out natively; `audio_spatial` / `smooth_toward` are `pub(crate)` | de-link to plain backticks |
| rustdoc `redundant explicit link target` | my own fix wrote `[`X`](crate::X)` where the label already resolves | reference-style link definition |
| stale rust-analyzer diagnostic claimed 2 constants unused | diagnostics captured mid-edit | re-grepped + fresh `cargo build`: both used, compile clean — corrected my own read |

### Local branch state at session end

| Branch | Remote counterpart | Action |
|---|---|---|
| `main` | in sync at `aef39e1` | — |
| `docs/handoff-seq2-session-closed` and 4 siblings | present on `origin` | left alone (merged, harmless) |
| **`docs/handoff-dm-adoption-seq4`** | **none** | **deliberately NOT deleted** — no `origin/` ref, so deletion is not provably lossless |

Feature branches created and deleted this session: `feat/audio-reactive-levels`, `feat/audio-reactive-spectrum`, `docs/smoke-sweep-classification`, `docs/patterns-backend-drift-audio-thread`.

### `/wrap` proposal outcome

| # | Kind | Candidate | Outcome |
|---|---|---|---|
| 1 | skill | `add-facade-capability` (14-file dual-backend wiring) | **deferred** — n=2, same day, same axis |
| 2 | PATTERNS | shared policy for cfg-split backends | **shipped in #385** |
| 3 | PATTERNS | real-time audio-thread producers | **shipped in #385** |
| 4 | footnote | web-sys type ⇒ Cargo.toml features | not applied (low priority) |
| 5 | workflow | mitigate verify background false-green | not applied; recommended the copy-paste-block option |

---

## Code Analysis

- **`LevelSlot`** (`src/audio/analysis.rs`) — `rms: AtomicU32`, `peak: AtomicU32` (both `f32::to_bits`), `seq: AtomicU64`, `bands: [AtomicU32; 32]`, `spectrum_on: AtomicBool`. `publish()` stores values `Relaxed` then bumps `seq` with `Release`; `read()` loads the values first and `seq` last with `Acquire`, so a bumped sequence implies the values are at least that new. A torn read pairs one window's rms with the next window's peak — invisible in a meter, and documented as acceptable.
- **`LevelTap<S: Source>`** — `Iterator::next()` forwards the sample **unchanged** (a test asserts byte equality of input and output), accumulating `sum_sq`/`peak`/`count`; at `ANALYSIS_WINDOW` it publishes `sqrt(sum_sq/count)` and the peak, both `.min(1.0)`. `Source` impl delegates all four methods to `inner`, exactly like `PannedSource`.
- **`AudioManager::tapped(channel, source)`** (`pub(super)`) — returns the *same* `Box` when the channel is not analyzed. This is what makes the off path byte-identical, and it is why the insertion in `append_decoded`/`play_tone` is a single line each.
- **Tap insertion points** — `playback.rs`: in `play_tone` just before `sink.append(source)`, and in `append_decoded` immediately after the effects stage and **before** pan. Post-effects, pre-pan, pre-volume.
- **`tick_analysis(dt)`** — called from `AudioManager::update`, which `AudioSystem` (native) and `AudioFacadeSystem` (both) already tick. Reads each slot, computes `producing = seq != last_seq`, and smooths toward either the raw values or zero. Bands get the same staleness rule.
- **`smooth_toward(current, target, release_secs, dt)`** — returns `target` immediately when `target >= current` (instant attack) or when `release_secs <= 0.0 || dt <= 0.0`; otherwise `current + (target-current) * clamp(dt/release_secs, 0, 1)`. Cannot overshoot below the target on a stalled frame (test-pinned with `dt = 10.0`).
- **`spectrum_into(samples, scratch_re, scratch_im, bands)`** — copies, applies Hann, zeroes the imaginary scratch, transforms in place, then folds bins `1..n/2` into bands via `log_band_range`, taking the **peak** per band (not the mean) and passing it through `normalized_db`. Normalization is `2.0 / (n * 0.5)` — the Hann coherent-gain correction, so a full-scale sine reaches ~1.0 regardless of `n`. Bails to `bands.fill(0.0)` on any shape mismatch.
- **`log_band_range(band, n_bands, usable_bins)`** — `edge(b) = usable_bins^(b/n_bands)`, `start = clamp(edge(band), 1, usable-1)`, `end = max(edge(band+1), start+1).min(usable)`. Starts at bin 1 because bin 0 is DC and carries no pitch. Degenerate inputs return `(1, 1)`.
- **`resample_bands(src, out)`** — averages the source bands falling in each output slot (half-open `[lo, hi)`, always ≥1 wide) so downsampling to 4 bars cannot drop a spike entirely. Empty `out` is a no-op; empty `src` fills zeros.
- **`WebAudio::tap(channel, node)`** — connects `node` to the channel's analyser **in parallel** with its existing output. An `AnalyserNode` is spec'd to work as a terminal node, and the graph is still pulled through the real output path, so this is a pure observer.
- **wasm music tapping is inside the async decode closure** — `start_music` clones `analysers` into the `spawn_local` body and looks up `MUSIC_CHANNEL` *after* the gain node exists, because playback only begins post-decode.
- **`Audio` facade internals** — `inner: AudioManager` (native) / `WebAudio` (wasm) behind `cfg`, plus native-only `sfx_seq`. Every analysis method is a bare forward; the only asymmetry left is the native round-robin voice ring (`sfx_voice_channel(seq, 16)`), which is why `play_sfx` cannot be metered.
- **`MUSIC_CHANNEL` moved, not duplicated.** `audio_facade.rs` previously owned `const MUSIC_CHANNEL: &str = "__facade_music"` as a private native-only const. It is now `use crate::audio_analysis::MUSIC_CHANNEL` — because the **wasm** side needs the identical string as its music-analyser key, and two definitions of "the music channel" could disagree. Re-exported publicly as `Audio::MUSIC_CHANNEL` for discoverability.
- **The native self-test deliberately does NOT go through `App`.** A headless frame loop runs as fast as it can while the audio device advances on the wall clock, so any frame-count-based assertion would be timing noise. It instead `thread::sleep(16ms)` → `audio.update(0.016)` → read, which reproduces a game's frame cadence against real audio time. It also samples the spectrum at the **loudest moment observed** rather than at the first threshold crossing, because the first crossing catches the attack ramp where the band shape is not yet meaningful.
- **`WebSelfCheck` mechanics** — a wasm-only `System` registered by `build_app()` behind `#[cfg(target_arch = "wasm32")]`. Calls `audio.resume()` for the first 30 frames (the `AudioContext` starts suspended; the smoke's `--autoplay-policy` flag makes that sufficient), tracks `max_rms` and captures `bands` at that maximum, then stamps a verdict at `rms > 0.02` or fails out after **900 frames**. Writes to both `document.title` (machine-read) and `#status` (human-read).
- **The example's meter layout is derived, not literal.** `PEAK_BAR_Y = RMS_BAR_Y + BAR_GAP`, label Y `= bar Y + LABEL_DY`, the threshold tick spans `PEAK_BAR_Y..+BAR_H`, and the spectrum bars grow upward from `SPEC_BOTTOM` so each bar's `y` comes from its height. This is the direct fix for the first capture's bug — independent magic numbers had drifted apart.
- **`SPECTRUM_BARS = 28` is the example's choice, not the engine's.** It is just the length of the slice passed to `bands()`; the engine resamples its internal 32 to whatever the caller asks for. The constant is commented as such so nobody mistakes it for a limit.

---

## Files Changed

### Source code
- `src/audio_analysis.rs` — **NEW (347)**. `AudioLevels`, `MUSIC_CHANNEL`, `DEFAULT_ANALYSIS_SMOOTHING`, `smooth_toward`, `SPECTRUM_BANDS`, `MIN_DB`/`MAX_DB`, `normalized_db` (native-only), `log_band_range`, `resample_bands` + 15 tests.
- `src/audio/analysis.rs` — **NEW (535)**. `LevelSlot`, `LevelTap`, `AnalysisChannel`, the `impl AudioManager` analysis surface, `tapped`, `tick_analysis` + 7 tests.
- `src/audio/spectrum.rs` — **NEW (268)**. `apply_hann`, `fft_in_place`, `spectrum_into` + 8 tests.
- `src/audio.rs` — `mod analysis;` + `mod spectrum;`; two `AudioManager` fields (`analysis`, `analysis_smoothing`).
- `src/audio/playback.rs` — two `tapped()` insertions; `tick_analysis(dt)` in `update`; constructor fields; `update` doc updated.
- `src/audio_wasm.rs` — `AnalyserState`, two `WebAudio` fields, the analysis/spectrum method block, `update`, `tap`, tone-channel + music hook points, `ANALYSER_FFT_SIZE`; module doc corrected (it claimed no per-frame tick was needed).
- `src/audio_facade.rs` — 11 forwarding methods + `MUSIC_CHANNEL` const; `MUSIC_CHANNEL` now imported from `audio_analysis`; `update`'s `cfg` pair removed.
- `src/lib.rs` — `pub mod audio_analysis;` + `pub use audio_analysis::{AudioLevels, DEFAULT_ANALYSIS_SMOOTHING};`.

### Tests
- `src/audio/tests.rs` — +11 device-guarded tests (6 levels, 5 spectrum) following the `_when_device_exists` convention.

### Examples (the acceptance tests)
- `examples/audio_reactive/audio_reactive.rs` — **NEW (644)**. `build_app()` shared by native/wasm entries; `AudioReactive` demo system; native `self_test()` behind `AUDIO_REACTIVE_SELFTEST=1`; wasm-only `WebSelfCheck` stamping `document.title`.
- `examples/audio_reactive/web/{build.sh,index.html}` — **NEW**. wasm-bindgen harness, `?autostart=1` for the smoke.

### Scripts
- `scripts/audio_reactive_smoke.sh` — **NEW (139)**. Headless Chrome + `--autoplay-policy=no-user-gesture-required` + DevTools-title verdict; port guard.

### Docs / release
- `docs/PATTERNS.md` — two new architecture patterns (+46).
- `docs/VERIFICATION.md` — `audio_reactive` smoke row; the byte-size-vs-assertion classification table (+18).
- `docs/WASM_SMOKES.md` — `audio_reactive` entry, twice extended.
- `CLAUDE.md` — audio-reactive module-map row (extended twice), Core-patterns summary extended, header v1.6.232 → **v1.6.235**.
- `docs/CHANGELOG.md` — 0.136.0 and 0.137.0 entries.
- `Cargo.toml` / `Cargo.lock` — 0.135.2 → 0.136.0 → 0.137.0; `AnalyserNode` web-sys feature; `[[example]] audio_reactive`.

### Local tooling (gitignored — memory is the only record)
- `.claude/skills/ship-wasm-example/SKILL.md` — exec-bit ⚠️ block; step 5 direct invocation; wasm-bindgen version check.
- `.claude/skills/{add-feature-example,split-module,ship}/SKILL.md` — `docs/VERIFICATION.md` pointer + exit-code rule.
- `.claude/proposals/2026-07-29.md` — **NEW (111 + appended outcome section)**.

### Memory (not in any PR)
- `engine-current-state.md` — seq 200/201/202/203 prepends; menu line rewritten; 25.5 KB → 34.2 KB.
- `local-tooling-skills.md` — 2026-07-29 audit section appended.
- `MEMORY.md` — index hook refreshed four times.

---

## User Feedback & Preferences (REQUIRED)

- **The opening instruction was a paste prompt** demanding narrated onboarding in four numbered steps, and stating **"THE FIRST ACTION IS THE BOARD GATE, and it is not optional"** plus "If both are empty — likely — ASK for direction in Korean. Do NOT self-pick." Followed literally.
- **"윈도우 캡쳐 거정 사유 알려줘"** — *tell me why windowed capture was declined.* Asked as an interruption to the direction question. **Calibration: the user will pause a decision to understand a premise behind it.** Answering from the primary source (the board thread) rather than summarizing from memory was the right instinct.
- **"오디오 리액티브 훅 (추천)"** — took the recommended option from the 4-item menu.
- **"진폭 → 스펙트럼, 2단계 PR (추천)"** and **"볼륨 전 — 신호 자체의 envelope (추천)"** — took the recommendation on both design questions. **Three for three on recommendations this session; the framing (why it is the smallest coherent step / what breaks otherwise) is doing real work — keep it.**
- **"이어서 진행"** — *continue* — terse, meaning PR2. Consistent with the parent handoff's note: short instructions reference the previously presented list; keep list labels stable.
- **"다음 추천작업 알려줘"** — *tell me the next recommended task.* **Not the same as "pick one for me":** they wanted analysis and a ranked recommendation, then chose themselves. Answering with a genuine ranking (including recommending maintenance over features) was accepted immediately.
- **"웹 스모크 일제 점검 (추천)"** — again the recommendation.
- **"2번 3번 추가. 사용되지 않거나 수정 할 만한 스킬은 없어?"** — adopt proposal candidates 2 and 3, **and** an unprompted second question about skill health. **That second question found the session's sharpest defect** (`ship-wasm-example`). Calibration: the user asks audit questions that are worth taking literally and widely.
- **`/wrap` was invoked with explicit criteria** (repeated pattern → skill; new ECS/wgpu/rapier2d pattern → skill; repeated fix → CLAUDE.md rule) and **"실제 파일 수정 말고 제안서만"** — proposal only, no edits. Respected; edits happened only after the follow-up message authorized them.
- **Merge authority is standing-delegated** — squash on green CI, async auto-merge, no per-PR confirmation. Exercised on all four PRs.
- **Korean to the user, English in artifacts** — every report, question and recommendation in Korean; code, comments, commit messages, PR bodies, CHANGELOG, docs and this handoff in English. The `.claude/proposals/` file is Korean because it is a gitignored personal doc.
- **No mid-session course corrections.** After each go-ahead the session ran end-to-end (design → implement → verify → ship → PR → merge → memory) without further input.
- **Scope discipline in both directions is expected.** Out-of-scope-but-necessary work was done and *reported as such* (tightening CLAUDE.md when it hit 200/200; fixing a doc/code mismatch I had authored). Conversely, the module-map extraction decision was flagged as the user's call, not made.
- **Corrections are expected plain and immediate.** Corrected the user's "three false-green mechanisms" to six, and corrected my own claim mid-flight when a stale diagnostic suggested unused constants that were in fact used.

---

## Where We're Going

1. **Board gate FIRST, every session** — `../dungeon-merchant/docs/engine-wishlist.md` (next free **EW-012**; EW-001–011 closed and archived) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`). A newly filed request preempts everything below.
2. **If both are empty (likely): ASK — do NOT self-pick.** The menu, with audio-reactive now removed (complete):
   - **A second capstone game** — the largest and, by VISION's logic, the most valuable: the engine's surface (10 UI widgets, game-feel toolkit, 3 procgen modes, dialogue, timeline, skeletal, audio-reactive) is exercised almost entirely by single-purpose demos. Needs a genre decision first.
   - **A fourth procgen mode** (drunkard's-walk / room-accretion / Voronoi) — safest, lowest marginal value; the family (rooms/caves/mazes) plus the `roguelike` capstone already covers the area.
   - **Adopt audio-reactive in an existing example or game** — a second consumer would stress the new API in real use, per VISION's "if the API feels awkward while writing that example, fix the API before release". Smaller than a capstone.
   - **The deferred `MapGenerator` trait** — still an anti-goal; its trigger (wanting to swap generators at runtime) has not fired.
3. **This handoff + plan land as their own `docs(handoff)` PR** (repo convention), chain `board-ew-triple` seq 3. Bump memory to **seq 204** after it merges so the recorded `main @ <hash>` points at the handoff merge — and note that the recorded tip will still be one commit behind its own merge, as happened to seq 2.
4. **CLAUDE.md is at 198/200 and will breach on the next feature.** The decision to put to the user is whether the module map (80 of 198 lines) moves to `docs/MODULE_MAP.md`. **Do not make that call unilaterally** — it changes how every session navigates the codebase. This is now the second consecutive handoff to flag it.
5. **Optional, cheap:** apply `/wrap` candidates 4 and 5 (the web-sys footnote; a copy-paste block or `scripts/verify_bg.sh` wrapper for the background false-green).

---

## Risks & Blockers

- **Verification tooling that lies remains the dominant risk, and documenting it has not been sufficient.** Trap 4 fired **twice this session** despite being written down 12 h earlier. Always read the `.exit` file, check its mtime, and never trust a background completion notification's exit code.
- **The verify gate still excludes examples for wasm.** After touching an example's `cfg(target_arch = "wasm32")` path — or adding one that claims web support — build it explicitly: `cargo build --example <name> --target wasm32-unknown-unknown`.
- **`core.fileMode = false` will hide the next mode change too.** `git ls-files -s`, never `ls -l`; set with `git update-index --chmod=+x`. The `ship-wasm-example` skill was fixed this session but any *new* script is still a fresh opportunity to get this wrong.
- **CLAUDE.md has 2 lines of headroom.** The next module-map row breaches the cap.
- **`engine-current-state` memory is 34.2 KB** and grows ~1.5 KB per seq. The Read/Edit cap is ~25k tokens (~76 KB); trim the chain tail into `engine-history-archive` well before that. It hit that ceiling once already (2026-07-29) and could not be read.
- **`Render tests (lavapipe)` took 10m34s on #382** vs a ~1m20s norm, then normalized. Unexplained. If it recurs and times out, that is the first place to look.
- **2 audio tests can fail on a no-audio box** (carried gotchas) — not hit; all new tests are device-guarded or device-free.
- **The Claude-in-Chrome interactive tab renders wasm demos blank** (carried from seq 2). Use the smoke scripts, not the tab.
- **`dungeon-merchant` has no CI/branch protection** — its board is discipline, not enforcement. `rust-survivors` is paused/deprecated; do not chase compatibility.

---

## Open Questions

- **Should the module map move out of CLAUDE.md?** 80 of 198 lines, and the sole growth driver. Second handoff in a row flagging it; now genuinely forced by the 2-line headroom.
- **Why did `Render tests (lavapipe)` take 10m34s on #382?** Passed, did not recur on the next three PRs. Runner variance is the likely answer but it was not investigated.
- **Should `bands()` offer a caller-selectable FFT size?** The 43 Hz/bin resolution is now documented, but a bass-heavy visualizer might genuinely want 4096 points (≈11 Hz/bin) at the cost of latency and CPU. Nothing has asked.
- **Should `embedded_image` get a web harness?** Carried unanswered from seq 2. It builds for wasm but has no `web/` directory; symmetry argues yes, nothing has asked.
- **Is the `add-facade-capability` skill worth writing?** Deferred at n=2. The trigger is a third dual-backend capability request.

---

## Quick Start for Next Session

```bash
# Nothing is dangling — all four PRs merged, main is clean at aef39e1 (v0.137.0).

cd ~/Projects/skeleton-engine
git log --oneline -5      # expect aef39e1 at the tip (or the handoff merge above it)
git status -s             # expect clean

# 1. READ THE BOARD FIRST — both channels (a new request preempts everything)
#   ../dungeon-merchant/docs/engine-wishlist.md        (next free EW-012; EW-001-011 all
#                                                       Verified + archived, NO open requests)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md   (currently _None._)

# 2. Verify starting state — read the exit code from the FILE, and check it is FRESH.
#    Never trust a background task's completion notification: it reported "exit code 0"
#    twice this session while the file held 1 and then 101.
rm -f /tmp/v.exit /tmp/v.log
./scripts/verify.sh > /tmp/v.log 2>&1; echo "VERIFY_EXIT=$?"
# expect 0 · 151 ok groups · 1338 lib tests

# 3. Re-prove this session's feature
cargo test --lib audio                      # expect 93 passed
AUDIO_REACTIVE_SELFTEST=1 cargo run --example audio_reactive
# expect: rms ~0.62 rise, low-biased spectrum, decay to ~0.01; exit 0
scripts/audio_reactive_smoke.sh             # optional; needs Chrome + wasm-bindgen-cli

# 4. Key files to read first
#   docs/PATTERNS.md                              — the 2 NEW patterns are at the end of
#                                                   "Core architecture patterns"
#   docs/VERIFICATION.md                          — 6 traps, 3 blind spots, smoke classification
#   src/audio_analysis.rs                         — the shared policy (start here)
#   src/audio/analysis.rs                         — LevelTap + the atomics + the seq counter
#   src/audio/spectrum.rs                         — the hand-written FFT and its exact tests
#   examples/audio_reactive/audio_reactive.rs     — build_app() + both self-checks

# 5. FIRST ACTION: board gate → if empty, ASK for direction. Do NOT self-pick.
#    Menu: a 2nd capstone game (largest value, needs a genre decision), a 4th procgen mode,
#    or adopting audio-reactive in an existing game as a second consumer.
#    Windowed capture is OFF the menu — the game declined it on 2026-07-27.
```
