# Plan — Tile Paint texture-swatch palette (skeleton-engine v8.12.0)

> Chain `editor-tile-painting` seq 2. Additive (semver-minor), native-gated editor UX upgrade.
> Replaces the numbered `1..N` paint palette with clickable **real tile thumbnails** from the
> selected tilemap's atlas. Acceptance test = the existing `tile_paint` example (4-colour atlas →
> 4 swatches) + native playtest. **Await user merge** (never self-merge).

## Goal

In the F2 docked editor's **Tile Paint** inspector section, show each paintable tile as an image
swatch (the actual atlas sub-texture) instead of a numbered button. Clicking a swatch sets
`paint_value`. Keeps the "Erase (0)" button and the current-value highlight. Everything else
(painting, undo, gizmo suppression) is unchanged.

## Feasibility (verified this session)

- A loaded atlas's GPU texture lives in `SpriteRenderer::texture_cache: HashMap<String, Arc<Texture>>`
  (`src/renderer/sprite.rs:58`), keyed by the atlas path. `TilemapAtlas.texture: String` is that key.
- `Texture` exposes `pub view: wgpu::TextureView` (`src/renderer/texture.rs:22`).
- `egui_wgpu::Renderer::register_native_texture(&device, &view, FilterMode) -> egui::TextureId` is
  already the docked-scene-RT pattern (`src/app/render.rs:200`), with `free_texture(&id)` for cleanup
  (`render.rs:174/216`). `EditorState::docked_texture_id` is the precedent for storing such an id.
- `TilemapAtlas::uv_for(tile_id) -> UvRect` (`src/tilemap.rs:35`) gives the per-tile UV sub-region.
- No prior `egui::Image`/`ImageButton` use in the repo — this introduces it (egui 0.34).

## Architecture decision — register before the egui pass

egui texture registration needs `&mut egui_renderer` + `&gpu.device` + the atlas `&TextureView`
(from `sprite_renderer.texture_cache`). The inspector UI is built *inside* the egui pass where
`egui_renderer` is already borrowed, so registration must happen **before** building the docked UI
(exactly like the docked-RT registration in `render.rs`). The UI then just reads a stored
`egui::TextureId`. Disjoint App fields (`egui_renderer` / `gpu` / `sprite_renderer` / `editor`) are
accessed separately to satisfy the borrow checker (mirror `render.rs:198`).

## Implementation steps

### 1. `SpriteRenderer` accessor — `src/renderer/sprite.rs`
```rust
/// Borrow the GPU texture view for a loaded image/atlas by its asset path, if cached.
pub fn texture_view(&self, path: &str) -> Option<&wgpu::TextureView> {
    self.texture_cache.get(path).map(|t| &t.view)
}
```

### 2. `EditorState` swatch-texture handle — `src/app/editor/state.rs`
```rust
/// (atlas path, egui texture id) currently registered for the Tile Paint swatch palette.
/// Freed via `egui_wgpu::Renderer::free_texture` when the atlas changes / paint mode exits.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) paint_atlas_tex: Option<(String, egui::TextureId)>,
```
Init `None` in `new()`.

### 3. Registration step (before docked UI) — `src/app/render.rs` (or a small `App` helper called there)
- Compute the desired atlas path: `if editor.paint_mode { inspector_selected → get::<Tilemap>() → atlas.texture.clone() } else { None }`.
- If desired path differs from `editor.paint_atlas_tex`'s path (or desired is `None`):
  - `free_texture(&old_id)` if present.
  - If desired `Some(path)` AND `sprite_renderer.texture_view(&path)` is `Some(view)`:
    `register_native_texture(&gpu.device, view, FilterMode::Nearest)` → store `Some((path, id))`.
    (Nearest = crisp pixel-art swatches.) If the view isn't cached yet (first frame), leave `None`
    — the UI falls back to numbered buttons that frame.
- Free on docked-mode exit too (so no leak): clear when `editor.mode != Docked` or `!paint_mode`.

### 4. Swatch palette UI — `src/app/editor/ui/docked.rs` `inspector_tab_body`
Replace the numbered-button row with:
- "Erase (0)" button (kept; `selectable`-highlighted when `paint_value == 0`).
- For `tile_id in 0..(columns*rows)` (paint value = `tile_id + 1`):
  - If `editor.paint_atlas_tex` is `Some((_, tex_id))`: render an
    `egui::ImageButton::new(egui::Image::new(egui::load::SizedTexture::new(*tex_id, [SZ, SZ])).uv(uv_rect_from(atlas.uv_for(tile_id))))`,
    where `uv_rect_from(UvRect) -> egui::Rect` maps `(u0,v0)-(u1,v1)` → `Rect::from_min_max`.
    `.selected(paint_value == tile_id+1)` for highlight; on click set `paint_value = tile_id+1`.
  - Else (texture not registered yet): the existing numbered button (graceful fallback).
- `SZ` ≈ 28–32 px swatches, wrapped (`horizontal_wrapped`).
- Add a tiny pure helper `fn uv_rect_to_egui(uv: UvRect) -> egui::Rect` (unit-testable).

### 5. Example / docs
- The existing `tile_paint` example already has a 4-tile atlas → 4 swatches; **no example change needed**
  (it's the acceptance test). Optionally widen the atlas to 6–8 distinct colours to show more swatches.
- `Cargo.toml` version 8.11.0 → 8.12.0; `Cargo.lock`; `docs/CHANGELOG.md` `## 8.12.0`; `CLAUDE.md`
  header + the editor module-map row (mention "image-swatch palette").

## Completion criteria (DONE = all checked)

1. `SpriteRenderer::texture_view(path) -> Option<&wgpu::TextureView>` accessor added.
2. `EditorState::paint_atlas_tex: Option<(String, egui::TextureId)>` (native-gated) + `new()` init.
3. Pre-egui registration step: registers the selected tilemap's atlas texture when paint mode is on,
   re-registers on atlas-path change, and **frees** the old/again on paint-mode exit, deselection,
   different-atlas selection, and docked-mode exit (no egui texture leak).
4. Inspector palette renders clickable **image swatches** (real atlas tiles) with per-tile UV from
   `atlas.uv_for`, current-value highlight, and the "Erase (0)" button; clicking sets `paint_value`.
5. Graceful fallback to numbered buttons when the atlas texture isn't registered yet (first frame /
   headless), so the UI never breaks.
6. `uv_rect_to_egui` pure helper has a unit test (UV→egui Rect mapping); existing 6 paint tests still pass.
7. Gate6 fully green (fmt / clippy / wasm lib+bins / test --all-targets / doc / package). Additive,
   native-only, wasm surface unchanged.
8. Native F2 playtest on `tile_paint`: select Tilemap → Tile Paint → swatches show the 4 real colours
   → clicking a swatch selects it (highlight) → painting uses that tile. Screenshot confirms thumbnails.
9. Version 8.12.0; CHANGELOG + CLAUDE updated; HANDOFF note.
10. Branch pushed, PR opened — **left for the user to merge**.

## Risks / unknowns

- **egui 0.34 ImageButton/Image API exact shape** — first use in this repo; verify
  `egui::ImageButton::new`, `egui::Image::new`, `egui::load::SizedTexture`, `.uv(Rect)`, `.selected(..)`
  against the pinned egui 0.34 during impl (adjust builder calls as the version requires).
- **Registration borrow** — `register_native_texture` while `sprite_renderer.texture_cache` is borrowed
  for the view: take the view ref and device ref, drop other borrows; mirror `render.rs:198` field-split.
- **Texture lifetime** — the atlas `Arc<Texture>` stays alive in `texture_cache`; the egui registration
  holds a view created from it. Free the egui id (not the wgpu texture) on change. Don't free the
  atlas's cache entry.
- **Playtest** — GUI-only; the cursor-freeze limit still applies, but swatch *selection* is an inspector
  click (works), and a single viewport paint with the chosen swatch is demonstrable (frozen cell).

## Out of scope
- Scrolling/searchable palette for huge atlases (wrap is enough for MVP).
- Multi-atlas / per-layer tilemaps.
- Drag-to-pick from the viewport (eyedropper).
