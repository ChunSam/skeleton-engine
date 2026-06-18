//! Keyboard focus for UI widgets.
//!
//! [`UiFocus`] is a `World` resource holding the currently keyboard-focused widget. [`UiSystem`]'s
//! focus pass cycles it with **Tab / Shift+Tab** across focusable widgets ([`Button`], [`TextInput`],
//! [`Slider`], [`CheckBox`]), draws a focus ring around it, and activates it on **Enter/Space**
//! (click a button, toggle a checkbox) or adjusts a focused [`Slider`] with **Left/Right**. Clicking
//! a widget also moves focus to it, so Tab resumes from there.
//!
//! It is inserted automatically (see `insert_core_resources`); read it to know what's focused.
//!
//! [`UiSystem`]: crate::ui::UiSystem
//! [`Button`]: crate::ui::Button
//! [`TextInput`]: crate::ui::TextInput
//! [`Slider`]: crate::ui::Slider
//! [`CheckBox`]: crate::ui::CheckBox

use crate::ecs::Entity;

/// The currently keyboard-focused UI widget, or `None` when nothing is focused.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UiFocus {
    /// The focused widget entity (`None` = no focus).
    pub entity: Option<Entity>,
}

impl UiFocus {
    /// Convenience: whether `entity` is the focused widget.
    pub fn is_focused(&self, entity: Entity) -> bool {
        self.entity == Some(entity)
    }
}
