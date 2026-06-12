#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use super::asset_key;
#[cfg(not(target_arch = "wasm32"))]
use super::image_loading::decode_image_with_state;
#[cfg(not(target_arch = "wasm32"))]
use super::script_loading::compile_script_file;
use super::AssetServer;

impl AssetServer {
    /// Returns a list of changed file paths and refreshes the internal CPU cache.
    ///
    /// `App` calls this every frame and re-uploads GPU textures for the returned paths.
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
                let key = asset_key(&path);
                let key_str = key.to_string();
                let is_known =
                    self.path_to_id.contains_key(&key) || self.script_path_to_id.contains_key(&key);
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
                if let Some(&id) = self.script_path_to_id.get(&key) {
                    self.scripts.insert(id, compile_script_file(path_str));
                }
            }
            seen
        }
    }
}
