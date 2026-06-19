use super::*;

mod state;
mod ui;

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

pub(super) use state::EditorState;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use state::ResizeHandle;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use state::{apply_f1, apply_f2, EditorMode, InspectorPanel, PaintTool};

// Re-exports so the existing `ui/` and `state` call sites that reference these by the
// `crate::app::editor::…` / `super::super::…` paths keep resolving after the split.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) use history::{EditorCmd, EditorHistory};
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) use prefab::entity_to_def;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) use util::{entity_matches_filter, snap_to_grid};
