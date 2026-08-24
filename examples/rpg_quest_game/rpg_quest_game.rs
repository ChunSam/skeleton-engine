//! `rpg_quest_game` — the RPG genre-game of the rebuilt examples tree.
//!
//! Phase 2 of `plans/2026-08-19-examples-rebuild-plan.md`. Where `platformer_game` owns *motion*,
//! this one owns **state that must survive**: the scene stack, the persistence registry and the
//! save file all answer the same question — what is allowed to be dropped — and the v0.139.1 audit
//! showed that question is only answerable under real transitions.
//!
//! ```text
//! cargo run --example rpg_quest_game                          # play it
//! RPG_QUEST_SELFTEST=1 cargo run --example rpg_quest_game     # the acceptance test (headless)
//! ```
//!
//! # Three scenes, because one scene cannot ask the question
//!
//! - **Town** — the merchant, the quest gate, the mine entrance. Entered by `Replace`.
//! - **Mine** — `PathGrid` + A*, a `BehaviorTree` guard, the coroutine cutscene. Entered by
//!   `start_scene_transition`, which covers, swaps and reveals.
//! - **Pause** — `Push`ed on top, so the world underneath is *suspended, not destroyed*. This is
//!   the scene that makes `Push`/`Pop` mean something: everything the player was doing is still
//!   there when it pops.
//!
//! ⚠️ **Every system is registered inside `Scene::on_enter`, never with `App::add_system`.**
//! `SceneCmd::Replace` drains the scene systems and resets the `World`; a system added on the `App`
//! before the first `set_scene` is swept away and never runs again. The engine logs a warning when
//! it discards one, which is easy to miss in a scrolling console.
//!
//! # What survives, and what deliberately does not
//!
//! | Resource | Persistent? | Why |
//! |---|---|---|
//! | `QuestState` | ✅ registered | gold and quest progress are the save file |
//! | `DialogueVars` | ✅ registered | choice gates read these; losing them un-completes the quest |
//! | `Settings` | ✅ registered | volume/locale are session config, not scene state |
//! | `SceneVisits` | ❌ **not** registered | a per-scene counter; it is *correct* for it to reset |
//!
//! The last row is not an oversight — it is the other half of check 1. A persistence registry that
//! preserved everything would pass a check that only tested survival.

use engine::{
    save, App, BehaviorNode, BehaviorStatus, BehaviorTree, Camera, CheckBox, Color, Coroutine,
    CoroutineRunner, CoroutineSystem, DataTableRegistry, DialogueBox, DialogueChoice, DialogueCond,
    DialogueEffect, DialogueOp, DialogueSystem, DialogueValue, DialogueVars, DrawText, Dropdown,
    Entity, Events, InputState, KeyCode, LayoutDir, LayoutSystem, LocaleData, LocaleResource,
    LocalizationSystem, LocalizedText, NineSlice, Panel, PathGrid, SaveMigrator, Scene,
    SceneChange, SceneCmd, ScrollView, Slider, Sprite, SystemConfig, SystemRegistrar, TabBar,
    TextInput, TextQueue, Tooltip, Transform, TransitionStyle, UiEvent, UiNode, UiSystem, World,
};
use glam::{IVec2, Vec2};
use serde::{Deserialize, Serialize};

// ── Window / layout ─────────────────────────────────────────────────────────────────────────────

const WINDOW_W: u32 = 960;
const WINDOW_H: u32 = 600;
const TILE: f32 = 48.0;

const SAVE_APP: &str = "skeleton-engine-rpg-quest";
const SAVE_FILE: &str = "quest.sav";
/// Schema version the game writes. `SaveMigrator` upgrades anything older on load.
const SAVE_VERSION: u32 = 1;

const ITEMS_TABLE: &str = "items";
const ITEMS_PATH: &str = "examples/rpg_quest_game/assets/items.ron";
const TOWN_SCENE_PATH: &str = "examples/rpg_quest_game/assets/town.scene.ron";

/// Gold the player starts with — one short of the lantern, so the quest gate has to open.
const START_GOLD: i64 = 12;

// ── Persistent state ────────────────────────────────────────────────────────────────────────────

/// The save file, and the thing that must survive a scene `Replace`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct QuestState {
    gold: i64,
    mine_cleared: bool,
    /// Incremented in `TownScene::on_enter`. `Push`/`Pop` must never move it — check 2 reads it.
    town_enters: u32,
    /// Added in save schema v1. A v0 save has no such field, and the migrator supplies it.
    lantern_name: String,
    /// ⚠️ **The quest flags are mirrored here as scalars, and that is not redundancy.**
    ///
    /// `DialogueVars` is the authority while the game runs — choice gates evaluate against it —
    /// and it is `Serialize`, so the obvious save payload is the whole bag. It cannot be: a save
    /// that still needs *migrating* is routed through a `ron::Value`, which cannot represent enum
    /// variants, and `DialogueVars` is a map of `String -> DialogueValue` (an enum). Measured, not
    /// assumed — the first version of this save shipped the bag and check 4 failed with
    /// `Expected enum DialogueValue but found a sequence`. So the save carries scalars and
    /// `rebuild_vars` reconstitutes the bag on load. `save_versioned`'s own docs prescribe exactly
    /// this ("mirror them into structs while a migration is pending").
    has_lantern: bool,
    knows_mine: bool,
}

impl Default for QuestState {
    fn default() -> Self {
        Self {
            gold: START_GOLD,
            mine_cleared: false,
            town_enters: 0,
            lantern_name: "brass lantern".to_string(),
            has_lantern: false,
            knows_mine: false,
        }
    }
}

/// Session config: sound, subtitles, language. Persistent — a scene change is not a settings reset.
#[derive(Debug, Clone, PartialEq)]
struct Settings {
    volume: f32,
    subtitles: bool,
    locale: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: 0.7,
            subtitles: true,
            locale: "en".to_string(),
        }
    }
}

/// Deliberately **not** registered persistent: a per-scene counter that is *supposed* to reset.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct SceneVisits {
    steps_here: u32,
}

/// Which scene is on top, for the HUD and for the selftest to read without downcasting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Where {
    Town,
    Mine,
    Paused,
}

/// Live handles for the scene currently running. Rebuilt by each `on_enter`, so it is *not*
/// persistent — the entities it names do not survive the world reset either.
struct Stage {
    player: Entity,
    merchant: Option<Entity>,
    guard: Option<Entity>,
    dialogue: Option<Entity>,
    place: Where,
    /// Set when the player finishes the mine, so the cutscene only plays once.
    cutscene_played: bool,
}

// ── Dialogue variables ──────────────────────────────────────────────────────────────────────────
//
// These live in `DialogueVars` rather than in `QuestState` because choice gates read them, and
// `DialogueVars` is what `DialogueChoice::cond_all` evaluates against. It is serializable for
// exactly this reason — the save round-trips the whole bag instead of naming every flag.

const VAR_LANTERN: &str = "has_lantern";
const VAR_KNOWS_MINE: &str = "knows_mine";
const VAR_GOLD: &str = "gold";

/// The quest gate: the mine is enterable only with **both** the lantern and the directions.
/// `cond_all` is the point — check 3 drives all four combinations of the two terms.
fn mine_gate() -> Vec<DialogueCond> {
    vec![
        DialogueCond::new(VAR_LANTERN, DialogueOp::Eq, DialogueValue::Bool(true)),
        DialogueCond::new(VAR_KNOWS_MINE, DialogueOp::Eq, DialogueValue::Bool(true)),
    ]
}

fn gate_open(vars: &DialogueVars) -> bool {
    mine_gate().iter().all(|c| c.eval(vars))
}

// ── Save ────────────────────────────────────────────────────────────────────────────────────────

/// What a save file holds. Scalars only — see the note on `QuestState::has_lantern`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SaveData {
    quest: QuestState,
}

/// v0 → v1 added `lantern_name` to `QuestState`. A v0 save has no such key, and RON deserialization
/// of the *whole* struct fails without it — so the migrator inserts a default before serde sees it.
///
/// ⚠️ The step operates on a `ron::Value`, which cannot represent enum variants; keep enum-carrying
/// fields out of a schema that still needs migrating (see `save_versioned`'s docs).
fn migrator() -> SaveMigrator {
    SaveMigrator::new().step(0, |value| {
        let mut value = value;
        if let ron::Value::Map(map) = &mut value {
            // `ron::Map` has no `get_mut` — iterate to find the field. Iterating is also the only
            // way that stays correct if the payload ever grows more top-level keys.
            for (key, entry) in map.iter_mut() {
                if !matches!(key, ron::Value::String(k) if k == "quest") {
                    continue;
                }
                if let ron::Value::Map(quest) = entry {
                    quest.insert(
                        ron::Value::String("lantern_name".into()),
                        ron::Value::String("brass lantern".into()),
                    );
                }
            }
        }
        value
    })
}

fn save_file() -> std::path::PathBuf {
    save::save_path(SAVE_APP, SAVE_FILE)
}

/// Snapshots the live dialogue flags into `QuestState`, then writes the versioned envelope.
fn write_save(world: &mut World) -> Result<(), save::SaveError> {
    snapshot_vars(world);
    let data = SaveData {
        quest: world.resource::<QuestState>().cloned().unwrap_or_default(),
    };
    save::save_versioned(&save_file(), SAVE_VERSION, &data)
}

/// Copies `DialogueVars` (the authority in play) into `QuestState` (the authority on disk).
fn snapshot_vars(world: &mut World) {
    let Some(vars) = world.resource::<DialogueVars>().cloned() else {
        return;
    };
    if let Some(q) = world.resource_mut::<QuestState>() {
        q.has_lantern = vars.get_bool(VAR_LANTERN).unwrap_or(false);
        q.knows_mine = vars.get_bool(VAR_KNOWS_MINE).unwrap_or(false);
    }
}

/// The inverse: rebuilds the dialogue bag from a loaded `QuestState`. Without this a load would
/// restore the gold and silently forget the quest, because the gates read `DialogueVars`.
fn rebuild_vars(world: &mut World, quest: &QuestState) {
    let mut vars = DialogueVars::new();
    vars.set_bool(VAR_LANTERN, quest.has_lantern);
    vars.set_bool(VAR_KNOWS_MINE, quest.knows_mine);
    vars.set_int(VAR_GOLD, quest.gold);
    world.insert_resource(vars);
}

fn read_save() -> Result<SaveData, save::SaveError> {
    save::load_migrated(&save_file(), &migrator())
}

// ── Localization ────────────────────────────────────────────────────────────────────────────────
//
// Built in code rather than loaded from RON so the two locales cannot drift apart silently: every
// key is written twice, side by side, and a missing translation is a compile-time-visible hole
// rather than a runtime fallback to the key string.

const LOCALES: [&str; 2] = ["en", "ko"];

fn locale_resource() -> LocaleResource {
    let mut locale = LocaleResource::new("en");
    let pairs: &[(&str, &str, &str)] = &[
        ("ui.settings", "Settings", "설정"),
        ("ui.volume", "Volume", "음량"),
        ("ui.subtitles", "Subtitles", "자막"),
        ("ui.language", "Language", "언어"),
        ("ui.quest_log", "Quest log", "퀘스트 기록"),
        ("ui.save_name", "Save name", "저장 이름"),
        ("ui.resume", "Resume", "계속하기"),
        ("npc.merchant", "Merchant", "상인"),
        (
            "line.greet",
            "The mine has been dark for weeks.",
            "광산이 몇 주째 어둡습니다.",
        ),
        (
            "line.after",
            "Mind the guard. He does not like visitors.",
            "경비를 조심하세요. 방문객을 싫어합니다.",
        ),
        ("choice.buy", "Buy the lantern", "등불을 산다"),
        ("choice.ask", "Ask about the mine", "광산에 대해 묻는다"),
        ("choice.leave", "Leave", "떠난다"),
    ];
    for (i, name) in LOCALES.iter().enumerate() {
        let mut data = LocaleData::default();
        for (key, en, ko) in pairs {
            let text = if i == 0 { *en } else { *ko };
            data.translations
                .insert((*key).to_string(), text.to_string());
        }
        locale.insert_locale(*name, data);
    }
    locale
}

// ── Data table ──────────────────────────────────────────────────────────────────────────────────

/// The lantern's price, read from the `items` data table rather than hardcoded — so the hot-reload
/// check has something whose change is observable in play.
fn lantern_price(world: &World) -> i64 {
    world
        .resource::<DataTableRegistry>()
        .and_then(|reg| reg.get(ITEMS_TABLE))
        .and_then(|table| {
            (0..table.rows.len()).find_map(|row| {
                let id = table.get(row, "id")?;
                if !matches!(id, ron::Value::String(s) if s == "lantern") {
                    return None;
                }
                match table.get(row, "price")? {
                    ron::Value::Number(n) => n.as_i64(),
                    _ => None,
                }
            })
        })
        .unwrap_or(10)
}

// ── Scenes ──────────────────────────────────────────────────────────────────────────────────────
//
// Each scene registers its OWN systems through the `SystemRegistrar`. `App::add_system` would
// survive until the first `Replace` and then vanish — see the header.

/// The town: merchant, quest gate, mine entrance.
struct TownScene;

impl Scene for TownScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        // Persistent resources arrive already populated (the registry re-inserted them after the
        // world reset); non-persistent ones are gone and are rebuilt here.
        if world.resource::<QuestState>().is_none() {
            world.insert_resource(QuestState::default());
        }
        if world.resource::<DialogueVars>().is_none() {
            let mut vars = DialogueVars::new();
            vars.set_bool(VAR_LANTERN, false);
            vars.set_bool(VAR_KNOWS_MINE, false);
            vars.set_int(VAR_GOLD, START_GOLD);
            world.insert_resource(vars);
        }
        if let Some(q) = world.resource_mut::<QuestState>() {
            q.town_enters += 1;
        }
        // Seed-if-absent, never overwrite. An `on_enter` that blindly re-inserts makes the
        // persistence registry untestable: a resource that *was* preserved would be clobbered here
        // and read as dropped. Measured — sabotaging `SceneVisits` into the persistent set left
        // check 1 green until this became conditional.
        if world.resource::<SceneVisits>().is_none() {
            world.insert_resource(SceneVisits::default());
        }

        let player = spawn_actor(world, Vec2::new(160.0, 380.0), Color::rgb(0.45, 0.75, 1.0));
        let merchant = spawn_actor(world, Vec2::new(560.0, 300.0), Color::rgb(0.95, 0.8, 0.35));
        let dialogue = world.spawn();
        world.add_component(dialogue, merchant_dialogue(world));
        // A 9-patch frame around the dialogue box: the one place a `NineSlice` earns its keep,
        // because the box grows with the longest line and a stretched sprite would smear its border.
        world.add_component(
            dialogue,
            Transform {
                position: Vec2::new(WINDOW_W as f32 * 0.5, WINDOW_H as f32 - 96.0),
                scale: Vec2::new(680.0, 132.0),
                z: 0.6,
                ..Default::default()
            },
        );
        world.add_component(dialogue, Sprite::colored(0.10, 0.11, 0.18));
        world.add_component(dialogue, NineSlice::uniform(10.0, 0.25));

        // Town props come from a prefab `SceneDef` on disk, so the layout is data rather than code.
        if let Ok(scene_def) = engine::SceneDef::load(std::path::Path::new(TOWN_SCENE_PATH)) {
            engine::spawn_scene_def(world, &scene_def);
        }

        world.insert_resource(Stage {
            player,
            merchant: Some(merchant),
            guard: None,
            dialogue: Some(dialogue),
            place: Where::Town,
            cutscene_played: false,
        });
        world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

        register_common(systems);
        systems.add(TownSystem);
    }
}

/// The mine: a walkable grid, an A* route, and a guard driven by a behaviour tree.
struct MineScene;

impl Scene for MineScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        // Seed-if-absent, never overwrite. An `on_enter` that blindly re-inserts makes the
        // persistence registry untestable: a resource that *was* preserved would be clobbered here
        // and read as dropped. Measured — sabotaging `SceneVisits` into the persistent set left
        // check 1 green until this became conditional.
        if world.resource::<SceneVisits>().is_none() {
            world.insert_resource(SceneVisits::default());
        }

        // A small blocked-out grid: the pillars are what make the A* route a *route* rather than a
        // straight line, so a broken pathfinder is visible as a guard walking into rock.
        let mut grid = PathGrid::new(MINE_W, MINE_H);
        for &(x, y) in MINE_PILLARS {
            grid.set_walkable(x, y, false);
        }
        world.insert_resource(grid);

        // Draw what the grid means, or the mine reads as an empty room and the guard's detour
        // around invisible rock looks like a bug in his walk.
        let floor = world.spawn();
        world.add_component(
            floor,
            Transform {
                position: MINE_ORIGIN
                    + Vec2::new((MINE_W - 1) as f32, (MINE_H - 1) as f32) * TILE * 0.5,
                scale: Vec2::new(MINE_W as f32 * TILE, MINE_H as f32 * TILE),
                z: -1.0,
                ..Default::default()
            },
        );
        world.add_component(floor, Sprite::colored(0.13, 0.12, 0.15));
        for &(x, y) in MINE_PILLARS {
            let rock = world.spawn();
            world.add_component(
                rock,
                Transform {
                    position: mine_world_pos(IVec2::new(x, y)),
                    scale: Vec2::splat(TILE),
                    z: -0.5,
                    ..Default::default()
                },
            );
            world.add_component(rock, Sprite::colored(0.28, 0.26, 0.30));
        }
        let ore = world.spawn();
        world.add_component(
            ore,
            Transform {
                position: mine_world_pos(ORE_CELL),
                scale: Vec2::splat(TILE * 0.6),
                z: 0.2,
                ..Default::default()
            },
        );
        world.add_component(ore, Sprite::colored(0.95, 0.85, 0.35));

        let player = spawn_actor(
            world,
            mine_world_pos(IVec2::new(1, 1)),
            Color::rgb(0.45, 0.75, 1.0),
        );
        let guard = spawn_actor(
            world,
            mine_world_pos(GUARD_HOME),
            Color::rgb(0.95, 0.4, 0.4),
        );
        world.add_component(guard, GuardBrain::new());
        world.add_component(
            guard,
            BehaviorTree::new(Box::new(engine::Sequence::new(vec![
                Box::new(PickNextWaypoint),
                Box::new(WalkToWaypoint),
            ]))),
        );

        world.insert_resource(Stage {
            player,
            merchant: None,
            guard: Some(guard),
            dialogue: None,
            place: Where::Mine,
            cutscene_played: false,
        });
        world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
        world.insert_resource(CoroutineRunner::new());

        register_common(systems);
        systems.add(MineSystem);
        systems.add(GuardSystem);
        systems.add_labeled(
            CoroutineSystem,
            SystemConfig::new().label("game::coroutine"),
        );
    }
}

/// The pause overlay: `Push`ed, so the scene underneath keeps every entity and resource it had.
/// This is where the UI set lives — and where the locale switch has to retranslate live.
struct PauseScene;

impl Scene for PauseScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        let settings = world.resource::<Settings>().cloned().unwrap_or_default();

        let root = world.spawn();
        world.add_component(root, UiNode::new(240.0, 90.0, 480.0, 420.0).with_z(4.0));
        world.add_component(
            root,
            Panel::new(LayoutDir::Vertical)
                .with_gap(14.0)
                .with_padding(18.0),
        );
        // The panel draws its own background through `UiSystem`; a `Sprite` here would need a
        // `Transform` and would draw in world space, behind the scene rather than over it.

        let mut widgets = Vec::new();

        let tabs = world.spawn();
        world.add_component(tabs, UiNode::new(0.0, 0.0, 440.0, 34.0).with_z(4.1));
        world.add_component(
            tabs,
            TabBar::new(["Settings", "Quest log"]).with_selected(0),
        );
        world.add_component(tabs, PauseWidget::Tabs);
        widgets.push(tabs);

        // `LocalizationSystem` writes into `Label.text`, `Button.label`, `CheckBox.label` and
        // `TextInput.placeholder` — and nowhere else. A `LocalizedText` on a Slider or a Dropdown
        // is inert, so those rows carry their own `Label` beside the control instead.
        let labelled = |world: &mut World, key: &str, kind: PauseWidget| -> Entity {
            let e = world.spawn();
            world.add_component(e, UiNode::new(0.0, 0.0, 440.0, 24.0).with_z(4.1));
            world.add_component(e, engine::Label::new(""));
            world.add_component(e, LocalizedText::new(key));
            world.add_component(e, kind);
            e
        };

        widgets.push(labelled(world, "ui.settings", PauseWidget::Title));

        widgets.push(labelled(world, "ui.volume", PauseWidget::Volume));
        let volume = world.spawn();
        world.add_component(volume, UiNode::new(0.0, 0.0, 440.0, 28.0).with_z(4.1));
        world.add_component(volume, Slider::new(0.0, 1.0, settings.volume));
        world.add_component(volume, Tooltip::new("0 mutes the town bell"));
        world.add_component(volume, PauseWidget::Volume);
        widgets.push(volume);

        // The checkbox and the text input are themselves translation targets, so they take the
        // `LocalizedText` directly — no sibling label.
        let subtitles = world.spawn();
        world.add_component(subtitles, UiNode::new(0.0, 0.0, 440.0, 28.0).with_z(4.1));
        world.add_component(
            subtitles,
            CheckBox::new("").with_checked(settings.subtitles),
        );
        world.add_component(subtitles, LocalizedText::new("ui.subtitles"));
        world.add_component(subtitles, PauseWidget::Subtitles);
        widgets.push(subtitles);

        widgets.push(labelled(world, "ui.language", PauseWidget::Language));
        let language = world.spawn();
        world.add_component(language, UiNode::new(0.0, 0.0, 440.0, 28.0).with_z(4.1));
        world.add_component(
            language,
            Dropdown::new(LOCALES.iter().map(|s| s.to_string()).collect::<Vec<_>>())
                .with_selected(locale_index(&settings.locale)),
        );
        world.add_component(language, PauseWidget::Language);
        widgets.push(language);

        widgets.push(labelled(world, "ui.quest_log", PauseWidget::Log));
        let log = world.spawn();
        world.add_component(log, UiNode::new(0.0, 0.0, 440.0, 96.0).with_z(4.1));
        world.add_component(
            log,
            ScrollView::new().with_items(vec![
                "Find the merchant".to_string(),
                "Buy a lantern".to_string(),
                "Ask about the mine".to_string(),
                "Clear the mine".to_string(),
            ]),
        );
        world.add_component(log, PauseWidget::Log);
        widgets.push(log);

        let name = world.spawn();
        world.add_component(name, UiNode::new(0.0, 0.0, 440.0, 30.0).with_z(4.1));
        world.add_component(name, TextInput::new(""));
        world.add_component(name, LocalizedText::new("ui.save_name"));
        world.add_component(name, PauseWidget::SaveName);
        widgets.push(name);

        // `Panel.children` is what `LayoutSystem` positions — **not** the `Parent`/`Children`
        // hierarchy. Attaching instead of filling this vec leaves every widget at its own UiNode
        // origin, which stacks the whole menu in the top-left corner and still renders, so it
        // looks like a styling problem rather than a wiring one.
        if let Some(panel) = world.get_mut::<Panel>(root) {
            panel.children = widgets.clone();
        }
        world.insert_resource(PauseUi { root, widgets });

        systems.add_labeled(
            LocalizationSystem,
            SystemConfig::new()
                .label(LocalizationSystem::LABEL)
                .before(UiSystem::LABEL),
        );
        systems.add_labeled(
            LayoutSystem,
            SystemConfig::new()
                .label(LayoutSystem::LABEL)
                .before(UiSystem::LABEL),
        );
        systems.add_labeled(
            UiSystem::default(),
            SystemConfig::new().label(UiSystem::LABEL),
        );
        systems.add_labeled(
            PauseSystem,
            SystemConfig::new()
                .label("game::pause")
                .after(UiSystem::LABEL),
        );
    }

    fn on_exit(&mut self, world: &mut World) {
        // The pause scene owns its widgets: `Pop` does not reset the world, so nothing else will
        // remove them and they would draw over the resumed scene forever.
        if let Some(ui) = world.remove_resource::<PauseUi>() {
            for e in ui.widgets {
                world.despawn(e);
            }
            world.despawn(ui.root);
        }
    }
}

/// Systems every playable scene needs. Kept in one place so a scene cannot forget one.
fn register_common(systems: &mut SystemRegistrar) {
    systems.add_labeled(DialogueSystem, SystemConfig::new().label("game::dialogue"));
    systems.add_labeled(
        PlayerSystem,
        SystemConfig::new()
            .label("game::player")
            .after("game::dialogue"),
    );
    systems.add_labeled(HudSystem, SystemConfig::new().label("game::hud"));
}

// ── Mine geometry ───────────────────────────────────────────────────────────────────────────────

const MINE_W: i32 = 13;
const MINE_H: i32 = 9;
const MINE_ORIGIN: Vec2 = Vec2::new(120.0, 110.0);
/// Blocked cells. They sit between the guard's waypoints on purpose: with them, the A* route has
/// to go around, so a pathfinder that returned a straight line would walk the guard through rock.
const MINE_PILLARS: &[(i32, i32)] = &[
    (4, 1),
    (4, 2),
    (4, 3),
    (4, 4),
    (8, 4),
    (8, 5),
    (8, 6),
    (8, 7),
];
const GUARD_HOME: IVec2 = IVec2::new(2, 6);
/// Chosen so that **two of the four legs are blocked in a straight line** — the pillars sit across
/// them, so the route has to go around and a naive interpolation would walk through rock. A patrol
/// whose legs an unobstructed monotone path can join does not test the pathfinder at all.
const GUARD_WAYPOINTS: &[IVec2] = &[
    IVec2::new(2, 6),
    IVec2::new(11, 6),
    IVec2::new(11, 1),
    IVec2::new(2, 1),
];
/// The ore the mine run is for.
const ORE_CELL: IVec2 = IVec2::new(11, 4);

fn mine_world_pos(cell: IVec2) -> Vec2 {
    MINE_ORIGIN + Vec2::new(cell.x as f32 * TILE, cell.y as f32 * TILE)
}

fn mine_cell_at(pos: Vec2) -> IVec2 {
    let rel = (pos - MINE_ORIGIN) / TILE;
    IVec2::new(rel.x.round() as i32, rel.y.round() as i32)
}

// ── Actors ──────────────────────────────────────────────────────────────────────────────────────

const ACTOR_SIZE: Vec2 = Vec2::new(30.0, 38.0);
const WALK_SPEED: f32 = 190.0;
const GUARD_SPEED: f32 = 120.0;
/// How close counts as "arrived" at a waypoint, in pixels.
const ARRIVE_EPS: f32 = 3.0;

fn spawn_actor(world: &mut World, pos: Vec2, color: Color) -> Entity {
    let e = world.spawn();
    world.add_component(
        e,
        Transform {
            position: pos,
            scale: ACTOR_SIZE,
            z: 0.5,
            ..Default::default()
        },
    );
    world.add_component(e, Sprite::colored(color.r, color.g, color.b));
    e
}

// ── The merchant's conversation ─────────────────────────────────────────────────────────────────

/// Line 0 greets; its choices are the quest. The buy option is gated by a **conjunction** — enough
/// gold *and* not already carrying one — which is the shape `cond_all` exists for and the shape
/// check 3 drives from all four corners.
fn merchant_dialogue(world: &World) -> DialogueBox {
    let price = lantern_price(world);
    DialogueBox::localized("npc.merchant", ["line.greet", "line.after"])
        .with_chars_per_sec(0.0)
        .with_choices(
            0,
            [
                DialogueChoice::localized("choice.buy", 1)
                    .when_all([
                        DialogueCond::new(VAR_GOLD, DialogueOp::Ge, DialogueValue::Int(price)),
                        DialogueCond::new(VAR_LANTERN, DialogueOp::Eq, DialogueValue::Bool(false)),
                    ])
                    .then(DialogueEffect::SetVar {
                        key: VAR_LANTERN.to_string(),
                        value: DialogueValue::Bool(true),
                    }),
                DialogueChoice::localized("choice.ask", 1)
                    .when(DialogueCond::new(
                        VAR_KNOWS_MINE,
                        DialogueOp::Eq,
                        DialogueValue::Bool(false),
                    ))
                    .then(DialogueEffect::SetVar {
                        key: VAR_KNOWS_MINE.to_string(),
                        value: DialogueValue::Bool(true),
                    }),
                DialogueChoice::localized("choice.leave", 1),
            ],
        )
}

// ── Guard behaviour tree ────────────────────────────────────────────────────────────────────────

/// Per-guard state the behaviour nodes share. A `Blackboard` would do, but the route is a
/// `Vec<IVec2>` recomputed by A* and this keeps the type honest.
struct GuardBrain {
    waypoint: usize,
    route: Vec<IVec2>,
    step: usize,
    /// Counts completed patrol legs — the observable a broken tree would leave at zero.
    legs: u32,
}

impl GuardBrain {
    fn new() -> Self {
        Self {
            waypoint: 0,
            route: Vec::new(),
            step: 0,
            legs: 0,
        }
    }
}

/// Picks the next waypoint and asks A* for a route to it. Fails (so the `Sequence` fails) when no
/// route exists — a guard with nowhere to go should stop, not teleport.
struct PickNextWaypoint;

impl BehaviorNode for PickNextWaypoint {
    fn tick(&mut self, world: &mut World, entity: Entity, _dt: f32) -> BehaviorStatus {
        let has_route = world
            .get::<GuardBrain>(entity)
            .map(|b| b.step < b.route.len())
            .unwrap_or(false);
        if has_route {
            return BehaviorStatus::Success;
        }
        let Some(pos) = world.get::<Transform>(entity).map(|t| t.position) else {
            return BehaviorStatus::Failure;
        };
        let from = mine_cell_at(pos);
        let Some(next_index) = world.get::<GuardBrain>(entity).map(|b| b.waypoint) else {
            return BehaviorStatus::Failure;
        };
        let goal = GUARD_WAYPOINTS[next_index % GUARD_WAYPOINTS.len()];
        let Some(grid) = world.resource::<PathGrid>() else {
            return BehaviorStatus::Failure;
        };
        let Some(route) = engine::find_path(grid, from, goal) else {
            return BehaviorStatus::Failure;
        };
        if let Some(brain) = world.get_mut::<GuardBrain>(entity) {
            brain.route = route;
            brain.step = 0;
        }
        BehaviorStatus::Success
    }
}

/// Walks the route one cell at a time; `Running` until the route is spent, then bumps the leg
/// counter and advances to the next waypoint.
struct WalkToWaypoint;

impl BehaviorNode for WalkToWaypoint {
    fn tick(&mut self, world: &mut World, entity: Entity, dt: f32) -> BehaviorStatus {
        let Some(target_cell) = world
            .get::<GuardBrain>(entity)
            .map(|b| b.route.get(b.step).copied())
        else {
            return BehaviorStatus::Failure;
        };
        let Some(cell) = target_cell else {
            if let Some(brain) = world.get_mut::<GuardBrain>(entity) {
                brain.waypoint = (brain.waypoint + 1) % GUARD_WAYPOINTS.len();
                brain.route.clear();
                brain.step = 0;
                brain.legs += 1;
            }
            return BehaviorStatus::Success;
        };
        let target = mine_world_pos(cell);
        let Some(pos) = world.get::<Transform>(entity).map(|t| t.position) else {
            return BehaviorStatus::Failure;
        };
        let delta = target - pos;
        if delta.length() <= ARRIVE_EPS.max(GUARD_SPEED * dt) {
            if let Some(t) = world.get_mut::<Transform>(entity) {
                t.position = target;
            }
            if let Some(brain) = world.get_mut::<GuardBrain>(entity) {
                brain.step += 1;
            }
        } else if let Some(t) = world.get_mut::<Transform>(entity) {
            t.position += delta.normalize_or_zero() * GUARD_SPEED * dt;
        }
        BehaviorStatus::Running
    }
}

/// Ticks each guard's tree. `BehaviorTree::tick` needs `&mut World` *and* the tree itself, so the
/// tree is taken out of the component store for the call and put back after — the borrow split
/// `docs/PATTERNS.md` describes, in its sharpest form.
struct GuardSystem;

impl engine::System for GuardSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let guards: Vec<Entity> = world.query::<BehaviorTree>().map(|(e, _)| e).collect();
        for guard in guards {
            let Some(mut tree) = world.take_component::<BehaviorTree>(guard) else {
                continue;
            };
            tree.tick(world, guard, dt);
            world.add_component(guard, tree);
        }
    }

    fn name(&self) -> &'static str {
        "GuardSystem"
    }
}

// ── Pause UI plumbing ───────────────────────────────────────────────────────────────────────────

/// Which control a pause-menu entity is, so `PauseSystem` can route a `UiEvent` without holding a
/// separate entity handle per widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PauseWidget {
    Tabs,
    Title,
    Volume,
    Subtitles,
    Language,
    Log,
    SaveName,
}

/// Entities the pause scene owns and must despawn on `Pop` — `Pop` does not reset the world.
struct PauseUi {
    root: Entity,
    widgets: Vec<Entity>,
}

fn locale_index(locale: &str) -> usize {
    LOCALES.iter().position(|l| *l == locale).unwrap_or(0)
}

// ── Systems ─────────────────────────────────────────────────────────────────────────────────────

/// Walks the player, opens the pause scene, and drives the dialogue. Shared by town and mine.
struct PlayerSystem;

impl engine::System for PlayerSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some(place) = world.resource::<Stage>().map(|s| s.place) else {
            return;
        };
        if place == Where::Paused {
            return;
        }

        let (axis, interact, pause, save_now, load_now) = world
            .resource::<InputState>()
            .map(|input| {
                let mut axis = Vec2::ZERO;
                if input.is_pressed(KeyCode::ArrowLeft) || input.is_pressed(KeyCode::KeyA) {
                    axis.x -= 1.0;
                }
                if input.is_pressed(KeyCode::ArrowRight) || input.is_pressed(KeyCode::KeyD) {
                    axis.x += 1.0;
                }
                if input.is_pressed(KeyCode::ArrowUp) || input.is_pressed(KeyCode::KeyW) {
                    axis.y -= 1.0;
                }
                if input.is_pressed(KeyCode::ArrowDown) || input.is_pressed(KeyCode::KeyS) {
                    axis.y += 1.0;
                }
                (
                    axis,
                    input.just_pressed(KeyCode::KeyE) || input.just_pressed(KeyCode::Space),
                    input.just_pressed(KeyCode::Escape),
                    input.just_pressed(KeyCode::F5),
                    input.just_pressed(KeyCode::F9),
                )
            })
            .unwrap_or((Vec2::ZERO, false, false, false, false));

        let Some(player) = world.resource::<Stage>().map(|s| s.player) else {
            return;
        };
        if axis != Vec2::ZERO {
            if let Some(t) = world.get_mut::<Transform>(player) {
                t.position += axis.normalize_or_zero() * WALK_SPEED * dt;
            }
            if let Some(v) = world.resource_mut::<SceneVisits>() {
                v.steps_here += 1;
            }
        }

        if save_now {
            let _ = write_save(world);
        }
        if load_now {
            if let Ok(data) = read_save() {
                rebuild_vars(world, &data.quest);
                world.insert_resource(data.quest);
            }
        }

        if pause {
            if let Some(sc) = world.resource_mut::<SceneChange>() {
                sc.request(SceneCmd::Push(Box::new(PauseScene)));
            }
            if let Some(stage) = world.resource_mut::<Stage>() {
                stage.place = Where::Paused;
            }
        }

        if interact {
            advance_dialogue(world);
        }
    }

    fn name(&self) -> &'static str {
        "PlayerSystem"
    }
}

/// One press advances the conversation, or takes the first *available* choice.
///
/// "Available" is the whole point: `visible_choices` filters by `DialogueVars`, so a gated choice
/// the player has not earned is not merely greyed out — it is not there to take.
fn advance_dialogue(world: &mut World) {
    let Some(entity) = world.resource::<Stage>().and_then(|s| s.dialogue) else {
        return;
    };
    // Talking needs the merchant within earshot. Without this the player could run the whole quest
    // from the far side of town, and "walk to the merchant" would stop being part of the route.
    let in_earshot = world
        .resource::<Stage>()
        .map(|s| (s.player, s.merchant))
        .map(|(player, merchant)| match merchant {
            Some(m) => match (world.get::<Transform>(player), world.get::<Transform>(m)) {
                (Some(p), Some(q)) => p.position.distance(q.position) < 110.0,
                _ => false,
            },
            None => true,
        })
        .unwrap_or(false);
    if !in_earshot {
        return;
    }
    let Some(vars) = world.resource::<DialogueVars>().cloned() else {
        return;
    };
    let choosing = world
        .get::<DialogueBox>(entity)
        .map(|d| d.is_choosing(&vars))
        .unwrap_or(false);
    if choosing {
        engine::dialogue::choose(world, entity, 0);
        apply_purchase(world);
    } else if let Some(d) = world.get_mut::<DialogueBox>(entity) {
        d.advance_with(&vars);
    }
}

/// Charges for the lantern once the dialogue's effect has granted it. The effect sets the flag;
/// the price is game logic, and it reads the data table so a re-priced item takes effect in play.
fn apply_purchase(world: &mut World) {
    let has_lantern = world
        .resource::<DialogueVars>()
        .and_then(|v| v.get_bool(VAR_LANTERN))
        .unwrap_or(false);
    let already_paid = world
        .resource::<QuestState>()
        .map(|q| q.gold < START_GOLD)
        .unwrap_or(false);
    if !has_lantern || already_paid {
        return;
    }
    let price = lantern_price(world);
    if let Some(q) = world.resource_mut::<QuestState>() {
        q.gold -= price;
    }
    let gold = world.resource::<QuestState>().map(|q| q.gold).unwrap_or(0);
    if let Some(vars) = world.resource_mut::<DialogueVars>() {
        vars.set_int(VAR_GOLD, gold);
    }
}

/// Town rules: standing on the mine entrance with the gate open starts the transition.
struct TownSystem;

impl engine::System for TownSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((player, place)) = world.resource::<Stage>().map(|s| (s.player, s.place)) else {
            return;
        };
        if place != Where::Town {
            return;
        }
        let Some(pos) = world.get::<Transform>(player).map(|t| t.position) else {
            return;
        };
        let at_entrance = pos.distance(MINE_ENTRANCE) < 42.0;
        let open = world
            .resource::<DialogueVars>()
            .map(gate_open)
            .unwrap_or(false);
        if at_entrance && open {
            // IrisOut, because this is the transition the docked-editor capture photographs: its
            // mask is a circle only when the shader is handed the *scene target's* aspect.
            engine::start_scene_transition(
                world,
                Box::new(MineScene),
                TransitionStyle::IrisOut,
                0.35,
            );
        }
    }

    fn name(&self) -> &'static str {
        "TownSystem"
    }
}

/// Mine rules: reaching the ore plays the cutscene once, then returns to town.
struct MineSystem;

impl engine::System for MineSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((player, place, played)) = world
            .resource::<Stage>()
            .map(|s| (s.player, s.place, s.cutscene_played))
        else {
            return;
        };
        if place != Where::Mine || played {
            return;
        }
        let Some(pos) = world.get::<Transform>(player).map(|t| t.position) else {
            return;
        };
        if pos.distance(mine_world_pos(ORE_CELL)) > 34.0 {
            return;
        }

        if let Some(stage) = world.resource_mut::<Stage>() {
            stage.cutscene_played = true;
        }
        if let Some(q) = world.resource_mut::<QuestState>() {
            q.mine_cleared = true;
        }
        // The payoff cutscene: hold, drift the camera, then go home. A `Coroutine` rather than a
        // state machine because it is a *sequence in time* and nothing else reads its stages.
        if let Some(runner) = world.resource_mut::<CoroutineRunner>() {
            runner.start(
                Coroutine::new()
                    .wait(0.2)
                    .run_for(0.6, |world, t| {
                        if let Some(cam) = world.resource_mut::<Camera>() {
                            cam.position.x = t * 40.0;
                        }
                    })
                    .run(|world| {
                        engine::start_scene_transition(
                            world,
                            Box::new(TownScene),
                            TransitionStyle::IrisIn,
                            0.35,
                        );
                    }),
            );
        }
    }

    fn name(&self) -> &'static str {
        "MineSystem"
    }
}

/// Routes pause-menu `UiEvent`s into `Settings`, and pops the scene on Escape.
///
/// The language dropdown writes `LocaleResource::set_locale` and nothing else: every label is a
/// `LocalizedText`, so `LocalizationSystem` retranslates them on the next frame with no rebuild.
struct PauseSystem;

impl engine::System for PauseSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let close = world
            .resource::<InputState>()
            .map(|i| i.just_pressed(KeyCode::Escape))
            .unwrap_or(false);

        let mut changes: Vec<(PauseWidget, UiEvent)> = Vec::new();
        if let Some(events) = world.resource::<Events<UiEvent>>() {
            for event in events.read() {
                let entity = match event {
                    UiEvent::SliderChanged(e, _)
                    | UiEvent::CheckBoxToggled(e, _)
                    | UiEvent::DropdownChanged(e, _)
                    | UiEvent::TabChanged(e, _)
                    | UiEvent::TextSubmitted(e, _) => *e,
                    _ => continue,
                };
                if let Some(kind) = world.get::<PauseWidget>(entity) {
                    changes.push((*kind, event.clone()));
                }
            }
        }

        for (kind, event) in changes {
            match (kind, event) {
                (PauseWidget::Volume, UiEvent::SliderChanged(_, v)) => {
                    if let Some(s) = world.resource_mut::<Settings>() {
                        s.volume = v;
                    }
                }
                (PauseWidget::Subtitles, UiEvent::CheckBoxToggled(_, on)) => {
                    if let Some(s) = world.resource_mut::<Settings>() {
                        s.subtitles = on;
                    }
                }
                (PauseWidget::Language, UiEvent::DropdownChanged(_, index)) => {
                    set_locale(world, LOCALES[index.min(LOCALES.len() - 1)]);
                }
                _ => {}
            }
        }

        if close {
            if let Some(sc) = world.resource_mut::<SceneChange>() {
                sc.request(SceneCmd::Pop);
            }
            if let Some(stage) = world.resource_mut::<Stage>() {
                stage.place = if stage.guard.is_some() {
                    Where::Mine
                } else {
                    Where::Town
                };
            }
        }
    }

    fn name(&self) -> &'static str {
        "PauseSystem"
    }
}

/// Switches the active language. Both halves matter: the `LocaleResource` drives every
/// `LocalizedText` *and* the dialogue box's own keys, and `Settings` is what the save carries.
fn set_locale(world: &mut World, locale: &str) {
    if let Some(res) = world.resource_mut::<LocaleResource>() {
        res.set_locale(locale);
    }
    if let Some(s) = world.resource_mut::<Settings>() {
        s.locale = locale.to_string();
    }
}

struct HudSystem;

impl engine::System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((place, gold, cleared)) =
            world
                .resource::<Stage>()
                .map(|s| s.place)
                .and_then(|place| {
                    world
                        .resource::<QuestState>()
                        .map(|q| (place, q.gold, q.mine_cleared))
                })
        else {
            return;
        };
        let open = world
            .resource::<DialogueVars>()
            .map(gate_open)
            .unwrap_or(false);
        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        tq.push(DrawText::new(
            "Move: WASD   Talk/advance: E   Pause: Esc   Save: F5   Load: F9",
            Vec2::new(16.0, 14.0),
            17.0,
            [220, 232, 248, 230],
        ));
        tq.push(DrawText::new(
            format!(
                "{}   gold {gold}   mine {}   gate {}",
                match place {
                    Where::Town => "Town",
                    Where::Mine => "Mine",
                    Where::Paused => "Paused",
                },
                if cleared { "cleared" } else { "dark" },
                if open { "open" } else { "shut" },
            ),
            Vec2::new(16.0, 38.0),
            17.0,
            [168, 208, 255, 220],
        ));
    }

    fn name(&self) -> &'static str {
        "HudSystem"
    }
}

// ── Setup ───────────────────────────────────────────────────────────────────────────────────────

/// Where the mine entrance sits in town.
const MINE_ENTRANCE: Vec2 = Vec2::new(820.0, 300.0);

/// Builds the app and enters the town. The selftest drives *this*, not a reduced copy.
fn build_app() -> App {
    let mut app = App::new();
    app.world.insert_resource(engine::WindowConfig {
        title: "skeleton-engine — rpg_quest_game".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.07, 0.08, 0.13, 1.0],
    });
    app.register_event::<UiEvent>();
    app.register_event::<engine::DialogueEvent>();

    // Item prices live in a data table so a designer can retune them without a rebuild — and so the
    // hot-reload path has an observable consequence in play. `load_data_table` also registers the
    // registry as persistent, so the table survives the scene `Replace` into the mine.
    app.load_data_table(ITEMS_TABLE, ITEMS_PATH);

    app.world.insert_resource(locale_resource());
    app.world.insert_resource(Settings::default());
    app.world.insert_resource(QuestState::default());
    let mut vars = DialogueVars::new();
    vars.set_bool(VAR_LANTERN, false);
    vars.set_bool(VAR_KNOWS_MINE, false);
    vars.set_int(VAR_GOLD, START_GOLD);
    app.world.insert_resource(vars);

    // The persistence contract, in one place. `SceneVisits` is deliberately absent.
    app.register_persistent::<QuestState>();
    app.register_persistent::<DialogueVars>();
    app.register_persistent::<Settings>();
    app.register_persistent::<LocaleResource>();

    // The editor hook: `QuestMarker` becomes inspectable and Save-Scene-able, and the registration
    // is replayed after a world reset (that replay is what `register_editable_component` adds over
    // a bare `register_serde_component`).
    app.register_editable_component::<QuestMarker>("QuestMarker", None);

    app.set_scene(Box::new(TownScene));
    app
}

/// A component that exists to be edited: it carries the quest step a marker entity stands for, and
/// it is what the docked editor's Inspector shows.
#[derive(
    Debug, Clone, Default, serde::Serialize, serde::Deserialize, engine_reflect_derive::Reflect,
)]
struct QuestMarker {
    step: i32,
}

// The engine reports trouble through `log`, which discards everything until a binary installs
// a logger. Every game installs the same one; the module explains what that buys and what it
// still does not cover in a browser.
#[path = "../shared/logging.rs"]
mod logging;

fn main() {
    logging::init();
    if std::env::var("RPG_QUEST_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }
    build_app().run();
}

// ── Acceptance test ─────────────────────────────────────────────────────────────────────────────
//
// `RPG_QUEST_SELFTEST=1 cargo run --example rpg_quest_game`, and `scripts/selftests.sh` in the gate.
//
// Everything runs over `App::step_headless`, which is the only way to cross a scene boundary: a
// hand-ticked schedule never sees `SceneCmd` applied, so it never sees the world reset — which is
// the single thing this game exists to check.
//
// Exit codes: 0 pass · 1 the persistence registry drops what it should keep, or keeps what it
// should drop · 2 `Push`/`Pop` resets scene state · 3 the `cond_all` quest gate opens on a partial
// condition · 4 a save does not round-trip, or a v0 save does not migrate · 5 a locale switch does
// not retranslate · 6 a data-table edit is not picked up · 7 the docked iris is not a circle, or
// the capture could not prove it was measuring the docked case at all.

const DT: f32 = 1.0 / 60.0;

fn quest(app: &App) -> QuestState {
    app.world
        .resource::<QuestState>()
        .cloned()
        .unwrap_or_default()
}

fn vars_of(app: &App) -> DialogueVars {
    app.world
        .resource::<DialogueVars>()
        .cloned()
        .unwrap_or_default()
}

fn step(app: &mut App, frames: u32) {
    for _ in 0..frames {
        app.step_headless(DT);
    }
}

/// Walks the player onto `target` by writing the transform — the routes are long and this test is
/// about scene state, not about locomotion (which `platformer_game` already covers two-sidedly).
fn place_player(app: &mut App, target: Vec2) {
    let Some(player) = app.world.resource::<Stage>().map(|s| s.player) else {
        return;
    };
    if let Some(t) = app.world.get_mut::<Transform>(player) {
        t.position = target;
    }
}

/// Runs the whole merchant conversation: both gated choices, in the order a player would take them.
fn run_quest_dialogue(app: &mut App) {
    place_player(app, Vec2::new(560.0, 300.0));
    for _ in 0..3 {
        // Each pass takes the first *available* choice: buy, then ask, then leave.
        advance_dialogue(&mut app.world);
        if let Some(d) = app.world.resource::<Stage>().and_then(|s| s.dialogue) {
            if let Some(b) = app.world.get_mut::<DialogueBox>(d) {
                b.reset();
            }
        }
        step(app, 1);
    }
}

fn self_test() -> i32 {
    // ── 1. The persistence registry keeps what it must and drops what it must ──────────────────
    //
    // Both halves, in this order. Survival alone passes on an engine that never resets the world —
    // which is exactly the bug this check exists for, and it is invisible in a screenshot because
    // a scene that kept everything looks identical to one that kept the right things.
    {
        let mut app = build_app();
        step(&mut app, 2);
        if let Some(q) = app.world.resource_mut::<QuestState>() {
            q.gold = 999;
        }
        if let Some(v) = app.world.resource_mut::<SceneVisits>() {
            v.steps_here = 77;
        }
        let visits_before = app
            .world
            .resource::<SceneVisits>()
            .map(|v| v.steps_here)
            .unwrap_or(0);

        app.set_scene(Box::new(MineScene));
        step(&mut app, 2);

        let gold_after = quest(&app).gold;
        let visits_after = app
            .world
            .resource::<SceneVisits>()
            .map(|v| v.steps_here)
            .unwrap_or(0);
        if gold_after != 999 || visits_before != 77 || visits_after != 0 {
            eprintln!(
                "FAIL: the scene reset did not respect the persistence registry — registered \
                 QuestState.gold 999 -> {gold_after} (want 999), unregistered SceneVisits \
                 {visits_before} -> {visits_after} (want 0)"
            );
            return 1;
        }
        println!(
            "ok: a Replace keeps the registered QuestState (gold {gold_after}) and drops the \
             unregistered SceneVisits ({visits_before} -> {visits_after})"
        );
    }

    // ── 2. Push/Pop suspends the scene; it does not reset it ──────────────────────────────────
    //
    // `town_enters` is incremented in `TownScene::on_enter`. A `Pop` written as a `Replace` reads
    // 2 here and photographs identically — same town, same merchant, same everything.
    //
    // Driven through the game's own Escape handling rather than `App::push_scene`, because the
    // engine's Push/Pop semantics and this game's wiring of them are different things, and only
    // the second one can be got wrong here. Measured: sabotaging `SceneCmd::Pop` into
    // `SceneCmd::Replace` left an `App::pop_scene`-driven version of this check green, because it
    // never ran the line that was broken.
    {
        let mut app = build_app();
        app.set_input_script(engine::InputScript::new([
            (2, engine::InputAction::KeyPress(KeyCode::Escape)),
            (10, engine::InputAction::KeyPress(KeyCode::Escape)),
        ]));
        step(&mut app, 2);
        let enters_before = quest(&app).town_enters;
        if let Some(v) = app.world.resource_mut::<SceneVisits>() {
            v.steps_here = 42;
        }

        step(&mut app, 6); // the Escape at frame 2 pushes the pause scene
        let paused_widgets = app.world.query::<PauseWidget>().count();
        let place_paused = app.world.resource::<Stage>().map(|s| s.place);
        step(&mut app, 8); // the Escape at frame 10 pops it

        let enters_after = quest(&app).town_enters;
        let visits_after = app
            .world
            .resource::<SceneVisits>()
            .map(|v| v.steps_here)
            .unwrap_or(0);
        let leftover_widgets = app.world.query::<PauseWidget>().count();
        if enters_before != 1
            || enters_after != 1
            || visits_after != 42
            || paused_widgets == 0
            || place_paused != Some(Where::Paused)
            || leftover_widgets != 0
        {
            eprintln!(
                "FAIL: Push/Pop did not suspend the scene — town_enters {enters_before} to \
                 {enters_after} (want 1 to 1), SceneVisits {visits_after} (want 42), place while \
                 open {place_paused:?} (want Paused), pause widgets {paused_widgets} while open \
                 (want more than 0) and {leftover_widgets} after (want 0)"
            );
            return 2;
        }
        println!(
            "ok: Escape pushes and pops rather than resetting — town_enters stays {enters_after}, \
             scene state survives, and all {paused_widgets} pause widgets are cleaned up on Pop"
        );
    }

    // ── 3. The quest gate needs BOTH terms ────────────────────────────────────────────────────
    //
    // All four corners of the conjunction. A gate that fired on either term alone would let the
    // player into the mine without a lantern, and the mine looks the same either way.
    {
        let cases = [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (true, true, true),
        ];
        for (lantern, knows, want) in cases {
            let mut vars = DialogueVars::new();
            vars.set_bool(VAR_LANTERN, lantern);
            vars.set_bool(VAR_KNOWS_MINE, knows);
            if gate_open(&vars) != want {
                eprintln!(
                    "FAIL: the mine gate is not a conjunction — lantern={lantern} knows={knows} \
                     opened={} (want {want})",
                    gate_open(&vars)
                );
                return 3;
            }
        }

        // …and the same conjunction as the player actually meets it: the shop choice is *absent*
        // until its own `cond_all` passes, which is what `visible_choices` filters on.
        let mut app = build_app();
        step(&mut app, 2);
        let dialogue = app
            .world
            .resource::<Stage>()
            .and_then(|s| s.dialogue)
            .expect("town has a dialogue box");
        let before = vars_of(&app);
        let choices_rich = app
            .world
            .get::<DialogueBox>(dialogue)
            .map(|d| d.visible_choices(&before).len())
            .unwrap_or(0);
        if let Some(v) = app.world.resource_mut::<DialogueVars>() {
            v.set_int(VAR_GOLD, 1); // too poor for the lantern
        }
        let poor = vars_of(&app);
        let choices_poor = app
            .world
            .get::<DialogueBox>(dialogue)
            .map(|d| d.visible_choices(&poor).len())
            .unwrap_or(0);
        if choices_rich != 3 || choices_poor != 2 {
            eprintln!(
                "FAIL: the shop choice is not gated on gold — {choices_rich} choices with enough \
                 gold (want 3), {choices_poor} without (want 2)"
            );
            return 3;
        }
        println!(
            "ok: the mine gate needs both terms in all four combinations, and the shop choice \
             disappears when gold drops ({choices_rich} -> {choices_poor} visible)"
        );
    }

    // ── 4. A save round-trips, and a v0 save migrates ─────────────────────────────────────────
    {
        let mut app = build_app();
        step(&mut app, 2);
        run_quest_dialogue(&mut app);
        let expected_vars = vars_of(&app);
        if !expected_vars.get_bool(VAR_LANTERN).unwrap_or(false)
            || !expected_vars.get_bool(VAR_KNOWS_MINE).unwrap_or(false)
        {
            eprintln!(
                "FAIL: the scripted conversation did not complete the quest — lantern={:?} \
                 knows_mine={:?}; the save check below would have been vacuous",
                expected_vars.get_bool(VAR_LANTERN),
                expected_vars.get_bool(VAR_KNOWS_MINE)
            );
            return 4;
        }

        if let Err(e) = write_save(&mut app.world) {
            eprintln!("FAIL: could not write the save file: {e}");
            return 4;
        }
        let loaded = match read_save() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("FAIL: could not read the save file back: {e}");
                return 4;
            }
        };
        // `expected` was snapshotted by `write_save`, so compare against the post-snapshot value.
        let expected = quest(&app);
        if loaded.quest != expected || !loaded.quest.has_lantern || !loaded.quest.knows_mine {
            eprintln!(
                "FAIL: the save did not round-trip — quest {:?} vs {expected:?}",
                loaded.quest
            );
            return 4;
        }

        // A v0 save: the same payload minus the field v1 added. Written through the *same* envelope
        // the game writes, so this exercises the real load path rather than a hand-rolled parse.
        #[derive(serde::Serialize)]
        struct QuestV0 {
            gold: i64,
            mine_cleared: bool,
            town_enters: u32,
            has_lantern: bool,
            knows_mine: bool,
        }
        #[derive(serde::Serialize)]
        struct SaveV0 {
            quest: QuestV0,
        }
        let old = SaveV0 {
            quest: QuestV0 {
                gold: 3,
                mine_cleared: true,
                town_enters: 5,
                has_lantern: true,
                knows_mine: true,
            },
        };
        if let Err(e) = save::save_versioned(&save_file(), 0, &old) {
            eprintln!("FAIL: could not write the v0 save fixture: {e}");
            return 4;
        }
        let migrated: SaveData = match read_save() {
            Ok(d) => d,
            Err(e) => {
                eprintln!(
                    "FAIL: a v0 save did not migrate: {e}. The step must insert every field v1 \
                     added, or serde rejects the whole struct."
                );
                return 4;
            }
        };
        if migrated.quest.gold != 3
            || !migrated.quest.mine_cleared
            || !migrated.quest.has_lantern
            || migrated.quest.lantern_name.is_empty()
        {
            eprintln!(
                "FAIL: the migrated v0 save is wrong — {:?} (want gold 3, cleared, a non-empty \
                 lantern_name supplied by the migrator)",
                migrated.quest
            );
            return 4;
        }
        let _ = save::delete(&save_file());
        println!(
            "ok: the save round-trips the quest (flags mirrored out of DialogueVars), and a v0 \
             save migrates (gold {}, lantern_name {:?} filled in by the step)",
            migrated.quest.gold, migrated.quest.lantern_name
        );
    }

    // ── 5. A locale switch retranslates, with no rebuild ──────────────────────────────────────
    //
    // Both the widget labels (`LocalizedText` → `LocalizationSystem`) and the dialogue box (its own
    // `line_keys` → `DialogueSystem`). Asserting only the widgets would miss the box entirely, and
    // the box is where the player actually reads the language.
    {
        let mut app = build_app();
        step(&mut app, 2);
        app.push_scene(Box::new(PauseScene));
        step(&mut app, 2);

        let labels_before = localized_labels(&app);
        let line_before = current_dialogue_line(&app);
        set_locale(&mut app.world, "ko");
        step(&mut app, 2);
        let labels_after = localized_labels(&app);
        let line_after = current_dialogue_line(&app);

        let widgets_changed = labels_before
            .iter()
            .zip(labels_after.iter())
            .filter(|(a, b)| a != b)
            .count();
        if labels_before.is_empty()
            || widgets_changed != labels_before.len()
            || line_before == line_after
            || line_after.is_empty()
        {
            eprintln!(
                "FAIL: switching to ko did not retranslate — {widgets_changed} of \
                 {} widget labels changed, dialogue line {line_before:?} -> {line_after:?}",
                labels_before.len()
            );
            return 5;
        }
        println!(
            "ok: a locale switch retranslates all {} widget labels and the dialogue line \
             ({line_before:?} -> {line_after:?}) with no rebuild",
            labels_before.len()
        );
    }

    // ── 6. A data-table edit reaches the running game ─────────────────────────────────────────
    //
    // The only wall-clock check in this file, and it has to be: the file watcher runs on `notify`,
    // so the loop is paced off `Instant` rather than counting frames. `docs/CLAUDE.md`'s capture
    // warning is about exactly this shape — a fixed-dt loop running as fast as the CPU allows can
    // finish "10 seconds" of frames before the OS has delivered a single event.
    //
    // It edits a **temp file**, not `assets/items.ron`: a check that rewrites a tracked asset
    // leaves the working tree dirty when it fails, and a dirty tree is how a bad asset gets
    // committed by accident.
    {
        use std::time::{Duration, Instant};

        let dir = std::env::temp_dir();
        let path = dir.join("skeleton_rpg_quest_hot_reload.ron");
        let write_price = |price: i64| -> std::io::Result<()> {
            std::fs::write(&path, format!("[(id: \"lantern\", price: {price})]\n"))
        };
        if let Err(e) = write_price(10) {
            eprintln!("FAIL: could not write the hot-reload fixture at {path:?}: {e}");
            return 6;
        }

        let mut app = build_app();
        app.load_data_table("hot", path.to_string_lossy().as_ref());
        step(&mut app, 2);

        let price_of = |app: &App| -> Option<i64> {
            app.world
                .resource::<DataTableRegistry>()
                .and_then(|r| r.get("hot"))
                .and_then(|t| t.get(0, "price"))
                .and_then(|v| match v {
                    ron::Value::Number(n) => n.as_i64(),
                    _ => None,
                })
        };

        // Control: the table has to be loaded and readable *before* the edit, or "the value
        // changed" would be indistinguishable from "the value appeared".
        let before = price_of(&app);
        if before != Some(10) {
            eprintln!(
                "FAIL: the data table did not load at all — price {before:?} (want 10). The \
                 reload check below would have proved nothing."
            );
            let _ = std::fs::remove_file(&path);
            return 6;
        }

        if let Err(e) = write_price(42) {
            eprintln!("FAIL: could not rewrite the hot-reload fixture: {e}");
            let _ = std::fs::remove_file(&path);
            return 6;
        }

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut seen = before;
        while Instant::now() < deadline {
            app.step_headless(DT);
            seen = price_of(&app);
            if seen == Some(42) {
                break;
            }
            std::thread::sleep(Duration::from_millis(8));
        }
        let _ = std::fs::remove_file(&path);

        if seen != Some(42) {
            eprintln!(
                "FAIL: a data-table edit never reached the running game — price still {seen:?} \
                 after 10 s of wall clock (want 42). The watcher is registered by \
                 `App::load_data_table`; a frame-counted loop would also fail here, so check the \
                 pacing before the watcher."
            );
            return 6;
        }
        println!("ok: a data-table edit on disk reaches the running game (price 10 -> 42)");
    }

    // ── 7. The guard's behaviour tree walks an A* route that goes AROUND the rock ──────────────
    //
    // Two failures this separates, neither of which a still frame shows. A dead tree leaves the
    // guard at his spawn, which reads as "the mine is quiet". And a pathfinder that returned the
    // straight line would walk him *through* a pillar — visible only if you already know where the
    // pillars are.
    //
    // ⚠️ The control comes first: the straight line between these two waypoints must actually hit
    // rock. The first version of this check ran between waypoints an unobstructed monotone path
    // could join, so "the route is walkable" was true of the naive answer too and the check proved
    // nothing. The map now forces the detour, and this asserts that it does.
    {
        let mut app = build_app();
        app.set_scene(Box::new(MineScene));
        step(&mut app, 2);

        let grid = app
            .world
            .resource::<PathGrid>()
            .expect("the mine has a path grid");
        let (from, to) = (GUARD_WAYPOINTS[0], GUARD_WAYPOINTS[1]);

        // Control: sample the straight segment. If it never touches a blocked cell, this map does
        // not test pathfinding at all.
        let steps = (to - from).abs().max_element().max(1);
        let straight_blocked = (0..=steps).any(|i| {
            let t = i as f32 / steps as f32;
            let x = (from.x as f32 + (to.x - from.x) as f32 * t).round() as i32;
            let y = (from.y as f32 + (to.y - from.y) as f32 * t).round() as i32;
            !grid.is_walkable(x, y)
        });
        let route = engine::find_path(grid, from, to);
        let manhattan = ((to.x - from.x).abs() + (to.y - from.y).abs()) as usize;
        let route_cells = route.as_ref().map(|r| r.len()).unwrap_or(0);
        let all_walkable = route
            .as_ref()
            .map(|r| r.iter().all(|c| grid.is_walkable(c.x, c.y)))
            .unwrap_or(false);

        if !straight_blocked {
            eprintln!(
                "FAIL: the straight line from {from:?} to {to:?} never touches a pillar, so this \
                 check cannot tell A* from a naive interpolation. Move a pillar or a waypoint."
            );
            return 7;
        }
        if !all_walkable || route_cells <= manhattan {
            eprintln!(
                "FAIL: the A* route is not a detour — {route_cells} cells against a {manhattan}-step \
                 manhattan distance (want more, since the straight line is blocked), every cell \
                 walkable: {all_walkable}"
            );
            return 7;
        }

        // …and the tree actually walks it. Long enough for at least one leg: the legs are ~10-14
        // cells at 120 px/s over 48 px cells.
        let guard = app
            .world
            .resource::<Stage>()
            .and_then(|s| s.guard)
            .expect("the mine has a guard");
        let start = app
            .world
            .get::<Transform>(guard)
            .map(|t| t.position)
            .unwrap_or_default();
        step(&mut app, 500);
        let legs = app
            .world
            .get::<GuardBrain>(guard)
            .map(|b| b.legs)
            .unwrap_or(0);
        let now = app
            .world
            .get::<Transform>(guard)
            .map(|t| t.position)
            .unwrap_or_default();
        let standing_on_rock = app
            .world
            .resource::<PathGrid>()
            .map(|g| {
                let c = mine_cell_at(now);
                !g.is_walkable(c.x, c.y)
            })
            .unwrap_or(true);
        if legs == 0 || now.distance(start) < 100.0 || standing_on_rock {
            eprintln!(
                "FAIL: the guard did not patrol — {legs} legs completed (want at least 1), moved \
                 {:.0} px, standing inside rock: {standing_on_rock}",
                now.distance(start)
            );
            return 7;
        }
        println!(
            "ok: the guard's tree walks an A* detour — {route_cells} cells where the manhattan \
             distance is {manhattan} and the straight line is blocked, {legs} patrol leg(s) in 500 \
             frames, {:.0} px travelled",
            now.distance(start)
        );
    }

    0
}

/// Every resolved  label currently in the world, keyed for a stable order.
fn localized_labels(app: &App) -> Vec<String> {
    // Read the *target* field, not the key: `LocalizedText` holds only the key, and the whole
    // question is whether the system wrote the translation into the widget.
    let mut labels: Vec<(String, String)> = app
        .world
        .query::<LocalizedText>()
        .filter_map(|(e, lt)| {
            let text = app
                .world
                .get::<engine::Label>(e)
                .map(|l| l.text.clone())
                .or_else(|| app.world.get::<CheckBox>(e).map(|c| c.label.clone()))
                .or_else(|| app.world.get::<TextInput>(e).map(|t| t.placeholder.clone()))?;
            Some((lt.key.clone(), text))
        })
        .collect();
    labels.sort_by(|a, b| a.0.cmp(&b.0));
    labels.into_iter().map(|(_, t)| t).collect()
}

fn current_dialogue_line(app: &App) -> String {
    app.world
        .resource::<Stage>()
        .and_then(|s| s.dialogue)
        .and_then(|d| app.world.get::<DialogueBox>(d))
        .and_then(|d| d.current_line())
        .unwrap_or_default()
        .to_string()
}
