use crate::app::App;
#[cfg(not(target_arch = "wasm32"))]
use crate::ecs::Entity;

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

    /// Load a RON dialogue-tree file from `path` and register it under `name`.
    ///
    /// Lazily inserts a [`crate::dialogue::DialogueRegistry`] resource if one is not yet
    /// present. On native builds the path is also registered with the `AssetServer` file
    /// watcher so that disk changes are hot-reloaded. Build a [`DialogueBox`](crate::DialogueBox)
    /// from a loaded tree via [`DialogueRegistry::box_of`](crate::DialogueRegistry::box_of).
    ///
    /// Errors (file not found, parse failure) are logged via `log::warn!` and silently
    /// dropped — the registry will simply not contain the tree.
    ///
    /// This method is a no-op on wasm (file I/O is unsupported there).
    pub fn load_dialogue(&mut self, name: impl Into<String>, path: impl Into<String>) {
        use crate::dialogue::DialogueRegistry;

        let name = name.into();
        let path = path.into();

        // Ensure the registry resource exists.
        if self.world.resource::<DialogueRegistry>().is_none() {
            self.world.insert_resource(DialogueRegistry::default());
        }
        // Preserve the registry across scene Replace world resets.
        self.register_persistent::<DialogueRegistry>();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(reg) = self.world.resource_mut::<DialogueRegistry>() {
                if let Err(e) = reg.load(name, &path) {
                    log::warn!("load_dialogue: failed to load '{path}': {e}");
                    return;
                }
            }
            // Register with the file watcher for hot-reload (generic entry point).
            if let Some(assets) = self.world.resource_mut::<crate::asset::AssetServer>() {
                assets.watch_path(&path);
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = (name, path);
        }
    }
}
