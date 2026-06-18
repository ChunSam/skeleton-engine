# Session wrap — 8 releases (v0.32.0→v0.39.0): wasm audio native-parity + UI/tilemap stretch trio

**Date:** 2026-06-18
**Status:** COMPLETED — all in-flight work merged + tagged, `main` clean & green
**Bead(s):** none (repo uses `plans/handoffs/`, not beads)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `38`
**Parent:** `HANDOFF_engine-hardening_stretch-trio_2026-06-18.md` (seq 37)
**Prior chain (this session):** seq 33 (`session-wrap`, the paste prompt that opened this session) > 34 `wasm-audio-depth` > 35 `wasm-audio-parity` > 36 `wasm-positional-bus` > 37 `stretch-trio` > **38 this (session wrap)**

> This is a **session-level wrap** tying together the four per-feature handoffs written this session
> (seq 34–37). Each shipped feature has its own detailed handoff with full code-level notes; this file
> is the index + arc synthesis + cross-cutting lessons. For granular detail on any one release, read
> its handoff (mapped in **Evidence & Data**).

---

## Since Last Handoff (the whole session vs the seq-33 close)

This session **started** from the seq-33 session-wrap paste prompt, which had closed the prior arc
(v0.24.0→v0.31.0, 8 releases) and listed exactly three open backlog items: "further wasm audio
(named buses / wasm crossfade)", "crates.io publish", and a stretch list (gamepad UI focus / flat-top
hex / autotile across iso+hex / focus-ring styling).

The user drove the session item-by-item off that list:
1. **"further wasm audio"** → shipped as two releases (v0.32.0 buses, v0.33.0 crossfade) — seq 34.
2. **"1번 진행" (remaining native-only audio)** → ducking (v0.34.0) + positional (v0.35.0) — seq 35.
3. **"2진행해" → "positional play_at_on_bus"** (a stretch pick) — v0.36.0 — seq 36.
4. **"1.2.3 모두 실행"** (the three remaining stretch items) → gamepad focus (v0.37.0), flat-top hex
   (v0.38.0), autotile iso+hex (v0.39.0) — seq 37.

Net: **wasm `WebAudio` reached native-`AudioManager` parity** (only auto-sidechain left, by design),
and the **seq-35 stretch list was fully drained**. crates.io publish remains the one untouched item.

## The Goal

skeleton-engine is a fork-friendly, MIT, genre-agnostic 2D engine (VISION: "the example is the
acceptance test"). This session's goal was to work down the post-roadmap backlog the user selected
one item at a time. End state: every requested item shipped as its own CI-green, merged, tagged 0.x
MINOR release with an example/tests + a per-feature handoff; the wasm audio path brought to feature
parity with native; the stretch list cleared.

## Where We Are

- `main` @ `a04c670` (seq-37 handoff merge; last *feature* merge `0981011` = v0.39.0), package
  **v0.39.0**, CLAUDE.md header **v1.6.88**, tree clean, CI green.
- **12 PRs merged + tagged/closed this session** — 8 feature releases (#121–#131) + 4 handoff PRs
  (#123/#126/#128/#132). Tags `v0.32.0`–`v0.39.0` all pushed (annotated, on their merge commits).
- Two arcs: **wasm audio** (v0.32–v0.36, 5 releases) and **UI/tilemap stretch** (v0.37–v0.39, 3).
- Verification split by domain (see Key Decisions): audio = headless wasm smoke (grew **12 → 38**);
  UI/tilemap = unit tests + the verify gate (focus 8, tilemap 69, autotile 31).

## What We Did (chronological, one row per release)

See **Evidence & Data** for the release table. Arc in brief:
1. **v0.32.0 (#121, seq 34)** — wasm **named mixer buses**: a bus = a named `GainNode` wired
   `bus → master`; `set_bus_volume`/`bus_volume`/`bus_names` + `play_on_bus`/`play_sfx_on_bus`.
2. **v0.33.0 (#122, seq 34)** — wasm **track-to-track crossfade**: music routes through a per-track
   `GainNode`; `crossfade_music` ramps old→0 + new 0→1 on the audio clock (`MusicChannel` struct).
3. **v0.34.0 (#124, seq 35)** — wasm **bus ducking**: bus became a 2-gain chain `duck → volume →
   master`; `duck_bus`/`release_bus`/`bus_duck`. Auto-sidechain deliberately NOT ported.
4. **v0.35.0 (#125, seq 35)** — wasm **2D positional**: `play_at` + `Sfx::update_position` +
   `Sfx::volume`/`pan` getters + `spatial_params` (native parity).
5. **v0.36.0 (#127, seq 36)** — wasm **`play_at_on_bus`**: positional routed through a named bus
   (a tiny composition of `play_sfx_on_bus` + `update_position`).
6. **v0.37.0 (#129, seq 37)** — **gamepad UI focus nav**: first connected pad folded into the focus
   pass (D-pad cycle/slider, A activate); `GamepadState::test_press` test helper.
7. **v0.38.0 (#130, seq 37)** — **flat-top hex** `TilemapProjection::HexagonalFlat` (odd-q, 90°-mirror
   of pointy odd-r); example `hex_tilemap_flat` + generated atlas.
8. **v0.39.0 (#131, seq 37)** — **autotile across iso+hex**: iso already worked (square topology,
   tested); hex got `Neighborhood::Hex6`/`Hex6Flat` parity-aware masks + `hex_6`/`hex_6_flat`.

## Key Decisions

- **Each item = its own MINOR release + PR + tag + per-feature handoff**, not one mega-PR — matches
  the repo's 0.x cadence and the user's item-by-item "test+merge" rhythm.
- **Branch each feature off *fresh main* AFTER the prior PR squash-merged** — squash diverges the
  base, so starting the next before the prior merged would cause dirty 3-way diffs. Cost: serial
  merges (each ~5–6 min CI); benefit: clean PRs. Shaped the whole session's pacing.
- **Auto-sidechain stays native-only (honest scope cut)** — it needs continuous per-frame "is the
  trigger bus playing?" evaluation, a poor fit for Web Audio's fire-and-forget model (and music
  isn't bus-routed on wasm). Manual `duck_bus`/`release_bus` covers the practical need. Surfaced to
  the user rather than shipping fragile `onended`-counting code.
- **Verification by domain.** Audio: the headless wasm smoke is the test (no native `cargo test` for
  the wasm path). UI/tilemap: unit tests + verify gate, because the windowed screencapture playtest
  came up **blank** (shell-launched GUI quirk) — so visual features leaned on unit-tested math + the
  already-proven shared render path. Named honestly in each handoff.
- **Hex autotile demo with 2 tiles, not 64** — an interior-vs-edge `mask_to_tile` demonstrates the
  `Hex6` neighbor logic without a 64-tile atlas; `hex_6` (the 64-tile constructor) is still provided
  + unit-tested for discoverability.

## Evidence & Data

### Releases shipped this session
| seq | ver | feat PR | merge | item | verify | tests / smoke |
|---|---|---|---|---|---|---|
| 34 | 0.32.0 | #121 | `b62b993` | wasm named mixer buses | green | audio smoke 19/19 |
| 34 | 0.33.0 | #122 | `db1ea90` | wasm track-to-track crossfade | green | audio smoke 22/22 |
| 35 | 0.34.0 | #124 | `69f3b9f` | wasm bus ducking | green | audio smoke 28/28 |
| 35 | 0.35.0 | #125 | `5abbcae` | wasm 2D positional | green | audio smoke 35/35 |
| 36 | 0.36.0 | #127 | `a64ba75` | wasm play_at_on_bus | green | audio smoke 38/38 |
| 37 | 0.37.0 | #129 | `d2c308e` | gamepad UI focus nav | green | focus 8 (+3) |
| 37 | 0.38.0 | #130 | `9aaf3a8` | flat-top hex HexagonalFlat | green | tilemap 69 (+4) |
| 37 | 0.39.0 | #131 | `0981011` | autotile across iso+hex | green | autotile 31 (+8) |

### Per-feature handoffs (navigation index — full code-level detail)
| seq | handoff file | covers |
|---|---|---|
| 34 | `HANDOFF_engine-hardening_wasm-audio-depth_2026-06-18.md` | v0.32.0 buses + v0.33.0 crossfade |
| 35 | `HANDOFF_engine-hardening_wasm-audio-parity_2026-06-18.md` | v0.34.0 ducking + v0.35.0 positional |
| 36 | `HANDOFF_engine-hardening_wasm-positional-bus_2026-06-18.md` | v0.36.0 play_at_on_bus |
| 37 | `HANDOFF_engine-hardening_stretch-trio_2026-06-18.md` | v0.37.0 + v0.38.0 + v0.39.0 |

### Backlog status (the seq-33 list)
| item | status |
|---|---|
| further wasm audio (named buses / crossfade) | ✅ shipped (#121, #122) |
| remaining native-only audio: ducking | ✅ shipped (#124) |
| remaining native-only audio: positional | ✅ shipped (#125) |
| positional on a bus (`play_at_on_bus`) | ✅ shipped (#127) |
| gamepad UI focus nav | ✅ shipped (#129) |
| flat-top hex | ✅ shipped (#130) |
| autotile across iso+hex | ✅ shipped (#131) |
| automatic sidechain on wasm | ⛔ deliberately native-only (poor fit; documented) |
| crates.io publish | ⬜ deferred (irreversible, needs explicit go) |

## Code Analysis (cross-cutting facts worth keeping)

- **`WebAudio` graph (`src/audio_wasm.rs`, wasm-only)** now: a sound → (optional `StereoPannerNode` +
  per-source `GainNode`) → [bus `duck` `GainNode` → bus `volume` `GainNode` →] master `GainNode` →
  destination. Music → per-track `GainNode` → master. Buses are `Bus { volume, duck }`; the current
  music is `MusicChannel { source, gain }`.
- **One ramp helper** `ramp_gain_to(gain, target, dur)` drives crossfade + ducking. `dur <= 0` =
  instant `set_value` (cancel + write — directly readable); `dur > 0` = `set_value_at_time(current) +
  linear_ramp_to_value_at_time(target)`. The instant path exists BOTH for semantics (0s = instant)
  AND headless testability (gotcha #1).
- **`spatial_params(source, listener, max_dist)`** (free fn): `vol = (1 - clamp(dist/max)).max(0)`,
  `pan = clamp(dx/max, -1, 1)` — shared by `play_at`/`play_at_on_bus`/`Sfx::update_position`.
- **No new web-sys feature added all session** — `AudioParam::{set_value_at_time,
  linear_ramp_to_value_at_time, cancel_scheduled_values, value, set_value}`, `AudioContext::
  current_time`, `AudioBufferSourceNode::stop_with_when` all ride already-enabled features.
  `stop_with_when` is under the same `#[allow(deprecated)]` as the existing `stop*` calls.
- **UI gamepad focus** (`src/ui/system/state.rs`): `InputSnapshot::from_world` ORs the first connected
  `GamepadState` pad into the keyboard snapshot — D-pad Down/Up → `tab` (+`shift` for Up), D-pad
  Left/Right → `nav_left/right`, South → `activate`. The focus pass (`focus_pass.rs`) is unchanged.
- **Tilemap projections** (`src/tilemap/mod.rs`): `TilemapProjection {Orthographic, Isometric,
  Hexagonal, HexagonalFlat}` (plain enum, NOT `#[non_exhaustive]` — adding a variant is a 0.x MINOR;
  4 internal matches must all be updated: `cell_center_world`/`cell_at_world`/`cell_z`/
  `cell_render_size`). HexagonalFlat = flat-top odd-q (`ts` = flat-to-flat height; sprite
  `ts·2/√3 × ts`, wider than tall).
- **Autotile is projection-independent by construction** (`src/tilemap/autotile.rs`): masks come from
  `tiles[row][col]` via a `filled` closure; the *neighborhood* picks offsets. Ortho + iso share
  `Edge4`/`Blob8`; hex needs `Hex6` (pointy odd-r) / `Hex6Flat` (flat odd-q) only because the 6
  neighbor offsets differ and are **parity-dependent** (row parity for odd-r, col parity for odd-q).
  `TilemapSystem` passes `autotile.neighborhood` straight through, so new neighborhoods "just work".

## Files Changed (by area; full per-file lists in each per-feature handoff)
### Source
- `src/audio_wasm.rs` — buses, crossfade, ducking, positional, play_at_on_bus, `Bus`/`MusicChannel`,
  `ramp_gain_to`, `spatial_params`, `Sfx` getters/`update_position` (seq 34–36).
- `src/ui/system/state.rs` — gamepad fold-in; `src/input/gamepad.rs` — `test_press` test helper (37).
- `src/tilemap/mod.rs` — `HexagonalFlat` + 4 projection methods; `src/tilemap/autotile.rs` —
  `Hex6`/`Hex6Flat` + `hex6_mask`/`hex6_flat_mask` + `hex_6`/`hex_6_flat` (37).
### Examples (new)
- `hex_tilemap_flat`, `hex_autotile` (+ generated `examples/assets/hex_tiles_flat.png`).
- `web_audio` + `examples/ui_focus.rs` extended (existing examples grown, not new).
### Tooling
- `scripts/wasm_audio_smoke.sh` — extended each audio release (verdict 12 → 38).
### Docs
- `docs/CHANGELOG.md` (8 entries), `CLAUDE.md` (header v1.6.80→v1.6.88, audio + UI + tilemap rows),
  4 per-feature handoffs (seq 34–37) + this wrap (seq 38).

## User Feedback & Preferences (calibrates next session)
- **Drives work item-by-item off a backlog list** — terse, decisive picks: "진행하고 머지까지",
  "1번 진행", "2진행해", "1.2.3 모두 실행". Offer a concrete prioritized list and let them choose.
- **Standing "test → merge" approval** once an item is picked — carry each through CI-green
  squash-merge + tag, but still report each merge. (Auto-merge is disabled on the repo; merges done
  via a background "wait green then `gh pr merge --squash`" task per PR.)
- **Wants gaps named honestly** (established last session, held this one) — e.g. auto-sidechain not
  ported, the blank windowed-playtest visual. Don't over-claim "done".
- Korean for all user-facing reports; English for code/docs/handoffs (project rule).
- Values the VISION loop: every feature shipped with a playable example.

## Where We're Going (next session — all optional, none committed)
1. **crates.io publish** — the one untouched backlog item. Irreversible; needs explicit go. Publish
   `engine_reflect_derive` too so `cargo add` users get `#[derive(Reflect)]`. (Repo's `Package
   dry-run` CI job already passes, so it's mechanically unblocked.)
2. **Smaller follow-ups surfaced this session:** gamepad **analog-stick** focus nav (needs per-frame
   threshold debounce — the stateless `InputSnapshot` can't do it as-is); a `hex_autotile` example
   for **flat-top** (`Hex6Flat`); a real **64-tile hex autotile atlas** (to exercise `hex_6`); the
   focus-ring styling knobs from the original stretch list (never picked).
3. **Eyeball the un-verified visuals** if a session has interactive display access:
   `hex_tilemap_flat` + `hex_autotile` (their math is unit-tested but the render wasn't eyeballed).

## Risks & Blockers
- None blocking. Tree clean, all CI green, all tags pushed.
- **Visual features (v0.38.0/v0.39.0) not eyeballed** — the shell-launched GUI window came up blank
  in screencapture. Mitigated by full unit-test coverage of the coordinate/mask math + the shared,
  already-proven `TilemapSystem` render path. Low risk, but flagged.
- Auto-merge disabled on the repo (`enablePullRequestAutoMerge` GraphQL error) — merges are manual
  via the background wait-green-merge task.

## Cross-cutting gotchas (expensive-to-rediscover lessons)
1. **Headless SwiftShader does NOT advance `AudioParam` automation.** A scheduled `linearRamp`'s live
   value is computed on the audio render thread, which doesn't run headless — so `gain.value()` during
   a ramp returns the anchor, not the ramped value (the first ducking smoke FAILED on exactly this).
   Direct `set_value` (the `value=` setter) IS reflected immediately. Fix: ducking is **instant when
   `dur <= 0`** (`set_value`), and the smoke uses `dur=0.0` for deterministic reads. Smooth ramps
   (and acoustic output) stay by-ear/real-browser checks. This is why positional (set_value path) is
   fully headless-verifiable but a duck *ramp* is not.
2. **`./scripts/verify.sh | tail` (or any trailing pipe) masks the real exit code** — reports `tail`'s
   0, hiding a `cargo fmt --check` failure. Bit twice this session. Run `verify.sh > log 2>&1; echo
   $?` (or capture `VERIFY_EXIT=$?` to a file) for the authoritative verdict.
3. **`cargo fmt` after EVERY edit, before the gate** — `cargo test` doesn't reformat; an unformatted
   `if let` chain failed `fmt --check` (caught by gotcha #2's discipline).
4. **Shell-launched GUI window comes up blank in screencapture** (macOS/winit) — the windowed visual
   playtest path is unreliable; cover visual features with unit-tested math + the shared render path,
   eyeball only when a real display session is available.
5. **`GamepadState` can't be driven in tests without `gilrs`** — its `just_pressed` set is populated
   only by native `process_event`. Added a `#[cfg(test)] pub(crate) GamepadState::test_press` helper.
6. **`TilemapProjection` is a plain (non-exhaustive-less) enum** — adding a variant breaks external
   exhaustive matches (fine under 0.x MINOR) and requires updating all 4 internal match sites.
7. **Squash-merge diverges the base** — branch each next feature off fresh main only AFTER the prior
   merged (carried over from last session; held all 8 releases).

## Quick Start for Next Session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -10           # a04c670 (#132 handoff) … 0981011 (#131 v0.39.0)
grep -m1 '^version' Cargo.toml  # 0.39.0
./scripts/verify.sh             # green (fmt/clippy/wasm build/test/doc)

# Optional local browser check (needs Chrome + matching wasm-bindgen-cli):
bash scripts/wasm_audio_smoke.sh   # PASS (38/38)

# Key files by area:
#   audio:   src/audio_wasm.rs (WebAudio: buses/crossfade/ducking/positional; Sfx; ramp_gain_to)
#   ui:      src/ui/system/state.rs (gamepad focus fold-in)
#   tilemap: src/tilemap/mod.rs (HexagonalFlat), src/tilemap/autotile.rs (Hex6/Hex6Flat)

# Next action (only if the user picks one): crates.io publish (explicit go required) — the requested
# backlog (wasm audio parity + stretch trio) is fully shipped; nothing else is required.
```

---

## Process / versioning notes
- 0.x cadence: **MINOR = any release (incl. breaking); PATCH = bugfix.** This session was all additive
  MINORs (v0.38.0/v0.39.0 add enum variants — technically breaking for external exhaustive matches,
  still MINOR under 0.x). Each release bumped Cargo.toml + Cargo.lock + CHANGELOG + CLAUDE.md header
  (`v1.6.80`→`v1.6.88`) together (the `ship`-skill four-edit set), then `cargo update -p
  skeleton-engine` for the lock.
- Tags are annotated `vX.Y.Z — desc (#NN)` on the squash-merge commit, pushed right after merge.
- Branch-protected (`enforce_admins=true`); required checks Build (WASM)/Test (native)/Rustdoc/
  Package dry-run. Test (native) ≈ 4.5–6 min is the long pole per PR. Handoffs go through PRs too.

---

## Session Closed
**Closed at:** 2026-06-18 23:29 KST
**Commit:** merged via PR (`docs(handoff)` seq-38 session wrap)
**Session status:** Handed off to next session
