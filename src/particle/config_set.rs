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
//!                   size:        (6.0, 6.0),
//!                   gravity:     (0.0, 200.0)),
//!         "smoke": (spawn_rate: 15.0, lifetime: 2.0,
//!                   emit_shape: Circle(radius: 8.0)),
//!     },
//! )
//! ```
//! All fields except the map key are optional and default to sensible values
//! (see [`EmitterDef`] defaults).  New optional fields: `gravity: (x, y)` applies
//! per-particle constant acceleration (pixels/s²; default `(0.0, 0.0)`) and
//! `emit_shape` controls spawn scatter — `Point` (default), `Circle(radius: r)`,
//! `Ring(radius: r)`, or `Box(half_extents: (w, h))`.
//! Use [`ParticleConfigSet::emitter`] to obtain a fresh [`super::ParticleEmitter`]
//! pre-filled from the config.
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
#[cfg(not(target_arch = "wasm32"))]
use crate::gpu_particle::GpuParticleEmitter;

// ── Private serde mirror types ────────────────────────────────────────────────

/// Serde-only tuple for [`Vec2`]: `(x, y)`.
#[derive(Deserialize, Clone, Copy, Debug, Default)]
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

/// Serde-only mirror of [`super::EmitShape`]. RON: `Point` / `Circle(radius: 12.0)` /
/// `Ring(radius: 20.0)` / `Box(half_extents: (10.0, 4.0))`.
#[derive(Deserialize, Clone, Copy, Debug, Default)]
enum EmitShapeDef {
    #[default]
    Point,
    Circle {
        radius: f32,
    },
    Ring {
        radius: f32,
    },
    Box {
        half_extents: Vec2Def,
    },
}

impl From<EmitShapeDef> for super::EmitShape {
    fn from(s: EmitShapeDef) -> super::EmitShape {
        match s {
            EmitShapeDef::Point => super::EmitShape::Point,
            EmitShapeDef::Circle { radius } => super::EmitShape::Circle { radius },
            EmitShapeDef::Ring { radius } => super::EmitShape::Ring { radius },
            EmitShapeDef::Box { half_extents } => super::EmitShape::Box {
                half_extents: half_extents.into(),
            },
        }
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
    /// Per-particle constant acceleration (pixels/s²). Defaults to zero.
    #[serde(default)]
    gravity: Vec2Def,
    /// Spawn scatter shape. Defaults to `Point`.
    #[serde(default)]
    emit_shape: EmitShapeDef,
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
/// # use engine::particle::{ParticleConfigSet, EmitShape};
/// # use glam::Vec2;
/// let ron = r#"(emitters: {
///     "fire":     (spawn_rate: 60.0, gravity: (0.0, 200.0)),
///     "fountain": (spawn_rate: 70.0, emit_shape: Circle(radius: 8.0)),
/// })"#;
/// let set = ParticleConfigSet::from_ron_str(ron).unwrap();
/// let fire = set.emitter("fire").unwrap();
/// assert_eq!(fire.spawn_rate, 60.0);
/// assert_eq!(fire.gravity, Vec2::new(0.0, 200.0));
/// let fountain = set.emitter("fountain").unwrap();
/// assert_eq!(fountain.emit_shape, EmitShape::Circle { radius: 8.0 });
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
            gravity: def.gravity.into(),
            emit_shape: def.emit_shape.into(),
            texture: def.texture.as_deref().map(std::sync::Arc::from),
            z: def.z,
            emit: def.emit,
            timer: 0.0,
        })
    }

    /// Returns a fresh [`GpuParticleEmitter`] built from the named config, or `None` if no
    /// emitter with that name exists — the GPU-emitter counterpart to [`emitter`](Self::emitter),
    /// so a single RON file can drive either the CPU or the compute-shader particle path.
    ///
    /// The nine fields shared with the CPU emitter (`spawn_rate`, `lifetime`, `velocity`,
    /// `velocity_spread`, `color_start`, `color_end`, `gravity`, `emit_shape`, `emit`) map 1:1.
    /// Two differences from [`emitter`](Self::emitter): a GPU particle is square, so the config's
    /// `size` `(w, h)` pair contributes its **width** (`size.0`) as the scalar GPU size; and
    /// `texture` / `z` have no `GpuParticleEmitter` equivalent and are ignored.
    ///
    /// Native-only: `GpuParticleEmitter` (and the GPU compute path) does not exist on wasm —
    /// use [`emitter`](Self::emitter) (the CPU path) there.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn gpu_emitter(&self, name: &str) -> Option<GpuParticleEmitter> {
        let idx = self.names.iter().position(|n| n == name)?;
        let def = &self.configs[idx];
        let size: Vec2 = def.size.into();
        // Functional update: GpuParticleEmitter's private timer/next_slot come from Default
        // (a plain default()+field-assignment trips clippy::field_reassign_with_default).
        Some(GpuParticleEmitter {
            spawn_rate: def.spawn_rate,
            lifetime: def.lifetime,
            velocity: def.velocity.into(),
            velocity_spread: def.velocity_spread.into(),
            color_start: def.color_start.into(),
            color_end: def.color_end.into(),
            size: size.x, // GPU particles are square → use the config width
            gravity: def.gravity.into(),
            emit_shape: def.emit_shape.into(),
            emit: def.emit,
            ..Default::default()
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
    inner: crate::ron_registry::RonRegistry<ParticleConfigSet>,
}

impl ParticleConfigRegistry {
    /// Load a [`ParticleConfigSet`] from `path` and register it under `name`.
    ///
    /// Native-only. Excluded from wasm builds.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, name: impl Into<String>, path: &str) -> Result<(), ParticleConfigError> {
        self.inner.load(name, path)
    }

    /// Insert a config set directly (useful for tests or in-memory sets).
    pub fn insert(&mut self, name: impl Into<String>, set: ParticleConfigSet) {
        self.inner.insert(name, set);
    }

    /// Look up a [`ParticleConfigSet`] by name.
    pub fn get(&self, name: &str) -> Option<&ParticleConfigSet> {
        self.inner.get(name)
    }

    /// Re-read the file whose registered path matches `path` (native only).
    ///
    /// Called automatically by [`crate::App`] on every hot-reload tick. If no
    /// set is registered for this path the call is a no-op.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload_path(&mut self, path: &str) {
        self.inner.reload_path(path, "particle_config");
    }

    /// Sorted list of registered config-set names.
    pub fn names(&self) -> Vec<String> {
        self.inner.names()
    }
}

// ── RonLoadable + HotReloadable impls ─────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
impl crate::ron_registry::RonLoadable for ParticleConfigSet {
    type Err = ParticleConfigError;
    fn load_ron(path: &str) -> Result<Self, Self::Err> {
        let text = std::fs::read_to_string(path)?;
        Self::from_ron_str(&text)
    }
}

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

    /// `gpu_emitter` mirrors the CPU emitter: shared fields map 1:1, the square GPU `size`
    /// uses the config width, `gravity`/`emit_shape` carry over, and a missing name → `None`.
    #[test]
    fn gpu_emitter_maps_fields_and_uses_width() {
        let ron = r#"(emitters: {
            "spark": (spawn_rate: 90.0, lifetime: 1.5, velocity: (0.0, -60.0),
                      velocity_spread: (40.0, 30.0),
                      color_start: (1.0, 0.7, 0.1, 1.0),
                      color_end:   (1.0, 0.1, 0.0, 0.0),
                      size: (7.0, 3.0),
                      gravity: (0.0, 140.0),
                      emit_shape: Circle(radius: 12.0)),
        })"#;
        let set = ParticleConfigSet::from_ron_str(ron).expect("parse");
        let e = set.gpu_emitter("spark").expect("gpu emitter");
        assert_eq!(e.spawn_rate, 90.0);
        assert_eq!(e.lifetime, 1.5);
        assert_eq!(e.velocity, Vec2::new(0.0, -60.0));
        assert_eq!(e.color_start, Color::rgba(1.0, 0.7, 0.1, 1.0));
        assert_eq!(
            e.size, 7.0,
            "square GPU particle uses the config width (size.0)"
        );
        assert_eq!(e.gravity, Vec2::new(0.0, 140.0));
        assert_eq!(
            e.emit_shape,
            crate::particle::EmitShape::Circle { radius: 12.0 }
        );
        assert!(e.emit);
        assert!(set.gpu_emitter("nope").is_none());
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
        // Omitting gravity/emit_shape must yield zero gravity and Point shape.
        assert_eq!(spark.gravity, Vec2::ZERO);
        assert_eq!(spark.emit_shape, super::super::EmitShape::Point);
    }

    /// `gravity` field parses and converts correctly.
    #[test]
    fn gravity_field_parses_correctly() {
        let ron = r#"(emitters: { "sparks": (gravity: (0.0, 200.0)) })"#;
        let set = ParticleConfigSet::from_ron_str(ron).expect("parse");
        let sparks = set.emitter("sparks").expect("sparks emitter");
        assert_eq!(sparks.gravity, Vec2::new(0.0, 200.0));
    }

    /// `emit_shape: Circle(radius: ...)` parses correctly.
    #[test]
    fn emit_shape_circle_parses_correctly() {
        let ron = r#"(emitters: { "burst": (emit_shape: Circle(radius: 12.0)) })"#;
        let set = ParticleConfigSet::from_ron_str(ron).expect("parse");
        let burst = set.emitter("burst").expect("burst emitter");
        assert_eq!(
            burst.emit_shape,
            super::super::EmitShape::Circle { radius: 12.0 }
        );
    }

    /// `emit_shape: Box(half_extents: ...)` parses and converts correctly.
    #[test]
    fn emit_shape_box_parses_correctly() {
        let ron = r#"(emitters: { "area": (emit_shape: Box(half_extents: (10.0, 4.0))) })"#;
        let set = ParticleConfigSet::from_ron_str(ron).expect("parse");
        let area = set.emitter("area").expect("area emitter");
        assert_eq!(
            area.emit_shape,
            super::super::EmitShape::Box {
                half_extents: Vec2::new(10.0, 4.0)
            }
        );
    }

    /// Registry load + get round-trip (in-memory via from_ron_str, cross-platform).
    #[test]
    fn registry_insert_and_get_round_trip() {
        let set = ParticleConfigSet::from_ron_str(FIRE_SMOKE).expect("parse");
        let mut reg = ParticleConfigRegistry::default();
        // In-memory insert (no disk), exercising the public API.
        reg.insert("vfx", set);

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
