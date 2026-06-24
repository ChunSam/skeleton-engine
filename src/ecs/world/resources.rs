//! Global singleton storage: typed `insert`/`get`/`remove`, the
//! `with_resource_mut` borrow helper, and the type-erased take/insert pair.

use super::World;
use std::any::{Any, TypeId};

impl World {
    pub fn insert_resource<T: 'static>(&mut self, resource: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }

    pub fn resource<T: 'static>(&self) -> Option<&T> {
        self.resources.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    pub fn resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<T>()
    }

    /// Removes resource `T` from the World and returns ownership.
    /// Returns `None` if the resource does not exist.
    pub fn remove_resource<T: 'static>(&mut self) -> Option<T> {
        self.resources
            .remove(&TypeId::of::<T>())
            .and_then(|b| b.downcast::<T>().ok().map(|b| *b))
    }

    /// Temporarily removes resource `R`, runs `f` with a mutable borrow of both `R`
    /// and the `World` (so a system can touch `R` and other world state at once
    /// without the borrow checker fighting), then re-inserts `R`. Returns `false`
    /// if `R` was not present (and `f` is not called).
    ///
    /// Hides the manual `remove_resource` / `insert_resource` dance. `R` is removed
    /// for the duration of `f`, so `f` must not assume `world.resource::<R>()` is
    /// available re-entrantly.
    pub fn with_resource_mut<R, F>(&mut self, f: F) -> bool
    where
        R: 'static,
        F: FnOnce(&mut R, &mut World),
    {
        match self.remove_resource::<R>() {
            Some(mut r) => {
                f(&mut r, self);
                self.insert_resource(r);
                true
            }
            None => false,
        }
    }

    /// Removes a resource by `TypeId` and returns it as a type-erased box (ownership transferred).
    ///
    /// Use this to move a resource into another `World` without knowing its static type
    /// (e.g. preserving persistent resources across scene transitions).
    /// Normal game code should use [`World::remove_resource`] instead.
    pub fn take_resource_erased(&mut self, type_id: TypeId) -> Option<Box<dyn Any>> {
        self.resources.remove(&type_id)
    }

    /// Re-inserts a box previously removed by [`World::take_resource_erased`].
    ///
    /// `type_id` must be the `TypeId` of the actual type held in the box
    /// (typically the same one used when taking it out).
    pub fn insert_resource_erased(&mut self, type_id: TypeId, resource: Box<dyn Any>) {
        self.resources.insert(type_id, resource);
    }
}
