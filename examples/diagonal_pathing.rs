//! Diagonal pathfinding demo — press **T** to toggle between 4-dir (cardinal) and
//! 8-dir (diagonal) A*. The 4-dir path makes a visible staircase; the 8-dir path cuts
//! straight across. Path length (step count) is shown on screen.
//!
//! Grid: 24×16 cells, 28px per cell. Start = top-left area, goal = bottom-right area.
//! A fixed obstacle layout creates an interesting comparison between the two modes.
use engine::{
    App, Camera, Color, DrawRect, DrawText, InputState, KeyCode, Scene, System, SystemRegistrar,
    TextQueue, UiQueue, ViewportSize, WindowConfig, World,
};
use glam::{IVec2, Vec2};

// ─── Grid constants ───────────────────────────────────────────────────────────

const COLS: i32 = 24;
const ROWS: i32 = 16;
const CELL: f32 = 28.0;

// Pixel origin of the grid (top-left corner), in positive world space.
const GRID_OX: f32 = 20.0;
const GRID_OY: f32 = 20.0;

// Colours
const COLOR_FLOOR: Color = Color::rgba(0.18, 0.18, 0.22, 1.0);
const COLOR_WALL: Color = Color::rgba(0.45, 0.20, 0.20, 1.0);
const COLOR_PATH_4: Color = Color::rgba(0.30, 0.70, 1.00, 0.75); // blue — cardinal
const COLOR_PATH_8: Color = Color::rgba(0.30, 1.00, 0.45, 0.75); // green — diagonal
const COLOR_START: Color = Color::rgba(1.00, 0.85, 0.10, 1.0);
const COLOR_GOAL: Color = Color::rgba(1.00, 0.35, 0.10, 1.0);

// Fixed start / goal cells
const START: IVec2 = IVec2::new(1, 1);
const GOAL: IVec2 = IVec2::new(22, 14);

// ─── Obstacle layout ─────────────────────────────────────────────────────────
// Each tuple is (x_start, y_start, width, height) of a wall rectangle.
// Chosen to force the 4-dir path into a long staircase while the 8-dir path cuts clean.
const WALLS: &[(i32, i32, i32, i32)] = &[
    // Diagonal staircase barrier down the centre
    (5, 0, 1, 5),
    (6, 3, 1, 5),
    (7, 6, 1, 5),
    (8, 9, 1, 5),
    // Horizontal wall near the bottom forcing a small detour
    (10, 12, 7, 1),
    // Small pocket on the right side
    (18, 5, 1, 6),
    (19, 5, 4, 1),
];

fn build_obstacle_set() -> Vec<IVec2> {
    let mut walls = Vec::new();
    for &(x, y, w, h) in WALLS {
        for dy in 0..h {
            for dx in 0..w {
                let cell = IVec2::new(x + dx, y + dy);
                // Never block start or goal
                if cell != START && cell != GOAL {
                    walls.push(cell);
                }
            }
        }
    }
    walls
}

fn build_grid(walls: &[IVec2]) -> engine::pathfinding::PathGrid {
    use engine::pathfinding::PathGrid;
    let mut grid = PathGrid::new(COLS, ROWS);
    for &w in walls {
        grid.set_walkable(w.x, w.y, false);
    }
    grid
}

// ─── Helper: pixel position of a cell's top-left corner ──────────────────────

fn cell_xy(c: IVec2) -> (f32, f32) {
    (GRID_OX + c.x as f32 * CELL, GRID_OY + c.y as f32 * CELL)
}

// ─── Scene ────────────────────────────────────────────────────────────────────

struct DemoScene;

impl Scene for DemoScene {
    fn on_enter(&mut self, _world: &mut World, systems: &mut SystemRegistrar) {
        systems.add(DemoSystem::new());
    }
    fn on_exit(&mut self, _world: &mut World) {}
}

// ─── System ───────────────────────────────────────────────────────────────────

struct DemoSystem {
    diagonal_mode: bool,
    walls: Vec<IVec2>,
    path: Option<Vec<IVec2>>,
    needs_recompute: bool,
}

impl DemoSystem {
    fn new() -> Self {
        Self {
            diagonal_mode: false,
            walls: build_obstacle_set(),
            path: None,
            needs_recompute: true,
        }
    }

    fn recompute(&mut self) {
        use engine::pathfinding::{find_path, find_path_diagonal};
        let grid = build_grid(&self.walls);
        self.path = if self.diagonal_mode {
            find_path_diagonal(&grid, START, GOAL)
        } else {
            find_path(&grid, START, GOAL)
        };
        self.needs_recompute = false;
    }
}

impl System for DemoSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // ── Input ────────────────────────────────────────────────────────────
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::KeyT) {
                self.diagonal_mode = !self.diagonal_mode;
                self.needs_recompute = true;
            }
        }

        if self.needs_recompute {
            self.recompute();
        }

        // ── Draw grid ────────────────────────────────────────────────────────
        if let Some(ui) = world.resource_mut::<UiQueue>() {
            // Grid border / background
            ui.items.push(DrawRect {
                x: GRID_OX - 1.0,
                y: GRID_OY - 1.0,
                w: COLS as f32 * CELL + 2.0,
                h: ROWS as f32 * CELL + 2.0,
                color: Color::rgba(0.08, 0.08, 0.10, 1.0),
                z: 0.0,
            });

            // Individual cells
            for row in 0..ROWS {
                for col in 0..COLS {
                    let cell = IVec2::new(col, row);
                    let is_wall = self.walls.contains(&cell);
                    let color = if is_wall { COLOR_WALL } else { COLOR_FLOOR };
                    let (cx, cy) = cell_xy(cell);
                    ui.items.push(DrawRect {
                        x: cx + 1.0,
                        y: cy + 1.0,
                        w: CELL - 2.0,
                        h: CELL - 2.0,
                        color,
                        z: 0.1,
                    });
                }
            }

            // Path highlight (skip goal — it gets its own colour below)
            let path_color = if self.diagonal_mode {
                COLOR_PATH_8
            } else {
                COLOR_PATH_4
            };
            if let Some(ref path) = self.path {
                for &p in path {
                    if p == GOAL {
                        continue;
                    }
                    let (px, py) = cell_xy(p);
                    ui.items.push(DrawRect {
                        x: px + 2.0,
                        y: py + 2.0,
                        w: CELL - 4.0,
                        h: CELL - 4.0,
                        color: path_color,
                        z: 0.2,
                    });
                }
            }

            // Start marker
            let (sx, sy) = cell_xy(START);
            ui.items.push(DrawRect {
                x: sx + 2.0,
                y: sy + 2.0,
                w: CELL - 4.0,
                h: CELL - 4.0,
                color: COLOR_START,
                z: 0.3,
            });

            // Goal marker
            let (gx, gy) = cell_xy(GOAL);
            ui.items.push(DrawRect {
                x: gx + 2.0,
                y: gy + 2.0,
                w: CELL - 4.0,
                h: CELL - 4.0,
                color: COLOR_GOAL,
                z: 0.3,
            });
        }

        // ── HUD text ─────────────────────────────────────────────────────────
        let hud_y = GRID_OY + ROWS as f32 * CELL + 12.0;
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            let mode_str = if self.diagonal_mode {
                "8-dir diagonal  (find_path_diagonal)"
            } else {
                "4-dir cardinal  (find_path)"
            };
            let steps = self.path.as_ref().map(|p| p.len()).unwrap_or(0);
            let mode_color: Color = if self.diagonal_mode {
                [80u8, 255, 120, 255].into()
            } else {
                [80u8, 180, 255, 255].into()
            };

            tq.push(DrawText::new(
                format!("Mode: {mode_str}"),
                Vec2::new(GRID_OX, hud_y),
                18.0,
                mode_color,
            ));
            tq.push(DrawText::new(
                format!("Path length: {steps} steps      [T] = toggle mode"),
                Vec2::new(GRID_OX, hud_y + 22.0),
                16.0,
                Color::rgba(0.78, 0.78, 0.78, 1.0),
            ));
            tq.push(DrawText::new(
                "Yellow = start   Red = goal   Blue = 4-dir path   Green = 8-dir path",
                Vec2::new(GRID_OX, hud_y + 44.0),
                13.0,
                Color::rgba(0.60, 0.60, 0.60, 1.0),
            ));
        }

        // Suppress unused-variable warning for vw/vh if ViewportSize isn't needed
        let _ = world.resource::<ViewportSize>();
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();

    // Window sized to show the full 24×16 grid plus HUD text below it
    let win_w = (COLS as f32 * CELL + GRID_OX * 2.0) as u32 + 20;
    let win_h = (ROWS as f32 * CELL + GRID_OY * 2.0) as u32 + 90;
    app.world.insert_resource(WindowConfig {
        title: "Diagonal Pathfinding Demo — [T] toggle mode".to_string(),
        width: win_w,
        height: win_h,
        clear_color: [0.05, 0.05, 0.07, 1.0],
    });

    // Camera at (0,0) — top-left, grid starts at positive coords
    if let Some(cam) = app.world.resource_mut::<Camera>() {
        cam.position = Vec2::ZERO;
    }

    app.set_scene(Box::new(DemoScene));
    app.run();
}
