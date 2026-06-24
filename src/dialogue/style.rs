//! [`DialogueStyle`] — the opt-in resource that restyles [`DialogueSystem`](crate::DialogueSystem)
//! (layout positions, font sizes, colors) without forking the engine.

use serde::{Deserialize, Serialize};

/// Visual style for [`DialogueSystem`](crate::DialogueSystem): the layout positions, font sizes,
/// and colors of the speaker, body, choice list, advance hint, and portrait. Insert a customized
/// one as a resource to restyle dialogue **without forking the engine** — absent, the system uses
/// [`DialogueStyle::default`], which reproduces the previous hardcoded look exactly, so existing
/// games are unchanged.
///
/// Vertical positions are **offsets up from the viewport bottom** (a taller window keeps the box
/// anchored to the bottom), matching the original `vh - N` layout. The advance-hint x is an offset
/// **left** from the viewport right edge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DialogueStyle {
    /// Fallback viewport `(width, height)` when no [`ViewportSize`](crate::ViewportSize) resource is present.
    pub fallback_viewport: crate::Vec2,
    /// Square speaker-portrait edge length (px).
    pub portrait_size: f32,
    /// Portrait left x (px).
    pub portrait_x: f32,
    /// Portrait top y, as an offset up from the viewport bottom (px).
    pub portrait_bottom_offset: f32,
    /// Gap between the portrait's right edge and the text column, when a portrait is shown (px).
    pub portrait_text_gap: f32,
    /// Left text margin when there is no portrait (px).
    pub text_margin: f32,
    /// Speaker label bottom offset (px up from bottom).
    pub speaker_bottom_offset: f32,
    /// Speaker label font size.
    pub speaker_font_size: f32,
    /// Speaker label color.
    pub speaker_color: crate::Color,
    /// Body-text bottom offset (px up from bottom).
    pub body_bottom_offset: f32,
    /// Body-text font size.
    pub body_font_size: f32,
    /// Body-text color.
    pub body_color: crate::Color,
    /// Body-text wrap-bounds height (px).
    pub body_height: f32,
    /// Choice-list first-row bottom offset (px up from bottom).
    pub choice_bottom_offset: f32,
    /// Choice-row x indent from the text column (px).
    pub choice_indent: f32,
    /// Choice-row font size.
    pub choice_font_size: f32,
    /// Choice-row color.
    pub choice_color: crate::Color,
    /// Per-choice-row vertical step (px).
    pub choice_line_step: f32,
    /// Advance-hint x as an offset left from the viewport right edge (px).
    pub hint_right_offset: f32,
    /// Advance-hint bottom offset (px up from bottom).
    pub hint_bottom_offset: f32,
    /// Advance-hint font size.
    pub hint_font_size: f32,
    /// Advance-hint color.
    pub hint_color: crate::Color,
    /// Advance-hint label (shown once a line is fully revealed and no choice is pending).
    pub hint_label: String,
}

impl Default for DialogueStyle {
    fn default() -> Self {
        Self {
            fallback_viewport: crate::Vec2::new(1280.0, 720.0),
            portrait_size: 96.0,
            portrait_x: 48.0,
            portrait_bottom_offset: 162.0,
            portrait_text_gap: 18.0,
            text_margin: 60.0,
            speaker_bottom_offset: 150.0,
            speaker_font_size: 22.0,
            speaker_color: crate::Color::rgb(1.0, 0.85, 0.35),
            body_bottom_offset: 118.0,
            body_font_size: 20.0,
            body_color: crate::Color::WHITE,
            body_height: 90.0,
            choice_bottom_offset: 86.0,
            choice_indent: 16.0,
            choice_font_size: 18.0,
            choice_color: crate::Color::rgb(0.85, 0.92, 1.0),
            choice_line_step: 26.0,
            hint_right_offset: 150.0,
            hint_bottom_offset: 42.0,
            hint_font_size: 16.0,
            hint_color: crate::Color::rgb(0.6, 0.7, 0.85),
            hint_label: "▼ space".to_string(),
        }
    }
}
