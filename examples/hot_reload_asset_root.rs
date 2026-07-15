//! Proof that F2 hot-reload fires for an asset whose file is **resolved under an asset root that is
//! not the working directory** — the packaged / foreign-cwd layout. This is the one EW-008
//! acceptance clause the engine did not meet before: the `notify` watcher used to register the
//! caller's *logical* path (which does not exist relative to a packaged build's cwd), so the watch
//! silently failed and an edit never triggered a reload.
//!
//! Not a windowed example. It drives the two objects the engine wires together for hot-reload —
//! [`AssetServer`] (the filesystem watcher + `poll_reloads`) and [`DataTableRegistry`]
//! (`reload_path`) — exactly as `src/app/schedule.rs` does, minus the GPU. It:
//!
//!   1. writes a data table under a temp asset root, addressed by a LOGICAL relative path that does
//!      not exist relative to the working directory,
//!   2. pins that root with `set_asset_root` (so `resolve(logical)` finds the file regardless of
//!      cwd — the packaged case),
//!   3. loads it, watches it, then edits the file on disk,
//!   4. polls `poll_reloads` and forwards changes to the registry (the schedule's forwarder,
//!      inlined), asserting the edited value is picked up.
//!
//! Exits 0 on a successful reload; non-zero (with a diagnosis) otherwise. Driven by
//! `scripts/hot_reload_smoke.sh`. `notify` has real latency, so it polls with a bounded retry
//! rather than once — deterministic in practice, not a timing race.

use engine::{asset_path, AssetServer, DataTableRegistry};
use std::time::{Duration, Instant};

fn main() {
    // 1. A data table under a temp asset root, addressed by a relative logical path that does NOT
    //    exist relative to the working directory (what a packaged build's launch dir looks like).
    let root =
        std::env::temp_dir().join(format!("skeleton-hot-reload-smoke-{}", std::process::id()));
    let data_dir = root.join("data");
    std::fs::create_dir_all(&data_dir).expect("create temp asset root");
    let file = data_dir.join("items.ron");
    let logical = "data/items.ron"; // relative — resolved against the asset root, not cwd
    std::fs::write(&file, br#"[ ( name: "sword", tier: "common" ) ]"#).expect("write table");

    // 2. Pin the asset root so `resolve(logical)` finds the temp file regardless of cwd. Process-
    //    global, which is exactly why this is a dedicated binary and not a unit test (a pinned root
    //    would leak across a parallel test suite — see `asset_path` docs).
    asset_path::set_asset_root(&root);

    // 3. Load + watch through the same two objects `App` wires together.
    let mut assets = AssetServer::new();
    let mut tables = DataTableRegistry::default();
    tables.load("items", logical).expect("initial load");
    assets.watch_data_table_path(logical); // registers the watch on resolve(logical), not logical

    let tier0 = tier_of(&tables);
    if !tier0.contains("common") {
        fail(
            &format!("initial load wrong: expected tier 'common', got {tier0:?}"),
            &root,
        );
    }

    // 4. Edit the file on disk — a hot-reload. Let the watcher settle before the write.
    std::thread::sleep(Duration::from_millis(300));
    std::fs::write(&file, br#"[ ( name: "sword", tier: "legendary" ) ]"#).expect("edit table");

    // 5. Poll for the change and forward it to the registry, exactly as `schedule.rs` does. Bounded
    //    retry: `notify` has real latency, so poll for a few seconds rather than exactly once.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut reloaded: Vec<String> = Vec::new();
    let mut tier1 = tier0.clone();
    while Instant::now() < deadline {
        for p in assets.poll_reloads() {
            reloaded.push(p.clone());
            tables.reload_path(&p); // the `HotReloadable` forwarder in schedule.rs, inlined
        }
        tier1 = tier_of(&tables);
        if tier1.contains("legendary") {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // 6. Verdict.
    if tier1.contains("legendary") {
        std::fs::remove_dir_all(&root).ok();
        println!(
            "OK: hot-reload fired under a foreign asset root — tier 'common' -> 'legendary'. \
             reloaded={reloaded:?}"
        );
    } else {
        fail(
            &format!(
                "an edit to a resolved-not-cwd file did NOT hot-reload (tier still {tier1:?}); \
                 reloaded={reloaded:?}"
            ),
            &root,
        );
    }
}

/// The first row's `tier` column, rendered via `Debug` so no `ron` import is needed here.
fn tier_of(tables: &DataTableRegistry) -> String {
    tables
        .get("items")
        .and_then(|t| t.get(0, "tier"))
        .map(|v| format!("{v:?}"))
        .unwrap_or_default()
}

fn fail(msg: &str, root: &std::path::Path) -> ! {
    std::fs::remove_dir_all(root).ok();
    eprintln!("FAIL: {msg}");
    std::process::exit(1);
}
