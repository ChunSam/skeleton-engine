use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

// The `impl World` surface is split across themed submodules; this file holds the
// data model (Entity / Archetype / World structs + type aliases) and the private
// archetype plumbing they all share. Each submodule re-opens `impl World` for one
// concern and reaches the private fields/helpers below as a descendant of `world`.
mod change_tracking;
mod clone;
mod components;
mod entities;
mod queries;
mod reflect;
mod resources;

// Parallel queries are native-only (WASM is single-threaded), so the whole module
// is gated out on wasm.
#[cfg(not(target_arch = "wasm32"))]
mod parallel;

/// Generation-checked ECS handle.
///
/// `index` identifies the storage slot and `generation` changes every time that slot is
/// despawned. A stale handle from an older generation does not match the reused entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    /// Storage slot index. Useful for debug labels, deterministic seeds, and migration logs.
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Generation number for this slot. A reused slot receives a higher generation.
    pub const fn generation(self) -> u32 {
        self.generation
    }

    /// Construct an entity handle from raw parts.
    ///
    /// This is primarily for integration boundaries such as scripting. It does not make the
    /// handle alive unless a `World` currently contains the same index and generation.
    pub const fn from_raw_parts(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
}

/// Storage unit for components. Requires `Send + Sync` to allow parallel queries.
type ComponentBox = Box<dyn Any + Send + Sync>;
/// `Arc`, not `Box`: `clone_component_by_typeid` hands itself a second handle rather than
/// removing the entry from the registry for the duration of the call. See there for why.
type CloneComponentFn = Arc<dyn Fn(&mut World, Entity, Entity) + Send + Sync>;

// ─── Reflect registry helpers ────────────────────────────────────────────────

fn get_reflect_impl<T: crate::reflect::Reflect + 'static>(
    b: &ComponentBox,
) -> Option<&dyn crate::reflect::Reflect> {
    b.downcast_ref::<T>()
        .map(|t| t as &dyn crate::reflect::Reflect)
}

fn get_reflect_mut_impl<T: crate::reflect::Reflect + 'static>(
    b: &mut ComponentBox,
) -> Option<&mut dyn crate::reflect::Reflect> {
    b.downcast_mut::<T>()
        .map(|t| t as &mut dyn crate::reflect::Reflect)
}

#[derive(Copy, Clone)]
struct ReflectEntry {
    get: fn(&ComponentBox) -> Option<&dyn crate::reflect::Reflect>,
    get_mut: fn(&mut ComponentBox) -> Option<&mut dyn crate::reflect::Reflect>,
    type_name: &'static str,
}

type ArchetypeId = usize;

/// A group of entities that share the same component set.
/// columns[T][i] is the T component of entities[i] (always the same length).
struct Archetype {
    type_set: Vec<TypeId>, // sorted TypeId list
    entities: Vec<Entity>,
    columns: HashMap<TypeId, Vec<ComponentBox>>,
}

impl Archetype {
    fn new(type_set: Vec<TypeId>) -> Self {
        let columns = type_set
            .iter()
            .map(|&t| (t, Vec::<ComponentBox>::new()))
            .collect();
        Self {
            type_set,
            entities: Vec::new(),
            columns,
        }
    }

    fn contains(&self, tid: TypeId) -> bool {
        self.type_set.binary_search(&tid).is_ok()
    }

    /// Debug-only invariant: every column holds exactly as many values as `entities`.
    ///
    /// Every iteration site in `queries.rs` and `parallel.rs` zips `entities` against the columns,
    /// and **zip yields the shorter of the two** — so a desync would silently drop entities from a
    /// query rather than fail. Four of those sites indexed instead until v0.152.6, and indexing
    /// panicked on exactly this; the check restores that loudness without the per-entity bounds
    /// checks. Compiled out entirely in release.
    #[inline]
    fn debug_assert_columns_aligned(&self) {
        #[cfg(debug_assertions)]
        for (tid, col) in &self.columns {
            debug_assert_eq!(
                col.len(),
                self.entities.len(),
                "archetype column {tid:?} desynced from entities — zip iteration would truncate"
            );
        }
    }
}

/// Central ECS storage.
///
/// - Entity: stale-safe handle with an index and a generation
/// - Components: dense column storage backed by Archetypes
/// - Resources: global singleton data
pub struct World {
    next_index: u32,
    free_indices: VecDeque<u32>,
    generations: Vec<u32>,
    entities: Vec<Entity>,
    /// Tracks each entity's index into `entities` for O(1) removal in `despawn`.
    entities_row: HashMap<Entity, usize>,
    archetypes: Vec<Archetype>,
    archetype_index: HashMap<Vec<TypeId>, ArchetypeId>,
    entity_location: HashMap<Entity, (ArchetypeId, usize)>,
    resources: HashMap<TypeId, Box<dyn Any>>,
    reflect_registry: HashMap<TypeId, ReflectEntry>,
    /// Change-tracking: component types first added this tick, keyed by entity.
    /// `HashMap<Entity, HashSet<TypeId>>` gives O(1) removal per entity at despawn
    /// (vs. the previous `HashSet<(Entity, TypeId)>` which required an O(N) retain).
    added_this_tick: HashMap<Entity, HashSet<TypeId>>,
    /// Change-tracking: component types replaced/mutated this tick, keyed by entity.
    changed_this_tick: HashMap<Entity, HashSet<TypeId>>,
    /// Function registry used to clone components inside `clone_entity`.
    clone_registry: HashMap<TypeId, CloneComponentFn>,
    /// Scratch for `move_entity`'s in-flight components. A per-call `HashMap` allocated on every
    /// archetype transition *and* grew with the entity's width; this is `clear()`ed and reused.
    move_scratch: Vec<(TypeId, ComponentBox)>,
    /// Scratch for the archetype signature `add_component` / `remove_component` build to look up
    /// their destination. On the common path that archetype already exists, so the `Vec` was
    /// allocated only to be dropped one line later.
    sig_scratch: Vec<TypeId>,
}

impl World {
    pub fn new() -> Self {
        let empty_arch = Archetype::new(vec![]);
        let mut archetype_index = HashMap::new();
        archetype_index.insert(vec![], 0);
        Self {
            next_index: 0,
            free_indices: VecDeque::new(),
            generations: Vec::new(),
            entities: Vec::new(),
            entities_row: HashMap::new(),
            archetypes: vec![empty_arch],
            archetype_index,
            entity_location: HashMap::new(),
            resources: HashMap::new(),
            reflect_registry: HashMap::new(),
            added_this_tick: HashMap::new(),
            changed_this_tick: HashMap::new(),
            clone_registry: HashMap::new(),
            move_scratch: Vec::new(),
            sig_scratch: Vec::new(),
        }
    }

    /// Returns true if the entity has a component with the given TypeId.
    pub(crate) fn has_component_typeid(&self, entity: Entity, tid: TypeId) -> bool {
        match self.entity_location.get(&entity) {
            Some(&(arch_id, _)) => self.archetypes[arch_id].contains(tid),
            None => false,
        }
    }

    /// Clones a single component by TypeId.
    ///
    /// The closure needs `&mut World` while it runs, which is why it cannot simply be borrowed
    /// out of `self.clone_registry`. This used to take it out of the map and put it back
    /// afterwards — a pattern that silently **unregisters the type for the rest of the session**
    /// if the clone panics, since the re-insert never runs. `T::clone` is user code in a forked
    /// engine, so that is reachable. Cloning the `Arc` costs one refcount bump, removes the map
    /// churn, and leaves the registry untouched no matter how the call ends.
    fn clone_component_by_typeid(&mut self, src: Entity, dst: Entity, tid: TypeId) {
        if let Some(clone_fn) = self.clone_registry.get(&tid).cloned() {
            clone_fn(self, src, dst);
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    /// Takes a slice, not an owned `Vec`: on the common path the archetype already exists, and an
    /// owned signature would be allocated by the caller only to be dropped here one line later.
    /// `Vec<T>: Borrow<[T]>`, so the map lookup works on the borrowed form unchanged.
    fn get_or_create_archetype(&mut self, sig: &[TypeId]) -> ArchetypeId {
        if let Some(&id) = self.archetype_index.get(sig) {
            return id;
        }
        let id = self.archetypes.len();
        self.archetypes.push(Archetype::new(sig.to_vec()));
        self.archetype_index.insert(sig.to_vec(), id);
        id
    }

    /// Moves an entity to target_arch_id, transferring shared components.
    /// The caller is responsible for pushing any newly added components.
    fn move_entity(&mut self, entity: Entity, target_arch_id: ArchetypeId) {
        let (src_arch_id, src_row) = self.entity_location[&entity];
        if src_arch_id == target_arch_id {
            return;
        }

        let src_len = self.archetypes[src_arch_id].entities.len();
        // `type_set` and `columns` are fields of the same Archetype, so a shared borrow of the
        // first cannot coexist with the `&mut` the loop needs on the second — and `split_at_mut`
        // does not help, since both borrows land on the same `Vec` element. This used to break
        // that by *cloning* `type_set`, once for the source and once for the destination, on
        // every transition: assembling an N-component entity re-copied the whole signature N
        // times, so the per-component allocation cost climbed with the entity's width.
        //
        // `mem::take` breaks the same borrow for nothing. Between the take and the restore the
        // archetype's `type_set` reads empty, which is safe here because the loop reaches only
        // `columns` and nothing in this function calls out to code that could observe an
        // Archetype. (Unlike the `with_resource_mut` case in v0.152.2, there is no user closure
        // in the gap — the only way out is an `expect` firing, and that already means the
        // archetype invariant this restores is broken.)
        let src_type_set = std::mem::take(&mut self.archetypes[src_arch_id].type_set);

        // Reused scratch rather than a fresh `HashMap` per transition. That map held one entry
        // per component the entity already carried, so it allocated every time and grew with the
        // width — the second half of the same O(N^2) shape. A `Vec` also beats a `HashMap` for
        // the handful of entries involved: the destination loop below scans it linearly.
        let mut extracted = std::mem::take(&mut self.move_scratch);
        debug_assert!(
            extracted.is_empty(),
            "move_entity: scratch left dirty by a previous call"
        );
        for &tid in &src_type_set {
            let comp = self.archetypes[src_arch_id]
                .columns
                .get_mut(&tid)
                .expect("move_entity: source column exists for every type in its type_set")
                .swap_remove(src_row);
            extracted.push((tid, comp));
        }
        self.archetypes[src_arch_id].type_set = src_type_set;

        self.archetypes[src_arch_id].entities.swap_remove(src_row);

        if src_row < src_len - 1 {
            let swapped = self.archetypes[src_arch_id].entities[src_row];
            self.entity_location.insert(swapped, (src_arch_id, src_row));
        }

        let dst_row = self.archetypes[target_arch_id].entities.len();
        self.archetypes[target_arch_id].entities.push(entity);

        // Same borrow, same fix as the source side above.
        let dst_type_set = std::mem::take(&mut self.archetypes[target_arch_id].type_set);
        for &tid in &dst_type_set {
            if let Some(pos) = extracted.iter().position(|(t, _)| *t == tid) {
                let (_, comp) = extracted.swap_remove(pos);
                self.archetypes[target_arch_id]
                    .columns
                    .get_mut(&tid)
                    .expect("move_entity: target column exists for every type in its type_set")
                    .push(comp);
            }
        }
        self.archetypes[target_arch_id].type_set = dst_type_set;

        // Whatever is still in `extracted` belongs to a type the destination does not carry —
        // a genuine removal. `clear()` drops those boxes (the old `HashMap` did it by going out
        // of scope) and keeps the capacity for the next transition.
        extracted.clear();
        self.move_scratch = extracted;

        self.entity_location
            .insert(entity, (target_arch_id, dst_row));
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
