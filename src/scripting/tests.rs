use super::context::{
    set_script_ctx, take_script_ctx, BbEntry, ScriptCommands, ScriptCtx, SteeringCmd,
};
use super::*;
use crate::ecs::Entity;
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
