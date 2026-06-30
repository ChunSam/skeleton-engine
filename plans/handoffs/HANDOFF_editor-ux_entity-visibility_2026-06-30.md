# Entity visibility — `Hidden` component + editor per-row eye toggle shipped (v0.93.0, PR #295)

**Date:** 2026-06-30
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `editor-ux` seq `3`
**Parent:** `HANDOFF_editor-ux_action-toasts_2026-06-30.md` (`editor-ux` seq 2)

> Chain rationale: continuing editor user-friendliness. User said "다음 후보 계속" → the next listed
> editor-UX candidate was entity-list QoL; I picked its highest-value, headless-verifiable sub-item:
> **per-entity visibility** (an eye toggle + a `Hidden` component). `editor-ux` seq 3.

## Related Handoffs

- `HANDOFF_editor-ux_action-toasts_2026-06-30.md` (seq 2) — prior editor-UX increment.
- `HANDOFF_editor-ux_keyboard-shortcuts-headless-editor-screenshot_2026-06-30.md` (seq 1) — added the
  headless editor screenshot. **Limitation surfaced here:** it captures **overlay** mode, but the
  entity list (where the eye toggle lives) is **docked**-only → the toggle glyph wasn't headless-verified
  (docked-mode headless capture remains the tracked follow-up).

## Reference Documents

- `CLAUDE.md` — components row now documents `Hidden`; editor row notes the per-row visibility toggle.
  Header → **v1.6.180** / package **v0.93.0**.
- `docs/CHANGELOG.md` — the 0.93.0 entry.
- `~/.claude/.../memory/engine-current-state.md` — bumped to **seq 126** on this handoff PR's landing.
- `../dungeon-merchant/docs/engine-wishlist.md` — game↔engine board; **ACTIVE EMPTY** (EW-004 next).
- Key files: `src/components.rs` (`Hidden`), `src/renderer/sprite/collect.rs` (the 3 skip sites),
  `src/app/editor/ui/docked.rs` (`entities_tab_body` eye toggle).

## The Goal

Editor user-friendliness: let you hide/show an entity from the entity list without despawning it (a
classic editor feature). Engine side: a `Hidden` marker the sprite pass skips, also usable by games.

## Where We Are

- **main @ `795819e`** (package **v0.93.0**, header **v1.6.180**), tree clean.
  _(This handoff lands as its own `docs(handoff)` PR; memory → seq 126 lands with it.)_
- **PR #295 merged** (squash `795819e`, CI 5/5 green incl. lavapipe render).
- **New** `engine::Hidden` (`src/components.rs`) — `#[derive(...)] pub struct Hidden;` marker. When
  present, the sprite collect pass skips the entity in **all three loops** — `Sprite` (incl. the
  `NineSlice` 9-quad expansion), `AtlasSprite`, `ShaderMaterial` (`src/renderer/sprite/collect.rs`).
  Does **not** affect particles / lights / screen-space text (documented scope). For ShaderMaterial,
  the skip excludes it from the *draw* set but NOT the `live_material_entities_scratch` retention set,
  so a hidden material's GPU buffers are retained (only despawn/material-removal frees them).
- **Registered** clone (`core_resources.rs`) + editor add/remove (`component_registry.rs`) like
  `SpriteFlip`. (Not serde-registered — consistent with `SpriteFlip`/`RenderLayer`/`YSort`.)
- **Editor entity list** (`entities_tab_body`, `docked.rs`): each row now has a leading **eye toggle**
  (👁 visible / 🙈 hidden) that adds/removes `Hidden`; hidden rows' labels are dimmed (`RichText::weak`).
- **Tests**: render `tests/render.rs::hidden_component_suppresses_sprite` (a Hidden green quad doesn't
  render; lavapipe-verified) + unit `hidden_registered_as_editor_component` (factory adds / remover
  removes). 1000 lib + doctest 85.

## What We Tried (Chronological)

1. **"다음 후보 계속"** → board empty → picked **entity visibility** (top entity-list QoL item, and the
   most headless-verifiable: a sprite appears/disappears).
2. **Built `Hidden`**: component in `components.rs`, re-export `engine::Hidden`, skip in the 3 sprite
   collect loops (Sprite via early `continue`; AtlasSprite + ShaderMaterial via `.filter()` on the
   query), clone + editor add/remove registration.
3. **Editor eye toggle** in `entities_tab_body`: wrapped each row in `ui.horizontal`, a `small_button`
   with 👁/🙈 toggling `Hidden`, plus a dimmed label for hidden entities.
4. **Render test** (game headless): two quads, right one Hidden → asserts left reads red, right region
   is background (green gone). Passes on Metal; lavapipe on CI.
5. **doc gate caught a broken intra-doc link** (`[`AtlasSprite`]` not in scope in `components.rs`) →
   fixed to `[`AtlasSprite`](crate::AtlasSprite)`. Re-ran doc green.
6. **Verify** all gates green. **`/ship`** → v0.93.0. **`/land-pr`** → branch `feat/entity-visibility`,
   PR #295, CI 5/5, squash `795819e`, synced.

## Key Decisions

- **`Hidden` scopes to the sprite-family passes only** (Sprite/NineSlice/AtlasSprite/ShaderMaterial),
  not particles/lights/text. That's the editor's primary need (hide clutter) and keeps the change
  bounded + the fast path one cheap per-entity lookup (consistent with `SpriteFlip`/`RenderLayer`).
  A "hide everything" visibility could come later if a game asks.
- **Clone + editor add/remove, NOT serde** — mirrors the sibling marker components
  (`SpriteFlip`/`RenderLayer`/`YSort`); visibility is a runtime/editing aid, not scene-persisted.
- **ShaderMaterial: skip the draw set, keep the live (retention) set** — so hiding a material entity
  doesn't free + later rebuild its GPU buffers (only real removal does).
- **Eye toggle in the entity list** (not the inspector) — fast per-row visibility, the conventional
  editor placement; dimmed label reinforces the state.
- **Versioning MINOR (v0.93.0)** — additive, absent `Hidden` byte-identical.

## Evidence & Data

| Hash | PR | Version | Seq | Summary |
|---|---|---|---|---|
| (squashed) `795819e` | #295 | v0.93.0 | 126 | entity visibility — `Hidden` + eye toggle |

### New public API (additive)

| Symbol | Location |
|---|---|
| `engine::Hidden` (marker component) | `src/components.rs` |

### Tests

`hidden_component_suppresses_sprite` (render, lavapipe) + `hidden_registered_as_editor_component`
(unit). `test --all-targets` = 1000 lib + integration, 0 failed (2 audio skipped). `test --doc` = 85.

### CI (PR #295 — 5/5): Test (native), Render (lavapipe), Build (WASM), Rustdoc, Package dry-run.

## Gotchas & Discoveries

- **The eye-toggle glyph is NOT headless-verified this round.** The entity list lives in the **docked**
  left panel; the seq-1 headless editor screenshot captures **overlay** mode only (EngineStats +
  Inspector + windows), so it can't show the docked entity list. The `Hidden` *behaviour* is
  render-tested; the 👁/🙈 glyphs use the same NotoEmoji stack as the proven toolbar emoji
  (🗑/🔍/💾). **Docked-mode headless capture** (seq-1 follow-up) would close this gap — worth doing
  before the next editor-UI-heavy feature.
- **rustdoc `-D warnings`**: an intra-doc `[`AtlasSprite`]` in `components.rs` errored (not in scope) —
  use the full path `[`AtlasSprite`](crate::AtlasSprite)`. Run the doc gate.
- **Filter on the query, not in the body**, for AtlasSprite/ShaderMaterial — `.filter(|(e,_)| world.get::<Hidden>(*e).is_none())` keeps the skip cheap + local.
- **GUI playtests still blocked (locked screen)** — headless is the verification path, but it's
  overlay-only today (see above).
- **Environmental audio (standing):** 2 audio-device tests fail locally; `--skip`, CI gates audio.

## Files Changed (PR #295)

- `src/components.rs` — `Hidden` marker.
- `src/lib.rs` — re-export `engine::Hidden`.
- `src/renderer/sprite/collect.rs` — skip `Hidden` in the 3 collect loops.
- `src/app/core_resources.rs` — `register_clone::<Hidden>()`.
- `src/app/editor/component_registry.rs` — `Hidden` add/remove factories.
- `src/app/editor/ui/docked.rs` — `entities_tab_body` per-row eye toggle + dimmed hidden rows.
- `src/app/editor/tests.rs` — `hidden_registered_as_editor_component`.
- `tests/render.rs` — `hidden_component_suppresses_sprite`.
- `CLAUDE.md` (rows + header), `docs/CHANGELOG.md` (0.93.0), `Cargo.toml`/`Cargo.lock`.

## User Feedback & Preferences

- **"다음 후보 계속"** = keep shipping the next editor-UX candidate; **board first**, then pick.
- Continues the editor user-friendliness direction; the choice of sub-feature was delegated to me.
- **Values headless verification** — chose the most headless-verifiable QoL item; flagged honestly
  where the glyph couldn't be verified (overlay-only capture).
- Korean for user-facing replies; English for code/docs/handoffs. Merge authority delegated.

## Where We're Going

Editor UX now has: keyboard shortcuts + cheatsheet (seq 1), action toasts (seq 2), entity visibility
(seq 3). Candidates next, by real-use friction:

1. **Docked-mode headless capture** — would let the eye toggle / entity list / Data Tables / Tile
   Paint be headless-verified + golden-tested. The clearest infra gap now (blocks visual verification
   of docked-panel features). Higher priority after this seq surfaced the limitation.
2. **Entity-list QoL continued**: type icons, inline rename, drag-to-reparent.
3. **Esc-to-deselect** (dropped twice as risky) — only with clear gating.
4. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004) — a real game
   request outranks self-picked polish.

Otherwise: **ASK the user for direction** when the board is empty.

## Risks & Blockers

- **Eye-toggle glyph visually unverified** (overlay-only headless capture) — low risk (proven font
  stack); docked-mode capture would confirm it.
- **`Hidden` is sprite-family only** — doesn't hide particles/lights/text; documented. A fuller
  visibility is a future option.
- No OS-gated-CI risk (renderer skip is cross-platform, lavapipe-verified; editor native-only). Clean.

## Open Questions

- Should `Hidden` also suppress particles/lights/text (a "full" visibility)? Deferred — sprite-family
  covers the editor's need; extend on a concrete request.
- Should visibility persist in saved scenes (serde-register `Hidden`)? Deferred — consistent with the
  non-serde sibling markers; revisit if a game wants persistent-hidden spawns.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5            # main tip = this handoff PR; feature = #295 (795819e)
git status -s                   # clean

# 1) Board FIRST
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick

# Memory: ~/.claude/.../memory/engine-current-state.md  (seq 126 tip)

# Key files
#   src/components.rs (Hidden) · src/renderer/sprite/collect.rs (skip sites)
#   src/app/editor/ui/docked.rs (entities_tab_body eye toggle)
#   src/app/headless.rs (headless editor screenshot — OVERLAY only; docked capture is the next infra gap)

# Verify (2 audio tests fail locally — env; read exit via echo $?, not ${PIPESTATUS[0]} in zsh)
cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Next: read the wishlist; if empty, ASK. Strong next infra candidate: DOCKED-mode headless capture
# (unblocks visual verification of docked-panel editor features like the eye toggle).
```

## Session Closed

**Closed:** 2026-06-30
**Chain:** `editor-ux` seq 3 — continuation of seq 2.
**Code landed:** #295 (v0.93.0), main @ `795819e`. This handoff lands as its own `docs(handoff)` PR; memory → seq 126 with it.
**Session status:** Handed off. The editor gained per-entity visibility (eye toggle + `Hidden` component), render-verified. Surfaced the docked-mode headless-capture gap as the next infra priority. Next session starts from the wishlist board or asks for direction.
