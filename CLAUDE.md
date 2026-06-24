# CLAUDE.md — skeleton-engine agent reference

> Version v1.6.142 | package `skeleton-engine` v0.67.0, library crate `engine` | wgpu-based Rust 2D game engine (wgpu 29, MSRV 1.95, CI pin Rust 1.95.0) | **Cargo workspace** (members `.` + `engine_reflect_derive` proc-macro)  
> WASM support: `cargo build --target wasm32-unknown-unknown` passes; an example game ships to
> the web via `cargo build --example` + `wasm-bindgen` (see `examples/games/coin_race/web/`)  
> Full API: `REFERENCE.html` | dev history / architecture decisions: `docs/HANDOFF.md`  
> **Versioning: pre-1.0 (0.x)** — MINOR = any release (incl. breaking), PATCH = bugfix; 1.0.0 later. (Reset from 10.7.0, 2026-06-17, pre-publish — see CHANGELOG 0.11.0.)

---

## Conversation language

- **User-facing reports/questions → Korean; everything else → English** — agent-to-agent
  (subagent/Workflow prompts, handoffs), code, paths, identifiers, command output, and
  file-written docs (see Documentation rules). User ruling 2026-06-18; supersedes the
  harness default (global `~/.claude` `"language"` setting cleared).

---

## Verification (run before declaring done)

A code/refactor change is **not done** until the **CI-equivalent** checks pass
*locally*. CI (`.github/workflows/ci.yml`) enforces these on every push, but run them
**before committing** so a regression never reaches `main`:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --target wasm32-unknown-unknown   # lib+bins — see wasm gotcha
cargo test --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps  # CI fails docs on broken intra-doc links
```

Or run all of them in order via `./scripts/verify.sh`.

- **Read the gate's exit code, don't pipe it:** `./scripts/verify.sh | tail` (or any
  trailing pipe) reports `tail`'s `0` and **hides** a real `fmt --check`/`clippy` failure
  (bit twice). Capture it: `./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?` (or
  `VERIFY_EXIT=$?`) is the authoritative verdict.
- **WASM gotcha:** do *not* gate on `--target wasm32 --all-targets` — it fails on the
  native-only examples (`platformer_game`/`mp_server`/`gpu_particles`, which pull in
  `rapier2d`/`tungstenite`/`GpuParticleEmitter`). The lib+bins build above (or `--lib`)
  is the real wasm gate.
- **CI is ubuntu only — `#[cfg(target_os = "macos")]`/`"windows"` code (and OS-only deps like
  `objc2-game-controller`) is never compiled/run on CI.** Green CI does **not** verify an
  OS-gated path; only the local build on that OS does, and OS-specific *runtime* behavior
  needs manual/hardware checking — so **don't merge an OS-gated change on green CI alone**
  (the macOS gamepad backend was merged on green CI **+** a hardware pad check). Build **both**
  cfg branches locally (`-D warnings`, esp. `dead_code`); one OS misses the other's lints.
- **Why this exists:** a prior refactor shipped declaring "done" on only `fmt --check` +
  `test --lib`, which misses the wasm-build + clippy regressions that the commands above
  catch. Don't narrow the bar.
- **Optional wasm smoke checks** (non-CI; need Chrome + a matching `wasm-bindgen-cli`):
  `./scripts/wasm_smoke.sh` (coin_race render), `wasm_save_smoke.sh` (AEAD localStorage),
  `wasm_audio_smoke.sh` (`WebAudio` lifecycle), `centered_text_smoke.sh` (EW-001 centered-text
  render). Each builds its example to wasm + runs it headless. See **`docs/WASM_SMOKES.md`**.

---

## Project direction (read `docs/VISION.md`)

This engine is a **skeleton**: a hackable, MIT-licensed, genre-agnostic 2D engine meant
to be forked and extended. Priorities — (1) open-source skeleton others can fork,
(2) personal foundation for 2D games, (3) learning vehicle.

When doing feature work, follow the core loop from `docs/VISION.md`:

- **A new feature is not done until a small, playable example game in `examples/`
  exercises it in real play.** The example is the acceptance test.
- **If the API feels awkward while writing that example, fix the API before release.**
- Keep new code fork-friendly: clear module boundaries and extension points. Breadth
  first, but not by leaving an unreadable mess.

---

## Module map

Where to read to find a given thing:

| Looking for | File |
|---------|------|
| Engine entry point, main loop, render orchestration, `load_image` (+ **`load_image_with_format`** = upload a texture with a caller-chosen `wgpu::TextureFormat`, e.g. `Rgba8Unorm` linear data textures sampled without the sRGB decode; default `load_image` stays `Rgba8UnormSrgb`; mirror `SpriteRenderer::load_texture_with_format`; example `texture_format`) | `src/app.rs` |
| Handle<T>, ImageAsset, ScriptAsset, AssetServer (asset load / caching / hot reload); **HotReloadable** trait + `App::register_hot_reloadable::<T>` (fork-friendly hot-reload extension point — built-in registries auto-registered; native-only) | `src/asset.rs` |
| TextureAtlas (uniform grid atlas), AtlasSprite (atlas tile render component) | `src/atlas.rs` |
| Reflect trait, ReflectValue (`F32`/`I32`/`Vec2`/`Bool`/`String`/`Color`, `#[non_exhaustive]`; runtime field read/write, egui Inspector integration); **`#[derive(Reflect)]`** proc-macro (in the `engine_reflect_derive` workspace crate, used via `engine_reflect_derive::Reflect`; path dev-dependency, not feature-gated) | `src/reflect.rs`, `engine_reflect_derive/` |
| DataTable, DataTableRegistry (schema-agnostic RON data tables; `App::load_data_table`, hot-reloaded, edited in the editor's Data Tables panel); `App::register_editable_component::<T>` (one-call reflect+clone+serde+inspector registration) | `src/data_table.rs`, `src/app/editor.rs` |
| **RonRegistry<V>**, **RonLoadable** (generic `name→value` RON registry with native canonical-path hot-reload; backs the particle/dialogue/animation-clip config registries AND is a **fork-friendly custom-asset registry** — a game impls `RonLoadable` for its own config type + uses `RonRegistry::load`/`get`/`names` w/o forking the engine; `load`/`reload_path` native-only [no wasm fs]; example `ron_registry`) | `src/ron_registry.rs` |
| ScriptAsset, ScriptRunner, ScriptingSystem, **ScriptRegistry** (Rhai scripting; `ScriptAsset`+loading live in `scripting/` not `asset.rs` — `ScriptRegistry` is the World resource that stores/loads/hot-reloads scripts, decoupled from `AssetServer`; `engine::ScriptAsset` re-exported at crate root) | `src/scripting.rs`, `src/scripting/{asset,loading,execution}.rs` |
| DebugUi (egui overlay, F1 toggle, custom panels via `ctx()`) | `src/debug_ui.rs` |
| In-game editor: F1 overlay + F2 **docked mode** (dock layout, viewport input gate, engine-level editor pause, gizmo (move + 8-handle resize + rotation handle) + screen-space UI gizmo, scene save, Data Tables panel, component copy/paste + entity-list search, prefab save/spawn, world-aligned grid overlay + cursor readout, bounds/colliders debug overlay, pathfinding-grid overlay (per-`Tilemap` walkable/blocked cells), audio bus mixer panel (per-bus volume sliders), particle live-tuner (inspector `ParticleEmitter` field drags + reset), lighting editor (`PointLight` add/edit + global `AmbientLight` control), state-machine editor (list-based `AnimationStateMachine` graph: states/transitions/params + edits), timeline editor (`Timeline` per-track keyframe list + playback controls), settings persistence (RON config), **Tile Paint** = viewport tile-painting for selected `Tilemap` entities — brush(N×N)/rectangle/bucket tools + eyedropper(Alt+click), L-paint/R-erase/digit-pick, **image-swatch palette** (atlas thumbnails via egui `register_native_texture`), stroke-level undo, visual-only/native; example `tile_paint`; **localization** — Korean-by-default editor via `i18n::tr(en, ko)` (English = source of truth, Korean inline) + `EditorLocale` (RON-persisted, EN/한국어 toolbar toggle); CJK renders via the **bundled Noto Sans KR (Hangul-only subset, ~2.3 MB — keeps crate < crates.io 10 MB limit; regen via `scripts/subset_korean_font.sh`, OFL in `assets/fonts/NotoSansKR-OFL.txt`)** installed as an egui fallback in `DebugUi::new_with_ctx` (`install_korean_fallback`, fixes □ tofu — also covers Korean data-table/tag values)) | `src/app/editor/` (+ `i18n.rs`) |
| Full public API re-export list | `src/lib.rs` |
| Entity / Component / Resource / Query (`query`/`query2`/`query3`/`query4`/`query_opt2`; **mutable** `query_mut`/`query2_mut`/`query3_mut` — multi-component `*_mut` use `HashMap::get_disjoint_mut`, distinct types, no collect-then-`get_mut`); (+ `register_persistent` survives scene reset, via `App`) | `src/ecs/world.rs`, `src/app.rs` |
| Event bus (`Events<E>`) | `src/ecs/events.rs` |
| `System` trait | `src/ecs/system.rs` |
| Scene transitions (Scene, SceneCmd, SceneChange; `App::set_scene` = Replace, **`App::push_scene`/`pop_scene`** = stack overlays e.g. pause menu), SystemRegistrar (labeled system registration from `on_enter`) | `src/scene.rs` |
| Transform, Sprite | `src/components.rs` |
| WindowConfig, GameState, ShouldQuit, DebugDraw (filled rects via `rect_filled_z`; `DebugShape` is `#[non_exhaustive]`); **WindowOptions** (opt-in resource, `ImeConfig`-style — absent = default resizable window, so NO `WindowConfig` field added/breaking: `resizable` / `mode: WindowMode` (Windowed/**BorderlessFullscreen**) / `lock_aspect: Option<f32>` (live resize corrected back to ratio, native-only — `window.rs` `Resized` re-derives height from width, converges in one step); example `window_mode`); **DesignResolution** (opt-in fixed virtual canvas, e.g. 1280×720: `ViewportSize`=design size, all `DrawRect`/`DrawText`/sprites/particles authored in design space + scaled to the window with a **uniform centered letterbox**; cursor + `screen_to_world` mapped back to design space; ignored in docked editor; example `design_resolution`) + **Letterbox** (the computed transform, inserted every frame, identity when no `DesignResolution`: `clip_scale` (centered = pure clip scale, post-multiplied on each scene projection via `camera::apply_letterbox`) + `px_scale`/`px_offset` (logical-px scale+offset for text/cursor); OFF-path byte-identical); **TimeScale** (global `dt` multiplier for scene systems → hit-stop/slow-mo, `App::set_time_scale`) + **RealDt** (real unscaled per-frame dt, for systems that must opt out of time-scaling) | `src/resources.rs` |
| Camera (coordinate transforms, zoom; `screen_to_world`/`world_to_screen`; `bounds` + `clamp_to_bounds` world-bounds clamp, auto-applied by App after follow) | `src/camera.rs` |
| ParallaxLayer (`factor: Vec2` depth scroll: 1=world-locked, 0=screen-locked, >1=foreground; lazy base capture), ParallaxSystem (user-added: `pos = base + (cam - cam_ref) * (1 - factor)`; reads `Camera`, add after camera-mover systems; example `parallax_scroll`) | `src/parallax.rs` |
| InputState, InputMap (keyboard + gamepad bindings: `bind_gamepad_button`/`bind_gamepad_axis` + `AxisBinding`, `*_with_gamepad` resolution) | `src/input/` |
| GamepadState, GamepadButton, GamepadAxis (gilrs-fed on Windows/Linux; **macOS uses a GameController-framework backend** — `src/input/gamepad_macos.rs`, polled from `about_to_wait`, since gilrs/IOKit-HID can't read modern Xbox/PS5 pads there; gilrs left uninit on macOS; diagnostic: example `gamepad_probe`) | `src/input/gamepad.rs`, `src/input/gamepad_macos.rs` |
| PhysicsWorld, PhysicsBody, PhysicsSystem (syncs body position **and rotation** → Transform), CollisionEvent, BodyHandle/ColliderHandle (opaque newtypes; `.raw()` = rapier escape hatch) | `src/physics/` |
| CharacterController (+ `request_drop`/`is_dropping` for one-way), RaycastHit, cast_ray, cast_ray_with_normal, move_character | `src/physics/character.rs`, `src/physics/world.rs` |
| add_kinematic_box, add_kinematic_circle, add_static_from_tilemap (one-shot), sync_static_from_tilemap + TileColliderIndex (incremental — diff add/remove for runtime-mutated tilemaps), **TilemapColliders** component + SolidTiles + `sync_tilemap_entity_colliders`/`App::sync_tilemap_colliders` (opt-in: keep tile colliders synced when the editor's Tile Paint or a game's `set_tile` mutates the map; example `dig_quest`), TileCollider, set_one_way/is_one_way | `src/physics/world.rs` |
| add_revolute_joint, add_distance_joint, add_prismatic_joint, remove_joint (return/take engine `JointHandle` newtype wrapping rapier; example: `crane_wrecking_ball`) | `src/physics/world/joints.rs` |
| SpatialGrid, Collider, CollisionLayer (SpatialGrid is mirrored to a World resource by CollisionGridSystem) | `src/collision/` |
| BehaviorTree, BehaviorNode, Sequence, Selector, Inverter, AlwaysSucceed, BehaviorSystem, Blackboard, BlackboardValue (`Path` variant + `set_path`/`get_path`) | `src/behavior.rs` |
| Seek, Flee, Arrive, Wander, SteeringVelocity, SteeringSystem (steering behaviors; O(1) per-entity component lookup; `Wander::direction_fn`/`with_direction_fn` overrides the direction picker — swap in real RNG without forking the system) | `src/steering.rs` |
| PathGrid, find_path (4-dir), find_path_diagonal (8-dir A*, octile heuristic, no corner-cut), PathGrid::from_tilemap | `src/pathfinding.rs` |
| AnimationPlayer, AnimationClip, AnimationSystem, BlendWeight (crossfade = true 2-UV shader-lerp; renderer `mix`es from/to frames) | `src/animation/player.rs`, `src/animation/system.rs` |
| AnimationClipSet, AnimationClipRegistry, ClipSetError (data-driven animation: named clips loaded from RON `(atlas, clips)`, frame indices → UvRect; `App::load_animation_clips` = registry + DataTable-style hot-reload) | `src/animation/clip_set.rs` |
| UvRect, BlendUv (GPU UV-region types, consumed engine-wide) | `src/renderer/uv.rs` |
| AnimationStateMachine, StateMachineSystem, TransitionCond, AnimParam (per-transition crossfade via `add_transition_crossfade`; editor **State Machine panel** — `state_names`/`state`/`param` accessors + `set_current_state`/`set_state_clip`/`remove_state`/`remove_transition`/`set_transition_conditions`/`set_transition_crossfade` edit ops drive a list-based graph editor in the docked inspector — live param editing + add-transition + condition add/remove; example `sm_crossfade`) | `src/animation/state_machine.rs` |
| BlendTree1D, BlendEntry, BlendTreeSystem (1D parameter-driven auto transitions + crossfade) | `src/animation/blend_tree.rs`, `src/animation/blend_system.rs` |
| SkeletalAnimator, SkeletalClip, BoneTrack, BoneKeyframe, SkeletalAnimationSystem, SkeletonBuilder (2D cutout skeletal animation) | `src/skeletal.rs` (details: `docs/SKELETAL.md`) |
| UI (UiNode, Button, Label, TextInput, ScrollView, Panel, LayoutSystem, UiEvent [now `PartialEq`]); **keyboard focus** — `UiFocus` resource (`Option<Entity>`, auto-inserted) + `UiSystem`'s focus pass (`src/ui/system/focus_pass.rs`): **Tab/Shift+Tab** (or gamepad **D-pad Down/Up** / **left stick** Down/Up) cycle focus across focusable widgets (Button/TextInput/Slider/CheckBox, by entity index, skips hidden/disabled), draws a focus ring (styled by the **`FocusRingStyle`** resource — `color`/`thickness`/`enabled` + optional `pulse_hz`/`pulse_min_alpha` ("breathing" alpha pulse, `pulse_hz=0` default = steady ring; clock accumulated in `UiSystem`) + optional **`corner_radius`** (rounds the ring; `0.0` default = sharp 4-bar, byte-identical), auto-inserted with the default amber 3px ring; `enabled=false` or `thickness<=0` suppresses it for a game's own indicator), **Enter/Space** (or gamepad **A**/South) activate (button click / checkbox toggle), **←/→** (or D-pad/left-stick Left/Right) nudge a focused Slider, click also focuses (by z-order — see pointer capture); **pointer capture** — a shared per-frame occlusion map (`src/ui/system/capture.rs`, internal `PointerCapture`, rebuilt once by `UiSystem`) means a widget covered by a higher-z widget kind (e.g. a `Panel`, which blocks at its drawn `z-0.01` background) does NOT receive clicks/hover/scroll/click-to-focus through it — the topmost pointer-opaque surface absorbs the interaction; ties broken by z then entity index, `Label` excluded (not pointer-opaque); gamepad nav is folded into `InputSnapshot` from the first connected `GamepadState` pad (`src/ui/system/state.rs`, optional — no pad = no-op; **D-pad + left analog stick**, the stick edge-detected via `StickNav` hysteresis [0.6 activate / 0.35 release] so one push = one step, no auto-repeat); **`DrawRect`** gains `corner_radius` + `border` (`with_corner_radius`/`with_border`; filled rounded rect or inset outline ring) via a dedicated UI SDF pipeline (`src/renderer/shaders/ui.wgsl` + `UiInstanceRaw`; `0,0` = fast path identical to before, sprite pipeline untouched); examples `ui_focus`/`ui_rounded`; `UiSystem`/`SteeringSystem` hold reused scratch buffers → construct via `::default()`/`::new()` | `src/ui/` |
| Slider (horizontal slider), CheckBox (toggle checkbox) | `src/ui/slider.rs`, `src/ui/checkbox.rs` |
| LocalizedText (key bound to a widget), LocalizationSystem (resolves `LocaleResource::t` into Label/Button/CheckBox each frame) | `src/ui/localized.rs` |
| Tag, EntityDef (+ `components` map), SceneDef, Prefab, spawn_entity_def, spawn_scene_def, **SerdeComponentRegistry** + `App::register_serde_component::<T>` (any serde component persists to scene RON; UI widgets + `AnimationStateMachine`/`Timeline`/`CameraTarget` auto-registered; `SerdeComponentRegistry` now lives in `src/serde_registry.rs`, re-exported from prefab) | `src/prefab.rs`, `src/serde_registry.rs` |
| Timer, **`Tween<T: Lerp = f32>`** (generic over the value type — `Tween<Vec2>`/`Tween<Color>`; default `f32` keeps old call sites + `TweenSequence` unchanged), TweenSequence (chained multi-segment `f32` tweens: per-segment easing + `looping`, carries leftover `dt` across segments; example `tween_sequence`), **Easing** (`#[non_exhaustive]`; Linear/EaseIn/Out/InOut/InBack/OutBack/InBounce/OutBounce/InElastic/OutElastic), Lerp (general interpolation trait); game-feel example `juice_demo` (hit-stop + shake + easing + fade) | `src/timer.rs`, `src/tween.rs` |
| **DialogueBox**, **DialogueChoice**, **DialogueSystem** (speaker + typewriter text box: per-line reveal at `chars_per_sec`, two-stage `advance()`, optional `portrait`, UTF-8-safe; **localization** — `localized(speaker_key, line_keys)` + `resolve(&LocaleResource)` fill `lines`/`speaker` from keys WITHOUT touching reveal, re-resolved each frame by `DialogueSystem` so live `set_locale` retranslates; **branching** — `choices: Vec<(line, Vec<DialogueChoice>)>`/`with_choices`/`pending_choices`/`choose(i)` jump to `goto` [OOB clamps to end], `advance()` no-ops while a decision pends); **data-driven (`tree.rs`)** — `DialogueTree` (ordered named nodes, line literal-or-`line_key`, `goto`-by-id choices) flattens to a `DialogueBox` (node order = line index); `DialogueRegistry` + `App::load_dialogue(name, path)` load+hot-reload like clip/particle sets; **conditional/effect (`vars.rs`)** — `DialogueVars` resource of `DialogueValue` (Bool/Int/Float/Str); `DialogueChoice.cond: DialogueCond` (Eq/Ne/Gt/Lt/Ge/Le) gates visibility, `.effect: DialogueEffect` (SetVar / EmitEvent→`Events<DialogueEvent>`); world-level `dialogue::advance`/`choose` honor conds + apply effects via `visible_choices`/`is_choosing`/`advance_with`/`choose_visible` (plain `advance`/`choose` stay for the no-vars case); RON authors cond/effect inline via IMPLICIT_SOME; builders `when`/`then`; **render** — `DialogueSystem` draws speaker+text via `TextQueue` AND, when a box sets `portrait` (`with_portrait`, `Handle<ImageAsset>`), draws it left via `UiImageQueue` + shifts the text right (no portrait = original left-margin layout, byte-identical); **styling** — `DialogueStyle` (opt-in resource: layout offsets/font sizes/colors for speaker/body/choices/hint/portrait + fallback viewport; vertical positions are offsets-up-from-bottom; absent = `default()` reproduces the original look exactly, so non-breaking) restyles `DialogueSystem` without forking; examples `dialogue_demo`/`dialogue_portrait` (per-speaker portraits)/`dialogue_branching` (live en↔ko)/`dialogue_quest` (RON tree + cond + event hooks)/`dialogue_style` (T toggles a custom `DialogueStyle`)) | `src/dialogue/` (`mod.rs`, `tree.rs`, `vars.rs`) |
| Coroutine, CoroutineRunner, CoroutineSystem (imperative timed-action sequencer — `wait(secs)` / `run(\|&mut World\|)` / `run_for(dur, \|&mut World, t\|)` steps chained via a builder; `CoroutineSystem` removes the `CoroutineRunner` resource, ticks all coroutines passing `&mut World` to the closures, then reinserts — closures must not re-enter the runner; carries leftover `dt` across steps; distinct from `Timeline` (data keyframes) / `TweenSequence` (value interp); example `coroutine_demo`) | `src/coroutine.rs` |
| Timeline, Track, Keyframe, TimelineSystem (keyframe cutscenes → entity Transform/Sprite; **CameraTarget** marker + `zoom` track route a timeline into the **Camera** resource as a virtual rig; `Track::add`/`keyframes`/`len`/`remove`/`set_time`/`set_value`/`set_easing`/`clear` editor accessors+edit ops drive the docked **Timeline panel** (per-track keyframe list + add-keyframe + per-type value editing + playback controls); example: `timeline_cutscene`) | `src/timeline.rs` |
| History (generic snapshot undo/redo for grid puzzles, turn-based, editors) | `src/history.rs` |
| ParticleEmitter (+ **`gravity: Vec2`** per-particle accel + **`emit_shape: EmitShape`** Point/Circle/Ring/Box spawn scatter — `with_gravity`/`with_emit_shape`; additive, ZERO/Point = prior behavior; example `particles_showcase`), ParticleSystem, ParticleBurst (one-shot burst + `ParticleEmitter::burst()`); ParticleConfigSet/ParticleConfigRegistry/ClipSetError-style ParticleConfigError (data-driven emitter configs from RON, **incl. `gravity` + `emit_shape`** [`emit_shape: Circle(radius: 10.0)` etc.]; `App::load_particle_configs` = registry + DataTable-style hot-reload); **`GpuParticleEmitter` also mirrors `gravity` (per-particle, integrated in the compute shader) + `emit_shape` (CPU-sampled at emit)** — `src/gpu_particle.rs` + `src/renderer/gpu_particle.rs` (80-byte `GpuParticle`); **`ParticleConfigSet::gpu_emitter(name)`** = RON→`GpuParticleEmitter` builder (native-only; mirrors `emitter()`, square `size`=width `size.0`, texture/z ignored), example `gpu_particles` loads `gpu_particles.ron` via `load_particle_configs`+`gpu_emitter` | `src/particle/` (`mod.rs`, `config_set.rs`) |
| Tilemap (+ runtime `set_tile`/`get_tile`/`cell_at_world`/`cell_center_world`/`cell_z`/`cell_render_size`/`dims`; **`TilemapProjection` (`#[non_exhaustive]`) Orthographic[default]/Isometric/Hexagonal/HexagonalFlat** via `with_projection` — iso = 2:1 diamond [picking = inverse diamond + round, depth-sorted `z=row+col`]; hex = pointy-top odd-r offset [`cell_center_world` row pitch `ts·√3/2` + odd-row half-shift, picking = pixel→axial→cube-round, `z=-1` no overlap, taller sprite via `cell_render_size`]; **HexagonalFlat = flat-top odd-q offset** [90°-rotated mirror: col pitch `ts·√3/2` + odd-col half-shift-down, `ts`=flat-to-flat height, wider sprite `ts·2/√3 × ts`, flat-top pixel→axial picking]; all branch in `cell_center_world`/`cell_at_world`/`cell_z`/`cell_render_size`; examples `iso_tilemap`/`hex_tilemap`/`hex_tilemap_flat`), TilemapAtlas, TilemapSystem (reactive — diffs cached grid, updates only changed cells; places tiles via `cell_center_world`+`cell_render_size`+`cell_z`), **TilemapAutotile** (neighbor-bitmask autotiling; ONE component, `mode: AutotileMode` = `Single{mask_to_tile}` [any-nonzero connects: `edge_16`/`blob_47`] or `Multi{rules: Vec<TerrainRule>}` [per-terrain same-value: `multi_edge_16`]; `Neighborhood::Edge4`/`Blob8` (square — also work on **iso**, same `tiles[][]` topology) + **`Hex6`** (pointy odd-r) / **`Hex6Flat`** (flat odd-q) hex neighborhoods (6 parity-aware nbrs, mask 0..64) + `hex_6`/`hex_6_flat` 64-tile constructors, `with_oob_filled`, `compute_tile_mask`/`compute_tile_mask_typed`; unified from the old separate `MultiTerrainAutotile`, ghost `ConnectRule` dropped — v0.30.0; iso+hex autotile + `hex_autotile`/`hex_autotile_flat` examples — v0.39.0; **64-tile hex blob atlas** v0.43.0 [`gen_hex_autotile_sheet` procedurally writes `examples/assets/hex_autotile{,_flat}.png` where tile index == the Hex6/Hex6Flat mask, the hex analogue of square `blob_47`; full-blob examples `hex_blob_autotile`/`hex_blob_autotile_flat`]); **animated tiles**: TileAnimation (frames + `frame_time`, `frame_at`) + TileAnimationSet (tilemap-entity component mapping tile value → animation) + AnimatedTileCell (per-tile-entity tag w/ precomputed frame UVs) + AnimatedTileSystem (cycles tagged cells' UvRect each frame — render-only, does NOT bump generation, so the non-animated fast path is untouched; `TilemapSystem` tags animated cells at spawn; example `animated_tiles`) | `src/tilemap/` (`mod.rs` data model, `autotile.rs`, `system.rs`, `animation.rs`) |
| AudioManager (playback, positional audio, bus mixer (`assign_bus`/`set_bus_volume`/`bus_volume`/`bus_names` — `bus_names` lists all buses for the editor mixer panel), fades + **track-to-track crossfade** (`crossfade` — fades the old track out + the new track in via an internal temp channel, reusing the `Fade`/`update` infra; example `music_crossfade`); **ducking/sidechain** via `duck_bus`/`release_bus`/`bus_duck`/`set_sidechain`/`clear_sidechain` — BusDuck/Sidechain), AudioSystem (built-in system that ticks `update(dt)` so fades + ducks progress; SFX file-bytes cache); **AudioManager is native-only** (`cfg(not(wasm32))`, rodio) — wasm has **`WebAudio`** (`src/audio_wasm.rs`, wasm-only: Web Audio `AudioContext` — `new`/`play(bytes)` fire-and-forget SFX + **synthesized tone** `play_tone(freq,dur,vol)`/`play_tone_on_bus` (`OscillatorNode`→gain attack/release envelope, no clip bytes) + **named tone channels** `play_tone_on_channel(ch,freq,dur,vol,bus)`/`is_channel_playing(ch)`/`set_low_pass(ch,hz)`/`clear_low_pass(ch)` (tracked per-channel `OscillatorNode` + optional `BiquadFilterNode` low-pass applied on next play — replay cuts the prior tone) + **controllable per-source SFX** `play_sfx(bytes) -> Sfx` (source→`StereoPannerNode`→per-source `GainNode`→master; `Sfx::set_pan`/`set_volume`/`is_playing`/`stop`, panner+gain created sync so set_pan/volume work pre-decode) + **master GainNode** `set_volume`/`volume` (all playback routes through it) + **looping music channel** `play_music`/`stop_music` (single channel) + **track-to-track crossfade** `crossfade_music` (audio-clock `linear_ramp_to_value_at_time` on per-music gain nodes — no per-frame `update()`; no-current-track = fade-in) + `suspend`/`resume` pause-all + **state accessors** `is_running` / `is_music_playing` + **named mixer buses** `set_bus_volume`/`bus_volume`/`bus_names` + `play_on_bus`/`play_sfx_on_bus` + **manual ducking** `duck_bus`/`release_bus`/`bus_duck` (a bus = a `duck → volume → master` 2-gain chain; sounds connect to `duck`, ducking ramps `duck` independent of `volume`; lazy-created, volume-only buses persist; audio-clock ramps, no per-frame `update()`, `dur<=0`=instant `set_value`) + **2D positional** `play_at(bytes, source, listener, max_dist) -> Sfx` (+ `play_at_on_bus(…, bus)` = positional routed through a named bus) + `Sfx::update_position`/`volume`/`pan` (vol = 1−clamp(dist/max), pan = clamp(dx/max), native parity; `play_at` routes to master) + **named positional channels** `play_at_on_channel`/`update_position`/`stop_channel` (a looping `Sfx`+panner tracked by name in `spatial_channels`; backs the facade's tracked positional); **automatic sidechain stays native-only** (needs continuous trigger-activity eval — drive ducking manually instead); example `web_audio` (wasm-only — drives the whole surface incl. `play_sfx` + buses + crossfade + ducking + positional + self-checks the lifecycle, see `scripts/wasm_audio_smoke.sh`)); **cross-platform `Audio` facade** (`src/audio_facade.rs`, un-gated) = ONE API over native `AudioManager` / wasm `WebAudio` so a dual-target game writes audio with **ZERO `cfg` guards** — intersection API (`play_sfx`/`play_sfx_on_bus`, **synthesized** `play_tone`/`play_tone_on_bus` [sine, no clip bytes — native rodio synth / wasm `OscillatorNode`], **named tone channels** `play_tone_on_channel`/`is_channel_playing`/`set_low_pass`/`clear_low_pass` [sustained/trackable tone + low-pass filter — native `AudioEffect.low_pass_hz` / wasm `BiquadFilterNode`, applied on next play], **tracked 2D positional** `play_at_on_channel`/`update_position`/`stop_channel` [a LOOPING positional sound on a named channel whose dist-volume + stereo pan track a moving source each frame — native `AudioManager::play_bytes_at` + positional channel / wasm looping `Sfx`+`StereoPannerNode` kept in a `spatial_channels` map], `play_music`/`crossfade_music`/`stop_music`, `set_master_volume`, `set_bus_volume`/`bus_volume`, `duck_bus`/`release_bus`/`bus_duck`, `resume` [web `AudioContext` unlock after a gesture; native no-op], `update` [native fades/ducks tick; web no-op]) + `AudioFacadeSystem`; native routes unrouted SFX/music/tones through a conventional `"master"` bus (round-robin 16-voice SFX channels, shared by `play_sfx`+`play_tone`) so a **named-bus sound bypasses `set_master_volume` on native** (native buses don't nest — documented); backed by additive `AudioManager::play_bytes`/`crossfade_bytes`; **excludes** UNTRACKED positional one-shots (`play_at`) + per-channel effects beyond low-pass (pitch/attack/release) + automatic sidechain (native-only, use the backend directly); examples `audio_facade` (native + web, same code — incl. named-channel BGM + low-pass via `G`/`L`) + `positional_audio` (orbiting source + arrow-key listener move, native+web) + games `survivor`/`shooter`/`settings_menu` adopt the facade (cfg-guard-free audio — sfx now play on web too) | `src/audio.rs`, `src/audio/`, `src/audio_wasm.rs`, `src/audio_facade.rs` |
| save / load (AEAD, player saves) / write_ron / read_ron (plaintext, design-time assets) / load_or_default / exists / delete / save_path / SaveError; **save_versioned + load_migrated + SaveMigrator** (versioned-schema save migration over ron::Value); **wasm parity: ALL of `save`/`load`/`save_versioned`/`load_migrated` + `write_ron`/`read_ron`/`exists`/`delete` work on wasm** via `localStorage` (internal `wasm_storage`, web-sys `Storage`) — the AEAD core (ChaCha20-Poly1305) is cross-platform; on wasm the encrypted blob is **hex-encoded** into localStorage, nonce RNG is `OsRng` (getrandom js); localStorage is inspectable so the embedded key = obfuscation+tamper-detection, not secrecy (same as the native file); the wasm AEAD localStorage round-trip is **browser-verified** by example `wasm_save` + `scripts/wasm_save_smoke.sh` (headless 7/7); examples `save_counter` (plaintext), `save_encrypted` (AEAD, native), `wasm_save` (wasm AEAD localStorage self-check) | `src/save.rs` |
| NetworkClient (native=tungstenite thread, wasm=web-sys), NetworkSystem (polls → `Events<NetworkEvent>`), NetworkEvent, NetworkConfig (queue/size caps), RemoteEntities (`id→Entity` lifecycle), SnapshotBuffer<T: Lerp> (generic per-entity interpolation buffer); relay demo `mp_server`/`mp_client`, authoritative game `examples/games/coin_race`, client-prediction `examples/games/predict_shooter`, interpolation-only `examples/games/orbital_dodger`, AOI-streaming/interest-managed `examples/games/salvage_run` | `src/network.rs` |
| PostProcessConfig, PostProcessRenderer, **Tonemap** (HDR tone-mapping: `PostProcessConfig.hdr` renders the scene into an `Rgba16Float` intermediate so `>1.0` colours survive, then the post pass applies `tonemap` [None/Reinhard/AcesFilmic] + `exposure`; default off = byte-identical. HDR feeds the sprite + UI-primitive + shader-material + render-plugin **+ GPU-particle** passes (all format-matched — material/UI v0.59.0, GPU particles v0.62.2: `GpuParticleRenderer` lazily builds + caches a render pipeline per non-surface target format, mirroring the sprite cache). Examples `tonemap`, `gpu_particles` (H toggles HDR post)); **real multi-pass bloom** (`PostProcessConfig.bloom` opt-in [+ `bloom_iterations` 0..=8 = **mip-pyramid depth** since v0.67.0], requires `enabled`; **mip-chain "dual filter"** [Jimenez/CoD]: bright-pass → 13-tap downsample chain (mip pyramid) → 3×3-tent additive upsample chain → additive composite onto the scene intermediate *before* post, which then skips its inline 4-tap via the `bloom_enabled` uniform flag; wider/smoother/energy-preserving glow vs the old fixed-half-res separable Gaussian — same `pub(crate)` API, so frame.rs/post_lighting.rs unchanged; `BloomRenderer` in `src/renderer/bloom.rs`, **single-target reconfigure** like post/lighting [NOT the per-format pipeline-cache], format-matched to the scene intermediate so it works under HDR; `false` default = byte-identical inline bloom; example `bloom`) | `src/renderer/post_process.rs`, `src/renderer/bloom.rs` |
| PointLight, AmbientLight, LightingRenderer (2D point-light pass, nearest-16 cull; native-only) | `src/renderer/lighting.rs` |
| RenderTarget, OffscreenCamera, create_render_target (render-to-texture; each offscreen target submits its **own** command buffer so it uses its own camera — exclude RT-display sprites via `layer_mask`; example: `security_camera`); **`App::create_render_target_with_format(name,w,h,format)`** = caller-chosen pixel format (e.g. `wgpu::TextureFormat::Rgba16Float` HDR, or linear `Rgba8Unorm`); `RenderTarget` stores its `format()`; the offscreen pass threads the RT's format into a **format-matched sprite pipeline** — `SpriteRenderer` lazily builds + caches an extra sprite pipeline per non-surface target format (`extra_sprite_pipelines`, keyed by format; the surface-format fast path is untouched, no per-frame cost), so an `OffscreenCamera` renders correctly into an HDR/linear RT (no wgpu format-mismatch). Tone-mapping an HDR RT for display is the game's job (the example uses an exposure-multiply on the display sprite's colour — sprite `Color` is f32, un-clamped, so values >1.0 reach the shader). **Material + UI pipelines are also format-matched** (v0.59.0; `MaterialRenderer.custom_pipelines` keyed by `(hash, format)`, `extra_ui_pipelines` per format) — a `ShaderMaterial` in an offscreen non-surface RT renders (example `offscreen_material`); UI primitives render through the HDR post intermediate. Examples `hdr_render_target` (over-bright scene → `Rgba16Float` vs default 8-bit RT side by side; lower exposure → HDR keeps core-vs-mid distinct, 8-bit collapses them). **Renderability query + fallback** (v0.65.0): **`RenderCapabilities`** resource (re-exported, inserted at GPU init) — `supports_render_target(format)` / `surface_format()` let a system/`on_enter` ask whether a format (e.g. `Rgba16Float`) is a usable color render target *before* requesting it (float RTs need `EXT_color_buffer_float` on WebGL2); `create_render_target_with_format` also **auto-falls-back** to the surface format + warns when a requested format isn't renderable (`GpuContext::resolve_render_target_format`, backed by the now-retained `adapter`); example `render_format_query` | `src/renderer/render_target.rs`, `src/renderer/context.rs`, `src/renderer/sprite.rs`, `src/app/render/offscreen.rs`, `src/app/assets.rs`, `src/components.rs` |
| **RenderPlugin** trait + `App::add_render_plugin` (fork-friendly custom render-pass hook — `record(ctx: &mut FrameContext, world, viewport)` runs per-frame after the sprite/UI/particle passes, before post/lighting; `FrameContext.format` lets a plugin build its own pipeline; additive — no-op when none registered; native+wasm; example `render_plugin`) | `src/renderer/render_plugin.rs`, `src/app/render.rs` (dispatch) |
| **ShaderMaterial** (per-entity custom fragment shader component — attach to an entity to replace the built-in sprite frag shader; bindings `@group(1)` texture/sampler + `@group(2)` `params: vec4<f32>`; renderer compiles + caches one pipeline per source hash via `MaterialRenderer`; example `shader_material`) | `src/material.rs`, `src/renderer/sprite/material.rs` |
| **NineSlice** (9-patch scalable sprite component — `border: [f32;4]` world-px + `uv_border: [f32;4]` UV-fraction, both `[left,right,top,bottom]`; the sprite pass emits 9 sub-quads (corners fixed-size, edges/center stretched) instead of 1 when a `NineSlice` is present, additive — ordinary sprites unchanged; rotates rigidly; not for `AtlasSprite`/`ShaderMaterial`; example `nine_slice`) | `src/nine_slice.rs`, `src/renderer/sprite.rs` (9-quad branch) |
| DrawText, TextQueue, TextAlign (Left/Center/Right/**End**/**Auto** — Auto right-aligns RTL automatically), TextAnchor (screen-space text, top-left origin; `DrawText::centered` anchors at text center; place at a world position via `Camera::world_to_screen`); **bidi/RTL shaping is built in** (`Shaping::Advanced`); `ExtraFonts` resource (multi-script font fallback alongside `FontData`); examples `rtl_text` (Hebrew + multi-font), `centered_text` (`DrawText::centered` off-center-x visual check — guide lines through each `position.x`, EW-001 demo; **also ships to the web** — `examples/centered_text/web/` + headless render smoke `scripts/centered_text_smoke.sh`) | `src/renderer/text.rs` |
| wgpu render pipeline (rarely edited directly) | `src/renderer/` |

---

## Core patterns & task recipes

Detailed in **`docs/PATTERNS.md`**:

- **Architecture patterns** — ECS query API (`query2`/`query_opt2`), borrow-checker
  workaround (collect entities then `get_mut`), render-layer separation
  (`AnimationSystem` → `UvRect` → renderer), **render-target-format-aware pipeline cache**
  (a new render pass keys its pipeline by *target* format, not `gpu.config.format`, so it
  survives an offscreen/HDR RT — else it silently vanishes; sprite/UI/material/GPU-particle),
  UI system order (`LayoutSystem` before `UiSystem`), animation state-machine order
  (`StateMachineSystem` after `AnimationSystem`), `PhysicsWorld` encapsulation accessors.
- **Task recipes** — adding a component / system / resource / event, and scene transitions.

---

## Agent working notes

### Context management

The longer a session runs, the more accumulated context degrades response quality. Split the approach by task type:

| Situation | Recommended approach |
|------|-----------|
| Single-file edit (clear requirements) | Edit directly in the main session |
| Feature spanning multiple files | Split out into a Task subagent |
| Exploration needs 3+ files | Explore subagent |
| Writing code after a long conversation | Task subagent (avoid context pollution) |

### Efficient exploration

- Locate symbols/keywords with `grep` before reading whole files
- If the path is already known, use Read directly (no Explore subagent needed)
- Reading order: `src/lib.rs` → module map → narrow down to the target file

### Subagent prompt principles

A subagent starts without knowing the current conversation context. Always include in the prompt:

1. **Paths to edit** (absolute paths)
2. **Patterns to apply** — pass a summary of this file's core-pattern sections (borrow workaround, layer separation, etc.)
3. **Expected result** — what behavior should change

---

## Documentation rules

- **Language**: Write doc prose in **English** to minimize token cost (English ≈ ⅓ the
  tokens of equivalent Korean). Code, file paths, identifier tables, and API names stay
  as written.
- **Exceptions kept in Korean**: the beginner glossary (`docs/ENGINE_TERMS_FOR_BEGINNERS.md`)
  and personal/gitignored one-off prompt or plan docs.
- New `docs/HANDOFF.md` entries are written in English.
- **Length**: keep `CLAUDE.md` / `AGENTS.md` ≤200 lines. Prefer concision, but **do not
  drop needed content to hit the limit** — when trimming would risk losing information,
  move the detail into a new `docs/*.md` (e.g. `docs/PATTERNS.md`, `docs/SUBSYSTEM.md`)
  and leave a one-line reference here.

---

## Document map

| Document | Purpose |
|------|------|
| `CLAUDE.md` (this file) | Agent quick reference — module map, task checklists |
| `docs/PATTERNS.md` | Core architecture patterns + task recipes (extracted from this file) |
| `docs/MACOS_FFI.md` | How to add an objc2 Apple-framework binding (version-pin, discover the API from registry source, feature flags) — e.g. the macOS GameController gamepad backend |
| `REFERENCE.html` | Full public API + code examples (detailed) |
| `docs/HANDOFF.md` | Per-phase dev history, background on architecture decisions |

> **Growth strategy**: when content would push this file past 200 lines, move detail into
> a `docs/*.md` (a new subsystem doc, or `docs/PATTERNS.md`) and leave only a one-line
> reference here. Never drop needed content just to stay under the limit.
