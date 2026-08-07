use crate::physics::world::{BodyHandle, ColliderHandle};

/// Component attached to entities that have a physics body.
/// Stores the handles returned by `PhysicsWorld::add_dynamic_box` / `add_static_box`.
///
/// # ⚠️ Despawning is not enough
///
/// The rigid body and collider live in [`PhysicsWorld`](crate::PhysicsWorld), **not** in the ECS,
/// so `world.despawn(entity)` drops only this component. The body stays in the rapier world and
/// keeps colliding — an invisible solid that blocks movement and emits collision events with no
/// entity behind them — and it is never reclaimed, so a game that spawns and despawns physics
/// entities leaks one body per spawn for as long as it runs.
///
/// Use [`despawn_with_body`] instead of `World::despawn`, or call
/// [`PhysicsWorld::remove_body`](crate::PhysicsWorld::remove_body) yourself first. (This mirrors
/// the same warning on [`TilemapColliders`](crate::TilemapColliders).)
///
/// ```rust,no_run
/// # use engine::{PhysicsWorld, physics::body::despawn_with_body};
/// # use engine::ecs::World;
/// # let mut world = World::new();
/// # let e = world.spawn();
/// # world.insert_resource(PhysicsWorld::new(glam::Vec2::ZERO));
/// despawn_with_body(&mut world, e);
/// ```
pub struct PhysicsBody {
    pub rigid_body_handle: BodyHandle,
    pub collider_handle: ColliderHandle,
}

/// Despawns `entity` **and** removes its rigid body + colliders from [`PhysicsWorld`](crate::PhysicsWorld).
///
/// This is what `World::despawn` cannot do on its own: the ECS knows nothing about the rapier
/// world, so despawning a [`PhysicsBody`] entity by hand leaves an invisible, still-colliding
/// ghost behind and leaks the body. Safe to call on an entity with no `PhysicsBody` (it just
/// despawns) and when no `PhysicsWorld` resource exists.
pub fn despawn_with_body(world: &mut crate::ecs::World, entity: crate::ecs::Entity) {
    if world.get::<PhysicsBody>(entity).is_some() {
        world.with_resource_mut::<crate::PhysicsWorld, _>(|physics, world| {
            if let Some(body) = world.get::<PhysicsBody>(entity) {
                physics.remove_body(body);
            }
        });
    }
    world.despawn(entity);
}
