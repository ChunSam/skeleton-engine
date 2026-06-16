pub mod animation;
pub mod app;
pub mod asset;
pub mod atlas;
#[cfg(not(target_arch = "wasm32"))]
pub mod audio;
pub mod behavior;
pub mod camera;
pub mod collision;
pub mod color;
pub mod components;
pub mod data_table;
pub mod debug_ui;
pub mod ecs;
#[cfg(not(target_arch = "wasm32"))]
pub mod gpu_particle;
pub mod hierarchy;
pub mod history;
pub mod input;
pub mod locale;
pub mod material;
pub mod network;
pub mod particle;
pub mod pathfinding;
#[cfg(not(target_arch = "wasm32"))]
pub mod physics;
pub mod pool;
pub mod prefab;
pub mod reflect;
pub mod renderer;
pub mod resources;
pub mod save;
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

pub use glam::{IVec2, Mat4, Vec2, Vec3};
pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

pub use animation::{
    AnimParam, AnimState, AnimTransition, AnimationClip, AnimationClipRegistry, AnimationClipSet,
    AnimationPlayer, AnimationStateMachine, AnimationSystem, BlendEntry, BlendTree1D,
    BlendTreeSystem, BlendWeight, ClipSetError, StateMachineSystem, TransitionCond,
};
pub use app::{App, ScheduleErrorPolicy, SystemPanicPolicy};
#[cfg(not(target_arch = "wasm32"))]
pub use asset::HotReloadable;
pub use asset::{
    AssetId, AssetLoadState, AssetServer, Handle, ImageAsset, ImageEntry, ScriptAsset,
};
pub use atlas::{AtlasSprite, TextureAtlas};
#[cfg(not(target_arch = "wasm32"))]
pub use audio::{AudioChannelState, AudioEffect, AudioManager, AudioSystem, BusDuck, Sidechain};
pub use behavior::{
    AlwaysSucceed, BehaviorNode, BehaviorStatus, BehaviorSystem, BehaviorTree, Blackboard,
    BlackboardValue, Inverter, Selector, Sequence,
};
pub use camera::Camera;
pub use collision::{
    Collider, CollisionDebugSystem, CollisionGridSystem, CollisionLayer, DebugConfig, SpatialGrid,
};
pub use color::Color;
pub use components::{OffscreenCamera, PointLight, RenderLayer, Sprite, Transform};
pub use data_table::{DataTable, DataTableRegistry};
pub use debug_ui::DebugUi;
pub use ecs::schedule::{ScheduleError, SystemConfig, SystemLabel};
pub use ecs::{Commands, Entity, Events, System, World};
#[cfg(not(target_arch = "wasm32"))]
pub use gpu_particle::GpuParticleEmitter;
pub use hierarchy::{
    attach, detach, topological_sort_entities, Children, GlobalTransform, HierarchySystem, Parent,
};
pub use history::History;
pub use input::AxisBinding;
pub use input::{GamepadAxis, GamepadButton, GamepadState, InputMap, InputState, TouchState};
pub use locale::{LocaleBundle, LocaleData, LocaleResource, TextDirection};
pub use material::ShaderMaterial;
pub use network::{
    NetworkClient, NetworkConfig, NetworkEvent, NetworkSystem, RemoteEntities, SnapshotBuffer,
};
pub use particle::{
    Particle, ParticleBurst, ParticleConfigError, ParticleConfigRegistry, ParticleConfigSet,
    ParticleEmitter, ParticleSystem,
};
pub use pathfinding::{find_path, find_path_diagonal, PathGrid};
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
// par_query_for_each / par_query_map / par_query2_for_each / par_query2_map are
// World methods, so they are accessible via the World re-export (no separate re-export needed)
pub use reflect::{Reflect, ReflectValue};
pub use renderer::texture::TextureError;
pub use renderer::{
    BlendUv, DrawImage, DrawRect, DrawText, PostProcessConfig, RenderTarget, TextAlign, TextAnchor,
    TextQueue, TextRenderer, UiImageQueue, UiQueue, UvRect,
};
pub use resources::{
    AmbientLight, CullConfig, DebugDraw, DebugShape, DisplayScaleFactor, ExtraFonts,
    FadeTransition, FontData, GameState, ImeConfig, LoadProgress, PanickedSystems, PendingResize,
    ProfilerData, RenderStats, SelectedEntity, ShouldQuit, SystemProfile, ViewportSize,
    WindowConfig,
};
pub use scene::{Scene, SceneChange, SceneCmd, SystemRegistrar};
pub use scripting::{ScriptRunner, ScriptingSystem};
pub use steering::{Arrive, Flee, Seek, SteeringSystem, SteeringVelocity, Wander};
pub use tilemap::{
    compute_tile_mask, compute_tile_mask_typed, ConnectRule, MultiTerrainAutotile, Neighborhood,
    TerrainRule, Tilemap, TilemapAtlas, TilemapAutotile, TilemapSystem,
};
pub use timeline::{CameraTarget, Keyframe, Timeline, TimelineSystem, Track};
pub use timer::Timer;
pub use tween::{Easing, Lerp, Tween};
pub use ui::{
    Anchor, Button, ButtonState, CheckBox, Label, LayoutDir, LayoutSystem, LocalizationSystem,
    LocalizedText, Panel, ScrollView, Slider, TextInput, UiEvent, UiNode, UiSystem,
    VirtualJoystick,
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
        ecs::{Entity, System, World},
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

            let data: Vec<(Entity, f32, f32, f32, f32)> = world
                .query2::<Transform, BounceVel>()
                .map(|(e, t, b)| (e, t.position.x, t.position.y, b.vx, b.vy))
                .collect();

            for (e, x, y, vx, vy) in data {
                let nx = x + vx * dt;
                let ny = y + vy * dt;
                let nvx = if nx < margin || nx > w - margin {
                    -vx
                } else {
                    vx
                };
                let nvy = if ny < margin || ny > h - margin {
                    -vy
                } else {
                    vy
                };
                if let Some(t) = world.get_mut::<Transform>(e) {
                    t.position.x = nx;
                    t.position.y = ny;
                    t.rotation += 1.5 * dt;
                }
                if let Some(b) = world.get_mut::<BounceVel>(e) {
                    b.vx = nvx;
                    b.vy = nvy;
                }
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
