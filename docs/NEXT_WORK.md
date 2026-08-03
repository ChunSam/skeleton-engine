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
| **`<NAME>_SELFTEST` coverage** | **4 of 21** playable games have one (`beat_crawler`, `survivor`, + `data_anim` and `data_particles` on 2026-08-03). This is the only defense against a headline feature degrading gracefully into silence, and CI cannot supply it. Promoted out of "Standing risks" on 2026-08-03 — it was a to-do filed as a hazard, where nobody would pick it up. Pick the games whose headline feature is *invisible to a screenshot* first; the `example-selftest` skill exists for exactly this. **Next-best candidates, and the reason each is hard:** the four networking games (`coin_race`, `predict_shooter`, `orbital_dodger`, `salvage_run`) — prediction/reconciliation/AOI are the most screenshot-invisible features in the tree, but each needs its `*_server` sibling running, so the harness has to spawn a process or skip; `settings_menu` and `scene_flow` — cross-scene persistence is the documented reset footgun, but it lives in `SceneCmd::Replace`, i.e. inside `App::update`, which an example cannot call. **That is the structural rule this session found: a self-test can drive anything expressed as systems + resources, and nothing expressed as an `App` frame step.** |
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

- **Audio is outside CI entirely.** Every audio claim in v0.140–v0.143 rests on a local device.
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

Rolled off 2026-08-03 (previous session's list; durable homes verified before removing): the four
v0.143.1–v0.143.4 releases are in `docs/CHANGELOG.md`; the gate's fusion trap and the
measure-your-own-threshold habit are in `docs/VERIFICATION.md`; the `bands()` decision is at its
call site. The one lesson with **no** other home — a squash-merge leaves the original tip dangling,
so an already-landed branch reads as "ahead" and the branch graph cannot clear it for deletion —
was moved to `docs/VERIFICATION.md` as **Trap 7** rather than dropped.
