pub(super) fn paint_jobs_contain_callbacks(paint_jobs: &[egui::ClippedPrimitive]) -> bool {
    paint_jobs
        .iter()
        .any(|job| matches!(job.primitive, egui::epaint::Primitive::Callback(_)))
}

pub(super) fn egui_render_pass(
    er: &egui_wgpu::Renderer,
    enc: &mut wgpu::CommandEncoder,
    paint_jobs: &[egui::ClippedPrimitive],
    screen_desc: &egui_wgpu::ScreenDescriptor,
    view: &wgpu::TextureView,
) {
    let rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("egui"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
    });
    // SAFETY: `er` and `rpass` both outlive this function call — `er` is borrowed
    // for the duration of the enclosing `render()` call, and `rpass` is dropped at
    // the closing `}` of this block (before `enc` is consumed by `finish()`).
    // The transmute to `'static` is required because `egui_wgpu::Renderer::render`
    // takes `RenderPass<'static>`, but the `RenderPass` here borrows `enc` which has
    // a non-`'static` lifetime.  Both the renderer and the pass are used only inside
    // this block and do not escape, so no dangling reference is possible.
    unsafe {
        let er_s: &'static egui_wgpu::Renderer = &*(er as *const _);
        let mut rpass_s: wgpu::RenderPass<'static> = std::mem::transmute(rpass);
        er_s.render(&mut rpass_s, paint_jobs, screen_desc);
    }
}
