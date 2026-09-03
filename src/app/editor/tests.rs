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
    // ⚠️ The fixture carries a serde-registered component, and the assertions below check it.
    // Tag + Transform alone made this test green on the exact regression it names — undo
    // restoring only the `EntityDef`'s named fields and dropping `def.components` (v0.156.15).
    let mut app = crate::app::App::new();
    app.register_serde_component::<CopyTest>("CopyTest", None);
    let world = &mut app.world;
    let e = world.spawn();
    world.add_component(e, Tag("Goblin".into()));
    world.add_component(
        e,
        crate::components::Transform::new(glam::Vec2::new(7.0, 9.0), glam::Vec2::splat(3.0), 0.5),
    );
    world.add_component(e, CopyTest { v: 42 });

    let def = entity_to_def(world, e).expect("entity_to_def");
    assert_eq!(def.tag.as_deref(), Some("Goblin"));
    assert!(
        def.components.contains_key("CopyTest"),
        "precondition: the def captured the non-core component, or the undo assertion below \
         cannot fail on its stated cause"
    );

    // Simulate the Delete button: push DeleteEntity, then despawn.
    let mut history = EditorHistory::new();
    history.push(EditorCmd::DeleteEntity {
        entities: None,
        defs: vec![(def.clone(), None)],
    });
    world.despawn(e);
    assert!(!world.is_alive(e), "entity should be despawned");

    // Undo: the entity comes back whole — tag, transform, AND the registered component.
    let mut sel: Option<Entity> = None;
    history.undo(world, &mut sel);
    let restored = sel.expect("undo must set selection");
    assert!(world.is_alive(restored), "entity must be alive after undo");
    let tag = world
        .get::<Tag>(restored)
        .expect("Tag must be restored after undo");
    assert_eq!(tag.0, "Goblin");
    assert_eq!(
        world
            .get::<crate::components::Transform>(restored)
            .map(|t| t.position),
        Some(glam::Vec2::new(7.0, 9.0)),
        "the transform comes back at its own values, not a default"
    );
    assert_eq!(
        world.get::<CopyTest>(restored),
        Some(&CopyTest { v: 42 }),
        "and so does the component that only `def.components` carries — the whole point of \
         \"full def\""
    );

    // Redo: entity must be despawned again.
    history.redo(world, &mut sel);
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
    // Same fixture reasoning as `delete_undo_restores_full_def`: the redo arm respawns from the
    // def, so the def has to carry more than a tag for "with its original components" to mean
    // anything (v0.156.15).
    let mut app = crate::app::App::new();
    app.register_serde_component::<CopyTest>("CopyTest", None);
    let world = &mut app.world;
    let e = world.spawn();
    world.add_component(e, Tag("Copy".into()));
    world.add_component(
        e,
        crate::components::Transform::new(glam::Vec2::new(4.0, 5.0), glam::Vec2::ONE, 0.0),
    );
    world.add_component(e, CopyTest { v: 7 });

    let def = entity_to_def(world, e);
    let mut history = EditorHistory::new();
    history.push(EditorCmd::CreateEntity {
        entity: e,
        def: def.clone(),
    });

    // Undo: entity must be despawned.
    let mut sel: Option<Entity> = Some(e);
    history.undo(world, &mut sel);
    assert!(sel.is_none());
    assert!(!world.is_alive(e), "entity must be despawned after undo");

    // Redo: re-spawned from the def with everything the def carried.
    history.redo(world, &mut sel);
    let redone = sel.expect("redo must set selection");
    assert!(world.is_alive(redone));
    let tag = world
        .get::<Tag>(redone)
        .expect("Tag must be present after redo");
    assert_eq!(tag.0, "Copy");
    assert_eq!(
        world
            .get::<crate::components::Transform>(redone)
            .map(|t| t.position),
        Some(glam::Vec2::new(4.0, 5.0)),
        "the transform is the def's, not a default"
    );
    assert_eq!(
        world.get::<CopyTest>(redone),
        Some(&CopyTest { v: 7 }),
        "and the registered component came back too"
    );
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
    s.show_bounds = true;
    s.show_pathgrid = true;
    s.paint_brush = 5;
    s.paint_tool = PaintTool::Bucket;
    // ⚠️ Every field must differ from `EditorState::new()`'s value, or its half of the round trip
    // is asserted against itself: `show_bounds` and `locale` sat at their defaults on both sides,
    // so dropping either from `from_state`/`apply_to` stayed green (v0.156.15).
    s.locale = match super::EditorLocale::default() {
        super::EditorLocale::English => super::EditorLocale::Korean,
        _ => super::EditorLocale::English,
    };
    let flipped_locale = s.locale;

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
    assert!(fresh.show_bounds);
    assert!(fresh.show_pathgrid);
    assert_eq!(fresh.paint_brush, 5);
    assert_eq!(fresh.paint_tool, PaintTool::Bucket);
    assert_eq!(fresh.locale, flipped_locale, "the locale crosses both ways");

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

#[test]
fn hidden_registered_as_editor_component() {
    let mut app = crate::app::App::new();
    assert!(app.editor.component_factories.contains_key("Hidden"));
    assert!(app.editor.component_removers.contains_key("Hidden"));

    let e = app.world.spawn();
    if let Some(factory) = app.editor.component_factories.get("Hidden") {
        factory(&mut app.world, e);
    }
    assert!(
        app.world.get::<crate::components::Hidden>(e).is_some(),
        "factory adds Hidden"
    );
    if let Some(remover) = app.editor.component_removers.get("Hidden") {
        remover(&mut app.world, e);
    }
    assert!(
        app.world.get::<crate::components::Hidden>(e).is_none(),
        "remover removes Hidden"
    );
}

#[test]
fn editor_toasts_push_and_cap_to_five() {
    let mut app = crate::app::App::new();
    for i in 0..8 {
        app.editor_toast(format!("toast {i}"));
    }
    // The queue is capped at 5, dropping the oldest.
    assert_eq!(app.editor.toasts.len(), 5);
    assert_eq!(app.editor.toasts.first().unwrap().message, "toast 3");
    assert_eq!(app.editor.toasts.last().unwrap().message, "toast 7");
}

#[test]
fn prefab_save_and_spawn_push_toasts() {
    use crate::app::editor::state::ToastKind;
    let mut app = crate::app::App::new();

    // Spawning from a missing file surfaces an Error toast.
    app.spawn_prefab("/nonexistent-skeleton-engine-dir/does_not_exist.ron");
    assert_eq!(
        app.editor.toasts.last().expect("a toast was pushed").kind,
        ToastKind::Error,
        "loading a missing prefab should toast an error"
    );

    // Saving a real entity to a temp path surfaces a Success toast.
    let e = app.world.spawn();
    app.world
        .add_component(e, crate::prefab::Tag("Saveable".into()));
    app.world
        .add_component(e, crate::components::Transform::default());
    let path = std::env::temp_dir().join("skeleton_engine_prefab_toast_test.ron");
    app.save_selected_as_prefab(e, path.to_str().unwrap());
    assert_eq!(
        app.editor.toasts.last().expect("a toast was pushed").kind,
        ToastKind::Success,
        "saving a prefab should toast success"
    );
    let _ = std::fs::remove_file(&path);
}

/// Undo/redo must be dropped when the world is rebuilt.
///
/// Every `EditorCmd` stores raw `Entity` handles, and the ECS reuses entity ids. So a command
/// left on the stack across a `Replace` does not merely fail after the reset — it **resolves onto
/// whatever new entity now occupies that slot**. One Ctrl+Z then deletes, moves or re-parents an
/// object the user never touched, in a scene they only just loaded.
#[test]
fn world_reset_clears_editor_undo_history() {
    let mut app = crate::App::new();
    let e = app.world.spawn();
    app.world
        .add_component(e, crate::components::Transform::default());
    app.editor.cmd_history.push(EditorCmd::DeleteEntity {
        entities: Some(vec![e]),
        defs: vec![(Default::default(), None)],
    });
    assert_eq!(app.editor.cmd_history.undo_len(), 1);

    app.reload_scene();

    assert_eq!(
        app.editor.cmd_history.undo_len(),
        0,
        "undo history survived a world reset — its stale Entity handles now alias live entities"
    );
}

// ── The three-way name contract: adder ⇒ scene-serialisable ───────────────────

/// Every component the Inspector's "+ Add Component" can add must survive a scene save.
///
/// A component is captured by a scene only if it is one of `EntityDef`'s named fields
/// (`tag` / `transform` / `sprite`) or is registered in [`SerdeComponentRegistry`] —
/// `serialize_entity` walks the registry and nothing else. Nothing tied the editor's
/// factory map to that registry, so the two drifted twice: four components were fixed in
/// `core_resources.rs` (see the comment above the `Hidden` registration) and six more were
/// left behind, silently dropping a placed `PointLight` or `ParticleEmitter` on reload.
///
/// This test is the tie. It drives the **real serialization path** rather than the
/// registry's bookkeeping: each factory is applied to a fresh entity and the component is
/// required to come back out of `serialize_entity` — the function `do_save_scene` calls, whose
/// `filter_map` drops any component whose `Serialize` fails. It said this while calling
/// `component_names_for`, the cheap presence check, until v0.156.15.
#[test]
fn every_editor_addable_component_survives_a_scene_save() {
    use crate::app::App;
    use crate::prefab::SerdeComponentRegistry;

    /// Covered by `EntityDef`'s own named fields rather than the serde registry.
    const NAMED_FIELDS: [&str; 3] = ["Tag", "Transform", "Sprite"];

    let mut app = App::new();

    // Take the factory map so the closures can borrow `world` mutably; put it back after.
    let factories = std::mem::take(&mut app.editor.component_factories);
    assert!(
        !factories.is_empty(),
        "control: App::new() must have populated the factory map — an empty map would \
         make every assertion below vacuous"
    );

    let mut missing: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for (name, factory) in &factories {
        if NAMED_FIELDS.contains(&name.as_str()) {
            continue;
        }
        let e = app.world.spawn();
        factory(&mut app.world, e);

        // `serialize_entity` needs the registry by value while inspecting the world.
        let registry = app
            .world
            .remove_resource::<SerdeComponentRegistry>()
            .expect("App::new() inserts SerdeComponentRegistry");
        let serialised = registry.serialize_entity(&app.world, e);
        app.world.insert_resource(registry);

        checked += 1;
        if !serialised.contains_key(name.as_str()) {
            missing.push(name.clone());
        }
        app.world.despawn(e);
    }
    app.editor.component_factories = factories;

    assert!(
        checked >= 20,
        "control: only {checked} components were checked — the skip list must not have \
         swallowed the map"
    );
    missing.sort();
    assert!(
        missing.is_empty(),
        "these components can be added in the Inspector but a scene cannot carry them, so \
         Save Scene drops them in silence: {missing:?}"
    );
}

// ── Deleting a parent takes its subtree (v0.156.0) ────────────────────────────

/// Spawn `parent` at (500, 300) with a child at local (16, 0); the child's world position is
/// therefore (516, 300). `parent_tag` decides whether the link is expressible through
/// `EntityDef.parent`, which is tag-based.
#[cfg(not(target_arch = "wasm32"))]
fn app_with_a_parented_pair(parent_tag: Option<&str>) -> (crate::App, Entity, Entity) {
    let mut app = crate::App::new();
    let parent = app.world.spawn();
    if let Some(t) = parent_tag {
        app.world.add_component(parent, Tag(t.into()));
    }
    app.world.add_component(
        parent,
        crate::components::Transform::new(glam::Vec2::new(500.0, 300.0), glam::Vec2::ONE, 0.0),
    );
    let child = app.world.spawn();
    app.world.add_component(child, Tag("Child".into()));
    app.world.add_component(
        child,
        crate::components::Transform::new(glam::Vec2::new(16.0, 0.0), glam::Vec2::ONE, 0.0),
    );
    crate::hierarchy::attach(&mut app.world, child, parent);
    (app, parent, child)
}

#[cfg(not(target_arch = "wasm32"))]
fn world_pos(app: &crate::App, e: Entity) -> glam::Vec2 {
    app.world
        .get::<crate::hierarchy::GlobalTransform>(e)
        .expect("GlobalTransform")
        .position
}

#[cfg(not(target_arch = "wasm32"))]
fn entity_tagged(app: &crate::App, tag: &str) -> Option<Entity> {
    app.world
        .query::<Tag>()
        .find(|(_, t)| t.0 == tag)
        .map(|(e, _)| e)
}

/// 🗑 Delete on a parent deletes its children too, and undo brings the whole subtree back with
/// the hierarchy intact — the child composes against its parent again rather than drawing at its
/// local offset.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deleting_a_parent_takes_the_subtree_and_undo_restores_the_links() {
    use crate::ecs::System;
    let (mut app, parent, child) = app_with_a_parented_pair(Some("Parent"));
    crate::hierarchy::HierarchySystem::default().run(&mut app.world, 0.0);
    assert_eq!(
        world_pos(&app, child),
        glam::Vec2::new(516.0, 300.0),
        "precondition: the child must start composed against its parent"
    );

    app.editor.inspector_selected = Some(parent);
    app.editor_delete_selection();
    assert!(!app.world.is_alive(parent));
    assert!(
        !app.world.is_alive(child),
        "the child outlived its deleted parent — it is now an orphan pointing at a dead handle"
    );

    let mut sel: Option<Entity> = None;
    app.editor.cmd_history.undo(&mut app.world, &mut sel);
    let restored_child = entity_tagged(&app, "Child").expect("the child must come back too");
    crate::hierarchy::HierarchySystem::default().run(&mut app.world, 0.0);
    assert_eq!(
        world_pos(&app, restored_child),
        glam::Vec2::new(516.0, 300.0),
        "the child came back as a root — undo restored the entities but not the parent link"
    );
    assert_eq!(
        sel,
        entity_tagged(&app, "Parent"),
        "undo should select the restored root of the subtree"
    );
}

/// The parent link survives undo even when `EntityDef.parent` **cannot express it**.
///
/// That field is a tag, so a parent with no `Tag` drops the link — the same "parent link dropped"
/// the scene saver warns about. The command records the link as an index into its own def list
/// instead, and this is the case that tells the two apart: with tag-based resolution the child
/// would come back at its local (16, 0).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn undo_restores_a_parent_link_that_the_tag_based_def_cannot_express() {
    use crate::ecs::System;
    let (mut app, parent, _child) = app_with_a_parented_pair(None);
    assert!(
        entity_to_def(&app.world, _child)
            .expect("def")
            .parent
            .is_none(),
        "precondition: with an untagged parent the def really cannot carry the link, or this \
         test is measuring the tag path after all"
    );

    app.editor.inspector_selected = Some(parent);
    app.editor_delete_selection();

    let mut sel: Option<Entity> = None;
    app.editor.cmd_history.undo(&mut app.world, &mut sel);
    let restored_child = entity_tagged(&app, "Child").expect("the child must come back");
    crate::hierarchy::HierarchySystem::default().run(&mut app.world, 0.0);
    assert_eq!(
        world_pos(&app, restored_child),
        glam::Vec2::new(516.0, 300.0),
        "the link was lost — an index-based parent is what this case needs"
    );
}

/// Redo re-deletes exactly the subtree undo recreated.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn redo_of_a_subtree_delete_removes_the_whole_subtree_again() {
    let (mut app, parent, _child) = app_with_a_parented_pair(Some("Parent"));
    app.editor.inspector_selected = Some(parent);
    app.editor_delete_selection();

    let mut sel: Option<Entity> = None;
    app.editor.cmd_history.undo(&mut app.world, &mut sel);
    assert!(
        entity_tagged(&app, "Child").is_some() && entity_tagged(&app, "Parent").is_some(),
        "control: undo must have restored both, or redo has nothing to remove"
    );

    app.editor.cmd_history.redo(&mut app.world, &mut sel);
    assert!(entity_tagged(&app, "Parent").is_none());
    assert!(
        entity_tagged(&app, "Child").is_none(),
        "redo removed the root and left the child behind"
    );
}

// ── EditorHistory heap budget (v0.156.1) ──────────────────────────────────────

/// A `PaintTiles` command holding `cells` changed cells — the payload that actually scales.
fn paint_of(cells: usize) -> EditorCmd {
    EditorCmd::PaintTiles {
        entity: Entity::from_raw_parts(0, 0),
        changes: vec![(0, 0, 0, 1); cells],
    }
}

#[test]
fn retained_bytes_counts_a_paint_payload_exactly() {
    let mut history = EditorHistory::new();
    history.push(paint_of(1000));
    assert_eq!(
        history.retained_bytes(),
        1000 * std::mem::size_of::<(usize, usize, u32, u32)>(),
        "a cell is 24 B and 1000 of them are 24,000 — the budget is only meaningful if this is"
    );
}

#[test]
fn the_oldest_commands_are_dropped_when_the_budget_is_passed() {
    let mut history = EditorHistory::new();
    // 100 cells = 2,400 B per command; a 6,000 B budget holds two.
    history.budget_bytes = 6_000;
    for _ in 0..5 {
        history.push(paint_of(100));
    }
    assert_eq!(
        history.undo_len(),
        2,
        "expected the budget to hold two commands"
    );
    assert!(
        history.retained_bytes() <= history.budget_bytes,
        "trimming stopped before reaching the budget"
    );
}

#[test]
fn the_newest_command_is_never_dropped_even_alone_over_budget() {
    // One Bucket fill on a large tilemap is exactly this case: 1.50 MB in a single stroke. The
    // user must still be able to undo the thing they just did.
    let mut history = EditorHistory::new();
    history.budget_bytes = 1_000;
    history.push(paint_of(10_000)); // 240,000 B, 240x the budget
    assert_eq!(history.undo_len(), 1);
    assert!(
        history.retained_bytes() > history.budget_bytes,
        "precondition"
    );
}

#[test]
fn small_commands_are_never_trimmed() {
    // Control against a trim that fires indiscriminately: 1,000 ordinary edits are ~0 heap, so
    // nothing may be dropped. This is the freehand end of the measurement — 70.75 KB for 1,000
    // strokes — and a count-based cap is what would have broken it.
    let mut history = EditorHistory::new();
    history.budget_bytes = 6_000;
    for _ in 0..1_000 {
        history.push(EditorCmd::MoveEntity {
            entity: Entity::from_raw_parts(0, 0),
            old_pos: glam::Vec2::ZERO,
            new_pos: glam::Vec2::ONE,
        });
    }
    assert_eq!(
        history.undo_len(),
        1_000,
        "an edit with no payload was trimmed"
    );
}

#[test]
fn the_default_budget_is_not_zero() {
    // `EditorHistory` derived `Default` before v0.156.1; a derived one would set the budget to 0
    // and trim every history to a single command. `EditorState::new` uses `new()`, but a future
    // `..Default::default()` would not.
    let mut history = EditorHistory::default();
    for _ in 0..3 {
        history.push(paint_of(100));
    }
    assert_eq!(history.undo_len(), 3);
}

// ── The copy clipboard survives a world reset (v0.156.2) ──────────────────────

/// `copy_clipboard` holds `EntityDef` **values**, so it cannot retarget across a reset the way an
/// undo command can — copying in one scene and pasting in another is a feature. `reset_scene`
/// used to clear it anyway while the editor's own 📂 Load never did, a divergence with no stated
/// intent on either side.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_world_reset_keeps_the_copy_clipboard() {
    let mut app = crate::App::new();
    app.editor.copy_clipboard = vec![crate::prefab::EntityDef {
        tag: Some("Copied".into()),
        ..Default::default()
    }];
    // Something the reset MUST drop, so "the clipboard survived" cannot be read as "the reset
    // never ran". This is the positive control the whole test hangs on.
    let e = app.world.spawn();
    app.editor.cmd_history.push(EditorCmd::DeleteEntity {
        entities: Some(vec![e]),
        defs: vec![(Default::default(), None)],
    });

    app.reload_scene();

    assert_eq!(
        app.editor.cmd_history.undo_len(),
        0,
        "control: the reset must have dropped the undo history, or it did not run at all"
    );
    assert_eq!(
        app.editor.copy_clipboard.len(),
        1,
        "the clipboard was cleared by the reset"
    );
    assert_eq!(
        app.editor.copy_clipboard[0].tag.as_deref(),
        Some("Copied"),
        "the clipboard survived but its contents did not"
    );
}

// ── The inspector write-back applies what the UI changed, and nothing else ────

/// One editor frame with `events` in its `RawInput`, the way `headless.rs` drives the docked
/// editor — no window, no GPU.
#[cfg(not(target_arch = "wasm32"))]
fn editor_frame(app: &mut crate::app::App, events: Vec<egui::Event>) {
    let ctx = egui::Context::default();
    // `handle_editor_shortcuts` reads the frame's `modifiers`, not the key event's; a real
    // winit frame sets both, so mirror the first key event's modifiers onto the frame.
    let modifiers = events
        .iter()
        .find_map(|e| match e {
            egui::Event::Key { modifiers, .. } => Some(*modifiers),
            _ => None,
        })
        .unwrap_or_default();
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 600.0),
        )),
        events,
        modifiers,
        ..Default::default()
    };
    ctx.begin_pass(raw);
    app.update_editor_ui(&Some(ctx.clone()), 1.0 / 60.0);
    let _ = ctx.end_pass();
}

#[cfg(not(target_arch = "wasm32"))]
fn ctrl_z() -> egui::Event {
    egui::Event::Key {
        key: egui::Key::Z,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::CTRL,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn position_of(app: &crate::app::App, e: Entity) -> glam::Vec2 {
    app.world
        .get::<crate::components::Transform>(e)
        .expect("has a Transform")
        .position
}

/// `update_editor_ui` stages every reflected field of the selected entity before the egui block
/// and used to write all of them back after it — so a world write made *inside* the block to the
/// selected entity was reverted the same frame. Ctrl+Z of that entity's own move is the everyday
/// case: the undo landed, then the write-back put the staged position straight back. Every gizmo
/// undo test in the tree calls `cmd_history.undo` directly, which is why none of them saw it.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ctrl_z_on_the_selected_entity_survives_the_inspector_write_back() {
    let moved = glam::Vec2::new(100.0, 100.0);
    let mut app = crate::app::App::new();
    let e = app.world.spawn();
    app.world.add_component(
        e,
        crate::components::Transform::new(moved, glam::Vec2::splat(20.0), 0.0),
    );
    app.editor_select_entity(e);
    app.editor.mode = super::EditorMode::Overlay;
    app.editor.cmd_history.push(EditorCmd::MoveEntity {
        entity: e,
        old_pos: glam::Vec2::ZERO,
        new_pos: moved,
    });

    editor_frame(&mut app, vec![ctrl_z()]);

    assert_eq!(
        app.editor.cmd_history.undo_len(),
        0,
        "precondition: Ctrl+Z reached the history inside the frame"
    );
    assert_eq!(
        position_of(&app, e),
        glam::Vec2::ZERO,
        "the undo's write must survive the write-back — the inspector changed nothing, so it \
         must write nothing"
    );

    // Control: with a *different* entity selected the same frame already passed, because the
    // write-back is skipped when the selection moves. So the assertion above is about the
    // same-entity case specifically, not about Ctrl+Z working at all.
    let mut app = crate::app::App::new();
    let e = app.world.spawn();
    app.world.add_component(
        e,
        crate::components::Transform::new(moved, glam::Vec2::splat(20.0), 0.0),
    );
    let other = app.world.spawn();
    app.world
        .add_component(other, crate::components::Transform::default());
    app.editor_select_entity(other);
    app.editor.mode = super::EditorMode::Overlay;
    app.editor.cmd_history.push(EditorCmd::MoveEntity {
        entity: e,
        old_pos: glam::Vec2::ZERO,
        new_pos: moved,
    });
    editor_frame(&mut app, vec![ctrl_z()]);
    assert_eq!(position_of(&app, e), glam::Vec2::ZERO, "control");
}

/// The same mechanism from the other side: a registered inspector panel that writes a reflected
/// component directly, inside the block, keeps its write. Before, the `Name:` field, the inline
/// rename and ⧉ Paste were all silently reverted this way for every reflected type.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_direct_write_inside_the_frame_survives_the_write_back() {
    let mut app = crate::app::App::new();
    let e = app.world.spawn();
    app.world.add_component(e, Tag("A".into()));
    app.world
        .add_component(e, crate::components::Transform::default());
    app.editor_select_entity(e);
    app.editor.mode = super::EditorMode::Overlay;
    let drew = std::rc::Rc::new(std::cell::Cell::new(false));
    let d = std::rc::Rc::clone(&drew);
    app.register_inspector_panel::<Tag>("Renamer", move |_ui, app, e| {
        d.set(true);
        if let Some(t) = app.world.get_mut::<Tag>(e) {
            t.0 = "B".into();
        }
    });

    editor_frame(&mut app, vec![]);

    assert!(
        drew.get(),
        "precondition: the panel drew inside the frame, or this test proves nothing"
    );
    assert_eq!(
        app.world.get::<Tag>(e).unwrap().0,
        "B",
        "a write made inside the frame must not be overwritten by the staged copy"
    );
}

// ── A delete releases the physics of what it despawns ─────────────────────────

/// Both editor delete paths despawned through `hierarchy::despawn_recursive`, which is
/// storage-level like `World::despawn`: the subtree's rapier bodies stayed in `PhysicsWorld` as
/// invisible colliders. `editor_despawn_subtree` releases them first — for the root and every
/// descendant.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deleting_a_subtree_releases_its_physics() {
    use crate::physics::{PhysicsBody, PhysicsWorld};
    let mut app = crate::app::App::new();
    app.world
        .insert_resource(PhysicsWorld::new(glam::Vec2::ZERO));
    let bodies = |app: &crate::app::App| {
        app.world
            .resource::<PhysicsWorld>()
            .unwrap()
            .rigid_body_set
            .len()
    };
    let baseline = bodies(&app);
    let with_body = |app: &mut crate::app::App| {
        let e = app.world.spawn();
        app.world
            .add_component(e, crate::components::Transform::default());
        let (rb, col) = app
            .world
            .resource_mut::<PhysicsWorld>()
            .unwrap()
            .add_dynamic_box(glam::Vec2::ZERO, 8.0, 8.0, true);
        app.world.add_component(
            e,
            PhysicsBody {
                rigid_body_handle: rb,
                collider_handle: col,
            },
        );
        e
    };
    let parent = with_body(&mut app);
    let child = with_body(&mut app);
    assert!(crate::hierarchy::reparent(
        &mut app.world,
        child,
        Some(parent)
    ));
    assert_eq!(
        bodies(&app),
        baseline + 2,
        "precondition: parent and child each own a body"
    );

    app.editor_select_entity(parent);
    app.editor_delete_selection();

    assert!(
        !app.world.is_alive(parent) && !app.world.is_alive(child),
        "the subtree is gone"
    );
    assert_eq!(
        bodies(&app),
        baseline,
        "both bodies must be released — the child's through the cascade, not only the root's"
    );
    assert_eq!(
        app.editor.cmd_history.undo_len(),
        1,
        "control: the delete is still recorded for undo"
    );
}

// ── Mode transitions, settings on disk, and the two unrecorded spawns ──────────

/// The table `set_editor_mode` applies. The toolbar's Exit button spelled the Docked→Off half
/// by hand without the save; now every caller reads this.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn mode_transition_table() {
    use super::state::{mode_transition, ModeTransition};
    use super::EditorMode::{Docked, Off, Overlay};
    let t = |old, new, loaded| mode_transition(old, new, loaded);
    let mt = |load_settings, save_settings, resume| ModeTransition {
        load_settings,
        save_settings,
        resume,
    };
    assert_eq!(
        t(Off, Docked, false),
        mt(true, false, false),
        "first Docked open loads"
    );
    assert_eq!(
        t(Off, Docked, true),
        mt(false, false, false),
        "later opens do not"
    );
    assert_eq!(
        t(Docked, Off, true),
        mt(false, true, true),
        "Docked exit saves and resumes"
    );
    assert_eq!(
        t(Docked, Overlay, true),
        mt(false, true, true),
        "so does Docked→Overlay"
    );
    assert_eq!(
        t(Overlay, Off, false),
        mt(false, false, true),
        "no Docked involved: resume only"
    );
    assert_eq!(
        t(Docked, Docked, true),
        mt(false, false, false),
        "no change: nothing"
    );
}

/// Leaving Docked through `set_editor_mode` writes the settings file — what the toolbar's Exit
/// button skipped — and a transition that never touched Docked writes nothing.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn leaving_docked_saves_the_settings_file() {
    use super::EditorMode;
    let path = std::env::temp_dir().join(format!("set_mode_saves_{}.ron", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut app = crate::app::App::new();
    app.editor.settings_path_override = Some(path.clone());
    app.editor.mode = EditorMode::Docked;
    app.editor.show_grid = true;
    app.editor.paused = true;

    app.set_editor_mode(EditorMode::Off);

    assert_eq!(app.editor.mode, EditorMode::Off);
    assert!(!app.editor.paused, "exiting Docked resumes");
    let saved: EditorSettings =
        crate::save::read_ron(&path).expect("the settings file was written");
    assert!(saved.show_grid, "and it holds the current preferences");
    let _ = std::fs::remove_file(&path);

    // Control: Overlay→Off never involves Docked, so nothing is written.
    let path2 = std::env::temp_dir().join(format!("set_mode_saves_ctl_{}.ron", std::process::id()));
    let _ = std::fs::remove_file(&path2);
    let mut app = crate::app::App::new();
    app.editor.settings_path_override = Some(path2.clone());
    app.editor.mode = EditorMode::Overlay;
    app.set_editor_mode(EditorMode::Off);
    assert!(!path2.exists(), "control: no Docked exit, no file");
}

/// A settings file that exists but does not parse is left alone and logged; the preferences in
/// memory stand. Before, the failure was indistinguishable from a missing file — every
/// preference silently reverted, and the next save overwrote the evidence.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_corrupt_settings_file_keeps_the_in_memory_preferences() {
    let path = std::env::temp_dir().join(format!("corrupt_settings_{}.ron", std::process::id()));
    std::fs::write(&path, "this is not RON (").unwrap();
    let mut app = crate::app::App::new();
    app.editor.settings_path_override = Some(path.clone());
    app.editor.snap_size = 24.0;
    app.load_editor_settings();
    assert_eq!(
        app.editor.snap_size, 24.0,
        "nothing applied from a file that did not parse"
    );

    // Control: a valid file at the same path applies.
    let good = EditorSettings {
        snap_size: 48.0,
        ..EditorSettings::from_state(&app.editor)
    };
    crate::save::write_ron(&path, &good).unwrap();
    app.load_editor_settings();
    assert_eq!(app.editor.snap_size, 48.0, "control: a valid file applies");
    let _ = std::fs::remove_file(&path);
}

/// `➕ Spawn` was the one spawn that recorded nothing: Ctrl+Z after it undid an older command
/// and the prefab instance stayed.
#[test]
fn spawning_a_prefab_is_recorded_for_undo() {
    let mut app = crate::app::App::new();
    let a = app.world.spawn();
    app.world.add_component(a, Tag("Hero".into()));
    app.world
        .add_component(a, crate::components::Transform::default());
    let path = std::env::temp_dir().join(format!("spawn_prefab_undo_{}.ron", std::process::id()));
    let path_str = path.to_string_lossy().to_string();
    app.save_selected_as_prefab(a, &path_str);
    let before = app.editor.cmd_history.undo_len();

    app.spawn_prefab(&path_str);
    let b = app.editor.inspector_selected.expect("spawned and selected");
    assert_eq!(
        app.editor.cmd_history.undo_len(),
        before + 1,
        "the spawn is one undo entry"
    );

    let mut sel = app.editor.inspector_selected;
    app.editor.cmd_history.undo(&mut app.world, &mut sel);
    assert!(!app.world.is_alive(b), "undo removes the spawned instance");
    assert!(
        app.world.is_alive(a),
        "and not the entity the prefab was saved from"
    );
    let _ = std::fs::remove_file(&path);
}

/// `reload_scene` cleared the save status but kept the load status, so the toolbar went on
/// naming a file the scene on screen did not come from.
#[test]
fn a_world_reset_clears_the_load_status_too() {
    let mut app = crate::app::App::new();
    app.editor.editor_load_status = Some("✓ 12 entities ← old.ron".into());
    app.editor.editor_save_status = Some("saved".into());
    app.reload_scene();
    assert_eq!(app.editor.editor_load_status, None);
    assert_eq!(
        app.editor.editor_save_status, None,
        "control: the save status was already cleared"
    );
}

// ── The overlays and F draw and focus where the renderer draws ─────────────────

/// The bounds overlay used `Transform` alone, the one place the collision grid stopped indexing
/// at: a parented collider's box drew at its offset from the origin while collision tested it
/// at its `GlobalTransform` — the placement this overlay exists to show.
#[test]
fn debug_bounds_draws_a_parented_collider_where_it_collides() {
    use crate::resources::DebugShape;
    let mut app = crate::app::App::new();
    let p = app.world.spawn();
    app.world.add_component(
        p,
        crate::components::Transform::new(
            glam::Vec2::new(300.0, 200.0),
            glam::Vec2::splat(32.0),
            0.0,
        ),
    );
    // C is local (16, 0) under P, composed to (316, 200) — what `HierarchySystem` writes.
    let c = app.world.spawn();
    app.world.add_component(
        c,
        crate::components::Transform::new(glam::Vec2::new(16.0, 0.0), glam::Vec2::splat(16.0), 0.0),
    );
    app.world.add_component(
        c,
        crate::hierarchy::GlobalTransform {
            position: glam::Vec2::new(316.0, 200.0),
            scale: glam::Vec2::splat(16.0),
            rotation: 0.0,
            z: 0.0,
        },
    );
    app.world.add_component(
        c,
        crate::collision::Collider::Aabb {
            half_extents: glam::Vec2::splat(12.0),
        },
    );
    // Control: a root with no `GlobalTransform` still draws at its `Transform`.
    let r = app.world.spawn();
    app.world.add_component(
        r,
        crate::components::Transform::new(glam::Vec2::new(50.0, 50.0), glam::Vec2::splat(8.0), 0.0),
    );
    app.world.add_component(
        r,
        crate::collision::Collider::Aabb {
            half_extents: glam::Vec2::splat(4.0),
        },
    );

    app.draw_debug_bounds();
    let dbg = app
        .world
        .resource::<crate::resources::DebugDraw>()
        .expect("DebugDraw resource");
    let rect_min_of_size = |size: f32| -> glam::Vec2 {
        dbg.shapes
            .iter()
            .find_map(|s| match s {
                DebugShape::Rect { min, max, .. } if ((*max - *min).x - size).abs() < 1e-4 => {
                    Some(*min)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no rect of size {size}"))
    };
    assert_eq!(
        rect_min_of_size(24.0),
        glam::Vec2::new(304.0, 188.0),
        "the collider box sits where the collision grid tests it"
    );
    assert_eq!(
        rect_min_of_size(16.0),
        glam::Vec2::new(308.0, 192.0),
        "and the child's bounds sit where the sprite draws"
    );
    assert_eq!(
        rect_min_of_size(8.0),
        glam::Vec2::new(46.0, 46.0),
        "control: a root is unchanged"
    );
}

/// The pathfinding overlay rebuilt each map without its `projection`, so an isometric map was
/// shaded as a square lattice at the orthographic positions — contradicting its own doc,
/// "visualizes exactly the grid a game … would navigate".
#[test]
fn pathfinding_overlay_places_cells_by_the_maps_projection() {
    use crate::tilemap::{Tilemap, TilemapAtlas, TilemapProjection};
    let mut app = crate::app::App::new();
    let tiles = vec![vec![0, 0, 0], vec![0, 1, 0], vec![0, 0, 0]];
    let tm = Tilemap::new(
        TilemapAtlas::new("t.png", 1, 1),
        tiles,
        16.0,
        glam::Vec2::ZERO,
    )
    .with_projection(TilemapProjection::Isometric);
    let expected = tm.cell_center_world(1, 1);
    let e = app.world.spawn();
    app.world.add_component(e, tm);

    app.draw_pathfinding_overlay();
    let dbg = app
        .world
        .resource::<crate::resources::DebugDraw>()
        .expect("DebugDraw resource");
    assert_eq!(dbg.filled_rects.len(), 1, "one blocked cell");
    let r = &dbg.filled_rects[0];
    let centre = (r.min + r.max) * 0.5;
    assert_eq!(
        centre, expected,
        "the blocked cell is shaded at its isometric centre"
    );
    // Control on the instrument: the orthographic centre is somewhere else entirely.
    assert_ne!(
        centre,
        glam::Vec2::new(24.0, 24.0),
        "control: not the square-lattice position"
    );
}

/// F centred the camera on the selection's local `Transform`, so focusing a child went to its
/// offset from the parent — near the origin — and left the child off-screen.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn focus_centres_on_where_a_child_is_drawn() {
    let mut app = crate::app::App::new();
    app.world
        .insert_resource(crate::camera::Camera::new(glam::Vec2::ZERO, 1.0));
    app.world
        .insert_resource(crate::resources::ViewportSize::new(800, 600));
    let p = app.world.spawn();
    app.world.add_component(
        p,
        crate::components::Transform::new(glam::Vec2::new(500.0, 300.0), glam::Vec2::ONE, 0.0),
    );
    let c = app.world.spawn();
    app.world.add_component(
        c,
        crate::components::Transform::new(glam::Vec2::new(16.0, 0.0), glam::Vec2::ONE, 0.0),
    );
    app.world.add_component(
        c,
        crate::hierarchy::GlobalTransform {
            position: glam::Vec2::new(516.0, 300.0),
            scale: glam::Vec2::ONE,
            rotation: 0.0,
            z: 0.0,
        },
    );

    app.editor_select_entity(c);
    app.editor_focus_camera_on_selection();
    let cam = app
        .world
        .resource::<crate::camera::Camera>()
        .unwrap()
        .position;
    assert_eq!(
        cam,
        glam::Vec2::new(516.0, 300.0) - glam::Vec2::new(400.0, 300.0),
        "centred on where C draws"
    );

    // Control: a root with no `GlobalTransform` focuses on its `Transform`, as before.
    app.editor_select_entity(p);
    app.editor_focus_camera_on_selection();
    let cam = app
        .world
        .resource::<crate::camera::Camera>()
        .unwrap()
        .position;
    assert_eq!(cam, glam::Vec2::new(100.0, 0.0), "control");
}

// ── The editor acts only while it is showing ───────────────────────────────────

/// The shortcuts block was gated on the egui context alone, which every windowed frame has, and
/// F1 / F2 never clear the selection. Overlay → click an entity → F1 → play: Backspace or Delete
/// despawned it (and its subtree) mid-game, Ctrl+S wrote `saved_scene.ron`, Ctrl+Z mutated the
/// world — with game input not suppressed either, so both fired.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn editor_shortcuts_do_not_fire_while_the_editor_is_off() {
    let delete = || egui::Event::Key {
        key: egui::Key::Delete,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    let mut app = crate::app::App::new();
    let e = app.world.spawn();
    app.world
        .add_component(e, crate::components::Transform::default());
    app.editor_select_entity(e);
    app.editor.mode = super::EditorMode::Off;

    editor_frame(&mut app, vec![delete()]);
    assert!(
        app.world.is_alive(e),
        "Delete must do nothing while the editor is Off"
    );
    assert_eq!(app.editor.cmd_history.undo_len(), 0, "and record nothing");

    // Control: the same frame with the editor showing deletes and records.
    app.editor.mode = super::EditorMode::Overlay;
    app.editor_select_entity(e);
    editor_frame(&mut app, vec![delete()]);
    assert!(
        !app.world.is_alive(e),
        "control: with the editor showing, Delete deletes"
    );
    assert_eq!(
        app.editor.cmd_history.undo_len(),
        1,
        "control: and records it"
    );
}

// ── The panels do not rewrite what they only display ──────────────────────────

/// One headless egui frame drawing `body` — no window, no GPU.
#[cfg(not(target_arch = "wasm32"))]
fn panel_frame(body: impl FnOnce(&mut egui::Ui)) {
    let ctx = egui::Context::default();
    let mut body = Some(body);
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
        (body.take().expect("one pass"))(ui);
    });
}

/// egui's `DragValue::range` clamps the *existing* value on display by default, and the particle
/// and lighting panels bind the component's fields directly — so merely selecting a rain emitter
/// at `spawn_rate 4000` (the component's own doc's number) wrote `2000` into the world with no
/// interaction, and a light at `intensity 20` became `10`.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn displaying_an_emitter_or_a_light_does_not_rewrite_its_fields() {
    let mut app = crate::app::App::new();
    let e = app.world.spawn();
    app.world.add_component(
        e,
        crate::particle::ParticleEmitter {
            spawn_rate: 4000.0,
            max_per_frame: 10_000,
            ..Default::default()
        },
    );
    app.world.add_component(
        e,
        crate::components::PointLight {
            intensity: 20.0,
            radius: 5000.0,
            ..Default::default()
        },
    );

    panel_frame(|ui| super::ui::particle_tuner_grid(ui, &mut app, e));
    panel_frame(|ui| super::ui::point_light_grid(ui, &mut app, e));

    let em = app
        .world
        .get::<crate::particle::ParticleEmitter>(e)
        .unwrap();
    assert_eq!(em.spawn_rate, 4000.0, "spawn_rate survives being displayed");
    assert_eq!(
        em.max_per_frame, 10_000,
        "max_per_frame survives being displayed"
    );
    let l = app.world.get::<crate::components::PointLight>(e).unwrap();
    assert_eq!(l.intensity, 20.0, "intensity survives being displayed");
    assert_eq!(l.radius, 5000.0, "radius survives being displayed");
}

// ── Snap size zero, and orphans in the Scene tree ──────────────────────────────

/// `snap_size` from the settings file was applied unclamped; the UI's clamps only run while the
/// widget shows. A `0.0` made every snapped drag `(x / 0).round() * 0 = NaN` — the sprite
/// vanished and the NaN move was never recorded, so it could not be undone.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_zero_snap_size_snaps_nothing_instead_of_producing_nan() {
    use super::settings::sanitize_snap_size;
    use super::util::snap_to_grid;
    let v = glam::Vec2::new(5.0, 7.0);
    assert_eq!(snap_to_grid(v, 0.0), v, "zero means no snapping");
    assert_eq!(snap_to_grid(v, f32::NAN), v, "and so does a NaN");
    assert_eq!(
        snap_to_grid(v, 4.0),
        glam::Vec2::new(4.0, 8.0),
        "control: a real size snaps"
    );

    assert_eq!(
        sanitize_snap_size(0.0),
        1.0,
        "a file's 0.0 lands at the toolbar's floor"
    );
    assert_eq!(
        sanitize_snap_size(f32::NAN),
        16.0,
        "a NaN lands at the editor's default"
    );
    assert_eq!(
        sanitize_snap_size(24.0),
        24.0,
        "control: an in-range size is kept"
    );
    let mut s = EditorState::new();
    EditorSettings {
        snap_size: 0.0,
        ..EditorSettings::from_state(&s)
    }
    .apply_to(&mut s);
    assert_eq!(s.snap_size, 1.0, "apply_to sanitizes");

    // The resize path had its own spelling of the division; it goes through `snap_to_grid` now.
    let mut app = crate::app::App::new();
    let e = app.world.spawn();
    let tr = crate::components::Transform::new(glam::Vec2::ZERO, glam::Vec2::splat(20.0), 0.0);
    app.world.add_component(e, tr.clone());
    app.editor_select_entity(e);
    app.editor.snap_enabled = true;
    app.editor.snap_size = 0.0;
    // Press the Right handle at (10, 0), drag to (15, 0): scale.x 20 → 30.
    app.update_transform_gizmo_native(
        e,
        tr.clone(),
        glam::Vec2::new(10.0, 0.0),
        true,
        false,
        false,
    );
    assert!(
        app.editor.resize_handle_active.is_some(),
        "precondition: resizing"
    );
    app.update_transform_gizmo_native(e, tr, glam::Vec2::new(15.0, 0.0), false, true, false);
    let scale = app
        .world
        .get::<crate::components::Transform>(e)
        .unwrap()
        .scale;
    assert!(
        scale.x.is_finite(),
        "a resize with snap size 0 must not produce NaN"
    );
    assert_eq!(scale.x, 30.0);
}

/// The Scene tree's root predicate was "no `Parent` component", so an entity whose parent had
/// been despawned by a raw `World::despawn` was neither a root nor anyone's child and vanished
/// from the tree — the one tool that could have re-parented it — while `HierarchySystem` and
/// `topological_sort_entities` both treat it as a root.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn an_orphan_is_a_root_in_the_scene_tree() {
    use super::ui::scene_tree_inputs;
    let mut app = crate::app::App::new();
    let p = app.world.spawn();
    let c = app.world.spawn();
    app.world.add_component(c, crate::hierarchy::Parent(p));
    let gone = app.world.spawn();
    let orphan = app.world.spawn();
    app.world
        .add_component(orphan, crate::hierarchy::Parent(gone));
    app.world.despawn(gone);

    let list: Vec<Entity> = app.world.entities_sorted();
    let tree = scene_tree_inputs(&app.world, &list);
    assert!(
        tree.roots.contains(&orphan),
        "an orphan is a root, so the tree shows it"
    );
    assert!(tree.roots.contains(&p), "control: a true root is a root");
    assert!(
        !tree.roots.contains(&c),
        "control: a child of a live parent is not"
    );
    assert_eq!(tree.children.get(&p).map(|v| v.as_slice()), Some(&[c][..]));
    assert!(
        tree.graph
            .iter()
            .any(|&(e, parent)| e == orphan && parent.is_none()),
        "the orphan's dead parent is reported as none"
    );
}

// ── A bad path typed into the Data Tables panel ───────────────────────────────

/// The Open button called `App::load_data_table` straight off, whose failure goes to
/// `asset_path::record_failure`: logged, appended to `asset_failures()` (which no editor UI
/// reads), and a **panic** under strict assets. The panel then cleared the typed path and
/// selected the name regardless, so the only trace of a typo was an empty field and a grid
/// saying the table was not found.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_bad_path_in_the_data_table_panel_reports_and_keeps_what_was_typed() {
    // Unique per process, so the `asset_failures()` assertion below cannot collide with another
    // test's entry — that list is process-global.
    let bad = format!(
        "{}/no_such_data_table_{}.ron",
        std::env::temp_dir().to_string_lossy(),
        std::process::id()
    );
    let mut app = crate::app::App::new();
    app.editor.data_table_open_name = "enemies".into();
    app.editor.data_table_open_path = bad.clone();

    let opened = app.editor_open_data_table("enemies".into(), bad.clone());

    assert!(!opened, "a path that does not load must not report success");
    assert!(
        !app.asset_failures().iter().any(|f| f.path == bad),
        "the bad path must never reach `record_failure` — that is what panics under strict assets"
    );
    let status = app
        .editor
        .data_table_status
        .as_deref()
        .expect("the panel says why");
    assert!(
        status.contains(&bad),
        "and the status names the path: {status}"
    );
    assert_eq!(
        app.editor.data_table_open_path, bad,
        "the typed path is kept so the typo can be corrected"
    );
    assert_eq!(app.editor.data_table_open_name, "enemies");
    assert!(
        app.world
            .resource::<crate::data_table::DataTableRegistry>()
            .is_none_or(|r| r.get("enemies").is_none()),
        "and nothing was registered under that name"
    );

    // Control: a file that does load opens, clears the fields, and selects the table — so the
    // assertions above are about the failure, not about Open being broken.
    let good = std::env::temp_dir().join(format!("open_ok_{}.ron", std::process::id()));
    std::fs::write(&good, "[ ( id: \"slime\", hp: 10 ) ]").expect("write");
    let good_path = good.to_string_lossy().to_string();
    app.editor.data_table_open_name = "ok".into();
    app.editor.data_table_open_path = good_path.clone();
    assert!(
        app.editor_open_data_table("ok".into(), good_path),
        "control: a loadable file opens"
    );
    assert_eq!(app.editor.selected_data_table.as_deref(), Some("ok"));
    assert!(
        app.editor.data_table_open_path.is_empty(),
        "control: fields cleared on success"
    );
    assert!(app.editor.data_table_status.is_none());
    let _ = std::fs::remove_file(&good);
}

// ── Where a pasted child's parent tag resolves ────────────────────────────────

/// Filed as a defect: "the tag lookup is first-wins and the original precedes the fresh spawn,
/// so the pasted parent has no child". **It does not reproduce.** A probe on the unmodified
/// paste path put the copy's parent at the copy, not the original, and so did a second paste.
///
/// The reason is `World::query`, which walks archetypes in creation order and then entities
/// within each: at the instant the copied child resolves its tag, the copied parent has no
/// `Children` yet and so sits in an earlier archetype than the original, which does. The right
/// answer therefore comes out of an ordering nothing states. This test is the statement.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn pasting_a_parent_and_child_keeps_them_together() {
    use crate::hierarchy::{Children, Parent};
    let mut app = crate::app::App::new();
    let p = app.world.spawn();
    app.world.add_component(p, Tag("Boss".into()));
    app.world
        .add_component(p, crate::components::Transform::default());
    let c = app.world.spawn();
    app.world.add_component(c, Tag("Minion".into()));
    app.world
        .add_component(c, crate::components::Transform::default());
    assert!(crate::hierarchy::reparent(&mut app.world, c, Some(p)));

    // What Ctrl+C does: a def per selected entity, in selection order.
    app.editor.copy_clipboard = [p, c]
        .iter()
        .filter_map(|&e| entity_to_def(&app.world, e))
        .collect();
    assert_eq!(
        app.editor.copy_clipboard[1].parent.as_deref(),
        Some("Boss"),
        "precondition: the child's def names its parent by tag, so two entities answer to it"
    );
    app.editor_paste_clipboard();

    let pasted = app.editor.selected_entities.clone();
    assert_eq!(pasted.len(), 2, "two entities pasted");
    let (p2, c2) = (pasted[0], pasted[1]);
    assert!(
        p2 != p && c2 != c,
        "precondition: the copies are new entities"
    );
    assert_eq!(
        app.world.get::<Parent>(c2).map(|x| x.0),
        Some(p2),
        "the pasted child belongs to the pasted parent"
    );
    assert_eq!(
        app.world
            .get::<Children>(p)
            .map(|x| x.0.clone())
            .unwrap_or_default(),
        vec![c],
        "and the original parent keeps exactly its own child"
    );

    // A second paste resolves against a world holding two "Boss" copies already, and still lands
    // on its own. That is the case the filed row expected to fail.
    app.editor_paste_clipboard();
    let again = app.editor.selected_entities.clone();
    assert_eq!(
        app.world.get::<Parent>(again[1]).map(|x| x.0),
        Some(again[0]),
        "a second paste lands on its own parent too"
    );
}

/// The other half of the same tag lookup, and the reason a batch-local map alone would be wrong:
/// a child whose parent was **not** copied attaches to the original.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn pasting_a_lone_child_still_attaches_to_the_uncopied_parent() {
    use crate::hierarchy::Parent;
    let mut app = crate::app::App::new();
    let p = app.world.spawn();
    app.world.add_component(p, Tag("Boss".into()));
    app.world
        .add_component(p, crate::components::Transform::default());
    let c = app.world.spawn();
    app.world.add_component(c, Tag("Minion".into()));
    app.world
        .add_component(c, crate::components::Transform::default());
    assert!(crate::hierarchy::reparent(&mut app.world, c, Some(p)));

    app.editor.copy_clipboard = entity_to_def(&app.world, c).into_iter().collect();
    app.editor_paste_clipboard();

    let c2 = app.editor.inspector_selected.expect("the paste selects");
    assert_ne!(c2, c);
    assert_eq!(
        app.world.get::<Parent>(c2).map(|x| x.0),
        Some(p),
        "with no copied parent in the world to prefer, the original is the only answer"
    );
}

// ── Two spellings, and a function that could not say no ───────────────────────

/// `entity_to_def` returned `Some` unconditionally, so a dead handle produced an all-`None` def.
/// The two callers that check for `None` were dead branches guarding a case they could not see.
#[test]
fn entity_to_def_declines_a_dead_entity() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, Tag("Goblin".into()));
    assert!(
        entity_to_def(&world, e).is_some(),
        "control: a live entity still yields a def"
    );
    world.despawn(e);
    assert!(
        entity_to_def(&world, e).is_none(),
        "a dead handle yields nothing, not a def of all-None"
    );
}

/// `gizmo_math::anchor_base` and `UiNode::screen_pos` each had their own copy of the anchor
/// `match` while the gizmo's doc called them "a single authoritative definition". They agreed;
/// nothing made them. Both go through `Anchor::base` now, and this walks every variant.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_gizmo_and_the_ui_node_place_every_anchor_the_same() {
    use crate::ui::{Anchor, UiNode};
    let vp = crate::resources::ViewportSize::new(800, 600);
    let size = glam::Vec2::new(120.0, 40.0);
    let offset = glam::Vec2::new(7.0, -3.0);
    let anchors = [
        Anchor::TopLeft,
        Anchor::TopCenter,
        Anchor::TopRight,
        Anchor::Center,
        Anchor::BottomLeft,
        Anchor::BottomCenter,
        Anchor::BottomRight,
    ];
    let mut seen = std::collections::HashSet::new();
    for a in anchors {
        let node = UiNode {
            anchor: a,
            size,
            offset,
            ..Default::default()
        };
        let from_gizmo = super::ui::anchor_base(a, size, vp.width, vp.height) + offset;
        assert_eq!(
            from_gizmo,
            node.screen_pos(&vp),
            "{a:?} must place the same in both"
        );
        seen.insert((from_gizmo.x.to_bits(), from_gizmo.y.to_bits()));
    }
    assert_eq!(
        seen.len(),
        7,
        "control: the seven anchors land in seven different places, so agreeing is not trivial"
    );
}

// ── Looking at the Ambient Light header does not switch lighting on ───────────

/// The control called `ensure_ambient_light` on every draw. The resource's presence *is* the
/// lighting-pass switch and its default is WHITE × 0.1, so expanding the header dropped a game
/// that used no lighting to 10 % brightness — with no editor path back.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn drawing_the_ambient_light_control_does_not_turn_lighting_on() {
    use crate::resources::AmbientLight;
    let mut app = crate::app::App::new();
    assert!(
        app.world.resource::<AmbientLight>().is_none(),
        "precondition: a fresh App has no AmbientLight, so the lighting pass is off"
    );

    panel_frame(|ui| super::ui::ambient_light_control(ui, &mut app));

    assert!(
        app.world.resource::<AmbientLight>().is_none(),
        "drawing the control must not insert one — that is what switches the pass on"
    );

    // Control: with one present the editors draw and leave it alone, so the assertion above is
    // about the insertion and not about the control refusing to work.
    app.world.insert_resource(AmbientLight {
        color: crate::color::Color::WHITE,
        intensity: 0.75,
    });
    panel_frame(|ui| super::ui::ambient_light_control(ui, &mut app));
    assert_eq!(
        app.world.resource::<AmbientLight>().map(|a| a.intensity),
        Some(0.75),
        "control: an existing AmbientLight is edited, not replaced"
    );
}
