use super::*;

mod state;
mod ui;

#[cfg(not(target_arch = "wasm32"))]
pub(super) mod docked_rt;

pub(super) use state::EditorState;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use state::ResizeHandle;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use state::{apply_f1, apply_f2, EditorMode};

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
        }
        self.undo.push(cmd);
    }
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
        self.register_component_remover("Tag", |world, e| {
            world.remove_component::<crate::prefab::Tag>(e);
        });
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
    /// # Example
    /// ```rust,no_run
    /// use engine::{App, Reflect};
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
    /// let mut app = App::new(Default::default());
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
}
