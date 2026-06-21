/// In-game egui debug overlay resource.
///
/// Inserted as a Resource into the ECS World; draw egui windows from a `System` via `debug_ui.ctx()`.
/// Toggle with the F1 key. Draw calls are skipped when disabled.
///
/// # Usage
/// ```rust,no_run
/// # use engine::{DebugUi, System, World};
/// struct MyDebugPanel;
/// impl System for MyDebugPanel {
///     fn run(&mut self, world: &mut World, _dt: f32) {
///         let debug = world.resource::<DebugUi>().unwrap();
///         if !debug.is_enabled() { return; }
///         egui::Window::new("Stats").show(debug.ctx(), |ui| {
///             ui.label("Hello from debug!");
///         });
///     }
/// }
/// ```
/// Bundled Korean font (Noto Sans KR, Regular — **Hangul-only subset**). Installed as an egui
/// fallback so the editor / debug overlay can render Hangul — the default egui fonts cover only
/// Latin + Cyrillic, so CJK text would otherwise show as `□` (tofu) boxes.
///
/// This is a subset (~2.3 MB vs the ~5.9 MB original) carrying the full modern Hangul syllables
/// block + Latin + common punctuation, with the Hanja (CJK ideographs) and unused OpenType layout
/// tables dropped so the crate stays under the crates.io 10 MB package limit. Archaic Hangul and
/// Hanja are not covered. See `scripts/subset_korean_font.sh` to regenerate, and
/// `assets/fonts/NotoSansKR-OFL.txt` for the (SIL OFL 1.1) license.
const KOREAN_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/NotoSansKR-Regular-subset.ttf"
));

/// Installs the bundled Korean font as the **lowest-priority** fallback for both the proportional
/// and monospace families. Latin/Cyrillic text keeps egui's default font (so existing metrics and
/// look are unchanged); Noto Sans KR is consulted only for glyphs the default font lacks (Hangul,
/// other CJK). Idempotent — egui skips the insert if a font with this name is already present.
fn install_korean_fallback(ctx: &egui::Context) {
    use egui::epaint::text::{FontPriority, InsertFontFamily};
    ctx.add_font(egui::epaint::text::FontInsert::new(
        "noto_sans_kr",
        egui::FontData::from_static(KOREAN_FONT),
        vec![
            InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: FontPriority::Lowest,
            },
            InsertFontFamily {
                family: egui::FontFamily::Monospace,
                priority: FontPriority::Lowest,
            },
        ],
    ));
}

pub struct DebugUi {
    ctx: egui::Context,
    enabled: bool,
}

impl DebugUi {
    pub(crate) fn new_with_ctx(ctx: egui::Context) -> Self {
        install_korean_fallback(&ctx);
        Self {
            ctx,
            enabled: false,
        }
    }

    /// Returns the egui draw context. Must only be used between `begin_frame` and `end_frame`.
    ///
    /// Custom paint callbacks are currently unsupported by the engine renderer and are skipped
    /// at render time to preserve the internal render-pass lifetime safety boundary.
    pub fn ctx(&self) -> &egui::Context {
        &self.ctx
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn set_enabled(&mut self, v: bool) {
        self.enabled = v;
    }
}

#[cfg(test)]
mod tests {
    use super::KOREAN_FONT;
    use ab_glyph::{Font, FontRef};

    /// The bundled `NotoSansKR-Regular-subset.ttf` is a subset; guard that it still parses with
    /// egui's rasterizer (ab_glyph) and actually carries the glyphs the editor relies on, so a
    /// future bad regeneration (e.g. dropping the Hangul block) fails loudly instead of shipping
    /// `□` tofu. Covers the whole modern Hangul range via its endpoints + chars seen in the UI.
    #[test]
    fn korean_subset_font_parses_and_has_hangul() {
        let font = FontRef::try_from_slice(KOREAN_FONT).expect("subset font must parse");
        // Modern Hangul syllables block endpoints (U+AC00 가 .. U+D7A3 힣) + UI/data samples.
        for ch in ['가', '힣', '한', '국', '어', '엔', '티', '저', '장'] {
            assert_ne!(
                font.glyph_id(ch).0,
                0,
                "subset font is missing a glyph for {ch:?} (U+{:04X})",
                ch as u32
            );
        }
        // Latin is kept too (the fallback can cover it even though egui's default also does).
        assert_ne!(font.glyph_id('A').0, 0, "subset font dropped Basic Latin");
    }
}
