# Changelog

All notable changes to `skeleton-engine` are documented here.

The package follows semantic versioning beginning with 1.0.0.

## 8.7.0

Multi-terrain autotiling. Additive — single-terrain `TilemapAutotile` is unchanged.

### Added

- **`MultiTerrainAutotile`** — a tilemap component (attach instead of `TilemapAutotile`)
  where each non-zero cell autotiles using the [`TerrainRule`] whose `terrain` equals the
  cell's value, connecting only to **same-value** neighbors. So distinct terrains
  (grass/water/sand) each border-tile independently. `edge_16(&[(terrain, base_id), …])`
  builds one identity edge-16 rule per terrain; `with_oob_filled`. Takes precedence over
  `TilemapAutotile`; reuses the reactive `TilemapSystem`'s 8-neighbor UV propagation.
- **`compute_tile_mask_typed(tiles, row, col, nb, oob_filled, terrain)`** — `compute_tile_mask`
  with same-terrain connectivity (a neighbor counts only when its value equals `terrain`).
- **Example `multi_terrain_game`** (+ `gen_multiterrain_sheet`) — grass/water/sand map; paint
  cells with `1`/`2`/`3` (`set_tile`) and watch every terrain re-border live.

## 8.5.0

Diagonal (8-direction) pathfinding. Additive — `find_path` is unchanged.

### Added

- **`find_path_diagonal(grid, start, goal)`** — A* on an 8-connected grid (cardinal
  cost 10, diagonal 14, admissible octile heuristic `10·(dx+dy) − 6·min(dx,dy)`). **No
  corner cutting**: a diagonal step is allowed only when both orthogonally-adjacent cells
  are walkable, so paths never slip through the gap between two wall corners. Same endpoint
  convention as `find_path` (excludes start, includes goal; `start==goal` → single cell).
- **Example `diagonal_pathing`** — a grid with a staircase wall barrier; `T` toggles 4-dir
  vs 8-dir and recomputes, so the cardinal zig-zag vs the diagonal shortcut is visible.

## 8.4.0

Audio bus **ducking + sidechain** mixing (native-only audio module). Additive.

### Added

- **Bus ducking** — `AudioManager::duck_bus(bus, gain, attack_secs)` / `release_bus(bus,
  release_secs)` / `bus_duck(bus) -> f32`. A duck is a per-bus gain multiplier (1.0 = none)
  with an attack/release envelope that rides on top of the bus volume, so it never clobbers
  `set_bus_volume`. Driven by `AudioManager::update(dt)`.
- **Sidechain** — `set_sidechain(trigger_bus, ducked_bus, gain, attack_secs, release_secs)` /
  `clear_sidechain(ducked_bus)`. Automatically ducks `ducked_bus` while any channel on
  `trigger_bus` is playing, then releases — the classic "music ducks under dialogue".
  `BusDuck` / `Sidechain` state types re-exported.
- **Example `audio_ducking`** — synthesized music + voice tones; Space plays a voice blip
  that sidechain-ducks the music bus; live on-screen `bus_duck("music")` readout (color-coded)
  makes the duck visually verifiable.

## 8.3.0

Two ergonomic helpers surfaced by the `dig_quest` example (tilemap arc). Additive.

### Added

- **`World::with_resource_mut::<R, _>(|r, world| …)`** — temporarily removes resource `R`,
  runs the closure with `&mut R` **and** `&mut World` at once (the common "I need this
  resource and the rest of the world" borrow), then re-inserts `R`; returns `false` if `R`
  is absent. Replaces the manual `remove_resource` / `insert_resource` dance.
- **`CharacterController::top_down()`** — a constructor for top-down games: like `new()` but
  with snap-to-ground and autostep disabled (the `new()` defaults are platformer-tuned and
  make a top-down character stick to wall surfaces). `slide` stays on.

### Changed

- `dig_quest` refactored onto both helpers (its two `remove_resource::<PhysicsWorld>()`
  sites and the player controller) — the validation that the new APIs read cleanly.

## 8.2.0

Runtime tilemap mutation + neighbor-bitmask autotiling, validated by the new `dig_quest`
example (a destructible-terrain top-down miner). All additive — no breaking changes.

### Added

- **`Tilemap` runtime mutation** — `set_tile` / `get_tile` / `dims` / `cell_center_world`
  / `cell_at_world`. `TilemapSystem` is now **reactive**: it diffs a per-entity cached grid
  and spawns / despawns / updates only the changed cells' tile sprites (a tilemap that never
  mutates renders exactly as before).
- **Autotiling** — the `TilemapAutotile` component (attach to the tilemap entity) selects
  each tile's display UV from its filled neighbors. `Neighborhood::Edge4` (16-tile) and
  `Blob8` (canonical 47-blob); `TilemapAutotile::edge_16` / `blob_47` rulesets, the
  `with_oob_filled(bool)` builder, and the pure `compute_tile_mask`. A changed cell also
  refreshes its 8 neighbors' UVs, so dug holes keep continuous outlines.
- **Incremental tile colliders** — `TileColliderIndex` +
  `PhysicsWorld::sync_static_from_tilemap` diff against the index and add / remove only the
  changed cells (reusing `remove_body`). Use it for the **initial** build too (empty index =
  full build); do not mix with `add_static_from_tilemap` on the same tiles (that would
  double-add colliders so a dug cell never frees). `add_static_from_tilemap` is unchanged for
  static maps.
- **Example `dig_quest_game`** (`examples/games/dig_quest/`) + the `gen_autotile_sheet`
  deterministic asset generator. Native playtest confirmed: digging updates the autotile
  outline + frees collision (the player enters), reset restores, post-reset re-dig works.

## 8.1.10

Deferred-item cleanup from the engine-wide review (asset hot-reload + scripting scope).
No public API change.

### Fixed

- **Atlas file changes are recognized by hot-reload.** `poll_reloads` checked image /
  script / data-table path maps but not `atlas_path_to_id`, so an atlas path was never
  treated as "known" (the underlying image pixels still reloaded via the inner
  `load_image`; this makes the path recognition self-consistent).
- **A failed image load no longer registers a dead file-watcher.** `load_image` watched
  the path even on a failed load; `notify` cannot watch a non-existent path, so a later
  file-create never fired. The watch is now registered only for successfully-loaded paths.
- **Rhai `ScriptRunner` scope no longer grows across frames.** The persistent per-entity
  `Scope` is rewound to its 5-var transform baseline (`x`/`y`/`rot`/`sx`/`sy`) after each
  `on_update`, so `let` bindings introduced per frame don't accumulate. The script scope
  is a transform transport, not a store for cross-frame custom state.

(Not changed: `with_ctx`/`with_ctx_mut` keep their `expect` — calling a Rhai API function
outside `ScriptingSystem::run` is a documented contract violation; a graceful path would
require a sprawling `R: Default` refactor across all script API functions.)

## 8.1.9

Bug fixes: surface-error handling. Final batch of a second-pass engine-wide review (app
main-loop / window / render orchestration + concurrency / WASM / panic-safety — the latter
entirely clean). No public API change.

### Fixed

- **A minimized/occluded window no longer spams `log::error!` every frame.** The surface
  acquisition's `Occluded` and `Timeout` results fell through to an `error!` log, firing
  once per frame while minimized. They are now skipped silently (`Lost`/`Outdated` still
  reconfigure; genuine errors like `Validation` still log).
- **A `Suboptimal` surface is now reconfigured.** After a DPI/monitor/rotation change the
  acquired `SurfaceTexture` can be flagged suboptimal; the frame is presented and the
  surface is then reconfigured so subsequent frames are optimal (was previously ignored,
  causing persistent degradation on some platforms).

## 8.1.8

Bug fixes: UI click/slider/scroll edge cases, save-path hardening, timeline loop wrap.
Final batch of an engine-wide review sweep (UI / asset-save-scripting / timeline-tween-
network — otherwise clean). No public API change.

### Fixed

- **Overlapping `Button`s no longer both fire on one click.** The button pass fired
  `ButtonClicked` for every button whose hit-test passed, so stacked buttons all fired.
  Only the top-most (highest `z`) clicked button now fires. (Cross-widget pointer
  consumption — a button beneath a different widget type — is left as a future TODO.)
- **`ScrollView` with `item_height == 0` no longer panics.** `size.y / 0.0 → inf`,
  `inf.ceil() as usize → usize::MAX`, `+ 1` overflowed (debug panic). Zero/negative item
  height is now guarded.
- **`Slider` emits exactly one `SliderChanged` on the press frame.** The press and the
  same-frame drag-recalculation both fired, producing two events with different values;
  the drag path is now skipped on the press frame.
- **`save_path` rejects path traversal.** `app_name`/`file` are sanitized (only `Normal`
  path components kept), so `"../../etc/passwd"` can no longer escape the data directory;
  legitimate sub-directories (e.g. `"saves/slot1.sav"`) are preserved.
- **Looping `Timeline` wraps with modulo.** A `dt` larger than the timeline `duration`
  (e.g. resuming after a stall) used a single subtract, leaving `time` past the end for
  several frames (stutter). It now wraps with `%` in one frame (guarded against
  `duration == 0`).

## 8.1.7

Bug fixes: audio bus-volume during fades, behavior-tree `AlwaysSucceed`, tilemap tile-id
bounds. Found by an engine-wide review sweep. No public API change.

### Fixed

- **Bus volume is no longer applied twice during audio fades.** A fade stored its start
  volume as `base × bus`, and `update()` multiplied by the bus volume again, so the sink
  got `base × bus²` — an audible volume pop at fade start and a fade at the wrong rate
  (only when a bus had volume ≠ 1.0). Fades now store/interpolate the pre-bus base volume
  and the bus factor is applied exactly once in `update()`.
- **`AlwaysSucceed` behavior-tree decorator passes `Running` through.** It discarded the
  child's status and always returned `Success`, so wrapping a multi-frame action made the
  parent `Sequence`/`Selector` advance on frame 1 and abandon the still-running child. It
  now returns `Running` while the child runs and only converts `Failure → Success`.
- **`TilemapAtlas::uv_for` clamps out-of-range tile ids.** A tile id ≥ `columns × rows`
  produced a UV rect outside `[0,1]`, sampling garbage/wrong tiles. Out-of-range ids now
  return `UvRect::FULL` instead.

## 8.1.6

Bug fixes: physics collision-event delivery + raycast freshness, animation clip-finish +
blend-tree state. Found by an engine-wide review sweep (physics/collision + animation/
skeletal — both otherwise clean). No public API change.

### Fixed

- **`CollisionEvent::Stopped` is delivered when a contacting entity despawns.** The
  handle→entity map was rebuilt each frame from live entities, so an entity removed while
  still touching another resolved to nothing and its `Stopped` exit event was silently
  dropped (listeners waiting for "no longer touching" never fired). The system now keeps
  the previous frame's map and falls back to it when resolving stopped pairs.
- **`cast_ray` no longer hits a just-removed body in the same frame.** The query pipeline
  was only refreshed inside `step()`, so a raycast issued after `remove_body` but before
  the next step saw a phantom collider. `remove_body` now refreshes the query pipeline
  immediately. (`cast_ray`/`cast_ray_with_normal` remain `&self` — no API change.)
- **A 1-frame non-looping clip is no longer reported finished before it is shown.**
  `is_finished()` returned `current_frame >= len-1`, which is `0 >= 0` (true) at entry for
  a 1-frame clip, so an `AnimationEnd` state-machine state transitioned away the same frame
  it was entered. The player now tracks a `finished` flag set when the advance actually
  reaches past the last frame.
- **BlendTree1D no longer gets stuck after a parameter reversal during a crossfade.** If
  `param` returned to the FROM clip's range mid-crossfade, `last_clip` was poisoned and the
  dedup skipped all later transitions, leaving the player stuck on the crossfade target.
  The "already on target" branch is now guarded by `!is_crossfading()`.

## 8.1.5

Bug fixes: scene-stack panic recovery + centered-text wrapping. Found by an engine-wide
review sweep (core ECS + rendering — both otherwise clean). No public API change.

### Fixed

- **`SceneCmd::Pop` no longer permanently silences the builtin tail system.** If a
  `Push`ed scene's first system panicked (added to the panic set) and the scene was then
  `Pop`ped, the retained panic index aliased `HierarchySystem`'s post-drain index and
  skipped it forever — parent-child `GlobalTransform` propagation silently stopped. The
  retain bound is now `new_scene_len` (drops drained + tail indices; the tail gets a
  clean retry, consistent with `reload_scene`).
- **`DrawText::centered` no longer wraps at half the viewport width.** With no explicit
  bounds, the layout buffer width was `viewport_w - position.x`; for a `Center`-anchored
  text positioned at the screen center that is only half the width, so a one-line title
  wrapped to two. `Center` anchor with no bounds now uses the full viewport width/height
  (top-left and explicit-bounds paths unchanged). Width/height selection factored into
  tested pure helpers.

## 8.1.4

Bug fixes: docked-editor gizmo + Inspector edge cases (follow-up to 8.1.3, found by a
second review sweep). No public API change.

### Fixed

- **Resizing a non-`TopLeft`-anchored `UiNode` no longer slides the widget.**
  `UiNode::screen_pos` is `anchor_base(anchor, size) + offset`, and for `Center`/
  `Bottom*`/`*Right` anchors the base depends on `size`. The gizmo resize math only kept
  the fixed corner stable for `TopLeft`, so resizing a `Center`-anchored widget (e.g. the
  `ui_layout_editor_game` menu buttons) drifted on screen. `ui_resize_new_layout` now
  applies an anchor-base compensation so the fixed corner stays put for every anchor
  (`TopLeft` behaviour is unchanged — its base is constant). A shared `anchor_base` helper
  is now the single source for both `screen_pos` and the gizmo.
- **Inspector field edits are no longer dropped when the archetype/selection changes
  mid-frame.** The write-back paired staged values to components by positional index, so
  adding/removing a component or an Undo/Redo that reselected a different entity in the
  same frame mis-paired them (silent edit loss). Write-back is now matched by component
  name and guarded to the entity the values were captured for.
- **Docked viewport mouse-release no longer double-fires.** On release inside the viewport
  the input release ran twice; a release with no matching press could also be produced
  when the pointer was outside. The stuck-state-clearing release now runs only when the
  primary (in-viewport) release path did not.
- **Undo/Duplicate/Paste of a child entity preserves its parent link.** `entity_to_def`
  hard-coded `parent: None`, so restoring a deleted (or duplicating/pasting a) child
  re-spawned it as a root, losing the hierarchy. It now resolves the entity's `Parent`
  to the parent's `Tag`, matching scene-save.

## 8.1.3

Bug fixes: docked-editor reliability (Undo/Redo, Load/Save, Data Tables). No API change
to the public surface; `DataTableRegistry::reload_path` now returns a `ReloadOutcome`
(was `()`).

### Fixed

- **Undo of Delete restores the whole entity.** `EditorCmd::DeleteEntity` captured only
  tag/transform/sprite, so undoing a delete dropped every other component — including
  game components registered via `register_editable_component` (e.g. `Stats`). It now
  captures the full `EntityDef` and restores via `spawn_entity_def`, preserving all
  serde-registered components.
- **Duplicate and Paste are now undoable.** The `⎘ Duplicate` button and Ctrl+V paste
  spawned entities without recording an undo step, so Ctrl+Z did nothing. They now push a
  `CreateEntity` command carrying the entity's `EntityDef`, so Undo removes the copy and
  Redo restores it with all its components.
- **Load Scene fully clears the previous scene.** `do_load_scene` despawned only
  `Transform`-bearing entities, leaving `UiNode`-only entities (menus/HUD) behind and
  duplicating them on load. It now despawns all entities before loading.
- **Data Tables "Reload" reports accurately.** `reload_path` skips reloading a table with
  unsaved edits (dirty-guard); the panel previously still showed "reloaded". It now
  reports the real outcome ("skipped reload — unsaved edits") via the new
  `ReloadOutcome` return value.
- **Save Scene no longer silently drops untagged-parent links.** When a child's parent
  entity has no `Tag` (unrepresentable in `EntityDef.parent`, which is tag-based), the
  link was dropped silently; Save now logs a warning and notes the count of dropped
  parent links in the save-status message.

## 8.1.2

Bug fix: the game-data editor (v8.1.0) is now functional under its documented usage.
No API change.

### Fixed

- **Game-side component registrations and data tables survive `set_scene`.**
  `App::set_scene` resets the `World` (via `SceneCmd::Replace` → `reload_scene`), which
  previously discarded everything registered *before* the first scene was set. With the
  documented pattern —
  ```rust
  app.register_editable_component::<Stats>("Stats", None);
  app.load_data_table("enemies", "enemies.ron");
  app.set_scene(Box::new(GameScene::new()));
  ```
  — the `Stats` reflect/clone/serde registrations and the loaded `DataTableRegistry`
  were silently lost, so `Stats` never appeared in the Inspector, was omitted from saved
  scene RON, and the Data Tables panel was empty. `App` now records these registrations
  and **replays them on every world reset** (mirroring the existing `event_initializers`
  mechanism), and `load_data_table` marks the `DataTableRegistry` persistent. The
  `stat_editor_game` example now works end-to-end (Inspector edit → Save → reload; live
  Data Tables). Built-in components (Transform/Sprite/Tag/UI widgets) were unaffected
  because they are re-registered by `insert_core_resources` each reset.
  - Internal: `SerdeComponentEntry.post_spawn` is now stored as `Arc` (was `Box`) so the
    registration can be replayed. No public signature change.

## 8.1.1

Event-loop responsiveness on macOS (no API change).

### Changed

- The native event loop now uses **`ControlFlow::WaitUntil` frame pacing** instead of
  a `ControlFlow::Poll` busy-spin: it sleeps between frames (requesting a redraw at the
  monitor refresh cadence, clamped to 60–240 Hz) so the macOS main run loop gets idle
  time — smoother window drag/resize and lower idle CPU/battery — while still rendering
  continuously (input events wake the loop immediately). This resolves the macOS
  event-loop-stall TODO previously noted in the surface config. wasm is unchanged
  (`Poll` maps to `requestAnimationFrame`).
- **`desired_maximum_frame_latency` 1 → 2**: lets the GPU keep ~1 frame queued so
  `get_current_texture()` no longer blocks the main thread on vsync for most of each
  frame.

> Note: the dominant factor in editor/game click responsiveness is **build profile** —
> run interactive testing with `--release`; debug builds spend far more per-frame CPU
> and feel laggy regardless of event-loop pacing.

## 8.1.0

Game-data editor: edit component stats and RON data tables in the docked editor
and persist them to disk. Third release of the in-engine editor arc (scene layout
shipped in 8.0.0). Fully additive — no migration needed.

### Added

- **`#[derive(Reflect)]`**: a proc-macro (new workspace crate
  `engine_reflect_derive`) that generates the `Reflect` impl for a struct of
  `f32`/`i32`/`Vec2`/`bool`/`String`/`Color`/`[f32; 4]` fields. `#[reflect(skip)]`
  omits a field; unsupported types fail with a clear compile error. Hand-written
  `Reflect` impls keep working. Add the crate to your `Cargo.toml` (the same way
  you add `engine`) and write `use engine_reflect_derive::Reflect;` then
  `#[derive(Reflect)]`. The macro is a separate crate rather than re-exported from
  `engine` so that `skeleton-engine` stays publishable without first publishing the
  proc-macro to crates.io.
- **`App::register_editable_component::<T>(name, post_spawn)`**: one call wires a
  component for full editor integration — Inspector field editing (Reflect), entity
  duplication (Clone), scene save/load (serde), and the Add/Remove Component
  buttons. `T: Reflect + Serialize + Deserialize + Clone + Default`.
- **Data tables** (`DataTable`, `DataTableRegistry`, `App::load_data_table`): load a
  schema-agnostic RON table (a sequence of `(col: value, …)` rows), read it as a
  World resource at runtime, edit it in the editor's new **Data Tables** tab (bottom
  panel) — per-cell number/string/bool editors, add/delete row, Save — and
  hot-reload disk changes into the running game (a dirty-guard protects unsaved
  edits). Native-only panel; the types are cross-platform.
- **`stat_editor` example** (`cargo run --example stat_editor_game`): entities with
  a derived `Stats` component seeded from an `enemies` data table; edit stats in the
  Inspector (live HUD updates) and tune `enemies`/`items` tables in the Data Tables
  panel — the game-data-editing acceptance test.

### Changed

- The crate is now a **Cargo workspace** (members `.` and `engine_reflect_derive`).
  Consumers that depend on `skeleton-engine` by path or git are unaffected (the
  package name and layout are unchanged); the proc-macro crate is host-compiled and
  does not affect the wasm target.

## 8.0.0

Scene layout editing: the docked editor (v7.1.0) can now select, move, and resize
UI widgets in the viewport and **persist them to a scene file**. Second release of
the in-engine editor arc (next: a game-data / stat-table editor). Breaking because
the scene file format and `EntityDef` shape changed; migration is mechanical.

### Added

- **serde + Reflect on every UI widget** (`UiNode`, `Button`, `Label`, `TextInput`,
  `Slider`, `CheckBox`, `ScrollView`, `Panel`, `LocalizedText`, plus `Anchor`,
  `TextAlign`, `LayoutDir`): widgets now serialize into scene RON and appear/edit
  in the F1/F2 editor Inspector. Runtime state (`ButtonState`, slider/text-input
  cursor & value, scroll offset, `Panel.children`) is `#[serde(skip)]`.
- **Component serialization registry**: `App::register_serde_component::<T>(name,
  post_spawn)` registers any `Serialize + DeserializeOwned + Clone` component so it
  is saved into / loaded from scene files. All UI widgets are auto-registered;
  games register their own types (e.g. stats) the same way. Backed by the
  `SerdeComponentRegistry` resource. Unregistered component names in a loaded file
  warn and are skipped (load never fails).
- **Screen-space UI gizmo**: select a `UiNode` widget to drag it (offset) and
  resize it via 8 handles in the docked/overlay viewport; world sprites gained
  8-handle scale resize (center-fixed). New undo entries
  `EditorCmd::{MoveUiNode, ResizeUiNode, ResizeEntity}` (Ctrl+Z).
- **`ui_layout_editor` example** (`cargo run --example ui_layout_editor_game`):
  load-or-default menu; arrange/resize widgets in the editor, click Save Scene,
  restart, and the edited layout loads — the scene-layout-editing acceptance test.

### Breaking

- **`SceneDef` version 2 → 3.** v2 files still load (the new `components` field
  defaults to empty; the existing version-mismatch warning is informational). v3
  files cannot be read by v7 engines.
- **`EntityDef` gains `components: HashMap<String, ron::Value>`.** Code that
  constructs `EntityDef { .. }` with explicit fields must add
  `components: Default::default()` (or use `..Default::default()`).
- **`TextInput` gains `initial_text: String`; `text` and other runtime fields are
  now `#[serde(skip)]`.** Set `initial_text` for design-time content; the registry
  post-spawn hook copies it into `text` on load. **`Slider` gains
  `initial_value: f32`; `value` is `#[serde(skip)]`** (same pattern). Constructors
  (`Slider::new`) are unchanged at runtime.
- Components are stored in scene RON as a string-encoded `ron::Value` (ron 0.8's
  `Value` cannot round-trip enums like `Anchor`); this is an internal format detail
  but visible in saved files.

## 7.1.0

The docked editor shell: a second editor mode that lays the screen out like a
commercial engine — side panels around a central game viewport — so editing no
longer covers the game. First release of the in-engine editor arc (next: UI
widget editing, then data tables). No breaking changes.

### Added

- **Docked editor mode (`F2`, native only)**: egui owns the window; the left
  panel holds Entities/Scene tabs, the right panel the Inspector, the bottom
  panel Assets, and a top toolbar carries play/pause (`▶`/`⏸`), single-frame
  step (`⏭`), snap controls, and scene save/load. The game renders into an
  editor-owned offscreen texture shown in the central panel (size follows the
  panel, 3-frame resize debounce). `F1` keeps the existing floating-window
  overlay unchanged; the modes are mutually exclusive.
- **Viewport-local input routing**: while docked, the game receives the cursor
  translated into viewport coordinates (`viewport_to_game`), and pointer events
  pass through a layer-aware gate (`docked_game_pointer_allowed`) — clicks
  inside the viewport reach the game/gizmo, clicks on panels and popups stay in
  egui, and typing in the Inspector never leaks into game input. The selection
  gizmo (drag to move, snap, undo) works inside the docked viewport.
- **Editor pause**: the toolbar pause skips scene systems at the engine level
  while keeping the builtin tail (`HierarchySystem`) running, so dragging a
  parent while paused still moves its children. `⏭` advances exactly one full
  frame. The `GameState` resource is untouched (it remains a game-side
  convention).
- **`ViewportSize` delegation**: while docked, `ViewportSize` reports the
  central panel's logical size, so cameras, screen-space UI, and
  `Camera::screen_to_world` work unchanged against the viewport; the real
  window size is restored on exit.

## 7.0.0

The renderer-dependency major window: the whole wgpu/glyphon/egui stack moves to
current majors, resolving `RUSTSEC-2026-0002` (glyphon 0.6 pinned `lru` < 0.16.3 —
previously archived as accepted risk in `docs/SECURITY_HARDENING_2026_05.md`, now
closed). Engine-side rendering behavior is preserved exactly (sRGB-first surface
format, AutoVsync, frame latency 1, WebGL2 limits on wasm, egui dithering off);
verified by the full gate suite, `wasm_smoke` (connect + non-blank render, HUD
correct), and windowed playtests (lighting pass, SM crossfade mid-blend, F1
inspector overlay).

### Breaking — toolchain & dependencies

- **MSRV 1.88 → 1.92** (`rust-version = "1.92"`): egui 0.34 requires Rust 1.92,
  cosmic-text 0.18 requires 1.89. CI pins Rust 1.95.0 (current stable, also used
  for local gates).
- **wgpu 22 → 29** (`webgl` feature unchanged), **glyphon 0.6 → 0.11**
  (cosmic-text 0.18), **egui / egui-wgpu / egui-winit 0.29 → 0.34**, winit minimum
  `0.30.13`. Transitive `lru` resolves to 0.16.4, closing `RUSTSEC-2026-0002`.

### Breaking — API changes

- **`GpuContext::clear()`** returns `Result<(), String>` (was
  `Result<(), wgpu::SurfaceError>` — wgpu 29 removed `SurfaceError`; surface
  acquisition reports through the `wgpu::CurrentSurfaceTexture` enum). *Migration:*
  treat the `Err` as an opaque message. The engine main loop handles
  reconfigure-on-`Lost`/`Outdated` internally, exactly as before.
- **`RenderTarget` pub fields and `DebugUi::ctx()` now expose wgpu 29 / egui 0.34
  types** — code touching `RenderTarget.{texture,view,sampler,bind_group}` or
  writing custom egui panels compiles against the new majors. Notable for panel
  code: `Rounding` → `CornerRadius`, `Context::style()` → `global_style()`. egui
  0.34's skrifa font backend renders text slightly differently (default text size
  12.5 → 13.0) — debug-UI only, game rendering unaffected. wgpu resources are now
  `Clone` (internally refcounted); `RenderTarget.bind_group` stays `Arc`-wrapped
  for API stability.

### Fixed

- **egui texture deltas are no longer dropped on skipped frames** — when surface
  acquisition failed for one frame (`Lost`/`Outdated`/`Timeout`, e.g. during a
  live window resize), the unconsumed `textures_delta` was overwritten by the next
  frame. egui 0.29's ab_glyph backend re-sent the full font atlas on every change,
  silently self-healing; egui 0.34's incremental skrifa updates made the latent
  bug fatal (panic on F1: "Tried to update a texture that has not been allocated
  yet"). Deltas now merge old → new (`merge_textures_delta` in
  `src/app/schedule.rs`, +2 regression tests). Found by the windowed playtest.

### Changed

- `src/app/egui_pass.rs` dropped its `unsafe` transmute — wgpu 29's
  `RenderPass::forget_lifetime()` is the supported replacement for the
  egui-wgpu `RenderPass<'static>` requirement.
- egui renderer keeps dithering **off**, matching the pre-0.34 explicit arguments
  (`RendererOptions::default()` would have silently enabled it).

## 6.0.0

The v6 breaking window: the three "Verified-but-deferred" items recorded in 5.1.3,
the v5.0.0 `Arc<str>` conversion completed for particles, and the HierarchySystem
pipeline integration. Every change below lists its migration. The fifth scoped item
(BehaviorSystem take/add archetype migrations) was investigated and **deliberately
kept** — the evaluation is recorded as a PERF comment in `BehaviorSystem::run`.

### Breaking — API changes

- **Animation systems own a scratch buffer** — `AnimationSystem`, `BlendTreeSystem`,
  and `StateMachineSystem` are no longer unit structs (they keep a reused per-frame
  entity buffer, eliminating three per-frame `Vec` allocations). *Migration:*
  construct with `::new()` (or `::default()`):
  `app.add_system(AnimationSystem)` → `app.add_system(AnimationSystem::new())`,
  `Box::new(BlendTreeSystem)` → `BlendTreeSystem::new()`, same for
  `StateMachineSystem`. `LABEL` constants and ordering semantics are unchanged.
- **Allocation-free state-machine parameter setters** —
  `AnimationStateMachine::{set_bool, set_float, add_trigger}` now take
  `impl Into<String> + AsRef<str>` and only allocate on first insert (updates are
  in-place). *Migration:* none for `&str` / `String` / `&String` / `Cow<str>`
  callers — these satisfy both bounds and compile unchanged. Only an exotic type
  implementing `Into<String>` but not `AsRef<str>` needs adapting.
- **`ParticleEmitter.texture` is `Option<Arc<str>>`** — completes the v5.0.0
  `Sprite.texture` conversion (analysis #9); per-spawn clones become refcount bumps.
  *Migration:* `texture: None` and `texture: Some("x.png".into())` compile
  unchanged; `texture: Some(string_var)` becomes `Some(string_var.into())`
  (std provides `From<String> for Arc<str>`). `ParticleEmitter` has no serde derive,
  so no save-format impact.
- **`HierarchySystem` joined the labeled pipeline** — it is registered automatically
  by `App::new()` as a permanent tail built-in (survives `SceneCmd::Replace`) instead
  of being force-run outside the scheduler. *Migration:* none for games that do not
  order around hierarchy propagation — default frame behavior (GlobalTransform
  updated after all user systems, before render) is identical. New capability:
  `.after(HierarchySystem::LABEL)` / `.before(...)` constraints now actually take
  effect (the LABEL previously existed but was a dead symbol). `docs/PATTERNS.md`
  gained the ordering row.

### Changed

- Examples updated to the `::new()` system constructors (sm_crossfade,
  blend_locomotion, platformer).

## 5.1.3

Cleanup batch over the low/leftover findings from the 2026-06-12 full-source review
(report §3 — 16 items locally re-verified first: 9 applied here, 2 refuted, 4 deferred
as breaking-or-architectural, 1 skipped as not worth the churn). Pure internal
refactors and perf fixes — zero public-API change, no migration.

### Performance

- **Particle emitter texture clone removed from the per-frame path** — the
  `Option<String>` texture is now looked up lazily only when particles actually spawn
  (was cloned per emitter per frame regardless of emission).
- **`World::despawn` change-tracking is O(1) per entity** — `added_this_tick` /
  `changed_this_tick` restructured from `HashSet<(Entity, TypeId)>` (full-set `retain`
  per despawn) to `HashMap<Entity, HashSet<TypeId>>`. Mass-despawn sites (tilemap
  teardown, particle bursts, pool clears) no longer scan the whole tracking set per
  entity. Query semantics unchanged.
- **Scripting blackboard snapshot allocates one String per entry instead of two**
  (`bb_snap` now stores keyless `BlackboardValue`s; the write-path `BbEntry` keeps its
  key, which the apply loop needs).

### Internal cleanup

- Sprite/AtlasSprite culling block (4 copies) extracted into one helper; UI widget
  passes' UiNode layout extraction (4 copies) extracted into `node_layout`; the
  `SCRIPT_CTX` access boilerplate (15 copies) extracted into `with_ctx`/`with_ctx_mut`;
  audio fade-start-volume logic (3 copies) unified into one `fade_start_vol` method.
- Editor entity labels standardized to `"Entity {index}:{generation}"` (the entity-list
  panel used a different short form than every other panel).
- Doc notes: hot reloading documented as native-only (silent empty result on `wasm32`),
  matching the lighting platform-note precedent; the physics sensor-pair `ordered_pair`
  normalization now carries a comment recording that it is defensive (verified against
  rapier's stable edge-slot order) so future reviews don't re-investigate.

### Verified-but-deferred (recorded, not fixed)

- Per-frame `Vec<Entity>` scratch in the three animation systems — requires turning pub
  unit structs into field structs (breaking); deferred to the next major.
- `AnimationStateMachine::set_bool`/`set_float` key allocation — needs a signature-bound
  change (breaking risk); deferred.
- `BehaviorSystem` take/add archetype migrations — structural (`tick` needs `&mut World`);
  acceptable at typical AI entity counts, revisit only if profiling demands.

## 5.1.2

Bug-fix batch from the scheduled 2026-06-12 full-source review
(`docs/CODE_ANALYSIS_2026-06-12.md` — Top-10 locally re-verified: 8 confirmed,
2 refuted; the 8 confirmed findings are all addressed here). No migration; one
small API addition noted below.

### Fixed

- **Network receive-queue overflow accounting** (the round's two high findings) —
  `ReceiveQueueFull.dropped` now accumulates across every rejected message (was a
  constant `1`, silently discarding all subsequent overflow), and events already in
  the queue are never evicted once the marker is installed. When the marker is first
  installed it displaces the youngest queued event, which is now *counted*
  (`dropped` starts at 2). Queue length never exceeds the configured capacity.
  Native and wasm paths are semantically identical.
- **Crossfade interrupt pop** — calling `play_with_crossfade` toward a *third* clip
  while a blend is in flight now promotes the in-flight TO side to the new FROM
  (`mix(B, C, 0)` on the first frame) instead of popping back to the original FROM
  clip image. The 5.1.1 same-target idempotency guard is unaffected.
- **Crossfade completion stutter** — completion now carries the to-clip's accumulated
  sub-frame timer into `AnimationPlayer.timer` (with this tick's `dt` counted exactly
  once) instead of resetting to `0.0`, which visibly stretched the first post-blend
  frame on low-fps clips.
- **Silent collision-event drop warning** — `PhysicsSystem` now `log::warn!`s once
  when collisions/triggers occur but `Events<CollisionEvent>` /
  `Events<TriggerEvent>` was never registered, naming the exact
  `register_event` call to add (previously the events vanished with no signal).
- **Per-sprite `Arc<str>` re-allocation** — the renderer's `image_handle` path uses
  the new `Handle::path_arc()` (O(1) refcount bump) instead of `Arc::from(h.path())`
  (per-sprite per-frame string copy).
- **Doc gaps** — `docs/PATTERNS.md` ordering table gains the
  `BlendTreeSystem` before `AnimationSystem` row; `AmbientLight` / `PointLight` /
  `LightingRenderer` doc comments now state the native-only / wasm32-no-op limitation.

### Added

- `Handle::path_arc() -> Arc<str>` — owned handle path without copying the string.

## 5.1.1

Bug-fix batch from the post-release code review of the 5.1.0 features (10 confirmed
findings, three root causes). No migration needed; one small API addition noted below.

### Fixed

- **Audio release envelope redesigned** (root cause: shadow state + stale volume reads).
  `stop()` during *any* in-progress `stop_when_done` fade (release **or** `fade_out`)
  now cuts immediately — `fade_out` is a real bypass path as documented, and a second
  `stop()` mid-release still cuts. The release fade starts from the **current
  interpolated** fade position instead of the stale override (no more start-of-release
  pop). Completed teardown fades no longer persist `0.0` into the channel volume, so
  the next `play_*` on a reused channel starts at the `set_volume` level (regression
  fix). `stop()` on a naturally-drained sink cuts immediately instead of scheduling a
  silent release. Internals: the `releasing` HashSet is gone; `Fade` construction is
  unified (`Fade::stop_fade`) with one consistent minimum-duration rule.
- **State-machine crossfade guards** (root cause: `current_clip` stays on the FROM clip
  during a blend). `AnimationPlayer::play_with_crossfade` re-fired with the same target
  mid-blend is now idempotent — oscillating threshold transitions can no longer reset
  the blend every frame. `StateMachineSystem` evaluates `AnimationEnd` via the new
  `AnimationPlayer::is_clip_finished(clip_index)` (returns true only when not
  crossfading and that clip is the finished current clip), so a crossfaded-into
  one-shot state plays its clip to completion instead of exiting on the first frame.
  The `AnimationStateMachine` ↔ `BlendTree1D` interaction is now documented (SM
  transitions intentionally interrupt an in-progress BT blend; avoid driving the same
  player with both unless that is desired).
- **Script steering commands are mutually exclusive** — `seek_target` / `flee_from` /
  `arrive_at` / `wander` each remove the other three steering components before
  attaching their own (previously a single `wander()` permanently overrode later
  commands via the steering system's last-writer-wins order), and `stop_steering()`
  removes all four so a stopped entity stays stopped. Rust-side multi-component
  steering composition is unaffected.

### Added

- `AnimationPlayer::is_clip_finished(clip_index)` — crossfade-aware finish check used
  by the state machine; public for game code with the same need.

## 5.1.0

The three feature candidates deliberately split out of the 2026-06-10 analysis round,
each validated by a playable example per the `docs/VISION.md` loop. Fully additive —
no migration needed from 5.0.0.

### Added

- **Per-transition crossfade on `AnimationStateMachine`** — `AnimTransition` gains a
  `crossfade_duration: f32` field (default `0.0` = hard switch, the previous behavior)
  and `add_transition_crossfade(from, to, conditions, duration)` registers a transition
  that blends into the target clip. `StateMachineSystem` drives the existing
  `AnimationPlayer::play_with_crossfade` path — the same 2-UV shader-lerp used by
  `BlendTreeSystem`, no new blend machinery. `add_transition` keeps its signature
  (now a thin wrapper with `0.0`). Example: `sm_crossfade` (side-by-side hard-switch
  vs. crossfaded character; run `gen_blend_sheet` first).
- **Rhai steering bindings for `Arrive` / `Wander`** — scripts can now use the full
  steering set (previously only Seek/Flee were bound):
  `arrive_at(tx, ty, speed, slow_radius, stop_radius)` and
  `wander(speed, change_interval)`, following the existing `seek_target`/`flee_from`
  conventions (f64 params, last call per frame wins, `SteeringVelocity` auto-attached).
  The Wander apply step preserves the component's internal timer/direction so per-frame
  script calls don't reset the direction-change rhythm. Example: `script_steering_game`
  (mouse-following Arrive agent + autonomous Wander agent, both script-driven).
- **`AudioEffect::release_secs` implemented** (was a documented no-op stub) —
  `AudioManager::stop` on a channel whose effect has `release_secs > 0.0` now fades the
  volume to zero over that duration through the existing fade machinery, then tears the
  sink down. `0.0` keeps the immediate cut. A second `stop` during the release, or a new
  `play_*` on the channel, cuts immediately. Requires `AudioSystem` (or manual
  `update(dt)`) to progress, like all fades. Example: `audio_fades` (extended — R/S/I
  keys demo release vs. immediate stop).

## 5.0.0

The breaking batch from the 2026-06-10 analysis (`docs/CODE_ANALYSIS_2026-06-10.md`):
Top-10 items #2 and #8, removal of everything deprecated in 4.6.0, the visibility
narrowings triaged out of the 4.6.0 sweep, and small breaking consistency items.
Every change below lists its migration.

### Breaking — removed (all deprecated since 4.6.0)

- **`DebugDrawQueue` / `DebugRect`** — migrate to `DebugDraw::rect_filled_z(min, max, color, z)`
  (or `rect_filled` for z = 0).
- **`World::register_reflect`** — use `register_reflect_named::<T>("Name")` (the removed
  overload stored an empty type name and broke the Inspector display).
- **`NetworkEvent::JsonParseError`** — never emitted by the engine; delete the match arm
  (protocol-level parse errors are the game's concern).
- **`App::load_texture`** — use `load_image` (returns a `Handle<ImageAsset>`, participates
  in hot reload).
- **`ParticleEmitter::for_burst`** — renamed to `ParticleEmitter::burst` in 4.6.0.
- **Pre-v5 re-export shims** — `animation::player::{UvRect, BlendUv}` → `renderer::uv`,
  `timeline::Lerp` → `tween::Lerp`, `prefab::topological_sort_entities` → `hierarchy`,
  and the `components::*` migration facade (`AnimationClip`, `AnimationPlayer`, `UvRect`,
  `FontData`, `GameState`, `PendingResize`, `ShouldQuit`, `ViewportSize`, `WindowConfig`).
  All root re-exports (`engine::UvRect`, `engine::Lerp`, `engine::topological_sort_entities`, …)
  keep working — only the deep legacy paths are gone.

### Breaking — API changes

- **Physics handle newtypes (analysis #2)** — `PhysicsWorld` no longer leaks rapier types:
  new `BodyHandle` / `ColliderHandle` newtypes (mirroring `JointHandle`) flow through every
  factory return, `PhysicsBody`'s fields, `RaycastHit.collider_handle`, raycasts, joints,
  `move_character`, and the collider accessors. *Migration:* code that only passes handles
  back into `PhysicsWorld` compiles unchanged via inference; code naming rapier handle types
  imports `engine::{BodyHandle, ColliderHandle}` instead. Escape hatch for forks that drop
  to raw rapier: `.raw()` on both newtypes, and `rigid_body[_mut]` / `get_collider[_mut]`
  still return raw rapier references.
- **`Scene::on_enter` takes a `SystemRegistrar` (analysis #8)** — scenes can finally
  register systems with label ordering. *Migration:*
  `fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>)` →
  `fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar)`;
  `systems.push(Box::new(X))` → `systems.add(X)`; ordering:
  `systems.add_labeled(X, SystemConfig::new().after(Y::LABEL))`. The settings_menu example
  demonstrates a real constraint (`UiSystem` after `LayoutSystem`).
- **`Sprite.texture` is `Option<Arc<str>>` (analysis #9 remainder)** — per-sprite per-frame
  batch-key `String` clones become refcount bumps. *Migration:* `Sprite::textured("x.png")`
  and `textured_with_handle` keep compiling (`impl Into<Arc<str>>`); struct literals need
  `texture: Some("x.png".into())`. RON/serde wire format unchanged.
- **`SystemMeta` merged into `SystemConfig`** — they were field-for-field identical.
  *Migration:* replace the name; `compute_order` now takes `&[SystemConfig]`.
- **`ShaderMaterial` caches its pipeline hash** — construct via
  `ShaderMaterial::new(frag_source, params)`; `frag_source` is private behind
  `frag_source()` / `set_frag_source()` (which re-hashes), so the cached hash can never
  desync. `params` stays pub. The renderer's per-frame WGSL hashing is gone.
- **`#[non_exhaustive]` on `DebugShape` and `NetworkEvent`** — external matches need a
  `_ =>` arm; future variants stop being breaking changes (`ReflectValue` precedent).
- **Visibility narrowings** — `GpuLightData` / `LightingUniforms` and
  `PostProcessRenderer.{target_view,width,height}` are `pub(crate)` (GPU internals);
  `TouchState` event fields are private behind `began()` / `moved()` / `ended()` /
  `pinch_delta()` / `swipe()` accessors; `input` submodules are private — import from
  `engine::input::{…}` or the crate root (`engine::AxisBinding` etc. unchanged).

### Changed

- Examples write quits via `ShouldQuit::quit()` instead of `q.0 = true` (field stays pub;
  examples teach the canonical API).

## 4.6.0

Non-breaking batch from the 2026-06-10 full-codebase analysis
(`docs/CODE_ANALYSIS_2026-06-10.md`, Top-10 items #1/#3/#4/#5/#6/#7/#9-partial/#10),
plus a follow-up sweep over ~30 of the remaining non-Top-10 findings (2026-06-11).
The remaining Top-10 items (#2 rapier handle newtypes, #8 `on_enter` system
registrar, plus removal of everything deprecated here) form the planned v5 breaking batch.

### Added

- **`LABEL` constants on all built-in systems** — Physics/CollisionGrid/CollisionDebug/
  Network/Particle/Tilemap/Audio/SkeletalAnimation/Hierarchy/Steering/Behavior/
  Localization/Scripting/Timeline join the five systems that already had one, so every
  engine system can now be referenced in `add_system_labeled` ordering. The platformer
  example demonstrates labeled registration; `docs/PATTERNS.md` gained a
  "System ordering with labels" section with the known constraints.
- **`SceneChange::take` / `is_pending`**, **`ShouldQuit::quit` / `is_quitting`**,
  **`ParticleEmitter::burst`** (canonical name for `for_burst`), and a root re-export
  for **`NetworkConfig`** — small API-surface consistency additions.

- **`save::write_ron` / `save::read_ron`** — plaintext pretty-RON read/write for design-time
  assets. `SceneDef`/`Prefab` `save`/`load` now produce human-editable text files instead of
  AEAD-encrypted binary (a hackability violation for level files); `read_ron` transparently
  falls back to the encrypted format so pre-4.6 files still load. Encrypted `save`/`load`
  remain the player-save path.
- **`DebugDraw::rect_filled` / `rect_filled_z`** — filled, z-ordered rectangles on the modern
  debug-draw resource, covering everything the legacy queue did.
- **Native `NetworkClient::is_connected()`** — parity with the wasm client (previously
  wasm-only, an undocumented platform API split); backed by an `AtomicBool` the socket
  thread clears on every exit path.

### Changed

- **`UvRect`/`BlendUv` moved to `renderer::uv`**, **`Lerp` moved to `tween`** — semantic
  homes instead of accidental ones (`animation::player`, `timeline`); six modules no longer
  compile-depend on `animation`, and `network::SnapshotBuffer` no longer imports the cutscene
  module. Old paths and all root re-exports keep working via `pub use` shims.
- **Editor state extracted from `App`** — 17 editor-only fields (gizmo, clipboard, undo
  history, component factories, snap, selection) now live in one internal `EditorState`
  struct (`src/app/editor/state.rs`); a fork removes the editor by deleting one field + one
  module. Internal-only; no public API change.
- **Per-frame allocation fixes** — the lighting pass no longer creates its bind group every
  frame (cached, invalidated on resize/reconfigure); the sprite renderer no longer clones
  WGSL material sources per frame (at most once per *new* pipeline). Remaining per-sprite
  texture-key `String` clones need an API break and are deferred to v5.
- **Findings-sweep cleanups (2026-06-11)** — per-frame allocation pass (text queue drained
  via `mem::take`, physics event-diff scratch buffers reused, single-pass particle emitters
  and panel layout, editor-UI allocations gated behind `is_enabled`, `exec_order` take/swap);
  `topological_sort_entities` rehomed `prefab` → `hierarchy` (shim kept); O(1) `despawn`;
  deduplicated `App::new` / `AssetServer::new` struct literals, editor Tag/multi-select UI
  blocks, input bind methods, and the fullscreen-quad vertex shader; dead private
  `play_streaming` removed; doc clarifications across modules (wasm no-op fades,
  CollisionGroups vs CollisionLayer, LocaleResource bridge, system-ordering caveats).

### Fixed

- **Animation frame catch-up** — the main-clip advance now catches up multiple frames on a
  large `dt` (previously advanced at most one frame per tick; the crossfade path already
  did this correctly).
- **`CharacterController::max_slope_angle` desync** — setting the public field directly now
  takes effect on the next `move_character` call (previously only `with_max_slope_deg`
  synced the internal rapier controller).

### Deprecated

- **`DebugDrawQueue` / `DebugRect`** — superseded by `DebugDraw::rect_filled_z`. Still
  registered and drained for compatibility; removal planned for v5. `CollisionDebugSystem`,
  the editor selection highlight, and the `sokoban` example are migrated.
- **`World::register_reflect`** (stores an empty type name, breaking Inspector display — use
  `register_reflect_named`), **`NetworkEvent::JsonParseError`** (never emitted by the
  engine), **`App::load_texture`** (use `load_image`), and **`ParticleEmitter::for_burst`**
  (renamed `burst`). All removal-planned for v5.

## 4.5.0

### Added

- **`salvage_run` example** (`examples/games/salvage_run/`) — an **area-of-interest (AOI)
  streaming** networked world: a single ship roams a world far larger than the window (2400×1800 vs
  800×600) while an authoritative server simulates ~120 wandering entities of two typed kinds
  (slow-drifting salvage, roaming drones) and streams each client **only** the entities within an
  interest radius of its last-reported position. Entities continuously stream in and out as the
  player moves — interest management made visible: a live "streaming X / 120" readout, a resizable
  AOI (`-` / `=`) with an on-screen boundary ring, and entity pop-in/out at the edge. Reuses
  `engine::SnapshotBuffer<Vec2>` per streamed entity (its third call site) for smooth motion at a
  low 12 Hz, two `RemoteEntities` maps for the two kinds, example-local last-seen + timeout eviction
  for entities that leave the AOI (the server signals departure only by *omission*), and
  `RemoteEntities::clear` on disconnect. The first example to stress AOI churn / staleness and to
  tear down on disconnect — see `docs/REMOTE_ENTITIES_DESIGN.md` (#4/#5/#7). Ships native + to the
  browser (`web/`). No engine API change (purely additive example).

## 4.4.0

### Added

- **`SnapshotBuffer<T: Lerp>`** — a generic per-entity snapshot-interpolation buffer for smoothing
  server-owned remote state that arrives at a low snapshot rate. Stamp each snapshot with the
  client clock (`push`), then `sample` a slightly delayed render time so playback always
  interpolates between two real samples (clamping at the ends). Generic over any `Lerp` value, so
  it interpolates `f32` (e.g. a rotation angle), `Vec2` (a position), `Color`, etc. It is
  **orthogonal** to `RemoteEntities`: that owns the `id → Entity` lifecycle, this owns the value
  history the renderer reads — games keep them as parallel maps. This is the promotion of
  `predict_shooter`'s former private `Interp` (now migrated onto it), triggered by a second
  interpolating example — see `docs/REMOTE_ENTITIES_DESIGN.md`.
- **`orbital_dodger` example** (`examples/games/orbital_dodger/`) — an interpolation-only networked
  game: cross the field to a vault while dodging the server's drifting, spinning hazards. The
  hazards are wholly server-authoritative at a low 10 Hz; the local player never round-trips, so
  the only netcode is interpolation (no prediction). Each hazard interpolates two channels —
  position (`SnapshotBuffer<Vec2>`) and spin angle (`SnapshotBuffer<f32>`) — which is what
  justified making the buffer generic. `I` toggles interpolation off to reveal the raw 10 Hz
  judder. Ships native + to the browser (`web/`).

## 4.3.1

### Fixed

- **Gamepad backend crash isolated (gilrs).** A controller could panic gilrs inside its event poll
  (`gamepad(id).unwrap()` on `None`), crashing the whole app ~1 s after launch. `App::poll_gilrs`
  now wraps the poll in `catch_unwind` (mirroring the per-system isolation in `schedule.rs`) — a
  flaky controller disables gamepad input for the session instead of crashing — and gilrs was
  upgraded `0.10 → 0.11.2` (reworked macOS HID backend). Note: on macOS the GameController
  framework takes *exclusive* ownership of Xbox/PlayStation pads, so gilrs (IOKit HID) sees a
  `Connected` event but no input; gamepad input works on Linux/Windows or with a generic-HID pad.

## 4.3.0

### Added

- **`RemoteEntities<K>`** — a reusable helper for the `id → Entity` lifecycle that networked games
  repeat: spawn-on-first-sight and despawn-on-removal of server-owned remote entities. Methods:
  `get_or_spawn`, `get`, `contains_key`, `remove` (despawns the entity), `clear`, `len`,
  `is_empty`, `iter`. It owns only the mapping plus spawn/despawn lifecycle — what to spawn (a
  closure), how to update an existing entity, and any parallel game-state maps stay in the game.
  The `mp_client` and `coin_race` examples now use it instead of inline `HashMap<usize, Entity>`
  bookkeeping. A richer version (interpolation, client-side prediction, update callbacks) is
  deliberately deferred until a third distinct networked example reveals its shape — see
  `docs/REMOTE_ENTITIES_DESIGN.md`.

## 4.2.0

### Changed

- **Crisp wasm rendering on Retina/HiDPI.** The wasm drawing buffer is now sized to the canvas's
  logical size × `devicePixelRatio` (uniform scale, capped so neither axis exceeds the WebGL2 2048
  max texture size) while the canvas CSS display box stays at the logical size, so the browser maps
  the buffer 1:1 instead of upscaling a logical-size buffer. Previously wasm rendered into a
  logical-size buffer (a deliberate `scale_factor = 1` workaround) — correct, but soft on Retina.
  The world viewport stays logical and `DisplayScaleFactor = buffer / logical`, so sprites and UI
  keep their coordinates and text now renders at device resolution. The logical size is read from
  the authored `<canvas>` width/height attributes (stable across scene transitions), not
  `WindowConfig`. Native rendering is unchanged.

## 4.1.0

### Added

- **`coin_race` runs in the browser (wasm).** The `coin_race_game` client now compiles to
  wasm and connects to the native `coin_race_server` over `ws://127.0.0.1:9002`, so native
  windows and browser tabs share one authoritative game. A `#[wasm_bindgen] run_coin_race`
  entry point lives in the example (not the engine library, keeping the engine a
  genre-agnostic skeleton), and `examples/games/coin_race/web/` adds an `index.html` plus a
  `build.sh` that drives `cargo build --example` + `wasm-bindgen`. This establishes the
  reusable path for shipping an engine *example game* to the web: previously only the bundled
  library demo (`examples/wasm/`, built with `wasm-pack`) could run in a browser, because
  `wasm-pack` builds only the library crate. Verified end-to-end — a browser tab's wasm
  WebSocket connects to the authoritative server and renders the player avatar and the
  server-spawned coin field via WebGL2.
- **Embedded default font for wasm text.** The browser sandbox has no system fonts, so
  `FontSystem::new()` loads an empty font db and the engine previously skipped creating the
  text renderer on wasm entirely (cosmic-text panics shaping with no fonts), meaning
  `DrawText`/HUD text silently did not render unless the game supplied a `FontData`. The engine
  now embeds DejaVu Sans (`assets/fonts/DejaVuSans.ttf`, Bitstream Vera / Arev license) and
  falls back to it on wasm when no `FontData` is set, so HUD text renders out of the box. The
  font is `include_bytes!`'d under a wasm-only `cfg`, so native binaries (which use OS fonts)
  do not embed it.

### Fixed

- **Wasm HiDPI viewport was halved on Retina displays.** The logical `ViewportSize` was
  computed as `surface_size / devicePixelRatio` for all targets, but on wasm the surface is
  already sized to the canvas DOM (CSS-logical) size — the resize handler caps it there to
  respect the WebGL2 texture limit — so dividing by the DPR again halved the world viewport.
  On a Retina display (DPR 2) a fixed-coordinate scene was projected into a half-size viewport
  and rendered almost entirely off-screen; the engine only rendered correctly at DPR 1. The
  DPR division now applies on native only (where the surface is physical pixels). Surfaced by
  playtesting the `coin_race` wasm example on a real Retina display — sprites that were pushed
  off-screen now render in place. (The `examples/wasm/` lib demo masked this because it adapts
  its layout to `ViewportSize` instead of using fixed coordinates.)
- **Wasm canvas was stretched, clipping HUD text.** winit sizes the canvas's CSS *display* box
  to the window's logical size (the 1280 default when `WindowConfig` isn't applied at canvas
  creation), which can differ from the drawing buffer — so the browser stretched an 800px buffer
  across a 1280px display and, being wider than the window, centred and clipped it. Fixed-position
  HUD text fell off the left edge while sprites (mid-canvas) stayed visible. `finish_init` now
  sets the canvas CSS width/height to its drawing-buffer size (after winit has sized it) so the
  canvas displays 1:1 with what the engine renders; a game can still override with `!important`
  CSS. Surfaced once the embedded default font made wasm text render for the first time.

## 4.0.0

### Added

- `coin_race` example (`examples/games/coin_race/` — `coin_race_game` client +
  `coin_race_server`): first playable-game use of `NetworkClient` / `NetworkSystem` /
  `NetworkEvent` in an **authoritative** design, not the position-relay model of
  `mp_client`/`mp_server`. Two or more players race to collect coins; the standalone server
  owns the coin field and the scoreboard, arbitrates contested pickups (first `grab` claim
  wins), keeps the field full, and announces the winner. Closes the last engine subsystem —
  networking — that had no playable-game example. No engine source changed: `NetworkClient`
  and `NetworkSystem` carried a full authoritative game as-is, confirming the API is
  sufficient for this pattern.

### Breaking

- **`engine::ImpulseJointHandle` (a re-export of `rapier2d`'s `ImpulseJointHandle`) is
  removed and replaced by the opaque `engine::JointHandle` newtype.**
  `PhysicsWorld::add_revolute_joint`, `add_distance_joint`, and `add_prismatic_joint` now
  return `engine::JointHandle`, and `remove_joint` takes it. The inner rapier handle is
  engine-private: a `JointHandle` can only be produced by an `add_*_joint` call, decoupling
  game code from the rapier type. Migration: replace `use engine::ImpulseJointHandle` (or
  `use rapier2d::dynamics::ImpulseJointHandle`) with `use engine::JointHandle`, and update
  return-type annotations / stored fields accordingly. Call sites that discard the return
  value need no change.

## 3.0.0

### Added

- `Color` newtype (`engine::Color`): a single unified RGBA color type with `rgb` / `rgba` /
  `rgba_u8` constructors, `From<[f32; 4]>` / `From<[f32; 3]>` / `From<[u8; 4]>` conversions,
  and `to_array` / `to_u8` / `to_rgb` helpers for the GPU / glyphon boundaries. Replaces the
  previous mix of raw color arrays throughout the public API (see Breaking).
- `AudioSystem` (`engine::AudioSystem`): a built-in system — register it like any other —
  that calls `AudioManager::update(dt)` every frame so scheduled fades (`fade_out` /
  `fade_volume`) actually advance. Previously fades were silently inert unless the game
  manually drove `update()`. Also adds an SFX file-bytes cache (path → `Arc<[u8]>`) so
  replaying the same sound effect no longer re-reads the file from disk on every `play()`;
  streaming BGM is unchanged.
- `DrawText::centered` + `TextAnchor` enum (`engine::TextAnchor`, `TopLeft` / `Center`): a
  draw position can now anchor at the measured text center, computed from the shaped buffer
  at render time with no manual `-width/2` math. Paired with `Camera::world_to_screen` (the
  inverse of `screen_to_world`) for placing screen-space text at a world position.
- `MouseButton` re-exported as `engine::MouseButton`, so games can import it from the crate
  root instead of reaching into `winit::event::`.
- `ReflectValue::I32` variant + `#[non_exhaustive]` on `ReflectValue`: integer fields are now
  inspectable in the egui Inspector alongside `F32`. `#[non_exhaustive]` means downstream
  exhaustive `match`es over `ReflectValue` must add a `_` arm.
- `ScriptingLimits` extended with `max_string_size`, `max_array_size`, `max_map_size`,
  `max_call_levels`, and `max_expr_depth`, all applied to the Rhai engine alongside the
  existing `max_operations`, with conservative defaults for trusted-local scripts.
- `spawn_scene_def` duplicate-`Tag` detection: duplicate tag keys are now first-wins with a
  `log::warn!` instead of silently overwriting. All entities still spawn; only parent-tag
  resolution is affected.
- `audio_fades` example (`examples/audio_fades.rs`): a small native demo confirming the new
  built-in `AudioSystem` drives fades in real play (Space to play, F to fade out, 1/2/3 to
  fade to a target volume) — previously the same sequence produced no audible change.
- `minimap` example gained a `WorldLabelSystem` that draws floating `"ENEMY"` nameplates
  above each enemy via `Camera::world_to_screen` + `DrawText::centered`, tracking them as the
  camera follows the player — the first live exercise of those two APIs with a moving camera.

### Breaking

- **`PhysicsWorld` is now a `World` resource**, not a field owned by `PhysicsSystem`.
  `PhysicsSystem::run` takes the resource out of the world, steps it, and re-inserts it, so
  game systems reach physics symmetrically with `world.resource_mut::<PhysicsWorld>()` —
  matching how `SpatialGrid` is exposed. Migration:

  ```rust
  // before
  let physics = PhysicsWorld::new();
  app.add_system(PhysicsSystem::new(physics, pixels_per_unit));

  // after
  app.world.insert_resource(PhysicsWorld::new());
  app.add_system(PhysicsSystem::new(pixels_per_unit));
  ```

- **All public color fields changed to `engine::Color`.** `Sprite`, `AtlasSprite`,
  `PointLight`, `AmbientLight`, `DrawRect`, `DrawText`, `DrawImage`, `ParticleEmitter`,
  `GpuParticleEmitter`, the `Timeline` color track, all UI widgets (`Button` / `CheckBox` /
  `Panel` / `ScrollView` / `Slider` / `TextInput` / `Label`), and `DebugDraw` previously held
  a mix of `[f32; 4]` / `[f32; 3]` / `[u8; 4]`. Color-accepting constructors and builders take
  `impl Into<Color>`, so call sites passing raw arrays still compile; only struct-literal
  `color:` initializers need updating (e.g. `color: [r, g, b, a]` → `color: Color::rgba(r, g,
  b, a)`). Raw arrays remain only at the GPU/glyphon boundaries via `to_array` / `to_u8` /
  `to_rgb`. Scene RON now serializes color as `(r:.., g:.., b:.., a:..)` struct form.

### Fixed

- **Per-frame hot-path costs removed.** `SpatialGrid` / `CollisionGridSystem` rebuild the
  world resource in place (remove → rebuild → insert) instead of deep-cloning two `HashMap`s
  into the resource every frame. `ScriptAsset.ast` is now `Arc<rhai::AST>` (clone = refcount
  bump, not a full AST deep-clone per scripted entity per frame), and `ScriptingSystem` reuses
  thread-local scratch buffers. A\* pathfinding (`find_path`) gained a closed set to prevent
  re-expanding stale heap entries and reuses its open list / score maps across calls (public
  signature unchanged). The sprite renderer opens a single render pass for the whole pre-sorted
  sprite stream and issues per-texture-run draws within it, instead of a new pass per batch.
- **`RenderLayer` negative values no longer fold to bit 0.** Layer→mask matching previously
  `clamp(0, 31)`-ed the layer index, so a `RenderLayer(-1)` background sprite mapped onto bit 0
  and leaked into `layer_mask: 1 << 0` offscreen passes. Layers outside `0..=31` cannot be
  addressed by a 32-bit mask and now simply never match under any non-zero mask (they still
  render under mask `0` = all layers); the engine warns once on an unmaskable layer.
- **Point-light radius falloff locked by a contract test.** A code-analysis pass flagged the
  CPU `radius * zoom / viewport_w` calculation as a possible unit mismatch; re-derivation
  confirmed it a false positive (the value is already in the shader's UV-fraction-of-width
  space and the falloff reaches zero at the world radius). A regression test now pins the
  correct behavior so a future "fix" cannot reintroduce a 2× error.

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
- `security_camera` example (`examples/security_camera.rs`): first playable-game use of
  `RenderTarget` / `OffscreenCamera`. A stealth puzzle where a guard patrols a room that is
  **entirely offscreen** — its only view is a wall monitor (an `OffscreenCamera` renders the guard
  room into a `RenderTarget` that a `Sprite` samples). Read the guard's position on the monitor and
  cross the doorway when it is away from the door stripe; reach the exit to escape, get caught to
  reset (`R` replays, `Esc` quits). The existing `minimap`/`split_screen` demos exercised the API but
  only ever framed the *same* region the main camera shows; this is the first use of an offscreen
  camera as the sole view of a **disjoint** region. Demonstrates the offscreen-render fix below.
- `timeline_cutscene` example (`examples/timeline_cutscene.rs`): first use of `Timeline` in a
  playable scene. Walk into a rune to trigger a cutscene that pans/zooms the camera, slides two gate
  panels apart, and fades a full-screen overlay — all driven by `Timeline` keyframe tracks; Space
  skips, control returns when it ends, then you cross the now-open gate to the exit. `Timeline`
  shipped with unit tests but had zero example/game coverage. Demonstrates the camera-drive addition
  below.
- `CameraTarget` marker component (`engine::CameraTarget`) + a `Timeline::zoom` track: a `Timeline`
  on an entity tagged `CameraTarget` drives the `Camera` resource (its `position` track → camera
  position, `zoom` track → camera zoom) as a virtual camera rig, instead of the entity's own
  `Transform`/`Sprite`. Lets a `Timeline` author camera moves for cutscenes — previously `Timeline`
  could only animate an entity's own transform/sprite. Additive: ordinary timelines are unaffected
  (the `zoom` track is empty by default).

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
- Offscreen render targets (`OffscreenCamera` → `RenderTarget`) now render with their **own** camera
  instead of the main camera. The sprite renderer's camera uniform is a single shared buffer updated
  via `queue.write_buffer`; the offscreen pass and the main pass were recorded into one command
  submission, and within a single submit only the **last** write to that buffer takes effect — so
  every offscreen target was drawn with the (later-written) main camera's view. The
  `minimap`/`split_screen` demos masked this because their offscreen content overlaps the main view;
  it became obvious only with an offscreen camera framing a *disjoint* region (the monitor rendered
  the main scene instead of the guard room). Each offscreen target now submits in its own command
  buffer so its camera write pairs with its own draws. Surfaced by `security_camera`; GPU-validated
  by a native run (CI compiles but cannot run the windowed app).
- `split_screen` example no longer crashes with a wgpu validation error on its second frame. It used
  `layer_mask: 0` (render all layers), so its render-target *display* sprites were drawn into the
  same targets they sample — a self-capture (a texture used as both color attachment and sampled
  resource within one render pass). It "survived" only frame 1, before the targets were registered.
  Fixed by masking the display sprites out of the offscreen pass (`layer_mask: 1 << 0`), the same
  self-capture-avoidance `minimap` already uses.

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
