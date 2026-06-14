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
    },
    DeleteEntity {
        /// Entity id recreated by undo. Filled in at undo time so redo can despawn exactly
        /// this entity (None on first creation — the original has already been despawned).
        entity: Option<Entity>,
        tag: Option<String>,
        transform: Option<crate::components::Transform>,
        sprite: Option<crate::components::Sprite>,
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
            EditorCmd::CreateEntity { entity } => {
                world.despawn(*entity);
                *selected = None;
            }
            EditorCmd::DeleteEntity {
                tag,
                transform,
                sprite,
                ..
            } => {
                let e = world.spawn();
                if let Some(tr) = transform {
                    world.add_component(e, tr.clone());
                }
                if let Some(sp) = sprite {
                    world.add_component(e, sp.clone());
                }
                if let Some(t) = tag {
                    world.add_component(e, Tag(t.clone()));
                }
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
            EditorCmd::CreateEntity { entity: _ } => {
                // was despawned by undo — spawn a new entity with a new id
                let e = world.spawn();
                world.add_component(e, crate::components::Transform::default());
                world.add_component(e, Tag("New Entity".into()));
                *selected = Some(e);
                // push the cmd updated with the new id onto the undo stack so it can be undone again
                // (previously drop(cmd) broke the chain, making the recreated entity un-undoable)
                self.undo.push(EditorCmd::CreateEntity { entity: e });
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
        parent: None,
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
