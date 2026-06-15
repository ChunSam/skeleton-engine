# D-3 — Editor audio bus mixer panel (v8.21.0)

## Goal

Add an **Audio** tab to the editor's bottom panel that lists every audio bus with a live volume
slider, so a designer can balance music/sfx/voice mixes while the game runs in the editor.

## Why this design / prerequisite

- The editor cannot enumerate buses today: `AudioManager` exposes `set_bus_volume`/`bus_volume`
  but its `bus_volumes` / `channel_buses` maps are private. **Prerequisite:** add
  `AudioManager::bus_names()` returning the sorted, deduplicated union of assigned buses and
  volume-only buses. Backed by a pure free fn `collect_bus_names` so the merge/sort/dedup logic is
  unit-testable headlessly (the device-backed `AudioManager::new()` returns `None` on CI).
- `AudioManager` is a World resource games insert (`app.world.insert_resource(AudioManager::new()…)`),
  so the panel reads it from `app.world`. It may be absent (audio uninitialized) → show a hint.

## Scope (additive, native-only)

- `AudioManager::bus_names(&self) -> Vec<String>` (public, native+wasm — pure) + private
  `collect_bus_names`.
- `src/app/editor/ui/audio_panel.rs` — `audio_mixer_panel_body(ui, app)`: snapshot
  `(name, bus_volume)` under an immutable borrow, render one `Slider` (0..=1) per bus, apply changed
  values under a fresh `resource_mut` borrow (collect-then-apply, mirrors the data-table panel).
- Wire module in `ui/mod.rs`; add bottom-panel tab `2 = Audio` in `docked.rs` (Assets | Data Tables | Audio).
- Version bump 8.20.0 → 8.21.0 (Cargo.toml, CLAUDE.md header + module-map rows, CHANGELOG).

## Completion criteria

1. `cargo test --lib` green, +tests:
   - `collect_bus_names_merges_sorts_and_dedups` (always runs, pure).
   - `collect_bus_names_empty_is_empty`.
   - `bus_names_round_trips_through_audio_manager` (device-guarded, skips headless).
2. Full Gate6 green.
3. Panel degrades gracefully: no `AudioManager` → hint label; buses empty → hint label.
4. `rust-survivors` unaffected (purely additive; new public method, nothing removed).

## Out of scope

- Per-channel volume / pan / mute-solo controls, metering. Bus-level volume is the MVP.
- Persisting bus volumes (they live in the game's `AudioManager`, not editor settings).
