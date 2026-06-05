use rhai::Engine;

use crate::ecs::Entity;

use super::context::{BbEntry, SteeringCmd, SCRIPT_CTX};
use super::{ScriptingLimits, ScriptingSystem};

impl ScriptingSystem {
    pub fn with_limits(limits: ScriptingLimits) -> Self {
        let mut engine = Engine::new();
        engine.set_max_operations(limits.max_operations);

        // ── 모든 함수를 1회만 등록 (thread_local로 실행 컨텍스트 전달) ──────

        engine.register_fn("log", |msg: &str| println!("[Script] {msg}"));

        engine.register_fn("spawn_entity", || -> i64 {
            SCRIPT_CTX.with(|c| {
                let mut borrow = c.borrow_mut();
                let ctx = borrow
                    .as_mut()
                    .expect("SCRIPT_CTX must be set during script execution");
                ctx.cmd_buf.spawn_count += 1;
                let handle = -(ctx.cmd_buf.spawn_count as i64);
                ctx.cmd_buf.spawned_ids.push(handle);
                handle
            })
        });

        engine.register_fn("despawn_entity", |index: i64, generation: i64| {
            SCRIPT_CTX.with(|c| {
                let mut borrow = c.borrow_mut();
                let ctx = borrow
                    .as_mut()
                    .expect("SCRIPT_CTX must be set during script execution");
                if index >= 0 && generation >= 0 {
                    ctx.cmd_buf
                        .despawn
                        .push(Entity::from_raw_parts(index as u32, generation as u32));
                }
            });
        });

        engine.register_fn("entity_index", || -> i64 {
            SCRIPT_CTX.with(|c| {
                let borrow = c.borrow();
                let ctx = borrow
                    .as_ref()
                    .expect("SCRIPT_CTX must be set during script execution");
                ctx.entity.index() as i64
            })
        });

        engine.register_fn("entity_generation", || -> i64 {
            SCRIPT_CTX.with(|c| {
                let borrow = c.borrow();
                let ctx = borrow
                    .as_ref()
                    .expect("SCRIPT_CTX must be set during script execution");
                ctx.entity.generation() as i64
            })
        });

        engine.register_fn("bb_set_bool", |key: &str, val: bool| {
            SCRIPT_CTX.with(|c| {
                let mut borrow = c.borrow_mut();
                let ctx = borrow
                    .as_mut()
                    .expect("SCRIPT_CTX must be set during script execution");
                ctx.bb_buf.push(BbEntry::Bool(key.to_string(), val));
            });
        });

        engine.register_fn("bb_set_float", |key: &str, val: f64| {
            SCRIPT_CTX.with(|c| {
                let mut borrow = c.borrow_mut();
                let ctx = borrow
                    .as_mut()
                    .expect("SCRIPT_CTX must be set during script execution");
                ctx.bb_buf.push(BbEntry::Float(key.to_string(), val));
            });
        });

        engine.register_fn("bb_set_int", |key: &str, val: i64| {
            SCRIPT_CTX.with(|c| {
                let mut borrow = c.borrow_mut();
                let ctx = borrow
                    .as_mut()
                    .expect("SCRIPT_CTX must be set during script execution");
                ctx.bb_buf.push(BbEntry::Int(key.to_string(), val));
            });
        });

        engine.register_fn("bb_get_bool", |key: &str| -> bool {
            SCRIPT_CTX.with(|c| {
                let borrow = c.borrow();
                let ctx = borrow
                    .as_ref()
                    .expect("SCRIPT_CTX must be set during script execution");
                match ctx.bb_snap.get(key) {
                    Some(BbEntry::Bool(_, v)) => *v,
                    _ => false,
                }
            })
        });

        engine.register_fn("bb_get_float", |key: &str| -> f64 {
            SCRIPT_CTX.with(|c| {
                let borrow = c.borrow();
                let ctx = borrow
                    .as_ref()
                    .expect("SCRIPT_CTX must be set during script execution");
                match ctx.bb_snap.get(key) {
                    Some(BbEntry::Float(_, v)) => *v,
                    _ => 0.0,
                }
            })
        });

        engine.register_fn("bb_get_int", |key: &str| -> i64 {
            SCRIPT_CTX.with(|c| {
                let borrow = c.borrow();
                let ctx = borrow
                    .as_ref()
                    .expect("SCRIPT_CTX must be set during script execution");
                match ctx.bb_snap.get(key) {
                    Some(BbEntry::Int(_, v)) => *v,
                    _ => 0,
                }
            })
        });

        engine.register_fn("seek_target", |tx: f64, ty: f64, speed: f64| {
            SCRIPT_CTX.with(|c| {
                let mut borrow = c.borrow_mut();
                let ctx = borrow
                    .as_mut()
                    .expect("SCRIPT_CTX must be set during script execution");
                ctx.steer_buf = Some(SteeringCmd::Seek {
                    tx: tx as f32,
                    ty: ty as f32,
                    speed: speed as f32,
                });
            });
        });

        engine.register_fn("flee_from", |tx: f64, ty: f64, speed: f64, radius: f64| {
            SCRIPT_CTX.with(|c| {
                let mut borrow = c.borrow_mut();
                let ctx = borrow
                    .as_mut()
                    .expect("SCRIPT_CTX must be set during script execution");
                ctx.steer_buf = Some(SteeringCmd::Flee {
                    tx: tx as f32,
                    ty: ty as f32,
                    speed: speed as f32,
                    radius: radius as f32,
                });
            });
        });

        engine.register_fn("stop_steering", || {
            SCRIPT_CTX.with(|c| {
                let mut borrow = c.borrow_mut();
                let ctx = borrow
                    .as_mut()
                    .expect("SCRIPT_CTX must be set during script execution");
                ctx.steer_buf = Some(SteeringCmd::Stop);
            });
        });

        Self { engine }
    }
}
