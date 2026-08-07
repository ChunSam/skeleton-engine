use std::collections::{HashMap, HashSet};

use crate::components::{Sprite, Transform};
use crate::ecs::{Entity, System, World};
use crate::renderer::uv::UvRect;

use super::animation::{AnimatedTileCell, TileAnimationSet};
use super::autotile::{compute_tile_mask, compute_tile_mask_typed, AutotileMode, TilemapAutotile};
use super::Tilemap;

// ─── System internals ─────────────────────────────────────────────────────────

/// Per-tilemap-entity view tracked by [`TilemapSystem`].
struct TilemapView {
    /// (row, col) → tile entity
    cells: HashMap<(usize, usize), Entity>,
    /// Last-seen tile grid (for diffing)
    cached_tiles: Vec<Vec<u32>>,
    /// Last-seen dimensions
    cached_dims: (usize, usize),
    /// Generation counter mirrored from [`Tilemap::generation`] after each rebuild.
    /// When this equals `Tilemap::generation` and dims are unchanged, the diff is skipped.
    cached_generation: u64,
}

// ─── Systems ──────────────────────────────────────────────────────────────────

/// System that reads Tilemap components and manages tile entities.
///
/// - Spawns tile entities the first time a Tilemap entity is seen.
/// - Reacts to tile mutations each frame: despawns removed tiles, spawns new ones,
///   and updates UvRect in place for changed tiles.
/// - Despawns all tile entities when the Tilemap entity disappears.
/// - When a [`TilemapAutotile`] component is present, chooses display UVs based on
///   neighbor connectivity and propagates UV updates to the 8 surrounding cells when
///   any cell changes.
pub struct TilemapSystem {
    views: HashMap<Entity, TilemapView>,
}

impl TilemapSystem {
    pub fn new() -> Self {
        Self {
            views: HashMap::new(),
        }
    }
}

impl Default for TilemapSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl TilemapSystem {
    /// Schedule label for ordering via `add_system_labeled`.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::tilemap";
}

/// Spawns a single tile entity for cell `(row, col)` with the given UV.
///
/// If `anim_cell` is `Some`, the `AnimatedTileCell` marker is also attached so
/// `AnimatedTileSystem` can advance the animation each frame.
fn spawn_tile_entity(
    world: &mut World,
    tm: &Tilemap,
    row: usize,
    col: usize,
    uv: UvRect,
    anim_cell: Option<AnimatedTileCell>,
) -> Entity {
    // Position + depth come from the tilemap's projection (orthographic square or isometric
    // diamond), so this one call site handles both.
    let position = tm.cell_center_world(row, col);
    let tile_entity = world.spawn();
    world.add_component(
        tile_entity,
        Transform {
            position,
            scale: tm.cell_render_size(),
            rotation: 0.0,
            z: tm.cell_z(row, col),
        },
    );
    world.add_component(tile_entity, Sprite::textured(tm.atlas.texture.as_str()));
    world.add_component(tile_entity, uv);
    if let Some(cell) = anim_cell {
        world.add_component(tile_entity, cell);
    }
    tile_entity
}

/// Builds an [`AnimatedTileCell`] for `tile_value` if the tilemap entity has a
/// `TileAnimationSet` containing that value.  Returns `None` for non-animated values.
fn make_anim_cell(
    world: &World,
    map_entity: Entity,
    tile_value: u32,
    tm: &Tilemap,
    row: usize,
    col: usize,
) -> Option<AnimatedTileCell> {
    let anim_set = world.get::<TileAnimationSet>(map_entity)?;
    let anim = anim_set.get(tile_value)?;
    if anim.frames.is_empty() {
        return None;
    }
    let frame_uvs: Vec<UvRect> = anim.frames.iter().map(|&id| tm.atlas.uv_for(id)).collect();
    // Phase offset based on cell position so neighbouring cells don't sync-flash. The stagger
    // factor is configurable on the TileAnimationSet (0.0 = synchronized lockstep).
    let phase = super::animation::stagger_phase(
        row,
        col,
        anim.frame_time,
        anim.total_time(),
        anim_set.stagger,
    );
    Some(AnimatedTileCell::new(frame_uvs, anim.frame_time, phase))
}

impl System for TilemapSystem {
    // `clippy::map_entry` is suppressed here because the "first-time" branch
    // calls `world.spawn`/`world.add_component` (mutable World borrows) between
    // the `contains_key` check and the eventual `self.views.insert`.  The entry
    // API would require holding a mutable borrow on `self.views` across those
    // world mutations, which the borrow checker disallows.
    #[allow(clippy::map_entry)]
    fn run(&mut self, world: &mut World, _dt: f32) {
        // ── Step 1: collect alive tilemap entities (avoids holding borrow) ──────
        let tilemap_entities: Vec<Entity> = world.query::<Tilemap>().map(|(e, _)| e).collect();

        // ── Step 2: despawn views for disappeared tilemap entities ────────────
        // Build a HashSet for O(1) membership checks instead of O(M×N) Vec::contains.
        let alive_set: HashSet<Entity> = tilemap_entities.iter().copied().collect();
        let removed: Vec<Entity> = self
            .views
            .keys()
            .filter(|e| !alive_set.contains(e))
            .copied()
            .collect();
        for map_entity in removed {
            if let Some(view) = self.views.remove(&map_entity) {
                for (_, tile_e) in view.cells {
                    world.despawn(tile_e);
                }
            }
        }

        // ── Step 3: process each alive tilemap entity ──────────────────────────
        for map_entity in tilemap_entities {
            // Clone out the data we need before any mutation. The optional `TilemapAutotile`
            // (single- or multi-terrain via its `mode`) drives display-UV selection.
            // Check the CHEAP fields first and bail before cloning anything. The
            // generation/dims fast path below already skipped the diff on an unchanged tilemap,
            // but it sat *after* this deep clone of the whole tile grid — so an idle tilemap
            // still copied every cell, every frame, before deciding there was nothing to do.
            // On a large map that is the single biggest per-frame allocation in the engine.
            {
                let Some(tm) = world.get::<Tilemap>(map_entity) else {
                    continue;
                };
                if let Some(view) = self.views.get(&map_entity) {
                    if tm.generation == view.cached_generation && tm.dims() == view.cached_dims {
                        continue;
                    }
                }
            }

            let (tm_clone, autotile) = {
                // The entity was alive when collected, but a script/coroutine could despawn it
                // mid-frame; skip instead of panicking (release builds abort on panic).
                let Some(tm) = world.get::<Tilemap>(map_entity) else {
                    continue;
                };
                let at = world.get::<TilemapAutotile>(map_entity).cloned();
                (tm.clone(), at)
            };

            // Whether autotiling is active (controls neighbor UV refresh on edits).
            let any_autotile = autotile.is_some();

            // Helper: resolve UV for a non-zero cell.
            let resolve_uv = |row: usize, col: usize, value: u32| -> UvRect {
                let Some(at) = &autotile else {
                    return tm_clone.atlas.uv_for(value - 1);
                };
                match &at.mode {
                    AutotileMode::Single { mask_to_tile } => {
                        let mask = compute_tile_mask(
                            &tm_clone.tiles,
                            row,
                            col,
                            at.neighborhood,
                            at.oob_filled,
                        );
                        let display_id = mask_to_tile.get(&mask).copied().unwrap_or(0);
                        tm_clone.atlas.uv_for(display_id)
                    }
                    AutotileMode::Multi { .. } => {
                        if let Some(rule) = at.rule_for(value) {
                            let mask = compute_tile_mask_typed(
                                &tm_clone.tiles,
                                row,
                                col,
                                at.neighborhood,
                                at.oob_filled,
                                value,
                            );
                            let display_id = rule.mask_to_tile.get(&mask).copied().unwrap_or(0);
                            tm_clone.atlas.uv_for(display_id)
                        } else {
                            // No rule for this terrain value: plain non-autotiled mapping.
                            tm_clone.atlas.uv_for(value - 1)
                        }
                    }
                }
            };

            if !self.views.contains_key(&map_entity) {
                // ── First time seen: full build ────────────────────────────────
                let dims = tm_clone.dims();
                let mut cells: HashMap<(usize, usize), Entity> = HashMap::new();

                for (row_idx, row) in tm_clone.tiles.iter().enumerate() {
                    for (col_idx, &tile_value) in row.iter().enumerate() {
                        if tile_value == 0 {
                            continue;
                        }
                        let uv = resolve_uv(row_idx, col_idx, tile_value);
                        let anim_cell = make_anim_cell(
                            world, map_entity, tile_value, &tm_clone, row_idx, col_idx,
                        );
                        let tile_e =
                            spawn_tile_entity(world, &tm_clone, row_idx, col_idx, uv, anim_cell);
                        cells.insert((row_idx, col_idx), tile_e);
                    }
                }

                self.views.insert(
                    map_entity,
                    TilemapView {
                        cells,
                        cached_tiles: tm_clone.tiles.clone(),
                        cached_dims: dims,
                        cached_generation: tm_clone.generation,
                    },
                );
            } else {
                // ── Already seen: diff and react ───────────────────────────────
                let current_dims = tm_clone.dims();
                let cached_dims = self.views[&map_entity].cached_dims;

                // Fast path: generation unchanged and dims identical → nothing mutated.
                if tm_clone.generation == self.views[&map_entity].cached_generation
                    && current_dims == cached_dims
                {
                    continue;
                }

                if current_dims != cached_dims {
                    // Dimension changed — full rebuild.
                    let old_cells: Vec<Entity> = self
                        .views
                        .get_mut(&map_entity)
                        .unwrap()
                        .cells
                        .drain()
                        .map(|(_, e)| e)
                        .collect();
                    for e in old_cells {
                        world.despawn(e);
                    }

                    let mut cells: HashMap<(usize, usize), Entity> = HashMap::new();
                    for (row_idx, row) in tm_clone.tiles.iter().enumerate() {
                        for (col_idx, &tile_value) in row.iter().enumerate() {
                            if tile_value == 0 {
                                continue;
                            }
                            let uv = resolve_uv(row_idx, col_idx, tile_value);
                            let anim_cell = make_anim_cell(
                                world, map_entity, tile_value, &tm_clone, row_idx, col_idx,
                            );
                            let tile_e = spawn_tile_entity(
                                world, &tm_clone, row_idx, col_idx, uv, anim_cell,
                            );
                            cells.insert((row_idx, col_idx), tile_e);
                        }
                    }

                    let view = self.views.get_mut(&map_entity).unwrap();
                    view.cells = cells;
                    view.cached_tiles = tm_clone.tiles.clone();
                    view.cached_dims = current_dims;
                    view.cached_generation = tm_clone.generation;
                } else {
                    // Same dimensions: cell-level diff.
                    // Collect changed cells first (borrow checker: we need view after).
                    let cached_clone = self.views[&map_entity].cached_tiles.clone();

                    // Find changed cells.
                    let mut changed_cells: Vec<(usize, usize, u32, u32)> = Vec::new(); // (row, col, old, new)
                    for (r, row) in tm_clone.tiles.iter().enumerate() {
                        for (c, &new_val) in row.iter().enumerate() {
                            let old_val = cached_clone
                                .get(r)
                                .and_then(|rr| rr.get(c))
                                .copied()
                                .unwrap_or(0);
                            if old_val != new_val {
                                changed_cells.push((r, c, old_val, new_val));
                            }
                        }
                    }

                    if changed_cells.is_empty() {
                        continue;
                    }

                    // Determine which cells need a UV refresh (changed + neighbors when autotile is active).
                    // HashSet deduplicates neighbor cells that multiple changed tiles share.
                    let mut uv_refresh: HashSet<(usize, usize)> = HashSet::new();

                    for &(r, c, old_val, new_val) in &changed_cells {
                        let view = self.views.get_mut(&map_entity).unwrap();

                        if old_val != 0 && new_val == 0 {
                            // Non-zero → zero: despawn tile entity.
                            if let Some(tile_e) = view.cells.remove(&(r, c)) {
                                world.despawn(tile_e);
                            }
                        } else if old_val == 0 && new_val != 0 {
                            // Zero → non-zero: spawn new tile entity.
                            let uv = resolve_uv(r, c, new_val);
                            let anim_cell =
                                make_anim_cell(world, map_entity, new_val, &tm_clone, r, c);
                            let tile_e = spawn_tile_entity(world, &tm_clone, r, c, uv, anim_cell);
                            view.cells.insert((r, c), tile_e);
                        } else if old_val != 0 && new_val != 0 {
                            // Non-zero → different non-zero: update UvRect in place and
                            // refresh the AnimatedTileCell tag (remove old, add new if animated).
                            let uv = resolve_uv(r, c, new_val);
                            if let Some(&tile_e) = view.cells.get(&(r, c)) {
                                if let Some(uv_comp) = world.get_mut::<UvRect>(tile_e) {
                                    *uv_comp = uv;
                                }
                                // Refresh animation tag: remove stale tag first, then add if
                                // new value is animated (handles animated→static and vice-versa).
                                world.remove_component::<AnimatedTileCell>(tile_e);
                                if let Some(anim_cell) =
                                    make_anim_cell(world, map_entity, new_val, &tm_clone, r, c)
                                {
                                    world.add_component(tile_e, anim_cell);
                                }
                            } else {
                                // Safety spawn (shouldn't happen in normal flow).
                                let anim_cell =
                                    make_anim_cell(world, map_entity, new_val, &tm_clone, r, c);
                                let tile_e =
                                    spawn_tile_entity(world, &tm_clone, r, c, uv, anim_cell);
                                view.cells.insert((r, c), tile_e);
                            }
                        }

                        // When autotile is active, neighbors also need UV refresh.
                        if any_autotile {
                            let (rows, cols) = current_dims;
                            for dr in -1i32..=1 {
                                for dc in -1i32..=1 {
                                    if dr == 0 && dc == 0 {
                                        continue;
                                    }
                                    let nr = r as i32 + dr;
                                    let nc = c as i32 + dc;
                                    if nr >= 0 && nr < rows as i32 && nc >= 0 && nc < cols as i32 {
                                        uv_refresh.insert((nr as usize, nc as usize));
                                    }
                                }
                            }
                        }
                    }

                    // Apply neighbor UV refreshes (only for non-zero cells that have entities).
                    for (nr, nc) in uv_refresh {
                        let current_val = tm_clone
                            .tiles
                            .get(nr)
                            .and_then(|r| r.get(nc))
                            .copied()
                            .unwrap_or(0);
                        if current_val == 0 {
                            continue;
                        }
                        let uv = resolve_uv(nr, nc, current_val);
                        let view = self.views.get_mut(&map_entity).unwrap();
                        if let Some(&tile_e) = view.cells.get(&(nr, nc)) {
                            if let Some(uv_comp) = world.get_mut::<UvRect>(tile_e) {
                                *uv_comp = uv;
                            }
                        }
                    }

                    // Update cached tiles.
                    let view = self.views.get_mut(&map_entity).unwrap();
                    view.cached_tiles = tm_clone.tiles.clone();
                    view.cached_dims = current_dims;
                    view.cached_generation = tm_clone.generation;
                }
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;
    use crate::tilemap::{Tilemap, TilemapAtlas};
    use glam::Vec2;

    fn make_atlas() -> TilemapAtlas {
        TilemapAtlas::new("tiles.png", 4, 4) // 16 tiles in a 4×4 grid
    }

    fn make_tilemap(tiles: Vec<Vec<u32>>) -> Tilemap {
        Tilemap::new(make_atlas(), tiles, 32.0, Vec2::ZERO)
    }

    fn run_tilemap_system(world: &mut World, sys: &mut TilemapSystem) {
        sys.run(world, 0.0);
    }

    #[test]
    fn tilemap_system_spawns_one_entity_per_non_zero_cell() {
        let mut world = World::new();
        let mut sys = TilemapSystem::new();

        // 2×2 map: 3 non-zero cells
        let map_e = world.spawn();
        world.add_component(map_e, make_tilemap(vec![vec![1, 0], vec![2, 3]]));
        run_tilemap_system(&mut world, &mut sys);

        let count = world.query::<UvRect>().count();
        assert_eq!(
            count, 3,
            "should spawn 3 tile entities for 3 non-zero cells"
        );
    }

    #[test]
    fn tilemap_system_reactive_despawn_on_zero() {
        let mut world = World::new();
        let mut sys = TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, make_tilemap(vec![vec![1, 2], vec![3, 4]]));
        run_tilemap_system(&mut world, &mut sys);
        assert_eq!(world.query::<UvRect>().count(), 4, "initial: 4 tiles");

        // Erase cell (0,0)
        world.get_mut::<Tilemap>(map_e).unwrap().set_tile(0, 0, 0);
        run_tilemap_system(&mut world, &mut sys);
        assert_eq!(world.query::<UvRect>().count(), 3, "after erase: 3 tiles");
    }

    #[test]
    fn tilemap_system_reactive_spawn_on_fill() {
        let mut world = World::new();
        let mut sys = TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, make_tilemap(vec![vec![1, 0], vec![0, 0]]));
        run_tilemap_system(&mut world, &mut sys);
        assert_eq!(world.query::<UvRect>().count(), 1, "initial: 1 tile");

        // Fill cell (1,1)
        world.get_mut::<Tilemap>(map_e).unwrap().set_tile(1, 1, 2);
        run_tilemap_system(&mut world, &mut sys);
        assert_eq!(world.query::<UvRect>().count(), 2, "after fill: 2 tiles");
    }

    /// TilemapSystem must not re-process a map that has not been mutated.
    /// After initial spawn, run the system once more with no mutation and verify
    /// the tile entity count stays identical (no duplicate spawns or despawns).
    #[test]
    fn tilemap_system_no_work_when_generation_unchanged() {
        let mut world = World::new();
        let mut sys = TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, make_tilemap(vec![vec![1, 2], vec![3, 4]]));
        sys.run(&mut world, 0.0); // initial build: 4 tiles
        assert_eq!(world.query::<UvRect>().count(), 4);

        // Run again without any mutation — generation is unchanged.
        sys.run(&mut world, 0.0);
        assert_eq!(
            world.query::<UvRect>().count(),
            4,
            "re-running with no mutation must not change the tile entity count"
        );
    }
}
