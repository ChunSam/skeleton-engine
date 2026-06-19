use glam::Vec2;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer as GlyphonTextRenderer, Viewport, Wrap,
};
use wgpu::{
    CommandEncoder, Device, LoadOp, MultisampleState, Operations, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureFormat, TextureView,
};

use crate::ecs::World;
use crate::resources::DisplayScaleFactor;

use super::cache::{CachedBuffer, PlainTextCacheKey};
use super::queue::{DrawText, TextAnchor, TextQueue};
use super::rich_text::parse_rich_text;

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
pub(super) fn build_font_system(font_data: &[u8], extra_fonts: &[Vec<u8>]) -> FontSystem {
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

/// Compute the layout buffer width for a `DrawText` given its anchor and viewport.
///
/// This is extracted into a standalone function so it can be unit-tested without GPU/FontSystem.
/// It mirrors the logic inside `TextRenderer::render`'s buffer-construction closure.
///
/// Returns:
/// - `None`  → single-line (unlimited width, no wrap) — not handled here; caller decides.
/// - `Some`  → the pixel width to pass to `Buffer::set_size`.
pub(super) fn layout_buffer_width(
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
pub(super) fn layout_buffer_height(
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
