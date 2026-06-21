use rapier2d::control::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use rapier2d::na;

/// Kinematic character controller component.
///
/// Attach to the same entity as `PhysicsBody` and drive movement each frame
/// via `PhysicsWorld::move_character()`.
///
/// ```rust,ignore
/// // At entity creation
/// let (rb, col) = physics.add_kinematic_box(start_pos / PPU, 0.4, 0.9);
/// let player = world.spawn();
/// world.add_component(player, PhysicsBody { rigid_body_handle: rb, collider_handle: col });
/// world.add_component(player, CharacterController::new());
///
/// // Inside system run()
/// let desired = Vec2::new(move_x * speed * dt, gravity_vel * dt);
/// physics.move_character(controller, body.rigid_body_handle, body.collider_handle, desired, dt, PPU);
/// if controller.grounded { /* can jump */ }
/// ```
pub struct CharacterController {
    /// Maximum slope angle that can be climbed (radians). Default π/4 (45°).
    ///
    /// This field is **authoritative**: `move_character` writes it into the
    /// internal Rapier controller before each move, so setting it directly is
    /// safe and takes effect on the very next call. Prefer `with_max_slope_deg`
    /// for the builder pattern, but direct assignment works too.
    pub max_slope_angle: f32,
    /// Whether the character was grounded after the previous `move_character` call.
    pub grounded: bool,
    pub(crate) inner: KinematicCharacterController,
    /// Remaining time (seconds) to drop through a one-way platform.
    /// Set by `request_drop`; decremented every frame by `move_character`.
    pub(crate) drop_timer: f32,
    /// How long (seconds) a `request_drop` keeps the one-way pass-through window open.
    /// Defaults to [`CharacterController::DROP_DURATION`]; tune via
    /// [`with_drop_duration`](CharacterController::with_drop_duration) or direct assignment
    /// (heavier characters may need a longer window to clear a thick platform).
    pub drop_duration: f32,
}

impl Default for CharacterController {
    fn default() -> Self {
        // The engine uses screen coordinates (Y+ is down), so up = -Y
        let inner = KinematicCharacterController {
            up: na::Unit::new_normalize(na::Vector2::new(0.0, -1.0)),
            max_slope_climb_angle: std::f32::consts::FRAC_PI_4,
            min_slope_slide_angle: std::f32::consts::FRAC_PI_4,
            snap_to_ground: Some(CharacterLength::Absolute(0.1)),
            autostep: Some(CharacterAutostep {
                max_height: CharacterLength::Absolute(0.3),
                min_width: CharacterLength::Absolute(0.1),
                include_dynamic_bodies: false,
            }),
            slide: true,
            ..Default::default()
        };

        Self {
            max_slope_angle: std::f32::consts::FRAC_PI_4,
            grounded: false,
            inner,
            drop_timer: 0.0,
            drop_duration: Self::DROP_DURATION,
        }
    }
}

impl CharacterController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum slope angle in degrees.
    pub fn with_max_slope_deg(mut self, degrees: f32) -> Self {
        let rad = degrees.to_radians();
        self.max_slope_angle = rad;
        self.inner.max_slope_climb_angle = rad;
        self.inner.min_slope_slide_angle = rad;
        self
    }

    /// Sets the maximum step height in physics units.
    pub fn with_autostep(mut self, max_height: f32, min_width: f32) -> Self {
        self.inner.autostep = Some(CharacterAutostep {
            max_height: CharacterLength::Absolute(max_height),
            min_width: CharacterLength::Absolute(min_width),
            include_dynamic_bodies: false,
        });
        self
    }

    /// Sets how long (seconds) a one-way drop-through stays active after `request_drop`.
    pub fn with_drop_duration(mut self, seconds: f32) -> Self {
        self.drop_duration = seconds;
        self
    }

    /// Sets the ground-snap distance in physics units. 0.0 disables snapping.
    pub fn with_snap_to_ground(mut self, distance: f32) -> Self {
        if distance > 0.0 {
            self.inner.snap_to_ground = Some(CharacterLength::Absolute(distance));
        } else {
            self.inner.snap_to_ground = None;
        }
        self
    }

    /// Creates a controller configured for top-down games.
    ///
    /// Differs from [`CharacterController::new()`] in two ways:
    /// - `snap_to_ground` is disabled (`None`): no gravity-driven ground-hugging, which
    ///   would push the character into walls in a top-down view.
    /// - `autostep` is disabled (`None`): stair-climbing logic assumes a vertical "up"
    ///   direction and produces unintended movement on flat top-down floors.
    ///
    /// `slide` remains enabled so the character still collides and slides along walls
    /// rather than stopping dead on contact.
    pub fn top_down() -> Self {
        let mut ctrl = Self::new();
        ctrl.inner.snap_to_ground = None;
        ctrl.inner.autostep = None;
        ctrl
    }

    /// Requests a drop through a one-way platform (call on the down key press).
    ///
    /// For a short time afterward, `move_character` ignores collisions with one-way colliders
    /// so the character falls through the platform underfoot. Solid colliders are unaffected.
    pub fn request_drop(&mut self) {
        self.drop_timer = self.drop_duration;
    }

    /// Returns whether a drop request is active (one-way pass-through window is still open).
    pub fn is_dropping(&self) -> bool {
        self.drop_timer > 0.0
    }
}

impl CharacterController {
    /// Default length of the one-way drop-through window (seconds), long enough for the
    /// character to clear a typical platform thickness. The per-instance value lives in
    /// [`drop_duration`](CharacterController::drop_duration).
    pub const DROP_DURATION: f32 = 0.2;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_max_slope_deg_keeps_field_and_inner_in_sync() {
        let ctrl = CharacterController::new().with_max_slope_deg(30.0);
        let expected = 30_f32.to_radians();
        assert!(
            (ctrl.max_slope_angle - expected).abs() < 1e-6,
            "max_slope_angle field must equal 30° in radians"
        );
        assert!(
            (ctrl.inner.max_slope_climb_angle - expected).abs() < 1e-6,
            "inner.max_slope_climb_angle must be synced by with_max_slope_deg"
        );
        assert!(
            (ctrl.inner.min_slope_slide_angle - expected).abs() < 1e-6,
            "inner.min_slope_slide_angle must be synced by with_max_slope_deg"
        );
    }

    #[test]
    fn top_down_disables_snap_and_autostep() {
        let platformer = CharacterController::new();
        let top_down = CharacterController::top_down();

        // new() enables snap_to_ground; top_down() must disable it.
        assert!(
            platformer.inner.snap_to_ground.is_some(),
            "new() should have snap_to_ground enabled"
        );
        assert!(
            top_down.inner.snap_to_ground.is_none(),
            "top_down() must disable snap_to_ground"
        );

        // new() enables autostep; top_down() must disable it.
        assert!(
            platformer.inner.autostep.is_some(),
            "new() should have autostep enabled"
        );
        assert!(
            top_down.inner.autostep.is_none(),
            "top_down() must disable autostep"
        );

        // slide must remain enabled in both.
        assert!(platformer.inner.slide, "new() must keep slide enabled");
        assert!(top_down.inner.slide, "top_down() must keep slide enabled");
    }

    #[test]
    fn direct_field_assignment_is_detected_by_move_character_sync() {
        // Verify the public field is authoritative: after a direct write, it differs from
        // inner until move_character syncs it. The sync itself is exercised in the physics
        // integration test; here we just verify the field write persists as expected.
        let mut ctrl = CharacterController::new();
        let new_angle = 20_f32.to_radians();
        ctrl.max_slope_angle = new_angle;
        // The public field must reflect the new value.
        assert!(
            (ctrl.max_slope_angle - new_angle).abs() < 1e-6,
            "direct field write must update max_slope_angle"
        );
        // inner is intentionally not yet updated (that happens in move_character).
        assert!(
            (ctrl.inner.max_slope_climb_angle - std::f32::consts::FRAC_PI_4).abs() < 1e-6,
            "inner is updated lazily by move_character, not on direct field write"
        );
    }
}
