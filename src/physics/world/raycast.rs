use glam::Vec2;
use rapier2d::prelude::*;

use super::{PhysicsWorld, RaycastHit};

impl PhysicsWorld {
    /// 단순 레이캐스트. 최초 충돌 콜라이더 핸들과 toi(레이 이동 거리 배율)를 반환한다.
    ///
    /// - `origin` / `direction` — 물리 단위 (픽셀 ÷ pixels_per_unit).
    /// - `max_toi` — 최대 레이 길이 배율 (보통 최대 거리 / direction.length()).
    /// - `solid` — `true`이면 레이 시작점이 콜라이더 내부일 때도 교차로 처리.
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
        self.query_pipeline.cast_ray(
            &self.rigid_body_set,
            &self.collider_set,
            &ray,
            max_toi,
            solid,
            QueryFilter::default(),
        )
    }

    /// 레이캐스트 — 충돌 지점과 법선 벡터를 포함한 `RaycastHit`를 반환한다.
    ///
    /// 물리 단위 기준. 픽셀 단위를 쓰려면 `origin`과 `direction`을 `pixels_per_unit`으로 나눠 전달하고,
    /// 반환된 `RaycastHit::point`에 `pixels_per_unit`을 곱해 변환한다.
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
                    collider_handle: handle,
                    point: Vec2::new(hit_point.x, hit_point.y),
                    normal: Vec2::new(intersection.normal.x, intersection.normal.y),
                    toi: intersection.time_of_impact,
                }
            })
    }
}
