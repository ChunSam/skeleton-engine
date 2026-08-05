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

Both channels were **empty** as of 2026-08-05:

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

- **The `<NAME>_SELFTEST` step is the native job's dominant cost — ~300 s, 60% of it.** Three warm
  measurements of the same step: **308 s** (`30967887895`), **182 s** (`30987090731`), **300 s**
  (`30988080080`). So ~300 s is typical and **the 182 s run was the outlier, not a new normal** —
  worth stating because a single fast run is exactly what would make a later 300 s look like a
  regression. It is the right place to look if the native job ever has to get shorter, but measure
  three runs before scoping anything: one sample here was off by 40%. This is not a complaint about
  the selftests — they are what stands between CI and the real-device claims, and the four networked
  ones do genuine socket work. Recorded because the v0.143.15 measurement surfaced it and nothing
  else tracks it.

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

- **Audio is outside CI entirely, and v0.143.10 established that it stays that way.** Five CI runs
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
- **11 of the 15 `scripts/*_smoke.sh` are still local-only.** The **4 native** ones went into CI in
  v0.143.11 (34 s total); the remaining 11 are **browser** smokes and need Chrome plus a
  `wasm-bindgen-cli` matching `Cargo.lock`. Two blockers there turned out to be stale and are worth
  knowing before anyone scopes it: the scripts **already** pass
  `--use-gl=angle --use-angle=swiftshader`, so **no GPU is needed**, and `ubuntu-latest` **ships
  `google-chrome` on PATH**, which the scripts already auto-detect. The real cost is the
  `wasm-bindgen-cli` install/cache. Note also that only **9 of the 15 self-verdict**
  (`*_CHECK: PASS`, a pixel assertion); the other 6 are byte-size-only and documented as
  eyeball-it, so they are poor CI candidates regardless.
- **A headless capture cannot photograph a meter** — fixed dt, no wall clock. Three sessions have
  now reached for `ENGINE_CAPTURE` before remembering this.

---

## Recently closed

> **Roll-off rule:** an entry stays here for **one session**, then goes. Its durable home is
> `docs/CHANGELOG.md` (what shipped) or `docs/PATTERNS.md` / `docs/VERIFICATION.md` (what was
> learned). Without this rule the section regrows the history that was just split out.

Closed 2026-08-05 (this session):

- **Both parked CI timings, settled on a warm cache** (#428, v0.143.15). They were the last thing in
  *Noted* with a defined trigger, and the trigger had fired: the cache landed in #422 and five
  successful `main` runs followed.
  - **`cargo build --release`: 265 s → 59 s** (26% → 10% of the job). **No action taken, and none
    needed** — the cache alone fixed it, so it stays in the native job rather than moving to its
    own. Most of the old figure was dependency compilation, exactly as v0.143.13 predicted.
  - **`Free disk space`: removed.** It was a fixed 63 s the cache could never touch. What justified
    removing it was not the disk size but the mechanism: the June failure was a version bump missing
    the exact cache key, whose **restore-key fallback stacked a stale whole-`target/`** until the
    disk filled — and v0.143.13 replaced that cache shape outright. v0.143.15 is itself a
    `Cargo.lock`-changing bump, so its own run re-triggered the June condition and went green.
    A `df -h /` survives as a ~1 s canary; it measured **88 GB free** before the build on both runs
    since (145 G disk, 40% used) against the step comment's stale "~14 GB free on /".
  - ⚠️ **The saving is 63 s, not the 180 s the first run appeared to show.** Native read 581 s →
    401 s, but that 401 s run happened to catch the selftest step's one fast sample (182 s); the
    very next run, on identical code, was 504 s with a 300 s selftest. Steady state is 581 − 63 ≈
    **505 s**, which is the removal and nothing more. Recorded because the flattering number came
    first and would have been the easy one to bank.

Rolled off 2026-08-05 (previous session's entry; durable home verified before removing): the
`scripts/selftests.sh` audio-device correction (#426) — its lesson, *when an experiment is reverted,
grep for the prose that described it*, lives in *Standing risks* above, which also carries the
measured record that CI audio does not work.
