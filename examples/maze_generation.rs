//! Procedural maze generation — a perfect (spanning-tree) maze you can regenerate, braid, and walk.
//!
//! [`generate_maze`] runs a recursive-backtracker walk over the odd-coordinate junction cells,
//! knocking out the wall between each junction and a random unvisited neighbor. The result is a
//! **spanning tree** over the junctions: every cell is reachable and exactly one path connects any
//! two — a *perfect* maze, guaranteed connected by construction. Generation is deterministic from the
//! seed, so **R** rolls a fresh seed and grows a new maze instantly. **B** *braids* the maze —
//! reopening dead-end walls into loops (`MazeParams::braid_chance`); braiding only removes walls, so
//! the maze stays connected. Dead-end tips (the maze tree's leaves, which braiding thins out) are
//! tinted warm so the perfect ↔ braided difference reads at a glance.
//!
//! - **WASD / arrows** — walk the yellow explorer (walls block)
//! - **R** — regenerate with the next seed
//! - **B** — toggle braiding (perfect maze ↔ braided, same seed)
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/maze.png cargo run --example maze_generation`): first self-checks
//! that the maze is a single connected region (exits non-zero if not), then captures the starting
//! maze. `HEADLESS_FRAMES=N` overrides the default 6 frames.
use engine::{
    generate_maze, App, Camera, Color, DrawRect, DrawText, DungeonMap, InputState, KeyCode,
    MazeParams, Scene, ShouldQuit, System, SystemRegistrar, TextQueue, UiQueue, WindowConfig,
    World,
};
use glam::{IVec2, Vec2};

const COLS: i32 = 45;
const ROWS: i32 = 29;
const CELL: f32 = 15.0;
const GRID_OX: f32 = 14.0;
const GRID_OY: f32 = 14.0;
/// How aggressively **B** braids the maze (fraction of dead ends reopened into loops).
const BRAID_CHANCE: f32 = 0.55;

const BACKDROP: Color = Color::rgba(0.05, 0.05, 0.07, 1.0); // walls
const PASSAGE: Color = Color::rgba(0.28, 0.32, 0.38, 1.0); // open corridor
const DEAD_END: Color = Color::rgba(0.52, 0.40, 0.22, 1.0); // a dead-end tip (a tree leaf)
const PLAYER: Color = Color::rgba(1.0, 0.85, 0.22, 1.0);

fn cell_xy(x: i32, y: i32) -> (f32, f32) {
    (GRID_OX + x as f32 * CELL, GRID_OY + y as f32 * CELL)
}

/// Generates a maze (optionally braided) and its spawn cell (the start junction).
fn make(seed: u64, braid: bool) -> (DungeonMap, IVec2) {
    let params = MazeParams {
        braid_chance: if braid { BRAID_CHANCE } else { 0.0 },
    };
    let map = generate_maze(COLS, ROWS, seed, &params);
    let spawn = map.first_room_center().unwrap_or(IVec2::new(1, 1));
    (map, spawn)
}

/// Whether `(x, y)` is a floor cell with exactly one orthogonal floor neighbor — a dead-end tip.
fn is_dead_end(map: &DungeonMap, x: i32, y: i32) -> bool {
    if !map.is_floor(x, y) {
        return false;
    }
    let n = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .filter(|(dx, dy)| map.is_floor(x + dx, y + dy))
        .count();
    n == 1
}

struct MazeScene;

impl Scene for MazeScene {
    fn on_enter(&mut self, _world: &mut World, systems: &mut SystemRegistrar) {
        systems.add(Demo::new(1));
    }
    fn on_exit(&mut self, _world: &mut World) {}
}

struct Demo {
    map: DungeonMap,
    player: IVec2,
    seed: u64,
    braid: bool,
}

impl Demo {
    fn new(seed: u64) -> Self {
        let braid = false;
        let (map, player) = make(seed, braid);
        Self {
            map,
            player,
            seed,
            braid,
        }
    }

    fn rebuild(&mut self) {
        let (map, player) = make(self.seed, self.braid);
        self.map = map;
        self.player = player;
    }

    fn regenerate(&mut self) {
        self.seed = self.seed.wrapping_add(1);
        self.rebuild();
    }

    fn toggle_braid(&mut self) {
        self.braid = !self.braid;
        self.rebuild();
    }

    fn try_move(&mut self, dir: IVec2) {
        let dest = self.player + dir;
        if self.map.is_floor(dest.x, dest.y) {
            self.player = dest;
        }
    }

    fn counts(&self) -> (usize, usize) {
        let (mut floor, mut dead) = (0, 0);
        for y in 0..ROWS {
            for x in 0..COLS {
                if self.map.is_floor(x, y) {
                    floor += 1;
                    if is_dead_end(&self.map, x, y) {
                        dead += 1;
                    }
                }
            }
        }
        (floor, dead)
    }
}

impl System for Demo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // ── Input (gather, then act after the InputState borrow drops) ─────────
        let (mut mv, mut regen, mut braid, mut quit) = (IVec2::ZERO, false, false, false);
        if let Some(input) = world.resource::<InputState>() {
            quit = input.just_pressed(KeyCode::Escape);
            regen = input.just_pressed(KeyCode::KeyR);
            braid = input.just_pressed(KeyCode::KeyB);
            if input.just_pressed(KeyCode::KeyW) || input.just_pressed(KeyCode::ArrowUp) {
                mv.y -= 1;
            }
            if input.just_pressed(KeyCode::KeyS) || input.just_pressed(KeyCode::ArrowDown) {
                mv.y += 1;
            }
            if input.just_pressed(KeyCode::KeyA) || input.just_pressed(KeyCode::ArrowLeft) {
                mv.x -= 1;
            }
            if input.just_pressed(KeyCode::KeyD) || input.just_pressed(KeyCode::ArrowRight) {
                mv.x += 1;
            }
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if regen {
            self.regenerate();
        }
        if braid {
            self.toggle_braid();
        }
        if mv != IVec2::ZERO {
            self.try_move(mv);
        }

        // ── Render: dark wall backdrop, floor cells on top (dead-end tips warm) ─
        if let Some(ui) = world.resource_mut::<UiQueue>() {
            ui.items.push(
                DrawRect::new(
                    GRID_OX,
                    GRID_OY,
                    COLS as f32 * CELL,
                    ROWS as f32 * CELL,
                    BACKDROP,
                )
                .with_z(0.0),
            );
            for y in 0..ROWS {
                for x in 0..COLS {
                    if !self.map.is_floor(x, y) {
                        continue; // wall → the dark backdrop shows through
                    }
                    let color = if is_dead_end(&self.map, x, y) {
                        DEAD_END
                    } else {
                        PASSAGE
                    };
                    let (px, py) = cell_xy(x, y);
                    ui.items
                        .push(DrawRect::new(px, py, CELL - 1.0, CELL - 1.0, color).with_z(0.1));
                }
            }
            let (px, py) = cell_xy(self.player.x, self.player.y);
            ui.items.push(
                DrawRect::new(px + 2.0, py + 2.0, CELL - 4.0, CELL - 4.0, PLAYER).with_z(0.2),
            );
        }

        // ── HUD ────────────────────────────────────────────────────────────────
        let hud_y = GRID_OY + ROWS as f32 * CELL + 10.0;
        let (floor, dead) = self.counts();
        let mode = if self.braid { "braided" } else { "perfect" };
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!(
                    "Recursive-backtracker maze — seed {} · {mode} · {floor} floor · {dead} dead ends",
                    self.seed
                ),
                Vec2::new(GRID_OX, hud_y),
                17.0,
                Color::rgb(0.86, 0.9, 0.88),
            ));
            tq.push(DrawText::new(
                "WASD / arrows: walk    R: regenerate    B: braid on/off    Esc: quit",
                Vec2::new(GRID_OX, hud_y + 22.0),
                14.0,
                Color::rgb(0.66, 0.7, 0.72),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "maze_generation"
    }
}

/// Headless acceptance check: flood-fill from the spawn and confirm every floor cell is reached —
/// i.e. the maze is a single connected region. Returns `(reached, total_floor)`.
fn connectivity(map: &DungeonMap) -> (usize, usize) {
    let mut total = 0;
    for y in 0..ROWS {
        for x in 0..COLS {
            if map.is_floor(x, y) {
                total += 1;
            }
        }
    }
    let Some(spawn) = map.first_room_center() else {
        return (0, total);
    };
    let idx = |x: i32, y: i32| (y * COLS + x) as usize;
    let mut seen = vec![false; (COLS * ROWS) as usize];
    let mut q = std::collections::VecDeque::new();
    seen[idx(spawn.x, spawn.y)] = true;
    q.push_back(spawn);
    let mut reached = 0;
    while let Some(p) = q.pop_front() {
        reached += 1;
        for d in [
            IVec2::new(1, 0),
            IVec2::new(-1, 0),
            IVec2::new(0, 1),
            IVec2::new(0, -1),
        ] {
            let n = p + d;
            if map.is_floor(n.x, n.y) && !seen[idx(n.x, n.y)] {
                seen[idx(n.x, n.y)] = true;
                q.push_back(n);
            }
        }
    }
    (reached, total)
}

fn main() {
    let mut app = App::new();
    let win_w = (COLS as f32 * CELL + GRID_OX * 2.0) as u32;
    let win_h = (ROWS as f32 * CELL + GRID_OY * 2.0) as u32 + 70;
    app.world.insert_resource(WindowConfig {
        title: "Procedural maze — R regenerate, B braid".to_string(),
        width: win_w,
        height: win_h,
        clear_color: [0.03, 0.03, 0.05, 1.0],
    });
    if let Some(cam) = app.world.resource_mut::<Camera>() {
        cam.position = Vec2::ZERO;
    }

    app.set_scene(Box::new(MazeScene));

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        // Self-check the defining invariant before capturing: a single connected region.
        let (map, spawn) = make(1, false);
        let (reached, total) = connectivity(&map);
        if total == 0 || reached != total {
            eprintln!(
                "FAIL: maze is not a single connected region — reached {reached}/{total} floor cells"
            );
            std::process::exit(1);
        }
        println!(
            "OK: recursive-backtracker maze is one connected region — {total} floor cells, spawn at ({}, {})",
            spawn.x, spawn.y
        );

        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(6);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
