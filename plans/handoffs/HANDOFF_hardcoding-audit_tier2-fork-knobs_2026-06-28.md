# Hardcoding-audit Tier-2 continued: three additive fork-config knobs (point-light cap, slider step, tile-anim stagger), each a MINOR PR + VISION example

**Date:** 2026-06-28
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `hardcoding-audit` seq `3`
**Parent:** `HANDOFF_hardcoding-audit_2026-06-27.md` (seq 2 — Tier-2 physics/timing bundle #255–#257)
**Prior chain:** `HANDOFF_hardcoding-audit_2026-06-26.md` (seq 1, Tier-1) > `HANDOFF_hardcoding-audit_2026-06-27.md` (seq 2, Tier-2 physics/timing) > this (seq 3, Tier-2 fork-config knobs)

## Related Handoffs

- `HANDOFF_headless-screenshot_2026-06-28.md` — the `headless-screenshot` chain seq 1 (the session immediately before this one, #259–#261). **Not a chain parent**, but its feature (`App::screenshot_headless` / `save_screenshot_headless`) was used this session to pixel-verify the GPU lighting change with the monitor off — the exact synergy that feature was built for.

---

## Since Last Handoff

Parent seq 2 closed with three explicit next-step pointers; this session executed the third:

1. **"Read the wishlist board FIRST, ASK if empty"** → done. `../dungeon-merchant/docs/engine-wishlist.md` was **ACTIVE EMPTY** (EW-004 next free). I asked the user for direction (AskUserQuestion) → they chose **"Tier-2 하드코딩 노브"**.
2. **"If more hardcoding work: `docs/HARDCODING_AUDIT_2026-06-26.md` Open — Tier 2 (remaining)"** → this is exactly what the session became. Parent named `SLIDER_STEP_FRAC` (per-Slider), `MAX_LIGHTS` (hardest, dynamic WGSL uniform), `desired_maximum_frame_latency`, `STICK_ACTIVATE/RELEASE`, editor `APP_ID` as candidates.
3. The session landed **three** of them as separate MINOR PRs, then the user chose to wrap (handoff). Picks were driven by *verifiability*: I deliberately led with `MAX_LIGHTS` (the highest-priority knob, **now** GPU-verifiable thanks to seq-105 headless), then chose the two **fully-CI-verifiable** knobs (slider step, tile stagger) over the hard-to-verify ones (MAX_GAMEPADS needs >4 physical pads; latency=1 is unobservable in a test/screenshot).

No risks from parent materialized. The one new wrinkle: the locked/remote macOS session **lost its audio device mid-session** (verify went green for #262, then 101 for #263/#264 on the same 2 audio tests) — the documented environmental failure, handled with `--skip` + CI.

## Reference Documents

- `docs/HARDCODING_AUDIT_2026-06-26.md` — the audit's found→fixed record. Its **"Resolution log — Tier 2 (shipped)"** table now lists #262/#263/#264; the **"Open — Tier 2 (remaining)"** table shrank by three rows. (Updated as part of this handoff PR, per the seq-2 precedent.)
- `CLAUDE.md` — module map rows updated for all three features (lighting / Slider / tilemap-animation).
- `plans/handoffs/HANDOFF_headless-screenshot_2026-06-28.md` — the headless feature used to verify #262.

## The Goal

Continue closing the Tier-2 "a fork must edit engine source" hardcoded values from the 2026-06-26 audit, each as an **additive, default-preserving** public knob (non-breaking) with a small **playable example** that exercises it (the skeleton-engine VISION acceptance test). The end state: a downstream game can raise the point-light cap, set a discrete keyboard slider step, and sync/desync animated tiles — none of which were configurable before — without forking the engine.

## Where We Are

- **main @ `5520c7f`** (v0.78.0), CLAUDE.md header **v1.6.162**, tree **clean**, **no open PRs**.
- **Three feature PRs merged this session** (squash, branches deleted, CI 4/4 each):
  - **#262** (`8531016`, v0.76.0 MINOR) — `LightingConfig { max_lights }`: configurable point-light cap.
  - **#263** (`40f3c82`, v0.77.0 MINOR) — `Slider::keyboard_step`: per-slider keyboard nudge step.
  - **#264** (`5520c7f`, v0.78.0 MINOR) — `TileAnimationSet::stagger`: animated-tile phase stagger.
- Memory `engine-current-state.md` bumped to **seq 107 → 108 → 109** (one per PR; recent-seqs list + description + body header all point at `5520c7f` / v0.78.0).
- The lib test count grew 945 → **951** across the three PRs (+6 unit tests: 1 lighting cull, 2 slider, 2 tilemap stagger, +1 from the existing suite recount).
- Three new examples: `lighting_cap`, `slider_keyboard_step`, `tile_anim_stagger`. One new smoke: `scripts/lighting_cap_smoke.sh`.

### #262 — `LightingConfig` (point-light cap), v0.76.0
- The 2D point-light pass (`src/renderer/lighting.rs`) was hard-capped at **16** lights, baked into the WGSL `array<GpuLight, 16>` + the `LightingUniforms` Rust struct + a nearest-16 cull. The highest-priority remaining Tier-2 knob.
- New opt-in `LightingConfig { max_lights }` resource (`src/resources/render.rs`), default `DEFAULT_MAX_LIGHTS` = 16, both re-exported from the crate root. **Absent → byte-identical 544-byte uniform** (non-breaking).
- `LightingRenderer::new` now takes `max_lights`: the WGSL light-array length is substituted at shader-build time and the uniform is a fixed 32-byte `LightingHeader` followed by a runtime-sized `[GpuLightData; max_lights]` region (`32 + N*32` bytes) written as **one byte block** — replacing the fixed `LightingUniforms` struct. Field order / std140 layout unchanged → default-16 is byte-identical.
- A runtime cap change rebuilds the renderer (`set_max_lights`), wired through `setup_lighting` (`src/app/render/post_lighting.rs`, reads `LightingConfig`, also handles a same-frame resize). `select_nearest_lights` takes the cap as a parameter (cap 0 → empty, shader array clamped to ≥1, no panic).
- Example `lighting_cap` (40-light hue grid; SPACE toggles cap 16↔40; `LIGHTING_CAP` env override; `HEADLESS_SHOT` self-reports a lit-pixel count) + `scripts/lighting_cap_smoke.sh` (headless GPU A/B).

### #263 — `Slider::keyboard_step`, v0.77.0
- A focused `Slider` moved by a hardcoded `const SLIDER_STEP_FRAC = 0.05` (5% of range) per ←/→ press (`src/ui/system/focus_pass.rs`) — wrong for a slider selecting a few discrete levels (5% of 0..3 = 0.15, lands between levels).
- New `Slider::keyboard_step: Option<f32>` field + `with_keyboard_step(step)` builder + `resolved_keyboard_step()` (override, else `DEFAULT_SLIDER_STEP_FRAC × (max−min)`). `DEFAULT_SLIDER_STEP_FRAC` (= 0.05) promoted to a re-exported `pub const`. `focus_pass` nudge calls the resolver.
- **Non-breaking** because `Slider` has a private field (`dragging`) → only ever constructed via `Slider::new` + builders (no struct literal). The new field carries `#[serde(default)]` so existing scene RON loads unchanged.
- Example `slider_keyboard_step` (continuous Volume 0..100 default-5%-step beside discrete Quality 0..3 `with_keyboard_step(1.0)` = one Low/Med/High/Ultra level per press).

### #264 — `TileAnimationSet::stagger`, v0.78.0
- Animated tiles of the same value start out of phase by `(row + col) × frame_time × 0.37` so neighbours don't sync-flash — but `0.37` was a hardcoded literal in `src/tilemap/system.rs::make_anim_cell`.
- New `TileAnimationSet::stagger` field (default `DEFAULT_TILE_ANIM_STAGGER` = 0.37, re-exported `pub const`) + `with_stagger(factor)` builder. `0.0` = every cell in lockstep, larger = more rippling.
- **Non-breaking** because `TileAnimationSet`'s `anims` map is a private field → only built via `new()` + `insert`. **Required a manual `Default` impl** (the old `#[derive(Default)]` would give `stagger = 0.0`, changing behavior).
- The per-cell phase formula moved from the inline literal into a unit-tested `pub(crate) tilemap::animation::stagger_phase(row, col, frame_time, total_time, stagger)` helper.
- Example `tile_anim_stagger` (two identical animated grids cycling an 8-frame hue ramp — left `with_stagger(0.0)` flat/lockstep, right default 0.37 diagonal frame gradient; difference visible in one frame).

## What We Tried (Chronological)

1. **Read the parent handoff + wishlist board first.** Board ACTIVE EMPTY → AskUserQuestion offered four directions (CI golden-image tests / more HEADLESS_SHOT examples / Tier-2 knobs / new feature). User chose **Tier-2 knobs**.
2. **Picked `MAX_LIGHTS` as the headline.** It's the only remaining *High*-priority Tier-2 item, and — critically — the seq-105 headless screenshot feature (shipped the prior session) means a GPU render change is now pixel-verifiable monitor-off. Read `lighting.rs` end-to-end first; confirmed `uniform_buffer_entry` uses `min_binding_size: None` (so a runtime-sized uniform binds fine) and that flat examples are auto-discovered (no Cargo.toml entry needed).
3. **#262 design decision: dynamic uniform, not storage buffer.** Lighting is native-only, so a storage buffer (uncapped) was viable — but I kept it a **uniform** (lower GPU risk, same binding type) just dynamically sized via a header-struct + raw byte assembly. Default 16 → byte-identical. Verified with `headless_screenshot` (cap 40 → 487,954 lit px vs cap 16 → 210,402 lit px) and a visual A/B sent to the user's mobile.
4. **User: "a 진행"** (continue with another Tier-2 knob). **Picked `SLIDER_STEP_FRAC`** as the cleanest *fully-CI-verifiable* one (keyboard-only example, no GPU/audio/hardware gate). Implemented as an absolute-value override (not a fraction) — more intuitive for discrete-level sliders.
5. **Mid-session: verify went 101.** The 2 audio-device tests (`play_tone_reports_playing_then_finished_when_audio_device_exists`, `stop_on_drained_sink_is_immediate`) started failing — the locked/remote session had **lost its audio device** (they'd passed for #262 earlier). Confirmed environmental (re-ran on the tree, fail regardless; UI change touches no audio) → verified with `--skip` + the doc gate, let CI gate audio.
6. **User: "a 진행"** again. **Picked `TileAnimationSet::stagger`** — CI-verifiable AND single-frame-screenshottable (synced grid = flat colour, staggered grid = diagonal frame gradient). Extracted the phase formula into a pure helper for the unit test.
7. **User: "b"** → wrap with this handoff.

## Key Decisions

- **`LightingConfig` is an opt-in resource, default 16 — not a `WindowConfig`/struct-field change.** Mirrors the `FrameConfig`/`WindowOptions`/`DesignResolution` pattern: absent = old behavior, so non-breaking. (`WindowConfig` field-adds are breaking, ~70 call sites — standing directive.)
- **Dynamic *uniform*, not a storage buffer, for the light array.** Same binding type → minimal GPU risk; the default cap reproduces the exact 544-byte block. The header/array split keeps the WGSL std140 field order identical.
- **All three knobs are non-breaking *despite* adding `pub` fields**, because each owning struct already has a private field (`Slider::dragging`, `TileAnimationSet::anims`) → external code can't use a struct literal, only `new()` + builders. This is the reusable trick for "add a configurable field to an existing public component."
- **`TileAnimationSet` needed a manual `Default`** — `#[derive(Default)]` would default `stagger` to `0.0` (synchronized), silently changing behavior. Manual impl sets `DEFAULT_TILE_ANIM_STAGGER`.
- **Match verification to what's checkable.** #262 is GPU-runtime → the gate is the local headless smoke (CI ubuntu can't render). #263/#264 are pure logic → fully CI-verified by unit tests. The 2 audio tests are unverifiable locally in this session → `--skip` + CI.
- **Led with the hardest/highest-value knob while it was freshly verifiable** (MAX_LIGHTS, enabled by seq-105 headless), then took the two clean CI-verifiable wins. Deferred MAX_GAMEPADS (needs >4 pads) and latency=1 (unobservable).

## Evidence & Data

### Commits landed (main)
| Hash | PR | Bump | Summary |
|---|---|---|---|
| `8531016` | #262 | 0.75.1→0.76.0 MINOR | `LightingConfig` configurable point-light cap |
| `40f3c82` | #263 | 0.76.0→0.77.0 MINOR | `Slider::keyboard_step` per-slider nudge step |
| `5520c7f` | #264 | 0.77.0→0.78.0 MINOR | `TileAnimationSet::stagger` animated-tile phase |

### CI (all 4/4 SUCCESS — native test gates the audio)
| PR | Test (native) | WASM | Rustdoc | Package |
|---|---|---|---|---|
| #262 | 5m16s | pass | pass | pass |
| #263 | 5m56s | pass | pass | pass |
| #264 | 6m16s | pass | pass | pass |

### #262 headless GPU A/B (captured monitor-off)
- `lighting_cap` cap=40 → **487,954 lit pixels** (all 40 colored pools render).
- `lighting_cap` cap=16 → **210,402 lit pixels** (only the 16 central pools nearest the camera; outer 24 dark — proves the nearest-to-camera cull).
- `scripts/lighting_cap_smoke.sh` asserts cap-40 lights >1.5× the area of cap-16 → PASS. The old hardcoded 16 could never render 40 — direct proof the cap drives the GPU pass.

### #264 single-frame visual (HEADLESS_FRAMES=40)
- Left grid `with_stagger(0.0)`: all 36 cells show the same frame → one flat cyan square (lockstep).
- Right grid default `0.37`: cells along each diagonal show successive frames → a cyan→blue→purple→magenta gradient (the rippling look).

### Unit tests added
| Test | File | Asserts |
|---|---|---|
| `select_nearest_lights_honors_a_custom_cap` | `renderer/lighting.rs` | cap 24 over 30 lights → 24 nearest; cap 0 → empty (no panic) |
| `keyboard_step_defaults_to_frac_of_range_and_honors_override` | `ui/slider.rs` | default 5% of range; absolute override; serde round-trip |
| `focused_slider_honors_a_custom_keyboard_step` | `ui/system/focus_pass.rs` | one ArrowRight steps exactly 1 level on a 0..3 / step-1 slider |
| `stagger_defaults_to_historical_value_and_builder_overrides` | `tilemap/animation.rs` | default 0.37; `with_stagger(0.0)` |
| `stagger_phase_synchronizes_at_zero_and_spreads_otherwise` | `tilemap/animation.rs` | stagger 0 → all phase 0; 0.37 → neighbours differ + phase wraps into [0,total) |

### The environmental audio-test failure (recurred this session)
- `cargo test --all-targets` fails exactly 2 tests in the locked/remote macOS state: `audio::tests::play_tone_reports_playing_then_finished_when_audio_device_exists` (assertion `Playing != Finished`) and `audio::tests::stop_on_drained_sink_is_immediate` ("tone should have drained").
- **Confirmed environmental:** they fail on the tree regardless of my change (which is UI/tilemap only, no audio), and pass on CI ubuntu (all three PRs' native tests green).
- Workaround: `cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate` (all others pass) + run `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` separately; let CI gate audio.

## Code Analysis

- **`LightingHeader`** (`renderer/lighting.rs`) is exactly 32 bytes: `ambient_color: [f32;3]` (12) + `ambient_intensity: f32` (4) + `light_count: u32` (4) + `aspect_ratio: f32` (4) + `_pad: [f32;2]` (8). The uniform is `header ++ [GpuLightData; max_lights]` (`GpuLightData` = 32 B). `uniform_buffer_entry` has `min_binding_size: None`, so any size binds.
- **`select_nearest_lights(positions, camera_pos, max_lights)`** uses `select_nth_unstable_by` on squared distance to the *viewport center* (not camera top-left). `max_lights == 0` clears the index list; the shader array clamps to `max_lights.max(1)` so it's never zero-length.
- **`Slider::resolved_keyboard_step()`** = `keyboard_step.unwrap_or((max - min) * DEFAULT_SLIDER_STEP_FRAC)`. `focus_pass.rs` calls it in the `nav_left || nav_right` branch.
- **`stagger_phase(row, col, frame_time, total_time, stagger)`** = `((row + col) as f32 * frame_time * stagger) % total_time.max(f32::EPSILON)`. `stagger = 0` → always 0.
- **The non-breaking-field pattern**: a public struct with a private field cannot be built with a struct literal externally, so adding a `pub` field doesn't break callers (they use `new()` + builders). Verified for both `Slider` (`dragging`) and `TileAnimationSet` (`anims`).

## Files Changed

### #262 (LightingConfig)
- `src/resources/render.rs` — `LightingConfig` resource + `DEFAULT_MAX_LIGHTS` const.
- `src/renderer/lighting.rs` — `LightingHeader` (replaces fixed `LightingUniforms` struct); `LightingRenderer::new` takes `max_lights`, stores it, sizes shader+buffer; `set_max_lights`; `max_lights()` getter; `select_nearest_lights` cap param; `update` assembles header+lights bytes; +1 test; struct-size test updated.
- `src/app/render/post_lighting.rs` — `setup_lighting` reads `LightingConfig`, passes the cap, rebuilds on change.
- `src/lib.rs` + `src/resources/mod.rs` — re-export `LightingConfig` + `DEFAULT_MAX_LIGHTS`.
- `examples/lighting_cap.rs` (new), `scripts/lighting_cap_smoke.sh` (new), `CLAUDE.md` lighting row.

### #263 (Slider::keyboard_step)
- `src/ui/slider.rs` — `keyboard_step` field, `with_keyboard_step`, `resolved_keyboard_step`, `DEFAULT_SLIDER_STEP_FRAC` const, manual init in `new`; +1 test.
- `src/ui/system/focus_pass.rs` — nudge uses `resolved_keyboard_step()`; removed local `SLIDER_STEP_FRAC`; +1 test.
- `src/lib.rs` + `src/ui/mod.rs` — re-export `DEFAULT_SLIDER_STEP_FRAC`.
- `examples/slider_keyboard_step.rs` (new), `CLAUDE.md` Slider row.

### #264 (TileAnimationSet::stagger)
- `src/tilemap/animation.rs` — `stagger` field, manual `Default`, `with_stagger`, `DEFAULT_TILE_ANIM_STAGGER` const, `stagger_phase` helper; +2 tests.
- `src/tilemap/system.rs` — `make_anim_cell` calls `stagger_phase` with `anim_set.stagger`.
- `src/lib.rs` + `src/tilemap/mod.rs` — re-export `DEFAULT_TILE_ANIM_STAGGER`.
- `examples/tile_anim_stagger.rs` (new), `CLAUDE.md` tilemap row.

### Paperwork (all three PRs)
- `Cargo.toml` / `Cargo.lock` 0.75.1→0.76.0→0.77.0→0.78.0; `docs/CHANGELOG.md` (3 entries); `CLAUDE.md` header v1.6.159→v1.6.162.
- Memory `engine-current-state.md` — seq 107/108/109.
- `docs/HARDCODING_AUDIT_2026-06-26.md` — Tier-2 resolution log (this handoff PR).

## User Feedback & Preferences

- **Picks the work direction from a short menu, then delegates execution.** Chose "Tier-2 하드코딩 노브" from the AskUserQuestion, then said **"a 진행"** twice to continue with another knob each time (delegating *which* knob to me), then **"b"** to wrap. I picked the knobs by verifiability and led with the highest-value one.
- **Values verification, not assertion.** Consistent with the prior session ("확인해줘" = actually run it). I used the headless screenshot to pixel-verify #262 and sent the PNGs to mobile; sent the slider + stagger demo PNGs too.
- **Merge authority is standing-delegated** (squash on green CI, no per-session re-confirm) — all three merged without asking.
- **Standing prefs honored:** user-facing reports in **Korean**, code/docs/PRs in **English**; `cargo fmt` before verify; never trust a masked gate exit (read the log); user-facing visual proof delivered via `SendUserFile`.

## Where We're Going

1. **This handoff + the audit-doc Tier-2 resolution-log edit land as one `docs(handoff)` PR** (`docs/handoff-tier2-fork-knobs`, no package bump). The memory `main @` pointer then updates to that merge.
2. **Next session: read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004 next). New EW → VISION loop; still empty → ASK.
3. **If more Tier-2 hardcoding work** (`docs/HARDCODING_AUDIT_2026-06-26.md` "Open — Tier 2", now 3 rows lighter). Remaining + verifiability notes:
   - `desired_maximum_frame_latency = 2` → `WindowOptions`/`WindowConfig` field. **Easy plumbing but the effect (input latency) is unobservable in a test/screenshot** — weak VISION demo.
   - `STICK_ACTIVATE 0.6` / `STICK_RELEASE 0.35` deadzone → `UiConfig` resource. CI-testable logic, but the *example* needs a gamepad to exercise.
   - `network/native.rs READ_TIMEOUT 5ms` → `NetworkConfig` field. Native-only; CI-testable plumbing, no clean playable demo.
   - `app/editor/settings.rs APP_ID "skeleton-engine"` → crate-level const a fork overrides. Editor-only, trivial, no real example.
   - `app/editor/ui/gizmo_math.rs ROT_HANDLE_GAP/ROT_HIT_RADIUS` → scale by zoom or `EditorSettings`. Editor-only.
   - `renderer/sprite/batch.rs` material params fixed at 16 B → larger/typed payload. Bigger change.
   - **`MAX_GAMEPADS`** (`input/gamepad.rs` `[Option;4]`) — easy code, but **verifying >4-pad local co-op needs >4 physical pads** (deferred all session).
   - **Tier 3** is naming/dedup polish (low value).

## Risks & Blockers

- **None blocking.** Tree clean, all three merges green, no open PRs.
- The #262 headless GPU **runtime** path is NOT CI-verified (ubuntu CI has no GPU) — `scripts/lighting_cap_smoke.sh` + `headless_screenshot_smoke.sh` are the local gates. A future edit to the lighting render/read-back path needs a local GPU smoke.
- In a **locked/remote macOS session, the 2 audio-device tests cannot pass locally** (no audio device). Don't mistake for a regression; verify with the `--skip` pattern + CI. Audio output itself remains ear-only / CI-unverifiable (the standing judgment gate).

## Open Questions

- **None blocking.** The remaining Tier-2 knobs are catalogued with verifiability caveats above; the next direction is the wishlist board (ASK if empty).

## Quick Start for Next Session

```bash
# 1. Downstream board FIRST (standing directive)
cat ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY? EW-004 next → ASK if empty

# 2. Confirm state
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5        # tip = this handoff merge, above 5520c7f (#264)
git status -s               # clean

# 3. Try the three new examples (all support HEADLESS_SHOT, monitor-off-capable):
HEADLESS_SHOT=/tmp/lights.png cargo run --example lighting_cap        # + LIGHTING_CAP=16 to A/B the cull
cargo run --example slider_keyboard_step                              # Tab + ←/→ (or HEADLESS_SHOT)
HEADLESS_SHOT=/tmp/stagger.png HEADLESS_FRAMES=40 cargo run --example tile_anim_stagger
scripts/lighting_cap_smoke.sh                                         # native GPU A/B smoke

# 4. More Tier-2? docs/HARDCODING_AUDIT_2026-06-26.md "Open — Tier 2" (3 rows lighter).

# 5. Verify (in a locked/remote session, the 2 audio-device tests fail environmentally):
cargo fmt && ./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"
#   If ONLY the 2 audio tests fail: re-run with
#   cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
#   then RUSTDOCFLAGS="-D warnings" cargo doc --no-deps; let CI gate audio.
#   (BACKGROUNDED verify must NOT append `; echo` — read the LOG; the task exit lies.)

# Next action: read the wishlist board; if empty, ASK the user for direction (don't auto-start backlog).
```

---

## Session Closed

**Closed at:** 2026-06-28 (KST)
**Commit:** lands via a `docs(handoff)` PR (this file + the `docs/HARDCODING_AUDIT_2026-06-26.md` Tier-2 resolution-log edit).
**Session status:** Handed off — three Tier-2 fork-config knobs (#262 `LightingConfig` v0.76.0, #263 `Slider::keyboard_step` v0.77.0, #264 `TileAnimationSet::stagger` v0.78.0) merged to `main`; each additive/non-breaking with a VISION example; #262 GPU-verified headless monitor-off; memory bumped to seq 109. This handoff is the session record.
