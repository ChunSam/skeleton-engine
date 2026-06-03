use super::*;

impl App {
    pub fn register_persistent<T: 'static>(&mut self) {
        let tid = std::any::TypeId::of::<T>();
        if !self.persistent_resources.contains(&tid) {
            self.persistent_resources.push(tid);
        }
    }

    pub fn reload_scene(&mut self) {
        // 리셋 전에 보존 대상 리소스를 type-erased 박스로 꺼내 둔다.
        let preserved: Vec<(std::any::TypeId, Box<dyn std::any::Any>)> = self
            .persistent_resources
            .iter()
            .filter_map(|&tid| self.world.take_resource_erased(tid).map(|b| (tid, b)))
            .collect();
        let debug_ui = self.world.remove_resource::<DebugUi>();

        self.world = World::new();
        Self::insert_core_resources(&mut self.world);
        // 등록된 이벤트 리소스 재삽입
        let inits = std::mem::take(&mut self.event_initializers);
        for init in &inits {
            init(&mut self.world);
        }
        self.event_initializers = inits;
        Self::register_core_component_metadata(&mut self.world);
        if let Some(debug_ui) = debug_ui {
            self.world.insert_resource(debug_ui);
        }
        // 보존 대상 리소스를 마지막에 재삽입 — 엔진 기본 리소스보다 우선시되도록
        // (같은 타입이면 보존본이 기본본을 덮어쓴다).
        for (tid, boxed) in preserved {
            self.world.insert_resource_erased(tid, boxed);
        }
        self.inspector_selected = None;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.selected_entities.clear();
            self.copy_clipboard.clear();
        }
        self.editor_save_status = None;
    }

    pub fn set_scene(&mut self, scene: Box<dyn Scene>) {
        self.apply_scene_cmd(SceneCmd::Replace(scene));
    }

    fn reconcile_meta(&mut self) {
        use crate::ecs::schedule::SystemMeta;
        if self.system_meta.len() < self.systems.len() {
            self.system_meta
                .resize(self.systems.len(), SystemMeta::default());
        } else if self.system_meta.len() > self.systems.len() {
            self.system_meta.truncate(self.systems.len());
        }
        self.schedule_dirty = true;
    }

    pub(super) fn apply_scene_cmd(&mut self, cmd: SceneCmd) {
        match cmd {
            SceneCmd::Replace(mut new_scene) => {
                for (mut scene, _) in self.scene_stack.drain(..).rev() {
                    scene.on_exit(&mut self.world);
                }
                self.systems.clear();
                self.reconcile_meta(); // systems.clear() 후 meta 동기화
                self.reload_scene();
                let before = self.systems.len();
                new_scene.on_enter(&mut self.world, &mut self.systems);
                let owned = self.systems.len() - before;
                self.scene_stack.push((new_scene, owned));
                self.reconcile_meta(); // on_enter 후 씬이 직접 push한 시스템 흡수
            }
            SceneCmd::Push(mut new_scene) => {
                let before = self.systems.len();
                new_scene.on_enter(&mut self.world, &mut self.systems);
                let owned = self.systems.len() - before;
                self.scene_stack.push((new_scene, owned));
                self.reconcile_meta(); // on_enter 후 씬이 직접 push한 시스템 흡수
            }
            SceneCmd::Pop => {
                if let Some((mut scene, owned)) = self.scene_stack.pop() {
                    scene.on_exit(&mut self.world);
                    let new_len = self.systems.len().saturating_sub(owned);
                    self.systems.truncate(new_len);
                    self.reconcile_meta(); // truncate 후 meta 동기화
                }
            }
        }
    }
}
