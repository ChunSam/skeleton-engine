use std::collections::HashMap;

use glam::Vec2;

use crate::animation::player::UvRect;
use crate::components::{Sprite, Transform};
use crate::ecs::{Entity, System, World};

// ─── Data types ───────────────────────────────────────────────────────────────

/// Texture atlas configuration for a tilemap.
#[derive(Debug, Clone)]
pub struct TilemapAtlas {
    /// Texture file path.
    pub texture: String,
    /// Number of tile columns in the atlas.
    pub columns: u32,
    /// Number of tile rows in the atlas.
    pub rows: u32,
}

impl TilemapAtlas {
    pub fn new(texture: impl Into<String>, columns: u32, rows: u32) -> Self {
        Self {
            texture: texture.into(),
            columns,
            rows,
        }
    }

    /// Returns the UV coordinates for the given tile ID (0-based).
    pub fn uv_for(&self, tile_id: u32) -> UvRect {
        if self.columns == 0 || self.rows == 0 {
            return UvRect::FULL;
        }
        let col = tile_id % self.columns;
        let row = tile_id / self.columns;
        UvRect::from_grid(col, row, self.columns, self.rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilemap_atlas_zero_grid_returns_full_uv() {
        let atlas = TilemapAtlas::new("tiles.png", 0, 0);
        assert_eq!(atlas.uv_for(0), UvRect::FULL);
    }
}

/// Tilemap component.
///
/// Attach to an entity and `TilemapSystem` will automatically spawn tile entities.
/// `tiles[row][col]` = 0 means an empty tile; 1 or more means use `atlas.uv_for(tile_id - 1)`.
#[derive(Debug, Clone)]
pub struct Tilemap {
    pub atlas: TilemapAtlas,
    /// `tiles[row][col]` layout. 0 = empty cell, 1+ = tile ID + 1.
    pub tiles: Vec<Vec<u32>>,
    /// Side length of one tile (pixels).
    pub tile_size: f32,
    /// Top-left origin of the tilemap (world coordinates).
    pub origin: Vec2,
}

impl Tilemap {
    pub fn new(atlas: TilemapAtlas, tiles: Vec<Vec<u32>>, tile_size: f32, origin: Vec2) -> Self {
        Self {
            atlas,
            tiles,
            tile_size,
            origin,
        }
    }
}

// ─── Systems ──────────────────────────────────────────────────────────────────

/// System that reads Tilemap components and manages tile entities.
///
/// Spawns tile entities when a Tilemap entity is first encountered.
/// Despawns tile entities when a Tilemap entity disappears.
pub struct TilemapSystem {
    /// Tilemap entity → list of spawned tile entities.
    tile_entities: HashMap<Entity, Vec<Entity>>,
}

impl TilemapSystem {
    pub fn new() -> Self {
        Self {
            tile_entities: HashMap::new(),
        }
    }
}

impl Default for TilemapSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for TilemapSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // Collect currently alive tilemap entities
        let tilemap_entities: Vec<Entity> = world.query::<Tilemap>().map(|(e, _)| e).collect();

        // Despawn tiles belonging to tilemap entities that have disappeared
        let removed: Vec<Entity> = self
            .tile_entities
            .keys()
            .filter(|e| !tilemap_entities.contains(e))
            .copied()
            .collect();
        for map_entity in removed {
            if let Some(tiles) = self.tile_entities.remove(&map_entity) {
                for tile in tiles {
                    world.despawn(tile);
                }
            }
        }

        // Spawn new tilemap entities
        for map_entity in tilemap_entities {
            if self.tile_entities.contains_key(&map_entity) {
                continue; // already processed
            }

            let (atlas, tiles, tile_size, origin) = {
                let tm = world.get::<Tilemap>(map_entity).unwrap();
                (tm.atlas.clone(), tm.tiles.clone(), tm.tile_size, tm.origin)
            };

            let mut spawned = Vec::new();
            for (row_idx, row) in tiles.iter().enumerate() {
                for (col_idx, &tile_id) in row.iter().enumerate() {
                    if tile_id == 0 {
                        continue;
                    }
                    let actual_id = tile_id - 1;
                    let uv = atlas.uv_for(actual_id);
                    let x = origin.x + col_idx as f32 * tile_size + tile_size * 0.5;
                    let y = origin.y + row_idx as f32 * tile_size + tile_size * 0.5;

                    let tile_entity = world.spawn();
                    world.add_component(
                        tile_entity,
                        Transform {
                            position: Vec2::new(x, y),
                            scale: Vec2::splat(tile_size),
                            rotation: 0.0,
                            z: -1.0,
                        },
                    );
                    // UV is controlled directly via the UvRect component without an AnimationPlayer.
                    world.add_component(tile_entity, Sprite::textured(&atlas.texture));
                    world.add_component(tile_entity, uv);
                    spawned.push(tile_entity);
                }
            }
            self.tile_entities.insert(map_entity, spawned);
        }
    }
}
