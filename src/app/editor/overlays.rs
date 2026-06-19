use crate::app::App;
use crate::ecs::Entity;

impl App {
    /// Draw the debug bounds overlay via `DebugDraw`: every entity's Transform AABB plus any
    /// collision `Collider` shape. Called each frame from the editor UI while `show_bounds` is on.
    pub(in crate::app) fn draw_debug_bounds(&mut self) {
        use crate::collision::Collider;
        // Collect first so the immutable world borrow is released before borrowing DebugDraw.
        let bounds: Vec<(glam::Vec2, glam::Vec2)> = self
            .world
            .query::<crate::components::Transform>()
            .map(|(_, t)| (t.position, t.scale))
            .collect();
        let colliders: Vec<(glam::Vec2, Collider)> = self
            .world
            .query2::<crate::components::Transform, Collider>()
            .map(|(_, t, c)| (t.position, *c))
            .collect();
        let Some(dbg) = self.world.resource_mut::<crate::resources::DebugDraw>() else {
            return;
        };
        let bound_color = crate::color::Color::rgba(0.3, 0.8, 1.0, 0.45);
        let col_color = crate::color::Color::rgba(0.3, 1.0, 0.4, 0.7);
        for (pos, scale) in bounds {
            let half = scale * 0.5;
            dbg.rect(pos - half, pos + half, bound_color);
        }
        for (pos, col) in colliders {
            match col {
                Collider::Aabb { half_extents } => {
                    dbg.rect(pos - half_extents, pos + half_extents, col_color);
                }
                Collider::Circle { radius } => {
                    dbg.circle(pos, radius, col_color);
                }
            }
        }
    }

    /// Draw the pathfinding-grid overlay via `DebugDraw`: for every `Tilemap` entity, build a
    /// [`crate::pathfinding::PathGrid`] (the standard "non-zero tile = blocked" convention) and
    /// shade each cell — blocked cells filled red, walkable cells outlined green. Called each
    /// frame from the editor UI while `show_pathgrid` is on. Visualizes exactly the grid a game
    /// following that convention would navigate, with no changes required to the game.
    pub(in crate::app) fn draw_pathfinding_overlay(&mut self) {
        use crate::pathfinding::PathGrid;
        use crate::tilemap::Tilemap;

        // We need a mutable borrow of DebugDraw and an immutable borrow of each Tilemap
        // at the same time, which the borrow checker forbids via `world`. To avoid
        // cloning the full Tilemap (which includes a TilemapAtlas with a heap-allocated
        // texture String), we collect only the minimal data needed by the overlay:
        // the tile grid (unavoidable — PathGrid::from_tilemap reads every cell),
        // plus tile_size and origin (two f32 + Vec2 scalars). The atlas is not needed
        // and is not cloned.
        struct TilemapSnapshot {
            tiles: Vec<Vec<u32>>,
            tile_size: f32,
            origin: glam::Vec2,
        }
        let snapshots: Vec<TilemapSnapshot> = self
            .world
            .query::<Tilemap>()
            .map(|(_, tm)| TilemapSnapshot {
                tiles: tm.tiles.clone(),
                tile_size: tm.tile_size,
                origin: tm.origin,
            })
            .collect();

        let Some(dbg) = self.world.resource_mut::<crate::resources::DebugDraw>() else {
            return;
        };
        let blocked_color = crate::color::Color::rgba(1.0, 0.3, 0.3, 0.35);
        let walkable_color = crate::color::Color::rgba(0.3, 1.0, 0.5, 0.25);
        for snap in snapshots {
            // Build a temporary Tilemap using the moved tile grid (no second clone).
            // The atlas is not used by PathGrid::from_tilemap; a zero-cost dummy is fine.
            let tm = Tilemap::new(
                crate::tilemap::TilemapAtlas::new("", 0, 0),
                snap.tiles,
                snap.tile_size,
                snap.origin,
            );
            let grid = PathGrid::from_tilemap(&tm, |id| id != 0);
            let half = glam::Vec2::splat(snap.tile_size * 0.5);
            for y in 0..grid.height {
                for x in 0..grid.width {
                    let center = tm.cell_center_world(y as usize, x as usize);
                    if grid.is_walkable(x, y) {
                        dbg.rect(center - half, center + half, walkable_color);
                    } else {
                        dbg.rect_filled_z(center - half, center + half, blocked_color, 0.0);
                    }
                }
            }
        }
    }

    /// Reset the selected entity's `ParticleEmitter` to its default configuration, preserving the
    /// currently-assigned texture. Backs the Particle Tuner "Reset to Default" button. No-op if the
    /// entity has no `ParticleEmitter`.
    pub(in crate::app) fn reset_particle_emitter(&mut self, sel: Entity) {
        if let Some(em) = self.world.get_mut::<crate::particle::ParticleEmitter>(sel) {
            let texture = em.texture.clone();
            *em = crate::particle::ParticleEmitter::default();
            em.texture = texture;
        }
    }

    /// Reset the selected entity's `PointLight` to its default configuration. Backs the Point Light
    /// editor "Reset to Default" button. No-op if the entity has no `PointLight`.
    pub(in crate::app) fn reset_point_light(&mut self, sel: Entity) {
        if let Some(light) = self.world.get_mut::<crate::components::PointLight>(sel) {
            *light = crate::components::PointLight::default();
        }
    }

    /// Ensure an `AmbientLight` resource exists (inserting the default if absent) so the editor's
    /// Ambient Light control always has something to edit. Returns whether one had to be inserted.
    pub(in crate::app) fn ensure_ambient_light(&mut self) -> bool {
        if self
            .world
            .resource::<crate::resources::AmbientLight>()
            .is_none()
        {
            self.world
                .insert_resource(crate::resources::AmbientLight::default());
            true
        } else {
            false
        }
    }
}
