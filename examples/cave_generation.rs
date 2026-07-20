//! Procedural cave generation — an organic, always-connected cavern you can regenerate and walk.
//!
//! [`generate_cellular_cave`] seeds the grid with random rock, smooths it with a few
//! cellular-automata passes, then keeps only the largest connected cavern — so, like the BSP
//! dungeon, every floor cell is reachable. Generation is deterministic from the seed, so **R** rolls
//! a fresh seed and grows a new (always-connected) cave instantly. Floor cells touching a wall are
//! tinted darker than the open interior, so the cavern's organic outline reads at a glance.
//!
//! - **WASD / arrows** — walk the yellow explorer (rock blocks)
//! - **R** — regenerate with the next seed
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/cave.png cargo run --example cave_generation`): first self-checks
//! that the cave is a single connected cavern (exits non-zero if not), then captures the starting
//! cave. `HEADLESS_FRAMES=N` overrides the default 6 frames.
use engine::{
    generate_cellular_cave, App, Camera, CaveParams, Color, DrawRect, DrawText, DungeonMap,
    InputState, KeyCode, Scene, ShouldQuit, System, SystemRegistrar, TextQueue, UiQueue,
    WindowConfig, World,
};
use glam::{IVec2, Vec2};

const COLS: i32 = 64;
const ROWS: i32 = 44;
const CELL: f32 = 14.0;
const GRID_OX: f32 = 14.0;
const GRID_OY: f32 = 14.0;

const BACKDROP: Color = Color::rgba(0.05, 0.05, 0.07, 1.0); // rock / walls
const CAVE_FLOOR: Color = Color::rgba(0.30, 0.34, 0.33, 1.0); // open interior
const CAVE_EDGE: Color = Color::rgba(0.20, 0.22, 0.24, 1.0); // floor touching rock
const PLAYER: Color = Color::rgba(1.0, 0.85, 0.22, 1.0);

fn cell_xy(x: i32, y: i32) -> (f32, f32) {
    (GRID_OX + x as f32 * CELL, GRID_OY + y as f32 * CELL)
}

/// Generates a cave and its spawn cell (the cavern's central floor cell).
fn make(seed: u64) -> (DungeonMap, IVec2) {
    let map = generate_cellular_cave(COLS, ROWS, seed, &CaveParams::default());
    let spawn = map.first_room_center().unwrap_or(IVec2::new(1, 1));
    (map, spawn)
}

/// Whether `(x, y)` is floor with at least one orthogonal wall neighbor — a cavern-outline cell.
fn is_edge(map: &DungeonMap, x: i32, y: i32) -> bool {
    map.is_floor(x, y)
        && (map.is_wall(x + 1, y)
            || map.is_wall(x - 1, y)
            || map.is_wall(x, y + 1)
            || map.is_wall(x, y - 1))
}

struct CaveScene;

impl Scene for CaveScene {
    fn on_enter(&mut self, _world: &mut World, systems: &mut SystemRegistrar) {
        systems.add(Demo::new(1));
    }
    fn on_exit(&mut self, _world: &mut World) {}
}

struct Demo {
    map: DungeonMap,
    player: IVec2,
    seed: u64,
}

impl Demo {
    fn new(seed: u64) -> Self {
        let (map, player) = make(seed);
        Self { map, player, seed }
    }

    fn regenerate(&mut self) {
        self.seed = self.seed.wrapping_add(1);
        let (map, player) = make(self.seed);
        self.map = map;
        self.player = player;
    }

    fn try_move(&mut self, dir: IVec2) {
        let dest = self.player + dir;
        if self.map.is_floor(dest.x, dest.y) {
            self.player = dest;
        }
    }

    fn floor_count(&self) -> usize {
        let mut n = 0;
        for y in 0..ROWS {
            for x in 0..COLS {
                if self.map.is_floor(x, y) {
                    n += 1;
                }
            }
        }
        n
    }
}

impl System for Demo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // ── Input (gather, then act after the InputState borrow drops) ─────────
        let (mut mv, mut regen, mut quit) = (IVec2::ZERO, false, false);
        if let Some(input) = world.resource::<InputState>() {
            quit = input.just_pressed(KeyCode::Escape);
            regen = input.just_pressed(KeyCode::KeyR);
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
        if mv != IVec2::ZERO {
            self.try_move(mv);
        }

        // ── Render: a dark rock backdrop, floor cells on top (edges darker) ────
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
                        continue; // rock → the dark backdrop shows through
                    }
                    let color = if is_edge(&self.map, x, y) {
                        CAVE_EDGE
                    } else {
                        CAVE_FLOOR
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
        let floor = self.floor_count();
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!(
                    "Cellular-automata cave — seed {} · {floor} floor cells",
                    self.seed
                ),
                Vec2::new(GRID_OX, hud_y),
                17.0,
                Color::rgb(0.86, 0.9, 0.88),
            ));
            tq.push(DrawText::new(
                "WASD / arrows: walk    R: regenerate (new seed)    Esc: quit",
                Vec2::new(GRID_OX, hud_y + 22.0),
                14.0,
                Color::rgb(0.66, 0.7, 0.72),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "cave_generation"
    }
}

/// Headless acceptance check: flood-fill from the spawn and confirm every floor cell is reached —
/// i.e. the cave is a single connected cavern. Returns `(reached, total_floor)`.
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
        title: "Procedural cellular-automata cave — R to regenerate".to_string(),
        width: win_w,
        height: win_h,
        clear_color: [0.03, 0.03, 0.05, 1.0],
    });
    if let Some(cam) = app.world.resource_mut::<Camera>() {
        cam.position = Vec2::ZERO;
    }

    app.set_scene(Box::new(CaveScene));

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        // Self-check the defining invariant before capturing: a single connected cavern.
        let (map, spawn) = make(1);
        let (reached, total) = connectivity(&map);
        if total == 0 || reached != total {
            eprintln!(
                "FAIL: cave is not a single connected cavern — reached {reached}/{total} floor cells"
            );
            std::process::exit(1);
        }
        println!(
            "OK: cellular-automata cave is one connected cavern — {total} floor cells, spawn at ({}, {})",
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
