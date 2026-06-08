use glam::Vec2;
use glyphon::{
    cosmic_text::Align, Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution,
    Shaping, Style, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer as GlyphonTextRenderer, Viewport, Weight, Wrap,
};
use wgpu::{
    CommandEncoder, Device, LoadOp, MultisampleState, Operations, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureFormat, TextureView,
};

use crate::color::Color as EngineColor;
use crate::ecs::World;
use crate::resources::DisplayScaleFactor;

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
}

/// X offset (buffer-local px) of the caret at byte `caret_byte` for a single-line
/// buffer: the x of the first glyph starting at/after the caret, or the line width
/// when the caret is past the last glyph. Assumes left alignment.
fn caret_x(buf: &Buffer, caret_byte: usize) -> f32 {
    // Single-line buffer → first (only) layout run.
    let Some(run) = buf.layout_runs().next() else {
        return 0.0;
    };
    for glyph in run.glyphs.iter() {
        if glyph.start >= caret_byte {
            return glyph.x;
        }
    }
    run.line_w
}

/// How a [`DrawText`]'s `position` maps to the rendered text box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAnchor {
    /// `position` is the top-left corner (default — original behavior).
    #[default]
    TopLeft,
    /// `position` is the center; the shaped text is offset by half its measured
    /// size at render time.
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl TextAlign {
    fn to_glyphon(self) -> Align {
        match self {
            TextAlign::Left => Align::Left,
            TextAlign::Center => Align::Center,
            TextAlign::Right => Align::Right,
        }
    }
}

/// Queue that accumulates text draw requests each frame.
///
/// Inserted as a `World` resource. Game systems add entries via `push`;
/// `TextRenderer::render` consumes them and calls `clear`.
#[derive(Default)]
pub struct TextQueue {
    items: Vec<DrawText>,
}

impl TextQueue {
    /// Adds a text item to the queue.
    pub fn push(&mut self, item: DrawText) {
        self.items.push(item);
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

/// Text renderer backed by glyphon 0.6.
///
/// ## Ownership layout
/// - `Cache` is created first and shared with `TextAtlas` / `Viewport`.
///   (`TextAtlas::new` requires `&Cache`; `TextRenderer` retains ownership of `Cache`.)
/// - `Viewport::update(queue, Resolution{w,h})` refreshes the GPU uniform each frame.
pub struct TextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    /// Cache is created first and shared with atlas / viewport (glyphon 0.6 requirement).
    /// TextAtlas clones the Cache internally, so this field does not have to be kept,
    /// but we retain explicit ownership here.
    #[allow(dead_code)]
    cache: Cache,
    atlas: TextAtlas,
    viewport: Viewport,
    renderer: GlyphonTextRenderer,
}

impl TextRenderer {
    /// Initialises GPU resources.
    ///
    /// If `font_data` is non-empty the TTF/OTF bytes are loaded into fontdb.
    /// Otherwise glyphon's system-font fallback is used.
    pub fn new(device: &Device, queue: &Queue, format: TextureFormat, font_data: &[u8]) -> Self {
        let mut font_system = FontSystem::new();
        if !font_data.is_empty() {
            font_system.db_mut().load_font_data(font_data.to_vec());
        }

        let swash_cache = SwashCache::new();

        // 2. Cache first, then Atlas / Viewport
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);

        // 3. TextRenderer (glyphon internal GlyphonTextRenderer)
        let renderer =
            GlyphonTextRenderer::new(&mut atlas, device, MultisampleState::default(), None);

        Self {
            font_system,
            swash_cache,
            cache,
            atlas,
            viewport,
            renderer,
        }
    }

    /// Pulls the `TextQueue` from the ECS `World` and renders all text.
    ///
    /// - Returns immediately without opening a render pass if the queue is empty.
    /// - Composites over the sprite pass with `LoadOp::Load`.
    /// - Clears the queue after rendering.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        world: &mut World,
        w: u32,
        h: u32,
    ) {
        // Pull items from the queue; early-return if empty.
        let items: Vec<DrawText> = match world.resource_mut::<TextQueue>() {
            Some(q) if !q.is_empty() => {
                let taken = q.items.clone();
                q.clear();
                taken
            }
            _ => return,
        };
        let scale_factor = world
            .resource::<DisplayScaleFactor>()
            .map(|s| s.0)
            .unwrap_or(1.0)
            .max(1.0);

        // Update Viewport (writes the resolution to the GPU uniform each frame)
        self.viewport.update(
            queue,
            Resolution {
                width: w,
                height: h,
            },
        );

        // Convert each DrawText into a glyphon Buffer.
        // - `Buffer::set_size` takes `(font_system, Option<f32>, Option<f32>)` in cosmic-text.
        // - `set_text` takes `(font_system, text, attrs, shaping)`.
        let buffers: Vec<(Buffer, DrawText, f32)> = items
            .into_iter()
            .map(|d| {
                let size = d.size * scale_factor;
                let position = d.position * scale_factor;
                let bounds = d.bounds.map(|b| b * scale_factor);
                let single_line = d.single_line_caret;
                let metrics = Metrics::new(size, size * 1.2); // line_height = 1.2× size
                let mut buf = Buffer::new(&mut self.font_system, metrics);
                // Single-line (TextInput): expand to unlimited width + no wrap, then
                // scroll horizontally below. Otherwise wrap at the bounds width.
                let width = if single_line.is_some() {
                    None
                } else {
                    Some(bounds.map_or(w as f32 - position.x, |b| b.x.max(0.0)))
                };
                buf.set_size(
                    &mut self.font_system,
                    width,
                    Some(bounds.map_or(h as f32 - position.y, |b| b.y.max(0.0))),
                );
                buf.set_wrap(
                    &mut self.font_system,
                    if single_line.is_some() {
                        Wrap::None
                    } else {
                        Wrap::WordOrGlyph
                    },
                );
                let default_attrs = Attrs::new().family(Family::SansSerif);
                if d.rich {
                    let rich = parse_rich_text(&d.text, default_attrs);
                    let spans: Vec<(&str, Attrs<'_>)> =
                        rich.iter().map(|(s, attrs)| (s.as_str(), *attrs)).collect();
                    buf.set_rich_text(
                        &mut self.font_system,
                        spans,
                        default_attrs,
                        Shaping::Advanced,
                    );
                } else {
                    buf.set_text(
                        &mut self.font_system,
                        &d.text,
                        default_attrs,
                        Shaping::Advanced,
                    );
                }
                for line in &mut buf.lines {
                    line.set_align(Some(d.align.to_glyphon()));
                }
                buf.shape_until_scroll(&mut self.font_system, false);
                // Single-line: compute horizontal scroll offset to keep the caret visible.
                // (If the caret exceeds field right edge minus margin, shift left by that amount.)
                let scroll = match single_line {
                    Some(caret_byte) => {
                        let field_w = bounds.map_or(w as f32 - position.x, |b| b.x.max(0.0));
                        let margin = size; // one-glyph margin so the caret doesn't hug the right edge
                        (caret_x(&buf, caret_byte) - (field_w - margin)).max(0.0)
                    }
                    None => 0.0,
                };
                // Anchor: when centered, shift the effective top-left by half the
                // shaped text size so `position` is the text's center. Measured
                // from the shaped buffer (line_height = 1.2 × size, see Metrics).
                let anchor_offset = match d.anchor {
                    TextAnchor::TopLeft => Vec2::ZERO,
                    TextAnchor::Center => {
                        let mut max_w = 0.0_f32;
                        let mut lines = 0.0_f32;
                        for run in buf.layout_runs() {
                            max_w = max_w.max(run.line_w);
                            lines += 1.0;
                        }
                        Vec2::new(max_w * 0.5, lines * size * 1.2 * 0.5)
                    }
                };
                let mut scaled = d;
                scaled.position = position - anchor_offset;
                scaled.bounds = bounds;
                scaled.size = size;
                (buf, scaled, scroll)
            })
            .collect();

        let text_areas: Vec<TextArea<'_>> = buffers
            .iter()
            .map(|(buf, d, scroll)| TextArea {
                buffer: buf,
                left: d.position.x - *scroll,
                top: d.position.y,
                scale: 1.0,
                bounds: TextBounds {
                    left: d.position.x as i32,
                    top: d.position.y as i32,
                    right: d
                        .bounds
                        .map_or(w as i32, |b| (d.position.x + b.x).ceil() as i32),
                    bottom: d
                        .bounds
                        .map_or(h as i32, |b| (d.position.y + b.y).ceil() as i32),
                },
                default_color: {
                    let [r, g, b, a] = d.color.to_u8();
                    Color::rgba(r, g, b, a)
                },
                custom_glyphs: &[],
            })
            .collect();

        // prepare — rasterize glyphs + upload to GPU buffers
        let _ = self.renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        );

        // Text render pass — composite over sprites with LoadOp::Load
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("text pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let _ = self.renderer.render(&self.atlas, &self.viewport, &mut pass);
        }

        // Trim unused glyphs from the atlas for the next frame
        self.atlas.trim();
    }
}

fn parse_rich_text<'a>(text: &str, default_attrs: Attrs<'a>) -> Vec<(String, Attrs<'a>)> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut color_stack: Vec<Option<Color>> = vec![None];
    let mut bold_depth = 0usize;
    let mut italic_depth = 0usize;
    let mut i = 0usize;

    while i < text.len() {
        let rest = &text[i..];
        let tag = if rest.starts_with("[b]") {
            Some(("b", 3))
        } else if rest.starts_with("[/b]") {
            Some(("/b", 4))
        } else if rest.starts_with("[i]") {
            Some(("i", 3))
        } else if rest.starts_with("[/i]") {
            Some(("/i", 4))
        } else if rest.starts_with("[/color]") {
            Some(("/color", 8))
        } else {
            parse_color_tag(rest).map(|(_, len)| ("color", len))
        };

        if let Some((name, len)) = tag {
            if !current.is_empty() {
                let attrs = rich_attrs(
                    default_attrs,
                    *color_stack.last().unwrap(),
                    bold_depth,
                    italic_depth,
                );
                spans.push((std::mem::take(&mut current), attrs));
            }
            match name {
                "b" => bold_depth += 1,
                "/b" => bold_depth = bold_depth.saturating_sub(1),
                "i" => italic_depth += 1,
                "/i" => italic_depth = italic_depth.saturating_sub(1),
                "color" => color_stack.push(parse_color_tag(rest).and_then(|(c, _)| c)),
                "/color" if color_stack.len() > 1 => {
                    color_stack.pop();
                }
                "/color" => {}
                _ => {}
            }
            i += len;
        } else {
            let ch = rest.chars().next().unwrap();
            current.push(ch);
            i += ch.len_utf8();
        }
    }

    if !current.is_empty() || spans.is_empty() {
        let attrs = rich_attrs(
            default_attrs,
            *color_stack.last().unwrap(),
            bold_depth,
            italic_depth,
        );
        spans.push((current, attrs));
    }
    spans
}

fn rich_attrs<'a>(
    default_attrs: Attrs<'a>,
    color: Option<Color>,
    bold_depth: usize,
    italic_depth: usize,
) -> Attrs<'a> {
    let mut attrs = default_attrs;
    if let Some(color) = color {
        attrs = attrs.color(color);
    }
    if bold_depth > 0 {
        attrs = attrs.weight(Weight::BOLD);
    }
    if italic_depth > 0 {
        attrs = attrs.style(Style::Italic);
    }
    attrs
}

fn parse_color_tag(rest: &str) -> Option<(Option<Color>, usize)> {
    let value = rest.strip_prefix("[color=")?;
    let end = value.find(']')?;
    let raw = &value[..end];
    Some((parse_color(raw), "[color=".len() + end + 1))
}

fn parse_color(raw: &str) -> Option<Color> {
    let hex = raw.strip_prefix('#').unwrap_or(raw);
    if hex.len() != 6 && hex.len() != 8 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let a = if hex.len() == 8 {
        u8::from_str_radix(&hex[6..8], 16).ok()?
    } else {
        255
    };
    Some(Color::rgba(r, g, b, a))
}

// ─── Unit tests (only the parts that can run without a GPU) ──────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_draw_text(text: &str) -> DrawText {
        DrawText::new(text, Vec2::new(0.0, 0.0), 24.0, [255, 255, 255, 255])
    }

    #[test]
    fn text_queue_push_and_clear() {
        let mut q = TextQueue::default();
        assert!(q.is_empty());
        q.push(make_draw_text("hello"));
        assert_eq!(q.len(), 1);
        assert!(!q.is_empty());
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn text_queue_iter_preserves_order() {
        let mut q = TextQueue::default();
        q.push(make_draw_text("first"));
        q.push(make_draw_text("second"));
        q.push(make_draw_text("third"));

        let texts: Vec<&str> = q.iter().map(|d| d.text.as_str()).collect();
        assert_eq!(texts, ["first", "second", "third"]);
    }

    #[test]
    fn drawtext_fields_preserved() {
        let d = DrawText::new("안녕", Vec2::new(10.0, 20.0), 24.0, [255, 0, 0, 255])
            .with_bounds(Vec2::new(120.0, 48.0))
            .with_align(TextAlign::Center)
            .rich();
        assert_eq!(d.text, "안녕");
        assert_eq!(d.position, Vec2::new(10.0, 20.0));
        assert_eq!(d.bounds, Some(Vec2::new(120.0, 48.0)));
        assert_eq!(d.size, 24.0);
        assert_eq!(d.color, EngineColor::from([255u8, 0, 0, 255]));
        assert_eq!(d.align, TextAlign::Center);
        assert!(d.rich);
        // Default anchor is TopLeft (backward compatible).
        assert_eq!(d.anchor, TextAnchor::TopLeft);
    }

    #[test]
    fn drawtext_centered_sets_center_anchor_and_align() {
        let d = DrawText::centered("Title", Vec2::new(400.0, 300.0), 32.0, [255, 255, 255, 255]);
        assert_eq!(d.position, Vec2::new(400.0, 300.0));
        assert_eq!(d.anchor, TextAnchor::Center);
        assert_eq!(d.align, TextAlign::Center);
    }

    #[test]
    fn drawtext_with_anchor_overrides_default() {
        let d = make_draw_text("x").with_anchor(TextAnchor::Center);
        assert_eq!(d.anchor, TextAnchor::Center);
        assert_eq!(TextAnchor::default(), TextAnchor::TopLeft);
    }

    #[test]
    fn rich_text_parser_strips_supported_tags() {
        let spans = parse_rich_text(
            "Hello [color=#ff0000][b]red[/b][/color] [i]italic[/i]",
            Attrs::new().family(Family::SansSerif),
        );
        let plain: String = spans.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(plain, "Hello red italic");
    }
}
