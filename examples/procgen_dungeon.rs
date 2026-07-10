//! Procedural dungeon generation — a BSP dungeon you can regenerate and walk.
//!
//! [`generate_bsp_dungeon`] carves rooms into a binary-space-partition tree and connects siblings
//! with corridors, so every room is reachable. Generation is deterministic from the seed, so **R**
//! rolls a fresh seed and rebuilds a new (always-connected) dungeon instantly. Rooms are tinted
//! warmer than the corridors that link them, so the BSP structure is visible at a glance.
//!
//! - **WASD / arrows** — walk the yellow explorer (walls block)
//! - **R** — regenerate with the next seed
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/procgen.png cargo run --example procgen_dungeon`): captures the
//! starting dungeon. `HEADLESS_FRAMES=N` overrides the default 6 frames.
use engine::{
    generate_bsp_dungeon, App, Camera, Color, DrawRect, DrawText, DungeonMap, DungeonParams,
    InputState, KeyCode, Scene, ShouldQuit, System, SystemRegistrar, TextQueue, UiQueue,
    WindowConfig, World,
};
use glam::{IVec2, Vec2};

const COLS: i32 = 48;
const ROWS: i32 = 32;
const CELL: f32 = 18.0;
const GRID_OX: f32 = 14.0;
const GRID_OY: f32 = 14.0;

const BACKDROP: Color = Color::rgba(0.05, 0.05, 0.07, 1.0);
const CORRIDOR: Color = Color::rgba(0.22, 0.23, 0.28, 1.0);
const ROOM: Color = Color::rgba(0.34, 0.31, 0.26, 1.0);
const PLAYER: Color = Color::rgba(1.0, 0.85, 0.22, 1.0);

fn cell_xy(x: i32, y: i32) -> (f32, f32) {
    (GRID_OX + x as f32 * CELL, GRID_OY + y as f32 * CELL)
}

/// Generates a dungeon plus a per-cell "is this a room (vs a corridor)?" mask and a spawn cell.
fn make(seed: u64) -> (DungeonMap, Vec<bool>, IVec2) {
    let map = generate_bsp_dungeon(COLS, ROWS, seed, &DungeonParams::default());
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
    (map, room_mask, spawn)
}

struct DungeonScene;

impl Scene for DungeonScene {
    fn on_enter(&mut self, _world: &mut World, systems: &mut SystemRegistrar) {
        systems.add(Demo::new(1));
    }
    fn on_exit(&mut self, _world: &mut World) {}
}

struct Demo {
    map: DungeonMap,
    room_mask: Vec<bool>,
    player: IVec2,
    seed: u64,
}

impl Demo {
    fn new(seed: u64) -> Self {
        let (map, room_mask, player) = make(seed);
        Self {
            map,
            room_mask,
            player,
            seed,
        }
    }

    fn regenerate(&mut self) {
        self.seed = self.seed.wrapping_add(1);
        let (map, room_mask, player) = make(self.seed);
        self.map = map;
        self.room_mask = room_mask;
        self.player = player;
    }

    fn try_move(&mut self, dir: IVec2) {
        let dest = self.player + dir;
        if self.map.is_floor(dest.x, dest.y) {
            self.player = dest;
        }
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

        // ── Render: a dark backdrop = walls, floor cells on top (rooms warmer) ─
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
                    let color = if self.room_mask[(y * COLS + x) as usize] {
                        ROOM
                    } else {
                        CORRIDOR
                    };
                    let (px, py) = cell_xy(x, y);
                    ui.items
                        .push(DrawRect::new(px, py, CELL - 1.0, CELL - 1.0, color).with_z(0.1));
                }
            }
            let (px, py) = cell_xy(self.player.x, self.player.y);
            ui.items.push(
                DrawRect::new(px + 3.0, py + 3.0, CELL - 6.0, CELL - 6.0, PLAYER).with_z(0.2),
            );
        }

        // ── HUD ────────────────────────────────────────────────────────────────
        let hud_y = GRID_OY + ROWS as f32 * CELL + 10.0;
        let rooms = self.map.rooms.len();
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!("BSP dungeon — seed {} · {rooms} rooms", self.seed),
                Vec2::new(GRID_OX, hud_y),
                17.0,
                Color::rgb(0.9, 0.88, 0.72),
            ));
            tq.push(DrawText::new(
                "WASD / arrows: walk    R: regenerate (new seed)    Esc: quit",
                Vec2::new(GRID_OX, hud_y + 22.0),
                14.0,
                Color::rgb(0.68, 0.7, 0.76),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "procgen_dungeon"
    }
}

fn main() {
    let mut app = App::new();
    let win_w = (COLS as f32 * CELL + GRID_OX * 2.0) as u32;
    let win_h = (ROWS as f32 * CELL + GRID_OY * 2.0) as u32 + 70;
    app.world.insert_resource(WindowConfig {
        title: "Procedural BSP dungeon — R to regenerate".to_string(),
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
