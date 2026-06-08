#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use super::image_loading::decode_image_with_state;
use super::image_loading::magenta_fallback;
use super::{alloc_id, asset_key, AssetId, AssetLoadState, AssetServer, Handle, ImageAsset};

// ─── Async Load Result (native channel) ──────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub(super) struct AsyncImageResult {
    id: AssetId,
    path: String,
    asset: ImageAsset,
    state: AssetLoadState,
}

// ─── WASM Async Queue ─────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
thread_local! {
    #[allow(clippy::type_complexity)]
    static WASM_ASYNC_QUEUE: RefCell<std::collections::VecDeque<(AssetId, String, (ImageAsset, AssetLoadState))>>
        = const { RefCell::new(std::collections::VecDeque::new()) };
}

impl AssetServer {
    /// Loads an image asynchronously in the background.
    ///
    /// Returns a handle immediately registered with a magenta fallback texture.
    /// `load_state()` returns `AssetLoadState::Loading` until the load completes.
    ///
    /// - Native: loads on a background thread via `std::thread::spawn`
    /// - WASM: `wasm_bindgen_futures::spawn_local` + `fetch` API
    pub fn load_image_async(&mut self, path: impl AsRef<Path>) -> Handle<ImageAsset> {
        let key = asset_key(path.as_ref());
        // cache check — return existing handle if already loaded or loading
        if let Some(&id) = self.path_to_id.get(&key) {
            return Handle {
                id,
                path: key,
                _marker: PhantomData,
            };
        }
        let id = alloc_id();
        // register immediately with magenta fallback + Loading state
        self.images.insert(id, magenta_fallback());
        self.image_load_states.insert(id, AssetLoadState::Loading);
        self.path_to_id.insert(Arc::clone(&key), id);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let tx = self.async_tx.clone();
            let path_str = key.to_string();
            std::thread::spawn(move || {
                let (asset, state) = decode_image_with_state(&path_str);
                let _ = tx.send(AsyncImageResult {
                    id,
                    path: path_str,
                    asset,
                    state,
                });
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            spawn_image_fetch_wasm(id, key.to_string());
        }

        Handle {
            id,
            path: key,
            _marker: PhantomData,
        }
    }

    /// Processes completed async load results and updates the internal cache.
    ///
    /// Called by `App` every frame. Returns a list of paths for completed assets.
    pub(crate) fn poll_async_completions(&mut self) -> Vec<(String, ImageAsset)> {
        let mut completed = Vec::new();

        #[cfg(not(target_arch = "wasm32"))]
        while let Ok(result) = self.async_rx.try_recv() {
            let asset = result.asset;
            self.images.insert(result.id, asset.clone());
            self.image_load_states.insert(result.id, result.state);
            completed.push((result.path, asset));
        }

        #[cfg(target_arch = "wasm32")]
        WASM_ASYNC_QUEUE.with(|q| {
            while let Some((id, path, (asset, state))) = q.borrow_mut().pop_front() {
                self.images.insert(id, asset.clone());
                self.image_load_states.insert(id, state);
                completed.push((path, asset));
            }
        });

        completed
    }

    /// Returns the number of images currently in `AssetLoadState::Loading` state.
    pub fn async_loading_count(&self) -> usize {
        self.image_load_states
            .values()
            .filter(|s| matches!(s, AssetLoadState::Loading))
            .count()
    }
}

// ─── WASM Fetch Helper ────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
async fn fetch_image_wasm(url: &str) -> (ImageAsset, AssetLoadState) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = match web_sys::window() {
        Some(w) => w,
        None => {
            let msg = format!("fetch failed '{url}': no window");
            log::error!("{msg}");
            return (magenta_fallback(), AssetLoadState::Failed(msg));
        }
    };

    let resp_value = match JsFuture::from(window.fetch_with_str(url)).await {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("fetch failed '{url}': {e:?}");
            log::error!("{msg}");
            return (magenta_fallback(), AssetLoadState::Failed(msg));
        }
    };

    let resp: web_sys::Response = match resp_value.dyn_into() {
        Ok(r) => r,
        Err(_) => {
            let msg = format!("fetch response cast failed '{url}'");
            return (magenta_fallback(), AssetLoadState::Failed(msg));
        }
    };

    if !resp.ok() {
        let msg = format!("fetch HTTP error '{url}': {}", resp.status());
        log::error!("{msg}");
        return (magenta_fallback(), AssetLoadState::Failed(msg));
    }

    let array_buffer_promise = match resp.array_buffer() {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("array_buffer() failed '{url}': {e:?}");
            return (magenta_fallback(), AssetLoadState::Failed(msg));
        }
    };

    let array_buffer = match JsFuture::from(array_buffer_promise).await {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("response read failed '{url}': {e:?}");
            return (magenta_fallback(), AssetLoadState::Failed(msg));
        }
    };

    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    let bytes = uint8_array.to_vec();

    match image::load_from_memory(&bytes) {
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
            let msg = format!("image decode failed '{url}': {e}");
            log::error!("{msg}");
            (magenta_fallback(), AssetLoadState::Failed(msg))
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn spawn_image_fetch_wasm(id: AssetId, path: String) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = fetch_image_wasm(&path).await;
        WASM_ASYNC_QUEUE.with(|q| q.borrow_mut().push_back((id, path, result)));
    });
}
