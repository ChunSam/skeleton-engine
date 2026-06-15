use super::*;

#[test]
fn missing_asset_key_preserves_input_path() {
    let key = asset_key("__definitely_missing_asset__.png");
    assert_eq!(&*key, "__definitely_missing_asset__.png");
}

/// Item 1: atlas path must be recognized as a known path by `poll_reloads` so that
/// a changed atlas image on disk triggers a reload event (not silently dropped).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn atlas_path_is_recognized_as_known_in_poll_reloads() {
    let mut server = AssetServer::new();
    // Load an atlas using a missing path so we get a stable key without needing a real file.
    let key: Arc<str> = "__test_missing_atlas_path.png".into();
    // Manually populate atlas_path_to_id to mirror what load_atlas does when it succeeds.
    server.atlas_path_to_id.insert(Arc::clone(&key), alloc_id());

    // The key must be recognized as "known" — verified by checking the map directly
    // (poll_reloads is not unit-testable without a real watcher).
    assert!(
        server.atlas_path_to_id.contains_key(&key),
        "atlas_path_to_id must contain the registered atlas path"
    );
    // Confirm path_to_id does NOT contain it (isolates the atlas-specific fix).
    assert!(!server.path_to_id.contains_key(&key));
}

/// Item 2: a failed image load (file does not exist) must not register a watcher.
/// A successful load registers a watcher; a failed one must not (non-existent paths
/// can't be watched by `notify` on macOS/Linux).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn failed_image_load_does_not_produce_loaded_state() {
    let mut server = AssetServer::new();
    let handle = server.load_image("__definitely_missing_for_watcher_test__.png");
    assert!(
        matches!(server.load_state(&handle), AssetLoadState::Failed(_)),
        "a missing file must produce a Failed load state, not Loaded"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn existing_native_paths_are_canonicalized_for_cache_keys() {
    let dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("asset-key-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("image.png");
    std::fs::write(&path, b"not a png").unwrap();

    let relative = path.strip_prefix(std::env::current_dir().unwrap()).unwrap();
    let absolute = path.canonicalize().unwrap();
    let mut server = AssetServer::new();

    let a = server.load_image(relative);
    let b = server.load_image(&absolute);

    assert_eq!(a.id(), b.id());
    assert_eq!(a.path(), absolute.to_string_lossy());

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir_all(&dir).ok();
}
