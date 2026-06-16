//! Type-erased serde-component registry (`SerdeComponentRegistry` + `SerdeComponentEntry`).
//!
//! Factored out of `prefab` so it can be used as an independent serde concern without
//! pulling in `SceneDef`/`Prefab`/`EntityDef`. Re-exported from `prefab` to preserve
//! the original public path `engine::prefab::{SerdeComponentEntry, SerdeComponentRegistry}`.

use std::collections::HashMap;

use crate::ecs::{Entity, World};

// ─── SerdeComponentRegistry ───────────────────────────────────────────────────

/// A single registered serde-capable component type.
///
/// Holds type-erased serialize / deserialize / post-spawn closures so that any
/// `Serialize + DeserializeOwned + Clone` component can participate in scene
/// save/load without hardcoding each type in the engine core.
///
/// `post_spawn` uses `Arc` (instead of `Box`) so the closure can be cheaply cloned
/// into the `App::world_registrars` replay list that re-registers all serde components
/// after a scene Replace world reset.
#[allow(clippy::type_complexity)]
pub struct SerdeComponentEntry {
    /// Returns `true` iff `entity` has this component, without serializing any values.
    /// Used by [`SerdeComponentRegistry::component_names_for`] for the per-frame
    /// inspector component list, which only needs names (not values).
    pub has_component: Box<dyn Fn(&World, Entity) -> bool + Send + Sync>,
    /// Extracts the component from `entity` and serializes it to a RON [`ron::Value`].
    /// Returns `None` when the entity does not have this component.
    pub serialize: Box<dyn Fn(&World, Entity) -> Option<ron::Value> + Send + Sync>,
    /// Deserializes a RON [`ron::Value`] and inserts the component onto `entity`.
    pub deserialize:
        Box<dyn Fn(&mut World, Entity, ron::Value) -> Result<(), String> + Send + Sync>,
    /// Optional hook run after a successful deserialize (e.g. copy initial_text → text).
    /// Stored as `Arc` so it can be cloned into the scene-reset replay list without
    /// requiring the closure to be `Clone`.
    pub post_spawn: Option<std::sync::Arc<dyn Fn(&mut World, Entity) + Send + Sync>>,
}

/// Type-erased registry for serde-capable components.
///
/// Insert this as a resource via [`crate::app::App::register_serde_component`] or directly.
/// [`crate::prefab::spawn_entity_def`] calls [`SerdeComponentRegistry::deserialize_into`] when
/// `EntityDef.components` is non-empty. The editor save path calls
/// [`SerdeComponentRegistry::serialize_entity`] to populate `EntityDef.components`.
///
/// # Example
/// ```rust,no_run
/// use engine::prefab::{SerdeComponentRegistry, SerdeComponentEntry};
/// use engine::ecs::World;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Clone, Serialize, Deserialize)]
/// struct Health { max: f32 }
///
/// let mut registry = SerdeComponentRegistry::default();
/// registry.register::<Health>("Health", None);
/// ```
#[derive(Default)]
pub struct SerdeComponentRegistry {
    entries: HashMap<String, SerdeComponentEntry>,
}

impl SerdeComponentRegistry {
    /// Registers a serde-capable component type under `name`.
    ///
    /// `post_spawn` is an optional closure run after every successful deserialize
    /// (useful for copying design-time fields to runtime counterparts, e.g.
    /// `initial_text` → `text` for [`crate::ui::TextInput`]).
    #[allow(clippy::type_complexity)]
    pub fn register<T>(
        &mut self,
        name: impl Into<String>,
        post_spawn: Option<Box<dyn Fn(&mut World, Entity) + Send + Sync>>,
    ) where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + Sync + 'static,
    {
        // Convert Box → Arc so the closure can be cloned into the scene-reset replay list.
        self.register_arc::<T>(name, post_spawn.map(std::sync::Arc::from));
    }

    /// Internal registration path accepting an `Arc`-wrapped post-spawn hook.
    /// Used by `register` (converts from `Box`) and by the scene-reset replay thunks
    /// recorded in `App::world_registrars` (which clone the `Arc` cheaply).
    #[allow(clippy::type_complexity)]
    pub(crate) fn register_arc<T>(
        &mut self,
        name: impl Into<String>,
        post_spawn: Option<std::sync::Arc<dyn Fn(&mut World, Entity) + Send + Sync>>,
    ) where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + Sync + 'static,
    {
        let key = name.into();
        self.entries.insert(
            key,
            SerdeComponentEntry {
                // Cheap presence check: no serialization, just a component lookup.
                has_component: Box::new(|world, entity| world.get::<T>(entity).is_some()),
                serialize: Box::new(|world, entity| {
                    // Serialize `T` to a RON string and store it as ron::Value::String.
                    // We do not parse to ron::Value::Map because that path loses enum
                    // variant names (the Value representation has no enum concept).
                    world.get::<T>(entity).and_then(|c| {
                        match ron::to_string(c) {
                            Ok(s) => Some(ron::Value::String(s)),
                            Err(e) => {
                                log::warn!(
                                    "SerdeComponentRegistry: failed to serialize {}: {} — omitted from scene",
                                    std::any::type_name::<T>(),
                                    e
                                );
                                None
                            }
                        }
                    })
                }),
                deserialize: Box::new(|world, entity, val| {
                    // Extract the stored RON string and parse it as T.
                    match val {
                        ron::Value::String(s) => ron::from_str::<T>(&s)
                            .map(|c| world.add_component(entity, c))
                            .map_err(|e| e.to_string()),
                        other => Err(format!("expected ron::Value::String, got {:?}", other)),
                    }
                }),
                post_spawn,
            },
        );
    }

    /// Returns the names of all registered components present on `entity`, sorted
    /// alphabetically for stable ordering.
    ///
    /// Cheaper than [`Self::serialize_entity`] because it uses the `has_component` closure
    /// (a single `world.get::<T>()` check) and never converts values to RON. Use this
    /// when only the name list is needed (e.g., the per-frame inspector component list).
    pub fn component_names_for(&self, world: &World, entity: Entity) -> Vec<String> {
        let mut names: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| (entry.has_component)(world, entity))
            .map(|(name, _)| name.clone())
            .collect();
        names.sort();
        names
    }

    /// Serializes all registered components present on `entity`.
    ///
    /// Returns a map of `type_name → ron::Value`. Components not present on the
    /// entity are omitted.
    pub fn serialize_entity(&self, world: &World, entity: Entity) -> HashMap<String, ron::Value> {
        self.entries
            .iter()
            .filter_map(|(name, entry)| (entry.serialize)(world, entity).map(|v| (name.clone(), v)))
            .collect()
    }

    /// Deserializes `components` into `entity`.
    ///
    /// Unknown component names are logged and skipped. Deserialization errors are
    /// logged and skipped (load never fails due to a bad component value).
    /// After a successful deserialize, the `post_spawn` hook is called if present.
    pub fn deserialize_into(
        &self,
        world: &mut World,
        entity: Entity,
        components: &HashMap<String, ron::Value>,
    ) {
        for (name, val) in components {
            match self.entries.get(name) {
                None => {
                    log::warn!(
                        "SerdeComponentRegistry: unknown component type {:?} — skipping",
                        name
                    );
                }
                Some(entry) => match (entry.deserialize)(world, entity, val.clone()) {
                    Ok(()) => {
                        if let Some(hook) = &entry.post_spawn {
                            hook(world, entity);
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "SerdeComponentRegistry: failed to deserialize {:?}: {} — skipping",
                            name,
                            e
                        );
                    }
                },
            }
        }
    }
}
