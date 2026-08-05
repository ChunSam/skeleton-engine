# Next Work — the live backlog

> Status: living document. Derived from `docs/VISION.md` (reset 2026-05-29), under its core loop:
> **a feature is not done until a small, playable example game in `examples/` exercises it in real
> play.**
>
> **This file holds only what is still open.** The completed candidate A–O playable-examples program
> and its release/hardening follow-ups moved to **`docs/PROGRAM_HISTORY.md`** on 2026-08-03 — they
> had grown to 84% of a file named *Next* Work, so the live decisions were buried under 400 lines of
> finished ones.
>
> Session narrative belongs in commit bodies and `docs/CHANGELOG.md`, durable lessons in
> `docs/PATTERNS.md` / `docs/VERIFICATION.md`. What has no other home is the **decision backlog**
> below — and that is exactly what kept getting buried.

## Board gate — check this first, every session

Both channels were **empty** as of 2026-08-06:

- `../dungeon-merchant/docs/engine-wishlist.md` — next free **EW-012**, unmoved since 2026-07-27
- `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — `_None._`, unmoved since 2026-07-14

Check whether they **moved** (`git log -1 --date=short -- <file>`), not whether they look empty.
A filed request preempts everything below.

## Open — engineering

| Item | State |
|---|---|
| **`<NAME>_SELFTEST` coverage** | **DONE at the planned stopping point — 8 of 21** (`beat_crawler`, `survivor`, `data_anim`, `data_particles`, `salvage_run`, `predict_shooter`, `orbital_dodger`, + `coin_race` on 2026-08-04). The remaining 13 games' headline features are all visible in a screenshot (`sokoban`, `platformer`, `maze_escape`, `dig_quest`, `shooter`, `lit_dungeon`, `multi_terrain`, `tile_paint`, `ui_layout_editor`, `stat_editor_game`, `script_steering`), so chasing the number past 8 is effort against failures that are already visible. **Do not reopen this as a coverage target.** The one real gap left is `settings_menu`/`scene_flow` cross-scene persistence, which stays blocked on `App::update` being crate-private — **a self-test can drive anything expressed as systems + resources, and nothing expressed as an `App` frame step**. If that ever becomes reachable, it is worth doing *because it is the documented reset footgun*, not to move a count. Durable findings from the four networked ones are in `docs/MODULE_MAP.md`'s `src/network.rs` row; the two that generalise beyond networking: **`InputState` has no public press setter**, so held input comes from `InputScript` (the `ENGINE_INPUT` replay path), keeping the real input read under test; and **assert an invariant, not an end state**, when a background process (a coin respawner, an entity spawner) can add to what you are counting. |
| **4th procgen mode** (drunkard's walk) | Unchanged, still the lowest marginal value: the engine cannot fail at it, so nothing is learned. |
| **`add-facade-capability` skill** | n=5 now (the facade + native + wasm + policy-module shape has repeated that many times). Deferred; the next facade capability makes the case by itself. |

## Open — process

Nothing open. The two items that lived here — the `main`-push hook and the oversized skills — both
closed on 2026-08-04.

## Noted — not scheduled

- **CLOSED by removal, not by diagnosis: the native job's 130 s swing was `cargo build --examples`.**
  Kept as a record because three successive versions of this entry got the *cause* wrong while the
  *measurement* was right each time, and the shape of that mistake is worth more than the finding.
  - **What it actually was.** The step named `<NAME>_SELFTEST` swung 179–308 s, so all three
    versions hunted a flaky selftest. Splitting the step by its own log timestamps ended it: the
    9 selftests run in **15 s, rock-stable**, and every second of the swing was the build in front
    of them — 142 example targets compiled to run 9. v0.143.18 narrowed that to 14, and **the step
    is now 26 s** with the variance gone. Nothing was diagnosed; the cause was deleted.
  - **The `nproc` canary (v0.143.16) is now moot.** It was added to test a runner-core-count
    hypothesis for a step that no longer dominates. It costs ~1 s and still records real headroom
    numbers, so it stays — but **do not resume that investigation**; the question it was asked to
    settle no longer has stakes.
  - **The transferable lesson: a step's name is not its contents.** Two entries pointed at the
    networked selftests' socket work as a suspected timeout on nothing but the step label, and a
    third read three samples as bimodal before a fourth refuted it. What finally worked was the
    cheapest thing available the whole time — per-line timestamps already in the CI log. **Split
    the step before theorising about it.**

- **The local verify-gate hook's two deliberate residuals** (fixed 2026-08-03, `.claude/` is
  gitignored so this is the only tracked record). It no longer over-matches prose, because it
  ignores everything from the first `<<` onward and requires a delete at a **command position**.
  The cost: a fusion written *after* a heredoc terminator is no longer seen (over-matching was the
  costlier failure), and an inline `-m` message containing a literal command-position delete
  alongside the gate name still trips it — **put that text in a file** rather than fighting the hook.

- **The rest of the `.claude/` inventory** (gitignored, so these lines are the only tracked record
  that any of it exists; rolled off *Recently closed* on 2026-08-05 but kept here for that reason).
  Two more hooks in `.claude/settings.local.json`, both proven to fire by sabotage and checked
  against real commands for false positives: **`git commit` is denied while any `*.sh` in the index
  is not `100755`** (the trap `core.fileMode = false` hides — fixed repo-wide in v0.135.2,
  reintroduced twice in v0.143.4, re-fixed in v0.143.14), and **`main`-push blocking** with
  `--delete` exempt so remote-branch cleanup still works (a branch named `maintenance-branch` does
  not trip the matcher). Skills: `handoff`, `wrap` and `example-selftest` all carry their detail in
  `references/` rather than the body. **Do not record their sizes here** — that number was wrong
  twice in a row in opposite directions (`wc -c` bytes against a character guideline, then a
  correct `wc -m` that was already stale at merge). The durable form is the command:
  `for f in ~/.claude/skills/*/SKILL.md; do wc -m "$f"; done`.

- **Seven directory-based examples silently drop out of `cargo package`.** `include` lists
  `examples/*.rs`, not `examples/*/*.rs`, so `embedded_atlas`, `embedded_image`, `audio_facade`,
  `centered_text`, `game_feel`, `web_audio` and `wasm_save` are warned-about-and-skipped. CI stays
  green because a skipped target is a warning. **Do not "fix" this by widening `include`** — it
  would break `cargo package`, since those examples `include_bytes!` from `examples/assets/`, which
  is not packaged either, so the verification build would fail on a missing PNG. Fixing it properly
  means packaging the assets too, and the engine is unpublished by design, so the payoff is zero.
  Recorded so the next person who notices the warnings does not spend a session on them.

## Known-unfalsifiable checks — do not mistake these for guarantees

- **`BEAT_CRAWLER_SELFTEST` exit `8`** ("the two meters are not independent") **cannot fail on
  native.** Each meter is a tap on its own channel, so the spectrum read never sees the mixer
  output — verified by firing the bass-heavy soundtrack as the impact clip and measuring no
  change at all. It is a tripwire for the **wasm** topology, where several sources share one
  `AnalyserNode`. Only its lower bound (the clock keeps working while impacts sound) guards
  anything today.

## Standing risks

Context for judging new work — not to-dos. Anything here that becomes actionable belongs in
**Open — engineering** instead; that is where `<NAME>_SELFTEST` coverage went on 2026-08-03.

- **NATIVE audio is outside CI, and v0.143.10 established that it stays that way — but "audio" was
  always too broad a word for it.** v0.143.17 put **Web Audio** under gate: `wasm_audio` (38/38) and
  `audio_reactive` (`rms=0.643`, bands `low=9.41` / `high=0.00` on a 110 Hz tone — real spectral
  discrimination) both pass in CI, because Chrome renders the graph in software and no hardware
  device is involved. The rule below is about **rodio/ALSA**, which does need one. Keep the two
  apart: "audio cannot be tested in CI" is now false as stated, and the browser half is the
  counter-example. Five CI runs
  tried a PulseAudio null sink (default and at 30 ms latency) and ALSA `snd-dummy`; the full table is
  in `docs/VERIFICATION.md`. Summary: a null sink *does* let rodio open a device and `beat_crawler`'s
  audio chain passes on CI, but it delivers samples in bursts, so the meters with sub-second
  deadlines read silence. `snd-dummy` does not exist on the runner kernel. **Do not re-litigate
  without new information** — a runner image with a real or dummy ALSA card would be new
  information; another sink tweak is not. `SKELETON_REQUIRE_AUDIO=1` exists so a *local* run can
  prove its audio checks ran rather than skipped. ⚠️ **`scripts/selftests.sh`'s own header claimed
  the opposite** ("CI provisions a PulseAudio null sink") from v0.143.10 until #426 on 2026-08-05 —
  the sentence was written for the null-sink experiment and survived its revert *in the same
  commit*. `ci.yml` and `docs/VERIFICATION.md` were right the whole time; only the file a reader
  actually opens was wrong. **When an experiment is reverted, grep for prose that described it** —
  the revert diff will not show you the comment three files away.
- **6 of the 15 `scripts/*_smoke.sh` stay local, deliberately.** The other **9 all gate** now: the
  4 native in v0.143.11, the 5 self-verdicting browser ones in v0.143.17. The remaining 6 assert
  only byte sizes and are documented as eyeball-it — a green run would prove nothing, so adding
  them would grow the gate without growing what it covers. **This is the finished state, not a
  backlog item**; reopen it only if one of the 6 gains a real assertion.
- **A headless capture cannot photograph a meter** — fixed dt, no wall clock. Three sessions have
  now reached for `ENGINE_CAPTURE` before remembering this.

---

## Recently closed

> **Roll-off rule:** an entry stays here for **one session**, then goes. Its durable home is
> `docs/CHANGELOG.md` (what shipped) or `docs/PATTERNS.md` / `docs/VERIFICATION.md` (what was
> learned). Without this rule the section regrows the history that was just split out.

Closed 2026-08-05/06 (this session):

- **The browser smokes gate now** (#431, v0.143.17). The 5 that report a `*_CHECK: PASS` run in CI;
  the other 6 assert byte sizes only and stay local on purpose. Both blockers named in *Standing
  risks* were stale — the scripts already request swiftshader and already find `google-chrome` —
  and the real cost, `wasm-bindgen-cli`, ships a prebuilt musl tarball. **Web Audio came with it**:
  both audio smokes pass in CI, which split "audio cannot be tested in CI" into a native half that
  is still true and a browser half that never was.
- **The selftest step: 179–308 s → 26 s** (#432, v0.143.18). It was building 142 example targets to
  run 9; it now builds 14 (the 9 plus the 5 sibling servers they spawn), both halves derived rather
  than listed. Coverage is unchanged — `cargo test --all-targets` already compile-checks the rest.
  Native job: 8m55s → 3m50s. Also corrected v0.143.16's "~21 example binaries", which was the count
  of *games with selftests*, not example targets.
- **`SKELETON_MUTE=1`** (#433, v0.144.0). The gate played real sound from three independent sources;
  one switch silences all of them, and the proof it weakens nothing is the gate passing with
  `SKELETON_MUTE=1` **and** `SKELETON_REQUIRE_AUDIO=1` at zero skips — silent, with every audio
  check genuinely executed against the device. `set_master_volume` could not serve here: it writes
  `MASTER_BUS`, but `effective_volume` uses each channel's *own* bus, so `survivor`'s `"sfx"`
  channels would have ignored it.
- **The gate has a scope rule now** (#432). "Run it before calling anything done" was unqualified,
  and three consecutive docs/CI-only PRs each paid ~6 minutes for a run that could not have failed —
  one of them a `ci.yml` change the local gate *cannot* test at all. See `CLAUDE.md`.

Rolled off (durable homes verified before removing): the two parked CI timings and their 63 s
disk-cleanup removal (#428, v0.143.15) — both live in `docs/CHANGELOG.md`, and the one thing that
outlived them, *a job total is not a step measurement*, is now carried by the `cargo build
--examples` entry in *Noted* above, which is the case that proved it twice over.
