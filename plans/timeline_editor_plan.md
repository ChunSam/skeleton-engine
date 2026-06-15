# Item 5 — Timeline editor (MVP, v8.27.0)

## Goal

Let a designer inspect and edit an entity's `Timeline` from the docked editor: playback controls plus
a per-track keyframe list (time / value / easing) with edits. The last item in the batch; list-based
MVP (visual track ruler = follow-up).

## Why this design

- `Track<T>`'s `keyframes` is **private with no read/remove/move accessors**, so the editor couldn't
  list or edit keyframes. Step one is generic accessors + edit ops on `Track<T>` — a unit-testable
  core — since autonomous *visual* validation is weak (cursor-freeze blocks dragging keyframe dots on
  a time ruler). Tests drive the real `Track`; the panel is a thin layer.
- `Timeline` is a component with six public tracks of different value types (`Vec2`/`f32`/`Color`). A
  generic `timeline_track_ui<T>(.., fmt)` renders each track with a type-appropriate value summary,
  reused across all six.

## Scope (additive)

- `src/timeline.rs` — `Track<T>` gains `keyframes()`, `len()`, `remove(index)`, `set_time(index, time)`
  (re-sorts), `clear()`. (`add`/`sample`/`duration`/`is_empty` already exist.)
- `src/app/editor/ui/docked.rs` — **Timeline** inspector section (gated on the component) +
  `timeline_panel` (playback: duration / loop / play-pause / restart / time scrub) +
  generic `timeline_track_ui<T>` (per-keyframe: editable time, value summary, easing, remove). Edits
  via one `get_mut::<Timeline>` (disjoint track fields edited sequentially).
- Version bump 8.26.0 → 8.27.0 (Cargo.toml, CLAUDE.md header + timeline & editor rows, CHANGELOG).
- Example: none new — `timeline_cutscene` already builds a `Timeline` (F2 → inspect/edit it).

## Completion criteria

1. `cargo test --lib` green, +tests on the new `Track` ops:
   `track_keyframes_accessor_is_sorted`, `track_remove_keyframe`, `track_set_time_resorts`, `track_clear`.
2. Full Gate6 green (editor panel native-only; watch wasm cfg).
3. Panel renders for `Timeline` entities; playback + keyframe edits mutate the component.
4. `rust-survivors` unaffected (additive `Track` methods; nothing removed).

## Out of scope (→ follow-up)

- Visual timeline (horizontal time ruler + draggable keyframe dots, playhead).
- Editing keyframe *values* (type-specific) and easing from the UI; adding keyframes from the UI.
- serde persistence of `Timeline` in scene saves.
