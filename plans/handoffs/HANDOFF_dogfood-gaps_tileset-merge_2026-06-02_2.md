# Platformer tileset finalized + PR #3 merged (v1.1.0 dogfooding epic DONE)

**Date:** 2026-06-02
**Status:** COMPLETED
**Bead(s):** none
**Epic:** skeleton-engine playable-examples dogfooding loop
**Chain:** `dogfood-gaps` seq `2`
**Parent:** `HANDOFF_dogfood-gaps_2026-06-02.md`
**Prior chain:** `HANDOFF_dogfood-gaps_2026-06-02.md` > this

## Since Last Handoff

Parent (seq 1) closed all 4 v1.1.0 gaps and left ONE open item: the platformer
tileset decision (keep generated / revert render / external asset), blocking PR #3 merge.

- **Open question resolved.** Grilled the tileset decision in 2 rounds. User chose **A: keep the generated `platform_tiles.png`** (no render-path change, no external asset, no run-merge), **+ commit the generator** for reproducibility, **+ keep the current art** as-is.
- **Reproducibility gap closed.** The PNG had been made by a deleted throwaway script (flagged as a risk in parent). Added `examples/gen_platform_tiles.rs` — deterministic, regenerates the committed PNG byte-for-byte.
- **PR #3 merged.** All 4 CI checks passed (WASM, Package dry-run, Rustdoc, Test native); merged via merge commit `4a0658c`; feature branch deleted; local on `main`, synced.
- **Trajectory:** the whole dogfooding epic is now DONE and on `main`. No follow-on planned.
- Parent's other "Where We're Going" items (user confirmation, optional doc update) also done.

## Reference Documents

- `CLAUDE.md` — conventions, module map (v1.1.0)
- `docs/VISION.md` — dogfooding core loop
- `docs/NEXT_WORK.md` — candidate×example matrix; A–F + the 4 gaps all marked done
- `docs/HANDOFF.md` — per-phase dev history (v1.1.0 entry added in parent session)
- `plans/handoffs/HANDOFF_dogfood-gaps_2026-06-02.md` — parent (full detail of the 4 gaps)

## The Goal

Close out the v1.1.0 "playable-example dogfooding gaps" work by resolving the last
open item (platformer tile rendering) and landing PR #3 on `main`. The engine fixes
were already done and validated in the parent session; this session was about the
final visual/asset decision, making the generated tileset reproducible, and merging.

## Where We Are

- **PR #3 MERGED** to `main` (`4a0658c Merge pull request #3 from ChunSam/feat/example-dogfood-gaps`). Working tree clean, on `main`, synced with origin. Feature branch deleted (local + remote).
- **v1.1.0 shipped on main:** #1 Blackboard path caching, #4 persistent resources, #3 tilemap→physics + `TileCollider`, #2 one-way platforms + drop-through (full API list in parent).
- **Platformer tileset = generated `platform_tiles.png`** (seamless, textured, no per-tile borders), reproducible via the new generator example.
- **`examples/gen_platform_tiles.rs`** added — deterministic generator (128×128, 4×4, 32px cells): ground idx0 (grass cap + speckled dirt), stone idx1 (top highlight / bottom shadow / cracks, NO border), one-way idx8 (wood planks + seams + grain). Writes to `examples/games/platformer/assets/platform_tiles.png`. Manual tool (not run by CI), run from repo root.
- **Verified deterministic:** `cargo run --example gen_platform_tiles` leaves `git status` clean (byte-identical regen) — confirmed twice (once after a `cargo fmt`).
- Provenance documented: comment in `platformer.rs` near the tile load + a line in `docs/CHANGELOG.md` 1.1.0.
- User confirmed the platformer renders correctly after the seamless-tileset fix (ran it before saying "머지 진행").
- Test/lint state unchanged: lib 253 + doctest 34 pass; `cargo clippy --all-targets -- -D warnings` clean (now includes the generator example); fmt clean; wasm lib + maze/scene_flow examples build.
- `docs/NEXT_WORK.md`: the playable-examples program (A–F) + the 4 gaps are all done; no remaining planned candidate.

## What We Tried (Chronological)

1. **Diagnosed the platformer "separated blocks" report** (carried from parent): pixel-analyzed `tiles.png` (throwaway `examples/_tilecheck.rs` using the `image` crate) → the original art is discrete object sprites with per-cell transparent margins (ground idx0 fills only `x[7..30] y[6..31]`, one-way idx8 `y[3..24]`), not a seamless tileset.
2. **Generated a flat seamless tileset** (commit `ee0a424`). User re-ran → still looked like separated boxes + "png not applied correctly".
3. **Proved it wasn't a spacing bug:** headless `TilemapSystem` dump showed tiles at `x=32/96/160, scale 64` — contiguous. The separation came from (a) a 1px border I'd drawn on the stone tile (formed a dark grid line) and (b) flat colors reading as placeholder boxes; the big gaps between groups are the level's intended jump gaps.
4. **Regenerated textured, border-free tileset** (commit `2b622f2`): top-highlight/bottom-shadow + deterministic speckle/grain, no side borders. User asked "why not reuse `tiles.png`" — explained the discrete-sprite finding and offered 3 options.
5. **Grilled the decision (this session, 2 rounds):** round 1 → option A (keep generated). Round 2 → commit generator for reproducibility + keep current art. Wrote the continuation plan to `/Users/jkl/.claude/plans/4-glistening-catmull.md`.
6. **Implemented:** added `examples/gen_platform_tiles.rs` (cleaned-up version of the throwaway: `match` arms, removed the t=1.0 `lerp`, underscore-separated hash constants — same pixels), added provenance comment + CHANGELOG line.
7. **Verified + committed `771d017` + pushed.** Then checked PR #3 (`MERGEABLE`, `CLEAN`, 4 CI checks pass) and **merged** (`gh pr merge 3 --merge --delete-branch`).

## Key Decisions

- **Option A (keep generated tileset)** over reverting render to stretched `AtlasSprite` (B), external seamless tileset (C), or grid→merged-run rendering (D): keeps the "level = one Tilemap drives render + collision" dogfooding story with zero extra render code; the original `tiles.png` simply isn't tileable.
- **Commit the generator as an example** rather than leaving an opaque binary: skeleton-engine is fork-friendly, so a reproducible asset recipe beats an unexplained committed PNG. Rejected: build-script generation (would run every build) and leaving the PNG sourceless.
- **Keep current art** (no further polish): no seams, gameplay correct; procedural simplicity accepted as intentional.
- **Merge commit** (not squash) to match the repo's `#2` convention and preserve the 6-commit history.

## Evidence & Data

### Commits added this session (then merged via `4a0658c`)

| Hash | Summary |
|---|---|
| `cf900ad` | docs: add session handoff (parent, seq 1) |
| `771d017` | chore(platformer): commit the seamless tileset generator as an example |
| `4a0658c` | Merge pull request #3 from ChunSam/feat/example-dogfood-gaps |

### PR #3 final state

```
state=MERGED  mergeable=MERGEABLE  mergeStateStatus=CLEAN  base=main
CI: Build (WASM) pass | Package dry-run pass | Rustdoc pass | Test (native) pass
```

### `tiles.png` per-cell opaque bbox (why it's not a tileset; 128×128, 4×4)

| idx | opaque x | opaque y | note |
|---|---|---|---|
| 0 ground | 7..30 | 6..31 | left gap 7px, right-shifted |
| 1 block | 5..29 | 17..31 | art only lower half |
| 8 one-way | 7..30 | 3..24 | sits high, ≠ idx0 vertical range |

### Determinism check

`cargo run --example gen_platform_tiles` → `git status -s …/platform_tiles.png` empty (byte-identical), verified after a `cargo fmt` too.

### Verification (final)

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 warnings (incl. gen example) |
| `cargo test --lib` | 253 passed |
| `cargo build --target wasm32-unknown-unknown --lib` | builds |
| generator byte-identical regen | yes |

## Files Changed (this session)

### Examples
- `examples/gen_platform_tiles.rs` — NEW. Deterministic tileset generator (image crate). Manual tool; writes `platform_tiles.png`.
- `examples/games/platformer/platformer.rs` — one comment line noting the tileset is generated by the example.

### Docs
- `docs/CHANGELOG.md` — 1.1.0 entry notes `platform_tiles.png` is reproducible via the generator and that `tiles.png` is discrete object sprites.
- `plans/handoffs/HANDOFF_dogfood-gaps_2026-06-02.md` — parent handoff (committed `cf900ad`).

### Removed
- `examples/_tilecheck.rs` — throwaway (pixel analysis / headless dump / first generator); never committed.

## User Feedback & Preferences

- Cares that examples *look* right, not just compile (drove the whole tileset thread).
- After understanding the root cause, chose to **keep the generated tileset** rather than chase the original art — pragmatic.
- Wanted the generator **kept in-repo** (reproducibility / fork-friendliness matters to them).
- "머지 진행" — wanted the PR merged once visuals confirmed; comfortable delegating the `gh` merge.
- Throughout the epic: preferred editing existing examples over new ones, and decisive bounded choices via grilling.

## Where We're Going

- **Nothing queued.** The v1.1.0 dogfooding epic is complete and merged. `docs/NEXT_WORK.md` shows no remaining planned candidate.
- If a new direction is chosen later, likely candidates (all previously deferred/cancelled, NOT scheduled): Entity Generation v2 (breaking, `docs/ENTITY_GENERATION_V2_PLAN.md`), dependency-security follow-up (`docs/SECURITY_HARDENING_2026_05.md`), or `add_static_from_tilemap` row-merging optimization.

## Risks & Blockers

- None. Working tree clean, on `main`, PR merged, CI green.

## Open Questions

- None. The only open item (tileset) is resolved.

## Quick Start for Next Session

```bash
# State: on main, PR #3 merged, v1.1.0 shipped. Nothing in flight.

# Reference docs
#   docs/NEXT_WORK.md (epic done), docs/VISION.md, CLAUDE.md
#   plans/handoffs/HANDOFF_dogfood-gaps_2026-06-02.md (parent — full gap detail)

# Verify current state
git switch main && git pull
cargo test --lib            # expect 253 passed
cargo clippy --all-targets -- -D warnings

# Regenerate the platformer tileset if ever needed
cargo run --example gen_platform_tiles   # deterministic; should leave git clean

# Next action
# No work is queued. Await a new direction from the user. If asked to continue the
# engine, the deferred candidates (Entity Gen v2 / dep-security / collider row-merge)
# are the documented starting points — none is currently scheduled.
```
