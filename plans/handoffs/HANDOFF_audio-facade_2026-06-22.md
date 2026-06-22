# Cross-platform `Audio` facade — one audio API for native + web (v0.51.0, additive feature)

**Date:** 2026-06-22
**Status:** COMPLETED + merged. main @ `f3f136f`, package **v0.51.0**, clean tree, full gate green, CI green, squash-merged (#197).
**Bead(s):** none
**Epic:** post-audit feature work (back on the VISION feature+example loop)
**Chain:** `standalone-4365aa4a` seq `6`
**Parent:** `HANDOFF_audit-item7-unwrap-hardening_2026-06-22.md` (seq 5)
**Prior chain:** `HANDOFF_engine-audit-fixes_2026-06-22.md` (seq 1) > `HANDOFF_audit-followup-refactors_2026-06-22.md` (seq 2) > `HANDOFF_audit-deferred-editor-tier5_2026-06-22.md` (seq 3) > `HANDOFF_audit-item6-texture-format_2026-06-22.md` (seq 4) > `HANDOFF_audit-item7-unwrap-hardening_2026-06-22.md` (seq 5) > this (seq 6)
**Auto:** false

---

## Stale References

None. The parent (seq 5) was the `ecs/world.rs` unwrap-hardening pass; all its identifiers (`world.rs` query/`par_query`/`spawn`/`despawn`, the `expect` messages, the texture-format anchors carried from seq 4) still resolve — this session did not touch `ecs/world.rs` or `renderer/texture.rs`. All new identifiers introduced here are listed under **Code Analysis** / **Files Changed**.

## Since Last Handoff (seq-5 plan vs reality)

The parent was the **terminal handoff of the engine-audit follow-up arc** (seq-1 audit deferred list items 1–8 fully closed). Its "Where We're Going" listed four candidate directions once the audit backlog emptied; this session executed one of them end-to-end.

- **Audit arc stays closed.** No audit work this session — the deferred list (items 1–8) remains fully closed (1–4 + 6 + 7 + 8 done, 5 was rejected-as-a-trait). This seq 6 is the **first post-audit feature**.
- **Picked the item-5 reframe.** The parent named it: *"Item 5 reframed (future feature, not a refactor): a cross-platform audio facade … `/add-feature-example` with a game that plays audio on both native and web. This is the rejected item 5, re-scoped as a feature."* That is exactly what shipped here — the rejected `AudioSurface` *trait* delivered as a *facade feature*.
- **Open question answered.** The parent's "Direction question for next session" (EW request / `RonRegistry` pub / audio facade / HDR RT / fresh feature) — the wishlist board was empty (next ID EW-002), so I asked the user; they chose the **audio facade**, then **Full** scope.
- **The other carried directions remain open** for a future session: `RonRegistry<V>`+`RonLoadable` pub at crate root (carried bonus), HDR/linear render-target (item-6 half), positional-audio handle abstraction (the part of the facade deliberately deferred here).

## Reference Documents

- `CLAUDE.md` — module map (the audio row now documents the `Audio` facade), verify rules, pre-1.0 versioning (MINOR = any release incl. additive feature).
- `docs/VISION.md` — the feature+example loop ("the example is the acceptance test; if the API feels awkward writing the example, fix the API").
- `docs/WASM_SMOKES.md` — now has a "Web examples without a dedicated smoke" section explaining why `audio_facade` has no headless smoke.
- `../dungeon-merchant/docs/engine-wishlist.md` — the wishlist board (ACTIVE empty, next ID EW-002). Read FIRST each session.
- Memory `engine-current-state` (now seq 69) + `MEMORY.md` index.

## The Goal

The engine ships two audio backends with deliberately divergent shapes — native `AudioManager` (rodio; channel-name + file-path keyed; `&mut self`; needs a per-frame `update(dt)` tick) and wasm `WebAudio` (Web Audio; bytes keyed; `&self`; audio-clock scheduled, no tick). A game that targets both platforms had to write **every audio call twice**: a `#[cfg(not(target_arch = "wasm32"))]` arm using `AudioManager` and a `#[cfg(target_arch = "wasm32")]` no-op/stub. The goal: a single cross-platform **`Audio`** facade so a dual-target game writes its audio logic with **zero `cfg` guards**, exercised by an example that plays audio identically on native and web (the VISION acceptance test). This is the seq-1 audit **item 5** (`AudioSurface` trait, rejected seq 66 as too-divergent-for-a-trait) re-scoped as a **facade feature**, not a refactor.

## Where We Are

- **main @ `f3f136f`, package v0.51.0, CLAUDE.md header v1.6.120, clean tree, `./scripts/verify.sh` → exit 0** (fmt + clippy `-D warnings` native + wasm32 lib/bins build + `test --all-targets` + doc `-D warnings`), run three times (post-edit, post-doc-fix, post-version-bump).
- **PR #197 squash-merged** on green CI (Build WASM 48s · Rustdoc 40s · Package dry-run 1m7s · Test native 5m1s), branch `feat/audio-facade` deleted, local main fast-forwarded.
- **New module `src/audio_facade.rs`** (~315 lines incl. tests) — `pub struct Audio` + `pub struct AudioFacadeSystem`, both re-exported **un-gated** from `src/lib.rs` (`pub mod audio_facade;` + `pub use audio_facade::{Audio, AudioFacadeSystem};`).
- **`Audio` API (Full scope, `&mut self`, bytes-keyed):** `new() -> Option<Self>`, `play_sfx`, `play_sfx_on_bus`, `play_music`, `crossfade_music`, `stop_music`, `set_master_volume`, `set_bus_volume`, `bus_volume`, `duck_bus`, `release_bus`, `bus_duck`, `resume`, `update`.
- **`AudioFacadeSystem`** — built-in system ticking `Audio::update(dt)` each frame (cross-platform analogue of the native-only `AudioSystem`); `LABEL = "engine::audio_facade"`.
- **Additive native API:** `AudioManager::play_bytes(channel, &[u8], repeat)` + `AudioManager::crossfade_bytes(channel, &[u8], repeat, dur)` (`src/audio/playback.rs`), backing the facade and useful standalone for `include_bytes!` audio.
- **Behavior-preserving refactor:** `play_internal` and `crossfade` now share private helpers `append_decoded` and `begin_crossfade` (`src/audio/playback.rs`); the existing audio test suite is untouched + green.
- **Example `examples/audio_facade/audio_facade.rs`** (registered in `Cargo.toml` as `[[example]]`) — cfg-free `run()` + `AudioDemo` system; dual-target `main`/`#[wasm_bindgen] run_audio_facade` boilerplate is the ONLY cfg split. Web harness `examples/audio_facade/web/{build.sh,index.html}` (pkg/ gitignored).
- **CLAUDE.md** audio module-map row extended with the facade; header bumped. **docs/CHANGELOG.md** 0.51.0 entry. **docs/WASM_SMOKES.md** notes the no-smoke decision.
- **Verification beyond the gate:** native real-play smoke (window launched, `play_music`/`play_sfx` decoded via rodio with no panic + clean stderr, on-screen readouts updated on synthetic keys — master vol 1.0→0.9); wasm example bundle builds (`audio_facade_bg.wasm` 13.5 MB + `.js`).
- **Memory** `engine-current-state` bumped to seq 69 (frontmatter description prepend + version/hash bump + lead paragraph + new seq-69 bullet); `MEMORY.md` index line refreshed.

## What We Tried (Chronological)

1. **Onboarding (read-only).** Read the full seq-5 parent, the wishlist board (`_None open._`, next ID EW-002 → no EW request preempts), ran `./scripts/verify.sh` (baseline green, exit 0), read memory `engine-current-state` (seq 68). Narrated the 5-step onboarding in Korean.
2. **Asked direction (no EW request).** `AskUserQuestion` → user chose **audio facade (item-5 reframe)** over RonRegistry-pub / HDR-RT / fresh-feature.
3. **Explored the two backends.** Read `src/audio.rs` (AudioManager struct + AudioSystem), `src/audio_wasm.rs` (full WebAudio surface), `src/lib.rs` audio cfg-gating, `src/audio/playback.rs` (`play_internal`, `crossfade`, `read_cached_bytes`, `effective_volume`), `src/audio/bus.rs` (`assign_bus`/`set_bus_volume`/`effective_volume_params`). **Key finding:** `play_internal` already decodes from an in-memory `Decoder::new(Cursor::new(bytes))` (the file path is only the cache key) → a bytes-based native play path is ~free.
4. **Grounded the dup in real game code.** `grep` over `examples/games/` → `settings_menu.rs` has **16 cfg-guards** around 10 audio calls (native `AudioManager` fn + wasm no-op stub per helper), `survivor.rs` 7, `shooter.rs` 4. Confirmed the facade target. No `Audio` naming collision in `src/`.
5. **Confirmed dual-target example model.** `web_audio.rs` generates WAV bytes in code via `sine_wav(freq, secs)` (no asset file needed) — reused. `centered_text` is the web-shipped windowed model: cfg-free `run()` + `#[cfg(not(wasm32))] fn main(){run()}` + `#[cfg(wasm32)] #[wasm_bindgen] pub fn run_x(){run()}` + empty wasm main. `World::insert_resource<T: 'static>` has **no Send+Sync bound** → `WebAudio` (Rc/RefCell) and thus `Audio` are valid resources.
6. **Asked API scope.** `AskUserQuestion` → user chose **Full** (sfx + music/crossfade + master vol + named buses + ducking) over Minimal / Full+positional. Presented the plan + first action, got "go".
7. **Branched `feat/audio-facade`** off main (`f3f136f`'s parent, `932038d`).
8. **Native bytes-play (src/audio/playback.rs).** Extracted `append_decoded` (decode→effects→pan/fade/repeat→insert) + `begin_crossfade` (temp-channel relocation) as private helpers; added pub `play_bytes`/`crossfade_bytes`; refactored `play_internal`/`crossfade` to call them. **Preserved ordering** so a failed read still tears down the old sink (the prior behavior).
9. **Wrote `src/audio_facade.rs`.** `Audio` with a cfg-gated `inner` (native `AudioManager` + `sfx_seq: u64`; wasm `WebAudio`). Per-method cfg bodies for divergent ops; plain pass-through (no cfg) for the 5 name-identical bus methods. Round-robin 16-voice SFX channels (`sfx_voice_channel(seq, voices)`, native-only, unit-tested). Added `resume()` beyond the agreed scope (web AudioContext unlock; native no-op) to keep the example cfg-free — flagged to the user.
10. **Wired lib.rs** (un-gated `pub mod` + re-export), **wrote the example** (`examples/audio_facade/audio_facade.rs`), **registered in Cargo.toml**, **updated CLAUDE.md** module-map row.
11. **First verify → FALSE "exit 0", actually `VERIFY_EXIT=1`.** The background-runner reported "exit code 0" — that was the trailing `echo`'s exit, NOT `verify.sh`'s. Reading `VERIFY_EXIT` from the task-output file showed **1**: `cargo fmt --check` failed (my hand-wrapped long lines). Ran `cargo fmt`; re-ran with `echo "VERIFY_EXIT=$?" >> log` so the real exit lands in the readable log.
12. **Second verify → `VERIFY_EXIT=101`** (doc gate). `RUSTDOCFLAGS="-D warnings" cargo doc` failed: 3× `unresolved link to crate::audio_wasm::WebAudio` (the module is `#[cfg(target_arch="wasm32")]` → absent on the NATIVE doc build) + 1× `redundant_explicit_links` (the in-scope `use`'d `AudioManager`). Fixed by converting **all backend-type doc references** (`AudioManager`/`WebAudio` + methods) to plain backticks; kept links only to `Audio`/`Self` methods, `AudioFacadeSystem`, `crate::AudioSystem`, `crate::audio_facade`.
13. **Third verify → `VERIFY_EXIT=0`** (full gate green: fmt + clippy + wasm32 lib/bins + `test --all-targets` [911 lib + new facade tests] + doc).
14. **Native real-play smoke.** Built + launched the example windowed (`caffeinate` background), `osascript` synthetic key codes (M=46 play_music, 1=18 play_sfx, 125 ArrowDown master vol), `screencapture`. Process alive after real rodio decode (no panic), stderr clean, screenshot showed `master vol: 0.9` / `last: set_master_volume 0.9`. Killed the example.
15. **`/ship-wasm-example`.** wasm-bindgen-cli version matched the crate; `cargo build --example audio_facade --target wasm32` → exit 0 (facade + example compile on wasm). Wrote `web/build.sh` (+x) + `web/index.html` (Start gesture → `run_audio_facade()` + canvas focus, key legend, `?autostart=1` hook). Ran build.sh → `pkg/audio_facade_bg.wasm` (13.5 MB) + `.js`; pkg gitignored. **Skipped the headless smoke** (documented why in `docs/WASM_SMOKES.md`).
16. **`/ship`** (the four edits): `Cargo.toml` 0.50.1→0.51.0, `cargo update -p skeleton-engine` (lock→0.51.0), `docs/CHANGELOG.md` 0.51.0 entry (Added / Notes / Changed (internal)), `CLAUDE.md` header v1.6.119→v1.6.120 + package v0.50.1→v0.51.0. Re-ran the full gate → exit 0.
17. **Landed.** `git add -A` (pkg excluded), commit `b934c80`, push, `gh pr create` (#197), `gh pr checks 197 --watch --fail-fast --interval 30` (background, exit 0), confirmed `mergeStateStatus: CLEAN`, `gh pr merge --squash --delete-branch` (merge `f3f136f`), `git pull --ff-only`, bumped memory to seq 69.

## Key Decisions

- **`bytes` is the cross-platform clip identity.** Native is path-keyed, wasm is bytes-keyed, but `include_bytes!` works on both and rodio decodes from an in-memory `Cursor` (already does in `play_internal`). So the facade API is bytes-based; native gets the tiny additive `play_bytes`/`crossfade_bytes`. Rejected: path-based (wasm has no filesystem) and a `{path, bytes}` struct (the game would still supply both).
- **Facade type named `Audio`** (no collision), driver system **`AudioFacadeSystem`** (`AudioSystem` is taken by the native-only `AudioManager` driver; a game uses one OR the other, but the type namespace forbids reuse).
- **`&mut self` everywhere** — native needs it; wasm's `&self` methods are callable through `&mut self`. Unifying on the stricter signature gives one API shape.
- **Native master volume = a conventional `"master"` bus.** Native buses don't nest (web's do), so the facade routes unrouted `play_sfx`/`play_music` to a `"master"` bus that `set_master_volume` controls. **Accepted nuance (documented):** a `play_sfx_on_bus` named-bus sound bypasses `set_master_volume` on native (use `set_bus_volume`); on web it nests under master. The only place the two platforms differ observably.
- **Round-robin 16-voice SFX channels** (`__facade_sfx_{seq % 16}`) — bounds the native sink count (a new one-shot reuses/cuts the oldest voice on wrap) with zero pruning logic. Rejected: monotonic channel names + `update()` pruning (needs tracking the live set + `is_finished`/`stop` per channel).
- **Added `resume()` beyond the agreed Full scope.** Web AudioContext starts suspended until a user gesture; without a resume primitive the web demo would queue silent. `resume()` = `WebAudio::resume()` on wasm, **no-op on native** (native audio is never locked — semantically a coherent "ensure audio is running", not a leaky stub). Flagged to the user. **Did NOT add `suspend()`** — its native no-op WOULD be misleading (native has no global pause).
- **Excluded `play_tone` (native-only synthesis) + positional `play_at`.** Positional needs a cross-platform handle abstraction (native channel-based `update_position` vs wasm `Sfx`) → a larger follow-on. The user picked "Full" not "Full + positional".
- **Behavior-preserving refactor over duplication.** `play_internal`/`crossfade` share `append_decoded`/`begin_crossfade` rather than the bytes path duplicating ~80 lines. Preserved the exact stop-immediate→sink→volume→byte-read ordering so a failed read still silences the channel (prior behavior). The decode-fail log message changed from path to channel name (a `log::warn` only; no test asserts it).
- **No dedicated headless wasm smoke for `audio_facade`.** Audio needs a user gesture + isn't headless-capturable; the facade's wasm branch is thin pass-throughs to `WebAudio`, whose runtime lifecycle is already covered by `wasm_audio_smoke.sh` (the `web_audio` example). A render-only smoke would only prove boot, marginal over the wasm build + `WebAudio::new`. Documented in `docs/WASM_SMOKES.md`.
- **MINOR bump v0.51.0** (pre-1.0: additive feature = MINOR). New public API, behavior-preserving for existing call sites.
- **Drove the whole chain on one go-ahead** (`/add-feature-example` → `/ship-wasm-example` → `/ship` → land + merge) per the standing merge-authority delegation + the seq-68 precedent.

## Evidence & Data

**Game-side dup that motivated the facade (the target to eliminate):**

| Example | cfg-guards | audio calls |
|---|---:|---:|
| `examples/games/settings_menu/settings_menu.rs` | 16 | 10 |
| `examples/games/survivor/survivor.rs` | 7 | 6 |
| `examples/games/shooter/shooter.rs` | 4 | 6 |

**Verify-gate runs (read the real exit from the LOG, not the runner summary):**

| Run | Result | Cause / note |
|---|---|---|
| post-edit #1 | `VERIFY_EXIT=1` | `cargo fmt --check` failed (hand-wrapped long lines) — runner falsely reported "exit 0" (the trailing `echo`'s exit) |
| post-fmt #2 | `VERIFY_EXIT=101` | doc gate: 3× unresolved `crate::audio_wasm::WebAudio` (wasm-only module absent on native doc build) + 1× `redundant_explicit_links` |
| post-doc-fix #3 | `VERIFY_EXIT=0` | full gate green; 911 lib tests + new facade tests |
| post-version-bump | `VERIFY_EXIT=0` | green after the 0.51.0 paperwork |

**CI on PR #197 (all pass):**

| Check | Time |
|---|---|
| Build (WASM) | 48s |
| Rustdoc | 40s |
| Package dry-run | 1m7s |
| Test (native) | 5m1s |

**Facade unit tests (native, headless — no audio device needed):**
- `sfx_voices_wrap_round_robin` — `sfx_voice_channel(16,16) == "__facade_sfx_0"` (wraps), 16 distinct names in one revolution.
- `next_sfx_channel_advances_and_wraps` — counter advances + wraps at `SFX_VOICES`.

**Native real-play smoke (synthetic input via `osascript` key codes):**
- Keys sent: `46` (M, `play_music` → 110 Hz loop), `18` (1, `play_sfx` → 440 Hz), `125` (ArrowDown, master vol 1.0→0.9).
- Result: process PID alive after real rodio decode (no panic), `/tmp/af_stderr.log` empty, screenshot readout `master vol: 0.9 / bed bus vol: 1.0 / bed duck: 1.00`, `last: set_master_volume 0.9`.

**wasm bundle (examples/audio_facade/web/pkg/, gitignored):** `audio_facade_bg.wasm` 13,475,367 B; `audio_facade.js` 153,709 B.

**Commit / merge:**

| Item | Value |
|---|---|
| Branch | `feat/audio-facade` (deleted post-merge) |
| Local commit | `b934c80` |
| Merged squash commit | `f3f136f` |
| PR | #197, `mergeStateStatus: CLEAN`, squash + branch-deleted |
| Version path | v0.50.1 → **v0.51.0** (MINOR) |
| Diff | 11 files (5 modified, 6 added incl. web harness; pkg/ excluded) |

**WAV clips in the example (`sine_wav(freq, secs)`, amplitude 0.25, 44.1 kHz mono PCM):** SFX 440/587/784 Hz × 0.25 s; music_a 110 Hz × 1 s (loop); music_b 165 Hz × 1 s (crossfade target); bed 220 Hz × 4 s (sustained pad on the "bed" bus).

**Example key → facade method map (the demo's coverage of the Full API):**

| Key | Facade call | Demonstrates |
|---|---|---|
| `1` / `2` / `3` | `play_sfx(440/587/784 Hz)` | fire-and-forget one-shot (master) |
| `M` | `play_music(110 Hz loop)` | looping music |
| `C` | `crossfade_music(165 Hz, 1.5 s)` | track-to-track crossfade |
| `X` | `stop_music()` | stop music |
| `↑` / `↓` | `set_master_volume(±0.1)` | master volume |
| `B` | `play_sfx_on_bus(220 Hz 4 s pad, "bed")` | named-bus routing |
| `[` / `]` | `set_bus_volume("bed", ±0.1)` + readout via `bus_volume` | per-bus volume |
| `D` | `duck_bus("bed", 0.2, 0.15)` | ducking |
| `F` | `release_bus("bed", 0.4)` | release |
| (any) | `resume()` | web AudioContext unlock |
| (per frame) | `update(dt)` via `AudioFacadeSystem` | native fades/ducks tick |
| readouts | `bus_volume("bed")`, `bus_duck("bed")` | live mixer state |

## Code Analysis

- **`Audio`** (`src/audio_facade.rs`): `struct Audio { #[cfg(not(wasm32))] inner: AudioManager, #[cfg(not(wasm32))] sfx_seq: u64, #[cfg(wasm32)] inner: WebAudio }`. `new() -> Option<Self>` mirrors both backends. The 5 name-identical bus methods (`set_bus_volume`/`bus_volume`/`duck_bus`/`release_bus`/`bus_duck`) are plain pass-through `self.inner.X(...)` (no cfg — both backends expose the same signatures). Divergent methods use per-method `#[cfg]` bodies.
- **Native routing:** `play_sfx` → `next_sfx_channel()` (ring), `assign_bus(channel, "master")`, `play_bytes(channel, bytes, false)`. `play_sfx_on_bus` → same ring channel `assign_bus(channel, bus)`. `play_music`/`crossfade_music` → fixed `"__facade_music"` channel, `assign_bus(.., "master")`. `assign_bus` is called before play so `effective_volume` picks up the bus.
- **Wasm routing:** `play_sfx` → `WebAudio::play`; `play_sfx_on_bus` → `play_sfx_on_bus` (returns `Sfx`, discarded — `Sfx` is not `#[must_use]`); `set_master_volume` → `WebAudio::set_volume`; `update` → no-op (`let _ = dt;`); `resume` → `WebAudio::resume`.
- **`src/audio/playback.rs` helpers:** `play_internal(channel, path, repeat, fade)` resolves bytes via `read_cached_bytes` then calls `append_decoded`. `play_bytes_internal(channel, Arc<[u8]>, repeat, fade)` skips the cache. `append_decoded(channel, sink, Arc<[u8]>, repeat, fade)` (5 params, under the clippy `too_many_arguments` boundary) reads pan, decodes via `Decoder::new(Cursor::new(bytes))`, applies effects+pan/fade/repeat, inserts the sink. `begin_crossfade(channel, dur)` does the temp-channel relocation.
- **Public bytes API:** `play_bytes`/`crossfade_bytes` take `&[u8]` and do `Arc::from(bytes)` (one copy per play — acceptable for SFX; mirrors the existing `file_cache: HashMap<String, Arc<[u8]>>`).
- **Constants (native-only, cfg-gated to avoid wasm dead_code):** `MASTER_BUS = "master"`, `MUSIC_CHANNEL = "__facade_music"`, `SFX_VOICES: u64 = 16`. `sfx_voice_channel`/`next_sfx_channel` are `#[cfg(not(wasm32))]`.
- **`World::insert_resource<T: 'static>`** (`src/ecs/world.rs:632`) — `'static` only, no `Send + Sync`, so `WebAudio` (Rc/RefCell) and `Audio` are valid resources. `ShouldQuit(pub bool)` (`.0 = true` to quit).
- **Dual-target example shape:** cfg-free `run()` builds the App; `#[cfg(not(wasm32))] fn main(){run()}`; `#[cfg(wasm32)] #[wasm_bindgen] pub fn run_audio_facade(){run()}` + empty wasm `main`. Engine default canvas id is `"game-canvas"` (matches the index.html `<canvas>`).
- **WebAudio surface the wasm branch depends on** (for the positional follow-on — all `&self`): `play(&[u8])`, `play_on_bus(&[u8], &str)`, `play_sfx(&[u8]) -> Sfx`, `play_sfx_on_bus(&[u8], &str) -> Sfx`, `play_music(&[u8])`, `crossfade_music(&[u8], f32)`, `stop_music()`, `set_volume(f32)` (master gain), `set_bus_volume`/`bus_volume`/`duck_bus`/`release_bus`/`bus_duck`, `resume()`/`suspend()`, `play_at(&[u8], Vec2, Vec2, f32) -> Sfx` + `play_at_on_bus(...)`, `Sfx::{set_volume,set_pan,update_position,is_playing,stop}`. A `bus` is a `duck → volume → master` 2-gain chain (nests under master — unlike native).

## Reference Snippets

**The facade's two body shapes** — pass-through (no cfg, name-identical on both backends) vs per-method cfg body (divergent):

```rust
// pass-through: both AudioManager and WebAudio expose set_bus_volume(&_, &str, f32)
pub fn set_bus_volume(&mut self, bus: &str, v: f32) { self.inner.set_bus_volume(bus, v); }

// divergent: per-method #[cfg] body
pub fn set_master_volume(&mut self, v: f32) {
    #[cfg(not(target_arch = "wasm32"))] { self.inner.set_bus_volume(MASTER_BUS, v); } // native "master" bus
    #[cfg(target_arch = "wasm32")]      { self.inner.set_volume(v); }                  // web master gain
}
pub fn play_sfx(&mut self, bytes: &[u8]) {
    #[cfg(not(target_arch = "wasm32"))] {
        let ch = self.next_sfx_channel();          // round-robin "__facade_sfx_{seq%16}"
        self.inner.assign_bus(&ch, MASTER_BUS);     // ride master so set_master_volume scales it
        self.inner.play_bytes(&ch, bytes, false);
    }
    #[cfg(target_arch = "wasm32")] { self.inner.play(bytes); }
}
```

**Master-volume nuance, worked:** with `set_master_volume(0.5)` then `play_sfx(s)` and `play_sfx_on_bus(s2, "sfx")` —
- **native:** `s` rides the `"master"` bus (vol 0.5) → scaled to 0.5; `s2` rides the `"sfx"` bus (vol 1.0, NOT under master) → plays at 1.0. To dim `s2`, call `set_bus_volume("sfx", ..)`.
- **web:** `s` → master gain 0.5; `s2` → `"sfx"` bus → master gain → also scaled by 0.5 (buses nest). The one observable cross-platform difference.

## Reusable Gotchas & Patterns (carry forward)

These cost time this session or are reusable beyond it — surface them so the next session doesn't re-discover them.

- **The false "exit 0" trap (bit twice this session).** Running `./scripts/verify.sh > log 2>&1; echo "VERIFY_EXIT=$?"` in a background task: the **background-runner's reported "exit code 0" is the trailing `echo`'s exit, NOT `verify.sh`'s** (the redirect applies only to verify.sh; echo's stdout goes to the task-output file). `set -euo pipefail` halts verify.sh at the first failing gate, so the redirected log just *stops* (e.g. mid-fmt-diff). **Fix:** append the real exit INTO the readable log — `./scripts/verify.sh > /tmp/v.log 2>&1; echo "VERIFY_EXIT=$?" >> /tmp/v.log` — then `grep VERIFY_EXIT /tmp/v.log`. Reinforces memory `ci-toolchain-pin` ("never mask exit codes in gate pipes"). Caught a `fmt --check` failure (#1, exit 1) and a doc failure (#2, exit 101) that both looked like "exit 0".
- **wasm-only intra-doc links break the NATIVE doc gate.** `RUSTDOCFLAGS="-D warnings" cargo doc` runs on native; a `[`Foo`](crate::audio_wasm::Foo)` link to a `#[cfg(target_arch="wasm32")]`-gated item is `unresolved` there (the module doesn't exist) → `error: could not document`. **Also:** an intra-doc link to a type that's `use`'d into scope (e.g. `[`AudioManager`](crate::audio::AudioManager)` after `use crate::audio::AudioManager`) trips `redundant_explicit_links`. **Rule for an un-gated module that references both backends:** use **plain backticks** for backend types/methods (`` `AudioManager` ``, `` `WebAudio::new` ``); reserve real intra-doc links for same-platform items (`Self::*`, `Audio::*`, `crate::audio_facade`, `crate::AudioSystem`).
- **`cargo fmt` reflows hand-wrapped long lines** (long `DrawText::new(...)` calls, a `.map(...).collect()` chain) → `fmt --check` fails the first gate. Run `cargo fmt` before the gate, or expect the bit. (`cargo fmt` does NOT reflow doc-comment prose, so doc edits are safe.)
- **`World::insert_resource<T: 'static>` has no `Send + Sync` bound** (`src/ecs/world.rs:632`) — that's why `WebAudio` (Rc/RefCell, not Send) and `Audio` are valid resources. Don't assume resources need Send+Sync.
- **Round-robin voice ring for fire-and-forget sinks:** `format!("__prefix_{}", seq % N)` + `seq = seq.wrapping_add(1)` bounds the native `AudioManager` sink count to N (a new one-shot's `stop_immediate` cuts the oldest voice on wrap) with zero pruning. Cheaper than monotonic names + `is_finished`/`stop` GC. Used N=16.
- **Windowed audio playtest on macOS** (extends memory `playtest-windowed-examples`): synthetic **key codes** reach winit (`osascript -e 'tell application "System Events" to key code N'`), synthetic clicks do NOT. Codes used: M=46, Digit1=18, ArrowDown=125, ArrowUp=126, Esc=53, B=11. Audio output is NOT screencapturable — verify via process-liveness-after-play (no panic) + clean stderr + on-screen readout deltas (master vol 1.0→0.9). Launch under `caffeinate -dimsu` in background, `screencapture -x`, then `pkill -f target/debug/examples/<name>`.
- **The verify wasm gate (`cargo build --target wasm32 --lib`) DOES compile an un-gated lib module's wasm branch** — so the facade's wasm code path was checked by `./scripts/verify.sh` even though examples aren't built for wasm there. The example's wasm-compile is only checked by `/ship-wasm-example`'s `cargo build --example --target wasm32`.

## Files Changed

### Source code
- `src/audio_facade.rs` — **new.** `Audio` facade + `AudioFacadeSystem` + the `sfx_voice_channel` ring helper + 2 unit tests.
- `src/audio/playback.rs` — added pub `play_bytes`/`crossfade_bytes`; extracted private `append_decoded`/`begin_crossfade`; refactored `play_internal`/`crossfade` to use them (behavior-preserving).
- `src/lib.rs` — `pub mod audio_facade;` (un-gated) + `pub use audio_facade::{Audio, AudioFacadeSystem};`.

### Example
- `examples/audio_facade/audio_facade.rs` — **new.** cfg-free `run()` + `AudioDemo` system (key-mapped facade calls + live legend/readouts) + `sine_wav` + dual-target entry points.
- `examples/audio_facade/web/build.sh` — **new** (cargo build --release --example --target wasm32 + wasm-bindgen --target web).
- `examples/audio_facade/web/index.html` — **new** (init → Start gesture → `run_audio_facade()` + canvas focus + key legend + `?autostart=1`).
- `Cargo.toml` — `[[example]] name = "audio_facade"` block.

### Docs / release paperwork
- `CLAUDE.md` — audio module-map row extended with the facade; header v1.6.120 + package v0.51.0.
- `docs/CHANGELOG.md` — 0.51.0 entry (Added / Notes / Changed (internal)).
- `docs/WASM_SMOKES.md` — "Web examples without a dedicated smoke" section (audio_facade, with rationale).
- `Cargo.lock` — refreshed to 0.51.0.

### Tests
- 2 new unit tests in `src/audio_facade.rs` (ring math). The existing audio test suite was untouched and stayed green (acceptance test for the behavior-preserving refactor).

### Memory (outside repo)
- `engine-current-state.md` → seq 69; `MEMORY.md` index line refreshed.

## User Feedback & Preferences (REQUIRED)

- **Direction = "audio facade (item-5 리프레임)"** — chosen via AskUserQuestion over RonRegistry-pub / HDR-RT / fresh-feature.
- **Scope = "Full (권장)"** — sfx + music/crossfade + master vol + named buses + ducking (not Minimal, not Full+positional).
- **"go"** — approved the plan + first action (native bytes-play foundation first) and let me drive the entire chain (feature → wasm example → ship → land + merge) without per-step confirmation.
- **`/handoff 하고 푸시`** — wants this handoff doc written AND landed via its own `docs(handoff)` PR (matching seq-2 #187, seq-3 #192, seq-4 #194, seq-5 #196).
- **Standing merge-authority delegation** (memory `merge-authority-delegated`) — squash-on-green-CI, no per-PR re-confirm. Applied.
- **Conventions:** user-facing reports in Korean, all artifacts/prompts/code in English (memory `conversation-language-korean`). Followed (this file is English; chat was Korean).
- The user values **methodical, evidence-first work** and **catching verification traps** — the false "exit 0" catch (reading the real `VERIFY_EXIT` from the log) is the kind of rigor expected; reinforces memory `ci-toolchain-pin`'s "never mask exit codes in gate pipes".

## Where We're Going

- **This feature is done + merged.** No follow-up required for the facade itself.
- **Next driver order (unchanged):** read `../dungeon-merchant/docs/engine-wishlist.md` FIRST (ACTIVE empty, next ID EW-002) — a new EW request is top priority.
- **Carried directions still open** (offer if no EW request):
  1. **`RonRegistry<V>` + `RonLoadable` pub at crate root** (carried bonus since seq 2) — small additive API so forks register custom RON-loaded asset types. `src/ron_registry.rs` (currently crate-internal) + `src/lib.rs`.
  2. **Positional audio on the facade** — the part deliberately excluded here. Needs a cross-platform handle abstraction over native channel-based `update_position` vs wasm `Sfx`. Its own session. `src/audio_facade.rs` + `src/audio/positional.rs` + `src/audio_wasm.rs`.
  3. **HDR/linear render-target** (item-6 half) — a format-matched sprite pipeline variant so a `Rgba16Float` render target can be rendered into. `src/renderer/sprite.rs` + `render_target.rs`.
  4. **Adopt the facade in a game** — migrate `examples/games/settings_menu` (16 cfg-guards) or `survivor`/`shooter` to `Audio` to validate the API ergonomics under real use and delete the dup. Strong VISION fit.

## Risks & Blockers

- **None blocking.** main is clean + green at v0.51.0.
- **Untested on real web hardware (by ear).** The facade's wasm branch compiles + is transitively covered by `web_audio`'s smoke, but no one has clicked Start in a browser and heard the `audio_facade` page this session. Low risk (thin pass-throughs to a smoke-covered backend); a human can `examples/audio_facade/web/build.sh` + serve + click to confirm.
- **Master-volume nuance is a real cross-platform behavior gap** (named-bus sounds bypass master on native) — documented, not a bug, but a game mixing named buses + a master slider on both platforms must know it.
- **Native `crossfade_music` out-going track briefly bypasses the master bus.** `begin_crossfade` relocates the old sink to a temp channel `"{ch}__xfade"` which has no `channel_buses` entry → its `effective_volume` uses bus_vol 1.0 during the fade-out (so `set_master_volume` doesn't scale the out-going tail for `dur` seconds). Brief + cosmetic; accepted for the MVP. If it ever matters, assign the temp channel to `MASTER_BUS` inside `begin_crossfade` (native) — but that helper is shared with the path-based `crossfade`, so weigh the cross-impact.
- **SFX voice ring can cut a long bus pad.** The 16-voice ring is shared across `play_sfx` and `play_sfx_on_bus`; firing 16+ one-shots while a long `play_sfx_on_bus` pad (e.g. the example's 4 s "bed") is playing will wrap around and cut it. Fine for the demo; a game wanting a guaranteed-sustained bus bed should use a dedicated channel (or a future `play_music`-style sustained-bus API).

## Open Questions

- **None for the facade** — done + merged.
- **Direction for next session:** EW request (board empty now) vs a carried direction (RonRegistry-pub / facade positional / HDR-RT / adopt-facade-in-a-game) vs a fresh feature. The user drives this at the next seam.
- **Should the facade gain `suspend()` for symmetry with `resume()`?** Deferred this session — a native global `suspend` (pause all sinks) is real work and a native no-op `suspend` would be misleading (unlike `resume`, which is a coherent "ensure running" no-op on native). If a game needs game-pause-mutes-audio cross-platform, this is the call to make (add a native pause-all to `AudioManager` first, then expose it on the facade).

## Quick Start for Next Session

```bash
# Sync + verify clean/green
git checkout main && git pull --ff-only        # expect main @ f3f136f or later (this handoff's docs PR)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?" >> /tmp/verify.log; grep VERIFY_EXIT /tmp/verify.log   # must be 0 — read it from the LOG (a trailing echo masks the real exit)

# The audit backlog is closed; check the game wishlist board FIRST (top driver)
cat ../dungeon-merchant/docs/engine-wishlist.md    # ACTIVE empty, next ID EW-002

# Live engine state + gotchas
#   memory engine-current-state (seq 69) + MEMORY.md index

# Key files to read first (the facade just shipped):
#   src/audio_facade.rs            — the Audio facade + AudioFacadeSystem
#   src/audio/playback.rs          — play_bytes/crossfade_bytes + append_decoded/begin_crossfade
#   examples/audio_facade/         — the native+web example + web/ harness
#   src/audio_wasm.rs / src/audio.rs — the two backends the facade wraps

# Try the web demo by ear (optional, not done this session):
#   examples/audio_facade/web/build.sh && python3 -m http.server 8080 --directory examples/audio_facade/web   # click Start, press keys

# Next action
#   Read the wishlist board. If an EW request exists, do that. Otherwise ASK the user which
#   carried direction (RonRegistry pub / facade positional audio / HDR render-target /
#   adopt the facade in settings_menu to delete its 16 cfg-guards / fresh feature).
```

---

## Session Closed

**Closed at:** 2026-06-22
**Session status:** Handed off to next session.
**Code work:** the `Audio` facade landed via PR **#197** (v0.51.0, merge commit `f3f136f`) — already on main before this handoff.
**Landed:** this handoff doc lands on `main` via its own `docs(handoff)` PR (matching seq-2 #187, seq-3 #192, seq-4 #194, seq-5 #196). Memory `engine-current-state` is at seq 69; `MEMORY.md` index refreshed. The seq-1 engine-audit deferred list (items 1–8) remains fully closed; this seq 6 is the first post-audit feature (the item-5 reframe).
