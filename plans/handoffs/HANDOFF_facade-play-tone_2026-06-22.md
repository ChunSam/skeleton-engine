# Cross-platform `play_tone` on the `Audio` facade + first game adoption (v0.52.0)

**Date:** 2026-06-22
**Status:** COMPLETED + merged. main @ `ca5080d`, package **v0.52.0**, clean tree, full gate green, CI green, squash-merged (#199).
**Bead(s):** none (bd unavailable in this environment)
**Epic:** post-audit feature work (VISION feature+example loop)
**Chain:** `standalone-4365aa4a` seq `7`
**Parent:** `HANDOFF_audio-facade_2026-06-22.md` (seq 6)
**Prior chain:** `HANDOFF_engine-audit-fixes_2026-06-22.md` (seq 1) > `HANDOFF_audit-followup-refactors_2026-06-22.md` (seq 2) > `HANDOFF_audit-deferred-editor-tier5_2026-06-22.md` (seq 3) > `HANDOFF_audit-item6-texture-format_2026-06-22.md` (seq 4) > `HANDOFF_audit-item7-unwrap-hardening_2026-06-22.md` (seq 5) > `HANDOFF_audio-facade_2026-06-22.md` (seq 6) > this (seq 7)
**Auto:** false

> NB the memory `engine-current-state` uses a *different* (engine-wide) seq counter: this same work is **seq 70** there, the seq-6 facade was **seq 69**. The handoff chain counter (seq 7) and the memory counter (seq 70) both refer to this session.

---

## Stale References

None. The parent (seq 6) introduced `Audio`, `AudioFacadeSystem`, `AudioManager::play_bytes`/`crossfade_bytes`, `append_decoded`/`begin_crossfade`, `sfx_voice_channel`/`next_sfx_channel`, `MASTER_BUS`/`MUSIC_CHANNEL`/`SFX_VOICES` — all still present and were **built on** this session (the new `play_tone` reuses the voice ring + `MASTER_BUS`). No identifier was renamed or removed.

## Since Last Handoff (seq-6 plan vs reality)

The parent's "Where We're Going" listed four carried directions; this session executed **#4 "Adopt the facade in a game"** — but with a twist the parent didn't foresee.

- **Direction picked = "adopt the facade in `settings_menu`" (the parent's strong-VISION-fit option).** The user chose it via AskUserQuestion (board empty, no EW request).
- **The premise broke on contact.** Exploration found that **all three audio example games (`settings_menu`, `survivor`, `shooter`) build their audio on `play_tone`** — procedural tone synthesis, no asset files — which seq 6 had **deliberately excluded** from the facade ("native-only synthesis"). So a direct "delete the cfg-guards" adoption was *impossible*: the guards wrap exactly the facade-excluded calls.
- **Reframe (the real VISION move).** Web Audio HAS oscillator synthesis (`OscillatorNode`), so `play_tone` IS cross-platformable — the seq-6 exclusion was conservative. Per VISION ("if the example reveals the API is awkward, fix the API first"), this session **grew the facade with `play_tone`/`play_tone_on_bus`**, then adopted it in `survivor`+`shooter`. The user picked this path over byte-banking the tones.
- **`settings_menu` was NOT adopted** — it additionally uses `AudioEffect` low-pass + sustained channels + `is_playing`, all still facade-out-of-scope. Deferred (a future Web Audio `BiquadFilter` pass could enable it).
- **Carried directions still open** (untouched this session): facade positional audio, `RonRegistry<V>` pub at crate root, HDR/linear render-target.

## Reference Documents

- `CLAUDE.md` — module map (the audio row now documents facade `play_tone`), verify rules, pre-1.0 versioning.
- `docs/VISION.md` — the feature+example loop; this session is a textbook instance (example revealed missing primitive → grew the API → adopted).
- `../dungeon-merchant/docs/engine-wishlist.md` — wishlist board (ACTIVE empty, next ID EW-002). Read FIRST each session.
- Memory `engine-current-state` (now seq 70) + `MEMORY.md` index.

## The Goal

The seq-6 `Audio` facade let a dual-target game write `play_sfx`/`play_music`/bus calls with zero `cfg` guards — but it **excluded `play_tone`** (procedural tone synthesis), and that is exactly what the engine's audio example games actually use (no asset files, just blips/beeps). So no game could adopt the facade. The goal this session: **make `play_tone` cross-platform on the facade** (native rodio synthesis it already had; wasm via a Web Audio `OscillatorNode`), then **adopt the facade in `survivor` and `shooter`** — deleting their hand-written native-vs-wasm audio split and, as a bonus, giving them audio on the web for the first time. The adopted games are the acceptance test (VISION).

## Where We Are

- **main @ `ca5080d`, package v0.52.0, CLAUDE.md header v1.6.121, clean tree, `./scripts/verify.sh` → exit 0** (fmt + clippy `-D warnings` native + wasm32 lib/bins build + `test --all-targets` + doc `-D warnings`), run twice (post-edit, post-version-bump).
- **PR #199 squash-merged** on green CI (Test native 4m48s · Build WASM 1m45s · Package dry-run 1m3s · Rustdoc 40s), branch `feat/audio-facade-tone` deleted, local main fast-forwarded.
- **`Audio::play_tone(freq, dur, vol)` + `Audio::play_tone_on_bus(freq, dur, vol, bus)`** added (`src/audio_facade.rs`). Native: `next_sfx_channel()` (the shared 16-voice ring) → `assign_bus(channel, MASTER_BUS | bus)` → `AudioManager::play_tone(channel, ...)`. Wasm: `WebAudio::play_tone`/`play_tone_on_bus`.
- **`WebAudio::play_tone` + `play_tone_on_bus` + private `play_tone_to`** added (`src/audio_wasm.rs`). `play_tone_to` wires `OscillatorNode → gain → dest` (dest = `bus_input(bus)` or `master`), schedules a 0→`vol`→0 attack/release envelope on the gain (`edge = (dur*0.25).min(0.008)`), `osc.start()` + `osc.stop_with_when(now+dur)`. `#[allow(deprecated)]` (the `start`/`stop*` bindings are deprecated, like `stop_at`).
- **`Cargo.toml`** — `web-sys` features gained `"OscillatorNode"`, `"OscillatorType"` (were absent; the only Cargo change).
- **`examples/audio_facade/audio_facade.rs`** — `T` key → `play_tone(330.0, 0.15, 0.3)`; legend + doc-comment + the 15-element key-edge tuple updated. (Hearable on web via the example's existing `web/` harness.)
- **`examples/games/shooter/shooter.rs`** — adopts the facade: **5→0** `#[cfg(wasm32)]` audio guards. `use engine::AudioManager` (gated) deleted; `Audio, AudioFacadeSystem` added to the un-gated import. `play_tone` helper is one un-gated fn calling `play_tone_on_bus(.., "sfx")`. Init uses `Audio::new()` + `app.add_system(AudioFacadeSystem)`. Call sites dropped the channel arg.
- **`examples/games/survivor/survivor.rs`** — same adoption; audio guards gone. **8→4** total cfg-guards; the 4 remaining are all `GpuParticleEmitter` (the GPU thruster), genuinely native-only and untouched.
- **Both games compile for wasm** with the un-gated facade (`cargo build --example survivor_game --example shooter_game --target wasm32-unknown-unknown` → exit 0) and now play sfx on the web build.
- **Native real-play smoke clean** — window launched, `play_music`/`play_sfx`/`play_tone` decoded via rodio with no panic, empty stderr, on-screen readout confirmed `play_tone (330 Hz, synthesized)`.
- **Docs/paperwork:** CLAUDE.md audio row (3 edits) + header bump; CHANGELOG 0.52.0 entry (Added/Changed/Notes); Cargo.lock → 0.52.0.
- **Memory** `engine-current-state` bumped to seq 70 (frontmatter description, body lead pointers, new seq-70 bullet); `MEMORY.md` index line refreshed.

## What We Tried (Chronological)

1. **Onboarding (read-only).** Read the full seq-6 parent, the wishlist board (`_None open._`, next ID EW-002 → no EW preempts), ran `./scripts/verify.sh` (baseline exit 0, read `VERIFY_EXIT` from the log), read memory `engine-current-state` (seq 69). Narrated the 5-step onboarding in Korean. main was @ `b8e1712` (the seq-6 handoff docs PR #198).
2. **Asked direction (no EW request).** `AskUserQuestion` → user chose **"파사드를 게임에 적용 (settings_menu)"** over facade-positional / RonRegistry-pub / HDR-RT.
3. **Explored `settings_menu` — premise broke.** `grep` + read: its audio is entirely `play_tone` (`blip`, `play_bgm` two-tone) + an `AudioEffect` low-pass (`set_effect`/`clear_effect`) + `is_playing` (keep-alive). Every one of those is facade-excluded. Only `set_bus_volume`/`bus_volume` are coverable.
4. **Checked the other two games + the whole tree.** `survivor` (8 guards) and `shooter` (5 guards) **also** do all audio via `play_tone` (fire/boom). **No game uses bytes-based audio** (the `.play()` grep hits were ParticleEmitter/AnimationPlayer + wasm-bindgen media, not engine audio). Confirmed `WebAudio` has NO tone synthesis (only `play(bytes)`/`play_sfx(bytes)`). `AudioManager::play_tone` exists (`src/audio/playback.rs:135`).
5. **Surfaced the finding + asked how to proceed.** `AskUserQuestion` (4 options: extend facade with play_tone / byte-bank the tones / settings_menu-limited / pivot direction) → user chose **"파사드에 play_tone 추가 → survivor+shooter 채택"**.
6. **Explored the backends to design the impl.** Read native `play_tone` (channel-based, `stop_immediate` reuse, applies bus vol via `effective_volume` + channel effects). Read `WebAudio` graph: `bus_entry`/`bus_input` (`duck→volume→master`), `play_sfx_to` (`source→panner→gain→dest`), `ramp_gain_to`/`stop_at`. **Key discovery:** `web-sys` features (Cargo.toml L212–219) lacked `OscillatorNode`/`OscillatorType`. **Also:** `survivor`/`shooter` route their tones to a **"sfx" bus** (vol 0.6) at init → the facade needs a `play_tone_on_bus` variant, not just `play_tone`.
7. **Presented the plan, got "go".** Branched `feat/audio-facade-tone` off main (`b8e1712`).
8. **Implemented (engine).** Cargo.toml features → `WebAudio::play_tone`/`play_tone_on_bus`/`play_tone_to` (OscillatorNode+envelope) → `Audio::play_tone`/`play_tone_on_bus` (ring voice + assign bus + native `play_tone`; wasm pass-through) → facade module docs (play_tone added to coverage, removed from the "native-only extras" exclusion).
9. **Implemented (example + games).** `audio_facade` `T` key → `survivor` adoption (import move, helper un-gate, init, 3 call sites) → `shooter` adoption (import delete, helper un-gate, init, 3 call sites).
10. **First verify → `VERIFY_EXIT=1`** (the known trap). `cargo fmt --check` failed: rustfmt wanted to expand the hand-wrapped 15-element key-edge tuple and the `if ... ||` connect chain. Ran `cargo fmt`, re-ran → **`VERIFY_EXIT=0`** (full gate green; the wasm lib build compiled the new `OscillatorNode` branch).
11. **Extra wasm verification.** `cargo build --example survivor_game --example shooter_game --target wasm32-unknown-unknown` → exit 0 (the standard gate doesn't wasm-build examples; this confirms the un-gated facade keeps the games wasm-buildable).
12. **Counted the dup deletion.** shooter 5→0; survivor 8→4 (remaining 4 = GpuParticleEmitter).
13. **CLAUDE.md audio row updated** (3 targeted edits: WebAudio gains play_tone; facade now covers tones + excludes only play_at/effects; survivor/shooter adopt).
14. **Native real-play smoke.** Built `audio_facade` debug binary, launched under `caffeinate` (background), `osascript` key codes M=46 / 1=18 / T=17 (×2), `screencapture`. Process alive after real rodio decode, stderr 0 bytes, readout `last: play_tone (330 Hz, synthesized)`. Killed (exit 144 = my own SIGTERM, expected).
15. **`/ship` (4 edits + lock + re-verify).** Cargo.toml 0.51.0→0.52.0, `cargo update -p skeleton-engine` (lock→0.52.0), CHANGELOG 0.52.0 (Added/Changed/Notes), CLAUDE.md header v1.6.120→v1.6.121 + package 0.51.0→0.52.0. Re-ran the full gate → exit 0.
16. **`/land-pr`.** commit `60826da`, push, `gh pr create` (#199), `gh pr checks 199 --watch --fail-fast --interval 30` (background, exit 0), confirmed `mergeStateStatus: CLEAN`, `gh pr merge --squash --delete-branch` (merge `ca5080d`), `git pull --ff-only`, bumped memory to seq 70.

## Key Decisions

- **`play_tone` IS cross-platform — overturn the seq-6 exclusion.** Seq 6 excluded tone synthesis as "native-only", but Web Audio `OscillatorNode` is trivial tone synthesis. The honest VISION fix is to add it to the facade, not to work around it in the games. Rejected: byte-banking the procedural tones (generate WAV via `sine_wav` at startup + `play_sfx`) — keeps the facade as-is but changes the games more invasively and doesn't grow the facade to cover what real games use.
- **Two methods: `play_tone` (master) + `play_tone_on_bus` (named bus).** Mirrors `play_sfx`/`play_sfx_on_bus`. Needed because `survivor`/`shooter` route sfx to a "sfx" bus (vol 0.6) — without the `_on_bus` variant the games couldn't preserve their bus-volume mixing.
- **Native `play_tone` reuses the shared `play_sfx` 16-voice ring.** Fire-and-forget, rides `MASTER_BUS` (or the named bus). Rejected: a fixed per-tone channel (would need the game to name channels, which the facade has no concept of). **Consequence (accepted, documented):** consecutive tones now **overlap** instead of cutting (was `stop_immediate` on the named channel). For short one-shot sfx (fire 0.04s, boom ≤0.3s) this is an improvement (fuller rapid fire).
- **Drop the channel argument from the games' `play_tone` helper.** The local helper went from `play_tone(world, channel, freq, dur, vol)` to `play_tone(world, freq, dur, vol)` — the channel ("fire"/"boom") is meaningless once the facade round-robins voices. 3 call sites per game updated. Honest API over minimal diff.
- **Web tone envelope: a short attack/release ramp.** `edge = (dur*0.25).min(0.008)` (≤8 ms, or a quarter of very short tones). A hard 0→vol step on an oscillator clicks; the ramp avoids it. Held flat between the two edges, ramped to 0 at `now+dur`, oscillator stopped then. Mirrors the file's existing `ramp_gain_to` audio-clock scheduling style.
- **`settings_menu` NOT adopted.** It needs `AudioEffect` low-pass (`set_effect`/`clear_effect`) + sustained two-tone bgm on named channels + `is_playing` keep-alive — none on the facade. Adopting it would gut its low-pass demo (the example's whole point) or leave most guards in place. Left native; flagged as a future Web Audio `BiquadFilter` pass.
- **`resume()` already covered the web gate.** The games' init didn't need a resume primitive added — `AudioFacadeSystem` ticks `update` (native fades/ducks; web no-op) and the games' own input already triggers playback after a gesture. (Note: the games don't call `resume()` explicitly; on web the first sfx after a gesture works because the keypress IS the gesture. A purist might add a `resume()` on first input — left as-is, low risk.)
- **MINOR bump v0.52.0** (pre-1.0: additive feature = MINOR). New public API, existing call sites byte-identical.
- **No web harness for `survivor`/`shooter` this session.** Out of scope (`/ship-wasm-example` is a separate, larger pass each). The `audio_facade` example already has a web harness and now demonstrates `play_tone` there, so the new API IS web-verifiable by ear without new harnesses.

## Evidence & Data

**cfg-guard reduction (the dup deletion — `grep -c 'cfg(.*wasm32'`):**

| Game | Before | After | Removed | Remaining are |
|---|---:|---:|---:|---|
| `shooter` | 5 | **0** | 5 | — (all were audio) |
| `survivor` | 8 | **4** | 4 | `GpuParticleEmitter` (GPU thruster, native-only) |
| `settings_menu` | 16 | 16 | 0 | not adopted (facade-out-of-scope audio) |

**The games' audio API — why settings_menu/survivor/shooter couldn't adopt seq-6's facade:**

| Game | Audio API used | Facade-coverable before seq 7? |
|---|---|---|
| `settings_menu` | `play_tone` (blip/bgm) + `AudioEffect` low-pass + `is_playing` + `set_bus_volume` | ❌ only bus volume |
| `survivor` | `play_tone` (fire/boom) + `set_bus_volume` (init) | ❌ `play_tone` excluded |
| `shooter` | `play_tone` (fire/boom) + `set_bus_volume` (init) | ❌ `play_tone` excluded |

**Verify-gate runs (read the real exit from the LOG, not the runner summary):**

| Run | Result | Cause / note |
|---|---|---|
| post-edit #1 | `VERIFY_EXIT=1` | `cargo fmt --check`: rustfmt reflowed the hand-wrapped 15-tuple let + the `if ... \|\|` connect chain |
| post-fmt #2 | `VERIFY_EXIT=0` | full gate green; wasm lib build compiled the new `OscillatorNode` branch |
| extra wasm-example | exit 0 | `cargo build --example survivor_game --example shooter_game --target wasm32` |
| post-version-bump | `VERIFY_EXIT=0` | green after the 0.52.0 paperwork |

**CI on PR #199 (all pass):**

| Check | Time |
|---|---|
| Test (native) | 4m48s |
| Build (WASM) | 1m45s |
| Package dry-run | 1m3s |
| Rustdoc | 40s |

**Native real-play smoke (synthetic input via `osascript` key codes):**
- Keys: `46` (M, `play_music` 110 Hz loop), `18` (1, `play_sfx` 440 Hz), `17` (T, `play_tone` 330 Hz) ×2.
- Result: PID alive after real rodio decode (no panic), `/tmp/af_stderr.log` 0 bytes, screenshot readout `last: play_tone (330 Hz, synthesized)`, `master vol: 1.0`.

**Example `T` key → method (the demo's coverage of the new API):** `T` → `Audio::play_tone(330.0, 0.15, 0.3)` → status `play_tone (330 Hz, synthesized)`.

**The games' tone calls (now channel-less, via `play_tone_on_bus(.., "sfx")`):**

| Game | Calls |
|---|---|
| `survivor` | fire `900,0.04,0.16`; boom `150,0.14,0.28`; boom `110,0.3,0.35` |
| `shooter` | fire `900,0.05,0.18`; boom `150,0.16,0.3`; boom `110,0.25,0.35` |

**Commit / merge:**

| Item | Value |
|---|---|
| Branch | `feat/audio-facade-tone` (deleted post-merge) |
| Local commit | `60826da` |
| Merged squash commit | `ca5080d` |
| PR | #199, `mergeStateStatus: CLEAN`, squash + branch-deleted |
| Version path | v0.51.0 → **v0.52.0** (MINOR) |
| Diff | 9 files modified (+185 / −60) |

## Code Analysis

- **`Audio::play_tone` / `play_tone_on_bus`** (`src/audio_facade.rs`): native body = `let channel = self.next_sfx_channel(); self.inner.assign_bus(&channel, MASTER_BUS | bus); self.inner.play_tone(&channel, freq, dur, vol);`. wasm body = `self.inner.play_tone(freq, dur, vol)` / `play_tone_on_bus(freq, dur, vol, bus)`. The ring (`next_sfx_channel`, `SFX_VOICES=16`) is shared with `play_sfx` — tones and sfx contend for the same 16 voices.
- **`WebAudio::play_tone_to`** (`src/audio_wasm.rs`): `let (Ok(osc), Ok(gain)) = (ctx.create_oscillator(), ctx.create_gain()) else { return };` → `osc.set_type(OscillatorType::Sine); osc.frequency().set_value(freq);` → connect `osc→gain→dest` (return on wiring failure) → envelope `g.set_value_at_time(0,now); linear_ramp_to(vol, now+edge); set_value_at_time(vol, max(now+dur-edge, now+edge)); linear_ramp_to(0, now+dur);` → `osc.start(); osc.stop_with_when(now+dur);`. `dest = bus_input(bus).unwrap_or(master.clone())`. The browser keeps connected nodes alive until the scheduled stop, so it's truly fire-and-forget. `#[allow(deprecated)]` on the fn.
- **`AudioManager::play_tone(channel, freq, dur, vol)`** (`src/audio/playback.rs:135`) — pre-existing; `stop_immediate(channel)` then a `SineWave::new(freq).take_duration(..).amplify(vol)` sink, with bus volume via `effective_volume(channel)` + channel effects. The facade's native `play_tone` delegates here (per voice-ring channel).
- **web-sys features** (`Cargo.toml:218`): added `"OscillatorNode", "OscillatorType"` to the existing audio block. Without these the wasm build fails E0599 on `create_oscillator`/`set_type`.
- **Game adoption shape:** `Audio`/`AudioFacadeSystem` move to the un-gated `use engine::{...}` block; init becomes un-gated `if let Some(mut audio) = Audio::new() { audio.set_bus_volume("sfx", 0.6); app.world.insert_resource(audio); } app.add_system(AudioFacadeSystem);` (no `assign_bus` — the facade assigns the bus per-call); the `play_tone` helper drops its `#[cfg]` arms + channel arg. `survivor` keeps a gated `use engine::GpuParticleEmitter;` (still native-only).
- **`WindowConfig`** — still built by full struct-literal in both games (a field add here = breaking, ~70 sites engine-wide). Untouched.

## Reference Snippets

**The facade's two `play_tone` methods** (native = ring voice + bus assign + delegate; wasm = pass-through):

```rust
pub fn play_tone(&mut self, freq: f32, dur: f32, vol: f32) {
    #[cfg(not(target_arch = "wasm32"))] {
        let channel = self.next_sfx_channel();          // shared 16-voice ring (same as play_sfx)
        self.inner.assign_bus(&channel, MASTER_BUS);     // ride master so set_master_volume scales it
        self.inner.play_tone(&channel, freq, dur, vol);  // pre-existing AudioManager synth
    }
    #[cfg(target_arch = "wasm32")] { self.inner.play_tone(freq, dur, vol); }
}
// play_tone_on_bus is identical but assigns `bus` instead of MASTER_BUS (native) /
// calls play_tone_on_bus(.., bus) (wasm).
```

**The wasm tone synthesizer** (`OscillatorNode` + click-free envelope, mirrors `play_sfx_to`'s routing):

```rust
#[allow(deprecated)]  // OscillatorNode::start/stop* (AudioScheduledSourceNode) bindings are deprecated
fn play_tone_to(&self, freq: f32, dur: f32, vol: f32, dest: &web_sys::GainNode) {
    let dur = dur.max(0.0) as f64;
    let vol = vol.clamp(0.0, 1.0);
    let (Ok(osc), Ok(gain)) = (self.ctx.create_oscillator(), self.ctx.create_gain()) else { return; };
    osc.set_type(web_sys::OscillatorType::Sine);
    osc.frequency().set_value(freq);
    if osc.connect_with_audio_node(&gain).is_err() || gain.connect_with_audio_node(dest).is_err() { return; }
    let now = self.ctx.current_time();
    let edge = (dur * 0.25).min(0.008);              // ≤8ms, or ¼ of a very short tone
    let g = gain.gain();
    let _ = g.set_value_at_time(0.0, now);
    let _ = g.linear_ramp_to_value_at_time(vol, now + edge);
    let _ = g.set_value_at_time(vol, (now + dur - edge).max(now + edge));
    let _ = g.linear_ramp_to_value_at_time(0.0, now + dur);
    let _ = osc.start();
    let _ = osc.stop_with_when(now + dur);
}
```

## Reusable Gotchas & Patterns (carry forward)

- **web-sys features are opt-in per Web Audio node type.** Using `OscillatorNode` needed `"OscillatorNode"` + `"OscillatorType"` added to the `web-sys` features in `Cargo.toml` (L218) — without them the wasm build fails `E0599` on `create_oscillator`/`set_type`. Any *new* `web_sys::Foo` type you touch likely needs its feature flag added there first.
- **The verify wasm gate builds lib+bins, NOT examples.** `./scripts/verify.sh`'s `cargo build --target wasm32` compiled the facade's new `OscillatorNode` branch (it's in the lib), but did NOT compile the example games for wasm. After un-gating a previously-`cfg`'d example path, verify it separately: `cargo build --example <name> --target wasm32-unknown-unknown` (the example targets are `survivor_game`/`shooter_game`, not `survivor`/`shooter`).
- **`cargo fmt --check` reflows hand-wrapped long lines** (a 15-element tuple `let`, an `if a.is_err() || b.is_err()` chain) → the *first* gate fails (`VERIFY_EXIT=1`). Run `cargo fmt` before the gate. (Bit again this session — same as seq 6.) `cargo fmt` does NOT reflow doc-comment prose.
- **The "false exit 0" trap is still live.** `./scripts/verify.sh > log 2>&1; echo $?` reported by a background runner is the trailing echo's exit, not verify.sh's. Append `echo "VERIFY_EXIT=$?" >> log` and `grep` it from the LOG. Caught the fmt failure (#1).
- **macOS synthetic key code: `T` = 17** (extends the seq-6 list — M=46, Digit1=18, ArrowDown=125, ArrowUp=126, Esc=53, B=11). Audio is not screencapturable; verify via process-liveness-after-play + empty stderr + on-screen readout delta. Launch under `caffeinate -dimsu` (background), `screencapture -x`, `pkill -f target/debug/examples/<name>` (the kill makes the bg task report exit 144 — expected, not a failure).
- **Web Audio fire-and-forget node lifetime.** A started `OscillatorNode`/`AudioBufferSourceNode` connected into the graph is kept alive by the browser until its scheduled stop, even after the JS handle drops — so `play_tone_to` needn't retain the nodes (same as the existing `play()` path). This is *the* reason fire-and-forget works without bookkeeping.
- **`let (Ok(a), Ok(b)) = (r1, r2) else { return };`** — a clean "both-or-bail" for paired fallible node creation (refutable let-else over a tuple of `Result`s).
- **The real cfg-guard dup lives in GAME code, not the engine** (confirmed again): each game wrote a native fn + a wasm no-op stub per audio call. The facade only deletes those guards if it covers the API the game actually uses — here that meant adding `play_tone` first. When eyeing a facade-adoption, grep the game's *actual* audio API before assuming the facade covers it.

## Files Changed

### Source code
- `src/audio_facade.rs` — added `Audio::play_tone` + `play_tone_on_bus`; module docs (coverage list + removed tone from the native-only-extras exclusion). No new tests (the voice ring is already unit-tested; the new methods are thin reuse).
- `src/audio_wasm.rs` — added `WebAudio::play_tone` + `play_tone_on_bus` + private `play_tone_to` (OscillatorNode + envelope).
- `Cargo.toml` — `web-sys` features `+ "OscillatorNode", "OscillatorType"`.

### Example
- `examples/audio_facade/audio_facade.rs` — `T` key → `play_tone`; key-edge tuple (now 15), legend, doc-comment.

### Games (adoption)
- `examples/games/shooter/shooter.rs` — facade adoption; 5→0 cfg-guards.
- `examples/games/survivor/survivor.rs` — facade adoption; 8→4 cfg-guards (rest GpuParticleEmitter).

### Docs / release paperwork
- `CLAUDE.md` — audio module-map row (3 edits) + header v1.6.121 + package v0.52.0.
- `docs/CHANGELOG.md` — 0.52.0 entry (Added / Changed / Notes).
- `Cargo.lock` — refreshed to 0.52.0.

### Memory (outside repo)
- `engine-current-state.md` → seq 70; `MEMORY.md` index line refreshed.

## User Feedback & Preferences (REQUIRED)

- **Direction #1 = "파사드를 게임에 적용 (settings_menu)"** — chosen via AskUserQuestion over facade-positional / RonRegistry-pub / HDR-RT.
- **Direction #2 (after the premise broke) = "파사드에 play_tone 추가 → survivor+shooter 채택"** — chosen via AskUserQuestion over byte-banking / settings_menu-limited / pivot.
- **"go"** — approved the plan + let me drive the whole chain (feature → /ship → /land-pr) without per-step confirmation, matching the seq-6 precedent.
- **"/handoff 하고 머지. /wrap 도 진행해줘"** — wants this handoff written AND landed via its own `docs(handoff)` PR (matching seq 2–6), then `/wrap` run.
- **Standing merge-authority delegation** (memory `merge-authority-delegated`) — squash-on-green-CI, no per-PR re-confirm. Applied.
- **Conventions:** user-facing reports in Korean, all artifacts/prompts/code/docs in English (memory `conversation-language-korean` + doc-language rule). Followed (chat Korean, this file English).
- The user values **evidence-first work + catching verification traps** — reading the real `VERIFY_EXIT` from the log, the targeted wasm-example build beyond the standard gate, the native real-play smoke. Expected rigor.

## Where We're Going

- **This feature is done + merged.** No follow-up required for `play_tone` itself.
- **Next driver order (unchanged):** read `../dungeon-merchant/docs/engine-wishlist.md` FIRST (ACTIVE empty, next ID EW-002) — a new EW request is top priority. When empty, ASK before backlog.
- **Carried directions still open** (offer if no EW request):
  1. **Adopt the facade in `settings_menu`** — needs cross-platform `AudioEffect` low-pass (Web Audio `BiquadFilterNode`) + a sustained-channel concept + `is_playing` on the facade. The next layer of the same dogfood; would close the last audio-cfg-guard game.
  2. **Positional audio on the facade** — the part excluded since seq 6. Needs a cross-platform handle over native channel `update_position` vs wasm `Sfx`. Its own session.
  3. **`RonRegistry<V>` + `RonLoadable` pub at crate root** (carried bonus since seq 2). `src/ron_registry.rs` + `src/lib.rs`.
  4. **HDR/linear render-target** (item-6 half) — a format-matched sprite pipeline variant. `src/renderer/sprite.rs` + `render_target.rs`.
- **Optional, low-cost:** ship `survivor`/`shooter` to the web (`/ship-wasm-example`) so their now-cross-platform audio is hearable in a browser, and confirm `audio_facade`'s `T` tone by ear on the web.

## Risks & Blockers

- **None blocking.** main is clean + green at v0.52.0.
- **Web tone untested by ear.** The wasm `OscillatorNode` path compiles (wasm lib + example builds) and is a thin standard Web Audio call, but no one has heard `play_tone` in a browser this session. Low risk; a human can run `examples/audio_facade/web/build.sh` + serve + press `T`.
- **Native master-volume nuance still applies to tones.** A tone sent to a *named* bus via `play_tone_on_bus` is not scaled by `set_master_volume` on native (buses don't nest); use `set_bus_volume`. On web it nests. (Documented; the games only set the "sfx" bus volume, so this doesn't bite them.)
- **Shared voice ring can cut.** `play_tone` and `play_sfx` share the 16-voice ring; 16+ rapid one-shots wrap and cut the oldest. Fine for these games (short tones, well under 16 simultaneous); a game needing a guaranteed-sustained tone should manage its own channel.

## Open Questions

- **None for `play_tone`** — done + merged.
- **Direction for next session:** EW request (board empty now) vs a carried direction (settings_menu adoption via `BiquadFilter` / facade positional / RonRegistry-pub / HDR-RT) vs fresh feature. The user drives this at the next seam.
- **Should the games call `resume()` on first input for web robustness?** Not added — the first sfx-triggering keypress IS the gesture, so playback unlocks naturally. If a web build ever queues silent until a second press, add a `resume()` on first input.

## Quick Start for Next Session

```bash
# Sync + verify clean/green
git checkout main && git pull --ff-only        # expect main @ ca5080d or later (this handoff's docs PR)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?" >> /tmp/verify.log; grep VERIFY_EXIT /tmp/verify.log   # must be 0 — read it from the LOG (a trailing echo masks the real exit)

# The audit backlog is closed; check the game wishlist board FIRST (top driver)
cat ../dungeon-merchant/docs/engine-wishlist.md    # ACTIVE empty, next ID EW-002

# Live engine state + gotchas
#   memory engine-current-state (seq 70) + MEMORY.md index

# Key files to read first (play_tone just shipped):
#   src/audio_facade.rs            — Audio::play_tone / play_tone_on_bus (+ the seq-6 facade)
#   src/audio_wasm.rs              — WebAudio::play_tone_to (OscillatorNode + envelope)
#   examples/games/shooter/shooter.rs , examples/games/survivor/survivor.rs — the adopted games
#   examples/audio_facade/         — the native+web demo (T key) + web/ harness

# Hear the new tone on web (optional, not done this session):
#   examples/audio_facade/web/build.sh && python3 -m http.server 8080 --directory examples/audio_facade/web   # click Start, press T

# Next action
#   Read the wishlist board. If an EW request exists, do that. Otherwise ASK which carried direction
#   (settings_menu adoption via a cross-platform AudioEffect low-pass / facade positional audio /
#   RonRegistry pub / HDR render-target / fresh feature).
```

---

## Session Closed

**Closed at:** 2026-06-22
**Session status:** Handed off to next session.
**Code work:** `play_tone` on the `Audio` facade + `shooter`/`survivor` adoption landed via PR **#199** (v0.52.0, merge commit `ca5080d`) — already on main before this handoff.
**Landing:** this handoff doc lands on `main` via its own `docs(handoff)` PR (matching seq 2–6). Memory `engine-current-state` is at seq 70; `MEMORY.md` index refreshed.
