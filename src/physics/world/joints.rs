use glam::Vec2;
use rapier2d::prelude::*;

use super::{JointHandle, PhysicsWorld};

impl PhysicsWorld {
    /// 두 바디 사이에 DistanceJoint를 생성한다.
    /// `anchor1/2` — 각 바디 로컬 공간의 연결점 (월드 단위).
    /// 내부적으로 `SpringJointBuilder`(stiffness=1000, damping=10)를 사용해 고정 거리를 유지한다.
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

    /// RevoluteJoint (힌지) — 두 바디가 공통 피벗점을 기준으로 자유 회전.
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

    /// PrismaticJoint (슬라이더) — 특정 축 방향으로만 상대 이동 허용.
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

    /// 조인트를 제거한다.
    pub fn remove_joint(&mut self, handle: JointHandle) {
        self.impulse_joint_set.remove(handle.0, true);
    }
}
