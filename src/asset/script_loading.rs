use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use notify::{RecursiveMode, Watcher};

use super::{alloc_id, asset_key, AssetId, AssetServer, Handle, ScriptAsset};

impl AssetServer {
    /// 스크립트를 로드해 핸들을 반환한다. 같은 경로 재호출 시 캐시된 핸들 반환.
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

    /// 스크립트 에셋을 id로 조회한다 (ScriptingSystem 내부용).
    pub fn get_script_by_id(&self, id: AssetId) -> Option<&ScriptAsset> {
        self.scripts.get(&id)
    }
}

pub(super) fn compile_script_file(path: &str) -> ScriptAsset {
    #[cfg(not(target_arch = "wasm32"))]
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("스크립트 파일 읽기 실패 '{path}': {e}");
            String::new()
        }
    };
    // wasm: 파일시스템이 없어 경로 기반 스크립트 로딩을 지원하지 않는다.
    // (조용히 빈 스크립트로 떨어지던 동작을 명시적 경고로 바꾼다)
    #[cfg(target_arch = "wasm32")]
    let source = {
        log::warn!(
            "load_script('{path}'): wasm 타깃은 파일시스템 스크립트 로딩을 지원하지 않습니다"
        );
        String::new()
    };
    let engine = rhai::Engine::new();
    let ast = engine.compile(&source).unwrap_or_else(|e| {
        log::error!("스크립트 컴파일 실패 '{path}': {e}");
        engine.compile("").unwrap()
    });
    ScriptAsset {
        source,
        ast: Arc::new(ast),
    }
}
