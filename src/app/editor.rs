use super::*;

mod state;
mod ui;

pub(super) use state::EditorState;

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub(super) enum EditorCmd {
    MoveEntity {
        entity: Entity,
        old_pos: glam::Vec2,
        new_pos: glam::Vec2,
    },
    CreateEntity {
        entity: Entity,
    },
    DeleteEntity {
        /// Entity id recreated by undo. Filled in at undo time so redo can despawn exactly
        /// this entity (None on first creation — the original has already been despawned).
        entity: Option<Entity>,
        tag: Option<String>,
        transform: Option<crate::components::Transform>,
        sprite: Option<crate::components::Sprite>,
    },
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
pub(super) struct EditorHistory {
    undo: Vec<EditorCmd>,
    redo: Vec<EditorCmd>,
}

#[cfg(not(target_arch = "wasm32"))]
impl EditorHistory {
    pub(super) fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub(super) fn push(&mut self, cmd: EditorCmd) {
        self.undo.push(cmd);
        self.redo.clear();
    }

    pub(super) fn undo(&mut self, world: &mut World, selected: &mut Option<Entity>) {
        let Some(mut cmd) = self.undo.pop() else {
            return;
        };
        // entity id recreated by DeleteEntity undo — recorded into cmd after the match
        let mut respawned: Option<Entity> = None;
        match &cmd {
            EditorCmd::MoveEntity {
                entity, old_pos, ..
            } => {
                if let Some(t) = world.get_mut::<crate::components::Transform>(*entity) {
                    t.position = *old_pos;
                }
                *selected = Some(*entity);
            }
            EditorCmd::CreateEntity { entity } => {
                world.despawn(*entity);
                *selected = None;
            }
            EditorCmd::DeleteEntity {
                tag,
                transform,
                sprite,
                ..
            } => {
                let e = world.spawn();
                if let Some(tr) = transform {
                    world.add_component(e, tr.clone());
                }
                if let Some(sp) = sprite {
                    world.add_component(e, sp.clone());
                }
                if let Some(t) = tag {
                    world.add_component(e, Tag(t.clone()));
                }
                *selected = Some(e);
                respawned = Some(e);
            }
        }
        // record the id so redo despawns the exact recreated entity, not the current selection
        if let (Some(e), EditorCmd::DeleteEntity { entity, .. }) = (respawned, &mut cmd) {
            *entity = Some(e);
        }
        self.redo.push(cmd);
    }

    pub(super) fn redo(&mut self, world: &mut World, selected: &mut Option<Entity>) {
        let Some(cmd) = self.redo.pop() else { return };
        match &cmd {
            EditorCmd::MoveEntity {
                entity, new_pos, ..
            } => {
                if let Some(t) = world.get_mut::<crate::components::Transform>(*entity) {
                    t.position = *new_pos;
                }
                *selected = Some(*entity);
            }
            EditorCmd::CreateEntity { entity: _ } => {
                // was despawned by undo — spawn a new entity with a new id
                let e = world.spawn();
                world.add_component(e, crate::components::Transform::default());
                world.add_component(e, Tag("New Entity".into()));
                *selected = Some(e);
                // push the cmd updated with the new id onto the undo stack so it can be undone again
                // (previously drop(cmd) broke the chain, making the recreated entity un-undoable)
                self.undo.push(EditorCmd::CreateEntity { entity: e });
                return;
            }
            EditorCmd::DeleteEntity { entity, .. } => {
                // despawn exactly the entity recreated by undo (independent of current selection)
                if let Some(e) = *entity {
                    world.despawn(e);
                    if *selected == Some(e) {
                        *selected = None;
                    }
                }
            }
        }
        self.undo.push(cmd);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn entity_to_def(world: &World, entity: Entity) -> Option<crate::prefab::EntityDef> {
    Some(crate::prefab::EntityDef {
        tag: world.get::<crate::prefab::Tag>(entity).map(|t| t.0.clone()),
        transform: world.get::<crate::components::Transform>(entity).cloned(),
        sprite: world.get::<crate::components::Sprite>(entity).cloned(),
        parent: None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn snap_to_grid(pos: glam::Vec2, snap_size: f32) -> glam::Vec2 {
    glam::Vec2::new(
        (pos.x / snap_size).round() * snap_size,
        (pos.y / snap_size).round() * snap_size,
    )
}

#[cfg(not(target_arch = "wasm32"))]
impl App {
    pub(super) fn register_default_components(&mut self) {
        self.register_component("Sprite", |world, e| {
            world.add_component(e, crate::components::Sprite::default());
        });
        self.register_component("RenderLayer", |world, e| {
            world.add_component(e, crate::components::RenderLayer::default());
        });
        self.register_component("ParticleEmitter", |world, e| {
            world.add_component(e, crate::particle::ParticleEmitter::default());
        });
        // register removal closures — the Inspector "✕" button uses this map to show/act
        self.register_component_remover("Sprite", |world, e| {
            world.remove_component::<crate::components::Sprite>(e);
        });
        self.register_component_remover("RenderLayer", |world, e| {
            world.remove_component::<crate::components::RenderLayer>(e);
        });
        self.register_component_remover("ParticleEmitter", |world, e| {
            world.remove_component::<crate::particle::ParticleEmitter>(e);
        });
        self.register_component_remover("Tag", |world, e| {
            world.remove_component::<crate::prefab::Tag>(e);
        });
    }

    pub fn register_component(
        &mut self,
        name: impl Into<String>,
        factory: impl Fn(&mut World, Entity) + Send + Sync + 'static,
    ) {
        self.editor
            .component_factories
            .insert(name.into(), Box::new(factory));
    }

    /// Registers a removal closure so the Inspector can remove a component.
    /// Call this with the same name as `register_component` to make custom components
    /// removable; components absent from this map have their "✕" button hidden.
    pub fn register_component_remover(
        &mut self,
        name: impl Into<String>,
        remover: impl Fn(&mut World, Entity) + Send + Sync + 'static,
    ) {
        self.editor
            .component_removers
            .insert(name.into(), Box::new(remover));
    }
}
