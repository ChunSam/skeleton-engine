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

// WASM: GPU init is async (WebGPU Promise-based), so we use thread_local to pass the result.
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

/// Engine entry point.
///
/// # Usage
/// ```rust,no_run
/// # use engine::App;
/// let mut app = App::new();
/// app.world.spawn();
/// // app.add_system(MySystem);
/// app.run();
/// ```
pub struct App {
    /// ECS world (entities, components, resources).
    pub world: World,

    systems: Vec<Box<dyn System>>,
    /// Per-system label/order/group metadata. Kept in parallel with `systems` by index.
    system_meta: Vec<crate::ecs::schedule::SystemMeta>,
    /// Execution order computed by `compute_order` (list of indices).
    exec_order: Vec<usize>,
    /// True when `system_meta` has changed — triggers a recompute on the next frame.
    schedule_dirty: bool,
    /// Disabled SystemSet labels. Systems belonging to a disabled set are skipped.
    disabled_sets: std::collections::HashSet<crate::ecs::schedule::SystemLabel>,
    /// Set of system indices disabled by a panic. Those systems are skipped in subsequent frames.
    panicked_systems: std::collections::HashSet<usize>,
    /// Policy for handling cyclic schedule dependencies.
    schedule_error_policy: ScheduleErrorPolicy,
    /// Policy for handling system panics.
    system_panic_policy: SystemPanicPolicy,
    /// (scene, number of systems registered by that scene). Used to restore systems on Push/Pop.
    scene_stack: Vec<(Box<dyn Scene>, usize)>,
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    sprite_renderer: Option<SpriteRenderer>,
    /// Overlays text immediately after the sprite pass. Filled with Some after GPU init.
    text_renderer: Option<TextRenderer>,
    /// Activated when the `PostProcessConfig` resource has `enabled = true`.
    post_renderer: Option<PostProcessRenderer>,
    /// Lighting renderer active while the `AmbientLight` resource is registered.
    #[cfg(not(target_arch = "wasm32"))]
    lighting_renderer: Option<crate::renderer::lighting::LightingRenderer>,
    /// Fade renderer executed as the final pass when `FadeTransition` has `alpha > 0`.
    #[cfg(not(target_arch = "wasm32"))]
    fade_renderer: Option<crate::renderer::fade::FadeRenderer>,
    /// Intermediate texture the lighting pass renders the scene into first.
    #[cfg(not(target_arch = "wasm32"))]
    scene_texture_for_lighting: Option<(
        wgpu::Texture,
        wgpu::TextureView,
        u32,
        u32,
        wgpu::TextureFormat,
    )>,
    /// Intermediate texture that passes the post-process result as input to the lighting pass.
    #[cfg(not(target_arch = "wasm32"))]
    post_texture_for_lighting: Option<(
        wgpu::Texture,
        wgpu::TextureView,
        u32,
        u32,
        wgpu::TextureFormat,
    )>,
    /// GPU compute-shader particle renderer (lazy init).
    #[cfg(not(target_arch = "wasm32"))]
    gpu_particle_renderer: Option<crate::renderer::gpu_particle::GpuParticleRenderer>,
    last_frame: Option<Instant>,
    last_dt: f32,
    /// Texture paths registered before GPU init. Actually loaded in `resumed()`.
    pending_textures: Vec<String>,
    /// Map of registered offscreen render targets (name → RenderTarget).
    render_targets: HashMap<String, crate::renderer::render_target::RenderTarget>,
    /// Render target info registered before GPU init. Actually created in `finish_init()`.
    pending_render_targets: Vec<(String, u32, u32)>,
    /// Closures that drain event queues at the end of each frame.
    event_flushers: Vec<EventHook>,
    /// Closures that re-insert event resources on `reload_scene`.
    event_initializers: Vec<EventHook>,
    /// Resource types to preserve across scene transitions (World reset) via `register_persistent`.
    persistent_resources: Vec<std::any::TypeId>,
    /// gilrs gamepad context. None if initialization failed (runs without gamepad).
    #[cfg(not(target_arch = "wasm32"))]
    gilrs: Option<gilrs::Gilrs>,
    /// egui renderer (wgpu backend).
    egui_renderer: Option<egui_wgpu::Renderer>,
    /// winit ↔ egui event adapter.
    egui_state: Option<egui_winit::State>,
    /// Temporary buffer carrying tessellated output from `update()` to `render()`.
    egui_output: Option<(Vec<egui::ClippedPrimitive>, egui::TexturesDelta, f32)>,
    /// Entity currently selected in the Inspector panel.
    inspector_selected: Option<Entity>,
    /// Multi-selected entity list (includes `inspector_selected`).
    #[cfg(not(target_arch = "wasm32"))]
    selected_entities: Vec<Entity>,
    /// EntityDef clipboard copied via Ctrl+C.
    #[cfg(not(target_arch = "wasm32"))]
    copy_clipboard: Vec<crate::prefab::EntityDef>,
    /// Whether a gizmo drag is in progress.
    gizmo_dragging: bool,
    /// Offset (entity position − cursor world position) captured at drag start.
    gizmo_drag_offset: glam::Vec2,
    /// Inspector scene save path (native only).
    #[cfg(not(target_arch = "wasm32"))]
    editor_save_path: String,
    /// Result message of the last scene save.
    editor_save_status: Option<String>,
    /// Result message of the last scene load.
    #[cfg(not(target_arch = "wasm32"))]
    editor_load_status: Option<String>,
    /// Current Inspector tab index (0: Entities, 1: Assets).
    inspector_tab: u8,
    /// Editor undo/redo history.
    #[cfg(not(target_arch = "wasm32"))]
    cmd_history: EditorHistory,
    /// Entity position at gizmo drag start (for recording undo).
    #[cfg(not(target_arch = "wasm32"))]
    gizmo_drag_start_pos: Option<glam::Vec2>,
    /// Positions of all selected entities at gizmo group-drag start (for group-move undo).
    #[cfg(not(target_arch = "wasm32"))]
    gizmo_drag_start_positions: Vec<(Entity, glam::Vec2)>,
    /// Component add factory map (native only). Type name → closure that adds the component to the World.
    #[cfg(not(target_arch = "wasm32"))]
    component_factories: HashMap<String, ComponentFactory>,
    /// Component remove closure map (native only). Type name → closure that removes the component from the World.
    /// Only components registered in this map expose the "✕" (remove) button in the Inspector.
    #[cfg(not(target_arch = "wasm32"))]
    component_removers: HashMap<String, ComponentFactory>,
    /// Component name currently selected in the "Add Component" dropdown (native only).
    #[cfg(not(target_arch = "wasm32"))]
    add_component_selected: String,
    /// Whether gizmo drag grid snap is enabled (native only).
    #[cfg(not(target_arch = "wasm32"))]
    snap_enabled: bool,
    /// Gizmo drag grid snap cell size in pixels (native only).
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

// ─── winit ApplicationHandler impl ───────────────────────────────────────────

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
    #[should_panic(expected = "system order circular dependency detected")]
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
        // Regression test: a system index disabled by panic in scene A must not
        // incorrectly skip the system at the same index in scene B after Replace.
        struct SceneA;
        impl Scene for SceneA {
            fn on_enter(&mut self, _w: &mut World, systems: &mut Vec<Box<dyn System>>) {
                systems.push(Box::new(PanicSystem)); // idx 0 — disabled after panic
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

        // Scene A: PanicSystem(idx0) panics → disabled, CountSystem(idx1) runs → +1
        app.update(1.0 / 60.0);
        assert_eq!(app.world.resource::<Counter>().unwrap().0, 1);
        assert!(app.panicked_systems.contains(&0));

        // Replace with scene B: panicked_systems is cleared so idx0/idx1 both run → +2
        app.set_scene(Box::new(SceneB));
        app.update(1.0 / 60.0);
        assert_eq!(
            app.world.resource::<Counter>().unwrap().0,
            3,
            "stale panicked index suppressed a new scene's system"
        );
        assert!(app.panicked_systems.is_empty());
    }

    #[test]
    fn builtin_system_labels_compose_for_ordering() {
        // Verify that built-in system LABEL constants enforce ordering in the real scheduler.
        use crate::animation::{AnimationSystem, StateMachineSystem};
        use crate::ecs::schedule::{compute_order, SystemMeta};
        use crate::ui::{LayoutSystem, UiSystem};

        let metas = vec![
            // idx0: StateMachine — after Animation
            SystemMeta {
                label: Some(StateMachineSystem::LABEL),
                after: vec![AnimationSystem::LABEL],
                ..Default::default()
            },
            // idx1: Animation
            SystemMeta {
                label: Some(AnimationSystem::LABEL),
                ..Default::default()
            },
            // idx2: Ui — after Layout
            SystemMeta {
                label: Some(UiSystem::LABEL),
                after: vec![LayoutSystem::LABEL],
                ..Default::default()
            },
            // idx3: Layout
            SystemMeta {
                label: Some(LayoutSystem::LABEL),
                ..Default::default()
            },
        ];
        let order = compute_order(&metas).unwrap();
        let pos = |i: usize| order.iter().position(|&x| x == i).unwrap();
        assert!(
            pos(1) < pos(0),
            "AnimationSystem::LABEL must order before StateMachineSystem::LABEL"
        );
        assert!(
            pos(3) < pos(2),
            "LayoutSystem::LABEL must order before UiSystem::LABEL"
        );
    }
}
