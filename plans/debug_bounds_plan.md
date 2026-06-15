# Plan — Debug bounds/colliders overlay (v8.19.0)

> Editor feature D-1 of the A–G loop (subsystem-editor category: debug visualization). Additive,
> native-gated. Universal (every entity has a Transform) + playtestable in `tile_paint` (painted tiles
> have Transforms). Validated by a unit test (DebugDraw shape count) + native F2 playtest.

## Goal
A toolbar **Bounds** toggle that draws, via `DebugDraw`, every entity's Transform AABB and any
collision `Collider` shape (Aabb/Circle) — a quick "see where everything is / what's collidable" view.

## Reuse
- `DebugDraw` (core resource, rendered by the engine): `rect(min,max,color)`, `circle(center,r,color)`.
- `world.query::<Transform>()`, `world.query2::<Transform, Collider>()`.
- `collision::Collider::{Aabb{half_extents}, Circle{radius}}` (+ `aabb(center)`).

## State (`state.rs`)
- `show_bounds: bool` (default false). Add to `EditorSettings` so it persists (feature F).

## Method (`App::draw_debug_bounds`, native)
- Collect `(pos, scale)` for all `Transform` entities and `(pos, Collider)` for collider entities
  (collect first to release the world borrow), then push to `DebugDraw`:
  - entity bounds: `rect(pos - scale/2, pos + scale/2, cyan@0.45)`.
  - collider: Aabb → `rect`, Circle → `circle`, in green@0.7.
- Called each frame from `update_editor_ui` when `editor.mode != Off && show_bounds`.

## UI
- Toolbar **Bounds** checkbox (next to Grid).

## Completion criteria
1. `show_bounds` state (+ persisted in `EditorSettings`); toolbar checkbox; `draw_debug_bounds` + per-frame call.
2. Unit test: World with N Transform entities + M colliders → `draw_debug_bounds` → assert `DebugDraw.shapes.len() == N + M`.
3. Gate6 green; additive; native-only.
4. Native F2 playtest in `tile_paint`: paint a few tiles → toggle **Bounds** → AABB outlines appear around the tiles. Screenshot.
5. v8.19.0; CHANGELOG + CLAUDE; merge.

## Out of scope (later D)
- Physics (rapier) collider/joint visualization; pathfinding grid overlay; audio mixer; lighting/particle
  editors; state-machine graph; timeline editor (the large D tail).
