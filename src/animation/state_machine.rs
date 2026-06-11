use std::collections::HashMap;

use crate::animation::player::AnimationPlayer;
use crate::ecs::{Entity, System, World};

// ─── Parameters ───────────────────────────────────────────────────────────────

/// Parameter value held by the state machine.
#[derive(Debug, Clone)]
pub enum AnimParam {
    Bool(bool),
    Float(f32),
    /// Trigger that is valid for one frame only. Activated via `fire_trigger()` and consumed each frame.
    Trigger(bool),
}

// ─── Transition conditions ────────────────────────────────────────────────────

/// A single condition that must be satisfied for a state transition to occur.
#[derive(Debug, Clone)]
pub enum TransitionCond {
    /// When a bool parameter matches the expected value.
    BoolEq(String, bool),
    /// When a float parameter exceeds a threshold.
    FloatGt(String, f32),
    /// When a float parameter is below a threshold.
    FloatLt(String, f32),
    /// When a trigger parameter is active.
    Trigger(String),
    /// When the current clip has reached its end (last frame of a non-looping clip).
    AnimationEnd,
}

// ─── Transitions ─────────────────────────────────────────────────────────────

/// A single state transition edge: target state + list of conditions that must all be met (AND).
#[derive(Debug, Clone)]
pub struct AnimTransition {
    /// Name of the state to transition to.
    pub to: String,
    /// All conditions must be satisfied for the transition to occur.
    pub conditions: Vec<TransitionCond>,
    /// Crossfade duration in seconds. `0.0` means an instant clip switch (default).
    pub crossfade_duration: f32,
}

// ─── State node ───────────────────────────────────────────────────────────────

/// One node in the state machine: an `AnimationPlayer` clip index and a list of transitions.
#[derive(Debug, Clone)]
pub struct AnimState {
    /// `AnimationPlayer` clip index to play in this state.
    pub clip_index: usize,
    /// Transition edges evaluated in this state (evaluated in registration order).
    pub transitions: Vec<AnimTransition>,
}

// ─── State machine component ──────────────────────────────────────────────────

/// Animation state machine component attached to an entity.
///
/// Add it to the same entity as an `AnimationPlayer`, then register `StateMachineSystem`
/// **after** `AnimationSystem`.
///
/// # Registration order
/// ```text
/// app.add_system(Box::new(AnimationSystem));     // advance frames
/// app.add_system(Box::new(StateMachineSystem));  // evaluate transitions
/// ```
///
/// # Interaction with `BlendTree1D`
/// `StateMachineSystem` runs **after** `BlendTreeSystem` in the documented system
/// order. When both components drive the same `AnimationPlayer`, a state-machine
/// transition will **interrupt** any in-progress `BlendTree1D` crossfade — the SM
/// wins because state changes are semantic (the character has entered a new logical
/// state), not merely cosmetic. This is intentional. If you do not want this
/// behaviour, do not attach both `AnimationStateMachine` and `BlendTree1D` to the
/// same entity simultaneously.
///
/// # Example
/// ```rust,ignore
/// let mut sm = AnimationStateMachine::new("idle", 0);
/// sm.add_state("run", 1)
///   .add_state("jump", 2);
/// sm.set_bool("is_running", false);
/// sm.add_trigger("jump");
/// sm.add_transition("idle", "run",  vec![TransitionCond::BoolEq("is_running".into(), true)]);
/// sm.add_transition("run",  "idle", vec![TransitionCond::BoolEq("is_running".into(), false)]);
/// sm.add_transition("idle", "jump", vec![TransitionCond::Trigger("jump".into())]);
/// sm.add_transition("jump", "idle", vec![TransitionCond::AnimationEnd]);
/// world.add_component(entity, sm);
/// ```
#[derive(Debug, Clone)]
pub struct AnimationStateMachine {
    states: HashMap<String, AnimState>,
    current: String,
    params: HashMap<String, AnimParam>,
}

impl AnimationStateMachine {
    /// Creates a state machine with the given initial state name and clip index.
    pub fn new(initial_state: impl Into<String>, initial_clip: usize) -> Self {
        let initial_state = initial_state.into();
        let mut states = HashMap::new();
        states.insert(
            initial_state.clone(),
            AnimState {
                clip_index: initial_clip,
                transitions: Vec::new(),
            },
        );
        Self {
            states,
            current: initial_state,
            params: HashMap::new(),
        }
    }

    // ── State / transition registration ────────────────────────────────────────

    /// Adds a new state. If a state with that name already exists, it is left unchanged.
    pub fn add_state(&mut self, name: impl Into<String>, clip_index: usize) -> &mut Self {
        self.states.entry(name.into()).or_insert(AnimState {
            clip_index,
            transitions: Vec::new(),
        });
        self
    }

    /// Registers a transition edge from `from` to `to` with an instant (hard) clip switch.
    /// Does nothing if the `from` state does not exist.
    pub fn add_transition(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        conditions: Vec<TransitionCond>,
    ) -> &mut Self {
        self.add_transition_crossfade(from, to, conditions, 0.0)
    }

    /// Registers a transition edge from `from` to `to` with a smooth crossfade.
    ///
    /// When the transition fires, the `AnimationPlayer` blends from the old clip to
    /// the new clip over `crossfade_duration` seconds (same mechanism as
    /// `AnimationPlayer::play_with_crossfade` and `BlendTree1D`). Pass `0.0` for an
    /// instant switch, which is equivalent to `add_transition`.
    ///
    /// Does nothing if the `from` state does not exist.
    ///
    /// # Example
    /// ```rust,ignore
    /// sm.add_transition_crossfade("idle", "run",
    ///     vec![TransitionCond::BoolEq("is_running".into(), true)],
    ///     0.1, // 100 ms blend
    /// );
    /// ```
    pub fn add_transition_crossfade(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        conditions: Vec<TransitionCond>,
        crossfade_duration: f32,
    ) -> &mut Self {
        let from = from.into();
        let to = to.into();
        if let Some(state) = self.states.get_mut(&from) {
            state.transitions.push(AnimTransition {
                to,
                conditions,
                crossfade_duration,
            });
        }
        self
    }

    // ── Parameter read / write ─────────────────────────────────────────────────

    /// Sets or updates a bool parameter.
    pub fn set_bool(&mut self, name: impl Into<String>, value: bool) {
        self.params.insert(name.into(), AnimParam::Bool(value));
    }

    /// Reads a bool parameter. Returns `None` if missing or the wrong type.
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        match self.params.get(name) {
            Some(AnimParam::Bool(v)) => Some(*v),
            _ => None,
        }
    }

    /// Sets or updates a float parameter.
    pub fn set_float(&mut self, name: impl Into<String>, value: f32) {
        self.params.insert(name.into(), AnimParam::Float(value));
    }

    /// Reads a float parameter. Returns `None` if missing or the wrong type.
    pub fn get_float(&self, name: &str) -> Option<f32> {
        match self.params.get(name) {
            Some(AnimParam::Float(v)) => Some(*v),
            _ => None,
        }
    }

    /// Registers a trigger parameter (initial value: false).
    pub fn add_trigger(&mut self, name: impl Into<String>) {
        self.params
            .entry(name.into())
            .or_insert(AnimParam::Trigger(false));
    }

    /// Activates a trigger. `StateMachineSystem` consumes it within the same frame.
    pub fn fire_trigger(&mut self, name: &str) {
        if let Some(AnimParam::Trigger(v)) = self.params.get_mut(name) {
            *v = true;
        }
    }

    /// Returns the name of the currently active state.
    pub fn current_state(&self) -> &str {
        &self.current
    }

    // ── Internal evaluation ────────────────────────────────────────────────────

    fn check_condition(&self, cond: &TransitionCond, anim_finished: bool) -> bool {
        match cond {
            TransitionCond::BoolEq(name, expected) => {
                matches!(self.params.get(name.as_str()), Some(AnimParam::Bool(v)) if v == expected)
            }
            TransitionCond::FloatGt(name, threshold) => {
                matches!(self.params.get(name.as_str()), Some(AnimParam::Float(v)) if v > threshold)
            }
            TransitionCond::FloatLt(name, threshold) => {
                matches!(self.params.get(name.as_str()), Some(AnimParam::Float(v)) if v < threshold)
            }
            TransitionCond::Trigger(name) => {
                matches!(
                    self.params.get(name.as_str()),
                    Some(AnimParam::Trigger(true))
                )
            }
            TransitionCond::AnimationEnd => anim_finished,
        }
    }

    /// Finds the first satisfied transition in the current state and returns
    /// `(target state, clip index, crossfade_duration)`.
    fn evaluate(&self, anim_finished: bool) -> Option<(String, usize, f32)> {
        let state = self.states.get(&self.current)?;
        for transition in &state.transitions {
            if transition
                .conditions
                .iter()
                .all(|c| self.check_condition(c, anim_finished))
            {
                let next_clip = self.states.get(&transition.to)?.clip_index;
                return Some((
                    transition.to.clone(),
                    next_clip,
                    transition.crossfade_duration,
                ));
            }
        }
        None
    }

    /// Consumes all trigger parameters (resets them to false).
    fn consume_triggers(&mut self) {
        for param in self.params.values_mut() {
            if let AnimParam::Trigger(v) = param {
                *v = false;
            }
        }
    }
}

// ─── System ───────────────────────────────────────────────────────────────────

/// Evaluates `AnimationStateMachine` transition conditions each frame and,
/// when a condition is met, instructs the `AnimationPlayer` to play the new clip.
///
/// Must be registered **after** `AnimationSystem` so that `is_finished()` is
/// reflected in the same frame.
pub struct StateMachineSystem;

impl StateMachineSystem {
    /// Schedule label. Recommended order: **after** `AnimationSystem::LABEL`
    /// (`SystemConfig::new().label(StateMachineSystem::LABEL).after(AnimationSystem::LABEL)`).
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::animation_state_machine";
}

impl System for StateMachineSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let entities: Vec<Entity> = world
            .query::<AnimationStateMachine>()
            .map(|(e, _)| e)
            .collect();

        for entity in entities {
            // `anim_finished` is true only when the player is NOT crossfading and the
            // current state's clip has reached its last frame.  During a crossfade,
            // `current_clip` is still the FROM clip; evaluating `is_finished()` there
            // would fire `AnimationEnd` for the FROM state and immediately exit the new
            // state on the very first frame it was entered.
            let current_state_clip = world
                .get_mut::<AnimationStateMachine>(entity)
                .and_then(|sm| sm.states.get(&sm.current).map(|s| s.clip_index));
            let anim_finished = match current_state_clip {
                Some(clip_index) => world
                    .get_mut::<AnimationPlayer>(entity)
                    .map(|p| p.is_clip_finished(clip_index))
                    .unwrap_or(false),
                None => false,
            };

            let transition = world
                .get_mut::<AnimationStateMachine>(entity)
                .and_then(|sm| sm.evaluate(anim_finished));

            if let Some((next_state, clip_index, crossfade_dur)) = transition {
                if let Some(sm) = world.get_mut::<AnimationStateMachine>(entity) {
                    sm.current = next_state;
                    sm.consume_triggers();
                }
                if let Some(player) = world.get_mut::<AnimationPlayer>(entity) {
                    if crossfade_dur > 0.0 {
                        player.play_with_crossfade(clip_index, crossfade_dur);
                    } else {
                        player.play(clip_index);
                    }
                }
            } else {
                // Even without a transition, triggers are only valid for one frame and must be consumed.
                if let Some(sm) = world.get_mut::<AnimationStateMachine>(entity) {
                    sm.consume_triggers();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
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

        let mut anim = AnimationSystem;
        let mut stm = StateMachineSystem;
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

        let mut anim = AnimationSystem;
        let mut stm = StateMachineSystem;
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

        let mut anim = AnimationSystem;
        let mut stm = StateMachineSystem;

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

        let mut anim = AnimationSystem;
        let mut stm = StateMachineSystem;

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

        let mut anim = AnimationSystem;
        let mut stm = StateMachineSystem;
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

        let mut anim = AnimationSystem;
        let mut stm = StateMachineSystem;

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

        let mut anim = AnimationSystem;
        let mut stm = StateMachineSystem;

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
}
