use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Settings {
    sfx: f32,
    music: f32,
    hi_score: u32,
}

fn unique_test_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "rust-gameengine-save-test-{}-{}",
        std::process::id(),
        id
    ))
}

#[test]
fn save_load_roundtrip() {
    let dir = unique_test_dir();
    let path = dir.join("settings.ron");

    let original = Settings {
        sfx: 0.8,
        music: 0.5,
        hi_score: 9999,
    };

    save(&path, &original).expect("save should succeed");
    let raw = fs::read(&path).expect("saved file should exist");
    assert!(
        !String::from_utf8_lossy(&raw).contains("hi_score"),
        "saved file should not contain plaintext RON fields"
    );

    let loaded: Settings = load(&path).expect("load should succeed");

    assert_eq!(original, loaded);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn load_missing_file_returns_io_error() {
    let result: Result<Settings, SaveError> = load(Path::new("/nonexistent/path/foo.ron"));

    assert!(
        matches!(result, Err(SaveError::Io(_))),
        "expected SaveError::Io, got {result:?}"
    );
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
struct Counter {
    value: u32,
}

#[test]
fn load_or_default_returns_default_when_missing() {
    let path = PathBuf::from("/nonexistent/path/counter.ron");
    let result: Result<Counter, SaveError> = load_or_default(&path);
    assert_eq!(result.unwrap(), Counter::default());
}

#[test]
fn load_or_default_returns_saved_value() {
    let dir = unique_test_dir();
    let path = dir.join("counter.ron");
    let data = Counter { value: 42 };
    save(&path, &data).unwrap();
    let loaded: Counter = load_or_default(&path).unwrap();
    assert_eq!(loaded, data);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn tampered_file_returns_corrupted() {
    let dir = unique_test_dir();
    let path = dir.join("settings.ron");
    let data = Settings {
        sfx: 1.0,
        music: 0.5,
        hi_score: 7,
    };

    save(&path, &data).unwrap();
    let mut raw = fs::read(&path).unwrap();
    let last = raw.len() - 1;
    raw[last] ^= 0x01;
    fs::write(&path, raw).unwrap();

    let loaded: Result<Settings, SaveError> = load(&path);
    assert!(
        matches!(loaded, Err(SaveError::Corrupted)),
        "expected SaveError::Corrupted, got {loaded:?}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn save_load_with_key_roundtrip_and_wrong_key_fails() {
    let dir = unique_test_dir();
    let path = dir.join("keyed-settings.ron");
    let data = Settings {
        sfx: 0.2,
        music: 0.9,
        hi_score: 123,
    };
    let key = SaveKey([7; 32]);
    let wrong_key = SaveKey([8; 32]);

    save_with_key(&path, &data, key).unwrap();
    let loaded: Settings = load_with_key(&path, key).unwrap();
    assert_eq!(loaded, data);

    let wrong: Result<Settings, SaveError> = load_with_key(&path, wrong_key);
    assert!(
        matches!(wrong, Err(SaveError::Corrupted)),
        "expected wrong key to fail authentication, got {wrong:?}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn write_ron_read_ron_roundtrip() {
    let dir = unique_test_dir();
    let path = dir.join("settings.ron");

    let original = Settings {
        sfx: 0.7,
        music: 0.4,
        hi_score: 42,
    };

    write_ron(&path, &original).expect("write_ron should succeed");

    // File must be human-readable plain text containing field names
    let raw = fs::read_to_string(&path).expect("file should be readable as utf-8");
    assert!(
        raw.contains("hi_score"),
        "written file should contain the field name 'hi_score'"
    );
    assert!(
        raw.contains("sfx"),
        "written file should contain the field name 'sfx'"
    );

    let loaded: Settings = read_ron(&path).expect("read_ron should succeed");
    assert_eq!(original, loaded);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn read_ron_backcompat_encrypted_file() {
    // A file written with the old `save` (encrypted) path must still load via `read_ron`.
    let dir = unique_test_dir();
    let path = dir.join("legacy.ron");

    let original = Settings {
        sfx: 0.1,
        music: 0.9,
        hi_score: 7,
    };

    // Write using the AEAD-encrypted path (simulates a pre-4.6 file)
    save(&path, &original).expect("old save should succeed");

    // read_ron must fall back to the encrypted load path transparently
    let loaded: Settings = read_ron(&path).expect("read_ron should load encrypted legacy file");
    assert_eq!(original, loaded);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn exists_and_delete() {
    let dir = unique_test_dir();
    let path = dir.join("flag.ron");
    let data = Counter { value: 1 };

    assert!(!exists(&path));
    save(&path, &data).unwrap();
    assert!(exists(&path));
    delete(&path).unwrap();
    assert!(!exists(&path));
    // Deleting a file that is already gone → Ok
    delete(&path).unwrap();
    fs::remove_dir_all(&dir).ok();
}

// ── Fix 4: save_path must not allow path traversal ───────────────────────

/// `../../etc/passwd` in the file argument must NOT escape the data directory.
/// Without the fix, `join("../../etc/passwd")` would resolve to `/etc/passwd`.
#[test]
fn save_path_traversal_is_blocked() {
    let traversal = save_path("game", "../../etc/passwd");
    let components: Vec<_> = traversal.components().collect();

    // The path must contain no ParentDir (..) components.
    use std::path::Component;
    let has_dotdot = components.iter().any(|c| matches!(c, Component::ParentDir));
    assert!(
        !has_dotdot,
        "save_path must not contain '..' components, got: {traversal:?}"
    );

    // The result must still live under the data dir root (no component escapes it).
    // Verify by checking that the string "etc/passwd" only appears without a leading "..".
    let path_str = traversal.to_string_lossy();
    assert!(
        !path_str.contains("../etc") && !path_str.contains("..\\etc"),
        "path traversal not blocked: {traversal:?}"
    );
}

/// Legitimate sub-directory in `file` must be preserved.
#[test]
fn save_path_subdir_preserved() {
    let path = save_path("mygame", "saves/slot1.sav");
    let path_str = path.to_string_lossy();
    assert!(
        path_str.contains("saves") && path_str.contains("slot1.sav"),
        "legitimate subdir was stripped: {path:?}"
    );
    assert!(
        path_str.contains("mygame"),
        "app_name was stripped: {path:?}"
    );
}

// ── Versioned save migration tests ───────────────────────────────────────

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct PlayerSaveV1 {
    level: u32,
    score: u32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct PlayerSaveV2 {
    level: u32,
    score: u32,
    coins: u32,
}

fn make_v1_to_v2_migrator() -> SaveMigrator {
    SaveMigrator::new()
        .step(0, |v| {
            // Step 0→1: no-op (schema bookmark, no structural change)
            v
        })
        .step(1, |mut v| {
            // Step 1→2: add `coins` field with default 0
            if let ron::Value::Map(ref mut m) = v {
                m.insert(
                    ron::Value::String("coins".into()),
                    ron::Value::Number(ron::value::Number::new(0i64)),
                );
            }
            v
        })
}

/// Test 1: round-trip at current version — no migration applied.
#[test]
fn versioned_roundtrip_at_current_version() {
    let dir = unique_test_dir();
    let path = dir.join("v2.save");

    let migrator = make_v1_to_v2_migrator();
    let original = PlayerSaveV2 {
        level: 5,
        score: 800,
        coins: 42,
    };

    save_versioned(&path, migrator.current_version(), &original).unwrap();
    let loaded: PlayerSaveV2 = load_migrated(&path, &migrator).unwrap();
    assert_eq!(original, loaded);

    fs::remove_dir_all(&dir).ok();
}

/// Test 2: migration applied — old v1 payload gains `coins` field via step 1→2.
#[test]
fn versioned_migration_v1_to_v2() {
    let dir = unique_test_dir();
    let path = dir.join("v1.save");

    let migrator = make_v1_to_v2_migrator();

    // Save an old v1 payload (schema version = 1, one step already applied)
    let old = PlayerSaveV1 {
        level: 7,
        score: 1200,
    };
    save_versioned(&path, 1, &old).unwrap();

    // Load and migrate to V2; the step 1→2 should insert coins=0
    let loaded: PlayerSaveV2 = load_migrated(&path, &migrator).unwrap();
    assert_eq!(loaded.level, 7);
    assert_eq!(loaded.score, 1200);
    assert_eq!(loaded.coins, 0, "migrated coins should default to 0");

    fs::remove_dir_all(&dir).ok();
}

/// Test 3: multi-step — version 0 migrates through 0→1 (no-op) and 1→2 (add coins).
#[test]
fn versioned_migration_multistep() {
    let dir = unique_test_dir();
    let path = dir.join("v0.save");

    let migrator = make_v1_to_v2_migrator();

    // Save at version 0 (PlayerSaveV1 fields, tagged as schema 0)
    let old = PlayerSaveV1 {
        level: 3,
        score: 500,
    };
    save_versioned(&path, 0, &old).unwrap();

    // Both steps (0→1 no-op, 1→2 insert coins) must apply
    let loaded: PlayerSaveV2 = load_migrated(&path, &migrator).unwrap();
    assert_eq!(loaded.level, 3);
    assert_eq!(loaded.score, 500);
    assert_eq!(loaded.coins, 0);

    fs::remove_dir_all(&dir).ok();
}

/// EW-006 regression: a data-carrying enum variant round-trips through the versioned envelope.
/// The pre-0.116 envelope parsed the payload into a `ron::Value` (which cannot represent enum
/// variants), so `Closed(reopen_day: …)` degraded to a bare map at save time and failed at load
/// with "expected enum, found map".
#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum MarketStatus {
    Open,
    Closed { reopen_day: u32 },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct MarketSave {
    day: u32,
    markets: Vec<(u32, MarketStatus)>,
}

#[test]
fn versioned_enum_struct_variant_roundtrips_at_current_version() {
    let dir = unique_test_dir();
    let path = dir.join("enum.save");

    let migrator = make_v1_to_v2_migrator(); // current = 2, no steps run at current version
    let original = MarketSave {
        day: 9,
        markets: vec![
            (1, MarketStatus::Open),
            (2, MarketStatus::Closed { reopen_day: 12 }),
        ],
    };

    save_versioned(&path, migrator.current_version(), &original).unwrap();
    let loaded: MarketSave = load_migrated(&path, &migrator).unwrap();
    assert_eq!(original, loaded);

    fs::remove_dir_all(&dir).ok();
}

/// The documented EW-006 constraint: an enum-carrying payload that still needs MIGRATING passes
/// through the `ron::Value` the steps operate on, so it fails with a RON error (instead of
/// silently corrupting) — enums survive only the no-migration path.
#[test]
fn versioned_enum_needing_migration_errors_instead_of_corrupting() {
    let dir = unique_test_dir();
    let path = dir.join("enum_migrated.save");

    let migrator = make_v1_to_v2_migrator(); // current = 2
    let original = MarketSave {
        day: 1,
        markets: vec![(1, MarketStatus::Closed { reopen_day: 3 })],
    };

    // Tagged one version behind → the 1→2 step runs at load, forcing the Value hop.
    save_versioned(&path, 1, &original).unwrap();
    let result: Result<MarketSave, SaveError> = load_migrated(&path, &migrator);
    assert!(matches!(result, Err(SaveError::Ron(_))), "got {result:?}");

    fs::remove_dir_all(&dir).ok();
}

/// Back-compat: a legacy (pre-0.116) envelope — payload stored as a `ron::Value` tree with no
/// `format` field — still loads and migrates through the legacy path.
#[test]
fn versioned_legacy_tree_envelope_still_loads_and_migrates() {
    let dir = unique_test_dir();
    let path = dir.join("legacy.save");

    // Replicate the pre-0.116 writer exactly: (version, data: <payload as a Value tree>).
    #[derive(Serialize)]
    struct LegacyEnvelope {
        version: u32,
        data: ron::Value,
    }
    let old = PlayerSaveV1 {
        level: 4,
        score: 900,
    };
    let data: ron::Value = ron::from_str(&ron::ser::to_string(&old).unwrap()).unwrap();
    save(&path, &LegacyEnvelope { version: 1, data }).unwrap();

    // The 1→2 step inserts `coins`; the legacy tree path must still feed it.
    let migrator = make_v1_to_v2_migrator();
    let loaded: PlayerSaveV2 = load_migrated(&path, &migrator).unwrap();
    assert_eq!(loaded.level, 4);
    assert_eq!(loaded.score, 900);
    assert_eq!(loaded.coins, 0, "legacy save migrated with defaulted coins");

    fs::remove_dir_all(&dir).ok();
}

/// Test 4: future version → SaveError::Unsupported.
#[test]
fn versioned_future_version_returns_unsupported() {
    let dir = unique_test_dir();
    let path = dir.join("future.save");

    let migrator = make_v1_to_v2_migrator(); // current = 2
    let data = PlayerSaveV2 {
        level: 1,
        score: 0,
        coins: 0,
    };

    // Save tagged as version 3 (future)
    save_versioned(&path, migrator.current_version() + 1, &data).unwrap();

    let result: Result<PlayerSaveV2, SaveError> = load_migrated(&path, &migrator);
    assert!(
        matches!(result, Err(SaveError::Unsupported)),
        "expected SaveError::Unsupported for future version, got {result:?}"
    );

    fs::remove_dir_all(&dir).ok();
}

/// Test 5a: current_version() equals the number of steps.
#[test]
fn migrator_current_version_equals_step_count() {
    let m0 = SaveMigrator::new();
    assert_eq!(m0.current_version(), 0);

    let m1 = SaveMigrator::new().step(0, |v| v);
    assert_eq!(m1.current_version(), 1);

    let m2 = SaveMigrator::new().step(0, |v| v).step(1, |v| v);
    assert_eq!(m2.current_version(), 2);
}

/// Test 5b: step() panics when called out of order.
#[test]
#[should_panic(expected = "SaveMigrator::step called out of order")]
fn migrator_step_out_of_order_panics() {
    SaveMigrator::new().step(1, |v| v); // should panic: expected from=0
}

/// Test 6: save_versioned_with_key / load_migrated_with_key round-trip with a
/// custom key; also verifies that DEFAULT key cannot decrypt the result.
#[test]
fn versioned_with_key_roundtrip_and_wrong_key_fails() {
    let dir = unique_test_dir();
    let path = dir.join("keyed-v2.save");

    let custom_key = SaveKey([0xAB; 32]);
    let wrong_key = SaveKey([0xCD; 32]);

    let migrator = make_v1_to_v2_migrator();
    let original = PlayerSaveV2 {
        level: 9,
        score: 333,
        coins: 7,
    };

    save_versioned_with_key(&path, migrator.current_version(), &original, custom_key)
        .expect("save_versioned_with_key should succeed");

    // Correct key must round-trip without migration.
    let loaded: PlayerSaveV2 =
        load_migrated_with_key(&path, &migrator, custom_key).expect("load should succeed");
    assert_eq!(original, loaded);

    // Wrong key must fail authentication.
    let bad: Result<PlayerSaveV2, SaveError> = load_migrated_with_key(&path, &migrator, wrong_key);
    assert!(
        matches!(bad, Err(SaveError::Corrupted)),
        "expected SaveError::Corrupted with wrong key, got {bad:?}"
    );

    // DEFAULT key must also fail (file was not written with it).
    let default_key_attempt: Result<PlayerSaveV2, SaveError> =
        load_migrated_with_key(&path, &migrator, SaveKey::DEFAULT);
    assert!(
        matches!(default_key_attempt, Err(SaveError::Corrupted)),
        "expected SaveError::Corrupted with DEFAULT key, got {default_key_attempt:?}"
    );

    fs::remove_dir_all(&dir).ok();
}

/// A save must be **staged through a sibling temp file and renamed**, never written in place.
///
/// `fs::write` truncates the target before it writes. A crash, kill, or power loss part-way
/// through leaves a truncated file — and a truncated AEAD blob fails authentication, so
/// `load_with_key` reports `SaveError::Decrypt` ("corrupted or tampered with"), blames tampering,
/// and has nothing to fall back to. The player's progress is gone.
///
/// A unit test cannot kill the process mid-write, so this pins the observable half of the
/// mechanism: the staging file is used and cleaned up, and a **stale** staging file left behind by
/// an earlier crashed run is overwritten rather than appended to or tripped over.
#[test]
fn save_stages_through_a_temp_file_and_leaves_none_behind() {
    let dir = unique_test_dir();
    let path = dir.join("progress.ron");
    let mut tmp_os = path.as_os_str().to_os_string();
    tmp_os.push(".tmp");
    let tmp = PathBuf::from(tmp_os);

    let first = Settings {
        sfx: 0.1,
        music: 0.2,
        hi_score: 1,
    };
    save(&path, &first).expect("save should succeed");
    assert!(path.exists(), "target save must exist");
    assert!(
        !tmp.exists(),
        "the staging file must be renamed away, not left behind: {tmp:?}"
    );

    // Simulate the debris of an earlier run that died between write and rename.
    fs::write(&tmp, b"garbage from a crashed run").expect("seed stale temp");

    let second = Settings {
        sfx: 0.9,
        music: 0.8,
        hi_score: 4242,
    };
    save(&path, &second).expect("save must succeed despite a stale staging file");
    assert!(
        !tmp.exists(),
        "a stale staging file must be consumed by the rename, not left behind"
    );
    let loaded: Settings = load(&path).expect("load should succeed");
    assert_eq!(
        second, loaded,
        "the target must hold the whole new save, never a mix"
    );

    fs::remove_dir_all(&dir).ok();
}
