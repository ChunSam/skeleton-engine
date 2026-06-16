//! # dig_quest — runtime tilemap mutation + autotiling acceptance test
//!
//! A top-down miner game that validates the engine's runtime tilemap mutation
//! API and the `TilemapAutotile` component.  The player digs through a field of
//! solid dirt to reach a buried gem.
//!
//! ## Features exercised
//! * `Tilemap::set_tile` / `get_tile` / `dims` / `cell_center_world` / `cell_at_world`
//! * `TilemapAutotile::edge_16` + `oob_filled = true`
//! * `TilemapColliders` + `sync_tilemap_entity_colliders` (incremental collider sync on dig)
//! * `TilemapSystem` reactive tile-entity management (auto-triggered by mutation)
//! * `CharacterController` + `move_character` for top-down collide-and-slide
//!
//! ## Controls
//! Arrow keys / WASD : move   Space : dig   R : reset   Esc : quit
//!
//! ## Map layout
//! 20 × 14 cells, 32 px tiles → 640 × 448 px field.
//! Window: 720 × 520 so there's a small HUD border.  Camera is fixed, centred
//! on the field; no scroll needed.

use engine::{
    sync_tilemap_entity_colliders, App, Camera, CharacterController, DrawText, Entity, InputState,
    KeyCode, PhysicsBody, PhysicsSystem, PhysicsWorld, ShouldQuit, SolidTiles, Sprite, System,
    TextQueue, Tilemap, TilemapAtlas, TilemapAutotile, TilemapColliders, TilemapSystem, Transform,
    WindowConfig, World,
};
use glam::Vec2;
use rapier2d::na as nalgebra; // required by the `vector!` macro used in reset

// ── Constants ─────────────────────────────────────────────────────────────────

const WINDOW_W: u32 = 720;
const WINDOW_H: u32 = 520;

/// Pixels per physics unit (matches platformer convention).
const PPU: f32 = 64.0;

/// Side length of one tile in pixels.
const TILE_SIZE: f32 = 32.0;

/// Map dimensions in cells.
const MAP_COLS: usize = 20;
const MAP_ROWS: usize = 14;

/// World-space top-left corner of the tilemap. The engine's camera maps its
/// `position` (default `(0,0)`) to the viewport's top-left with Y down, so the
/// map lives at POSITIVE world coords — here centred in the window.
const MAP_ORIGIN: Vec2 = Vec2::new(
    (WINDOW_W as f32 - MAP_COLS as f32 * TILE_SIZE) * 0.5,
    (WINDOW_H as f32 - MAP_ROWS as f32 * TILE_SIZE) * 0.5,
);

/// Tile value: 0 = empty/dug, 1 = solid dirt.
const TILE_DIRT: u32 = 1;
const TILE_EMPTY: u32 = 0;

/// Starting pocket: a 3×3 clear area around the player spawn (row 1..=3, col 1..=3).
const POCKET_ROWS: std::ops::RangeInclusive<usize> = 1..=3;
const POCKET_COLS: std::ops::RangeInclusive<usize> = 1..=3;

/// Gem location (buried in dirt).
const GEM_ROW: usize = 10;
const GEM_COL: usize = 17;

/// Player spawn position in world space (centre of cell (2, 2)).
fn player_start() -> Vec2 {
    cell_center_world_const(2, 2)
}

/// Gem position in world space (centre of the gem cell).
fn gem_world_pos() -> Vec2 {
    cell_center_world_const(GEM_ROW, GEM_COL)
}

/// Const-compatible version of `Tilemap::cell_center_world` for static positions.
const fn cell_center_world_const(row: usize, col: usize) -> Vec2 {
    Vec2::new(
        MAP_ORIGIN.x + col as f32 * TILE_SIZE + TILE_SIZE * 0.5,
        MAP_ORIGIN.y + row as f32 * TILE_SIZE + TILE_SIZE * 0.5,
    )
}

/// Player physics half-extents in physics units (slightly smaller than a tile).
const PLAYER_HALF: f32 = 0.20; // 0.20 * 64 PPU = ~12.8 px

/// Movement speed in pixels per second (top-down, no acceleration curve needed).
const PLAYER_SPEED: f32 = 120.0;

// ── Map helpers ───────────────────────────────────────────────────────────────

/// Build the initial tile grid: all solid except the starting pocket.
fn initial_tiles() -> Vec<Vec<u32>> {
    let mut tiles = vec![vec![TILE_DIRT; MAP_COLS]; MAP_ROWS];
    for r in POCKET_ROWS {
        for c in POCKET_COLS.clone() {
            tiles[r][c] = TILE_EMPTY;
        }
    }
    tiles
}

// ── Game state resource ───────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum GameStatus {
    Playing,
    Won,
}

struct DigQuestSession {
    player: Entity,
    tilemap_entity: Entity,
    status: GameStatus,
    /// Last non-zero input direction, used as "facing" for dig targeting.
    facing: Vec2,
}

// ── Dig & movement system ─────────────────────────────────────────────────────

/// Combined movement, dig, and reset system.
///
/// The persistent `TileColliderIndex` now lives in the tilemap entity's
/// [`TilemapColliders`] component, so this system only carries the inner
/// `PhysicsSystem` that drives kinematic body integration.
struct DigSystem {
    physics: PhysicsSystem,
}

impl DigSystem {
    fn new() -> Self {
        Self {
            physics: PhysicsSystem::new(PPU),
        }
    }

    /// Syncs all tile colliders from the current tilemap state. The solid-tile rule
    /// (value != 0) and the persistent index live in the entity's `TilemapColliders`.
    fn sync_colliders(&mut self, world: &mut World, tilemap_entity: Entity) {
        sync_tilemap_entity_colliders(world, tilemap_entity);
    }

    /// Reset the world back to its initial state.
    fn reset(&mut self, world: &mut World) {
        let (player, tilemap_entity) = match world.resource::<DigQuestSession>() {
            Some(s) => (s.player, s.tilemap_entity),
            None => return,
        };

        // Restore tile grid.
        if let Some(tm) = world.get_mut::<Tilemap>(tilemap_entity) {
            tm.tiles = initial_tiles();
            // Direct field replacement bypasses set_tile's generation bump, so mark the
            // tilemap dirty explicitly or the reactive TilemapSystem won't re-render the reset.
            tm.bump_generation();
        }

        // Re-sync colliders against the restored grid using the SAME index: the
        // cells dug during play are re-added and everything else is untouched.
        // (Replacing the index would drop tracking and double-add colliders.)
        self.sync_colliders(world, tilemap_entity);

        // Reset player position.
        let start = player_start();
        if let Some(t) = world.get_mut::<Transform>(player) {
            t.position = start;
        }
        if let Some(body) = world
            .get::<PhysicsBody>(player)
            .map(|b| b.rigid_body_handle)
        {
            if let Some(physics) = world.resource_mut::<PhysicsWorld>() {
                if let Some(rb) = physics.rigid_body_mut(body) {
                    use rapier2d::prelude::vector;
                    let p = start / PPU;
                    rb.set_translation(vector![p.x, p.y], true);
                    rb.set_next_kinematic_translation(vector![p.x, p.y]);
                }
            }
        }

        // Reset session state.
        if let Some(s) = world.resource_mut::<DigQuestSession>() {
            s.status = GameStatus::Playing;
            s.facing = Vec2::X; // default facing: east
        }
    }
}

impl System for DigSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // ── 1. Read input ──────────────────────────────────────────────────────
        let (move_dir, dig_pressed, reset_pressed, quit_pressed) = world
            .resource::<InputState>()
            .map(|input| {
                let mut dir = Vec2::ZERO;
                if input.is_pressed(KeyCode::ArrowLeft) || input.is_pressed(KeyCode::KeyA) {
                    dir.x -= 1.0;
                }
                if input.is_pressed(KeyCode::ArrowRight) || input.is_pressed(KeyCode::KeyD) {
                    dir.x += 1.0;
                }
                if input.is_pressed(KeyCode::ArrowUp) || input.is_pressed(KeyCode::KeyW) {
                    dir.y -= 1.0;
                }
                if input.is_pressed(KeyCode::ArrowDown) || input.is_pressed(KeyCode::KeyS) {
                    dir.y += 1.0;
                }
                // Normalise diagonal movement.
                if dir.length_squared() > 1.0 {
                    dir = dir.normalize();
                }
                let dig = input.just_pressed(KeyCode::Space);
                let reset = input.just_pressed(KeyCode::KeyR);
                let quit = input.just_pressed(KeyCode::Escape);
                (dir, dig, reset, quit)
            })
            .unwrap_or((Vec2::ZERO, false, false, false));

        if quit_pressed {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        if reset_pressed {
            self.reset(world);
            // After reset, run physics once to settle positions, then return.
            self.physics.run(world, dt);
            return;
        }

        let (player, tilemap_entity, status) = match world.resource::<DigQuestSession>() {
            Some(s) => (s.player, s.tilemap_entity, s.status),
            None => return,
        };

        if status == GameStatus::Won {
            // Still tick physics so the player isn't frozen.
            self.physics.run(world, dt);
            return;
        }

        // ── 2. Update facing ──────────────────────────────────────────────────
        if move_dir != Vec2::ZERO {
            if let Some(s) = world.resource_mut::<DigQuestSession>() {
                s.facing = move_dir.normalize();
            }
        }

        // ── 3. Move character (collide-and-slide) ─────────────────────────────
        let handles = world
            .get::<PhysicsBody>(player)
            .map(|b| (b.rigid_body_handle, b.collider_handle));

        if let Some((rb, col)) = handles {
            let velocity = move_dir * PLAYER_SPEED;
            // `move_character` needs `&mut PhysicsWorld` and `&mut CharacterController`
            // simultaneously. `with_resource_mut` removes PhysicsWorld for the duration
            // of the closure so the borrow checker is satisfied, then re-inserts it.
            world.with_resource_mut::<PhysicsWorld, _>(|physics, world| {
                if let Some(controller) = world.get_mut::<CharacterController>(player) {
                    physics.move_character(controller, rb, col, velocity * dt, dt, PPU);
                }
            });
        }

        // ── 4. Dig ────────────────────────────────────────────────────────────
        if dig_pressed {
            let facing = world
                .resource::<DigQuestSession>()
                .map(|s| s.facing)
                .unwrap_or(Vec2::X);

            // Player's current world position (updated by move_character above).
            let player_pos = world
                .get::<Transform>(player)
                .map(|t| t.position)
                .unwrap_or_default();

            // Target cell = the cell one tile-step ahead in the facing direction.
            // We offset by slightly more than a half-tile to land in the next cell.
            let target_world = player_pos + facing * TILE_SIZE;

            // Look up target cell in the tilemap.
            let target_cell = world
                .get::<Tilemap>(tilemap_entity)
                .and_then(|tm| tm.cell_at_world(target_world));

            if let Some((tr, tc)) = target_cell {
                // Only dig solid cells (value != 0).
                let is_solid = world
                    .get::<Tilemap>(tilemap_entity)
                    .and_then(|tm| tm.get_tile(tr, tc))
                    .map(|v| v != 0)
                    .unwrap_or(false);

                if is_solid {
                    // Mutate the tilemap — TilemapSystem will pick this up next frame.
                    if let Some(tm) = world.get_mut::<Tilemap>(tilemap_entity) {
                        tm.set_tile(tr, tc, TILE_EMPTY);
                    }
                    // Immediately remove the collider for the dug cell so the player
                    // can walk in this frame without clipping.
                    self.sync_colliders(world, tilemap_entity);
                }
            }
        }

        // ── 5. Win check ──────────────────────────────────────────────────────
        let player_pos = world
            .get::<Transform>(player)
            .map(|t| t.position)
            .unwrap_or_default();

        let on_gem_cell = world
            .get::<Tilemap>(tilemap_entity)
            .and_then(|tm| {
                let (pr, pc) = tm.cell_at_world(player_pos)?;
                if pr == GEM_ROW && pc == GEM_COL {
                    Some(true)
                } else {
                    None
                }
            })
            .unwrap_or(false);

        if on_gem_cell {
            if let Some(s) = world.resource_mut::<DigQuestSession>() {
                s.status = GameStatus::Won;
            }
        }

        // ── 6. Tick physics ───────────────────────────────────────────────────
        self.physics.run(world, dt);
    }

    fn name(&self) -> &'static str {
        "DigSystem"
    }
}

// ── HUD system ────────────────────────────────────────────────────────────────

struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // Collect status before taking the mutable TextQueue borrow.
        let status = world
            .resource::<DigQuestSession>()
            .map(|s| s.status)
            .unwrap_or(GameStatus::Playing);

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };

        // Instructions line — screen space, top-left origin (Y down).
        tq.push(DrawText::new(
            "Arrows/WASD: move   Space: dig   R: reset   Esc: quit",
            Vec2::new(12.0, WINDOW_H as f32 - 26.0),
            17.0,
            [220, 230, 255, 200],
        ));

        match status {
            GameStatus::Playing => {
                tq.push(DrawText::new(
                    "Dig to the gem buried in the dark!",
                    Vec2::new(196.0, 10.0),
                    18.0,
                    [255, 220, 100, 220],
                ));
            }
            GameStatus::Won => {
                tq.push(DrawText::new(
                    "You reached the gem!  Press R to reset.",
                    Vec2::new(150.0, 244.0),
                    26.0,
                    [100, 255, 160, 255],
                ));
            }
        }
    }

    fn name(&self) -> &'static str {
        "HudSystem"
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();

    app.world.insert_resource(WindowConfig {
        title: "dig quest — skeleton-engine tilemap mutation demo".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.05, 0.04, 0.08, 1.0],
    });

    // ── Camera: fixed at the world origin ────────────────────────────────────
    // `Camera.position` is the viewport's TOP-LEFT world coordinate (Y down), so
    // position (0,0) zoom 1.0 shows world [0,720] x [0,520] — the whole field
    // (placed at the positive, window-centred MAP_ORIGIN) is visible, no scroll.
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    // ── Load tile texture so TilemapSystem sprites can resolve it by path ────
    // `load_atlas` registers the texture in the asset server with its path as
    // the cache key; TilemapSystem will find it when it spawns tile sprites.
    app.load_atlas("examples/games/dig_quest/assets/autotile.png", 4, 4);

    // ── Build initial tile grid ───────────────────────────────────────────────
    let tiles = initial_tiles();
    let tilemap = Tilemap::new(
        TilemapAtlas::new("examples/games/dig_quest/assets/autotile.png", 4, 4),
        tiles,
        TILE_SIZE,
        MAP_ORIGIN,
    );

    let tilemap_entity = app.world.spawn();
    app.world.add_component(tilemap_entity, tilemap);

    // Attach autotile: 16-tile edge sheet, oob = filled so the outer world
    // boundary reads as solid cave wall and only dig holes show interior outlines.
    app.world.add_component(
        tilemap_entity,
        TilemapAutotile::edge_16(0).with_oob_filled(true),
    );

    // ── Physics world (no gravity — top-down) ────────────────────────────────
    let mut physics = PhysicsWorld::new(Vec2::ZERO);

    // ── Initial tile colliders ────────────────────────────────────────────────
    // Build the initial colliders into a `TilemapColliders` component, which tracks
    // the index so later per-dig syncs are incremental (remove the exact dug cell).
    // Solid rule: any non-zero tile id (`SolidTiles::NonZero`). Done through `sync`
    // (NOT `add_static_from_tilemap`) so every collider is tracked from the start.
    let mut colliders = TilemapColliders::new(PPU, SolidTiles::NonZero);
    {
        let tilemap_ref = app.world.get::<Tilemap>(tilemap_entity).unwrap();
        colliders.sync(&mut physics, tilemap_ref);
    }
    app.world.add_component(tilemap_entity, colliders);

    // ── Player ────────────────────────────────────────────────────────────────
    let start = player_start();
    let (player_rb, player_col) = physics.add_kinematic_box(start / PPU, PLAYER_HALF, PLAYER_HALF);

    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: start,
            scale: Vec2::splat(TILE_SIZE * 0.75), // slightly smaller than a tile
            z: 0.5,
            ..Default::default()
        },
    );
    // Simple coloured square for the player (no separate sprite asset needed).
    app.world.add_component(
        player,
        Sprite {
            color: [80u8, 180, 255, 255].into(),
            ..Default::default()
        },
    );
    app.world.add_component(
        player,
        PhysicsBody {
            rigid_body_handle: player_rb,
            collider_handle: player_col,
        },
    );
    app.world
        .add_component(player, CharacterController::top_down());

    // ── Gem: a yellow square rendered at the buried cell ─────────────────────
    let gem_pos = gem_world_pos();
    let gem_entity = app.world.spawn();
    app.world.add_component(
        gem_entity,
        Transform {
            position: gem_pos,
            scale: Vec2::splat(TILE_SIZE * 0.55),
            z: 0.3, // below the player, above tile sprites (-1)
            ..Default::default()
        },
    );
    app.world.add_component(
        gem_entity,
        Sprite {
            color: [255u8, 220, 40, 255].into(),
            ..Default::default()
        },
    );

    // ── Session resource ──────────────────────────────────────────────────────
    app.world.insert_resource(DigQuestSession {
        player,
        tilemap_entity,
        status: GameStatus::Playing,
        facing: Vec2::X, // default facing: east
    });

    app.world.insert_resource(physics);

    // ── Systems ───────────────────────────────────────────────────────────────
    // TilemapSystem must run so tile mutations are reflected in sprites.
    // It runs first so the render state is always consistent with the logical
    // state by the time the frame is drawn.
    app.add_system(TilemapSystem::new());

    // DigSystem handles movement + digging + physics integration.
    app.add_system(DigSystem::new());

    // HUD last so it overlays everything.
    app.add_system(HudSystem);

    app.run();
}
