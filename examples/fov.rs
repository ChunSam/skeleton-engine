//! Field-of-view / fog-of-war demo — walk a torch-lit observer around a walled dungeon and watch
//! walls cast shadows in real time.
//!
//! [`FovMap`] runs recursive shadowcasting each time the observer moves: cells in the current line
//! of sight render **bright**, cells you have explored but can no longer see render **dim** (the
//! fog-of-war memory), and cells you have never seen stay **black**. Gems are only revealed when
//! they fall inside your sight — the classic roguelike "discover the room" feel.
//!
//! - **WASD / arrows** — move one cell (walls block movement and sight)
//! - **+ / -** — grow / shrink the sight radius
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/fov.png cargo run --example fov`): captures the starting field of
//! view — a lit pocket around the observer, the rest of the dungeon in fog. `HEADLESS_FRAMES=N`
//! overrides the default 6 frames.
use engine::{
    App, Camera, Color, DrawRect, DrawText, FovMap, InputState, KeyCode, Scene, ShouldQuit, System,
    SystemRegistrar, TextQueue, UiQueue, WindowConfig, World,
};
use glam::{IVec2, Vec2};

// ─── Dungeon map ────────────────────────────────────────────────────────────────
// '#' = wall, '.' = floor, '@' = observer start, '*' = gem. Rows may be any length —
// missing / out-of-bounds cells are treated as wall, so a ragged row is harmless.
const MAP: &[&str] = &[
    "##############################",
    "#........#...........#.......#",
    "#..*.....#....*......#...*...#",
    "#........#...........#.......#",
    "#........#....####...........#",
    "#####.####..........#####.####",
    "#..........#.........#.......#",
    "#....@.....#....*....#.......#",
    "#..........#.........#...*...#",
    "#..........#.........#.......#",
    "####.#######.#######.####.####",
    "#........#..........#........#",
    "#..*.....#..........#....*...#",
    "#........#....##.....#.......#",
    "#........#....##.....#.......#",
    "#........#..........#........#",
    "#........#..........#........#",
    "##############################",
];

const CELL: f32 = 26.0;
const GRID_OX: f32 = 16.0;
const GRID_OY: f32 = 16.0;
const DEFAULT_RADIUS: i32 = 7;

// Lit / dim (explored-but-unseen) colours, precomputed so rendering does no per-cell math.
const FLOOR_LIT: Color = Color::rgba(0.30, 0.31, 0.37, 1.0);
const FLOOR_DIM: Color = Color::rgba(0.10, 0.10, 0.13, 1.0);
const WALL_LIT: Color = Color::rgba(0.56, 0.43, 0.30, 1.0);
const WALL_DIM: Color = Color::rgba(0.19, 0.15, 0.11, 1.0);
const GEM_LIT: Color = Color::rgba(0.32, 0.90, 0.96, 1.0);
const GEM_DIM: Color = Color::rgba(0.13, 0.30, 0.33, 1.0);
const PLAYER: Color = Color::rgba(1.0, 0.85, 0.22, 1.0);

fn cell_xy(c: IVec2) -> (f32, f32) {
    (GRID_OX + c.x as f32 * CELL, GRID_OY + c.y as f32 * CELL)
}

/// Parses the ASCII map into an opaque [`FovMap`], the observer start, and the gem cells.
fn parse_map() -> (FovMap, IVec2, Vec<IVec2>) {
    let height = MAP.len() as i32;
    let width = MAP.iter().map(|r| r.len()).max().unwrap_or(0) as i32;
    let mut fov = FovMap::new(width, height);
    let mut start = IVec2::ZERO;
    let mut gems = Vec::new();
    for (y, row) in MAP.iter().enumerate() {
        for (x, ch) in row.bytes().enumerate() {
            let (cx, cy) = (x as i32, y as i32);
            match ch {
                b'#' => fov.set_opaque(cx, cy, true),
                b'@' => start = IVec2::new(cx, cy),
                b'*' => gems.push(IVec2::new(cx, cy)),
                _ => {}
            }
        }
    }
    (fov, start, gems)
}

// ─── Scene ──────────────────────────────────────────────────────────────────────

struct DungeonScene;

impl Scene for DungeonScene {
    fn on_enter(&mut self, _world: &mut World, systems: &mut SystemRegistrar) {
        systems.add(FovDemo::new());
    }
    fn on_exit(&mut self, _world: &mut World) {}
}

// ─── System ───────────────────────────────────────────────────────────────────

struct FovDemo {
    fov: FovMap,
    player: IVec2,
    gems: Vec<IVec2>,
    found: Vec<bool>,
    radius: i32,
    dirty: bool,
}

impl FovDemo {
    fn new() -> Self {
        let (fov, player, gems) = parse_map();
        let found = vec![false; gems.len()];
        Self {
            fov,
            player,
            gems,
            found,
            radius: DEFAULT_RADIUS,
            dirty: true,
        }
    }

    /// Moves the observer by `dir` if the destination is in bounds and not a wall.
    fn try_move(&mut self, dir: IVec2) {
        let dest = self.player + dir;
        if dest.x >= 0
            && dest.y >= 0
            && dest.x < self.fov.width
            && dest.y < self.fov.height
            && !self.fov.is_opaque(dest.x, dest.y)
        {
            self.player = dest;
            self.dirty = true;
        }
    }
}

impl System for FovDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // ── Input (gather intents, then act after the InputState borrow drops) ──
        let (mut mv, mut d_radius, mut quit) = (IVec2::ZERO, 0, false);
        if let Some(input) = world.resource::<InputState>() {
            quit = input.just_pressed(KeyCode::Escape);
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
            if input.just_pressed(KeyCode::Equal) || input.just_pressed(KeyCode::NumpadAdd) {
                d_radius += 1;
            }
            if input.just_pressed(KeyCode::Minus) || input.just_pressed(KeyCode::NumpadSubtract) {
                d_radius -= 1;
            }
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if mv != IVec2::ZERO {
            self.try_move(mv);
        }
        if d_radius != 0 {
            self.radius = (self.radius + d_radius).clamp(0, 20);
            self.dirty = true;
        }

        // ── Recompute the field of view only when something changed ────────────
        if self.dirty {
            self.fov.compute(self.player, self.radius);
            for (i, &gem) in self.gems.iter().enumerate() {
                if self.fov.is_visible(gem.x, gem.y) {
                    self.found[i] = true;
                }
            }
            self.dirty = false;
        }

        // ── Draw the map: bright in sight, dim if explored, black if unseen ────
        if let Some(ui) = world.resource_mut::<UiQueue>() {
            // A dark backdrop over the whole dungeon so never-seen cells read as intentional black
            // (fog) regardless of the window clear colour.
            ui.items.push(
                DrawRect::new(
                    GRID_OX,
                    GRID_OY,
                    self.fov.width as f32 * CELL,
                    self.fov.height as f32 * CELL,
                    Color::rgba(0.04, 0.04, 0.06, 1.0),
                )
                .with_z(0.0),
            );
            for y in 0..self.fov.height {
                for x in 0..self.fov.width {
                    if !self.fov.is_revealed(x, y) {
                        continue; // never seen → leave it black (fog)
                    }
                    let visible = self.fov.is_visible(x, y);
                    let color = match (self.fov.is_opaque(x, y), visible) {
                        (true, true) => WALL_LIT,
                        (true, false) => WALL_DIM,
                        (false, true) => FLOOR_LIT,
                        (false, false) => FLOOR_DIM,
                    };
                    let (px, py) = cell_xy(IVec2::new(x, y));
                    ui.items.push(
                        DrawRect::new(px + 1.0, py + 1.0, CELL - 2.0, CELL - 2.0, color)
                            .with_z(0.1),
                    );
                }
            }

            // Gems: bright in sight, a dim ghost once explored (remembered location).
            for (i, &gem) in self.gems.iter().enumerate() {
                let color = if self.fov.is_visible(gem.x, gem.y) {
                    GEM_LIT
                } else if self.found[i] {
                    GEM_DIM
                } else {
                    continue;
                };
                let (px, py) = cell_xy(gem);
                ui.items.push(
                    DrawRect::new(px + 7.0, py + 7.0, CELL - 14.0, CELL - 14.0, color).with_z(0.2),
                );
            }

            // Observer.
            let (px, py) = cell_xy(self.player);
            ui.items.push(
                DrawRect::new(px + 4.0, py + 4.0, CELL - 8.0, CELL - 8.0, PLAYER).with_z(0.3),
            );
        }

        // ── HUD ────────────────────────────────────────────────────────────────
        let hud_y = GRID_OY + self.fov.height as f32 * CELL + 10.0;
        let found = self.found.iter().filter(|&&f| f).count();
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!(
                    "Field of view — gems discovered: {found} / {}   sight radius: {}",
                    self.gems.len(),
                    self.radius
                ),
                Vec2::new(GRID_OX, hud_y),
                17.0,
                Color::rgb(0.9, 0.88, 0.72),
            ));
            tq.push(DrawText::new(
                "WASD / arrows: move    + / -: sight radius    Esc: quit",
                Vec2::new(GRID_OX, hud_y + 22.0),
                14.0,
                Color::rgb(0.68, 0.7, 0.76),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "fov_demo"
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();

    let cols = MAP.iter().map(|r| r.len()).max().unwrap_or(0) as f32;
    let rows = MAP.len() as f32;
    let win_w = (cols * CELL + GRID_OX * 2.0) as u32;
    let win_h = (rows * CELL + GRID_OY * 2.0) as u32 + 70;
    app.world.insert_resource(WindowConfig {
        title: "Field of view — WASD move, walls cast shadows".to_string(),
        width: win_w,
        height: win_h,
        clear_color: [0.03, 0.03, 0.05, 1.0],
    });
    if let Some(cam) = app.world.resource_mut::<Camera>() {
        cam.position = Vec2::ZERO;
    }

    app.set_scene(Box::new(DungeonScene));

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
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
