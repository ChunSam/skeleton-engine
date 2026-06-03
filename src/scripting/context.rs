use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::ecs::Entity;

/// 스크립트 실행 중 수집된 ECS 명령.
#[derive(Default)]
pub(super) struct ScriptCommands {
    pub(super) despawn: Vec<Entity>,
    pub(super) spawn_count: u32,
    pub(super) spawned_ids: Vec<i64>,
}

#[derive(Clone)]
pub(super) enum BbEntry {
    Bool(String, bool),
    Float(String, f64),
    Int(String, i64),
}

#[derive(Clone)]
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
    Stop,
}

// 엔티티마다 register_fn을 반복 호출하는 대신, with_limits()에서 1회만 등록하고
// 실행 컨텍스트(버퍼)를 thread_local로 전달한다.
// ECS 시스템은 단일 스레드이므로 RefCell이 안전하다.
pub(super) struct ScriptCtx {
    pub(super) entity: Entity,
    pub(super) cmd_buf: Arc<Mutex<ScriptCommands>>,
    pub(super) bb_buf: Arc<Mutex<Vec<BbEntry>>>,
    pub(super) steer_buf: Arc<Mutex<Option<SteeringCmd>>>,
    pub(super) bb_snap: Arc<Mutex<HashMap<String, BbEntry>>>,
}

thread_local! {
    pub(super) static SCRIPT_CTX: RefCell<Option<ScriptCtx>> = const { RefCell::new(None) };
}

/// thread_local 컨텍스트를 설정한다. 스크립트 실행 전 호출.
pub(super) fn set_script_ctx(ctx: ScriptCtx) {
    SCRIPT_CTX.with(|c| *c.borrow_mut() = Some(ctx));
}

/// thread_local 컨텍스트를 제거한다. 스크립트 실행 후 호출.
pub(super) fn clear_script_ctx() {
    SCRIPT_CTX.with(|c| *c.borrow_mut() = None);
}
