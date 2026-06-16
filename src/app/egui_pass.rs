/// Begin an egui frame.
///
/// Takes the pending raw input from `egui_winit::State`, calls `ctx.begin_pass`, and
/// returns the `egui::Context` that must be passed to every egui draw call this frame
/// and ultimately handed back to [`end_egui_frame`].
///
/// Returns `None` when the window or egui state is not yet available (pre-GPU-init),
/// or when no `DebugUi` resource is registered.
pub(super) fn begin_egui_frame(
    window: Option<&winit::window::Window>,
    egui_state: Option<&mut egui_winit::State>,
    debug_ui: Option<&super::DebugUi>,
) -> Option<egui::Context> {
    let window = window?;
    let state = egui_state?;
    let debug_ui = debug_ui?;
    let ctx = debug_ui.ctx().clone();
    let raw_input = state.take_egui_input(window);
    ctx.begin_pass(raw_input);
    Some(ctx)
}

/// End an egui frame: tessellate, merge pending texture deltas, and store the result
/// in `egui_output` for [`render()`][super::App::render] to consume.
///
/// **Ordering:** reads and clears the PREVIOUS `egui_output` (the pending delta) BEFORE
/// writing the new one. This preserves the old → new delta ordering documented on
/// [`super::schedule::merge_textures_delta`].
pub(super) fn end_egui_frame(
    ctx: egui::Context,
    window: Option<&winit::window::Window>,
    egui_output: &mut Option<(Vec<egui::ClippedPrimitive>, egui::TexturesDelta, f32)>,
) {
    use super::schedule::merge_textures_delta;
    let ppp = window.map(|w| w.scale_factor() as f32).unwrap_or(1.0);
    let full_output = ctx.end_pass();
    let paint_jobs = ctx.tessellate(full_output.shapes, ppp);
    let textures_delta = merge_textures_delta(
        egui_output.take().map(|(_, pending, _)| pending),
        full_output.textures_delta,
    );
    *egui_output = Some((paint_jobs, textures_delta, ppp));
}

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
