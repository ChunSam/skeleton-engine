//! Font-data resources picked up by `TextRenderer` at startup.

/// Font bytes used by the game. Insert before `App::run()` for `TextRenderer` to pick it up.
pub struct FontData(pub Vec<u8>);

/// Additional font blobs loaded alongside [`FontData`] for multi-script coverage — e.g. a Latin UI
/// font in `FontData` plus an RTL-script font (Hebrew/Arabic) here. cosmic-text falls back across all
/// loaded fonts by script, so a single `DrawText` containing mixed LTR + RTL text shapes correctly.
/// Insert before `App::run()` for `TextRenderer` to pick it up.
#[derive(Default)]
pub struct ExtraFonts(pub Vec<Vec<u8>>);
