use crate::app::render_state::RenderState;
use crate::renderer::GpuContext;

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

/// Submits the pending egui output (`render.egui_output`) onto `view` using `render.egui_renderer`:
/// updates the texture deltas, updates the vertex/index buffers, records the overlay render pass,
/// submits its own command encoder, frees the freed textures, then restores the renderer. A no-op
/// when either the renderer or the pending output is absent (pre-GPU-init / nothing to draw).
///
/// `guard_callbacks` selects the two call sites' only difference: the **final surface overlay**
/// passes `true` — paint callbacks are unsupported there and skipped with a warn (for render-pass
/// lifetime safety); the **docked placeholder** passes `false` and records directly (it never
/// produces callbacks). Behavior is byte-identical to the two inlined blocks this replaces.
pub(super) fn submit_egui(
    render: &mut RenderState,
    gpu: &GpuContext,
    view: &wgpu::TextureView,
    guard_callbacks: bool,
) {
    let (Some(mut er), Some((paint_jobs, textures_delta, ppp))) =
        (render.egui_renderer.take(), render.egui_output.take())
    else {
        return;
    };
    let screen_desc = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [gpu.config.width, gpu.config.height],
        pixels_per_point: ppp,
    };
    for (id, delta) in &textures_delta.set {
        er.update_texture(&gpu.device, &gpu.queue, *id, delta);
    }
    let mut egui_enc = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("egui encoder"),
        });
    er.update_buffers(
        &gpu.device,
        &gpu.queue,
        &mut egui_enc,
        &paint_jobs,
        &screen_desc,
    );
    if guard_callbacks && paint_jobs_contain_callbacks(&paint_jobs) {
        log::warn!(
            "egui paint callbacks are unsupported and were skipped to preserve render-pass lifetime safety"
        );
    } else {
        egui_render_pass(&er, &mut egui_enc, &paint_jobs, &screen_desc, view);
    }
    gpu.queue.submit(std::iter::once(egui_enc.finish()));
    for id in &textures_delta.free {
        er.free_texture(id);
    }
    render.egui_renderer = Some(er);
}

pub(super) fn paint_jobs_contain_callbacks(paint_jobs: &[egui::ClippedPrimitive]) -> bool {
    paint_jobs
        .iter()
        .any(|job| matches!(job.primitive, egui::epaint::Primitive::Callback(_)))
}

fn egui_render_pass(
    er: &egui_wgpu::Renderer,
    enc: &mut wgpu::CommandEncoder,
    paint_jobs: &[egui::ClippedPrimitive],
    screen_desc: &egui_wgpu::ScreenDescriptor,
    view: &wgpu::TextureView,
) {
    let rpass = crate::renderer::common::begin_color_pass(enc, "egui", view, wgpu::LoadOp::Load);
    // wgpu 29 provides `RenderPass::forget_lifetime()` for exactly this use case:
    // `egui_wgpu::Renderer::render` requires `RenderPass<'static>`, and
    // `forget_lifetime` safely opts out of the borrow-of-encoder lifetime check.
    // Both the renderer and the pass are used only within this function and do not
    // escape, so no dangling reference is possible.
    let mut rpass_s = rpass.forget_lifetime();
    er.render(&mut rpass_s, paint_jobs, screen_desc);
}
