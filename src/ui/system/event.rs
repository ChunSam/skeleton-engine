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
    /// Dropdown selection changed. The second field is the new selected item index.
    /// Emitted only when the index actually changed (re-picking the current item is silent).
    DropdownChanged(Entity, usize),
    /// RadioGroup selection changed. The second field is the new selected option index.
    /// Emitted only when the index actually changed (re-picking the current option is silent).
    RadioChanged(Entity, usize),
    /// TabBar active tab changed. The second field is the new active tab index.
    /// Emitted only when the index actually changed (re-picking the current tab is silent).
    TabChanged(Entity, usize),
    /// ListBox selection changed. The second field is the new selected row index.
    /// Emitted only when the index actually changed (re-picking the current row is silent).
    ListBoxChanged(Entity, usize),
    /// Stepper value changed. The second field is the new value.
    /// Emitted only when the value actually changed (stepping past a bound is silent).
    StepperChanged(Entity, f32),
    /// Switch (boolean toggle) flipped. The second field is the new on/off state.
    /// A click always flips the state, so every click emits; ←/→ set the state absolutely and emit
    /// only when it actually changed (turning an already-on switch on again is silent).
    SwitchToggled(Entity, bool),
}
