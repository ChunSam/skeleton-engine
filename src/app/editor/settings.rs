use crate::app::App;

use super::state::{mode_transition, EditorState};
use super::{EditorMode, PaintTool};

/// The snap size a settings file is allowed to hand the editor: the toolbar's `1..=128`, with the
/// editor's default for anything that is not a number. The file is the one input the UI's
/// clamps never see, and `0.0` — the derived `Default` — made every snapped drag NaN
/// (v0.156.14).
pub(in crate::app) fn sanitize_snap_size(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(1.0, 128.0)
    } else {
        16.0
    }
}

/// Persisted docked-editor preferences (snap / grid / paint tool + brush). Written to a RON
/// config file when the editor closes and restored when it next opens.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub(in crate::app) struct EditorSettings {
    pub snap_enabled: bool,
    pub snap_size: f32,
    pub show_grid: bool,
    #[serde(default)]
    pub show_bounds: bool,
    #[serde(default)]
    pub show_pathgrid: bool,
    pub paint_brush: u32,
    pub paint_tool: PaintTool,
    /// Editor UI language (English / Korean). Defaults to Korean for old settings files.
    #[serde(default)]
    pub locale: super::EditorLocale,
}

impl EditorSettings {
    pub(in crate::app) fn from_state(s: &EditorState) -> Self {
        Self {
            snap_enabled: s.snap_enabled,
            snap_size: s.snap_size,
            show_grid: s.show_grid,
            show_bounds: s.show_bounds,
            show_pathgrid: s.show_pathgrid,
            paint_brush: s.paint_brush,
            paint_tool: s.paint_tool,
            locale: s.locale,
        }
    }

    pub(in crate::app) fn apply_to(&self, s: &mut EditorState) {
        s.snap_enabled = self.snap_enabled;
        s.snap_size = sanitize_snap_size(self.snap_size);
        s.show_grid = self.show_grid;
        s.show_bounds = self.show_bounds;
        s.show_pathgrid = self.show_pathgrid;
        s.paint_brush = self.paint_brush;
        s.paint_tool = self.paint_tool;
        s.locale = self.locale;
    }
}

impl App {
    /// Path of the persisted editor settings: the config dir, unless a test overrode it.
    fn editor_settings_path(&self) -> std::path::PathBuf {
        self.editor
            .settings_path_override
            .clone()
            .unwrap_or_else(|| crate::save::save_path("skeleton-engine", "editor_settings.ron"))
    }

    /// Write the current docked-editor preferences to the RON config file.
    pub(in crate::app) fn save_editor_settings(&self) {
        let settings = EditorSettings::from_state(&self.editor);
        let _ = crate::save::write_ron(&self.editor_settings_path(), &settings);
    }

    /// Load persisted editor preferences (if the config file exists) and apply them. A file
    /// that exists but does not parse is logged and left alone — the in-memory defaults stand,
    /// and the next save overwrites it; silently reverting every preference used to be the
    /// only evidence that the file was corrupt.
    pub(in crate::app) fn load_editor_settings(&mut self) {
        let path = self.editor_settings_path();
        if crate::save::exists(&path) {
            match crate::save::read_ron::<EditorSettings>(&path) {
                Ok(settings) => settings.apply_to(&mut self.editor),
                Err(e) => log::warn!(
                    "editor settings at {} did not parse ({e}); keeping the defaults, and the next \
                     save will overwrite the file",
                    path.display()
                ),
            }
        }
    }

    /// Switches the editor mode and does everything the switch implies — the settings load on
    /// the first Docked open, the settings save on every Docked exit, the pause reset, the
    /// `DebugUi` sync — as decided by [`mode_transition`]. **The one entry point** for the F1 and
    /// F2 keys and the toolbar's Exit button.
    pub(in crate::app) fn set_editor_mode(&mut self, new_mode: EditorMode) {
        let t = mode_transition(self.editor.mode, new_mode, self.editor.settings_loaded);
        self.editor.mode = new_mode;
        if t.load_settings {
            self.load_editor_settings();
            self.editor.settings_loaded = true;
        }
        if t.save_settings {
            self.save_editor_settings();
        }
        if t.resume {
            self.editor.paused = false;
            self.editor.step_once = false;
        }
        // `DebugUi.enabled` is true only in Overlay mode, so systems that query `is_enabled()`
        // keep working there.
        if let Some(debug_ui) = self.world.resource_mut::<crate::debug_ui::DebugUi>() {
            debug_ui.set_enabled(new_mode == EditorMode::Overlay);
        }
    }
}
