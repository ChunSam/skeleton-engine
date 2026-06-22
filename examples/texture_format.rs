//! `texture_format` — demonstrates choosing a texture's pixel format via
//! [`App::load_image_with_format`].
//!
//! The same grayscale ramp PNG is loaded twice with **byte-identical** pixels but
//! two different GPU formats, then drawn side by side:
//!
//! - **Left (sRGB, the default):** loaded with [`App::load_image`]. The sampler
//!   decodes the sRGB-encoded bytes to linear for shading, and the sRGB surface
//!   re-encodes on write — so the ramp displays as authored.
//! - **Right (linear, `Rgba8Unorm`):** loaded with `load_image_with_format`. The
//!   sampler reads the bytes *verbatim* (no sRGB→linear decode), so the same bytes
//!   are treated as already-linear and the surface encode lightens the midtones —
//!   the ramp looks visibly brighter.
//!
//! That brightness gap is the whole point: a **data texture** (normal map, mask,
//! height or lookup table) holds non-color bytes that must be sampled verbatim, so
//! it wants the linear format — exactly the right-hand behavior.
//!
//! Controls: Esc quit.

use engine::{
    App, DrawText, Handle, ImageAsset, InputState, KeyCode, ShouldQuit, Sprite, System, TextQueue,
    Transform, Vec2, WindowConfig, World,
};
use image::{Rgba, RgbaImage};
use wgpu::TextureFormat;

const TEX_W: u32 = 128;
const TEX_H: u32 = 128;

/// Writes a horizontal black→white grayscale ramp PNG to `path`.
///
/// The bytes are a plain linear column index scaled to 0..=255. Whether they *read*
/// as sRGB-encoded color or as raw linear data is decided entirely by the load format.
fn generate_ramp_png(path: &std::path::Path) {
    let img = RgbaImage::from_fn(TEX_W, TEX_H, |x, _y| {
        let v = (x * 255 / (TEX_W - 1)) as u8;
        Rgba([v, v, v, 255])
    });
    img.save(path).expect("write ramp png");
}

/// Loads the ramp twice (identical bytes, two formats) and returns both handles.
fn load_both_ramps(app: &mut App) -> (Handle<ImageAsset>, Handle<ImageAsset>) {
    // Two distinct temp paths so the two formats don't collide in the texture cache
    // (one format per path). The files are byte-identical — only the load format differs.
    let srgb_path = std::env::temp_dir().join("skeleton_texfmt_ramp_srgb.png");
    let linear_path = std::env::temp_dir().join("skeleton_texfmt_ramp_linear.png");
    generate_ramp_png(&srgb_path);
    generate_ramp_png(&linear_path);

    let srgb = app.load_image(srgb_path.to_string_lossy().into_owned());
    let linear = app.load_image_with_format(
        linear_path.to_string_lossy().into_owned(),
        TextureFormat::Rgba8Unorm,
    );
    (srgb, linear)
}

/// Spawns a `size`×`size` sprite of `handle` centered at `(cx, cy)`.
fn spawn_swatch(app: &mut App, handle: &Handle<ImageAsset>, cx: f32, cy: f32, size: f32) {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: Vec2::new(cx, cy),
            scale: Vec2::new(size, size),
            ..Default::default()
        },
    );
    app.world
        .add_component(e, Sprite::with_handle(handle.clone()));
}

/// Pushes the explanatory labels each frame and handles Esc.
struct DemoSystem;

impl System for DemoSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                if let Some(quit) = world.resource_mut::<ShouldQuit>() {
                    quit.quit();
                }
                return;
            }
        }

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        tq.push(DrawText::new(
            "Texture format — same ramp bytes, two GPU formats",
            Vec2::new(20.0, 18.0),
            20.0,
            [255, 245, 220, 240],
        ));
        tq.push(DrawText::new(
            "load_image  ->  Rgba8UnormSrgb (default)",
            Vec2::new(70.0, 92.0),
            16.0,
            [225, 230, 240, 235],
        ));
        tq.push(DrawText::new(
            "load_image_with_format  ->  Rgba8Unorm (linear)",
            Vec2::new(470.0, 92.0),
            16.0,
            [225, 230, 240, 235],
        ));
        tq.push(DrawText::new(
            "color art: bytes are sRGB, sampler decodes -> displays as authored",
            Vec2::new(70.0, 470.0),
            14.0,
            [170, 180, 195, 210],
        ));
        tq.push(DrawText::new(
            "data texture: bytes sampled verbatim -> midtones look brighter",
            Vec2::new(470.0, 470.0),
            14.0,
            [170, 180, 195, 210],
        ));
        tq.push(DrawText::new(
            "Esc: quit",
            Vec2::new(20.0, 560.0),
            15.0,
            [160, 170, 185, 200],
        ));
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "texture_format — sRGB vs linear texture upload".to_string(),
        width: 900,
        height: 600,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    let (srgb, linear) = load_both_ramps(&mut app);
    spawn_swatch(&mut app, &srgb, 250.0, 280.0, 330.0);
    spawn_swatch(&mut app, &linear, 650.0, 280.0, 330.0);

    app.add_system(DemoSystem);
    app.run();
}
