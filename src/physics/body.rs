use crate::physics::world::{BodyHandle, ColliderHandle};

/// Component attached to entities that have a physics body.
/// Stores the handles returned by `PhysicsWorld::add_dynamic_box` / `add_static_box`.
pub struct PhysicsBody {
    pub rigid_body_handle: BodyHandle,
    pub collider_handle: ColliderHandle,
}
