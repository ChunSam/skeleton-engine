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

    /// Registers a transition edge from `from` to `to`.
    /// Does nothing if the `from` state does not exist.
    pub fn add_transition(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        conditions: Vec<TransitionCond>,
    ) -> &mut Self {
        let from = from.into();
        let to = to.into();
        if let Some(state) = self.states.get_mut(&from) {
            state.transitions.push(AnimTransition { to, conditions });
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

    /// Finds the first satisfied transition in the current state and returns `(target state, clip index)`.
    fn evaluate(&self, anim_finished: bool) -> Option<(String, usize)> {
        let state = self.states.get(&self.current)?;
        for transition in &state.transitions {
            if transition
                .conditions
                .iter()
                .all(|c| self.check_condition(c, anim_finished))
            {
                let next_clip = self.states.get(&transition.to)?.clip_index;
                return Some((transition.to.clone(), next_clip));
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

            if let Some((next_state, clip_index)) = transition {
                if let Some(sm) = world.get_mut::<AnimationStateMachine>(entity) {
                    sm.current = next_state;
                    sm.consume_triggers();
                }
                if let Some(player) = world.get_mut::<AnimationPlayer>(entity) {
                    player.play(clip_index);
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
