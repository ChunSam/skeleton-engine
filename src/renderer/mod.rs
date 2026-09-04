pub(crate) mod bloom;
pub(crate) mod common;
pub mod context;
#[cfg(not(target_arch = "wasm32"))]
pub mod fade;
#[cfg(not(target_arch = "wasm32"))]
pub mod gpu_particle;
#[cfg(not(target_arch = "wasm32"))]
pub mod lighting;
pub mod post_process;
pub mod render_plugin;
pub mod render_target;
pub mod sprite;
pub mod text;
pub mod texture;
pub mod transition;
pub mod ui;
pub mod uv;

// ─── Shared renderer GPU types ────────────────────────────────────────────────

/// Camera view-projection uniform uploaded to the GPU each frame.
///
/// Shared between the sprite renderer (`sprite/geometry.rs`) and the GPU-particle
/// renderer (`gpu_particle.rs`). Both previously held a local copy; deduplicated here
/// to close an OCP finding from the 2026-06-16 module-cohesion review (that report was
/// deleted 2026-09-04; the fix it asked for is this type).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct CameraUniform {
    pub(crate) view_proj: [[f32; 4]; 4],
}

pub use context::{GpuContext, GpuContextError, RenderCapabilities};
pub use post_process::{PostProcessConfig, PostProcessRenderer, Tonemap};
pub use render_plugin::RenderPlugin;
pub use render_target::RenderTarget;
pub use sprite::{FrameContext, SpriteRenderer};
pub use text::{DrawText, TextAlign, TextAnchor, TextQueue, TextRenderer};
pub use texture::Texture;
pub use ui::{DrawImage, DrawRect, UiImageQueue, UiQueue};
pub use uv::{BlendUv, UvRect};

/// Embedded default font (DejaVu Sans, Bitstream Vera / Arev license — see
/// `assets/fonts/DejaVuSans-LICENSE.txt`). The browser sandbox has no system fonts, so on
/// wasm the engine falls back to this when a game supplies no [`crate::resources::FontData`],
/// letting `DrawText`/HUD text render out of the box. Native uses system fonts and does not
/// embed this (the `include_bytes!` is wasm-gated to keep it out of native binaries).
#[cfg(target_arch = "wasm32")]
pub(crate) const DEFAULT_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/DejaVuSans.ttf"
));
