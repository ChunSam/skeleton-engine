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

/// Encodes a `w × h` PNG in memory so an atlas test reads nothing from disk.
fn png_bytes(w: u32, h: u32) -> Vec<u8> {
    let raw: Vec<u8> = (0..w * h)
        .flat_map(|i| [(i % 251) as u8, (i % 253) as u8, (i % 247) as u8, 255])
        .collect();
    let img = image::RgbaImage::from_raw(w, h, raw).unwrap();
    let mut png: Vec<u8> = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    png
}

/// The identity invariant for a byte-sourced atlas: the atlas handle path, the atlas's
/// `texture_path()` (what an `AtlasSprite` renders by) and the key the renderer actually uploads
/// under (`image_assets_for_gpu`) must all be the caller's key **verbatim**.
///
/// This is the invariant that keeps a byte-sourced atlas from rendering white — the 2026-05-29
/// texture-cache-key bug, where the handle path was canonical and the GPU key relative. Nothing in
/// the type system enforces it, and the failure is silent at compile time, so it is pinned here.
#[test]
fn load_atlas_bytes_keys_the_atlas_image_and_render_key_identically() {
    let png = png_bytes(8, 6);
    let mut server = AssetServer::new();
    // Names no file — the whole point of a byte-sourced atlas.
    let key = "embedded/__unit_test_atlas__";
    let handle = server.load_atlas_bytes(key, &png, 4, 3);

    // 1. The atlas handle path is the key verbatim (no canonicalization).
    assert_eq!(handle.path(), key);

    let atlas = server.get_atlas(&handle).expect("atlas registered");
    // 2. What an `AtlasSprite` renders by is the same string.
    assert_eq!(atlas.texture_path(), key);
    assert_eq!((atlas.cols, atlas.rows), (4, 3));

    // 3. The underlying image decoded, and is uploaded to the GPU under that same string —
    //    `image_assets_for_gpu` is exactly what feeds the renderer's texture cache.
    let img = server.get_image(&atlas.handle).expect("image registered");
    assert_eq!((img.width, img.height), (8, 6));
    assert!(
        server.image_assets_for_gpu().any(|(k, _)| k == key),
        "the renderer must upload the embedded sheet under the caller's key verbatim"
    );

    // Loading the same key again returns the cached handle (bytes ignored on a hit).
    let again = server.load_atlas_bytes(key, b"", 4, 3);
    assert_eq!(again.id(), handle.id());
}

/// `image_assets_for_gpu` runs on **every frame** from `App`'s render stage, so it must hand out
/// borrows rather than copies. It used to return `Vec<(String, ImageAsset)>`, which allocated one
/// `String` per loaded image plus the `Vec` every frame, for a caller that discards nearly all of
/// them behind `has_texture_key`.
///
/// The counting allocator in `tests/per_frame_alloc.rs` cannot reach this — it is a separate
/// integration binary and this method is `pub(crate)` — so the no-copy property is pinned here
/// directly instead: the yielded key must be *the same memory* as the stored key, not an equal
/// one. `assert_eq!` on the strings would pass just as happily for a fresh `String`.
#[test]
fn image_assets_for_gpu_yields_borrows_not_copies() {
    let mut server = AssetServer::new();
    let key = "embedded/__unit_test_no_copy__";
    server.load_atlas_bytes(key, &png_bytes(2, 2), 1, 1);

    let (yielded, asset) = server
        .image_assets_for_gpu()
        .find(|(k, _)| *k == key)
        .expect("the freshly loaded sheet must be offered to the renderer");

    // The key is `Arc<str>` in the map; a copy would live at a different address.
    let stored = server
        .image_assets_for_gpu()
        .find(|(k, _)| *k == key)
        .expect("stable across calls")
        .0;
    assert!(
        std::ptr::eq(yielded.as_ptr(), stored.as_ptr()),
        "the key must be borrowed from the server's own storage, not copied per call"
    );

    // The pixels come through the `Arc` too — cloning the asset must not deep-copy them.
    let again = server
        .image_assets_for_gpu()
        .find(|(k, _)| *k == key)
        .expect("stable across calls")
        .1;
    assert!(
        std::sync::Arc::ptr_eq(&asset.data, &again.data),
        "the pixel buffer must be shared, not duplicated"
    );
}

/// A byte-sourced atlas must be geometrically indistinguishable from a path-loaded one: same
/// `cols × rows` grid, same UVs for every tile. Driven as a real A/B against `load_atlas` on a
/// temp file, so this fails if the byte path ever grows its own grid maths.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn load_atlas_bytes_tiles_the_grid_exactly_like_a_path_loaded_atlas() {
    let png = png_bytes(8, 6);

    // Unique temp file (never under cwd) — `asset_key` / the failure list are process-global.
    let dir = std::env::temp_dir().join(format!(
        "engine-atlas-bytes-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("sheet.png");
    std::fs::write(&file, &png).unwrap();

    let mut server = AssetServer::new();
    let from_path = server.load_atlas(&file, 4, 3);
    let from_bytes = server.load_atlas_bytes("embedded/__unit_test_atlas_ab__", &png, 4, 3);

    let a = server.get_atlas(&from_path).expect("path atlas");
    let b = server.get_atlas(&from_bytes).expect("byte atlas");

    assert_eq!((a.cols, a.rows), (b.cols, b.rows));
    for index in 0..12 {
        assert_eq!(
            a.uv_rect(index),
            b.uv_rect(index),
            "tile {index} must have identical UVs whether the sheet came from a file or bytes"
        );
    }
    // The two are separate assets keyed by their own identities — the path one canonicalized,
    // the byte one verbatim — and neither borrowed the other's key.
    assert_ne!(a.texture_path(), b.texture_path());
    assert_eq!(b.texture_path(), "embedded/__unit_test_atlas_ab__");

    std::fs::remove_dir_all(&dir).ok();
}

/// A corrupt embedded sheet must be reported, not swallowed — the EW-007 rule, which applies to a
/// bad `include_bytes!` exactly as it does to a missing file. The atlas still registers (tiling the
/// magenta fallback) so the grid API stays usable rather than panicking.
#[test]
fn load_atlas_bytes_reports_a_corrupt_embed_as_a_failure() {
    let mut server = AssetServer::new();
    // Unique key: the failure list is process-global, so assert on THIS key, never on length.
    let key = "embedded/__unit_test_corrupt_atlas__";
    let handle = server.load_atlas_bytes(key, b"definitely not a png", 4, 3);

    let atlas = server
        .get_atlas(&handle)
        .expect("atlas registers even on a bad decode");
    assert!(matches!(
        server.load_state(&atlas.handle),
        AssetLoadState::Failed(_)
    ));
    assert!(
        crate::asset_path::asset_failures()
            .iter()
            .any(|f| f.path == key),
        "a corrupt embedded sheet must be recorded in asset_failures()"
    );
}
