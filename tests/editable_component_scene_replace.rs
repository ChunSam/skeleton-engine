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

/// Loaded atlases and scripts must survive a scene `Replace`.
///
/// `AssetServer` and `ScriptRegistry` are caches — the category `docs/PATTERNS.md` names
/// explicitly as session state that must persist — but `insert_core_resources` rebuilt both empty
/// on every `Replace`. The loss was invisible in the worst possible way: the `SpriteRenderer`
/// texture cache lives on `App`, not in the World, so plain `Sprite`s kept rendering perfectly
/// while `AtlasSprite` (which needs `AssetServer.atlases` for its UV lookup) and every
/// `ScriptRunner` quietly stopped working. Load your sheets at startup — the obvious thing to do —
/// and you got a working first scene and a broken second one, with no error anywhere.
#[test]
fn asset_server_and_script_registry_survive_scene_replace() {
    struct Empty;
    impl engine::scene::Scene for Empty {
        fn on_enter(
            &mut self,
            _w: &mut engine::ecs::World,
            _s: &mut engine::scene::SystemRegistrar,
        ) {
        }
        fn on_exit(&mut self, _w: &mut engine::ecs::World) {}
    }

    let mut app = engine::App::new();
    // A byte-sourced atlas needs no file on disk and keys verbatim, so it is the cleanest probe.
    let png = {
        // 1x1 transparent PNG.
        const BYTES: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        BYTES
    };
    let key = "embedded/__scene_replace_atlas__";
    let handle = app.load_atlas_bytes(key, png, 1, 1);
    assert!(
        app.world
            .resource::<engine::AssetServer>()
            .and_then(|a| a.get_atlas(&handle))
            .is_some(),
        "atlas must be registered before the transition"
    );

    app.set_scene(Box::new(Empty));

    let assets = app
        .world
        .resource::<engine::AssetServer>()
        .expect("AssetServer present after Replace");
    assert!(
        assets.get_atlas(&handle).is_some(),
        "the atlas loaded before the scene change was dropped by the World reset — every \
         pre-existing AtlasSprite would render nothing, silently"
    );
    assert!(
        app.world.resource::<engine::ScriptRegistry>().is_some(),
        "ScriptRegistry must survive the reset too"
    );
}
