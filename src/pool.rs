use crate::ecs::{Entity, World};
use std::collections::VecDeque;

/// An ECS entity reuse pool.
///
/// Recycles frequently created/destroyed entities (bullets, particles, etc.)
/// to reduce archetype reallocation costs.
///
/// # Usage pattern
///
/// ```rust,ignore
/// // Register as a resource
/// world.insert_resource(Pool::new(32));
///
/// // Acquire (spawns a new entity if none are available)
/// let bullet = pool.acquire(world, |w, e| {
///     w.add_component(e, Bullet::default());
///     w.add_component(e, Transform::default());
/// });
///
/// // Release (entity is kept alive; an inactive marker is added)
/// pool.release(bullet, world);
/// ```
pub struct Pool {
    available: VecDeque<Entity>,
    capacity: usize,
}

impl Pool {
    /// Creates a pool that stores up to `capacity` entities.
    pub fn new(capacity: usize) -> Self {
        Self {
            available: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Acquires an entity from the pool.
    ///
    /// If no entity is available, a new one is created with `world.spawn()`.
    /// The `setup` closure initializes the entity's components.
    pub fn acquire(&mut self, world: &mut World, setup: impl FnOnce(&mut World, Entity)) -> Entity {
        // Try to reuse an existing entity
        while let Some(entity) = self.available.pop_front() {
            if world.is_alive(entity) {
                // Remove the Pooled marker to "activate" it
                world.remove_component::<Pooled>(entity);
                setup(world, entity);
                return entity;
            }
            // Entity was despawned externally — skip it
        }
        // No available entity — spawn a new one
        let entity = world.spawn();
        setup(world, entity);
        entity
    }

    /// Returns an entity to the pool.
    ///
    /// If the pool is full (exceeds `capacity`), the entity is despawned.
    /// A `Pooled` marker component is added to indicate the inactive state.
    pub fn release(&mut self, entity: Entity, world: &mut World) {
        // A double release used to push the same entity into `available` twice, so two later
        // `acquire` calls handed the SAME `Entity` to two different callers, which then wrote
        // over each other's components with no error anywhere. `Pooled` is precisely the
        // "currently parked" marker (`acquire` removes it), so its presence is the cheap test.
        if world.get::<Pooled>(entity).is_some() {
            log::warn!(
                "Pool::release: {entity:?} is already parked in this pool — ignoring the \
                 double release (it would hand the same entity to two acquirers)"
            );
            return;
        }
        if !world.is_alive(entity) {
            log::warn!("Pool::release: {entity:?} is not alive — ignoring");
            return;
        }
        if self.available.len() >= self.capacity {
            world.despawn(entity);
            return;
        }
        world.add_component(entity, Pooled);
        self.available.push_back(entity);
    }

    /// Returns the number of entities currently available in the pool.
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// Returns the maximum capacity of the pool.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Despawns all entities in the pool and empties it.
    pub fn clear(&mut self, world: &mut World) {
        for entity in self.available.drain(..) {
            if world.is_alive(entity) {
                world.despawn(entity);
            }
        }
    }
}

/// Marker component that tags an entity returned to the object pool.
///
/// Entities with this component are in an "inactive" state.
/// Exclude them from rendering/systems with `query_without::<Pooled>()`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Pooled;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Bullet {
        speed: f32,
    }

    #[test]
    fn acquire_spawns_when_empty() {
        let mut world = World::new();
        let mut pool = Pool::new(4);
        let e = pool.acquire(&mut world, |w, e| {
            w.add_component(e, Bullet { speed: 10.0 });
        });
        assert!(world.is_alive(e));
        assert!(world.get::<Bullet>(e).is_some());
        assert_eq!(pool.available_count(), 0);
    }

    #[test]
    fn double_release_does_not_hand_one_entity_to_two_acquirers() {
        // Releasing the same entity twice used to push it into `available` twice, so the next
        // two `acquire` calls returned the SAME `Entity` to two different callers, which then
        // silently wrote over each other's components.
        let mut world = World::new();
        let mut pool = Pool::new(4);
        let e = pool.acquire(&mut world, |_, _| {});
        pool.release(e, &mut world);
        pool.release(e, &mut world); // ignored
        assert_eq!(
            pool.available_count(),
            1,
            "a double release must not park the entity twice"
        );

        let a = pool.acquire(&mut world, |_, _| {});
        let b = pool.acquire(&mut world, |_, _| {});
        assert_ne!(a, b, "two acquires must never yield the same Entity");
    }

    #[test]
    fn release_and_reacquire() {
        let mut world = World::new();
        let mut pool = Pool::new(4);
        let e1 = pool.acquire(&mut world, |w, e| {
            w.add_component(e, Bullet { speed: 5.0 });
        });
        pool.release(e1, &mut world);
        assert_eq!(pool.available_count(), 1);
        // Pooled marker should be added
        assert!(world.get::<Pooled>(e1).is_some());

        // Reacquire — should return same entity
        let e2 = pool.acquire(&mut world, |w, e| {
            w.add_component(e, Bullet { speed: 20.0 });
        });
        assert_eq!(e1, e2);
        assert_eq!(pool.available_count(), 0);
        // Pooled marker removed on reacquire
        assert!(world.get::<Pooled>(e2).is_none());
        assert_eq!(world.get::<Bullet>(e2).unwrap().speed, 20.0);
    }

    #[test]
    fn overflow_despawns_entity() {
        let mut world = World::new();
        let mut pool = Pool::new(1); // capacity = 1
        let e1 = pool.acquire(&mut world, |_, _| {});
        let e2 = pool.acquire(&mut world, |_, _| {});
        pool.release(e1, &mut world); // fills pool
        pool.release(e2, &mut world); // overflow → despawn e2
        assert!(world.is_alive(e1));
        assert!(!world.is_alive(e2));
    }

    #[test]
    fn clear_despawns_all() {
        let mut world = World::new();
        let mut pool = Pool::new(4);
        let e1 = pool.acquire(&mut world, |_, _| {});
        let e2 = pool.acquire(&mut world, |_, _| {});
        pool.release(e1, &mut world);
        pool.release(e2, &mut world);
        assert_eq!(pool.available_count(), 2);
        pool.clear(&mut world);
        assert_eq!(pool.available_count(), 0);
        assert!(!world.is_alive(e1));
        assert!(!world.is_alive(e2));
    }

    #[test]
    fn skips_externally_despawned_entity() {
        let mut world = World::new();
        let mut pool = Pool::new(4);
        let e = pool.acquire(&mut world, |_, _| {});
        pool.release(e, &mut world);
        // Externally despawn the pooled entity
        world.despawn(e);
        // acquire should gracefully skip dead entity and return a live entity
        let e2 = pool.acquire(&mut world, |_, _| {});
        assert!(world.is_alive(e2));
        assert_eq!(pool.available_count(), 0);
    }
}
