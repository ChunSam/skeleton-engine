use crate::ecs::{Entity, World};
use crate::prefab::Tag;

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub(in crate::app) enum EditorCmd {
    MoveEntity {
        entity: Entity,
        old_pos: glam::Vec2,
        new_pos: glam::Vec2,
    },
    CreateEntity {
        entity: Entity,
        /// Full entity def for redo — `Some` for Duplicate/Paste (restores all components),
        /// `None` for New Entity (redo spawns a default entity).
        def: Option<crate::prefab::EntityDef>,
    },
    DeleteEntity {
        /// Entity id recreated by undo. Filled in at undo time so redo can despawn exactly
        /// this entity (None on first creation — the original has already been despawned).
        entity: Option<Entity>,
        /// Full captured state at delete time; undo restores all components via spawn_entity_def.
        def: crate::prefab::EntityDef,
    },
    /// Move a screen-space `UiNode` widget (offset changed, size/anchor unchanged).
    MoveUiNode {
        entity: Entity,
        old_offset: glam::Vec2,
        new_offset: glam::Vec2,
    },
    /// Resize a screen-space `UiNode` widget (both offset and size may change).
    ResizeUiNode {
        entity: Entity,
        old_offset: glam::Vec2,
        old_size: glam::Vec2,
        new_offset: glam::Vec2,
        new_size: glam::Vec2,
    },
    /// Resize a world-space sprite by changing `Transform.scale` (center fixed).
    ResizeEntity {
        entity: Entity,
        old_scale: glam::Vec2,
        new_scale: glam::Vec2,
    },
    /// Rotate a world-space sprite by changing `Transform.rotation` (radians).
    RotateEntity {
        entity: Entity,
        old_rotation: f32,
        new_rotation: f32,
    },
    /// One tile-paint stroke on a `Tilemap` entity. `changes` lists every cell the stroke
    /// actually modified as `(row, col, old, new)`; undo restores `old` (reverse order),
    /// redo re-applies `new`. A whole drag is a single undo step.
    PaintTiles {
        entity: Entity,
        changes: Vec<(usize, usize, u32, u32)>,
    },
    /// Re-parent an entity in the Scene-tree hierarchy (drag-to-reparent). Undo restores the
    /// `old_parent`, redo re-applies the `new_parent` — both through the cycle-safe
    /// [`crate::hierarchy::reparent`]. `None` means a root (no parent).
    Reparent {
        entity: Entity,
        old_parent: Option<Entity>,
        new_parent: Option<Entity>,
    },
}

#[derive(Default)]
pub(in crate::app) struct EditorHistory {
    undo: Vec<EditorCmd>,
    redo: Vec<EditorCmd>,
}

impl EditorHistory {
    pub(in crate::app) fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub(in crate::app) fn push(&mut self, cmd: EditorCmd) {
        self.undo.push(cmd);
        self.redo.clear();
    }

    pub(in crate::app) fn undo(&mut self, world: &mut World, selected: &mut Option<Entity>) {
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
            EditorCmd::CreateEntity { entity, .. } => {
                world.despawn(*entity);
                *selected = None;
            }
            EditorCmd::DeleteEntity { def, .. } => {
                let e = crate::prefab::spawn_entity_def(world, def);
                *selected = Some(e);
                respawned = Some(e);
            }
            EditorCmd::MoveUiNode {
                entity, old_offset, ..
            } => {
                if let Some(n) = world.get_mut::<crate::ui::UiNode>(*entity) {
                    n.offset = *old_offset;
                }
                *selected = Some(*entity);
            }
            EditorCmd::ResizeUiNode {
                entity,
                old_offset,
                old_size,
                ..
            } => {
                if let Some(n) = world.get_mut::<crate::ui::UiNode>(*entity) {
                    n.offset = *old_offset;
                    n.size = *old_size;
                }
                *selected = Some(*entity);
            }
            EditorCmd::ResizeEntity {
                entity, old_scale, ..
            } => {
                if let Some(t) = world.get_mut::<crate::components::Transform>(*entity) {
                    t.scale = *old_scale;
                }
                *selected = Some(*entity);
            }
            EditorCmd::RotateEntity {
                entity,
                old_rotation,
                ..
            } => {
                if let Some(t) = world.get_mut::<crate::components::Transform>(*entity) {
                    t.rotation = *old_rotation;
                }
                *selected = Some(*entity);
            }
            EditorCmd::PaintTiles { entity, changes } => {
                if let Some(tm) = world.get_mut::<crate::tilemap::Tilemap>(*entity) {
                    for (row, col, old, _new) in changes.iter().rev() {
                        tm.set_tile(*row, *col, *old);
                    }
                }
                // Re-sync colliders so undo restores the pre-stroke physics state too.
                crate::physics::sync_tilemap_entity_colliders(world, *entity);
                *selected = Some(*entity);
            }
            EditorCmd::Reparent {
                entity, old_parent, ..
            } => {
                crate::hierarchy::reparent(world, *entity, *old_parent);
                *selected = Some(*entity);
            }
        }
        // record the id so redo despawns the exact recreated entity, not the current selection
        if let (Some(e), EditorCmd::DeleteEntity { entity, .. }) = (respawned, &mut cmd) {
            *entity = Some(e);
        }
        self.redo.push(cmd);
    }

    pub(in crate::app) fn redo(&mut self, world: &mut World, selected: &mut Option<Entity>) {
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
            EditorCmd::CreateEntity { def, .. } => {
                // was despawned by undo — re-spawn from def (Duplicate/Paste) or default (New Entity)
                let e = if let Some(d) = def {
                    crate::prefab::spawn_entity_def(world, d)
                } else {
                    let e = world.spawn();
                    world.add_component(e, crate::components::Transform::default());
                    world.add_component(e, Tag("New Entity".into()));
                    e
                };
                *selected = Some(e);
                // push cmd updated with the new entity id so it can be undone again
                self.undo.push(EditorCmd::CreateEntity {
                    entity: e,
                    def: def.clone(),
                });
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
            EditorCmd::MoveUiNode {
                entity, new_offset, ..
            } => {
                if let Some(n) = world.get_mut::<crate::ui::UiNode>(*entity) {
                    n.offset = *new_offset;
                }
                *selected = Some(*entity);
            }
            EditorCmd::ResizeUiNode {
                entity,
                new_offset,
                new_size,
                ..
            } => {
                if let Some(n) = world.get_mut::<crate::ui::UiNode>(*entity) {
                    n.offset = *new_offset;
                    n.size = *new_size;
                }
                *selected = Some(*entity);
            }
            EditorCmd::ResizeEntity {
                entity, new_scale, ..
            } => {
                if let Some(t) = world.get_mut::<crate::components::Transform>(*entity) {
                    t.scale = *new_scale;
                }
                *selected = Some(*entity);
            }
            EditorCmd::RotateEntity {
                entity,
                new_rotation,
                ..
            } => {
                if let Some(t) = world.get_mut::<crate::components::Transform>(*entity) {
                    t.rotation = *new_rotation;
                }
                *selected = Some(*entity);
            }
            EditorCmd::PaintTiles { entity, changes } => {
                if let Some(tm) = world.get_mut::<crate::tilemap::Tilemap>(*entity) {
                    for (row, col, _old, new) in changes.iter() {
                        tm.set_tile(*row, *col, *new);
                    }
                }
                // Re-sync colliders so redo re-applies the post-stroke physics state too.
                crate::physics::sync_tilemap_entity_colliders(world, *entity);
                *selected = Some(*entity);
            }
            EditorCmd::Reparent {
                entity, new_parent, ..
            } => {
                crate::hierarchy::reparent(world, *entity, *new_parent);
                *selected = Some(*entity);
            }
        }
        self.undo.push(cmd);
    }
}
