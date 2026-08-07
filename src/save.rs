#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use rand::{rngs::OsRng, RngCore};
use serde::{de::DeserializeOwned, Serialize};

// AEAD core (magic, nonce, cipher, encrypt/decrypt) is cross-platform. Only the *storage*
// backend differs per target: a file on native, a hex-encoded string in `localStorage` on wasm.
const SAVE_MAGIC: &[u8; 9] = b"R2DAEAD01";
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
    /// The operation isn't available in the current environment (e.g. no `localStorage`), or a
    /// save is tagged with a schema version newer than the migrator knows.
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
                    "Operation not supported in this environment (no storage, or a future save version)"
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
///
/// On wasm the encrypted blob is hex-encoded and stored in `localStorage` (keyed by the path
/// string) instead of a file. Note that `localStorage` is inspectable by the user — like the
/// native file, the embedded key offers tamper-detection and obfuscation, not true secrecy.
pub fn save_with_key<T: Serialize>(path: &Path, data: &T, key: SaveKey) -> Result<(), SaveError> {
    let plaintext = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
        .map_err(|e| SaveError::Ron(e.to_string()))?;
    let encrypted = encrypt_save_bytes(plaintext.as_bytes(), key)?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Write-then-rename, never write-in-place. A crash, a kill, or a power loss part-way
        // through `fs::write` leaves a TRUNCATED file — and a truncated AEAD blob fails
        // authentication on load, so `load_with_key` reports it as `SaveError::Decrypt`
        // ("corrupted or tampered with") with no fallback and no previous version to fall back
        // to: the player's progress is simply gone, and the message blames tampering.
        //
        // `fs::rename` is atomic within a filesystem on POSIX and on Windows (`MoveFileEx`
        // replace semantics), so a reader sees either the whole old file or the whole new one.
        // The temp name APPENDS rather than replacing the extension, so `save.ron` stages
        // through `save.ron.tmp` and cannot collide with a differently-suffixed sibling.
        let mut tmp = path.as_os_str().to_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        fs::write(&tmp, encrypted)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }
    // wasm: localStorage holds strings → hex-encode the binary AEAD blob.
    #[cfg(target_arch = "wasm32")]
    {
        wasm_storage::set(&path.to_string_lossy(), &to_hex(&encrypted))
    }
}

/// Decrypts the save file and deserializes it from RON. Returns `Err(SaveError::Io(NotFound))` if the file is absent.
///
/// Uses [`SaveKey::DEFAULT`] for backwards compatibility. Prefer [`load_with_key`]
/// when loading saves written with [`save_with_key`].
pub fn load<T: DeserializeOwned>(path: &Path) -> Result<T, SaveError> {
    load_with_key(path, SaveKey::DEFAULT)
}

/// Decrypts the save with the specified key and deserializes it from RON. Returns
/// `Err(SaveError::Io(NotFound))` when the file (native) or `localStorage` key (wasm) is absent.
pub fn load_with_key<T: DeserializeOwned>(path: &Path, key: SaveKey) -> Result<T, SaveError> {
    #[cfg(not(target_arch = "wasm32"))]
    let bytes = fs::read(path)?;
    #[cfg(target_arch = "wasm32")]
    let bytes = match wasm_storage::get(&path.to_string_lossy())? {
        Some(hex) => from_hex(&hex).ok_or(SaveError::Corrupted)?,
        None => {
            return Err(SaveError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "key not present in localStorage",
            )))
        }
    };
    let plaintext = decrypt_save_bytes(&bytes, key)?;
    let s = std::str::from_utf8(&plaintext).map_err(|_| SaveError::Corrupted)?;
    ron::from_str(s).map_err(|e| SaveError::Ron(e.to_string()))
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
    // wasm: persist the RON text to localStorage, keyed by the path string.
    #[cfg(target_arch = "wasm32")]
    {
        let text = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
            .map_err(|e| SaveError::Ron(e.to_string()))?;
        wasm_storage::set(&path.to_string_lossy(), &text)
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
    // wasm: read the value from localStorage. The value could be either:
    //   - A hex-encoded AEAD blob (written by the old `save()` path) → decrypt then parse.
    //   - A plain-text RON string (written by `write_ron()`) → parse directly.
    #[cfg(target_arch = "wasm32")]
    {
        match wasm_storage::get(&path.to_string_lossy())? {
            Some(s) => {
                // Try to decode as hex; if the resulting bytes start with SAVE_MAGIC it is an
                // encrypted blob that we fall back to decrypting (back-compat with old save()).
                if let Some(bytes) = from_hex(&s) {
                    if bytes.starts_with(SAVE_MAGIC) {
                        let plaintext = decrypt_save_bytes(&bytes, SaveKey::DEFAULT)?;
                        let ron_str =
                            std::str::from_utf8(&plaintext).map_err(|_| SaveError::Corrupted)?;
                        return ron::from_str(ron_str).map_err(|e| SaveError::Ron(e.to_string()));
                    }
                }
                // Not a hex AEAD blob — treat as plain-text RON.
                ron::from_str(&s).map_err(|e| SaveError::Ron(e.to_string()))
            }
            None => Err(SaveError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "key not present in localStorage",
            ))),
        }
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

/// Returns whether the save exists. On wasm, whether the `localStorage` key is present.
pub fn exists(path: &Path) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        path.exists()
    }
    #[cfg(target_arch = "wasm32")]
    {
        wasm_storage::get(&path.to_string_lossy())
            .ok()
            .flatten()
            .is_some()
    }
}

/// Deletes the save. Returns `Ok(())` if it does not exist. On wasm, removes the `localStorage` key.
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
        wasm_storage::remove(&path.to_string_lossy())
    }
}

/// Lowercase-hex encode — wasm `localStorage` holds strings, so the binary AEAD blob is hex'd.
#[cfg(target_arch = "wasm32")]
fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0x0f) as u32, 16).unwrap());
    }
    s
}

/// Decode a hex string back to bytes; `None` on odd length or a non-hex digit.
#[cfg(target_arch = "wasm32")]
fn from_hex(s: &str) -> Option<Vec<u8>> {
    let bytes = s.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16)?;
        let lo = (bytes[i + 1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
        i += 2;
    }
    Some(out)
}

/// Thin `localStorage` wrapper used by the wasm save paths. Both plain-text designer assets
/// (`write_ron` / `read_ron`) and encrypted player saves (`save` / `load`, hex-encoded) persist
/// here, keyed by the path string; `exists` / `delete` operate on the same keys.
#[cfg(target_arch = "wasm32")]
mod wasm_storage {
    use super::SaveError;

    fn storage() -> Result<web_sys::Storage, SaveError> {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .ok_or(SaveError::Unsupported)
    }

    pub fn get(key: &str) -> Result<Option<String>, SaveError> {
        storage()?.get_item(key).map_err(|_| SaveError::Unsupported)
    }

    pub fn set(key: &str, value: &str) -> Result<(), SaveError> {
        storage()?
            .set_item(key, value)
            .map_err(|_| SaveError::Unsupported)
    }

    pub fn remove(key: &str) -> Result<(), SaveError> {
        storage()?
            .remove_item(key)
            .map_err(|_| SaveError::Unsupported)
    }
}

fn cipher(key: SaveKey) -> ChaCha20Poly1305 {
    ChaCha20Poly1305::new(Key::from_slice(&key.0))
}

fn encrypt_save_bytes(plaintext: &[u8], key: SaveKey) -> Result<Vec<u8>, SaveError> {
    // OsRng works on both native and wasm (the latter via getrandom's `js` backend), unlike
    // `thread_rng()` which is not wired up on wasm.
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher(key)
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|_| SaveError::Corrupted)?;

    let mut out = Vec::with_capacity(SAVE_MAGIC.len() + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(SAVE_MAGIC);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

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
/// `format` selects how `data` encodes the payload:
/// - [`ENVELOPE_FORMAT_TEXT`] (written since 0.116): `data` is the payload's **RON text** —
///   full serde fidelity, including data-carrying enum variants that a [`ron::Value`] cannot
///   represent (EW-006). The load path parses it into a `ron::Value` only when migration steps
///   actually have to run.
/// - `0` / absent (saves written before 0.116): `data` is the payload parsed into a generic
///   [`ron::Value`] tree; loaded via the legacy path (which degrades enum variants to maps —
///   the reason the format changed).
#[derive(Serialize)]
struct VersionedEnvelope<'a> {
    version: u32,
    format: u32,
    data: &'a str,
}

/// Owned counterpart used during loading. `data` stays a [`ron::Value`] so **both** formats
/// parse: a text envelope reads it as `Value::String` holding the payload text, a legacy
/// envelope as the payload tree. `format` defaults to `0` (legacy) when the field is absent.
#[derive(serde::Deserialize)]
struct VersionedEnvelopeOwned {
    version: u32,
    #[serde(default)]
    format: u32,
    data: ron::Value,
}

/// Envelope `format` tag for payload-as-RON-text (see [`VersionedEnvelope`]).
const ENVELOPE_FORMAT_TEXT: u32 = 1;

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
/// The on-disk format is an encrypted RON envelope holding the payload as RON text, so any
/// serde-serializable payload round-trips with full fidelity — **including data-carrying enum
/// variants** (e.g. `Closed { reopen_day: u32 }`), which the pre-0.116 envelope silently lost.
/// Use [`load_migrated`] to load and automatically upgrade old saves.
///
/// One constraint remains: [`SaveMigrator`] steps operate on a [`ron::Value`], which cannot
/// represent enum variants — so a save that still needs **migrating** cannot carry them through
/// the steps (see [`load_migrated`]). Keep enum-carrying fields stable across schema versions,
/// or mirror them into structs while a migration is pending.
///
/// On wasm the encrypted envelope is hex-encoded into `localStorage` (see [`save_with_key`]).
pub fn save_versioned<T: Serialize>(path: &Path, version: u32, data: &T) -> Result<(), SaveError> {
    save_versioned_with_key(path, version, data, SaveKey::DEFAULT)
}

/// Like [`save_versioned`] but encrypts with `key` instead of [`SaveKey::DEFAULT`].
///
/// Pair with [`load_migrated_with_key`] using the same key.
///
/// On wasm the encrypted envelope is hex-encoded into `localStorage` (see [`save_with_key`]).
pub fn save_versioned_with_key<T: Serialize>(
    path: &Path,
    version: u32,
    data: &T,
    key: SaveKey,
) -> Result<(), SaveError> {
    // Keep the payload as RON TEXT in the envelope. The pre-0.116 envelope parsed it into a
    // generic ron::Value tree, but ron::Value cannot represent enum variants — a data-carrying
    // enum (`Closed(reopen_day: 3)`) silently degraded to a bare map at save time and then
    // failed at load with "expected enum, found map" (EW-006). Text keeps full serde fidelity;
    // the load path only falls back to the Value hop when migration steps actually run.
    let ron_str = ron::ser::to_string(data).map_err(|e| SaveError::Ron(e.to_string()))?;
    let envelope = VersionedEnvelope {
        version,
        format: ENVELOPE_FORMAT_TEXT,
        data: &ron_str,
    };
    save_with_key(path, &envelope, key)
}

/// Loads an encrypted versioned save written by [`save_versioned`], applies
/// migration steps as needed, and deserializes the result into `T`.
///
/// If the file's stored version is **older** than `migrator.current_version()`,
/// each registered step from `stored_version` up to `current_version` is applied
/// in order before deserialization.
///
/// When the stored version already **equals** the current version (no steps to run), the
/// payload deserializes straight from its stored RON text with full serde fidelity — including
/// data-carrying enum variants (EW-006). When steps **do** run, the payload passes through the
/// [`ron::Value`] the steps operate on, which cannot represent enum variants — an enum-carrying
/// payload that needs migrating fails with a RON error instead of round-tripping (see
/// [`save_versioned`] for the workaround). Saves written before 0.116 (the tree envelope) load
/// via the legacy path unchanged.
///
/// # Errors
///
/// - [`SaveError::Unsupported`] if the stored version is **newer** than
///   `migrator.current_version()` (save written by a future build).
/// - [`SaveError::Corrupted`] / [`SaveError::Ron`] / [`SaveError::Io`] for the
///   usual decrypt or parse failures.
///
/// On wasm this reads the hex-encoded envelope from `localStorage` (see [`load_with_key`]).
pub fn load_migrated<T: DeserializeOwned>(
    path: &Path,
    migrator: &SaveMigrator,
) -> Result<T, SaveError> {
    load_migrated_with_key(path, migrator, SaveKey::DEFAULT)
}

/// Like [`load_migrated`] but decrypts with `key` instead of [`SaveKey::DEFAULT`].
///
/// Pair with [`save_versioned_with_key`] using the same key.
///
/// On wasm this reads the hex-encoded envelope from `localStorage` (see [`load_with_key`]).
pub fn load_migrated_with_key<T: DeserializeOwned>(
    path: &Path,
    migrator: &SaveMigrator,
    key: SaveKey,
) -> Result<T, SaveError> {
    let envelope: VersionedEnvelopeOwned = load_with_key(path, key)?;
    let stored = envelope.version;
    let current = migrator.current_version();
    if stored > current {
        return Err(SaveError::Unsupported);
    }
    let value = match (envelope.format, envelope.data) {
        // Text envelope (0.116+): the payload is RON text. With no pending migration steps,
        // deserialize `T` straight from the text — no ron::Value hop, so data-carrying enum
        // variants round-trip (EW-006).
        (ENVELOPE_FORMAT_TEXT, ron::Value::String(text)) => {
            if stored == current {
                return ron::from_str::<T>(&text).map_err(|e| SaveError::Ron(e.to_string()));
            }
            // Migration steps operate on ron::Value (see [`SaveMigrator`]), so the hop — and
            // its enum limitation — still applies to saves that need migrating.
            ron::from_str::<ron::Value>(&text).map_err(|e| SaveError::Ron(e.to_string()))?
        }
        // A text-format envelope whose payload is not a string was tampered with.
        (ENVELOPE_FORMAT_TEXT, _) => return Err(SaveError::Corrupted),
        // Legacy tree envelope (pre-0.116 save) — the historical path.
        (_, value) => value,
    };
    let migrated = migrator.migrate(value, stored);
    // Drive serde deserialization directly from the ron::Value tree — avoids the struct-syntax
    // round-trip issue where re-serialising a Value::Map produces `{"field": value}` instead of
    // the `(field: value)` syntax RON expects for named structs.
    migrated
        .into_rust::<T>()
        .map_err(|e| SaveError::Ron(e.to_string()))
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
