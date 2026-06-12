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
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
        multiview_mask: None,
    });
    // wgpu 29 provides `RenderPass::forget_lifetime()` for exactly this use case:
    // `egui_wgpu::Renderer::render` requires `RenderPass<'static>`, and
    // `forget_lifetime` safely opts out of the borrow-of-encoder lifetime check.
    // Both the renderer and the pass are used only within this function and do not
    // escape, so no dangling reference is possible.
    let mut rpass_s = rpass.forget_lifetime();
    er.render(&mut rpass_s, paint_jobs, screen_desc);
}
