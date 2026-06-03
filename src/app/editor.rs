use super::*;

mod ui;

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
        tag: Option<String>,
        transform: Option<crate::components::Transform>,
        sprite: Option<crate::components::Sprite>,
    },
}

#[cfg(not(target_arch = "wasm32"))]
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
        let Some(cmd) = self.undo.pop() else { return };
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
            }
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
                // 엔티티가 이미 despawn 됐으므로 새로 스폰 (id가 달라짐 — 허용)
                let e = world.spawn();
                world.add_component(e, crate::components::Transform::default());
                world.add_component(e, Tag("New Entity".into()));
                *selected = Some(e);
                // redo stack의 cmd를 업데이트할 수 없으므로 이 분기는 새 entity로 처리
                drop(cmd);
                return;
            }
            EditorCmd::DeleteEntity { .. } => {
                if let Some(sel) = *selected {
                    world.despawn(sel);
                    *selected = None;
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
    }

    pub fn register_component(
        &mut self,
        name: impl Into<String>,
        factory: impl Fn(&mut World, Entity) + Send + Sync + 'static,
    ) {
        self.component_factories
            .insert(name.into(), Box::new(factory));
    }
}
