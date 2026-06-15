# D-5 — Editor lighting editor (v8.23.0)

## Goal

Let a designer place and tune 2D lights from the editor: add/edit `PointLight`s per entity and edit
the global `AmbientLight`, all live (the lighting pass reads them each frame).

## Why this design

- `PointLight` is a plain cross-platform component (only `LightingRenderer` is native-only) with
  `color` / `radius` / `intensity` / `light_height`. Games drive it via `world.get_mut::<PointLight>`
  (see `lit_dungeon`). The editor mirrors that: a **Point Light** inspector section with per-field
  drags (same proven pattern as Tile Paint / Particle Tuner).
- True viewport click-to-place is unreliable under the docked cursor-freeze, so placement is done by
  **selecting an entity and adding a `PointLight`** — the entity's `Transform` position is the light
  position. To make that one click, `PointLight` is registered as an editor component (factory +
  remover) so it appears in Add/Remove and the "+ Add" dropdown.
- `AmbientLight` is a global resource (games may never insert it; `App::new` doesn't). The Ambient
  section calls `App::ensure_ambient_light` to guarantee one exists before editing.

## Scope (additive, native-only editor surface)

- `register_default_components`: add `PointLight` factory + remover.
- `App::reset_point_light(sel)` and `App::ensure_ambient_light() -> bool` (testable cores).
- `docked.rs`: `point_light_grid`, `ambient_light_control`, `color_rgb_drags` helpers; a **Point
  Light** inspector section (gated on `PointLight` present) with Reset button; an **Ambient Light**
  collapsing section at the top of the inspector scroll area.
- Version bump 8.22.0 → 8.23.0 (Cargo.toml, CLAUDE.md header + editor row, CHANGELOG).

## Completion criteria

1. `cargo test --lib` green, +tests:
   - `reset_point_light_restores_defaults`.
   - `ensure_ambient_light_inserts_default_once` (inserts once, second call no-op).
   - `pointlight_registered_as_editor_component` (factory + remover registered; factory adds it).
2. Full Gate6 green.
3. Sections appear correctly (Point Light only for `PointLight` entities; Ambient always).
4. `rust-survivors` unaffected (additive; editor-internal methods + a new factory registration).

## Out of scope

- Click-to-place lights in the viewport (cursor-freeze ceiling — done via add-component instead).
- Per-light gizmo handles for radius. Drag editors cover tuning.
