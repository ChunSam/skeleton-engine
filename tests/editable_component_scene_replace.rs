//! Acceptance-level regression for the `stat_editor_game` flow: a game component
//! registered with the REAL `#[derive(Reflect)]` macro via `register_editable_component`,
//! plus a `load_data_table`, must survive `set_scene`'s world reset (SceneCmd::Replace).
//!
//! This complements the in-lib unit tests (which use a hand-written `Reflect` impl): it
//! drives the exact public-API path a game uses — derive macro, `register_editable_component`,
//! `load_data_table`, and an entity spawned INSIDE `on_enter` (i.e. during the Replace) —
//! which is what the original bug broke.

use engine::{App, Entity, Scene, SystemRegistrar, World};
use engine_reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

#[derive(Reflect, Serialize, Deserialize, Clone, Default, Debug)]
struct Stats {
    hp: f32,
    strength: i32,
    agility: i32,
}

/// Spawns an entity WITH the registered component during `on_enter` — mirrors the example.
struct GameScene;
impl Scene for GameScene {
    fn on_enter(&mut self, world: &mut World, _s: &mut SystemRegistrar) {
        let e = world.spawn();
        world.add_component(
            e,
            Stats {
                hp: 30.0,
                strength: 5,
                agility: 8,
            },
        );
    }
    fn on_exit(&mut self, _world: &mut World) {}
}

fn stats_entity(app: &App) -> Entity {
    app.world
        .query::<Stats>()
        .map(|(e, _)| e)
        .next()
        .expect("entity with Stats must exist after on_enter")
}

#[test]
fn editable_component_survives_set_scene() {
    let mut app = App::new();
    app.register_editable_component::<Stats>("Stats", None);
    app.set_scene(Box::new(GameScene));

    let e = stats_entity(&app);
    let tid = std::any::TypeId::of::<Stats>();

    // (a) Reflect registration survived → Inspector can show it.
    assert!(
        app.world.reflected_components(e).contains(&tid),
        "Stats must be reflect-registered after set_scene (Inspector visibility)"
    );

    // (b) Serde registration survived → Save Scene serializes it.
    let registry = app
        .world
        .resource::<engine::SerdeComponentRegistry>()
        .expect("SerdeComponentRegistry present");
    let serialized = registry.serialize_entity(&app.world, e);
    assert!(
        serialized.contains_key("Stats"),
        "Stats must serialize after set_scene; got keys {:?}",
        serialized.keys().collect::<Vec<_>>()
    );
}

#[test]
fn data_table_survives_set_scene() {
    let dir = std::env::temp_dir();
    let path = dir.join("editable_component_scene_replace_enemies.ron");
    std::fs::write(
        &path,
        r#"[(name: "Goblin", hp: 30, strength: 5, agility: 8)]"#,
    )
    .expect("write temp data table");

    let mut app = App::new();
    app.load_data_table("enemies", path.to_str().unwrap());
    app.set_scene(Box::new(GameScene));

    let reg = app
        .world
        .resource::<engine::DataTableRegistry>()
        .expect("DataTableRegistry must survive set_scene");
    assert!(
        reg.get("enemies").is_some(),
        "enemies table must survive set_scene"
    );

    let _ = std::fs::remove_file(&path);
}

/// The four render-modifier components must survive **Save Scene**.
///
/// `Hidden`, `RenderLayer`, `SpriteFlip` and `YSort` were `register_clone`d (so copy/paste and
/// scene reset carried them) and were editor-addable, but never serde-registered — and
/// `serialize_entity` walks only the serde registry. So scene save dropped them in total silence.
///
/// The eye 👁 toggle in the docked entity list adds `Hidden`, which makes this the most common
/// editor gesture whose result did not survive Ctrl+S: hide three entities, save, reload, and all
/// three are visible again with no error anywhere. All four derive `Serialize + Deserialize +
/// Clone`, so none of them was transient by design.
#[test]
fn render_modifier_components_are_serde_registered() {
    let mut app = engine::App::new();
    let e = app.world.spawn();
    app.world.add_component(e, engine::Hidden);
    app.world.add_component(e, engine::RenderLayer(3));
    app.world.add_component(e, engine::SpriteFlip::horizontal());
    app.world.add_component(e, engine::YSort::default());

    let registry = app
        .world
        .resource::<engine::SerdeComponentRegistry>()
        .expect("SerdeComponentRegistry present");
    let serialized = registry.serialize_entity(&app.world, e);

    for name in ["Hidden", "RenderLayer", "SpriteFlip", "YSort"] {
        assert!(
            serialized.contains_key(name),
            "{name} is not serde-registered, so Save Scene silently drops it; got keys {:?}",
            serialized.keys().collect::<Vec<_>>()
        );
    }
}
