# Dialogue per-line portrait rendering (v0.25.0)

**Date:** 2026-06-18
**Status:** **COMPLETED — implemented, windowed-playtested (4 frames), CI-equivalent gate green; shipped as v0.25.0 on branch `feat/dialogue-portrait` (PR pending push/open).**
**Chain:** `engine-hardening` seq `26` · **Parent:** `HANDOFF_engine-hardening_webaudio-verify_2026-06-18.md` (seq 25)
**Origin:** A-tier item A1 from the post-roadmap feature list (user picked it). Completes a
half-built feature: `DialogueBox.portrait` (field + `with_portrait` builder) shipped in v0.19.0 but
`DialogueSystem` only ever drew text, so the portrait was stored and never rendered.

---

## What shipped
- **`DialogueSystem` now renders the portrait** (`src/dialogue/mod.rs`): when an active box has a
  `portrait: Option<Handle<ImageAsset>>`, it pushes a 96×96 `DrawImage::with_handle` to the
  existing `UiImageQueue` (screen-space, left of the text at `(48, vh-162)`) and shifts the text
  right (`x0 = 48+96+18 = 162`) to clear it. A box with **no** portrait keeps the original left
  margin (`x0 = 60`) → byte-identical to before. No new public API — the field + builder existed.
- **Example `dialogue_portrait`** (`examples/dialogue_portrait.rs`) — a 4-segment conversation
  (Sage→Knight→Sage→Narrator) where the portrait switches per speaker and the last (Narrator)
  segment omits a portrait to show the text-only fallback. Mirrors `dialogue_demo`'s SPACE-advance
  driver. Two **generated** portrait assets `examples/assets/portrait_{sage,knight}.png` (128×128
  RGBA, made by a throwaway pure-Python `zlib` PNG encoder — no PIL/deps; simple bg+head+eyes).
- Release paperwork v0.25.0 (Cargo.toml/lock, CHANGELOG, CLAUDE.md header v1.6.74 + dialogue
  module-map row).

## Implementation notes / gotchas
- **`clippy::type_complexity`** fired once the per-frame gather tuple grew to 5 fields
  (`+ Option<Handle<ImageAsset>>`). Fix = a small private `struct DrawItem { speaker, body, full,
  choices, portrait }`. Keep that pattern if the gather grows again.
- **Two resource borrows can't overlap:** images go to `UiImageQueue` and text to `TextQueue` in
  **separate** `resource_mut` blocks. Portraits iterate `&items` (borrow), the text block consumes
  `items` (move) — order matters.
- `Handle<T>` is `Clone` (O(1), manual impl in `src/asset.rs:61`); cloned out of the immutable
  `query::<DialogueBox>()` into the `DrawItem`.
- `DrawImage`/`UiImageQueue` are re-exported from `crate::renderer` and the queue is a default
  resource (`app/core_resources.rs`), taken+rendered each frame like `TextQueue` — so pushing every
  frame is correct.

## Verification
- **Windowed playtest (macOS, the canonical VISION acceptance test)** — launched the example,
  drove it with synthetic `key code 49` (SPACE; clicks don't reach winit, keys do), screencaptured
  3 states: **Sage** portrait (teal) + right-shifted text → **Knight** portrait (purple, switched)
  → **Narrator** line with NO portrait at the original left margin (fallback). All correct.
- Full `./scripts/verify.sh` green **after** the version bump (fmt / clippy --all-targets / wasm
  lib+bins build / test --all-targets / rustdoc -D warnings).

## Files changed
- `src/dialogue/mod.rs` (portrait render + `DrawItem`), `examples/dialogue_portrait.rs` (new),
  `examples/assets/portrait_{sage,knight}.png` (new), `Cargo.toml`, `Cargo.lock`,
  `docs/CHANGELOG.md`, `CLAUDE.md`.

## Risks & Blockers
- None. Additive, no API change, gate green, visually confirmed.

## Where to go next (optional — from the seq-25 feature list)
- **C1 isometric/hex tilemap** (biggest unmet demand; `TilemapProjection: Square|Iso|Hex`).
- **A2 fuller wasm audio** (per-source handles, `StereoPannerNode` pan) — continues seq-25.
- **B1** `TilemapAutotile { mode }` unification; **B2** UI Tab/focus navigation.
- crates.io publish (irreversible, explicit go).

## Quick start
```bash
cd /Users/jkl/Projects/skeleton-engine
grep -m1 '^version' Cargo.toml        # 0.25.0
./scripts/verify.sh                    # green
cargo run --example dialogue_portrait  # SPACE to advance; portrait switches per speaker
```
