# Verification — the gate, its traps, and what it does NOT cover

The command list itself lives in `CLAUDE.md` (§ Verification), because it is run every
session. This file holds the *why* behind it: the traps that have actually bitten, and the
cases where a green gate is not enough. Read it once; re-read it when a gate result
surprises you.

---

## Reading a gate's result

`./scripts/verify.sh` runs all seven checks in order. The **only** authoritative verdict is
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
`ok`-group and lib-test counts for the tree (152 groups / 1339 lib tests at v0.138.0; adding
an example target adds a group, adding a test adds a test). A count that moved when your
change should not have moved it is worth a look even when the exit code is `0`.

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

## What each step does and does not cover

### The WASM step is lib+bins — examples are a separate, derived step

Do **not** gate on `--target wasm32 --all-targets`: it fails on the native-only examples, and
correctly so. There are more of them than the old note here listed — besides `platformer_game` /
`mp_server` / `gpu_particles` (`rapier2d` / `tungstenite` / `GpuParticleEmitter`), the examples
using native-only *engine* APIs also fail: `headless_screenshot`, `hot_reload_asset_root`,
`tile_anim_stagger`, `slider_keyboard_step`, `ui_stepper`, `ui_tabs`.

`cargo build --target wasm32-unknown-unknown` (lib+bins) is still the library wasm gate.
**Since v0.143.8 the examples have their own step**: `scripts/build_wasm_examples.sh` builds the
16 examples that declare a `#[wasm_bindgen]` entry point — the ones an `index.html` actually calls.
The set is **derived from that entry point, not hardcoded**, so a new web example is picked up
without anyone remembering to register it. It runs in CI's Build (WASM) job and in `verify.sh`.

**The consequence this closed:** an example could be broken for wasm indefinitely and the gate
stayed green. `embedded_image` was unbuildable for `wasm32` from the day it was added (it called
the native-only `save_screenshot_headless` unconditionally) until v0.135.1 — and nothing *ran* it
on the web until v0.143.4 gave it a browser harness and a render smoke.

An example with **no** wasm entry point is still not covered; if you want one built for wasm, it
needs the entry point (which is also what makes it loadable) or an explicit build.

### A skip is not a pass — `scripts/selftests.sh`

The `<NAME>_SELFTEST` acceptance tests are the only defense against a headline feature degrading
gracefully into silence. Each was proven non-vacuous by sabotage when written — and until v0.143.8
**nothing ran them again**: neither CI nor `verify.sh` contained the string `SELFTEST`.

They now run in both, via `scripts/selftests.sh`. The reason that is a script rather than a list of
`cargo run` lines is that **every one of these tests opts out with exit 0** when its environment
cannot support a check, so the exit code alone cannot distinguish "passed" from "ran nothing":

- `SKIP: no audio device` is tolerated **by default**, so a box without a sound card still passes.
  **`SKELETON_REQUIRE_AUDIO=1` makes it fatal.** CI does *not* set it — see below, CI has no usable
  sound card — but run it locally when you want proof the audio checks actually executed rather than
  opted out: `SKELETON_REQUIRE_AUDIO=1 ./scripts/selftests.sh`.
- **Every other skip is a failure.** The networked tests skip their live checks when the sibling
  server binary is absent, and `cargo run --example salvage_run` builds only `salvage_run` — so a
  naive CI step would drop exactly the checks that cover the most, and report success. This was
  measured, not assumed: with `predict_shooter_server` hidden, the raw exit code is **0**.

So the script builds `--examples` first, then greps each run's output and fails on any non-audio
skip. Same principle as the render job's `SKELETON_REQUIRE_GPU=1`: an environment opt-out must
never read as a green pass.

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

**So every audio claim still rests on a local device**, and `SKELETON_REQUIRE_AUDIO=1` is the tool
for making that local run prove itself. Anything that would change this answer — a runner image with
a real or dummy ALSA card, or a different sink whose delivery is continuous — is new information.

Unrelated but learned the same day: a **corrupt cargo cache** produced
`collect2: fatal error: ld terminated with signal 7 [Bus error]` twice in a row on a runner with
108 GB free. It is not disk. `gh cache delete` for the `Linux-cargo-*` keys cleared it.

**The list is derived, not hardcoded** — an example is a selftest iff it reads a `<NAME>_SELFTEST`
environment variable. The first version of the script hardcoded it, and the very next selftest to
land (`ORBITAL_DODGER_SELFTEST`, v0.143.9) was not in the list: the gate went green having never run
the test that was the entire point of that change. A registry you must remember to edit is a
registry that silently shrinks — which is the same failure the script exists to prevent, one level
up. `scripts/build_wasm_examples.sh` derives its set for the same reason.

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
claim about the web, run a render smoke (below).

### Don't narrow the bar

A prior "done" on only `fmt --check` + `test --lib` shipped the wasm-build and clippy
regressions that the full list catches.

---

## Checks that are NOT part of the gate

### GPU render tests (these DO run on CI)

The `render` job renders `tests/render.rs` headlessly with Mesa **lavapipe** (software
Vulkan) on the GPU-less ubuntu runner, asserting renderer-tolerant invariants
(sprite / text / lighting / letterbox). `SKELETON_REQUIRE_GPU=1` hard-fails when no adapter
is present; otherwise it skips cleanly, and it runs under `verify.sh` where a GPU exists.
See **`docs/RENDER_TESTING.md`**.

### wasm smoke checks (5 gate in CI as of v0.143.17; the rest are local)

Each builds an example to wasm, serves it, and renders it in headless Chrome. Prerequisites
are `rustup target add wasm32-unknown-unknown`, a matching `wasm-bindgen-cli`, and Chrome.

The `wasm-smokes` job runs the five that report a `*_CHECK: PASS` verdict — `wasm_save`,
`render_format_query`, `bloom_web`, `wasm_audio`, `audio_reactive`. The others assert byte sizes
only, so they stay local where a human can look at the frame. See `docs/WASM_SMOKES.md`.

| Script | Asserts |
|---|---|
| `scripts/wasm_smoke.sh` | `coin_race` runs and its WebSocket path works |
| `scripts/wasm_save_smoke.sh` | the AEAD `localStorage` round-trip (7/7) |
| `scripts/wasm_audio_smoke.sh` | the `web_audio` surface incl. buses/ducking/positional |
| `scripts/centered_text_smoke.sh` | non-blank render at DPR=2 (EW-001 centering — eyeball it) |
| `scripts/embedded_atlas_smoke.sh`, `embedded_image_smoke.sh` | no image is served beside the page **and** the frame is non-blank |
| `scripts/audio_reactive_smoke.sh` | `Audio::levels` reports a live level **and** `Audio::bands` a low-biased spectrum in a browser (the wasm `AnalyserNode` half shares almost no code with the native tap + FFT) |
| `scripts/game_feel_web_smoke.sh`, `bloom_web_smoke.sh`, `hdr_web_smoke.sh`, `render_format_query_smoke.sh` | their example renders on the web |

See **`docs/WASM_SMOKES.md`** for the full list and how to add one.

A byte-size check alone is weak — it proves *something* drew, not that it drew *correctly*.
Where a stronger structural assertion is available, pair the two: the two byte-source smokes
(`embedded_atlas_smoke.sh`, `embedded_image_smoke.sh`) also assert no image file exists in the
served directory, so a non-blank frame cannot have come from a fetch. **Eyeball the saved
screenshot** for anything positional; no byte count catches a wrong tile or a mis-centered label.

Set the byte threshold by *measuring* the same page with the engine never drawing (load it
without `?autostart=1`), not by copying a number from a sibling script — a threshold below what
the DOM alone paints passes on a frame the engine never touched.

### Which smokes actually prove their claim, and which need your eyes

Nine of the fifteen assert something specific — a page verdict (`*_CHECK: PASS`), a pixel
ratio, a reported failure. **Six are byte-size-only**, so a green run means "a frame drew",
not "the right frame drew". For those, `SMOKE_KEEP=1` and *look*:

| Byte-size only — eyeball it | What only the screenshot can tell you |
|---|---|
| `centered_text_smoke.sh` | each label's center actually sits on its guide line (EW-001) |
| `game_feel_web_smoke.sh` | player, three dummies, platform gap and HUD are all present |
| `hdr_web_smoke.sh` | HDR keeps core-vs-mid distinct where LDR collapses them to flat grey |
| `wasm_smoke.sh` | the HUD says "Player #1" — i.e. the WebSocket handshake really happened |
| `embedded_atlas_smoke.sh` | the 12 tiles are the right tiles (its no-image-served check is structural, but grid maths are not covered) |
| `embedded_image_smoke.sh` | the sprite is the *right* image and not the white fallback — the exact failure the verbatim-key invariant exists to prevent, and it still draws a non-blank frame |

Sweeping all fifteen takes about fifteen minutes and is worth doing after any change to
the render path, the asset path, or a `web/build.sh` — they are the only checks that run
engine code in a browser at all.

### Anything CI cannot exercise

A windowed playtest, audio playback, hot-reload, or a gamepad. Get the real-behavior
confirmation before merging, not just green checks — see the judgment gates in the
`land-pr` workflow.
