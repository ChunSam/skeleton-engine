# Changelog

All notable changes to `skeleton-engine` are documented here.

The package follows semantic versioning beginning with 1.0.0.

## 2.0.0

### Added

- `lit_dungeon_game` example (`examples/games/lit_dungeon/`): first playable-game use of 2D
  lighting (`PointLight`/`AmbientLight`) and `PostProcessConfig`. A dark top-down brazier-
  lighting puzzle with a decaying torch; bloom + vignette post-process (toggle with `P`).
- `blend_locomotion` example (`examples/blend_locomotion.rs`) + `gen_blend_sheet` asset generator:
  first use of `BlendTree1D` in a real interactive loop. A single speed parameter drives
  idle/walk/run clip blending; demonstrates the new true crossfade and the stranding fix below.
- `BlendUv { to, weight }` component (`engine::BlendUv`): written by `AnimationSystem` during a
  crossfade and read by the sprite renderer to cross-dissolve the two frames per-pixel.
- `ImeConfig { allowed: bool }` resource (`engine::ImeConfig`, default **off**): controls whether
  the window accepts IME text composition. Insert `ImeConfig { allowed: true }` before `App::run()`
  in apps that need text input. See the IME fix under Fixed.
- `crane_wrecking_ball` example (`examples/crane_wrecking_ball.rs`): first playable-example use of
  the physics **joint** API (`PhysicsWorld::add_revolute_joint` / `add_distance_joint`). A kinematic
  crane cart hangs a revolute-pinned arm with a distance-tethered wrecking ball; drive the cart to
  swing the ball and knock a block stack off its pedestal. The joint methods shipped with unit tests
  but had zero game/example coverage. Demonstrates the rotation-sync fix below.

### Breaking

- `Entity` is now an opaque generation-checked handle with `index()`, `generation()`, and
  `from_raw_parts(index, generation)`. Direct `entity.0` access is removed.
- `World::clone_entity(src)` now returns `Option<Entity>` and returns `None` for stale or
  despawned handles.
- Rhai scripting now uses `despawn_entity(index, generation)` instead of index-only
  `despawn_entity(id)`.
- Rhai scripting exposes `entity_index()` and `entity_generation()` for the current
  script runner entity.
- Removed the misleading public `Sprite.normal_texture` and `Sprite.normal_handle` fields.
  v2 keeps flat-normal lighting internally but does not expose per-sprite normal maps.

### Fixed

- Post-process shader (`post_process.wgsl`) declares the bloom tap-offset array as `var` instead
  of `let`, fixing a naga validation error ("may only be indexed by a constant") that panicked on
  shader creation whenever `PostProcessConfig.enabled` was `true`. Surfaced by the new
  `lit_dungeon_game` — the first runtime use of post-processing (CI compiles but never runs the
  windowed app).
- 2D lighting now projects `PointLight` positions with the **logical** viewport size (matching
  the sprite pass) instead of the physical surface size. On HiDPI/Retina displays (scale > 1)
  lights previously drifted from their sprites and rendered at half radius; on scale-1.0 displays
  it happened to line up, which is why it went unnoticed. Also surfaced by `lit_dungeon_game`.
- Screen-space text (`TextQueue`/`DrawText`) now renders **after** the post-process and lighting
  passes, so HUD/overlay text is no longer dimmed by world lighting (or warped by post effects).
  Trade-off: `DrawText` is no longer affected by `PostProcessConfig`; route text through egui if
  you want it post-processed. Surfaced by `lit_dungeon_game`.
- Post-processing and lighting now compose as `scene -> post -> lighting -> final` when both
  effects are active.
- Lighting intermediate targets are recreated after viewport resize, and `PointLight`
  positions now respect camera position and zoom.
- Scene replacement restores the same core engine resources as initial app creation, including
  panic recovery state, and preserves initialized `DebugUi`.
- Images loaded directly through `AssetServer::load_image` are lazily uploaded to the GPU cache,
  so scene-owned loading no longer depends on `App::load_image`.
- `BlendTreeSystem` no longer strands an entity on an intermediate clip when the blend parameter
  crosses two thresholds (e.g. idle→walk→run) within a single crossfade: it now defers the new
  transition instead of recording an unachieved target, and re-evaluates once the crossfade ends.
  Surfaced by `blend_locomotion`; regression test in `src/animation/blend_system.rs`.
- Game key input is no longer broken when a CJK IME (Korean/Japanese/Chinese) is active. The window
  previously enabled IME unconditionally, so on macOS the OS could route key-release events into IME
  composition and leave keys stuck "pressed" (e.g. a held movement key never released → the
  character kept moving). IME is now **off by default** and opt-in via the new `ImeConfig` resource;
  only text-input apps (`settings_menu_game`) enable it. Surfaced by `blend_locomotion` (a held
  accelerate key stayed latched under a Korean IME, so the clip never returned to idle).
- `PhysicsSystem` now syncs each body's **rotation** into `Transform.rotation`, not just its
  position. Previously a body that rotated under physics (e.g. a joint-driven swinging arm) kept a
  bolt-upright sprite because rotation was silently dropped. Rotation-locked bodies (`lock_rotation:
  true`) are unaffected (their angle is always 0). Surfaced by `crane_wrecking_ball`; regression
  tests in `src/physics/system.rs`. Behaviorally inert for consumers that own a raw `PhysicsWorld`
  and sync transforms themselves (e.g. `rust-survivors`).

### Changed

- `SceneDef` schema version is now `2`; old v1 files with removed normal-map sprite fields are
  accepted and those fields are ignored.
- Agent instructions now define this repository as the default and only verification scope unless
  the user explicitly asks for external project checks.
- Lighting now renders the **nearest 16** point lights to the camera when a scene exceeds the
  16-light hard cap (previously the first 16 in arbitrary query order), and warns once. Light
  occlusion/shadows and per-sprite normal maps remain out of scope; lighting stays native-only.
- Animation crossfades are now a true **2-UV shader-lerp** cross-dissolve (`mix(from, to, weight)`
  in `sprite.wgsl`) instead of a 50% hard frame-swap, and `BlendWeight` is finally consumed by the
  renderer (via the new `BlendUv` component). Additive: a sprite that is not crossfading
  (`weight = 0`) renders byte-identically to before. `InstanceRaw` gained internal `to_uv`/`blend`
  fields; the sprite path stays cross-platform (the blend works on wasm too).

## 1.3.0

### Added

- `TextInput` single-line **horizontal scrolling**: long values no longer wrap or clip out of view.
  The field renders as one non-wrapping line and scrolls so the caret stays visible while typing or
  navigating (`Home`/`End`/arrows); an unfocused field anchors to the start. New `DrawText`
  opt-in `with_single_line_caret(caret_byte)` drives it — the renderer measures the caret x via
  glyphon `Buffer::layout_runs()` and shifts the `TextArea` left, clipped to the field by
  `TextBounds` (no new render pipeline).
- `TextInput::remaining_capacity()` and `TextInput::caret_display_offset()` helpers.

### Fixed

- IME at `max_len`: composing input when the field is full no longer shows a phantom, uncommittable
  preedit. `UiSystem` only displays the IME preedit while it still fits in the remaining capacity
  (`remaining_capacity() >= preedit.len()`); commits already truncate to fit.

### Example

- `settings_menu_game` Settings scene gained a dedicated narrow long-text field (prefilled past its
  width, `max_len` 48) that exercises horizontal scroll, caret-follow, and IME-at-capacity.

## 1.2.1

### Fixed

- macOS live window-**resize** drag froze the content: while the OS runs its modal resize loop
  the normal `about_to_wait → request_redraw → RedrawRequested` cadence is parked, and the
  `Resized` handler only reconfigured the surface without drawing. The frame step (update +
  render) is now factored into `App::step_frame` and is also driven inline from `Resized`, so
  animations keep advancing while the window is being resized.

### Added

- `Window::pre_present_notify()` is now called immediately before `surface.present()`, the
  winit-recommended compositor hint that trims presentation latency.
- `settings_menu_game` gained a small always-animating spinner (bottom-left, `dt`-driven, no
  input) so a window-drag freeze is visible by eye — it stalls during a drag and resumes after.
- Debug instrumentation: `step_frame` logs `frame gap <ms>` at `debug` level when the
  inter-frame gap exceeds ~33 ms, to quantify drag/stall (e.g. `RUST_LOG=engine=debug`). The
  `settings_menu_game` example now initializes `env_logger` (native-only dev-dependency) so the
  `log` output is actually visible — previously no log backend was installed.

### Known limitations

- A one-frame lag remains at the **start** of a live drag (both resize and titlebar move): the
  window content tracks the cursor a beat late on the first movement, then follows normally for
  the rest of the drag. The hard freeze is gone — content keeps animating throughout both drag
  kinds on the tested macOS (15.x / Darwin 25) — but this residual start-of-drag latency is a
  macOS/winit present-timing artifact left as a documented limitation per the "known levers"
  scope (deeper fixes — background redraw thread / native Cocoa hooks — were out of scope).

## 1.2.0

### Added

- `LocalizedText` component plus `LocalizationSystem` — bind a translation key to a `Label`,
  `Button`, or `CheckBox` and the system keeps its text in sync with the current locale every
  frame. Switching language is now just `LocaleResource::set_locale(..)`; the whole UI
  retranslates with no manual per-widget rebuild. Re-exported from the crate root.
- `settings_menu_game` example (`examples/games/settings_menu/`) — a Title → Settings → Dialogue
  slice that is the first playable-game coverage for the UI-depth + localization + audio-bus
  surface: `TextInput`, `Slider`, `CheckBox`, `ScrollView`, `Panel`/`LayoutSystem`, rich/multiline
  `Label`, `LocaleResource` (EN/KO/ES) + `LocalizedText`, and `AudioManager` buses + `AudioEffect`
  low-pass. Cross-scene `Settings`/locale/`AudioManager` survive `SceneCmd::Replace` via
  `App::register_persistent`.

### Fixed

- Clicks landing on the wrong widget after a mouse move: `InputState` keeps only the latest
  cursor, so when a press and a following move collapsed into one frame the click was hit-tested
  at the moved-to position (e.g. pressing empty space then moving onto a button activated it,
  while pressing a button then moving off did nothing). `InputState` now records the cursor at the
  press and release moments (`mouse_press_cursor`/`mouse_release_cursor`), and `UiSystem` hit-tests
  clicks/toggles/drag-starts against those (hover and drag-tracking still use the live cursor).
- `TextInput` caret rendering: the caret `|` was always appended at the end of the string, so it
  never matched the real cursor after navigation and text appeared to be inserted "in the middle".
  Added `TextInput::display_with_caret` which inserts the caret (and IME preedit) at the byte
  cursor; `UiSystem` uses it. The caret blinks while focused but its slot is always reserved (a
  space when off, `|` when on) so blinking no longer shifts the trailing text.
- Input-to-display latency: `desired_maximum_frame_latency` lowered from 2 to 1 (vsync kept, no
  tearing) so button/drag feedback lands a frame sooner.
- IME / non-Latin input: `set_ime_allowed(true)` is now called on the window, so macOS (and other
  platforms) compose CJK input and deliver it via `Ime::Commit`. Previously IME was never enabled,
  so Korean arrived as separated jamo (per-keystroke `Character` events).
- `AudioManager::play_tone` now applies the channel's effective bus volume to the sink and the
  channel `AudioEffect` (low-pass / pitch / fade-in), matching file playback. Previously tones
  ignored both, so `set_bus_volume` and `set_effect` had no audible effect on tone channels.
- Interactive responsiveness: the event loop never set a `ControlFlow`, defaulting to `Wait`, so
  drags/hover updated a beat late and sliders did not track the cursor smoothly. It now runs with
  `ControlFlow::Poll` for a continuous per-frame loop (vsync-paced via the existing redraw request).
- `TextInput` cursor editing: added `move_left`/`move_right`/`move_home`/`move_end`/`delete_forward`
  (UTF-8 safe) on `TextInput`, and `UiSystem` now applies ←/→/Home/End/Delete to the focused field.
  Previously the caret could only sit where typing left it (no navigation, no forward delete).
- HiDPI mouse/touch hit-testing: the cursor was stored in physical pixels while UI hit-testing,
  `ViewportSize`, and `Camera::screen_to_world` all work in logical pixels, so on a scaled display
  (e.g. Retina 2×) clicks landed offset from the cursor. `CursorMoved` and the touch→mouse
  emulation now divide by the window scale factor, storing the cursor in logical coordinates
  (no-op at scale 1.0). Surfaced by `settings_menu_game`'s click-heavy widgets; also corrects
  editor gizmo dragging and any `screen_to_world` use on HiDPI.

### Known gaps (surfaced, not yet addressed)

- `LocaleData.font` is not applied at runtime: `TextRenderer` takes its font once at init via the
  `FontData` resource, so per-locale font switching is unsupported. Non-Latin scripts render only
  through native system-font fallback and are absent on wasm (no system fonts). Korean in
  `settings_menu_game` therefore renders on macOS but not on Linux CI / wasm.
- `LocaleData.direction` / `TextDirection::RightToLeft` is metadata only — the text renderer does
  not auto-apply RTL alignment from the locale (it maps `TextAlign::Right` explicitly). No RTL
  locale ships in the example, so RTL is left for a future dedicated example.
- No window fullscreen-request path exists yet, so `settings_menu_game`'s Fullscreen checkbox only
  stores a preference (its label says so); wiring real OS fullscreen is deferred.
- The built-in `TextInput` is single-line with no horizontal scrolling: text longer than the field
  width clips at the edge, and IME composition at the `max_len` cap shows an uncommittable preedit.
  Adequate for short fields (names, search); a scrolling multi-line field is future work.
- The blinking `TextInput` caret is drawn inline (a reserved `|`/space slot), so it can still shift
  the trailing text by a sub-pixel on blink. A fully stable caret needs a renderer-measured overlay
  (the text renderer drawing the caret quad at the glyph position); deferred.
- Residual input-to-display latency on macOS: even with `frame_latency=1`, a click registers a beat
  late, and the window content lags during a live OS window drag (winit enters a modal event-loop
  mode). `AutoNoVsync` only helped marginally while uncapping the frame rate, so it was not adopted.
  Treated as a macOS/winit optimization to revisit.

## 1.1.0

### Added

- `BlackboardValue::Path(Vec<IVec2>)` plus `Blackboard::set_path`/`get_path` so behavior
  trees can cache a whole A* path instead of recomputing every tick. `BlackboardValue` is
  now `#[non_exhaustive]`. Validated by `maze_escape_game`, whose enemies now cache the path
  and only re-run `find_path` when the player's goal tile changes.
- `App::register_persistent::<T>()` plus `World::take_resource_erased`/`insert_resource_erased`
  to preserve chosen resources across the `World` reset that `SceneCmd::Replace` triggers.
  `scene_flow_game` uses it to drop its `Arc<Mutex<_>>` cross-scene state workaround.
- `PhysicsWorld::add_static_from_tilemap(tilemap, ppu, collider_for)` and the `TileCollider`
  descriptor (`solid` / `solid_with` / `one_way`) to generate one static collider per solid
  tile, aligned to `TilemapSystem`'s tile coordinates. `platformer_game`'s level is now a
  single `Tilemap` that drives both rendering and collision; its seamless tileset is
  reproducible via `examples/gen_platform_tiles.rs` (the original `tiles.png` is a set of
  discrete object sprites with transparent margins, not a seamless tileset).
- One-way platforms: `PhysicsWorld::set_one_way`/`is_one_way` and
  `CharacterController::request_drop`/`is_dropping`. `move_character` now passes through
  one-way colliders when ascending or dropping and only lands on them from above.
  `platformer_game` adds a one-way platform and an S/Down drop-through key.

### Added (pre-1.1 carryover)

- 2D cutout (rigged) skeletal animation in `src/skeletal.rs`: `SkeletalAnimator`,
  `SkeletalClip`, `BoneTrack`, `BoneKeyframe`, `SkeletalAnimationSystem`, and the
  `SkeletonBuilder` authoring helper. Bones are hierarchy entities whose local
  `Transform` is keyframed; the existing `HierarchySystem` and sprite renderer draw them
  with no renderer changes. See `docs/SKELETAL.md` and `examples/skeletal_puppet.rs`.
- Re-exported `AssetId`, `SaveKey`, `save_with_key`, and `load_with_key` from the crate root so public examples match the stable API surface.
- Added `ScheduleErrorPolicy` and `SystemPanicPolicy` so apps can opt into stricter schedule-cycle and system-panic behavior while keeping the existing fallback defaults.
- Added `examples/runtime_policies.rs` to show strict runtime policy configuration without opening a long-running window.
- Added `World::mark_changed<T>()` and `World::get_mut_tracked<T>()` for explicit ECS change tracking after direct component mutation.
- Added native `AudioChannelState` plus `AudioManager::playback_state`, `is_finished`, and `is_playing` so games can advance non-looping playlists when a channel naturally drains.
- Added `docs/ENTITY_GENERATION_V2_PLAN.md` to lock the v2 design for generation-checked entity handles.

### Changed

- `HierarchySystem` now propagates `GlobalTransform` in topological (root→child) order in a
  single pass, supporting arbitrary hierarchy depth. It previously ran a fixed 2-pass loop
  capped at depth 3 — a limit surfaced by deep skeletal bone chains.
- Aligned save encryption and async asset examples in the public reference with the current source.
- Native `AssetServer` cache keys now canonicalize existing file paths, reducing duplicate handles and hot-reload misses caused by mixed relative/absolute paths. Missing paths and WASM URLs keep their existing string behavior.
- Sprite renderer file texture cache lookups now accept both the original requested path and the canonical `AssetServer` handle path, so `Sprite::textured_with_handle(...)`, `DrawImage::textured_with_handle(...)`, and atlas textures no longer fall back to white when images are loaded through relative paths.
- Native audio decoding now enables MP3 in addition to WAV and Vorbis/OGG.
- `PhysicsSystem` now documents the physics-unit to pixel-unit boundary and defensively clamps invalid `pixels_per_unit` values in release builds while asserting in debug builds.
- Clarified that Rhai scripting is intended for trusted local game code, not hostile sandboxing, and documented the limits of temporary script spawn IDs.
- **Breaking rendering behavior fix:** fixed the default sprite quad UV orientation so `Sprite`, `DrawImage`, `AtlasSprite`,
  `UvRect::FULL`, `UvRect::from_grid(...)`, and `UvRect::from_pixels(...)` render
  normal top-left-origin PNGs upright without requiring `UvRect::flipped_y()`.
  Existing game-side `.flipped_y()` orientation workarounds should be removed after
  updating the engine.

### Fixed

- Restored the `wasm32-unknown-unknown` build: the WebSocket `wasm_impl` module called
  `push_event_bounded` unqualified without importing it, breaking the wasm target while the
  native build was unaffected. The function is now imported into the module scope.
- Removed the redundant manual `unsafe impl Send/Sync for BehaviorTree`. The
  `BehaviorNode: Send + Sync` trait bound already guarantees both, so the hand-written impl
  was unnecessary and would have silently masked unsoundness if that bound were ever relaxed.

## [1.0.0] - 2026-05-27

### Added

- Stable `skeleton-engine` package metadata with library crate name `engine`.
- Rust 1.88 minimum supported Rust version declaration.
- README, MIT license, changelog, and beginner `examples/basic.rs`.
- CI gates for formatting, clippy, full native tests, release build, WASM build, rustdoc warnings, `cargo package`, and `cargo publish --dry-run`.

### Changed

- Documented release package hygiene with an explicit crates.io include list.
- Updated public documentation examples for current `OffscreenCamera`, `Sprite`, `TouchState`, and `glam::Vec2` usage.
