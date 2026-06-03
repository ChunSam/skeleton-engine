use super::*;
use crate::physics::character::CharacterController;

fn make_world() -> PhysicsWorld {
    PhysicsWorld::new(Vec2::new(0.0, 9.8))
}

#[test]
fn collision_groups_filter_contacts() {
    let mut pw = PhysicsWorld::new(Vec2::ZERO);
    let player = 1 << 0;
    let enemy = 1 << 1;
    let pickup = 1 << 2;

    let (_, player_col) = pw.add_dynamic_box_with_groups(
        Vec2::ZERO,
        0.5,
        0.5,
        false,
        CollisionGroups::new(player, enemy),
    );
    let (_, enemy_col) =
        pw.add_static_box_with_groups(Vec2::ZERO, 0.5, 0.5, CollisionGroups::new(enemy, player));
    let (_, pickup_col) =
        pw.add_static_box_with_groups(Vec2::ZERO, 0.5, 0.5, CollisionGroups::new(pickup, pickup));

    pw.step(1.0 / 60.0);

    assert!(pw.has_contact(player_col));
    assert!(pw.has_contact(enemy_col));
    assert!(
        !pw.has_contact(pickup_col),
        "pickup layer should be ignored by player/enemy filters"
    );
}

#[test]
fn set_collision_groups_updates_existing_collider() {
    let mut pw = PhysicsWorld::new(Vec2::ZERO);
    let (_, col) = pw.add_dynamic_circle(Vec2::ZERO, 0.5, false);
    let groups = CollisionGroups::new(1 << 3, 1 << 4);

    assert!(pw.set_collision_groups(col, groups));
    assert_eq!(pw.collision_groups(col), Some(groups));
}

#[test]
fn collision_groups_layer_bounds_are_checked() {
    assert_eq!(
        CollisionGroups::try_layer(31),
        Some(CollisionGroups::new(1 << 31, CollisionGroups::ALL_BITS))
    );
    assert_eq!(CollisionGroups::try_layer(32), None);
}

#[test]
fn cast_ray_hits_static_box() {
    let mut pw = make_world();
    // Y=0 에 두께 1 바닥
    pw.add_static_box(Vec2::new(0.0, 0.0), 5.0, 0.5);
    pw.step(1.0 / 60.0); // query_pipeline 갱신

    // Y=-5 에서 아래(+Y)로 레이캐스트
    let result = pw.cast_ray(Vec2::new(0.0, -5.0), Vec2::new(0.0, 1.0), 10.0, true);
    assert!(result.is_some(), "바닥에 레이가 맞아야 함");
    let (_, toi) = result.unwrap();
    assert!(toi > 0.0 && toi < 10.0, "toi 범위 확인: {toi}");
}

#[test]
fn cast_ray_misses_when_no_obstacle() {
    let mut pw = make_world();
    pw.add_static_box(Vec2::new(100.0, 0.0), 5.0, 0.5); // 멀리 있음
    pw.step(1.0 / 60.0);

    // X 방향으로 레이캐스트 — 바닥이 Y 방향에 있으므로 맞지 않음
    let result = pw.cast_ray(Vec2::new(0.0, -5.0), Vec2::new(0.0, -1.0), 5.0, true);
    assert!(result.is_none(), "반대 방향은 맞지 않아야 함");
}

#[test]
fn cast_ray_with_normal_returns_correct_normal() {
    let mut pw = make_world();
    pw.add_static_box(Vec2::new(0.0, 0.0), 5.0, 0.5);
    pw.step(1.0 / 60.0);

    let hit = pw.cast_ray_with_normal(Vec2::new(0.0, -5.0), Vec2::new(0.0, 1.0), 20.0, true);
    assert!(hit.is_some());
    let h = hit.unwrap();
    // 위에서 아래로 쐈으므로 법선은 위쪽 (Y < 0 in physics coords)
    assert!(
        h.normal.y < 0.0,
        "법선은 레이 반대 방향이어야 함: {:?}",
        h.normal
    );
}

#[test]
fn add_kinematic_box_creates_body() {
    let mut pw = make_world();
    let (rb, col) = pw.add_kinematic_box(Vec2::new(1.0, 2.0), 0.5, 1.0);
    assert!(pw.rigid_body(rb).is_some());
    assert!(pw.get_collider(col).is_some());
    let body = pw.rigid_body(rb).unwrap();
    assert!(body.is_kinematic(), "키네마틱 바디여야 함");
}

#[test]
fn add_kinematic_circle_creates_body() {
    let mut pw = make_world();
    let (rb, _col) = pw.add_kinematic_circle(Vec2::new(0.0, 0.0), 0.5);
    let body = pw.rigid_body(rb).unwrap();
    assert!(body.is_kinematic());
}

#[test]
fn move_character_grounded_on_floor() {
    let mut pw = make_world();
    // 바닥: Y=2.0, half_h=0.5 → 상단이 Y=1.5
    pw.add_static_box(Vec2::new(0.0, 2.0), 5.0, 0.5);
    // 캐릭터: Y=0.0, half_h=0.5 → 하단이 Y=0.5 (바닥과 1.0 떨어짐)
    let (rb, col) = pw.add_kinematic_box(Vec2::new(0.0, 0.0), 0.4, 0.5);
    pw.step(1.0 / 60.0);

    let mut ctrl = CharacterController::new();
    // 아래로 이동 시도 (픽셀 단위, ppu=1)
    pw.move_character(
        &mut ctrl,
        rb,
        col,
        Vec2::new(0.0, 5.0), // 아래로 이동
        1.0 / 60.0,
        1.0,
    );
    pw.step(1.0 / 60.0);

    assert!(ctrl.grounded, "바닥에 닿으면 grounded=true여야 함");
}

#[test]
fn character_controller_builder_methods() {
    let ctrl = CharacterController::new()
        .with_max_slope_deg(30.0)
        .with_autostep(0.5, 0.2)
        .with_snap_to_ground(0.2);
    assert!((ctrl.max_slope_angle - 30_f32.to_radians()).abs() < 1e-5);
}

#[test]
fn add_distance_joint_creates_and_removes() {
    let mut pw = make_world();
    let (b1, _) = pw.add_dynamic_box(Vec2::new(-1.0, 0.0), 0.4, 0.4, false);
    let (b2, _) = pw.add_dynamic_box(Vec2::new(1.0, 0.0), 0.4, 0.4, false);
    let h = pw.add_distance_joint(b1, b2, Vec2::ZERO, Vec2::ZERO, 2.0);
    assert!(pw.impulse_joint_set.get(h).is_some());
    pw.remove_joint(h);
    assert!(pw.impulse_joint_set.get(h).is_none());
}

#[test]
fn add_revolute_joint_creates() {
    let mut pw = make_world();
    let (b1, _) = pw.add_dynamic_box(Vec2::new(0.0, 0.0), 0.4, 0.4, false);
    let (b2, _) = pw.add_dynamic_box(Vec2::new(1.0, 0.0), 0.4, 0.4, false);
    let h = pw.add_revolute_joint(b1, b2, Vec2::new(0.5, 0.0), Vec2::new(-0.5, 0.0));
    assert!(pw.impulse_joint_set.get(h).is_some());
}

#[test]
fn add_prismatic_joint_creates() {
    let mut pw = make_world();
    let (b1, _) = pw.add_dynamic_box(Vec2::new(0.0, 0.0), 0.4, 0.4, false);
    let (b2, _) = pw.add_dynamic_box(Vec2::new(0.0, 1.0), 0.4, 0.4, false);
    let h = pw.add_prismatic_joint(b1, b2, Vec2::ZERO, Vec2::ZERO, Vec2::new(0.0, 1.0));
    assert!(pw.impulse_joint_set.get(h).is_some());
}

#[test]
fn add_static_from_tilemap_creates_collider_per_matching_tile() {
    use crate::tilemap::{Tilemap, TilemapAtlas};

    // tiles[row][col]: 1 = solid, 2 = one-way, 0 = empty.
    let tiles = vec![vec![1, 0, 2], vec![0, 1, 1]];
    let tilemap = Tilemap::new(TilemapAtlas::new("x", 1, 1), tiles, 32.0, Vec2::ZERO);
    let mut pw = make_world();

    // Solid tiles (id 1) get a collider; everything else is skipped.
    let solids =
        pw.add_static_from_tilemap(&tilemap, 32.0, |id| (id == 1).then(TileCollider::solid));
    assert_eq!(solids.len(), 3, "id==1 타일 3개에만 콜라이더 생성");
    assert_eq!(pw.collider_set.len(), 3);

    // The one-way tile (id 2) is created and auto-registered as one-way.
    let one_ways =
        pw.add_static_from_tilemap(&tilemap, 32.0, |id| (id == 2).then(TileCollider::one_way));
    assert_eq!(one_ways.len(), 1, "id==2 타일 1개");
    assert_eq!(pw.collider_set.len(), 4);
    assert!(
        pw.is_one_way(one_ways[0].1),
        "one-way 콜라이더로 표시되어야"
    );

    // Tile center for [row=0][col=0] with tile_size 32, ppu 32 → world (16,16) → physics (0.5,0.5).
    let body = pw.rigid_body_set.get(solids[0].0).unwrap();
    let t = body.translation();
    assert!((t.x - 0.5).abs() < 1e-5 && (t.y - 0.5).abs() < 1e-5);
}

#[test]
fn one_way_platform_blocks_from_top_passes_from_below_and_on_drop() {
    // 화면 좌표(+y 아래). 플랫폼 y=2.0, half 0.5 → 윗면 1.5 / 밑면 2.5.
    // 캐릭터 half 0.5. ppu=1.0이라 픽셀=물리 단위. move_character가 설정한
    // next_kinematic_translation을 step()으로 실제 이동시킨 뒤 y를 읽어 판정한다.
    let dt = 1.0 / 60.0;
    let run = |start_y: f32, desired_y: f32, one_way: bool, drop: bool| -> f32 {
        let mut pw = PhysicsWorld::new(Vec2::ZERO);
        let (_p, pcol) = pw.add_static_box(Vec2::new(0.0, 2.0), 1.0, 0.5);
        pw.set_one_way(pcol, one_way);
        let (crb, ccol) = pw.add_kinematic_box(Vec2::new(0.0, start_y), 0.5, 0.5);
        let mut ctrl = CharacterController::new();
        if drop {
            ctrl.request_drop();
        }
        // 초기 step으로 query_pipeline에 콜라이더를 등록한다 (move_character가 이를 질의함).
        pw.step(dt);
        pw.move_character(&mut ctrl, crb, ccol, Vec2::new(0.0, desired_y), dt, 1.0);
        pw.step(dt);
        pw.rigid_body(crb).unwrap().translation().y
    };

    // 1) 위에서 내려옴: one-way·솔리드 모두 막아 윗면(center≈1.0)에서 멈춘다.
    assert!(
        run(0.5, 0.6, true, false) < 1.05,
        "one-way도 위에서 내려오면 막아야"
    );
    assert!(run(0.5, 0.6, false, false) < 1.05, "솔리드는 막아야");

    // 2) 아래에서 위로: one-way는 통과(올라감), 솔리드는 밑면(center≈3.0)에서 막힌다.
    assert!(
        run(3.0, -1.5, true, false) < 2.0,
        "one-way는 아래에서 위로 통과해야"
    );
    assert!(
        run(3.0, -1.5, false, false) > 2.8,
        "솔리드는 아래에서 막아야"
    );

    // 3) 윗면에 선 채 drop 요청 + 내려옴: one-way는 통과(내려감), 솔리드는 그대로.
    assert!(
        run(1.0, 0.6, true, true) > 1.4,
        "drop 요청 시 one-way 통과해야"
    );
    assert!(run(1.0, 0.6, false, true) < 1.05, "솔리드는 drop 요청 무시");
}
