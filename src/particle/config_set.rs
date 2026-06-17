//! Data-driven particle emitter configs: load [`ParticleConfigSet`] from a RON file
//! with hot-reload support (native only).
//!
//! # File format
//! ```ron
//! (
//!     emitters: {
//!         "fire":  (spawn_rate: 60.0, lifetime: 0.8, velocity: (0.0, -40.0),
//!                   velocity_spread: (30.0, 20.0),
//!                   color_start: (1.0, 0.85, 0.35, 1.0),
//!                   color_end:   (1.0, 0.25, 0.1,  0.0),
//!                   size:        (6.0, 6.0)),
//!         "smoke": (spawn_rate: 15.0, lifetime: 2.0),
//!     },
//! )
//! ```
//! All fields except the map key are optional and default to sensible values
//! (see [`EmitterDef`] defaults).  Use [`ParticleConfigSet::emitter`] to obtain
//! a fresh [`super::ParticleEmitter`] pre-filled from the config.
//!
//! # Hot reload
//! On native builds, call [`crate::App::load_particle_configs`] instead of
//! constructing the registry directly.  The engine wires up the filesystem watcher
//! and calls [`ParticleConfigRegistry::reload_path`] automatically each frame.

use std::collections::HashMap;

use serde::Deserialize;

use crate::color::Color;
use glam::Vec2;

use super::ParticleEmitter;

// ── Private serde mirror types ────────────────────────────────────────────────

/// Serde-only tuple for [`Vec2`]: `(x, y)`.
#[derive(Deserialize, Clone, Copy, Debug)]
struct Vec2Def(f32, f32);

impl From<Vec2Def> for Vec2 {
    fn from(v: Vec2Def) -> Vec2 {
        Vec2::new(v.0, v.1)
    }
}

/// Serde-only tuple for [`Color`]: `(r, g, b, a)`.
#[derive(Deserialize, Clone, Copy, Debug)]
struct ColorDef(f32, f32, f32, f32);

impl From<ColorDef> for Color {
    fn from(c: ColorDef) -> Color {
        Color::rgba(c.0, c.1, c.2, c.3)
    }
}

/// Per-emitter definition inside the RON file.
///
/// All fields are optional and fall back to [`ParticleEmitter::default`] values.
#[derive(Deserialize, Clone, Debug)]
struct EmitterDef {
    #[serde(default = "default_spawn_rate")]
    spawn_rate: f32,
    #[serde(default = "default_lifetime")]
    lifetime: f32,
    #[serde(default = "default_velocity")]
    velocity: Vec2Def,
    #[serde(default = "default_velocity_spread")]
    velocity_spread: Vec2Def,
    #[serde(default = "default_color_start")]
    color_start: ColorDef,
    #[serde(default = "default_color_end")]
    color_end: ColorDef,
    #[serde(default = "default_size")]
    size: Vec2Def,
    /// Continuous emission enabled. Defaults to `true`.
    #[serde(default = "default_emit")]
    emit: bool,
    /// Texture path. `None` renders a solid-color rectangle.
    #[serde(default)]
    texture: Option<String>,
    /// Z-depth of spawned particles. Defaults to `0.0`.
    #[serde(default)]
    z: f32,
}

// ── Serde defaults mirroring `ParticleEmitter::default()` ────────────────────

fn default_spawn_rate() -> f32 {
    20.0
}
fn default_lifetime() -> f32 {
    1.0
}
fn default_velocity() -> Vec2Def {
    Vec2Def(0.0, -50.0)
}
fn default_velocity_spread() -> Vec2Def {
    Vec2Def(20.0, 10.0)
}
fn default_color_start() -> ColorDef {
    ColorDef(1.0, 1.0, 1.0, 1.0)
}
fn default_color_end() -> ColorDef {
    ColorDef(1.0, 1.0, 1.0, 0.0)
}
fn default_size() -> Vec2Def {
    Vec2Def(8.0, 8.0)
}
fn default_emit() -> bool {
    true
}

// ── Top-level RON document ────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct ParticleConfigDoc {
    emitters: HashMap<String, EmitterDef>,
}

// ── Public error type ─────────────────────────────────────────────────────────

/// Error returned by [`ParticleConfigSet::from_ron_str`] and
/// [`ParticleConfigRegistry::load`].
///
/// This is a type alias for [`crate::asset::AssetLoadError`], which is shared
/// with [`crate::animation::ClipSetError`]. The alias keeps the public name
/// stable (non-breaking) while eliminating duplicated error boilerplate.
///
/// **Note:** the `Display` output changed from `"RON parse error: …"` to
/// `"RON error: …"` as part of this unification (cosmetic only).
pub type ParticleConfigError = crate::asset::AssetLoadError;

// ── ParticleConfigSet ─────────────────────────────────────────────────────────

/// A named set of particle emitter configs loaded from a single RON file.
///
/// Obtain via [`ParticleConfigSet::from_ron_str`] (cross-platform) or through
/// [`ParticleConfigRegistry`] / [`crate::App::load_particle_configs`] (native,
/// with hot-reload).
///
/// ```rust,no_run
/// # use engine::particle::{ParticleConfigSet};
/// let ron = r#"(emitters: { "fire": (spawn_rate: 60.0) })"#;
/// let set = ParticleConfigSet::from_ron_str(ron).unwrap();
/// let emitter = set.emitter("fire").unwrap();
/// assert_eq!(emitter.spawn_rate, 60.0);
/// ```
pub struct ParticleConfigSet {
    /// Alphabetically sorted emitter names — deterministic order guaranteed.
    names: Vec<String>,
    /// Emitter configs aligned with `names` (same index).
    configs: Vec<EmitterDef>,
}

impl ParticleConfigSet {
    /// Parse a [`ParticleConfigSet`] from a RON string.
    ///
    /// Returns `Err` on malformed input rather than panicking.
    pub fn from_ron_str(s: &str) -> Result<Self, ParticleConfigError> {
        let doc: ParticleConfigDoc =
            ron::from_str(s).map_err(|e| ParticleConfigError::Ron(e.to_string()))?;

        // Sort alphabetically for deterministic `names()` output.
        let mut pairs: Vec<(String, EmitterDef)> = doc.emitters.into_iter().collect();
        pairs.sort_by(|(a, _), (b, _)| a.cmp(b));

        let (names, configs): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();

        Ok(ParticleConfigSet { names, configs })
    }

    /// Returns a fresh [`ParticleEmitter`] built from the named config, or `None`
    /// if no emitter with that name exists.
    ///
    /// The returned emitter is an owned value — mutate it freely without affecting
    /// the registry entry.
    pub fn emitter(&self, name: &str) -> Option<ParticleEmitter> {
        let idx = self.names.iter().position(|n| n == name)?;
        let def = &self.configs[idx];
        Some(ParticleEmitter {
            spawn_rate: def.spawn_rate,
            lifetime: def.lifetime,
            velocity: def.velocity.into(),
            velocity_spread: def.velocity_spread.into(),
            color_start: def.color_start.into(),
            color_end: def.color_end.into(),
            size: def.size.into(),
            gravity: glam::Vec2::ZERO,
            emit_shape: crate::particle::EmitShape::Point,
            texture: def.texture.as_deref().map(std::sync::Arc::from),
            z: def.z,
            emit: def.emit,
            timer: 0.0,
        })
    }

    /// Iterate emitter names in **alphabetical order** (deterministic).
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }
}

// ── ParticleConfigRegistry ────────────────────────────────────────────────────

/// Registry of named [`ParticleConfigSet`]s, stored as a World resource.
///
/// On native builds, load configs via [`crate::App::load_particle_configs`] which
/// wires up the filesystem watcher for hot-reload. On wasm, use
/// [`ParticleConfigSet::from_ron_str`] directly.
///
/// # Example
/// ```rust,no_run
/// # use engine::App;
/// let mut app = App::new();
/// app.load_particle_configs("vfx", "assets/particles.ron");
/// // Later in a system:
/// // let reg = world.resource::<engine::particle::ParticleConfigRegistry>().unwrap();
/// // let emitter = reg.get("vfx").unwrap().emitter("fire").unwrap();
/// ```
#[derive(Default)]
pub struct ParticleConfigRegistry {
    sets: HashMap<String, ParticleConfigSet>,
    /// Source paths for each registered set (native only, for reload lookup).
    #[cfg(not(target_arch = "wasm32"))]
    paths: HashMap<String, String>, // name -> path
}

impl ParticleConfigRegistry {
    /// Load a [`ParticleConfigSet`] from `path` and register it under `name`.
    ///
    /// Native-only. Excluded from wasm builds.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, name: impl Into<String>, path: &str) -> Result<(), ParticleConfigError> {
        let text = std::fs::read_to_string(path)?;
        let set = ParticleConfigSet::from_ron_str(&text)?;
        let name = name.into();
        // Store the canonicalized key so `reload_path` matches the path that
        // `AssetServer::poll_reloads` reports (it returns `asset_key`, which canonicalizes).
        self.paths
            .insert(name.clone(), crate::asset::asset_key(path).to_string());
        self.sets.insert(name, set);
        Ok(())
    }

    /// Look up a [`ParticleConfigSet`] by name.
    pub fn get(&self, name: &str) -> Option<&ParticleConfigSet> {
        self.sets.get(name)
    }

    /// Re-read the file whose registered path matches `path` (native only).
    ///
    /// Called automatically by [`crate::App`] on every hot-reload tick. If no
    /// set is registered for this path the call is a no-op.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload_path(&mut self, path: &str) {
        // Find the name for this path.
        let name = self
            .paths
            .iter()
            .find(|(_, p)| p.as_str() == path)
            .map(|(n, _)| n.clone());
        let Some(name) = name else { return };

        match std::fs::read_to_string(path).and_then(|text| {
            ParticleConfigSet::from_ron_str(&text)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
        }) {
            Ok(set) => {
                self.sets.insert(name, set);
            }
            Err(e) => {
                log::warn!("particle_config: hot-reload failed for {path}: {e}");
            }
        }
    }

    /// Sorted list of registered config-set names.
    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.sets.keys().cloned().collect();
        v.sort();
        v
    }
}

// ── HotReloadable impl ────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
impl crate::asset::HotReloadable for ParticleConfigRegistry {
    fn reload_path(&mut self, path: &str) {
        // Delegate to the inherent method via UFCS to avoid any same-name
        // trait/inherent ambiguity or accidental self-recursion.
        ParticleConfigRegistry::reload_path(self, path);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const FIRE_SMOKE: &str = r#"(
    emitters: {
        "fire":  (spawn_rate: 60.0, lifetime: 0.8, velocity: (0.0, -40.0),
                  velocity_spread: (30.0, 20.0),
                  color_start: (1.0, 0.85, 0.35, 1.0),
                  color_end:   (1.0, 0.25, 0.1,  0.0),
                  size:        (6.0, 6.0)),
        "smoke": (spawn_rate: 15.0, lifetime: 2.5, velocity: (5.0, -20.0),
                  color_start: (0.6, 0.6, 0.6, 0.8),
                  color_end:   (0.3, 0.3, 0.3, 0.0)),
    },
)"#;

    /// `from_ron_str` parses 2 emitters; named access returns correct fields.
    #[test]
    fn parse_two_emitters_and_access_by_name() {
        let set = ParticleConfigSet::from_ron_str(FIRE_SMOKE).expect("parse");
        let fire = set.emitter("fire").expect("fire emitter");
        assert_eq!(fire.spawn_rate, 60.0);
        assert_eq!(fire.lifetime, 0.8);
        assert_eq!(fire.size, Vec2::new(6.0, 6.0));
        assert!(set.emitter("nope").is_none());
    }

    /// `names()` is alphabetical and deterministic.
    #[test]
    fn names_are_alphabetical_and_deterministic() {
        let set = ParticleConfigSet::from_ron_str(FIRE_SMOKE).expect("parse");
        let names: Vec<&str> = set.names().collect();
        // "fire" < "smoke" alphabetically
        assert_eq!(names, vec!["fire", "smoke"]);
    }

    /// Vec2 and Color fields convert correctly.
    #[test]
    fn vec2_and_color_fields_convert_correctly() {
        let set = ParticleConfigSet::from_ron_str(FIRE_SMOKE).expect("parse");
        let fire = set.emitter("fire").expect("fire emitter");
        assert_eq!(fire.color_start, Color::rgba(1.0, 0.85, 0.35, 1.0));
        assert_eq!(fire.color_end, Color::rgba(1.0, 0.25, 0.1, 0.0));
        assert_eq!(fire.velocity, Vec2::new(0.0, -40.0));
        assert_eq!(fire.velocity_spread, Vec2::new(30.0, 20.0));
    }

    /// Malformed RON returns Err, not panic.
    #[test]
    fn malformed_ron_returns_err() {
        let result = ParticleConfigSet::from_ron_str("this is not ron at all {{{{");
        assert!(result.is_err(), "malformed RON must return Err");
    }

    /// Minimal RON (only required field = the map key) uses serde defaults.
    #[test]
    fn minimal_ron_uses_serde_defaults() {
        let ron = r#"(emitters: { "spark": () })"#;
        let set = ParticleConfigSet::from_ron_str(ron).expect("parse minimal");
        let spark = set.emitter("spark").expect("spark emitter");
        // Check defaults match ParticleEmitter::default()
        assert_eq!(spark.spawn_rate, 20.0);
        assert_eq!(spark.lifetime, 1.0);
        assert_eq!(spark.velocity, Vec2::new(0.0, -50.0));
        assert_eq!(spark.size, Vec2::new(8.0, 8.0));
        assert!(spark.emit);
        assert!(spark.texture.is_none());
    }

    /// Registry load + get round-trip (in-memory via from_ron_str, cross-platform).
    #[test]
    fn registry_insert_and_get_round_trip() {
        let set = ParticleConfigSet::from_ron_str(FIRE_SMOKE).expect("parse");
        let mut reg = ParticleConfigRegistry::default();
        // Manually insert (mimics what `load` does on native without touching disk).
        reg.sets.insert("vfx".to_string(), set);
        #[cfg(not(target_arch = "wasm32"))]
        reg.paths
            .insert("vfx".to_string(), "/fake/particles.ron".to_string());

        let got = reg.get("vfx").expect("get vfx");
        assert!(got.emitter("fire").is_some());
        assert!(got.emitter("smoke").is_some());
        assert!(reg.get("missing").is_none());
    }

    // Regression: a playtest found hot-reload silently no-op'd because `load` stored the
    // raw path while `reload_path` is called with the canonicalized path that
    // `AssetServer::poll_reloads` reports — so the name lookup never matched. `load` now
    // stores the canonical key.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn reload_path_matches_canonical_key_from_poll_reloads() {
        let path = std::env::temp_dir().join(format!("pcs_reload_{}.ron", std::process::id()));
        let p = path.to_string_lossy().to_string();
        std::fs::write(&path, r#"(emitters: {"a": (spawn_rate: 10.0)})"#).expect("write");

        let mut reg = ParticleConfigRegistry::default();
        reg.load("set", &p).expect("load");
        assert_eq!(
            reg.get("set").unwrap().emitter("a").unwrap().spawn_rate,
            10.0
        );

        // Edit on disk, then reload via the CANONICAL path (as poll_reloads would report it).
        std::fs::write(&path, r#"(emitters: {"a": (spawn_rate: 99.0)})"#).expect("rewrite");
        let canonical = crate::asset::asset_key(&p).to_string();
        reg.reload_path(&canonical);
        assert_eq!(
            reg.get("set").unwrap().emitter("a").unwrap().spawn_rate,
            99.0,
            "reload_path must match the canonical path poll_reloads reports"
        );
        let _ = std::fs::remove_file(&path);
    }
}
