use glam::Vec2;
use rapier2d::prelude::*;

use crate::tilemap::Tilemap;

use super::{PhysicsWorld, TileCollider};

impl PhysicsWorld {
    /// 타일맵의 타일마다 정적 박스 콜라이더를 생성한다.
    ///
    /// [`crate::tilemap::TilemapSystem`]이 타일 스프라이트를 배치하는 좌표 규약과
    /// 동일하게 각 타일의 세계 중심을 계산하므로, 렌더되는 타일과 콜라이더가 정확히
    /// 겹친다. `collider_for`가 [`Some`]을 반환하는 타일에만 콜라이더를 만들고,
    /// [`None`]이면 건너뛴다 (빈 칸·장식 타일 등). [`TileCollider`]로 솔리드와
    /// one-way 플랫폼을 타일별로 섞어 한 번에 만들 수 있다 (one-way 타일은
    /// [`PhysicsWorld::set_one_way`]로 자동 표시된다).
    ///
    /// `pixels_per_unit`은 세계(픽셀) 좌표를 물리(미터) 단위로 변환하는 비율이다.
    /// 반환값은 생성된 `(RigidBodyHandle, ColliderHandle)` 목록이다 (스폰 순서: 행→열).
    pub fn add_static_from_tilemap(
        &mut self,
        tilemap: &Tilemap,
        pixels_per_unit: f32,
        mut collider_for: impl FnMut(u32) -> Option<TileCollider>,
    ) -> Vec<(RigidBodyHandle, ColliderHandle)> {
        let ppu = pixels_per_unit.max(f32::MIN_POSITIVE);
        let half = (tilemap.tile_size * 0.5) / ppu;
        let mut handles = Vec::new();
        for (row_idx, row) in tilemap.tiles.iter().enumerate() {
            for (col_idx, &tile_id) in row.iter().enumerate() {
                let Some(kind) = collider_for(tile_id) else {
                    continue;
                };
                let x =
                    tilemap.origin.x + col_idx as f32 * tilemap.tile_size + tilemap.tile_size * 0.5;
                let y =
                    tilemap.origin.y + row_idx as f32 * tilemap.tile_size + tilemap.tile_size * 0.5;
                let pair =
                    self.add_static_box_with_groups(Vec2::new(x, y) / ppu, half, half, kind.groups);
                if kind.one_way {
                    self.one_way_colliders.insert(pair.1);
                }
                handles.push(pair);
            }
        }
        handles
    }
}
