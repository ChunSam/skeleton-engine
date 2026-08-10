use super::*;

// `LocaleResource` was previously in scope via `super`'s glob (mod.rs's private
// `use crate::locale::LocaleResource;`). After the box/style/system split that import
// lives in the submodules, so name it directly here. (World/Events/TextQueue/ViewportSize
// are already imported locally inside the tests that use them.)
use crate::locale::LocaleResource;

#[test]
fn typewriter_reveals_over_time() {
    let mut d = DialogueBox::new("", ["hello"]).with_chars_per_sec(10.0);
    assert_eq!(d.visible_text(), "");
    d.tick(0.25); // 2.5 chars
    assert_eq!(d.visible_text(), "he");
    d.tick(0.30); // 5.5 chars → full
    assert_eq!(d.visible_text(), "hello");
    assert!(d.line_fully_revealed());
}

#[test]
fn instant_when_cps_zero() {
    let d = DialogueBox::new("", ["instant"]).with_chars_per_sec(0.0);
    assert_eq!(d.visible_text(), "instant");
    assert!(d.line_fully_revealed());
}

#[test]
fn advance_completes_then_moves_to_next_line() {
    let mut d = DialogueBox::new("NPC", ["one", "two"]).with_chars_per_sec(10.0);
    d.tick(0.1); // mid-reveal of "one"
    assert!(!d.line_fully_revealed());
    d.advance(); // first press → complete the line
    assert_eq!(d.visible_text(), "one");
    assert_eq!(d.current, 0);
    d.advance(); // second press → next line
    assert_eq!(d.current, 1);
    assert_eq!(d.visible_text(), "");
}

#[test]
fn finishes_after_last_line() {
    let mut d = DialogueBox::new("", ["only"]).with_chars_per_sec(0.0);
    assert!(!d.is_finished());
    d.advance(); // line is already full (cps 0) → advances past the end
    assert!(d.is_finished());
    assert_eq!(d.visible_text(), "");
    d.advance(); // no-op when finished
    assert!(d.is_finished());
}

#[test]
fn utf8_safe_reveal() {
    let mut d = DialogueBox::new("", ["héllo"]).with_chars_per_sec(10.0);
    d.tick(0.25); // 2 chars: "hé" (must not split the multibyte 'é')
    assert_eq!(d.visible_text(), "hé");
}

// --- Phase 1: localization -------------------------------------------------------------

const LOCALE_SAMPLE: &str = r#"
(
    default_locale: "en",
    locales: {
        "en": ( translations: {
            "npc.guide": "Guide",
            "intro.welcome": "Welcome, traveler.",
            "intro.continue": "Press space to continue.",
            "choice.left": "Go left",
            "choice.right": "Go right",
        } ),
        "ko": ( translations: {
            "npc.guide": "안내자",
            "intro.welcome": "어서 오세요, 여행자여.",
            "intro.continue": "계속하려면 스페이스를 누르세요.",
            "choice.left": "왼쪽으로",
            "choice.right": "오른쪽으로",
        } ),
    },
)
"#;

#[test]
fn localized_box_resolves_then_reresolves_on_locale_switch() {
    let mut locale = LocaleResource::from_ron_str(LOCALE_SAMPLE).unwrap();
    let mut d = DialogueBox::localized("npc.guide", ["intro.welcome", "intro.continue"]);
    // Keys are stored separately; display text starts empty until the first resolve.
    assert!(d.lines.is_empty());
    assert!(d.speaker.is_empty());

    d.resolve(&locale);
    assert_eq!(d.speaker, "Guide");
    assert_eq!(
        d.lines,
        vec!["Welcome, traveler.", "Press space to continue."]
    );

    assert!(locale.set_locale("ko"));
    d.resolve(&locale);
    assert_eq!(d.speaker, "안내자");
    assert_eq!(d.lines[0], "어서 오세요, 여행자여.");
}

#[test]
fn resolve_is_noop_for_literal_box() {
    let locale = LocaleResource::from_ron_str(LOCALE_SAMPLE).unwrap();
    let mut d = DialogueBox::new("NPC", ["literal one", "literal two"]);
    d.resolve(&locale); // no line_keys → literal mode, unchanged
    assert_eq!(d.speaker, "NPC");
    assert_eq!(d.lines, vec!["literal one", "literal two"]);
}

#[test]
fn resolve_preserves_typewriter_progress() {
    let locale = LocaleResource::from_ron_str(LOCALE_SAMPLE).unwrap();
    let mut d = DialogueBox::localized("npc.guide", ["intro.welcome", "intro.continue"])
        .with_chars_per_sec(10.0);
    d.resolve(&locale);
    d.tick(0.3); // 3 chars into "Welcome, traveler."
    assert_eq!(d.visible_text(), "Wel");
    assert!(!d.line_fully_revealed());

    d.resolve(&locale); // re-resolve must NOT touch current/elapsed/full
    assert_eq!(d.current, 0);
    assert_eq!(
        d.visible_text(),
        "Wel",
        "resolve must preserve reveal progress"
    );
    assert!(!d.line_fully_revealed());
}

#[test]
fn serde_roundtrip_and_legacy_back_compat() {
    // A localized box round-trips its keys.
    let d = DialogueBox::localized("npc.guide", ["a.key", "b.key"]);
    let ron = ron::to_string(&d).unwrap();
    let back: DialogueBox = ron::from_str(&ron).unwrap();
    assert_eq!(back.line_keys, vec!["a.key", "b.key"]);
    assert_eq!(back.speaker_key.as_deref(), Some("npc.guide"));

    // A pre-localization scene (no line_keys/speaker_key) still loads via serde defaults.
    let legacy = r#"(speaker:"NPC",lines:["one","two"],current:0,chars_per_sec:28.0,elapsed:0.0,full:false)"#;
    let d2: DialogueBox = ron::from_str(legacy).unwrap();
    assert_eq!(d2.lines, vec!["one", "two"]);
    assert!(d2.line_keys.is_empty());
    assert_eq!(d2.speaker_key, None);
    assert!(d2.choices.is_empty());
}

// --- Phase 2: branching choices --------------------------------------------------------

#[test]
fn advance_is_blocked_while_choices_pending_then_choose_branches() {
    let mut d = DialogueBox::new("NPC", ["Pick a door.", "Left room.", "Right room."])
        .with_chars_per_sec(0.0) // instant reveal
        .with_choices(
            0,
            [
                DialogueChoice::new("Left", 1),
                DialogueChoice::new("Right", 2),
            ],
        );
    // Line 0 is instantly revealed and has choices → a decision is pending.
    assert_eq!(d.pending_choices().map(|v| v.len()), Some(2));

    // A plain advance must NOT skip the decision.
    d.advance();
    assert_eq!(d.current, 0, "advance is a no-op while choices are pending");

    // choose(0) jumps to the first choice's goto.
    d.choose(0);
    assert_eq!(d.current, 1);
    assert!(d.pending_choices().is_none());
}

#[test]
fn choose_lands_on_distinct_targets() {
    let make = || {
        DialogueBox::new("NPC", ["Q", "A", "B"])
            .with_chars_per_sec(0.0)
            .with_choices(
                0,
                [DialogueChoice::new("a", 1), DialogueChoice::new("b", 2)],
            )
    };
    let mut left = make();
    left.choose(0);
    assert_eq!(left.current, 1);

    let mut right = make();
    right.choose(1);
    assert_eq!(right.current, 2);
}

#[test]
fn out_of_range_goto_finishes_safely() {
    let mut d = DialogueBox::new("NPC", ["only"])
        .with_chars_per_sec(0.0)
        .with_choices(0, [DialogueChoice::new("leave", 99)]);
    d.choose(0); // goto 99 clamps to lines.len() (==1) → finished, no panic
    assert!(d.is_finished());
    assert_eq!(d.current, 1);
}

#[test]
fn out_of_range_choice_index_is_noop() {
    let mut d = DialogueBox::new("NPC", ["Q", "A"])
        .with_chars_per_sec(0.0)
        .with_choices(0, [DialogueChoice::new("a", 1)]);
    d.choose(5); // no such choice → no-op
    assert_eq!(d.current, 0);
    assert!(d.pending_choices().is_some());
}

#[test]
fn first_advance_completes_reveal_then_choices_appear() {
    let mut d = DialogueBox::new("NPC", ["Pick", "X", "Y"])
        .with_chars_per_sec(10.0)
        .with_choices(
            0,
            [DialogueChoice::new("x", 1), DialogueChoice::new("y", 2)],
        );
    d.tick(0.1); // mid-reveal → not fully shown yet, so no choices pending
    assert!(d.pending_choices().is_none());
    d.advance(); // first press completes the reveal
    assert!(d.line_fully_revealed());
    assert!(
        d.pending_choices().is_some(),
        "choices appear once the line is fully revealed"
    );
    d.advance(); // now a decision is pending → no-op
    assert_eq!(d.current, 0);
}

#[test]
fn empty_choice_list_for_a_line_is_not_pending() {
    let mut d = DialogueBox::new("NPC", ["line"])
        .with_chars_per_sec(0.0)
        .with_choices(0, []); // explicitly empty → treated as no choices
    assert!(d.pending_choices().is_none());
    d.advance(); // behaves like a normal linear line
    assert!(d.is_finished());
}

#[test]
fn localized_choices_resolve_text() {
    let mut locale = LocaleResource::from_ron_str(LOCALE_SAMPLE).unwrap();
    let mut d = DialogueBox::localized("npc.guide", ["intro.welcome"])
        .with_chars_per_sec(0.0)
        .with_choices(
            0,
            [
                DialogueChoice::localized("choice.left", 0),
                DialogueChoice::localized("choice.right", 0),
            ],
        );
    d.resolve(&locale);
    let cs = d.pending_choices().expect("choices pending after resolve");
    assert_eq!(cs[0].text, "Go left");
    assert_eq!(cs[1].text, "Go right");

    assert!(locale.set_locale("ko"));
    d.resolve(&locale);
    let cs = d.pending_choices().unwrap();
    assert_eq!(cs[0].text, "왼쪽으로");
    assert_eq!(cs[1].text, "오른쪽으로");
}

#[test]
fn serde_roundtrip_with_choices() {
    let d = DialogueBox::new("NPC", ["Q", "A", "B"]).with_choices(
        0,
        [
            DialogueChoice::new("a", 1),
            DialogueChoice::localized("k", 2),
        ],
    );
    let ron = ron::to_string(&d).unwrap();
    let back: DialogueBox = ron::from_str(&ron).unwrap();
    assert_eq!(back.choices.len(), 1);
    assert_eq!(back.choices[0].0, 0);
    assert_eq!(back.choices[0].1[0], DialogueChoice::new("a", 1));
    assert_eq!(back.choices[0].1[1].goto, 2);
    assert_eq!(back.choices[0].1[1].key.as_deref(), Some("k"));
}

// --- P1 1b/1c: conditional choices + effect hooks --------------------------------------

use crate::ecs::{Events, World};

#[test]
fn conditional_choice_hidden_until_var_set() {
    let mut d = DialogueBox::new("NPC", ["Pick", "secret", "normal"])
        .with_chars_per_sec(0.0)
        .with_choices(
            0,
            [
                DialogueChoice::new("secret", 1).when(DialogueCond::new(
                    "vip",
                    DialogueOp::Eq,
                    DialogueValue::Bool(true),
                )),
                DialogueChoice::new("normal", 2),
            ],
        );
    let mut vars = DialogueVars::new();
    // vip unset → only the ungated choice is visible.
    assert_eq!(d.visible_choices(&vars).len(), 1);
    assert_eq!(d.visible_choices(&vars)[0].text, "normal");

    vars.set_bool("vip", true);
    assert_eq!(d.visible_choices(&vars).len(), 2);
    // visible index 0 is now the secret branch → jumps to line 1.
    assert!(d.choose_visible(0, &vars).is_none());
    assert_eq!(d.current, 1);
}

#[test]
fn advance_with_skips_line_whose_choices_are_all_gated_out() {
    let mut d = DialogueBox::new("NPC", ["gate", "after"])
        .with_chars_per_sec(0.0)
        .with_choices(
            0,
            [DialogueChoice::new("locked", 1).when(DialogueCond::new(
                "key",
                DialogueOp::Eq,
                DialogueValue::Bool(true),
            ))],
        );
    let vars = DialogueVars::new(); // key unset → the only choice is gated out
    assert!(!d.is_choosing(&vars));
    d.advance_with(&vars); // line is full (cps 0) → advances past it like a normal line
    assert_eq!(d.current, 1);
}

#[test]
fn choose_visible_returns_effect() {
    let mut d = DialogueBox::new("NPC", ["q", "a"])
        .with_chars_per_sec(0.0)
        .with_choices(
            0,
            [DialogueChoice::new("take", 1).then(DialogueEffect::SetVar {
                key: "got".into(),
                value: DialogueValue::Bool(true),
            })],
        );
    let vars = DialogueVars::new();
    let effect = d.choose_visible(0, &vars).expect("effect returned");
    assert_eq!(
        effect,
        DialogueEffect::SetVar {
            key: "got".into(),
            value: DialogueValue::Bool(true)
        }
    );
    assert_eq!(d.current, 1);
}

#[test]
fn world_choose_applies_setvar() {
    let mut world = World::new();
    world.insert_resource(Events::<DialogueEvent>::default());
    let e = world.spawn();
    world.add_component(
        e,
        DialogueBox::new("NPC", ["q", "buy", "leave"])
            .with_chars_per_sec(0.0)
            .with_choices(
                0,
                [DialogueChoice::new("buy", 1).then(DialogueEffect::SetVar {
                    key: "bought".into(),
                    value: DialogueValue::Bool(true),
                })],
            ),
    );
    super::choose(&mut world, e, 0);
    assert_eq!(world.get::<DialogueBox>(e).unwrap().current, 1);
    assert_eq!(
        world.resource::<DialogueVars>().unwrap().get_bool("bought"),
        Some(true)
    );
}

#[test]
fn world_choose_emits_event() {
    let mut world = World::new();
    world.insert_resource(Events::<DialogueEvent>::default());
    let e = world.spawn();
    world.add_component(
        e,
        DialogueBox::new("NPC", ["q", "x"])
            .with_chars_per_sec(0.0)
            .with_choices(
                0,
                [
                    DialogueChoice::new("go", 1).then(DialogueEffect::EmitEvent {
                        name: "door".into(),
                    }),
                ],
            ),
    );
    super::choose(&mut world, e, 0);
    let evs = world.resource::<Events<DialogueEvent>>().unwrap().read();
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].name, "door");
    assert_eq!(evs[0].entity, e);
}

#[test]
fn world_advance_blocks_on_visible_choice() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(
        e,
        DialogueBox::new("NPC", ["pick", "a", "b"])
            .with_chars_per_sec(0.0)
            .with_choices(
                0,
                [DialogueChoice::new("a", 1), DialogueChoice::new("b", 2)],
            ),
    );
    super::advance(&mut world, e); // a decision is pending → no-op
    assert_eq!(world.get::<DialogueBox>(e).unwrap().current, 0);
}

// --- Bug fixes: plain API + NaN chars_per_sec ------------------------------------------

/// A line whose ONLY choices are conditional must not deadlock the plain `advance()`.
/// With no unconditional choices, `pending_choices()` returns `None` and `advance()` moves
/// past the line normally.
#[test]
fn advance_not_deadlocked_when_all_choices_are_conditional() {
    let mut d = DialogueBox::new("NPC", ["gate", "after"])
        .with_chars_per_sec(0.0)
        .with_choices(
            0,
            [DialogueChoice::new("locked", 1).when(DialogueCond::new(
                "key",
                DialogueOp::Eq,
                DialogueValue::Bool(true),
            ))],
        );
    // All choices conditional → plain pending_choices() is None → advance() is not blocked.
    assert!(d.pending_choices().is_none());
    d.advance();
    assert_eq!(
        d.current, 1,
        "advance must move past a conditional-only choice line"
    );
}

/// A line with a mix of conditional and unconditional choices: the plain API exposes only
/// the unconditional subset, while `visible_choices` (with vars) exposes both when the cond
/// passes.
#[test]
fn pending_choices_exposes_only_unconditional_subset() {
    let mut d = DialogueBox::new("NPC", ["pick", "secret", "normal"])
        .with_chars_per_sec(0.0)
        .with_choices(
            0,
            [
                DialogueChoice::new("secret", 1).when(DialogueCond::new(
                    "vip",
                    DialogueOp::Eq,
                    DialogueValue::Bool(true),
                )),
                DialogueChoice::new("normal", 2),
            ],
        );
    // Plain API: only the unconditional "normal" choice is exposed.
    let plain = d.pending_choices().expect("unconditional choice present");
    assert_eq!(plain.len(), 1);
    assert_eq!(plain[0].text, "normal");

    // choose(0) picks the first unconditional choice ("normal" → goto 2).
    d.choose(0);
    assert_eq!(d.current, 2);

    // Vars-aware API with vip=true sees both choices.
    let d2 = DialogueBox::new("NPC", ["pick", "secret", "normal"])
        .with_chars_per_sec(0.0)
        .with_choices(
            0,
            [
                DialogueChoice::new("secret", 1).when(DialogueCond::new(
                    "vip",
                    DialogueOp::Eq,
                    DialogueValue::Bool(true),
                )),
                DialogueChoice::new("normal", 2),
            ],
        );
    let mut vars = DialogueVars::new();
    vars.set_bool("vip", true);
    assert_eq!(d2.visible_choices(&vars).len(), 2);
}

/// Existing behavior: all-unconditional choices are fully counted and unchanged.
#[test]
fn plain_api_all_unconditional_unchanged() {
    let mut d = DialogueBox::new("NPC", ["Q", "A", "B"])
        .with_chars_per_sec(0.0)
        .with_choices(
            0,
            [DialogueChoice::new("a", 1), DialogueChoice::new("b", 2)],
        );
    // Both choices unconditional → plain pending_choices() returns both.
    assert_eq!(d.pending_choices().map(|v| v.len()), Some(2));
    // advance is blocked while unconditional choices are pending.
    d.advance();
    assert_eq!(d.current, 0);
    // choose(1) selects "b" → goto 2.
    d.choose(1);
    assert_eq!(d.current, 2);
}

/// NaN `chars_per_sec` must reveal the full line instantly (not render blank).
#[test]
fn nan_chars_per_sec_reveals_instantly() {
    let d = DialogueBox::new("", ["hello world"]).with_chars_per_sec(f32::NAN);
    assert_eq!(d.visible_text(), "hello world");
    assert!(d.line_fully_revealed());
}

/// `+Inf` `chars_per_sec` also reveals instantly (complementary non-finite guard).
#[test]
fn infinite_chars_per_sec_reveals_instantly() {
    let d = DialogueBox::new("", ["hello"]).with_chars_per_sec(f32::INFINITY);
    assert_eq!(d.visible_text(), "hello");
    assert!(d.line_fully_revealed());
}

/// With no `DialogueStyle` resource, `DialogueSystem` draws the speaker + body at the original
/// hardcoded positions / sizes / colors — locks in "default matches the previous look".
#[test]
fn dialogue_system_default_style_matches_original_literals() {
    use crate::ecs::{System, World};
    use crate::renderer::TextQueue;
    use crate::resources::ViewportSize;

    let mut world = World::new();
    world.insert_resource(ViewportSize::new(900, 600));
    world.insert_resource(TextQueue::default());
    let e = world.spawn();
    world.add_component(
        e,
        DialogueBox::new("NPC", ["hello"]).with_chars_per_sec(0.0),
    );

    DialogueSystem.run(&mut world, 0.016);

    let tq = world.resource::<TextQueue>().unwrap();
    // Speaker: x = text_margin 60, y = vh(600) − speaker_bottom_offset 150 = 450, size 22, gold.
    let speaker = tq.iter().find(|t| t.text == "NPC").expect("speaker drawn");
    assert_eq!(speaker.position, crate::Vec2::new(60.0, 450.0));
    assert!((speaker.size - 22.0).abs() < f32::EPSILON);
    assert_eq!(speaker.color, crate::Color::rgb(1.0, 0.85, 0.35));
    // Body: x = 60, y = 600 − 118 = 482, size 20, white.
    let body = tq.iter().find(|t| t.text == "hello").expect("body drawn");
    assert_eq!(body.position, crate::Vec2::new(60.0, 482.0));
    assert!((body.size - 20.0).abs() < f32::EPSILON);
    assert_eq!(body.color, crate::Color::WHITE);
}

/// A custom `DialogueStyle` resource overrides the layout — confirms the resource is wired through.
#[test]
fn dialogue_system_custom_style_overrides() {
    use crate::ecs::{System, World};
    use crate::renderer::TextQueue;
    use crate::resources::ViewportSize;

    let mut world = World::new();
    world.insert_resource(ViewportSize::new(900, 600));
    world.insert_resource(TextQueue::default());
    world.insert_resource(DialogueStyle {
        text_margin: 110.0,
        speaker_font_size: 28.0,
        speaker_color: crate::Color::rgb(0.0, 1.0, 1.0),
        ..Default::default()
    });
    let e = world.spawn();
    world.add_component(
        e,
        DialogueBox::new("NPC", ["hello"]).with_chars_per_sec(0.0),
    );

    DialogueSystem.run(&mut world, 0.016);

    let tq = world.resource::<TextQueue>().unwrap();
    let speaker = tq.iter().find(|t| t.text == "NPC").expect("speaker drawn");
    assert!((speaker.position.x - 110.0).abs() < f32::EPSILON); // text_margin override
    assert!((speaker.size - 28.0).abs() < f32::EPSILON);
    assert_eq!(speaker.color, crate::Color::rgb(0.0, 1.0, 1.0));
}

// ── Multi-term choice gates (`cond_all` / `cond_any`) ────────────────────────
//
// The motivating case is `rpg_quest`'s shop: `gold >= 10 && !has_lantern` is an ordinary RPG
// requirement that a single `cond` cannot express, and its negation — the "no deal" branch —
// is the matching disjunction. Both halves are exercised below.
//
// Every test here drives the gate in *both* directions. A gate that is never satisfied and a
// gate that is never read produce the same green assertion, which is the trap
// `docs/VERIFICATION.md` § *a fixture that omits the subject reads clean* is about.

fn gold_and_no_lantern() -> [DialogueCond; 2] {
    [
        DialogueCond::new("gold", DialogueOp::Ge, DialogueValue::Int(10)),
        DialogueCond::new("has_lantern", DialogueOp::Eq, DialogueValue::Bool(false)),
    ]
}

/// `cond_all` is a conjunction: **both** terms must hold, and failing either one hides the
/// choice. The two single-term failures are the controls — without them, a `cond_all` that was
/// silently ignored would still pass the "both true → visible" assertion.
#[test]
fn cond_all_requires_every_term() {
    let d = DialogueBox::new("Merchant", ["ask", "buy"])
        .with_chars_per_sec(0.0)
        .with_choices(
            0,
            [DialogueChoice::localized("choice.buy", 1).when_all(gold_and_no_lantern())],
        );

    // Neither term holds (gold unset → 0, has_lantern unset → false): the gold term fails.
    let mut vars = DialogueVars::new();
    assert_eq!(d.visible_choices(&vars).len(), 0, "broke → no offer");

    // Only the gold term holds.
    vars.set_int("gold", 10);
    vars.set_bool("has_lantern", true);
    assert_eq!(
        d.visible_choices(&vars).len(),
        0,
        "rich but already carrying a lantern → no offer; the second term is not being read"
    );

    // Only the lantern term holds.
    vars.set_int("gold", 9);
    vars.set_bool("has_lantern", false);
    assert_eq!(
        d.visible_choices(&vars).len(),
        0,
        "needs one more gold → no offer; the first term is not being read"
    );

    // Both hold.
    vars.set_int("gold", 10);
    assert_eq!(
        d.visible_choices(&vars).len(),
        1,
        "exactly at the price with no lantern → the offer is on"
    );
}

/// `cond_any` is a disjunction: **one** term is enough, and it hides the choice only when every
/// term fails. This is `rpg_quest`'s "no deal" branch — the negation of the conjunction above.
#[test]
fn cond_any_requires_one_term() {
    let d = DialogueBox::new("Merchant", ["ask", "nodeal"])
        .with_chars_per_sec(0.0)
        .with_choices(
            0,
            [DialogueChoice::localized("choice.nodeal", 1).when_any([
                DialogueCond::new("gold", DialogueOp::Lt, DialogueValue::Int(10)),
                DialogueCond::new("has_lantern", DialogueOp::Eq, DialogueValue::Bool(true)),
            ])],
        );

    // Both terms fail → hidden. The control: this is the only reading that can go wrong if
    // `cond_any` is treated as "always passes".
    let mut vars = DialogueVars::new();
    vars.set_int("gold", 10);
    vars.set_bool("has_lantern", false);
    assert_eq!(d.visible_choices(&vars).len(), 0, "can buy → no 'no deal'");

    // First term only.
    vars.set_int("gold", 9);
    assert_eq!(d.visible_choices(&vars).len(), 1, "too poor → 'no deal'");

    // Second term only.
    vars.set_int("gold", 10);
    vars.set_bool("has_lantern", true);
    assert_eq!(
        d.visible_choices(&vars).len(),
        1,
        "already has one → 'no deal'"
    );

    // Both terms.
    vars.set_int("gold", 9);
    assert_eq!(d.visible_choices(&vars).len(), 1, "both → still one choice");
}

/// The three gate fields are ANDed, so they compose into `a && (b || c)` without nesting.
/// Each of the three is individually falsified to prove all three are read.
#[test]
fn the_three_gates_are_anded_together() {
    let choice = DialogueChoice::new("go", 1)
        .when(DialogueCond::new(
            "quest_open",
            DialogueOp::Eq,
            DialogueValue::Bool(true),
        ))
        .when_all([DialogueCond::new(
            "gold",
            DialogueOp::Ge,
            DialogueValue::Int(5),
        )])
        .when_any([
            DialogueCond::new("has_map", DialogueOp::Eq, DialogueValue::Bool(true)),
            DialogueCond::new("knows_mine", DialogueOp::Eq, DialogueValue::Bool(true)),
        ]);
    let d = DialogueBox::new("NPC", ["ask", "go"])
        .with_chars_per_sec(0.0)
        .with_choices(0, [choice]);

    let mut vars = DialogueVars::new();
    vars.set_bool("quest_open", true);
    vars.set_int("gold", 5);
    vars.set_bool("knows_mine", true);
    assert_eq!(d.visible_choices(&vars).len(), 1, "all three gates pass");

    // Falsify `cond` alone.
    vars.set_bool("quest_open", false);
    assert_eq!(d.visible_choices(&vars).len(), 0, "`cond` is not read");
    vars.set_bool("quest_open", true);

    // Falsify `cond_all` alone.
    vars.set_int("gold", 4);
    assert_eq!(d.visible_choices(&vars).len(), 0, "`cond_all` is not read");
    vars.set_int("gold", 5);

    // Falsify `cond_any` alone (neither disjunct holds).
    vars.set_bool("knows_mine", false);
    assert_eq!(d.visible_choices(&vars).len(), 0, "`cond_any` is not read");

    // And restoring the other disjunct brings it back — `cond_any` is a disjunction, not a
    // second conjunction.
    vars.set_bool("has_map", true);
    assert_eq!(
        d.visible_choices(&vars).len(),
        1,
        "either disjunct suffices"
    );
}

/// A choice gated *only* by the new fields must not be treated as unconditional. The plain
/// (vars-unaware) API offers unconditional choices to avoid deadlocking a line, so a
/// `cond_all`-only choice leaking into it would let the player pick a gated branch.
#[test]
fn multi_term_gates_are_not_unconditional() {
    assert!(DialogueChoice::new("plain", 1).is_unconditional());
    assert!(
        !DialogueChoice::new("all", 1)
            .when_all(gold_and_no_lantern())
            .is_unconditional(),
        "a cond_all-gated choice is not unconditional"
    );
    assert!(
        !DialogueChoice::new("any", 1)
            .when_any(gold_and_no_lantern())
            .is_unconditional(),
        "a cond_any-gated choice is not unconditional"
    );
    // An empty list is not a gate: `when_all([])` restores the unconditional reading.
    assert!(
        DialogueChoice::new("empty", 1)
            .when_all([])
            .when_any([])
            .is_unconditional(),
        "empty gate lists pass vacuously and leave the choice unconditional"
    );

    // The behavioural half: the plain API hides it, the vars-aware API shows it.
    let d = DialogueBox::new("NPC", ["pick", "gated", "plain"])
        .with_chars_per_sec(0.0)
        .with_choices(
            0,
            [
                DialogueChoice::new("gated", 1).when_all(gold_and_no_lantern()),
                DialogueChoice::new("plain", 2),
            ],
        );
    let plain = d.pending_choices().expect("one unconditional choice");
    assert_eq!(plain.len(), 1);
    assert_eq!(plain[0].text, "plain");

    let mut vars = DialogueVars::new();
    vars.set_int("gold", 10);
    assert_eq!(d.visible_choices(&vars).len(), 2, "vars-aware sees both");
}
