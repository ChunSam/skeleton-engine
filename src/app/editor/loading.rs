use crate::app::App;
#[cfg(not(target_arch = "wasm32"))]
use crate::ecs::Entity;

impl App {
    /// Resync the static tile colliders of `entity` against the [`PhysicsWorld`](crate::physics::PhysicsWorld)
    /// resource. Thin wrapper over [`sync_tilemap_entity_colliders`](crate::physics::sync_tilemap_entity_colliders);
    /// The editor calls this after a Tile Paint stroke; games call it after `set_tile`.
    ///
    /// ⚠️ **The `bool` says a `PhysicsWorld` was there, not that anything synced.** It is `false`
    /// only when `entity` has no `Tilemap` or the resource is absent; a `Tilemap` with a
    /// `PhysicsWorld` and **no** `TilemapColliders` returns `true` having synced nothing, which
    /// `tile_collider.rs`'s own test asserts. Do not gate "colliders are up to date" on it. The
    /// doc said `false` for that case until 2026-09-03 and the code never did.
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
    /// A failure (file not found, parse error) is reported as an asset failure: logged at
    /// `error!` with the roots searched, and listed by [`asset_failures`](Self::asset_failures)
    /// so a game can refuse to start rather than boot on an empty table. With
    /// [`set_strict_assets(true)`](Self::set_strict_assets) it panics at the load instead. The
    /// registry itself is left without the table either way.
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
                if let Err(e) = reg.load(name.clone(), &path) {
                    crate::asset_path::record_failure(&path, format!("data table '{name}': {e}"));
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
            // File I/O is not supported on wasm: a silent no-op, as the doc above says. (This
            // said "log and return" until 2026-09-03 and nothing ever logged.)
            let _ = (name, path);
        }
    }

    /// Opens `path` as a data table registered under `name` — the Data Tables panel's Open
    /// button. Returns whether it opened.
    ///
    /// ⚠️ The file is validated through [`DataTable::load`](crate::data_table::DataTable::load),
    /// which returns a `Result`, **before** [`load_data_table`](Self::load_data_table), whose
    /// failure path goes to `asset_path::record_failure`: that logs at `error!`, appends to
    /// [`asset_failures`](Self::asset_failures) — which no editor UI reads — and **panics
    /// outright under [`set_strict_assets(true)`](Self::set_strict_assets)**. A typo in an editor
    /// text field must not be able to abort the process, and the person who typed it has to be
    /// able to see why (v0.156.16).
    ///
    /// On failure the typed name and path are left alone, so the typo can be corrected instead of
    /// retyped; on success they are cleared and the new table is selected.
    ///
    /// The file is read twice on the success path. This is a one-shot button, and the alternative
    /// is duplicating `load_data_table`'s watcher and persistent-resource registration here.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn editor_open_data_table(&mut self, name: String, path: String) -> bool {
        match crate::data_table::DataTable::load(&path) {
            Ok(_) => {
                self.load_data_table(name.clone(), path);
                self.editor.selected_data_table = Some(name);
                self.editor.data_table_status = None;
                self.editor.data_table_open_name.clear();
                self.editor.data_table_open_path.clear();
                true
            }
            Err(e) => {
                self.editor.data_table_status = Some(format!("✗ {path}: {e}"));
                false
            }
        }
    }

    /// Load an animation clip set from `path` and register it under `name`.
    ///
    /// Lazily inserts an [`crate::animation::clip_set::AnimationClipRegistry`] resource
    /// if one is not yet present. On native builds the path is also registered with the
    /// `AssetServer` file watcher so that disk changes are hot-reloaded.
    ///
    /// A failure (file not found, parse error) is reported as an asset failure — `error!` plus
    /// [`asset_failures`](Self::asset_failures), or a panic under
    /// [`set_strict_assets`](Self::set_strict_assets). The registry will simply not contain
    /// the set.
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
                if let Err(e) = reg.load(name.clone(), &path) {
                    crate::asset_path::record_failure(
                        &path,
                        format!("animation clip set '{name}': {e}"),
                    );
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
    /// A failure (file not found, parse error) is reported as an asset failure — `error!` plus
    /// [`asset_failures`](Self::asset_failures), or a panic under
    /// [`set_strict_assets`](Self::set_strict_assets). The registry will simply not contain
    /// the config set.
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
                if let Err(e) = reg.load(name.clone(), &path) {
                    crate::asset_path::record_failure(
                        &path,
                        format!("particle config set '{name}': {e}"),
                    );
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
            // File I/O is not supported on wasm: a silent no-op, as the doc above says. (This
            // said "log and return" until 2026-09-03 and nothing ever logged.)
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
    /// A failure (file not found, parse error) is reported as an asset failure — `error!` plus
    /// [`asset_failures`](Self::asset_failures), or a panic under
    /// [`set_strict_assets`](Self::set_strict_assets). The registry will simply not contain
    /// the tree.
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
                if let Err(e) = reg.load(name.clone(), &path) {
                    crate::asset_path::record_failure(
                        &path,
                        format!("dialogue tree '{name}': {e}"),
                    );
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

    /// Load a RON trigger-zone file from `path` and register the set under `name`.
    ///
    /// Lazily inserts a [`crate::trigger_zone::TriggerZoneRegistry`] resource if one is not yet
    /// present. On native builds the path is also registered with the `AssetServer` file watcher so
    /// disk changes are hot-reloaded. Spawn the loaded zones with
    /// [`spawn_trigger_zones`](Self::spawn_trigger_zones).
    ///
    /// A failure (file not found, parse error) is reported as an asset failure — `error!` plus
    /// [`asset_failures`](Self::asset_failures), or a panic under
    /// [`set_strict_assets`](Self::set_strict_assets). The registry will simply not contain the
    /// set. This method is a no-op on wasm (file I/O is unsupported there; parse with
    /// [`TriggerZoneSet::from_ron_str`](crate::TriggerZoneSet::from_ron_str)
    /// and [`TriggerZoneRegistry::insert`](crate::TriggerZoneRegistry::insert) instead).
    pub fn load_trigger_zones(&mut self, name: impl Into<String>, path: impl Into<String>) {
        use crate::trigger_zone::TriggerZoneRegistry;

        let name = name.into();
        let path = path.into();

        if self.world.resource::<TriggerZoneRegistry>().is_none() {
            self.world.insert_resource(TriggerZoneRegistry::default());
        }
        // Preserve the registry across scene Replace world resets.
        self.register_persistent::<TriggerZoneRegistry>();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(reg) = self.world.resource_mut::<TriggerZoneRegistry>() {
                if let Err(e) = reg.load(name.clone(), &path) {
                    crate::asset_path::record_failure(
                        &path,
                        format!("trigger-zone set '{name}': {e}"),
                    );
                    return;
                }
            }
            if let Some(assets) = self.world.resource_mut::<crate::asset::AssetServer>() {
                assets.watch_path(&path);
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = (name, path);
        }
    }

    /// Spawn the zones of the named [`TriggerZoneSet`](crate::TriggerZoneSet) (loaded via
    /// [`load_trigger_zones`](Self::load_trigger_zones)) into the world, returning the spawned
    /// entities. Cross-platform. Returns an empty vec (and warns) if no set is registered under
    /// `name`.
    pub fn spawn_trigger_zones(&mut self, name: &str) -> Vec<crate::ecs::Entity> {
        // Clone the set out so we can take `&mut World` to spawn it.
        let set = self
            .world
            .resource::<crate::trigger_zone::TriggerZoneRegistry>()
            .and_then(|reg| reg.get(name).cloned());
        match set {
            Some(set) => set.spawn_into(&mut self.world),
            None => {
                log::warn!("spawn_trigger_zones: no trigger-zone set registered under '{name}'");
                Vec::new()
            }
        }
    }

    /// Load a RON zone-effect binding table from `path` and register it under `name`.
    ///
    /// Lazily inserts a [`crate::zone_effect::ZoneEffectRegistry`] resource if one is not yet
    /// present. On native builds the path is also registered with the `AssetServer` file watcher so
    /// disk changes are hot-reloaded. Apply the loaded table with a
    /// [`ZoneEffectSystem::new(name)`](crate::ZoneEffectSystem) added after
    /// [`TriggerZoneSystem`](crate::TriggerZoneSystem).
    ///
    /// A failure (file not found, parse error) is reported as an asset failure — `error!` plus
    /// [`asset_failures`](Self::asset_failures), or a panic under
    /// [`set_strict_assets`](Self::set_strict_assets). The registry will simply not contain the
    /// table. This method is a no-op on wasm (file I/O is unsupported there; parse with
    /// [`ZoneEffectBindings::from_ron_str`](crate::ZoneEffectBindings::from_ron_str) and
    /// [`ZoneEffectRegistry::insert`](crate::ZoneEffectRegistry::insert) instead).
    pub fn load_zone_effects(&mut self, name: impl Into<String>, path: impl Into<String>) {
        use crate::zone_effect::ZoneEffectRegistry;

        let name = name.into();
        let path = path.into();

        if self.world.resource::<ZoneEffectRegistry>().is_none() {
            self.world.insert_resource(ZoneEffectRegistry::default());
        }
        // Preserve the registry across scene Replace world resets.
        self.register_persistent::<ZoneEffectRegistry>();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(reg) = self.world.resource_mut::<ZoneEffectRegistry>() {
                if let Err(e) = reg.load(name.clone(), &path) {
                    crate::asset_path::record_failure(
                        &path,
                        format!("zone-effect table '{name}': {e}"),
                    );
                    return;
                }
            }
            if let Some(assets) = self.world.resource_mut::<crate::asset::AssetServer>() {
                assets.watch_path(&path);
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = (name, path);
        }
    }

    /// Load a RON anim-effect binding table from `path` and register it under `name`.
    ///
    /// Lazily inserts a [`crate::anim_effect::AnimEffectRegistry`] resource if one is not yet
    /// present. On native builds the path is also registered with the `AssetServer` file watcher so
    /// disk changes are hot-reloaded. Apply the loaded table with an
    /// [`AnimEffectSystem::new(name)`](crate::AnimEffectSystem) added after
    /// [`AnimationSystem`](crate::animation::AnimationSystem).
    ///
    /// A failure (file not found, parse error) is reported as an asset failure — `error!` plus
    /// [`asset_failures`](Self::asset_failures), or a panic under
    /// [`set_strict_assets`](Self::set_strict_assets). The registry will simply not contain the
    /// table. This method is a no-op on wasm (file I/O is unsupported there; parse with
    /// [`AnimEffectBindings::from_ron_str`](crate::AnimEffectBindings::from_ron_str) and
    /// [`AnimEffectRegistry::insert`](crate::AnimEffectRegistry::insert) instead).
    pub fn load_anim_effects(&mut self, name: impl Into<String>, path: impl Into<String>) {
        use crate::anim_effect::AnimEffectRegistry;

        let name = name.into();
        let path = path.into();

        if self.world.resource::<AnimEffectRegistry>().is_none() {
            self.world.insert_resource(AnimEffectRegistry::default());
        }
        // Preserve the registry across scene Replace world resets.
        self.register_persistent::<AnimEffectRegistry>();

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(reg) = self.world.resource_mut::<AnimEffectRegistry>() {
                if let Err(e) = reg.load(name.clone(), &path) {
                    crate::asset_path::record_failure(
                        &path,
                        format!("anim-effect table '{name}': {e}"),
                    );
                    return;
                }
            }
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

/// The loud-failure contract these loaders share: a table that cannot be read or parsed is
/// reported, while one that is *legitimately* empty is not.
///
/// Both tests key their assertions on a **path unique to the test**, never on the length of
/// `asset_failures()` — that list is process-global and shared with every other test in this
/// binary (see the caveats in [`crate::asset_path`]).
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::app::App;

    /// A missing data table used to `warn!` and register nothing, so the game booted on an empty
    /// table with no way to tell that apart from a table that is empty on purpose. It is now an
    /// asset failure.
    #[test]
    fn a_missing_data_table_is_recorded_as_an_asset_failure() {
        let missing = "examples/assets/__missing_items_table__.ron";
        assert!(
            !std::path::Path::new(missing).exists(),
            "precondition: {missing} must not exist, or this test proves nothing"
        );

        let mut app = App::new();
        app.load_data_table("items", missing);

        let failure = app
            .asset_failures()
            .into_iter()
            .find(|f| f.path == missing)
            .unwrap_or_else(|| {
                panic!("a missing data table must be reported via asset_failures(), got nothing")
            });
        assert!(
            failure.error.contains("items"),
            "the failure should name the table it was loading: {}",
            failure.error
        );
        assert!(
            app.world
                .resource::<crate::data_table::DataTableRegistry>()
                .expect("registry")
                .get("items")
                .is_none(),
            "a failed load must not leave an empty table behind under the name"
        );
    }

    /// The other half of the contract: an *intentionally* empty table parses fine, so it must
    /// register (with zero rows) and must NOT be reported as a failure.
    #[test]
    fn an_intentionally_empty_data_table_is_not_a_failure() {
        // Absolute path: `resolve` passes it through, so this does not depend on the working
        // directory (which another integration test moves).
        let path = std::env::temp_dir().join("skeleton_engine_empty_table_test.ron");
        std::fs::write(&path, "[]").expect("write fixture");
        let path = path.display().to_string();

        let mut app = App::new();
        app.load_data_table("empty", &path);

        assert!(
            !app.asset_failures().iter().any(|f| f.path == path),
            "a valid, empty table must not be reported as an asset failure"
        );
        let table = app
            .world
            .resource::<crate::data_table::DataTableRegistry>()
            .expect("registry")
            .get("empty")
            .expect("an empty table still registers");
        assert!(table.rows.is_empty());

        let _ = std::fs::remove_file(&path);
    }
}
