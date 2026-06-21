use glam::Vec2;
use rapier2d::prelude::*;

use crate::physics::body::PhysicsBody;

use super::{BodyHandle, ColliderHandle, CollisionGroups, PhysicsWorld};

impl PhysicsWorld {
    /// Adds a dynamic box body that responds to gravity.
    pub fn add_dynamic_box(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
        lock_rotation: bool,
    ) -> (BodyHandle, ColliderHandle) {
        self.add_dynamic_box_with_groups(
            position,
            half_w,
            half_h,
            lock_rotation,
            CollisionGroups::all(),
        )
    }

    /// Adds a dynamic box body that responds to gravity with a specified collision group.
    pub fn add_dynamic_box_with_groups(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
        lock_rotation: bool,
        groups: CollisionGroups,
    ) -> (BodyHandle, ColliderHandle) {
        let mut builder = RigidBodyBuilder::dynamic().translation(vector![position.x, position.y]);
        if lock_rotation {
            builder = builder.lock_rotations();
        }
        let handle = self.rigid_body_set.insert(builder.build());
        let collider = ColliderBuilder::cuboid(half_w, half_h)
            .friction(crate::physics::DEFAULT_FRICTION)
            .restitution(crate::physics::DEFAULT_RESTITUTION)
            .collision_groups(groups.to_rapier())
            .build();
        let col_handle =
            self.collider_set
                .insert_with_parent(collider, handle, &mut self.rigid_body_set);
        (
            BodyHandle::from_raw(handle),
            ColliderHandle::from_raw(col_handle),
        )
    }

    /// Adds a static (immovable) floor, wall, or platform.
    pub fn add_static_box(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
    ) -> (BodyHandle, ColliderHandle) {
        self.add_static_box_with_groups(position, half_w, half_h, CollisionGroups::all())
    }

    /// Adds a static (immovable) box body with a specified collision group.
    pub fn add_static_box_with_groups(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
        groups: CollisionGroups,
    ) -> (BodyHandle, ColliderHandle) {
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
        (
            BodyHandle::from_raw(handle),
            ColliderHandle::from_raw(col_handle),
        )
    }

    /// Adds a dynamic circle body that responds to gravity.
    pub fn add_dynamic_circle(
        &mut self,
        position: Vec2,
        radius: f32,
        lock_rotation: bool,
    ) -> (BodyHandle, ColliderHandle) {
        self.add_dynamic_circle_with_groups(position, radius, lock_rotation, CollisionGroups::all())
    }

    /// Adds a dynamic circle body that responds to gravity with a specified collision group.
    pub fn add_dynamic_circle_with_groups(
        &mut self,
        position: Vec2,
        radius: f32,
        lock_rotation: bool,
        groups: CollisionGroups,
    ) -> (BodyHandle, ColliderHandle) {
        let mut builder = RigidBodyBuilder::dynamic().translation(vector![position.x, position.y]);
        if lock_rotation {
            builder = builder.lock_rotations();
        }
        let handle = self.rigid_body_set.insert(builder.build());
        let collider = ColliderBuilder::ball(radius)
            .friction(crate::physics::DEFAULT_FRICTION)
            .restitution(crate::physics::DEFAULT_RESTITUTION)
            .collision_groups(groups.to_rapier())
            .build();
        let col_handle =
            self.collider_set
                .insert_with_parent(collider, handle, &mut self.rigid_body_set);
        (
            BodyHandle::from_raw(handle),
            ColliderHandle::from_raw(col_handle),
        )
    }

    /// Adds a kinematic box body (no gravity response; position controlled manually).
    pub fn add_kinematic_box(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
    ) -> (BodyHandle, ColliderHandle) {
        self.add_kinematic_box_with_groups(position, half_w, half_h, CollisionGroups::all())
    }

    /// Adds a kinematic box body with a specified collision group.
    pub fn add_kinematic_box_with_groups(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
        groups: CollisionGroups,
    ) -> (BodyHandle, ColliderHandle) {
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
        (
            BodyHandle::from_raw(handle),
            ColliderHandle::from_raw(col_handle),
        )
    }

    /// Adds a kinematic circle body (no gravity response; position controlled manually).
    pub fn add_kinematic_circle(
        &mut self,
        position: Vec2,
        radius: f32,
    ) -> (BodyHandle, ColliderHandle) {
        self.add_kinematic_circle_with_groups(position, radius, CollisionGroups::all())
    }

    /// Adds a kinematic circle body with a specified collision group.
    pub fn add_kinematic_circle_with_groups(
        &mut self,
        position: Vec2,
        radius: f32,
        groups: CollisionGroups,
    ) -> (BodyHandle, ColliderHandle) {
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
        (
            BodyHandle::from_raw(handle),
            ColliderHandle::from_raw(col_handle),
        )
    }

    /// Adds a static box sensor that detects overlaps without physical response.
    pub fn add_sensor_box(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
    ) -> (BodyHandle, ColliderHandle) {
        self.add_sensor_box_with_groups(position, half_w, half_h, CollisionGroups::all())
    }

    /// Adds a static box sensor with a specified collision group.
    pub fn add_sensor_box_with_groups(
        &mut self,
        position: Vec2,
        half_w: f32,
        half_h: f32,
        groups: CollisionGroups,
    ) -> (BodyHandle, ColliderHandle) {
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
        (
            BodyHandle::from_raw(handle),
            ColliderHandle::from_raw(col_handle),
        )
    }

    /// Adds a static circle sensor that detects overlaps without physical response.
    pub fn add_sensor_circle(
        &mut self,
        position: Vec2,
        radius: f32,
    ) -> (BodyHandle, ColliderHandle) {
        self.add_sensor_circle_with_groups(position, radius, CollisionGroups::all())
    }

    /// Adds a static circle sensor with a specified collision group.
    pub fn add_sensor_circle_with_groups(
        &mut self,
        position: Vec2,
        radius: f32,
        groups: CollisionGroups,
    ) -> (BodyHandle, ColliderHandle) {
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
        (
            BodyHandle::from_raw(handle),
            ColliderHandle::from_raw(col_handle),
        )
    }

    /// Removes all colliders attached to a body, then deletes the rigid body.
    pub fn remove_body(&mut self, body: &PhysicsBody) {
        // Clean up this body's colliders from the one_way set before removal.
        // Rapier reuses collider handles, so without this cleanup a new collider
        // receiving the same handle would inherit the stale one-way flag and
        // unintentionally behave as a one-way platform.
        let colliders: Vec<_> = self
            .rigid_body_set
            .get(body.rigid_body_handle.0)
            .map(|rb| rb.colliders().to_vec())
            .unwrap_or_default();
        for collider in colliders {
            self.one_way_colliders.remove(&collider);
        }
        self.rigid_body_set.remove(
            body.rigid_body_handle.0,
            &mut self.island_manager,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            true,
        );
        // Refresh the query pipeline immediately so a same-frame `cast_ray*` issued after
        // this removal does not hit the just-removed collider (the pipeline is otherwise
        // only rebuilt inside `step()`). Cheap at this engine's scale (few removals/frame).
        self.query_pipeline.update(&self.collider_set);
    }
}
