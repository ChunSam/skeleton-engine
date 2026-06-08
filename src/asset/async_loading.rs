#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use super::image_loading::decode_image_with_state;
use super::image_loading::magenta_fallback;
use super::{alloc_id, asset_key, AssetId, AssetLoadState, AssetServer, Handle, ImageAsset};

// ─── 비동기 로드 결과 (네이티브 채널용) ───────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub(super) struct AsyncImageResult {
    id: AssetId,
    path: String,
    asset: ImageAsset,
    state: AssetLoadState,
}

// ─── WASM 비동기 큐 ───────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
thread_local! {
    #[allow(clippy::type_complexity)]
    static WASM_ASYNC_QUEUE: RefCell<std::collections::VecDeque<(AssetId, String, (ImageAsset, AssetLoadState))>>
        = const { RefCell::new(std::collections::VecDeque::new()) };
}

impl AssetServer {
    /// 이미지를 백그라운드에서 비동기로 로드한다.
    ///
    /// 즉시 마젠타 폴백 텍스처가 등록된 핸들을 반환한다. 로딩 완료 전까지
    /// `load_state()`는 `AssetLoadState::Loading`을 반환한다.
    ///
    /// - 네이티브: `std::thread::spawn` 백그라운드 스레드에서 로드
    /// - WASM: `wasm_bindgen_futures::spawn_local` + `fetch` API
    pub fn load_image_async(&mut self, path: impl AsRef<Path>) -> Handle<ImageAsset> {
        let key = asset_key(path.as_ref());
        // 캐시 확인 — 이미 로드됐거나 로딩 중이면 기존 핸들 반환
        if let Some(&id) = self.path_to_id.get(&key) {
            return Handle {
                id,
                path: key,
                _marker: PhantomData,
            };
        }
        let id = alloc_id();
        // 마젠타 폴백 + Loading 상태로 즉시 등록
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

    /// 완료된 비동기 로드 결과를 처리해 내부 캐시를 갱신한다.
    ///
    /// `App`이 매 프레임 호출한다. 완료된 에셋의 경로 목록을 반환한다.
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

    /// 현재 `AssetLoadState::Loading` 상태인 이미지 수를 반환한다.
    pub fn async_loading_count(&self) -> usize {
        self.image_load_states
            .values()
            .filter(|s| matches!(s, AssetLoadState::Loading))
            .count()
    }
}

// ─── WASM fetch 헬퍼 ─────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
async fn fetch_image_wasm(url: &str) -> (ImageAsset, AssetLoadState) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = match web_sys::window() {
        Some(w) => w,
        None => {
            let msg = format!("fetch 실패 '{url}': window 없음");
            log::error!("{msg}");
            return (magenta_fallback(), AssetLoadState::Failed(msg));
        }
    };

    let resp_value = match JsFuture::from(window.fetch_with_str(url)).await {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("fetch 실패 '{url}': {e:?}");
            log::error!("{msg}");
            return (magenta_fallback(), AssetLoadState::Failed(msg));
        }
    };

    let resp: web_sys::Response = match resp_value.dyn_into() {
        Ok(r) => r,
        Err(_) => {
            let msg = format!("fetch 응답 변환 실패 '{url}'");
            return (magenta_fallback(), AssetLoadState::Failed(msg));
        }
    };

    if !resp.ok() {
        let msg = format!("fetch HTTP 오류 '{url}': {}", resp.status());
        log::error!("{msg}");
        return (magenta_fallback(), AssetLoadState::Failed(msg));
    }

    let array_buffer_promise = match resp.array_buffer() {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("array_buffer() 실패 '{url}': {e:?}");
            return (magenta_fallback(), AssetLoadState::Failed(msg));
        }
    };

    let array_buffer = match JsFuture::from(array_buffer_promise).await {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("응답 읽기 실패 '{url}': {e:?}");
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
            let msg = format!("이미지 디코딩 실패 '{url}': {e}");
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
