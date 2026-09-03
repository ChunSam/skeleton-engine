use crate::app::App;
use crate::ecs::{Entity, World};

/// The position and scale an entity is drawn at: its `GlobalTransform` when the hierarchy has
/// composed one, else its own `Transform`. This is the renderer's policy
/// (`renderer/sprite/collect.rs`) and the collision grid's (`collision/grid.rs`), so the
/// overlays show what those two see — a parented entity's `Transform.position` is only its
/// offset from the parent.
fn world_placement(
    world: &World,
    e: Entity,
    t: &crate::components::Transform,
) -> (glam::Vec2, glam::Vec2) {
    world
        .get::<crate::hierarchy::GlobalTransform>(e)
        .map(|g| (g.position, g.scale))
        .unwrap_or((t.position, t.scale))
}

impl App {
    /// Draw the debug bounds overlay via `DebugDraw`: every entity's Transform AABB plus any
    /// collision `Collider` shape. Called each frame from the editor UI while `show_bounds` is on.
    pub(in crate::app) fn draw_debug_bounds(&mut self) {
        use crate::collision::Collider;
        // Collect first so the immutable world borrow is released before borrowing DebugDraw.
        // Placed where things are DRAWN and COLLIDE (`world_placement`), not at their local
        // offset: until v0.156.11 this read `Transform` alone, so a parented collider's box drew
        // at its offset from the origin while collision tested it at its world position — the
        // exact placement this overlay exists to show.
        let world = &self.world;
        let bounds: Vec<(glam::Vec2, glam::Vec2)> = world
            .query::<crate::components::Transform>()
            .map(|(e, t)| world_placement(world, e, t))
            .collect();
        let colliders: Vec<(glam::Vec2, Collider)> = world
            .query2::<crate::components::Transform, Collider>()
            .map(|(e, t, c)| (world_placement(world, e, t).0, *c))
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

        // A mutable borrow of DebugDraw and an immutable borrow of each Tilemap cannot coexist
        // through `world`, so the grid is built while the tilemap is borrowed and only the
        // result crosses over: the `PathGrid`, plus the scalars that place a cell.
        // `PathGrid::from_tilemap` takes `&Tilemap`, so the tile grid never needed cloning —
        // the old comment called that clone "unavoidable". ⚠️ `projection` is part of the
        // placement: leaving it out drew every isometric and hexagonal map as a square lattice
        // at the wrong positions, contradicting the doc above (v0.156.11).
        struct TilemapSnapshot {
            grid: PathGrid,
            tile_size: f32,
            origin: glam::Vec2,
            projection: crate::tilemap::TilemapProjection,
        }
        let snapshots: Vec<TilemapSnapshot> = self
            .world
            .query::<Tilemap>()
            .map(|(_, tm)| TilemapSnapshot {
                grid: PathGrid::from_tilemap(tm, |id| id != 0),
                tile_size: tm.tile_size,
                origin: tm.origin,
                projection: tm.projection,
            })
            .collect();

        let Some(dbg) = self.world.resource_mut::<crate::resources::DebugDraw>() else {
            return;
        };
        let blocked_color = crate::color::Color::rgba(1.0, 0.3, 0.3, 0.35);
        let walkable_color = crate::color::Color::rgba(0.3, 1.0, 0.5, 0.25);
        for snap in snapshots {
            // A tile-less Tilemap carries the placement math: `cell_center_world` and
            // `cell_render_size` read only `tile_size`, `origin` and `projection`.
            let geom = Tilemap::new(
                crate::tilemap::TilemapAtlas::new("", 0, 0),
                Vec::new(),
                snap.tile_size,
                snap.origin,
            )
            .with_projection(snap.projection);
            let grid = snap.grid;
            let half = geom.cell_render_size() * 0.5;
            for y in 0..grid.height {
                for x in 0..grid.width {
                    let center = geom.cell_center_world(y as usize, x as usize);
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

    /// Insert a default `AmbientLight` if there is none. Returns whether one had to be inserted.
    ///
    /// ⚠️ **This switches the 2D lighting pass on**, because the pass is gated on the resource
    /// existing at all — so it is what the Ambient Light control's "Enable lighting" button
    /// does, and nothing calls it just to make a control drawable (v0.156.21).
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
