use glam::Vec2;
use rapier2d::prelude::*;

use super::{JointHandle, PhysicsWorld};

impl PhysicsWorld {
    /// Creates a DistanceJoint between two bodies.
    /// `anchor1/2` — attachment point in each body's local space (world units).
    /// Internally uses `SpringJointBuilder` (stiffness=1000, damping=10) to maintain a fixed distance.
    pub fn add_distance_joint(
        &mut self,
        body1: RigidBodyHandle,
        body2: RigidBodyHandle,
        anchor1: Vec2,
        anchor2: Vec2,
        rest_length: f32,
    ) -> JointHandle {
        let data = SpringJointBuilder::new(rest_length, 1000.0, 10.0)
            .local_anchor1(point![anchor1.x, anchor1.y])
            .local_anchor2(point![anchor2.x, anchor2.y])
            .build();
        JointHandle(self.impulse_joint_set.insert(body1, body2, data, true))
    }

    /// RevoluteJoint (hinge) — two bodies rotate freely around a shared pivot point.
    pub fn add_revolute_joint(
        &mut self,
        body1: RigidBodyHandle,
        body2: RigidBodyHandle,
        anchor1: Vec2,
        anchor2: Vec2,
    ) -> JointHandle {
        let data = RevoluteJointBuilder::new()
            .local_anchor1(point![anchor1.x, anchor1.y])
            .local_anchor2(point![anchor2.x, anchor2.y])
            .build();
        JointHandle(self.impulse_joint_set.insert(body1, body2, data, true))
    }

    /// PrismaticJoint (slider) — allows relative movement along a specific axis only.
    pub fn add_prismatic_joint(
        &mut self,
        body1: RigidBodyHandle,
        body2: RigidBodyHandle,
        anchor1: Vec2,
        anchor2: Vec2,
        axis: Vec2,
    ) -> JointHandle {
        let unit_axis = UnitVector::new_normalize(vector![axis.x, axis.y]);
        let data = PrismaticJointBuilder::new(unit_axis)
            .local_anchor1(point![anchor1.x, anchor1.y])
            .local_anchor2(point![anchor2.x, anchor2.y])
            .build();
        JointHandle(self.impulse_joint_set.insert(body1, body2, data, true))
    }

    /// Removes a joint.
    pub fn remove_joint(&mut self, handle: JointHandle) {
        self.impulse_joint_set.remove(handle.0, true);
    }
}
