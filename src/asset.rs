use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::channel;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::Receiver;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use notify::{EventKind, RecommendedWatcher};

mod async_loading;
mod atlas_loading;
mod hot_reload;
mod image_loading;
mod script_loading;

pub type AssetId = u64;

static NEXT_ASSET_ID: AtomicU64 = AtomicU64::new(1);

fn alloc_id() -> AssetId {
    NEXT_ASSET_ID.fetch_add(1, Ordering::Relaxed)
}

// ─── Handle<T> ────────────────────────────────────────────────────────────────

/// Typed, lightweight reference to a loaded asset.
///
/// Clone is O(1) (Arc pointer copy). Stores the canonical path so the renderer
/// can resolve the GPU texture without an extra AssetServer lookup.
pub struct Handle<T> {
    pub(crate) id: AssetId,
    pub(crate) path: Arc<str>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    pub fn id(&self) -> AssetId {
        self.id
    }

    /// 이 핸들이 가리키는 파일 경로.
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            path: Arc::clone(&self.path),
            _marker: PhantomData,
        }
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T> Eq for Handle<T> {}

impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Handle({}, {:?})", self.id, &*self.path)
    }
}

// ─── ImageAsset ───────────────────────────────────────────────────────────────

/// CPU-side decoded image (RGBA8). Cheap to clone (data behind Arc).
#[derive(Clone)]
pub struct ImageAsset {
    pub data: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

/// 에셋 브라우저에서 표시할 이미지 항목 정보.
#[derive(Debug, Clone)]
pub struct ImageEntry {
    pub path: String,
    pub id: AssetId,
    pub width: u32,
    pub height: u32,
}

// ─── ScriptAsset ─────────────────────────────────────────────────────────────

/// CPU-side Rhai 스크립트 에셋.
pub struct ScriptAsset {
    pub source: String,
    pub ast: rhai::AST,
}

// ─── AssetLoadState ───────────────────────────────────────────────────────────

/// 에셋 로드 결과 상태. `AssetServer::load_state()`로 조회한다.
#[derive(Debug, Clone, PartialEq)]
pub enum AssetLoadState {
    /// 비동기 로드 진행 중. 마젠타 폴백 텍스처가 표시된다.
    Loading,
    /// 정상 로드됨.
    Loaded,
    /// 로드 실패 (파일 없음, 디코딩 오류 등). 마젠타 폴백 텍스처로 대체된 상태.
    Failed(String),
}

// ─── AssetServer ──────────────────────────────────────────────────────────────

/// 에셋 관리자 — 이미지 로드·캐싱·핫 리로딩.
///
/// ECS World에 Resource로 삽입해 사용하거나 `App::load_image`를 통해 간접적으로 접근한다.
/// 네이티브 빌드에서는 존재하는 파일 경로를 canonical path로 정규화해 캐시 키로 사용한다.
/// 존재하지 않는 경로는 입력 문자열을 그대로 보존해 기존 fallback 동작을 유지하고, WASM에서는
/// URL/상대경로 의미를 보존하기 위해 정규화하지 않는다.
///
/// # 핫 리로딩
/// 파일이 변경되면 `poll_reloads()`가 변경된 경로 목록을 반환한다.
/// `App`이 매 프레임 이를 호출해 GPU 텍스처를 재업로드한다.
///
/// # 예시
/// ```rust,no_run
/// # use engine::App;
/// let mut app = App::new();
/// let handle = app.load_image("assets/player.png");
/// // 로드 실패 여부 확인
/// # use engine::asset::AssetLoadState;
/// # let assets = app.world.resource::<engine::AssetServer>().unwrap();
/// // if assets.load_state(&handle) == AssetLoadState::Failed { ... }
/// ```
pub struct AssetServer {
    images: HashMap<AssetId, ImageAsset>,
    image_load_states: HashMap<AssetId, AssetLoadState>,
    path_to_id: HashMap<Arc<str>, AssetId>,
    scripts: HashMap<AssetId, ScriptAsset>,
    script_path_to_id: HashMap<Arc<str>, AssetId>,
    atlases: HashMap<AssetId, crate::atlas::TextureAtlas>,
    atlas_path_to_id: HashMap<Arc<str>, AssetId>,
    #[cfg(not(target_arch = "wasm32"))]
    reload_rx: Option<Receiver<PathBuf>>,
    #[cfg(not(target_arch = "wasm32"))]
    _watcher: Option<RecommendedWatcher>,
    // 비동기 로드용 채널 (네이티브 전용)
    #[cfg(not(target_arch = "wasm32"))]
    async_tx: std::sync::mpsc::SyncSender<async_loading::AsyncImageResult>,
    #[cfg(not(target_arch = "wasm32"))]
    async_rx: std::sync::mpsc::Receiver<async_loading::AsyncImageResult>,
}

pub(crate) fn asset_key(path: impl AsRef<Path>) -> Arc<str> {
    let path = path.as_ref();
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(canonical) = path.canonicalize() {
            return canonical.to_string_lossy().as_ref().into();
        }
    }
    path.to_string_lossy().as_ref().into()
}

impl AssetServer {
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (async_tx, async_rx) =
                std::sync::mpsc::sync_channel::<async_loading::AsyncImageResult>(128);
            let (tx, rx) = channel::<PathBuf>();
            let watcher_result =
                notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                    if let Ok(event) = res {
                        match event.kind {
                            EventKind::Modify(_) | EventKind::Create(_) => {
                                for path in event.paths {
                                    let _ = tx.send(path);
                                }
                            }
                            _ => {}
                        }
                    }
                });
            match watcher_result {
                Ok(w) => Self {
                    images: HashMap::new(),
                    image_load_states: HashMap::new(),
                    path_to_id: HashMap::new(),
                    scripts: HashMap::new(),
                    script_path_to_id: HashMap::new(),
                    atlases: HashMap::new(),
                    atlas_path_to_id: HashMap::new(),
                    reload_rx: Some(rx),
                    _watcher: Some(w),
                    async_tx,
                    async_rx,
                },
                Err(e) => {
                    log::warn!("파일 감시 초기화 실패 (핫 리로딩 비활성): {e}");
                    let (async_tx2, async_rx2) =
                        std::sync::mpsc::sync_channel::<async_loading::AsyncImageResult>(128);
                    Self {
                        images: HashMap::new(),
                        image_load_states: HashMap::new(),
                        path_to_id: HashMap::new(),
                        scripts: HashMap::new(),
                        script_path_to_id: HashMap::new(),
                        atlases: HashMap::new(),
                        atlas_path_to_id: HashMap::new(),
                        reload_rx: None,
                        _watcher: None,
                        async_tx: async_tx2,
                        async_rx: async_rx2,
                    }
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        Self {
            images: HashMap::new(),
            image_load_states: HashMap::new(),
            path_to_id: HashMap::new(),
            scripts: HashMap::new(),
            script_path_to_id: HashMap::new(),
            atlases: HashMap::new(),
            atlas_path_to_id: HashMap::new(),
        }
    }
}

impl Default for AssetServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
