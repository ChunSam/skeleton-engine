use super::context::{clear_script_ctx, set_script_ctx, BbEntry, ScriptCommands, ScriptCtx};
use super::*;
use crate::ecs::Entity;
use rhai::Scope;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn test_entity() -> Entity {
    Entity::from_raw_parts(7, 3)
}

fn make_engine() -> ScriptingSystem {
    ScriptingSystem::new()
}

fn eval_with_ctx(sys: &ScriptingSystem, ctx: ScriptCtx, script: &str) {
    let ast = sys.engine.compile(script).unwrap();
    let mut scope = Scope::new();
    set_script_ctx(ctx);
    let _ = sys
        .engine
        .eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast);
    clear_script_ctx();
}

#[test]
fn scripting_spawn_entity_works() {
    let sys = make_engine();
    let cmd_buf = Arc::new(Mutex::new(ScriptCommands::default()));
    let ctx = ScriptCtx {
        entity: test_entity(),
        cmd_buf: Arc::clone(&cmd_buf),
        bb_buf: Arc::new(Mutex::new(Vec::new())),
        steer_buf: Arc::new(Mutex::new(None)),
        bb_snap: Arc::new(Mutex::new(HashMap::new())),
    };
    eval_with_ctx(&sys, ctx, "let id = spawn_entity(); spawn_entity();");
    assert_eq!(cmd_buf.lock().unwrap().spawn_count, 2);
}

#[test]
fn scripting_bb_roundtrip() {
    let sys = make_engine();
    let bb_buf = Arc::new(Mutex::new(Vec::new()));
    let bb_snap = Arc::new(Mutex::new(HashMap::new()));
    // 사전에 스냅샷에 값 넣기 (get 테스트용)
    bb_snap.lock().unwrap().insert(
        "score".to_string(),
        BbEntry::Float("score".to_string(), 42.0),
    );

    let ctx = ScriptCtx {
        entity: test_entity(),
        cmd_buf: Arc::new(Mutex::new(ScriptCommands::default())),
        bb_buf: Arc::clone(&bb_buf),
        steer_buf: Arc::new(Mutex::new(None)),
        bb_snap: Arc::clone(&bb_snap),
    };
    eval_with_ctx(
        &sys,
        ctx,
        r#"
            bb_set_bool("active", true);
            bb_set_int("hp", 99);
            let s = bb_get_float("score");
        "#,
    );

    let changes = bb_buf.lock().unwrap().clone();
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

    // 엔티티 A 실행
    let bb_buf_a = Arc::new(Mutex::new(Vec::new()));
    let ctx_a = ScriptCtx {
        entity: test_entity(),
        cmd_buf: Arc::new(Mutex::new(ScriptCommands::default())),
        bb_buf: Arc::clone(&bb_buf_a),
        steer_buf: Arc::new(Mutex::new(None)),
        bb_snap: Arc::new(Mutex::new(HashMap::new())),
    };
    eval_with_ctx(&sys, ctx_a, r#"bb_set_bool("flag_a", true);"#);

    // 엔티티 B 실행
    let bb_buf_b = Arc::new(Mutex::new(Vec::new()));
    let ctx_b = ScriptCtx {
        entity: Entity::from_raw_parts(8, 4),
        cmd_buf: Arc::new(Mutex::new(ScriptCommands::default())),
        bb_buf: Arc::clone(&bb_buf_b),
        steer_buf: Arc::new(Mutex::new(None)),
        bb_snap: Arc::new(Mutex::new(HashMap::new())),
    };
    eval_with_ctx(&sys, ctx_b, r#"bb_set_bool("flag_b", true);"#);

    // A 버퍼에는 flag_a만, B 버퍼에는 flag_b만 있어야 한다
    let a = bb_buf_a.lock().unwrap();
    let b = bb_buf_b.lock().unwrap();
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
    let bb_buf = Arc::new(Mutex::new(Vec::new()));
    let ctx = ScriptCtx {
        entity: test_entity(),
        cmd_buf: Arc::new(Mutex::new(ScriptCommands::default())),
        bb_buf: Arc::clone(&bb_buf),
        steer_buf: Arc::new(Mutex::new(None)),
        bb_snap: Arc::new(Mutex::new(HashMap::new())),
    };
    eval_with_ctx(
        &sys,
        ctx,
        r#"
            bb_set_int("idx", entity_index());
            bb_set_int("gen", entity_generation());
        "#,
    );

    let changes = bb_buf.lock().unwrap().clone();
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
    let cmd_buf = Arc::new(Mutex::new(ScriptCommands::default()));
    let entity = test_entity();
    let ctx = ScriptCtx {
        entity,
        cmd_buf: Arc::clone(&cmd_buf),
        bb_buf: Arc::new(Mutex::new(Vec::new())),
        steer_buf: Arc::new(Mutex::new(None)),
        bb_snap: Arc::new(Mutex::new(HashMap::new())),
    };
    eval_with_ctx(
        &sys,
        ctx,
        "despawn_entity(entity_index(), entity_generation());",
    );

    assert_eq!(cmd_buf.lock().unwrap().despawn, vec![entity]);
}
