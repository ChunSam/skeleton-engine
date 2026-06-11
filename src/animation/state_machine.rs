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
            let anim_finished = world
                .get_mut::<AnimationPlayer>(entity)
                .map(|p| p.is_finished())
                .unwrap_or(false);

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
}
