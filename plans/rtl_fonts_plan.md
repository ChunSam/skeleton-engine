# Item 3 — RTL text: multi-font loading + reading-direction alignment (v8.25.0)

## Goal

Make right-to-left text (Hebrew/Arabic) usable end-to-end: a game can load an RTL-script font
alongside its Latin font, and RTL text aligns to the right automatically. Ship a playable example.

## Why this design / key discovery

- **Bidi/RTL shaping already works** — the text renderer uses `Shaping::Advanced` (cosmic-text +
  rustybuzz), so RTL text shapes correctly *given a font with the glyphs*. So "RTL fonts" is NOT a
  from-scratch shaping build; the real gaps are font **coverage** and **alignment**:
  1. `FontData` loads a single blob → no way to supply Latin + an RTL-script font together.
  2. `TextAlign` had only Left/Center/Right → no reading-direction-aware option (RTL should right-align).
- The bundled default is a Latin font with no Hebrew/Arabic, so an example needs a real OFL RTL font.

## Scope (additive)

- **Alignment (A):** `TextAlign::Auto` (→ `set_align(None)`, cosmic-text aligns by resolved line
  direction → RTL right-aligns) + `TextAlign::End` (→ `cosmic_text::Align::End`). `to_glyphon` now
  returns `Option<Align>`; the render path passes it straight to `set_align`.
- **Multi-font (B):** `ExtraFonts(Vec<Vec<u8>>)` resource, loaded in `window.rs` alongside `FontData`
  and passed to `TextRenderer::new(.., extra_fonts)`. Font loading extracted into a headless-testable
  `build_font_system(font_data, extra_fonts)`.
- **Asset:** `assets/fonts/NotoSansHebrew-Regular.ttf` (16KB, SIL OFL) + `NotoSansHebrew-OFL.txt`,
  matching the repo's existing `DejaVuSans.ttf` + license convention.
- **Example (C):** `examples/rtl_text.rs` — DejaVu (Latin) as `FontData` + Noto Hebrew as `ExtraFonts`;
  renders Latin LTR, Hebrew RTL (Auto right-aligns), forced-left Hebrew, mixed bidi, and End-aligned
  Latin. Auto-discovered (no `[[example]]` entry needed — it's a top-level example).
- Exports: `ExtraFonts` in `lib.rs`. Version bump 8.24.0 → 8.25.0.

## Completion criteria

1. `cargo test --lib` green, +tests:
   - `text_align_to_glyphon_mapping` — Auto→None, End→Some(End), Left/Center/Right map through.
   - `build_font_system_loads_extra_fonts` — loading the bundled Hebrew font adds exactly one db face.
   - `build_font_system_skips_empty_blobs` — empty blobs are skipped.
2. Full Gate6 green (incl. `rtl_text` builds; `cargo package` includes the new font asset).
3. `rust-survivors` unaffected (purely additive: new resource + enum variants; `to_glyphon` is private).

## Out of scope

- Per-locale automatic font selection (font tied to `LocaleResource`) — possible follow-up; the
  game controls fonts via `FontData` + `ExtraFonts` for now.
- Bundling an Arabic font (cursive joining) — Hebrew is smaller and demonstrates RTL; Arabic works the
  same way once a game supplies a Noto Sans Arabic blob in `ExtraFonts`.
