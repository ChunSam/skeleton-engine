use glam::Vec2;
use std::collections::HashSet;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

/// ECS resource holding keyboard and mouse state.
///
/// Insert into the World and access from systems via `world.resource::<InputState>()`.
pub struct InputState {
    // ── Keyboard ─────────────────────────────────────────────────────────────
    pressed: HashSet<KeyCode>,
    just_pressed: HashSet<KeyCode>,
    just_released: HashSet<KeyCode>,

    // ── Mouse ─────────────────────────────────────────────────────────────────
    cursor: Vec2,
    mouse_pressed: [bool; 3],
    mouse_just_pressed: [bool; 3],
    mouse_just_released: [bool; 3],
    /// Cursor position at the exact *moment* each button was pressed/released.
    /// Click hit-tests should use this value rather than the current cursor (which
    /// is updated by move events), so that a press and a subsequent move in the
    /// same frame don't shift the hit-test to the wrong position.
    mouse_press_cursor: [Vec2; 3],
    mouse_release_cursor: [Vec2; 3],
    scroll: f32,
    text_input_chars: Vec<char>,
    ime_preedit: String,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
            cursor: Vec2::ZERO,
            mouse_pressed: [false; 3],
            mouse_just_pressed: [false; 3],
            mouse_just_released: [false; 3],
            mouse_press_cursor: [Vec2::ZERO; 3],
            mouse_release_cursor: [Vec2::ZERO; 3],
            scroll: 0.0,
            text_input_chars: Vec::new(),
            ime_preedit: String::new(),
        }
    }
}

impl InputState {
    // ── Keyboard public methods ───────────────────────────────────────────────

    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed.contains(&key)
    }

    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    pub fn just_released(&self, key: KeyCode) -> bool {
        self.just_released.contains(&key)
    }

    // ── Mouse public methods ──────────────────────────────────────────────────

    pub fn cursor(&self) -> Vec2 {
        self.cursor
    }

    pub fn is_mouse_pressed(&self, btn: MouseButton) -> bool {
        mouse_button_index(btn).is_some_and(|i| self.mouse_pressed[i])
    }

    pub fn mouse_just_pressed(&self, btn: MouseButton) -> bool {
        mouse_button_index(btn).is_some_and(|i| self.mouse_just_pressed[i])
    }

    pub fn mouse_just_released(&self, btn: MouseButton) -> bool {
        mouse_button_index(btn).is_some_and(|i| self.mouse_just_released[i])
    }

    /// Cursor position at the last moment the button was *pressed*. For click/drag-start hit-testing.
    pub fn mouse_press_cursor(&self, btn: MouseButton) -> Vec2 {
        mouse_button_index(btn).map_or(self.cursor, |i| self.mouse_press_cursor[i])
    }

    /// Cursor position at the last moment the button was *released*. For confirming click hit-tests.
    pub fn mouse_release_cursor(&self, btn: MouseButton) -> Vec2 {
        mouse_button_index(btn).map_or(self.cursor, |i| self.mouse_release_cursor[i])
    }

    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    /// Returns a slice of characters typed this frame.
    ///
    /// `'\x08'` = Backspace, `'\n'` = Enter, everything else = regular character.
    pub fn text_chars(&self) -> &[char] {
        &self.text_input_chars
    }

    pub fn ime_preedit(&self) -> &str {
        &self.ime_preedit
    }

    // ── Internal update (called only from App) ────────────────────────────────

    pub(crate) fn press(&mut self, key: KeyCode) {
        if self.pressed.insert(key) {
            self.just_pressed.insert(key);
        }
    }

    pub(crate) fn release(&mut self, key: KeyCode) {
        // Only record a just_released pulse if the key was actually pressed.
        // This prevents spurious just_released events when the editor keyboard
        // gate flips and the OS delivers a synthetic release for a key that was
        // never seen as pressed by the engine.
        if self.pressed.remove(&key) {
            self.just_released.insert(key);
        }
    }

    pub(crate) fn set_cursor(&mut self, pos: Vec2) {
        self.cursor = pos;
    }

    pub(crate) fn press_mouse(&mut self, btn: MouseButton) {
        if let Some(i) = mouse_button_index(btn) {
            // Current cursor = position where this press occurred (CursorMoved is processed first).
            self.mouse_press_cursor[i] = self.cursor;
            if !self.mouse_pressed[i] {
                self.mouse_pressed[i] = true;
                self.mouse_just_pressed[i] = true;
            }
        }
    }

    pub(crate) fn release_mouse(&mut self, btn: MouseButton) {
        if let Some(i) = mouse_button_index(btn) {
            // Only fire mouse_just_released when the button was actually pressed.
            // Matches the keyboard guard in release() — prevents spurious events
            // from OS release messages delivered after focus is lost.
            if self.mouse_pressed[i] {
                self.mouse_release_cursor[i] = self.cursor;
                self.mouse_pressed[i] = false;
                self.mouse_just_released[i] = true;
            }
        }
    }

    /// Clear all held-down state without generating any just_released pulses.
    ///
    /// Called by the window focus-loss handler when the OS no longer delivers
    /// individual key-up / button-up events. Flooding `just_released` for every
    /// held key would trigger unintended one-shot reactions in game systems.
    pub(crate) fn release_all(&mut self) {
        self.pressed.clear();
        self.just_pressed.clear();
        self.mouse_pressed = [false; 3];
        self.mouse_just_pressed = [false; 3];
        // Intentionally do NOT populate just_released / mouse_just_released here.
    }

    pub(crate) fn add_scroll(&mut self, delta: f32) {
        self.scroll += delta;
    }

    pub(crate) fn push_char(&mut self, c: char) {
        self.text_input_chars.push(c);
    }

    pub(crate) fn push_text(&mut self, s: &str) {
        self.text_input_chars.extend(s.chars());
    }

    pub(crate) fn push_backspace(&mut self) {
        self.text_input_chars.push('\x08');
    }

    pub(crate) fn push_enter(&mut self) {
        self.text_input_chars.push('\n');
    }

    pub(crate) fn set_ime_preedit(&mut self, preedit: String) {
        self.ime_preedit = preedit;
    }

    pub(crate) fn clear_ime_preedit(&mut self) {
        self.ime_preedit.clear();
    }

    pub(crate) fn flush(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.mouse_just_pressed = [false; 3];
        self.mouse_just_released = [false; 3];
        self.scroll = 0.0;
        self.text_input_chars.clear();
    }
}

fn mouse_button_index(btn: MouseButton) -> Option<usize> {
    match btn {
        MouseButton::Left => Some(0),
        MouseButton::Right => Some(1),
        MouseButton::Middle => Some(2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── release() guard ───────────────────────────────────────────────────────

    #[test]
    fn release_without_prior_press_no_just_released() {
        let mut input = InputState::default();
        // Release a key that was never pressed — should NOT produce just_released.
        input.release(KeyCode::Space);
        assert!(!input.just_released(KeyCode::Space));
    }

    #[test]
    fn release_after_press_produces_just_released() {
        let mut input = InputState::default();
        input.press(KeyCode::Space);
        input.release(KeyCode::Space);
        assert!(input.just_released(KeyCode::Space));
        assert!(!input.is_pressed(KeyCode::Space));
    }

    // ── release_mouse() guard ─────────────────────────────────────────────────

    #[test]
    fn release_mouse_without_prior_press_no_just_released() {
        let mut input = InputState::default();
        // Release without press — should NOT produce mouse_just_released.
        input.release_mouse(MouseButton::Left);
        assert!(!input.mouse_just_released(MouseButton::Left));
    }

    #[test]
    fn release_mouse_after_press_produces_just_released() {
        let mut input = InputState::default();
        input.press_mouse(MouseButton::Left);
        input.release_mouse(MouseButton::Left);
        assert!(input.mouse_just_released(MouseButton::Left));
        assert!(!input.is_mouse_pressed(MouseButton::Left));
    }

    // ── release_all ───────────────────────────────────────────────────────────

    #[test]
    fn release_all_clears_pressed_no_just_released() {
        let mut input = InputState::default();
        input.press(KeyCode::Space);
        input.press(KeyCode::ArrowLeft);
        input.press_mouse(MouseButton::Left);
        input.release_all();

        assert!(!input.is_pressed(KeyCode::Space));
        assert!(!input.is_pressed(KeyCode::ArrowLeft));
        assert!(!input.is_mouse_pressed(MouseButton::Left));
        // No spurious just_released pulses.
        assert!(!input.just_released(KeyCode::Space));
        assert!(!input.just_released(KeyCode::ArrowLeft));
        assert!(!input.mouse_just_released(MouseButton::Left));
    }

    #[test]
    fn release_all_also_clears_just_pressed() {
        let mut input = InputState::default();
        input.press(KeyCode::Space);
        // just_pressed is populated by press(); release_all should clear it.
        input.release_all();
        assert!(!input.just_pressed(KeyCode::Space));
    }

    #[test]
    fn mouse_press_sets_state() {
        let mut input = InputState::default();
        input.press_mouse(MouseButton::Left);
        assert!(input.is_mouse_pressed(MouseButton::Left));
        assert!(input.mouse_just_pressed(MouseButton::Left));
        input.flush();
        assert!(input.is_mouse_pressed(MouseButton::Left));
        assert!(!input.mouse_just_pressed(MouseButton::Left));
    }

    #[test]
    fn mouse_release_clears_pressed() {
        let mut input = InputState::default();
        input.press_mouse(MouseButton::Left);
        input.release_mouse(MouseButton::Left);
        assert!(!input.is_mouse_pressed(MouseButton::Left));
        assert!(input.mouse_just_released(MouseButton::Left));
        input.flush();
        assert!(!input.mouse_just_released(MouseButton::Left));
    }

    #[test]
    fn mouse_press_twice_no_repeat() {
        let mut input = InputState::default();
        input.press_mouse(MouseButton::Left);
        input.press_mouse(MouseButton::Left);
        assert!(input.is_mouse_pressed(MouseButton::Left));
        input.flush();
        input.press_mouse(MouseButton::Left);
        assert!(!input.mouse_just_pressed(MouseButton::Left));
    }

    #[test]
    fn cursor_updates() {
        let mut input = InputState::default();
        input.set_cursor(Vec2::new(123.0, 45.0));
        assert_eq!(input.cursor(), Vec2::new(123.0, 45.0));
    }

    #[test]
    fn scroll_accumulates_and_resets() {
        let mut input = InputState::default();
        input.add_scroll(1.0);
        input.add_scroll(2.5);
        assert!((input.scroll() - 3.5).abs() < f32::EPSILON);
        input.flush();
        assert_eq!(input.scroll(), 0.0);
    }

    #[test]
    fn ime_preedit_persists_until_cleared() {
        let mut input = InputState::default();
        input.set_ime_preedit("한".to_string());
        input.push_text("글");
        input.flush();
        assert_eq!(input.ime_preedit(), "한");
        assert!(input.text_chars().is_empty());
        input.clear_ime_preedit();
        assert_eq!(input.ime_preedit(), "");
    }
}
