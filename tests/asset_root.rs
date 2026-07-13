//! The regression test for the magenta-screen bug: a relative asset path must resolve no matter
//! what the process working directory is.
//!
//! Before `asset_path`, `App::load_image("examples/assets/…")` read the file through `std::fs`,
//! which resolves a relative path against the **working directory**. Launch a packaged build from
//! anywhere but the repo root and every texture load failed, the renderer substituted its magenta
//! 1×1 fallback, and the window turned solid magenta.
//!
//! This test reproduces exactly that launch: it moves the working directory somewhere unrelated
//! and then loads an asset by a relative path. It fails on the old behavior and passes on the new.
//!
//! **This is deliberately the only test in this file.** The working directory is process-global,
//! and cargo runs tests within one binary in parallel — but each integration-test *file* is its own
//! process, so a single test here can move the working directory without racing anything. Do not
//! add a second test to this file.
//!
//! No GPU is needed: the `AssetServer` decodes on the CPU, so the failure is recorded (or not)
//! without ever reaching the renderer — which is what lets this run on the plain CI test job.

use engine::App;

/// Loaded by a relative path, and known to exist in the repo.
const RELATIVE_ASSET: &str = "examples/assets/hex_tiles.png";

#[test]
fn a_relative_asset_resolves_from_a_foreign_working_directory() {
    let repo_root = std::env::current_dir().expect("cwd");
    assert!(
        repo_root.join(RELATIVE_ASSET).exists(),
        "precondition: {RELATIVE_ASSET} must exist under the repo root"
    );

    // The launch that used to paint the window magenta.
    let elsewhere = std::env::temp_dir();
    std::env::set_current_dir(&elsewhere).expect("set cwd");
    assert!(
        !elsewhere.join(RELATIVE_ASSET).exists(),
        "precondition: the asset must NOT be reachable relative to the new cwd, \
         or this test proves nothing"
    );

    let mut app = App::new();
    let _handle = app.load_image(RELATIVE_ASSET);
    let failures = app.asset_failures();

    // Restore before asserting, so a failure doesn't leave the cwd moved for the harness.
    std::env::set_current_dir(&repo_root).expect("restore cwd");

    assert!(
        !failures.iter().any(|f| f.path == RELATIVE_ASSET),
        "'{RELATIVE_ASSET}' failed to load from working dir {}: {:?}\n\
         The engine must resolve relative asset paths against its own asset root \
         (executable dir + ancestors), not the working directory.",
        elsewhere.display(),
        failures
    );
}
