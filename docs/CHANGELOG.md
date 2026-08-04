# Changelog

All notable changes to `skeleton-engine` are documented here.

The package follows semantic versioning. It is currently **pre-1.0 (0.x)**: MINOR covers any release (including breaking changes), PATCH is a bugfix/point release; 1.0.0 will mark a deliberate compatibility commitment.

## 0.143.14

**The `embedded_image` web smoke could not be run at all — neither of its two scripts had the executable bit.** Tooling and docs only; no source, no API, no runtime behaviour changed.

`scripts/embedded_image_smoke.sh` and `examples/embedded_image/web/build.sh` both landed as `100644` in v0.143.4. `docs/WASM_SMOKES.md` invokes the first as `./scripts/embedded_image_smoke.sh`, and that script executes the second directly at line 82 — so the smoke was broken twice over, the same shape v0.135.2 fixed repo-wide when 26 of 31 scripts were missing the bit.

Three things hid it, and each is worth naming:

- **`core.fileMode = false`** in this repo, so the mode is invisible in `git status` and `git diff`. `git ls-files -s` is the only local view of it — Trap 6 in `docs/VERIFICATION.md`.
- **The file was not executable in the working tree either**, so there was no local run that could have worked.
- **It is a browser smoke, and no browser smoke is in CI** (they need Chrome plus a wasm-bindgen-cli matching `Cargo.lock`). v0.143.8 caught this exact trap on two *other* new scripts by checking `git ls-files -s`; these two were outside that check's scope.

Both are now `100755`, and no shell script anywhere in the repo is non-executable. Verified by running the smoke the way its own docs say: PASS, an 84,639-byte frame, and the screenshot **looked at** rather than only measured — the byte threshold cannot tell the correct sprite from the white fallback texture, which is the exact failure the verbatim-key invariant exists to prevent.

### Docs — three claims that had gone stale
- **`CLAUDE.md` said `verify.sh` "mirrors CI". It does not.** The gate covers CI's `wasm` and `docs` jobs in full and its `test` job bar two steps, and does not touch the `render` or `package` jobs at all. The not-covered list now names what it misses: the render tests and the three native render smokes, `cargo build --release` (the only check that the `lto = "thin"` shipping profile links), `scripts/hot_reload_smoke.sh`, and `cargo package --locked`.
- **"CI is ubuntu (plus one Windows *build* job)"** — v0.143.13 added `Build (macOS / Metal)`. Both platforms now compile in CI; neither ever *runs*, which is the distinction that matters and the one the line was making badly.
- **"None of the 15 smoke scripts is in CI"** — v0.143.11 put four native ones there. Rewritten as a property (browser smokes need Chrome, so they stay out; the native ones run) with a `grep` to check, rather than a count that goes stale again.

`docs/NEXT_WORK.md` also compared bytes against a character budget: the `handoff`/`wrap` skill sizes were recorded as "4,531 and 5,987 chars", which are `wc -c` **bytes**. The 800 guideline counts characters, and Hangul is 3 bytes each in UTF-8, so the item read as roughly twice as urgent as it is. Measured with `wc -m`: 2,245 and 3,195, i.e. 2.8× and 4.0×.

## 0.143.13

**The native job's cache was a guaranteed miss on every run, and had been for some time.** CI only; no library code, no scripts, no docs beyond this entry.

### The measurement
Run 30884878663, the native job's own log:

```
Cache not found for input keys: Linux-cargo-<hash>, Linux-cargo-
```

Even the `restore-keys` prefix fallback found nothing, because **no `Linux-cargo-*` entry existed at all** — the cache list showed ten entries totalling 8.63 GB of a 10 GB budget, for every job *except* this one.

The mechanism: the native job cached the whole `target/`, which made it the largest cache in the repo, and it is also the slowest job (16m41s), so it saved last. The five fast jobs (46s–1m53s) filled the budget first and the native entry was evicted as LRU every time. It therefore rebuilt from zero every run **and** spent **221 s — 22% of its wall time — writing a cache that was thrown away before the next run could read it.**

### Fixed
- **`Swatinem/rust-cache@v2` replaces the six hand-written `actions/cache` blocks.** It caches dependency artifacts and prunes the workspace's own — the part that changes every commit and made these caches enormous — and keys per job and toolchain itself.
- **A `concurrency` group, cancelling superseded PR runs.** Measured on the v0.143.10 branch: two full runs four minutes apart on the same branch, both allowed to finish, both red. `main` runs are explicitly *never* cancelled, for the reason below.

### Added — `Build (macOS / Metal)`
The same argument the Windows job was added on, applied to the other platform nobody was compiling. Until now `src/input/gamepad_macos.rs` (142 lines) and seven `target_os = "macos"` branches across `app.rs`, `app/window.rs`, `input/mod.rs` and `input/gamepad.rs` were built by one thing only: somebody remembering to do it locally, as `CLAUDE.md` asks.

That backend is not decorative — gilrs cannot read modern Xbox/PS5 pads on macOS because Apple's GameController framework claims them, so macOS runs a separate gamepad path entirely. Build only, matching the Windows job: gamepads, audio playback and windowed playtest still need a real machine.

⚠️ **A new job is advisory until it is added to the required status checks on `main`** — that is a repository setting, not something this file can do.

### The `push: [main]` run is not duplication — and nearly got deleted for looking like it
It re-tests a tree that has already passed: a squash-merge from an up-to-date branch produces the same tree the PR was built from (verified on #421 — both `323199e4`), and `main` is protected with all six jobs as required checks, `strict`, and `enforce_admins`, so nothing can arrive unverified. Every part of that is true, and the conclusion drawn from it was still wrong.

**GitHub isolates caches by ref.** A `pull_request` run writes into that PR's own scope, readable only inside it; caches written on the default branch are readable by every branch. So the `push: [main]` run is the only thing populating the cache the *next* PR restores from. Measured: all ten surviving entries are scoped to `refs/heads/main` — not one was written by a PR run. Removing that trigger would have made every PR start cold and quietly cancelled out the fix above.

It is now commented in `ci.yml` as load-bearing, since the next person to look at CI cost will reach the same wrong conclusion from the same correct facts.

## 0.143.12

**`coin_race` proves its server authority, with two clients contesting one coin.** The fourth acceptance test over the network stack, and the first to drive **two** clients at once — a contested coin has no meaning with one player, so no single-client test and no screenshot can reach this. No library code changed; examples and docs only.

The failure this guards against is the *flattering* kind. A client that deleted the coin and scored the point itself would look **better** in single player: the coin vanishes the instant you touch it, with no round trip to wait through. The damage exists only when two players touch the same coin — both see themselves win it, and from that moment the two scoreboards disagree forever. Neither screen shows anything wrong on its own.

Coverage goes from **7 of 21** playable games to **8**, which `docs/NEXT_WORK.md` has called the natural stopping point since 2026-08-03: every remaining game's headline feature is visible in a screenshot.

### Added
- **`COIN_RACE_SELFTEST=1 cargo run --example coin_race_game`**, six checks / exit codes. Checks 1-4 need no server: `hello` builds the field (`1`); **touching a coin claims it and does not delete it locally** (`2`); a `taken` removes the coin and credits whoever the server names, not us (`3`); a decided game freezes movement *and* further claims (`4`).
- **Checks 5-6 spawn the real `coin_race_server`** on an OS-assigned port and put **both** players on the same coin on the same frame: the server must resolve it (`5`), and the field must refill afterwards (`6`). **SKIP with exit 0** if that binary was never built.
- **`server_addr()` in `server.rs`** — `ADDR` unless `COIN_RACE_ADDR` overrides it. Fourth instance of the same additive shape.

### The two-client check needed a different kind of assertion
Score deltas are the obvious thing to measure and the wrong one: the server respawns a coin at a random position after every take, so one landing under a player's feet adds a point nobody asked for and the measurement flakes. The check asserts an **invariant** instead — *points gained == coins the server took away* — which holds however many coins get taken, plus *both clients see the same scoreboard*.

Both clauses earn their place. The sabotage that removes the server's first-claim-wins guard leaves the two boards **agreeing** on 1-1, because both clients faithfully apply both `taken` messages; it is caught only by 2 points scored against 1 coin removed.

### Two limits worth knowing before writing the next one
- **`NetworkClient` has no readable outbox**, so "a `grab` was sent" cannot be observed offline. Check 2 asserts the *consequence* instead — the coin is still standing and sits in `claimed` — and needs both halves, since a client that never notices the coin also leaves it standing.
- **Only the server reads `COIN_RACE_ADDR`.** `NEXT_WORK` had this example blocked on having no `protocol.rs` to host a `server_addr()`; the precedent it was copying puts the override on the server alone, so the client keeps dialling `SERVER_URL` and the self-test builds its own `NetworkClient` against the port it reserved. No shared module was needed.

## 0.143.11

**The four native smokes now run in CI.** They were local-only because "CI cannot render (no GPU)" — a claim that stopped being true when the lavapipe render job was added, and that three of the four scripts still state in their own headers. Nothing had run them since; every claim they make rested on someone remembering to run them on a Mac. No library code changed; CI, scripts and docs only.

### Added
- **Three render smokes in the `render` job**, which is the one with an adapter — `headless_screenshot`, `lighting_cap`, `packaged_assets`. They run under the same `SKELETON_REQUIRE_GPU=1` as the render tests, so a missing adapter fails rather than skips.
- **`hot_reload_smoke.sh` in the native job** — it needs no adapter, and it covers a path the hot-reload selftests do not: `DATA_ANIM`/`DATA_PARTICLES` watch a *direct* path, while this watches a **logical path under an asset root** (EW-008), the clause where `notify` used to register the caller's path and miss the real file.

### Why these four and not the other eleven
The remaining smokes are browser smokes — they need Chrome and a `wasm-bindgen-cli` matching `Cargo.lock`. These four are native, need neither, and cost **34 s** in total locally. Splitting them out was the whole point of scoping the fifteen rather than treating them as one lump.

None of the four has a skip path: they assert non-blank frames or a picked-up edit and fail hard, so there is no silent-pass risk to guard the way `selftests.sh` has to.

### Fixed
- The stale "CI is ubuntu-only and cannot render (no GPU)" headers in `headless_screenshot_smoke.sh` and `lighting_cap_smoke.sh`. That was the **third** stale CI assumption found this session, after the wasm-examples gap and the "no Chrome" claim.

## 0.143.10

**Audio in CI was attempted, measured, and does not work — the negative result is the deliverable.** "Audio is outside CI entirely" has been a standing risk since v0.140, and the obvious fix (give the runner a virtual sound card) turned out not to hold. Five CI runs went into establishing that. No library code changed; scripts and docs only.

### Added
- **`SKELETON_REQUIRE_AUDIO=1`** in `scripts/selftests.sh` — turns an audio-device skip from a tolerated opt-out into a **failure**. Default off, so a box without a sound card still passes. Run it locally when you want proof the audio checks actually executed rather than quietly opting out: `SKELETON_REQUIRE_AUDIO=1 ./scripts/selftests.sh`. This is the tool that made the negative result *legible* — without it every attempt below would have been a silent green.

### What was tried, and what each attempt measured

| Attempt | Result |
|---|---|
| **PulseAudio null sink, default latency** | The sink comes up and **rodio opens a device** — `beat_crawler`'s whole audio chain passed on CI, finding **16 kicks in a real mix at 0.638 s spacing**. But `audio_reactive` read rms **0.0000** against its 1200 ms rise deadline, and `survivor`'s peak reached 1.0000 while its **600 ms watchdog engaged**. |
| **Null sink + `PULSE_LATENCY_MSEC=30`** | **Worse.** `beat_crawler` finds *no* kick at all, `survivor`'s peak is 0.0000. The small buffer broke the one thing that worked, which refuted the burst-latency hypothesis that motivated it. |
| **ALSA `snd-dummy`** | **Not available at all** — the runner's azure kernel ships no such module even with `linux-modules-extra` installed. |

A null sink delivers samples in bursts rather than continuously, so the level tap publishes and then goes stale in the gaps. Checks that sample over *seconds* ride it out; sub-second deadlines land in the gaps. Those deadlines are calibrated against real hardware, and loosening them to make CI green would have discarded exactly the guarantee they exist to make — so they were left alone and the CI change was dropped instead.

**Every audio claim therefore still rests on a local device.** Recorded in `docs/VERIFICATION.md` so this is not re-litigated without new information.

### Also learned
A **corrupt cargo cache** produces `collect2: fatal error: ld terminated with signal 7 [Bus error]`, twice in a row, on a runner with **108 GB free**. It is not disk exhaustion, which is what that signature usually means and what it was first misread as. `gh cache delete` on the `Linux-cargo-*` keys cleared it.

## 0.143.9

**`orbital_dodger` proves its interpolation, and that collision agrees with what you see.** The third acceptance test over the network stack; the server-spawning harness transferred a third time with no changes, which is what makes it a pattern rather than one example's trick. No library code changed; examples and docs only.

This example is interpolation *in isolation* — no prediction, no reconciliation, no client→server message at all — so its failure mode is narrow and completely invisible in a still frame. A client that ignored its buffers and drew the newest 10 Hz sample renders hazards at plausible positions in every single frame. Only motion shows it, and only as judder, which is why the example ships an `I` key to toggle interpolation off: watching it was the sole way to know.

Coverage goes from **6 of 21** playable games to **7**.

### Added
- **`ORBITAL_DODGER_SELFTEST=1 cargo run --example orbital_dodger`**, six checks / exit codes. Checks 1-5 need no server: `Hello` spawns the full hazard set before any snapshot (`1`); hazard position is interpolated rather than snapped (`2`); the spin angle is interpolated on its **own** channel (`3`); the `I` toggle actually changes what is drawn (`4`); and **collision is tested against the displayed position, not the newest snapshot** (`5`).
- **Check 6 spawns the real `orbital_dodger_server`** on an OS-assigned port and asserts hazards arrive, move and stay in the field. **SKIPs with exit 0** if that binary was never built.
- **`protocol::server_addr()`** — `SERVER_ADDR` unless `ORBITAL_DODGER_ADDR` overrides it. Third instance of the same additive shape.
- `DodgerScene::on_enter` splits its world-building into a shared `build_world`, so the self-test stands up the arrangement the game uses. The socket deliberately stays in `on_enter`.

### Check 5 is the one that earns the test
At 10 Hz the displayed position and the newest snapshot are far apart — measured **63.3 px** against a **38 px** collision radius. A client that renders interpolated but collides against the raw snapshot kills you for touching a hazard that is not where you see it. That reads as "the hitboxes feel off", never as a bug, and no frame of it looks wrong.

Both halves are asserted, because each alone is passable by a broken client: one that never collides passes the "safe" half, one that collides with everything passes the "caught" half. The sabotage inverts them exactly — 31 catches where 0 are wanted, 0 where some are.

### Verified
- **All six exit codes proven by sabotage**, each reverted and the revert re-checked by `grep`: `Hello` ignored (`1`); the interpolation delay dropped (`2`); the **angle channel alone** snapped while position stayed interpolated (`3`); the `I` toggle unwired (`4`); collision moved to the raw snapshot (`5`); and — for `6` — a **server** that sends `Hello` and never a `Snap`, which leaves all five offline checks green and is caught only live.
- Three consecutive runs green. The client still builds for `wasm32`.

### Fixed during the work — a test-side lesson that generalizes
**A warm-up is part of the property, not setup noise.** Check 5's first draft parked the player from frame 0 and scored 12 catches in the case that must be safe. The cause was not the game: until the buffer holds more than `interp_delay` of history there is nothing to interpolate *between*, so the displayed position and the newest sample coincide and a player parked on either is standing on the hazard. The check now waits out that window at the player's spawn and counts catches only over the parked phase. Any interpolation assertion has this window; measuring inside it measures the warm-up.

Also worth recording, because it cost a confusing three-run result: **rebuilding the client does not rebuild the server.** `cargo build --example orbital_dodger` left a sabotaged `orbital_dodger_server` on disk, so a reverted tree kept failing check 6. This is the same shape as the skip trap `scripts/selftests.sh` guards.

### Fixed: `scripts/selftests.sh` no longer hardcodes its list
The script added one release ago listed its selftests by hand, and **this release's new selftest was not in it** — the gate ran green having never executed the test that was the entire point of the change. Caught here only because the omission was one commit old.

The list is now **derived**: an example is a selftest iff it reads a `<NAME>_SELFTEST` environment variable (8 discovered, up from the 7 hardcoded). A registry you must remember to edit is a registry that silently shrinks, which is the same failure the script exists to prevent, one level up. `scripts/build_wasm_examples.sh` already derived its set for this reason; the selftest runner should have from the start.

## 0.143.8

**CI now runs the acceptance tests, and builds the examples that ship to the web.** Two gaps, both found by asking what the gate actually executes rather than what it is supposed to cover. No library code changed; CI, scripts and docs only.

The first is the uncomfortable one: **the seven `<NAME>_SELFTEST` acceptance tests had never run in CI, or in `verify.sh`.** Neither file contained the string `SELFTEST`. Every one of them was proven non-vacuous by sabotage on the day it was written, and then nothing ran it again. They all still pass (verified before wiring them up), so nothing had rotted — but that was luck, not a guarantee, and these tests are the project's only defense against a headline feature degrading gracefully into silence.

### Added
- **`scripts/selftests.sh`** — runs all seven selftests and reports one verdict. Wired into both CI's native job and `verify.sh`, so the gate keeps mirroring CI. ~58 s locally with a sound card; less in CI, where the audio halves skip.
- **`scripts/build_wasm_examples.sh`** — builds the **16** examples that declare a `#[wasm_bindgen]` entry point for `wasm32`. Wired into CI's Build (WASM) job and `verify.sh`. The set is **derived from the entry point, not hardcoded**, so a new web example is covered without anyone remembering to register it.

### Why the selftest runner is a script and not a `cargo run` line
**A skip is not a pass.** Every selftest correctly opts out with exit 0 when its environment cannot support a check — a machine with no sound card should not fail the build. But that makes a skip indistinguishable from a pass at the exit code, and one skip is *not* environmental: the networked tests drop their live checks when the sibling server binary is missing, and `cargo run --example salvage_run` builds only `salvage_run`. A naive CI step would therefore silently drop exactly the checks that cover the most and report success.

Measured, not assumed: with `predict_shooter_server` hidden, the raw exit code is **0**. The script builds `--examples` first, then treats `no audio device` as the only tolerable skip and fails on every other. Same principle as the render job's `SKELETON_REQUIRE_GPU=1`.

### Why the wasm example build names its targets
A blanket `--examples --target wasm32` fails, and correctly — several examples are native-only by construction. The old note in `docs/VERIFICATION.md` listed three (`platformer_game`/`mp_server`/`gpu_particles`); the real set is larger, and the rest fail on native-only *engine* APIs rather than dependencies: `headless_screenshot`, `hot_reload_asset_root`, `tile_anim_stagger`, `slider_keyboard_step`, `ui_stepper`, `ui_tabs`. So the build has to name the examples meant to reach the web, which is what the entry-point derivation does.

### Fixed during the work
- **`chmod +x` on the two new scripts was invisible to git** — this repo has `core.fileMode = false`, so both were staged `100644` and CI would have failed with permission denied. This is Trap 6 in `docs/VERIFICATION.md`, caught by checking `git ls-files -s` rather than trusting the local file mode. Fixed with `git update-index --chmod=+x`.
- The selftest summary filter matched `ok:`/`PASS` but not `OK:`, which `audio_reactive` uses — so that test printed nothing under its heading and read as having done nothing, while actually running four checks.

### Still not covered, stated plainly
- **Audio playback.** The audio halves skip on CI, so every audio claim still rests on a local device.
- **Running on the web.** None of the 15 `scripts/*_smoke.sh` runtime smokes is in CI; compiling for wasm is not running on wasm.
- **macOS/Windows `cfg` branches.** CI is ubuntu plus one Windows *build* job.
- Hot-reload, previously on this list, **is** covered now — `DATA_ANIM_SELFTEST` and `DATA_PARTICLES_SELFTEST` do real `notify` file-watching in CI.

## 0.143.7

**`predict_shooter` proves its client-side prediction and server reconciliation.** This is the second acceptance test over the network stack, and it copies the server-spawning harness `salvage_run` built — which was the whole point of building it. Reconciliation is the load-bearing invisible property here, and its failure *flatters*: with the client's `reconcile` call never reached, input still moves you the instant you press a key and the motion is still perfectly smooth, because prediction alone is what makes it feel good. The only symptom is that the server quietly disagrees about where you are — no message says so, nothing renders differently, and in a single-window playtest there is nothing to compare against. `client_net.rs` already unit-tested `Prediction` in isolation; nothing covered the **ECS wiring** that calls it. No library code changed; examples and docs only.

Coverage goes from **5 of 21** playable games to **6**.

### Added
- **`PREDICT_SHOOTER_SELFTEST=1 cargo run --example predict_shooter`**, seven checks / exit codes. Checks 1-5 need no server: they drive the real `ShooterClient` by injecting real protocol JSON into `Events<NetworkEvent>` — `welcome` wires up the local player (`1`); held input is predicted immediately **and reaches the avatar's `Transform`** (`2`); an authoritative correction reconciles (`3`); reconciliation **replays the un-acked inputs** rather than snapping to the acked position (`4`); remote players are interpolated rather than snapped (`5`).
- **Checks 6-7 spawn the real `predict_shooter_server`** on an OS-assigned port and assert what no client-side test can reach: that prediction **converges** on the server's authority (`6`), and that a `fire` input round-trips into a server-spawned bullet (`7`). They **SKIP with exit 0** if that binary was never built.
- Check 6 leans on a disagreement the example builds in on purpose. The client seeds its prediction at the field centre while the server spawns each player at a *random* position — `handle_message` says so: "the first snapshot reconciles to the real spawn". A client that never reconciles stays anchored to the centre forever. Measured across runs: **0.00 px** between the client and the server's last word after ~285 px of driven movement.
- **`protocol::server_addr()`** — `SERVER_ADDR` unless `PREDICT_SHOOTER_ADDR` overrides it. Additive: unset, it is byte-identical to the constant. Same rationale and same shape as `salvage_run`'s, and like it the windowed client keeps dialling the constant while the self-test builds its own `NetworkClient` against the port it reserved.
- Held input is synthesized with **`InputScript`** — the engine's own `ENGINE_INPUT` replay path — because `InputState` has no public press setter. That routes the checks through the real `read_input` instead of a harness that fakes a direction.

### Verified
- **All seven exit codes proven by sabotage**, each reverted and the revert re-checked by `grep` (`client_net.rs` came back byte-identical to `main`): `welcome` not seeding the prediction (`1`); the `Transform` write dropped (`2`); the `reconcile` call removed (`3`); `reconcile` snapping without replay (`4`); the interpolation delay ignored (`5`); inputs predicted but **never sent** (`6`); `fire: false` on the wire (`7`).
- Two of those are worth stating on their own. The `reconcile`-removed sabotage leaves checks 1-2 **green** — the game still predicts, the avatar still moves, it still feels right — which is exactly the exposure. And the never-sent sabotage leaves all five offline checks green and is caught only by check 6, which is the argument for the live checks existing at all.
- Live checks are stable against the random spawn: the drive direction is picked away from the nearer wall, so the asserted travel is never clamped short. Three consecutive runs measured 284/288/288 px travelled and 0.00 px of disagreement.
- SKIP path exercised by hiding the server binary (exit 0, offline checks only). The client still builds for `wasm32`.

## 0.143.6

**`salvage_run` gets an acceptance test, and it is the first one over the network stack.** Interest management is the most screenshot-invisible feature in the tree, and its failure is worse than invisible — it is *flattering*. The server signals that an entity left the area-of-interest only by omission, so with the client's eviction dead the world only ever gains entities: the HUD's "streaming N / total" climbs toward the total and the game looks like it is streaming beautifully. There is no message whose absence anything would notice. Nothing in the client had a test of any kind before this. No library code changed; examples and docs only.

Coverage goes from **4 of 21** playable games to **5**, and the harness this needed unblocks the other three networked games.

### Added
- **`SALVAGE_RUN_SELFTEST=1 cargo run --example salvage_run`**, six checks / exit codes. Checks 1-4 need no server: they drive the real `SalvageClient` by injecting real protocol JSON into `Events<NetworkEvent>` — entities stream in (`1`); **removal-by-omission** evicts what stopped arriving and never drops what is still arriving (`2`); a disconnect clears `RemoteEntities` *and* the parallel `salvage_buf`/`drone_buf`/`last_seen` maps it does not own (`3`); streamed motion is interpolated rather than snapped (`4`).
- **Checks 5-6 spawn the real `salvage_run_server`** and assert the properties no client-side test can reach: that the server tailors its snapshot to the AOI radius (`5`) and that shrinking the radius back *drains* what it streamed (`6`). Measured: radius 200 streams **3** entities, radius 1100 streams **97 of 120**, and shrinking back drains 97 → **5**. They **SKIP with exit 0** if that binary was never built — the rule `BEAT_CRAWLER_SELFTEST` uses for a missing audio device.
- **`protocol::server_addr()`** — `SERVER_ADDR` unless `SALVAGE_RUN_ADDR` overrides it (client and server both). Additive: unset, it is byte-identical to the constant, so every documented way of running the example is unchanged. It exists because the port was hardcoded on both sides and the server `bind`s it with an `expect`, so a self-test on 9005 would collide with a server the user is already running — a hang-and-fail, not a skip. The test asks the OS for a free port and hands it to the child.
- `SalvageScene::on_enter` splits its world-building into a shared `build_world`, so the self-test stands up the same arrangement. The socket deliberately stays in `on_enter`: the offline checks must not have a live connection racing them, and the connect line is covered by checks 5-6 instead.

### Verified
- **All six exit codes proven by sabotage**, each reverted and the revert re-checked by `grep`: `ingest` dropped (`1`); eviction disabled (`2`); a parallel map left behind on disconnect (`3`); the interpolation delay ignored (`4`); the server's AOI filter made unbounded — 120 vs 120 (`5`); and, for `6`, a server whose **interest set never shrinks** (`c.r = c.r.max(r)`), which passes check 5 at 24 → 92 and fails 6 at 92 → 93.
- Default port unchanged and exercised end to end: the server started with no environment override, the real client run against it, and the frame **looked at** — HUD reads `Connected`. The client still builds for `wasm32`.

### Fixed during the work — two test-side lessons that generalize
- **An end-state assertion missed a flicker.** Check 2's "still-arriving entities are never evicted" half originally read the map once, at the end. A `STALE_TIMEOUT` sabotaged to 0.05 s — below the 0.083 s snapshot interval — evicts a still-arriving entity *between* snapshots and re-spawns it on the next one, so the endpoint looks perfect. It passed. The check now watches every frame, and that sabotage now fails it naming the ids that flickered.
- **Interpolation lag must be measured against where the *sender* was, not against the newest snapshot.** That sample is already up to one snapshot interval old, so the first draft expected 37.5 px against a measured 17.5 and only passed because the tolerance was loose enough to swallow the gap. The expectation is now derived from the trajectory the test itself authored; it matches to the pixel (995.5 vs 995.5) with a tolerance of one tick of motion.
- Also worth knowing before writing the next one of these: **`NetworkClient::connect` dials once and does not retry.** Connecting to a just-spawned server and hoping is not a slower path, it is a guaranteed failure — probe with a plain `TcpStream::connect` until it binds.

### Also
- `CLAUDE.md`'s capture caveat gains its second instance: a networked game photographs `streaming 0 / 120` while the server is happily sending, for the same reason an audio meter photographs `0.0`. Anything arriving on a wall clock — audio, sockets, file watchers — needs a loop paced off `Instant`.

## 0.143.5

**Hot-reload now has playable-example acceptance tests.** `data_anim` and `data_particles` exist to show one thing — edit a RON file while the game runs and the animation/effect changes — and until now nothing anywhere asserted it. `CLAUDE.md` lists hot-reload among the things CI cannot run, and every link in the chain fails *silently*: a watch that was never registered, a registry that reloaded nothing, or a re-sync system that stopped rebuilding its component all leave a perfectly good sprite animating the clips it was born with. That is what a screenshot photographs either way, and what the person editing the RON sees when their change "did nothing" and they go looking at their own file instead of at the engine. No library code changed; examples and docs only.

This takes `<NAME>_SELFTEST` coverage from **2 of 21** playable games to **4**.

### Added
- **`DATA_ANIM_SELFTEST=1 cargo run --example data_anim_game`** — six checks / exit codes: the animation advances at the fps the RON declares (`1`); each clip's own fps reaches the render layer (`2`); an edit on disk reaches the registry (`3`); it reaches the **already-playing** `AnimationPlayer` (`4`); the selected clip survives the rebuild (`5`); the new clip is what actually gets drawn (`6`).
- **`DATA_PARTICLES_SELFTEST=1 cargo run --example data_particles_game`** — the same six-code shape over `ParticleConfigRegistry`. Being the *second* registry through this path is the point: it makes the arrangement a pattern rather than one example's trick.
- Both split their setup into a `setup(&mut App, path)` that `main` and the self-test share, so the ordering that fails quietly — load and watch the file *before* building the component from it — is the ordering under test. The self-test passes a **temp copy** of the RON, so the tracked asset is never edited and a mid-run death cannot leave the repo dirty.

### Verified
- **All twelve exit codes proven by sabotage**, each reverted and the revert re-checked by `grep`: fps → 0 in the RON (`1`); the `2` key branch dead (`2`); `watch_animation_clip_path` / `watch_particle_config_path` removed from `src/app/editor/loading.rs` (`3` — "the watcher did NOT report the change"); the re-sync branch dead (`4`); the rebuild forced back to clip 0 (`5`); a **partial** re-sync that copies the new rate but keeps the old frames/size (`6`).
- **Check `6` was rewritten to be falsifiable rather than kept as a tripwire.** Asserting only the reloaded *rate* left nothing that could fail once checks 4 and 5 passed — the player is rebuilt wholesale, so there is no staleness to catch. Making the edit change the frame list / particle size **as well as** the rate gives it a real failure to detect, and the assertion compares the drawn `UvRect`s / `Transform::scale`s against the ones drawn *before* the edit rather than against values the test computed itself.
- **Thresholds measured, then set — and one of them caught the guess.** The animation counts are exact (`SECONDS × fps`: 8, 24, 60). The particle population settles at `spawn_rate × lifetime` **plus one frame of spawning**, so the first flat tolerance passed the two slow emitters and failed the fast one at 224 against a wanted 220; the tolerance is now `rate * DT + 2`, which is the shape of the error rather than a fudge over it.
- Both examples still run their normal windowed path, checked through `ENGINE_CAPTURE` and the frames **looked at**, not just measured.

### Fixed
- **A wrong comment in `data_particles`, corrected against a measurement.** Replacing the `ParticleEmitter` every frame was documented as making it "never spawn particles". It does not: it resets the emit timer every frame, which clamps emission to one particle per tick — **60/s against a configured 90/s**, a 63-particle population settling at 42. It under-emits rather than stopping, which is why the self-test asserts the rate and not merely that something spawned.

### Also
- `docs/VERIFICATION.md` gains **Trap 7** — a squash-merge leaves the original tip dangling, so an already-landed branch reads as "ahead by N" and the branch graph cannot clear it for deletion; verify by content. Moved out of `docs/NEXT_WORK.md`'s "Recently closed" (which rolls off every session) because it had no other durable home.

## 0.143.4

**`embedded_image` now runs on the web, instead of merely compiling for it.** `App::load_image_bytes` exists so a PNG can ship *inside* the `.wasm` module — the single-file/jam case where a path load would need async fetch plumbing — but the only thing ever proving that on wasm was a compile. v0.136.0 closed exactly this gap for the atlas twin (`embedded_atlas` got a browser harness and a render smoke) and left the image half carrying only a `cargo build --target wasm32` behind it. It is now symmetric. No library code changed; this is examples and tooling only, so no public API is affected.

The gap was not theoretical for this particular example: it was silently unbuildable for `wasm32` from the day it was added until v0.135.1, because nothing built it for that target. "It compiles" was a weak claim to begin with, and it was the only one being made.

### Added
- **`examples/embedded_image/web/`** (`index.html` + `build.sh`) — the `cargo build --example` + `wasm-bindgen` path, mirroring `embedded_atlas/web/`. The example gained a `#[cfg(target_arch = "wasm32")] run_embedded_image()` entry point; `main` and the headless acceptance test split off a shared `build_app()` so the browser runs the *same* code the native run does.
- **`scripts/embedded_image_smoke.sh`** — a headless-Chrome render smoke (optional local check; CI has no Chrome/GPU). Like its atlas twin it asserts **both** halves of the claim: that no image file is served beside the page, *and* that the rendered frame is non-blank. Either alone is weak — a non-blank frame could have come from a fetch, and an empty directory proves nothing if nothing drew — but together they say the sprite rendered and cannot have come from a file.
- The example moved to `examples/embedded_image/embedded_image.rs` (with a `[[example]]` entry) so the harness has a directory to live in, and its `include_bytes!` switched to the `CARGO_MANIFEST_DIR`-anchored form `embedded_atlas` uses, so the embed does not break the next time the file moves.

### Verified
- **Rendered in a real browser** at DPR=2 under SwiftShader: the 32×32 sprite draws from the embedded bytes, the HUD reads `working dir: <unknown>` (the wasm case — there is no filesystem to have loaded from), and the `App::asset_failures()` panel names `embedded/corrupt`. The screenshot was **looked at**, not just measured: the byte check cannot distinguish the correct sprite from the **white fallback texture**, which is the exact failure the verbatim-key invariant exists to prevent and which still draws a non-blank frame.
- **The byte threshold was measured, not inherited.** Loading the same page *without* `?autostart=1` — the DOM paints, the engine never draws — yields **5,582** bytes against the real frame's **84,639**. The 15,000 threshold sits between them with room on both sides. Copying a sibling script's number would have been a guess about a differently-sized canvas.
- Native `HEADLESS_SHOT` acceptance test unchanged and still passing after the move.

### Known, unchanged, and deliberately not "fixed"
- `embedded_image` now joins six sibling directory-based examples (`embedded_atlas`, `audio_facade`, `centered_text`, `game_feel`, `web_audio`, `wasm_save`) that `cargo package` **warns about and skips**, because `include` lists `examples/*.rs` and not `examples/*/*.rs`. CI's `cargo package --locked` stays green (a skipped target is a warning, not an error). Widening `include` would **break** it: these examples `include_bytes!` from `examples/assets/`, which is not packaged either, so packaging their sources would fail the verification build on a missing PNG. Harmless in practice — the engine is [unpublished by design](VISION.md).

## 0.143.3

**`beat_crawler` fires its melee impact through `Audio::play_sfx_metered`** — the playable-game acceptance test v0.143.0 shipped without. Until now the only caller was `examples/audio_facade`, which demonstrates the surface but is a demo, and `CLAUDE.md`'s rule is that a feature is not done until a small playable example exercises it in real play. No public API change; example-only.

The choice of host was not decorative. `beat_crawler` is the one place where a **looping** metered track and an **overlapping** metered one-shot sound at the same time, and both go through `append_decoded` — the function where v0.141.2's loop-meter bug lived. That combination had never been exercised anywhere.

### Added
- **`assets/hit.wav` + `assets/hit.py`** — a 0.13 s impact synthesized from scratch (mid sine sweep + first-differenced noise), CC0 like the soundtrack. Built empty where the turn clock listens: **0.75%** of its energy sits in 20–200 Hz against the kick's **99.44%**, measured with a flat window.
- **The meter drives the flurry shake.** Not a hit count — the game already knows how many swings landed. What it does not know is how loud the result is *right now*, so the shake scales with the summed peak and decays with the clip's real tail instead of a hand-tuned timer.
- **Three new self-test checks (exits `6`/`7`/`8`)**: the impact meter reads something at all; three overlapping impacts read louder than one; and the turn clock keeps finding kicks while impacts are sounding.

### Verified
- **Real device, `BEAT_CRAWLER_SELFTEST=1`:** 16 kicks at 0.638 s spacing (grid 0.640 s), on-grid 15/15 — unchanged from v0.142.0 — plus impact meter **single 0.8000 → burst-of-3 1.0000**, and 3 kicks still heard while swinging. CI cannot produce any of this.
- **The summing check is non-vacuous by construction:** one voice reads the clip's own 0.80 normalization and three saturate the meter's 1.0 ceiling. Rendering `hit.py` at 1.0 would put a single swing on the ceiling too and leave the check nothing to discriminate — noted in `assets/README.md` so a future edit does not silently erase it.
- **Proven non-vacuous by sabotage**, each reverted after measuring: removing `enable_analysis(HIT_METER)` → exit `6` (`0.0000`, the clip still sounding — the v0.141.2 bug's exact shape); firing 1 impact instead of 3 → exit `7` (`0.8000` vs `0.8000`).
- **Exit `8` could NOT be tripped, and that is recorded rather than papered over.** Firing the bass-heavy `soundtrack.wav` as the impact clip changed the kick count not at all, and firing it at `BEAT_METER` trips exit `6` before `8` is reached. The reason is structural: on native each meter is a tap on its own channel, so `bands()` reads the music's tap and never the mixer output — the two *cannot* leak. Exit `8`'s upper bound is therefore a tripwire for a future topology change (the wasm backend, where several sources share one `AnalyserNode`), not something a badly chosen clip can trip today. Its lower bound — the clock must keep working while impacts sound — is the part that earns its place. The claim has been corrected in the example's docs and in `assets/README.md`.
- The first draft of the summing check **failed for a test-side reason** and is recorded here because the failure mode is reusable: it overloaded one `Vec` to carry both impact times and kick times, so the kick pushes grew the vector and the burst's `len() == 1` guard never came true. The burst never fired, and the `0.0007` reading was the single hit's decaying tail — a green-looking measurement of a thing that never happened.

## 0.143.2

**`SceneCmd::Replace` now warns when it discards a system the game registered on the `App`.** Systems added with `App::add_system` / `add_system_labeled` land in the *scene* portion of the schedule, which a `Replace` (and therefore `set_scene`) drains wholesale. For a system a `Scene` registered in `on_enter` that is correct — it dies with its scene. For one added straight to the `App` it is a silent loss: the system simply stops running, and nothing anywhere says so.

This is not hypothetical. `beat_crawler` shipped several releases with its `AudioFacadeSystem` registered before `set_scene`, so `Audio::update` never ticked, `bands()` returned `0.000` forever, and the game's headline feature — "the turn clock is the music" — ran permanently on its watchdog fallback. The only symptom was a HUD string reading `schedule (nothing heard)` instead of `listening` (fixed in v0.142.0). A `log::warn!` costs nothing and makes the next occurrence loud.

### Added
- A `log::warn!` on `SceneCmd::Replace` naming how many systems are being discarded and pointing at `Scene::on_enter` + `SystemRegistrar` as the fix.
- The count is `scene_len - Σ owned`, where `owned` is what each scene on the stack registered — so **a scene's own systems are never reported**, only the excess that came from the `App`. This covers both orderings of the mistake: registering before the first `set_scene` (beat_crawler's case, where the stack is empty and the whole scene portion is excess) and registering on the `App` after a scene is already running.
- Three tests: the arithmetic (scene-owned → 0, App-registered → the excess, and a saturating floor so drifting bookkeeping cannot underflow), plus a behavioural pair — a system added before `set_scene` must **not** run afterwards, and the same system registered through the scene's `SystemRegistrar` must. The pair matters: without the control, the first test would also pass if systems stopped running entirely.

### Verified
- **Proven non-vacuous by sabotage:** panicking on the warning branch instead of logging fails the before-`set_scene` test with `orphaned=1` and leaves the scene-registered control passing, i.e. the branch is reached with the right value in exactly one of the two cases. Reverted after measuring, removal re-checked by `grep`.
- **Ships silent:** all 20 examples that call `set_scene` were checked and none registers a system on the `App` first, so no existing example emits the new warning.

## 0.143.1

**`SURVIVOR_SELFTEST=1` — the survivor example can now prove its headline feature is alive.** `beat_crawler` ran for several releases with its turn clock silently on a watchdog fallback, and the only symptom was a HUD string nobody read (v0.142.0). `survivor` has the same shape and had no self-test at all: its shake and pulse degrade *gracefully* onto `FEEL_WATCHDOG` when the meter reads nothing, so a dead metering path presents as a slightly duller game rather than as a bug. This closes that exposure from both sides — the fallback must engage when it should, and must **not** engage when a real device is present. No public API change; example-only.

### Added
- **`SURVIVOR_SELFTEST=1 cargo run --release --example survivor_game`** — a headless acceptance test that ticks the game's **real system chain** (grid → bullets → spawn → seek → engine `SteeringSystem` → thruster → collision → `AudioFeelSystem` → particles, in `main`'s order), not a probe that reimplements it. Five checks, exit codes `1`–`7`:
  1. `drive_from_amplitude` still spans its documented range (`0.35` at silence → `0.69` at one voice → `1.00` at the summed ceiling, monotonic). Pins the re-basing v0.141.0 forced.
  2. Seekers close distance — 32 seekers closed 60.0 px in 30 frames. Guards the surface that already shipped one silent O(N²) regression.
  3. A bullet kills and its pool slot comes back (1 kill in 18 frames, enemy despawned, no live bullet left). A pool leak starves the gun silently.
  4. With **no** audio device the watchdog engages and the fallback keeps firing — measured: engaged at 0.63 s against the 0.60 s limit, 12 fallback shakes over 153 kills.
  5. With a **real** device the meter drives the feel and overlapping kills sum past one voice — measured: 147 kills in 2.5 s, peak **1.0000** across 146 live frames (span 0.82), watchdog never engaged.
- Checks 1–4 need no audio device, so CI runs them; check 5 skips (exit `0`) when there is no device, matching `BEAT_CRAWLER_SELFTEST`.

### Changed
- `main`'s world construction is extracted into `init_audio` + `setup_game`, and `spawn_enemy` now returns the `Entity` it spawned. The self-test builds its `World` through the same two functions — a harness that rebuilt the setup itself would only ever prove that its *copy* works, and the `enable_analysis`-before-first-play ordering that `init_audio` owns is exactly the thing that fails silently.

### Verified
- **Proven non-vacuous by sabotage**, each reverted after measuring: removing `enable_analysis` → exit `5` (peak `0.0000`, 147 kills still scored — the game plays fine, which is the point); `play_tone_metered` → `play_tone_on_channel` → exit `7` with the peak pinned at **0.6000**, reproducing the pre-v0.140.0 measurement exactly; removing the watchdog flip → exit `4`; dropping the `Seek` component → exit `2`. Three consecutive clean runs exit `0`.

## 0.143.0

**`Audio::play_sfx_metered` — the clip counterpart of `play_tone_metered`.** A one-shot that overlaps consecutive plays *and* is metered. 0.140.0 closed that gap for tones only, and `enable_analysis`'s docs said so explicitly; clips still faced the original either/or. `play_sfx` / `play_sfx_on_bus` already overlap (they round-robin a ring of anonymous voices on native) but that ring has no name for `enable_analysis` to address, and moving a clip onto a named channel to meter it costs the overlap, because a replay there **cuts** what was already playing.

Same shape as the tone path and the same semantics: a ring of `POLY_VOICES` private sink channels (`__poly_{meter}_{voice}`) rotated per call, all pointing their level tap at one shared meter entry, with `levels()` reporting the **sum** of the sounding voices. No new policy — `sum_levels` and `combine_voices` are reused unchanged.

The one real difference from the tone path is where the tap goes in. A tone is a `SamplesBuffer` that `play_tone_poly` wraps itself; a clip must be decoded, and the decode → effects → repeat → tap → pan chain lives in `append_decoded`. The voice is therefore threaded down to it rather than applied at the top, which keeps both paths sharing one chain instead of forking it.

### Added
- **`Audio::play_sfx_metered(meter, bytes, bus)`** — native `AudioManager::play_sfx_poly`, wasm `WebAudio::play_sfx_metered`. The wasm side needed no voice pool, exactly like the tone path: `play_sfx_to` already builds a fresh per-source gain node per call, so the only missing piece was routing each into the meter's `AnalyserNode` — and connecting several sources to one analyser makes the browser mix them, which *is* the sum contract.
- Two device-free tests: that the tone and clip paths share one voice ring (separate counters would put a tone and a clip fired together under one name on the same channel, and one would cut the other), and that a voice channel is private and never collides with the meter name.

### Changed
- **`enable_analysis`'s docs no longer say clips are excluded.** They were accurate; they are not any more.

### Documented
- **⚠️ `Audio::levels` now warns that a headless `ENGINE_CAPTURE` run cannot photograph a meter.** Capture advances the game with a fixed `1/60` dt as fast as the CPU allows while the audio thread publishes in real time (~21 ms), so the smoothing release drains within milliseconds of wall clock and the captured frame reads `0.0` even though the sound is playing correctly. This cost two separate investigations before it was written down — verify metering in real time, never from a captured PNG.

### Verified on a real device (CI cannot exercise any of this)
- One metered clip peaks at **0.4929**; three fired 60 ms apart peak at **0.9259** (**1.88×**) — they sum, so they overlap rather than cutting.
- The control: the same three fired **600 ms** apart peak at **0.4929**, identical to a single voice. Per-voice staleness works on the byte path too — a drained voice stops contributing instead of accumulating.
- `examples/audio_facade` gains key **`4`**: three overlapping metered clips on one meter, with the summed level in the readout.

## 0.142.0

**`examples/games/beat_crawler` now runs its turn clock off a real mixed soundtrack instead of two tones chosen to be trivially separable.** The old version played a 110 Hz kick and an 880 Hz blip and asserted they were far apart in the low band — measured 4.00 vs 0.61, 6.5× — which is a test that cannot fail. This is the first case where the low-band detector could genuinely be wrong, and it was: three separate things had to be fixed before the example worked.

The track ships as `examples/games/beat_crawler/assets/soundtrack.wav`, synthesized from scratch by the `soundtrack.py` beside it (sine arithmetic and a seeded PRNG — CC0, no sample pack), the same provenance rule as `src/audio/fixtures/README.md`. It is PCM rather than Ogg because the loop has to be sample-exact.

### What the mix broke, and what fixed it

- **Metering died after the first loop.** Shipped separately as **0.141.2** — the analysis tap sat inside `repeat_infinite`'s buffer. Without that fix this example cannot work at all.
- **The first arrangement made the kick undetectable.** With the bass on C2/F2/G2, every low band sat pinned at full scale and the kick's transient vanished into it; **no threshold worked** — every setting fired on bass wobble. Moving the bass up an octave is what a real mix does for the same reason, and it leaves the two separable without separating them.
- **`AudioFacadeSystem` was never running.** Registered on the `App` *before* `set_scene`, and `SceneCmd::Replace` swaps out the entire systems list — so the system that ticks `Audio::update` was silently dropped, `bands()` returned `0.000` forever, and the game ran permanently on `BEAT_WATCHDOG`'s schedule fallback. This predates the soundtrack: the example's headline feature was not actually running in the playable game, and the only symptom was the HUD reading "schedule (nothing heard)". It is now registered by `CrawlScene::on_enter`, so the ordering cannot be got wrong again.

### Changed
- **`LOW_BANDS` 4 → 2.** Measured per layer: the kick owns bands 0–1, the bass saturates 2–6. Summing a saturated band adds a constant, not information.
- **`KICK_THRESHOLD` 1.2 → 1.6**, mid-plateau: over 7 bars the correct count (28) and spacing (0.640 s, sd 0.03) hold anywhere in 1.45–1.95.
- **Arm/re-arm replaced by a retrigger cooldown (`KICK_COOLDOWN = 0.40`).** **This is the second independent confirmation of 0.139.0's finding** — `examples/games/survivor` hit it first, on a completely different signal. A rising-edge latch assumes silence between events; a mix has none, so the low band never falls back far enough to re-arm honestly. Measured over the same 7 bars: arm/re-arm 31 fires with gap sd 0.16 s, cooldown exactly 28 at sd 0.03 s.
- **`BEAT_CRAWLER_SELFTEST=1` asserts the real question.** It no longer measures two tones; it runs the game's own detector against the mix and asserts the kicks are found at the spacing the music actually has. Exit `5` now means "detections did not land on the beat grid". Measured: **16 kicks over 4 bars, mean gap 0.638 s against a 0.640 s grid, 15/15 on-grid.**
- `PATTERN` no longer plays anything — the audible groove is the `.wav`. It survives as the watchdog's schedule and as the written description of the bar.

### Notes
- **A timing trap worth knowing:** the first measurement loop paced itself with `t += 1.0/60.0`. `sleep(1/60)` sleeps *at least* 1/60, so that clock runs slower than the music and every measured gap came out ~28% short — which made a correct detector look like it was over-firing by 40%. All timings here come off `Instant`.
- Headless `ENGINE_CAPTURE` runs still show "schedule (nothing heard)": frames run far faster than sound, so game time outruns real audio and the watchdog necessarily drives them. That is the documented behaviour, not a regression. Real-time behaviour is what the selftest covers.

## 0.141.2

**Metering a looping sound went dead after its first pass.** `levels()` and `bands()` reported a live signal for one play-through of a looped source and then flat zeros forever — while the sound kept playing, audibly, with `is_channel_playing` still `true`. This hit `play_music` (whose whole point is that `Audio::MUSIC_CHANNEL` is meterable) and `play_at_on_channel`. Any game driving visuals or logic off a looping track saw the effect run for a few seconds and then quietly stop.

The cause is a composition order. `repeat_infinite()` wraps its input in rodio's `Buffered` and **clones that buffer for every pass after the first**, so anything inside it is polled exactly once no matter how long the sound loops. The analysis tap was inside. `append_decoded` now repeats first and taps second, which puts the tap outside the buffer where it sees every pass — and still before pan and sink volume, so the pre-volume semantics documented on `AudioLevels` are unchanged.

Found by giving `examples/games/beat_crawler` a real looping soundtrack instead of scheduled tones: the meter was live for one 2.56 s bar and then read `0.0000` for the rest of the run.

### Fixed
- **`AudioManager::append_decoded` applies `repeat_infinite` before the level tap, not after.** Metering a looped sound now works for its entire lifetime.
- **A fade-in on a looping sound no longer repeats on every pass.** Same root cause — `fade_in` was also baked into the buffer, so `crossfade_music` re-faded the track in at the top of every loop instead of once. Pan and fade now both sit outside the repeat.

### Notes
- Regression test `a_tap_outside_repeat_keeps_publishing_on_every_pass` (`src/audio/analysis.rs`) is device-free: it composes the same source chain and asserts the tap publishes once per pass, plus the contrast case (tapping inside repeat publishes only the first pass) as a tripwire if rodio ever stops buffering `Repeat`.
- **CI cannot catch this class of bug.** Verified on a real device: before the fix the meter read `0.0000` from 2.6 s onward across an 8 s run; after it, `rms` 0.08–0.38 sustained through every pass.

## 0.141.1

**`Audio` now survives a scene reset without the game asking.** `App::new` registers it as persistent, so inserting the resource is enough — the two examples that were hand-rolling `register_persistent::<Audio>()` (`settings_menu`, `beat_crawler`) no longer do. This closes a decision that had been deferred rather than taken, and whose stated reopen trigger fired in 0.139.1.

The deferral's argument was that `Audio` is inserted *by the game*, so persisting it would mean auto-persisting anything a game happens to insert. The 0.139.1 audit found seven config resources being silently dropped and **four of them are game-inserted** — so who inserts a type was never the distinction that mattered. The line that audit settled on is whether the *engine* defines the type; a type the game defines is still the game's job.

`Audio` is the one entry that line does not reach cleanly, because `AudioFacadeSystem` drives it every frame rather than reading it as config. It is registered anyway, because it owns an **OS output device handle** rather than a value — which `docs/PATTERNS.md` already classified as session state — and because its failure mode is the worst in the set: losing it does not revert a value, it takes the device, so audio dies with no error and an `Audio`-clocked game stops progressing entirely.

**No breaking change.** A game that still calls `register_persistent::<Audio>()` is unaffected — the call is idempotent.

### Fixed
- **`App::new` registers `Audio` as a persistent resource.** A `SceneCmd::Replace` no longer drops the audio device on games that did not know to ask.

### Changed
- **`examples/games/settings_menu` and `examples/games/beat_crawler` drop their hand-rolled `register_persistent::<Audio>()`.** `beat_crawler` is the example that motivated the change: its turn clock *is* `Audio::bands()`, so a dropped `Audio` did not merely mute it — the world stopped taking turns.
- **`docs/PATTERNS.md`'s "Surviving a scene reset" block is rewritten as decided rather than pending**, and records what would reopen it: a game that genuinely wants a per-scene device teardown, which would get an opt-out rather than a revert to the footgun.

### Notes
- The regression test (`audio_is_registered_to_survive_a_scene_reset`, `src/app.rs`) asserts on the **registration**, not on a surviving instance. `Audio::new()` opens a real output device and returns `None` where there is none, which is every CI runner; the preservation mechanism itself is already covered by the eight resource round-trip tests beside it. Confirmed to fail without the one-line fix.

## 0.141.0

**`examples/games/survivor` adopts `play_tone_metered`, which retires the workaround it was the reason for.** This is 0.140.0's acceptance test in the `docs/VISION.md` sense — the feature is not done until an example exercises it in real play — and it closes a loop that opened two releases ago: the game hit the constraint, the constraint became an API, the game now uses the API.

It also corrected a claim this example was itself making.

### Changed
- **The kill tone moves from `play_tone_on_channel` to `play_tone_metered`.** `KILL_CHANNEL` becomes `KILL_METER`, because it no longer names a channel.
- **`drive_from_amplitude` is re-based on the summed ceiling (`KILL_PEAK_FULL`), not `KILL_VOL_MAX`.** Required, not cosmetic — see below.

### Measured on a real device (300-frame headless run, scripted input)
- **22 of 25 kill-tone replays were cutting a tone that was still sounding.** The previous release reasoned that the kill tone "can afford" a named channel because it fires at most once per frame. Measured, it could not: kills land close enough together that a named-channel replay silenced the previous one 88% of the time.
- **The metered peak went from 0.6000 to 1.0000.** Before, every reading was pinned at or under the single-voice ceiling (0 frames above 0.60 in 301). After, 61 frames read above it, because `levels()` sums the sounding voices. That *is* the new API working, visible end-to-end on a real device rather than inferred from unit tests.
- **Which then broke the tuning, and had to be fixed.** `drive_from_amplitude` normalised against `KILL_VOL_MAX = 0.60` — one voice's ceiling — so with sums arriving, 61 of 197 sounding frames (31%) saturated the drive and sat at full shake. Re-basing on the summed ceiling brought that to 12 of 197 (6%), which puts a single kill mid-range and reserves the top for several kills landing together.

### Notes for anyone building on it
- **Adopting a polyphonic meter means re-checking anything that normalises against a single voice's maximum.** This is the generalizable lesson: the API change is additive and compiles fine, but any constant that assumed "the meter cannot exceed what one play produces" is now wrong. It shows up as a *feel* regression — a permanently saturated effect — not as an error.
- **The previous release's stated reason for leaving the bullet tone on the anonymous ring does not survive arithmetic.** It said the bullet tone "fired every 0.14 s, cannot" afford being cut — but the tone is 0.04 s long, so consecutive bullet tones never overlap and a named channel would never have cut one. The bullet tone stays on the ring, now for the honest reason: it has nothing to meter.

## 0.140.0

**`Audio::play_tone_metered` — a one-shot that overlaps itself *and* can be metered.** Until now a game had to pick one. `play_tone` / `play_tone_on_bus` ride a ring of anonymous voices, so consecutive plays overlap but there is no name for `enable_analysis` to address; `play_tone_on_channel` has a name, but opens with `stop_immediate` and **cuts** whatever that channel was already playing. A rapid-fire sound could be heard properly or measured properly, never both.

That was not a hunch — it was measured. Adopting the metering API in `examples/games/survivor` (0.139.0) hit the constraint, worked around it by leaving the bullet tone unmetered, and documented the workaround in three places. This closes it.

### Added
- **`Audio::play_tone_metered(meter, freq, dur, vol, bus)`** — plays a tone that overlaps its own previous plays, with `levels(meter)` reporting it. Native rotates a ring of 8 sink channels *private to that meter name*, so the sinks never collide, while pointing every voice's tap at the one meter entry. The channel model is untouched: those voices are ordinary one-sink channels.
- **`audio_analysis::sum_levels`** — the documented combination policy, in the un-gated shared module.

### Notes for anyone building on it
- **`levels()` reports the SUM of the sounding voices, clamped to full scale.** Three overlapping hits read louder than one, which is the point of metering them. The choice was not free: the web backend connects every voice to one `AnalyserNode` and Web Audio **mixes** multiple inputs, so a native `max` would have made the two platforms disagree about *meaning* rather than about rounding. Native sums per-voice measurements while the browser measures the summed signal — equivalent, not identical, the same caveat `bands()` carries.
- **Each voice needs its own publication slot.** Pointing several taps at one slot would race: `LevelSlot::publish` is a plain store, so a reader would see whichever voice published last — neither the sum nor the loudest.
- **Staleness is tested per voice, not per name.** Otherwise a voice that has drained keeps contributing its last level for as long as any *other* voice is still sounding, and the meter never falls between bursts.
- **The web backend needed no voice pool.** `play_tone_to` already builds a fresh oscillator per call, so wasm tones never cut each other; the only missing piece was routing each one into the analyser. The asymmetry is real and is why the *policy* lives in `audio_analysis` rather than in either backend.
- **Limits, on purpose.** The meter name is not a channel: `stop_channel` / `set_low_pass` / `is_channel_playing` do not address it, and `set_effect` does not apply — a sound you need to stop or filter wants a named channel. `bands()` reports zeros for a metered one-shot; a spectrum would cost an FFT per voice to serve an API whose use case is a soundtrack. Eight voices per name, then it wraps and reuses its oldest.
- **Still a per-sound decision.** Overlap costs a voice, and a sound that repeats slower than it decays gains nothing from this over a named channel.

### Fixed
- Three `docs/PATTERNS.md` and two `docs/MODULE_MAP.md` references dated the previous release's scene-reset audit **v0.140.0**; it shipped as **v0.139.1**.

## 0.139.1

**Seven more configuration resources were being silently reverted by a scene reset. `WindowConfig` (0.137.1) was not the only one — it was just the one found by accident.** That fix was prompted by a capstone game losing its clear color; nobody had then asked the obvious follow-up question, which is what *else* `insert_core_resources` inserts that a game sets once and expects to keep. This release is that audit, and its answer.

All 27 resources the engine inserts were classified against the session-state-vs-scene-state test in `docs/PATTERNS.md`, along with the engine-defined config types a game inserts itself. Seven were session config with no persistence; each is now registered, and each carries a regression test that was **confirmed to fail before the fix** rather than merely added alongside it.

The remaining 20 are correctly scene state. That negative result is written into `docs/PATTERNS.md` on purpose, so the audit does not get run a third time.

### Fixed
- **`FocusRingStyle`, `StickNavConfig`, `FrameConfig`** — engine-inserted config a game overrides once at setup (focus-ring theming, stick-navigation deadzones, the per-frame `max_dt` cap). Each was replaced by its engine default on the first scene enter.
- **`DesignResolution`, `WindowOptions`, `LightingConfig`, `DialogueStyle`** — engine-*defined* config the game inserts. `DesignResolution` is the consequential one: losing it does not revert a value, it silently switches letterboxing **off**, so a fixed-aspect layout starts stretching. `WindowOptions` silently made a deliberately non-resizable window resizable again.

### Notes for anyone building on it
- **Who inserts a resource turned out not to be the distinction that matters.** The v0.137.1 reasoning was "the engine inserts `WindowConfig`, so the engine should persist it" — but four of the seven above are inserted by the *game*, and they fail identically. The line this audit settled on is whether **the engine defines the type and only reads it** (no production path mutates one), whoever inserts it. Registering a type the engine never inserts is free: `reload_scene` skips a type it does not find.
- **`TimeScale` is deliberately excluded**, and it is the useful counter-example. It is config-shaped and engine-inserted, but games drive it moment-to-moment for hit-stop and slow-mo — a frozen or slowed *old* scene leaking into the next one is the worse bug. Resetting it per scene is correct.
- **This fires the reopen trigger on the deferred `Audio` auto-persistence decision**, which named "another engine-inserted config type turns out to be dropped the way `WindowConfig` was" as a condition. Seven did. `Audio` is deliberately left unchanged here — folding a behaviour change into the audit that fired its trigger would bury the question — but the argument holding it open is now thinner and is recorded in `docs/PATTERNS.md`.

## 0.139.0

**Audio-reactive game feel adopted in `examples/games/survivor` — the first consumer of `levels()` that was not written by the API's designer, which is the point.** `audio_reactive` and `beat_crawler` both shipped in the same sessions that built the metering API, so neither could tell us whether the API survives contact with older code. `survivor` is older, denser, and already used the `Audio` facade. Its kill tone now rides a named, metered channel whose volume follows a decaying kill combo, and the *measured* envelope — not a second hardcoded constant — sets the `Camera::shake` amplitude and a player pulse. There is one intensity knob, so the sound and the picture cannot drift apart.

The adoption is small (one example file), but it produced four findings that generalize, and a public-doc correction it exposed. **No public API change** — the only library edits are doc comments.

### Changed
- **`examples/games/survivor/survivor.rs`** — the kill tone moves from `play_tone_on_bus` to `play_tone_on_channel` on a named `"kill"` channel with `enable_analysis`; a new `AudioFeel` resource and `AudioFeelSystem` read `levels("kill").peak` each frame and drive `Camera::shake` (2.4–7.0 px measured) plus a player pulse (3.4–9.6 px measured). A HUD row reports the metered value and which feedback path is live. The rapid-fire bullet tone and the death tone are deliberately left on the anonymous ring.

### Fixed
- **`Audio::enable_analysis` documented only `play_sfx` as unmeterable**, but `play_sfx_on_bus`, `play_tone` and `play_tone_on_bus` all round-robin the *same* ring of 16 anonymous voices on native and are equally unmeterable. `play_tone_on_bus` did not mention the ring at all. Both docs now state it, and that moving a sound to a named channel is a trade-off rather than a free rename.

### Notes for anyone building on it
- **Meterability and overlap are mutually exclusive today.** Only a stable channel name can be metered, and a replay on a named channel *cuts* the sound already there, where the anonymous ring lets consecutive one-shots overlap. That is a per-sound decision, not a global one: `survivor` meters its kill tone and keeps its bullet tone (fired every 0.14 s) on the ring.
- **Arm/re-arm does not survive a continuous stream.** Detecting an event by latching a rising edge and re-arming below a lower threshold — the `beat_crawler` kick detector — fired **once in 300 frames** here, because under a stream of kills the metered envelope never falls back below the re-arm threshold; the screen went still exactly when the action was hottest. A retrigger *cooldown* fired 25 times over the same run. Arm/re-arm is for discrete events separated by silence; a sustained level needs a cooldown.
- **Metering only pays when the sound carries information the game does not already hold.** The first cut keyed the tone's volume to kills-*this-frame*. Measured, that was 1 kill in 40 of 40 kill frames — survivor fires one bullet per cooldown and a bullet kills at most one enemy — so the tone had exactly one amplitude (peak pinned at 0.230) and reading it back recovered a constant: a round-trip. Re-keyed to a decaying combo the same meter spans 0.23–0.60 (p50 0.33). The win comes from audio the game does **not** author, which is why `beat_crawler` meters a soundtrack no gameplay code can see.
- **A meter-driven effect needs a watchdog**, the same conclusion `BEAT_WATCHDOG` reached independently — so it generalizes rather than being specific to a turn clock. With no audio device (headless, muted, web before the first gesture) the meter never moves and the feel would silently vanish; after 0.6 s of scored-but-unheard kills `survivor` drives the feel from the combo directly and says so in the HUD. Verified by disabling `enable_analysis` and confirming the fallback engages.

## 0.138.0

**A second capstone game: `beat_crawler`, a dungeon crawler whose turn clock is the music itself.** Where `roguelike` composed two features (procgen + fog-of-war), this one exists to exercise the engine's recently grown surface *in real play* — the place `docs/VISION.md` expects API awkwardness to actually surface, and which single-purpose demos cannot reach.

**The composition is structural, not decorative.** The soundtrack plays a repeating 16-step pattern of 110 Hz **kicks** and 880 Hz **blips**; only the kicks are turns, and the game finds them by summing the low bands of `Audio::bands()`. An amplitude meter could not do this — the blips are just as loud, and each would fire a phantom turn — so the rhythm is discriminated by *frequency*, which is precisely the capability `bands()` added in 0.137.0 and which `levels()` does not have. The pattern constant is read only by the audio scheduler: no gameplay code sees it, so editing the groove changes the game's turn structure because the game learns the rhythm by listening.

Around that: `generate_bsp_dungeon` on odd depths and `generate_cellular_cave` on even (both produce a `DungeonMap`, so swapping generator is one line), a per-depth `Rng` seed, `FovMap` fog-of-war that also gates the AI so an unseen monster does not act, `find_path` enemy pathing on the beat, and `HitFlash` / `FloatingText` / `Camera::shake` / `ProgressBar` for feedback.

### Added
- **`examples/games/beat_crawler/`** (`cargo run --example beat_crawler_game`) — descend, move on the beat, bump monsters to attack, find the stair. Pressing in time with the beat doubles damage. `BEAT_CRAWLER_SELFTEST=1` runs a headless acceptance test; `ENGINE_CAPTURE` photographs it with no window.

### Notes for anyone building on it
- **Enemy and stair placement is BFS distance from the spawn, not the room list.** `generate_cellular_cave` records only a single 1×1 room, so room-based placement silently degenerates on even depths. Putting the stair at the farthest reachable cell also makes every level solvable by construction, for either generator.
- **A `bands()`-driven clock needs a watchdog.** Turns are keyed to wall-clock audio, so anything that silences output — a muted OS, a busy device, an `AudioContext` that never unlocks on the web — would otherwise leave the dungeon frozen with no explanation. After 1.2 s of scheduled-but-unheard kicks the game falls back to the schedule and says so in the HUD. This is a real robustness requirement of audio-driven gameplay, not a test affordance.
- **`HitFlash` owns `Sprite.color` while it runs.** The per-frame fog repaint therefore skips any entity currently carrying one; a game that also drove the color every frame would fight it.
- The game calls `register_persistent::<Audio>()` (precedent: `settings_menu`), since the `World` reset on the first scene enter would otherwise drop the audio device and with it the turn clock. `WindowConfig` no longer needs this as of 0.137.1.

## 0.137.1

**Fixes a bug that had been routed around twice instead of fixed: `WindowConfig` did not survive a scene change.** `App::set_scene` resets the `World`, and every resource that is not registered persistent goes with it — including `WindowConfig`, which a game inserts once during setup and never again. It was therefore silently replaced by `WindowConfig::default()` on the **first** scene enter, so any `Scene`-based game lost its `clear_color` (which is read per frame) and `save_screenshot_headless` / `ENGINE_CAPTURE` captured at the default 1280x720 rather than the game's own window size. 20 of the shipped examples set both `WindowConfig` and a scene, and were all affected.

The engine already knew: `src/app.rs` grew a wasm `WASM_LOGICAL_SIZE` thread-local explicitly "because a scene transition (`World` reset) can revert `WindowConfig` to its default", and `schedule.rs` notes the canvas size is "stable across scene resets, unlike `WindowConfig`". Both are workarounds for this; the cause is now fixed instead.

### Fixed
- **`WindowConfig` is auto-registered persistent in `App::new`**, so it survives the `World` reset on a scene change — the same treatment `TextMeasurer` already had. A game that deliberately varies `WindowConfig` per scene now keeps its change instead of having it reverted, which is the expected behavior in both directions. No API change; the wasm workarounds are left in place as they are still correct.
- Regression test `app::tests::window_config_survives_a_scene_reset` asserts size, title and clear color all survive `reload_scene` (it fails on the previous behavior).

## 0.137.0

**The second half of the audio-reactive story: `Audio::bands` reports a channel's frequency spectrum, so a game can build an actual analyzer display and not just a pulse.** `bands(channel, &mut out)` writes `out.len()` log-spaced bands from low frequency to high, each `0.0..=1.0` — **the caller chooses the band count**, which is what keeps each platform's very different FFT out of the API. Purely additive; `levels()` and everything from 0.136.0 are untouched.

The two backends are further apart here than anywhere else in the engine: wasm gets a transform for free from a Web Audio `AnalyserNode`, while native has none available at all — rodio does not analyze and no dependency in the tree provides an FFT. Rather than take on a DSP crate for one 1024-point transform, `src/audio/spectrum.rs` is a plain iterative radix-2 Cooley–Tukey implementation, in the same spirit as the hand-written SplitMix64 in `Rng` and the shadowcasting in `FovMap`. That is defensible because an FFT is one of the few pieces of DSP that can be checked *exactly* rather than by eyeball, and the tests do exactly that: energy lands in the bin matching a known sine, Parseval's theorem holds, DC lands in bin 0 alone.

**Comparability across the two is engineered rather than hoped for.** `MIN_DB`/`MAX_DB` are pinned to Web Audio's own `AnalyserNode` defaults (−100/−30 dB) and set explicitly on the node, so a browser changing its defaults cannot silently desync the platforms; and both backends fold FFT bins into bands through one shared `log_band_range`, so "band 7" covers the same frequencies on both. Values are nonetheless **equivalent, not bit-identical**, and the docs say so — drive visuals with them, not equality checks.

### Added
- **`Audio::enable_spectrum` / `disable_spectrum` / `bands`**, mirrored on `AudioManager` (native) and `WebAudio` (wasm). Spectrum is a **separate opt-in** from `enable_analysis`: it costs an FFT per window on native, while a pulse or a kick flash only needs `levels`. A levels-only channel runs no transform at all.
- **`src/audio/spectrum.rs`** — Hann window, in-place radix-2 FFT, and the log-spaced band fold. A non-power-of-two length is left untouched rather than transformed incorrectly, since a silently wrong spectrum is worse than none.
- **`log_band_range`, `normalized_db`, `resample_bands`, `SPECTRUM_BANDS`, `MIN_DB`, `MAX_DB`** in the shared `src/audio_analysis.rs` — the band spacing and decibel window both backends agree on.
- The `audio_reactive` example gains a 28-bar spectrum analyzer, and **both** its self-checks (native and the headless-Chrome one) now assert the spectrum's *shape* — low-half versus high-half energy for the 110 Hz kick — rather than merely that it is non-zero. That is deliberately tie-independent: at this transform size the tone saturates several of the lowest bands at once, so which one "wins" an argmax is an implementation detail.

### Notes
- Bands are spaced logarithmically and normalized over a decibel window because pitch and loudness are both perceived logarithmically; linear spacing or linear magnitude both produce a display that looks dead.
- Frequency resolution is bounded by the transform: 1024 points at 44.1 kHz is ~43 Hz per bin, so the lowest log-spaced bands cover only a bin or two and **move together**. That is the physics of the transform, not a display artifact, and asking for more bands does not manufacture detail that is not there. Documented on `bands()`.
- The native transform runs on the playback thread, over **mono-downmixed** frames. An FFT over raw interleaved stereo alternates left and right samples and does not describe the signal at all.

## 0.136.0

**Audio-reactive hooks: a channel's live loudness is now readable from game code, on native and on the web, through one call.** `Audio::levels(channel)` returns `AudioLevels { rms, peak }` — the input a music visualizer, a beat-reactive spawn or a mouth flap needs, and something the engine previously exposed no way to obtain at all. The two backends could hardly be less alike (a rodio `Source` tap on the playback thread vs a Web Audio `AnalyserNode`), so the meter's smoothing policy lives in one un-gated module both call, the same trick `audio_spatial` uses to keep positional audio from drifting between builds. Purely additive: no existing type or signature changed, and a channel without analysis enabled builds exactly the audio graph it did before.

Two deliberate semantics, both demonstrated by the example rather than only documented:

- **Measured pre-volume.** Levels are taken after a sound's own effects but *before* channel volume, bus volume, ducking and the master gain — so they describe *the sound*, not *what the player hears*, and a beat-reactive visual keeps working when the player mutes. On native this falls out structurally: volume lives on the rodio sink (`sink.set_volume`), not in the source chain, so any tap in that chain is pre-volume by construction.
- **Instant attack, timed release.** A meter rises the frame a transient lands (so a hit never looks late) and falls over `DEFAULT_ANALYSIS_SMOOTHING` (0.15 s), which is what keeps it from strobing at frame rate.

### Added
- **`AudioLevels { rms, peak }`** and **`DEFAULT_ANALYSIS_SMOOTHING`** in the new un-gated `src/audio_analysis.rs`, alongside the shared `smooth_toward` meter policy and `MUSIC_CHANNEL` (one definition of the facade's music-channel name, because both backends need the same string).
- **`Audio::enable_analysis` / `disable_analysis` / `is_analysis_enabled` / `levels` / `set_analysis_smoothing` / `analysis_smoothing`**, plus **`Audio::MUSIC_CHANNEL`** for metering `play_music`. The same surface exists on `AudioManager` (native) and `WebAudio` (wasm) for games using a backend directly.
- **Native** (`src/audio/analysis.rs`): `LevelTap<S: Source>`, a pass-through source wrapper modelled on the existing `PannedSource`, publishing RMS and peak per 1024-sample window into a lock-free `LevelSlot` of atomics — the tap runs on rodio's playback thread, where taking a lock is not an option. A monotonic sequence counter lets `update` distinguish "still producing" from "stopped", so a channel that ends **decays to silence** instead of freezing its meter at the last value it saw.
- **WASM** (`src/audio_wasm.rs`): a parallel-tapped `AnalyserNode` per analyzed channel, sampled with `get_float_time_domain_data`. No staleness counter is needed there — an analyser reads the live graph, so silence decays on its own. Enables the `AnalyserNode` web-sys feature.
- Example **`audio_reactive`** (native + web, same code): an rms-driven pulse, a peak-driven kick flash and two meters. `M` mutes the master and the pulse keeps going — the pre-volume decision, made visible; `S` cycles the release time to show why the default is not 0. Ships with `examples/audio_reactive/web/` and **`scripts/audio_reactive_smoke.sh`**, which asserts in headless Chrome that the meter actually moves on the web — the wasm half shares almost no code with the native path, so compiling it proves very little.

### Changed
- **`Audio::update` is no longer a no-op on wasm.** It now samples the level meters (Web Audio still drives volumes, ramps and ducks itself). `AudioFacadeSystem` already called it every frame, so no game code changes; a page that never enables analysis is unaffected.

## 0.135.2

**Every bundled shell script is executable again — 26 of the repo's 31 were committed without the executable bit, so running them the way their own docs say fails on a fresh clone.** Each script documents its usage as `scripts/foo.sh …`, which needs the bit; without it the shell answers "Permission denied". Three web render smokes were broken twice over, because they also execute the example's `web/build.sh` directly. Tooling only — no source, no API, no runtime behavior changed.

### Fixed
- `git update-index --chmod=+x` on all 16 `scripts/*.sh` and all 15 bundled `examples/**/build.sh` (31 files; only `verify.sh`, `wasm_audio_smoke.sh`, `wasm_save_smoke.sh` and two `build.sh` had the bit). This had gone unnoticed because the repo sets `core.fileMode = false`, so a local `chmod +x` makes a script work for the person who ran it while git records nothing — which is also how the 0.135.1 `embedded_atlas` smoke and its `build.sh` shipped without the bit.
- Directly affected: `scripts/centered_text_smoke.sh`, `scripts/game_feel_web_smoke.sh`, `scripts/hdr_web_smoke.sh` and `scripts/embedded_atlas_smoke.sh` now run instead of dying before doing any work. (`bloom_web_smoke.sh` and `render_format_query_smoke.sh` invoke their `build.sh` via `bash …` and were unaffected.)

## 0.135.1

**The byte-source asset examples now actually run on the web, instead of merely compiling for it.** 0.135.0 claimed `load_atlas_bytes` works on wasm and verified only that the example *builds* for `wasm32` — which would not have caught a sheet that decoded but never reached the GPU. `embedded_atlas` now ships a browser harness and a render smoke that proves the claim end to end, and `embedded_image` — which had been silently unbuildable for wasm since it was added — builds again. No library code changed; this is examples and tooling only, so no public API is affected.

### Fixed
- `examples/embedded_image.rs` failed `cargo build --example embedded_image --target wasm32-unknown-unknown` (E0599): it called the native-only `save_screenshot_headless` unconditionally. The headless acceptance test moved into a `#[cfg(not(target_arch = "wasm32"))]` function, exactly as `embedded_atlas` does, so the example builds for both targets with the native check unchanged. The verify gate never caught this because its wasm step builds lib+bins only, never examples.

### Added
- `examples/embedded_atlas/web/` — a wasm-bindgen harness (`build.sh` + `index.html`) for the embedded-atlas demo. The example moved to `examples/embedded_atlas/embedded_atlas.rs` with a `[[example]]` entry, matching every other web-shipped example, and gained a `#[wasm_bindgen]` entry point beside a shared `build_app()` so the browser runs exactly what `cargo run` does.
- `scripts/embedded_atlas_smoke.sh` — a headless-Chrome render smoke (optional local check; CI has no Chrome/GPU). It asserts **both** halves of the feature's claim: that no image file is served beside the page, *and* that the rendered frame is non-blank. Either alone is weak — a non-blank frame could have come from a fetch, and an empty directory proves nothing if nothing drew — but together they say the atlas rendered and cannot have come from a file. Verified in a browser: all 12 tiles render from the embedded sheet with no image request. — `load_atlas_bytes` is the atlas half of the `include_bytes!` story that `load_image_bytes` started.** Every atlas API was path-based, so a jam entry, a single-file distribution or a wasm demo could embed a lone image but had to ship its *gridded* art as a file beside the executable. `App::load_atlas_bytes(key, bytes, cols, rows)` takes a `&[u8]` the caller already holds and registers it as a `TextureAtlas` with the same uniform grid `load_atlas` produces — no filesystem read, on any target. The `key` is used **verbatim** on all three sides (atlas cache key, `Handle::path()`, and the renderer's texture key, because the underlying image is registered through `load_image_bytes` under the same string), which is the identity invariant that keeps a byte-sourced atlas from rendering white. Purely additive: no existing type or signature changed.

### Added
- `AssetServer::load_atlas_bytes(key, bytes, cols, rows)` (`src/asset/atlas_loading.rs`) and its `App::load_atlas_bytes` twin (`src/app/assets.rs`) — the byte-source counterpart of `load_atlas`. An `AtlasSprite` built on the returned handle renders exactly like a path-loaded atlas; `uv_rect` is the shared grid maths, so tiling cannot diverge between the two sources. Loading the same `key` again returns the cached handle (the bytes are ignored on a hit).
- The `key` is a **logical identifier, not a path**: never resolved against the asset root and never canonicalized, so it needs no file on disk and cannot collide with one. Like `load_image_bytes`, nothing is pushed to `pending_textures` — the decoded sheet reaches the GPU through the same per-frame `upload_asset_server_images_to_gpu` seam that async loads use.
- Cross-platform including wasm, where `include_bytes!` sidesteps the async fetch a path load needs — which is the single-file build this exists for.
- A corrupt embedded sheet is reported through `App::asset_failures()` exactly like a missing file (the EW-007 rule), and the atlas still registers over the magenta fallback so the grid API stays usable rather than panicking.
- Example `examples/embedded_atlas.rs` — renders all 12 tiles of an embedded 4×3, 64px-cell sheet (the same sheet `blend_locomotion` loads *by path*), each labelled with its atlas index, alongside a deliberately corrupt embed shown in the failures panel. Its headless pass asserts the sheet decoded to 256×192 under the verbatim key, that the grid tiles as a path-loaded atlas would, and that the corrupt embed was reported, exiting non-zero otherwise. Unlike `embedded_image`, this example **builds for `wasm32-unknown-unknown`** — its headless self-check is `cfg`-gated, since `save_screenshot_headless` is native-only.
- 3 unit tests in `src/asset/tests.rs`: the verbatim-key identity across the atlas handle, `texture_path()` and the renderer's upload key (`image_assets_for_gpu`); an A/B against `load_atlas` on a real temp file asserting identical UVs for all 12 tiles; and a corrupt embed surfacing in `asset_failures()`.

## 0.134.0

**A game can now drive itself through its own screens and photograph them — no window, no display, no OS automation permissions.** Verifying that a screen still looks right normally needs a live, unlocked desktop plus OS automation (on macOS: Accessibility + Screen-Recording permissions, `osascript` key codes, a synthetic-mouse helper); none of that runs on CI, so the usual fallback is a boot smoke test that proves the app *starts* but is blind to a wrong z-order, a missing icon or a misplaced panel. `InputScript` replaces the human by injecting `(frame, action)` events into the **same** `InputState` the window feeds, and `App::capture_frames_headless` writes a PNG at each chosen frame. Both are reachable from environment variables that `App::run` reads, so **an existing game needs no code change at all** — verified by driving the unrelated `maze_generation` example through its `B` and `R` keys and capturing the result. Purely additive: no existing type or signature changed. Closes dungeon-merchant **EW-011**.

### Added
- `InputScript` (`src/input_script.rs`, re-exported from `src/lib.rs`) — a frame-indexed list of `InputAction`s played into `InputState` one frame at a time. Applied at the **start** of `App::update`, before the game's systems and after the previous frame's `flush`, so a scripted press reads as `just_pressed` within its own frame exactly as a real one does. `InputAction` covers `KeyDown`/`KeyUp`/`KeyPress` (a tap, released on the following frame so `just_released` behaves), `MouseMove`, `MouseDown`/`MouseUp`/`Click`, `Scroll` and `Quit`. Constructors `InputScript::new` (in code), `from_ron_str` and `load` (asset-root-resolved), with `frame` / `len` / `last_frame` / `is_finished` accessors.
- RON scripts name keys by their `winit::keyboard::KeyCode` variant (`"KeyA"`, `"Digit2"`, `"Space"`, `"ArrowUp"`, `"F5"`, `"Numpad7"`, …) through the new `key_from_name` / `key_names` helpers, and buttons through `mouse_button_from_name`. An unknown name is a load **error** rather than a silently dropped event — a typo in a verification script would otherwise look exactly like a failing feature. Because winit ships no serde support, the file shape is carried by private mirror types and resolved into the runtime enum, matching the particle/trigger-zone config sets.
- `App::capture_frames_headless(&[(frame, path)])` (`src/app/headless.rs`) — runs headlessly and writes a PNG at **each** listed frame, so one pass can photograph several screens. Reuses the existing offscreen `screenshot_headless` render path.
- `App::set_input_script`, which also registers the script persistent so a scene change cannot cancel a run in progress.
- Environment entry points read by `App::run` on native: `ENGINE_INPUT=<script.ron>` plays a script, and `ENGINE_CAPTURE=<frame>:<path>[,<frame>:<path>…]` runs headlessly, writes each PNG and returns **instead of** opening a window. A malformed `ENGINE_CAPTURE` entry is reported and skipped rather than ignored.
- Example `examples/scripted_capture.rs` + `examples/scripted_capture.ron` — a three-screen toy shop (Menu → Shop grid → item Detail) driven entirely by the bundled script. Its headless pass captures one PNG per screen and asserts the scripted keys and click actually produced the transitions, exiting non-zero otherwise.
- 13 unit tests in `src/input_script.rs` (press reads as `just_pressed` in its own frame and `just_released` in the next, held keys, click position feeding the hit-test, scroll/quit reaching their resources, frame sorting with stable same-frame order, a passed frame not being dropped, RON parsing, unknown-name errors, key/button name resolution including every advertised name, and `ENGINE_CAPTURE` parsing incl. a Windows drive letter and malformed entries).

### Changed (internal)
- `App::save_screenshot_headless` and the new capture path share a `save_rgba_png` helper.

## 0.133.0

**A `DataTable`'s column schema is now the union of every row's keys, so a column that only some rows carry is no longer silently discarded.** `DataTable::parse` derived `columns` from row 0 alone: a column that first appeared on a later row was dropped with only a `log::warn!` — invisible in a normal run, while the symptom ("the feature just doesn't apply to some rows") surfaced far from the cause. That made every optional column carry an implicit "MUST also be present on row 0" rule, enforced by header comments in the RON files and re-learned by each new contributor. The schema is now collected across all rows (still sorted alphabetically, so a row-0-complete table parses exactly as before) and the fill mechanism is unchanged: a row that omits a column gets `ron::Value::Unit`, as rows missing a row-0 column always did. Nothing can be discarded anymore, so the extra-column warning is retired. Closes dungeon-merchant **EW-010**.

### Fixed
- `DataTable::parse` (`src/data_table.rs`) collects `columns` from the **union** of every row's keys instead of row 0's alone, so a late-appearing column joins the schema and keeps its authored values. Rows that omit it are filled with `ron::Value::Unit`, exactly as before. A table whose row 0 already lists every column parses identically to the previous behavior.
- The `data_table: row {idx} has extra column '{key}' not in schema; value discarded` warning is removed — with a union schema no row can carry a column outside it, so the condition is unreachable.
- `DataTable::add_row` seeds each new cell from the first row that actually carries a value for that column, rather than from row 0. With a union schema an optional column is `Unit` on every row that omits it, so reading only row 0 would type a late-appearing column as `Unit` and seed the new row with nothing.

### Added
- 3 unit tests in `src/data_table.rs`: a column present only on the last row joins the schema and keeps its value while the other rows hold `Unit`; a row-0-complete table parses unchanged with no `Unit` cells; `add_row` types a late-appearing column from the row that has it.

## 0.132.0

**Text can now be measured before it is drawn, so a panel can be sized to fit it exactly.** Text-fitted UI — a tooltip, a name chip, a speech bubble — has to be as wide as its string, but the width is only known after shaping, so games fall back to a `chars × px` guess. That guess breaks the moment scripts mix: at 15 px a Hangul glyph advances ≈ 15 px while a digit advances ≈ 8 px, so `"사슬갑옷"` and `"200g"` — both four characters — differ by nearly 2×; the guess then gets padded until it stops clipping and every panel is loose forever after. `TextMeasurer` shapes a string through the renderer's **own** code path and reports the real extent. Exactness is structural rather than coincidental: the GPU renderer and the measurer now shape through one extracted `shape_text(font_system, &ShapeSpec)` helper (same `Metrics`, `Shaping::Advanced`, attrs and wrap mode) and load one font stack through the shared `font_blobs(world)` (`FontData` + `ExtraFonts` + the wasm default-font fallback), so a measurement cannot drift from what is rendered. Purely additive: no existing type or signature changed. Closes dungeon-merchant **EW-009**.

### Added
- `TextMeasurer` (`src/text_measure.rs`, re-exported from `src/lib.rs`) — `measure(text, size)` (single line), `measure_rich(text, size)` (markup parsed away first, so the tags are not counted as text), `measure_wrapped(text, size, max_width)` (returns the **longest wrapped line**, so a panel is tight rather than always `max_width` wide; non-positive width measures unwrapped), `line_height(size)` (`= size × 1.2`), and `TextMeasurer::new(font_data, extra_fonts)` for inserting the resource during setup. Empty text measures `Vec2::ZERO`.
- World-level `measure_text(world, text, size)` / `measure_text_wrapped(world, text, size, max_width)` / `text_measurer(world)` — callable from inside a `System::run`, since they borrow only the `World` and never the renderer — plus the `App::measure_text` / `App::measure_text_wrapped` twins for setup code. Results are in `DrawText`'s logical pixels (pre-`DisplayScaleFactor`/`Letterbox`), so a measured width is directly usable as a `DrawRect` width.
- Example `examples/text_measure.rs` — shop tooltips drawn twice, sized by the `chars × 15 + 22` heuristic and by `measure_text`, with a tick marking the true text edge so the heuristic's error is visible (amber when loose, red when it clips); ↑↓ changes the font size to show a heuristic tuned at one size going wrong, ←→ drives `measure_text_wrapped`. Under `HEADLESS_SHOT` it self-checks that measurement is script-aware, that it disagrees with the heuristic, that the heuristic clips at a larger size, and that a wrapped measurement stays inside its bound — exiting non-zero on failure.
- 11 unit tests in `src/text_measure.rs` (deterministic ASCII width, empty-is-zero, monotonicity, same-char-count strings measuring differently, width scaling with font size, newline adding a line of height, wrap fitting its bound and growing taller, non-positive wrap width, rich markup not counted, the shared line-height factor, lazy resource creation).

### Changed (internal)
- Buffer shaping extracted from `TextRenderer::render`'s buffer-construction closure into `shape_text(&mut FontSystem, &ShapeSpec)` (`src/renderer/text/renderer.rs`), now the single shaping entry point for the engine; the renderer calls it with exactly the arguments it used inline before, so rendering is byte-identical. The `1.2` line-height multiplier became the shared `LINE_HEIGHT_FACTOR` constant, used by the metrics, the `TextAnchor::Center` offset and `TextMeasurer::line_height`.
- The renderer's font-blob collection (`FontData` + `ExtraFonts` + the wasm `DEFAULT_FONT` fallback) moved out of `App::init_gpu_renderers` into `text_measure::font_blobs(world)` and is shared with the measurer's lazy construction.
- `TextMeasurer` is auto-`register_persistent` in `App::new`, so a scene change never rebuilds its `FontSystem` (registration is free when the resource is never created).

## 0.131.0

**A third procedural-map generator — perfect mazes — alongside the BSP dungeon and the cellular cave.** `generate_maze(width, height, seed, &MazeParams)` carves a perfect maze: a recursive-backtracker (depth-first, explicit stack) walk over the odd-coordinate *junction* cells knocks out the wall between each junction and a random unvisited neighbor, building a **spanning tree** over the junction graph — so the maze is **guaranteed connected by construction** (no keep-largest pass) and, for `braid_chance == 0.0`, exactly one path joins any two cells. `MazeParams { braid_chance }` optionally *braids* the maze afterward, reopening a fraction of dead-end walls into loops; since braiding only removes walls, connectivity holds at any value. Deterministic (same seed + params → identical map), seeded by the shared `engine::Rng`, and returns the *same* `DungeonMap` type as the other two generators, so `to_path_grid` / `to_tilemap_tiles` / `FovMap::from_path_grid` compose with any of them. Purely additive: no existing type or signature changed.

### Added
- `generate_maze(width: i32, height: i32, seed: u64, &MazeParams) -> DungeonMap` and `MazeParams { braid_chance }` (`src/mapgen.rs`, re-exported from `src/lib.rs`). Recursive backtracker over the junction graph with an explicit stack (a maze can nest as deep as every cell), building a spanning tree; records a single 1×1 `Room` at the start junction `(1, 1)` so `first_room_center` is a valid spawn. Optional `braid_dead_ends` post-pass reopens dead-end walls into loops (`braid_chance`, clamped to `0.0..=1.0`). Cell count capped at `MAX_PATH_GRID_CELLS`; degenerate/oversized dimensions collapse to an empty map, mirroring the other generators.
- Example `examples/maze_generation.rs` — renders a perfect maze (dead-end tips tinted warm to show the tree's leaves), WASD/arrows to walk (walls block), R to regenerate from the next seed, B to toggle braiding (perfect ↔ braided, same seed). Under `HEADLESS_SHOT` it self-checks single-region connectivity (flood-fill from spawn reaches every floor cell) before capturing the screenshot, exiting non-zero on failure.
- 9 unit tests in `src/mapgen.rs` (determinism, border-is-wall, spawn-is-the-start-junction, single-connected-region across seeds, the exact perfect-maze `2·jw·jh − 1` spanning-tree floor count, braided-stays-connected-with-more-floor, `to_path_grid`/`FovMap` composition, degenerate sizes safe) plus a maze clause in the module doctest.

## 0.130.0

**A second procedural-map generator — cellular-automata caves — alongside the BSP dungeon.** `generate_cellular_cave(width, height, seed, &CaveParams)` grows an organic cavern: it seeds the interior with random rock, runs a few cellular-automata smoothing passes (the classic 4-5 birth/survival rule, where an existing wall survives with one fewer neighbor than a new one needs), then keeps only the largest connected cavern — filling every smaller pocket — so, like the BSP dungeon, the result is **guaranteed connected**. It is deterministic (same seed + params → identical map), seeded by the shared `engine::Rng`, and returns the *same* `DungeonMap` type as `generate_bsp_dungeon`, so `to_path_grid` / `to_tilemap_tiles` / `FovMap::from_path_grid` compose with either generator. Purely additive: no existing type or signature changed.

### Added
- `generate_cellular_cave(width: i32, height: i32, seed: u64, &CaveParams) -> DungeonMap` and `CaveParams { initial_wall_prob, steps, wall_threshold }` (`src/mapgen.rs`, re-exported from `src/lib.rs`). Random-rock fill → CA smoothing (`cave_smooth`, the hysteretic 4-5 rule) → `keep_largest_cavern` (4-connectivity flood fill, keeps the biggest region, fills the rest with wall, and records a single 1×1 `Room` at the cavern's most-central cell so `first_room_center` is a valid spawn). Cell count capped at `MAX_PATH_GRID_CELLS`; degenerate/oversized dimensions collapse to an empty map, mirroring `generate_bsp_dungeon` / `PathGrid` / `FovMap`.
- Example `examples/cave_generation.rs` — renders an organic cave (floor cells touching rock tinted darker to accent the cavern outline), WASD/arrows to walk (rock blocks), R to regenerate a fresh connected cave from the next seed. Under `HEADLESS_SHOT` it self-checks single-cavern connectivity (flood-fill from spawn reaches every floor cell) before capturing the screenshot, exiting non-zero on failure.
- 8 unit tests in `src/mapgen.rs` (determinism, border-is-wall, single-connected-cavern across seeds, central-floor spawn, `to_path_grid`/`FovMap` composition, `steps: 0` still connected, degenerate sizes safe) plus a cave clause in the module doctest.

## 0.129.1

**The enabled audio codecs (`wav` / `vorbis` / `mp3`) now have decode-time test coverage.** After the rodio 0.19 → 0.22 swap moved decoding onto symphonia, nothing in the engine exercised those features end to end — an `.ogg`/vorbis stream in particular was decoded by nothing, so a dropped feature or a symphonia regression could have shipped silently. This adds three committed, synthesized (public-domain / CC0) fixtures and a decode test that pins each codec. Decoding needs no audio output device, so the test runs on CI's headless machine, unlike the playback path. No public API change; test-only.

### Added
- `codec_decode_tests` in `src/audio/playback.rs` — decodes `src/audio/fixtures/{tone.wav,tone.ogg,tone.mp3}` through `rodio::Decoder` and asserts each yields the expected sine (sample rate, channel count, non-empty stream). `Decoder::new` only recognizes a container whose codec feature is compiled in, so a red test means the engine lost a codec it promises. Proven non-vacuous: removing the `vorbis` feature reds the `.ogg` test.
- Test fixtures `src/audio/fixtures/{tone.wav,tone.ogg,tone.mp3}` + `README.md` — the same ~0.15 s, 22 050 Hz mono, 440 Hz sine synthesized from scratch (no third-party audio, so CC0-safe to commit), encoded to each codec. The README documents provenance and a regeneration recipe.

## 0.129.0

**Hot-reload now fires for an asset whose file is resolved under an asset root that is not the working directory — the packaged / foreign-cwd layout.** Since v0.126.0 a relative asset path is read via `asset_path::resolve` (searching the executable's directory and ancestors, not just the cwd), but the `notify` filesystem watcher still registered the caller's *logical* path. In a packaged build that path does not exist relative to the launch directory, and `notify` cannot watch a nonexistent path — so the watch silently failed and an edit never triggered a reload. This was the one EW-008 acceptance clause the engine did not meet (disclosed to the downstream game rather than silently claimed). The watcher now watches `resolve(logical)` (the file that actually exists) and translates each event back to the logical dispatch key before dispatch, so images re-decode and RON registries reload by the key they stored. Asset **identity is untouched** — cache keys and `Handle::path()` stay logical, per the same constraint that prevents the 2026-05-29 white-sprite cache-key bug. Dev-from-repo-root behavior is byte-identical (the resolved→logical map is an identity there). No public API change.

### Fixed
- Hot-reload under an asset root: `AssetServer::watch_path` and the image watch site now register the `notify` watcher on `asset_path::resolve(logical)` instead of the logical path verbatim, and record an internal `asset_key(resolve(logical)) → asset_key(logical)` reverse map. `AssetServer::poll_reloads` translates each changed path back to the logical key (new `pub(super)` `logical_for_changed`, falling back to today's `asset_key(path)` when unmapped) before the `is_known` membership check, so a change dispatches under the key the image cache / `watched_paths` / RON registries actually stored. Native-only (hot-reload is native-only); `#[cfg(not(wasm32))]` throughout.

### Added
- Example `hot_reload_asset_root` + `scripts/hot_reload_smoke.sh` — a headless proof (no GPU) that edits a data table under a pinned asset root from a foreign working directory and asserts the reload fires, driving `AssetServer::poll_reloads` + `DataTableRegistry::reload_path` exactly as `src/app/schedule.rs` does. Exercises the real `notify` round-trip; the deterministic regression guards are two new unit tests in `src/asset/tests.rs` (the non-identity resolved→logical translation, and that `watch_path` registers the reverse-map entry — both proven to fail without the fix).

## 0.128.0

**Images can now be loaded from bytes already in memory, with no file on disk.** Every other image API is path-based (`App::load_image("assets/hero.png")`), which is right for a game that ships an `assets/` folder — but a small/jam game or a wasm demo often wants to ship as **one file** with its art baked in via `include_bytes!`. `App::load_image_bytes(key, bytes)` decodes an in-memory image immediately and registers it under a **verbatim logical `key`** — used identically as the texture-cache key, the `Handle::path()`, and a `Sprite::textured(key)` render lookup, so it never rewrites an asset's identity (the invariant that the 2026-05-29 white-sprite cache-key bug taught). It is cross-platform, including wasm, where `include_bytes!` sidesteps the async fetch a path load needs. Fully additive: no existing signature changed.

### Added
- `App::load_image_bytes(key, bytes)` and `AssetServer::load_image_bytes(key, bytes)` — register an already-in-memory image (an `include_bytes!` asset or a runtime-generated one) under `key`, returning a `Handle<ImageAsset>` that renders via `Sprite::with_handle` / `Sprite::textured(key)` exactly like a path-loaded image. The `key` is a logical identifier, never resolved against the asset root, and is stored **verbatim** (no `asset_key`/canonicalize) so the cache key, handle path, and render key provably agree. No filesystem read and no `pending_textures` entry — the decoded image rides the same per-frame GPU-upload path as async loads. A corrupt-bytes decode failure is reported through `asset_path::asset_failures()` (a magenta 1×1 fallback is stored), exactly like a missing file. Backed by `decode_bytes_with_state` in `src/asset/image_loading.rs`; cross-platform (the `image` crate is available on every target).
- Example `embedded_image` — renders a 32×32 PNG embedded via `include_bytes!` with no external asset file (runs correctly from any working directory), and demonstrates that a deliberately corrupt embed is still reported by `asset_failures()`. Under `HEADLESS_SHOT` it is a self-checking acceptance test (the embed must decode to 32×32 and not be a failure; the corrupt embed must be reported).

## 0.127.0

**A data table that fails to load is no longer silent.** 0.126.0 taught the engine *where* to look for a relative asset; this closes the other half — what happens when it still isn't there. Only the image loaders reported failures; every other path-based loader `warn!`d and registered nothing, which is indistinguishable from success for a caller: a downstream game shipped a packaged Windows build whose 19 data tables all resolved to nothing, so every table registered **empty** and the game booted "correctly" with an empty shop and no dungeons. It took a player's bug report to find, because the symptom (empty content) surfaces screens away from the cause (the loader). Every path-based loader now reports through the one channel `asset_failures()`. Additive: no signature changed, and a load that succeeds behaves exactly as before.

### Changed
- `App::load_data_table` / `load_animation_clips` / `load_particle_configs` / `load_dialogue` / `load_trigger_zones` / `load_zone_effects` / `load_anim_effects` now report a failed load (missing file, RON parse error) as an **asset failure**: an `error!` naming the file, the reason, **and the roots searched**, plus an entry in `asset_path::asset_failures()` / `App::asset_failures()`. Under `set_strict_assets(true)` the load panics instead. Previously each logged a `warn!` and returned, leaving the registry silently without the table. The failure message names the registry key too (`data table 'items': …`), since that is what the game's own code will look up.
- Rhai script loading reports both a failed **read** and a failed **compile** the same way (a script that won't compile silently became an empty AST), and `AudioManager`'s file read reports a missing clip instead of `warn!`ing.
- A *hot-reload* failure deliberately stays a `warn!` — the file loaded once, and a half-saved edit should not kill a strict dev build. An intentionally empty asset that parses fine is likewise not a failure (test-pinned).

### Added
- Example `packaged_assets` now loads a real **and** a deliberately missing data table beside its textures, so the panel shows the failure that used to be invisible. In `HEADLESS_SHOT` mode it is a real acceptance test: it exits non-zero unless the real texture and table resolve, the table carries rows, **and** the missing table is reported.
- `scripts/packaged_assets_smoke.sh` — builds that example and runs it from `/`, which is what a shipped build's working directory looks like (running it from the checkout is the case that always worked, so it proves nothing).
- Two unit tests in `src/app/editor/loading.rs`: a missing data table is recorded and does not leave an empty table behind; an intentionally empty (but valid) table registers with zero rows and is **not** a failure. Both key on a path unique to the test — `asset_failures()` is process-global, so never assert on its length.

## 0.126.0

**Asset roots: a packaged build no longer renders a magenta screen when launched from the "wrong" directory.** Every asset API is path-based, and a relative path was read straight through `std::fs` — which resolves against the **process working directory**. A shipped executable therefore only worked when launched from one specific place: double-clicked in a file manager, started from a shortcut, or run as `cd /elsewhere && game.exe`, *every* texture load failed, the renderer substituted its magenta 1x1 fallback, and — since a title screen is usually one full-screen textured quad — the whole window turned magenta, explained by nothing but a `warn!`. The engine now resolves relative paths against an asset root it determines itself, and a failed load is *reported* instead of swallowed. Additive: a new module plus re-exports; no existing example needed a change.

### Added
- `engine::asset_path` (`src/asset_path.rs`) — `resolve(path)` maps a relative asset path onto an engine-determined root. Candidates, in order: a macOS bundle's `Contents/Resources`, the executable's directory and its ancestors (`ANCESTOR_DEPTH` = 4), then the working directory and its ancestors; the first candidate under which the file actually **exists** wins. Absolute paths pass through unchanged, and wasm (no filesystem — paths are URLs) resolves to identity. Executable-derived candidates precede the working directory so a shipped build never silently picks up a stray `assets/` from wherever it was launched.
- `asset_path::set_asset_root(root)` / `App::set_asset_root` — pin one explicit root, which then becomes the only candidate.
- `asset_path::asset_failures()` / `App::asset_failures()` → `Vec<AssetFailure { path, error }>`, and `asset_path::set_strict_assets(bool)` / `App::set_strict_assets` to panic at the load instead of falling back. A failed load is now an `error!` (was a `warn!`) that names **the roots the engine searched**, because "file not found" is no help when the question is *where it looked*.
- Example `packaged_assets` — loads a texture by a relative path and shows the roots searched plus the live `asset_failures()` panel (it deliberately loads one missing texture). Run the built binary from any directory and the sprite still renders.
- `tests/asset_root.rs` — the regression test: it moves the working directory somewhere unrelated and asserts a relative asset still loads. It fails on the old behavior. It is deliberately the **only** test in that file, since the working directory is process-global and cargo parallelizes tests within a binary (each integration-test *file* is its own process).

### Changed
- The ~10 filesystem read sites now resolve through `asset_path::resolve`: `renderer/texture.rs`, `asset/image_loading.rs`, `audio/playback.rs`, `scripting/loading.rs`, and the RON loaders (`data_table`, `ron_registry`, `animation/clip_set`, `dialogue/tree`, `particle/config_set`, `trigger_zone`, `zone_effect`, `anim_effect`).
- **Resolution is applied only at the read, never to an asset's identity.** Cache keys and `Handle::path()` remain exactly the string the caller passed. Rewriting them to the resolved path would reintroduce the bug fixed in 0.11.x-era "unify image texture cache keys" — a handle keyed canonically while the GPU texture cache is keyed relatively, so every sprite silently renders white.

## 0.125.0

**Fixes the Windows/DX12 build: `rodio` 0.19 → 0.22, which drops the stale `windows` crate out of the dependency graph.** `rodio 0.19` pulled `cpal 0.15.3`, which pins `windows 0.54`. `gpu-allocator` accepts a wide `windows` range (`>=0.53, <=0.62`) and will happily reuse whatever node is already in the graph — so it could resolve onto 0.54 and then hand `wgpu-hal 29` (which is on `windows 0.62`) D3D12 types from the wrong crate version, failing the DX12 backend with a wall of type mismatches. This is invisible on macOS/Linux and only bites when someone packages a Windows release. Bumping `rodio` removes `windows 0.54` entirely, leaving exactly one `windows` version. **No public API change** — `AudioManager`, the `Audio` facade, and the wasm `WebAudio` backend keep their signatures.

### Fixed
- `rodio` 0.19 → 0.22.2 (`Cargo.toml`). `cpal` 0.15.3 → 0.17.3, which drops `windows 0.54`; the MSVC target now resolves a single `windows` version (0.62.2), so `gpu-allocator` and `wgpu-hal` agree and the DX12 backend compiles.
- The engine's codec policy (`wav`/`vorbis`/`mp3`, owned engine-side since 0.11.x) is preserved across the bump — those features are now symphonia-backed, and `playback` is enabled explicitly since `rodio` moved `cpal` behind it.

### Added
- CI job **Build (Windows / DX12)** (`.github/workflows/ci.yml`) — the other jobs are ubuntu/wasm only, so nothing ever compiled the DX12 backend, which is precisely why this conflict could hide. The job builds on `windows-latest` and asserts the real invariant directly: exactly one `windows` crate version on the MSVC target, so a future dependency bump fails with a readable message instead of a wall of D3D12 type mismatches.

### Changed (internal)
- `src/audio/` migrated to the rodio 0.22 API: `Sink` → `Player` (`Player::connect_new(mixer)`, now infallible — the "failed to create sink" branches are gone), `OutputStream`/`OutputStreamHandle` → a single `MixerDeviceSink` (`DeviceSinkBuilder::open_default_sink`), `Source::current_frame_len` → `current_span_len`, and `channels()`/`sample_rate()` now return `NonZero` newtypes (`ChannelCount`/`SampleRate`).
- Decoded samples are `f32` end to end in rodio 0.22, so the per-channel effect chain (pitch → low-pass → fade-in) no longer round-trips through `i16` between stages — one less quantization hop, and the `append_decoded` branch tree collapses accordingly. `PannedSource` is now generic over plain `Source`.

## 0.124.0

**Styled scene transitions — fade / wipe / iris with automatic scene swapping.** `SceneTransition` is the styled successor to the solid-colour `FadeTransition`: coverage runs 0 → 1 (cover) then 1 → 0 (reveal), so the screen covers, the scene swaps *while hidden*, and the new scene is revealed. `App::transition_to_scene` (from `&mut App`) or `start_scene_transition` (from a system) do the whole cover → swap → reveal in one call; `App` swaps the scene at the fully-covered midpoint and drops the transition when it finishes. Additive — a new module, renderer, and re-exports; `FadeTransition` is untouched.

### Added
- `engine::SceneTransition` + `TransitionStyle` (`#[non_exhaustive]`: `Fade`, `WipeLeft`/`Right`/`Down`/`Up`, `IrisIn`, `IrisOut`) + `TransitionPhase` (`Out`/`In`/`Done`) (`src/scene_transition.rs`). `new(style, half_duration)` / `with_color`, `update(dt)`, `just_covered`, `is_done`, and `covered_at(x, y, aspect)` (a CPU mirror of the shader geometry for logic/tests). 5 unit tests + 1 doctest.
- `engine::start_scene_transition(world, scene, style, half_duration)` — the world-level trigger callable from a **system** (the twin of the `&mut App` `App::transition_to_scene`). Both cover the screen, swap the scene while hidden (via an internal `PendingSceneTransition` resource consumed at full cover), then reveal it. `SceneTransition` is auto-registered persistent so the reveal survives the mid-transition world reset. An integration test (`scene_transition_auto_swaps_at_cover_and_clears_when_done`) drives `App::update` headlessly to verify the swap timing.
- `TransitionRenderer` (`src/renderer/transition.rs`): a native-only full-screen styled-coverage pass, run in the same slot as the fade pass (`app/render/frame.rs`); an all-`vec4` uniform (avoiding the WGSL `vec3` alignment trap) carries `coverage`/`style`/`aspect`/`softness` + colour; the shader mirrors `covered_at` with a soft edge. The swap still happens on wasm; only the overlay is native-only.
- Example `scene_transition` (`examples/scene_transition.rs`): keys **1**–**7** transition between four full-screen-colour levels, one per style; `HEADLESS_SHOT` freezes a mid-iris still.

## 0.123.0

**Seedable deterministic `Rng` + `WeightedTable` loot/spawn tables.** A public SplitMix64 PRNG whose stream is fixed by its seed — a game stores just a seed to reproduce a run, a procedural level, or a loot sequence, independent of `rand`'s thread RNG. `WeightedTable<T>` draws items by relative weight (the loot-drop / spawn-table primitive), driven by a caller-supplied `Rng` so a table + seed reproduce the same sequence. `mapgen` adopted `Rng` and dropped its own private copy — the generated dungeons are byte-identical (its determinism tests are unchanged), which is exactly the point of a shared RNG. Additive — a new module and re-export; `mapgen`'s refactor is behavior-preserving.

### Added
- `engine::Rng` (`src/rng.rs`): `new(seed)`, `next_u64` / `next_u32`, `range(lo, hi)` (i32 `[lo, hi)`, empty range → `lo`), `f32_unit()` (`[0, 1)`), `range_f32`, `bool()`, `chance(p)`, `pick(&[T])`, `shuffle(&mut [T])` (Fisher–Yates). A golden-value test pins the stream so the SplitMix64 constants can't silently drift (which would change every seed-derived artifact).
- `engine::WeightedTable<T>` (`src/rng.rs`): `new()` / `with(item, weight)` / `add` (f32 relative weights; `<= 0` or non-finite ignored), `pick(&mut Rng)` / `pick_index`, `total_weight` / `len` / `is_empty` / `items`. 11 unit tests (stream stability, range/`f32_unit` bounds, `chance` extremes + rate, pick/shuffle permutation, weighted distribution ~9:1, deterministic replay) + 2 doctests.
- Example `loot_table` (`examples/loot_table.rs`): weighted rarity drops (Common/Uncommon/Rare/Legendary) with a live histogram converging to per-rarity target markers; **Space** draws one, **A** draws 100, **R** replays the identical sequence (determinism), **N** picks a new seed. `HEADLESS_SHOT` pre-rolls 200 draws.

### Changed (internal)
- `src/mapgen.rs`: the BSP generator now seeds from `engine::Rng` instead of a duplicate private SplitMix64 `struct Rng`. Byte-identical output (same constants + `range`/`bool`/`chance` semantics); mapgen's determinism/connectivity tests pass unchanged (its redundant RNG-internals test was removed — `rng.rs` covers it).

## 0.122.0

**procgen ↔ FOV composition bridge + the `roguelike` capstone.** `DungeonMap::to_path_grid()` turns a generated dungeon into a `PathGrid` whose walkable cells are exactly its floor cells — the direct bridge to enemy pathfinding (`find_path`) *and*, via `FovMap::from_path_grid`, to field-of-view, so the walls that block movement are exactly the walls that block sight. The `roguelike` example composes this session's two features (seq-1 `FovMap` + seq-2 `generate_bsp_dungeon`) into one playable slice: a seeded BSP dungeon explored under fog-of-war. Additive — one new method plus an example, no existing API touched.

### Added
- `engine::DungeonMap::to_path_grid(&self) -> PathGrid` (`src/mapgen.rs`): walkable = `Tile::Floor`, coords 1:1. Composes with `FovMap::from_path_grid` in one line (`FovMap::from_path_grid(&map.to_path_grid())`) — the same seam feeds `find_path`. 2 unit tests (walkability mirrors floor; a `FovMap` built from a generated dungeon sees the spawn room) + 1 doctest.
- Example `roguelike` (`examples/roguelike.rs`): the procgen↔FOV capstone. A seeded `generate_bsp_dungeon` explored under `FovMap` fog-of-war — cells in sight render bright, explored-but-unseen dim, never-seen black; rooms tinted warmer than corridors; a gem hides in every non-spawn room, revealed only when seen. WASD/arrows move, **+/-** torch radius, **R** descends to a fresh always-connected dungeon (new seed, blank fog). `HEADLESS_SHOT` capture of the lit spawn room in fog.

## 0.121.0

**Procedural dungeon generation — BSP rooms + corridors, guaranteed connected and deterministic.** `generate_bsp_dungeon(w, h, seed, &params)` recursively splits the map into partitions, carves a room into each leaf, and connects sibling partitions with L-corridors while unwinding — so every room is reachable from every other. Generation depends only on the `seed` (a private SplitMix64 PRNG, never `rand`'s thread RNG), so the same seed + params always produce the identical `DungeonMap` — a game stores just the seed to regenerate the level or reproduce a run. A plain owned grid like `FovMap`/`PathGrid` (not an ECS type); composes with them via `to_tilemap_tiles`. Additive — a new module and one re-export, no existing API touched.

### Added
- `engine::mapgen` (`src/mapgen.rs`): `generate_bsp_dungeon(width, height, seed: u64, &DungeonParams) -> DungeonMap`. `DungeonMap { width, height, rooms: Vec<Room> }` with `tile` / `is_floor` / `is_wall` (out-of-bounds reads `Wall`), `first_room_center`, and `to_tilemap_tiles(floor_id, wall_id) -> Vec<Vec<u32>>` (build a `Tilemap` / `PathGrid` / `FovMap` from the same layout). `Tile` (`Wall` / `Floor`), `Room { x, y, w, h }` (+ `center` / `contains`), and `DungeonParams` (min/max leaf, min room, margin, max depth). Cell count capped at `MAX_PATH_GRID_CELLS`. Re-exported at `engine::{generate_bsp_dungeon, DungeonMap, DungeonParams, Room, Tile}`. 8 unit tests (determinism, all-rooms-connected flood fill, border-is-wall, rooms in-bounds + min-sized, degenerate/overflow safe, tilemap-tiles, PRNG determinism/bounds) + 1 doctest.
- Example `procgen_dungeon` (`examples/procgen_dungeon.rs`): renders the full generated dungeon (rooms tinted warmer than the corridors linking them) over a dark backdrop; WASD/arrows walk an explorer (walls block), **R** regenerates with the next seed (a fresh, always-connected dungeon), HUD shows the seed + room count. `HEADLESS_SHOT` capture of the starting dungeon.

## 0.120.0

**`FovMap` — grid field of view / fog-of-war via recursive shadowcasting.** A plain owned helper (like `PathGrid`/`InputBuffer` — not an ECS component or system): a game keeps one and recomputes it when the observer moves. `compute(origin, radius)` runs recursive shadowcasting over 8 octants (an opaque cell is itself lit, then casts a shadow over everything behind it; Euclidean `dx²+dy²≤radius²`), clearing the `visible` set each call and accumulating a `revealed` set — the fog-of-war "explored" memory. `line_of_sight(a, b)` is the companion point-to-point Bresenham sight check. Genre-agnostic: roguelike FOV, stealth sight lines, top-down fog-of-war. Additive — a new module and one re-export, no existing API touched.

### Added
- `engine::FovMap` (`src/fov.rs`): row-major `opaque` / `visible` / `revealed` grids. `new(w, h)` (cell count capped at `MAX_PATH_GRID_CELLS` — over-cap/overflow yields an empty map with a logged `error!`, mirroring `PathGrid`); `from_path_grid` (a `PathGrid`'s non-walkable cell → opaque, coords 1:1); `set_opaque` / `is_opaque` / `is_visible` / `is_revealed` / `clear_visible` / `reset`; `compute(origin, radius)` (recursive shadowcasting, `radius ≤ 0` lights only the origin); `line_of_sight(a, b)` (Bresenham, endpoints excluded so the observer can see the wall it looks at). 9 unit tests + 2 doctests.
- Example `fov` (`examples/fov.rs`): a playable top-down dungeon — WASD/arrows move an observer, walls cast shadows in real time, cells in sight render bright / explored-but-unseen dim / never-seen black, and gems are revealed only when they fall inside the field of view. `+`/`-` grow/shrink the sight radius. `HEADLESS_SHOT` capture of the starting field of view.

## 0.119.0

**`Switch` — a styled boolean toggle (sliding track + knob).** The switch-look alternative to `CheckBox` (same boolean meaning, a more natural affordance for settings on/off rows): a pill track colored by state with a round knob that slides left (off) / right (on), plus an optional label. Clicking anywhere on the node flips it and emits `UiEvent::SwitchToggled(entity, bool)`; while focused, Enter/Space toggle it and ←/→ set it off/on absolutely (a 2-position-slider feel, more keyboard/gamepad-operable than a checkbox). One entity is the whole widget; render and the knob position share a single geometry source (`track_rect`/`knob_rect`). Additive — no existing widget or event changed.

### Added
- `engine::ui::Switch` (re-exported at `engine::Switch`): `on` / `label` / `track_width` / `track_height` / `on`·`off`·`knob`·`text` colors / `font_size`; builders `new`/`with_on`/`with_size`/`with_colors`/`with_text_color`/`with_font_size`; helpers `track_color` / `track_rect` / `knob_rect` / `track_radius` / `knob_radius` (pill radius = track height / 2, knob = a circle padded inside the track). `src/ui/switch.rs`.
- `UiEvent::SwitchToggled(Entity, bool)` — the new on/off state. A click always flips (so every click emits); the ←/→ focus arm sets the state absolutely and emits only when it actually changed.
- `switch_pass` in the `UiSystem` order (after Stepper, before Dropdown): whole-node click-toggle (CheckBox-style press+release ownership, drag-off cancels) + render of the track, knob, and label. `Switch` is one focus stop (Tab), pointer-opaque in `PointerCapture`, and reflect/clone/serde/editor-add registered like the other widgets.
- Example `ui_switch` — Sound / Music / Fullscreen switches plus a larger custom-green Vsync, wired to a live on/off readout and a change counter (`HEADLESS_SHOT` capture path).

## 0.118.0

**`Stepper` — a numeric `-`/`+` spinner widget.** Adds the value-adjustment control the widget suite was missing (quantity fields, discrete settings values): a bounded `f32` `value` in `min..=max` stepped by `step`, like a `Slider` but nudged discretely by flanking `-`/`+` buttons rather than dragged. Clicking a button (or ←/→ while focused) steps the value clamped to the bounds and emits `UiEvent::StepperChanged(entity, f32)` only when it actually changed — a click at a bound is silent. One entity is the whole widget; the node rect splits into a `-` button, a centered value label, and a `+` button from a single geometry source. Additive — no existing widget or event changed.

### Added
- `engine::ui::Stepper` (re-exported at `engine::Stepper`): `value` / `min` / `max` / `step` / `decimals` / `font_size` / `bg`·`button`·`button_hover`·`text`·`border` colors / `corner_radius` / `border`; builders `new`/`with_step`/`with_decimals`/`with_font_size`/`with_colors`/`with_corner_radius`/`with_border`; helpers `clamped_value` (uses `max/min` not `f32::clamp`, so a reflect-edited `min > max` can't panic) / `stepped` / `at_min` / `at_max` / `label` / `button_width` / `zone_at`. `src/ui/stepper.rs`.
- `engine::ui::StepButton` (`Dec` / `Inc`) — the return of `Stepper::zone_at`, the single geometry source shared by render and click resolution.
- `UiEvent::StepperChanged(Entity, f32)` — the new value, emitted only when it actually changed (stepping past a bound is silent); an `f32` payload like `SliderChanged`, not an index.
- `system/stepper_pass.rs`: click-select (CheckBox-style press+release ownership, drag-off cancels) + render, inserted in the `UiSystem` pass order after ListBox and before Dropdown. Registered in `PointerCapture` (pointer-opaque) and as a focus stop (←/→ step the value). No transient state — button hover is recomputed each frame.
- `examples/ui_stepper.rs` — three steppers (volume 0..100 step 5 / difficulty 1..5 / a custom-styled zoom 0.5..3.0 step 0.25, 2 decimals) wired to a live HUD readout + change counter, with a `HEADLESS_SHOT` capture path.

## 0.117.0

**`ListBox` — a scrollable, selectable single-column list widget.** Rounds out the widget suite alongside `RadioGroup`/`Dropdown`/`TabBar` with the one common control they were missing: a many-item list that scrolls (inventory / level-select / file / dialogue-choice lists). One entity holds all rows + the single selected index (like `RadioGroup`, but scrollable). The mouse wheel scrolls the list under the pointer; clicking a *visible* row selects it (CheckBox-style press+release ownership, drag-off cancels); it is one focus stop where **↑/↓** or **←/→** step the selection and auto-scroll it into view. Additive — no existing widget or event changed.

### Added
- `engine::ui::ListBox` (re-exported at `engine::ListBox`): `items` / `selected` / transient `scroll_offset` / `row_height` / `font_size` / `bg`·`hover`·`selected`·`text`·`border` colors / `corner_radius` / `border`; builders `new`/`with_selected`/`with_row_height`/`with_font_size`/`with_colors`/`with_corner_radius`/`with_border`; scroll-aware geometry helpers `selected_index`/`selected_item`/`content_height`/`row_at`/`clamp_scroll`/`scroll_to_selected`. `src/ui/list_box.rs`.
- `UiEvent::ListBoxChanged(Entity, usize)` — emitted only when the selection actually changed (re-picking the current row is silent), like the other widget-change events.
- `system/list_box_pass.rs`: wheel-scroll + click-select + render, inserted in the `UiSystem` pass order after TabBar and before Dropdown. Highlight rects are clamped and labels bottom-clipped to the node rect so a partly-scrolled row never spills. Registered in `PointerCapture` (pointer-opaque) and as a focus stop.
- Example `ui_list_box` (12-item inventory that scrolls + a custom-styled level-select list, wired to a live HUD readout + change counter; `HEADLESS_SHOT` self-check). Reflect/clone/serde/editor-add registrations mirror the other widgets.

### Changed (internal)
- `InputSnapshot` gains keyboard-only `nav_up`/`nav_down` (Arrow Up/Down), consumed only by the `ListBox` focus arm — a gamepad's D-pad/stick Up/Down still cycle focus, so a pad steps a list with Left/Right. The existing Left/Right widget arms are untouched.

## 0.116.0

**Versioned saves round-trip data-carrying enum variants (EW-006).** `save_versioned` used to re-parse the payload into a generic `ron::Value` for the envelope — but `ron::Value` cannot represent enum variants, so a struct variant like `Closed { reopen_day: 3 }` silently degraded to a bare map at save time and failed at load with "expected enum, found map". The envelope now keeps the payload as **RON text** (tagged `format: 1`); when the stored version equals the migrator's current version (no steps to run), `load_migrated` deserializes the target type straight from that text with full serde fidelity. Saves written before 0.116 (the tree envelope, `format` absent) still load and migrate via the legacy path.

### Fixed
- `save_versioned`/`load_migrated` (+ `_with_key` variants): any serde-serializable payload — including enums with payload — round-trips unchanged when no migration is pending. Regression test `versioned_enum_struct_variant_roundtrips_at_current_version` (the dungeon-merchant repro: `Vec<(u32, MarketStatus)>`).

### Changed
- New envelope format `(version, format: 1, data: "<payload RON text>")` — older engine builds cannot read saves written by 0.116+ (forward compatibility was never promised); 0.116 reads both formats. A `format: 1` envelope whose `data` is not a string reads as `SaveError::Corrupted`.
- **Documented constraint:** `SaveMigrator` steps still operate on a `ron::Value`, so an enum-carrying payload that actually *needs migrating* cannot pass through the steps — it now fails with a RON error (test-pinned) instead of silently corrupting. Keep enum-carrying fields stable across schema versions, or mirror them into structs while a migration is pending (both fn docs spell this out).
- Example `save_migration`: the current save format now carries `GameMode::Custom { multiplier: f32 }` (`#[serde(default)]`, so migrated v1 saves default to `Normal`) — the Space re-save/reload round-trip exercises the fix in real play.

## 0.115.0

**`Button` catches up to the widget-suite styling conventions (EW-005).** The oldest widget predates the 0.107–0.113 suite: styling one meant imperative field assignment while every neighbouring widget chains `with_*` builders, and its background could not be rounded. Additive — existing field-style construction keeps working, and the new `corner_radius` defaults to `0.0` (sharp, byte-identical; old scene RON loads unchanged via the struct-level `#[serde(default)]`).

### Added
- `Button` builders: `with_colors(normal, hovered, pressed)`, `with_disabled_color`, `with_text_color`, `with_font_size`, `with_corner_radius` — the `TabBar`/`Dropdown`/`RadioGroup` convention.
- `Button.corner_radius: f32` — the background rect is now pushed `with_corner_radius(...)` through the UI SDF pipeline (`0.0` = the sharp fast path); also a Reflect field, so the editor Inspector can edit it.
- Tests: builder chain sets every field, old RON without `corner_radius` still loads, reflect roundtrip, and a `UiSystem`-level check that the radius reaches the queued `DrawRect`.
- Example `ui_rounded` now spawns **rounded buttons** styled entirely through the builders (plus a `HEADLESS_SHOT` capture path).

## 0.114.0

**`FloatingText` gains a UI-layer z passthrough (EW-004).** A floating combat number can now composite *among* the UI rects instead of always drawing on top — so a higher-z overlay (a pause-menu scrim) actually covers live floats instead of the numbers bleeding through it. Additive: without `with_z` the queued `DrawText` carries `z: None` and the render path is byte-identical to before.

### Added
- `FloatingText.z: Option<f32>` + `FloatingText::with_z(z)` — passed through by `FloatingTextSystem` to the `DrawText` it queues (same semantics as `DrawText::z`, the 0.110 layered-text machinery): `Some(z)` renders under any UI rect drawn at a greater z and over a lower one; `None` (default) keeps the historical always-on-top pass.
- Render regression test `floating_text_with_z_hides_under_a_higher_z_rect` (`tests/render.rs`, runs on CI lavapipe): a covering rect hides a layered float, an uncovered control renders, and a default (no-z) float still draws over every rect.
- Example `floating_text` now demos the pattern: **P** toggles a z-100 overlay scrim the layered floats hide behind (rising numbers emerge above its top edge), **Z** switches new pops between layered (`with_z(50)`) and the legacy on-top pass; headless auto mode raises the scrim partway through.

## 0.113.2

**The game-feel capstone ships to the web.** `examples/game_feel.rs` — the arena that composes the whole juice toolkit with a live widget-suite settings menu — now runs in a browser via the established `cargo build --example` + `wasm-bindgen` harness. Examples/scripts/docs only; **no engine code or public API change.**

### Added
- `examples/game_feel/web/` — `build.sh` (release wasm build + `wasm-bindgen --target web`) and `index.html` (Start button — winit wants a user gesture; `?autostart=1` for headless harnesses; canvas keyboard focus for ←/→/Space/X/Esc). The example moved to directory style (`examples/game_feel/game_feel.rs` + a `[[example]]` entry) like the other web-shipped examples.
- Dual-target entry: setup factored into a shared `build_app()` called by both the native `main()` (windowed + `HEADLESS_SHOT`, which is native-only and now cfg-gated out of the wasm build) and the new `#[wasm_bindgen]` `run_game_feel()` — the browser runs the exact same setup as native.
- `scripts/game_feel_web_smoke.sh` — render-only wasm smoke (centered_text model): builds, serves, renders one headless DPR=2 frame under SwiftShader, asserts non-blank; listed in `docs/WASM_SMOKES.md`.

## 0.113.1

**Dropdown open-list seam fix — one rounded background instead of stacked rounded rows.** With a `corner_radius` set, each open-list row was its own 4-corner-rounded `DrawRect`, so stacked rows left notch seams between their corners — and covered widget labels could bleed through the gaps. Visual-only; interaction, geometry helpers, and the public API are untouched.

### Fixed
- `system/dropdown_pass.rs` — the open list now renders **one** full-list rounded background (`list_pos` × `list_height`, the existing single-source geometry) plus a separate square hover highlight, inset horizontally by the corner radius so it never pokes out of the background's rounding on the first/last row. Row labels moved one Z sub-layer up with the highlight (equal-z tie keeps text over its surface), still under `TOOLTIP_Z`. `corner_radius: 0.0` renders visually identical to before. Existing queue-shape test updated + a new geometry test (background + inset highlight, exact rects); 18 dropdown tests green. Headless before/after captures verified the seams (and the label bleed through them) are gone.

## 0.113.0

**The game-feel capstone example — the juice toolkit and the widget suite proven to compose.** `examples/game_feel.rs` is one small playable arena that drives the engine's game-feel helpers **together**, configured live from a real pause/settings menu — the composition target the 0.107.0–0.112.0 widget run was building toward. Engine change is one tiny additive builder the example surfaced (the VISION "fix the API when the example is awkward" rule); everything else composes on the existing public API, unchanged.

### Added
- Example `game_feel` — a training arena on two pads with a jumpable gap: `InputBuffer` jumps (buffered + coyote), a `SpriteTrail` on the player, and an **X** attack on three dummies that fires the whole juice stack at once — `HitFlash` + a `FloatingText` damage number (crit-styled every 4th hit, sideways-staggered so stacked numbers stay readable) + `Camera::shake` + a `TimeScale` hit-stop timed in `RealDt`. **Esc** pauses (`TimeScale` 0) into a settings panel built from the widget suite: a `TabBar` switches a *Feel* page (`RadioGroup` shake Off/Low/High, `Slider` hit-stop ms, `Dropdown` trail preset Off/Subtle/Heavy — applied on change, since re-adding a `SpriteTrail` each frame would reset its emit timer) and a *Movement* page (`Slider`s for speed + the two `InputBuffer` forgiveness windows, rebuilt on change; both at 0 = a strict jump). Every setting drives the effects the moment you resume. Headless mode plays itself and opens the menu for the capture.
- `UiNode::with_visible(bool)` builder — spawn a widget hidden (the pause-menu pattern: entities exist from startup, revealed by flipping `visible`) without mutating after construction; previously only reachable by post-construction field assignment. +1 unit test. **No other public API change.**

**A `TabBar` widget — equal-width tab headers, one active; content switching stays the game's job.** The last of the settings-menu staples after `Dropdown` (0.109.0) and `RadioGroup` (0.111.0). The bar renders headers only — a game reads `UiEvent::TabChanged` (or polls `selected_index`) and toggles its per-tab widgets' `UiNode::visible`, which keeps the widget genre-agnostic and composes with focus (hidden widgets are already skipped by Tab cycling, so keyboard nav never lands on an inactive tab's content). Additive, native+wasm.

### Added
- `TabBar` (`src/ui/tab_bar.rs`) `{tabs, selected, bg_color, active_color, hover_color, text_color, active_text_color, font_size, corner_radius, gap}`. Ctors `new(tabs)` + `with_selected` / `with_colors` / `with_hover_color` / `with_text_colors` / `with_font_size` / `with_corner_radius` / `with_gap` builders; accessors `selected_index` (clamps on read like `Dropdown`/`RadioGroup`) / `selected_tab`; geometry helpers `tab_width` (equal split minus gaps, floored at zero) / `tab_rect` / `tab_at` — the **single geometry source** shared by rendering and click resolution (the gaps between headers select nothing). Registered for reflect / clone / serde / editor add-remove like the other widgets; re-exported from `engine` (and `engine::ui`).
- `system/tab_bar_pass.rs` — runs after the radio-group pass, before dropdowns. A completed click (press **and** release owned via the shared `PointerCapture`, `CheckBox`-style; drag-off cancels) selects the header under the release point → new **`UiEvent::TabChanged(entity, index)`** (only when the index actually changed; re-picking the current tab is silent). Headers render active / hovered / inactive backgrounds + centered titles (`TextAlign::Center`).
- Focus-pass integration: `TabBar` is focusable (Tab / D-pad / stick) as **one focus stop for the whole bar**; **←/→** (or D-pad Left/Right) steps the active tab (clamped, `Slider`-style, emitting `TabChanged`). It also joins the pointer-capture surfaces, so a covered bar doesn't select through an overlay.
- Example `ui_tabs` — the intended "tab container" wiring: a 3-tab bar (Stats / Inventory / Options) whose tab groups (progress bars / a scroll list / checkbox + slider) are shown/hidden by a tiny per-frame visibility system. 12 unit/integration tests + 1 doctest.

### Notes
- Adding the `TabChanged` variant to `UiEvent` is technically breaking for exhaustive matches on that enum (pre-1.0 license; add a wildcard arm or the new variant).

## 0.111.0

**A `RadioGroup` widget — mutually exclusive options, one entity per group.** The settings-menu companion to `Dropdown`: the whole option list lives on one entity (like `Dropdown`/`ScrollView`), so the options can never disagree about which one is selected. Click a row to select it; fully keyboard/gamepad-operable. Additive, native+wasm.

### Added
- `RadioGroup` (`src/ui/radio_group.rs`) `{items, selected, circle_color, fill_color, hover_color, text_color, font_size, circle_size, item_height}`. Ctors `new(items)` + `with_selected` / `with_colors` / `with_text_color` / `with_font_size` / `with_circle_size` / `with_item_height` (`0.0` = divide the node height evenly) builders; accessors `selected_index` (clamps on read like `Dropdown`) / `selected_item`; geometry helpers `resolved_item_height` / `row_at` — the **single geometry source** shared by rendering and click resolution (rows are clickable only inside the node rect, the pointer-capture surface; overflowing rows render but don't select). Registered for reflect / clone / serde / editor add-remove like the other widgets; re-exported from `engine` (and `engine::ui`).
- `system/radio_group_pass.rs` — runs after the checkbox pass, before dropdowns. A completed click (press **and** release owned by the widget through the shared `PointerCapture`, `CheckBox`-style — dragging off cancels) selects the row under the release point and emits the new **`UiEvent::RadioChanged(entity, index)`** (only when the index actually changed; re-picking the current option is silent). Each row renders a circle ring (an SDF rounded rect at full corner radius), a filled dot on the selected row, the option label, and a subtle hover tint under the cursor's row.
- Focus-pass integration: `RadioGroup` is focusable (Tab / D-pad / stick) as **one focus stop for the whole group**; **←/→** (or D-pad Left/Right) steps the selection directly (clamped, `Slider`-style, emitting `RadioChanged`). It also joins the pointer-capture surfaces, so a covered group doesn't select through an overlay.
- Example `ui_radio` — two live groups (difficulty + a custom-styled music group with bigger circles and fixed row heights) wired to a HUD readout + a `RadioChanged` change counter. 13 unit/integration tests + 1 doctest.

### Notes
- Adding the `RadioChanged` variant to `UiEvent` is technically breaking for exhaustive matches on that enum (pre-1.0 license; add a wildcard arm or the new variant).

## 0.110.0

**Text z-ordering — an overlay finally hides the labels underneath it.** Historically all text drew in one final pass on top of every UI rect, so an open dropdown list, a tooltip, or a panel could never visually cover a widget label (the label bled through — surfaced by the `ui_dropdown` capture in 0.109.0). A `DrawText` can now carry a UI-layer z and composite **among** the rects; the widget passes set it automatically. Additive — text without a z keeps the exact historical on-top behavior.

### Added
- `DrawText.z: Option<f32>` + `with_z(z)`. `None` (default) = the final on-top text pass, over every rect and after post-processing — right for HUD readouts, byte-identical to before. `Some(z)` = the text interleaves with the UI rects/images at that z (same scale as `DrawRect::z`): a surface above covers it, a surface below stays behind, a z tie draws the text on top (a label over its own widget background). Layered text renders **before** post-processing, so under an HDR/bloom pipeline it is graded together with its widget.
- The interleave machinery: `text/layering.rs::interleave_runs` (pure run-splitting, 7 unit tests); `SpriteRenderer::prepare_ui_primitives` + `render_ui_primitive_range` (the sorted primitives upload **once** per frame — `queue.write_buffer` executes at submit, so per-run re-uploads would clobber earlier passes — and each run draws an instance-buffer sub-range); `TextRenderer::render_batch` + `end_frame` with per-format `FormatPool`s (a format-bound glyphon atlas + one pooled glyphon renderer per batch, since each holds exactly one prepared batch's vertex buffers; non-surface formats — e.g. the HDR `Rgba16Float` intermediate — get their own pool lazily, the text analogue of the format-matched pipeline caches). The shaped-buffer cache is shared across batches; eviction/trim/reset happen once per frame in `end_frame`.
- Every widget pass now layers its text at its widget's z (label / button / checkbox / text input / scroll view / dropdown box + rows / tooltip) — so the open dropdown list and the tooltip box now hide covered labels for real. `TextQueue::take_layered` (crate-internal) partitions the queue in the frame orchestration.
- Example `text_layers` — two overlapping cards whose captions layer with them (Space raises/lowers the overlay; the covered caption half disappears/reappears) + an on-top HUD line. Render test `layered_text_is_covered_by_higher_z_rect` (covered layered text reads as background, uncovered control renders, z-None text still draws over a z=50 rect) — runs on CI lavapipe.

### Notes
- `DrawText` is documented as builder-constructed (`new`/`centered` + `with_*`), so the new field is non-breaking for documented usage.
- Widget label text moving from the post-post on-top pass into the pre-post scene affects HDR/bloom scenes only: widget text is now tone-mapped with its widgets (arguably the correct look); game-pushed HUD text (no z) is unchanged.

## 0.109.2

**Example layout nit from the playtest re-test.** No engine change.

### Fixed
- Example `ui_dropdown`: the bottom-edge (flip-up) dropdown sat at x=320 where the HUD event/hint lines overlapped its box — moved to the bottom-right corner (x=520). Reported during the 2026-07-02 real-mouse re-test; cosmetic only.

## 0.109.1

**Dropdown fixes from the first real-mouse playtest.** Three findings, three fixes; no new public API.

### Fixed
- Example `ui_dropdown` never called `app.register_event::<UiEvent>()`, so the `Events<UiEvent>` bus did not exist and every `DropdownChanged` / `ButtonClicked` was silently dropped — the HUD event line and the Apply click counter never moved. (Engine behavior was correct; the widget's own tests insert the bus. Example-only fix.)
- Opening a dropdown with **Enter** now closes any other open dropdown, matching the pointer path (where pressing one dropdown is a press-away for every other). Previously a keyboard-opened list left a mouse-opened one dangling.

### Changed
- The closed box now opens on **press** (was: on the completed press+release click), enabling the native combobox one-gesture flow — press the box, drag onto a row, release to select. The opening press's own release on the box keeps the list open (a transient `press_opened` flag, never serialized); a later box click still closes. Same-frame press+release (a fast click) behaves exactly as before.

## 0.109.0

**A `Dropdown` / combobox widget — the settings-menu staple.** Click to open the item list, click an item to select it, press anywhere else to close. The open list overlays — and absorbs the pointer over — everything underneath it, and a bottom-edge dropdown flips its list upward. Fully keyboard/gamepad-operable via the existing focus pass. Additive, native+wasm.

### Added
- `Dropdown` (`src/ui/dropdown.rs`) `{items, selected, open, bg_color, hover_color, text_color, font_size, corner_radius, item_height}`; `open` is transient (`#[serde(skip)]` — a saved scene never restores an open list; a game may set it to open programmatically). Ctors `new(items)` + `with_selected` / `with_colors` / `with_hover_color` / `with_font_size` / `with_corner_radius` / `with_item_height` (`0.0` = the node's own height) builders; accessors `selected_index` (clamps like `ProgressBar::fraction`) / `selected_item`; geometry helpers `resolved_item_height` / `list_height` / `flips_up` / `list_pos` / `expanded_rect` — the **single geometry source** shared by rendering, click row resolution, and the pointer capture, so they can never disagree; const `DROPDOWN_LIST_Z` = 90.0 (above normal UI, below `TOOLTIP_Z`). Registered for reflect / clone / serde / editor add-remove like the other widgets; re-exported from `engine` (and `engine::ui`).
- `system/dropdown_pass.rs` — runs after the checkbox pass, before tooltips. Click on the closed box opens; a completed click on a row selects it + closes + emits the new **`UiEvent::DropdownChanged(entity, index)`** (only when the index actually changed); press-drag-release onto a row also selects (native combobox feel); a press anywhere else closes without selecting. The row under the cursor is hover-highlighted; the selected row carries a `•` marker; the box shows a ▼/▲ state arrow. An empty or hidden dropdown never stays open (`item_height <= 0` is division-safe).
- `PointerCapture` now registers an **open** dropdown's whole expanded rect (box + list, `Dropdown::expanded_rect`) at `DROPDOWN_LIST_Z`, so a widget under the open list neither clicks nor hovers through it (a closed dropdown captures like any widget at its node z).
- Focus-pass integration: `Dropdown` is focusable (Tab / D-pad / stick); **Enter/Space/A** toggles the list, **←/→** steps the selection directly (clamped, `Slider`-style, emitting `DropdownChanged`) — a settings row is fully operable without a pointer.
- Example `ui_dropdown` — a settings panel: quality + difficulty dropdowns over a row of buttons (the HUD's Apply click counter proves the open list absorbs clicks), a bottom-edge dropdown that opens upward, and a `DropdownChanged` event readout. 14 unit/integration tests + 1 doctest.

### Notes
- Adding the `DropdownChanged` variant to `UiEvent` is technically breaking for exhaustive matches on that enum (pre-1.0 license; add a wildcard arm or the new variant).
- Known engine-wide limitation, unchanged by this widget: text renders in its own pass after all UI rects, so a covered widget's *label* still shows through an overlay (same as a `Panel` over a `Button`). The open list absorbs the pointer correctly; only text bleeds. A per-text z is the candidate fix.

## 0.108.0

**A hover `Tooltip` widget — the popup every inventory slot, stat readout, and icon button wants.** Attach a `Tooltip` next to any `UiNode` widget (`Button`, `ProgressBar`, `Label`, `Panel`, …); rest the cursor on it and after a delay a small text box fades in next to the cursor. Additive, native+wasm; one small deliberate API opening (`InputState::set_cursor` is now `pub`).

### Added
- `Tooltip` (`src/ui/tooltip.rs`) `{text, delay_secs, fade_secs, font_size, text_color, bg_color, corner_radius, border, border_color, padding, offset, size_override}` + a private transient hover timer (`#[serde(skip)]`). Ctors `new(text)` + `with_delay` / `with_fade` / `with_font_size` / `with_colors` / `with_corner_radius` / `with_border` / `with_padding` / `with_offset` / `with_size` builders; accessors `hovered_secs` / `is_showing` / `fade_alpha` / `estimated_size`; consts `DEFAULT_TOOLTIP_DELAY_SECS` = 0.4, `DEFAULT_TOOLTIP_FADE_SECS` = 0.1, `TOOLTIP_Z` = 100.0. The box auto-sizes from a shaped-width **estimate** (≈ 0.5 em per ASCII char, 1 em per CJK/full-width char, 1.2× line height, `\n` breaks lines; exact shaping happens at render time) — `with_size` pins the content box exactly. Empty `text` disables. Registered for reflect / clone / serde / editor add-remove like the other widgets; re-exported from `engine` (and `engine::ui`).
- `system/tooltip_pass.rs` — runs **last** in `UiSystem` (so tooltip text is queued after, and draws over, other widget text). Hover requires the cursor inside the node's rect **and** the point not covered by a pointer-opaque widget drawn above it (new `PointerCapture::occludes` — the host widget itself need not be pointer-opaque, so tooltips on `Label`/`ProgressBar` work, while one under an overlay panel stays silent). After the delay it draws the background box (+ optional border) at `TOOLTIP_Z` via the UI SDF pipeline and the text with a fade-in; the box is viewport-clamped (right overflow slides left, bottom overflow flips above the cursor).
- Example `ui_tooltip` — a button with a multi-line bordered stats tooltip, a health bar whose tooltip text tracks its live value each frame, and a label with an instant custom-colored tooltip. 13 unit/integration tests + 1 doctest.

### Changed
- `InputState::set_cursor` is now `pub` (was `pub(crate)`). Normally the windowing loop feeds the cursor; it is public so a game can drive a **virtual cursor** (e.g. a gamepad right-stick pointer hovering UI) and so a headless run can synthesize hover for captures/tests (the `ui_tooltip` example's headless path uses it). No behavior change.

## 0.107.0

**A read-only UI `ProgressBar` / gauge widget — health, mana, loading, XP.** Fills a gap in the widget set (Button/Slider/CheckBox/Label/TextInput/ScrollView/Panel had no read-only bar). Unlike `Slider` it takes no input — the game drives its `value` each frame and the bar reflects it. Additive, native+wasm, one new UI widget.

### Added
- `ProgressBar` (`src/ui/progress_bar.rs`) `{value: 0.0..=1.0, fill_color, bg_color, corner_radius, border, border_color}`. Ctors `new(value)` (clamps) + `with_colors` / `with_corner_radius` / `with_border` builders + `fraction()` (clamps to `0..=1`). Registered for reflect / clone / serde / editor add-remove like the other widgets. Re-exported from `engine` (and `engine::ui`).
- `system/progress_bar_pass.rs` — a non-interactive render pass (mirrors `label_pass`) that draws a background track rect + a fill rect `fraction × width` wide via `DrawRect` (rounded corners + optional border come from the existing UI SDF pipeline), the fill one Z sub-layer above the track. Wired into `UiSystem`.
- Example `ui_progress` — three bars driven over time (oscillating health, looping loading, slow XP), each rounded + bordered with a live percentage readout. 5 unit tests + 1 doctest.

## 0.106.1

**Internal refactor: split the 1619-line `docked.rs` editor module into a `docked/` directory.** Behavior-preserving — no public API change, tests unchanged (only moved with their code). The docked in-game editor's UI, which had grown into a single 1600+-line file, is now split by concern.

### Changed (internal)
- `src/app/editor/ui/docked.rs` → `src/app/editor/ui/docked/{mod, toolbar, entity_kind, context_menu, entities_tab, scene_tab, inspector_tab, assets_tab, save_load}.rs`. `mod.rs` keeps the `update_docked_ui` orchestrator + the `pub(in crate::app)` re-exports so the parent `ui/mod.rs` import is unchanged; the shared entity-kind classifier (`entity_kind`/`entity_type_icon`/`sorted_entity_list`) and context-menu dispatch (`entity_context_menu`/`editor_apply_entity_context_action`) become `pub(super)` in their submodules. The three test modules (`icon_tests`, `context_action_tests`, `swatch_tests`) moved verbatim alongside the code they test. No behavior change; largest file now 341 lines.

## 0.106.0

**Sprite trail / afterimage — leave a fading ghost behind a moving sprite.** The motion trail every dash / dodge / fast projectile wants: attach a `SpriteTrail` to a moving `Sprite` and `SpriteTrailSystem` drops a fading copy behind it every few frames, each fading out and despawning on its own. Additive, native+wasm, models the `HitFlash`/`FloatingText` pattern (transient + user-added system).

### Added
- `SpriteTrail` component (`src/sprite_trail.rs`) `{interval, lifetime, start_alpha}` + private timer, plus `SpriteTrailGhost` (the emitted fading copy). Ctors `new(interval, lifetime)` / `default()` (`DEFAULT_TRAIL_INTERVAL` = 0.045, `DEFAULT_TRAIL_LIFETIME` = 0.4, `DEFAULT_TRAIL_START_ALPHA` = 0.55) + `with_start_alpha` / `with_lifetime`. Clone- and editor-add/remove-registered like `YSort`.
- `SpriteTrailSystem` (user-added, like `HitFlashSystem`) — two passes per frame: snapshot a ghost for every due `SpriteTrail` (a full `Sprite` clone, drawn at `Transform.z - 0.1` so it sits behind the live sprite; **one ghost per source per frame** so a slow frame can't spawn a storm; ghosts carry no `SpriteTrail` so they never emit ghosts of their own), then fade every `SpriteTrailGhost`'s alpha to zero over its lifetime and despawn it. A ghost only fades itself — it never touches the source, so a `SpriteTrail` is safe on a gameplay entity you keep. All re-exported from `engine`.
- Example `sprite_trail` — an orbiting box leaves a fading arc; Space toggles the trail (existing ghosts keep fading out). 6 unit tests + 2 doctests.

## 0.105.0

**Input buffering + coyote time — the two forgiveness tricks that make a platformer jump feel fair.** A jump pressed just *before* landing now fires on touchdown (input buffering), and a jump pressed just *after* walking off a ledge still fires (coyote time). `InputBuffer` is a small pure-logic helper you drive yourself (like `Timer`/`Tween` — not a system or component), genre-agnostic. Additive, native+wasm, one new public type.

### Added
- `InputBuffer` (`src/input_buffer.rs`) — per-frame `set_grounded(bool)` → `press()` → `try_consume() -> bool` → `tick(dt)` (tick **last**). A press is live while `since_press <= buffer_secs`; a jump is eligible while grounded or within `coyote_secs` of leaving the ground; consuming clears both windows so one press yields exactly one jump (no mid-air double-jump). Ctors `new(buffer_secs, coyote_secs)` / `default()` (`DEFAULT_BUFFER_SECS` = 0.12, `DEFAULT_COYOTE_SECS` = 0.10), accessors `buffered_remaining` / `coyote_remaining` / `is_buffered` / `is_coyote_available` / `is_grounded` / `buffer_secs` / `coyote_secs`. All re-exported from `engine`.
- Example `input_buffer` — a kinematic box on a platform with a gap: walk off the right edge to feel coyote time, tap jump before landing to feel the buffer; the HUD shows both windows counting down and labels each jump (ground / coyote / buffered). 9 unit tests + 2 doctests.

### Notes
- **Ticking last** (after `try_consume`) is deliberate: it means a zero-length window disables the forgiveness without a footgun — a grounded same-frame press still jumps. Windows are clamped to `>= 0`.

## 0.104.0

**Floating combat text — pop rising, fading numbers at a world position, then despawn.** A genre-agnostic game-feel primitive: spawn a short-lived `FloatingText` (a damage number, a "+15" heal, a "MISS") that drifts up, fades out, and despawns itself, projected to the screen through the camera. Models the existing `HitFlash` pattern (transient component + user-added system, no serde). Additive, native+wasm, one new public module.

### Added
- `FloatingText` component (`src/floating_text.rs`) `{text, color, velocity, size, lifetime, fade}` + private `elapsed` runtime state. Ctors `new(text)` / `colored(text, color)`, builders `with_velocity` / `with_size` / `with_lifetime` / `with_fade`, read accessors `progress()` / `is_finished()`. Default velocity rises (negative Y is up in this engine's world space, as in `YSort`). Clone-registered; **no serde** (transient effect, like `HitFlash`).
- `FloatingTextSystem` (user-added, like `HitFlashSystem`) — each frame ages every `FloatingText`, drifts its entity by `velocity`, fades the alpha over `lifetime`, draws it via the `TextQueue` (screen-space, projected with `Camera::world_to_screen`; a `Transform`-position fallback when there's no `Camera`), and **despawns the whole entity** when it expires. Add it before rendering.
- `spawn_floating_text(world, pos, ft) -> Entity` free helper (usable from inside a system, where combat text is usually spawned) + `App::spawn_floating_text` / `App::spawn_floating_text_colored` convenience wrappers for the app context. All re-exported from `engine`, alongside `DEFAULT_FLOAT_SPEED` / `DEFAULT_FLOAT_LIFETIME` / `DEFAULT_FLOAT_SIZE`.
- Example `floating_text` — Space pops colored numbers over three targets, left-click pops one at the cursor, headless auto-fires on a timer (`HEADLESS_SHOT` capture). 6 unit tests (rise+despawn, alpha fade, no-fade, camera projection, no-camera fallback, accessors) + 2 doctests.

### Notes
- `FloatingText` is deliberately **not** registered as an editor add/remove component: because the system despawns its own entity when the text expires, hand-authoring one on an existing entity in the editor would silently despawn that entity. Spawn a dedicated ephemeral entity for it (as `spawn_floating_text` / `App::spawn_floating_text` do). Precedent: `Timer` is likewise clone-registered but not editor-added.

## 0.103.0

**The docked editor's right-click context menu now works on the Scene tree too — plus a new "＋ Add child".** Right-clicking a Scene-tab tree node opens the same Rename / Duplicate / Focus camera / Delete menu the Entities list already had, and adds a Scene-tree-only **＋ Add child** that spawns a fresh entity parented under the node (then selects it) — the first context-menu path that *creates* a hierarchy relationship. The menu markup is now shared between the two tabs. Additive, native-only, no public API.

### Added
- `EntityContextAction::AddChild` + its dispatch in `App::editor_apply_entity_context_action` (`src/app/editor/ui/docked.rs`) — spawns a `Transform`+`Tag("New Entity")` entity (the same shape as the "＋ New Entity" toolbar button), parents it under the right-clicked node via the cycle-safe `crate::hierarchy::reparent`, and selects the new child (so it does **not** pre-select the parent the way the selection-scoped ops do). Not undoable, matching the New Entity button. 2 unit tests (spawns + parents + selects the child; a dead target is a no-op).
- `scene_tab_body` (`src/app/editor/ui/docked.rs`) attaches the context menu to each tree node's response (collect-then-apply, like the existing drag-to-reparent drop — the menu closure only records `(entity, action)`, applied after the tree is drawn). A secondary-click doesn't disturb the primary-drag reparent source.

### Changed (internal)
- Extracted `docked.rs::entity_context_menu(ui, entity, add_child, out)` — one shared helper drawing the Rename/Duplicate/Focus/Delete buttons (and the Scene-tree-only ＋ Add child) — replacing the inline menu that lived in `entities_tab_body`. Both the Entities list (`add_child = false`) and the Scene tree (`add_child = true`) call it, so the two menus can't drift.

## 0.102.0

**Right-click context menu on the docked editor's Entities list.** Right-clicking a row now opens a menu with Rename / Duplicate / Focus camera / Delete — the same operations available from the toolbar and keyboard shortcuts, surfaced per-row and on the entity you clicked. Additive, native-only, no public API; every action reuses an existing, already-tested op.

### Added
- `EntityContextAction` + `App::editor_apply_entity_context_action(entity, action)` (`src/app/editor/ui/docked.rs`) — the menu buttons only record `(entity, action)`; the dispatch selects that entity first (so a duplicate/delete/focus acts on the right-clicked row, not the prior selection), then runs the matching op (`editor_begin_rename` / `editor_duplicate_selection` / `editor_focus_camera_on_selection` / `editor_delete_selection`). A dead entity is a no-op. 4 unit tests (rename seeds + selects, delete acts on the clicked row not the old selection, duplicate clones + reselects, dead-entity no-op).
- `entities_tab_body` (`src/app/editor/ui/docked.rs`) attaches the `egui` context menu to each row (collect-then-apply, so the menu closure never mutates `App` mid-iteration).

## 0.101.0

**The docked editor's Entities list can be sorted by name or by kind.** A `Sort:` toggle above the list orders the displayed rows — Default (raw insertion order), A–Z (case-insensitive by label), or Type (grouped by the same kind classification as the per-row type-icon, then by name). The sort is display-only: it reorders a copy for the panel, so the world's entity order and the scene-save order are untouched. Additive, native-only, no public API; Default reproduces the previous list exactly.

### Added
- `EntitySortMode` (`src/app/editor/state.rs`, `Insertion`/`Name`/`Kind`) + `EditorState::entity_sort` (transient, not persisted) — the chosen order for the Entities tab, driven by a three-way toolbar toggle in `entities_tab_body`.
- `sorted_entity_list(entity_list, mode, world, tag_map)` (`src/app/editor/ui/docked.rs`) — returns a display-only sorted copy; a stable sort, so within a group equal keys keep insertion order. 3 unit tests (Insertion preserves raw order, Name is case-insensitive A–Z, Kind groups Light→Sprite→Transform→Bare then name).

### Changed (internal)
- `entity_type_icon` now delegates to a shared `entity_kind(world, entity) -> EntityKind` classifier (`src/app/editor/ui/docked.rs`); `EntityKind::icon()` maps a kind to its glyph and the enum's variant order backs the Kind sort — so the type-icon and the "sort by type" grouping can never drift apart. The rendered glyphs are unchanged (existing 6 icon unit tests still pass).

## 0.100.0

**Inline entity rename now works from the docked editor's Scene tree, not just the Entities list.** Double-clicking a node in the Scene tab replaces its label with a focused text box (Enter/click-away commits `Tag`, Escape cancels) — the same in-place rename shipped for the Entities list in 0.96.0, reusing the same shared rename state and commit/cancel path. Additive; native-only; no new public API (the existing `App::editor_begin_rename` already drives both views).

### Changed
- `scene_tab_body` (`src/app/editor/ui/docked.rs`) draws a focused text box for a node whose entity is being renamed (bound to the shared `EditorState::entity_rename` buffer) instead of the draggable label, and starts a rename on a node's `double_clicked()`. The rename text box is deliberately **not** wrapped in the `dnd_drag_source`, so typing or dragging within the field never starts a reparent DnD; a non-renaming node is unchanged (draggable, click-to-select). New render test `editor_docked_scene_tree_rename_renders_headless` drives the path headlessly (nest a node, show the Scene tab, begin a rename, capture).

## 0.99.0

**Per-row type-icon in the docked editor's entity views.** The docked editor's Entities list and Scene tree each show a small glyph before an entity's label hinting its "kind", derived from its most salient component so an entity's type is legible at a glance without opening the Inspector. Additive and native-only; no public API (a private helper), a pure `world.get` scan that never mutates, and no behavior change to any existing panel.

### Added
- `entity_type_icon(world, entity) -> &'static str` (`src/app/editor/ui/docked.rs`, private helper) — maps an entity to a one-glyph kind hint in priority order (first match wins): `PointLight` 💡, `Tilemap` 🗺, `ParticleEmitter` ✨, `CameraTarget` 🎥, `AnimationPlayer`/`AnimationStateMachine` 🎬, a UI widget (`UiNode`/`Button`/`Label`/`TextInput`/`Slider`/`CheckBox`/`Panel`) 🔘, the sprite family (`Sprite`/`AtlasSprite`/`NineSlice`/`ShaderMaterial`) 🖼, a transform-only entity 🔹, and a bare/marker-only entity ·. Priority means a light that also carries a sprite still reads as a light. Glyphs are chosen from egui's bundled emoji set (verified to render — not □ tofu — via the headless docked capture). 6 unit tests cover the mapping and the priority.

### Changed
- `entities_tab_body` and `scene_tab_body` (`src/app/editor/ui/docked.rs`) draw the icon before each row's label (the Entities list adds a weak-styled label between the eye toggle and the name; the Scene tree injects the glyph into the node's display string). The eye/visibility toggle, inline rename, drag-to-reparent, selection, and every other panel are unchanged.

## 0.98.0

**Editor drag-to-reparent is now undoable.** The Scene-tree drag-to-reparent shipped in 0.97.0 recorded no undo step, so a mis-drag couldn't be reverted with Ctrl+Z (unlike the gizmo move/resize/rotate, which do record undo). A reparent now pushes an `EditorCmd::Reparent` onto the existing editor history: Ctrl+Z restores the entity's previous parent and Ctrl+Shift+Z re-applies the move, both through the cycle-safe `hierarchy::reparent`. A rejected drag (self / descendant-cycle / no-op) records nothing, so undo never replays a move that never happened. Additive — no API change to `App::editor_reparent`; the only behavioral change is that a successful reparent now leaves an undo entry.

### Changed
- `App::editor_reparent` (`src/app/editor/ui/reparent.rs`) reads the child's parent before the move and, on a real change, pushes an `EditorCmd::Reparent { entity, old_parent, new_parent }` onto `EditorHistory` (alongside the existing success toast). The Scene-tab drag wiring and `hierarchy::reparent` are unchanged.
- `EditorCmd::Reparent` (`src/app/editor/history.rs`) — new undo/redo variant; undo calls `hierarchy::reparent(entity, old_parent)`, redo calls `hierarchy::reparent(entity, new_parent)`, each selecting the moved entity. 3 unit tests (undo/redo restores parent, undo reattaches after detach-to-root, a rejected move records no undo).

## 0.97.0

**Scene-tree drag-to-reparent in the editor + a cycle-safe `hierarchy::reparent`.** The docked editor's Scene tab was a read-only hierarchy view; now dragging a node onto another re-parents it under that node, and dragging onto the bottom "⤴ unparent" zone detaches it to a root. The graph edit goes through a new public `hierarchy::reparent`, which adds cycle prevention on top of the low-level `attach`/`detach` (a drop onto self or a descendant, or a no-op move, leaves the graph untouched). Additive — the read-only tree, `attach`/`detach`, and every other panel are unchanged.

### Added
- `hierarchy::reparent(world, child, new_parent: Option<Entity>) -> bool` (`src/hierarchy.rs`, public, re-exported at crate root) — detach + cycle-checked attach; returns `false` and makes no change for a self-parent, a descendant target (would cycle), or a no-op (same parent / detaching a root). Keeps the child's local `Transform`, like `attach`. 6 unit tests.
- `App::editor_reparent(child, new_parent)` (`src/app/editor/ui/reparent.rs`, public) — drives `hierarchy::reparent` from the editor and shows a success toast on a real change; native-only. 3 unit tests.
- `App::editor_show_scene_tree()` (`src/app/headless.rs`, public) — switch the docked left panel to the Scene tab (for headless capture of the hierarchy view).
- `tests/render.rs::editor_docked_scene_tree_reparent_renders_headless` — reparent + show the Scene tab + capture, asserting the tree composites (lavapipe-safe).

### Changed
- `scene_tab_body` (`src/app/editor/ui/docked.rs`) wraps each tree node in an egui `dnd_drag_source` (the OR'd inner `selectable_label` keeps click-to-select) and treats each node as a drop target (`dnd_release_payload`); a bottom `dnd_drop_zone` detaches to a root. Drops call `App::editor_reparent`. New `DragEntity(Entity)` DnD payload.

## 0.96.0

**Inline entity rename in the editor entity list.** Double-clicking a row in the docked editor's Entities list now replaces its label with a focused text box; Enter or clicking away commits the new name (`Tag`), Escape cancels. Previously renaming meant selecting the entity first and using the separate "Name:" field — this edits in place. A blank or whitespace-only name, or a despawned entity, cancels instead of overwriting. Additive (the "Name:" field and all other rows are unchanged); the only new public API is `App::editor_begin_rename`, so pre-1.0 additive → MINOR.

### Added
- `App::editor_begin_rename(entity)` (`src/app/editor/ui/rename.rs`, public) — start an inline rename programmatically (the same state a list-row double-click produces); seeds the buffer from the entity's current `Tag` and requests one-frame keyboard focus. Native-only.
- `EditorState::entity_rename` / `EntityRename { entity, buffer, focus_pending }` (`src/app/editor/state.rs`) — transient edit state (not serde, like the other editor markers); cleared when its entity is despawned.
- `tests/render.rs::editor_docked_inline_rename_renders_headless` — drives the in-edit text box headlessly (lavapipe-safe) and asserts the left panel still composites.

### Changed
- `entities_tab_body` (`src/app/editor/ui/docked.rs`) renders a text box for the renaming row (Enter/click-away → `App::editor_commit_rename`, Esc → `editor_cancel_rename`) and starts a rename on a row double-click; non-renaming rows gain a "double-click to rename" hover hint. Internal `App::editor_commit_rename` / `editor_cancel_rename` hold the commit/cancel behavior. 7 unit tests in `rename.rs` (buffer seeding, trim, blank-cancel, despawn-noop, cancel-discard).

## 0.95.0

**Editor prefab save/spawn now surface action toasts.** Saving the selected entity as a prefab, or spawning a prefab, in the docked editor previously only set the inline `prefab_status` label; both now also push a colour-coded action toast (success/error) like the scene-save action, completing the toast coverage of the editor's file actions. Editor-only behavior; no public API change.

### Changed
- `App::save_selected_as_prefab` / `App::spawn_prefab` (`src/app/editor/prefab.rs`) push an `App::push_editor_toast` (`ToastKind::Success` / `ToastKind::Error`) in addition to setting `prefab_status`, mirroring `do_save_scene_with_list`'s feedback.
- Unit test `prefab_save_and_spawn_push_toasts` (`src/app/editor/tests.rs`): a missing-file spawn toasts `Error`; a successful save toasts `Success`.

## 0.94.0

**Docked-mode headless editor capture — verify docked-panel editor UI with no window/display.** The headless editor screenshot could only draw the *overlay* editor (cheatsheet/toasts); the **docked** layout — entity list, inspector, Data Tables, and the game scene in the central viewport — was invisible to it, so docked-panel UI (e.g. the entity-list eye toggle) couldn't be visually verified or golden-image tested. New `App::screenshot_editor_docked_headless[_rgba]` enter docked mode before driving egui, so the full docked editor composites onto the offscreen texture with no window (monitor off / asleep / locked / CI lavapipe). **All additive; the overlay path is unchanged.** Pre-1.0 additive → MINOR.

### Added
- `App::screenshot_editor_docked_headless(frames, path)` / `screenshot_editor_docked_headless_rgba(frames)` (`src/app/headless.rs`) — render the full docked editor layout headlessly. The central game viewport is a debounced offscreen RT, so `frames >= 5` shows the scene; fewer frames render the side panels with a placeholder central. Native-only.
- `App::editor_select_entity(entity)` (`src/app/headless.rs`) — public editor-selection API (sets the Inspector selection + sole multi-selection), so a headless capture or a game can populate the Inspector programmatically. Closes the documented-but-missing "select an entity to populate the Inspector" gap.
- Render test `tests/render.rs::editor_docked_renders_headless` (lavapipe-verified: the docked left panel renders bright UI text headlessly) + example `editor_docked_headless_shot` (entity list with the eye toggle + a Hidden quad + the scene in the central viewport).

### Changed (internal)
- The overlay/docked headless captures now share one `App::editor_headless_capture(frames, docked)` driver (`src/app/headless.rs`); `screenshot_editor_headless_rgba` is a thin wrapper over it. The shared `tests/render.rs::editor_render_or_skip` helper gained a `docked` flag. Behavior-preserving for the existing overlay path.

## 0.93.0

**Entity visibility — a `Hidden` component + an editor per-entity eye toggle.** The editor entity list gains a per-row visibility toggle (👁/🙈) that hides/shows an entity's sprite without despawning it; the engine side is a new `Hidden` marker component the sprite pass skips, also usable directly by games. **All additive; absent `Hidden` = byte-identical.** Pre-1.0 additive → MINOR.

### Added
- `engine::Hidden` (`src/components.rs`) — marker component; when present, the sprite collect pass (`src/renderer/sprite/collect.rs`, all three loops: `Sprite` incl. `NineSlice` / `AtlasSprite` / `ShaderMaterial`) skips the entity, suppressing its sprite-family rendering. Does **not** affect particles/lights/screen-space text. Registered for clone + editor add/remove like `SpriteFlip`.
- Editor entity list: a per-row **visibility toggle** (eye 👁 visible / 🙈 hidden) that adds/removes `Hidden` and dims hidden rows (`src/app/editor/ui/docked.rs`).
- Render test `tests/render.rs::hidden_component_suppresses_sprite` (lavapipe-verified: a `Hidden` quad does not render) + unit test `hidden_registered_as_editor_component`.

## 0.92.0

**Editor action toasts — transient, colour-coded feedback popups.** Editor actions used to write a status string into a panel where it's easy to miss; now they also surface a bottom-right toast ("Scene saved (3)", "Deleted 2", "Load failed: …") that auto-expires and fades out. Verified headlessly with the v0.91.0 headless editor-screenshot path. **All additive; editor is native-only.** Pre-1.0 additive → MINOR.

### Added
- `src/app/editor/ui/toasts.rs` — `EditorState::toasts` + `Toast`/`ToastKind` (Info/Success/Error); `App::push_editor_toast(msg, kind)` (internal) + public `App::editor_toast` / `editor_toast_success` / `editor_toast_error`; `App::draw_editor_toasts` ages + draws a bottom-right stack (fading over the last ~0.6 s, queue capped at 5) in both overlay and docked modes.
- Wired to the keyboard/editor actions: delete ("Deleted N"), duplicate ("Duplicated N"), paste ("Pasted N"), and scene save (success "Scene saved (N)" / error "Save failed: …", covering both the toolbar button and Ctrl+S).
- Unit test `editor_toasts_push_and_cap_to_five` + render test `tests/render.rs::editor_toast_renders_headless` (lavapipe-verified). Example `editor_headless_shot` now also shows the colour-coded toast stack.

## 0.91.0

**Editor keyboard-UX upgrade + a headless editor-screenshot capability to verify it without a display.** The in-game editor gains standard keyboard shortcuts and a discoverable cheatsheet; and because the egui editor overlay is normally driven by the windowed loop (so a plain headless screenshot can't capture it), a new path drives egui manually with no window — making editor UI visually verifiable with the monitor off/locked and on CI (lavapipe). **All additive; the editor is native-only, so nothing wasm-facing changes.**

### Added — editor keyboard UX
- Editor shortcuts (`src/app/editor/ui/shortcuts.rs`): **Ctrl+S** save scene, **Ctrl+D** duplicate selection, **Delete**/**Backspace** delete selection, **F** focus camera on the selection, **?** toggle a cheatsheet (alongside the existing Ctrl+Z/Shift+Z/C/V). Bare single-key shortcuts are gated on `egui_wants_keyboard_input()` so typing in a text field never triggers them.
- Selection-scoped editor ops factored into `App::editor_delete_selection` / `editor_duplicate_selection` / `editor_focus_camera_on_selection` (multi-select aware, each recorded for undo). Undo/redo/paste logic extracted into helpers.
- A **keyboard-shortcuts cheatsheet** window (`EditorState::show_shortcuts`), toggled by `?` or a new **`? Keys`** toolbar button; localized (English/Korean) like the rest of the editor.

### Added — headless editor screenshot
- `App::screenshot_editor_headless(frames, path)` / `screenshot_editor_headless_rgba(frames)` (`src/app/headless.rs`) render the egui **editor overlay** to an offscreen texture with **no window/display** — driving egui manually (synthesized `RawInput` → `App::update_editor_ui` → tessellate → the render path's egui pass), with a fresh `DebugUi`/egui `Context` and an offscreen-format `egui_wgpu::Renderer`. Native-only.
- `App::set_editor_shortcuts_visible(bool)` opens the cheatsheet programmatically.
- Example `editor_headless_shot` and render test `tests/render.rs::editor_overlay_renders_headless` (runs on the GPU-less CI runner via lavapipe), so the editor UI is now CI-verifiable.

## 0.90.0

**Richer effect payload — per-effect spawn offset.** `Effect::SpawnParticles` gains an `offset: (x, y)` field — a world-space displacement added to the burst's anchor position, so a data-authored burst can spawn off-center (e.g. footstep dust at the feet, not the entity center) instead of always at the anchor's `Transform`. The field is shared by both effect sources (`zone_effect` + `anim_effect`) since it lives on the common `Effect` vocabulary. **Additive, non-breaking** — `#[serde(default)]` makes `offset` default to `(0.0, 0.0)`, which is byte-identical to the prior behavior (existing RON and the old burst position are unchanged).

### Added
- `Effect::SpawnParticles.offset: (f32, f32)` (`src/effect.rs`) — world-space `(x, y)` offset added to the anchor's `Transform` position in `resolve_effect` (`pos = position + offset`); not rotated/scaled by the transform. Default `(0.0, 0.0)`.
- `examples/anim_effects` now spawns the footstep dust at the walker's **feet** via the new `offset` (`anim_effects.ron`: `SpawnParticles(particles: "dust", count: 18, offset: (0.0, 70.0))`), with `＋ center` / `◦ feet` HUD guides showing the displacement. Closes the prior "dust spawns at the entity center" limitation noted when `anim_effect` shipped (0.89.0).
- Unit test `anim_effect::tests::spawn_particles_offset_displaces_burst` (burst Transform = anchor + offset); `zone_effect`'s default-parse test now also asserts `offset` defaults to `(0.0, 0.0)`.

## 0.89.0

**Data-driven Animation→Effect bindings — fire RON-authored effects on tagged animation frames.** The companion to Zone→Effect bindings (0.88.0): where that reacts to a zone overlap, this reacts to an `AnimationEvent` — an `AnimEffectBindings` table maps a frame's `tag` (e.g. `"footstep"`) to a bare `Vec<Effect>` (no phase — an animation event is an instantaneous fire), and a user-added `AnimEffectSystem::new(name)` (run after `AnimationSystem`) reads each event, looks up its tag's effects, and applies them anchored at the animating entity. The shared `Effect` vocabulary (`SpawnParticles` / `PlayTone` / `Flash`) + its application were extracted into a new `engine::effect` module and are reused directly — so animation reactions, the particles they fire, and the animation itself can all live in RON. **Additive types + one `App` method; the shared-module extraction is behavior-preserving with no public API change** (`engine::Effect`/`engine::EffectAnchor` keep their crate-root paths).

### Added
- `engine::AnimEffectBindings` — `from_ron_str(s)` (cross-platform), `effects_for(tag)`, `len`/`is_empty`; the tag-keyed `HashMap<String, Vec<Effect>>` table.
- `engine::AnimEffectRegistry` (World resource) + `engine::AnimEffectError`. Auto-registered as hot-reloadable, so loaded files reload on edit (native).
- `App::load_anim_effects(name, path)` — lazily inserts the registry + watches the file; no-op on wasm.
- `engine::AnimEffectSystem::new(name)` — user-added system applying a named binding table to incoming `AnimationEvent`s (add after `AnimationSystem`); anchors `SpawnParticles` + `Flash` at the animating entity (`EffectAnchor` is not consulted).
- Example `anim_effects` — a walk cycle whose contact frames fire a dust burst + a click tone + a warm flash, all authored in RON (`HEADLESS_SHOT` supported).

### Changed (internal)
- Extracted the shared `Effect` / `EffectAnchor` vocabulary and the effect-application machinery (`resolve_effect` / `apply_pending` / `lookup_emitter`) out of `zone_effect` into a new `engine::effect` module, so both `zone_effect` and `anim_effect` reuse one implementation. **No public API change** — `engine::Effect` and `engine::EffectAnchor` keep their crate-root re-export paths; `zone_effect` is behavior-identical (its tests are unchanged).

## 0.88.0

**Data-driven Zone→Effect bindings — author *what happens* on a trigger zone in RON.** The companion to data-driven trigger zones (0.87.0): a `ZoneEffectBindings` table maps a zone's `Tag` name to rules, each a `ZonePhase` (Entered/Stayed/Exited) paired with an `Effect`. The supported effects are the three game-feel reactions the engine already produces — `SpawnParticles` (a one-shot burst reusing a named `ParticleConfigRegistry` emitter, anchored at the entering entity or the zone), `PlayTone` (the cross-platform `Audio` facade), and `Flash` (a `HitFlash` on the entrant). A user-added `ZoneEffectSystem::new(name)` (run after `TriggerZoneSystem`) reads each `ZoneEvent`, resolves the zone's `Tag` to a key, and applies the matching rules — so a level's zones, the particles they fire, and their reactions all live in RON, composed by tag, with no Rust glue. Mirrors the existing config-set pattern (registry + native hot-reload, auto-registered); the effect vocabulary is deliberately ZoneEvent-focused (an `AnimationEvent`→effect binding is a natural future reuse of `Effect`). **Additive types + one `App` method; no breaking change.**

### Added
- `engine::Effect` (enum: `SpawnParticles { particles, count, at }` / `PlayTone { freq, dur, vol, bus }` / `Flash { color, secs }`), `engine::EffectAnchor` (`Other`/`Zone`), `engine::ZonePhase` (`Entered`/`Stayed`/`Exited`), `engine::ZoneEffectRule { on, effect }`.
- `engine::ZoneEffectBindings` — `from_ron_str(s)` (cross-platform), `rules_for(tag)`, `len`/`is_empty`; the tag-keyed binding table.
- `engine::ZoneEffectRegistry` (World resource) + `engine::ZoneEffectError`. Auto-registered as hot-reloadable, so loaded files reload on edit (native).
- `App::load_zone_effects(name, path)` — lazily inserts the registry + watches the file; no-op on wasm (parse with `ZoneEffectBindings::from_ron_str` + `ZoneEffectRegistry::insert` there).
- `engine::ZoneEffectSystem::new(name)` — user-added system applying a named binding table to incoming `ZoneEvent`s (add after `TriggerZoneSystem`).
- Example `zone_effects` — heal/damage/goal zones, particle emitters, and effect bindings all authored in three RON files, composed by tag (`HEADLESS_SHOT` supported).

## 0.87.0

**Data-driven trigger zones — author a level's zones in RON, load + spawn them.** The code-built `TriggerZone` (0.84.0) now has a data-driven counterpart: describe a set of zones in a RON file — each with a `tag` (name), `pos`, `shape` (`Circle`/`Rect`), and `mask` — load it with `App::load_trigger_zones(name, path)` (registry + native hot-reload, auto-registered like the particle/dialogue config registries), and spawn it with `App::spawn_trigger_zones(name)`. Each entry becomes an entity with a `Transform`, a `TriggerZone`, and (when tagged) a `Tag`, so a game reacts to a `ZoneEvent` by reading the zone entity's `Tag` and can add/move/retune zones by editing the RON without touching code. Mirrors the existing config-set pattern (private serde-mirror types), so no serde is added to the runtime `TriggerShape`/`CollisionLayer`. **Additive types + two `App` methods; no breaking change.**

### Added
- `engine::TriggerZoneSet` — `from_ron_str(s)` (cross-platform), `len`/`is_empty`/`tags`, and `spawn_into(world) -> Vec<Entity>` (one entity per def: `Transform` + `TriggerZone` + a `Tag` when the def has a non-empty tag; zones carry no `Sprite` — the game draws them).
- `engine::TriggerZoneRegistry` (World resource) + `engine::TriggerZoneSetError`. The registry is auto-registered as hot-reloadable, so loaded files reload on edit (native).
- `App::load_trigger_zones(name, path)` (lazily inserts the registry + watches the file; no-op on wasm) and `App::spawn_trigger_zones(name) -> Vec<Entity>` (cross-platform).
- Example `data_trigger_zones` (+ `examples/data_trigger_zones.ron`) — the heal/damage/goal walk from `trigger_zones`, but the zones are loaded from RON and spawned; the example sizes a debug quad from each zone's shape and resolves `ZoneEvent`s to names via `Tag`. `HEADLESS_SHOT` supported (auto-drifts the player; defaults to 130 warmup frames).
- Unit tests: parse + spawn (entities get the right shape/mask/Tag, untagged → no Tag, mask defaults to all), malformed RON → Err, a spawned zone detects overlap end-to-end, and registry insert/get/names.

## 0.86.0

**Camera motion lookahead — bias the camera ahead of a moving follow target.** With a plain smooth-follow the camera centers on the target, so a fast-moving player sees as much behind them as ahead. Set `Camera::lookahead` (world units) and the camera now leads the view in the target's direction of motion: `Camera::update` derives the target's velocity from the change in follow position, eases an offset toward `direction * lookahead` (at `Camera::lookahead_speed`), and aims the follow at `position + offset` — so the player sees more of where they are going, and the view recenters when they stop. It is entirely internal to `Camera` (the App's per-frame `Camera::update` call site is unchanged), and `lookahead == 0` (the default) is byte-identical to a direct follow. **Additive `Camera` fields + one accessor; no breaking change.**

### Added
- `Camera::lookahead` (world units, default `0.0` = disabled) and `Camera::lookahead_speed` (default `DEFAULT_LOOKAHEAD_SPEED` = 3.0, re-exported) public fields, plus the `Camera::lookahead_offset()` accessor returning the current smoothed offset (`Vec2::ZERO` when disabled or stationary).
- Example `camera_lookahead` — a player crosses a field of posts with a camera-anchor follow; a screen-space center line shows the lead (a player moving right sits left of center with lookahead on, re-centers when off). `Space` toggles lookahead; `HEADLESS_SHOT` auto-drifts the player so the capture shows the lead (defaults to 120 warmup frames).
- Unit tests: lookahead-off is byte-identical to a plain follow, the offset leads in the direction of motion (bounded by `lookahead`), and it recenters when the target stops.

## 0.85.0

**Hit-flash — flash a sprite when it is hit, then fade it back.** Add a `HitFlash` component to an entity with a `Sprite` and the engine pops the sprite to a flash color (white by default) and eases it back to its original color over a short duration, then removes the component — the game-feel staple every action game re-implements. `HitFlashSystem` (user-added, like `YSortSystem`) drives it: it captures the sprite's pre-flash color on the **first** run (so it never needs to know the color in advance), `Lerp`s from the flash color back to that base over `secs`, and removes the component when finished (restoring the original color exactly). It pairs naturally with a damage event — a `ZoneEvent` from a damage `TriggerZone`, a `CollisionEvent`, or an animation event — by adding a `HitFlash` to the hit entity. `HitFlash` carries no serde (it is a transient runtime effect, like `TriggerZone`); it is clone- and editor-add/remove-registered like `YSort`. **Additive component + opt-in system; no breaking change.**

### Added
- `engine::HitFlash { color, secs }` component (plus private `elapsed`/`base` runtime state) — ctors `white(secs)` (the common white "hit" pop) / `new(color, secs)`, and read accessors `progress()` (0→1) / `is_finished()` for polling. `secs <= 0` restores and removes the same frame. Clone- and editor-add/remove-registered like `YSort`.
- `engine::HitFlashSystem` (user-added, like `YSortSystem`) — `query2_mut::<HitFlash, Sprite>`, fades each flashing sprite back to its captured base color and removes the `HitFlash` when the flash finishes. A `HitFlash` on an entity without a `Sprite` is inert.
- Example `hit_flash` — **Space** flashes three resting-colored targets white and they fade back to their own colors. `HEADLESS_SHOT` supported: with no input the targets continuously re-flash on slightly staggered durations so a capture always lands mid-flash (defaults to 90 warmup frames).
- Unit tests: fade from flash color back to base then auto-remove, flash color dominates at the start, base captured at first run (not construction), zero-duration restores+removes immediately, a sprite without `HitFlash` is untouched, and the `progress`/`is_finished` accessors.

## 0.84.0

**Trigger zones — Area2D-style enter/stay/exit zones.** Add a `TriggerZone` component (a circle or rectangle, centered on the entity's `Transform`) and the engine reports every entity that enters, stays in, or leaves that area — the idiomatic 2D pattern for pickups, damage fields, room/door triggers, aggro ranges, and checkpoints, without hand-rolling per-frame overlap polling. `TriggerZoneSystem` reads the `SpatialGrid` built by `CollisionGridSystem`, diffs each zone's current overlap set against last frame's, and sends a `ZoneEvent` (`Entered`/`Stayed`/`Exited { zone, other }`) on `Events<ZoneEvent>`; the current occupant set also lives on the component (`TriggerZone::occupants` / `contains`) for direct polling. Watched entities are the ones already in the grid (`Transform` + `Collider` [+ a matching `CollisionLayer`]); a zone never reports itself. The event type is named **`ZoneEvent`**, deliberately distinct from the physics `TriggerEvent` (rapier sensors) — this is a lightweight, grid-based, physics-free zone. **Additive component + opt-in system + new event type; no breaking change.**

### Added
- `engine::TriggerZone { shape, mask, occupants }` component — ctors `circle(radius)` / `rect(half_extents)`, builder `with_mask(CollisionLayer)`, and `contains(entity)` for polling; `engine::TriggerShape` (`Circle { radius }` / `Rect { half_extents }`). Clone- and editor-add/remove-registered like `YSort`.
- `engine::TriggerZoneSystem` (user-added, like `YSortSystem`) — add it **after** a `CollisionGridSystem`. If the `SpatialGrid` resource or `Events<ZoneEvent>` bus is missing it warns once and continues (occupants still update for polling), so events are opt-in.
- `engine::ZoneEvent` enum — `Entered` / `Stayed` / `Exited { zone, other }`, emitted on `Events<ZoneEvent>` (register with `App::register_event::<ZoneEvent>()`).
- Example `trigger_zones` — a player walks left→right through a heal / damage / goal zone; the **event** stream drives an entry counter while **polling** `occupants` tints each zone as it is occupied (showing both consumption styles). `HEADLESS_SHOT` supported (auto-drifts the player; defaults to 130 warmup frames).
- Unit tests: enter/stay/exit lifecycle, layer-mask filtering, rect-shape overlap, a zone never reports itself, unregistered-bus drops events but still updates occupants and warns once, and missing-grid warns once + stays inert.

## 0.83.0

**Animation events — fire a tagged event when a playhead enters a frame.** Add an `AnimationEvents` component next to an `AnimationPlayer` and list `(clip, frame, tag)` triples; when `AnimationSystem` advances the playhead **onto** a listed frame it sends an `AnimationEvent` on `Events<AnimationEvent>`, the idiomatic way to drive footsteps / attack hit-frames / VFX off an animation without hand-polling `current_frame`. This is a **separate component, not a field on `AnimationClip`**, so every existing clip and player keeps working unchanged (the 26 existing `AnimationClip { … }` literals are untouched) and a game can attach different event sets to entities that share the same clip data. Events fire on frame **transitions** only — the initial frame at spawn never fires, a looping clip re-fires frame 0 on each wrap, and during a crossfade only the outgoing clip emits. **Additive component + new event type; no breaking change.**

### Added
- `engine::AnimationEvents { events: Vec<FrameEvent> }` component (builder `new().on(clip, frame, tag)`; `Clone`/`Default`/`serde`), `engine::FrameEvent { clip, frame, tag }`, and `engine::AnimationEvent { entity, clip, frame, tag }` emitted on `Events<AnimationEvent>` (register with `App::register_event::<AnimationEvent>()`; events are dropped with a one-time warning if the bus is unregistered). Clone- and editor-add/remove-registered like `RenderLayer`.
- `AnimationSystem` now records the frames a player's playhead transitions onto each tick and emits matching events after the per-entity advance — zero allocation on the hot path (no `AnimationEvents` component / no frame advance → no work).
- Example `animation_events` — a 4-frame walk cycle with `"footstep"` events on the two contact frames; a reader system counts the steps, flashes "STEP!", and logs each one. `HEADLESS_SHOT` supported (defaults to 60 warmup frames so the cycle advances).
- Unit tests: builder/serde round-trip, event fires on frame entry (incl. multi-frame jumps), non-matching frame emits nothing, initial frame does not fire but loop-wrap does, unregistered bus drops + warns without panic, entity without the component emits nothing.

## 0.82.0

**Top-down depth sorting — `YSort` + `YSortSystem`.** Add a `YSort` component and the engine depth-orders the sprite by its world Y so an entity lower on the screen (Y increases downward) overlaps one higher up — the standard top-down "draw nearer things in front" rule. `YSortSystem` writes `Transform.z = position.y + bias` each frame; the sprite renderer already sorts by `(layer, z)` (higher z in front), so ordering is correct within a `RenderLayer`. Previously a top-down game had to compute per-sprite z by hand. **Additive component + opt-in system; no breaking change.**

### Added
- `engine::YSort { bias }` component (`new(bias)`; `Copy`/`Default`/`serde`) + `engine::YSortSystem` (user-added, like `ParallaxSystem`). `bias` shifts the sort point along Y (e.g. +half the sprite height to sort by the feet). Clone- and editor-add/remove-registered.
- Example `ysort` — three overlapping "trees" + a player that weaves in front of/behind them with **↑/↓**; **Space** toggles sorting off (sprites fall back to spawn order — the overlap bug Y-sort fixes). `HEADLESS_SHOT` supported.
- Unit tests: `z = y + bias`, non-`YSort` entities untouched, lower entity gets the higher z.

## 0.81.0

**Sprite flipping — `SpriteFlip` component.** Add `SpriteFlip { x, y }` to any sprite-bearing entity to mirror it horizontally / vertically at render time. The renderer flips the sampled UV region in place, so it works uniformly across plain `Sprite`, `AtlasSprite`, and `ShaderMaterial` — including animation frames and atlas tiles — with no texture swap and without negating `Transform::scale` (negative scale also mirrors lighting/children and breaks rotation). This closes a genuine breadth gap: a downstream game previously had to hand-roll UV flipping in game code to face a character by movement direction. Absent or `default()` (no flip) renders byte-identically. **Additive component; no breaking change.**

### Added
- `engine::SpriteFlip` component (`{ x: bool, y: bool }`) — ctors `horizontal()` / `vertical()` / `NONE` + `is_flipped()`; derives `Copy`/`Default`/`Serialize`/`Deserialize`; clone- and editor-add/remove-registered like `RenderLayer`.
- `UvRect::flipped(flip_x, flip_y)` — composes the existing `flipped_x` / `flipped_y` primitives (previously unused); `flipped(false, false)` is a no-op.
- Example `sprite_flip` — a fixed reference vs. a controllable copy of a 4-quadrant texture, toggled with **H** / **V** (`HEADLESS_SHOT` supported).
- Unit tests: `UvRect::flipped` composition / no-op; covered by the sprite render path.

### Changed
- `renderer/sprite/collect.rs` applies a present `SpriteFlip` to the sampled UV in all three sprite paths (plain `Sprite` incl. crossfade target frame, `AtlasSprite`, `ShaderMaterial`). **`NineSlice` is excluded** (a 9-patch has no meaningful mirror).

## 0.80.1

**Tier-3 cleanup: named constants for duplicated default magic numbers.** Behavior-preserving (value-identical) dedup of the genuinely duplicated literals flagged by `docs/HARDCODING_AUDIT_2026-06-26.md` — no API change beyond two additive default constants, no behavior change. Two audit entries were re-classified as *not* real duplications and deliberately left (see below).

### Changed (internal)
- `DEFAULT_WINDOW_WIDTH` / `DEFAULT_WINDOW_HEIGHT` (`resources/display.rs`, re-exported from `engine::resources`) — single source of truth for `WindowConfig::default` and `ViewportSize::default`, which both hardcoded `1280×720` separately (cross-struct drift risk).
- `UI_SUBLAYER_Z_STEP` (`ui/system.rs`) — the `0.001` z-step shared by the checkbox and slider passes.
- `MIN_AUDIO_DURATION_SECS` (`audio.rs`) — the fade min-duration floor shared by `audio/types.rs` and `audio/bus.rs`.
- Deliberately **not** deduped (would be semantically wrong): the other `0.001` sites in `src/audio/` are comparison epsilons of different units (seconds vs `pan`/`pitch` ratios); `64*1024` in `network/event.rs` (WS message cap) and `scripting.rs` (Rhai string limit) are unrelated values that coincidentally match.

## 0.80.0

**Configurable native socket read timeout — `NetworkConfig::read_timeout`.** The native `NetworkClient` runs its WebSocket on a background thread that wakes on a socket read timeout to flush queued outbound messages; that interval was a hardcoded `const READ_TIMEOUT = 5ms` in `network/native.rs`. It is now a `NetworkConfig` field (default `DEFAULT_READ_TIMEOUT` = 5ms, so behavior is unchanged), letting a latency-sensitive game poll the socket more often (sends flush sooner, slightly more CPU) or a relaxed one poll less. Clamped to a 1ms floor (a zero timeout would block the thread). **No effect on WASM** (its receive path is event-driven), mirroring how `max_buffered_bytes` is WASM-only. Continues the hardcoding-audit Tier-2 fork-config-knob chain. **Additive field on an existing config struct; no breaking change.**

### Added
- `NetworkConfig::read_timeout: Duration` + `engine::network::DEFAULT_READ_TIMEOUT` (5ms). Passed via the existing `NetworkClient::connect_with_config`; a `NetworkConfig` doctest shows the usage.
- Unit tests: `read_timeout` default; a custom value carried via struct-update syntax; `connect_with_config` with a custom timeout constructs cleanly (native dead-port smoke).

### Changed
- `network/native.rs` — the client thread reads `config.read_timeout` (clamped `≥ 1ms`) instead of the hardcoded 5ms const for both plain-TCP and rustls-TLS sockets.
- Example `mp_client` now connects via `connect_with_config` with a 2ms `read_timeout` (snappier position updates) — the knob's usage site.

## 0.79.0

**Configurable analog-stick UI-navigation deadzone — `StickNavConfig`.** When a gamepad is connected, the left stick drives UI focus navigation the same way Tab / the D-pad do, edge-detected with hysteresis so one push = one focus step. The two hysteresis thresholds were hardcoded literals (`0.6` activate / `0.35` release); they are now an opt-in `StickNavConfig` resource (auto-inserted with those exact defaults, so behavior is **byte-identical** when left untouched). A game can retune the deadzone — tighter for snappier menus, wider for jitter rejection — without forking the UI system. Continues the hardcoding-audit Tier-2 fork-config-knob chain. **Additive opt-in resource; no breaking change.**

### Added
- `engine::StickNavConfig` (in `src/ui/focus.rs`) — `activate` / `release` thresholds + `resolved()` (clamps `release` into `[0, activate]` and `activate` into `[0, 1]` so a misconfigured pair can never invert the hysteresis band). Auto-inserted in `insert_core_resources`.
- `engine::DEFAULT_STICK_ACTIVATE` (0.6) / `engine::DEFAULT_STICK_RELEASE` (0.35) — the historical hardcoded thresholds, re-exported as named constants.
- Example `ui_nav_deadzone` — a focus list + a live bar visualizing the stick-X neutral/activate bands and marker, with runtime threshold tuning (`[` `]` / `,` `.`); `HEADLESS_SHOT` supported.
- Unit tests for a tighter custom deadzone firing earlier and for `resolved()` clamping invalid thresholds (`src/ui/system/state.rs`).

### Changed
- `src/ui/system/state.rs` — the internal `StickNav` edge detector (`update` / `step_axis`) now takes the resolved `(activate, release)` thresholds; the focus pass reads them from `StickNavConfig` each frame (default if absent). The module-private `STICK_ACTIVATE` / `STICK_RELEASE` consts moved to `focus.rs` as the public `DEFAULT_*` constants.

## 0.78.1

**CI-verifiable GPU render path.** CI is ubuntu-only with no GPU, so every render pass (sprite / text / lighting / letterbox) was exercised *only* by local macOS shell smokes — a shader, pipeline, or projection regression could pass all of CI and ship. A new `render` CI job installs Mesa **lavapipe** (a software Vulkan driver) so `wgpu` gets a CPU adapter and `tests/render.rs` renders the real path headlessly on the runner, asserting **renderer-tolerant** invariants (relative to a sampled background pixel, never absolute RGB — so the same assertions pass on local Metal and CI lavapipe). **No public API change**; no runtime behavior change (test + CI + docs only).

### Added
- `tests/render.rs` — headless render tests over the real render path via `App::screenshot_headless`: `red_quad_reads_red` (sprite color + placement), `hud_text_non_blank` (glyph pass, injects the bundled DejaVu Sans for runner-independent fonts), `lighting_cap_lights_more_when_raised` (`LightingConfig::max_lights` drives the GPU lighting pass), `design_resolution_letterboxes` (design-resolution scale+center projection). `render_or_skip` probes for an adapter and skips cleanly when none is present — unless `SKELETON_REQUIRE_GPU=1`, which makes a missing adapter a hard failure so CI can never silently no-op green.
- `render` job in `.github/workflows/ci.yml` (Mesa lavapipe via `mesa-vulkan-drivers`; ICD globbed into `VK_ICD_FILENAMES`; `vulkaninfo` sanity guard; `SKELETON_REQUIRE_GPU=1` + an `[render-test] adapter=` marker grep as two independent silent-skip guards).
- `docs/RENDER_TESTING.md` — the harness, the lavapipe CI setup, the renderer-tolerance rationale, and how to add a render test.

## 0.78.0

**Configurable animated-tile phase stagger — `TileAnimationSet::stagger`.** Animated tiles of the same value start out of phase by `(row + col) × frame_time × stagger` so neighbours don't sync-flash — but that stagger factor was a **hardcoded** `0.37` literal in `TilemapSystem`. It's now a field on `TileAnimationSet` (default `DEFAULT_TILE_ANIM_STAGGER` = 0.37, so existing maps are unchanged): set `0.0` to make every cell of a value animate in lockstep (a synchronized pulse), or a larger value for a more rippling spread. `TileAnimationSet`'s map is a private field, so the field add is non-breaking (constructed via `new()` + `insert`).

### Added
- `TileAnimationSet::stagger` field + `with_stagger(factor)` builder + `DEFAULT_TILE_ANIM_STAGGER` re-exported `pub const`.
- Example `tile_anim_stagger` (two identical animated grids cycling a colour ramp — left `with_stagger(0.0)` flat/lockstep, right default 0.37 showing a diagonal frame gradient; the difference is visible in a single frame).

### Changed (internal)
- The per-cell phase formula moved from an inline literal in `tilemap/system.rs` into a unit-tested `tilemap::animation::stagger_phase(row, col, frame_time, total_time, stagger)` helper. No behavior change at the default.

## 0.77.0

**Per-slider keyboard nudge step — `Slider::keyboard_step`.** A focused `Slider` moved by a **hardcoded** 5%-of-range step on each ←/→ (or D-pad / left-stick) press — wrong for a slider that selects a few discrete levels (5% of a `0..3` range = 0.15, landing *between* levels). `Slider::with_keyboard_step(s)` now overrides it with an absolute step in value units; the default (`None`) is byte-identical to before. `Slider` has a private field so it is only constructed via `Slider::new` + builders, making the field add non-breaking; the field carries `#[serde(default)]` so existing scene RON loads unchanged.

### Added
- `Slider::keyboard_step: Option<f32>` field + `with_keyboard_step(step)` builder + `resolved_keyboard_step()` (override, else `DEFAULT_SLIDER_STEP_FRAC × range`). `DEFAULT_SLIDER_STEP_FRAC` (= 0.05) is now a re-exported `pub const`.
- Example `slider_keyboard_step` (a continuous Volume slider at the default 5% step beside a discrete 0..3 Quality slider stepping one level per press).

### Changed (internal)
- `ui/system/focus_pass.rs` keyboard/gamepad slider nudge now calls `Slider::resolved_keyboard_step()` instead of the local hardcoded `SLIDER_STEP_FRAC`. No behavior change at the default.

## 0.76.0

**Configurable point-light cap — `LightingConfig`.** The 2D point-light pass was hard-capped at **16** lights, baked into the WGSL `array<GpuLight, 16>` and the uniform struct; a fork hit that ceiling with no way to raise it. The cap is now an opt-in `LightingConfig { max_lights }` resource (default `DEFAULT_MAX_LIGHTS` = 16), so **not inserting it preserves the old behavior exactly** (byte-identical 544-byte uniform). Native-only (lighting is a no-op on wasm32).

### Added
- `LightingConfig { max_lights }` resource + `DEFAULT_MAX_LIGHTS` const (`src/resources/render.rs`, re-exported from the crate root). Insert before `App::run()` to raise/lower the per-frame point-light cap; the nearest-to-camera cull keeps the closest `max_lights`.
- Example `lighting_cap` (40-light grid; **SPACE** toggles the cap 16 ↔ 40; `LIGHTING_CAP` env overrides the initial cap; `HEADLESS_SHOT` self-reports a lit-pixel count) and `scripts/lighting_cap_smoke.sh` (headless GPU A/B asserting the 40-cap lights >1.5× the area of the 16-cap).

### Changed (internal)
- `LightingRenderer` is built for a runtime `max_lights`: the WGSL light-array length is substituted at shader-build time and the uniform is a fixed 32-byte `LightingHeader` followed by a runtime-sized `[GpuLightData; max_lights]` region (`32 + max_lights * 32` bytes) written as one byte block, replacing the fixed `LightingUniforms` struct. A runtime cap change rebuilds the renderer (`set_max_lights`), wired through `setup_lighting` (reads `LightingConfig`). `select_nearest_lights` takes the cap as a parameter. No behavior change at the default cap.

## 0.75.1

**Examples: headless screenshot via `HEADLESS_SHOT`.** The `solver_iterations`, `one_way_tolerance`, and `frame_dt_cap` examples now render to a PNG and exit when `HEADLESS_SHOT=<path>` is set (with `HEADLESS_FRAMES` overriding the settle frame count) instead of opening a window — so they can be pixel-verified with the monitor off or from a remote session via `App::save_screenshot_headless` (added in 0.75.0). No library change.

### Changed
- `examples/{solver_iterations,one_way_tolerance,frame_dt_cap}.rs` gained a `HEADLESS_SHOT` env-var branch (no-op unless the variable is set, so `cargo run --example …` is unchanged).

## 0.75.0

**Feature: headless screenshots — render a frame to a PNG with no window, no surface, and no display.** `App::save_screenshot_headless(frames, path)` (and the raw-bytes `screenshot_headless(frames) -> (w, h, RGBA8)`) run the engine's **real** render path into an offscreen GPU texture and read it back, so the captured image matches what the windowed app draws. Because nothing touches the windowing system, it works with the monitor off/asleep/locked and on a machine with no display attached — exactly the cases where the OS window-capture path fails. Useful for golden-image/CI/away-from-keyboard verification of GPU rendering. Native-only (the offscreen read-back path is not on wasm).

### Added
- `App::save_screenshot_headless(frames, path)` → PNG, and `App::screenshot_headless(frames)` → `(width, height, Vec<u8>)` tightly-packed sRGB RGBA8. Call **instead of** `run`.
- `GpuContext::new_headless(width, height)` — a surfaceless GPU context (adapter requested with `compatible_surface: None`) rendering into an offscreen color texture — plus `read_headless_rgba()`, `headless_view()`, and `is_headless()`.
- `App::init_gpu_renderers(&GpuContext)` factors the window-independent renderer init (sprite/text renderers, pre-GPU render targets, `RenderCapabilities`) out of the windowed `finish_init`, shared with the headless path.
- Example `headless_screenshot` (renders three quads + text to a PNG and self-checks it is non-blank) and `scripts/headless_screenshot_smoke.sh` (a native GPU render smoke needing no display/Chrome).

### Changed (internal)
- `GpuContext.surface` is now `Option<wgpu::Surface>` (`None` in headless mode), and the render path branches its frame acquire/present accordingly. `GpuContext` is not re-exported, so this is not a public-API change.

## 0.74.0

**Feature: the main loop's per-frame delta-time cap is now configurable (`FrameConfig::max_dt`, default 0.1 s).** Each frame's `dt` is clamped so a single stall — a window drag, a debugger breakpoint, a backgrounded tab — can't hand systems a huge `dt` that tunnels physics or leaps animations. The cap was hardcoded to `0.1`, so a game wanting larger catch-up steps (a fixed-timestep sim that re-derives `dt`) or a tighter bound had to edit engine source. It is now a `FrameConfig` resource, auto-inserted with `max_dt = 0.1`, so a `World` that never touches it behaves exactly as before (non-breaking). Tier-2 hardcoding-audit knob.

### Added
- `FrameConfig` resource (`max_dt`, default 0.1 s) + `FrameConfig::cap(raw_dt)` helper; re-exported and auto-inserted in `insert_core_resources`.
- Example `frame_dt_cap` — two markers move at the same speed while the demo hitches on purpose; the engine-`dt` marker (clamped to `max_dt`) never leaps, while the raw wall-clock marker lurches on each stall. `↑/↓` change `max_dt` live.

### Changed
- The main loop (`app/render/frame.rs` `step_frame`) clamps `dt` via `FrameConfig::cap` instead of a hardcoded `.min(0.1)`.

## 0.73.0

**Feature: the physics constraint-solver iteration count is now tunable (`PhysicsWorld::set_solver_iterations`, rapier default 4).** Each `step` runs the solver a fixed number of iterations; more iterations converge contacts and **joints** harder. The count was baked into `IntegrationParameters::default()`, so a fork with stiff ragdolls, long joint chains, or high mass ratios that need a stiffer solver had to edit engine source. It is now adjustable; the default is unchanged (rapier's 4), so every existing world is byte-identical (non-breaking). Tier-2 hardcoding-audit knob.

### Added
- `PhysicsWorld::set_solver_iterations(n)` — sets `IntegrationParameters::num_solver_iterations` (clamped to ≥1, since rapier panics on 0).
- `PhysicsWorld::with_integration_params(IntegrationParameters)` builder — full solver/CCD override escape hatch — and `integration_params()` getter (`dt` is still set per-`step`, so any `dt` here is ignored).
- Example `solver_iterations` — two heavy-ended hanging joint chains in separate worlds; the 2-iteration chain visibly stretches under its weight while the 16-iteration chain holds taut, with a live per-chain `stretch` readout. The effect is also pinned by the deterministic test `solver_iterations_stiffen_a_joint_chain`.

## 0.72.0

**Feature: a `CharacterController`'s one-way-platform landing skin width is now configurable (`one_way_tolerance`, default 0.05).** A one-way collider keeps blocking a downward-moving character until its bottom sinks more than this much below the platform's top surface, so a resting character does not jitter or slip through on slight numerical penetration. The width was hardcoded to `0.05` *physics units* — wrong at any `pixels_per_unit` ≠ the engine's nominal ≈64, so a game at a coarser scale (small PPU) had to edit engine source. It is now a field; the default stays `0.05`, so every existing controller is byte-identical (non-breaking). Tier-2 hardcoding-audit knob.

### Added
- `CharacterController::one_way_tolerance: f32` field + `with_one_way_tolerance(t)` builder; `DEFAULT_ONE_WAY_TOLERANCE` pub const (= 0.05). The field is read fresh by `move_character` every frame, so direct assignment works too.
- Example `one_way_tolerance` — a small playable one-way platformer at a deliberately coarse **PPU 24** that scales the tolerance to its PPU (`DEFAULT_ONE_WAY_TOLERANCE * 64 / PPU`); jump up *through* the platform and land on top, or press Down to drop through onto the solid ground.

### Changed
- `PhysicsWorld::move_character` reads `controller.one_way_tolerance` for the one-way landing predicate instead of a hardcoded `const ONE_WAY_TOLERANCE = 0.05`.

## 0.71.0

**Feature: a continuous `ParticleEmitter`'s per-frame spawn cap is now configurable (`max_per_frame`, default 64).** The runaway guard that bounds particles spawned in one frame was hardcoded to 64, so an emitter whose `spawn_rate` exceeded `64 * fps` (e.g. dense rain/snow above ≈3840/s at 60 fps) silently under-emitted with no way to fix it short of editing engine source. The cap is now a field; the default stays 64, so every existing emitter is byte-identical (non-breaking). Tier-2 hardcoding-audit knob.

### Added
- `ParticleEmitter::max_per_frame: u32` field + `with_max_per_frame(n)` builder; `DEFAULT_MAX_PER_FRAME` pub const (= 64, re-exported at the crate root).
- `max_per_frame` is also a RON emitter-config field (`#[serde(default)]` = 64) and an editor Particle-Tuner row, so dense-rain presets and live tuning can raise it.
- Example `particle_spawn_cap` — two rain emitters with the same high `spawn_rate` side by side; the default-cap (64) column visibly under-emits vs the raised-cap (256) column, with a live per-column particle count.

### Changed
- `ParticleSystem`'s continuous-emission path caps the per-frame spawn at `emitter.max_per_frame` instead of the hardcoded `64`.

## 0.70.0

**Feature: render targets can be sampled with a caller-chosen `FilterMode` for display.** A `RenderTarget`'s sampler was hardcoded to `Nearest`, so any RT shown scaled up or used as a blur source was always pixelated — a fork had to edit engine source to change it. `App::create_render_target_with_filter(name, w, h, filter)` (surface format) now lets the caller pick `Linear` for a smooth scaled/blurred RT; the default stays `Nearest`, so every existing render target is byte-identical (non-breaking). Tier-2 hardcoding-audit knob.

### Added
- `App::create_render_target_with_filter(name, width, height, filter)` — create an RT (surface format) whose display sampler uses the given `wgpu::FilterMode`.
- `RenderTarget::new_with_filter(device, width, height, format, filter)` + `RenderTarget::filter()` accessor; `RenderTarget` stores its sampler filter (mirrors the `format()` pattern). `RenderTarget::new` delegates to `new_with_filter` with `Nearest` (unchanged behavior).
- Example `render_target_filter` — the same tiny 40×40 scene rendered into two RTs and displayed 7.5× side by side; left `Nearest` (blocky), right `Linear` (smooth). The only difference is the sampler filter.

### Changed (internal)
- `create_render_target_impl` + the `pending_render_targets` deferred-creation tuple thread a `wgpu::FilterMode`; the existing `create_render_target` / `create_render_target_with_format` pass `Nearest`.

## 0.69.1

**Fix: native synthesized tones (`AudioManager::play_tone` with no channel effect) no longer click on/off.** The wasm `WebAudio` backend already wraps every tone in a short attack/release gain envelope, but the native rodio path emitted a raw `SineWave` that started and ended at an abrupt amplitude — an audible click, and a cross-platform behavior gap. The default (no-effect) native tone now carries the same `min(25% of the tone, 8 ms)` linear attack+release envelope as wasm. rodio 0.19 has no source `fade_out`, so the enveloped tone is materialized into a `SamplesBuffer`. Tones that already configure a channel effect are unchanged. No public API change.

### Fixed
- `src/audio/playback.rs` — `play_tone`'s no-effect branch now emits an enveloped tone via `enveloped_tone_samples` (linear attack/release matching the wasm envelope formula, named consts `TONE_ENVELOPE_FRAC` / `TONE_ENVELOPE_MAX_SECS`). Added `tone_envelope_ramps_to_zero_at_both_ends` + `tone_sample_count_matches_duration` tests that assert the de-click objectively (first/last samples at zero gain, body still peaks near full amplitude) without an audio device.

## 0.69.0

**Added: per-tilemap render depth via `Tilemap::z` / `with_z`, so multiple orthographic or hexagonal tilemaps can be stacked as background/foreground layers.** Previously `cell_z()` returned a fixed `-1.0` for every orthographic/hexagonal tile, so two such tilemaps drew at the same depth and could not be reliably layered. `Tilemap` now carries a `pub z: f32` (default `-1.0`, set via the `with_z` builder) that `cell_z()` returns for those projections. Isometric is unchanged — it still derives a per-cell depth (`row + col`) and ignores `z`. Existing maps keep the `-1.0` default, so behavior is unchanged unless `z` is set.

### Added
- `src/tilemap/mod.rs` — `Tilemap::z` field + `Tilemap::with_z(z)` builder; `cell_z()` returns `self.z` for orthographic/hexagonal projections (isometric keeps `row + col`). Tests: `cell_z_defaults_to_minus_one`, `with_z_sets_render_depth_for_ortho_and_hex`, `isometric_ignores_z`.
- `examples/tilemap_layers.rs` — stacks a background floor (`z = -2.0`) under a foreground decoration map (`z = -1.0`); the foreground is spawned *first* yet draws on top, demonstrating that `z` (not spawn order) drives layering. Verified natively (screenshot).

## 0.68.6

**Fix: an over-large or dimension-overflowing `PathGrid` no longer fails silently.** `grid_cell_count` returned `0` (→ an empty grid where every cell is unwalkable, so all pathfinding fails) when `width × height` exceeded the internal `10_000_000`-cell cap or overflowed `i32`, with no diagnostic — a fork building a large world (e.g. 4096² = 16M cells) got an empty grid and no clue why. Those two cases now log a `log::error!` explaining the empty grid and how to fix it; the benign empty grid from a zero/negative dimension stays silent. The cell cap is also promoted to a public constant so forks can size against it. No behavior change beyond the added logging.

### Changed
- `src/pathfinding.rs` — `MAX_PATH_GRID_CELLS` is now `pub` (and re-exported as `engine::MAX_PATH_GRID_CELLS`) with documentation. `grid_cell_count` logs an `error!` on `i32` overflow and on exceeding the cap (but not on a legitimately empty zero-dimension grid). Added `oversized_grid_is_empty_not_allocated` + `overflowing_grid_dims_are_empty` regression tests.

## 0.68.5

**Behavior-preserving cleanup: duplicated hardcoded literals (window clear color, panel z-offset) hoisted into named constants.** Found by a whole-`src/` hardcoding audit. The window clear color `[0.08, 0.08, 0.12, 1.0]` appeared as four independent literals (two array-form, two `wgpu::Color`-form) that could silently drift; the panel-background z-offset `0.01` had a named constant in the pointer-capture pass but `LayoutSystem` still used a raw literal that has to match it. Each is now a single source of truth. Values are byte-identical, so rendering is unchanged. **No behavior change**; one additive public re-export (`DEFAULT_CLEAR_COLOR`).

### Changed (internal)
- `src/resources/display.rs` — new `pub const DEFAULT_CLEAR_COLOR: [f64; 4]` is the single source for `WindowConfig::default().clear_color` and the clear-pass fallback in `frame.rs` (which previously repeated the literal). Re-exported from `crate::resources`.
- `src/app/render/mod.rs` — new `const EDITOR_SURFACE_CLEAR: wgpu::Color`, shared by the two docked-editor surface clears (`frame.rs` post-scene clear + `docked.rs` warm-up placeholder), which each previously inlined the same `wgpu::Color`. Kept a separate constant from `DEFAULT_CLEAR_COLOR` on purpose: a game changing its own `WindowConfig::clear_color` should not repaint the editor letterbox.
- `src/ui/panel.rs` — hoisted `pub(crate) const PANEL_BG_Z_OFFSET: f32 = 0.01` (the value `LayoutSystem` draws panel backgrounds at) and used it in place of the raw literal; `src/ui/system/capture.rs` now imports it instead of re-declaring its own copy, so render and pointer-capture can no longer drift apart.

## 0.68.4

**Behavior-preserving refactor: the 730-line grab-bag `src/dialogue/mod.rs` is split into themed submodules.** The flat module mixed the `DialogueBox`/`DialogueChoice` data model, the `DialogueStyle` resource, and the `DialogueSystem` renderer in one file. Those concerns moved into focused submodules, all re-exported from `mod.rs` so `crate::dialogue::*` and the `engine::*` re-exports in `lib.rs` resolve unchanged. Pure code movement — method/struct bodies are verbatim; only module location, imports, and a few intra-doc links changed. **No public API change, no behavior change** (the dialogue unit + doc tests are unchanged and pass). Follows the 0.68.1 `resources.rs` / 0.68.2 `world.rs` split-PATCH precedent.

### Changed (internal)
- `src/dialogue/mod.rs` → keeps only the module doc, the `mod` declarations, and the `pub use` re-exports (it is now a 44-line coordinator). Grouping: `model.rs` (`DialogueChoice` + `DialogueBox` + the world-level `advance`/`choose` free fns — kept together because `DialogueBox::visible_choices` calls the private `DialogueChoice::is_available`), `style.rs` (`DialogueStyle` + its `Default`), `system.rs` (the private `DrawItem` + `DialogueSystem`). The existing `tree.rs` / `vars.rs` submodules are unchanged.
- `DialogueBox`'s private fields (`elapsed`/`full`) stay encapsulated in `model.rs`; `DialogueSystem` only touches the public API, so nothing had to be widened. Intra-doc links to `DialogueSystem` from `model.rs`/`style.rs` are now qualified (`crate::DialogueSystem`) since it lives in a sibling module.
- `src/dialogue/tests.rs` gains one `use crate::locale::LocaleResource;` — it had relied on `mod.rs`'s now-relocated private import being visible through its `use super::*` glob (`World`/`Events`/`TextQueue`/`ViewportSize` were already imported locally per-test). Otherwise unchanged.
- `CLAUDE.md` module-map row for the dialogue module updated to list the new submodules.

## 0.68.3

**Behavior-preserving refactor: the docked-editor render-target management block is extracted out of the `frame.rs` `render()` god-function.** `App::render()` held a single ~680-line body whose largest self-contained chunk was the native-only block that (re)creates the docked-editor offscreen scene texture on its debounce schedule, keeps the egui registration in sync, and yields this frame's scene target. That block moved verbatim into a new associated fn `App::prepare_docked_scene_view(render, editor, window, gpu) -> Option<TextureView>` in `src/app/render/docked.rs`, alongside the existing `present_docked_placeholder` helper it pairs with. `render()` calls it in one line, shrinking ~133 lines. Pure code movement — only `self.<field>` accesses became the matching parameters; the logic is unchanged. **No public API change, no behavior change**: the common (non-docked) render path is structurally byte-identical because the extracted fn returns `None` when not docked, so every downstream pass targets the surface exactly as before. Continues the 0.68.1 `resources.rs` / 0.68.2 `world.rs` split-PATCH precedent, applied at the function level.

### Changed (internal)
- `src/app/render/frame.rs` — the `#[cfg(not(target_arch = "wasm32"))] let docked_render_view = { … }` block (~150 lines) in `render()` is replaced by a `Self::prepare_docked_scene_view(&mut self.render, &mut self.editor, self.window.as_deref(), gpu)` call. The wasm fallback (`docked_render_view = None`) is unchanged.
- `src/app/render/docked.rs` — new native-only `pub(in crate::app) fn prepare_docked_scene_view`; takes `&mut RenderState`, `&mut EditorState`, `Option<&Window>`, `&GpuContext` as disjoint borrows (the same associated-fn pattern as `present_docked_placeholder` / `render_offscreen_targets`, so it composes with the `gpu = self.gpu.as_mut()` borrow held across `render()`).

## 0.68.2

**Behavior-preserving refactor: the ~900-line `impl World` in `src/ecs/world.rs` is split into concern submodules under `src/ecs/world/`.** The core ECS file held a single 26-method `impl World` block (spawn/despawn/components/queries/resources/reflect/change-tracking/clone) inline alongside the data model; that surface is now grouped into one submodule per concern, each re-opening `impl World` as a descendant module so it reaches `World`'s private fields and the shared private archetype plumbing (`get_or_create_archetype`/`move_entity`/`clone_component_by_typeid`/`has_component_typeid`) unchanged. Pure code movement — only module location and imports changed; method bodies are verbatim. **No public API change, no behavior change** (931 lib tests unchanged). Directly follows the 0.68.1 `resources.rs` split precedent.

### Changed (internal)
- `src/ecs/world.rs` now holds only the data model (`Entity`/`Archetype`/`World` structs, type aliases, reflect free-fns, `ReflectEntry`), `World::new`, the private archetype helpers, and the `Default` impl. The `impl World` API moved verbatim into `src/ecs/world/{entities,components,queries,resources,reflect,change_tracking,clone}.rs`, plus native-only `parallel.rs` for the `#[cfg(not(target_arch = "wasm32"))]` `par_query*` block.
- Grouping: `entities` (`spawn`/`despawn`/`is_alive`/`entity_count`/`entities`/`apply_commands`), `components` (`add`/`remove`/`take_component`/`get`/`get_mut`/`has_component`), `queries` (`query`/`query2`-`query4`/`query_mut`/`query2_mut`/`query3_mut`/`query_with`/`query_without`/`query_opt2`), `resources` (`insert_resource`/`resource`/`resource_mut`/`remove_resource`/`with_resource_mut`/`*_erased`), `reflect` (`register_reflect_named`/`reflect_registered_types`/`get_reflect`/`get_reflect_mut`/`reflected_components`), `change_tracking` (`clear_change_tracking`/`query_added`/`query_changed`/`mark_changed`/`get_mut_tracked`), `clone` (`register_clone`/`clone_entity`).
- `world/tests.rs` is unchanged (`use super::*` still resolves; `World`/`Entity` stay in the `world` module). `lib.rs` and all call sites are unchanged.
- `CLAUDE.md` module-map row for the ECS world updated to describe the split.

## 0.68.1

**Behavior-preserving refactor: the 871-line grab-bag `src/resources.rs` is split into `src/resources/` by concern.** The flat module that held ~26 unrelated engine resources became a directory of seven focused submodules, all re-exported from `mod.rs` so `crate::resources::*` and the `engine::*` re-exports in `lib.rs` resolve unchanged. Pure code movement — only module location, imports, and paths changed; the moved tests are otherwise verbatim. **No public API change, no behavior change.** Follows the prior editor god-file split precedent (e.g. `docked.rs` → 0.49.1).

### Changed (internal)
- `src/resources.rs` → `src/resources/{mod,debug_draw,display,fonts,lifecycle,profiling,render,time}.rs`. Grouping: `debug_draw` (`DebugShape`/`DebugDraw`/`FilledRect`), `display` (`ViewportSize`/`DesignResolution`/`Letterbox`/`DisplayScaleFactor`/`DEFAULT_CANVAS_ID`/`WindowConfig`/`WindowMode`/`WindowOptions`/`ImeConfig`/`PendingResize`), `fonts` (`FontData`/`ExtraFonts`), `lifecycle` (`PanickedSystems`/`LoadProgress`/`GameState`/`ShouldQuit`/`FadeTransition`), `profiling` (`SelectedEntity`/`SystemProfile`/`RenderStats`/`ProfilerData`), `render` (`CullConfig`/`AmbientLight`), `time` (`TimeScale`/`RealDt`).
- `mod.rs` re-exports all 27 public items, keeping every existing path stable; `lib.rs` is unchanged. The `debug_draw`/`letterbox`/`fade` unit tests moved with their types and run under the new module paths.
- `CLAUDE.md` module-map row for the resources updated to point at `src/resources/`.

## 0.68.0

**The `bloom` example now runs in the browser — the mip-chain "dual filter" bloom renders on WebGL2.** Ships the existing `bloom` example to the web with the same engine code as native (the `hdr_render_target` v0.60.0 / `render_format_query` v0.66.0 precedent: a web harness is a MINOR even with no library change). This is the most worthwhile example to put on the web because the bloom pass renders the scene into an `Rgba16Float` HDR intermediate and blurs the highlights through a pyramid of `Rgba16Float` mip render targets — all usable on WebGL2 only with the `EXT_color_buffer_float` extension, so whether the whole HDR + mip-chain pipeline actually runs differs from the desktop case. **No library (`src/`) change.**

### Added
- `examples/bloom.rs` gains a wasm browser entry point: `#[wasm_bindgen] run_bloom()` (app setup factored into a shared `build_app()` used by both the native `main()` and the wasm entry), plus a wasm-only headless self-check — the demo survives ~30 frames of HDR + mip-chain bloom and writes `BLOOM_WEB_CHECK: PASS (1/1)` to the tab title (a boot panic on an unrenderable target fires `console_error_panic_hook` and no verdict appears).
- `examples/bloom/web/{build.sh,index.html}` — the `cargo build --example` + `wasm-bindgen --target web` harness with a **Start** button (`pkg/` is gitignored).
- `scripts/bloom_web_smoke.sh` — an optional (non-CI) headless smoke that boots the example under SwiftShader WebGL2 (`?autostart=1`) and asserts the `BLOOM_WEB_CHECK: PASS` verdict over Chrome's DevTools endpoint, confirming the `Rgba16Float` HDR intermediate + the mip-pyramid float bloom targets actually render in a browser. Documented in `docs/WASM_SMOKES.md`.

## 0.67.0

**Fuller bloom: the real multi-pass bloom now uses a downsample/upsample mip pyramid ("dual filter").** The opt-in `PostProcessConfig.bloom` pass was rebuilt from a fixed-half-res separable Gaussian (ping-ponged `bloom_iterations` times) into the physically-based mip-chain bloom from Jimenez's "Next Generation Post Processing in CoD: Advanced Warfare": a 13-tap **downsample chain** builds a mip pyramid of the bright highlights, and a 3×3-tent **upsample chain** accumulates the levels back up with additive blending, producing a wider, smoother, energy-preserving glow in a few passes (the separable blur's reach was bounded by `kernel × iterations` and looked boxy when pushed). The **public API is unchanged** — `bloom_iterations` now selects the pyramid depth (number of mip levels); both old and new meaning control glow width. The bloom OFF path (cheap inline 4-tap) is byte-identical to before; the pass still runs on the scene intermediate format so it works under HDR.

### Changed
- `src/renderer/bloom.rs` (`BloomRenderer`, `pub(crate)` API unchanged: `new`/`reconfigure`/`resize`/`update`/`run` + `MAX_BLOOM_ITERATIONS`) rewritten to a mip pyramid: a `Vec<Mip>` of half-and-down render targets (capped at `MAX_BLOOM_ITERATIONS` levels and stopping before a dimension degenerates), prefilter → downsample chain → additive upsample chain → additive composite. The render-orchestration call sites (`frame.rs` step 3.5, `post_lighting.rs` setup) are untouched.
- `src/renderer/shaders/bloom.wgsl` replaces `fs_blur` (separable Gaussian) with `fs_downsample` (13-tap) + `fs_upsample` (3×3 tent); `fs_prefilter` now does the 13-tap bright-pass downsample; `fs_composite` unchanged in spirit. The shared uniform gains a `radius` (upsample tent) field.
- `PostProcessConfig::bloom_iterations` doc updated: it is now the pyramid depth (clamped to `0..=8` and to the levels the resolution allows). Default `4` and all field types unchanged.
- Example `bloom` wording updated (mip-chain / "pyramid depth"); behaviour and keys unchanged.

## 0.66.0

**`render_format_query` example shipped to the web.** The v0.65.0 render-format renderability query (`RenderCapabilities`) now runs in the browser — which is where it matters most: on WebGL2 a float render target like `Rgba16Float` is renderable only with the `EXT_color_buffer_float` extension, so a backend's actual renderability differs from the desktop case. Same engine code as native; no library change.

### Added
- `render_format_query` web harness (`examples/render_format_query/web/build.sh` + `index.html`) over a new wasm-only `#[wasm_bindgen] run_render_format_query` entry point; the example's app setup moved into a shared `build_app()`. A Start button boots it (`?autostart=1` boots headless). `pkg/` is gitignored.
- `scripts/render_format_query_smoke.sh` — headless Chrome (SwiftShader WebGL2) boots the example and asserts the query is correct on the live WebGL2 backend: the example self-checks two backend-independent invariants (the surface format is a renderable color render target; a block-compressed format is not) and writes `RENDER_FORMAT_QUERY_CHECK: PASS (2/2)` to the page title. Optional local check, not a CI gate; documented in `docs/WASM_SMOKES.md`.

## 0.65.0

**Engine-level render-format renderability query + automatic render-target fallback.** A game can now ask whether a `wgpu::TextureFormat` is a usable color render target on the current GPU/backend *before* requesting one — float formats like `Rgba16Float` are renderable only with `EXT_color_buffer_float` on WebGL2, for example. The new `RenderCapabilities` resource exposes the query to systems, and `App::create_render_target_with_format` now degrades gracefully (falls back to the surface format with a warning) instead of creating an invalid texture when a requested format is not renderable. This closes the "real fallback" deferred from the HDR/render-format arc. The supported/default path is unchanged.

### Added
- `RenderCapabilities` resource (re-exported at the crate root, inserted into the `World` at GPU initialization): `supports_render_target(format) -> bool` and `surface_format() -> wgpu::TextureFormat`. Read it from a system or `Scene::on_enter` to branch on GPU capabilities (e.g. request an HDR render target only where renderable).
- `GpuContext` now retains the GPU `adapter` (previously dropped after init) and gains `supports_render_target` + `resolve_render_target_format` (`src/renderer/context.rs`).
- Example `render_format_query` (`cargo run --example render_format_query`): renders a color-coded yes/no capability table + the HDR-vs-fallback decision; logs the table once for a headless smoke.

### Changed
- `App::create_render_target_with_format` (and the deferred-creation path drained at GPU init) now resolve a non-renderable caller-chosen format to the surface format with a `log::warn!`, instead of attempting to create an invalid render target. A `None` format and a renderable format behave exactly as before.

## 0.64.0

**Real multi-pass bloom in the post-process pass.** Opt-in via `PostProcessConfig.bloom` (default `false`, so the OFF path is byte-identical to before — the cheap inline 4-tap bloom). When enabled (requires `enabled: true`), a new `BloomRenderer` extracts scene highlights, blurs them with a separable Gaussian ping-ponged at half resolution `bloom_iterations` times (0..=8, default 4), and additively composites the soft glow back onto the scene intermediate *before* the post-process composite; the post shader then skips its inline 4-tap. The bloom pipelines are built for the scene intermediate format, so it works under HDR (`Rgba16Float`). Like the post-process and lighting renderers (and unlike the sprite/material/UI/GPU-particle per-target-format pipeline *cache*), there is one target per frame, so a format change just rebuilds the renderer.

### Added
- `PostProcessConfig::bloom` (`bool`, default `false`) and `PostProcessConfig::bloom_iterations` (`u32`, default `4`, clamped to `0..=8`).
- `src/renderer/bloom.rs` (`BloomRenderer`, `pub(crate)`) + `src/renderer/shaders/bloom.wgsl` (`fs_prefilter` / `fs_blur` / `fs_composite`).
- `App::setup_bloom_renderer` wiring in `src/app/render/post_lighting.rs`; bloom pass (Step 3.5) in `src/app/render/frame.rs`; `bloom_renderer` field on `RenderState`.
- Example `bloom` (`cargo run --example bloom`): over-bright emitters glow, dim ones don't; `B` toggles real vs inline bloom.

### Changed
- `post_process.wgsl` / `PostProcessUniforms` gained a `bloom_enabled` flag (uniform grows to 64 B) so the inline 4-tap is skipped when the real bloom pass ran (avoids double bloom).

## 0.63.3

**Internal: extracted shared wgpu render boilerplate into `renderer::common`.** Code-quality scan P3 (2026-06-23): the sprite / lighting / post-process / texture / UI-primitive / egui passes each built byte-identical bind-group-layout entries, clamp samplers, and single-color render passes inline, so a wgpu upgrade meant editing the copies in lockstep. New `pub(crate)` helpers centralize the shared shapes; the values produced are identical to the previous inline literals. **No public API change, no behavior change** (verified by a native render smoke of `lit_dungeon` lighting + `tonemap` post-process).

### Changed (internal)
- New `src/renderer/common.rs`: `filterable_texture_entry` / `filtering_sampler_entry` / `uniform_buffer_entry` (bind-group-layout entries), `create_clamp_sampler` (clamp-to-edge sampler, filter as a param), `begin_color_pass` (single color attachment + store + no depth, load op as a param; `.forget_lifetime()` at the egui site).
- Call sites switched to the helpers: `texture.rs` (layout + nearest sampler), `post_process.rs` (layout + linear sampler + pass), `lighting.rs` (4-entry layout + linear sampler + clear-normal + lighting passes), `sprite/draw.rs` + `sprite/ui_primitives.rs` + `app/egui_pass.rs` (passes).

## 0.63.2

**Internal: promoted DebugDraw and native frame-pacing hardcoded literals to named constants.** Two low-risk code-quality findings from the 2026-06-23 scan (P3): the debug-overlay shape renderer embedded magic numbers (stroke `1.5`, circle `24` segments, step floor `0.5`, min length `0.001`, z `999.0`) inline, and the native redraw-cadence policy embedded `60.0` fallback / `[60.0, 240.0]` clamp / `1.0` min-valid in one expression. Both are now named constants with intent docs, ready for a future `DebugDrawConfig` / `FramePacingConfig` to lift without hunting through code. **No public API change, no behavior change.**

### Changed (internal)
- `src/app/render/debug_draw.rs` — `DEBUG_Z` / `LINE_THICKNESS` / `CIRCLE_SEGMENTS` / `MIN_STEP_THICKNESS` / `MIN_SEGMENT_LEN` replace inline literals.
- `src/app/window.rs` — private `frame_pacing` module (`FALLBACK_REFRESH_HZ` / `MIN_VALID_REFRESH_HZ` / `MIN_REFRESH_HZ` / `MAX_REFRESH_HZ`, native-only) replaces the inline refresh-rate fallback + clamp.

## 0.63.1

**Internal: egui overlay submission consolidated into one helper.** The egui renderer lifecycle (update texture deltas → update buffers → record the render pass → submit → free textures → restore the renderer) was duplicated near-identically in the final surface overlay (`frame.rs`) and the docked-editor placeholder (`docked.rs`), differing only in callback handling. Both now call a single `submit_egui(render, gpu, view, guard_callbacks)` in `egui_pass.rs`, so future egui changes are made once and the two paths can't drift. **No public API change, no behavior change.**

### Changed (internal)
- `egui_pass::submit_egui` is the single egui-submission flow; `frame.rs::present_egui` passes `guard_callbacks = true` (paint callbacks unsupported → skipped with a warn), `docked.rs` passes `false` (the placeholder never produces callbacks). `egui_render_pass` is now private to `egui_pass` (only `submit_egui` uses it). No public API change.

## 0.63.0

**`DialogueStyle` — restyle `DialogueSystem` without forking the engine.** Dialogue layout (positions, font sizes, colors for the speaker / body / choice list / advance hint / portrait, plus the no-`ViewportSize` fallback) was hardcoded in `DialogueSystem`, so a game had to edit engine source to change normal dialogue style. It's now a new opt-in `DialogueStyle` resource: insert a customized one to restyle, or leave it absent and the system uses `DialogueStyle::default()`, which reproduces the previous look **exactly** — existing games are unchanged. Vertical positions are offsets up from the viewport bottom (so the box stays anchored on a taller window).

### Added
- `DialogueStyle` resource (`src/dialogue/mod.rs`), re-exported from `engine`. Fields cover the portrait (size/x/bottom-offset/text-gap), text margin, and per-element (speaker/body/choice/hint) bottom-offsets, font sizes, colors, the body wrap height, the choice indent/line-step, and the advance-hint label + right-offset. `Default` matches the original literals.
- `dialogue_style` example — the same `DialogueBox` drawn with a custom style vs. the default; `T` toggles the `DialogueStyle` resource on/off live.

### Changed
- `DialogueSystem::run` reads `DialogueStyle` (cloned once, falling back to `default()`) and draws from it instead of inline constants. No behavior change when the resource is absent.

## 0.62.2

**GPU particles now render under HDR post-process.** HDR post (`PostProcessConfig::hdr`) renders the scene into an `Rgba16Float` intermediate; the GPU-particle render pipeline was built only for the surface format, so particles were skipped (with a warn-once) under HDR — the last render pass not yet format-matched. The renderer now lazily builds and caches a render pipeline per non-surface target format, mirroring the sprite/material/UI pipeline caches (v0.56.0/v0.59.0). With this, **every scene pass is format-matched under HDR.** The surface-format path is unchanged (the common case is a cache no-op).

### Fixed
- GPU particles are no longer skipped under HDR post-process. `GpuParticleRenderer` gains `ensure_render_pipeline(device, format)` + an internal per-format pipeline cache; `render` takes the scene's `target_format` and selects the matching pipeline. The surface-format fast path is byte-identical. (`src/renderer/gpu_particle.rs`, `src/app/render/frame.rs`.)

### Changed
- `gpu_particles` example: `H` toggles HDR post-process (ACES tonemap) to demonstrate particles rendering into the HDR intermediate.

## 0.62.1

**Fixed: a UI widget covered by another widget kind could still fire.** Each widget pass (button / checkbox / slider / scroll / focus) used to hit-test only its own kind, so e.g. a button behind an opaque `Panel` (or any higher-z widget) still emitted `ButtonClicked` on a click that visually landed on the panel. A new shared per-frame pointer-occlusion map now decides which single widget owns each point across *all* widget kinds, and every pointer interaction (click, toggle, slider press, wheel scroll, click-to-focus) is granted only to the topmost pointer-opaque surface under the cursor. No public API change.

### Fixed
- Cross-widget pointer capture: a button / checkbox / slider / scroll view / focusable covered by a higher-z widget (e.g. a `Panel`) no longer receives the pointer interaction — the topmost surface absorbs it (`src/ui/system/capture.rs`, new internal `PointerCapture`; `UiSystem` rebuilds it once per frame and the focus/button/checkbox/slider/scroll passes query it).

### Changed
- Click-to-focus now follows draw order (z): when focusable widgets overlap, a click focuses the one drawn on top (greater z), consistent with the other widget passes and with what the player sees. Previously focus ignored z and picked the highest entity index; a z tie is still broken by entity index. (Behavior correction; no API change.)
- `Panel` registers in the capture set at `z - 0.01` (where its background actually draws), so a panel occludes lower-z widgets behind it but never its own children. `Label` is excluded from capture (text is not pointer-opaque).

## 0.62.0

**Fixed design (virtual) resolution + letterbox scaling — a new opt-in `DesignResolution` resource (EW-003).** A game can author its whole UI at one logical canvas size (e.g. 1280×720) and ship at any window size: the engine reports the design size as `ViewportSize`, renders all content in design space, then scales it to the real window with a **uniform, centered scale + letterbox bars**. Cursor input and `Camera::screen_to_world` are mapped back into design space so hit-testing still lines up. Absent (the default), `ViewportSize` equals the window size and nothing is scaled — existing games are unaffected, and the OFF path is byte-identical (the letterbox is a `(1,1)` clip scale = a no-op). Implemented as a coordinate transform (no offscreen render target / extra blit), so content renders crisply at native window resolution. Companion to EW-002.

### Added
- `DesignResolution { width, height }` resource (`src/resources.rs`), re-exported from `engine`. Insert before `App::run()`.
- `Letterbox` resource — the computed transform, inserted every frame (identity when no `DesignResolution`): `clip_scale` (a centered clip-space scale post-multiplied onto each scene projection), and `px_scale`/`px_offset` (logical-pixel scale + offset for text and cursor mapping). `Letterbox::compute`/`window_to_design` + unit tests.
- `camera::apply_letterbox(clip_scale, proj)` — post-multiplies the letterbox clip scale onto a projection (returns the projection unchanged for the identity case).

### Changed
- `compute_viewport` (`src/app/schedule.rs`) applies `DesignResolution` → `ViewportSize` = design size and inserts the computed `Letterbox` (identity in the docked editor / when unset).
- The scene render passes thread the letterbox clip scale: `SpriteRenderer::render`, `render_ui_primitives_from_slices`, `GpuParticleRenderer::render`, and `LightingRenderer::update` apply it to their projections / light NDC; the text renderer (`src/renderer/text/renderer.rs`) maps design-space positions/sizes into the window via `px_scale`/`px_offset`. Offscreen render targets always pass identity. The cursor handler (`src/app/window.rs`) maps the window cursor back into design space.
- New example `design_resolution` (a 1280×720 UI letterboxed to any window).

## 0.61.0

**Window mode control — a new opt-in `WindowOptions` resource for resizability, fullscreen, and a 16:9-style aspect lock (EW-002).** A fixed-aspect game can now stop a freeform OS resize from distorting its UI without forking the engine. Delivered as a *separate* resource (the `ImeConfig` pattern) rather than new `WindowConfig` fields, so **no public API breaks** — absent, the engine behaves exactly as before (a normal resizable windowed window). The aspect-lock resize correction is native-only (the wasm canvas size is owned by the HTML page).

### Added
- `WindowOptions` resource (`src/resources.rs`) — `resizable: bool` (default `true`), `mode: WindowMode`, `lock_aspect: Option<f32>`. Insert before `App::run()`; absent = default behavior. Re-exported from `engine`.
- `WindowMode` enum — `Windowed` (default) / `BorderlessFullscreen` (borderless fullscreen on the current monitor).
- `src/app/window.rs` applies `WindowOptions`: `with_resizable` + `with_fullscreen(Borderless(None))` at window creation, and a native `WindowEvent::Resized` handler that re-derives height from width to hold `lock_aspect` (converges in one step — the corrected size's next `Resized` already matches, no feedback loop).
- `examples/window_mode.rs` — 16:9 aspect-lock demo: a live-`ViewportSize` border ring + centered box that stay 16:9 as the window is dragged.

## 0.60.0

**`hdr_render_target` example shipped to the web — and `Rgba16Float` render targets confirmed on WebGL2.** The HDR-vs-LDR render-target demo now has a `web/` harness, so it runs in a browser with the same code as native. This also answers a portability question the HDR-render-target feature left open: an `Rgba16Float` color **render target** works on `wgpu`'s WebGL2 backend (it needs the `EXT_color_buffer_float` extension) — verified headless under SwiftShader, the lowest-common-denominator WebGL2 backend, so modern browsers work too. Render targets were already cross-platform (not native-only as the example previously claimed); only the example's wasm entry point + harness were missing. No library change.

### Added
- `hdr_render_target` web harness (`examples/hdr_render_target/web/build.sh` + `index.html`) over a new `#[wasm_bindgen] run_hdr_render_target` entry; the example's `main` body moved into a shared `run()`. `pkg/` is gitignored.
- `scripts/hdr_web_smoke.sh` — headless-Chrome render smoke that asserts the wasm example renders a non-blank frame (i.e. the `Rgba16Float` render target was created) under SwiftShader; documented in `docs/WASM_SMOKES.md`.

### Changed
- `hdr_render_target` example docs corrected: render targets are **not** native-only; the HDR target needs `EXT_color_buffer_float` on WebGL2 (present in modern browsers).

## 0.59.0

**Format-matched material + UI pipelines — they render into non-surface targets now.** Completes the render-format story: `ShaderMaterial` and UI-primitive (`DrawRect`/`DrawImage`) pipelines were compiled for the surface format only, so they were skipped when drawing into a non-surface target (an HDR/linear offscreen render target, or the v0.58 HDR post-process intermediate). Both now keep a per-target-format pipeline cache (like sprites), built lazily on first use, so a material in an offscreen `Rgba16Float` render target renders correctly, and UI primitives render through the HDR post-process intermediate. The surface-format fast path is untouched (the extra caches stay empty when only one format is used). This **lifts the v0.58 skips** for materials and UI; only GPU particles remain skipped under HDR post. Additive.

### Added
- `offscreen_material` example — an animated-plasma `ShaderMaterial` quad captured by two `OffscreenCamera`s into an `Rgba16Float` (HDR) and a surface render target side by side; both monitors render the material (the HDR one was previously blank).
- A UI `DrawRect` accent bar in the `tonemap` example — exercises UI primitives rendering through the HDR post-process intermediate.

### Changed
- **`MaterialRenderer::custom_pipelines`** is now keyed by `(frag-source hash, target format)`; `compile_pipeline` / `compile_material_pipeline` / `batch_and_upload` / `collect_draw_entries` / `record_draw_pass` thread the target format through, and the material draw selects the pipeline matching the pass's attachment format (`src/renderer/sprite/{material,batch,collect,draw}.rs`). The `materials_supported` skip from v0.58 is removed (materials are format-matched).
- **`SpriteRenderer`** gains `extra_ui_pipelines` (a per-format UI-pipeline cache) + `ensure_ui_pipeline` / `ui_pipeline_for` (mirroring the sprite cache); `render_ui_primitives_from_slices` selects the UI pipeline matching `ctx.format` (`src/renderer/sprite.rs`, `ui_primitives.rs`). The v0.58 UI-skip guard in `frame.rs` is removed.

## 0.58.0

**HDR tone-mapping in the post-process pass.** `PostProcessConfig` gains `hdr` / `tonemap` / `exposure`. With `hdr: true` the scene is rendered into an `Rgba16Float` intermediate (so colour values `> 1.0` survive instead of clamping at 8-bit store time), then the post-process pass applies the chosen `tonemap` operator (`Tonemap::{None, Reinhard, AcesFilmic}`) and `exposure` to map it back to the display. This is the engine-side tone-map the previous HDR-render-target feature left to the game. Default (`hdr: false`, `exposure: 1.0`, `tonemap: None`) is byte-identical to the previous pass — exposure `× 1.0` and `None` are exact no-ops, and the uniform's old `pad0: vec2` was repurposed for `exposure` + `tonemap` so the buffer layout is unchanged. The HDR intermediate is fed by the **sprite + render-plugin** passes; UI primitives, GPU particles, and shader-materials are **skipped** while HDR post is active (a one-time warning is logged) because their pipelines are compiled for the surface format — format-matching them is future work. Additive.

### Added
- **`Tonemap`** enum (`None` / `Reinhard` / `AcesFilmic`, `#[non_exhaustive]`, re-exported at the crate root) and **`PostProcessConfig::{hdr, exposure, tonemap}`** fields (`src/renderer/post_process.rs`).
- ACES-filmic + Reinhard tone-map operators + an exposure multiply in `src/renderer/shaders/post_process.wgsl`.
- `tonemap` example — a row of same-hue, increasing-brightness swatches; toggle the operator / exposure / HDR to see over-bright values clamp to flat white (None) vs roll off and stay distinct (ACES).

### Changed
- `PostProcessRenderer` now distinguishes its **intermediate** texture format (HDR `Rgba16Float` when enabled) from its **output** (display) format; `setup_post_renderer` reconfigures when either changes (`src/app/render/post_lighting.rs`, `frame.rs`).
- The sprite pass renders into the HDR intermediate via the existing per-format sprite pipeline cache; `record_draw_pass` gained a `materials_supported` flag and skips `ShaderMaterial` entries when drawing into a non-surface format (`src/renderer/sprite/draw.rs`).

## 0.57.0

**`wgpu` re-exported at the crate root + the `positional_audio` example shipped to the web.** Two additive conveniences. `pub use wgpu;` lets a game name GPU types (e.g. `wgpu::TextureFormat`) without its own `wgpu` dependency — these types already appear in the public API (`App::create_render_target_with_format` / `App::load_image_with_format` both take a `wgpu::TextureFormat`), so a game already had to match the engine's `wgpu` version; the re-export just removes a redundant direct dependency. (Because `wgpu` is part of the public surface, a `wgpu` major bump remains a breaking change for such games — this only makes the existing coupling explicit.) Separately, the cross-platform `positional_audio` example (the P1+P2 `Audio`-facade arc) now has a `web/` harness so the stereo-panner positional audio can be heard in a browser, same code as native. No behavior change.

### Added
- **`pub use wgpu;`** at the crate root (`src/lib.rs`) — re-exports the `wgpu` crate so games can reference `wgpu::TextureFormat` (and other GPU types in the public API) without a direct `wgpu` dependency.
- `positional_audio` web harness (`examples/positional_audio/web/build.sh` + `index.html`) — `cargo build --example` + `wasm-bindgen` bundle over the example's existing `#[wasm_bindgen] run_positional_audio` entry point; a Start button unlocks the browser `AudioContext` and focuses the canvas, `?autostart=1` boots headless for a render smoke. `pkg/` is gitignored.

## 0.56.0

**HDR / linear render targets — a caller-chosen pixel format + a format-matched sprite pipeline.** Render targets were locked to the surface format; now `App::create_render_target_with_format(name, w, h, format)` creates one with any `wgpu::TextureFormat` (e.g. `Rgba16Float` for an HDR offscreen buffer, or a linear `Rgba8Unorm`). The offscreen pass threads the target's real format into the sprite renderer, which lazily builds and caches a sprite pipeline matching it — so an `OffscreenCamera` renders correctly into a non-surface-format target (previously a wgpu format mismatch). The single-surface-format fast path is untouched (no per-frame cost when no extra format is used). Tone-mapping an HDR target for final display is the game's responsibility. Additive — existing render targets and call sites are unchanged.

### Added
- **`App::create_render_target_with_format(name, w, h, format)`** (`src/app/assets.rs`) — a render target with a caller-chosen pixel format; `create_render_target` now delegates to it with the surface format.
- **`RenderTarget::format()`** (`src/renderer/render_target.rs`) — the target's pixel format (now stored on the target).
- **Format-matched sprite pipeline** (`src/renderer/sprite.rs`) — `SpriteRenderer` keeps a lazily-built cache of sprite pipelines keyed by target format (`extra_sprite_pipelines`); the offscreen pass (`src/app/render/offscreen.rs`) renders each target with the pipeline matching its format. UI and material pipelines remain surface-format (the offscreen pass is sprite-only).
- `hdr_render_target` example — an over-bright off-screen scene (sprite colours > 1.0) rendered into both an `Rgba16Float` HDR target and the default 8-bit one, shown side by side with an adjustable exposure. Lowering the exposure shows the HDR target keeping the bright core distinct from the mid square while the 8-bit target collapses them (it clamped both to 1.0 at store time).

### Changed
- Render targets are no longer fixed to the surface format; an `OffscreenCamera` targeting a non-surface-format render target renders with a matching pipeline instead of failing.

## 0.55.0

**`RonRegistry<V>` + `RonLoadable` are now public — a fork-friendly custom-asset registry.** The generic `name → value` RON registry that backs the engine's particle / dialogue / animation-clip config registries (with native canonical-path hot-reload) is now re-exported at the crate root. A game can register its **own** RON-loaded config type without forking the engine: implement `RonLoadable` for it (e.g. via `read_ron`) and use `RonRegistry::load` / `get` / `names`. Purely additive — no behavior change.

### Added
- **`RonRegistry<V>`** re-exported at the crate root (`engine::RonRegistry`) — a generic `name → value` registry; `insert`/`get`/`names` are cross-platform, `load`/`reload_path` (canonical-path hot-reload) are native-only.
- **`RonLoadable`** re-exported (`engine::RonLoadable`, native-only) — implement it for a config type to load it from a RON file via `RonRegistry::load`.
- `ron_registry` example — a game's own `CreatureStats` config loaded from `examples/assets/creatures/*.ron` into a `RonRegistry<CreatureStats>`, rendered as colour bars sized by HP, with `R` to hot-reload from disk.
- A doc example on `RonRegistry` and fork-facing module docs.

## 0.54.0

**Tracked 2D positional sound on the cross-platform `Audio` facade.** The facade gains a positional sound addressed by a stable **channel name**: `play_at_on_channel` starts a *looping* positional sound (distance-based volume + stereo pan), `update_position` repositions it each frame to follow a moving source, and `stop_channel` stops it. On native this maps to a new byte-based `AudioManager::play_bytes_at` + the existing positional channel; on web to a looping `Sfx` (a new `set_loop` path) with a stereo panner, tracked by name. A new `positional_audio` example drives it — an orbiting sound source with an arrow-key-movable listener — on native **and** web from the same code. Additive — existing facade/`AudioManager`/`WebAudio` call sites are unchanged.

### Added
- **`Audio::play_at_on_channel(channel, bytes, source, listener, max_dist, bus)`** (`src/audio_facade.rs`) — a looping 2D positional sound on a caller-named channel routed through a named bus; distance falloff (silent at `max_dist`) + stereo pan applied immediately, then tracked via `update_position`.
- **`Audio::update_position(channel, source, listener, max_dist)`** — repositions a named positional channel each frame (both backends expose `update_position`, so the facade needs no `cfg` split here).
- **`Audio::stop_channel(channel)`** — stops and forgets whatever plays on a named channel (a positional sound and/or a named tone).
- **`AudioManager::play_bytes_at(channel, bytes, repeat, source, listener, max_dist)`** (`src/audio/positional.rs`) — the byte-based analogue of `play_at` (which reads a file path), backing the facade's positional playback from `include_bytes!` clips.
- **`WebAudio::play_at_on_channel` / `update_position` / `stop_channel`** (`src/audio_wasm.rs`) — a looping `Sfx` (via a new `set_loop` path in the shared SFX builder) with a stereo panner, tracked by channel name in a `spatial_channels` map.
- `positional_audio` example — a cross-platform positional-audio demo (orbiting source, arrow-key listener movement, live volume/pan readout), native + web entry points.

### Changed
- The facade no longer **excludes** all positional audio — only *untracked* positional one-shots (`play_at`) stay native-only; tracked positional sound is now covered.

## 0.53.0

**Named tone channels + a low-pass filter on the cross-platform `Audio` facade — and `settings_menu` adopts it.** The facade gains caller-named, *trackable* tone channels: `play_tone_on_channel` plays a sustained/re-triggered tone you address by name, `is_channel_playing` reports whether it is still sounding (re-arm it when it drains), and `set_low_pass` / `clear_low_pass` toggle a low-pass filter on it (applied on the next play). On native this maps to `AudioManager` channels + `AudioEffect.low_pass_hz`; on web it tracks per-channel `OscillatorNode`s with an optional `BiquadFilterNode`. With these covered, the `settings_menu` example game drops its hand-written native-vs-wasm audio split (its sustained two-tone BGM + the muffle low-pass demo) and calls the facade directly — so it now runs, with audio, on the web too. Additive — existing facade/`AudioManager`/`WebAudio` call sites are unchanged.

### Added
- **`Audio::play_tone_on_channel(channel, freq, dur, vol, bus)`** (`src/audio_facade.rs`) — a synthesized tone on a caller-named channel routed through a named bus. Unlike fire-and-forget `play_tone`, a named channel is stable: a replay on the same channel cuts the previous tone, and it can be queried/filtered.
- **`Audio::is_channel_playing(channel)`** — whether a named tone channel is still sounding (to re-arm a sustained tone when it drains).
- **`Audio::set_low_pass(channel, cutoff_hz)` / `Audio::clear_low_pass(channel)`** — toggle a low-pass filter on a named tone channel, applied on the next play. Native: an `AudioEffect` with `low_pass_hz`; web: a `BiquadFilterNode`.
- **`WebAudio::play_tone_on_channel` / `is_channel_playing` / `set_low_pass` / `clear_low_pass`** (`src/audio_wasm.rs`) — the web backing: tracked per-channel `OscillatorNode`s (with scheduled stop times for liveness) and an optional `BiquadFilterNode` low-pass inserted into the channel's graph.
- web-sys `BiquadFilterNode` + `BiquadFilterType` features (`Cargo.toml`) — required for the wasm low-pass path.
- `audio_facade` example: a `G` key toggles a sustained two-tone BGM on named channels (kept alive via `is_channel_playing`) and `L` toggles a low-pass muffle on it (demonstrates the new API, audible on the web via the existing web harness).

### Changed
- **`examples/games/settings_menu`** now uses the `Audio` facade for all its audio (sustained BGM tones, UI blips, bus volumes, and the muffle low-pass) — its audio `#[cfg(target_arch = "wasm32")]` guards are gone (the one remaining native-only guard is `env_logger`, unrelated to audio); the example now builds and plays audio on the web.
- The facade's tone coverage no longer **excludes** the low-pass filter — only positional `play_at`, per-channel effects beyond the low-pass (pitch, attack/release envelopes), and automatic sidechains stay native-only.

## 0.52.0

**Synthesized tones on the cross-platform `Audio` facade — and the first games adopt it.** The facade gains `play_tone` / `play_tone_on_bus` (a pure sine tone, no clip bytes), so a dual-target game can make retro blips/beeps without bundling audio files. The web backend grows real tone synthesis (`OscillatorNode`), which `play_tone` had been excluded from before. With tones covered, the `shooter` and `survivor` example games drop their hand-written native-vs-wasm audio split and call the facade directly — and now play their sfx on the web too. Additive — existing facade/`AudioManager`/`WebAudio` call sites are unchanged.

### Added
- **`Audio::play_tone(freq, dur, vol)` / `Audio::play_tone_on_bus(freq, dur, vol, bus)`** (`src/audio_facade.rs`) — fire-and-forget synthesized sine tone, no clip bytes. On native it rides the same round-robin 16-voice ring as `play_sfx` (master bus, or a named bus); on web it routes `OscillatorNode → gain → master`/bus.
- **`WebAudio::play_tone` / `WebAudio::play_tone_on_bus`** (`src/audio_wasm.rs`) — the wasm tone synthesizer: an `OscillatorNode` with a short attack/release gain envelope (avoids click artifacts), the web analogue of native `AudioManager::play_tone`.
- web-sys `OscillatorNode` + `OscillatorType` features (`Cargo.toml`) — required for the wasm tone path.
- `audio_facade` example: a `T` key plays a synthesized tone (demonstrates `play_tone`; audible on the web via the existing web harness).

### Changed
- **`examples/games/shooter`** now uses the `Audio` facade for its fire/explosion tones — **all 5** `#[cfg(target_arch = "wasm32")]` audio guards removed; sfx now play on the web build.
- **`examples/games/survivor`** likewise adopts the facade for its tones — the audio cfg-guards are gone (the 4 remaining guards are all `GpuParticleEmitter`, genuinely native-only); sfx now play on the web build.
- The facade no longer **excludes** tone synthesis — only positional `play_at` and per-channel effects stay native-only.

### Notes
- **Native tone behavior change:** a game's `play_tone` calls now route through the shared round-robin voice ring (like `play_sfx`) instead of a fixed per-effect channel, so consecutive tones **overlap** rather than cutting each other off. For short one-shot sfx this is an improvement (fuller rapid fire); a game wanting a hard single-voice tone should manage that itself.
- **Native master-volume nuance still applies:** a tone sent to a *named* bus via `play_tone_on_bus` is not scaled by `set_master_volume` on native (native buses don't nest); use `set_bus_volume`. On web it nests under master.

## 0.51.0

**Cross-platform audio facade — write one audio path for native AND web.** New `Audio` type wraps the native `AudioManager` (rodio) and the wasm `WebAudio` (Web Audio) behind one bytes-keyed API, so a dual-target game writes its audio logic with **zero `cfg` guards** instead of a native arm + a wasm stub for every call. Additive — existing `AudioManager` / `WebAudio` code is unchanged, and the refactor that backs it is behavior-preserving.

### Added
- **`Audio`** (`src/audio_facade.rs`, re-exported un-gated at the crate root) — cross-platform facade: `play_sfx` / `play_sfx_on_bus`, `play_music` / `crossfade_music` / `stop_music`, `set_master_volume`, `set_bus_volume` / `bus_volume`, `duck_bus` / `release_bus` / `bus_duck`, `resume` (unlocks the web `AudioContext` after a user gesture; no-op on native), and `update` (ticks native fades/ducks; no-op on web). Clips are passed as encoded `bytes` (`include_bytes!`) — the only cross-platform clip source, since wasm has no filesystem.
- **`AudioFacadeSystem`** — built-in system that ticks the `Audio` resource's `update` each frame (the cross-platform analogue of the native-only `AudioSystem`).
- **`AudioManager::play_bytes`** and **`AudioManager::crossfade_bytes`** — byte-slice playback (the analogues of `play` / `crossfade`) for `include_bytes!` audio; these back the facade on native and are useful standalone for embedded audio.
- **example `audio_facade`** (`examples/audio_facade/`, native + web via `examples/audio_facade/web/`) — the same `Audio` code drives key-triggered SFX, looping music + crossfade, a "bed" mixer bus with volume + ducking, on both targets with no `cfg` guards.

### Notes
- **Native master-volume nuance:** on web, named buses nest under the master gain, so `set_master_volume` scales bus-routed sounds too; native buses do **not** nest, so a sound sent to a named bus via `play_sfx_on_bus` is **not** affected by `set_master_volume` on native (control it with `set_bus_volume`). Documented on the facade.
- **Out of scope:** tone synthesis (`AudioManager::play_tone`) and positional `play_at` are native-only and intentionally **not** on the facade — reach for the platform backend directly when a game needs them.

### Changed (internal)
- **`src/audio/playback.rs`** — `play_internal` and `crossfade` were refactored to share helpers (`append_decoded`, `begin_crossfade`) so the new `play_bytes` / `crossfade_bytes` reuse the existing decode → effects → pan/fade/repeat path. **Behavior-preserving:** the old sink is still torn down before the byte read (a failed read leaves the channel silent, as before), and the audio test suite stays green and untouched.

## 0.50.1

**ECS `World` unwrap hardening — every structural invariant in `world.rs` now names itself.** Behavior-preserving diagnostics pass: all 51 raw `.unwrap()` in `src/ecs/world.rs` became `.expect("<invariant>")` documenting why each is infallible. Under `[profile.release] panic = "abort"` an unwrap that ever fires aborts the whole process with no unwind, so a generic `unwrap() on None` at a line number is replaced by a message naming the broken invariant — far better triage if a future refactor breaks one. **No public API change, no behavior change** (`expect` and `unwrap` are codegen-identical on the happy path; the string only materializes on the cold panic path). The existing ECS test suite stays green and untouched. Closes engine-audit deferred **item 7** — the last open item of the 0.48.0 (seq-1) audit deferred list (items 1–8: 1–4 + 6 + 8 done, 5 rejected, 7 done now).

### Changed (internal)
- **`src/ecs/world.rs`** — the 51 unwraps fall into two infallible families, each given a descriptive `expect`: (a) **column presence** — `columns.get(&tid)` after an `arch.contains(tid)` filter (all `query*` / `par_query*`) or for a `tid` drawn from the archetype's own `type_set` (`despawn` / `move_entity` / `add_component`); (b) **downcast** — `downcast_ref/mut::<T>()` on a column fetched by `TypeId::of::<T>()`, which can only hold `T`. The entity allocation / generation / free-list paths were audited and were already defensively guarded (`.get_mut()` + `if let Some` + a `u32::MAX` overflow guard), so they needed no change — item 7 reduced to the `expect` documentation pass with no guard/test work. The single `.unwrap_err()` (binary_search insertion position) is intentional and left as-is.

## 0.50.0

**Texture upload pixel-format parameterization — load linear data textures, not just sRGB color art.** The CPU→GPU texture upload path hardcoded `Rgba8UnormSrgb`, so every loaded image was interpreted as sRGB-encoded color. This adds an additive way to choose the format, so a **data texture** (normal map, mask, height or lookup table) can be uploaded as `Rgba8Unorm` (linear) and sampled verbatim — without the sRGB→linear decode that would corrupt non-color bytes. The default path is unchanged. Closes engine-audit deferred **item 6** (0.48.0 audit). A truly differently-formatted *render target* (HDR `Rgba16Float`) was deliberately left out of scope: the sprite pipeline binds a single color-target format at construction, so an HDR target needs a format-matched pipeline — a separate, larger feature.

### Added
- **`App::load_image_with_format(path, format)`** — like `load_image` but uploads the texture with a caller-chosen `wgpu::TextureFormat`. Records the per-path format override (`App.pending_texture_formats`) and applies it when the pending texture is loaded at GPU init (call before `run()`, like `load_image`).
- **`SpriteRenderer::load_texture_with_format(device, queue, path, format)`** — public format-aware sibling of `load_texture` for forks with raw GPU access.
- **`Texture::from_rgba_with_format` / `from_path_with_format` / `try_from_path_with_format`** (`pub(crate)`) — carry the real bodies; the existing `from_rgba` / `from_path` / `try_from_path` are now thin wrappers passing `Rgba8UnormSrgb`, so all existing call sites are byte-identical.
- **Example `texture_format`** — loads one grayscale-ramp PNG twice with byte-identical pixels but two formats (sRGB vs `Rgba8Unorm` linear) and draws them side by side; the linear ramp's midtones render visibly brighter, demonstrating the format controls the sRGB decode. The example is the acceptance test (VISION loop).

**Additive only — no breaking change.** `Rgba8Unorm` textures stay sampleable by the ordinary sprite pipeline (both formats satisfy the `Float { filterable }` bind-group layout, so no new pipeline is needed). Default `load_image` behavior is unchanged.

## 0.49.4

**Tier-5 cleanups — pathfinding path-reconstruction dedup + a spatial-grid query fast path.** Two small behavior-preserving improvements from the 0.48.0 engine-audit deferred list (item 8):

### Changed (internal)
- **`src/pathfinding.rs`** — the identical "walk `came_from` back and reverse" tail of `find_path` and `find_path_diagonal` is now a shared `reconstruct_path` helper (was duplicated verbatim in both). All 16 pathfinding tests unchanged and green.
- **`src/collision/grid.rs`** — `SpatialGrid::candidates_in_aabb` gains a single-cell fast path: when the query AABB fits inside one grid cell (the common case for small colliders) it returns that bucket directly and skips the per-query dedup `HashSet` allocation, since an entity appears at most once in any one bucket. Multi-cell queries are unchanged (still deduped). New regression test `candidates_dedup_across_cells_and_single_cell_fast_path` covers both the dedup and the fast path. The scratch-buffer approach the audit suggested was rejected — it would need `&mut self` / interior mutability (the query API is `&self`), and the fast path captures most of the benefit without the API churn.

**No public API change.** The other audit item-8 candidates were left alone: the editor asset-browser `[ ]` thumbnail placeholder is a missing-feature stub (not a behavior-preserving cleanup), and the editor's named-magic-numbers were already centralized in 0.49.3 (`theme.rs`).

## 0.49.3

**Central editor theming constants — `src/app/editor/theme.rs`.** Behavior-preserving refactor that pulls the editor chrome's inline visual magic numbers — gizmo overlay colors + z-biases (999/1000), grid-overlay line width/alpha + cursor-readout alpha/font size, the central viewport frame fill, and the three docked-panel default/min/max sizes — into one named-constant module, so the editor look is tweakable in one place instead of scattered across `ui/gizmo.rs`, `ui/grid_overlay.rs`, and `ui/docked.rs`. Every constant equals the literal it replaced. The game-facing `SliderStyle` / `CheckBoxStyle` widget defaults in `src/ui/` were deliberately **left alone** — they are reusable widget styling (already named `Default` fields), not editor chrome, and pulling them into an editor module would invert the dependency. **No public API change** (constants are `pub(in crate::app::editor)`). Completes the 0.48.0 engine-audit deferred-item list (item 4, editor theming constants).

### Changed (internal)
- **`src/app/editor/theme.rs`** (new) — the editor theming constants. Gating mirrors the call sites: nearly all are native-only (`#[cfg(not(target_arch = "wasm32"))]`); `GIZMO_SELECT_COLOR` + `GIZMO_SELECT_Z_BIAS` stay cross-platform (the screen-space UI-node selection highlight that uses them compiles, dead, on wasm).
- **`src/app/editor/ui/gizmo.rs`**, **`src/app/editor/ui/grid_overlay.rs`**, **`src/app/editor/ui/docked.rs`** — inline literals replaced with `theme::*` references.

## 0.49.2

**Editor god-file split, part 2 — `gizmo.rs` shed its pure geometry math (1183 → 707 lines).** Behavior-preserving refactor: the side-effect-free anchor/resize/rotation math and the gizmo size/snap constants — `anchor_base`, `ui_drag_new_offset`, `handle_centers`, `hit_test_handles`, `ui_resize_new_layout`, `rotation_handle_pos`, `cursor_angle`, `snap_angle`, `applied_rotation`, and the `MIN_*`/`HANDLE_*`/`ROT_*` constants — moved into a new `gizmo_math.rs`, leaving `gizmo.rs` to hold only the `impl App` input-handling + rendering interaction logic. The 10 pure-math unit tests moved with them; the one App-level test (`rotation_gizmo_drag_rotates_and_undoes`, which drives `update_transform_gizmo_native`) stays in `gizmo.rs`. Pure code movement — only visibility (private items the interaction logic still calls became `pub(super)`), imports, and paths changed. **No public API change** (all moved items are `pub(crate)`/`pub(super)` crate-internal). Completes the 0.48.0 engine-audit deferred-item list (item 3, editor god-file split — `docked.rs` was 0.49.1).

### Changed (internal)
- **`src/app/editor/ui/gizmo_math.rs`** (new) — the pure gizmo geometry helpers + constants + their 10 unit tests, individually `#[cfg(not(target_arch = "wasm32"))]`-gated to mirror `gizmo.rs`.
- **`src/app/editor/ui/gizmo.rs`** — imports the math it needs from `gizmo_math`; keeps the `impl App` gizmo interaction logic and the one App-level rotation test.

## 0.49.1

**Editor god-file split — `docked.rs` shed three self-contained panels/overlays (1233 → 958 lines).** Behavior-preserving refactor following the existing `audio_panel.rs` / `data_table_panel.rs` pattern: the particle live-tuner, the lighting controls, and the world-aligned grid overlay each moved out of the docked-UI god-file into their own native-only modules. Pure code movement — only visibility, imports, and paths changed; the moved functions and their tests are otherwise verbatim. **No public API change** (all moved functions are `pub(in crate::app)` crate-internal). Continues the 0.48.0 engine-audit deferred-item list (item 3, editor god-file split).

### Changed (internal)
- **`src/app/editor/ui/particle_panel.rs`** (new) — `particle_tuner_grid` + its private `color_rgba_drags` helper.
- **`src/app/editor/ui/lighting_panel.rs`** (new) — `point_light_grid` + `ambient_light_control` + their private `color_rgb_drags` helper.
- **`src/app/editor/ui/grid_overlay.rs`** (new) — `draw_editor_grid` + `grid_lines_in_range` + the grid-lines unit test.
- **`src/app/editor/ui/docked.rs`** — drops those concerns; `point_light_grid`/`particle_tuner_grid` are now re-exported from their new homes via `ui/mod.rs`. `uv_rect_to_egui` and the Tile Paint swatch palette stay put.

## 0.49.0

**Generic `RonRegistry<V>` deduplicates the data-driven config registries.** Behavior-preserving refactor: `ParticleConfigRegistry`, `DialogueRegistry`, and `AnimationClipRegistry` each re-implemented the same `name → value` + `name → path` maps, `load`, canonical-path `reload_path`, and `HotReloadable` glue — the very triplication that produced the 0.48.0 dialogue `reload_path` divergence bug. They now wrap a shared internal `RonRegistry<V>` and keep their exact public types and signatures. The data-table registry stays bespoke (it tracks `dirty` and returns a richer `ReloadOutcome`). All existing tests are unchanged and green.

### Added
- **`ParticleConfigRegistry::insert`** — direct in-memory insert, matching the long-standing `DialogueRegistry`/`AnimationClipRegistry` method (the one small additive API in this refactor).

### Changed (internal)
- **`src/ron_registry.rs`** (new, crate-internal) — `RonRegistry<V>` + the `RonLoadable` trait factor the canonical-key hot-reload out once; covered by its own unit tests.
- **`src/particle/config_set.rs`**, **`src/dialogue/tree.rs`**, **`src/animation/clip_set.rs`** — the three registries are now thin wrappers over `RonRegistry<V>`; ~130 lines of triplicated reload/path logic removed.

## 0.48.1

**Scheduler topological sort — O((V+E)·log V) instead of O(V·E).** Behavior-preserving refactor of `compute_order` (`src/ecs/schedule.rs`): the Kahn ready-set was a `Vec` scanned with `iter().min()` + `retain` (O(V) per pop) and the relaxation rescanned the *entire* edge set on every pop (O(V·E)). It now builds an adjacency list once and uses a `BinaryHeap<Reverse<usize>>` min-heap, relaxing each out-edge exactly once while preserving the deterministic lowest-index tie-break. **No public API or behavior change** — identical execution order; the scheduler tests are unchanged and green.

### Changed (internal)
- **`src/ecs/schedule.rs`** — `compute_order` uses an adjacency list + min-heap (`Reverse`) ready-set; the cycle-detection `remaining` set is now derived from leftover in-degree (equivalent to the old `!order.contains(i)`, since a node is popped iff its in-degree reached 0).

## 0.48.0

**Engine-wide audit pass — logic-bug fixes, Rust↔WGSL drift guards, and fork-friendly knobs.** A 7-subsystem code audit produced this batch of correctness fixes (each with a regression test), internal hardening, and additive public API to un-hardcode values a fork would want to tune. Scope is `src/` only (29 files, +378/−39) — all API changes are additive, no example/game code changed, and the full CI gate is green.

### Added
- **`ScriptingLimits`** re-exported from the crate root — Rhai op/memory/depth limits were previously unreachable from `engine::` (`ScriptingSystem::with_limits` already consumed them).
- **`DEFAULT_CANVAS_ID`** const — single source for the wasm `<canvas>` element id (was the literal `"game-canvas"` in 4 places across `app/window.rs` + `renderer/context.rs`).
- **`GpuParticleConfig`** optional resource (`capacity`, default 4096; native-only) — sizes the GPU particle ring-buffer without forking (was a hardcoded `4096` in `app/render/frame.rs`).
- **`physics::DEFAULT_FRICTION` / `physics::DEFAULT_RESTITUTION`** consts, and **`PhysicsWorld::set_collider_friction` / `set_collider_restitution`** convenience setters (the bodies' `0.3`/`0.0` were baked into the `add_*` factories).
- **`PhysicsWorld::add_spring_joint(.., stiffness, damping)`** — tunable spring; `add_distance_joint` now delegates to it with named `DISTANCE_JOINT_STIFFNESS`/`DAMPING` defaults.
- **`CharacterController::drop_duration`** field + **`with_drop_duration`** builder; `CharacterController::DROP_DURATION` is now `pub`.

### Fixed
- **`TweenSequence::tick`** no longer infinite-loops when a *looping* sequence is built entirely from zero-duration segments (added a zero-crossing guard).
- **`AnimationStateMachine`** transitions with empty conditions no longer auto-fire every frame (`[].all() == true`); they stay inert until a condition is attached (the editor adds condition-less placeholders).
- **`SkeletalAnimator::play`** ignores out-of-bounds clip indices with a warning instead of silently freezing the animator (mirrors `AnimationPlayer::play`).
- **`pathfinding::find_path`** (cardinal) uses `saturating_add` for cost parity with `find_path_diagonal`.
- **`Wander`** steering carries leftover `dt` instead of zeroing the timer (no extra interval gap on slow frames).
- **`hierarchy::attach`** rejects attaching an entity to itself (was a corrupt `Parent(self)`/`Children([self])` cycle).
- **`DialogueRegistry::reload_path`** reloads from the stored registered path rather than the caller's path string (fixes hot-reload when the two canonicalize-equal but differ as strings).
- Guarded several collect-then-mutate `unwrap()`s against mid-frame despawn (`tilemap/system.rs`, `audio/ducking.rs`, `audio/playback.rs`) and the material draw-pass `HashMap` indexing (`renderer/sprite/draw.rs`).

### Changed (internal)
- **Rust↔WGSL drift guards:** size-assertion tests for `GpuParticle` (80 B), `InstanceRaw` (116 B), `UiInstanceRaw` (112 B) — only `LightingUniforms` had one before. `COMPUTE_WORKGROUP_SIZE` and `MAX_LIGHTS` are now single-sourced from Rust into the WGSL at shader-load (token substitution) instead of duplicated literals.
- **`input/gamepad_macos.rs`** carries a self-contained `#![cfg(target_os = "macos")]` gate (CI is ubuntu-only and never compiles it).

## 0.47.1

**F2 editor data-table UX — readable string cells + a freely-resizable bottom panel.** The Data Tables panel's string cells were stuck at ~40px (long sentences unreadable) and the bottom panel couldn't be dragged past ~300px. Root cause for the cells: inside an `egui::Grid`, a cell's `available_width()` is only the default `min_col_width` (~40px), and `TextEdit::desired_width` is clamped by `at_most(available_width)` — so setting `desired_width` alone has no effect. Editor-only, native-only; **no public API change**, no effect on games or the wasm build.

### Fixed
- **`src/app/editor/ui/data_table_panel.rs`** — string cells now use `ui.add_sized([STRING_CELL_WIDTH, h], TextEdit::singleline(..))` (260px) instead of `.desired_width(..)`; `add_sized` allocates a fixed-width region first, which both sizes the box and grows the grid column. The grid stays in a `ScrollArea::both`, so over-wide rows add horizontal scroll rather than squashing other columns.

### Changed
- **`src/app/editor/ui/docked.rs`** — bottom editor panel `size_range` cap raised `300.0 → 2000.0` (egui still clamps the drag to the real available height, so it can't cover the toolbar) and `default_size` `150.0 → 200.0` for more out-of-box room. The 300px cap was the actual limit on growing the panel.

## 0.47.0

**macOS gamepad support — a GameController-framework backend for `GamepadState`.** On macOS the engine's gilrs (IOKit-HID) backend can't read modern Bluetooth/USB Xbox & PS5 pads — Apple's GameController driver claims them, so gilrs enumerates the pad but receives no input (confirmed via `gamepad_probe`: gilrs all-zero, GameController full input). The engine now polls the GameController framework on macOS and feeds the same `GamepadState`, so gamepads work there with **no public API or game-code change** — `GamepadState` / `InputMap` behave identically across platforms.

### Added
- **`src/input/gamepad_macos.rs`** — macOS GameController backend: polls `GCController` each frame (from `about_to_wait`) and writes the full button/axis set into `GamepadState` (`apply_macos_snapshot`, which diffs held buttons against the previous frame to derive `just_pressed`/`just_released`). Stick / D-pad Y is negated to match the engine's `AxisBinding` convention (up = −Y), so games written against the gilrs path behave identically.
- **`objc2-game-controller`** (+ `objc2`, `objc2-foundation`) as a macOS-only dependency, pinned to the objc2 0.5 already in the tree (winit/wgpu). macOS-only — no effect on the wasm/Linux/Windows builds or those platforms' published crate.

### Changed
- **gilrs is no longer initialized on macOS** (`App::new`); it stays the backend on Windows/Linux. The `gamepad_probe` example is reframed from a bug demo into a backend cross-check (engine `GamepadState` vs a direct GameController read).

## 0.46.4

**`gamepad_probe`: add a throttled stdout log of the gilrs-vs-GameController verdict.** The 0.46.3 probe only rendered its comparison on-screen, so a terminal run showed nothing. It now also prints a compact line (~0.3 s apart, while a pad is active) tagging which backend receives input (`GC-only` / `HID-only` / `both`) with the raw stick/button/trigger values — capturable in the terminal and headless-friendly. Confirmed on macOS with a Bluetooth Xbox pad: gilrs/HID reads all-zero while GameController reads full input, empirically validating that a GameController-framework backend is the fix. Example-only; no library / public API change.

### Changed
- **`examples/gamepad_probe.rs`** — throttled stdout log of the per-backend snapshot + verdict; the GameController view is now read once per frame and reused by both the log and the overlay.

## 0.46.3

**Add a `gamepad_probe` diagnostic example — the first step of the per-OS gamepad input pass.** On macOS the engine's gilrs (IOKit-HID) backend enumerates modern Xbox/PS5 pads but receives no input reports, because Apple's GameController framework claims them. This example renders, side by side every frame, what gilrs/HID sees (the engine's `GamepadState`) versus what the GameController framework sees, so running it on a Mac with a pad empirically confirms the input path and whether a GameController-framework backend is the fix. No library / public API change.

### Added
- **`examples/gamepad_probe.rs`** — side-by-side gilrs(HID) vs macOS GameController live view (left stick / face buttons / triggers) with a self-explaining verdict line. The GameController column is macOS-only (`#[cfg(target_os = "macos")]`); on Windows/Linux the gilrs/HID path is the live one.
- A **macOS-only dev-dependency** `objc2-game-controller` (pinned to 0.2 to reuse the `objc2` 0.5 already pulled by winit/wgpu — no new objc2 version). The GameController FFI lives entirely in the example for now; promoting it to a real `GamepadState` backend is the follow-up, gated on hardware confirmation that the framework delivers input where HID does not.

## 0.46.2

**Subset the bundled Korean editor font so the crate fits the crates.io 10 MB package limit.** The full Noto Sans KR Regular added in 0.46.0 was ~5.9 MB, which pushed the packaged crate to ~11.7 MB compressed — over the 10 MB publish ceiling (CI's `cargo package` dry-run does not enforce it, so this only blocked an actual publish). The bundled font is now a ~2.3 MB Hangul-only subset; the package is **9.7 MB compressed**. No public API change; the editor still renders all modern Korean. Also fixes a latent licensing gap (the OFL text for the font was never bundled).

### Changed
- **`assets/fonts/NotoSansKR-Regular.ttf` → `assets/fonts/NotoSansKR-Regular-subset.ttf`** — subset (via `pyftsubset`) to Basic Latin/Latin-1, the full modern Hangul Syllables block (U+AC00–U+D7A3, all 11172), Hangul Jamo, CJK + general punctuation, fullwidth forms, and ₩; the Hanja (CJK ideographs) and the OpenType layout / hinting tables (unused by egui's `ab_glyph` rasterizer) were dropped. Archaic Hangul and Hanja are no longer covered. `src/debug_ui.rs` embeds the new file.

### Added
- **`assets/fonts/NotoSansKR-OFL.txt`** — the SIL OFL 1.1 license for Noto Sans KR (previously missing), noting the bundled file is a subset ("Modified Version"); the primary font name stays "Noto Sans KR" and does not use the Reserved Font Name.
- **`scripts/subset_korean_font.sh`** — reproducible regeneration of the subset (documents the source, the kept Unicode ranges, and the exact `pyftsubset` invocation).
- A regression test (`src/debug_ui.rs`) that loads the bundled subset with `ab_glyph` (egui's rasterizer, added as a dev-dependency) and asserts it parses and carries the Hangul glyphs the editor relies on — so a future bad re-subset fails loudly instead of shipping `□` tofu.

## 0.46.1

**Editor localization coverage fix — translate the last few user-facing strings the 0.46.0 pass missed.** A coverage audit of the in-game editor found two spots whose text was built with `format!` (not an inline egui call) and so escaped the 0.46.0 `tr(en, ko)` sweep. No public API change; behavior is identical apart from the now-localized text.

### Fixed
- **`src/app/editor/prefab.rs`**: the prefab save/load status messages (`Saved prefab`, `Save failed`, `No entity to save`, `Spawned prefab from`, `Load failed`), shown in the docked editor, are now wrapped with `tr(en, ko)`.
- **`src/app/editor/ui/mod.rs`**: the inspector's `Entity {index}:{generation}` fallback label (shown for an untagged entity) and the matching default name written by "Add Name" are now localized, keeping the displayed label and the pre-filled name in sync with the active locale.

## 0.46.0

**Korean (CJK) support + a Korean-by-default localization for the in-game editor.** The egui editor / debug overlay previously rendered CJK text as `□` (tofu) because egui's default fonts cover only Latin + Cyrillic. Two parts fix this:

1. **Bundled Korean font.** Noto Sans KR (Regular) ships in `assets/fonts/` and is installed as the **lowest-priority egui fallback** in `DebugUi::new_with_ctx` — Latin/Cyrillic keep the default font (unchanged look/metrics); the Noto fallback is consulted only for glyphs the default lacks (Hangul, other CJK). This also makes Korean *data* (e.g. RON data-table values, entity tags) render correctly in the editor.
2. **Editor localization layer.** A lightweight `tr(en, ko)` helper (`src/app/editor/i18n.rs`) translates editor UI strings at the call site; **English is the source of truth**, Korean is inline. The active `EditorLocale` is a thread-local set each frame from the persisted editor settings and **defaults to Korean**. A toolbar button toggles English ⇄ 한국어 (persisted to `editor_settings.ron`), so a forker can switch the editor back to English. ~130 user-facing editor strings across the docked + overlay UI are now localized; component registration keys, ids, file paths, and data-derived labels are deliberately left untranslated.

### Added
- **`assets/fonts/NotoSansKR-Regular.ttf`** + `DebugUi`'s `install_korean_fallback` (egui `add_font`, `FontPriority::Lowest`).
- **`src/app/editor/i18n.rs`**: `EditorLocale {English, Korean}` (default Korean), thread-local active locale, `set_locale`/`locale`/`tr(en, ko)`.
- **`EditorSettings::locale`** (`#[serde(default)]`, RON-persisted) + `EditorState::locale`; an EN/한국어 toggle button in the docked toolbar.

### Changed
- Editor UI files under `src/app/editor/` wrap user-facing strings in `tr(..)`; `update_editor_ui` publishes the active locale each frame.

## 0.45.0

**Rounded corners for UI rects and the keyboard-focus ring.** `DrawRect` gains two optional, default-off knobs — `corner_radius` (round the corners) and `border` (draw only an inset outline ring of that width instead of a fill) — rendered by a new dedicated UI pipeline + SDF shader. `FocusRingStyle::corner_radius` rides the same machinery so the engine's focus ring can be rounded. **Additive and byte-identical by default**: `corner_radius == 0.0 && border == 0.0` takes a shader fast path that renders exactly like the old plain quad, so every existing `DrawRect`/`DrawImage` and the sharp four-bar focus ring are unchanged. The sprite pipeline (`InstanceRaw` / `sprite.wgsl`) is untouched — the UI primitive pass now uses its own `UiInstanceRaw` + pipeline.

### Added
- **`src/renderer/shaders/ui.wgsl`** (new): UI primitive shader with a rounded-rect SDF — filled when `border == 0`, an inset outline ring otherwise; a `corner == (0, 0)` fast path identical to the prior plain quad.
- **`src/renderer/sprite/geometry.rs`**: `UiInstanceRaw` (model / color / uv + `px_size` + `corner = [radius, border]`) and its vertex buffer layout.
- **`src/renderer/sprite.rs`**: a dedicated `ui_pipeline` (own shader, reusing the camera + texture bind-group layouts).
- **`src/renderer/ui.rs`**: `DrawRect::{corner_radius, border}` fields + `with_corner_radius` / `with_border` builders, with unit tests.
- **`src/ui/focus.rs`**: `FocusRingStyle::corner_radius` (default `0.0` → the historical sharp ring).
- **`examples/ui_rounded.rs`** (new): the acceptance example — a rounded card panel, the three fill modes (sharp fill / rounded fill / rounded outline), and a rounded focus ring you can Tab around.

### Changed
- **`src/renderer/sprite/ui_primitives.rs`**: the UI primitive pass builds `UiInstanceRaw` (threading each rect's `corner`; images pass `[0, 0]`) and draws through `ui_pipeline`.
- **`src/ui/system/focus_pass.rs`**: `push_ring` emits a single rounded outline `DrawRect` when `corner_radius > 0`, otherwise the historical four border bars.
- **`examples/diagonal_pathing.rs`, `examples/loading_bar.rs`**: converted their direct `DrawRect { … }` struct literals to the `DrawRect::new(...)` builder (future-proof against added fields).

## 0.44.0

**Optional "breathing" pulse for the keyboard-focus ring.** `FocusRingStyle` gains two fields — `pulse_hz` (cycles/sec) and `pulse_min_alpha` (alpha at the trough, a fraction of the ring color's alpha) — that fade the focus ring's alpha on a raised sine to draw the eye to the focused widget. **Additive and off by default** (`pulse_hz = 0.0` → a steady ring, byte-identical to before), so existing call sites and the historical amber 3px default are unchanged. The pulse clock is accumulated inside `UiSystem` (wrapped so `+= dt` never loses precision) and threaded into the focus pass, mirroring the existing cursor-blink timing.

### Added
- **`src/ui/focus.rs`**: `FocusRingStyle::pulse_hz` + `pulse_min_alpha` fields (both default to the no-pulse case) and a `pulse_alpha(t) -> f32` helper returning the `[pulse_min_alpha, 1.0]` multiplier (a flat `1.0` when no pulse is configured). Unit tests cover the disabled/unity case, the oscillation range + peak/trough, and negative-min clamping.
- **`examples/ui_focus.rs`**: the demo's cyan ring now pulses (`pulse_hz: 1.2`, `pulse_min_alpha: 0.35`) — the acceptance example for the feature.

### Changed
- **`src/ui/system.rs`**: `UiSystem` accumulates a wrapped `ring_elapsed` clock and passes it to the focus pass.
- **`src/ui/system/focus_pass.rs`**: `push_ring` takes the elapsed time and multiplies the ring color's alpha by `style.pulse_alpha(elapsed)`; a non-pulsing style is unaffected.

## 0.43.6

**Fix `DrawText::centered` horizontal drift (downstream wishlist EW-001).** A no-bounds centered text (`anchor = Center` + `align = Center`) rendered its horizontal center ~half the viewport to the *right* of `position` whenever `position.x` was off-center: the layout buffer is the full viewport width (so centered titles don't wrap early), `align = Center` distributes glyphs around the *buffer* center, but the anchor offset still subtracted `max_w / 2` — a left-aligned assumption. The offset is now measured from the actual shaped glyph extents, so the rendered center lands exactly on `position.x` for any alignment. **No public API change**; the anchor=Center + default-Left-align combination (the game's workaround) is byte-identical, since for left-aligned text the measured center reduces to `max_w / 2`.

### Fixed
- **`src/renderer/text/renderer.rs`**: new `shaped_center_x(&Buffer)` helper measures the rendered horizontal center from glyph extents (`min(glyph.x) .. max(glyph.x + glyph.w)`); the `TextAnchor::Center` anchor offset now uses it instead of `max_w / 2`. Correct for `Left`/`Center`/`Right`/`End`/`Auto`.
- **`src/renderer/text/tests.rs`**: headless-shaping regression tests `ew001_centered_text_center_lands_on_position_x` (centered text at an off-center x lands on `position.x`, and the old `max_w/2` offset is shown to drift) and `ew001_left_align_center_anchor_unchanged` (the Left-align workaround is unaffected). Uses the bundled DejaVu Sans for deterministic glyph metrics; no GPU needed.

## 0.43.5

**Behavior-preserving split of `src/renderer/text.rs` and `src/renderer/sprite.rs` (P5 — the final step of the engine-hardening refactor sweep).** The 1102-line text module is broken into `src/renderer/text/` (`queue`/`cache`/`rich_text`/`renderer`/`tests`), and `sprite.rs`'s ~540-line `render()` is decomposed into its collect / batch / draw phases under `src/renderer/sprite/`. **No public API change, no behavior change**: `engine::{DrawText, TextAlign, TextAnchor, TextQueue, TextRenderer}` and `engine::renderer::{SpriteRenderer, FrameContext}` re-export unchanged; 883 lib tests unchanged; wasm build green.

### Changed (internal)
- **`src/renderer/text.rs` → `src/renderer/text/`**: `queue.rs` (`DrawText`/`TextAnchor`/`TextAlign`/`TextQueue`), `cache.rs` (shaped-buffer cache types), `rich_text.rs` (markup parser), `renderer.rs` (`TextRenderer` + font/layout helpers), `tests.rs`. `text.rs` is now a 17-line module root.
- **`src/renderer/sprite.rs`**: `render()`'s phases extracted to `sprite/collect.rs` (`collect_draw_entries`), `sprite/batch.rs` (`batch_and_upload`), `sprite/draw.rs` (`record_draw_pass`) as `pub(super)` helpers; `render()` is now three sequential calls and `sprite.rs` shrank 825 → 355 lines. Logic moved verbatim. Cross-submodule helpers are `pub(super)`; public types/re-exports untouched.

## 0.43.4

**Behavior-preserving split of `src/app/editor.rs` (P2 of the engine-hardening refactor sweep — the highest-risk file).** The 1509-line editor module — which mixed native-only editor internals with cross-platform public registration/loading API — is broken into focused files under `src/app/editor/`, leaving `editor.rs` a 43-line module root (declarations + re-exports). **No public API change, no behavior change**: 883 lib tests unchanged, and every `#[cfg(target_arch = "wasm32")]` boundary is preserved — the cross-platform `App::register_editable_component`/`register_serde_component`/`load_data_table`/`load_animation_clips`/`load_particle_configs`/`load_dialogue` stay compiled on wasm while the native-only editor internals stay gated (the `cargo build --target wasm32-unknown-unknown` gate is green).

### Changed (internal)
- **`src/app/editor.rs` → `src/app/editor/` submodules**: `history.rs` (`EditorCmd`/`EditorHistory`), `settings.rs` (`EditorSettings` + persistence), `prefab.rs` (`entity_to_def` + copy/paste/prefab), `overlays.rs` (debug-bounds / pathfinding overlays + inspector resets), `component_registry.rs` (component registration — native-only and cross-platform `impl App` blocks), `loading.rs` (data-table / clip / particle / dialogue loaders), `util.rs` (small helpers), `tests.rs`. The `component_registry` and `loading` module declarations are intentionally un-gated (their native-only contents keep inner `#[cfg(not(target_arch = "wasm32"))]`); cross-module `pub(super)` items were raised to `pub(in crate::app)` to preserve reachability. Operational code and public API untouched.

## 0.43.3

**Behavior-preserving split of `src/app/render.rs` (P1 of the engine-hardening refactor sweep).** The 1200-line frame-render orchestration is broken into a `src/app/render/` directory by concern — `debug_draw.rs` (`DebugShape` → `DrawRect`), `offscreen.rs` (offscreen `RenderTarget` rendering), `docked.rs` (native-only docked-editor RT), `post_lighting.rs` (post-process + lighting setup), and `frame.rs` (`render` + `step_frame`/`step_frame_once` orchestration) — with `render/mod.rs` as the module root. **No public API change, no behavior change**: 883 lib tests unchanged, every `#[cfg(target_arch = "wasm32")]` boundary preserved (`RenderState` stays in `render_state.rs`; the `docked` submodule is gated native-only).

### Changed (internal)
- **`src/app/render.rs` → `src/app/render/` submodules** (`debug_draw`/`offscreen`/`docked`/`post_lighting`/`frame` + `mod.rs`). Cross-submodule `App` methods were raised from `pub(super)`/private to `pub(in crate::app)` to preserve their existing reachability after the move (no new public surface); a handful of now-unused `use` imports were dropped from `src/app.rs`. Operational code and public API untouched.

## 0.43.2

**Behavior-preserving split of `src/network.rs` (P4 of the engine-hardening refactor sweep).** The 1372-line network module is broken into a `src/network/` directory — `event.rs` (`NetworkEvent`/`NetworkConfig` + the queue-size consts), `native.rs` / `wasm_impl.rs` (the cfg-gated `NetworkClient` WebSocket bodies), `system.rs` (`NetworkSystem`), `remote_entities.rs` (`RemoteEntities`), `snapshot.rs` (`SnapshotBuffer`), and `tests.rs` — with `network.rs` reduced to the module root (declarations, re-exports, the shared `push_event_bounded` helper). **No public API change, no behavior change**: `engine::{NetworkClient, NetworkConfig, NetworkEvent, NetworkSystem, RemoteEntities, SnapshotBuffer}` and the `engine::network::*` consts are re-exported unchanged; all `#[cfg(target_arch = "wasm32")]` gating is preserved (the wasm build is green); 883 lib tests unchanged.

### Changed (internal)
- **`src/network.rs` → `src/network/` submodules** (`event`/`native`/`wasm_impl`/`system`/`remote_entities`/`snapshot`/`tests`). Three previously module-private fields became `pub(super)` to keep the moved sibling `tests` module's existing access; two intra-doc links were qualified to `crate::…`. Operational code and public API untouched.

## 0.43.1

**Behavior-preserving test extraction (P3 of an engine-hardening refactor sweep).** Five oversized modules each had their bottom-of-file `#[cfg(test)] mod tests { … }` block moved verbatim into a sibling `tests.rs` child module, shrinking the operational files without touching runtime code. **No public API change, no behavior change** — the 883 lib tests are unchanged (same `use super::*` visibility, the test module is still a child of its parent), and the full verify gate is green.

### Changed (internal)
- **Test modules split out** to their own files (declared `#[cfg(test)] mod tests;`):
  `src/animation/state_machine.rs` → `state_machine/tests.rs`, `src/tilemap/autotile.rs` →
  `autotile/tests.rs`, `src/dialogue/mod.rs` → `dialogue/tests.rs`, `src/prefab.rs` →
  `prefab/tests.rs`, `src/save.rs` → `save/tests.rs`. Operational code and public API untouched.

## 0.43.0

**Hexagonal autotiling now ships a real 64-tile "blob" atlas — the hex analogue of the square `blob_47`.** The `hex_6`/`hex_6_flat` constructors have existed since 0.39.0, but no 64-tile atlas backed them (the `hex_autotile` examples used a 2-tile interior/edge rule). This release adds the missing assets and full-blob examples so every open hex edge is outlined correctly, not just "interior vs edge". **No public API change** — assets + examples only.

### Added
- **`gen_hex_autotile_sheet`** (`examples/gen_hex_autotile_sheet.rs`): a deterministic, manual asset generator (the hex sibling of `gen_autotile_sheet`) that procedurally draws `examples/assets/hex_autotile.png` (pointy-top, 8×8 of 64×74 cells) and `examples/assets/hex_autotile_flat.png` (flat-top, 8×8 of 74×64 cells). Each cell renders a regular hexagon (vertices fill the cell, matching `Tilemap::cell_render_size`); an edge whose neighbor bit is CLEAR is outlined, a connected side blends to the boundary so filled hexes tessellate. **Tile index == the 6-bit `Hex6`/`Hex6Flat` neighbor mask**, lining up with `TilemapAutotile::hex_6(base)` / `hex_6_flat(base)`'s identity mask→tile map.
- **`hex_blob_autotile`** + **`hex_blob_autotile_flat`** examples: full-blob dig/fill demos (`TilemapProjection::Hexagonal` + `hex_6`, and `HexagonalFlat` + `hex_6_flat`) over the new atlases. A solid field with carved holes auto-outlines its rim and every hole, recomputed reactively by `TilemapSystem`. The VISION acceptance test for the 64-tile hex blob; both orientations verified on-screen.

## 0.42.0

**The UI focus ring is now restyleable via the `FocusRingStyle` resource.** The focus pass's
previously hardcoded `RING_COLOR`/`RING_THICKNESS` constants move to a `FocusRingStyle` World
resource (`color` / `thickness` / `enabled`), auto-inserted with the default amber 3px ring so
existing behavior is byte-identical. Insert your own to recolor/resize it, or set `enabled = false`
(or `thickness <= 0.0`) to suppress the engine ring entirely and draw your own focus indicator.
Additive — default behavior is unchanged.

### Added
- **`FocusRingStyle`** (`src/ui/focus.rs`): a `Copy` World resource for the focus-ring appearance
  (`color: Color`, `thickness: f32`, `enabled: bool`) plus `is_visible()`. `Default` reproduces the
  historical hardcoded ring exactly (amber `rgba(1.0, 0.85, 0.3, 1.0)`, 3px). Auto-inserted in
  `insert_core_resources` next to `UiFocus`; re-exported at the crate root.
- **`focus_pass` reads the resource** (`src/ui/system/focus_pass.rs`): `push_ring` is styled by the
  resolved `FocusRingStyle` (falling back to the default when the resource is absent) and draws
  nothing when the style is not visible. The hardcoded `RING_COLOR`/`RING_THICKNESS` constants are
  removed.
- 5 tests (3 `push_ring` unit tests for custom color/thickness, disabled/zero-thickness, and the
  default-appearance contract; 2 `UiSystem` integration tests confirming the resource is consumed
  end-to-end into the `UiQueue` and that a disabled style draws no ring); lib tests 878 → 883.
  Example `ui_focus` restyles the ring to a thicker cyan one to demonstrate.

## 0.41.0

**Left analog stick now drives UI focus navigation, alongside the existing D-pad.** The
`UiSystem` focus pass folds the first connected pad's left stick into its per-frame input snapshot:
push **Up/Down** to cycle focus across widgets, **Left/Right** to nudge a focused `Slider`. The
stick is edge-detected (one push = one focus step, no auto-repeat) so it behaves like the D-pad
rather than spraying steps while held. Additive — keyboard and D-pad navigation are unchanged, and
**no public API change** (the new `StickNav` edge detector is `pub(super)`).

> **Hardware verification deferred.** The stick logic is covered by 8 new tests (4 `StickNav` unit +
> 4 focus-pass integration) and its axis signs match the engine's existing `AxisBinding` convention
> (see the `survivor` example: up = −Y, down = +Y, right = +X). Real-pad confirmation is pending: on
> macOS, gilrs (IOKit HID) enumerates a Bluetooth/GameController-claimed Xbox controller — so it
> connects — but the OS routes its input through Apple's GameController framework, so gilrs receives
> no button/axis events. This is an environment limitation, not a defect (the existing `survivor`
> gamepad support hits the same wall there); it will be revisited during per-OS input optimization.

### Added
- **Left analog stick → UI focus nav** (`src/ui/system/state.rs`): new `StickNav` per-axis edge
  detector with hysteresis (0.6 activate / 0.35 release) converts the continuous left stick into
  discrete D-pad-style steps. `InputSnapshot::from_world` now takes `&mut StickNav` and folds the
  stick in next to the D-pad; `UiSystem` holds the `StickNav` across frames alongside its scratch
  buffers. Left-stick Up/Down cycle focus (Up = reverse, like Shift+Tab), Left/Right nudge a focused
  `Slider`.
- **`GamepadState::test_axis`** (`src/input/gamepad.rs`, `#[cfg(test)]`): mirrors `test_press` for
  analog input, letting non-`gilrs` tests drive the stick.
- 8 tests (`StickNav` hysteresis unit tests + focus-pass integration tests for advance/reverse/wrap,
  held-no-repeat, and slider nudge); lib tests 870 → 878. Example `ui_focus` updated to advertise the
  left stick.

## 0.40.2

**Behavior-preserving dedup of the two hex autotile bitmask functions.** Internal refactor only —
**no public API change** and no behavior change; the 31 autotile tests (incl. the parity-dependent
hex offset and all-six-neighbor cases) and the full verify gate confirm parity.

### Changed (internal)
- **Hex autotile mask dedup** (`src/tilemap/autotile.rs`): `hex6_mask` (pointy-top, odd-r) and
  `hex6_flat_mask` (flat-top, odd-q) each open-coded the same six `if filled(..) { mask |= bit }`
  accumulation. Both now build a `[(drow, dcol); 6]` offset table (in ascending bit order) and
  delegate to a shared `hex_mask_from_offsets()` accumulator; each layout's distinct bit order and
  parity-dependent offsets stay explicit in its table.

## 0.40.1

**Behavior-preserving cleanup of two deferred code-review items (audio / UI focus).** Internal
refactors only — **no public API change** and no behavior change; the verify gate (870 lib tests) and
the wasm audio smoke (38/38) confirm parity.

### Changed (internal)
- **wasm `WebAudio` positional dedup** (`src/audio_wasm.rs`): `play_at` and `play_at_on_bus` shared an
  identical `update_position`-then-return tail over differently-routed SFX. Both now delegate to a
  shared private `play_at_to(dest)` helper (master vs. bus gain is the only difference), mirroring the
  existing `play_sfx`/`play_sfx_on_bus` → `play_sfx_to` structure.
- **`focus_pass` membership cost** (`src/ui/system/focus_pass.rs`): the per-frame focus sync tested
  membership against the index-sorted focusables list with two linear `contains` scans and cloned the
  scratch vector each frame. Both `contains` calls now use an `is_focusable()` binary search (`O(log n)`)
  and the redundant `focusables_snapshot` clone is gone.

## 0.40.0

**Code-review hardening of the 2026-06-18 feature arc (audio / dialogue / UI focus / save) +
cleanups.** A multi-angle review of the v0.32→v0.39 work surfaced several real bugs (mostly
edge-case races and conditional-path footguns); this release fixes them. The one breaking change is
`TilemapProjection` becoming `#[non_exhaustive]` (external exhaustive `match`es must add a wildcard
arm) — a MINOR under the 0.x cadence. wasm audio + save smokes pass (38/38, 7/7); 870 lib tests.

### Fixed
- **wasm `WebAudio` races** (`src/audio_wasm.rs`): `Sfx::stop()` called before the async decode
  finished was a no-op (the sound played anyway) — now a shared `stopped` flag suppresses the
  deferred `start()`. Rapid `crossfade_music`/`play_music` calls within the decode window orphaned a
  looping track that could never be stopped — a `music_gen` generation guard makes a superseded
  pending track stop itself. `start_music` connected the per-track gain to master *before* decoding,
  leaking a dead node on decode failure — the gain is now created/connected only after decode
  succeeds.
- **Dialogue conditional-choice deadlock** (`src/dialogue/mod.rs`): the vars-unaware
  `advance`/`choose`/`pending_choices` ignored choice conditions, so a line whose choices were all
  `cond`-gated blocked `advance()` forever (and `choose(i)` could pick a hidden choice). The plain
  API now considers only *unconditional* choices (the vars-aware `advance_with`/`dialogue::*` path is
  unchanged); no-condition dialogues are byte-identical.
- **Dialogue typewriter on bad data**: a non-finite `chars_per_sec` (e.g. `NaN` from malformed RON)
  rendered the line blank — the reveal guards now treat non-finite as "reveal instantly".
- **UI focus split-authority** (`src/ui/system/`): `focus_pass` and `text_input_pass` both wrote
  `ti.focused` with conflicting click semantics, so clicking outside a focused `TextInput` fired a
  spurious `TextBlurred` + dropped a frame of input. `focus_pass` is now the single owner of
  `ti.focused` and the `TextFocused`/`TextBlurred` events; `Enter`-to-submit clears `UiFocus` so the
  field isn't re-focused next frame.
- **wasm save parity** (`src/save.rs`): `read_ron` on wasm lacked native's `SAVE_MAGIC` fallback, so
  reading an AEAD-saved key returned a confusing parse error instead of decrypting — the wasm branch
  now mirrors native (hex-decode → magic check → decrypt, else plaintext RON).

### Changed
- **`TilemapProjection` is now `#[non_exhaustive]`** (`src/tilemap/mod.rs`) — matches the engine's
  other growable enums (`DebugShape`, `ReflectValue`, `Easing`); external exhaustive matches must add
  a `_ =>` arm. Breaking, hence MINOR.

### Added
- **`examples/hex_autotile_flat.rs`** — the flat-top (`HexagonalFlat` + `Neighborhood::Hex6Flat`)
  counterpart of `hex_autotile`, closing the VISION "an example exercises it" gap for flat-top hex
  autotiling.

### Changed (internal)
- `spatial_params` (linear distance falloff + x-pan) deduplicated into a cross-platform
  `src/audio_spatial.rs` (`pub(crate)`), shared by native `AudioManager` and wasm `WebAudio` (was a
  byte-for-byte copy in each). Autotile's duplicated bounds-check closure and the `hex_6`/`hex_6_flat`
  constructors are unified via private helpers. No behavior change.

## 0.39.0

**Autotiling across isometric and hexagonal projections.** Autotile bitmasks are computed from the
`tiles[row][col]` grid topology, so they already worked on **isometric** maps unchanged (iso is the
same square grid as orthographic, just rendered as diamonds) — now confirmed + tested. For **hex**
maps, two new neighborhoods compute the correct 6 parity-aware neighbors: `Neighborhood::Hex6`
(pointy-top, odd-r) and `Hex6Flat` (flat-top, odd-q). **Additive** — `Edge4`/`Blob8` unchanged.

### Added
- `Neighborhood::Hex6` (bits E=1, W=2, NE=4, NW=8, SE=16, SW=32) and `Neighborhood::Hex6Flat`
  (N=1, S=2, NE=4, SE=8, NW=16, SW=32) — 6-neighbor hex masks (`0..64`); the four diagonal offsets
  shift with row parity (odd-r) / column parity (odd-q) to match the staggered hex layout.
- `TilemapAutotile::hex_6(base)` / `hex_6_flat(base)` — 64-tile single-terrain hex autotile layouts
  (`mask → base + mask`), the hex analogue of `edge_16`/`blob_47`.
- Example `hex_autotile` — a pointy-top hex map with an interior-vs-edge `Hex6` rule (grass interior /
  sand open-edge) over the existing 2-tile hex atlas; dig holes and the rim re-tiles reactively.
- Unit tests: Hex6 / Hex6Flat interior + parity-dependent offsets, the hex constructors, and an
  isometric-autotile test confirming the square neighborhoods carry over.

## 0.38.0

**Flat-top hexagonal tilemap projection.** `TilemapProjection::Hexagonal` (v0.29.0) was pointy-top
only; the new `HexagonalFlat` variant is the **flat-top** counterpart in odd-q offset coordinates
(odd columns shifted down by half a tile) — the 90°-rotated mirror. `tile_size` is the flat-to-flat
**height**, and a flat-top hex is wider than tall. All four projection methods branch on it, so
`TilemapSystem` renders + picks it automatically. **Additive** — existing projections unchanged.

### Added
- `TilemapProjection::HexagonalFlat` — flat-top hex, odd-q offset. `cell_center_world` (col pitch
  `tile_size·√3/2` + odd-col half-shift-down), `cell_at_world` (flat-top pixel→axial→cube-round→
  odd-q), `cell_render_size` (`tile_size·2/√3 × tile_size`, wider than tall), `cell_z` (`-1`, no
  overlap).
- Example `hex_tilemap_flat` + generated `examples/assets/hex_tiles_flat.png` (flat-top hex atlas).
- 4 unit tests (odd-col offset, center↔world round-trip, off-center picking, render-size/z).

## 0.37.0

**Gamepad navigation for UI keyboard focus.** The focus pass (v0.31.0) was keyboard + mouse only.
It now also reads the first connected gamepad: **D-pad Down/Up** cycle focus (Up = reverse, like
Shift+Tab), **D-pad Left/Right** nudge a focused slider, and **A** (South) activates the focused
button/checkbox. Folded into `InputSnapshot` alongside the keyboard, so the existing focus-pass logic
(ring, activation, slider nudge, TextInput sync) is reused unchanged. **Additive** — no pad / no
`GamepadState` resource is a no-op; keyboard + mouse behavior is identical.

### Added
- Gamepad focus navigation in `UiSystem`'s focus pass (`src/ui/system/state.rs`): D-pad
  Up/Down/Left/Right + A from `GamepadState::primary()`.
- `ui_focus` example help text updated to mention the gamepad controls.

### Notes
- D-pad only (digital, edge-detected via `just_pressed`); the analog stick is not used (would need
  per-frame threshold debounce). Real-pad operation is a human check; the focus-move/activate logic
  is covered by unit tests via a new `GamepadState::test_press` test helper.

## 0.36.0

**Positional audio on a mixer bus for the wasm `WebAudio` path.** `play_at` (0.35.0) routed straight
to master; now `play_at_on_bus` routes a positional one-shot through a named bus, so the bus's
`set_bus_volume`/`duck_bus` scale the whole group on top of the sound's distance-based volume/pan.
A tiny additive composition of the existing positional + bus paths.

### Added
- `WebAudio::play_at_on_bus(bytes, source, listener, max_dist, bus) -> Sfx` — positional playback
  (distance falloff + x-offset pan) routed through a named mixer bus. The returned `Sfx`'s per-source
  volume/pan carry the spatial result, independent of the (downstream) bus level.
- `web_audio` example + `scripts/wasm_audio_smoke.sh` extended (headless lifecycle check now 38/38).

## 0.35.0

**2D positional audio for the wasm `WebAudio` path.** SFX could be panned/volumed by hand, but
positioning a sound from world coordinates was native-only (`AudioManager::play_at`). `WebAudio` now
computes volume + pan from 2D positions, reusing the existing per-source gain + stereo panner on the
[`Sfx`] handle. **Additive** — built entirely on the existing `Sfx` controls.

### Added
- `WebAudio::play_at(bytes, source, listener, max_dist) -> Sfx` — play a positional one-shot:
  volume falls off linearly (silent at `max_dist`), stereo pan follows the x-offset (native parity).
- `Sfx::update_position(source, listener, max_dist)` — reposition a playing sound each frame to
  track a moving source.
- `Sfx::volume()` / `Sfx::pan()` — read back the current per-source volume / pan.

### Done — "remaining native-only audio" backlog
- With ducking (0.34.0) and positional (this release), the wasm `WebAudio` mixer reaches native
  parity for the common cases. The only native-only audio feature left is **automatic sidechain**
  (`set_sidechain`), which needs continuous per-frame trigger-activity evaluation that doesn't fit
  the fire-and-forget Web Audio model — use manual `duck_bus`/`release_bus` instead.

## 0.34.0

**Bus ducking for the wasm `WebAudio` path.** Named buses could be volume-controlled but not ducked
— ducking was native-only (`AudioManager`). Each bus is now a two-gain chain `duck → volume →
master`: `set_bus_volume` drives `volume`, and the new `duck_bus`/`release_bus` ramp the `duck`
multiplier independently (so ducking never clobbers the bus volume and vice-versa), matching the
native mixer. Ramps run on the Web Audio clock (`AudioParam`), so — like the rest of the wasm audio
path — there is **no per-frame `update()` tick**. **Additive** — existing bus behavior is unchanged
(a bus rests at duck = 1.0, a transparent pass-through).

### Added
- `WebAudio::duck_bus(bus, gain, attack_secs)` / `release_bus(bus, release_secs)` — ramp a bus's
  duck multiplier toward `gain` (clamped `0.0..=1.0`) / back to `1.0`. `attack/release <= 0.0` is an
  instant set.
- `WebAudio::bus_duck(bus)` — the current duck multiplier (`1.0` if none / unknown bus).
- `web_audio` example + `scripts/wasm_audio_smoke.sh` extended to drive + self-check ducking
  (headless lifecycle check now 28/28).

### Changed (internal)
- Buses now store a `Bus { volume, duck }` two-`GainNode` chain (was a single `GainNode`); sounds
  routed to a bus connect to its `duck` input. `set_bus_volume`/`bus_volume`/`play_on_bus`/
  `play_sfx_on_bus` are unchanged in behavior. No public API change to those methods.

### Not ported (still native-only)
- **Automatic sidechain** (`set_sidechain`/`clear_sidechain`): it requires continuously evaluating
  "is the trigger bus playing?" every frame, which doesn't fit Web Audio's fire-and-forget model
  (and music isn't bus-routed on wasm). Drive ducking manually with `duck_bus`/`release_bus`.

## 0.33.0

**Track-to-track music crossfade for the wasm `WebAudio` path.** The music channel could be started
and stopped, but switching tracks meant a hard cut — crossfade was native-only (`AudioManager`).
`WebAudio::crossfade_music` now fades the current track out (then stops it) while the new track fades
in, so they overlap. Music now routes through a dedicated per-track `GainNode`, and the fades are
scheduled on the Web Audio clock (`AudioParam::linear_ramp_to_value_at_time`) — so, unlike the native
`Fade`/`update` infra, there is **no per-frame `update()` tick** and no temporary channel to tear
down. **Additive** — `play_music`/`stop_music` behave exactly as before (music just gains an internal
gain node); calling `crossfade_music` with nothing playing is simply a fade-in.

### Added
- `WebAudio::crossfade_music(bytes, dur)` — fade the music channel from the current track to a new
  one over `dur` seconds (no-current-track = fade-in).
- `web_audio` example + `scripts/wasm_audio_smoke.sh` extended to drive + self-check crossfade
  (headless lifecycle check now 22/22).

### Changed (internal)
- The music channel now stores a `MusicChannel { source, gain }` (was a bare source) so its volume
  can be ramped independently; `play_music`/`stop_music` updated accordingly. No public API change to
  those methods.

## 0.32.0

**Named mixer buses for the wasm `WebAudio` path.** The browser audio wrapper had a master volume
but no way to group sounds — bus mixing was native-only (`AudioManager`). `WebAudio` now has named
mixer buses: route sounds to a bus by name and control them together. A bus is just a named
`GainNode` wired `bus → master` (Web Audio is a node graph, so this needs no per-frame `update()`
tick, unlike the native fade infra). **Additive** — existing `WebAudio` calls route straight to
master exactly as before; this only adds the bus methods + `_on_bus` play variants.

### Added
- `WebAudio::set_bus_volume`/`bus_volume`/`bus_names` — per-bus volume (clamped `0.0..=1.0`) and the
  sorted list of known buses (native `AudioManager` mixer parity). Buses are created lazily on first
  reference; a volume-only bus (set without playing) persists in `bus_names`.
- `WebAudio::play_on_bus` / `play_sfx_on_bus` — fire-and-forget and controllable (`-> Sfx`) playback
  routed through a named bus (`source → [panner → per-source gain →] bus gain → master`).
- `web_audio` example + `scripts/wasm_audio_smoke.sh` extended to drive + self-check the bus surface
  (headless lifecycle check now 19/19).

## 0.31.0

**UI keyboard focus navigation.** UI widgets could only be operated with the mouse (and a clicked
`TextInput` typed). Now `UiSystem` has a focus pass: **Tab / Shift+Tab** cycle keyboard focus across
focusable widgets (`Button`, `TextInput`, `Slider`, `CheckBox`), a focus ring is drawn around the
focused widget, **Enter / Space** activate it (button click / checkbox toggle), **Left / Right**
nudge a focused `Slider`, and clicking a widget focuses it. **Additive** (one new resource +
auto-registered; existing UI behavior unchanged).

### Added
- `UiFocus` resource (`Option<Entity>`, auto-inserted) — the currently keyboard-focused widget;
  read it to style/inspect focus.
- `UiSystem` focus pass: Tab/Shift+Tab cycling (by entity index, skipping hidden/disabled widgets),
  focus ring, Enter/Space activation, Left/Right slider nudge, click-to-focus, and `TextInput`
  focus sync (a Tab-focused field receives typed characters).
- Example `ui_focus`.
- `UiEvent` now derives `PartialEq`.

## 0.30.0

**Autotile API unification + dead-code removal.** The two separate autotile component types are now
one. `TilemapAutotile` gains a `mode: AutotileMode` — `Single { mask_to_tile }` (any non-zero cell
connects, built with `edge_16`/`blob_47`) or `Multi { rules }` (per-terrain same-value, built with
the new `multi_edge_16`). This mirrors the dispatch `TilemapSystem` already did internally. The
ghost `ConnectRule` (a do-nothing marker struct + field) is removed. **Breaking** (0.x MINOR):
`MultiTerrainAutotile` and `ConnectRule` are gone, and `TilemapAutotile`'s `mask_to_tile` field
moved into `mode`. Single-terrain users calling `TilemapAutotile::edge_16(..)` are unaffected.

### Changed
- **`MultiTerrainAutotile::edge_16(&terrains)` → `TilemapAutotile::multi_edge_16(&terrains)`** —
  multi-terrain autotiling is now a `TilemapAutotile` in `AutotileMode::Multi`.
- `TilemapAutotile { neighborhood, oob_filled, mask_to_tile, connect }` →
  `TilemapAutotile { neighborhood, oob_filled, mode }` (the bitmask map lives in
  `AutotileMode::Single`).
- `TilemapSystem` reads the single `TilemapAutotile` (matches on `mode`) instead of two components.

### Removed
- `MultiTerrainAutotile` (folded into `TilemapAutotile` + `AutotileMode::Multi`).
- `ConnectRule` (unused marker / extension-point stub that never did anything).

### Added
- `AutotileMode` enum + `TilemapAutotile::multi_edge_16`.

## 0.29.0

**Hexagonal tilemaps.** Completes the projection set (C2 after C1's isometric): `TilemapProjection`
gains `Hexagonal` — a pointy-top hex grid in odd-r offset coordinates (odd rows shifted right by
half a tile), so the rectangular `tiles[row][col]` array maps straight onto it. `cell_center_world`
lays out the hex grid; `cell_at_world` picks via pixel → axial → cube-round (exact at hex borders);
hexes tessellate without overlap so they keep a fixed `z`. **Additive** — the new enum variant is
the only change to existing types.

### Added
- `TilemapProjection::Hexagonal` (pointy-top, odd-r offset).
- `Tilemap::cell_render_size()` — the sprite size `TilemapSystem` draws a tile at: square for
  orthographic/isometric, taller (`tile_size × tile_size · 2/√3`) for hexagons.
- `cell_center_world` / `cell_at_world` / `cell_z` now handle the hexagonal projection.
- Example `hex_tilemap` — a pointy-top hex grid with keyboard cell selection (reactive `set_tile`)
  and mouse hover-picking (`cell_at_world`); includes a generated `hex_tiles.png` atlas.

### Changed
- `TilemapSystem` sizes tiles via `cell_render_size()` (was a hardcoded square `tile_size`) so hex
  tiles get their taller sprite. No change for orthographic/isometric maps.

## 0.28.0

**Isometric tilemaps.** `Tilemap` could only lay out a square grid; now it has a
`TilemapProjection` — `Orthographic` (default, unchanged) or `Isometric` (a 2:1 diamond grid).
`cell_center_world`/`cell_at_world` branch on the projection (isometric picking inverts the diamond
transform and rounds to the nearest cell), and `TilemapSystem` depth-sorts isometric cells
back-to-front. **Additive — existing tilemaps are byte-identical** (projection defaults to
`Orthographic`). Hex grids are the next step (C2).

### Added
- `TilemapProjection` enum (`Orthographic` default / `Isometric`) + `Tilemap::with_projection`.
- `Tilemap::cell_z(row, col)` — the render `z` `TilemapSystem` assigns a cell (`-1.0` orthographic;
  `row + col` painter's-order depth for isometric).
- `cell_center_world` / `cell_at_world` now honor the projection (isometric diamond math).
- Example `iso_tilemap` — a diamond grid with keyboard cell selection (reactive `set_tile`) and
  mouse hover-picking (`cell_at_world`); includes a generated `iso_tiles.png` diamond atlas.

### Changed
- `TilemapSystem` places tiles via `cell_center_world` + `cell_z` (one call site handles both
  projections) instead of an inlined orthographic formula. No change for orthographic maps.

## 0.27.0

**wasm AEAD save/load browser verification.** v0.22.0 made the `save`/`load`/`save_versioned`/
`load_migrated` family cross-platform (hex-encoded ChaCha20-Poly1305 blob in `localStorage` on
wasm), but that path was only compile-gated + native-playtested — never *run* in a browser. This
closes that verification debt with an autonomous headless check. **No engine code change — example
+ tooling only.**

### Added
- Example `wasm_save` (wasm-only, `examples/wasm_save/`) — exercises the localStorage save backend
  end-to-end: `save` → `exists` → `load` round-trip, asserts the stored value is hex ciphertext
  (not plaintext), verifies **AEAD tamper detection** (a corrupted blob makes `load` fail), and a
  `save_versioned`/`load_migrated` round-trip + `delete`.
- `scripts/wasm_save_smoke.sh` — optional local (non-CI) headless check that runs the example and
  asserts the round-trip via the verdict read live over Chrome's DevTools endpoint. Result: **7/7**.

## 0.26.0

**WebAudio: controllable per-source SFX with stereo pan (wasm).** A step toward native↔wasm audio
parity: `WebAudio` could only fire-and-forget sound effects (`play`) — you couldn't pan, set a
per-sound volume, or stop one. New `play_sfx` returns an `Sfx` handle that does all three. Routes
`source → StereoPannerNode → per-source GainNode → master`; the panner + gain are created
synchronously, so `set_pan`/`set_volume` apply even before the clip finishes decoding. **Additive —
`play` and the rest of `WebAudio` are unchanged.** Crossfade, buses, ducking and full positional
audio remain native-only.

### Added
- `WebAudio::play_sfx(bytes) -> Sfx` — a controllable one-shot SFX.
- `Sfx` handle (re-exported at the crate root): `set_volume` (per-source, 0..1), `set_pan`
  (-1 left .. 1 right), `is_playing`, `stop`. Cloning a handle controls the same sound; if the
  per-source nodes can't be created it falls back to routing straight to master (volume/pan no-op).
- web-sys feature `StereoPannerNode`.
- The `web_audio` example + `scripts/wasm_audio_smoke.sh` now also exercise `play_sfx`
  (pan/volume/stop) — the headless lifecycle check is **12/12**.

### Fixed
- `examples/web_audio/web/build.sh` + `scripts/wasm_audio_smoke.sh` are now marked executable in
  git (were 0644), and the smoke script invokes `build.sh` via `bash` so it works regardless.

## 0.25.0

**Dialogue portrait rendering.** `DialogueBox::portrait` has existed (set via `with_portrait`) but
`DialogueSystem` only ever drew text, so the portrait was stored and never shown — this completes
the half-built feature. The system now draws the speaker's portrait to the left through the UI
image queue and shifts the text right to clear it; a box with no portrait renders exactly as before
(original left margin). **Additive — no breaking change, no new public API** (the `portrait` field
and `with_portrait` builder already existed).

### Added
- `DialogueSystem` renders `DialogueBox::portrait` (96×96 screen-space image, left of the text;
  text auto-shifts right when a portrait is present).
- Example `dialogue_portrait` — a multi-speaker conversation whose portrait switches per speaker,
  with a final portrait-less line showing the text-only fallback. Includes two generated portrait
  assets (`examples/assets/portrait_{sage,knight}.png`).

### Changed
- Internal: `DialogueSystem`'s per-frame gather uses a small `DrawItem` struct instead of a tuple
  (keeps `clippy::type_complexity` happy with the added portrait field). No behavior change.

## 0.24.0

**WebAudio runtime verification + first example.** Closes the v0.23.0 verification debt: the
wasm `WebAudio` mixer shipped compile-checked but had never *run* in a browser and had no
example. This adds a playable `web_audio` example that drives the whole surface and a headless
smoke harness that asserts the audio-graph lifecycle at runtime (all 9 lifecycle checks pass in
headless Chrome). Acoustic output stays a human step — there is no audio capture in the flow.
**Additive — no breaking change.**

### Added
- `WebAudio::is_running` — whether the `AudioContext` is unlocked and not suspended (for a "tap
  to enable sound" prompt or paused-audio indicator).
- `WebAudio::is_music_playing` — whether the music channel is occupied (for a music on/off UI),
  which also makes the async `play_music` decode observable.
- Example `web_audio` (wasm-only, `examples/web_audio/`) — generates an in-memory sine WAV and
  exercises `new` / volume set+clamp / `resume` / `play_music` (looping) / `suspend` / `resume`,
  reporting a pass/fail line per step plus a verdict in the page title.
- `scripts/wasm_audio_smoke.sh` — optional local (non-CI) headless check that runs the example
  and asserts the lifecycle via the verdict read live over Chrome's DevTools endpoint. Runs in
  real time (not `--virtual-time-budget`: the audio thread's suspend/resume transitions race a
  virtual clock).
- web-sys feature `AudioContextState`.

## 0.23.0

**WebAudio depth (wasm).** The browser audio player grows from fire-and-forget SFX into a
small but usable mixer: a master volume, a looping music channel, and pause/resume.

### Added
- `WebAudio::set_volume` / `volume` — a master `GainNode` that all playback routes through.
- `WebAudio::play_music` / `stop_music` — a single looping music channel (stops any current
  music, starts the new clip looping; `stop_music` stops it).
- `WebAudio::suspend` / `resume` — pause and resume all audio (also the call to satisfy the
  browser's user-gesture gate).
- web-sys features `GainNode`, `AudioParam`.

### Notes
- Native `AudioManager` (rodio) is unchanged. Per-source mixing, crossfade, buses, ducking and
  positional audio remain native-only.
- Compiles + wasm-clippy clean; runtime audio is verified in a browser (no autonomous audio
  capture), consistent with how the v0.17.0 WebAudio one-shot was checked.

## 0.22.0

**wasm AEAD save/load parity.** The encrypted player-save path (`save` / `load` /
`save_versioned` / `load_migrated`) now works on wasm, not just native — closing the gap where
those returned `SaveError::Unsupported` in the browser.

### Changed
- The ChaCha20-Poly1305 AEAD core (magic / nonce / cipher / encrypt / decrypt / versioned
  envelope / migration) is now **cross-platform**; the only per-target difference is the storage
  backend: a file on native, a **hex-encoded** blob in `localStorage` (keyed by the path string)
  on wasm. So `save` / `load` / `load_or_default` / `save_versioned` / `load_migrated` all work on
  both targets.
- Nonce generation switched `rand::thread_rng()` → `rand::rngs::OsRng` (works on wasm via
  `getrandom`'s `js` backend; `thread_rng` is not wired up there).
- `SaveError::Unsupported` now means "storage unavailable / future save version" rather than
  "no filesystem"; its `Display` text updated to match.

### Notes
- `localStorage` is user-inspectable, so the binary-embedded key gives tamper-detection +
  obfuscation, **not** secrecy against a determined user — the same trust model as the native
  save file (documented on `save_with_key`).

### Added
- Example `save_encrypted` — an encrypted launch counter persisted via `save` / `load` (a file
  natively, hex in `localStorage` on the web). Playtested windowed (count persists across runs;
  the saved file is `R2DAEAD01` magic + ciphertext, not plaintext).

## 0.21.0

**Particle RON→GPU builder.** `ParticleConfigSet::gpu_emitter(name)` builds a
`GpuParticleEmitter` from a RON particle config — the GPU-compute counterpart to `emitter()`
(the CPU path) — so a single `.ron` file can drive either path. Closes the 0.18.0 gap where
RON `gravity` / `emit_shape` reached only CPU emitters.

### Added
- `ParticleConfigSet::gpu_emitter(name) -> Option<GpuParticleEmitter>` (native-only, like
  `GpuParticleEmitter` itself — wasm has no GPU compute path; use `emitter()` there). The nine
  fields shared with the CPU emitter map 1:1 (`spawn_rate` / `lifetime` / `velocity` /
  `velocity_spread` / `color_start` / `color_end` / `gravity` / `emit_shape` / `emit`); the
  square GPU `size` takes the config width (`size.0`); `texture` / `z` have no GPU-emitter
  equivalent and are ignored.

### Changed
- Example `gpu_particles` now loads its emitter from `examples/gpu_particles.ron` via
  `App::load_particle_configs` + `gpu_emitter`, and auto-spawns one emitter at center so the
  RON→GPU path is visible on launch (more spawn on left-click). Playtested windowed.

## 0.20.0

**Data-driven dialogue: RON dialogue trees, conditional choices, and choice→event/effect
hooks.** Builds on 0.19.0's in-code branching `DialogueBox` to make conversations data-driven
and consequential — all purely additive (a box with no tree/cond/effect renders
byte-identically to 0.19.0; old scene RON still loads).

### Added
- **RON dialogue-tree loader** — `DialogueTree` (an ordered list of named nodes, each a line
  [literal or localization key] plus optional `goto`-by-id branching choices) flattens to an
  ordinary `DialogueBox` at spawn (node order = line index), so the existing `DialogueSystem`
  drives it unchanged. `DialogueRegistry` (World resource) + `App::load_dialogue(name, path)`
  load and hot-reload it, mirroring `load_animation_clips` / `load_particle_configs`. Parsing
  validates duplicate node ids, unknown `goto` targets, and literal/localized consistency.
- **Conditional choices** — `DialogueChoice` gains an optional `cond: DialogueCond` (compares a
  `DialogueVars` variable via `Eq/Ne/Gt/Lt/Ge/Le`); gated-out choices are hidden. `DialogueVars`
  is a new World resource of `DialogueValue` (`Bool/Int/Float/Str`) flags/counters.
- **Choice → event/effect hooks** — `DialogueChoice` gains an optional `effect: DialogueEffect`
  (`SetVar` writes a variable; `EmitEvent` sends a `DialogueEvent` to `Events<DialogueEvent>`).
  World-level `dialogue::advance(world, e)` / `dialogue::choose(world, e, i)` honor conditions
  and apply effects; `DialogueBox` gains `visible_choices` / `is_choosing` / `advance_with` /
  `choose_visible` for the vars-aware path (the original `advance` / `choose` stay for the
  simple case). Choice builders `when(cond)` / `then(effect)`. RON authors `cond` / `effect`
  inline via RON's IMPLICIT_SOME extension.
- Example `dialogue_quest` — loads a branching quest from `dialogue_quest.dlg.ron`, grants a
  lantern via an `EmitEvent` effect, gates a later "secret" choice on the granted variable, and
  flips EN↔KO live (the VISION acceptance test; playtested windowed).

### Changed
- `src/dialogue.rs` is now the `src/dialogue/` module (`mod.rs` + `tree.rs` + `vars.rs`).
- `DialogueChoice` no longer derives `Eq` (its new `cond` / `effect` can hold an `f32` via
  `DialogueValue::Float`); it still derives `PartialEq`.

### Deferred
- Per-line portraits (the dialogue renderer is text-only today) and a `DialogueBox`-level node
  `goto` (unconditional jumps use a single choice, as the examples do).

## 0.19.0

**Dialogue depth: `DialogueBox` gains localization keys and branching choices.** The Phase-4
`DialogueBox` was a linear typewriter over a `Vec<String>`; this makes it production-grade for
RPG/visual-novel use on two fronts — (1) lines/speaker/choices can be driven by translation keys
resolved against the existing `LocaleResource` each frame (so a live `set_locale` retranslates a
conversation mid-flow without losing the reader's place), and (2) a line can present numbered
choices that jump the conversation to another line. Purely additive — a box with no `line_keys`
and no `choices` renders byte-identically to before; both new field groups are `#[serde(default)]`
so pre-localization scene RON still loads. `DialogueSystem` stays input-agnostic (the game calls
`advance`/`choose`).

### Added
- Localization: `DialogueBox::localized(speaker_key, line_keys)` constructor + `line_keys: Vec<String>`
  / `speaker_key: Option<String>` fields + `resolve(&LocaleResource)`, which fills `lines`/`speaker`
  from translation keys **without** touching `current`/`elapsed`/reveal state (safe to call every
  frame). `DialogueSystem` resolves every box against the current locale before ticking. `src/dialogue.rs`.
- Branching: new `DialogueChoice { text, key: Option<String>, goto }` type (`new`/`localized` ctors,
  re-exported from the crate root) + `choices: Vec<(usize, Vec<DialogueChoice>)>` field + `with_choices`
  builder + `pending_choices()` / `choose(i)`. Selecting choice `i` jumps to its `goto` line (out-of-range
  `goto` clamps to the end, finishing the conversation); `advance()` is a no-op while a decision is
  pending so a plain advance can't skip a choice. Localized choice labels resolve like line keys.
  `DialogueSystem` draws the numbered choice list in place of the ▼ advance hint. `src/dialogue.rs`.
- `examples/dialogue_branching.rs` — a localized, branching merchant scene: SPACE advances, `1`/`2`
  pick choices, `L` toggles the locale between English and Korean live, `R` replays. The buy/dark
  branches show distinct lines and reconverge on a shared farewell. (12 new unit tests; the existing
  linear `dialogue_demo` is unchanged.)

## 0.18.0

**Particle depth, completed: `gravity` + `emit_shape` now reach the GPU emitter and RON configs too.**
Phase 6 (v0.16.0) added `gravity` and `emit_shape` to the CPU `ParticleEmitter` only, leaving the
compute-shader `GpuParticleEmitter` and the data-driven `ParticleConfigSet` (RON) without them. This
closes that follow-up on both fronts. Purely additive — zero gravity / `Point` shape / omitted RON
fields reproduce the prior behavior byte-for-byte; the native `AudioManager` and CPU particle paths are
untouched.

### Added
- `GpuParticleEmitter::gravity: Vec2` + `emit_shape: EmitShape` (with `with_gravity`/`with_emit_shape`
  builders), mirroring the CPU `ParticleEmitter`. Gravity is carried per-particle and integrated in the
  compute shader (`p.vel += p.gravity * dt`); the emit shape is sampled on the CPU at emission time via
  the existing `EmitShape::sample_offset`. `src/gpu_particle.rs`.
- RON `ParticleConfigSet` (`EmitterDef`) gains optional `gravity` and `emit_shape` fields via a private
  `EmitShapeDef` serde mirror (`Point` / `Circle(radius:)` / `Ring(radius:)` / `Box(half_extents:)`);
  both default to zero / `Point`. `src/particle/config_set.rs` (+ 4 unit tests).
- `examples/gpu_particles.rs` spawns with a gravity arc + `Circle` emit shape;
  `examples/games/data_particles/assets/particles.ron` exercises both new RON fields (hot-reloadable).

### Changed
- `GpuParticle` GPU struct grew 64 → 80 bytes (added `gravity: vec2<f32>` at offset 64 + padding to a
  16-byte-aligned stride); the compute and render WGSL `Particle` structs match. Native-only; the
  per-particle buffer is 4096 slots (≈320 KB). No public API removed.

## 0.17.0

**WASM audio (one-shot SFX) — Phase 7 of the user-experience roadmap, the final phase.** The native
`AudioManager` is rodio-based and native-only, leaving browser builds silent. `WebAudio` adds a minimal
Web Audio one-shot SFX path so wasm games can play sound effects.

### Added
- **`WebAudio`** (`src/audio_wasm.rs`, wasm-only) — a tiny `AudioContext` wrapper. `WebAudio::new()`
  creates the context; `play(bytes)` decodes an encoded clip (WAV/MP3/OGG, whatever the browser supports)
  and plays it once (fire-and-forget via `spawn_local`). Store it as a `World` resource. Re-exported as
  `engine::WebAudio` on `wasm32`. Added the Web Audio `web-sys` features.

### Scope / notes
- Intentionally minimal — one-shot SFX only. Music, mixing, fades, buses, ducking, and positional audio
  remain in the native `AudioManager` (rodio); a full cross-platform unification (e.g. via `kira`) is a
  separate future effort.
- Browsers gate audio behind a user gesture — trigger the first `play` from an input handler, or the
  context may stay suspended.
- ⚠️ The actual sound output was **not** verified this session (the dev machine was locked — no browser,
  no way to hear it — and audio has no meaningful unit test). The wasm code **compiles + lints clean** (CI
  Build (WASM) + wasm clippy) against the standard Web Audio API; **hearing it in a browser is the
  outstanding verification step.**

## 0.16.0

**Particle depth — Phase 6 of the user-experience roadmap.** `ParticleEmitter` gains the two knobs 2D
effects most need — per-particle gravity and a spawn shape — so fire, fountains, and scatter showers
are configurable instead of all-uniform. Additive: the defaults (`gravity = ZERO`, `emit_shape = Point`)
reproduce the prior behavior exactly.

### Added
- `ParticleEmitter::gravity: Vec2` — constant per-particle acceleration integrated each frame (`ZERO` =
  none, e.g. `(0, 300)` falling / `(0, -60)` rising). Builder `with_gravity`.
- `ParticleEmitter::emit_shape: EmitShape` — `Point` (default) / `Circle { radius }` / `Ring { radius }` /
  `Box { half_extents }`; new particles spawn at an offset sampled from the shape. Builder
  `with_emit_shape`. `EmitShape` re-exported as `engine::EmitShape`.
- Example `examples/particles_showcase.rs` — a fountain (gravity down), buoyant fire (gravity up + circle
  base), and box-scattered sparks.

### Notes
- The `GpuParticleEmitter` (native GPU path) does not yet mirror these fields — follow-up.
- Live visual playtest deferred (the dev machine was locked); the new math is unit-tested (gravity
  integration, zero-gravity = constant velocity, emit-shape sample bounds), and existing particle
  behavior is byte-identical with default values.

## 0.15.0

**WASM persistence — Phase 5 of the user-experience roadmap.** Browser-deployed games can finally save
data: the plain-text RON save functions route to `localStorage` on wasm instead of returning
`Unsupported`, using the same API as the native filesystem path.

### Changed
- `save::write_ron` / `read_ron` / `exists` / `delete` now work on `wasm32` via `localStorage` (keyed by
  the save-path string) — previously `Unsupported` / `false` / no-op. A small internal `wasm_storage`
  wrapper (web-sys `Storage`, newly enabled) backs them. `read_ron` returns a `NotFound` `Io` error for an
  absent key, so `unwrap_or` / default patterns behave the same as native.

### Added
- Example `examples/save_counter.rs` — a launch counter persisted with `write_ron`/`read_ron`; the
  identical code uses a file natively and `localStorage` on the web.

### Notes
- Player-save AEAD (`save` / `load` / `save_versioned`) stays `Unsupported` on wasm: a hardcoded key in a
  browser-inspectable store adds little, and binary ciphertext would need base64 in a string store. Use
  `write_ron` / `read_ron` for browser persistence.
- The live browser `localStorage` round-trip was not run this session (the dev machine was locked); the
  wasm code compiles + lints clean (CI Build (WASM) + wasm clippy) and the native path is unchanged.

## 0.14.0

**Dialogue box primitive — Phase 4 of the user-experience roadmap.** The most re-invented narrative
boilerplate (a speaker + typewriter text box) is now a first-class component, so RPG / visual-novel /
narrative forks no longer hand-roll it.

### Added
- **`DialogueBox`** component (`src/dialogue.rs`) — `speaker` + a list of `lines`, each revealed with a
  per-character typewriter at `chars_per_sec` (`<= 0` = instant). Two-stage `advance()`: the first press
  completes the current line's reveal, the next moves to the following line (then finishes after the
  last). `is_finished`, `reset`, optional `portrait` handle. UTF-8-safe reveal. Re-exported as
  `engine::DialogueBox`.
- **`DialogueSystem`** — ticks every box's typewriter (via the `query_mut` added in 0.13.0) and renders
  the active box (speaker + revealed text + an advance hint) as screen-space text near the bottom of the
  viewport. Input-agnostic: the game calls `DialogueBox::advance` (e.g. on Space). Re-exported.
- Example **`examples/dialogue_demo.rs`** — a multi-speaker conversation with the typewriter + advance.

### Notes
- Rendering is text-only (no background panel) so it composes with whatever box art the game draws; the
  `portrait` field is data for the game to render. For localization, resolve keys via `LocaleResource`
  into the box's lines (store resolved strings).

## 0.13.0

**Core API ergonomics — Phase 3 of the user-experience roadmap.** Removes the most common ECS
papercut (the "collect the entities, then `get_mut` each" workaround for mutating several
components together) and the scene-stack asymmetry. The flagship WASM demo is refactored onto the
new API, so the first code a newcomer reads no longer teaches the workaround.

### Added
- **`World::query2_mut<A, B>`** and **`World::query3_mut<A, B, C>`** — mutable multi-component
  queries yielding `(Entity, &mut A, &mut B[, &mut C])`. They borrow the distinct archetype columns
  simultaneously via `HashMap::get_disjoint_mut`, so a system updates several components in one pass
  with no allocate-every-frame collect step. `A`/`B`/`C` must be distinct types.
- **`App::push_scene`** / **`App::pop_scene`** — App-level convenience for `SceneCmd::Push`/`Pop`
  (stack a pause menu or overlay over the running scene and resume it), mirroring `App::set_scene`.

### Changed
- `run_demo` (`src/lib.rs`, the WASM demo) now uses `query2_mut::<Transform, BounceVel>` instead of
  collect-then-`get_mut`.
- `FORKING.md` + the `CLAUDE.md` module map document the mutable queries and the scene-stack helpers.

## 0.12.0

**Game-feel core ("juice") — Phase 2 of the user-experience roadmap.** Adds the highest-leverage
"feel" primitives — global time-scaling (hit-stop/slow-mo) and value/easing tweening — and a
`juice_demo` example that also gives the previously-undemonstrated `FadeTransition`, camera shake,
and `PostProcessConfig` their first playable example.

### Added
- **`TimeScale`** resource + **`App::set_time_scale`** / **`App::time_scale`** — a global multiplier
  applied to the `dt` that gameplay (scene) systems receive (`1.0` normal, `0.0` hit-stop, `0.5`
  slow-mo, `2.0` fast-forward). Built-in tail systems (hierarchy/gizmo) and engine post-frame work
  (fades, hot-reload, asset upload, camera) keep real time, so the editor and transitions stay
  responsive at any scale.
- **`RealDt`** resource — the real (unscaled) per-frame delta, written every frame before `TimeScale`
  is applied. Lets a system opt out of time-scaling (e.g. a hit-stop controller that sets
  `TimeScale(0.0)` still needs real time to end its own freeze).
- **`Tween<T: Lerp>`** is now generic over the value type (defaults to `f32`). `Tween<Vec2>`,
  `Tween<Color>`, etc. interpolate in one tween instead of juggling separate `f32` tweens. Existing
  `Tween::new(0.0, 100.0, 1.0)` call sites and `TweenSequence` (still `f32`) are unchanged.
- Four easing curves: **`EaseInBounce`**, **`EaseOutBounce`**, **`EaseInElastic`**, **`EaseOutElastic`**.
  The editor's Timeline easing picker lists them too.
- Example **`examples/juice_demo.rs`** — hit-stop + camera shake + vignette pulse on impact, a
  `Tween<Vec2>` slide-in, and a row of sprites bobbing with the new easing curves. Doubles as the
  acceptance example for `FadeTransition`, `Camera::shake`, and `PostProcessConfig`.

### Changed
- **`Easing` is now `#[non_exhaustive]`** — adding future curves is no longer a breaking change.
  Downstream `match`es on `Easing` must add a `_` arm. (A one-time break, taken now while pre-1.0.)

**First-hour onboarding pass (docs + example, no library API change).** Lowers the barrier
for a new forker's first hour: a true minimal "image on screen" example, a fork-first README
that no longer lies about installation, and an English getting-started guide. No `src/` change,
so the public API and the 772-test suite are untouched.

### Added
- `examples/hello_sprite.rs` — the smallest textured-sprite example (load a PNG → render it),
  filling the ladder gap between `basic.rs` (solid-color, no asset) and the full example games.
  It demonstrates the asset workflow (`App::load_image` → `Sprite::textured_with_handle`) and
  is the recommended starting point to copy into your own `examples/my_game.rs`.
- `examples/assets/player.png` — a 32×32 placeholder sprite for `hello_sprite`.
- `FORKING.md` — English getting-started guide: the fork-first model, crate layout, how to
  start your own game, asset-path resolution, the borrow-split pattern, and the verify gate.

### Fixed
- `README.md` — replaced the false `skeleton-engine = "2.0.0"` crates.io install block (the
  crate is unpublished and at 0.x) with a fork-first **Getting Started** section; corrected the
  stale MSRV (`1.88` → `1.95`); de-versioned the obsolete "v2.0 notes" framing; noted that
  `REFERENCE.html` / `ARCHITECTURE.html` are written in Korean and linked `FORKING.md`.
- `CLAUDE.md` — the module-map row for `#[derive(Reflect)]` said "(`derive` feature, default
  on)", but that feature was removed when `engine_reflect_derive` became a path dev-dependency;
  corrected to describe the actual state.

## 0.11.0

**Version line reset: pre-1.0.** No code changes. The project moves from the 10.x SemVer
line back to a 0.x ("pre-1.0") line to honestly signal that the public API is not yet
stability-committed and may break between releases — matching the engine's actual state
(feature-rich but still evolving, single author, never published). The full prior history
(0.3.0 → 10.7.0) is preserved below and in git tags; only the go-forward line is
renumbered. 0.x cadence: MINOR = any release (incl. breaking), PATCH = point fix. 1.0.0
will mark a deliberate compatibility commitment later.

## 10.7.0

**Nine-slice (9-patch) scalable sprites (new feature + example).** Resizing a bordered/rounded
sprite (a UI panel, button, frame) by scaling the whole quad distorts its corners. `NineSlice`
makes the sprite renderer emit nine sub-quads instead of one — the four corners keep their fixed
size while the edges and center stretch to fill. Additive: the new branch only runs when a
`NineSlice` component is present, so ordinary sprites render byte-identically.

### Added

- `NineSlice` component — `border: [f32; 4]` (world-pixel border widths) + `uv_border: [f32; 4]`
  (matching source-texture UV fractions), both indexed `[left, right, top, bottom]`. Constructors
  `new` and `uniform(border_px, uv_frac)`. Re-exported as `engine::NineSlice`.
- The sprite pass computes the nine sub-quads (each with its own model matrix + UV sub-rect) for a
  `NineSlice` entity; corners stay fixed-size at any panel size, and the whole panel rotates rigidly.
  Does not apply to `AtlasSprite` or entities carrying a `ShaderMaterial`.
- Example `examples/nine_slice.rs` — one generated bordered texture drawn at many sizes (wide/tall/
  small/large), a rotating panel, and a naive-stretch comparison showing the corner distortion a
  9-slice avoids.

## 10.6.0

**Animated tiles (new feature + example).** A `Tilemap` was static per cell; common 2D needs
(water, lava, animated decals) want per-tile frame animation. `TileAnimationSet` maps a tile value
to a `TileAnimation` (frame list + frame time); matching cells cycle their atlas frame at runtime.
Decoupled from the reactive diff so non-animated maps pay nothing. Additive.

### Added

- `TileAnimation` (frame ids + `frame_time`, `frame_at(elapsed)`), `TileAnimationSet` (a component on
  the tilemap entity mapping a tile value → animation), `AnimatedTileCell` (per-tile-entity tag with
  precomputed frame UVs), `AnimatedTileSystem` (cycles tagged cells' `UvRect` each frame). Re-exported
  at the crate root.
- `TilemapSystem` tags animated cells at spawn and refreshes the tag when a cell's value changes; the
  per-frame cycling is render-only and does not bump the tilemap generation, so the unchanged-map
  fast path is fully preserved.
- Example `examples/animated_tiles.rs` — a procedurally generated atlas with cycling water/lava tiles
  beside static ground.

## 10.5.0

**Coroutine sequencer (new feature + example).** The engine had `Timer`, `Tween`, and
`Timeline` (data keyframes) but no imperative "do these actions with waits between them"
primitive. `Coroutine` adds scripted-gameplay sequencing — chain `wait` / `run` / `run_for`
steps that execute arbitrary closures against the `World`. Distinct from `Timeline`
(keyframe data) and `TweenSequence` (value interpolation). Purely additive.

### Added

- `Coroutine` — builder: `new` / `wait(secs)` / `run(|&mut World|)` / `run_for(dur, |&mut World, t|)`
  (progress `t` runs 0→1). `CoroutineRunner` — World resource (`start` / `active_count`).
  `CoroutineSystem` — ticks active coroutines each frame; it removes the runner resource,
  ticks (so closures get a free `&mut World`), then reinserts it (closures must not re-enter
  the runner). Leftover `dt` carries across steps within a frame. Re-exported at the crate root.
- Example `examples/coroutine_demo.rs` — a scripted scene: wait → spawn a box → slide it across
  via `run_for` → recolor → loop.

## 10.4.0

**Music-track crossfade (new feature + example).** `AudioManager` had per-channel fades
(`fade_out` / `play_fade_in` / `fade_volume`) but no single call to crossfade one track into
another with the two overlapping. `crossfade` adds it: the current track on a channel fades out
while the new track fades in, reusing the existing `Fade` + `update` infrastructure. Native-only,
purely additive.

### Added

- `AudioManager::crossfade(channel, new_path, repeat, dur)` — relocates the channel's current sink
  to an internal temp channel and schedules a stop-when-done fade-out there, then `play_fade_in`s
  the new track on the channel, so the two overlap. Degrades to a plain fade-in when nothing is
  playing. The temp sink is torn down by `update()` when its fade completes.
- Example `examples/music_crossfade.rs` — generates two short sine-wave WAVs in the temp dir and
  crossfades between them on a key press.

## 10.3.0

**`TweenSequence` — chained tweens (new feature + example).** `Tween` interpolates one value
over one duration; there was no primitive to chain multiple eased legs into a single animation.
`TweenSequence` plays a list of `Tween` segments back-to-back, each with its own easing,
optionally looping, carrying leftover `dt` across segment boundaries so a large frame step
doesn't stall on a segment edge. Purely additive.

### Added

- `TweenSequence` — builder (`new` / `then` / `push` / `looping`) + runtime (`tick` / `value` /
  `finished` / `fraction` / `reset` / `current_segment` / `segment_count`). Re-exported as
  `engine::TweenSequence`.
- Example `examples/tween_sequence.rs` — a square loops a rectangular path driven by two
  `TweenSequence`s (one per axis), each leg using a different easing.

## 10.2.1

**Partial split of `App::render()` (v10 item F — internal, risk-managed).** The ~890-line
`render()` god-function had its **separable concerns** extracted into named helper methods, while the
inherently-sequential scene-pass core (sprite → UI → particles → plugins → post → lighting → text →
fade, whose `render_view` aliases into `RenderState`) was deliberately **left inline** as one annotated
flow — splitting it would mean threading the encoder + target view through eight micro-functions, hurting
readability and adding GPU-silent-regression surface for no benefit. `render()` drops from ~890 to ~610
lines. No public API change.

### Changed (internal)

- Extracted from `App::render()` into private helpers on `impl App`: `setup_post_renderer` /
  `setup_lighting` (pre-frame renderer init/resize), `render_offscreen_targets` (the per-`OffscreenCamera`
  RT pass, each its own submission), `present_docked_placeholder` (the docked-editor RT warm-up frame),
  and `present_egui` (the final egui overlay pass). Behavior is byte-identical — same operation order,
  submit boundaries, and `cfg` gates.
- Verified by a **full render-mode visual playtest** (CI has no GPU test): normal + custom-shader
  pipeline (`shader_material`), offscreen RT (`security_camera`), lighting + post-process
  (`lit_dungeon`), docked editor (`basic` + F2), GPU particles (`gpu_particles`), and a fade-using
  scene (`timeline_cutscene`) all render correctly with no validation errors.

## 10.2.0

**Parallax scrolling (new feature + example).** A genre-agnostic 2D primitive the engine lacked —
background/foreground layers that scroll at a fraction of the camera's motion to fake depth.
Purely additive; pairs with the existing camera shake/follow.

### Added

- `ParallaxLayer` component — `factor: Vec2` per-axis scroll rate (`1.0` = world-locked, `0.0` =
  screen-locked, `0.0..1.0` = depth, `>1.0` = faster-than-world foreground). Constructors `new` /
  `horizontal(fx)` / `vertical(fy)`. The rest anchor is **lazily captured** from the entity's
  `Transform` on the first system run (plus the camera position at that moment), so you just place
  the sprite — no base bookkeeping.
- `ParallaxSystem` — offsets every `ParallaxLayer` entity's `Transform` each frame via
  `pos = base + (cam - cam_ref) * (1 - factor)`. Add with `app.add_system(ParallaxSystem)`. As a
  normal user system it reads the camera from the end of the previous frame (engine finalizes camera
  follow after the user loop), a sub-perceptual one-frame lag for backgrounds; add it after the
  systems that move the camera-followed entity.
- Both re-exported at the crate root (`engine::{ParallaxLayer, ParallaxSystem}`).
- Example `examples/parallax_scroll.rs` — a playable side-scroller: A/D moves a player, the camera
  follows (via an anchor entity, since `Camera::position` is the viewport top-left), and four layers
  (sky `0.10`, mountains `0.35`, trees `0.65`, foreground `1.10`) scroll at visibly different rates.

## 10.1.0

**`ShaderMaterial` example (VISION acceptance test, additive).** The custom per-entity
fragment-shader feature (`ShaderMaterial`, shipped earlier) had **no example** exercising it — a gap
against the VISION rule that a feature isn't done until a playable example does. This adds one, which
also serves as the only end-to-end validation of the `MaterialRenderer` custom-pipeline path (CI has
no GPU test). No library/API change.

### Added

- Example `examples/shader_material.rs` — three side-by-side sprites, each with a **distinct** custom
  WGSL fragment shader (hue-cycle, sin-wave plasma, noise dissolve), exercising the renderer's
  per-source-hash pipeline cache with multiple live pipelines at once. A system writes `params[0] =
  elapsed` into all three each frame (the per-frame `world.get_mut::<ShaderMaterial>` update path), and
  ↑/↓ drive the dissolve sprite's threshold (`params[1]`). Self-contained — uses `Sprite::colored`
  (white 1×1 fallback texture) so no asset files are needed, while still proving the `t_sprite` /
  `s_sprite` bindings. Validates the documented `ShaderMaterial` shader contract (subset `VertexOutput`,
  `@group(1)` texture + `@group(2)` params bindings) end-to-end via a visual playtest.

## 10.0.0

**v10 architecture pass** — a scoped set of breaking + internal refactors from the cohesion review
(`docs/MODULE_COHESION_REVIEW_2026-06-16.md`); plan + PR sequencing in
`plans/V10_BREAKING_PASS_PLAN_2026-06-16.md`. The one planned item **not** done — splitting the 839-line
`App::render()` — was intentionally descoped: its fork-friendliness goal is already met by the
`RenderPlugin` hook (added in 9.6.0), and it's the only refactor CI can't verify (no GPU test), so the
internal-readability gain didn't justify the render-path regression risk.

### Added

- `RenderTarget` escape-hatch read accessors `texture()` / `view()` / `sampler()` / `bind_group()`
  (the underlying wgpu objects are no longer `pub` fields; these mirror the physics `.raw()` hatches).
- `ScriptRegistry` — a World resource that owns Rhai script storage/loading + hot-reload (split out of
  `AssetServer`; see the scripting-decouple entry below).

### Changed (internal)

- Split the 1620-line `src/tilemap.rs` into `src/tilemap/{mod,autotile,system}.rs` (data model /
  autotiling / reactive render system), mirroring `src/physics/`. Pure relocation — the 10
  re-exported public names (`engine::tilemap::*`) are unchanged.
- Extracted the 14 renderer/texture/egui fields from the `App` god-struct into a new internal
  `RenderState` (`src/app/render_state.rs`); `App` now holds one `render: RenderState` field (`gpu`
  and `world` stay on `App`). No public API change — sets up the `update()` split.
- Split the 386-line `schedule::update()` god-function into `compute_viewport()` / `run_systems()` /
  `post_systems()` helpers, and moved egui frame begin/end into `egui_pass.rs`. Operation order is
  unchanged (guarded by the existing pause / egui-delta-merge / scene-transition tests). Internal-only.
- Split the 5-concern `SpriteRenderer` into `SpriteRenderer` (sprite batching + UI primitives) owning
  a `TextureCache` (texture/RT-bind-group cache + texture layout) and a `MaterialRenderer` (ShaderMaterial
  custom pipelines). All bind-group layouts / pipeline configs / draw order are byte-identical (verified
  by a visual smoke test of the `basic` + `security_camera` examples). Internal-only (new types `pub(crate)`).

### Changed (breaking)

- **`UiSystem` and `SteeringSystem` are no longer unit structs** — they now hold reused scratch
  buffers, so construct them with `UiSystem::default()` / `SteeringSystem::default()` (or `::new()`)
  instead of the bare `UiSystem` / `SteeringSystem` in `add_system(...)`. Eliminates 11 per-frame
  `Vec<Entity>` allocations (6 UI widget passes + 5 steering passes); behavior is identical. (Closes
  the deferred allocation finding #76.)
- _(Theme 3 encapsulation)_ `RenderTarget`'s wgpu fields `texture` / `view` / `sampler` /
  `bind_group` are now `pub(crate)` (use the new accessors above). `width` / `height` / `clear_color`
  stay `pub`.
- `RenderTarget::new` no longer takes a `texture_layout` argument — it builds its own bind-group
  layout internally. Forks create RTs via `App::create_render_target`, which is unchanged.
- `LightingRenderer`'s `normal_view` / `width` / `height` are now `pub(crate)` (native-only type,
  not in the prelude).
- **Scripting decoupled from the asset module.** `ScriptAsset` (and its Rhai `ast`) moved out of
  `src/asset.rs` into `src/scripting/`, and script storage/loading/hot-reload moved from `AssetServer`
  to the new `ScriptRegistry` resource — so `asset.rs` no longer references Rhai (a forker can swap
  scripting backends without touching the generic asset module). `engine::ScriptAsset` is still
  re-exported at the crate root (source-compatible), but `engine::asset::ScriptAsset` no longer exists
  and `ScriptAsset::ast` is now `pub(crate)`. `App::load_script` is unchanged.

### Removed (breaking)

- `engine::tilemap::cell_display_uv` — an unused public helper (zero callers in-tree; was redundant
  with `TilemapSystem`'s inline UV resolution).
- `SpriteRenderer::texture_layout()` — unused after `RenderTarget::new` became self-contained.

### Fixed

- `scripts/verify.sh` is now executable in git (`100755`); the documented `./scripts/verify.sh` gate
  command previously failed with "permission denied" on a fresh clone (it relied on a local-only
  execute bit).

## 9.6.1

**Cleanups (additive / docs / CI).** Small fork-friction + doc-quality fixes.

### Added

- `Wander::direction_fn: Option<fn(u32, Vec2) -> Vec2>` + `Wander::with_direction_fn(f)` builder —
  lets a game override the wander direction picker (e.g. plug in real `rand`) without forking
  `SteeringSystem`. Defaults to `None` (the existing deterministic built-in picker), so behavior is
  unchanged unless set. It's a plain `fn` pointer, so `Wander` stays `Clone`/`Debug`.

### Fixed

- Two broken rustdoc examples that never compiled (CI skipped doctests): `register_serde_component`
  used the stale `App::new(Default::default())` (now `App::new()`), and `register_editable_component`
  imported the `Reflect` *trait* instead of the derive macro (now `use engine_reflect_derive::Reflect`).
- CI + `scripts/verify.sh` now run `cargo test --doc` (which `--all-targets` skips) so fork-facing
  doc examples can't silently rot again.

## 9.6.0

**Pluggable render-pass hook (cohesion review item 7, additive).** A fork could not inject a
custom GPU pass (outlines, shadows, debug overlays, screen effects) without forking the engine's
`render()`. Now there's a registration hook. Fully additive — when no plugin is registered the
dispatch is skipped and the rendered output is byte-identical to before.

### Added

- `RenderPlugin` trait — `fn record(&mut self, ctx: &mut FrameContext, world: &World, viewport: (u32, u32))`.
  Implement it to record a custom render pass; runs once per frame after the main
  sprite/UI/particle passes and **before** post-processing/lighting, so downstream effects still
  apply to whatever the plugin draws. Read-only `&World` access. Native + wasm.
- `App::add_render_plugin(impl RenderPlugin + 'static) -> &mut Self` — registers a plugin; plugins
  run in registration order.
- `FrameContext` gains a `pub format: wgpu::TextureFormat` field (the target view's format) so a
  plugin can build its own `wgpu::RenderPipeline`. `FrameContext` and `RenderPlugin` are now
  re-exported from the crate root.
- Example `examples/render_plugin.rs` — an animated vignette plugin (lazy self-built pipeline via
  `ctx.format`, reads a `Pulse` ECS resource each frame, composites with `LoadOp::Load`).

## 9.5.1

**Internal cleanup (cohesion review, behavior-identical).** No public API or behavior change.

### Changed (internal)

- `AssetServer`'s three separate hot-reload watch-sets (`data_table_paths` /
  `animation_clip_paths` / `particle_config_paths`) are unified into one `watched_paths` set with
  a single `watch_path` method. The three public `watch_*` methods are **kept as delegates** (no
  break). Forwarding was already unified via `HotReloadable` in 9.3.0, so adding a new
  hot-reloadable registry no longer needs a new watch-set/method/branch (closes the OCP finding).
- `CameraUniform` (identical `#[repr(C)]` view-proj struct) was duplicated in `gpu_particle.rs`
  and `sprite/geometry.rs`; now defined once as `pub(crate)` in `renderer`.

## 9.5.0

**Pluggable editor inspector panels (cohesion review item 7, additive).** The docked editor's
inspector hardcoded its per-component sub-panels, so a fork couldn't add one for their own
component without editing `docked.rs`. Now there's a registration hook. Fully additive.

### Added

- `App::register_inspector_panel::<T>(title, draw)` (native-only) — registers a collapsing
  inspector sub-panel shown whenever the selected entity has component `T`. `draw` is
  `Fn(&mut egui::Ui, &mut App, Entity)`. Forks can add inspector UI for their own components
  without touching the engine.

### Changed (internal, behavior-identical)

- The four uniform built-in inspector panels (Particle Tuner / Point Light / State Machine /
  Timeline) are now registered through `register_inspector_panel` and dispatched by a single
  loop instead of hardcoded `if has_component` blocks (`docked.rs` −81 lines). Tile Paint stays
  hardcoded (non-uniform shape). No user-visible change.

## 9.4.1

**Module-home reorganization (cohesion review item 3, pure relocation).** Zero behavior change,
all public API paths preserved via re-export. Splits two god-files for fork-friendliness.

### Changed (internal — no API change)

- `SerdeComponentRegistry` + `SerdeComponentEntry` moved out of `prefab.rs` into a dedicated
  `serde_registry` module (re-exported from `prefab`, so `engine::SerdeComponentRegistry` is
  unchanged).
- The docked editor's **State Machine** and **Timeline** inspector panels were extracted from the
  2003-line `editor/ui/docked.rs` into `editor/ui/state_machine_panel.rs` +
  `editor/ui/timeline_panel.rs` (mirroring the existing `audio_panel.rs`/`data_table_panel.rs`).
  `docked.rs` 2003 → 1259 lines.
- The tile-paint input methods moved out of `editor/ui/gizmo.rs` into `editor/ui/tile_paint.rs`
  (tile painting is not a gizmo op). `gizmo.rs` 1869 → 1183 lines.
- *(`CameraUniform` dedup was skipped — the two definitions differ in field visibility.)*

## 9.4.0

**Module-cohesion review follow-ups (safe additive subset).** The first batch from
`docs/MODULE_COHESION_REVIEW_2026-06-16.md` — the non-breaking, behavior-identical items.
Breaking/architectural items (the `App`/`render()` extraction, encapsulation tightening,
`UiSystem`/`SteeringSystem` scratch fields) are deferred to a future `v10` design pass.

### Added

- `engine::AssetLoadError` — a shared RON/IO asset-load error. `ClipSetError` and
  `ParticleConfigError` are now type aliases of it (their public names + variant names are
  unchanged, so existing `match` arms keep compiling).
- `JointHandle::raw()` — escape hatch to the underlying rapier `ImpulseJointHandle`, matching
  the existing `BodyHandle::raw()` / `ColliderHandle::raw()` pattern (native-only).

### Changed (internal, behavior-identical)

- `SpriteRenderer` now reuses three scratch buffers (`atlas_entries`, `live_material_entities`,
  `seen_new_hashes`) across frames instead of allocating them per `render()` (closes the
  per-frame-allocation finding #72 from the hardening coverage ledger).
- `Tilemap::compute_tile_mask` / `compute_tile_mask_typed` now delegate to one shared
  `compute_mask_raw` (a closure-parameterized core), removing ~50 lines of copy-pasted Blob8
  logic — output is bit-identical.

### Docs

- `src/animation` gained a module-level "System registration order" doc (BlendTreeSystem →
  AnimationSystem → StateMachineSystem).
- `register_editable_component` now documents that its editor factory/remover halves are
  native-only (reflect+clone+serde still apply on wasm).

## 9.3.0

**`HotReloadable` trait — fork-friendly hot-reload extension point.** The hot-reload loop
forwarded changed asset paths to a hardcoded set of three registries via an internal macro;
adding a new hot-reloadable registry required editing engine internals. It's now a public
trait + registration, so forks register their own registries without touching the engine.
Fully additive (the three built-ins are auto-registered — behavior unchanged). Native-only,
matching the existing hot-reload code.

### Added

- `engine::HotReloadable` trait (`fn reload_path(&mut self, path: &str)`) — implement it on a
  resource to make it hot-reloadable.
- `App::register_hot_reloadable::<T: HotReloadable>()` — register a resource to receive every
  changed asset path each frame.
- `DataTableRegistry`, `AnimationClipRegistry`, and `ParticleConfigRegistry` implement
  `HotReloadable` and are auto-registered in `App::new` (replacing the internal
  `forward_reloads!` macro; same runtime behavior).

## 9.2.0

**Editor depth — State-Machine & Timeline panels gain real editing.** The docked editor's
SM and Timeline inspector panels were display-mostly; they now author content. Fully additive.

### Added

- `AnimationStateMachine::set_transition_conditions(from, index, conditions) -> bool` and
  `set_transition_crossfade(from, index, seconds) -> bool` — mutate an existing transition's
  conditions / crossfade (false on missing state or out-of-range index).
- **State-Machine panel:** live parameter editing (bool checkbox / float drag / trigger fire),
  add-transition (target ComboBox + crossfade), and per-transition condition add/remove — all
  routed through the tested edit ops.
- **Timeline panel:** per-track add-keyframe (empty tracks now render an add button) and
  per-track-type value editing (Vec2 = x/y drags, f32 = drag, Color = r/g/b/a drags), in
  addition to the existing time / easing / remove controls. `timeline_track_ui` now takes a
  `make_default` + `value_edit` closure instead of a read-only `fmt`.

### Notes

- The egui panel *layout* (condition-editor row width, value-drag rows) is functional but not
  visually tuned — a known cosmetic follow-up. All data mutations go through unit-tested ops.

## 9.1.0

**Editor-edit persistence** — `AnimationStateMachine` and `Timeline` now persist through
scene save/load. Both component families gained `serde::{Serialize, Deserialize}` derives and
are **auto-registered** in the `SerdeComponentRegistry` (alongside the UI widgets), so the
in-editor State-Machine and Timeline editors' edits survive a save/load round-trip with no
user action. Fully additive.

### Added

- `AnimationStateMachine`, `AnimParam`, `TransitionCond`, `AnimTransition`, `AnimState` now
  derive `Serialize, Deserialize` (and `PartialEq`).
- `Timeline`, `Track<T>`, `Keyframe<T>`, `CameraTarget` now derive `Serialize, Deserialize`.
  `Track<T>`/`Keyframe<T>` serialize for any `T: Serialize` (the concrete tracks are
  `Vec2`/`f32`/`Color`).
- `Easing` (`src/tween.rs`) now derives `Serialize, Deserialize` (required by `Keyframe`).
- `AnimationStateMachine`, `Timeline`, and `CameraTarget` are auto-registered for serde in
  `register_core_component_metadata`, so scene save/load captures them automatically.

### Notes

- `Timeline::time` and `Timeline::playing` are serialized (lossless save of the playback
  position); add a `post_spawn` hook if you want a reset-on-load policy.

## 9.0.0

Engine-wide **hardening pass** — 80 findings from a 14-subsystem code analysis
(`docs/CODE_ANALYSIS_2026-06-16.md`). Dominant theme: **fail-loud over fail-quiet** — bad
input/data/state that used to silently misbehave now panics-guards, logs, or returns `None`.
Mostly additive; a handful of small breaking changes are listed first.

### Changed (breaking)

- **`NetworkSystem` is now a struct** (holds warn-once state for a missing `Events<NetworkEvent>`).
  Construct it with `NetworkSystem::new()` (or `::default()`) instead of the bare `NetworkSystem`
  unit value — `app.add_system(NetworkSystem::new())`.
- **`GamepadButton` / `GamepadAxis` are `#[non_exhaustive]`** — external `match`es must add a
  wildcard (`_ =>`) arm. Future button/axis additions are now non-breaking (before v1.0 freeze).
- **`SerdeComponentEntry` gained a `has_component` field** — only `register_arc` constructs it
  internally, but external code building the struct literal directly must add the field.
- **`SolidTiles::Only` stores a `HashSet<u32>`** (was `Vec<u32>`) for O(1) per-tile lookup; an
  additive `IntoIterator` constructor preserves the ergonomic build path.
- **MSRV raised 1.92 → 1.95** to match the only toolchain CI actually verifies (the declared 1.92
  was never built/tested).

### Added

- `ParticleEmitter::z` + `with_z()` (+ RON `EmitterDef` `z`) — spawned particles inherit the
  emitter's z-depth instead of being hardcoded to `0.0`.
- `Track<T>::set_value(i, v)` / `set_easing(i, e)` keyframe mutators; the editor Timeline panel
  wires per-keyframe easing editing.
- `World::has_component::<T>()` public existence check (no wasteful downcast).
- `save_versioned_with_key` / `load_migrated_with_key` (custom-key + versioned migration).
- `InputMap::axis_value(action, gamepad, pad)`; gamepad **axis** bindings are now honored by
  `just_pressed` / `just_released` (previously axis-only actions always read `false`).
- `Camera` shake / zoom-tween accessors; `RenderTarget` per-target `clear_color`.
- `Panel::direction` exposed via `Reflect`; `LocalizationSystem` can bind `TextInput.placeholder`.
- Editor: `+ Add Component` / remove `✕` now cover all 8 serde UI widgets (UiNode/Button/Label/
  TextInput/Slider/CheckBox/ScrollView/Panel); `SerdeComponentRegistry::component_names_for`
  (presence check without RON serialization).
- Audio `clear_file_cache()`; hot-reload dispatch de-duplicated so a fork wires a new RON registry
  in one line.
- Four crate-boundary integration tests (`tests/{pathfinding,timeline,behavior,save}_smoke.rs`).

### Fixed

- **`blob_47` autotile mask table** used the wrong bit convention — 36 reachable masks silently fell
  back to atlas tile 0 (plain orthogonal-neighbor tiles rendered as tile 0).
  **Migration:** the mask→atlas-index mapping is now the canonical Blob-47 order. If you previously
  rearranged your atlas to match the old (mostly tile-0) indices, regenerate/reorder it to match.
- **Gamepad axis `just_pressed`/`just_released`** ignored axis bindings (stick-triggered one-shot
  actions never fired).
- **Held keys stuck on focus loss** — `WindowEvent::Focused(false)` now calls `InputState::release_all()`
  (no phantom `just_released`); spurious `just_released` for never-pressed keys fixed.
- **macOS modal resize double-stepped** physics/tween/timer (`Resized` + `RedrawRequested` both
  stepped in one iteration) — guarded to one step per event-loop iteration.
- **A panicking system no longer runs the frame's remaining systems** on a half-mutated World
  (frame aborts; the system is disabled for subsequent frames).
- `find_path` / `find_path_diagonal` return `None` (not `Some([blocked])`) for a blocked `start == goal`.
- Animation RON robustness: `columns == 0` div-by-zero panic, out-of-bounds frame index, `play(OOB)`
  freeze + immediate `AnimationEnd`, dead-transition (nonexistent target) warning, skeletal
  `is_finished()` at construction.
- `add_prismatic_joint` zero-axis NaN guard; `contact_pairs` ordered-pair symmetry.
- Audio fade interactions: `set_bus_volume`/`set_volume`/`update_position` mid-fade no longer snap/drop.
- **Offscreen render loop**: raw `*const TextureView` (dangle-on-realloc UB) replaced with owned
  `TextureView` handles — `unsafe` removed.
- TextInput focus respects z-order and ignores hidden widgets; inspector write-back keyed by `TypeId`.
- Many **fail-loud `log::warn!`s** where data silently vanished (DataTable extra columns, serde
  serialize failures, missing registries, dropped network events, glyphon errors).
- wasm: `web_sys::ErrorEvent` feature enabled so the network `on_error` diagnostic path compiles.

### Performance

- **Tilemap** no longer clones the full grid every frame — a `generation` counter dirty-guards the
  `TilemapSystem` (idle cost is one `u64` compare); removed-entity check is a `HashSet`.
- Per-frame scratch allocations reused as fields across `SpriteRenderer`, `PhysicsSystem`,
  `SteeringSystem`, `SpatialGrid::rebuild`, `AudioManager::update`, `query_added/changed`.
- Atlas sprite path is `Arc`-cloned (refcount bump) not string-copied; glyphon shaped-buffer cache for
  static text; bloom `texel_size` uniform; light cull measured from viewport center + frustum prefilter.
- Inspector component list no longer full-RON-serializes every component each frame; pathfinding
  overlay snapshots tiles instead of cloning whole `Tilemap`s.

### Build / CI

- `serde_json` moved to `[dev-dependencies]` (the lib never used it — examples/tests only).
- wasm CI job gains a clippy pass (`--target wasm32 --lib -D warnings`).

### Notes

- Iterations 1–4 of the batch; **603 → 698 lib tests (+95)** plus 33 new integration tests. Full
  Gate6 green (fmt, clippy `--all-targets -D warnings`, wasm lib+bins build, `test --all-targets`,
  doc `-D warnings`).
- `#9` hot-reload fork-friendliness shipped as a `macro_rules!` dedup (a full `HotReloadable` trait is
  a planned follow-up).

## 8.27.0

Editor timeline editor (MVP) + `Track` keyframe inspection/edit API. Additive.

### Added

- `Track<T>` keyframe accessors/edit ops — `keyframes()`, `len()`, `remove(index)`,
  `set_time(index, time)` (re-sorts), `clear()` — alongside the existing `add`/`sample`/`duration`.
- Editor **Timeline** inspector panel (entities with a `Timeline`): playback controls (duration, loop,
  play/pause, restart, time scrub) plus a per-track keyframe list (position / rotation / scale / color
  / alpha / zoom) showing each keyframe's editable time, value summary, and easing, with per-keyframe
  remove. Exercised by the `timeline_cutscene` example (F2 → select the animated entity).

### Notes

- List-based MVP; a visual track/keyframe timeline (horizontal time ruler, draggable keyframe dots) is
  a planned follow-up. Edit ops validated by unit tests through the real `Track` data model
  (autonomous visual validation is weak under the docked cursor-freeze).

## 8.26.0

Editor state-machine editor (MVP) + `AnimationStateMachine` inspection/edit API. Additive.

### Added

- `AnimationStateMachine` inspection accessors — `state_names()`, `state(name)`, `state_count()`,
  `param_names()`, `param(name)` — and edit operations — `set_current_state()`, `set_state_clip()`,
  `remove_state()` (prunes inbound transitions; refuses the active/last state), `remove_transition()`.
- Editor **State Machine** inspector panel (entities with an `AnimationStateMachine`): lists states
  (current highlighted) with their transitions (target + condition summary + crossfade) and parameters,
  and offers edits — set current, edit clip index, remove state/transition, add state. Exercised by the
  `sm_crossfade` example (F2 → select the animated entity).

### Notes

- This is the list-based MVP; a visual node-graph rendering (positioned nodes + drawn edges) is a
  planned follow-up. Edit operations are validated by unit tests through the real data model
  (autonomous visual validation is weak under the docked cursor-freeze).

## 8.25.0

RTL text support: multi-font loading + reading-direction alignment. Additive.

### Added

- `ExtraFonts(Vec<Vec<u8>>)` resource — additional font blobs loaded alongside `FontData` for
  multi-script coverage (e.g. a Latin UI font + an RTL-script font). cosmic-text falls back across all
  loaded fonts by script, so a single `DrawText` mixing Latin + Hebrew/Arabic shapes correctly.
- `TextAlign::Auto` (no explicit alignment — cosmic-text aligns each line by its resolved direction,
  so RTL text right-aligns automatically) and `TextAlign::End` (reading-direction end: right for LTR,
  left for RTL). Existing `Left`/`Center`/`Right` unchanged.
- Example `rtl_text`: renders mixed Latin + Hebrew (RTL) text using a bundled OFL Noto Sans Hebrew
  font as `ExtraFonts`, demonstrating multi-font fallback + RTL-aware alignment.

### Notes

- Bidirectional/RTL **shaping was already supported** (the text renderer uses `Shaping::Advanced`);
  this release adds the font-coverage + alignment pieces needed to actually use it.
- Bundled `assets/fonts/NotoSansHebrew-Regular.ttf` (+ `NotoSansHebrew-OFL.txt`, SIL Open Font License).

## 8.24.0

Tile collider sync (editor Tile Paint + runtime). Additive.

### Added

- `TilemapColliders` component + `SolidTiles` rule (`NonZero` | `Only(ids)`): opt-in config that keeps a
  tilemap's static physics colliders in sync when the tilemap mutates. Carries the `pixels_per_unit` +
  solid-tile rule the generic sync needs, plus the persistent `TileColliderIndex` for incremental
  resyncs.
- `sync_tilemap_entity_colliders(world, entity)` free fn + `App::sync_tilemap_colliders(entity)` wrapper
  — resync an entity's tile colliders against the `PhysicsWorld` resource (no-op without a `Tilemap` +
  `TilemapColliders` + `PhysicsWorld`).
- The editor's **Tile Paint** now resyncs colliders after each stroke (and on undo/redo) for tilemaps
  that opted in via `TilemapColliders`.

### Changed

- `dig_quest` example refactored onto `TilemapColliders` + `sync_tilemap_entity_colliders`, replacing its
  hand-rolled `TileColliderIndex` field + manual `with_resource_mut`/`sync_static_from_tilemap` dance
  (behavior unchanged). Demonstrates the new API in real play.

## 8.23.0

Editor lighting editor. Additive — editor-internal; no public engine API change.

### Added

- **Point Light** inspector section (shown for entities with a `PointLight`): drag editors for
  color (r/g/b) / radius / intensity / light_height, mutating the component so the lighting pass
  updates next frame, plus a **Reset to Default** button (`App::reset_point_light`). The entity's
  `Transform` position is the light position, so selecting an entity and adding a light places it.
- **Ambient Light** inspector section: edits the global `AmbientLight` resource (color + intensity),
  inserting a default one first if the game never set it (`App::ensure_ambient_light`).
- `PointLight` is now registered as an editor component (Add/Remove buttons + "+ Add" dropdown).

## 8.22.0

Editor particle live-tuner. Additive — editor-internal; no public engine API change.

### Added

- **Particle Tuner** inspector section (shown for entities with a `ParticleEmitter`): drag editors
  for `emit` / `spawn_rate` / `lifetime` / `velocity` / `velocity_spread` / `size` and r/g/b/a drags
  for `color_start` / `color_end`. Edits mutate the component in place, so they take effect live on
  the next spawn while the simulation runs. A **Reset to Default** button restores the default config
  while preserving the assigned texture (`App::reset_particle_emitter`).

## 8.21.0

Editor audio bus mixer panel + `AudioManager::bus_names()`. Additive.

### Added

- `AudioManager::bus_names()` — returns every known bus name (sorted, deduplicated) from the
  channel→bus assignments and the explicit bus-volume map. Lets a UI enumerate all buses.
- **Audio** tab in the editor's bottom panel: one volume slider per audio bus (driven by
  `bus_names()`); dragging applies the new value live via `set_bus_volume`. Shows a hint when no
  `AudioManager` resource is present or no buses are assigned. Native-only, editor-internal.

## 8.20.0

Editor pathfinding-grid overlay. Additive — editor-internal; no public engine API change.

### Added

- **Path** debug overlay (toolbar toggle) in the editor: for each `Tilemap` entity it builds a
  `PathGrid` (the standard "non-zero tile = blocked" convention via `PathGrid::from_tilemap`) and
  shades every cell — blocked cells filled red, walkable cells outlined green — via `DebugDraw`.
  A quick "what would pathfinding navigate here" view. The toggle persists with the other editor
  settings (`#[serde(default)]`, so existing config files still load).

## 8.19.0

Editor debug bounds/colliders overlay. Additive — editor-internal; no public engine API change.

### Added

- **Bounds** debug overlay (toolbar toggle) in the editor: draws every entity's `Transform` AABB and
  any collision `Collider` shape (Aabb → rectangle, Circle → circle) via `DebugDraw` — a quick "where is
  everything / what's collidable" view. The toggle persists with the other editor settings.

## 8.18.0

Editor settings persistence. Additive — editor-internal; no public engine API change.

### Added

- **Editor preferences persist across restarts.** Snap on/off + size, the grid-overlay toggle, and the
  Tile Paint tool + brush size are written to a RON config file when the docked editor closes (F2) and
  restored the first time it opens. A toolbar **💾 Set.** button saves on demand. `PaintTool` gains
  serde derives so it round-trips.

## 8.17.0

Prefab create/instancing in the editor. Additive — editor-internal; no public engine API change.

### Added

- **Prefab** section in the docked editor inspector: **Save Selected** writes the selected entity
  (tag/transform/sprite/parent + serde-registered components, via `entity_to_def`) to a prefab RON
  file; **Spawn** loads a prefab from the path and instances it (with a `PrefabInstance` marker, so the
  existing **Break Prefab** works), selecting the new entity. A path field + status line drive it.

## 8.16.0

Rotation gizmo. Additive — editor-internal; no public engine API change.

### Added

- **Rotation handle** on the world-sprite gizmo. A green handle above the selected entity's top
  edge; dragging it rotates the entity (`Transform.rotation`) to follow the cursor. Completes the
  gizmo (move + 8-handle resize + rotate). With the **Snap** toggle on, rotation snaps to 15°
  increments. Each rotation is one undoable `EditorCmd::RotateEntity` (Ctrl+Z reverts).

## 8.15.0

Editor grid overlay. Additive — editor-internal; no public engine API change.

### Added

- **Grid overlay** in the F2 docked viewport (toolbar **Grid** toggle). Draws world-aligned grid
  lines at the editor snap spacing as an egui overlay on top of the game image — it reads the `Camera`
  to map world↔screen and does not touch the camera or game systems. Lines are skipped when the cells
  would be denser than a few pixels (zoomed out). A live **cursor readout** shows the world `(x, y)`
  under the pointer, plus the hovered `(row, col)` when a `Tilemap` is selected.

## 8.14.0

Inspector quality-of-life. Additive — editor-internal; no public engine API change.

### Added

- **Component copy/paste** in the docked editor's inspector. Each serde-registered component on the
  selected entity gets a **⧉ copy** button; a **Paste {type}** button then applies the copied
  component to the selected entity (insert or overwrite). Useful for transferring a tuned component
  (stats, sprite, …) between entities. Like Add/Remove-component, paste is not pushed to undo history.
- **Entity-list search** — a search box filters the left entity list by label (case-insensitive
  substring); the ✕ clears it.

## 8.13.0

Tile Paint tools. Additive — editor-internal; no public engine API change.

### Added

- **Paint tools for the docked editor's Tile Paint:**
  - **Brush** (freehand) with a selectable **N×N size** (1 / 3 / 5) — paints a block per hovered cell.
  - **Rectangle** — press-drag-release fills the rectangle spanned by the two cells.
  - **Bucket** — a click flood-fills the 4-connected region of same-valued cells.
  - **Eyedropper** — Alt+click picks the hovered cell's value into the paint value (works with any tool).
  Right-click still erases (value 0); every gesture commits as one `PaintTiles` command, so a single
  Ctrl+Z reverts the whole area. Tool + brush size are chosen in the Tile Paint inspector section.

## 8.12.0

Tile Paint swatch palette. Additive — an editor UX upgrade; no public engine API change.

### Added

- **Image-swatch palette for Tile Paint.** The F2 docked editor's **Tile Paint** section now
  renders each paintable tile as a real thumbnail of the selected tilemap's atlas (clickable
  `egui::Button::image` swatches with per-tile UVs from `TilemapAtlas::uv_for`) instead of numbered
  buttons. Clicking a swatch sets the paint value; the current value is highlighted; the "Erase"
  button is kept. Falls back to numbered buttons on the first frame before the atlas texture is
  registered. The atlas texture is registered with egui (`register_native_texture`) before the UI
  pass and freed when paint mode exits or the selection changes, so there is no texture leak.
- **`SpriteRenderer::texture_view(path) -> Option<&wgpu::TextureView>`** — borrow a cached
  image/atlas texture view by asset path (used by the editor to hand an atlas to egui).

## 8.11.0

Editor tile-painting. Additive — a new in-editor authoring tool; no public engine API change.

### Added

- **In-editor tile painting.** In the F2 docked editor, selecting an entity that carries a
  `Tilemap` component now shows a **Tile Paint** section in the inspector. Toggle **Paint mode**
  and paint directly in the viewport: **left-click/drag** paints the selected tile value,
  **right-click/drag** erases (value `0`), number keys **1–9** pick the paint value (**0** = erase,
  clamped to the atlas tile count). Painting reuses `Tilemap::cell_at_world` + `set_tile`, so the
  reactive `TilemapSystem` reflects each change the next frame. While paint mode is on, the
  move/resize gizmo is suppressed so clicks never drag the tilemap.
- **Stroke-level undo.** Each press→release stroke is recorded as a single `PaintTiles` editor
  command; one **Ctrl+Z** reverts the entire stroke (redo re-applies it).
- **Example `tile_paint`** — a blank 20×15 tilemap with a runtime-generated 4-colour atlas; the
  acceptance test for painting in the docked editor.

> **Note:** editor painting is **visual-only** — it does not sync tile colliders. Keep physics in
> step yourself with `PhysicsWorld::sync_static_from_tilemap` if the painted map is collidable.
> The feature is native-only (the docked-editor gizmo path is native).

## 8.10.0

### Fixed

- **`DataTable` file hot-reload now works for relatively-loaded tables.** `DataTableRegistry::reload_path`
  compared the raw stored path against the **canonical** path `AssetServer::poll_reloads` reports
  (`asset_key` canonicalizes), so a table loaded via a relative path silently never hot-reloaded from
  disk. It now matches by canonicalized path (the same approach the data-driven animation registry
  uses). Surfaced while fixing the equivalent bug in the new particle-config registry (8.9.0).

## 8.9.0

Data-driven particle emitter configs. Additive.

### Added

- **`ParticleConfigSet`** — load named `ParticleEmitter` configs from a RON file
  (`(emitters: { "fire": (spawn_rate, lifetime, velocity, velocity_spread, color_start,
  color_end, size, …), … })`); `Vec2` as `(x, y)`, `Color` as `(r, g, b, a)`, missing fields use
  serde defaults. `from_ron_str`, `emitter(name) -> Option<ParticleEmitter>` (a fresh emitter),
  `names()` (deterministic, alphabetical).
- **`App::load_particle_configs(name, path)`** + **`ParticleConfigRegistry`** — registry resource
  (survives scene reset) with **hot-reload** via the `AssetServer` watcher, mirroring
  `load_data_table` / `load_animation_clips`. `ParticleConfigError` for parse/IO. File I/O wasm-gated.
- **Example `data_particles`** (+ a `particles.ron`) — emitters defined entirely in RON; switch
  emitters by name and edit the RON to hot-reload the effect live.

## 8.8.0

Data-driven animation clips. Additive.

### Added

- **`AnimationClipSet`** — load named `AnimationClip`s from a RON file
  (`(atlas: (columns, rows), clips: { "idle": (frames: [0,1,2,1], fps: 6.0, looping: true), … })`);
  frame *indices* are resolved to `UvRect`s via the atlas grid. Clips are ordered alphabetically
  by name (deterministic `index`/`clips`). `from_ron_str`, `clips()`, `index(name)`, `clip(name)`,
  `names()`. Build a player with `AnimationPlayer::new(set.clips().to_vec())` and drive it via
  `player.play(set.index("idle").unwrap())`.
- **`App::load_animation_clips(name, path)`** + **`AnimationClipRegistry`** — registry resource
  (survives scene reset via `register_persistent`) with **hot-reload** wired through the
  `AssetServer` file watcher, mirroring `load_data_table`: editing the RON updates the clips live.
  `ClipSetError` for parse/IO failures.
- **Example `data_anim`** (+ `gen_anim_sheet`) — a sprite animated entirely from a RON clip set;
  switch clips by name; edit the RON to hot-reload the animation.

## 8.7.0

Multi-terrain autotiling. Additive — single-terrain `TilemapAutotile` is unchanged.

### Added

- **`MultiTerrainAutotile`** — a tilemap component (attach instead of `TilemapAutotile`)
  where each non-zero cell autotiles using the [`TerrainRule`] whose `terrain` equals the
  cell's value, connecting only to **same-value** neighbors. So distinct terrains
  (grass/water/sand) each border-tile independently. `edge_16(&[(terrain, base_id), …])`
  builds one identity edge-16 rule per terrain; `with_oob_filled`. Takes precedence over
  `TilemapAutotile`; reuses the reactive `TilemapSystem`'s 8-neighbor UV propagation.
- **`compute_tile_mask_typed(tiles, row, col, nb, oob_filled, terrain)`** — `compute_tile_mask`
  with same-terrain connectivity (a neighbor counts only when its value equals `terrain`).
- **Example `multi_terrain_game`** (+ `gen_multiterrain_sheet`) — grass/water/sand map; paint
  cells with `1`/`2`/`3` (`set_tile`) and watch every terrain re-border live.

## 8.6.0

Versioned save migration. Additive.

### Added

- **`SaveMigrator`** — a chain of schema migrations: `step(n, |value| …)` registers the
  upgrade from version `n` to `n+1`, transforming the decoded `ron::Value`; `current_version()`
  = the number of steps.
- **`save_versioned(path, version, &T)`** — writes an AEAD envelope `{ version, data }` (the
  payload as a `ron::Value`).
- **`load_migrated::<T>(path, &migrator)`** — reads the envelope, applies `steps[stored..current]`
  in order, then deserializes via `ron::Value::into_rust` (bypassing the RON map-vs-struct
  string round-trip). A save tagged newer than the migrator knows → `SaveError::Unsupported`.
- **Example `save_migration`** — writes a v1 save, loads + migrates it to a v2 schema (adds a
  defaulted field), and shows the migrated result on screen.

## 8.5.0

Diagonal (8-direction) pathfinding. Additive — `find_path` is unchanged.

### Added

- **`find_path_diagonal(grid, start, goal)`** — A* on an 8-connected grid (cardinal
  cost 10, diagonal 14, admissible octile heuristic `10·(dx+dy) − 6·min(dx,dy)`). **No
  corner cutting**: a diagonal step is allowed only when both orthogonally-adjacent cells
  are walkable, so paths never slip through the gap between two wall corners. Same endpoint
  convention as `find_path` (excludes start, includes goal; `start==goal` → single cell).
- **Example `diagonal_pathing`** — a grid with a staircase wall barrier; `T` toggles 4-dir
  vs 8-dir and recomputes, so the cardinal zig-zag vs the diagonal shortcut is visible.

## 8.4.0

Audio bus **ducking + sidechain** mixing (native-only audio module). Additive.

### Added

- **Bus ducking** — `AudioManager::duck_bus(bus, gain, attack_secs)` / `release_bus(bus,
  release_secs)` / `bus_duck(bus) -> f32`. A duck is a per-bus gain multiplier (1.0 = none)
  with an attack/release envelope that rides on top of the bus volume, so it never clobbers
  `set_bus_volume`. Driven by `AudioManager::update(dt)`.
- **Sidechain** — `set_sidechain(trigger_bus, ducked_bus, gain, attack_secs, release_secs)` /
  `clear_sidechain(ducked_bus)`. Automatically ducks `ducked_bus` while any channel on
  `trigger_bus` is playing, then releases — the classic "music ducks under dialogue".
  `BusDuck` / `Sidechain` state types re-exported.
- **Example `audio_ducking`** — synthesized music + voice tones; Space plays a voice blip
  that sidechain-ducks the music bus; live on-screen `bus_duck("music")` readout (color-coded)
  makes the duck visually verifiable.

## 8.3.0

Two ergonomic helpers surfaced by the `dig_quest` example (tilemap arc). Additive.

### Added

- **`World::with_resource_mut::<R, _>(|r, world| …)`** — temporarily removes resource `R`,
  runs the closure with `&mut R` **and** `&mut World` at once (the common "I need this
  resource and the rest of the world" borrow), then re-inserts `R`; returns `false` if `R`
  is absent. Replaces the manual `remove_resource` / `insert_resource` dance.
- **`CharacterController::top_down()`** — a constructor for top-down games: like `new()` but
  with snap-to-ground and autostep disabled (the `new()` defaults are platformer-tuned and
  make a top-down character stick to wall surfaces). `slide` stays on.

### Changed

- `dig_quest` refactored onto both helpers (its two `remove_resource::<PhysicsWorld>()`
  sites and the player controller) — the validation that the new APIs read cleanly.

## 8.2.0

Runtime tilemap mutation + neighbor-bitmask autotiling, validated by the new `dig_quest`
example (a destructible-terrain top-down miner). All additive — no breaking changes.

### Added

- **`Tilemap` runtime mutation** — `set_tile` / `get_tile` / `dims` / `cell_center_world`
  / `cell_at_world`. `TilemapSystem` is now **reactive**: it diffs a per-entity cached grid
  and spawns / despawns / updates only the changed cells' tile sprites (a tilemap that never
  mutates renders exactly as before).
- **Autotiling** — the `TilemapAutotile` component (attach to the tilemap entity) selects
  each tile's display UV from its filled neighbors. `Neighborhood::Edge4` (16-tile) and
  `Blob8` (canonical 47-blob); `TilemapAutotile::edge_16` / `blob_47` rulesets, the
  `with_oob_filled(bool)` builder, and the pure `compute_tile_mask`. A changed cell also
  refreshes its 8 neighbors' UVs, so dug holes keep continuous outlines.
- **Incremental tile colliders** — `TileColliderIndex` +
  `PhysicsWorld::sync_static_from_tilemap` diff against the index and add / remove only the
  changed cells (reusing `remove_body`). Use it for the **initial** build too (empty index =
  full build); do not mix with `add_static_from_tilemap` on the same tiles (that would
  double-add colliders so a dug cell never frees). `add_static_from_tilemap` is unchanged for
  static maps.
- **Example `dig_quest_game`** (`examples/games/dig_quest/`) + the `gen_autotile_sheet`
  deterministic asset generator. Native playtest confirmed: digging updates the autotile
  outline + frees collision (the player enters), reset restores, post-reset re-dig works.

## 8.1.10

Deferred-item cleanup from the engine-wide review (asset hot-reload + scripting scope).
No public API change.

### Fixed

- **Atlas file changes are recognized by hot-reload.** `poll_reloads` checked image /
  script / data-table path maps but not `atlas_path_to_id`, so an atlas path was never
  treated as "known" (the underlying image pixels still reloaded via the inner
  `load_image`; this makes the path recognition self-consistent).
- **A failed image load no longer registers a dead file-watcher.** `load_image` watched
  the path even on a failed load; `notify` cannot watch a non-existent path, so a later
  file-create never fired. The watch is now registered only for successfully-loaded paths.
- **Rhai `ScriptRunner` scope no longer grows across frames.** The persistent per-entity
  `Scope` is rewound to its 5-var transform baseline (`x`/`y`/`rot`/`sx`/`sy`) after each
  `on_update`, so `let` bindings introduced per frame don't accumulate. The script scope
  is a transform transport, not a store for cross-frame custom state.

(Not changed: `with_ctx`/`with_ctx_mut` keep their `expect` — calling a Rhai API function
outside `ScriptingSystem::run` is a documented contract violation; a graceful path would
require a sprawling `R: Default` refactor across all script API functions.)

## 8.1.9

Bug fixes: surface-error handling. Final batch of a second-pass engine-wide review (app
main-loop / window / render orchestration + concurrency / WASM / panic-safety — the latter
entirely clean). No public API change.

### Fixed

- **A minimized/occluded window no longer spams `log::error!` every frame.** The surface
  acquisition's `Occluded` and `Timeout` results fell through to an `error!` log, firing
  once per frame while minimized. They are now skipped silently (`Lost`/`Outdated` still
  reconfigure; genuine errors like `Validation` still log).
- **A `Suboptimal` surface is now reconfigured.** After a DPI/monitor/rotation change the
  acquired `SurfaceTexture` can be flagged suboptimal; the frame is presented and the
  surface is then reconfigured so subsequent frames are optimal (was previously ignored,
  causing persistent degradation on some platforms).

## 8.1.8

Bug fixes: UI click/slider/scroll edge cases, save-path hardening, timeline loop wrap.
Final batch of an engine-wide review sweep (UI / asset-save-scripting / timeline-tween-
network — otherwise clean). No public API change.

### Fixed

- **Overlapping `Button`s no longer both fire on one click.** The button pass fired
  `ButtonClicked` for every button whose hit-test passed, so stacked buttons all fired.
  Only the top-most (highest `z`) clicked button now fires. (Cross-widget pointer
  consumption — a button beneath a different widget type — is left as a future TODO.)
- **`ScrollView` with `item_height == 0` no longer panics.** `size.y / 0.0 → inf`,
  `inf.ceil() as usize → usize::MAX`, `+ 1` overflowed (debug panic). Zero/negative item
  height is now guarded.
- **`Slider` emits exactly one `SliderChanged` on the press frame.** The press and the
  same-frame drag-recalculation both fired, producing two events with different values;
  the drag path is now skipped on the press frame.
- **`save_path` rejects path traversal.** `app_name`/`file` are sanitized (only `Normal`
  path components kept), so `"../../etc/passwd"` can no longer escape the data directory;
  legitimate sub-directories (e.g. `"saves/slot1.sav"`) are preserved.
- **Looping `Timeline` wraps with modulo.** A `dt` larger than the timeline `duration`
  (e.g. resuming after a stall) used a single subtract, leaving `time` past the end for
  several frames (stutter). It now wraps with `%` in one frame (guarded against
  `duration == 0`).

## 8.1.7

Bug fixes: audio bus-volume during fades, behavior-tree `AlwaysSucceed`, tilemap tile-id
bounds. Found by an engine-wide review sweep. No public API change.

### Fixed

- **Bus volume is no longer applied twice during audio fades.** A fade stored its start
  volume as `base × bus`, and `update()` multiplied by the bus volume again, so the sink
  got `base × bus²` — an audible volume pop at fade start and a fade at the wrong rate
  (only when a bus had volume ≠ 1.0). Fades now store/interpolate the pre-bus base volume
  and the bus factor is applied exactly once in `update()`.
- **`AlwaysSucceed` behavior-tree decorator passes `Running` through.** It discarded the
  child's status and always returned `Success`, so wrapping a multi-frame action made the
  parent `Sequence`/`Selector` advance on frame 1 and abandon the still-running child. It
  now returns `Running` while the child runs and only converts `Failure → Success`.
- **`TilemapAtlas::uv_for` clamps out-of-range tile ids.** A tile id ≥ `columns × rows`
  produced a UV rect outside `[0,1]`, sampling garbage/wrong tiles. Out-of-range ids now
  return `UvRect::FULL` instead.

## 8.1.6

Bug fixes: physics collision-event delivery + raycast freshness, animation clip-finish +
blend-tree state. Found by an engine-wide review sweep (physics/collision + animation/
skeletal — both otherwise clean). No public API change.

### Fixed

- **`CollisionEvent::Stopped` is delivered when a contacting entity despawns.** The
  handle→entity map was rebuilt each frame from live entities, so an entity removed while
  still touching another resolved to nothing and its `Stopped` exit event was silently
  dropped (listeners waiting for "no longer touching" never fired). The system now keeps
  the previous frame's map and falls back to it when resolving stopped pairs.
- **`cast_ray` no longer hits a just-removed body in the same frame.** The query pipeline
  was only refreshed inside `step()`, so a raycast issued after `remove_body` but before
  the next step saw a phantom collider. `remove_body` now refreshes the query pipeline
  immediately. (`cast_ray`/`cast_ray_with_normal` remain `&self` — no API change.)
- **A 1-frame non-looping clip is no longer reported finished before it is shown.**
  `is_finished()` returned `current_frame >= len-1`, which is `0 >= 0` (true) at entry for
  a 1-frame clip, so an `AnimationEnd` state-machine state transitioned away the same frame
  it was entered. The player now tracks a `finished` flag set when the advance actually
  reaches past the last frame.
- **BlendTree1D no longer gets stuck after a parameter reversal during a crossfade.** If
  `param` returned to the FROM clip's range mid-crossfade, `last_clip` was poisoned and the
  dedup skipped all later transitions, leaving the player stuck on the crossfade target.
  The "already on target" branch is now guarded by `!is_crossfading()`.

## 8.1.5

Bug fixes: scene-stack panic recovery + centered-text wrapping. Found by an engine-wide
review sweep (core ECS + rendering — both otherwise clean). No public API change.

### Fixed

- **`SceneCmd::Pop` no longer permanently silences the builtin tail system.** If a
  `Push`ed scene's first system panicked (added to the panic set) and the scene was then
  `Pop`ped, the retained panic index aliased `HierarchySystem`'s post-drain index and
  skipped it forever — parent-child `GlobalTransform` propagation silently stopped. The
  retain bound is now `new_scene_len` (drops drained + tail indices; the tail gets a
  clean retry, consistent with `reload_scene`).
- **`DrawText::centered` no longer wraps at half the viewport width.** With no explicit
  bounds, the layout buffer width was `viewport_w - position.x`; for a `Center`-anchored
  text positioned at the screen center that is only half the width, so a one-line title
  wrapped to two. `Center` anchor with no bounds now uses the full viewport width/height
  (top-left and explicit-bounds paths unchanged). Width/height selection factored into
  tested pure helpers.

## 8.1.4

Bug fixes: docked-editor gizmo + Inspector edge cases (follow-up to 8.1.3, found by a
second review sweep). No public API change.

### Fixed

- **Resizing a non-`TopLeft`-anchored `UiNode` no longer slides the widget.**
  `UiNode::screen_pos` is `anchor_base(anchor, size) + offset`, and for `Center`/
  `Bottom*`/`*Right` anchors the base depends on `size`. The gizmo resize math only kept
  the fixed corner stable for `TopLeft`, so resizing a `Center`-anchored widget (e.g. the
  `ui_layout_editor_game` menu buttons) drifted on screen. `ui_resize_new_layout` now
  applies an anchor-base compensation so the fixed corner stays put for every anchor
  (`TopLeft` behaviour is unchanged — its base is constant). A shared `anchor_base` helper
  is now the single source for both `screen_pos` and the gizmo.
- **Inspector field edits are no longer dropped when the archetype/selection changes
  mid-frame.** The write-back paired staged values to components by positional index, so
  adding/removing a component or an Undo/Redo that reselected a different entity in the
  same frame mis-paired them (silent edit loss). Write-back is now matched by component
  name and guarded to the entity the values were captured for.
- **Docked viewport mouse-release no longer double-fires.** On release inside the viewport
  the input release ran twice; a release with no matching press could also be produced
  when the pointer was outside. The stuck-state-clearing release now runs only when the
  primary (in-viewport) release path did not.
- **Undo/Duplicate/Paste of a child entity preserves its parent link.** `entity_to_def`
  hard-coded `parent: None`, so restoring a deleted (or duplicating/pasting a) child
  re-spawned it as a root, losing the hierarchy. It now resolves the entity's `Parent`
  to the parent's `Tag`, matching scene-save.

## 8.1.3

Bug fixes: docked-editor reliability (Undo/Redo, Load/Save, Data Tables). No API change
to the public surface; `DataTableRegistry::reload_path` now returns a `ReloadOutcome`
(was `()`).

### Fixed

- **Undo of Delete restores the whole entity.** `EditorCmd::DeleteEntity` captured only
  tag/transform/sprite, so undoing a delete dropped every other component — including
  game components registered via `register_editable_component` (e.g. `Stats`). It now
  captures the full `EntityDef` and restores via `spawn_entity_def`, preserving all
  serde-registered components.
- **Duplicate and Paste are now undoable.** The `⎘ Duplicate` button and Ctrl+V paste
  spawned entities without recording an undo step, so Ctrl+Z did nothing. They now push a
  `CreateEntity` command carrying the entity's `EntityDef`, so Undo removes the copy and
  Redo restores it with all its components.
- **Load Scene fully clears the previous scene.** `do_load_scene` despawned only
  `Transform`-bearing entities, leaving `UiNode`-only entities (menus/HUD) behind and
  duplicating them on load. It now despawns all entities before loading.
- **Data Tables "Reload" reports accurately.** `reload_path` skips reloading a table with
  unsaved edits (dirty-guard); the panel previously still showed "reloaded". It now
  reports the real outcome ("skipped reload — unsaved edits") via the new
  `ReloadOutcome` return value.
- **Save Scene no longer silently drops untagged-parent links.** When a child's parent
  entity has no `Tag` (unrepresentable in `EntityDef.parent`, which is tag-based), the
  link was dropped silently; Save now logs a warning and notes the count of dropped
  parent links in the save-status message.

## 8.1.2

Bug fix: the game-data editor (v8.1.0) is now functional under its documented usage.
No API change.

### Fixed

- **Game-side component registrations and data tables survive `set_scene`.**
  `App::set_scene` resets the `World` (via `SceneCmd::Replace` → `reload_scene`), which
  previously discarded everything registered *before* the first scene was set. With the
  documented pattern —
  ```rust
  app.register_editable_component::<Stats>("Stats", None);
  app.load_data_table("enemies", "enemies.ron");
  app.set_scene(Box::new(GameScene::new()));
  ```
  — the `Stats` reflect/clone/serde registrations and the loaded `DataTableRegistry`
  were silently lost, so `Stats` never appeared in the Inspector, was omitted from saved
  scene RON, and the Data Tables panel was empty. `App` now records these registrations
  and **replays them on every world reset** (mirroring the existing `event_initializers`
  mechanism), and `load_data_table` marks the `DataTableRegistry` persistent. The
  `stat_editor_game` example now works end-to-end (Inspector edit → Save → reload; live
  Data Tables). Built-in components (Transform/Sprite/Tag/UI widgets) were unaffected
  because they are re-registered by `insert_core_resources` each reset.
  - Internal: `SerdeComponentEntry.post_spawn` is now stored as `Arc` (was `Box`) so the
    registration can be replayed. No public signature change.

## 8.1.1

Event-loop responsiveness on macOS (no API change).

### Changed

- The native event loop now uses **`ControlFlow::WaitUntil` frame pacing** instead of
  a `ControlFlow::Poll` busy-spin: it sleeps between frames (requesting a redraw at the
  monitor refresh cadence, clamped to 60–240 Hz) so the macOS main run loop gets idle
  time — smoother window drag/resize and lower idle CPU/battery — while still rendering
  continuously (input events wake the loop immediately). This resolves the macOS
  event-loop-stall TODO previously noted in the surface config. wasm is unchanged
  (`Poll` maps to `requestAnimationFrame`).
- **`desired_maximum_frame_latency` 1 → 2**: lets the GPU keep ~1 frame queued so
  `get_current_texture()` no longer blocks the main thread on vsync for most of each
  frame.

> Note: the dominant factor in editor/game click responsiveness is **build profile** —
> run interactive testing with `--release`; debug builds spend far more per-frame CPU
> and feel laggy regardless of event-loop pacing.

## 8.1.0

Game-data editor: edit component stats and RON data tables in the docked editor
and persist them to disk. Third release of the in-engine editor arc (scene layout
shipped in 8.0.0). Fully additive — no migration needed.

### Added

- **`#[derive(Reflect)]`**: a proc-macro (new workspace crate
  `engine_reflect_derive`) that generates the `Reflect` impl for a struct of
  `f32`/`i32`/`Vec2`/`bool`/`String`/`Color`/`[f32; 4]` fields. `#[reflect(skip)]`
  omits a field; unsupported types fail with a clear compile error. Hand-written
  `Reflect` impls keep working. Add the crate to your `Cargo.toml` (the same way
  you add `engine`) and write `use engine_reflect_derive::Reflect;` then
  `#[derive(Reflect)]`. The macro is a separate crate rather than re-exported from
  `engine` so that `skeleton-engine` stays publishable without first publishing the
  proc-macro to crates.io.
- **`App::register_editable_component::<T>(name, post_spawn)`**: one call wires a
  component for full editor integration — Inspector field editing (Reflect), entity
  duplication (Clone), scene save/load (serde), and the Add/Remove Component
  buttons. `T: Reflect + Serialize + Deserialize + Clone + Default`.
- **Data tables** (`DataTable`, `DataTableRegistry`, `App::load_data_table`): load a
  schema-agnostic RON table (a sequence of `(col: value, …)` rows), read it as a
  World resource at runtime, edit it in the editor's new **Data Tables** tab (bottom
  panel) — per-cell number/string/bool editors, add/delete row, Save — and
  hot-reload disk changes into the running game (a dirty-guard protects unsaved
  edits). Native-only panel; the types are cross-platform.
- **`stat_editor` example** (`cargo run --example stat_editor_game`): entities with
  a derived `Stats` component seeded from an `enemies` data table; edit stats in the
  Inspector (live HUD updates) and tune `enemies`/`items` tables in the Data Tables
  panel — the game-data-editing acceptance test.

### Changed

- The crate is now a **Cargo workspace** (members `.` and `engine_reflect_derive`).
  Consumers that depend on `skeleton-engine` by path or git are unaffected (the
  package name and layout are unchanged); the proc-macro crate is host-compiled and
  does not affect the wasm target.

## 8.0.0

Scene layout editing: the docked editor (v7.1.0) can now select, move, and resize
UI widgets in the viewport and **persist them to a scene file**. Second release of
the in-engine editor arc (next: a game-data / stat-table editor). Breaking because
the scene file format and `EntityDef` shape changed; migration is mechanical.

### Added

- **serde + Reflect on every UI widget** (`UiNode`, `Button`, `Label`, `TextInput`,
  `Slider`, `CheckBox`, `ScrollView`, `Panel`, `LocalizedText`, plus `Anchor`,
  `TextAlign`, `LayoutDir`): widgets now serialize into scene RON and appear/edit
  in the F1/F2 editor Inspector. Runtime state (`ButtonState`, slider/text-input
  cursor & value, scroll offset, `Panel.children`) is `#[serde(skip)]`.
- **Component serialization registry**: `App::register_serde_component::<T>(name,
  post_spawn)` registers any `Serialize + DeserializeOwned + Clone` component so it
  is saved into / loaded from scene files. All UI widgets are auto-registered;
  games register their own types (e.g. stats) the same way. Backed by the
  `SerdeComponentRegistry` resource. Unregistered component names in a loaded file
  warn and are skipped (load never fails).
- **Screen-space UI gizmo**: select a `UiNode` widget to drag it (offset) and
  resize it via 8 handles in the docked/overlay viewport; world sprites gained
  8-handle scale resize (center-fixed). New undo entries
  `EditorCmd::{MoveUiNode, ResizeUiNode, ResizeEntity}` (Ctrl+Z).
- **`ui_layout_editor` example** (`cargo run --example ui_layout_editor_game`):
  load-or-default menu; arrange/resize widgets in the editor, click Save Scene,
  restart, and the edited layout loads — the scene-layout-editing acceptance test.

### Breaking

- **`SceneDef` version 2 → 3.** v2 files still load (the new `components` field
  defaults to empty; the existing version-mismatch warning is informational). v3
  files cannot be read by v7 engines.
- **`EntityDef` gains `components: HashMap<String, ron::Value>`.** Code that
  constructs `EntityDef { .. }` with explicit fields must add
  `components: Default::default()` (or use `..Default::default()`).
- **`TextInput` gains `initial_text: String`; `text` and other runtime fields are
  now `#[serde(skip)]`.** Set `initial_text` for design-time content; the registry
  post-spawn hook copies it into `text` on load. **`Slider` gains
  `initial_value: f32`; `value` is `#[serde(skip)]`** (same pattern). Constructors
  (`Slider::new`) are unchanged at runtime.
- Components are stored in scene RON as a string-encoded `ron::Value` (ron 0.8's
  `Value` cannot round-trip enums like `Anchor`); this is an internal format detail
  but visible in saved files.

## 7.1.0

The docked editor shell: a second editor mode that lays the screen out like a
commercial engine — side panels around a central game viewport — so editing no
longer covers the game. First release of the in-engine editor arc (next: UI
widget editing, then data tables). No breaking changes.

### Added

- **Docked editor mode (`F2`, native only)**: egui owns the window; the left
  panel holds Entities/Scene tabs, the right panel the Inspector, the bottom
  panel Assets, and a top toolbar carries play/pause (`▶`/`⏸`), single-frame
  step (`⏭`), snap controls, and scene save/load. The game renders into an
  editor-owned offscreen texture shown in the central panel (size follows the
  panel, 3-frame resize debounce). `F1` keeps the existing floating-window
  overlay unchanged; the modes are mutually exclusive.
- **Viewport-local input routing**: while docked, the game receives the cursor
  translated into viewport coordinates (`viewport_to_game`), and pointer events
  pass through a layer-aware gate (`docked_game_pointer_allowed`) — clicks
  inside the viewport reach the game/gizmo, clicks on panels and popups stay in
  egui, and typing in the Inspector never leaks into game input. The selection
  gizmo (drag to move, snap, undo) works inside the docked viewport.
- **Editor pause**: the toolbar pause skips scene systems at the engine level
  while keeping the builtin tail (`HierarchySystem`) running, so dragging a
  parent while paused still moves its children. `⏭` advances exactly one full
  frame. The `GameState` resource is untouched (it remains a game-side
  convention).
- **`ViewportSize` delegation**: while docked, `ViewportSize` reports the
  central panel's logical size, so cameras, screen-space UI, and
  `Camera::screen_to_world` work unchanged against the viewport; the real
  window size is restored on exit.

## 7.0.0

The renderer-dependency major window: the whole wgpu/glyphon/egui stack moves to
current majors, resolving `RUSTSEC-2026-0002` (glyphon 0.6 pinned `lru` < 0.16.3 —
previously archived as accepted risk in `docs/SECURITY_HARDENING_2026_05.md`, now
closed). Engine-side rendering behavior is preserved exactly (sRGB-first surface
format, AutoVsync, frame latency 1, WebGL2 limits on wasm, egui dithering off);
verified by the full gate suite, `wasm_smoke` (connect + non-blank render, HUD
correct), and windowed playtests (lighting pass, SM crossfade mid-blend, F1
inspector overlay).

### Breaking — toolchain & dependencies

- **MSRV 1.88 → 1.92** (`rust-version = "1.92"`): egui 0.34 requires Rust 1.92,
  cosmic-text 0.18 requires 1.89. CI pins Rust 1.95.0 (current stable, also used
  for local gates).
- **wgpu 22 → 29** (`webgl` feature unchanged), **glyphon 0.6 → 0.11**
  (cosmic-text 0.18), **egui / egui-wgpu / egui-winit 0.29 → 0.34**, winit minimum
  `0.30.13`. Transitive `lru` resolves to 0.16.4, closing `RUSTSEC-2026-0002`.

### Breaking — API changes

- **`GpuContext::clear()`** returns `Result<(), String>` (was
  `Result<(), wgpu::SurfaceError>` — wgpu 29 removed `SurfaceError`; surface
  acquisition reports through the `wgpu::CurrentSurfaceTexture` enum). *Migration:*
  treat the `Err` as an opaque message. The engine main loop handles
  reconfigure-on-`Lost`/`Outdated` internally, exactly as before.
- **`RenderTarget` pub fields and `DebugUi::ctx()` now expose wgpu 29 / egui 0.34
  types** — code touching `RenderTarget.{texture,view,sampler,bind_group}` or
  writing custom egui panels compiles against the new majors. Notable for panel
  code: `Rounding` → `CornerRadius`, `Context::style()` → `global_style()`. egui
  0.34's skrifa font backend renders text slightly differently (default text size
  12.5 → 13.0) — debug-UI only, game rendering unaffected. wgpu resources are now
  `Clone` (internally refcounted); `RenderTarget.bind_group` stays `Arc`-wrapped
  for API stability.

### Fixed

- **egui texture deltas are no longer dropped on skipped frames** — when surface
  acquisition failed for one frame (`Lost`/`Outdated`/`Timeout`, e.g. during a
  live window resize), the unconsumed `textures_delta` was overwritten by the next
  frame. egui 0.29's ab_glyph backend re-sent the full font atlas on every change,
  silently self-healing; egui 0.34's incremental skrifa updates made the latent
  bug fatal (panic on F1: "Tried to update a texture that has not been allocated
  yet"). Deltas now merge old → new (`merge_textures_delta` in
  `src/app/schedule.rs`, +2 regression tests). Found by the windowed playtest.

### Changed

- `src/app/egui_pass.rs` dropped its `unsafe` transmute — wgpu 29's
  `RenderPass::forget_lifetime()` is the supported replacement for the
  egui-wgpu `RenderPass<'static>` requirement.
- egui renderer keeps dithering **off**, matching the pre-0.34 explicit arguments
  (`RendererOptions::default()` would have silently enabled it).

## 6.0.0

The v6 breaking window: the three "Verified-but-deferred" items recorded in 5.1.3,
the v5.0.0 `Arc<str>` conversion completed for particles, and the HierarchySystem
pipeline integration. Every change below lists its migration. The fifth scoped item
(BehaviorSystem take/add archetype migrations) was investigated and **deliberately
kept** — the evaluation is recorded as a PERF comment in `BehaviorSystem::run`.

### Breaking — API changes

- **Animation systems own a scratch buffer** — `AnimationSystem`, `BlendTreeSystem`,
  and `StateMachineSystem` are no longer unit structs (they keep a reused per-frame
  entity buffer, eliminating three per-frame `Vec` allocations). *Migration:*
  construct with `::new()` (or `::default()`):
  `app.add_system(AnimationSystem)` → `app.add_system(AnimationSystem::new())`,
  `Box::new(BlendTreeSystem)` → `BlendTreeSystem::new()`, same for
  `StateMachineSystem`. `LABEL` constants and ordering semantics are unchanged.
- **Allocation-free state-machine parameter setters** —
  `AnimationStateMachine::{set_bool, set_float, add_trigger}` now take
  `impl Into<String> + AsRef<str>` and only allocate on first insert (updates are
  in-place). *Migration:* none for `&str` / `String` / `&String` / `Cow<str>`
  callers — these satisfy both bounds and compile unchanged. Only an exotic type
  implementing `Into<String>` but not `AsRef<str>` needs adapting.
- **`ParticleEmitter.texture` is `Option<Arc<str>>`** — completes the v5.0.0
  `Sprite.texture` conversion (analysis #9); per-spawn clones become refcount bumps.
  *Migration:* `texture: None` and `texture: Some("x.png".into())` compile
  unchanged; `texture: Some(string_var)` becomes `Some(string_var.into())`
  (std provides `From<String> for Arc<str>`). `ParticleEmitter` has no serde derive,
  so no save-format impact.
- **`HierarchySystem` joined the labeled pipeline** — it is registered automatically
  by `App::new()` as a permanent tail built-in (survives `SceneCmd::Replace`) instead
  of being force-run outside the scheduler. *Migration:* none for games that do not
  order around hierarchy propagation — default frame behavior (GlobalTransform
  updated after all user systems, before render) is identical. New capability:
  `.after(HierarchySystem::LABEL)` / `.before(...)` constraints now actually take
  effect (the LABEL previously existed but was a dead symbol). `docs/PATTERNS.md`
  gained the ordering row.

### Changed

- Examples updated to the `::new()` system constructors (sm_crossfade,
  blend_locomotion, platformer).

## 5.1.3

Cleanup batch over the low/leftover findings from the 2026-06-12 full-source review
(report §3 — 16 items locally re-verified first: 9 applied here, 2 refuted, 4 deferred
as breaking-or-architectural, 1 skipped as not worth the churn). Pure internal
refactors and perf fixes — zero public-API change, no migration.

### Performance

- **Particle emitter texture clone removed from the per-frame path** — the
  `Option<String>` texture is now looked up lazily only when particles actually spawn
  (was cloned per emitter per frame regardless of emission).
- **`World::despawn` change-tracking is O(1) per entity** — `added_this_tick` /
  `changed_this_tick` restructured from `HashSet<(Entity, TypeId)>` (full-set `retain`
  per despawn) to `HashMap<Entity, HashSet<TypeId>>`. Mass-despawn sites (tilemap
  teardown, particle bursts, pool clears) no longer scan the whole tracking set per
  entity. Query semantics unchanged.
- **Scripting blackboard snapshot allocates one String per entry instead of two**
  (`bb_snap` now stores keyless `BlackboardValue`s; the write-path `BbEntry` keeps its
  key, which the apply loop needs).

### Internal cleanup

- Sprite/AtlasSprite culling block (4 copies) extracted into one helper; UI widget
  passes' UiNode layout extraction (4 copies) extracted into `node_layout`; the
  `SCRIPT_CTX` access boilerplate (15 copies) extracted into `with_ctx`/`with_ctx_mut`;
  audio fade-start-volume logic (3 copies) unified into one `fade_start_vol` method.
- Editor entity labels standardized to `"Entity {index}:{generation}"` (the entity-list
  panel used a different short form than every other panel).
- Doc notes: hot reloading documented as native-only (silent empty result on `wasm32`),
  matching the lighting platform-note precedent; the physics sensor-pair `ordered_pair`
  normalization now carries a comment recording that it is defensive (verified against
  rapier's stable edge-slot order) so future reviews don't re-investigate.

### Verified-but-deferred (recorded, not fixed)

- Per-frame `Vec<Entity>` scratch in the three animation systems — requires turning pub
  unit structs into field structs (breaking); deferred to the next major.
- `AnimationStateMachine::set_bool`/`set_float` key allocation — needs a signature-bound
  change (breaking risk); deferred.
- `BehaviorSystem` take/add archetype migrations — structural (`tick` needs `&mut World`);
  acceptable at typical AI entity counts, revisit only if profiling demands.

## 5.1.2

Bug-fix batch from the scheduled 2026-06-12 full-source review
(`docs/CODE_ANALYSIS_2026-06-12.md` — Top-10 locally re-verified: 8 confirmed,
2 refuted; the 8 confirmed findings are all addressed here). No migration; one
small API addition noted below.

### Fixed

- **Network receive-queue overflow accounting** (the round's two high findings) —
  `ReceiveQueueFull.dropped` now accumulates across every rejected message (was a
  constant `1`, silently discarding all subsequent overflow), and events already in
  the queue are never evicted once the marker is installed. When the marker is first
  installed it displaces the youngest queued event, which is now *counted*
  (`dropped` starts at 2). Queue length never exceeds the configured capacity.
  Native and wasm paths are semantically identical.
- **Crossfade interrupt pop** — calling `play_with_crossfade` toward a *third* clip
  while a blend is in flight now promotes the in-flight TO side to the new FROM
  (`mix(B, C, 0)` on the first frame) instead of popping back to the original FROM
  clip image. The 5.1.1 same-target idempotency guard is unaffected.
- **Crossfade completion stutter** — completion now carries the to-clip's accumulated
  sub-frame timer into `AnimationPlayer.timer` (with this tick's `dt` counted exactly
  once) instead of resetting to `0.0`, which visibly stretched the first post-blend
  frame on low-fps clips.
- **Silent collision-event drop warning** — `PhysicsSystem` now `log::warn!`s once
  when collisions/triggers occur but `Events<CollisionEvent>` /
  `Events<TriggerEvent>` was never registered, naming the exact
  `register_event` call to add (previously the events vanished with no signal).
- **Per-sprite `Arc<str>` re-allocation** — the renderer's `image_handle` path uses
  the new `Handle::path_arc()` (O(1) refcount bump) instead of `Arc::from(h.path())`
  (per-sprite per-frame string copy).
- **Doc gaps** — `docs/PATTERNS.md` ordering table gains the
  `BlendTreeSystem` before `AnimationSystem` row; `AmbientLight` / `PointLight` /
  `LightingRenderer` doc comments now state the native-only / wasm32-no-op limitation.

### Added

- `Handle::path_arc() -> Arc<str>` — owned handle path without copying the string.

## 5.1.1

Bug-fix batch from the post-release code review of the 5.1.0 features (10 confirmed
findings, three root causes). No migration needed; one small API addition noted below.

### Fixed

- **Audio release envelope redesigned** (root cause: shadow state + stale volume reads).
  `stop()` during *any* in-progress `stop_when_done` fade (release **or** `fade_out`)
  now cuts immediately — `fade_out` is a real bypass path as documented, and a second
  `stop()` mid-release still cuts. The release fade starts from the **current
  interpolated** fade position instead of the stale override (no more start-of-release
  pop). Completed teardown fades no longer persist `0.0` into the channel volume, so
  the next `play_*` on a reused channel starts at the `set_volume` level (regression
  fix). `stop()` on a naturally-drained sink cuts immediately instead of scheduling a
  silent release. Internals: the `releasing` HashSet is gone; `Fade` construction is
  unified (`Fade::stop_fade`) with one consistent minimum-duration rule.
- **State-machine crossfade guards** (root cause: `current_clip` stays on the FROM clip
  during a blend). `AnimationPlayer::play_with_crossfade` re-fired with the same target
  mid-blend is now idempotent — oscillating threshold transitions can no longer reset
  the blend every frame. `StateMachineSystem` evaluates `AnimationEnd` via the new
  `AnimationPlayer::is_clip_finished(clip_index)` (returns true only when not
  crossfading and that clip is the finished current clip), so a crossfaded-into
  one-shot state plays its clip to completion instead of exiting on the first frame.
  The `AnimationStateMachine` ↔ `BlendTree1D` interaction is now documented (SM
  transitions intentionally interrupt an in-progress BT blend; avoid driving the same
  player with both unless that is desired).
- **Script steering commands are mutually exclusive** — `seek_target` / `flee_from` /
  `arrive_at` / `wander` each remove the other three steering components before
  attaching their own (previously a single `wander()` permanently overrode later
  commands via the steering system's last-writer-wins order), and `stop_steering()`
  removes all four so a stopped entity stays stopped. Rust-side multi-component
  steering composition is unaffected.

### Added

- `AnimationPlayer::is_clip_finished(clip_index)` — crossfade-aware finish check used
  by the state machine; public for game code with the same need.

## 5.1.0

The three feature candidates deliberately split out of the 2026-06-10 analysis round,
each validated by a playable example per the `docs/VISION.md` loop. Fully additive —
no migration needed from 5.0.0.

### Added

- **Per-transition crossfade on `AnimationStateMachine`** — `AnimTransition` gains a
  `crossfade_duration: f32` field (default `0.0` = hard switch, the previous behavior)
  and `add_transition_crossfade(from, to, conditions, duration)` registers a transition
  that blends into the target clip. `StateMachineSystem` drives the existing
  `AnimationPlayer::play_with_crossfade` path — the same 2-UV shader-lerp used by
  `BlendTreeSystem`, no new blend machinery. `add_transition` keeps its signature
  (now a thin wrapper with `0.0`). Example: `sm_crossfade` (side-by-side hard-switch
  vs. crossfaded character; run `gen_blend_sheet` first).
- **Rhai steering bindings for `Arrive` / `Wander`** — scripts can now use the full
  steering set (previously only Seek/Flee were bound):
  `arrive_at(tx, ty, speed, slow_radius, stop_radius)` and
  `wander(speed, change_interval)`, following the existing `seek_target`/`flee_from`
  conventions (f64 params, last call per frame wins, `SteeringVelocity` auto-attached).
  The Wander apply step preserves the component's internal timer/direction so per-frame
  script calls don't reset the direction-change rhythm. Example: `script_steering_game`
  (mouse-following Arrive agent + autonomous Wander agent, both script-driven).
- **`AudioEffect::release_secs` implemented** (was a documented no-op stub) —
  `AudioManager::stop` on a channel whose effect has `release_secs > 0.0` now fades the
  volume to zero over that duration through the existing fade machinery, then tears the
  sink down. `0.0` keeps the immediate cut. A second `stop` during the release, or a new
  `play_*` on the channel, cuts immediately. Requires `AudioSystem` (or manual
  `update(dt)`) to progress, like all fades. Example: `audio_fades` (extended — R/S/I
  keys demo release vs. immediate stop).

## 5.0.0

The breaking batch from the 2026-06-10 analysis (`docs/CODE_ANALYSIS_2026-06-10.md`):
Top-10 items #2 and #8, removal of everything deprecated in 4.6.0, the visibility
narrowings triaged out of the 4.6.0 sweep, and small breaking consistency items.
Every change below lists its migration.

### Breaking — removed (all deprecated since 4.6.0)

- **`DebugDrawQueue` / `DebugRect`** — migrate to `DebugDraw::rect_filled_z(min, max, color, z)`
  (or `rect_filled` for z = 0).
- **`World::register_reflect`** — use `register_reflect_named::<T>("Name")` (the removed
  overload stored an empty type name and broke the Inspector display).
- **`NetworkEvent::JsonParseError`** — never emitted by the engine; delete the match arm
  (protocol-level parse errors are the game's concern).
- **`App::load_texture`** — use `load_image` (returns a `Handle<ImageAsset>`, participates
  in hot reload).
- **`ParticleEmitter::for_burst`** — renamed to `ParticleEmitter::burst` in 4.6.0.
- **Pre-v5 re-export shims** — `animation::player::{UvRect, BlendUv}` → `renderer::uv`,
  `timeline::Lerp` → `tween::Lerp`, `prefab::topological_sort_entities` → `hierarchy`,
  and the `components::*` migration facade (`AnimationClip`, `AnimationPlayer`, `UvRect`,
  `FontData`, `GameState`, `PendingResize`, `ShouldQuit`, `ViewportSize`, `WindowConfig`).
  All root re-exports (`engine::UvRect`, `engine::Lerp`, `engine::topological_sort_entities`, …)
  keep working — only the deep legacy paths are gone.

### Breaking — API changes

- **Physics handle newtypes (analysis #2)** — `PhysicsWorld` no longer leaks rapier types:
  new `BodyHandle` / `ColliderHandle` newtypes (mirroring `JointHandle`) flow through every
  factory return, `PhysicsBody`'s fields, `RaycastHit.collider_handle`, raycasts, joints,
  `move_character`, and the collider accessors. *Migration:* code that only passes handles
  back into `PhysicsWorld` compiles unchanged via inference; code naming rapier handle types
  imports `engine::{BodyHandle, ColliderHandle}` instead. Escape hatch for forks that drop
  to raw rapier: `.raw()` on both newtypes, and `rigid_body[_mut]` / `get_collider[_mut]`
  still return raw rapier references.
- **`Scene::on_enter` takes a `SystemRegistrar` (analysis #8)** — scenes can finally
  register systems with label ordering. *Migration:*
  `fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>)` →
  `fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar)`;
  `systems.push(Box::new(X))` → `systems.add(X)`; ordering:
  `systems.add_labeled(X, SystemConfig::new().after(Y::LABEL))`. The settings_menu example
  demonstrates a real constraint (`UiSystem` after `LayoutSystem`).
- **`Sprite.texture` is `Option<Arc<str>>` (analysis #9 remainder)** — per-sprite per-frame
  batch-key `String` clones become refcount bumps. *Migration:* `Sprite::textured("x.png")`
  and `textured_with_handle` keep compiling (`impl Into<Arc<str>>`); struct literals need
  `texture: Some("x.png".into())`. RON/serde wire format unchanged.
- **`SystemMeta` merged into `SystemConfig`** — they were field-for-field identical.
  *Migration:* replace the name; `compute_order` now takes `&[SystemConfig]`.
- **`ShaderMaterial` caches its pipeline hash** — construct via
  `ShaderMaterial::new(frag_source, params)`; `frag_source` is private behind
  `frag_source()` / `set_frag_source()` (which re-hashes), so the cached hash can never
  desync. `params` stays pub. The renderer's per-frame WGSL hashing is gone.
- **`#[non_exhaustive]` on `DebugShape` and `NetworkEvent`** — external matches need a
  `_ =>` arm; future variants stop being breaking changes (`ReflectValue` precedent).
- **Visibility narrowings** — `GpuLightData` / `LightingUniforms` and
  `PostProcessRenderer.{target_view,width,height}` are `pub(crate)` (GPU internals);
  `TouchState` event fields are private behind `began()` / `moved()` / `ended()` /
  `pinch_delta()` / `swipe()` accessors; `input` submodules are private — import from
  `engine::input::{…}` or the crate root (`engine::AxisBinding` etc. unchanged).

### Changed

- Examples write quits via `ShouldQuit::quit()` instead of `q.0 = true` (field stays pub;
  examples teach the canonical API).

## 4.6.0

Non-breaking batch from the 2026-06-10 full-codebase analysis
(`docs/CODE_ANALYSIS_2026-06-10.md`, Top-10 items #1/#3/#4/#5/#6/#7/#9-partial/#10),
plus a follow-up sweep over ~30 of the remaining non-Top-10 findings (2026-06-11).
The remaining Top-10 items (#2 rapier handle newtypes, #8 `on_enter` system
registrar, plus removal of everything deprecated here) form the planned v5 breaking batch.

### Added

- **`LABEL` constants on all built-in systems** — Physics/CollisionGrid/CollisionDebug/
  Network/Particle/Tilemap/Audio/SkeletalAnimation/Hierarchy/Steering/Behavior/
  Localization/Scripting/Timeline join the five systems that already had one, so every
  engine system can now be referenced in `add_system_labeled` ordering. The platformer
  example demonstrates labeled registration; `docs/PATTERNS.md` gained a
  "System ordering with labels" section with the known constraints.
- **`SceneChange::take` / `is_pending`**, **`ShouldQuit::quit` / `is_quitting`**,
  **`ParticleEmitter::burst`** (canonical name for `for_burst`), and a root re-export
  for **`NetworkConfig`** — small API-surface consistency additions.

- **`save::write_ron` / `save::read_ron`** — plaintext pretty-RON read/write for design-time
  assets. `SceneDef`/`Prefab` `save`/`load` now produce human-editable text files instead of
  AEAD-encrypted binary (a hackability violation for level files); `read_ron` transparently
  falls back to the encrypted format so pre-4.6 files still load. Encrypted `save`/`load`
  remain the player-save path.
- **`DebugDraw::rect_filled` / `rect_filled_z`** — filled, z-ordered rectangles on the modern
  debug-draw resource, covering everything the legacy queue did.
- **Native `NetworkClient::is_connected()`** — parity with the wasm client (previously
  wasm-only, an undocumented platform API split); backed by an `AtomicBool` the socket
  thread clears on every exit path.

### Changed

- **`UvRect`/`BlendUv` moved to `renderer::uv`**, **`Lerp` moved to `tween`** — semantic
  homes instead of accidental ones (`animation::player`, `timeline`); six modules no longer
  compile-depend on `animation`, and `network::SnapshotBuffer` no longer imports the cutscene
  module. Old paths and all root re-exports keep working via `pub use` shims.
- **Editor state extracted from `App`** — 17 editor-only fields (gizmo, clipboard, undo
  history, component factories, snap, selection) now live in one internal `EditorState`
  struct (`src/app/editor/state.rs`); a fork removes the editor by deleting one field + one
  module. Internal-only; no public API change.
- **Per-frame allocation fixes** — the lighting pass no longer creates its bind group every
  frame (cached, invalidated on resize/reconfigure); the sprite renderer no longer clones
  WGSL material sources per frame (at most once per *new* pipeline). Remaining per-sprite
  texture-key `String` clones need an API break and are deferred to v5.
- **Findings-sweep cleanups (2026-06-11)** — per-frame allocation pass (text queue drained
  via `mem::take`, physics event-diff scratch buffers reused, single-pass particle emitters
  and panel layout, editor-UI allocations gated behind `is_enabled`, `exec_order` take/swap);
  `topological_sort_entities` rehomed `prefab` → `hierarchy` (shim kept); O(1) `despawn`;
  deduplicated `App::new` / `AssetServer::new` struct literals, editor Tag/multi-select UI
  blocks, input bind methods, and the fullscreen-quad vertex shader; dead private
  `play_streaming` removed; doc clarifications across modules (wasm no-op fades,
  CollisionGroups vs CollisionLayer, LocaleResource bridge, system-ordering caveats).

### Fixed

- **Animation frame catch-up** — the main-clip advance now catches up multiple frames on a
  large `dt` (previously advanced at most one frame per tick; the crossfade path already
  did this correctly).
- **`CharacterController::max_slope_angle` desync** — setting the public field directly now
  takes effect on the next `move_character` call (previously only `with_max_slope_deg`
  synced the internal rapier controller).

### Deprecated

- **`DebugDrawQueue` / `DebugRect`** — superseded by `DebugDraw::rect_filled_z`. Still
  registered and drained for compatibility; removal planned for v5. `CollisionDebugSystem`,
  the editor selection highlight, and the `sokoban` example are migrated.
- **`World::register_reflect`** (stores an empty type name, breaking Inspector display — use
  `register_reflect_named`), **`NetworkEvent::JsonParseError`** (never emitted by the
  engine), **`App::load_texture`** (use `load_image`), and **`ParticleEmitter::for_burst`**
  (renamed `burst`). All removal-planned for v5.

## 4.5.0

### Added

- **`salvage_run` example** (`examples/games/salvage_run/`) — an **area-of-interest (AOI)
  streaming** networked world: a single ship roams a world far larger than the window (2400×1800 vs
  800×600) while an authoritative server simulates ~120 wandering entities of two typed kinds
  (slow-drifting salvage, roaming drones) and streams each client **only** the entities within an
  interest radius of its last-reported position. Entities continuously stream in and out as the
  player moves — interest management made visible: a live "streaming X / 120" readout, a resizable
  AOI (`-` / `=`) with an on-screen boundary ring, and entity pop-in/out at the edge. Reuses
  `engine::SnapshotBuffer<Vec2>` per streamed entity (its third call site) for smooth motion at a
  low 12 Hz, two `RemoteEntities` maps for the two kinds, example-local last-seen + timeout eviction
  for entities that leave the AOI (the server signals departure only by *omission*), and
  `RemoteEntities::clear` on disconnect. The first example to stress AOI churn / staleness and to
  tear down on disconnect — see `docs/REMOTE_ENTITIES_DESIGN.md` (#4/#5/#7). Ships native + to the
  browser (`web/`). No engine API change (purely additive example).

## 4.4.0

### Added

- **`SnapshotBuffer<T: Lerp>`** — a generic per-entity snapshot-interpolation buffer for smoothing
  server-owned remote state that arrives at a low snapshot rate. Stamp each snapshot with the
  client clock (`push`), then `sample` a slightly delayed render time so playback always
  interpolates between two real samples (clamping at the ends). Generic over any `Lerp` value, so
  it interpolates `f32` (e.g. a rotation angle), `Vec2` (a position), `Color`, etc. It is
  **orthogonal** to `RemoteEntities`: that owns the `id → Entity` lifecycle, this owns the value
  history the renderer reads — games keep them as parallel maps. This is the promotion of
  `predict_shooter`'s former private `Interp` (now migrated onto it), triggered by a second
  interpolating example — see `docs/REMOTE_ENTITIES_DESIGN.md`.
- **`orbital_dodger` example** (`examples/games/orbital_dodger/`) — an interpolation-only networked
  game: cross the field to a vault while dodging the server's drifting, spinning hazards. The
  hazards are wholly server-authoritative at a low 10 Hz; the local player never round-trips, so
  the only netcode is interpolation (no prediction). Each hazard interpolates two channels —
  position (`SnapshotBuffer<Vec2>`) and spin angle (`SnapshotBuffer<f32>`) — which is what
  justified making the buffer generic. `I` toggles interpolation off to reveal the raw 10 Hz
  judder. Ships native + to the browser (`web/`).

## 4.3.1

### Fixed

- **Gamepad backend crash isolated (gilrs).** A controller could panic gilrs inside its event poll
  (`gamepad(id).unwrap()` on `None`), crashing the whole app ~1 s after launch. `App::poll_gilrs`
  now wraps the poll in `catch_unwind` (mirroring the per-system isolation in `schedule.rs`) — a
  flaky controller disables gamepad input for the session instead of crashing — and gilrs was
  upgraded `0.10 → 0.11.2` (reworked macOS HID backend). Note: on macOS the GameController
  framework takes *exclusive* ownership of Xbox/PlayStation pads, so gilrs (IOKit HID) sees a
  `Connected` event but no input; gamepad input works on Linux/Windows or with a generic-HID pad.

## 4.3.0

### Added

- **`RemoteEntities<K>`** — a reusable helper for the `id → Entity` lifecycle that networked games
  repeat: spawn-on-first-sight and despawn-on-removal of server-owned remote entities. Methods:
  `get_or_spawn`, `get`, `contains_key`, `remove` (despawns the entity), `clear`, `len`,
  `is_empty`, `iter`. It owns only the mapping plus spawn/despawn lifecycle — what to spawn (a
  closure), how to update an existing entity, and any parallel game-state maps stay in the game.
  The `mp_client` and `coin_race` examples now use it instead of inline `HashMap<usize, Entity>`
  bookkeeping. A richer version (interpolation, client-side prediction, update callbacks) is
  deliberately deferred until a third distinct networked example reveals its shape — see
  `docs/REMOTE_ENTITIES_DESIGN.md`.

## 4.2.0

### Changed

- **Crisp wasm rendering on Retina/HiDPI.** The wasm drawing buffer is now sized to the canvas's
  logical size × `devicePixelRatio` (uniform scale, capped so neither axis exceeds the WebGL2 2048
  max texture size) while the canvas CSS display box stays at the logical size, so the browser maps
  the buffer 1:1 instead of upscaling a logical-size buffer. Previously wasm rendered into a
  logical-size buffer (a deliberate `scale_factor = 1` workaround) — correct, but soft on Retina.
  The world viewport stays logical and `DisplayScaleFactor = buffer / logical`, so sprites and UI
  keep their coordinates and text now renders at device resolution. The logical size is read from
  the authored `<canvas>` width/height attributes (stable across scene transitions), not
  `WindowConfig`. Native rendering is unchanged.

## 4.1.0

### Added

- **`coin_race` runs in the browser (wasm).** The `coin_race_game` client now compiles to
  wasm and connects to the native `coin_race_server` over `ws://127.0.0.1:9002`, so native
  windows and browser tabs share one authoritative game. A `#[wasm_bindgen] run_coin_race`
  entry point lives in the example (not the engine library, keeping the engine a
  genre-agnostic skeleton), and `examples/games/coin_race/web/` adds an `index.html` plus a
  `build.sh` that drives `cargo build --example` + `wasm-bindgen`. This establishes the
  reusable path for shipping an engine *example game* to the web: previously only the bundled
  library demo (`examples/wasm/`, built with `wasm-pack`) could run in a browser, because
  `wasm-pack` builds only the library crate. Verified end-to-end — a browser tab's wasm
  WebSocket connects to the authoritative server and renders the player avatar and the
  server-spawned coin field via WebGL2.
- **Embedded default font for wasm text.** The browser sandbox has no system fonts, so
  `FontSystem::new()` loads an empty font db and the engine previously skipped creating the
  text renderer on wasm entirely (cosmic-text panics shaping with no fonts), meaning
  `DrawText`/HUD text silently did not render unless the game supplied a `FontData`. The engine
  now embeds DejaVu Sans (`assets/fonts/DejaVuSans.ttf`, Bitstream Vera / Arev license) and
  falls back to it on wasm when no `FontData` is set, so HUD text renders out of the box. The
  font is `include_bytes!`'d under a wasm-only `cfg`, so native binaries (which use OS fonts)
  do not embed it.

### Fixed

- **Wasm HiDPI viewport was halved on Retina displays.** The logical `ViewportSize` was
  computed as `surface_size / devicePixelRatio` for all targets, but on wasm the surface is
  already sized to the canvas DOM (CSS-logical) size — the resize handler caps it there to
  respect the WebGL2 texture limit — so dividing by the DPR again halved the world viewport.
  On a Retina display (DPR 2) a fixed-coordinate scene was projected into a half-size viewport
  and rendered almost entirely off-screen; the engine only rendered correctly at DPR 1. The
  DPR division now applies on native only (where the surface is physical pixels). Surfaced by
  playtesting the `coin_race` wasm example on a real Retina display — sprites that were pushed
  off-screen now render in place. (The `examples/wasm/` lib demo masked this because it adapts
  its layout to `ViewportSize` instead of using fixed coordinates.)
- **Wasm canvas was stretched, clipping HUD text.** winit sizes the canvas's CSS *display* box
  to the window's logical size (the 1280 default when `WindowConfig` isn't applied at canvas
  creation), which can differ from the drawing buffer — so the browser stretched an 800px buffer
  across a 1280px display and, being wider than the window, centred and clipped it. Fixed-position
  HUD text fell off the left edge while sprites (mid-canvas) stayed visible. `finish_init` now
  sets the canvas CSS width/height to its drawing-buffer size (after winit has sized it) so the
  canvas displays 1:1 with what the engine renders; a game can still override with `!important`
  CSS. Surfaced once the embedded default font made wasm text render for the first time.

## 4.0.0

### Added

- `coin_race` example (`examples/games/coin_race/` — `coin_race_game` client +
  `coin_race_server`): first playable-game use of `NetworkClient` / `NetworkSystem` /
  `NetworkEvent` in an **authoritative** design, not the position-relay model of
  `mp_client`/`mp_server`. Two or more players race to collect coins; the standalone server
  owns the coin field and the scoreboard, arbitrates contested pickups (first `grab` claim
  wins), keeps the field full, and announces the winner. Closes the last engine subsystem —
  networking — that had no playable-game example. No engine source changed: `NetworkClient`
  and `NetworkSystem` carried a full authoritative game as-is, confirming the API is
  sufficient for this pattern.

### Breaking

- **`engine::ImpulseJointHandle` (a re-export of `rapier2d`'s `ImpulseJointHandle`) is
  removed and replaced by the opaque `engine::JointHandle` newtype.**
  `PhysicsWorld::add_revolute_joint`, `add_distance_joint`, and `add_prismatic_joint` now
  return `engine::JointHandle`, and `remove_joint` takes it. The inner rapier handle is
  engine-private: a `JointHandle` can only be produced by an `add_*_joint` call, decoupling
  game code from the rapier type. Migration: replace `use engine::ImpulseJointHandle` (or
  `use rapier2d::dynamics::ImpulseJointHandle`) with `use engine::JointHandle`, and update
  return-type annotations / stored fields accordingly. Call sites that discard the return
  value need no change.

## 3.0.0

### Added

- `Color` newtype (`engine::Color`): a single unified RGBA color type with `rgb` / `rgba` /
  `rgba_u8` constructors, `From<[f32; 4]>` / `From<[f32; 3]>` / `From<[u8; 4]>` conversions,
  and `to_array` / `to_u8` / `to_rgb` helpers for the GPU / glyphon boundaries. Replaces the
  previous mix of raw color arrays throughout the public API (see Breaking).
- `AudioSystem` (`engine::AudioSystem`): a built-in system — register it like any other —
  that calls `AudioManager::update(dt)` every frame so scheduled fades (`fade_out` /
  `fade_volume`) actually advance. Previously fades were silently inert unless the game
  manually drove `update()`. Also adds an SFX file-bytes cache (path → `Arc<[u8]>`) so
  replaying the same sound effect no longer re-reads the file from disk on every `play()`;
  streaming BGM is unchanged.
- `DrawText::centered` + `TextAnchor` enum (`engine::TextAnchor`, `TopLeft` / `Center`): a
  draw position can now anchor at the measured text center, computed from the shaped buffer
  at render time with no manual `-width/2` math. Paired with `Camera::world_to_screen` (the
  inverse of `screen_to_world`) for placing screen-space text at a world position.
- `MouseButton` re-exported as `engine::MouseButton`, so games can import it from the crate
  root instead of reaching into `winit::event::`.
- `ReflectValue::I32` variant + `#[non_exhaustive]` on `ReflectValue`: integer fields are now
  inspectable in the egui Inspector alongside `F32`. `#[non_exhaustive]` means downstream
  exhaustive `match`es over `ReflectValue` must add a `_` arm.
- `ScriptingLimits` extended with `max_string_size`, `max_array_size`, `max_map_size`,
  `max_call_levels`, and `max_expr_depth`, all applied to the Rhai engine alongside the
  existing `max_operations`, with conservative defaults for trusted-local scripts.
- `spawn_scene_def` duplicate-`Tag` detection: duplicate tag keys are now first-wins with a
  `log::warn!` instead of silently overwriting. All entities still spawn; only parent-tag
  resolution is affected.
- `audio_fades` example (`examples/audio_fades.rs`): a small native demo confirming the new
  built-in `AudioSystem` drives fades in real play (Space to play, F to fade out, 1/2/3 to
  fade to a target volume) — previously the same sequence produced no audible change.
- `minimap` example gained a `WorldLabelSystem` that draws floating `"ENEMY"` nameplates
  above each enemy via `Camera::world_to_screen` + `DrawText::centered`, tracking them as the
  camera follows the player — the first live exercise of those two APIs with a moving camera.

### Breaking

- **`PhysicsWorld` is now a `World` resource**, not a field owned by `PhysicsSystem`.
  `PhysicsSystem::run` takes the resource out of the world, steps it, and re-inserts it, so
  game systems reach physics symmetrically with `world.resource_mut::<PhysicsWorld>()` —
  matching how `SpatialGrid` is exposed. Migration:

  ```rust
  // before
  let physics = PhysicsWorld::new();
  app.add_system(PhysicsSystem::new(physics, pixels_per_unit));

  // after
  app.world.insert_resource(PhysicsWorld::new());
  app.add_system(PhysicsSystem::new(pixels_per_unit));
  ```

- **All public color fields changed to `engine::Color`.** `Sprite`, `AtlasSprite`,
  `PointLight`, `AmbientLight`, `DrawRect`, `DrawText`, `DrawImage`, `ParticleEmitter`,
  `GpuParticleEmitter`, the `Timeline` color track, all UI widgets (`Button` / `CheckBox` /
  `Panel` / `ScrollView` / `Slider` / `TextInput` / `Label`), and `DebugDraw` previously held
  a mix of `[f32; 4]` / `[f32; 3]` / `[u8; 4]`. Color-accepting constructors and builders take
  `impl Into<Color>`, so call sites passing raw arrays still compile; only struct-literal
  `color:` initializers need updating (e.g. `color: [r, g, b, a]` → `color: Color::rgba(r, g,
  b, a)`). Raw arrays remain only at the GPU/glyphon boundaries via `to_array` / `to_u8` /
  `to_rgb`. Scene RON now serializes color as `(r:.., g:.., b:.., a:..)` struct form.

### Fixed

- **Per-frame hot-path costs removed.** `SpatialGrid` / `CollisionGridSystem` rebuild the
  world resource in place (remove → rebuild → insert) instead of deep-cloning two `HashMap`s
  into the resource every frame. `ScriptAsset.ast` is now `Arc<rhai::AST>` (clone = refcount
  bump, not a full AST deep-clone per scripted entity per frame), and `ScriptingSystem` reuses
  thread-local scratch buffers. A\* pathfinding (`find_path`) gained a closed set to prevent
  re-expanding stale heap entries and reuses its open list / score maps across calls (public
  signature unchanged). The sprite renderer opens a single render pass for the whole pre-sorted
  sprite stream and issues per-texture-run draws within it, instead of a new pass per batch.
- **`RenderLayer` negative values no longer fold to bit 0.** Layer→mask matching previously
  `clamp(0, 31)`-ed the layer index, so a `RenderLayer(-1)` background sprite mapped onto bit 0
  and leaked into `layer_mask: 1 << 0` offscreen passes. Layers outside `0..=31` cannot be
  addressed by a 32-bit mask and now simply never match under any non-zero mask (they still
  render under mask `0` = all layers); the engine warns once on an unmaskable layer.
- **Point-light radius falloff locked by a contract test.** A code-analysis pass flagged the
  CPU `radius * zoom / viewport_w` calculation as a possible unit mismatch; re-derivation
  confirmed it a false positive (the value is already in the shader's UV-fraction-of-width
  space and the falloff reaches zero at the world radius). A regression test now pins the
  correct behavior so a future "fix" cannot reintroduce a 2× error.

## 2.0.0

### Added

- `lit_dungeon_game` example (`examples/games/lit_dungeon/`): first playable-game use of 2D
  lighting (`PointLight`/`AmbientLight`) and `PostProcessConfig`. A dark top-down brazier-
  lighting puzzle with a decaying torch; bloom + vignette post-process (toggle with `P`).
- `blend_locomotion` example (`examples/blend_locomotion.rs`) + `gen_blend_sheet` asset generator:
  first use of `BlendTree1D` in a real interactive loop. A single speed parameter drives
  idle/walk/run clip blending; demonstrates the new true crossfade and the stranding fix below.
- `BlendUv { to, weight }` component (`engine::BlendUv`): written by `AnimationSystem` during a
  crossfade and read by the sprite renderer to cross-dissolve the two frames per-pixel.
- `ImeConfig { allowed: bool }` resource (`engine::ImeConfig`, default **off**): controls whether
  the window accepts IME text composition. Insert `ImeConfig { allowed: true }` before `App::run()`
  in apps that need text input. See the IME fix under Fixed.
- `crane_wrecking_ball` example (`examples/crane_wrecking_ball.rs`): first playable-example use of
  the physics **joint** API (`PhysicsWorld::add_revolute_joint` / `add_distance_joint`). A kinematic
  crane cart hangs a revolute-pinned arm with a distance-tethered wrecking ball; drive the cart to
  swing the ball and knock a block stack off its pedestal. The joint methods shipped with unit tests
  but had zero game/example coverage. Demonstrates the rotation-sync fix below.
- `security_camera` example (`examples/security_camera.rs`): first playable-game use of
  `RenderTarget` / `OffscreenCamera`. A stealth puzzle where a guard patrols a room that is
  **entirely offscreen** — its only view is a wall monitor (an `OffscreenCamera` renders the guard
  room into a `RenderTarget` that a `Sprite` samples). Read the guard's position on the monitor and
  cross the doorway when it is away from the door stripe; reach the exit to escape, get caught to
  reset (`R` replays, `Esc` quits). The existing `minimap`/`split_screen` demos exercised the API but
  only ever framed the *same* region the main camera shows; this is the first use of an offscreen
  camera as the sole view of a **disjoint** region. Demonstrates the offscreen-render fix below.
- `timeline_cutscene` example (`examples/timeline_cutscene.rs`): first use of `Timeline` in a
  playable scene. Walk into a rune to trigger a cutscene that pans/zooms the camera, slides two gate
  panels apart, and fades a full-screen overlay — all driven by `Timeline` keyframe tracks; Space
  skips, control returns when it ends, then you cross the now-open gate to the exit. `Timeline`
  shipped with unit tests but had zero example/game coverage. Demonstrates the camera-drive addition
  below.
- `CameraTarget` marker component (`engine::CameraTarget`) + a `Timeline::zoom` track: a `Timeline`
  on an entity tagged `CameraTarget` drives the `Camera` resource (its `position` track → camera
  position, `zoom` track → camera zoom) as a virtual camera rig, instead of the entity's own
  `Transform`/`Sprite`. Lets a `Timeline` author camera moves for cutscenes — previously `Timeline`
  could only animate an entity's own transform/sprite. Additive: ordinary timelines are unaffected
  (the `zoom` track is empty by default).

### Breaking

- `Entity` is now an opaque generation-checked handle with `index()`, `generation()`, and
  `from_raw_parts(index, generation)`. Direct `entity.0` access is removed.
- `World::clone_entity(src)` now returns `Option<Entity>` and returns `None` for stale or
  despawned handles.
- Rhai scripting now uses `despawn_entity(index, generation)` instead of index-only
  `despawn_entity(id)`.
- Rhai scripting exposes `entity_index()` and `entity_generation()` for the current
  script runner entity.
- Removed the misleading public `Sprite.normal_texture` and `Sprite.normal_handle` fields.
  v2 keeps flat-normal lighting internally but does not expose per-sprite normal maps.

### Fixed

- Post-process shader (`post_process.wgsl`) declares the bloom tap-offset array as `var` instead
  of `let`, fixing a naga validation error ("may only be indexed by a constant") that panicked on
  shader creation whenever `PostProcessConfig.enabled` was `true`. Surfaced by the new
  `lit_dungeon_game` — the first runtime use of post-processing (CI compiles but never runs the
  windowed app).
- 2D lighting now projects `PointLight` positions with the **logical** viewport size (matching
  the sprite pass) instead of the physical surface size. On HiDPI/Retina displays (scale > 1)
  lights previously drifted from their sprites and rendered at half radius; on scale-1.0 displays
  it happened to line up, which is why it went unnoticed. Also surfaced by `lit_dungeon_game`.
- Screen-space text (`TextQueue`/`DrawText`) now renders **after** the post-process and lighting
  passes, so HUD/overlay text is no longer dimmed by world lighting (or warped by post effects).
  Trade-off: `DrawText` is no longer affected by `PostProcessConfig`; route text through egui if
  you want it post-processed. Surfaced by `lit_dungeon_game`.
- Post-processing and lighting now compose as `scene -> post -> lighting -> final` when both
  effects are active.
- Lighting intermediate targets are recreated after viewport resize, and `PointLight`
  positions now respect camera position and zoom.
- Scene replacement restores the same core engine resources as initial app creation, including
  panic recovery state, and preserves initialized `DebugUi`.
- Images loaded directly through `AssetServer::load_image` are lazily uploaded to the GPU cache,
  so scene-owned loading no longer depends on `App::load_image`.
- `BlendTreeSystem` no longer strands an entity on an intermediate clip when the blend parameter
  crosses two thresholds (e.g. idle→walk→run) within a single crossfade: it now defers the new
  transition instead of recording an unachieved target, and re-evaluates once the crossfade ends.
  Surfaced by `blend_locomotion`; regression test in `src/animation/blend_system.rs`.
- Game key input is no longer broken when a CJK IME (Korean/Japanese/Chinese) is active. The window
  previously enabled IME unconditionally, so on macOS the OS could route key-release events into IME
  composition and leave keys stuck "pressed" (e.g. a held movement key never released → the
  character kept moving). IME is now **off by default** and opt-in via the new `ImeConfig` resource;
  only text-input apps (`settings_menu_game`) enable it. Surfaced by `blend_locomotion` (a held
  accelerate key stayed latched under a Korean IME, so the clip never returned to idle).
- `PhysicsSystem` now syncs each body's **rotation** into `Transform.rotation`, not just its
  position. Previously a body that rotated under physics (e.g. a joint-driven swinging arm) kept a
  bolt-upright sprite because rotation was silently dropped. Rotation-locked bodies (`lock_rotation:
  true`) are unaffected (their angle is always 0). Surfaced by `crane_wrecking_ball`; regression
  tests in `src/physics/system.rs`. Behaviorally inert for consumers that own a raw `PhysicsWorld`
  and sync transforms themselves (e.g. `rust-survivors`).
- Offscreen render targets (`OffscreenCamera` → `RenderTarget`) now render with their **own** camera
  instead of the main camera. The sprite renderer's camera uniform is a single shared buffer updated
  via `queue.write_buffer`; the offscreen pass and the main pass were recorded into one command
  submission, and within a single submit only the **last** write to that buffer takes effect — so
  every offscreen target was drawn with the (later-written) main camera's view. The
  `minimap`/`split_screen` demos masked this because their offscreen content overlaps the main view;
  it became obvious only with an offscreen camera framing a *disjoint* region (the monitor rendered
  the main scene instead of the guard room). Each offscreen target now submits in its own command
  buffer so its camera write pairs with its own draws. Surfaced by `security_camera`; GPU-validated
  by a native run (CI compiles but cannot run the windowed app).
- `split_screen` example no longer crashes with a wgpu validation error on its second frame. It used
  `layer_mask: 0` (render all layers), so its render-target *display* sprites were drawn into the
  same targets they sample — a self-capture (a texture used as both color attachment and sampled
  resource within one render pass). It "survived" only frame 1, before the targets were registered.
  Fixed by masking the display sprites out of the offscreen pass (`layer_mask: 1 << 0`), the same
  self-capture-avoidance `minimap` already uses.

### Changed

- `SceneDef` schema version is now `2`; old v1 files with removed normal-map sprite fields are
  accepted and those fields are ignored.
- Agent instructions now define this repository as the default and only verification scope unless
  the user explicitly asks for external project checks.
- Lighting now renders the **nearest 16** point lights to the camera when a scene exceeds the
  16-light hard cap (previously the first 16 in arbitrary query order), and warns once. Light
  occlusion/shadows and per-sprite normal maps remain out of scope; lighting stays native-only.
- Animation crossfades are now a true **2-UV shader-lerp** cross-dissolve (`mix(from, to, weight)`
  in `sprite.wgsl`) instead of a 50% hard frame-swap, and `BlendWeight` is finally consumed by the
  renderer (via the new `BlendUv` component). Additive: a sprite that is not crossfading
  (`weight = 0`) renders byte-identically to before. `InstanceRaw` gained internal `to_uv`/`blend`
  fields; the sprite path stays cross-platform (the blend works on wasm too).

## 1.3.0

### Added

- `TextInput` single-line **horizontal scrolling**: long values no longer wrap or clip out of view.
  The field renders as one non-wrapping line and scrolls so the caret stays visible while typing or
  navigating (`Home`/`End`/arrows); an unfocused field anchors to the start. New `DrawText`
  opt-in `with_single_line_caret(caret_byte)` drives it — the renderer measures the caret x via
  glyphon `Buffer::layout_runs()` and shifts the `TextArea` left, clipped to the field by
  `TextBounds` (no new render pipeline).
- `TextInput::remaining_capacity()` and `TextInput::caret_display_offset()` helpers.

### Fixed

- IME at `max_len`: composing input when the field is full no longer shows a phantom, uncommittable
  preedit. `UiSystem` only displays the IME preedit while it still fits in the remaining capacity
  (`remaining_capacity() >= preedit.len()`); commits already truncate to fit.

### Example

- `settings_menu_game` Settings scene gained a dedicated narrow long-text field (prefilled past its
  width, `max_len` 48) that exercises horizontal scroll, caret-follow, and IME-at-capacity.

## 1.2.1

### Fixed

- macOS live window-**resize** drag froze the content: while the OS runs its modal resize loop
  the normal `about_to_wait → request_redraw → RedrawRequested` cadence is parked, and the
  `Resized` handler only reconfigured the surface without drawing. The frame step (update +
  render) is now factored into `App::step_frame` and is also driven inline from `Resized`, so
  animations keep advancing while the window is being resized.

### Added

- `Window::pre_present_notify()` is now called immediately before `surface.present()`, the
  winit-recommended compositor hint that trims presentation latency.
- `settings_menu_game` gained a small always-animating spinner (bottom-left, `dt`-driven, no
  input) so a window-drag freeze is visible by eye — it stalls during a drag and resumes after.
- Debug instrumentation: `step_frame` logs `frame gap <ms>` at `debug` level when the
  inter-frame gap exceeds ~33 ms, to quantify drag/stall (e.g. `RUST_LOG=engine=debug`). The
  `settings_menu_game` example now initializes `env_logger` (native-only dev-dependency) so the
  `log` output is actually visible — previously no log backend was installed.

### Known limitations

- A one-frame lag remains at the **start** of a live drag (both resize and titlebar move): the
  window content tracks the cursor a beat late on the first movement, then follows normally for
  the rest of the drag. The hard freeze is gone — content keeps animating throughout both drag
  kinds on the tested macOS (15.x / Darwin 25) — but this residual start-of-drag latency is a
  macOS/winit present-timing artifact left as a documented limitation per the "known levers"
  scope (deeper fixes — background redraw thread / native Cocoa hooks — were out of scope).

## 1.2.0

### Added

- `LocalizedText` component plus `LocalizationSystem` — bind a translation key to a `Label`,
  `Button`, or `CheckBox` and the system keeps its text in sync with the current locale every
  frame. Switching language is now just `LocaleResource::set_locale(..)`; the whole UI
  retranslates with no manual per-widget rebuild. Re-exported from the crate root.
- `settings_menu_game` example (`examples/games/settings_menu/`) — a Title → Settings → Dialogue
  slice that is the first playable-game coverage for the UI-depth + localization + audio-bus
  surface: `TextInput`, `Slider`, `CheckBox`, `ScrollView`, `Panel`/`LayoutSystem`, rich/multiline
  `Label`, `LocaleResource` (EN/KO/ES) + `LocalizedText`, and `AudioManager` buses + `AudioEffect`
  low-pass. Cross-scene `Settings`/locale/`AudioManager` survive `SceneCmd::Replace` via
  `App::register_persistent`.

### Fixed

- Clicks landing on the wrong widget after a mouse move: `InputState` keeps only the latest
  cursor, so when a press and a following move collapsed into one frame the click was hit-tested
  at the moved-to position (e.g. pressing empty space then moving onto a button activated it,
  while pressing a button then moving off did nothing). `InputState` now records the cursor at the
  press and release moments (`mouse_press_cursor`/`mouse_release_cursor`), and `UiSystem` hit-tests
  clicks/toggles/drag-starts against those (hover and drag-tracking still use the live cursor).
- `TextInput` caret rendering: the caret `|` was always appended at the end of the string, so it
  never matched the real cursor after navigation and text appeared to be inserted "in the middle".
  Added `TextInput::display_with_caret` which inserts the caret (and IME preedit) at the byte
  cursor; `UiSystem` uses it. The caret blinks while focused but its slot is always reserved (a
  space when off, `|` when on) so blinking no longer shifts the trailing text.
- Input-to-display latency: `desired_maximum_frame_latency` lowered from 2 to 1 (vsync kept, no
  tearing) so button/drag feedback lands a frame sooner.
- IME / non-Latin input: `set_ime_allowed(true)` is now called on the window, so macOS (and other
  platforms) compose CJK input and deliver it via `Ime::Commit`. Previously IME was never enabled,
  so Korean arrived as separated jamo (per-keystroke `Character` events).
- `AudioManager::play_tone` now applies the channel's effective bus volume to the sink and the
  channel `AudioEffect` (low-pass / pitch / fade-in), matching file playback. Previously tones
  ignored both, so `set_bus_volume` and `set_effect` had no audible effect on tone channels.
- Interactive responsiveness: the event loop never set a `ControlFlow`, defaulting to `Wait`, so
  drags/hover updated a beat late and sliders did not track the cursor smoothly. It now runs with
  `ControlFlow::Poll` for a continuous per-frame loop (vsync-paced via the existing redraw request).
- `TextInput` cursor editing: added `move_left`/`move_right`/`move_home`/`move_end`/`delete_forward`
  (UTF-8 safe) on `TextInput`, and `UiSystem` now applies ←/→/Home/End/Delete to the focused field.
  Previously the caret could only sit where typing left it (no navigation, no forward delete).
- HiDPI mouse/touch hit-testing: the cursor was stored in physical pixels while UI hit-testing,
  `ViewportSize`, and `Camera::screen_to_world` all work in logical pixels, so on a scaled display
  (e.g. Retina 2×) clicks landed offset from the cursor. `CursorMoved` and the touch→mouse
  emulation now divide by the window scale factor, storing the cursor in logical coordinates
  (no-op at scale 1.0). Surfaced by `settings_menu_game`'s click-heavy widgets; also corrects
  editor gizmo dragging and any `screen_to_world` use on HiDPI.

### Known gaps (surfaced, not yet addressed)

- `LocaleData.font` is not applied at runtime: `TextRenderer` takes its font once at init via the
  `FontData` resource, so per-locale font switching is unsupported. Non-Latin scripts render only
  through native system-font fallback and are absent on wasm (no system fonts). Korean in
  `settings_menu_game` therefore renders on macOS but not on Linux CI / wasm.
- `LocaleData.direction` / `TextDirection::RightToLeft` is metadata only — the text renderer does
  not auto-apply RTL alignment from the locale (it maps `TextAlign::Right` explicitly). No RTL
  locale ships in the example, so RTL is left for a future dedicated example.
- No window fullscreen-request path exists yet, so `settings_menu_game`'s Fullscreen checkbox only
  stores a preference (its label says so); wiring real OS fullscreen is deferred.
- The built-in `TextInput` is single-line with no horizontal scrolling: text longer than the field
  width clips at the edge, and IME composition at the `max_len` cap shows an uncommittable preedit.
  Adequate for short fields (names, search); a scrolling multi-line field is future work.
- The blinking `TextInput` caret is drawn inline (a reserved `|`/space slot), so it can still shift
  the trailing text by a sub-pixel on blink. A fully stable caret needs a renderer-measured overlay
  (the text renderer drawing the caret quad at the glyph position); deferred.
- Residual input-to-display latency on macOS: even with `frame_latency=1`, a click registers a beat
  late, and the window content lags during a live OS window drag (winit enters a modal event-loop
  mode). `AutoNoVsync` only helped marginally while uncapping the frame rate, so it was not adopted.
  Treated as a macOS/winit optimization to revisit.

## 1.1.0

### Added

- `BlackboardValue::Path(Vec<IVec2>)` plus `Blackboard::set_path`/`get_path` so behavior
  trees can cache a whole A* path instead of recomputing every tick. `BlackboardValue` is
  now `#[non_exhaustive]`. Validated by `maze_escape_game`, whose enemies now cache the path
  and only re-run `find_path` when the player's goal tile changes.
- `App::register_persistent::<T>()` plus `World::take_resource_erased`/`insert_resource_erased`
  to preserve chosen resources across the `World` reset that `SceneCmd::Replace` triggers.
  `scene_flow_game` uses it to drop its `Arc<Mutex<_>>` cross-scene state workaround.
- `PhysicsWorld::add_static_from_tilemap(tilemap, ppu, collider_for)` and the `TileCollider`
  descriptor (`solid` / `solid_with` / `one_way`) to generate one static collider per solid
  tile, aligned to `TilemapSystem`'s tile coordinates. `platformer_game`'s level is now a
  single `Tilemap` that drives both rendering and collision; its seamless tileset is
  reproducible via `examples/gen_platform_tiles.rs` (the original `tiles.png` is a set of
  discrete object sprites with transparent margins, not a seamless tileset).
- One-way platforms: `PhysicsWorld::set_one_way`/`is_one_way` and
  `CharacterController::request_drop`/`is_dropping`. `move_character` now passes through
  one-way colliders when ascending or dropping and only lands on them from above.
  `platformer_game` adds a one-way platform and an S/Down drop-through key.

### Added (pre-1.1 carryover)

- 2D cutout (rigged) skeletal animation in `src/skeletal.rs`: `SkeletalAnimator`,
  `SkeletalClip`, `BoneTrack`, `BoneKeyframe`, `SkeletalAnimationSystem`, and the
  `SkeletonBuilder` authoring helper. Bones are hierarchy entities whose local
  `Transform` is keyframed; the existing `HierarchySystem` and sprite renderer draw them
  with no renderer changes. See `docs/SKELETAL.md` and `examples/skeletal_puppet.rs`.
- Re-exported `AssetId`, `SaveKey`, `save_with_key`, and `load_with_key` from the crate root so public examples match the stable API surface.
- Added `ScheduleErrorPolicy` and `SystemPanicPolicy` so apps can opt into stricter schedule-cycle and system-panic behavior while keeping the existing fallback defaults.
- Added `examples/runtime_policies.rs` to show strict runtime policy configuration without opening a long-running window.
- Added `World::mark_changed<T>()` and `World::get_mut_tracked<T>()` for explicit ECS change tracking after direct component mutation.
- Added native `AudioChannelState` plus `AudioManager::playback_state`, `is_finished`, and `is_playing` so games can advance non-looping playlists when a channel naturally drains.
- Added `docs/ENTITY_GENERATION_V2_PLAN.md` to lock the v2 design for generation-checked entity handles.

### Changed

- `HierarchySystem` now propagates `GlobalTransform` in topological (root→child) order in a
  single pass, supporting arbitrary hierarchy depth. It previously ran a fixed 2-pass loop
  capped at depth 3 — a limit surfaced by deep skeletal bone chains.
- Aligned save encryption and async asset examples in the public reference with the current source.
- Native `AssetServer` cache keys now canonicalize existing file paths, reducing duplicate handles and hot-reload misses caused by mixed relative/absolute paths. Missing paths and WASM URLs keep their existing string behavior.
- Sprite renderer file texture cache lookups now accept both the original requested path and the canonical `AssetServer` handle path, so `Sprite::textured_with_handle(...)`, `DrawImage::textured_with_handle(...)`, and atlas textures no longer fall back to white when images are loaded through relative paths.
- Native audio decoding now enables MP3 in addition to WAV and Vorbis/OGG.
- `PhysicsSystem` now documents the physics-unit to pixel-unit boundary and defensively clamps invalid `pixels_per_unit` values in release builds while asserting in debug builds.
- Clarified that Rhai scripting is intended for trusted local game code, not hostile sandboxing, and documented the limits of temporary script spawn IDs.
- **Breaking rendering behavior fix:** fixed the default sprite quad UV orientation so `Sprite`, `DrawImage`, `AtlasSprite`,
  `UvRect::FULL`, `UvRect::from_grid(...)`, and `UvRect::from_pixels(...)` render
  normal top-left-origin PNGs upright without requiring `UvRect::flipped_y()`.
  Existing game-side `.flipped_y()` orientation workarounds should be removed after
  updating the engine.

### Fixed

- Restored the `wasm32-unknown-unknown` build: the WebSocket `wasm_impl` module called
  `push_event_bounded` unqualified without importing it, breaking the wasm target while the
  native build was unaffected. The function is now imported into the module scope.
- Removed the redundant manual `unsafe impl Send/Sync for BehaviorTree`. The
  `BehaviorNode: Send + Sync` trait bound already guarantees both, so the hand-written impl
  was unnecessary and would have silently masked unsoundness if that bound were ever relaxed.

## [1.0.0] - 2026-05-27

### Added

- Stable `skeleton-engine` package metadata with library crate name `engine`.
- Rust 1.88 minimum supported Rust version declaration.
- README, MIT license, changelog, and beginner `examples/basic.rs`.
- CI gates for formatting, clippy, full native tests, release build, WASM build, rustdoc warnings, `cargo package`, and `cargo publish --dry-run`.

### Changed

- Documented release package hygiene with an explicit crates.io include list.
- Updated public documentation examples for current `OffscreenCamera`, `Sprite`, `TouchState`, and `glam::Vec2` usage.
