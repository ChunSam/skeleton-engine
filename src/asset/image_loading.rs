use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use notify::{RecursiveMode, Watcher};

#[cfg(target_arch = "wasm32")]
use super::async_loading::spawn_image_fetch_wasm;
use super::{
    alloc_id, asset_key, AssetId, AssetLoadState, AssetServer, Handle, ImageAsset, ImageEntry,
};

impl AssetServer {
    /// Loads an image and returns a handle. Returns the cached handle if the same path is loaded again.
    ///
    /// On failure a magenta (1×1) fallback texture is used; check the result via `load_state()`.
    pub fn load_image(&mut self, path: impl AsRef<Path>) -> Handle<ImageAsset> {
        let key = asset_key(path.as_ref());
        if let Some(&id) = self.path_to_id.get(&key) {
            return Handle {
                id,
                path: key,
                _marker: PhantomData,
            };
        }
        let id = alloc_id();
        self.path_to_id.insert(Arc::clone(&key), id);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let (asset, state) = decode_image_with_state(&key);
            self.images.insert(id, asset);
            self.image_load_states.insert(id, state);
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.images.insert(id, magenta_fallback());
            self.image_load_states.insert(id, AssetLoadState::Loading);
            spawn_image_fetch_wasm(id, key.to_string());
        }

        // Only watch paths that loaded successfully — `notify` cannot watch non-existent
        // paths on macOS/Linux, and silently failing to register would mean a later
        // file-create never triggers a reload. Skip the watch on failure; the image
        // can be re-requested explicitly when the file is known to exist.
        #[cfg(not(target_arch = "wasm32"))]
        if matches!(
            self.image_load_states.get(&id),
            Some(AssetLoadState::Loaded)
        ) {
            if let Some(ref mut w) = self._watcher {
                let _ = w.watch(path.as_ref(), RecursiveMode::NonRecursive);
            }
        }
        Handle {
            id,
            path: key,
            _marker: PhantomData,
        }
    }

    /// Returns the load state for a handle.
    ///
    /// Returns `AssetLoadState::Failed` for an unknown handle.
    pub fn load_state(&self, handle: &Handle<ImageAsset>) -> AssetLoadState {
        self.image_load_states
            .get(&handle.id)
            .cloned()
            .unwrap_or_else(|| AssetLoadState::Failed("unknown handle".into()))
    }

    /// Returns a list of image handles that failed to load (for debugging).
    pub fn failed_images(&self) -> Vec<AssetId> {
        self.image_load_states
            .iter()
            .filter_map(|(&id, state)| matches!(state, AssetLoadState::Failed(_)).then_some(id))
            .collect()
    }

    /// Returns the number of currently cached image assets.
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    /// Returns the CPU-side image data.
    pub fn get_image(&self, handle: &Handle<ImageAsset>) -> Option<&ImageAsset> {
        self.images.get(&handle.id)
    }

    /// Looks up an image asset directly by id (for asset browsers).
    pub fn get_image_by_id(&self, id: AssetId) -> Option<&ImageAsset> {
        self.images.get(&id)
    }

    /// Returns the list of currently loaded image assets (for asset browsers).
    pub fn image_list(&self) -> Vec<ImageEntry> {
        self.path_to_id
            .iter()
            .filter_map(|(path, &id)| {
                self.images.get(&id).map(|img| ImageEntry {
                    path: path.to_string(),
                    id,
                    width: img.width,
                    height: img.height,
                })
            })
            .collect()
    }

    /// CPU-side images paired with their cache keys for lazy GPU upload.
    pub(crate) fn image_assets_for_gpu(&self) -> Vec<(String, ImageAsset)> {
        self.path_to_id
            .iter()
            .filter_map(|(path, &id)| {
                self.images
                    .get(&id)
                    .map(|img| (path.to_string(), img.clone()))
            })
            .collect()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn decode_image_with_state(path: &str) -> (ImageAsset, AssetLoadState) {
    // Resolved only for the read; `path` remains the asset's key and handle path.
    match std::fs::read(crate::asset_path::resolve(path)) {
        Ok(bytes) => match image::load_from_memory(&bytes) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                (
                    ImageAsset {
                        data: Arc::new(rgba.into_raw()),
                        width: w,
                        height: h,
                    },
                    AssetLoadState::Loaded,
                )
            }
            Err(e) => {
                let msg = format!("image decode failed '{path}': {e}");
                crate::asset_path::record_failure(path, format!("image decode failed: {e}"));
                (magenta_fallback(), AssetLoadState::Failed(msg))
            }
        },
        Err(e) => {
            let msg = format!("image file read failed '{path}': {e}");
            crate::asset_path::record_failure(path, format!("image file read failed: {e}"));
            (magenta_fallback(), AssetLoadState::Failed(msg))
        }
    }
}

pub(super) fn magenta_fallback() -> ImageAsset {
    ImageAsset {
        data: Arc::new(vec![255, 0, 255, 255]),
        width: 1,
        height: 1,
    }
}
