use crate::app::App;

use super::state::EditorState;
use super::PaintTool;

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
        }
    }

    pub(in crate::app) fn apply_to(&self, s: &mut EditorState) {
        s.snap_enabled = self.snap_enabled;
        s.snap_size = self.snap_size;
        s.show_grid = self.show_grid;
        s.show_bounds = self.show_bounds;
        s.show_pathgrid = self.show_pathgrid;
        s.paint_brush = self.paint_brush;
        s.paint_tool = self.paint_tool;
    }
}

/// Config-dir path for the persisted editor settings.
fn editor_settings_path() -> std::path::PathBuf {
    crate::save::save_path("skeleton-engine", "editor_settings.ron")
}

impl App {
    /// Write the current docked-editor preferences to the RON config file.
    pub(in crate::app) fn save_editor_settings(&self) {
        let settings = EditorSettings::from_state(&self.editor);
        let _ = crate::save::write_ron(&editor_settings_path(), &settings);
    }

    /// Load persisted editor preferences (if the config file exists) and apply them.
    pub(in crate::app) fn load_editor_settings(&mut self) {
        let path = editor_settings_path();
        if crate::save::exists(&path) {
            if let Ok(settings) = crate::save::read_ron::<EditorSettings>(&path) {
                settings.apply_to(&mut self.editor);
            }
        }
    }
}
