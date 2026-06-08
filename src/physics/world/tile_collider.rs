use glam::Vec2;
use rapier2d::prelude::*;

use crate::tilemap::Tilemap;

use super::{PhysicsWorld, TileCollider};

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
    /// The return value is a list of created `(RigidBodyHandle, ColliderHandle)` pairs
    /// (spawn order: row → column).
    pub fn add_static_from_tilemap(
        &mut self,
        tilemap: &Tilemap,
        pixels_per_unit: f32,
        mut collider_for: impl FnMut(u32) -> Option<TileCollider>,
    ) -> Vec<(RigidBodyHandle, ColliderHandle)> {
        let ppu = pixels_per_unit.max(f32::MIN_POSITIVE);
        let half = (tilemap.tile_size * 0.5) / ppu;
        let mut handles = Vec::new();
        for (row_idx, row) in tilemap.tiles.iter().enumerate() {
            for (col_idx, &tile_id) in row.iter().enumerate() {
                let Some(kind) = collider_for(tile_id) else {
                    continue;
                };
                let x =
                    tilemap.origin.x + col_idx as f32 * tilemap.tile_size + tilemap.tile_size * 0.5;
                let y =
                    tilemap.origin.y + row_idx as f32 * tilemap.tile_size + tilemap.tile_size * 0.5;
                let pair =
                    self.add_static_box_with_groups(Vec2::new(x, y) / ppu, half, half, kind.groups);
                if kind.one_way {
                    self.one_way_colliders.insert(pair.1);
                }
                handles.push(pair);
            }
        }
        handles
    }
}
