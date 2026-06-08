use std::collections::HashMap;
use std::hash::Hash;

use winit::keyboard::KeyCode;

use crate::input::state::InputState;

/// Resource that maps game actions to `KeyCode` bindings.
///
/// ```rust,no_run
/// use engine::InputMap;
/// use winit::keyboard::KeyCode;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash)]
/// enum Action { Jump, Left, Right }
///
/// let mut map = InputMap::new();
/// map.bind(Action::Jump, KeyCode::Space);
/// ```
pub struct InputMap<A: Eq + Hash> {
    bindings: HashMap<A, KeyCode>,
}

impl<A: Eq + Hash> InputMap<A> {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn bind(&mut self, action: A, key: KeyCode) {
        self.bindings.insert(action, key);
    }

    pub fn unbind(&mut self, action: &A) {
        self.bindings.remove(action);
    }

    pub fn key_for(&self, action: &A) -> Option<KeyCode> {
        self.bindings.get(action).copied()
    }

    pub fn is_pressed(&self, action: &A, input: &InputState) -> bool {
        self.key_for(action)
            .map(|k| input.is_pressed(k))
            .unwrap_or(false)
    }

    pub fn just_pressed(&self, action: &A, input: &InputState) -> bool {
        self.key_for(action)
            .map(|k| input.just_pressed(k))
            .unwrap_or(false)
    }

    pub fn just_released(&self, action: &A, input: &InputState) -> bool {
        self.key_for(action)
            .map(|k| input.just_released(k))
            .unwrap_or(false)
    }
}

impl<A: Eq + Hash> Default for InputMap<A> {
    fn default() -> Self {
        Self::new()
    }
}
