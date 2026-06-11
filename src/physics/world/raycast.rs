use glam::Vec2;
use rapier2d::prelude::*;

use super::{ColliderHandle, PhysicsWorld, RaycastHit};

impl PhysicsWorld {
    /// Simple raycast. Returns the first hit collider handle and toi (ray travel distance multiplier).
    ///
    /// - `origin` / `direction` — in physics units (pixels ÷ pixels_per_unit).
    /// - `max_toi` — maximum ray length multiplier (typically max_distance / direction.length()).
    /// - `solid` — if `true`, a ray starting inside a collider is treated as an intersection.
    pub fn cast_ray(
        &self,
        origin: Vec2,
        direction: Vec2,
        max_toi: f32,
        solid: bool,
    ) -> Option<(ColliderHandle, f32)> {
        let ray = Ray::new(
            point![origin.x, origin.y],
            vector![direction.x, direction.y],
        );
        self.query_pipeline
            .cast_ray(
                &self.rigid_body_set,
                &self.collider_set,
                &ray,
                max_toi,
                solid,
                QueryFilter::default(),
            )
            .map(|(handle, toi)| (ColliderHandle::from_raw(handle), toi))
    }

    /// Raycast that returns a `RaycastHit` including the hit point and normal vector.
    ///
    /// Uses physics units. To work in pixel units, divide `origin` and `direction` by
    /// `pixels_per_unit` before passing them in, then multiply the returned `RaycastHit::point`
    /// by `pixels_per_unit` to convert back.
    pub fn cast_ray_with_normal(
        &self,
        origin: Vec2,
        direction: Vec2,
        max_toi: f32,
        solid: bool,
    ) -> Option<RaycastHit> {
        let ray = Ray::new(
            point![origin.x, origin.y],
            vector![direction.x, direction.y],
        );
        self.query_pipeline
            .cast_ray_and_get_normal(
                &self.rigid_body_set,
                &self.collider_set,
                &ray,
                max_toi,
                solid,
                QueryFilter::default(),
            )
            .map(|(handle, intersection)| {
                let hit_point = ray.point_at(intersection.time_of_impact);
                RaycastHit {
                    collider_handle: ColliderHandle::from_raw(handle),
                    point: Vec2::new(hit_point.x, hit_point.y),
                    normal: Vec2::new(intersection.normal.x, intersection.normal.y),
                    toi: intersection.time_of_impact,
                }
            })
    }
}
