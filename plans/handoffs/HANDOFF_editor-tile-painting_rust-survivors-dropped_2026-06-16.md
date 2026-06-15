# rust-survivors dropped as a maintained consumer + doc/memory scrub

**Date:** 2026-06-16
**Status:** COMPLETED (decision executed; pushed; nothing pending merge)
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine — project scope / maintenance policy
**Chain:** `editor-tile-painting` seq `5`
**Parent:** `HANDOFF_editor-tile-painting_items1-5-batch_2026-06-16.md`
**Prior chain:** `..._v8.11-shipped_...` (1) > `..._a-g-editor-loop_...` (2) > `..._d2-d5-loop_...` (3) > `..._items1-5-batch_...` (4) > this (5)

## Stale References

None. This was a docs/memory-only change — no engine code, identifiers, or APIs touched. `main` stays at
v8.27.0 (the seq-4 feature batch); this session added one commit on top (`92e05fe`).

## Since Last Handoff

Seq 4 closed the items-1-5 remaining-work batch with **item 1 (rust-survivors pin bump) PARKED** pending
the user's uncommitted WIP. This session, the user **decided to drop rust-survivors entirely**:

> "rust-survivors는 너무 엔진 초창기에 같이 개발된 게임이라 … 현재 엔진상태와 비교해서 같이 가기 어렵다 …
> 다음부터는 rust-survivors에 대한 최적화는 안챙겨도 될것같아. 문서에 관련 내용 남아있으면 없애줘."

So item 1 flipped from "parked, do later" to **"dropped, never"** — and I scrubbed the forward-looking
rust-survivors maintenance guidance from the living docs + memory. The seq-4 handoff's forward sections
were also corrected (its in-session *record* of the assessment is left intact; only its next-session
guidance changed).

## The Goal

Stop treating rust-survivors as a maintained downstream consumer of the engine, and remove the documents
+ memories that would otherwise steer a future session into "check impact on rust-survivors / bump its
engine pin / sync it." Keep historical records (CHANGELOG, dev-log) intact — scrub only forward guidance.

## Where We Are

- **`main` = `92e05fe`**, clean tree, pushed. One commit this session: `docs: drop rust-survivors as a
  maintained consumer`. **No open PRs; nothing pending merge.**
- **Living docs scrubbed (repo):**
  - `CLAUDE.md` — removed the entire **"Related projects"** section (the rust-survivors row + the "on
    breaking changes, check the impact on the game side" instruction). Now 183 lines (≤200 rule holds).
  - **Deleted** `docs/RUST_SURVIVORS_TEXTURE_CACHE_KEY_PROMPT.md` + `docs/RUST_SURVIVORS_UV_MIGRATION_PROMPT.md`
    (git-tracked → recoverable) — dedicated game-side migration prompt docs.
  - `docs/NEXT_WORK.md` — removed the "rust-survivors WIP docs cleanup" forward action item + a stray
    "complements rust-survivors" note.
  - `docs/HANDOFF.md` — removed a now-dangling pointer to the deleted prompt doc (kept the engine-fix record).
  - The seq-4 handoff — forward sections (Where We're Going #1, Quick Start, Risks, Status) corrected to
    "rust-survivors DROPPED"; a post-session note added at the top.
- **Memory scrubbed (`~/.claude/.../memory/`):**
  - **Deleted** `rust-survivors-engine-pin.md`; **added** `rust-survivors-deprecated.md` (the one fact worth
    keeping: don't sync/pin-bump rust-survivors; engine validated on its own).
  - `MEMORY.md` index — replaced the pin pointer with the deprecated pointer; refreshed the stale
    `engine-current-state` pointer (was v8.19.0 → now v8.27.0, rust-survivors removed).
  - `engine-current-state.md` — item-1 framing → DROPPED; removed the rust-survivors next-action; trimmed a
    stale "REMAINING (deferred)" tail. `project-vision.md` — removed the "(e.g. rust-survivors)" example.
- **Left intact (history, not guidance):** `docs/CHANGELOG.md` "rust-survivors unaffected" entries;
  `docs/HANDOFF.md` per-phase dev-log mentions (line 15 already says rust-survivors is out of verification
  scope); two incidental past-tense mentions in `docs/NEXT_WORK.md` (l.54 "rebuilt clean", l.266 old wasm
  pin push) embedded in engine-change records.

## What We Tried (Chronological)

1. **Swept all references** — grepped `rust-survivors` across `CLAUDE.md`, `docs/`, `AGENTS.md` (none), and
   `memory/`. Classified each as *forward guidance* (scrub) vs *historical record* (keep).
2. **Edited / deleted** per the classification (see Where We Are). Recorded the decision as a new memory.
3. **Verified** — re-grepped: `CLAUDE.md` clean; no dangling refs to the deleted docs except one in
   `HANDOFF.md` (fixed); confirmed only the 2 incidental historical mentions remain in `NEXT_WORK.md`.
4. **Committed + pushed** `92e05fe`. **Merge check:** `gh pr list --state open` = empty → nothing to merge.

## Key Decisions

- **rust-survivors dropped (user directive), not merely parked.** The game is too early-era to keep in sync;
  engine work is no longer gated on or measured by it.
- **Scrub forward guidance, preserve history.** Removed maintenance pointers + dedicated docs + the
  next-action; left CHANGELOG/dev-log records (rewriting history is wrong and they're not guidance).
- **Left 2 incidental historical mentions in NEXT_WORK.md** (past-tense, embedded in engine-change records) —
  surfaced to the user rather than rewriting old session records unprompted.
- **Did NOT merge/clean the stale old branches** (`feat/v7.1-docked-editor`, `feat/v8-scene-layout-editing`,
  `feat/v8.1-data-editor`, `fix/macos-mainthread-pacing`, `docs/english-conversion`, remote
  `fix/v8.1.5-scene-pop-text-wrap`). They have no open PRs and predate this session — blind-merging would be
  wrong. Flagged for the user.

## Evidence & Data

### Files changed (commit `92e05fe`)

| File | Change |
|---|---|
| `CLAUDE.md` | removed "Related projects" section (−14 lines) |
| `docs/RUST_SURVIVORS_TEXTURE_CACHE_KEY_PROMPT.md` | **deleted** |
| `docs/RUST_SURVIVORS_UV_MIGRATION_PROMPT.md` | **deleted** |
| `docs/NEXT_WORK.md` | dropped a forward action item + a stray note |
| `docs/HANDOFF.md` | removed a dangling pointer to a deleted doc |
| `plans/handoffs/HANDOFF_..._items1-5-batch_...md` | forward sections → "rust-survivors DROPPED" |

### Memory changes (not in the repo — `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/`)

`rust-survivors-engine-pin.md` deleted · `rust-survivors-deprecated.md` added · `MEMORY.md`,
`engine-current-state.md`, `project-vision.md` updated.

### Verification

`grep -i rust-survivors CLAUDE.md` → ∅. No dangling refs to the deleted docs. `gh pr list --state open`
→ ∅ (nothing to merge). `wc -l CLAUDE.md` → 183 (≤200).

## User Feedback & Preferences (REQUIRED — never omit)

- **"다음부터는 rust-survivors에 대한 최적화는 안챙겨도 될것같아. 문서에 관련 내용 남아있으면 없애줘."** —
  drop rust-survivors maintenance; remove related content from docs. (Captured in `[[rust-survivors-deprecated]]`.)
- The user distinguishes **forward guidance vs history** implicitly — I preserved historical records and
  surfaced the borderline cases rather than scrubbing everything. Confirm if they want history scrubbed too.
- **"/handoff 하고 머지 남아 있으면 머지 진행"** — write the handoff; merge anything pending (there was nothing).
- **Standing:** Korean prose to the user, English code/docs/handoff; never drop `CLAUDE.md` content to hit
  ≤200; subagents on Sonnet with explicit `model:`.

## Where We're Going

1. **Engine/editor feature work continues, rust-survivors-free.** Validate on Gate6 + in-repo `examples/`
   only. Next candidates (from seq 4): SM **node-graph** + timeline **time-ruler** visual editors
   (iteration 2); `AnimationStateMachine`/`Timeline` **serde** so editor edits survive scene save/load.
2. **(optional) Finish the history scrub** — if the user wants, remove the 2 incidental rust-survivors
   mentions in `docs/NEXT_WORK.md` (l.54, l.266) and any dev-log mentions in `docs/HANDOFF.md`.
3. **(optional) Stale-branch cleanup** — 4 local + ~2 remote abandoned branches predate this work; delete on
   the user's say-so.

## Risks & Blockers

- **None.** `main` green + clean; no pending merges; change was docs/memory-only.

## Open Questions

- Scrub the remaining *historical* rust-survivors mentions (NEXT_WORK l.54/266, HANDOFF dev-log) too, or
  keep them as records? (Default kept.)
- Delete the stale local/remote branches?

## Quick Start for Next Session

```bash
# No beads. Reference: CLAUDE.md (Gate6), docs/VISION.md, plans/*_plan.md.
git -C /Users/jkl/Projects/skeleton-engine log --oneline -1   # expect 92e05fe
cargo test --lib                # 603 pass (unchanged — docs-only commit on top)

# rust-survivors is DROPPED — do not sync/bump it. See [[rust-survivors-deprecated]] memory.

# Next action — pick ONE (engine work, rust-survivors-free):
#   (a) SM node-graph / timeline time-ruler visual editors (iteration 2), OR
#   (b) AnimationStateMachine / Timeline serde (editor edits survive scene save/load), OR
#   (c) a new feature + example per docs/VISION.md.
# Key files: src/app/editor/ui/docked.rs (panels), src/animation/state_machine.rs, src/timeline.rs.
```

## Session Closed
**Closed at:** 2026-06-16
**Commit:** `92e05fe` (rust-survivors drop) — on top of `2ee9cee` (v8.27.0).
**Session status:** Handed off — rust-survivors dropped as a maintained consumer; nothing pending merge.
