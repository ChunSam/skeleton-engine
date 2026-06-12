use rhai::Engine;

use crate::ecs::Entity;

use crate::behavior::BlackboardValue;

use super::context::{with_ctx, with_ctx_mut, BbEntry, SteeringCmd};
use super::{ScriptingLimits, ScriptingSystem};

impl ScriptingSystem {
    pub fn with_limits(limits: ScriptingLimits) -> Self {
        let mut engine = Engine::new();
        engine.set_max_operations(limits.max_operations);
        engine.set_max_string_size(limits.max_string_size);
        engine.set_max_array_size(limits.max_array_size);
        engine.set_max_map_size(limits.max_map_size);
        engine.set_max_call_levels(limits.max_call_levels);
        // Same cap for global and in-function expression depth.
        engine.set_max_expr_depths(limits.max_expr_depth, limits.max_expr_depth);

        // ── Register all functions once (execution context passed via thread_local) ──────

        engine.register_fn("log", |msg: &str| println!("[Script] {msg}"));

        engine.register_fn("spawn_entity", || -> i64 {
            with_ctx_mut(|ctx| {
                ctx.cmd_buf.spawn_count += 1;
                let handle = -(ctx.cmd_buf.spawn_count as i64);
                ctx.cmd_buf.spawned_ids.push(handle);
                handle
            })
        });

        engine.register_fn("despawn_entity", |index: i64, generation: i64| {
            with_ctx_mut(|ctx| {
                if index >= 0 && generation >= 0 {
                    ctx.cmd_buf
                        .despawn
                        .push(Entity::from_raw_parts(index as u32, generation as u32));
                }
            });
        });

        engine.register_fn("entity_index", || -> i64 {
            with_ctx(|ctx| ctx.entity.index() as i64)
        });

        engine.register_fn("entity_generation", || -> i64 {
            with_ctx(|ctx| ctx.entity.generation() as i64)
        });

        engine.register_fn("bb_set_bool", |key: &str, val: bool| {
            with_ctx_mut(|ctx| ctx.bb_buf.push(BbEntry::Bool(key.to_string(), val)));
        });

        engine.register_fn("bb_set_float", |key: &str, val: f64| {
            with_ctx_mut(|ctx| ctx.bb_buf.push(BbEntry::Float(key.to_string(), val)));
        });

        engine.register_fn("bb_set_int", |key: &str, val: i64| {
            with_ctx_mut(|ctx| ctx.bb_buf.push(BbEntry::Int(key.to_string(), val)));
        });

        engine.register_fn("bb_get_bool", |key: &str| -> bool {
            with_ctx(|ctx| match ctx.bb_snap.get(key) {
                Some(BlackboardValue::Bool(v)) => *v,
                _ => false,
            })
        });

        engine.register_fn("bb_get_float", |key: &str| -> f64 {
            with_ctx(|ctx| match ctx.bb_snap.get(key) {
                Some(BlackboardValue::Float(v)) => *v as f64,
                _ => 0.0,
            })
        });

        engine.register_fn("bb_get_int", |key: &str| -> i64 {
            with_ctx(|ctx| match ctx.bb_snap.get(key) {
                Some(BlackboardValue::Int(v)) => *v as i64,
                _ => 0,
            })
        });

        engine.register_fn("seek_target", |tx: f64, ty: f64, speed: f64| {
            with_ctx_mut(|ctx| {
                ctx.steer_buf = Some(SteeringCmd::Seek {
                    tx: tx as f32,
                    ty: ty as f32,
                    speed: speed as f32,
                });
            });
        });

        engine.register_fn("flee_from", |tx: f64, ty: f64, speed: f64, radius: f64| {
            with_ctx_mut(|ctx| {
                ctx.steer_buf = Some(SteeringCmd::Flee {
                    tx: tx as f32,
                    ty: ty as f32,
                    speed: speed as f32,
                    radius: radius as f32,
                });
            });
        });

        // arrive_at(tx, ty, speed, slow_radius, stop_radius)
        // Decelerate smoothly as the entity nears (tx, ty); stop within stop_radius.
        engine.register_fn(
            "arrive_at",
            |tx: f64, ty: f64, speed: f64, slow_radius: f64, stop_radius: f64| {
                with_ctx_mut(|ctx| {
                    ctx.steer_buf = Some(SteeringCmd::Arrive {
                        tx: tx as f32,
                        ty: ty as f32,
                        speed: speed as f32,
                        slow_radius: slow_radius as f32,
                        stop_radius: stop_radius as f32,
                    });
                });
            },
        );

        // wander(speed, change_interval)
        // Roam in a random direction; pick a new direction every change_interval seconds.
        engine.register_fn("wander", |speed: f64, change_interval: f64| {
            with_ctx_mut(|ctx| {
                ctx.steer_buf = Some(SteeringCmd::Wander {
                    speed: speed as f32,
                    change_interval: change_interval as f32,
                });
            });
        });

        engine.register_fn("stop_steering", || {
            with_ctx_mut(|ctx| {
                ctx.steer_buf = Some(SteeringCmd::Stop);
            });
        });

        Self { engine }
    }
}
