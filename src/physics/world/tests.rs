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
    // Floor of thickness 1 at Y=0
    pw.add_static_box(Vec2::new(0.0, 0.0), 5.0, 0.5);
    pw.step(1.0 / 60.0); // update query_pipeline

    // Raycast downward (+Y) from Y=-5
    let result = pw.cast_ray(Vec2::new(0.0, -5.0), Vec2::new(0.0, 1.0), 10.0, true);
    assert!(result.is_some(), "ray should hit the floor");
    let (_, toi) = result.unwrap();
    assert!(toi > 0.0 && toi < 10.0, "toi range check: {toi}");
}

#[test]
fn cast_ray_misses_when_no_obstacle() {
    let mut pw = make_world();
    pw.add_static_box(Vec2::new(100.0, 0.0), 5.0, 0.5); // far away
    pw.step(1.0 / 60.0);

    // Raycast along X — floor is in the Y direction so it should not be hit
    let result = pw.cast_ray(Vec2::new(0.0, -5.0), Vec2::new(0.0, -1.0), 5.0, true);
    assert!(result.is_none(), "opposite direction should not hit");
}

#[test]
fn cast_ray_with_normal_returns_correct_normal() {
    let mut pw = make_world();
    pw.add_static_box(Vec2::new(0.0, 0.0), 5.0, 0.5);
    pw.step(1.0 / 60.0);

    let hit = pw.cast_ray_with_normal(Vec2::new(0.0, -5.0), Vec2::new(0.0, 1.0), 20.0, true);
    assert!(hit.is_some());
    let h = hit.unwrap();
    // Shot from above downward, so the normal points upward (Y < 0 in physics coords)
    assert!(
        h.normal.y < 0.0,
        "normal should face opposite to the ray direction: {:?}",
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
    assert!(body.is_kinematic(), "should be a kinematic body");
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
    // Floor: Y=2.0, half_h=0.5 → top surface at Y=1.5
    pw.add_static_box(Vec2::new(0.0, 2.0), 5.0, 0.5);
    // Character: Y=0.0, half_h=0.5 → bottom at Y=0.5 (1.0 above the floor)
    let (rb, col) = pw.add_kinematic_box(Vec2::new(0.0, 0.0), 0.4, 0.5);
    pw.step(1.0 / 60.0);

    let mut ctrl = CharacterController::new();
    // Attempt to move downward (pixel units, ppu=1)
    pw.move_character(
        &mut ctrl,
        rb,
        col,
        Vec2::new(0.0, 5.0), // move downward
        1.0 / 60.0,
        1.0,
    );
    pw.step(1.0 / 60.0);

    assert!(
        ctrl.grounded,
        "grounded should be true when touching the floor"
    );
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
    assert!(pw.impulse_joint_set.get(h.0).is_some());
    pw.remove_joint(h);
    assert!(pw.impulse_joint_set.get(h.0).is_none());
}

#[test]
fn add_revolute_joint_creates() {
    let mut pw = make_world();
    let (b1, _) = pw.add_dynamic_box(Vec2::new(0.0, 0.0), 0.4, 0.4, false);
    let (b2, _) = pw.add_dynamic_box(Vec2::new(1.0, 0.0), 0.4, 0.4, false);
    let h = pw.add_revolute_joint(b1, b2, Vec2::new(0.5, 0.0), Vec2::new(-0.5, 0.0));
    assert!(pw.impulse_joint_set.get(h.0).is_some());
}

#[test]
fn add_prismatic_joint_creates() {
    let mut pw = make_world();
    let (b1, _) = pw.add_dynamic_box(Vec2::new(0.0, 0.0), 0.4, 0.4, false);
    let (b2, _) = pw.add_dynamic_box(Vec2::new(0.0, 1.0), 0.4, 0.4, false);
    let h = pw.add_prismatic_joint(b1, b2, Vec2::ZERO, Vec2::ZERO, Vec2::new(0.0, 1.0));
    assert!(pw.impulse_joint_set.get(h.0).is_some());
}

#[test]
fn distance_joint_holds_rest_length_under_gravity() {
    // Attach a dynamic body to a static body (at origin) via a distance joint. Gravity (+y)
    // pulls it downward, but the joint holds rest_length so the distance from origin should
    // stay ~2.0.
    let mut pw = make_world(); // gravity (0, 9.8)
    let (anchor, _) = pw.add_static_box(Vec2::ZERO, 0.1, 0.1);
    let (ball, _) = pw.add_dynamic_box(Vec2::new(2.0, 0.0), 0.2, 0.2, false);
    pw.add_distance_joint(anchor, ball, Vec2::ZERO, Vec2::ZERO, 2.0);

    for _ in 0..240 {
        pw.step(1.0 / 60.0);
    }

    let p = pw.rigid_body(ball).unwrap().translation();
    let dist = (p.x * p.x + p.y * p.y).sqrt();
    assert!(
        (dist - 2.0).abs() < 0.3,
        "distance joint should maintain rest_length(2.0): {dist}"
    );
    assert!(
        p.y > 0.5,
        "should hang downward (+y) under gravity: {}",
        p.y
    );
}

#[test]
fn revolute_joint_keeps_anchor_pinned_under_gravity() {
    // Pin a dynamic body to a static body (at origin) with a revolute joint. The body swings
    // freely around the pivot (world origin), so it swings downward but its center always
    // stays within the arm length (1.0) of the pivot (it would fall indefinitely if the joint
    // broke).
    let mut pw = make_world();
    let (anchor, _) = pw.add_static_box(Vec2::ZERO, 0.1, 0.1);
    let (arm, _) = pw.add_dynamic_box(Vec2::new(1.0, 0.0), 0.4, 0.1, false);
    // Static body local (0,0)=origin, arm local (-1,0)=left end of arm=world origin → pivot=origin.
    pw.add_revolute_joint(anchor, arm, Vec2::ZERO, Vec2::new(-1.0, 0.0));

    for _ in 0..240 {
        pw.step(1.0 / 60.0);
    }

    let p = pw.rigid_body(arm).unwrap().translation();
    let dist = (p.x * p.x + p.y * p.y).sqrt();
    assert!(
        dist < 1.2,
        "revolute should pin the body within arm length of the pivot: {dist}"
    );
    assert!(
        p.y > 0.5,
        "should swing below the pivot under gravity: {}",
        p.y
    );
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
    assert_eq!(
        solids.len(),
        3,
        "collider should be created only for the 3 id==1 tiles"
    );
    assert_eq!(pw.collider_set.len(), 3);

    // The one-way tile (id 2) is created and auto-registered as one-way.
    let one_ways =
        pw.add_static_from_tilemap(&tilemap, 32.0, |id| (id == 2).then(TileCollider::one_way));
    assert_eq!(one_ways.len(), 1, "1 tile with id==2");
    assert_eq!(pw.collider_set.len(), 4);
    assert!(
        pw.is_one_way(one_ways[0].1),
        "should be marked as a one-way collider"
    );

    // Tile center for [row=0][col=0] with tile_size 32, ppu 32 → world (16,16) → physics (0.5,0.5).
    let body = pw.rigid_body_set.get(solids[0].0 .0).unwrap();
    let t = body.translation();
    assert!((t.x - 0.5).abs() < 1e-5 && (t.y - 0.5).abs() < 1e-5);
}

#[test]
fn one_way_platform_blocks_from_top_passes_from_below_and_on_drop() {
    // Screen coords (+y down). Platform y=2.0, half 0.5 → top surface 1.5 / bottom 2.5.
    // Character half 0.5. ppu=1.0 so pixels == physics units. move_character sets
    // next_kinematic_translation; step() applies it, then we read y to check the result.
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
        // Initial step registers colliders in the query_pipeline (move_character queries it).
        pw.step(dt);
        pw.move_character(&mut ctrl, crb, ccol, Vec2::new(0.0, desired_y), dt, 1.0);
        pw.step(dt);
        pw.rigid_body(crb).unwrap().translation().y
    };

    // 1) Descending from above: both one-way and solid block, stopping at the top surface (center≈1.0).
    assert!(
        run(0.5, 0.6, true, false) < 1.05,
        "one-way should also block when descending from above"
    );
    assert!(run(0.5, 0.6, false, false) < 1.05, "solid should block");

    // 2) Ascending from below: one-way passes through (goes up), solid blocks at the bottom surface (center≈3.0).
    assert!(
        run(3.0, -1.5, true, false) < 2.0,
        "one-way should pass through when ascending from below"
    );
    assert!(
        run(3.0, -1.5, false, false) > 2.8,
        "solid should block from below"
    );

    // 3) Standing on top and drop-requesting while moving down: one-way passes through, solid stays.
    assert!(
        run(1.0, 0.6, true, true) > 1.4,
        "one-way should be passable when drop is requested"
    );
    assert!(
        run(1.0, 0.6, false, true) < 1.05,
        "solid should ignore drop request"
    );
}
