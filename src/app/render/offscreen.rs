use super::super::{App, OffscreenRenderInfo};
use crate::app::render_state::RenderState;
use crate::ecs::World;
use crate::renderer::{FrameContext, GpuContext};
use crate::resources::WindowConfig;

impl App {
    /// Render every `OffscreenCamera` entity's scene into its `RenderTarget` texture, each as
    /// its **own** command submission. The `SpriteRenderer`'s camera uniform is a single shared
    /// buffer, so each offscreen draw must be submitted together with its camera write before the
    /// main pass overwrites it. Extracted from `render()`; uses its own encoders and does not touch
    /// the main frame encoder, render target, or post/lighting state.
    pub(in crate::app) fn render_offscreen_targets(
        render: &mut RenderState,
        world: &mut World,
        gpu: &GpuContext,
    ) {
        // Step 1: collect render info for each offscreen camera.
        // We call `Texture::create_view` up front to obtain owned `TextureView` handles.
        // A `TextureView` is a zero-cost reference-counted GPU handle: creating a second
        // view on the same texture is safe and free — it does not copy GPU memory.
        // This replaces the previous `*const wgpu::TextureView` raw-pointer scheme that
        // required `unsafe` and could dangle if the HashMap reallocated.
        let offscreen_cams: Vec<(String, crate::camera::Camera, u32)> = world
            .query::<crate::components::OffscreenCamera>()
            .map(|(_, oc)| (oc.target.clone(), oc.camera, oc.layer_mask))
            .collect();

        let rt_info: Vec<OffscreenRenderInfo> = offscreen_cams
            .into_iter()
            .filter_map(|(name, cam, layer_mask)| {
                render.render_targets.get(&name).map(|rt| {
                    // Create a fresh owned TextureView each frame (zero-cost GPU handle).
                    // Safe to use after this point even if render_targets is later touched.
                    let owned_view = rt
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    (
                        name,
                        cam,
                        rt.width,
                        rt.height,
                        owned_view,
                        std::sync::Arc::clone(&rt.bind_group),
                        layer_mask,
                        rt.clear_color,
                        rt.format,
                    )
                })
            })
            .collect();

        for (target_name, cam, rt_w, rt_h, rt_view, bg, layer_mask, rt_clear_color, rt_format) in
            rt_info
        {
            // ① Swap camera — if no prior camera existed remove it after render, otherwise restore
            let saved_cam = world.resource::<crate::camera::Camera>().copied();
            world.insert_resource(cam);

            // ② The owned TextureView was created up front; no unsafe dereference needed.
            let rt_view = &rt_view;

            // Each offscreen target is rendered with its **own command submission**.
            // The SpriteRenderer's camera uniform is a single shared buffer (`camera_buf`)
            // updated via `queue.write_buffer`. Within a single submit only the **last**
            // write to that buffer takes effect, so recording the offscreen draw and the main
            // pass in the same submit would cause the offscreen target to render with the
            // (later-written) **main camera**.
            // Submitting here ties this target's camera write and its draw together as a pair.
            let mut oenc = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("offscreen encoder"),
                });

            // ③ Clear the RT — use per-target color if set, else inherit WindowConfig::clear_color.
            {
                let [cr, cg, cb, ca] = rt_clear_color.unwrap_or_else(|| {
                    world
                        .resource::<WindowConfig>()
                        .map(|c| c.clear_color)
                        .unwrap_or([0.0, 0.0, 0.0, 1.0])
                });
                let _pass = oenc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("offscreen clear"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: rt_view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: cr,
                                g: cg,
                                b: cb,
                                a: ca,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });
            }

            // ④ Render sprites → RT (layer_mask prevents self-capture). Use the RT's own format so
            // the sprite pass picks a pipeline whose color-target format matches (HDR / linear RTs).
            if let Some(sr) = &mut render.sprite_renderer {
                sr.render(
                    &mut FrameContext {
                        device: &gpu.device,
                        queue: &gpu.queue,
                        view: rt_view,
                        format: rt_format,
                        encoder: &mut oenc,
                    },
                    world,
                    rt_w,
                    rt_h,
                    layer_mask,
                );
            }

            // Immediately flush this target's camera write + draw. (Applied before the main
            // pass's camera write; the RT texture is filled before the main pass samples it.)
            gpu.queue.submit(std::iter::once(oenc.finish()));

            // ⑤ Restore camera — remove it if it was absent before to avoid polluting the World
            match saved_cam {
                Some(c) => world.insert_resource(c),
                None => {
                    world.remove_resource::<crate::camera::Camera>();
                }
            }

            // ⑥ Register the RT bind_group with the sprite renderer (sampleable via Sprite.texture key)
            if let Some(sr) = &mut render.sprite_renderer {
                sr.register_render_target(&target_name, bg);
            }
        }
    }
}
