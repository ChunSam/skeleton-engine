use std::collections::HashMap;

use glam::Vec2;

use crate::components::{Sprite, Transform};
use crate::ecs::{Entity, System, World};
use crate::renderer::uv::UvRect;

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
    ///
    /// Returns [`UvRect::FULL`] (safe fallback) when the tile ID is out of range
    /// (i.e. `tile_id >= columns * rows`) to avoid producing UVs outside `[0, 1]`.
    pub fn uv_for(&self, tile_id: u32) -> UvRect {
        if self.columns == 0 || self.rows == 0 {
            return UvRect::FULL;
        }
        let col = tile_id % self.columns;
        let row = tile_id / self.columns;
        // Guard: out-of-range tile ID would produce a row >= rows, yielding UV offset >= 1.0.
        if row >= self.rows {
            return UvRect::FULL;
        }
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

    #[test]
    fn tilemap_atlas_out_of_range_tile_id_returns_full_uv() {
        // 2×2 atlas = 4 valid tile IDs (0–3). ID 99 is out of range and must
        // return UvRect::FULL (safe fallback) — not a UV with offset >= 1.0,
        // which would produce garbage rendering without this guard.
        let atlas = TilemapAtlas::new("tiles.png", 2, 2);
        let uv = atlas.uv_for(99);
        assert_eq!(
            uv,
            UvRect::FULL,
            "out-of-range tile_id must return UvRect::FULL, got {uv:?}"
        );
        // Also verify the fallback holds at the exact boundary (tile 4 = first out-of-range).
        let boundary = atlas.uv_for(4);
        assert_eq!(
            boundary,
            UvRect::FULL,
            "tile_id == columns*rows must return UvRect::FULL, got {boundary:?}"
        );
    }

    #[test]
    fn tilemap_atlas_in_range_tile_id_returns_correct_uv() {
        // 2×2 atlas: tile 0 = top-left (col 0, row 0), tile 3 = bottom-right (col 1, row 1).
        let atlas = TilemapAtlas::new("tiles.png", 2, 2);
        // Tile 0 → col 0, row 0 → u_offset=0.0, v_offset=0.0
        let uv0 = atlas.uv_for(0);
        assert!(
            uv0.u_offset.abs() < 1e-5 && uv0.v_offset.abs() < 1e-5,
            "tile 0 must be top-left, got {uv0:?}"
        );
        // Tile 3 → col 1, row 1 → u_offset=0.5, v_offset=0.5
        let uv3 = atlas.uv_for(3);
        assert!(
            (uv3.u_offset - 0.5).abs() < 1e-5 && (uv3.v_offset - 0.5).abs() < 1e-5,
            "tile 3 must be bottom-right, got {uv3:?}"
        );
        // Ensure no in-range UV has offset >= 1.0.
        for id in 0..4 {
            let uv = atlas.uv_for(id);
            assert!(
                uv.u_offset < 1.0 && uv.v_offset < 1.0,
                "tile {id} UV offset must be < 1.0, got {uv:?}"
            );
        }
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

impl TilemapSystem {
    /// Schedule label for ordering via `add_system_labeled`.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::tilemap";
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
                    world.add_component(tile_entity, Sprite::textured(atlas.texture.as_str()));
                    world.add_component(tile_entity, uv);
                    spawned.push(tile_entity);
                }
            }
            self.tile_entities.insert(map_entity, spawned);
        }
    }
}
