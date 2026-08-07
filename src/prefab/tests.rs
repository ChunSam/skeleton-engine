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

// ── component_names_for ────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct CompA {
    x: f32,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct CompB {
    y: i32,
}

/// `component_names_for` returns only the names whose component is present on
/// the entity, sorted alphabetically, without doing a full RON serialization.
#[test]
fn component_names_for_present_and_absent() {
    let mut world = World::new();
    let mut registry = SerdeComponentRegistry::default();
    registry.register::<CompA>("CompA", None);
    registry.register::<CompB>("CompB", None);

    let e = world.spawn();
    world.add_component(e, CompA { x: 1.0 });
    // CompB is NOT added.

    let names = registry.component_names_for(&world, e);
    assert_eq!(names, vec!["CompA".to_string()]);
}

/// `component_names_for` returns all registered names when all components are present.
#[test]
fn component_names_for_all_present_sorted() {
    let mut world = World::new();
    let mut registry = SerdeComponentRegistry::default();
    // Register in reverse alphabetical order to verify sort.
    registry.register::<CompB>("CompB", None);
    registry.register::<CompA>("CompA", None);

    let e = world.spawn();
    world.add_component(e, CompA { x: 2.0 });
    world.add_component(e, CompB { y: 3 });

    let names = registry.component_names_for(&world, e);
    assert_eq!(names, vec!["CompA".to_string(), "CompB".to_string()]);
}

/// `component_names_for` returns empty when no registered components are present.
#[test]
fn component_names_for_none_present() {
    let mut world = World::new();
    let mut registry = SerdeComponentRegistry::default();
    registry.register::<CompA>("CompA", None);

    let e = world.spawn(); // no CompA added
    let names = registry.component_names_for(&world, e);
    assert!(names.is_empty());
}

/// When `def.components` is non-empty but no `SerdeComponentRegistry` resource
/// is present, `spawn_entity_def` must still return a valid entity (no panic),
/// and the components are simply dropped (the warn log is not captured here but
/// the absence of a panic confirms the guard path is taken).
#[test]
fn spawn_entity_def_no_registry_with_components_does_not_panic() {
    let mut world = World::new(); // no SerdeComponentRegistry inserted

    let mut def = EntityDef {
        tag: Some("ghost".into()),
        ..Default::default()
    };
    def.components.insert(
        "SomeComponent".to_string(),
        ron::Value::String("value".to_string()),
    );

    // Must not panic; entity is alive and its Tag is set, serde components dropped.
    let entity = spawn_entity_def(&mut world, &def);
    assert!(world.is_alive(entity));
    assert_eq!(
        world.get::<Tag>(entity).map(|t| t.0.as_str()),
        Some("ghost")
    );
}

/// `spawn_entity_def` must honour `EntityDef.parent`.
///
/// The field's own doc says "on spawn, the entity is attached as a child of the entity with this
/// tag", but the attach only ever existed in `spawn_scene_def`'s second pass. Every single-entity
/// path — undo-of-delete, Duplicate/Paste redo, Ctrl+V, `Prefab::spawn` — therefore restored the
/// entity as a **root**, and its `Transform` (authored as parent-relative) was then read as world
/// space, so the entity visibly teleported. Silent: nothing errored, the entity just moved.
#[test]
fn spawn_entity_def_attaches_to_parent_tag() {
    let mut world = World::new();
    let parent = world.spawn();
    world.add_component(parent, Tag("Rig".to_string()));

    let def = EntityDef {
        tag: Some("Hand".to_string()),
        parent: Some("Rig".to_string()),
        ..Default::default()
    };
    let child = spawn_entity_def(&mut world, &def);

    assert_eq!(
        world.get::<crate::hierarchy::Parent>(child).map(|p| p.0),
        Some(parent),
        "spawn_entity_def ignored EntityDef.parent, so the entity came back as a root"
    );
}

/// A `parent` tag matching nothing must leave the entity a root rather than panicking — the
/// undo-of-delete case where the parent was deleted too.
#[test]
fn spawn_entity_def_with_unknown_parent_tag_spawns_a_root() {
    let mut world = World::new();
    let def = EntityDef {
        tag: Some("Orphan".to_string()),
        parent: Some("NoSuchRig".to_string()),
        ..Default::default()
    };
    let e = spawn_entity_def(&mut world, &def);
    assert_eq!(world.get::<crate::hierarchy::Parent>(e).map(|p| p.0), None);
}

/// Scene loading must still resolve parents from its **scene-local** tag map, not the world.
///
/// Within one scene the parent is frequently listed *after* the child, so `spawn_scene_def`'s
/// first pass deliberately skips world-search resolution and its second pass attaches from the
/// map. This pins that a forward reference (child before parent) still links up — the case a
/// naive "resolve in spawn_entity_def" change breaks.
#[test]
fn spawn_scene_def_resolves_a_forward_parent_reference() {
    let mut world = World::new();
    let scene = SceneDef {
        entities: vec![
            EntityDef {
                tag: Some("Child".to_string()),
                parent: Some("Parent".to_string()),
                ..Default::default()
            },
            EntityDef {
                tag: Some("Parent".to_string()),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let spawned = spawn_scene_def(&mut world, &scene);
    assert_eq!(spawned.len(), 2);
    assert_eq!(
        world
            .get::<crate::hierarchy::Parent>(spawned[0])
            .map(|p| p.0),
        Some(spawned[1]),
        "the child listed before its parent must still attach"
    );
}
