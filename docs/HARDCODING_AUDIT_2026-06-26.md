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
| `renderer/lighting.rs` `MAX_LIGHTS = 16` (WGSL `array<GpuLight,16>` + `LightingUniforms` size) | `LightingConfig { max_lights }` resource (default `DEFAULT_MAX_LIGHTS`=16) — runtime-sized uniform (32 B `LightingHeader` + `[GpuLightData; N]`), WGSL array length substituted at shader build, `set_max_lights` rebuild; example `lighting_cap` + `lighting_cap_smoke.sh` (headless GPU A/B). Kept a uniform (not a storage buffer) → minimal GPU risk, default byte-identical. | #262 / v0.76.0 |
| `ui/system/focus_pass.rs` `SLIDER_STEP_FRAC 0.05` | `Slider::keyboard_step: Option<f32>` + `with_keyboard_step` + `resolved_keyboard_step()` + `DEFAULT_SLIDER_STEP_FRAC` const; example `slider_keyboard_step`. Non-breaking (private field → builder-only construction). | #263 / v0.77.0 |
| `tilemap/system.rs` autotile phase `0.37` stagger | `TileAnimationSet::stagger` field + `with_stagger` + `DEFAULT_TILE_ANIM_STAGGER` const; formula → unit-tested `stagger_phase` helper; example `tile_anim_stagger` (synced vs rippling). Manual `Default` to keep 0.37. | #264 / v0.78.0 |
| `ui/system/state.rs` `STICK_ACTIVATE 0.6`/`STICK_RELEASE 0.35` gamepad UI-nav deadzone | `StickNavConfig { activate, release }` resource (in `ui/focus.rs`) + `DEFAULT_STICK_ACTIVATE`/`DEFAULT_STICK_RELEASE` consts; `resolved()` clamps `release<=activate`; threaded through the focus pass; example `ui_nav_deadzone`. Auto-inserted → byte-identical default. | #267 / v0.79.0 |
| `network/native.rs` `READ_TIMEOUT 5ms` socket poll | `NetworkConfig::read_timeout: Duration` + `DEFAULT_READ_TIMEOUT` (5ms); clamped `≥1ms` in native.rs; no-op on WASM (event-driven); set via `connect_with_config`; example `mp_client` (2ms). | #268 / v0.80.0 |

## Open — Tier 2 (remaining fork-configurability knobs; additive, not yet done)

**Status (2026-06-29):** the clean, CI-verifiable Tier-2 knobs are **done** (8 shipped, #253–#268).
What remains is genuinely weaker — each is either invasive, hardware-gated, or editor/tooling-only
with no playable VISION example. Still additive + default-preserving (non-breaking), but pick
deliberately: none is a clean "field + example + unit test" win like the shipped ones.

Ordered best-remaining-first. "Verif." = can CI prove it works (CI is ubuntu, no GPU/pads/display).

| # | Site | Limit (fork must edit source) | Suggested API | Effort | Verif. | Notes / tradeoff |
|---|---|---|---|---|---|---|
| T2-a | `app/editor/settings.rs:52` `save_path("skeleton-engine", …)` | every fork shares ONE editor-settings dir on disk → settings collide between forks | crate-level `EditorSettings::app_id` (or an `App` setter) a fork overrides; default `"skeleton-engine"` | **Low** | partial (unit-test the path builder; the dir write is native-only) | Cleanest remaining. But editor/native-only + **no playable example** (the editor itself is the "feature") — breaks the VISION example rule, so it'd ship with a unit test + doc only. |
| T2-b | `renderer/context.rs:174,238` `desired_maximum_frame_latency: 2` | rhythm/fighting forks can't request latency 1 (lower input-to-photon) | opt-in `WindowOptions`/render-config field (default 2) | **Low** | **No** (latency is temporal; CI/screenshot can't measure it) | The default **2 is load-bearing on macOS** — the inline comment records that latency=1 blocked the AppKit main thread. Exposing it lets a fork reintroduce that bug; ship only with a strong doc warning. |
| T2-c | `input/gamepad.rs:78` `slots: [Option<Slot>;4]` (+ `pad < 4` guards) | no >4-pad local co-op | `pub const MAX_GAMEPADS` + widen array → `Vec`/const-generic; update the macOS + gilrs backends | **High** | **No** (needs >4 physical pads + per-OS hardware) | Hardware-gated AND invasive (fixed array → dynamic; touches both input backends). Slot-assignment logic could be unit-tested with synthetic connect events, but the real value (5th+ pad) is unprovable in CI. |
| T2-d | `material.rs:42` `params: [f32;4]` → WGSL `@group(2) var<uniform> params: vec4<f32>` | no richer per-material shader data (>4 floats) | variable-length / typed params payload | **High** | yes (lavapipe render test) | **Breaking-ish**: the `vec4<f32>` binding is the contract every existing `ShaderMaterial` shader declares. A clean non-breaking path (keep vec4 default + opt-in extra payload) is awkward. 4 floats already covers most uses (pack more, or sample a texture). Low value-for-effort. |
| T2-e | `app/editor/ui/gizmo_math.rs` `ROT_HANDLE_GAP 16`/`ROT_HIT_RADIUS 8` (world units) | rotation gizmo handle unusable at a non-default world scale | scale the handle by camera zoom, or expose in `EditorSettings` | **Med** | partial (math unit-testable; visual is editor-only) | Editor/native-only; better framed as a *fix* (scale-by-zoom) than a knob. |

**Recommendation if continuing Tier-2:** T2-a (editor app-id) is the only low-effort one, but it has
no game example. If the bar is "clean field + example + CI test" like the shipped knobs, that bar is
**met** — consider this chain effectively complete and pivot to: (1) the dungeon-merchant wishlist
board (ACTIVE empty, next ID EW-004), or (2) a new VISION breadth feature, or (3) Tier-3 dedup below.

## Resolution log — Tier 3 (shipped)

Behavior-preserving (value-identical) named-constant dedups for **genuinely** duplicated
same-logical-value literals. The "low value" caveat held: only a handful were real duplications.

| Item | Fix | PR / version |
|---|---|---|
| `1280×720` duplicated in `ViewportSize::default` vs `WindowConfig::default` (cross-struct drift risk) | `DEFAULT_WINDOW_WIDTH`/`DEFAULT_WINDOW_HEIGHT` (`resources/display.rs`, re-exported from `resources`) — single source of truth for both | #270 / v0.80.1 |
| UI sublayer z-step `0.001` in `checkbox_pass` + `slider_pass` | `UI_SUBLAYER_Z_STEP` (`ui/system.rs`, `pub(super)`) shared by both widget passes | #270 / v0.80.1 |
| Fade min-duration floor `.max(0.001)` in `audio/types.rs` + `audio/bus.rs` | `MIN_AUDIO_DURATION_SECS` (`audio.rs`) shared by both fade constructors | #270 / v0.80.1 |

## Open — Tier 3 (naming / dedup; low value)

- ~~Magic-number duplication~~ — the real duplications shipped (#270, above). **Two audit entries were
  re-classified as NOT duplications and deliberately left:** (1) the ~10 `0.001` sites in `src/audio/`
  are mostly **comparison epsilons** of *different units* (attack/release seconds vs `pan`/`pitch`
  ratios) — coupling them under one constant would be semantically wrong; only the 2 true duration
  *floors* were deduped. (2) `64*1024` in `network/event.rs` (WS message cap, already named
  `DEFAULT_MAX_MESSAGE_BYTES`) vs `scripting.rs` (`max_string_size`, Rhai string limit) are
  **semantically unrelated**, coincidentally equal; a shared const would couple unrelated limits, so
  both stay as-is.
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
