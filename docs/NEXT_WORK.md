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

Both channels were **empty** as of 2026-08-03:

- `../dungeon-merchant/docs/engine-wishlist.md` — next free **EW-012**, unmoved since 2026-07-27
- `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — `_None._`, unmoved since 2026-07-14

Check whether they **moved** (`git log -1 --date=short -- <file>`), not whether they look empty.
A filed request preempts everything below.

## Open — engineering

| Item | State |
|---|---|
| **`<NAME>_SELFTEST` coverage** | **7 of 21** playable games have one (`beat_crawler`, `survivor`, + `data_anim`/`data_particles`/`salvage_run`/`predict_shooter`/`orbital_dodger` on 2026-08-03/04). **`coin_race` is the last one worth doing** — after it the remaining 13 games' headline features are all visible in a screenshot (`sokoban`, `platformer`, `maze_escape`, `dig_quest`, `shooter`, `lit_dungeon`, `multi_terrain`, `tile_paint`, `ui_layout_editor`, `stat_editor_game`, `script_steering`), so **8 of 21 is the natural stopping point**, not 21. The exceptions are `settings_menu`/`scene_flow` cross-scene persistence, which stays blocked on `App::update` being crate-private. Chasing the coverage number past 8 is effort against failures that are already visible. This is the only defense against a headline feature degrading gracefully into silence, and CI cannot supply it. Promoted out of "Standing risks" on 2026-08-03 — it was a to-do filed as a hazard, where nobody would pick it up. Pick the games whose headline feature is *invisible to a screenshot* first; the `example-selftest` skill exists for exactly this. **Structural rule found 2026-08-03: a self-test can drive anything expressed as systems + resources, and nothing expressed as an `App` frame step** (`App::update` is crate-private) — which is why `settings_menu` / `scene_flow` cross-scene persistence is still out of reach even though it is the documented reset footgun. **The networking harness is now a proven pattern, not one example's trick**: `salvage_run` built it (OS-assigned port via an `*_ADDR` env override, `TcpStream` bind probe, SKIP if the binary was never built) and `predict_shooter` reused it unchanged on 2026-08-04. `orbital_dodger` and `coin_race` are the two left that can copy it — `orbital_dodger` first, since interpolation-only is the same screenshot-invisible shape and `coin_race` has no `protocol.rs` to host `server_addr()`. **Second reusable finding (2026-08-04): `InputState` has no public press setter**, so a self-test that needs held input drives `InputScript` (the `ENGINE_INPUT` replay path) rather than faking a direction — which keeps the real `read_input` under test. |
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
- **None of the 15 `scripts/*_smoke.sh` runtime web smokes is in CI** (measured 2026-08-04; the one
  `smoke` hit in `ci.yml` is a comment). This is where "it compiles for wasm" stops and "it runs on
  the web" begins, and it is the largest remaining CI gap now that the selftests and the wasm
  example builds are wired in. Cost is the blocker: headless Chrome setup plus flake risk.
- **A headless capture cannot photograph a meter** — fixed dt, no wall clock. Three sessions have
  now reached for `ENGINE_CAPTURE` before remembering this.

---

## Recently closed

> **Roll-off rule:** an entry stays here for **one session**, then goes. Its durable home is
> `docs/CHANGELOG.md` (what shipped) or `docs/PATTERNS.md` / `docs/VERIFICATION.md` (what was
> learned). Without this rule the section regrows the history that was just split out.

Closed 2026-08-03 (this session):

- **`DATA_ANIM_SELFTEST=1` and `DATA_PARTICLES_SELFTEST=1`** (v0.143.5) — the first two games taken
  off the `<NAME>_SELFTEST` backlog above, chosen because hot-reload is named in `CLAUDE.md` as
  something CI cannot run *and* is perfectly invisible to a screenshot: a sprite animating the clips
  it was born with looks exactly like one that just reloaded. Both drive a real `notify` edit
  through `poll_reloads` → `reload_path` → the game's own re-sync system. All twelve exit codes
  proven by sabotage, each reverted and the revert re-checked by `grep`.
- **The `data_particles` emit-timer comment was wrong, and the measurement is now in the code** —
  replacing the `ParticleEmitter` every frame does not "never spawn particles"; it clamps emission
  to one per tick, measured at 60/s against a configured 90/s. Corrected in place.
- **`SALVAGE_RUN_SELFTEST=1`** (v0.143.6, 2026-08-04) — the first acceptance test over the network
  stack, and the one that unblocks the other three networked games. Two of its six checks spawn the
  real server; four drive the client offline with injected protocol JSON. Three test-side lessons
  worth more than the checks: an **end-state assertion missed a flicker** (a hysteresis window
  shorter than the snapshot interval evicts a still-arriving entity *between* snapshots and
  re-spawns it, so the endpoint looks fine — the check now watches every frame); **interpolation lag
  has to be measured against where the sender was**, not against the newest snapshot, which is
  already an interval old; and `NetworkClient::connect` **dials once**, so a spawned server must be
  probed until it binds.

Rolled off 2026-08-03 (previous session's list; durable homes verified before removing): the four
v0.143.1–v0.143.4 releases are in `docs/CHANGELOG.md`; the gate's fusion trap and the
measure-your-own-threshold habit are in `docs/VERIFICATION.md`; the `bands()` decision is at its
call site. The one lesson with **no** other home — a squash-merge leaves the original tip dangling,
so an already-landed branch reads as "ahead" and the branch graph cannot clear it for deletion —
was moved to `docs/VERIFICATION.md` as **Trap 7** rather than dropped.
