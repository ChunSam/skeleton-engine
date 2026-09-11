# Next Work — the live backlog

> Status: living document. Derived from `docs/VISION.md` (reset 2026-05-29), under its core loop:
> **a feature is not done until a small, playable example game in `examples/` exercises it in real
> play.** ⚠️ The tree was deleted on 2026-08-19 and rebuilt as five games; **all five exist** as
> of 2026-08-21 (`platformer_game`, `rpg_quest_game`, `survivor_game`, `puzzle_grid_game`,
> `netplay_game`), so the subsystems they name meet that bar and the rest of `src/` does not — see
> the top section below.
>
> **This file holds only what is still open.** The completed candidate A–O playable-examples program
> and its release/hardening follow-ups moved to **`docs/PROGRAM_HISTORY.md`** on 2026-08-03 — they
> had grown to 84% of a file named *Next* Work, so the live decisions were buried under 400 lines of
> finished ones.
>
> Session narrative belongs in commit bodies and `docs/CHANGELOG.md`, durable lessons in
> `docs/PATTERNS.md` / `docs/VERIFICATION.md`. What has no other home is the **decision backlog**
> below — and that is exactly what kept getting buried.

## ⚠️ Top of the backlog — rebuild the examples tree (opened 2026-08-19)

**All 22 playable games and ~85 feature demos were deleted on 2026-08-19 at the maintainer's
request, to rebuild a smaller set of feature-test games from scratch.** This is the one open item
that is *not* gated on a trigger, and it outranks everything below it.

What went with them, because it was built on them:

| Deleted | Consequence |
|---|---|
| 11 `<NAME>_SELFTEST` acceptance tests + `scripts/selftests.sh` | the 11 tests are gone; **the runner is back** (phase 0, 2026-08-19) and gates **5** rebuilt selftests — 35 checks, 7 per game (phases 1-5, 2026-08-19 → 2026-08-21) |
| 16 `scripts/*_smoke.sh` (12 browser, 4 native) + the `wasm-smokes` CI job | the 16 are gone; **the job is back** (phase 5b, 2026-08-21) gating **3** rebuilt `*_web_smoke.sh` — Web Audio, the wasm WebSocket path, and the failure paths. Still no render smokes |
| `scripts/build_wasm_examples.sh` + the CI step calling it | **back** (phase 4) — 5 of the 8 example targets build for wasm, 3 declared native-only |
| `scripts/hot_reload_smoke.sh` + the `DATA_ANIM` / `DATA_PARTICLES` selftests | **covered again** — `RPG_QUEST_SELFTEST` check 6 rewrites a data table on disk and waits on wall clock for the running game to read 42. Animation and particle reload have no equivalent |

✅ **Branch protection is clean — re-verified 2026-08-21, after phase 5b restored the eighth job.**
**8 CI jobs, 8 required contexts, exact correspondence in both directions**, so neither failure mode
exists: no job without a required context (a check nobody is gated on) and no context without a job
(which blocks every merge — the v0.153.0 failure). Re-check with the API, never with this line:
`gh api repos/ChunSam/skeleton-engine/branches/main/protection --jq '.required_status_checks.contexts'`

⚠️ **Check it in both directions, and against `origin/main`'s workflow — not your working tree.**
"The list looks right" is how the drift at the bottom of this file survived, and getting the
*reference* wrong is how phase 5b briefly blocked every merge. Use:
`git show origin/main:.github/workflows/ci.yml` and diff its job `name:`s against the API's context
list, both set differences empty.

⚠️ **Order matters: land the job on `main` FIRST, then add the context.** `pull_request` workflows
run from the PR's merge ref, so a job that exists only on a feature branch reports on *that* PR and
on nothing else. Adding its required context first leaves every PR cut from `main` waiting forever
on a check its `ci.yml` cannot produce — the same dead-check state as a context for a *deleted* job,
reached from the opposite direction. That happened on 2026-08-21: the correspondence was checked
against the branch that *added* the job rather than against `main`, so it read 8/8 while `main`
still had 7. Only the PR carrying the job could merge, which is also what hid it.

⚠️ **`Browser smokes (Chrome + swiftshader)` was added with the additive endpoint**
(`POST …/protection/required_status_checks/contexts`), not by PUT-ing the whole protection object.
The full PUT resets every field it does not carry, so a hand-built body silently drops `strict`,
`enforce_admins`, or the force-push and deletion bars. Confirmed unchanged afterwards: `strict`
true, `enforce_admins` true, force-pushes and deletions still barred.

Nothing was migrated into `tests/`. Before rebuilding a game, read `docs/PROGRAM_HISTORY.md` (what
each one covered and why) and `docs/VERIFICATION.md` § *A skip is not a pass* (the runner shape —
derived, never hardcoded — that this repo already paid for twice).

**Phase progress — the rebuild is done bar the browser half.** Phases 0-1 landed 2026-08-19,
phases 2-4 on 2026-08-20, and **phase 5's game landed 2026-08-21**: `netplay_game` +
`netplay_server`, 7 selftest checks. The gate now runs **35 checks across 5 games**, plus two render
tests (docked-iris, nearest-light-cull) and `build_wasm_examples.sh`.

✅ **Phase 5b landed 2026-08-21 — the rebuild is closed.** The `wasm-smokes` job is back with
**2 of the planned 4** browser smokes, and its branch-protection context was re-added in the same
change. A browser loads this engine again for the first time since 2026-08-19.

| Smoke | Asserts | State |
|---|---|---|
| `survivor_audio_web_smoke.sh` | `Audio::levels` live **and** `Audio::bands` low-biased | ✅ rms 0.5621, low 2.733 vs high 0.009 on a 110 Hz tone |
| `netplay_web_smoke.sh` | the WebSocket handshake completed and entities streamed in | ✅ 23 entities over a browser socket |
| RPG save round-trip (AEAD `localStorage`) | the wasm save branch | ⬜ not built — the machinery now exists, so it is one page + one script |
| puzzle render at DPR=2 | a non-blank frame | ⬜ not built, and see the pixel caveat below |
| `wasm_failpaths_web_smoke.sh` | that a *broken* path is handled — a 404 reaching `asset_failures()` **and** a send before the socket opens surviving | ✅ both, verified by **reinstating** v0.150.1 and v0.150.2 |

✅ **The failure-path smoke was the one nobody planned, and it is now built** (2026-08-21).
Every other check in the tree passes when nothing goes wrong — which is exactly how v0.150.1's
broken 404 → `asset_failures()` and v0.150.2's send-before-open both shipped green, both
compile-verified only. `examples/wasm_failpaths/` takes both paths on purpose against a native echo
server.

⚠️ **Its sabotage verification is the strongest in the tree, because the bugs are real.** Rather
than inventing breakage, both historical defects were **reinstated in `src/`** and each reddened
the smoke on *its own half* while the other half kept reporting `true`:

| Reinstated | What the page reported |
|---|---|
| v0.150.1 (`record_failure` removed) | `asset_failures saw the 404: false · the pre-open send came back: true` |
| v0.150.2 (CONNECTING queue removed) | `asset_failures saw the 404: true · the pre-open send came back: false` |

⚠️ **It is `main.rs`, not `wasm_failpaths.rs`, and that is deliberate.** `scripts/selftests.sh`
defines a *game* as `examples/<name>/<name>.rs` and requires every game to carry a selftest; this is
a harness with no native behaviour, so a native selftest could only print a skip — and "a skip is
not a pass" is the rule that layout enforces. It still carries an explicit `[[example]]` block,
because `build_wasm_examples.sh` derives its list from those and a page that exists only to run in a
browser must at minimum be checked to build for one.

⚠️ **Three of four planned, plus one unplanned, is a deliberate stop.** The three built are the ones
with no coverage anywhere: Web Audio (the only *working measurement* the deletion removed), the wasm
WebSocket path (a separate implementation that had not executed a line since the deletion), and the
failure paths. The save round-trip has a native equivalent that runs every gate, and the DPR render
is the weakest of the four.

⚠️ **There is still no pixel-level browser check, and there is a reason.** Reading a wgpu canvas
back needs `preserveDrawingBuffer`, which changes how the surface is configured — so such a check
would be measuring a configuration the game does not ship. Stated as a decision rather than left as
an implied gap.

✅ **The job caught a real defect immediately.** `netplay_game`'s wasm entry point shipped in #494 as
`#[no_mangle] pub extern "C"` instead of `#[wasm_bindgen]`. It compiled, `build_wasm_examples.sh`
went green, and the generated JS contained **zero** occurrences of the function the page imports —
the game could not start at all. That is precisely the gap between "compiles for wasm" and "runs on
wasm", and no build gate can close it.

✅ **`scripts/build_wasm_examples.sh` is back** (phase 4). The list is derived from Cargo.toml's
`[[example]]` blocks, and a game that cannot build for wasm declares `NATIVE_ONLY` in its own
source. That declaration is checked **both ways** — an undeclared failure fails, and a declaration
on a game that *does* build also fails, because a stale claim hides the regression the script exists
to catch. Currently **5 of 8 example targets build for wasm**; the three declared native-only are
`platformer_game` (rapier2d has no wasm backend), `netplay_server` and `wasm_failpaths_echo_server`
(both TCP servers — tungstenite and `std::net` cannot be a browser tab, and that is not a limitation
to lift). It
runs in `verify.sh` **and** as a step in CI's existing `Build (WASM)` job, so `CLAUDE.md`'s claim
that the gate covers that job in full stays true.

✅ **Every game installs a logger — DONE v0.154.3, natively.** The engine's `error!`/`warn!` sites
had nowhere to go from v0.153.0 (which deleted `env_logger` along with its only callers) until now.
`examples/shared/logging.rs` is included by all five games and called first thing in `main`;
`env_logger` returns as a **native-only dev-dependency**, so the published crate is untouched.

⚠️ **A plain `env_logger::init()` would not have fixed it** — its default filter is `error`, which
still drops every `warn!` site, including the unregistered-event-bus warning that presents as "the
event never arrives" while the engine is fine. The default is `warn`, `RUST_LOG` overrides.

⚠️ **The row's own count was stale and is not re-pinned.** It said 88; the number is **86** at
v0.154.3 (30 `error!` + 72 `warn!` minus comment lines). Since it moves with every release, the
durable form is the command, not the figure:
`grep -rnE '(log::)?(error|warn)!\(' src --include='*.rs' | grep -vE ':\s*(//|\*)' | wc -l`

⚠️ **The browser half is still open, deliberately.** On wasm the install is a no-op: `log` needs a
browser sink (`console_log` or equivalent) this repo does not depend on, and adding one is a
dependency decision. `console_error_panic_hook` covers *panics* in a browser and nothing else, so
what is still invisible there is exactly the non-fatal half — which is also the half the three
browser smokes would most want to read. **This is the open remainder of this row**; take it with the
next wasm dependency decision, not before.

⚠️ **The selftest path cannot prove any of this, and that shaped the verification.**
`apply_input_script_env` is called from `src/app/window.rs`, which a headless `step()` loop never
enters — the first attempt at a proof ran `PUZZLE_GRID_SELFTEST=1` with a broken `ENGINE_INPUT` and
saw nothing, which looked like the fix failing and was the harness never taking the path. Proven
under `ENGINE_CAPTURE` instead, three ways: the broken run logs the `ERROR`; commenting out
`logging::init()` (**sabotage**) returns it to 0 lines; and a healthy run logs 0 `ERROR` lines
(**control** — it is not printing unconditionally).

⚠️ **Still uncovered after five games and two browser smokes**: **native audio on CI**, and any
**pixel-level** claim about the browser. The native audio check skips wherever there is no device,
i.e. every CI runner — the browser half (restored in phase 5b) is once again the only automated
audio evidence in the repo, exactly as it was from v0.143.17 to v0.153.0.

✅ **Networking is covered, and natively.** `NETPLAY_SELFTEST` runs 7 checks over the four
techniques the deleted `coin_race` / `predict_shooter` / `orbital_dodger` / `salvage_run` each owned
alone; checks 6-7 spawn the real `netplay_server` on an OS-assigned port and drive **two clients at
once**, because a contested pickup has no meaning with one. All 13 sabotages were verified to fire,
and to fire on the matching check only — the table is in `docs/VERIFICATION.md`.

⚠️ **A missing server binary is now a FAILURE (exit 8), not a skip.** The deleted tree measured the
alternative: with the server hidden the raw exit code was **0**, silently dropping the two checks
that covered the most. `scripts/selftests.sh` also treats any non-audio SKIP as a failure, so this
is belt and braces — the belt matters because running the binary directly bypasses the runner.

⚠️ **The plan's line estimates were low by roughly 2×, and all five games said so — final
tally.** `platformer_game` 1,783 lines against ~800 estimated; `rpg_quest_game` 1,885 against
~1,000; `survivor_game` 1,590 against ~900; `puzzle_grid_game` 1,193 against ~600; `netplay_game`
3,006 across three files (1,855 client + 753 server + 398 shared protocol) against ~900. The plan's
"5 games, ~4,000 lines" came in at **9,457** — still a **51%** cut from the deleted tree's 19,154,
but not the 78% the plan claimed. Nothing here was padding to cut; the estimate was optimistic, and
the acceptance half (which the plan wanted designed first) is a quarter to a third of each file.
`netplay_game` is the extreme case at 2.4× **because it is two binaries** — a client, an
authoritative server, and a wire protocol both compile their own copy of.

**Phase 2 closed the docked-transition row by measurement.** `tests/render.rs`'s
`docked_iris_is_a_circle_in_the_docked_target` photographs a mid-transition `IrisIn` through
`screenshot_editor_docked_headless_rgba` and measures the hole's width against its height. Verified
both ways: against a pre-v0.153.3 aspect it reports `hole 236x372 (ratio 0.634)`, and at a window
size where the docked RT and the surface happen to share an aspect its **control** assertion fires
instead of passing vacuously. It lives in the render job rather than the selftest because it needs a
GPU, and the selftest runner tolerates no skips.

⚠️ **Two of phase 2's checks were wrong first, and both were the same shape** — a half no sabotage
moved. The persistence check's "dropped" half passed even with `SceneVisits` wrongly registered,
because `on_enter` re-inserted it unconditionally (fixed: scenes seed-if-absent). The Push/Pop check
passed with `SceneCmd::Pop` sabotaged into `Replace`, because it drove `App::pop_scene` directly
rather than the game's Escape handler (fixed: it now drives the input script). Both are the trap
`docs/VERIFICATION.md` § *Sabotage each half separately* names; it is now three-for-three.

What phase 1 leaves for the others, stated so it is not mistaken for coverage: **no wasm build**
(rapier2d is native-only, so the platformer has no web target and the browser-smoke half of the plan
is untouched), no audio, no GPU particles, no `FloatingText`, no `ShaderMaterial`, no scene
transition, no editor. Its selftest is the only acceptance test in the repo.

## Board gate — check this first, every session

Both channels were **empty** as of 2026-09-02 (re-checked with `git log -1`; neither has moved):

- `../dungeon-merchant/docs/engine-wishlist.md` — next free **EW-012**, unmoved since 2026-07-27
- `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — `_None._`, unmoved since 2026-07-14

Check whether they **moved** (`git log -1 --date=short -- <file>`), not whether they look empty.
A filed request preempts everything below.

## Open — engineering

**Three items, all deliberately unscheduled** — each gated on a trigger, none on a decision. The
flake filed here on 2026-08-24 closed as v0.155.1 the next day and is struck through below. A fifth
(`TextCacheStats`) was added and closed the same day, 2026-08-20: it was filed as gated on a
first caller and turned out not to be, which is struck through below. **The 2026-08-18 ECS review's efficiency remainder is now
empty**: its last two items closed on 2026-08-19, one by the measurement it was gated on and one by
shipping (v0.152.6). That section is kept below as the record of what measuring did to it. The
follow-up review of that work left nine small items of its own — they have their own section below
and are **not** gated the way these three are. Neither is the 2026-08-19 **render** review, which
added a section of its own after shipping three fixes as v0.152.9. **The 2026-09-01 timing-check
review** has a section too, and it is the odd one out: its list was never written into the tree,
so the section records a gap rather than a queue. ⚠️ **The 2026-09-06 `src/ui` review was the newest
and by far the largest**: 38 rows off the first full read of the subsystem, filed before any of them
shipped. Four are panics reachable from public API and one is a permanently stuck input, so that
section — not this table — was where the next engineering work came from. **It closed on
2026-09-09 (v0.159.0), all 39 rows.** ⚠️ **The newest is now the 2026-09-11 `src/audio` review**:
9 rows off 6,670 lines, one of them measured — a crossfade leaves the incoming track at 7.1× its
mixed level and off the master bus. That section is where the next engineering work comes from.

⚠️ **This paragraph described the backlog before 2026-09-06 and is kept for its reasoning, not its
arithmetic** — the `src/ui` review put 38 rows in. A backlog this short is still the *expected* state,
not a gap to fill: two programs closed in v0.150.7 and v0.151.1, and the board gate above is empty. Manufacturing work to fill it would be a
new analysis — say so out loud and scope it, rather than letting it arrive as "the backlog said so".
The ECS rows below are **not** that: they came out of a review that shipped twelve fixes across
v0.152.1–v0.152.4, and they are what that review deliberately did not do. The render rows are the
same kind of residue, from a review the user asked for — read them as *what a full read found and
did not fix*, not as a queue that has to be drained.

| Item | State |
|---|---|
| **4th procgen mode** (drunkard's walk) | Unchanged, still the lowest marginal value: the engine cannot fail at it, so nothing is learned. `src/mapgen.rs` already ships three generators over one shared `DungeonMap` (BSP rooms, cellular cave, perfect maze), each with its own example and each guaranteed-connected by a different mechanism. |
| **`add-facade-capability` skill** | n=5 now (the facade + native + wasm + policy-module shape has repeated that many times). Deferred; the next facade capability makes the case by itself. **Building it now would ship a skill with nothing to apply it to** — no facade capability is queued, so do it *alongside* the next one, not before. |
| ~~**`TextCacheStats`** — shaped-text cache hit/miss counters~~ | **DONE v0.154.0 — and it did not need the caller after all.** The row said to build it alongside `survivor_game`'s `FloatingText`, on the reasoning that nothing else could exercise it. That was wrong in one direction: `tests/render.rs` drives real multi-frame text through a real `TextRenderer`, so the whole check — moving draw, static draw, content-changing draw, warm-up window, all three controls — fits in a render test with **no game involved**. Built as its own resource (not a `RenderStats` field, which is documented sprite-pass-only), reset in `TextRenderer::end_frame`. ⚠️ The check was **seen to fail**: reinstating `position` in the cache key gives `hits=1 misses=2` against the fixed `hits=2 misses=1`. The lesson worth keeping is that "needs a game" deserves the same re-derivation as any other filed diagnosis — the render harness reaches further than the row assumed. |
| ~~**`NETPLAY_SELFTEST` check 6 is flaky** — the contested pickup~~ | **DONE v0.155.1 — and the row's own two candidate causes were both wrong.** It named "the claim never reached the server" or "its distance validation refused a legitimate one"; the answer was that the client never *sent* one. The check stages its contested moment by writing `prediction.pos` directly, which no input backs, and the frame drains the network — and `reconcile`, which overwrites `prediction.pos` wholesale — **before** `pump_claims`. ~83 ms between snapshots against a ~16 ms frame put the erasure at roughly one teleport in five. Claims are now pumped at the staged instant, which is what the comment above already said was happening. ⚠️ A second flake was one slow machine away: `try_claim` measures from the server's copy of the ship, which was **76.5** and **96.7** px from the pickup against a reach of **120** — 23 px of margin on a legitimate claim. `APPROACH` 70 → 40 roughly doubled it — ⚠️ **superseded by v0.155.2, which put it back to 70.** Narrowing the approach bought the server margin out of the *client* one (16 px of frame travel against 46, i.e. one ~35 ms iteration), which is a second flake of the opposite shape. The ships now hold still for `SETTLE` ≈ 4 snapshots instead, so the server's copy catches up and both margins end up wider than either setting gave. ⚠️ **The first probe hid the bug** (65 clean runs with a server-side `eprintln!`; the failure returned on run 17 of 25 once it was reverted) — that lesson is in `docs/VERIFICATION.md` § *A probe that changes timing…*, along with why counting clean runs proved nothing and forcing the race did. |
| **Last-seen eviction helper** (`RemoteEntities` #5) | **Back to n=1 as of 2026-08-21** — `netplay_game` implements exactly this shape (`last_seen: HashMap<NetId, f64>` + `AOI_EVICT_SECS`, `evict_stale` in `examples/netplay_game/netplay_game.rs`), so the gate is reachable again and the row is once more waiting on a **2nd** call site, the same bar that held `SnapshotBuffer`. It read n=0 from 2026-08-19, when its one call site went with the examples tree. ⚠️ The rebuild plan *predicted* this — "**Bonus:** AOI streaming restores the `RemoteEntities` last-seen eviction gate" — and the row still went stale for a day, because phase 5a updated the sections it was editing and not the one a different section had forecast. A row that names its own trigger does not update itself. Historical detail — `salvage_run`'s AOI streaming produces **removal-by-omission**: the server never sends a `Bye`, an entity just stops appearing in snapshots, so the client infers eviction from `last_seen` + timeout. Candidate shape (`touch(key, t)` / `expired(now - timeout) -> Vec<K>`) is written up in `docs/REMOTE_ENTITIES_DESIGN.md` § *5th example*, **flagged not built**. Surfaced here 2026-08-10 because that doc was its only home — the four sibling verdicts in the same section all resolved to *keep minimal / zero engine change*, and this is the one that did not. |

### Open — the 2026-09-01 timing-check review's remainder

A review of the timing-dependent checks — `NETPLAY_SELFTEST` check 6's margins and the server waits
in the three `*_web_smoke.sh` — run on 2026-09-01 by the session that closed #526. Its verdict, as
#527 records it: the check family is sound and its reds say less than they know. **Three shipped
the same day.** v0.156.4 (#527: check 6 emits the margins it survived on, and a server that died
is reported as dead rather than slow), #528 ("item 2": `scripts/soak.sh` and the nightly
`.github/workflows/soak.yml`, so a flake gets a rate instead of an anecdote), and #530 ("item 4":
the smokes wait for the server's LISTEN, with a 10 s budget that names the server when it never
serves). #529 was not an item: `soak.yml` landed on `main` unparseable — a workflow expression
inside a shell comment — and was fixed the same day; that is Trap 10 in `docs/VERIFICATION.md`, and
`scripts/lint_workflows.py` now runs first in the `Rustdoc` job (a step, not a ninth job, so the
required set is still every job in `ci.yml`).

⚠️ **The review's own list was never written into the tree, and the tree cannot give it back.**
Searched 2026-09-02: `docs/`, `plans/`, `scripts/`, `.github/`, the five PR bodies, the commit
bodies, and this machine's session transcripts. The only trace is two commit bodies calling
themselves "Item 2" and "Item 4". #527 never numbers itself (item 1, by inference); **item 3, and
anything after 4, is unknown.** This section is therefore a gap, not a queue — filed so the next
reader does not spend a session looking for a list that is not there.

| Item | Where | What settles it |
|---|---|---|
| **Item 3 and any later item — lost.** Not scheduled. Do not reconstruct it by guessing what a timing review "would" have found: each of the three that shipped carried a measurement that reversed its own filed reasoning (#530's bind race "does not exist" — a bind delayed 3, 10 and 20 s passed against the *old* code; #528's first sabotage read 60/60 because macOS clocks in microseconds and `subsec_nanos() % 10` was always 0), and a guessed row is exactly the thing that would lack one. | — | Nothing in the tree can. A new pass over the timing checks would be a new review, scoped and named as one, **and its list goes into this file before its first item ships** (`subsystem-review` rule 7) — which is the one lesson this section exists to carry. |

**The instrument's first readings**, kept as the baseline the next red gets judged against. The
durable description is `docs/VERIFICATION.md` § *The soak — a detector, never a proof*; the
numbers below are what it has said so far, and `NETPLAY` is the only selftest that reports margins.

| Run | Machine | Result | `NETPLAY` worst of the batch: server gap · slowest frame · flight |
|---|---|---|---|
| 8 local runs, 2026-09-01 (#528's body) | macOS | margins only | 69.7 px of 120 · 38 ms of 102 · 0.7 s of 12 |
| `workflow_dispatch`, 2026-09-01 | ubuntu runner | 0/20 on all five | 69.3 px · 17 ms · 1.1 s |
| first nightly, 2026-09-02 | ubuntu runner | 0/20 on all five | 70.0 px · 17 ms · 1.0 s |

Two things the table already says: the runner's frames are *faster* than the laptop's (17 ms
against 38), and its flight is slower (1.0–1.1 s against 0.7, of a 12 s budget) — neither is near a
limit, and both are now numbers rather than a feeling. Read the latest run, not this table:
`gh run list --workflow soak.yml --limit 3`, then `gh run view <id> --log | grep '\[soak\]'`.
⚠️ A zero is the detection floor, not a clean bill: at N=20 a 15% flake is missed 4% of the time, a
5% one 36%, a 1% one 82%.

### Open — the 2026-09-06 `src/ui` review

The first full read of `src/ui`: **39 files / 11,202 lines**, every inline `mod tests` included,
by five passes reading in order (the spine in the main session, four widget families in parallel).
Filed **before the first fix ships**, per `subsystem-review` rule 7 — the 2026-09-01 timing review
above is what happens otherwise.

**39 rows. 20 of them were reproduced by running code in this session; the other 19 are
read-derived.** Every row says which, because the two are not the same evidence and this repo has
been burned by treating them alike. The reproductions were made with temporary probes that were
reverted — `git status` was clean before and after each. (The filing paragraph said "38 / 18" —
one row short of its own tables from the day it was written; count the rows, do not cite this line.)

**Shipped: all 39 rows, in eight batches** — the review is closed. The list below is kept as the record of what a
full read of an 11,202-line subsystem found and what each row cost to settle. Every row is closed, the
`VirtualJoystick` half included (v0.159.0). Previously:
**36 of the 39, in seven batches** — v0.156.27 (the four panics + two blank-render
rows), v0.156.28 (five widgets stuck when they stopped being interactable), v0.156.29 (eight rows
where a green suite was not saying what it looked like it was saying), v0.156.30 (five widgets
whose geometry contradicted their own docs), the four doc-drift rows on 2026-09-08 (prose, no
version bump), v0.156.31 (four read-derived behaviour rows), v0.157.0 (the four allocation
rows — the first release in this program to break API), and v0.158.0 (the three that needed a
maintainer decision). **Nothing in this section is open.**

**What the three decisions had in common**, worth more than any one of them: none was a defect, and
each was a place where the code had *made* a policy without writing it down — a hover derived from a
frozen cursor, opacity inferred from an alpha channel, and one enum carrying both runtime and
authored state. A review can reproduce a defect on its own; it cannot decide these, and filing them
as bugs would have shipped a guess in the maintainer's name.

**The read-derived label held up, on the one batch that tested it.** v0.156.31 took four rows
filed from reading alone and wrote the test each row specified: **all four were red on the first
run, on exactly the stated cause**, and the tooltip row's predicted `-140.0, -140.0` was the
literal failure message. That is evidence about *these* four, not a licence to stop labelling —
the reason the distinction is kept is that a wrong read costs a session, and this batch cannot
show what the label's failure rate is. The one read-derived row still open (hover staleness) is
open on a *decision*, not on doubt about the reading.

⚠️ **The subsystem's own alloc gate was measuring an early return, and the backlog recorded that as
a verdict.** See the first row: `NEXT_WORK`'s "False positive — measures 0 … do not reopen without a
measurement that disagrees" (below, § *The 2026-08-07 analysis's unverified candidates*) was written
against a test whose world has no `ViewportSize`, so `LayoutSystem::run` returned before doing
anything. That row has been corrected in place rather than left to be cited again.

⚠️ **`src/ui` is a game path, so the per-frame-allocation policy applies here** — unlike
`src/app/editor`, whose allocation rows are explicitly exempt. That is why the allocation rows below
are filed as rows rather than as notes.

#### Panics reachable from public API — REPRODUCED

| Item | Where | What settles it |
|---|---|---|
| ~~**`TextInput::insert_char` indexes `text` with the raw `cursor`, not `safe_cursor()`, so typing into a field whose `text` was assigned from code panics** — and `text_input_pass` hits this write site on every keystroke. The `safe_cursor` doc promises the read sites are handled; this is the write site it does not cover. | `src/ui/text_input.rs:216`, hit from `src/ui/system/text_input_pass.rs:83` | **REPRODUCED**: `text = "a very long previous value"; cursor = text.len(); text = "hi"; insert_char('!')` → `panicked: assertion failed: self.is_char_boundary(idx)`. Unit test asserting `"hi!"`. The existing `assigning_text_without_resetting_cursor_does_not_panic` (`:276`) builds this exact state but calls only the three read methods and `backspace`, so it stays green. → **DONE v0.156.27.** Both write sites now call `safe_cursor()` like their three siblings. |
| ~~**`TextInput::delete_forward` also uses the raw `cursor`**: its `>= len` guard catches a past-the-end cursor but not one sitting **mid-UTF-8**, so `String::drain` panics on the start bound. | `src/ui/text_input.rs:210` | **REPRODUCED**: `text = "한글"; cursor = 1; delete_forward()` → `panicked: assertion failed: self.is_char_boundary(start)`. Unit test asserting `"글"`. → **DONE v0.156.27.** ⚠️ `nav_delete` is still read at `text_input_pass.rs:58` with **no test anywhere setting it** — that half stays open. |
| ~~**`Slider` panics on an inverted or NaN range** — `new`, `Reflect::set_field("initial_value")` and the focused ←/→ nudge all call `f32::clamp`, which is documented to panic when `min > max` or either is NaN. `min`/`max` are free `pub` fields, `Reflect` writes them with no ordering check, and `#[serde(default)]` accepts `min: 1.0, max: 0.0` from a scene verbatim. | `src/ui/slider.rs:108`, `:82`; `src/ui/system/focus_pass.rs:167` | **REPRODUCED, both paths**: `set_field("max", -1.0)` then `set_field("initial_value", 10.0)` → `panicked: min > max, or either was NaN. min = 0.0, max = -1.0`; and `Slider::new(1.0, 0.0, 0.5)` panics in the constructor. → **DONE v0.156.27.** New `Slider::clamped` uses the `max/min` form `Stepper::clamped_value` already documented. |
| ~~**`ScrollView` overflows on an unclamped large `scroll_offset`**: `first` saturates to `usize::MAX` and `first + visible + 1` overflows. The idiomatic "pin to bottom" write `scroll_offset = f32::MAX` reaches it. This is the same overflow the existing `item_height == 0` guard was written for — one input was closed, the other left open. | `src/ui/system/scroll_view_pass.rs:72` | **REPRODUCED**: `scroll_offset = f32::MAX`, one frame → `panicked: attempt to add with overflow` in a debug `cargo test`. → **DONE v0.156.27.** ⚠️ The per-frame clamp fixes the reachable case; the window arithmetic is **also** saturating now, because a tiny positive `item_height`/`row_height` slips past the `<= 0.0` guard and saturates the row count on its own. That second half was not in this row — the test found it in the first version of the fix. |

#### Wrong behaviour — REPRODUCED

| Item | Where | What settles it |
|---|---|---|
| ~~**A `VirtualJoystick` sticks permanently if `update()` misses the single frame carrying the release.** The only release signal is the one-frame `ended()` buffer; a state-gated movement system, a pause, or a scene swap misses it, and then `output` keeps its last non-zero value forever, `is_active()` stays true, and **a new touch cannot claim the stick** (step 1 is gated on `touch_id.is_none()`). `touch_id` is private with no public reset. The durable signal the fix needs, `active_touches()`, is read two lines away. `update_raw` has the identical hole. | `src/ui/joystick.rs:83-91`, `:153` | **REPRODUCED**: start touch → `active=true output=(0.5, 0.0)`; miss the release frame → still `active=true output=(0.5, 0.0)`; **a fresh touch afterwards → still `(0.5, 0.0)`**. Unit test asserting `!is_active()` and `output == ZERO` after a missed release. → **DONE v0.156.28.** The release now also comes from `active_touches()`, the durable signal; `update_raw` had the identical hole and got the same fix. |
| ~~**Focus stays pinned to a dead entity handle when its slot is recycled.** `is_focusable` compares only `Entity::index()`, ignoring the generation, so the "is my focus still valid?" filter passes for a despawned widget whose index a new focusable has taken. `world.get` then returns `None` everywhere: **no focus ring, and Enter/Space/arrows do nothing**, while `f.entity = focus` writes the dead handle back every frame. Any menu that despawns-and-respawns a row reaches it — `World::spawn` pops `free_indices` FIFO. | `src/ui/system/focus_pass.rs:294`, used at `:57` | **REPRODUCED**: focus `Entity{index:0,gen:0}`, despawn it, respawn a `Button` that takes `Entity{index:0,gen:1}`, press Enter → focus is still `{index:0,gen:0}` and **`ButtonClicked` count is 0**. Unit test asserting focus drops to `None`. → **DONE v0.156.28.** The generation is part of the comparison now. |
| ~~**A `ListBox` renders completely blank after its `items` shrink.** `clamp_scroll` runs only inside the wheel branch and inside `scroll_to_selected`, never per frame, so a stale `scroll_offset` survives and the render window `first..last` is empty — background and border draw, no rows, and `row_at` returns `None` for every click, until the player happens to wheel or arrow it back. | `src/ui/system/list_box_pass.rs:91` (only clamp is `:51`) | **REPRODUCED**: `scroll_offset = 84.0` (the legal max its own wheel test asserts), then `items` replaced with 2 entries, one frame → **texts drawn: `[]`**. → **DONE v0.156.27.** `clamp_scroll` now runs every frame, not only in the wheel branch. |
| ~~**`ScrollView` has the same defect from the same cause** — `clamp_scroll` only inside the `scroll_delta != 0.0` branch. A log or inventory that clears itself goes blank until the player spins the wheel. | `src/ui/system/scroll_view_pass.rs:39` (only call site) / `:71-72` | **REPRODUCED**: 100 items at `item_height 24`, `scroll_offset = 2300.0`, `items.truncate(3)`, one frame → **texts drawn = 0**. → **DONE v0.156.27.** Same one-line shape as the `ListBox` fix above; `scroll_view_pass.rs` gained its first test module. |
| ~~**A slider drag begun while visible is never released if the node is hidden before the mouse comes up**: `if !visible { continue; }` sits above the only `dragging = false` clear. The next time the widget is shown while any button is held, the thumb jumps to the cursor and emits a `SliderChanged` the player never made. | `src/ui/system/slider_pass.rs:36` (guard) vs `:71-73` (clear) | **REPRODUCED**: press inside → `dragging=true`; hide the node and release → **`dragging` is still `true`**. Unit test asserting it clears, plus a third frame asserting the value does not jump. → **DONE v0.156.28.** ⚠️ The third phase had to be rewritten during the fix: pressing *inside* the slider to resurface it is a legitimate new drag, so the test has to re-show the widget under a button held from elsewhere — which is the frame the stale latch actually resumed on. |
| ~~**Tab away from a keyboard-opened `Dropdown` leaves it open**, and an open dropdown registers its whole `expanded_rect` in `PointerCapture` at `DROPDOWN_LIST_Z`, so the abandoned list keeps eating hover and clicks for everything drawn beneath it. Nothing closes on focus loss, and there is no Escape path (`rg -c Escape src/ui/` → no matches). | `src/ui/system/focus_pass.rs:145` | **REPRODUCED**: Tab to the dropdown, Enter → `open=true`; Tab away → focus moves to the button and **`open` is still `true`**. Unit test asserting it closes. → **DONE v0.156.28.** Compared against the *previous* focus, not "any unfocused open dropdown" — a mouse-opened list spends one frame open before the click's release moves focus onto it, and the looser rule would close it on that frame. Ships with that control as a second test. |
| ~~**A nested `Panel` lays its children out one frame late.** `LayoutSystem` snapshots every panel's position *before* it mutates any child, so an inner panel's own children use the inner panel's start-of-frame position — one frame of lag per nesting level.~~ | `src/ui/panel.rs:160-176` then `:180` | **REPRODUCED**: outer moves +100px → same frame `inner=(100,0)` but `leaf=(0,0)`; the leaf reaches `(100,0)` only on the next frame. Unit test: outer > inner > leaf, move the outer, one frame, assert the leaf followed. → **DONE v0.156.30.** Panels are laid out parents-first (breadth-first from the panels no other panel lists as a child) and each reads its position when it is processed. ⚠️ The background rect lagged with them — not in this row, found by the test. A `children` **cycle** is possible (free `pub` field) and now lays each panel out once in collect order rather than hanging the pass; that has its own test. |
| ~~**A `Switch`'s knob does not move for any `with_size` where `track_width <= track_height`** — `knob_rect` never clamps the "on" x against the track's left edge. Equal width and height freezes the knob; narrower puts it **outside the track**. The on/off affordance silently disappears while the widget still toggles.~~ | `src/ui/switch.rs:141` | **REPRODUCED**: `with_size(24.0, 24.0)` → `off_x = 3`, `on_x = 3` (no movement); `with_size(20.0, 24.0)` → `on_x = -1`, left of a track starting at 0. The existing `knob_sits_left_when_off_and_right_when_on` only exercises the default 46×24. → **DONE v0.156.30.** Clamping the "on" x alone would have kept the knob inside the track and still frozen, so the knob *diameter* is now capped at half the inner track width (`Switch::knob_diameter`, public) — it always has room to travel its own width. Defaults are byte-identical (46×24 hits the height term) and a test pins that as the control. |
| ~~**`RadioGroup`'s raw `selected` is never normalized by clicking the row that is already drawn selected** (`row != selected_index()` is false, so nothing is written), so the raw field — the one that serializes — and the drawn selection disagree indefinitely; a later `items.push` then moves the drawn selection with no `RadioChanged`. `Reflect::set_field` clamps negatives but not the upper bound, so the inspector reaches this directly.~~ | `src/ui/system/radio_group_pass.rs:57`; `src/ui/radio_group.rs:194-197` | **REPRODUCED**: `selected = 10` with 3 items → drawn index 2; clicking that very row leaves **raw `selected` at 10**. → **DONE v0.156.30.** The click writes the resolved row unconditionally; the event stays emit-on-change. ⚠️ **Left open deliberately**: the *keyboard* path (`focus_pass.rs:205-217`) has the same `if next != cur` shape, so ArrowRight at the last option still leaves an out-of-range raw index — and its three sibling widgets (Dropdown/TabBar/ListBox) share that shape, so writing on a clamped step is a policy call across four widgets, not this row's fix. |
| ~~**`ProgressBar::fraction()` returns NaN for a NaN `value`** — `f32::clamp` propagates NaN — while the doc says it "clamps to `0.0..=1.0`". The renderer itself is safe (`if fill_w > 0.0` is false for NaN), but every game reading `fraction()` for a percentage readout propagates it.~~ | `src/ui/progress_bar.rs:89`, doc at `:30-31` | **REPRODUCED**: `ProgressBar::new(f32::NAN).fraction()` → `NaN`. Unit test asserting `0.0`. → **DONE v0.156.30.** `fraction()` and `new()` both use the `max/min` form. ⚠️ Both need `#[allow(clippy::manual_clamp)]`: with literal bounds clippy proves the ordering and demands `f32::clamp` — its own note ("clamp returns NaN if the input is NaN") is the bug. `Stepper`/`Slider` escape the lint only because their bounds are runtime fields. |
| ~~**Negative `TabBar::gap` makes adjacent headers overlap**, contradicting the doc two lines above `tab_rect` — "The same geometry drives rendering and click resolution, so they can never disagree". `tab_at` uses `find` and returns the leftmost header, while the render loop draws header *i+1* over header *i* at the same z. `gap` is settable three ways and none clamps.~~ | `src/ui/tab_bar.rs:149` (doc), `:162` (`find`) | **REPRODUCED**: `with_gap(-20.0)` over a 200-wide bar → tab 0 spans x 10..120, tab 1 starts at 100, and `tab_at(110)` returns `Some(0)` in a band tab 1 covers. → **DONE v0.156.30 — floored at 0, not resolved to the topmost tab.** Making `tab_at` return the *last* match was the other candidate and it cannot be right: headers draw over the previous one but each **title** draws over the next header's background (`z + UI_SUBLAYER_Z_STEP`), so an overlap has no coherent top for click resolution to agree with. `resolved_gap()` is public. |

#### Wrong behaviour — read-derived, not yet reproduced

| Item | Where | What settles it |
|---|---|---|
| ~~**Every hover tint in the subsystem has the tooltip's staleness**, and as of v0.156.28 the signal to fix it exists. `input.cursor` freezes when the pointer leaves the window, so a hovered button/list row/tab keeps its highlight until the cursor comes back. Only the tooltip was wired to `cursor_inside`; the hover sites were left alone deliberately, as a separate behavioural call. | `capture.topmost_at(input.cursor)` call sites: `button_pass.rs`, `list_box_pass.rs`, `tab_bar_pass.rs`, `radio_group_pass.rs`, `scroll_view_pass.rs`, `stepper_pass.rs`, `switch_pass.rs`, `slider_pass.rs` | **unit test**, the same shape as `the_pointer_leaving_the_window_hides_a_shown_tooltip`: hover a button, assert the hover tint, `set_cursor_inside(false)`, assert it is gone. ⚠️ Decide first whether a *held* drag should also end when the pointer leaves — the slider's `dragging` and a press begun in-window are the cases that make this more than a repaint.~~ → **DONE v0.158.0.** Decided 2026-09-09: **hover only, the drag keeps dragging** — a press owns its widget through `press_cursor`/`release_cursor`, which is what a captured pointer does in every native toolkit. The policy lives in one `hover_owner(capture, input)` in `state.rs` rather than eight copies of `input.cursor_inside &&`. ⚠️ **This row's own site list was wrong in each direction**: `switch_pass` draws no hover tint at all, and `dropdown_pass`, which does, was not listed. Eight sites either way — read-derived, and re-derived when it shipped. |
| ~~**A blurred `TextInput` keeps its last IME preedit forever** — the preedit refresh lives inside the `if focused` block, so Tab/click away mid-composition leaves the stale string on the component, and `display_with_caret` renders it instead of the placeholder.~~ | `src/ui/system/text_input_pass.rs:100`, guarded by `:36` | **unit test**: focus, one frame with `ime_preedit = "한"`, then `focused = false` and a frame with an empty preedit; assert `preedit == ""` and the placeholder is shown. → **DONE v0.156.31.** The refresh moved out of the `if focused` block and reads `focused` **fresh**, which the row did not ask for and which matters: Enter blurs within the same frame, so the outer `focused` the block was gated on is already stale by the time the preedit is written. |
| ~~**The per-char loop does not stop at `'\n'`**: two Enters arriving in one frame emit **two** `TextSubmitted` and **two** `TextBlurred` for one blur, and any character after the `'\n'` is inserted into the now-unfocused field. Reachable because `InputState`'s char buffer accumulates across a whole frame and `dt` is capped at 0.1 s, so a stalled frame absorbs a double-tap.~~ | `src/ui/system/text_input_pass.rs:76` | **unit test**: focused field, `chars_input(vec!['\n','\n'])`; assert exactly one `TextSubmitted` and one `TextBlurred`. → **DONE v0.156.31.** Red at 2 and 2; a `break` at the first Enter. The test feeds `['\n','\n','x']` so it also pins the second half of the row — the `x` no longer reaches the blurred field. |
| ~~**`TextChanged` is emitted for input that changed nothing** — a full field (`max_len` reached, char rejected) and a backspace at cursor 0 both push an event carrying identical text. Every sibling widget in this subsystem is emit-on-change and says so; `TextChanged` is the one variant in `event.rs` without that doc, so this contradicts the convention rather than a written promise.~~ | `src/ui/system/text_input_pass.rs:85`, `:73` | **unit test**: `with_max_len(2)` holding `"ab"`, feed `'c'` → assert the `TextChanged` count is 0 (today 1). → **DONE v0.156.31.** Red at 1, as written. Both paths compare `text.len()` across the edit — an insert or a delete always moves the byte length, so the check costs nothing and no second clone. The missing doc line is now on `TextChanged`, so the convention is written down where the other ten variants write it. |
| ~~**A shown tooltip never goes away when the pointer leaves the window.** `hovered` derives purely from `input.cursor`, and the engine has no `CursorLeft` handling (`rg -c CursorLeft src/` → no matches), so the cursor freezes at its last in-window value and the box keeps drawing at `TOOLTIP_Z` over the game. | `src/ui/system/tooltip_pass.rs:41`; root cause `src/app/window.rs:292` | **eyes in a windowed run** — hover until it shows, then leave the window edge without crossing another widget. There is genuinely no unit seam: `InputState` has no representation of "outside the window", and adding one *is* the fix. → **DONE v0.156.28.** `InputState::cursor_inside` added (default `true`, so headless and virtual-cursor callers are unaffected), driven by `CursorLeft`/`CursorEntered` and by the docked-mode freeze. **The row's "eyes in a windowed run" gate was right about the code as it stood and wrong about the cost** — adding the representation turned it into a headless unit test, which is the fourth time in this program a written gate over-estimated what a row needed. |
| ~~**The tooltip box is clamped against only the right and bottom edges**, so a negative `Tooltip::offset` puts it off the left/top with no clamp, while the component doc promises it is "clamped to the viewport so it never runs off screen" unconditionally. `with_offset` is public and a negative offset is exercised in this file's own test.~~ | `src/ui/system/tooltip_pass.rs:65` | **unit test**: `.with_offset(Vec2::new(-200.0, -200.0))`, run, assert the bg rect has `x >= 0.0 && y >= 0.0` (today `-140.0, -140.0`). → **DONE v0.156.31.** Red at exactly `-140.0, -140.0`, the number the row predicted. `p = p.max(Vec2::ZERO)` after the two overflow branches; `docs/MODULE_MAP.md`'s tooltip row said two edges and now says four. |

#### Tests that cannot fail on their stated cause

| Item | Where | What settles it |
|---|---|---|
| ~~**`drag_off_the_widget_cancels_the_selection` releases at (350, 280) — outside the node — so `row_at` returns `None` on its own and the capture-ownership guard the test is named for is never exercised.** | `src/ui/system/list_box_pass.rs:290`, guard at `:58` | **REPRODUCED by sabotage**: deleting the `released_owner == Some(entity)` guard leaves all 12 tests in the module green. Fix: release *inside* the node but over a second widget that owns the pixel, so only the ownership check can stop it. → **DONE v0.156.29.** |
| ~~**`clamped_value_survives_inverted_bounds_without_panicking` never inverts the bounds** — it sets `min = 8.0` while `max` stays `10.0`, and its comment "now min > max" is wrong. Rewriting `clamped_value`'s body as `f32::clamp` (the thing its doc says it avoids) keeps this and every other test in the file green. | `src/ui/stepper.rs:343` | **unit test**: add `s.max = 3.0` so the range is genuinely inverted. Sabotage proof: the `f32::clamp` body then panics, the shipped `max/min` body returns `3.0`. → **DONE v0.156.29.** |
| ~~**Two focus tests name widget invisibility but pass through a different branch** — both hide the *only* focusable, so `collect_focusables` returns empty and the early return satisfies every assertion. Neither goes red if the visibility filter in `collect_focusables` *or* the one in `PointerCapture::rebuild` is deleted; it takes breaking both. | `src/ui/system/focus_pass.rs:935`, `:958` | **unit test**: add one visible `Button` to each fixture so `focusables` is non-empty. Both still pass today — the point is that afterwards they can fail on their stated cause. → **DONE v0.156.29.** |
| ~~**The block that clears `focused` on TextInputs outside the focusables list has zero coverage** — delete it and the suite stays green. It is the only thing that clears `ti.focused` and emits `TextBlurred` for a hidden TextInput *when another focusable exists*. Without it a hidden field keeps `focused == true`, a public field games read, and a commit-on-blur handler waits forever. | `src/ui/system/focus_pass.rs:101-115` | **unit test**: visible `Button` + focused `TextInput`, hide the TextInput, run; assert `!focused` and that `TextBlurred` was emitted. Sabotage: deleting `:101-115` reddens this and nothing else. → **DONE v0.156.29.** ⚠️ It needed no test of its own: giving the invisibility test above a second live focusable is what finally routes through this block, so one repair closed two rows. |
| ~~**`checkbox_pass::run` has no test at all** — not the click toggle, not the capture guard, not the drag-off cancel, not the render. Deleting its entire body leaves `cargo test` green. The only CheckBox behaviour test in the repo exercises the *focus* pass. | `src/ui/system/checkbox_pass.rs:13-107` | **unit tests** modelled on `radio_group_pass`'s (the only widget pass that has any): click toggles + one event; covered by a higher-z panel → no toggle; press in / release out → no toggle; one frame → 2 rects + 1 text. → **DONE v0.156.29.** |
| ~~**`label_pass::run` and `progress_bar_pass::run` have no test asserting their output either** — both bodies can be deleted without a red test; the widgets would simply stop drawing. | `src/ui/system/label_pass.rs:9-37`; `src/ui/system/progress_bar_pass.rs:13-64` | **unit tests**: a `Label` frame queues its text at the node's z; `ProgressBar::new(0.5)` on a 200-wide node queues 3 rects with a 100.0-wide fill; `new(0.0)` queues 1. → **DONE v0.156.29.** |
| ~~**The guard that stops a disabled button emitting `ButtonClicked` on a pointer click is untested** — the only Disabled test asserts Tab *focus order* and never clicks. Deleting the condition leaves every test green. | `src/ui/system/button_pass.rs:43` | **unit test**: `setup_button_world`, set `ButtonState::Disabled`, press+release inside → assert 0 clicks. → **DONE v0.156.29.** |
| ~~**The guard that force-closes an open `Dropdown` when its node goes hidden or its `items` empty has no test** — nothing in the 599-line file sets `visible = false` or clears `items` on an open dropdown. The guard is correct; it is unpinned. | `src/ui/system/dropdown_pass.rs:52` | **characterization test**: open it, set `visible = false`, run, assert `!open` and `!press_opened`; a second case clears `items` to pin the other half of the `||`. → **DONE v0.156.29.** ⚠️ **Both tests passed the first sabotage** — the verification frame did not flush the input, so the previous frame's release was still latched and the ordinary click-closes-it path ran instead of the guard. With a flush, each half of the `||` reddens its own test. |

#### Per-frame allocation — MEASURED (all four closed as v0.157.0)

| Item | Where | What settles it |
|---|---|---|
| ~~⚠️ **`layout_system_steady_state_does_not_allocate` measures an early return.** Its world has no `ViewportSize`, so `LayoutSystem::run` returns before touching a panel (`World::new` starts with an empty resource map). **Inserting `ViewportSize` into that test's own 50-panel × 8-child fixture measures 56 allocations / 9,616 bytes.** This is the measurement the "False positive — measures 0" row said it was waiting for. | `tests/per_frame_alloc.rs:349-365`; `src/ui/panel.rs:140-143` | **REPRODUCED** (exit 101 read from a file, not from the pipe). Fix the fixture; the assertion then names the real number.~~ → **DONE v0.157.0.** The fixture now inserts `ViewportSize` **and** carries a positive control in two halves — the cold frame must cost something, and a child node must actually have been positioned. Proven non-vacuous by deleting the `ViewportSize` line again: the control fires instead of the test passing green. The assertion did not end up naming 58 after all — the row below fixed the system in the same batch, so it names 0, on the path that lays panels out. |
| ~~**`LayoutSystem` allocates per frame and has no scratch fields** — `panel.children.clone()` per panel plus **four** fresh `Vec`s — while every sibling buffer in `UiSystem` is a scratch field. | `src/ui/panel.rs:165`, `:160`, `:179` | **REPRODUCED**: the fixed test above, 56/frame at 50 panels × 8 children — **58 as of v0.156.30**, re-measured with the same probe (`10,066` bytes against `9,616`). The parents-first ordering that fix needed added the two `Vec`s (`queued`, `order`), both pre-sized; the count is +2 exactly. ⚠️ The obvious fix — scratch fields — makes `LayoutSystem` stop being a unit struct, and `app.add_system(Box::new(LayoutSystem))` is what the rustdoc, `docs/PATTERNS.md` and `scene.rs`'s example all show, so this row carries an API decision, not just a refactor.~~ → **DONE v0.157.0**, and the maintainer took the API break: `LayoutSystem` holds five scratch buffers and a bare `LayoutSystem` no longer compiles as a value (`::default()`/`::new()`, matching `UiSystem` and `SteeringSystem`). The per-panel `children.clone()` went too — child lists flatten into one reused `Vec` with a `Range<usize>` per snapshot. **0 allocations** on the fixed fixture; reverting the buffers reads 12. |
| ~~**`UiSystem` is measured nowhere** (`rg -c UiSystem tests/per_frame_alloc.rs` → no matches) and allocates ~1.1× the visible widget count per frame. | `src/ui/system.rs:113` (`UiOutput::default()`), `src/ui/system/state.rs:158-163` | **REPRODUCED**, no `UiQueue`/`TextQueue` inserted so queue growth is excluded: **n=0 → 0 allocs · n=1 → 3 · n=10 → 16 · n=50 → 60 (9,210 bytes)**. Attribution: the `src/ui`-actionable half is `UiOutput`'s three fresh `Vec`s; the ~1-per-widget term is `DrawText.text: String` (`src/renderer/text/queue.rs:26`), a renderer-API cost every text-drawing pass in the engine pays — **out of this subsystem's scope, do not file it against a widget pass.**~~ → **DONE v0.157.0.** `UiOutput` is a field now, cleared per frame, and `submit_output` drains it. A frame costs **exactly one allocation per drawn text** (10 → 10, 50 → 50, 290 bytes) and the test asserts that exact count — which is what lets it see the fixed term return: reverting the reuse reads 13 and 55 (6,434 bytes). ⚠️ The first version of this test asserted only the *slope* and stayed green under that sabotage; a shape assertion was not sharp enough to hold the row. |
| ~~**`focus_pass` collects a fresh `Vec<Entity>` of every `TextInput` each frame** even though the pass owns a reusable scratch buffer — `focus_scratch` is still borrowed as `focusables` there, so the fix is a second scratch field, not a reuse. The `.drain(..)` on a `Vec` dropped one line later is a separate no-op oddity. | `src/ui/system/focus_pass.rs:103` (and `:39`) | **a delta measurement**, not `assert_no_steady_state_allocation` — a `UiSystem` frame allocates regardless (row above). Identical worlds with 0 vs 200 `TextInput`s; assert the byte difference does not scale with the count.~~ → **DONE v0.157.0**, exactly as the row specified. A second scratch buffer (not a reuse — `focus_scratch` is still the focusable list at that point), used by **both** collect sites; the no-op `.drain(..)` went with it. 200 invisible `TextInput`s now cost the same frame as none; before, one more allocation and **1,600 more bytes**. |

#### Doc drift

| Item | Where | What settles it |
|---|---|---|
| ~~**The focusable list is stale by six widget kinds and the file contradicts itself** — the row says focus cycles across "Button/TextInput/Slider/CheckBox" while `collect_focusables` extends ten kinds, and the same row later calls Stepper and Switch each "focusable as ONE stop".~~ | `docs/MODULE_MAP.md:84` vs `src/ui/system/focus_pass.rs:268-277` | **no runnable instrument** — prose, outside `verify.sh`. Both passages quoted and re-derived. → **DONE 2026-09-08 (no version bump).** ⚠️ **The row named one home and there were two**: `src/ui/focus.rs`'s module doc carried the identical four-kind list plus "activates it on Enter/Space (click a button, toggle a checkbox) or adjusts a focused Slider" — found by grepping the *claim* instead of the cited path. Both now enumerate all ten kinds and split the keys by what each actually does (activate: Button/CheckBox/Switch/Dropdown; ←/→: Slider/Switch/Dropdown/RadioGroup/TabBar/ListBox/Stepper; ↑/↓ keyboard-only: ListBox; a focused TextInput takes none of them), derived from `collect_focusables` and the activate/nav chains. |
| ~~**`LocalizedText`'s docs name Label/Button/CheckBox twice** while the system also resolves and writes `TextInput::placeholder`, which one of its own tests asserts.~~ | `src/ui/localized.rs:14`, `:61` vs `:109-110`, `:145` | **the gate**, if the fix turns the ` ```ignore ` example at `:20` into a running doctest that includes a `TextInput`. → **DONE 2026-09-08 (no version bump).** It did: the example builds a real `LocaleResource` from RON, binds a `Label` and a `TextInput`, and asserts both after a `set_locale` — `cargo test --doc` 127 → **128** passed, one fewer `ignore`d (34 → 33), measured either side of the change. The `docs/MODULE_MAP.md:86` row said three kinds too. ⚠️ **`locale_switch_updates_every_widget_kind` covered three of the four** its name claims; it now spawns the `TextInput` as well. |
| ~~**`RadioGroup::row_at`'s doc opens "The same geometry drives rendering and click resolution, so they can never disagree"** — the next two sentences then correctly describe them disagreeing, and the code confirms it (rendering loops all items unclipped; `row_at` bounds at `min(node_size.y)`). Self-correcting paragraph, false first sentence.~~ | `src/ui/radio_group.rs:144-148` vs `src/ui/system/radio_group_pass.rs:81-82` | **the gate** (prose; docs-only, no version bump). → **DONE 2026-09-08.** The sentence now says what the code does: the two share the row geometry but not the same *area*, the pass rendering unclipped while a row is clickable only inside the node rect. ⚠️ The same sentence appears in `switch.rs`, `tab_bar.rs` (×2), `stepper.rs` (×2) and `dropdown.rs` — all six re-read and **true**, each being one function feeding both paths with no extra bound. RadioGroup was the only one where `row_at` adds a check the renderer does not. |
| ~~**A garbled self-correcting editing artifact survives in a committed test comment**: "click row 2 = \"c\" at y 230..260? No — rows top-down: row 0 at 170, row 2 at 230." The test body is correct; only the comment is noise.~~ | `src/ui/system/dropdown_pass.rs:356` | **the gate**. → **DONE 2026-09-08.** Replaced with the geometry re-derived from the fixture (node at y 260, 3 items × 30 → the flipped list spans 170..260, rows top-down at 170/200/230; the click at 245 is row 2). |

#### ❌ False positives — read, judged correct, do not "fix"

- **`localized.rs` does re-resolve on a runtime locale change.** `LocaleResource::t` is a live lookup with no cache, `set_locale` only swaps the current locale, and the staleness gate compares against the freshly resolved value — so a switch marks every bound widget stale on the next frame. Already proven by `locale_switch_updates_every_widget_kind`.
- **The dropdown has no unclosable state.** While open, `expanded_rect` registers the union of box and list at `DROPDOWN_LIST_Z` with no dead band, so every press resolves: a row selects and closes, the box closes, anywhere else takes the press-away branch. `item_h == 0.0` is guarded, an empty `items` force-closes before the `len() - 1`, and a negative `item_height` falls back to the node height.
- **`press_opened` left set after a release off the widget is deliberate** and documented in two places — the list stays open and the next click on the box closes it normally.
- **The raw `selected` field being unnormalized is stated policy in all three collection widgets**, documented identically three times; every consumer reads through `selected_index()`, which clamps and returns 0 for an empty list. (The `RadioGroup` row above is filed because the *raw* field is what serializes, not because `selected_index()` is wrong.)
- **The radio group's "exactly one selected" invariant cannot be broken** — one entity owns the whole group, so two dots or zero dots is structurally unreachable, and `rows_render_rings_and_one_dot` genuinely guards it.
- **The caret blink's single `-= 0.5` cannot run away**: `dt` is capped at `FrameConfig::max_dt` = 0.1 s, so one subtraction always restores the invariant. It would take a fork raising `max_dt` above 0.5 s.
- **The stepper has no press-and-hold repeat at all**, so none of the repeat-timer questions apply — no timer, no accumulator, no doc claiming one.
- **A covered or invisible button/checkbox/radio/slider/scroll view cannot fire**: `PointerCapture::rebuild` filters `node.visible` on every kind, which is why `button_pass`'s missing visibility check in its first loop is safe.
- **`ProgressBar` and `Label` are deliberately not capture items** — text and read-only gauges must not block clicks behind them, and both `capture.rs` and `occludes`'s doc say so.
- **`PointerCapture::rebuild` reuses its allocation** (clear + extend) — measured 0 allocations at n=0.
- **`UiFocus` is correctly not registered persistent**, so a scene reset drops the focus handle; the stale-index row above is an in-scene despawn/respawn problem only.

#### Decisions, not defects — these need the maintainer

| Item | Where | Why it is a decision |
|---|---|---|
| ~~**A fully transparent `Panel` still swallows every click behind it.** `capture.rs` registers every visible Panel as pointer-opaque regardless of `background_color`'s alpha, while `CaptureItem`'s own doc calls a capture item "A visible UI surface that is **opaque** to the pointer". | `src/ui/system/capture.rs:65`, `:21-22`; `src/ui/panel.rs:214-225` | **REPRODUCED**: a button at z 0.5 under an alpha-0.0 panel at z 0.9 receives **0 clicks**. Whether an invisible layout container should block the pointer is a design call — a scrim wants this, a bare layout box does not. Unit test either way mirrors `button_covered_by_higher_z_panel_does_not_fire`.~~ → **DONE v0.158.0.** Decided 2026-09-09: a **`blocks_pointer: bool` field, default `true`** — not alpha-derived. Reading the policy out of `background_color`'s alpha would silently break an invisible scrim (a real pattern) the moment someone tuned it to zero; declaring it leaves the scrim working by default and lets a bare layout box opt out. Default reproduces the old behaviour exactly. |
| ~~**`ButtonState` is `#[serde(skip)]` and absent from `Button`'s `Reflect::fields`**, so `Disabled` is reachable only from code — a scene RON or an inspector edit cannot author a disabled button, and a saved scene reloads one as `Normal`. `button_serde_roundtrip` asserts this as intended. Likewise `VirtualJoystick` derives no `Clone`/`Serialize`/`Reflect`, so it cannot appear in a scene despite its doc saying "attach it to an entity". | `src/ui/button.rs:167-168`; `src/ui/joystick.rs` | Design consequence, not a regression. Filed so it is a choice rather than an accident.~~ → **DONE v0.158.0** for the Button half. Decided 2026-09-09: a separate **`disabled: bool`** that serializes and reflects, with `ButtonState` left as runtime-only. Two meanings of one enum became two fields; a game that writes `state = Disabled` directly still works. ⚠️ **The `VirtualJoystick` half followed as v0.159.0**, closing the row and the review: `Clone`/`Serialize`/`Reflect` derived and registered in all four places (reflect, clone, serde registry, Inspector add/remove), with the same authored/runtime split — `center`/`radius`/`visible` save, `output`/`stick_pos`/`touch_id` do not. ⚠️ Skipping `stick_pos` broke an invariant nothing else could break ("not held ⇒ knob at centre"), because it loads as `Vec2::ZERO` while `center` comes from the scene; `update`/`update_raw` re-establish it and a test pins it. The type's ` ```ignore ` rustdoc example **did not compile** — it held `world.get_mut` and `world.resource` at once, the exact conflict `update_raw` exists for — and is a running doctest now. |

⚠️ **Two stacked tooltip-bearing widgets that are both pointer-transparent** (a `Label` over a `Panel`, say) each draw a tooltip at the same `TOOLTIP_Z` and the same cursor-anchored position, overlapping unreadably — `occludes` needs a capture item with strictly greater z, and labels deliberately are not capture items. Nothing in the docs promises one tooltip at a time, so this is recorded rather than filed.

⚠️ **Process note for the next parallel review**: temporary measurement probes in the main session edited `tests/per_frame_alloc.rs` and `src/ui/*` while four read-only agents were reading the same tree, and two of them saw and reported the transient state. Nothing was mis-filed — both checked `git status` and said so — but the next such review should put probes in a worktree, or run them after the readers finish.

### Open — the 2026-09-11 `src/audio` review

The first full read of the audio subsystem: **6,670 lines / 15 files** — `src/audio.rs` + the nine
`src/audio/*.rs` + `audio_facade.rs`, `audio_wasm.rs`, `audio_analysis.rs`, `audio_spatial.rs`.
⚠️ **The scale line filed at the start of the session said 6,513 / 14 and was wrong**: it missed
`src/audio.rs`, the parent module holding `AudioManager` itself. Count the files before quoting a
number, including the one the directory listing does not show.

Filed **before the first fix ships**, per `subsystem-review` rule 7.

⚠️ **Read in full**: `audio.rs`, `types.rs`, `source.rs`, `playback.rs`, `bus.rs`, `ducking.rs`,
`positional.rs`, `effects.rs`, `audio_spatial.rs`, `audio_facade.rs`. **Read in part**:
`analysis.rs` (the tap, the manager side; not its 17 inline tests), `spectrum.rs` (the FFT and the
fold; not its 8 tests), `audio_wasm.rs` (~700 of 1,181 — the graph, the meters, the metered and
positional paths; not the music/decode tail), `audio_analysis.rs` (the doc contract and signatures,
not every body), `tests.rs` (the 43 test *names* and the crossfade group, not all 1,124 lines).
The rows below are what that read found; a claim about the parts marked "in part" is worth less
than one about the parts marked "in full", and the table says which.

**Why this subsystem.** It was picked by the measure the 2026-08-28 editor review used — tests per
1k lines — applied to everything unreviewed: `audio_facade.rs` scores **2.6** (the lowest real
density in the tree) and `audio_wasm.rs` **0.0** across 1,181 lines. It is also the exact shape
`CLAUDE.md` warns about under *cfg-split backends share the policy, not the implementation*.

⚠️ **The headline is not a row — it is what the rows sit on.** **36 of the subsystem's 109
`#[test]`s never run in CI**: they open `let Some(mut audio) = AudioManager::new() else { return; }`
and return silently wherever there is no device, which is every CI job. Sixteen of them say so in
their own names (`..._when_device_exists`). So the subsystem's apparent coverage is a third
smaller than it counts, and *"a skip is not a pass"* (`docs/VERIFICATION.md`) applies to a third of
it. The remedy is already demonstrated **four times inside this very subsystem** —
`collect_bus_names`, `read_cached_bytes`, `sfx_voice_channel` and `next_poly_voice` are each split
out as a pure function precisely so a headless machine can test them, each with a comment saying
so. Every row below whose instrument is "a pure helper" means that split, not a new harness.

| Item | Where | What settles it |
|---|---|---|
| ~~**(MEASURED) A crossfade strips the live channel's bus *and* its base volume, so the incoming track plays at full scale and the master control no longer reaches it.**~~ `begin_crossfade` moves `channel_buses[ch]` and `volume_overrides[ch]` onto the temp channel and never restores them, so `effective_volume(ch)` falls back to `1.0 × 1.0`. Reproduced on a real device: `bgm` on bus `master` at 0.2 with base 0.7 reads **effective 0.14** before `crossfade_bytes` and **1.0** after — a **7.1× jump** — and a later `set_bus_volume("master", 0.5)` leaves it at 1.0, because the channel is no longer on any bus. Through the facade this is `crossfade_music`: after one crossfade, `set_master_volume` silently stops affecting the music until the next `play_music`/`crossfade_music` re-assigns. ⚠️ The hunk's own comment describes fixing exactly this spike **for the outgoing track** ("with a master bus at 0.2 that is a 5x spike on every crossfade") — it moved the spike to the incoming one. | `src/audio/playback.rs:596-625`, `src/audio_facade.rs:448-458` | Reproduced; needs a regression test that runs in CI. The three existing crossfade tests assert only the **temp** side and that a sink exists on the channel, which is why this shipped. Extract the map surgery into a pure `fn relocate_channel_state(maps…, channel, temp)` — the `collect_bus_names` / `read_cached_bytes` split, already used twice in these two files — and assert the live channel keeps its bus and base. | → **DONE v0.159.4 — copy, do not move, and drop the copy on teardown.** `copy_channel_mixer_state` / `drop_channel_mixer_state` / `xfade_channel` / `is_xfade_channel` are pure functions beside `collect_bus_names` and `read_cached_bytes`, so **the policy is pinned by four tests that run in CI** — the first rows of this subsystem that are. ⚠️ The *wiring* (the two call sites) is still only covered by a device-gated end-to-end test, which skips in CI; that is the 36-of-109 problem above, not a new one, and the test says so in its own doc. Each of three sabotages reddened its own hunk: restoring the move reddens both policy tests and the end-to-end; removing the `is_xfade_channel` gate reddens three device-gated tests including two that predate this change; removing the teardown call reddens the end-to-end alone. |
| **(read-derived) Native and web use different stereo pan laws, ~3 dB apart at centre.** Native `PannedSource::gains` is linear — `((1-pan).clamp(0,1), (1+pan).clamp(0,1))`, so centre is `(1.0, 1.0)` and hard-left `(1.0, 0.0)`: total energy **halves** as a sound moves off centre. Web uses `StereoPannerNode`, whose spec law is equal-power — centre `(0.707, 0.707)`, hard-left `(1.0, 0.0)`: total energy constant. So the same positional sound moving centre → side gets quieter on native and holds level on web, on top of the distance attenuation the two backends *do* share. ⚠️ `audio_spatial.rs`'s module doc says it exists "so the two builds can't drift" — and the `(volume, pan)` **pair** genuinely cannot, but what each backend does with the pan afterwards is not shared. Sharing a formula is not sharing a policy. | `src/audio/source.rs:48-51`; `src/audio_wasm.rs:1116`, `:1134-1136`; doc claim at `src/audio_spatial.rs:1-5` | A pure unit test, **no device**: drive `PannedSource` over a stereo `SamplesBuffer` (the file's three existing tests already do exactly this) and assert the gains at pan 0 / ±0.5 / ±1 against the equal-power curve. Then a decision — move native to equal-power, or write the divergence down where `audio_spatial` currently promises the opposite. |
| **(read-derived) The facade documents an eight-voice bound that only one backend has.** `play_tone_metered` / `play_sfx_metered` both say, under *Limits worth knowing*, "**Eight voices per name**, then it wraps and reuses its oldest — a bound, not a leak." Native has that ring (`POLY_VOICES = 8`, `next_poly_voice`). The web path has none — `play_tone_to_metered` builds a fresh oscillator+gain per call and its own doc says "the web backend needs no voice pool". So on web the count is unbounded per frame, and `levels(meter)` saturates sooner because the browser mixes every live voice rather than eight. | `src/audio_facade.rs:262-266`, `:318-321`; `src/audio_wasm.rs:519-528`, `:765-778` | Read — the two docs contradict each other in the same repo. A decision: bound the web path, or state the bound as native-only. |
| **(read-derived) `levels()` is measured at a different point on the two backends, and the difference shows on positional sound.** `AudioLevels`' doc promises the **pre-volume** envelope on both — "turning the volume down, ducking a bus, or muting entirely does not change these values". Native honours that: the tap sits before the sink volume, which is where `update_position` writes the distance attenuation, so a native meter does **not** fall as the source recedes. Web taps *after* the sound's own gain node — which is exactly the node `Sfx::update_position` writes the distance attenuation into — so a web meter **does** fall with distance. A HUD driven off `levels()` for a `play_at_on_channel` sound therefore behaves differently on the two platforms. | `src/audio/analysis.rs:447-455` + `src/audio/positional.rs:96-101`; `src/audio_wasm.rs:367-372`, `:915-940`, `:1151-1158`; contract at `src/audio_analysis.rs:70-79` | A browser smoke reading `levels` for a positional channel at two distances, against the native selftest doing the same — the two answers should agree or the contract should say they do not. ⚠️ Cheaper first step: decide which point is intended, because the doc currently claims one of them for both. |
| **(read-derived) The spatial volume and the game's channel volume are the same slot, so each silently destroys the other.** `play_at` / `play_bytes_at` / `update_position` all write `volume_overrides[channel]`, which is the slot `set_volume` owns. A game that calls `set_volume("sfx", 0.5)` and then `play_at(...)` loses the 0.5 with no diagnostic; a `set_volume` after `play_at` wipes the distance attenuation until the next `update_position` puts it back. Neither doc mentions the other. | `src/audio/positional.rs:22-23`, `:44-45`, `:89-90`; `src/audio/bus.rs:81-83` | Read. A pure test once the state is reachable headlessly (same split as the crossfade row). Then a decision: a separate `spatial_volume` factor multiplied into `effective_volume`, or document the collision at both sites. |
| **(read-derived) `set_pan` is a permanent no-op on any tone channel.** `set_pan` writes `pans[ch]` and then `pan_handles.get(ch)`, but `play_tone` and `play_tone_poly` never wrap their source in `PannedSource` and never create a handle — only `append_decoded` (the clip path) does. So a tone channel has no handle on the first call and none ever after, and the pan is applied to nothing. The doc warns that a **mono** source cannot be panned, which happens to make the audible result the same, but by a different mechanism and only by coincidence: the tone path would still ignore a stereo source. | `src/audio/playback.rs:208-228`, `:255-268`; `set_pan` at `src/audio/positional.rs:113-122` | Read. One line to wrap the tone source, or one sentence in `set_pan`'s doc — the choice is whether a tone is ever meant to be positional. |
| **(read-derived) `update()`'s "avoid a heap allocation" comment is wrong about what it avoids.** The `[Option<String>; 8]` stack buffer skips one `Vec` allocation and then performs **one `String` allocation per fading channel** (`ch.clone()`), which is the allocation that matters. Only reached while a fade is live, so the cost is small and transient — the defect is the comment, which reads as a per-frame guarantee this code does not give. | `src/audio/playback.rs:313-336` | Read. `alloc-measure` if it ever matters; otherwise delete the claim or borrow the keys properly. **Not** filed as an efficiency defect — it is UNMEASURED and not on a hot path. |
| **(read-derived) The facade's low-pass helpers clobber the channel's whole effect on native.** `set_low_pass` writes `AudioEffect { low_pass_hz: Some(hz), ..Default::default() }`, resetting `pitch`, `attack_secs` and `release_secs`; `clear_low_pass` calls `clear_effect`, removing all four. The web counterparts touch only the filter. A game that sets an envelope through `AudioManager` and a filter through `Audio` loses the envelope, and only on native. | `src/audio_facade.rs:337-352`, `:355-366` | Read. Read-modify-write the existing effect instead of replacing it; a pure test needs the same headless split as the rows above. |
| **(read-derived) `LevelTap` captures the channel count once and downmixes with it forever.** `LevelTap::new` reads `inner.channels()` at construction, but a rodio `Source` may change format between spans (which is why `current_span_len` exists). A source whose span channel count changes mid-stream then folds the wrong number of samples per frame into the spectrum buffer, skewing every band. Levels are unaffected (they do not use `channels`). | `src/audio/analysis.rs:176-177`, `:196-210` | Read, and speculative about how often rodio actually does it — **re-derive before paying for it**. A pure test can chain two `SamplesBuffer`s of different channel counts and check the mono fold. |

**Not filed, checked and found sound** — recorded so they are not re-chased: the FFT (`spectrum.rs`
is pinned by Parseval, a known-bin sine and a DC case, which is the right way to check a transform);
the lock-free meter publication (`LevelSlot`'s sequence counter, and the per-voice staleness test in
`combine_voices`); the tap's position relative to `repeat_infinite`, which has its own regression
test and a contrast case; `MIN_AUDIO_DURATION_SECS` clamping on every fade constructor; and the
`stop` / `fade_out` / release interaction, which is the most carefully documented state machine in
the subsystem and has a test per path — all of them device-gated.

### Open — the 2026-09-02 editor review continuation

The 2026-08-28 review's unread half, read on 2026-09-02 by five parallel read-only passes, **every
finding re-derived against the code before it was filed here**. Read in full: `ui/gizmo_math.rs`,
`ui/gizmo.rs` (both halves), `ui/grid_overlay.rs`, `overlays.rs`, `ui/tile_paint.rs` (helpers and
tests), `ui/state_machine_panel.rs`, `ui/data_table_panel.rs`, `ui/timeline_panel.rs`,
`loading.rs`, `prefab.rs`, `settings.rs`, `i18n.rs`, `theme.rs`, `util.rs`, and `tests.rs` (33
tests, each judged on whether it can fail on its stated cause), `ui/docked/*` (all seven files),
and `ui/mod.rs` + `shortcuts.rs` + `reparent.rs` + `rename.rs` + the four small panels. Two of
the five passes died to a usage limit mid-read and were re-run from scratch; all five are in.
**With this the subsystem is fully read** — the 2026-08-28 half and this one, 10,893 lines
across 36 files — and this section plus the one below it are the complete residue.

**Filed before the first fix shipped, on purpose.** The 2026-09-01 timing-check review above is
what happens otherwise: three items shipped and the list they came from was never in the tree.

Fifty rows — counted from the table, not from a running tally; "fifty-five" stood here until
2026-09-03 and was never true. The behaviour ones are ordered by what they cost a user; the
test-quality ones (a test that cannot fail on its stated cause) are one batch, the way v0.155.2
shipped three; the doc ones are cheap and go with whichever fix touches the file. Every row
names its instrument.

**48 of the 50 rows are settled and 2 are open** — 40 shipped as releases v0.156.6–v0.156.26 on
2026-09-03, and 6 needed no release entry of their own: five settled by documentation or a
corrected in-app label, one decided as unsupported and said so at the site. The 47th is the
timeline keyframe drag, shipped as **v0.159.1** on 2026-09-10, and the 48th is the editor-settings
transition, decided and shipped as **v0.159.2** the same day — see the re-derivation note below.
Both remaining rows are the UNMEASURED per-frame-allocation observations, which are not defects,
so **the behaviour half of this review is empty**. ✅ The v0.159.2 decision left one new row behind on
purpose — quitting with the editor open saved nothing — and it **closed the next day as v0.159.3**
once the maintainer chose to take it. It is struck through in the table below rather than folded
into the closed row, because what it cost is the part worth keeping.
(Struck-through rows are done and keep whatever the fix contradicted.)

✅ **All six maintainer decisions are made and shipped**, so the "a decision, not a fix" kind is
empty: the gizmo's handles swallowing a small entity (screen-space hit-test), `Ctrl+D`'s component
set, the Ambient Light header (an explicit button), drag-to-reparent (preserve the world position),
`Ctrl+Z` in a focused text field (blocked while typing), and the three panels' edits (documented,
not made undoable). The "cheap prose" kind is empty too.

⚠️ **The "needs eyes or another OS" kind lost a row to a re-derived gate, not to a windowed run.**
`Ctrl+C` / `Ctrl+V` was filed as needing Windows or Linux. `egui_winit::is_copy_command` is
`modifiers.command` with **no `cfg`**, so the behaviour is identical everywhere: one file read in
the cargo registry settled the premise and two headless frames settled the fix (v0.156.26). That is
the third time in this program a written gate over-estimated its cost and the first where the gate
alone kept a row deferred. **Re-derive the gate before paying what a row claims to cost.** The four
below were re-derived on 2026-09-03 and all four held.

⚠️ **And on 2026-09-10 the two *behaviour* rows of those four fell to the same re-derivation,
which makes it the fourth and fifth time and the second and third where the gate alone kept a row
deferred — so the 2026-09-03 re-derivation above is itself an example of the thing it warns
about, twice over.** The second one cost even less than the first: "a windowed run — the load sits
inside the winit handler with no unit seam" was wrong on its face, because `App::set_editor_mode`
is `pub(in crate::app)` and its own doc calls itself *"The one entry point"*, while
`settings_path_override` had been sitting in `EditorState` for exactly this purpose and two tests
already used it. Three tests, 0.13 s, no window. **Both gates were written by readers who did not
open the entry point they were describing.** "A windowed run, or a
multi-frame pointer-drag harness … is heavy" survived a re-derivation that never tried it. The
harness is **~60 lines** and needs nothing egui does not already hand back: one persistent
`egui::Context` (the editor's existing `editor_frame` builds a fresh one per call, which is why
no drag had ever been driven here), a synthesized `RawInput` per frame, and — the part that
looked hard — **widget positions read out of the frame's own `Shape::Text` list**, since a
`DragValue` draws its number. No window, no GPU, no new dependency, 0.04 s for four tests. The
lesson is not "gates are pessimistic" but the narrower one this file keeps re-learning: **a gate
is re-derived by attempting it, not by re-reading the row.**

| Open row | Instrument | Re-derived 2026-09-03 |
|---|---|---|
| ~~Timeline keyframe drag hands the drag to its neighbour~~ | a windowed run, or a multi-frame pointer-drag harness | ❌ **The diagnosis held, the gate did not — DONE v0.159.1.** Right that the drag is egui's, keyed on the row index's widget id, and that a pure test of the re-sort pins nothing. Wrong that this needs eyes: a headless multi-frame harness reproduced it in one sitting. ⚠️ And the *symptom* was wrong too — "both keyframes leapfrog toward the cursor" is not what happens. The widget alternates which keyframe it drives on every frame, so the pair advances **together**, locked at the gap they met with, and neither ever passes the other: dragging keyframe 0 from 0.0 s to 2.4 s carried its neighbour from 1.0 s to **2.2 s**. |
| ~~Editor settings read on exactly one transition~~ | a windowed run | ❌ **The facts held, the gate did not — DECIDED + DONE v0.159.2.** "No unit seam" was wrong: `set_editor_mode` is `pub(in crate::app)`, is documented as the one entry point, and `settings_path_override` exists for exactly this. All three claims reproduced headlessly on the first run. ⚠️ That row's own `Where` column was stale and is corrected. |
| Per-frame allocation, editor-only | `alloc-measure` | **Holds.** UNMEASURED by choice and not on a game path; filed as an observation, not a defect. |
| Per-frame allocation in the docked panels | `alloc-measure` | **Holds.** Same — UNMEASURED, editor-only. |

| Item | Where | What settles it |
|---|---|---|
| ~~**A gesture started on entity A commits on entity B when Ctrl+Z re-selects B mid-drag.**~~ | `src/app/editor/ui/gizmo.rs` | **DONE v0.156.6** — with the two rows below, because they are one mechanism: the release handler was the only place a gesture was recorded, so every non-release exit dropped what it had applied. `gesture_entity` + one `commit_gesture` for release and every abandon site. |
| ~~**A Docked drag whose cursor leaves the central panel is abandoned with the partial move applied and no undo entry.**~~ | `src/app/editor/ui/gizmo.rs`, gate at `docked_rt.rs` | **DONE v0.156.6** — recorded on the way out. Overlay mode was never affected: egui's `wants_pointer_input` keeps a held drag flowing. |
| ~~**Two abandon sites still clear only `gizmo_dragging`** (the `UiNode`-gone and `Transform`-gone arms), the v0.155.4 defect on the sites it did not reach.~~ | `src/app/editor/ui/gizmo.rs` | **DONE v0.156.6.** |
| ~~**📂 Load — and 🗑 Delete through `hierarchy::despawn_recursive` — despawns with bare `World::despawn`, so the replaced scene's rapier bodies and tile colliders stay in `PhysicsWorld`** as invisible colliders.~~ `PhysicsWorld` is not persistent, so a *Replace* drops it with the World; the editor's own two paths keep the World and leak. `physics::despawn_with_body` exists and its doc names exactly this. | `src/app/editor/ui/docked/save_load.rs:139-142`, `src/hierarchy.rs:131-133` | Unit test: `App::new()`, insert `PhysicsWorld`, spawn an entity with a `PhysicsBody` (handle `h`), write a one-entity scene, `do_load_scene` → `physics.rigid_body(h).is_none()`; today `Some`. Same for a synced `TilemapColliders`. → **DONE v0.156.8** — new `physics::release_physics` (body + tile colliders, no despawn; `despawn_with_body` is now that plus the despawn), called by Load for every entity and by one `App::editor_despawn_subtree` both delete paths share. Four sabotages, including root-released-descendants-not, each on its own test. |
| ~~**The Alt+click eyedropper only checks *just-pressed*, so on the second held frame Freehand starts a stroke and paints the sampled value around the sampled cell**~~ (or erases, for Alt+right). Any real click spans several frames; undoable, since the stroke commits. | `src/app/editor/ui/tile_paint.rs:168`, `:229-234` | Unit test: Alt+Left press, `update_tile_paint`, `flush`, `update_tile_paint` again → `tile(0,0) == 0` and `!paint_active`. The existing `tile_eyedropper_alt_click_picks_value` runs one frame only. → **DONE v0.156.9** — Alt short-circuits the tools for as long as it is held. Test: the held frame after an Alt+click paints nothing; control: the same frame without Alt paints. |
| ~~**Bucket never clears `paint_active` and its unconditional `paint_stroke.clear()` discards a stroke left active by a tool switch mid-hold**~~; the follow-on Freehand stroke is then lost too, because `commit_paint_stroke` `take()`s `changes` before finding `paint_entity == None`. Reachable only by clicking the inspector's tool button with the other mouse button held. | `src/app/editor/ui/tile_paint.rs:326-327`, `:229`, `:354-357` | Unit test: Right press + update, `paint_tool = Bucket`, release + update, Left press + update → `cmd_history.undo_len() == 2` and one undo restores the erased cell. → **DONE v0.156.9** — a stroke belongs to its tool (`paint_stroke_tool`): a tool change under it ends it the way a selection change does, and Bucket finishes rather than clears. The row's test, as written, was red on the bug (`undo_len` 0) and is green now (2, both undo). |
| ~~**The data-table panel's Open on a bad path loses the typed path, reports nothing in the panel, and panics the editor under strict assets**~~ — `App::load_data_table` returns `()`, its error goes to `record_failure`, which panics when `set_strict_assets(true)` (first load, so the hot-reload exemption does not apply), else to `asset_failures`, which no editor UI reads (`rg -c asset_failures` under `app/editor/ui`: 0). | `src/app/editor/ui/data_table_panel.rs:59-68`, `src/app/editor/loading.rs:53-55`, `src/asset_path.rs:177-178` | Unit test, once `load_data_table` reports its outcome (today it cannot be branched on): a bad path → `Err`, and a `#[should_panic]` twin under strict mode stating whether that is wanted. → **DONE v0.156.16** — `App::editor_open_data_table` validates through `DataTable::load` first, so a bad path never reaches `record_failure`. ⚠️ The row asked for a `#[should_panic]` twin under strict mode; that flag is **process-global** and `cargo test` is threaded, so turning it on would panic unrelated tests. The test asserts the absence from `asset_failures()` instead, which is what makes the panic unreachable. |
| ~~**The bounds overlay draws each `Collider` at `Transform.position` — the exact place `collision/grid.rs` stopped indexing at**~~, and the Transform AABB likewise; the renderer prefers `GlobalTransform` (`renderer/sprite/collect.rs:67`). A parented collider's green box draws at its local offset while collision tests it at its world position — the overlay meant to make that visible reproduces the pre-fix placement. | `src/app/editor/overlays.rs:15-19`, `:29-37` | Unit test: P at (300,200) with child C local (16,0) carrying `Collider::Aabb{8}` and a `GlobalTransform` at (316,200); `draw_debug_bounds()` → the collider `DebugShape::Rect` has `min == (308,192)`, today `(8,-8)`. → **DONE v0.156.11** — `world_placement` (GlobalTransform, else Transform) for both passes; the row's test as written, with a root control. |
| ~~**The pathfinding overlay rebuilds each `Tilemap` without its `projection`, so isometric and hexagonal maps are shaded as a square lattice at the wrong positions**~~ — contradicting its doc "visualizes exactly the grid a game … would navigate". The grid readout in `grid_overlay.rs:80` uses the real map, so the two overlays disagree on one map. | `src/app/editor/overlays.rs:57-70`, `:80-87` | Unit test: 3×3 map `.with_projection(Isometric)`, blocked (1,1), `draw_pathfinding_overlay()` → the one filled rect's centre equals `tm.cell_center_world(1,1)`. Fix shape: snapshot `(PathGrid, tile_size, origin, projection)` inside the query — which also answers the row's "unavoidable clone" claim (`PathGrid::from_tilemap` takes `&Tilemap`). UNMEASURED as an efficiency claim. → **DONE v0.156.11** — the snapshot carries the projection and the `PathGrid` is built inside the query, which also removed the tile-grid clone the old comment called unavoidable; that efficiency half was never measured and is now moot. |
| ~~**The Transform gizmo hit-tests and highlights a parented entity at its local `Transform.position` while the sprite renders at `GlobalTransform`.**~~ A press on the visible sprite starts nothing; a press on the ghost box at the local offset drags it. Design item: the fix needs delta→local conversion through the parent. | `src/app/editor/ui/gizmo.rs:329`, `:452-468` | Unit test: P/C as above with `GlobalTransform` at (316,200); `update_transform_gizmo_native(C, trC, (316,200), pressed)` → `gizmo_dragging`. Control: the same press at (16,0) is accepted today, proving the box is misplaced rather than absent. → **DONE v0.156.18** — the gizmo reads `App::editor_world_transform` everywhere and inverts the parent's matrix on the way back, so a rotated and scaled parent is exact for position (scale is component-wise, which is exact bar the shear case `GlobalTransform` cannot represent either). ⚠️ The row called the drag a design item; it is one inverse. The harder part was the tests: two of three bypassed the read by passing the transform in, and only a mouse press through `update_editor_gizmo` exercised it. |
| ~~**An entity ≤ 16 world units on a side has no move region**~~: every interior point is within a handle's 8-unit hit radius, first hit wins, and zoom does not help because the radius is in world units for the Transform gizmo (the doc says "logical pixels" — true only for the UI gizmo). A 16×16 tile can never be dragged. | `src/app/editor/ui/gizmo_math.rs:66`, `:142-165`, `src/app/editor/ui/gizmo.rs:385` | Decision (hit-test handles in screen space via the camera, or body-first when the entity is smaller than 3× the radius), settled by `hit_test_handles(splat(-8), splat(16), ZERO) == None` — today `Some(Top)`. → **DECIDED + DONE v0.156.22 — screen-space**, the option the user chose over body-first. `hit_test_handles` takes the radius; the Transform gizmo divides the constant by the camera zoom (`Camera::safe_zoom`, now public so the guard has one spelling), the UI gizmo passes it unchanged. ⚠️ The row's own test could not settle it alone: the division lives in the gizmo, so a sabotage left the pure test green and a real mouse press through `update_editor_gizmo` was needed. Only the radii are corrected, not the drawn handles. |
| ~~**`snap_size` from the settings file is applied unclamped; a `0.0` makes every snapped drag produce NaN, and the NaN move is not recorded so it cannot be undone.**~~ The UIs clamp only while shown; `grid_overlay.rs:44` spells the same value as `.max(1.0)`, so the grid keeps drawing at 1 px while the gizmo snaps to 0 — two spellings. Hand-edited file only; the derived `Default` (`snap_size: 0.0`) has no caller. | `src/app/editor/settings.rs:40`, `src/app/editor/util.rs:8-13`, `src/app/editor/ui/gizmo.rs:628` | Unit test: `snap_to_grid(Vec2::splat(5.0), 0.0).is_finite()`, or `EditorSettings { snap_size: 0.0, .. }.apply_to` yields ≥ 1.0. → **DONE v0.156.14** — `snap_to_grid` treats a non-positive or non-finite size as no snapping, both resize paths go through it, and `sanitize_snap_size` clamps the file's value; the row's tests plus a real resize at 0 landing on a finite 30. |
| ~~**Reload in the data-table panel resolves by *path*, not the selected *name*, and `reload_path` takes the first HashMap entry whose canonical path matches**~~ — two names sharing one file reload or skip nondeterministically, and the status names the selected table either way. | `src/app/editor/ui/data_table_panel.rs:255-266`, `src/data_table.rs:335-353` | Unit test on `DataTableRegistry`: two names, one path, one dirty — the result depends on hash order. Fix is a `reload_name`, or passing the name through. → **DONE v0.156.14** — `DataTableRegistry::reload_name`; `reload_path` delegates and its shared-file limitation is stated. The row's test as written. |
| ~~**Dragging a keyframe's time past its neighbour hands the drag to the neighbour**~~: `retime` is keyed by row index, `set_time` re-sorts, and egui keeps the drag on the row's widget id. Both keyframes leapfrog toward the cursor. | `src/app/editor/ui/timeline_panel.rs:86-97`, `:140-142`; `src/timeline.rs:135-143` | Eyes in a windowed run. A headless multi-frame `egui::Context::run_ui` pointer-drag harness would pin it and is heavy. → **DONE v0.159.1 — and the harness was not heavy** (see the note above the table). A live time drag is now parked in `ui.data` and written through on release, so the track is re-sorted exactly once, with no drag in flight. The row shows the time the drag has reached while it is parked, which is the half that separates a fix from a mute — pinned by its own test. Four tests, each **seen to fail** against the hunk it guards: the repro reddens on the pre-fix row (the neighbour at 1.4 s by drag step 7), the live-display test with it, the interrupted-drag flush and the typed-time branch on their own hunks. |
| ~~**Editing one channel of a colour keyframe writes back all four clamped to [0,1]**~~ — egui 0.34's `DragValue::range` clamps the *existing* value by default (`clamp_existing_to_range`), the sibling's `changed()` carries the clamped snapshot through `revalue`, and `Color` is un-clamped by design (HDR targets). Same shape at `:158-163` for a duration above 3600. | `src/app/editor/ui/timeline_panel.rs:243-278`, `:100-103`, `:134-136` | Eyes, or the same harness; fix is `clamp_existing_to_range(false)` on the four channels. → **DONE v0.156.13** — the four channels and `duration` carry the flag; `tl.time`'s clamp to the duration is the intended one and stays. Pinned by reading, not by a test: the colour widget is a closure the panel hands to `timeline_track_ui`, and the emitter/light test covers the mechanism. |
| ~~**The per-state add-transition target is never re-validated after a state is removed**~~, so "+" pushes a dead edge to a state that no longer exists (warn-logged, recoverable). | `src/app/editor/ui/state_machine_panel.rs:332-337`; `src/animation/state_machine.rs:176-183` | Unit test on a pure `resolve_add_target(stored, &other_states)` once extracted; until then eyes. → **DONE v0.156.19** — `resolve_add_target`, the pure helper the row asked for, re-derived every frame; sabotage-verified. |
| ~~**"+" on an existing state name silently no-ops**~~ (`add_state` is `entry().or_insert`) while the panel clears the box as if it succeeded. | `src/app/editor/ui/state_machine_panel.rs:394-399`, `:487-489` | Eyes; check `all_state_names.contains(&name)` first. → **DONE v0.156.19** — `add_state_verdict` splits empty / duplicate / addable; a duplicate raises an error toast and keeps the typed name. The row said "eyes"; the decision came out as a pure function, so a test settles it. |
| ~~**A scene Replace mid-stroke leaves `paint_active` / `paint_entity` / `paint_stroke` standing; the next frame's abandon path commits the stroke — against an old-world `Entity` — into the history the reset just emptied.**~~ The in-flight *rename* buffer survives the reset the same way and commits onto whatever new entity aliases the old handle (`ui/mod.rs:191-195`'s liveness check passes for an alias). Narrow (a game-driven Replace with the button held), but it is the stale-handle-across-reset class. | `src/app/scenes.rs:62-77` (what `reload_scene` clears), `src/app/editor/ui/tile_paint.rs:353-367` | Unit test: `setup_paint_app`, set `paint_active`, `paint_entity = Some(e)`, a non-empty `paint_stroke`; `reload_scene()`; `finish_paint_stroke()` → `cmd_history.undo_len() == 0`, today 1. → **DONE v0.156.9** — `EditorState::drop_in_flight_edits` on reset: stroke, rename buffer and gizmo gesture forgotten, none recorded. The test asserts the alias (`b == a` after the reset) as its precondition, and its control shows the reset-less abandon still commits. |
| **Editor settings — including the locale — are read on exactly one transition (the first Off/Overlay→Docked) and written only on Docked→other or the two toolbar buttons**, so Overlay mode runs on `EditorState::new` defaults and closing the window while Docked saves nothing. Three docs say the locale is "set each frame from the persisted setting". The Overlay Snap control changes `snap_size` that is never persisted unless Docked is later entered *and* left. | `src/app/window.rs:223-229` (sole `load_editor_settings` call); docs at `i18n.rs:9-10`, `:19-20`, `settings.rs:6-7`, `editor.rs:8`; `ui/mod.rs:445-452` | ⚠️ **The `Where` column above is stale** — `load_editor_settings` is not in `window.rs`; it is `settings.rs:81`, called only from `set_editor_mode` under `mode_transition`'s `load_settings`, and `i18n.rs` is `src/app/editor/i18n.rs`. **The three docs are DONE 2026-09-03**, verified by reading: the per-frame source is `EditorState::locale`, which `EditorSettings` fills in only on the first Docked open, so an overlay-only session keeps the `EditorState::new` default. The behaviour half is still open — eyes in a windowed run, since the transition has no unit seam. → **DECIDED + DONE v0.159.2 — widen the transition**, chosen over a debounced save-on-change and over documenting the Docked-only policy as intended. `mode_transition` now loads on the first entry into **any** editor mode and saves on every exit from one, so the overlay honours the persisted snap/grid/locale and its own edits survive. ⚠️ **No windowed run was needed for any of it** — see the re-derivation note above the table. Three headless tests, each seen to fail: the two behaviour ones and the transition table redden on the pre-v0.159.2 formula, and the quit-while-open row (below) reddens against a `Drop` that saves. |
| ~~**(new, 2026-09-10) Quitting with the editor still open saves nothing**~~ — `set_editor_mode` is the only caller of `save_editor_settings`, so a session that closes the window without switching mode drops every preference it changed. | `src/app/editor/settings.rs:72` (sole writer), `:99` | The fix is a debounced save when `EditorSettings::from_state` differs from what was last written (`RtDebounce` is the precedent); the cost is a per-frame struct compare on the editor path only. → **DONE v0.159.3 — and the filed fix shape was the wrong one.** ⚠️ A debounce was never needed: winit's `ApplicationHandler` already has an `exiting` hook, this repo did not implement it, and one call to a new `save_editor_settings_on_exit` seam closes the row with **no per-frame work at all**. The row's own remedy would have added a struct compare to every editor frame to buy nothing extra — the third time in three days a filed *remedy*, not just a gate, over-estimated itself. The remaining gap is smaller and is stated at the site: a crash or a force-quit reaches neither writer, because nothing writes on change. ⚠️ **The wiring is one line in a winit callback no headless frame can reach**, so it is pinned by an `include_str!` source test with a control — the `the_three_panels_record_nothing_on_the_undo_stack` idiom. Deleting that line leaves the two behavioural tests green while the fix is gone from the shipped binary, which the sabotage run confirmed. |
| ~~**`sync_tilemap_colliders`'s doc says it "returns `false` unless `entity` has `Tilemap` + `TilemapColliders` and a `PhysicsWorld` exists"; it returns `true` for a `Tilemap` with a `PhysicsWorld` and no `TilemapColliders`**~~ — and `tile_collider.rs:591-605` asserts exactly that `true`. A game gating "colliders synced" on the return gets `true` with nothing synced. | `src/app/editor/loading.rs:8`; `src/physics/world/tile_collider.rs:341-348` | Doc, test, or function must move; a unit test asserting `false` on Tilemap + PhysicsWorld without `TilemapColliders` fails today and contradicts the existing test. → **DONE 2026-09-03 — the doc moved, not the function.** `tile_collider.rs`'s own test asserts the `true`, so changing the behaviour would have broken a check that means what it says. The doc now states what the `bool` is (a `PhysicsWorld` was there) and says outright not to gate "colliders are up to date" on it. |
| ~~**`entity_to_def` is infallible**~~, so `save_selected_as_prefab`'s "No entity to save" arm and `subtree_to_defs`'s `unwrap_or_default` are dead, and a dead entity yields an all-`None` def instead of `None`. Unreachable in practice — the selection is liveness-checked each frame — so the cost is a misleading API. | `src/app/editor/prefab.rs:27`, `:40`, `:105-108` | Unit test `entity_to_def(&world, despawned).is_none()` — fails today. → **DONE v0.156.20** — it declines a dead entity, which makes both `None` branches live; the row's test as written. |
| ~~**`load_editor_settings` swallows a RON parse error with no log**~~; a corrupt file silently reverts every preference and the next save overwrites the evidence. No historical trigger — every non-`#[serde(default)]` field dates from the file's first version. | `src/app/editor/settings.rs:66` | Unit test, only after `editor_settings_path()` is injectable; today it points at the user's real data dir. → **DONE v0.156.10** — logged at `warn`, and the path is injectable now (`settings_path_override`), so the corrupt-file case is pinned by a test; the log line itself is what a test cannot see. |
| ~~**A negative `Transform.scale` (a flipped sprite) inverts the highlight rect and collapses to 2 px on the first resize frame**~~ — the rotation handle anticipates negative scale (`.abs()`), the rest does not. Low: `components.rs:242` steers users to `SpriteFlip`. | `src/app/editor/ui/gizmo.rs:339`, `:517-543` | Decide it is unsupported and say so at the site, or a test that a Right-handle drag keeps the sign and magnitude. → **DECIDED 2026-09-03 — unsupported, and said so at the site.** `MIN_SPRITE_SCALE` floors every arm on purpose; `Transform`'s own doc steers mirroring to `SpriteFlip` and warns that a negative scale breaks rotation. The resize is undoable, so the cost is one Ctrl+Z. Recorded rather than fixed. |
| ~~**Test-quality batch — four checks that cannot fail on their stated cause.**~~ (a) `delete_undo_restores_full_def` promises "full def (including non-core components)"; the fixture is Tag+Transform with no registry and it asserts the Tag. (b) `create_entity_with_def_undo_redo` says redo "re-spawns with its original components"; asserts only the Tag. (c) `editor_settings_round_trip` never sets `show_bounds` or `locale`, so both sit at defaults on both sides. (d) `brush_cells_sizes_and_clamp` pins only `.len()` for the 3×3 and 5×5 blocks, so a block shifted by +1 in both axes passes. | `src/app/editor/tests.rs:16-52`, `:56-87`, `:235-262`; `src/app/editor/ui/tile_paint.rs:766`, `:771` | Sabotages that stay green today and must not: (a) `history.rs:236` spawn `EntityDef { tag, transform, sprite, ..Default::default() }`; (b) `history.rs:324` spawn a tag-only def; (c) delete `settings.rs:46` (`s.locale = self.locale`); (d) swap row/col in the output tuple. Fix (d) by comparing sorted membership plus one asymmetric case. → **DONE v0.156.15**, all four, each sabotage-verified against the defect it names. (c)'s fix is the general one: every field of the round trip now differs from the default, so no half is asserted against itself. |
| ~~**`every_editor_addable_component_survives_a_scene_save`'s docstring says it "drives the real serialization path rather than the registry's bookkeeping"; the body calls `component_names_for`, which *is* the bookkeeping**~~ (the presence check). Save runs `serialize_entity`, whose `serialize` closure returns `None` when `ron::to_string` fails — a registered component whose `Serialize` errs is listed by the test and dropped by the save. The property it does check (adder ⇒ registered) is real; docstring-vs-body. | `src/app/editor/tests.rs:610-612` vs `:645`; `src/prefab/serde_registry.rs:118-135`, `:156-165` | Sabotage that stays green: register a component whose `Serialize` impl returns `Err`. Either drive `serialize_entity`, or say what the test checks. → **DONE v0.156.15** — it calls `serialize_entity` now, so a component that is present but does not serialize is caught. The row's own sabotage (a component filtered out of the serialize path) reddens it and could not have touched the old version. |
| ~~**None of the three panels' edits go through `EditorHistory`**~~ — state machine (11 edit kinds), timeline (add / retime / revalue / remove, duration, loop, time), data table (cells, rows) — and none of the three module docs says so, while the same inspector advertises "Ctrl+Z undo". A removed state stays gone and Ctrl+Z undoes the last gizmo or paint action instead. | `src/app/editor/ui/state_machine_panel.rs:1-7`, `timeline_panel.rs:1-6`, `data_table_panel.rs:1-15` | Doc: one sentence per module doc. If the decision is to keep them non-undoable, a test that `undo_len()` is unchanged after a panel edit makes it explicit. → **DECIDED + DONE — document it**, which the user chose over making the edits undoable: the panels edit *structure* (states, keyframes, rows), so an editor command per edit kind is a feature, not a note. One paragraph in each of the three module docs, and `the_three_panels_record_nothing_on_the_undo_stack` pins it. ⚠️ That test reads the **source text**, which is a new idiom here and deliberate: the panels only edit on a real interaction, and a headless egui frame interacts with nothing — a behavioural `undo_len()`-unchanged assertion would pass without the panel ever editing, which is a test of nothing. It carries a control that fails the same search against a file that does record. No version bump: no behaviour changed. |
| ~~**`finish_paint_stroke`'s doc says `paint_mode` survives both abandon sites ("re-selecting a tilemap resumes painting"); at the selection-ceasing-to-be-a-Tilemap site the caller clears it on the next line**~~, and `state.rs:330` says so. Only the selection→`None` site leaves it set. | `src/app/editor/ui/tile_paint.rs:377-378` vs `src/app/editor/ui/gizmo.rs:74-75` | A test per site (select a non-Tilemap, `update_editor_gizmo`, `assert!(!paint_mode)`; select `None`, assert it survives), then the sentence names the split. → **DONE v0.156.9** — the doc now says who clears `paint_mode` where (the `None` site keeps it, the non-Tilemap site clears it, the docked inspector clears it first). |
| ~~**`anchor_base`'s doc says it is the single shared definition `UiNode::screen_pos` uses; `screen_pos` has its own copy**~~ (`rg -n anchor_base src` → 4 hits, all in `gizmo_math.rs`), and the only test uses `anchor_base` on both sides. They are two *because* `anchor_base` is `cfg(not(wasm))` and `screen_pos` compiles on wasm — the doc claims the opposite. They match today. | `src/app/editor/ui/gizmo_math.rs:18-23` vs `src/ui/node.rs:145-158` | Unit test over all 7 anchors: `anchor_base(a, size, vw, vh) + offset == UiNode { anchor: a, size, offset }.screen_pos(&vp)`; then the doc says why there are two. → **DONE v0.156.20** — the formula is `Anchor::base` and both call it, so the doc's claim is now true rather than merely lucky. The row's test over all seven anchors, with a control that they land in seven different places. |
| ~~**Two wasm-branch comments say "log and return"; nothing logs.**~~ The doc at `:29` ("no-op on wasm") is the truth — bar the registry insert + `register_persistent` at `:35-45`, which do run on wasm. | `src/app/editor/loading.rs:66`, `:168` | Eyes — a comment; nothing executable distinguishes it. → **DONE 2026-09-03** — both say what they are, a silent no-op, and that they claimed otherwise. |
| **Per-frame allocation, all editor-only and UNMEASURED, not filed as defects:** the data-table panel clones every cell of the selected table, `columns`, and a sorted `names()` every frame the tab is open (a 10k-row table: ~10k×cols `Value` clones per frame; the unvirtualised egui `Grid` likely dominates); `overlays.rs:10-19` two `Vec`s per frame; `grid_overlay.rs:16-31`, `:77`, `:81` two `Vec`s + a `String`; `gizmo.rs` `others` `Vec` per held frame; `entity_matches_filter` two `to_lowercase` per entity per frame while a filter is typed. `gizmo_drag_start_pos` is write-only (2 writes, 0 reads) — dead, harmless. | `src/app/editor/ui/data_table_panel.rs:28-32`, `:114-133`, and as listed | `alloc-measure` if any of them ever matters; none is on a game path. |
| ~~**The inspector stages every reflect-registered field of the selected entity *before* the egui block and writes all of them back unconditionally *after* it, so a world write to one of those components made inside the block is reverted the same frame**~~ — Ctrl+Z / redo of Move / Resize / Rotate / MoveUiNode / ResizeUiNode on the *selected* entity, an inline-rename commit, the "Name:" field, ⧉ Paste of a component already present. The guard only helps when the undo *moves* the selection. ⚠️ Confirmed by reading: `set_field` writes unconditionally, and the write-back is keyed on `comp_fields_entity == Some(sel)`, which an undo of the selected entity's own move satisfies — every gizmo undo test in the tree calls `cmd_history.undo` directly, which is why none of them sees it. The docked build's own direct writes are victims too — the `Name:` field (`ui/mod.rs:122-123`), the inline rename (`rename.rs:43`), ⧉ Paste of a component already present (`inspector_tab.rs:213`) — all dead for reflected types (`Tag`, `Transform`, `Sprite`, `UiNode`…) since both sides landed on 2026-05-25; only the reflect grid's own fields work. | `src/app/editor/ui/mod.rs:208-224` (stage), `:235` (Ctrl+Z runs after staging), `:649-665` (write-back) | Unit test: entity `e` selected with `Transform` (100,100) and `MoveEntity{e, (0,0)→(100,100)}` on the stack; a headless egui frame with a Ctrl+Z key event around `update_editor_ui` → `position == (0,0)`, today (100,100). Control: the same with a *different* entity selected passes today. Fix shape: write back only the fields the UI changed, by comparing against the staged copy. → **DONE v0.156.7, exactly that shape.** `changed_fields` is the decision; two headless-frame tests (a Ctrl+Z key event, a registered panel writing `Tag`) plus the pure one, each sabotage landing on its own. ⚠️ Harness lesson for the next frame-driven test: `handle_editor_shortcuts` reads `RawInput.modifiers`, not the key event's, so a synthetic frame has to set both. |
| ~~**`＋ Add child` spawns without recording an `EditorCmd::CreateEntity`, while its own comment claims parity with "＋ New Entity" — which does record one.**~~ Ctrl+Z after it undoes the *previous* command and the child stays. | `src/app/editor/ui/docked/context_menu.rs:37-48` vs `entities_tab.rs:27-32` | Unit test: `editor_apply_entity_context_action(p, AddChild)` → `undo_len() == 1`; undo → the child is dead and `Children(p)` holds no live child. → **DONE v0.156.10** — recorded with a def carrying the parent's tag; `CreateEntity`'s undo now cascades through the hierarchy so the parent's `Children` is clean. The row's test, plus redo re-attaching. |
| ~~**`➕ Spawn` (prefab) creates an entity without recording it**~~, unlike Duplicate / Paste / New Entity. | `src/app/editor/prefab.rs:117-136` | Unit test: write a prefab to a temp path, `spawn_prefab(path)` → `undo_len() == 1`; undo despawns it. → **DONE v0.156.10** — the row's test as written. |
| ~~**The toolbar "Exit (F2)" button re-implements the Docked→Off transition and omits `save_editor_settings()`**~~, so settings save on the F2 *key* and not on the button labelled with it. Two spellings of one transition. | `src/app/editor/ui/docked/toolbar.rs:106-113` vs `src/app/window.rs:218-238` | Unit test after extracting one `App::exit_docked()` used by both sites: it rewrites `editor_settings.ron`. → **DONE v0.156.10** — `mode_transition` (pure, table-tested) + `App::set_editor_mode` for F1, F2 and the button; the settings path is test-overridable, so the save is asserted on disk. |
| ~~**In Docked mode the inspector clears `paint_mode` for a non-Tilemap selection *before* the gizmo runs, so the gizmo's "selection ceased to be a Tilemap → `finish_paint_stroke`" abandon site is unreachable there**~~ — the buffered stroke is neither committed nor cleared; it commits later (clearing the redo stack) or never. The third abandon site, and the reason the `paint_mode` doc row above is wrong in Docked mode specifically. | `src/app/editor/ui/docked/inspector_tab.rs:146-149` vs `ui/gizmo.rs:64-76`; order `ui/mod.rs:278` before `:682` | Unit test: a buffered stroke on A, select a non-tilemap, one Docked frame → `undo_len() == 1` and `!paint_active` (today 0 / true). Fix shape: the gizmo ends a stroke whose entity is no longer the selection regardless of `paint_mode`. → **DONE v0.156.9** — the gizmo ends a stroke whose entity is no longer the selection regardless of `paint_mode`. Tested at the gizmo level with exactly the state the inspector leaves behind. |
| ~~**The Scene tree's root predicate is "no `Parent`", so an entity whose parent is dead is neither a root nor anyone's child and vanishes from the tree**~~, while `topological_sort_entities` and `HierarchySystem` treat it as a root — one derived value, three spellings (`ui/mod.rs` has a verbatim second copy at `:322-348`). The one tool that could repair the orphan cannot see it. | `src/app/editor/ui/mod.rs:257-276`, `:322-348` vs `src/hierarchy.rs:210-216` | Unit test: extract `scene_tree_inputs(world, &entity_list)` replacing both copies; an orphan lands in `root_entities`. → **DONE v0.156.14** — one `scene_tree_inputs` for both branches; a root is an entity whose parent is not in the list. The row's test as written. |
| ~~**`reload_scene` clears `editor_save_status` but not `editor_load_status`**~~, so the toolbar keeps "✓ N entities ← path" for a scene that no longer exists. | `src/app/scenes.rs:77` | Unit test: set `editor_load_status`, `reload_scene()` → `None`. → **DONE v0.156.10** — the row's test as written. |
| ~~**The "is a UI widget" set is spelled twice**~~ — `entity_kind` lists 7 widget types, the editor registry adds 16 — so nine editor-addable widgets do not classify as `Ui` on their own. Low: widgets need a `UiNode`, which the `UiNode` arm catches. | `src/app/editor/ui/docked/entity_kind.rs:65-72` vs `component_registry.rs:50-97` | Unit test: every UI factory name applied to a fresh entity → `entity_type_icon == "🔘"`. → **DONE v0.156.20** — the ladder covers all sixteen. ⚠️ The row called it low because a widget needs a `UiNode`; the "+ Add" factory does not add one, so the gap was reachable exactly there. The tie is the row's own test, which is a third spelling whose only job is to redden on drift. |
| ~~**Eight doc comments that no longer match the code**~~, all low: `context_menu.rs:26-27` "its only callers live in this file" (they are in `entities_tab.rs:205` and `scene_tab.rs:148`); `context_menu.rs:104` "Entities-list" tests (shared with the Scene tree); `toolbar.rs:5` lists 5 of 11 controls; `toolbar.rs:118-123` a "Shared tab-body functions" header with nothing under it; `docked/mod.rs:184-186` says `inner_rect`, the code reads `response.rect`; `state.rs:198` `inspector_tab` omits 2 = Scene; `state.rs:307` `bottom_tab` omits 2 = Audio; `state.rs:278` `central_rect` "(x, y, width, height)" is an `egui::Rect`. | as listed | Eyes: prose against the cited lines. → **DONE 2026-09-03**, all eight, each saying what it used to claim so the correction is not mistaken for the original. The "Shared tab-body functions" header was kept rather than deleted: the property it states still holds and still matters (a body must not change behaviour based on whether the docked panel or the overlay window is drawing it), so it now points at where those bodies actually live. |
| **Per-frame allocation in the docked panels, UNMEASURED**: `inspector_tab.rs:207` deep-clones the `ron::Value` clipboard every frame to read its name; `:171-175` a `HashSet<String>` and `:219-224` a sorted `Vec<String>` per frame; `assets_tab.rs:10-14` `image_list()` (a `String` per image) and `:27-30` a filename `String` per entry, every frame the tab is open; `entity_kind.rs:108-113` a lowercase `String` per comparison inside `sort_by_key`. | as listed | `alloc-measure`; none is on a game path. |
| ~~**Editor shortcuts, the `?` cheatsheet and toasts run in `EditorMode::Off`**~~: the block is gated only on `egui_ctx`, which is `Some` in every windowed frame, and F1/F2 never clear the selection. Overlay → click an entity → F1 → play: Backspace/Delete despawns it and its subtree, Ctrl+S writes `saved_scene.ron`, Ctrl+Z mutates the world, `F` recentres the camera — with game input not suppressed either, so both fire. The staging + write-back also run every frame for a lingering selection with no editor showing. | `src/app/editor/ui/mod.rs:227-241`, `src/app/editor/ui/shortcuts.rs:22-83`; ctx always present: `src/app/window.rs:823-846`, `src/app/schedule.rs:543` | Unit test: `mode = Off`, select `e`, a headless frame with `Key::Delete` in `RawInput.events` around `update_editor_ui` → `e` alive (despawned today). → **DONE v0.156.12** — gated on `mode != Off`, staging included; the row's test as written, with Overlay as the control. |
| ~~**Merely displaying a `ParticleEmitter` or `PointLight` in the inspector rewrites any field outside the panel's `DragValue::range`**~~ — egui 0.34's default `clamp_existing_to_range = true` writes the clamped value back and marks it changed without interaction, and these panels bind the world value directly. A rain emitter at `spawn_rate 4000, max_per_frame 10000` (the component's own doc's numbers) reads `2000 / 8192` after one look; `PointLight.intensity 20` → `10`, `radius 5000` → `4000`. The reflect grid carries no ranges, so two editors for one field disagree. Same mechanism as the colour-keyframe row. | `src/app/editor/ui/particle_panel.rs:32,41,48,70,76,85,91,117`, `src/app/editor/ui/lighting_panel.rs:31,39,47,66,79` | Unit test: a headless frame drawing `particle_tuner_grid` for an emitter at `spawn_rate 4000` → still `4000` after `end_pass`. Fix is `.clamp_existing_to_range(false)` on every bound `DragValue` in the three panels. → **DONE v0.156.13** — `.clamp_existing_to_range(false)` on every bound `DragValue` in both panels; the row's test as written (4000 / 10000 and 20 / 5000 read back unchanged). |
| ~~**Expanding the "Ambient Light" header inserts `AmbientLight::default()` (WHITE × 0.1), and the resource's presence *is* the lighting-pass switch**~~ — so a game with no lighting renders at 10 % brightness from that moment on, with no editor path to remove it. Collapsing the header changes nothing; dragging intensity to 1.0 restores brightness but the pass stays on. The doc says only "so the control is always usable". | `src/app/editor/ui/lighting_panel.rs:53-56` → `overlays.rs:122-134`; default `src/resources/render.rs:58-64`; gate `src/app/render/post_lighting.rs:157` | Eyes in a windowed run, or a `render`-job capture before/after expanding the header (`tests.rs:388-404` asserts the insertion as desired and cannot see brightness). Decision: an explicit "enable lighting" button, or insert at intensity 1.0. → **DECIDED + DONE v0.156.21 — an explicit button.** The user chose separating it over inserting at intensity 1.0 or adding a remove button: the first two leave the pass switched on by looking, and only this one makes turning it on visible. Drawing the control inserts nothing; a headless-frame test asserts that, with an existing resource as the control. |
| ~~**Ctrl+D copies a different component set than Ctrl+C/V and detaches children**~~: `clone_entity` copies only `register_clone`d types (no `PointLight`, `ParticleEmitter`, `Parent`, `Children`) while paste round-trips through the serde registry plus the parent tag; the doc's "clone all components" is false, and `entities_tab.rs:55-75` is a second hand-rolled spelling of the same operation. Duplicate a child at local (16,0) under a parent at (500,300) → the copy is a root ~500 px away. | `src/app/editor/ui/shortcuts.rs:172-187`, `src/ecs/world/clone.rs:25-61`, `src/app/editor/ui/docked/entities_tab.rs:55-75` | Unit test: duplicate an entity carrying `PointLight` and a `Parent` → both on the clone. → **DECIDED + DONE v0.156.23 — match the paste path's outcome**, which is what the user chose. ⚠️ Doing it *literally* would have traded one loss for another: `clone_entity` carries `AnimationPlayer` and `Timer`, neither of which derives `Serialize`, so an `EntityDef` cannot hold them. The duplicate takes all three sources — clone, then the serde components, then the parent by **handle** rather than by tag. The Entities tab's second spelling now calls it. Redo stays lossy (the def cannot carry the clone-only two), and says so. |
| ~~**`F` centres the camera on the selection's *local* `Transform.position`, not the `GlobalTransform` it is drawn at**~~, so focusing a child goes to the wrong place. Same family as the gizmo / overlay `GlobalTransform` rows. | `src/app/editor/ui/shortcuts.rs:258` | Unit test: attach C to P, one `HierarchySystem` run, focus C → `cam.position == (516,300) - viewport/2`. → **DONE v0.156.11** — GlobalTransform first, Transform as the fallback; the row's test with a root control. |
| ~~**Ctrl+C / Ctrl+V never reach `key_pressed` on Windows and Linux**~~: egui-winit turns `command + C/V` into `Event::Copy` / `Event::Paste` and returns *before* pushing the `Key` event, and `command` is Ctrl there — they work on macOS only because `command` is Cmd there, so Ctrl passes through as a plain key. ⚠️ Confirmed in egui-winit 0.34.3 `lib.rs:1013-1030`, `:1311-1321`. The cheatsheet advertises both. `Event::Paste` is emitted only when the OS clipboard is non-empty, so a paste-key fix has to live with that. | `src/app/editor/ui/shortcuts.rs:34-35` | Eyes in a windowed run on Windows or Linux — CI only *builds* there. Fix direction: match `Event::Copy` / `Event::Paste` in `i.events` alongside the key. → **DONE v0.156.26 — and the gate this row named was wrong.** It said "eyes in a windowed run on Windows or Linux"; `egui_winit::is_copy_command` is `modifiers.command` with **no `cfg`**, so the behaviour is identical on every platform and the events go straight into a headless frame. Reading one file in the registry settled the premise; two tests settled the fix. ⚠️ Third time in this program a written gate over-estimated its cost, and the first where that alone is what kept the row deferred — re-derive the gate, not just the cause. |
| ~~**Ctrl+Z / Shift+Z / D / S fire while an egui text field has focus**~~ (the comment calls this deliberate); egui's `TextEdit` handles its own Ctrl+Z and does not consume the event, so on Windows/Linux Ctrl+Z in the rename box is a double action — the last world command is undone and `sync_selection_after_history` moves the selection while the box is still bound to the old row. | `src/app/editor/ui/shortcuts.rs:23-26`, `:27`, `:32-37` | Unit test: headless frame 1 shows a `TextEdit` with `request_focus`; frame 2 injects `Key Z + ctrl` → `undo_len()` unchanged. A decision as much as a defect. → **DECIDED + DONE v0.156.24 — blocked while typing**, the option the user chose over keeping the documented behaviour. One gate for all nine keys instead of three `&& !typing` at the call sites, which is how the other six came to be exempt. The row asked for the headless two-frame test and it needed exactly that: focus is carried across frames, so a same-frame `request_focus()` is not yet in effect when the shortcut is read. |
| ~~**Pasting a copied parent+child pair attaches the child copy to the *original* parent**~~ — the tag lookup is first-wins and the original precedes the fresh spawn — so the pasted parent has no child. Known as a tag limitation on the delete-undo path (`history.rs:28-32`); unmentioned on the paste path. | `src/app/editor/ui/shortcuts.rs:112-132` → `src/prefab.rs:310-334` | Unit test: copy P+C, paste → `Parent(C′) == P′`. → ❌ **FALSE POSITIVE, settled by running it (v0.156.17).** The copy's parent is the copy, and so it is on a second paste with two candidates already in the world. `World::query` walks archetypes in creation order, and the fresh copy has no `Children` yet, so it sits in an earlier archetype than the original. ⚠️ The right answer therefore rests on an ordering nothing stated, so two tests pin it — sabotage-verified on the lookup (last-match reddens one, no-lookup reddens both). A batch-local map was written and dropped: removing it again left the tests green, and an unverifiable change to working code is what the sabotage rule refuses. |
| ~~**Scene-tree drag-to-reparent moves the entity on screen**~~ (local `Transform` kept, no world-position compensation) **and `reparent.rs` never says so** — the module and method docs describe only the graph edit; `hierarchy.rs:145-147` documents the shift. A root at (500,316) dropped onto a parent at (500,300) draws at (1000,616). | `src/app/editor/ui/reparent.rs:1-7`, `:17-22` | A decision: preserve world position (unit test: `GlobalTransform(C)` unchanged after `editor_reparent` + one hierarchy run), or one sentence on `editor_reparent`. → **DECIDED + DONE v0.156.25 — preserve the world position**, which the user chose over documenting the shift. New `hierarchy::reparent_keeping_world_position` and `hierarchy::world_to_local_position`; the gizmo's own inverse now calls the latter, so the two conversions are one. ⚠️ Undo and redo had to go through it as well — the drag re-derives the local position, so a plain `reparent` on undo would move the entity rather than put it back. The row's test, with the raw `reparent` as the control. |
| ~~**The cheatsheet says "Delete"; Backspace deletes too.**~~ | `src/app/editor/ui/shortcuts.rs:39` vs `:220` | Reading; one row of text. → **DONE 2026-09-03** — the row reads "Delete / Backspace". |

**Killed by reading — recorded so they are not re-chased.** Each looked wrong and is not:

- **Tilemap overlays ignoring the map entity's own `Transform`.** Tiles are spawned as *root*
  entities at absolute `cell_center_world` (`src/tilemap/system.rs:84-96`), so `tm.origin` is the
  truth; both the pathfinding overlay and the grid readout are right to ignore it.
- **Grid overlay coordinate frame.** `Camera::screen_to_world` uses no viewport size and
  `world_to_screen` is viewport-relative; `Camera` has no rotation field (`rg -n rotation
  src/camera.rs` → 0), so an axis-aligned grid is right.
- **`loading.rs:9` "the editor calls this after a Tile Paint stroke" — true**:
  `commit_paint_stroke` calls `sync_tilemap_colliders(owner)` (`tile_paint.rs:366`); undo/redo call
  the free function (`history.rs:293`, `:395`).
- **"Does the load path sync tile colliders?" — there is nothing to sync.** No serde registration
  exists for `Tilemap`, `TilemapColliders` or `PhysicsBody` under any name, and `TilemapColliders`
  has a private index, so a RON scene cannot spawn one. The load-side defect is the *leak* (row 4).
- **A failed `load_*` skipping the watcher registration** is coherent: `reload_path` dispatches by
  matching a *registered* table's stored path, so a watch without an entry would be `NotFound`
  anyway. Consequence — a mis-parsed file needs a re-`load_*` or a restart — is undocumented but
  nothing claims otherwise.
- **An Overlay-mode drag crossing a floating egui window is not abandoned.** egui 0.34.3's
  `wants_pointer_input` is `is_using_pointer || (over_egui && !any_down)`, so a held drag keeps
  flowing; only the docked gate had the defect (now v0.156.6).
- **A stuck mouse button after a docked drag exits the panel** — `window.rs:363-372` forwards the
  release outside the rect. And `Focused(false)` calls `release_all`, so Freehand's finish runs on
  the next frame; the Bucket row needs an actual *tool switch*, not a lost release.
- **`reload_scene` is synchronous** (`World::new()` + history clear in one function), so
  `world_reset_clears_editor_undo_history` and `a_world_reset_keeps_the_copy_clipboard` — and the
  latter's positive control — are sound.
- **`PaintTiles` "every cell the stroke actually modified" is accurate**: `set_tile` returns `true`
  only in bounds *and* changed, and `apply_paint_cells` pushes only on `true`. `flood_fill`
  terminates and is bounded (`seen` gives each cell one visit); `rect_cells` / `brush_cells` cannot
  underflow because every input comes from `cell_at_world`, which rejects the far edge too.
- **Data-table edit + delete in one frame is ordered correctly** (edits by snapshot row index,
  then add, then delete); the "edit dropped because the row lacks the column" branch is
  unreachable — `parse` fills missing columns with `Unit` and `add_row` builds every column.
- **State-machine panel remove-state guards match the model** (`remove_state` refuses
  current/last/nonexistent; the buttons are gated identically). Its `clip` and crossfade
  `DragValue`s clamp a local snapshot and push only on `changed()`, so the colour-keyframe row is
  the only place `clamp_existing_to_range` bites.
- **`retained_bytes_counts_a_paint_payload_exactly`** restates the implementation's formula — a
  spec test; it would still catch a zero or a wrong element size.
- **`entity_to_def` "mirrors `do_save_scene_with_list`'s parent resolution"** — both map `Parent`
  → parent's `Tag`; the scene saver additionally warns and counts dropped links. No drift.
- **theme.rs**: every constant has ≥ 1 use; `i18n`: `tr` on wasm is always Korean, deliberate and
  documented; stored `tr` output (status strings, toasts) staying in the old language after a
  toggle is a record of a past action, not a label.
- **The rotation drag crossing atan2's ±π seam** adds 2π to `rotation` — visually identical; the
  inspector shows 270° instead of −90°. Harmless.
- **Context-menu Delete bypassing history or the cascade** — no: it routes through
  `editor_delete_selection` (`shortcuts.rs:146-170`), which records `subtree_to_defs` and calls
  `despawn_recursive`; Duplicate / Focus / Rename likewise go through the shared ops.
- **`paint_value` clamp spelled twice** (`inspector_tab.rs:100-102`, `tile_paint.rs:147-155`) —
  both bound to `atlas.columns.saturating_mul(rows)`, and `0` = erase is a valid floor on both.
- **Add / Remove / Paste component and Break Prefab not undoable** — by design, documented at
  `editor/prefab.rs:69-70`. Paste *of a component already present* is the write-back row instead.
- **A stale `sel` inside the inspector** — liveness is checked at `ui/mod.rs:177-183`, and every
  despawn path that can run earlier in the frame clears or re-targets the selection. The one-frame
  mismatch between `comp_fields` and the rest of the body is cosmetic and guarded.
- **`tag_name_editor` drawn twice per docked frame** — different panels, different egui Ids, and
  `changed()` fires only on user edits. Its write *is* reverted — that is the write-back row.
- **Scene-tree row clicks on a `dnd_drag_source` response** — egui 0.34.3 returns
  `dnd_response | response` with the inner label carrying `Sense::click`, so click, double-click
  and the context menu all work.
- **`scene_tab.rs:30` `let _ = scene_graph_data;`** — an unused parameter, not a silenced
  observable; the data is consumed via `root_entities` / `children_map`.
- **The toast queue and its timing** — capped at 5 by draining the oldest (asserted on real
  observables), aged by the raw frame `dt` handed to `post_systems`, so wall-clock at any fps;
  under `ENGINE_CAPTURE` the fixed 1/60 is by design.
- **The audio sliders clamping on view** — `Slider` does clamp on display, but `mark_changed`
  compares against the already-clamped old value, the panel writes only on `changed()` against a
  local copy, and `set_bus_volume` clamps at the source. The one panel here that is safe.
- **The write-back guard against a selection change mid-frame** works: every undo / redo / load /
  delete path either changes `inspector_selected` (the guard skips) or leaves it alone. The
  write-back row is the *same-entity* case, which the guard cannot see.
- **`F` undone by camera follow** — `Camera::update` re-lerps toward `follow_entity` every frame,
  so F is a one-frame nudge in a following game; the game's follow policy winning reads as intended.
- **Entity ordering** — both list sites use `World::entities_sorted()`; no hand-rolled sort remains.
- **`editor_reparent`'s docs** — cycle / self / no-change → `false`, nothing pushed, no toast,
  matching `hierarchy::reparent`; its six tests assert real observables.
- **Focus math for roots** is correct (`Camera.position` is top-left; Docked `ViewportSize` is
  the central rect); the `F` row above is the child case only.

### Open — the 2026-08-28 `src/app/editor` review's remainder

⚠️ **This review is PARTIAL and the number is the point.** `src/app/editor` is 36 files /
**9,792 lines** (9,195 non-test), and roughly **4,700** of them have been read — `state.rs`,
`history.rs`, `docked_rt.rs`, `component_registry.rs`, the gesture paths of `ui/gizmo.rs`, the stroke machinery of `ui/tile_paint.rs`, `ui/docked/save_load.rs`, `ui/docked/entities_tab.rs`, plus
the cross-subsystem call sites they reach (`schedule.rs`, `render/docked.rs`, `window.rs`,
`ui/docked/mod.rs`, `rename.rs`, `prefab.rs`, `core_resources.rs`). **Not yet read**:
`ui/gizmo_math.rs`, the drawing half of `ui/gizmo.rs`, the cell-set helpers and tests of
`ui/tile_paint.rs`, `ui/mod.rs`,
`loading.rs`, `ui/state_machine_panel.rs`, `ui/data_table_panel.rs`, six of the seven `ui/docked/*`
files, `overlays.rs`, `prefab.rs`, `settings.rs`, `i18n.rs`, `theme.rs`.
Finishing it is a continuation, not a new review — **and it was finished on 2026-09-02**; see
*Open — the 2026-09-02 editor review continuation* below for what that read found. The line
counts above are the 2026-08-28 figures; the subsystem is 10,893 lines now.

It was scoped by measurement, not by hunch: at **10.6 tests per 1k lines** the editor had the
lowest density in the repo — the renderer sat at 10.7 before its own review found 21 items.

⚠️ **Both "unproven" rows are now settled by running them, and so is the one filed as
*read-only*.** The stroke-buffer row was right that something was broken and wrong about how it
was reached (v0.155.8). The cap row was right and now has numbers, which turned it into a
decision. And the `clear()` row — the one whose *"what settles it"* said **Read** — was a false
positive that only running it could kill: the generation-check argument is sound within one world
and a scene reset builds a new one. **Three for three, the instrument was execution, not reading**,
in a section whose recurring finding is stale prose.

⚠️ **Stale doc comments are the recurring shape here, not a one-off.** Three of this review's
findings are a comment describing behaviour the code does not have — the `clear()` justification,
the "until package 2" trio, and `update_tile_paint`'s claim that painting does not sync tile
colliders (fixed in passing; `commit_paint_stroke` has synced since, and `loading.rs:9` said so
all along). In a repo that cites its own docs as evidence, that is the finding, not the noise.

**Seven have shipped.** v0.155.8 — a tile-paint stroke abandoned by its selection was carried
into the next stroke on a different tilemap, so one undo wrote the first map's cells into the
second. v0.155.7 — `RtDebounce`'s three-stable-frame rule did not survive a
docked-mode round trip, because the teardown reset hung off a texture that did not exist yet.
v0.155.6 — the docked viewport's degenerate-window disagreement, below;
the margin subtraction lived in two places and the two spellings disagreed about windows too small
to hold a central panel, so `ViewportSize` published a 1x1 for frames the renderer skipped.
v0.155.5 — 📂 Load cleared the selection but not the undo history, so one
Ctrl+Z after a scene load resurrected an entity from the scene that had just been replaced (
`DeleteEntity`'s undo spawns unconditionally, so generation checks cannot absorb it) and the next
save wrote it to disk. v0.155.3 — the six editor-addable components a scene could not carry.
v0.155.4 — a drag abandoned by the selection going away left `rotate_active` set, and the press
guard then refused every later gesture (`EditorState::clear_drag_state` now owns that policy for
all three abandon sites). All were wrong *behaviour*, which is why each shipped alone rather than
riding along with the cleanups; what is left below is mostly not. ⚠️ v0.155.6 is the exception that
proves the rule rather than breaking it — it carried three stale comments with it, because they
described the very code it moved.

| Item | Where | What settles it |
|---|---|---|
| ~~**`dispatch_fires_only_for_matching_entity` tests the test, not the code.**~~ The name and doc claimed the *draw closure* fires only when `presence` is true; the body never invoked `draw`, silenced its counter with `_ = counter;`, and asserted a `fire_count` incremented by an `if` written **in the test**. | `src/app/editor/ui/docked/inspector_tab.rs` | **DONE v0.156.5 — and the row's "a unit test cannot build `&mut App`" was wrong.** `App::new()` is headless and 22 editor tests already construct one; an `egui::Ui` comes from `Context::run_ui` with no window. So the split was to move the loop into `draw_registered_panels` and call it, not to extract a decision out of it. ⚠️ Two things reading did not find: `App::new()` ships **five built-in panels** (the first draft assumed an empty registry and failed on its own control), and the restore-by-assignment silently dropped a panel registered mid-draw — fixed with the move, since it is the line the loop was moved with. Sabotage: `presence` inverted reddens both tests; restore-by-assignment reddens only the second. |
| ~~**`EditorHistory::clear()`'s doc names a danger the ECS makes impossible.**~~ | `src/app/editor/history.rs` | ❌ **FALSE POSITIVE — the doc was right and the row was wrong, settled by running it.** The row reasoned that `Entity` is generation-checked and `despawn` bumps the generation, so a stale handle cannot match. True *within one world* — and a reset does not stay within one: `app/scenes.rs` does `self.world = World::new()`, so the new world's counters start at 0. Probe: an `Entity { index: 0, generation: 0 }` from the old world compares **equal** to the first entity spawned after the reset and reads that entity's component (`Some(99)`), while the same-world despawn→respawn pair compares unequal in the same run. The doc now carries that mechanism, and the load path's separate justification with it. ⚠️ **The row's own "What settles it" said *Read — both sides already read*, and reading is exactly what produced it.** |
| ~~**Two consumers of the same rect disagree on the degenerate window.**~~ | `src/app/editor/docked_rt.rs` | **DONE v0.155.6 — one function now, and the row named the smaller of the two halves.** Both call sites read `docked_rt::docked_viewport`, which returns the logical rect **and** its physical pixel size or `None`; the renderer skips the frame and `compute_viewport` **holds** the previous `ViewportSize` rather than inventing a 1x1. ⚠️ **The hand-rolled arm branched on the rect alone**, so a sub-pixel central panel — one egui itself had squeezed — was published verbatim while the renderer refused it as un-renderable. That half was not in the row. The margins, `compute_central_rect` and `rect_to_physical` are **private to the module** now, so the subtraction cannot be spelled a second time; none of them was public API (`docked_rt` is `pub(super)`, absent from `src/lib.rs`). Four tests with controls, two sabotages, each reddening its own half and no other. |
| ~~**Three comments say a migration is pending that has landed.**~~ | `src/app/editor/docked_rt.rs:1`, `src/app/editor/state.rs:280`, `src/app/render/docked.rs:99` | **DONE v0.155.6**, all three in that change rather than as a docs row of its own: two of them sat on the code that moved (the margins, now private; the `or_else` the renderer no longer spells), and leaving "until package 2 replaces them" on a line just edited is worse than the staleness it started as. They now say what is true — `ui/docked` writes the real panel rect every docked frame, so the margins are the fallback for the first frames of a session. |
| ~~**`RtDebounce::reset` is not called on the exit it documents.**~~ | `src/app/render/docked.rs`, doc at `src/app/editor/docked_rt.rs` | **DONE v0.155.7 — the one-line hoist, taken as the split the row asked for.** `docked_teardown(&mut RtDebounce, has_texture) -> DockedTeardown` owns the asymmetry (the restart is unconditional, the texture teardown is not) and **mutates rather than returning a plan**, so a caller handling only the texture cannot forget the reset. ⚠️ **Its reachability is thinner than the row implied and is now written down**: the stale count survives only when no RT exists at the exit, so it needs a mode exit within two frames of a still-unstable size — ~33 ms at 60 fps, a third of a second in a debug build at 6 fps. The doc claiming a call the code did not make is the part that was true at any frame rate. Three tests; the sabotage (both halves back on `has_texture`) reddens one, alone. |
| ~~**(new, surfaced by v0.155.6) A third consumer of `central_rect` disagrees on the *first* frames.**~~ | `src/app/window.rs` | **DECIDED + DONE v0.156.3 — align it.** `game_cursor` reads `docked_rt::docked_viewport` now, so all three consumers branch on one decision; `None` freezes the cursor, as leaving the panel already did. ⚠️ **The first attempt regressed the published-rect case** and a test caught it: requiring the surface up front also broke the path that never needed the window size. Only the fallback does, so the surface is optional and a zero window stands in — `compute_central_rect` refuses that, leaving a published rect to win alone. Headless tests cannot reach the fallback arm (no surface), which is stated rather than implied; the decision is unit-tested in `docked_rt`. |
| ~~**`EditorHistory` has no cap.**~~ | `src/app/editor/history.rs` | **MEASURED then DECIDED + DONE v0.156.1 — a byte budget, oldest-first.** The measurement is what chose the shape: one Bucket fill on a 256x256 map retains **1.50 MB** (65,536 cells x 24 B, no slack) so 50 retain 75 MB, while **1,000 freehand strokes retain 70.75 KB** all in. Four orders of magnitude between two ordinary gestures, so a count-based cap could not work; `budget_bytes` defaults to 64 MB. The newest command is never dropped — a Bucket fill alone exceeds most budgets and must still be undoable. ⚠️ The accounting is exact for `PaintTiles` and a **floor** elsewhere (`ron::Value` deep size is not walked), and `Default` is hand-written because the derived one set the budget to **0**. The row also missed that `EditorCmd` is **208 B** itself, sized by `DeleteEntity`'s inline `EntityDef`. |

| ~~**(unproven) The stroke buffer is not tied to the entity it was painted on.**~~ | `src/app/editor/ui/tile_paint.rs`, `src/app/editor/ui/gizmo.rs` | **DONE v0.155.8 — real, and the row's route was the wrong one.** The row supposed the selection moving to *another `Tilemap`* mid-stroke, and asked for that reachability to be shown first. It is reachable, but the path that bites goes through **`None`**: Ctrl+Z over a `CreateEntity` drops the selection with the button still held, `update_editor_gizmo` returns at its `let Some(sel) … else` arm every frame after, and `paint_active` stays set — which disarms the *next* stroke's `clear()`, because that is guarded on `!paint_active`. Probe on the bug: one undo put `B(0,0) = 1`, a tile only A ever had, and left A's own paint standing with no history entry. Fixed as the row's title says — `EditorState::paint_entity` owns the batch — plus two abandon sites that now **commit** rather than drop (the cells are on the map either way) and a guard that ends a stroke when the selection moves to another tilemap. Three branches, three sabotages, each red on its own assertion. |

| ~~**`reset_scene` clears `copy_clipboard`; the editor's scene load does not.**~~ | `src/app/scenes.rs` | **DECIDED + DONE v0.156.2 — keep it on both sides.** The row already suspected this ("clearing it on reset may simply be over-caution") and the reason holds: the clipboard stores `EntityDef` **values**, so it cannot retarget the way a handle-carrying undo command can — which v0.155.8's probe showed is a real hazard *for handles*. The intent is now stated where the `clear()` used to be. ⚠️ The test hangs on its control: "the clipboard survived" is equally true of a reset that never ran, so it stages an undo command the reset must drop and asserts that first. |

| ~~**Deleting a parent in the editor makes its children jump, and leaves them pointing at a corpse.**~~ | `src/hierarchy.rs`, `src/app/editor/ui/docked/entities_tab.rs`, `src/app/editor/ui/shortcuts.rs` | **DECIDED + DONE v0.156.0 — cascade-delete the subtree**, of the three options the row put up (cascade / reparent-to-root preserving world position / keep the jump). New public API `hierarchy::despawn_recursive` + `descendants`, both re-exported; both editor delete paths use it. ⚠️ **Undo needed a link `EntityDef` cannot express** — its `parent` is a *tag*, so `EditorCmd::DeleteEntity` now carries the subtree parents-before-children with the parent as an **index into its own def list**; the deleted root's own parent stays tag-based since it lies outside the subtree. The old jump is pinned by a test that drives `HierarchySystem` and asserts the orphan lands at `(16, 0)`, so the doc's justification cannot rot silently. |

**Killed by reading — recorded so they are not re-chased.** Each looked real and is not:

- **Stale `central_rect` after leaving Docked.** It is never cleared (only `EditorState::new` sets it `None`). Harmless: **every** reader is gated on `mode == Docked` — `window.rs:341`, `:400`, `:605`, `gizmo.rs:31`. All four checked.
- **Entity-id reuse in the undo stack.** Generation-checked; stale handles fail safely. See the `clear()` row for what this *does* break.
- **Adder/remover drift in the editor registry.** 27 adders, 28 removers, **zero** orphans; the extra remover is `Tag`, which has no adder by design and is re-addable through the rename box (`rename.rs:43` uses `add_component`).
- **A paint/undo collider asymmetry.** `history.rs` syncs tile colliders on undo *and* redo, and `tile_paint.rs` appeared not to — which would have made redo produce a better world state than the original paint. It does sync, at `commit_paint_stroke`. The grep that "proved" the absence searched for the free function `sync_tilemap_entity_colliders` and missed the `App::sync_tilemap_colliders` wrapper the call actually uses.

⚠️ **Five greps were wrong across this review, and each looked conclusive.** Four of them are
recorded in the rows and bullets above; the fifth is the collider one just below. The pattern is
always the same — a *narrower* pattern than the code, reported as an absence. The name-contract diff read
"9 components unregistered", then 7, then 6 — a line-based pattern missed multi-line
`registry.register::<T>(\n "Name",` calls, and a second version conflated the serde registry with
`register_reflect_named`. What settled it was a per-name check **with a control**: the six score 0 in
both registries while `Button` (2/1) and `Panel` (4/1) score non-zero, proving the instrument can
tell them apart. The lesson is the repo's own — a filed diagnosis is a hypothesis, and so is the grep
that produced it.

### Open — the 2026-08-19 render review's remainder

A full read of the render subsystem (`src/renderer/**` + `src/app/render/**`, 10,549 lines across
41 files, WGSL included) on 2026-08-19. **Three shipped as v0.152.9** — the `interleave_runs` NaN
hang, the draw queues surviving a skipped frame, and the hot-reload sRGB downgrade. **A fourth
shipped as v0.153.1** (the `debug_draw` step-count hang). **Eight more shipped as v0.153.2** — every
row a read or a CPU-side measurement could settle. **One as v0.153.3** (the docked transition
aspect, closed by the capture it asked for). **Two as v0.154.0** and **the last six as v0.154.1**,
which also added a seventh nobody had filed. The rest is below, split by what settles it. **None is
gated on a decision**; they are ordered by how cheaply each can be proven, not by how much they are
worth.

✅ **This review's remainder is now empty.** The last two — the bloom pipeline rebuild and
`load_texture_with_format` ignoring `format` on a cache hit — closed as v0.154.2. Nothing here is
open; **do not reopen this section as a source of work**, the way `docs/PROGRAM_HISTORY.md` says of
the 2026-08-07 analysis. A new pass over `src/renderer/**` would be a new review, and should be
scoped and named as one.

⚠️ **Two of the final four rows were closed by contradicting them**, which is the note to carry
forward rather than the fixes: the offscreen camera swap was a false positive, and
`load_texture_with_format`'s "cache key has no format" was a correct observation attached to a
wrong remedy — the key *cannot* carry a format, because a sprite names a texture by path alone.

⚠️ **"What is left is exactly what needs hardware" was written here three times and was wrong
every time.** It claimed the UI-primitive allocations needed the `render` job (v0.154.0 measured
them with a standalone counting allocator — the functions take no device), that `TextCacheStats`
needed a game to exercise it (v0.154.0 built the check in `tests/render.rs` with no game), and
that the whole v0.154.1 batch below needed a GPU (six rows, all closed headlessly). The rule that
keeps surviving: **ask what the claim actually depends on, not what module it lives in.** A pure
function in a GPU file is still a pure function, and a `pub(super)` type can be split out until a
unit test reaches it — which is how `rt_registration`, `split_material_entities` and
`reload_format` all became testable.

What genuinely needs hardware, as of 2026-08-20, is **one row**: the bloom `resize` timing claim,
which needs a live windowed drag. Its *fix* does not — that is ordinary code.

The efficiency rows are the ones to be careful with. This repo's own habit — see the closed section
below — is that an efficiency claim that has not been measured is a hypothesis, and five of them
reversed under measurement between v0.150.7 and v0.152.5. Two rows here **are** measured and say so;
treat the rest as unproven until the named instrument runs.

| Item | Where | What settles it |
|---|---|---|
| ~~**`Arc::from("")` per untextured sprite per frame.**~~ **DONE v0.153.2 — the row was right, its byte figure was not.** Re-measured with a counting allocator: 1000 allocations / **16,000 bytes requested** per 1000 sprites, all pointers distinct → 0 and 1 after interning in a `OnceLock`. The row's "32 KB" is the allocator's bucket, not the requested layout. ⚠️ Its *justification* also expired — "`Sprite::colored` is common across the examples" cites a tree deleted the same day; the measurement stands on its own, the impact estimate does not. | `src/renderer/sprite/collect.rs:74` | ~~Measured already~~ — re-measured, plus a pointer-identity test |
| ~~**The shaped-text cache keys on `position` where position provably cannot affect layout.**~~ **DONE v0.153.2 — and the fix is bigger than the row asked for.** Keying on the computed `(width, height)` is right, and it turns out `position`, the viewport, `bounds` **and** `anchor` all reach shaping *only* through that pair — `shape_text` is a pure function of `ShapeSpec` and the `FontSystem`. So the key is now `ShapeSpec` field-for-field and those four dropped out entirely. Two wins the row did not name: bounded text survives a resize in the cache, and two anchors landing on the same layout size share one shaping. `cache_key_miss_when_position_differs` moved with it, as predicted. | `src/renderer/text/cache.rs:26`, `src/renderer/text/renderer.rs:406` | ~~Read off the two pure functions~~ — key tests moved, plus a real-font test comparing every glyph at two positions |
| ~~**The UI-primitive path allocates 4 `Vec`s + a `String` per image, per frame.**~~ **DONE v0.154.0 — and it never needed the GPU.** The row said "needs a GPU (`prepare_ui_primitives` takes a device) — the render job, or extract the sort". Wrong: `sorted_ui_primitives` and `DrawImage::texture_key` are **pure functions taking no device**, so a standalone counting allocator measured the whole thing, exactly as v0.153.2 measured `Arc::from("")`. A HUD frame went 13 allocations / 16,800 B → **2 / 640 B**; a 40-image frame 45 → **2**. The residual 2 are `keys`/`zs`, which leave the renderer as `PreparedUiPrimitives` and so cannot be scratch — constant, and independent of how much UI is on screen. ⚠️ Two findings the row did not have: the per-image cost was a `String` copy the sprite path had **already** fixed via `Arc<str>` (now mirrored, breaking `DrawImage.texture`), and **both** z-sorts were `sort_by`, whose temporary allocation depends on element size as well as count — a 16-byte proxy reads "safe below 1000" while the engine's ~112-byte elements allocate from ~32. Both are `sort_unstable_by` now; the comparators end in a unique `order`, so stability was never load-bearing. |
| ~~**`BloomRenderer::resize` recompiles the shader and all four pipelines.**~~ | `src/renderer/bloom.rs` | **DONE v0.154.2 — the row was right, and the timing claim it asked for is still not made.** The size-dependent half is now a `Pyramid` struct with its own `build`, and `resize` rebuilds exactly that. ⚠️ **The split is enforced by construction, not by care**: the shader module and pipeline layout are locals of `new` and do not exist afterwards, so `resize` *cannot* rebuild a pipeline even by mistake. `reconfigure` still takes the wide path, correctly — a format change needs it. `prefilter_ub` survives a resize despite a size-derived `texel`, because `update` rewrites the struct every frame and the frame calls it immediately before `run`. **What is claimed is countable by reading — one shader compile and four `create_render_pipeline` calls per resize event, now zero — not a wall-clock number**, which still needs the windowed drag this repo cannot automate. The pyramid arithmetic came out as a pure `pyramid_dims`, the only part of the bloom renderer testable without a GPU; four tests, including the cap binding on a huge scene *with a control that the last level is still healthy*, and a sabotage-verified guard that a degenerate scene still yields one non-zero mip (`mips[0]` is indexed unconditionally). |
| ~~**The GPU-particle renderer is never torn down.**~~ **DONE v0.153.2 — by stopping the work, not the renderer.** The per-frame cost was the point, and the pass now runs only while there is something to simulate. Tearing the renderer *down* was rejected: its pipelines and buffer are a cache, so a game whose emitters blink off would trade a per-frame cost for a shader recompile. ⚠️ The gate is **not** `has_emitters` — the frame after the last emitter despawns, its particles are still alive. Death happens on the GPU, so liveness is bounded CPU-side by counting the longest uploaded `life` down by the same `dt` the shader uses, which errs only towards "maybe alive". The capacity half is fixed too: a `GpuParticleConfig` change now rebuilds (discarding particles in flight). | `src/app/render/frame.rs:454`, `src/app/render_state.rs:56` | ~~Read; cost needs the render job~~ — read; the *saving* still needs the render job to quantify |
| ~~**One `queue.write_buffer` per new particle.**~~ **DONE v0.153.2.** The row's reasoning held exactly: uploads are grouped into contiguous ring runs, so an emission is one write, or two across a wrap. Four tests, including the control that non-consecutive slots are never merged — merging them would overwrite the particles in between. | `src/app/render/frame.rs:487` | ~~Read~~ — read + 4 tests |
| ~~**The docked transition overlay uses the surface aspect.**~~ **CLOSED 2026-08-20 — fixed v0.153.3, proven by the capture the row asked for.** The diagnosis held exactly, and the correct value turned out to be in the same function already: the text pass's target size, which was *named* `text_w`/`text_h` and therefore read as text-specific, so the transition re-derived it from `gpu.config` instead. Renamed `scene_target_w`/`_h` with a comment saying any pass drawing into `scene_target` must use it; the general lesson is in `docs/PATTERNS.md` § *The target's size is not `gpu.config` either*. The gate is `tests/render.rs::docked_iris_is_a_circle_in_the_docked_target` (phase 2 of the examples rebuild): it captures a mid-`IrisIn` docked frame and measures the hole's chords — **296x296, roundness 1.000** at a 1000x460 window whose docked viewport is 392x372. ⚠️ Both halves are sabotage-verified: against the pre-v0.153.3 aspect it reports **236x372 (ratio 0.634)**, and at a window size where the docked RT and the surface share an aspect (1382x200 -> viewport 774x112) the **control** assertion fires rather than passing vacuously. | `src/app/render/frame.rs:725` | ~~A docked screenshot~~ — done, in the render job (it needs a GPU, and the selftest runner tolerates no skips) |
| ~~**`live_material_entities_scratch` excludes `Hidden`, and two comments above it say it does not.**~~ **DONE v0.153.2 — the comments were right.** Keeping a hidden entity's buffers is the cheaper behaviour and was the stated intent; the code was the half that was wrong. Both sets now come from one pass, split into a free function so a test can reach it — `SpriteRenderer` needs a GPU to construct, so the fix was otherwise unprovable. | `src/renderer/sprite/collect.rs:232` | ~~Read~~ — read + a test over `&World` |
| ~~**`setup_lighting`'s arm order swallows a same-frame cap change.**~~ **DONE v0.153.2.** The exclusive `match` arms became independent steps decided by a pure `lighting_fixups`. The ordering needs no re-checking and the doc says why: `reconfigure` already rebuilds at the new size, `set_max_lights` preserves size and format, `resize` early-returns on a match. Five tests, including the control that an unchanged frame rebuilds nothing. | `src/app/render/post_lighting.rs:120` | ~~Read~~ — read + 5 tests |
| ~~**`load_texture_with_format` ignores `format` on a cache hit**~~ | `src/renderer/sprite/textures.rs` | **DONE v0.154.2 — the ignoring is correct and stays; the silence was the bug.** ⚠️ **The obvious fix does not work.** Putting the format in the cache key cannot: the sampling side reaches a texture through `bind_group_for_texture_key`, whose only input is the string on `Sprite.texture`, so a format-carrying key has nothing to match against and one path would map to two textures with no way for a sprite to choose. Lifting this means changing how sprites *name* textures — a design change, not a bugfix. So a second format is still ignored, and now logs: a file pulled in as sRGB colour and then registered as a linear data texture (normal map, mask, LUT) via `App::load_image_with_format` came back sRGB-decoded and subtly wrong with no diagnostic, and the two colliding calls are usually in different files. Detected across the whole **alias set**, so two spellings of one file collide visibly too. **`reload_format` needed no revisiting** — the row warned it might, because v0.152.9's hot-reload fix depends on a path holding one format; it still does, and that invariant is now stated and enforced rather than merely true. Decided by a pure `cached_format_verdict`; both conflict directions tested, with controls that an agreeing format and an uncached path stay silent. |
| ~~**A non-finite debug-draw coordinate hangs the frame.**~~ **DONE v0.153.1 — and the row named one of three failure modes.** The hang was real and exactly as described. But the same saturating cast sends a **NaN to `0`**, so a NaN never hung — it drew a single garbage quad at NaN coordinates, a second bug the row did not see. And a **finite** `len` is not safe either: `1e18` asks for 9.4e17 iterations without saturating anything. So the guard the row implies — reject non-finite input — is half a fix; the step count is capped as well, which is what makes termination a property of the loop rather than of its input. ⚠️ Also: the finiteness check belongs on the **length**, not on the endpoints. Two *finite* endpoints far enough apart overflow `length()` on their own, since it squares them. | `src/app/render/debug_draw.rs:62` | ~~A unit test (pure function)~~ — six added, eleven existing untouched |
| ~~**The lighting bind-group cache keys on a reference's address.**~~ **DONE v0.153.2 — made local, not removed.** Recreating the intermediate now calls `LightingRenderer::invalidate_bind_group` explicitly, so correctness no longer rests on the standing invariant that every such path also resizes or reconfigures. The pointer check stays as a same-frame fast path, with its doc now saying plainly that the address identifies the caller's *slot*, not the view. Still unreachable today; the change is that it cannot become reachable silently. | `src/renderer/lighting.rs:479` | ~~Read; latent~~ — read; no test (the failure needs a GPU to observe) |
| ~~**`RenderStats::draw_calls` counts only the sprite pass.**~~ **DONE v0.153.2 — scoped, not counted.** Counting the UI pass means adding a parameter to the public `render_ui_primitives_from_slices`, and even then the text pass draws through `glyphon`, whose internal draw count the engine cannot observe **at all** — so no honest frame total is available. The field's doc and the Engine Stats label ("sprite draw calls") now say what the number is. | `src/renderer/sprite/draw.rs:77` | ~~Read~~ — read; doc + label only |
| ~~**Smaller, no home of their own** (five items)~~ | — | **ALL FIVE SETTLED v0.154.1 — two were real, two were not leaks, one was a false positive, and reading them found a sixth.** ⚠️ **The nine-slice one was worse than filed.** The row said `min_pixel_size` "can drop a panel's corners while its centre stays"; it drops the corners **and all four edges**, of a panel of *any size*, because eight of the nine sub-quads measure the **border** — which does not grow with the panel. A 5000x3000 panel with an 8 px border under `min_pixel_size: 16.0` drew as one bare centre quad. LOD is now tested once against the panel (it is one *sprite*, which is what `CullConfig` documents); the frustum test stays per-sub-quad. The premise was already in-tree and unread — `corner_keeps_border_size_independent_of_panel_size` had been asserting exactly the independence that makes the bug work. `upload_particles` now logs the offset/required/capacity instead of dropping in silence (it is `pub`, so a game's own emitter hit this with no diagnostic). **`custom_pipelines` and `rt_cache` are not leaks** and now say so where they are declared: the first is keyed by *source* hash, so it grows with distinct shader sources compiled, never with entities or scene loads, and holding them across a reset is the point; the second mirrors `RenderState::render_targets`, which has **no removal path at all**, so it cannot outgrow its source — if destroying a render target ever lands, the fix belongs upstream. **The offscreen camera swap is a false positive**: `Camera` is `Copy` so nothing is removed, and v0.152.2's gap was dangerous only because `SystemPanicPolicy::DisableSystemAndContinue` catches a *system* panic and continues — the render stage has no `catch_unwind` above it, so a panic there takes the process. Recorded in the file with the condition that would make it real (an early exit appearing between swap and restore). **The per-texture sampler stays**: load-time, not per-frame, and removing it costs a new public constructor or a device-keyed cache for an unmeasured win — noted in place that `wgpu::Sampler` is `Clone`, so one shared sampler is the whole fix if a D3D12 descriptor-heap ceiling ever forces it (needs a Windows runtime to confirm; this repo has none). |
| ~~**`register_render_target` allocated a `String` per render target per frame.**~~ | `src/renderer/sprite/textures.rs:146` | **DONE v0.154.1 — found by reading the `rt_cache` row above, not filed by the review.** `render_offscreen_targets` re-registers every target every frame, always with an `Arc::clone` of the bind group already cached, and the method did `insert(key.to_string(), bg)` unconditionally — a `String` allocated per target per frame to overwrite an entry with an identical one. Now compares by pointer identity first; a genuinely rebuilt target (resized/reformatted, so a **different** `Arc` under the same name) still replaces, which a value comparison would have got wrong. Decided by a generic free function so a test reaches it without a GPU, the same split `reload_format` uses. |
| ~~**Two per-frame allocations found while doing the v0.153.2 batch**~~ | `src/renderer/text/renderer.rs:461`, `src/renderer/sprite/collect.rs:289` | **BOTH DONE v0.154.1, and the row's own "needs a GPU renderer" was half wrong.** (a) `PlainTextCacheKey` now holds `Arc<str>` with a `TextInterner` in front, so a lookup builds its key from a `&str` probe with no copy: a steady-state all-hit frame went **6 allocs / 63 B → 0** (6 HUD lines), **40 / 74 B → 0** (40 `FloatingText`), **12 / 686 B → 0** (12 dialogue lines). ⚠️ **The control is what chose the design.** The obvious two-level `HashMap<Arc<str>, HashMap<ShapeKey, _>>` also reads 0 on hits — and **13** on a frame of six all-new strings against the `String` key's 7, because each new string also allocates an inner map. The interner reads 8 there (parity + one amortised table growth). A score readout changes its text every frame, so that path is not a corner case; a fix checked only on the workload it was aimed at would have shipped a 2x regression. Interning needs its own eviction and has one — a string drops once no key refers to it (`Arc::strong_count == 1`), strictly after the buffer cache evicts. Both halves pinned by `Arc::ptr_eq` (equal text in two allocations compares equal — `assert_eq!` would pass on the bug) and both sabotage-verified red. (b) `mat_ids` is now `drawn_material_entities_scratch`, cleared and refilled beside its `live`/`seen` siblings; the consuming loop is indexed rather than iterated, since the element is `Copy` and the body needs `&mut self.material` — no `mem::take` put-back needed. **On the instrument**: the numbers come from a standalone counting allocator over the key-construction + probe path (the two lines that allocate), since a `TextRenderer` needs a GPU; the *correctness* is pinned in-crate against the real `TextInterner`.

### Open — the 2026-08-19 follow-up review's remainder

A review of everything v0.152.1–v0.152.7 changed (`25c49e5..13ce809`; 88 src lines, 484 test lines)
produced 13 findings. **Two shipped as v0.152.8** — the `move_entity` drop-order unwind hole and the
editor's `EntitySortMode::Insertion` that no longer meant insertion. **One was a false positive**
(the row above).

✅ **The nine below all shipped as v0.155.0 on 2026-08-24, and this section is now empty.** They
were held back because none changes what correct code does and bundling them would have buried the
two that did; once those shipped, the reason to keep them apart was gone. **Do not reopen this
section as a source of work** — a new pass over that diff would be a new review.

⚠️ **Two of the nine were worth more than "one edit each" implied, and both for the same reason:
the row named a symptom and the fix needed a measurement.**

- **The ratio tests.** The row said a spawn-only baseline "would make them mean what they say" and
  estimated ~0.13 of the 0.226 allocation allowance was structural. Measured, `spawn()` costs
  **0.010 allocations** per entity — the real distortion is ~0.004, negligible. But it costs **188
  bytes**, and on the *bytes* test that was decisive: subtracting it moved the healthy ratio from
  0.506 to 0.645 and, crucially, the reading under the bug the test names from 0.881 to 1.172. The
  old 1.25 bar could not fail on its own stated cause. It is 1.0 now and does.
- **`World::entities_sorted`.** Filed as a choice between an engine API and an editor-local helper.
  It resolved itself on reading: the policy is prescribed in the doc comment on `World::entities`,
  so the helper belongs beside it, and no `lib.rs` or `MODULE_MAP` change was needed because
  `World` is already exported.

The transferable half is the one this file keeps re-learning: **a filed diagnosis is a hypothesis,
and so is its size**. Both estimates above were right in kind and wrong by more than an order of
magnitude, in opposite directions.

| Item | Where |
|---|---|
| `with_resource_mut`'s two doc guarantees contradict each other on replace-then-panic (the replacement wins; the entry value's mutations are dropped). Untested combination. Also worth stating: a deliberate `remove_resource::<R>()` inside `f` is undone by the restore. | `src/ecs/world/resources.rs:41` |
| The self-label warning cannot tell the `.label(X).after(X)` typo from the shared-label barrier idiom — which the same release's own test blesses. `by_label` is built 30 lines above, so `by_label[l].len() > 1` separates them for free. | `src/ecs/schedule.rs:104` |
| A new test was inserted mid-doc-comment: `dangling_after_label_creates_no_constraint` has no doc, and `self_referencing_label_creates_no_constraint` carries four paragraphs about the dangling case. | `src/ecs/schedule.rs:290` |
| The take → put-back cost claim cites `tests/per_frame_alloc.rs`, which has no coverage of that path (`grep -c 'take_component' → 0`). The accounting is right; the citation cannot settle it. | `src/ecs/world/components.rs:97` |
| Both archetype-transition ratio tests divide a fixed `spawn` cost by `width`, so part of `wide < narrow` is structural. Solving the shipped numbers: ~0.13 of the 0.226 allowance is spent before `move_entity` is measured, and the bytes variant would pass if per-component bytes quadrupled. They do still catch width-scaling (sabotage-checked); subtracting a spawn-only baseline would make them mean what they say. | `tests/per_frame_alloc.rs:776`, `:832` |
| `debug_assert!(extracted.is_empty(), "scratch left dirty by a previous call")` is unreachable — the restore is unconditionally preceded by `clear()`, and a panic leaves the field as the empty `Vec` the `mem::take` put there. It reads as a re-entrancy guard it cannot be. | `src/ecs/world.rs:252` |
| `entity_list.sort_unstable_by_key(|e| e.index())` is copy-pasted at three call sites and prescribed by a fourth doc comment. n=4 now; a `World::entities_sorted()` or an editor-local helper would carry the policy the type system currently does not. | `src/app/editor/ui/mod.rs:250`, `:307`, `docked/save_load.rs:122` |
| `query3_mut`'s assert prints all three type names but not which pair collided — the eyeballing `query2_mut`'s message was written to remove. The check is already pairwise. | `src/ecs/world/queries.rs:123` |
| The batch's one genuinely breaking change (`query2_mut::<A, A>()` now panics) is documented in full prose but without the ⚠️ marker the file uses 20 times elsewhere — including twice in v0.152.6 for softer caveats. Presentation only; the `assert` is the right call. | `docs/CHANGELOG.md` 0.152.4 |

**One finding was rejected outright and is not in that table**: `query_added` / `query_changed` were
read as allocating a `Vec` on every call because `clear_change_tracking` retains keys with emptied
sets, so the `is_empty()` fast path rarely fires. The scan is real; the allocation is not — an empty
filtered iterator collects with a lower bound of 0 and never touches the heap. Only the word
"immediately" in that doc is overstated, and it predates this diff.

### Closed 2026-08-19 — the 2026-08-18 ECS review's efficiency remainder

A full read of `src/ecs` (2,626 lines, 14 files) on 2026-08-18 produced 15 findings. **Twelve
shipped** across v0.152.1–v0.152.4 — the take → put-back change-tracking bug, three panic-unsafe
remove → call → reinsert pairs, two `HashMap`-seed determinism leaks, and four silent failures made
loud. **One was a false positive** (see the closed row below). Three were the remainder, and they were
remainder *by decision*: every one is an efficiency claim, and this repo's own habit —
`tests/per_frame_alloc.rs`, the v0.151.1 debug-draw row — is that an efficiency claim ships with a
number or not at all. **All three now have their number, and all three are closed** — one shipped
as a fix (v0.152.5), one closed by the measurement it was gated on, one shipped as a cleanup
(v0.152.6). Nothing in this section is open.

**One of the three closed the same day (v0.152.5), and this row's own reading of it was wrong.** It
said the fix was `mem::take` on `move_entity`'s two `type_set` clones, "which would leave 5". The
clones were real but they were the *constant* half; the half that made building an entity O(N²) was
the fresh `HashMap` of extracted components, which grows with the entity's width. Measuring first —
which is what the row's own gate demanded — showed per-component cost climbing 5.01 → 5.75 → 6.50
as an entity widened, and the finished fix landed at **1.38–1.51, flat**. Writing the test first is
what turned a guess into a number.

**Both of the remaining two were measured on 2026-08-19, and the archetype row's stated mechanism
was wrong as well** — a third consecutive reversal in this section, which is the point of the
advisory rule in `CLAUDE.md`. Instrumentation and the `survivor` input script were kept in `.claude/`
afterwards, and **deleted 2026-09-04** once both were shown dead rather than assumed so: the patch
no longer applies (`git apply --check` exits 1 on `ecs/world/parallel.rs` and `queries.rs` — it was
cut against v0.152.x) and the script drove the **pre-deletion** `survivor`, which `survivor_game`
replaced without its `G`/`B` keys. They were gitignored, so nothing recovers them — but
re-instrumenting is a handful of `Instant::now()` lines, and this row demands a fresh measurement
anyway. The tree was returned byte-identical to HEAD after the original run.

| Item | State |
|---|---|
| **Empty archetypes are never reclaimed** | ✅ **MEASURED — do neither. Recommend closing.** The gate this row named ("nobody has counted archetypes in a real game") is now closed: all 22 game examples were run headlessly with a per-frame archetype dump. **The Vec is bounded and saturates.** `survivor` — driven through a real 1800-frame session by an input script (invulnerability on, `B` waves every 90 frames) rather than the 3.8 s death an unscripted capture gets — reaches **34 archetypes by frame ~150 and never gains another**, while its entities grow 2 → 665. `salvage_run` (against its live server, 109 streamed entities) reaches **4**. No other game exceeds 20. Nothing removes an entry, but the reachable set is the distinct signature *prefixes the game's own spawn code can produce*, which is a property of the code, not of runtime. **The row's mechanism was wrong.** The cost is not "a binary search across the whole Vec": a *non-matching* empty archetype is rejected by that search in a few ns. Only an empty archetype whose signature still **matches** costs anything — it passes the filter and pays two `HashMap<TypeId, _>` column lookups in the `flat_map` body to iterate zero entities, ≈**19 ns** each. `survivor` steady state: **115 filter passes per frame, 98 of them empty (85%)** → ≈**1.9 µs/frame**, **0.57%** of a 326 µs `App::update`. A/B with the one-line guard applied to all 14 sites: **328.2 µs guarded vs 326.3 µs baseline** (3 release runs each) — no measurable change, the ±1.4% run-to-run spread swamps it. ⚠️ An isolated micro-bench *did* show −17%, and it overstated the case: every prefix in that synthetic world contained the queried pair, so all 18 empty archetypes matched. Real games do not have that shape. **Do not reopen without a fork that actually has hundreds of entity kinds** — the scan is linear in archetype count, so a different regime would need re-measuring, but no game here is near it. |
| **`query2`/`query3`/`query4`/`query_opt2` index where their neighbours zip** | ✅ **CLOSED 2026-08-19 (v0.152.6)** — `query2`/`query3`/`query4` now zip, so the file has one spelling of one operation. Measured against a copy of the exact spelling it replaced: **2131 → 1748 ns per 650-entity pass, −18%** (0.59 ns/entity), reproducing to ±0.2% across runs. ⚠️ **The percentage is not portable**: an earlier four-variant harness put the same change at −8.5%, and since runs within each harness agree to ±0.2%, the spread is code layout between harnesses, not noise. Direction and per-entity size are what generalise. It shipped as a cleanup — nobody has shown it moving `App::update`. `query_opt2` keeps its index by design (`B` is optional, so its column has no per-element iterator to zip); its mandatory pair zips anyway. ⚠️ Indexing panicked on an `entities`/column length desync where zip silently yields the shorter; six sites already had zip's behaviour, so this made the file consistent rather than adding the exposure. **That follow-up shipped the same day (v0.152.7)**: `Archetype::debug_assert_columns_aligned` now guards all **fourteen** iteration sites — the row said ten and missed `parallel.rs`'s four, which zip the same data through rayon — and ships with a sabotage-verified test that makes it fire. |

### Closed — do not reopen without new information

⚠️ **All three of these sat in the table above marked closed**, which is the exact burial this file
exists to prevent: a reader scanning *Open — engineering* for work met 3 finished rows out of 5.
Kept verbatim rather than trimmed, because each carries a lesson whose only home is this row.

| Item | Verdict |
|---|---|
| **`<NAME>_SELFTEST` coverage** | **DONE — 10 of 21**, and the one real gap is now closed (`beat_crawler`, `survivor`, `data_anim`, `data_particles`, `salvage_run`, `predict_shooter`, `orbital_dodger`, `coin_race`, + `settings_menu` and `scene_flow` on 2026-08-06). The remaining 11 games' headline features are all visible in a screenshot (`sokoban`, `platformer`, `maze_escape`, `dig_quest`, `shooter`, `lit_dungeon`, `multi_terrain`, `tile_paint`, `ui_layout_editor`, `stat_editor_game`, `script_steering`), so chasing the number past 10 is effort against failures that are already visible. **Do not reopen this as a coverage target.** Durable findings from the four networked ones are in `docs/MODULE_MAP.md`'s `src/network.rs` row; the two that generalise beyond networking: **`InputState` has no public press setter**, so held input comes from `InputScript` (the `ENGINE_INPUT` replay path), keeping the real input read under test; and **assert an invariant, not an end state**, when a background process (a coin respawner, an entity spawner) can add to what you are counting. |
| **`DialogueChoice.cond` cannot express a conjunction** | ✅ **CLOSED 2026-08-10 (v0.152.0)** — `cond_all` / `cond_any` ship alongside `cond`, and `rpg_quest`'s `can_buy_lantern` workaround is deleted. ⚠️ **This row's own fix shape was impossible**, which is why it is worth reading twice: it specified `All([…])`/`Any([…])` *variants* on `DialogueCond`, "additive if the single-cond form still parses" — and in RON 0.8 those are mutually exclusive. An externally tagged enum rewrites every existing `cond: (var: …)` into `cond: Cmp((var: …))`; `#[serde(untagged)]`, the usual way out, cannot even re-read its own serialized output. **One throwaway test settled it in a minute; no amount of reading would have.** Durable homes: `docs/PATTERNS.md` § *Extend a type that is authored in RON*, `docs/CHANGELOG.md` 0.152.0. **Do not reopen for arbitrary nesting** — `a && (b || c)` is expressible, deeper trees need a helper var, and no example has asked. |
| **`System::name()`'s "anonymous" fallback** | ❌ **FALSE POSITIVE — the doc is correct, do not "fix" it.** The 2026-08-18 ECS review flagged `src/ecs/system.rs:10` (*An empty string displays as "anonymous"*) as documenting a fallback that does not exist. It does exist, in `src/app/editor/ui/mod.rs` as `tr("anonymous", "익명")` — and that is the only profiler renderer in the crate, so the doc holds for every path. The finding survived because the grep that "proved" absence was piped through `head` and truncated before reaching it. Recorded here because the next reviewer will grep for the bare literal and reach the same wrong conclusion: `rg -c 'anonymous' src/` (a count, not a listing) settles it. Same trap as `docs/VERIFICATION.md`'s trailing-`tail` rule, in a different tool. |
| **`take_component` → transfer → fresh `add_component`** | ❌ **FALSE POSITIVE — the classification is correct, do not "fix" it.** The 2026-08-19 follow-up review of v0.152.1–v0.152.7 read `add_component`'s `put_back_this_tick` check (`src/ecs/world/components.rs:157`) as mis-classifying a *genuinely new* component when the taken value had been handed to a different entity: `query_added` would miss it. It does miss it, and that is right. `query_added` means **first added this tick**, and the entity carried the component when the tick began — so a plain in-place replace reports `changed` too, and the take path agreeing with it is the consistent outcome, not the bug. Reversed by running it, not by re-reading it: a 30-line probe printed `added=[chest] changed=[hero]` and, in the same run, `replace: added=0 changed=1`. The one genuine asymmetry it surfaced is `remove_component` → `add_component`, which reports **added** — deliberate, since a removal states the component is gone. All three paths are pinned in `a_mid_tick_replacement_reports_changed_but_a_removal_then_add_reports_added` and spelled out on `take_component` and `query_added`, because reading any one path alone invites the same wrong conclusion again. |
| **2026-08-07 analysis §10** | ✅ **CLOSED 2026-08-10 (v0.151.1). The whole program is finished.** Step 0 of the plan never ran, and nothing recorded that until 2026-08-08; seven shipped across v0.150.1–v0.150.4, two docs/test hygiene items closed on 2026-08-09, the unmeasured v0.150.0 fixes were all measured by v0.150.7, and the last item — `src/app/render/debug_draw.rs:34` — closed in v0.151.1. Nothing from this analysis is open. **Do not reopen it as a source of work**; a new pass would be a new analysis. What it leaves behind is `tests/per_frame_alloc.rs` and the two habits in `docs/PATTERNS.md` / `docs/VERIFICATION.md`. |

### The 2026-08-07 analysis's unverified candidates — the closed record

> **Nothing below is open** as of 2026-08-10 (v0.151.1). It is kept as the record of *how* the
> program closed, because three of its readings were reversed by measurement and one by re-deriving
> a problem the row had written off. Read it for the habits, not for work.

`plans/2026-08-07-analysis-followup.md` had **fourteen** steps, 0 through 13. Steps 1–13 all
shipped (#438–#450, v0.145.1 → v0.150.0). **Step 0 — re-running the 33 verification agents that
died on a session limit, so §10's candidates would get the adversarial pass §1–§8 got — did not
run**, and it left no trace anywhere: `docs/CODE_ANALYSIS_2026-08-07.md` §10 said only "worth
verifying in a follow-up session", which is not a backlog. That is the burial this file exists to
prevent, so it is written down here instead. ⚠️ **Both files named in this paragraph were deleted
2026-09-04** with the rest of the pre-reset archive (`git show <sha>^:<path>` recovers them), which
makes the paragraphs below the only surviving record — the point of having written them here.

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
  corrected the false *claim* in three docs, and the *decision* it exposed **closed on 2026-08-08**
  and was made again on 2026-08-21 when phase 5b restored the job: `Browser smokes (Chrome +
  swiftshader)` is the eighth required context. ⚠️ This line said "still open" until 2026-08-26,
  including through the sweep that was editing this very file — see *Open — process* above, which
  has said "closed" the whole time. Re-read the list rather than either paragraph:
  `gh api repos/ChunSam/skeleton-engine/branches/main/protection --jq '.required_status_checks.contexts'`
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
| `src/ui/panel.rs` `LayoutSystem` | ~~**False positive** — measures 0 over 50 panels × 8 children.~~ ⚠️ **REOPENED 2026-09-06 — the measurement that disagrees is in.** The 0 was an early return: `layout_system_steady_state_does_not_allocate` never inserts `ViewportSize`, so `LayoutSystem::run` returned at `panel.rs:140-143` before touching a panel (`World::new` starts with an empty resource map). Inserting it into that test's own fixture measures **56 allocations / 9,616 bytes** per steady-state frame. The row is now in § *Open — the 2026-09-06 `src/ui` review*. ⚠️ **The lesson is the shape, not the number**: this row's "do not reopen without a measurement that disagrees" was doing its job — what failed is that nobody asked whether the *instrument* had run at all. A green alloc test on a system with an early return is indistinguishable from a green one on a system that allocates nothing. → **CLOSED for real as v0.157.0**: the system now has scratch fields and measures 0 on a fixture that runs it, and the test carries a positive control (a cold frame that must cost something, a child that must have moved) so this failure mode cannot come back silently. |
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
`scripts/wasm_failpaths_web_smoke.sh` reads the verdict; it gates in the `wasm-smokes` job. It is
sabotage-verified in both directions, each half reddening only for its own defect.

⚠️ **The standing lesson, which outlived the two items:** every other browser smoke passes when
nothing goes wrong, so a *failure* handler can be entirely broken with every check still green.
Two shipped that way. When adding a check, ask what it does when the thing it guards is removed —
and if a new failure path gets a handler, it belongs in `wasm_failpaths`, not in a new smoke.

## Open — process

Nothing open. **The required-check question closed on 2026-08-08** and was answered a second
time on 2026-08-21: `Browser smokes (Chrome + swiftshader)` is the **eighth** required context, so
the only automated check that exercises the wasm WebSocket path can actually block a merge. Both
times, verified against the branch-protection API before and after: the other settings (`strict`,
`enforce_admins`, force-push and deletion bans) are byte-identical, only the context list changed.
Re-read the real list rather than trusting this paragraph:
`gh api repos/ChunSam/skeleton-engine/branches/main/protection --jq '.required_status_checks.contexts'`

✅ **The failure-path gap this paragraph opened is closed.** It once added "and, since v0.150.3,
the only one that asserts a *failure* path", which v0.153.0 falsified by deleting `wasm_failpaths`;
phase 5b restored two *success*-path smokes (Web Audio, the WebSocket handshake) and left the gap
standing. That gap was how v0.150.1's broken 404 → `asset_failures()` and v0.150.2's
send-before-open both shipped green. **`wasm_failpaths_web_smoke.sh` was rebuilt on 2026-08-21** and
takes both paths on purpose — the sabotage table at the top of this file reinstated both defects and
watched each redden its own half.

⚠️ **No longer the critical path — re-measured 2026-08-21.** On the first `main` run of the
rebuilt job: **3 m 15 s** for the browser smokes against **3 m 45 s** for `Test (native)`. The old
figures (5 m 33 s / 5 m 37 s against 4 m 3 s / 3 m 53 s) were the *pre-deletion* job, which ran
seven smokes; the rebuilt one runs two. Reverting is still one command (`-X DELETE` on the same
endpoint) if the cost stops being worth it — but the cost is currently not what decides it.

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
  exactly that. ⚠️ **Most of what this bullet inventoried is gone as of 2026-09-04** — the prune
  deleted every dated `CODE_ANALYSIS_*`, `ENTITY_GENERATION_V2_PLAN.md`, `CODE_ANALYSIS.md` and
  `docs/HANDOFF.md`. **The rule outlives its inventory**, because the hazard was never really those
  docs: `## 4.3.0` and `## 10.7.0` are still real entries in `docs/CHANGELOG.md`, sitting *below*
  `## 0.11.0` in the same file. Of the pre-reset docs still tracked, `ROADMAP.md` opens *historical
  roadmap*, `REMOTE_ENTITIES_DESIGN.md` *minimal helper shipped … deliberately deferred*, and
  `SKELETAL.md` is a feature reference with no status to go stale — all three self-marked, so
  **do not go marker-ify anything.**

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

- ✅ **The four Korean HTML docs will not be rewritten — decided 2026-09-05, and the promise is
  deleted rather than carried.** `REFERENCE.html`, `ARCHITECTURE.html`, `STRUCTURE.html` and
  `DEPENDENCY_GRAPH.html` went on 2026-08-20 (#488) for describing an examples tree that no longer
  existed — 145 cargo targets against the **8** this tree has now, which is why a rewrite was never
  a docs chore but a fresh pass over `src/lib.rs` and `docs/MODULE_MAP.md`. This bullet was the row
  behind a commitment that had none, and it named its own cheaper resolution; the maintainer took
  it. `README.md`, `FORKING.md` and `CLAUDE.md` now say *will not be rewritten* and point at the two
  live English substitutes — inverted rather than stripped, so the next reader gets the answer
  instead of re-opening the question. `git show 52f6307^:REFERENCE.html` still has the deleted files
  if a structure is ever worth reusing; the *content* is the part that expired. ⚠️
  `docs/VERIFICATION.md`'s concept-grep lesson quotes what those files contained on 2026-08-20 and
  is a dated record — it stays as written.

- **The rest of the `.claude/` inventory** (gitignored, so these lines are the only tracked record
  that any of it exists; rolled off *Recently closed* on 2026-08-05 but kept here for that reason).
  Two more hooks in `.claude/settings.local.json`, both proven to fire by sabotage and checked
  against real commands for false positives: **`git commit` is denied while any `*.sh` in the index
  is not `100755`** (the trap `core.fileMode = false` hides — fixed repo-wide in v0.135.2,
  reintroduced twice in v0.143.4, re-fixed in v0.143.14), and **`main`-push blocking** with
  `--delete` exempt so remote-branch cleanup still works (a branch named `maintenance-branch` does
  not trip the matcher). Skills: **all nine** carry their detail in `references/` rather than the
  body — seven in this repo's `.claude/skills/`, two in `~/.claude/skills/` (`handoff`, `wrap`).
  ⚠️ This line enumerated three of them until 2026-09-05; the other six had each grown one and
  nothing said so, which is the same decay the sizes warning below is about. Count, do not quote:
  `ls -d .claude/skills/*/references ~/.claude/skills/*/references | wc -l`.
  **Do not record their sizes here** — that number was wrong
  twice in a row in opposite directions (`wc -c` bytes against a character guideline, then a
  correct `wc -m` that was already stale at merge). The durable form is the command:
  `for f in ~/.claude/skills/*/SKILL.md; do wc -m "$f"; done`.

- ~~**Eight directory-based examples silently drop out of `cargo package`.**~~ **Moot 2026-08-19** —
  the examples are deleted and the `examples/**` entries are out of `include`. The lesson survives
  the subject: `include` globs are per-level, so `examples/*.rs` never matched `examples/*/*.rs`,
  and a skipped package target is a *warning*, so CI stayed green over it for months. If a rebuilt
  example is ever meant to ship in the package, glob it explicitly and check `cargo package
  --locked --list` actually contains it.

## Known-unfalsifiable checks — do not mistake these for guarantees

- ~~**`BEAT_CRAWLER_SELFTEST` exit `8`**~~ **deleted 2026-08-19 with the example.** Kept as a
  worked example of the failure mode this section is for: the check ("the two meters are not
  independent") **could not fail on native**, because each meter taps its own channel and the
  spectrum read never sees the mixer output — verified by firing the bass-heavy soundtrack as the
  impact clip and measuring no change at all. It was a tripwire for the **wasm** topology, where
  several sources share one `AnalyserNode`. Ask of every check you rebuild: *on the platform I am
  actually running it, can this assertion fail at all?*

## Standing risks

Context for judging new work — not to-dos. Anything here that becomes actionable belongs in
**Open — engineering** instead; that is where `<NAME>_SELFTEST` coverage went on 2026-08-03.

- **Only the BROWSER half of audio is under CI — restored 2026-08-21, and the native half never
  will be.** v0.143.10 established that **native** (rodio/ALSA) audio stays outside CI; that part is
  unchanged and is what the rest of this bullet is about. v0.143.17 had put **Web Audio** under gate
  (`wasm_audio` 38/38, `audio_reactive` `rms=0.643` with bands `low=9.41` / `high=0.00` on a 110 Hz
  tone), those smokes died with the examples tree on 2026-08-19, and phase 5b put the claim back
  with `survivor_audio_web_smoke.sh` — rms 0.5621, low 2.733 vs high 0.009 on the same 110 Hz tone.
  It passes in CI because Chrome renders the graph in software with no hardware device, which is
  exactly what the native half cannot do. **The distinction is the point** ("audio cannot be tested
  in CI" is false about the *browser* half and true about the native half). Five CI runs
  tried a PulseAudio null sink (default and at 30 ms latency) and ALSA `snd-dummy`; the full table is
  in `docs/VERIFICATION.md`. Summary: a null sink *does* let rodio open a device and `beat_crawler`'s
  audio chain passes on CI, but it delivers samples in bursts, so the meters with sub-second
  deadlines read silence. `snd-dummy` does not exist on the runner kernel. **Do not re-litigate
  without new information** — a runner image with a real or dummy ALSA card would be new
  information; another sink tweak is not. `SKELETON_REQUIRE_AUDIO=1` exists so a *local* run can
  prove its audio checks ran rather than skipped. **Dead from v0.153.0 until phase 0 restored it on
  2026-08-19** — only `scripts/selftests.sh` has ever read it, so it lives and dies with that file. ⚠️ **`scripts/selftests.sh`'s own header claimed
  the opposite** ("CI provisions a PulseAudio null sink") from v0.143.10 until #426 on 2026-08-05 —
  the sentence was written for the null-sink experiment and survived its revert *in the same
  commit*. `ci.yml` and `docs/VERIFICATION.md` were right the whole time; only the file a reader
  actually opens was wrong. **When an experiment is reverted, grep for prose that described it** —
  the revert diff will not show you the comment three files away.
- **All 3 `scripts/*_web_smoke.sh` run in CI, and the local-only tier is empty.** The old tree had
  16 smokes of which 5 (`centered_text`, `embedded_atlas`, `embedded_image`, `game_feel_web`,
  `hdr_web`) stayed local because they asserted byte sizes only — a green run proved nothing. All 16
  died on 2026-08-19; each of the three rebuilt ones self-verdicts, so the tier that existed to hold
  eyeball-it checks has nothing left in it. **Keep it empty** — a smoke that cannot fail on its own
  stated cause belongs nowhere, not in a tier that excuses it.
  ✅ **"run in CI" *is* "gate"**: `Browser smokes (Chrome + swiftshader)` became a required context
  on 2026-08-08, went with the job on 2026-08-19, and was re-added with it on 2026-08-21 (see
  *Open — process*). Count before quoting a number here; this line has now been wrong three times:
  `grep -cE '^\s*[^#]*scripts/[a-z_]*_smoke\.sh' .github/workflows/ci.yml`
- **A headless capture cannot photograph a meter** — fixed dt, no wall clock. Three sessions have
  now reached for `ENGINE_CAPTURE` before remembering this.
- **The nightly `Soak` is a detector on its own clock, not a gate — and must never become one.**
  `.github/workflows/soak.yml` (#528) runs `scripts/soak.sh` over every selftest on the runner,
  where timing differs from a laptop. Its job `Selftest soak` is deliberately absent from branch
  protection: a `schedule`-only context can never report on a PR, so requiring it would block every
  merge forever — the v0.153.0 failure reached from the other direction. Re-verified 2026-09-02
  against the API and `origin/main`'s `ci.yml`: eight contexts, eight jobs, both set differences
  empty, `Selftest soak` absent; `strict` and `enforce_admins` true, force-pushes and deletions
  barred. ⚠️ GitHub silently disables a scheduled workflow after 60 days without repository
  activity — if the nightly goes quiet, check that before believing the tests got better;
  `workflow_dispatch` always works.

---

## Recently closed

> **Roll-off rule:** an entry stays here for **one session**, then goes. Its durable home is
> `docs/CHANGELOG.md` (what shipped) or `docs/PATTERNS.md` / `docs/VERIFICATION.md` (what was
> learned). Without this rule the section regrows the history that was just split out.
>
> ⚠️ **Open the home and read it before rolling an entry off — citing it from memory is how this
> rule nearly ate its own lessons.** The 2026-08-18 `src/ecs` review came one session from going
> out with two of its three lessons homed nowhere; both were written into `docs/VERIFICATION.md`
> first (Trap 8, and the addendum under *A required check is only real once its job is on `main`*).
> Rolling off on schedule is the default, not an obligation: an entry whose lesson lives nowhere
> else **stays until it does**.

### Kept — decisions and pointers this file is the home for, not entries awaiting roll-off

**The pre-reset archive is gone (2026-09-04), and archive pruning is closed.** 242 files /
4.88 MB — `plans/handoffs/` (207), the dead top-level `plans/` feature plans (23) and 12 pre-reset
`docs/` snapshots including the frozen `docs/HANDOFF.md` — plus the four untracked local docs
(`REMAINING_WORK.md`, `PARALLEL_TASKS.md`, `rust_game_engine_plan.md`,
`ENGINE_REVIEW_FIX_PROMPT.md`), held back at first and deleted hours later once an audit found
**zero citations** and three of them naming the just-deleted `docs/HANDOFF.md` as "current status" —
noise converted into confident wrong answers, which is the failure the prune existed to remove.
The judgement it settles: **they were costing search, not context** — hits across 12 working terms
fell 6418 → 1139 (**−82%**), with `netplay` 41 → 41 as the control proving the drop was the archive
and not the measurement. **20 tracked `*.md` remain and `docs/` is 15 files, all live**
(re-counted 2026-09-05). ⚠️ **Do not reopen archive pruning as a noise fix** — the remaining ~18%
is `docs/CHANGELOG.md` and this file, both live. Recover a tracked one with
`git log --diff-filter=D --name-only -- <path>` then `git show <commit>^:<path>`.

⚠️ **The four ignored docs are now unrecoverable, and the backup pointer that said otherwise is
dead.** #560 wrote them to `~/skeleton-engine-local-docs-backup-2026-09-04.tar.gz` (16 KB, 4 files)
before deleting; **checked 2026-09-05 — the path, the whole of `~`, and `~/.Trash` are all empty.**
No harm done, since the audit that authorised the delete found zero citations. The transferable
half is that **an out-of-repo backup is not a home**: it decays with nothing in the tree recording
that it did, exactly like the ignored files it was standing in for. Do not carry an untracked
recovery route forward as if it were one — re-check it, or drop the claim.

⚠️ **The lesson with no other home: prose about noise is noise.** Measuring the second delete read
1749 → 1733 — the four docs were −19 and **+3 came back from the paragraph reporting the cut**,
which named `verify.sh`, `clippy` and `netplay` in order to report it. Worth the three there; the
next person quoting any of these numbers should **re-measure rather than subtract**.

**One handoff path, and it is not tracked (decided 2026-09-04).** `plans/handoffs/` is retired; the
sole destination is the gitignored 15-line `.claude/handoff.md` (the `handoff` skill, whose
`references/modes.md` also covers recovering from a session that died before writing one). The
tracked statement of this is `CLAUDE.md`'s orientation table. **The tracked path is what inflated
it** — a committed file needs a PR and a PR needs justifying, which is how handoffs reached 61 KB
each, against a skill format that caps at 15 lines and forbids work logs. What routes *this* file:
**if something in a handoff looks like it should survive, that is the signal it belongs here or in
`PATTERNS`, not in the handoff.**

**Two procedures live only in gitignored skill files**, so a tracked one-line pointer stays here or
nothing records that they exist — the standing rule for anything living in `.claude/`:

- `.claude/skills/prose-sweep/references/gotchas.md` § 0 — **the pre-deletion audit.** A prose sweep
  is the second half; what paid on 2026-09-04 was counting the tree and auditing citations across
  live docs *and* code **before** deleting anything.
- `.claude/skills/session-cleanup/references/targets.md` — **the in-repo leftover check**: per
  directory, compare `git ls-files | wc -l` against `find -type f | wc -l`; **tracked=0 with
  files>0 is a leftover.** It found **181 MB** of orphaned wasm-bindgen output under `examples/`,
  for examples whose sources went in the 2026-08-19 deletion — `.gitignore`'s `pkg/` had hidden it
  from every `git status` for three months, and it could not even be rebuilt. Worth re-running
  after any tree deletion. ⚠️ `docs/VERIFICATION.md` was checked on 2026-09-05 and carries **no**
  version of this check, which is why the pointer is not optional.

**Rolled off 2026-09-05: the 2026-09-04 batch (#559–#564).** Every home was opened and read rather
than trusted — `CLAUDE.md`'s orientation table (the handoff destination and the
`git show <sha>^:<path>` recovery), both skill references above (confirmed to carry the procedure
verbatim, at `gotchas.md:7` and `targets.md:44`), and `docs/VERIFICATION.md`, which turned out to
carry neither procedure. The prune and handoff decisions shortened rather than left, as the entry
they replace predicted; reading the homes also killed a dead backup pointer, above.
**Nothing transient is left in this section.**
