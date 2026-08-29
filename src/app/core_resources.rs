use crate::{
    asset::AssetServer,
    camera::Camera,
    ecs::World,
    input::{GamepadState, InputState, TouchState},
    renderer::{TextQueue, UiImageQueue, UiQueue},
    resources::{
        DebugDraw, FrameConfig, GameState, LoadProgress, PanickedSystems, PendingResize,
        ProfilerData, RealDt, SelectedEntity, ShouldQuit, TimeScale, ViewportSize, WindowConfig,
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
    world.insert_resource(FrameConfig::default());
    world.insert_resource(WindowConfig::default());
    world.insert_resource(ViewportSize::default());
    world.insert_resource(PendingResize::default());
    world.insert_resource(Camera::default());
    world.insert_resource(TextQueue::default());
    world.insert_resource(UiQueue::default());
    world.insert_resource(UiImageQueue::default());
    world.insert_resource(crate::ui::UiFocus::default());
    world.insert_resource(crate::ui::FocusRingStyle::default());
    world.insert_resource(crate::ui::StickNavConfig::default());
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
    world.register_reflect_named::<crate::ui::ProgressBar>("ProgressBar");
    world.register_reflect_named::<crate::ui::RadioGroup>("RadioGroup");
    world.register_reflect_named::<crate::ui::TabBar>("TabBar");
    world.register_reflect_named::<crate::ui::ListBox>("ListBox");
    world.register_reflect_named::<crate::ui::Stepper>("Stepper");
    world.register_reflect_named::<crate::ui::Switch>("Switch");
    world.register_reflect_named::<crate::ui::Dropdown>("Dropdown");
    world.register_reflect_named::<crate::ui::Tooltip>("Tooltip");
    world.register_reflect_named::<crate::ui::LocalizedText>("LocalizedText");

    world.register_clone::<crate::components::Transform>();
    world.register_clone::<crate::components::Sprite>();
    world.register_clone::<crate::components::RenderLayer>();
    world.register_clone::<crate::components::SpriteFlip>();
    world.register_clone::<crate::components::Hidden>();
    world.register_clone::<crate::ysort::YSort>();
    world.register_clone::<crate::trigger_zone::TriggerZone>();
    world.register_clone::<crate::hit_flash::HitFlash>();
    world.register_clone::<crate::floating_text::FloatingText>();
    world.register_clone::<crate::sprite_trail::SpriteTrail>();
    world.register_clone::<crate::prefab::Tag>();
    world.register_clone::<crate::animation::player::AnimationPlayer>();
    world.register_clone::<crate::animation::events::AnimationEvents>();
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
    world.register_clone::<crate::ui::ProgressBar>();
    world.register_clone::<crate::ui::RadioGroup>();
    world.register_clone::<crate::ui::TabBar>();
    world.register_clone::<crate::ui::ListBox>();
    world.register_clone::<crate::ui::Stepper>();
    world.register_clone::<crate::ui::Switch>();
    world.register_clone::<crate::ui::Dropdown>();
    world.register_clone::<crate::ui::Tooltip>();
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
        // Render-modifier components. These were `register_clone`d (so copy/paste and scene
        // reset carried them) and editor-addable, but never serde-registered — and
        // `serialize_entity` walks only the serde registry, so scene save dropped them in
        // silence. The eye 👁 toggle in the entity list adds `Hidden`, which makes this the
        // single most common editor gesture whose result did not survive Ctrl+S. All four
        // derive `Serialize + Deserialize + Clone`, so nothing about them was transient by
        // design. Names must match the `register_reflect_named` / editor keys exactly.
        registry.register::<crate::components::Hidden>("Hidden", None);
        registry.register::<crate::components::RenderLayer>("RenderLayer", None);
        registry.register::<crate::components::SpriteFlip>("SpriteFlip", None);
        registry.register::<crate::ysort::YSort>("YSort", None);

        // The same gap, six more. All six are editor-addable via
        // `register_default_components` and carry a ✕ remover, but were in neither this
        // registry nor `EntityDef`'s named fields (`tag` / `transform` / `sprite`), so
        // `serialize_entity` walked straight past them and Save Scene dropped them in
        // silence — a PointLight or ParticleEmitter placed in the Inspector did not
        // survive a reload. Runtime state is `#[serde(skip)]`ped at the type rather than
        // here: `HitFlash::{elapsed, base}`, `SpriteTrail::elapsed`, and
        // `TriggerZone::occupants`, which holds `Entity` handles that cannot survive a
        // reload at all (they are generation-checked against a World that no longer exists).
        registry.register::<crate::trigger_zone::TriggerZone>("TriggerZone", None);
        registry.register::<crate::hit_flash::HitFlash>("HitFlash", None);
        registry.register::<crate::sprite_trail::SpriteTrail>("SpriteTrail", None);
        registry.register::<crate::animation::events::AnimationEvents>("AnimationEvents", None);
        registry.register::<crate::particle::ParticleEmitter>("ParticleEmitter", None);
        registry.register::<crate::components::PointLight>("PointLight", None);

        registry.register::<crate::ui::CheckBox>("CheckBox", None);
        registry.register::<crate::ui::ProgressBar>("ProgressBar", None);
        registry.register::<crate::ui::RadioGroup>("RadioGroup", None);
        registry.register::<crate::ui::TabBar>("TabBar", None);
        registry.register::<crate::ui::ListBox>("ListBox", None);
        registry.register::<crate::ui::Stepper>("Stepper", None);
        registry.register::<crate::ui::Switch>("Switch", None);
        registry.register::<crate::ui::Dropdown>("Dropdown", None);
        registry.register::<crate::ui::Tooltip>("Tooltip", None);
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
