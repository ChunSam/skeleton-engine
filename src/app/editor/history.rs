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
    /// One 🗑 Delete, of an entity **and its whole subtree** — `hierarchy::despawn_recursive`,
    /// since v0.156.0. Deleting a parent alone left its children pointing at a dead handle and
    /// drawing at their local offsets.
    DeleteEntity {
        /// Entity ids recreated by undo, aligned with `defs`. Filled in at undo time so redo
        /// despawns exactly those (None on first creation — the originals are already gone).
        entities: Option<Vec<Entity>>,
        /// The deleted subtree, **parents before children**, each with the index *into this same
        /// list* of its parent; `None` marks the deleted root.
        ///
        /// ⚠️ The link is an index and not `EntityDef.parent`, which is a **tag**: a parent with
        /// no `Tag`, or two entities sharing one, drops or misroutes it — that is the "parent
        /// link dropped" the scene saver warns about. Within one delete the real structure is
        /// known, so it is kept. The root is the exception: its own parent is outside the subtree
        /// and stays tag-based, exactly as it was before.
        defs: Vec<(crate::prefab::EntityDef, Option<usize>)>,
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
    ///
    /// Undo/redo route through the same cycle-safe `reparent`, so if an interleaved edit had made
    /// `old_parent` a descendant of `entity`, restoring it would be a silent no-op. Normal LIFO
    /// undo can't reach that state (every hierarchy edit is itself an undo entry, unwound in order).
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

    /// Drops both stacks. Called whenever entity identity stops being meaningful — a world
    /// reset or a scene load — because every `EditorCmd` stores raw `Entity` handles. The ECS
    /// reuses entity ids, so a stale `DeleteEntity(e)` sitting in the undo stack does not fail
    /// after a reset: it resolves onto whatever NEW entity now occupies that slot and deletes
    /// that instead. Ctrl+Z then destroys something the user never touched.
    ///
    /// ⚠️ **`Entity` being generation-checked does not save this, and the reasoning that says it
    /// does is a trap this doc has already caught once.** A reset does `self.world =
    /// World::new()` (`app/scenes.rs`), so the new world's generation counters start at 0 and an
    /// old `Entity { index: 0, generation: 0 }` compares **equal** to the first entity spawned
    /// after it. Measured: a handle from the old world reads the *new* world's component through
    /// it. The generation check rules out reuse only *within* one world, where `despawn` bumps
    /// the counter. Do not simplify this call away on the strength of it.
    ///
    /// The editor's own scene load is the second call site and is justified differently: it
    /// despawns rather than rebuilding the world, so generations do bump and most stale commands
    /// become silent no-ops — but `DeleteEntity`'s undo spawns unconditionally, which is the
    /// resurrection v0.155.5 fixed.
    pub(in crate::app) fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Number of undoable commands on the stack. Test-only accessor — the fields are private
    /// to this module and `editor/tests.rs` is a sibling, not a descendant.
    #[cfg(test)]
    pub(in crate::app) fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub(in crate::app) fn push(&mut self, cmd: EditorCmd) {
        self.undo.push(cmd);
        self.redo.clear();
    }

    pub(in crate::app) fn undo(&mut self, world: &mut World, selected: &mut Option<Entity>) {
        let Some(mut cmd) = self.undo.pop() else {
            return;
        };
        // entity ids recreated by DeleteEntity undo — recorded into cmd after the match
        let mut respawned: Option<Vec<Entity>> = None;
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
            EditorCmd::DeleteEntity { defs, .. } => {
                // Parents before children, so `spawned[ix]` is always already live.
                let mut spawned: Vec<Entity> = Vec::with_capacity(defs.len());
                for (def, parent_ix) in defs {
                    let e = crate::prefab::spawn_entity_def(world, def);
                    if let Some(&p) = parent_ix.and_then(|ix| spawned.get(ix)) {
                        // `reparent`, not `attach`: `spawn_entity_def` may already have attached
                        // this entity by tag, and attaching twice would leave the first parent's
                        // `Children` holding it. Reparent detaches first (and refuses cycles).
                        crate::hierarchy::reparent(world, e, Some(p));
                    }
                    spawned.push(e);
                }
                *selected = spawned.first().copied();
                respawned = Some(spawned);
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
        if let (Some(list), EditorCmd::DeleteEntity { entities, .. }) = (respawned, &mut cmd) {
            *entities = Some(list);
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
            EditorCmd::DeleteEntity { entities, .. } => {
                // Despawn exactly the entities recreated by undo, independent of the current
                // selection. Plain `despawn` and not `despawn_recursive`: the list already holds
                // the whole subtree, and anything attached *since* the undo is not ours to take.
                for &e in entities.iter().flatten() {
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
