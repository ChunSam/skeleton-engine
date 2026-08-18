//! Entity cloning: register clonable component types and deep-copy a registered
//! subset of an entity's components into a fresh entity.

use super::{Entity, World};
use std::any::TypeId;

impl World {
    /// Registers component T so it can be cloned by `clone_entity`.
    ///
    /// T must be Clone + Send + Sync + 'static.
    /// Unregistered components are not copied during `clone_entity`.
    pub fn register_clone<T: Clone + Send + Sync + 'static>(&mut self) {
        self.clone_registry.insert(
            TypeId::of::<T>(),
            Box::new(|world, src, dst| {
                if let Some(comp) = world.get::<T>(src) {
                    let cloned = comp.clone();
                    world.add_component(dst, cloned);
                }
            }),
        );
    }

    /// Clones an entity. Only components registered with `register_clone` are copied.
    ///
    /// Returns `None` if `src` is not alive.
    ///
    /// Components are copied in a fixed `TypeId` order, so the archetype layout the clone
    /// leaves behind — and therefore query iteration order — is the same on every run.
    pub fn clone_entity(&mut self, src: Entity) -> Option<Entity> {
        if !self.is_alive(src) {
            return None;
        }

        // 1. Collect TypeIds from clone_registry that src actually has
        let mut tids: Vec<TypeId> = self
            .clone_registry
            .keys()
            .filter(|&&tid| self.has_component_typeid(src, tid))
            .copied()
            .collect();
        // `clone_registry` is a `HashMap`, whose iteration order is seeded per process. Each
        // `add_component` below walks the entity through a different intermediate archetype
        // signature, so an unsorted order left `world.archetypes` in a different Vec order on
        // every launch — and `query::<T>()` iterates archetypes in exactly that order. One
        // editor Duplicate was enough to make a replay diverge: the sprite sort
        // (`renderer/sprite/sort.rs`) is stable, so equal keys draw in query order, and
        // `prefab.rs`'s parent-tag resolution takes the first `query::<Tag>()` match.
        tids.sort_unstable();

        // 2. Spawn the destination entity
        let dst = self.spawn();

        // 3. remove → call → reinsert pattern to clone without borrow conflicts
        for tid in tids {
            self.clone_component_by_typeid(src, dst, tid);
        }

        Some(dst)
    }
}
