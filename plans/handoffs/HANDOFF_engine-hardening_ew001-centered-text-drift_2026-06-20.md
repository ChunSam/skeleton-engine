# EW-001 fixed — `DrawText::centered` horizontal drift (first wishlist-driven engine change)

**Date:** 2026-06-20 (KST)
**Status:** COMPLETED — PR #159 merged + green; `main` @ `90b9cea`, package **v0.43.6**, tree clean.
**Bead(s):** none (repo uses `plans/handoffs/`)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `47`
**Parent:** `HANDOFF_engine-hardening_reference-api-gaps_2026-06-20.md` (seq 46)

> First engine change **driven by the dungeon-merchant wishlist board**. seq-46 registered the board and
> confirmed it empty (next ID EW-001); this session the game had filed **EW-001** (centered-text drift), and
> I implemented the engine-side fix end-to-end: root-caused, fixed the anchor-offset math, added headless
> regression tests, shipped v0.43.6 (PATCH, bugfix), merged green, and replied on the board with status
> `Shipped (v0.43.6)`. The board now awaits game-side verification (ball is with the game).

---

## The Goal

Implement wishlist **EW-001**: a no-bounds `DrawText::centered` (anchor=Center + align=Center) rendered its
horizontal center **~half the viewport to the right** of `position` whenever `position.x` was off-center
(game hit it on inventory item labels centered on bag cells). Acceptance (game's words): "With no `bounds`, a
`DrawText` with `anchor = Center` has its shaped center at `position` for arbitrary `position.x`. Ideally a
regression test asserting the rendered center matches `position` at an off-center x."

## Where We Are

- `main` @ **`90b9cea`** (PR #159), tree clean, CI 4/4 green. Package **v0.43.6** (PATCH — bugfix, no API change).
- **PR #159** (`fix/ew001-centered-text-drift` → squash-merged, branch deleted), +182 / −15 across 6 files:
  `src/renderer/text/renderer.rs` (the fix), `src/renderer/text/tests.rs` (2 regression tests), plus the
  v0.43.6 ship paperwork (`Cargo.toml`/`Cargo.lock`/`docs/CHANGELOG.md`/`CLAUDE.md` header → v1.6.100).
- **Wishlist board** (`../dungeon-merchant/docs/engine-wishlist.md`): EW-001 status flipped `Proposed` →
  **`Shipped (v0.43.6)`**, `[Engine]` thread reply appended. **Edit left UNCOMMITTED in the game repo** (that
  repo had its own unrelated dirty state on `main` — `Cargo.lock`, `.claude/`; the game session owns its
  commits). Ball is now with the game: pull/bump the path dep, verify, set `- [x]`.

## Root Cause (verified, not guessed)

`DrawText::centered` sets `anchor = Center` **AND** `align = Center` (`src/renderer/text/queue.rs:69-80`).
A prior "Fix 2" made the layout buffer the **full viewport width** for anchor=Center (so centered titles
don't wrap early — `layout_buffer_width`, `renderer.rs:~437`). The interaction nobody caught:

- `align = Center` makes glyphon center each line around the **buffer** center → glyph block center sits at
  buffer-local `viewport_w / 2`.
- But the anchor offset subtracted `max_w / 2` (`renderer.rs`, old code) — a **left-aligned** assumption
  (valid only when glyphs start at buffer-local x=0).
- Net: rendered center = `position.x + (viewport_w − max_w)/2` → drift ≈ half the viewport. Exactly the
  game's "~half a viewport to the right."

The game's workaround (anchor=Center + **default Left** align) works precisely because Left-aligned glyphs
*do* start at x=0, so `max_w/2` is the correct offset there.

## The Fix

`renderer.rs`: new `pub(super) fn shaped_center_x(&Buffer) -> f32` measures the **actual** rendered
horizontal center from glyph extents (`min(glyph.x) .. max(glyph.x + glyph.w)`). The `TextAnchor::Center`
anchor offset now uses it instead of `max_w / 2`. Properties:

- **align=Center** → measured center = buffer center (`viewport_w/2`) → rendered center lands on `position.x`. ✓
- **align=Left** (the workaround) → glyphs at x≈0 → measured center reduces to `max_w/2` → **byte-identical
  to before**, no regression. ✓
- Correct for **every** align (Left/Center/Right/End/Auto) — it measures where glyphs landed, not an assumption.
- **Kept `align = Center`** (did NOT take the game's "drop align" suggestion): dropping it would left-align
  the lines of a multi-line centered title. Measuring the real center fixes position *and* preserves per-line
  centering.

## Key Decisions

- **PATCH bump (0.43.5 → 0.43.6), not MINOR.** Per CLAUDE.md 0.x rule "PATCH = bugfix": this is a pure
  behavior fix, no public API surface change. (MINOR is for features/breaking.)
- **Measure glyph extents rather than special-case align.** More robust than branching on align with
  `line_w`; handles RTL/Auto for free; no second shape pass (the buffer is already shaped when the offset is
  computed). The shaped-buffer cache is unaffected (its key already includes `align`/`anchor`).
- **No new example.** This is a bugfix to existing API (already exercised by examples), and the game asked
  specifically for a **regression test** — that is the acceptance here. The one example using
  `with_anchor(TextAnchor::Center)` (`examples/games/stat_editor_game/stat_editor.rs:183`) uses default Left
  align → on the unchanged path, so no example needed updating. (Optional visual confirmation deferred — see
  Where We're Going.)
- **Did not commit the cross-repo board edit.** The game repo manages its own commits; left the status/reply
  in its working tree.

## Evidence & Data

### Tests (acceptance) — both green, 885 lib tests total
- `ew001_centered_text_center_lands_on_position_x` — shapes "Hello" (anchor=Center, align=Center) at an
  **off-center** `px=150` in 800×600; asserts measured center ≈ `vw/2` (within 2px), the new offset lands the
  center on `position.x` (within 0.5px), and the **old `max_w/2` offset drifts right by > vw/4** (regression guard).
- `ew001_left_align_center_anchor_unchanged` — asserts the Left-align workaround's measured center reduces to
  `max_w/2` (within 1px) → unchanged.
- Both shape headlessly with the bundled `assets/fonts/DejaVuSans.ttf` (cosmic-text shaping is CPU-only — no
  GPU). Helper `shape_no_bounds` mirrors the render path (full-viewport buffer, `set_align`, `shape_until_scroll`).

### Verify gate (local, before push) + CI (PR #159)
```
./scripts/verify.sh → VERIFY_EXIT=0   (fmt / clippy -D warnings / wasm lib+bins / test --all-targets / rustdoc)
CI 4/4 green: Rustdoc 35s · Build(WASM) 1m15s · Package dry-run 1m0s · Test(native) 3m20s
```

### Verified source coordinates (for the next editor)
| Item | file:line | fact |
|---|---|---|
| `DrawText::centered` | `src/renderer/text/queue.rs:69-80` | sets `anchor = Center` **and** `align = Center` |
| anchor offset (fixed) | `src/renderer/text/renderer.rs` (`TextAnchor::Center` arm) | now `Vec2::new(shaped_center_x(&buf), lines*size*1.2*0.5)` |
| `shaped_center_x` | `src/renderer/text/renderer.rs` (above `layout_buffer_width`) | `min(g.x)..max(g.x+g.w)` center; `0.0` if no glyphs |
| `LayoutGlyph.x/.w` | cosmic-text 0.18.2 `src/layout.rs` | `pub x: f32`, `pub w: f32` (advance) — both public |
| unchanged example | `examples/games/stat_editor_game/stat_editor.rs:183` | `with_anchor(TextAnchor::Center)` + default Left align |

## Files Changed
- **PR #159 (engine repo):** `src/renderer/text/renderer.rs`, `src/renderer/text/tests.rs`, `Cargo.toml`,
  `Cargo.lock`, `docs/CHANGELOG.md`, `CLAUDE.md` (header v1.6.100 / package v0.43.6).
- **Cross-repo (uncommitted, game repo working tree):** `../dungeon-merchant/docs/engine-wishlist.md`
  (EW-001 → `Shipped (v0.43.6)` + `[Engine]` reply).
- **Memory:** `engine-current-state.md` refreshed to v0.43.6 / seq 47.

## User Feedback & Preferences
- The wishlist board **is** the front door now — this session is the proof: read board → pick the filed
  EW-NNN (no backlog-style confirm needed for a real EW item) → implement → ship → reply on the board.
- Standing prefs (unchanged): Korean user-facing reports; English code/handoffs/sub-agent prompts;
  source-verify before writing; merge standing-delegated (squash on green CI); honest gap-naming.

## Where We're Going
1. **EW-001 verification (game's court).** The game should bump its `skeleton-engine` path dep / pull v0.43.6,
   confirm inventory labels center correctly, and set `- [x]` (then move EW-001 to Done/archive). If it can
   switch `centered_text` → `DrawText::centered`, both are correct now.
2. **Optional visual confirmation (deferred).** The fix is proven by deterministic math tests; there is no
   example rendering `DrawText::centered` at an off-center x to eyeball. If desired, add a tiny example (or a
   line to an existing one) + `wasm_smoke`/windowed playtest. Low priority — the test is the acceptance.
3. **Watch the board for EW-002+.** Next free ID is EW-002. Each engine session: read
   `../dungeon-merchant/docs/engine-wishlist.md` first.
4. **Engine-hardening backlog (unchanged, needs a user go):** crates.io publish (irreversible; publish
   `engine_reflect_derive` too), per-OS gamepad input + the deferred analog-stick Y-sign hardware test.
5. **Optional polish:** seq-43 focus-ring corner-radius/pulse. Low value.

## Risks & Blockers
- **None outstanding** — PR merged green, tree clean (engine repo).
- The board edit lives in `../dungeon-merchant` and is **uncommitted** there by design — if that repo is
  reset/cleaned, the EW-001 status reply would be lost. The game session is expected to commit it.
- HTML/REFERENCE not touched this session (no QA-scan needed).

## Quick Start for Next Session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3           # 90b9cea (#159 EW-001 fix) … 7406bb2 (#158 seq-46 close)
grep -m1 '^version' Cargo.toml # 0.43.6
git status -s                  # clean

# FIRST: check the wishlist board for new/updated requests
sed -n '53,70p' ../dungeon-merchant/docs/engine-wishlist.md  # EW-001 = Shipped (v0.43.6); next free ID EW-002

# If touching text centering again, the regression tests + helper live here:
#   src/renderer/text/renderer.rs  → shaped_center_x()
#   src/renderer/text/tests.rs     → ew001_* (headless shaping via bundled DejaVu Sans)
cargo test --lib renderer::text::tests::ew001   # both green
```

## Cross-cutting Gotchas (expensive-to-rediscover)
1. **anchor=Center offset must match the alignment.** The anchor offset is "where is the text's center inside
   the (possibly full-viewport) buffer." For left-aligned glyphs that's `max_w/2`; for `align=Center` it's the
   buffer center; etc. Assuming `max_w/2` for all aligns is the EW-001 bug. `shaped_center_x` measures it from
   glyph extents so it's align-agnostic — reuse it, don't reintroduce a per-align assumption.
2. **cosmic-text shaping is GPU-free → headless text tests are possible.** Build a `FontSystem` via
   `build_font_system(&font_bytes, &[])`, shape into a `Buffer`, inspect `layout_runs()`/glyph `x`/`w`. Use the
   bundled `assets/fonts/DejaVuSans.ttf` for deterministic Latin metrics across platforms (CI is Linux).
3. **Poll `gh pr checks <n>` before `--watch`.** `--watch` exits immediately if run <~30s after `pr create`.
   Loop until a check registers, THEN `--watch`. (Held again: checks registered after ~4s here.)
4. **Cross-repo board write is intentional but uncommitted.** The engine session edits the game repo's
   `engine-wishlist.md` to set status/reply, but does NOT commit there (the game owns its commits). Don't
   `git add`/commit in `../dungeon-merchant`.
5. **Code change → bump version (`/ship`).** Unlike the docs-only seq-45/46 PRs, this touched `src/` → full
   four-file paperwork (Cargo.toml/lock + CHANGELOG + CLAUDE.md header) + verify re-run. PATCH for a bugfix.

---

## Session Status
**Goal met** — EW-001 fixed and shipped (v0.43.6, PR #159 merged green), regression tests added, board replied
`Shipped`. `main` @ `90b9cea`, tree clean. Handed off to next session (seq 48). Ball on EW-001 is with the game.

## EW-001 Closed by Game (post-handoff update)
The game session **verified EW-001 on v0.43.6** and closed it — board status `Verified`, heading `- [x]`,
moved to **Done / archive** (`../dungeon-merchant/docs/engine-wishlist.md`). Game's `[Game] 2026-06-20`
reply: "Verified on v0.43.6 (Cargo.lock bumped, game builds + boots clean). Engine tests
`ew001_centered_text_center_lands_on_position_x` and `ew001_left_align_center_anchor_unchanged` both pass —
the latter confirms our `centered_text` (anchor=Center + default Left align) is unchanged, so we're keeping it
as-is." **Active requests on the board are now empty; next free ID is EW-002.** The full engine↔game loop
(file → ship → verify → close) completed within the same day — the wishlist workflow is proven end-to-end.

## Session Closed
**Closed at:** 2026-06-20 (KST)
**Commits:** #159 (`90b9cea`, EW-001 fix v0.43.6) → handoff #160 (`4034c0e`) → this close marker.
**Session status:** Goal met, EW-001 verified+closed by the game. Handed off to next session (seq 48).
