# Changelog

All notable changes to `skeleton-engine` are documented here.

The package follows semantic versioning. It is currently **pre-1.0 (0.x)**: MINOR covers any release (including breaking changes), PATCH is a bugfix/point release; 1.0.0 will mark a deliberate compatibility commitment.

## 0.43.0

**Hexagonal autotiling now ships a real 64-tile "blob" atlas — the hex analogue of the square `blob_47`.** The `hex_6`/`hex_6_flat` constructors have existed since 0.39.0, but no 64-tile atlas backed them (the `hex_autotile` examples used a 2-tile interior/edge rule). This release adds the missing assets and full-blob examples so every open hex edge is outlined correctly, not just "interior vs edge". **No public API change** — assets + examples only.

### Added
- **`gen_hex_autotile_sheet`** (`examples/gen_hex_autotile_sheet.rs`): a deterministic, manual asset generator (the hex sibling of `gen_autotile_sheet`) that procedurally draws `examples/assets/hex_autotile.png` (pointy-top, 8×8 of 64×74 cells) and `examples/assets/hex_autotile_flat.png` (flat-top, 8×8 of 74×64 cells). Each cell renders a regular hexagon (vertices fill the cell, matching `Tilemap::cell_render_size`); an edge whose neighbor bit is CLEAR is outlined, a connected side blends to the boundary so filled hexes tessellate. **Tile index == the 6-bit `Hex6`/`Hex6Flat` neighbor mask**, lining up with `TilemapAutotile::hex_6(base)` / `hex_6_flat(base)`'s identity mask→tile map.
- **`hex_blob_autotile`** + **`hex_blob_autotile_flat`** examples: full-blob dig/fill demos (`TilemapProjection::Hexagonal` + `hex_6`, and `HexagonalFlat` + `hex_6_flat`) over the new atlases. A solid field with carved holes auto-outlines its rim and every hole, recomputed reactively by `TilemapSystem`. The VISION acceptance test for the 64-tile hex blob; both orientations verified on-screen.

## 0.42.0

**The UI focus ring is now restyleable via the `FocusRingStyle` resource.** The focus pass's
previously hardcoded `RING_COLOR`/`RING_THICKNESS` constants move to a `FocusRingStyle` World
resource (`color` / `thickness` / `enabled`), auto-inserted with the default amber 3px ring so
existing behavior is byte-identical. Insert your own to recolor/resize it, or set `enabled = false`
(or `thickness <= 0.0`) to suppress the engine ring entirely and draw your own focus indicator.
Additive — default behavior is unchanged.

### Added
- **`FocusRingStyle`** (`src/ui/focus.rs`): a `Copy` World resource for the focus-ring appearance
  (`color: Color`, `thickness: f32`, `enabled: bool`) plus `is_visible()`. `Default` reproduces the
  historical hardcoded ring exactly (amber `rgba(1.0, 0.85, 0.3, 1.0)`, 3px). Auto-inserted in
  `insert_core_resources` next to `UiFocus`; re-exported at the crate root.
- **`focus_pass` reads the resource** (`src/ui/system/focus_pass.rs`): `push_ring` is styled by the
  resolved `FocusRingStyle` (falling back to the default when the resource is absent) and draws
  nothing when the style is not visible. The hardcoded `RING_COLOR`/`RING_THICKNESS` constants are
  removed.
- 5 tests (3 `push_ring` unit tests for custom color/thickness, disabled/zero-thickness, and the
  default-appearance contract; 2 `UiSystem` integration tests confirming the resource is consumed
  end-to-end into the `UiQueue` and that a disabled style draws no ring); lib tests 878 → 883.
  Example `ui_focus` restyles the ring to a thicker cyan one to demonstrate.

## 0.41.0

**Left analog stick now drives UI focus navigation, alongside the existing D-pad.** The
`UiSystem` focus pass folds the first connected pad's left stick into its per-frame input snapshot:
push **Up/Down** to cycle focus across widgets, **Left/Right** to nudge a focused `Slider`. The
stick is edge-detected (one push = one focus step, no auto-repeat) so it behaves like the D-pad
rather than spraying steps while held. Additive — keyboard and D-pad navigation are unchanged, and
**no public API change** (the new `StickNav` edge detector is `pub(super)`).

> **Hardware verification deferred.** The stick logic is covered by 8 new tests (4 `StickNav` unit +
> 4 focus-pass integration) and its axis signs match the engine's existing `AxisBinding` convention
> (see the `survivor` example: up = −Y, down = +Y, right = +X). Real-pad confirmation is pending: on
> macOS, gilrs (IOKit HID) enumerates a Bluetooth/GameController-claimed Xbox controller — so it
> connects — but the OS routes its input through Apple's GameController framework, so gilrs receives
> no button/axis events. This is an environment limitation, not a defect (the existing `survivor`
> gamepad support hits the same wall there); it will be revisited during per-OS input optimization.

### Added
- **Left analog stick → UI focus nav** (`src/ui/system/state.rs`): new `StickNav` per-axis edge
  detector with hysteresis (0.6 activate / 0.35 release) converts the continuous left stick into
  discrete D-pad-style steps. `InputSnapshot::from_world` now takes `&mut StickNav` and folds the
  stick in next to the D-pad; `UiSystem` holds the `StickNav` across frames alongside its scratch
  buffers. Left-stick Up/Down cycle focus (Up = reverse, like Shift+Tab), Left/Right nudge a focused
  `Slider`.
- **`GamepadState::test_axis`** (`src/input/gamepad.rs`, `#[cfg(test)]`): mirrors `test_press` for
  analog input, letting non-`gilrs` tests drive the stick.
- 8 tests (`StickNav` hysteresis unit tests + focus-pass integration tests for advance/reverse/wrap,
  held-no-repeat, and slider nudge); lib tests 870 → 878. Example `ui_focus` updated to advertise the
  left stick.

## 0.40.2

**Behavior-preserving dedup of the two hex autotile bitmask functions.** Internal refactor only —
**no public API change** and no behavior change; the 31 autotile tests (incl. the parity-dependent
hex offset and all-six-neighbor cases) and the full verify gate confirm parity.

### Changed (internal)
- **Hex autotile mask dedup** (`src/tilemap/autotile.rs`): `hex6_mask` (pointy-top, odd-r) and
  `hex6_flat_mask` (flat-top, odd-q) each open-coded the same six `if filled(..) { mask |= bit }`
  accumulation. Both now build a `[(drow, dcol); 6]` offset table (in ascending bit order) and
  delegate to a shared `hex_mask_from_offsets()` accumulator; each layout's distinct bit order and
  parity-dependent offsets stay explicit in its table.

## 0.40.1

**Behavior-preserving cleanup of two deferred code-review items (audio / UI focus).** Internal
refactors only — **no public API change** and no behavior change; the verify gate (870 lib tests) and
the wasm audio smoke (38/38) confirm parity.

### Changed (internal)
- **wasm `WebAudio` positional dedup** (`src/audio_wasm.rs`): `play_at` and `play_at_on_bus` shared an
  identical `update_position`-then-return tail over differently-routed SFX. Both now delegate to a
  shared private `play_at_to(dest)` helper (master vs. bus gain is the only difference), mirroring the
  existing `play_sfx`/`play_sfx_on_bus` → `play_sfx_to` structure.
- **`focus_pass` membership cost** (`src/ui/system/focus_pass.rs`): the per-frame focus sync tested
  membership against the index-sorted focusables list with two linear `contains` scans and cloned the
  scratch vector each frame. Both `contains` calls now use an `is_focusable()` binary search (`O(log n)`)
  and the redundant `focusables_snapshot` clone is gone.

## 0.40.0

**Code-review hardening of the 2026-06-18 feature arc (audio / dialogue / UI focus / save) +
cleanups.** A multi-angle review of the v0.32→v0.39 work surfaced several real bugs (mostly
edge-case races and conditional-path footguns); this release fixes them. The one breaking change is
`TilemapProjection` becoming `#[non_exhaustive]` (external exhaustive `match`es must add a wildcard
arm) — a MINOR under the 0.x cadence. wasm audio + save smokes pass (38/38, 7/7); 870 lib tests.

### Fixed
- **wasm `WebAudio` races** (`src/audio_wasm.rs`): `Sfx::stop()` called before the async decode
  finished was a no-op (the sound played anyway) — now a shared `stopped` flag suppresses the
  deferred `start()`. Rapid `crossfade_music`/`play_music` calls within the decode window orphaned a
  looping track that could never be stopped — a `music_gen` generation guard makes a superseded
  pending track stop itself. `start_music` connected the per-track gain to master *before* decoding,
  leaking a dead node on decode failure — the gain is now created/connected only after decode
  succeeds.
- **Dialogue conditional-choice deadlock** (`src/dialogue/mod.rs`): the vars-unaware
  `advance`/`choose`/`pending_choices` ignored choice conditions, so a line whose choices were all
  `cond`-gated blocked `advance()` forever (and `choose(i)` could pick a hidden choice). The plain
  API now considers only *unconditional* choices (the vars-aware `advance_with`/`dialogue::*` path is
  unchanged); no-condition dialogues are byte-identical.
- **Dialogue typewriter on bad data**: a non-finite `chars_per_sec` (e.g. `NaN` from malformed RON)
  rendered the line blank — the reveal guards now treat non-finite as "reveal instantly".
- **UI focus split-authority** (`src/ui/system/`): `focus_pass` and `text_input_pass` both wrote
  `ti.focused` with conflicting click semantics, so clicking outside a focused `TextInput` fired a
  spurious `TextBlurred` + dropped a frame of input. `focus_pass` is now the single owner of
  `ti.focused` and the `TextFocused`/`TextBlurred` events; `Enter`-to-submit clears `UiFocus` so the
  field isn't re-focused next frame.
- **wasm save parity** (`src/save.rs`): `read_ron` on wasm lacked native's `SAVE_MAGIC` fallback, so
  reading an AEAD-saved key returned a confusing parse error instead of decrypting — the wasm branch
  now mirrors native (hex-decode → magic check → decrypt, else plaintext RON).

### Changed
- **`TilemapProjection` is now `#[non_exhaustive]`** (`src/tilemap/mod.rs`) — matches the engine's
  other growable enums (`DebugShape`, `ReflectValue`, `Easing`); external exhaustive matches must add
  a `_ =>` arm. Breaking, hence MINOR.

### Added
- **`examples/hex_autotile_flat.rs`** — the flat-top (`HexagonalFlat` + `Neighborhood::Hex6Flat`)
  counterpart of `hex_autotile`, closing the VISION "an example exercises it" gap for flat-top hex
  autotiling.

### Changed (internal)
- `spatial_params` (linear distance falloff + x-pan) deduplicated into a cross-platform
  `src/audio_spatial.rs` (`pub(crate)`), shared by native `AudioManager` and wasm `WebAudio` (was a
  byte-for-byte copy in each). Autotile's duplicated bounds-check closure and the `hex_6`/`hex_6_flat`
  constructors are unified via private helpers. No behavior change.

## 0.39.0

**Autotiling across isometric and hexagonal projections.** Autotile bitmasks are computed from the
`tiles[row][col]` grid topology, so they already worked on **isometric** maps unchanged (iso is the
same square grid as orthographic, just rendered as diamonds) — now confirmed + tested. For **hex**
maps, two new neighborhoods compute the correct 6 parity-aware neighbors: `Neighborhood::Hex6`
(pointy-top, odd-r) and `Hex6Flat` (flat-top, odd-q). **Additive** — `Edge4`/`Blob8` unchanged.

### Added
- `Neighborhood::Hex6` (bits E=1, W=2, NE=4, NW=8, SE=16, SW=32) and `Neighborhood::Hex6Flat`
  (N=1, S=2, NE=4, SE=8, NW=16, SW=32) — 6-neighbor hex masks (`0..64`); the four diagonal offsets
  shift with row parity (odd-r) / column parity (odd-q) to match the staggered hex layout.
- `TilemapAutotile::hex_6(base)` / `hex_6_flat(base)` — 64-tile single-terrain hex autotile layouts
  (`mask → base + mask`), the hex analogue of `edge_16`/`blob_47`.
- Example `hex_autotile` — a pointy-top hex map with an interior-vs-edge `Hex6` rule (grass interior /
  sand open-edge) over the existing 2-tile hex atlas; dig holes and the rim re-tiles reactively.
- Unit tests: Hex6 / Hex6Flat interior + parity-dependent offsets, the hex constructors, and an
  isometric-autotile test confirming the square neighborhoods carry over.

## 0.38.0

**Flat-top hexagonal tilemap projection.** `TilemapProjection::Hexagonal` (v0.29.0) was pointy-top
only; the new `HexagonalFlat` variant is the **flat-top** counterpart in odd-q offset coordinates
(odd columns shifted down by half a tile) — the 90°-rotated mirror. `tile_size` is the flat-to-flat
**height**, and a flat-top hex is wider than tall. All four projection methods branch on it, so
`TilemapSystem` renders + picks it automatically. **Additive** — existing projections unchanged.

### Added
- `TilemapProjection::HexagonalFlat` — flat-top hex, odd-q offset. `cell_center_world` (col pitch
  `tile_size·√3/2` + odd-col half-shift-down), `cell_at_world` (flat-top pixel→axial→cube-round→
  odd-q), `cell_render_size` (`tile_size·2/√3 × tile_size`, wider than tall), `cell_z` (`-1`, no
  overlap).
- Example `hex_tilemap_flat` + generated `examples/assets/hex_tiles_flat.png` (flat-top hex atlas).
- 4 unit tests (odd-col offset, center↔world round-trip, off-center picking, render-size/z).

## 0.37.0

**Gamepad navigation for UI keyboard focus.** The focus pass (v0.31.0) was keyboard + mouse only.
It now also reads the first connected gamepad: **D-pad Down/Up** cycle focus (Up = reverse, like
Shift+Tab), **D-pad Left/Right** nudge a focused slider, and **A** (South) activates the focused
button/checkbox. Folded into `InputSnapshot` alongside the keyboard, so the existing focus-pass logic
(ring, activation, slider nudge, TextInput sync) is reused unchanged. **Additive** — no pad / no
`GamepadState` resource is a no-op; keyboard + mouse behavior is identical.

### Added
- Gamepad focus navigation in `UiSystem`'s focus pass (`src/ui/system/state.rs`): D-pad
  Up/Down/Left/Right + A from `GamepadState::primary()`.
- `ui_focus` example help text updated to mention the gamepad controls.

### Notes
- D-pad only (digital, edge-detected via `just_pressed`); the analog stick is not used (would need
  per-frame threshold debounce). Real-pad operation is a human check; the focus-move/activate logic
  is covered by unit tests via a new `GamepadState::test_press` test helper.

## 0.36.0

**Positional audio on a mixer bus for the wasm `WebAudio` path.** `play_at` (0.35.0) routed straight
to master; now `play_at_on_bus` routes a positional one-shot through a named bus, so the bus's
`set_bus_volume`/`duck_bus` scale the whole group on top of the sound's distance-based volume/pan.
A tiny additive composition of the existing positional + bus paths.

### Added
- `WebAudio::play_at_on_bus(bytes, source, listener, max_dist, bus) -> Sfx` — positional playback
  (distance falloff + x-offset pan) routed through a named mixer bus. The returned `Sfx`'s per-source
  volume/pan carry the spatial result, independent of the (downstream) bus level.
- `web_audio` example + `scripts/wasm_audio_smoke.sh` extended (headless lifecycle check now 38/38).

## 0.35.0

**2D positional audio for the wasm `WebAudio` path.** SFX could be panned/volumed by hand, but
positioning a sound from world coordinates was native-only (`AudioManager::play_at`). `WebAudio` now
computes volume + pan from 2D positions, reusing the existing per-source gain + stereo panner on the
[`Sfx`] handle. **Additive** — built entirely on the existing `Sfx` controls.

### Added
- `WebAudio::play_at(bytes, source, listener, max_dist) -> Sfx` — play a positional one-shot:
  volume falls off linearly (silent at `max_dist`), stereo pan follows the x-offset (native parity).
- `Sfx::update_position(source, listener, max_dist)` — reposition a playing sound each frame to
  track a moving source.
- `Sfx::volume()` / `Sfx::pan()` — read back the current per-source volume / pan.

### Done — "remaining native-only audio" backlog
- With ducking (0.34.0) and positional (this release), the wasm `WebAudio` mixer reaches native
  parity for the common cases. The only native-only audio feature left is **automatic sidechain**
  (`set_sidechain`), which needs continuous per-frame trigger-activity evaluation that doesn't fit
  the fire-and-forget Web Audio model — use manual `duck_bus`/`release_bus` instead.

## 0.34.0

**Bus ducking for the wasm `WebAudio` path.** Named buses could be volume-controlled but not ducked
— ducking was native-only (`AudioManager`). Each bus is now a two-gain chain `duck → volume →
master`: `set_bus_volume` drives `volume`, and the new `duck_bus`/`release_bus` ramp the `duck`
multiplier independently (so ducking never clobbers the bus volume and vice-versa), matching the
native mixer. Ramps run on the Web Audio clock (`AudioParam`), so — like the rest of the wasm audio
path — there is **no per-frame `update()` tick**. **Additive** — existing bus behavior is unchanged
(a bus rests at duck = 1.0, a transparent pass-through).

### Added
- `WebAudio::duck_bus(bus, gain, attack_secs)` / `release_bus(bus, release_secs)` — ramp a bus's
  duck multiplier toward `gain` (clamped `0.0..=1.0`) / back to `1.0`. `attack/release <= 0.0` is an
  instant set.
- `WebAudio::bus_duck(bus)` — the current duck multiplier (`1.0` if none / unknown bus).
- `web_audio` example + `scripts/wasm_audio_smoke.sh` extended to drive + self-check ducking
  (headless lifecycle check now 28/28).

### Changed (internal)
- Buses now store a `Bus { volume, duck }` two-`GainNode` chain (was a single `GainNode`); sounds
  routed to a bus connect to its `duck` input. `set_bus_volume`/`bus_volume`/`play_on_bus`/
  `play_sfx_on_bus` are unchanged in behavior. No public API change to those methods.

### Not ported (still native-only)
- **Automatic sidechain** (`set_sidechain`/`clear_sidechain`): it requires continuously evaluating
  "is the trigger bus playing?" every frame, which doesn't fit Web Audio's fire-and-forget model
  (and music isn't bus-routed on wasm). Drive ducking manually with `duck_bus`/`release_bus`.

## 0.33.0

**Track-to-track music crossfade for the wasm `WebAudio` path.** The music channel could be started
and stopped, but switching tracks meant a hard cut — crossfade was native-only (`AudioManager`).
`WebAudio::crossfade_music` now fades the current track out (then stops it) while the new track fades
in, so they overlap. Music now routes through a dedicated per-track `GainNode`, and the fades are
scheduled on the Web Audio clock (`AudioParam::linear_ramp_to_value_at_time`) — so, unlike the native
`Fade`/`update` infra, there is **no per-frame `update()` tick** and no temporary channel to tear
down. **Additive** — `play_music`/`stop_music` behave exactly as before (music just gains an internal
gain node); calling `crossfade_music` with nothing playing is simply a fade-in.

### Added
- `WebAudio::crossfade_music(bytes, dur)` — fade the music channel from the current track to a new
  one over `dur` seconds (no-current-track = fade-in).
- `web_audio` example + `scripts/wasm_audio_smoke.sh` extended to drive + self-check crossfade
  (headless lifecycle check now 22/22).

### Changed (internal)
- The music channel now stores a `MusicChannel { source, gain }` (was a bare source) so its volume
  can be ramped independently; `play_music`/`stop_music` updated accordingly. No public API change to
  those methods.

## 0.32.0

**Named mixer buses for the wasm `WebAudio` path.** The browser audio wrapper had a master volume
but no way to group sounds — bus mixing was native-only (`AudioManager`). `WebAudio` now has named
mixer buses: route sounds to a bus by name and control them together. A bus is just a named
`GainNode` wired `bus → master` (Web Audio is a node graph, so this needs no per-frame `update()`
tick, unlike the native fade infra). **Additive** — existing `WebAudio` calls route straight to
master exactly as before; this only adds the bus methods + `_on_bus` play variants.

### Added
- `WebAudio::set_bus_volume`/`bus_volume`/`bus_names` — per-bus volume (clamped `0.0..=1.0`) and the
  sorted list of known buses (native `AudioManager` mixer parity). Buses are created lazily on first
  reference; a volume-only bus (set without playing) persists in `bus_names`.
- `WebAudio::play_on_bus` / `play_sfx_on_bus` — fire-and-forget and controllable (`-> Sfx`) playback
  routed through a named bus (`source → [panner → per-source gain →] bus gain → master`).
- `web_audio` example + `scripts/wasm_audio_smoke.sh` extended to drive + self-check the bus surface
  (headless lifecycle check now 19/19).

## 0.31.0

**UI keyboard focus navigation.** UI widgets could only be operated with the mouse (and a clicked
`TextInput` typed). Now `UiSystem` has a focus pass: **Tab / Shift+Tab** cycle keyboard focus across
focusable widgets (`Button`, `TextInput`, `Slider`, `CheckBox`), a focus ring is drawn around the
focused widget, **Enter / Space** activate it (button click / checkbox toggle), **Left / Right**
nudge a focused `Slider`, and clicking a widget focuses it. **Additive** (one new resource +
auto-registered; existing UI behavior unchanged).

### Added
- `UiFocus` resource (`Option<Entity>`, auto-inserted) — the currently keyboard-focused widget;
  read it to style/inspect focus.
- `UiSystem` focus pass: Tab/Shift+Tab cycling (by entity index, skipping hidden/disabled widgets),
  focus ring, Enter/Space activation, Left/Right slider nudge, click-to-focus, and `TextInput`
  focus sync (a Tab-focused field receives typed characters).
- Example `ui_focus`.
- `UiEvent` now derives `PartialEq`.

## 0.30.0

**Autotile API unification + dead-code removal.** The two separate autotile component types are now
one. `TilemapAutotile` gains a `mode: AutotileMode` — `Single { mask_to_tile }` (any non-zero cell
connects, built with `edge_16`/`blob_47`) or `Multi { rules }` (per-terrain same-value, built with
the new `multi_edge_16`). This mirrors the dispatch `TilemapSystem` already did internally. The
ghost `ConnectRule` (a do-nothing marker struct + field) is removed. **Breaking** (0.x MINOR):
`MultiTerrainAutotile` and `ConnectRule` are gone, and `TilemapAutotile`'s `mask_to_tile` field
moved into `mode`. Single-terrain users calling `TilemapAutotile::edge_16(..)` are unaffected.

### Changed
- **`MultiTerrainAutotile::edge_16(&terrains)` → `TilemapAutotile::multi_edge_16(&terrains)`** —
  multi-terrain autotiling is now a `TilemapAutotile` in `AutotileMode::Multi`.
- `TilemapAutotile { neighborhood, oob_filled, mask_to_tile, connect }` →
  `TilemapAutotile { neighborhood, oob_filled, mode }` (the bitmask map lives in
  `AutotileMode::Single`).
- `TilemapSystem` reads the single `TilemapAutotile` (matches on `mode`) instead of two components.

### Removed
- `MultiTerrainAutotile` (folded into `TilemapAutotile` + `AutotileMode::Multi`).
- `ConnectRule` (unused marker / extension-point stub that never did anything).

### Added
- `AutotileMode` enum + `TilemapAutotile::multi_edge_16`.

## 0.29.0

**Hexagonal tilemaps.** Completes the projection set (C2 after C1's isometric): `TilemapProjection`
gains `Hexagonal` — a pointy-top hex grid in odd-r offset coordinates (odd rows shifted right by
half a tile), so the rectangular `tiles[row][col]` array maps straight onto it. `cell_center_world`
lays out the hex grid; `cell_at_world` picks via pixel → axial → cube-round (exact at hex borders);
hexes tessellate without overlap so they keep a fixed `z`. **Additive** — the new enum variant is
the only change to existing types.

### Added
- `TilemapProjection::Hexagonal` (pointy-top, odd-r offset).
- `Tilemap::cell_render_size()` — the sprite size `TilemapSystem` draws a tile at: square for
  orthographic/isometric, taller (`tile_size × tile_size · 2/√3`) for hexagons.
- `cell_center_world` / `cell_at_world` / `cell_z` now handle the hexagonal projection.
- Example `hex_tilemap` — a pointy-top hex grid with keyboard cell selection (reactive `set_tile`)
  and mouse hover-picking (`cell_at_world`); includes a generated `hex_tiles.png` atlas.

### Changed
- `TilemapSystem` sizes tiles via `cell_render_size()` (was a hardcoded square `tile_size`) so hex
  tiles get their taller sprite. No change for orthographic/isometric maps.

## 0.28.0

**Isometric tilemaps.** `Tilemap` could only lay out a square grid; now it has a
`TilemapProjection` — `Orthographic` (default, unchanged) or `Isometric` (a 2:1 diamond grid).
`cell_center_world`/`cell_at_world` branch on the projection (isometric picking inverts the diamond
transform and rounds to the nearest cell), and `TilemapSystem` depth-sorts isometric cells
back-to-front. **Additive — existing tilemaps are byte-identical** (projection defaults to
`Orthographic`). Hex grids are the next step (C2).

### Added
- `TilemapProjection` enum (`Orthographic` default / `Isometric`) + `Tilemap::with_projection`.
- `Tilemap::cell_z(row, col)` — the render `z` `TilemapSystem` assigns a cell (`-1.0` orthographic;
  `row + col` painter's-order depth for isometric).
- `cell_center_world` / `cell_at_world` now honor the projection (isometric diamond math).
- Example `iso_tilemap` — a diamond grid with keyboard cell selection (reactive `set_tile`) and
  mouse hover-picking (`cell_at_world`); includes a generated `iso_tiles.png` diamond atlas.

### Changed
- `TilemapSystem` places tiles via `cell_center_world` + `cell_z` (one call site handles both
  projections) instead of an inlined orthographic formula. No change for orthographic maps.

## 0.27.0

**wasm AEAD save/load browser verification.** v0.22.0 made the `save`/`load`/`save_versioned`/
`load_migrated` family cross-platform (hex-encoded ChaCha20-Poly1305 blob in `localStorage` on
wasm), but that path was only compile-gated + native-playtested — never *run* in a browser. This
closes that verification debt with an autonomous headless check. **No engine code change — example
+ tooling only.**

### Added
- Example `wasm_save` (wasm-only, `examples/wasm_save/`) — exercises the localStorage save backend
  end-to-end: `save` → `exists` → `load` round-trip, asserts the stored value is hex ciphertext
  (not plaintext), verifies **AEAD tamper detection** (a corrupted blob makes `load` fail), and a
  `save_versioned`/`load_migrated` round-trip + `delete`.
- `scripts/wasm_save_smoke.sh` — optional local (non-CI) headless check that runs the example and
  asserts the round-trip via the verdict read live over Chrome's DevTools endpoint. Result: **7/7**.

## 0.26.0

**WebAudio: controllable per-source SFX with stereo pan (wasm).** A step toward native↔wasm audio
parity: `WebAudio` could only fire-and-forget sound effects (`play`) — you couldn't pan, set a
per-sound volume, or stop one. New `play_sfx` returns an `Sfx` handle that does all three. Routes
`source → StereoPannerNode → per-source GainNode → master`; the panner + gain are created
synchronously, so `set_pan`/`set_volume` apply even before the clip finishes decoding. **Additive —
`play` and the rest of `WebAudio` are unchanged.** Crossfade, buses, ducking and full positional
audio remain native-only.

### Added
- `WebAudio::play_sfx(bytes) -> Sfx` — a controllable one-shot SFX.
- `Sfx` handle (re-exported at the crate root): `set_volume` (per-source, 0..1), `set_pan`
  (-1 left .. 1 right), `is_playing`, `stop`. Cloning a handle controls the same sound; if the
  per-source nodes can't be created it falls back to routing straight to master (volume/pan no-op).
- web-sys feature `StereoPannerNode`.
- The `web_audio` example + `scripts/wasm_audio_smoke.sh` now also exercise `play_sfx`
  (pan/volume/stop) — the headless lifecycle check is **12/12**.

### Fixed
- `examples/web_audio/web/build.sh` + `scripts/wasm_audio_smoke.sh` are now marked executable in
  git (were 0644), and the smoke script invokes `build.sh` via `bash` so it works regardless.

## 0.25.0

**Dialogue portrait rendering.** `DialogueBox::portrait` has existed (set via `with_portrait`) but
`DialogueSystem` only ever drew text, so the portrait was stored and never shown — this completes
the half-built feature. The system now draws the speaker's portrait to the left through the UI
image queue and shifts the text right to clear it; a box with no portrait renders exactly as before
(original left margin). **Additive — no breaking change, no new public API** (the `portrait` field
and `with_portrait` builder already existed).

### Added
- `DialogueSystem` renders `DialogueBox::portrait` (96×96 screen-space image, left of the text;
  text auto-shifts right when a portrait is present).
- Example `dialogue_portrait` — a multi-speaker conversation whose portrait switches per speaker,
  with a final portrait-less line showing the text-only fallback. Includes two generated portrait
  assets (`examples/assets/portrait_{sage,knight}.png`).

### Changed
- Internal: `DialogueSystem`'s per-frame gather uses a small `DrawItem` struct instead of a tuple
  (keeps `clippy::type_complexity` happy with the added portrait field). No behavior change.

## 0.24.0

**WebAudio runtime verification + first example.** Closes the v0.23.0 verification debt: the
wasm `WebAudio` mixer shipped compile-checked but had never *run* in a browser and had no
example. This adds a playable `web_audio` example that drives the whole surface and a headless
smoke harness that asserts the audio-graph lifecycle at runtime (all 9 lifecycle checks pass in
headless Chrome). Acoustic output stays a human step — there is no audio capture in the flow.
**Additive — no breaking change.**

### Added
- `WebAudio::is_running` — whether the `AudioContext` is unlocked and not suspended (for a "tap
  to enable sound" prompt or paused-audio indicator).
- `WebAudio::is_music_playing` — whether the music channel is occupied (for a music on/off UI),
  which also makes the async `play_music` decode observable.
- Example `web_audio` (wasm-only, `examples/web_audio/`) — generates an in-memory sine WAV and
  exercises `new` / volume set+clamp / `resume` / `play_music` (looping) / `suspend` / `resume`,
  reporting a pass/fail line per step plus a verdict in the page title.
- `scripts/wasm_audio_smoke.sh` — optional local (non-CI) headless check that runs the example
  and asserts the lifecycle via the verdict read live over Chrome's DevTools endpoint. Runs in
  real time (not `--virtual-time-budget`: the audio thread's suspend/resume transitions race a
  virtual clock).
- web-sys feature `AudioContextState`.

## 0.23.0

**WebAudio depth (wasm).** The browser audio player grows from fire-and-forget SFX into a
small but usable mixer: a master volume, a looping music channel, and pause/resume.

### Added
- `WebAudio::set_volume` / `volume` — a master `GainNode` that all playback routes through.
- `WebAudio::play_music` / `stop_music` — a single looping music channel (stops any current
  music, starts the new clip looping; `stop_music` stops it).
- `WebAudio::suspend` / `resume` — pause and resume all audio (also the call to satisfy the
  browser's user-gesture gate).
- web-sys features `GainNode`, `AudioParam`.

### Notes
- Native `AudioManager` (rodio) is unchanged. Per-source mixing, crossfade, buses, ducking and
  positional audio remain native-only.
- Compiles + wasm-clippy clean; runtime audio is verified in a browser (no autonomous audio
  capture), consistent with how the v0.17.0 WebAudio one-shot was checked.

## 0.22.0

**wasm AEAD save/load parity.** The encrypted player-save path (`save` / `load` /
`save_versioned` / `load_migrated`) now works on wasm, not just native — closing the gap where
those returned `SaveError::Unsupported` in the browser.

### Changed
- The ChaCha20-Poly1305 AEAD core (magic / nonce / cipher / encrypt / decrypt / versioned
  envelope / migration) is now **cross-platform**; the only per-target difference is the storage
  backend: a file on native, a **hex-encoded** blob in `localStorage` (keyed by the path string)
  on wasm. So `save` / `load` / `load_or_default` / `save_versioned` / `load_migrated` all work on
  both targets.
- Nonce generation switched `rand::thread_rng()` → `rand::rngs::OsRng` (works on wasm via
  `getrandom`'s `js` backend; `thread_rng` is not wired up there).
- `SaveError::Unsupported` now means "storage unavailable / future save version" rather than
  "no filesystem"; its `Display` text updated to match.

### Notes
- `localStorage` is user-inspectable, so the binary-embedded key gives tamper-detection +
  obfuscation, **not** secrecy against a determined user — the same trust model as the native
  save file (documented on `save_with_key`).

### Added
- Example `save_encrypted` — an encrypted launch counter persisted via `save` / `load` (a file
  natively, hex in `localStorage` on the web). Playtested windowed (count persists across runs;
  the saved file is `R2DAEAD01` magic + ciphertext, not plaintext).

## 0.21.0

**Particle RON→GPU builder.** `ParticleConfigSet::gpu_emitter(name)` builds a
`GpuParticleEmitter` from a RON particle config — the GPU-compute counterpart to `emitter()`
(the CPU path) — so a single `.ron` file can drive either path. Closes the 0.18.0 gap where
RON `gravity` / `emit_shape` reached only CPU emitters.

### Added
- `ParticleConfigSet::gpu_emitter(name) -> Option<GpuParticleEmitter>` (native-only, like
  `GpuParticleEmitter` itself — wasm has no GPU compute path; use `emitter()` there). The nine
  fields shared with the CPU emitter map 1:1 (`spawn_rate` / `lifetime` / `velocity` /
  `velocity_spread` / `color_start` / `color_end` / `gravity` / `emit_shape` / `emit`); the
  square GPU `size` takes the config width (`size.0`); `texture` / `z` have no GPU-emitter
  equivalent and are ignored.

### Changed
- Example `gpu_particles` now loads its emitter from `examples/gpu_particles.ron` via
  `App::load_particle_configs` + `gpu_emitter`, and auto-spawns one emitter at center so the
  RON→GPU path is visible on launch (more spawn on left-click). Playtested windowed.

## 0.20.0

**Data-driven dialogue: RON dialogue trees, conditional choices, and choice→event/effect
hooks.** Builds on 0.19.0's in-code branching `DialogueBox` to make conversations data-driven
and consequential — all purely additive (a box with no tree/cond/effect renders
byte-identically to 0.19.0; old scene RON still loads).

### Added
- **RON dialogue-tree loader** — `DialogueTree` (an ordered list of named nodes, each a line
  [literal or localization key] plus optional `goto`-by-id branching choices) flattens to an
  ordinary `DialogueBox` at spawn (node order = line index), so the existing `DialogueSystem`
  drives it unchanged. `DialogueRegistry` (World resource) + `App::load_dialogue(name, path)`
  load and hot-reload it, mirroring `load_animation_clips` / `load_particle_configs`. Parsing
  validates duplicate node ids, unknown `goto` targets, and literal/localized consistency.
- **Conditional choices** — `DialogueChoice` gains an optional `cond: DialogueCond` (compares a
  `DialogueVars` variable via `Eq/Ne/Gt/Lt/Ge/Le`); gated-out choices are hidden. `DialogueVars`
  is a new World resource of `DialogueValue` (`Bool/Int/Float/Str`) flags/counters.
- **Choice → event/effect hooks** — `DialogueChoice` gains an optional `effect: DialogueEffect`
  (`SetVar` writes a variable; `EmitEvent` sends a `DialogueEvent` to `Events<DialogueEvent>`).
  World-level `dialogue::advance(world, e)` / `dialogue::choose(world, e, i)` honor conditions
  and apply effects; `DialogueBox` gains `visible_choices` / `is_choosing` / `advance_with` /
  `choose_visible` for the vars-aware path (the original `advance` / `choose` stay for the
  simple case). Choice builders `when(cond)` / `then(effect)`. RON authors `cond` / `effect`
  inline via RON's IMPLICIT_SOME extension.
- Example `dialogue_quest` — loads a branching quest from `dialogue_quest.dlg.ron`, grants a
  lantern via an `EmitEvent` effect, gates a later "secret" choice on the granted variable, and
  flips EN↔KO live (the VISION acceptance test; playtested windowed).

### Changed
- `src/dialogue.rs` is now the `src/dialogue/` module (`mod.rs` + `tree.rs` + `vars.rs`).
- `DialogueChoice` no longer derives `Eq` (its new `cond` / `effect` can hold an `f32` via
  `DialogueValue::Float`); it still derives `PartialEq`.

### Deferred
- Per-line portraits (the dialogue renderer is text-only today) and a `DialogueBox`-level node
  `goto` (unconditional jumps use a single choice, as the examples do).

## 0.19.0

**Dialogue depth: `DialogueBox` gains localization keys and branching choices.** The Phase-4
`DialogueBox` was a linear typewriter over a `Vec<String>`; this makes it production-grade for
RPG/visual-novel use on two fronts — (1) lines/speaker/choices can be driven by translation keys
resolved against the existing `LocaleResource` each frame (so a live `set_locale` retranslates a
conversation mid-flow without losing the reader's place), and (2) a line can present numbered
choices that jump the conversation to another line. Purely additive — a box with no `line_keys`
and no `choices` renders byte-identically to before; both new field groups are `#[serde(default)]`
so pre-localization scene RON still loads. `DialogueSystem` stays input-agnostic (the game calls
`advance`/`choose`).

### Added
- Localization: `DialogueBox::localized(speaker_key, line_keys)` constructor + `line_keys: Vec<String>`
  / `speaker_key: Option<String>` fields + `resolve(&LocaleResource)`, which fills `lines`/`speaker`
  from translation keys **without** touching `current`/`elapsed`/reveal state (safe to call every
  frame). `DialogueSystem` resolves every box against the current locale before ticking. `src/dialogue.rs`.
- Branching: new `DialogueChoice { text, key: Option<String>, goto }` type (`new`/`localized` ctors,
  re-exported from the crate root) + `choices: Vec<(usize, Vec<DialogueChoice>)>` field + `with_choices`
  builder + `pending_choices()` / `choose(i)`. Selecting choice `i` jumps to its `goto` line (out-of-range
  `goto` clamps to the end, finishing the conversation); `advance()` is a no-op while a decision is
  pending so a plain advance can't skip a choice. Localized choice labels resolve like line keys.
  `DialogueSystem` draws the numbered choice list in place of the ▼ advance hint. `src/dialogue.rs`.
- `examples/dialogue_branching.rs` — a localized, branching merchant scene: SPACE advances, `1`/`2`
  pick choices, `L` toggles the locale between English and Korean live, `R` replays. The buy/dark
  branches show distinct lines and reconverge on a shared farewell. (12 new unit tests; the existing
  linear `dialogue_demo` is unchanged.)

## 0.18.0

**Particle depth, completed: `gravity` + `emit_shape` now reach the GPU emitter and RON configs too.**
Phase 6 (v0.16.0) added `gravity` and `emit_shape` to the CPU `ParticleEmitter` only, leaving the
compute-shader `GpuParticleEmitter` and the data-driven `ParticleConfigSet` (RON) without them. This
closes that follow-up on both fronts. Purely additive — zero gravity / `Point` shape / omitted RON
fields reproduce the prior behavior byte-for-byte; the native `AudioManager` and CPU particle paths are
untouched.

### Added
- `GpuParticleEmitter::gravity: Vec2` + `emit_shape: EmitShape` (with `with_gravity`/`with_emit_shape`
  builders), mirroring the CPU `ParticleEmitter`. Gravity is carried per-particle and integrated in the
  compute shader (`p.vel += p.gravity * dt`); the emit shape is sampled on the CPU at emission time via
  the existing `EmitShape::sample_offset`. `src/gpu_particle.rs`.
- RON `ParticleConfigSet` (`EmitterDef`) gains optional `gravity` and `emit_shape` fields via a private
  `EmitShapeDef` serde mirror (`Point` / `Circle(radius:)` / `Ring(radius:)` / `Box(half_extents:)`);
  both default to zero / `Point`. `src/particle/config_set.rs` (+ 4 unit tests).
- `examples/gpu_particles.rs` spawns with a gravity arc + `Circle` emit shape;
  `examples/games/data_particles/assets/particles.ron` exercises both new RON fields (hot-reloadable).

### Changed
- `GpuParticle` GPU struct grew 64 → 80 bytes (added `gravity: vec2<f32>` at offset 64 + padding to a
  16-byte-aligned stride); the compute and render WGSL `Particle` structs match. Native-only; the
  per-particle buffer is 4096 slots (≈320 KB). No public API removed.

## 0.17.0

**WASM audio (one-shot SFX) — Phase 7 of the user-experience roadmap, the final phase.** The native
`AudioManager` is rodio-based and native-only, leaving browser builds silent. `WebAudio` adds a minimal
Web Audio one-shot SFX path so wasm games can play sound effects.

### Added
- **`WebAudio`** (`src/audio_wasm.rs`, wasm-only) — a tiny `AudioContext` wrapper. `WebAudio::new()`
  creates the context; `play(bytes)` decodes an encoded clip (WAV/MP3/OGG, whatever the browser supports)
  and plays it once (fire-and-forget via `spawn_local`). Store it as a `World` resource. Re-exported as
  `engine::WebAudio` on `wasm32`. Added the Web Audio `web-sys` features.

### Scope / notes
- Intentionally minimal — one-shot SFX only. Music, mixing, fades, buses, ducking, and positional audio
  remain in the native `AudioManager` (rodio); a full cross-platform unification (e.g. via `kira`) is a
  separate future effort.
- Browsers gate audio behind a user gesture — trigger the first `play` from an input handler, or the
  context may stay suspended.
- ⚠️ The actual sound output was **not** verified this session (the dev machine was locked — no browser,
  no way to hear it — and audio has no meaningful unit test). The wasm code **compiles + lints clean** (CI
  Build (WASM) + wasm clippy) against the standard Web Audio API; **hearing it in a browser is the
  outstanding verification step.**

## 0.16.0

**Particle depth — Phase 6 of the user-experience roadmap.** `ParticleEmitter` gains the two knobs 2D
effects most need — per-particle gravity and a spawn shape — so fire, fountains, and scatter showers
are configurable instead of all-uniform. Additive: the defaults (`gravity = ZERO`, `emit_shape = Point`)
reproduce the prior behavior exactly.

### Added
- `ParticleEmitter::gravity: Vec2` — constant per-particle acceleration integrated each frame (`ZERO` =
  none, e.g. `(0, 300)` falling / `(0, -60)` rising). Builder `with_gravity`.
- `ParticleEmitter::emit_shape: EmitShape` — `Point` (default) / `Circle { radius }` / `Ring { radius }` /
  `Box { half_extents }`; new particles spawn at an offset sampled from the shape. Builder
  `with_emit_shape`. `EmitShape` re-exported as `engine::EmitShape`.
- Example `examples/particles_showcase.rs` — a fountain (gravity down), buoyant fire (gravity up + circle
  base), and box-scattered sparks.

### Notes
- The `GpuParticleEmitter` (native GPU path) does not yet mirror these fields — follow-up.
- Live visual playtest deferred (the dev machine was locked); the new math is unit-tested (gravity
  integration, zero-gravity = constant velocity, emit-shape sample bounds), and existing particle
  behavior is byte-identical with default values.

## 0.15.0

**WASM persistence — Phase 5 of the user-experience roadmap.** Browser-deployed games can finally save
data: the plain-text RON save functions route to `localStorage` on wasm instead of returning
`Unsupported`, using the same API as the native filesystem path.

### Changed
- `save::write_ron` / `read_ron` / `exists` / `delete` now work on `wasm32` via `localStorage` (keyed by
  the save-path string) — previously `Unsupported` / `false` / no-op. A small internal `wasm_storage`
  wrapper (web-sys `Storage`, newly enabled) backs them. `read_ron` returns a `NotFound` `Io` error for an
  absent key, so `unwrap_or` / default patterns behave the same as native.

### Added
- Example `examples/save_counter.rs` — a launch counter persisted with `write_ron`/`read_ron`; the
  identical code uses a file natively and `localStorage` on the web.

### Notes
- Player-save AEAD (`save` / `load` / `save_versioned`) stays `Unsupported` on wasm: a hardcoded key in a
  browser-inspectable store adds little, and binary ciphertext would need base64 in a string store. Use
  `write_ron` / `read_ron` for browser persistence.
- The live browser `localStorage` round-trip was not run this session (the dev machine was locked); the
  wasm code compiles + lints clean (CI Build (WASM) + wasm clippy) and the native path is unchanged.

## 0.14.0

**Dialogue box primitive — Phase 4 of the user-experience roadmap.** The most re-invented narrative
boilerplate (a speaker + typewriter text box) is now a first-class component, so RPG / visual-novel /
narrative forks no longer hand-roll it.

### Added
- **`DialogueBox`** component (`src/dialogue.rs`) — `speaker` + a list of `lines`, each revealed with a
  per-character typewriter at `chars_per_sec` (`<= 0` = instant). Two-stage `advance()`: the first press
  completes the current line's reveal, the next moves to the following line (then finishes after the
  last). `is_finished`, `reset`, optional `portrait` handle. UTF-8-safe reveal. Re-exported as
  `engine::DialogueBox`.
- **`DialogueSystem`** — ticks every box's typewriter (via the `query_mut` added in 0.13.0) and renders
  the active box (speaker + revealed text + an advance hint) as screen-space text near the bottom of the
  viewport. Input-agnostic: the game calls `DialogueBox::advance` (e.g. on Space). Re-exported.
- Example **`examples/dialogue_demo.rs`** — a multi-speaker conversation with the typewriter + advance.

### Notes
- Rendering is text-only (no background panel) so it composes with whatever box art the game draws; the
  `portrait` field is data for the game to render. For localization, resolve keys via `LocaleResource`
  into the box's lines (store resolved strings).

## 0.13.0

**Core API ergonomics — Phase 3 of the user-experience roadmap.** Removes the most common ECS
papercut (the "collect the entities, then `get_mut` each" workaround for mutating several
components together) and the scene-stack asymmetry. The flagship WASM demo is refactored onto the
new API, so the first code a newcomer reads no longer teaches the workaround.

### Added
- **`World::query2_mut<A, B>`** and **`World::query3_mut<A, B, C>`** — mutable multi-component
  queries yielding `(Entity, &mut A, &mut B[, &mut C])`. They borrow the distinct archetype columns
  simultaneously via `HashMap::get_disjoint_mut`, so a system updates several components in one pass
  with no allocate-every-frame collect step. `A`/`B`/`C` must be distinct types.
- **`App::push_scene`** / **`App::pop_scene`** — App-level convenience for `SceneCmd::Push`/`Pop`
  (stack a pause menu or overlay over the running scene and resume it), mirroring `App::set_scene`.

### Changed
- `run_demo` (`src/lib.rs`, the WASM demo) now uses `query2_mut::<Transform, BounceVel>` instead of
  collect-then-`get_mut`.
- `FORKING.md` + the `CLAUDE.md` module map document the mutable queries and the scene-stack helpers.

## 0.12.0

**Game-feel core ("juice") — Phase 2 of the user-experience roadmap.** Adds the highest-leverage
"feel" primitives — global time-scaling (hit-stop/slow-mo) and value/easing tweening — and a
`juice_demo` example that also gives the previously-undemonstrated `FadeTransition`, camera shake,
and `PostProcessConfig` their first playable example.

### Added
- **`TimeScale`** resource + **`App::set_time_scale`** / **`App::time_scale`** — a global multiplier
  applied to the `dt` that gameplay (scene) systems receive (`1.0` normal, `0.0` hit-stop, `0.5`
  slow-mo, `2.0` fast-forward). Built-in tail systems (hierarchy/gizmo) and engine post-frame work
  (fades, hot-reload, asset upload, camera) keep real time, so the editor and transitions stay
  responsive at any scale.
- **`RealDt`** resource — the real (unscaled) per-frame delta, written every frame before `TimeScale`
  is applied. Lets a system opt out of time-scaling (e.g. a hit-stop controller that sets
  `TimeScale(0.0)` still needs real time to end its own freeze).
- **`Tween<T: Lerp>`** is now generic over the value type (defaults to `f32`). `Tween<Vec2>`,
  `Tween<Color>`, etc. interpolate in one tween instead of juggling separate `f32` tweens. Existing
  `Tween::new(0.0, 100.0, 1.0)` call sites and `TweenSequence` (still `f32`) are unchanged.
- Four easing curves: **`EaseInBounce`**, **`EaseOutBounce`**, **`EaseInElastic`**, **`EaseOutElastic`**.
  The editor's Timeline easing picker lists them too.
- Example **`examples/juice_demo.rs`** — hit-stop + camera shake + vignette pulse on impact, a
  `Tween<Vec2>` slide-in, and a row of sprites bobbing with the new easing curves. Doubles as the
  acceptance example for `FadeTransition`, `Camera::shake`, and `PostProcessConfig`.

### Changed
- **`Easing` is now `#[non_exhaustive]`** — adding future curves is no longer a breaking change.
  Downstream `match`es on `Easing` must add a `_` arm. (A one-time break, taken now while pre-1.0.)

**First-hour onboarding pass (docs + example, no library API change).** Lowers the barrier
for a new forker's first hour: a true minimal "image on screen" example, a fork-first README
that no longer lies about installation, and an English getting-started guide. No `src/` change,
so the public API and the 772-test suite are untouched.

### Added
- `examples/hello_sprite.rs` — the smallest textured-sprite example (load a PNG → render it),
  filling the ladder gap between `basic.rs` (solid-color, no asset) and the full example games.
  It demonstrates the asset workflow (`App::load_image` → `Sprite::textured_with_handle`) and
  is the recommended starting point to copy into your own `examples/my_game.rs`.
- `examples/assets/player.png` — a 32×32 placeholder sprite for `hello_sprite`.
- `FORKING.md` — English getting-started guide: the fork-first model, crate layout, how to
  start your own game, asset-path resolution, the borrow-split pattern, and the verify gate.

### Fixed
- `README.md` — replaced the false `skeleton-engine = "2.0.0"` crates.io install block (the
  crate is unpublished and at 0.x) with a fork-first **Getting Started** section; corrected the
  stale MSRV (`1.88` → `1.95`); de-versioned the obsolete "v2.0 notes" framing; noted that
  `REFERENCE.html` / `ARCHITECTURE.html` are written in Korean and linked `FORKING.md`.
- `CLAUDE.md` — the module-map row for `#[derive(Reflect)]` said "(`derive` feature, default
  on)", but that feature was removed when `engine_reflect_derive` became a path dev-dependency;
  corrected to describe the actual state.

## 0.11.0

**Version line reset: pre-1.0.** No code changes. The project moves from the 10.x SemVer
line back to a 0.x ("pre-1.0") line to honestly signal that the public API is not yet
stability-committed and may break between releases — matching the engine's actual state
(feature-rich but still evolving, single author, never published). The full prior history
(0.3.0 → 10.7.0) is preserved below and in git tags; only the go-forward line is
renumbered. 0.x cadence: MINOR = any release (incl. breaking), PATCH = point fix. 1.0.0
will mark a deliberate compatibility commitment later.

## 10.7.0

**Nine-slice (9-patch) scalable sprites (new feature + example).** Resizing a bordered/rounded
sprite (a UI panel, button, frame) by scaling the whole quad distorts its corners. `NineSlice`
makes the sprite renderer emit nine sub-quads instead of one — the four corners keep their fixed
size while the edges and center stretch to fill. Additive: the new branch only runs when a
`NineSlice` component is present, so ordinary sprites render byte-identically.

### Added

- `NineSlice` component — `border: [f32; 4]` (world-pixel border widths) + `uv_border: [f32; 4]`
  (matching source-texture UV fractions), both indexed `[left, right, top, bottom]`. Constructors
  `new` and `uniform(border_px, uv_frac)`. Re-exported as `engine::NineSlice`.
- The sprite pass computes the nine sub-quads (each with its own model matrix + UV sub-rect) for a
  `NineSlice` entity; corners stay fixed-size at any panel size, and the whole panel rotates rigidly.
  Does not apply to `AtlasSprite` or entities carrying a `ShaderMaterial`.
- Example `examples/nine_slice.rs` — one generated bordered texture drawn at many sizes (wide/tall/
  small/large), a rotating panel, and a naive-stretch comparison showing the corner distortion a
  9-slice avoids.

## 10.6.0

**Animated tiles (new feature + example).** A `Tilemap` was static per cell; common 2D needs
(water, lava, animated decals) want per-tile frame animation. `TileAnimationSet` maps a tile value
to a `TileAnimation` (frame list + frame time); matching cells cycle their atlas frame at runtime.
Decoupled from the reactive diff so non-animated maps pay nothing. Additive.

### Added

- `TileAnimation` (frame ids + `frame_time`, `frame_at(elapsed)`), `TileAnimationSet` (a component on
  the tilemap entity mapping a tile value → animation), `AnimatedTileCell` (per-tile-entity tag with
  precomputed frame UVs), `AnimatedTileSystem` (cycles tagged cells' `UvRect` each frame). Re-exported
  at the crate root.
- `TilemapSystem` tags animated cells at spawn and refreshes the tag when a cell's value changes; the
  per-frame cycling is render-only and does not bump the tilemap generation, so the unchanged-map
  fast path is fully preserved.
- Example `examples/animated_tiles.rs` — a procedurally generated atlas with cycling water/lava tiles
  beside static ground.

## 10.5.0

**Coroutine sequencer (new feature + example).** The engine had `Timer`, `Tween`, and
`Timeline` (data keyframes) but no imperative "do these actions with waits between them"
primitive. `Coroutine` adds scripted-gameplay sequencing — chain `wait` / `run` / `run_for`
steps that execute arbitrary closures against the `World`. Distinct from `Timeline`
(keyframe data) and `TweenSequence` (value interpolation). Purely additive.

### Added

- `Coroutine` — builder: `new` / `wait(secs)` / `run(|&mut World|)` / `run_for(dur, |&mut World, t|)`
  (progress `t` runs 0→1). `CoroutineRunner` — World resource (`start` / `active_count`).
  `CoroutineSystem` — ticks active coroutines each frame; it removes the runner resource,
  ticks (so closures get a free `&mut World`), then reinserts it (closures must not re-enter
  the runner). Leftover `dt` carries across steps within a frame. Re-exported at the crate root.
- Example `examples/coroutine_demo.rs` — a scripted scene: wait → spawn a box → slide it across
  via `run_for` → recolor → loop.

## 10.4.0

**Music-track crossfade (new feature + example).** `AudioManager` had per-channel fades
(`fade_out` / `play_fade_in` / `fade_volume`) but no single call to crossfade one track into
another with the two overlapping. `crossfade` adds it: the current track on a channel fades out
while the new track fades in, reusing the existing `Fade` + `update` infrastructure. Native-only,
purely additive.

### Added

- `AudioManager::crossfade(channel, new_path, repeat, dur)` — relocates the channel's current sink
  to an internal temp channel and schedules a stop-when-done fade-out there, then `play_fade_in`s
  the new track on the channel, so the two overlap. Degrades to a plain fade-in when nothing is
  playing. The temp sink is torn down by `update()` when its fade completes.
- Example `examples/music_crossfade.rs` — generates two short sine-wave WAVs in the temp dir and
  crossfades between them on a key press.

## 10.3.0

**`TweenSequence` — chained tweens (new feature + example).** `Tween` interpolates one value
over one duration; there was no primitive to chain multiple eased legs into a single animation.
`TweenSequence` plays a list of `Tween` segments back-to-back, each with its own easing,
optionally looping, carrying leftover `dt` across segment boundaries so a large frame step
doesn't stall on a segment edge. Purely additive.

### Added

- `TweenSequence` — builder (`new` / `then` / `push` / `looping`) + runtime (`tick` / `value` /
  `finished` / `fraction` / `reset` / `current_segment` / `segment_count`). Re-exported as
  `engine::TweenSequence`.
- Example `examples/tween_sequence.rs` — a square loops a rectangular path driven by two
  `TweenSequence`s (one per axis), each leg using a different easing.

## 10.2.1

**Partial split of `App::render()` (v10 item F — internal, risk-managed).** The ~890-line
`render()` god-function had its **separable concerns** extracted into named helper methods, while the
inherently-sequential scene-pass core (sprite → UI → particles → plugins → post → lighting → text →
fade, whose `render_view` aliases into `RenderState`) was deliberately **left inline** as one annotated
flow — splitting it would mean threading the encoder + target view through eight micro-functions, hurting
readability and adding GPU-silent-regression surface for no benefit. `render()` drops from ~890 to ~610
lines. No public API change.

### Changed (internal)

- Extracted from `App::render()` into private helpers on `impl App`: `setup_post_renderer` /
  `setup_lighting` (pre-frame renderer init/resize), `render_offscreen_targets` (the per-`OffscreenCamera`
  RT pass, each its own submission), `present_docked_placeholder` (the docked-editor RT warm-up frame),
  and `present_egui` (the final egui overlay pass). Behavior is byte-identical — same operation order,
  submit boundaries, and `cfg` gates.
- Verified by a **full render-mode visual playtest** (CI has no GPU test): normal + custom-shader
  pipeline (`shader_material`), offscreen RT (`security_camera`), lighting + post-process
  (`lit_dungeon`), docked editor (`basic` + F2), GPU particles (`gpu_particles`), and a fade-using
  scene (`timeline_cutscene`) all render correctly with no validation errors.

## 10.2.0

**Parallax scrolling (new feature + example).** A genre-agnostic 2D primitive the engine lacked —
background/foreground layers that scroll at a fraction of the camera's motion to fake depth.
Purely additive; pairs with the existing camera shake/follow.

### Added

- `ParallaxLayer` component — `factor: Vec2` per-axis scroll rate (`1.0` = world-locked, `0.0` =
  screen-locked, `0.0..1.0` = depth, `>1.0` = faster-than-world foreground). Constructors `new` /
  `horizontal(fx)` / `vertical(fy)`. The rest anchor is **lazily captured** from the entity's
  `Transform` on the first system run (plus the camera position at that moment), so you just place
  the sprite — no base bookkeeping.
- `ParallaxSystem` — offsets every `ParallaxLayer` entity's `Transform` each frame via
  `pos = base + (cam - cam_ref) * (1 - factor)`. Add with `app.add_system(ParallaxSystem)`. As a
  normal user system it reads the camera from the end of the previous frame (engine finalizes camera
  follow after the user loop), a sub-perceptual one-frame lag for backgrounds; add it after the
  systems that move the camera-followed entity.
- Both re-exported at the crate root (`engine::{ParallaxLayer, ParallaxSystem}`).
- Example `examples/parallax_scroll.rs` — a playable side-scroller: A/D moves a player, the camera
  follows (via an anchor entity, since `Camera::position` is the viewport top-left), and four layers
  (sky `0.10`, mountains `0.35`, trees `0.65`, foreground `1.10`) scroll at visibly different rates.

## 10.1.0

**`ShaderMaterial` example (VISION acceptance test, additive).** The custom per-entity
fragment-shader feature (`ShaderMaterial`, shipped earlier) had **no example** exercising it — a gap
against the VISION rule that a feature isn't done until a playable example does. This adds one, which
also serves as the only end-to-end validation of the `MaterialRenderer` custom-pipeline path (CI has
no GPU test). No library/API change.

### Added

- Example `examples/shader_material.rs` — three side-by-side sprites, each with a **distinct** custom
  WGSL fragment shader (hue-cycle, sin-wave plasma, noise dissolve), exercising the renderer's
  per-source-hash pipeline cache with multiple live pipelines at once. A system writes `params[0] =
  elapsed` into all three each frame (the per-frame `world.get_mut::<ShaderMaterial>` update path), and
  ↑/↓ drive the dissolve sprite's threshold (`params[1]`). Self-contained — uses `Sprite::colored`
  (white 1×1 fallback texture) so no asset files are needed, while still proving the `t_sprite` /
  `s_sprite` bindings. Validates the documented `ShaderMaterial` shader contract (subset `VertexOutput`,
  `@group(1)` texture + `@group(2)` params bindings) end-to-end via a visual playtest.

## 10.0.0

**v10 architecture pass** — a scoped set of breaking + internal refactors from the cohesion review
(`docs/MODULE_COHESION_REVIEW_2026-06-16.md`); plan + PR sequencing in
`plans/V10_BREAKING_PASS_PLAN_2026-06-16.md`. The one planned item **not** done — splitting the 839-line
`App::render()` — was intentionally descoped: its fork-friendliness goal is already met by the
`RenderPlugin` hook (added in 9.6.0), and it's the only refactor CI can't verify (no GPU test), so the
internal-readability gain didn't justify the render-path regression risk.

### Added

- `RenderTarget` escape-hatch read accessors `texture()` / `view()` / `sampler()` / `bind_group()`
  (the underlying wgpu objects are no longer `pub` fields; these mirror the physics `.raw()` hatches).
- `ScriptRegistry` — a World resource that owns Rhai script storage/loading + hot-reload (split out of
  `AssetServer`; see the scripting-decouple entry below).

### Changed (internal)

- Split the 1620-line `src/tilemap.rs` into `src/tilemap/{mod,autotile,system}.rs` (data model /
  autotiling / reactive render system), mirroring `src/physics/`. Pure relocation — the 10
  re-exported public names (`engine::tilemap::*`) are unchanged.
- Extracted the 14 renderer/texture/egui fields from the `App` god-struct into a new internal
  `RenderState` (`src/app/render_state.rs`); `App` now holds one `render: RenderState` field (`gpu`
  and `world` stay on `App`). No public API change — sets up the `update()` split.
- Split the 386-line `schedule::update()` god-function into `compute_viewport()` / `run_systems()` /
  `post_systems()` helpers, and moved egui frame begin/end into `egui_pass.rs`. Operation order is
  unchanged (guarded by the existing pause / egui-delta-merge / scene-transition tests). Internal-only.
- Split the 5-concern `SpriteRenderer` into `SpriteRenderer` (sprite batching + UI primitives) owning
  a `TextureCache` (texture/RT-bind-group cache + texture layout) and a `MaterialRenderer` (ShaderMaterial
  custom pipelines). All bind-group layouts / pipeline configs / draw order are byte-identical (verified
  by a visual smoke test of the `basic` + `security_camera` examples). Internal-only (new types `pub(crate)`).

### Changed (breaking)

- **`UiSystem` and `SteeringSystem` are no longer unit structs** — they now hold reused scratch
  buffers, so construct them with `UiSystem::default()` / `SteeringSystem::default()` (or `::new()`)
  instead of the bare `UiSystem` / `SteeringSystem` in `add_system(...)`. Eliminates 11 per-frame
  `Vec<Entity>` allocations (6 UI widget passes + 5 steering passes); behavior is identical. (Closes
  the deferred allocation finding #76.)
- _(Theme 3 encapsulation)_ `RenderTarget`'s wgpu fields `texture` / `view` / `sampler` /
  `bind_group` are now `pub(crate)` (use the new accessors above). `width` / `height` / `clear_color`
  stay `pub`.
- `RenderTarget::new` no longer takes a `texture_layout` argument — it builds its own bind-group
  layout internally. Forks create RTs via `App::create_render_target`, which is unchanged.
- `LightingRenderer`'s `normal_view` / `width` / `height` are now `pub(crate)` (native-only type,
  not in the prelude).
- **Scripting decoupled from the asset module.** `ScriptAsset` (and its Rhai `ast`) moved out of
  `src/asset.rs` into `src/scripting/`, and script storage/loading/hot-reload moved from `AssetServer`
  to the new `ScriptRegistry` resource — so `asset.rs` no longer references Rhai (a forker can swap
  scripting backends without touching the generic asset module). `engine::ScriptAsset` is still
  re-exported at the crate root (source-compatible), but `engine::asset::ScriptAsset` no longer exists
  and `ScriptAsset::ast` is now `pub(crate)`. `App::load_script` is unchanged.

### Removed (breaking)

- `engine::tilemap::cell_display_uv` — an unused public helper (zero callers in-tree; was redundant
  with `TilemapSystem`'s inline UV resolution).
- `SpriteRenderer::texture_layout()` — unused after `RenderTarget::new` became self-contained.

### Fixed

- `scripts/verify.sh` is now executable in git (`100755`); the documented `./scripts/verify.sh` gate
  command previously failed with "permission denied" on a fresh clone (it relied on a local-only
  execute bit).

## 9.6.1

**Cleanups (additive / docs / CI).** Small fork-friction + doc-quality fixes.

### Added

- `Wander::direction_fn: Option<fn(u32, Vec2) -> Vec2>` + `Wander::with_direction_fn(f)` builder —
  lets a game override the wander direction picker (e.g. plug in real `rand`) without forking
  `SteeringSystem`. Defaults to `None` (the existing deterministic built-in picker), so behavior is
  unchanged unless set. It's a plain `fn` pointer, so `Wander` stays `Clone`/`Debug`.

### Fixed

- Two broken rustdoc examples that never compiled (CI skipped doctests): `register_serde_component`
  used the stale `App::new(Default::default())` (now `App::new()`), and `register_editable_component`
  imported the `Reflect` *trait* instead of the derive macro (now `use engine_reflect_derive::Reflect`).
- CI + `scripts/verify.sh` now run `cargo test --doc` (which `--all-targets` skips) so fork-facing
  doc examples can't silently rot again.

## 9.6.0

**Pluggable render-pass hook (cohesion review item 7, additive).** A fork could not inject a
custom GPU pass (outlines, shadows, debug overlays, screen effects) without forking the engine's
`render()`. Now there's a registration hook. Fully additive — when no plugin is registered the
dispatch is skipped and the rendered output is byte-identical to before.

### Added

- `RenderPlugin` trait — `fn record(&mut self, ctx: &mut FrameContext, world: &World, viewport: (u32, u32))`.
  Implement it to record a custom render pass; runs once per frame after the main
  sprite/UI/particle passes and **before** post-processing/lighting, so downstream effects still
  apply to whatever the plugin draws. Read-only `&World` access. Native + wasm.
- `App::add_render_plugin(impl RenderPlugin + 'static) -> &mut Self` — registers a plugin; plugins
  run in registration order.
- `FrameContext` gains a `pub format: wgpu::TextureFormat` field (the target view's format) so a
  plugin can build its own `wgpu::RenderPipeline`. `FrameContext` and `RenderPlugin` are now
  re-exported from the crate root.
- Example `examples/render_plugin.rs` — an animated vignette plugin (lazy self-built pipeline via
  `ctx.format`, reads a `Pulse` ECS resource each frame, composites with `LoadOp::Load`).

## 9.5.1

**Internal cleanup (cohesion review, behavior-identical).** No public API or behavior change.

### Changed (internal)

- `AssetServer`'s three separate hot-reload watch-sets (`data_table_paths` /
  `animation_clip_paths` / `particle_config_paths`) are unified into one `watched_paths` set with
  a single `watch_path` method. The three public `watch_*` methods are **kept as delegates** (no
  break). Forwarding was already unified via `HotReloadable` in 9.3.0, so adding a new
  hot-reloadable registry no longer needs a new watch-set/method/branch (closes the OCP finding).
- `CameraUniform` (identical `#[repr(C)]` view-proj struct) was duplicated in `gpu_particle.rs`
  and `sprite/geometry.rs`; now defined once as `pub(crate)` in `renderer`.

## 9.5.0

**Pluggable editor inspector panels (cohesion review item 7, additive).** The docked editor's
inspector hardcoded its per-component sub-panels, so a fork couldn't add one for their own
component without editing `docked.rs`. Now there's a registration hook. Fully additive.

### Added

- `App::register_inspector_panel::<T>(title, draw)` (native-only) — registers a collapsing
  inspector sub-panel shown whenever the selected entity has component `T`. `draw` is
  `Fn(&mut egui::Ui, &mut App, Entity)`. Forks can add inspector UI for their own components
  without touching the engine.

### Changed (internal, behavior-identical)

- The four uniform built-in inspector panels (Particle Tuner / Point Light / State Machine /
  Timeline) are now registered through `register_inspector_panel` and dispatched by a single
  loop instead of hardcoded `if has_component` blocks (`docked.rs` −81 lines). Tile Paint stays
  hardcoded (non-uniform shape). No user-visible change.

## 9.4.1

**Module-home reorganization (cohesion review item 3, pure relocation).** Zero behavior change,
all public API paths preserved via re-export. Splits two god-files for fork-friendliness.

### Changed (internal — no API change)

- `SerdeComponentRegistry` + `SerdeComponentEntry` moved out of `prefab.rs` into a dedicated
  `serde_registry` module (re-exported from `prefab`, so `engine::SerdeComponentRegistry` is
  unchanged).
- The docked editor's **State Machine** and **Timeline** inspector panels were extracted from the
  2003-line `editor/ui/docked.rs` into `editor/ui/state_machine_panel.rs` +
  `editor/ui/timeline_panel.rs` (mirroring the existing `audio_panel.rs`/`data_table_panel.rs`).
  `docked.rs` 2003 → 1259 lines.
- The tile-paint input methods moved out of `editor/ui/gizmo.rs` into `editor/ui/tile_paint.rs`
  (tile painting is not a gizmo op). `gizmo.rs` 1869 → 1183 lines.
- *(`CameraUniform` dedup was skipped — the two definitions differ in field visibility.)*

## 9.4.0

**Module-cohesion review follow-ups (safe additive subset).** The first batch from
`docs/MODULE_COHESION_REVIEW_2026-06-16.md` — the non-breaking, behavior-identical items.
Breaking/architectural items (the `App`/`render()` extraction, encapsulation tightening,
`UiSystem`/`SteeringSystem` scratch fields) are deferred to a future `v10` design pass.

### Added

- `engine::AssetLoadError` — a shared RON/IO asset-load error. `ClipSetError` and
  `ParticleConfigError` are now type aliases of it (their public names + variant names are
  unchanged, so existing `match` arms keep compiling).
- `JointHandle::raw()` — escape hatch to the underlying rapier `ImpulseJointHandle`, matching
  the existing `BodyHandle::raw()` / `ColliderHandle::raw()` pattern (native-only).

### Changed (internal, behavior-identical)

- `SpriteRenderer` now reuses three scratch buffers (`atlas_entries`, `live_material_entities`,
  `seen_new_hashes`) across frames instead of allocating them per `render()` (closes the
  per-frame-allocation finding #72 from the hardening coverage ledger).
- `Tilemap::compute_tile_mask` / `compute_tile_mask_typed` now delegate to one shared
  `compute_mask_raw` (a closure-parameterized core), removing ~50 lines of copy-pasted Blob8
  logic — output is bit-identical.

### Docs

- `src/animation` gained a module-level "System registration order" doc (BlendTreeSystem →
  AnimationSystem → StateMachineSystem).
- `register_editable_component` now documents that its editor factory/remover halves are
  native-only (reflect+clone+serde still apply on wasm).

## 9.3.0

**`HotReloadable` trait — fork-friendly hot-reload extension point.** The hot-reload loop
forwarded changed asset paths to a hardcoded set of three registries via an internal macro;
adding a new hot-reloadable registry required editing engine internals. It's now a public
trait + registration, so forks register their own registries without touching the engine.
Fully additive (the three built-ins are auto-registered — behavior unchanged). Native-only,
matching the existing hot-reload code.

### Added

- `engine::HotReloadable` trait (`fn reload_path(&mut self, path: &str)`) — implement it on a
  resource to make it hot-reloadable.
- `App::register_hot_reloadable::<T: HotReloadable>()` — register a resource to receive every
  changed asset path each frame.
- `DataTableRegistry`, `AnimationClipRegistry`, and `ParticleConfigRegistry` implement
  `HotReloadable` and are auto-registered in `App::new` (replacing the internal
  `forward_reloads!` macro; same runtime behavior).

## 9.2.0

**Editor depth — State-Machine & Timeline panels gain real editing.** The docked editor's
SM and Timeline inspector panels were display-mostly; they now author content. Fully additive.

### Added

- `AnimationStateMachine::set_transition_conditions(from, index, conditions) -> bool` and
  `set_transition_crossfade(from, index, seconds) -> bool` — mutate an existing transition's
  conditions / crossfade (false on missing state or out-of-range index).
- **State-Machine panel:** live parameter editing (bool checkbox / float drag / trigger fire),
  add-transition (target ComboBox + crossfade), and per-transition condition add/remove — all
  routed through the tested edit ops.
- **Timeline panel:** per-track add-keyframe (empty tracks now render an add button) and
  per-track-type value editing (Vec2 = x/y drags, f32 = drag, Color = r/g/b/a drags), in
  addition to the existing time / easing / remove controls. `timeline_track_ui` now takes a
  `make_default` + `value_edit` closure instead of a read-only `fmt`.

### Notes

- The egui panel *layout* (condition-editor row width, value-drag rows) is functional but not
  visually tuned — a known cosmetic follow-up. All data mutations go through unit-tested ops.

## 9.1.0

**Editor-edit persistence** — `AnimationStateMachine` and `Timeline` now persist through
scene save/load. Both component families gained `serde::{Serialize, Deserialize}` derives and
are **auto-registered** in the `SerdeComponentRegistry` (alongside the UI widgets), so the
in-editor State-Machine and Timeline editors' edits survive a save/load round-trip with no
user action. Fully additive.

### Added

- `AnimationStateMachine`, `AnimParam`, `TransitionCond`, `AnimTransition`, `AnimState` now
  derive `Serialize, Deserialize` (and `PartialEq`).
- `Timeline`, `Track<T>`, `Keyframe<T>`, `CameraTarget` now derive `Serialize, Deserialize`.
  `Track<T>`/`Keyframe<T>` serialize for any `T: Serialize` (the concrete tracks are
  `Vec2`/`f32`/`Color`).
- `Easing` (`src/tween.rs`) now derives `Serialize, Deserialize` (required by `Keyframe`).
- `AnimationStateMachine`, `Timeline`, and `CameraTarget` are auto-registered for serde in
  `register_core_component_metadata`, so scene save/load captures them automatically.

### Notes

- `Timeline::time` and `Timeline::playing` are serialized (lossless save of the playback
  position); add a `post_spawn` hook if you want a reset-on-load policy.

## 9.0.0

Engine-wide **hardening pass** — 80 findings from a 14-subsystem code analysis
(`docs/CODE_ANALYSIS_2026-06-16.md`). Dominant theme: **fail-loud over fail-quiet** — bad
input/data/state that used to silently misbehave now panics-guards, logs, or returns `None`.
Mostly additive; a handful of small breaking changes are listed first.

### Changed (breaking)

- **`NetworkSystem` is now a struct** (holds warn-once state for a missing `Events<NetworkEvent>`).
  Construct it with `NetworkSystem::new()` (or `::default()`) instead of the bare `NetworkSystem`
  unit value — `app.add_system(NetworkSystem::new())`.
- **`GamepadButton` / `GamepadAxis` are `#[non_exhaustive]`** — external `match`es must add a
  wildcard (`_ =>`) arm. Future button/axis additions are now non-breaking (before v1.0 freeze).
- **`SerdeComponentEntry` gained a `has_component` field** — only `register_arc` constructs it
  internally, but external code building the struct literal directly must add the field.
- **`SolidTiles::Only` stores a `HashSet<u32>`** (was `Vec<u32>`) for O(1) per-tile lookup; an
  additive `IntoIterator` constructor preserves the ergonomic build path.
- **MSRV raised 1.92 → 1.95** to match the only toolchain CI actually verifies (the declared 1.92
  was never built/tested).

### Added

- `ParticleEmitter::z` + `with_z()` (+ RON `EmitterDef` `z`) — spawned particles inherit the
  emitter's z-depth instead of being hardcoded to `0.0`.
- `Track<T>::set_value(i, v)` / `set_easing(i, e)` keyframe mutators; the editor Timeline panel
  wires per-keyframe easing editing.
- `World::has_component::<T>()` public existence check (no wasteful downcast).
- `save_versioned_with_key` / `load_migrated_with_key` (custom-key + versioned migration).
- `InputMap::axis_value(action, gamepad, pad)`; gamepad **axis** bindings are now honored by
  `just_pressed` / `just_released` (previously axis-only actions always read `false`).
- `Camera` shake / zoom-tween accessors; `RenderTarget` per-target `clear_color`.
- `Panel::direction` exposed via `Reflect`; `LocalizationSystem` can bind `TextInput.placeholder`.
- Editor: `+ Add Component` / remove `✕` now cover all 8 serde UI widgets (UiNode/Button/Label/
  TextInput/Slider/CheckBox/ScrollView/Panel); `SerdeComponentRegistry::component_names_for`
  (presence check without RON serialization).
- Audio `clear_file_cache()`; hot-reload dispatch de-duplicated so a fork wires a new RON registry
  in one line.
- Four crate-boundary integration tests (`tests/{pathfinding,timeline,behavior,save}_smoke.rs`).

### Fixed

- **`blob_47` autotile mask table** used the wrong bit convention — 36 reachable masks silently fell
  back to atlas tile 0 (plain orthogonal-neighbor tiles rendered as tile 0).
  **Migration:** the mask→atlas-index mapping is now the canonical Blob-47 order. If you previously
  rearranged your atlas to match the old (mostly tile-0) indices, regenerate/reorder it to match.
- **Gamepad axis `just_pressed`/`just_released`** ignored axis bindings (stick-triggered one-shot
  actions never fired).
- **Held keys stuck on focus loss** — `WindowEvent::Focused(false)` now calls `InputState::release_all()`
  (no phantom `just_released`); spurious `just_released` for never-pressed keys fixed.
- **macOS modal resize double-stepped** physics/tween/timer (`Resized` + `RedrawRequested` both
  stepped in one iteration) — guarded to one step per event-loop iteration.
- **A panicking system no longer runs the frame's remaining systems** on a half-mutated World
  (frame aborts; the system is disabled for subsequent frames).
- `find_path` / `find_path_diagonal` return `None` (not `Some([blocked])`) for a blocked `start == goal`.
- Animation RON robustness: `columns == 0` div-by-zero panic, out-of-bounds frame index, `play(OOB)`
  freeze + immediate `AnimationEnd`, dead-transition (nonexistent target) warning, skeletal
  `is_finished()` at construction.
- `add_prismatic_joint` zero-axis NaN guard; `contact_pairs` ordered-pair symmetry.
- Audio fade interactions: `set_bus_volume`/`set_volume`/`update_position` mid-fade no longer snap/drop.
- **Offscreen render loop**: raw `*const TextureView` (dangle-on-realloc UB) replaced with owned
  `TextureView` handles — `unsafe` removed.
- TextInput focus respects z-order and ignores hidden widgets; inspector write-back keyed by `TypeId`.
- Many **fail-loud `log::warn!`s** where data silently vanished (DataTable extra columns, serde
  serialize failures, missing registries, dropped network events, glyphon errors).
- wasm: `web_sys::ErrorEvent` feature enabled so the network `on_error` diagnostic path compiles.

### Performance

- **Tilemap** no longer clones the full grid every frame — a `generation` counter dirty-guards the
  `TilemapSystem` (idle cost is one `u64` compare); removed-entity check is a `HashSet`.
- Per-frame scratch allocations reused as fields across `SpriteRenderer`, `PhysicsSystem`,
  `SteeringSystem`, `SpatialGrid::rebuild`, `AudioManager::update`, `query_added/changed`.
- Atlas sprite path is `Arc`-cloned (refcount bump) not string-copied; glyphon shaped-buffer cache for
  static text; bloom `texel_size` uniform; light cull measured from viewport center + frustum prefilter.
- Inspector component list no longer full-RON-serializes every component each frame; pathfinding
  overlay snapshots tiles instead of cloning whole `Tilemap`s.

### Build / CI

- `serde_json` moved to `[dev-dependencies]` (the lib never used it — examples/tests only).
- wasm CI job gains a clippy pass (`--target wasm32 --lib -D warnings`).

### Notes

- Iterations 1–4 of the batch; **603 → 698 lib tests (+95)** plus 33 new integration tests. Full
  Gate6 green (fmt, clippy `--all-targets -D warnings`, wasm lib+bins build, `test --all-targets`,
  doc `-D warnings`).
- `#9` hot-reload fork-friendliness shipped as a `macro_rules!` dedup (a full `HotReloadable` trait is
  a planned follow-up).

## 8.27.0

Editor timeline editor (MVP) + `Track` keyframe inspection/edit API. Additive.

### Added

- `Track<T>` keyframe accessors/edit ops — `keyframes()`, `len()`, `remove(index)`,
  `set_time(index, time)` (re-sorts), `clear()` — alongside the existing `add`/`sample`/`duration`.
- Editor **Timeline** inspector panel (entities with a `Timeline`): playback controls (duration, loop,
  play/pause, restart, time scrub) plus a per-track keyframe list (position / rotation / scale / color
  / alpha / zoom) showing each keyframe's editable time, value summary, and easing, with per-keyframe
  remove. Exercised by the `timeline_cutscene` example (F2 → select the animated entity).

### Notes

- List-based MVP; a visual track/keyframe timeline (horizontal time ruler, draggable keyframe dots) is
  a planned follow-up. Edit ops validated by unit tests through the real `Track` data model
  (autonomous visual validation is weak under the docked cursor-freeze).

## 8.26.0

Editor state-machine editor (MVP) + `AnimationStateMachine` inspection/edit API. Additive.

### Added

- `AnimationStateMachine` inspection accessors — `state_names()`, `state(name)`, `state_count()`,
  `param_names()`, `param(name)` — and edit operations — `set_current_state()`, `set_state_clip()`,
  `remove_state()` (prunes inbound transitions; refuses the active/last state), `remove_transition()`.
- Editor **State Machine** inspector panel (entities with an `AnimationStateMachine`): lists states
  (current highlighted) with their transitions (target + condition summary + crossfade) and parameters,
  and offers edits — set current, edit clip index, remove state/transition, add state. Exercised by the
  `sm_crossfade` example (F2 → select the animated entity).

### Notes

- This is the list-based MVP; a visual node-graph rendering (positioned nodes + drawn edges) is a
  planned follow-up. Edit operations are validated by unit tests through the real data model
  (autonomous visual validation is weak under the docked cursor-freeze).

## 8.25.0

RTL text support: multi-font loading + reading-direction alignment. Additive.

### Added

- `ExtraFonts(Vec<Vec<u8>>)` resource — additional font blobs loaded alongside `FontData` for
  multi-script coverage (e.g. a Latin UI font + an RTL-script font). cosmic-text falls back across all
  loaded fonts by script, so a single `DrawText` mixing Latin + Hebrew/Arabic shapes correctly.
- `TextAlign::Auto` (no explicit alignment — cosmic-text aligns each line by its resolved direction,
  so RTL text right-aligns automatically) and `TextAlign::End` (reading-direction end: right for LTR,
  left for RTL). Existing `Left`/`Center`/`Right` unchanged.
- Example `rtl_text`: renders mixed Latin + Hebrew (RTL) text using a bundled OFL Noto Sans Hebrew
  font as `ExtraFonts`, demonstrating multi-font fallback + RTL-aware alignment.

### Notes

- Bidirectional/RTL **shaping was already supported** (the text renderer uses `Shaping::Advanced`);
  this release adds the font-coverage + alignment pieces needed to actually use it.
- Bundled `assets/fonts/NotoSansHebrew-Regular.ttf` (+ `NotoSansHebrew-OFL.txt`, SIL Open Font License).

## 8.24.0

Tile collider sync (editor Tile Paint + runtime). Additive.

### Added

- `TilemapColliders` component + `SolidTiles` rule (`NonZero` | `Only(ids)`): opt-in config that keeps a
  tilemap's static physics colliders in sync when the tilemap mutates. Carries the `pixels_per_unit` +
  solid-tile rule the generic sync needs, plus the persistent `TileColliderIndex` for incremental
  resyncs.
- `sync_tilemap_entity_colliders(world, entity)` free fn + `App::sync_tilemap_colliders(entity)` wrapper
  — resync an entity's tile colliders against the `PhysicsWorld` resource (no-op without a `Tilemap` +
  `TilemapColliders` + `PhysicsWorld`).
- The editor's **Tile Paint** now resyncs colliders after each stroke (and on undo/redo) for tilemaps
  that opted in via `TilemapColliders`.

### Changed

- `dig_quest` example refactored onto `TilemapColliders` + `sync_tilemap_entity_colliders`, replacing its
  hand-rolled `TileColliderIndex` field + manual `with_resource_mut`/`sync_static_from_tilemap` dance
  (behavior unchanged). Demonstrates the new API in real play.

## 8.23.0

Editor lighting editor. Additive — editor-internal; no public engine API change.

### Added

- **Point Light** inspector section (shown for entities with a `PointLight`): drag editors for
  color (r/g/b) / radius / intensity / light_height, mutating the component so the lighting pass
  updates next frame, plus a **Reset to Default** button (`App::reset_point_light`). The entity's
  `Transform` position is the light position, so selecting an entity and adding a light places it.
- **Ambient Light** inspector section: edits the global `AmbientLight` resource (color + intensity),
  inserting a default one first if the game never set it (`App::ensure_ambient_light`).
- `PointLight` is now registered as an editor component (Add/Remove buttons + "+ Add" dropdown).

## 8.22.0

Editor particle live-tuner. Additive — editor-internal; no public engine API change.

### Added

- **Particle Tuner** inspector section (shown for entities with a `ParticleEmitter`): drag editors
  for `emit` / `spawn_rate` / `lifetime` / `velocity` / `velocity_spread` / `size` and r/g/b/a drags
  for `color_start` / `color_end`. Edits mutate the component in place, so they take effect live on
  the next spawn while the simulation runs. A **Reset to Default** button restores the default config
  while preserving the assigned texture (`App::reset_particle_emitter`).

## 8.21.0

Editor audio bus mixer panel + `AudioManager::bus_names()`. Additive.

### Added

- `AudioManager::bus_names()` — returns every known bus name (sorted, deduplicated) from the
  channel→bus assignments and the explicit bus-volume map. Lets a UI enumerate all buses.
- **Audio** tab in the editor's bottom panel: one volume slider per audio bus (driven by
  `bus_names()`); dragging applies the new value live via `set_bus_volume`. Shows a hint when no
  `AudioManager` resource is present or no buses are assigned. Native-only, editor-internal.

## 8.20.0

Editor pathfinding-grid overlay. Additive — editor-internal; no public engine API change.

### Added

- **Path** debug overlay (toolbar toggle) in the editor: for each `Tilemap` entity it builds a
  `PathGrid` (the standard "non-zero tile = blocked" convention via `PathGrid::from_tilemap`) and
  shades every cell — blocked cells filled red, walkable cells outlined green — via `DebugDraw`.
  A quick "what would pathfinding navigate here" view. The toggle persists with the other editor
  settings (`#[serde(default)]`, so existing config files still load).

## 8.19.0

Editor debug bounds/colliders overlay. Additive — editor-internal; no public engine API change.

### Added

- **Bounds** debug overlay (toolbar toggle) in the editor: draws every entity's `Transform` AABB and
  any collision `Collider` shape (Aabb → rectangle, Circle → circle) via `DebugDraw` — a quick "where is
  everything / what's collidable" view. The toggle persists with the other editor settings.

## 8.18.0

Editor settings persistence. Additive — editor-internal; no public engine API change.

### Added

- **Editor preferences persist across restarts.** Snap on/off + size, the grid-overlay toggle, and the
  Tile Paint tool + brush size are written to a RON config file when the docked editor closes (F2) and
  restored the first time it opens. A toolbar **💾 Set.** button saves on demand. `PaintTool` gains
  serde derives so it round-trips.

## 8.17.0

Prefab create/instancing in the editor. Additive — editor-internal; no public engine API change.

### Added

- **Prefab** section in the docked editor inspector: **Save Selected** writes the selected entity
  (tag/transform/sprite/parent + serde-registered components, via `entity_to_def`) to a prefab RON
  file; **Spawn** loads a prefab from the path and instances it (with a `PrefabInstance` marker, so the
  existing **Break Prefab** works), selecting the new entity. A path field + status line drive it.

## 8.16.0

Rotation gizmo. Additive — editor-internal; no public engine API change.

### Added

- **Rotation handle** on the world-sprite gizmo. A green handle above the selected entity's top
  edge; dragging it rotates the entity (`Transform.rotation`) to follow the cursor. Completes the
  gizmo (move + 8-handle resize + rotate). With the **Snap** toggle on, rotation snaps to 15°
  increments. Each rotation is one undoable `EditorCmd::RotateEntity` (Ctrl+Z reverts).

## 8.15.0

Editor grid overlay. Additive — editor-internal; no public engine API change.

### Added

- **Grid overlay** in the F2 docked viewport (toolbar **Grid** toggle). Draws world-aligned grid
  lines at the editor snap spacing as an egui overlay on top of the game image — it reads the `Camera`
  to map world↔screen and does not touch the camera or game systems. Lines are skipped when the cells
  would be denser than a few pixels (zoomed out). A live **cursor readout** shows the world `(x, y)`
  under the pointer, plus the hovered `(row, col)` when a `Tilemap` is selected.

## 8.14.0

Inspector quality-of-life. Additive — editor-internal; no public engine API change.

### Added

- **Component copy/paste** in the docked editor's inspector. Each serde-registered component on the
  selected entity gets a **⧉ copy** button; a **Paste {type}** button then applies the copied
  component to the selected entity (insert or overwrite). Useful for transferring a tuned component
  (stats, sprite, …) between entities. Like Add/Remove-component, paste is not pushed to undo history.
- **Entity-list search** — a search box filters the left entity list by label (case-insensitive
  substring); the ✕ clears it.

## 8.13.0

Tile Paint tools. Additive — editor-internal; no public engine API change.

### Added

- **Paint tools for the docked editor's Tile Paint:**
  - **Brush** (freehand) with a selectable **N×N size** (1 / 3 / 5) — paints a block per hovered cell.
  - **Rectangle** — press-drag-release fills the rectangle spanned by the two cells.
  - **Bucket** — a click flood-fills the 4-connected region of same-valued cells.
  - **Eyedropper** — Alt+click picks the hovered cell's value into the paint value (works with any tool).
  Right-click still erases (value 0); every gesture commits as one `PaintTiles` command, so a single
  Ctrl+Z reverts the whole area. Tool + brush size are chosen in the Tile Paint inspector section.

## 8.12.0

Tile Paint swatch palette. Additive — an editor UX upgrade; no public engine API change.

### Added

- **Image-swatch palette for Tile Paint.** The F2 docked editor's **Tile Paint** section now
  renders each paintable tile as a real thumbnail of the selected tilemap's atlas (clickable
  `egui::Button::image` swatches with per-tile UVs from `TilemapAtlas::uv_for`) instead of numbered
  buttons. Clicking a swatch sets the paint value; the current value is highlighted; the "Erase"
  button is kept. Falls back to numbered buttons on the first frame before the atlas texture is
  registered. The atlas texture is registered with egui (`register_native_texture`) before the UI
  pass and freed when paint mode exits or the selection changes, so there is no texture leak.
- **`SpriteRenderer::texture_view(path) -> Option<&wgpu::TextureView>`** — borrow a cached
  image/atlas texture view by asset path (used by the editor to hand an atlas to egui).

## 8.11.0

Editor tile-painting. Additive — a new in-editor authoring tool; no public engine API change.

### Added

- **In-editor tile painting.** In the F2 docked editor, selecting an entity that carries a
  `Tilemap` component now shows a **Tile Paint** section in the inspector. Toggle **Paint mode**
  and paint directly in the viewport: **left-click/drag** paints the selected tile value,
  **right-click/drag** erases (value `0`), number keys **1–9** pick the paint value (**0** = erase,
  clamped to the atlas tile count). Painting reuses `Tilemap::cell_at_world` + `set_tile`, so the
  reactive `TilemapSystem` reflects each change the next frame. While paint mode is on, the
  move/resize gizmo is suppressed so clicks never drag the tilemap.
- **Stroke-level undo.** Each press→release stroke is recorded as a single `PaintTiles` editor
  command; one **Ctrl+Z** reverts the entire stroke (redo re-applies it).
- **Example `tile_paint`** — a blank 20×15 tilemap with a runtime-generated 4-colour atlas; the
  acceptance test for painting in the docked editor.

> **Note:** editor painting is **visual-only** — it does not sync tile colliders. Keep physics in
> step yourself with `PhysicsWorld::sync_static_from_tilemap` if the painted map is collidable.
> The feature is native-only (the docked-editor gizmo path is native).

## 8.10.0

### Fixed

- **`DataTable` file hot-reload now works for relatively-loaded tables.** `DataTableRegistry::reload_path`
  compared the raw stored path against the **canonical** path `AssetServer::poll_reloads` reports
  (`asset_key` canonicalizes), so a table loaded via a relative path silently never hot-reloaded from
  disk. It now matches by canonicalized path (the same approach the data-driven animation registry
  uses). Surfaced while fixing the equivalent bug in the new particle-config registry (8.9.0).

## 8.9.0

Data-driven particle emitter configs. Additive.

### Added

- **`ParticleConfigSet`** — load named `ParticleEmitter` configs from a RON file
  (`(emitters: { "fire": (spawn_rate, lifetime, velocity, velocity_spread, color_start,
  color_end, size, …), … })`); `Vec2` as `(x, y)`, `Color` as `(r, g, b, a)`, missing fields use
  serde defaults. `from_ron_str`, `emitter(name) -> Option<ParticleEmitter>` (a fresh emitter),
  `names()` (deterministic, alphabetical).
- **`App::load_particle_configs(name, path)`** + **`ParticleConfigRegistry`** — registry resource
  (survives scene reset) with **hot-reload** via the `AssetServer` watcher, mirroring
  `load_data_table` / `load_animation_clips`. `ParticleConfigError` for parse/IO. File I/O wasm-gated.
- **Example `data_particles`** (+ a `particles.ron`) — emitters defined entirely in RON; switch
  emitters by name and edit the RON to hot-reload the effect live.

## 8.8.0

Data-driven animation clips. Additive.

### Added

- **`AnimationClipSet`** — load named `AnimationClip`s from a RON file
  (`(atlas: (columns, rows), clips: { "idle": (frames: [0,1,2,1], fps: 6.0, looping: true), … })`);
  frame *indices* are resolved to `UvRect`s via the atlas grid. Clips are ordered alphabetically
  by name (deterministic `index`/`clips`). `from_ron_str`, `clips()`, `index(name)`, `clip(name)`,
  `names()`. Build a player with `AnimationPlayer::new(set.clips().to_vec())` and drive it via
  `player.play(set.index("idle").unwrap())`.
- **`App::load_animation_clips(name, path)`** + **`AnimationClipRegistry`** — registry resource
  (survives scene reset via `register_persistent`) with **hot-reload** wired through the
  `AssetServer` file watcher, mirroring `load_data_table`: editing the RON updates the clips live.
  `ClipSetError` for parse/IO failures.
- **Example `data_anim`** (+ `gen_anim_sheet`) — a sprite animated entirely from a RON clip set;
  switch clips by name; edit the RON to hot-reload the animation.

## 8.7.0

Multi-terrain autotiling. Additive — single-terrain `TilemapAutotile` is unchanged.

### Added

- **`MultiTerrainAutotile`** — a tilemap component (attach instead of `TilemapAutotile`)
  where each non-zero cell autotiles using the [`TerrainRule`] whose `terrain` equals the
  cell's value, connecting only to **same-value** neighbors. So distinct terrains
  (grass/water/sand) each border-tile independently. `edge_16(&[(terrain, base_id), …])`
  builds one identity edge-16 rule per terrain; `with_oob_filled`. Takes precedence over
  `TilemapAutotile`; reuses the reactive `TilemapSystem`'s 8-neighbor UV propagation.
- **`compute_tile_mask_typed(tiles, row, col, nb, oob_filled, terrain)`** — `compute_tile_mask`
  with same-terrain connectivity (a neighbor counts only when its value equals `terrain`).
- **Example `multi_terrain_game`** (+ `gen_multiterrain_sheet`) — grass/water/sand map; paint
  cells with `1`/`2`/`3` (`set_tile`) and watch every terrain re-border live.

## 8.6.0

Versioned save migration. Additive.

### Added

- **`SaveMigrator`** — a chain of schema migrations: `step(n, |value| …)` registers the
  upgrade from version `n` to `n+1`, transforming the decoded `ron::Value`; `current_version()`
  = the number of steps.
- **`save_versioned(path, version, &T)`** — writes an AEAD envelope `{ version, data }` (the
  payload as a `ron::Value`).
- **`load_migrated::<T>(path, &migrator)`** — reads the envelope, applies `steps[stored..current]`
  in order, then deserializes via `ron::Value::into_rust` (bypassing the RON map-vs-struct
  string round-trip). A save tagged newer than the migrator knows → `SaveError::Unsupported`.
- **Example `save_migration`** — writes a v1 save, loads + migrates it to a v2 schema (adds a
  defaulted field), and shows the migrated result on screen.

## 8.5.0

Diagonal (8-direction) pathfinding. Additive — `find_path` is unchanged.

### Added

- **`find_path_diagonal(grid, start, goal)`** — A* on an 8-connected grid (cardinal
  cost 10, diagonal 14, admissible octile heuristic `10·(dx+dy) − 6·min(dx,dy)`). **No
  corner cutting**: a diagonal step is allowed only when both orthogonally-adjacent cells
  are walkable, so paths never slip through the gap between two wall corners. Same endpoint
  convention as `find_path` (excludes start, includes goal; `start==goal` → single cell).
- **Example `diagonal_pathing`** — a grid with a staircase wall barrier; `T` toggles 4-dir
  vs 8-dir and recomputes, so the cardinal zig-zag vs the diagonal shortcut is visible.

## 8.4.0

Audio bus **ducking + sidechain** mixing (native-only audio module). Additive.

### Added

- **Bus ducking** — `AudioManager::duck_bus(bus, gain, attack_secs)` / `release_bus(bus,
  release_secs)` / `bus_duck(bus) -> f32`. A duck is a per-bus gain multiplier (1.0 = none)
  with an attack/release envelope that rides on top of the bus volume, so it never clobbers
  `set_bus_volume`. Driven by `AudioManager::update(dt)`.
- **Sidechain** — `set_sidechain(trigger_bus, ducked_bus, gain, attack_secs, release_secs)` /
  `clear_sidechain(ducked_bus)`. Automatically ducks `ducked_bus` while any channel on
  `trigger_bus` is playing, then releases — the classic "music ducks under dialogue".
  `BusDuck` / `Sidechain` state types re-exported.
- **Example `audio_ducking`** — synthesized music + voice tones; Space plays a voice blip
  that sidechain-ducks the music bus; live on-screen `bus_duck("music")` readout (color-coded)
  makes the duck visually verifiable.

## 8.3.0

Two ergonomic helpers surfaced by the `dig_quest` example (tilemap arc). Additive.

### Added

- **`World::with_resource_mut::<R, _>(|r, world| …)`** — temporarily removes resource `R`,
  runs the closure with `&mut R` **and** `&mut World` at once (the common "I need this
  resource and the rest of the world" borrow), then re-inserts `R`; returns `false` if `R`
  is absent. Replaces the manual `remove_resource` / `insert_resource` dance.
- **`CharacterController::top_down()`** — a constructor for top-down games: like `new()` but
  with snap-to-ground and autostep disabled (the `new()` defaults are platformer-tuned and
  make a top-down character stick to wall surfaces). `slide` stays on.

### Changed

- `dig_quest` refactored onto both helpers (its two `remove_resource::<PhysicsWorld>()`
  sites and the player controller) — the validation that the new APIs read cleanly.

## 8.2.0

Runtime tilemap mutation + neighbor-bitmask autotiling, validated by the new `dig_quest`
example (a destructible-terrain top-down miner). All additive — no breaking changes.

### Added

- **`Tilemap` runtime mutation** — `set_tile` / `get_tile` / `dims` / `cell_center_world`
  / `cell_at_world`. `TilemapSystem` is now **reactive**: it diffs a per-entity cached grid
  and spawns / despawns / updates only the changed cells' tile sprites (a tilemap that never
  mutates renders exactly as before).
- **Autotiling** — the `TilemapAutotile` component (attach to the tilemap entity) selects
  each tile's display UV from its filled neighbors. `Neighborhood::Edge4` (16-tile) and
  `Blob8` (canonical 47-blob); `TilemapAutotile::edge_16` / `blob_47` rulesets, the
  `with_oob_filled(bool)` builder, and the pure `compute_tile_mask`. A changed cell also
  refreshes its 8 neighbors' UVs, so dug holes keep continuous outlines.
- **Incremental tile colliders** — `TileColliderIndex` +
  `PhysicsWorld::sync_static_from_tilemap` diff against the index and add / remove only the
  changed cells (reusing `remove_body`). Use it for the **initial** build too (empty index =
  full build); do not mix with `add_static_from_tilemap` on the same tiles (that would
  double-add colliders so a dug cell never frees). `add_static_from_tilemap` is unchanged for
  static maps.
- **Example `dig_quest_game`** (`examples/games/dig_quest/`) + the `gen_autotile_sheet`
  deterministic asset generator. Native playtest confirmed: digging updates the autotile
  outline + frees collision (the player enters), reset restores, post-reset re-dig works.

## 8.1.10

Deferred-item cleanup from the engine-wide review (asset hot-reload + scripting scope).
No public API change.

### Fixed

- **Atlas file changes are recognized by hot-reload.** `poll_reloads` checked image /
  script / data-table path maps but not `atlas_path_to_id`, so an atlas path was never
  treated as "known" (the underlying image pixels still reloaded via the inner
  `load_image`; this makes the path recognition self-consistent).
- **A failed image load no longer registers a dead file-watcher.** `load_image` watched
  the path even on a failed load; `notify` cannot watch a non-existent path, so a later
  file-create never fired. The watch is now registered only for successfully-loaded paths.
- **Rhai `ScriptRunner` scope no longer grows across frames.** The persistent per-entity
  `Scope` is rewound to its 5-var transform baseline (`x`/`y`/`rot`/`sx`/`sy`) after each
  `on_update`, so `let` bindings introduced per frame don't accumulate. The script scope
  is a transform transport, not a store for cross-frame custom state.

(Not changed: `with_ctx`/`with_ctx_mut` keep their `expect` — calling a Rhai API function
outside `ScriptingSystem::run` is a documented contract violation; a graceful path would
require a sprawling `R: Default` refactor across all script API functions.)

## 8.1.9

Bug fixes: surface-error handling. Final batch of a second-pass engine-wide review (app
main-loop / window / render orchestration + concurrency / WASM / panic-safety — the latter
entirely clean). No public API change.

### Fixed

- **A minimized/occluded window no longer spams `log::error!` every frame.** The surface
  acquisition's `Occluded` and `Timeout` results fell through to an `error!` log, firing
  once per frame while minimized. They are now skipped silently (`Lost`/`Outdated` still
  reconfigure; genuine errors like `Validation` still log).
- **A `Suboptimal` surface is now reconfigured.** After a DPI/monitor/rotation change the
  acquired `SurfaceTexture` can be flagged suboptimal; the frame is presented and the
  surface is then reconfigured so subsequent frames are optimal (was previously ignored,
  causing persistent degradation on some platforms).

## 8.1.8

Bug fixes: UI click/slider/scroll edge cases, save-path hardening, timeline loop wrap.
Final batch of an engine-wide review sweep (UI / asset-save-scripting / timeline-tween-
network — otherwise clean). No public API change.

### Fixed

- **Overlapping `Button`s no longer both fire on one click.** The button pass fired
  `ButtonClicked` for every button whose hit-test passed, so stacked buttons all fired.
  Only the top-most (highest `z`) clicked button now fires. (Cross-widget pointer
  consumption — a button beneath a different widget type — is left as a future TODO.)
- **`ScrollView` with `item_height == 0` no longer panics.** `size.y / 0.0 → inf`,
  `inf.ceil() as usize → usize::MAX`, `+ 1` overflowed (debug panic). Zero/negative item
  height is now guarded.
- **`Slider` emits exactly one `SliderChanged` on the press frame.** The press and the
  same-frame drag-recalculation both fired, producing two events with different values;
  the drag path is now skipped on the press frame.
- **`save_path` rejects path traversal.** `app_name`/`file` are sanitized (only `Normal`
  path components kept), so `"../../etc/passwd"` can no longer escape the data directory;
  legitimate sub-directories (e.g. `"saves/slot1.sav"`) are preserved.
- **Looping `Timeline` wraps with modulo.** A `dt` larger than the timeline `duration`
  (e.g. resuming after a stall) used a single subtract, leaving `time` past the end for
  several frames (stutter). It now wraps with `%` in one frame (guarded against
  `duration == 0`).

## 8.1.7

Bug fixes: audio bus-volume during fades, behavior-tree `AlwaysSucceed`, tilemap tile-id
bounds. Found by an engine-wide review sweep. No public API change.

### Fixed

- **Bus volume is no longer applied twice during audio fades.** A fade stored its start
  volume as `base × bus`, and `update()` multiplied by the bus volume again, so the sink
  got `base × bus²` — an audible volume pop at fade start and a fade at the wrong rate
  (only when a bus had volume ≠ 1.0). Fades now store/interpolate the pre-bus base volume
  and the bus factor is applied exactly once in `update()`.
- **`AlwaysSucceed` behavior-tree decorator passes `Running` through.** It discarded the
  child's status and always returned `Success`, so wrapping a multi-frame action made the
  parent `Sequence`/`Selector` advance on frame 1 and abandon the still-running child. It
  now returns `Running` while the child runs and only converts `Failure → Success`.
- **`TilemapAtlas::uv_for` clamps out-of-range tile ids.** A tile id ≥ `columns × rows`
  produced a UV rect outside `[0,1]`, sampling garbage/wrong tiles. Out-of-range ids now
  return `UvRect::FULL` instead.

## 8.1.6

Bug fixes: physics collision-event delivery + raycast freshness, animation clip-finish +
blend-tree state. Found by an engine-wide review sweep (physics/collision + animation/
skeletal — both otherwise clean). No public API change.

### Fixed

- **`CollisionEvent::Stopped` is delivered when a contacting entity despawns.** The
  handle→entity map was rebuilt each frame from live entities, so an entity removed while
  still touching another resolved to nothing and its `Stopped` exit event was silently
  dropped (listeners waiting for "no longer touching" never fired). The system now keeps
  the previous frame's map and falls back to it when resolving stopped pairs.
- **`cast_ray` no longer hits a just-removed body in the same frame.** The query pipeline
  was only refreshed inside `step()`, so a raycast issued after `remove_body` but before
  the next step saw a phantom collider. `remove_body` now refreshes the query pipeline
  immediately. (`cast_ray`/`cast_ray_with_normal` remain `&self` — no API change.)
- **A 1-frame non-looping clip is no longer reported finished before it is shown.**
  `is_finished()` returned `current_frame >= len-1`, which is `0 >= 0` (true) at entry for
  a 1-frame clip, so an `AnimationEnd` state-machine state transitioned away the same frame
  it was entered. The player now tracks a `finished` flag set when the advance actually
  reaches past the last frame.
- **BlendTree1D no longer gets stuck after a parameter reversal during a crossfade.** If
  `param` returned to the FROM clip's range mid-crossfade, `last_clip` was poisoned and the
  dedup skipped all later transitions, leaving the player stuck on the crossfade target.
  The "already on target" branch is now guarded by `!is_crossfading()`.

## 8.1.5

Bug fixes: scene-stack panic recovery + centered-text wrapping. Found by an engine-wide
review sweep (core ECS + rendering — both otherwise clean). No public API change.

### Fixed

- **`SceneCmd::Pop` no longer permanently silences the builtin tail system.** If a
  `Push`ed scene's first system panicked (added to the panic set) and the scene was then
  `Pop`ped, the retained panic index aliased `HierarchySystem`'s post-drain index and
  skipped it forever — parent-child `GlobalTransform` propagation silently stopped. The
  retain bound is now `new_scene_len` (drops drained + tail indices; the tail gets a
  clean retry, consistent with `reload_scene`).
- **`DrawText::centered` no longer wraps at half the viewport width.** With no explicit
  bounds, the layout buffer width was `viewport_w - position.x`; for a `Center`-anchored
  text positioned at the screen center that is only half the width, so a one-line title
  wrapped to two. `Center` anchor with no bounds now uses the full viewport width/height
  (top-left and explicit-bounds paths unchanged). Width/height selection factored into
  tested pure helpers.

## 8.1.4

Bug fixes: docked-editor gizmo + Inspector edge cases (follow-up to 8.1.3, found by a
second review sweep). No public API change.

### Fixed

- **Resizing a non-`TopLeft`-anchored `UiNode` no longer slides the widget.**
  `UiNode::screen_pos` is `anchor_base(anchor, size) + offset`, and for `Center`/
  `Bottom*`/`*Right` anchors the base depends on `size`. The gizmo resize math only kept
  the fixed corner stable for `TopLeft`, so resizing a `Center`-anchored widget (e.g. the
  `ui_layout_editor_game` menu buttons) drifted on screen. `ui_resize_new_layout` now
  applies an anchor-base compensation so the fixed corner stays put for every anchor
  (`TopLeft` behaviour is unchanged — its base is constant). A shared `anchor_base` helper
  is now the single source for both `screen_pos` and the gizmo.
- **Inspector field edits are no longer dropped when the archetype/selection changes
  mid-frame.** The write-back paired staged values to components by positional index, so
  adding/removing a component or an Undo/Redo that reselected a different entity in the
  same frame mis-paired them (silent edit loss). Write-back is now matched by component
  name and guarded to the entity the values were captured for.
- **Docked viewport mouse-release no longer double-fires.** On release inside the viewport
  the input release ran twice; a release with no matching press could also be produced
  when the pointer was outside. The stuck-state-clearing release now runs only when the
  primary (in-viewport) release path did not.
- **Undo/Duplicate/Paste of a child entity preserves its parent link.** `entity_to_def`
  hard-coded `parent: None`, so restoring a deleted (or duplicating/pasting a) child
  re-spawned it as a root, losing the hierarchy. It now resolves the entity's `Parent`
  to the parent's `Tag`, matching scene-save.

## 8.1.3

Bug fixes: docked-editor reliability (Undo/Redo, Load/Save, Data Tables). No API change
to the public surface; `DataTableRegistry::reload_path` now returns a `ReloadOutcome`
(was `()`).

### Fixed

- **Undo of Delete restores the whole entity.** `EditorCmd::DeleteEntity` captured only
  tag/transform/sprite, so undoing a delete dropped every other component — including
  game components registered via `register_editable_component` (e.g. `Stats`). It now
  captures the full `EntityDef` and restores via `spawn_entity_def`, preserving all
  serde-registered components.
- **Duplicate and Paste are now undoable.** The `⎘ Duplicate` button and Ctrl+V paste
  spawned entities without recording an undo step, so Ctrl+Z did nothing. They now push a
  `CreateEntity` command carrying the entity's `EntityDef`, so Undo removes the copy and
  Redo restores it with all its components.
- **Load Scene fully clears the previous scene.** `do_load_scene` despawned only
  `Transform`-bearing entities, leaving `UiNode`-only entities (menus/HUD) behind and
  duplicating them on load. It now despawns all entities before loading.
- **Data Tables "Reload" reports accurately.** `reload_path` skips reloading a table with
  unsaved edits (dirty-guard); the panel previously still showed "reloaded". It now
  reports the real outcome ("skipped reload — unsaved edits") via the new
  `ReloadOutcome` return value.
- **Save Scene no longer silently drops untagged-parent links.** When a child's parent
  entity has no `Tag` (unrepresentable in `EntityDef.parent`, which is tag-based), the
  link was dropped silently; Save now logs a warning and notes the count of dropped
  parent links in the save-status message.

## 8.1.2

Bug fix: the game-data editor (v8.1.0) is now functional under its documented usage.
No API change.

### Fixed

- **Game-side component registrations and data tables survive `set_scene`.**
  `App::set_scene` resets the `World` (via `SceneCmd::Replace` → `reload_scene`), which
  previously discarded everything registered *before* the first scene was set. With the
  documented pattern —
  ```rust
  app.register_editable_component::<Stats>("Stats", None);
  app.load_data_table("enemies", "enemies.ron");
  app.set_scene(Box::new(GameScene::new()));
  ```
  — the `Stats` reflect/clone/serde registrations and the loaded `DataTableRegistry`
  were silently lost, so `Stats` never appeared in the Inspector, was omitted from saved
  scene RON, and the Data Tables panel was empty. `App` now records these registrations
  and **replays them on every world reset** (mirroring the existing `event_initializers`
  mechanism), and `load_data_table` marks the `DataTableRegistry` persistent. The
  `stat_editor_game` example now works end-to-end (Inspector edit → Save → reload; live
  Data Tables). Built-in components (Transform/Sprite/Tag/UI widgets) were unaffected
  because they are re-registered by `insert_core_resources` each reset.
  - Internal: `SerdeComponentEntry.post_spawn` is now stored as `Arc` (was `Box`) so the
    registration can be replayed. No public signature change.

## 8.1.1

Event-loop responsiveness on macOS (no API change).

### Changed

- The native event loop now uses **`ControlFlow::WaitUntil` frame pacing** instead of
  a `ControlFlow::Poll` busy-spin: it sleeps between frames (requesting a redraw at the
  monitor refresh cadence, clamped to 60–240 Hz) so the macOS main run loop gets idle
  time — smoother window drag/resize and lower idle CPU/battery — while still rendering
  continuously (input events wake the loop immediately). This resolves the macOS
  event-loop-stall TODO previously noted in the surface config. wasm is unchanged
  (`Poll` maps to `requestAnimationFrame`).
- **`desired_maximum_frame_latency` 1 → 2**: lets the GPU keep ~1 frame queued so
  `get_current_texture()` no longer blocks the main thread on vsync for most of each
  frame.

> Note: the dominant factor in editor/game click responsiveness is **build profile** —
> run interactive testing with `--release`; debug builds spend far more per-frame CPU
> and feel laggy regardless of event-loop pacing.

## 8.1.0

Game-data editor: edit component stats and RON data tables in the docked editor
and persist them to disk. Third release of the in-engine editor arc (scene layout
shipped in 8.0.0). Fully additive — no migration needed.

### Added

- **`#[derive(Reflect)]`**: a proc-macro (new workspace crate
  `engine_reflect_derive`) that generates the `Reflect` impl for a struct of
  `f32`/`i32`/`Vec2`/`bool`/`String`/`Color`/`[f32; 4]` fields. `#[reflect(skip)]`
  omits a field; unsupported types fail with a clear compile error. Hand-written
  `Reflect` impls keep working. Add the crate to your `Cargo.toml` (the same way
  you add `engine`) and write `use engine_reflect_derive::Reflect;` then
  `#[derive(Reflect)]`. The macro is a separate crate rather than re-exported from
  `engine` so that `skeleton-engine` stays publishable without first publishing the
  proc-macro to crates.io.
- **`App::register_editable_component::<T>(name, post_spawn)`**: one call wires a
  component for full editor integration — Inspector field editing (Reflect), entity
  duplication (Clone), scene save/load (serde), and the Add/Remove Component
  buttons. `T: Reflect + Serialize + Deserialize + Clone + Default`.
- **Data tables** (`DataTable`, `DataTableRegistry`, `App::load_data_table`): load a
  schema-agnostic RON table (a sequence of `(col: value, …)` rows), read it as a
  World resource at runtime, edit it in the editor's new **Data Tables** tab (bottom
  panel) — per-cell number/string/bool editors, add/delete row, Save — and
  hot-reload disk changes into the running game (a dirty-guard protects unsaved
  edits). Native-only panel; the types are cross-platform.
- **`stat_editor` example** (`cargo run --example stat_editor_game`): entities with
  a derived `Stats` component seeded from an `enemies` data table; edit stats in the
  Inspector (live HUD updates) and tune `enemies`/`items` tables in the Data Tables
  panel — the game-data-editing acceptance test.

### Changed

- The crate is now a **Cargo workspace** (members `.` and `engine_reflect_derive`).
  Consumers that depend on `skeleton-engine` by path or git are unaffected (the
  package name and layout are unchanged); the proc-macro crate is host-compiled and
  does not affect the wasm target.

## 8.0.0

Scene layout editing: the docked editor (v7.1.0) can now select, move, and resize
UI widgets in the viewport and **persist them to a scene file**. Second release of
the in-engine editor arc (next: a game-data / stat-table editor). Breaking because
the scene file format and `EntityDef` shape changed; migration is mechanical.

### Added

- **serde + Reflect on every UI widget** (`UiNode`, `Button`, `Label`, `TextInput`,
  `Slider`, `CheckBox`, `ScrollView`, `Panel`, `LocalizedText`, plus `Anchor`,
  `TextAlign`, `LayoutDir`): widgets now serialize into scene RON and appear/edit
  in the F1/F2 editor Inspector. Runtime state (`ButtonState`, slider/text-input
  cursor & value, scroll offset, `Panel.children`) is `#[serde(skip)]`.
- **Component serialization registry**: `App::register_serde_component::<T>(name,
  post_spawn)` registers any `Serialize + DeserializeOwned + Clone` component so it
  is saved into / loaded from scene files. All UI widgets are auto-registered;
  games register their own types (e.g. stats) the same way. Backed by the
  `SerdeComponentRegistry` resource. Unregistered component names in a loaded file
  warn and are skipped (load never fails).
- **Screen-space UI gizmo**: select a `UiNode` widget to drag it (offset) and
  resize it via 8 handles in the docked/overlay viewport; world sprites gained
  8-handle scale resize (center-fixed). New undo entries
  `EditorCmd::{MoveUiNode, ResizeUiNode, ResizeEntity}` (Ctrl+Z).
- **`ui_layout_editor` example** (`cargo run --example ui_layout_editor_game`):
  load-or-default menu; arrange/resize widgets in the editor, click Save Scene,
  restart, and the edited layout loads — the scene-layout-editing acceptance test.

### Breaking

- **`SceneDef` version 2 → 3.** v2 files still load (the new `components` field
  defaults to empty; the existing version-mismatch warning is informational). v3
  files cannot be read by v7 engines.
- **`EntityDef` gains `components: HashMap<String, ron::Value>`.** Code that
  constructs `EntityDef { .. }` with explicit fields must add
  `components: Default::default()` (or use `..Default::default()`).
- **`TextInput` gains `initial_text: String`; `text` and other runtime fields are
  now `#[serde(skip)]`.** Set `initial_text` for design-time content; the registry
  post-spawn hook copies it into `text` on load. **`Slider` gains
  `initial_value: f32`; `value` is `#[serde(skip)]`** (same pattern). Constructors
  (`Slider::new`) are unchanged at runtime.
- Components are stored in scene RON as a string-encoded `ron::Value` (ron 0.8's
  `Value` cannot round-trip enums like `Anchor`); this is an internal format detail
  but visible in saved files.

## 7.1.0

The docked editor shell: a second editor mode that lays the screen out like a
commercial engine — side panels around a central game viewport — so editing no
longer covers the game. First release of the in-engine editor arc (next: UI
widget editing, then data tables). No breaking changes.

### Added

- **Docked editor mode (`F2`, native only)**: egui owns the window; the left
  panel holds Entities/Scene tabs, the right panel the Inspector, the bottom
  panel Assets, and a top toolbar carries play/pause (`▶`/`⏸`), single-frame
  step (`⏭`), snap controls, and scene save/load. The game renders into an
  editor-owned offscreen texture shown in the central panel (size follows the
  panel, 3-frame resize debounce). `F1` keeps the existing floating-window
  overlay unchanged; the modes are mutually exclusive.
- **Viewport-local input routing**: while docked, the game receives the cursor
  translated into viewport coordinates (`viewport_to_game`), and pointer events
  pass through a layer-aware gate (`docked_game_pointer_allowed`) — clicks
  inside the viewport reach the game/gizmo, clicks on panels and popups stay in
  egui, and typing in the Inspector never leaks into game input. The selection
  gizmo (drag to move, snap, undo) works inside the docked viewport.
- **Editor pause**: the toolbar pause skips scene systems at the engine level
  while keeping the builtin tail (`HierarchySystem`) running, so dragging a
  parent while paused still moves its children. `⏭` advances exactly one full
  frame. The `GameState` resource is untouched (it remains a game-side
  convention).
- **`ViewportSize` delegation**: while docked, `ViewportSize` reports the
  central panel's logical size, so cameras, screen-space UI, and
  `Camera::screen_to_world` work unchanged against the viewport; the real
  window size is restored on exit.

## 7.0.0

The renderer-dependency major window: the whole wgpu/glyphon/egui stack moves to
current majors, resolving `RUSTSEC-2026-0002` (glyphon 0.6 pinned `lru` < 0.16.3 —
previously archived as accepted risk in `docs/SECURITY_HARDENING_2026_05.md`, now
closed). Engine-side rendering behavior is preserved exactly (sRGB-first surface
format, AutoVsync, frame latency 1, WebGL2 limits on wasm, egui dithering off);
verified by the full gate suite, `wasm_smoke` (connect + non-blank render, HUD
correct), and windowed playtests (lighting pass, SM crossfade mid-blend, F1
inspector overlay).

### Breaking — toolchain & dependencies

- **MSRV 1.88 → 1.92** (`rust-version = "1.92"`): egui 0.34 requires Rust 1.92,
  cosmic-text 0.18 requires 1.89. CI pins Rust 1.95.0 (current stable, also used
  for local gates).
- **wgpu 22 → 29** (`webgl` feature unchanged), **glyphon 0.6 → 0.11**
  (cosmic-text 0.18), **egui / egui-wgpu / egui-winit 0.29 → 0.34**, winit minimum
  `0.30.13`. Transitive `lru` resolves to 0.16.4, closing `RUSTSEC-2026-0002`.

### Breaking — API changes

- **`GpuContext::clear()`** returns `Result<(), String>` (was
  `Result<(), wgpu::SurfaceError>` — wgpu 29 removed `SurfaceError`; surface
  acquisition reports through the `wgpu::CurrentSurfaceTexture` enum). *Migration:*
  treat the `Err` as an opaque message. The engine main loop handles
  reconfigure-on-`Lost`/`Outdated` internally, exactly as before.
- **`RenderTarget` pub fields and `DebugUi::ctx()` now expose wgpu 29 / egui 0.34
  types** — code touching `RenderTarget.{texture,view,sampler,bind_group}` or
  writing custom egui panels compiles against the new majors. Notable for panel
  code: `Rounding` → `CornerRadius`, `Context::style()` → `global_style()`. egui
  0.34's skrifa font backend renders text slightly differently (default text size
  12.5 → 13.0) — debug-UI only, game rendering unaffected. wgpu resources are now
  `Clone` (internally refcounted); `RenderTarget.bind_group` stays `Arc`-wrapped
  for API stability.

### Fixed

- **egui texture deltas are no longer dropped on skipped frames** — when surface
  acquisition failed for one frame (`Lost`/`Outdated`/`Timeout`, e.g. during a
  live window resize), the unconsumed `textures_delta` was overwritten by the next
  frame. egui 0.29's ab_glyph backend re-sent the full font atlas on every change,
  silently self-healing; egui 0.34's incremental skrifa updates made the latent
  bug fatal (panic on F1: "Tried to update a texture that has not been allocated
  yet"). Deltas now merge old → new (`merge_textures_delta` in
  `src/app/schedule.rs`, +2 regression tests). Found by the windowed playtest.

### Changed

- `src/app/egui_pass.rs` dropped its `unsafe` transmute — wgpu 29's
  `RenderPass::forget_lifetime()` is the supported replacement for the
  egui-wgpu `RenderPass<'static>` requirement.
- egui renderer keeps dithering **off**, matching the pre-0.34 explicit arguments
  (`RendererOptions::default()` would have silently enabled it).

## 6.0.0

The v6 breaking window: the three "Verified-but-deferred" items recorded in 5.1.3,
the v5.0.0 `Arc<str>` conversion completed for particles, and the HierarchySystem
pipeline integration. Every change below lists its migration. The fifth scoped item
(BehaviorSystem take/add archetype migrations) was investigated and **deliberately
kept** — the evaluation is recorded as a PERF comment in `BehaviorSystem::run`.

### Breaking — API changes

- **Animation systems own a scratch buffer** — `AnimationSystem`, `BlendTreeSystem`,
  and `StateMachineSystem` are no longer unit structs (they keep a reused per-frame
  entity buffer, eliminating three per-frame `Vec` allocations). *Migration:*
  construct with `::new()` (or `::default()`):
  `app.add_system(AnimationSystem)` → `app.add_system(AnimationSystem::new())`,
  `Box::new(BlendTreeSystem)` → `BlendTreeSystem::new()`, same for
  `StateMachineSystem`. `LABEL` constants and ordering semantics are unchanged.
- **Allocation-free state-machine parameter setters** —
  `AnimationStateMachine::{set_bool, set_float, add_trigger}` now take
  `impl Into<String> + AsRef<str>` and only allocate on first insert (updates are
  in-place). *Migration:* none for `&str` / `String` / `&String` / `Cow<str>`
  callers — these satisfy both bounds and compile unchanged. Only an exotic type
  implementing `Into<String>` but not `AsRef<str>` needs adapting.
- **`ParticleEmitter.texture` is `Option<Arc<str>>`** — completes the v5.0.0
  `Sprite.texture` conversion (analysis #9); per-spawn clones become refcount bumps.
  *Migration:* `texture: None` and `texture: Some("x.png".into())` compile
  unchanged; `texture: Some(string_var)` becomes `Some(string_var.into())`
  (std provides `From<String> for Arc<str>`). `ParticleEmitter` has no serde derive,
  so no save-format impact.
- **`HierarchySystem` joined the labeled pipeline** — it is registered automatically
  by `App::new()` as a permanent tail built-in (survives `SceneCmd::Replace`) instead
  of being force-run outside the scheduler. *Migration:* none for games that do not
  order around hierarchy propagation — default frame behavior (GlobalTransform
  updated after all user systems, before render) is identical. New capability:
  `.after(HierarchySystem::LABEL)` / `.before(...)` constraints now actually take
  effect (the LABEL previously existed but was a dead symbol). `docs/PATTERNS.md`
  gained the ordering row.

### Changed

- Examples updated to the `::new()` system constructors (sm_crossfade,
  blend_locomotion, platformer).

## 5.1.3

Cleanup batch over the low/leftover findings from the 2026-06-12 full-source review
(report §3 — 16 items locally re-verified first: 9 applied here, 2 refuted, 4 deferred
as breaking-or-architectural, 1 skipped as not worth the churn). Pure internal
refactors and perf fixes — zero public-API change, no migration.

### Performance

- **Particle emitter texture clone removed from the per-frame path** — the
  `Option<String>` texture is now looked up lazily only when particles actually spawn
  (was cloned per emitter per frame regardless of emission).
- **`World::despawn` change-tracking is O(1) per entity** — `added_this_tick` /
  `changed_this_tick` restructured from `HashSet<(Entity, TypeId)>` (full-set `retain`
  per despawn) to `HashMap<Entity, HashSet<TypeId>>`. Mass-despawn sites (tilemap
  teardown, particle bursts, pool clears) no longer scan the whole tracking set per
  entity. Query semantics unchanged.
- **Scripting blackboard snapshot allocates one String per entry instead of two**
  (`bb_snap` now stores keyless `BlackboardValue`s; the write-path `BbEntry` keeps its
  key, which the apply loop needs).

### Internal cleanup

- Sprite/AtlasSprite culling block (4 copies) extracted into one helper; UI widget
  passes' UiNode layout extraction (4 copies) extracted into `node_layout`; the
  `SCRIPT_CTX` access boilerplate (15 copies) extracted into `with_ctx`/`with_ctx_mut`;
  audio fade-start-volume logic (3 copies) unified into one `fade_start_vol` method.
- Editor entity labels standardized to `"Entity {index}:{generation}"` (the entity-list
  panel used a different short form than every other panel).
- Doc notes: hot reloading documented as native-only (silent empty result on `wasm32`),
  matching the lighting platform-note precedent; the physics sensor-pair `ordered_pair`
  normalization now carries a comment recording that it is defensive (verified against
  rapier's stable edge-slot order) so future reviews don't re-investigate.

### Verified-but-deferred (recorded, not fixed)

- Per-frame `Vec<Entity>` scratch in the three animation systems — requires turning pub
  unit structs into field structs (breaking); deferred to the next major.
- `AnimationStateMachine::set_bool`/`set_float` key allocation — needs a signature-bound
  change (breaking risk); deferred.
- `BehaviorSystem` take/add archetype migrations — structural (`tick` needs `&mut World`);
  acceptable at typical AI entity counts, revisit only if profiling demands.

## 5.1.2

Bug-fix batch from the scheduled 2026-06-12 full-source review
(`docs/CODE_ANALYSIS_2026-06-12.md` — Top-10 locally re-verified: 8 confirmed,
2 refuted; the 8 confirmed findings are all addressed here). No migration; one
small API addition noted below.

### Fixed

- **Network receive-queue overflow accounting** (the round's two high findings) —
  `ReceiveQueueFull.dropped` now accumulates across every rejected message (was a
  constant `1`, silently discarding all subsequent overflow), and events already in
  the queue are never evicted once the marker is installed. When the marker is first
  installed it displaces the youngest queued event, which is now *counted*
  (`dropped` starts at 2). Queue length never exceeds the configured capacity.
  Native and wasm paths are semantically identical.
- **Crossfade interrupt pop** — calling `play_with_crossfade` toward a *third* clip
  while a blend is in flight now promotes the in-flight TO side to the new FROM
  (`mix(B, C, 0)` on the first frame) instead of popping back to the original FROM
  clip image. The 5.1.1 same-target idempotency guard is unaffected.
- **Crossfade completion stutter** — completion now carries the to-clip's accumulated
  sub-frame timer into `AnimationPlayer.timer` (with this tick's `dt` counted exactly
  once) instead of resetting to `0.0`, which visibly stretched the first post-blend
  frame on low-fps clips.
- **Silent collision-event drop warning** — `PhysicsSystem` now `log::warn!`s once
  when collisions/triggers occur but `Events<CollisionEvent>` /
  `Events<TriggerEvent>` was never registered, naming the exact
  `register_event` call to add (previously the events vanished with no signal).
- **Per-sprite `Arc<str>` re-allocation** — the renderer's `image_handle` path uses
  the new `Handle::path_arc()` (O(1) refcount bump) instead of `Arc::from(h.path())`
  (per-sprite per-frame string copy).
- **Doc gaps** — `docs/PATTERNS.md` ordering table gains the
  `BlendTreeSystem` before `AnimationSystem` row; `AmbientLight` / `PointLight` /
  `LightingRenderer` doc comments now state the native-only / wasm32-no-op limitation.

### Added

- `Handle::path_arc() -> Arc<str>` — owned handle path without copying the string.

## 5.1.1

Bug-fix batch from the post-release code review of the 5.1.0 features (10 confirmed
findings, three root causes). No migration needed; one small API addition noted below.

### Fixed

- **Audio release envelope redesigned** (root cause: shadow state + stale volume reads).
  `stop()` during *any* in-progress `stop_when_done` fade (release **or** `fade_out`)
  now cuts immediately — `fade_out` is a real bypass path as documented, and a second
  `stop()` mid-release still cuts. The release fade starts from the **current
  interpolated** fade position instead of the stale override (no more start-of-release
  pop). Completed teardown fades no longer persist `0.0` into the channel volume, so
  the next `play_*` on a reused channel starts at the `set_volume` level (regression
  fix). `stop()` on a naturally-drained sink cuts immediately instead of scheduling a
  silent release. Internals: the `releasing` HashSet is gone; `Fade` construction is
  unified (`Fade::stop_fade`) with one consistent minimum-duration rule.
- **State-machine crossfade guards** (root cause: `current_clip` stays on the FROM clip
  during a blend). `AnimationPlayer::play_with_crossfade` re-fired with the same target
  mid-blend is now idempotent — oscillating threshold transitions can no longer reset
  the blend every frame. `StateMachineSystem` evaluates `AnimationEnd` via the new
  `AnimationPlayer::is_clip_finished(clip_index)` (returns true only when not
  crossfading and that clip is the finished current clip), so a crossfaded-into
  one-shot state plays its clip to completion instead of exiting on the first frame.
  The `AnimationStateMachine` ↔ `BlendTree1D` interaction is now documented (SM
  transitions intentionally interrupt an in-progress BT blend; avoid driving the same
  player with both unless that is desired).
- **Script steering commands are mutually exclusive** — `seek_target` / `flee_from` /
  `arrive_at` / `wander` each remove the other three steering components before
  attaching their own (previously a single `wander()` permanently overrode later
  commands via the steering system's last-writer-wins order), and `stop_steering()`
  removes all four so a stopped entity stays stopped. Rust-side multi-component
  steering composition is unaffected.

### Added

- `AnimationPlayer::is_clip_finished(clip_index)` — crossfade-aware finish check used
  by the state machine; public for game code with the same need.

## 5.1.0

The three feature candidates deliberately split out of the 2026-06-10 analysis round,
each validated by a playable example per the `docs/VISION.md` loop. Fully additive —
no migration needed from 5.0.0.

### Added

- **Per-transition crossfade on `AnimationStateMachine`** — `AnimTransition` gains a
  `crossfade_duration: f32` field (default `0.0` = hard switch, the previous behavior)
  and `add_transition_crossfade(from, to, conditions, duration)` registers a transition
  that blends into the target clip. `StateMachineSystem` drives the existing
  `AnimationPlayer::play_with_crossfade` path — the same 2-UV shader-lerp used by
  `BlendTreeSystem`, no new blend machinery. `add_transition` keeps its signature
  (now a thin wrapper with `0.0`). Example: `sm_crossfade` (side-by-side hard-switch
  vs. crossfaded character; run `gen_blend_sheet` first).
- **Rhai steering bindings for `Arrive` / `Wander`** — scripts can now use the full
  steering set (previously only Seek/Flee were bound):
  `arrive_at(tx, ty, speed, slow_radius, stop_radius)` and
  `wander(speed, change_interval)`, following the existing `seek_target`/`flee_from`
  conventions (f64 params, last call per frame wins, `SteeringVelocity` auto-attached).
  The Wander apply step preserves the component's internal timer/direction so per-frame
  script calls don't reset the direction-change rhythm. Example: `script_steering_game`
  (mouse-following Arrive agent + autonomous Wander agent, both script-driven).
- **`AudioEffect::release_secs` implemented** (was a documented no-op stub) —
  `AudioManager::stop` on a channel whose effect has `release_secs > 0.0` now fades the
  volume to zero over that duration through the existing fade machinery, then tears the
  sink down. `0.0` keeps the immediate cut. A second `stop` during the release, or a new
  `play_*` on the channel, cuts immediately. Requires `AudioSystem` (or manual
  `update(dt)`) to progress, like all fades. Example: `audio_fades` (extended — R/S/I
  keys demo release vs. immediate stop).

## 5.0.0

The breaking batch from the 2026-06-10 analysis (`docs/CODE_ANALYSIS_2026-06-10.md`):
Top-10 items #2 and #8, removal of everything deprecated in 4.6.0, the visibility
narrowings triaged out of the 4.6.0 sweep, and small breaking consistency items.
Every change below lists its migration.

### Breaking — removed (all deprecated since 4.6.0)

- **`DebugDrawQueue` / `DebugRect`** — migrate to `DebugDraw::rect_filled_z(min, max, color, z)`
  (or `rect_filled` for z = 0).
- **`World::register_reflect`** — use `register_reflect_named::<T>("Name")` (the removed
  overload stored an empty type name and broke the Inspector display).
- **`NetworkEvent::JsonParseError`** — never emitted by the engine; delete the match arm
  (protocol-level parse errors are the game's concern).
- **`App::load_texture`** — use `load_image` (returns a `Handle<ImageAsset>`, participates
  in hot reload).
- **`ParticleEmitter::for_burst`** — renamed to `ParticleEmitter::burst` in 4.6.0.
- **Pre-v5 re-export shims** — `animation::player::{UvRect, BlendUv}` → `renderer::uv`,
  `timeline::Lerp` → `tween::Lerp`, `prefab::topological_sort_entities` → `hierarchy`,
  and the `components::*` migration facade (`AnimationClip`, `AnimationPlayer`, `UvRect`,
  `FontData`, `GameState`, `PendingResize`, `ShouldQuit`, `ViewportSize`, `WindowConfig`).
  All root re-exports (`engine::UvRect`, `engine::Lerp`, `engine::topological_sort_entities`, …)
  keep working — only the deep legacy paths are gone.

### Breaking — API changes

- **Physics handle newtypes (analysis #2)** — `PhysicsWorld` no longer leaks rapier types:
  new `BodyHandle` / `ColliderHandle` newtypes (mirroring `JointHandle`) flow through every
  factory return, `PhysicsBody`'s fields, `RaycastHit.collider_handle`, raycasts, joints,
  `move_character`, and the collider accessors. *Migration:* code that only passes handles
  back into `PhysicsWorld` compiles unchanged via inference; code naming rapier handle types
  imports `engine::{BodyHandle, ColliderHandle}` instead. Escape hatch for forks that drop
  to raw rapier: `.raw()` on both newtypes, and `rigid_body[_mut]` / `get_collider[_mut]`
  still return raw rapier references.
- **`Scene::on_enter` takes a `SystemRegistrar` (analysis #8)** — scenes can finally
  register systems with label ordering. *Migration:*
  `fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>)` →
  `fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar)`;
  `systems.push(Box::new(X))` → `systems.add(X)`; ordering:
  `systems.add_labeled(X, SystemConfig::new().after(Y::LABEL))`. The settings_menu example
  demonstrates a real constraint (`UiSystem` after `LayoutSystem`).
- **`Sprite.texture` is `Option<Arc<str>>` (analysis #9 remainder)** — per-sprite per-frame
  batch-key `String` clones become refcount bumps. *Migration:* `Sprite::textured("x.png")`
  and `textured_with_handle` keep compiling (`impl Into<Arc<str>>`); struct literals need
  `texture: Some("x.png".into())`. RON/serde wire format unchanged.
- **`SystemMeta` merged into `SystemConfig`** — they were field-for-field identical.
  *Migration:* replace the name; `compute_order` now takes `&[SystemConfig]`.
- **`ShaderMaterial` caches its pipeline hash** — construct via
  `ShaderMaterial::new(frag_source, params)`; `frag_source` is private behind
  `frag_source()` / `set_frag_source()` (which re-hashes), so the cached hash can never
  desync. `params` stays pub. The renderer's per-frame WGSL hashing is gone.
- **`#[non_exhaustive]` on `DebugShape` and `NetworkEvent`** — external matches need a
  `_ =>` arm; future variants stop being breaking changes (`ReflectValue` precedent).
- **Visibility narrowings** — `GpuLightData` / `LightingUniforms` and
  `PostProcessRenderer.{target_view,width,height}` are `pub(crate)` (GPU internals);
  `TouchState` event fields are private behind `began()` / `moved()` / `ended()` /
  `pinch_delta()` / `swipe()` accessors; `input` submodules are private — import from
  `engine::input::{…}` or the crate root (`engine::AxisBinding` etc. unchanged).

### Changed

- Examples write quits via `ShouldQuit::quit()` instead of `q.0 = true` (field stays pub;
  examples teach the canonical API).

## 4.6.0

Non-breaking batch from the 2026-06-10 full-codebase analysis
(`docs/CODE_ANALYSIS_2026-06-10.md`, Top-10 items #1/#3/#4/#5/#6/#7/#9-partial/#10),
plus a follow-up sweep over ~30 of the remaining non-Top-10 findings (2026-06-11).
The remaining Top-10 items (#2 rapier handle newtypes, #8 `on_enter` system
registrar, plus removal of everything deprecated here) form the planned v5 breaking batch.

### Added

- **`LABEL` constants on all built-in systems** — Physics/CollisionGrid/CollisionDebug/
  Network/Particle/Tilemap/Audio/SkeletalAnimation/Hierarchy/Steering/Behavior/
  Localization/Scripting/Timeline join the five systems that already had one, so every
  engine system can now be referenced in `add_system_labeled` ordering. The platformer
  example demonstrates labeled registration; `docs/PATTERNS.md` gained a
  "System ordering with labels" section with the known constraints.
- **`SceneChange::take` / `is_pending`**, **`ShouldQuit::quit` / `is_quitting`**,
  **`ParticleEmitter::burst`** (canonical name for `for_burst`), and a root re-export
  for **`NetworkConfig`** — small API-surface consistency additions.

- **`save::write_ron` / `save::read_ron`** — plaintext pretty-RON read/write for design-time
  assets. `SceneDef`/`Prefab` `save`/`load` now produce human-editable text files instead of
  AEAD-encrypted binary (a hackability violation for level files); `read_ron` transparently
  falls back to the encrypted format so pre-4.6 files still load. Encrypted `save`/`load`
  remain the player-save path.
- **`DebugDraw::rect_filled` / `rect_filled_z`** — filled, z-ordered rectangles on the modern
  debug-draw resource, covering everything the legacy queue did.
- **Native `NetworkClient::is_connected()`** — parity with the wasm client (previously
  wasm-only, an undocumented platform API split); backed by an `AtomicBool` the socket
  thread clears on every exit path.

### Changed

- **`UvRect`/`BlendUv` moved to `renderer::uv`**, **`Lerp` moved to `tween`** — semantic
  homes instead of accidental ones (`animation::player`, `timeline`); six modules no longer
  compile-depend on `animation`, and `network::SnapshotBuffer` no longer imports the cutscene
  module. Old paths and all root re-exports keep working via `pub use` shims.
- **Editor state extracted from `App`** — 17 editor-only fields (gizmo, clipboard, undo
  history, component factories, snap, selection) now live in one internal `EditorState`
  struct (`src/app/editor/state.rs`); a fork removes the editor by deleting one field + one
  module. Internal-only; no public API change.
- **Per-frame allocation fixes** — the lighting pass no longer creates its bind group every
  frame (cached, invalidated on resize/reconfigure); the sprite renderer no longer clones
  WGSL material sources per frame (at most once per *new* pipeline). Remaining per-sprite
  texture-key `String` clones need an API break and are deferred to v5.
- **Findings-sweep cleanups (2026-06-11)** — per-frame allocation pass (text queue drained
  via `mem::take`, physics event-diff scratch buffers reused, single-pass particle emitters
  and panel layout, editor-UI allocations gated behind `is_enabled`, `exec_order` take/swap);
  `topological_sort_entities` rehomed `prefab` → `hierarchy` (shim kept); O(1) `despawn`;
  deduplicated `App::new` / `AssetServer::new` struct literals, editor Tag/multi-select UI
  blocks, input bind methods, and the fullscreen-quad vertex shader; dead private
  `play_streaming` removed; doc clarifications across modules (wasm no-op fades,
  CollisionGroups vs CollisionLayer, LocaleResource bridge, system-ordering caveats).

### Fixed

- **Animation frame catch-up** — the main-clip advance now catches up multiple frames on a
  large `dt` (previously advanced at most one frame per tick; the crossfade path already
  did this correctly).
- **`CharacterController::max_slope_angle` desync** — setting the public field directly now
  takes effect on the next `move_character` call (previously only `with_max_slope_deg`
  synced the internal rapier controller).

### Deprecated

- **`DebugDrawQueue` / `DebugRect`** — superseded by `DebugDraw::rect_filled_z`. Still
  registered and drained for compatibility; removal planned for v5. `CollisionDebugSystem`,
  the editor selection highlight, and the `sokoban` example are migrated.
- **`World::register_reflect`** (stores an empty type name, breaking Inspector display — use
  `register_reflect_named`), **`NetworkEvent::JsonParseError`** (never emitted by the
  engine), **`App::load_texture`** (use `load_image`), and **`ParticleEmitter::for_burst`**
  (renamed `burst`). All removal-planned for v5.

## 4.5.0

### Added

- **`salvage_run` example** (`examples/games/salvage_run/`) — an **area-of-interest (AOI)
  streaming** networked world: a single ship roams a world far larger than the window (2400×1800 vs
  800×600) while an authoritative server simulates ~120 wandering entities of two typed kinds
  (slow-drifting salvage, roaming drones) and streams each client **only** the entities within an
  interest radius of its last-reported position. Entities continuously stream in and out as the
  player moves — interest management made visible: a live "streaming X / 120" readout, a resizable
  AOI (`-` / `=`) with an on-screen boundary ring, and entity pop-in/out at the edge. Reuses
  `engine::SnapshotBuffer<Vec2>` per streamed entity (its third call site) for smooth motion at a
  low 12 Hz, two `RemoteEntities` maps for the two kinds, example-local last-seen + timeout eviction
  for entities that leave the AOI (the server signals departure only by *omission*), and
  `RemoteEntities::clear` on disconnect. The first example to stress AOI churn / staleness and to
  tear down on disconnect — see `docs/REMOTE_ENTITIES_DESIGN.md` (#4/#5/#7). Ships native + to the
  browser (`web/`). No engine API change (purely additive example).

## 4.4.0

### Added

- **`SnapshotBuffer<T: Lerp>`** — a generic per-entity snapshot-interpolation buffer for smoothing
  server-owned remote state that arrives at a low snapshot rate. Stamp each snapshot with the
  client clock (`push`), then `sample` a slightly delayed render time so playback always
  interpolates between two real samples (clamping at the ends). Generic over any `Lerp` value, so
  it interpolates `f32` (e.g. a rotation angle), `Vec2` (a position), `Color`, etc. It is
  **orthogonal** to `RemoteEntities`: that owns the `id → Entity` lifecycle, this owns the value
  history the renderer reads — games keep them as parallel maps. This is the promotion of
  `predict_shooter`'s former private `Interp` (now migrated onto it), triggered by a second
  interpolating example — see `docs/REMOTE_ENTITIES_DESIGN.md`.
- **`orbital_dodger` example** (`examples/games/orbital_dodger/`) — an interpolation-only networked
  game: cross the field to a vault while dodging the server's drifting, spinning hazards. The
  hazards are wholly server-authoritative at a low 10 Hz; the local player never round-trips, so
  the only netcode is interpolation (no prediction). Each hazard interpolates two channels —
  position (`SnapshotBuffer<Vec2>`) and spin angle (`SnapshotBuffer<f32>`) — which is what
  justified making the buffer generic. `I` toggles interpolation off to reveal the raw 10 Hz
  judder. Ships native + to the browser (`web/`).

## 4.3.1

### Fixed

- **Gamepad backend crash isolated (gilrs).** A controller could panic gilrs inside its event poll
  (`gamepad(id).unwrap()` on `None`), crashing the whole app ~1 s after launch. `App::poll_gilrs`
  now wraps the poll in `catch_unwind` (mirroring the per-system isolation in `schedule.rs`) — a
  flaky controller disables gamepad input for the session instead of crashing — and gilrs was
  upgraded `0.10 → 0.11.2` (reworked macOS HID backend). Note: on macOS the GameController
  framework takes *exclusive* ownership of Xbox/PlayStation pads, so gilrs (IOKit HID) sees a
  `Connected` event but no input; gamepad input works on Linux/Windows or with a generic-HID pad.

## 4.3.0

### Added

- **`RemoteEntities<K>`** — a reusable helper for the `id → Entity` lifecycle that networked games
  repeat: spawn-on-first-sight and despawn-on-removal of server-owned remote entities. Methods:
  `get_or_spawn`, `get`, `contains_key`, `remove` (despawns the entity), `clear`, `len`,
  `is_empty`, `iter`. It owns only the mapping plus spawn/despawn lifecycle — what to spawn (a
  closure), how to update an existing entity, and any parallel game-state maps stay in the game.
  The `mp_client` and `coin_race` examples now use it instead of inline `HashMap<usize, Entity>`
  bookkeeping. A richer version (interpolation, client-side prediction, update callbacks) is
  deliberately deferred until a third distinct networked example reveals its shape — see
  `docs/REMOTE_ENTITIES_DESIGN.md`.

## 4.2.0

### Changed

- **Crisp wasm rendering on Retina/HiDPI.** The wasm drawing buffer is now sized to the canvas's
  logical size × `devicePixelRatio` (uniform scale, capped so neither axis exceeds the WebGL2 2048
  max texture size) while the canvas CSS display box stays at the logical size, so the browser maps
  the buffer 1:1 instead of upscaling a logical-size buffer. Previously wasm rendered into a
  logical-size buffer (a deliberate `scale_factor = 1` workaround) — correct, but soft on Retina.
  The world viewport stays logical and `DisplayScaleFactor = buffer / logical`, so sprites and UI
  keep their coordinates and text now renders at device resolution. The logical size is read from
  the authored `<canvas>` width/height attributes (stable across scene transitions), not
  `WindowConfig`. Native rendering is unchanged.

## 4.1.0

### Added

- **`coin_race` runs in the browser (wasm).** The `coin_race_game` client now compiles to
  wasm and connects to the native `coin_race_server` over `ws://127.0.0.1:9002`, so native
  windows and browser tabs share one authoritative game. A `#[wasm_bindgen] run_coin_race`
  entry point lives in the example (not the engine library, keeping the engine a
  genre-agnostic skeleton), and `examples/games/coin_race/web/` adds an `index.html` plus a
  `build.sh` that drives `cargo build --example` + `wasm-bindgen`. This establishes the
  reusable path for shipping an engine *example game* to the web: previously only the bundled
  library demo (`examples/wasm/`, built with `wasm-pack`) could run in a browser, because
  `wasm-pack` builds only the library crate. Verified end-to-end — a browser tab's wasm
  WebSocket connects to the authoritative server and renders the player avatar and the
  server-spawned coin field via WebGL2.
- **Embedded default font for wasm text.** The browser sandbox has no system fonts, so
  `FontSystem::new()` loads an empty font db and the engine previously skipped creating the
  text renderer on wasm entirely (cosmic-text panics shaping with no fonts), meaning
  `DrawText`/HUD text silently did not render unless the game supplied a `FontData`. The engine
  now embeds DejaVu Sans (`assets/fonts/DejaVuSans.ttf`, Bitstream Vera / Arev license) and
  falls back to it on wasm when no `FontData` is set, so HUD text renders out of the box. The
  font is `include_bytes!`'d under a wasm-only `cfg`, so native binaries (which use OS fonts)
  do not embed it.

### Fixed

- **Wasm HiDPI viewport was halved on Retina displays.** The logical `ViewportSize` was
  computed as `surface_size / devicePixelRatio` for all targets, but on wasm the surface is
  already sized to the canvas DOM (CSS-logical) size — the resize handler caps it there to
  respect the WebGL2 texture limit — so dividing by the DPR again halved the world viewport.
  On a Retina display (DPR 2) a fixed-coordinate scene was projected into a half-size viewport
  and rendered almost entirely off-screen; the engine only rendered correctly at DPR 1. The
  DPR division now applies on native only (where the surface is physical pixels). Surfaced by
  playtesting the `coin_race` wasm example on a real Retina display — sprites that were pushed
  off-screen now render in place. (The `examples/wasm/` lib demo masked this because it adapts
  its layout to `ViewportSize` instead of using fixed coordinates.)
- **Wasm canvas was stretched, clipping HUD text.** winit sizes the canvas's CSS *display* box
  to the window's logical size (the 1280 default when `WindowConfig` isn't applied at canvas
  creation), which can differ from the drawing buffer — so the browser stretched an 800px buffer
  across a 1280px display and, being wider than the window, centred and clipped it. Fixed-position
  HUD text fell off the left edge while sprites (mid-canvas) stayed visible. `finish_init` now
  sets the canvas CSS width/height to its drawing-buffer size (after winit has sized it) so the
  canvas displays 1:1 with what the engine renders; a game can still override with `!important`
  CSS. Surfaced once the embedded default font made wasm text render for the first time.

## 4.0.0

### Added

- `coin_race` example (`examples/games/coin_race/` — `coin_race_game` client +
  `coin_race_server`): first playable-game use of `NetworkClient` / `NetworkSystem` /
  `NetworkEvent` in an **authoritative** design, not the position-relay model of
  `mp_client`/`mp_server`. Two or more players race to collect coins; the standalone server
  owns the coin field and the scoreboard, arbitrates contested pickups (first `grab` claim
  wins), keeps the field full, and announces the winner. Closes the last engine subsystem —
  networking — that had no playable-game example. No engine source changed: `NetworkClient`
  and `NetworkSystem` carried a full authoritative game as-is, confirming the API is
  sufficient for this pattern.

### Breaking

- **`engine::ImpulseJointHandle` (a re-export of `rapier2d`'s `ImpulseJointHandle`) is
  removed and replaced by the opaque `engine::JointHandle` newtype.**
  `PhysicsWorld::add_revolute_joint`, `add_distance_joint`, and `add_prismatic_joint` now
  return `engine::JointHandle`, and `remove_joint` takes it. The inner rapier handle is
  engine-private: a `JointHandle` can only be produced by an `add_*_joint` call, decoupling
  game code from the rapier type. Migration: replace `use engine::ImpulseJointHandle` (or
  `use rapier2d::dynamics::ImpulseJointHandle`) with `use engine::JointHandle`, and update
  return-type annotations / stored fields accordingly. Call sites that discard the return
  value need no change.

## 3.0.0

### Added

- `Color` newtype (`engine::Color`): a single unified RGBA color type with `rgb` / `rgba` /
  `rgba_u8` constructors, `From<[f32; 4]>` / `From<[f32; 3]>` / `From<[u8; 4]>` conversions,
  and `to_array` / `to_u8` / `to_rgb` helpers for the GPU / glyphon boundaries. Replaces the
  previous mix of raw color arrays throughout the public API (see Breaking).
- `AudioSystem` (`engine::AudioSystem`): a built-in system — register it like any other —
  that calls `AudioManager::update(dt)` every frame so scheduled fades (`fade_out` /
  `fade_volume`) actually advance. Previously fades were silently inert unless the game
  manually drove `update()`. Also adds an SFX file-bytes cache (path → `Arc<[u8]>`) so
  replaying the same sound effect no longer re-reads the file from disk on every `play()`;
  streaming BGM is unchanged.
- `DrawText::centered` + `TextAnchor` enum (`engine::TextAnchor`, `TopLeft` / `Center`): a
  draw position can now anchor at the measured text center, computed from the shaped buffer
  at render time with no manual `-width/2` math. Paired with `Camera::world_to_screen` (the
  inverse of `screen_to_world`) for placing screen-space text at a world position.
- `MouseButton` re-exported as `engine::MouseButton`, so games can import it from the crate
  root instead of reaching into `winit::event::`.
- `ReflectValue::I32` variant + `#[non_exhaustive]` on `ReflectValue`: integer fields are now
  inspectable in the egui Inspector alongside `F32`. `#[non_exhaustive]` means downstream
  exhaustive `match`es over `ReflectValue` must add a `_` arm.
- `ScriptingLimits` extended with `max_string_size`, `max_array_size`, `max_map_size`,
  `max_call_levels`, and `max_expr_depth`, all applied to the Rhai engine alongside the
  existing `max_operations`, with conservative defaults for trusted-local scripts.
- `spawn_scene_def` duplicate-`Tag` detection: duplicate tag keys are now first-wins with a
  `log::warn!` instead of silently overwriting. All entities still spawn; only parent-tag
  resolution is affected.
- `audio_fades` example (`examples/audio_fades.rs`): a small native demo confirming the new
  built-in `AudioSystem` drives fades in real play (Space to play, F to fade out, 1/2/3 to
  fade to a target volume) — previously the same sequence produced no audible change.
- `minimap` example gained a `WorldLabelSystem` that draws floating `"ENEMY"` nameplates
  above each enemy via `Camera::world_to_screen` + `DrawText::centered`, tracking them as the
  camera follows the player — the first live exercise of those two APIs with a moving camera.

### Breaking

- **`PhysicsWorld` is now a `World` resource**, not a field owned by `PhysicsSystem`.
  `PhysicsSystem::run` takes the resource out of the world, steps it, and re-inserts it, so
  game systems reach physics symmetrically with `world.resource_mut::<PhysicsWorld>()` —
  matching how `SpatialGrid` is exposed. Migration:

  ```rust
  // before
  let physics = PhysicsWorld::new();
  app.add_system(PhysicsSystem::new(physics, pixels_per_unit));

  // after
  app.world.insert_resource(PhysicsWorld::new());
  app.add_system(PhysicsSystem::new(pixels_per_unit));
  ```

- **All public color fields changed to `engine::Color`.** `Sprite`, `AtlasSprite`,
  `PointLight`, `AmbientLight`, `DrawRect`, `DrawText`, `DrawImage`, `ParticleEmitter`,
  `GpuParticleEmitter`, the `Timeline` color track, all UI widgets (`Button` / `CheckBox` /
  `Panel` / `ScrollView` / `Slider` / `TextInput` / `Label`), and `DebugDraw` previously held
  a mix of `[f32; 4]` / `[f32; 3]` / `[u8; 4]`. Color-accepting constructors and builders take
  `impl Into<Color>`, so call sites passing raw arrays still compile; only struct-literal
  `color:` initializers need updating (e.g. `color: [r, g, b, a]` → `color: Color::rgba(r, g,
  b, a)`). Raw arrays remain only at the GPU/glyphon boundaries via `to_array` / `to_u8` /
  `to_rgb`. Scene RON now serializes color as `(r:.., g:.., b:.., a:..)` struct form.

### Fixed

- **Per-frame hot-path costs removed.** `SpatialGrid` / `CollisionGridSystem` rebuild the
  world resource in place (remove → rebuild → insert) instead of deep-cloning two `HashMap`s
  into the resource every frame. `ScriptAsset.ast` is now `Arc<rhai::AST>` (clone = refcount
  bump, not a full AST deep-clone per scripted entity per frame), and `ScriptingSystem` reuses
  thread-local scratch buffers. A\* pathfinding (`find_path`) gained a closed set to prevent
  re-expanding stale heap entries and reuses its open list / score maps across calls (public
  signature unchanged). The sprite renderer opens a single render pass for the whole pre-sorted
  sprite stream and issues per-texture-run draws within it, instead of a new pass per batch.
- **`RenderLayer` negative values no longer fold to bit 0.** Layer→mask matching previously
  `clamp(0, 31)`-ed the layer index, so a `RenderLayer(-1)` background sprite mapped onto bit 0
  and leaked into `layer_mask: 1 << 0` offscreen passes. Layers outside `0..=31` cannot be
  addressed by a 32-bit mask and now simply never match under any non-zero mask (they still
  render under mask `0` = all layers); the engine warns once on an unmaskable layer.
- **Point-light radius falloff locked by a contract test.** A code-analysis pass flagged the
  CPU `radius * zoom / viewport_w` calculation as a possible unit mismatch; re-derivation
  confirmed it a false positive (the value is already in the shader's UV-fraction-of-width
  space and the falloff reaches zero at the world radius). A regression test now pins the
  correct behavior so a future "fix" cannot reintroduce a 2× error.

## 2.0.0

### Added

- `lit_dungeon_game` example (`examples/games/lit_dungeon/`): first playable-game use of 2D
  lighting (`PointLight`/`AmbientLight`) and `PostProcessConfig`. A dark top-down brazier-
  lighting puzzle with a decaying torch; bloom + vignette post-process (toggle with `P`).
- `blend_locomotion` example (`examples/blend_locomotion.rs`) + `gen_blend_sheet` asset generator:
  first use of `BlendTree1D` in a real interactive loop. A single speed parameter drives
  idle/walk/run clip blending; demonstrates the new true crossfade and the stranding fix below.
- `BlendUv { to, weight }` component (`engine::BlendUv`): written by `AnimationSystem` during a
  crossfade and read by the sprite renderer to cross-dissolve the two frames per-pixel.
- `ImeConfig { allowed: bool }` resource (`engine::ImeConfig`, default **off**): controls whether
  the window accepts IME text composition. Insert `ImeConfig { allowed: true }` before `App::run()`
  in apps that need text input. See the IME fix under Fixed.
- `crane_wrecking_ball` example (`examples/crane_wrecking_ball.rs`): first playable-example use of
  the physics **joint** API (`PhysicsWorld::add_revolute_joint` / `add_distance_joint`). A kinematic
  crane cart hangs a revolute-pinned arm with a distance-tethered wrecking ball; drive the cart to
  swing the ball and knock a block stack off its pedestal. The joint methods shipped with unit tests
  but had zero game/example coverage. Demonstrates the rotation-sync fix below.
- `security_camera` example (`examples/security_camera.rs`): first playable-game use of
  `RenderTarget` / `OffscreenCamera`. A stealth puzzle where a guard patrols a room that is
  **entirely offscreen** — its only view is a wall monitor (an `OffscreenCamera` renders the guard
  room into a `RenderTarget` that a `Sprite` samples). Read the guard's position on the monitor and
  cross the doorway when it is away from the door stripe; reach the exit to escape, get caught to
  reset (`R` replays, `Esc` quits). The existing `minimap`/`split_screen` demos exercised the API but
  only ever framed the *same* region the main camera shows; this is the first use of an offscreen
  camera as the sole view of a **disjoint** region. Demonstrates the offscreen-render fix below.
- `timeline_cutscene` example (`examples/timeline_cutscene.rs`): first use of `Timeline` in a
  playable scene. Walk into a rune to trigger a cutscene that pans/zooms the camera, slides two gate
  panels apart, and fades a full-screen overlay — all driven by `Timeline` keyframe tracks; Space
  skips, control returns when it ends, then you cross the now-open gate to the exit. `Timeline`
  shipped with unit tests but had zero example/game coverage. Demonstrates the camera-drive addition
  below.
- `CameraTarget` marker component (`engine::CameraTarget`) + a `Timeline::zoom` track: a `Timeline`
  on an entity tagged `CameraTarget` drives the `Camera` resource (its `position` track → camera
  position, `zoom` track → camera zoom) as a virtual camera rig, instead of the entity's own
  `Transform`/`Sprite`. Lets a `Timeline` author camera moves for cutscenes — previously `Timeline`
  could only animate an entity's own transform/sprite. Additive: ordinary timelines are unaffected
  (the `zoom` track is empty by default).

### Breaking

- `Entity` is now an opaque generation-checked handle with `index()`, `generation()`, and
  `from_raw_parts(index, generation)`. Direct `entity.0` access is removed.
- `World::clone_entity(src)` now returns `Option<Entity>` and returns `None` for stale or
  despawned handles.
- Rhai scripting now uses `despawn_entity(index, generation)` instead of index-only
  `despawn_entity(id)`.
- Rhai scripting exposes `entity_index()` and `entity_generation()` for the current
  script runner entity.
- Removed the misleading public `Sprite.normal_texture` and `Sprite.normal_handle` fields.
  v2 keeps flat-normal lighting internally but does not expose per-sprite normal maps.

### Fixed

- Post-process shader (`post_process.wgsl`) declares the bloom tap-offset array as `var` instead
  of `let`, fixing a naga validation error ("may only be indexed by a constant") that panicked on
  shader creation whenever `PostProcessConfig.enabled` was `true`. Surfaced by the new
  `lit_dungeon_game` — the first runtime use of post-processing (CI compiles but never runs the
  windowed app).
- 2D lighting now projects `PointLight` positions with the **logical** viewport size (matching
  the sprite pass) instead of the physical surface size. On HiDPI/Retina displays (scale > 1)
  lights previously drifted from their sprites and rendered at half radius; on scale-1.0 displays
  it happened to line up, which is why it went unnoticed. Also surfaced by `lit_dungeon_game`.
- Screen-space text (`TextQueue`/`DrawText`) now renders **after** the post-process and lighting
  passes, so HUD/overlay text is no longer dimmed by world lighting (or warped by post effects).
  Trade-off: `DrawText` is no longer affected by `PostProcessConfig`; route text through egui if
  you want it post-processed. Surfaced by `lit_dungeon_game`.
- Post-processing and lighting now compose as `scene -> post -> lighting -> final` when both
  effects are active.
- Lighting intermediate targets are recreated after viewport resize, and `PointLight`
  positions now respect camera position and zoom.
- Scene replacement restores the same core engine resources as initial app creation, including
  panic recovery state, and preserves initialized `DebugUi`.
- Images loaded directly through `AssetServer::load_image` are lazily uploaded to the GPU cache,
  so scene-owned loading no longer depends on `App::load_image`.
- `BlendTreeSystem` no longer strands an entity on an intermediate clip when the blend parameter
  crosses two thresholds (e.g. idle→walk→run) within a single crossfade: it now defers the new
  transition instead of recording an unachieved target, and re-evaluates once the crossfade ends.
  Surfaced by `blend_locomotion`; regression test in `src/animation/blend_system.rs`.
- Game key input is no longer broken when a CJK IME (Korean/Japanese/Chinese) is active. The window
  previously enabled IME unconditionally, so on macOS the OS could route key-release events into IME
  composition and leave keys stuck "pressed" (e.g. a held movement key never released → the
  character kept moving). IME is now **off by default** and opt-in via the new `ImeConfig` resource;
  only text-input apps (`settings_menu_game`) enable it. Surfaced by `blend_locomotion` (a held
  accelerate key stayed latched under a Korean IME, so the clip never returned to idle).
- `PhysicsSystem` now syncs each body's **rotation** into `Transform.rotation`, not just its
  position. Previously a body that rotated under physics (e.g. a joint-driven swinging arm) kept a
  bolt-upright sprite because rotation was silently dropped. Rotation-locked bodies (`lock_rotation:
  true`) are unaffected (their angle is always 0). Surfaced by `crane_wrecking_ball`; regression
  tests in `src/physics/system.rs`. Behaviorally inert for consumers that own a raw `PhysicsWorld`
  and sync transforms themselves (e.g. `rust-survivors`).
- Offscreen render targets (`OffscreenCamera` → `RenderTarget`) now render with their **own** camera
  instead of the main camera. The sprite renderer's camera uniform is a single shared buffer updated
  via `queue.write_buffer`; the offscreen pass and the main pass were recorded into one command
  submission, and within a single submit only the **last** write to that buffer takes effect — so
  every offscreen target was drawn with the (later-written) main camera's view. The
  `minimap`/`split_screen` demos masked this because their offscreen content overlaps the main view;
  it became obvious only with an offscreen camera framing a *disjoint* region (the monitor rendered
  the main scene instead of the guard room). Each offscreen target now submits in its own command
  buffer so its camera write pairs with its own draws. Surfaced by `security_camera`; GPU-validated
  by a native run (CI compiles but cannot run the windowed app).
- `split_screen` example no longer crashes with a wgpu validation error on its second frame. It used
  `layer_mask: 0` (render all layers), so its render-target *display* sprites were drawn into the
  same targets they sample — a self-capture (a texture used as both color attachment and sampled
  resource within one render pass). It "survived" only frame 1, before the targets were registered.
  Fixed by masking the display sprites out of the offscreen pass (`layer_mask: 1 << 0`), the same
  self-capture-avoidance `minimap` already uses.

### Changed

- `SceneDef` schema version is now `2`; old v1 files with removed normal-map sprite fields are
  accepted and those fields are ignored.
- Agent instructions now define this repository as the default and only verification scope unless
  the user explicitly asks for external project checks.
- Lighting now renders the **nearest 16** point lights to the camera when a scene exceeds the
  16-light hard cap (previously the first 16 in arbitrary query order), and warns once. Light
  occlusion/shadows and per-sprite normal maps remain out of scope; lighting stays native-only.
- Animation crossfades are now a true **2-UV shader-lerp** cross-dissolve (`mix(from, to, weight)`
  in `sprite.wgsl`) instead of a 50% hard frame-swap, and `BlendWeight` is finally consumed by the
  renderer (via the new `BlendUv` component). Additive: a sprite that is not crossfading
  (`weight = 0`) renders byte-identically to before. `InstanceRaw` gained internal `to_uv`/`blend`
  fields; the sprite path stays cross-platform (the blend works on wasm too).

## 1.3.0

### Added

- `TextInput` single-line **horizontal scrolling**: long values no longer wrap or clip out of view.
  The field renders as one non-wrapping line and scrolls so the caret stays visible while typing or
  navigating (`Home`/`End`/arrows); an unfocused field anchors to the start. New `DrawText`
  opt-in `with_single_line_caret(caret_byte)` drives it — the renderer measures the caret x via
  glyphon `Buffer::layout_runs()` and shifts the `TextArea` left, clipped to the field by
  `TextBounds` (no new render pipeline).
- `TextInput::remaining_capacity()` and `TextInput::caret_display_offset()` helpers.

### Fixed

- IME at `max_len`: composing input when the field is full no longer shows a phantom, uncommittable
  preedit. `UiSystem` only displays the IME preedit while it still fits in the remaining capacity
  (`remaining_capacity() >= preedit.len()`); commits already truncate to fit.

### Example

- `settings_menu_game` Settings scene gained a dedicated narrow long-text field (prefilled past its
  width, `max_len` 48) that exercises horizontal scroll, caret-follow, and IME-at-capacity.

## 1.2.1

### Fixed

- macOS live window-**resize** drag froze the content: while the OS runs its modal resize loop
  the normal `about_to_wait → request_redraw → RedrawRequested` cadence is parked, and the
  `Resized` handler only reconfigured the surface without drawing. The frame step (update +
  render) is now factored into `App::step_frame` and is also driven inline from `Resized`, so
  animations keep advancing while the window is being resized.

### Added

- `Window::pre_present_notify()` is now called immediately before `surface.present()`, the
  winit-recommended compositor hint that trims presentation latency.
- `settings_menu_game` gained a small always-animating spinner (bottom-left, `dt`-driven, no
  input) so a window-drag freeze is visible by eye — it stalls during a drag and resumes after.
- Debug instrumentation: `step_frame` logs `frame gap <ms>` at `debug` level when the
  inter-frame gap exceeds ~33 ms, to quantify drag/stall (e.g. `RUST_LOG=engine=debug`). The
  `settings_menu_game` example now initializes `env_logger` (native-only dev-dependency) so the
  `log` output is actually visible — previously no log backend was installed.

### Known limitations

- A one-frame lag remains at the **start** of a live drag (both resize and titlebar move): the
  window content tracks the cursor a beat late on the first movement, then follows normally for
  the rest of the drag. The hard freeze is gone — content keeps animating throughout both drag
  kinds on the tested macOS (15.x / Darwin 25) — but this residual start-of-drag latency is a
  macOS/winit present-timing artifact left as a documented limitation per the "known levers"
  scope (deeper fixes — background redraw thread / native Cocoa hooks — were out of scope).

## 1.2.0

### Added

- `LocalizedText` component plus `LocalizationSystem` — bind a translation key to a `Label`,
  `Button`, or `CheckBox` and the system keeps its text in sync with the current locale every
  frame. Switching language is now just `LocaleResource::set_locale(..)`; the whole UI
  retranslates with no manual per-widget rebuild. Re-exported from the crate root.
- `settings_menu_game` example (`examples/games/settings_menu/`) — a Title → Settings → Dialogue
  slice that is the first playable-game coverage for the UI-depth + localization + audio-bus
  surface: `TextInput`, `Slider`, `CheckBox`, `ScrollView`, `Panel`/`LayoutSystem`, rich/multiline
  `Label`, `LocaleResource` (EN/KO/ES) + `LocalizedText`, and `AudioManager` buses + `AudioEffect`
  low-pass. Cross-scene `Settings`/locale/`AudioManager` survive `SceneCmd::Replace` via
  `App::register_persistent`.

### Fixed

- Clicks landing on the wrong widget after a mouse move: `InputState` keeps only the latest
  cursor, so when a press and a following move collapsed into one frame the click was hit-tested
  at the moved-to position (e.g. pressing empty space then moving onto a button activated it,
  while pressing a button then moving off did nothing). `InputState` now records the cursor at the
  press and release moments (`mouse_press_cursor`/`mouse_release_cursor`), and `UiSystem` hit-tests
  clicks/toggles/drag-starts against those (hover and drag-tracking still use the live cursor).
- `TextInput` caret rendering: the caret `|` was always appended at the end of the string, so it
  never matched the real cursor after navigation and text appeared to be inserted "in the middle".
  Added `TextInput::display_with_caret` which inserts the caret (and IME preedit) at the byte
  cursor; `UiSystem` uses it. The caret blinks while focused but its slot is always reserved (a
  space when off, `|` when on) so blinking no longer shifts the trailing text.
- Input-to-display latency: `desired_maximum_frame_latency` lowered from 2 to 1 (vsync kept, no
  tearing) so button/drag feedback lands a frame sooner.
- IME / non-Latin input: `set_ime_allowed(true)` is now called on the window, so macOS (and other
  platforms) compose CJK input and deliver it via `Ime::Commit`. Previously IME was never enabled,
  so Korean arrived as separated jamo (per-keystroke `Character` events).
- `AudioManager::play_tone` now applies the channel's effective bus volume to the sink and the
  channel `AudioEffect` (low-pass / pitch / fade-in), matching file playback. Previously tones
  ignored both, so `set_bus_volume` and `set_effect` had no audible effect on tone channels.
- Interactive responsiveness: the event loop never set a `ControlFlow`, defaulting to `Wait`, so
  drags/hover updated a beat late and sliders did not track the cursor smoothly. It now runs with
  `ControlFlow::Poll` for a continuous per-frame loop (vsync-paced via the existing redraw request).
- `TextInput` cursor editing: added `move_left`/`move_right`/`move_home`/`move_end`/`delete_forward`
  (UTF-8 safe) on `TextInput`, and `UiSystem` now applies ←/→/Home/End/Delete to the focused field.
  Previously the caret could only sit where typing left it (no navigation, no forward delete).
- HiDPI mouse/touch hit-testing: the cursor was stored in physical pixels while UI hit-testing,
  `ViewportSize`, and `Camera::screen_to_world` all work in logical pixels, so on a scaled display
  (e.g. Retina 2×) clicks landed offset from the cursor. `CursorMoved` and the touch→mouse
  emulation now divide by the window scale factor, storing the cursor in logical coordinates
  (no-op at scale 1.0). Surfaced by `settings_menu_game`'s click-heavy widgets; also corrects
  editor gizmo dragging and any `screen_to_world` use on HiDPI.

### Known gaps (surfaced, not yet addressed)

- `LocaleData.font` is not applied at runtime: `TextRenderer` takes its font once at init via the
  `FontData` resource, so per-locale font switching is unsupported. Non-Latin scripts render only
  through native system-font fallback and are absent on wasm (no system fonts). Korean in
  `settings_menu_game` therefore renders on macOS but not on Linux CI / wasm.
- `LocaleData.direction` / `TextDirection::RightToLeft` is metadata only — the text renderer does
  not auto-apply RTL alignment from the locale (it maps `TextAlign::Right` explicitly). No RTL
  locale ships in the example, so RTL is left for a future dedicated example.
- No window fullscreen-request path exists yet, so `settings_menu_game`'s Fullscreen checkbox only
  stores a preference (its label says so); wiring real OS fullscreen is deferred.
- The built-in `TextInput` is single-line with no horizontal scrolling: text longer than the field
  width clips at the edge, and IME composition at the `max_len` cap shows an uncommittable preedit.
  Adequate for short fields (names, search); a scrolling multi-line field is future work.
- The blinking `TextInput` caret is drawn inline (a reserved `|`/space slot), so it can still shift
  the trailing text by a sub-pixel on blink. A fully stable caret needs a renderer-measured overlay
  (the text renderer drawing the caret quad at the glyph position); deferred.
- Residual input-to-display latency on macOS: even with `frame_latency=1`, a click registers a beat
  late, and the window content lags during a live OS window drag (winit enters a modal event-loop
  mode). `AutoNoVsync` only helped marginally while uncapping the frame rate, so it was not adopted.
  Treated as a macOS/winit optimization to revisit.

## 1.1.0

### Added

- `BlackboardValue::Path(Vec<IVec2>)` plus `Blackboard::set_path`/`get_path` so behavior
  trees can cache a whole A* path instead of recomputing every tick. `BlackboardValue` is
  now `#[non_exhaustive]`. Validated by `maze_escape_game`, whose enemies now cache the path
  and only re-run `find_path` when the player's goal tile changes.
- `App::register_persistent::<T>()` plus `World::take_resource_erased`/`insert_resource_erased`
  to preserve chosen resources across the `World` reset that `SceneCmd::Replace` triggers.
  `scene_flow_game` uses it to drop its `Arc<Mutex<_>>` cross-scene state workaround.
- `PhysicsWorld::add_static_from_tilemap(tilemap, ppu, collider_for)` and the `TileCollider`
  descriptor (`solid` / `solid_with` / `one_way`) to generate one static collider per solid
  tile, aligned to `TilemapSystem`'s tile coordinates. `platformer_game`'s level is now a
  single `Tilemap` that drives both rendering and collision; its seamless tileset is
  reproducible via `examples/gen_platform_tiles.rs` (the original `tiles.png` is a set of
  discrete object sprites with transparent margins, not a seamless tileset).
- One-way platforms: `PhysicsWorld::set_one_way`/`is_one_way` and
  `CharacterController::request_drop`/`is_dropping`. `move_character` now passes through
  one-way colliders when ascending or dropping and only lands on them from above.
  `platformer_game` adds a one-way platform and an S/Down drop-through key.

### Added (pre-1.1 carryover)

- 2D cutout (rigged) skeletal animation in `src/skeletal.rs`: `SkeletalAnimator`,
  `SkeletalClip`, `BoneTrack`, `BoneKeyframe`, `SkeletalAnimationSystem`, and the
  `SkeletonBuilder` authoring helper. Bones are hierarchy entities whose local
  `Transform` is keyframed; the existing `HierarchySystem` and sprite renderer draw them
  with no renderer changes. See `docs/SKELETAL.md` and `examples/skeletal_puppet.rs`.
- Re-exported `AssetId`, `SaveKey`, `save_with_key`, and `load_with_key` from the crate root so public examples match the stable API surface.
- Added `ScheduleErrorPolicy` and `SystemPanicPolicy` so apps can opt into stricter schedule-cycle and system-panic behavior while keeping the existing fallback defaults.
- Added `examples/runtime_policies.rs` to show strict runtime policy configuration without opening a long-running window.
- Added `World::mark_changed<T>()` and `World::get_mut_tracked<T>()` for explicit ECS change tracking after direct component mutation.
- Added native `AudioChannelState` plus `AudioManager::playback_state`, `is_finished`, and `is_playing` so games can advance non-looping playlists when a channel naturally drains.
- Added `docs/ENTITY_GENERATION_V2_PLAN.md` to lock the v2 design for generation-checked entity handles.

### Changed

- `HierarchySystem` now propagates `GlobalTransform` in topological (root→child) order in a
  single pass, supporting arbitrary hierarchy depth. It previously ran a fixed 2-pass loop
  capped at depth 3 — a limit surfaced by deep skeletal bone chains.
- Aligned save encryption and async asset examples in the public reference with the current source.
- Native `AssetServer` cache keys now canonicalize existing file paths, reducing duplicate handles and hot-reload misses caused by mixed relative/absolute paths. Missing paths and WASM URLs keep their existing string behavior.
- Sprite renderer file texture cache lookups now accept both the original requested path and the canonical `AssetServer` handle path, so `Sprite::textured_with_handle(...)`, `DrawImage::textured_with_handle(...)`, and atlas textures no longer fall back to white when images are loaded through relative paths.
- Native audio decoding now enables MP3 in addition to WAV and Vorbis/OGG.
- `PhysicsSystem` now documents the physics-unit to pixel-unit boundary and defensively clamps invalid `pixels_per_unit` values in release builds while asserting in debug builds.
- Clarified that Rhai scripting is intended for trusted local game code, not hostile sandboxing, and documented the limits of temporary script spawn IDs.
- **Breaking rendering behavior fix:** fixed the default sprite quad UV orientation so `Sprite`, `DrawImage`, `AtlasSprite`,
  `UvRect::FULL`, `UvRect::from_grid(...)`, and `UvRect::from_pixels(...)` render
  normal top-left-origin PNGs upright without requiring `UvRect::flipped_y()`.
  Existing game-side `.flipped_y()` orientation workarounds should be removed after
  updating the engine.

### Fixed

- Restored the `wasm32-unknown-unknown` build: the WebSocket `wasm_impl` module called
  `push_event_bounded` unqualified without importing it, breaking the wasm target while the
  native build was unaffected. The function is now imported into the module scope.
- Removed the redundant manual `unsafe impl Send/Sync for BehaviorTree`. The
  `BehaviorNode: Send + Sync` trait bound already guarantees both, so the hand-written impl
  was unnecessary and would have silently masked unsoundness if that bound were ever relaxed.

## [1.0.0] - 2026-05-27

### Added

- Stable `skeleton-engine` package metadata with library crate name `engine`.
- Rust 1.88 minimum supported Rust version declaration.
- README, MIT license, changelog, and beginner `examples/basic.rs`.
- CI gates for formatting, clippy, full native tests, release build, WASM build, rustdoc warnings, `cargo package`, and `cargo publish --dry-run`.

### Changed

- Documented release package hygiene with an explicit crates.io include list.
- Updated public documentation examples for current `OffscreenCamera`, `Sprite`, `TouchState`, and `glam::Vec2` usage.
