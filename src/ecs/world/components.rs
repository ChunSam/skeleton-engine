//! Per-entity component access: add / remove / take / get / has.

use super::{ComponentBox, Entity, World};
use std::any::TypeId;

impl World {
    /// Removes component T from an entity. Does not panic if the component is absent.
    pub fn remove_component<T: 'static>(&mut self, entity: Entity) {
        let tid = TypeId::of::<T>();
        let (arch_id, _) = match self.entity_location.get(&entity) {
            Some(&loc) => loc,
            None => return,
        };

        if !self.archetypes[arch_id].contains(tid) {
            return;
        }

        let new_sig: Vec<TypeId> = self.archetypes[arch_id]
            .type_set
            .iter()
            .copied()
            .filter(|&t| t != tid)
            .collect();

        let new_arch_id = self.get_or_create_archetype(new_sig);
        self.move_entity(entity, new_arch_id);
        if let Some(s) = self.added_this_tick.get_mut(&entity) {
            s.remove(&tid);
            if s.is_empty() {
                self.added_this_tick.remove(&entity);
            }
        }
        if let Some(s) = self.changed_this_tick.get_mut(&entity) {
            s.remove(&tid);
            if s.is_empty() {
                self.changed_this_tick.remove(&entity);
            }
        }
    }

    /// Removes component T from an entity and returns its value. Returns None if absent.
    ///
    /// Unlike `remove_component`, this transfers ownership of the component value.
    /// Use it when code like `BehaviorSystem` needs to borrow a component temporarily
    /// while still having mutable access to `World`.
    ///
    /// Putting the value back with [`add_component`](World::add_component) in the same tick is
    /// reported as a **change**, not an addition — see the tracking note on the re-add branch of
    /// `add_component`.
    pub fn take_component<T: Send + Sync + 'static>(&mut self, entity: Entity) -> Option<T> {
        let tid = TypeId::of::<T>();
        // `take_component` is one half of the take → mutate → put-back idiom that
        // `BehaviorSystem` and `TimelineSystem` run every frame, so note which change-tracking
        // bucket the component sits in *before* the `remove_component` below clears both of
        // them. `add_component` reads it back to classify the put-back correctly.
        let was_added_this_tick = self
            .added_this_tick
            .get(&entity)
            .is_some_and(|tids| tids.contains(&tid));
        // Step 1: swap the real value out with a Box<()> placeholder to gain ownership
        let value: T = {
            let (arch_id, row) = *self.entity_location.get(&entity)?;
            let arch = &mut self.archetypes[arch_id];
            if !arch.contains(tid) {
                return None;
            }
            let col = arch.columns.get_mut(&tid)?;
            // swap the real value for a unit placeholder
            let placeholder: ComponentBox = Box::new(());
            let extracted = std::mem::replace(&mut col[row], placeholder);
            // Box<dyn Any+Send+Sync> → Box<T> → T
            *extracted.downcast::<T>().ok()?
        }; // archetypes borrow released
           // Step 2: remove the slot (which now holds the placeholder) from the archetype
        self.remove_component::<T>(entity);
        // Step 3: re-arm the bucket `remove_component` just cleared. A component that was
        // already on the entity when the tick began comes back as *changed*; one that was
        // genuinely new this tick stays *added* across the round trip. Costs the same single
        // `HashSet` insert the `add_component` put-back used to pay, so per-frame allocation
        // is unchanged (`tests/per_frame_alloc.rs`).
        let bucket = if was_added_this_tick {
            &mut self.added_this_tick
        } else {
            &mut self.changed_this_tick
        };
        bucket.entry(entity).or_default().insert(tid);
        Some(value)
    }

    /// Attaches a component to an entity. Replaces it if one already exists.
    ///
    /// `T: Send + Sync` is required to allow cross-thread sharing in parallel queries (`par_query*`).
    pub fn add_component<T: Send + Sync + 'static>(&mut self, entity: Entity, component: T) {
        let tid = TypeId::of::<T>();
        let (arch_id, _) = match self.entity_location.get(&entity) {
            Some(&loc) => loc,
            None => return,
        };

        if self.archetypes[arch_id].contains(tid) {
            let (a, row) = self.entity_location[&entity];
            self.archetypes[a]
                .columns
                .get_mut(&tid)
                .expect("add_component: archetype contains this column (checked above)")[row] =
                Box::new(component);
            self.changed_this_tick
                .entry(entity)
                .or_default()
                .insert(tid);
            return;
        }

        let new_sig: Vec<TypeId> = {
            let arch = &self.archetypes[arch_id];
            let mut sig = arch.type_set.clone();
            let pos = sig.binary_search(&tid).unwrap_err();
            sig.insert(pos, tid);
            sig
        };

        let new_arch_id = self.get_or_create_archetype(new_sig);
        self.move_entity(entity, new_arch_id);

        let (na, _) = self.entity_location[&entity];
        self.archetypes[na]
            .columns
            .get_mut(&tid)
            .expect("add_component: target archetype contains the newly added column")
            .push(Box::new(component));
        // A component put back by `take_component` this tick is a *change*, not an addition.
        // `take_component` leaves its TypeId in `changed_this_tick`, and the archetype move it
        // performs is exactly what routes the put-back through this branch instead of the
        // in-place one above. Without the check, the take → mutate → put-back idiom re-reported
        // every ticked entity as newly added on *every* frame — so a one-shot init keyed on
        // `query_added` ran forever — while `query_changed` never saw the write at all.
        let put_back_this_tick = self
            .changed_this_tick
            .get(&entity)
            .is_some_and(|tids| tids.contains(&tid));
        if !put_back_this_tick {
            self.added_this_tick.entry(entity).or_default().insert(tid);
        }
    }

    /// Returns an immutable reference to an entity's component.
    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        let &(arch_id, row) = self.entity_location.get(&entity)?;
        self.archetypes[arch_id]
            .columns
            .get(&TypeId::of::<T>())?
            .get(row)?
            .downcast_ref::<T>()
    }

    /// Returns a mutable reference to an entity's component.
    ///
    /// Directly modifying fields through this method is NOT automatically recorded
    /// in `query_changed<T>()`. Systems that need change detection should call
    /// [`World::mark_changed`] or use [`World::get_mut_tracked`].
    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        let &(arch_id, row) = self.entity_location.get(&entity)?;
        self.archetypes[arch_id]
            .columns
            .get_mut(&TypeId::of::<T>())?
            .get_mut(row)?
            .downcast_mut::<T>()
    }

    /// Returns `true` if the entity has component `T`.
    ///
    /// Zero-cost wrapper around `has_component_typeid` — no downcast or allocation.
    ///
    /// # Example
    /// ```rust,ignore
    /// if world.has_component::<Health>(player) {
    ///     // ...
    /// }
    /// ```
    pub fn has_component<T: 'static>(&self, entity: Entity) -> bool {
        self.has_component_typeid(entity, std::any::TypeId::of::<T>())
    }
}
