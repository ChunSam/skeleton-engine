use crate::renderer::{PostProcessRenderer, SpriteRenderer, TextRenderer};
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct RenderState {
    pub(crate) sprite_renderer: Option<SpriteRenderer>,
    pub(crate) text_renderer: Option<TextRenderer>,
    pub(crate) post_renderer: Option<PostProcessRenderer>,
    /// Real multi-pass bloom renderer (lazy init; only when `PostProcessConfig::bloom` is on, which
    /// requires post-process enabled). Runs on both native and wasm.
    pub(crate) bloom_renderer: Option<crate::renderer::bloom::BloomRenderer>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) lighting_renderer: Option<crate::renderer::lighting::LightingRenderer>,
    /// Fade renderer executed as the final pass when `FadeTransition` has `alpha > 0`.
    ///
    /// **Platform note:** This field exists on native targets only. On wasm,
    /// `FadeTransition` is accepted without error but the fade effect is not rendered
    /// (no render pass is issued). Use `#[cfg(not(target_arch = "wasm32"))]` guards
    /// if your game relies on fade transitions for scene changes.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fade_renderer: Option<crate::renderer::fade::FadeRenderer>,
    /// Intermediate texture the lighting pass renders the scene into first.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) scene_texture_for_lighting: Option<(
        wgpu::Texture,
        wgpu::TextureView,
        u32,
        u32,
        wgpu::TextureFormat,
    )>,
    /// Intermediate texture that passes the post-process result as input to the lighting pass.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) post_texture_for_lighting: Option<(
        wgpu::Texture,
        wgpu::TextureView,
        u32,
        u32,
        wgpu::TextureFormat,
    )>,
    /// Offscreen texture the docked editor renders the game scene into.
    /// (width, height, format, texture, view) — recreated when the central panel resizes.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) docked_scene_texture: Option<(
        u32,
        u32,
        wgpu::TextureFormat,
        wgpu::Texture,
        wgpu::TextureView,
    )>,
    /// GPU compute-shader particle renderer (lazy init).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) gpu_particle_renderer: Option<crate::renderer::gpu_particle::GpuParticleRenderer>,
    /// Custom render plugins registered via [`App::add_render_plugin`].
    /// Invoked once per frame after the main sprite/UI/particle passes, before
    /// post-processing and lighting. Runs on both native and wasm.
    pub(crate) render_plugins: Vec<Box<dyn crate::renderer::RenderPlugin>>,
    /// Map of registered offscreen render targets (name → RenderTarget).
    pub(crate) render_targets: HashMap<String, crate::renderer::render_target::RenderTarget>,
    pub(crate) egui_renderer: Option<egui_wgpu::Renderer>,
    pub(crate) egui_state: Option<egui_winit::State>,
    /// Temporary buffer carrying tessellated output from `update()` to `render()`.
    pub(crate) egui_output: Option<(Vec<egui::ClippedPrimitive>, egui::TexturesDelta, f32)>,
}

impl RenderState {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}
