use super::*;

mod state;
mod ui;

// Editor localization. Cross-platform: `tr` is called from both the native docked editor and the
// shared wasm overlay path, so the module is NOT wasm-gated. The active locale is a thread-local
// set each frame from `EditorState::locale` (native) — which `EditorSettings` fills in only on the
// first Docked open, so an overlay-only session keeps the default; wasm has no locale toggle at
// all and stays at the default. See `i18n`'s module doc.
mod i18n;

#[cfg(not(target_arch = "wasm32"))]
mod history;
#[cfg(not(target_arch = "wasm32"))]
mod overlays;
#[cfg(not(target_arch = "wasm32"))]
mod prefab;
#[cfg(not(target_arch = "wasm32"))]
mod settings;
#[cfg(not(target_arch = "wasm32"))]
mod util;

// Cross-platform: `GIZMO_SELECT_COLOR` is used by `update_ui_node_gizmo`, which compiles (dead)
// on wasm; the rest of the editor theme constants are native-only and gated inside the module.
mod theme;

// Cross-platform: both `component_registry` and `loading` carry methods that MUST compile on
// wasm too (`register_editable_component`/`register_serde_component`/`load_*`). The module files
// are un-gated; their native-only contents (the `register_component`/`register_default_components`
// block, `sync_tilemap_colliders`) carry their own `#[cfg(not(target_arch = "wasm32"))]` inside.
mod component_registry;
mod loading;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub(super) mod docked_rt;

// `tr` is used on both native + the wasm overlay path; `set_locale` / `EditorLocale` are only used
// by the native editor (toolbar toggle + persisted setting), so re-export them native-only.
pub(in crate::app) use i18n::tr;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) use i18n::{set_locale, EditorLocale};

pub(super) use state::EditorState;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use state::ResizeHandle;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use state::{apply_f1, apply_f2, EditorMode, EntitySortMode, InspectorPanel, PaintTool};

// Re-exports so the existing `ui/` and `state` call sites that reference these by the
// `crate::app::editor::…` / `super::super::…` paths keep resolving after the split.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) use history::{EditorCmd, EditorHistory};
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) use prefab::entity_to_def;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) use util::{entity_matches_filter, snap_to_grid};
