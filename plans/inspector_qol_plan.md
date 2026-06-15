# Plan — Inspector QoL: component copy/paste + entity search (v8.14.0)

> Editor feature C of the A–G loop. Additive, native-gated, **display-independent** (validated by
> unit tests — the Mac display is asleep, so no GUI smoke this run). Merge authorized by the loop.

## Goal
Two inspector quality-of-life features:
1. **Component copy/paste** — copy one serde-registered component off the selected entity and paste it
   onto another entity (e.g. copy a tuned `Stats`/`Sprite` and apply it elsewhere).
2. **Entity-list search/filter** — a text box that filters the left entity list by label substring.

## Reuse (verified)
- `SerdeComponentRegistry::serialize_entity(world, e) -> HashMap<String, ron::Value>` (prefab.rs:220).
- `SerdeComponentRegistry::deserialize_into(&self, &mut World, e, &map)` (prefab.rs:232) — apply via the
  `remove_resource → deserialize_into → insert_resource` dance (prefab.rs:456-458; registry can't be
  borrowed while `&mut World` is needed).
- Component list UI lives in `inspector_tab_body` (docked.rs); entity list in `entities_tab_body`.
- Consistent with existing "+ Add" / "✕" remove, component paste is **not** pushed to undo history
  (those aren't either) — documented, not a regression.

## State (`state.rs`, native-gated)
- `component_clipboard: Option<(String, ron::Value)>` — (type name, serialized value).
- `entity_filter: String` — entity-list search query.

## Logic (`App` methods, native, unit-tested)
- `copy_component(&mut self, sel, type_name: &str)` — `serialize_entity(sel).remove(type_name)` →
  store `Some((type_name, value))`.
- `paste_component(&mut self, sel)` — if clipboard `Some((name, val))`: build a one-entry map, run the
  registry dance to `deserialize_into(sel, &map)`.
- `fn entity_matches_filter(label: &str, filter: &str) -> bool` (pure) — case-insensitive substring;
  empty filter matches all.

## UI
- `inspector_tab_body` component list: compute `serialize_entity(sel)` once; for each component name
  present in it, add a small **copy** button (⧉) next to the existing remove button. Below the list,
  when the clipboard holds a component, a **"Paste {type}"** button applies it to `sel`.
- `entities_tab_body`: a `TextEdit::singleline` bound to `entity_filter` at the top; skip rendering
  entities whose label fails `entity_matches_filter`.

## Completion criteria
1. State fields + init.
2. `copy_component` / `paste_component` / `entity_matches_filter` implemented.
3. Unit tests: (a) `entity_matches_filter` (empty/case/substring/no-match); (b) component copy/paste
   round-trip via the real registry (register a serde component, copy from A, paste onto B, assert B's
   value matches) — reuses the `register_serde_component` flow.
4. UI: copy buttons per serde component, paste button, entity search box.
5. Gate6 green; additive; native-only.
6. v8.14.0; CHANGELOG + CLAUDE; merge.

## Out of scope / notes
- Component paste is not undoable (matches add/remove component). Could add an `EditorCmd` later.
- Multi-component clipboard; cross-session clipboard persistence.
- GUI smoke deferred (display asleep) — the UI reuses proven egui patterns (buttons, `TextEdit`).
