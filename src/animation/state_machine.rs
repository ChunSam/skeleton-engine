use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::animation::player::AnimationPlayer;
use crate::ecs::{Entity, System, World};

// ─── Parameters ───────────────────────────────────────────────────────────────

/// Parameter value held by the state machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnimParam {
    Bool(bool),
    Float(f32),
    /// Trigger that is valid for one frame only. Activated via `fire_trigger()` and consumed each frame.
    Trigger(bool),
}

// ─── Transition conditions ────────────────────────────────────────────────────

/// A single condition that must be satisfied for a state transition to occur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
/// app.add_system(AnimationSystem::new());     // advance frames
/// app.add_system(StateMachineSystem::new());  // evaluate transitions
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        // Warn when the target state does not (yet) exist — the transition is still
        // pushed so that states registered later are handled, but evaluate() will
        // silently skip any transition whose `to` state is missing at fire time.
        if !self.states.contains_key(&to) {
            log::warn!(
                "AnimationStateMachine::add_transition: target state \"{to}\" is not registered \
                 (from \"{from}\"); the transition is recorded but will be a dead edge until \
                 add_state(\"{to}\", ...) is called"
            );
        }
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
    ///
    /// When the key already exists the value is updated in place without any
    /// heap allocation. The first call for a new key allocates the `String` key
    /// via `name.into()`. Accepts `&str`, `String`, `&String`, or any type that
    /// implements `Into<String> + AsRef<str>`.
    ///
    /// **v6 breaking note:** the bound changed from `impl Into<String>` to
    /// `impl Into<String> + AsRef<str>`. All common string types satisfy both.
    pub fn set_bool(&mut self, name: impl Into<String> + AsRef<str>, value: bool) {
        if let Some(p) = self.params.get_mut(name.as_ref()) {
            *p = AnimParam::Bool(value);
        } else {
            self.params.insert(name.into(), AnimParam::Bool(value));
        }
    }

    /// Reads a bool parameter. Returns `None` if missing or the wrong type.
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        match self.params.get(name) {
            Some(AnimParam::Bool(v)) => Some(*v),
            _ => None,
        }
    }

    /// Sets or updates a float parameter.
    ///
    /// When the key already exists the value is updated in place without any
    /// heap allocation. The first call for a new key allocates the `String` key
    /// via `name.into()`. Accepts `&str`, `String`, `&String`, or any type that
    /// implements `Into<String> + AsRef<str>`.
    ///
    /// **v6 breaking note:** the bound changed from `impl Into<String>` to
    /// `impl Into<String> + AsRef<str>`. All common string types satisfy both.
    pub fn set_float(&mut self, name: impl Into<String> + AsRef<str>, value: f32) {
        if let Some(p) = self.params.get_mut(name.as_ref()) {
            *p = AnimParam::Float(value);
        } else {
            self.params.insert(name.into(), AnimParam::Float(value));
        }
    }

    /// Reads a float parameter. Returns `None` if missing or the wrong type.
    pub fn get_float(&self, name: &str) -> Option<f32> {
        match self.params.get(name) {
            Some(AnimParam::Float(v)) => Some(*v),
            _ => None,
        }
    }

    /// Registers a trigger parameter (initial value: false).
    ///
    /// Uses `entry()` so only the first call for a given key allocates.
    /// Accepts `&str`, `String`, `&String`, or any type that implements
    /// `Into<String> + AsRef<str>`.
    ///
    /// **v6 breaking note:** the bound changed from `impl Into<String>` to
    /// `impl Into<String> + AsRef<str>`. All common string types satisfy both.
    pub fn add_trigger(&mut self, name: impl Into<String> + AsRef<str>) {
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

    // ── Inspection (editor / tooling) ──────────────────────────────────────────

    /// All state names, sorted (stable order for an editor/graph view).
    pub fn state_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.states.keys().cloned().collect();
        names.sort();
        names
    }

    /// The state node for `name`, if it exists (read its `clip_index` / `transitions`).
    pub fn state(&self, name: &str) -> Option<&AnimState> {
        self.states.get(name)
    }

    /// Number of states.
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// All parameter names, sorted.
    pub fn param_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.params.keys().cloned().collect();
        names.sort();
        names
    }

    /// The parameter value for `name`, if it exists.
    pub fn param(&self, name: &str) -> Option<&AnimParam> {
        self.params.get(name)
    }

    // ── Editing (editor / tooling) ─────────────────────────────────────────────

    /// Force the active state to `name` (e.g. an editor "jump to state"). No-op (returns `false`)
    /// if no such state exists.
    pub fn set_current_state(&mut self, name: &str) -> bool {
        if self.states.contains_key(name) {
            self.current = name.to_string();
            true
        } else {
            false
        }
    }

    /// Change a state's `clip_index`. No-op (returns `false`) if the state doesn't exist.
    pub fn set_state_clip(&mut self, name: &str, clip_index: usize) -> bool {
        if let Some(state) = self.states.get_mut(name) {
            state.clip_index = clip_index;
            true
        } else {
            false
        }
    }

    /// Remove a state and every transition pointing **to** it. Refuses (returns `false`) to remove
    /// the currently-active state, the last remaining state, or a non-existent state.
    pub fn remove_state(&mut self, name: &str) -> bool {
        if name == self.current || self.states.len() <= 1 || !self.states.contains_key(name) {
            return false;
        }
        self.states.remove(name);
        for state in self.states.values_mut() {
            state.transitions.retain(|t| t.to != name);
        }
        true
    }

    /// Remove the transition at `index` from state `from`. Returns `false` if `from` or the index
    /// is out of range.
    pub fn remove_transition(&mut self, from: &str, index: usize) -> bool {
        if let Some(state) = self.states.get_mut(from) {
            if index < state.transitions.len() {
                state.transitions.remove(index);
                return true;
            }
        }
        false
    }

    /// Replace the conditions of the transition at `index` from state `from`. Returns `false` if
    /// `from` is missing or `index` is out of range.
    pub fn set_transition_conditions(
        &mut self,
        from: &str,
        index: usize,
        conditions: Vec<TransitionCond>,
    ) -> bool {
        if let Some(state) = self.states.get_mut(from) {
            if index < state.transitions.len() {
                state.transitions[index].conditions = conditions;
                return true;
            }
        }
        false
    }

    /// Set the crossfade duration of the transition at `index` from state `from`. Returns `false`
    /// if `from` is missing or `index` is out of range.
    pub fn set_transition_crossfade(&mut self, from: &str, index: usize, seconds: f32) -> bool {
        if let Some(state) = self.states.get_mut(from) {
            if index < state.transitions.len() {
                state.transitions[index].crossfade_duration = seconds;
                return true;
            }
        }
        false
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
///
/// The `scratch` buffer is reused across frames to avoid a per-frame allocation.
/// Create with `StateMachineSystem::new()` or `StateMachineSystem::default()`.
#[derive(Default)]
pub struct StateMachineSystem {
    scratch: Vec<Entity>,
}

impl StateMachineSystem {
    /// Creates a new `StateMachineSystem` with a pre-allocated scratch buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedule label. Recommended order: **after** `AnimationSystem::LABEL`
    /// (`SystemConfig::new().label(StateMachineSystem::LABEL).after(AnimationSystem::LABEL)`).
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::animation_state_machine";
}

impl System for StateMachineSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        self.scratch.clear();
        self.scratch
            .extend(world.query::<AnimationStateMachine>().map(|(e, _)| e));

        for i in 0..self.scratch.len() {
            let entity = self.scratch[i];
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
mod tests;
