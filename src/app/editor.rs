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
        /// undo 가 재생성한 엔티티 id. redo 에서 정확히 이 엔티티를 despawn 하기 위해
        /// undo 시점에 채워진다(최초 생성 시엔 None — 원본은 이미 despawn 됨).
        entity: Option<Entity>,
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
        let Some(mut cmd) = self.undo.pop() else {
            return;
        };
        // DeleteEntity undo 가 재생성한 엔티티 id — match 종료 후 cmd 에 기록한다.
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
        // redo 가 현재 선택이 아니라 정확히 재생성된 엔티티를 despawn 하도록 id 를 기록.
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
                // undo 시 despawn 됐으므로 새 엔티티를 스폰한다(새 id).
                let e = world.spawn();
                world.add_component(e, crate::components::Transform::default());
                world.add_component(e, Tag("New Entity".into()));
                *selected = Some(e);
                // 새 id 로 갱신한 cmd 를 undo 스택에 올려 다시 undo 가능하게 한다.
                // (기존엔 drop(cmd) 로 체인이 끊겨 재생성된 엔티티를 undo 할 수 없었다)
                self.undo.push(EditorCmd::CreateEntity { entity: e });
                return;
            }
            EditorCmd::DeleteEntity { entity, .. } => {
                // undo 가 재생성한 엔티티를 정확히 despawn (현재 선택과 무관).
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
        // 제거 클로저 등록 — Inspector "✕" 버튼이 이 맵을 기준으로 노출/동작한다.
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
        self.component_factories
            .insert(name.into(), Box::new(factory));
    }

    /// Inspector에서 컴포넌트를 제거할 수 있도록 제거 클로저를 등록한다.
    /// `register_component` 로 추가 가능한 커스텀 컴포넌트를 제거 가능하게 하려면
    /// 같은 이름으로 이 메서드도 호출한다(이 맵에 없는 컴포넌트는 "✕" 버튼이 숨겨진다).
    pub fn register_component_remover(
        &mut self,
        name: impl Into<String>,
        remover: impl Fn(&mut World, Entity) + Send + Sync + 'static,
    ) {
        self.component_removers
            .insert(name.into(), Box::new(remover));
    }
}
