use std::cell::RefCell;
use std::collections::HashMap;

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
// ECS 시스템은 단일 스레드이므로 `Arc<Mutex<_>>` 없이 plain 버퍼를 RefCell 안에서
// 직접 빌려 쓴다. 버퍼는 `ScriptingSystem::run`이 엔티티마다 move-in/move-out 하며
// 재사용하므로(할당 재사용) 엔티티당 힙 할당이 발생하지 않는다.
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

/// thread_local 컨텍스트를 설정한다. 스크립트 실행 전 호출.
pub(super) fn set_script_ctx(ctx: ScriptCtx) {
    SCRIPT_CTX.with(|c| *c.borrow_mut() = Some(ctx));
}

/// thread_local 컨텍스트를 꺼내 돌려준다(버퍼 회수). 스크립트 실행 후 호출.
pub(super) fn take_script_ctx() -> Option<ScriptCtx> {
    SCRIPT_CTX.with(|c| c.borrow_mut().take())
}
