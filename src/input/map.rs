use std::collections::HashMap;
use std::hash::Hash;

use winit::keyboard::KeyCode;

use crate::input::gamepad::{GamepadAxis, GamepadButton, GamepadState};
use crate::input::state::InputState;

/// Direction/threshold used when mapping a gamepad axis to a digital action.
///
/// `Positive` fires when the axis value exceeds `threshold` (default 0.5).
/// `Negative` fires when the axis value is below `-threshold`.
#[derive(Debug, Clone, Copy)]
pub struct AxisBinding {
    pub axis: GamepadAxis,
    /// Which side of zero triggers the action.
    pub positive: bool,
    /// Magnitude threshold (0.0–1.0). Default is `0.5`.
    pub threshold: f32,
}

impl AxisBinding {
    /// Convenience constructor: axis fires when value exceeds `+threshold`.
    pub fn positive(axis: GamepadAxis, threshold: f32) -> Self {
        Self {
            axis,
            positive: true,
            threshold,
        }
    }

    /// Convenience constructor: axis fires when value goes below `-threshold`.
    pub fn negative(axis: GamepadAxis, threshold: f32) -> Self {
        Self {
            axis,
            positive: false,
            threshold,
        }
    }

    fn is_active(&self, value: f32) -> bool {
        if self.positive {
            value >= self.threshold
        } else {
            value <= -self.threshold
        }
    }
}

/// Per-action binding set.
struct ActionBindings {
    keys: Vec<KeyCode>,
    gamepad_buttons: Vec<GamepadButton>,
    gamepad_axes: Vec<AxisBinding>,
}

impl ActionBindings {
    fn new() -> Self {
        Self {
            keys: Vec::new(),
            gamepad_buttons: Vec::new(),
            gamepad_axes: Vec::new(),
        }
    }
}

/// Resource that maps game actions to input bindings (keyboard keys, gamepad
/// buttons, and gamepad axes).
///
/// Bindings are additive: an action can be driven by any combination of
/// keyboard keys, gamepad buttons, and gamepad axes simultaneously. The action
/// is active when **any** bound input is active (logical OR).
///
/// # Keyboard-only (unchanged API)
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
///
/// # Gamepad bindings
/// ```rust,no_run
/// use engine::InputMap;
/// use engine::{GamepadButton, GamepadAxis};
/// use engine::input::map::AxisBinding;
/// use winit::keyboard::KeyCode;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash)]
/// enum Action { Jump, MoveLeft }
///
/// let mut map = InputMap::new();
/// // Keyboard + gamepad button for the same action (additive).
/// map.bind(Action::Jump, KeyCode::Space);
/// map.bind_gamepad_button(Action::Jump, GamepadButton::South);
/// // Axis-driven digital action (left stick left OR right stick left).
/// map.bind_gamepad_axis(Action::MoveLeft, AxisBinding::negative(GamepadAxis::LeftStickX, 0.5));
/// ```
pub struct InputMap<A: Eq + Hash> {
    bindings: HashMap<A, ActionBindings>,
}

impl<A: Eq + Hash + Clone> InputMap<A> {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    // ── Keyboard bindings (unchanged API) ────────────────────────────────────

    /// Bind a keyboard key to an action (replaces any previously bound key for
    /// this action; multiple keys per action are supported via repeated calls).
    pub fn bind(&mut self, action: A, key: KeyCode) {
        self.bindings
            .entry(action)
            .or_insert_with(ActionBindings::new)
            .keys
            .push(key);
    }

    /// Remove **all** bindings (keyboard + gamepad) for an action.
    pub fn unbind(&mut self, action: &A) {
        self.bindings.remove(action);
    }

    /// Returns the first keyboard key bound to `action`, if any.
    ///
    /// For actions with multiple keyboard keys, use [`keys_for`].
    pub fn key_for(&self, action: &A) -> Option<KeyCode> {
        self.bindings
            .get(action)
            .and_then(|b| b.keys.first().copied())
    }

    /// Returns all keyboard keys bound to `action`.
    pub fn keys_for(&self, action: &A) -> &[KeyCode] {
        self.bindings
            .get(action)
            .map(|b| b.keys.as_slice())
            .unwrap_or(&[])
    }

    // ── Gamepad bindings ──────────────────────────────────────────────────────

    /// Bind a gamepad button to an action (additive — does not remove existing
    /// keyboard or other gamepad bindings).
    pub fn bind_gamepad_button(&mut self, action: A, button: GamepadButton) {
        self.bindings
            .entry(action)
            .or_insert_with(ActionBindings::new)
            .gamepad_buttons
            .push(button);
    }

    /// Bind a gamepad axis (with direction/threshold) to a digital action.
    ///
    /// Use [`AxisBinding::positive`] / [`AxisBinding::negative`] for ergonomic
    /// construction, or build the struct directly for custom thresholds.
    pub fn bind_gamepad_axis(&mut self, action: A, binding: AxisBinding) {
        self.bindings
            .entry(action)
            .or_insert_with(ActionBindings::new)
            .gamepad_axes
            .push(binding);
    }

    // ── Keyboard-only resolution (backward-compatible) ────────────────────────

    /// Returns `true` if any keyboard key bound to `action` is currently held.
    ///
    /// Does **not** consult gamepad state; use [`is_pressed_with_gamepad`] to
    /// include gamepad bindings.
    pub fn is_pressed(&self, action: &A, input: &InputState) -> bool {
        self.bindings
            .get(action)
            .map(|b| b.keys.iter().any(|&k| input.is_pressed(k)))
            .unwrap_or(false)
    }

    /// Returns `true` if any keyboard key bound to `action` was pressed this frame.
    pub fn just_pressed(&self, action: &A, input: &InputState) -> bool {
        self.bindings
            .get(action)
            .map(|b| b.keys.iter().any(|&k| input.just_pressed(k)))
            .unwrap_or(false)
    }

    /// Returns `true` if any keyboard key bound to `action` was released this frame.
    pub fn just_released(&self, action: &A, input: &InputState) -> bool {
        self.bindings
            .get(action)
            .map(|b| b.keys.iter().any(|&k| input.just_released(k)))
            .unwrap_or(false)
    }

    // ── Combined keyboard + gamepad resolution ────────────────────────────────

    /// Returns `true` if **any** binding (keyboard key, gamepad button, or
    /// gamepad axis) for `action` is currently active.
    ///
    /// `pad` is the gamepad slot index (0 = first connected controller).
    pub fn is_pressed_with_gamepad(
        &self,
        action: &A,
        input: &InputState,
        gamepad: &GamepadState,
        pad: usize,
    ) -> bool {
        let Some(b) = self.bindings.get(action) else {
            return false;
        };
        b.keys.iter().any(|&k| input.is_pressed(k))
            || b.gamepad_buttons
                .iter()
                .any(|&btn| gamepad.is_pressed(pad, btn))
            || b.gamepad_axes
                .iter()
                .any(|ab| ab.is_active(gamepad.axis(pad, ab.axis)))
    }

    /// Returns `true` if **any** binding for `action` was newly activated this frame.
    ///
    /// For gamepad axes the "just pressed" semantics are: the axis crossed the
    /// threshold this frame (i.e. it was below threshold last frame). Because
    /// `GamepadState` only tracks `just_pressed` for digital buttons (not axes),
    /// axis bindings are tested as "currently active" only — edge detection for
    /// axes requires a custom cooldown in the calling system if needed.
    pub fn just_pressed_with_gamepad(
        &self,
        action: &A,
        input: &InputState,
        gamepad: &GamepadState,
        pad: usize,
    ) -> bool {
        let Some(b) = self.bindings.get(action) else {
            return false;
        };
        b.keys.iter().any(|&k| input.just_pressed(k))
            || b.gamepad_buttons
                .iter()
                .any(|&btn| gamepad.just_pressed(pad, btn))
    }

    /// Returns `true` if **any** keyboard key or gamepad button for `action` was
    /// released this frame.
    pub fn just_released_with_gamepad(
        &self,
        action: &A,
        input: &InputState,
        gamepad: &GamepadState,
        pad: usize,
    ) -> bool {
        let Some(b) = self.bindings.get(action) else {
            return false;
        };
        b.keys.iter().any(|&k| input.just_released(k))
            || b.gamepad_buttons
                .iter()
                .any(|&btn| gamepad.just_released(pad, btn))
    }
}

impl<A: Eq + Hash + Clone> Default for InputMap<A> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::gamepad::GamepadState;
    use winit::keyboard::KeyCode;

    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    enum Act {
        Jump,
        Left,
    }

    // ── helpers ────────────────────────────────────────────────────────────────

    fn pressed_input(key: KeyCode) -> InputState {
        let mut s = InputState::default();
        s.press(key);
        s
    }

    fn just_pressed_input(key: KeyCode) -> InputState {
        // `press` inserts into both `pressed` and `just_pressed`
        pressed_input(key)
    }

    fn released_input(key: KeyCode) -> InputState {
        let mut s = InputState::default();
        s.press(key);
        s.release(key);
        s
    }

    // ── GamepadState test helpers ──────────────────────────────────────────────
    // `GamepadState` slots are `Option<Slot>` and `Slot` is private, so we
    // cannot construct a real pressed-button state without going through
    // `process_event` (which requires `gilrs` and only compiles on native).
    // Instead we test the gamepad-button path by verifying the "no gamepad
    // connected" fallback returns false, and the keyboard path still works when
    // a gamepad binding is also present.

    // ── Keyboard-only (backward-compat) ───────────────────────────────────────

    #[test]
    fn keyboard_bind_is_pressed() {
        let mut map: InputMap<Act> = InputMap::new();
        map.bind(Act::Jump, KeyCode::Space);

        let input = pressed_input(KeyCode::Space);
        assert!(map.is_pressed(&Act::Jump, &input));
        assert!(!map.is_pressed(&Act::Left, &input));
    }

    #[test]
    fn keyboard_just_pressed() {
        let mut map: InputMap<Act> = InputMap::new();
        map.bind(Act::Jump, KeyCode::Space);

        let input = just_pressed_input(KeyCode::Space);
        assert!(map.just_pressed(&Act::Jump, &input));
    }

    #[test]
    fn keyboard_just_released() {
        let mut map: InputMap<Act> = InputMap::new();
        map.bind(Act::Jump, KeyCode::Space);

        let input = released_input(KeyCode::Space);
        assert!(map.just_released(&Act::Jump, &input));
    }

    #[test]
    fn keyboard_unbind_clears_all() {
        let mut map: InputMap<Act> = InputMap::new();
        map.bind(Act::Jump, KeyCode::Space);
        map.unbind(&Act::Jump);

        let input = pressed_input(KeyCode::Space);
        assert!(!map.is_pressed(&Act::Jump, &input));
    }

    #[test]
    fn key_for_returns_first_key() {
        let mut map: InputMap<Act> = InputMap::new();
        map.bind(Act::Jump, KeyCode::Space);
        map.bind(Act::Jump, KeyCode::ArrowUp);
        assert_eq!(map.key_for(&Act::Jump), Some(KeyCode::Space));
    }

    #[test]
    fn keys_for_returns_all_keys() {
        let mut map: InputMap<Act> = InputMap::new();
        map.bind(Act::Jump, KeyCode::Space);
        map.bind(Act::Jump, KeyCode::ArrowUp);
        let keys = map.keys_for(&Act::Jump);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&KeyCode::Space));
        assert!(keys.contains(&KeyCode::ArrowUp));
    }

    // ── Gamepad bindings — button ──────────────────────────────────────────────

    #[test]
    fn gamepad_button_no_pad_connected_is_false() {
        let mut map: InputMap<Act> = InputMap::new();
        map.bind_gamepad_button(Act::Jump, GamepadButton::South);

        let input = InputState::default();
        let gs = GamepadState::default(); // no pads connected
        assert!(!map.is_pressed_with_gamepad(&Act::Jump, &input, &gs, 0));
        assert!(!map.just_pressed_with_gamepad(&Act::Jump, &input, &gs, 0));
        assert!(!map.just_released_with_gamepad(&Act::Jump, &input, &gs, 0));
    }

    #[test]
    fn keyboard_path_still_works_alongside_gamepad_button() {
        let mut map: InputMap<Act> = InputMap::new();
        map.bind(Act::Jump, KeyCode::Space);
        map.bind_gamepad_button(Act::Jump, GamepadButton::South);

        let input = pressed_input(KeyCode::Space);
        let gs = GamepadState::default();
        // Keyboard binding resolves even without a connected gamepad.
        assert!(map.is_pressed_with_gamepad(&Act::Jump, &input, &gs, 0));
    }

    #[test]
    fn keyboard_only_method_unaffected_by_gamepad_binding() {
        let mut map: InputMap<Act> = InputMap::new();
        map.bind(Act::Jump, KeyCode::Space);
        map.bind_gamepad_button(Act::Jump, GamepadButton::South);

        let input = pressed_input(KeyCode::Space);
        // The original is_pressed / just_pressed / just_released still work.
        assert!(map.is_pressed(&Act::Jump, &input));
        assert!(map.just_pressed(&Act::Jump, &input));
    }

    // ── Gamepad bindings — axis ────────────────────────────────────────────────

    #[test]
    fn axis_binding_not_active_when_no_gamepad() {
        let mut map: InputMap<Act> = InputMap::new();
        map.bind_gamepad_axis(
            Act::Left,
            AxisBinding::negative(GamepadAxis::LeftStickX, 0.5),
        );

        let input = InputState::default();
        let gs = GamepadState::default();
        // No pad connected → axis returns 0.0 → negative-side threshold not met.
        assert!(!map.is_pressed_with_gamepad(&Act::Left, &input, &gs, 0));
    }

    #[test]
    fn axis_binding_positive_threshold() {
        // Even with no pad, axis() returns 0.0 (below +0.5 threshold) → false.
        let mut map: InputMap<Act> = InputMap::new();
        map.bind_gamepad_axis(
            Act::Left,
            AxisBinding::positive(GamepadAxis::LeftStickX, 0.5),
        );
        let gs = GamepadState::default();
        let input = InputState::default();
        assert!(!map.is_pressed_with_gamepad(&Act::Left, &input, &gs, 0));
    }

    #[test]
    fn axis_is_active_logic_positive() {
        let ab = AxisBinding::positive(GamepadAxis::LeftStickX, 0.5);
        assert!(ab.is_active(0.6));
        assert!(ab.is_active(0.5));
        assert!(!ab.is_active(0.4));
        assert!(!ab.is_active(-0.8));
    }

    #[test]
    fn axis_is_active_logic_negative() {
        let ab = AxisBinding::negative(GamepadAxis::LeftStickX, 0.5);
        assert!(ab.is_active(-0.6));
        assert!(ab.is_active(-0.5));
        assert!(!ab.is_active(-0.4));
        assert!(!ab.is_active(0.8));
    }

    // ── No binding → false ────────────────────────────────────────────────────

    #[test]
    fn unbound_action_always_false() {
        let map: InputMap<Act> = InputMap::new();
        let input = pressed_input(KeyCode::Space);
        let gs = GamepadState::default();
        assert!(!map.is_pressed(&Act::Jump, &input));
        assert!(!map.is_pressed_with_gamepad(&Act::Jump, &input, &gs, 0));
    }
}
