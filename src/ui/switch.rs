use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

/// Gap in pixels between the track edge and the knob (and between the knob and the far edge in the
/// `on` position). Keeps the round knob clear of the pill's rounded ends.
const KNOB_PAD: f32 = 3.0;

/// A styled boolean toggle — a sliding **track + knob** rendered as a pill: the knob sits left when
/// `off` and right when `on`, and the track changes color between the two states. The switch-look
/// alternative to a [`CheckBox`](crate::ui::CheckBox) (same boolean meaning, different affordance —
/// reach for it in settings rows where an on/off switch reads more naturally than a tick box).
///
/// Attach it to an entity alongside a [`UiNode`](crate::ui::UiNode); **one entity is the whole
/// widget**. Clicking anywhere on the node flips `on` and emits
/// [`UiEvent::SwitchToggled`](crate::ui::UiEvent::SwitchToggled) with the new state (like a
/// [`CheckBox`](crate::ui::CheckBox)). An optional `label` is drawn to the right of the track.
///
/// Keyboard/gamepad: the switch is one focus stop (Tab / D-pad / stick). **Enter/Space** (or gamepad
/// **A**) toggle it; **←** turns it off and **→** turns it on (keyboard, D-pad, or left stick), each
/// emitting only when the state actually changed — so it is fully operable without a pointer.
///
/// # Example
/// ```
/// # use engine::ui::Switch;
/// let s = Switch::new("Sound").with_on(true);
/// assert!(s.on);
/// assert_eq!(s.label, "Sound");
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Switch {
    /// The current on/off state.
    pub on: bool,
    /// Optional text drawn to the right of the track (empty = no label).
    pub label: String,
    /// Track (pill) width in pixels.
    pub track_width: f32,
    /// Track (pill) height in pixels; also sets the pill corner radius (`height / 2`) and the knob
    /// diameter (`height - 2·pad`).
    pub track_height: f32,
    /// Track fill when `on`.
    pub on_color: Color,
    /// Track fill when `off`.
    pub off_color: Color,
    /// Fill of the sliding knob.
    pub knob_color: Color,
    /// Label text color.
    pub text_color: Color,
    /// Label font size in pixels.
    pub font_size: f32,
}

impl Default for Switch {
    fn default() -> Self {
        Self::new("")
    }
}

impl Switch {
    /// A switch (initially `off`) with `label` and the default style.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            on: false,
            label: label.into(),
            track_width: 46.0,
            track_height: 24.0,
            on_color: Color::rgba(0.28, 0.56, 0.90, 1.0),
            off_color: Color::rgba(0.24, 0.25, 0.30, 1.0),
            knob_color: Color::rgba_u8(235, 237, 242, 255),
            text_color: Color::rgba_u8(210, 210, 220, 255),
            font_size: 16.0,
        }
    }

    /// Set the initial on/off state. Builder form.
    pub fn with_on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    /// Set the track width and height in pixels. Builder form.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.track_width = width;
        self.track_height = height;
        self
    }

    /// Set the on, off, and knob colors. Builder form.
    pub fn with_colors(mut self, on: Color, off: Color, knob: Color) -> Self {
        self.on_color = on;
        self.off_color = off;
        self.knob_color = knob;
        self
    }

    /// Set the label text color. Builder form.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set the label font size in pixels. Builder form.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// The current track fill: [`on_color`](Self::on_color) when on, else [`off_color`](Self::off_color).
    pub fn track_color(&self) -> Color {
        if self.on {
            self.on_color
        } else {
            self.off_color
        }
    }

    /// Pill corner radius (half the track height) — makes the track a rounded capsule.
    pub fn track_radius(&self, node_size: Vec2) -> f32 {
        self.track_size(node_size).y / 2.0
    }

    /// The track rect `(pos, size)` for a widget drawn at `(pos, node_size)`: left-aligned in the
    /// node and vertically centered, its height clamped to the node height. The single geometry
    /// source shared by rendering and the knob position so they can never disagree.
    pub fn track_rect(&self, pos: Vec2, node_size: Vec2) -> (Vec2, Vec2) {
        let size = self.track_size(node_size);
        let y = pos.y + (node_size.y - size.y) / 2.0;
        (Vec2::new(pos.x, y), size)
    }

    /// The knob rect `(pos, size)` for the current [`on`](Self::on) state: a square (diameter =
    /// track height − 2·pad) padded inside the track, flush left when off and flush right when on.
    pub fn knob_rect(&self, pos: Vec2, node_size: Vec2) -> (Vec2, Vec2) {
        let (track_pos, track_size) = self.track_rect(pos, node_size);
        let d = (track_size.y - KNOB_PAD * 2.0).max(0.0);
        let y = track_pos.y + KNOB_PAD;
        let x = if self.on {
            track_pos.x + track_size.x - KNOB_PAD - d
        } else {
            track_pos.x + KNOB_PAD
        };
        (Vec2::new(x, y), Vec2::splat(d))
    }

    /// Knob corner radius (half its diameter) — makes the knob a circle.
    pub fn knob_radius(&self, node_size: Vec2) -> f32 {
        (self.track_size(node_size).y - KNOB_PAD * 2.0).max(0.0) / 2.0
    }

    /// The clamped track size: [`track_width`](Self::track_width) × min(track_height, node height).
    fn track_size(&self, node_size: Vec2) -> Vec2 {
        Vec2::new(
            self.track_width.max(0.0),
            self.track_height.min(node_size.y).max(0.0),
        )
    }
}

impl Reflect for Switch {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("on", ReflectValue::Bool(self.on)),
            ("label", ReflectValue::String(self.label.clone())),
            ("track_width", ReflectValue::F32(self.track_width)),
            ("track_height", ReflectValue::F32(self.track_height)),
            ("font_size", ReflectValue::F32(self.font_size)),
            ("on_color", ReflectValue::Color(self.on_color.to_array())),
            ("off_color", ReflectValue::Color(self.off_color.to_array())),
            (
                "knob_color",
                ReflectValue::Color(self.knob_color.to_array()),
            ),
            (
                "text_color",
                ReflectValue::Color(self.text_color.to_array()),
            ),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("on", ReflectValue::Bool(v)) => {
                self.on = v;
                true
            }
            ("label", ReflectValue::String(v)) => {
                self.label = v;
                true
            }
            ("track_width", ReflectValue::F32(v)) => {
                self.track_width = v;
                true
            }
            ("track_height", ReflectValue::F32(v)) => {
                self.track_height = v;
                true
            }
            ("font_size", ReflectValue::F32(v)) => {
                self.font_size = v;
                true
            }
            ("on_color", ReflectValue::Color(c)) => {
                self.on_color = Color::from(c);
                true
            }
            ("off_color", ReflectValue::Color(c)) => {
                self.off_color = Color::from(c);
                true
            }
            ("knob_color", ReflectValue::Color(c)) => {
                self.knob_color = Color::from(c);
                true
            }
            ("text_color", ReflectValue::Color(c)) => {
                self.text_color = Color::from(c);
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "Switch"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_off_with_the_label() {
        let s = Switch::new("Fullscreen");
        assert!(!s.on);
        assert_eq!(s.label, "Fullscreen");
    }

    #[test]
    fn track_color_follows_state() {
        let s = Switch::new("x");
        assert_eq!(s.track_color(), s.off_color);
        assert_eq!(s.clone().with_on(true).track_color(), s.on_color);
    }

    #[test]
    fn knob_sits_left_when_off_and_right_when_on() {
        let pos = Vec2::new(10.0, 10.0);
        let node = Vec2::new(120.0, 40.0);
        let off = Switch::new("x"); // off
        let on = Switch::new("x").with_on(true);
        let (off_knob, off_size) = off.knob_rect(pos, node);
        let (on_knob, on_size) = on.knob_rect(pos, node);
        assert_eq!(
            off_size, on_size,
            "the knob is the same size in both states"
        );
        assert!(
            on_knob.x > off_knob.x,
            "the knob slides right when on: {} vs {}",
            on_knob.x,
            off_knob.x
        );
        // Both knobs stay inside the track horizontally.
        let (track_pos, track_size) = off.track_rect(pos, node);
        assert!(off_knob.x >= track_pos.x);
        assert!(on_knob.x + on_size.x <= track_pos.x + track_size.x + f32::EPSILON);
    }

    #[test]
    fn track_is_vertically_centered_and_height_clamped() {
        let pos = Vec2::new(0.0, 0.0);
        // Node taller than the track: track is centered, keeps its own height.
        let s = Switch::new("x"); // track 46×24
        let (tp, ts) = s.track_rect(pos, Vec2::new(200.0, 60.0));
        assert_eq!(ts, Vec2::new(46.0, 24.0));
        assert_eq!(tp.y, 18.0, "centered: (60 - 24) / 2");
        // Node shorter than the track height: the track height clamps to the node.
        let (_, ts2) = s.track_rect(pos, Vec2::new(200.0, 16.0));
        assert_eq!(ts2.y, 16.0);
    }

    #[test]
    fn pill_and_knob_are_round() {
        let s = Switch::new("x");
        let node = Vec2::new(120.0, 40.0);
        // Pill radius = track height / 2.
        assert_eq!(s.track_radius(node), 12.0);
        // Knob radius = (track height - 2*pad) / 2 = (24 - 6) / 2 = 9.
        assert_eq!(s.knob_radius(node), 9.0);
    }

    #[test]
    fn degenerate_node_has_a_non_negative_knob() {
        // A zero-height node must not produce a negative knob diameter.
        let s = Switch::new("x");
        let (_, knob) = s.knob_rect(Vec2::ZERO, Vec2::new(40.0, 0.0));
        assert!(knob.x >= 0.0 && knob.y >= 0.0);
    }

    #[test]
    fn serde_roundtrip_keeps_state_and_label() {
        let s = Switch::new("Music").with_on(true).with_size(60.0, 30.0);
        let ron = ron::to_string(&s).expect("serialize");
        let back: Switch = ron::from_str(&ron).expect("deserialize");
        assert!(back.on);
        assert_eq!(back.label, "Music");
        assert_eq!(back.track_width, 60.0);
        assert_eq!(back.track_height, 30.0);
    }

    #[test]
    fn reflect_roundtrip() {
        let mut s = Switch::new("x");
        assert!(s.set_field("on", ReflectValue::Bool(true)));
        assert!(s.on);
        assert!(s.set_field("track_width", ReflectValue::F32(60.0)));
        assert_eq!(s.track_width, 60.0);
        let fields = s.fields();
        assert!(fields.iter().any(|(n, _)| *n == "on"));
        assert!(fields.iter().any(|(n, _)| *n == "knob_color"));
    }
}
