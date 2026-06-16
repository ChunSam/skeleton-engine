use super::*;

mod state;
mod ui;

#[cfg(not(target_arch = "wasm32"))]
pub(super) mod docked_rt;

pub(super) use state::EditorState;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use state::ResizeHandle;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use state::{apply_f1, apply_f2, EditorMode, InspectorPanel, PaintTool};

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub(super) enum EditorCmd {
    MoveEntity {
        entity: Entity,
        old_pos: glam::Vec2,
        new_pos: glam::Vec2,
    },
    CreateEntity {
        entity: Entity,
        /// Full entity def for redo — `Some` for Duplicate/Paste (restores all components),
        /// `None` for New Entity (redo spawns a default entity).
        def: Option<crate::prefab::EntityDef>,
    },
    DeleteEntity {
        /// Entity id recreated by undo. Filled in at undo time so redo can despawn exactly
        /// this entity (None on first creation — the original has already been despawned).
        entity: Option<Entity>,
        /// Full captured state at delete time; undo restores all components via spawn_entity_def.
        def: crate::prefab::EntityDef,
    },
    /// Move a screen-space `UiNode` widget (offset changed, size/anchor unchanged).
    MoveUiNode {
        entity: Entity,
        old_offset: glam::Vec2,
        new_offset: glam::Vec2,
    },
    /// Resize a screen-space `UiNode` widget (both offset and size may change).
    ResizeUiNode {
        entity: Entity,
        old_offset: glam::Vec2,
        old_size: glam::Vec2,
        new_offset: glam::Vec2,
        new_size: glam::Vec2,
    },
    /// Resize a world-space sprite by changing `Transform.scale` (center fixed).
    ResizeEntity {
        entity: Entity,
        old_scale: glam::Vec2,
        new_scale: glam::Vec2,
    },
    /// Rotate a world-space sprite by changing `Transform.rotation` (radians).
    RotateEntity {
        entity: Entity,
        old_rotation: f32,
        new_rotation: f32,
    },
    /// One tile-paint stroke on a `Tilemap` entity. `changes` lists every cell the stroke
    /// actually modified as `(row, col, old, new)`; undo restores `old` (reverse order),
    /// redo re-applies `new`. A whole drag is a single undo step.
    PaintTiles {
        entity: Entity,
        changes: Vec<(usize, usize, u32, u32)>,
    },
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
pub(super) struct EditorHistory {
    undo: Vec<EditorCmd>,
    redo: Vec<EditorCmd>,
}

#[cfg(not(target_arch = "wasm32"))]
impl EditorHistory {
    pub(super) fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub(super) fn push(&mut self, cmd: EditorCmd) {
        self.undo.push(cmd);
        self.redo.clear();
    }

    pub(super) fn undo(&mut self, world: &mut World, selected: &mut Option<Entity>) {
        let Some(mut cmd) = self.undo.pop() else {
            return;
        };
        // entity id recreated by DeleteEntity undo — recorded into cmd after the match
        let mut respawned: Option<Entity> = None;
        match &cmd {
            EditorCmd::MoveEntity {
                entity, old_pos, ..
            } => {
                if let Some(t) = world.get_mut::<crate::components::Transform>(*entity) {
                    t.position = *old_pos;
                }
                *selected = Some(*entity);
            }
            EditorCmd::CreateEntity { entity, .. } => {
                world.despawn(*entity);
                *selected = None;
            }
            EditorCmd::DeleteEntity { def, .. } => {
                let e = crate::prefab::spawn_entity_def(world, def);
                *selected = Some(e);
                respawned = Some(e);
            }
            EditorCmd::MoveUiNode {
                entity, old_offset, ..
            } => {
                if let Some(n) = world.get_mut::<crate::ui::UiNode>(*entity) {
                    n.offset = *old_offset;
                }
                *selected = Some(*entity);
            }
            EditorCmd::ResizeUiNode {
                entity,
                old_offset,
                old_size,
                ..
            } => {
                if let Some(n) = world.get_mut::<crate::ui::UiNode>(*entity) {
                    n.offset = *old_offset;
                    n.size = *old_size;
                }
                *selected = Some(*entity);
            }
            EditorCmd::ResizeEntity {
                entity, old_scale, ..
            } => {
                if let Some(t) = world.get_mut::<crate::components::Transform>(*entity) {
                    t.scale = *old_scale;
                }
                *selected = Some(*entity);
            }
            EditorCmd::RotateEntity {
                entity,
                old_rotation,
                ..
            } => {
                if let Some(t) = world.get_mut::<crate::components::Transform>(*entity) {
                    t.rotation = *old_rotation;
                }
                *selected = Some(*entity);
            }
            EditorCmd::PaintTiles { entity, changes } => {
                if let Some(tm) = world.get_mut::<crate::tilemap::Tilemap>(*entity) {
                    for (row, col, old, _new) in changes.iter().rev() {
                        tm.set_tile(*row, *col, *old);
                    }
                }
                // Re-sync colliders so undo restores the pre-stroke physics state too.
                crate::physics::sync_tilemap_entity_colliders(world, *entity);
                *selected = Some(*entity);
            }
        }
        // record the id so redo despawns the exact recreated entity, not the current selection
        if let (Some(e), EditorCmd::DeleteEntity { entity, .. }) = (respawned, &mut cmd) {
            *entity = Some(e);
        }
        self.redo.push(cmd);
    }

    pub(super) fn redo(&mut self, world: &mut World, selected: &mut Option<Entity>) {
        let Some(cmd) = self.redo.pop() else { return };
        match &cmd {
            EditorCmd::MoveEntity {
                entity, new_pos, ..
            } => {
                if let Some(t) = world.get_mut::<crate::components::Transform>(*entity) {
                    t.position = *new_pos;
                }
                *selected = Some(*entity);
            }
            EditorCmd::CreateEntity { def, .. } => {
                // was despawned by undo — re-spawn from def (Duplicate/Paste) or default (New Entity)
                let e = if let Some(d) = def {
                    crate::prefab::spawn_entity_def(world, d)
                } else {
                    let e = world.spawn();
                    world.add_component(e, crate::components::Transform::default());
                    world.add_component(e, Tag("New Entity".into()));
                    e
                };
                *selected = Some(e);
                // push cmd updated with the new entity id so it can be undone again
                self.undo.push(EditorCmd::CreateEntity {
                    entity: e,
                    def: def.clone(),
                });
                return;
            }
            EditorCmd::DeleteEntity { entity, .. } => {
                // despawn exactly the entity recreated by undo (independent of current selection)
                if let Some(e) = *entity {
                    world.despawn(e);
                    if *selected == Some(e) {
                        *selected = None;
                    }
                }
            }
            EditorCmd::MoveUiNode {
                entity, new_offset, ..
            } => {
                if let Some(n) = world.get_mut::<crate::ui::UiNode>(*entity) {
                    n.offset = *new_offset;
                }
                *selected = Some(*entity);
            }
            EditorCmd::ResizeUiNode {
                entity,
                new_offset,
                new_size,
                ..
            } => {
                if let Some(n) = world.get_mut::<crate::ui::UiNode>(*entity) {
                    n.offset = *new_offset;
                    n.size = *new_size;
                }
                *selected = Some(*entity);
            }
            EditorCmd::ResizeEntity {
                entity, new_scale, ..
            } => {
                if let Some(t) = world.get_mut::<crate::components::Transform>(*entity) {
                    t.scale = *new_scale;
                }
                *selected = Some(*entity);
            }
            EditorCmd::RotateEntity {
                entity,
                new_rotation,
                ..
            } => {
                if let Some(t) = world.get_mut::<crate::components::Transform>(*entity) {
                    t.rotation = *new_rotation;
                }
                *selected = Some(*entity);
            }
            EditorCmd::PaintTiles { entity, changes } => {
                if let Some(tm) = world.get_mut::<crate::tilemap::Tilemap>(*entity) {
                    for (row, col, _old, new) in changes.iter() {
                        tm.set_tile(*row, *col, *new);
                    }
                }
                // Re-sync colliders so redo re-applies the post-stroke physics state too.
                crate::physics::sync_tilemap_entity_colliders(world, *entity);
                *selected = Some(*entity);
            }
        }
        self.undo.push(cmd);
    }
}

/// Persisted docked-editor preferences (snap / grid / paint tool + brush). Written to a RON
/// config file when the editor closes and restored when it next opens.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct EditorSettings {
    pub snap_enabled: bool,
    pub snap_size: f32,
    pub show_grid: bool,
    #[serde(default)]
    pub show_bounds: bool,
    #[serde(default)]
    pub show_pathgrid: bool,
    pub paint_brush: u32,
    pub paint_tool: PaintTool,
}

#[cfg(not(target_arch = "wasm32"))]
impl EditorSettings {
    pub(super) fn from_state(s: &EditorState) -> Self {
        Self {
            snap_enabled: s.snap_enabled,
            snap_size: s.snap_size,
            show_grid: s.show_grid,
            show_bounds: s.show_bounds,
            show_pathgrid: s.show_pathgrid,
            paint_brush: s.paint_brush,
            paint_tool: s.paint_tool,
        }
    }

    pub(super) fn apply_to(&self, s: &mut EditorState) {
        s.snap_enabled = self.snap_enabled;
        s.snap_size = self.snap_size;
        s.show_grid = self.show_grid;
        s.show_bounds = self.show_bounds;
        s.show_pathgrid = self.show_pathgrid;
        s.paint_brush = self.paint_brush;
        s.paint_tool = self.paint_tool;
    }
}

/// Config-dir path for the persisted editor settings.
#[cfg(not(target_arch = "wasm32"))]
fn editor_settings_path() -> std::path::PathBuf {
    crate::save::save_path("skeleton-engine", "editor_settings.ron")
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn entity_to_def(world: &World, entity: Entity) -> Option<crate::prefab::EntityDef> {
    let components = world
        .resource::<crate::prefab::SerdeComponentRegistry>()
        .map(|r| r.serialize_entity(world, entity))
        .unwrap_or_default();
    Some(crate::prefab::EntityDef {
        tag: world.get::<crate::prefab::Tag>(entity).map(|t| t.0.clone()),
        transform: world.get::<crate::components::Transform>(entity).cloned(),
        sprite: world.get::<crate::components::Sprite>(entity).cloned(),
        // Resolve the parent link to the parent's Tag (EntityDef.parent is tag-based) so
        // Undo-of-Delete / Duplicate / Paste restore the hierarchy, not a root entity.
        // Mirrors do_save_scene_with_list's parent resolution.
        parent: world
            .get::<crate::hierarchy::Parent>(entity)
            .and_then(|p| world.get::<crate::prefab::Tag>(p.0).map(|t| t.0.clone())),
        components,
    })
}

#[cfg(not(target_arch = "wasm32"))]
impl App {
    /// Copy the named serde-registered component off `sel` into the component clipboard.
    /// No-op if the component isn't present or isn't serde-registered.
    pub(in crate::app) fn copy_component(&mut self, sel: Entity, type_name: &str) {
        let value = self
            .world
            .resource::<crate::prefab::SerdeComponentRegistry>()
            .and_then(|r| r.serialize_entity(&self.world, sel).remove(type_name));
        if let Some(value) = value {
            self.editor.component_clipboard = Some((type_name.to_string(), value));
        }
    }

    /// Paste the clipboard component onto `sel`, inserting or overwriting that one component.
    /// Uses the `remove_resource → deserialize_into → insert_resource` dance (the registry
    /// cannot be borrowed while `&mut World` is needed). Not pushed to undo history — matching
    /// the editor's existing Add/Remove-component actions.
    pub(in crate::app) fn paste_component(&mut self, sel: Entity) {
        let Some((name, value)) = self.editor.component_clipboard.clone() else {
            return;
        };
        if let Some(registry) = self
            .world
            .remove_resource::<crate::prefab::SerdeComponentRegistry>()
        {
            let mut one = std::collections::HashMap::new();
            one.insert(name, value);
            registry.deserialize_into(&mut self.world, sel, &one);
            self.world.insert_resource(registry);
        }
    }

    /// Save the selected entity as a prefab RON file at `path` (captures tag/transform/sprite/
    /// parent + serde-registered components via `entity_to_def`). Sets `prefab_status`.
    pub(in crate::app) fn save_selected_as_prefab(&mut self, sel: Entity, path: &str) {
        let status = match entity_to_def(&self.world, sel) {
            Some(def) => {
                let prefab = crate::prefab::Prefab { def };
                match prefab.save(std::path::Path::new(path)) {
                    Ok(()) => format!("Saved prefab → {path}"),
                    Err(e) => format!("Save failed: {e}"),
                }
            }
            None => "No entity to save".to_string(),
        };
        self.editor.prefab_status = Some(status);
    }

    /// Load a prefab from `path` and spawn it (with a `PrefabInstance` marker so Break Prefab
    /// works), then select the new entity. Sets `prefab_status`.
    pub(in crate::app) fn spawn_prefab(&mut self, path: &str) {
        let status = match crate::prefab::Prefab::load(std::path::Path::new(path)) {
            Ok(prefab) => {
                let e = prefab.spawn_with_tracking(&mut self.world, path.to_string());
                self.editor.inspector_selected = Some(e);
                self.editor.selected_entities = vec![e];
                format!("Spawned prefab from {path}")
            }
            Err(e) => format!("Load failed: {e}"),
        };
        self.editor.prefab_status = Some(status);
    }

    /// Write the current docked-editor preferences to the RON config file.
    pub(in crate::app) fn save_editor_settings(&self) {
        let settings = EditorSettings::from_state(&self.editor);
        let _ = crate::save::write_ron(&editor_settings_path(), &settings);
    }

    /// Load persisted editor preferences (if the config file exists) and apply them.
    pub(in crate::app) fn load_editor_settings(&mut self) {
        let path = editor_settings_path();
        if crate::save::exists(&path) {
            if let Ok(settings) = crate::save::read_ron::<EditorSettings>(&path) {
                settings.apply_to(&mut self.editor);
            }
        }
    }

    /// Draw the debug bounds overlay via `DebugDraw`: every entity's Transform AABB plus any
    /// collision `Collider` shape. Called each frame from the editor UI while `show_bounds` is on.
    pub(in crate::app) fn draw_debug_bounds(&mut self) {
        use crate::collision::Collider;
        // Collect first so the immutable world borrow is released before borrowing DebugDraw.
        let bounds: Vec<(glam::Vec2, glam::Vec2)> = self
            .world
            .query::<crate::components::Transform>()
            .map(|(_, t)| (t.position, t.scale))
            .collect();
        let colliders: Vec<(glam::Vec2, Collider)> = self
            .world
            .query2::<crate::components::Transform, Collider>()
            .map(|(_, t, c)| (t.position, *c))
            .collect();
        let Some(dbg) = self.world.resource_mut::<crate::resources::DebugDraw>() else {
            return;
        };
        let bound_color = crate::color::Color::rgba(0.3, 0.8, 1.0, 0.45);
        let col_color = crate::color::Color::rgba(0.3, 1.0, 0.4, 0.7);
        for (pos, scale) in bounds {
            let half = scale * 0.5;
            dbg.rect(pos - half, pos + half, bound_color);
        }
        for (pos, col) in colliders {
            match col {
                Collider::Aabb { half_extents } => {
                    dbg.rect(pos - half_extents, pos + half_extents, col_color);
                }
                Collider::Circle { radius } => {
                    dbg.circle(pos, radius, col_color);
                }
            }
        }
    }

    /// Draw the pathfinding-grid overlay via `DebugDraw`: for every `Tilemap` entity, build a
    /// [`crate::pathfinding::PathGrid`] (the standard "non-zero tile = blocked" convention) and
    /// shade each cell — blocked cells filled red, walkable cells outlined green. Called each
    /// frame from the editor UI while `show_pathgrid` is on. Visualizes exactly the grid a game
    /// following that convention would navigate, with no changes required to the game.
    pub(in crate::app) fn draw_pathfinding_overlay(&mut self) {
        use crate::pathfinding::PathGrid;
        use crate::tilemap::Tilemap;

        // We need a mutable borrow of DebugDraw and an immutable borrow of each Tilemap
        // at the same time, which the borrow checker forbids via `world`. To avoid
        // cloning the full Tilemap (which includes a TilemapAtlas with a heap-allocated
        // texture String), we collect only the minimal data needed by the overlay:
        // the tile grid (unavoidable — PathGrid::from_tilemap reads every cell),
        // plus tile_size and origin (two f32 + Vec2 scalars). The atlas is not needed
        // and is not cloned.
        struct TilemapSnapshot {
            tiles: Vec<Vec<u32>>,
            tile_size: f32,
            origin: glam::Vec2,
        }
        let snapshots: Vec<TilemapSnapshot> = self
            .world
            .query::<Tilemap>()
            .map(|(_, tm)| TilemapSnapshot {
                tiles: tm.tiles.clone(),
                tile_size: tm.tile_size,
                origin: tm.origin,
            })
            .collect();

        let Some(dbg) = self.world.resource_mut::<crate::resources::DebugDraw>() else {
            return;
        };
        let blocked_color = crate::color::Color::rgba(1.0, 0.3, 0.3, 0.35);
        let walkable_color = crate::color::Color::rgba(0.3, 1.0, 0.5, 0.25);
        for snap in snapshots {
            // Build a temporary Tilemap using the moved tile grid (no second clone).
            // The atlas is not used by PathGrid::from_tilemap; a zero-cost dummy is fine.
            let tm = Tilemap::new(
                crate::tilemap::TilemapAtlas::new("", 0, 0),
                snap.tiles,
                snap.tile_size,
                snap.origin,
            );
            let grid = PathGrid::from_tilemap(&tm, |id| id != 0);
            let half = glam::Vec2::splat(snap.tile_size * 0.5);
            for y in 0..grid.height {
                for x in 0..grid.width {
                    let center = tm.cell_center_world(y as usize, x as usize);
                    if grid.is_walkable(x, y) {
                        dbg.rect(center - half, center + half, walkable_color);
                    } else {
                        dbg.rect_filled_z(center - half, center + half, blocked_color, 0.0);
                    }
                }
            }
        }
    }

    /// Reset the selected entity's `ParticleEmitter` to its default configuration, preserving the
    /// currently-assigned texture. Backs the Particle Tuner "Reset to Default" button. No-op if the
    /// entity has no `ParticleEmitter`.
    pub(in crate::app) fn reset_particle_emitter(&mut self, sel: Entity) {
        if let Some(em) = self.world.get_mut::<crate::particle::ParticleEmitter>(sel) {
            let texture = em.texture.clone();
            *em = crate::particle::ParticleEmitter::default();
            em.texture = texture;
        }
    }

    /// Reset the selected entity's `PointLight` to its default configuration. Backs the Point Light
    /// editor "Reset to Default" button. No-op if the entity has no `PointLight`.
    pub(in crate::app) fn reset_point_light(&mut self, sel: Entity) {
        if let Some(light) = self.world.get_mut::<crate::components::PointLight>(sel) {
            *light = crate::components::PointLight::default();
        }
    }

    /// Ensure an `AmbientLight` resource exists (inserting the default if absent) so the editor's
    /// Ambient Light control always has something to edit. Returns whether one had to be inserted.
    pub(in crate::app) fn ensure_ambient_light(&mut self) -> bool {
        if self
            .world
            .resource::<crate::resources::AmbientLight>()
            .is_none()
        {
            self.world
                .insert_resource(crate::resources::AmbientLight::default());
            true
        } else {
            false
        }
    }
}

/// Case-insensitive substring match for the entity-list search box. An empty (or
/// whitespace-only) filter matches every entity.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn entity_matches_filter(label: &str, filter: &str) -> bool {
    let filter = filter.trim();
    filter.is_empty() || label.to_lowercase().contains(&filter.to_lowercase())
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn snap_to_grid(pos: glam::Vec2, snap_size: f32) -> glam::Vec2 {
    glam::Vec2::new(
        (pos.x / snap_size).round() * snap_size,
        (pos.y / snap_size).round() * snap_size,
    )
}

#[cfg(not(target_arch = "wasm32"))]
impl App {
    pub(super) fn register_default_components(&mut self) {
        self.register_component("Sprite", |world, e| {
            world.add_component(e, crate::components::Sprite::default());
        });
        self.register_component("RenderLayer", |world, e| {
            world.add_component(e, crate::components::RenderLayer::default());
        });
        self.register_component("ParticleEmitter", |world, e| {
            world.add_component(e, crate::particle::ParticleEmitter::default());
        });
        self.register_component("PointLight", |world, e| {
            world.add_component(e, crate::components::PointLight::default());
        });
        // UI widget component factories — names must match the serde registration names
        // used by `core_resources::init_ui_serde_components` so "+ Add Component" and
        // scene save/load agree on the key strings.
        self.register_component("UiNode", |world, e| {
            world.add_component(e, crate::ui::UiNode::default());
        });
        self.register_component("Button", |world, e| {
            world.add_component(e, crate::ui::Button::default());
        });
        self.register_component("Label", |world, e| {
            world.add_component(e, crate::ui::Label::default());
        });
        self.register_component("TextInput", |world, e| {
            world.add_component(e, crate::ui::TextInput::default());
        });
        self.register_component("Slider", |world, e| {
            world.add_component(e, crate::ui::Slider::default());
        });
        self.register_component("CheckBox", |world, e| {
            world.add_component(e, crate::ui::CheckBox::default());
        });
        self.register_component("ScrollView", |world, e| {
            world.add_component(e, crate::ui::ScrollView::default());
        });
        self.register_component("Panel", |world, e| {
            world.add_component(e, crate::ui::panel::Panel::default());
        });
        // register removal closures — the Inspector "✕" button uses this map to show/act
        self.register_component_remover("Sprite", |world, e| {
            world.remove_component::<crate::components::Sprite>(e);
        });
        self.register_component_remover("RenderLayer", |world, e| {
            world.remove_component::<crate::components::RenderLayer>(e);
        });
        self.register_component_remover("ParticleEmitter", |world, e| {
            world.remove_component::<crate::particle::ParticleEmitter>(e);
        });
        self.register_component_remover("PointLight", |world, e| {
            world.remove_component::<crate::components::PointLight>(e);
        });
        self.register_component_remover("Tag", |world, e| {
            world.remove_component::<crate::prefab::Tag>(e);
        });
        self.register_component_remover("UiNode", |world, e| {
            world.remove_component::<crate::ui::UiNode>(e);
        });
        self.register_component_remover("Button", |world, e| {
            world.remove_component::<crate::ui::Button>(e);
        });
        self.register_component_remover("Label", |world, e| {
            world.remove_component::<crate::ui::Label>(e);
        });
        self.register_component_remover("TextInput", |world, e| {
            world.remove_component::<crate::ui::TextInput>(e);
        });
        self.register_component_remover("Slider", |world, e| {
            world.remove_component::<crate::ui::Slider>(e);
        });
        self.register_component_remover("CheckBox", |world, e| {
            world.remove_component::<crate::ui::CheckBox>(e);
        });
        self.register_component_remover("ScrollView", |world, e| {
            world.remove_component::<crate::ui::ScrollView>(e);
        });
        self.register_component_remover("Panel", |world, e| {
            world.remove_component::<crate::ui::panel::Panel>(e);
        });

        // ── Built-in inspector sub-panels ─────────────────────────────────────
        // Registered via the same API available to forkers; order here determines
        // the render order in the docked inspector.
        self.register_inspector_panel::<crate::particle::ParticleEmitter>(
            "Particle Tuner",
            |ui_panel, app, e| {
                ui::particle_tuner_grid(ui_panel, app, e);
                if ui_panel
                    .button("↺ Reset to Default")
                    .on_hover_text("reset all fields (keeps the texture)")
                    .clicked()
                {
                    app.reset_particle_emitter(e);
                }
                ui_panel.label(
                    egui::RichText::new("edits apply live while the sim runs (unpause)").weak(),
                );
            },
        );

        self.register_inspector_panel::<crate::components::PointLight>(
            "Point Light",
            |ui_panel, app, e| {
                ui::point_light_grid(ui_panel, app, e);
                if ui_panel
                    .button("↺ Reset to Default")
                    .on_hover_text("reset color / radius / intensity / height")
                    .clicked()
                {
                    app.reset_point_light(e);
                }
                ui_panel.label(
                    egui::RichText::new("the entity's Transform position is the light position")
                        .weak(),
                );
            },
        );

        self.register_inspector_panel::<crate::animation::AnimationStateMachine>(
            "State Machine",
            |ui_panel, app, e| {
                ui::state_machine_panel(ui_panel, app, e);
            },
        );

        self.register_inspector_panel::<crate::timeline::Timeline>(
            "Timeline",
            |ui_panel, app, e| {
                ui::timeline_panel(ui_panel, app, e);
            },
        );
    }

    pub fn register_component(
        &mut self,
        name: impl Into<String>,
        factory: impl Fn(&mut World, Entity) + Send + Sync + 'static,
    ) {
        self.editor
            .component_factories
            .insert(name.into(), Box::new(factory));
    }

    /// Registers a removal closure so the Inspector can remove a component.
    /// Call this with the same name as `register_component` to make custom components
    /// removable; components absent from this map have their "✕" button hidden.
    pub fn register_component_remover(
        &mut self,
        name: impl Into<String>,
        remover: impl Fn(&mut World, Entity) + Send + Sync + 'static,
    ) {
        self.editor
            .component_removers
            .insert(name.into(), Box::new(remover));
    }
}

impl App {
    /// Resync the static tile colliders of `entity` against the [`PhysicsWorld`](crate::physics::PhysicsWorld)
    /// resource. Thin wrapper over [`sync_tilemap_entity_colliders`](crate::physics::sync_tilemap_entity_colliders);
    /// no-op (returns `false`) unless `entity` has a `Tilemap` + `TilemapColliders` and a `PhysicsWorld`
    /// resource exists. The editor calls this after a Tile Paint stroke; games call it after `set_tile`.
    ///
    /// Native-only: the `physics` module (rapier2d) is excluded from wasm builds.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sync_tilemap_colliders(&mut self, entity: Entity) -> bool {
        crate::physics::sync_tilemap_entity_colliders(&mut self.world, entity)
    }

    /// Load a RON data table from `path` and register it under `name`.
    ///
    /// Lazily inserts a [`crate::data_table::DataTableRegistry`] resource if one is
    /// not yet present. On native builds the path is also registered with the
    /// `AssetServer` file watcher so that disk changes are hot-reloaded.
    ///
    /// Errors (file not found, parse failure) are logged via `log::warn!` and
    /// silently dropped — the registry will simply not contain the table.
    ///
    /// This method is a no-op on wasm (file I/O is unsupported there).
    pub fn load_data_table(&mut self, name: impl Into<String>, path: impl Into<String>) {
        let name = name.into();
        let path = path.into();

        // Ensure the registry resource exists.
        if self
            .world
            .resource::<crate::data_table::DataTableRegistry>()
            .is_none()
        {
            self.world
                .insert_resource(crate::data_table::DataTableRegistry::default());
        }
        // Preserve the DataTableRegistry across scene Replace world resets so loaded
        // tables (and any unsaved editor edits) survive the transition.
        self.register_persistent::<crate::data_table::DataTableRegistry>();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(reg) = self
                .world
                .resource_mut::<crate::data_table::DataTableRegistry>()
            {
                if let Err(e) = reg.load(name, &path) {
                    log::warn!("load_data_table: failed to load '{path}': {e}");
                    return;
                }
            }
            // Register with the file watcher for hot-reload.
            if let Some(assets) = self.world.resource_mut::<crate::asset::AssetServer>() {
                assets.watch_data_table_path(&path);
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            // File I/O is not supported on wasm; log and return.
            let _ = (name, path);
        }
    }

    /// Load an animation clip set from `path` and register it under `name`.
    ///
    /// Lazily inserts an [`crate::animation::clip_set::AnimationClipRegistry`] resource
    /// if one is not yet present. On native builds the path is also registered with the
    /// `AssetServer` file watcher so that disk changes are hot-reloaded.
    ///
    /// Errors (file not found, parse failure) are logged via `log::warn!` and silently
    /// dropped — the registry will simply not contain the set.
    ///
    /// This method is a no-op on wasm (file I/O is unsupported there).
    pub fn load_animation_clips(&mut self, name: impl Into<String>, path: impl Into<String>) {
        use crate::animation::clip_set::AnimationClipRegistry;

        let name = name.into();
        let path = path.into();

        // Ensure the registry resource exists.
        if self.world.resource::<AnimationClipRegistry>().is_none() {
            self.world.insert_resource(AnimationClipRegistry::default());
        }
        self.register_persistent::<AnimationClipRegistry>();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(reg) = self.world.resource_mut::<AnimationClipRegistry>() {
                if let Err(e) = reg.load(name, &path) {
                    log::warn!("load_animation_clips: failed to load '{path}': {e}");
                    return;
                }
            }
            // Register with the file watcher for hot-reload.
            if let Some(assets) = self.world.resource_mut::<crate::asset::AssetServer>() {
                assets.watch_animation_clip_path(&path);
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = (name, path);
        }
    }

    /// Load a RON particle-config file from `path` and register it under `name`.
    ///
    /// Lazily inserts a [`crate::particle::ParticleConfigRegistry`] resource if one
    /// is not yet present. On native builds the path is also registered with the
    /// `AssetServer` file watcher so that disk changes are hot-reloaded.
    ///
    /// Errors (file not found, parse failure) are logged via `log::warn!` and
    /// silently dropped — the registry will simply not contain the config set.
    ///
    /// This method is a no-op on wasm (file I/O is unsupported there).
    pub fn load_particle_configs(&mut self, name: impl Into<String>, path: impl Into<String>) {
        let name = name.into();
        let path = path.into();

        // Ensure the registry resource exists.
        if self
            .world
            .resource::<crate::particle::ParticleConfigRegistry>()
            .is_none()
        {
            self.world
                .insert_resource(crate::particle::ParticleConfigRegistry::default());
        }
        // Preserve the registry across scene Replace world resets.
        self.register_persistent::<crate::particle::ParticleConfigRegistry>();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(reg) = self
                .world
                .resource_mut::<crate::particle::ParticleConfigRegistry>()
            {
                if let Err(e) = reg.load(name, &path) {
                    log::warn!("load_particle_configs: failed to load '{path}': {e}");
                    return;
                }
            }
            // Register with the file watcher for hot-reload.
            if let Some(assets) = self.world.resource_mut::<crate::asset::AssetServer>() {
                assets.watch_particle_config_path(&path);
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            // File I/O is not supported on wasm; log and return.
            let _ = (name, path);
        }
    }

    /// Registers a component for full editor integration in one call:
    /// Inspector field editing ([`Reflect`](crate::reflect::Reflect)), entity duplication
    /// ([`Clone`]), scene save/load (serde), and the Add/Remove Component buttons.
    ///
    /// `T` must derive `Reflect`, `Serialize`, `Deserialize`, `Clone`, and `Default`.
    ///
    /// This is the preferred registration path for game-side stats/config components.
    /// On wasm only the reflect + clone + serde registrations run (the editor buttons
    /// are native-only); the method still compiles on both targets.
    ///
    /// # Platform-gated registrations
    ///
    /// [`register_component`](Self::register_component) (the "Add Component" factory) and
    /// [`register_component_remover`](Self::register_component_remover) (the "Remove
    /// Component" button) are **native-only** — they drive the docked editor UI which does
    /// not exist on wasm. On wasm only the reflect, clone, and serde registrations apply,
    /// which still cover `Inspector` field display, entity clone, and scene round-trips.
    ///
    /// # Example
    /// ```rust,no_run
    /// use engine::App;
    /// use engine_reflect_derive::Reflect;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Reflect, Serialize, Deserialize, Clone, Default)]
    /// struct Stats { hp: f32, strength: i32 }
    ///
    /// let mut app = App::new();
    /// app.register_editable_component::<Stats>("Stats", None);
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn register_editable_component<T>(
        &mut self,
        name: &'static str,
        post_spawn: Option<Box<dyn Fn(&mut World, Entity) + Send + Sync>>,
    ) where
        T: crate::reflect::Reflect
            + serde::Serialize
            + serde::de::DeserializeOwned
            + Clone
            + Default
            + Send
            + Sync
            + 'static,
    {
        self.world.register_reflect_named::<T>(name);
        self.world.register_clone::<T>();
        // Push a replay thunk for reflect+clone so they survive scene Replace.
        // `name` is `&'static str` (Copy), so no clone needed.
        self.world_registrars.push(Box::new(move |world| {
            world.register_reflect_named::<T>(name);
            world.register_clone::<T>();
        }));
        self.register_serde_component::<T>(name, post_spawn);
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.register_component(name, |world, entity| {
                world.add_component(entity, T::default());
            });
            self.register_component_remover(name, |world, entity| {
                world.remove_component::<T>(entity);
            });
        }
    }

    /// Registers a custom inspector sub-panel that appears in the docked editor whenever
    /// the selected entity has component `T`.
    ///
    /// The panel is rendered as a [`egui::CollapsingHeader`] with the given `title`,
    /// open by default, inserted **after** the built-in sub-panels (Particle Tuner,
    /// Point Light, State Machine, Timeline) and **before** the Component List.
    ///
    /// This is a **native-only** method — it compiles out on wasm. Gate any call
    /// site with `#[cfg(not(target_arch = "wasm32"))]` when needed.
    ///
    /// # Example
    /// ```rust,no_run
    /// use engine::App;
    ///
    /// #[derive(Default)]
    /// struct HeatSource { temperature: f32, radius: f32 }
    ///
    /// let mut app = App::new();
    /// app.register_inspector_panel::<HeatSource>("Heat Source", |ui, app, entity| {
    ///     if let Some(h) = app.world.get_mut::<HeatSource>(entity) {
    ///         ui.horizontal(|ui| {
    ///             ui.label("temperature");
    ///             ui.add(egui::DragValue::new(&mut h.temperature).speed(0.5));
    ///         });
    ///         ui.horizontal(|ui| {
    ///             ui.label("radius");
    ///             ui.add(egui::DragValue::new(&mut h.radius).range(0.0..=1000.0).speed(1.0));
    ///         });
    ///     }
    /// });
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn register_inspector_panel<T: 'static>(
        &mut self,
        title: impl Into<String>,
        draw: impl Fn(&mut egui::Ui, &mut App, Entity) + 'static,
    ) {
        self.editor
            .inspector_panels
            .push(crate::app::editor::InspectorPanel {
                presence: |world, entity| world.has_component::<T>(entity),
                title: title.into(),
                draw: Box::new(draw),
            });
    }

    /// Registers a serde-capable component type so it is included in scene save/load.
    ///
    /// Call this for every component `T` that should survive a round-trip through a
    /// `.ron` scene file. The `name` must be unique across all registered types and is
    /// used as the key in [`crate::prefab::EntityDef::components`].
    ///
    /// `post_spawn` is an optional closure run after deserialization — useful for
    /// copying design-time fields to runtime counterparts (e.g. `initial_text` → `text`).
    ///
    /// # Example
    /// ```rust,no_run
    /// use engine::App;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Clone, Serialize, Deserialize)]
    /// struct Health { max: f32 }
    ///
    /// let mut app = App::new();
    /// app.register_serde_component::<Health>("Health", None);
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn register_serde_component<T>(
        &mut self,
        name: impl Into<String>,
        post_spawn: Option<Box<dyn Fn(&mut World, Entity) + Send + Sync>>,
    ) where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + Sync + 'static,
    {
        let name: String = name.into();
        // Convert Box → Arc so the closure can be cloned into the replay thunk below.
        let ps: Option<std::sync::Arc<dyn Fn(&mut World, Entity) + Send + Sync>> =
            post_spawn.map(std::sync::Arc::from);
        Self::do_register_serde_component::<T>(&mut self.world, name.clone(), ps.clone());

        // Record a replay thunk: after a scene Replace resets the World, this closure
        // re-inserts the serde registration so scene save/load keeps working.
        self.world_registrars.push(Box::new(move |world| {
            Self::do_register_serde_component::<T>(world, name.clone(), ps.clone());
        }));
    }

    /// Shared implementation for the immediate registration and scene-reset replay.
    #[allow(clippy::type_complexity)]
    fn do_register_serde_component<T>(
        world: &mut World,
        name: String,
        post_spawn: Option<std::sync::Arc<dyn Fn(&mut World, Entity) + Send + Sync>>,
    ) where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + Sync + 'static,
    {
        if let Some(registry) = world.resource_mut::<crate::prefab::SerdeComponentRegistry>() {
            registry.register_arc::<T>(name, post_spawn);
        }
    }
}

// ── Editor command undo/redo tests ────────────────────────────────────────────

#[cfg(all(test, not(target_arch = "wasm32")))]
mod editor_cmd_tests {
    use super::*;
    use crate::ecs::World;
    use crate::prefab::Tag;

    // ── Fix A: DeleteEntity undo restores full def (including non-core components) ──

    /// Regression test for Fix A: undo of DeleteEntity must restore all
    /// components captured in `def`, not just tag/transform/sprite.
    #[test]
    fn delete_undo_restores_full_def() {
        let mut world = World::new();
        // Spawn an entity with tag + transform only (no serde components needed here,
        // the important thing is the def captures what was there).
        let e = world.spawn();
        world.add_component(e, Tag("Goblin".into()));
        world.add_component(e, crate::components::Transform::default());

        let def = entity_to_def(&world, e).expect("entity_to_def");
        assert_eq!(def.tag.as_deref(), Some("Goblin"));

        // Simulate the Delete button: push DeleteEntity, then despawn.
        let mut history = EditorHistory::new();
        history.push(EditorCmd::DeleteEntity {
            entity: None,
            def: def.clone(),
        });
        world.despawn(e);
        assert!(!world.is_alive(e), "entity should be despawned");

        // Undo: entity must come back with its tag.
        let mut sel: Option<Entity> = None;
        history.undo(&mut world, &mut sel);
        let restored = sel.expect("undo must set selection");
        assert!(world.is_alive(restored), "entity must be alive after undo");
        let tag = world
            .get::<Tag>(restored)
            .expect("Tag must be restored after undo");
        assert_eq!(tag.0, "Goblin");

        // Redo: entity must be despawned again.
        history.redo(&mut world, &mut sel);
        assert!(
            sel.is_none() || !world.is_alive(sel.unwrap()),
            "entity must be dead after redo"
        );
    }

    // ── Fix B: CreateEntity with def round-trips through undo/redo ────────────

    /// Regression test for Fix B: undo of CreateEntity (Duplicate) must despawn
    /// the entity; redo must re-spawn it with its original components.
    #[test]
    fn create_entity_with_def_undo_redo() {
        let mut world = World::new();
        // Spawn a "duplicated" entity.
        let e = world.spawn();
        world.add_component(e, Tag("Copy".into()));
        world.add_component(e, crate::components::Transform::default());

        let def = entity_to_def(&world, e);
        let mut history = EditorHistory::new();
        history.push(EditorCmd::CreateEntity {
            entity: e,
            def: def.clone(),
        });

        // Undo: entity must be despawned.
        let mut sel: Option<Entity> = Some(e);
        history.undo(&mut world, &mut sel);
        assert!(sel.is_none());
        assert!(!world.is_alive(e), "entity must be despawned after undo");

        // Redo: entity must be re-spawned from def with its tag.
        history.redo(&mut world, &mut sel);
        let redone = sel.expect("redo must set selection");
        assert!(world.is_alive(redone));
        let tag = world
            .get::<Tag>(redone)
            .expect("Tag must be present after redo");
        assert_eq!(tag.0, "Copy");
    }

    // ── Fix B: CreateEntity without def (New Entity) still works ─────────────

    #[test]
    fn create_entity_no_def_undo_redo() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, crate::components::Transform::default());
        world.add_component(e, Tag("New Entity".into()));

        let mut history = EditorHistory::new();
        history.push(EditorCmd::CreateEntity {
            entity: e,
            def: None,
        });

        let mut sel: Option<Entity> = Some(e);
        history.undo(&mut world, &mut sel);
        assert!(sel.is_none());
        assert!(!world.is_alive(e));

        // Redo of None-def spawns a fresh default entity.
        history.redo(&mut world, &mut sel);
        let redone = sel.expect("redo must yield an entity");
        assert!(world.is_alive(redone));
    }

    // ── entity_to_def captures the parent link (Undo/Duplicate restore hierarchy) ──
    #[test]
    fn entity_to_def_captures_parent_tag() {
        let mut world = World::new();
        let parent = world.spawn();
        world.add_component(parent, Tag("Parent".into()));
        let child = world.spawn();
        world.add_component(child, Tag("Child".into()));
        world.add_component(child, crate::components::Transform::default());
        world.add_component(child, crate::hierarchy::Parent(parent));

        let def = entity_to_def(&world, child).expect("entity_to_def");
        assert_eq!(
            def.parent.as_deref(),
            Some("Parent"),
            "entity_to_def must capture the parent's Tag so Undo-of-Delete restores hierarchy"
        );
    }

    #[test]
    fn entity_filter_matches_case_insensitive_substring() {
        assert!(entity_matches_filter("Player", "")); // empty matches all
        assert!(entity_matches_filter("Player", "  ")); // whitespace-only matches all
        assert!(entity_matches_filter("Player", "lay")); // substring
        assert!(entity_matches_filter("Player", "PLAYER")); // case-insensitive
        assert!(entity_matches_filter("EnemyGoblin", "goblin"));
        assert!(!entity_matches_filter("Player", "enemy")); // no match
    }

    #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Debug)]
    struct CopyTest {
        v: i32,
    }

    #[test]
    fn component_copy_paste_round_trips() {
        let mut app = crate::app::App::new();
        app.register_serde_component::<CopyTest>("CopyTest", None);

        let a = app.world.spawn();
        app.world.add_component(a, CopyTest { v: 7 });

        // Copy from A.
        app.copy_component(a, "CopyTest");
        assert!(
            app.editor.component_clipboard.is_some(),
            "clipboard populated by copy"
        );

        // Paste onto a fresh entity B.
        let b = app.world.spawn();
        app.paste_component(b);
        assert_eq!(
            app.world.get::<CopyTest>(b),
            Some(&CopyTest { v: 7 }),
            "pasted component matches the copied value"
        );
    }

    #[test]
    fn component_copy_unregistered_is_noop() {
        let mut app = crate::app::App::new();
        // No serde registration for CopyTest → copy finds nothing.
        let a = app.world.spawn();
        app.world.add_component(a, CopyTest { v: 1 });
        app.copy_component(a, "CopyTest");
        assert!(app.editor.component_clipboard.is_none());
    }

    #[test]
    fn prefab_save_spawn_round_trips() {
        let mut app = crate::app::App::new();
        app.register_serde_component::<CopyTest>("CopyTest", None);

        let a = app.world.spawn();
        app.world
            .add_component(a, crate::prefab::Tag("Hero".into()));
        app.world.add_component(
            a,
            crate::components::Transform::new(
                glam::Vec2::new(7.0, 9.0),
                glam::Vec2::splat(32.0),
                0.0,
            ),
        );
        app.world.add_component(a, CopyTest { v: 42 });

        let path = std::env::temp_dir().join(format!("test_prefab_{}.ron", std::process::id()));
        let path_str = path.to_string_lossy().to_string();

        app.save_selected_as_prefab(a, &path_str);
        assert!(path.exists(), "prefab file written");

        // Spawn it back into a fresh entity.
        app.spawn_prefab(&path_str);
        let b = app
            .editor
            .inspector_selected
            .expect("spawned prefab selected");
        assert_ne!(a, b);
        assert_eq!(
            app.world.get::<crate::prefab::Tag>(b).map(|t| t.0.as_str()),
            Some("Hero")
        );
        assert_eq!(
            app.world
                .get::<crate::components::Transform>(b)
                .map(|t| t.position),
            Some(glam::Vec2::new(7.0, 9.0))
        );
        assert_eq!(
            app.world.get::<CopyTest>(b),
            Some(&CopyTest { v: 42 }),
            "serde component round-trips through the prefab"
        );
        assert!(
            app.world.get::<crate::prefab::PrefabInstance>(b).is_some(),
            "spawned prefab carries a PrefabInstance marker"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn editor_settings_round_trip() {
        let mut s = EditorState::new();
        s.snap_enabled = true;
        s.snap_size = 24.0;
        s.show_grid = true;
        s.show_pathgrid = true;
        s.paint_brush = 5;
        s.paint_tool = PaintTool::Bucket;

        let settings = EditorSettings::from_state(&s);
        let path =
            std::env::temp_dir().join(format!("test_editor_settings_{}.ron", std::process::id()));
        crate::save::write_ron(&path, &settings).unwrap();
        let loaded: EditorSettings = crate::save::read_ron(&path).unwrap();
        assert_eq!(loaded, settings, "settings round-trip through RON");

        // Apply onto a fresh (default) state.
        let mut fresh = EditorState::new();
        loaded.apply_to(&mut fresh);
        assert!(fresh.snap_enabled);
        assert_eq!(fresh.snap_size, 24.0);
        assert!(fresh.show_grid);
        assert!(fresh.show_pathgrid);
        assert_eq!(fresh.paint_brush, 5);
        assert_eq!(fresh.paint_tool, PaintTool::Bucket);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn debug_bounds_draws_aabbs_and_colliders() {
        let mut app = crate::app::App::new();
        // Three Transform entities; one also has a collider.
        for _ in 0..3 {
            let e = app.world.spawn();
            app.world
                .add_component(e, crate::components::Transform::default());
            if app.world.query::<crate::components::Transform>().count() == 3 {
                app.world.add_component(
                    e,
                    crate::collision::Collider::Aabb {
                        half_extents: glam::Vec2::splat(8.0),
                    },
                );
            }
        }

        app.draw_debug_bounds();
        let dbg = app
            .world
            .resource::<crate::resources::DebugDraw>()
            .expect("DebugDraw resource");
        // 3 entity-bounds rects + 1 collider rect = 4 shapes.
        assert_eq!(dbg.shapes.len(), 4);
    }

    #[test]
    fn pathfinding_overlay_shades_blocked_and_walkable_cells() {
        use crate::tilemap::{Tilemap, TilemapAtlas};
        let mut app = crate::app::App::new();
        // 3×3 map, one blocked (non-zero) cell in the center.
        let tiles = vec![vec![0, 0, 0], vec![0, 1, 0], vec![0, 0, 0]];
        let tm = Tilemap::new(
            TilemapAtlas::new("t.png", 1, 1),
            tiles,
            16.0,
            glam::Vec2::ZERO,
        );
        let e = app.world.spawn();
        app.world.add_component(e, tm);

        app.draw_pathfinding_overlay();
        let dbg = app
            .world
            .resource::<crate::resources::DebugDraw>()
            .expect("DebugDraw resource");
        // 8 walkable cells → outline shapes; 1 blocked cell → filled rect.
        assert_eq!(dbg.shapes.len(), 8, "walkable cells drawn as outlines");
        assert_eq!(
            dbg.filled_rects.len(),
            1,
            "blocked cell drawn as a filled rect"
        );
    }

    #[test]
    fn reset_particle_emitter_restores_defaults_keeping_texture() {
        use crate::particle::ParticleEmitter;
        let mut app = crate::app::App::new();
        let e = app.world.spawn();
        let mut em = ParticleEmitter {
            spawn_rate: 999.0,
            lifetime: 42.0,
            emit: false,
            texture: Some(std::sync::Arc::from("spark.png")),
            ..Default::default()
        };
        em.velocity = glam::Vec2::new(123.0, 456.0);
        app.world.add_component(e, em);

        app.reset_particle_emitter(e);

        let got = app.world.get::<ParticleEmitter>(e).expect("emitter");
        let def = ParticleEmitter::default();
        assert_eq!(got.spawn_rate, def.spawn_rate, "spawn_rate reset");
        assert_eq!(got.lifetime, def.lifetime, "lifetime reset");
        assert_eq!(got.velocity, def.velocity, "velocity reset");
        assert!(got.emit, "emit reset to default (true)");
        assert_eq!(
            got.texture.as_deref(),
            Some("spark.png"),
            "texture is preserved across reset"
        );
    }

    #[test]
    fn reset_particle_emitter_no_emitter_is_noop() {
        let mut app = crate::app::App::new();
        let e = app.world.spawn();
        // No panic / no insertion when the entity lacks a ParticleEmitter.
        app.reset_particle_emitter(e);
        assert!(app
            .world
            .get::<crate::particle::ParticleEmitter>(e)
            .is_none());
    }

    #[test]
    fn reset_point_light_restores_defaults() {
        use crate::components::PointLight;
        let mut app = crate::app::App::new();
        let e = app.world.spawn();
        app.world.add_component(
            e,
            PointLight {
                color: crate::color::Color::rgb(1.0, 0.0, 0.0),
                radius: 12.0,
                intensity: 9.0,
                light_height: 1.9,
            },
        );

        app.reset_point_light(e);

        let got = app.world.get::<PointLight>(e).expect("light");
        let def = PointLight::default();
        assert_eq!(got.radius, def.radius);
        assert_eq!(got.intensity, def.intensity);
        assert_eq!(got.light_height, def.light_height);
        assert_eq!(got.color, def.color);
    }

    #[test]
    fn ensure_ambient_light_inserts_default_once() {
        let mut app = crate::app::App::new();
        // Fresh App has no AmbientLight resource → first call inserts it.
        assert!(
            app.ensure_ambient_light(),
            "first call inserts a default AmbientLight"
        );
        assert!(app
            .world
            .resource::<crate::resources::AmbientLight>()
            .is_some());
        // Second call is a no-op (already present).
        assert!(
            !app.ensure_ambient_light(),
            "second call leaves the existing resource untouched"
        );
    }

    #[test]
    fn pointlight_registered_as_editor_component() {
        let mut app = crate::app::App::new();
        // register_default_components (run by App::new) registers PointLight factory + remover.
        assert!(app.editor.component_factories.contains_key("PointLight"));
        assert!(app.editor.component_removers.contains_key("PointLight"));

        let e = app.world.spawn();
        // Disjoint field borrows: factory comes from `app.editor`, world is `app.world`.
        if let Some(factory) = app.editor.component_factories.get("PointLight") {
            factory(&mut app.world, e);
        }
        assert!(
            app.world.get::<crate::components::PointLight>(e).is_some(),
            "factory adds a PointLight"
        );
    }
}
