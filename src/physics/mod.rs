pub mod body;
pub mod character;
pub mod events;
pub mod system;
pub mod world;

pub use body::PhysicsBody;
pub use character::CharacterController;
pub use events::{CollisionEvent, TriggerEvent};
// TODO(#28, breaking): wrap this rapier handle in an engine `JointHandle` newtype
// (like `CollisionGroups`) so rapier doesn't leak through the public API. Deferred
// — it's a breaking change, batch with a future v3.x.
pub use rapier2d::prelude::ImpulseJointHandle;
pub use system::PhysicsSystem;
pub use world::{CollisionGroups, PhysicsWorld, RaycastHit, TileCollider};
