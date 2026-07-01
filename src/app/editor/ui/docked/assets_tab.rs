//! Asset browser tab body.

use super::*;

/// Asset browser body.
///
/// Used in: docked bottom panel, overlay Inspector window (tab 1).
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn assets_tab_body(ui: &mut egui::Ui, app: &App) {
    let entries = app
        .world
        .resource::<AssetServer>()
        .map(|a| a.image_list())
        .unwrap_or_default();
    if entries.is_empty() {
        ui.label(tr("(No images loaded)", "(로드된 이미지 없음)"));
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt("docked_assets_browser")
        .show(ui, |ui| {
            egui::Grid::new("docked_asset_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    for entry in &entries {
                        let filename = std::path::Path::new(&entry.path)
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_else(|| entry.path.clone());
                        ui.label("[ ]");
                        ui.vertical(|ui| {
                            ui.label(&filename);
                            ui.small(format!("{}×{}", entry.width, entry.height));
                        });
                        ui.end_row();
                    }
                });
        });
}
