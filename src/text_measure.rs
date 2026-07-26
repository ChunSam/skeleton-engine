//! Measure shaped text **before** drawing it — the sizing primitive for text-fitted UI.
//!
//! Every panel that has to wrap around a string (a tooltip, a name chip, a speech bubble)
//! otherwise guesses its size from a `chars × px` heuristic, which is wrong the moment the
//! string mixes scripts: at size 15 a Hangul glyph advances ≈ 15 px while `4` or `g` advances
//! ≈ 8 px, so `"사슬갑옷"` and `"200g"` — both 4 characters — differ by nearly 2×. The
//! heuristic is padded until it stops clipping, and the panel is loose forever after.
//!
//! [`TextMeasurer`] shapes the string through the **same** code path the renderer uses (the
//! shared `shape_text` helper: same metrics, same `Shaping::Advanced`, same font stack incl.
//! [`ExtraFonts`] script fallback) and reports the resulting
//! advance width and height. Sizing a panel from it is exact, not padded.
//!
//! ```rust,no_run
//! # use engine::{App, DrawText, TextQueue};
//! # let mut app = App::new();
//! let label = "사슬갑옷";
//! let size = app.measure_text(label, 15.0);
//! // `size.x` is the width the very same string will occupy when drawn at 15.0.
//! ```
//!
//! ## Coordinate space
//!
//! Measurements are in the **same logical pixels as [`DrawText`](crate::DrawText)** —
//! `position`, `size` and the returned extent all live before
//! [`DisplayScaleFactor`](crate::resources::DisplayScaleFactor) /
//! [`Letterbox`](crate::resources::Letterbox) are applied, so a measured width can be used
//! directly as a [`DrawRect`](crate::DrawRect) width. (The renderer shapes at the scaled size
//! for crispness; that is a HiDPI implementation detail, not a different coordinate space.)
//!
//! ## Cost
//!
//! The first measurement builds a [`FontSystem`], which on native scans the system font
//! directory — a one-time cost of a few tens of milliseconds. It is paid lazily, so a game
//! that never measures never pays it; to move it off the first frame, insert the resource
//! yourself during setup (see [`TextMeasurer::new`]). The measurer is registered persistent,
//! so a scene change does not rebuild it.

use glam::Vec2;
use glyphon::{FontSystem, Wrap};

use crate::app::App;
use crate::ecs::World;
use crate::renderer::text::{build_font_system, shape_text, ShapeSpec, LINE_HEIGHT_FACTOR};
use crate::resources::{ExtraFonts, FontData};

/// Shapes text on the CPU to report how much room it will need.
///
/// Lives in the [`World`] as a resource (lazily created by [`measure_text`]) and owns its own
/// [`FontSystem`], so measuring from inside a `System::run` never borrows against the GPU
/// renderer's font system — the two are independent instances built from the same font blobs.
///
/// See the [module docs](self) for the coordinate space and the one-time construction cost.
pub struct TextMeasurer {
    font_system: FontSystem,
}

impl TextMeasurer {
    /// Build a measurer over `font_data` plus every blob in `extra_fonts` — the same pair the
    /// renderer is initialized from ([`FontData`] / [`ExtraFonts`]).
    ///
    /// Insert the result as a resource during setup to pay the font-scan cost up front:
    ///
    /// ```rust,no_run
    /// # use engine::{App, TextMeasurer};
    /// # let mut app = App::new();
    /// # let my_font: Vec<u8> = Vec::new();
    /// app.world.insert_resource(TextMeasurer::new(&my_font, &[]));
    /// ```
    pub fn new(font_data: &[u8], extra_fonts: &[Vec<u8>]) -> Self {
        Self {
            font_system: build_font_system(font_data, extra_fonts),
        }
    }

    /// Width and height of `text` drawn at `font_size` on a **single line** (no wrapping).
    ///
    /// This is the tooltip/chip/button case: the returned `x` is the advance width the string
    /// will occupy, `y` is one line's height (see [`line_height`](Self::line_height)) — or the
    /// height of however many lines an embedded `\n` produces.
    ///
    /// Empty text measures [`Vec2::ZERO`] (a panel around nothing has no size).
    pub fn measure(&mut self, text: &str, font_size: f32) -> Vec2 {
        self.measure_inner(text, font_size, None, Wrap::None, false)
    }

    /// Like [`measure`](Self::measure) but the string carries `[color=…]` / `[b]` / `[i]`
    /// markup, as drawn by [`DrawText::rich`](crate::DrawText::rich).
    ///
    /// The markup is parsed away first, so only the visible glyphs are measured — passing a
    /// rich string to plain [`measure`](Self::measure) would count the tags as text and
    /// over-report the width.
    pub fn measure_rich(&mut self, text: &str, font_size: f32) -> Vec2 {
        self.measure_inner(text, font_size, None, Wrap::None, true)
    }

    /// Width and height of `text` at `font_size` when wrapped to at most `max_width`.
    ///
    /// The returned `x` is the width of the **longest resulting line** (never more than
    /// `max_width`), so a panel sized by it is tight rather than always `max_width` wide; `y`
    /// grows with the line count. Wrapping is by word, falling back to per-glyph for a word
    /// that cannot fit — the same rule a bounded [`DrawText`](crate::DrawText) follows.
    ///
    /// A non-positive `max_width` measures as unwrapped (there is no width to wrap into).
    pub fn measure_wrapped(&mut self, text: &str, font_size: f32, max_width: f32) -> Vec2 {
        if max_width <= 0.0 {
            return self.measure(text, font_size);
        }
        self.measure_inner(text, font_size, Some(max_width), Wrap::WordOrGlyph, false)
    }

    /// Height of one line of text at `font_size` — the engine-wide `1.2 × font_size`.
    ///
    /// Useful for laying out rows before their contents are known (`n as f32 *
    /// TextMeasurer::line_height(size)`), and for padding a measured box vertically.
    pub fn line_height(font_size: f32) -> f32 {
        font_size * LINE_HEIGHT_FACTOR
    }

    fn measure_inner(
        &mut self,
        text: &str,
        font_size: f32,
        width: Option<f32>,
        wrap: Wrap,
        rich: bool,
    ) -> Vec2 {
        if text.is_empty() {
            return Vec2::ZERO;
        }
        let buf = shape_text(
            &mut self.font_system,
            &ShapeSpec {
                text,
                size: font_size,
                width,
                // Unlimited: a measurement must never be clipped by a layout box it does not have.
                height: None,
                wrap,
                // Alignment moves glyphs inside the layout box; it cannot change how wide they are.
                align: None,
                rich,
            },
        );
        let mut max_w: f32 = 0.0;
        let mut lines = 0.0f32;
        for run in buf.layout_runs() {
            max_w = max_w.max(run.line_w);
            lines += 1.0;
        }
        Vec2::new(max_w, lines * Self::line_height(font_size))
    }
}

/// The font blobs a text stack should be built from: [`FontData`] plus [`ExtraFonts`].
///
/// Shared by the GPU renderer's init and [`TextMeasurer`]'s lazy construction so the two
/// always load the identical font stack — a measurement taken against a different set of
/// fonts than the renderer draws with would be silently wrong for exactly the fallback
/// characters (CJK, RTL) that motivate measuring in the first place.
pub(crate) fn font_blobs(world: &World) -> (Vec<u8>, Vec<Vec<u8>>) {
    let font_bytes = world
        .resource::<FontData>()
        .map(|f| f.0.clone())
        .unwrap_or_default();
    // WASM: the browser sandbox has no system fonts, so cosmic-text would panic when it
    // shapes text against an empty font db. Fall back to the engine's embedded default font
    // when the game supplies no `FontData`, so `DrawText`/HUD text renders out of the box.
    // Native loads system fonts inside `FontSystem::new()` and does not embed the default.
    #[cfg(target_arch = "wasm32")]
    let font_bytes = if font_bytes.is_empty() {
        crate::renderer::DEFAULT_FONT.to_vec()
    } else {
        font_bytes
    };
    let extra_fonts = world
        .resource::<ExtraFonts>()
        .map(|f| f.0.clone())
        .unwrap_or_default();
    (font_bytes, extra_fonts)
}

/// The world's [`TextMeasurer`], created from [`FontData`]/[`ExtraFonts`] on first use.
///
/// Callable from a `System::run` — it borrows only the world, never the renderer.
///
/// ```rust,no_run
/// # use engine::{ecs::World, text_measurer};
/// # fn f(world: &mut World) {
/// let w = text_measurer(world).measure("사슬갑옷", 15.0).x;
/// # }
/// ```
pub fn text_measurer(world: &mut World) -> &mut TextMeasurer {
    if world.resource::<TextMeasurer>().is_none() {
        let (font_bytes, extra_fonts) = font_blobs(world);
        world.insert_resource(TextMeasurer::new(&font_bytes, &extra_fonts));
    }
    world
        .resource_mut::<TextMeasurer>()
        .expect("TextMeasurer was just inserted")
}

/// Measure `text` at `font_size` on a single line — the world-level [`TextMeasurer::measure`].
///
/// The form to call from a system; see [`text_measurer`] for the underlying resource.
pub fn measure_text(world: &mut World, text: &str, font_size: f32) -> Vec2 {
    text_measurer(world).measure(text, font_size)
}

/// Measure `text` at `font_size` wrapped to `max_width` — the world-level
/// [`TextMeasurer::measure_wrapped`].
pub fn measure_text_wrapped(world: &mut World, text: &str, font_size: f32, max_width: f32) -> Vec2 {
    text_measurer(world).measure_wrapped(text, font_size, max_width)
}

impl App {
    /// Measure `text` at `font_size` on a single line, in [`DrawText`](crate::DrawText)'s
    /// logical pixels.
    ///
    /// The `&mut App` twin of [`measure_text`] — use it from setup / `on_enter`; use the free
    /// function from inside a system.
    pub fn measure_text(&mut self, text: &str, font_size: f32) -> Vec2 {
        measure_text(&mut self.world, text, font_size)
    }

    /// Measure `text` at `font_size` wrapped to `max_width`. See [`measure_text_wrapped`].
    pub fn measure_text_wrapped(&mut self, text: &str, font_size: f32, max_width: f32) -> Vec2 {
        measure_text_wrapped(&mut self.world, text, font_size, max_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bundled DejaVu Sans — the same fixture the text-renderer tests shape against, so
    /// measurements are deterministic across machines (a bare system-font scan would make
    /// widths depend on what happens to be installed).
    fn test_font() -> Vec<u8> {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/DejaVuSans.ttf"
        ))
        .to_vec()
    }

    fn measurer() -> TextMeasurer {
        TextMeasurer::new(&test_font(), &[])
    }

    #[test]
    fn measures_ascii_text() {
        let mut m = measurer();
        let size = m.measure("Hello", 16.0);
        assert!(size.x > 0.0, "shaped text must have width, got {}", size.x);
        assert_eq!(size.y, 16.0 * 1.2, "one line at 16px");
    }

    #[test]
    fn empty_text_measures_zero() {
        let mut m = measurer();
        assert_eq!(m.measure("", 16.0), Vec2::ZERO);
        assert_eq!(m.measure_wrapped("", 16.0, 100.0), Vec2::ZERO);
    }

    #[test]
    fn longer_text_is_wider() {
        let mut m = measurer();
        assert!(m.measure("aa", 16.0).x > m.measure("a", 16.0).x);
    }

    /// The whole point of the API: a `chars × px` guess cannot tell these apart, but the
    /// shaped widths differ substantially because the glyphs advance differently.
    #[test]
    fn same_char_count_can_differ_in_width() {
        let mut m = measurer();
        let narrow = m.measure("iiii", 16.0).x;
        let wide = m.measure("WWWW", 16.0).x;
        assert!(
            wide > narrow * 1.5,
            "4 wide glyphs ({wide}) should far exceed 4 narrow ones ({narrow})"
        );
    }

    #[test]
    fn width_scales_with_font_size() {
        let mut m = measurer();
        let small = m.measure("measure", 10.0).x;
        let large = m.measure("measure", 20.0).x;
        let ratio = large / small;
        assert!(
            (1.8..=2.2).contains(&ratio),
            "doubling the size should roughly double the width, got ratio {ratio}"
        );
    }

    #[test]
    fn newline_adds_a_line_of_height() {
        let mut m = measurer();
        let one = m.measure("one", 16.0);
        let two = m.measure("one\ntwo", 16.0);
        assert_eq!(two.y, one.y * 2.0);
    }

    #[test]
    fn wrapping_respects_max_width_and_grows_taller() {
        let mut m = measurer();
        let text = "the quick brown fox jumps over the lazy dog";
        let unwrapped = m.measure(text, 16.0);
        let max_width = unwrapped.x / 3.0;
        let wrapped = m.measure_wrapped(text, 16.0, max_width);
        assert!(
            wrapped.x <= max_width + 0.5,
            "longest line {} must fit in {max_width}",
            wrapped.x
        );
        assert!(
            wrapped.y > unwrapped.y,
            "wrapping into ~3 lines must be taller than one line"
        );
    }

    #[test]
    fn non_positive_max_width_does_not_wrap() {
        let mut m = measurer();
        let text = "some text that would wrap";
        assert_eq!(m.measure_wrapped(text, 16.0, 0.0), m.measure(text, 16.0));
        assert_eq!(m.measure_wrapped(text, 16.0, -10.0), m.measure(text, 16.0));
    }

    /// Rich markup must be parsed away, not measured as literal characters.
    #[test]
    fn rich_markup_is_not_counted() {
        let mut m = measurer();
        let plain = m.measure("danger", 16.0);
        let rich = m.measure_rich("[color=#ff0000]danger[/color]", 16.0);
        assert!(
            (rich.x - plain.x).abs() < 0.5,
            "rich {} should measure like plain {}",
            rich.x,
            plain.x
        );
        assert!(
            m.measure("[color=#ff0000]danger[/color]", 16.0).x > plain.x * 2.0,
            "measuring rich text as plain over-reports — that's why measure_rich exists"
        );
    }

    #[test]
    fn line_height_is_the_shared_factor() {
        assert_eq!(TextMeasurer::line_height(10.0), 12.0);
        assert_eq!(TextMeasurer::line_height(0.0), 0.0);
    }

    #[test]
    fn world_level_measure_creates_the_resource_lazily() {
        let mut world = World::new();
        assert!(world.resource::<TextMeasurer>().is_none());
        world.insert_resource(FontData(test_font()));
        let w = measure_text(&mut world, "lazy", 16.0).x;
        assert!(w > 0.0);
        assert!(
            world.resource::<TextMeasurer>().is_some(),
            "first measurement must install the resource"
        );
    }
}
