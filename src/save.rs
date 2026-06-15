#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
#[cfg(not(target_arch = "wasm32"))]
use rand::RngCore;
use serde::{de::DeserializeOwned, Serialize};

#[cfg(not(target_arch = "wasm32"))]
const SAVE_MAGIC: &[u8; 9] = b"R2DAEAD01";
#[cfg(not(target_arch = "wasm32"))]
const NONCE_LEN: usize = 12;
const SAVE_KEY_BYTES: [u8; 32] = [
    0x52, 0x32, 0x44, 0x45, 0x2d, 0x53, 0x41, 0x56, 0x45, 0x2d, 0x41, 0x45, 0x41, 0x44, 0x2d, 0x4b,
    0x31, 0x9f, 0x6c, 0x21, 0xb8, 0x43, 0xd0, 0x75, 0xe2, 0x0a, 0x5c, 0x99, 0x13, 0xfe, 0x67, 0x2b,
];

/// AEAD key used to encrypt and authenticate save files.
///
/// A key embedded in a client binary is not a secret against a determined user. Use
/// [`SaveKey`] to separate saves between builds/users and to detect tampering, not
/// as a complete secret-protection boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveKey(pub [u8; 32]);

impl SaveKey {
    /// Built-in compatibility key used by [`save`] and [`load`].
    ///
    /// This default key is public by construction because it ships in the binary.
    pub const DEFAULT: Self = Self(SAVE_KEY_BYTES);
}

/// Save/load error type.
#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    Ron(String),
    Corrupted,
    /// Save/load is not supported on the current target (e.g. wasm — no filesystem).
    Unsupported,
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "IO error: {e}"),
            SaveError::Ron(s) => write!(f, "RON error: {s}"),
            SaveError::Corrupted => write!(f, "Save file is corrupted or has been tampered with"),
            SaveError::Unsupported => {
                write!(
                    f,
                    "Save/load is not supported on this target (no filesystem)"
                )
            }
        }
    }
}

impl std::error::Error for SaveError {}

impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self {
        SaveError::Io(e)
    }
}

/// Strips any `..`, absolute-root, or drive-prefix components from a path string,
/// keeping only the `Normal` (plain name) segments.  This prevents path-traversal
/// when caller-supplied strings are joined into a trusted base directory.
///
/// Legitimate sub-directory separators are preserved (`"saves/slot1.sav"` → `"saves/slot1.sav"`);
/// only `..` and absolute/root escapes are removed.
fn sanitize_path_component(s: &str) -> PathBuf {
    use std::path::Component;
    PathBuf::from(s)
        .components()
        .filter_map(|c| match c {
            Component::Normal(n) => Some(PathBuf::from(n)),
            // ParentDir (..), RootDir (/), Prefix (C:\) are all stripped.
            _ => None,
        })
        .collect()
}

/// Returns the save-file path under the OS standard data directory.
///
/// `app_name` and `file` are sanitized to remove any `..` components or absolute-path
/// escapes, so callers cannot traverse outside the data directory.
/// Legitimate subdirectories in `file` (e.g. `"saves/slot1.sav"`) are preserved.
///
/// On WASM returns a relative path `{app_name}/{file}` (filesystem not supported).
pub fn save_path(app_name: &str, file: &str) -> PathBuf {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let safe_app = sanitize_path_component(app_name);
        let safe_file = sanitize_path_component(file);
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(safe_app)
            .join(safe_file)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let safe_app = sanitize_path_component(app_name);
        let safe_file = sanitize_path_component(file);
        PathBuf::from(format!("{}/{}", safe_app.display(), safe_file.display()))
    }
}

/// Creates the directory, serializes data to RON, encrypts with AEAD, and writes the file.
///
/// Uses [`SaveKey::DEFAULT`] for backwards compatibility. Prefer [`save_with_key`]
/// when the application can provide its own stable key material.
pub fn save<T: Serialize>(path: &Path, data: &T) -> Result<(), SaveError> {
    save_with_key(path, data, SaveKey::DEFAULT)
}

/// Creates the directory and saves data encrypted with AEAD using the specified key.
pub fn save_with_key<T: Serialize>(path: &Path, data: &T, key: SaveKey) -> Result<(), SaveError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let plaintext = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
            .map_err(|e| SaveError::Ron(e.to_string()))?;
        let encrypted = encrypt_save_bytes(plaintext.as_bytes(), key)?;
        fs::write(path, encrypted)?;
        Ok(())
    }
    // wasm: no filesystem, so return explicit Unsupported instead of a runtime IO error.
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, data, key);
        Err(SaveError::Unsupported)
    }
}

/// Decrypts the save file and deserializes it from RON. Returns `Err(SaveError::Io(NotFound))` if the file is absent.
///
/// Uses [`SaveKey::DEFAULT`] for backwards compatibility. Prefer [`load_with_key`]
/// when loading saves written with [`save_with_key`].
pub fn load<T: DeserializeOwned>(path: &Path) -> Result<T, SaveError> {
    load_with_key(path, SaveKey::DEFAULT)
}

/// Decrypts the save file with the specified key and deserializes it from RON.
pub fn load_with_key<T: DeserializeOwned>(path: &Path, key: SaveKey) -> Result<T, SaveError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let bytes = fs::read(path)?;
        let plaintext = decrypt_save_bytes(&bytes, key)?;
        let s = std::str::from_utf8(&plaintext).map_err(|_| SaveError::Corrupted)?;
        ron::from_str(s).map_err(|e| SaveError::Ron(e.to_string()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, key);
        Err(SaveError::Unsupported)
    }
}

/// Serializes `data` to pretty RON and writes it as **plain text** — no encryption, no
/// binary header.
///
/// Intended for design-time assets such as level files and prefabs that a level designer
/// should be able to open and edit in a text editor. For player saves (scores, settings)
/// use [`save`] / [`load`] instead.
pub fn write_ron<T: Serialize>(path: &Path, data: &T) -> Result<(), SaveError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
            .map_err(|e| SaveError::Ron(e.to_string()))?;
        fs::write(path, text)?;
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, data);
        Err(SaveError::Unsupported)
    }
}

/// Reads and deserializes a plain-text RON file written by [`write_ron`].
///
/// **Back-compat:** if the file starts with the encrypted-format magic bytes (`R2DAEAD01`)
/// it falls back to the AEAD-decrypting [`load`] path so files written by older engine
/// versions (pre-4.6) still load correctly.
///
/// Intended for design-time assets such as level files and prefabs. For player saves
/// (scores, settings) use [`save`] / [`load`] instead.
pub fn read_ron<T: DeserializeOwned>(path: &Path) -> Result<T, SaveError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let bytes = fs::read(path)?;
        // If the file was written by the old encrypted path, fall back to decryption.
        if bytes.starts_with(SAVE_MAGIC) {
            let plaintext = decrypt_save_bytes(&bytes, SaveKey::DEFAULT)?;
            let s = std::str::from_utf8(&plaintext).map_err(|_| SaveError::Corrupted)?;
            return ron::from_str(s).map_err(|e| SaveError::Ron(e.to_string()));
        }
        let s = std::str::from_utf8(&bytes).map_err(|_| SaveError::Corrupted)?;
        ron::from_str(s).map_err(|e| SaveError::Ron(e.to_string()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        Err(SaveError::Unsupported)
    }
}

/// Loads and decrypts the file if it exists; returns `T::default()` if absent. Decryption/parse errors propagate as-is.
pub fn load_or_default<T: DeserializeOwned + Default>(path: &Path) -> Result<T, SaveError> {
    match load(path) {
        Ok(v) => Ok(v),
        Err(SaveError::Io(e)) if e.kind() == io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e),
    }
}

/// Returns whether the save file exists. Always `false` on wasm.
pub fn exists(path: &Path) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        path.exists()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        false
    }
}

/// Deletes the save file. Returns `Ok(())` if the file does not exist. Always `Ok(())` on wasm.
pub fn delete(path: &Path) -> Result<(), SaveError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(SaveError::Io(e)),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn cipher(key: SaveKey) -> ChaCha20Poly1305 {
    ChaCha20Poly1305::new(Key::from_slice(&key.0))
}

#[cfg(not(target_arch = "wasm32"))]
fn encrypt_save_bytes(plaintext: &[u8], key: SaveKey) -> Result<Vec<u8>, SaveError> {
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher(key)
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|_| SaveError::Corrupted)?;

    let mut out = Vec::with_capacity(SAVE_MAGIC.len() + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(SAVE_MAGIC);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

#[cfg(not(target_arch = "wasm32"))]
fn decrypt_save_bytes(bytes: &[u8], key: SaveKey) -> Result<Vec<u8>, SaveError> {
    let header_len = SAVE_MAGIC.len() + NONCE_LEN;
    if bytes.len() <= header_len || !bytes.starts_with(SAVE_MAGIC) {
        return Err(SaveError::Corrupted);
    }

    let nonce = Nonce::from_slice(&bytes[SAVE_MAGIC.len()..header_len]);
    cipher(key)
        .decrypt(nonce, &bytes[header_len..])
        .map_err(|_| SaveError::Corrupted)
}

// ── Versioned save migration ─────────────────────────────────────────────────

/// Internal envelope stored on disk by [`save_versioned`].
///
/// The `data` field holds the user payload serialized to a `ron::Value` so that
/// migration steps can inspect and mutate individual fields before the final
/// deserialization into the concrete target type.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Serialize)]
struct VersionedEnvelope<'a> {
    version: u32,
    data: &'a ron::Value,
}

/// Owned counterpart used during loading.
#[cfg(not(target_arch = "wasm32"))]
#[derive(serde::Deserialize)]
struct VersionedEnvelopeOwned {
    version: u32,
    data: ron::Value,
}

/// A chain of save-schema migration steps.
///
/// Each step is a closure that transforms the raw [`ron::Value`] representing the
/// save payload from one schema version to the next. Steps must be registered in
/// ascending order starting from 0.
///
/// The *current schema version* equals the number of registered steps: zero steps
/// means version 0, one step means the migrator can upgrade version 0 → 1 and the
/// current version is 1, and so on.
///
/// # Example
///
/// ```rust,ignore
/// let migrator = SaveMigrator::new()
///     .step(0, |mut v| {
///         if let ron::Value::Map(ref mut m) = v {
///             m.insert(
///                 ron::Value::String("coins".into()),
///                 ron::Value::Number(ron::value::Number::new(0i64)),
///             );
///         }
///         v
///     });
/// assert_eq!(migrator.current_version(), 1);
/// ```
pub struct SaveMigrator {
    steps: Vec<Box<dyn Fn(ron::Value) -> ron::Value + Send + Sync>>,
}

impl SaveMigrator {
    /// Creates an empty migrator (current version = 0).
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Registers the upgrade from schema version `from` to `from + 1`.
    ///
    /// Steps **must** be registered in order (`from = 0, 1, 2, …`).
    ///
    /// # Panics
    ///
    /// Panics if `from` does not equal the number of steps already registered,
    /// i.e. if steps are registered out of order.
    pub fn step(
        mut self,
        from: u32,
        f: impl Fn(ron::Value) -> ron::Value + Send + Sync + 'static,
    ) -> Self {
        let expected = self.steps.len() as u32;
        assert_eq!(
            from, expected,
            "SaveMigrator::step called out of order: expected from={expected}, got from={from}"
        );
        self.steps.push(Box::new(f));
        self
    }

    /// The schema version this migrator upgrades **to** (= number of registered steps).
    pub fn current_version(&self) -> u32 {
        self.steps.len() as u32
    }

    /// Applies all steps in `steps[stored_version .. current_version]` to `value`.
    #[cfg(not(target_arch = "wasm32"))]
    fn migrate(&self, mut value: ron::Value, stored_version: u32) -> ron::Value {
        for step in &self.steps[stored_version as usize..] {
            value = step(value);
        }
        value
    }
}

impl Default for SaveMigrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Saves `data` tagged with the given schema `version` using AEAD encryption
/// (same key as [`save`]).
///
/// The on-disk format is an encrypted RON envelope `(version: u32, data: <payload>)`.
/// Use [`load_migrated`] to load and automatically upgrade old saves.
///
/// Always returns `Err(SaveError::Unsupported)` on wasm (no filesystem).
pub fn save_versioned<T: Serialize>(path: &Path, version: u32, data: &T) -> Result<(), SaveError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Serialize the user payload to a generic ron::Value first so the envelope
        // stores a plain data tree rather than a double-encoded string.
        let data_value: ron::Value = {
            let ron_str = ron::ser::to_string(data).map_err(|e| SaveError::Ron(e.to_string()))?;
            ron::from_str(&ron_str).map_err(|e| SaveError::Ron(e.to_string()))?
        };
        let envelope = VersionedEnvelope {
            version,
            data: &data_value,
        };
        save(path, &envelope)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, version, data);
        Err(SaveError::Unsupported)
    }
}

/// Loads an encrypted versioned save written by [`save_versioned`], applies
/// migration steps as needed, and deserializes the result into `T`.
///
/// If the file's stored version is **older** than `migrator.current_version()`,
/// each registered step from `stored_version` up to `current_version` is applied
/// in order before deserialization.
///
/// # Errors
///
/// - [`SaveError::Unsupported`] if the stored version is **newer** than
///   `migrator.current_version()` (save written by a future build).
/// - [`SaveError::Corrupted`] / [`SaveError::Ron`] / [`SaveError::Io`] for the
///   usual decrypt or parse failures.
///
/// Always returns `Err(SaveError::Unsupported)` on wasm.
pub fn load_migrated<T: DeserializeOwned>(
    path: &Path,
    migrator: &SaveMigrator,
) -> Result<T, SaveError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let envelope: VersionedEnvelopeOwned = load(path)?;
        let stored = envelope.version;
        let current = migrator.current_version();
        if stored > current {
            return Err(SaveError::Unsupported);
        }
        let migrated = migrator.migrate(envelope.data, stored);
        // Drive serde deserialization directly from the ron::Value tree — avoids the
        // struct-syntax round-trip issue where re-serialising a Value::Map produces
        // `{"field": value}` instead of the `(field: value)` syntax RON expects for
        // named structs.
        migrated
            .into_rust::<T>()
            .map_err(|e| SaveError::Ron(e.to_string()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, migrator);
        Err(SaveError::Unsupported)
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
}
