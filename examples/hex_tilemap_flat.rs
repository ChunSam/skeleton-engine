//! hex_tilemap_flat — a **flat-top** hexagonal tilemap (`TilemapProjection::HexagonalFlat`).
//!
//! The flat-top counterpart of `hex_tilemap` (which is pointy-top): odd-q offset coordinates, where
//! odd **columns** are shifted down by half a tile. Same `Tilemap` + `TilemapSystem`, built with
//! `.with_projection(TilemapProjection::HexagonalFlat)`. Demonstrates both directions:
//! - **selection** (arrow keys) moves a cell; the selected cell is painted sand via reactive
//!   `set_tile`, positioned by the engine's `cell_center_world`.
//! - **picking** (mouse) reads the hovered cell with `cell_at_world` (pixel → axial → cube-round).
//!
//! Run from the repo root:  `cargo run --example hex_tilemap_flat`
//! Arrows = move selection · mouse = hover-pick · ESC = quit.

use engine::{
    App, Camera, Color, DrawText, InputState, KeyCode, ShouldQuit, System, TextQueue, Tilemap,
    TilemapAtlas, TilemapProjection, TilemapSystem, Vec2, ViewportSize, WindowConfig, World,
};

const ROWS: usize = 8;
const COLS: usize = 8;
const GRASS: u32 = 1; // atlas tile 0
const SAND: u32 = 2; // atlas tile 1

/// Moves the selection over the flat-top hex grid and reports the mouse-picked cell.
struct HexControl {
    map: engine::Entity,
    sel: (usize, usize),
}

impl System for HexControl {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (mut dr, mut dc, mut quit) = (0i32, 0i32, false);
        let mut cursor = Vec2::ZERO;
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::ArrowUp) {
                dr -= 1;
            }
            if input.just_pressed(KeyCode::ArrowDown) {
                dr += 1;
            }
            if input.just_pressed(KeyCode::ArrowLeft) {
                dc -= 1;
            }
            if input.just_pressed(KeyCode::ArrowRight) {
                dc += 1;
            }
            quit = input.just_pressed(KeyCode::Escape);
            cursor = input.cursor();
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // Move selection: clear the old sand cell, paint the new one (reactive set_tile).
        if dr != 0 || dc != 0 {
            let nr = (self.sel.0 as i32 + dr).clamp(0, ROWS as i32 - 1) as usize;
            let nc = (self.sel.1 as i32 + dc).clamp(0, COLS as i32 - 1) as usize;
            if (nr, nc) != self.sel {
                if let Some(tm) = world.get_mut::<Tilemap>(self.map) {
                    tm.set_tile(self.sel.0, self.sel.1, GRASS);
                    tm.set_tile(nr, nc, SAND);
                }
                self.sel = (nr, nc);
            }
        }

        // Pick the hovered hex from the cursor's world position (flat-top pixel → axial → cube-round).
        let hover = world
            .resource::<Camera>()
            .map(|cam| cam.screen_to_world(cursor))
            .and_then(|wp| {
                world
                    .get::<Tilemap>(self.map)
                    .and_then(|tm| tm.cell_at_world(wp))
            });

        let vh = world
            .resource::<ViewportSize>()
            .map(|v| v.height)
            .unwrap_or(600.0);
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Flat-top hexagonal tilemap — Arrows: move selection · mouse: hover-pick · ESC: quit",
                Vec2::new(20.0, 20.0),
                18.0,
                Color::rgb(0.75, 0.82, 0.95),
            ));
            tq.push(DrawText::new(
                format!("selected cell (row, col) = {:?}", self.sel),
                Vec2::new(20.0, vh - 60.0),
                18.0,
                Color::rgb(0.95, 0.85, 0.55),
            ));
            tq.push(DrawText::new(
                match hover {
                    Some(c) => format!("mouse hover cell = {c:?}"),
                    None => "mouse hover cell = (outside grid)".to_string(),
                },
                Vec2::new(20.0, vh - 34.0),
                18.0,
                Color::rgb(0.6, 0.85, 1.0),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "hex_flat_control"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "hex_tilemap_flat — flat-top hexagonal projection".to_string(),
        width: 900,
        height: 600,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });

    // 2-tile atlas (2 cols × 1 row): tile 0 = grass hex, tile 1 = sand hex (flat-top art).
    app.load_image("examples/assets/hex_tiles_flat.png");
    let atlas = TilemapAtlas::new("examples/assets/hex_tiles_flat.png", 2, 1);

    // All grass with a sand border so the hex layout + odd-column offset are obvious.
    let mut tiles = vec![vec![GRASS; COLS]; ROWS];
    for (r, row) in tiles.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() {
            if r == 0 || c == 0 || r == ROWS - 1 || c == COLS - 1 {
                *cell = SAND;
            }
        }
    }
    let sel = (ROWS / 2, COLS / 2);
    tiles[sel.0][sel.1] = SAND;

    // origin = cell (0,0)'s center. tile_size = hex flat-to-flat height (flat-top).
    let tilemap = Tilemap::new(atlas, tiles, 64.0, Vec2::new(140.0, 110.0))
        .with_projection(TilemapProjection::HexagonalFlat);

    let map = app.world.spawn();
    app.world.add_component(map, tilemap);

    app.add_system(TilemapSystem::new());
    app.add_system(HexControl { map, sel });
    app.run();
}
