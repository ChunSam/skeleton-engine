//! Screen-space text rendering, backed by glyphon 0.6.
//!
//! - `queue` — the public queue-facing types ([`DrawText`], [`TextAlign`],
//!   [`TextAnchor`], [`TextQueue`]).
//! - `renderer` — the GPU-backed [`TextRenderer`] plus pure layout helpers.
//! - `cache` — the cross-frame shaped-buffer cache types.
//! - `rich_text` — the `[color]`/`[b]`/`[i]` markup parser.
//! - `layering` — z-interleaving of layered text batches with the UI-primitive runs.

mod cache;
mod layering;
mod queue;
mod renderer;
mod rich_text;
#[cfg(test)]
mod tests;

pub(crate) use layering::interleave_runs;
pub use queue::{DrawText, TextAlign, TextAnchor, TextQueue};
pub use renderer::TextRenderer;
