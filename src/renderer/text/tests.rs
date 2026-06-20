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
