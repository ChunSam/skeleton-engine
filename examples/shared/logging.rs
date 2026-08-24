//! One logger install, shared by every game.
//!
//! # Why this file exists
//!
//! The engine reports trouble through `log::warn!` / `log::error!` — 86 sites as of v0.154.3
//! (`grep -rnE '(log::)?(error|warn)!\(' src --include='*.rs' | grep -vE ':\s*(//|\*)' | wc -l`).
//! The `log` crate routes those to whatever logger the *binary* installs, and a binary that
//! installs none discards them silently. Between v0.153.0 (which deleted the examples tree along
//! with the only `env_logger` callers) and this file, no example installed one, so every warning
//! the engine emitted went nowhere.
//!
//! That is not a convenience gap. `docs/MODULE_MAP.md` describes asset failures as **loud** —
//! `error!` *plus* `asset_failures()` — and half of that was mute. So were the unregistered-event-bus
//! warning, `Pool::release`'s double-release guard, `SceneCmd::Replace` discarding App-registered
//! systems, and `TriggerZoneSystem`'s missing-grid notice. Every one of them is addressed to a game
//! author, and none of them reached one.
//!
//! It was found the way these things are always found: an `ENGINE_INPUT` script silently did
//! nothing on 2026-08-20. One bad key name, `log::error!("ENGINE_INPUT: {err}")` fired exactly as
//! designed ([`src/input_script.rs`]), and the run looked like a script that simply had no effect.
//!
//! # Why the default filter is `warn` and not `env_logger`'s own
//!
//! ⚠️ **`env_logger::init()` alone would not have fixed the bug above.** Its default filter is
//! `error`, so a plain `init()` still drops all 72 `warn!` sites — including the unregistered event
//! bus, which presents as "the event never arrives" while the engine is behaving perfectly. The
//! default here is therefore `warn`, and `RUST_LOG` overrides it as usual:
//!
//! ```sh
//! cargo run --example puzzle_grid_game              # warn + error
//! RUST_LOG=info cargo run --example puzzle_grid_game # + the engine's 7 info/debug/trace sites
//! RUST_LOG=off  cargo run --example puzzle_grid_game # silence
//! ```
//!
//! # What this deliberately does NOT cover
//!
//! **The browser.** On wasm this is a no-op, so the same 86 sites are still invisible in a web
//! build — `log` needs a wasm-specific sink (`console_log` or equivalent) that this repo does not
//! depend on, and adding one is a dependency decision rather than a drive-by. The engine's
//! `console_error_panic_hook` (`src/lib.rs`) covers *panics* in the browser and nothing else, so
//! the gap is exactly the non-fatal half. Named here rather than left implied.
//!
//! **The two servers.** `netplay_server` and `wasm_failpaths_echo_server` use no engine code at all
//! (`std::net` + `tungstenite`), so they have no engine log sites to surface and report through
//! their own `println!`.

/// Installs the process-wide logger. Call once, first thing in `main`.
///
/// A second call would panic (`log` allows one logger per process), which is why every caller is a
/// `main` and none is a system.
pub fn init() {
    // env_logger is a native-only dev-dependency: see the module docs on the browser gap.
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
}
