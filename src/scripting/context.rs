use std::cell::RefCell;
use std::collections::HashMap;

use crate::behavior::BlackboardValue;
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

/// Blackboard entry used by the write path (`bb_buf`).
///
/// The `String` key identifies which blackboard field to set when the buffer is
/// applied after script execution.  The snapshot read path (`bb_snap`) uses a
/// plain `BlackboardValue` keyed by the map key, so the key is not duplicated.
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
    /// Read-only snapshot of the entity's Blackboard at the start of the frame.
    /// Uses `BlackboardValue` directly — the key is already the map key, so no
    /// duplicate allocation is needed here (unlike `bb_buf` which carries the key
    /// inside `BbEntry` because the apply loop needs it).
    pub(super) bb_snap: HashMap<String, BlackboardValue>,
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

/// Borrows the context mutably and runs `f`.
///
/// All registered Rhai functions that write to the context use this helper to
/// avoid repeating the `with(|c| borrow_mut().as_mut().expect(…))` pattern.
/// Safe because Rhai evaluation is single-threaded and non-reentrant — no
/// nested `with()` call can occur while `f` is executing.
#[inline]
pub(super) fn with_ctx_mut<R>(f: impl FnOnce(&mut ScriptCtx) -> R) -> R {
    SCRIPT_CTX.with(|c| {
        let mut borrow = c.borrow_mut();
        let ctx = borrow
            .as_mut()
            .expect("SCRIPT_CTX must be set during script execution");
        f(ctx)
    })
}

/// Borrows the context immutably and runs `f`.
///
/// All registered Rhai functions that only read from the context use this
/// helper.  Same single-threaded non-reentrant guarantee as [`with_ctx_mut`].
#[inline]
pub(super) fn with_ctx<R>(f: impl FnOnce(&ScriptCtx) -> R) -> R {
    SCRIPT_CTX.with(|c| {
        let borrow = c.borrow();
        let ctx = borrow
            .as_ref()
            .expect("SCRIPT_CTX must be set during script execution");
        f(ctx)
    })
}
