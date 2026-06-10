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
//!             parent: None,
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
}

// ─── SceneDef ─────────────────────────────────────────────────────────────────

/// Current RON format version for `SceneDef`. Increment on structural changes.
pub const SCENE_DEF_VERSION: u32 = 2;

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

/// Topologically sorts the entity list so roots come before their children.
///
/// When saving a scene, parents must appear before children so the two-pass attach in `spawn_scene_def()` works correctly.
pub fn topological_sort_entities(entities: &[Entity], world: &World) -> Vec<Entity> {
    use std::collections::VecDeque;

    // Parent → children adjacency map
    let mut children_map: HashMap<Entity, Vec<Entity>> = HashMap::new();
    let entity_set: std::collections::HashSet<Entity> = entities.iter().copied().collect();
    let mut roots: Vec<Entity> = Vec::new();

    for &e in entities {
        match world.get::<crate::hierarchy::Parent>(e) {
            Some(p) if entity_set.contains(&p.0) => {
                children_map.entry(p.0).or_default().push(e);
            }
            _ => roots.push(e),
        }
    }

    // BFS: collect from roots down to children
    let mut result = Vec::with_capacity(entities.len());
    let mut queue: VecDeque<Entity> = roots.into_iter().collect();
    while let Some(e) = queue.pop_front() {
        result.push(e);
        if let Some(kids) = children_map.get(&e) {
            for &kid in kids {
                queue.push_back(kid);
            }
        }
    }
    result
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
                },
                EntityDef {
                    tag: Some("player".into()),
                    transform: Some(Transform::default()),
                    sprite: None,
                    parent: None,
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
                },
                EntityDef {
                    tag: Some("child".into()),
                    transform: Some(Transform::default()),
                    sprite: None,
                    parent: Some("parent".into()),
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
        let sorted = topological_sort_entities(&entities, &world);

        // Parent must come first
        assert_eq!(sorted[0], parent);
        assert_eq!(sorted[1], child);

        // Double-check that the Parent component on child points to parent
        let p = world.get::<Parent>(child).unwrap();
        assert_eq!(p.0, parent);
    }
}
