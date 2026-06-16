//! Schema-agnostic RON data table: rows of name→value pairs editable at runtime.
//!
//! A [`DataTable`] holds an ordered grid of cells where each row is a list of
//! `(column_name, ron::Value)` pairs. Tables can be loaded from disk, edited in
//! the docked editor panel, saved back, and hot-reloaded when the file changes.
//!
//! # File format
//! A plain RON sequence of maps / structs, one element per row:
//! ```ron
//! [
//!     (name: "Goblin", hp: 30, speed: 1.2),
//!     (name: "Orc",    hp: 80, speed: 0.8),
//! ]
//! ```
//! **Note:** comments and custom formatting are NOT preserved on save; the file is
//! always rewritten by `to_ron_string`.

use std::collections::HashMap;

use crate::save::SaveError;

// ── DataTable ────────────────────────────────────────────────────────────────

/// One parsed RON data table.
///
/// Rows are stored as ordered `(column, value)` pairs so that the original
/// column order is preserved across load/save cycles. Because the engine uses
/// ron 0.8 **without** the `indexmap` feature the internal `ron::Map` uses
/// `BTreeMap` (sorted keys), so column order is taken from the first row but
/// sorted alphabetically — that is the canonical column order throughout.
pub struct DataTable {
    /// Column names in alphabetical order (derived from the first row at parse time).
    pub columns: Vec<String>,
    /// Cell data: one `Vec<(col, value)>` per row, same order as `columns`.
    pub rows: Vec<Vec<(String, ron::Value)>>,
    /// Source path (empty string for in-memory tables created without a file).
    pub path: String,
    /// `true` when the editor has unsaved changes.
    pub dirty: bool,
}

impl DataTable {
    /// Parse a RON data table from a string.
    ///
    /// The RON text must be a sequence (`[…]`) whose elements are either maps
    /// (`{key: val, …}`) or anonymous structs (`(key: val, …)`).
    /// Column order is taken from the first row and sorted alphabetically (because
    /// ron 0.8 without `indexmap` uses `BTreeMap` which sorts keys).
    /// Subsequent rows that are missing a column get `ron::Value::Unit`.
    ///
    /// Prefer [`std::str::FromStr`] (`"…".parse::<DataTable>()`) for idiomatic usage.
    /// This method is kept for convenience and explicit error type use.
    pub fn parse(ron_text: &str) -> Result<Self, SaveError> {
        let value: ron::Value =
            ron::from_str(ron_text).map_err(|e| SaveError::Ron(e.to_string()))?;

        let rows_raw = match value {
            ron::Value::Seq(seq) => seq,
            other => {
                return Err(SaveError::Ron(format!(
                    "expected a RON sequence at the top level, got {:?}",
                    other
                )))
            }
        };

        if rows_raw.is_empty() {
            return Ok(DataTable {
                columns: Vec::new(),
                rows: Vec::new(),
                path: String::new(),
                dirty: false,
            });
        }

        // Extract (key, value) pairs from one row element.
        let extract_pairs = |v: ron::Value| -> Result<Vec<(String, ron::Value)>, SaveError> {
            match v {
                ron::Value::Map(m) => {
                    let mut pairs: Vec<(String, ron::Value)> = Vec::new();
                    for (k, val) in m.iter() {
                        let col = match k {
                            ron::Value::String(s) => s.clone(),
                            // Non-string keys: serialize via ron for a stable string key.
                            other => {
                                ron::ser::to_string(other).unwrap_or_else(|_| format!("{other:?}"))
                            }
                        };
                        pairs.push((col, val.clone()));
                    }
                    Ok(pairs)
                }
                other => Err(SaveError::Ron(format!(
                    "expected a map/struct element in the sequence, got {:?}",
                    other
                ))),
            }
        };

        // Build columns from the first row (BTreeMap = alphabetical sort already).
        let first_pairs = extract_pairs(rows_raw[0].clone())?;
        let mut columns: Vec<String> = first_pairs.iter().map(|(c, _)| c.clone()).collect();
        columns.sort();

        // Build all rows in column order.
        let mut rows: Vec<Vec<(String, ron::Value)>> = Vec::with_capacity(rows_raw.len());
        for (idx, raw) in rows_raw.into_iter().enumerate() {
            let pairs = extract_pairs(raw)?;
            let pair_map: HashMap<String, ron::Value> = pairs.into_iter().collect();
            // Warn about keys present in this row but absent from the schema (row 0).
            for key in pair_map.keys() {
                if !columns.contains(key) {
                    log::warn!(
                        "data_table: row {idx} has extra column '{key}' not in schema; value discarded"
                    );
                }
            }
            let row: Vec<(String, ron::Value)> = columns
                .iter()
                .map(|col| {
                    let v = pair_map.get(col).cloned().unwrap_or(ron::Value::Unit);
                    (col.clone(), v)
                })
                .collect();
            rows.push(row);
        }

        Ok(DataTable {
            columns,
            rows,
            path: String::new(),
            dirty: false,
        })
    }

    /// Load a data table from `path` on disk.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(path: &str) -> Result<Self, SaveError> {
        let text = std::fs::read_to_string(path)?;
        let mut table = Self::parse(&text)?;
        table.path = path.to_string();
        Ok(table)
    }

    /// Serialise to a RON string.
    ///
    /// Rebuilds a `ron::Value::Seq` of `ron::Value::Map` from `self.rows` and
    /// pretty-prints it. Comments and custom formatting from the original file are
    /// NOT preserved.
    pub fn to_ron_string(&self) -> Result<String, SaveError> {
        use ron::ser::PrettyConfig;

        let seq: Vec<ron::Value> = self
            .rows
            .iter()
            .map(|row| {
                let map: ron::Map = row
                    .iter()
                    .map(|(col, val)| (ron::Value::String(col.clone()), val.clone()))
                    .collect();
                ron::Value::Map(map)
            })
            .collect();

        let top = ron::Value::Seq(seq);
        ron::ser::to_string_pretty(&top, PrettyConfig::default())
            .map_err(|e| SaveError::Ron(e.to_string()))
    }

    /// Write the table back to its source file and clear `dirty`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&mut self) -> Result<(), SaveError> {
        if self.path.is_empty() {
            return Err(SaveError::Ron("table has no path set".into()));
        }
        let s = self.to_ron_string()?;
        std::fs::write(&self.path, s)?;
        self.dirty = false;
        Ok(())
    }

    /// Retrieve the value at `(row, col)` by column name.
    pub fn get(&self, row: usize, col: &str) -> Option<&ron::Value> {
        self.rows
            .get(row)?
            .iter()
            .find(|(c, _)| c == col)
            .map(|(_, v)| v)
    }

    /// Retrieve a mutable reference to the value at `(row, col)` by column name.
    pub fn get_mut(&mut self, row: usize, col: &str) -> Option<&mut ron::Value> {
        self.rows
            .get_mut(row)?
            .iter_mut()
            .find(|(c, _)| c == col)
            .map(|(_, v)| v)
    }

    /// Return a zeroed/empty default value matching the type of the value at `(0, col)`.
    fn default_value_for_col(&self, col: &str) -> ron::Value {
        self.get(0, col)
            .map(default_ron_value)
            .unwrap_or(ron::Value::Unit)
    }

    /// Append a new row with zeroed/empty values based on the first row's types.
    pub fn add_row(&mut self) {
        let row: Vec<(String, ron::Value)> = self
            .columns
            .iter()
            .map(|col| (col.clone(), self.default_value_for_col(col)))
            .collect();
        self.rows.push(row);
        self.dirty = true;
    }

    /// Delete the row at `index`.
    pub fn delete_row(&mut self, index: usize) {
        if index < self.rows.len() {
            self.rows.remove(index);
            self.dirty = true;
        }
    }
}

impl std::str::FromStr for DataTable {
    type Err = SaveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DataTable::parse(s)
    }
}

/// Return a "zeroed" clone of the given value — same variant, zero/empty payload.
fn default_ron_value(v: &ron::Value) -> ron::Value {
    match v {
        ron::Value::Bool(_) => ron::Value::Bool(false),
        ron::Value::Number(ron::Number::Integer(_)) => ron::Value::Number(ron::Number::Integer(0)),
        ron::Value::Number(ron::Number::Float(_)) => {
            ron::Value::Number(ron::Number::Float(ron::value::Float::new(0.0)))
        }
        ron::Value::String(_) => ron::Value::String(String::new()),
        ron::Value::Char(_) => ron::Value::Char('\0'),
        ron::Value::Unit => ron::Value::Unit,
        ron::Value::Seq(_) => ron::Value::Seq(Vec::new()),
        ron::Value::Map(_) => ron::Value::Map(ron::Map::new()),
        ron::Value::Option(_) => ron::Value::Option(None),
    }
}

// ── DataTableRegistry ────────────────────────────────────────────────────────

/// Registry of named [`DataTable`]s, stored as a World resource.
///
/// Game code accesses tables by name; the editor panel loads, edits, and saves
/// them through the registry. Hot reload wires through `reload_path`.
#[derive(Default)]
pub struct DataTableRegistry {
    tables: HashMap<String, DataTable>,
}

/// Outcome returned by [`DataTableRegistry::reload_path`].
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, PartialEq, Eq)]
pub enum ReloadOutcome {
    /// The table was successfully reloaded from disk.
    Reloaded,
    /// The table has unsaved edits — reload was skipped to avoid data loss.
    SkippedDirty,
    /// No table is registered for the given path.
    NotFound,
    /// Disk read or parse error; the existing table is unchanged.
    Err,
}

impl DataTableRegistry {
    /// Load a table from `path` on disk and register it under `name`.
    ///
    /// If a table with the same name already exists it is replaced.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, name: impl Into<String>, path: &str) -> Result<(), SaveError> {
        let table = DataTable::load(path)?;
        self.tables.insert(name.into(), table);
        Ok(())
    }

    /// Insert a table directly (useful for tests or in-memory tables).
    pub fn insert(&mut self, name: impl Into<String>, table: DataTable) {
        self.tables.insert(name.into(), table);
    }

    /// Look up a table by name (immutable).
    pub fn get(&self, name: &str) -> Option<&DataTable> {
        self.tables.get(name)
    }

    /// Look up a table by name (mutable — editor writes go here).
    pub fn get_mut(&mut self, name: &str) -> Option<&mut DataTable> {
        self.tables.get_mut(name)
    }

    /// Sorted list of registered table names (for the editor panel selector).
    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.tables.keys().cloned().collect();
        v.sort();
        v
    }

    /// Re-load the table whose `.path == path` from disk — unless `dirty` is set,
    /// in which case the unsaved edits are kept and a warning is logged.
    ///
    /// Returns a [`ReloadOutcome`] so callers can show an accurate status message.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload_path(&mut self, path: &str) -> ReloadOutcome {
        // `AssetServer::poll_reloads` reports the CANONICAL path (`asset_key` canonicalizes),
        // while `t.path` is whatever was passed to `load` (often relative). Compare by
        // canonicalized form so a relatively-loaded table still hot-reloads from disk.
        let canon = |p: &str| {
            std::path::Path::new(p)
                .canonicalize()
                .unwrap_or_else(|_| std::path::PathBuf::from(p))
        };
        let target = canon(path);
        let name = self
            .tables
            .iter()
            .find(|(_, t)| canon(&t.path) == target)
            .map(|(n, _)| n.clone());
        let Some(name) = name else {
            return ReloadOutcome::NotFound;
        };

        let table = self.tables.get(&name).expect("name came from iter");
        if table.dirty {
            log::warn!(
                "data_table: skipping hot-reload of '{}' (path: {path}) — table has unsaved edits",
                name
            );
            return ReloadOutcome::SkippedDirty;
        }

        match DataTable::load(path) {
            Ok(reloaded) => {
                self.tables.insert(name, reloaded);
                ReloadOutcome::Reloaded
            }
            Err(e) => {
                log::warn!("data_table: hot-reload failed for {path}: {e}");
                ReloadOutcome::Err
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
    (name: "Goblin", hp: 30, speed: 1.2),
    (name: "Orc",    hp: 80, speed: 0.8),
]"#;

    #[test]
    fn parse_columns_and_rows() {
        let table = DataTable::parse(SAMPLE).expect("parse");
        // Columns are sorted alphabetically (BTreeMap — no indexmap feature).
        assert_eq!(table.columns, vec!["hp", "name", "speed"]);
        assert_eq!(table.rows.len(), 2);
        // get by name
        let hp = table.get(0, "hp").expect("hp cell");
        assert!(
            matches!(hp, ron::Value::Number(ron::Number::Integer(30))),
            "expected Integer(30), got {hp:?}"
        );
        let speed = table.get(0, "speed").expect("speed cell");
        assert!(
            matches!(speed, ron::Value::Number(ron::Number::Float(_))),
            "expected Float, got {speed:?}"
        );
    }

    #[test]
    fn round_trip_after_edit() {
        let mut table = DataTable::parse(SAMPLE).expect("parse");
        // Edit hp of row 0 from 30 → 99
        {
            let cell = table.get_mut(0, "hp").expect("hp mut");
            *cell = ron::Value::Number(ron::Number::Integer(99));
        }
        table.dirty = true;

        let serialised = table.to_ron_string().expect("serialize");
        let table2 = DataTable::parse(&serialised).expect("re-parse");
        let hp2 = table2.get(0, "hp").expect("hp2");
        assert!(
            matches!(hp2, ron::Value::Number(ron::Number::Integer(99))),
            "expected Integer(99), got {hp2:?}"
        );
    }

    #[test]
    fn from_str_trait() {
        let table: DataTable = SAMPLE.parse().expect("from_str trait");
        assert_eq!(table.rows.len(), 2);
    }

    #[test]
    fn registry_insert_and_query() {
        let mut reg = DataTableRegistry::default();
        let table = DataTable::parse(SAMPLE).expect("parse");
        reg.insert("enemies", table);

        assert_eq!(reg.names(), vec!["enemies"]);
        let t = reg.get("enemies").expect("get");
        assert_eq!(t.columns, vec!["hp", "name", "speed"]);
        assert!(reg.get("nope").is_none());
    }

    // Regression: file-watcher hot-reload silently no-op'd because `reload_path` compared
    // the raw stored path against the CANONICAL path `AssetServer::poll_reloads` reports.
    // Loading with a non-canonical path (extra "./") here forces that exact mismatch.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn reload_path_matches_canonical_path() {
        let dir = std::env::temp_dir();
        let file = dir.join(format!("dt_reload_{}.ron", std::process::id()));
        std::fs::write(&file, SAMPLE).expect("write");

        // Load via a deliberately non-canonical path so `t.path` != the canonical path
        // `poll_reloads` reports.
        let load_path = format!(
            "{}/./{}",
            dir.to_string_lossy(),
            file.file_name().unwrap().to_string_lossy()
        );
        let mut reg = DataTableRegistry::default();
        reg.load("enemies", &load_path).expect("load");

        // Edit on disk, then reload via the canonical path (as poll_reloads would report it).
        std::fs::write(&file, r#"[(name: "Goblin", hp: 99, speed: 1.2)]"#).expect("rewrite");
        let canonical = file.canonicalize().unwrap().to_string_lossy().to_string();
        let outcome = reg.reload_path(&canonical);
        assert!(
            matches!(outcome, ReloadOutcome::Reloaded),
            "reload_path must match the canonical path poll_reloads reports; got {outcome:?}"
        );
        let hp = reg.get("enemies").unwrap().get(0, "hp").expect("hp cell");
        assert!(
            matches!(hp, ron::Value::Number(ron::Number::Integer(99))),
            "reloaded table must reflect the on-disk edit, got {hp:?}"
        );
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn add_and_delete_row() {
        let mut table = DataTable::parse(SAMPLE).expect("parse");
        assert_eq!(table.rows.len(), 2);
        table.add_row();
        assert_eq!(table.rows.len(), 3);
        assert!(table.dirty);
        table.delete_row(2);
        assert_eq!(table.rows.len(), 2);
    }

    #[test]
    fn empty_sequence_parses() {
        let table = DataTable::parse("[]").expect("empty parse");
        assert!(table.columns.is_empty());
        assert!(table.rows.is_empty());
    }

    // ── Fix E: reload_path returns SkippedDirty / NotFound correctly ─────────

    /// `reload_path` on a table with `dirty = true` must return `SkippedDirty`
    /// (not reload and not panic).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn reload_path_skipped_when_dirty() {
        let mut reg = DataTableRegistry::default();
        let mut table = DataTable::parse(SAMPLE).expect("parse");
        table.path = "/fake/path.ron".into();
        table.dirty = true;
        reg.insert("enemies", table);

        let outcome = reg.reload_path("/fake/path.ron");
        assert_eq!(outcome, ReloadOutcome::SkippedDirty);
        // Table must still be intact (not cleared).
        assert_eq!(reg.get("enemies").unwrap().rows.len(), 2);
    }

    /// `reload_path` for an unregistered path must return `NotFound`.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn reload_path_not_found() {
        let mut reg = DataTableRegistry::default();
        let outcome = reg.reload_path("/no/such/path.ron");
        assert_eq!(outcome, ReloadOutcome::NotFound);
    }
}
