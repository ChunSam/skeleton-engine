#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use notify::{RecursiveMode, Watcher};

#[cfg(not(target_arch = "wasm32"))]
use super::asset_key;
#[cfg(not(target_arch = "wasm32"))]
use super::image_loading::decode_image_with_state;
use super::AssetServer;

impl AssetServer {
    /// Returns a list of changed file paths and refreshes the internal CPU cache.
    ///
    /// `App` calls this every frame and re-uploads GPU textures for the returned paths.
    /// Paths registered via `watch_path` (or the typed delegates) are also included in
    /// the returned `Vec` so that `App` can forward them to registered
    /// [`crate::asset::HotReloadable`] registries.
    ///
    /// **Platform note:** hot reloading is native-only. On `wasm32` targets this method
    /// always returns an empty `Vec` (no-op on wasm32).
    pub fn poll_reloads(&mut self) -> Vec<String> {
        #[cfg(target_arch = "wasm32")]
        {
            Vec::new()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let rx = match &self.reload_rx {
                Some(r) => r,
                None => return Vec::new(),
            };
            let mut seen: Vec<String> = Vec::new();
            while let Ok(path) = rx.try_recv() {
                // The watcher fires on the *resolved* file; translate it back to the *logical*
                // key the caches and registries stored, so a change under an asset root
                // (packaged / foreign-cwd layout) dispatches under the key they can match.
                let key = self.logical_for_changed(&path);
                let key_str = key.to_string();
                let is_known = self.path_to_id.contains_key(&key)
                    || self.watched_paths.contains(&key)
                    || self.atlas_path_to_id.contains_key(&key);
                if is_known && !seen.contains(&key_str) {
                    seen.push(key_str);
                }
            }
            for path_str in &seen {
                let key: Arc<str> = path_str.as_str().into();
                if let Some(&id) = self.path_to_id.get(&key) {
                    let (asset, state) = decode_image_with_state(path_str);
                    self.images.insert(id, asset);
                    self.image_load_states.insert(id, state);
                }
                // Non-image paths are returned as-is; registries (ScriptRegistry,
                // DataTableRegistry, etc.) reload them via the HotReloadable forwarder
                // in schedule.rs.
            }
            seen
        }
    }

    /// Translate a path the OS watcher reported (the *resolved* file it fired on) back to the
    /// *logical* dispatch key (`asset_key(logical)`) that the image cache, `watched_paths`, and the
    /// RON registries stored.
    ///
    /// Falls back to `asset_key(changed)` — the historical behavior — when the path was not
    /// registered through the resolved→logical map. That covers the dev-from-repo-root case (where
    /// `resolve(logical)` canonicalizes to the same path as `asset_key(logical)`, so the map entry
    /// is an identity anyway) and any absolute-path caller, keeping their dispatch byte-identical.
    ///
    /// Pure over `&self` — no watcher, so it is directly unit-testable (see `asset/tests.rs`).
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn logical_for_changed(&self, changed: &std::path::Path) -> Arc<str> {
        let key = asset_key(changed);
        self.watched_resolved_to_logical
            .get(&key)
            .cloned()
            .unwrap_or(key)
    }

    /// Register a file path with the filesystem watcher so that edits on disk are
    /// included in the next `poll_reloads` result.
    ///
    /// This is the canonical, type-agnostic entry point. The typed helpers
    /// `watch_data_table_path`, `watch_animation_clip_path`, and
    /// `watch_particle_config_path` are thin delegates to this method and are kept
    /// for backwards compatibility.
    ///
    /// Idempotent: calling with the same path a second time is a no-op.
    ///
    /// Native-only. Excluded from wasm builds.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn watch_path(&mut self, path: impl Into<String>) {
        let path_str = path.into();
        let key = asset_key(std::path::Path::new(&path_str));
        if self.watched_paths.contains(&key) {
            return;
        }
        // Watch the file that actually exists. `resolve` finds it under the asset root, which in a
        // packaged / foreign-cwd layout is NOT the logical path — and `notify` cannot watch a
        // nonexistent path, so watching the logical path verbatim silently failed to register there.
        let resolved = crate::asset_path::resolve(&path_str);
        if let Some(ref mut w) = self._watcher {
            let _ = w.watch(&resolved, RecursiveMode::NonRecursive);
        }
        // Map the key the watcher will fire on (the resolved file) back to the logical dispatch key,
        // so `poll_reloads` reloads under the key the registries stored. Identity when they coincide.
        self.watched_resolved_to_logical
            .insert(asset_key(&resolved), Arc::clone(&key));
        self.watched_paths.insert(key);
    }

    /// Register a data-table file path with the filesystem watcher so that edits
    /// on disk are included in the next `poll_reloads` result.
    ///
    /// Idempotent: calling with the same path a second time is a no-op.
    ///
    /// Native-only. Excluded from wasm builds.
    ///
    /// Delegates to [`Self::watch_path`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn watch_data_table_path(&mut self, path: impl Into<String>) {
        self.watch_path(path);
    }

    /// Register an animation-clip file path with the filesystem watcher so that edits
    /// on disk are included in the next `poll_reloads` result.
    ///
    /// Idempotent: calling with the same path a second time is a no-op.
    ///
    /// Native-only. Excluded from wasm builds.
    ///
    /// Delegates to [`Self::watch_path`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn watch_animation_clip_path(&mut self, path: impl Into<String>) {
        self.watch_path(path);
    }

    /// Register a particle-config file path with the filesystem watcher so that
    /// edits on disk are included in the next `poll_reloads` result.
    ///
    /// Idempotent: calling with the same path a second time is a no-op.
    ///
    /// Native-only. Excluded from wasm builds.
    ///
    /// Delegates to [`Self::watch_path`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn watch_particle_config_path(&mut self, path: impl Into<String>) {
        self.watch_path(path);
    }
}
