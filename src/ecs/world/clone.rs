//! Entity cloning: register clonable component types and deep-copy a registered
//! subset of an entity's components into a fresh entity.

use super::{Entity, World};
use std::any::TypeId;
use std::sync::Arc;

impl World {
    /// Registers component T so it can be cloned by `clone_entity`.
    ///
    /// T must be Clone + Send + Sync + 'static.
    /// Unregistered components are not copied during `clone_entity`.
    pub fn register_clone<T: Clone + Send + Sync + 'static>(&mut self) {
        self.clone_registry.insert(
            TypeId::of::<T>(),
            Arc::new(|world, src, dst| {
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
    pub fn clone_entity(&mut self, src: Entity) -> Option<Entity> {
        if !self.is_alive(src) {
            return None;
        }

        // 1. Collect TypeIds from clone_registry that src actually has
        let tids: Vec<TypeId> = self
            .clone_registry
            .keys()
            .filter(|&&tid| self.has_component_typeid(src, tid))
            .copied()
            .collect();

        // 2. Spawn the destination entity
        let dst = self.spawn();

        // 3. remove → call → reinsert pattern to clone without borrow conflicts
        for tid in tids {
            self.clone_component_by_typeid(src, dst, tid);
        }

        Some(dst)
    }
}
