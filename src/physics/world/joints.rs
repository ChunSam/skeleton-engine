use glam::Vec2;
use rapier2d::prelude::*;

use super::{BodyHandle, JointHandle, PhysicsWorld};

/// Default spring constants for `add_distance_joint` — stiff enough to behave like a near-rigid
/// rod. Use `add_spring_joint` for a softer, tunable spring.
const DISTANCE_JOINT_STIFFNESS: f32 = 1000.0;
const DISTANCE_JOINT_DAMPING: f32 = 10.0;

impl PhysicsWorld {
    /// Creates a DistanceJoint between two bodies.
    /// `anchor1/2` — attachment point in each body's local space (world units).
    /// A near-rigid spring (stiffness=1000, damping=10) maintains a fixed distance; use
    /// [`add_spring_joint`](PhysicsWorld::add_spring_joint) to pick your own stiffness/damping.
    pub fn add_distance_joint(
        &mut self,
        body1: BodyHandle,
        body2: BodyHandle,
        anchor1: Vec2,
        anchor2: Vec2,
        rest_length: f32,
    ) -> JointHandle {
        self.add_spring_joint(
            body1,
            body2,
            anchor1,
            anchor2,
            rest_length,
            DISTANCE_JOINT_STIFFNESS,
            DISTANCE_JOINT_DAMPING,
        )
    }

    /// Creates a spring joint between two bodies with explicit `stiffness` and `damping`.
    ///
    /// A soft spring (low stiffness) bounces and stretches; a high stiffness approximates a
    /// rigid rod. `damping` bleeds off oscillation. `anchor1/2` are in each body's local space.
    #[allow(clippy::too_many_arguments)]
    pub fn add_spring_joint(
        &mut self,
        body1: BodyHandle,
        body2: BodyHandle,
        anchor1: Vec2,
        anchor2: Vec2,
        rest_length: f32,
        stiffness: f32,
        damping: f32,
    ) -> JointHandle {
        let data = SpringJointBuilder::new(rest_length, stiffness, damping)
            .local_anchor1(point![anchor1.x, anchor1.y])
            .local_anchor2(point![anchor2.x, anchor2.y])
            .build();
        JointHandle(self.impulse_joint_set.insert(body1.0, body2.0, data, true))
    }

    /// RevoluteJoint (hinge) — two bodies rotate freely around a shared pivot point.
    pub fn add_revolute_joint(
        &mut self,
        body1: BodyHandle,
        body2: BodyHandle,
        anchor1: Vec2,
        anchor2: Vec2,
    ) -> JointHandle {
        let data = RevoluteJointBuilder::new()
            .local_anchor1(point![anchor1.x, anchor1.y])
            .local_anchor2(point![anchor2.x, anchor2.y])
            .build();
        JointHandle(self.impulse_joint_set.insert(body1.0, body2.0, data, true))
    }

    /// PrismaticJoint (slider) — allows relative movement along a specific axis only.
    ///
    /// `axis` must be a non-zero direction vector; it is normalised internally.
    /// If a near-zero `axis` is supplied (length² ≤ ε²), a warning is logged and
    /// [`Vec2::X`] is used as a fallback so the joint is never created with a NaN axis.
    pub fn add_prismatic_joint(
        &mut self,
        body1: BodyHandle,
        body2: BodyHandle,
        anchor1: Vec2,
        anchor2: Vec2,
        axis: Vec2,
    ) -> JointHandle {
        let safe_axis = if axis.length_squared() <= f32::EPSILON * f32::EPSILON {
            log::warn!(
                "add_prismatic_joint: near-zero axis supplied ({axis:?}), using Vec2::X fallback"
            );
            Vec2::X
        } else {
            axis
        };
        let unit_axis = UnitVector::new_normalize(vector![safe_axis.x, safe_axis.y]);
        let data = PrismaticJointBuilder::new(unit_axis)
            .local_anchor1(point![anchor1.x, anchor1.y])
            .local_anchor2(point![anchor2.x, anchor2.y])
            .build();
        JointHandle(self.impulse_joint_set.insert(body1.0, body2.0, data, true))
    }

    /// Removes a joint.
    pub fn remove_joint(&mut self, handle: JointHandle) {
        self.impulse_joint_set.remove(handle.0, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    fn make_two_bodies(physics: &mut PhysicsWorld) -> (BodyHandle, BodyHandle) {
        let (b1, _) = physics.add_dynamic_box(Vec2::ZERO, 0.5, 0.5, false);
        let (b2, _) = physics.add_dynamic_box(Vec2::new(1.0, 0.0), 0.5, 0.5, false);
        (b1, b2)
    }

    /// A zero axis must not produce a NaN-axis prismatic joint — the guard must
    /// substitute Vec2::X and the joint data must contain only finite values.
    #[test]
    fn prismatic_joint_zero_axis_uses_fallback() {
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let (b1, b2) = make_two_bodies(&mut physics);
        let handle = physics.add_prismatic_joint(b1, b2, Vec2::ZERO, Vec2::ZERO, Vec2::ZERO);

        // The joint must exist and its motor axis must be finite (no NaN).
        let joint = physics
            .impulse_joint_set
            .get(handle.0)
            .expect("joint present");
        let unit = joint
            .data
            .as_prismatic()
            .expect("is prismatic")
            .local_axis1();
        let v = unit.into_inner();
        assert!(
            v.x.is_finite() && v.y.is_finite(),
            "joint axis must be finite after zero-axis guard: ({}, {})",
            v.x,
            v.y
        );
        // The fallback axis is Vec2::X, so the normalised axis should be (1, 0).
        assert!(
            (v.x - 1.0).abs() < 1e-5 && v.y.abs() < 1e-5,
            "fallback axis should be X (1, 0): ({}, {})",
            v.x,
            v.y
        );
    }

    /// A normal axis must produce a correctly oriented prismatic joint.
    #[test]
    fn prismatic_joint_normal_axis_preserved() {
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let (b1, b2) = make_two_bodies(&mut physics);
        let handle = physics.add_prismatic_joint(b1, b2, Vec2::ZERO, Vec2::ZERO, Vec2::Y);

        let joint = physics
            .impulse_joint_set
            .get(handle.0)
            .expect("joint present");
        let unit = joint
            .data
            .as_prismatic()
            .expect("is prismatic")
            .local_axis1();
        let v = unit.into_inner();
        assert!(
            v.x.abs() < 1e-5 && (v.y - 1.0).abs() < 1e-5,
            "axis should be Y (0, 1): ({}, {})",
            v.x,
            v.y
        );
    }

    /// `JointHandle::raw` returns the underlying rapier handle; `from_raw`
    /// reconstructs an identical `JointHandle`. Round-trip must be lossless.
    #[test]
    fn joint_handle_raw_round_trip() {
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let (b1, b2) = make_two_bodies(&mut physics);
        let handle = physics.add_revolute_joint(b1, b2, Vec2::ZERO, Vec2::ZERO);

        // raw() must expose the inner rapier handle.
        let raw = handle.raw();
        assert_eq!(
            raw, handle.0,
            "raw() must return the inner ImpulseJointHandle"
        );

        // from_raw() must reconstruct an equivalent JointHandle.
        let reconstructed = JointHandle::from_raw(raw);
        assert_eq!(
            reconstructed, handle,
            "JointHandle::from_raw(handle.raw()) must equal the original"
        );

        // The reconstructed handle must still address the same joint in the rapier set.
        assert!(
            physics.impulse_joint_set.get(reconstructed.0).is_some(),
            "reconstructed handle must address a live joint"
        );
    }
}
