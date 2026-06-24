//! Reflect registry: register types for runtime field inspection and look them up
//! as `&dyn Reflect` / `&mut dyn Reflect` (drives the egui Inspector panel).

use super::{get_reflect_impl, get_reflect_mut_impl, Entity, ReflectEntry, World};
use std::any::TypeId;

impl World {
    /// Registers type T with the Reflect registry under a display name.
    ///
    /// Registered types are accessible via `get_reflect`, `get_reflect_mut`, and
    /// `reflected_components`, and are automatically displayed in the egui Inspector panel
    /// under `name`. This name is returned when querying via `reflect_registered_types()`.
    pub fn register_reflect_named<T: crate::reflect::Reflect + 'static>(
        &mut self,
        name: &'static str,
    ) {
        self.reflect_registry.insert(
            TypeId::of::<T>(),
            ReflectEntry {
                get: get_reflect_impl::<T>,
                get_mut: get_reflect_mut_impl::<T>,
                type_name: name,
            },
        );
    }

    /// Returns a list of `(TypeId, type_name)` for all types registered with the Reflect registry.
    pub fn reflect_registered_types(&self) -> Vec<(TypeId, &'static str)> {
        self.reflect_registry
            .iter()
            .map(|(&tid, entry)| (tid, entry.type_name))
            .collect()
    }

    /// Returns a specific component of an entity as `&dyn Reflect`.
    ///
    /// Returns `None` if the component for `type_id` is absent or the type is not registered.
    pub fn get_reflect(
        &self,
        entity: Entity,
        type_id: TypeId,
    ) -> Option<&dyn crate::reflect::Reflect> {
        let entry = self.reflect_registry.get(&type_id)?;
        let &(arch_id, row) = self.entity_location.get(&entity)?;
        let boxed = self.archetypes[arch_id].columns.get(&type_id)?.get(row)?;
        (entry.get)(boxed)
    }

    /// Returns a specific component of an entity as `&mut dyn Reflect`.
    ///
    /// Copies the `ReflectEntry` before mutably accessing the Archetype, so no borrow conflict.
    pub fn get_reflect_mut(
        &mut self,
        entity: Entity,
        type_id: TypeId,
    ) -> Option<&mut dyn crate::reflect::Reflect> {
        let entry = *self.reflect_registry.get(&type_id)?; // Copy → borrow released
        let &(arch_id, row) = self.entity_location.get(&entity)?;
        let boxed = self.archetypes[arch_id]
            .columns
            .get_mut(&type_id)?
            .get_mut(row)?;
        (entry.get_mut)(boxed)
    }

    /// Returns the `TypeId` list of components on the entity that are registered with the Reflect registry.
    pub fn reflected_components(&self, entity: Entity) -> Vec<TypeId> {
        let &(arch_id, _) = match self.entity_location.get(&entity) {
            Some(loc) => loc,
            None => return vec![],
        };
        self.archetypes[arch_id]
            .type_set
            .iter()
            .copied()
            .filter(|tid| self.reflect_registry.contains_key(tid))
            .collect()
    }
}
