//! Generic `name → value` registry with native, canonical-path hot-reload.
//!
//! The data-driven config registries (particle configs, dialogue trees, animation clip sets)
//! all share one shape: a `name → value` map plus a `name → source-path` map, a
//! `load(name, path)`, and a `reload_path(path)` that re-reads the file whose canonical source
//! path matches the one `AssetServer::poll_reloads` reports. This module factors that out so the
//! three public registries become thin wrappers that keep their own value and error types.
//!
//! The data-table registry is intentionally **not** built on this — it stores the path on each
//! value and adds dirty-tracking plus a richer reload outcome, so it stays bespoke.

use std::collections::HashMap;

/// A value type that can be loaded from a RON file on disk.
///
/// Native-only: hot-reload (the sole consumer) does not run on wasm.
#[cfg(not(target_arch = "wasm32"))]
pub trait RonLoadable: Sized {
    /// Error returned by [`load_ron`](RonLoadable::load_ron).
    type Err: std::fmt::Display;
    /// Read and parse the value from the file at `path`.
    fn load_ron(path: &str) -> Result<Self, Self::Err>;
}

/// Generic `name → value` registry. Hot-reload (native) keys off the canonical source path.
pub struct RonRegistry<V> {
    items: HashMap<String, V>,
    /// `name → canonical source key` (native only). Stored as the `asset_key` so it matches the
    /// canonical path `AssetServer::poll_reloads` reports.
    #[cfg(not(target_arch = "wasm32"))]
    paths: HashMap<String, String>,
}

// Hand-written so `V` need not be `Default`.
impl<V> Default for RonRegistry<V> {
    fn default() -> Self {
        Self {
            items: HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            paths: HashMap::new(),
        }
    }
}

impl<V> RonRegistry<V> {
    /// Insert a value directly under `name` (in-memory; records no source path).
    pub fn insert(&mut self, name: impl Into<String>, value: V) {
        self.items.insert(name.into(), value);
    }

    /// Look up a value by name.
    pub fn get(&self, name: &str) -> Option<&V> {
        self.items.get(name)
    }

    /// Sorted list of registered names.
    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.items.keys().cloned().collect();
        v.sort();
        v
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<V: RonLoadable> RonRegistry<V> {
    /// Load a value from `path` and register it under `name`.
    pub fn load(&mut self, name: impl Into<String>, path: &str) -> Result<(), V::Err> {
        let value = V::load_ron(path)?;
        let name = name.into();
        // Store the canonical key so `reload_path` matches the path poll_reloads reports.
        self.paths
            .insert(name.clone(), crate::asset::asset_key(path).to_string());
        self.items.insert(name, value);
        Ok(())
    }

    /// Re-read the file whose registered source path matches `path`.
    ///
    /// No-op if nothing is registered for that path; a read/parse failure is logged under the
    /// `what` subsystem tag and leaves the existing value untouched. `path` is canonicalized
    /// before matching, so both the canonical path poll_reloads reports and a raw path resolve.
    pub fn reload_path(&mut self, path: &str, what: &str) {
        let key = crate::asset::asset_key(path).to_string();
        let Some(name) = self
            .paths
            .iter()
            .find(|(_, p)| p.as_str() == key.as_str())
            .map(|(n, _)| n.clone())
        else {
            return;
        };
        match V::load_ron(path) {
            Ok(value) => {
                self.items.insert(name, value);
            }
            Err(e) => {
                log::warn!("{what}: hot-reload failed for {path}: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get_names_sorted() {
        let mut reg: RonRegistry<i32> = RonRegistry::default();
        reg.insert("b", 2);
        reg.insert("a", 1);
        assert_eq!(reg.get("a"), Some(&1));
        assert_eq!(reg.get("missing"), None);
        assert_eq!(reg.names(), vec!["a".to_string(), "b".to_string()]);
    }

    // The canonical-path hot-reload contract every wrapper relies on: store the canonical key
    // at load, then match a (possibly differently-spelled) path by re-canonicalizing it.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn load_then_reload_by_canonical_path() {
        struct V(String);
        impl RonLoadable for V {
            type Err = std::io::Error;
            fn load_ron(path: &str) -> Result<Self, Self::Err> {
                Ok(V(std::fs::read_to_string(path)?))
            }
        }

        let path = std::env::temp_dir().join(format!("ron_reg_{}.txt", std::process::id()));
        let p = path.to_string_lossy().to_string();
        std::fs::write(&path, "one").expect("write");

        let mut reg: RonRegistry<V> = RonRegistry::default();
        reg.load("k", &p).expect("load");
        assert_eq!(reg.get("k").unwrap().0, "one");

        std::fs::write(&path, "two").expect("rewrite");
        reg.reload_path(crate::asset::asset_key(&p).as_ref(), "test");
        assert_eq!(
            reg.get("k").unwrap().0,
            "two",
            "reload must match the canonical path poll_reloads reports"
        );
        let _ = std::fs::remove_file(&path);
    }
}
