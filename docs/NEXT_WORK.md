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
| **`<NAME>_SELFTEST` coverage** | **2 of 21** playable games have one (`beat_crawler`, `survivor`). This is the only defense against a headline feature degrading gracefully into silence, and CI cannot supply it. Promoted out of "Standing risks" on 2026-08-03 — it was a to-do filed as a hazard, where nobody would pick it up. Pick the games whose headline feature is *invisible to a screenshot* first; the `example-selftest` skill exists for exactly this. |
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

- **Four dead remote branches are still on `origin`** — `docs/english-conversion`,
  `fix/v8.1.5-scene-pop-text-wrap`, `claude/editor-ux-scene-reparent-rename-hc2bfu` (landed as
  #304) and `docs/handoff-seq7-anim-effects` (landed as #288). All four are **fully contained in
  `main`**, verified by content, not by the branch graph — the last two read as "ahead by 1" only
  because a squash-merge leaves the original tip dangling. Deleting them needs a human hand
  (`git push origin --delete <branch>`); an agent's attempt is refused by the remote-destructive
  permission gate, which is the correct behavior and not a bug to route around.

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

Closed 2026-08-03:

- **`SURVIVOR_SELFTEST=1`** (v0.143.1) — carried three sessions, done.
- **`add_system` before `set_scene` is now loud** (v0.143.2) — the silent-drop footgun that cost
  `beat_crawler` several releases of a dead headline feature now emits a warning naming the count.
- **`play_sfx_metered` has a playable-game caller** (v0.143.3) — `beat_crawler`'s melee impact,
  closing the VISION acceptance test v0.143.0 shipped without.
- **`embedded_image` web harness** (v0.143.4) — built rather than carried a seventh session. The
  image half of the byte-source story now matches the atlas half. Its smoke's byte threshold was
  *measured* against an engine-never-drew load of the same page (5,582 vs 84,639) instead of copied
  from the sibling script — a habit now written into `docs/VERIFICATION.md`.
- **The gate's fusion trap** — Trap 5 now states that the cleanup step and the background run must
  be separate *calls*, which is the part that actually failed.
- **`bands()` for a metered one-shot** — closed by its own stated rule (no use case in three
  sessions). The behavior is documented where it is actually needed, at
  `examples/games/beat_crawler/beat_crawler.rs:1179`, so the backlog row guarded nothing.
