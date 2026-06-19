//! Scene serialization + prefab system (Phase 16)
//!
//! # Core types
//! - [`Tag`] — string component for entity identification
//! - [`EntityDef`] — serializable struct describing a single entity
//! - [`SceneDef`] — collection of [`EntityDef`]s (a full level/scene)
//! - [`Prefab`] — file-backed single-entity template
//!
//! # Quick usage example
//! ```rust,no_run
//! use engine::prefab::{SceneDef, EntityDef, spawn_scene_def};
//! use engine::{Transform, Sprite, ecs::World};
//! use glam::Vec2;
//! use std::path::Path;
//!
//! let mut world = World::new();
//!
//! // Build the scene definition
//! let scene = SceneDef {
//!     entities: vec![
//!         EntityDef {
//!             tag: Some("player".into()),
//!             transform: Some(Transform::new(Vec2::ZERO, Vec2::splat(64.0), 0.0)),
//!             sprite: Some(Sprite::textured("assets/player.png")),
//!             ..EntityDef::default()
//!         },
//!     ],
//!     ..SceneDef::default()
//! };
//!
//! // Save to file then load
//! scene.save(Path::new("levels/level1.ron")).unwrap();
//! let loaded = SceneDef::load(Path::new("levels/level1.ron")).unwrap();
//! let entities = spawn_scene_def(&mut world, &loaded);
//! ```

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::components::{Sprite, Transform};
use crate::ecs::{Entity, World};
use crate::reflect::{Reflect, ReflectValue};
use crate::save::{read_ron, write_ron, SaveError};

// ─── Tag component ────────────────────────────────────────────────────────────

/// String tag component for entity identification.
///
/// Use it to distinguish roles such as "player" or "enemy" after level load,
/// or to query for a specific entity.
///
/// # Example
/// ```rust,no_run
/// # use engine::prefab::Tag;
/// # use engine::ecs::World;
/// # let mut world = engine::ecs::World::new();
/// let e = world.spawn();
/// world.add_component(e, Tag("player".into()));
///
/// // Find it later
/// for (entity, tag) in world.query::<Tag>() {
///     if tag.0 == "player" { /* ... */ }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag(pub String);

impl Reflect for Tag {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![("tag", ReflectValue::String(self.0.clone()))]
    }
    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("tag", ReflectValue::String(s)) => {
                self.0 = s;
                true
            }
            _ => false,
        }
    }
    fn type_name(&self) -> &'static str {
        "Tag"
    }
}

// ─── EntityDef ────────────────────────────────────────────────────────────────

/// Serializable struct describing a single entity.
///
/// Every field is `Option`, so only the components you need must be specified.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityDef {
    /// Entity identification tag (optional)
    pub tag: Option<String>,
    /// Position, size, and rotation (optional)
    pub transform: Option<Transform>,
    /// Texture and color (optional)
    pub sprite: Option<Sprite>,
    /// Tag string of the parent entity. `None` means a root entity.
    /// On spawn, the entity is attached as a child of the entity with this tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Arbitrary serializable components, keyed by type name.
    /// Populated by the editor on save; applied by `spawn_entity_def` via the
    /// [`SerdeComponentRegistry`] resource.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub components: HashMap<String, ron::Value>,
}

// ─── SerdeComponentRegistry ───────────────────────────────────────────────────
// Moved to `src/serde_registry.rs`; re-exported here so the public path
// `engine::prefab::{SerdeComponentEntry, SerdeComponentRegistry}` remains stable.

pub use crate::serde_registry::{SerdeComponentEntry, SerdeComponentRegistry};

// ─── SceneDef ─────────────────────────────────────────────────────────────────

/// Current RON format version for `SceneDef`. Increment on structural changes.
pub const SCENE_DEF_VERSION: u32 = 3;

/// Serializable struct describing an entire level/scene.
///
/// One RON file corresponds to one `SceneDef`.
///
/// # RON example
/// ```ron
/// SceneDef(
///     version: 2,
///     entities: [
///         EntityDef(
///             tag: Some("ground"),
///             transform: Some(Transform(
///                 position: (0.0, -200.0),
///                 scale: (800.0, 32.0),
///                 rotation: 0.0,
///                 z: 0.0,
///             )),
///             sprite: Some(Sprite(
///                 texture: None,
///                 color: (r: 0.3, g: 0.6, b: 0.3, a: 1.0),
///             )),
///         ),
///     ],
/// )
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDef {
    /// File format version. Old files without a version field deserialize as 0.
    #[serde(default)]
    pub version: u32,
    pub entities: Vec<EntityDef>,
}

impl Default for SceneDef {
    fn default() -> Self {
        Self {
            version: SCENE_DEF_VERSION,
            entities: Vec::new(),
        }
    }
}

impl SceneDef {
    /// Loads a scene definition from a plain-text RON file.
    ///
    /// If the file version differs from the current version, a warning is emitted but loading
    /// continues. Files written by the engine before v4.6 (AEAD-encrypted binary format) are
    /// detected automatically and decrypted transparently for backward compatibility.
    pub fn load(path: &Path) -> Result<Self, SaveError> {
        let scene: SceneDef = read_ron(path)?;
        if scene.version != SCENE_DEF_VERSION {
            log::warn!(
                "scene file version mismatch: file={}, current={} ({})",
                scene.version,
                SCENE_DEF_VERSION,
                path.display()
            );
        }
        Ok(scene)
    }

    /// Saves the scene definition to a plain-text RON file. Always written with the current
    /// version. The resulting file is human-readable and can be edited in any text editor.
    pub fn save(&self, path: &Path) -> Result<(), SaveError> {
        let versioned = SceneDef {
            version: SCENE_DEF_VERSION,
            ..self.clone()
        };
        write_ron(path, &versioned)
    }

    /// Adds an entity to the scene definition and returns `self` for builder chaining.
    pub fn with(mut self, def: EntityDef) -> Self {
        self.entities.push(def);
        self
    }
}

// ─── PrefabInstance ───────────────────────────────────────────────────────────

/// Tracks which prefab file an entity was spawned from.
///
/// Can be removed via the "Break Prefab" button in the Inspector.
///
/// # Example
/// ```rust,no_run
/// use engine::prefab::{Prefab, break_prefab_instance};
/// use engine::ecs::World;
/// use std::path::Path;
///
/// let mut world = engine::ecs::World::new();
/// let prefab = Prefab::load(Path::new("prefabs/coin.ron")).unwrap();
/// let entity = prefab.spawn_with_tracking(&mut world, "prefabs/coin.ron");
/// // To sever the link later:
/// break_prefab_instance(&mut world, entity);
/// ```
#[derive(Clone)]
pub struct PrefabInstance {
    /// Path of the source prefab file (for display purposes)
    pub source_path: String,
}

/// Removes the `PrefabInstance` marker from an entity, severing the prefab link.
pub fn break_prefab_instance(world: &mut World, entity: Entity) {
    world.remove_component::<PrefabInstance>(entity);
}

// ─── Prefab ───────────────────────────────────────────────────────────────────

/// Single-entity template stored in a file.
///
/// Useful for spawning the same entity multiple times or reusing it in an editor.
///
/// # Example
/// ```rust,no_run
/// use engine::prefab::Prefab;
/// use engine::ecs::World;
/// use std::path::Path;
///
/// let mut world = engine::ecs::World::new();
/// let prefab = Prefab::load(Path::new("prefabs/coin.ron")).unwrap();
/// let _e = prefab.spawn(&mut world);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Prefab {
    pub def: EntityDef,
}

impl Prefab {
    /// Loads a prefab from a plain-text RON file.
    ///
    /// Files written by the engine before v4.6 (AEAD-encrypted binary format) are detected
    /// automatically and decrypted transparently for backward compatibility.
    pub fn load(path: &Path) -> Result<Self, SaveError> {
        read_ron(path)
    }

    /// Saves the prefab to a plain-text RON file.
    ///
    /// The resulting file is human-readable and can be edited in any text editor.
    pub fn save(&self, path: &Path) -> Result<(), SaveError> {
        write_ron(path, self)
    }

    /// Spawns the prefab into the world and returns the created entity.
    pub fn spawn(&self, world: &mut World) -> Entity {
        spawn_entity_def(world, &self.def)
    }

    /// Spawns the prefab into the world, attaches a `PrefabInstance` marker, and returns the entity.
    ///
    /// The link can be severed via the "Break Prefab" button in the Inspector.
    pub fn spawn_with_tracking(&self, world: &mut World, path: impl Into<String>) -> Entity {
        let entity = self.spawn(world);
        world.add_component(
            entity,
            PrefabInstance {
                source_path: path.into(),
            },
        );
        entity
    }
}

// ─── Free functions ────────────────────────────────────────────────────────────

/// Spawns a single `EntityDef` into the world and returns the entity.
///
/// Only the components specified in `def` are inserted.
/// If the world contains a [`SerdeComponentRegistry`] resource and `def.components` is
/// non-empty, the registry is used to deserialize the extra components onto the entity.
pub fn spawn_entity_def(world: &mut World, def: &EntityDef) -> Entity {
    let entity = world.spawn();

    if let Some(tag) = &def.tag {
        world.add_component(entity, Tag(tag.clone()));
    }
    if let Some(transform) = &def.transform {
        world.add_component(entity, transform.clone());
    }
    if let Some(sprite) = &def.sprite {
        world.add_component(entity, sprite.clone());
    }

    // Deserialize arbitrary serde components via the registry (if present).
    if !def.components.is_empty() {
        if let Some(registry) = world.remove_resource::<SerdeComponentRegistry>() {
            registry.deserialize_into(world, entity, &def.components);
            world.insert_resource(registry);
        } else {
            log::warn!(
                "spawn_entity_def: entity has {} serde component(s) {:?} but no \
                 SerdeComponentRegistry is present — components dropped. \
                 Call App::register_serde_component to register them.",
                def.components.len(),
                def.components.keys().collect::<Vec<_>>(),
            );
        }
    }

    entity
}

/// Spawns all entities from a `SceneDef` into the world and returns the entity list.
///
/// If `EntityDef.parent` is set, the entity is attached as a child of the entity with that tag.
/// Legacy scenes without a `parent` key in the RON file are loaded as-is (backward compatible).
pub fn spawn_scene_def(world: &mut World, scene: &SceneDef) -> Vec<Entity> {
    // Pass 1: spawn all entities and build a tag → Entity map.
    //
    // When the same tag appears on multiple entities, parent resolution becomes
    // ambiguous. Previously the last entity silently won (last-wins). Now we use
    // **first-wins** and warn on duplicates. (All entities are still spawned; only
    // parent resolution uses the first entity.)
    let mut tag_to_entity: HashMap<String, Entity> = HashMap::new();
    let entities: Vec<Entity> = scene
        .entities
        .iter()
        .map(|def| {
            let e = spawn_entity_def(world, def);
            if let Some(tag) = &def.tag {
                use std::collections::hash_map::Entry;
                match tag_to_entity.entry(tag.clone()) {
                    Entry::Vacant(slot) => {
                        slot.insert(e);
                    }
                    Entry::Occupied(_) => {
                        log::warn!(
                            "spawn_scene_def: duplicate tag {tag:?}; keeping the first \
                             entity for parent resolution and ignoring later duplicates."
                        );
                    }
                }
            }
            e
        })
        .collect();

    // Pass 2: attach entities that have a parent tag
    for (def, &child) in scene.entities.iter().zip(entities.iter()) {
        if let Some(parent_tag) = &def.parent {
            if let Some(&parent) = tag_to_entity.get(parent_tag) {
                crate::hierarchy::attach(world, child, parent);
            }
        }
    }

    entities
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
