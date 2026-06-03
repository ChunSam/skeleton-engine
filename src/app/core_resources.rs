use crate::{
    asset::AssetServer,
    camera::Camera,
    ecs::World,
    input::{GamepadState, InputState, TouchState},
    renderer::{TextQueue, UiImageQueue, UiQueue},
    resources::{
        DebugDraw, DebugDrawQueue, GameState, LoadProgress, PanickedSystems, PendingResize,
        ProfilerData, SelectedEntity, ShouldQuit, ViewportSize, WindowConfig,
    },
    scene::SceneChange,
};

pub(super) fn insert_core_resources(world: &mut World) {
    world.insert_resource(InputState::default());
    world.insert_resource(GamepadState::default());
    world.insert_resource(TouchState::default());
    world.insert_resource(GameState::Playing);
    world.insert_resource(ShouldQuit(false));
    world.insert_resource(WindowConfig::default());
    world.insert_resource(ViewportSize::default());
    world.insert_resource(PendingResize::default());
    world.insert_resource(Camera::default());
    world.insert_resource(TextQueue::default());
    world.insert_resource(UiQueue::default());
    world.insert_resource(UiImageQueue::default());
    world.insert_resource(DebugDrawQueue::default());
    world.insert_resource(DebugDraw::new());
    world.insert_resource(SelectedEntity::default());
    world.insert_resource(ProfilerData::default());
    world.insert_resource(SceneChange::default());
    world.insert_resource(AssetServer::new());
    world.insert_resource(LoadProgress::default());
    world.insert_resource(PanickedSystems::default());
}

pub(super) fn register_core_component_metadata(world: &mut World) {
    world.register_reflect_named::<crate::components::Transform>("Transform");
    world.register_reflect_named::<crate::components::Sprite>("Sprite");
    world.register_reflect_named::<crate::prefab::Tag>("Tag");

    world.register_clone::<crate::components::Transform>();
    world.register_clone::<crate::components::Sprite>();
    world.register_clone::<crate::components::RenderLayer>();
    world.register_clone::<crate::prefab::Tag>();
    world.register_clone::<crate::animation::player::AnimationPlayer>();
    world.register_clone::<crate::timer::Timer>();
}
