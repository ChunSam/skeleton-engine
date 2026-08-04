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

Both channels were **empty** as of 2026-08-04:

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

- **`main`-push blocking hook** — proposed 2026-08-03, not applied, low priority (no observed
  violation). Lives only in `.claude/proposals/2026-08-03.md`, which is gitignored.
- **`handoff` / `wrap` skills exceed the 800-char guideline** — split the detail into reference
  files. **Measured 2026-08-03: 4,531 and 5,987 chars** (bodies alone 4,250 / 5,723), i.e. **5.7×
  and 7.5×** the guideline. The previously tracked figures (2,245 / 3,195) were roughly half that,
  so this item had been prioritised against numbers that were no longer true — re-measure before
  judging it again. They live in **`~/.claude/skills/`** (user-global), not the project `.claude/`;
  either way they are untracked, so this line is the only durable record.

## Noted — not scheduled

- **The local verify-gate hook's two deliberate residuals** (fixed 2026-08-03, `.claude/` is
  gitignored so this is the only tracked record). It no longer over-matches prose, because it
  ignores everything from the first `<<` onward and requires a delete at a **command position**.
  The cost: a fusion written *after* a heredoc terminator is no longer seen (over-matching was the
  costlier failure), and an inline `-m` message containing a literal command-position delete
  alongside the gate name still trips it — **put that text in a file** rather than fighting the hook.

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
  prove its audio checks ran rather than skipped.
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

Closed 2026-08-04 (this session):

- **`COIN_RACE_SELFTEST=1`** (v0.143.12) — the eighth and **last planned** `<NAME>_SELFTEST`, closing
  that backlog item at its stated stopping point. The first test in the tree to drive **two clients
  at once**: a contested coin has no meaning with one player, so nothing single-client could have
  reached it. Six exit codes, each proven by sabotage and each revert confirmed byte-identical
  against a pristine copy. Two findings that outlive the example: **`NetworkClient` has no readable
  outbox**, so "a message was sent" is unobservable offline and the check has to assert the
  consequence instead; and **assert an invariant, not an end state**, when something in the
  background can add to what you are counting — score deltas would have flaked on a coin respawning
  under a player's feet, so the check asserts *points gained == coins the server took away*. That
  invariant is also what caught the sharpest sabotage: dropping the server's first-claim-wins guard
  leaves both scoreboards **agreeing** on 1-1, and only the accounting sees 2 points against 1 coin.
- **The `coin_race` `protocol.rs` blocker was not real.** This item sat behind "no `protocol.rs` to
  host `server_addr()`" — but the precedent it was copying puts the `*_ADDR` override on the
  **server alone**; the client keeps dialling its constant and the self-test builds its own
  `NetworkClient`. One 3-line function in `server.rs`, no shared module. Worth remembering as a
  shape: a blocker inherited from a sibling's *structure* rather than measured against the actual
  requirement.

Rolled off 2026-08-04 (previous session's list; durable homes verified before removing): the
`DATA_ANIM`/`DATA_PARTICLES` and `SALVAGE_RUN` self-tests are in `docs/CHANGELOG.md` (v0.143.5,
v0.143.6) with their lessons in the `src/network.rs` row of `docs/MODULE_MAP.md`; the corrected
`data_particles` emit-timer measurement lives in the code it describes.
