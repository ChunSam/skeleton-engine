# Named tone channels + low-pass on the `Audio` facade; `settings_menu` adopts it (v0.53.0)

**Date:** 2026-06-22
**Status:** COMPLETED + merged. main @ `22e4108`, package **v0.53.0**, clean tree, full gate green, CI green, squash-merged (#201).
**Bead(s):** none (bd unavailable in this environment)
**Epic:** post-audit feature work (VISION feature+example loop) — the carried-direction roadmap P1→P4
**Chain:** `standalone-4365aa4a` seq `8`
**Parent:** `HANDOFF_facade-play-tone_2026-06-22.md` (seq 7)
**Auto:** false (driven by a `/goal`: do carried directions P1→P4 in order, each phase test→handoff→merge)

> NB the memory `engine-current-state` uses a *different* (engine-wide) seq counter: this same work is **seq 71** there. This is **P1** of the four carried audio/engine directions the user set as a goal this session.

---

## The Goal

P1 of the session goal (`/goal`: "p1 부터 순서대로 진행, 각 페이즈 완료 후 테스트 후 handoff 하고 머지"). The carried direction **P1 = adopt the facade in `settings_menu`** — the last audio example game still split native-vs-wasm. It needed three facade capabilities seq 6/7 had excluded: a **sustained, named, queryable tone channel** + a cross-platform **low-pass filter**. This session grew the facade with named tone channels (`play_tone_on_channel` / `is_channel_playing` / `set_low_pass` / `clear_low_pass`), wired the web backend (`OscillatorNode` per channel + `BiquadFilterNode`), and adopted `settings_menu` — closing the last audio-cfg game.

## Where We Are

- **main @ `22e4108`, package v0.53.0, CLAUDE.md header v1.6.122, clean tree, `./scripts/verify.sh` → exit 0** (fmt + clippy `-D warnings` native + wasm32 lib/bins build + `test --all-targets` + doc `-D warnings`).
- **PR #201 squash-merged** on green CI. Branch `feat/facade-tone-channels` deleted, local main fast-forwarded.
- **New facade API** (`src/audio_facade.rs`):
  - `play_tone_on_channel(channel, freq, dur, vol, bus)` — a synthesized tone on a caller-named channel routed through a named bus; a replay on the same channel cuts the previous tone.
  - `is_channel_playing(channel) -> bool` — `&self`; re-arm a sustained tone when it drains.
  - `set_low_pass(channel, cutoff_hz)` / `clear_low_pass(channel)` — low-pass, applied on the **next** play (mirrors native `AudioEffect` semantics).
- **Native arm**: delegates to `AudioManager` — `assign_bus(channel, bus)` + `play_tone(channel, ...)` / `is_playing(channel)` / `set_effect(channel, AudioEffect { low_pass_hz: Some(hz), ..Default::default() })` / `clear_effect(channel)`. Imported `crate::audio::AudioEffect` (native-gated).
- **Web backend** (`src/audio_wasm.rs`): added `tone_channels: Rc<RefCell<HashMap<String, ToneVoice>>>` (osc + scheduled `stop_time`) + `tone_lowpass: Rc<RefCell<HashMap<String, f32>>>` (pending cutoff). `play_tone_on_channel` cuts any prior osc on the channel, wires `osc → [biquad lowpass] → gain → dest`, schedules the same click-free attack/release envelope as `play_tone_to`, records the osc + stop time. `is_channel_playing` = `ctx.current_time() < stop_time`. `set_low_pass`/`clear_low_pass` update the pending map (applied on next play, matching native).
- **Cargo.toml**: `web-sys` features gained `"BiquadFilterNode"`, `"BiquadFilterType"`.
- **`examples/games/settings_menu`** adopted the facade: 5 audio helpers (`set_bus`/`blip`/`play_bgm`/`keep_bgm`/`set_muffle`) + init + the 3 BGM constants un-gated; `AudioManager`/`AudioEffect` import dropped, `Audio` added. **Audio cfg-guards: gone** (the one remaining `#[cfg(not(wasm32))]` is `env_logger::try_init`, non-audio). Builds + plays audio on the web now.
- **`examples/audio_facade`** gained `G` (toggle a sustained two-tone named-channel BGM, kept alive via `is_channel_playing`) + `L` (toggle the low-pass muffle) — the new API is hearable on the web via the existing `web/` harness; window grew 300→340 px to fit the new legend block.
- **Docs/paperwork:** CLAUDE.md audio row (3 edits: WebAudio named channels, facade coverage list, exclusion narrowed + `settings_menu` added to adopters) + header bump; CHANGELOG 0.53.0; Cargo.lock → 0.53.0.
- **Memory** `engine-current-state` bumped to seq 71; `MEMORY.md` index refreshed.

## What We Tried (Chronological)

1. **Onboarding** — synced main @ `68024df` (v0.52.0), verify exit 0, wishlist board empty (next ID EW-002), read seq-7 handoff + memory. (Earlier this session: recorded two rule memories [`cargo-fmt-reflow-trap`, `wasm-gate-excludes-examples`] + added a "Handoff mode" section to the `/land-pr` skill, from the wrap analysis.)
2. **Set the goal** — user picked the P1→P4 carried-direction order via `/goal`, autonomous per-phase test→handoff→merge.
3. **Explored both audio backends + settings_menu's real API.** Confirmed `settings_menu` uses `play_tone` on *named* channels ("bgm_low"/"bgm_high"/"ui"), `is_playing`, and `set_effect`/`clear_effect` (low-pass) — none on the facade. Designed the named-tone-channel API as the minimal honest cross-platform cover.
4. **Implemented** — Cargo.toml features → WebAudio named channels + BiquadFilter → facade 4 methods + docs → settings_menu adoption → audio_facade G/L demo → CLAUDE.md.
5. **First verify → `VERIFY_EXIT=101`** at the rustdoc step: `redundant explicit link target` on `[`AudioEffect`](crate::audio::AudioEffect)` (AudioEffect is in scope, so the explicit target is redundant under `-D warnings`). Fixed to the shorthand `[`AudioEffect`]`. (The fmt trap did **not** bite this time — ran `cargo fmt` first per the new memory.)
6. **Re-verify → exit 0.** Then `cargo build --example settings_menu_game --example audio_facade --target wasm32` → exit 0 (the new `OscillatorNode`/`BiquadFilterNode` example paths; the standard gate is lib+bins only — per the `wasm-gate-excludes-examples` memory).
7. **Native real-play smoke** (`audio_facade`, keys G/L/T via `osascript` key codes G=5, L=37, T=17) — process alive after real rodio playback, stderr 0 bytes, screenshot readout `BGM: on`.
8. **`/ship` paperwork** (Cargo.toml 0.52→0.53, lock, CHANGELOG 0.53.0, CLAUDE.md header + audio row), re-verify exit 0.
9. **`/land-pr`** — commit `ccdddb3`, push, PR #201, watch CI, squash-merge, sync.

## Key Decisions

- **Named tone channels are the honest minimal cover.** The facade is deliberately channel-less (bytes-keyed, fire-and-forget). `settings_menu` needs a tone it can *query* (`is_playing`) and *filter* (low-pass) — that requires a stable name. So the facade gains an explicit *named-channel* tone API (vs the round-robin `play_tone`), documented as "for sounds you track."
- **Low-pass only, not the full `AudioEffect`.** Native `AudioEffect` also has pitch/attack/release; web has no cheap equivalent. Exposing only `set_low_pass`/`clear_low_pass` keeps the facade an honest intersection (both backends do low-pass cleanly: rodio `low_pass` / Web Audio `BiquadFilterNode`). Pitch/attack/release stay native-only.
- **"Applied on next play" semantics mirrored on web.** Native `set_effect` applies on the next `play_*`. The web side stores a pending per-channel cutoff and inserts the `BiquadFilterNode` when the channel is next (re)played — byte-for-byte the same toggle-then-replay flow `settings_menu` already used.
- **No `AudioFacadeSystem` in `settings_menu`.** It uses no fades/ducks/crossfade, so `update()` is a pure no-op — the original never ticked `AudioSystem` either. Skipped it (also sidesteps the scene-reset-vs-system question, since systems are re-registered per scene via `SystemRegistrar`).
- **`settings_menu` reaches "audio cfg-guard-free", not literally 0 cfg.** The lone remaining `#[cfg(not(wasm32))]` wraps `env_logger::try_init` (native logging, not a wasm dep). Honest result; documented.
- **Extended `audio_facade` for web-by-ear coverage.** `settings_menu` has no web harness, so its BiquadFilter path wouldn't be heard. `audio_facade` *does* have one — adding G/L there exercises the new low-pass on the web (the VISION "exercise it in real play" on both platforms).
- **MINOR bump v0.53.0** (pre-1.0: additive feature = MINOR).

## Reusable Gotchas & Patterns (carry forward)

- **`redundant explicit link target` (rustdoc `-D warnings`).** When the linked item is already in scope (imported), use the shorthand `[`Foo`]`, not `[`Foo`](path::Foo)` — the explicit path is redundant and rustdoc errors under `-D warnings`. Bit at the doc step this session (the only verify failure).
- **The fmt-reflow trap did NOT bite** — ran `cargo fmt` before the gate per the new `cargo-fmt-reflow-trap` memory. The memory paid off immediately.
- **New `web_sys::Foo` ⇒ add its feature** — `BiquadFilterNode` + `BiquadFilterType` to `web-sys` features in Cargo.toml (same pattern as `OscillatorNode` in seq 7). Without them the wasm build fails E0599 on `create_biquad_filter`/`set_type`.
- **`cutoff.and_then(|hz| { let biquad = ctx.create_biquad_filter().ok()?; ...; Some(biquad) })`** — a clean "build-an-optional-node-or-fall-back" idiom: the whole filter insertion is `Option<BiquadFilterNode>`, and the graph wires `osc → biquad → gain` or `osc → gain` by matching on it.
- **web-sys node up-casts via `.into()`** — `OscillatorNode`/`BiquadFilterNode` → `AudioNode` (parent class) for a uniform `connect_with_audio_node` call site; `&GainNode` coerces to `&AudioNode` via Deref at the call.
- **macOS synthetic key codes** (extends the running list): `G = 5`, `L = 37` (M=46, Digit1=18, T=17, ArrowDown=125/Up=126, Esc=53, B=11).

## Files Changed

### Source
- `src/audio_wasm.rs` — `WebAudio`: `tone_channels` + `tone_lowpass` fields + `ToneVoice` struct; `play_tone_on_channel` + private `play_tone_channel_to` (osc + optional biquad + envelope) + `is_channel_playing` + `set_low_pass` + `clear_low_pass`.
- `src/audio_facade.rs` — `Audio`: `play_tone_on_channel` / `is_channel_playing` / `set_low_pass` / `clear_low_pass`; native `AudioEffect` import; module docs (coverage list + narrowed exclusion).
- `Cargo.toml` — `web-sys` features `+ "BiquadFilterNode", "BiquadFilterType"`; version 0.52.0 → 0.53.0.

### Examples
- `examples/games/settings_menu/settings_menu.rs` — facade adoption; audio cfg-guards gone (env_logger native-only remains).
- `examples/audio_facade/audio_facade.rs` — `G`/`L` keys (named-channel BGM + low-pass), window 300→340 px.

### Docs / release paperwork
- `CLAUDE.md` — audio row (3 edits) + header v1.6.122 + package v0.53.0.
- `docs/CHANGELOG.md` — 0.53.0 entry.
- `Cargo.lock` — → 0.53.0.

### Memory (outside repo)
- `engine-current-state.md` → seq 71; `MEMORY.md` index line refreshed.

## User Feedback & Preferences

- **`/goal`**: do carried directions **P1→P4 in order**, each phase **test → handoff → merge**, **final report only** (skip intermediate reports). Autonomous — drive the whole chain without per-step confirmation.
- Standing **merge-authority delegation** (squash on green CI). Applied.
- **Conventions**: user-facing reports in Korean, all artifacts/prompts/code/docs in English. This file is English.
- Values **evidence-first** + catching verify traps (real `VERIFY_EXIT` from the log, the wasm-example build beyond the gate, the native real-play smoke).

## Where We're Going

- **P1 done + merged.** Continuing the same goal: **P2 = positional audio on the facade** next (the part excluded since seq 6 — a cross-platform handle over native channel `update_position` vs wasm `Sfx`), then **P3 = `RonRegistry<V>` + `RonLoadable` pub at crate root**, then **P4 = HDR/linear render-target**. Each: implement → verify (native gate + wasm-example builds + smoke where applicable) → handoff → merge.
- **Carried, still open after P1:** P2, P3, P4 (above). Wishlist board EMPTY (next ID EW-002) — a new EW request would still preempt.

## Risks & Blockers

- **None blocking.** main clean + green at v0.53.0.
- **Web low-pass untested by ear** — the `BiquadFilterNode` path compiles (wasm lib + example builds) and is a thin standard Web Audio call, but no one has *heard* the muffle in a browser this session. Low risk; run `examples/audio_facade/web/build.sh` + serve + press `G` then `L`.
- **`is_channel_playing` on web is time-based** (`current_time() < stop_time`), not a real `onended` callback. For a *re-armed* sustained tone (the only use) this is exact; a game that lets a tone finish and never re-arms will read `false` correctly. No callback bookkeeping needed.

## Quick Start for Next Session (→ P2)

```bash
git checkout main && git pull --ff-only        # expect main @ the seq-8 handoff docs PR or later
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?" >> /tmp/verify.log; grep VERIFY_EXIT /tmp/verify.log

# P2 = positional audio on the Audio facade. Read first:
#   src/audio_facade.rs                 — the facade (now incl. named tone channels)
#   src/audio_wasm.rs : play_at / Sfx   — web positional + the Sfx handle (set/update position)
#   src/audio/positional.rs             — native AudioManager::play_at + channel update_position
# Design: a cross-platform positional handle over native channel update_position vs wasm Sfx.
```

---

## Session Closed (P1)

**Closed at:** 2026-06-22
**Code work:** named tone channels + low-pass on the `Audio` facade + `settings_menu` adoption landed via PR **#201** (v0.53.0, merge `22e4108`).
**Landing:** this handoff lands on `main` via its own `docs(handoff)` PR. Memory `engine-current-state` is at seq 71; `MEMORY.md` refreshed. Continuing to **P2**.
