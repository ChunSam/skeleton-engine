# Plan — Prefab create/instancing GUI (v8.17.0)

> Editor feature G of the A–G loop. Additive, native-gated. Button-driven (no drag-place yet) so the
> core round-trip is unit-testable through the real prefab code path (works around the viewport
> cursor-freeze playtest limit).

## Goal
From the docked editor: **Save the selected entity as a prefab** (RON file) and **Spawn a prefab**
from a path (instanced with a `PrefabInstance` marker → existing Break Prefab works).

## Reuse (verified)
- `Prefab { def: EntityDef }`; `Prefab::save(&Path)` / `load(&Path)` (plain RON); `spawn(&mut World)`;
  `spawn_with_tracking(&mut World, path)` (adds `PrefabInstance`). `spawn` internally runs the
  `SerdeComponentRegistry` dance, so serde components round-trip.
- `entity_to_def(world, e) -> Option<EntityDef>` captures tag/transform/sprite/parent + serde components.

## State (`state.rs`, native-gated)
- `prefab_path: String` (default `"prefab.ron"`), `prefab_status: Option<String>`.

## Logic (`App` methods, native, unit-tested)
- `save_selected_as_prefab(&mut self, sel, path: &str)` — `Prefab { def: entity_to_def(sel)? }.save(Path)`;
  set `prefab_status` to ok/err.
- `spawn_prefab(&mut self, path: &str)` — `Prefab::load(Path)` → `spawn_with_tracking(&mut world, path)`;
  select the new entity; set status.

## UI (`docked.rs`, inspector)
- A "Prefab" section: `prefab_path` text field; **💾 Save Selected as Prefab** (enabled when a
  non-UI entity is selected); **➕ Spawn Prefab**; status label.

## Completion criteria
1. State + `save_selected_as_prefab` / `spawn_prefab` + inspector UI.
2. Unit test: register a serde component, spawn an entity (tag+transform+serde comp), save as prefab to
   a temp file, spawn it back, assert the new entity has the same tag/transform/component value
   (round-trip through the real `Prefab` save/load/spawn path).
3. Gate6 green; additive; native-only.
4. v8.17.0; CHANGELOG + CLAUDE; merge. (GUI render smoke if display available.)

## Out of scope (later)
- Drag-place spawn palette, prefab thumbnails, a prefab browser, nested-prefab overrides.
