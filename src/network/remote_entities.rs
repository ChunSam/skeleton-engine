use crate::ecs::world::{Entity, World};

/// Tracks server-owned "remote" entities by a network id, handling the spawn-on-first-sight /
/// despawn-on-removal lifecycle that every networked game otherwise reimplements inline (a
/// `HashMap<id, Entity>` plus get-or-spawn and remove-and-despawn).
///
/// It owns only the `id → Entity` mapping and the spawn/despawn lifecycle. Deciding *what* to spawn
/// (the `spawn` closure), *how* to update an existing entity (call [`get`](Self::get), then mutate
/// it through the `World`), and any parallel game-state maps stay in the game — keeping this a thin,
/// genre-agnostic slice.
///
/// ```
/// # use engine::{RemoteEntities, World};
/// let mut world = World::new();
/// let mut remotes: RemoteEntities<u32> = RemoteEntities::new();
/// // On the first update for network id 7, spawn + insert; later updates reuse the entity.
/// let e = remotes.get_or_spawn(&mut world, 7, |w| w.spawn());
/// let again = remotes.get_or_spawn(&mut world, 7, |w| w.spawn());
/// assert_eq!(again, e);
/// assert_eq!(remotes.get(&7), Some(e));
/// assert_eq!(remotes.len(), 1);
/// // On a "bye" for id 7, remove + despawn.
/// remotes.remove(&mut world, &7);
/// assert!(remotes.get(&7).is_none());
/// ```
///
/// # Deliberately minimal — future deep-dive
///
/// This is intentionally just the lifecycle map. A *richer* version (state interpolation,
/// client-side prediction/reconciliation, per-entity update callbacks, staleness/generation
/// handling) is deferred: the two shipping call sites (`mp_client`, `coin_race`) are structurally
/// similar (JSON relay, `HashMap<usize, Entity>`, spawn-square-on-update), so they don't yet reveal
/// the right shape for those features, and a wrong public (semver-bound) API is worse than the small
/// duplication it removes. Revisit once a *third, distinct* networked example exists — see
/// `docs/REMOTE_ENTITIES_DESIGN.md`.
pub struct RemoteEntities<K: Eq + std::hash::Hash> {
    map: std::collections::HashMap<K, Entity>,
}

impl<K: Eq + std::hash::Hash> Default for RemoteEntities<K> {
    fn default() -> Self {
        Self {
            map: std::collections::HashMap::new(),
        }
    }
}

impl<K: Eq + std::hash::Hash> RemoteEntities<K> {
    /// Creates an empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the entity mapped to `key`, spawning and inserting one via `spawn` on first sight.
    ///
    /// If the cached entity has been despawned externally (e.g. after a scene reset), a new entity
    /// is spawned and the map entry is replaced. Call [`clear`](Self::clear) on scene reset to
    /// avoid stale entries accumulating; this liveness check is a safety net, not a substitute.
    pub fn get_or_spawn(
        &mut self,
        world: &mut World,
        key: K,
        spawn: impl FnOnce(&mut World) -> Entity,
    ) -> Entity {
        if let Some(&cached) = self.map.get(&key) {
            if world.is_alive(cached) {
                return cached;
            }
            // Stale entry — the entity was despawned externally; fall through to re-spawn.
        }
        let entity = spawn(world);
        self.map.insert(key, entity);
        entity
    }

    /// The entity currently mapped to `key`, if any.
    pub fn get(&self, key: &K) -> Option<Entity> {
        self.map.get(key).copied()
    }

    /// Whether `key` is currently tracked.
    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// Removes `key` and despawns its entity. No-op if `key` is absent.
    pub fn remove(&mut self, world: &mut World, key: &K) {
        if let Some(entity) = self.map.remove(key) {
            world.despawn(entity);
        }
    }

    /// Despawns every tracked entity and clears the map (e.g. on disconnect or scene reset).
    pub fn clear(&mut self, world: &mut World) {
        for (_, entity) in self.map.drain() {
            world.despawn(entity);
        }
    }

    /// Number of tracked entities.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether no entities are tracked.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Iterates `(&key, entity)` pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, Entity)> {
        self.map.iter().map(|(k, &e)| (k, e))
    }
}
