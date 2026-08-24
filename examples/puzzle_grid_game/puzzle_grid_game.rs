//! `puzzle_grid_game` — the puzzle genre-game of the rebuilt examples tree.
//!
//! Phase 4 of `plans/2026-08-19-examples-rebuild-plan.md`. The other three games move things;
//! this one **edits the world and takes it back**. The board is reconstructed from a snapshot on
//! every undo, which is what makes `History` meaningful — and what makes autotile correctness
//! *assertable*, because a wrong tile index after an undo is a visible difference rather than a
//! feeling.
//!
//! ```text
//! cargo run --example puzzle_grid_game                         # play it
//! PUZZLE_GRID_SELFTEST=1 cargo run --example puzzle_grid_game  # the acceptance test (headless)
//! PUZZLE_GRID_GEN_ASSETS=1 cargo run --example puzzle_grid_game  # regenerate assets/tiles.png
//! ```
//!
//! # The puzzle
//!
//! You are somewhere in a generated cavern and the exit is somewhere else. You cannot walk — you
//! **carve** (`Space` on a wall) and **fill** (`Space` on floor), on a budget, until the exit is
//! *visible* from where you stand. Visibility is [`FovMap`], so the puzzle is line-of-sight: a
//! straight corridor is worth more than a big room.
//!
//! Every edit is a move, every move is a snapshot, and `Z` / `Y` walk the history. That is the
//! whole design — the board is small, the rules are two verbs, and the interesting part is that
//! **undo has to restore the render, not just the data**.
//!
//! # Three generators, one board
//!
//! `Tab` cycles BSP rooms → cellular cave → perfect maze, reseeding as it goes. They are here
//! together because each guarantees connectivity by a *different mechanism* (corridor carving,
//! keep-largest-region, spanning-tree carve), so "the board is connected" is one property with
//! three independent proofs — which is exactly what check 3 exploits.
//!
//! # assets/tiles.png
//!
//! An 8×4 grid of 32 px cells: **two 16-cell edge-autotile strips**, wall at index 0 and floor at
//! index 16, which is the layout `TilemapAutotile::multi_edge_16(&[(WALL, 0), (FLOOR, 16)])`
//! indexes into. Within a strip the cell index is the edge-4 neighbour mask (N=1, E=2, S=4, W=8).
//! Regenerate with `PUZZLE_GRID_GEN_ASSETS=1`; the generator is deterministic.

use engine::{
    App, CaveParams, Coroutine, CoroutineRunner, CoroutineSystem, DebugDraw, DrawText, DungeonMap,
    DungeonParams, Easing, Entity, FovMap, History, InputState, KeyCode, MazeParams, PathGrid,
    ShouldQuit, Sprite, System, SystemConfig, TextQueue, Tilemap, TilemapAtlas, TilemapAutotile,
    TilemapSystem, Transform, Tween, WindowConfig, World,
};
use glam::{IVec2, Vec2};
use serde::{Deserialize, Serialize};

// ── Board ───────────────────────────────────────────────────────────────────────────────────────

const WINDOW_W: u32 = 960;
const WINDOW_H: u32 = 640;
const TILE: f32 = 24.0;
const BOARD_W: i32 = 33;
const BOARD_H: i32 = 21;
/// Top-left of the board in world pixels, leaving room for the HUD.
const ORIGIN: Vec2 = Vec2::new(48.0, 96.0);

/// Tile values. `0` would be "empty" to `Tilemap`; this board has no empty cells, so both terrains
/// are non-zero and the autotile has two strips to choose between.
const WALL: u32 = 1;
const FLOOR: u32 = 2;

/// How far the player can see. Small enough that the exit is never visible by accident.
const SIGHT: i32 = 7;
/// Edits allowed per level. The budget is what makes it a puzzle rather than a shovel.
const BUDGET: u32 = 14;

const TILES_PATH: &str = "examples/puzzle_grid_game/assets/tiles.png";
const SAVE_APP: &str = "skeleton-engine-puzzle-grid";
const SAVE_FILE: &str = "progress.sav";

/// Which generator built the current board.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Gen {
    Rooms,
    Cave,
    Maze,
}

impl Gen {
    fn next(self) -> Self {
        match self {
            Gen::Rooms => Gen::Cave,
            Gen::Cave => Gen::Maze,
            Gen::Maze => Gen::Rooms,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Gen::Rooms => "BSP rooms",
            Gen::Cave => "cellular cave",
            Gen::Maze => "perfect maze",
        }
    }

    /// Builds a board. Each generator guarantees connectivity by its own mechanism — corridors
    /// between BSP leaves, keep-largest-region after smoothing, and a spanning-tree carve.
    fn build(self, seed: u64) -> DungeonMap {
        match self {
            Gen::Rooms => {
                engine::generate_bsp_dungeon(BOARD_W, BOARD_H, seed, &DungeonParams::default())
            }
            Gen::Cave => {
                engine::generate_cellular_cave(BOARD_W, BOARD_H, seed, &CaveParams::default())
            }
            Gen::Maze => engine::generate_maze(BOARD_W, BOARD_H, seed, &MazeParams::default()),
        }
    }
}

/// The whole mutable board state, and the unit `History` snapshots.
///
/// A `Vec<u32>` of tile values rather than the `Tilemap` itself: snapshots must be cheap enough to
/// take on **every move**, and comparing two of them is what check 1 does — "the board came back"
/// is a byte comparison, not a win flag that could be true for other reasons.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
struct Board {
    tiles: Vec<u32>,
    player: (i32, i32),
    exit: (i32, i32),
    edits_left: u32,
}

impl Board {
    fn at(&self, x: i32, y: i32) -> u32 {
        if x < 0 || y < 0 || x >= BOARD_W || y >= BOARD_H {
            return WALL;
        }
        self.tiles[(y * BOARD_W + x) as usize]
    }

    fn set(&mut self, x: i32, y: i32, v: u32) {
        if x < 0 || y < 0 || x >= BOARD_W || y >= BOARD_H {
            return;
        }
        self.tiles[(y * BOARD_W + x) as usize] = v;
    }

    /// A `PathGrid` over the current board — walls block, floor walks. `FovMap::from_path_grid`
    /// then derives opacity from exactly the same data, which is the bridge check 4 pins.
    fn path_grid(&self) -> PathGrid {
        let mut grid = PathGrid::new(BOARD_W, BOARD_H);
        for y in 0..BOARD_H {
            for x in 0..BOARD_W {
                grid.set_walkable(x, y, self.at(x, y) == FLOOR);
            }
        }
        grid
    }

    /// Visibility from the player, at `SIGHT` radius.
    fn fov(&self) -> FovMap {
        let mut fov = FovMap::from_path_grid(&self.path_grid());
        fov.compute(IVec2::new(self.player.0, self.player.1), SIGHT);
        fov
    }

    fn exit_visible(&self) -> bool {
        self.fov().is_visible(self.exit.0, self.exit.1)
    }

    /// Builds a board from a generated map: the player goes in the first open cell, the exit in the
    /// farthest one, so a level always starts unsolved.
    fn from_map(map: &DungeonMap) -> Self {
        let mut tiles = vec![WALL; (BOARD_W * BOARD_H) as usize];
        let mut floors = Vec::new();
        for y in 0..BOARD_H {
            for x in 0..BOARD_W {
                if map.is_floor(x, y) {
                    tiles[(y * BOARD_W + x) as usize] = FLOOR;
                    floors.push((x, y));
                }
            }
        }
        let player = floors.first().copied().unwrap_or((1, 1));
        let exit = floors
            .iter()
            .copied()
            .max_by_key(|(x, y)| {
                let dx = (x - player.0) as i64;
                let dy = (y - player.1) as i64;
                dx * dx + dy * dy
            })
            .unwrap_or(player);
        Self {
            tiles,
            player,
            exit,
            edits_left: BUDGET,
        }
    }
}

/// Saved progress. Deliberately small: which levels are solved and what the next seed is.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Progress {
    solved: u32,
    seed: u64,
}

fn save_file() -> std::path::PathBuf {
    engine::save::save_path(SAVE_APP, SAVE_FILE)
}

// ── Session ─────────────────────────────────────────────────────────────────────────────────────

struct Puzzle {
    map_entity: Entity,
    cursor_entity: Entity,
    exit_entity: Entity,
    player_entity: Entity,
    board: Board,
    /// Snapshots, one per move. The unit is the whole board — small enough to copy, and the only
    /// thing that makes "undo restored it" a comparison rather than an opinion.
    history: History<Board>,
    generator: Gen,
    seed: u64,
    cursor: IVec2,
    progress: Progress,
    solved_this_level: bool,
    /// The win flourish: a tween the coroutine drives.
    flash: Option<Tween>,
    show_fov: bool,
}

// ── Board ↔ Tilemap ─────────────────────────────────────────────────────────────────────────────

/// Pushes the whole board into the `Tilemap`.
///
/// `set_tile` bumps the generation counter, so the reactive `TilemapSystem` re-renders only what
/// changed — and with a `TilemapAutotile` attached it re-derives each cell's *display* index from
/// its neighbours. That derivation is the thing an undo has to get right: restoring the values is
/// not enough if the neighbour masks are not recomputed, and the difference is visible.
fn push_board(world: &mut World, map_entity: Entity, board: &Board) {
    let Some(tilemap) = world.get_mut::<Tilemap>(map_entity) else {
        return;
    };
    for y in 0..BOARD_H {
        for x in 0..BOARD_W {
            tilemap.set_tile(y as usize, x as usize, board.at(x, y));
        }
    }
}

fn cell_to_world(cell: IVec2) -> Vec2 {
    ORIGIN
        + Vec2::new(
            cell.x as f32 * TILE + TILE * 0.5,
            cell.y as f32 * TILE + TILE * 0.5,
        )
}

/// Regenerates the level: new map, new board, cleared history.
fn new_level(world: &mut World, generator: Gen, seed: u64) {
    let map = generator.build(seed);
    let board = Board::from_map(&map);
    let Some((map_entity, player_e, exit_e)) = world
        .resource::<Puzzle>()
        .map(|p| (p.map_entity, p.player_entity, p.exit_entity))
    else {
        return;
    };
    push_board(world, map_entity, &board);
    if let Some(t) = world.get_mut::<Transform>(player_e) {
        t.position = cell_to_world(IVec2::new(board.player.0, board.player.1));
    }
    if let Some(t) = world.get_mut::<Transform>(exit_e) {
        t.position = cell_to_world(IVec2::new(board.exit.0, board.exit.1));
    }
    if let Some(p) = world.resource_mut::<Puzzle>() {
        p.cursor = IVec2::new(board.player.0, board.player.1);
        p.board = board;
        p.history = History::new();
        p.generator = generator;
        p.seed = seed;
        p.solved_this_level = false;
        p.flash = None;
    }
}

// ── Systems ─────────────────────────────────────────────────────────────────────────────────────

const L_INPUT: &str = "game::input";
const L_RULES: &str = "game::rules";

/// Cursor movement, carve/fill, undo/redo, level cycling.
struct InputSystem;

impl System for InputSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(keys) = world.resource::<InputState>().map(|i| Keys {
            left: i.just_pressed(KeyCode::ArrowLeft) || i.just_pressed(KeyCode::KeyA),
            right: i.just_pressed(KeyCode::ArrowRight) || i.just_pressed(KeyCode::KeyD),
            up: i.just_pressed(KeyCode::ArrowUp) || i.just_pressed(KeyCode::KeyW),
            down: i.just_pressed(KeyCode::ArrowDown) || i.just_pressed(KeyCode::KeyS),
            act: i.just_pressed(KeyCode::Space),
            undo: i.just_pressed(KeyCode::KeyZ),
            redo: i.just_pressed(KeyCode::KeyY),
            cycle: i.just_pressed(KeyCode::Tab),
            fov: i.just_pressed(KeyCode::KeyF),
            quit: i.just_pressed(KeyCode::Escape),
        }) else {
            return;
        };

        if keys.quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if keys.fov {
            if let Some(p) = world.resource_mut::<Puzzle>() {
                p.show_fov = !p.show_fov;
            }
        }

        let mut delta = IVec2::ZERO;
        if keys.left {
            delta.x -= 1;
        }
        if keys.right {
            delta.x += 1;
        }
        if keys.up {
            delta.y -= 1;
        }
        if keys.down {
            delta.y += 1;
        }
        if delta != IVec2::ZERO {
            if let Some(p) = world.resource_mut::<Puzzle>() {
                p.cursor = IVec2::new(
                    (p.cursor.x + delta.x).clamp(0, BOARD_W - 1),
                    (p.cursor.y + delta.y).clamp(0, BOARD_H - 1),
                );
            }
        }

        if keys.cycle {
            let (next, seed) = world
                .resource::<Puzzle>()
                .map(|p| (p.generator.next(), p.seed.wrapping_add(1)))
                .unwrap_or((Gen::Rooms, 1));
            new_level(world, next, seed);
            return;
        }

        if keys.act {
            apply_edit(world);
        }
        if keys.undo {
            step_history(world, true);
        }
        if keys.redo {
            step_history(world, false);
        }
    }

    fn name(&self) -> &'static str {
        "InputSystem"
    }
}

struct Keys {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    act: bool,
    undo: bool,
    redo: bool,
    cycle: bool,
    fov: bool,
    quit: bool,
}

/// Carve a wall or fill a floor at the cursor, recording the *pre-move* board first.
///
/// ⚠️ Order is the whole contract of `History::record`: it stores the snapshot you hand it and
/// `undo` swaps the current value with the top of the stack. Recording *after* mutating would make
/// undo restore the state you were already in — the classic off-by-one that still looks like
/// "undo works" as long as you only press it once.
fn apply_edit(world: &mut World) {
    let Some((mut board, map_entity)) = world
        .resource::<Puzzle>()
        .map(|p| (p.board.clone(), p.map_entity))
    else {
        return;
    };
    let (cursor, budget) = world
        .resource::<Puzzle>()
        .map(|p| (p.cursor, p.board.edits_left))
        .unwrap_or((IVec2::ZERO, 0));
    if budget == 0 {
        return;
    }
    // The player's own cell and the exit are not editable — filling either would make the level
    // unsolvable in a way the player cannot see.
    if (cursor.x, cursor.y) == board.player || (cursor.x, cursor.y) == board.exit {
        return;
    }

    let before = board.clone();
    let current = board.at(cursor.x, cursor.y);
    board.set(
        cursor.x,
        cursor.y,
        if current == WALL { FLOOR } else { WALL },
    );
    board.edits_left -= 1;

    push_board(world, map_entity, &board);
    if let Some(p) = world.resource_mut::<Puzzle>() {
        p.history.record(before);
        p.board = board;
    }
}

/// Walks the history one step and re-pushes the board — including the autotile re-derivation.
fn step_history(world: &mut World, undo: bool) {
    let Some((mut board, map_entity)) = world
        .resource::<Puzzle>()
        .map(|p| (p.board.clone(), p.map_entity))
    else {
        return;
    };
    let moved = {
        let Some(p) = world.resource_mut::<Puzzle>() else {
            return;
        };
        if undo {
            p.history.undo(&mut board)
        } else {
            p.history.redo(&mut board)
        }
    };
    if !moved {
        return;
    }
    push_board(world, map_entity, &board);
    if let Some(p) = world.resource_mut::<Puzzle>() {
        p.board = board;
        p.solved_this_level = false;
    }
}

/// Win detection, the flourish, and progress saving.
struct RulesSystem;

impl System for RulesSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // Keep the marker sprites on their cells (they move with a new level or an undo).
        let Some((player_e, exit_e, cursor_e, player_cell, exit_cell, cursor)) =
            world.resource::<Puzzle>().map(|p| {
                (
                    p.player_entity,
                    p.exit_entity,
                    p.cursor_entity,
                    IVec2::new(p.board.player.0, p.board.player.1),
                    IVec2::new(p.board.exit.0, p.board.exit.1),
                    p.cursor,
                )
            })
        else {
            return;
        };
        for (e, cell) in [
            (player_e, player_cell),
            (exit_e, exit_cell),
            (cursor_e, cursor),
        ] {
            if let Some(t) = world.get_mut::<Transform>(e) {
                t.position = cell_to_world(cell);
            }
        }

        // Advance the flourish if one is running.
        let finished = {
            let Some(p) = world.resource_mut::<Puzzle>() else {
                return;
            };
            match &mut p.flash {
                Some(tween) => {
                    tween.tick(dt);
                    tween.finished()
                }
                None => false,
            }
        };
        if finished {
            if let Some(p) = world.resource_mut::<Puzzle>() {
                p.flash = None;
            }
        }

        let (solved_now, already) = world
            .resource::<Puzzle>()
            .map(|p| (p.board.exit_visible(), p.solved_this_level))
            .unwrap_or((false, true));
        if !solved_now || already {
            return;
        }

        if let Some(p) = world.resource_mut::<Puzzle>() {
            p.solved_this_level = true;
            p.progress.solved += 1;
            p.progress.seed = p.seed;
            // The flourish: a value the HUD reads to brighten the exit. A `Tween` rather than a
            // hand-rolled timer because the easing is the point of the flourish.
            p.flash = Some(Tween::new(1.0, 0.0, 0.9).with_easing(Easing::EaseOutBounce));
        }
        let progress = world.resource::<Puzzle>().map(|p| p.progress.clone());
        if let Some(progress) = progress {
            let _ = engine::save::save(&save_file(), &progress);
        }

        // A coroutine so the "next level" beat is a sequence in time rather than a flag somebody
        // has to remember to clear.
        if let Some(runner) = world.resource_mut::<CoroutineRunner>() {
            runner.start(Coroutine::new().wait(1.1).run(|world| {
                let (next, seed) = world
                    .resource::<Puzzle>()
                    .map(|p| (p.generator.next(), p.seed.wrapping_add(1)))
                    .unwrap_or((Gen::Rooms, 1));
                new_level(world, next, seed);
            }));
        }
    }

    fn name(&self) -> &'static str {
        "RulesSystem"
    }
}

/// HUD + the optional FOV overlay.
struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((generator, seed, board, solved, show_fov, flash, undo_depth, can_redo)) =
            world.resource::<Puzzle>().map(|p| {
                (
                    p.generator,
                    p.seed,
                    p.board.clone(),
                    p.progress.solved,
                    p.show_fov,
                    p.flash.as_ref().map(|t| t.value()).unwrap_or(0.0),
                    p.history.undo_depth(),
                    p.history.can_redo(),
                )
            })
        else {
            return;
        };

        if show_fov {
            // Immediate-mode: pushed fresh every frame, converted at the render stage. Nothing to
            // clean up, which is why it suits an overlay that changes with every cursor move.
            let fov = board.fov();
            if let Some(dd) = world.resource_mut::<DebugDraw>() {
                for y in 0..BOARD_H {
                    for x in 0..BOARD_W {
                        if fov.is_visible(x, y) {
                            // `rect` takes **min and max**, not min and size — passing a size
                            // draws a box from the cell to the near corner of the world instead,
                            // which is a rectangle, just not the one you meant.
                            let at = cell_to_world(IVec2::new(x, y)) - Vec2::splat(TILE * 0.5);
                            dd.rect_filled_z(at, at + Vec2::splat(TILE), [90, 200, 255, 70], 0.8);
                        }
                    }
                }
            }
        }

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        tq.push(DrawText::new(
            "arrows move cursor   Space carve/fill   Z undo   Y redo   Tab new board   F show sight   Esc quit",
            Vec2::new(16.0, 14.0),
            16.0,
            [222, 234, 248, 230],
        ));
        tq.push(DrawText::new(
            format!(
                "{}   seed {seed:#x}   edits left {}   undo {undo_depth}{}   solved {solved}{}",
                generator.label(),
                board.edits_left,
                if can_redo { " (redo available)" } else { "" },
                if flash > 0.0 { "   SOLVED" } else { "" },
            ),
            Vec2::new(16.0, 38.0),
            16.0,
            [170, 210, 255, 220],
        ));
        tq.push(DrawText::new(
            "make the exit visible from where you stand",
            Vec2::new(16.0, 60.0),
            14.0,
            [150, 180, 210, 190],
        ));
    }

    fn name(&self) -> &'static str {
        "HudSystem"
    }
}

// ── Setup ───────────────────────────────────────────────────────────────────────────────────────

fn seed_from_env() -> u64 {
    std::env::var("PUZZLE_GRID_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0xC0FFEE)
}

fn build_app() -> App {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — puzzle_grid_game".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });
    app.load_atlas(TILES_PATH, 8, 4);

    // Two 16-cell edge strips: wall at atlas index 0, floor at 16. Each terrain autotiles against
    // its own kind only, which is what makes a carved corridor read as a corridor.
    let map_entity = app.world.spawn();
    app.world.add_component(
        map_entity,
        Tilemap::new(
            TilemapAtlas::new(TILES_PATH, 8, 4),
            vec![vec![WALL; BOARD_W as usize]; BOARD_H as usize],
            TILE,
            ORIGIN,
        ),
    );
    app.world.add_component(
        map_entity,
        TilemapAutotile::multi_edge_16(&[(WALL, 0), (FLOOR, 16)]).with_oob_filled(true),
    );

    let marker = |app: &mut App, color: (f32, f32, f32), z: f32| -> Entity {
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: Vec2::ZERO,
                scale: Vec2::splat(TILE * 0.7),
                z,
                ..Default::default()
            },
        );
        app.world
            .add_component(e, Sprite::colored(color.0, color.1, color.2));
        e
    };
    let player_entity = marker(&mut app, (0.55, 0.85, 1.0), 0.5);
    let exit_entity = marker(&mut app, (0.45, 0.95, 0.55), 0.5);
    let cursor_entity = marker(&mut app, (1.0, 0.85, 0.35), 0.4);

    let seed = seed_from_env();
    let progress = engine::save::load_or_default::<Progress>(&save_file()).unwrap_or_default();
    app.world.insert_resource(Puzzle {
        map_entity,
        cursor_entity,
        exit_entity,
        player_entity,
        board: Board {
            tiles: vec![WALL; (BOARD_W * BOARD_H) as usize],
            player: (1, 1),
            exit: (1, 1),
            edits_left: BUDGET,
        },
        history: History::new(),
        generator: Gen::Rooms,
        seed,
        cursor: IVec2::new(1, 1),
        progress,
        solved_this_level: false,
        flash: None,
        show_fov: false,
    });
    app.world.insert_resource(CoroutineRunner::new());
    new_level(&mut app.world, Gen::Rooms, seed);

    app.add_system_labeled(InputSystem, SystemConfig::new().label(L_INPUT));
    app.add_system_labeled(
        RulesSystem,
        SystemConfig::new().label(L_RULES).after(L_INPUT),
    );
    app.add_system_labeled(
        TilemapSystem::new(),
        SystemConfig::new()
            .label(TilemapSystem::LABEL)
            .after(L_RULES),
    );
    app.add_system(CoroutineSystem);
    app.add_system(HudSystem);
    app
}

// The engine reports trouble through `log`, which discards everything until a binary installs
// a logger. Every game installs the same one; the module explains what that buys and what it
// still does not cover in a browser.
#[path = "../shared/logging.rs"]
mod logging;

fn main() {
    logging::init();
    if std::env::var("PUZZLE_GRID_GEN_ASSETS").is_ok() {
        generate_tileset(TILES_PATH);
        return;
    }
    if std::env::var("PUZZLE_GRID_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }
    build_app().run();
}

// ── Asset generation ────────────────────────────────────────────────────────────────────────────

/// Writes `assets/tiles.png`: an 8×4 sheet of 32 px cells holding **two** 16-cell edge-autotile
/// strips — wall at index 0, floor at index 16 — which is the layout `multi_edge_16` indexes into.
/// Within each strip the cell index is the edge-4 neighbour mask (N=1, E=2, S=4, W=8), and the open
/// sides (bits *clear*) get the lit rim.
///
/// Deterministic: the speckle is a pure hash of the pixel coordinate.
fn generate_tileset(path: &str) {
    const CELL: u32 = 32;
    const RIM: u32 = 3;

    fn hash(x: u32, y: u32, salt: u32) -> u32 {
        let mut v = x
            .wrapping_mul(374_761_393)
            .wrapping_add(y.wrapping_mul(668_265_263))
            ^ salt.wrapping_mul(2_246_822_519);
        v ^= v >> 13;
        v = v.wrapping_mul(1_274_126_177);
        (v ^ (v >> 16)) & 0xff
    }

    let mut img = image::RgbaImage::new(CELL * 8, CELL * 4);
    for index in 0u32..32 {
        let (ox, oy) = (index % 8 * CELL, index / 8 * CELL);
        let mask = index % 16;
        let is_floor = index >= 16;
        let (open_n, open_e, open_s, open_w) =
            (mask & 1 == 0, mask & 2 == 0, mask & 4 == 0, mask & 8 == 0);
        for y in 0..CELL {
            for x in 0..CELL {
                let speck = hash(x, y, if is_floor { 11 } else { 3 });
                let base = if is_floor {
                    match speck {
                        n if n < 30 => [0x2c, 0x36, 0x3f, 0xff],
                        n if n > 230 => [0x3f, 0x4c, 0x59, 0xff],
                        _ => [0x35, 0x41, 0x4c, 0xff],
                    }
                } else {
                    match speck {
                        n if n < 30 => [0x5a, 0x50, 0x46, 0xff],
                        n if n > 230 => [0x7d, 0x71, 0x63, 0xff],
                        _ => [0x6a, 0x5f, 0x54, 0xff],
                    }
                };
                let rim = if is_floor {
                    [0x22, 0x2a, 0x33, 0xff]
                } else {
                    [0x94, 0x87, 0x76, 0xff]
                };
                let on_open_edge = (open_n && y < RIM)
                    || (open_s && y + RIM >= CELL)
                    || (open_w && x < RIM)
                    || (open_e && x + RIM >= CELL);
                img.put_pixel(
                    ox + x,
                    oy + y,
                    image::Rgba(if on_open_edge { rim } else { base }),
                );
            }
        }
    }
    img.save(path)
        .unwrap_or_else(|e| panic!("could not write {path}: {e}"));
    println!("wrote {path} — two 16-cell edge strips (wall @0, floor @16), {CELL} px cells");
}

// ── Acceptance test ─────────────────────────────────────────────────────────────────────────────
//
// `PUZZLE_GRID_SELFTEST=1 cargo run --example puzzle_grid_game`, and `scripts/selftests.sh`.
//
// Exit codes: 0 pass · 1 undo does not restore the board · 2 the autotile display is not restored
// with it · 3 a generator produces a disconnected map · 4 `FovMap` and `PathGrid` disagree ·
// 5 carving does not change what is visible · 6 progress does not round-trip · 7 the solve
// sequence does not fire, or fires twice.

const DT: f32 = 1.0 / 60.0;

fn puzzle(app: &App) -> &Puzzle {
    app.world.resource::<Puzzle>().expect("Puzzle resource")
}

fn step(app: &mut App, frames: u32) {
    for _ in 0..frames {
        app.step_headless(DT);
    }
}

/// The tilemap's own tile grid — the *rendered* board, as opposed to the game's `Board`.
fn tilemap_rows(app: &App) -> Vec<Vec<u32>> {
    app.world
        .get::<Tilemap>(puzzle(app).map_entity)
        .map(|t| t.tiles.clone())
        .unwrap_or_default()
}

/// The edge-4 autotile mask every cell of `terrain` would display, in row-major order. This is
/// what `TilemapSystem` derives each frame, so it is the observable for "the render came back".
fn autotile_masks(rows: &[Vec<u32>], terrain: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(rows.len() * rows[0].len());
    for r in 0..rows.len() {
        for c in 0..rows[r].len() {
            out.push(if rows[r][c] == terrain {
                engine::compute_tile_mask_typed(
                    rows,
                    r,
                    c,
                    engine::Neighborhood::Edge4,
                    true,
                    terrain,
                )
            } else {
                0
            });
        }
    }
    out
}

/// Flood-fills the floor from one cell and reports how many floor cells it reached.
fn reachable_floor(map: &DungeonMap) -> (usize, usize) {
    let mut floors = Vec::new();
    for y in 0..BOARD_H {
        for x in 0..BOARD_W {
            if map.is_floor(x, y) {
                floors.push((x, y));
            }
        }
    }
    if floors.is_empty() {
        return (0, 0);
    }
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![floors[0]];
    seen.insert(floors[0]);
    while let Some((x, y)) = stack.pop() {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = (x + dx, y + dy);
            if n.0 < 0 || n.1 < 0 || n.0 >= BOARD_W || n.1 >= BOARD_H {
                continue;
            }
            if map.is_floor(n.0, n.1) && seen.insert(n) {
                stack.push(n);
            }
        }
    }
    (seen.len(), floors.len())
}

/// Moves the cursor to a cell and presses `Space`, through the game's own edit path.
fn edit_at(app: &mut App, cell: IVec2) {
    if let Some(p) = app.world.resource_mut::<Puzzle>() {
        p.cursor = cell;
    }
    apply_edit(&mut app.world);
}

fn self_test() -> i32 {
    // ── 1. Undo restores the exact prior board ─────────────────────────────────────────────────
    //
    // The comparison is the whole `Board`, not a win flag: a flag can be false again for a dozen
    // reasons and would pass on an undo that restored nothing at all.
    {
        let mut app = build_app();
        step(&mut app, 2);
        let before = puzzle(&app).board.clone();
        let rows_before = tilemap_rows(&app);

        // Three edits at cells the rules allow (not the player's, not the exit's).
        let cells = [IVec2::new(4, 4), IVec2::new(5, 4), IVec2::new(6, 4)];
        for c in cells {
            edit_at(&mut app, c);
        }
        let after_edits = puzzle(&app).board.clone();
        if after_edits == before {
            eprintln!(
                "FAIL: three edits changed nothing — the edit path is not reaching the board"
            );
            return 1;
        }

        for _ in 0..cells.len() {
            step_history(&mut app.world, true);
        }
        let restored = puzzle(&app).board.clone();
        let rows_restored = tilemap_rows(&app);

        if restored != before {
            let diff = restored
                .tiles
                .iter()
                .zip(before.tiles.iter())
                .filter(|(a, b)| a != b)
                .count();
            eprintln!(
                "FAIL: undo did not restore the board — {diff} cells differ, edits_left {} vs {}",
                restored.edits_left, before.edits_left
            );
            return 1;
        }
        if rows_restored != rows_before {
            eprintln!("FAIL: the board came back but the tilemap did not — the render is stale");
            return 1;
        }

        // …and redo returns to the edited state. Without this half, an `undo` that simply cleared
        // the history would pass.
        for _ in 0..cells.len() {
            step_history(&mut app.world, false);
        }
        if puzzle(&app).board != after_edits {
            eprintln!("FAIL: redo did not return to the edited board");
            return 1;
        }
        println!(
            "ok: {} edits undo to the exact prior board (tilemap included) and redo returns to the \
             edited one",
            cells.len()
        );
    }

    // ── 2. The autotile display is restored with the board ─────────────────────────────────────
    //
    // The plan's reason for this game: a wrong tile index after an undo is a *visible* difference.
    // The values coming back is not enough — the display index is derived from each cell's
    // neighbours, so a restore that skipped the re-derivation leaves the right board drawn wrong.
    {
        let mut app = build_app();
        step(&mut app, 2);
        let masks_before = autotile_masks(&tilemap_rows(&app), FLOOR);

        edit_at(&mut app, IVec2::new(8, 6));
        let masks_edited = autotile_masks(&tilemap_rows(&app), FLOOR);
        if masks_edited == masks_before {
            eprintln!(
                "FAIL: one edit changed no autotile mask at all — this check cannot tell a restored \
                 render from a stale one"
            );
            return 2;
        }

        step_history(&mut app.world, true);
        let masks_restored = autotile_masks(&tilemap_rows(&app), FLOOR);
        if masks_restored != masks_before {
            let diff = masks_restored
                .iter()
                .zip(masks_before.iter())
                .filter(|(a, b)| a != b)
                .count();
            eprintln!("FAIL: undo left {diff} cells displaying the wrong autotile index");
            return 2;
        }
        let changed = masks_edited
            .iter()
            .zip(masks_before.iter())
            .filter(|(a, b)| a != b)
            .count();
        println!(
            "ok: one edit moves {changed} autotile masks and undo puts every one of them back"
        );
    }

    // ── 3. All three generators produce connected maps ─────────────────────────────────────────
    //
    // One property, three independent mechanisms: BSP carves corridors between leaves, the cave
    // keeps only the largest region after smoothing, the maze carves a spanning tree. A regression
    // in any one of them is a level the player cannot finish.
    {
        const SEEDS: u64 = 12;
        for generator in [Gen::Rooms, Gen::Cave, Gen::Maze] {
            for seed in 0..SEEDS {
                let map = generator.build(seed * 7919 + 13);
                let (reached, total) = reachable_floor(&map);
                if total == 0 {
                    eprintln!(
                        "FAIL: {} produced a map with no floor at all (seed index {seed})",
                        generator.label()
                    );
                    return 3;
                }
                if reached != total {
                    eprintln!(
                        "FAIL: {} produced a disconnected map at seed index {seed} — {reached} of \
                         {total} floor cells reachable from the first one",
                        generator.label()
                    );
                    return 3;
                }
            }
        }
        println!("ok: all three generators are connected across {SEEDS} seeds each");
    }

    // ── 4. The `PathGrid` → `FovMap` bridge ────────────────────────────────────────────────────
    //
    // `FovMap::from_path_grid` has exactly one contract: **an unwalkable cell becomes an opaque
    // cell.** That is the bridge, and it is checkable cell for cell.
    //
    // ⚠️ It is *not* checkable by comparing `compute` against `line_of_sight`, which is what the
    // first version of this check did — 9 of 54 visible cells failed the line test and it looked
    // like an engine bug. The docs say plainly that the two are independent: `compute` is recursive
    // shadowcasting over 8 octants ("symmetric-ish"), `line_of_sight` is a single Bresenham walk,
    // and a shadowcast visible set famously does not agree cell-for-cell with a line test. Reading
    // the contract before filing the bug is the rule this repo already has; this is another
    // instance of it.
    {
        let mut app = build_app();
        step(&mut app, 2);
        let board = puzzle(&app).board.clone();
        let grid = board.path_grid();
        let fov = FovMap::from_path_grid(&grid);

        let mut mismatches = 0;
        let mut opaque = 0;
        for y in 0..BOARD_H {
            for x in 0..BOARD_W {
                let walkable = grid.is_walkable(x, y);
                if fov.is_opaque(x, y) {
                    opaque += 1;
                }
                if fov.is_opaque(x, y) == walkable {
                    mismatches += 1;
                }
            }
        }
        if mismatches > 0 || opaque == 0 || opaque == BOARD_W * BOARD_H {
            eprintln!(
                "FAIL: the PathGrid -> FovMap bridge is broken — {mismatches} cells whose opacity \
                 does not mirror walkability, {opaque} of {} opaque (want a mix, or the board is \
                 uniform and this proves nothing)",
                BOARD_W * BOARD_H
            );
            return 4;
        }

        // …and the opacity is actually used: on a hand-built corridor the wall is seen and the cell
        // behind it is not. Without this half, a `FovMap` that ignored its own opaque grid would
        // pass the mirror check above.
        let mut fov = FovMap::new(9, 9);
        fov.set_opaque(4, 3, true);
        fov.compute(IVec2::new(4, 5), 6);
        if !fov.is_visible(4, 3) || fov.is_visible(4, 2) {
            eprintln!(
                "FAIL: shadowcasting is not using the opaque grid — the wall at (4,3) is visible: \
                 {}, the cell behind it at (4,2) is visible: {} (want true, false)",
                fov.is_visible(4, 3),
                fov.is_visible(4, 2)
            );
            return 4;
        }
        println!(
            "ok: every cell's opacity mirrors its walkability ({opaque} opaque of {}), and a wall \
             shadows what is behind it",
            BOARD_W * BOARD_H
        );
    }

    // ── 5. Carving changes what is visible ─────────────────────────────────────────────────────
    //
    // Two-sided by construction: a wall between the player and a target hides it, and carving that
    // wall reveals it. Asserting only the second half would pass on an engine where everything is
    // always visible.
    {
        let mut app = build_app();
        step(&mut app, 2);
        // A hand-built board is used here rather than a generated one: the check needs a *known*
        // wall between two known cells, and hunting for one in a random cave would make the check
        // depend on the generator it is not testing.
        let mut board = Board {
            tiles: vec![WALL; (BOARD_W * BOARD_H) as usize],
            player: (2, 2),
            exit: (6, 2),
            edits_left: BUDGET,
        };
        for x in 2..=6 {
            board.set(x, 2, FLOOR);
        }
        board.set(4, 2, WALL); // the one wall in the corridor
        if board.exit_visible() {
            eprintln!("FAIL: the exit is visible through a wall — sight is not being blocked");
            return 5;
        }
        board.set(4, 2, FLOOR);
        if !board.exit_visible() {
            eprintln!(
                "FAIL: carving the only wall between player and exit did not reveal it — the FOV \
                 is not recomputed from the edited board"
            );
            return 5;
        }
        println!("ok: a wall hides the exit and carving it reveals it");
    }

    // ── 6. Progress round-trips ────────────────────────────────────────────────────────────────
    {
        let path = save_file();
        let want = Progress {
            solved: 7,
            seed: 0xDEAD_BEEF,
        };
        if let Err(e) = engine::save::save(&path, &want) {
            eprintln!("FAIL: could not write progress: {e}");
            return 6;
        }
        let got = match engine::save::load::<Progress>(&path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("FAIL: could not read progress back: {e}");
                return 6;
            }
        };
        let _ = engine::save::delete(&path);
        if got != want {
            eprintln!("FAIL: progress did not round-trip — {got:?} vs {want:?}");
            return 6;
        }
        println!(
            "ok: progress round-trips through the save file ({} solved)",
            got.solved
        );
    }

    // ── 7. Solving fires the sequence exactly once ─────────────────────────────────────────────
    //
    // The flourish is a `Tween` a `Coroutine` waits on, and the level advances when it ends. Both
    // halves matter: a solve that never fires leaves the player stuck on a finished board, and one
    // that fires every frame restarts the level under them.
    {
        let mut app = build_app();
        step(&mut app, 2);
        // Put the exit where the player already sees it — solving without needing a specific board.
        if let Some(p) = app.world.resource_mut::<Puzzle>() {
            let player = p.board.player;
            p.board.exit = player;
        }
        step(&mut app, 2);
        let solved_after_one = puzzle(&app).progress.solved;
        let flashing = puzzle(&app).flash.is_some();
        step(&mut app, 20);
        let solved_after_more = puzzle(&app).progress.solved;

        if solved_after_one == 0 || !flashing {
            eprintln!(
                "FAIL: a solved board did not fire the sequence — solved {solved_after_one}, \
                 flourish running: {flashing}"
            );
            return 7;
        }
        if solved_after_more != solved_after_one {
            eprintln!(
                "FAIL: the solve fired again while the board was still solved — {solved_after_one} \
                 -> {solved_after_more}. The level would restart under the player."
            );
            return 7;
        }
        // …and the coroutine eventually hands over a fresh level.
        step(&mut app, 120);
        let fresh =
            puzzle(&app).board.edits_left == BUDGET && puzzle(&app).history.undo_depth() == 0;
        if !fresh {
            eprintln!(
                "FAIL: the coroutine never advanced to a new level — edits_left {}, undo depth {}",
                puzzle(&app).board.edits_left,
                puzzle(&app).history.undo_depth()
            );
            return 7;
        }
        println!(
            "ok: solving fires the flourish once ({solved_after_one} solved, still \
             {solved_after_more} twenty frames later) and the coroutine deals a fresh level"
        );
    }

    0
}
