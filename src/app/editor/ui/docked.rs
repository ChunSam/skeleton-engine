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

    // ── 2. Bottom panel: Assets | Data Tables ───────────────────────────────
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
            });
            ui.separator();
            if app.editor.bottom_tab == 0 {
                assets_tab_body(ui, app);
            } else {
                super::data_table_panel_body(ui, app);
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
                ui.image((tex, avail));
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
                .push(super::super::EditorCmd::CreateEntity { entity: e });
        }
        if let Some(sel) = app.editor.inspector_selected {
            if ui
                .add_enabled(true, egui::Button::new("🗑 Delete"))
                .clicked()
            {
                let tag = app
                    .world
                    .get::<crate::prefab::Tag>(sel)
                    .map(|t| t.0.clone());
                let transform = app.world.get::<crate::components::Transform>(sel).cloned();
                let sprite = app.world.get::<crate::components::Sprite>(sel).cloned();
                app.editor
                    .cmd_history
                    .push(super::super::EditorCmd::DeleteEntity {
                        entity: None,
                        tag,
                        transform,
                        sprite,
                    });
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
                }
            }
        }
    });
    ui.separator();

    // Flat entity list with multi-select
    egui::ScrollArea::vertical()
        .id_salt("docked_ent_list")
        .show(ui, |ui| {
            for &e in entity_list {
                let label = entity_label(e, tag_map);
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
                ui.separator();
                ui.strong("Component List");

                let mut to_remove: Option<&'static str> = None;
                for &comp_name in selected_comp_names {
                    let removable = comp_name != "Transform"
                        && app.editor.component_removers.contains_key(comp_name);
                    ui.horizontal(|ui| {
                        ui.label(comp_name);
                        if removable && ui.small_button("✕").clicked() {
                            to_remove = Some(comp_name);
                        }
                    });
                }
                if let Some(name) = to_remove {
                    if let Some(remover) = app.editor.component_removers.get(name) {
                        remover(&mut app.world, sel);
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
    for &e in &sorted {
        let tag = app.world.get::<crate::prefab::Tag>(e).map(|t| t.0.clone());
        let transform = app.world.get::<crate::components::Transform>(e).cloned();
        let sprite = app.world.get::<crate::components::Sprite>(e).cloned();
        let parent = app
            .world
            .get::<crate::hierarchy::Parent>(e)
            .and_then(|p| tag_map.get(&p.0))
            .cloned();
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
        Ok(()) => Some(format!("✓ {count} entities → {path}")),
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
            // Remove existing editor entities (those with Transform or Tag)
            let to_remove: Vec<Entity> = app
                .world
                .query::<crate::components::Transform>()
                .map(|(e, _)| e)
                .collect();
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
