use std::collections::HashMap;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

mod assets;
mod core_resources;
mod editor;
mod egui_pass;
mod render;
mod render_state;
mod scenes;
mod schedule;
mod window;

pub use schedule::{ScheduleErrorPolicy, SystemPanicPolicy};

use editor::EditorState;
#[cfg(test)]
use egui_pass::paint_jobs_contain_callbacks;
use render_state::RenderState;

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
    renderer::{GpuContext, SpriteRenderer, TextRenderer},
    resources::{DisplayScaleFactor, FontData, LoadProgress, ViewportSize, WindowConfig},
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
    wgpu::TextureView,
    Arc<wgpu::BindGroup>,
    u32,              // layer_mask
    Option<[f64; 4]>, // clear_color
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
    render: RenderState,
    last_frame: Option<Instant>,
    last_dt: f32,
    /// Next scheduled frame time. The native event loop uses `ControlFlow::WaitUntil`
    /// to sleep until this instant, requesting a redraw when it is due. This yields the
    /// macOS main run loop idle time between frames (smooth window drag + prompt input)
    /// instead of busy-spinning under `ControlFlow::Poll`. wasm is unaffected (rAF-driven).
    #[cfg(not(target_arch = "wasm32"))]
    next_frame: Option<Instant>,
    /// Target interval between rendered frames, derived from the monitor refresh rate at
    /// init (fallback 60 Hz). Frames are still vsync-paced by `AutoVsync`; this only bounds
    /// the redraw-request cadence so the loop doesn't spin.
    #[cfg(not(target_arch = "wasm32"))]
    frame_interval: Duration,
    /// Texture paths registered before GPU init. Actually loaded in `resumed()`.
    pending_textures: Vec<String>,
    /// Render target info registered before GPU init. Actually created in `finish_init()`.
    pending_render_targets: Vec<(String, u32, u32)>,
    /// Closures that drain event queues at the end of each frame.
    event_flushers: Vec<EventHook>,
    /// Closures that re-insert event resources on `reload_scene`.
    event_initializers: Vec<EventHook>,
    /// Closures that re-apply game-side world registrations (reflect, clone, serde components)
    /// after a scene Replace resets the World. Scene Replace calls `World::new()`, which
    /// discards all per-type metadata stored on the old world; replaying these thunks
    /// restores `register_editable_component` / `register_serde_component` registrations.
    world_registrars: Vec<EventHook>,
    /// Resource types to preserve across scene transitions (World reset) via `register_persistent`.
    persistent_resources: Vec<std::any::TypeId>,
    /// gilrs gamepad context. None if initialization failed (runs without gamepad).
    #[cfg(not(target_arch = "wasm32"))]
    gilrs: Option<gilrs::Gilrs>,
    /// Guard flag: set to `true` the first time `step_frame` runs in a given winit
    /// event-loop iteration. Reset in `about_to_wait` at the start of each new
    /// iteration. Prevents both `Resized` and `RedrawRequested` from calling
    /// `step_frame` in the same iteration (double-stepping physics/tween/timer).
    stepped_this_iteration: bool,
    /// All editor/inspector-only state grouped for clean fork extraction.
    editor: EditorState,
    /// Registered hot-reload forwarders. Each entry is a monomorphized fn that calls
    /// `HotReloadable::reload_path` on one resource type. Populated by
    /// [`App::register_hot_reloadable`] and the three built-in auto-registrations in
    /// `App::new`. Native-only — hot reload is not supported on wasm.
    #[cfg(not(target_arch = "wasm32"))]
    hot_reload_forwarders: Vec<fn(&mut crate::ecs::World, &[String])>,
}

impl App {
    fn insert_core_resources(world: &mut World) {
        core_resources::insert_core_resources(world);
    }

    fn register_core_component_metadata(world: &mut World) {
        core_resources::register_core_component_metadata(world);
    }

    pub fn new() -> Self {
        // gilrs (IOKit-HID) can't read modern Xbox/PS5 pads on macOS — Apple's GameController
        // framework claims them. macOS uses the GameController backend (`input::gamepad_macos`,
        // polled in `about_to_wait`) instead, so gilrs is left uninitialized there.
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos")))]
        let gilrs = gilrs::Gilrs::new().ok();
        #[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
        let gilrs: Option<gilrs::Gilrs> = None;
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
            render: RenderState::new(),
            last_frame: None,
            last_dt: 1.0 / 60.0,
            #[cfg(not(target_arch = "wasm32"))]
            next_frame: None,
            #[cfg(not(target_arch = "wasm32"))]
            frame_interval: Duration::from_secs_f64(1.0 / 60.0),
            pending_textures: Vec::new(),
            pending_render_targets: Vec::new(),
            event_flushers: Vec::new(),
            event_initializers: Vec::new(),
            world_registrars: Vec::new(),
            persistent_resources: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            gilrs,
            stepped_this_iteration: false,
            editor: editor_state,
            #[cfg(not(target_arch = "wasm32"))]
            hot_reload_forwarders: Vec::new(),
        };
        #[cfg(not(target_arch = "wasm32"))]
        app.register_default_components();

        // Auto-register the four built-in hot-reloadable registries.
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.register_hot_reloadable::<crate::scripting::ScriptRegistry>();
            app.register_hot_reloadable::<crate::data_table::DataTableRegistry>();
            app.register_hot_reloadable::<crate::animation::clip_set::AnimationClipRegistry>();
            app.register_hot_reloadable::<crate::particle::ParticleConfigRegistry>();
            app.register_hot_reloadable::<crate::dialogue::DialogueRegistry>();
        }

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

    /// Register a resource type as a hot-reload target (native only).
    ///
    /// After registration, the engine calls `T::reload_path` on the resource every frame
    /// for each changed asset path detected by the file watcher.
    ///
    /// # Requirements
    /// - `T` must implement [`crate::asset::HotReloadable`].
    /// - `T` must already be inserted as a resource in `self.world` before hot-reload ticks
    ///   fire (typically done right after `App::new()`).
    ///
    /// # Example
    /// ```rust,no_run
    /// # use engine::{App, asset::HotReloadable};
    /// struct MyRegistry;
    /// impl HotReloadable for MyRegistry {
    ///     fn reload_path(&mut self, path: &str) { /* ... */ }
    /// }
    ///
    /// let mut app = App::new();
    /// // app.world.insert_resource(MyRegistry);
    /// app.register_hot_reloadable::<MyRegistry>();
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn register_hot_reloadable<T: crate::asset::HotReloadable>(&mut self) {
        self.hot_reload_forwarders.push(forward_hot_reload::<T>);
    }

    /// Register a custom render pass invoked once per frame after the main sprite
    /// pass and before post-processing/lighting. See [`RenderPlugin`](crate::RenderPlugin).
    /// Plugins run in registration order. Available on native and wasm.
    pub fn add_render_plugin(
        &mut self,
        plugin: impl crate::renderer::RenderPlugin + 'static,
    ) -> &mut Self {
        self.render.render_plugins.push(Box::new(plugin));
        self
    }
}

/// Monomorphized hot-reload forwarder: extracts resource `T` from the world and forwards
/// each changed path to it via the `HotReloadable` trait. Stored as a function pointer so
/// `App` does not need to carry a `Box<dyn Fn>` per registry.
#[cfg(not(target_arch = "wasm32"))]
fn forward_hot_reload<T: crate::asset::HotReloadable>(
    world: &mut crate::ecs::World,
    paths: &[String],
) {
    if let Some(reg) = world.resource_mut::<T>() {
        for p in paths {
            reg.reload_path(p);
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

        // Frame 1: PanicSystem panics → remaining systems aborted this frame (counter stays 0).
        // Frame 2: PanicSystem disabled, CountSystem runs → counter = 1.
        // Frame 3: same → counter = 2.
        app.update(1.0 / 60.0);
        app.update(1.0 / 60.0);
        app.update(1.0 / 60.0);

        assert_eq!(app.world.resource::<Counter>().unwrap().0, 2);
        assert_eq!(app.panicked_systems.len(), 1);
    }

    /// When a system panics, subsequent systems in the SAME frame must not run
    /// (the World may be half-mutated). On the NEXT frame they resume normally.
    #[test]
    fn panic_aborts_remaining_systems_this_frame() {
        let mut app = app_with_counter();
        app.add_system(PanicSystem);
        app.add_system(CountSystem);

        // Frame 1: PanicSystem panics → CountSystem must NOT run this frame.
        app.update(1.0 / 60.0);
        assert_eq!(
            app.world.resource::<Counter>().unwrap().0,
            0,
            "CountSystem must not run in the same frame as a panic"
        );

        // Frame 2: PanicSystem is disabled, CountSystem runs normally.
        app.update(1.0 / 60.0);
        assert_eq!(
            app.world.resource::<Counter>().unwrap().0,
            1,
            "CountSystem must run on subsequent frames after panic"
        );
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

        // Scene A: PanicSystem(idx0) panics → disabled AND the frame aborts, so
        // CountSystem(idx1) does NOT run this frame → Counter stays 0.
        app.update(1.0 / 60.0);
        assert_eq!(app.world.resource::<Counter>().unwrap().0, 0);
        assert!(app.panicked_systems.contains(&0));

        // Replace with scene B: panicked_systems is cleared so idx0/idx1 both run → +2
        app.set_scene(Box::new(SceneB));
        app.update(1.0 / 60.0);
        assert_eq!(
            app.world.resource::<Counter>().unwrap().0,
            2,
            "stale panicked index suppressed a new scene's system"
        );
        assert!(app.panicked_systems.is_empty());
    }

    /// Regression test: after `SceneCmd::Pop`, a panicked index from the popped scene
    /// that equals `new_scene_len` must NOT alias the builtin tail (`HierarchySystem`).
    ///
    /// Setup:
    ///   - SceneA registers 0 systems (so new_scene_len = 0 after Pop).
    ///   - SceneB is Pushed on top with 1 system (PanicSystem at index 0).
    ///   - PanicSystem panics → index 0 added to panicked_systems.
    ///   - SceneB is Popped: the drain removes index 0..1.
    ///   - Bug (before fix): retain `i < new_len` (= 0 + tail = 1) kept index 0,
    ///     which now aliases HierarchySystem → HierarchySystem permanently skipped.
    ///   - Fix: retain `i < new_scene_len` (= 0) → index 0 is dropped.
    ///
    /// Because panic injection via `std::panic::catch_unwind` in an App update works
    /// the same way as in other panic-recovery tests, we use that real path.
    /// After Pop we assert that panicked_systems does NOT contain the tail index
    /// (the HierarchySystem index = new_scene_len = 0 in this configuration).
    #[test]
    fn scene_pop_does_not_alias_builtin_tail_panicked_index() {
        struct SceneA;
        impl Scene for SceneA {
            fn on_enter(&mut self, _w: &mut World, _systems: &mut crate::scene::SystemRegistrar) {
                // no systems — new_scene_len will be 0 after Pop back to SceneA
            }
            fn on_exit(&mut self, _w: &mut World) {}
        }
        struct SceneB;
        impl Scene for SceneB {
            fn on_enter(&mut self, _w: &mut World, systems: &mut crate::scene::SystemRegistrar) {
                // PanicSystem inserted at index 0 (before the tail HierarchySystem).
                // After panic, panicked_systems = {0}.
                systems.add(PanicSystem);
            }
            fn on_exit(&mut self, _w: &mut World) {}
        }

        let mut app = App::new();
        // Push SceneA first (0 user systems; tail = HierarchySystem at idx 0).
        app.apply_scene_cmd(crate::scene::SceneCmd::Replace(Box::new(SceneA)));
        // Push SceneB (PanicSystem at idx 0; HierarchySystem now at idx 1).
        app.apply_scene_cmd(crate::scene::SceneCmd::Push(Box::new(SceneB)));

        // Run a frame: PanicSystem panics → added to panicked_systems (index 0).
        app.update(1.0 / 60.0);
        assert!(
            app.panicked_systems.contains(&0),
            "PanicSystem at idx 0 should be in panicked_systems"
        );

        // Pop SceneB: drain removes PanicSystem (index 0..1).
        // After drain, HierarchySystem slides back to index 0 (= new_scene_len = 0).
        // Bug: retain `i < new_len` (0 + 1 = 1) would keep panicked index 0,
        //      which now aliases HierarchySystem → it would be permanently skipped.
        // Fix: retain `i < new_scene_len` (= 0) → panicked_systems becomes empty.
        app.apply_scene_cmd(crate::scene::SceneCmd::Pop);

        // The tail index (HierarchySystem = index new_scene_len = 0) must NOT be in panicked_systems.
        let hierarchy_tail_idx = app.systems.len().saturating_sub(app.builtin_tail_count);
        assert!(
            !app.panicked_systems.contains(&hierarchy_tail_idx),
            "HierarchySystem's index {hierarchy_tail_idx} must not be in panicked_systems after Pop; \
             got panicked_systems = {:?}",
            app.panicked_systems
        );
        assert!(
            app.panicked_systems.is_empty(),
            "panicked_systems should be empty after Pop clears the stale index; \
             got: {:?}",
            app.panicked_systems
        );
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

    // ─── register_editable_component / load_data_table scene-reset regression tests ──

    /// A component registered via `register_editable_component` must still be inspectable
    /// and serialisable after a `set_scene` (which triggers a World reset via SceneCmd::Replace).
    ///
    /// This test reproduces the exact usage pattern from `stat_editor_game` that was silently
    /// broken: register → set_scene → assert the registration survived the World swap.
    ///
    /// We implement `Reflect` manually rather than via `#[derive(engine_reflect_derive::Reflect)]`
    /// because the derive macro emits `engine::reflect::…` paths — valid in integration tests
    /// (where `engine` is the extern crate name) but NOT inside the `skeleton-engine` lib itself.
    #[test]
    fn editable_component_survives_scene_replace() {
        #[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
        struct TestStat {
            hp: f32,
        }
        impl crate::reflect::Reflect for TestStat {
            fn fields(&self) -> Vec<(&'static str, crate::reflect::ReflectValue)> {
                vec![("hp", crate::reflect::ReflectValue::F32(self.hp))]
            }
            fn set_field(&mut self, name: &str, val: crate::reflect::ReflectValue) -> bool {
                if name == "hp" {
                    if let crate::reflect::ReflectValue::F32(v) = val {
                        self.hp = v;
                        return true;
                    }
                }
                false
            }
            fn type_name(&self) -> &'static str {
                "TestStat"
            }
        }

        use crate::scene::SystemRegistrar;

        struct EmptyScene;
        impl Scene for EmptyScene {
            fn on_enter(&mut self, _w: &mut World, _s: &mut SystemRegistrar) {}
            fn on_exit(&mut self, _w: &mut World) {}
        }

        let mut app = App::new();
        // Register BEFORE set_scene — this is the pattern that was broken.
        app.register_editable_component::<TestStat>("TestStat", None);
        app.set_scene(Box::new(EmptyScene));

        // Spawn an entity and add the component.
        let e = app.world.spawn();
        app.world.add_component(e, TestStat { hp: 42.0 });

        // (a) Reflect: the Inspector must be able to retrieve it.
        let type_id = std::any::TypeId::of::<TestStat>();
        let reflected = app.world.reflected_components(e);
        assert!(
            reflected.contains(&type_id),
            "TestStat must appear in reflected_components after scene Replace"
        );

        // (b) Serde: SerdeComponentRegistry must round-trip it.
        let registry = app
            .world
            .resource::<crate::prefab::SerdeComponentRegistry>()
            .expect("SerdeComponentRegistry must exist after scene Replace");
        let serialized = registry.serialize_entity(&app.world, e);
        assert!(
            serialized.contains_key("TestStat"),
            "TestStat must be serialisable after scene Replace; got keys: {:?}",
            serialized.keys().collect::<Vec<_>>()
        );
    }

    /// `load_data_table` must preserve the `DataTableRegistry` across a scene Replace so
    /// that the Data Tables editor panel retains its loaded data.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn data_table_registry_survives_scene_replace() {
        use crate::scene::SystemRegistrar;

        struct EmptyScene;
        impl Scene for EmptyScene {
            fn on_enter(&mut self, _w: &mut World, _s: &mut SystemRegistrar) {}
            fn on_exit(&mut self, _w: &mut World) {}
        }

        // Write a minimal valid DataTable RON to a temp file.
        let dir = std::env::temp_dir();
        let path = dir.join("test_enemies_table.ron");
        std::fs::write(&path, r#"[(name: "goblin", hp: 10)]"#).expect("write temp ron");

        let mut app = App::new();
        app.load_data_table("enemies", path.to_str().unwrap());

        // Verify the table is present before the scene reset.
        assert!(
            app.world
                .resource::<crate::data_table::DataTableRegistry>()
                .is_some(),
            "DataTableRegistry must exist after load_data_table"
        );

        app.set_scene(Box::new(EmptyScene));

        // After scene Replace the registry must still be there (preserved resource).
        let reg = app
            .world
            .resource::<crate::data_table::DataTableRegistry>()
            .expect("DataTableRegistry must survive scene Replace");
        assert!(
            reg.get("enemies").is_some(),
            "enemies table must survive scene Replace; tables present: {:?}",
            reg.names()
        );

        let _ = std::fs::remove_file(&path);
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

    #[test]
    fn render_plugins_starts_empty() {
        let app = App::new();
        assert_eq!(app.render.render_plugins.len(), 0);
    }

    #[test]
    fn add_render_plugin_preserves_order() {
        #[allow(dead_code)]
        struct DummyPlugin(u32);
        impl crate::renderer::RenderPlugin for DummyPlugin {
            fn record(
                &mut self,
                _ctx: &mut crate::renderer::FrameContext<'_>,
                _world: &crate::ecs::World,
                _viewport: (u32, u32),
            ) {
            }
        }

        let mut app = App::new();
        app.add_render_plugin(DummyPlugin(1));
        app.add_render_plugin(DummyPlugin(2));
        assert_eq!(app.render.render_plugins.len(), 2);
    }

    // ── HotReloadable / register_hot_reloadable tests (native only) ───────────

    #[cfg(not(target_arch = "wasm32"))]
    mod hot_reload_tests {
        use super::super::{forward_hot_reload, App};
        use crate::asset::HotReloadable;
        use crate::ecs::World;

        /// A simple recorder that collects every path forwarded to it.
        #[derive(Default)]
        struct Recorder(Vec<String>);

        impl HotReloadable for Recorder {
            fn reload_path(&mut self, path: &str) {
                self.0.push(path.to_owned());
            }
        }

        /// `forward_hot_reload` calls `HotReloadable::reload_path` on the resource.
        #[test]
        fn forward_hot_reload_drives_recorder() {
            let mut world = World::new();
            world.insert_resource(Recorder::default());

            let paths = vec!["assets/foo.ron".to_owned(), "assets/bar.ron".to_owned()];
            forward_hot_reload::<Recorder>(&mut world, &paths);

            let rec = world.resource::<Recorder>().unwrap();
            assert_eq!(rec.0, paths, "recorder should have received both paths");
        }

        /// `register_hot_reloadable` pushes exactly one forwarder per call.
        #[test]
        fn register_hot_reloadable_pushes_one_forwarder() {
            let mut app = App::new();
            let count_before = app.hot_reload_forwarders.len();
            app.register_hot_reloadable::<Recorder>();
            assert_eq!(
                app.hot_reload_forwarders.len(),
                count_before + 1,
                "one forwarder should have been added"
            );
        }

        /// `register_hot_reloadable` + `forward_hot_reload` round-trip through App.
        #[test]
        fn round_trip_recorder_via_app() {
            let mut app = App::new();
            app.world.insert_resource(Recorder::default());
            app.register_hot_reloadable::<Recorder>();

            let paths = vec!["my/asset.ron".to_owned()];
            // Drive via the forwarder fn directly (avoids needing a live window/GPU).
            let f = *app.hot_reload_forwarders.last().unwrap();
            f(&mut app.world, &paths);

            let rec = app.world.resource::<Recorder>().unwrap();
            assert_eq!(rec.0, paths);
        }

        /// Verifies the `DataTableRegistry` HotReloadable impl does NOT stack-overflow
        /// (UFCS delegation is correct — trait method calls the inherent, not itself).
        #[test]
        fn data_table_registry_no_infinite_recursion() {
            use crate::data_table::DataTableRegistry;
            let mut reg = DataTableRegistry::default();
            // Call the trait method with a path that won't match any table → no-op, no crash.
            HotReloadable::reload_path(&mut reg, "nonexistent/path.ron");
        }
    }
}
