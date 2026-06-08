use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use notify::{RecursiveMode, Watcher};

use super::{alloc_id, asset_key, AssetId, AssetServer, Handle, ScriptAsset};

impl AssetServer {
    /// Loads a script and returns a handle. Returns the cached handle on repeated calls with the same path.
    pub fn load_script(&mut self, path: impl AsRef<Path>) -> Handle<ScriptAsset> {
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
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref mut w) = self._watcher {
            let _ = w.watch(path.as_ref(), RecursiveMode::NonRecursive);
        }
        Handle {
            id,
            path: key,
            _marker: PhantomData,
        }
    }

    /// Looks up a script asset by id (internal use by `ScriptingSystem`).
    pub fn get_script_by_id(&self, id: AssetId) -> Option<&ScriptAsset> {
        self.scripts.get(&id)
    }
}

pub(super) fn compile_script_file(path: &str) -> ScriptAsset {
    #[cfg(not(target_arch = "wasm32"))]
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("failed to read script file '{path}': {e}");
            String::new()
        }
    };
    // wasm: no filesystem, so path-based script loading is not supported.
    // (replaces the previous silent empty-script fallback with an explicit warning)
    #[cfg(target_arch = "wasm32")]
    let source = {
        log::warn!(
            "load_script('{path}'): filesystem script loading is not supported on the wasm target"
        );
        String::new()
    };
    let engine = rhai::Engine::new();
    let ast = engine.compile(&source).unwrap_or_else(|e| {
        log::error!("script compile failed '{path}': {e}");
        engine.compile("").unwrap()
    });
    ScriptAsset {
        source,
        ast: Arc::new(ast),
    }
}
