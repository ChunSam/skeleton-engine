//! `render_format_query` — ask the engine which texture formats this GPU can render into,
//! via the [`RenderCapabilities`] resource, and branch accordingly.
//!
//! Some formats are not usable as a color render target on every backend — most notably
//! float formats like `Rgba16Float` on WebGL2, which need the `EXT_color_buffer_float`
//! extension. Before requesting an HDR render target a game can ask:
//!
//! ```ignore
//! let hdr_ok = world
//!     .resource::<RenderCapabilities>()
//!     .is_some_and(|caps| caps.supports_render_target(wgpu::TextureFormat::Rgba16Float));
//! ```
//!
//! `RenderCapabilities` is inserted into the world at GPU init, so it is read from a system
//! or `Scene::on_enter` (not before `run()`, where the GPU does not exist yet).
//!
//! This example queries a spread of formats and renders a color-coded yes/no table, plus the
//! decision a game would make (HDR render target vs. an 8-bit fallback). It also requests an
//! HDR render target up front to exercise the **automatic fallback**: when a requested format
//! is not renderable, [`App::create_render_target_with_format`] falls back to the surface
//! format and logs a warning (run with `RUST_LOG=warn` to see it on an unsupported GPU).
//!
//! Controls: Esc quit.

use engine::{
    App, DrawText, InputState, KeyCode, RenderCapabilities, ShouldQuit, System, TextQueue, Vec2,
    WindowConfig, World,
};
use wgpu::TextureFormat;

/// The format a game would prefer for an HDR offscreen buffer.
const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

/// Formats probed by the demo. `RenderCapabilities` reports this GPU/backend's *actual*
/// capability, not the portable WebGPU subset: on a desktop Vulkan/Metal/DX12 backend the float
/// and 8-bit color formats are all renderable, while a block-compressed format (`Bc1RgbaUnorm`)
/// is never a color render target — so the table shows both `yes` and `no`. On WebGL2 the float
/// formats flip to `no` without `EXT_color_buffer_float`.
const PROBED: &[(&str, TextureFormat)] = &[
    ("Rgba16Float  (HDR)", TextureFormat::Rgba16Float),
    ("Rgba32Float  (HDR)", TextureFormat::Rgba32Float),
    ("Rgba8Unorm   (linear)", TextureFormat::Rgba8Unorm),
    ("Rgba8UnormSrgb", TextureFormat::Rgba8UnormSrgb),
    ("Bc1RgbaUnorm (compressed)", TextureFormat::Bc1RgbaUnorm),
];

const YES: [u8; 4] = [120, 220, 140, 245];
const NO: [u8; 4] = [232, 132, 120, 245];

/// Reads `RenderCapabilities` each frame and renders the capability table + decision.
#[derive(Default)]
struct QuerySystem {
    /// One-shot guard so the capability dump is logged to stderr only once.
    reported: bool,
}

impl System for QuerySystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                if let Some(quit) = world.resource_mut::<ShouldQuit>() {
                    quit.quit();
                }
                return;
            }
        }

        // Read the capabilities (cloned out so we can borrow TextQueue mutably below).
        let caps = match world.resource::<RenderCapabilities>() {
            Some(caps) => caps.clone(),
            None => return, // GPU not initialized yet
        };
        let hdr_ok = caps.supports_render_target(HDR_FORMAT);

        // Log the table once (handy as a CLI capability dump; also lets a headless smoke assert).
        if !self.reported {
            self.reported = true;
            log::info!("surface format: {:?}", caps.surface_format());
            for (_label, format) in PROBED {
                log::info!(
                    "render target {:?}: {}",
                    format,
                    if caps.supports_render_target(*format) {
                        "yes"
                    } else {
                        "no"
                    }
                );
            }
        }

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };

        tq.push(DrawText::new(
            "RenderCapabilities — which formats can this GPU render into?",
            Vec2::new(24.0, 22.0),
            21.0,
            [255, 245, 220, 245],
        ));
        tq.push(DrawText::new(
            format!("surface format: {:?}", caps.surface_format()),
            Vec2::new(24.0, 60.0),
            15.0,
            [180, 190, 205, 220],
        ));

        let mut y = 110.0;
        for (label, format) in PROBED {
            let ok = caps.supports_render_target(*format);
            tq.push(DrawText::new(
                format!(
                    "{label:22}  render target: {}",
                    if ok { "yes" } else { "no" }
                ),
                Vec2::new(40.0, y),
                17.0,
                if ok { YES } else { NO },
            ));
            y += 34.0;
        }

        // The decision a game makes from the query.
        let decision = if hdr_ok {
            "-> Rgba16Float renderable: use an HDR render target"
        } else {
            "-> Rgba16Float NOT renderable: fall back to an 8-bit render target"
        };
        tq.push(DrawText::new(
            decision,
            Vec2::new(24.0, y + 24.0),
            17.0,
            if hdr_ok { YES } else { NO },
        ));
        tq.push(DrawText::new(
            "create_render_target_with_format also falls back automatically (see RUST_LOG=warn)",
            Vec2::new(24.0, y + 58.0),
            13.0,
            [165, 175, 190, 205],
        ));
        tq.push(DrawText::new(
            "Esc: quit",
            Vec2::new(24.0, y + 96.0),
            14.0,
            [160, 170, 185, 200],
        ));
    }
}

fn main() {
    env_logger::init();
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "render_format_query — GPU render-format capabilities".to_string(),
        width: 720,
        height: 520,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    // Request an HDR render target up front. If this GPU cannot render into Rgba16Float, the
    // engine resolves it to the surface format automatically (and logs a warning) — the demo
    // never has to special-case an unsupported backend just to boot.
    app.create_render_target_with_format("scene_hdr", 256, 256, HDR_FORMAT);

    app.add_system(QuerySystem::default());
    app.run();
}
