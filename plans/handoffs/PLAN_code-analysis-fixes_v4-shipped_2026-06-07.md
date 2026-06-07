# Land the rust-survivors v4 migration + visually verify the new engine APIs in real play

**Date:** 2026-06-07
**Status:** PLANNED
**Bead(s):** none (`bd` unavailable)
**Epic:** code-analysis remediation (`docs/CODE_ANALYSIS.md`) — now CLOSED; this plan is the verification/landing tail.
**Chain:** `code-analysis-fixes` seq `3`
**Context:** See `HANDOFF_code-analysis-fixes_v4-shipped_2026-06-07.md` for full session data — commit table, the #15 derivation, the 12 rust-survivors Color sites, subsystem invariants, and the verification reference.

---

## Problem Statement

The code-analysis epic is complete: `skeleton-engine` `main` is v4.0.0 (30/30 issues, PRs #7–#11 merged) and `rust-survivors` builds + passes 200 tests against it. Two things are unfinished. (1) The rust-survivors migration commit (`da11775`) is **local-only** — not pushed. (2) Everything new this session — `DrawText::centered`/`TextAnchor`, `Camera::world_to_screen`, `AudioSystem` fades, and the rust-survivors Color migration — has only been **compile- and unit-tested**, never run on screen. The engine's VISION explicitly sets the bar at "exercised in real play," and the #15 lighting conclusion, while math-verified twice, was never eyeballed. This plan lands the migration and closes the visual-verification gap.

## Key Findings

- rust-survivors migration is isolated in `da11775` (9 files); its repo carries **21 unrelated WIP files** on `main` — do not bundle them. → drives Phase 1.
- The migration's only impact was #11 Color; PhysicsWorld (#13) is inert (game owns a raw `PhysicsWorld`) and joints (#28) are unused → low risk, safe to push. → Phase 1.
- New rendering APIs (`DrawText::centered` measured from the shaped buffer; `world_to_screen` nameplates that track world entities as the camera moves) are visual behaviors unit tests can't fully confirm. → drives Phase 2.
- `audio_fades` is the only place `AudioSystem`-driven fades run; audio is audible-only (no screenshot) — verify via no-crash + on-screen HUD status, and by ear if possible. → Phase 2.
- #15 (lighting) is a confirmed false positive (UV-space math + independent re-derivation + contract test); a `lit_dungeon`/`security_camera` run would be the final eyeball but is NOT required to trust it. → optional in Phase 2.
- Only the 5 #27 examples were English-ified; the broader repo-wide conversion is a separate tracked effort (`docs/english-conversion` branch). → drives the optional Phase 3.

## Anti-Goals (What NOT To Do)

- **Do NOT re-"fix" #15 lighting.** The CPU `radius*zoom/viewport_w` is already in the shader's UV space; `2*radius/viewport_w` would double light size. The contract test guards it. (Handoff → Key Decisions / Subsystem invariants.)
- **Do NOT bundle rust-survivors' 21 WIP files** (powerup.rs, README.md, docs) into anything. Push only what's already in `da11775`.
- **Do NOT re-introduce `clamp(0,31)` in `layer_matches_mask`**, re-add the `Arc<Mutex>` script buffers, change `find_path`'s signature, or auto-register `AudioSystem`. (Handoff → Subsystem invariants.)
- **Do NOT verify with default-stable `cargo`.** Always `cargo +1.88.0` (rustfmt 1.8.0); default stable fails CI.
- **Do NOT attempt the repo-wide English conversion as a quick inline pass** — it's large; fan out subagents and treat it as its own scoped effort, or skip.

## Plan

### Phase 1: Push the rust-survivors v4 migration

**Goal:** Land the already-tested migration on rust-survivors' remote without disturbing the user's WIP.

**Why this approach:** The migration is verified (200 tests) and isolated in one commit; pushing is the only remaining "landing" step. Risk is low (Color-only impact).

- Confirm with the user before pushing (outward-facing).
- `git -C /Users/jkl/Projects/rust-survivors log --oneline -2` → verify `da11775` is the migration and is the only commit ahead of the remote (the 21 WIP files must still be **uncommitted**, not in the commit).
- `git -C /Users/jkl/Projects/rust-survivors status -s` → confirm the WIP is still unstaged (don't touch it).
- Push: `git -C /Users/jkl/Projects/rust-survivors push`.
- If the remote rejects (diverged), do NOT force; fetch, inspect, and report — the user may have pushed other work.
- Optional sanity: after push, `cargo +1.88.0 test --workspace` in rust-survivors still 200-green (cached; engine pin already at `60328fa`).

**Files:** none (push only). **Validates with:** `git -C …/rust-survivors status` shows `da11775` pushed, WIP intact; remote build/CI (if any) green.
**Rollback:** nothing committed here; if the push was wrong, it's a normal commit on their `main` to revert.

### Phase 2: Visual playtest of the new APIs (engine + game)

**Goal:** Confirm the new rendering/audio APIs and the game's Color migration actually look right on screen — the VISION "real play" bar.

**Why this approach:** `centered` text, `world_to_screen` tracking, and audio fades are visual/temporal behaviors unit tests can't fully assert. Use the macOS window-capture harness (memory `playtest-windowed-examples`: osascript window bounds + screencapture + synthetic key input + caffeinate).

- **`audio_fades`** (`cargo run --example audio_fades`): window opens, HUD shows instructions + "status:" line; press Space (status → "playing"), F (status → "fading out"), 1/2/3 (status → "fading volume → …"). No crash. Audio audible if a device exists; the point is `AudioSystem` advances the fade (status changes + no freeze).
- **`minimap`** (`cargo run --example minimap`): green player + red enemies + minimap inset; an **"ENEMY" nameplate sits above each enemy and tracks it** as you move (WASD) — confirms `world_to_screen`. Labels are centered over enemies — confirms `DrawText::centered`.
- **`loading_bar`** (`cargo run --example loading_bar`): the "Loading… N%" text is **horizontally centered on the bar** (no eyeballed offset), then the centered "Loading complete!" — confirms `centered`.
- **rust-survivors** (`cargo run -p game --bin survivor`, or its documented run): enemies render in their expected colors; **hit-flash turns an enemy white then restores its color** (exercises `damage.rs`/`weapon.rs` Color conversions). No magenta/black mis-tints (would indicate a bad Color conversion).
- **(Optional, #15 eyeball)** `cargo run --example lit_dungeon` or `security_camera`: point lights look full-size, not halved/doubled. Not required — #15 is math-confirmed.
- Capture a screenshot per example; note any visual regression with the example name + what looked wrong.

**Files:** none (run only); fix-forward only if a real visual bug appears (then it's a new engine change → its own commit + full `+1.88.0` gate).
**Validates with:** screenshots showing centered text, tracking nameplates, correct game colors; `audio_fades` HUD status cycles without freeze.
**Rollback:** N/A (read-only verification). If a bug is found and fixed, revert that specific fix.

### Phase 3 (OPTIONAL): repo-wide example Korean→English comment conversion

**Goal:** Extend #27's English-comment conversion from the 5 done examples to the rest of `examples/` (the remaining top-level + `games/*`).

**Why this approach:** Aligns all examples with the doc-language rule; mechanical and parallelizable. Only do this if the user wants it — it's tracked separately on `docs/english-conversion` and is pure churn (comments only).

- Enumerate remaining Korean-commented examples: `for f in $(git ls-files 'examples/*.rs'); do n=$(grep -cP '[가-힣]' "$f"); [ "$n" -gt 0 ] && echo "$f: $n"; done`.
- Fan out one Sonnet subagent per file (or per game dir) with the exact #27 C instructions: translate ONLY comments (`//`/`///`/`//!`), preserve box-drawing dividers and prefixes, touch no code/identifiers/string-literals, no formatter.
- Independently verify: `grep -P '[가-힣]'` = 0 in touched files; `cargo +1.88.0 fmt --check`, `clippy --all-targets`, `build --examples`; confirm the diff is comments-only (`git diff … | grep '^[-+]' | grep -v '^[-+]\{3\}' | grep -vE '//'` empty-ish, accounting for trailing comments).
- One commit `docs(examples): English-ify remaining Korean comments`. Coordinate with the `docs/english-conversion` branch to avoid conflicts.

**Files:** many `examples/**/*.rs` (comments only). **Validates with:** 0 Korean in touched files; full `+1.88.0` example gate green; comments-only diff.
**Rollback:** revert the single commit.

## Dependencies & Order

- Phase 1 and Phase 2 are independent; do **Phase 1 first** (quick landing) then **Phase 2** (the substantive verification). Either could be skipped if the user only wants one.
- Phase 3 is optional and independent of 1/2; do last if at all.
- Phase 2's rust-survivors run uses the engine pin already at `60328fa` (no dependency on Phase 1's push).

## Risks & Mitigations

- **rust-survivors remote diverged** (user pushed elsewhere). Likelihood: low. Mitigation: fetch + inspect, never force-push; report.
- **A real visual bug surfaces in Phase 2** (e.g., centered text mis-measured, nameplate offset wrong, a bad Color conversion). Likelihood: low (unit-tested). Mitigation: fix-forward as a new engine commit through the full `+1.88.0` gate + a PR; don't hot-patch on `main`.
- **Playtest harness flakiness on macOS** (window focus, key injection). Likelihood: medium. Mitigation: follow memory `playtest-windowed-examples`; fall back to manual run + visual description if automation fails.
- **Phase 3 churn conflicts with `docs/english-conversion`.** Likelihood: medium if both move. Mitigation: check that branch first; only one should own the conversion.

## Success Criteria

- **Minimum viable:** Phase 1 done — `da11775` pushed, rust-survivors WIP untouched, remote clean.
- **Full success:** Phase 2 screenshots confirm centered text, tracking `world_to_screen` nameplates, correct rust-survivors colors + white hit-flash, and a non-frozen `audio_fades` HUD — with no visual regressions vs. the prior look.
- **Stretch:** Phase 3 — 0 Korean comments left across `examples/`, comments-only diff, full gate green.
- Baselines to compare against: engine 311 lib tests / 34 doctests; rust-survivors 200 tests (handoff → Evidence).

## Quick Start

```bash
cd /Users/jkl/Projects/skeleton-engine

# Restore full context
cat plans/handoffs/HANDOFF_code-analysis-fixes_v4-shipped_2026-06-07.md

# Confirm engine starting state (PINNED toolchain)
git log --oneline -3            # 60328fa, v4.0.0
cargo +1.88.0 test --all-targets   # 311 lib, 0 fail

# Phase 1 — inspect the rust-survivors migration before pushing
git -C /Users/jkl/Projects/rust-survivors log --oneline -2     # da11775 = migration
git -C /Users/jkl/Projects/rust-survivors status -s            # 21 WIP files must stay uncommitted

# First concrete action (Phase 1, after user confirms the outward-facing push):
git -C /Users/jkl/Projects/rust-survivors push

# Phase 2 — playtest (memory: playtest-windowed-examples)
cargo run --example minimap        # ENEMY nameplates track enemies (world_to_screen + centered)
cargo run --example loading_bar    # centered loading text
cargo run --example audio_fades    # Space/F/1-2-3 → HUD status cycles, fade not frozen
```
