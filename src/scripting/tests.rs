use super::context::{
    set_script_ctx, take_script_ctx, BbEntry, ScriptCommands, ScriptCtx, SteeringCmd,
};
use super::*;
use crate::asset::AssetServer;
use crate::components::Transform;
use crate::ecs::{Entity, System, World};
use crate::steering::{Arrive, Flee, Seek, SteeringVelocity, Wander};
use glam::Vec2;
use rhai::Scope;
use std::collections::HashMap;

fn test_entity() -> Entity {
    Entity::from_raw_parts(7, 3)
}

fn make_engine() -> ScriptingSystem {
    ScriptingSystem::new()
}

/// Sets the thread_local ctx, evaluates `script`, then recovers and returns the
/// ctx (with the buffers the script wrote into).
fn eval_with_ctx(sys: &ScriptingSystem, ctx: ScriptCtx, script: &str) -> ScriptCtx {
    let ast = sys.engine.compile(script).unwrap();
    let mut scope = Scope::new();
    set_script_ctx(ctx);
    let _ = sys
        .engine
        .eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast);
    take_script_ctx().expect("ctx was set above")
}

fn empty_ctx(entity: Entity) -> ScriptCtx {
    ScriptCtx {
        entity,
        cmd_buf: ScriptCommands::default(),
        bb_buf: Vec::new(),
        steer_buf: None,
        bb_snap: HashMap::new(),
    }
}

#[test]
fn scripting_limits_default_is_conservative() {
    // #30: defaults should bound strings/arrays/maps/recursion/expr-depth so a
    // trusted-local script can't accidentally run away, while staying generous.
    let limits = ScriptingLimits::default();
    assert_eq!(limits.max_operations, 1_000_000);
    assert!(limits.max_string_size > 0, "string size must be bounded");
    assert!(limits.max_array_size > 0, "array size must be bounded");
    assert!(limits.max_map_size > 0, "map size must be bounded");
    assert!(
        (1..=256).contains(&limits.max_call_levels),
        "call depth must be bounded but usable"
    );
    assert!(
        (1..=512).contains(&limits.max_expr_depth),
        "expr depth must be bounded but usable"
    );

    // The engine builds with these limits applied (no panic from the setters).
    let _sys = ScriptingSystem::with_limits(limits);
}

#[test]
fn scripting_spawn_entity_works() {
    let sys = make_engine();
    let ctx = eval_with_ctx(
        &sys,
        empty_ctx(test_entity()),
        "let id = spawn_entity(); spawn_entity();",
    );
    assert_eq!(ctx.cmd_buf.spawn_count, 2);
}

#[test]
fn scripting_bb_roundtrip() {
    let sys = make_engine();
    let mut ctx = empty_ctx(test_entity());
    // Pre-populate the snapshot with a value (for get testing)
    ctx.bb_snap.insert(
        "score".to_string(),
        BbEntry::Float("score".to_string(), 42.0),
    );

    let ctx = eval_with_ctx(
        &sys,
        ctx,
        r#"
            bb_set_bool("active", true);
            bb_set_int("hp", 99);
            let s = bb_get_float("score");
        "#,
    );

    let changes = &ctx.bb_buf;
    assert!(changes
        .iter()
        .any(|e| matches!(e, BbEntry::Bool(k, true) if k == "active")));
    assert!(changes
        .iter()
        .any(|e| matches!(e, BbEntry::Int(k, 99) if k == "hp")));
}

#[test]
fn scripting_two_entities_no_buffer_cross_contamination() {
    let sys = make_engine();

    // Run entity A
    let ctx_a = eval_with_ctx(
        &sys,
        empty_ctx(test_entity()),
        r#"bb_set_bool("flag_a", true);"#,
    );

    // Run entity B
    let ctx_b = eval_with_ctx(
        &sys,
        empty_ctx(Entity::from_raw_parts(8, 4)),
        r#"bb_set_bool("flag_b", true);"#,
    );

    // A's buffer should contain only flag_a; B's buffer should contain only flag_b
    let a = &ctx_a.bb_buf;
    let b = &ctx_b.bb_buf;
    assert!(a
        .iter()
        .any(|e| matches!(e, BbEntry::Bool(k, _) if k == "flag_a")));
    assert!(!a
        .iter()
        .any(|e| matches!(e, BbEntry::Bool(k, _) if k == "flag_b")));
    assert!(b
        .iter()
        .any(|e| matches!(e, BbEntry::Bool(k, _) if k == "flag_b")));
    assert!(!b
        .iter()
        .any(|e| matches!(e, BbEntry::Bool(k, _) if k == "flag_a")));
}

#[test]
fn scripting_entity_identity_functions_return_context_entity() {
    let sys = make_engine();
    let ctx = eval_with_ctx(
        &sys,
        empty_ctx(test_entity()),
        r#"
            bb_set_int("idx", entity_index());
            bb_set_int("gen", entity_generation());
        "#,
    );

    let changes = &ctx.bb_buf;
    assert!(changes
        .iter()
        .any(|e| matches!(e, BbEntry::Int(k, 7) if k == "idx")));
    assert!(changes
        .iter()
        .any(|e| matches!(e, BbEntry::Int(k, 3) if k == "gen")));
}

#[test]
fn scripting_self_despawn_can_use_context_identity() {
    let sys = make_engine();
    let entity = test_entity();
    let ctx = eval_with_ctx(
        &sys,
        empty_ctx(entity),
        "despawn_entity(entity_index(), entity_generation());",
    );

    assert_eq!(ctx.cmd_buf.despawn, vec![entity]);
}

#[test]
fn scripting_arrive_at_sets_arrive_cmd() {
    let sys = make_engine();
    let ctx = eval_with_ctx(
        &sys,
        empty_ctx(test_entity()),
        "arrive_at(300.0, 200.0, 150.0, 80.0, 10.0);",
    );
    match ctx.steer_buf {
        Some(SteeringCmd::Arrive {
            tx,
            ty,
            speed,
            slow_radius,
            stop_radius,
        }) => {
            assert!((tx - 300.0).abs() < 1e-4, "tx mismatch: {tx}");
            assert!((ty - 200.0).abs() < 1e-4, "ty mismatch: {ty}");
            assert!((speed - 150.0).abs() < 1e-4, "speed mismatch: {speed}");
            assert!(
                (slow_radius - 80.0).abs() < 1e-4,
                "slow_radius mismatch: {slow_radius}"
            );
            assert!(
                (stop_radius - 10.0).abs() < 1e-4,
                "stop_radius mismatch: {stop_radius}"
            );
        }
        other => panic!("expected SteeringCmd::Arrive, got {other:?}"),
    }
}

#[test]
fn scripting_wander_sets_wander_cmd() {
    let sys = make_engine();
    let ctx = eval_with_ctx(&sys, empty_ctx(test_entity()), "wander(120.0, 2.5);");
    match ctx.steer_buf {
        Some(SteeringCmd::Wander {
            speed,
            change_interval,
        }) => {
            assert!((speed - 120.0).abs() < 1e-4, "speed mismatch: {speed}");
            assert!(
                (change_interval - 2.5).abs() < 1e-4,
                "change_interval mismatch: {change_interval}"
            );
        }
        other => panic!("expected SteeringCmd::Wander, got {other:?}"),
    }
}

#[test]
fn scripting_arrive_at_overwrites_previous_steer_cmd() {
    // Only one steering command per frame — the last one wins.
    let sys = make_engine();
    let ctx = eval_with_ctx(
        &sys,
        empty_ctx(test_entity()),
        r#"
            seek_target(100.0, 0.0, 50.0);
            arrive_at(200.0, 100.0, 80.0, 40.0, 5.0);
        "#,
    );
    assert!(
        matches!(ctx.steer_buf, Some(SteeringCmd::Arrive { .. })),
        "Arrive should win over the earlier Seek"
    );
}

#[test]
fn scripting_wander_overwrites_previous_steer_cmd() {
    let sys = make_engine();
    let ctx = eval_with_ctx(
        &sys,
        empty_ctx(test_entity()),
        r#"
            seek_target(100.0, 0.0, 50.0);
            wander(90.0, 1.0);
        "#,
    );
    assert!(
        matches!(ctx.steer_buf, Some(SteeringCmd::Wander { .. })),
        "Wander should win over the earlier Seek"
    );
}

// ─── World-level exclusivity tests ───────────────────────────────────────────
//
// These tests exercise the full ScriptingSystem::run path so that component
// mutations on the World are verified — not just the steer_buf in the context.
//
// Helper: build a minimal World+entity with a ScriptRunner wired to `source`.
fn make_world_with_script(source: &str) -> (World, Entity, ScriptingSystem) {
    let mut world = World::new();
    let mut assets = AssetServer::new();
    let handle = assets.load_script_inline(format!("inline::{source}"), source);
    world.insert_resource(assets);
    let e = world.spawn();
    world.add_component(
        e,
        Transform {
            position: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation: 0.0,
            z: 0.0,
        },
    );
    world.add_component(e, ScriptRunner::new(handle));
    let sys = ScriptingSystem::new();
    (world, e, sys)
}

/// (a) wander-then-arrive: after a frame with wander() followed by arrive_at() via
/// two separate script runs (simulating a previous-frame Wander + this-frame Arrive),
/// only Arrive should remain and no Wander component should be on the entity.
#[test]
fn steering_exclusivity_wander_then_arrive_removes_wander() {
    // First frame: entity runs wander() → Wander component added
    let wander_src = "fn on_update(dt) { wander(90.0, 1.5); }";
    let (mut world, e, mut sys) = make_world_with_script(wander_src);
    sys.run(&mut world, 0.016);
    assert!(
        world.get::<Wander>(e).is_some(),
        "Wander component should be present after wander() script"
    );

    // Second frame: swap to arrive_at() script
    // Simulate switching by overwriting the ScriptRunner with an arrive script.
    let mut assets = world.remove_resource::<AssetServer>().unwrap();
    let arrive_handle = assets.load_script_inline(
        "inline::arrive",
        "fn on_update(dt) { arrive_at(500.0, 300.0, 120.0, 60.0, 8.0); }",
    );
    world.insert_resource(assets);
    if let Some(runner) = world.get_mut::<ScriptRunner>(e) {
        runner.script = arrive_handle;
        runner.started = false;
    }
    sys.run(&mut world, 0.016);

    assert!(
        world.get::<Arrive>(e).is_some(),
        "Arrive component should be present after arrive_at() script"
    );
    assert!(
        world.get::<Wander>(e).is_none(),
        "Wander component must be removed when arrive_at() is called"
    );
    assert!(world.get::<Seek>(e).is_none(), "Seek must not be present");
    assert!(world.get::<Flee>(e).is_none(), "Flee must not be present");
}

/// (b) arrive-then-seek: after an Arrive frame, switching to seek_target() removes Arrive.
#[test]
fn steering_exclusivity_arrive_then_seek_removes_arrive() {
    // First frame: arrive_at()
    let (mut world, e, mut sys) =
        make_world_with_script("fn on_update(dt) { arrive_at(100.0, 0.0, 80.0, 40.0, 5.0); }");
    sys.run(&mut world, 0.016);
    assert!(
        world.get::<Arrive>(e).is_some(),
        "Arrive should be set after first frame"
    );

    // Second frame: seek_target()
    let mut assets = world.remove_resource::<AssetServer>().unwrap();
    let seek_handle = assets.load_script_inline(
        "inline::seek",
        "fn on_update(dt) { seek_target(200.0, 150.0, 100.0); }",
    );
    world.insert_resource(assets);
    if let Some(runner) = world.get_mut::<ScriptRunner>(e) {
        runner.script = seek_handle;
        runner.started = false;
    }
    sys.run(&mut world, 0.016);

    assert!(
        world.get::<Seek>(e).is_some(),
        "Seek should be present after seek_target()"
    );
    assert!(
        world.get::<Arrive>(e).is_none(),
        "Arrive must be removed when seek_target() takes over"
    );
    assert!(
        world.get::<Wander>(e).is_none(),
        "Wander must not be present"
    );
    assert!(world.get::<Flee>(e).is_none(), "Flee must not be present");
}

/// (c) stop_steering removes all steering components and the entity stays stopped.
/// On the frame after stop_steering() the entity must have no Seek/Flee/Arrive/Wander
/// and SteeringVelocity must be zero.
#[test]
fn steering_exclusivity_stop_clears_all_and_stays_stopped() {
    // Frame 1: wander() → Wander + SteeringVelocity added
    let (mut world, e, mut sys) = make_world_with_script("fn on_update(dt) { wander(90.0, 1.5); }");
    sys.run(&mut world, 0.016);
    assert!(
        world.get::<Wander>(e).is_some(),
        "Wander present after frame 1"
    );

    // Frame 2: stop_steering()
    let mut assets = world.remove_resource::<AssetServer>().unwrap();
    let stop_handle =
        assets.load_script_inline("inline::stop", "fn on_update(dt) { stop_steering(); }");
    world.insert_resource(assets);
    if let Some(runner) = world.get_mut::<ScriptRunner>(e) {
        runner.script = stop_handle;
        runner.started = false;
    }
    sys.run(&mut world, 0.016);

    assert!(world.get::<Seek>(e).is_none(), "Seek absent after stop");
    assert!(world.get::<Flee>(e).is_none(), "Flee absent after stop");
    assert!(world.get::<Arrive>(e).is_none(), "Arrive absent after stop");
    assert!(world.get::<Wander>(e).is_none(), "Wander absent after stop");
    if let Some(sv) = world.get::<SteeringVelocity>(e) {
        assert!(
            sv.velocity.length_squared() < 1e-8,
            "SteeringVelocity must be zeroed by stop_steering()"
        );
    }

    // Frame 3: no steering call — entity should remain stopped (no components re-fire).
    sys.run(&mut world, 0.016);
    assert!(
        world.get::<Wander>(e).is_none(),
        "Wander stays absent on frame 3 (no re-apply)"
    );
    if let Some(sv) = world.get::<SteeringVelocity>(e) {
        assert!(
            sv.velocity.length_squared() < 1e-8,
            "SteeringVelocity stays zero on frame 3"
        );
    }
}

/// (d) wander-then-wander: re-calling wander() preserves existing timer/current_dir.
#[test]
fn steering_wander_reapply_preserves_timer_and_dir() {
    let (mut world, e, mut sys) = make_world_with_script("fn on_update(dt) { wander(90.0, 1.5); }");

    // Frame 1: Wander added fresh.
    sys.run(&mut world, 0.016);
    // Manually advance timer to a non-zero state to detect preservation.
    if let Some(w) = world.get_mut::<Wander>(e) {
        w.timer = 0.7;
        w.current_dir = Vec2::new(0.5, 0.866);
    }

    // Frame 2: same wander() call; timer/dir must survive.
    sys.run(&mut world, 0.016);

    let w = world
        .get::<Wander>(e)
        .expect("Wander must still be present after re-apply");
    // Timer is ticked by SteeringSystem (not running here), so it should still be ≈ 0.7
    // (ScriptingSystem only calls add_component which preserves the existing value via
    // the merge-on-reapply path).
    assert!(
        w.timer > 0.5,
        "timer should be preserved (~0.7), got {}",
        w.timer
    );
    assert!(
        (w.current_dir - Vec2::new(0.5, 0.866)).length() < 1e-3,
        "current_dir should be preserved, got {:?}",
        w.current_dir
    );
}
