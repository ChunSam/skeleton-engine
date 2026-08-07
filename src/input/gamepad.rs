use std::collections::{HashMap, HashSet};

/// Gamepad button identifier.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    South,        // A (Xbox) / Cross (PS)
    East,         // B (Xbox) / Circle (PS)
    North,        // Y (Xbox) / Triangle (PS)
    West,         // X (Xbox) / Square (PS)
    LeftBumper,   // LB / L1
    RightBumper,  // RB / R1
    LeftTrigger,  // LT / L2 (digital)
    RightTrigger, // RT / R2 (digital)
    Select,
    Start,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// Gamepad axis identifier.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,  // L2/LT analog (0.0 ~ 1.0)
    RightTrigger, // R2/RT analog (0.0 ~ 1.0)
    DPadX,
    DPadY,
}

struct Slot {
    pressed: HashSet<GamepadButton>,
    just_pressed: HashSet<GamepadButton>,
    just_released: HashSet<GamepadButton>,
    axes: HashMap<GamepadAxis, f32>,
}

impl Slot {
    #[cfg(not(target_arch = "wasm32"))]
    fn new() -> Self {
        Self {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
            axes: HashMap::new(),
        }
    }

    fn flush(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// Folds a **full held-button snapshot** into this slot, *accumulating* the `just_*` edges.
    ///
    /// The edge policy is the whole point, and it must match the event-driven `gilrs` path
    /// (`process_event`), which `insert`s into `just_pressed`/`just_released` and lets [`flush`]
    /// — once per frame — be the only thing that clears them. A snapshot backend that *assigns*
    /// the diff instead silently drops inputs, because it is polled far more often than once per
    /// frame: `about_to_wait` runs once per event-loop **iteration**, and input events wake the
    /// loop, so during any mouse movement several snapshots land between two frames. The second
    /// one computes an empty diff (`pressed` already holds the button) and wipes the edge the
    /// first one recorded, before the frame ever read it.
    ///
    /// Deliberately **not** `#[cfg(target_os = "macos")]` even though the GameController backend
    /// is its only caller: a macOS-gated test never runs in CI (the macOS job only builds), and
    /// this is exactly the "share the policy, not the implementation" rule the repo applies to
    /// cfg-split backends. Keeping it un-gated is what lets the regression test run everywhere.
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    fn apply_snapshot(&mut self, buttons: HashSet<GamepadButton>, axes: HashMap<GamepadAxis, f32>) {
        self.just_pressed
            .extend(buttons.difference(&self.pressed).copied());
        self.just_released
            .extend(self.pressed.difference(&buttons).copied());
        self.pressed = buttons;
        self.axes = axes;
    }
}

/// ECS resource holding gamepad input state.
///
/// Supports up to 4 gamepads (slots 0–3).
/// Automatically inserted by `App::new()`; no manual registration required.
///
/// # Example
/// ```ignore
/// // Slot 0 (first connected pad)
/// if let Some(gs) = world.resource::<GamepadState>() {
///     if gs.just_pressed(0, GamepadButton::South) { /* jump */ }
///     let lx = gs.axis(0, GamepadAxis::LeftStickX);
/// }
/// ```
#[derive(Default)]
pub struct GamepadState {
    slots: [Option<Slot>; 4],
    #[cfg(not(target_arch = "wasm32"))]
    id_map: HashMap<gilrs::GamepadId, usize>,
}

impl GamepadState {
    // ── Public query methods ───────────────────────────────────────────────────

    /// Returns `true` if the `pad` slot is connected.
    pub fn is_connected(&self, pad: usize) -> bool {
        pad < 4 && self.slots[pad].is_some()
    }

    /// Returns `true` if at least one gamepad is connected.
    pub fn any_connected(&self) -> bool {
        self.slots.iter().any(|s| s.is_some())
    }

    /// Slot index of the first connected gamepad.
    pub fn primary(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_some())
    }

    /// Returns `true` if `button` is held in slot `pad`.
    pub fn is_pressed(&self, pad: usize, button: GamepadButton) -> bool {
        self.slot(pad).is_some_and(|s| s.pressed.contains(&button))
    }

    /// Returns `true` if `button` was pressed this frame in slot `pad`.
    pub fn just_pressed(&self, pad: usize, button: GamepadButton) -> bool {
        self.slot(pad)
            .is_some_and(|s| s.just_pressed.contains(&button))
    }

    /// Returns `true` if `button` was released this frame in slot `pad`.
    pub fn just_released(&self, pad: usize, button: GamepadButton) -> bool {
        self.slot(pad)
            .is_some_and(|s| s.just_released.contains(&button))
    }

    /// Returns the `axis` value for slot `pad` (−1.0 ~ 1.0, no dead-zone applied).
    pub fn axis(&self, pad: usize, axis: GamepadAxis) -> f32 {
        self.slot(pad)
            .and_then(|s| s.axes.get(&axis).copied())
            .unwrap_or(0.0)
    }

    fn slot(&self, pad: usize) -> Option<&Slot> {
        self.slots.get(pad)?.as_ref()
    }

    // ── Internal event handling (called only from App) ─────────────────────────

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn process_event(&mut self, event: gilrs::Event) {
        use gilrs::EventType;

        let gid = event.id;

        match event.event {
            EventType::Connected if !self.id_map.contains_key(&gid) => {
                if let Some(idx) = self.slots.iter().position(|s| s.is_none()) {
                    self.slots[idx] = Some(Slot::new());
                    self.id_map.insert(gid, idx);
                }
            }
            EventType::Connected => {}
            EventType::Disconnected => {
                if let Some(idx) = self.id_map.remove(&gid) {
                    self.slots[idx] = None;
                }
            }
            EventType::ButtonPressed(btn, _) => {
                if let Some(gb) = map_button(btn) {
                    if let Some(slot) = self.slot_mut(gid) {
                        slot.pressed.insert(gb);
                        slot.just_pressed.insert(gb);
                    }
                }
            }
            EventType::ButtonReleased(btn, _) => {
                if let Some(gb) = map_button(btn) {
                    if let Some(slot) = self.slot_mut(gid) {
                        slot.pressed.remove(&gb);
                        slot.just_released.insert(gb);
                    }
                }
            }
            EventType::AxisChanged(axis, value, _) => {
                if let Some(ga) = map_axis(axis) {
                    if let Some(slot) = self.slot_mut(gid) {
                        slot.axes.insert(ga, value);
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn flush(&mut self) {
        for slot in self.slots.iter_mut().flatten() {
            slot.flush();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn slot_mut(&mut self, gid: gilrs::GamepadId) -> Option<&mut Slot> {
        let idx = *self.id_map.get(&gid)?;
        self.slots[idx].as_mut()
    }

    /// Apply a poll-based snapshot of one pad (the macOS GameController backend; see
    /// `input::gamepad_macos`). Unlike the event-driven gilrs path, this gets the full held-button
    /// set + axis values each frame, so `just_pressed`/`just_released` are derived by diffing the
    /// new held set against the previous one (`pressed` persists across [`flush`], which only clears
    /// the `just_*` edges). Connects the slot on first sight.
    #[cfg(target_os = "macos")]
    pub(crate) fn apply_macos_snapshot(
        &mut self,
        pad: usize,
        buttons: HashSet<GamepadButton>,
        axes: HashMap<GamepadAxis, f32>,
    ) {
        if pad >= 4 {
            return;
        }
        self.slots[pad]
            .get_or_insert_with(Slot::new)
            .apply_snapshot(buttons, axes);
    }

    /// Drop the macOS gamepad in `pad` (the GameController framework no longer lists it).
    #[cfg(target_os = "macos")]
    pub(crate) fn disconnect_macos(&mut self, pad: usize) {
        if pad < 4 {
            self.slots[pad] = None;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn map_button(btn: gilrs::Button) -> Option<GamepadButton> {
    use gilrs::Button;
    Some(match btn {
        Button::South => GamepadButton::South,
        Button::East => GamepadButton::East,
        Button::North => GamepadButton::North,
        Button::West => GamepadButton::West,
        Button::LeftTrigger => GamepadButton::LeftBumper,
        Button::RightTrigger => GamepadButton::RightBumper,
        Button::LeftTrigger2 => GamepadButton::LeftTrigger,
        Button::RightTrigger2 => GamepadButton::RightTrigger,
        Button::Select => GamepadButton::Select,
        Button::Start => GamepadButton::Start,
        Button::LeftThumb => GamepadButton::LeftThumb,
        Button::RightThumb => GamepadButton::RightThumb,
        Button::DPadUp => GamepadButton::DPadUp,
        Button::DPadDown => GamepadButton::DPadDown,
        Button::DPadLeft => GamepadButton::DPadLeft,
        Button::DPadRight => GamepadButton::DPadRight,
        _ => return None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn map_axis(axis: gilrs::Axis) -> Option<GamepadAxis> {
    use gilrs::Axis;
    Some(match axis {
        Axis::LeftStickX => GamepadAxis::LeftStickX,
        Axis::LeftStickY => GamepadAxis::LeftStickY,
        Axis::RightStickX => GamepadAxis::RightStickX,
        Axis::RightStickY => GamepadAxis::RightStickY,
        Axis::LeftZ => GamepadAxis::LeftTrigger,
        Axis::RightZ => GamepadAxis::RightTrigger,
        Axis::DPadX => GamepadAxis::DPadX,
        Axis::DPadY => GamepadAxis::DPadY,
        _ => return None,
    })
}

#[cfg(test)]
impl GamepadState {
    /// Test-only: connect `pad` (if needed) and mark `button` as just-pressed + held, as if a
    /// `ButtonPressed` event had arrived this frame. Lets non-`gilrs` tests drive gamepad input.
    pub(crate) fn test_press(&mut self, pad: usize, button: GamepadButton) {
        let slot = self.slots[pad].get_or_insert_with(|| Slot {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
            axes: HashMap::new(),
        });
        slot.pressed.insert(button);
        slot.just_pressed.insert(button);
    }

    /// Test-only: connect `pad` (if needed) and set `axis` to `value`, as if an `AxisChanged` event
    /// had arrived. Lets non-`gilrs` tests drive analog input (e.g. analog-stick UI navigation).
    pub(crate) fn test_axis(&mut self, pad: usize, axis: GamepadAxis, value: f32) {
        let slot = self.slots[pad].get_or_insert_with(|| Slot {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
            axes: HashMap::new(),
        });
        slot.axes.insert(axis, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A snapshot backend must **accumulate** press/release edges until the frame's [`flush`],
    /// exactly like the event-driven `gilrs` path does.
    ///
    /// `apply_macos_snapshot` used to *assign* `slot.just_pressed = <diff>`. `flush()` runs once
    /// per frame (`app/schedule.rs`), but the snapshot is taken in `about_to_wait`, which runs
    /// once per event-loop **iteration** — and input events wake the loop, so during any mouse
    /// movement several snapshots land between two frames. The second one sees the button already
    /// in `pressed`, computes an empty diff, and wipes the edge before the frame ever read it:
    /// button presses vanish on macOS whenever the player is also moving the mouse.
    ///
    /// This drives that exact interleaving. It runs on every platform because the edge policy
    /// lives in the un-gated `Slot::apply_snapshot`; a `#[cfg(target_os = "macos")]` test would
    /// never run in CI, since the macOS job only builds.
    #[test]
    fn snapshot_accumulates_edges_until_flush() {
        let mut slot = Slot::new();

        // Event-loop iteration 1: the button goes down.
        slot.apply_snapshot(HashSet::from([GamepadButton::South]), HashMap::new());
        assert!(slot.just_pressed.contains(&GamepadButton::South));

        // Iteration 2 — still held, and still no frame boundary, so no flush has run.
        slot.apply_snapshot(HashSet::from([GamepadButton::South]), HashMap::new());
        assert!(
            slot.just_pressed.contains(&GamepadButton::South),
            "press edge dropped by a second snapshot taken before the frame read it"
        );

        // The frame reads input and only then flushes.
        slot.flush();
        assert!(slot.just_pressed.is_empty(), "flush must clear the edge");
        assert!(
            slot.pressed.contains(&GamepadButton::South),
            "flush must NOT clear the held set"
        );

        // The release edge has the same property.
        slot.apply_snapshot(HashSet::new(), HashMap::new());
        assert!(slot.just_released.contains(&GamepadButton::South));
        slot.apply_snapshot(HashSet::new(), HashMap::new());
        assert!(
            slot.just_released.contains(&GamepadButton::South),
            "release edge dropped by a second snapshot taken before the frame read it"
        );
    }

    #[test]
    fn default_is_disconnected() {
        let gs = GamepadState::default();
        assert!(!gs.any_connected());
        assert!(gs.primary().is_none());
    }

    #[test]
    fn axis_returns_zero_when_disconnected() {
        let gs = GamepadState::default();
        assert_eq!(gs.axis(0, GamepadAxis::LeftStickX), 0.0);
    }
}
