use std::collections::HashMap;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

mod assets;
mod core_resources;
mod editor;
mod egui_pass;
mod render;
mod scenes;
mod schedule;
mod window;

pub use schedule::{ScheduleErrorPolicy, SystemPanicPolicy};

#[cfg(not(target_arch = "wasm32"))]
use editor::EditorHistory;
#[cfg(test)]
use egui_pass::paint_jobs_contain_callbacks;

// WASM: GPU 초기화는 async(WebGPU Promise 기반)이므로 thread_local로 결과를 전달한다.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static PENDING_GPU: std::cell::RefCell<Option<(
        crate::renderer::GpuContext,
        Arc<winit::window::Window>,
    )>> = const { std::cell::RefCell::new(None) };
}

use glam::Vec2;
use winit::window::Window;

use crate::{
    asset::{AssetServer, Handle, ImageAsset},
    camera::Camera,
    debug_ui::DebugUi,
    ecs::{Entity, Events, System, World},
    hierarchy::HierarchySystem,
    input::{GamepadState, InputState, TouchState},
    prefab::Tag,
    reflect::ReflectValue,
    renderer::{
        DrawRect, FrameContext, GpuContext, PostProcessConfig, PostProcessRenderer, SpriteRenderer,
        TextRenderer, UiImageQueue, UiQueue,
    },
    resources::{
        DebugDraw, DebugDrawQueue, DebugRect, DisplayScaleFactor, FontData, LoadProgress,
        PendingResize, ShouldQuit, ViewportSize, WindowConfig,
    },
    scene::{Scene, SceneChange, SceneCmd},
};

type EventHook = Box<dyn Fn(&mut World)>;
#[cfg(not(target_arch = "wasm32"))]
type ComponentFactory = Box<dyn Fn(&mut World, Entity) + Send + Sync>;
type OffscreenRenderInfo = (
    String,
    crate::camera::Camera,
    u32, // rt_w
    u32, // rt_h
    *const wgpu::TextureView,
    Arc<wgpu::BindGroup>,
    u32, // layer_mask
);

/// 엔진 진입점.
///
/// # 사용법
/// ```rust,no_run
/// # use engine::App;
/// let mut app = App::new();
/// app.world.spawn();
/// // app.add_system(MySystem);
/// app.run();
/// ```
pub struct App {
    /// ECS 세계 (엔티티·컴포넌트·리소스)
    pub world: World,

    systems: Vec<Box<dyn System>>,
    /// 시스템별 라벨/순서/그룹 메타데이터. systems와 동일한 인덱스로 평행 보관.
    system_meta: Vec<crate::ecs::schedule::SystemMeta>,
    /// compute_order가 계산한 실행 순서 (인덱스 목록).
    exec_order: Vec<usize>,
    /// system_meta가 변경됐을 때 true — 다음 프레임에 재계산.
    schedule_dirty: bool,
    /// 비활성화된 SystemSet 라벨. 해당 set의 시스템은 실행을 건너뛴다.
    disabled_sets: std::collections::HashSet<crate::ecs::schedule::SystemLabel>,
    /// 패닉으로 비활성화된 시스템 인덱스 집합. 해당 시스템은 이후 프레임에서 건너뛴다.
    panicked_systems: std::collections::HashSet<usize>,
    /// 스케줄 순환 의존성 처리 정책.
    schedule_error_policy: ScheduleErrorPolicy,
    /// 시스템 panic 처리 정책.
    system_panic_policy: SystemPanicPolicy,
    /// (씬, 해당 씬이 등록한 시스템 수). Push/Pop 시 시스템 복원에 사용.
    scene_stack: Vec<(Box<dyn Scene>, usize)>,
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    sprite_renderer: Option<SpriteRenderer>,
    /// 스프라이트 pass 직후 텍스트를 덮어쓴다. GPU 초기화 이후 Some으로 채워진다.
    text_renderer: Option<TextRenderer>,
    /// PostProcessConfig 리소스가 enabled=true일 때 활성화된다.
    post_renderer: Option<PostProcessRenderer>,
    /// AmbientLight 리소스가 등록된 동안 활성화되는 라이팅 렌더러.
    #[cfg(not(target_arch = "wasm32"))]
    lighting_renderer: Option<crate::renderer::lighting::LightingRenderer>,
    /// FadeTransition 리소스가 alpha > 0 일 때 마지막 패스로 실행되는 페이드 렌더러.
    #[cfg(not(target_arch = "wasm32"))]
    fade_renderer: Option<crate::renderer::fade::FadeRenderer>,
    /// 라이팅 패스가 씬을 먼저 그릴 중간 텍스처.
    #[cfg(not(target_arch = "wasm32"))]
    scene_texture_for_lighting: Option<(
        wgpu::Texture,
        wgpu::TextureView,
        u32,
        u32,
        wgpu::TextureFormat,
    )>,
    /// post+lighting 조합에서 post 결과를 lighting 입력으로 넘기는 중간 텍스처.
    #[cfg(not(target_arch = "wasm32"))]
    post_texture_for_lighting: Option<(
        wgpu::Texture,
        wgpu::TextureView,
        u32,
        u32,
        wgpu::TextureFormat,
    )>,
    /// GPU 컴퓨트 셰이더 기반 파티클 렌더러 (lazy init).
    #[cfg(not(target_arch = "wasm32"))]
    gpu_particle_renderer: Option<crate::renderer::gpu_particle::GpuParticleRenderer>,
    last_frame: Option<Instant>,
    last_dt: f32,
    /// GPU 초기화 전에 등록된 텍스처 경로를 보관한다. resumed()에서 실제로 로드한다.
    pending_textures: Vec<String>,
    /// 등록된 오프스크린 렌더 타겟 맵 (이름 → RenderTarget).
    render_targets: HashMap<String, crate::renderer::render_target::RenderTarget>,
    /// GPU 초기화 전에 등록된 렌더 타겟 정보. finish_init()에서 실제 생성한다.
    pending_render_targets: Vec<(String, u32, u32)>,
    /// 매 프레임 종료 시 이벤트 큐를 비우는 클로저 목록.
    event_flushers: Vec<EventHook>,
    /// reload_scene 시 이벤트 리소스를 재삽입하는 클로저 목록.
    event_initializers: Vec<EventHook>,
    /// 씬 전환(World 리셋)을 넘어 보존할 리소스 타입 목록 (`register_persistent`).
    persistent_resources: Vec<std::any::TypeId>,
    /// gilrs 게임패드 컨텍스트. 초기화 실패 시 None (게임패드 없이 동작).
    #[cfg(not(target_arch = "wasm32"))]
    gilrs: Option<gilrs::Gilrs>,
    /// egui 렌더러 (wgpu 백엔드).
    egui_renderer: Option<egui_wgpu::Renderer>,
    /// winit ↔ egui 이벤트 변환기.
    egui_state: Option<egui_winit::State>,
    /// update() 에서 tessellate 한 결과를 render() 까지 전달하는 임시 버퍼.
    egui_output: Option<(Vec<egui::ClippedPrimitive>, egui::TexturesDelta, f32)>,
    /// Inspector 패널에서 현재 선택된 엔티티.
    inspector_selected: Option<Entity>,
    /// 멀티 선택된 엔티티 목록 (inspector_selected 포함).
    #[cfg(not(target_arch = "wasm32"))]
    selected_entities: Vec<Entity>,
    /// Ctrl+C로 복사된 EntityDef 클립보드.
    #[cfg(not(target_arch = "wasm32"))]
    copy_clipboard: Vec<crate::prefab::EntityDef>,
    /// Gizmo 드래그 중인지 여부.
    gizmo_dragging: bool,
    /// 드래그 시작 시 (엔티티 position - 커서 월드 좌표) 오프셋.
    gizmo_drag_offset: glam::Vec2,
    /// Inspector 씬 저장 경로 (네이티브 전용).
    #[cfg(not(target_arch = "wasm32"))]
    editor_save_path: String,
    /// 마지막 씬 저장 결과 메시지.
    editor_save_status: Option<String>,
    /// 마지막 씬 로드 결과 메시지.
    #[cfg(not(target_arch = "wasm32"))]
    editor_load_status: Option<String>,
    /// Inspector 현재 탭 인덱스 (0: Entities, 1: Assets).
    inspector_tab: u8,
    /// 에디터 실행 취소/다시 실행 히스토리.
    #[cfg(not(target_arch = "wasm32"))]
    cmd_history: EditorHistory,
    /// Gizmo 드래그 시작 시 엔티티 위치 (undo 기록용).
    #[cfg(not(target_arch = "wasm32"))]
    gizmo_drag_start_pos: Option<glam::Vec2>,
    /// Gizmo 그룹 드래그 시작 시 각 선택 엔티티의 위치 (그룹 이동 undo 기록용).
    #[cfg(not(target_arch = "wasm32"))]
    gizmo_drag_start_positions: Vec<(Entity, glam::Vec2)>,
    /// 컴포넌트 추가 팩토리 맵 (네이티브 전용). 타입 이름 → World에 컴포넌트를 추가하는 클로저.
    #[cfg(not(target_arch = "wasm32"))]
    component_factories: HashMap<String, ComponentFactory>,
    /// 컴포넌트 제거 클로저 맵 (네이티브 전용). 타입 이름 → World에서 컴포넌트를 제거하는 클로저.
    /// 이 맵에 등록된 컴포넌트만 Inspector에서 "✕"(제거) 버튼이 노출/동작한다.
    #[cfg(not(target_arch = "wasm32"))]
    component_removers: HashMap<String, ComponentFactory>,
    /// "Add Component" 드롭다운에서 현재 선택된 컴포넌트 이름 (네이티브 전용).
    #[cfg(not(target_arch = "wasm32"))]
    add_component_selected: String,
    /// Gizmo 드래그 Grid Snap 활성화 여부 (네이티브 전용).
    #[cfg(not(target_arch = "wasm32"))]
    snap_enabled: bool,
    /// Gizmo 드래그 Grid Snap 격자 크기 (픽셀, 네이티브 전용).
    #[cfg(not(target_arch = "wasm32"))]
    snap_size: f32,
}

impl App {
    fn insert_core_resources(world: &mut World) {
        core_resources::insert_core_resources(world);
    }

    fn register_core_component_metadata(world: &mut World) {
        core_resources::register_core_component_metadata(world);
    }

    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let gilrs = gilrs::Gilrs::new().ok();
        let mut world = World::new();
        Self::insert_core_resources(&mut world);
        Self::register_core_component_metadata(&mut world);

        #[cfg(not(target_arch = "wasm32"))]
        let mut app = Self {
            world,
            systems: Vec::new(),
            system_meta: Vec::new(),
            exec_order: Vec::new(),
            schedule_dirty: true,
            disabled_sets: std::collections::HashSet::new(),
            panicked_systems: std::collections::HashSet::new(),
            schedule_error_policy: ScheduleErrorPolicy::default(),
            system_panic_policy: SystemPanicPolicy::default(),
            scene_stack: Vec::new(),
            window: None,
            gpu: None,
            sprite_renderer: None,
            text_renderer: None,
            post_renderer: None,
            lighting_renderer: None,
            fade_renderer: None,
            scene_texture_for_lighting: None,
            post_texture_for_lighting: None,
            gpu_particle_renderer: None,
            last_frame: None,
            last_dt: 1.0 / 60.0,
            pending_textures: Vec::new(),
            render_targets: HashMap::new(),
            pending_render_targets: Vec::new(),
            event_flushers: Vec::new(),
            event_initializers: Vec::new(),
            persistent_resources: Vec::new(),
            gilrs,
            egui_renderer: None,
            egui_state: None,
            egui_output: None,
            inspector_selected: None,
            selected_entities: Vec::new(),
            copy_clipboard: Vec::new(),
            gizmo_dragging: false,
            gizmo_drag_offset: glam::Vec2::ZERO,
            editor_save_path: "saved_scene.ron".into(),
            editor_save_status: None,
            editor_load_status: None,
            inspector_tab: 0,
            cmd_history: EditorHistory::new(),
            gizmo_drag_start_pos: None,
            gizmo_drag_start_positions: Vec::new(),
            component_factories: HashMap::new(),
            component_removers: HashMap::new(),
            add_component_selected: String::new(),
            snap_enabled: false,
            snap_size: 16.0,
        };
        #[cfg(not(target_arch = "wasm32"))]
        app.register_default_components();
        #[cfg(not(target_arch = "wasm32"))]
        return app;

        #[cfg(target_arch = "wasm32")]
        Self {
            world,
            systems: Vec::new(),
            system_meta: Vec::new(),
            exec_order: Vec::new(),
            schedule_dirty: true,
            disabled_sets: std::collections::HashSet::new(),
            panicked_systems: std::collections::HashSet::new(),
            schedule_error_policy: ScheduleErrorPolicy::default(),
            system_panic_policy: SystemPanicPolicy::default(),
            scene_stack: Vec::new(),
            window: None,
            gpu: None,
            sprite_renderer: None,
            text_renderer: None,
            post_renderer: None,
            last_frame: None,
            last_dt: 1.0 / 60.0,
            pending_textures: Vec::new(),
            render_targets: HashMap::new(),
            pending_render_targets: Vec::new(),
            event_flushers: Vec::new(),
            event_initializers: Vec::new(),
            persistent_resources: Vec::new(),
            egui_renderer: None,
            egui_state: None,
            egui_output: None,
            inspector_selected: None,
            gizmo_dragging: false,
            gizmo_drag_offset: glam::Vec2::ZERO,
            editor_save_status: None,
            inspector_tab: 0,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// ─── winit ApplicationHandler 구현 ───────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::schedule::SystemConfig;

    #[derive(Default)]
    struct Counter(u32);

    struct CountSystem;
    impl System for CountSystem {
        fn run(&mut self, world: &mut World, _dt: f32) {
            world.resource_mut::<Counter>().unwrap().0 += 1;
        }
        fn name(&self) -> &'static str {
            "count"
        }
    }

    struct PanicSystem;
    impl System for PanicSystem {
        fn run(&mut self, _world: &mut World, _dt: f32) {
            panic!("intentional test panic");
        }
        fn name(&self) -> &'static str {
            "panic"
        }
    }

    fn app_with_counter() -> App {
        let mut app = App::new();
        app.world.insert_resource(Counter::default());
        app
    }

    #[test]
    fn schedule_cycle_default_falls_back_to_insertion_order() {
        let mut app = app_with_counter();
        app.add_system_labeled(CountSystem, SystemConfig::new().label("a").after("b"));
        app.add_system_labeled(CountSystem, SystemConfig::new().label("b").after("a"));

        app.update(1.0 / 60.0);

        assert_eq!(app.world.resource::<Counter>().unwrap().0, 2);
    }

    #[test]
    fn schedule_cycle_can_skip_user_systems() {
        let mut app = app_with_counter();
        app.set_schedule_error_policy(ScheduleErrorPolicy::DisableRunOnCycle);
        app.add_system_labeled(CountSystem, SystemConfig::new().label("a").after("b"));
        app.add_system_labeled(CountSystem, SystemConfig::new().label("b").after("a"));

        app.update(1.0 / 60.0);

        assert_eq!(app.world.resource::<Counter>().unwrap().0, 0);
    }

    #[test]
    #[should_panic(expected = "시스템 순서 순환 의존성 감지")]
    fn schedule_cycle_can_panic() {
        let mut app = app_with_counter();
        app.set_schedule_error_policy(ScheduleErrorPolicy::PanicOnCycle);
        app.add_system_labeled(CountSystem, SystemConfig::new().label("a").after("b"));
        app.add_system_labeled(CountSystem, SystemConfig::new().label("b").after("a"));

        app.update(1.0 / 60.0);
    }

    #[test]
    fn default_panic_policy_disables_panicked_system_and_continues() {
        let mut app = app_with_counter();
        app.add_system(PanicSystem);
        app.add_system(CountSystem);

        app.update(1.0 / 60.0);
        app.update(1.0 / 60.0);

        assert_eq!(app.world.resource::<Counter>().unwrap().0, 2);
        assert_eq!(app.panicked_systems.len(), 1);
    }

    #[test]
    #[should_panic(expected = "intentional test panic")]
    fn abort_after_log_panic_policy_rethrows() {
        let mut app = app_with_counter();
        app.set_system_panic_policy(SystemPanicPolicy::AbortAfterLog);
        app.add_system(PanicSystem);

        app.update(1.0 / 60.0);
    }

    #[test]
    fn reload_scene_restores_core_resources_and_preserves_debug_ui() {
        let mut app = App::new();
        app.world
            .insert_resource(DebugUi::new_with_ctx(egui::Context::default()));

        app.reload_scene();

        assert!(app
            .world
            .resource::<crate::resources::PanickedSystems>()
            .is_some());
        assert!(app.world.resource::<SceneChange>().is_some());
        assert!(app.world.resource::<LoadProgress>().is_some());
        assert!(app.world.resource::<DebugUi>().is_some());
    }

    #[test]
    fn egui_callback_jobs_are_detected_before_unsafe_render_pass() {
        let callback = egui::ClippedPrimitive {
            clip_rect: egui::Rect::EVERYTHING,
            primitive: egui::epaint::Primitive::Callback(egui::epaint::PaintCallback {
                rect: egui::Rect::EVERYTHING,
                callback: std::sync::Arc::new(()),
            }),
        };

        assert!(paint_jobs_contain_callbacks(&[callback]));
        assert!(!paint_jobs_contain_callbacks(&[]));
    }

    #[test]
    fn scene_replace_clears_panicked_systems() {
        // 회귀 테스트: 씬 A 에서 패닉으로 비활성화된 시스템 인덱스가 Replace 이후
        // 씬 B 의 같은 인덱스 시스템을 잘못 건너뛰면 안 된다.
        struct SceneA;
        impl Scene for SceneA {
            fn on_enter(&mut self, _w: &mut World, systems: &mut Vec<Box<dyn System>>) {
                systems.push(Box::new(PanicSystem)); // idx 0 — 패닉 후 비활성화
                systems.push(Box::new(CountSystem)); // idx 1
            }
            fn on_exit(&mut self, _w: &mut World) {}
        }
        struct SceneB;
        impl Scene for SceneB {
            fn on_enter(&mut self, _w: &mut World, systems: &mut Vec<Box<dyn System>>) {
                systems.push(Box::new(CountSystem)); // idx 0
                systems.push(Box::new(CountSystem)); // idx 1
            }
            fn on_exit(&mut self, _w: &mut World) {}
        }

        let mut app = App::new();
        app.register_persistent::<Counter>();
        app.world.insert_resource(Counter::default());
        app.set_scene(Box::new(SceneA));

        // 씬 A: PanicSystem(idx0) 패닉→비활성화, CountSystem(idx1) 실행 → +1
        app.update(1.0 / 60.0);
        assert_eq!(app.world.resource::<Counter>().unwrap().0, 1);
        assert!(app.panicked_systems.contains(&0));

        // 씬 B 로 교체: panicked_systems 가 초기화되어 idx0/idx1 모두 실행 → +2
        app.set_scene(Box::new(SceneB));
        app.update(1.0 / 60.0);
        assert_eq!(
            app.world.resource::<Counter>().unwrap().0,
            3,
            "stale panicked index suppressed a new scene's system"
        );
        assert!(app.panicked_systems.is_empty());
    }
}
