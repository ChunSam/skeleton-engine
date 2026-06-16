use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::asset::{alloc_id, asset_key, AssetId, Handle};

use super::loading::compile_script_file;

// ─── ScriptAsset ─────────────────────────────────────────────────────────────

/// CPU-side Rhai script asset.
///
/// `ast` is wrapped in an `Arc`. `ScriptingSystem` clones the AST handle for every
/// script entity each frame; with `Arc` only the refcount is bumped instead of
/// deep-cloning the entire tree.
pub struct ScriptAsset {
    pub source: String,
    pub(crate) ast: Arc<rhai::AST>,
}

// ─── ScriptRegistry ──────────────────────────────────────────────────────────

/// Resource that owns all compiled script assets and manages script loading,
/// deduplication, and hot-reload.
///
/// Registered automatically in `App::new`. Access via
/// `world.resource_mut::<ScriptRegistry>()`.
///
/// # Hot reloading (native only)
/// `ScriptRegistry` implements [`crate::asset::HotReloadable`] and is registered
/// with the engine via `register_hot_reloadable`. When the file watcher detects a
/// change the engine calls `reload_path`, which recompiles any tracked script whose
/// canonical path matches.
#[derive(Default)]
pub struct ScriptRegistry {
    scripts: HashMap<AssetId, ScriptAsset>,
    script_path_to_id: HashMap<Arc<str>, AssetId>,
}

impl ScriptRegistry {
    /// Loads a script from `path`, compiles it, and returns a handle.
    /// Returns the cached handle on repeated calls with the same path.
    pub fn load_script(&mut self, path: impl AsRef<std::path::Path>) -> Handle<ScriptAsset> {
        let key = asset_key(path.as_ref());
        if let Some(&id) = self.script_path_to_id.get(&key) {
            return Handle {
                id,
                path: key,
                _marker: PhantomData,
            };
        }
        let id = alloc_id();
        let asset = compile_script_file(&key);
        self.scripts.insert(id, asset);
        self.script_path_to_id.insert(Arc::clone(&key), id);
        Handle {
            id,
            path: key,
            _marker: PhantomData,
        }
    }

    /// Looks up a script asset by id (internal use by `ScriptingSystem`).
    pub(crate) fn get_script_by_id(&self, id: AssetId) -> Option<&ScriptAsset> {
        self.scripts.get(&id)
    }

    /// Compiles an inline Rhai source string and returns a handle.
    ///
    /// Primarily intended for tests that need a fully-functional `ScriptRunner` without
    /// loading from disk. Uses a synthetic path key so it participates in the normal
    /// handle-dedup cache; repeated calls with the same `key` return the cached handle.
    #[cfg(test)]
    pub fn load_script_inline(
        &mut self,
        key: impl Into<Arc<str>>,
        source: &str,
    ) -> Handle<ScriptAsset> {
        let key: Arc<str> = key.into();
        if let Some(&id) = self.script_path_to_id.get(&key) {
            return Handle {
                id,
                path: key,
                _marker: PhantomData,
            };
        }
        let id = alloc_id();
        let engine = rhai::Engine::new();
        let ast = engine.compile(source).unwrap_or_else(|e| {
            panic!("load_script_inline: compile failed: {e}");
        });
        self.scripts.insert(
            id,
            ScriptAsset {
                source: source.to_string(),
                ast: Arc::new(ast),
            },
        );
        self.script_path_to_id.insert(Arc::clone(&key), id);
        Handle {
            id,
            path: key,
            _marker: PhantomData,
        }
    }
}

// ─── HotReloadable impl ───────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
impl crate::asset::HotReloadable for ScriptRegistry {
    /// Called with each changed asset path. If the path matches a tracked script,
    /// recompiles it in place (preserving the existing AssetId so all live handles
    /// remain valid).
    fn reload_path(&mut self, path: &str) {
        let key: Arc<str> = path.into();
        if let Some(&id) = self.script_path_to_id.get(&key) {
            self.scripts.insert(id, compile_script_file(path));
        }
    }
}
