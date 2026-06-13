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

use editor::EditorState;
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
        DebugDraw, DisplayScaleFactor, FontData, LoadProgress, PendingResize, ShouldQuit,
        ViewportSize, WindowConfig,
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

// WASM: the logical (CSS) canvas size, captured from the authored `<canvas>` width/height
// attributes in `finish_init`. The drawing buffer is this × devicePixelRatio (uniform, capped so
// neither axis exceeds WebGL2's 2048 limit) for a crisp Retina render, while the CSS display box
// stays at the logical size. The per-frame viewport math (`schedule.rs`) divides the buffer by
// `buffer / logical` to recover the logical viewport. Stored here rather than read from
// `WindowConfig` because a scene transition (`World` reset) can revert `WindowConfig` to its
// default, whereas the canvas attributes are stable.
#[cfg(target_arch = "wasm32")]
thread_local! {
    pub(crate) static WASM_LOGICAL_SIZE: std::cell::Cell<(u32, u32)> =
        const { std::cell::Cell::new((0, 0)) };
}

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
    system_meta: Vec<crate::ecs::schedule::SystemConfig>,
    /// Number of systems at the **tail** of `systems` that are permanent engine built-ins.
    ///
    /// These systems are registered by `App::new()` and survive scene transitions
    /// (`SceneCmd::Replace`). Scene systems occupy `systems[..systems.len() - builtin_tail_count]`;
    /// built-ins occupy `systems[systems.len() - builtin_tail_count..]`.
    ///
    /// Currently always 1: `HierarchySystem` is the only engine-forced built-in.
    builtin_tail_count: usize,
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
    /// This field exists on native targets only. On wasm, `FadeTransition` is accepted
    /// without error but the fade effect is silently skipped (no render pass is issued).
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
    /// Offscreen texture the docked editor renders the game scene into.
    /// (width, height, format, texture, view) — recreated when the central panel resizes.
    #[cfg(not(target_arch = "wasm32"))]
    docked_scene_texture: Option<(
        u32,
        u32,
        wgpu::TextureFormat,
        wgpu::Texture,
        wgpu::TextureView,
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
    /// All editor/inspector-only state grouped for clean fork extraction.
    editor: EditorState,
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

        // EditorState::new() (native) registers default component removers; wasm only
        // has Default (no native-only fields).
        #[cfg(not(target_arch = "wasm32"))]
        let editor_state = EditorState::new();
        #[cfg(target_arch = "wasm32")]
        let editor_state = EditorState::default();

        // Single struct literal covering both targets.  Fields that exist only on
        // native (lighting/fade/gilrs/…) are gated with #[cfg]; the cfg on the struct
        // field definition ensures the field is absent on wasm, so this compiles on
        // both targets without duplication.
        // `mut` is needed on native for register_default_components(); wasm doesn't use it.
        #[allow(unused_mut)]
        let mut app = Self {
            world,
            systems: Vec::new(),
            system_meta: Vec::new(),
            builtin_tail_count: 0,
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
            #[cfg(not(target_arch = "wasm32"))]
            lighting_renderer: None,
            #[cfg(not(target_arch = "wasm32"))]
            fade_renderer: None,
            #[cfg(not(target_arch = "wasm32"))]
            scene_texture_for_lighting: None,
            #[cfg(not(target_arch = "wasm32"))]
            post_texture_for_lighting: None,
            #[cfg(not(target_arch = "wasm32"))]
            gpu_particle_renderer: None,
            #[cfg(not(target_arch = "wasm32"))]
            docked_scene_texture: None,
            last_frame: None,
            last_dt: 1.0 / 60.0,
            pending_textures: Vec::new(),
            render_targets: HashMap::new(),
            pending_render_targets: Vec::new(),
            event_flushers: Vec::new(),
            event_initializers: Vec::new(),
            persistent_resources: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            gilrs,
            egui_renderer: None,
            egui_state: None,
            egui_output: None,
            editor: editor_state,
        };
        #[cfg(not(target_arch = "wasm32"))]
        app.register_default_components();

        // Register HierarchySystem as the one permanent tail built-in.
        //
        // The tail design: built-in systems live at the *end* of `self.systems`.
        // `builtin_tail_count` tells the engine how many trailing entries are permanent
        // (i.e. survive `SceneCmd::Replace`).  `add_system` / `add_system_labeled` and
        // `SystemRegistrar` all insert *before* the tail so the tail indices stay highest.
        // Kahn's algorithm breaks ties by lowest index, which means unconstrained user
        // systems (smaller indices) run before HierarchySystem (largest index) by default.
        // A user system that explicitly declares `.after(HierarchySystem::LABEL)` is placed
        // after it by the topological sort, so it sees the freshly-propagated GlobalTransforms.
        app.systems.push(Box::new(HierarchySystem));
        app.system_meta
            .push(crate::ecs::schedule::SystemConfig::new().label(HierarchySystem::LABEL));
        app.builtin_tail_count = 1;
        app.schedule_dirty = true;

        app
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
            fn on_enter(&mut self, _w: &mut World, systems: &mut crate::scene::SystemRegistrar) {
                systems.add(PanicSystem); // idx 0 — disabled after panic
                systems.add(CountSystem); // idx 1
            }
            fn on_exit(&mut self, _w: &mut World) {}
        }
        struct SceneB;
        impl Scene for SceneB {
            fn on_enter(&mut self, _w: &mut World, systems: &mut crate::scene::SystemRegistrar) {
                systems.add(CountSystem); // idx 0
                systems.add(CountSystem); // idx 1
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
        use crate::ecs::schedule::{compute_order, SystemConfig};
        use crate::ui::{LayoutSystem, UiSystem};

        let metas = vec![
            // idx0: StateMachine — after Animation
            SystemConfig {
                label: Some(StateMachineSystem::LABEL),
                after: vec![AnimationSystem::LABEL],
                ..Default::default()
            },
            // idx1: Animation
            SystemConfig {
                label: Some(AnimationSystem::LABEL),
                ..Default::default()
            },
            // idx2: Ui — after Layout
            SystemConfig {
                label: Some(UiSystem::LABEL),
                after: vec![LayoutSystem::LABEL],
                ..Default::default()
            },
            // idx3: Layout
            SystemConfig {
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

    // ─── HierarchySystem pipeline-integration tests ───────────────────────────

    /// A user system that mutates `Transform` then `App::update` must see the updated
    /// `GlobalTransform` in the same frame — HierarchySystem propagates after user
    /// systems when running inside the labeled pipeline.
    #[test]
    fn hierarchy_propagates_gt_after_user_transform_mutation() {
        use crate::components::Transform;
        use crate::hierarchy::GlobalTransform;
        use glam::Vec2;

        struct MoveParent(Entity);
        impl System for MoveParent {
            fn run(&mut self, world: &mut World, _dt: f32) {
                if let Some(t) = world.get_mut::<Transform>(self.0) {
                    t.position.x += 1.0;
                }
            }
        }

        let mut app = app_with_counter(); // borrows counter resource but that's fine
        let parent = app.world.spawn();
        let child = app.world.spawn();
        app.world.add_component(
            parent,
            Transform {
                position: Vec2::ZERO,
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 0.0,
            },
        );
        app.world.add_component(
            child,
            Transform {
                position: Vec2::new(5.0, 0.0),
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 0.0,
            },
        );
        crate::hierarchy::attach(&mut app.world, child, parent);
        app.add_system(MoveParent(parent));

        // Frame 1: MoveParent shifts parent to x=1; HierarchySystem propagates → child GT = 6
        app.update(1.0 / 60.0);
        let x1 = app.world.get::<GlobalTransform>(child).unwrap().position.x;
        assert!(
            (x1 - 6.0).abs() < 1e-3,
            "frame 1: child GT.x expected 6.0, got {x1}"
        );

        // Frame 2: parent at x=2 → child GT = 7
        app.update(1.0 / 60.0);
        let x2 = app.world.get::<GlobalTransform>(child).unwrap().position.x;
        assert!(
            (x2 - 7.0).abs() < 1e-3,
            "frame 2: child GT.x expected 7.0, got {x2}"
        );
    }

    /// A system declared `.after(HierarchySystem::LABEL)` must see `GlobalTransform`
    /// values that reflect `Transform` mutations from the same frame.
    #[test]
    fn after_hierarchy_label_sees_fresh_global_transform() {
        use crate::components::Transform;
        use crate::hierarchy::{GlobalTransform, HierarchySystem};
        use glam::Vec2;
        use std::sync::{Arc, Mutex};

        struct SetParentX {
            parent: Entity,
        }
        impl System for SetParentX {
            fn run(&mut self, world: &mut World, _dt: f32) {
                if let Some(t) = world.get_mut::<Transform>(self.parent) {
                    t.position.x = 10.0;
                }
            }
        }

        struct ReadChildGt {
            child: Entity,
            sink: Arc<Mutex<f32>>,
        }
        impl System for ReadChildGt {
            fn run(&mut self, world: &mut World, _dt: f32) {
                if let Some(gt) = world.get::<GlobalTransform>(self.child) {
                    *self.sink.lock().unwrap() = gt.position.x;
                }
            }
        }

        let sink: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
        let mut app = App::new();
        let parent = app.world.spawn();
        let child = app.world.spawn();
        app.world.add_component(
            parent,
            Transform {
                position: Vec2::ZERO,
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 0.0,
            },
        );
        app.world.add_component(
            child,
            Transform {
                position: Vec2::new(3.0, 0.0),
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 0.0,
            },
        );
        crate::hierarchy::attach(&mut app.world, child, parent);

        // SetParentX has no ordering constraint — runs before HierarchySystem by default.
        app.add_system(SetParentX { parent });
        // ReadChildGt explicitly runs after HierarchySystem: sees GT propagated this frame.
        app.add_system_labeled(
            ReadChildGt {
                child,
                sink: sink.clone(),
            },
            SystemConfig::new().after(HierarchySystem::LABEL),
        );

        app.update(1.0 / 60.0);

        // parent.x = 10, child local.x = 3 → child GT.x = 13
        let recorded = *sink.lock().unwrap();
        assert!(
            (recorded - 13.0).abs() < 1e-3,
            "ReadChildGt (after HierarchySystem::LABEL) expected GT.x=13.0, got {recorded}"
        );
    }

    /// After `SceneCmd::Replace`, `HierarchySystem` must still propagate `GlobalTransform` —
    /// it is a permanent built-in that survives scene transitions.
    #[test]
    fn hierarchy_survives_scene_replace() {
        use crate::components::Transform;
        use crate::hierarchy::GlobalTransform;
        use crate::scene::SystemRegistrar;
        use glam::Vec2;

        struct EmptyScene;
        impl Scene for EmptyScene {
            fn on_enter(&mut self, _w: &mut World, _s: &mut SystemRegistrar) {}
            fn on_exit(&mut self, _w: &mut World) {}
        }

        let mut app = App::new();
        app.set_scene(Box::new(EmptyScene));

        let parent = app.world.spawn();
        let child = app.world.spawn();
        app.world.add_component(
            parent,
            Transform {
                position: Vec2::new(20.0, 0.0),
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 0.0,
            },
        );
        app.world.add_component(
            child,
            Transform {
                position: Vec2::new(5.0, 0.0),
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 0.0,
            },
        );
        crate::hierarchy::attach(&mut app.world, child, parent);

        app.update(1.0 / 60.0);

        let x = app.world.get::<GlobalTransform>(child).unwrap().position.x;
        assert!(
            (x - 25.0).abs() < 1e-3,
            "after scene replace, child GT.x expected 25.0, got {x}"
        );
    }
}
