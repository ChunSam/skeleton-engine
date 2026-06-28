//! `TileAnimationSet::stagger` — per-cell animation phase stagger.
//!
//! Animated tiles of the same value don't all start on frame 0: each cell's start phase is
//! `(row + col) × frame_time × stagger`, so neighbours ripple slightly out of sync instead of
//! flashing in unison. That stagger factor used to be hardcoded (`0.37`); it's now a field on
//! `TileAnimationSet` (default [`engine::DEFAULT_TILE_ANIM_STAGGER`]).
//!
//! Two identical animated grids sit side by side, cycling an 8-frame colour ramp:
//! - **Left — `with_stagger(0.0)`**: every cell is in lockstep, so the whole grid is one flat
//!   colour that pulses through the ramp together.
//! - **Right — default `0.37`**: the start phase grows along each diagonal, so at any instant
//!   the grid shows a diagonal **gradient** of frames — the rippling look.
//!
//! A single frame already shows the difference (flat vs gradient). Press **Esc** to quit.
//!
//! Headless: `HEADLESS_SHOT=/tmp/stagger.png cargo run --example tile_anim_stagger`.
use engine::ecs::System;
use engine::{
    AnimatedTileSystem, App, DrawText, InputState, KeyCode, ShouldQuit, TextQueue, TileAnimation,
    TileAnimationSet, Tilemap, TilemapAtlas, TilemapSystem, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 760;
const WIN_H: u32 = 460;

const FRAMES: u32 = 8; // atlas columns = animation frames
const FRAME_PX: u32 = 24;
const GRID: usize = 6; // GRID×GRID cells per map
const TILE: f32 = 44.0;
const ANIM_VAL: u32 = 1;
const FRAME_TIME: f32 = 0.16;

/// 8-frame horizontal colour-ramp atlas (each frame a distinct hue) so the *frame index* a
/// cell is showing is obvious at a glance — that's what makes the stagger visible in one shot.
fn write_atlas_png() -> String {
    let path = "target/tile_anim_stagger_atlas.png";
    let mut img = image::RgbaImage::new(FRAMES * FRAME_PX, FRAME_PX);
    for f in 0..FRAMES {
        // Hue ramp across the 8 frames (red → orange → … → magenta).
        let t = f as f32 / FRAMES as f32;
        let (r, g, b) = hue(t);
        for py in 0..FRAME_PX {
            for px in 0..FRAME_PX {
                // 1px dark gutter so individual cells stay legible when tiled.
                let edge = px == 0 || py == 0;
                let c = if edge {
                    [20, 20, 26, 255]
                } else {
                    [r, g, b, 255]
                };
                img.put_pixel(f * FRAME_PX + px, py, image::Rgba(c));
            }
        }
    }
    img.save(path).expect("write atlas png");
    path.to_string()
}

fn hue(t: f32) -> (u8, u8, u8) {
    let h = (t * 6.0).clamp(0.0, 6.0);
    let i = h.floor() as i32;
    let f = h - i as f32;
    let (r, g, b) = match i {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };
    ((r * 235.0) as u8, (g * 235.0) as u8, (b * 235.0) as u8)
}

/// Spawns a GRID×GRID animated tilemap at `origin` with the given stagger factor.
fn spawn_grid(app: &mut App, atlas_path: &str, origin: Vec2, stagger: f32) {
    let tiles = vec![vec![ANIM_VAL; GRID]; GRID];
    let atlas = TilemapAtlas::new(atlas_path, FRAMES, 1);
    let tilemap = Tilemap::new(atlas, tiles, TILE, origin);

    let mut anim_set = TileAnimationSet::new().with_stagger(stagger);
    anim_set.insert(
        ANIM_VAL,
        TileAnimation::new((0..FRAMES).collect(), FRAME_TIME),
    );

    let e = app.world.spawn();
    app.world.add_component(e, tilemap);
    app.world.add_component(e, anim_set);
}

struct Hud;

impl System for Hud {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if world
            .resource::<InputState>()
            .is_some_and(|i| i.just_pressed(KeyCode::Escape))
        {
            if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                sq.quit();
            }
        }
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "TileAnimationSet::stagger — per-cell animation phase offset (Esc: quit)",
                Vec2::new(16.0, 14.0),
                16.0,
                [235, 232, 215, 255],
            ));
            tq.push(DrawText::new(
                "with_stagger(0.0): lockstep (flat)",
                Vec2::new(40.0, 56.0),
                14.0,
                [150, 200, 255, 230],
            ));
            tq.push(DrawText::new(
                "default 0.37: rippling (diagonal gradient)",
                Vec2::new(WIN_W as f32 * 0.5 + 30.0, 56.0),
                14.0,
                [255, 200, 150, 230],
            ));
        }
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "tile_anim_stagger — animation phase stagger".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });

    let atlas_path = write_atlas_png();
    app.load_image(&atlas_path);

    let grid_w = GRID as f32 * TILE;
    let top = 90.0;
    // Left grid: synchronized. Right grid: default stagger.
    spawn_grid(&mut app, &atlas_path, Vec2::new(60.0, top), 0.0);
    spawn_grid(
        &mut app,
        &atlas_path,
        Vec2::new(WIN_W as f32 - 60.0 - grid_w, top),
        engine::DEFAULT_TILE_ANIM_STAGGER,
    );

    app.add_system(TilemapSystem::new());
    app.add_system(AnimatedTileSystem::new());
    app.add_system(Hud);

    // `HEADLESS_SHOT=path` → render to a PNG with no window and exit. A few frames in, the
    // synced grid is one flat colour and the staggered grid is a diagonal gradient.
    if let Ok(path) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(40);
        app.save_screenshot_headless(frames, &path)
            .expect("headless screenshot");
        println!("wrote {path} ({frames} frames)");
        return;
    }
    app.run();
}
