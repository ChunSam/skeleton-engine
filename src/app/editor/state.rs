use crate::ecs::Entity;

#[cfg(not(target_arch = "wasm32"))]
use super::EditorHistory;

#[cfg(not(target_arch = "wasm32"))]
use crate::app::ComponentFactory;

/// Default lifetime (seconds) of an editor toast before it fades out and is dropped.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) const DEFAULT_TOAST_TTL: f32 = 2.6;

/// Severity / colour of an editor [`Toast`] (transient action-feedback popup).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::app) enum ToastKind {
    /// Neutral notice (e.g. "Pasted 2").
    Info,
    /// A succeeded action (e.g. "Scene saved").
    Success,
    /// A failed action (e.g. "Save failed: …").
    Error,
}

/// A transient on-screen action-feedback popup (e.g. "Scene saved", "3 deleted") drawn bottom-right
/// and aged out after [`DEFAULT_TOAST_TTL`]. Pushed by editor actions via `App::push_editor_toast`,
/// aged + rendered each frame by `App::draw_editor_toasts`.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) struct Toast {
    pub(in crate::app) message: String,
    pub(in crate::app) kind: ToastKind,
    /// Seconds since the toast was pushed.
    pub(in crate::app) age: f32,
    /// Total lifetime in seconds (the last ~0.6 s fades out).
    pub(in crate::app) ttl: f32,
}

/// In-progress inline rename of an entity in the editor entity list.
///
/// Set when a list row is double-clicked (or [`App::editor_begin_rename`]): the editor renders a
/// focused text box bound to [`buffer`](EntityRename::buffer) in place of that row's label.
/// Committing (Enter or click-away) writes `Tag(buffer)` to the entity; Escape cancels.
///
/// Native-only — the docked/overlay entity list is excluded from wasm builds.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) struct EntityRename {
    /// The entity whose name is being edited.
    pub(in crate::app) entity: Entity,
    /// The editable name buffer, seeded from the entity's current `Tag` (empty if it has none).
    pub(in crate::app) buffer: String,
    /// `true` for the first frame after the rename begins, so the text box grabs keyboard focus
    /// exactly once (re-requesting focus every frame would trap it and block click-away commits).
    pub(in crate::app) focus_pending: bool,
}

/// A pluggable inspector sub-panel registered via [`App::register_inspector_panel`].
///
/// When the inspector draws the selected entity, each registered panel is evaluated:
/// `presence` is called to test whether the entity has the relevant component, and
/// `draw` is called only when it returns `true`.
///
/// Native-only — the docked editor is excluded from wasm builds.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) struct InspectorPanel {
    /// Returns `true` iff the entity has the component this panel targets.
    pub(in crate::app) presence: fn(&crate::ecs::World, Entity) -> bool,
    /// Title shown in the `CollapsingHeader` for this panel.
    pub(in crate::app) title: String,
    /// Draws the panel body. Called only when `presence` returns `true`.
    #[allow(clippy::type_complexity)]
    pub(in crate::app) draw: Box<dyn Fn(&mut egui::Ui, &mut crate::app::App, Entity)>,
}

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

/// How the docked editor's **Entities** list is ordered for display. Sorts a display-only copy —
/// the world's entity order and the scene-save order are unaffected. `Index` (the default) is the
/// order the caller hands in, which every call site builds by sorting `World::entities()` on
/// `Entity::index`.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::app) enum EntitySortMode {
    /// Ascending `Entity::index` — **not** spawn order, and no longer the raw `entity_list`.
    ///
    /// `World::entities()` is storage order and `despawn` swap_removes into the hole, so the raw
    /// list reshuffles when an unrelated entity is deleted; sorting on the index is what stops a
    /// hierarchy row from jumping. The tradeoff is that `World::spawn` recycles indices FIFO, so a
    /// newly created entity can land anywhere in the list rather than at the end. This order is
    /// stable, not chronological — nothing in the engine can reconstruct spawn order, which
    /// `World::entities()`'s own doc spells out.
    #[default]
    Index,
    /// Alphabetical by display label (case-insensitive), ties keep the incoming index order.
    Name,
    /// Grouped by entity kind (the same classification as the type-icon), then by name.
    Kind,
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

/// What a mode change has to do besides setting the mode, decided in one place and applied by
/// `App::set_editor_mode`, which the F1 / F2 keys and the toolbar's Exit button all call. The
/// toolbar used to spell the Docked→Off half by hand and left the settings save out, so
/// preferences were saved on the F2 *key* and not on the button labelled with it (v0.156.10).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) struct ModeTransition {
    /// Load the persisted settings: the first time Docked opens in this session.
    pub load_settings: bool,
    /// Persist the settings: on every exit from Docked.
    pub save_settings: bool,
    /// Drop pause and single-step: whenever the new mode is not Docked.
    pub resume: bool,
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn mode_transition(
    old: EditorMode,
    new: EditorMode,
    settings_loaded: bool,
) -> ModeTransition {
    let was_docked = old == EditorMode::Docked;
    let is_docked = new == EditorMode::Docked;
    ModeTransition {
        load_settings: is_docked && !was_docked && !settings_loaded,
        save_settings: was_docked && !is_docked,
        resume: !is_docked,
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
    /// The entity a gizmo gesture (move / resize / rotate, world or UI) started on. Every press
    /// that starts one sets it, and `App::commit_gesture` records against it — not against
    /// whatever is selected when the gesture ends. Those were the same value until Ctrl+Z was
    /// allowed to re-select mid-drag; the case where they differ is a gesture begun on one
    /// entity being applied to, and recorded under, another.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) gesture_entity: Option<Entity>,
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
    /// Written every docked frame from the real egui panel bounds (`ui/docked::draw`, the
    /// central panel's own response rect), so `None` means the first frames of a session and
    /// nothing else — `docked_rt::docked_viewport` stands in with fallback margins there, and
    /// both the scene render and the published `ViewportSize` read it through that one call.
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
    ///
    /// The cells carry no entity of their own — [`Self::paint_entity`] is what says whose they
    /// are, and the two must be cleared together.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paint_stroke: Vec<(usize, usize, u32, u32)>,
    /// The tilemap [`Self::paint_stroke`]'s cells were painted on, captured when the stroke
    /// started.
    ///
    /// A stroke belongs to one entity. Until v0.155.8 the commit attributed the whole batch to
    /// whatever happened to be selected at release time, and a stroke could outlive its selection
    /// — Ctrl+Z over a `CreateEntity` sets the selection to `None` with the button still held —
    /// so the next stroke on a *different* tilemap appended to the same buffer and one undo wrote
    /// the first map's old values into the second.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paint_entity: Option<crate::ecs::Entity>,
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
    /// The tool the in-progress stroke was started with. A stroke belongs to its tool the way
    /// it belongs to its entity ([`Self::paint_entity`]): the inspector's tool buttons can be
    /// clicked with the other mouse button still held, and a stroke that outlives its tool used
    /// to be dropped by Bucket's `clear()` and to swallow the next Freehand stroke with it.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) paint_stroke_tool: Option<PaintTool>,

    // ── Inspector QoL (native only) ───────────────────────────────────────────
    /// Single-component clipboard: `(type name, serialized value)` from a "copy component".
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) component_clipboard: Option<(String, ron::Value)>,
    /// Entity-list search query (case-insensitive substring of the entity label).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) entity_filter: String,
    /// How the Entities list is ordered for display (name / kind / raw insertion order).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) entity_sort: EntitySortMode,
    /// In-progress inline entity rename (double-click a list row to edit its name). `None` when
    /// not renaming. See [`EntityRename`] + `App::editor_begin_rename`/`editor_commit_rename`.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) entity_rename: Option<EntityRename>,
    /// Whether the world-aligned grid overlay is drawn on the docked viewport.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) show_grid: bool,
    /// Whether the debug bounds/colliders overlay is drawn (entity AABBs + collider shapes).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) show_bounds: bool,
    /// Whether the pathfinding-grid overlay is drawn (per-`Tilemap` walkable/blocked cells).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) show_pathgrid: bool,
    /// Whether the keyboard-shortcuts cheatsheet window is shown (toggled by `?` or the toolbar
    /// `? Keys` button). Discoverability aid — lists every editor shortcut.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) show_shortcuts: bool,
    /// Active action-feedback toasts (transient popups; aged + drawn bottom-right each frame).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) toasts: Vec<Toast>,
    /// Editor UI language (English / Korean). Persisted in `editor_settings.ron`.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) locale: super::EditorLocale,

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

    /// Name typed into the State Machine panel's "add state" box.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) sm_add_state_name: String,
    /// Per-state add-transition target (state name) typed in the SM panel. Keyed by source state.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) sm_add_trans_target: std::collections::HashMap<String, String>,
    /// Per-state add-transition crossfade duration input in the SM panel. Keyed by source state.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) sm_add_trans_xf: std::collections::HashMap<String, f32>,

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
    /// Where the settings file lives. `None` is the real config dir; a test points it at a
    /// temp path so a mode switch can be asserted on disk without touching the user's file.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) settings_path_override: Option<std::path::PathBuf>,

    // ── Pluggable inspector panels (native only) ──────────────────────────────
    /// Sub-panels registered via [`App::register_inspector_panel`].
    /// Iterated after the hardcoded panels; each fires only when its `presence` fn
    /// returns `true` for the selected entity.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) inspector_panels: Vec<InspectorPanel>,
}

impl EditorState {
    /// Abandon every in-progress gizmo gesture.
    ///
    /// Three sites drop a drag — the selection going away, the `UiNode` branch, and egui taking
    /// the pointer — and each used to clear its own subset of the flags. The shortest of them
    /// cleared **one of three**, so a drag interrupted by a selection change (`EditorCmd::
    /// CreateEntity` undo sets the selection to `None` while the button is still held) left
    /// `rotate_active` / `resize_handle_active` set with no path to the release handler that
    /// would clear them. `update_transform_gizmo_native`'s press guard requires all three clear,
    /// so the gizmo then refused every later gesture until egui happened to take the pointer.
    ///
    /// One method so the policy lives in one place rather than being re-remembered per site.
    ///
    /// ⚠️ Clearing is only half of ending a gesture. A move, resize or rotation that was
    /// abandoned has already been applied to the entity, so dropping the flags alone leaves
    /// that change with no undo entry. `App::abandon_gesture` records it first — through the
    /// same `commit_gesture` the release handler uses — and then calls this; the abandon sites
    /// call *that*. Call this directly only where nothing can have been applied.
    ///
    /// ⚠️ A tile-paint stroke is abandoned the same way and is **not** handled here, because
    /// ending one is not a matter of clearing flags: the cells are already on the map, so the
    /// batch has to be committed against the entity it was painted on
    /// ([`Self::paint_entity`]) or the paint becomes un-undoable. `App::finish_paint_stroke`
    /// owns that, and the abandon sites call both.
    pub(in crate::app) fn clear_drag_state(&mut self) {
        self.gizmo_dragging = false;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.resize_handle_active = None;
            self.rotate_active = false;
            self.gizmo_drag_start_pos = None;
            self.gizmo_drag_start_positions.clear();
            self.gesture_entity = None;
        }
    }

    /// Forgets every edit that is mid-gesture — a paint stroke, an inline rename, a gizmo drag —
    /// **without recording any of it**. For a world reset only. The entities those edits refer to
    /// are gone, and the counters restart, so a handle kept across the reset aliases whatever the
    /// new scene spawns first: a stroke that survived the reset was committed the next frame as a
    /// `PaintTiles` against the old tilemap's handle, onto the fresh history the reset had just
    /// emptied, and one Ctrl+Z then wrote the old scene's cell values into the new scene's map
    /// (v0.156.9). The same handle problem applies to a rename buffer and a gizmo gesture.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn drop_in_flight_edits(&mut self) {
        self.paint_active = false;
        self.paint_entity = None;
        self.paint_stroke.clear();
        self.paint_anchor = None;
        self.paint_stroke_tool = None;
        self.entity_rename = None;
        self.clear_drag_state();
    }

    /// Whether any gizmo gesture is in progress — the three flags the press guard checks.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn gesture_active(&self) -> bool {
        self.gizmo_dragging || self.rotate_active || self.resize_handle_active.is_some()
    }

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
            gesture_entity: None,
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
            paint_entity: None,
            paint_active: false,
            paint_atlas_tex: None,
            paint_tool: PaintTool::Freehand,
            paint_brush: 1,
            paint_anchor: None,
            paint_erase: false,
            paint_stroke_tool: None,
            component_clipboard: None,
            entity_filter: String::new(),
            entity_sort: EntitySortMode::Index,
            entity_rename: None,
            show_grid: false,
            show_bounds: false,
            show_pathgrid: false,
            show_shortcuts: false,
            toasts: Vec::new(),
            locale: super::EditorLocale::default(),
            rotate_active: false,
            rotate_start_rotation: 0.0,
            rotate_start_angle: 0.0,
            sm_add_state_name: String::new(),
            sm_add_trans_target: std::collections::HashMap::new(),
            sm_add_trans_xf: std::collections::HashMap::new(),
            prefab_path: "prefab.ron".into(),
            prefab_status: None,
            settings_loaded: false,
            settings_path_override: None,
            inspector_panels: Vec::new(),
        }
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    // ── helpers ──────────────────────────────────────────────────────────────
    // Minimal stand-ins for the `presence` half of the registry. The dispatch itself is
    // tested for real — with an `App` and an egui frame — beside the code that does it,
    // in `ui/docked/inspector_tab.rs`.

    /// A minimal `presence` fn factory — wraps `World::has_component::<T>`.
    fn make_presence<T: 'static>() -> fn(&crate::ecs::World, crate::ecs::Entity) -> bool {
        |world, entity| world.has_component::<T>(entity)
    }

    // ── test components ───────────────────────────────────────────────────────
    struct MarkerA;
    struct MarkerB;

    // ── tests ─────────────────────────────────────────────────────────────────

    /// `register_inspector_panel` pushes exactly one entry per call.
    #[test]
    fn register_panel_pushes_one_entry() {
        let mut state = super::EditorState::new();
        assert_eq!(state.inspector_panels.len(), 0);

        state.inspector_panels.push(super::InspectorPanel {
            presence: make_presence::<MarkerA>(),
            title: "Marker A".into(),
            draw: Box::new(|_ui, _app, _e| {}),
        });
        assert_eq!(state.inspector_panels.len(), 1);
        assert_eq!(state.inspector_panels[0].title, "Marker A");
    }

    /// The stored `presence` fn returns `true` only for entities that actually
    /// carry the target component.
    #[test]
    fn presence_fn_correct_for_component() {
        let mut world = crate::ecs::World::new();
        let with_marker = world.spawn();
        world.add_component(with_marker, MarkerA);
        let without_marker = world.spawn();

        let presence = make_presence::<MarkerA>();

        assert!(presence(&world, with_marker), "should detect MarkerA");
        assert!(
            !presence(&world, without_marker),
            "entity without MarkerA should return false"
        );
    }

    /// Presence fn for `MarkerB` does not fire for an entity that only has
    /// `MarkerA` — types are distinguished correctly.
    #[test]
    fn presence_fn_distinct_types() {
        let mut world = crate::ecs::World::new();
        let e = world.spawn();
        world.add_component(e, MarkerA);

        let presence_a = make_presence::<MarkerA>();
        let presence_b = make_presence::<MarkerB>();

        assert!(presence_a(&world, e));
        assert!(!presence_b(&world, e));
    }
}
