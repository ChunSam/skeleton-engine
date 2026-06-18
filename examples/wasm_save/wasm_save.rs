//! wasm_save — self-check for the AEAD save/load path on the web (wasm `localStorage`).
//!
//! v0.22.0 made the whole `save`/`load`/`save_versioned`/`load_migrated` family cross-platform:
//! the ChaCha20-Poly1305 core is target-agnostic, and on wasm the encrypted blob is hex-encoded
//! into `localStorage` (keyed by the path string). That path was compile-gated + native-playtested
//! but never *run* in a browser. This example exercises it end-to-end and writes a pass/fail line
//! per step into the page (and the document title), so a headless browser can assert the
//! round-trip automatically — see `scripts/wasm_save_smoke.sh`.
//!
//! It is wasm-only (the API also works natively, but the *localStorage* backend is the point);
//! running it natively just prints how to build it.
//!
//! ```text
//! examples/wasm_save/web/build.sh                       # build to wasm
//! python3 -m http.server 8080 --directory examples/wasm_save/web
//! open http://localhost:8080
//! ```

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("wasm_save exercises the *localStorage* save backend, so build it for the browser:");
    println!("  examples/wasm_save/web/build.sh");
    println!("  python3 -m http.server 8080 --directory examples/wasm_save/web");
    println!("  open http://localhost:8080");
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;

/// WASM entry point — `examples/wasm_save/web/index.html` calls this after `init()`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_save_check() {
    use engine::{delete, exists, load, load_migrated, save, save_versioned, SaveMigrator};
    use serde::{Deserialize, Serialize};
    use std::path::Path;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Profile {
        name: String,
        level: u32,
        gold: i64,
    }

    let mut lines: Vec<String> = Vec::new();
    let mut passed = 0u32;
    let mut total = 0u32;
    let mut first_fail: Option<String> = None;

    macro_rules! check {
        ($cond:expr, $msg:expr) => {{
            let pass = $cond;
            total += 1;
            if pass {
                passed += 1;
            } else if first_fail.is_none() {
                first_fail = Some($msg.to_string());
            }
            lines.push(format!("{} {}", if pass { "✅" } else { "❌" }, $msg));
            render(&lines);
        }};
    }

    let path = Path::new("wasm_save_check.sav");
    let vpath = Path::new("wasm_save_check_v.sav");
    let profile = Profile {
        name: "Hero".into(),
        level: 7,
        gold: 1234,
    };

    // Clean slate (ignore errors — keys may not exist yet).
    let _ = delete(path);
    let _ = delete(vpath);

    // 1. AEAD save to localStorage.
    check!(
        save(path, &profile).is_ok(),
        "save() AEAD-encrypted to localStorage"
    );
    // 2. exists() sees the key.
    check!(exists(path), "exists() is true after save");
    // 3. load() round-trips the struct unchanged.
    check!(
        load::<Profile>(path).ok().as_ref() == Some(&profile),
        "load() round-trips the struct"
    );
    // 4. The stored value is the hex-encoded ciphertext, NOT plaintext (no "Hero" in it).
    check!(
        stored_is_ciphertext("wasm_save_check.sav"),
        "localStorage holds hex ciphertext, not plaintext"
    );
    // 5. Tamper detection: corrupting the blob makes load() fail (AEAD auth).
    check!(
        tamper_makes_load_fail::<Profile>(path),
        "tampered blob -> load() errors (AEAD tamper detection)"
    );
    // 6. Versioned envelope round-trips through load_migrated.
    let migrator = SaveMigrator::new();
    check!(
        save_versioned(vpath, 0, &profile).is_ok()
            && load_migrated::<Profile>(vpath, &migrator).ok().as_ref() == Some(&profile),
        "save_versioned()/load_migrated() round-trips"
    );
    // 7. delete() removes the key.
    check!(
        delete(path).is_ok() && !exists(path),
        "delete() removes the localStorage key"
    );
    let _ = delete(vpath);

    finish(passed, total, first_fail);
}

/// The browser `localStorage`, if available.
#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// Whether the stored value for `key` looks like the hex-encoded AEAD blob: non-empty, all hex
/// digits, and containing none of the plaintext (`Hero`).
#[cfg(target_arch = "wasm32")]
fn stored_is_ciphertext(key: &str) -> bool {
    match local_storage().and_then(|s| s.get_item(key).ok().flatten()) {
        Some(v) => !v.is_empty() && !v.contains("Hero") && v.bytes().all(|b| b.is_ascii_hexdigit()),
        None => false,
    }
}

/// Overwrites the stored blob with a wrong (but well-formed hex) value and checks that `load` then
/// fails — i.e. the AEAD authentication tag rejects tampering.
#[cfg(target_arch = "wasm32")]
fn tamper_makes_load_fail<T>(path: &std::path::Path) -> bool
where
    T: serde::de::DeserializeOwned,
{
    let key = path.to_string_lossy().to_string();
    let Some(storage) = local_storage() else {
        return false;
    };
    if storage
        .set_item(&key, "deadbeefdeadbeefdeadbeefdeadbeef")
        .is_err()
    {
        return false;
    }
    engine::load::<T>(path).is_err()
}

/// Writes the running list of step lines into the page's `#status` element.
#[cfg(target_arch = "wasm32")]
fn render(lines: &[String]) {
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("status"))
    {
        el.set_inner_html(&lines.join("<br>"));
    }
}

/// Stamps the verdict into the `#result` element and the document title — `SAVE_CHECK: PASS (n/n)`
/// / `SAVE_CHECK: FAIL: <step>` — which `scripts/wasm_save_smoke.sh` reads over the DevTools
/// endpoint.
#[cfg(target_arch = "wasm32")]
fn finish(passed: u32, total: u32, first_fail: Option<String>) {
    let verdict = match first_fail {
        None => format!("SAVE_CHECK: PASS ({passed}/{total})"),
        Some(step) => format!("SAVE_CHECK: FAIL: {step}"),
    };
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        doc.set_title(&verdict);
        if let Some(el) = doc.get_element_by_id("result") {
            el.set_inner_html(&verdict);
        }
    }
}
