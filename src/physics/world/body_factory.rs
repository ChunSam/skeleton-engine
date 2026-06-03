use glam::Vec2;
use rapier2d::prelude::*;

use crate::physics::body::PhysicsBody;

use super::{CollisionGroups, PhysicsWorld};

impl PhysicsWorld {
    /// 중력에 반응하는 동적 박스 바디를 추가한다.
    pub fn add_dynamic_box(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
        lock_rotation: bool,
    ) -> (RigidBodyHandle, ColliderHandle) {
        self.add_dynamic_box_with_groups(
            position,
            half_w,
            half_h,
            lock_rotation,
            CollisionGroups::all(),
        )
    }

    /// 충돌 그룹을 지정해 중력에 반응하는 동적 박스 바디를 추가한다.
    pub fn add_dynamic_box_with_groups(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
        lock_rotation: bool,
        groups: CollisionGroups,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let mut builder = RigidBodyBuilder::dynamic().translation(vector![position.x, position.y]);
        if lock_rotation {
            builder = builder.lock_rotations();
        }
        let handle = self.rigid_body_set.insert(builder.build());
        let collider = ColliderBuilder::cuboid(half_w, half_h)
            .friction(0.3)
            .restitution(0.0)
            .collision_groups(groups.to_rapier())
            .build();
        let col_handle =
            self.collider_set
                .insert_with_parent(collider, handle, &mut self.rigid_body_set);
        (handle, col_handle)
    }

    /// 움직이지 않는 정적 바닥·벽·플랫폼을 추가한다.
    pub fn add_static_box(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
    ) -> (RigidBodyHandle, ColliderHandle) {
        self.add_static_box_with_groups(position, half_w, half_h, CollisionGroups::all())
    }

    /// 충돌 그룹을 지정해 움직이지 않는 정적 박스 바디를 추가한다.
    pub fn add_static_box_with_groups(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
        groups: CollisionGroups,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let body = RigidBodyBuilder::fixed()
            .translation(vector![position.x, position.y])
            .build();
        let handle = self.rigid_body_set.insert(body);
        let collider = ColliderBuilder::cuboid(half_w, half_h)
            .collision_groups(groups.to_rapier())
            .build();
        let col_handle =
            self.collider_set
                .insert_with_parent(collider, handle, &mut self.rigid_body_set);
        (handle, col_handle)
    }

    /// 중력에 반응하는 동적 원형 바디를 추가한다.
    pub fn add_dynamic_circle(
        &mut self,
        position: Vec2,
        radius: f32,
        lock_rotation: bool,
    ) -> (RigidBodyHandle, ColliderHandle) {
        self.add_dynamic_circle_with_groups(position, radius, lock_rotation, CollisionGroups::all())
    }

    /// 충돌 그룹을 지정해 중력에 반응하는 동적 원형 바디를 추가한다.
    pub fn add_dynamic_circle_with_groups(
        &mut self,
        position: Vec2,
        radius: f32,
        lock_rotation: bool,
        groups: CollisionGroups,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let mut builder = RigidBodyBuilder::dynamic().translation(vector![position.x, position.y]);
        if lock_rotation {
            builder = builder.lock_rotations();
        }
        let handle = self.rigid_body_set.insert(builder.build());
        let collider = ColliderBuilder::ball(radius)
            .friction(0.3)
            .restitution(0.0)
            .collision_groups(groups.to_rapier())
            .build();
        let col_handle =
            self.collider_set
                .insert_with_parent(collider, handle, &mut self.rigid_body_set);
        (handle, col_handle)
    }

    /// 키네마틱 박스 바디를 추가한다 (중력 비반응, 수동 위치 제어).
    pub fn add_kinematic_box(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
    ) -> (RigidBodyHandle, ColliderHandle) {
        self.add_kinematic_box_with_groups(position, half_w, half_h, CollisionGroups::all())
    }

    /// 충돌 그룹을 지정해 키네마틱 박스 바디를 추가한다.
    pub fn add_kinematic_box_with_groups(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
        groups: CollisionGroups,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let body = RigidBodyBuilder::kinematic_position_based()
            .translation(vector![position.x, position.y])
            .build();
        let handle = self.rigid_body_set.insert(body);
        let collider = ColliderBuilder::cuboid(half_w, half_h)
            .collision_groups(groups.to_rapier())
            .build();
        let col_handle =
            self.collider_set
                .insert_with_parent(collider, handle, &mut self.rigid_body_set);
        (handle, col_handle)
    }

    /// 키네마틱 원형 바디를 추가한다 (중력 비반응, 수동 위치 제어).
    pub fn add_kinematic_circle(
        &mut self,
        position: Vec2,
        radius: f32,
    ) -> (RigidBodyHandle, ColliderHandle) {
        self.add_kinematic_circle_with_groups(position, radius, CollisionGroups::all())
    }

    /// 충돌 그룹을 지정해 키네마틱 원형 바디를 추가한다.
    pub fn add_kinematic_circle_with_groups(
        &mut self,
        position: Vec2,
        radius: f32,
        groups: CollisionGroups,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let body = RigidBodyBuilder::kinematic_position_based()
            .translation(vector![position.x, position.y])
            .build();
        let handle = self.rigid_body_set.insert(body);
        let collider = ColliderBuilder::ball(radius)
            .collision_groups(groups.to_rapier())
            .build();
        let col_handle =
            self.collider_set
                .insert_with_parent(collider, handle, &mut self.rigid_body_set);
        (handle, col_handle)
    }

    /// 물리 반응 없이 교차만 감지하는 정적 박스 센서를 추가한다.
    pub fn add_sensor_box(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
    ) -> (RigidBodyHandle, ColliderHandle) {
        self.add_sensor_box_with_groups(position, half_w, half_h, CollisionGroups::all())
    }

    /// 충돌 그룹을 지정해 정적 박스 센서를 추가한다.
    pub fn add_sensor_box_with_groups(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
        groups: CollisionGroups,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let body = RigidBodyBuilder::fixed()
            .translation(vector![position.x, position.y])
            .build();
        let handle = self.rigid_body_set.insert(body);
        let collider = ColliderBuilder::cuboid(half_w, half_h)
            .sensor(true)
            .collision_groups(groups.to_rapier())
            .build();
        let col_handle =
            self.collider_set
                .insert_with_parent(collider, handle, &mut self.rigid_body_set);
        (handle, col_handle)
    }

    /// 물리 반응 없이 교차만 감지하는 정적 원형 센서를 추가한다.
    pub fn add_sensor_circle(
        &mut self,
        position: Vec2,
        radius: f32,
    ) -> (RigidBodyHandle, ColliderHandle) {
        self.add_sensor_circle_with_groups(position, radius, CollisionGroups::all())
    }

    /// 충돌 그룹을 지정해 정적 원형 센서를 추가한다.
    pub fn add_sensor_circle_with_groups(
        &mut self,
        position: Vec2,
        radius: f32,
        groups: CollisionGroups,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let body = RigidBodyBuilder::fixed()
            .translation(vector![position.x, position.y])
            .build();
        let handle = self.rigid_body_set.insert(body);
        let collider = ColliderBuilder::ball(radius)
            .sensor(true)
            .collision_groups(groups.to_rapier())
            .build();
        let col_handle =
            self.collider_set
                .insert_with_parent(collider, handle, &mut self.rigid_body_set);
        (handle, col_handle)
    }

    /// 바디와 연결된 모든 콜라이더를 제거한 뒤 강체를 삭제한다.
    pub fn remove_body(&mut self, body: &PhysicsBody) {
        self.rigid_body_set.remove(
            body.rigid_body_handle,
            &mut self.island_manager,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            true,
        );
    }
}
