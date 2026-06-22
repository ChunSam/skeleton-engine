# Tracked 2D positional sound on the `Audio` facade + `positional_audio` example (v0.54.0)

**Date:** 2026-06-23
**Status:** COMPLETED + merged. main @ `9c413a8`, package **v0.54.0**, clean tree, full gate green, CI green, squash-merged (#203).
**Bead(s):** none (bd unavailable in this environment)
**Epic:** post-audit feature work — the `/goal` P1→P4 carried-direction run (this = **P2**)
**Chain:** `standalone-4365aa4a` seq `9`
**Parent:** `HANDOFF_facade-tone-channels-lowpass_2026-06-22.md` (seq 8, P1)
**Auto:** false (driven by the P1→P4 `/goal`: each phase test→handoff→merge)

> NB the memory `engine-current-state` uses a *different* (engine-wide) seq counter: this same work is **seq 72** there.

---

## The Goal

P2 of the session goal: the carried direction **positional audio on the facade** — the part excluded since the facade landed (seq 6). A dual-target game wanting a 2D positional sound had to write the native/wasm split by hand. This session added a cross-platform **tracked positional channel** to the facade and an example exercising it on both platforms.

## Where We Are

- **main @ `9c413a8`, package v0.54.0, CLAUDE.md header v1.6.123, clean tree, `./scripts/verify.sh` → exit 0.**
- **PR #203 squash-merged** on green CI. Branch `feat/facade-positional` deleted, local main fast-forwarded.
- **New facade API** (`src/audio_facade.rs`):
  - `play_at_on_channel(channel, bytes, source, listener, max_dist, bus)` — a **looping** 2D positional sound on a caller-named channel (distance volume + stereo pan applied immediately).
  - `update_position(channel, source, listener, max_dist)` — reposition each frame. **No `cfg` split** — both backends expose `update_position` with the same signature, so the facade body is a single `self.inner.update_position(...)`.
  - `stop_channel(channel)` — stop a positional sound and/or a named tone on the channel.
- **Native** (`src/audio/positional.rs`): new `AudioManager::play_bytes_at(channel, bytes, repeat, source, listener, max_dist)` — the **byte-based** analogue of the path-based `play_at` (sets `volume_overrides`+`pans` from `spatial_params`, then `play_bytes`). The facade's native `play_at_on_channel` = `assign_bus` + `play_bytes_at(.., repeat=true, ..)`; `stop_channel` = `AudioManager::stop`. `update_position` already existed.
- **Web** (`src/audio_wasm.rs`): new `spatial_channels: Rc<RefCell<HashMap<String, Sfx>>>` field. `play_sfx_to` refactored to delegate to `play_sfx_to_opts(bytes, dest, repeat)` (adds `src.set_loop(repeat)`); the existing callers pass `false`. `play_at_on_channel` builds a **looping** `Sfx` via `play_sfx_to_opts(.., true)`, applies the spatial params, stores it under the channel; `update_position` looks up the `Sfx` and calls `Sfx::update_position`; `stop_channel` removes+stops the spatial `Sfx` **and** any tone voice on that channel (mirrors native's uniform `stop`).
- **Example** `examples/positional_audio.rs` (flat, native + wasm entry) — an orbiting sound source (ellipse), arrow-key-movable listener, live volume/pan ASCII meters + numeric readout. Window 760×360.
- **Docs/paperwork:** CLAUDE.md audio row (3 edits) + header bump; CHANGELOG 0.54.0; Cargo.lock → 0.54.0.
- **Memory** `engine-current-state` → seq 72; `MEMORY.md` index refreshed.

## What We Tried (Chronological)

1. **Explored both positional paths.** **Key mismatch:** native `AudioManager::play_at` reads a **file path** (`play_at(channel, path, repeat, ...)` → `play`), while wasm `WebAudio::play_at` takes **bytes** and returns an `Sfx`. The facade is bytes-keyed → native needed a new `play_bytes_at` (path-free), and wasm needed channel-addressing over its handle-based `Sfx`.
2. **Designed the named-channel positional API** — consistent with the seq-8 named-tone-channel pattern (a tracked sound the game addresses by name). Looping by default (the tracked use-case is a sustained emitter).
3. **Implemented** — native `play_bytes_at` → wasm `spatial_channels` + `play_sfx_to_opts(repeat)` + `play_at_on_channel`/`update_position`/`stop_channel` → facade 3 methods + `Vec2` import + module docs → `positional_audio` example.
4. **Built the example** (caught nothing — `is_pressed`/`KeyCode::Space`/`Vec2::distance` all valid) → full verify gate **exit 0** (fmt trap did not bite — `cargo fmt` first).
5. **wasm example build** `cargo build --example positional_audio --example audio_facade --target wasm32` → exit 0 (the web positional path compiles).
6. **Native real-play smoke** — first run's `keystroke`/`key code` Space didn't focus the winit window (readout froze at the angle=0 orbit value, `vol 0.22`); re-ran with `set frontmost` + `keystroke " "` + a region-capture (`screencapture -R<x,y,w,h>` from the window bounds) → **`playing: yes`, `last: play_at_on_channel (looping, orbiting)`, source orbited to (130,148), pan −0.78** — the looping positional sound played via rodio + `update_position` tracked each frame, process alive, stderr empty.
7. **`/ship`** (0.53→0.54, lock, CHANGELOG, CLAUDE.md), re-verify exit 0. **`/land-pr`** — commit `b2ab955`, push, PR #203, CI, squash-merge, sync.

## Key Decisions

- **Channel-named, looping positional — not a returned handle.** Consistent with the seq-8 named-tone-channel API: the game addresses a tracked sound by a stable name (`update_position(channel, ..)`), rather than holding a cross-platform handle (awkward on native, where updating needs `&mut AudioManager`). Looping by default because a *tracked* positional sound is a sustained emitter (engine hum, fire); an untracked one-shot doesn't need tracking and stays native-only (`play_at`).
- **New native `play_bytes_at` rather than reusing `play_at`.** `play_at` is path-based (no wasm filesystem); the facade is bytes-keyed. `play_bytes_at` mirrors `play_at` exactly but ends in `play_bytes` — additive, the path version untouched.
- **`update_position` needs no `cfg` split.** Both `AudioManager::update_position` and `WebAudio::update_position` have the identical `(channel, source, listener, max_dist)` signature, so the facade method is a single un-guarded `self.inner.update_position(...)` — the cleanest possible facade method.
- **`play_sfx_to` → `play_sfx_to_opts(repeat)` refactor** over duplicating the decode/wire closure. The looping path is one `src.set_loop(repeat)` line; existing callers get a thin `repeat=false` wrapper (behavior-preserving).
- **`stop_channel` stops both a positional `Sfx` and a tone voice on the name** (web), mirroring native `stop`'s uniform channel teardown — so a game can `stop_channel("x")` regardless of what kind of sound `"x"` is.
- **Flat example, native+web entry, no web harness this session.** The example wasm-builds + native-smokes; a browser `web/` harness is a separate `/ship-wasm-example` pass (deferred — the positional web path is a thin standard Web Audio panner+loop, low risk). Matches the seq-7 precedent (web-by-ear optional).
- **MINOR bump v0.54.0** (additive).

## Reusable Gotchas & Patterns (carry forward)

- **macOS synthetic-key focus gotcha (cost a wasted smoke):** `osascript ... set frontmost` alone did **not** give the winit window keyboard focus for the first `key code` — the demo's state didn't change (a stuck readout is the tell). Re-running with `set frontmost` **then** `keystroke " "` worked; if a readout looks frozen at its initial value, the key never landed. Region-capture the window cleanly with `screencapture -x -o -R<x>,<y>,<w>,<h>` using the bounds from `get {position, size} of front window` — far more legible than cropping the full-desktop shot. macOS key codes: **Space=49, S=1, Left=123, Right=124, Up=126, Down=125**.
- **A "frozen at the initial value" readout = the input never registered.** The orbit at angle=0 gives `vol 0.22`; seeing exactly that after "pressing Space" meant the orbit never advanced (Space dropped), not that the audio failed. Pick example readouts that *move* so a no-op input is visible.
- **Native `play_at` is path-based, wasm `play_at` is bytes-based** — the backends diverge on the clip source for positional too; the facade (bytes) needs a `play_bytes_at` on native. (Same shape as the seq-6 `play_bytes` need.)
- **The fmt-reflow trap did NOT bite again** — `cargo fmt` before the gate ([[cargo-fmt-reflow-trap]]). No verify failures this phase.

## Files Changed

- `src/audio/positional.rs` — `AudioManager::play_bytes_at` (additive).
- `src/audio_wasm.rs` — `spatial_channels` field + init; `play_sfx_to` → `play_sfx_to_opts(repeat)`; `play_at_on_channel` / `update_position` / `stop_channel`.
- `src/audio_facade.rs` — `Audio::play_at_on_channel` / `update_position` / `stop_channel`; `glam::Vec2` import; module docs (coverage + narrowed exclusion).
- `examples/positional_audio.rs` — new (flat, native + wasm entry).
- `CLAUDE.md` (audio row 3 edits + header v1.6.123 + v0.54.0), `docs/CHANGELOG.md` (0.54.0), `Cargo.lock`.
- Memory `engine-current-state` → seq 72; `MEMORY.md`.

## Where We're Going

- **P2 done + merged.** Continuing the `/goal`: **P3 = `RonRegistry<V>` + `RonLoadable` pub at crate root** next (the carried bonus from seq 2 — `src/ron_registry.rs` is crate-internal; expose it as a fork-friendly custom-asset registry + a tiny example/doc). Then **P4 = HDR/linear render-target**.
- **Optional follow-up:** ship `positional_audio` to the web (`/ship-wasm-example`) so the positional panner is hearable in a browser.

## Risks & Blockers

- **None blocking.** main clean + green at v0.54.0.
- **Web positional untested by ear** — the path compiles (wasm lib + example builds) and is a thin Web Audio panner+loop, but no browser run this session. Low risk.
- **Looping-only positional on the facade.** A positional *one-shot* (explosion at a point, not tracked) is not on the facade — use the backend `play_at` directly (documented). Acceptable: tracked sounds are the facade's remit.

## Quick Start for Next Session (→ P3)

```bash
git checkout main && git pull --ff-only        # expect the seq-9 handoff docs PR or later
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?" >> /tmp/verify.log; grep VERIFY_EXIT /tmp/verify.log

# P3 = expose RonRegistry<V> + RonLoadable at the crate root. Read first:
#   src/ron_registry.rs   — the crate-internal generic registry (RonRegistry<V> + RonLoadable)
#   src/lib.rs            — re-export site (currently `mod ron_registry;` not re-exported)
#   src/particle/config_set.rs / src/dialogue/tree.rs / src/animation/clip_set.rs — its 3 current users (the wrappers)
# Decide: make RonRegistry<V>/RonLoadable pub + documented as a fork-friendly custom-asset registry;
# consider a small example loading a custom RON config type via it. Native-gated (load/reload are).
```

---

## Session Closed (P2)

**Closed at:** 2026-06-23
**Code work:** tracked 2D positional on the `Audio` facade + `positional_audio` example landed via PR **#203** (v0.54.0, merge `9c413a8`).
**Landing:** this handoff lands on `main` via its own `docs(handoff)` PR. Memory `engine-current-state` is at seq 72. Continuing to **P3**.
