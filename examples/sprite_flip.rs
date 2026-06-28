//! `SpriteFlip` — mirror a sprite across X / Y without swapping textures or negating scale.
//!
//! Add a [`SpriteFlip`] component to any sprite-bearing entity (plain `Sprite`, `AtlasSprite`, or
//! `ShaderMaterial`) and the renderer mirrors the sampled UV region in place — free, and correct for
//! animation frames + atlas tiles. The usual use is "face the way you're moving":
//! `flip.x = velocity.x < 0.0`.
//!
//! This demo draws the same 4-colour-quadrant texture twice: a fixed **reference** on the left and a
//! **controllable** copy on the right. Toggle the copy's flip and watch the quadrants mirror against
//! the reference:
//! - **H** — toggle horizontal flip (red↔green on top, blue↔yellow on the bottom)
//! - **V** — toggle vertical flip (top row ↔ bottom row)
//! - **Esc** — quit
//!
//! Headless: `HEADLESS_SHOT=/tmp/flip.png cargo run --example sprite_flip`.
use engine::{
    App, Camera, Color, DrawText, InputState, KeyCode, ShouldQuit, Sprite, SpriteFlip, System,
    TextQueue, Transform, Vec2, WindowConfig, World,
};
use std::path::PathBuf;

const WIN_W: u32 = 720;
const WIN_H: u32 = 440;
const SPRITE: f32 = 200.0;

/// Writes a 64×64 four-quadrant texture (TL red, TR green, BL blue, BR yellow) so a flip is
/// unmistakable: horizontal flip swaps the columns, vertical flip swaps the rows.
fn generate_quadrant_texture() -> PathBuf {
    use image::{Rgba, RgbaImage};
    const S: u32 = 64;
    let img = RgbaImage::from_fn(S, S, |x, y| {
        let left = x < S / 2;
        let top = y < S / 2;
        Rgba(match (left, top) {
            (true, true) => [222, 70, 70, 255],    // top-left  — red
            (false, true) => [70, 200, 95, 255],   // top-right — green
            (true, false) => [80, 120, 235, 255],  // bot-left  — blue
            (false, false) => [235, 200, 70, 255], // bot-right — yellow
        })
    });
    let path = std::env::temp_dir().join("skeleton_sprite_flip_quadrants.png");
    img.save(&path).expect("write sprite_flip texture");
    path
}

struct Demo {
    /// The right-hand sprite the player flips.
    target: engine::Entity,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (toggle_x, toggle_y, quit) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.just_pressed(KeyCode::KeyH),
                    i.just_pressed(KeyCode::KeyV),
                    i.just_pressed(KeyCode::Escape),
                )
            })
            .unwrap_or((false, false, false));

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if toggle_x || toggle_y {
            if let Some(flip) = world.get_mut::<SpriteFlip>(self.target) {
                flip.x ^= toggle_x;
                flip.y ^= toggle_y;
            }
        }

        let flip = world
            .get::<SpriteFlip>(self.target)
            .copied()
            .unwrap_or_default();
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "SpriteFlip — mirror a sprite across X / Y (UV flip, no texture swap)",
                Vec2::new(40.0, 26.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                "reference",
                Vec2::new(
                    WIN_W as f32 * 0.3 - 38.0,
                    WIN_H as f32 * 0.5 + SPRITE * 0.5 + 14.0,
                ),
                15.0,
                Color::rgb(0.75, 0.82, 0.95),
            ));
            tq.push(DrawText::new(
                format!("flipped  x={}  y={}", flip.x, flip.y),
                Vec2::new(
                    WIN_W as f32 * 0.68 - 52.0,
                    WIN_H as f32 * 0.5 + SPRITE * 0.5 + 14.0,
                ),
                15.0,
                Color::rgb(0.75, 0.95, 0.78),
            ));
            tq.push(DrawText::new(
                "H: flip X    V: flip Y    Esc: quit",
                Vec2::new(40.0, WIN_H as f32 - 28.0),
                15.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "sprite_flip_demo"
    }
}

fn spawn_sprite(
    app: &mut App,
    x_frac: f32,
    tex: &str,
    handle: engine::Handle<engine::ImageAsset>,
) -> engine::Entity {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: Vec2::new(WIN_W as f32 * x_frac, WIN_H as f32 * 0.5),
            scale: Vec2::splat(SPRITE),
            ..Default::default()
        },
    );
    app.world
        .add_component(e, Sprite::textured_with_handle(tex, Some(handle)));
    e
}

fn main() {
    let path = generate_quadrant_texture();
    let tex = path.to_string_lossy().to_string();

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "sprite_flip — mirror across X / Y".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    // Top-left anchored camera (camera.position = viewport top-left), so world coords match the
    // window pixels used above.
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    let handle = app.load_image(&tex);
    let _reference = spawn_sprite(&mut app, 0.3, &tex, handle.clone());
    let target = spawn_sprite(&mut app, 0.68, &tex, handle);
    // Start the controllable copy mirrored horizontally so the difference is visible at a glance.
    app.world.add_component(target, SpriteFlip::horizontal());

    app.add_system(Demo { target });

    // `HEADLESS_SHOT=path` → render to a PNG with no window and exit.
    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
