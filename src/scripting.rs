use rhai::{Engine, Scope};

use crate::asset::{Handle, ScriptAsset};

mod api;
mod context;
mod execution;

// ─── ScriptRunner ─────────────────────────────────────────────────────────────

/// 엔티티에 붙이는 스크립트 실행기 컴포넌트.
///
/// ```rust,no_run
/// # use engine::{ScriptRunner, ScriptingSystem};
/// # let mut app = engine::App::new();
/// let handle = app.load_script("assets/enemy_ai.rhai");
/// // world.add_component(entity, ScriptRunner::new(handle));
/// // app.add_system(Box::new(ScriptingSystem::new()));
/// ```
pub struct ScriptRunner {
    pub script: Handle<ScriptAsset>,
    pub(crate) scope: Scope<'static>,
    pub(crate) started: bool,
}

impl ScriptRunner {
    pub fn new(script: Handle<ScriptAsset>) -> Self {
        let mut scope = Scope::new();
        scope.push("x", 0.0_f64);
        scope.push("y", 0.0_f64);
        scope.push("rot", 0.0_f64);
        scope.push("sx", 1.0_f64);
        scope.push("sy", 1.0_f64);
        Self {
            script,
            scope,
            started: false,
        }
    }

    /// 다음 프레임에 on_start()가 다시 호출되도록 리셋한다 (핫 리로드 후 유용).
    pub fn reset(&mut self) {
        self.started = false;
    }
}

// ─── ScriptingSystem ──────────────────────────────────────────────────────────

/// ScriptRunner를 가진 모든 엔티티에 대해 매 프레임 스크립트를 실행하는 시스템.
///
/// 이 시스템은 신뢰된 로컬 게임 스크립트를 실행하기 위한 기능이다. Rhai operation limit은
/// 실수로 만든 무한 루프를 줄여 주지만, 적대적/원격 사용자 입력을 안전하게 격리하는
/// sandbox 경계로 보장하지 않는다.
///
/// 스코프 변수: `x`, `y`, `rot`, `sx`, `sy`  (Transform 읽기/쓰기)
///
/// 라이프사이클:
/// - `fn on_start()` — 처음 한 번만 호출 (없어도 무방)
/// - `fn on_update(dt)` — 매 프레임 호출
///
/// ## 추가 스크립트 API (Phase 38d)
///
/// ### Commands
/// ```rhai
/// let id = spawn_entity();   // 새 엔티티 생성 → ID(i64) 반환
/// let index = entity_index();
/// let generation = entity_generation();
/// despawn_entity(index, generation); // 엔티티 삭제 예약
/// ```
///
/// `spawn_entity()`가 반환하는 음수 ID는 같은 스크립트 안에서 실제 엔티티를 조작할 수 있는
/// 안정 핸들이 아니다. 실제 스폰은 스크립트 실행 후 적용된다. `despawn_entity(index,
/// generation)`은 generation-checked ECS handle을 구성하며 stale handle이면 조용히 무시된다.
///
/// ### Blackboard
/// ```rhai
/// bb_set_bool("is_chasing", true);
/// bb_set_float("speed", 150.0);
/// bb_set_int("hp", 100);
/// let chasing = bb_get_bool("is_chasing");  // 없으면 false
/// let speed   = bb_get_float("speed");       // 없으면 0.0
/// let hp      = bb_get_int("hp");            // 없으면 0
/// ```
///
/// ### Steering
/// ```rhai
/// seek_target(player_x, player_y, 120.0);        // Seek 컴포넌트 설정
/// flee_from(enemy_x, enemy_y, 200.0, 80.0);      // Flee 컴포넌트 설정
/// stop_steering();                                // SteeringVelocity 속도 리셋
/// ```
pub struct ScriptingSystem {
    engine: Engine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptingLimits {
    pub max_operations: u64,
}

impl Default for ScriptingLimits {
    fn default() -> Self {
        Self {
            max_operations: 1_000_000,
        }
    }
}

impl ScriptingSystem {
    /// Creates a scripting system for trusted local script assets.
    ///
    /// Rhai operation limits reduce accidental runaway scripts, but engine script
    /// assets are still treated as trusted local game code rather than hostile
    /// sandboxed input.
    pub fn new() -> Self {
        Self::with_limits(ScriptingLimits::default())
    }
}

impl Default for ScriptingSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
