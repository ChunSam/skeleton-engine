//! dialogue_branching — localized, branching dialogue with `DialogueBox`.
//!
//! Extends `dialogue_demo` with the two features that turn the linear typewriter into a real
//! RPG/visual-novel conversation:
//!   1. **Localization** — the box is built from translation *keys* (`DialogueBox::localized`),
//!      and `DialogueSystem` resolves them against the `LocaleResource` every frame. Press `L`
//!      to flip the locale between English and Korean *live* — the speaker, lines, and choices
//!      all retranslate on the next frame, mid-conversation, without losing your place.
//!   2. **Branching choices** — a line can present numbered choices (`with_choices`); selecting
//!      one jumps the conversation to that choice's target line (`goto`). The merchant scene
//!      branches on "buy a lantern" vs "travel in the dark" and the two paths reconverge on a
//!      shared farewell — proof that `choose` can both split and merge a conversation.
//!
//! Run from the repo root:  `cargo run --example dialogue_branching`
//! SPACE = advance · 1/2 = pick a choice · L = toggle language · R = replay · ESC = quit.

use engine::{
    App, Color, DialogueBox, DialogueChoice, DialogueSystem, DrawText, Entity, InputState, KeyCode,
    LocaleResource, ShouldQuit, System, TextQueue, Vec2, ViewportSize, WindowConfig, World,
};

/// English + Korean translation bundle. Korean here is fixture data (allowed — see the engine's
/// i18n examples); the point is to show a live locale switch driving every dialogue field.
const LOCALES_RON: &str = r#"
(
    default_locale: "en",
    locales: {
        "en": ( translations: {
            "npc.merchant":   "Old Merchant",
            "dlg.intro":      "Welcome, traveler. Night falls fast on this road.",
            "dlg.ask":        "Will you take a lantern, or trust the dark?",
            "dlg.buy":        "Wise. Its warm glow keeps the old shadows at bay.",
            "dlg.dark":       "Brave — or foolish. The dark remembers either way.",
            "dlg.bye":        "Either way... safe travels, friend.",
            "choice.buy":     "Buy the lantern (5 coin)",
            "choice.dark":    "Travel in the dark",
            "choice.go":      "Step into the night",
        } ),
        "ko": ( translations: {
            "npc.merchant":   "노상인",
            "dlg.intro":      "어서 오게, 여행자. 이 길엔 밤이 빨리 찾아오지.",
            "dlg.ask":        "등불을 사겠나, 아니면 어둠을 믿겠나?",
            "dlg.buy":        "현명하군. 그 따뜻한 빛이 오랜 그림자를 막아줄 걸세.",
            "dlg.dark":       "용감하군 — 아니면 어리석거나. 어둠은 어느 쪽이든 기억하지.",
            "dlg.bye":        "어느 쪽이든... 부디 무사히 가게, 친구.",
            "choice.buy":     "등불을 산다 (5코인)",
            "choice.dark":    "어둠 속을 간다",
            "choice.go":      "밤 속으로 발을 내딛는다",
        } ),
    },
)
"#;

/// Builds the branching merchant conversation. Line indices:
///   0 intro → 1 ask → {2 buy | 3 dark} → 4 farewell.
/// Line 1 branches (buy→2, dark→3); lines 2 and 3 each reconverge on line 4 via a "go" choice,
/// so whichever branch you take, only its own payoff line is shown before the shared farewell.
fn merchant_box() -> DialogueBox {
    DialogueBox::localized(
        "npc.merchant",
        ["dlg.intro", "dlg.ask", "dlg.buy", "dlg.dark", "dlg.bye"],
    )
    .with_chars_per_sec(34.0)
    .with_choices(
        1,
        [
            DialogueChoice::localized("choice.buy", 2),
            DialogueChoice::localized("choice.dark", 3),
        ],
    )
    .with_choices(2, [DialogueChoice::localized("choice.go", 4)])
    .with_choices(3, [DialogueChoice::localized("choice.go", 4)])
}

/// Feeds input into the box (engine `DialogueSystem` does the resolve/tick/render) and draws a
/// small HUD. Input-agnostic engine + game-owned input is the intended split.
struct DialogueDriver {
    box_entity: Entity,
}

impl System for DialogueDriver {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (advance, pick0, pick1, toggle_lang, replay, quit) =
            match world.resource::<InputState>() {
                Some(i) => (
                    i.just_pressed(KeyCode::Space),
                    i.just_pressed(KeyCode::Digit1),
                    i.just_pressed(KeyCode::Digit2),
                    i.just_pressed(KeyCode::KeyL),
                    i.just_pressed(KeyCode::KeyR),
                    i.just_pressed(KeyCode::Escape),
                ),
                None => (false, false, false, false, false, false),
            };

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // Live locale toggle — DialogueSystem re-resolves every box next frame.
        if toggle_lang {
            if let Some(locale) = world.resource_mut::<LocaleResource>() {
                let next = if locale.current_locale() == "en" {
                    "ko"
                } else {
                    "en"
                };
                locale.set_locale(next);
            }
        }

        // Drive the conversation. `choose`/`advance` are each a safe no-op when they don't apply
        // (a number key with no choices pending, SPACE while a decision is pending, etc.).
        if let Some(d) = world.get_mut::<DialogueBox>(self.box_entity) {
            if replay {
                d.reset();
            } else if pick0 {
                d.choose(0);
            } else if pick1 {
                d.choose(1);
            } else if advance {
                d.advance();
            }
        }

        // HUD: controls + current language + an end note once finished.
        let lang = world
            .resource::<LocaleResource>()
            .map(|l| l.current_locale().to_string())
            .unwrap_or_default();
        let finished = world
            .get::<DialogueBox>(self.box_entity)
            .map(|d| d.is_finished())
            .unwrap_or(true);
        let vh = world
            .resource::<ViewportSize>()
            .map(|v| v.height)
            .unwrap_or(600.0);
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!("SPACE advance   1/2 choose   L language [{lang}]   R replay   ESC quit"),
                Vec2::new(40.0, 24.0),
                18.0,
                Color::rgb(0.6, 0.7, 0.85),
            ));
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

    fn name(&self) -> &'static str {
        "dialogue_driver"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "dialogue_branching — localized + branching DialogueBox".to_string(),
        width: 960,
        height: 600,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });
    app.world
        .insert_resource(LocaleResource::from_ron_str(LOCALES_RON).expect("valid locale bundle"));

    let box_entity = app.world.spawn();
    app.world.add_component(box_entity, merchant_box());

    app.add_system(DialogueSystem);
    app.add_system(DialogueDriver { box_entity });
    app.run();
}
