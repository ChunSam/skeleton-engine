//! Roguelike slice — a seeded procedural dungeon explored under fog-of-war.
//!
//! The capstone that composes two engine features into one playable loop:
//! [`generate_bsp_dungeon`] carves a fresh, always-connected BSP dungeon from a seed, and
//! [`FovMap`] fog-of-war reveals it only as you walk. The bridge between them is a single line —
//! [`DungeonMap::to_path_grid`] turns the generated map into a `PathGrid` (the same grid an enemy
//! would path over), and [`FovMap::from_path_grid`] builds the field of view straight from it, so
//! the walls that block movement are exactly the walls that block sight:
//!
//! ```ignore
//! let map = generate_bsp_dungeon(COLS, ROWS, seed, &DungeonParams::default());
//! let fov = FovMap::from_path_grid(&map.to_path_grid());
//! ```
//!
//! Cells in your current line of sight render **bright**; cells you have explored but can no longer
//! see render **dim** (the fog-of-war memory); cells you have never seen stay **black**. Rooms are
//! tinted warmer than the corridors that link them, and a gem hides in every room but your spawn —
//! revealed only when it falls inside your sight. Every dungeon is fully connected, so every gem is
//! reachable.
//!
//! - **WASD / arrows** — move one cell (walls block movement and sight)
//! - **+ / -** — grow / shrink the torch radius
//! - **R** — descend: regenerate a fresh dungeon (new seed) with blank fog
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/roguelike.png cargo run --example roguelike`): captures the lit
//! spawn room with the rest of the dungeon in fog. `HEADLESS_FRAMES=N` overrides the default 6.
use engine::{
    generate_bsp_dungeon, App, Camera, Color, DrawRect, DrawText, DungeonMap, DungeonParams,
    FovMap, InputState, KeyCode, Scene, ShouldQuit, System, SystemRegistrar, TextQueue, UiQueue,
    WindowConfig, World,
};
use glam::{IVec2, Vec2};

const COLS: i32 = 48;
const ROWS: i32 = 32;
const CELL: f32 = 18.0;
const GRID_OX: f32 = 14.0;
const GRID_OY: f32 = 14.0;
const DEFAULT_RADIUS: i32 = 8;

// Fog palette: LIT = in sight now, DIM = explored but out of sight (remembered).
const BACKDROP: Color = Color::rgba(0.04, 0.04, 0.06, 1.0);
const CORRIDOR_LIT: Color = Color::rgba(0.30, 0.31, 0.37, 1.0);
const CORRIDOR_DIM: Color = Color::rgba(0.10, 0.10, 0.13, 1.0);
const ROOM_LIT: Color = Color::rgba(0.42, 0.36, 0.28, 1.0);
const ROOM_DIM: Color = Color::rgba(0.15, 0.13, 0.10, 1.0);
const WALL_LIT: Color = Color::rgba(0.56, 0.43, 0.30, 1.0);
const WALL_DIM: Color = Color::rgba(0.19, 0.15, 0.11, 1.0);
const GEM_LIT: Color = Color::rgba(0.32, 0.90, 0.96, 1.0);
const GEM_DIM: Color = Color::rgba(0.13, 0.30, 0.33, 1.0);
const PLAYER: Color = Color::rgba(1.0, 0.85, 0.22, 1.0);

fn cell_xy(x: i32, y: i32) -> (f32, f32) {
    (GRID_OX + x as f32 * CELL, GRID_OY + y as f32 * CELL)
}

/// Generates a dungeon and everything derived from it: the FOV (walls block sight), a per-cell
/// room-vs-corridor mask, the spawn, a gem in every non-spawn room, and the total floor count (for
/// the explored-% readout). This is the whole composition seam in one place.
fn make(seed: u64) -> Dungeon {
    let map = generate_bsp_dungeon(COLS, ROWS, seed, &DungeonParams::default());
    // The one-line bridge: dungeon → PathGrid → FovMap. The walls that block movement block sight.
    let fov = FovMap::from_path_grid(&map.to_path_grid());

    let mut room_mask = vec![false; (COLS * ROWS) as usize];
    for r in &map.rooms {
        for cy in r.y..r.y + r.h {
            for cx in r.x..r.x + r.w {
                if cx >= 0 && cy >= 0 && cx < COLS && cy < ROWS {
                    room_mask[(cy * COLS + cx) as usize] = true;
                }
            }
        }
    }

    let spawn = map.first_room_center().unwrap_or(IVec2::new(1, 1));
    // A gem at the center of every room except the spawn room (rooms[0]) — a reason to explore.
    let gems: Vec<IVec2> = map.rooms.iter().skip(1).map(engine::Room::center).collect();
    let found = vec![false; gems.len()];

    let total_floor = (0..ROWS)
        .flat_map(|y| (0..COLS).map(move |x| (x, y)))
        .filter(|&(x, y)| map.is_floor(x, y))
        .count();

    Dungeon {
        map,
        fov,
        room_mask,
        player: spawn,
        gems,
        found,
        total_floor,
        seed,
        radius: DEFAULT_RADIUS,
        dirty: true,
    }
}

/// A generated dungeon plus the live exploration state layered on top of it.
struct Dungeon {
    map: DungeonMap,
    fov: FovMap,
    room_mask: Vec<bool>,
    player: IVec2,
    gems: Vec<IVec2>,
    found: Vec<bool>,
    total_floor: usize,
    seed: u64,
    radius: i32,
    dirty: bool,
}

impl Dungeon {
    fn is_room(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < COLS && y < ROWS && self.room_mask[(y * COLS + x) as usize]
    }

    /// Moves the observer by `dir` if the destination is floor (walls block).
    fn try_move(&mut self, dir: IVec2) {
        let dest = self.player + dir;
        if self.map.is_floor(dest.x, dest.y) {
            self.player = dest;
            self.dirty = true;
        }
    }

    /// Fraction of the dungeon's floor that has been revealed, in `0..=1`.
    fn explored_fraction(&self) -> f32 {
        if self.total_floor == 0 {
            return 0.0;
        }
        let seen = (0..ROWS)
            .flat_map(|y| (0..COLS).map(move |x| (x, y)))
            .filter(|&(x, y)| self.map.is_floor(x, y) && self.fov.is_revealed(x, y))
            .count();
        seen as f32 / self.total_floor as f32
    }

    fn floor_color(&self, x: i32, y: i32, visible: bool) -> Color {
        match (self.is_room(x, y), visible) {
            (true, true) => ROOM_LIT,
            (true, false) => ROOM_DIM,
            (false, true) => CORRIDOR_LIT,
            (false, false) => CORRIDOR_DIM,
        }
    }
}

struct DungeonScene;

impl Scene for DungeonScene {
    fn on_enter(&mut self, _world: &mut World, systems: &mut SystemRegistrar) {
        systems.add(Crawl {
            state: make(1),
            depth: 1,
        });
    }
    fn on_exit(&mut self, _world: &mut World) {}
}

struct Crawl {
    state: Dungeon,
    depth: u32,
}

impl Crawl {
    /// Descends to a fresh dungeon (next seed, blank fog).
    fn descend(&mut self) {
        self.depth += 1;
        self.state = make(self.state.seed.wrapping_add(1));
    }
}

impl System for Crawl {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // ── Input (gather intents, then act after the InputState borrow drops) ──
        let (mut mv, mut d_radius, mut descend, mut quit) = (IVec2::ZERO, 0, false, false);
        if let Some(input) = world.resource::<InputState>() {
            quit = input.just_pressed(KeyCode::Escape);
            descend = input.just_pressed(KeyCode::KeyR);
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
        if descend {
            self.descend();
        }
        if mv != IVec2::ZERO {
            self.state.try_move(mv);
        }
        if d_radius != 0 {
            self.state.radius = (self.state.radius + d_radius).clamp(0, 20);
            self.state.dirty = true;
        }

        // ── Recompute the field of view only when something changed ────────────
        if self.state.dirty {
            let (player, radius) = (self.state.player, self.state.radius);
            self.state.fov.compute(player, radius);
            for i in 0..self.state.gems.len() {
                let g = self.state.gems[i];
                if self.state.fov.is_visible(g.x, g.y) {
                    self.state.found[i] = true;
                }
            }
            self.state.dirty = false;
        }

        // ── Draw the dungeon: bright in sight, dim if explored, black if unseen ─
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
                    if !self.state.fov.is_revealed(x, y) {
                        continue; // never seen → leave it black (fog)
                    }
                    let visible = self.state.fov.is_visible(x, y);
                    let color = if self.state.map.is_wall(x, y) {
                        if visible {
                            WALL_LIT
                        } else {
                            WALL_DIM
                        }
                    } else {
                        self.state.floor_color(x, y, visible)
                    };
                    let (px, py) = cell_xy(x, y);
                    ui.items
                        .push(DrawRect::new(px, py, CELL - 1.0, CELL - 1.0, color).with_z(0.1));
                }
            }

            // Gems: bright in sight, a dim ghost once discovered, hidden until then.
            for (i, &gem) in self.state.gems.iter().enumerate() {
                let color = if self.state.fov.is_visible(gem.x, gem.y) {
                    GEM_LIT
                } else if self.state.found[i] {
                    GEM_DIM
                } else {
                    continue;
                };
                let (px, py) = cell_xy(gem.x, gem.y);
                ui.items.push(
                    DrawRect::new(px + 5.0, py + 5.0, CELL - 10.0, CELL - 10.0, color).with_z(0.2),
                );
            }

            // Observer.
            let (px, py) = cell_xy(self.state.player.x, self.state.player.y);
            ui.items.push(
                DrawRect::new(px + 3.0, py + 3.0, CELL - 6.0, CELL - 6.0, PLAYER).with_z(0.3),
            );
        }

        // ── HUD ────────────────────────────────────────────────────────────────
        let hud_y = GRID_OY + ROWS as f32 * CELL + 10.0;
        let found = self.state.found.iter().filter(|&&f| f).count();
        let total = self.state.gems.len();
        let explored = (self.state.explored_fraction() * 100.0).round() as i32;
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!(
                    "Depth {}  ·  seed {}  ·  gems {found}/{total}  ·  explored {explored}%",
                    self.depth, self.state.seed
                ),
                Vec2::new(GRID_OX, hud_y),
                17.0,
                Color::rgb(0.9, 0.88, 0.72),
            ));
            tq.push(DrawText::new(
                "WASD / arrows: move    + / -: torch radius    R: descend (new dungeon)    Esc: quit",
                Vec2::new(GRID_OX, hud_y + 22.0),
                14.0,
                Color::rgb(0.68, 0.7, 0.76),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "roguelike"
    }
}

fn main() {
    let mut app = App::new();
    let win_w = (COLS as f32 * CELL + GRID_OX * 2.0) as u32;
    let win_h = (ROWS as f32 * CELL + GRID_OY * 2.0) as u32 + 70;
    app.world.insert_resource(WindowConfig {
        title: "Roguelike — procgen dungeon under fog-of-war (R to descend)".to_string(),
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
