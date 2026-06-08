use crate::collision::grid::SpatialGrid;
use crate::ecs::{System, World};
use crate::resources::{DebugDrawQueue, DebugRect};

// ─── Resource ─────────────────────────────────────────────────────────────────

/// Debug rendering configuration resource.
///
/// Activate with `world.insert_resource(DebugConfig { show_colliders: true })`.
#[derive(Debug, Clone, Default)]
pub struct DebugConfig {
    /// When true, visualizes collision areas as translucent rectangles.
    pub show_colliders: bool,
}

// ─── System ───────────────────────────────────────────────────────────────────

/// Collision-debug visualization system.
///
/// When `DebugConfig::show_colliders` is true, draws each collider as a
/// translucent rectangle into the `DebugDrawQueue`.
///
/// If a `CollisionGridSystem` already mirrors a `SpatialGrid` into the World this
/// frame, that grid's entries are reused (no second rebuild). When used
/// standalone (no `CollisionGridSystem` registered), it falls back to rebuilding
/// its own internal grid via `self.grid`.
pub struct CollisionDebugSystem {
    grid: SpatialGrid,
}

impl CollisionDebugSystem {
    pub fn new(cell_size: f32) -> Self {
        Self {
            grid: SpatialGrid::new(cell_size),
        }
    }

    pub fn grid(&self) -> &SpatialGrid {
        &self.grid
    }
}

impl System for CollisionDebugSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let enabled = world
            .resource::<DebugConfig>()
            .map(|c| c.show_colliders)
            .unwrap_or(false);
        if !enabled {
            return;
        }

        // Prefer the grid a `CollisionGridSystem` already mirrored this frame.
        // Only rebuild our own when running standalone (no such system present).
        let build_rects = |grid: &SpatialGrid| -> Vec<DebugRect> {
            grid.entries
                .values()
                .map(|entry| {
                    let (aabb_min, aabb_max) = entry.collider.aabb(entry.center);
                    DebugRect {
                        min: aabb_min,
                        max: aabb_max,
                        color: crate::color::Color::rgba(0.0, 1.0, 0.2, 0.25),
                        z: 999.0,
                    }
                })
                .collect()
        };

        let rects: Vec<DebugRect> = match world.resource::<SpatialGrid>() {
            Some(grid) => build_rects(grid),
            None => {
                self.grid.rebuild(world);
                build_rects(&self.grid)
            }
        };

        if let Some(queue) = world.resource_mut::<DebugDrawQueue>() {
            queue.items.extend(rects);
        }
    }
}
