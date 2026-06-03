use glam::Vec2;
use rapier2d::prelude::*;

use crate::physics::character::CharacterController;

use super::PhysicsWorld;

impl PhysicsWorld {
    /// `CharacterController`를 이용해 충돌 해결 후 키네마틱 바디를 이동한다.
    ///
    /// `desired_translation` — **픽셀 단위** 이동 벡터.
    /// 내부에서 `pixels_per_unit`으로 물리 단위로 변환하고,
    /// 충돌 해결 후 `set_next_kinematic_translation()`으로 바디에 적용한다.
    /// 다음 `step()` 호출 시 해당 위치로 이동한다.
    ///
    /// `controller.grounded`가 갱신되므로 이 메서드를 `PhysicsSystem::run()` 이전에 호출해야 한다.
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

        // 콜라이더 위치와 shape를 먼저 복사해 borrow 분리
        let (col_pos, shape_type) = match self.collider_set.get(col_handle) {
            Some(c) => (*c.position(), c.shape().shape_type()),
            None => return,
        };

        // shape를 collider_set에서 재획득 (두 번째 불변 참조 — Rust 허용)
        let shape = match self.collider_set.get(col_handle) {
            Some(c) => c.shape(),
            None => return,
        };
        let _ = shape_type; // 타입 힌트용으로 저장, 실제 사용은 shape 참조

        // one-way 플랫폼 처리: drop 윈도를 갱신하고, 이번 프레임의 통과 판정값을 미리 구한다.
        // 화면 좌표(Y+는 아래)이므로 "내려옴" = desired.y > 0, 캐릭터 밑면 = AABB.maxs.y.
        let drop_active = controller.drop_timer > 0.0;
        controller.drop_timer = (controller.drop_timer - dt).max(0.0);
        let moving_down = desired.y > 1e-6;
        let char_bottom = shape.compute_aabb(&col_pos).maxs.y;
        // 윗면보다 살짝(스킨 두께) 위까지는 "위에 있음"으로 간주해 안정적으로 착지/접지한다.
        const ONE_WAY_TOLERANCE: f32 = 0.05;
        let one_way = &self.one_way_colliders;
        let predicate = move |handle: ColliderHandle, collider: &Collider| -> bool {
            if !one_way.contains(&handle) {
                return true; // 일반 솔리드: 항상 충돌.
            }
            if drop_active || !moving_down {
                return false; // drop 중이거나 상승 중 → 통과.
            }
            // 내려오는 중 + 캐릭터 밑면이 플랫폼 윗면 위(또는 거의 닿음)일 때만 막는다.
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

        // 바디 현재 위치 + 이동 벡터로 next_kinematic_translation 설정
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
