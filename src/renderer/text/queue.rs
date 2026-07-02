use glam::Vec2;
use glyphon::cosmic_text::Align;

use crate::color::Color as EngineColor;

/// A text draw command.
///
/// # Coordinate space
///
/// `DrawText` / [`TextQueue`] are **screen-space**: `position` is in pixels with
/// the origin at the **top-left** of the window (x → right, y → down), and is
/// **not** affected by the [`Camera`](crate::Camera). To place text relative to a
/// world entity, convert with [`Camera::world_to_screen`](crate::Camera::world_to_screen)
/// first. (World-space sprites use `Transform` + the camera; UI/text does not.)
///
/// By default `position` is the text's **top-left** corner. Use
/// [`DrawText::centered`] (or [`with_anchor`](DrawText::with_anchor)) to treat
/// `position` as the text's center instead — handy for titles/HUD readouts where
/// you'd otherwise eyeball a `-width/2` offset.
///
/// Construct values with [`DrawText::new`] / [`DrawText::centered`] and builder
/// methods instead of struct literals so future layout fields can be added
/// without breaking call sites.
#[derive(Debug, Clone)]
pub struct DrawText {
    pub text: String,
    pub position: Vec2,
    /// Text layout area. `None` means the text extends to the edge of the screen.
    pub bounds: Option<Vec2>,
    /// Font size in pixels.
    pub size: f32,
    /// RGBA (0–255)
    pub color: EngineColor,
    pub align: TextAlign,
    /// How `position` anchors the text box (top-left by default, or its center).
    pub anchor: TextAnchor,
    /// Interprets `[color=#RRGGBB]...[/color]`, `[b]...[/b]`, and `[i]...[/i]` tags.
    pub rich: bool,
    /// When `Some(caret_byte)`, renders as a single non-wrapping line and scrolls
    /// horizontally to keep the caret byte position visible within `bounds`.
    /// Used by `TextInput`. `None` uses the normal line-wrap behaviour.
    pub single_line_caret: Option<usize>,
    /// UI-layer depth. `None` (default) keeps the historical behavior: the text is drawn in the
    /// final on-top text pass, over every UI rect/image and after post-processing — right for HUD
    /// readouts. `Some(z)` composites the text **among** the UI rects/images at that z (same scale
    /// as [`DrawRect::z`](crate::renderer::DrawRect)): a rect drawn above it (greater z) covers it,
    /// which is what widget labels want so an overlay (an open dropdown list, a tooltip, a panel)
    /// actually hides the text underneath. On a z tie the text draws over the rect. Layered text
    /// renders before post-processing, so under an HDR/bloom pipeline it is graded with its widget
    /// (on-top `None` text is not). Set via [`with_z`](DrawText::with_z); the built-in widget
    /// passes set it to their widget's z automatically.
    pub z: Option<f32>,
}

impl DrawText {
    pub fn new(
        text: impl Into<String>,
        position: Vec2,
        size: f32,
        color: impl Into<EngineColor>,
    ) -> Self {
        Self {
            text: text.into(),
            position,
            bounds: None,
            size,
            color: color.into(),
            align: TextAlign::Left,
            anchor: TextAnchor::TopLeft,
            rich: false,
            single_line_caret: None,
            z: None,
        }
    }

    /// Like [`new`](DrawText::new), but `position` is the **center** of the text
    /// (anchor = [`TextAnchor::Center`]) and lines are centered horizontally
    /// ([`TextAlign::Center`]). The shaped text size is measured at render time,
    /// so no manual `-width/2` offset is needed.
    pub fn centered(
        text: impl Into<String>,
        position: Vec2,
        size: f32,
        color: impl Into<EngineColor>,
    ) -> Self {
        Self {
            anchor: TextAnchor::Center,
            align: TextAlign::Center,
            ..Self::new(text, position, size, color)
        }
    }

    pub fn with_bounds(mut self, bounds: Vec2) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Set how `position` anchors the text box (top-left vs. center).
    pub fn with_anchor(mut self, anchor: TextAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn rich(mut self) -> Self {
        self.rich = true;
        self
    }

    /// Render as a single non-wrapping line that scrolls horizontally to keep the
    /// caret (a byte offset into `text`) visible inside `bounds`. Used by `TextInput`.
    pub fn with_single_line_caret(mut self, caret_byte: usize) -> Self {
        self.single_line_caret = Some(caret_byte);
        self
    }

    /// Composite this text among the UI rects/images at `z` instead of the on-top text pass —
    /// a rect drawn above it then actually covers it (see [`DrawText::z`]).
    pub fn with_z(mut self, z: f32) -> Self {
        self.z = Some(z);
        self
    }
}

/// How a [`DrawText`]'s `position` maps to the rendered text box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextAnchor {
    /// `position` is the top-left corner (default — original behavior).
    #[default]
    TopLeft,
    /// `position` is the center; the shaped text is offset by half its measured
    /// size at render time.
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    /// Reading-direction end: the line hugs the **right** edge for LTR text and the **left** edge
    /// for RTL text. Maps to `cosmic_text::Align::End`.
    End,
    /// No explicit alignment — cosmic-text aligns each line by its own resolved direction, so RTL
    /// paragraphs (Hebrew, Arabic) right-align automatically while LTR stays left. The right default
    /// for mixed/bidi UIs. Maps to `set_align(None)`.
    Auto,
}

impl TextAlign {
    /// The glyphon alignment, or `None` for [`TextAlign::Auto`] (direction-natural).
    pub(super) fn to_glyphon(self) -> Option<Align> {
        match self {
            TextAlign::Left => Some(Align::Left),
            TextAlign::Center => Some(Align::Center),
            TextAlign::Right => Some(Align::Right),
            TextAlign::End => Some(Align::End),
            TextAlign::Auto => None,
        }
    }
}

/// Queue that accumulates text draw requests each frame.
///
/// Inserted as a `World` resource. Game systems add entries via `push`;
/// `TextRenderer::render` consumes them and calls `clear`.
#[derive(Default)]
pub struct TextQueue {
    pub(super) items: Vec<DrawText>,
}

impl TextQueue {
    /// Adds a text item to the queue.
    pub fn push(&mut self, item: DrawText) {
        self.items.push(item);
    }

    /// Removes and returns the **layered** items (`z = Some`), leaving the on-top (`z = None`)
    /// items queued for the final text pass. Called once per frame by the render orchestration,
    /// which composites the layered items among the UI rects by z.
    pub(crate) fn take_layered(&mut self) -> Vec<DrawText> {
        let (layered, top): (Vec<_>, Vec<_>) = std::mem::take(&mut self.items)
            .into_iter()
            .partition(|t| t.z.is_some());
        self.items = top;
        layered
    }

    /// Removes all items.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Iterator over all queued items.
    pub fn iter(&self) -> impl Iterator<Item = &DrawText> {
        self.items.iter()
    }

    /// Number of items in the queue.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
