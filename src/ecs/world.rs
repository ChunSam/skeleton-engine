use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet, VecDeque};

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
type CloneComponentFn = Box<dyn Fn(&mut World, Entity, Entity) + Send + Sync>;

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
        }
    }

    /// Returns the number of currently alive entities.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Spawns an empty entity and returns it.
    pub fn spawn(&mut self) -> Entity {
        let index = if let Some(reused) = self.free_indices.pop_front() {
            reused
        } else {
            let index = self.next_index;
            self.next_index += 1;
            self.generations.push(0);
            index
        };
        let generation = self.generations[index as usize];
        let entity = Entity::from_raw_parts(index, generation);
        let row = self.archetypes[0].entities.len();
        self.archetypes[0].entities.push(entity);
        self.entity_location.insert(entity, (0, row));
        let entities_pos = self.entities.len();
        self.entities.push(entity);
        self.entities_row.insert(entity, entities_pos);
        entity
    }

    /// Removes an entity and releases all of its components. Idempotent.
    pub fn despawn(&mut self, entity: Entity) {
        let (arch_id, row) = match self.entity_location.get(&entity) {
            Some(&loc) => loc,
            None => return,
        };

        let arch_len = self.archetypes[arch_id].entities.len();
        let type_set: Vec<TypeId> = self.archetypes[arch_id].type_set.clone();

        for &tid in &type_set {
            self.archetypes[arch_id]
                .columns
                .get_mut(&tid)
                .unwrap()
                .swap_remove(row);
        }
        self.archetypes[arch_id].entities.swap_remove(row);

        if row < arch_len - 1 {
            let swapped = self.archetypes[arch_id].entities[row];
            self.entity_location.insert(swapped, (arch_id, row));
        }

        // O(1) removal: look up the entity's position in the flat `entities` list, then
        // swap_remove and update the row index for the entity that was swapped into that slot.
        if let Some(pos) = self.entities_row.remove(&entity) {
            let last = self.entities.len() - 1;
            self.entities.swap_remove(pos);
            if pos < last {
                // The entity now at `pos` was previously at `last`; update its tracked row.
                self.entities_row.insert(self.entities[pos], pos);
            }
        }

        self.entity_location.remove(&entity);
        if let Some(generation) = self.generations.get_mut(entity.index as usize) {
            if *generation != u32::MAX {
                *generation += 1;
                self.free_indices.push_back(entity.index);
            }
        }
        self.added_this_tick.remove(&entity);
        self.changed_this_tick.remove(&entity);
    }

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
    pub fn take_component<T: Send + Sync + 'static>(&mut self, entity: Entity) -> Option<T> {
        let tid = TypeId::of::<T>();
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
            self.archetypes[a].columns.get_mut(&tid).unwrap()[row] = Box::new(component);
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
            .unwrap()
            .push(Box::new(component));
        self.added_this_tick.entry(entity).or_default().insert(tid);
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

    /// Iterates over all (Entity, &T) pairs that have component T.
    pub fn query<T: 'static>(&self) -> impl Iterator<Item = (Entity, &T)> {
        let tid = TypeId::of::<T>();
        self.archetypes
            .iter()
            .filter(move |arch| arch.contains(tid))
            .flat_map(move |arch| {
                let col = arch.columns.get(&tid).unwrap();
                arch.entities
                    .iter()
                    .zip(col.iter())
                    .map(|(&e, c)| (e, c.downcast_ref::<T>().unwrap()))
            })
    }

    /// Iterates over all `(Entity, &mut T)` pairs with **mutable** references.
    ///
    /// Mutable variant of `query::<T>()`. Avoids the two-pass workaround
    /// ("collect entities then `get_mut`", which allocates every frame) when you
    /// want to mutate a single component across many entities.
    ///
    /// As with `get_mut`, changes made here are NOT automatically recorded in
    /// `query_changed<T>()`. Call [`World::mark_changed`] if change detection is needed.
    pub fn query_mut<T: 'static>(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        let tid = TypeId::of::<T>();
        self.archetypes
            .iter_mut()
            .filter(move |arch| arch.contains(tid))
            .flat_map(move |arch| {
                // `entities` (immutable) and `columns` (mutable) are distinct fields of
                // Archetype, so destructuring lets us borrow both disjointly at once.
                let Archetype {
                    entities, columns, ..
                } = arch;
                let col = columns.get_mut(&tid).unwrap();
                entities
                    .iter()
                    .zip(col.iter_mut())
                    .map(|(&e, c)| (e, c.downcast_mut::<T>().unwrap()))
            })
    }

    /// Iterates over entities that have both A and B.
    pub fn query2<A: 'static, B: 'static>(&self) -> impl Iterator<Item = (Entity, &A, &B)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        self.archetypes
            .iter()
            .filter(move |arch| arch.contains(ta) && arch.contains(tb))
            .flat_map(move |arch| {
                let ca = arch.columns.get(&ta).unwrap();
                let cb = arch.columns.get(&tb).unwrap();
                arch.entities.iter().enumerate().map(move |(i, &e)| {
                    (
                        e,
                        ca[i].downcast_ref::<A>().unwrap(),
                        cb[i].downcast_ref::<B>().unwrap(),
                    )
                })
            })
    }

    /// Iterates over entities that have A, B, and C.
    pub fn query3<A: 'static, B: 'static, C: 'static>(
        &self,
    ) -> impl Iterator<Item = (Entity, &A, &B, &C)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        let tc = TypeId::of::<C>();
        self.archetypes
            .iter()
            .filter(move |arch| arch.contains(ta) && arch.contains(tb) && arch.contains(tc))
            .flat_map(move |arch| {
                let ca = arch.columns.get(&ta).unwrap();
                let cb = arch.columns.get(&tb).unwrap();
                let cc = arch.columns.get(&tc).unwrap();
                arch.entities.iter().enumerate().map(move |(i, &e)| {
                    (
                        e,
                        ca[i].downcast_ref::<A>().unwrap(),
                        cb[i].downcast_ref::<B>().unwrap(),
                        cc[i].downcast_ref::<C>().unwrap(),
                    )
                })
            })
    }

    /// Iterates over entities that have A, B, C, and D.
    pub fn query4<A: 'static, B: 'static, C: 'static, D: 'static>(
        &self,
    ) -> impl Iterator<Item = (Entity, &A, &B, &C, &D)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        let tc = TypeId::of::<C>();
        let td = TypeId::of::<D>();
        self.archetypes
            .iter()
            .filter(move |arch| {
                arch.contains(ta) && arch.contains(tb) && arch.contains(tc) && arch.contains(td)
            })
            .flat_map(move |arch| {
                let ca = arch.columns.get(&ta).unwrap();
                let cb = arch.columns.get(&tb).unwrap();
                let cc = arch.columns.get(&tc).unwrap();
                let cd = arch.columns.get(&td).unwrap();
                arch.entities.iter().enumerate().map(move |(i, &e)| {
                    (
                        e,
                        ca[i].downcast_ref::<A>().unwrap(),
                        cb[i].downcast_ref::<B>().unwrap(),
                        cc[i].downcast_ref::<C>().unwrap(),
                        cd[i].downcast_ref::<D>().unwrap(),
                    )
                })
            })
    }

    /// Iterates over entities that have A and also have B.
    pub fn query_with<A: 'static, B: 'static>(&self) -> impl Iterator<Item = (Entity, &A)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        self.archetypes
            .iter()
            .filter(move |arch| arch.contains(ta) && arch.contains(tb))
            .flat_map(move |arch| {
                let col = arch.columns.get(&ta).unwrap();
                arch.entities
                    .iter()
                    .zip(col.iter())
                    .map(|(&e, c)| (e, c.downcast_ref::<A>().unwrap()))
            })
    }

    /// Iterates over entities that have A but do not have B.
    pub fn query_without<A: 'static, B: 'static>(&self) -> impl Iterator<Item = (Entity, &A)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        self.archetypes
            .iter()
            .filter(move |arch| arch.contains(ta) && !arch.contains(tb))
            .flat_map(move |arch| {
                let col = arch.columns.get(&ta).unwrap();
                arch.entities
                    .iter()
                    .zip(col.iter())
                    .map(|(&e, c)| (e, c.downcast_ref::<A>().unwrap()))
            })
    }

    /// Iterates over all entities with A. B is Some if present, None otherwise.
    pub fn query_opt2<A: 'static, B: 'static>(
        &self,
    ) -> impl Iterator<Item = (Entity, &A, Option<&B>)> {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        self.archetypes
            .iter()
            .filter(move |arch| arch.contains(ta))
            .flat_map(move |arch| {
                let ca = arch.columns.get(&ta).unwrap();
                let cb = arch.columns.get(&tb);
                arch.entities.iter().enumerate().map(move |(i, &e)| {
                    let a = ca[i].downcast_ref::<A>().unwrap();
                    let b = cb.map(|col| col[i].downcast_ref::<B>().unwrap());
                    (e, a, b)
                })
            })
    }

    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    // ── Resources ────────────────────────────────────────────────────────────

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

    // ── Reflect registry ──────────────────────────────────────────────────────

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

    /// Returns true if the entity is alive (false if despawned or never created).
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entity_location.contains_key(&entity)
    }

    /// Applies all buffered Commands to the World immediately.
    ///
    /// Must be called after system execution finishes (i.e. after all query iterators
    /// have been dropped) to safely mutate entities/components without borrow conflicts.
    ///
    /// ```rust,ignore
    /// fn run(&mut self, world: &mut World, _dt: f32) {
    ///     let mut cmds = Commands::new();
    ///     cmds.spawn(|world, e| { world.add_component(e, MyTag); });
    ///     world.apply_commands(cmds);
    /// }
    /// ```
    pub fn apply_commands(&mut self, commands: crate::ecs::Commands) {
        commands.apply(self);
    }

    // ── Change detection ──────────────────────────────────────────────────────

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
    pub fn query_added<T: 'static>(&self) -> impl Iterator<Item = (Entity, &T)> {
        let tid = TypeId::of::<T>();
        let entities: Vec<Entity> = self
            .added_this_tick
            .iter()
            .filter(|(_, tids)| tids.contains(&tid))
            .map(|(e, _)| *e)
            .collect();
        entities
            .into_iter()
            .filter_map(move |e| self.get::<T>(e).map(|c| (e, c)))
    }

    /// Returns only entities whose component T was *replaced* this tick.
    ///
    /// **Note:** allocates a `Vec<Entity>` on every call to collect the matching set before
    /// returning the iterator. Intended for low-frequency use (e.g. reactive UI updates);
    /// avoid calling in hot per-frame loops with many entities.
    pub fn query_changed<T: 'static>(&self) -> impl Iterator<Item = (Entity, &T)> {
        let tid = TypeId::of::<T>();
        let entities: Vec<Entity> = self
            .changed_this_tick
            .iter()
            .filter(|(_, tids)| tids.contains(&tid))
            .map(|(e, _)| *e)
            .collect();
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

    // ── Entity cloning ────────────────────────────────────────────────────────

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

    /// Returns true if the entity has a component with the given TypeId.
    pub(crate) fn has_component_typeid(&self, entity: Entity, tid: TypeId) -> bool {
        match self.entity_location.get(&entity) {
            Some(&(arch_id, _)) => self.archetypes[arch_id].contains(tid),
            None => false,
        }
    }

    /// Clones a single component by TypeId using the remove → call clone_fn → reinsert pattern.
    fn clone_component_by_typeid(&mut self, src: Entity, dst: Entity, tid: TypeId) {
        if let Some(clone_fn) = self.clone_registry.remove(&tid) {
            clone_fn(self, src, dst);
            self.clone_registry.insert(tid, clone_fn);
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn get_or_create_archetype(&mut self, sig: Vec<TypeId>) -> ArchetypeId {
        if let Some(&id) = self.archetype_index.get(&sig) {
            return id;
        }
        let id = self.archetypes.len();
        self.archetypes.push(Archetype::new(sig.clone()));
        self.archetype_index.insert(sig, id);
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
        // Clone required: we need `type_set` keys to call `columns.get_mut()` on the same
        // Archetype. Holding `&type_set` while also holding `&mut columns` would alias fields
        // of the same struct, which the borrow checker disallows. split_at_mut cannot help
        // here because both borrows are on the same Vec element.
        let src_type_set: Vec<TypeId> = self.archetypes[src_arch_id].type_set.clone();

        let mut extracted: HashMap<TypeId, ComponentBox> = HashMap::new();
        for &tid in &src_type_set {
            let comp = self.archetypes[src_arch_id]
                .columns
                .get_mut(&tid)
                .unwrap()
                .swap_remove(src_row);
            extracted.insert(tid, comp);
        }

        self.archetypes[src_arch_id].entities.swap_remove(src_row);

        if src_row < src_len - 1 {
            let swapped = self.archetypes[src_arch_id].entities[src_row];
            self.entity_location.insert(swapped, (src_arch_id, src_row));
        }

        let dst_row = self.archetypes[target_arch_id].entities.len();
        self.archetypes[target_arch_id].entities.push(entity);

        // Same borrow-checker constraint as src_type_set above: `type_set` and `columns`
        // are fields of the same Archetype, so a shared ref to `type_set` cannot coexist
        // with a mutable borrow of `columns`.
        let dst_type_set: Vec<TypeId> = self.archetypes[target_arch_id].type_set.clone();
        for &tid in &dst_type_set {
            if let Some(comp) = extracted.remove(&tid) {
                self.archetypes[target_arch_id]
                    .columns
                    .get_mut(&tid)
                    .unwrap()
                    .push(comp);
            }
        }

        self.entity_location
            .insert(entity, (target_arch_id, dst_row));
    }
}

// ─── Parallel queries (native only — WASM is single-threaded) ────────────────

#[cfg(not(target_arch = "wasm32"))]
impl World {
    /// Applies a closure **in parallel** to all entities with T (read-only).
    ///
    /// To collect results, use a `Mutex` or channel inside the closure,
    /// or use `par_query_map` if you need return values.
    ///
    /// ```text
    /// world.par_query_for_each::<Transform, _>(|e, t| {
    ///     println!("{e:?} pos={}", t.position);
    /// });
    /// ```
    pub fn par_query_for_each<T, F>(&self, f: F)
    where
        T: Send + Sync + 'static,
        F: Fn(Entity, &T) + Send + Sync,
    {
        use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
        let tid = TypeId::of::<T>();
        self.archetypes
            .par_iter()
            .filter(|arch| arch.contains(tid))
            .for_each(|arch| {
                let col = arch.columns.get(&tid).unwrap();
                arch.entities
                    .par_iter()
                    .zip(col.par_iter())
                    .for_each(|(&e, c)| f(e, c.downcast_ref::<T>().unwrap()));
            });
    }

    /// Applies a mapping closure **in parallel** to all entities with T and returns the results as `Vec<R>`.
    ///
    /// ```text
    /// let positions: Vec<(Entity, Vec2)> =
    ///     world.par_query_map::<Transform, _, _>(|e, t| (e, t.position));
    /// ```
    pub fn par_query_map<T, R, F>(&self, f: F) -> Vec<R>
    where
        T: Send + Sync + 'static,
        R: Send,
        F: Fn(Entity, &T) -> R + Send + Sync,
    {
        use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
        let tid = TypeId::of::<T>();
        self.archetypes
            .par_iter()
            .filter(|arch| arch.contains(tid))
            .flat_map(|arch| {
                let col = arch.columns.get(&tid).unwrap();
                arch.entities
                    .par_iter()
                    .zip(col.par_iter())
                    .map(|(&e, c)| f(e, c.downcast_ref::<T>().unwrap()))
            })
            .collect()
    }

    /// Applies a closure **in parallel** to all entities with both A and B (read-only).
    pub fn par_query2_for_each<A, B, F>(&self, f: F)
    where
        A: Send + Sync + 'static,
        B: Send + Sync + 'static,
        F: Fn(Entity, &A, &B) + Send + Sync,
    {
        use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        self.archetypes
            .par_iter()
            .filter(move |arch| arch.contains(ta) && arch.contains(tb))
            .for_each(|arch| {
                let ca = arch.columns.get(&ta).unwrap();
                let cb = arch.columns.get(&tb).unwrap();
                arch.entities
                    .par_iter()
                    .zip(ca.par_iter())
                    .zip(cb.par_iter())
                    .for_each(|((&e, a), b)| {
                        f(
                            e,
                            a.downcast_ref::<A>().unwrap(),
                            b.downcast_ref::<B>().unwrap(),
                        );
                    });
            });
    }

    /// Applies a mapping closure **in parallel** to all entities with both A and B and returns the results as `Vec<R>`.
    pub fn par_query2_map<A, B, R, F>(&self, f: F) -> Vec<R>
    where
        A: Send + Sync + 'static,
        B: Send + Sync + 'static,
        R: Send,
        F: Fn(Entity, &A, &B) -> R + Send + Sync,
    {
        use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        self.archetypes
            .par_iter()
            .filter(move |arch| arch.contains(ta) && arch.contains(tb))
            .flat_map(|arch| {
                let ca = arch.columns.get(&ta).unwrap();
                let cb = arch.columns.get(&tb).unwrap();
                arch.entities
                    .par_iter()
                    .zip(ca.par_iter())
                    .zip(cb.par_iter())
                    .map(|((&e, a), b)| {
                        f(
                            e,
                            a.downcast_ref::<A>().unwrap(),
                            b.downcast_ref::<B>().unwrap(),
                        )
                    })
            })
            .collect()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
