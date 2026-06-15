# Plan — Rotation gizmo (v8.16.0)

> Editor feature B of the A–G loop. Additive, native-gated. Completes the world-sprite gizmo
> (move + 8-handle resize → + rotate). Validation = unit tests for the angle math + native F2 playtest.

## Goal
A **rotation handle** above the selected entity's top edge; dragging it rotates the entity
(`Transform.rotation`) to follow the cursor. Each rotation is one undoable `EditorCmd::RotateEntity`.
Snap-to-15° when the editor Snap toggle is on.

## Reuse / structure
- Gizmo draws via `DebugDraw::rect_filled_z` and inputs via `update_transform_gizmo_native`
  (gizmo.rs) with press/hold/release. Add rotation alongside the existing resize/move branches.
- `Transform { position, scale, rotation: f32 }` (radians).

## State (`state.rs`, native-gated)
- `rotate_active: bool`, `rotate_start_rotation: f32`, `rotate_start_angle: f32`.

## Pure helpers (`gizmo.rs`, unit-tested)
- `rotation_handle_pos(position, scale, gap) -> Vec2` — above the top edge (Y down): `(x, y - |scale.y|/2 - gap)`.
- `cursor_angle(center, cursor) -> f32` — `atan2(d.y, d.x)`.
- `snap_angle(angle, step) -> f32` — round to nearest multiple (`step<=0` → identity).
- `applied_rotation(start_rot, start_angle, cur_angle, snap: Option<f32>) -> f32` —
  `start_rot + (cur_angle - start_angle)`, optionally snapped.

## Handler (`update_transform_gizmo_native`)
- **Press**: hit-test the rotation handle first (it sits outside the AABB, no overlap with resize/body).
  If hit within `ROT_HIT_RADIUS`: `rotate_active = true`, `rotate_start_rotation = tr.rotation`,
  `rotate_start_angle = cursor_angle(tr.position, world_pos)`. Else fall through to resize/move.
- **Hold** (if `rotate_active`): `new = applied_rotation(start_rot, start_angle, cursor_angle(center, world_pos),
  snap_enabled.then_some(PI/12))`; set `Transform.rotation`.
- **Release** (if `rotate_active`): push `EditorCmd::RotateEntity { entity, old_rotation, new_rotation }`
  (if changed); clear `rotate_active`.
- Clear `rotate_active` in the `egui_wants_mouse` else-branch (like `resize_handle_active`).

## EditorCmd (`editor.rs`)
- `RotateEntity { entity, old_rotation: f32, new_rotation: f32 }` + undo (set `rotation = old`) /
  redo (set `rotation = new`) arms.

## Draw (`update_transform_gizmo`)
- After the 8 resize handles, draw the rotation handle as a small green `rect_filled_z` at
  `rotation_handle_pos(...)` (distinct colour so it reads as "rotate").

## Completion criteria
1. State + `EditorCmd::RotateEntity` (+ undo/redo) + rotation handle draw + press/hold/release wiring.
2. Unit tests: `cursor_angle` (cardinal directions), `rotation_handle_pos`, `snap_angle`,
   `applied_rotation` (delta + snap). Existing gizmo tests still pass.
3. Gate6 green; additive; native-only (wasm gizmo path unchanged = move-only).
4. Native F2 playtest: select an entity → drag the rotation handle → entity rotates; Snap → 15° steps;
   Ctrl+Z reverts. Screenshot.
5. v8.16.0; CHANGELOG + CLAUDE; merge.

## Out of scope
- Rotation-aware AABB / handle positions (handles stay axis-aligned to the un-rotated box for MVP).
- Per-entity rotation pivot; multi-select group rotation.
