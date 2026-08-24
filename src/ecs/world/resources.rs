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
    ///
    /// Two guarantees the naive dance does not give you:
    ///
    /// - **A panic in `f` does not lose `R`.** The resource is restored before the panic is
    ///   re-raised unchanged.
    /// - **A replacement wins.** If `f` inserts a fresh `R` through its `&mut World`, that value
    ///   is kept; the one taken at entry is dropped.
    ///
    /// ⚠️ **Read those two together, not separately.** The restore is *conditional on `R` being
    /// absent at exit*, and that condition decides both cases where the guarantees meet:
    ///
    /// - **Replace, then panic — the replacement wins.** `R` is present, so it is not overwritten,
    ///   and every mutation `f` made to the value it was handed is dropped with that value. "A
    ///   panic does not lose `R`" means the slot is never left empty; it does not mean the entry
    ///   value survives.
    /// - **A deliberate `remove_resource::<R>()` inside `f` is undone.** An empty slot at exit is
    ///   indistinguishable from "nothing replaced it", so the value taken at entry goes back in.
    ///   Remove `R` from outside this helper if you need it to stay removed.
    pub fn with_resource_mut<R, F>(&mut self, f: F) -> bool
    where
        R: 'static,
        F: FnOnce(&mut R, &mut World),
    {
        let Some(mut r) = self.remove_resource::<R>() else {
            return false;
        };
        // `r` lives OUTSIDE the World for the duration of `f`, so an unwind through `f` would
        // drop it and delete session state for good. That is not hypothetical: `App`'s default
        // `SystemPanicPolicy::DisableSystemAndContinue` catches a system panic and keeps the frame
        // loop running, so the game would carry on for the rest of the session missing e.g.
        // `PhysicsWorld` — with only the panicking system's name in the log and nothing anywhere
        // naming the resource that vanished. Catch, restore, then re-raise the payload unchanged.
        //
        // Under `panic = "abort"` (the shipping profile) there is no unwind to catch and the
        // process is already gone, so there is nothing left to lose either way.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut r, self)));
        // Only put `r` back if `f` did not install a replacement. Inserting a fresh `R` from
        // inside the closure is exactly what the `&mut World` argument is for — a config or
        // registry hot-reload — and the unconditional re-insert this used to do overwrote the new
        // value with the stale one, silently turning the reload into a no-op.
        if self.resource::<R>().is_none() {
            self.insert_resource(r);
        }
        match outcome {
            Ok(()) => true,
            Err(payload) => std::panic::resume_unwind(payload),
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
