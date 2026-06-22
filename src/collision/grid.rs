use glam::Vec2;
use std::collections::HashMap;

use crate::components::Transform;
use crate::ecs::{Entity, System, World};

// ─── Collider ────────────────────────────────────────────────────────────────

/// Collision shape component attached to an entity
#[derive(Debug, Clone, Copy)]
pub enum Collider {
    Circle { radius: f32 },
    Aabb { half_extents: Vec2 },
}

impl Collider {
    /// AABB (min, max) occupied by this collider — used for grid cell indexing
    pub fn aabb(&self, center: Vec2) -> (Vec2, Vec2) {
        match self {
            Collider::Circle { radius } => (
                Vec2::new(center.x - radius, center.y - radius),
                Vec2::new(center.x + radius, center.y + radius),
            ),
            Collider::Aabb { half_extents } => (center - *half_extents, center + *half_extents),
        }
    }
}

// ─── CollisionLayer ──────────────────────────────────────────────────────────

/// Bitmask collision layer. If `self & mask == 0`, the collision is ignored.
///
/// Example:
/// ```rust
/// const LAYER_PLAYER: u32 = 1 << 0;
/// const LAYER_ENEMY:  u32 = 1 << 1;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionLayer(pub u32);

impl CollisionLayer {
    pub const ALL: Self = Self(u32::MAX);
    pub const NONE: Self = Self(0);

    /// Returns `true` if this layer and `mask` share at least one bit
    pub fn matches(&self, mask: CollisionLayer) -> bool {
        (self.0 & mask.0) != 0
    }
}

// ─── SpatialGrid ─────────────────────────────────────────────────────────────

/// Per-entity grid entry — used during queries
#[derive(Debug, Clone, Copy)]
pub struct GridEntry {
    pub center: Vec2,
    pub collider: Collider,
    pub layer: CollisionLayer,
}

/// Spatial hash grid.
///
/// Rebuilt every frame via `rebuild`; queried via `query_radius` / `query_aabb`.
/// Default cell size: 128 pixels.
///
/// `CollisionGridSystem` mirrors the rebuilt grid into the World as a resource each
/// frame, so external systems can issue read-only queries via
/// `world.resource::<SpatialGrid>()`.
#[derive(Debug, Clone)]
pub struct SpatialGrid {
    /// Cell side length (pixels)
    pub cell: f32,
    /// (col, row) → list of entities overlapping that cell
    pub buckets: HashMap<(i32, i32), Vec<Entity>>,
    /// Cache so center/collider/layer don't need to be re-read during queries
    pub entries: HashMap<Entity, GridEntry>,
}

impl SpatialGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell: cell_size,
            buckets: HashMap::new(),
            entries: HashMap::new(),
        }
    }

    /// Clears internal state.
    pub fn clear(&mut self) {
        self.buckets.clear();
        self.entries.clear();
    }

    /// Reads all entities with `(Transform, Collider)` from the World and rebuilds the grid.
    ///
    /// Entities without a `CollisionLayer` component are treated as `CollisionLayer::ALL`.
    pub fn rebuild(&mut self, world: &World) {
        self.clear();

        for (entity, transform, collider) in world.query2::<Transform, Collider>() {
            let center = transform.position;
            let layer = world
                .get::<CollisionLayer>(entity)
                .copied()
                .unwrap_or(CollisionLayer::ALL);

            // Compute cell index range
            let (aabb_min, aabb_max) = collider.aabb(center);
            let col_min = (aabb_min.x / self.cell).floor() as i32;
            let col_max = (aabb_max.x / self.cell).floor() as i32;
            let row_min = (aabb_min.y / self.cell).floor() as i32;
            let row_max = (aabb_max.y / self.cell).floor() as i32;

            for col in col_min..=col_max {
                for row in row_min..=row_max {
                    self.buckets.entry((col, row)).or_default().push(entity);
                }
            }

            self.entries.insert(
                entity,
                GridEntry {
                    center,
                    collider: *collider,
                    layer,
                },
            );
        }
    }

    /// Returns the cell key (col, row) for the given world coordinates.
    fn cell_key(&self, x: f32, y: f32) -> (i32, i32) {
        (
            (x / self.cell).floor() as i32,
            (y / self.cell).floor() as i32,
        )
    }

    /// Collects unique candidate entities from all cells that overlap the given AABB.
    pub(crate) fn candidates_in_aabb(&self, min: Vec2, max: Vec2) -> Vec<Entity> {
        let (col_min, row_min) = self.cell_key(min.x, min.y);
        let (col_max, row_max) = self.cell_key(max.x, max.y);

        // Single-cell fast path: an entity appears at most once in any one bucket, so no
        // cross-cell deduplication is needed — skip the per-query `HashSet` allocation. This is
        // the common case for small colliders whose AABB fits inside one grid cell.
        if col_min == col_max && row_min == row_max {
            return self
                .buckets
                .get(&(col_min, row_min))
                .cloned()
                .unwrap_or_default();
        }

        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for col in col_min..=col_max {
            for row in row_min..=row_max {
                if let Some(bucket) = self.buckets.get(&(col, row)) {
                    for &entity in bucket {
                        if seen.insert(entity) {
                            result.push(entity);
                        }
                    }
                }
            }
        }
        result
    }
}

// ─── CollisionGridSystem ──────────────────────────────────────────────────────

/// Rebuilds the `SpatialGrid` World resource every frame.
///
/// Mirrors the [`crate::physics::PhysicsSystem`] / `PhysicsWorld` pattern: the
/// grid lives in the World as a resource, and this system *moves* it out, rebuilds
/// it in place, and moves it back each frame (remove → rebuild → insert). Because
/// `rebuild` only `clear()`s and refills the existing `HashMap`s, their allocations
/// are reused across frames and no per-frame deep clone is made.
///
/// After this system runs, any later system can read the grid with
/// `world.resource::<SpatialGrid>()` and call `query_radius` / `query_aabb`.
///
/// ```ignore
/// app.add_system(CollisionGridSystem::new(128.0));
/// // later systems: if let Some(grid) = world.resource::<SpatialGrid>() { grid.query_radius(..) }
/// ```
pub struct CollisionGridSystem {
    cell_size: f32,
}

impl CollisionGridSystem {
    pub fn new(cell_size: f32) -> Self {
        Self { cell_size }
    }
}

impl CollisionGridSystem {
    /// Schedule label. Systems reading the mirrored `SpatialGrid` resource should
    /// declare `.after(CollisionGridSystem::LABEL)`.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::collision_grid";
}

impl System for CollisionGridSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // Take last frame's grid (keeping its allocated buckets) or make a fresh
        // one on the first frame, rebuild in place, then move it back. No clone.
        let mut grid = world
            .remove_resource::<SpatialGrid>()
            .unwrap_or_else(|| SpatialGrid::new(self.cell_size));
        grid.rebuild(world);
        world.insert_resource(grid);
    }
}

// ─── Unit tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;

    fn make_world_with_circle(pos: Vec2, radius: f32) -> (World, Entity) {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: pos,
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 0.0,
            },
        );
        world.add_component(e, Collider::Circle { radius });
        (world, e)
    }

    /// `query_radius` on an empty grid must return an empty Vec.
    #[test]
    fn empty_grid_returns_empty_query() {
        let grid = SpatialGrid::new(128.0);
        let result = grid.query_radius(Vec2::ZERO, 100.0, CollisionLayer::ALL);
        assert!(result.is_empty());
    }

    /// A single Circle collider within the radius must be detected.
    #[test]
    fn single_circle_in_radius() {
        let (world, e) = make_world_with_circle(Vec2::new(50.0, 50.0), 16.0);
        let mut grid = SpatialGrid::new(128.0);
        grid.rebuild(&world);

        // distance from query center = 0, radius 200 → must be detected
        let result = grid.query_radius(Vec2::new(50.0, 50.0), 200.0, CollisionLayer::ALL);
        assert!(result.contains(&e), "entity should be within the radius");
    }

    /// Entities whose layer ANDs to 0 with the mask must be excluded.
    #[test]
    fn layer_mask_filters_results() {
        let mut world = World::new();

        // LAYER_A entity
        let e_a = world.spawn();
        world.add_component(
            e_a,
            Transform {
                position: Vec2::new(10.0, 10.0),
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 0.0,
            },
        );
        world.add_component(e_a, Collider::Circle { radius: 8.0 });
        world.add_component(e_a, CollisionLayer(1 << 0)); // bit 0

        // LAYER_B entity
        let e_b = world.spawn();
        world.add_component(
            e_b,
            Transform {
                position: Vec2::new(20.0, 10.0),
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 0.0,
            },
        );
        world.add_component(e_b, Collider::Circle { radius: 8.0 });
        world.add_component(e_b, CollisionLayer(1 << 1)); // bit 1

        let mut grid = SpatialGrid::new(128.0);
        grid.rebuild(&world);

        // bit 0 mask → only e_a detected, e_b excluded
        let result = grid.query_radius(Vec2::ZERO, 500.0, CollisionLayer(1 << 0));
        assert!(result.contains(&e_a), "e_a should match the mask");
        assert!(!result.contains(&e_b), "e_b should not match the mask");
    }

    /// `CollisionGridSystem` must mirror the rebuilt grid into the World resource
    /// (remove→rebuild→insert pattern, no deep clone).
    #[test]
    fn grid_system_mirrors_grid_to_resource() {
        let (mut world, e) = make_world_with_circle(Vec2::new(50.0, 50.0), 16.0);
        let mut system = CollisionGridSystem::new(128.0);

        // First frame: builds a fresh grid from scratch (no prior resource).
        system.run(&mut world, 0.016);
        let grid = world
            .resource::<SpatialGrid>()
            .expect("grid system should mirror a SpatialGrid resource");
        assert!(
            grid.entries.contains_key(&e),
            "entity should be in the mirror"
        );
        assert!(grid
            .query_radius(Vec2::new(50.0, 50.0), 200.0, CollisionLayer::ALL)
            .contains(&e));

        // Second frame: must reuse the previous frame's grid (remove→rebuild→insert).
        system.run(&mut world, 0.016);
        let grid = world.resource::<SpatialGrid>().unwrap();
        assert!(
            grid.entries.contains_key(&e),
            "mirror should persist after rebuild"
        );

        // After despawn, one more frame should remove the entity from the mirror too.
        world.despawn(e);
        system.run(&mut world, 0.016);
        let grid = world.resource::<SpatialGrid>().unwrap();
        assert!(
            !grid.entries.contains_key(&e),
            "despawned entity should be removed from the mirror"
        );
    }

    /// After despawn and rebuild, the entity must be absent from results.
    #[test]
    fn rebuild_after_despawn() {
        let (mut world, e) = make_world_with_circle(Vec2::new(30.0, 30.0), 10.0);
        let mut grid = SpatialGrid::new(128.0);

        grid.rebuild(&world);
        assert!(!grid
            .query_radius(Vec2::ZERO, 500.0, CollisionLayer::ALL)
            .is_empty());

        // rebuild after despawn
        world.despawn(e);
        grid.rebuild(&world);

        let result = grid.query_radius(Vec2::ZERO, 500.0, CollisionLayer::ALL);
        assert!(
            result.is_empty(),
            "despawned entity should not appear in results"
        );
    }

    /// Regression for `candidates_in_aabb`: a collider larger than a cell is registered in
    /// several buckets, so a multi-cell query must dedup it to a single result, while a
    /// single-cell query (the fast path that skips the dedup `HashSet`) still finds it.
    #[test]
    fn candidates_dedup_across_cells_and_single_cell_fast_path() {
        // Circle r=200 at (100,100) with cell=128 → AABB spans multiple cells (in each bucket).
        let (world, e) = make_world_with_circle(Vec2::new(100.0, 100.0), 200.0);
        let mut grid = SpatialGrid::new(128.0);
        grid.rebuild(&world);

        // Multi-cell query: the dedup path must return the spanning collider exactly once.
        let multi = grid.query_aabb(
            Vec2::new(-200.0, -200.0),
            Vec2::new(400.0, 400.0),
            CollisionLayer::ALL,
        );
        assert_eq!(
            multi.iter().filter(|&&x| x == e).count(),
            1,
            "collider spanning multiple cells must be deduped to a single result"
        );

        // Single-cell query (fast path, no dedup set): still returns the in-cell candidate.
        let single = grid.query_aabb(
            Vec2::new(100.0, 100.0),
            Vec2::new(101.0, 101.0),
            CollisionLayer::ALL,
        );
        assert!(
            single.contains(&e),
            "single-cell fast path must return the in-cell candidate"
        );
    }
}
