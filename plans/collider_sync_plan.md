# Item 2 — Collider sync while painting / mutating a tilemap (v8.24.0)

## Goal

Keep a tilemap's static physics colliders in sync when the tilemap is mutated — by the editor's Tile
Paint (the headline ask) or a game's runtime `set_tile`. Opt-in, additive, reusable both ways.

## Why this design

- `PhysicsWorld::sync_static_from_tilemap(tm, ppu, collider_for, &mut index)` already does incremental
  diff sync, but it needs the game's `ppu`, solid-tile rule, and a persistent `TileColliderIndex` — none
  of which the generic editor knows. `dig_quest` carries them by hand (a `tile_index` resource + a
  `sync_colliders` method that clones the tilemap, `with_resource_mut::<PhysicsWorld>`, and calls
  `sync_static_from_tilemap` with `|v| (v != 0).then(TileCollider::solid)`).
- **Package that exact pattern as an opt-in component** `TilemapColliders { pixels_per_unit, solid,
  index }` + a reusable `App::sync_tilemap_colliders(entity)` / free fn. A game (or the editor) attaches
  the component once; mutating the tilemap then calls one method to re-sync. `dig_quest` is refactored
  onto it — proving the API in real play and deleting its hand-rolled dance (VISION: API extracted from
  real usage).

## Scope (additive)

- `TilemapColliders` component (`src/physics/world/tile_collider.rs`): `pixels_per_unit: f32`,
  `solid: SolidTiles`, private `index: TileColliderIndex`. Methods: `new(ppu, solid)`,
  `sync(&mut self, &mut PhysicsWorld, &Tilemap)`, `collider_count() -> usize`.
- `SolidTiles` enum: `NonZero` (any non-zero tile → solid box) | `Only(Vec<u32>)`. `collider_for(id) ->
  Option<TileCollider>` (solid → `TileCollider::solid()`).
- Free fn `sync_tilemap_entity_colliders(world, entity) -> bool` — the clone-tilemap +
  `with_resource_mut::<PhysicsWorld>` + `get_mut::<TilemapColliders>` dance. Returns whether a sync ran.
- `App::sync_tilemap_colliders(entity) -> bool` — thin public wrapper over the free fn.
- **Editor wiring:** `commit_paint_stroke` calls `App::sync_tilemap_colliders(sel)` after pushing the
  `PaintTiles` cmd; `EditorHistory::{undo,redo}` `PaintTiles` arms call the free fn after the `set_tile`
  loop — so paint, undo, and redo all keep colliders consistent.
- **Example:** refactor `dig_quest` to attach `TilemapColliders` + call the free fn (replaces its
  `tile_index` field + `sync_colliders`), preserving behavior exactly.
- Exports: `physics` mod + `lib.rs` (`TilemapColliders`, `SolidTiles`).
- Version bump 8.23.0 → 8.24.0 (Cargo.toml, CLAUDE.md header + module-map row, CHANGELOG).

## Completion criteria

1. `cargo test --lib` green, +tests:
   - `tilemap_colliders_sync_adds_and_removes` — build from a tilemap (real `PhysicsWorld`), paint a
     cell → `collider_count` up; erase → down (drives `sync` through the real physics path).
   - `app_sync_tilemap_colliders_noop_without_physics_or_component` — returns false / no panic.
   - `solid_tiles_rule_nonzero_and_only` — `collider_for` logic.
2. Full Gate6 green; `dig_quest` still builds + behaves (it's the runtime example).
3. Editor: painting a `Tilemap` that has `TilemapColliders` + a `PhysicsWorld` re-syncs colliders
   (validated by an editor-path unit test driving `commit_paint_stroke`).
4. `rust-survivors` unaffected (purely additive new public types/method).

## Out of scope

- Per-cell collider *kind* changes (groups/one_way) — `sync_static_from_tilemap` v1 only diffs presence
  (documented limitation); changing a cell's kind needs clear-then-restore.
- Visualizing rapier tile colliders in the editor overlay (they're `PhysicsWorld` bodies, not `Collider`
  components; the Bounds overlay shows the latter). Possible follow-up.
