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
/// Bundled Korean font (Noto Sans KR, Regular). Installed as an egui fallback so the editor /
/// debug overlay can render Hangul — the default egui fonts cover only Latin + Cyrillic, so CJK
/// text would otherwise show as `□` (tofu) boxes.
const KOREAN_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/NotoSansKR-Regular.ttf"
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
