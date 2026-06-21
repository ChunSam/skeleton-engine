use super::*;
use crate::animation::player::AnimationClip;
use crate::animation::system::AnimationSystem;
use crate::ecs::World;
use crate::renderer::uv::UvRect;

fn loop_clip() -> AnimationClip {
    AnimationClip {
        frames: vec![UvRect::FULL, UvRect::FULL],
        fps: 10.0,
        looping: true,
    }
}

fn one_shot_clip() -> AnimationClip {
    AnimationClip {
        frames: vec![UvRect::FULL, UvRect::FULL],
        fps: 10.0,
        looping: false,
    }
}

/// `add_transition` (no crossfade) performs an instant hard switch.
#[test]
fn hard_switch_transitions_instant() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, AnimationPlayer::new(vec![loop_clip(), loop_clip()]));

    let mut sm = AnimationStateMachine::new("idle", 0);
    sm.add_state("run", 1);
    sm.set_bool("is_running", false);
    sm.add_transition(
        "idle",
        "run",
        vec![TransitionCond::BoolEq("is_running".into(), true)],
    );
    world.add_component(e, sm);

    // Fire condition.
    world
        .get_mut::<AnimationStateMachine>(e)
        .unwrap()
        .set_bool("is_running", true);

    let mut anim = AnimationSystem::new();
    let mut stm = StateMachineSystem::new();
    anim.run(&mut world, 0.05);
    stm.run(&mut world, 0.05);

    let player = world.get::<AnimationPlayer>(e).unwrap();
    assert_eq!(player.current_clip, 1, "should have switched to clip 1");
    assert!(
        !player.is_crossfading(),
        "hard switch must not start a crossfade"
    );
}

/// `add_transition_crossfade` starts a blend rather than an instant switch.
#[test]
fn crossfade_transition_starts_blend() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, AnimationPlayer::new(vec![loop_clip(), loop_clip()]));

    let mut sm = AnimationStateMachine::new("idle", 0);
    sm.add_state("run", 1);
    sm.set_bool("is_running", false);
    sm.add_transition_crossfade(
        "idle",
        "run",
        vec![TransitionCond::BoolEq("is_running".into(), true)],
        0.2, // 200 ms crossfade
    );
    world.add_component(e, sm);

    world
        .get_mut::<AnimationStateMachine>(e)
        .unwrap()
        .set_bool("is_running", true);

    let mut anim = AnimationSystem::new();
    let mut stm = StateMachineSystem::new();
    anim.run(&mut world, 0.05);
    stm.run(&mut world, 0.05);

    let player = world.get::<AnimationPlayer>(e).unwrap();
    // State machine has committed to clip 1, but `current_clip` stays 0 until the
    // crossfade completes (AnimationSystem promotes the "to" clip when elapsed >= duration).
    assert!(
        player.is_crossfading(),
        "crossfade transition must start a blend"
    );
}

/// After the crossfade duration elapses, `AnimationSystem` completes the switch.
#[test]
fn crossfade_transition_completes_after_duration() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, AnimationPlayer::new(vec![loop_clip(), loop_clip()]));

    let mut sm = AnimationStateMachine::new("idle", 0);
    sm.add_state("run", 1);
    sm.set_bool("is_running", false);
    sm.add_transition_crossfade(
        "idle",
        "run",
        vec![TransitionCond::BoolEq("is_running".into(), true)],
        0.1, // 100 ms crossfade
    );
    world.add_component(e, sm);

    world
        .get_mut::<AnimationStateMachine>(e)
        .unwrap()
        .set_bool("is_running", true);

    let mut anim = AnimationSystem::new();
    let mut stm = StateMachineSystem::new();

    // Tick enough frames to exceed the 0.1 s crossfade.
    for _ in 0..20 {
        anim.run(&mut world, 0.01);
        stm.run(&mut world, 0.01);
    }

    let player = world.get::<AnimationPlayer>(e).unwrap();
    assert!(!player.is_crossfading(), "crossfade must be complete");
    assert_eq!(
        player.current_clip, 1,
        "must have settled on the target clip"
    );
}

/// `AnimationEnd` condition fires correctly for a non-looping clip.
#[test]
fn animation_end_condition_triggers_transition() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, AnimationPlayer::new(vec![one_shot_clip(), loop_clip()]));

    let mut sm = AnimationStateMachine::new("attack", 0);
    sm.add_state("idle", 1);
    sm.add_transition("attack", "idle", vec![TransitionCond::AnimationEnd]);
    world.add_component(e, sm);

    let mut anim = AnimationSystem::new();
    let mut stm = StateMachineSystem::new();

    // Advance past the clip's last frame (2 frames at 10 fps -> 0.2 s).
    for _ in 0..30 {
        anim.run(&mut world, 0.01);
        stm.run(&mut world, 0.01);
    }

    let player = world.get::<AnimationPlayer>(e).unwrap();
    assert_eq!(player.current_clip, 1, "should have transitioned to idle");
}

/// A `crossfade_duration: 0.0` stored on `AnimTransition` must behave like a hard switch.
#[test]
fn zero_crossfade_duration_is_hard_switch() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, AnimationPlayer::new(vec![loop_clip(), loop_clip()]));

    let mut sm = AnimationStateMachine::new("a", 0);
    sm.add_state("b", 1);
    sm.add_trigger("go");
    sm.add_transition_crossfade("a", "b", vec![TransitionCond::Trigger("go".into())], 0.0);
    world.add_component(e, sm);

    world
        .get_mut::<AnimationStateMachine>(e)
        .unwrap()
        .fire_trigger("go");

    let mut anim = AnimationSystem::new();
    let mut stm = StateMachineSystem::new();
    anim.run(&mut world, 0.05);
    stm.run(&mut world, 0.05);

    let player = world.get::<AnimationPlayer>(e).unwrap();
    assert_eq!(player.current_clip, 1);
    assert!(!player.is_crossfading());
}

// ── Regression tests for crossfade guard fixes ─────────────────────────────

/// (Fix 1a) Re-firing `play_with_crossfade` to the same in-flight target must NOT
/// reset `elapsed` — the blend must still complete.
#[test]
fn refiring_same_crossfade_target_does_not_reset_elapsed() {
    use crate::animation::player::AnimationClip;

    let mut player = AnimationPlayer::new(vec![
        AnimationClip {
            frames: vec![UvRect::FULL, UvRect::FULL],
            fps: 10.0,
            looping: true,
        },
        AnimationClip {
            frames: vec![UvRect::FULL, UvRect::FULL],
            fps: 10.0,
            looping: true,
        },
    ]);

    // Start a 0.5 s crossfade to clip 1.
    player.play_with_crossfade(1, 0.5);
    assert!(player.is_crossfading());

    // Manually advance elapsed to 0.3 s by inspecting internal state.
    player.crossfade.as_mut().unwrap().elapsed = 0.3;

    // Re-fire to the same target — must be a no-op.
    player.play_with_crossfade(1, 0.5);
    let elapsed = player.crossfade.as_ref().unwrap().elapsed;
    assert!(
        (elapsed - 0.3).abs() < 1e-6,
        "elapsed was reset to {elapsed} instead of staying at 0.3"
    );
}

/// (Fix 1b) With a noisy threshold parameter oscillating across the boundary every
/// frame, the crossfade must still complete within its duration instead of being
/// reset every frame and never finishing.
#[test]
fn oscillating_threshold_param_lets_crossfade_complete() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, AnimationPlayer::new(vec![loop_clip(), loop_clip()]));

    let mut sm = AnimationStateMachine::new("idle", 0);
    sm.add_state("run", 1);
    sm.set_float("speed", 0.0);
    // Crossfade 0.3 s.
    sm.add_transition_crossfade(
        "idle",
        "run",
        vec![TransitionCond::FloatGt("speed".into(), 0.5)],
        0.3,
    );
    // No back-transition — we just want to test the forward blend.
    world.add_component(e, sm);

    let mut anim = AnimationSystem::new();
    let mut stm = StateMachineSystem::new();

    // First tick: push speed above the threshold to start the crossfade.
    world
        .get_mut::<AnimationStateMachine>(e)
        .unwrap()
        .set_float("speed", 1.0);
    anim.run(&mut world, 0.01);
    stm.run(&mut world, 0.01);
    assert!(
        world.get::<AnimationPlayer>(e).unwrap().is_crossfading(),
        "crossfade should have started"
    );

    // Subsequent ticks: oscillate the speed around the threshold every frame.
    // Without the guard this would reset elapsed=0 every other frame and the
    // blend would never complete.
    for tick in 0..60 {
        // Alternate above/below threshold to simulate noisy input.
        let speed = if tick % 2 == 0 { 1.0 } else { 0.3 };
        world
            .get_mut::<AnimationStateMachine>(e)
            .unwrap()
            .set_float("speed", speed);
        anim.run(&mut world, 0.01);
        stm.run(&mut world, 0.01);
    }

    // After 60 frames × 0.01 s = 0.6 s the blend must have completed (duration 0.3 s).
    let player = world.get::<AnimationPlayer>(e).unwrap();
    assert!(
        !player.is_crossfading(),
        "crossfade must have completed despite oscillating parameter"
    );
    assert_eq!(
        player.current_clip, 1,
        "player must have settled on the run clip"
    );
}

/// (Fix 2) A crossfaded-into one-shot state must play its full clip before
/// `AnimationEnd` fires. The FROM clip was already finished; without the fix
/// the new state would exit immediately after one frame.
///
/// Setup:
///  - clip 0 (idle, one-shot, 2 frames at 10 fps → 0.2 s)
///  - clip 1 (attack, one-shot, 6 frames at 10 fps → 0.6 s)
///  - clip 2 (idle_loop, looping)
///  - SM: idle → attack on trigger with 0.05 s crossfade
///  - SM: attack → idle_loop on AnimationEnd
///
/// The crossfade (0.05 s) completes well before the attack clip (0.6 s).
/// During the crossfade the FROM clip (0) is finished; without the fix
/// `AnimationEnd` would fire on the attack state immediately, skipping it.
#[test]
fn crossfaded_one_shot_state_survives_until_its_clip_finishes() {
    fn long_one_shot() -> AnimationClip {
        // 6 frames at 10 fps → 0.6 s total duration
        AnimationClip {
            frames: vec![
                UvRect::FULL,
                UvRect::FULL,
                UvRect::FULL,
                UvRect::FULL,
                UvRect::FULL,
                UvRect::FULL,
            ],
            fps: 10.0,
            looping: false,
        }
    }

    let mut world = World::new();
    let e = world.spawn();
    // Clip 0: short one-shot (FROM state, finishes quickly).
    // Clip 1: long one-shot (attack state, 0.6 s).
    // Clip 2: looping idle.
    world.add_component(
        e,
        AnimationPlayer::new(vec![one_shot_clip(), long_one_shot(), loop_clip()]),
    );

    // idle (clip 0) → attack (clip 1) on trigger, short crossfade (50 ms).
    // attack (clip 1) → idle_loop (clip 2) on AnimationEnd.
    let mut sm = AnimationStateMachine::new("idle", 0);
    sm.add_state("attack", 1);
    sm.add_state("idle_loop", 2);
    sm.add_trigger("attack");
    sm.add_transition_crossfade(
        "idle",
        "attack",
        vec![TransitionCond::Trigger("attack".into())],
        0.05, // 50 ms crossfade — much shorter than attack clip (0.6 s)
    );
    sm.add_transition("attack", "idle_loop", vec![TransitionCond::AnimationEnd]);
    world.add_component(e, sm);

    let mut anim = AnimationSystem::new();
    let mut stm = StateMachineSystem::new();

    // Advance clip 0 past its last frame so is_finished() returns true for clip 0.
    // Clip 0: 2 frames at 10 fps → needs > 0.1 s.  Run 15 ticks × 0.01 s = 0.15 s.
    for _ in 0..15 {
        anim.run(&mut world, 0.01);
        stm.run(&mut world, 0.01);
    }
    // SM still in "idle" — no transition condition defined for idle yet.
    assert_eq!(
        world
            .get_mut::<AnimationStateMachine>(e)
            .unwrap()
            .current_state(),
        "idle",
        "still in idle before trigger"
    );
    // Clip 0 must be finished so the pre-fix regression is exercised.
    assert!(
        world.get::<AnimationPlayer>(e).unwrap().is_finished(),
        "clip 0 must be on its last frame before we fire the trigger"
    );

    // Fire trigger — SM commits to "attack" state, starts 50 ms crossfade clip 0 → clip 1.
    world
        .get_mut::<AnimationStateMachine>(e)
        .unwrap()
        .fire_trigger("attack");
    anim.run(&mut world, 0.01);
    stm.run(&mut world, 0.01);

    assert_eq!(
        world
            .get_mut::<AnimationStateMachine>(e)
            .unwrap()
            .current_state(),
        "attack",
        "SM must have entered attack state"
    );
    // Crossfade is in progress — SM must NOT exit "attack" via AnimationEnd
    // even though clip 0 (current_clip) is still finished.
    assert!(
        world.get::<AnimationPlayer>(e).unwrap().is_crossfading(),
        "crossfade must be in progress right after transition"
    );

    // Run enough to exceed the 0.05 s crossfade.  We use 10 ticks × 0.01 s = 0.10 s so
    // that f32 rounding (5 × 0.01f32 can be slightly < 0.05f32) does not trip us up.
    for _ in 0..10 {
        anim.run(&mut world, 0.01);
        stm.run(&mut world, 0.01);
    }
    assert!(
        !world.get::<AnimationPlayer>(e).unwrap().is_crossfading(),
        "crossfade must be complete after 0.10 s"
    );
    assert_eq!(
        world.get::<AnimationPlayer>(e).unwrap().current_clip,
        1,
        "player must have promoted to clip 1 after crossfade"
    );
    // Attack clip (6 frames at 10 fps) has been advancing for 0.10 s inside
    // the crossfade — at most 1 frame done (frame_dur = 0.1 s), far from finished.
    assert_eq!(
        world
            .get_mut::<AnimationStateMachine>(e)
            .unwrap()
            .current_state(),
        "attack",
        "still in attack — clip 1 has not yet played to completion"
    );

    // Now let clip 1 (attack, 6 frames at 10 fps → 0.6 s) run to completion.
    // Run 70 more ticks × 0.01 s = 0.7 s — plenty to finish the remaining frames.
    for _ in 0..70 {
        anim.run(&mut world, 0.01);
        stm.run(&mut world, 0.01);
    }
    assert_eq!(
        world
            .get_mut::<AnimationStateMachine>(e)
            .unwrap()
            .current_state(),
        "idle_loop",
        "SM must have transitioned to idle_loop after attack clip finished"
    );
}

// ── Editor accessors + edit operations ─────────────────────────────────────

fn editor_sm() -> AnimationStateMachine {
    let mut sm = AnimationStateMachine::new("idle", 0);
    sm.add_state("run", 1).add_state("jump", 2);
    sm.add_transition(
        "idle",
        "run",
        vec![TransitionCond::BoolEq("running".into(), true)],
    );
    sm.add_transition("run", "idle", vec![TransitionCond::AnimationEnd]);
    sm.add_transition("idle", "jump", vec![TransitionCond::Trigger("jump".into())]);
    sm
}

#[test]
fn state_accessors_report_states_and_transitions() {
    let sm = editor_sm();
    assert_eq!(sm.state_count(), 3);
    assert_eq!(sm.state_names(), vec!["idle", "jump", "run"]); // sorted
    let idle = sm.state("idle").expect("idle state");
    assert_eq!(idle.transitions.len(), 2, "idle → run, idle → jump");
    assert_eq!(idle.clip_index, 0);
    assert!(sm.state("missing").is_none());
}

#[test]
fn set_current_and_set_clip() {
    let mut sm = editor_sm();
    assert!(sm.set_current_state("run"));
    assert_eq!(sm.current_state(), "run");
    assert!(!sm.set_current_state("nope"));
    assert_eq!(sm.current_state(), "run", "unchanged on bad name");

    assert!(sm.set_state_clip("jump", 7));
    assert_eq!(sm.state("jump").unwrap().clip_index, 7);
    assert!(!sm.set_state_clip("nope", 1));
}

#[test]
fn remove_state_prunes_inbound_transitions() {
    let mut sm = editor_sm();
    // idle is current → cannot remove it.
    assert!(!sm.remove_state("idle"), "cannot remove the active state");
    // jump has an inbound transition from idle. Remove it.
    assert!(sm.remove_state("jump"));
    assert_eq!(sm.state_count(), 2);
    assert!(sm.state("jump").is_none());
    // idle's transition to jump must be pruned (only idle→run remains).
    let idle = sm.state("idle").unwrap();
    assert_eq!(idle.transitions.len(), 1);
    assert_eq!(idle.transitions[0].to, "run");
    assert!(!sm.remove_state("missing"));
}

#[test]
fn remove_state_refuses_last_state() {
    let mut sm = AnimationStateMachine::new("only", 0);
    assert!(!sm.remove_state("only"), "cannot remove the last state");
    assert_eq!(sm.state_count(), 1);
}

#[test]
fn remove_transition_by_index() {
    let mut sm = editor_sm();
    assert_eq!(sm.state("idle").unwrap().transitions.len(), 2);
    assert!(sm.remove_transition("idle", 0));
    assert_eq!(sm.state("idle").unwrap().transitions.len(), 1);
    // Out-of-range index and missing state are no-ops.
    assert!(!sm.remove_transition("idle", 9));
    assert!(!sm.remove_transition("missing", 0));
}

#[test]
fn param_accessors() {
    let mut sm = editor_sm();
    sm.set_bool("running", true);
    sm.set_float("speed", 1.5);
    sm.add_trigger("jump");
    assert_eq!(sm.param_names(), vec!["jump", "running", "speed"]); // sorted
    assert!(matches!(sm.param("running"), Some(AnimParam::Bool(true))));
    assert!(matches!(sm.param("speed"), Some(AnimParam::Float(_))));
    assert!(matches!(sm.param("jump"), Some(AnimParam::Trigger(_))));
    assert!(sm.param("missing").is_none());
}

// ── Dead-edge warning test (fix 3) ─────────────────────────────────────────

/// Adding a transition to a state that does not exist yet is allowed (dead edge),
/// but must not fire when conditions are met — evaluate() silently skips it.
#[test]
fn transition_to_nonexistent_state_does_not_fire() {
    // Build a state machine that has a transition to a state that is never registered.
    // The transition target "ghost" does not exist; evaluate() must skip it rather than panic.
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, AnimationPlayer::new(vec![loop_clip(), loop_clip()]));

    let mut sm = AnimationStateMachine::new("idle", 0);
    sm.set_bool("go", false);
    // add_transition_crossfade internally calls add_transition_crossfade — the
    // target "ghost" is not registered and should trigger the log::warn path.
    sm.add_transition(
        "idle",
        "ghost",
        vec![TransitionCond::BoolEq("go".into(), true)],
    );
    world.add_component(e, sm);

    // Fire the condition.
    world
        .get_mut::<AnimationStateMachine>(e)
        .unwrap()
        .set_bool("go", true);

    let mut anim = AnimationSystem::new();
    let mut stm = StateMachineSystem::new();
    anim.run(&mut world, 0.05);
    stm.run(&mut world, 0.05);

    // The SM must remain in "idle" — the dead-edge transition must not fire.
    assert_eq!(
        world
            .get_mut::<AnimationStateMachine>(e)
            .unwrap()
            .current_state(),
        "idle",
        "dead-edge transition to nonexistent state must not fire"
    );
    // Player must still be on clip 0.
    assert_eq!(
        world.get::<AnimationPlayer>(e).unwrap().current_clip,
        0,
        "clip must remain 0 when transition target state does not exist"
    );
}

// ── Serde round-trip tests ─────────────────────────────────────────────────

fn rich_sm() -> AnimationStateMachine {
    let mut sm = AnimationStateMachine::new("idle", 0);
    sm.add_state("run", 1).add_state("jump", 2);
    sm.set_bool("is_running", false);
    sm.set_float("speed", 0.0);
    sm.add_trigger("jump");
    sm.add_transition(
        "idle",
        "run",
        vec![TransitionCond::BoolEq("is_running".into(), true)],
    );
    sm.add_transition_crossfade(
        "run",
        "idle",
        vec![TransitionCond::FloatLt("speed".into(), 0.1)],
        0.2,
    );
    sm.add_transition("idle", "jump", vec![TransitionCond::Trigger("jump".into())]);
    sm.add_transition("jump", "idle", vec![TransitionCond::AnimationEnd]);
    sm
}

#[test]
fn state_machine_serde_round_trip_ron() {
    let original = rich_sm();
    let serialized = ron::to_string(&original).expect("serialize must succeed");
    let deserialized: AnimationStateMachine =
        ron::from_str(&serialized).expect("deserialize must succeed");

    assert_eq!(original, deserialized, "round-trip must be lossless");
}

#[test]
fn state_machine_serde_preserves_state_count_and_names() {
    let sm = rich_sm();
    let s = ron::to_string(&sm).unwrap();
    let rt: AnimationStateMachine = ron::from_str(&s).unwrap();

    assert_eq!(rt.state_count(), 3);
    assert_eq!(rt.state_names(), sm.state_names());
}

#[test]
fn state_machine_serde_preserves_transitions_and_crossfade() {
    let sm = rich_sm();
    let s = ron::to_string(&sm).unwrap();
    let rt: AnimationStateMachine = ron::from_str(&s).unwrap();

    // idle has two transitions: →run (bool) and →jump (trigger)
    let idle = rt.state("idle").expect("idle must exist after round-trip");
    assert_eq!(idle.transitions.len(), 2);

    // run has one crossfade transition back to idle
    let run = rt.state("run").expect("run must exist after round-trip");
    assert_eq!(run.transitions.len(), 1);
    assert!((run.transitions[0].crossfade_duration - 0.2).abs() < 1e-6);
}

#[test]
fn state_machine_serde_preserves_params() {
    let sm = rich_sm();
    let s = ron::to_string(&sm).unwrap();
    let rt: AnimationStateMachine = ron::from_str(&s).unwrap();

    assert!(matches!(
        rt.param("is_running"),
        Some(AnimParam::Bool(false))
    ));
    assert!(matches!(rt.param("speed"), Some(AnimParam::Float(_))));
    assert!(matches!(rt.param("jump"), Some(AnimParam::Trigger(false))));
}

#[test]
fn state_machine_serde_registry_round_trip() {
    use crate::ecs::World;
    use crate::prefab::SerdeComponentRegistry;

    let mut registry = SerdeComponentRegistry::default();
    registry.register::<AnimationStateMachine>("AnimationStateMachine", None);

    // Build world with the component.
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, rich_sm());

    // Serialize.
    let components = registry.serialize_entity(&world, e);
    assert!(
        components.contains_key("AnimationStateMachine"),
        "registry must serialize the component"
    );

    // Deserialize into a fresh entity in a new world.
    let mut world2 = World::new();
    let e2 = world2.spawn();
    registry.deserialize_into(&mut world2, e2, &components);

    let restored = world2
        .get::<AnimationStateMachine>(e2)
        .expect("component must be present after registry round-trip");
    assert_eq!(*restored, rich_sm());
}

// ── set_transition_conditions tests ───────────────────────────────────────

#[test]
fn set_transition_conditions_success() {
    let mut sm = editor_sm();
    // editor_sm() has idle→run (index 0) and idle→jump (index 1).
    let new_conds = vec![
        TransitionCond::FloatGt("speed".into(), 0.5),
        TransitionCond::BoolEq("running".into(), true),
    ];
    assert!(sm.set_transition_conditions("idle", 0, new_conds.clone()));
    let idle = sm.state("idle").unwrap();
    assert_eq!(idle.transitions[0].conditions, new_conds);
}

#[test]
fn set_transition_conditions_missing_state() {
    let mut sm = editor_sm();
    assert!(!sm.set_transition_conditions("nope", 0, vec![]));
}

#[test]
fn set_transition_conditions_out_of_range() {
    let mut sm = editor_sm();
    assert!(!sm.set_transition_conditions("idle", 99, vec![]));
}

// ── set_transition_crossfade tests ────────────────────────────────────────

#[test]
fn set_transition_crossfade_success() {
    let mut sm = editor_sm();
    // idle→run is transition index 0; set crossfade to 0.3 s.
    assert!(sm.set_transition_crossfade("idle", 0, 0.3));
    let idle = sm.state("idle").unwrap();
    assert!((idle.transitions[0].crossfade_duration - 0.3).abs() < 1e-6);
}

#[test]
fn set_transition_crossfade_missing_state() {
    let mut sm = editor_sm();
    assert!(!sm.set_transition_crossfade("nope", 0, 0.5));
}

#[test]
fn set_transition_crossfade_out_of_range() {
    let mut sm = editor_sm();
    assert!(!sm.set_transition_crossfade("idle", 99, 0.5));
}

/// A transition with no conditions must never auto-fire. Regression: `[].iter().all()`
/// returns `true`, so a condition-less transition (the editor's placeholder) used to jump
/// on the first evaluated frame.
#[test]
fn empty_conditions_never_fire() {
    let mut world = World::new();
    let e = world.spawn();
    world.add_component(e, AnimationPlayer::new(vec![loop_clip(), loop_clip()]));

    let mut sm = AnimationStateMachine::new("idle", 0);
    sm.add_state("run", 1);
    sm.add_transition("idle", "run", vec![]); // condition-less placeholder
    world.add_component(e, sm);

    let mut anim = AnimationSystem::new();
    let mut stm = StateMachineSystem::new();
    anim.run(&mut world, 0.05);
    stm.run(&mut world, 0.05);

    let player = world.get::<AnimationPlayer>(e).unwrap();
    assert_eq!(
        player.current_clip, 0,
        "empty-condition transition must not fire"
    );
}
