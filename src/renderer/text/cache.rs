use std::sync::Arc;

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
/// ⚠️ **`text` is an `Arc<str>`, and that is a measurement, not a style choice.** The lookup
/// path has to *build* one of these keys before it can probe the map — `HashMap::remove` takes
/// `&Q where K: Borrow<Q>`, and no `Q` borrows a struct that owns its text — so a `String` here
/// meant one string copy per plain `DrawText` per frame, **on a cache hit as much as a miss**.
/// With an `Arc<str>` the key clone is a refcount bump, and [`TextInterner`] hands the caller an
/// owned `Arc<str>` back from a `&str` probe, so a text drawn again costs nothing at all.
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
    pub(super) text: Arc<str>,
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

/// The set of text strings the shaped-buffer cache currently holds, each as one `Arc<str>`.
///
/// Its whole job is to turn a `&str` into an **owned** `Arc<str>` without allocating when the
/// string is already cached, which is what makes [`PlainTextCacheKey`] cheap to build for a
/// lookup. `Arc<str>: Borrow<str>`, so `HashSet::get` probes it with a plain `&str`.
///
/// Measured, on a steady-state frame whose draws are all cache hits (standalone counting
/// allocator over the key-construction + probe path, `String` key vs this one):
///
/// | frame | before | after |
/// |---|---|---|
/// | 6 static HUD lines | 6 allocs / 63 B | **0 / 0** |
/// | 40 `FloatingText` | 40 allocs / 74 B | **0 / 0** |
/// | 12 dialogue lines (~60 chars) | 12 allocs / 686 B | **0 / 0** |
///
/// ⚠️ **The miss path was checked too, and it is what rejected the obvious alternative.** A
/// frame of six all-new strings costs 8 allocations here against the `String` key's 7 — the one
/// extra is this set's amortised table growth. The two-level `HashMap<Arc<str>, HashMap<..>>`
/// that suggests itself instead reads 0 on hits as well, but **13** on that same control,
/// because every new string also allocates an inner map. Free on hits is not worth a 2x miss
/// path when a score readout changes its text every frame.
#[derive(Default)]
pub(super) struct TextInterner {
    texts: std::collections::HashSet<Arc<str>>,
}

impl TextInterner {
    /// The interned `Arc<str>` for `text`, allocating one only if this is the first time the
    /// cache has seen this string.
    pub(super) fn intern(&mut self, text: &str) -> Arc<str> {
        if let Some(existing) = self.texts.get(text) {
            return Arc::clone(existing);
        }
        let arc: Arc<str> = Arc::from(text);
        self.texts.insert(Arc::clone(&arc));
        arc
    }

    /// Drops every string no cache key refers to any more. Called at the frame boundary,
    /// **after** the shaped-buffer cache has evicted its own stale entries.
    ///
    /// A count of 1 means this set is the only holder left, which is exactly the condition
    /// "no cached key names this text". Reading `strong_count` is sound here because the
    /// caller holds `&mut TextRenderer` and every key built during the frame has already been
    /// re-inserted into the cache or dropped — there is no outstanding clone to race with. A
    /// leaked one would only delay an eviction by a frame, never free a live string.
    pub(super) fn evict_unreferenced(&mut self) {
        self.texts.retain(|t| Arc::strong_count(t) > 1);
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.texts.len()
    }
}
