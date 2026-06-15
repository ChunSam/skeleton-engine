use std::collections::HashMap;

use glam::Vec2;

use crate::physics::body::PhysicsBody;
use crate::tilemap::Tilemap;

use super::{BodyHandle, ColliderHandle, PhysicsWorld, TileCollider};

// ── TileColliderIndex ────────────────────────────────────────────────────────

/// Tracks which tile cell owns which static collider so a mutating tilemap can
/// be re-synced incrementally (add/remove only changed cells) rather than
/// rebuilt from scratch on every change.
///
/// Pass an empty `TileColliderIndex` to
/// [`PhysicsWorld::sync_static_from_tilemap`] on the first call — that
/// performs a full build equivalent to
/// [`PhysicsWorld::add_static_from_tilemap`].
///
/// **Limitation (v1):** when a cell is present in *both* the current index
/// and the new desired state, the existing collider is left untouched even if
/// the `TileCollider` kind (groups, `one_way`) changed.  To change a cell's
/// kind, first clear it (tile id → 0) and sync, then restore the new kind and
/// sync again.
#[derive(Debug, Default)]
pub struct TileColliderIndex {
    // (row, col) -> (BodyHandle, ColliderHandle)
    cells: HashMap<(usize, usize), (BodyHandle, ColliderHandle)>,
}

impl TileColliderIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of tracked collider cells.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Returns `true` if no cells are tracked.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

// ── Per-cell creation helper ─────────────────────────────────────────────────

/// Creates a single static tile collider using the same world-center + half-extent
/// math as `add_static_from_tilemap` so rendered tiles and colliders align exactly.
///
/// Inserts into `one_way_colliders` when `kind.one_way` is set.
/// Returns the `(BodyHandle, ColliderHandle)` pair.
fn create_tile_collider(
    world: &mut PhysicsWorld,
    tilemap: &Tilemap,
    ppu: f32,
    half: f32,
    row_idx: usize,
    col_idx: usize,
    kind: TileCollider,
) -> (BodyHandle, ColliderHandle) {
    let x = tilemap.origin.x + col_idx as f32 * tilemap.tile_size + tilemap.tile_size * 0.5;
    let y = tilemap.origin.y + row_idx as f32 * tilemap.tile_size + tilemap.tile_size * 0.5;
    let pair = world.add_static_box_with_groups(Vec2::new(x, y) / ppu, half, half, kind.groups);
    if kind.one_way {
        world.one_way_colliders.insert(pair.1 .0);
    }
    pair
}

// ── impl PhysicsWorld ────────────────────────────────────────────────────────

impl PhysicsWorld {
    /// Creates a static box collider for each tile in the tilemap.
    ///
    /// World-center coordinates are computed using the same convention as
    /// [`crate::tilemap::TilemapSystem`] when placing tile sprites, so rendered tiles
    /// and colliders align exactly. A collider is created only for tiles where
    /// `collider_for` returns [`Some`]; [`None`] tiles are skipped (empty cells,
    /// decorative tiles, etc.). [`TileCollider`] allows mixing solid and one-way
    /// platform tiles in a single call (one-way tiles are automatically tagged via
    /// [`PhysicsWorld::set_one_way`]).
    ///
    /// `pixels_per_unit` is the scale factor that converts world (pixel) coordinates
    /// to physics (meter) units.
    /// The return value is a list of created `(BodyHandle, ColliderHandle)` pairs
    /// (spawn order: row → column).
    pub fn add_static_from_tilemap(
        &mut self,
        tilemap: &Tilemap,
        pixels_per_unit: f32,
        mut collider_for: impl FnMut(u32) -> Option<TileCollider>,
    ) -> Vec<(BodyHandle, ColliderHandle)> {
        let ppu = pixels_per_unit.max(f32::MIN_POSITIVE);
        let half = (tilemap.tile_size * 0.5) / ppu;
        let mut handles = Vec::new();
        for (row_idx, row) in tilemap.tiles.iter().enumerate() {
            for (col_idx, &tile_id) in row.iter().enumerate() {
                let Some(kind) = collider_for(tile_id) else {
                    continue;
                };
                let pair = create_tile_collider(self, tilemap, ppu, half, row_idx, col_idx, kind);
                handles.push(pair);
            }
        }
        handles
    }

    /// Incrementally syncs tile colliders against `index` so only cells whose
    /// desired collider presence changed are added or removed.
    ///
    /// An empty `index` on the first call performs a full build equivalent to
    /// [`PhysicsWorld::add_static_from_tilemap`].
    ///
    /// `pixels_per_unit` is the scale factor that converts world (pixel)
    /// coordinates to physics (meter) units.
    ///
    /// **Algorithm:**
    /// 1. Compute desired set — every `(row, col)` where
    ///    `collider_for(tile_id)` returns `Some`.
    /// 2. *Removals* — cells in `index` but not in the desired set: the
    ///    collider is removed via [`PhysicsWorld::remove_body`] (which already
    ///    purges `one_way_colliders` and refreshes the query pipeline) and the
    ///    entry is dropped from `index`.
    /// 3. *Additions* — desired cells not yet in `index`: a new static box
    ///    collider is created (same math as `add_static_from_tilemap`, row →
    ///    column order) and stored in `index`.
    /// 4. *Unchanged cells* (present in both index and desired set) are left
    ///    untouched.
    ///
    /// **Limitation (v1):** an existing cell whose `TileCollider` kind
    /// (groups / `one_way`) changes is NOT re-evaluated — only its *presence*
    /// is diffed.  To change a cell's kind, clear it (tile id → 0) and sync,
    /// then restore the new kind and sync again.
    pub fn sync_static_from_tilemap(
        &mut self,
        tilemap: &Tilemap,
        pixels_per_unit: f32,
        mut collider_for: impl FnMut(u32) -> Option<TileCollider>,
        index: &mut TileColliderIndex,
    ) {
        let ppu = pixels_per_unit.max(f32::MIN_POSITIVE);
        let half = (tilemap.tile_size * 0.5) / ppu;

        // 1. Compute desired set.
        let mut desired: HashMap<(usize, usize), TileCollider> = HashMap::new();
        for (row_idx, row) in tilemap.tiles.iter().enumerate() {
            for (col_idx, &tile_id) in row.iter().enumerate() {
                if let Some(kind) = collider_for(tile_id) {
                    desired.insert((row_idx, col_idx), kind);
                }
            }
        }

        // 2. Removals: cells in index but not desired.
        let to_remove: Vec<(usize, usize)> = index
            .cells
            .keys()
            .filter(|key| !desired.contains_key(*key))
            .copied()
            .collect();
        for key in to_remove {
            if let Some((body_handle, collider_handle)) = index.cells.remove(&key) {
                let physics_body = PhysicsBody {
                    rigid_body_handle: body_handle,
                    collider_handle,
                };
                self.remove_body(&physics_body);
            }
        }

        // 3. Additions: desired cells not yet in index (row → col order for stability).
        let mut to_add: Vec<(usize, usize)> = desired
            .keys()
            .filter(|key| !index.cells.contains_key(*key))
            .copied()
            .collect();
        // Sort by row then col for deterministic, stable ordering.
        to_add.sort_unstable();
        for (row_idx, col_idx) in to_add {
            let kind = desired[&(row_idx, col_idx)];
            let pair = create_tile_collider(self, tilemap, ppu, half, row_idx, col_idx, kind);
            index.cells.insert((row_idx, col_idx), pair);
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tilemap::TilemapAtlas;

    fn make_world() -> PhysicsWorld {
        PhysicsWorld::new(glam::Vec2::ZERO)
    }

    /// Build a solid 3×3 tilemap where every cell has tile_id = 1.
    fn solid_3x3() -> Tilemap {
        let atlas = TilemapAtlas::new("tiles.png", 4, 4);
        let tiles = vec![vec![1u32; 3]; 3];
        Tilemap::new(atlas, tiles, 32.0, glam::Vec2::ZERO)
    }

    fn solid_kind(tile_id: u32) -> Option<TileCollider> {
        if tile_id > 0 {
            Some(TileCollider::solid())
        } else {
            None
        }
    }

    // ── 1. Empty index = full build ───────────────────────────────────────────

    #[test]
    fn empty_index_full_build_parity() {
        let tilemap = solid_3x3();
        const PPU: f32 = 32.0;

        // Reference count from add_static_from_tilemap.
        let mut ref_world = make_world();
        let handles = ref_world.add_static_from_tilemap(&tilemap, PPU, solid_kind);
        let expected = handles.len();

        // sync on empty index should produce the same count.
        let mut world = make_world();
        let mut index = TileColliderIndex::new();
        world.sync_static_from_tilemap(&tilemap, PPU, solid_kind, &mut index);

        assert_eq!(
            index.len(),
            expected,
            "sync on empty index must match add_static_from_tilemap count"
        );
        assert_eq!(expected, 9, "3×3 all-solid = 9 colliders");
    }

    // ── 2. Remove one cell ────────────────────────────────────────────────────

    #[test]
    fn remove_one_cell_decrements_index() {
        let mut tilemap = solid_3x3();
        const PPU: f32 = 32.0;

        let mut world = make_world();
        let mut index = TileColliderIndex::new();
        world.sync_static_from_tilemap(&tilemap, PPU, solid_kind, &mut index);
        assert_eq!(index.len(), 9);

        // Clear cell (1, 2) — set tile_id to 0.
        tilemap.tiles[1][2] = 0;
        world.sync_static_from_tilemap(&tilemap, PPU, solid_kind, &mut index);

        assert_eq!(index.len(), 8, "one cell removed → 8 remaining");
        assert!(
            !index.cells.contains_key(&(1, 2)),
            "removed cell key must be gone from index"
        );
    }

    // ── 3. Re-add a cell ──────────────────────────────────────────────────────

    #[test]
    fn re_add_cell_increments_index() {
        let mut tilemap = solid_3x3();
        const PPU: f32 = 32.0;

        let mut world = make_world();
        let mut index = TileColliderIndex::new();

        // Full build, then remove one.
        tilemap.tiles[0][0] = 0;
        world.sync_static_from_tilemap(&tilemap, PPU, solid_kind, &mut index);
        assert_eq!(index.len(), 8);

        // Restore cell (0, 0).
        tilemap.tiles[0][0] = 1;
        world.sync_static_from_tilemap(&tilemap, PPU, solid_kind, &mut index);

        assert_eq!(index.len(), 9, "re-added cell → back to 9");
        assert!(
            index.cells.contains_key(&(0, 0)),
            "re-added cell key must be in index"
        );
    }

    // ── 4. One-way retained in one_way_colliders ──────────────────────────────

    #[test]
    fn one_way_tile_registered_in_world() {
        let atlas = TilemapAtlas::new("tiles.png", 4, 4);
        // tile_id 1 = solid, tile_id 2 = one-way.
        let tiles = vec![vec![1u32, 2u32]];
        let tilemap = Tilemap::new(atlas, tiles, 32.0, glam::Vec2::ZERO);

        let mut world = make_world();
        let mut index = TileColliderIndex::new();
        world.sync_static_from_tilemap(
            &tilemap,
            32.0,
            |id| match id {
                1 => Some(TileCollider::solid()),
                2 => Some(TileCollider::one_way()),
                _ => None,
            },
            &mut index,
        );

        assert_eq!(index.len(), 2, "both tiles should produce colliders");

        // The one-way cell (0, 1) must be flagged in the world.
        let (_, one_way_col) = index.cells[&(0, 1)];
        assert!(
            world.is_one_way(one_way_col),
            "one-way tile collider must be marked in one_way_colliders"
        );
        // Solid cell (0, 0) must NOT be one-way.
        let (_, solid_col) = index.cells[&(0, 0)];
        assert!(
            !world.is_one_way(solid_col),
            "solid tile collider must NOT be marked as one-way"
        );
    }

    // ── 5. No-op sync leaves count unchanged ─────────────────────────────────

    #[test]
    fn noop_sync_leaves_count_unchanged() {
        let tilemap = solid_3x3();
        const PPU: f32 = 32.0;

        let mut world = make_world();
        let mut index = TileColliderIndex::new();

        world.sync_static_from_tilemap(&tilemap, PPU, solid_kind, &mut index);
        let count_after_first = index.len();

        // Sync again without any tilemap change.
        world.sync_static_from_tilemap(&tilemap, PPU, solid_kind, &mut index);

        assert_eq!(
            index.len(),
            count_after_first,
            "no-op sync must not change index length"
        );
    }
}
