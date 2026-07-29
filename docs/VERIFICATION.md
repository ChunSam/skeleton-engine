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

### Trap 5 — a stale `.exit` file from a previous session

An `until [ -f /tmp/verify.exit ]; do …; done` waiter matches a **leftover file from days
ago** and returns that old code instantly — the gate looks like it passed while `cargo
test` is still running. `rm -f` the file before spawning, or wait on the PID
(`while kill -0 <pid>`), and check the file's mtime alongside its contents.

### Trap 6 — `core.fileMode = false` makes `chmod +x` invisible to git

A script you `chmod +x` runs for you and ships **644 to everyone else**; `ls -l` shows the
working tree, which lies here. Use `git update-index --chmod=+x <files>` and verify with
`git ls-files -s '*.sh'`. (This is how v0.135.1 shipped a smoke script and its `build.sh`
without the bit — fixed in v0.135.2.)

---

## What each step does and does not cover

### The WASM step is lib+bins, never examples

Do **not** gate on `--target wasm32 --all-targets`: it fails on the native-only examples
(`platformer_game` / `mp_server` / `gpu_particles`, which pull in `rapier2d` /
`tungstenite` / `GpuParticleEmitter`). `cargo build --target wasm32-unknown-unknown`
(lib+bins) or `--lib` is the real wasm gate.

**Consequence:** an example can be broken for wasm indefinitely and the gate stays green.
`examples/embedded_image.rs` was unbuildable for `wasm32` from the day it was added (it
called the native-only `save_screenshot_headless` unconditionally) until v0.135.1. After
touching an example's `cfg(target_arch = "wasm32")` path — or adding an example that claims
to work on the web — build it explicitly:

```bash
cargo build --example <name> --target wasm32-unknown-unknown
```

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

### Optional wasm smoke checks (local only — CI has no Chrome or GPU)

Each builds an example to wasm, serves it, and renders it in headless Chrome. Prerequisites
are `rustup target add wasm32-unknown-unknown`, a matching `wasm-bindgen-cli`, and Chrome.

| Script | Asserts |
|---|---|
| `scripts/wasm_smoke.sh` | `coin_race` runs and its WebSocket path works |
| `scripts/wasm_save_smoke.sh` | the AEAD `localStorage` round-trip (7/7) |
| `scripts/wasm_audio_smoke.sh` | the `web_audio` surface incl. buses/ducking/positional |
| `scripts/centered_text_smoke.sh` | non-blank render at DPR=2 (EW-001 centering — eyeball it) |
| `scripts/embedded_atlas_smoke.sh` | no image is served beside the page **and** the frame is non-blank |
| `scripts/audio_reactive_smoke.sh` | `Audio::levels` reports a live level **and** `Audio::bands` a low-biased spectrum in a browser (the wasm `AnalyserNode` half shares almost no code with the native tap + FFT) |
| `scripts/game_feel_web_smoke.sh`, `bloom_web_smoke.sh`, `hdr_web_smoke.sh`, `render_format_query_smoke.sh` | their example renders on the web |

See **`docs/WASM_SMOKES.md`** for the full list and how to add one.

A byte-size check alone is weak — it proves *something* drew, not that it drew *correctly*.
Where a stronger structural assertion is available, pair the two: `embedded_atlas_smoke.sh`
also asserts no image file exists in the served directory, so a non-blank frame cannot have
come from a fetch. **Eyeball the saved screenshot** for anything positional; no byte count
catches a wrong tile or a mis-centered label.

### Which smokes actually prove their claim, and which need your eyes

Nine of the fourteen assert something specific — a page verdict (`*_CHECK: PASS`), a pixel
ratio, a reported failure. **Five are byte-size-only**, so a green run means "a frame drew",
not "the right frame drew". For those, `SMOKE_KEEP=1` and *look*:

| Byte-size only — eyeball it | What only the screenshot can tell you |
|---|---|
| `centered_text_smoke.sh` | each label's center actually sits on its guide line (EW-001) |
| `game_feel_web_smoke.sh` | player, three dummies, platform gap and HUD are all present |
| `hdr_web_smoke.sh` | HDR keeps core-vs-mid distinct where LDR collapses them to flat grey |
| `wasm_smoke.sh` | the HUD says "Player #1" — i.e. the WebSocket handshake really happened |
| `embedded_atlas_smoke.sh` | the 12 tiles are the right tiles (its no-image-served check is structural, but grid maths are not covered) |

Sweeping all fourteen takes about fifteen minutes and is worth doing after any change to
the render path, the asset path, or a `web/build.sh` — they are the only checks that run
engine code in a browser at all.

### Anything CI cannot exercise

A windowed playtest, audio playback, hot-reload, or a gamepad. Get the real-behavior
confirmation before merging, not just green checks — see the judgment gates in the
`land-pr` workflow.
