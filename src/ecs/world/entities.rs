//! Entity lifecycle: spawn / despawn / liveness, plus buffered-command application.

use super::{Entity, World};
use std::any::TypeId;

impl World {
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
                .expect("despawn: column exists for every type in the archetype's type_set")
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

    /// All live entities, in **storage order, not spawn order**.
    ///
    /// `despawn` fills the hole with `swap_remove`, so deleting one entity moves the
    /// last-spawned one into its slot: spawn five and despawn the second and this returns
    /// indices `[0, 4, 2, 3]`. The order is stable between mutations but is not recoverable
    /// spawn order — `Entity::index` is recycled too, so a caller cannot sort its way back to it.
    ///
    /// Anything that presents or persists this list should impose its own order — use
    /// [`entities_sorted`](Self::entities_sorted), which is that order.
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// All live entities in a **stable, presentable order**: [`entities`](Self::entities) sorted by
    /// [`Entity::index`].
    ///
    /// `Entity::index` is unique among live entities, so this is a total order. It is what any
    /// caller that *presents* or *persists* the entity list wants, and for the same reason in both
    /// cases: storage order reshuffles on `despawn`, so a hierarchy row jumps when an unrelated
    /// entity is deleted, and a one-entity change churns the whole RON file.
    ///
    /// ⚠️ **Not recovered spawn order, and it cannot be.** `Entity::index` is recycled, so a
    /// recycled slot sorts by where it sits now, not by when it was spawned. This is a *stable*
    /// order, not a *historical* one.
    ///
    /// Allocates — it is a sorted copy. Every caller was already doing `entities().to_vec()`
    /// before sorting, so this costs what the copy-paste it replaces cost; it exists because the
    /// policy was prescribed in prose here and re-implemented identically at three call sites,
    /// which is a rule the type system was not carrying.
    pub fn entities_sorted(&self) -> Vec<Entity> {
        let mut sorted = self.entities.to_vec();
        sorted.sort_unstable_by_key(|e| e.index());
        sorted
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
}
