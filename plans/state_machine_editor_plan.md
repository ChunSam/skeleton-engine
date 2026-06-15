# Item 4 — State-machine graph editor (MVP, v8.26.0)

## Goal

Let a designer inspect and edit an entity's `AnimationStateMachine` from the docked editor: see states,
their transitions, and parameters; change the active state and clip indices; add/remove states and
transitions. First of two cycles for item 4 (this = list-based MVP; visual node-graph = follow-up).

## Why this design

- `AnimationStateMachine`'s `states`/`current`/`params` are **private with no read accessors**, so the
  editor couldn't inspect the machine at all. Step one is read accessors + edit operations on the
  component — a serializable-by-construction, **unit-testable core** — since autonomous *visual*
  validation is weak (the docked cursor-freeze blocks drag-editing a node graph). Tests drive the real
  data model; the panel is a thin layer over it.
- A list-based view (states → transitions, current highlighted) delivers the full *editing* capability
  robustly and is the standard "states + parameters" panel shape. A drawn node-graph (positioned boxes
  + edges) is deferred to the next cycle — it adds egui-painter geometry for marginal validatable value
  while drag-editing stays blocked by cursor-freeze.

## Scope (additive)

- `src/animation/state_machine.rs` — accessors `state_names`/`state`/`state_count`/`param_names`/`param`
  and edit ops `set_current_state`/`set_state_clip`/`remove_state`/`remove_transition`
  (`add_state`/`add_transition` already exist). `remove_state` prunes inbound transitions and refuses
  the active/last state.
- `src/app/editor/ui/docked.rs` — **State Machine** inspector section (gated on the component) +
  `state_machine_panel` (snapshot → render + collect edit intents → apply via `get_mut`, mirroring the
  audio/data-table panels) + `cond_summary`/`param_display` helpers.
- `EditorState.sm_add_state_name` field for the add-state text box.
- Version bump 8.25.0 → 8.26.0 (Cargo.toml, CLAUDE.md header + editor & state-machine rows, CHANGELOG).
- Example: none new — `sm_crossfade` already builds an `AnimationStateMachine` (F2 → inspect/edit it).

## Completion criteria

1. `cargo test --lib` green, +tests on the accessors + every edit op:
   `state_accessors_report_states_and_transitions`, `set_current_and_set_clip`,
   `remove_state_prunes_inbound_transitions`, `remove_state_refuses_last_state`,
   `remove_transition_by_index`, `param_accessors`.
2. Full Gate6 green (editor panel native-only; watch wasm cfg).
3. Panel renders for `AnimationStateMachine` entities and the edit buttons mutate the component.
4. `rust-survivors` unaffected (additive public methods; nothing removed).

## Out of scope (→ next cycle / item 4 iteration 2)

- Visual node-graph rendering (positioned state boxes + drawn transition edges).
- Adding/editing transitions + conditions from the UI (currently: add state, remove state/transition,
  set current, edit clip). Editing parameter values live.
- serde persistence of `AnimationStateMachine` in scene saves.
