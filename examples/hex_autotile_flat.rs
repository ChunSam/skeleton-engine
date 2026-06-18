//! hex_autotile_flat — autotiling on a **flat-top hexagonal** tilemap (`Neighborhood::Hex6Flat`).
//!
//! Autotile masks are computed from the `tiles[row][col]` grid, so they work on any projection once
//! the neighborhood matches the layout. This pairs `TilemapProjection::HexagonalFlat` (flat-top,
//! odd-q) with `Neighborhood::Hex6Flat` (the 6 parity-aware flat-top hex neighbors). A simple
//! **interior vs edge** rule shows the mask driving tile choice: a hex with all 6 neighbors filled
//! (mask 63) draws grass; any hex with an open edge draws sand. Dig holes and the rim around them
//! turns to sand — recomputed reactively by `TilemapSystem`.
//!
//! Run from the repo root:  `cargo run --example hex_autotile_flat`
//! Arrows = move cursor · Space = dig/fill the cell · ESC = quit.

use engine::{
    App, AutotileMode, Color, DrawText, InputState, KeyCode, Neighborhood, ShouldQuit, System,
    TextQueue, Tilemap, TilemapAtlas, TilemapAutotile, TilemapProjection, TilemapSystem, Vec2,
    ViewportSize, WindowConfig, World,
};
use std::collections::HashMap;

const ROWS: usize = 8;
const COLS: usize = 8;
const FILLED: u32 = 1; // any non-zero connects (single terrain)

/// Moves a cursor and digs/fills the hovered hex; the rest is the engine's reactive autotiling.
struct DigControl {
    map: engine::Entity,
    sel: (usize, usize),
}

impl System for DigControl {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (mut dr, mut dc, mut dig, mut quit) = (0i32, 0i32, false, false);
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
            dig = input.just_pressed(KeyCode::Space);
            quit = input.just_pressed(KeyCode::Escape);
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if dr != 0 || dc != 0 {
            self.sel.0 = (self.sel.0 as i32 + dr).clamp(0, ROWS as i32 - 1) as usize;
            self.sel.1 = (self.sel.1 as i32 + dc).clamp(0, COLS as i32 - 1) as usize;
        }
        if dig {
            if let Some(tm) = world.get_mut::<Tilemap>(self.map) {
                let cur = tm.get_tile(self.sel.0, self.sel.1).unwrap_or(0);
                tm.set_tile(self.sel.0, self.sel.1, if cur == 0 { FILLED } else { 0 });
            }
        }

        let vh = world
            .resource::<ViewportSize>()
            .map(|v| v.height)
            .unwrap_or(600.0);
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Flat-top hex autotile (Hex6Flat) — Arrows: move · Space: dig/fill · ESC: quit",
                Vec2::new(20.0, 20.0),
                18.0,
                Color::rgb(0.75, 0.82, 0.95),
            ));
            tq.push(DrawText::new(
                format!(
                    "cursor (row, col) = {:?}  (grass = interior, sand = open edge)",
                    self.sel
                ),
                Vec2::new(20.0, vh - 40.0),
                18.0,
                Color::rgb(0.95, 0.85, 0.55),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "hex_flat_dig_control"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "hex_autotile_flat — Hex6Flat interior/edge autotiling".to_string(),
        width: 900,
        height: 600,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });

    // Flat-top hex atlas: tile 0 = grass (interior), tile 1 = sand (edge).
    app.load_image("examples/assets/hex_tiles_flat.png");
    let atlas = TilemapAtlas::new("examples/assets/hex_tiles_flat.png", 2, 1);

    // Interior/edge rule: only mask 63 (all 6 hex neighbors filled) shows grass; every other mask
    // (any open edge) shows sand. A 2-tile autotile driven entirely by the Hex6Flat neighbor mask.
    let mask_to_tile: HashMap<u8, u32> = (0u8..64)
        .map(|m| (m, if m == 63 { 0 } else { 1 }))
        .collect();
    let autotile = TilemapAutotile {
        neighborhood: Neighborhood::Hex6Flat,
        oob_filled: false, // map-edge hexes read as edge (sand)
        mode: AutotileMode::Single { mask_to_tile },
    };

    let tiles = vec![vec![FILLED; COLS]; ROWS];
    let tilemap = Tilemap::new(atlas, tiles, 72.0, Vec2::new(150.0, 110.0))
        .with_projection(TilemapProjection::HexagonalFlat);

    let map = app.world.spawn();
    app.world.add_component(map, tilemap);
    app.world.add_component(map, autotile);

    app.add_system(TilemapSystem::new());
    app.add_system(DigControl {
        map,
        sel: (ROWS / 2, COLS / 2),
    });
    app.run();
}
