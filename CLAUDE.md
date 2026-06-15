# CLAUDE.md — skeleton-engine agent reference

> Version v1.6.32 | package `skeleton-engine` v8.27.0, library crate `engine` | wgpu-based Rust 2D game engine (wgpu 29, MSRV 1.92, CI pin Rust 1.95.0) | **Cargo workspace** (members `.` + `engine_reflect_derive` proc-macro)  
> WASM support: `cargo build --target wasm32-unknown-unknown` passes; an example game ships to
> the web via `cargo build --example` + `wasm-bindgen` (see `examples/games/coin_race/web/`)  
> Full API: `REFERENCE.html` | dev history / architecture decisions: `docs/HANDOFF.md`

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

- **WASM gotcha:** do *not* gate on `--target wasm32 --all-targets` — it fails on the
  native-only examples (`platformer_game`/`mp_server`/`gpu_particles`, which pull in
  `rapier2d`/`tungstenite`/`GpuParticleEmitter`). The lib+bins build above (or `--lib`)
  is the real wasm gate.
- **Why this exists:** a prior refactor shipped declaring "done" on only `fmt --check` +
  `test --lib`, which misses the wasm-build + clippy regressions that the commands above
  catch. Don't narrow the bar.
- **Optional wasm render check:** `./scripts/wasm_smoke.sh` builds the `coin_race` example
  to wasm, runs it headless on a simulated Retina (DPR=2) display, and asserts the app
  **connects + renders a non-blank frame**, saving the screenshot to eyeball for subtle
  geometry/text bugs. *Not* a CI gate (CI has no Chrome/GPU); needs Chrome + a
  `wasm-bindgen-cli` matching the `wasm-bindgen` crate. Run it after wasm-affecting changes.

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
| Engine entry point, main loop, render orchestration, `load_image` | `src/app.rs` |
| Handle<T>, ImageAsset, ScriptAsset, AssetServer (asset load / caching / hot reload) | `src/asset.rs` |
| TextureAtlas (uniform grid atlas), AtlasSprite (atlas tile render component) | `src/atlas.rs` |
| Reflect trait, ReflectValue (`F32`/`I32`/`Vec2`/`Bool`/`String`/`Color`, `#[non_exhaustive]`; runtime field read/write, egui Inspector integration); **`#[derive(Reflect)]`** proc-macro (`derive` feature, default on) | `src/reflect.rs`, `engine_reflect_derive/` |
| DataTable, DataTableRegistry (schema-agnostic RON data tables; `App::load_data_table`, hot-reloaded, edited in the editor's Data Tables panel); `App::register_editable_component::<T>` (one-call reflect+clone+serde+inspector registration) | `src/data_table.rs`, `src/app/editor.rs` |
| ScriptAsset, ScriptRunner, ScriptingSystem (Rhai scripting) | `src/scripting.rs` |
| DebugUi (egui overlay, F1 toggle, custom panels via `ctx()`) | `src/debug_ui.rs` |
| In-game editor: F1 overlay + F2 **docked mode** (dock layout, viewport input gate, engine-level editor pause, gizmo (move + 8-handle resize + rotation handle) + screen-space UI gizmo, scene save, Data Tables panel, component copy/paste + entity-list search, prefab save/spawn, world-aligned grid overlay + cursor readout, bounds/colliders debug overlay, pathfinding-grid overlay (per-`Tilemap` walkable/blocked cells), audio bus mixer panel (per-bus volume sliders), particle live-tuner (inspector `ParticleEmitter` field drags + reset), lighting editor (`PointLight` add/edit + global `AmbientLight` control), state-machine editor (list-based `AnimationStateMachine` graph: states/transitions/params + edits), timeline editor (`Timeline` per-track keyframe list + playback controls), settings persistence (RON config), **Tile Paint** = viewport tile-painting for selected `Tilemap` entities — brush(N×N)/rectangle/bucket tools + eyedropper(Alt+click), L-paint/R-erase/digit-pick, **image-swatch palette** (atlas thumbnails via egui `register_native_texture`), stroke-level undo, visual-only/native; example `tile_paint`) | `src/app/editor/` |
| Full public API re-export list | `src/lib.rs` |
| Entity / Component / Resource / Query (+ `register_persistent` survives scene reset, via `App`) | `src/ecs/world.rs`, `src/app.rs` |
| Event bus (`Events<E>`) | `src/ecs/events.rs` |
| `System` trait | `src/ecs/system.rs` |
| Scene transitions (Scene, SceneCmd, SceneChange), SystemRegistrar (labeled system registration from `on_enter`) | `src/scene.rs` |
| Transform, Sprite | `src/components.rs` |
| WindowConfig, GameState, ShouldQuit, DebugDraw (filled rects via `rect_filled_z`; `DebugShape` is `#[non_exhaustive]`) | `src/resources.rs` |
| Camera (coordinate transforms, zoom; `screen_to_world`/`world_to_screen`; `bounds` + `clamp_to_bounds` world-bounds clamp, auto-applied by App after follow) | `src/camera.rs` |
| InputState, InputMap (keyboard + gamepad bindings: `bind_gamepad_button`/`bind_gamepad_axis` + `AxisBinding`, `*_with_gamepad` resolution) | `src/input/` |
| GamepadState, GamepadButton, GamepadAxis | `src/input/gamepad.rs` |
| PhysicsWorld, PhysicsBody, PhysicsSystem (syncs body position **and rotation** → Transform), CollisionEvent, BodyHandle/ColliderHandle (opaque newtypes; `.raw()` = rapier escape hatch) | `src/physics/` |
| CharacterController (+ `request_drop`/`is_dropping` for one-way), RaycastHit, cast_ray, cast_ray_with_normal, move_character | `src/physics/character.rs`, `src/physics/world.rs` |
| add_kinematic_box, add_kinematic_circle, add_static_from_tilemap (one-shot), sync_static_from_tilemap + TileColliderIndex (incremental — diff add/remove for runtime-mutated tilemaps), **TilemapColliders** component + SolidTiles + `sync_tilemap_entity_colliders`/`App::sync_tilemap_colliders` (opt-in: keep tile colliders synced when the editor's Tile Paint or a game's `set_tile` mutates the map; example `dig_quest`), TileCollider, set_one_way/is_one_way | `src/physics/world.rs` |
| add_revolute_joint, add_distance_joint, add_prismatic_joint, remove_joint (return/take engine `JointHandle` newtype wrapping rapier; example: `crane_wrecking_ball`) | `src/physics/world/joints.rs` |
| SpatialGrid, Collider, CollisionLayer (SpatialGrid is mirrored to a World resource by CollisionGridSystem) | `src/collision/` |
| BehaviorTree, BehaviorNode, Sequence, Selector, Inverter, AlwaysSucceed, BehaviorSystem, Blackboard, BlackboardValue (`Path` variant + `set_path`/`get_path`) | `src/behavior.rs` |
| Seek, Flee, Arrive, Wander, SteeringVelocity, SteeringSystem (steering behaviors; O(1) per-entity component lookup) | `src/steering.rs` |
| PathGrid, find_path (4-dir), find_path_diagonal (8-dir A*, octile heuristic, no corner-cut), PathGrid::from_tilemap | `src/pathfinding.rs` |
| AnimationPlayer, AnimationClip, AnimationSystem, BlendWeight (crossfade = true 2-UV shader-lerp; renderer `mix`es from/to frames) | `src/animation/player.rs`, `src/animation/system.rs` |
| AnimationClipSet, AnimationClipRegistry, ClipSetError (data-driven animation: named clips loaded from RON `(atlas, clips)`, frame indices → UvRect; `App::load_animation_clips` = registry + DataTable-style hot-reload) | `src/animation/clip_set.rs` |
| UvRect, BlendUv (GPU UV-region types, consumed engine-wide) | `src/renderer/uv.rs` |
| AnimationStateMachine, StateMachineSystem, TransitionCond, AnimParam (per-transition crossfade via `add_transition_crossfade`; editor **State Machine panel** — `state_names`/`state`/`param` accessors + `set_current_state`/`set_state_clip`/`remove_state`/`remove_transition` edit ops drive a list-based graph editor in the docked inspector; example `sm_crossfade`) | `src/animation/state_machine.rs` |
| BlendTree1D, BlendEntry, BlendTreeSystem (1D parameter-driven auto transitions + crossfade) | `src/animation/blend_tree.rs`, `src/animation/blend_system.rs` |
| SkeletalAnimator, SkeletalClip, BoneTrack, BoneKeyframe, SkeletalAnimationSystem, SkeletonBuilder (2D cutout skeletal animation) | `src/skeletal.rs` (details: `docs/SKELETAL.md`) |
| UI (UiNode, Button, Label, TextInput, ScrollView, Panel, LayoutSystem, UiEvent) | `src/ui/` |
| Slider (horizontal slider), CheckBox (toggle checkbox) | `src/ui/slider.rs`, `src/ui/checkbox.rs` |
| LocalizedText (key bound to a widget), LocalizationSystem (resolves `LocaleResource::t` into Label/Button/CheckBox each frame) | `src/ui/localized.rs` |
| Tag, EntityDef (+ `components` map), SceneDef, Prefab, spawn_entity_def, spawn_scene_def, **SerdeComponentRegistry** + `App::register_serde_component::<T>` (any serde component persists to scene RON; UI widgets auto-registered) | `src/prefab.rs` |
| Timer, Tween, Easing, Lerp (general interpolation trait) | `src/timer.rs`, `src/tween.rs` |
| Timeline, Track, Keyframe, TimelineSystem (keyframe cutscenes → entity Transform/Sprite; **CameraTarget** marker + `zoom` track route a timeline into the **Camera** resource as a virtual rig; `Track::keyframes`/`len`/`remove`/`set_time`/`clear` editor accessors+edit ops drive the docked **Timeline panel** (per-track keyframe list + playback controls); example: `timeline_cutscene`) | `src/timeline.rs` |
| History (generic snapshot undo/redo for grid puzzles, turn-based, editors) | `src/history.rs` |
| ParticleEmitter, ParticleSystem, ParticleBurst (one-shot burst + `ParticleEmitter::burst()`); ParticleConfigSet/ParticleConfigRegistry/ClipSetError-style ParticleConfigError (data-driven emitter configs from RON; `App::load_particle_configs` = registry + DataTable-style hot-reload) | `src/particle/` (`mod.rs`, `config_set.rs`) |
| Tilemap (+ runtime `set_tile`/`get_tile`/`cell_at_world`/`dims`), TilemapAtlas, TilemapSystem (reactive — diffs cached grid, updates only changed cells), TilemapAutotile (neighbor-bitmask autotiling: `Neighborhood::Edge4`/`Blob8`, `edge_16`/`blob_47`, `with_oob_filled`, `compute_tile_mask`), MultiTerrainAutotile + TerrainRule + compute_tile_mask_typed (per-terrain same-value autotiling) | `src/tilemap.rs` |
| AudioManager (playback, positional audio, bus mixer (`assign_bus`/`set_bus_volume`/`bus_volume`/`bus_names` — `bus_names` lists all buses for the editor mixer panel), fades; **ducking/sidechain** via `duck_bus`/`release_bus`/`bus_duck`/`set_sidechain`/`clear_sidechain` — BusDuck/Sidechain), AudioSystem (built-in system that ticks `update(dt)` so fades + ducks progress; SFX file-bytes cache) | `src/audio.rs`, `src/audio/` |
| save / load (AEAD, player saves) / write_ron / read_ron (plaintext, design-time assets) / load_or_default / exists / delete / save_path / SaveError; **save_versioned + load_migrated + SaveMigrator** (versioned-schema save migration over ron::Value) | `src/save.rs` |
| NetworkClient (native=tungstenite thread, wasm=web-sys), NetworkSystem (polls → `Events<NetworkEvent>`), NetworkEvent, NetworkConfig (queue/size caps), RemoteEntities (`id→Entity` lifecycle), SnapshotBuffer<T: Lerp> (generic per-entity interpolation buffer); relay demo `mp_server`/`mp_client`, authoritative game `examples/games/coin_race`, client-prediction `examples/games/predict_shooter`, interpolation-only `examples/games/orbital_dodger`, AOI-streaming/interest-managed `examples/games/salvage_run` | `src/network.rs` |
| PostProcessConfig, PostProcessRenderer | `src/renderer/post_process.rs` |
| PointLight, AmbientLight, LightingRenderer (2D point-light pass, nearest-16 cull; native-only) | `src/renderer/lighting.rs` |
| RenderTarget, OffscreenCamera, create_render_target (render-to-texture; each offscreen target submits its **own** command buffer so it uses its own camera — exclude RT-display sprites via `layer_mask`; example: `security_camera`) | `src/renderer/render_target.rs`, `src/app/render.rs`, `src/components.rs` |
| DrawText, TextQueue, TextAlign (Left/Center/Right/**End**/**Auto** — Auto right-aligns RTL automatically), TextAnchor (screen-space text, top-left origin; `DrawText::centered` anchors at text center; place at a world position via `Camera::world_to_screen`); **bidi/RTL shaping is built in** (`Shaping::Advanced`); `ExtraFonts` resource (multi-script font fallback alongside `FontData`); example `rtl_text` (Hebrew + multi-font) | `src/renderer/text.rs` |
| wgpu render pipeline (rarely edited directly) | `src/renderer/` |

---

## Core patterns & task recipes

Detailed in **`docs/PATTERNS.md`**:

- **Architecture patterns** — ECS query API (`query2`/`query_opt2`), borrow-checker
  workaround (collect entities then `get_mut`), render-layer separation
  (`AnimationSystem` → `UvRect` → renderer), UI system order (`LayoutSystem` before
  `UiSystem`), animation state-machine order (`StateMachineSystem` after
  `AnimationSystem`), `PhysicsWorld` encapsulation accessors.
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

## Related projects

| Repo | Path | Role |
|--------|------|------|
| skeleton-engine | `/Users/jkl/Projects/skeleton-engine` | Engine core (this repo) |
| rust-survivors | `/Users/jkl/Projects/rust-survivors` | Game project that uses the engine |

`rust-survivors` consumes the `skeleton-engine` package under the crate name `engine`.
On breaking changes to the engine's public API, check the impact on the game side.

---

## Document map

| Document | Purpose |
|------|------|
| `CLAUDE.md` (this file) | Agent quick reference — module map, task checklists |
| `docs/PATTERNS.md` | Core architecture patterns + task recipes (extracted from this file) |
| `REFERENCE.html` | Full public API + code examples (detailed) |
| `docs/HANDOFF.md` | Per-phase dev history, background on architecture decisions |

> **Growth strategy**: when content would push this file past 200 lines, move detail into
> a `docs/*.md` (a new subsystem doc, or `docs/PATTERNS.md`) and leave only a one-line
> reference here. Never drop needed content just to stay under the limit.
