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
    /// 변경된 파일 경로 목록을 반환하고 내부 CPU 캐시를 갱신한다.
    ///
    /// `App`이 매 프레임 이를 호출하고, 반환된 경로들에 대해 GPU 텍스처를 재업로드한다.
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
