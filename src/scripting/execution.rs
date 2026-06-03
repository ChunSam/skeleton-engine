use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rhai::{Engine, EvalAltResult, Scope};

use crate::asset::AssetServer;
use crate::behavior::Blackboard;
use crate::components::Transform;
use crate::ecs::{Entity, System, World};
use crate::steering::{Flee, Seek, SteeringVelocity};

use super::context::{
    clear_script_ctx, set_script_ctx, BbEntry, ScriptCommands, ScriptCtx, SteeringCmd,
};
use super::{ScriptRunner, ScriptingSystem};

impl System for ScriptingSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let entities: Vec<Entity> = world.query::<ScriptRunner>().map(|(e, _)| e).collect();

        for entity in entities {
            // 스크립트 핸들 id + started 플래그 읽기
            let (script_id, is_started) = match world.get::<ScriptRunner>(entity) {
                Some(r) => (r.script.id(), r.started),
                None => continue,
            };

            // Transform 스냅샷 읽기
            let (tx, ty, tr, tsx, tsy) = world
                .get::<Transform>(entity)
                .map(|t| {
                    (
                        t.position.x as f64,
                        t.position.y as f64,
                        t.rotation as f64,
                        t.scale.x as f64,
                        t.scale.y as f64,
                    )
                })
                .unwrap_or((0.0, 0.0, 0.0, 1.0, 1.0));

            // AST 읽기
            let ast = match world
                .resource::<AssetServer>()
                .and_then(|s| s.get_script_by_id(script_id))
                .map(|a| a.ast.clone())
            {
                Some(a) => a,
                None => continue,
            };

            // ── 엔티티별 버퍼 생성 ─────────────────────────────────────────────
            let cmd_buf: Arc<Mutex<ScriptCommands>> =
                Arc::new(Mutex::new(ScriptCommands::default()));
            let bb_buf: Arc<Mutex<Vec<BbEntry>>> = Arc::new(Mutex::new(Vec::new()));
            let steer_buf: Arc<Mutex<Option<SteeringCmd>>> = Arc::new(Mutex::new(None));
            let bb_snap: Arc<Mutex<HashMap<String, BbEntry>>> =
                Arc::new(Mutex::new(HashMap::new()));

            // Blackboard 스냅샷 수집
            {
                use crate::behavior::BlackboardValue;
                if let Some(bb) = world.get::<Blackboard>(entity) {
                    let mut snap = bb_snap.lock().unwrap();
                    for (key, val) in bb.entries() {
                        let entry = match val {
                            BlackboardValue::Bool(v) => BbEntry::Bool(key.to_string(), *v),
                            BlackboardValue::Float(v) => BbEntry::Float(key.to_string(), *v as f64),
                            BlackboardValue::Int(v) => BbEntry::Int(key.to_string(), *v as i64),
                            _ => continue,
                        };
                        snap.insert(key.to_string(), entry);
                    }
                }
            }

            // ── thread_local 컨텍스트 설정 → 스크립트 실행 → 컨텍스트 제거 ───
            set_script_ctx(ScriptCtx {
                entity,
                cmd_buf: Arc::clone(&cmd_buf),
                bb_buf: Arc::clone(&bb_buf),
                steer_buf: Arc::clone(&steer_buf),
                bb_snap: Arc::clone(&bb_snap),
            });

            let (new_tx, new_ty, new_tr, new_tsx, new_tsy) = {
                let runner = world.get_mut::<ScriptRunner>(entity).unwrap();
                runner.scope.set_value("x", tx);
                runner.scope.set_value("y", ty);
                runner.scope.set_value("rot", tr);
                runner.scope.set_value("sx", tsx);
                runner.scope.set_value("sy", tsy);

                if !is_started {
                    call_fn_optional(&self.engine, &mut runner.scope, &ast, "on_start", ());
                    runner.started = true;
                }
                call_fn_optional(
                    &self.engine,
                    &mut runner.scope,
                    &ast,
                    "on_update",
                    (dt as f64,),
                );

                let nx = runner.scope.get_value::<f64>("x").unwrap_or(tx);
                let ny = runner.scope.get_value::<f64>("y").unwrap_or(ty);
                let nr = runner.scope.get_value::<f64>("rot").unwrap_or(tr);
                let nsx = runner.scope.get_value::<f64>("sx").unwrap_or(tsx);
                let nsy = runner.scope.get_value::<f64>("sy").unwrap_or(tsy);
                (nx, ny, nr, nsx, nsy)
            };

            clear_script_ctx();

            // ── Transform 결과 적용 ──────────────────────────────────────────
            if let Some(t) = world.get_mut::<Transform>(entity) {
                t.position.x = new_tx as f32;
                t.position.y = new_ty as f32;
                t.rotation = new_tr as f32;
                t.scale.x = new_tsx as f32;
                t.scale.y = new_tsy as f32;
            }

            // ── Commands 적용 ────────────────────────────────────────────────
            let (spawn_count, despawn_list) = {
                let guard = cmd_buf.lock().unwrap();
                (guard.spawn_count, guard.despawn.clone())
            };
            for _ in 0..spawn_count {
                world.spawn();
            }
            for e in despawn_list {
                world.despawn(e);
            }

            // ── Blackboard 변경 적용 ──────────────────────────────────────────
            let bb_changes = { bb_buf.lock().unwrap().clone() };
            if !bb_changes.is_empty() {
                if world.get::<Blackboard>(entity).is_none() {
                    world.add_component(entity, Blackboard::new());
                }
                if let Some(bb) = world.get_mut::<Blackboard>(entity) {
                    for entry in bb_changes {
                        match entry {
                            BbEntry::Bool(k, v) => bb.set_bool(&k, v),
                            BbEntry::Float(k, v) => bb.set_float(&k, v as f32),
                            BbEntry::Int(k, v) => bb.set_int(&k, v as i32),
                        }
                    }
                }
            }

            // ── Steering 변경 적용 ────────────────────────────────────────────
            let steer_cmd = steer_buf.lock().unwrap().take();
            if let Some(cmd) = steer_cmd {
                match cmd {
                    SteeringCmd::Seek { tx, ty, speed } => {
                        use glam::Vec2;
                        if world.get::<SteeringVelocity>(entity).is_none() {
                            world.add_component(entity, SteeringVelocity::default());
                        }
                        world.add_component(
                            entity,
                            Seek {
                                target: Vec2::new(tx, ty),
                                max_speed: speed,
                            },
                        );
                    }
                    SteeringCmd::Flee {
                        tx,
                        ty,
                        speed,
                        radius,
                    } => {
                        use glam::Vec2;
                        if world.get::<SteeringVelocity>(entity).is_none() {
                            world.add_component(entity, SteeringVelocity::default());
                        }
                        world.add_component(
                            entity,
                            Flee {
                                target: Vec2::new(tx, ty),
                                max_speed: speed,
                                flee_radius: radius,
                            },
                        );
                    }
                    SteeringCmd::Stop => {
                        if let Some(sv) = world.get_mut::<SteeringVelocity>(entity) {
                            sv.velocity = glam::Vec2::ZERO;
                        }
                    }
                }
            }
        }
    }
}

// ─── 내부 헬퍼 ────────────────────────────────────────────────────────────────

fn call_fn_optional<A: rhai::FuncArgs>(
    engine: &Engine,
    scope: &mut Scope,
    ast: &rhai::AST,
    fn_name: &str,
    args: A,
) {
    if let Err(e) = engine.call_fn::<()>(scope, ast, fn_name, args) {
        match *e {
            EvalAltResult::ErrorFunctionNotFound(_, _) => {}
            ref other => log::warn!("Script '{fn_name}' 오류: {other}"),
        }
    }
}
