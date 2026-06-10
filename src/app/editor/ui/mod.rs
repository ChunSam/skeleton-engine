use super::*;

mod gizmo;
mod shortcuts;

// ── Private free helpers ──────────────────────────────────────────────────────

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
        let mut comp_fields: Vec<(&'static str, Vec<(&'static str, ReflectValue)>)> = Vec::new();
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
            if self
                .world
                .resource::<DebugUi>()
                .map(|d| d.is_enabled())
                .unwrap_or(false)
            {
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
                            // Scene graph: root → children indented tree
                            let mut clicked_entity: Option<Entity> = None;
                            let mut ctrl_clicked: bool = false;

                            egui::ScrollArea::vertical()
                                .id_salt("scene_graph")
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    // Stack-based DFS instead of recursion (can't call fn inside a closure)
                                    let mut stack: Vec<(Entity, usize)> =
                                        root_entities.iter().rev().map(|&e| (e, 0)).collect();
                                    while let Some((entity, depth)) = stack.pop() {
                                        let name =
                                            tag_map.get(&entity).cloned().unwrap_or_else(|| {
                                                format!(
                                                    "Entity {}:{}",
                                                    entity.index(),
                                                    entity.generation()
                                                )
                                            });
                                        // Multi-select: highlight based on selected_entities
                                        let is_selected =
                                            self.editor.selected_entities.contains(&entity);
                                        let has_children = children_map
                                            .get(&entity)
                                            .map(|c| !c.is_empty())
                                            .unwrap_or(false);
                                        let prefix = if has_children { "▶ " } else { "  " };
                                        let label_text =
                                            format!("{}{}{}", "  ".repeat(depth), prefix, name);

                                        let response =
                                            ui.selectable_label(is_selected, &label_text);
                                        if response.clicked() {
                                            clicked_entity = Some(entity);
                                            ctrl_clicked = ui.input(|i| i.modifiers.ctrl);
                                        }

                                        // Push children in reverse order (to maintain DFS order)
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
                                    &mut self.editor.selected_entities,
                                    &mut self.editor.inspector_selected,
                                );
                            }

                            // Edit the Tag name of the selected entity
                            ui.separator();
                            if let Some(sel) = self.editor.inspector_selected {
                                tag_name_editor(ui, sel, &tag_map, &mut self.world);
                            } else {
                                ui.label("(no entity selected)");
                            }
                        }

                        if self.editor.inspector_tab == 1 {
                            // ── Asset browser ─────────────────────────────────────
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
                                                            .unwrap_or_else(|| entry.path.clone());
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
                        } else {
                            // ── Editor action buttons ────────────────────────────────
                            ui.horizontal(|ui| {
                                if ui.button("＋ New Entity").clicked() {
                                    let e = self.world.spawn();
                                    self.world
                                        .add_component(e, crate::components::Transform::default());
                                    self.world
                                        .add_component(e, crate::prefab::Tag("New Entity".into()));
                                    self.editor.inspector_selected = Some(e);
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        self.editor.selected_entities = vec![e];
                                        self.editor
                                            .cmd_history
                                            .push(EditorCmd::CreateEntity { entity: e });
                                    }
                                }
                                if let Some(sel) = self.editor.inspector_selected {
                                    if ui
                                        .add_enabled(true, egui::Button::new("🗑 Delete"))
                                        .clicked()
                                    {
                                        #[cfg(not(target_arch = "wasm32"))]
                                        {
                                            let tag = self
                                                .world
                                                .get::<crate::prefab::Tag>(sel)
                                                .map(|t| t.0.clone());
                                            let transform = self
                                                .world
                                                .get::<crate::components::Transform>(sel)
                                                .cloned();
                                            let sprite = self
                                                .world
                                                .get::<crate::components::Sprite>(sel)
                                                .cloned();
                                            self.editor.cmd_history.push(EditorCmd::DeleteEntity {
                                                entity: None,
                                                tag,
                                                transform,
                                                sprite,
                                            });
                                        }
                                        self.world.despawn(sel);
                                        self.editor.inspector_selected = None;
                                        #[cfg(not(target_arch = "wasm32"))]
                                        self.editor.selected_entities.retain(|&x| x != sel);
                                    }
                                    if ui
                                        .add_enabled(true, egui::Button::new("⎘ Duplicate"))
                                        .clicked()
                                    {
                                        if let Some(new_entity) = self.world.clone_entity(sel) {
                                            if let Some(t) = self
                                                .world
                                                .get_mut::<crate::components::Transform>(new_entity)
                                            {
                                                t.position += glam::Vec2::new(16.0, 16.0);
                                            }
                                            self.editor.inspector_selected = Some(new_entity);
                                            #[cfg(not(target_arch = "wasm32"))]
                                            {
                                                self.editor.selected_entities = vec![new_entity];
                                            }
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
                                                let label =
                                                    tag_map.get(&e).cloned().unwrap_or_else(|| {
                                                        format!("E{}:{}", e.index(), e.generation())
                                                    });
                                                // Multi-select highlight (native) or single selection (WASM)
                                                #[cfg(not(target_arch = "wasm32"))]
                                                let is_sel =
                                                    self.editor.selected_entities.contains(&e);
                                                #[cfg(target_arch = "wasm32")]
                                                let is_sel =
                                                    self.editor.inspector_selected == Some(e);
                                                let resp = ui.selectable_label(is_sel, &label);
                                                if resp.clicked() {
                                                    #[cfg(not(target_arch = "wasm32"))]
                                                    apply_multiselect(
                                                        e,
                                                        ui.input(|i| i.modifiers.ctrl),
                                                        &mut self.editor.selected_entities,
                                                        &mut self.editor.inspector_selected,
                                                    );
                                                    #[cfg(target_arch = "wasm32")]
                                                    {
                                                        self.editor.inspector_selected = Some(e);
                                                    }
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
                                                            for (fname, fval) in fields.iter_mut() {
                                                                ui.label(*fname);
                                                                match fval {
                                                                    ReflectValue::F32(v) => {
                                                                        ui.add(
                                                                            egui::DragValue::new(v)
                                                                                .speed(0.5),
                                                                        );
                                                                    }
                                                                    ReflectValue::I32(v) => {
                                                                        ui.add(
                                                                            egui::DragValue::new(v)
                                                                                .speed(1.0),
                                                                        );
                                                                    }
                                                                    ReflectValue::Vec2(v) => {
                                                                        ui.horizontal(|ui| {
                                                                            ui.add(
                                                                            egui::DragValue::new(
                                                                                &mut v.x,
                                                                            )
                                                                            .speed(0.5)
                                                                            .prefix("x:"),
                                                                        );
                                                                            ui.add(
                                                                            egui::DragValue::new(
                                                                                &mut v.y,
                                                                            )
                                                                            .speed(0.5)
                                                                            .prefix("y:"),
                                                                        );
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
                                                                            ui.add(
                                                                            egui::DragValue::new(
                                                                                &mut c[0],
                                                                            )
                                                                            .speed(0.01)
                                                                            .prefix("r:"),
                                                                        );
                                                                            ui.add(
                                                                            egui::DragValue::new(
                                                                                &mut c[1],
                                                                            )
                                                                            .speed(0.01)
                                                                            .prefix("g:"),
                                                                        );
                                                                            ui.add(
                                                                            egui::DragValue::new(
                                                                                &mut c[2],
                                                                            )
                                                                            .speed(0.01)
                                                                            .prefix("b:"),
                                                                        );
                                                                            ui.add(
                                                                            egui::DragValue::new(
                                                                                &mut c[3],
                                                                            )
                                                                            .speed(0.01)
                                                                            .prefix("a:"),
                                                                        );
                                                                        });
                                                                    }
                                                                }
                                                                ui.end_row();
                                                            }
                                                        });
                                                });
                                            }
                                        });
                                });
                            });

                            // ── Add/remove components (native only, Phase 39b) ───────────
                            #[cfg(not(target_arch = "wasm32"))]
                            if let Some(sel) = self.editor.inspector_selected {
                                ui.separator();
                                ui.strong("Component List");

                                // Determine which component to remove (decided outside the closure).
                                // Only components registered via register_component_remover expose
                                // a working "✕" button (including custom components).
                                let mut to_remove: Option<&'static str> = None;
                                for &comp_name in &selected_comp_names {
                                    let removable = comp_name != "Transform"
                                        && self.editor.component_removers.contains_key(comp_name);
                                    ui.horizontal(|ui| {
                                        ui.label(comp_name);
                                        if removable && ui.small_button("✕").clicked() {
                                            to_remove = Some(comp_name);
                                        }
                                    });
                                }

                                // Perform the actual removal after the closure ends — dispatch to the registered remover.
                                if let Some(name) = to_remove {
                                    if let Some(remover) = self.editor.component_removers.get(name)
                                    {
                                        remover(&mut self.world, sel);
                                    }
                                }

                                ui.separator();
                                // Add Component dropdown
                                let factory_names: Vec<String> = {
                                    let mut names: Vec<String> =
                                        self.editor.component_factories.keys().cloned().collect();
                                    names.sort();
                                    names
                                };
                                if !factory_names.is_empty() {
                                    if self.editor.add_component_selected.is_empty() {
                                        self.editor.add_component_selected =
                                            factory_names[0].clone();
                                    }
                                    let cur = self.editor.add_component_selected.clone();
                                    egui::ComboBox::from_id_salt("add_comp_combo")
                                        .selected_text(&cur)
                                        .show_ui(ui, |ui| {
                                            for name in &factory_names {
                                                ui.selectable_value(
                                                    &mut self.editor.add_component_selected,
                                                    name.clone(),
                                                    name,
                                                );
                                            }
                                        });
                                    if ui.button("+ Add").clicked() {
                                        let chosen = self.editor.add_component_selected.clone();
                                        if let Some(factory) =
                                            self.editor.component_factories.get(&chosen)
                                        {
                                            factory(&mut self.world, sel);
                                        }
                                    }
                                }
                            }

                            // ── PrefabInstance / Break Prefab (native only) ───────────
                            #[cfg(not(target_arch = "wasm32"))]
                            if let Some(sel) = self.editor.inspector_selected {
                                let prefab_path = self
                                    .world
                                    .get::<crate::prefab::PrefabInstance>(sel)
                                    .map(|pi| pi.source_path.clone());
                                if let Some(path) = prefab_path {
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        ui.label(format!("Prefab: {path}"));
                                        if ui.button("Break Prefab").clicked() {
                                            crate::prefab::break_prefab_instance(
                                                &mut self.world,
                                                sel,
                                            );
                                        }
                                    });
                                }
                            }

                            // ── Edit selected entity name (Tag) (native only) ───────────
                            #[cfg(not(target_arch = "wasm32"))]
                            if let Some(sel) = self.editor.inspector_selected {
                                ui.separator();
                                tag_name_editor(ui, sel, &tag_map, &mut self.world);
                            }
                            // ── Scene save (Phase 28) ────────────────────────────────
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label("Path:");
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            &mut self.editor.editor_save_path,
                                        )
                                        .desired_width(180.0),
                                    );
                                    if ui.button("📂 Load Scene").clicked() {
                                        let path =
                                            std::path::Path::new(&self.editor.editor_save_path);
                                        match crate::prefab::SceneDef::load(path) {
                                            Ok(scene_def) => {
                                                // Remove existing editor entities (those with Transform or Tag)
                                                let to_remove: Vec<Entity> = self
                                                    .world
                                                    .query::<crate::components::Transform>()
                                                    .map(|(e, _)| e)
                                                    .collect();
                                                for e in to_remove {
                                                    self.world.despawn(e);
                                                }
                                                self.editor.inspector_selected = None;
                                                self.editor.selected_entities.clear();
                                                let count = scene_def.entities.len();
                                                crate::prefab::spawn_scene_def(
                                                    &mut self.world,
                                                    &scene_def,
                                                );
                                                self.editor.editor_load_status = Some(format!(
                                                    "✓ {count} entities ← {}",
                                                    self.editor.editor_save_path
                                                ));
                                                self.editor.editor_save_status = None;
                                            }
                                            Err(e) => {
                                                self.editor.editor_load_status =
                                                    Some(format!("✗ {e}"));
                                            }
                                        }
                                    }
                                    if ui.button("💾 Save Scene").clicked() {
                                        let mut scene_def = crate::prefab::SceneDef::default();
                                        // Topological sort so parents appear before children
                                        let sorted = crate::prefab::topological_sort_entities(
                                            &entity_list,
                                            &self.world,
                                        );
                                        for &e in &sorted {
                                            let tag = self
                                                .world
                                                .get::<crate::prefab::Tag>(e)
                                                .map(|t| t.0.clone());
                                            let transform = self
                                                .world
                                                .get::<crate::components::Transform>(e)
                                                .cloned();
                                            let sprite = self
                                                .world
                                                .get::<crate::components::Sprite>(e)
                                                .cloned();
                                            // Parent component → parent's tag string
                                            let parent = self
                                                .world
                                                .get::<crate::hierarchy::Parent>(e)
                                                .and_then(|p| tag_map.get(&p.0))
                                                .cloned();
                                            if tag.is_some()
                                                || transform.is_some()
                                                || sprite.is_some()
                                            {
                                                scene_def.entities.push(crate::prefab::EntityDef {
                                                    tag,
                                                    transform,
                                                    sprite,
                                                    parent,
                                                });
                                            }
                                        }
                                        let count = scene_def.entities.len();
                                        let path = self.editor.editor_save_path.clone();
                                        self.editor.editor_save_status = match scene_def
                                            .save(std::path::Path::new(&path))
                                        {
                                            Ok(()) => Some(format!("✓ {count} entities → {path}")),
                                            Err(e) => Some(format!("✗ {e}")),
                                        };
                                    }
                                });
                                if let Some(msg) = &self.editor.editor_save_status {
                                    ui.small(msg.as_str());
                                }
                                if let Some(msg) = &self.editor.editor_load_status {
                                    ui.small(msg.as_str());
                                }
                            }
                        } // end Entities tab
                    });
            }
        }

        // Inspector: apply staged values to the World (before the egui frame ends)
        if let Some(sel) = self.editor.inspector_selected {
            let type_ids = self.world.reflected_components(sel);
            for (i, tid) in type_ids.iter().enumerate() {
                if i < comp_fields.len() {
                    if let Some(refl) = self.world.get_reflect_mut(sel, *tid) {
                        for (fname, fval) in &comp_fields[i].1 {
                            refl.set_field(fname, fval.clone());
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

        self.update_editor_gizmo(egui_ctx);
    }
}
