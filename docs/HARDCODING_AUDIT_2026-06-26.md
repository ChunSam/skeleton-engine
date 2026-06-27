# Hardcoding audit — 2026-06-26

A whole-`src/` sweep for values baked into engine code that a fork or downstream game would
reasonably want to configure, or that should be a named constant. Driven by the user request
"check the whole code for remaining hardcoding".

- **Baseline:** `main` @ `947cd25`, package v0.68.4.
- **Method:** 9 parallel read-only `Explore` agents, one per subsystem area (renderer; app/window;
  editor UI; ECS/resources/camera; physics/pathfinding/AI; audio; animation/particle/tilemap;
  UI/dialogue/timeline; input/network/save/scripting). ~118 findings, triaged into themes, then
  each High finding verified against the source by hand before acting.
- **Scope of "hardcode":** a value with no public knob that a fork would want to change, a magic
  number duplicated across sites (drift risk), or a baked default with no named anchor. Excluded:
  test code, legitimate math/protocol constants, and values that already have a config knob.

This file is the found → fixed record (mirrors `docs/CODE_QUALITY_FINDINGS_2026-06-23.md`).

---

## Resolution log — Tier 1 (shipped this session)

The user chose to land all of Tier 1 (verified, genuinely worth doing), as four separate PRs.

| Item | Area | Fix | PR / version |
|---|---|---|---|
| Clear color `[0.08,0.08,0.12,1.0]` in 4 sites (2 forms) could drift | render/ui | `DEFAULT_CLEAR_COLOR` (`resources/display.rs`, re-exported) for the game default + clear-pass fallback; `EDITOR_SURFACE_CLEAR` (`app/render/mod.rs`) for the two docked-editor clears, kept separate so a game's `clear_color` doesn't repaint the editor letterbox | #248 / v0.68.5 |
| Panel bg z-offset `0.01` was a named const in `capture.rs` but a raw literal in `panel.rs` (must match) | ui | Hoisted `PANEL_BG_Z_OFFSET` to `ui/panel.rs` (where `LayoutSystem` draws it); `capture.rs` imports it | #248 / v0.68.5 |
| `PathGrid` silently became an empty grid (all pathfinding fails) on `i32` overflow or exceeding the 10M-cell cap, with no diagnostic | pathfinding | Both cases now `log::error!`; benign zero-dim empty grid stays silent; `MAX_PATH_GRID_CELLS` made a documented `pub const` (re-exported) | #249 / v0.68.6 |
| `Tilemap::cell_z()` returned a fixed `-1.0` for ortho/hex → two tilemaps couldn't be layered | tilemap | `Tilemap::z` field + `with_z` builder (default `-1.0`); `cell_z()` returns it for ortho/hex; isometric unchanged (per-cell `row+col`). Example `tilemap_layers` | #250 / v0.69.0 |
| Native `play_tone` emitted a raw `SineWave` → click on/off, while wasm `WebAudio` always enveloped tones | audio | Default (no-effect) native tone now carries a `min(25% of the tone, 8 ms)` linear attack+release envelope matching wasm (materialized into a `SamplesBuffer`, since rodio 0.19 has no source `fade_out`). Ear-verified before merge | #251 / v0.69.1 |

---

## Resolution log — Tier 2 (shipped)

Landed as additive, default-preserving knobs (non-breaking), each with a VISION example.

| Item | Fix | PR / version |
|---|---|---|
| `renderer/render_target.rs` sampler always `Nearest` | `App::create_render_target_with_filter` / `RenderTarget::with_filter(FilterMode)` | #253 / v0.70.0 |
| `particle/mod.rs` per-frame `count.min(64)` spawn cap | `ParticleEmitter::max_per_frame` field + `with_max_per_frame` + `DEFAULT_MAX_PER_FRAME`; RON + editor row; example `particle_spawn_cap` | #254 / v0.71.0 |
| `physics/world/character_movement.rs` `ONE_WAY_TOLERANCE = 0.05` | `CharacterController::one_way_tolerance` field + `with_one_way_tolerance` + `DEFAULT_ONE_WAY_TOLERANCE`; example `one_way_tolerance`; deterministic threshold test | #255 / v0.72.0 |
| `physics/world.rs` `IntegrationParameters::default()` | `PhysicsWorld::set_solver_iterations(n)` + `with_integration_params` builder + `integration_params()` getter; example `solver_iterations` (heavy-ended joint chains); behavioral test. NOTE: rapier 0.22's default-4 TGS solver is already robust for plain stacks — the regime where iteration count *visibly* matters is loaded joints/ragdolls (the example) and extreme mass ratios, not ordinary box stacks. | #256 / v0.73.0 |
| `app/render/frame.rs` max-dt cap `.min(0.1)` | `FrameConfig { max_dt }` resource (default 0.1, auto-inserted) + `FrameConfig::cap`; example `frame_dt_cap`; default+clamp tests | #257 / v0.74.0 |

## Open — Tier 2 (remaining fork-configurability knobs; additive, not yet done)

These are genuine "a fork must edit engine source" limits. Each is an additive field / builder /
const (default preserved → non-breaking). Candidates for a future feature pass, ideally each with
a small example per the VISION loop.

| Pri | Site | Limit | Suggested API |
|---|---|---|---|
| High | `renderer/lighting.rs` `MAX_LIGHTS = 16` | baked into the WGSL `array<GpuLight,16>` + `LightingUniforms` size → hard cap of 16 point lights | configurable cap (non-trivial: dynamic uniform array) |
| High | `input/gamepad.rs` `[Option<Slot>;4]` + `pad < 4` ×3 | no >4-pad local co-op | `pub const MAX_GAMEPADS` + widen the array (hard to verify: needs >4 physical pads) |
| Med | `renderer/context.rs` `desired_maximum_frame_latency: 2` | rhythm/fighting forks can't request latency=1 | `WindowConfig`/`WindowOptions` field |
| Med | `tilemap/mod.rs` autotile phase `0.37` stagger | animated-tile stagger not configurable | const or `TileAnimationSet` field |
| Med | `ui/system/state.rs` `STICK_ACTIVATE 0.6`/`STICK_RELEASE 0.35` | gamepad deadzone not tunable | `UiConfig` resource fields |
| Med | `ui/system/focus_pass.rs` `SLIDER_STEP_FRAC 0.05` | keyboard slider step fixed | per-`Slider` override |
| Med | `network/native.rs` `READ_TIMEOUT 5ms` | poll granularity not in `NetworkConfig` | `NetworkConfig` field |
| Med | `renderer/sprite/batch.rs` material params fixed at 16 B (`[f32;4]`) | no richer per-material shader data | larger/typed params payload |
| Med | `app/editor/settings.rs` app-id `"skeleton-engine"` in the settings dir | every fork shares one editor-settings dir on disk | crate-level `APP_ID` const a fork overrides |
| Med | `app/editor/ui/gizmo_math.rs` `ROT_HANDLE_GAP 16`/`ROT_HIT_RADIUS 8` (world units) | rotation gizmo unusable at non-default world scale | scale the handle by zoom or expose in `EditorSettings` |

## Open — Tier 3 (naming / dedup; low value)

- Magic-number duplication: `1280×720` in `ViewportSize::default` vs `WindowConfig::default`; the
  `0.001` fade/duck/release floors scattered ~10 sites in `src/audio/`; UI sublayer z-step `0.001`
  ×3; `64*1024` in `network/event.rs` vs `scripting.rs`.
- Default named-const gaps (a knob already exists): camera shake `30Hz`/`1.7`/`2.3`; `chars_per_sec
  28.0` duplicated in two `DialogueBox` ctors; Rhai limits in `scripting.rs`; text line-height
  `×1.2`; sprite/UI/material initial buffer caps `128`/`64`/`16`.
- Editor i18n: a few English UI strings bypass `tr()` (state-machine condition combo box, asset-list
  placeholder).
- Cross-platform drift remaining: the wasm tone-envelope formula is duplicated at two sites in
  `audio_wasm.rs` (native now matches it but via its own consts — full native↔wasm dedup is a
  separate concern); CPU vs GPU particle default velocity sign (`-y` vs `+y`).

## Won't-fix / by-design

- `save.rs` `SAVE_KEY_BYTES` (embedded AEAD key) — **intentional**: the adjacent doc states a key in
  a client binary is not a secret; use `save_with_key`/`SaveKey` to separate saves and detect
  tampering. A knob already exists. Agent-flagged High, but it is not an issue.
- `components.rs` `Transform::default` scale `64.0` — real surprise default, but changing it is a
  **breaking** visual change for every example/game. Only a named-const for discoverability is safe;
  the value should not change.
- `tilemap` hex pitch `√3` and pathfinding octile `10/14` costs — legitimate geometry/heuristic math.
