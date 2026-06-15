use crate::ecs::Entity;

#[cfg(not(target_arch = "wasm32"))]
use super::EditorHistory;

#[cfg(not(target_arch = "wasm32"))]
use crate::app::ComponentFactory;

/// One of the 8 axis-aligned resize handles drawn around a selected object.
///
/// Naming follows compass directions. `Top`/`Bottom`/`Left`/`Right` are edge
/// midpoints; the four diagonal names are corners.
///
/// Native-only: the gizmo resize path (geometry helpers + unit tests) is
/// excluded from the reduced wasm editor, so the enum is gated to match and keep
/// the wasm build free of dead-code warnings.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

/// Which editor mode is currently active.
///
/// # Key bindings
/// - **F1**: toggles Overlay. If Docked is active, exits Docked and enters Overlay.
/// - **F2**: toggles Docked. If Overlay is active, turns Overlay off first (mutual exclusion).
/// - Both modes are native-only; wasm builds compile out all Docked/Overlay state.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::app) enum EditorMode {
    /// No editor UI is visible.
    #[default]
    Off,
    /// Floating egui windows overlaid on top of the running game (pre-existing mode).
    Overlay,
    /// Full docked layout: egui owns the whole window; the game renders into an
    /// offscreen texture displayed in the central panel.
    Docked,
}

/// Active Tile Paint tool (which set of cells a viewport gesture affects).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub(in crate::app) enum PaintTool {
    /// Paint each hovered cell (an N×N brush block) while dragging.
    #[default]
    Freehand,
    /// Press-drag-release fills the rectangle spanned by the two cells.
    Rectangle,
    /// A click flood-fills the 4-connected region of same-valued cells.
    Bucket,
}

/// Apply the F1 press transition to `mode`.
///
/// Extracted as a pure function so the logic is unit-testable without spinning up App.
///
/// | Current mode | After F1 |
/// |---|---|
/// | Off | Overlay |
/// | Overlay | Off |
/// | Docked | Overlay (exits Docked, enters Overlay) |
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn apply_f1(mode: EditorMode) -> EditorMode {
    match mode {
        EditorMode::Off => EditorMode::Overlay,
        EditorMode::Overlay => EditorMode::Off,
        EditorMode::Docked => EditorMode::Overlay,
    }
}

/// Apply the F2 press transition to `mode`.
///
/// | Current mode | After F2 |
/// |---|---|
/// | Off | Docked |
/// | Overlay | Docked (Overlay is implicitly off — mutual exclusion) |
/// | Docked | Off |
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn apply_f2(mode: EditorMode) -> EditorMode {
    match mode {
        EditorMode::Off | EditorMode::Overlay => EditorMode::Docked,
        EditorMode::Docked => EditorMode::Off,
    }
}

/// All mutable state that belongs exclusively to the editor/inspector.
///
/// Grouping these fields here means a fork that removes the editor needs only to
/// delete this struct and the single `editor: EditorState` field on `App` — no
/// surgical field-by-field removal required.
#[derive(Default)]
pub(in crate::app) struct EditorState {
    /// Entity currently selected in the Inspector panel.
    pub(in crate::app) inspector_selected: Option<Entity>,
    /// Whether a gizmo drag is in progress.
    pub(in crate::app) gizmo_dragging: bool,
    /// Offset (entity position − cursor world position) captured at drag start.
    pub(in crate::app) gizmo_drag_offset: glam::Vec2,
    /// Result message of the last scene save.
    pub(in crate::app) editor_save_status: Option<String>,
    /// Current Inspector tab index (0: Entities, 1: Assets).
    pub(in crate::app) inspector_tab: u8,

    /// Multi-selected entity list (includes `inspector_selected`).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) selected_entities: Vec<Entity>,
    /// EntityDef clipboard copied via Ctrl+C.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) copy_clipboard: Vec<crate::prefab::EntityDef>,
    /// Inspector scene save path (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) editor_save_path: String,
    /// Result message of the last scene load.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) editor_load_status: Option<String>,
    /// Editor undo/redo history.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) cmd_history: EditorHistory,
    /// Entity position at gizmo drag start (for recording undo).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) gizmo_drag_start_pos: Option<glam::Vec2>,
    /// Positions of all selected entities at gizmo group-drag start (for group-move undo).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) gizmo_drag_start_positions: Vec<(Entity, glam::Vec2)>,
    /// Component add factory map (native only). Type name → closure that adds the component to the World.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) component_factories: std::collections::HashMap<String, ComponentFactory>,
    /// Component remove closure map (native only). Type name → closure that removes the component from the World.
    /// Only components registered in this map expose the "✕" (remove) button in the Inspector.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) component_removers: std::collections::HashMap<String, ComponentFactory>,
    /// Component name currently selected in the "Add Component" dropdown (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) add_component_selected: String,
    /// Whether gizmo drag grid snap is enabled (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) snap_enabled: bool,
    /// Gizmo drag grid snap cell size in pixels (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) snap_size: f32,

    // ── Resize handle state (native only) ────────────────────────────────────
    /// Which resize handle (if any) is currently being dragged.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) resize_handle_active: Option<ResizeHandle>,
    /// Cursor position (screen space) at the start of a resize drag.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) resize_drag_start_cursor: glam::Vec2,
    /// UiNode offset at the start of a resize drag.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) resize_drag_start_offset: glam::Vec2,
    /// UiNode size at the start of a resize drag.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) resize_drag_start_size: glam::Vec2,
    /// Transform.scale at the start of a world-sprite resize drag.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) resize_drag_start_scale: glam::Vec2,

    // ── Docked-mode fields (native only) ─────────────────────────────────────
    /// Current editor mode (Off / Overlay / Docked).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) mode: EditorMode,

    /// Whether the game simulation is paused while in Docked mode.
    ///
    /// When `true`, the system pipeline (scene systems) is skipped each frame.
    /// The builtin tail systems (e.g. `HierarchySystem`) continue to run so that
    /// gizmo-driven Transform changes propagate through the hierarchy while paused.
    /// Leaving Docked mode (F2) clears this flag automatically.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paused: bool,

    /// Advance exactly one full frame and then remain paused.
    ///
    /// Set by the ⏭ step button in the docked toolbar.  The pipeline runs a full
    /// frame this tick (all systems including scene systems), then `step_once` is
    /// cleared and `paused` stays true.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) step_once: bool,

    /// The central viewport rect expressed as `(x, y, width, height)` in logical
    /// points.  `None` until the first docked frame computes it.
    ///
    /// **Package 2 contract**: write this field from the real egui panel rects once
    /// side panels are laid out.  The docked scene render reads it every frame.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) central_rect: Option<egui::Rect>,

    /// egui `TextureId` of the current editor offscreen texture, if allocated.
    /// Must be freed (`egui_wgpu::Renderer::free_texture`) before reallocation.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) docked_texture_id: Option<egui::TextureId>,

    /// The most recent cursor position in window space (logical points), updated
    /// on every `CursorMoved` regardless of editor mode.
    ///
    /// `InputState::cursor()` holds the *translated* (viewport-local, frozen
    /// outside the panel) position while docked, so it cannot answer "is the
    /// pointer physically inside the central rect right now?" — this field can.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) window_cursor: Option<egui::Pos2>,

    /// Debounce state for the docked offscreen render-target size.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) rt_debounce: super::docked_rt::RtDebounce,

    // ── Data-table panel fields (native only) ─────────────────────────────────
    /// Active tab in the bottom panel: 0 = Assets, 1 = Data Tables.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) bottom_tab: u8,
    /// Name of the currently selected table in the Data Tables panel.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) selected_data_table: Option<String>,
    /// Status message shown below the data-table panel after save/reload.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) data_table_status: Option<String>,
    /// Path text field for opening a new data table in the panel.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) data_table_open_path: String,
    /// Name text field for opening a new data table in the panel.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) data_table_open_name: String,

    // ── Tile-paint fields (native only) ───────────────────────────────────────
    /// Whether viewport tile-painting is active for the selected `Tilemap` entity.
    /// While `true`, viewport clicks paint tiles instead of driving the move/resize gizmo.
    /// Auto-cleared when the selected entity is not a `Tilemap`.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paint_mode: bool,
    /// Tile value applied by a left-click paint. `0` = erase, `1..` = atlas index + 1.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paint_value: u32,
    /// Changed cells accumulated during the in-progress stroke: `(row, col, old, new)`.
    /// Committed to history as one `EditorCmd::PaintTiles` on mouse release.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paint_stroke: Vec<(usize, usize, u32, u32)>,
    /// Whether a press→release paint stroke is currently in progress.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paint_active: bool,
    /// `(atlas path, egui texture id)` currently registered with the egui renderer for
    /// the Tile Paint swatch palette. Re-registered when the selected tilemap's atlas
    /// changes and freed (`egui_wgpu::Renderer::free_texture`) when paint mode exits —
    /// see `App::register_paint_atlas_texture`. `None` until the atlas texture is cached.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paint_atlas_tex: Option<(String, egui::TextureId)>,
    /// Active Tile Paint tool (freehand / rectangle / bucket).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paint_tool: PaintTool,
    /// Freehand brush side length in cells — 1, 3, or 5 (an N×N block).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paint_brush: u32,
    /// Rectangle-tool drag anchor cell (set on press, cleared on release).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paint_anchor: Option<(usize, usize)>,
    /// Whether the in-progress stroke erases (right button) rather than paints.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paint_erase: bool,

    // ── Inspector QoL (native only) ───────────────────────────────────────────
    /// Single-component clipboard: `(type name, serialized value)` from a "copy component".
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) component_clipboard: Option<(String, ron::Value)>,
    /// Entity-list search query (case-insensitive substring of the entity label).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) entity_filter: String,
    /// Whether the world-aligned grid overlay is drawn on the docked viewport.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) show_grid: bool,
    /// Whether the debug bounds/colliders overlay is drawn (entity AABBs + collider shapes).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) show_bounds: bool,

    // ── Rotation gizmo (native only) ──────────────────────────────────────────
    /// Whether a rotation-handle drag is in progress.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) rotate_active: bool,
    /// Entity rotation (radians) captured at the start of a rotation drag (for undo).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) rotate_start_rotation: f32,
    /// Cursor angle around the entity centre captured at the start of a rotation drag.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) rotate_start_angle: f32,

    // ── Prefab create/instancing (native only) ────────────────────────────────
    /// File path used by the Save-as-Prefab / Spawn-Prefab controls.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) prefab_path: String,
    /// Status message from the last prefab save/spawn.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) prefab_status: Option<String>,
    /// Whether persisted editor settings have been loaded this process (load once on first open).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) settings_loaded: bool,
}

impl EditorState {
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn new() -> Self {
        Self {
            inspector_selected: None,
            gizmo_dragging: false,
            gizmo_drag_offset: glam::Vec2::ZERO,
            editor_save_status: None,
            inspector_tab: 0,
            selected_entities: Vec::new(),
            copy_clipboard: Vec::new(),
            editor_save_path: "saved_scene.ron".into(),
            editor_load_status: None,
            cmd_history: EditorHistory::new(),
            gizmo_drag_start_pos: None,
            gizmo_drag_start_positions: Vec::new(),
            component_factories: std::collections::HashMap::new(),
            component_removers: std::collections::HashMap::new(),
            add_component_selected: String::new(),
            snap_enabled: false,
            snap_size: 16.0,
            resize_handle_active: None,
            resize_drag_start_cursor: glam::Vec2::ZERO,
            resize_drag_start_offset: glam::Vec2::ZERO,
            resize_drag_start_size: glam::Vec2::ZERO,
            resize_drag_start_scale: glam::Vec2::ZERO,
            mode: EditorMode::Off,
            paused: false,
            step_once: false,
            central_rect: None,
            docked_texture_id: None,
            window_cursor: None,
            rt_debounce: super::docked_rt::RtDebounce::default(),
            bottom_tab: 0,
            selected_data_table: None,
            data_table_status: None,
            data_table_open_path: String::new(),
            data_table_open_name: String::new(),
            paint_mode: false,
            paint_value: 1,
            paint_stroke: Vec::new(),
            paint_active: false,
            paint_atlas_tex: None,
            paint_tool: PaintTool::Freehand,
            paint_brush: 1,
            paint_anchor: None,
            paint_erase: false,
            component_clipboard: None,
            entity_filter: String::new(),
            show_grid: false,
            show_bounds: false,
            rotate_active: false,
            rotate_start_rotation: 0.0,
            rotate_start_angle: 0.0,
            prefab_path: "prefab.ron".into(),
            prefab_status: None,
            settings_loaded: false,
        }
    }
}
