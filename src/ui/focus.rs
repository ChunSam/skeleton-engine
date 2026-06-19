//! Keyboard focus for UI widgets.
//!
//! [`UiFocus`] is a `World` resource holding the currently keyboard-focused widget. [`UiSystem`]'s
//! focus pass cycles it with **Tab / Shift+Tab** across focusable widgets ([`Button`], [`TextInput`],
//! [`Slider`], [`CheckBox`]), draws a focus ring around it, and activates it on **Enter/Space**
//! (click a button, toggle a checkbox) or adjusts a focused [`Slider`] with **Left/Right**. Clicking
//! a widget also moves focus to it, so Tab resumes from there.
//!
//! It is inserted automatically (see `insert_core_resources`); read it to know what's focused.
//! The ring's appearance is configurable via the [`FocusRingStyle`] resource (also auto-inserted).
//!
//! [`UiSystem`]: crate::ui::UiSystem
//! [`Button`]: crate::ui::Button
//! [`TextInput`]: crate::ui::TextInput
//! [`Slider`]: crate::ui::Slider
//! [`CheckBox`]: crate::ui::CheckBox

use crate::color::Color;
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

/// Appearance of the focus ring [`UiSystem`]'s focus pass draws around the focused widget.
///
/// A `World` resource (auto-inserted with the default amber 3px ring, see `insert_core_resources`).
/// Insert your own to restyle it, e.g. a thicker cyan ring:
///
/// ```
/// # use engine::{Color, FocusRingStyle};
/// let style = FocusRingStyle {
///     color: Color::rgb(0.3, 0.9, 1.0),
///     thickness: 5.0,
///     ..Default::default()
/// };
/// assert!(style.is_visible());
/// ```
///
/// Set `enabled = false` (or `thickness <= 0.0`) to suppress the engine ring entirely — useful when
/// a game draws its own focus indicator. The default matches the historical hardcoded ring exactly.
///
/// [`UiSystem`]: crate::ui::UiSystem
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusRingStyle {
    /// Ring border color.
    pub color: Color,
    /// Border thickness in screen pixels. `<= 0.0` draws no ring.
    pub thickness: f32,
    /// Whether to draw the ring at all. `false` draws no ring (the focus logic still runs).
    pub enabled: bool,
}

impl Default for FocusRingStyle {
    fn default() -> Self {
        Self {
            color: Color::rgba(1.0, 0.85, 0.3, 1.0),
            thickness: 3.0,
            enabled: true,
        }
    }
}

impl FocusRingStyle {
    /// Whether a ring should actually be drawn (enabled, with a positive thickness).
    pub fn is_visible(&self) -> bool {
        self.enabled && self.thickness > 0.0
    }
}
