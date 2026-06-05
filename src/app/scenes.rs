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
        // 패닉으로 비활성화된 시스템 인덱스 집합을 초기화한다.
        // 월드가 리셋되면 시스템에 다시 기회를 주고, Replace 로 systems 가 통째로
        // 교체될 때 옛 인덱스가 새 씬의 무관한 시스템을 잘못 건너뛰는 것을 막는다.
        // (새 월드에는 빈 PanickedSystems 리소스가 재삽입되므로 표시도 함께 초기화됨)
        self.panicked_systems.clear();
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
                    // Pop 으로 사라진 시스템 인덱스를 패닉 집합에서 제거한다(남은 인덱스는
                    // 그대로 유효). 이렇게 하지 않으면 잘려나간 인덱스가 그대로 남아
                    // 이후 같은 인덱스에 들어오는 시스템을 잘못 건너뛴다.
                    self.panicked_systems.retain(|&i| i < new_len);
                    // 표시용 PanickedSystems 리소스도 남은 인덱스 기준으로 재구성.
                    let names: Vec<String> = self
                        .panicked_systems
                        .iter()
                        .filter_map(|&i| self.systems.get(i).map(|s| s.name().to_string()))
                        .collect();
                    if let Some(ps) = self
                        .world
                        .resource_mut::<crate::resources::PanickedSystems>()
                    {
                        ps.disabled = names;
                    }
                    self.reconcile_meta(); // truncate 후 meta 동기화
                }
            }
        }
    }
}
