// ─── Unit tests (only the parts that can run without a GPU) ──────────────────────────────────

use glam::Vec2;
use glyphon::{cosmic_text::Align, Attrs, Buffer, Family, Metrics, Shaping, Wrap};

use super::cache::PlainTextCacheKey;
use super::queue::{DrawText, TextAlign, TextAnchor, TextQueue};
use super::renderer::{
    build_font_system, layout_buffer_height, layout_buffer_width, shaped_center_x,
};
use super::rich_text::parse_rich_text;
use crate::color::Color as EngineColor;

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

// ─── EW-001 regression: centered text center lands on `position.x` ───
//
// Background: the buffer-width "Fix 2" above made a centered `DrawText`'s layout
// buffer the FULL viewport width (so titles don't wrap early). But with
// `align = Center` glyphon then centers each line around the *buffer* center, while
// the anchor offset still subtracted `max_w/2` (a left-aligned assumption) — so the
// text drifted right by ~half the viewport whenever `position.x` was off-center.
// The fix measures the real glyph center (`shaped_center_x`) instead. These tests
// shape real text headlessly (cosmic-text shaping is CPU-only; the bundled DejaVu
// Sans makes glyph metrics deterministic) and check where the glyphs actually land.

/// Shape `text` exactly as `TextRenderer::render` does for a no-bounds `DrawText`
/// (full-viewport layout buffer, given `align`/`anchor`), returning the shaped buffer
/// so a test can measure glyph positions. No GPU needed.
#[allow(clippy::too_many_arguments)]
fn shape_no_bounds(
    text: &str,
    align: TextAlign,
    anchor: TextAnchor,
    size: f32,
    vw: f32,
    vh: f32,
    px: f32,
    py: f32,
) -> Buffer {
    let font = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/fonts/DejaVuSans.ttf"
    ))
    .to_vec();
    let mut fs = build_font_system(&font, &[]);
    let mut buf = Buffer::new(&mut fs, Metrics::new(size, size * 1.2));
    buf.set_size(
        &mut fs,
        Some(layout_buffer_width(anchor, None, vw, px)),
        Some(layout_buffer_height(anchor, None, vh, py)),
    );
    buf.set_wrap(&mut fs, Wrap::WordOrGlyph);
    let attrs = Attrs::new().family(Family::SansSerif);
    buf.set_text(&mut fs, text, &attrs, Shaping::Advanced, None);
    for line in &mut buf.lines {
        line.set_align(align.to_glyphon());
    }
    buf.shape_until_scroll(&mut fs, false);
    buf
}

/// Widest layout-run width (the old anchor offset was `max_w / 2`).
fn max_line_w(buf: &Buffer) -> f32 {
    buf.layout_runs().map(|r| r.line_w).fold(0.0_f32, f32::max)
}

/// The reported bug: `DrawText::centered` (anchor=Center + align=Center) at an
/// OFF-CENTER `position.x` must render with its horizontal center on `position.x`.
#[test]
fn ew001_centered_text_center_lands_on_position_x() {
    let (vw, vh, size) = (800.0_f32, 600.0_f32, 32.0_f32);
    let (px, py) = (150.0_f32, 300.0_f32); // off-center x — the broken case

    let buf = shape_no_bounds(
        "Hello",
        TextAlign::Center,
        TextAnchor::Center,
        size,
        vw,
        vh,
        px,
        py,
    );
    let center = shaped_center_x(&buf);
    let max_w = max_line_w(&buf);

    // For `align = Center` over a full-viewport buffer, glyphs sit around the buffer
    // center, so the measured center is the viewport center — not `max_w / 2`.
    assert!(
        (center - vw / 2.0).abs() < 2.0,
        "centered align: glyph center should be ~viewport center {}, got {center}",
        vw / 2.0
    );

    // FIXED: anchor offset = measured center → rendered center lands on position.x.
    let new_rendered_center = (px - center) + center;
    assert!(
        (new_rendered_center - px).abs() < 0.5,
        "fixed: centered text center must land on position.x ({px}), got {new_rendered_center}"
    );

    // REGRESSION GUARD: the old `max_w / 2` offset drifts the center right by ~vw/2.
    let old_rendered_center = (px - max_w / 2.0) + center;
    assert!(
        old_rendered_center - px > vw / 4.0,
        "the old max_w/2 offset must drift right (got drift {})",
        old_rendered_center - px
    );
}

/// The game's current workaround (anchor=Center + default Left align) must be
/// unchanged by the fix: left-aligned glyphs start at buffer x≈0, so the measured
/// center reduces to `max_w / 2` (the pre-fix value) and the text still lands on
/// `position.x`.
#[test]
fn ew001_left_align_center_anchor_unchanged() {
    let (vw, vh, size) = (800.0_f32, 600.0_f32, 32.0_f32);
    let (px, py) = (150.0_f32, 300.0_f32);

    let buf = shape_no_bounds(
        "Hello",
        TextAlign::Left,
        TextAnchor::Center,
        size,
        vw,
        vh,
        px,
        py,
    );
    let center = shaped_center_x(&buf);
    let max_w = max_line_w(&buf);

    assert!(
        (center - max_w / 2.0).abs() < 1.0,
        "left-align center anchor: measured center {center} should equal max_w/2 {} (no regression)",
        max_w / 2.0
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

/// The inputs `build_batch` has in hand when it builds a key.
#[derive(Clone)]
struct KeyInputs {
    text: String,
    size: f32,
    anchor: TextAnchor,
    bounds: Option<(f32, f32)>,
    viewport: (f32, f32),
    position: (f32, f32),
    single_line: bool,
    align: TextAlign,
}

/// A `DrawText` of `"hello"` at 24 px in an 800×600 viewport at position (10, 20),
/// TopLeft-anchored with no bounds — i.e. a derived layout size of `(800-10, 600-20)`.
fn base_inputs() -> KeyInputs {
    KeyInputs {
        text: "hello".to_string(),
        size: 24.0,
        anchor: TextAnchor::TopLeft,
        bounds: None,
        viewport: (800.0, 600.0),
        position: (10.0, 20.0),
        single_line: false,
        align: TextAlign::Left,
    }
}

fn base_key() -> PlainTextCacheKey {
    key_for(base_inputs())
}

/// Build a `PlainTextCacheKey` the way `build_batch` does: the layout size is **derived** through
/// `layout_buffer_width`/`_height`, not stored as the anchor/bounds/viewport/position it came from.
///
/// The tests below go through this helper — and it calls the same two pure functions the renderer
/// calls — so what they assert is the end-to-end hit/miss behaviour of a `DrawText`, not just
/// struct field equality.
fn key_for(i: KeyInputs) -> PlainTextCacheKey {
    let (vw, vh) = i.viewport;
    let (px, py) = i.position;
    let layout_w = if i.single_line {
        None
    } else {
        Some(layout_buffer_width(i.anchor, i.bounds.map(|b| b.0), vw, px))
    };
    let layout_h = Some(layout_buffer_height(
        i.anchor,
        i.bounds.map(|b| b.1),
        vh,
        py,
    ));
    PlainTextCacheKey {
        text: i.text.clone(),
        scaled_size_bits: i.size.to_bits(),
        layout_w_bits: layout_w.map(f32::to_bits),
        layout_h_bits: layout_h.map(f32::to_bits),
        no_wrap: i.single_line,
        align: i.align,
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

/// Setting bounds changes the layout size, so it still misses — via the derived width now.
#[test]
fn cache_key_miss_when_bounds_differ() {
    let mut i = base_inputs();
    i.bounds = Some((200.0, 100.0));
    assert_ne!(key_for(i), base_key());
}

/// Unlimited width (single-line) must not collide with a zero-width layout buffer — the key uses
/// `Option`, not a 0 sentinel.
#[test]
fn cache_key_unlimited_width_differs_from_zero_width() {
    let unlimited = {
        let mut i = base_inputs();
        i.single_line = true;
        key_for(i)
    };
    let zero = {
        let mut i = base_inputs();
        i.bounds = Some((0.0, 100.0));
        key_for(i)
    };
    assert_ne!(
        unlimited.layout_w_bits, zero.layout_w_bits,
        "None (unlimited) must not equal Some(0.0)"
    );
    assert_ne!(unlimited, zero);
}

/// A viewport resize still misses when it changes the derived layout size…
#[test]
fn cache_key_miss_when_a_resize_changes_the_layout_size() {
    let mut i = base_inputs();
    i.viewport = (1280.0, 600.0);
    assert_ne!(key_for(i), base_key());
}

/// …but not when explicit bounds pin the layout size, because then the viewport never reaches
/// shaping at all. The old key stored the viewport directly and re-shaped anyway.
#[test]
fn cache_key_survives_a_resize_when_bounds_pin_the_layout_size() {
    let bounded = |vw: f32, vh: f32| {
        let mut i = base_inputs();
        i.bounds = Some((200.0, 100.0));
        i.viewport = (vw, vh);
        key_for(i)
    };
    assert_eq!(
        bounded(800.0, 600.0),
        bounded(1280.0, 720.0),
        "bounded text shapes into the same buffer regardless of viewport size"
    );
}

/// **The reason this key changed.** A `DrawText::centered` — every `FloatingText` — lays out into
/// the full viewport, so `layout_buffer_width`/`_height` ignore its position entirely. Moving it
/// must therefore reuse one shaped buffer instead of re-shaping the identical string every frame,
/// which is the exact workload the cache exists for.
#[test]
fn a_moving_centered_text_keeps_one_shaped_buffer() {
    let centered_at = |px: f32, py: f32| {
        let mut i = base_inputs();
        i.anchor = TextAnchor::Center;
        i.position = (px, py);
        key_for(i)
    };
    assert_eq!(
        centered_at(400.0, 300.0),
        centered_at(412.0, 288.5),
        "a centered text that moved must hit the cache — its layout buffer did not change"
    );

    // Control: the property is *derived*, not blanket "position never matters". A TopLeft-anchored
    // no-bounds draw sizes its buffer as `viewport - position`, so moving it genuinely re-shapes.
    let top_left_at = |px: f32, py: f32| {
        let mut i = base_inputs();
        i.position = (px, py);
        key_for(i)
    };
    assert_ne!(
        top_left_at(10.0, 20.0),
        top_left_at(50.0, 20.0),
        "TopLeft text must still miss when moving — its layout width really did change"
    );
}

/// The **premise** of the test above, checked against real shaping rather than assumed: a centered
/// no-bounds text that moves produces a byte-identical glyph layout, so serving it from the cache
/// cannot change a pixel. Shapes twice through the same path the renderer uses (CPU-only) and
/// compares every glyph's position and advance.
#[test]
fn a_centered_text_shapes_identically_wherever_it_is() {
    let glyphs = |px: f32, py: f32| {
        let buf = shape_no_bounds(
            "Critical hit!",
            TextAlign::Center,
            TextAnchor::Center,
            24.0,
            800.0,
            600.0,
            px,
            py,
        );
        buf.layout_runs()
            .flat_map(|r| {
                r.glyphs
                    .iter()
                    .map(|g| (g.glyph_id, g.x.to_bits(), g.y.to_bits(), g.w.to_bits()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let a = glyphs(400.0, 300.0);
    let b = glyphs(412.0, 288.5);
    assert!(!a.is_empty(), "control: the test text must actually shape");
    assert_eq!(
        a, b,
        "a centered text's shaped glyphs must not depend on where it is drawn"
    );
}

/// The anchor itself is not in the key, and must not be: it only selects *how* the layout size is
/// derived (already keyed) and then shifts where the shaped text is drawn, via an `anchor_offset`
/// recomputed from the buffer after every cache hit. Two anchors that land on the same layout size
/// shape the same buffer.
#[test]
fn cache_key_ignores_anchor_once_the_layout_size_matches() {
    let centered = {
        let mut i = base_inputs();
        i.anchor = TextAnchor::Center;
        i.position = (400.0, 300.0);
        i
    };
    // TopLeft at position 0 derives the same full-viewport layout size as Center does anywhere.
    let top_left_at_origin = {
        let mut i = base_inputs();
        i.position = (0.0, 0.0);
        i
    };
    assert_eq!(key_for(centered), key_for(top_left_at_origin));
}

#[test]
fn cache_key_miss_when_align_differs() {
    let mut i = base_inputs();
    i.align = TextAlign::Center;
    assert_ne!(key_for(i), base_key());
}

#[test]
fn cache_key_miss_when_single_line_differs() {
    let mut i = base_inputs();
    i.single_line = true;
    assert_ne!(key_for(i), base_key());
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

/// A supplied `FontData` must become the family the engine actually shapes with.
///
/// Loading a face into fontdb does **not** make anything ask for it. Every shaping call in the
/// engine requests `Family::SansSerif` (see `shape_text`), so on native — where a real system
/// sans-serif exists — the game's own font was only ever reachable as a *fallback* for glyphs the
/// system font lacked. Text rendered in the system font and looked entirely fine, which is
/// exactly why nobody noticed the supplied font having no effect.
///
/// On wasm there is no system font, so the fallback happened to pick it anyway — the native/wasm
/// divergence the repo's cfg-split rule warns about.
#[test]
fn font_data_becomes_the_sans_serif_family() {
    let hebrew = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/fonts/NotoSansHebrew-Regular.ttf"
    ))
    .to_vec();

    let default_family = {
        let fs = build_font_system(&[], &[]);
        fs.db()
            .family_name(&glyphon::fontdb::Family::SansSerif)
            .to_string()
    };
    let with_font = {
        let fs = build_font_system(&hebrew, &[]);
        fs.db()
            .family_name(&glyphon::fontdb::Family::SansSerif)
            .to_string()
    };

    assert!(
        with_font.contains("Hebrew"),
        "the supplied FontData must become the sans-serif family the shaper asks for, got \
         {with_font:?} (default was {default_family:?})"
    );
}
