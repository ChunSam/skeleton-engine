//! Multi-terrain autotiling demo.
//!
//! A map of three terrains — grass (1), water (2), sand (3) — each autotiling its
//! own border against the others via [`TilemapAutotile::multi_edge_16`]. Move a cursor with the
//! arrow keys and paint the cursor cell with `1`/`2`/`3` (grass/water/sand) or `0`
//! (erase): `Tilemap::set_tile` mutates the grid and the reactive `TilemapSystem`
//! re-tiles the painted cell **and its neighbors** so every terrain border stays
//! continuous in real time.
//!
//! Engine convention: the camera maps its position (default `(0,0)`) to the viewport's
//! TOP-LEFT (Y down), so the map lives at POSITIVE world coordinates.

use engine::{
    App, Camera, DrawText, InputState, KeyCode, Sprite, System, TextQueue, Tilemap, TilemapAtlas,
    TilemapAutotile, TilemapSystem, Transform, WindowConfig, World,
};
use glam::Vec2;

const TILE: f32 = 32.0;
const COLS: usize = 16;
const ROWS: usize = 10;
const WIN_W: u32 = 600;
const WIN_H: u32 = 440;

// Tile values = terrain ids (0 = empty).
const GRASS: u32 = 1;
const WATER: u32 = 2;
const SAND: u32 = 3;

/// World-space top-left of the map, centred in the window (positive coords).
const ORIGIN: Vec2 = Vec2::new(
    (WIN_W as f32 - COLS as f32 * TILE) * 0.5,
    (WIN_H as f32 - ROWS as f32 * TILE) * 0.5 + 8.0,
);

/// Initial map: grass background, a water lake and a sand patch that meet grass and
/// each other, so every terrain's autotiled border is visible.
fn initial_tiles() -> Vec<Vec<u32>> {
    let mut t = vec![vec![GRASS; COLS]; ROWS];
    for (r, row) in t.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() {
            if (2..6).contains(&r) && (2..8).contains(&c) {
                *cell = WATER;
            }
            if (5..9).contains(&r) && (7..13).contains(&c) {
                *cell = SAND;
            }
        }
    }
    t
}

struct CursorState {
    tilemap: engine::Entity,
    marker: engine::Entity,
    row: usize,
    col: usize,
}

struct PaintSystem;

impl System for PaintSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (left, right, up, down, p_grass, p_water, p_sand, erase) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.just_pressed(KeyCode::ArrowLeft),
                    i.just_pressed(KeyCode::ArrowRight),
                    i.just_pressed(KeyCode::ArrowUp),
                    i.just_pressed(KeyCode::ArrowDown),
                    i.just_pressed(KeyCode::Digit1),
                    i.just_pressed(KeyCode::Digit2),
                    i.just_pressed(KeyCode::Digit3),
                    i.just_pressed(KeyCode::Digit0),
                )
            })
            .unwrap_or((false, false, false, false, false, false, false, false));

        let (tilemap, marker, mut row, mut col) = match world.resource::<CursorState>() {
            Some(s) => (s.tilemap, s.marker, s.row, s.col),
            None => return,
        };

        // Move cursor (clamped to the grid).
        if left && col > 0 {
            col -= 1;
        }
        if right && col + 1 < COLS {
            col += 1;
        }
        if up && row > 0 {
            row -= 1;
        }
        if down && row + 1 < ROWS {
            row += 1;
        }

        // Paint the cursor cell — set_tile + the reactive TilemapSystem re-tile it + neighbors.
        let paint = if p_grass {
            Some(GRASS)
        } else if p_water {
            Some(WATER)
        } else if p_sand {
            Some(SAND)
        } else if erase {
            Some(0)
        } else {
            None
        };
        if let Some(v) = paint {
            if let Some(tm) = world.get_mut::<Tilemap>(tilemap) {
                tm.set_tile(row, col, v);
            }
        }

        // Update cursor state + the marker sprite position.
        if let Some(s) = world.resource_mut::<CursorState>() {
            s.row = row;
            s.col = col;
        }
        let mpos = Vec2::new(
            ORIGIN.x + col as f32 * TILE + TILE * 0.5,
            ORIGIN.y + row as f32 * TILE + TILE * 0.5,
        );
        if let Some(t) = world.get_mut::<Transform>(marker) {
            t.position = mpos;
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Arrows: move cursor    1 grass  2 water  3 sand  0 erase",
                Vec2::new(12.0, 10.0),
                16.0,
                [230, 230, 240, 230],
            ));
            tq.push(DrawText::new(
                format!("cursor: ({row}, {col}) — paint to watch each terrain re-border live"),
                Vec2::new(12.0, WIN_H as f32 - 26.0),
                14.0,
                [180, 200, 220, 220],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "PaintSystem"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Multi-Terrain Autotiling".to_string(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.04, 0.05, 0.08, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    let tex = "examples/games/multi_terrain/assets/terrains.png";
    app.load_atlas(tex, 16, 3);

    let tilemap = Tilemap::new(TilemapAtlas::new(tex, 16, 3), initial_tiles(), TILE, ORIGIN);
    let tilemap_entity = app.world.spawn();
    app.world.add_component(tilemap_entity, tilemap);
    // Three terrains: value 1 → atlas base 0, value 2 → base 16, value 3 → base 32.
    app.world.add_component(
        tilemap_entity,
        TilemapAutotile::multi_edge_16(&[(GRASS, 0), (WATER, 16), (SAND, 32)])
            .with_oob_filled(false),
    );

    // Cursor marker: a translucent white square above the tiles.
    let marker = app.world.spawn();
    app.world.add_component(
        marker,
        Transform {
            position: ORIGIN + Vec2::splat(TILE * 0.5),
            scale: Vec2::splat(TILE),
            z: 1.0,
            ..Default::default()
        },
    );
    app.world.add_component(
        marker,
        Sprite {
            color: [255u8, 255, 255, 70].into(),
            ..Default::default()
        },
    );

    app.world.insert_resource(CursorState {
        tilemap: tilemap_entity,
        marker,
        row: ROWS / 2,
        col: COLS / 2,
    });

    app.add_system(TilemapSystem::new());
    app.add_system(PaintSystem);
    app.run();
}
