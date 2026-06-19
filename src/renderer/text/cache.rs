use glyphon::Buffer;

use super::queue::{TextAlign, TextAnchor};

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
pub(super) struct PlainTextCacheKey {
    pub(super) text: String,
    /// Font size in pixels × scale_factor, as f32 bits.
    pub(super) scaled_size_bits: u32,
    /// Scaled bounds, if set. `None` = no explicit bounds (buffer size derived from viewport).
    pub(super) bounds_w_bits: Option<u32>, // None or Some(f32::to_bits())
    pub(super) bounds_h_bits: Option<u32>,
    /// Viewport dimensions (affect buffer size when no explicit bounds).
    pub(super) viewport_w: u32,
    pub(super) viewport_h: u32,
    /// Position (affects buffer width/height for TopLeft anchor).
    pub(super) position_x_bits: u32,
    pub(super) position_y_bits: u32,
    /// Whether the DrawText is in single-line mode (affects Wrap). The caret *byte offset* is
    /// deliberately NOT part of the key: scroll is always recomputed from the shaped buffer after
    /// a cache hit (via `caret_x`), so two carets over identical text safely share one buffer.
    pub(super) is_single_line: bool,
    pub(super) align: TextAlign,
    pub(super) anchor: TextAnchor,
}

pub(super) struct CachedBuffer {
    pub(super) buffer: Buffer,
    /// Frame generation when this entry was last used. Entries not accessed in
    /// the current frame are evicted at the end of `render()`.
    pub(super) last_used_gen: u64,
}
