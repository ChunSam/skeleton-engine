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
/// Use [`despawn_with_body`] instead of `World::despawn`, or [`release_physics`] before whatever
/// despawns the entity for you (a subtree through `hierarchy::despawn_recursive`, a scene load).
/// (This mirrors the same warning on [`TilemapColliders`](crate::TilemapColliders).)
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

/// Removes everything `entity` owns in [`PhysicsWorld`](crate::PhysicsWorld) — a [`PhysicsBody`]'s
/// rigid body and colliders, and every tile collider its
/// [`TilemapColliders`](crate::TilemapColliders) tracks — **without** despawning it.
///
/// The half of [`despawn_with_body`] that the ECS cannot do, on its own for callers that despawn
/// some other way: a subtree through `hierarchy::despawn_recursive`, or a whole scene through
/// `World::despawn` in a loop. Both are storage-level by design and know nothing about rapier,
/// so without this call the bodies stay behind as invisible, still-colliding ghosts — which is
/// how the editor's 📂 Load and 🗑 Delete leaked until v0.156.8. No-op without a `PhysicsWorld`,
/// or for an entity that owns nothing in it.
pub fn release_physics(world: &mut crate::ecs::World, entity: crate::ecs::Entity) {
    use crate::physics::world::TilemapColliders;
    let owns_body = world.get::<PhysicsBody>(entity).is_some();
    let owns_tiles = world
        .get::<TilemapColliders>(entity)
        .is_some_and(|t| t.collider_count() > 0);
    if !(owns_body || owns_tiles) {
        return;
    }
    world.with_resource_mut::<crate::PhysicsWorld, _>(|physics, world| {
        if let Some(body) = world.get::<PhysicsBody>(entity) {
            physics.remove_body(body);
        }
        if let Some(tiles) = world.get_mut::<TilemapColliders>(entity) {
            tiles.drain_into_physics(physics);
        }
    });
}

/// Despawns `entity` **and** releases everything it owns in [`PhysicsWorld`](crate::PhysicsWorld)
/// — see [`release_physics`]; since v0.156.8 that includes a `TilemapColliders`' tile colliders.
///
/// This is what `World::despawn` cannot do on its own: the ECS knows nothing about the rapier
/// world, so despawning a [`PhysicsBody`] entity by hand leaves an invisible, still-colliding
/// ghost behind and leaks the body. Safe to call on an entity that owns no physics (it just
/// despawns) and when no `PhysicsWorld` resource exists.
pub fn despawn_with_body(world: &mut crate::ecs::World, entity: crate::ecs::Entity) {
    release_physics(world, entity);
    world.despawn(entity);
}

#[cfg(test)]
mod tests {
    use super::{release_physics, PhysicsBody};
    use crate::ecs::World;
    use crate::physics::world::{SolidTiles, TilemapColliders};
    use crate::PhysicsWorld;

    fn bodies(world: &World) -> usize {
        world
            .resource::<PhysicsWorld>()
            .expect("PhysicsWorld present")
            .rigid_body_set
            .len()
    }

    /// One body and nine tile colliders go in; `release_physics` takes both halves out, a second
    /// call is a no-op, and an entity that owns nothing leaves the count alone.
    #[test]
    fn release_physics_removes_a_body_and_its_tile_colliders() {
        let mut world = World::new();
        world.insert_resource(PhysicsWorld::new(glam::Vec2::ZERO));
        let baseline = bodies(&world);

        let body = world.spawn();
        let (rb, col) = world
            .resource_mut::<PhysicsWorld>()
            .unwrap()
            .add_dynamic_box(glam::Vec2::ZERO, 8.0, 8.0, true);
        world.add_component(
            body,
            PhysicsBody {
                rigid_body_handle: rb,
                collider_handle: col,
            },
        );
        let map = world.spawn();
        world.add_component(
            map,
            crate::tilemap::Tilemap::new(
                crate::tilemap::TilemapAtlas::new("", 1, 1),
                vec![vec![1u32; 3]; 3],
                32.0,
                glam::Vec2::ZERO,
            ),
        );
        world.add_component(map, TilemapColliders::new(32.0, SolidTiles::NonZero));
        assert!(crate::physics::sync_tilemap_entity_colliders(
            &mut world, map
        ));
        assert_eq!(
            bodies(&world),
            baseline + 10,
            "precondition: 1 body + 9 tile colliders"
        );

        release_physics(&mut world, body);
        assert_eq!(bodies(&world), baseline + 9, "the body half is released");
        release_physics(&mut world, map);
        assert_eq!(
            bodies(&world),
            baseline,
            "the tile-collider half is released"
        );
        assert_eq!(
            world.get::<TilemapColliders>(map).unwrap().collider_count(),
            0,
            "the index is drained, so a later sync rebuilds from scratch"
        );

        // A second release is a no-op, and so is one on an entity that owns nothing.
        release_physics(&mut world, map);
        let bare = world.spawn();
        release_physics(&mut world, bare);
        assert_eq!(bodies(&world), baseline);
        assert!(
            world.is_alive(body) && world.is_alive(map),
            "release does not despawn"
        );
    }

    #[test]
    fn release_physics_without_a_physics_world_is_a_no_op() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, TilemapColliders::new(32.0, SolidTiles::NonZero));
        release_physics(&mut world, e);
        assert!(world.is_alive(e));
    }
}
