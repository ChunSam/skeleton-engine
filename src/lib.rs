pub mod anim_effect;
pub mod animation;
pub mod app;
pub mod asset;
pub mod atlas;
#[cfg(not(target_arch = "wasm32"))]
pub mod audio;
/// Cross-platform audio facade (`Audio`) over the native `AudioManager` / wasm `WebAudio` backends.
pub mod audio_facade;
pub(crate) mod audio_spatial;
#[cfg(target_arch = "wasm32")]
pub mod audio_wasm;
pub mod behavior;
pub mod camera;
pub mod collision;
pub mod color;
pub mod components;
pub mod coroutine;
pub mod data_table;
pub mod debug_ui;
pub mod dialogue;
pub mod ecs;
pub mod effect;
pub mod floating_text;
#[cfg(not(target_arch = "wasm32"))]
pub mod gpu_particle;
pub mod hierarchy;
pub mod history;
pub mod hit_flash;
pub mod input;
pub mod locale;
pub mod material;
pub mod network;
pub mod nine_slice;
pub mod parallax;
pub mod particle;
pub mod pathfinding;
#[cfg(not(target_arch = "wasm32"))]
pub mod physics;
pub mod pool;
pub mod prefab;
pub mod reflect;
pub mod renderer;
pub mod resources;
/// Generic RON-file registry backing the particle/dialogue/animation-clip registries — also a
/// fork-friendly extension point for a game's own RON-loaded config type (see [`RonRegistry`]).
mod ron_registry;
pub mod save;
pub mod serde_registry;
pub mod trigger_zone;
pub mod ysort;
pub mod zone_effect;
pub use save::{
    delete, exists, load, load_migrated, load_or_default, load_with_key, save, save_path,
    save_versioned, save_with_key, SaveError, SaveKey, SaveMigrator,
};
pub mod scene;
pub mod scripting;
pub mod skeletal;
pub mod steering;
pub mod tilemap;
pub mod timeline;
pub mod timer;
pub mod tween;
pub mod ui;

// ── Convenience re-exports ─────────────────────────────────────────────────────

/// The `wgpu` crate, re-exported so a game can name GPU types (e.g.
/// [`wgpu::TextureFormat`]) without declaring its own `wgpu` dependency.
///
/// These types already appear in the engine's public API — e.g.
/// [`App::create_render_target_with_format`] and [`App::load_image_with_format`] both take a
/// `wgpu::TextureFormat` — so a game must already match the engine's `wgpu` version. This
/// re-export just removes the need for a redundant direct dependency. (Because `wgpu` is part
/// of the public surface, a `wgpu` major version bump is a breaking change for such games.)
pub use wgpu;

pub use glam::{IVec2, Mat4, Vec2, Vec3};
pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

pub use animation::{
    AnimParam, AnimState, AnimTransition, AnimationClip, AnimationClipRegistry, AnimationClipSet,
    AnimationEvent, AnimationEvents, AnimationPlayer, AnimationStateMachine, AnimationSystem,
    BlendEntry, BlendTree1D, BlendTreeSystem, BlendWeight, ClipSetError, FrameEvent,
    StateMachineSystem, TransitionCond,
};
pub use app::{App, ScheduleErrorPolicy, SystemPanicPolicy};
#[cfg(not(target_arch = "wasm32"))]
pub use asset::HotReloadable;
pub use asset::{
    AssetId, AssetLoadError, AssetLoadState, AssetServer, Handle, ImageAsset, ImageEntry,
};
// ScriptAsset re-exported from scripting (canonical location after K-refactor).
// Also bridged below via `pub use scripting::ScriptAsset` for source-compat.
pub use anim_effect::{AnimEffectBindings, AnimEffectError, AnimEffectRegistry, AnimEffectSystem};
pub use atlas::{AtlasSprite, TextureAtlas};
#[cfg(not(target_arch = "wasm32"))]
pub use audio::{AudioChannelState, AudioEffect, AudioManager, AudioSystem, BusDuck, Sidechain};
pub use audio_facade::{Audio, AudioFacadeSystem};
#[cfg(target_arch = "wasm32")]
pub use audio_wasm::{Sfx, WebAudio};
pub use behavior::{
    AlwaysSucceed, BehaviorNode, BehaviorStatus, BehaviorSystem, BehaviorTree, Blackboard,
    BlackboardValue, Inverter, Selector, Sequence,
};
pub use camera::{Camera, DEFAULT_LOOKAHEAD_SPEED};
pub use collision::{
    Collider, CollisionDebugSystem, CollisionGridSystem, CollisionLayer, DebugConfig, SpatialGrid,
};
pub use color::Color;
pub use components::{
    Hidden, OffscreenCamera, PointLight, RenderLayer, Sprite, SpriteFlip, Transform,
};
pub use coroutine::{Coroutine, CoroutineRunner, CoroutineSystem};
pub use data_table::{DataTable, DataTableRegistry};
pub use debug_ui::DebugUi;
pub use dialogue::{
    DialogueBox, DialogueChoice, DialogueCond, DialogueEffect, DialogueEvent, DialogueOp,
    DialogueRegistry, DialogueStyle, DialogueSystem, DialogueTree, DialogueValue, DialogueVars,
};
pub use ecs::schedule::{ScheduleError, SystemConfig, SystemLabel};
pub use ecs::{Commands, Entity, Events, System, World};
pub use effect::{Effect, EffectAnchor};
pub use floating_text::{
    spawn_floating_text, FloatingText, FloatingTextSystem, DEFAULT_FLOAT_LIFETIME,
    DEFAULT_FLOAT_SIZE, DEFAULT_FLOAT_SPEED,
};
#[cfg(not(target_arch = "wasm32"))]
pub use gpu_particle::{GpuParticleConfig, GpuParticleEmitter};
pub use hierarchy::{
    attach, detach, reparent, topological_sort_entities, Children, GlobalTransform,
    HierarchySystem, Parent,
};
pub use history::History;
pub use hit_flash::{HitFlash, HitFlashSystem};
pub use input::AxisBinding;
pub use input::{GamepadAxis, GamepadButton, GamepadState, InputMap, InputState, TouchState};
pub use locale::{LocaleBundle, LocaleData, LocaleResource, TextDirection};
pub use material::ShaderMaterial;
pub use network::{
    NetworkClient, NetworkConfig, NetworkEvent, NetworkSystem, RemoteEntities, SnapshotBuffer,
};
pub use nine_slice::NineSlice;
pub use parallax::{ParallaxLayer, ParallaxSystem};
pub use particle::{
    EmitShape, Particle, ParticleBurst, ParticleConfigError, ParticleConfigRegistry,
    ParticleConfigSet, ParticleEmitter, ParticleSystem, DEFAULT_MAX_PER_FRAME,
};
pub use pathfinding::{find_path, find_path_diagonal, PathGrid, MAX_PATH_GRID_CELLS};
#[cfg(not(target_arch = "wasm32"))]
pub use physics::{
    sync_tilemap_entity_colliders, BodyHandle, CharacterController, ColliderHandle, CollisionEvent,
    CollisionGroups, JointHandle, PhysicsBody, PhysicsSystem, PhysicsWorld, RaycastHit, SolidTiles,
    TileCollider, TileColliderIndex, TilemapColliders, TriggerEvent,
};
pub use pool::{Pool, Pooled};
pub use prefab::{
    break_prefab_instance, spawn_entity_def, spawn_scene_def, EntityDef, Prefab, PrefabInstance,
    SceneDef, SerdeComponentEntry, SerdeComponentRegistry, Tag, SCENE_DEF_VERSION,
};
pub use skeletal::{
    BoneKeyframe, BoneTrack, SkeletalAnimationSystem, SkeletalAnimator, SkeletalClip,
    SkeletonBuilder,
};
pub use trigger_zone::{
    TriggerShape, TriggerZone, TriggerZoneRegistry, TriggerZoneSet, TriggerZoneSetError,
    TriggerZoneSystem, ZoneEvent,
};
pub use ysort::{YSort, YSortSystem};
pub use zone_effect::{
    ZoneEffectBindings, ZoneEffectError, ZoneEffectRegistry, ZoneEffectRule, ZoneEffectSystem,
    ZonePhase,
};
// par_query_for_each / par_query_map / par_query2_for_each / par_query2_map are
// World methods, so they are accessible via the World re-export (no separate re-export needed)
pub use reflect::{Reflect, ReflectValue};
pub use renderer::texture::TextureError;
pub use renderer::{
    BlendUv, DrawImage, DrawRect, DrawText, FrameContext, PostProcessConfig, RenderCapabilities,
    RenderPlugin, RenderTarget, TextAlign, TextAnchor, TextQueue, TextRenderer, Tonemap,
    UiImageQueue, UiQueue, UvRect,
};
pub use resources::{
    AmbientLight, CullConfig, DebugDraw, DebugShape, DesignResolution, DisplayScaleFactor,
    ExtraFonts, FadeTransition, FontData, FrameConfig, GameState, ImeConfig, Letterbox,
    LightingConfig, LoadProgress, PanickedSystems, PendingResize, ProfilerData, RealDt,
    RenderStats, SelectedEntity, ShouldQuit, SystemProfile, TimeScale, ViewportSize, WindowConfig,
    WindowMode, WindowOptions, DEFAULT_CANVAS_ID, DEFAULT_MAX_LIGHTS,
};
/// Native-only: the RON-load trait (hot-reload does not run on wasm).
#[cfg(not(target_arch = "wasm32"))]
pub use ron_registry::RonLoadable;
pub use ron_registry::RonRegistry;
pub use scene::{Scene, SceneChange, SceneCmd, SystemRegistrar};
pub use scripting::{ScriptAsset, ScriptRegistry, ScriptRunner, ScriptingLimits, ScriptingSystem};
pub use steering::{Arrive, Flee, Seek, SteeringSystem, SteeringVelocity, Wander};
pub use tilemap::{
    compute_tile_mask, compute_tile_mask_typed, AnimatedTileCell, AnimatedTileSystem, AutotileMode,
    Neighborhood, TerrainRule, TileAnimation, TileAnimationSet, Tilemap, TilemapAtlas,
    TilemapAutotile, TilemapProjection, TilemapSystem, DEFAULT_TILE_ANIM_STAGGER,
};
pub use timeline::{CameraTarget, Keyframe, Timeline, TimelineSystem, Track};
pub use timer::Timer;
pub use tween::{Easing, Lerp, Tween, TweenSequence};
pub use ui::{
    Anchor, Button, ButtonState, CheckBox, FocusRingStyle, Label, LayoutDir, LayoutSystem,
    LocalizationSystem, LocalizedText, Panel, ScrollView, Slider, StickNavConfig, TextInput,
    UiEvent, UiFocus, UiNode, UiSystem, VirtualJoystick, DEFAULT_SLIDER_STEP_FRAC,
    DEFAULT_STICK_ACTIVATE, DEFAULT_STICK_RELEASE,
};

// ── WASM panic hook ───────────────────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn wasm_init() {
    console_error_panic_hook::set_once();
}

// ── WASM demo entry point ─────────────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_demo() {
    use crate::{
        app::App,
        components::{Sprite, Transform},
        ecs::{System, World},
        renderer::{DrawText, TextQueue},
        resources::{ViewportSize, WindowConfig},
    };
    use glam::Vec2;
    use std::f32::consts::TAU;

    struct BounceVel {
        vx: f32,
        vy: f32,
    }

    struct DemoSystem;
    impl System for DemoSystem {
        fn run(&mut self, world: &mut World, dt: f32) {
            // Coordinate system: (0,0) = top-left, (width, height) = bottom-right
            let (w, h) = world
                .resource::<ViewportSize>()
                .map(|v| (v.width, v.height))
                .unwrap_or((1280.0, 720.0));
            let margin = 32.0;

            // Mutate Transform + BounceVel together in one pass — no collect-then-get_mut
            // workaround (see World::query2_mut).
            for (_e, t, b) in world.query2_mut::<Transform, BounceVel>() {
                t.position.x += b.vx * dt;
                t.position.y += b.vy * dt;
                if t.position.x < margin || t.position.x > w - margin {
                    b.vx = -b.vx;
                }
                if t.position.y < margin || t.position.y > h - margin {
                    b.vy = -b.vy;
                }
                t.rotation += 1.5 * dt;
            }

            if let Some(tq) = world.resource_mut::<TextQueue>() {
                tq.push(DrawText::new(
                    "skeleton-engine  —  WASM demo  (wgpu + WebGL2)",
                    Vec2::new(20.0, 20.0),
                    22.0,
                    [255, 255, 255, 220],
                ));
                tq.push(DrawText::new(
                    "ECS  ·  Rendering  ·  Animation  ·  Scripting  ·  Reflect  ·  UI",
                    Vec2::new(20.0, 52.0),
                    16.0,
                    [160, 210, 255, 200],
                ));
            }
        }
    }

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine WASM demo".to_string(),
        width: 1280,
        height: 720,
        clear_color: [0.05, 0.05, 0.10, 1.0],
    });

    let colors: &[[f32; 3]] = &[
        [1.0, 0.4, 0.4],
        [0.4, 1.0, 0.4],
        [0.4, 0.5, 1.0],
        [1.0, 1.0, 0.3],
        [1.0, 0.4, 1.0],
        [0.3, 1.0, 0.9],
        [1.0, 0.65, 0.3],
        [0.6, 0.3, 1.0],
        [0.3, 0.9, 0.5],
        [0.9, 0.4, 0.2],
    ];
    let velocities: &[(f32, f32)] = &[
        (150.0, 110.0),
        (-130.0, 170.0),
        (190.0, -100.0),
        (-160.0, -145.0),
        (120.0, 185.0),
        (-200.0, 130.0),
        (145.0, -175.0),
        (-115.0, -160.0),
        (175.0, 140.0),
        (-155.0, 125.0),
    ];

    // Arrange entities in a circle around the screen center (640, 360)
    let cx = 640.0_f32;
    let cy = 360.0_f32;
    for (i, (&[r, g, b], &(vx, vy))) in colors.iter().zip(velocities.iter()).enumerate() {
        let angle = i as f32 * TAU / colors.len() as f32;
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: Vec2::new(cx + angle.cos() * 220.0, cy + angle.sin() * 160.0),
                scale: Vec2::splat(64.0),
                rotation: angle,
                z: 0.0,
            },
        );
        app.world.add_component(e, Sprite::colored(r, g, b));
        app.world.add_component(e, BounceVel { vx, vy });
    }

    app.add_system(DemoSystem);
    app.run();
}
