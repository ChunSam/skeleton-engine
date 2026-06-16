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
    fn to_glyphon(self) -> Option<Align> {
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

/// Cache key for a plain (non-rich) shaped text buffer.
///
/// All layout-affecting inputs are included. `f32` fields are stored as their
/// bit patterns so the key is `Eq + Hash` without precision issues — two
/// DrawTexts that are bit-identical will always share a buffer, and any field
/// that changes (e.g. size, position used for buffer-width computation) produces
/// a different key and a cache miss.
///
/// Rich text is NOT cached here (span attrs are hard to hash; see
/// `TextRenderer::shaped_buffer_cache`).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct PlainTextCacheKey {
    text: String,
    /// Font size in pixels × scale_factor, as f32 bits.
    scaled_size_bits: u32,
    /// Scaled bounds, if set. `None` = no explicit bounds (buffer size derived from viewport).
    bounds_w_bits: Option<u32>, // None or Some(f32::to_bits())
    bounds_h_bits: Option<u32>,
    /// Viewport dimensions (affect buffer size when no explicit bounds).
    viewport_w: u32,
    viewport_h: u32,
    /// Position (affects buffer width/height for TopLeft anchor).
    position_x_bits: u32,
    position_y_bits: u32,
    /// Whether the DrawText is in single-line mode (affects Wrap). The caret *byte offset* is
    /// deliberately NOT part of the key: scroll is always recomputed from the shaped buffer after
    /// a cache hit (via `caret_x`), so two carets over identical text safely share one buffer.
    is_single_line: bool,
    align: TextAlign,
    anchor: TextAnchor,
}

struct CachedBuffer {
    buffer: Buffer,
    /// Frame generation when this entry was last used. Entries not accessed in
    /// the current frame are evicted at the end of `render()`.
    last_used_gen: u64,
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
    /// Cross-frame cache of shaped plain-text Buffers, keyed by all layout-affecting inputs.
    /// Entries not accessed in the current frame are evicted after `render()` (generation-based,
    /// mirroring the `atlas.trim()` per-frame pattern). Rich text is NOT cached.
    shaped_buffer_cache: std::collections::HashMap<PlainTextCacheKey, CachedBuffer>,
    /// Monotonically increasing frame counter used to evict stale cache entries.
    cache_generation: u64,
}

/// Build a cosmic-text [`FontSystem`] loading `font_data` (if non-empty) plus every blob in
/// `extra_fonts` (multi-script coverage). Extracted from [`TextRenderer::new`] so font loading is
/// unit-testable without a GPU device.
fn build_font_system(font_data: &[u8], extra_fonts: &[Vec<u8>]) -> FontSystem {
    let mut font_system = FontSystem::new();
    if !font_data.is_empty() {
        font_system.db_mut().load_font_data(font_data.to_vec());
    }
    for blob in extra_fonts {
        if !blob.is_empty() {
            font_system.db_mut().load_font_data(blob.clone());
        }
    }
    font_system
}

impl TextRenderer {
    /// Initialises GPU resources.
    ///
    /// If `font_data` is non-empty the TTF/OTF bytes are loaded into fontdb.
    /// Otherwise glyphon's system-font fallback is used.
    ///
    /// `extra_fonts` are additional TTF/OTF blobs loaded alongside `font_data` for multi-script
    /// coverage (e.g. a Latin UI font in `font_data` plus an RTL-script font here). cosmic-text
    /// falls back across all loaded fonts by script, so mixed-direction text shapes correctly.
    pub fn new(
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        font_data: &[u8],
        extra_fonts: &[Vec<u8>],
    ) -> Self {
        let font_system = build_font_system(font_data, extra_fonts);

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
            shaped_buffer_cache: std::collections::HashMap::new(),
            cache_generation: 0,
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
        // `std::mem::take` drains the Vec in O(1) without cloning — the queue is
        // re-filled by game systems each frame, so draining here is correct
        // (same pattern as the DebugDraw drain in app/render.rs).
        let items: Vec<DrawText> = match world.resource_mut::<TextQueue>() {
            Some(q) if !q.is_empty() => std::mem::take(&mut q.items),
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

        // Increment frame generation for shaped-buffer cache eviction.
        self.cache_generation += 1;
        let gen = self.cache_generation;

        // Convert each DrawText into a glyphon Buffer.
        // - `Buffer::set_size` takes `(font_system, Option<f32>, Option<f32>)` in cosmic-text.
        // - `set_text` takes `(font_system, text, attrs, shaping)`.
        //
        // Plain (non-rich) DrawTexts are served from a cross-frame shaped-buffer cache
        // keyed by all layout-affecting inputs (text, size, bounds, viewport, position,
        // align, anchor, is_single_line). On a cache hit the shaped Buffer is reused
        // directly (skipping set_text + shape_until_scroll). Scroll and anchor-offset
        // are always recomputed from the buffer's layout runs since they are cheap.
        //
        // Rich text is NOT cached: span attrs (including per-span colors from [color=…])
        // are hard to hash correctly, so the safe choice is to keep the existing
        // per-frame build path for rich DrawTexts.
        //
        // Cache strategy: `remove` the entry to get an owned Buffer, use it, then
        // re-insert with the updated generation. This avoids lifetime conflicts
        // between the cache (field on self) and text_areas (borrows the Vec).
        let buffers: Vec<(Buffer, DrawText, f32, Option<PlainTextCacheKey>)> = items
            .into_iter()
            .map(|d| {
                let size = d.size * scale_factor;
                let position = d.position * scale_factor;
                let bounds = d.bounds.map(|b| b * scale_factor);
                let single_line = d.single_line_caret;

                // ── Cache lookup (plain text only) ────────────────────────────
                let plain_key: Option<PlainTextCacheKey> = if !d.rich {
                    Some(PlainTextCacheKey {
                        text: d.text.clone(),
                        scaled_size_bits: size.to_bits(),
                        bounds_w_bits: bounds.map(|b| b.x.to_bits()),
                        bounds_h_bits: bounds.map(|b| b.y.to_bits()),
                        viewport_w: w,
                        viewport_h: h,
                        position_x_bits: position.x.to_bits(),
                        position_y_bits: position.y.to_bits(),
                        is_single_line: single_line.is_some(),
                        align: d.align,
                        anchor: d.anchor,
                    })
                } else {
                    None
                };

                // Try to remove a cached entry so we own it for the duration of the frame.
                let cached = plain_key
                    .as_ref()
                    .and_then(|k| self.shaped_buffer_cache.remove(k));

                let buf = if let Some(CachedBuffer { buffer, .. }) = cached {
                    // Cache hit: reuse the already-shaped buffer.
                    buffer
                } else {
                    // Cache miss (or rich text): build and shape from scratch.
                    let metrics = Metrics::new(size, size * 1.2); // line_height = 1.2× size
                    let mut buf = Buffer::new(&mut self.font_system, metrics);
                    // Single-line (TextInput): expand to unlimited width + no wrap, then
                    // scroll horizontally below. Otherwise wrap at the bounds width.
                    //
                    // For a centered DrawText (anchor = Center, position = text center),
                    // the position is typically near the middle of the viewport (e.g. w/2).
                    // Using `w - position.x` would give only half the viewport width, causing
                    // text to wrap prematurely.  Instead we give the buffer the FULL viewport
                    // dimension and let the existing anchor-offset logic center the shaped text.
                    // For the default TopLeft anchor the old behavior is preserved.
                    let width = if single_line.is_some() {
                        None
                    } else {
                        Some(layout_buffer_width(
                            d.anchor,
                            bounds.map(|b| b.x),
                            w as f32,
                            position.x,
                        ))
                    };
                    buf.set_size(
                        &mut self.font_system,
                        width,
                        Some(layout_buffer_height(
                            d.anchor,
                            bounds.map(|b| b.y),
                            h as f32,
                            position.y,
                        )),
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
                        let rich = parse_rich_text(&d.text, &default_attrs);
                        let spans: Vec<(&str, Attrs<'_>)> = rich
                            .iter()
                            .map(|(s, attrs)| (s.as_str(), attrs.clone()))
                            .collect();
                        buf.set_rich_text(
                            &mut self.font_system,
                            spans,
                            &default_attrs,
                            Shaping::Advanced,
                            None,
                        );
                    } else {
                        buf.set_text(
                            &mut self.font_system,
                            &d.text,
                            &default_attrs,
                            Shaping::Advanced,
                            None,
                        );
                    }
                    for line in &mut buf.lines {
                        line.set_align(d.align.to_glyphon());
                    }
                    buf.shape_until_scroll(&mut self.font_system, false);
                    buf
                };

                // ── Post-shaping: scroll + anchor (always recomputed, cheap) ─
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
                (buf, scaled, scroll, plain_key)
            })
            .collect();

        let text_areas: Vec<TextArea<'_>> = buffers
            .iter()
            .map(|(buf, d, scroll, _key)| TextArea {
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
        if let Err(e) = self.renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        ) {
            log::warn!("text prepare failed: {e}");
        }

        // Text render pass — composite over sprites with LoadOp::Load
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("text pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Err(e) = self.renderer.render(&self.atlas, &self.viewport, &mut pass) {
                log::error!("text render failed: {e}");
            }
        }

        // ── Re-insert plain-text buffers into the shaped-buffer cache ────────
        // Only plain (non-rich) entries have a key; rich text buffers are dropped.
        // Entries from previous frames that were not seen this frame (removed at
        // the start of this loop and not re-inserted) are implicitly evicted.
        // Additionally, any entries that were NOT removed at the start (i.e. their
        // DrawText was not in the queue this frame) still hold their previous
        // generation. We evict those below by retaining only entries from the
        // current generation.
        for (buffer, _d, _scroll, plain_key) in buffers {
            if let Some(key) = plain_key {
                self.shaped_buffer_cache.insert(
                    key,
                    CachedBuffer {
                        buffer,
                        last_used_gen: gen,
                    },
                );
            }
            // Rich-text buffers drop here (no key).
        }
        // Evict entries that were NOT accessed this frame (stale from a prior frame
        // where different text was rendered). Mirrors atlas.trim() below.
        self.shaped_buffer_cache
            .retain(|_k, v| v.last_used_gen == gen);

        // Trim unused glyphs from the atlas for the next frame
        self.atlas.trim();
    }
}

fn parse_rich_text<'a>(text: &str, default_attrs: &Attrs<'a>) -> Vec<(String, Attrs<'a>)> {
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
    default_attrs: &Attrs<'a>,
    color: Option<Color>,
    bold_depth: usize,
    italic_depth: usize,
) -> Attrs<'a> {
    let mut attrs = default_attrs.clone();
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

/// Compute the layout buffer width for a `DrawText` given its anchor and viewport.
///
/// This is extracted into a standalone function so it can be unit-tested without GPU/FontSystem.
/// It mirrors the logic inside `TextRenderer::render`'s buffer-construction closure.
///
/// Returns:
/// - `None`  → single-line (unlimited width, no wrap) — not handled here; caller decides.
/// - `Some`  → the pixel width to pass to `Buffer::set_size`.
fn layout_buffer_width(
    anchor: TextAnchor,
    bounds_x: Option<f32>, // already scaled
    viewport_w: f32,
    position_x: f32, // already scaled
) -> f32 {
    bounds_x.map_or_else(
        || match anchor {
            TextAnchor::Center => viewport_w,
            TextAnchor::TopLeft => (viewport_w - position_x).max(0.0),
        },
        |b| b.max(0.0),
    )
}

/// Compute the layout buffer height for a `DrawText` given its anchor and viewport.
fn layout_buffer_height(
    anchor: TextAnchor,
    bounds_y: Option<f32>, // already scaled
    viewport_h: f32,
    position_y: f32, // already scaled
) -> f32 {
    bounds_y.map_or_else(
        || match anchor {
            TextAnchor::Center => viewport_h,
            TextAnchor::TopLeft => (viewport_h - position_y).max(0.0),
        },
        |b| b.max(0.0),
    )
}

// ─── Unit tests (only the parts that can run without a GPU) ──────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_draw_text(text: &str) -> DrawText {
        DrawText::new(text, Vec2::new(0.0, 0.0), 24.0, [255, 255, 255, 255])
    }

    #[test]
    fn text_align_to_glyphon_mapping() {
        // Auto → None (cosmic-text aligns by direction; RTL right-aligns automatically).
        assert!(TextAlign::Auto.to_glyphon().is_none());
        // Explicit variants → Some(matching glyphon Align).
        assert!(matches!(TextAlign::Left.to_glyphon(), Some(Align::Left)));
        assert!(matches!(
            TextAlign::Center.to_glyphon(),
            Some(Align::Center)
        ));
        assert!(matches!(TextAlign::Right.to_glyphon(), Some(Align::Right)));
        assert!(matches!(TextAlign::End.to_glyphon(), Some(Align::End)));
    }

    #[test]
    fn build_font_system_loads_extra_fonts() {
        // The bundled OFL Hebrew font is a real RTL-script face the engine can load alongside the
        // default. Loading it as an extra font must add exactly one face to the db.
        let hebrew = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/NotoSansHebrew-Regular.ttf"
        ))
        .to_vec();
        let base = build_font_system(&[], &[]).db().faces().count();
        let with_extra = build_font_system(&[], &[hebrew]).db().faces().count();
        assert_eq!(
            with_extra,
            base + 1,
            "one extra font face is loaded into the db"
        );
    }

    #[test]
    fn build_font_system_skips_empty_blobs() {
        let base = build_font_system(&[], &[]).db().faces().count();
        // Empty font_data + empty extra blobs add nothing.
        let same = build_font_system(&[], &[Vec::new(), Vec::new()])
            .db()
            .faces()
            .count();
        assert_eq!(same, base, "empty blobs are skipped");
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

    // ─── Fix 2 regression: centered no-bounds text uses full viewport width ───

    /// Regression: before the fix, `DrawText::centered` at position (400, 300) in a
    /// 800×600 viewport gave a layout buffer width of `800 - 400 = 400` (half the
    /// screen), causing a title that fits on one line to wrap.  After the fix the
    /// buffer width is the full `800`.
    ///
    /// This test exercises `layout_buffer_width` / `layout_buffer_height` directly
    /// (pure functions, no GPU/FontSystem needed).
    #[test]
    fn centered_no_bounds_uses_full_viewport_width() {
        let (vw, vh) = (800.0_f32, 600.0_f32);
        // Centered position (typical: screen center)
        let (px, py) = (400.0_f32, 300.0_f32);

        let w = layout_buffer_width(TextAnchor::Center, None, vw, px);
        let h = layout_buffer_height(TextAnchor::Center, None, vh, py);

        assert_eq!(
            w, vw,
            "Center anchor: layout width must be full viewport width {vw}, got {w}"
        );
        assert_eq!(
            h, vh,
            "Center anchor: layout height must be full viewport height {vh}, got {h}"
        );
    }

    /// TopLeft anchor retains the old behavior: buffer width = viewport - position.
    #[test]
    fn top_left_no_bounds_subtracts_position() {
        let (vw, vh) = (800.0_f32, 600.0_f32);
        let (px, py) = (100.0_f32, 50.0_f32);

        let w = layout_buffer_width(TextAnchor::TopLeft, None, vw, px);
        let h = layout_buffer_height(TextAnchor::TopLeft, None, vh, py);

        assert_eq!(w, vw - px, "TopLeft: width should be {}", vw - px);
        assert_eq!(h, vh - py, "TopLeft: height should be {}", vh - py);
    }

    /// Explicit bounds override anchor in both cases.
    #[test]
    fn explicit_bounds_override_anchor() {
        let bounds_x = 200.0_f32;
        let bounds_y = 100.0_f32;

        for anchor in [TextAnchor::Center, TextAnchor::TopLeft] {
            let w = layout_buffer_width(anchor, Some(bounds_x), 800.0, 400.0);
            let h = layout_buffer_height(anchor, Some(bounds_y), 600.0, 300.0);
            assert_eq!(w, bounds_x, "{anchor:?}: explicit bounds_x ignored");
            assert_eq!(h, bounds_y, "{anchor:?}: explicit bounds_y ignored");
        }
    }

    /// TopLeft with position beyond viewport gives 0 (clamped), not negative.
    #[test]
    fn top_left_position_beyond_viewport_clamps_to_zero() {
        let w = layout_buffer_width(TextAnchor::TopLeft, None, 800.0, 900.0);
        let h = layout_buffer_height(TextAnchor::TopLeft, None, 600.0, 700.0);
        assert_eq!(w, 0.0, "should clamp to 0 when position > viewport");
        assert_eq!(h, 0.0, "should clamp to 0 when position > viewport");
    }

    /// BUG REPRODUCTION (fails without the fix): before the fix, centered text with
    /// position at the viewport center would get half the viewport as buffer width,
    /// not the full viewport.  This asserts the buggy value is NOT what we produce.
    #[test]
    fn centered_no_bounds_does_not_use_half_viewport() {
        let (vw, vh) = (800.0_f32, 600.0_f32);
        let (px, py) = (vw / 2.0, vh / 2.0); // typical centered position

        let w = layout_buffer_width(TextAnchor::Center, None, vw, px);
        let h = layout_buffer_height(TextAnchor::Center, None, vh, py);

        // Before the fix, w would be `vw - px = 400` (half). Now it must be `vw = 800`.
        assert_ne!(
            w,
            vw - px,
            "REGRESSION: Center anchor must NOT produce half-viewport width {}",
            vw - px
        );
        assert_eq!(w, vw, "Center anchor must produce full viewport width {vw}");
        assert_ne!(
            h,
            vh - py,
            "REGRESSION: Center anchor must NOT produce half-viewport height"
        );
        assert_eq!(
            h, vh,
            "Center anchor must produce full viewport height {vh}"
        );
    }

    #[test]
    fn rich_text_parser_strips_supported_tags() {
        let default_attrs = Attrs::new().family(Family::SansSerif);
        let spans = parse_rich_text(
            "Hello [color=#ff0000][b]red[/b][/color] [i]italic[/i]",
            &default_attrs,
        );
        let plain: String = spans.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(plain, "Hello red italic");
    }

    // ─── Shaped-buffer cache key tests ───────────────────────────────────────

    fn base_key() -> PlainTextCacheKey {
        PlainTextCacheKey {
            text: "hello".to_string(),
            scaled_size_bits: 24.0_f32.to_bits(),
            bounds_w_bits: None,
            bounds_h_bits: None,
            viewport_w: 800,
            viewport_h: 600,
            position_x_bits: 10.0_f32.to_bits(),
            position_y_bits: 20.0_f32.to_bits(),
            is_single_line: false,
            align: TextAlign::Left,
            anchor: TextAnchor::TopLeft,
        }
    }

    /// Identical inputs produce the same key (cache hit).
    #[test]
    fn cache_key_hit_when_identical() {
        assert_eq!(base_key(), base_key());
    }

    /// Any change to any field produces a different key (cache miss).
    #[test]
    fn cache_key_miss_when_text_differs() {
        let mut k = base_key();
        k.text = "world".to_string();
        assert_ne!(k, base_key());
    }

    #[test]
    fn cache_key_miss_when_size_differs() {
        let mut k = base_key();
        k.scaled_size_bits = 32.0_f32.to_bits();
        assert_ne!(k, base_key());
    }

    #[test]
    fn cache_key_miss_when_bounds_w_differs() {
        let mut k = base_key();
        k.bounds_w_bits = Some(200.0_f32.to_bits()); // None → Some(200.0)
        assert_ne!(k, base_key());
    }

    #[test]
    fn cache_key_miss_when_bounds_h_differs() {
        let mut k = base_key();
        k.bounds_h_bits = Some(100.0_f32.to_bits()); // None → Some(100.0)
        assert_ne!(k, base_key());
    }

    /// Bounds of 0.0 must not collide with "no bounds" — uses Option, not a 0 sentinel.
    #[test]
    fn cache_key_zero_bounds_differs_from_no_bounds() {
        let mut k = base_key();
        k.bounds_w_bits = Some(0.0_f32.to_bits()); // Some(0.0), distinct from None
        assert_ne!(k, base_key(), "Some(0.0) bounds must not equal None bounds");
    }

    #[test]
    fn cache_key_miss_when_viewport_differs() {
        let mut k = base_key();
        k.viewport_w = 1280;
        assert_ne!(k, base_key());
    }

    #[test]
    fn cache_key_miss_when_position_differs() {
        let mut k = base_key();
        k.position_x_bits = 50.0_f32.to_bits();
        assert_ne!(k, base_key());
    }

    #[test]
    fn cache_key_miss_when_align_differs() {
        let mut k = base_key();
        k.align = TextAlign::Center;
        assert_ne!(k, base_key());
    }

    #[test]
    fn cache_key_miss_when_anchor_differs() {
        let mut k = base_key();
        k.anchor = TextAnchor::Center;
        assert_ne!(k, base_key());
    }

    #[test]
    fn cache_key_miss_when_single_line_differs() {
        let mut k = base_key();
        k.is_single_line = true;
        assert_ne!(k, base_key());
    }

    /// Eviction: entries with a generation older than the current frame's generation
    /// should be removed. This mirrors the logic in TextRenderer::render's retain call.
    #[test]
    fn cache_eviction_removes_stale_entries() {
        // Simulate the retain logic: keep only entries where last_used_gen == current gen.
        let current_gen = 5u64;
        let mut cache: std::collections::HashMap<PlainTextCacheKey, u64> =
            std::collections::HashMap::new();
        let k1 = base_key();
        let mut k2 = base_key();
        k2.text = "other".to_string();
        // k1 was used this frame (gen == current), k2 was not (gen == old).
        cache.insert(k1.clone(), current_gen);
        cache.insert(k2.clone(), current_gen - 1);

        cache.retain(|_k, gen| *gen == current_gen);

        assert!(
            cache.contains_key(&k1),
            "current-frame entry should be retained"
        );
        assert!(!cache.contains_key(&k2), "stale entry should be evicted");
    }
}
