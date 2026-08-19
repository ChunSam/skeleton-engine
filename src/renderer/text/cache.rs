use glyphon::Buffer;

use super::queue::TextAlign;

/// Cache key for a plain (non-rich) shaped text buffer.
///
/// **This key is [`ShapeSpec`](super::renderer::ShapeSpec), field for field.** That is the whole
/// design rule, and it is what makes the cache both correct and useful: `shape_text` is a pure
/// function of a `ShapeSpec` and the (fixed) `FontSystem`, so two draws with equal specs *must*
/// shape identically, and two draws with different specs *may* not. Anything that is not a
/// `ShapeSpec` field does not belong here — if it mattered to shaping it would be one.
///
/// `f32` fields are stored as bit patterns so the key is `Eq + Hash` without precision issues.
///
/// ⚠️ **The layout `width`/`height` are the computed values, not the inputs they came from.**
/// A `DrawText`'s `position`, the viewport size, its `bounds` and its `anchor` reach shaping
/// **only** through `layout_buffer_width`/`_height`, and those two functions ignore `position`
/// entirely whenever `bounds` is set *or* the anchor is `Center` (their own tests
/// `explicit_bounds_override_anchor` and `centered_no_bounds_uses_full_viewport_width` say so).
/// Keying on `position` therefore missed on every frame a centered text *moved* while shaping the
/// byte-identical buffer again — i.e. every `FloatingText`, which is the exact workload the cache
/// was built for. Keying on the computed pair fixes that and subsumes all four inputs.
///
/// Rich text is NOT cached here (span attrs are hard to hash; see
/// `TextRenderer::shaped_buffer_cache`).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(super) struct PlainTextCacheKey {
    pub(super) text: String,
    /// Font size in pixels × scale_factor, as f32 bits.
    pub(super) scaled_size_bits: u32,
    /// Computed layout-buffer width, as f32 bits. `None` = unlimited (single-line, no wrap).
    pub(super) layout_w_bits: Option<u32>,
    /// Computed layout-buffer height, as f32 bits. `None` = unlimited.
    pub(super) layout_h_bits: Option<u32>,
    /// `Wrap::None` rather than `Wrap::WordOrGlyph` — the single-line (TextInput) path.
    ///
    /// Redundant with `layout_w_bits.is_none()` today, since the caller derives both from the same
    /// `single_line`. It is kept explicit because it is a distinct `ShapeSpec` field: a future
    /// unlimited-width *wrapping* spec would make them disagree, and the key must follow
    /// `ShapeSpec`, not the current caller.
    ///
    /// The caret *byte offset* is deliberately NOT part of the key: scroll is always recomputed
    /// from the shaped buffer after a cache hit (via `caret_x`), so two carets over identical text
    /// safely share one buffer.
    pub(super) no_wrap: bool,
    pub(super) align: TextAlign,
}

pub(super) struct CachedBuffer {
    pub(super) buffer: Buffer,
    /// Frame generation when this entry was last used. Entries not accessed in
    /// the current frame are evicted at the end of `render()`.
    pub(super) last_used_gen: u64,
}
