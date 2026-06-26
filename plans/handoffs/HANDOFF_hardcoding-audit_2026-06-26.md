# Hardcoding audit → Tier-1 fixes: 4 PRs (clear-color/panel-z dedupe, PathGrid silent-fail, Tilemap layer z, native tone de-click)

**Date:** 2026-06-26
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `hardcoding-audit` seq `1`
**Parent:** none (new chain — a fresh work stream, not a continuation of `codebase-cleanup`)
**Related handoffs:** `HANDOFF_codebase-cleanup_2026-06-25.md` (the immediately prior session; its "Where We're Going" said board-empty → ASK + lean feature work — this session is the user's answer to that ASK, but it is its own audit-driven chain, not a cleanup continuation).

---

## Since Last Handoff

The `codebase-cleanup` seq-2 handoff (2026-06-25) ended with the wishlist board ACTIVE EMPTY, main @ `947cd25` (v0.68.4), and a directive: read `../dungeon-merchant/docs/engine-wishlist.md` FIRST; if still empty, ASK; cleanup has reached diminishing returns → lean feature work.

This session followed that exactly: re-read the board (still ACTIVE EMPTY, EW-004 next), ASKED the user via `/remote-control`. The user did NOT pick from the offered cleanup/feature options — they chose a custom direction: **"check the whole code for remaining hardcoding."** That kicked off a new audit-driven chain.

## Reference Documents

- `CLAUDE.md` — agent quick reference (module map, verification gate, conventions). Header now **v1.6.152**; the pathfinding + tilemap module-map rows describe this session's additions.
- `docs/HARDCODING_AUDIT_2026-06-26.md` — **NEW this session**: the full audit found→fixed record (Tier-1 resolution log + Tier-2/Tier-3 remainder catalog + won't-fix). The durable companion to this handoff.
- `docs/CODE_QUALITY_FINDINGS_2026-06-23.md` — the precedent (seq-85): a committed scan-findings doc with a resolution log. This session mirrors its format.
- `docs/VISION.md` — the forkable-skeleton north star + the feature+example acceptance loop (drove the `tilemap_layers` example for #250).
- `../dungeon-merchant/docs/engine-wishlist.md` — the downstream game↔engine board (EW-NNN). Read FIRST each session. Still ACTIVE EMPTY (EW-004 next).
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — the LIVE per-seq state, **already bumped this session to seq 96–99** (main @ `53776e5`, v0.69.1).

## The Goal

Find values baked into engine code that a fork/downstream game would reasonably want to configure (or that should be named constants), across all of `src/`, and land the genuinely-worth-fixing ones. The user explicitly scoped it as a whole-codebase audit, then — after seeing the triage — chose to land **all of Tier 1** (the verified, high-value items) as separate PRs this session. Acceptance bar throughout: no unwanted behavior change, verify gate green, CI green, and — for the one CI-unverifiable item (audio output) — human verification before merge.

## Where We Are

- **main @ `53776e5`**, working tree **clean** except the untracked `docs/HARDCODING_AUDIT_2026-06-26.md` (this session's findings doc, to be committed with this handoff). Package **v0.69.1**, CLAUDE.md header **v1.6.152**. No open PRs.
- **All four Tier-1 PRs MERGED** on green CI (4/4 each), squash-merged, branches deleted:
  - **#248** (`9da0d0d`, v0.68.5, refactor) — dedupe clear-color + panel z-offset into named constants.
  - **#249** (`eb454fc`, v0.68.6, fix) — PathGrid over-cap/overflow → `log::error!` + pub `MAX_PATH_GRID_CELLS`.
  - **#250** (`851f219`, v0.69.0, MINOR feat) — `Tilemap::z`/`with_z` per-map render depth + example `tilemap_layers`.
  - **#251** (`53776e5`, v0.69.1, fix) — native `play_tone` de-click envelope matching wasm.
- Memory `engine-current-state.md` bumped to **seq 96–99** (one seq per PR); description recall-summary + body header updated to main @ `53776e5` / v0.69.1; the Tier-2/3 remainder is recorded there too.
- The audit's **Tier-2 (fork-configurability knobs) and Tier-3 (naming/dedup) findings are NOT done** — catalogued in `docs/HARDCODING_AUDIT_2026-06-26.md` for a future session.
- Board remains **ACTIVE EMPTY** (EW-004 next).

## What We Tried (Chronological)

1. **Onboarding + board check.** Read the `codebase-cleanup` seq-2 handoff + the wishlist board → ACTIVE EMPTY. ASKED the user (`/remote-control`). User answered "Other": **check the whole code for remaining hardcoding.**
2. **Surveyed `src/`** — ~61K lines, 37 top-level files + many submodule dirs. Partitioned into 9 non-overlapping areas for parallel audit.
3. **Fanned out 9 parallel `Explore` agents** (model `sonnet`, per the explicit-model policy), each with a precise rubric: report only problematic hardcodes (no public knob / duplicated magic / baked default with no anchor), EXCLUDE test code + legit math + values that already have a knob, output `path:line — value — category — fork impact — suggested fix — knob? — priority(High/Med/Low)`. Areas: renderer; app/window/schedule; editor UI; ECS/resources/camera; physics/pathfinding/AI; audio; animation/particle/tilemap; UI/dialogue/timeline; input/network/save/scripting.
4. **Collected ~118 findings** (~29 High). Synthesized into a themed report (`scratchpad/hardcoding_audit_2026-06-26.md`, session-local): Theme A real fork blockers, B native↔wasm drift, C magic-number dup, D default-naming gaps, E game-opinionated defaults, F security/identity, G editor i18n, H asset/file paths. Sent the file to the user + a Korean digest.
5. **User chose "review details first."** I then HAND-VERIFIED the top High findings against source (didn't trust agents blind): confirmed Transform scale 64.0, MAX_LIGHTS=16 (WGSL-baked), editor app-id, PathGrid 10M silent-zero (`grid_cell_count` returns 0 on overflow/cap), tilemap cell_z fixed -1.0, the clear-color 4-site/2-form spread, the native tone path (rodio effect only applies fade_in when an effect is set), panel z-offset (capture.rs HAS the named const, panel.rs uses a raw literal). **Downgraded** the agent-flagged save AEAD key to non-issue (by-design + documented + `save_with_key` exists) and Transform scale 64.0 to naming-only (changing it is breaking). Produced a Tier-1/2/3 triage.
6. **User chose "Tier 1 전부 한 번에."** Landed 4 PRs serially (version-paperwork files conflict if parallel, so each: branch → edit → `cargo fmt` → `verify.sh` → `/ship` → re-verify → commit → push → PR → watch CI → squash-merge → sync main):
   - **#248** clear-color + panel-z dedupe (details below).
   - **#249** PathGrid silent-fail (details below).
   - **#250** Tilemap layer z + `tilemap_layers` example, native-screenshot-verified.
   - **#251** native tone de-click — implemented, opened PR, **held the merge for the user's ear** (audio output is CI-unverifiable); user ran `cargo run --example audio_facade`, pressed `T`, confirmed **"클릭없음"** → merged.
7. **Wrap.** User chose "handoff + commit the findings doc." Wrote `docs/HARDCODING_AUDIT_2026-06-26.md` + this handoff; they land together as one `docs(handoff)` PR.

## Key Decisions

- **9-agent parallel audit, not a solo sweep.** 61K lines across many subsystems — fan-out per area with a strict rubric (report only actionable hardcodes, note if a knob already exists) gave broad coverage fast. Agents were `sonnet` with explicit model (the new-model-subagent-incompat policy).
- **Verify agent findings before acting.** Agents over-flag: the save AEAD key was flagged High but is by-design/documented; Transform scale 64.0 is real but breaking-to-change. I read each High finding's source before triaging. The audit is advice; the source is truth.
- **EDITOR_SURFACE_CLEAR kept SEPARATE from DEFAULT_CLEAR_COLOR (#248).** They hold the same value today, but coupling them would mean a game changing its `WindowConfig::clear_color` repaints the editor letterbox — undesirable. So two named anchors, not one, with a comment explaining the deliberate split. The dedupe removed the *real* duplication (4 raw literals → 2 named consts) without inventing a questionable coupling.
- **EDITOR_SURFACE_CLEAR lives in `app/render/mod.rs`, not `docked.rs` (#248).** `mod docked` is `#[cfg(not(wasm32))]`, but `frame.rs:537` (one of the two docked clears) compiles on wasm — so the const must be in a cross-platform parent. Both `frame.rs` and `docked.rs` import it via `super::`.
- **PANEL_BG_Z_OFFSET source-of-truth in `panel.rs`, not `capture.rs` (#248).** The value originates from `LayoutSystem` (in panel.rs) drawing the bg at `z - 0.01`; capture.rs *mirrors* it. So the const lives where the behavior is authored; capture.rs imports it.
- **PathGrid: log, don't change behavior or break the API (#249).** Distinguished the 3 zero-return cases: `i32` overflow (#1) and over-cap (#3) are silent data loss → `log::error!`; a zero/negative dimension (#2) is a *legitimate* empty grid → stays silent. Made `MAX_PATH_GRID_CELLS` pub (additive) rather than returning `Result` (breaking).
- **Tilemap z applies to ortho/hex only; isometric ignores it (#250).** Isometric's `cell_z` is per-cell `row+col` (depth sort) — applying a base z would change iso default behavior or conflict with the -1.0 default. Scoping z to the projections that share a fixed depth keeps every existing map byte-identical and matches the finding precisely.
- **#250's example proves z, not spawn order, drives layering.** The foreground map is spawned FIRST but given the higher z — it still draws on top, because the renderer sorts by z. A naive demo (spawn order == draw order) wouldn't prove the feature.
- **#251 native tone: materialize into a SamplesBuffer.** rodio 0.19 has `fade_in` but NO source `fade_out` (only `take_crossfade_with`, which fades from the start). To get a release ramp at the END, the enveloped tone is generated into a `SamplesBuffer` and the attack+release applied to the samples directly — also makes the de-click objectively testable (no audio device).
- **#251 scope = the no-effect (default) tone path only.** The audit finding was the DEFAULT `play_tone` clicking. The `Some(effect)` branch (user explicitly configured attack/low-pass/pitch) is left byte-identical — minimal blast radius.
- **#251 NOT merged on green CI alone.** Audio output is CI-unverifiable AND I can't hear it. Per the documented gate ("OS/GPU-AV-CI-unverifiable changes override blanket merge delegation"), the PR was opened + held until the user ear-verified. The objective sample-ramp tests verify the de-click mechanically; the user's ear verifies it perceptually.
- **Version bumps:** #248 PATCH (refactor), #249 PATCH (fix), #250 MINOR (additive `with_z`/`z` public API), #251 PATCH (fix). Pre-1.0 rule: MINOR = additive feature; PATCH = refactor/fix.
- **Four separate PRs, not one.** "One PR = one coherent change." Refactor / pathfinding-fix / tilemap-feature / audio-fix are unrelated subsystems — separate PRs keep each diff + verification self-contained. (Cost: version-paperwork files conflict if branched in parallel → strictly serialized: merge + sync main before the next branch.)

## Evidence & Data

### Commits landed this session (main)

| Hash | PR | Type | Bump | Summary |
|---|---|---|---|---|
| `9da0d0d` | #248 | refactor(render,ui) | 0.68.4→0.68.5 | dedupe clear-color + panel z-offset into named constants |
| `eb454fc` | #249 | fix(pathfinding) | 0.68.5→0.68.6 | make PathGrid over-cap/overflow failure diagnosable |
| `851f219` | #250 | feat(tilemap) | 0.68.6→0.69.0 | per-map render depth Tilemap::z / with_z for layering |
| `53776e5` | #251 | fix(audio) | 0.69.0→0.69.1 | de-click native synthesized tones to match wasm |

### CI results (all 4/4 SUCCESS, squash-merged)

| PR | Test (native) | Build (WASM) | Rustdoc | Package dry-run |
|---|---|---|---|---|
| #248 | pass 6m19s | pass | pass | pass |
| #249 | pass 3m54s | pass | pass | pass |
| #250 | pass 4m42s | pass | pass | pass |
| #251 | pass 4m20s | pass | pass | pass |

### #248 — clear-color sites (verified by grep, 2 forms)

- Array form `[0.08, 0.08, 0.12, 1.0]`: `resources/display.rs:159` (`WindowConfig::default`), `app/render/frame.rs:202` (clear-pass fallback).
- `wgpu::Color { r/g/b/a }` form: `app/render/frame.rs:537` (docked post-scene clear), `app/render/docked.rs:43` (warm-up placeholder).
- Type is `[f64; 4]` (WindowConfig.clear_color) / `wgpu::Color` (f64 fields). Result: 4 literals → `DEFAULT_CLEAR_COLOR` ([f64;4]) + `EDITOR_SURFACE_CLEAR` (wgpu::Color). Panel: `panel.rs:218` raw `snap.z - 0.01` → `snap.z - PANEL_BG_Z_OFFSET`; `capture.rs:31` const removed, now imports from panel.

### #249 — PathGrid behavior

`grid_cell_count` returns 0 in three cases: `checked_mul` None (overflow), `size <= 0` (benign empty), `size > MAX_PATH_GRID_CELLS` (over-cap). `PathGrid::new` does `vec![true; size]` → size 0 = empty cells; `index()` returns None (idx < cells.len()==0) → `is_walkable` false. Tests: `oversized_grid_is_empty_not_allocated` (4000×4000=16M > 10M), `overflowing_grid_dims_are_empty` (`i32::MAX × 2`).

### #250 — Tilemap z, native screenshot

`Tilemap` derives only `Debug, Clone` (no serde/reflect → field-add is serde-transparent); `generation` is `pub(crate)` → external struct-literal construction is impossible, so `new` is the only construction site (verified by grep: other `Tilemap {` matches are `-> Tilemap {` return types). cell_z ortho/hex arm `-1.0` → `self.z`; iso arm `(row+col)` unchanged. Native run of `tilemap_layers` screenshotted (osascript window bounds `580,145,760,592` + `screencapture -R`): orange decoration (border+cross, z=-1, spawned first) renders over green floor (z=-2), green visible through the foreground's empty cells. Run log clean (no errors/panics). Tests: `cell_z_defaults_to_minus_one`, `with_z_sets_render_depth_for_ortho_and_hex`, `isometric_ignores_z`.

### #251 — tone envelope

rodio `SineWave` = 48 kHz mono, `sin(2π·freq·(i+1)/48000)`. Native `play_tone` (`audio/playback.rs:173`) only applied `fade_in` when a channel effect with `attack_secs > 0.001` existed; the `None` branch was raw `base` → click. Now `None` → `SamplesBuffer::new(1, 48_000, enveloped_tone_samples(freq, dur, vol))`. Envelope edge = `(dur * 0.25).clamp(0.0, 0.008)` (= wasm `audio_wasm.rs` formula), linear attack over first `edge_n` samples + symmetric release over last `edge_n`. Consts `TONE_ENVELOPE_FRAC`/`TONE_ENVELOPE_MAX_SECS`/`TONE_SAMPLE_RATE`. Facade route: `Audio::play_tone` (native) → `assign_bus` + `AudioManager::play_tone` (no effect set → None branch → enveloped). Ear-check example: `cargo run --example audio_facade`, key `T`. Tests: `tone_envelope_ramps_to_zero_at_both_ends` (first/last < 1e-6, peak > 0.9), `tone_sample_count_matches_duration` (0.1s = 4800).

### Verify gate

`./scripts/verify.sh` ran green (`VERIFY_EXIT=0`) on every PR after edit + after version bump. Two transient failures fixed mid-session (both clippy, both PRs re-verified green): #249 `assertions_on_constants` (a constant `assert!` documenting 16M>10M — dropped the line); #251 `manual_clamp` (`.min(MAX).max(0.0)` → `.clamp(0.0, MAX)`). Also #251's first build was 101 from an unclosed `mod tests` brace (added the test block but dropped the module's closing `}` — fixed). 931→936 lib tests (added 2+3+2 across #249/#250/#251).

## Gotchas & Discoveries (this session)

- **Editing a file's last test = watch the module's closing brace.** Replacing the final test in pathfinding.rs `mod tests` (whose `old_string` ended with `}\n}` — the inner test `}` + the mod `}`) and adding new tests after it dropped the mod-closing `}` → "unclosed delimiter" at the mod-open line. Append the mod `}` back. (Same trap looms whenever the Edit `old_string` includes a module's terminal brace.)
- **clippy `assertions_on_constants`** rejects `assert!(CONST_EXPR)` under `-D warnings` (e.g. `assert!((4000*4000) > MAX_PATH_GRID_CELLS)`) — a constant assertion is flagged; just drop the documentation-only assert.
- **clippy `manual_clamp`** rewrites `.min(hi).max(lo)` → `.clamp(lo, hi)` under `-D warnings`.
- **A const in a `#[cfg(not(wasm32))]` module can't back a wasm-compiled site.** `EDITOR_SURFACE_CLEAR` had to go in the cross-platform `app/render/mod.rs`, not the wasm-gated `docked.rs`, because `frame.rs:537` (which uses it) compiles on wasm.
- **A child module can read a parent module's private item.** `EDITOR_SURFACE_CLEAR` is a plain (private) const in `render/mod.rs`; `frame.rs`/`docked.rs` reach it via `super::EDITOR_SURFACE_CLEAR` — no `pub` needed (Rust visibility: an item is visible to its module and all descendants).
- **rodio 0.19 has `fade_in` but no source `fade_out`** (only `take_crossfade_with`, which fades from the start). A finite-tone release ramp therefore needs sample materialization (`SamplesBuffer`), not a lazy adapter.
- **Audit agents over-flag "High".** The save AEAD key (agent: High, fork blocker) is by-design + documented + has `save_with_key`. Always read the source + nearby docs before acting on an agent's priority.
- **The version-paperwork files (`Cargo.toml`/`Cargo.lock`/`docs/CHANGELOG.md`/`CLAUDE.md`) are touched by EVERY `/ship`.** Multi-PR sessions must serialize (merge + `git pull` main before the next branch), or the bumps conflict.
- **The native render/AV-unverifiable judgment gate is real and was exercised twice this session in opposite ways:** #250 (render) I *could* verify myself via a native screenshot → merged autonomously; #251 (audio) I could NOT hear → opened the PR, asked the user, merged only on "클릭없음". Match the verification to what's actually checkable.

## Files Changed

### #248 (clear-color + panel z dedupe)
- `src/resources/display.rs` — new `pub const DEFAULT_CLEAR_COLOR: [f64;4]`; `WindowConfig::default` uses it.
- `src/resources/mod.rs` — re-export `DEFAULT_CLEAR_COLOR`.
- `src/app/render/mod.rs` — new `const EDITOR_SURFACE_CLEAR: wgpu::Color`.
- `src/app/render/frame.rs` — import both; fallback `unwrap_or(DEFAULT_CLEAR_COLOR)`; docked clear → `EDITOR_SURFACE_CLEAR`.
- `src/app/render/docked.rs` — import + use `EDITOR_SURFACE_CLEAR`.
- `src/ui/panel.rs` — new `pub(crate) const PANEL_BG_Z_OFFSET`; used at the bg DrawRect.
- `src/ui/system/capture.rs` — import `PANEL_BG_Z_OFFSET` from panel; removed local copy.

### #249 (PathGrid)
- `src/pathfinding.rs` — `MAX_PATH_GRID_CELLS` pub + doc; `grid_cell_count` logs on overflow/cap; +2 tests.
- `src/lib.rs` — re-export `MAX_PATH_GRID_CELLS`.

### #250 (Tilemap z)
- `src/tilemap/mod.rs` — `pub z: f32` field + `with_z` builder; `cell_z` ortho/hex → `self.z`; +3 tests.
- `examples/tilemap_layers.rs` — **new** flat example (procedural atlas via `image`; two layered tilemaps).
- `CLAUDE.md` — tilemap module-map row updated.

### #251 (native tone de-click)
- `src/audio/playback.rs` — import `SamplesBuffer`; `enveloped_tone_samples` fn + 3 consts; `play_tone` None branch → enveloped buffer; +2 tests (`mod tone_envelope_tests`).

### Docs/paperwork (all 4 PRs + this wrap)
- `Cargo.toml`/`Cargo.lock` 0.68.4→0.68.5→0.68.6→0.69.0→0.69.1; `docs/CHANGELOG.md` (4 entries); `CLAUDE.md` header v1.6.148→152 (+ pathfinding/tilemap rows).
- `docs/HARDCODING_AUDIT_2026-06-26.md` — **new**, the findings record (lands with this handoff).
- Memory `engine-current-state.md` — bumped to seq 96–99.

## User Feedback & Preferences

- Session opener (`/remote-control`): "마지막 핸드오프 확인하고 작업 알려줘" → I read the handoff + board, ASKED → user's custom answer: **check the whole code for remaining hardcoding.**
- On the triage offer, user chose **"세부 항목 먼저 검토"** (review details first) → I hand-verified findings before proposing work.
- Then **"Tier1 전부 한 번에"** (all of Tier 1, as serial PRs).
- For the audio item, user chose **"내가 직접 들어볼 테니 진행"** (proceed, I'll listen myself) → implemented + held merge; user confirmed **"클릭없음"**.
- At wrap, user chose **"핸드오프 + 감사 문서 커밋"** (handoff + commit the findings doc).
- Standing prefs honored: user-facing reports in **Korean**, agent-to-agent/code/docs in **English**; merge authority standing-delegated (squash on green CI) EXCEPT the OS/GPU/AV-CI-unverifiable gate; always pass explicit `model` to subagents (used `sonnet` for the 9 audit agents); run `cargo fmt` before verify; never mask a gate's exit code.

## Where We're Going

1. **This handoff + `docs/HARDCODING_AUDIT_2026-06-26.md` land as one `docs(handoff)` PR** (branch `docs/handoff-hardcoding-audit`, commit `docs(handoff): hardcoding-audit seq 1 …`, no package bump — pure docs). After it merges, the memory seq-bump for THIS handoff PR is already reflected (seqs 96–99 cover the code PRs; the handoff PR itself is the session record).
2. **Next session: read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004 next). New EW → VISION feature+example loop. Still empty → ASK.
3. **If the user wants more hardcoding work:** `docs/HARDCODING_AUDIT_2026-06-26.md` has the Tier-2 catalog. The easiest/highest-value additive knobs (each a small feature+example): `RenderTarget::with_filter(FilterMode)`, gamepad `MAX_GAMEPADS` const + wider slot array, particle per-frame spawn-cap field, `StickNav` deadzone in a `UiConfig` resource, physics solver-iteration override. The hardest is `MAX_LIGHTS` configurable (dynamic WGSL uniform array). Tier-3 is naming/dedup polish (low value).
4. **If feature work:** VISION loop — a new capability isn't done until a small playable `examples/` game exercises it; `/add-feature-example` → `/ship` → `/land-pr`.

## Risks & Blockers

- **None blocking.** Tree clean (one untracked findings doc, intentional), CI green on all 4 merges, no open PRs.
- #251's de-click is a behavior change to audio output that CI cannot verify; it was ear-verified by the user this session. **Any future edit to `enveloped_tone_samples` or the tone path needs the same ear-check** (run `cargo run --example audio_facade`, press `T`) — CI will not catch a regression in how it *sounds*.
- The full audit report (themed, all ~118 findings) was a session-local scratchpad file; the durable subset is now `docs/HARDCODING_AUDIT_2026-06-26.md`. Some Low/Tier-3 detail did not make the committed doc — if a future session wants the exhaustive list it must re-run the audit (cheap: the 9-agent fan-out).

## Open Questions

- **None blocking.** All verify failures this session were resolved (2 clippy, 1 brace, both transient).
- Open-but-not-urgent: is the hardcoding work "done"? Tier 1 (the genuinely-worth-it items) is shipped. Tier 2 is real fork-configurability value but additive/larger; Tier 3 is polish. The user's call next session.

## Quick Start for Next Session

```bash
# 1. Read the downstream wishlist board FIRST (standing directive)
cat ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY? EW-004 next → ASK if empty

# 2. Confirm engine state
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -6        # tip should be this seq-1 handoff merge, above 53776e5 (#251)
git status -s               # clean

# 3. If continuing the hardcoding work, read the remainder catalog:
#    docs/HARDCODING_AUDIT_2026-06-26.md   — Tier-2 knobs + Tier-3 dedup, with file:line + suggested API

# 4. Verify (foreground; read the exit code; cargo fmt first)
cargo fmt && ./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"
#   (BACKGROUNDED verify must NOT append `; echo` — the task reports the echo's 0, masking a real fail.)

# Next action:
#   Read ../dungeon-merchant/docs/engine-wishlist.md. New EW → VISION loop.
#   Still empty → ASK; offer Tier-2 knobs (RenderTarget::with_filter, MAX_GAMEPADS, particle spawn cap…) or feature work.
```

---

## Session Closed

**Closed at:** 2026-06-26 (KST)
**Commit:** lands via a `docs(handoff)` PR (this file + `docs/HARDCODING_AUDIT_2026-06-26.md`)
**Session status:** Handed off — 4 Tier-1 PRs (#248–#251) merged to `main` (v0.68.4 → v0.69.1); memory bumped to seq 96–99; audit remainder catalogued. This handoff + findings doc are the session record.
