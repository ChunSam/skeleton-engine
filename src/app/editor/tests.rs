// ── Editor command undo/redo tests ────────────────────────────────────────────

use super::history::{EditorCmd, EditorHistory};
use super::prefab::entity_to_def;
use super::settings::EditorSettings;
use super::state::{EditorState, PaintTool};
use super::util::entity_matches_filter;
use crate::ecs::{Entity, World};
use crate::prefab::Tag;

// ── Fix A: DeleteEntity undo restores full def (including non-core components) ──

/// Regression test for Fix A: undo of DeleteEntity must restore all
/// components captured in `def`, not just tag/transform/sprite.
#[test]
fn delete_undo_restores_full_def() {
    let mut world = World::new();
    // Spawn an entity with tag + transform only (no serde components needed here,
    // the important thing is the def captures what was there).
    let e = world.spawn();
    world.add_component(e, Tag("Goblin".into()));
    world.add_component(e, crate::components::Transform::default());

    let def = entity_to_def(&world, e).expect("entity_to_def");
    assert_eq!(def.tag.as_deref(), Some("Goblin"));

    // Simulate the Delete button: push DeleteEntity, then despawn.
    let mut history = EditorHistory::new();
    history.push(EditorCmd::DeleteEntity {
        entity: None,
        def: def.clone(),
    });
    world.despawn(e);
    assert!(!world.is_alive(e), "entity should be despawned");

    // Undo: entity must come back with its tag.
    let mut sel: Option<Entity> = None;
    history.undo(&mut world, &mut sel);
    let restored = sel.expect("undo must set selection");
    assert!(world.is_alive(restored), "entity must be alive after undo");
    let tag = world
        .get::<Tag>(restored)
        .expect("Tag must be restored after undo");
    assert_eq!(tag.0, "Goblin");

    // Redo: entity must be despawned again.
    history.redo(&mut world, &mut sel);
    assert!(
        sel.is_none() || !world.is_alive(sel.unwrap()),
        "entity must be dead after redo"
    );
}

// ── Fix B: CreateEntity with def round-trips through undo/redo ────────────

/// Regression test for Fix B: undo of CreateEntity (Duplicate) must despawn
/// the entity; redo must re-spawn it with its original components.
#[test]
fn create_entity_with_def_undo_redo() {
    let mut world = World::new();
    // Spawn a "duplicated" entity.
    let e = world.spawn();
    world.add_component(e, Tag("Copy".into()));
    world.add_component(e, crate::components::Transform::default());

    let def = entity_to_def(&world, e);
    let mut history = EditorHistory::new();
    history.push(EditorCmd::CreateEntity {
        entity: e,
        def: def.clone(),
    });

    // Undo: entity must be despawned.
    let mut sel: Option<Entity> = Some(e);
    history.undo(&mut world, &mut sel);
    assert!(sel.is_none());
    assert!(!world.is_alive(e), "entity must be despawned after undo");

    // Redo: entity must be re-spawned from def with its tag.
    history.redo(&mut world, &mut sel);
    let redone = sel.expect("redo must set selection");
    assert!(world.is_alive(redone));
    let tag = world
        .get::<Tag>(redone)
        .expect("Tag must be present after redo");
    assert_eq!(tag.0, "Copy");
}

// ── Fix B: CreateEntity without def (New Entity) still works ─────────────

#[test]
fn create_entity_no_def_undo_redo() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, crate::components::Transform::default());
    world.add_component(e, Tag("New Entity".into()));

    let mut history = EditorHistory::new();
    history.push(EditorCmd::CreateEntity {
        entity: e,
        def: None,
    });

    let mut sel: Option<Entity> = Some(e);
    history.undo(&mut world, &mut sel);
    assert!(sel.is_none());
    assert!(!world.is_alive(e));

    // Redo of None-def spawns a fresh default entity.
    history.redo(&mut world, &mut sel);
    let redone = sel.expect("redo must yield an entity");
    assert!(world.is_alive(redone));
}

// ── entity_to_def captures the parent link (Undo/Duplicate restore hierarchy) ──
#[test]
fn entity_to_def_captures_parent_tag() {
    let mut world = World::new();
    let parent = world.spawn();
    world.add_component(parent, Tag("Parent".into()));
    let child = world.spawn();
    world.add_component(child, Tag("Child".into()));
    world.add_component(child, crate::components::Transform::default());
    world.add_component(child, crate::hierarchy::Parent(parent));

    let def = entity_to_def(&world, child).expect("entity_to_def");
    assert_eq!(
        def.parent.as_deref(),
        Some("Parent"),
        "entity_to_def must capture the parent's Tag so Undo-of-Delete restores hierarchy"
    );
}

#[test]
fn entity_filter_matches_case_insensitive_substring() {
    assert!(entity_matches_filter("Player", "")); // empty matches all
    assert!(entity_matches_filter("Player", "  ")); // whitespace-only matches all
    assert!(entity_matches_filter("Player", "lay")); // substring
    assert!(entity_matches_filter("Player", "PLAYER")); // case-insensitive
    assert!(entity_matches_filter("EnemyGoblin", "goblin"));
    assert!(!entity_matches_filter("Player", "enemy")); // no match
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Debug)]
struct CopyTest {
    v: i32,
}

#[test]
fn component_copy_paste_round_trips() {
    let mut app = crate::app::App::new();
    app.register_serde_component::<CopyTest>("CopyTest", None);

    let a = app.world.spawn();
    app.world.add_component(a, CopyTest { v: 7 });

    // Copy from A.
    app.copy_component(a, "CopyTest");
    assert!(
        app.editor.component_clipboard.is_some(),
        "clipboard populated by copy"
    );

    // Paste onto a fresh entity B.
    let b = app.world.spawn();
    app.paste_component(b);
    assert_eq!(
        app.world.get::<CopyTest>(b),
        Some(&CopyTest { v: 7 }),
        "pasted component matches the copied value"
    );
}

#[test]
fn component_copy_unregistered_is_noop() {
    let mut app = crate::app::App::new();
    // No serde registration for CopyTest → copy finds nothing.
    let a = app.world.spawn();
    app.world.add_component(a, CopyTest { v: 1 });
    app.copy_component(a, "CopyTest");
    assert!(app.editor.component_clipboard.is_none());
}

#[test]
fn prefab_save_spawn_round_trips() {
    let mut app = crate::app::App::new();
    app.register_serde_component::<CopyTest>("CopyTest", None);

    let a = app.world.spawn();
    app.world
        .add_component(a, crate::prefab::Tag("Hero".into()));
    app.world.add_component(
        a,
        crate::components::Transform::new(glam::Vec2::new(7.0, 9.0), glam::Vec2::splat(32.0), 0.0),
    );
    app.world.add_component(a, CopyTest { v: 42 });

    let path = std::env::temp_dir().join(format!("test_prefab_{}.ron", std::process::id()));
    let path_str = path.to_string_lossy().to_string();

    app.save_selected_as_prefab(a, &path_str);
    assert!(path.exists(), "prefab file written");

    // Spawn it back into a fresh entity.
    app.spawn_prefab(&path_str);
    let b = app
        .editor
        .inspector_selected
        .expect("spawned prefab selected");
    assert_ne!(a, b);
    assert_eq!(
        app.world.get::<crate::prefab::Tag>(b).map(|t| t.0.as_str()),
        Some("Hero")
    );
    assert_eq!(
        app.world
            .get::<crate::components::Transform>(b)
            .map(|t| t.position),
        Some(glam::Vec2::new(7.0, 9.0))
    );
    assert_eq!(
        app.world.get::<CopyTest>(b),
        Some(&CopyTest { v: 42 }),
        "serde component round-trips through the prefab"
    );
    assert!(
        app.world.get::<crate::prefab::PrefabInstance>(b).is_some(),
        "spawned prefab carries a PrefabInstance marker"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn editor_settings_round_trip() {
    let mut s = EditorState::new();
    s.snap_enabled = true;
    s.snap_size = 24.0;
    s.show_grid = true;
    s.show_pathgrid = true;
    s.paint_brush = 5;
    s.paint_tool = PaintTool::Bucket;

    let settings = EditorSettings::from_state(&s);
    let path =
        std::env::temp_dir().join(format!("test_editor_settings_{}.ron", std::process::id()));
    crate::save::write_ron(&path, &settings).unwrap();
    let loaded: EditorSettings = crate::save::read_ron(&path).unwrap();
    assert_eq!(loaded, settings, "settings round-trip through RON");

    // Apply onto a fresh (default) state.
    let mut fresh = EditorState::new();
    loaded.apply_to(&mut fresh);
    assert!(fresh.snap_enabled);
    assert_eq!(fresh.snap_size, 24.0);
    assert!(fresh.show_grid);
    assert!(fresh.show_pathgrid);
    assert_eq!(fresh.paint_brush, 5);
    assert_eq!(fresh.paint_tool, PaintTool::Bucket);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn debug_bounds_draws_aabbs_and_colliders() {
    let mut app = crate::app::App::new();
    // Three Transform entities; one also has a collider.
    for _ in 0..3 {
        let e = app.world.spawn();
        app.world
            .add_component(e, crate::components::Transform::default());
        if app.world.query::<crate::components::Transform>().count() == 3 {
            app.world.add_component(
                e,
                crate::collision::Collider::Aabb {
                    half_extents: glam::Vec2::splat(8.0),
                },
            );
        }
    }

    app.draw_debug_bounds();
    let dbg = app
        .world
        .resource::<crate::resources::DebugDraw>()
        .expect("DebugDraw resource");
    // 3 entity-bounds rects + 1 collider rect = 4 shapes.
    assert_eq!(dbg.shapes.len(), 4);
}

#[test]
fn pathfinding_overlay_shades_blocked_and_walkable_cells() {
    use crate::tilemap::{Tilemap, TilemapAtlas};
    let mut app = crate::app::App::new();
    // 3×3 map, one blocked (non-zero) cell in the center.
    let tiles = vec![vec![0, 0, 0], vec![0, 1, 0], vec![0, 0, 0]];
    let tm = Tilemap::new(
        TilemapAtlas::new("t.png", 1, 1),
        tiles,
        16.0,
        glam::Vec2::ZERO,
    );
    let e = app.world.spawn();
    app.world.add_component(e, tm);

    app.draw_pathfinding_overlay();
    let dbg = app
        .world
        .resource::<crate::resources::DebugDraw>()
        .expect("DebugDraw resource");
    // 8 walkable cells → outline shapes; 1 blocked cell → filled rect.
    assert_eq!(dbg.shapes.len(), 8, "walkable cells drawn as outlines");
    assert_eq!(
        dbg.filled_rects.len(),
        1,
        "blocked cell drawn as a filled rect"
    );
}

#[test]
fn reset_particle_emitter_restores_defaults_keeping_texture() {
    use crate::particle::ParticleEmitter;
    let mut app = crate::app::App::new();
    let e = app.world.spawn();
    let mut em = ParticleEmitter {
        spawn_rate: 999.0,
        lifetime: 42.0,
        emit: false,
        texture: Some(std::sync::Arc::from("spark.png")),
        ..Default::default()
    };
    em.velocity = glam::Vec2::new(123.0, 456.0);
    app.world.add_component(e, em);

    app.reset_particle_emitter(e);

    let got = app.world.get::<ParticleEmitter>(e).expect("emitter");
    let def = ParticleEmitter::default();
    assert_eq!(got.spawn_rate, def.spawn_rate, "spawn_rate reset");
    assert_eq!(got.lifetime, def.lifetime, "lifetime reset");
    assert_eq!(got.velocity, def.velocity, "velocity reset");
    assert!(got.emit, "emit reset to default (true)");
    assert_eq!(
        got.texture.as_deref(),
        Some("spark.png"),
        "texture is preserved across reset"
    );
}

#[test]
fn reset_particle_emitter_no_emitter_is_noop() {
    let mut app = crate::app::App::new();
    let e = app.world.spawn();
    // No panic / no insertion when the entity lacks a ParticleEmitter.
    app.reset_particle_emitter(e);
    assert!(app
        .world
        .get::<crate::particle::ParticleEmitter>(e)
        .is_none());
}

#[test]
fn reset_point_light_restores_defaults() {
    use crate::components::PointLight;
    let mut app = crate::app::App::new();
    let e = app.world.spawn();
    app.world.add_component(
        e,
        PointLight {
            color: crate::color::Color::rgb(1.0, 0.0, 0.0),
            radius: 12.0,
            intensity: 9.0,
            light_height: 1.9,
        },
    );

    app.reset_point_light(e);

    let got = app.world.get::<PointLight>(e).expect("light");
    let def = PointLight::default();
    assert_eq!(got.radius, def.radius);
    assert_eq!(got.intensity, def.intensity);
    assert_eq!(got.light_height, def.light_height);
    assert_eq!(got.color, def.color);
}

#[test]
fn ensure_ambient_light_inserts_default_once() {
    let mut app = crate::app::App::new();
    // Fresh App has no AmbientLight resource → first call inserts it.
    assert!(
        app.ensure_ambient_light(),
        "first call inserts a default AmbientLight"
    );
    assert!(app
        .world
        .resource::<crate::resources::AmbientLight>()
        .is_some());
    // Second call is a no-op (already present).
    assert!(
        !app.ensure_ambient_light(),
        "second call leaves the existing resource untouched"
    );
}

#[test]
fn pointlight_registered_as_editor_component() {
    let mut app = crate::app::App::new();
    // register_default_components (run by App::new) registers PointLight factory + remover.
    assert!(app.editor.component_factories.contains_key("PointLight"));
    assert!(app.editor.component_removers.contains_key("PointLight"));

    let e = app.world.spawn();
    // Disjoint field borrows: factory comes from `app.editor`, world is `app.world`.
    if let Some(factory) = app.editor.component_factories.get("PointLight") {
        factory(&mut app.world, e);
    }
    assert!(
        app.world.get::<crate::components::PointLight>(e).is_some(),
        "factory adds a PointLight"
    );
}

// ── Keyboard-UX shortcuts: delete / duplicate selection + camera focus ──────────

#[test]
fn editor_delete_selection_despawns_all_and_clears_with_undo() {
    use crate::components::Transform;
    let mut app = crate::app::App::new();
    let a = app.world.spawn();
    app.world.add_component(a, Tag("A".into()));
    app.world.add_component(a, Transform::default());
    let b = app.world.spawn();
    app.world.add_component(b, Tag("B".into()));
    app.world.add_component(b, Transform::default());
    app.editor.selected_entities = vec![a, b];
    app.editor.inspector_selected = Some(a);

    app.editor_delete_selection();

    assert!(
        !app.world.is_alive(a) && !app.world.is_alive(b),
        "both gone"
    );
    assert!(app.editor.selected_entities.is_empty());
    assert!(app.editor.inspector_selected.is_none());

    // One undo brings the most recent delete back (two delete cmds were recorded).
    let mut sel = None;
    app.editor.cmd_history.undo(&mut app.world, &mut sel);
    app.editor.cmd_history.undo(&mut app.world, &mut sel);
    let alive = app.world.query::<Tag>().count();
    assert_eq!(alive, 2, "both entities restored by two undos");
}

#[test]
fn editor_duplicate_selection_clones_offset_and_selects() {
    use crate::components::Transform;
    let mut app = crate::app::App::new();
    let e = app.world.spawn();
    app.world.add_component(e, Tag("Orig".into()));
    app.world.add_component(
        e,
        Transform {
            position: glam::Vec2::new(100.0, 50.0),
            ..Default::default()
        },
    );
    app.editor.selected_entities = vec![e];
    app.editor.inspector_selected = Some(e);

    app.editor_duplicate_selection();

    // Now two entities; the selection points at the clone, offset by (16,16).
    assert_eq!(app.world.query::<Tag>().count(), 2);
    let clone = app.editor.inspector_selected.expect("clone selected");
    assert_ne!(clone, e, "selection moved to the new clone");
    let pos = app.world.get::<Transform>(clone).unwrap().position;
    assert_eq!(pos, glam::Vec2::new(116.0, 66.0));
}

#[test]
fn editor_focus_camera_centers_on_selection() {
    use crate::camera::Camera;
    use crate::components::Transform;
    use crate::resources::ViewportSize;
    let mut app = crate::app::App::new();
    app.world
        .insert_resource(Camera::new(glam::Vec2::ZERO, 1.0));
    app.world.insert_resource(ViewportSize::new(800, 600));
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: glam::Vec2::new(500.0, 300.0),
            ..Default::default()
        },
    );
    app.editor.inspector_selected = Some(e);

    app.editor_focus_camera_on_selection();

    // position = entity_pos - viewport/2 / zoom = (500,300) - (400,300) = (100,0).
    let cam = app.world.resource::<Camera>().unwrap();
    assert_eq!(cam.position, glam::Vec2::new(100.0, 0.0));
}
