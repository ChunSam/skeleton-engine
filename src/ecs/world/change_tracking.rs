//! Per-tick change detection: `clear_change_tracking`, the `query_added` /
//! `query_changed` iterators, and the `mark_changed` / `get_mut_tracked` writers.

use super::{Entity, World};
use std::any::TypeId;

impl World {
    /// Resets change tracking for this tick. Called by `App` at the start of every frame.
    ///
    /// Empties the per-entity sets **in place** rather than dropping them. `HashMap::clear` drops
    /// the values, so every entity that changed last frame paid a fresh `HashSet` allocation to be
    /// recorded again this frame. That is the whole cost for a component rewritten every frame by
    /// construction: `HierarchySystem` writes `GlobalTransform` for every entity with a
    /// `Transform`, so before this, change tracking alone was 200 allocations / 44,212 bytes per
    /// frame for 200 entities (`tests/per_frame_alloc.rs`).
    ///
    /// Keeping the entries means an entity that stops changing keeps an empty set, so the map
    /// would grow to every entity that has ever changed. The empties are pruned once they
    /// outnumber the live ones — waste bounded at 2×, and a steady scene never prunes, which is
    /// what makes the steady state allocation-free. Same shape as `SpatialGrid`'s bucket reuse.
    pub fn clear_change_tracking(&mut self) {
        Self::clear_tick_map_reusing_capacity(&mut self.added_this_tick);
        Self::clear_tick_map_reusing_capacity(&mut self.changed_this_tick);
    }

    fn clear_tick_map_reusing_capacity(
        map: &mut std::collections::HashMap<Entity, std::collections::HashSet<TypeId>>,
    ) {
        // Decide before emptying: `HashSet::clear` keeps capacity, so afterwards there is no way
        // left to tell which entries had actually gone quiet.
        let live = map.values().filter(|set| !set.is_empty()).count();
        if map.len() > 2 * live.max(1) {
            map.retain(|_, set| !set.is_empty());
        }
        for set in map.values_mut() {
            set.clear();
        }
    }

    /// Returns only entities whose component T was *first added* this tick.
    ///
    /// **First** is literal: an entity that already carried `T` when the tick began never appears
    /// here, however the value is replaced mid-tick — in place, or via
    /// [`take_component`](World::take_component) and a fresh `add_component`. Both report through
    /// `query_changed`. Only [`remove_component`](World::remove_component) resets that, because it
    /// states the component is gone.
    ///
    /// **Note:** allocates a `Vec<Entity>` on every call to collect the matching set before
    /// returning the iterator. Intended for low-frequency use (e.g. one-shot init logic);
    /// avoid calling in hot per-frame loops with many entities.
    ///
    /// Returns an empty iterator immediately (no allocation) when no additions have
    /// been recorded this tick — the common case in most frames.
    ///
    /// Entities are yielded in ascending `(index, generation)` order, so repeated runs over
    /// identical input agree.
    pub fn query_added<T: 'static>(&self) -> impl Iterator<Item = (Entity, &T)> {
        let tid = TypeId::of::<T>();
        // Fast path: skip allocation entirely when the tracking set is empty.
        let mut entities: Vec<Entity> = if self.added_this_tick.is_empty() {
            Vec::new()
        } else {
            self.added_this_tick
                .iter()
                .filter(|(_, tids)| tids.contains(&tid))
                .map(|(e, _)| *e)
                .collect()
        };
        // `added_this_tick` is a `HashMap<Entity, _>`, so collecting from it hands back a
        // per-process-random order — these were the only public queries in the crate whose
        // iteration order was not reproducible across runs. Every other query walks
        // `self.archetypes` in Vec order, so a fork that assumes stable query order was right
        // everywhere except here, and a reactive system that spawns, plays a sound, or resolves
        // first-wins produced a different result on each launch from identical input.
        entities.sort_unstable_by_key(|e| (e.index(), e.generation()));
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
    ///
    /// Entities are yielded in ascending `(index, generation)` order, so repeated runs over
    /// identical input agree.
    pub fn query_changed<T: 'static>(&self) -> impl Iterator<Item = (Entity, &T)> {
        let tid = TypeId::of::<T>();
        // Fast path: skip allocation entirely when the tracking set is empty.
        let mut entities: Vec<Entity> = if self.changed_this_tick.is_empty() {
            Vec::new()
        } else {
            self.changed_this_tick
                .iter()
                .filter(|(_, tids)| tids.contains(&tid))
                .map(|(e, _)| *e)
                .collect()
        };
        // `changed_this_tick` is a `HashMap<Entity, _>`, so collecting from it hands back a
        // per-process-random order — these were the only public queries in the crate whose
        // iteration order was not reproducible across runs. Every other query walks
        // `self.archetypes` in Vec order, so a fork that assumes stable query order was right
        // everywhere except here, and a reactive system that spawns, plays a sound, or resolves
        // first-wins produced a different result on each launch from identical input.
        entities.sort_unstable_by_key(|e| (e.index(), e.generation()));
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
