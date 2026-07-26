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
use crate::resources::{DisplayScaleFactor, Letterbox};

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

/// One target-format's glyphon resources: a format-bound [`TextAtlas`] plus a pool of glyphon
/// renderers — one per text **batch** rendered this frame. Each glyphon `TextRenderer` owns the
/// vertex buffers of exactly one prepared batch; reusing a single one across batches would
/// overwrite earlier batches' buffers before the encoder executes (the same reason the UI pass
/// uploads its instance buffer once). `used` counts the batches taken this frame and is reset by
/// [`TextRenderer::end_frame`]. Non-surface formats (e.g. the HDR `Rgba16Float` intermediate) get
/// their own pool lazily — the text analogue of the sprite/UI format-matched pipeline caches.
struct FormatPool {
    format: TextureFormat,
    atlas: TextAtlas,
    renderers: Vec<GlyphonTextRenderer>,
    used: usize,
}

/// Text renderer backed by glyphon 0.6.
///
/// ## Ownership layout
/// - `Cache` is created first and shared with every `TextAtlas` / the `Viewport`.
///   (`TextAtlas::new` requires `&Cache`; `TextRenderer` retains ownership of `Cache`.)
/// - `Viewport::update(queue, Resolution{w,h})` refreshes the GPU uniform each frame.
/// - `pools[0]` is created for the init (surface) format; further formats are added lazily by
///   [`render_batch`](Self::render_batch).
pub struct TextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    /// Cache is created first and shared with the atlases / viewport (glyphon 0.6 requirement);
    /// retained so lazily-created per-format pools can build their atlas from it.
    cache: Cache,
    viewport: Viewport,
    /// Per-target-format atlas + renderer pools (see [`FormatPool`]).
    pools: Vec<FormatPool>,
    /// Cross-frame cache of shaped plain-text Buffers, keyed by all layout-affecting inputs.
    /// Entries not accessed in the current frame are evicted by [`end_frame`](Self::end_frame)
    /// (generation-based, mirroring the `atlas.trim()` per-frame pattern). Rich text is NOT cached.
    shaped_buffer_cache: std::collections::HashMap<PlainTextCacheKey, CachedBuffer>,
    /// Monotonically increasing frame counter used to evict stale cache entries.
    cache_generation: u64,
}

/// Build a cosmic-text [`FontSystem`] loading `font_data` (if non-empty) plus every blob in
/// `extra_fonts` (multi-script coverage). Extracted from [`TextRenderer::new`] so font loading is
/// unit-testable without a GPU device.
pub(crate) fn build_font_system(font_data: &[u8], extra_fonts: &[Vec<u8>]) -> FontSystem {
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
            viewport,
            pools: vec![FormatPool {
                format,
                atlas,
                renderers: vec![renderer],
                used: 0,
            }],
            shaped_buffer_cache: std::collections::HashMap::new(),
            cache_generation: 0,
        }
    }

    /// Index of the pool for `format`, creating it (atlas + one renderer) on first use.
    fn pool_index(&mut self, device: &Device, queue: &Queue, format: TextureFormat) -> usize {
        if let Some(i) = self.pools.iter().position(|p| p.format == format) {
            return i;
        }
        let mut atlas = TextAtlas::new(device, queue, &self.cache, format);
        let renderer =
            GlyphonTextRenderer::new(&mut atlas, device, MultisampleState::default(), None);
        self.pools.push(FormatPool {
            format,
            atlas,
            renderers: vec![renderer],
            used: 0,
        });
        self.pools.len() - 1
    }

    /// Ends the frame's text rendering: evicts shaped-buffer cache entries not used this frame,
    /// trims every pool's atlas, resets the per-frame batch counters, and advances the cache
    /// generation. Called once per frame by the render orchestration, after the last text pass.
    pub fn end_frame(&mut self) {
        let gen = self.cache_generation;
        self.shaped_buffer_cache
            .retain(|_k, v| v.last_used_gen == gen);
        for pool in &mut self.pools {
            pool.atlas.trim();
            pool.used = 0;
        }
        self.cache_generation = self.cache_generation.wrapping_add(1);
    }

    /// Pulls the `TextQueue` from the ECS `World` and renders all text onto `view` (assumed to be
    /// in the init/surface format) — the final, on-top text pass.
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
        // Design-resolution letterbox: map design-space text into the centered window sub-rect
        // (identity when no DesignResolution is set). `px_scale`/`px_offset` are in logical pixels;
        // `scale_factor` then takes logical → physical for crisp device-resolution glyphs.
        let lb = world.resource::<Letterbox>().copied().unwrap_or_default();
        let format = self.pools[0].format;
        self.render_batch(
            device,
            queue,
            encoder,
            view,
            format,
            items,
            w,
            h,
            scale_factor,
            lb,
        );
    }

    /// Renders one batch of `items` onto `view` (whose texture format is `format`) in a single
    /// glyphon prepare + render pass, using the next pooled renderer for that format. Used both by
    /// the on-top [`render`](Self::render) wrapper and by the z-interleaved layered-text runs in
    /// the frame orchestration (which alternates UI-primitive sub-ranges with text batches).
    /// [`end_frame`](Self::end_frame) must run once per frame after the last batch.
    #[allow(clippy::too_many_arguments)]
    pub fn render_batch(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        format: TextureFormat,
        items: Vec<DrawText>,
        w: u32,
        h: u32,
        scale_factor: f32,
        lb: Letterbox,
    ) {
        if items.is_empty() {
            return;
        }
        // Update Viewport (writes the resolution to the GPU uniform; idempotent across the
        // frame's batches, which all target same-sized views).
        self.viewport.update(
            queue,
            Resolution {
                width: w,
                height: h,
            },
        );
        let gen = self.cache_generation;
        let buffers = self.build_batch(items, w, h, scale_factor, lb);
        let pool_idx = self.pool_index(device, queue, format);

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

        {
            let pool = &mut self.pools[pool_idx];
            if pool.used >= pool.renderers.len() {
                pool.renderers.push(GlyphonTextRenderer::new(
                    &mut pool.atlas,
                    device,
                    MultisampleState::default(),
                    None,
                ));
            }
            let batch = pool.used;
            pool.used += 1;
            let FormatPool {
                atlas, renderers, ..
            } = pool;
            let renderer = &mut renderers[batch];

            // prepare — rasterize glyphs + upload to GPU buffers
            if let Err(e) = renderer.prepare(
                device,
                queue,
                &mut self.font_system,
                atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            ) {
                log::warn!("text prepare failed: {e}");
            }

            // Text render pass — composite over what is already in `view` with LoadOp::Load
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
            if let Err(e) = renderer.render(atlas, &self.viewport, &mut pass) {
                log::error!("text render failed: {e}");
            }
        }

        // ── Re-insert plain-text buffers into the shaped-buffer cache ────────
        // Only plain (non-rich) entries have a key; rich text buffers are dropped. Entries not
        // re-inserted by any batch this frame keep their old generation and are evicted by
        // `end_frame`.
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
        }
    }

    /// Shapes one batch of `DrawText`s into glyphon Buffers (serving plain text from the
    /// cross-frame shaped-buffer cache), returning `(buffer, scaled draw, scroll, cache key)` per
    /// item. Extracted from the old monolithic `render` so multiple batches per frame share the
    /// cache.
    fn build_batch(
        &mut self,
        items: Vec<DrawText>,
        w: u32,
        h: u32,
        scale_factor: f32,
        lb: crate::resources::Letterbox,
    ) -> Vec<(Buffer, DrawText, f32, Option<PlainTextCacheKey>)> {
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
        // re-insert with the updated generation (done by `render_batch` after drawing). This
        // avoids lifetime conflicts between the cache (field on self) and text_areas (which
        // borrows the returned Vec).
        items
            .into_iter()
            .map(|d| {
                let size = d.size * lb.px_scale * scale_factor;
                let position = (d.position * lb.px_scale + lb.px_offset) * scale_factor;
                let bounds = d.bounds.map(|b| b * lb.px_scale * scale_factor);
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
                    //
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
                    shape_text(
                        &mut self.font_system,
                        &ShapeSpec {
                            text: &d.text,
                            size,
                            width,
                            height: Some(layout_buffer_height(
                                d.anchor,
                                bounds.map(|b| b.y),
                                h as f32,
                                position.y,
                            )),
                            wrap: if single_line.is_some() {
                                Wrap::None
                            } else {
                                Wrap::WordOrGlyph
                            },
                            align: d.align.to_glyphon(),
                            rich: d.rich,
                        },
                    )
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
                // Anchor: when centered, shift the effective top-left so `position`
                // lands on the text's true center. The horizontal center is MEASURED
                // from the shaped glyph extents (see `shaped_center_x`), not assumed
                // to be `max_w / 2`: the layout buffer is the full viewport width (so
                // centered titles don't wrap early), and `align = Center` then
                // distributes glyphs around the buffer's center rather than its left
                // edge. Offsetting by `max_w / 2` double-counted that and drifted the
                // text right by ~half the viewport (EW-001). Measuring the real glyph
                // span is correct for every `align` (Left/Center/Right/Auto): for
                // left-aligned text it reduces to `max_w / 2`, for centered text it is
                // the buffer center. Vertical center stays a line-count estimate
                // (line_height = 1.2 × size, see Metrics).
                let anchor_offset = match d.anchor {
                    TextAnchor::TopLeft => Vec2::ZERO,
                    TextAnchor::Center => {
                        let lines = buf.layout_runs().count() as f32;
                        Vec2::new(
                            shaped_center_x(&buf),
                            lines * size * LINE_HEIGHT_FACTOR * 0.5,
                        )
                    }
                };
                let mut scaled = d;
                scaled.position = position - anchor_offset;
                scaled.bounds = bounds;
                scaled.size = size;
                (buf, scaled, scroll, plain_key)
            })
            .collect()
    }
}

/// Line height as a multiple of the font size, shared by every text path.
///
/// The renderer shapes with `Metrics::new(size, size * LINE_HEIGHT_FACTOR)`, the
/// [`TextAnchor::Center`] offset divides by it, and
/// [`TextMeasurer`](crate::TextMeasurer) reports heights in terms of it — so the three
/// cannot drift apart.
pub(crate) const LINE_HEIGHT_FACTOR: f32 = 1.2;

/// Everything that affects how a string is shaped into a [`Buffer`].
///
/// Extracted so the GPU renderer and the CPU-only [`TextMeasurer`](crate::TextMeasurer)
/// shape through **one** code path: a measurement can only match the rendered result if it
/// uses the same metrics, wrap mode, attrs and [`Shaping`] level, and the compiler now
/// enforces that by construction rather than by two call sites agreeing.
pub(crate) struct ShapeSpec<'a> {
    pub text: &'a str,
    /// Font size in the buffer's own pixel space (the renderer passes the display-scaled size).
    pub size: f32,
    /// Layout width; `None` = unlimited (no wrapping possible).
    pub width: Option<f32>,
    /// Layout height; `None` = unlimited.
    pub height: Option<f32>,
    pub wrap: Wrap,
    /// Per-line alignment; `None` = glyphon's default (script-derived, i.e. `TextAlign::Auto`).
    pub align: Option<glyphon::cosmic_text::Align>,
    /// Parse `[color=…]`/`[b]`/`[i]` markup instead of shaping the literal string.
    pub rich: bool,
}

/// Shape `spec` into a laid-out [`Buffer`] using `font_system`'s font stack.
///
/// The single shaping entry point for the whole engine — see [`ShapeSpec`].
pub(crate) fn shape_text(font_system: &mut FontSystem, spec: &ShapeSpec<'_>) -> Buffer {
    let metrics = Metrics::new(spec.size, spec.size * LINE_HEIGHT_FACTOR);
    let mut buf = Buffer::new(font_system, metrics);
    buf.set_size(font_system, spec.width, spec.height);
    buf.set_wrap(font_system, spec.wrap);
    let default_attrs = Attrs::new().family(Family::SansSerif);
    if spec.rich {
        let rich = parse_rich_text(spec.text, &default_attrs);
        let spans: Vec<(&str, Attrs<'_>)> = rich
            .iter()
            .map(|(s, attrs)| (s.as_str(), attrs.clone()))
            .collect();
        buf.set_rich_text(font_system, spans, &default_attrs, Shaping::Advanced, None);
    } else {
        buf.set_text(
            font_system,
            spec.text,
            &default_attrs,
            Shaping::Advanced,
            None,
        );
    }
    for line in &mut buf.lines {
        line.set_align(spec.align);
    }
    buf.shape_until_scroll(font_system, false);
    buf
}

/// Horizontal center of shaped text within its layout buffer, in buffer-local pixels.
///
/// Measured from the actual glyph extents (`min(glyph.x) .. max(glyph.x + glyph.w)`), so it
/// is correct for ANY alignment: left-aligned text centers at half its own span, while
/// `Align::Center`/`Right`/`End` text — which glyphon distributes around or against the
/// *buffer* edges, not glyph 0 — reports the center of where the glyphs actually landed.
///
/// Subtracting this from a [`TextAnchor::Center`] draw's `position.x` makes the rendered
/// text's horizontal center land exactly on `position.x`, independent of `align` or the
/// (full-viewport) buffer width. This is the EW-001 fix: the old `max_w / 2` assumption was
/// only valid for left-aligned text, so `DrawText::centered` (which sets `align = Center`)
/// drifted right by ~half the viewport. Returns `0.0` for empty text (no glyphs).
pub(super) fn shaped_center_x(buf: &Buffer) -> f32 {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    for run in buf.layout_runs() {
        for g in run.glyphs.iter() {
            min_x = min_x.min(g.x);
            max_x = max_x.max(g.x + g.w);
        }
    }
    if min_x.is_finite() {
        (min_x + max_x) * 0.5
    } else {
        0.0
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
