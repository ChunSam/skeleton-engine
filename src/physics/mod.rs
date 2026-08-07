pub mod body;
pub mod character;
pub mod events;
pub mod system;
pub mod world;

pub use body::{despawn_with_body, PhysicsBody};
pub use character::CharacterController;
pub use events::{CollisionEvent, TriggerEvent};
pub use system::PhysicsSystem;
pub use world::{
    sync_tilemap_entity_colliders, BodyHandle, ColliderHandle, CollisionGroups, JointHandle,
    PhysicsWorld, RaycastHit, SolidTiles, TileCollider, TileColliderIndex, TilemapColliders,
};

/// Default friction coefficient applied to colliders created by the `add_*` body factories.
/// Override per-collider with [`PhysicsWorld::set_collider_friction`].
pub const DEFAULT_FRICTION: f32 = 0.3;

/// Default restitution (bounciness) applied to colliders created by the `add_*` body factories.
/// Override per-collider with [`PhysicsWorld::set_collider_restitution`].
pub const DEFAULT_RESTITUTION: f32 = 0.0;
