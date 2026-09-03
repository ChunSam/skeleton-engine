//! Docked editor UI — native only; excluded from wasm builds entirely.
#![cfg(not(target_arch = "wasm32"))]

use super::grid_overlay::draw_editor_grid;
use super::lighting_panel::ambient_light_control;
use super::*;
use crate::app::editor::theme;
use crate::app::editor::tr;
use crate::app::editor::EditorMode;
use crate::app::editor::EntitySortMode;

mod assets_tab;
mod context_menu;
mod entities_tab;
mod entity_kind;
mod inspector_tab;
mod save_load;
mod scene_tab;
mod toolbar;

use toolbar::docked_toolbar;

pub(in crate::app) use assets_tab::assets_tab_body;
pub(in crate::app) use entities_tab::entities_tab_body;
pub(in crate::app) use inspector_tab::inspector_tab_body;
pub(in crate::app) use save_load::{do_load_scene, do_save_scene, save_load_controls};
pub(in crate::app) use scene_tab::scene_tab_body;

/// Full docked editor layout (Package 2).
///
/// Layout order (egui panel rules require this order):
///   1. Top toolbar panel
///   2. Bottom assets panel
///   3. Left entities/scene panel
///   4. Right inspector panel
///   5. Central panel  ← game image; also writes `EditorState::central_rect`
///
/// Panels are added in the order required by egui — top/bottom first, then
/// left/right, and CentralPanel last. egui 0.34 deprecates the top-level
/// `Panel::show(ctx, ...)` API in favour of `show_inside(ui, ...)`.  At the
/// top level there is no parent `Ui`, so we use the deprecated `show(ctx)`
/// path with `#[allow(deprecated)]`, matching Package 1's convention.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn update_docked_ui(
    ctx: &egui::Context,
    app: &mut App,
    comp_fields: &mut super::InspectorCompFields,
    entity_list: &[Entity],
    tag_map: &HashMap<Entity, String>,
    selected_comp_names: &[&'static str],
    scene_graph_data: &[(Entity, Option<Entity>)],
    children_map: &HashMap<Entity, Vec<Entity>>,
    root_entities: &[Entity],
) {
    // ── 1. Top toolbar ───────────────────────────────────────────────────────
    // `Panel::top(...).show(ctx, ...)` is the only top-level path in egui 0.34;
    // `show_inside` requires a parent `Ui` that does not exist at this level.
    // The deprecation warning is suppressed — see Package 1 comment for details.
    #[allow(deprecated)]
    egui::Panel::top("docked_toolbar")
        .exact_size(28.0)
        .show(ctx, |ui| {
            docked_toolbar(ui, app);
        });

    // ── 2. Bottom panel: Assets | Data Tables | Audio ───────────────────────
    // High upper bound so the data-table grid can be dragged tall enough to read
    // many rows at once; egui still clamps the drag to the real available height,
    // so this is effectively "free" without letting the panel cover the toolbar.
    #[allow(deprecated)]
    egui::Panel::bottom("docked_assets")
        .default_size(theme::BOTTOM_PANEL_DEFAULT_H)
        .size_range(theme::BOTTOM_PANEL_MIN_H..=theme::BOTTOM_PANEL_MAX_H)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(app.editor.bottom_tab == 0, tr("Assets", "에셋"))
                    .clicked()
                {
                    app.editor.bottom_tab = 0;
                }
                if ui
                    .selectable_label(
                        app.editor.bottom_tab == 1,
                        tr("Data Tables", "데이터 테이블"),
                    )
                    .clicked()
                {
                    app.editor.bottom_tab = 1;
                }
                if ui
                    .selectable_label(app.editor.bottom_tab == 2, tr("Audio", "오디오"))
                    .clicked()
                {
                    app.editor.bottom_tab = 2;
                }
            });
            ui.separator();
            match app.editor.bottom_tab {
                1 => super::data_table_panel_body(ui, app),
                2 => super::audio_mixer_panel_body(ui, app),
                _ => assets_tab_body(ui, app),
            }
        });

    // ── 3. Left entities / scene panel ───────────────────────────────────────
    #[allow(deprecated)]
    egui::Panel::left("docked_left")
        .default_size(theme::LEFT_PANEL_DEFAULT_W)
        .size_range(theme::LEFT_PANEL_MIN_W..=theme::LEFT_PANEL_MAX_W)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(app.editor.inspector_tab == 0, tr("Entities", "엔티티"))
                    .clicked()
                {
                    app.editor.inspector_tab = 0;
                }
                if ui
                    .selectable_label(app.editor.inspector_tab == 2, tr("Scene", "씬"))
                    .clicked()
                {
                    app.editor.inspector_tab = 2;
                }
            });
            ui.separator();
            if app.editor.inspector_tab == 2 {
                scene_tab_body(
                    ui,
                    app,
                    tag_map,
                    scene_graph_data,
                    children_map,
                    root_entities,
                );
            } else {
                entities_tab_body(ui, app, entity_list, tag_map);
            }
        });

    // ── 4. Right inspector panel ─────────────────────────────────────────────
    #[allow(deprecated)]
    egui::Panel::right("docked_inspector")
        .default_size(theme::RIGHT_PANEL_DEFAULT_W)
        .size_range(theme::RIGHT_PANEL_MIN_W..=theme::RIGHT_PANEL_MAX_W)
        .resizable(true)
        .show(ctx, |ui| {
            ui.strong(tr("Inspector", "인스펙터"));
            ui.separator();
            inspector_tab_body(
                ui,
                app,
                comp_fields,
                selected_comp_names,
                tag_map,
                entity_list,
            );
            save_load_controls(ui, app, entity_list, tag_map);
        });

    // ── 5. Central panel (game image) ─────────────────────────────────────────
    // This must be last. After layout we capture the inner rect and write it to
    // `editor.central_rect` so that the RT/ViewportSize logic tracks real panel bounds.
    #[allow(deprecated)]
    let central_response = egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(theme::VIEWPORT_FRAME_FILL))
        .show(ctx, |ui| {
            if let Some(tex) = app.editor.docked_texture_id {
                let avail = ui.available_size();
                let img_rect = ui.image((tex, avail)).rect;
                if app.editor.show_grid {
                    draw_editor_grid(ui, app, img_rect);
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(tr("(no game frame yet)", "(게임 프레임 없음)"));
                });
            }
        });

    // Write the real central rect in LOGICAL points so the RT/ViewportSize
    // delegation in schedule.rs picks up the actual panel geometry.
    // `response.rect` is the panel's own rect. (`inner_rect`, which this named until
    // 2026-09-03, is the content area inside the margins — a different rectangle.)
    app.editor.central_rect = Some(central_response.response.rect);
}
