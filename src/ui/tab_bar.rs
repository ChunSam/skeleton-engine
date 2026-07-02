use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

/// A tab bar — a horizontal row of equal-width tab headers, exactly one active.
///
/// Attach it to an entity alongside a [`UiNode`](crate::ui::UiNode); the node's rect is the header
/// row, divided into equal-width tabs separated by [`gap`](Self::gap) pixels. Clicking a header
/// selects it and emits [`UiEvent::TabChanged`](crate::ui::UiEvent::TabChanged) when the selection
/// actually changed (re-picking the current tab is silent).
///
/// The bar is **headers only** — switching the content is the game's job (read the event, or poll
/// [`selected_index`](Self::selected_index), and toggle the per-tab panels'
/// [`UiNode::visible`](crate::ui::UiNode)). That keeps the widget genre-agnostic and the content
/// fully under the game's control; see the `ui_tabs` example for the wiring.
///
/// Keyboard/gamepad: the widget is focusable (Tab / D-pad / stick); **←/→** (or D-pad Left/Right)
/// steps the active tab directly, clamped like a [`Slider`](crate::ui::Slider) nudge.
///
/// # Example
/// ```
/// # use engine::ui::TabBar;
/// let tb = TabBar::new(["Stats", "Inventory", "Options"]).with_selected(1);
/// assert_eq!(tb.selected_tab(), Some("Inventory"));
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TabBar {
    /// The tab titles, rendered left-to-right.
    pub tabs: Vec<String>,
    /// Index of the active tab. Read via [`selected_index`](Self::selected_index), which clamps
    /// into range; the raw field is left as set so an out-of-range value is not silently mutated.
    pub selected: usize,
    /// Inactive tab background color.
    pub bg_color: Color,
    /// Active tab background color.
    pub active_color: Color,
    /// Background of the hovered inactive tab.
    pub hover_color: Color,
    /// Inactive tab title color.
    pub text_color: Color,
    /// Active tab title color.
    pub active_text_color: Color,
    /// Title font size in pixels.
    pub font_size: f32,
    /// Corner radius in pixels for the tab headers (`0.0` = sharp).
    pub corner_radius: f32,
    /// Horizontal gap in pixels between adjacent tab headers.
    pub gap: f32,
}

impl Default for TabBar {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            selected: 0,
            bg_color: Color::rgba(0.16, 0.16, 0.20, 1.0),
            active_color: Color::rgba(0.28, 0.56, 0.90, 1.0),
            hover_color: Color::rgba(0.24, 0.24, 0.30, 1.0),
            text_color: Color::rgba_u8(170, 172, 184, 255),
            active_text_color: Color::rgba_u8(240, 242, 248, 255),
            font_size: 16.0,
            corner_radius: 0.0,
            gap: 2.0,
        }
    }
}

impl TabBar {
    /// A tab bar over `tabs` with the first tab active and the default style.
    pub fn new<I, S>(tabs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            tabs: tabs.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Select tab `index` (clamped into range on read). Builder form.
    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Set the inactive and active tab background colors. Builder form.
    pub fn with_colors(mut self, bg_color: Color, active_color: Color) -> Self {
        self.bg_color = bg_color;
        self.active_color = active_color;
        self
    }

    /// Set the hovered-inactive-tab background color. Builder form.
    pub fn with_hover_color(mut self, color: Color) -> Self {
        self.hover_color = color;
        self
    }

    /// Set the inactive and active title colors. Builder form.
    pub fn with_text_colors(mut self, text_color: Color, active_text_color: Color) -> Self {
        self.text_color = text_color;
        self.active_text_color = active_text_color;
        self
    }

    /// Set the title font size in pixels. Builder form.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Round the tab-header corners to `radius` pixels. Builder form.
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Set the horizontal gap between adjacent headers in pixels. Builder form.
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// The active index clamped into `tabs` range — what the renderer and
    /// [`selected_tab`](Self::selected_tab) use. `0` when `tabs` is empty.
    pub fn selected_index(&self) -> usize {
        self.selected.min(self.tabs.len().saturating_sub(1))
    }

    /// The active tab's title, or `None` when `tabs` is empty.
    pub fn selected_tab(&self) -> Option<&str> {
        self.tabs.get(self.selected_index()).map(String::as_str)
    }

    /// Width of one tab header for a bar `node_width` wide (equal split minus the gaps, floored
    /// at zero). `0.0` when there are no tabs.
    pub fn tab_width(&self, node_width: f32) -> f32 {
        let n = self.tabs.len();
        if n == 0 {
            return 0.0;
        }
        ((node_width - self.gap * (n as f32 - 1.0)) / n as f32).max(0.0)
    }

    /// Top-left and size of tab header `index` for a bar drawn at `(pos, node_size)`. The same
    /// geometry drives rendering and click resolution, so they can never disagree.
    pub fn tab_rect(&self, index: usize, pos: Vec2, node_size: Vec2) -> (Vec2, Vec2) {
        let w = self.tab_width(node_size.x);
        (
            Vec2::new(pos.x + index as f32 * (w + self.gap), pos.y),
            Vec2::new(w, node_size.y),
        )
    }

    /// The tab header under `cursor` for a bar drawn at `(pos, node_size)`, or `None` when the
    /// cursor is outside every header (the gaps between headers select nothing).
    pub fn tab_at(&self, cursor: Vec2, pos: Vec2, node_size: Vec2) -> Option<usize> {
        (0..self.tabs.len()).find(|&i| {
            let (tpos, tsize) = self.tab_rect(i, pos, node_size);
            cursor.x >= tpos.x
                && cursor.x < tpos.x + tsize.x
                && cursor.y >= tpos.y
                && cursor.y < tpos.y + tsize.y
        })
    }
}

impl Reflect for TabBar {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("selected", ReflectValue::I32(self.selected as i32)),
            ("font_size", ReflectValue::F32(self.font_size)),
            ("corner_radius", ReflectValue::F32(self.corner_radius)),
            ("gap", ReflectValue::F32(self.gap)),
            ("bg_color", ReflectValue::Color(self.bg_color.to_array())),
            (
                "active_color",
                ReflectValue::Color(self.active_color.to_array()),
            ),
            (
                "hover_color",
                ReflectValue::Color(self.hover_color.to_array()),
            ),
            (
                "text_color",
                ReflectValue::Color(self.text_color.to_array()),
            ),
            (
                "active_text_color",
                ReflectValue::Color(self.active_text_color.to_array()),
            ),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("selected", ReflectValue::I32(v)) => {
                self.selected = v.max(0) as usize;
                true
            }
            ("font_size", ReflectValue::F32(v)) => {
                self.font_size = v;
                true
            }
            ("corner_radius", ReflectValue::F32(v)) => {
                self.corner_radius = v;
                true
            }
            ("gap", ReflectValue::F32(v)) => {
                self.gap = v;
                true
            }
            ("bg_color", ReflectValue::Color(c)) => {
                self.bg_color = Color::from(c);
                true
            }
            ("active_color", ReflectValue::Color(c)) => {
                self.active_color = Color::from(c);
                true
            }
            ("hover_color", ReflectValue::Color(c)) => {
                self.hover_color = Color::from(c);
                true
            }
            ("text_color", ReflectValue::Color(c)) => {
                self.text_color = Color::from(c);
                true
            }
            ("active_text_color", ReflectValue::Color(c)) => {
                self.active_text_color = Color::from(c);
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "TabBar"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_clamps_on_read_without_mutating() {
        let tb = TabBar::new(["a", "b", "c"]).with_selected(10);
        assert_eq!(tb.selected, 10, "raw field left as set");
        assert_eq!(tb.selected_index(), 2, "clamped to the last tab");
        assert_eq!(tb.selected_tab(), Some("c"));

        let empty = TabBar::new(Vec::<String>::new());
        assert_eq!(empty.selected_index(), 0);
        assert_eq!(empty.selected_tab(), None);
    }

    #[test]
    fn tab_width_splits_evenly_minus_gaps() {
        // 3 tabs, gap 2 → (302 - 4) / 3 = 99.333…
        let tb = TabBar::new(["a", "b", "c"]);
        assert!((tb.tab_width(302.0) - 99.333_336).abs() < 1e-3);

        let empty = TabBar::new(Vec::<String>::new());
        assert_eq!(empty.tab_width(302.0), 0.0);

        // Gaps wider than the node floor at zero instead of going negative.
        let cramped = TabBar::new(["a", "b"]).with_gap(50.0);
        assert_eq!(cramped.tab_width(40.0), 0.0);
    }

    #[test]
    fn tab_at_maps_cursor_to_headers_and_gaps_to_none() {
        // 3 tabs of 100 px + 2 px gaps in a 304-wide bar at (10, 10):
        // headers at x 10..110, 112..212, 214..314.
        let tb = TabBar::new(["a", "b", "c"]);
        let pos = Vec2::new(10.0, 10.0);
        let size = Vec2::new(304.0, 30.0);
        assert_eq!(tb.tab_at(Vec2::new(50.0, 20.0), pos, size), Some(0));
        assert_eq!(tb.tab_at(Vec2::new(150.0, 20.0), pos, size), Some(1));
        assert_eq!(tb.tab_at(Vec2::new(300.0, 20.0), pos, size), Some(2));
        assert_eq!(tb.tab_at(Vec2::new(111.0, 20.0), pos, size), None, "gap");
        assert_eq!(
            tb.tab_at(Vec2::new(50.0, 50.0), pos, size),
            None,
            "below the bar"
        );
    }

    #[test]
    fn builders_set_fields() {
        let tb = TabBar::new(["x"])
            .with_selected(0)
            .with_colors(Color::RED, Color::BLUE)
            .with_hover_color(Color::GREEN)
            .with_text_colors(Color::WHITE, Color::BLACK)
            .with_font_size(20.0)
            .with_corner_radius(6.0)
            .with_gap(4.0);
        assert_eq!(tb.tabs, vec!["x"]);
        assert_eq!(tb.bg_color, Color::RED);
        assert_eq!(tb.active_color, Color::BLUE);
        assert_eq!(tb.hover_color, Color::GREEN);
        assert_eq!(tb.text_color, Color::WHITE);
        assert_eq!(tb.active_text_color, Color::BLACK);
        assert_eq!(tb.font_size, 20.0);
        assert_eq!(tb.corner_radius, 6.0);
        assert_eq!(tb.gap, 4.0);
    }

    #[test]
    fn serde_roundtrip_keeps_tabs_and_selection() {
        let tb = TabBar::new(["Stats", "Options"]).with_selected(1);
        let ron = ron::to_string(&tb).expect("serialize");
        let back: TabBar = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.tabs, vec!["Stats", "Options"]);
        assert_eq!(back.selected, 1);
    }

    #[test]
    fn reflect_roundtrip() {
        let mut tb = TabBar::new(["a", "b"]);
        assert!(tb.set_field("selected", ReflectValue::I32(1)));
        assert_eq!(tb.selected, 1);
        assert!(tb.set_field("selected", ReflectValue::I32(-3)));
        assert_eq!(tb.selected, 0, "negative reflect writes clamp to 0");
        assert!(tb.set_field("gap", ReflectValue::F32(6.0)));
        assert_eq!(tb.gap, 6.0);
        let fields = tb.fields();
        assert!(fields.iter().any(|(n, _)| *n == "selected"));
        assert!(fields.iter().any(|(n, _)| *n == "active_color"));
    }
}
