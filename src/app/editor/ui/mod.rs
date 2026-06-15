use super::*;

#[cfg(not(target_arch = "wasm32"))]
mod data_table_panel;
mod docked;
mod gizmo;
mod shortcuts;

#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::EditorMode;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) use data_table_panel::data_table_panel_body;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) use docked::{
    assets_tab_body, entities_tab_body, inspector_tab_body, save_load_controls, scene_tab_body,
    update_docked_ui,
};

// ── Private free helpers ──────────────────────────────────────────────────────

/// Render a single [`ReflectValue`] field editor into the current `ui`.
///
/// Defined here (not behind a `#[cfg]`) so both native docked/overlay paths and the
/// wasm overlay path share the same implementation.
pub(super) fn reflect_value_editor(ui: &mut egui::Ui, fval: &mut ReflectValue) {
    match fval {
        ReflectValue::F32(v) => {
            ui.add(egui::DragValue::new(v).speed(0.5));
        }
        ReflectValue::I32(v) => {
            ui.add(egui::DragValue::new(v).speed(1.0));
        }
        ReflectValue::Vec2(v) => {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut v.x).speed(0.5).prefix("x:"));
                ui.add(egui::DragValue::new(&mut v.y).speed(0.5).prefix("y:"));
            });
        }
        ReflectValue::Bool(v) => {
            ui.checkbox(v, "");
        }
        ReflectValue::String(s) => {
            ui.text_edit_singleline(s);
        }
        ReflectValue::Color(c) => {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut c[0]).speed(0.01).prefix("r:"));
                ui.add(egui::DragValue::new(&mut c[1]).speed(0.01).prefix("g:"));
                ui.add(egui::DragValue::new(&mut c[2]).speed(0.01).prefix("b:"));
                ui.add(egui::DragValue::new(&mut c[3]).speed(0.01).prefix("a:"));
            });
        }
    }
}

/// Canonical display label for an entity: its `Tag` name when available,
/// otherwise `"Entity {index}:{generation}"`.
fn entity_label(e: Entity, tag_map: &HashMap<Entity, String>) -> String {
    tag_map
        .get(&e)
        .cloned()
        .unwrap_or_else(|| format!("Entity {}:{}", e.index(), e.generation()))
}

/// Render the "Name:" tag-editing row for `sel` inside an existing `ui` horizontal.
///
/// Shows a text-edit when the entity already has a `Tag`; otherwise shows the
/// raw `Entity` id and an "Add Name" button.  Writes back through `world`.
/// Only used in native-only UI paths.
#[cfg(not(target_arch = "wasm32"))]
fn tag_name_editor(
    ui: &mut egui::Ui,
    sel: Entity,
    tag_map: &HashMap<Entity, String>,
    world: &mut World,
) {
    let current_name = tag_map.get(&sel).cloned().unwrap_or_default();
    let has_tag = world.get::<Tag>(sel).is_some();
    ui.horizontal(|ui| {
        ui.label("Name:");
        if has_tag {
            let mut name_buf = current_name;
            if ui.text_edit_singleline(&mut name_buf).changed() {
                world.add_component(sel, Tag(name_buf));
            }
        } else {
            ui.label(format!("Entity {}:{}", sel.index(), sel.generation()));
            if ui.button("Add Name").clicked() {
                world.add_component(
                    sel,
                    Tag(format!("Entity {}:{}", sel.index(), sel.generation())),
                );
            }
        }
    });
}

/// Apply Ctrl+click multi-select semantics for `entity`.
///
/// If `ctrl` is true the entity is toggled in `selected`; otherwise it becomes
/// the sole selection.  `inspector` is updated to track the primary selection.
#[cfg(not(target_arch = "wasm32"))]
fn apply_multiselect(
    entity: Entity,
    ctrl: bool,
    selected: &mut Vec<Entity>,
    inspector: &mut Option<Entity>,
) {
    if ctrl {
        if let Some(pos) = selected.iter().position(|&x| x == entity) {
            selected.remove(pos);
            *inspector = selected.last().copied();
        } else {
            selected.push(entity);
            *inspector = Some(entity);
        }
    } else {
        *inspector = Some(entity);
        *selected = vec![entity];
    }
}

// ─────────────────────────────────────────────────────────────────────────────

impl App {
    pub(in crate::app) fn update_editor_ui(&mut self, egui_ctx: &Option<egui::Context>, dt: f32) {
        // Inspector: validate selected entity + stage fields
        if let Some(sel) = self.editor.inspector_selected {
            if !self.world.is_alive(sel) {
                self.editor.inspector_selected = None;
                #[cfg(not(target_arch = "wasm32"))]
                self.editor.selected_entities.clear();
            }
        }
        // Remove dead entities from the multi-select list (native only)
        #[cfg(not(target_arch = "wasm32"))]
        self.editor
            .selected_entities
            .retain(|&e| self.world.is_alive(e));
        // comp_fields must be built unconditionally: it is consumed after the egui block
        // for inspector write-back (~line 742).  entity_list / tag_map are only needed
        // inside the is_enabled() gate and are moved there to avoid per-frame allocations
        // when the overlay is closed.
        //
        // We also record which entity comp_fields was built for so that write-back can
        // guard against stale data if the selection changed mid-frame (Fix 2).
        let mut comp_fields: Vec<(&'static str, Vec<(&'static str, ReflectValue)>)> = Vec::new();
        let comp_fields_entity: Option<Entity> = self.editor.inspector_selected;
        if let Some(sel) = self.editor.inspector_selected {
            for tid in self.world.reflected_components(sel) {
                if let Some(refl) = self.world.get_reflect(sel, tid) {
                    comp_fields.push((refl.type_name(), refl.fields()));
                }
            }
        }

        // Built-in EngineStats panel + Inspector
        if let Some(ctx) = egui_ctx {
            // ── Undo (Ctrl+Z) / Redo (Ctrl+Shift+Z) / Copy (Ctrl+C) / Paste (Ctrl+V) ─
            #[cfg(not(target_arch = "wasm32"))]
            self.handle_editor_shortcuts(ctx);

            // Docked mode: draw the full docked layout.
            #[cfg(not(target_arch = "wasm32"))]
            if self.editor.mode == EditorMode::Docked {
                // Build the data that both overlay and docked modes need.
                let entity_list: Vec<Entity> = self.world.entities().to_vec();
                let tag_map: HashMap<Entity, String> = self
                    .world
                    .query::<Tag>()
                    .map(|(e, t)| (e, t.0.clone()))
                    .collect();
                let selected_comp_names: Vec<&'static str> =
                    comp_fields.iter().map(|(name, _)| *name).collect();
                let scene_graph_data: Vec<(Entity, Option<Entity>)> = entity_list
                    .iter()
                    .map(|&e| {
                        let parent = self.world.get::<crate::hierarchy::Parent>(e).map(|p| p.0);
                        (e, parent)
                    })
                    .collect();
                let children_map: HashMap<Entity, Vec<Entity>> = {
                    let mut map: HashMap<Entity, Vec<Entity>> = HashMap::new();
                    for &(child, parent_opt) in &scene_graph_data {
                        if let Some(parent) = parent_opt {
                            map.entry(parent).or_default().push(child);
                        }
                    }
                    map
                };
                let root_entities: Vec<Entity> = scene_graph_data
                    .iter()
                    .filter_map(|&(e, p)| if p.is_none() { Some(e) } else { None })
                    .collect();

                update_docked_ui(
                    ctx,
                    self,
                    &mut comp_fields,
                    &entity_list,
                    &tag_map,
                    &selected_comp_names,
                    &scene_graph_data,
                    &children_map,
                    &root_entities,
                );
            }

            // Overlay is gated on mode == Overlay (native) or DebugUi.is_enabled() (wasm).
            #[cfg(not(target_arch = "wasm32"))]
            let overlay_visible = self.editor.mode == EditorMode::Overlay;
            #[cfg(target_arch = "wasm32")]
            let overlay_visible = self
                .world
                .resource::<DebugUi>()
                .map(|d| d.is_enabled())
                .unwrap_or(false);

            if overlay_visible {
                // ── Build entity/tag data only when the overlay is visible ───────────
                // Avoids a Vec + HashMap allocation every frame when the debug overlay
                // is closed (is_enabled() == false).
                let entity_list: Vec<Entity> = self.world.entities().to_vec();
                let tag_map: HashMap<Entity, String> = self
                    .world
                    .query::<Tag>()
                    .map(|(e, t)| (e, t.0.clone()))
                    .collect();

                // List of Reflect-registered component names on the selected entity (native only).
                // Extracting names from comp_fields avoids a borrow conflict.
                #[cfg(not(target_arch = "wasm32"))]
                let selected_comp_names: Vec<&'static str> =
                    comp_fields.iter().map(|(name, _)| *name).collect();

                // ── Pre-collect scene graph data (native only) ─────────────────────
                // Borrow-checker workaround: copy the entire hierarchy before entering
                // the egui closure.
                #[cfg(not(target_arch = "wasm32"))]
                let scene_graph_data: Vec<(Entity, Option<Entity>)> = {
                    // (entity, parent_entity_or_none)
                    entity_list
                        .iter()
                        .map(|&e| {
                            let parent = self.world.get::<crate::hierarchy::Parent>(e).map(|p| p.0);
                            (e, parent)
                        })
                        .collect()
                };
                // children_map: parent → list of children
                #[cfg(not(target_arch = "wasm32"))]
                let children_map: HashMap<Entity, Vec<Entity>> = {
                    let mut map: HashMap<Entity, Vec<Entity>> = HashMap::new();
                    for &(child, parent_opt) in &scene_graph_data {
                        if let Some(parent) = parent_opt {
                            map.entry(parent).or_default().push(child);
                        }
                    }
                    map
                };
                // Root entities = those without a Parent component
                #[cfg(not(target_arch = "wasm32"))]
                let root_entities: Vec<Entity> = scene_graph_data
                    .iter()
                    .filter_map(|&(e, p)| if p.is_none() { Some(e) } else { None })
                    .collect();
                let entity_count = self.world.entity_count();
                let asset_count = self
                    .world
                    .resource::<AssetServer>()
                    .map(|a| a.image_count())
                    .unwrap_or(0);
                egui::Window::new("Engine Stats")
                    .default_pos([10.0, 10.0])
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.label(format!("FPS   {:>6.1}", 1.0_f32 / dt.max(0.001)));
                        ui.label(format!("ms    {:>6.2}", dt * 1000.0));
                        ui.label(format!("Ent   {entity_count}"));
                        ui.label(format!("Asset {asset_count}"));
                        ui.separator();
                        if let Some(prof) = self.world.resource::<crate::resources::ProfilerData>()
                        {
                            ui.collapsing("Systems", |ui| {
                                egui::Grid::new("sys_prof")
                                    .num_columns(2)
                                    .striped(true)
                                    .show(ui, |ui| {
                                        for sys in &prof.systems {
                                            let label = if sys.name.is_empty() {
                                                "anonymous"
                                            } else {
                                                &sys.name
                                            };
                                            ui.label(label);
                                            ui.label(format!("{:.0} µs", sys.avg_us));
                                            ui.end_row();
                                        }
                                    });
                            });
                            let r = prof.render;
                            ui.collapsing("Render", |ui| {
                                ui.label(format!("draw calls  {}", r.draw_calls));
                                ui.label(format!("rendered    {}", r.sprites_rendered));
                                ui.label(format!("culled      {}", r.sprites_culled));
                            });
                        }
                    });

                // Inspector panel: entity list + component field editor + asset browser
                egui::Window::new("Inspector")
                    .default_pos([10.0, 130.0])
                    .default_size([440.0, 380.0])
                    .show(ctx, |ui| {
                        // ── Tab selection ────────────────────────────────────────
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(self.editor.inspector_tab == 0, "Entities")
                                .clicked()
                            {
                                self.editor.inspector_tab = 0;
                            }
                            if ui
                                .selectable_label(self.editor.inspector_tab == 1, "Assets")
                                .clicked()
                            {
                                self.editor.inspector_tab = 1;
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            if ui
                                .selectable_label(self.editor.inspector_tab == 2, "Scene")
                                .clicked()
                            {
                                self.editor.inspector_tab = 2;
                            }
                        });
                        ui.separator();

                        // ── Grid Snap controls (Entities tab, native only) ────────
                        #[cfg(not(target_arch = "wasm32"))]
                        if self.editor.inspector_tab == 0 {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.editor.snap_enabled, "Snap");
                                if self.editor.snap_enabled {
                                    ui.add(
                                        egui::DragValue::new(&mut self.editor.snap_size)
                                            .range(1.0..=128.0)
                                            .speed(1.0)
                                            .suffix(" px"),
                                    );
                                }
                            });
                        }

                        // ── Scene graph tab (native only) ────────────────────────
                        #[cfg(not(target_arch = "wasm32"))]
                        if self.editor.inspector_tab == 2 {
                            scene_tab_body(
                                ui,
                                self,
                                &tag_map,
                                &scene_graph_data,
                                &children_map,
                                &root_entities,
                            );
                        }

                        if self.editor.inspector_tab == 1 {
                            // ── Asset browser ─────────────────────────────────────
                            #[cfg(not(target_arch = "wasm32"))]
                            assets_tab_body(ui, self);
                            #[cfg(target_arch = "wasm32")]
                            {
                                let entries = self
                                    .world
                                    .resource::<AssetServer>()
                                    .map(|a| a.image_list())
                                    .unwrap_or_default();
                                if entries.is_empty() {
                                    ui.label("(No images loaded)");
                                } else {
                                    egui::ScrollArea::vertical()
                                        .id_salt("asset_browser")
                                        .max_height(300.0)
                                        .show(ui, |ui| {
                                            egui::Grid::new("asset_grid")
                                                .num_columns(2)
                                                .spacing([8.0, 4.0])
                                                .show(ui, |ui| {
                                                    for entry in &entries {
                                                        let filename =
                                                            std::path::Path::new(&entry.path)
                                                                .file_name()
                                                                .map(|f| {
                                                                    f.to_string_lossy().into_owned()
                                                                })
                                                                .unwrap_or_else(|| {
                                                                    entry.path.clone()
                                                                });
                                                        ui.label("[ ]");
                                                        ui.vertical(|ui| {
                                                            ui.label(&filename);
                                                            ui.small(format!(
                                                                "{}×{}",
                                                                entry.width, entry.height
                                                            ));
                                                        });
                                                        ui.end_row();
                                                    }
                                                });
                                        });
                                }
                            }
                        } else if self.editor.inspector_tab == 0 {
                            // ── Entities tab ─────────────────────────────────────

                            // Action row + entity list
                            // Re-use the entities_tab_body for the flat entity list.
                            #[cfg(not(target_arch = "wasm32"))]
                            entities_tab_body(ui, self, &entity_list, &tag_map);

                            // Component editor (right side) and add/remove — use shared inspector body.
                            #[cfg(not(target_arch = "wasm32"))]
                            inspector_tab_body(
                                ui,
                                self,
                                &mut comp_fields,
                                &selected_comp_names,
                                &tag_map,
                                &entity_list,
                            );

                            // WASM: minimal entity list + component editor (no multi-select)
                            #[cfg(target_arch = "wasm32")]
                            {
                                ui.horizontal(|ui| {
                                    if ui.button("＋ New Entity").clicked() {
                                        let e = self.world.spawn();
                                        self.world.add_component(
                                            e,
                                            crate::components::Transform::default(),
                                        );
                                        self.world.add_component(
                                            e,
                                            crate::prefab::Tag("New Entity".into()),
                                        );
                                        self.editor.inspector_selected = Some(e);
                                    }
                                    if let Some(sel) = self.editor.inspector_selected {
                                        if ui
                                            .add_enabled(true, egui::Button::new("🗑 Delete"))
                                            .clicked()
                                        {
                                            self.world.despawn(sel);
                                            self.editor.inspector_selected = None;
                                        }
                                        if ui
                                            .add_enabled(true, egui::Button::new("⎘ Duplicate"))
                                            .clicked()
                                        {
                                            if let Some(new_entity) = self.world.clone_entity(sel) {
                                                if let Some(t) = self
                                                    .world
                                                    .get_mut::<crate::components::Transform>(
                                                    new_entity,
                                                ) {
                                                    t.position += glam::Vec2::new(16.0, 16.0);
                                                }
                                                self.editor.inspector_selected = Some(new_entity);
                                            }
                                        }
                                    }
                                });
                                ui.separator();
                                ui.horizontal_top(|ui| {
                                    // Left: entity list
                                    ui.vertical(|ui| {
                                        ui.set_min_width(130.0);
                                        ui.strong("Entities");
                                        egui::ScrollArea::vertical()
                                            .id_salt("inspector_ent")
                                            .max_height(250.0)
                                            .show(ui, |ui| {
                                                for &e in &entity_list {
                                                    let label = entity_label(e, &tag_map);
                                                    let is_sel =
                                                        self.editor.inspector_selected == Some(e);
                                                    let resp = ui.selectable_label(is_sel, &label);
                                                    if resp.clicked() {
                                                        self.editor.inspector_selected = Some(e);
                                                    }
                                                }
                                            });
                                    });
                                    ui.separator();
                                    // Right: component field editor
                                    ui.vertical(|ui| {
                                        ui.strong("Components");
                                        egui::ScrollArea::vertical()
                                            .id_salt("inspector_comp")
                                            .max_height(250.0)
                                            .show(ui, |ui| {
                                                for (comp_name, fields) in comp_fields.iter_mut() {
                                                    ui.collapsing(*comp_name, |ui| {
                                                        egui::Grid::new(*comp_name)
                                                            .num_columns(2)
                                                            .spacing([4.0, 2.0])
                                                            .show(ui, |ui| {
                                                                for (fname, fval) in
                                                                    fields.iter_mut()
                                                                {
                                                                    ui.label(*fname);
                                                                    reflect_value_editor(ui, fval);
                                                                    ui.end_row();
                                                                }
                                                            });
                                                    });
                                                }
                                            });
                                    });
                                });
                            }

                            // ── Scene save (native only) ─────────────────────────
                            #[cfg(not(target_arch = "wasm32"))]
                            save_load_controls(ui, self, &entity_list, &tag_map);
                        } // end Entities tab
                    });
            }
        }

        // Inspector: apply staged values to the World (before the egui frame ends).
        //
        // Match by component TYPE NAME rather than positional index so that an
        // archetype change (add/remove component) or a mid-frame selection change
        // cannot silently apply fields to the wrong component.  Also guard: skip
        // write-back entirely if the current selection differs from the entity that
        // comp_fields was built for (e.g. Ctrl+Z changed the selection this frame).
        if let Some(sel) = self.editor.inspector_selected {
            // Only write back if comp_fields was built for this same entity.
            if comp_fields_entity == Some(sel) {
                // Build a name → TypeId map for the types currently on this entity.
                let registered = self.world.reflect_registered_types();
                let current_tids: Vec<std::any::TypeId> = self.world.reflected_components(sel);
                // Filter registered types to those present on the current entity.
                let name_to_tid: std::collections::HashMap<&'static str, std::any::TypeId> =
                    registered
                        .into_iter()
                        .filter(|(tid, _)| current_tids.contains(tid))
                        .map(|(tid, name)| (name, tid))
                        .collect();

                for (comp_name, fields) in &comp_fields {
                    if let Some(&tid) = name_to_tid.get(comp_name) {
                        if let Some(refl) = self.world.get_reflect_mut(sel, tid) {
                            for (fname, fval) in fields {
                                refl.set_field(fname, fval.clone());
                            }
                        }
                    }
                }
            }
        }

        // ── Sync SelectedEntity resource ─────────────────────────────────────────
        if let Some(res) = self
            .world
            .resource_mut::<crate::resources::SelectedEntity>()
        {
            res.0 = self.editor.inspector_selected;
        }

        // Gizmo runs in Overlay AND Docked modes (native), or when DebugUi is
        // enabled (wasm). In Docked mode the gizmo operates on viewport-local
        // coordinates: InputState::cursor() is already translated by
        // viewport_to_game and ViewportSize is delegated to the central rect, so
        // the Camera::screen_to_world math needs no change.
        #[cfg(not(target_arch = "wasm32"))]
        if matches!(self.editor.mode, EditorMode::Overlay | EditorMode::Docked) {
            self.update_editor_gizmo(egui_ctx);
            if self.editor.show_bounds {
                self.draw_debug_bounds();
            }
            if self.editor.show_pathgrid {
                self.draw_pathfinding_overlay();
            }
        }
        #[cfg(target_arch = "wasm32")]
        self.update_editor_gizmo(egui_ctx);
    }
}
