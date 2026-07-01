//! Inspector (component fields) tab body.

use super::*;

/// Inspector (component fields) tab body for the currently selected entity.
///
/// Used in: docked right panel, overlay Inspector window.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn inspector_tab_body(
    ui: &mut egui::Ui,
    app: &mut App,
    comp_fields: &mut super::InspectorCompFields,
    selected_comp_names: &[&'static str],
    tag_map: &HashMap<Entity, String>,
    _entity_list: &[Entity],
) {
    egui::ScrollArea::vertical()
        .id_salt("docked_inspector_fields")
        .show(ui, |ui| {
            // ── Ambient Light (global scene lighting) ─────────────────────────
            egui::CollapsingHeader::new(tr("Ambient Light", "환경광"))
                .default_open(false)
                .show(ui, |ui| {
                    ambient_light_control(ui, app);
                });

            // Component field editor
            for (_, comp_name, fields) in comp_fields.iter_mut() {
                ui.collapsing(*comp_name, |ui| {
                    egui::Grid::new(*comp_name)
                        .num_columns(2)
                        .spacing([4.0, 2.0])
                        .show(ui, |ui| {
                            for (fname, fval) in fields.iter_mut() {
                                ui.label(*fname);
                                reflect_value_editor(ui, fval);
                                ui.end_row();
                            }
                        });
                });
            }

            // Add / remove components
            if let Some(sel) = app.editor.inspector_selected {
                // ── Tile Paint (shown only for Tilemap entities) ─────────────
                let atlas_dims = app
                    .world
                    .get::<crate::tilemap::Tilemap>(sel)
                    .map(|tm| (tm.atlas.columns, tm.atlas.rows));
                if let Some((cols, rows)) = atlas_dims {
                    let tile_count = cols.saturating_mul(rows);
                    ui.separator();
                    egui::CollapsingHeader::new(tr("Tile Paint", "타일 페인트"))
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.checkbox(
                                &mut app.editor.paint_mode,
                                tr("Paint mode (suppresses gizmo)", "페인트 모드 (기즈모 숨김)"),
                            );
                            // Tool selector (freehand brush / rectangle / bucket fill).
                            ui.horizontal(|ui| {
                                use crate::app::editor::PaintTool;
                                let tool = app.editor.paint_tool;
                                if ui
                                    .selectable_label(tool == PaintTool::Freehand, tr("Brush", "브러시"))
                                    .clicked()
                                {
                                    app.editor.paint_tool = PaintTool::Freehand;
                                }
                                if ui
                                    .selectable_label(tool == PaintTool::Rectangle, tr("Rect", "사각형"))
                                    .clicked()
                                {
                                    app.editor.paint_tool = PaintTool::Rectangle;
                                }
                                if ui
                                    .selectable_label(tool == PaintTool::Bucket, tr("Bucket", "채우기"))
                                    .clicked()
                                {
                                    app.editor.paint_tool = PaintTool::Bucket;
                                }
                            });
                            // Brush size (freehand only).
                            if app.editor.paint_tool == crate::app::editor::PaintTool::Freehand {
                                ui.horizontal(|ui| {
                                    ui.label(tr("Brush:", "브러시:"));
                                    for size in [1u32, 3, 5] {
                                        if ui
                                            .selectable_label(
                                                app.editor.paint_brush == size,
                                                format!("{size}×{size}"),
                                            )
                                            .clicked()
                                        {
                                            app.editor.paint_brush = size;
                                        }
                                    }
                                });
                            }
                            if app.editor.paint_value > tile_count {
                                app.editor.paint_value = tile_count;
                            }
                            // The atlas texture (if registered with egui) lets us draw
                            // each tile as a real thumbnail; else fall back to numbers.
                            let swatch_tex = app.editor.paint_atlas_tex.as_ref().map(|(_, id)| *id);
                            const SWATCH: f32 = 26.0;
                            ui.horizontal_wrapped(|ui| {
                                let cur = app.editor.paint_value;
                                if ui.selectable_label(cur == 0, tr("Erase", "지우기")).clicked() {
                                    app.editor.paint_value = 0;
                                }
                                for tile_id in 0..tile_count {
                                    let value = tile_id + 1;
                                    let clicked = if let Some(tex) = swatch_tex {
                                        let uv = crate::renderer::uv::UvRect::from_grid(
                                            tile_id % cols,
                                            tile_id / cols,
                                            cols,
                                            rows,
                                        );
                                        let img = egui::Image::new(egui::load::SizedTexture::new(
                                            tex,
                                            egui::vec2(SWATCH, SWATCH),
                                        ))
                                        .uv(uv_rect_to_egui(uv));
                                        ui.add(egui::Button::image(img).selected(cur == value))
                                            .on_hover_text(format!("{} {value}", tr("tile", "타일")))
                                            .clicked()
                                    } else {
                                        ui.selectable_label(cur == value, format!("{value}"))
                                            .clicked()
                                    };
                                    if clicked {
                                        app.editor.paint_value = value;
                                    }
                                }
                            });
                            ui.label(
                                egui::RichText::new(tr(
                                    "L paint · R erase · Alt+click pick · 1–9 value · Ctrl+Z undo",
                                    "L 페인트 · R 지우기 · Alt+클릭 스포이드 · 1–9 값 · Ctrl+Z 실행 취소",
                                ))
                                .weak(),
                            );
                        });
                } else {
                    // Selection is not a Tilemap — ensure paint mode is off.
                    app.editor.paint_mode = false;
                }

                // ── Registered inspector panels (built-in + user-defined) ─────
                // Take the vec off `app.editor` to avoid holding a borrow of
                // `app.editor` while the draw closure receives `&mut app`.
                let panels = std::mem::take(&mut app.editor.inspector_panels);
                for p in &panels {
                    if (p.presence)(&app.world, sel) {
                        ui.separator();
                        egui::CollapsingHeader::new(&p.title)
                            .default_open(true)
                            .show(ui, |ui| (p.draw)(ui, app, sel));
                    }
                }
                app.editor.inspector_panels = panels;

                ui.separator();
                ui.strong(tr("Component List", "컴포넌트 목록"));

                // Serde-registered components on this entity can be copied to the clipboard.
                // `component_names_for` checks presence only (no RON serialization), so this
                // is O(registry size) instead of serializing every component value per frame.
                let copyable: std::collections::HashSet<String> = app
                    .world
                    .resource::<crate::prefab::SerdeComponentRegistry>()
                    .map(|r| r.component_names_for(&app.world, sel).into_iter().collect())
                    .unwrap_or_default();

                let mut to_remove: Option<&'static str> = None;
                let mut to_copy: Option<&'static str> = None;
                for &comp_name in selected_comp_names {
                    let removable = comp_name != "Transform"
                        && app.editor.component_removers.contains_key(comp_name);
                    let copyable_now = copyable.contains(comp_name);
                    ui.horizontal(|ui| {
                        ui.label(comp_name);
                        if removable && ui.small_button("✕").on_hover_text(tr("remove", "제거")).clicked() {
                            to_remove = Some(comp_name);
                        }
                        if copyable_now
                            && ui
                                .small_button("⧉")
                                .on_hover_text(tr("copy component", "컴포넌트 복사"))
                                .clicked()
                        {
                            to_copy = Some(comp_name);
                        }
                    });
                }
                if let Some(name) = to_copy {
                    app.copy_component(sel, name);
                }
                if let Some(name) = to_remove {
                    if let Some(remover) = app.editor.component_removers.get(name) {
                        remover(&mut app.world, sel);
                    }
                }
                // Paste the clipboard component onto this entity.
                if let Some((clip_name, _)) = app.editor.component_clipboard.clone() {
                    if ui
                        .button(format!("⧉ {} {clip_name}", tr("Paste", "붙여넣기")))
                        .on_hover_text(tr("apply the copied component to this entity", "복사한 컴포넌트를 이 엔티티에 적용"))
                        .clicked()
                    {
                        app.paste_component(sel);
                    }
                }

                ui.separator();
                // Add component dropdown
                let factory_names: Vec<String> = {
                    let mut names: Vec<String> =
                        app.editor.component_factories.keys().cloned().collect();
                    names.sort();
                    names
                };
                if !factory_names.is_empty() {
                    // Reset the selection if it is empty or stale (e.g. after a scene reload
                    // where the factory map can change and the stored name may no longer be
                    // a valid key — silently no-op-ing the "+ Add" button otherwise).
                    if app.editor.add_component_selected.is_empty()
                        || !factory_names
                            .iter()
                            .any(|n| n == &app.editor.add_component_selected)
                    {
                        app.editor.add_component_selected = factory_names[0].clone();
                    }
                    let cur = app.editor.add_component_selected.clone();
                    egui::ComboBox::from_id_salt("docked_add_comp")
                        .selected_text(&cur)
                        .show_ui(ui, |ui| {
                            for name in &factory_names {
                                ui.selectable_value(
                                    &mut app.editor.add_component_selected,
                                    name.clone(),
                                    name,
                                );
                            }
                        });
                    if ui.button(tr("+ Add", "+ 추가")).clicked() {
                        let chosen = app.editor.add_component_selected.clone();
                        if let Some(factory) = app.editor.component_factories.get(&chosen) {
                            factory(&mut app.world, sel);
                        }
                    }
                }

                // Prefab create / spawn
                ui.separator();
                ui.collapsing(tr("Prefab", "프리팹"), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr("Path:", "경로:"));
                        ui.add(
                            egui::TextEdit::singleline(&mut app.editor.prefab_path)
                                .desired_width(140.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        if ui.button(tr("💾 Save Selected", "💾 선택 항목 저장")).clicked() {
                            let path = app.editor.prefab_path.clone();
                            app.save_selected_as_prefab(sel, &path);
                        }
                        if ui.button(tr("➕ Spawn", "➕ 생성")).clicked() {
                            let path = app.editor.prefab_path.clone();
                            app.spawn_prefab(&path);
                        }
                    });
                    if let Some(status) = &app.editor.prefab_status {
                        ui.label(egui::RichText::new(status).weak());
                    }
                });

                // PrefabInstance / Break Prefab
                let prefab_path = app
                    .world
                    .get::<crate::prefab::PrefabInstance>(sel)
                    .map(|pi| pi.source_path.clone());
                if let Some(path) = prefab_path {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(format!("{}: {path}", tr("Prefab", "프리팹")));
                        if ui.button(tr("Break Prefab", "프리팹 분리")).clicked() {
                            crate::prefab::break_prefab_instance(&mut app.world, sel);
                        }
                    });
                }

                // Tag/name editor
                ui.separator();
                tag_name_editor(ui, sel, tag_map, &mut app.world);
            } else {
                ui.label(tr("(no entity selected)", "(선택된 엔티티 없음)"));
            }
        });
}

/// Convert an engine [`UvRect`](crate::renderer::uv::UvRect) (offset + size) into the
/// `min..max` [`egui::Rect`] that `egui::Image::uv` expects, so a Tile Paint swatch
/// samples exactly its atlas tile.
#[cfg(not(target_arch = "wasm32"))]
fn uv_rect_to_egui(uv: crate::renderer::uv::UvRect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(uv.u_offset, uv.v_offset),
        egui::pos2(uv.u_offset + uv.u_size, uv.v_offset + uv.v_size),
    )
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod swatch_tests {
    use super::uv_rect_to_egui;
    use crate::renderer::uv::UvRect;

    #[test]
    fn uv_rect_maps_offset_size_to_min_max() {
        // Tile (col 1, row 0) of a 2×2 atlas → offset (0.5, 0.0), size (0.5, 0.5).
        let uv = UvRect::from_grid(1, 0, 2, 2);
        let r = uv_rect_to_egui(uv);
        assert!((r.min.x - 0.5).abs() < 1e-6, "min.x");
        assert!((r.min.y - 0.0).abs() < 1e-6, "min.y");
        assert!((r.max.x - 1.0).abs() < 1e-6, "max.x");
        assert!((r.max.y - 0.5).abs() < 1e-6, "max.y");
    }

    #[test]
    fn uv_rect_full_covers_whole_texture() {
        let r = uv_rect_to_egui(UvRect::FULL);
        assert_eq!(r.min, egui::pos2(0.0, 0.0));
        assert_eq!(r.max, egui::pos2(1.0, 1.0));
    }
}
