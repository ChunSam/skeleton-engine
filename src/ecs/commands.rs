use crate::ecs::{Entity, World};

/// Type alias for a deferred World-mutation function.
type DeferredFn = Box<dyn FnOnce(&mut World) + Send>;

/// A buffer that safely defers entity/component changes while an ECS system is running.
///
/// A system cannot call `world.spawn()` etc. directly while it already holds a borrow
/// on `world`. `Commands` enqueues mutation closures and applies them all at once after
/// the system finishes, via `world.apply_commands(cmds)`.
///
/// # Example
///
/// ```rust,ignore
/// use engine::{Commands, System, World};
///
/// struct SpawnSystem;
/// impl System for SpawnSystem {
///     fn run(&mut self, world: &mut World, _dt: f32) {
///         let mut cmds = Commands::new();
///
///         // Spawn a new entity
///         cmds.spawn(|world, e| {
///             world.add_component(e, MyTag);
///         });
///
///         // Add a component to an existing entity
///         let entities: Vec<_> = world.query::<MyComp>().map(|(e, _)| e).collect();
///         for entity in entities {
///             cmds.insert(entity, NewComp { value: 42 });
///         }
///
///         world.apply_commands(cmds);
///     }
/// }
/// ```
pub struct Commands {
    deferred: Vec<DeferredFn>,
}

impl Commands {
    /// Creates an empty Commands buffer.
    pub fn new() -> Self {
        Self {
            deferred: Vec::new(),
        }
    }

    /// Schedules a command to spawn a new entity.
    ///
    /// The closure receives the newly created `Entity` and `&mut World` at apply time,
    /// so components can be added freely inside it.
    ///
    /// ```rust,ignore
    /// cmds.spawn(|world, e| {
    ///     world.add_component(e, Transform::default());
    ///     world.add_component(e, Sprite::colored(1.0, 0.0, 0.0));
    /// });
    /// ```
    pub fn spawn(&mut self, f: impl FnOnce(&mut World, Entity) + Send + 'static) {
        self.deferred.push(Box::new(move |world: &mut World| {
            let e = world.spawn();
            f(world, e);
        }));
    }

    /// Schedules a command to despawn an existing entity.
    ///
    /// If the entity has already been despawned by the time `apply` runs, this is silently ignored (idempotent).
    pub fn despawn(&mut self, entity: Entity) {
        self.deferred.push(Box::new(move |world: &mut World| {
            world.despawn(entity);
        }));
    }

    /// Schedules a command to insert a component into an existing entity.
    ///
    /// If a component of the same type already exists, it is replaced.
    /// If the entity no longer exists at apply time, this is silently ignored.
    pub fn insert<T: Send + Sync + 'static>(&mut self, entity: Entity, comp: T) {
        self.deferred.push(Box::new(move |world: &mut World| {
            world.add_component(entity, comp);
        }));
    }

    /// Schedules a command to remove a component from an existing entity.
    ///
    /// Silently ignored if the component is absent or the entity does not exist (idempotent).
    pub fn remove<T: Send + Sync + 'static>(&mut self, entity: Entity) {
        self.deferred.push(Box::new(move |world: &mut World| {
            world.remove_component::<T>(entity);
        }));
    }

    /// Applies all buffered commands to the World in order.
    ///
    /// Prefer `world.apply_commands(cmds)` in normal usage.
    pub fn apply(self, world: &mut World) {
        for f in self.deferred {
            f(world);
        }
    }
}

impl Default for Commands {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Unit tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Health(u32);

    #[derive(Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }

    /// 1. spawn — after Commands::spawn + apply, a new entity exists in the world
    #[test]
    fn spawn_creates_entity() {
        let mut world = World::new();
        let mut cmds = Commands::new();

        cmds.spawn(|world, e| {
            world.add_component(e, Health(100));
        });

        assert_eq!(world.entity_count(), 0);
        world.apply_commands(cmds);
        assert_eq!(world.entity_count(), 1);

        let count = world.query::<Health>().count();
        assert_eq!(count, 1);

        let health = world.query::<Health>().next().map(|(_, h)| h.0).unwrap();
        assert_eq!(health, 100);
    }

    /// 2. despawn — after Commands::despawn + apply, the entity is removed
    #[test]
    fn despawn_removes_entity() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Health(50));

        let mut cmds = Commands::new();
        cmds.despawn(e);

        assert_eq!(world.entity_count(), 1);
        world.apply_commands(cmds);
        assert_eq!(world.entity_count(), 0);
        assert!(!world.is_alive(e));
    }

    /// 3. insert — component is added to an existing entity
    #[test]
    fn insert_adds_component() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Health(10));

        let mut cmds = Commands::new();
        cmds.insert(e, Position { x: 1.0, y: 2.0 });

        assert!(world.get::<Position>(e).is_none());
        world.apply_commands(cmds);
        let pos = world.get::<Position>(e).unwrap();
        assert_eq!(pos.x, 1.0);
        assert_eq!(pos.y, 2.0);
    }

    /// 4. remove — component is removed from an existing entity
    #[test]
    fn remove_removes_component() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Health(99));
        world.add_component(e, Position { x: 5.0, y: 5.0 });

        let mut cmds = Commands::new();
        cmds.remove::<Position>(e);

        assert!(world.get::<Position>(e).is_some());
        world.apply_commands(cmds);
        assert!(world.get::<Position>(e).is_none());
        // Health should remain unchanged
        assert_eq!(world.get::<Health>(e).unwrap().0, 99);
    }

    /// 5. ordering guarantee — spawn → insert applied in order works correctly
    #[test]
    fn spawn_then_insert_ordering() {
        let mut world = World::new();
        let mut cmds = Commands::new();

        // Add components during spawn (inside the closure)
        cmds.spawn(|world, e| {
            world.add_component(e, Health(42));
            world.add_component(e, Position { x: 10.0, y: 20.0 });
        });

        world.apply_commands(cmds);

        let results: Vec<_> = world.query2::<Health, Position>().collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1 .0, 42);
        assert_eq!(results[0].2.x, 10.0);
        assert_eq!(results[0].2.y, 20.0);
    }

    /// 6. multiple spawns — all entities are created when several are scheduled at once
    #[test]
    fn multiple_spawns() {
        let mut world = World::new();
        let mut cmds = Commands::new();

        for i in 0..5u32 {
            cmds.spawn(move |world, e| {
                world.add_component(e, Health(i * 10));
            });
        }

        world.apply_commands(cmds);
        assert_eq!(world.entity_count(), 5);
        assert_eq!(world.query::<Health>().count(), 5);
    }

    /// 7. despawn on a non-existent entity does not panic (idempotent)
    #[test]
    fn despawn_nonexistent_is_noop() {
        let mut world = World::new();
        let e = world.spawn();
        world.despawn(e); // removed directly

        let mut cmds = Commands::new();
        cmds.despawn(e); // entity no longer exists
        world.apply_commands(cmds); // should not panic
    }

    /// 8. insert on a non-existent entity does not panic
    #[test]
    fn insert_nonexistent_entity_is_noop() {
        let mut world = World::new();
        let e = world.spawn();
        world.despawn(e);

        let mut cmds = Commands::new();
        cmds.insert(e, Health(1));
        world.apply_commands(cmds); // should not panic
    }
}
