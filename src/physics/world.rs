use glam::Vec2;
use rapier2d::prelude::*;

mod body_factory;
mod character_movement;
mod joints;
mod raycast;
mod tile_collider;

// ── 충돌 그룹 ────────────────────────────────────────────────────────────────

/// Rapier `InteractionGroups`를 감싼 엔진용 충돌 레이어/마스크.
///
/// `memberships`는 이 콜라이더가 속한 레이어 비트, `filter`는 상호작용을 허용할
/// 상대 레이어 비트다. 두 콜라이더가 모두 서로를 허용해야 충돌/센서 교차가 발생한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionGroups {
    pub memberships: u32,
    pub filter: u32,
}

impl CollisionGroups {
    pub const ALL_BITS: u32 = u32::MAX;
    pub const NONE_BITS: u32 = 0;

    pub const fn new(memberships: u32, filter: u32) -> Self {
        Self {
            memberships,
            filter,
        }
    }

    pub const fn all() -> Self {
        Self::new(Self::ALL_BITS, Self::ALL_BITS)
    }

    pub const fn none() -> Self {
        Self::new(Self::NONE_BITS, Self::NONE_BITS)
    }

    pub const fn layer(bit_index: u8) -> Self {
        assert!(
            bit_index < u32::BITS as u8,
            "collision layer bit index must be < 32"
        );
        let bit = 1u32 << bit_index;
        Self::new(bit, Self::ALL_BITS)
    }

    pub const fn try_layer(bit_index: u8) -> Option<Self> {
        if bit_index < u32::BITS as u8 {
            Some(Self::new(1u32 << bit_index, Self::ALL_BITS))
        } else {
            None
        }
    }

    pub const fn with_filter(mut self, filter: u32) -> Self {
        self.filter = filter;
        self
    }

    fn to_rapier(self) -> InteractionGroups {
        InteractionGroups::new(Group::from(self.memberships), Group::from(self.filter))
    }

    fn from_rapier(groups: InteractionGroups) -> Self {
        Self::new(groups.memberships.bits(), groups.filter.bits())
    }
}

impl Default for CollisionGroups {
    fn default() -> Self {
        Self::all()
    }
}

// ── 레이캐스트 결과 ──────────────────────────────────────────────────────────

/// 레이캐스트 충돌 결과.
#[derive(Debug, Clone, Copy)]
pub struct RaycastHit {
    /// 충돌한 콜라이더 핸들.
    pub collider_handle: ColliderHandle,
    /// 월드 공간 충돌 지점 (물리 단위).
    pub point: Vec2,
    /// 충돌 면의 법선 벡터 (정규화됨).
    pub normal: Vec2,
    /// 레이 시작점으로부터의 거리 배율 (`origin + direction * toi` = 충돌 지점).
    pub toi: f32,
}

/// [`PhysicsWorld::add_static_from_tilemap`]이 타일마다 만들 콜라이더의 종류.
///
/// 충돌 그룹과 one-way 여부를 함께 지정한다. 편의 생성자로 흔한 경우를 만든다:
/// [`TileCollider::solid`](전체 충돌), [`TileCollider::one_way`](위에서만 막음).
#[derive(Debug, Clone, Copy)]
pub struct TileCollider {
    /// 콜라이더 충돌 그룹.
    pub groups: CollisionGroups,
    /// `true`면 위에서 내려올 때만 막는 one-way 플랫폼.
    pub one_way: bool,
}

impl TileCollider {
    /// 모든 방향을 막는 일반 솔리드 타일 (`CollisionGroups::all`).
    pub fn solid() -> Self {
        Self {
            groups: CollisionGroups::all(),
            one_way: false,
        }
    }

    /// 충돌 그룹을 지정한 솔리드 타일.
    pub fn solid_with(groups: CollisionGroups) -> Self {
        Self {
            groups,
            one_way: false,
        }
    }

    /// 위에서 내려올 때만 막고 아래/위 통과를 허용하는 one-way 플랫폼 타일.
    pub fn one_way() -> Self {
        Self {
            groups: CollisionGroups::all(),
            one_way: true,
        }
    }
}

/// rapier2d 2D 물리 시뮬레이션 세계.
///
/// `PhysicsSystem`이 소유하거나 직접 시스템 구조체에 넣어 사용한다.
pub struct PhysicsWorld {
    pub(crate) rigid_body_set: RigidBodySet,
    pub(crate) collider_set: ColliderSet,
    pub(crate) narrow_phase: NarrowPhase,
    gravity: Vector<f32>,
    integration_params: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    pub(crate) island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    pub(crate) impulse_joint_set: ImpulseJointSet,
    pub(crate) multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
    /// 위에서만 막는 one-way 플랫폼으로 표시된 콜라이더 집합.
    /// `move_character`가 이동 방향에 따라 이 콜라이더와의 충돌을 동적으로 무시한다.
    one_way_colliders: std::collections::HashSet<ColliderHandle>,
}

impl PhysicsWorld {
    pub fn new(gravity: Vec2) -> Self {
        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            narrow_phase: NarrowPhase::new(),
            gravity: vector![gravity.x, gravity.y],
            integration_params: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            one_way_colliders: std::collections::HashSet::new(),
        }
    }

    /// 콜라이더를 one-way(위에서만 막는) 플랫폼으로 표시하거나 해제한다.
    ///
    /// one-way로 표시된 콜라이더는 [`PhysicsWorld::move_character`]에서 캐릭터가
    /// **아래로 내려오며 그 윗면 위에 있을 때만** 충돌하고, 위로 상승 중이거나
    /// drop 요청(`CharacterController::request_drop`) 중에는 통과한다.
    pub fn set_one_way(&mut self, handle: ColliderHandle, one_way: bool) {
        if one_way {
            self.one_way_colliders.insert(handle);
        } else {
            self.one_way_colliders.remove(&handle);
        }
    }

    /// 콜라이더가 one-way 플랫폼으로 표시되어 있는지 여부.
    pub fn is_one_way(&self, handle: ColliderHandle) -> bool {
        self.one_way_colliders.contains(&handle)
    }

    /// dt초 만큼 물리 시뮬레이션을 진행한다.
    pub fn step(&mut self, dt: f32) {
        self.integration_params.dt = dt;
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_params,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );
    }

    /// 콜라이더가 다른 오브젝트와 접촉 중인지 확인 (착지 판정에 사용).
    pub fn has_contact(&self, col_handle: ColliderHandle) -> bool {
        self.narrow_phase
            .contact_pairs_with(col_handle)
            .any(|pair| pair.has_any_active_contact)
    }

    // ── 타입 안전 접근자 ──────────────────────────────────────────────────────

    /// 핸들로 강체(rigid body)를 불변 참조로 가져온다.
    pub fn rigid_body(&self, handle: RigidBodyHandle) -> Option<&RigidBody> {
        self.rigid_body_set.get(handle)
    }

    /// 핸들로 강체를 가변 참조로 가져온다.
    pub fn rigid_body_mut(&mut self, handle: RigidBodyHandle) -> Option<&mut RigidBody> {
        self.rigid_body_set.get_mut(handle)
    }

    /// 핸들로 rapier 콜라이더를 불변 참조로 가져온다.
    pub fn get_collider(&self, handle: ColliderHandle) -> Option<&Collider> {
        self.collider_set.get(handle)
    }

    /// 핸들로 rapier 콜라이더를 가변 참조로 가져온다.
    pub fn get_collider_mut(&mut self, handle: ColliderHandle) -> Option<&mut Collider> {
        self.collider_set.get_mut(handle)
    }

    /// 콜라이더의 충돌 그룹을 변경한다. 핸들이 없으면 `false`를 반환한다.
    pub fn set_collision_groups(
        &mut self,
        handle: ColliderHandle,
        groups: CollisionGroups,
    ) -> bool {
        let Some(collider) = self.collider_set.get_mut(handle) else {
            return false;
        };
        collider.set_collision_groups(groups.to_rapier());
        true
    }

    /// 콜라이더의 현재 충돌 그룹을 반환한다.
    pub fn collision_groups(&self, handle: ColliderHandle) -> Option<CollisionGroups> {
        self.collider_set
            .get(handle)
            .map(|collider| CollisionGroups::from_rapier(collider.collision_groups()))
    }
}

#[cfg(test)]
mod tests;
