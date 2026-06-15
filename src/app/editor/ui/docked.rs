//! Docked editor UI — native only; excluded from wasm builds entirely.
#![cfg(not(target_arch = "wasm32"))]

use super::*;
use crate::app::editor::EditorMode;

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
    comp_fields: &mut Vec<(&'static str, Vec<(&'static str, ReflectValue)>)>,
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
    #[allow(deprecated)]
    egui::Panel::bottom("docked_assets")
        .default_size(150.0)
        .size_range(60.0..=300.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(app.editor.bottom_tab == 0, "Assets")
                    .clicked()
                {
                    app.editor.bottom_tab = 0;
                }
                if ui
                    .selectable_label(app.editor.bottom_tab == 1, "Data Tables")
                    .clicked()
                {
                    app.editor.bottom_tab = 1;
                }
                if ui
                    .selectable_label(app.editor.bottom_tab == 2, "Audio")
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
        .default_size(260.0)
        .size_range(120.0..=500.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(app.editor.inspector_tab == 0, "Entities")
                    .clicked()
                {
                    app.editor.inspector_tab = 0;
                }
                if ui
                    .selectable_label(app.editor.inspector_tab == 2, "Scene")
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
        .default_size(300.0)
        .size_range(120.0..=600.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.strong("Inspector");
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
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(20, 20, 25)))
        .show(ctx, |ui| {
            if let Some(tex) = app.editor.docked_texture_id {
                let avail = ui.available_size();
                let img_rect = ui.image((tex, avail)).rect;
                if app.editor.show_grid {
                    draw_editor_grid(ui, app, img_rect);
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("(no game frame yet)");
                });
            }
        });

    // Write the real central rect in LOGICAL points so the RT/ViewportSize
    // delegation in schedule.rs picks up the actual panel geometry.
    // `inner_rect` is the content area inside panel margins.
    app.editor.central_rect = Some(central_response.response.rect);
}

// ── Toolbar ─────────────────────────────────────────────────────────────────

/// Toolbar contents: ▶/⏸, ⏭ step, Snap, scene path + Save/Load, Exit(F2).
#[cfg(not(target_arch = "wasm32"))]
fn docked_toolbar(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        // ▶ / ⏸ toggle
        let pause_label = if app.editor.paused {
            "▶ Resume"
        } else {
            "⏸ Pause"
        };
        if ui.button(pause_label).clicked() {
            app.editor.paused = !app.editor.paused;
            if !app.editor.paused {
                app.editor.step_once = false;
            }
        }

        // ⏭ step-one-frame — only when paused
        ui.add_enabled_ui(app.editor.paused, |ui| {
            if ui.button("⏭ Step").clicked() {
                app.editor.step_once = true;
            }
        });

        ui.separator();

        // Snap toggle + grid size
        ui.checkbox(&mut app.editor.snap_enabled, "Snap");
        if app.editor.snap_enabled {
            ui.add(
                egui::DragValue::new(&mut app.editor.snap_size)
                    .range(1.0..=128.0)
                    .speed(1.0)
                    .suffix(" px"),
            );
        }
        // Grid overlay toggle (world-aligned to the snap size).
        ui.checkbox(&mut app.editor.show_grid, "Grid");
        // Debug bounds/colliders overlay toggle.
        ui.checkbox(&mut app.editor.show_bounds, "Bounds");
        // Pathfinding-grid overlay toggle (per-Tilemap walkable/blocked cells).
        ui.checkbox(&mut app.editor.show_pathgrid, "Path")
            .on_hover_text("show the pathfinding grid (non-zero tile = blocked) for each Tilemap");
        // Persist current editor preferences now (also auto-saved on closing the editor).
        if ui
            .button("💾 Set.")
            .on_hover_text("save editor settings (snap / grid / paint tool)")
            .clicked()
        {
            app.save_editor_settings();
        }

        ui.separator();

        // Scene path + Save + Load
        ui.add(egui::TextEdit::singleline(&mut app.editor.editor_save_path).desired_width(160.0));

        if ui.button("💾 Save").clicked() {
            do_save_scene(app);
        }
        if ui.button("📂 Load").clicked() {
            do_load_scene(app);
        }

        if let Some(msg) = &app.editor.editor_save_status {
            ui.small(msg.as_str());
        }
        if let Some(msg) = &app.editor.editor_load_status {
            ui.small(msg.as_str());
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Exit (F2)").clicked() {
                app.editor.mode = EditorMode::Off;
                app.editor.paused = false;
                app.editor.step_once = false;
                if let Some(debug_ui) = app.world.resource_mut::<DebugUi>() {
                    debug_ui.set_enabled(false);
                }
            }
        });
    });
}

// ── Shared tab-body functions ────────────────────────────────────────────────
//
// Each of these renders a self-contained piece of UI into the given `ui`.
// They are called from BOTH the docked panels (above) and the overlay windows
// (`ui/mod.rs`).  The overlay windows must NOT change behaviour: the bodies
// only mutate `app.editor.*` and `app.world` through well-defined paths.

/// Entity list tab body.  Shows a flat, multi-selectable list of all entities.
///
/// Used in: docked left panel (Entities tab), overlay Inspector window.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn entities_tab_body(
    ui: &mut egui::Ui,
    app: &mut App,
    entity_list: &[Entity],
    tag_map: &HashMap<Entity, String>,
) {
    // Action row: + New Entity, Delete, Duplicate
    ui.horizontal(|ui| {
        if ui.button("＋ New Entity").clicked() {
            let e = app.world.spawn();
            app.world
                .add_component(e, crate::components::Transform::default());
            app.world
                .add_component(e, crate::prefab::Tag("New Entity".into()));
            app.editor.inspector_selected = Some(e);
            app.editor.selected_entities = vec![e];
            app.editor
                .cmd_history
                .push(super::super::EditorCmd::CreateEntity {
                    entity: e,
                    def: None,
                });
        }
        if let Some(sel) = app.editor.inspector_selected {
            if ui
                .add_enabled(true, egui::Button::new("🗑 Delete"))
                .clicked()
            {
                // Capture the full entity def before despawning so undo can restore
                // all components (not just tag/transform/sprite).
                let def = super::super::entity_to_def(&app.world, sel).unwrap_or_default();
                app.editor
                    .cmd_history
                    .push(super::super::EditorCmd::DeleteEntity { entity: None, def });
                app.world.despawn(sel);
                app.editor.inspector_selected = None;
                app.editor.selected_entities.retain(|&x| x != sel);
            }
            if ui
                .add_enabled(true, egui::Button::new("⎘ Duplicate"))
                .clicked()
            {
                if let Some(new_entity) = app.world.clone_entity(sel) {
                    if let Some(t) = app
                        .world
                        .get_mut::<crate::components::Transform>(new_entity)
                    {
                        t.position += glam::Vec2::new(16.0, 16.0);
                    }
                    app.editor.inspector_selected = Some(new_entity);
                    app.editor.selected_entities = vec![new_entity];
                    // Capture the def after offset so redo restores the same position.
                    let def = super::super::entity_to_def(&app.world, new_entity);
                    app.editor
                        .cmd_history
                        .push(super::super::EditorCmd::CreateEntity {
                            entity: new_entity,
                            def,
                        });
                }
            }
        }
    });
    ui.separator();

    // Search box: filters the list by entity label (case-insensitive substring).
    ui.horizontal(|ui| {
        ui.label("🔍");
        ui.text_edit_singleline(&mut app.editor.entity_filter);
        if !app.editor.entity_filter.is_empty() && ui.small_button("✕").clicked() {
            app.editor.entity_filter.clear();
        }
    });

    // Flat entity list with multi-select
    let filter = app.editor.entity_filter.clone();
    egui::ScrollArea::vertical()
        .id_salt("docked_ent_list")
        .show(ui, |ui| {
            for &e in entity_list {
                let label = entity_label(e, tag_map);
                if !super::super::entity_matches_filter(&label, &filter) {
                    continue;
                }
                let is_sel = app.editor.selected_entities.contains(&e);
                let resp = ui.selectable_label(is_sel, &label);
                if resp.clicked() {
                    apply_multiselect(
                        e,
                        ui.input(|i| i.modifiers.ctrl),
                        &mut app.editor.selected_entities,
                        &mut app.editor.inspector_selected,
                    );
                }
            }
        });
}

/// Scene graph tab body.  Shows a parent→children indented tree.
///
/// Used in: docked left panel (Scene tab), overlay Inspector window (tab 2).
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn scene_tab_body(
    ui: &mut egui::Ui,
    app: &mut App,
    tag_map: &HashMap<Entity, String>,
    scene_graph_data: &[(Entity, Option<Entity>)],
    children_map: &HashMap<Entity, Vec<Entity>>,
    root_entities: &[Entity],
) {
    let _ = scene_graph_data; // used implicitly through root_entities + children_map

    let mut clicked_entity: Option<Entity> = None;
    let mut ctrl_clicked: bool = false;

    egui::ScrollArea::vertical()
        .id_salt("docked_scene_graph")
        .show(ui, |ui| {
            let mut stack: Vec<(Entity, usize)> =
                root_entities.iter().rev().map(|&e| (e, 0)).collect();
            while let Some((entity, depth)) = stack.pop() {
                let name = entity_label(entity, tag_map);
                let is_selected = app.editor.selected_entities.contains(&entity);
                let has_children = children_map
                    .get(&entity)
                    .map(|c| !c.is_empty())
                    .unwrap_or(false);
                let prefix = if has_children { "▶ " } else { "  " };
                let label_text = format!("{}{}{}", "  ".repeat(depth), prefix, name);
                let response = ui.selectable_label(is_selected, &label_text);
                if response.clicked() {
                    clicked_entity = Some(entity);
                    ctrl_clicked = ui.input(|i| i.modifiers.ctrl);
                }
                if let Some(ch) = children_map.get(&entity) {
                    for &child in ch.iter().rev() {
                        stack.push((child, depth + 1));
                    }
                }
            }
        });

    if let Some(e) = clicked_entity {
        apply_multiselect(
            e,
            ctrl_clicked,
            &mut app.editor.selected_entities,
            &mut app.editor.inspector_selected,
        );
    }

    ui.separator();
    if let Some(sel) = app.editor.inspector_selected {
        tag_name_editor(ui, sel, tag_map, &mut app.world);
    } else {
        ui.label("(no entity selected)");
    }
}

/// Inspector (component fields) tab body for the currently selected entity.
///
/// Used in: docked right panel, overlay Inspector window.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn inspector_tab_body(
    ui: &mut egui::Ui,
    app: &mut App,
    comp_fields: &mut Vec<(&'static str, Vec<(&'static str, ReflectValue)>)>,
    selected_comp_names: &[&'static str],
    tag_map: &HashMap<Entity, String>,
    _entity_list: &[Entity],
) {
    egui::ScrollArea::vertical()
        .id_salt("docked_inspector_fields")
        .show(ui, |ui| {
            // ── Ambient Light (global scene lighting) ─────────────────────────
            egui::CollapsingHeader::new("Ambient Light")
                .default_open(false)
                .show(ui, |ui| {
                    ambient_light_control(ui, app);
                });

            // Component field editor
            for (comp_name, fields) in comp_fields.iter_mut() {
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
                    egui::CollapsingHeader::new("Tile Paint")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.checkbox(
                                &mut app.editor.paint_mode,
                                "Paint mode (suppresses gizmo)",
                            );
                            // Tool selector (freehand brush / rectangle / bucket fill).
                            ui.horizontal(|ui| {
                                use crate::app::editor::PaintTool;
                                let tool = app.editor.paint_tool;
                                if ui
                                    .selectable_label(tool == PaintTool::Freehand, "Brush")
                                    .clicked()
                                {
                                    app.editor.paint_tool = PaintTool::Freehand;
                                }
                                if ui
                                    .selectable_label(tool == PaintTool::Rectangle, "Rect")
                                    .clicked()
                                {
                                    app.editor.paint_tool = PaintTool::Rectangle;
                                }
                                if ui
                                    .selectable_label(tool == PaintTool::Bucket, "Bucket")
                                    .clicked()
                                {
                                    app.editor.paint_tool = PaintTool::Bucket;
                                }
                            });
                            // Brush size (freehand only).
                            if app.editor.paint_tool == crate::app::editor::PaintTool::Freehand {
                                ui.horizontal(|ui| {
                                    ui.label("Brush:");
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
                                if ui.selectable_label(cur == 0, "Erase").clicked() {
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
                                            .on_hover_text(format!("tile {value}"))
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
                                egui::RichText::new(
                                    "L paint · R erase · Alt+click pick · 1–9 value · Ctrl+Z undo",
                                )
                                .weak(),
                            );
                        });
                } else {
                    // Selection is not a Tilemap — ensure paint mode is off.
                    app.editor.paint_mode = false;
                }

                // ── Particle Tuner (shown only for ParticleEmitter entities) ──
                if app
                    .world
                    .get::<crate::particle::ParticleEmitter>(sel)
                    .is_some()
                {
                    ui.separator();
                    egui::CollapsingHeader::new("Particle Tuner")
                        .default_open(true)
                        .show(ui, |ui| {
                            particle_tuner_grid(ui, app, sel);
                            if ui
                                .button("↺ Reset to Default")
                                .on_hover_text("reset all fields (keeps the texture)")
                                .clicked()
                            {
                                app.reset_particle_emitter(sel);
                            }
                            ui.label(
                                egui::RichText::new(
                                    "edits apply live while the sim runs (unpause)",
                                )
                                .weak(),
                            );
                        });
                }

                // ── Point Light editor (shown only for PointLight entities) ───
                if app
                    .world
                    .get::<crate::components::PointLight>(sel)
                    .is_some()
                {
                    ui.separator();
                    egui::CollapsingHeader::new("Point Light")
                        .default_open(true)
                        .show(ui, |ui| {
                            point_light_grid(ui, app, sel);
                            if ui
                                .button("↺ Reset to Default")
                                .on_hover_text("reset color / radius / intensity / height")
                                .clicked()
                            {
                                app.reset_point_light(sel);
                            }
                            ui.label(
                                egui::RichText::new(
                                    "the entity's Transform position is the light position",
                                )
                                .weak(),
                            );
                        });
                }

                ui.separator();
                ui.strong("Component List");

                // Serde-registered components on this entity can be copied to the clipboard.
                let copyable: std::collections::HashSet<String> = app
                    .world
                    .resource::<crate::prefab::SerdeComponentRegistry>()
                    .map(|r| r.serialize_entity(&app.world, sel).into_keys().collect())
                    .unwrap_or_default();

                let mut to_remove: Option<&'static str> = None;
                let mut to_copy: Option<&'static str> = None;
                for &comp_name in selected_comp_names {
                    let removable = comp_name != "Transform"
                        && app.editor.component_removers.contains_key(comp_name);
                    let copyable_now = copyable.contains(comp_name);
                    ui.horizontal(|ui| {
                        ui.label(comp_name);
                        if removable && ui.small_button("✕").on_hover_text("remove").clicked() {
                            to_remove = Some(comp_name);
                        }
                        if copyable_now
                            && ui
                                .small_button("⧉")
                                .on_hover_text("copy component")
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
                        .button(format!("⧉ Paste {clip_name}"))
                        .on_hover_text("apply the copied component to this entity")
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
                    if app.editor.add_component_selected.is_empty() {
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
                    if ui.button("+ Add").clicked() {
                        let chosen = app.editor.add_component_selected.clone();
                        if let Some(factory) = app.editor.component_factories.get(&chosen) {
                            factory(&mut app.world, sel);
                        }
                    }
                }

                // Prefab create / spawn
                ui.separator();
                ui.collapsing("Prefab", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Path:");
                        ui.add(
                            egui::TextEdit::singleline(&mut app.editor.prefab_path)
                                .desired_width(140.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        if ui.button("💾 Save Selected").clicked() {
                            let path = app.editor.prefab_path.clone();
                            app.save_selected_as_prefab(sel, &path);
                        }
                        if ui.button("➕ Spawn").clicked() {
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
                        ui.label(format!("Prefab: {path}"));
                        if ui.button("Break Prefab").clicked() {
                            crate::prefab::break_prefab_instance(&mut app.world, sel);
                        }
                    });
                }

                // Tag/name editor
                ui.separator();
                tag_name_editor(ui, sel, tag_map, &mut app.world);
            } else {
                ui.label("(no entity selected)");
            }
        });
}

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
        ui.label("(No images loaded)");
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

/// Scene save controls (path text field + Save/Load buttons + status messages).
///
/// Used in: docked right panel footer, overlay Inspector window.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn save_load_controls(
    ui: &mut egui::Ui,
    app: &mut App,
    entity_list: &[Entity],
    tag_map: &HashMap<Entity, String>,
) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Path:");
        ui.add(egui::TextEdit::singleline(&mut app.editor.editor_save_path).desired_width(160.0));
        if ui.button("📂 Load").clicked() {
            do_load_scene(app);
        }
        if ui.button("💾 Save").clicked() {
            do_save_scene_with_list(app, entity_list, tag_map);
        }
    });
    if let Some(msg) = &app.editor.editor_save_status {
        ui.small(msg.as_str());
    }
    if let Some(msg) = &app.editor.editor_load_status {
        ui.small(msg.as_str());
    }
}

// ── Save / Load helpers ──────────────────────────────────────────────────────

/// Execute a scene save using the current `entity_list` and `tag_map`.
///
/// Uses a topological sort so parents appear before children in the RON output.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn do_save_scene_with_list(
    app: &mut App,
    entity_list: &[Entity],
    tag_map: &HashMap<Entity, String>,
) {
    let mut scene_def = crate::prefab::SceneDef::default();
    let sorted = crate::hierarchy::topological_sort_entities(entity_list, &app.world);
    let mut dropped_parent_links: u32 = 0;
    for &e in &sorted {
        let tag = app.world.get::<crate::prefab::Tag>(e).map(|t| t.0.clone());
        let transform = app.world.get::<crate::components::Transform>(e).cloned();
        let sprite = app.world.get::<crate::components::Sprite>(e).cloned();
        let parent_entity = app.world.get::<crate::hierarchy::Parent>(e).map(|p| p.0);
        let parent = parent_entity.and_then(|p| tag_map.get(&p)).cloned();
        // Warn when a parent exists but has no tag — the hierarchy link cannot
        // be represented in the RON format and will be silently lost on reload.
        if parent_entity.is_some() && parent.is_none() {
            let child_name = tag.as_deref().unwrap_or("(untagged)");
            log::warn!(
                "scene save: parent of '{}' has no Tag — parent link dropped (will not restore on load)",
                child_name
            );
            dropped_parent_links += 1;
        }
        let components = app
            .world
            .resource::<crate::prefab::SerdeComponentRegistry>()
            .map(|r| r.serialize_entity(&app.world, e))
            .unwrap_or_default();
        if tag.is_some() || transform.is_some() || sprite.is_some() || !components.is_empty() {
            scene_def.entities.push(crate::prefab::EntityDef {
                tag,
                transform,
                sprite,
                parent,
                components,
            });
        }
    }
    let count = scene_def.entities.len();
    let path = app.editor.editor_save_path.clone();
    app.editor.editor_save_status = match scene_def.save(std::path::Path::new(&path)) {
        Ok(()) => {
            if dropped_parent_links > 0 {
                Some(format!(
                    "✓ {count} entities → {path} ({dropped_parent_links} parent link(s) dropped: untagged parent)"
                ))
            } else {
                Some(format!("✓ {count} entities → {path}"))
            }
        }
        Err(e) => Some(format!("✗ {e}")),
    };
    app.editor.editor_load_status = None;
}

/// Execute a scene save without an explicit entity list (queries the world).
///
/// Used by the toolbar "💾 Save" which runs before the entity list is built.
#[cfg(not(target_arch = "wasm32"))]
fn do_save_scene(app: &mut App) {
    let entity_list: Vec<Entity> = app.world.entities().to_vec();
    let tag_map: HashMap<Entity, String> = app
        .world
        .query::<Tag>()
        .map(|(e, t)| (e, t.0.clone()))
        .collect();
    do_save_scene_with_list(app, &entity_list, &tag_map);
}

/// Execute a scene load from `editor.editor_save_path`.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn do_load_scene(app: &mut App) {
    let path_str = app.editor.editor_save_path.clone();
    let path = std::path::Path::new(&path_str);
    match crate::prefab::SceneDef::load(path) {
        Ok(scene_def) => {
            // Despawn ALL current entities before loading so that UiNode-only
            // entities (menus/HUD without a Transform) don't accumulate on reload.
            let to_remove: Vec<Entity> = app.world.entities().to_vec();
            for e in to_remove {
                app.world.despawn(e);
            }
            app.editor.inspector_selected = None;
            app.editor.selected_entities.clear();
            let count = scene_def.entities.len();
            crate::prefab::spawn_scene_def(&mut app.world, &scene_def);
            app.editor.editor_load_status = Some(format!("✓ {count} entities ← {path_str}"));
            app.editor.editor_save_status = None;
        }
        Err(e) => {
            app.editor.editor_load_status = Some(format!("✗ {e}"));
        }
    }
}

// reflect_value_editor is defined in super (ui/mod.rs) without a cfg gate,
// so both native and wasm can call it.  docked.rs calls it via `super::reflect_value_editor`
// through `use super::*` at the top of this file.

/// Render the live-tunable fields of the selected entity's `ParticleEmitter` as a column of
/// drag editors. Edits mutate the component in place, so they take effect on the next spawn while
/// the simulation runs. No-op (renders nothing) if the entity has no `ParticleEmitter`.
#[cfg(not(target_arch = "wasm32"))]
fn particle_tuner_grid(ui: &mut egui::Ui, app: &mut App, sel: crate::ecs::Entity) {
    let Some(em) = app.world.get_mut::<crate::particle::ParticleEmitter>(sel) else {
        return;
    };
    ui.horizontal(|ui| {
        ui.checkbox(&mut em.emit, "emit");
    });
    ui.horizontal(|ui| {
        ui.label("spawn_rate");
        ui.add(
            egui::DragValue::new(&mut em.spawn_rate)
                .range(0.0..=2000.0)
                .speed(1.0),
        );
    });
    ui.horizontal(|ui| {
        ui.label("lifetime");
        ui.add(
            egui::DragValue::new(&mut em.lifetime)
                .range(0.0..=60.0)
                .speed(0.05)
                .suffix(" s"),
        );
    });
    ui.horizontal(|ui| {
        ui.label("velocity");
        ui.add(
            egui::DragValue::new(&mut em.velocity.x)
                .speed(1.0)
                .prefix("x "),
        );
        ui.add(
            egui::DragValue::new(&mut em.velocity.y)
                .speed(1.0)
                .prefix("y "),
        );
    });
    ui.horizontal(|ui| {
        ui.label("vel spread");
        ui.add(
            egui::DragValue::new(&mut em.velocity_spread.x)
                .range(0.0..=10000.0)
                .speed(1.0)
                .prefix("x "),
        );
        ui.add(
            egui::DragValue::new(&mut em.velocity_spread.y)
                .range(0.0..=10000.0)
                .speed(1.0)
                .prefix("y "),
        );
    });
    ui.horizontal(|ui| {
        ui.label("size");
        ui.add(
            egui::DragValue::new(&mut em.size.x)
                .range(0.0..=512.0)
                .speed(0.5)
                .prefix("w "),
        );
        ui.add(
            egui::DragValue::new(&mut em.size.y)
                .range(0.0..=512.0)
                .speed(0.5)
                .prefix("h "),
        );
    });
    ui.horizontal(|ui| {
        ui.label("color start");
        color_rgba_drags(ui, &mut em.color_start);
    });
    ui.horizontal(|ui| {
        ui.label("color end");
        color_rgba_drags(ui, &mut em.color_end);
    });
}

/// Four compact 0..=1 drag editors (r/g/b/a) for an engine [`Color`](crate::color::Color).
/// Exact (no sRGB round-trip), so it is safe for the linear-space particle colors.
#[cfg(not(target_arch = "wasm32"))]
fn color_rgba_drags(ui: &mut egui::Ui, c: &mut crate::color::Color) {
    for (label, ch) in [
        ("r", &mut c.r),
        ("g", &mut c.g),
        ("b", &mut c.b),
        ("a", &mut c.a),
    ] {
        ui.add(
            egui::DragValue::new(ch)
                .range(0.0..=1.0)
                .speed(0.01)
                .prefix(label),
        );
    }
}

/// Three compact 0..=1 drag editors (r/g/b) for a light [`Color`](crate::color::Color).
/// Lights ignore alpha, so only the RGB channels are exposed.
#[cfg(not(target_arch = "wasm32"))]
fn color_rgb_drags(ui: &mut egui::Ui, c: &mut crate::color::Color) {
    for (label, ch) in [("r", &mut c.r), ("g", &mut c.g), ("b", &mut c.b)] {
        ui.add(
            egui::DragValue::new(ch)
                .range(0.0..=1.0)
                .speed(0.01)
                .prefix(label),
        );
    }
}

/// Render the live-tunable fields of the selected entity's `PointLight` (color / radius /
/// intensity / light_height). Edits mutate the component so the lighting pass reflects them next
/// frame. No-op if the entity has no `PointLight`.
#[cfg(not(target_arch = "wasm32"))]
fn point_light_grid(ui: &mut egui::Ui, app: &mut App, sel: crate::ecs::Entity) {
    let Some(l) = app.world.get_mut::<crate::components::PointLight>(sel) else {
        return;
    };
    ui.horizontal(|ui| {
        ui.label("color");
        color_rgb_drags(ui, &mut l.color);
    });
    ui.horizontal(|ui| {
        ui.label("radius");
        ui.add(
            egui::DragValue::new(&mut l.radius)
                .range(0.0..=4000.0)
                .speed(2.0),
        );
    });
    ui.horizontal(|ui| {
        ui.label("intensity");
        ui.add(
            egui::DragValue::new(&mut l.intensity)
                .range(0.0..=10.0)
                .speed(0.05),
        );
    });
    ui.horizontal(|ui| {
        ui.label("light_height");
        ui.add(
            egui::DragValue::new(&mut l.light_height)
                .range(0.01..=2.0)
                .speed(0.01),
        );
    });
}

/// Edit the global `AmbientLight` resource (color + intensity), inserting a default one first if
/// the game never set it, so the control is always usable.
#[cfg(not(target_arch = "wasm32"))]
fn ambient_light_control(ui: &mut egui::Ui, app: &mut App) {
    app.ensure_ambient_light();
    if let Some(amb) = app.world.resource_mut::<crate::resources::AmbientLight>() {
        ui.horizontal(|ui| {
            ui.label("color");
            color_rgb_drags(ui, &mut amb.color);
        });
        ui.horizontal(|ui| {
            ui.label("intensity");
            ui.add(
                egui::DragValue::new(&mut amb.intensity)
                    .range(0.0..=1.0)
                    .speed(0.01),
            );
        });
    }
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

/// World coordinates of grid lines within `[start, end]` at `spacing` intervals
/// (aligned to multiples of `spacing`). Empty for degenerate inputs.
#[cfg(not(target_arch = "wasm32"))]
fn grid_lines_in_range(start: f32, end: f32, spacing: f32) -> Vec<f32> {
    if spacing <= 0.0 || end <= start {
        return Vec::new();
    }
    let first = (start / spacing).ceil() * spacing;
    let mut out = Vec::new();
    let mut x = first;
    let mut guard = 0;
    // Guard caps the count so a tiny spacing / huge range can't spin forever.
    while x <= end && guard < 100_000 {
        out.push(x);
        x += spacing;
        guard += 1;
    }
    out
}

/// Draw the world-aligned grid overlay + cursor coordinate readout on the docked viewport.
/// Pure egui painting on top of the game image — it does not touch the camera or game systems.
#[cfg(not(target_arch = "wasm32"))]
fn draw_editor_grid(ui: &egui::Ui, app: &App, rect: egui::Rect) {
    if rect.width() < 1.0 || rect.height() < 1.0 {
        return;
    }
    let cam_default = crate::camera::Camera::default();
    let cam = app
        .world
        .resource::<crate::camera::Camera>()
        .unwrap_or(&cam_default);
    let spacing = app.editor.snap_size.max(1.0);
    let painter = ui.painter_at(rect);
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(26));

    // Visible world range (top-left → bottom-right of the image rect).
    let tl = cam.screen_to_world(glam::Vec2::ZERO);
    let br = cam.screen_to_world(glam::Vec2::new(rect.width(), rect.height()));

    // Only draw when grid cells are at least a few pixels apart on screen.
    if spacing * cam.zoom >= 4.0 {
        for wx in grid_lines_in_range(tl.x, br.x, spacing) {
            let sx = rect.min.x + cam.world_to_screen(glam::Vec2::new(wx, tl.y)).x;
            painter.line_segment(
                [egui::pos2(sx, rect.top()), egui::pos2(sx, rect.bottom())],
                stroke,
            );
        }
        for wy in grid_lines_in_range(tl.y, br.y, spacing) {
            let sy = rect.min.y + cam.world_to_screen(glam::Vec2::new(tl.x, wy)).y;
            painter.line_segment(
                [egui::pos2(rect.left(), sy), egui::pos2(rect.right(), sy)],
                stroke,
            );
        }
    }

    // Cursor world-coordinate readout (and hovered cell if a Tilemap is selected).
    if let Some(p) = ui.ctx().pointer_hover_pos() {
        if rect.contains(p) {
            let world = cam.screen_to_world(glam::Vec2::new(p.x - rect.min.x, p.y - rect.min.y));
            let mut text = format!("x {:.0}  y {:.0}", world.x, world.y);
            if let Some(sel) = app.editor.inspector_selected {
                if let Some(tm) = app.world.get::<crate::tilemap::Tilemap>(sel) {
                    if let Some((row, col)) = tm.cell_at_world(world) {
                        text.push_str(&format!("  cell ({row}, {col})"));
                    }
                }
            }
            painter.text(
                rect.left_top() + egui::vec2(6.0, 6.0),
                egui::Align2::LEFT_TOP,
                text,
                egui::FontId::monospace(12.0),
                egui::Color32::from_white_alpha(190),
            );
        }
    }
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

    #[test]
    fn grid_lines_align_to_multiples_within_range() {
        use super::grid_lines_in_range;
        assert_eq!(
            grid_lines_in_range(0.0, 50.0, 16.0),
            vec![0.0, 16.0, 32.0, 48.0]
        );
        // Start not on a multiple → first line snaps up to the next multiple.
        assert_eq!(grid_lines_in_range(5.0, 50.0, 16.0), vec![16.0, 32.0, 48.0]);
        // Negative range works (camera can show negative world coords).
        assert_eq!(
            grid_lines_in_range(-10.0, 10.0, 5.0),
            vec![-10.0, -5.0, 0.0, 5.0, 10.0]
        );
        // Degenerate inputs → empty.
        assert!(grid_lines_in_range(0.0, 50.0, 0.0).is_empty());
        assert!(grid_lines_in_range(50.0, 0.0, 16.0).is_empty());
    }
}
