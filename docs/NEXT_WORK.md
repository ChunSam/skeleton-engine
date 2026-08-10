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

Both channels were **empty** as of 2026-08-10 (re-checked; neither has moved):

- `../dungeon-merchant/docs/engine-wishlist.md` — next free **EW-012**, unmoved since 2026-07-27
- `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — `_None._`, unmoved since 2026-07-14

Check whether they **moved** (`git log -1 --date=short -- <file>`), not whether they look empty.
A filed request preempts everything below.

## Open — engineering

**Three items, all deliberately unscheduled** — each gated on a trigger, none on a decision. A
backlog this short is the *expected* state, not a gap to fill: two programs closed in v0.150.7 and
v0.151.1, and the board gate above is empty.
Manufacturing work to fill it would be a new analysis — say so out loud and scope it, rather than
letting it arrive as "the backlog said so".

| Item | State |
|---|---|
| **4th procgen mode** (drunkard's walk) | Unchanged, still the lowest marginal value: the engine cannot fail at it, so nothing is learned. `src/mapgen.rs` already ships three generators over one shared `DungeonMap` (BSP rooms, cellular cave, perfect maze), each with its own example and each guaranteed-connected by a different mechanism. |
| **`add-facade-capability` skill** | n=5 now (the facade + native + wasm + policy-module shape has repeated that many times). Deferred; the next facade capability makes the case by itself. **Building it now would ship a skill with nothing to apply it to** — no facade capability is queued, so do it *alongside* the next one, not before. |
| **Last-seen eviction helper** (`RemoteEntities` #5) | **n=1, gated on a 2nd staleness example** — the same bar that held `SnapshotBuffer` until its 2nd call site. `salvage_run`'s AOI streaming produces **removal-by-omission**: the server never sends a `Bye`, an entity just stops appearing in snapshots, so the client infers eviction from `last_seen` + timeout. Candidate shape (`touch(key, t)` / `expired(now - timeout) -> Vec<K>`) is written up in `docs/REMOTE_ENTITIES_DESIGN.md` § *5th example*, **flagged not built**. Surfaced here 2026-08-10 because that doc was its only home — the four sibling verdicts in the same section all resolved to *keep minimal / zero engine change*, and this is the one that did not. |

### Closed — do not reopen without new information

⚠️ **All three of these sat in the table above marked closed**, which is the exact burial this file
exists to prevent: a reader scanning *Open — engineering* for work met 3 finished rows out of 5.
Kept verbatim rather than trimmed, because each carries a lesson whose only home is this row.

| Item | Verdict |
|---|---|
| **`<NAME>_SELFTEST` coverage** | **DONE — 10 of 21**, and the one real gap is now closed (`beat_crawler`, `survivor`, `data_anim`, `data_particles`, `salvage_run`, `predict_shooter`, `orbital_dodger`, `coin_race`, + `settings_menu` and `scene_flow` on 2026-08-06). The remaining 11 games' headline features are all visible in a screenshot (`sokoban`, `platformer`, `maze_escape`, `dig_quest`, `shooter`, `lit_dungeon`, `multi_terrain`, `tile_paint`, `ui_layout_editor`, `stat_editor_game`, `script_steering`), so chasing the number past 10 is effort against failures that are already visible. **Do not reopen this as a coverage target.** Durable findings from the four networked ones are in `docs/MODULE_MAP.md`'s `src/network.rs` row; the two that generalise beyond networking: **`InputState` has no public press setter**, so held input comes from `InputScript` (the `ENGINE_INPUT` replay path), keeping the real input read under test; and **assert an invariant, not an end state**, when a background process (a coin respawner, an entity spawner) can add to what you are counting. |
| **`DialogueChoice.cond` cannot express a conjunction** | ✅ **CLOSED 2026-08-10 (v0.152.0)** — `cond_all` / `cond_any` ship alongside `cond`, and `rpg_quest`'s `can_buy_lantern` workaround is deleted. ⚠️ **This row's own fix shape was impossible**, which is why it is worth reading twice: it specified `All([…])`/`Any([…])` *variants* on `DialogueCond`, "additive if the single-cond form still parses" — and in RON 0.8 those are mutually exclusive. An externally tagged enum rewrites every existing `cond: (var: …)` into `cond: Cmp((var: …))`; `#[serde(untagged)]`, the usual way out, cannot even re-read its own serialized output. **One throwaway test settled it in a minute; no amount of reading would have.** Durable homes: `docs/PATTERNS.md` § *Extend a type that is authored in RON*, `docs/CHANGELOG.md` 0.152.0. **Do not reopen for arbitrary nesting** — `a && (b || c)` is expressible, deeper trees need a helper var, and no example has asked. |
| **2026-08-07 analysis §10** | ✅ **CLOSED 2026-08-10 (v0.151.1). The whole program is finished.** Step 0 of the plan never ran, and nothing recorded that until 2026-08-08; seven shipped across v0.150.1–v0.150.4, two docs/test hygiene items closed on 2026-08-09, the unmeasured v0.150.0 fixes were all measured by v0.150.7, and the last item — `src/app/render/debug_draw.rs:34` — closed in v0.151.1. Nothing from this analysis is open. **Do not reopen it as a source of work**; a new pass would be a new analysis. What it leaves behind is `tests/per_frame_alloc.rs` and the two habits in `docs/PATTERNS.md` / `docs/VERIFICATION.md`. |

### The 2026-08-07 analysis's unverified candidates — the closed record

> **Nothing below is open** as of 2026-08-10 (v0.151.1). It is kept as the record of *how* the
> program closed, because three of its readings were reversed by measurement and one by re-deriving
> a problem the row had written off. Read it for the habits, not for work.

`plans/2026-08-07-analysis-followup.md` had **fourteen** steps, 0 through 13. Steps 1–13 all
shipped (#438–#450, v0.145.1 → v0.150.0). **Step 0 — re-running the 33 verification agents that
died on a session limit, so §10's candidates would get the adversarial pass §1–§8 got — did not
run**, and it left no trace anywhere: `docs/CODE_ANALYSIS_2026-08-07.md` §10 still says only "worth
verifying in a follow-up session", which is not a backlog. That is the burial this file exists to
prevent, so it is written down here instead.

§10 was hand-checked against the tree on **2026-08-08** rather than re-run. Its 21 bullets split
**10 / 1 / 1 / 9**:

- **10 are already closed**, four of them by the very steps that ran while §10 sat unverified —
  `serde_registry` duplicate names now warn and name both types (#442), `audio_wasm::is_channel_playing`
  answers for positional channels (#447), the GPU-particle verification blind spot is closed by
  `tests/render.rs` `gpu_particles_accumulate_across_frames` (#438), and `PATTERNS.md`'s 20-vs-22
  disagreement was amended (#444). The other six are the drift items #450 fixed: `wasm_smoke.sh` into
  CI, `selftests.sh`'s stale counts, `network/system.rs`'s non-existent `world.register_event`,
  this file's seven-vs-eight, `MODULE_MAP`'s `dig_quest`/`tile_paint` target names, and `ci.yml:176`.
- **1 became the process item above** — `ci.yml:7`, "`wasm-smokes` is not a required check". #450
  corrected the false *claim* in three docs; the *decision* it exposed is still open.
- **1 is a false positive** — `src/ron_registry.rs:11`'s "nobody registers the path with the file
  watcher". Hot reload *is* wired: `App::register_hot_reloadable` → `forward_hot_reload` →
  `HotReloadable::reload_path`, and `particle/config_set.rs` has a test pinning the canonical-path
  match. Do not re-open it.
- **9 survived, and all 9 are now closed** — the touch letterbox map and the wasm asset failure
  hook (v0.150.1), the wasm pre-open send drop and the untested wasm event queue (v0.150.2), the
  three per-frame allocation candidates (v0.150.4, measured first), and the last two docs/test
  hygiene items on 2026-08-09 (no version bump; the table below records what the second one found).

⚠️ §10's header says **23**; the section lists **21** bullets. The header is the wrong number —
count the bullets, and do not propagate either figure without counting.

| # | Where | What | Confidence |
|---|---|---|---|
| 1 | `src/input/gamepad.rs` | ~~`GamepadState` is permanently unresponsive on wasm and the type's doc comment says nothing about it~~ **DONE 2026-08-09** — the doc comment now carries a *Native only* section naming every method that stays `false` / `None` / `0.0` on wasm, and says to give web builds a keyboard or touch path. No behaviour change, no version bump. | Confirmed (docs) |
| 2 | `src/renderer/texture.rs:293` | ~~`decode_valid_png_returns_rgba` is vacuous~~ **DONE 2026-08-09 — and the vacuity was hiding a broken fixture.** Replacing `let _ = …` with a real assertion showed the "1×1 red pixel PNG (minimal valid PNG)" had a **wrong IDAT CRC** and had never decoded once, so `decode_image_bytes`' success path had *zero* coverage — only the failure path was tested. The fixture is regenerated (CRCs verified, generator command in the test), and the test now asserts dimensions and the RGBA pixel. Both assertions sabotage-verified red and reverted byte-identical. No behaviour change, no version bump. | **Confirmed** |

The three per-frame allocation candidates shipped in v0.150.4, **measured rather than read** —
all three claims were real (401 / 190 / 1 allocations per steady-state frame). Two went to zero;
`ParticleSystem` was deliberately left at one bulk allocation because the proposed `query3_mut`
fix would stop a hand-spawned `Particle` from ageing, and a particle that never ages never
despawns. The reasoning is in the test that guards it.

⚠️ **Measure before adding to this list.** `tests/per_frame_alloc.rs` exists so a per-frame
allocation claim can be settled in one command instead of another reading of the code. v0.150.5
pointed it at v0.150.0's six "fixed" and three "not addressed" claims and **reversed two of them**:
`HierarchySystem` was still allocating 200×/frame (the scratch buffers were converted; the
`add_component` write at the end of the loop was not), and `LayoutSystem` — named as an
unaddressed hot spot — measures **zero**. Reading got both backwards. It also turned up an
ECS-wide cost nobody had listed at all: `clear_change_tracking` dropped a `HashSet` per changed
entity every frame.

**The three v0.150.0 named as "not addressed" are now accounted for**, and this is where they
should have been recorded in the first place rather than only in a CHANGELOG entry — the same
burial that hid step 0:

| Item | Verdict |
|---|---|
| `src/ui/panel.rs` `LayoutSystem` | **False positive** — measures 0 over 50 panels × 8 children. Do not reopen without a measurement that disagrees. |
| `src/app/assets.rs:262` | **FIXED v0.150.6 — and it was never a measurement problem.** This row asked for a fixture (in-crate unit test or the render job) to *measure* a `pub(crate)` method the harness cannot see. But an allocation you can read off the signature does not need measuring: `image_assets_for_gpu` returned `Vec<(String, ImageAsset)>` from a per-frame call site over `Arc<str>` keys. Yielding `(&str, &ImageAsset)` deletes it in four lines, and leaves nothing to measure. Pinned in-crate by **identity** (`ptr::eq` on the key, `Arc::ptr_eq` on the pixels) — `assert_eq!` on the strings would have passed for a fresh `String`. ⚠️ **Ask "can I just delete this?" before "how do I measure this?"** — the harness's reach is not the only route to a claim. |
| `src/app/render/debug_draw.rs:34` | **FIXED v0.151.1 — and both of this row's own readings of it were wrong.** It was mis-filed as an allocation claim (it is draw-call volume, so `per_frame_alloc.rs` was the wrong instrument — that part this row got right), and then written off as not implementable because `DrawRect` has no rotation. Rotation is only needed for **diagonals**. An axis-aligned segment collapses to one quad with no renderer change, because `push_line`'s step is always `<= thickness`, so the dots' union *is* the rect — an identity. `centered_text`'s three guide columns went 825 → 3 quads/frame, a `Cross` 30 → 2, with a byte-identical capture. ⚠️ **"not implementable" was a claim about the *suggested fix*, not about the problem** — the row never asked whether a different fix existed, and the answer was three lines below in the same file, where the `Rect` arm already drew its four edges as four quads. |

✅ **All six of v0.150.0's fixes are now measured** (v0.150.7 closed the last four). Final tally
across v0.150.4 → v0.150.7: of the six claims, **three were wrong** — `HierarchySystem` and
`LocalizationSystem` never stopped allocating and were fixed properly, and `TilemapSystem` was
still allocating twice per frame. Reading got half of them backwards.

| Claim | Verdict |
|---|---|
| `AnimEffectSystem` — bus snapshot before the registry clone | **Real.** Idle frame 0; reverting the order restores 129 allocations / 9,150 B |
| `ZoneEffectSystem` — same shape | **Real.** Idle frame 0; 129 / 9,662 B reverted |
| `DialogueSystem` — `LocaleResource` clone guarded on a box with keys | **Real.** Frame cost independent of table size; unguarded it goes 8 → 809 |
| `TilemapSystem` (idle, populated) | **Was still allocating** — 2/frame, fixed in v0.150.7 |
| `HierarchySystem` / `LocalizationSystem` | Were still allocating; fixed in v0.150.5 / v0.150.4 |

⚠️ **The tilemap one is the finding worth carrying, and it is not about tilemaps.** The test that
was supposed to guard the grid clone — `tilemap_system_steady_state_does_not_allocate`, shipped in
v0.150.4 — builds a `World` with **no `Tilemap` in it**. `run` collects an empty entity list and
returns, so it never reaches the clone. It passed for four releases, and the v0.150.5 CHANGELOG
reported v0.150.0's tilemap fix "confirmed" on the strength of it. **A green must-be-zero assertion
is two claims glued together — *the code is clean* and *the code ran* — and only the second is cheap
to check.** Every fixture in that file now carries a positive control that drives the guarded path
and requires a non-zero reading; the rule is written up in `docs/VERIFICATION.md` § *a fixture that
omits the subject reads clean*, next to #456's vacuous PNG assert, which is the same family.

The two docs/test hygiene items below were closed on 2026-08-09 with no version bump — neither
changed behaviour. ⚠️ One of them was **not** the bottom of the barrel it was filed as: the
vacuous PNG test was hiding a fixture that had never decoded. **"It only changes a test" is not
the same as "it cannot find anything"** — a check that asserts nothing tells you nothing about the
check itself, and that is exactly where rot hides.

**Both follow-ups are closed** (v0.150.3). The gap was that the wasm halves of v0.150.1 and
v0.150.2 were compile-verified only, because nothing drove them — a 404 was never requested and a
pre-open send was never made. `examples/wasm_failpaths` now does both on purpose and
`scripts/wasm_failpaths_smoke.sh` reads the verdict; it gates in the `wasm-smokes` job. It is
sabotage-verified in both directions, each half reddening only for its own defect.

⚠️ **The standing lesson, which outlived the two items:** every other browser smoke passes when
nothing goes wrong, so a *failure* handler can be entirely broken with every check still green.
Two shipped that way. When adding a check, ask what it does when the thing it guards is removed —
and if a new failure path gets a handler, it belongs in `wasm_failpaths`, not in a new smoke.

## Open — process

Nothing open. **The required-check question closed on 2026-08-08**: `Browser smokes (Chrome +
swiftshader)` is now the **eighth** required context, so the only automated check that exercises
the wasm WebSocket path — and, since v0.150.3, the only one that asserts a *failure* path — can
actually block a merge. Verified against the branch-protection API before and after: the other
settings (`strict`, `enforce_admins`, force-push and deletion bans) are byte-identical, only the
context list changed. Re-read the real list rather than trusting this paragraph:
`gh api repos/ChunSam/skeleton-engine/branches/main/protection --jq '.required_status_checks.contexts'`

⚠️ **Expect ~8 min to merge, not ~4.** That job is the slowest in CI and is now on the critical
path. Reverting is one command (`-X DELETE` on the same endpoint) if the cost stops being worth it.
Measured on the two PRs that followed: 5 m 33 s and 5 m 37 s for that job against 4 m 3 s and
3 m 53 s for `Test (native)`, so it is the critical path but the ~8 min estimate was pessimistic.

⚠️ **A repo-settings change leaves no trace in the tree, so this row went stale invisibly.** It was
closed here on 2026-08-08, and the copy on `main` still said "seven contexts, and the browser-smokes
job is **not** among them" until #456 on 2026-08-09 — which found it only by running the command
this row had itself recorded. #456 also mis-dated the closure to the day it was noticed; the
decision was made on 2026-08-08, and this branch is the record of it.

The three items that used to live here — the `main`-push hook, the oversized skills, and the
required-check decision above — closed on 2026-08-04, 2026-08-04, and 2026-08-08.

## Noted — not scheduled

- **Every doc dated before 2026-06-17 cites a version *higher* than today's, and none of them is
  wrong.** The project ran a `1.0.0` → `10.7.0` SemVer line from 2026-05-26 to 2026-06-16 (ten
  majors in three weeks), then **reset to `0.11.0` on 2026-06-17** — `docs/CHANGELOG.md` § 0.11.0,
  *"Version line reset: pre-1.0"*, no code changes, full prior history preserved below it and in git
  tags. So `## 4.3.0` and `## 10.7.0` are real CHANGELOG entries that are **older** than `0.152.0`,
  and a pre-reset doc citing `v8.11.0` is not ahead of `main`. Recorded so the next person who hits
  one does not "correct" a real version into a wrong one — #464 came within a command of doing
  exactly that. **Do not go marker-ify the other pre-reset docs — they are already covered**, and
  this was checked rather than assumed. Of the **11** tracked docs last touched before the reset,
  **6** carry the date in the filename (`CODE_ANALYSIS_2026-06-16.md` and friends) and the other 5
  open with an explicit status line: `ROADMAP.md` *historical roadmap*, `ENTITY_GENERATION_V2_PLAN.md`
  *Implemented in v2.0.0*, `CODE_ANALYSIS.md` *Generated 2026-06-05*, `REMOTE_ENTITIES_DESIGN.md`
  *minimal helper shipped … deliberately deferred*, while `SKELETAL.md` is a feature reference with
  no status to go stale. `docs/HANDOFF.md` was the only one carrying **no marker of any kind**, which
  is why it alone needed one. (A first draft of this bullet claimed the others self-mark *by
  filename*; 5 of 11 do not. The conclusion held, the reason did not — and only listing them showed
  which.)

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
  - **The transferable lesson: a step's name is not its contents — and a job total is not a step
    measurement.** Two entries pointed at the networked selftests' socket work as a suspected
    timeout on nothing but the step label, and a third read three samples as bimodal before a
    fourth refuted it. Both halves are the same mistake: reading a label or an aggregate where a
    measurement was available. What finally worked was the cheapest thing on hand the whole time —
    per-line timestamps already in the CI log. **Split the step before theorising about it.**

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

- **Eight directory-based examples silently drop out of `cargo package`.** `include` lists
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
- **5 of the 16 `scripts/*_smoke.sh` stay local, deliberately.** The other **11 run in CI**: 4 native
  in v0.143.11, 5 self-verdicting browser ones in v0.143.17, `wasm_smoke.sh` in #450 — which had
  been counted among the byte-size-only ones and was not; it self-verdicts, and it is the only
  automated exercise of the wasm WebSocket *success* path — and `wasm_failpaths_smoke.sh` in
  v0.150.3, the only one that asserts a **failure** path. The remaining 5 (`centered_text`,
  `embedded_atlas`, `embedded_image`, `game_feel_web`, `hdr_web`) assert only byte sizes and are
  documented as eyeball-it — a green run would prove nothing. Reopen only if one gains a real
  assertion.
  ✅ **"run in CI" now *is* "gate"** for the browser six: their job became a required check on
  2026-08-08 (see *Open — process*). This line said the opposite until #456 on 2026-08-09. Count
  before quoting a number here; it has now been wrong twice:
  `grep -cE '^\s*[^#]*scripts/[a-z_]*_smoke\.sh' .github/workflows/ci.yml`
- **A headless capture cannot photograph a meter** — fixed dt, no wall clock. Three sessions have
  now reached for `ENGINE_CAPTURE` before remembering this.

---

## Recently closed

> **Roll-off rule:** an entry stays here for **one session**, then goes. Its durable home is
> `docs/CHANGELOG.md` (what shipped) or `docs/PATTERNS.md` / `docs/VERIFICATION.md` (what was
> learned). Without this rule the section regrows the history that was just split out.

Closed 2026-08-10 — **`docs/HANDOFF.md` presented a frozen 2026-06-16 record as current state**
(#464; docs-only, no version bump, so it has no CHANGELOG entry and the commit body is its detail).
It now carries the historical marker `docs/ROADMAP.md` and `docs/PROGRAM_HISTORY.md` already had,
and the two pointers that sent readers into it — `CLAUDE.md`'s orientation row and `FORKING.md`'s
docs table — name the live files instead. Two things worth carrying:

- ⚠️ **The wrong-looking version was right, and "fixing" it would have *introduced* the error.**
  The file says `v4.3.0` against a `Cargo.toml` at `0.152.0`, which reads as a typo. It is a real
  and *older* version — see the version-line note under *Noted — not scheduled*. One command
  settled it (`grep -n '^## 4\.3\.0' docs/CHANGELOG.md` → line 4931). **This is the fourth
  consecutive session where re-deriving a claim beat trusting it** (`debug_draw.rs:34`,
  `assets.rs:262`, the RON fix shape, this) — and the first where the claim being re-derived was
  the session's *own*, asserted ten minutes earlier. Age is not what makes a claim worth checking.
- **A commit that removes a concept leaves prose behind a few lines away.** `92e05fe` dropped
  rust-survivors as a consumer and cleaned a dangling pointer at line 121, while line 23's
  "uses this engine as a dependency" sat eight lines under its own contradiction at line 15. Same
  family as the reverted null-sink experiment whose comment survived in `scripts/selftests.sh`:
  **grep for the concept, not for the file you happened to be editing.**

Rolled off this session, having served theirs: **the `DialogueChoice.cond` conjunction gap**
(v0.152.0) — durable homes `docs/CHANGELOG.md` 0.152.0, the `src/dialogue/` row in
`docs/MODULE_MAP.md`, and `docs/PATTERNS.md` § *Extend a type that is authored in RON* — and before
it `debug_draw.rs:34` with the 2026-08-07 analysis program (v0.151.1), the RPG genre gap (v0.151.0)
and the four unmeasured v0.150.0 allocation claims (v0.150.7).

⚠️ **Two programs ended here, and the file should stay small now.** The v0.150.x measurement program
closed with v0.150.7 and the 2026-08-07 analysis with v0.151.1. Nothing from either is open. What
survives them is instruments and habits, not work: `tests/per_frame_alloc.rs` settles an allocation
claim in one command, and `docs/PATTERNS.md` carries the two rules they cost — a fail-path check is
worthless until you revert the fix and watch it go red, and a measurement is worthless until a
control proves the instrument can see anything at all.
