use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

/// Default hover time (seconds) before a [`Tooltip`] appears.
pub const DEFAULT_TOOLTIP_DELAY_SECS: f32 = 0.4;
/// Default fade-in time (seconds) once the delay has elapsed. `0.0` = appear instantly.
pub const DEFAULT_TOOLTIP_FADE_SECS: f32 = 0.1;
/// Z at which every tooltip draws — far above the recommended `0.0..=1.0` [`UiNode`] z range, so a
/// tooltip composites over all normal UI rects. (Text draws in its own later pass; the tooltip pass
/// runs last inside [`UiSystem`], so tooltip text also lands on top of other widget text.)
///
/// [`UiNode`]: crate::ui::UiNode
/// [`UiSystem`]: crate::ui::UiSystem
pub const TOOLTIP_Z: f32 = 100.0;

/// A hover tooltip — a small text popup shown after the cursor rests on a widget.
///
/// Attach it to any entity that has a [`UiNode`](crate::ui::UiNode) (a `Button`, `ProgressBar`,
/// `Label`, `Panel`, …). While the cursor is inside the node's rect — and the point is not covered
/// by a pointer-opaque widget drawn above it — [`UiSystem`](crate::ui::UiSystem) accumulates hover
/// time; once it passes [`delay_secs`](Self::delay_secs) the tooltip fades in next to the cursor
/// (offset by [`offset`](Self::offset), clamped to the viewport so it never runs off screen).
/// Moving the cursor off the widget resets the delay.
///
/// The background box auto-sizes from a **shaped-width estimate** (≈ 0.5 em per ASCII char, 1 em
/// per CJK/full-width char, 1.2 × font-size line height, `\n` starts a new line). Exact glyph
/// shaping happens later, at render time, so the estimate can be a little loose for unusual
/// scripts — pass [`with_size`](Self::with_size) to pin the content box exactly.
///
/// # Example
/// ```
/// # use engine::ui::Tooltip;
/// let tip = Tooltip::new("Attack: 12\nCrit: 5%").with_delay(0.25);
/// let two_lines = tip.estimated_size();
/// assert_eq!(two_lines.y, 2.0 * tip.font_size * 1.2); // two lines high
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tooltip {
    /// Tooltip body. `\n` breaks lines. An empty string disables the tooltip.
    pub text: String,
    /// Hover time (seconds) before the tooltip appears. Negative values behave as `0.0`.
    pub delay_secs: f32,
    /// Fade-in time (seconds) after the delay elapses. `0.0` = fully visible immediately.
    pub fade_secs: f32,
    /// Font size in pixels.
    pub font_size: f32,
    /// Body text color.
    pub text_color: Color,
    /// Background box color.
    pub bg_color: Color,
    /// Corner radius in pixels for the background box (`0.0` = sharp).
    pub corner_radius: f32,
    /// Outline width in pixels drawn on top of the box (`0.0` = no border).
    pub border: f32,
    /// Outline color when [`border`](Self::border) `> 0`.
    pub border_color: Color,
    /// Inner padding in pixels between the box edge and the text.
    pub padding: f32,
    /// Pixel offset from the cursor to the box's top-left (default down-right, OS style).
    pub offset: Vec2,
    /// Exact content size (excluding padding) overriding the text-size estimate. `None` = auto.
    pub size_override: Option<Vec2>,
    /// Accumulated hover time this hover (transient — reset when the cursor leaves; not saved).
    #[serde(skip)]
    hovered_secs: f32,
}

impl Default for Tooltip {
    fn default() -> Self {
        Self {
            text: String::new(),
            delay_secs: DEFAULT_TOOLTIP_DELAY_SECS,
            fade_secs: DEFAULT_TOOLTIP_FADE_SECS,
            font_size: 14.0,
            text_color: Color::rgba(0.95, 0.95, 0.97, 1.0),
            bg_color: Color::rgba(0.09, 0.09, 0.12, 0.95),
            corner_radius: 4.0,
            border: 0.0,
            border_color: Color::rgba(1.0, 1.0, 1.0, 0.25),
            padding: 6.0,
            offset: Vec2::new(14.0, 18.0),
            size_override: None,
            hovered_secs: 0.0,
        }
    }
}

impl Tooltip {
    /// A tooltip showing `text` with the default dark style and delay.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// Set the hover delay in seconds. Builder form.
    pub fn with_delay(mut self, secs: f32) -> Self {
        self.delay_secs = secs;
        self
    }

    /// Set the fade-in time in seconds (`0.0` = instant). Builder form.
    pub fn with_fade(mut self, secs: f32) -> Self {
        self.fade_secs = secs;
        self
    }

    /// Set the font size in pixels. Builder form.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the text and background colors. Builder form.
    pub fn with_colors(mut self, text_color: Color, bg_color: Color) -> Self {
        self.text_color = text_color;
        self.bg_color = bg_color;
        self
    }

    /// Round the background box corners to `radius` pixels. Builder form.
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Draw an outline of `width` pixels in `color` on top of the box. Builder form.
    pub fn with_border(mut self, width: f32, color: Color) -> Self {
        self.border = width;
        self.border_color = color;
        self
    }

    /// Set the inner padding in pixels. Builder form.
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set the cursor→box offset in pixels. Builder form.
    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    /// Pin the content size (excluding padding) exactly, overriding the text estimate.
    /// Builder form.
    pub fn with_size(mut self, size: Vec2) -> Self {
        self.size_override = Some(size);
        self
    }

    /// Accumulate hover time. Called by the UI pass each frame the cursor rests on the widget.
    pub fn tick_hover(&mut self, dt: f32) {
        self.hovered_secs += dt.max(0.0);
    }

    /// Reset the hover timer. Called by the UI pass when the cursor leaves the widget.
    pub fn reset_hover(&mut self) {
        self.hovered_secs = 0.0;
    }

    /// Accumulated hover time (seconds) for the current hover.
    pub fn hovered_secs(&self) -> f32 {
        self.hovered_secs
    }

    /// True once the hover delay has elapsed (the pass has been accumulating hover time).
    pub fn is_showing(&self) -> bool {
        self.hovered_secs > 0.0 && self.hovered_secs >= self.delay_secs.max(0.0)
    }

    /// Fade-in alpha in `0.0..=1.0`: `0.0` right when the delay elapses, ramping to `1.0` over
    /// [`fade_secs`](Self::fade_secs). With `fade_secs <= 0.0` it is `1.0` as soon as
    /// [`is_showing`](Self::is_showing) is true.
    pub fn fade_alpha(&self) -> f32 {
        if !self.is_showing() {
            return 0.0;
        }
        if self.fade_secs <= 0.0 {
            return 1.0;
        }
        ((self.hovered_secs - self.delay_secs.max(0.0)) / self.fade_secs).clamp(0.0, 1.0)
    }

    /// The content size (excluding padding) the box will use: [`size_override`](Self::size_override)
    /// if set, else the text-size estimate (see the type docs for the heuristic).
    pub fn estimated_size(&self) -> Vec2 {
        if let Some(size) = self.size_override {
            return size;
        }
        let mut max_w: f32 = 0.0;
        let mut lines = 0u32;
        for line in self.text.split('\n') {
            lines += 1;
            let w: f32 = line.chars().map(char_advance_em).sum::<f32>() * self.font_size;
            max_w = max_w.max(w);
        }
        // Line height matches the text renderer's Metrics (1.2 × font size).
        Vec2::new(max_w, lines.max(1) as f32 * self.font_size * 1.2)
    }
}

/// Rough per-character advance in em for the width estimate: full-width scripts (CJK ideographs,
/// Hangul, kana, full-width forms) ≈ 1 em; everything else ≈ 0.5 em (the average advance of
/// proportional Latin text).
fn char_advance_em(c: char) -> f32 {
    match c as u32 {
        // Hangul Jamo, CJK radicals..ideographs (incl. kana + symbols), Hangul syllables,
        // CJK compatibility ideographs, full-width forms.
        0x1100..=0x11FF | 0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFF60 => {
            1.0
        }
        _ => 0.5,
    }
}

impl Reflect for Tooltip {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("text", ReflectValue::String(self.text.clone())),
            ("delay_secs", ReflectValue::F32(self.delay_secs)),
            ("fade_secs", ReflectValue::F32(self.fade_secs)),
            ("font_size", ReflectValue::F32(self.font_size)),
            ("padding", ReflectValue::F32(self.padding)),
            ("corner_radius", ReflectValue::F32(self.corner_radius)),
            ("offset", ReflectValue::Vec2(self.offset)),
            (
                "text_color",
                ReflectValue::Color(self.text_color.to_array()),
            ),
            ("bg_color", ReflectValue::Color(self.bg_color.to_array())),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("text", ReflectValue::String(s)) => {
                self.text = s;
                true
            }
            ("delay_secs", ReflectValue::F32(v)) => {
                self.delay_secs = v;
                true
            }
            ("fade_secs", ReflectValue::F32(v)) => {
                self.fade_secs = v;
                true
            }
            ("font_size", ReflectValue::F32(v)) => {
                self.font_size = v;
                true
            }
            ("padding", ReflectValue::F32(v)) => {
                self.padding = v;
                true
            }
            ("corner_radius", ReflectValue::F32(v)) => {
                self.corner_radius = v;
                true
            }
            ("offset", ReflectValue::Vec2(v)) => {
                self.offset = v;
                true
            }
            ("text_color", ReflectValue::Color(c)) => {
                self.text_color = Color::from(c);
                true
            }
            ("bg_color", ReflectValue::Color(c)) => {
                self.bg_color = Color::from(c);
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "Tooltip"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_delay_gates_visibility() {
        let mut tip = Tooltip::new("hi").with_delay(0.4);
        assert!(!tip.is_showing(), "not hovered yet");
        tip.tick_hover(0.2);
        assert!(!tip.is_showing(), "0.2s < 0.4s delay");
        tip.tick_hover(0.3);
        assert!(tip.is_showing(), "0.5s >= 0.4s delay");
    }

    #[test]
    fn losing_hover_resets_the_timer() {
        let mut tip = Tooltip::new("hi").with_delay(0.1);
        tip.tick_hover(0.5);
        assert!(tip.is_showing());
        tip.reset_hover();
        assert!(!tip.is_showing());
        assert_eq!(tip.hovered_secs(), 0.0);
    }

    #[test]
    fn zero_delay_shows_on_the_first_hover_tick() {
        let mut tip = Tooltip::new("hi").with_delay(0.0);
        assert!(!tip.is_showing(), "no hover yet");
        tip.tick_hover(1.0 / 60.0);
        assert!(tip.is_showing(), "first hovered frame shows immediately");
        // A negative delay clamps to zero rather than inverting the gate.
        let mut neg = Tooltip::new("hi").with_delay(-1.0);
        neg.tick_hover(1.0 / 60.0);
        assert!(neg.is_showing());
    }

    #[test]
    fn fade_alpha_ramps_after_the_delay() {
        let mut tip = Tooltip::new("hi").with_delay(0.4).with_fade(0.1);
        tip.tick_hover(0.3);
        assert_eq!(tip.fade_alpha(), 0.0, "before the delay");
        tip.tick_hover(0.15); // 0.45s = delay + 0.05 → halfway through the fade
        assert!(
            (tip.fade_alpha() - 0.5).abs() < 1e-4,
            "{}",
            tip.fade_alpha()
        );
        tip.tick_hover(0.2); // well past delay + fade
        assert_eq!(tip.fade_alpha(), 1.0);

        let mut instant = Tooltip::new("hi").with_delay(0.1).with_fade(0.0);
        instant.tick_hover(0.1);
        assert_eq!(instant.fade_alpha(), 1.0, "fade 0 = instant full alpha");
    }

    #[test]
    fn estimated_size_scales_with_text_font_and_lines() {
        let short = Tooltip::new("hi").with_font_size(14.0);
        let long = Tooltip::new("hello world").with_font_size(14.0);
        assert!(long.estimated_size().x > short.estimated_size().x);

        // CJK chars count a full em, ASCII half — same char count, wider box.
        let ascii = Tooltip::new("ab").with_font_size(14.0);
        let cjk = Tooltip::new("한글").with_font_size(14.0);
        assert!((cjk.estimated_size().x - 2.0 * ascii.estimated_size().x).abs() < 1e-4);

        // Two lines: height doubles, width is the widest line.
        let two = Tooltip::new("hello\nhi").with_font_size(14.0);
        let one = Tooltip::new("hello").with_font_size(14.0);
        assert!((two.estimated_size().y - 2.0 * one.estimated_size().y).abs() < 1e-4);
        assert_eq!(two.estimated_size().x, one.estimated_size().x);

        // Bigger font scales both axes.
        let big = Tooltip::new("hello").with_font_size(28.0);
        assert!((big.estimated_size().x - 2.0 * one.estimated_size().x).abs() < 1e-4);
    }

    #[test]
    fn size_override_wins_over_the_estimate() {
        let tip = Tooltip::new("a very long tooltip line").with_size(Vec2::new(40.0, 10.0));
        assert_eq!(tip.estimated_size(), Vec2::new(40.0, 10.0));
    }

    #[test]
    fn builders_set_fields() {
        let tip = Tooltip::new("t")
            .with_delay(0.2)
            .with_fade(0.3)
            .with_font_size(18.0)
            .with_colors(Color::WHITE, Color::RED)
            .with_corner_radius(7.0)
            .with_border(2.0, Color::BLUE)
            .with_padding(9.0)
            .with_offset(Vec2::new(-4.0, 4.0));
        assert_eq!(tip.text, "t");
        assert_eq!(tip.delay_secs, 0.2);
        assert_eq!(tip.fade_secs, 0.3);
        assert_eq!(tip.font_size, 18.0);
        assert_eq!(tip.text_color, Color::WHITE);
        assert_eq!(tip.bg_color, Color::RED);
        assert_eq!(tip.corner_radius, 7.0);
        assert_eq!(tip.border, 2.0);
        assert_eq!(tip.border_color, Color::BLUE);
        assert_eq!(tip.padding, 9.0);
        assert_eq!(tip.offset, Vec2::new(-4.0, 4.0));
    }

    #[test]
    fn serde_roundtrip_drops_hover_state() {
        let mut tip = Tooltip::new("saved")
            .with_delay(0.25)
            .with_corner_radius(3.0);
        tip.tick_hover(5.0);
        assert!(tip.is_showing());
        let ron = ron::to_string(&tip).expect("serialize");
        let back: Tooltip = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.text, "saved");
        assert_eq!(back.delay_secs, 0.25);
        assert_eq!(back.hovered_secs(), 0.0, "hover state is transient");
        assert!(!back.is_showing());
    }

    #[test]
    fn reflect_roundtrip() {
        let mut tip = Tooltip::new("a");
        assert!(tip.set_field("text", ReflectValue::String("b".into())));
        assert_eq!(tip.text, "b");
        assert!(tip.set_field("delay_secs", ReflectValue::F32(0.7)));
        assert!((tip.delay_secs - 0.7).abs() < f32::EPSILON);
        assert!(tip.set_field("offset", ReflectValue::Vec2(Vec2::new(1.0, 2.0))));
        assert_eq!(tip.offset, Vec2::new(1.0, 2.0));
        let fields = tip.fields();
        assert!(fields.iter().any(|(n, _)| *n == "text"));
        assert!(fields.iter().any(|(n, _)| *n == "bg_color"));
    }
}
