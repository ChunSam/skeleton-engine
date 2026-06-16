//! Data-driven animation clip sets loaded from RON files.
//!
//! A [`AnimationClipSet`] bundles one or more [`AnimationClip`]s resolved from a
//! spritesheet atlas definition. Clips are identified by name; the set converts
//! frame *indices* into [`UvRect`]s so an [`AnimationPlayer`](crate::animation::AnimationPlayer)
//! can be constructed directly from `set.clips().to_vec()`.
//!
//! # Clip ordering
//! Clips are stored sorted alphabetically by name. This guarantees that
//! [`AnimationClipSet::index`] and [`AnimationClipSet::clips`] return stable,
//! deterministic results regardless of HashMap iteration order or RON map layout.
//! Callers using `AnimationPlayer::play(set.index("idle").unwrap())` will always
//! get the same index for the same RON file.
//!
//! # RON format
//! ```ron
//! (
//!     atlas: (columns: 4, rows: 4),
//!     clips: {
//!         "idle": (frames: [0, 1, 2, 1], fps: 6.0, looping: true),
//!         "run":  (frames: [3, 4, 5, 6], fps: 12.0, looping: true),
//!     },
//! )
//! ```

use std::collections::HashMap;

use serde::Deserialize;

use crate::animation::player::AnimationClip;
use crate::renderer::uv::UvRect;

// ── Private serde def structs ────────────────────────────────────────────────

#[derive(Deserialize)]
struct AtlasDef {
    columns: u32,
    rows: u32,
}

#[derive(Deserialize)]
struct ClipDef {
    frames: Vec<u32>,
    fps: f32,
    looping: bool,
}

#[derive(Deserialize)]
struct AnimationClipSetDef {
    atlas: AtlasDef,
    clips: HashMap<String, ClipDef>,
}

// ── Error type ───────────────────────────────────────────────────────────────

/// Error from parsing or loading an [`AnimationClipSet`].
#[derive(Debug)]
pub enum ClipSetError {
    /// RON parse or deserialization error.
    Ron(String),
    /// Filesystem I/O error (native only).
    #[cfg(not(target_arch = "wasm32"))]
    Io(std::io::Error),
}

impl std::fmt::Display for ClipSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipSetError::Ron(msg) => write!(f, "RON error: {msg}"),
            #[cfg(not(target_arch = "wasm32"))]
            ClipSetError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for ClipSetError {}

#[cfg(not(target_arch = "wasm32"))]
impl From<std::io::Error> for ClipSetError {
    fn from(e: std::io::Error) -> Self {
        ClipSetError::Io(e)
    }
}

// ── AnimationClipSet ─────────────────────────────────────────────────────────

/// A named, ordered set of [`AnimationClip`]s parsed from a RON file.
///
/// Clips are ordered alphabetically by name so that
/// [`index`](Self::index) / [`clips`](Self::clips) indices are deterministic.
/// Pass `set.clips().to_vec()` to [`AnimationPlayer::new`](crate::animation::AnimationPlayer::new)
/// and use `set.index("idle")` to drive playback by name.
#[derive(Debug, Clone)]
pub struct AnimationClipSet {
    /// Clips ordered alphabetically by name.
    clips: Vec<AnimationClip>,
    /// Clip name → index in `clips`.
    names: HashMap<String, usize>,
}

impl AnimationClipSet {
    /// Parse a clip set from RON text.
    ///
    /// Frame indices are resolved to [`UvRect`]s via the embedded atlas grid.
    /// Clips are stored sorted alphabetically by name for deterministic ordering.
    pub fn from_ron_str(s: &str) -> Result<Self, ClipSetError> {
        let def: AnimationClipSetDef =
            ron::from_str(s).map_err(|e| ClipSetError::Ron(e.to_string()))?;

        let columns = def.atlas.columns;
        let rows = def.atlas.rows;

        // Guard against division-by-zero: atlas must have at least one column and one row.
        if columns == 0 || rows == 0 {
            return Err(ClipSetError::Ron(format!(
                "atlas dimensions must be non-zero, got {columns}x{rows}"
            )));
        }

        // Collect clip names and sort for deterministic ordering.
        let mut entries: Vec<(String, ClipDef)> = def.clips.into_iter().collect();
        entries.sort_by(|(a, _), (b, _)| a.cmp(b));

        let mut clips = Vec::with_capacity(entries.len());
        let mut names = HashMap::with_capacity(entries.len());

        for (name, clip_def) in entries {
            // Validate that every frame index lies within the atlas bounds.
            for &i in &clip_def.frames {
                if i >= columns * rows {
                    return Err(ClipSetError::Ron(format!(
                        "frame index {i} exceeds {columns}x{rows} atlas (max index {})",
                        columns * rows - 1,
                    )));
                }
            }
            let frames: Vec<UvRect> = clip_def
                .frames
                .iter()
                .map(|&i| UvRect::from_grid(i % columns, i / columns, columns, rows))
                .collect();
            let index = clips.len();
            clips.push(AnimationClip {
                frames,
                fps: clip_def.fps,
                looping: clip_def.looping,
            });
            names.insert(name, index);
        }

        Ok(AnimationClipSet { clips, names })
    }

    /// Load a clip set from a file path (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(path: &str) -> Result<Self, ClipSetError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_ron_str(&text)
    }

    /// All clips in alphabetical-name order. Pass `.to_vec()` to `AnimationPlayer::new`.
    pub fn clips(&self) -> &[AnimationClip] {
        &self.clips
    }

    /// Index of the named clip (for `AnimationPlayer::play`).
    pub fn index(&self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }

    /// Reference to the named clip.
    pub fn clip(&self, name: &str) -> Option<&AnimationClip> {
        let idx = self.names.get(name).copied()?;
        self.clips.get(idx)
    }

    /// Iterate over all clip names in [`clips`](Self::clips) / [`index`](Self::index) order
    /// (alphabetical), so the name yielded at position `i` is the name of `clips()[i]`.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        let mut ordered: Vec<&str> = vec![""; self.clips.len()];
        for (name, &i) in &self.names {
            if let Some(slot) = ordered.get_mut(i) {
                *slot = name.as_str();
            }
        }
        ordered.into_iter()
    }
}

// ── AnimationClipRegistry ────────────────────────────────────────────────────

/// Registry of named [`AnimationClipSet`]s, stored as a World resource.
///
/// Load sets via [`App::load_animation_clips`](crate::App::load_animation_clips); retrieve them at runtime with [`get`](Self::get).
#[derive(Default)]
pub struct AnimationClipRegistry {
    sets: HashMap<String, AnimationClipSet>,
    /// Source path for each registered set name (native only — used by reload).
    #[cfg(not(target_arch = "wasm32"))]
    paths: HashMap<String, String>,
}

impl AnimationClipRegistry {
    /// Load a clip set from `path` and register it under `name` (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, name: impl Into<String>, path: &str) -> Result<(), ClipSetError> {
        let name = name.into();
        let set = AnimationClipSet::load(path)?;
        self.sets.insert(name.clone(), set);
        self.paths.insert(name, path.to_string());
        Ok(())
    }

    /// Insert a clip set directly (useful for tests or in-memory sets).
    pub fn insert(&mut self, name: impl Into<String>, set: AnimationClipSet) {
        let name = name.into();
        self.sets.insert(name, set);
    }

    /// Look up a clip set by name.
    pub fn get(&self, name: &str) -> Option<&AnimationClipSet> {
        self.sets.get(name)
    }

    /// Re-load the clip set whose source path matches `path` from disk.
    ///
    /// No-op if no set with that path is registered or if file read/parse fails
    /// (logs a warning in that case).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload_path(&mut self, path: &str) {
        // Find the name whose registered path matches.
        let name = self
            .paths
            .iter()
            .find(|(_, p)| {
                // Compare canonicalized paths when possible.
                let a = std::path::Path::new(p.as_str())
                    .canonicalize()
                    .unwrap_or_else(|_| std::path::PathBuf::from(p.as_str()));
                let b = std::path::Path::new(path)
                    .canonicalize()
                    .unwrap_or_else(|_| std::path::PathBuf::from(path));
                a == b
            })
            .map(|(n, _)| n.clone());

        let Some(name) = name else {
            return;
        };

        match AnimationClipSet::load(path) {
            Ok(set) => {
                self.sets.insert(name, set);
            }
            Err(e) => {
                log::warn!("animation_clip: hot-reload failed for {path}: {e}");
            }
        }
    }

    /// Sorted list of registered set names.
    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.sets.keys().cloned().collect();
        v.sort();
        v
    }
}

// ── HotReloadable impl ────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
impl crate::asset::HotReloadable for AnimationClipRegistry {
    fn reload_path(&mut self, path: &str) {
        // Delegate to the inherent method via UFCS to avoid any same-name
        // trait/inherent ambiguity or accidental self-recursion.
        AnimationClipRegistry::reload_path(self, path);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::player::AnimationPlayer;
    use crate::renderer::uv::UvRect;

    const SAMPLE_RON: &str = r#"(
    atlas: (columns: 4, rows: 4),
    clips: {
        "idle": (frames: [0, 1, 2, 1], fps: 6.0, looping: true),
        "run":  (frames: [3, 4, 5, 6], fps: 12.0, looping: true),
    },
)"#;

    #[test]
    fn from_ron_str_parses_two_clips() {
        let set = AnimationClipSet::from_ron_str(SAMPLE_RON).expect("parse");
        // 2 clips total
        assert_eq!(set.clips().len(), 2);

        // idle clip
        let idle_idx = set.index("idle").expect("idle index");
        let idle = set.clip("idle").expect("idle clip");
        assert_eq!(idle.frames.len(), 4);
        assert!((idle.fps - 6.0).abs() < 1e-6, "idle fps");
        assert!(idle.looping, "idle looping");
        assert_eq!(idle_idx, set.index("idle").unwrap());

        // run clip
        let run_idx = set.index("run").expect("run index");
        let run = set.clip("run").expect("run clip");
        assert_eq!(run.frames.len(), 4);
        assert!((run.fps - 12.0).abs() < 1e-6, "run fps");
        assert!(run.looping, "run looping");
        // idle and run must have different indices
        assert_ne!(idle_idx, run_idx);
    }

    /// Frame 6 in a 4×4 atlas → col=2, row=1.
    /// atlas index 6: col = 6 % 4 = 2, row = 6 / 4 = 1
    #[test]
    fn frame_uv_conversion_correct() {
        let set = AnimationClipSet::from_ron_str(SAMPLE_RON).expect("parse");
        let run = set.clip("run").expect("run");
        // run frames: [3, 4, 5, 6] → indices 0..3
        // frame[3] = atlas index 6 → UvRect::from_grid(2, 1, 4, 4)
        let expected = UvRect::from_grid(2, 1, 4, 4);
        assert_eq!(
            run.frames[3], expected,
            "atlas index 6 should map to col=2, row=1"
        );
    }

    /// Clips are ordered alphabetically: "idle" < "run" → indices 0 and 1.
    #[test]
    fn names_are_in_clip_order() {
        // names() must align with clips()/index() order (alphabetical), not HashMap order,
        // so `names().nth(i)` is the name of `clips()[i]`. (Regression: a playtest HUD showed
        // the wrong clip name because names() iterated the underlying HashMap.)
        let set = AnimationClipSet::from_ron_str(SAMPLE_RON).expect("parse");
        let names: Vec<&str> = set.names().collect();
        assert_eq!(names, vec!["idle", "run"]);
        assert_eq!(set.index(names[0]), Some(0));
        assert_eq!(set.index(names[1]), Some(1));
    }

    #[test]
    fn clip_order_is_alphabetical() {
        let set = AnimationClipSet::from_ron_str(SAMPLE_RON).expect("parse");
        assert_eq!(set.index("idle"), Some(0), "idle is first alphabetically");
        assert_eq!(set.index("run"), Some(1), "run is second alphabetically");
        // clips() slice matches same order
        assert!(
            (set.clips()[0].fps - 6.0).abs() < 1e-6,
            "clips[0] is idle (fps=6)"
        );
        assert!(
            (set.clips()[1].fps - 12.0).abs() < 1e-6,
            "clips[1] is run (fps=12)"
        );
    }

    #[test]
    fn malformed_ron_returns_err() {
        let result = AnimationClipSet::from_ron_str("not valid ron {{{{");
        assert!(result.is_err(), "malformed RON must return Err");
    }

    /// Atlas with columns=0 must return a Ron error (guards against i % 0 panic).
    #[test]
    fn zero_columns_returns_err() {
        let ron = r#"(
    atlas: (columns: 0, rows: 4),
    clips: {
        "idle": (frames: [0], fps: 6.0, looping: true),
    },
)"#;
        let result = AnimationClipSet::from_ron_str(ron);
        assert!(
            result.is_err(),
            "columns=0 atlas must return Err, not panic"
        );
        match result {
            Err(ClipSetError::Ron(msg)) => assert!(
                msg.contains("non-zero"),
                "error message should mention non-zero, got: {msg}"
            ),
            other => panic!("expected ClipSetError::Ron, got {other:?}"),
        }
    }

    /// Atlas with rows=0 must also return a Ron error.
    #[test]
    fn zero_rows_returns_err() {
        let ron = r#"(
    atlas: (columns: 4, rows: 0),
    clips: {
        "idle": (frames: [0], fps: 6.0, looping: true),
    },
)"#;
        let result = AnimationClipSet::from_ron_str(ron);
        assert!(result.is_err(), "rows=0 atlas must return Err, not panic");
    }

    /// A frame index that exceeds the atlas capacity must return a Ron error.
    #[test]
    fn out_of_range_frame_index_returns_err() {
        // 2×2 atlas has 4 cells (indices 0–3); frame index 4 is out of range.
        let ron = r#"(
    atlas: (columns: 2, rows: 2),
    clips: {
        "bad": (frames: [0, 1, 4], fps: 10.0, looping: false),
    },
)"#;
        let result = AnimationClipSet::from_ron_str(ron);
        assert!(result.is_err(), "out-of-range frame index must return Err");
        match result {
            Err(ClipSetError::Ron(msg)) => assert!(
                msg.contains("frame index 4"),
                "error message should mention the bad index, got: {msg}"
            ),
            other => panic!("expected ClipSetError::Ron, got {other:?}"),
        }
    }

    /// Valid RON with every frame in range must still succeed.
    #[test]
    fn valid_ron_still_ok_after_validation() {
        let result = AnimationClipSet::from_ron_str(SAMPLE_RON);
        assert!(result.is_ok(), "valid RON must still parse successfully");
    }

    /// Registry load+get round-trip and reload with in-memory substitute.
    #[test]
    fn registry_insert_and_get() {
        let set = AnimationClipSet::from_ron_str(SAMPLE_RON).expect("parse");
        let mut reg = AnimationClipRegistry::default();
        reg.insert("sprites", set);

        let got = reg.get("sprites").expect("get sprites");
        assert_eq!(got.clips().len(), 2);
        assert!(reg.get("missing").is_none());
    }

    /// Replacing an existing set in the registry via insert (simulates reload).
    #[test]
    fn registry_reload_via_insert_replaces_set() {
        let set1 = AnimationClipSet::from_ron_str(SAMPLE_RON).expect("parse set1");
        let mut reg = AnimationClipRegistry::default();
        reg.insert("sprites", set1);

        // Simulate a "reload" by inserting a new set (1 clip only)
        let minimal_ron = r#"(
    atlas: (columns: 2, rows: 1),
    clips: {
        "walk": (frames: [0, 1], fps: 8.0, looping: true),
    },
)"#;
        let set2 = AnimationClipSet::from_ron_str(minimal_ron).expect("parse set2");
        reg.insert("sprites", set2);

        let got = reg.get("sprites").expect("get after reload");
        assert_eq!(got.clips().len(), 1, "replaced set has 1 clip");
        assert!(got.clip("walk").is_some(), "replaced set has 'walk' clip");
    }

    /// AnimationPlayer can be driven from a clip set by name.
    #[test]
    fn animation_player_driven_by_clip_set() {
        let set = AnimationClipSet::from_ron_str(SAMPLE_RON).expect("parse");
        let mut player = AnimationPlayer::new(set.clips().to_vec());
        let run_idx = set.index("run").expect("run index");
        player.play(run_idx);

        // First frame of "run" is atlas index 3 in a 4×4 grid → col=3, row=0
        let expected_first = UvRect::from_grid(3, 0, 4, 4);
        assert_eq!(player.current_uv(), expected_first, "first frame of run");
    }
}
