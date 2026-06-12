/// Minimal docked editor layout (Package 1).
///
/// Draws a top menu bar with an informational label, then a `CentralPanel`
/// containing the game viewport image.  No side panels or real functionality —
/// package 2 builds those.
///
/// Only compiled on native targets; the function is never called on wasm.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn update_docked_ui(
    ctx: &egui::Context,
    texture_id: Option<egui::TextureId>,
    central_rect: Option<egui::Rect>,
) {
    // Top bar with a status label.
    // `Panel::top(...).show(ctx, ...)` is the top-level API in egui 0.34 (the alternative
    // `show_inside` requires a `&mut Ui` from a surrounding panel, which does not exist at
    // the top level).  The deprecation warning on `.show` is suppressed here; package 2
    // can switch to `show_inside` once it introduces a proper root `Ui`.
    #[allow(deprecated)]
    egui::Panel::top("docked_top_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Docked Editor (F2 to exit) \u{2014} package 2 adds panels");
        });
    });

    // Central panel: fill with the game texture (or a placeholder).
    // Same note about `.show` — top-level usage, deprecated warning suppressed.
    #[allow(deprecated)]
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(20, 20, 25)))
        .show(ctx, |ui| {
            if let (Some(tex), Some(rect)) = (texture_id, central_rect) {
                let size = egui::vec2(rect.width(), rect.height());
                ui.image((tex, size));
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("(no game frame yet)");
                });
            }
        });
}
