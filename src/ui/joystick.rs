use glam::Vec2;

use crate::input::TouchState;

/// Virtual joystick component.
///
/// Attach it to an entity. Call `update()` every frame to process touch
/// (or mouse-emulated) input and refresh the `output` direction vector.
///
/// # Example
/// ```ignore
/// let joy_e = world.spawn();
/// world.add_component(joy_e, VirtualJoystick::new(Vec2::new(120.0, 480.0), 60.0));
///
/// // inside a system
/// if let Some(joy) = world.get_mut::<VirtualJoystick>(joy_e) {
///     if let Some(ts) = world.resource::<TouchState>() {
///         joy.update(ts);
///     }
///     let dir = joy.output; // Vec2 (-1..1, -1..1)
/// }
/// ```
pub struct VirtualJoystick {
    /// Center coordinate of the joystick base (screen/UI coordinate space).
    pub center: Vec2,

    /// Maximum radius the stick can travel (pixels).
    pub radius: f32,

    /// Normalized output direction. Each axis range: -1.0 to 1.0.
    /// `Vec2::ZERO` when there is no input.
    pub output: Vec2,

    /// Current screen coordinate of the stick knob (for rendering/debug visualization).
    pub stick_pos: Vec2,

    /// When `true`, the joystick circle is visualized via DebugDraw.
    pub visible: bool,

    /// ID of the touch point currently controlling this joystick.
    touch_id: Option<u64>,
}

impl VirtualJoystick {
    /// Creates a new virtual joystick with the given center and radius.
    pub fn new(center: Vec2, radius: f32) -> Self {
        Self {
            center,
            radius,
            output: Vec2::ZERO,
            stick_pos: center,
            visible: true,
            touch_id: None,
        }
    }

    /// Updates the joystick state from `TouchState` each frame.
    ///
    /// Must be called before `TouchState::flush()`.
    pub fn update(&mut self, touch_state: &TouchState) {
        // 1. No touch_id: find a touch within the radius in the began list and assign it.
        if self.touch_id.is_none() {
            for &(id, pos) in &touch_state.began {
                if (pos - self.center).length() <= self.radius {
                    self.touch_id = Some(id);
                    self.update_stick(pos);
                    break;
                }
            }
        }

        // 2. touch_id is set: track the current position of that touch point.
        if let Some(active_id) = self.touch_id {
            // Check for an ended event.
            let is_ended = touch_state.ended.iter().any(|&(id, _)| id == active_id);

            if is_ended {
                self.touch_id = None;
                self.output = Vec2::ZERO;
                self.stick_pos = self.center;
            } else {
                // Find the current position of the active touch point.
                let pos = touch_state
                    .active_touches()
                    .find(|(id, _)| *id == active_id)
                    .map(|(_, pos)| pos);

                if let Some(pos) = pos {
                    self.update_stick(pos);
                }
            }
        }
    }

    /// Updates the stick position and output vector to the given touch position.
    fn update_stick(&mut self, pos: Vec2) {
        let delta = pos - self.center;
        let magnitude = delta.length();

        if magnitude < f32::EPSILON {
            self.output = Vec2::ZERO;
            self.stick_pos = self.center;
        } else if magnitude > self.radius {
            // Outside radius: keep direction only.
            self.output = delta / magnitude; // normalize
            self.stick_pos = self.center + self.output * self.radius;
        } else {
            // Inside radius: normalize to 0..1.
            self.output = delta / self.radius;
            self.stick_pos = pos;
        }
    }

    /// Returns whether the joystick is currently being held.
    pub fn is_active(&self) -> bool {
        self.touch_id.is_some()
    }

    /// Updates the joystick by passing raw `TouchState` data directly.
    ///
    /// Use this when a system cannot simultaneously borrow `world.resource::<TouchState>()`
    /// and `world.get_mut::<VirtualJoystick>()`.
    /// Copy the touch data into owned values first, then call this method via `world.get_mut`.
    ///
    /// # Arguments
    /// - `began`: touches that started this frame `(id, position)`
    /// - `ended`: touches that ended this frame `(id, position)`
    /// - `active`: currently active touches `(id, position)`
    pub fn update_raw(
        &mut self,
        began: &[(u64, Vec2)],
        ended: &[(u64, Vec2)],
        active: &[(u64, Vec2)],
    ) {
        // 1. No touch_id: find a touch within the radius in began and assign it.
        if self.touch_id.is_none() {
            for &(id, pos) in began {
                if (pos - self.center).length() <= self.radius {
                    self.touch_id = Some(id);
                    self.update_stick(pos);
                    break;
                }
            }
        }

        // 2. touch_id is set: track the current position.
        if let Some(active_id) = self.touch_id {
            let is_ended = ended.iter().any(|&(id, _)| id == active_id);
            if is_ended {
                self.touch_id = None;
                self.output = Vec2::ZERO;
                self.stick_pos = self.center;
            } else if let Some(&(_, pos)) = active.iter().find(|(id, _)| *id == active_id) {
                self.update_stick(pos);
            }
        }
    }

    /// Returns the output with a deadzone applied.
    ///
    /// Small inputs within the `deadzone` range are treated as `Vec2::ZERO`.
    pub fn output_with_deadzone(&self, deadzone: f32) -> Vec2 {
        if self.output.length() < deadzone {
            Vec2::ZERO
        } else {
            self.output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::TouchState;

    #[test]
    fn joystick_activates_on_touch_within_radius() {
        let mut joy = VirtualJoystick::new(Vec2::new(100.0, 100.0), 60.0);
        let mut ts = TouchState::default();

        ts.on_touch_started(0, Vec2::new(110.0, 100.0)); // within radius
        joy.update(&ts);

        assert!(joy.is_active());
        assert!(joy.output.length() > 0.0);
    }

    #[test]
    fn joystick_ignores_touch_outside_radius() {
        let mut joy = VirtualJoystick::new(Vec2::new(100.0, 100.0), 60.0);
        let mut ts = TouchState::default();

        ts.on_touch_started(0, Vec2::new(300.0, 300.0)); // outside radius
        joy.update(&ts);

        assert!(!joy.is_active());
    }

    #[test]
    fn joystick_resets_on_touch_end() {
        let mut joy = VirtualJoystick::new(Vec2::new(100.0, 100.0), 60.0);
        let mut ts = TouchState::default();

        ts.on_touch_started(0, Vec2::new(110.0, 100.0));
        joy.update(&ts);
        assert!(joy.is_active());

        ts.flush();
        ts.on_touch_ended(0, Vec2::new(110.0, 100.0));
        joy.update(&ts);

        assert!(!joy.is_active());
        assert_eq!(joy.output, Vec2::ZERO);
        assert_eq!(joy.stick_pos, joy.center);
    }

    #[test]
    fn joystick_output_clamped_at_unit_when_outside_radius() {
        let mut joy = VirtualJoystick::new(Vec2::new(0.0, 0.0), 50.0);
        let mut ts = TouchState::default();

        // Start inside radius to activate the joystick.
        ts.on_touch_started(0, Vec2::new(10.0, 0.0));
        joy.update(&ts);
        assert!(joy.is_active());

        // Next frame: move outside the radius.
        ts.flush();
        ts.on_touch_started(0, Vec2::new(10.0, 0.0)); // keep in active
        ts.on_touch_moved(0, Vec2::new(200.0, 0.0));
        joy.update(&ts);

        // output magnitude should be 1.0 (normalized)
        assert!((joy.output.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn joystick_output_proportional_inside_radius() {
        let mut joy = VirtualJoystick::new(Vec2::new(0.0, 0.0), 100.0);
        let mut ts = TouchState::default();

        ts.on_touch_started(0, Vec2::new(50.0, 0.0)); // half the radius
        joy.update(&ts);

        // output.x ≈ 0.5
        assert!((joy.output.x - 0.5).abs() < 1e-5);
        assert!(joy.output.y.abs() < 1e-5);
    }

    #[test]
    fn deadzone_suppresses_small_input() {
        let mut joy = VirtualJoystick::new(Vec2::new(0.0, 0.0), 100.0);
        let mut ts = TouchState::default();

        ts.on_touch_started(0, Vec2::new(5.0, 0.0)); // very small movement
        joy.update(&ts);

        assert_eq!(joy.output_with_deadzone(0.1), Vec2::ZERO);
    }
}
