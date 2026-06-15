# Plan — Editor settings persistence (v8.18.0)

> Editor feature F of the A–G loop (workflow QoL). Additive, native-gated, display-independent
> (validated by a unit-test round-trip). Editor prefs survive a restart.

## Goal
Persist the docked editor's preferences (snap on/off + size, grid overlay, paint tool + brush) to a
RON config file: **load** them the first time the editor opens, **save** them when it closes.

## Reuse
- `save::save_path(app, file)` (config-dir path), `save::write_ron` / `read_ron` / `exists`.
- F2 toggle in `window.rs` (`apply_f2`) is the open/close hook point.

## Struct (`editor.rs`, native-gated)
- `EditorSettings { snap_enabled: bool, snap_size: f32, show_grid: bool, paint_brush: u32, paint_tool: PaintTool }`
  with serde derives. Add `Serialize`/`Deserialize` to `PaintTool`.
- `from_state(&EditorState) -> Self`, `apply_to(&self, &mut EditorState)`.

## State (`state.rs`)
- `settings_loaded: bool` (load once per process).

## App methods (`editor.rs`, native)
- `save_editor_settings(&self)` → `write_ron(save_path("skeleton-engine","editor_settings.ron"), from_state)`.
- `load_editor_settings(&mut self)` → if `exists`, `read_ron` + `apply_to`.

## Hook (`window.rs`, F2 handler)
- Capture `old_mode` before applying. After setting `self.editor.mode = new_mode`:
  - entering Docked the first time (`is_docked && !was_docked && !settings_loaded`) → `load_editor_settings()`, set flag.
  - leaving Docked (`was_docked && !is_docked`) → `save_editor_settings()`.

## UI
- Toolbar **💾 Set.** button → `save_editor_settings()` (explicit save without closing).

## Completion criteria
1. `EditorSettings` (+ serde on `PaintTool`) + `from_state`/`apply_to`; `settings_loaded`; save/load methods; window.rs hook; toolbar button.
2. Unit test: build a state with non-default prefs → `from_state` → `write_ron`/`read_ron` round-trip → `apply_to` a fresh state → assert prefs match (and a default-on-missing-file path is safe).
3. Gate6 green; additive; native-only.
4. v8.18.0; CHANGELOG + CLAUDE; merge.

## Out of scope (later)
- Autosave of the scene; window/panel layout persistence; per-project settings; shortcuts/command palette.
