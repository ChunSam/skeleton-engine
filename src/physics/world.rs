use glam::Vec2;
use rapier2d::prelude::ColliderHandle as RapierColliderHandle;
use rapier2d::prelude::RigidBodyHandle as RapierBodyHandle;
use rapier2d::prelude::*;

mod body_factory;
mod character_movement;
mod joints;
mod raycast;
mod tile_collider;

pub use tile_collider::TileColliderIndex;

// ── Body handle ───────────────────────────────────────────────────────────────

/// Opaque handle to a rigid body created by one of the `PhysicsWorld::add_*`
/// factory methods.
///
/// Wraps rapier's `RigidBodyHandle` so the rapier type does not leak through
/// the engine's public API. Pass it back to `PhysicsWorld` accessors such as
/// [`PhysicsWorld::rigid_body`] or [`PhysicsWorld::move_character`].
/// The inner handle is engine-private, so it can only be obtained from
/// `add_*` — not forged.
///
/// **Escape hatch for forks:** call [`BodyHandle::raw`] to retrieve the
/// underlying `rapier2d::prelude::RigidBodyHandle` if you need to drop down
/// to raw rapier APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyHandle(pub(crate) RapierBodyHandle);

impl BodyHandle {
    /// Returns the underlying rapier `RigidBodyHandle`.
    ///
    /// This is an escape hatch for forks that need direct rapier access.
    /// Prefer the engine's `PhysicsWorld` accessors where possible.
    pub fn raw(self) -> RapierBodyHandle {
        self.0
    }

    pub(crate) fn from_raw(h: RapierBodyHandle) -> Self {
        Self(h)
    }
}

// ── Collider handle ───────────────────────────────────────────────────────────

/// Opaque handle to a collider created by one of the `PhysicsWorld::add_*`
/// factory methods.
///
/// Wraps rapier's `ColliderHandle` so the rapier type does not leak through
/// the engine's public API. Pass it back to `PhysicsWorld` accessors such as
/// [`PhysicsWorld::get_collider`], [`PhysicsWorld::set_one_way`], or
/// [`PhysicsWorld::cast_ray`].
/// The inner handle is engine-private, so it can only be obtained from
/// `add_*` — not forged.
///
/// **Escape hatch for forks:** call [`ColliderHandle::raw`] to retrieve the
/// underlying `rapier2d::prelude::ColliderHandle` if you need to drop down
/// to raw rapier APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColliderHandle(pub(crate) RapierColliderHandle);

impl ColliderHandle {
    /// Returns the underlying rapier `ColliderHandle`.
    ///
    /// This is an escape hatch for forks that need direct rapier access.
    /// Prefer the engine's `PhysicsWorld` accessors where possible.
    pub fn raw(self) -> RapierColliderHandle {
        self.0
    }

    pub(crate) fn from_raw(h: RapierColliderHandle) -> Self {
        Self(h)
    }
}

// ── Joint handle ─────────────────────────────────────────────────────────────

/// Opaque handle to a physics joint created by one of the
/// `PhysicsWorld::add_*_joint` methods.
///
/// Wraps rapier's `ImpulseJointHandle` so the rapier type does not leak through
/// the engine's public API (mirrors how [`CollisionGroups`] wraps
/// `InteractionGroups`). Pass it back to [`PhysicsWorld::remove_joint`] to remove
/// the joint. The inner handle is engine-private, so it can only be obtained from
/// `add_*_joint` — not forged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JointHandle(pub(crate) ImpulseJointHandle);

// ── Collision groups ──────────────────────────────────────────────────────────

/// Engine-level collision layer/mask wrapping Rapier's `InteractionGroups`.
///
/// `memberships` is the layer bitmask this collider belongs to; `filter` is the
/// bitmask of layers it is allowed to interact with. Both colliders must permit
/// each other for a collision or sensor overlap to occur.
///
/// **Distinction from `collision::CollisionLayer`:** `CollisionGroups` controls
/// Rapier physics filtering — which rigid bodies and sensors physically interact
/// inside the physics simulation. `collision::CollisionLayer` is an independent
/// engine-side tag used by `SpatialGrid` / `CollisionGridSystem` for broad-phase
/// overlap queries that are entirely outside the physics world (no forces, no
/// solver). Use `CollisionGroups` when you need physics-accurate responses; use
/// `CollisionLayer` when you only need fast AABB overlap lookups.
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

// ── Raycast result ────────────────────────────────────────────────────────────

/// Result of a raycast hit.
#[derive(Debug, Clone, Copy)]
pub struct RaycastHit {
    /// Engine handle of the collider that was hit.
    pub collider_handle: ColliderHandle,
    /// Hit point in world space (physics units).
    pub point: Vec2,
    /// Normal vector of the hit surface (normalized).
    pub normal: Vec2,
    /// Distance multiplier from the ray origin (`origin + direction * toi` = hit point).
    pub toi: f32,
}

/// Kind of collider that [`PhysicsWorld::add_static_from_tilemap`] creates per tile.
///
/// Specifies the collision group and whether the tile is one-way.
/// Convenience constructors cover the common cases:
/// [`TileCollider::solid`] (blocks all directions), [`TileCollider::one_way`] (blocks from above only).
#[derive(Debug, Clone, Copy)]
pub struct TileCollider {
    /// Collision groups for this collider.
    pub groups: CollisionGroups,
    /// When `true`, acts as a one-way platform that only blocks from above.
    pub one_way: bool,
}

impl TileCollider {
    /// A solid tile that blocks all directions (`CollisionGroups::all`).
    pub fn solid() -> Self {
        Self {
            groups: CollisionGroups::all(),
            one_way: false,
        }
    }

    /// A solid tile with a specific collision group.
    pub fn solid_with(groups: CollisionGroups) -> Self {
        Self {
            groups,
            one_way: false,
        }
    }

    /// A one-way platform tile that blocks only from above; passable from below and through.
    pub fn one_way() -> Self {
        Self {
            groups: CollisionGroups::all(),
            one_way: true,
        }
    }
}

/// rapier2d 2D physics simulation world.
///
/// Owned by `PhysicsSystem` or embedded directly in a system struct.
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
    /// Set of colliders marked as one-way platforms (block from above only).
    /// `move_character` dynamically ignores collisions with these based on movement direction.
    pub(crate) one_way_colliders: std::collections::HashSet<RapierColliderHandle>,
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

    /// Marks or unmarks a collider as a one-way (blocks from above only) platform.
    ///
    /// A one-way collider only collides in [`PhysicsWorld::move_character`] when the
    /// character is **descending and above the platform's top surface**. It is passed
    /// through while ascending or during a drop request (`CharacterController::request_drop`).
    pub fn set_one_way(&mut self, handle: ColliderHandle, one_way: bool) {
        if one_way {
            self.one_way_colliders.insert(handle.0);
        } else {
            self.one_way_colliders.remove(&handle.0);
        }
    }

    /// Returns whether a collider is marked as a one-way platform.
    pub fn is_one_way(&self, handle: ColliderHandle) -> bool {
        self.one_way_colliders.contains(&handle.0)
    }

    /// Advances the physics simulation by `dt` seconds.
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

    /// Returns whether a collider is currently in contact with another object (used for ground detection).
    pub fn has_contact(&self, col_handle: ColliderHandle) -> bool {
        self.narrow_phase
            .contact_pairs_with(col_handle.0)
            .any(|pair| pair.has_any_active_contact)
    }

    // ── Type-safe accessors ───────────────────────────────────────────────────

    /// Returns an immutable reference to the rigid body for the given handle.
    ///
    /// Returns the raw rapier [`RigidBody`] — an escape hatch for forks that need
    /// direct rapier access beyond what the engine API provides.
    pub fn rigid_body(&self, handle: BodyHandle) -> Option<&RigidBody> {
        self.rigid_body_set.get(handle.0)
    }

    /// Returns a mutable reference to the rigid body for the given handle.
    ///
    /// Returns the raw rapier [`RigidBody`] — an escape hatch for forks that need
    /// direct rapier access beyond what the engine API provides.
    pub fn rigid_body_mut(&mut self, handle: BodyHandle) -> Option<&mut RigidBody> {
        self.rigid_body_set.get_mut(handle.0)
    }

    /// Returns an immutable reference to the rapier collider for the given handle.
    ///
    /// Returns the raw rapier [`Collider`] — an escape hatch for forks that need
    /// direct rapier access beyond what the engine API provides.
    pub fn get_collider(&self, handle: ColliderHandle) -> Option<&Collider> {
        self.collider_set.get(handle.0)
    }

    /// Returns a mutable reference to the rapier collider for the given handle.
    ///
    /// Returns the raw rapier [`Collider`] — an escape hatch for forks that need
    /// direct rapier access beyond what the engine API provides.
    pub fn get_collider_mut(&mut self, handle: ColliderHandle) -> Option<&mut Collider> {
        self.collider_set.get_mut(handle.0)
    }

    /// Changes the collision groups of a collider. Returns `false` if the handle is not found.
    pub fn set_collision_groups(
        &mut self,
        handle: ColliderHandle,
        groups: CollisionGroups,
    ) -> bool {
        let Some(collider) = self.collider_set.get_mut(handle.0) else {
            return false;
        };
        collider.set_collision_groups(groups.to_rapier());
        true
    }

    /// Returns the current collision groups of a collider.
    pub fn collision_groups(&self, handle: ColliderHandle) -> Option<CollisionGroups> {
        self.collider_set
            .get(handle.0)
            .map(|collider| CollisionGroups::from_rapier(collider.collision_groups()))
    }
}

#[cfg(test)]
mod tests;
