//! Smoke tests for the save/load module.
//!
//! `write_ron`/`read_ron` and the AEAD `save`/`load` functions are native-only
//! (they require a filesystem), so the entire test module is cfg-gated.
//! Uses `std::env::temp_dir()` — no extra crate dependencies needed.

#![cfg(not(target_arch = "wasm32"))]

use engine::save::{read_ron, write_ron};
use engine::SaveError;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Level {
    name: String,
    width: u32,
    height: u32,
}

/// RON round-trip: write then read back produces the original value.
#[test]
fn write_read_ron_round_trip() {
    let original = Level {
        name: "forest".to_string(),
        width: 32,
        height: 24,
    };

    let path = env::temp_dir().join("engine_save_smoke_ron.ron");
    write_ron(&path, &original).expect("write_ron failed");

    let loaded: Level = read_ron(&path).expect("read_ron failed");
    assert_eq!(loaded, original);

    // Clean up
    let _ = std::fs::remove_file(&path);
}

/// Reading a non-existent file produces an Io error (not a panic).
#[test]
fn read_ron_missing_file_returns_io_err() {
    let path = env::temp_dir().join("engine_save_smoke_nonexistent_xyz.ron");
    // Make sure it does not exist
    let _ = std::fs::remove_file(&path);

    let result: Result<Level, SaveError> = read_ron(&path);
    assert!(
        matches!(result, Err(SaveError::Io(_))),
        "expected Io error, got {:?}",
        result
    );
}

/// `engine::exists` mirrors `std::fs::try_exists` semantics.
#[test]
fn exists_reflects_filesystem_state() {
    let path = env::temp_dir().join("engine_save_smoke_exists_check.ron");
    let _ = std::fs::remove_file(&path);
    assert!(!engine::exists(&path));

    let level = Level {
        name: "x".to_string(),
        width: 1,
        height: 1,
    };
    write_ron(&path, &level).unwrap();
    assert!(engine::exists(&path));

    engine::delete(&path).unwrap();
    assert!(!engine::exists(&path));
}
