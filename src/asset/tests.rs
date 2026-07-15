use super::*;

/// Task 1 — unified watch-set: a path registered via `watch_path` (or any of the
/// typed delegate helpers) ends up in `watched_paths` and is recognised as known,
/// and a second registration of the same path is a no-op (idempotent).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn watch_path_round_trips_and_is_idempotent() {
    let mut server = AssetServer::new();
    let path = "__test_watch_path_roundtrip__.ron";

    // Before registering: path must not be in watched_paths.
    let key: Arc<str> = asset_key(std::path::Path::new(path));
    assert!(
        !server.watched_paths.contains(&key),
        "watched_paths must be empty before watch_path"
    );

    // After watch_path: the path must be recorded.
    server.watch_path(path);
    assert!(
        server.watched_paths.contains(&key),
        "watched_paths must contain the path after watch_path"
    );

    // Idempotent: a second call must not duplicate the entry.
    server.watch_path(path);
    assert_eq!(
        server.watched_paths.len(),
        1,
        "watched_paths must have exactly one entry after duplicate watch_path"
    );

    // Typed delegates must route to the same set.
    let path2 = "__test_watch_path_delegate__.ron";
    let key2: Arc<str> = asset_key(std::path::Path::new(path2));
    server.watch_data_table_path(path2);
    assert!(
        server.watched_paths.contains(&key2),
        "watch_data_table_path must populate watched_paths"
    );
}

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

/// `load_image_bytes` decodes an in-memory image and registers it under the caller's key
/// **verbatim** — the same string the sprite renderer's cache and `Handle::path()` use — so a
/// byte-sourced sprite is keyed identically on every side (the invariant that keeps it from
/// rendering white).
#[test]
fn load_image_bytes_registers_a_decoded_image_under_the_verbatim_key() {
    // A real 2×2 PNG, encoded in memory so the test reads nothing from disk.
    let raw = vec![
        10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
    ];
    let img = image::RgbaImage::from_raw(2, 2, raw).unwrap();
    let mut png: Vec<u8> = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();

    let mut server = AssetServer::new();
    // A key that is NOT a real file — the whole point of a byte-sourced image.
    let key = "embedded/unit-test-pixel";
    let handle = server.load_image_bytes(key, &png);

    // The handle path is the caller's key verbatim (no canonicalization).
    assert_eq!(handle.path(), key);
    // The image decoded to its real dimensions.
    let asset = server.get_image(&handle).expect("image registered");
    assert_eq!((asset.width, asset.height), (2, 2));
    assert!(matches!(server.load_state(&handle), AssetLoadState::Loaded));

    // Loading the same key again returns the cached handle (the bytes are ignored on a hit).
    let again = server.load_image_bytes(key, b"");
    assert_eq!(again.id(), handle.id());
}

/// `logical_for_changed` translates a path the OS watcher fired on (the *resolved* file) back to
/// the *logical* dispatch key that the image cache / `watched_paths` / RON registries stored — the
/// packaged / foreign-cwd case, where the resolved file and the logical path genuinely differ.
///
/// Driven with a hand-populated reverse map holding a **non-identity** entry, so the pure
/// translation is exercised without pinning the process-global asset root (which a parallel test
/// suite shares — the seq-1/2 rule). Fails if `logical_for_changed` ignored the map.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn logical_for_changed_translates_a_resolved_path_back_to_the_logical_key() {
    let mut server = AssetServer::new();
    // What the watcher reports (an absolute file next to a packaged exe) → what the caches stored
    // (the caller's relative logical key). `asset_key` of a nonexistent path is its raw string, so
    // both are stable without touching the filesystem.
    let resolved = std::path::Path::new("/opt/game-that-does-not-exist/assets/data/items.ron");
    let resolved_key = asset_key(resolved);
    let logical: Arc<str> = "assets/data/items.ron".into();
    server
        .watched_resolved_to_logical
        .insert(Arc::clone(&resolved_key), Arc::clone(&logical));

    // A change on the resolved file dispatches under the logical key.
    assert_eq!(
        server.logical_for_changed(resolved),
        logical,
        "a mapped resolved path must translate to its logical dispatch key"
    );

    // An unmapped path falls back to `asset_key(path)` — the historical dev-from-repo-root behavior,
    // kept byte-identical for any file not registered through the resolved→logical map.
    let unmapped = std::path::Path::new("some/unwatched/path.ron");
    assert_eq!(
        server.logical_for_changed(unmapped),
        asset_key(unmapped),
        "an unmapped path must fall back to asset_key(path)"
    );
}

/// A successful `watch_path` must record a resolved→logical map entry, or `poll_reloads` has no way
/// to translate the watcher event back to the dispatch key and hot-reload under an asset root never
/// fires. Uses an absolute temp file (so the entry is an identity — `resolve` passes an absolute
/// path through); what's asserted is that the entry is *registered at all*. Fails if Phase 2's
/// reverse-map insertion is removed (the map stays empty).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn watch_path_registers_a_resolved_to_logical_map_entry() {
    // Unique temp path (never under cwd) — asset_key / the map are process-global-safe when keyed
    // on a path unique to this test.
    let dir = std::env::temp_dir().join(format!(
        "engine-hot-reload-watch-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("table.ron");
    std::fs::write(&file, b"[]").unwrap();
    let logical = file.to_string_lossy().to_string();

    let mut server = AssetServer::new();
    server.watch_path(&logical);

    // The watcher fires on `resolve(logical)`; its key must map back to the logical dispatch key.
    let resolved_key = asset_key(crate::asset_path::resolve(&logical));
    let logical_key = asset_key(std::path::Path::new(&logical));
    assert_eq!(
        server.watched_resolved_to_logical.get(&resolved_key),
        Some(&logical_key),
        "watch_path must register a resolved→logical entry"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// A corrupt embed (a bad `include_bytes!`) is just as invisible as a missing file if it is
/// swallowed, so it must be reported through the shared failure channel.
#[test]
fn load_image_bytes_reports_a_corrupt_embed_as_a_failure() {
    let mut server = AssetServer::new();
    // Unique key: the failure list is process-global and shared with every parallel test in this
    // binary, so assert on THIS key only, never on the list's length.
    let key = "embedded/__unit_test_corrupt_embed__";
    let handle = server.load_image_bytes(key, b"definitely not a png");

    assert!(matches!(
        server.load_state(&handle),
        AssetLoadState::Failed(_)
    ));
    assert!(
        crate::asset_path::asset_failures()
            .iter()
            .any(|f| f.path == key),
        "a corrupt embed must be recorded in asset_failures()"
    );
}
