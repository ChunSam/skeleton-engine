use crate::ecs::Entity;

/// Events emitted by the UI system.
///
/// After calling `app.register_event::<UiEvent>()`, read them via `world.resource::<Events<UiEvent>>()`.
#[derive(Debug, Clone, PartialEq)]
pub enum UiEvent {
    ButtonClicked(Entity),
    TextChanged(Entity, String),
    TextSubmitted(Entity, String),
    TextFocused(Entity),
    TextBlurred(Entity),
    /// Slider value changed. The second field is the new value.
    SliderChanged(Entity, f32),
    /// CheckBox state toggled. The second field is the new checked value.
    CheckBoxToggled(Entity, bool),
}
