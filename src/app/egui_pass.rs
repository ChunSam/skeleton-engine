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
    // Safety: see the invariant documented in the doc comment above.
    unsafe {
        let er_s: &'static egui_wgpu::Renderer = &*(er as *const _);
        let mut rpass_s: wgpu::RenderPass<'static> = std::mem::transmute(rpass);
        er_s.render(&mut rpass_s, paint_jobs, screen_desc);
    }
}
