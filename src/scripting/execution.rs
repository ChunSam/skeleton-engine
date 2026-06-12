use std::collections::HashMap;

use rhai::{Engine, EvalAltResult, Scope};

use crate::asset::AssetServer;
use crate::behavior::Blackboard;
use crate::components::Transform;
use crate::ecs::{Entity, System, World};
use crate::steering::{Arrive, Flee, Seek, SteeringVelocity, Wander};

use crate::behavior::BlackboardValue;

use super::context::{
    set_script_ctx, take_script_ctx, BbEntry, ScriptCommands, ScriptCtx, SteeringCmd,
};
use super::{ScriptRunner, ScriptingSystem};

impl ScriptingSystem {
    /// Schedule label for ordering via `add_system_labeled`.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::scripting";
}

impl System for ScriptingSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let entities: Vec<Entity> = world.query::<ScriptRunner>().map(|(e, _)| e).collect();

        // Reusable scratch buffers — created once and moved through the
        // thread_local `ScriptCtx` for each entity, then recovered. This reuses
        // their allocations across every scripted entity instead of allocating
        // four heap buffers (previously `Arc<Mutex<_>>`) per entity per frame.
        let mut cmd_buf = ScriptCommands::default();
        let mut bb_buf: Vec<BbEntry> = Vec::new();
        let mut steer_buf: Option<SteeringCmd> = None;
        let mut bb_snap: HashMap<String, BlackboardValue> = HashMap::new();

        for entity in entities {
            // Read script handle id + started flag
            let (script_id, is_started) = match world.get::<ScriptRunner>(entity) {
                Some(r) => (r.script.id(), r.started),
                None => continue,
            };

            // Read Transform snapshot
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

            // Read AST
            let ast = match world
                .resource::<AssetServer>()
                .and_then(|s| s.get_script_by_id(script_id))
                .map(|a| a.ast.clone())
            {
                Some(a) => a,
                None => continue,
            };

            // ── Reset reusable buffers (preserve allocations, clear contents) ─────────────────
            // `steer_buf` is set to None each iteration by the `take()` in set_script_ctx
            // and the `take()` in the steering application below, so no explicit reset is needed.
            cmd_buf.despawn.clear();
            cmd_buf.spawn_count = 0;
            cmd_buf.spawned_ids.clear();
            bb_buf.clear();
            bb_snap.clear();

            // Collect Blackboard snapshot — one to_string() per entry (the map key only).
            if let Some(bb) = world.get::<Blackboard>(entity) {
                for (key, val) in bb.entries() {
                    match val {
                        BlackboardValue::Bool(_) | BlackboardValue::Float(_) | BlackboardValue::Int(_) => {
                            bb_snap.insert(key.to_string(), val.clone());
                        }
                        _ => {}
                    }
                }
            }

            // ── Move buffers into the thread_local context, execute, then recover ──────────
            set_script_ctx(ScriptCtx {
                entity,
                cmd_buf: std::mem::take(&mut cmd_buf),
                bb_buf: std::mem::take(&mut bb_buf),
                steer_buf: steer_buf.take(),
                bb_snap: std::mem::take(&mut bb_snap),
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

            // Recover buffers — get back execution results and reusable allocations.
            let ctx = take_script_ctx().expect("script ctx was set above");
            cmd_buf = ctx.cmd_buf;
            bb_buf = ctx.bb_buf;
            steer_buf = ctx.steer_buf;
            bb_snap = ctx.bb_snap;

            // ── Apply Transform results ──────────────────────────────────────────
            if let Some(t) = world.get_mut::<Transform>(entity) {
                t.position.x = new_tx as f32;
                t.position.y = new_ty as f32;
                t.rotation = new_tr as f32;
                t.scale.x = new_tsx as f32;
                t.scale.y = new_tsy as f32;
            }

            // ── Apply Commands ────────────────────────────────────────────────
            for _ in 0..cmd_buf.spawn_count {
                world.spawn();
            }
            for &e in &cmd_buf.despawn {
                world.despawn(e);
            }

            // ── Apply Blackboard changes ──────────────────────────────────────────
            if !bb_buf.is_empty() {
                if world.get::<Blackboard>(entity).is_none() {
                    world.add_component(entity, Blackboard::new());
                }
                if let Some(bb) = world.get_mut::<Blackboard>(entity) {
                    for entry in &bb_buf {
                        match entry {
                            BbEntry::Bool(k, v) => bb.set_bool(k, *v),
                            BbEntry::Float(k, v) => bb.set_float(k, *v as f32),
                            BbEntry::Int(k, v) => bb.set_int(k, *v as i32),
                        }
                    }
                }
            }

            // ── Apply Steering changes ────────────────────────────────────────────
            // Script steering commands are mutually exclusive: applying any one behavior
            // removes the other three from this entity so stale components cannot
            // override the newly requested behavior via SteeringSystem's fixed evaluation
            // order (Seek → Flee → Arrive → Wander, last-writer-wins).
            // SteeringCmd::Stop removes all four AND zeroes SteeringVelocity so the
            // entity stays stopped on subsequent frames.
            // NOTE: this exclusivity applies to the SCRIPT apply path only. Rust-side
            // code may still compose multiple steering components manually (documented
            // feature of SteeringSystem).
            if let Some(cmd) = steer_buf.take() {
                match cmd {
                    SteeringCmd::Seek { tx, ty, speed } => {
                        use glam::Vec2;
                        // Remove competing components before applying Seek.
                        world.remove_component::<Flee>(entity);
                        world.remove_component::<Arrive>(entity);
                        world.remove_component::<Wander>(entity);
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
                        // Remove competing components before applying Flee.
                        world.remove_component::<Seek>(entity);
                        world.remove_component::<Arrive>(entity);
                        world.remove_component::<Wander>(entity);
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
                    SteeringCmd::Arrive {
                        tx,
                        ty,
                        speed,
                        slow_radius,
                        stop_radius,
                    } => {
                        use glam::Vec2;
                        // Remove competing components before applying Arrive.
                        world.remove_component::<Seek>(entity);
                        world.remove_component::<Flee>(entity);
                        world.remove_component::<Wander>(entity);
                        if world.get::<SteeringVelocity>(entity).is_none() {
                            world.add_component(entity, SteeringVelocity::default());
                        }
                        world.add_component(
                            entity,
                            Arrive {
                                target: Vec2::new(tx, ty),
                                max_speed: speed,
                                slow_radius,
                                stop_radius,
                            },
                        );
                    }
                    SteeringCmd::Wander {
                        speed,
                        change_interval,
                    } => {
                        // Remove competing components before applying Wander.
                        world.remove_component::<Seek>(entity);
                        world.remove_component::<Flee>(entity);
                        world.remove_component::<Arrive>(entity);
                        if world.get::<SteeringVelocity>(entity).is_none() {
                            world.add_component(entity, SteeringVelocity::default());
                        }
                        // Preserve the existing Wander component's timer/direction if present;
                        // only update speed/interval when the script changes the parameters.
                        let existing = world.get::<Wander>(entity).cloned();
                        match existing {
                            Some(mut w) => {
                                w.max_speed = speed;
                                w.change_interval = change_interval;
                                world.add_component(entity, w);
                            }
                            None => {
                                world.add_component(entity, Wander::new(speed, change_interval));
                            }
                        }
                    }
                    SteeringCmd::Stop => {
                        // Remove all four steering components so they cannot re-apply
                        // on subsequent frames. Zeroing SteeringVelocity ensures the
                        // entity stays stopped even if SteeringSystem runs this frame.
                        world.remove_component::<Seek>(entity);
                        world.remove_component::<Flee>(entity);
                        world.remove_component::<Arrive>(entity);
                        world.remove_component::<Wander>(entity);
                        if let Some(sv) = world.get_mut::<SteeringVelocity>(entity) {
                            sv.velocity = glam::Vec2::ZERO;
                        }
                    }
                }
            }
        }
    }
}

// ─── Internal helpers ────────────────────────────────────────────────────────────────

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
            ref other => log::warn!("Script '{fn_name}' error: {other}"),
        }
    }
}
