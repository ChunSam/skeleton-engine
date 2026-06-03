use super::*;

#[test]
fn missing_asset_key_preserves_input_path() {
    let key = asset_key("__definitely_missing_asset__.png");
    assert_eq!(&*key, "__definitely_missing_asset__.png");
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
