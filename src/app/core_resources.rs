use crate::{
    asset::AssetServer,
    camera::Camera,
    ecs::World,
    input::{GamepadState, InputState, TouchState},
    renderer::{TextQueue, UiImageQueue, UiQueue},
    resources::{
        DebugDraw, GameState, LoadProgress, PanickedSystems, PendingResize, ProfilerData, RealDt,
        SelectedEntity, ShouldQuit, TimeScale, ViewportSize, WindowConfig,
    },
    scene::SceneChange,
};

pub(super) fn insert_core_resources(world: &mut World) {
    world.insert_resource(InputState::default());
    world.insert_resource(GamepadState::default());
    world.insert_resource(TouchState::default());
    world.insert_resource(GameState::Playing);
    world.insert_resource(ShouldQuit(false));
    world.insert_resource(TimeScale::default());
    world.insert_resource(RealDt::default());
    world.insert_resource(WindowConfig::default());
    world.insert_resource(ViewportSize::default());
    world.insert_resource(PendingResize::default());
    world.insert_resource(Camera::default());
    world.insert_resource(TextQueue::default());
    world.insert_resource(UiQueue::default());
    world.insert_resource(UiImageQueue::default());
    world.insert_resource(crate::ui::UiFocus::default());
    world.insert_resource(crate::ui::FocusRingStyle::default());
    world.insert_resource(DebugDraw::new());
    world.insert_resource(SelectedEntity::default());
    world.insert_resource(ProfilerData::default());
    world.insert_resource(SceneChange::default());
    world.insert_resource(AssetServer::new());
    world.insert_resource(crate::scripting::ScriptRegistry::default());
    world.insert_resource(LoadProgress::default());
    world.insert_resource(PanickedSystems::default());
    world.insert_resource(crate::prefab::SerdeComponentRegistry::default());
}

pub(super) fn register_core_component_metadata(world: &mut World) {
    world.register_reflect_named::<crate::components::Transform>("Transform");
    world.register_reflect_named::<crate::components::Sprite>("Sprite");
    world.register_reflect_named::<crate::prefab::Tag>("Tag");

    // UI widget components
    world.register_reflect_named::<crate::ui::UiNode>("UiNode");
    world.register_reflect_named::<crate::ui::Button>("Button");
    world.register_reflect_named::<crate::ui::Label>("Label");
    world.register_reflect_named::<crate::ui::TextInput>("TextInput");
    world.register_reflect_named::<crate::ui::Slider>("Slider");
    world.register_reflect_named::<crate::ui::CheckBox>("CheckBox");
    world.register_reflect_named::<crate::ui::ScrollView>("ScrollView");
    world.register_reflect_named::<crate::ui::panel::Panel>("Panel");
    world.register_reflect_named::<crate::ui::LocalizedText>("LocalizedText");

    world.register_clone::<crate::components::Transform>();
    world.register_clone::<crate::components::Sprite>();
    world.register_clone::<crate::components::RenderLayer>();
    world.register_clone::<crate::prefab::Tag>();
    world.register_clone::<crate::animation::player::AnimationPlayer>();
    world.register_clone::<crate::timer::Timer>();

    // UI widget clone registrations
    world.register_clone::<crate::ui::UiNode>();
    world.register_clone::<crate::ui::Button>();
    world.register_clone::<crate::ui::Label>();
    world.register_clone::<crate::ui::TextInput>();
    world.register_clone::<crate::ui::Slider>();
    world.register_clone::<crate::ui::CheckBox>();
    world.register_clone::<crate::ui::ScrollView>();
    world.register_clone::<crate::ui::panel::Panel>();
    world.register_clone::<crate::ui::LocalizedText>();

    // Register UI widget serde components for scene save/load
    if let Some(registry) = world.resource_mut::<crate::prefab::SerdeComponentRegistry>() {
        registry.register::<crate::ui::UiNode>("UiNode", None);
        registry.register::<crate::ui::Button>("Button", None);
        registry.register::<crate::ui::Label>("Label", None);
        registry.register::<crate::ui::TextInput>(
            "TextInput",
            Some(Box::new(|w, e| {
                if let Some(ti) = w.get_mut::<crate::ui::TextInput>(e) {
                    ti.text = ti.initial_text.clone();
                    ti.cursor = ti.text.len();
                }
            })),
        );
        registry.register::<crate::ui::Slider>(
            "Slider",
            Some(Box::new(|w, e| {
                if let Some(s) = w.get_mut::<crate::ui::Slider>(e) {
                    s.value = s.initial_value.clamp(s.min, s.max);
                }
            })),
        );
        registry.register::<crate::ui::CheckBox>("CheckBox", None);
        registry.register::<crate::ui::ScrollView>("ScrollView", None);
        registry.register::<crate::ui::panel::Panel>("Panel", None);
        registry.register::<crate::ui::LocalizedText>("LocalizedText", None);

        // Animation and timeline components for editor save/load
        registry.register::<crate::animation::state_machine::AnimationStateMachine>(
            "AnimationStateMachine",
            None,
        );
        registry.register::<crate::timeline::Timeline>("Timeline", None);
        registry.register::<crate::timeline::CameraTarget>("CameraTarget", None);
    }
}
