use serde::{Deserialize, Serialize};

use glam::Vec2;

use crate::input::TouchState;
use crate::reflect::{Reflect, ReflectValue};

/// Virtual joystick component.
///
/// Attach it to an entity. Call [`update`](Self::update) every frame to process touch
/// (or mouse-emulated) input and refresh the [`output`](Self::output) direction vector.
///
/// Authored state — [`center`](Self::center), [`radius`](Self::radius),
/// [`visible`](Self::visible) — serializes and reflects, so a joystick can be placed in a scene
/// file or the inspector like any other UI component (v0.159.0; before that this type derived
/// nothing and the "attach it to an entity" above was only reachable from code). The knob's live
/// position and the touch it is tracking are runtime state and are **not** saved.
///
/// # Example
///
/// [`update_raw`](Self::update_raw) is the borrow-friendly entry point: a system cannot hold
/// `world.resource::<TouchState>()` and `world.get_mut::<VirtualJoystick>()` at the same time, so
/// copy the touch lists out first and pass them in.
///
/// ```
/// # use engine::{VirtualJoystick, Vec2, World};
/// let mut world = World::new();
/// let joy_e = world.spawn();
/// world.add_component(joy_e, VirtualJoystick::new(Vec2::new(120.0, 480.0), 60.0));
///
/// // A touch lands 30 px right of the centre — half of the 60 px radius.
/// let touches = [(0, Vec2::new(150.0, 480.0))];
/// if let Some(joy) = world.get_mut::<VirtualJoystick>(joy_e) {
///     joy.update_raw(&touches, &[], &touches);
///     assert!(joy.is_active());
///     assert_eq!(joy.output, Vec2::new(0.5, 0.0)); // Vec2 (-1..1, -1..1)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VirtualJoystick {
    /// Center coordinate of the joystick base (screen/UI coordinate space).
    pub center: Vec2,

    /// Maximum radius the stick can travel (pixels).
    pub radius: f32,

    /// Normalized output direction. Each axis range: -1.0 to 1.0.
    /// `Vec2::ZERO` when there is no input.
    ///
    /// Runtime state — not serialized; a loaded joystick starts centred.
    #[serde(skip)]
    pub output: Vec2,

    /// Current screen coordinate of the stick knob (for rendering/debug visualization).
    ///
    /// Runtime state — not serialized. While the stick is not held this equals
    /// [`center`](Self::center), which `update`/`update_raw` re-establish on their first call so a
    /// joystick loaded from a scene does not draw its knob at the origin.
    #[serde(skip)]
    pub stick_pos: Vec2,

    /// When `true`, the joystick circle is visualized via DebugDraw.
    pub visible: bool,

    /// ID of the touch point currently controlling this joystick. Runtime state — not serialized,
    /// so a loaded joystick is never already latched onto a touch from another session.
    #[serde(skip)]
    touch_id: Option<u64>,
}

impl Default for VirtualJoystick {
    fn default() -> Self {
        Self::new(Vec2::ZERO, 60.0)
    }
}

impl Reflect for VirtualJoystick {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("center", ReflectValue::Vec2(self.center)),
            ("radius", ReflectValue::F32(self.radius)),
            ("visible", ReflectValue::Bool(self.visible)),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("center", ReflectValue::Vec2(v)) => {
                // Moving the base moves the resting knob with it; otherwise an inspector drag
                // leaves the knob behind until the next touch.
                if !self.is_active() {
                    self.stick_pos = v;
                }
                self.center = v;
                true
            }
            ("radius", ReflectValue::F32(v)) => {
                self.radius = v;
                true
            }
            ("visible", ReflectValue::Bool(v)) => {
                self.visible = v;
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "VirtualJoystick"
    }
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
        self.rest_knob_at_center();

        // 1. No touch_id: find a touch within the radius in the began list and assign it.
        if self.touch_id.is_none() {
            for &(id, pos) in touch_state.began() {
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
            let is_ended = touch_state.ended().iter().any(|&(id, _)| id == active_id);

            if is_ended {
                self.release();
            } else {
                // Find the current position of the active touch point.
                let pos = touch_state
                    .active_touches()
                    .find(|(id, _)| *id == active_id)
                    .map(|(_, pos)| pos);

                match pos {
                    Some(pos) => self.update_stick(pos),
                    // Gone from the live set without this joystick seeing its `ended` frame —
                    // release it. See `release`.
                    None => self.release(),
                }
            }
        }
    }

    /// Re-establishes "not held ⇒ the knob rests at the centre".
    ///
    /// Every other path already keeps that invariant — `release` re-centres and `update_stick`
    /// only ever runs with a touch latched. Deserialization is the one way in that does not:
    /// `stick_pos` is `#[serde(skip)]` runtime state, so it loads as `Vec2::ZERO` while `center`
    /// loads as whatever the scene says, and a joystick placed at (120, 480) would draw its knob
    /// in the corner until the player first touched it.
    fn rest_knob_at_center(&mut self) {
        if self.touch_id.is_none() {
            self.stick_pos = self.center;
        }
    }

    /// Un-latches the stick: forgets the tracked touch and re-centres the output.
    ///
    /// ⚠️ **`ended()` is a one-frame buffer, so it cannot be the only release signal.** Until
    /// v0.156.28 it was: a frame this joystick did not observe — a movement system gated behind a
    /// game state, a pause, a scene swap — consumed the release, after which `touch_id` stayed
    /// `Some` forever. `output` kept its last non-zero value, `is_active()` stayed true, and no
    /// new touch could claim the stick (step 1 is gated on `touch_id.is_none()`), leaving the
    /// player walking into a wall with no way out and no public reset. `active_touches()` is the
    /// durable signal: a touch missing from it is gone whether or not its `ended` frame was seen.
    fn release(&mut self) {
        self.touch_id = None;
        self.output = Vec2::ZERO;
        self.stick_pos = self.center;
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
        self.rest_knob_at_center();

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
                self.release();
            } else if let Some(&(_, pos)) = active.iter().find(|(id, _)| *id == active_id) {
                self.update_stick(pos);
            } else {
                // Same missed-release hole as `update`; see `release`.
                self.release();
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

    /// v0.156.28: `ended()` is a one-frame buffer, so a frame `update` did not observe consumed
    /// the release and the stick stayed latched at its last output forever — no new touch could
    /// claim it and there was no public reset.
    #[test]
    fn a_missed_release_frame_still_releases_the_stick() {
        let mut ts = TouchState::default();
        let mut joy = VirtualJoystick::new(Vec2::ZERO, 50.0);

        ts.on_touch_started(0, Vec2::new(25.0, 0.0));
        joy.update(&ts);
        assert!(joy.is_active());
        assert_ne!(joy.output, Vec2::ZERO);

        // The frame carrying `ended` goes by without this joystick seeing it.
        ts.flush();
        ts.on_touch_ended(0, Vec2::new(25.0, 0.0));
        ts.flush();

        joy.update(&ts);
        assert!(
            !joy.is_active(),
            "a touch gone from the live set must release the stick"
        );
        assert_eq!(joy.output, Vec2::ZERO);

        // ...and the stick is claimable again.
        ts.on_touch_started(1, Vec2::new(-25.0, 0.0));
        joy.update(&ts);
        assert!(joy.is_active());
        assert!(
            joy.output.x < 0.0,
            "the new touch drives it, got {:?}",
            joy.output
        );
    }

    /// v0.159.0: the authored fields round-trip through a scene file; the runtime ones do not.
    #[test]
    fn joystick_serde_roundtrip_drops_runtime_state() {
        let mut joy = VirtualJoystick::new(Vec2::new(120.0, 480.0), 60.0);
        joy.visible = false;
        joy.update_raw(
            &[(7, Vec2::new(150.0, 480.0))],
            &[],
            &[(7, Vec2::new(150.0, 480.0))],
        );
        assert!(joy.is_active(), "the fixture is a joystick mid-hold");

        let ron = ron::to_string(&joy).expect("serialize");
        let back: VirtualJoystick = ron::from_str(&ron).expect("deserialize");

        assert_eq!(back.center, Vec2::new(120.0, 480.0));
        assert_eq!(back.radius, 60.0);
        assert!(!back.visible);
        assert!(
            !back.is_active(),
            "a loaded joystick must not still be latched onto a touch from another session"
        );
        assert_eq!(back.output, Vec2::ZERO, "and its output starts centred");
    }

    /// The knob of a *loaded* joystick rests at its centre, not at the origin. `stick_pos` is
    /// runtime state and deserializes to `Vec2::ZERO` while `center` comes from the scene, so the
    /// invariant every other path keeps has to be re-established on the first update.
    #[test]
    fn a_loaded_joystick_rests_its_knob_at_the_center() {
        let joy = VirtualJoystick::new(Vec2::new(120.0, 480.0), 60.0);
        let ron = ron::to_string(&joy).expect("serialize");
        let mut back: VirtualJoystick = ron::from_str(&ron).expect("deserialize");
        assert_eq!(
            back.stick_pos,
            Vec2::ZERO,
            "serde alone cannot know the centre"
        );

        back.update(&TouchState::default());

        assert_eq!(
            back.stick_pos,
            Vec2::new(120.0, 480.0),
            "the first update must put the resting knob back on the base"
        );
    }

    /// A held stick is not re-centred by the guard above — only an unheld one is.
    #[test]
    fn resting_the_knob_does_not_disturb_a_live_hold() {
        let mut joy = VirtualJoystick::new(Vec2::new(100.0, 100.0), 60.0);
        let touch = [(3, Vec2::new(130.0, 100.0))];
        joy.update_raw(&touch, &[], &touch);
        let held = joy.stick_pos;
        assert_ne!(held, joy.center, "the fixture holds the knob off-centre");

        joy.update_raw(&[], &[], &touch);

        assert_eq!(joy.stick_pos, held, "a live hold keeps its knob position");
        assert_eq!(joy.output, Vec2::new(0.5, 0.0));
    }

    #[test]
    fn joystick_reflect_roundtrip() {
        let mut joy = VirtualJoystick::new(Vec2::new(10.0, 20.0), 40.0);
        assert!(joy.set_field("radius", ReflectValue::F32(75.0)));
        assert!(joy.set_field("visible", ReflectValue::Bool(false)));
        assert!(joy.set_field("center", ReflectValue::Vec2(Vec2::new(200.0, 300.0))));
        assert_eq!(joy.radius, 75.0);
        assert!(!joy.visible);
        assert_eq!(joy.center, Vec2::new(200.0, 300.0));
        assert_eq!(
            joy.stick_pos,
            Vec2::new(200.0, 300.0),
            "moving the base moves the resting knob with it"
        );

        let names: Vec<_> = joy.fields().into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, vec!["center", "radius", "visible"]);
        assert!(
            !joy.set_field("output", ReflectValue::Vec2(Vec2::ONE)),
            "runtime state is not settable through Reflect"
        );
    }
}
