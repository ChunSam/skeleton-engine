//! dialogue_quest — data-driven branching dialogue: RON tree + conditional choices + event hooks.
//!
//! The capstone for the data-driven dialogue feature. Where `dialogue_branching` builds its
//! conversation in code, this one **loads it from a RON file** (`dialogue_quest.dlg.ron`) via
//! [`App::load_dialogue`] and exercises the two things a quest needs beyond plain branching:
//!
//!   1. **Choice → event hook** — the "buy the lantern" choice carries an `EmitEvent` effect.
//!      Taking it sends a [`DialogueEvent`] the game reads to *grant the lantern* (set the
//!      `has_lantern` dialogue variable + light the HUD marker). The engine stays
//!      input/consequence-agnostic; the game decides what an event means.
//!   2. **Conditional choice** — at the second question the "ask about the secret" choice is
//!      gated on `has_lantern == true` ([`DialogueCond`]). It only appears once you've bought
//!      the lantern, so the conversation reacts to earlier decisions.
//!
//! Plus the `dialogue_branching` staples: localized lines/choices (`L` flips EN↔KO live) and a
//! reconverging branch structure. Edit `dialogue_quest.dlg.ron` while it runs to hot-reload.
//!
//! Run from the repo root:  `cargo run --example dialogue_quest`
//! SPACE = advance · 1/2 = pick a choice · L = toggle language · R = replay · ESC = quit.

use engine::{
    dialogue, App, Color, DialogueBox, DialogueEvent, DialogueRegistry, DialogueSystem, DialogueVars,
    DrawText, Entity, Events, InputState, KeyCode, LocaleResource, ShouldQuit, System, TextQueue,
    Vec2, ViewportSize, WindowConfig, World,
};

const DLG_PATH: &str = "examples/dialogue_quest.dlg.ron";

/// English + Korean bundle. Korean is fixture data (the point is the live locale switch driving
/// every dialogue field). Keys match `dialogue_quest.dlg.ron`.
const LOCALES_RON: &str = r#"
(
    default_locale: "en",
    locales: {
        "en": ( translations: {
            "npc.merchant":  "Old Merchant",
            "dlg.intro":     "Welcome, traveler. The eastern road runs dark past dusk.",
            "dlg.ask":       "Care for a lantern before you go?",
            "dlg.buy":       "Here — a steady flame. Mind the wind.",
            "dlg.ask2":      "Anything else I can do for you?",
            "dlg.secret":    "With that light, seek the old mine east of here. Few dare it.",
            "dlg.bye":       "Safe travels, friend.",
            "choice.buy":    "Buy the lantern",
            "choice.leave":  "No thanks, I'll be off",
            "choice.more":   "Actually, one more thing...",
            "choice.secret": "Know any secrets worth the dark?",
            "choice.go":     "I'll head out, then",
        } ),
        "ko": ( translations: {
            "npc.merchant":  "노상인",
            "dlg.intro":     "어서 오게, 여행자. 동쪽 길은 해가 지면 캄캄하다네.",
            "dlg.ask":       "떠나기 전에 등불 하나 챙기겠나?",
            "dlg.buy":       "여기 — 꺼지지 않는 불꽃일세. 바람을 조심하게.",
            "dlg.ask2":      "더 필요한 게 있나?",
            "dlg.secret":    "그 빛이 있다면, 동쪽의 옛 광산을 찾아보게. 감히 가는 자가 드물지.",
            "dlg.bye":       "부디 무사히 가게, 친구.",
            "choice.buy":    "등불을 산다",
            "choice.leave":  "괜찮네, 이만 가보지",
            "choice.more":   "사실, 하나 더 있는데...",
            "choice.secret": "어둠 속에서 쓸만한 비밀이라도 아나?",
            "choice.go":     "그럼 이만 나서겠네",
        } ),
    },
)
"#;

/// Reacts to dialogue [`DialogueEvent`]s: granting the lantern sets the `has_lantern` variable,
/// which unlocks the gated "secret" choice. This is the game-side meaning of a choice effect.
struct QuestEvents;

impl System for QuestEvents {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // Collect event names first (immutable borrow), then mutate the vars resource.
        let granted: Vec<String> = world
            .resource::<Events<DialogueEvent>>()
            .map(|ev| ev.read().iter().map(|e| e.name.clone()).collect())
            .unwrap_or_default();
        for name in granted {
            if name == "grant_lantern" {
                if world.resource::<DialogueVars>().is_none() {
                    world.insert_resource(DialogueVars::default());
                }
                if let Some(v) = world.resource_mut::<DialogueVars>() {
                    v.set_bool("has_lantern", true);
                }
            }
        }
    }
}

/// Feeds input to the conversation via the world-level `dialogue::advance` / `dialogue::choose`
/// helpers (so conditional gating + effects are honored) and draws a small HUD.
struct QuestInput {
    box_entity: Entity,
}

impl System for QuestInput {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (advance, p0, p1, p2, toggle_lang, replay, quit) = match world.resource::<InputState>() {
            Some(i) => (
                i.just_pressed(KeyCode::Space),
                i.just_pressed(KeyCode::Digit1),
                i.just_pressed(KeyCode::Digit2),
                i.just_pressed(KeyCode::Digit3),
                i.just_pressed(KeyCode::KeyL),
                i.just_pressed(KeyCode::KeyR),
                i.just_pressed(KeyCode::Escape),
            ),
            None => (false, false, false, false, false, false, false),
        };

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        if toggle_lang {
            if let Some(locale) = world.resource_mut::<LocaleResource>() {
                let next = if locale.current_locale() == "en" { "ko" } else { "en" };
                locale.set_locale(next);
            }
        }

        if replay {
            // Restart the conversation and clear the quest state.
            if let Some(d) = world.get_mut::<DialogueBox>(self.box_entity) {
                d.reset();
            }
            if let Some(v) = world.resource_mut::<DialogueVars>() {
                v.clear();
            }
        } else if p0 {
            dialogue::choose(world, self.box_entity, 0);
        } else if p1 {
            dialogue::choose(world, self.box_entity, 1);
        } else if p2 {
            dialogue::choose(world, self.box_entity, 2);
        } else if advance {
            dialogue::advance(world, self.box_entity);
        }

        self.draw_hud(world);
    }

    fn name(&self) -> &'static str {
        "quest_input"
    }
}

impl QuestInput {
    fn draw_hud(&self, world: &mut World) {
        let lang = world
            .resource::<LocaleResource>()
            .map(|l| l.current_locale().to_string())
            .unwrap_or_default();
        let has_lantern = world
            .resource::<DialogueVars>()
            .and_then(|v| v.get_bool("has_lantern"))
            .unwrap_or(false);
        let finished = world
            .get::<DialogueBox>(self.box_entity)
            .map(|d| d.is_finished())
            .unwrap_or(true);
        let vh = world.resource::<ViewportSize>().map(|v| v.height).unwrap_or(620.0);

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!("SPACE advance   1/2/3 choose   L language [{lang}]   R replay   ESC quit"),
                Vec2::new(40.0, 24.0),
                17.0,
                Color::rgb(0.6, 0.7, 0.85),
            ));
            // Quest marker — lit once the lantern is granted by the choice effect.
            let (marker, color) = if has_lantern {
                ("[*] Lantern acquired", Color::rgb(1.0, 0.82, 0.35))
            } else {
                ("[ ] No lantern", Color::rgb(0.5, 0.52, 0.58))
            };
            tq.push(DrawText::new(marker, Vec2::new(40.0, 50.0), 17.0, color));
            if finished {
                tq.push(DrawText::new(
                    "[ conversation over — R to replay ]",
                    Vec2::new(40.0, vh - 118.0),
                    20.0,
                    Color::rgb(0.7, 0.7, 0.75),
                ));
            }
        }
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "dialogue_quest — RON tree + conditional choices + event hooks".to_string(),
        width: 960,
        height: 620,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });
    app.world
        .insert_resource(LocaleResource::from_ron_str(LOCALES_RON).expect("valid locale bundle"));
    app.world.insert_resource(DialogueVars::default());
    app.register_event::<DialogueEvent>();

    // Load the quest conversation from RON and spawn a box for it.
    app.load_dialogue("quest", DLG_PATH);
    let quest_box = app
        .world
        .resource::<DialogueRegistry>()
        .and_then(|r| r.box_of("quest"))
        .unwrap_or_else(|| panic!("failed to load dialogue tree from {DLG_PATH} (run from repo root)"));
    let box_entity = app.world.spawn();
    app.world.add_component(box_entity, quest_box);

    // Order: input (may emit an event) → event handler (grants lantern) → dialogue tick/render.
    app.add_system(QuestInput { box_entity });
    app.add_system(QuestEvents);
    app.add_system(DialogueSystem);
    app.run();
}
