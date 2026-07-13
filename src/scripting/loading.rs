use super::asset::ScriptAsset;

/// Compiles a script from the file at `path`. Handles native/wasm differences.
///
/// On native: reads the file from the filesystem.
/// On wasm: file I/O is unsupported; logs a warning and returns an empty script.
pub(super) fn compile_script_file(path: &str) -> ScriptAsset {
    #[cfg(not(target_arch = "wasm32"))]
    let source = match std::fs::read_to_string(crate::asset_path::resolve(path)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("failed to read script file '{path}': {e}");
            String::new()
        }
    };
    // wasm: no filesystem, so path-based script loading is not supported.
    // (replaces the previous silent empty-script fallback with an explicit warning)
    #[cfg(target_arch = "wasm32")]
    let source = {
        log::warn!(
            "load_script('{path}'): filesystem script loading is not supported on the wasm target"
        );
        String::new()
    };
    let engine = rhai::Engine::new();
    let ast = engine.compile(&source).unwrap_or_else(|e| {
        log::error!("script compile failed '{path}': {e}");
        engine.compile("").unwrap()
    });
    ScriptAsset {
        source,
        ast: std::sync::Arc::new(ast),
    }
}
