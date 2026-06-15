use std::collections::{HashMap, HashSet};

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
    /// [`PhysicsWorld::add_static_from_tilemap`]. For a tilemap you intend to
    /// mutate at runtime, do the **initial** build through this method (with a
    /// fresh [`TileColliderIndex`]) rather than `add_static_from_tilemap` — that
    /// way the index tracks every collider and later syncs can remove the exact
    /// cell that was dug. Do not mix the two on the same tiles: colliders created
    /// by `add_static_from_tilemap` are untracked, so a later sync would add a
    /// second, overlapping collider and the original would never be removed.
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

// ── TilemapColliders component ─────────────────────────────────────────────────

/// Which tile ids become solid box colliders when a [`TilemapColliders`] entity is synced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolidTiles {
    /// Any non-zero tile id is solid (the common "0 = empty, anything else = wall" convention).
    NonZero,
    /// Only these specific tile ids are solid; everything else is empty.
    ///
    /// Stored as a [`HashSet`] for O(1) membership tests — prefer the [`SolidTiles::only`]
    /// constructor over constructing this variant directly.
    Only(HashSet<u32>),
}

impl SolidTiles {
    /// Builds a `SolidTiles::Only` from any iterable of tile ids.
    ///
    /// ```rust,no_run
    /// use engine::SolidTiles;
    /// let solid = SolidTiles::only([1, 2, 3]);
    /// ```
    pub fn only(ids: impl IntoIterator<Item = u32>) -> Self {
        SolidTiles::Only(ids.into_iter().collect())
    }

    /// The `collider_for` rule this variant represents: solid tiles → [`TileCollider::solid`].
    pub fn collider_for(&self, tile_id: u32) -> Option<TileCollider> {
        let solid = match self {
            SolidTiles::NonZero => tile_id != 0,
            SolidTiles::Only(ids) => ids.contains(&tile_id),
        };
        solid.then(TileCollider::solid)
    }
}

/// Opt-in component: attach to a [`Tilemap`] entity to keep its static tile colliders in the
/// [`PhysicsWorld`] resource in sync when the tilemap is mutated — by the editor's Tile Paint or a
/// game's runtime `set_tile`. Without it, tilemap edits are visual-only (no collider change).
///
/// Carries the `pixels_per_unit` + solid-tile rule the generic sync needs (which the editor can't
/// infer) plus the persistent [`TileColliderIndex`] that makes resyncs incremental. Build the initial
/// colliders through [`sync`](Self::sync) (a fresh index does a full build) — do **not** also call
/// [`PhysicsWorld::add_static_from_tilemap`] on the same tiles, or you get untracked duplicates.
///
/// # Cleanup — call `drain_into_physics` before despawning
///
/// [`World::despawn`] has no lifecycle hook, so **the rapier bodies owned by this component are
/// NOT removed automatically when the entity is despawned**. Failing to clean up leaks every
/// `(BodyHandle, ColliderHandle)` tracked by the internal index. Always call
/// [`drain_into_physics`](Self::drain_into_physics) before (or instead of) despawning the entity:
///
/// ```rust,no_run
/// # use engine::{TilemapColliders, PhysicsWorld};
/// # use engine::ecs::World;
/// # let mut world = World::new();
/// # let e = world.spawn();
/// # world.insert_resource(PhysicsWorld::new(glam::Vec2::ZERO));
/// world.with_resource_mut::<PhysicsWorld, _>(|physics, world| {
///     if let Some(tc) = world.get_mut::<TilemapColliders>(e) {
///         tc.drain_into_physics(physics);
///     }
/// });
/// world.despawn(e);
/// ```
///
/// # Example
/// ```rust,no_run
/// use engine::{TilemapColliders, SolidTiles};
/// # use engine::{App, Tilemap, TilemapAtlas, PhysicsWorld};
/// # use glam::Vec2;
/// let mut app = App::new();
/// # let tm = Tilemap::new(TilemapAtlas::new("t.png", 1, 1), vec![vec![1]], 32.0, Vec2::ZERO);
/// let e = app.world.spawn();
/// app.world.add_component(e, tm);
/// app.world.add_component(e, TilemapColliders::new(32.0, SolidTiles::NonZero));
/// app.world.insert_resource(PhysicsWorld::new(Vec2::ZERO));
/// // after any set_tile / paint:
/// app.sync_tilemap_colliders(e);
/// ```
#[derive(Debug)]
pub struct TilemapColliders {
    /// Scale converting world (pixel) coordinates to physics (meter) units. Must match the
    /// `pixels_per_unit` the game's `PhysicsSystem` uses.
    pub pixels_per_unit: f32,
    /// Which tile ids become solid colliders.
    pub solid: SolidTiles,
    /// Tracks created colliders so resyncs only touch changed cells. Not serialized.
    index: TileColliderIndex,
}

impl TilemapColliders {
    /// New collider config with an empty index (first [`sync`](Self::sync) does a full build).
    pub fn new(pixels_per_unit: f32, solid: SolidTiles) -> Self {
        Self {
            pixels_per_unit,
            solid,
            index: TileColliderIndex::new(),
        }
    }

    /// Sync this tilemap's colliders into `physics` (incremental add/remove of changed cells).
    /// Call after the `tilemap` mutates.
    pub fn sync(&mut self, physics: &mut PhysicsWorld, tilemap: &Tilemap) {
        physics.sync_static_from_tilemap(
            tilemap,
            self.pixels_per_unit,
            |id| self.solid.collider_for(id),
            &mut self.index,
        );
    }

    /// Number of tile colliders currently tracked.
    pub fn collider_count(&self) -> usize {
        self.index.len()
    }

    /// Removes every rapier body tracked by this component from `physics` and clears the
    /// internal index.
    ///
    /// **Must be called before despawning the entity** — [`World::despawn`] has no lifecycle
    /// hook, so bodies are not cleaned up automatically. After this call the component holds an
    /// empty index; calling [`sync`](Self::sync) again will perform a full rebuild.
    pub fn drain_into_physics(&mut self, physics: &mut PhysicsWorld) {
        for (_key, (body_handle, collider_handle)) in self.index.cells.drain() {
            let body = PhysicsBody {
                rigid_body_handle: body_handle,
                collider_handle,
            };
            physics.remove_body(&body);
        }
    }
}

/// Resync the static tile colliders of `entity` against the world's [`PhysicsWorld`] resource.
///
/// No-op (returns `false`) unless `entity` has both a [`Tilemap`] and a [`TilemapColliders`] and the
/// world holds a [`PhysicsWorld`] resource. Usable from any system with `&mut World`; the editor calls
/// it after a paint stroke (and on undo/redo) so colliders track the tiles. Returns `true` if a
/// `PhysicsWorld` was present (the sync ran).
pub fn sync_tilemap_entity_colliders(
    world: &mut crate::ecs::World,
    entity: crate::ecs::Entity,
) -> bool {
    // Clone the tilemap out first so the immutable component borrow is released before we take the
    // mutable PhysicsWorld resource + the mutable TilemapColliders component.
    let Some(tilemap) = world.get::<Tilemap>(entity).cloned() else {
        return false;
    };
    world.with_resource_mut::<PhysicsWorld, _>(|physics, world| {
        if let Some(colliders) = world.get_mut::<TilemapColliders>(entity) {
            colliders.sync(physics, &tilemap);
        }
    })
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

    // ── TilemapColliders component ────────────────────────────────────────────

    #[test]
    fn solid_tiles_rules() {
        assert!(SolidTiles::NonZero.collider_for(1).is_some());
        assert!(SolidTiles::NonZero.collider_for(0).is_none());
        let only = SolidTiles::only([2, 3]);
        assert!(only.collider_for(2).is_some());
        assert!(only.collider_for(3).is_some());
        assert!(only.collider_for(1).is_none());
        assert!(only.collider_for(0).is_none());
    }

    /// `SolidTiles::only` constructor — a tile in the set is solid, one not in it is not.
    #[test]
    fn solid_tiles_only_constructor_membership() {
        let solid = SolidTiles::only([10u32, 20, 30]);
        // Members → solid.
        assert!(
            solid.collider_for(10).is_some(),
            "tile 10 should be solid"
        );
        assert!(
            solid.collider_for(20).is_some(),
            "tile 20 should be solid"
        );
        assert!(
            solid.collider_for(30).is_some(),
            "tile 30 should be solid"
        );
        // Non-members → empty.
        assert!(
            solid.collider_for(0).is_none(),
            "tile 0 should not be solid"
        );
        assert!(
            solid.collider_for(11).is_none(),
            "tile 11 should not be solid"
        );
        // Clone equivalence — HashSet preserves equality.
        assert_eq!(solid, SolidTiles::only([30u32, 20, 10]));
    }

    #[test]
    fn tilemap_colliders_sync_adds_and_removes() {
        let mut tm = solid_3x3();
        let mut physics = make_world();
        let mut tc = TilemapColliders::new(32.0, SolidTiles::NonZero);

        tc.sync(&mut physics, &tm);
        assert_eq!(tc.collider_count(), 9, "full build of a 3×3 solid map");

        // Erase the center cell → one collider removed.
        tm.set_tile(1, 1, 0);
        tc.sync(&mut physics, &tm);
        assert_eq!(
            tc.collider_count(),
            8,
            "erasing a cell removes its collider"
        );

        // Paint it back → collider restored.
        tm.set_tile(1, 1, 1);
        tc.sync(&mut physics, &tm);
        assert_eq!(tc.collider_count(), 9, "repainting restores the collider");
    }

    #[test]
    fn sync_tilemap_entity_colliders_free_fn() {
        use crate::ecs::World;
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, solid_3x3());
        world.add_component(e, TilemapColliders::new(32.0, SolidTiles::NonZero));

        // No PhysicsWorld resource → no-op, returns false.
        assert!(!sync_tilemap_entity_colliders(&mut world, e));

        // With a PhysicsWorld resource → syncs, returns true.
        world.insert_resource(make_world());
        assert!(sync_tilemap_entity_colliders(&mut world, e));
        assert_eq!(
            world.get::<TilemapColliders>(e).unwrap().collider_count(),
            9
        );
    }

    #[test]
    fn sync_tilemap_entity_colliders_missing_component_is_false_path() {
        use crate::ecs::World;
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, solid_3x3());
        world.insert_resource(make_world());
        // Has PhysicsWorld + Tilemap but NO TilemapColliders → with_resource_mut runs (true) but
        // nothing is synced; the entity gains no collider tracking.
        let ran = sync_tilemap_entity_colliders(&mut world, e);
        assert!(
            ran,
            "PhysicsWorld present so with_resource_mut returns true"
        );
        assert!(world.get::<TilemapColliders>(e).is_none());
    }

    // ── Task 6: drain_into_physics removes bodies from physics ────────────────

    /// Build colliders for a 3×3 tilemap, call `drain_into_physics`, and assert that the
    /// physics body count returns to the baseline (no bodies leaked).
    #[test]
    fn drain_into_physics_removes_all_bodies() {
        let tilemap = solid_3x3();
        let mut physics = make_world();
        let baseline = physics.rigid_body_set.len();

        let mut tc = TilemapColliders::new(32.0, SolidTiles::NonZero);
        tc.sync(&mut physics, &tilemap);

        assert_eq!(
            physics.rigid_body_set.len(),
            baseline + 9,
            "sync should have added 9 bodies"
        );

        tc.drain_into_physics(&mut physics);

        assert_eq!(
            physics.rigid_body_set.len(),
            baseline,
            "after drain, body count must return to baseline (no leak)"
        );
        assert_eq!(
            tc.collider_count(),
            0,
            "index must be empty after drain"
        );
    }

    /// A second `sync` after `drain_into_physics` must rebuild the full set of colliders.
    #[test]
    fn drain_then_resync_rebuilds() {
        let tilemap = solid_3x3();
        let mut physics = make_world();
        let mut tc = TilemapColliders::new(32.0, SolidTiles::NonZero);

        tc.sync(&mut physics, &tilemap);
        assert_eq!(tc.collider_count(), 9);

        tc.drain_into_physics(&mut physics);
        assert_eq!(tc.collider_count(), 0);

        // A fresh sync must treat the empty index as a full build.
        tc.sync(&mut physics, &tilemap);
        assert_eq!(tc.collider_count(), 9, "resync after drain must rebuild 9 colliders");
    }
}
