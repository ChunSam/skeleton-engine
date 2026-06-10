use glam::Vec2;
use rapier2d::prelude::*;

use crate::physics::character::CharacterController;

use super::PhysicsWorld;

impl PhysicsWorld {
    /// Moves a kinematic body using `CharacterController` with collision resolution.
    ///
    /// `desired_translation` — movement vector in **pixel units**.
    /// Internally converts to physics units via `pixels_per_unit`, resolves collisions,
    /// then applies the result with `set_next_kinematic_translation()`.
    /// The body moves to that position on the next `step()` call.
    ///
    /// `controller.grounded` is updated here, so call this method before `PhysicsSystem::run()`.
    pub fn move_character(
        &mut self,
        controller: &mut CharacterController,
        body_handle: RigidBodyHandle,
        col_handle: ColliderHandle,
        desired_translation: Vec2,
        dt: f32,
        pixels_per_unit: f32,
    ) {
        let ppu = pixels_per_unit;
        let desired = vector![desired_translation.x / ppu, desired_translation.y / ppu];

        // Make max_slope_angle authoritative: sync the public field into the Rapier
        // controller before every move so a direct field assignment takes effect immediately.
        controller.inner.max_slope_climb_angle = controller.max_slope_angle;
        controller.inner.min_slope_slide_angle = controller.max_slope_angle;

        // Copy collider position first to split the borrow, then re-acquire the shape reference.
        let col_pos = match self.collider_set.get(col_handle) {
            Some(c) => *c.position(),
            None => return,
        };

        // Re-acquire shape from collider_set (a second shared reference — allowed by Rust)
        let shape = match self.collider_set.get(col_handle) {
            Some(c) => c.shape(),
            None => return,
        };

        // One-way platform handling: update the drop window and pre-compute this frame's pass-through values.
        // Screen coordinates (Y+ is down): "moving down" = desired.y > 0, character bottom = AABB.maxs.y.
        let drop_active = controller.drop_timer > 0.0;
        controller.drop_timer = (controller.drop_timer - dt).max(0.0);
        let moving_down = desired.y > 1e-6;
        let char_bottom = shape.compute_aabb(&col_pos).maxs.y;
        // Treat anything slightly above the top surface (within skin thickness) as "above" for stable landing.
        const ONE_WAY_TOLERANCE: f32 = 0.05;
        let one_way = &self.one_way_colliders;
        let predicate = move |handle: ColliderHandle, collider: &Collider| -> bool {
            if !one_way.contains(&handle) {
                return true; // Regular solid: always collide.
            }
            if drop_active || !moving_down {
                return false; // Dropping or moving up → pass through.
            }
            // Block only when moving down AND the character bottom is above (or nearly touching) the platform top.
            let platform_top = collider.compute_aabb().mins.y;
            char_bottom <= platform_top + ONE_WAY_TOLERANCE
        };

        let output = controller.inner.move_shape(
            dt,
            &self.rigid_body_set,
            &self.collider_set,
            &self.query_pipeline,
            shape,
            &col_pos,
            desired,
            QueryFilter::default()
                .exclude_collider(col_handle)
                .predicate(&predicate),
            |_| {},
        );

        controller.grounded = output.grounded;

        // Set next_kinematic_translation to current body position + movement vector
        let body_t = self
            .rigid_body_set
            .get(body_handle)
            .map(|b| *b.translation())
            .unwrap_or_default();
        let new_t = body_t + output.translation;
        if let Some(body) = self.rigid_body_set.get_mut(body_handle) {
            body.set_next_kinematic_translation(new_t);
        }
    }
}
