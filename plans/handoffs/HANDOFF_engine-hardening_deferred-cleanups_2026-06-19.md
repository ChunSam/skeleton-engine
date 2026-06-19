# Deferred low-risk code-review cleanups (A/B → v0.40.1, C → v0.40.2; D skipped)

**Date:** 2026-06-19
**Status:** COMPLETED — both PRs merged + green, `main` clean
**Bead(s):** none (repo uses `plans/handoffs/`, not beads)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `41`
**Parent:** `HANDOFF_engine-hardening_review-fixes_2026-06-19.md` (seq 40)
**Prior chain:** seq 37 `stretch-trio` > 38 `session-wrap-2` > 39 `visual-audio-verify` > 40 `review-fixes` > **41 this (deferred-cleanups)**

> This session drained the **deferred low-risk cleanup items** that seq-40's code review flagged but
> deliberately did NOT force ("risk > value" / "low value"). Three were implemented (A/B/C), one was
> evaluated and **deliberately skipped** (D) after reading the code proved it negligible-value.

---

## Since Last Handoff (vs seq-40's "Where We're Going")

Seq-40 listed three next-step buckets: (1) crates.io publish, (2) optional review follow-ups
(focus_pass efficiency #11, the `play_at`/`hex6_mask` micro-dedups), (3) user-only gamepad hardware.
What happened this session:
- **The user picked bucket (2)** — "리뷰 후속 정리 (저위험)" via an AskUserQuestion at onboarding.
- Shipped **A** (`play_at` dup) + **B** (focus_pass `contains`→binary_search + drop clone) as **v0.40.1**
  (PR #139), then **C** (`hex6_mask`/`hex6_flat_mask` dedup) as **v0.40.2** (PR #140).
- **D** (focus_pass `node_layout` caching, the remainder of #11) was **evaluated and skipped** — reading
  `node_layout` proved it a single component-get + arithmetic, so caching is negligible-value and adds a
  cache-consistency risk. The user confirmed "d 스킵".
- **crates.io still untouched** (bucket 1, unchanged since seq 33). Gamepad hardware (bucket 3) untouched.

## Reference Documents

- `CLAUDE.md` — conventions (now **v1.6.91**, package **v0.40.2**). R1 verify-exit rule (added seq 40).
- Parent `HANDOFF_engine-hardening_review-fixes_2026-06-19.md` (seq 40) — the code review whose deferred
  items this session drained. Its finding table #10–#13 + #11 are the source of A/B/C/D.
- `docs/VISION.md` — fork-friendly skeleton; behavior-preserving cleanups keep modules readable.

## The Goal

skeleton-engine is a fork-friendly, MIT, genre-agnostic 2D engine. The engine-hardening arc drains a
post-roadmap backlog. This segment's goal: **finish the low-risk cleanup tail of the seq-40 code review**
(behavior-preserving dedup + an efficiency micro-opt), shipping each as its own PATCH release, while
honestly skipping any "cleanup" that reading the code shows isn't worth the risk.

## Where We Are

- `main` @ **`9ff2806`** (PR #140, v0.40.2), package **v0.40.2**, CLAUDE.md header **v1.6.91**, tree clean,
  CI green. No tags pushed this session (none requested; seq-40 tagged `v0.40.0`, these PATCHes untagged).
- **2 PRs merged this segment:** #139 (A+B, `af9046f`, v0.40.1) + #140 (C, `9ff2806`, v0.40.2).
- **Verification:** `./scripts/verify.sh` green (VERIFY_EXIT=0), **870 lib tests** (unchanged — refactors
  reused existing tests, added none). wasm **audio smoke 38/38** (after A touched `audio_wasm.rs`).
  **autotile 31/31** (C's safety net). No new tests were needed; existing coverage pinned the refactors.
- **Merge friction from seq-40 did NOT recur** — plain `gh pr merge --squash --delete-branch` passed
  cleanly for both #139 and #140 (no classifier block; the work was driven by direct instructions, never
  a Korean AskUserQuestion merge option).

## What We Did (Chronological)

1. **Onboarding.** Read the seq-40 handoff + key files (`audio_wasm.rs`, `dialogue/mod.rs`,
   `ui/system/{focus_pass,text_input_pass}.rs`, `save.rs`, `tilemap/{mod,autotile}.rs`, `audio_spatial.rs`)
   + 3 adjacent (`audio/positional.rs`, `ui/system.rs` pass order, autotile hex region). Ran the verify
   gate per R1 (VERIFY_EXIT captured separately) — green, 870 tests.
2. **Picked the work via AskUserQuestion.** User chose "리뷰 후속 정리 (저위험)". Then scoped each deferred
   item by reading the actual code and gave a per-item risk verdict (A/B = do, C = do, D = defer).
3. **A + B (PR #139, v0.40.1).** Branched `chore/cleanup-play-at-focus-pass`. A: `play_at_to(dest)` helper.
   B: `is_focusable()` binary search + dropped `scratch.clone()`. Verify green; wasm audio smoke 38/38;
   `/ship` v0.40.1 paperwork; pushed; CI 4/4 green; squash-merged + branch deleted.
4. **C (PR #140, v0.40.2).** User said "cd 진행". Branched `refactor/hex-mask-dedup`. Reread the hex mask
   test coverage first (7 hex tests + reference-values test → strong net), then dedup'd both hex mask fns
   into a shared `hex_mask_from_offsets()` + per-layout offset tables. autotile 31/31; verify green;
   `/ship` v0.40.2; CI 4/4 green; squash-merged + branch deleted.
5. **D evaluated + skipped.** Read `node_layout` = `world.get::<UiNode>()` + `screen_pos()` arithmetic.
   Presented the honest cost/benefit (caching saves a handful of trivial gets only on click frames, adds a
   cache-consistency surface). User confirmed "d 스킵".
6. **This handoff** (seq 41), to be committed on a branch + PR + merged (standing-delegated).

## Key Decisions

- **D (node_layout caching) deliberately NOT done — honest skip.** `node_layout` is one component get +
  cheap arithmetic; caching it threads a parallel cache that must stay consistent with the focusables
  list, for ~0 saving. The *real* O(n)→O(log n) win of #11 was already shipped as B (binary_search). This
  was surfaced to the user with evidence before skipping (the user values honest gap-naming).
- **C via shared offset-table accumulator, NOT a literal 90° coordinate rotation.** The handoff called the
  item "hex6_mask/hex6_flat_mask 90°-unification". A true rotation-derive would be *more* obscure; instead
  each layout keeps its explicit bit-order offset table and only the 6× `if filled { mask |= bit }` loop
  is shared. This effectively resolves the deferred "90°-unification" item more safely. Readability kept.
- **One PATCH release per logical change** (A+B together = v0.40.1; C alone = v0.40.2) under the 0.x
  cadence — both are behavior-preserving with **no public API change** (`play_at_to`/`is_focusable`/
  `hex_mask_from_offsets` are all private; module map unchanged).
- **No new tests added.** All three refactors are byte-for-byte behavior-preserving and the existing tests
  (focus 13 + audio positional 6 + autotile 31, incl. parity/all-six hex cases) directly pin them. Adding
  tests would have been noise. The verify gate + targeted module test runs were the proof.
- **Parallelized paperwork with the verify gate.** For C, the v0.40.2 version-bump edits were made while
  the C-source gate ran (they don't affect an already-started cargo run), then one final gate covered
  source + paperwork together — saving a gate cycle.

## Evidence & Data

### Merges this segment
| PR | what | merge commit | version |
|---|---|---|---|
| #139 | A (wasm play_at dedup) + B (focus_pass binary_search + drop clone) | `af9046f` | v0.40.1 |
| #140 | C (hex autotile bitmask dedup) | `9ff2806` | v0.40.2 |

### The four deferred items (seq-40 #10–#13 / #11 remainder)
| id | item | verdict | where |
|---|---|---|---|
| A | `play_at`/`play_at_on_bus` 1-line dup | ✅ done | `src/audio_wasm.rs` (#139) |
| B | focus_pass per-frame O(n) `contains` scans + clone (#11 part) | ✅ done | `src/ui/system/focus_pass.rs` (#139) |
| C | `hex6_mask`/`hex6_flat_mask` "90°-unification" | ✅ done (offset-table dedup) | `src/tilemap/autotile.rs` (#140) |
| D | focus_pass `node_layout` caching (#11 remainder) | ⏭️ skipped (negligible value) | — |

### Verification (each PR)
```
#139: verify.sh VERIFY_EXIT=0 · 870 lib tests · clippy(native+wasm)+wasm build+doc clean
      wasm audio smoke PASS 38/38  (audio_wasm.rs positional path regression)
#140: verify.sh VERIFY_EXIT=0 · 870 lib tests · autotile 31/31 (hex parity + all-six pinned)
both:  CI 4 jobs green — Build(WASM) · Package dry-run · Rustdoc · Test(native)
```

## Code Analysis (the changes)

- **`src/audio_wasm.rs` (A)** — new private `fn play_at_to(&self, bytes, source, listener, max_dist,
  dest: &web_sys::GainNode) -> Sfx` = `play_sfx_to(bytes, dest)` then `update_position`. `play_at` passes
  `&self.master`; `play_at_on_bus` passes `self.bus_input(bus).unwrap_or_else(|| self.master.clone())`.
  Mirrors the existing `play_sfx`/`play_sfx_on_bus` → `play_sfx_to` structure exactly. wasm-only file.
- **`src/ui/system/focus_pass.rs` (B)** — new private `fn is_focusable(focusables: &[Entity], e: Entity)
  -> bool` = `focusables.binary_search_by_key(&e.index(), |x| x.index()).is_ok()` (the slice is built
  index-sorted by `collect_focusables`). Replaced two `Vec::contains` scans (the `UiFocus` filter + the
  "sync TextInputs not in focusables" loop) and removed the per-frame `let focusables_snapshot =
  scratch.clone();` (the loop now binary-searches the live `focusables = &*scratch` borrow — no borrow
  conflict, since `scratch` is never mutated after `collect_focusables`).
- **`src/tilemap/autotile.rs` (C)** — new private `fn hex_mask_from_offsets(r, c, offsets:
  [(i32,i32); 6], filled) -> u8` loops `for (i, &(dr,dc)) in offsets.iter().enumerate() { if filled(r+dr,
  c+dc) { mask |= 1 << i; } }`. `hex6_mask` (pointy-top, odd-r; bits E/W/NE/NW/SE/SW) and `hex6_flat_mask`
  (flat-top, odd-q; bits N/S/NE/SE/NW/SW) each build a parity-branched `[(drow,dcol); 6]` table in
  ascending bit order and delegate. **Offset tables (verified against the parity tests):**
  - hex6 odd_row `[(0,1),(0,-1),(-1,1),(-1,0),(1,1),(1,0)]`; even `[(0,1),(0,-1),(-1,0),(-1,-1),(1,0),(1,-1)]`
  - hex6_flat odd_col `[(-1,0),(1,0),(0,1),(1,1),(0,-1),(1,-1)]`; even `[(-1,0),(1,0),(-1,1),(0,1),(-1,-1),(0,-1)]`
- **`node_layout` (why D was skipped)** — `src/ui/system/state.rs:127`: `world.get::<UiNode>(entity)?` then
  `(node.screen_pos(viewport), node.size, node.z, node.visible)`. One HashMap get + arithmetic.

## Files Changed
### Source
- `src/audio_wasm.rs` (A), `src/ui/system/focus_pass.rs` (B) — PR #139
- `src/tilemap/autotile.rs` (C) — PR #140
### Bookkeeping (each PR: `/ship`'s four-edit set)
- `Cargo.toml` + `Cargo.lock` (v0.40.1 then v0.40.2), `docs/CHANGELOG.md` (0.40.1 + 0.40.2 entries),
  `CLAUDE.md` header (v1.6.90/v0.40.1 then v1.6.91/v0.40.2). Module map NOT touched (no public API change).

## User Feedback & Preferences
- **"a+b 먼저 진행"** then **"cd 진행"** then **"d 스킵"** — incremental go-aheads; the user pursued the
  low-risk basket one chunk at a time, and accepted the evidence-based skip of D.
- **"핸드오프 작성 하고 푸시"** — close with a handoff, committed/pushed (this file).
- Values **honest gap-naming** (held: D was skipped with a code-evidence rationale, not silently dropped;
  C was named as "not a literal 90° rotation").
- Korean for user-facing reports/questions; English for code/docs/handoffs/agent prompts.
- Merge standing-delegated — expressed as direct instruction; no AskUserQuestion merge option (avoids the
  seq-40 classifier misread). Held: both merges passed cleanly.

## Where We're Going
1. **crates.io publish** — the one persistent untouched backlog item (irreversible, needs explicit go;
   publish `engine_reflect_derive` too). Package dry-run CI passes on every PR.
2. **Remaining seq-38/39 follow-ups (open, low priority):** 64-tile hex atlas asset, gamepad analog-stick
   nav (UI focus currently D-pad only), focus-ring styling. All additive feature-ish, not cleanup.
3. **User-only:** real gamepad hardware test (gilrs needs a physical pad).
- The seq-40 deferred *cleanup* tail is now **fully drained** (A/B/C done, D consciously skipped) — there
  are no remaining "risk > value" micro-dedups flagged from that review.

## Risks & Blockers
- None blocking — tree clean, CI green, both PATCHes merged.
- **crates.io is irreversible** — do not publish without an explicit user go.
- The seq-40 auto-mode merge-classifier issue (Korean AskUserQuestion misread) **did not recur** this
  session; the direct-instruction workaround (recorded in `merge-authority-delegated` memory) held.

## Open Questions
- None outstanding from this segment. The only standing unknown is whether the seq-40 classifier
  merge-block was a one-off (still unconfirmed; no harness visibility) — but it did not reappear here.

## Quick Start for Next Session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 9ff2806 (#140 v0.40.2) … af9046f (#139 v0.40.1) … a863073 (#138)
grep -m1 '^version' Cargo.toml  # 0.40.2
./scripts/verify.sh > /tmp/v.log 2>&1; echo $?   # 0  (R1: capture the exit, don't pipe)

# Optional wasm regression (Chrome + matching wasm-bindgen-cli present):
bash scripts/wasm_audio_smoke.sh   # 38/38  (run if you touch src/audio_wasm.rs again)

# Files touched this session:
#   src/audio_wasm.rs (play_at_to helper), src/ui/system/focus_pass.rs (is_focusable binary search)
#   src/tilemap/autotile.rs (hex_mask_from_offsets shared accumulator)

# Next action (only if the user picks one): crates.io publish (explicit go required), or a seq-38/39
# feature follow-up. Nothing is required — the seq-40 cleanup tail is fully drained. Merge standing-delegated.
```

## Cross-cutting gotchas (expensive-to-rediscover)
1. **Stale `/tmp/verify_exit.txt` is itself an R1 trap.** At onboarding the file held the literal string
   `VERIFY_EXIT=0` from a *prior session* — but this session's gate writes a bare `0`. Reading it before
   the current job finished would have given a false-positive. Always confirm the exit file was written by
   *this* run (e.g. `rm -f` it first, or use a fresh path like `/tmp/verify2_exit.txt`).
2. **`node_layout` is trivial — don't cache it.** `world.get::<UiNode>()` + arithmetic. Any "cache the
   layout" efficiency idea in focus_pass is negligible-value and adds a consistency surface. The real
   focus_pass win was the `contains`→binary_search (done, B).
3. **The deferred "hex 90°-unification" is now resolved by C** — via a shared offset-table accumulator,
   NOT a coordinate rotation. Don't re-attempt a literal 90° rotation; it would be less readable.
4. **A new private fn in a wasm-only file (`audio_wasm.rs`) is only compile-checked by the wasm build
   step** of the gate (native `cargo test` skips the file). Run `scripts/wasm_audio_smoke.sh` for runtime
   confirmation when you touch it — it caught nothing here (pure factoring) but is the right check.
5. **Paperwork can run in parallel with the source gate.** Editing `Cargo.toml`/CHANGELOG/CLAUDE.md while
   a `verify.sh` is mid-run doesn't affect that run (cargo already read the manifest); a single final gate
   then covers source + paperwork. Saves a ~4-min gate cycle.

---

## Process / versioning notes
- 0.x cadence: PATCH = behavior-preserving fix/refactor with no API change. A+B = v0.40.1, C = v0.40.2.
  `/ship` did the four-edit set each time (Cargo.toml + lock + CHANGELOG + CLAUDE.md header). No module-map
  edits (no public API change). No tags (none requested for PATCH cleanups).
- Each PR independently green (verify gate locally + CI 4 jobs) before a standing-delegated squash-merge.

---

## Session Closed
**Closed at:** 2026-06-19 (KST)
**Commit:** A/B merged `af9046f` (#139, v0.40.1); C merged `9ff2806` (#140, v0.40.2); this handoff via its own PR.
**Session status:** Handed off to next session
