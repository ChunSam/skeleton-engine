//! Per-tick change detection: `clear_change_tracking`, the `query_added` /
//! `query_changed` iterators, and the `mark_changed` / `get_mut_tracked` writers.

use super::{Entity, World};
use std::any::TypeId;

impl World {
    /// Resets change tracking for this tick. Called by `App` at the start of every frame.
    pub fn clear_change_tracking(&mut self) {
        self.added_this_tick.clear();
        self.changed_this_tick.clear();
    }

    /// Returns only entities whose component T was *first added* this tick.
    ///
    /// **Note:** allocates a `Vec<Entity>` on every call to collect the matching set before
    /// returning the iterator. Intended for low-frequency use (e.g. one-shot init logic);
    /// avoid calling in hot per-frame loops with many entities.
    ///
    /// Returns an empty iterator immediately (no allocation) when no additions have
    /// been recorded this tick — the common case in most frames.
    pub fn query_added<T: 'static>(&self) -> impl Iterator<Item = (Entity, &T)> {
        let tid = TypeId::of::<T>();
        // Fast path: skip allocation entirely when the tracking set is empty.
        let entities: Vec<Entity> = if self.added_this_tick.is_empty() {
            Vec::new()
        } else {
            self.added_this_tick
                .iter()
                .filter(|(_, tids)| tids.contains(&tid))
                .map(|(e, _)| *e)
                .collect()
        };
        entities
            .into_iter()
            .filter_map(move |e| self.get::<T>(e).map(|c| (e, c)))
    }

    /// Returns only entities whose component T was *replaced* this tick.
    ///
    /// **Note:** allocates a `Vec<Entity>` on every call to collect the matching set before
    /// returning the iterator. Intended for low-frequency use (e.g. reactive UI updates);
    /// avoid calling in hot per-frame loops with many entities.
    ///
    /// Returns an empty iterator immediately (no allocation) when no changes have
    /// been recorded this tick — the common case in most frames.
    pub fn query_changed<T: 'static>(&self) -> impl Iterator<Item = (Entity, &T)> {
        let tid = TypeId::of::<T>();
        // Fast path: skip allocation entirely when the tracking set is empty.
        let entities: Vec<Entity> = if self.changed_this_tick.is_empty() {
            Vec::new()
        } else {
            self.changed_this_tick
                .iter()
                .filter(|(_, tids)| tids.contains(&tid))
                .map(|(e, _)| *e)
                .collect()
        };
        entities
            .into_iter()
            .filter_map(move |e| self.get::<T>(e).map(|c| (e, c)))
    }

    /// Explicitly marks component T on an entity as changed this tick.
    ///
    /// Use this after directly mutating fields via `get_mut<T>()` when a reactive
    /// system needs to detect the change via `query_changed<T>()`.
    /// Returns `false` if the entity or its T component does not exist.
    pub fn mark_changed<T: 'static>(&mut self, entity: Entity) -> bool {
        let tid = TypeId::of::<T>();
        if !self.has_component_typeid(entity, tid) {
            return false;
        }
        self.changed_this_tick
            .entry(entity)
            .or_default()
            .insert(tid);
        true
    }

    /// Mutable component accessor that also records a change.
    ///
    /// The returned reference is recorded as changed even if you do not actually modify it.
    /// For conditional mutation, prefer `get_mut<T>()` and call [`World::mark_changed`]
    /// only when a change actually occurs.
    pub fn get_mut_tracked<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        if !self.mark_changed::<T>(entity) {
            return None;
        }
        self.get_mut::<T>(entity)
    }
}
