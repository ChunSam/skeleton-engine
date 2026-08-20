# Verification — the gate, its traps, and what it does NOT cover

The command list itself lives in `CLAUDE.md` (§ Verification), because it is run every
session. This file holds the *why* behind it: the traps that have actually bitten, and the
cases where a green gate is not enough. Read it once; re-read it when a gate result
surprises you.

> ⚠️ **The examples tree was deleted on 2026-08-19, and with it more than half of what this file
> describes.** The `<NAME>_SELFTEST` acceptance runner, the wasm-example build step, and all
> sixteen `*_smoke.sh` scripts are gone; `verify.sh` is now fmt / clippy / wasm build / test /
> doctest / doc. Sections about that machinery are marked **[gone]** and kept deliberately — the traps in
> them are what rebuilding an acceptance layer runs into, and every one was paid for once already.
> The traps under *Reading a gate's result* and *Searching so the result means something* are
> unaffected and apply today.

---

## Reading a gate's result

`./scripts/verify.sh` runs all six checks in order. The **only** authoritative verdict is
its exit code, read from a command that is not piped:

```bash
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"
```

### Trap 1 — a trailing pipe reports the pipe's status, not the gate's

`./scripts/verify.sh | tail` reports `tail`'s `0` and **hides** a real `fmt --check` /
`clippy` failure. This has bitten more than once.

### Trap 2 — zsh's pipe-status array is `$pipestatus`, and it is 1-indexed

The shell here is **zsh**. Bash-style `${PIPESTATUS[0]}` is always the empty string — a
whole session of `echo "X_EXIT=${PIPESTATUS[0]}"` silently printed `X_EXIT=`. If you must
index a pipe, use `${pipestatus[1]}`. Better: do not pipe a gate at all.

### Trap 3 — `;` does not stop on failure

`./scripts/verify.sh > log 2>&1; echo $?; git commit …` in one call **commits even when the
gate is red**: `;` does not short-circuit, and the printed exit code is only read
afterwards. Capture it and *branch* on it (`[ $VERIFY_EXIT -ne 0 ] && exit 1`), or run
verify as its own call and read the result before committing.

### Trap 4 — a background task's completion notification reports the wrong command

A `run_in_background` call of `./scripts/verify.sh > log 2>&1; echo $? > exit` reports the
trailing `echo`'s status (always 0) in its completion summary. **Read the `.exit` file**,
never the notification's "exit code 0".

This is the most-repeated trap in the project's history: it fired **twice on 2026-07-29**
(the notification said `0` while the file held `1`, then `101`) and **again on 2026-07-30**
(said `0`, file held `1` — a `cargo fmt` reflow). Writing it down has not been enough, so
here is the whole procedure as one copy-paste block. It closes Traps 4 **and** 5 together:

```bash
# 1. remove first, or an `until [ -f … ]` waiter matches a file from days ago (Trap 5).
#    Run this as its OWN call — fusing it into step 2 puts the `rm` inside the background job.
rm -f /tmp/v.exit /tmp/v.log
# 2. run non-piped; write the gate's OWN status to the file
(./scripts/verify.sh > /tmp/v.log 2>&1; echo $? > /tmp/v.exit) &
# 3. wait for the file, then read it — NOT the completion notification (Trap 4)
until [ -f /tmp/v.exit ]; do sleep 20; done
echo "EXIT=$(cat /tmp/v.exit)"; ls -l /tmp/v.exit; date '+now %H:%M:%S'   # mtime must be fresh
# 4. corroborate: the counts should match the tree you expect
echo "ok groups: $(grep -c 'test result: ok' /tmp/v.log)"
grep -E 'running [0-9]+ tests' /tmp/v.log | head -1
```

**Corroborate, don't just trust the number.** A green run should report roughly the expected
`ok`-group and lib-test counts for the tree, and a count that moved when your change should not
have moved it is worth a look even when the exit code is `0`. ⚠️ The old reference figures
(152 groups / 1339 lib tests at v0.138.0) are void — most of those groups were example targets,
deleted on 2026-08-19. The post-deletion baseline, measured on a green run at v0.153.3, is
**13 `ok` groups / 1461 lib tests**. The trail, since two releases moved it without saying so:
1443 at v0.153.0 → **1449 at v0.153.1** (#482 added 6 and did not update this line) → 1461 at
v0.153.2 (+12). Groups 12 and 13 are `examples/platformer_game` and `examples/rpg_quest_game`: an
example target adds one `ok` group even when it contributes no `#[test]` — its selftest is an
env-var entry point, so the group reads `running 0 tests`. Every rebuilt game adds one more; the
lib count does not move with them.

### Trap 5 — a stale `.exit` file from a previous session

An `until [ -f /tmp/verify.exit ]; do …; done` waiter matches a **leftover file from days
ago** and returns that old code instantly — the gate looks like it passed while `cargo
test` is still running. `rm -f` the file before spawning, or wait on the PID
(`while kill -0 <pid>`), and check the file's mtime alongside its contents.

**Steps 1 and 2 of the block above must be two separate calls.** Fusing them into one
backgrounded command — `rm -f /tmp/v.exit; (./scripts/verify.sh …; echo $? > /tmp/v.exit) &`
— moves the `rm` *inside* the background job, where it can still be pending when the `until`
waiter runs. The waiter then matches the previous run's file and the Trap 5 defence is as
absent as if it had never been written. This happened on **2026-08-03**: a red gate (clippy
`needless_range_loop`, already sitting in the local log) was reported green, and CI caught it
instead. Step 3's mtime check is the backstop — it was also skipped that run.

### Trap 6 — `core.fileMode = false` makes `chmod +x` invisible to git

A script you `chmod +x` runs for you and ships **644 to everyone else**; `ls -l` shows the
working tree, which lies here. Use `git update-index --chmod=+x <files>` and verify with
`git ls-files -s '*.sh'`. (This is how v0.135.1 shipped a smoke script and its `build.sh`
without the bit — fixed in v0.135.2.)

### Trap 7 — a squash-merged branch reads as "ahead", so the branch graph cannot tell you it is safe to delete

A squash-merge writes a *new* commit and leaves the original tip dangling, so
`git branch --contains` / "ahead by N" report an already-landed branch as unmerged. Both branches
deleted on 2026-08-03 that looked ahead by 1 were in fact fully contained in `main`. **Verify by
content** — diff the branch's tree against `main`, or confirm the PR is merged — never by the graph.
(An agent's `git push --delete` is refused by the remote-destructive permission gate, which is
correct; a human runs that step.)

---

## Searching so the result means something

Traps 1–3 are one shape — a pipe that discards what you meant to read. That shape is not confined
to exit codes, and both of these have cost a session.

### A truncated search cannot prove absence

`rg 'anonymous' src/ | head` reported ten unrelated hits and stopped. On 2026-08-18 a code review
read that as "this fallback does not exist anywhere" and filed a finding against a rustdoc comment
that was **correct** — the one line that disproved it — `tr("anonymous", "익명")` in
`src/app/editor/ui/mod.rs` — sat below the cut. The finding survived into a written report and
was withdrawn only because the claim was re-derived before acting on it.

`head`/`tail` answer "show me some matches". They never answer "are there none". For absence, use a
form whose whole output fits or whose result is a single value:

```bash
rg -c 'anonymous' src/          # per-file counts — nothing to truncate
rg -q 'anonymous' src/; echo $? # 0 = found, 1 = not found
```

Same rule as Trap 1, one layer up: **if the thing you are about to conclude is "there is no X",
nothing in the pipeline may be allowed to drop lines.**

### Grep for the concept, not for the file you were editing

A commit that removes a concept leaves prose behind a few lines away from where it looked. `92e05fe`
dropped rust-survivors as a consumer and cleaned the dangling pointer at `docs/HANDOFF.md:121` —
while line 23's "uses this engine as a dependency" sat eight lines *under* its own contradiction at
line 15, and survived until #464. The reverted null-sink experiment did the same thing: the code
went, its comment stayed in `scripts/selftests.sh`.

v0.153.0 did it again, in this very file. Deleting the examples tree swept every reference to
`scripts/selftests.sh` — but **`SKELETON_REQUIRE_AUDIO`, the flag that script alone read**, survived
in two live paragraphs describing it as an available tool. A file-name grep cannot find the names a
deleted file *owned*. Enumerate those separately: env vars, flags, and coined terms. (Restoring the
script on 2026-08-19 made those paragraphs true again, which does not retire the lesson — it ran
the other way that day, and every paragraph asserting the flag was *dead* had to be found the same
way, by its name rather than its file.)

⚠️ **The concept grep has a file-type blind spot, and it is easy to walk into twice in one day.**
2026-08-20: phase 2 of the examples rebuild swept "the `examples/` tree is empty" out of
`docs/NEXT_WORK.md`, `docs/VISION.md`, `README.md` and `FORKING.md` — a correct concept grep, run as
`grep -rn '…' docs/*.md README.md FORKING.md`. It found four of eight. The other four are
**`REFERENCE.html`, `STRUCTURE.html`, `ARCHITECTURE.html` and `DEPENDENCY_GRAPH.html`**, which
`CLAUDE.md`'s own orientation table lists as the user-facing docs. `STRUCTURE.html` still described
145 cargo targets and 43,990 lines of examples, and pointed at `scripts/build_wasm_examples.sh`,
deleted in v0.153.0; `REFERENCE.html` still linked 15 example paths that no longer exist.

The rule: **name the concept, then drop every path and extension filter** — `grep -rn '<concept>' .`
first, narrow after. A repo whose docs are half Markdown and half generated HTML will answer
`--include='*.md'` truthfully and incompletely, and an incomplete answer to "did I get them all?"
reads exactly like a complete one.

Verifying a removal means searching the **repo** for the idea in every phrasing it might wear, not
re-reading the file you happened to have open. The file you were editing is the one place you have
already looked.

### Never cite `docs/CHANGELOG.md` by line number — it decays with nobody touching it

Two rules above already govern numbers in prose: *a number pinned in prose is fixed by whoever
changes it*, and *a citation that cannot settle the claim is a defect*. Both assume **someone
changed something**. A line number into `docs/CHANGELOG.md` needs no such help: every release
prepends, so every line below the new entry moves, and a citation that was exact when written is
wrong by the next release without a single edit to the text it points at.

On 2026-08-20 a peer session cited `docs/CHANGELOG.md:1090` for the warm-up-window precedent. It was
exact when read. By the time it was checked, v0.153.2 (+96 CHANGELOG lines) and v0.153.3 (+31) had
landed, and the paragraph had moved to **1217** — the drift is 96 + 31 = 127, to the line. Line 1090
by then held an unrelated entry about `core.fileMode`, which is the dangerous part: the citation did
not dangle, it silently **re-pointed at other content**. Nothing was committed carrying the stale
number, so this cost a message rather than a session — the next one may not be so cheap.

Cite by heading or by a quoted phrase, which `grep` still finds at any depth:

```bash
# ✅ survives any number of releases
# docs/CHANGELOG.md § "A warm-up is part of the property, not setup noise"
grep -n 'A warm-up is part of the property' docs/CHANGELOG.md
```

This is specific to append-at-the-top files — `docs/CHANGELOG.md` is the one in this repo. A line
number into a source file is stable until someone edits that file, and the rules above cover that
case.

---

## What each step does and does not cover

### The WASM step is lib+bins — examples were a separate, derived step **[gone]**

`cargo build --target wasm32-unknown-unknown` (lib+bins) is the library wasm gate, and with no
examples in the tree it is now the whole of it.

Do **not** gate on `--target wasm32 --all-targets` when examples come back. It fails on the
native-only ones, correctly so: physics / sockets / GPU particles pull `rapier2d`, `tungstenite`
and `GpuParticleEmitter`, and others call native-only *engine* APIs (headless screenshots,
hot-reload). `scripts/build_wasm_examples.sh` existed for that — it built the examples declaring a
`#[wasm_bindgen]` entry point, the set **derived from that entry point, not hardcoded**, so a new
web example was picked up without anyone remembering to register it.

**The consequence that closed:** an example could be broken for wasm indefinitely and the gate
stayed green. `embedded_image` was unbuildable for `wasm32` from the day it was added (it called
the native-only `save_screenshot_headless` unconditionally) until v0.135.1 — and nothing *ran* it
on the web until v0.143.4 gave it a browser harness and a render smoke. That hole is open again.

### A skip is not a pass — `scripts/selftests.sh` **[rebuilt 2026-08-19]**

The `<NAME>_SELFTEST` acceptance tests were the only defense against a headline feature degrading
gracefully into silence. Each was proven non-vacuous by sabotage when written — and until v0.143.8
**nothing ran them again**: neither CI nor `verify.sh` contained the string `SELFTEST`. From
v0.143.8 they ran in both, via `scripts/selftests.sh`. All of it went with the examples on
2026-08-19. The **runner** returned the same day as phase 0 of the rebuild, carrying every rule
below, and phase 1's `PLATFORMER_SELFTEST` (7 checks) is the first rebuilt test it gates. **Read
this section before writing the next one**, because the runner's shape was not incidental:

The reason it was a script rather than a list of `cargo run` lines is that **every one of these
tests opts out with exit 0** when its environment cannot support a check, so the exit code alone
cannot distinguish "passed" from "ran nothing":

- `SKIP: no audio device` is tolerated **by default**, so a box without a sound card still passes.
  **`SKELETON_REQUIRE_AUDIO=1` makes it fatal.** CI does *not* set it — see below, CI has no usable
  sound card — but run it locally when you want proof the audio checks actually executed rather than
  opted out: `SKELETON_REQUIRE_AUDIO=1 ./scripts/selftests.sh`.
- **Every other skip is a failure.** The networked tests skip their live checks when the sibling
  server binary is absent, and `cargo run --example salvage_run` builds only `salvage_run` — so a
  naive CI step would drop exactly the checks that cover the most, and report success. This was
  measured, not assumed: with `predict_shooter_server` hidden, the raw exit code is **0**.

So the script builds the selftest targets and their sibling servers — the old one built
`--examples`, which was the native job's largest single cost — then greps each run's output and
fails on any non-audio skip. Same principle as the render job's `SKELETON_REQUIRE_GPU=1`: an
environment opt-out must never read as a green pass. The rebuild adds two rules the old runner had
no way to enforce, because it arrived after its games rather than before them: **every
`examples/<name>/<name>.rs` must carry a selftest** (the exemption that left 11 of 22 unverified),
and **a run that exits 0 having printed no `ok:` or `SKIP:` verdict fails as vacuous**.

**The subtler version, one level in: a skip condition the failure itself can forge.** The runner
above polices skips it can *see*. It cannot police a check that decides to skip for the wrong
reason. `SETTINGS_MENU_SELFTEST` (v0.145.0) asserts that the `Audio` handle survives a scene reset,
and its first draft read `world.resource::<Audio>()` after the transition: `None` there means both
"this machine has no sound card" and "the reset dropped the handle" — **the failure and the skip are
the same observable**. Sabotaging the engine's own `register_persistent::<Audio>` call made the
check print `SKIP` and report success, which is precisely the outcome the whole runner exists to
prevent, reached from inside the test.

The rule: **a skip condition must be sampled from a source the failure cannot produce.** Here that
meant probing the device with a throwaway `Audio::new()` *before the app is built*, so "no device"
is answered by the hardware rather than by the machinery under test. Ask it of any check that can
opt out: *if the thing I am testing were completely broken, could that alone make this check skip?*

### Sabotage each half separately, and believe the half that stays green

"Sabotage-verify every check" is the existing rule; phase 1's platformer selftest (2026-08-19) found
the version of it that actually bites. **A two-sided check can have a side that no sabotage you tried
has ever moved — and it reads exactly like a side that works.** Two cases from writing seven checks:

- The one-way plank check asserts a drop *down* through it and a jump *up* through it. Every game-side
  sabotage (plank not tagged one-way, `request_drop` never issued) turned the whole check red — but
  always via the **down** half, so the up half had still never been seen to fire. Breaking the engine's
  `!moving_down` clause, the obvious suspect, **also did not move it**: at this geometry the predicate's
  position test alone (`char_bottom <= platform_top + tolerance`) already passes a character whose feet
  are below the plank. Only making a one-way collider block outright, except during a drop, turned the
  up half red on its own. That negative result is worth as much as the check: it says which engine
  branch the check does *not* cover.
- The blend-tree check first asked for **≥ 2 distinct clips and one crossfade**, and a blend tree pinned
  to a constant parameter passed it — the tree still switches **once**, off the `AnimationPlayer`'s
  starting clip, and that single switch supplies both the second clip and the crossfade. Counting
  *switches* (≥ 4 over a full parameter sweep) is what the pinned sabotage cannot fake. The general
  form: **"visited more than one value" is satisfied by any one-time initialization**; a check that a
  thing keeps varying has to count the variations.

The procedure this implies: for every clause of a check, name a sabotage that flips **that clause and
not the others**, and if you cannot make one fire, say so in the comment rather than assuming it holds.

### Writing one: assert an invariant, not an end state

Two of these tests have now been bitten by the same shape, so it is a rule rather than an anecdote.
When anything in the background can add to what you are counting, an end-state assertion is not a
weaker check — it is a **differently-wrong** one, and it usually fails in the direction that looks
green.

- `SALVAGE_RUN_SELFTEST` (v0.143.6) asserted the endpoint of an eviction and passed against a
  `STALE_TIMEOUT` sabotaged to 0.05 s: the entity was evicted between snapshots and re-spawned on
  the next one, so only a **per-frame** watch saw the flicker.
- `COIN_RACE_SELFTEST` (v0.143.12) could not count score deltas, because the server respawns a coin
  at a random position after every take and one landing under a player's feet scores a point nobody
  asked for. It asserts *points gained == coins the server took away* instead, which holds however
  many coins get taken. That invariant is also what caught the sharpest sabotage: with the server's
  first-claim-wins guard removed, both clients' scoreboards still **agree** (both faithfully apply
  both `taken` messages) and only the accounting sees 2 points against 1 coin removed.

The related limit: you can only assert what the API exposes. `NetworkClient` has no readable outbox,
so "a message was sent" is unobservable offline — state the consequence you *can* see, and say in the
failure message which property that stands for, rather than re-deriving the code under test.

### Writing one: check where your "before" sample is actually taken

The obvious fix for the skip trap above was to sample the same thing *earlier* — before the
transition, while the app was still known-good. That draft skipped too, and the reason generalises:
**`App::set_scene` is itself a `SceneCmd::Replace`**, so an `App` has already performed one full
world reset by the time its setup function returns. A "before the transition" sample taken after
`build_app()` is still a sample taken **downstream of the mechanism under test**.

Anything that means to observe a pre-reset state has to establish it before the `App` exists, or
establish it from outside the `App` entirely. The general shape: when a check depends on a baseline,
confirm the baseline is upstream of every operation the check is about — an off-by-one-stage
baseline does not fail loudly, it agrees with the broken state and reports success.

Sabotage is what surfaces both of these. Neither draft could have been reasoned wrong from reading
it — each looked like it was testing the right thing, and each was confirmed green before the
sabotage was tried.

### Writing one: a fixture that omits the subject reads clean

`tilemap_system_steady_state_does_not_allocate` (v0.150.4) built a `World`, added a `TilemapSystem`,
measured a steady-state frame and asserted zero. It passed for four releases. It contained **no
`Tilemap`** — so `TilemapSystem::run` collected an empty entity list and returned, never reaching
the grid clone the test was written to guard. The v0.150.5 entry reported v0.150.0's tilemap fix
"confirmed" on the strength of it. Given a populated map, the same system measured 2 allocations per
idle frame (v0.150.7).

This is the same family as the vacuous PNG assertion #456 found, and the tell is the same: **a check
that cannot fail reports exactly what a working system reports.** A green "must be zero" assertion is
two claims glued together — *the code is clean* and *the code ran* — and only the second is cheap to
verify. So verify it: pair every must-be-zero assertion with a **positive control in the same test**
that drives the guarded path and requires a non-zero reading. If the control cannot be made to fail,
the fixture is not measuring what its name says.

The corollary for a claim that is genuinely not reachable as a zero — `DialogueSystem` legitimately
allocates on a frame with a visible dialogue box — is to measure a **difference** instead of an
absolute. Two runs differing only in the size of the thing that must not be cloned settle it, and
the control is the same shape: a third run where that thing *is* used, which must cost more.

**The same trap wearing a different hat: a byte-identical `ENGINE_CAPTURE` diff.** v0.151.1 changed
how `DebugDraw` emits axis-aligned segments and proved it invisible by capturing `centered_text`
before and after — 0 differing bytes out of 2,073,600. That number is worth exactly nothing on its
own, because **a frame that never drew the subject also diffs to zero**, and so does a capture that
silently failed and wrote the same clear colour twice. It is the identical two-claims-glued-together
shape: *the change is invisible* and *the changed thing was on screen*. The control is cheap — the
same pass over the pixels located the three guide columns at x=191/479/767 at luminance 152.7
against a 68.7 background, which is what makes the zero mean anything. **Before reporting a
before/after image as unchanged, assert where the subject is in it.**

### Audio in CI was attempted and does not work — do not re-litigate without new information

Five CI runs went into this in v0.143.10 and the answer was no. Recorded so the next person does not
spend the same day.

| Attempt | Result |
|---|---|
| **PulseAudio null sink, default latency** | The sink comes up and **rodio opens a device** — `beat_crawler`'s whole audio chain passed on CI, finding 16 kicks in a real mix at 0.638 s spacing. But `audio_reactive` read rms **0.0000** against its 1200 ms rise deadline, and `survivor`'s peak reached 1.0000 while its **600 ms watchdog engaged**. |
| **Null sink + `PULSE_LATENCY_MSEC=30`** | **Worse.** `beat_crawler` now finds *no* kick at all and `survivor`'s peak is 0.0000. A small buffer broke the one thing that worked. |
| **ALSA `snd-dummy`** | **Not available.** The runner's azure kernel ships no such module even with `linux-modules-extra` installed: `modprobe: FATAL: Module snd-dummy not found in /lib/modules/6.17.0-1020-azure`. |

The pattern is that a null sink delivers samples in bursts rather than continuously, so the level
tap publishes and then goes stale in the gaps. Checks that sample over *seconds* ride it out; the
sub-second deadlines land in the gaps. Those deadlines are calibrated against real hardware and
loosening them would discard the guarantee they exist to make, so they were left alone.

**So every audio claim still rests on a local device**, and the tool for making that local run
prove itself is back: `SKELETON_REQUIRE_AUDIO=1` turns a tolerated audio skip into a failure, and
the rebuilt `scripts/selftests.sh` reads it again as of 2026-08-19. It was dead between v0.153.0
deleting that script and phase 0 restoring it — the flag has never lived anywhere else, so it dies
with that one file every time. Anything
that would change the CI answer — a runner image with a real or dummy ALSA card, or a different
sink whose delivery is continuous — is new information.

Unrelated but learned the same day: a **corrupt cargo cache** produced
`collect2: fatal error: ld terminated with signal 7 [Bus error]` twice in a row on a runner with
108 GB free. It is not disk. `gh cache delete` for the `Linux-cargo-*` keys cleared it.

**The list must be derived, not hardcoded** — an example was a selftest iff it read a
`<NAME>_SELFTEST` environment variable. The first version of the script hardcoded it, and the very
next selftest to land (`ORBITAL_DODGER_SELFTEST`, v0.143.9) was not in the list: the gate went green
having never run the test that was the entire point of that change. A registry you must remember to
edit is a registry that silently shrinks — the same failure the script exists to prevent, one level
up. `scripts/build_wasm_examples.sh` derived its set for the same reason. **This is the single most
transferable lesson here; build any replacement runner the same way.**

### CI is ubuntu only

`#[cfg(target_os = "macos")]` / `"windows"` code, and OS-only deps like
`objc2-game-controller`, are **never compiled or run on CI** — so green CI alone does not
verify an OS-gated change. The macOS gamepad backend (v0.47.0) was merged on green CI
**plus** a local build **plus** a hardware pad check. Build **both** cfg branches locally
with `-D warnings` (especially `dead_code`); one OS misses the other's lints.

### Compiling for wasm is not running on wasm

A wasm build proves the code type-checks, not that it draws. v0.135.0 claimed
`load_atlas_bytes` works on the web on the strength of a compile — which would not have
caught a texture that decoded and never reached the GPU. For anything making a runtime
claim about the web you need a render smoke — and there is currently no such thing in the tree, so
a runtime web claim cannot be backed at all right now.

### Don't narrow the bar

A prior "done" on only `fmt --check` + `test --lib` shipped the wasm-build and clippy
regressions that the full list catches.

---

## Checks that are NOT part of the gate

### GPU render tests (these DO run on CI)

The `render` job renders `tests/render.rs` headlessly with Mesa **lavapipe** (software
Vulkan) on the GPU-less ubuntu runner, asserting renderer-tolerant invariants
(sprite / text / lighting / letterbox). `SKELETON_REQUIRE_GPU=1` hard-fails when no adapter
is present; otherwise it skips cleanly. See **`docs/RENDER_TESTING.md`**. Its three companion
smokes (`headless_screenshot`, `lighting_cap`, `packaged_assets`) were examples and are gone, so
`tests/render.rs` is now the entirety of the engine's render verification.

### wasm smoke checks **[gone]**

⚠️ **All of these were deleted on 2026-08-19 with the examples they drove, and the `wasm-smokes`
CI job with them. Nothing loads the engine in a browser any more.** The rest of this section is
kept as the specification for rebuilding them — what each asserted, and which ones a green run did
not actually prove. Historical text below; none of these scripts or examples exist.

Each built an example to wasm, served it, and rendered it in headless Chrome. Prerequisites
were `rustup target add wasm32-unknown-unknown`, a matching `wasm-bindgen-cli`, and Chrome.

The `wasm-smokes` job ran the five that report a `*_CHECK: PASS` verdict — `wasm_save`,
`render_format_query`, `bloom_web`, `wasm_audio`, `audio_reactive`. The others assert byte sizes
only, so they stayed local where a human can look at the frame.

| Script | Asserts |
|---|---|
| `scripts/wasm_smoke.sh` | `coin_race` runs and its WebSocket path works |
| `scripts/wasm_save_smoke.sh` | the AEAD `localStorage` round-trip (7/7) |
| `scripts/wasm_audio_smoke.sh` | the `web_audio` surface incl. buses/ducking/positional |
| `scripts/centered_text_smoke.sh` | non-blank render at DPR=2 (EW-001 centering — eyeball it) |
| `scripts/embedded_atlas_smoke.sh`, `embedded_image_smoke.sh` | no image is served beside the page **and** the frame is non-blank |
| `scripts/audio_reactive_smoke.sh` | `Audio::levels` reports a live level **and** `Audio::bands` a low-biased spectrum in a browser (the wasm `AnalyserNode` half shares almost no code with the native tap + FFT) |
| `scripts/game_feel_web_smoke.sh`, `bloom_web_smoke.sh`, `hdr_web_smoke.sh`, `render_format_query_smoke.sh` | their example renders on the web |

A byte-size check alone is weak — it proves *something* drew, not that it drew *correctly*.
Where a stronger structural assertion is available, pair the two: the two byte-source smokes
(`embedded_atlas_smoke.sh`, `embedded_image_smoke.sh`) also assert no image file exists in the
served directory, so a non-blank frame cannot have come from a fetch. **Eyeball the saved
screenshot** for anything positional; no byte count catches a wrong tile or a mis-centered label.

Set the byte threshold by *measuring* the same page with the engine never drawing (load it
without `?autostart=1`), not by copying a number from a sibling script — a threshold below what
the DOM alone paints passes on a frame the engine never touched.

### Which smokes actually prove their claim, and which need your eyes **[gone]**

Nine of the fifteen asserted something specific — a page verdict (`*_CHECK: PASS`), a pixel
ratio, a reported failure. **Six were byte-size-only**, so a green run meant "a frame drew",
not "the right frame drew". For those, `SMOKE_KEEP=1` and *look* — the distinction is the
transferable part, and a rebuilt smoke should land in the first group, not the second:

| Byte-size only — eyeball it | What only the screenshot can tell you |
|---|---|
| `centered_text_smoke.sh` | each label's center actually sits on its guide line (EW-001) |
| `game_feel_web_smoke.sh` | player, three dummies, platform gap and HUD are all present |
| `hdr_web_smoke.sh` | HDR keeps core-vs-mid distinct where LDR collapses them to flat grey |
| `wasm_smoke.sh` | the HUD says "Player #1" — i.e. the WebSocket handshake really happened |
| `embedded_atlas_smoke.sh` | the 12 tiles are the right tiles (its no-image-served check is structural, but grid maths are not covered) |
| `embedded_image_smoke.sh` | the sprite is the *right* image and not the white fallback — the exact failure the verbatim-key invariant exists to prevent, and it still draws a non-blank frame |

Sweeping all of them took about fifteen minutes and was worth doing after any change to
the render path, the asset path, or a `web/build.sh` — they were the only checks that ran
engine code in a browser at all, which is why their deletion leaves the web entirely unverified.

### Anything CI cannot exercise

A windowed playtest, audio playback, hot-reload, or a gamepad. Get the real-behavior
confirmation before merging, not just green checks — see the judgment gates in the
`land-pr` workflow.
