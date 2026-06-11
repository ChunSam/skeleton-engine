use std::cell::RefCell;
use std::collections::HashMap;

use crate::ecs::Entity;

/// ECS commands collected during script execution.
#[derive(Default)]
pub(super) struct ScriptCommands {
    pub(super) despawn: Vec<Entity>,
    pub(super) spawn_count: u32,
    /// Write-only: `spawn_entity()` pushes the negative handle here so the buffer allocation
    /// is reused across frames, but the values are never read back — `spawn_entity()` already
    /// returns the handle directly to the calling script. Kept only for buffer-reuse bookkeeping.
    /// TODO: remove this field once the scripting API exposes a way to map script-side handles
    /// to real entities (at which point the Vec becomes load-bearing again).
    pub(super) spawned_ids: Vec<i64>,
}

/// Blackboard entry used in two contexts:
/// - `bb_buf`: entries written by the script this frame (the `String` key is needed here to
///   know which blackboard field to set when applying the buffer).
/// - `bb_snap` (`HashMap<String, BbEntry>`): snapshot read by the script. In this context the
///   `String` key inside the variant is redundant (the HashMap key already carries it), but
///   `BbEntry` is shared between both contexts so the field cannot be removed without splitting
///   the type.
#[derive(Clone)]
pub(super) enum BbEntry {
    Bool(String, bool),
    Float(String, f64),
    Int(String, i64),
}

#[derive(Clone, Debug)]
pub(super) enum SteeringCmd {
    Seek {
        tx: f32,
        ty: f32,
        speed: f32,
    },
    Flee {
        tx: f32,
        ty: f32,
        speed: f32,
        radius: f32,
    },
    /// Decelerate toward a point; stop within `stop_radius`.
    Arrive {
        tx: f32,
        ty: f32,
        speed: f32,
        slow_radius: f32,
        stop_radius: f32,
    },
    /// Roam randomly; direction changes every `change_interval` seconds.
    Wander {
        speed: f32,
        change_interval: f32,
    },
    Stop,
}

// Instead of calling register_fn for every entity, functions are registered once inside
// with_limits() and the execution context (buffer) is passed via thread_local.
// Since the ECS system is single-threaded, plain buffers are borrowed directly inside a
// RefCell without needing `Arc<Mutex<_>>`. The buffer is moved in/out by
// `ScriptingSystem::run` for each entity (allocation reuse), so no per-entity heap
// allocation occurs.
pub(super) struct ScriptCtx {
    pub(super) entity: Entity,
    pub(super) cmd_buf: ScriptCommands,
    pub(super) bb_buf: Vec<BbEntry>,
    pub(super) steer_buf: Option<SteeringCmd>,
    pub(super) bb_snap: HashMap<String, BbEntry>,
}

thread_local! {
    pub(super) static SCRIPT_CTX: RefCell<Option<ScriptCtx>> = const { RefCell::new(None) };
}

/// Sets the thread_local context. Call before executing a script.
pub(super) fn set_script_ctx(ctx: ScriptCtx) {
    SCRIPT_CTX.with(|c| *c.borrow_mut() = Some(ctx));
}

/// Takes and returns the thread_local context (buffer reclaim). Call after executing a script.
pub(super) fn take_script_ctx() -> Option<ScriptCtx> {
    SCRIPT_CTX.with(|c| c.borrow_mut().take())
}
