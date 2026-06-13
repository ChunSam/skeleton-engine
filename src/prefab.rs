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

/// A single registered serde-capable component type.
///
/// Holds type-erased serialize / deserialize / post-spawn closures so that any
/// `Serialize + DeserializeOwned + Clone` component can participate in scene
/// save/load without hardcoding each type in the engine core.
#[allow(clippy::type_complexity)]
pub struct SerdeComponentEntry {
    /// Extracts the component from `entity` and serializes it to a RON [`ron::Value`].
    /// Returns `None` when the entity does not have this component.
    pub serialize: Box<dyn Fn(&World, Entity) -> Option<ron::Value> + Send + Sync>,
    /// Deserializes a RON [`ron::Value`] and inserts the component onto `entity`.
    pub deserialize:
        Box<dyn Fn(&mut World, Entity, ron::Value) -> Result<(), String> + Send + Sync>,
    /// Optional hook run after a successful deserialize (e.g. copy initial_text → text).
    pub post_spawn: Option<Box<dyn Fn(&mut World, Entity) + Send + Sync>>,
}

/// Type-erased registry for serde-capable components.
///
/// Insert this as a resource via [`crate::app::App::register_serde_component`] or directly.
/// [`spawn_entity_def`] calls [`SerdeComponentRegistry::deserialize_into`] when
/// `EntityDef.components` is non-empty. The editor save path calls
/// [`SerdeComponentRegistry::serialize_entity`] to populate `EntityDef.components`.
///
/// # Example
/// ```rust,no_run
/// use engine::prefab::{SerdeComponentRegistry, SerdeComponentEntry};
/// use engine::ecs::World;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Clone, Serialize, Deserialize)]
/// struct Health { max: f32 }
///
/// let mut registry = SerdeComponentRegistry::default();
/// registry.register::<Health>("Health", None);
/// ```
#[derive(Default)]
pub struct SerdeComponentRegistry {
    entries: HashMap<String, SerdeComponentEntry>,
}

impl SerdeComponentRegistry {
    /// Registers a serde-capable component type under `name`.
    ///
    /// `post_spawn` is an optional closure run after every successful deserialize
    /// (useful for copying design-time fields to runtime counterparts, e.g.
    /// `initial_text` → `text` for [`crate::ui::TextInput`]).
    #[allow(clippy::type_complexity)]
    pub fn register<T>(
        &mut self,
        name: impl Into<String>,
        post_spawn: Option<Box<dyn Fn(&mut World, Entity) + Send + Sync>>,
    ) where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + Sync + 'static,
    {
        let key = name.into();
        self.entries.insert(
            key,
            SerdeComponentEntry {
                serialize: Box::new(|world, entity| {
                    // Serialize `T` to a RON string and store it as ron::Value::String.
                    // We do not parse to ron::Value::Map because that path loses enum
                    // variant names (the Value representation has no enum concept).
                    world
                        .get::<T>(entity)
                        .and_then(|c| ron::to_string(c).ok().map(ron::Value::String))
                }),
                deserialize: Box::new(|world, entity, val| {
                    // Extract the stored RON string and parse it as T.
                    match val {
                        ron::Value::String(s) => ron::from_str::<T>(&s)
                            .map(|c| world.add_component(entity, c))
                            .map_err(|e| e.to_string()),
                        other => Err(format!("expected ron::Value::String, got {:?}", other)),
                    }
                }),
                post_spawn,
            },
        );
    }

    /// Serializes all registered components present on `entity`.
    ///
    /// Returns a map of `type_name → ron::Value`. Components not present on the
    /// entity are omitted.
    pub fn serialize_entity(&self, world: &World, entity: Entity) -> HashMap<String, ron::Value> {
        self.entries
            .iter()
            .filter_map(|(name, entry)| (entry.serialize)(world, entity).map(|v| (name.clone(), v)))
            .collect()
    }

    /// Deserializes `components` into `entity`.
    ///
    /// Unknown component names are logged and skipped. Deserialization errors are
    /// logged and skipped (load never fails due to a bad component value).
    /// After a successful deserialize, the `post_spawn` hook is called if present.
    pub fn deserialize_into(
        &self,
        world: &mut World,
        entity: Entity,
        components: &HashMap<String, ron::Value>,
    ) {
        for (name, val) in components {
            match self.entries.get(name) {
                None => {
                    log::warn!(
                        "SerdeComponentRegistry: unknown component type {:?} — skipping",
                        name
                    );
                }
                Some(entry) => match (entry.deserialize)(world, entity, val.clone()) {
                    Ok(()) => {
                        if let Some(hook) = &entry.post_spawn {
                            hook(world, entity);
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "SerdeComponentRegistry: failed to deserialize {:?}: {} — skipping",
                            name,
                            e
                        );
                    }
                },
            }
        }
    }
}

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
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::components::{Sprite, Transform};
    use glam::Vec2;
    use std::fs;

    fn tmp_path(name: &str) -> std::path::PathBuf {
        // One unique dir per test (keyed by the file name) so each test's cleanup
        // (`remove_dir` on the parent) never races a sibling running in parallel —
        // previously all prefab tests shared `engine-prefab-test-{pid}` and a
        // concurrent `remove_dir` could delete it between another test's
        // `create_dir_all` and `fs::write`, yielding a flaky `Io(NotFound)`.
        std::env::temp_dir()
            .join(format!(
                "engine-prefab-test-{}-{}",
                std::process::id(),
                name
            ))
            .join(name)
    }

    #[test]
    fn entity_def_spawn_inserts_components() {
        let mut world = World::new();

        let def = EntityDef {
            tag: Some("hero".into()),
            transform: Some(Transform::new(
                Vec2::new(10.0, 20.0),
                Vec2::splat(64.0),
                0.5,
            )),
            sprite: Some(Sprite::colored(1.0, 0.0, 0.0)),
            parent: None,
            components: HashMap::new(),
        };

        let entity = spawn_entity_def(&mut world, &def);

        let tag = world.get::<Tag>(entity).expect("Tag should be present");
        assert_eq!(tag.0, "hero");

        let tf = world
            .get::<Transform>(entity)
            .expect("Transform should be present");
        assert_eq!(tf.position, Vec2::new(10.0, 20.0));

        let sp = world
            .get::<Sprite>(entity)
            .expect("Sprite should be present");
        assert_eq!(sp.color.r, 1.0);
    }

    #[test]
    fn empty_entity_def_spawn_no_components() {
        let mut world = World::new();
        let entity = spawn_entity_def(&mut world, &EntityDef::default());
        assert!(world.get::<Tag>(entity).is_none());
        assert!(world.get::<Transform>(entity).is_none());
        assert!(world.get::<Sprite>(entity).is_none());
    }

    #[test]
    fn scene_def_roundtrip() {
        let path = tmp_path("scene1.ron");

        let scene = SceneDef {
            entities: vec![
                EntityDef {
                    tag: Some("ground".into()),
                    transform: Some(Transform::new(
                        Vec2::new(0.0, -200.0),
                        Vec2::new(800.0, 32.0),
                        0.0,
                    )),
                    sprite: Some(Sprite::colored(0.3, 0.6, 0.3)),
                    parent: None,
                    components: HashMap::new(),
                },
                EntityDef {
                    tag: Some("player".into()),
                    transform: Some(Transform::default()),
                    sprite: None,
                    parent: None,
                    components: HashMap::new(),
                },
            ],
            ..Default::default()
        };

        scene.save(&path).expect("save should succeed");
        let loaded = SceneDef::load(&path).expect("load should succeed");

        assert_eq!(loaded.entities.len(), 2);
        assert_eq!(loaded.entities[0].tag.as_deref(), Some("ground"));
        assert_eq!(loaded.entities[1].tag.as_deref(), Some("player"));
        assert!(loaded.entities[1].sprite.is_none());

        let tf = loaded.entities[0].transform.as_ref().unwrap();
        assert_eq!(tf.position, Vec2::new(0.0, -200.0));

        fs::remove_file(&path).ok();
        fs::remove_dir(path.parent().unwrap()).ok();
    }

    #[test]
    fn scene_def_v1_sprite_normal_fields_are_ignored() {
        let text = r#"
SceneDef(
    version: 1,
    entities: [
        EntityDef(
            tag: Some("legacy"),
            sprite: Some(Sprite(
                texture: Some("legacy.png"),
                color: (r: 1.0, g: 0.5, b: 0.25, a: 1.0),
                normal_texture: Some("legacy_normal.png"),
            )),
        ),
    ],
)
"#;

        let scene: SceneDef = ron::from_str(text).expect("old normal field should be ignored");
        assert_eq!(scene.version, 1);
        let sprite = scene.entities[0].sprite.as_ref().unwrap();
        assert_eq!(sprite.texture.as_deref(), Some("legacy.png"));
        assert_eq!(sprite.color, Color::from([1.0, 0.5, 0.25, 1.0]));
    }

    #[test]
    fn prefab_roundtrip_and_spawn() {
        let path = tmp_path("coin.ron");

        let prefab = Prefab {
            def: EntityDef {
                tag: Some("coin".into()),
                transform: Some(Transform::new(
                    Vec2::new(100.0, 50.0),
                    Vec2::splat(32.0),
                    0.0,
                )),
                sprite: Some(Sprite::textured("assets/coin.png")),
                parent: None,
                components: HashMap::new(),
            },
        };

        prefab.save(&path).expect("save prefab");
        let loaded = Prefab::load(&path).expect("load prefab");

        assert_eq!(loaded.def.tag.as_deref(), Some("coin"));
        let sp = loaded.def.sprite.as_ref().unwrap();
        assert_eq!(sp.texture.as_deref(), Some("assets/coin.png"));

        let mut world = World::new();
        let entity = loaded.spawn(&mut world);
        let tag = world.get::<Tag>(entity).unwrap();
        assert_eq!(tag.0, "coin");

        fs::remove_file(&path).ok();
        fs::remove_dir(path.parent().unwrap()).ok();
    }

    /// Scene files are human-readable plain-text RON (no binary header).
    #[test]
    fn scene_def_file_is_human_readable() {
        let path = tmp_path("readable_scene.ron");

        let scene = SceneDef {
            entities: vec![EntityDef {
                tag: Some("readable_entity".into()),
                transform: Some(Transform::new(Vec2::ZERO, Vec2::splat(32.0), 0.0)),
                sprite: None,
                parent: None,
                components: HashMap::new(),
            }],
            ..Default::default()
        };

        scene.save(&path).expect("save should succeed");

        let raw = fs::read_to_string(&path).expect("file should be valid utf-8");
        assert!(
            raw.contains("readable_entity"),
            "scene file should contain the tag string as plain text"
        );
        assert!(
            raw.contains("entities"),
            "scene file should contain the field name 'entities'"
        );

        fs::remove_file(&path).ok();
        fs::remove_dir(path.parent().unwrap()).ok();
    }

    /// Prefab files are human-readable plain-text RON (no binary header).
    #[test]
    fn prefab_file_is_human_readable() {
        let path = tmp_path("readable_prefab.ron");

        let prefab = Prefab {
            def: EntityDef {
                tag: Some("readable_prefab_tag".into()),
                transform: None,
                sprite: None,
                parent: None,
                components: HashMap::new(),
            },
        };

        prefab.save(&path).expect("save should succeed");

        let raw = fs::read_to_string(&path).expect("file should be valid utf-8");
        assert!(
            raw.contains("readable_prefab_tag"),
            "prefab file should contain the tag string as plain text"
        );

        fs::remove_file(&path).ok();
        fs::remove_dir(path.parent().unwrap()).ok();
    }

    /// Files written by pre-4.6 `save()` (AEAD-encrypted) must still load via `SceneDef::load`.
    #[test]
    fn scene_def_backcompat_encrypted_load() {
        use crate::save::save;

        let path = tmp_path("legacy_scene.ron");

        let scene = SceneDef {
            entities: vec![EntityDef {
                tag: Some("legacy_tag".into()),
                transform: None,
                sprite: None,
                parent: None,
                components: HashMap::new(),
            }],
            ..Default::default()
        };

        // Simulate a pre-4.6 file written with the encrypted path
        save(&path, &scene).expect("old encrypted save should succeed");

        let loaded = SceneDef::load(&path).expect("load of encrypted legacy file should succeed");
        assert_eq!(
            loaded.entities[0].tag.as_deref(),
            Some("legacy_tag"),
            "tag must survive the back-compat encrypted load path"
        );

        fs::remove_file(&path).ok();
        fs::remove_dir(path.parent().unwrap()).ok();
    }

    /// Files written by pre-4.6 `save()` (AEAD-encrypted) must still load via `Prefab::load`.
    #[test]
    fn prefab_backcompat_encrypted_load() {
        use crate::save::save;

        let path = tmp_path("legacy_prefab.ron");

        let prefab = Prefab {
            def: EntityDef {
                tag: Some("legacy_prefab_tag".into()),
                transform: None,
                sprite: None,
                parent: None,
                components: HashMap::new(),
            },
        };

        // Simulate a pre-4.6 file written with the encrypted path
        save(&path, &prefab).expect("old encrypted save should succeed");

        let loaded = Prefab::load(&path).expect("load of encrypted legacy file should succeed");
        assert_eq!(
            loaded.def.tag.as_deref(),
            Some("legacy_prefab_tag"),
            "tag must survive the back-compat encrypted load path"
        );

        fs::remove_file(&path).ok();
        fs::remove_dir(path.parent().unwrap()).ok();
    }

    #[test]
    fn spawn_scene_def_returns_correct_count() {
        let mut world = World::new();
        let scene = SceneDef {
            entities: vec![
                EntityDef::default(),
                EntityDef::default(),
                EntityDef::default(),
            ],
            ..Default::default()
        };
        let entities = spawn_scene_def(&mut world, &scene);
        assert_eq!(entities.len(), 3);
    }

    /// #23: when the same tag appears on more than one entity, parent resolution
    /// is fixed to the **first** entity (first-wins). All entities are still spawned.
    #[test]
    fn spawn_scene_def_duplicate_tag_is_first_wins() {
        use crate::hierarchy::Parent;

        let scene = SceneDef {
            entities: vec![
                // First "shared" — wins for parent resolution
                EntityDef {
                    tag: Some("shared".into()),
                    ..Default::default()
                },
                // Second "shared" — duplicate (warning), ignored for parent resolution
                EntityDef {
                    tag: Some("shared".into()),
                    ..Default::default()
                },
                // Child that references "shared" as its parent
                EntityDef {
                    parent: Some("shared".into()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let mut world = World::new();
        let entities = spawn_scene_def(&mut world, &scene);
        // All three entities are spawned even with a duplicate tag.
        assert_eq!(entities.len(), 3);

        let first = entities[0];
        let child = entities[2];
        let parent = world
            .get::<Parent>(child)
            .expect("child should be attached to a parent");
        assert_eq!(
            parent.0, first,
            "parent must resolve to the FIRST tagged entity (first-wins)"
        );
    }

    #[test]
    fn scene_hierarchy_roundtrip() {
        use crate::hierarchy::Parent;

        let path = tmp_path("hierarchy_scene.ron");

        // Save a scene with a parent → child hierarchy
        let scene = SceneDef {
            entities: vec![
                EntityDef {
                    tag: Some("parent".into()),
                    transform: Some(Transform::default()),
                    sprite: None,
                    parent: None,
                    components: HashMap::new(),
                },
                EntityDef {
                    tag: Some("child".into()),
                    transform: Some(Transform::default()),
                    sprite: None,
                    parent: Some("parent".into()),
                    components: HashMap::new(),
                },
            ],
            ..Default::default()
        };

        scene.save(&path).expect("save should succeed");
        let loaded = SceneDef::load(&path).expect("load should succeed");

        // Verify the parent field is preserved in the RON
        assert_eq!(loaded.entities[1].parent.as_deref(), Some("parent"));

        // Verify the Parent component after spawning
        let mut world = World::new();
        let entities = spawn_scene_def(&mut world, &loaded);
        let parent_entity = entities[0];
        let child_entity = entities[1];
        let p = world
            .get::<Parent>(child_entity)
            .expect("child should have Parent component");
        assert_eq!(p.0, parent_entity);

        fs::remove_file(&path).ok();
        fs::remove_dir(path.parent().unwrap()).ok();
    }

    #[test]
    fn topological_sort_roots_before_children() {
        use crate::hierarchy::{attach, Parent};

        let mut world = World::new();
        let parent = world.spawn();
        let child = world.spawn();
        attach(&mut world, child, parent);

        let entities = vec![child, parent]; // provided in reverse order
        let sorted = crate::hierarchy::topological_sort_entities(&entities, &world);

        // Parent must come first
        assert_eq!(sorted[0], parent);
        assert_eq!(sorted[1], child);

        // Double-check that the Parent component on child points to parent
        let p = world.get::<Parent>(child).unwrap();
        assert_eq!(p.0, parent);
    }

    // ── SerdeComponentRegistry tests ─────────────────────────────────────────

    #[test]
    fn serde_registry_roundtrip() {
        use crate::ui::UiNode;

        let mut world = World::new();
        let mut registry = SerdeComponentRegistry::default();
        registry.register::<UiNode>("UiNode", None);
        world.insert_resource(registry);

        // Spawn an entity with UiNode
        let src = world.spawn();
        world.add_component(src, UiNode::default());

        // Serialize
        let registry = world.remove_resource::<SerdeComponentRegistry>().unwrap();
        let components = registry.serialize_entity(&world, src);
        world.insert_resource(registry);

        assert!(
            components.contains_key("UiNode"),
            "serialized map should contain UiNode"
        );

        // Deserialize into a fresh entity
        let dst = world.spawn();
        let registry = world.remove_resource::<SerdeComponentRegistry>().unwrap();
        registry.deserialize_into(&mut world, dst, &components);
        world.insert_resource(registry);

        assert!(
            world.get::<UiNode>(dst).is_some(),
            "UiNode should be present after deserialize"
        );
    }

    #[test]
    fn serde_registry_unknown_component_tolerance() {
        let mut world = World::new();
        let registry = SerdeComponentRegistry::default();
        world.insert_resource(registry);

        // EntityDef with a component name not in the registry
        let def = EntityDef {
            components: {
                let mut m = HashMap::new();
                m.insert("Nonexistent".to_string(), ron::Value::String("oops".into()));
                m
            },
            ..Default::default()
        };

        // spawn_entity_def must not panic and must succeed
        let entity = spawn_entity_def(&mut world, &def);
        // The entity exists, nothing bogus was added
        assert!(world.is_alive(entity));
    }

    #[test]
    fn scene_def_v2_backcompat_empty_components() {
        let text = r#"SceneDef(version: 2, entities: [(tag: Some("x"),)])"#;
        let scene: SceneDef = ron::from_str(text).expect("v2 scene must parse");
        assert_eq!(scene.version, 2);
        assert_eq!(scene.entities[0].tag.as_deref(), Some("x"));
        assert!(
            scene.entities[0].components.is_empty(),
            "v2 files must load with empty components"
        );
    }

    #[test]
    fn text_input_post_spawn_hook() {
        use crate::ui::TextInput;

        let mut world = World::new();
        let mut registry = SerdeComponentRegistry::default();
        registry.register::<TextInput>(
            "TextInput",
            Some(Box::new(|w, e| {
                if let Some(ti) = w.get_mut::<TextInput>(e) {
                    ti.text = ti.initial_text.clone();
                    ti.cursor = ti.text.len();
                }
            })),
        );
        world.insert_resource(registry);

        let ti = TextInput {
            initial_text: "hi".to_string(),
            ..TextInput::default()
        };

        let mut def = EntityDef::default();
        let ron_str = ron::to_string(&ti).expect("serialize TextInput");
        // Store as ron::Value::String — this is the format the registry serialize path uses.
        let val = ron::Value::String(ron_str);
        def.components.insert("TextInput".to_string(), val);

        let entity = spawn_entity_def(&mut world, &def);
        let ti = world.get::<TextInput>(entity).expect("TextInput present");
        assert_eq!(
            ti.text, "hi",
            "post_spawn hook must copy initial_text to text"
        );
    }
}
