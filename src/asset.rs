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

pub type AssetId = u64;

static NEXT_ASSET_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn alloc_id() -> AssetId {
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
    pub(crate) _marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    pub fn id(&self) -> AssetId {
        self.id
    }

    /// The file path this handle points to.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns a clone of the internal `Arc<str>` — an O(1) reference-count bump.
    ///
    /// Prefer this over `Arc::from(handle.path())` wherever an owned `Arc<str>` is needed
    /// (e.g. per-sprite texture keys in the renderer). The latter copies the string bytes;
    /// this only increments a counter.
    pub fn path_arc(&self) -> Arc<str> {
        Arc::clone(&self.path)
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

// ─── AssetLoadError ───────────────────────────────────────────────────────────

/// Shared error type for data-driven asset loaders (animation clip sets, particle
/// configs, and similar RON-based registries).
///
/// Both [`crate::animation::ClipSetError`] and
/// [`crate::particle::ParticleConfigError`] are type aliases of this type, so
/// match arms and `From` impls written against either alias also work with the
/// other — a fork that adds a third RON-loader can reuse the same pattern.
///
/// The `Io` variant is native-only (`wasm32` has no file system).
#[derive(Debug)]
pub enum AssetLoadError {
    /// RON parse or deserialization error.
    Ron(String),
    /// Filesystem I/O error (native only).
    #[cfg(not(target_arch = "wasm32"))]
    Io(std::io::Error),
}

impl std::fmt::Display for AssetLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetLoadError::Ron(msg) => write!(f, "RON error: {msg}"),
            #[cfg(not(target_arch = "wasm32"))]
            AssetLoadError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for AssetLoadError {}

#[cfg(not(target_arch = "wasm32"))]
impl From<std::io::Error> for AssetLoadError {
    fn from(e: std::io::Error) -> Self {
        AssetLoadError::Io(e)
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

/// Image entry info displayed in the asset browser.
#[derive(Debug, Clone)]
pub struct ImageEntry {
    pub path: String,
    pub id: AssetId,
    pub width: u32,
    pub height: u32,
}

// ─── HotReloadable ───────────────────────────────────────────────────────────

/// A resource that can hot-reload itself from a changed asset path.
///
/// Register concrete implementors with [`crate::app::App::register_hot_reloadable`] so the
/// engine forwards every changed path to them each frame (native only).
///
/// # Example — registering a custom registry
/// ```rust,no_run
/// # use engine::{App, asset::HotReloadable};
/// struct MyRegistry { /* ... */ }
/// impl HotReloadable for MyRegistry {
///     fn reload_path(&mut self, path: &str) {
///         // check if `path` matches a tracked asset and reload
///     }
/// }
///
/// let mut app = App::new();
/// // Insert the registry as a resource first so the engine can find it.
/// // app.world.insert_resource(MyRegistry { ... });
/// #[cfg(not(target_arch = "wasm32"))]
/// app.register_hot_reloadable::<MyRegistry>();
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub trait HotReloadable: 'static {
    /// Called with each changed asset path. The implementor decides whether the path
    /// matches one of its tracked assets and reloads if so.
    fn reload_path(&mut self, path: &str);
}

// ─── AssetLoadState ───────────────────────────────────────────────────────────

/// Asset load result state. Query via `AssetServer::load_state()`.
#[derive(Debug, Clone, PartialEq)]
pub enum AssetLoadState {
    /// Async load in progress. A magenta fallback texture is displayed.
    Loading,
    /// Successfully loaded.
    Loaded,
    /// Load failed (file not found, decode error, etc.). Replaced by the magenta fallback texture.
    Failed(String),
}

// ─── AssetServer ──────────────────────────────────────────────────────────────

/// Asset manager — image loading, caching, and hot reloading.
///
/// Insert as a Resource into the ECS World, or access indirectly via `App::load_image`.
/// On native builds, existing file paths are canonicalized and used as cache keys.
/// Non-existent paths are preserved as-is to maintain legacy fallback behavior.
/// On WASM, paths are never canonicalized to preserve URL/relative-path semantics.
///
/// # Hot reloading
/// When a file changes, `poll_reloads()` returns the list of changed paths.
/// `App` calls this every frame and re-uploads the affected GPU textures.
/// Hot reloading is native-only; `poll_reloads()` is a no-op on `wasm32`.
///
/// # Example
/// ```rust,no_run
/// # use engine::App;
/// let mut app = App::new();
/// let handle = app.load_image("assets/player.png");
/// // Check for load failure
/// # use engine::asset::AssetLoadState;
/// # let assets = app.world.resource::<engine::AssetServer>().unwrap();
/// // if assets.load_state(&handle) == AssetLoadState::Failed { ... }
/// ```
pub struct AssetServer {
    images: HashMap<AssetId, ImageAsset>,
    image_load_states: HashMap<AssetId, AssetLoadState>,
    path_to_id: HashMap<Arc<str>, AssetId>,
    atlases: HashMap<AssetId, crate::atlas::TextureAtlas>,
    atlas_path_to_id: HashMap<Arc<str>, AssetId>,
    #[cfg(not(target_arch = "wasm32"))]
    reload_rx: Option<Receiver<PathBuf>>,
    #[cfg(not(target_arch = "wasm32"))]
    _watcher: Option<RecommendedWatcher>,
    /// Unified set of canonical paths registered for hot-reload watching (native only).
    /// Covers data-table, animation-clip, and particle-config paths. Used by `poll_reloads`
    /// to include non-image asset changes in the returned Vec.
    #[cfg(not(target_arch = "wasm32"))]
    watched_paths: std::collections::HashSet<std::sync::Arc<str>>,
    // Channel for async loading (native only)
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
            let (watcher, reload_rx) = match watcher_result {
                Ok(w) => (Some(w), Some(rx)),
                Err(e) => {
                    log::warn!("file watcher initialization failed (hot reloading disabled): {e}");
                    (None, None)
                }
            };
            Self {
                images: HashMap::new(),
                image_load_states: HashMap::new(),
                path_to_id: HashMap::new(),
                atlases: HashMap::new(),
                atlas_path_to_id: HashMap::new(),
                reload_rx,
                _watcher: watcher,
                watched_paths: std::collections::HashSet::new(),
                async_tx,
                async_rx,
            }
        }
        #[cfg(target_arch = "wasm32")]
        Self {
            images: HashMap::new(),
            image_load_states: HashMap::new(),
            path_to_id: HashMap::new(),
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
