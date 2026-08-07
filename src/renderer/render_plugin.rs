use super::FrameContext;
use crate::ecs::World;

/// A user-supplied render pass invoked once per frame during the scene render.
///
/// Implement this trait and register it with [`App::add_render_plugin`](crate::App::add_render_plugin)
/// to inject a custom GPU pass (outlines, shadows, debug overlays, screen effects)
/// without forking the engine's `render()` function. The pass runs after the main
/// sprite/UI/particle passes and BEFORE post-processing and lighting, so downstream
/// effects still apply to whatever the plugin draws.
///
/// # Example
///
/// ```no_run
/// use engine::{RenderPlugin, FrameContext, World};
///
/// struct OutlinePass;
///
/// impl RenderPlugin for OutlinePass {
///     fn record(&mut self, ctx: &mut FrameContext, _world: &World, viewport: (u32, u32)) {
///         // Begin a render pass that composites OVER the already-drawn scene.
///         let mut pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
///             label: Some("outline pass"),
///             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
///                 view: ctx.view,
///                 resolve_target: None,
///                 depth_slice: None,
///                 ops: wgpu::Operations {
///                     // LoadOp::Load preserves the existing scene pixels.
///                     load: wgpu::LoadOp::Load,
///                     store: wgpu::StoreOp::Store,
///                 },
///             })],
///             depth_stencil_attachment: None,
///             occlusion_query_set: None,
///             timestamp_writes: None,
///             multiview_mask: None,
///         });
///         // set pipeline, bind groups, draw calls ...
///         let _ = (pass, viewport);
///     }
/// }
/// ```
pub trait RenderPlugin {
    /// Record this frame's custom GPU commands.
    ///
    /// - `ctx.view` is the scene render target. Begin a render pass on `ctx.encoder`
    ///   with [`wgpu::LoadOp::Load`] to composite over the already-rendered scene.
    /// - `ctx.format` is that target's [`wgpu::TextureFormat`]. Use it when building
    ///   a [`wgpu::RenderPipeline`] that targets the same render pass.
    /// - `world` is read-only ECS access — read `Camera`, custom resources, entity
    ///   `Transform`s, etc.
    /// - `viewport` is the logical `(width, height)` in pixels matching the sprite pass.
    ///
    /// # ⚠️ `DesignResolution` — apply the letterbox yourself
    ///
    /// Every built-in scene pass (sprite, UI, GPU particle, lighting, text) is handed the
    /// letterbox clip scale; a `RenderPlugin` is not, and `viewport` is the **design** size, not
    /// the window. So a plugin that builds its projection straight from `viewport` draws
    /// correctly with no `DesignResolution` and then leaks outside the letterbox the moment one
    /// is set — the geometry is simply in a different space from everything around it.
    ///
    /// `record` takes `&World`, and both pieces are public, so the correction is one line:
    ///
    /// ```rust,no_run
    /// # use engine::{Letterbox, camera};
    /// # use engine::ecs::World;
    /// # let world = World::new();
    /// # let proj = glam::Mat4::IDENTITY;
    /// let clip_scale = world.resource::<Letterbox>().map(|l| l.clip_scale).unwrap_or(glam::Vec2::ONE);
    /// let proj = camera::apply_letterbox(clip_scale, proj);
    /// ```
    fn record(&mut self, ctx: &mut FrameContext<'_>, world: &World, viewport: (u32, u32));
}
