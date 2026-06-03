use crate::ecs::Entity;

/// UI 시스템이 발행하는 이벤트.
///
/// `app.register_event::<UiEvent>()` 후 `world.resource::<Events<UiEvent>>()` 로 읽는다.
#[derive(Debug, Clone)]
pub enum UiEvent {
    ButtonClicked(Entity),
    TextChanged(Entity, String),
    TextSubmitted(Entity, String),
    TextFocused(Entity),
    TextBlurred(Entity),
    /// Slider 값이 변경됨. 두 번째 필드는 새 값.
    SliderChanged(Entity, f32),
    /// CheckBox 상태가 토글됨. 두 번째 필드는 새 checked 값.
    CheckBoxToggled(Entity, bool),
}
