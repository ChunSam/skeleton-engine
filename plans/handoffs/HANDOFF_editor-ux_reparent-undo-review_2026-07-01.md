# Reparent-undo (#304) code review + interleave-cycle doc note (#305, v0.98.0)

**Date:** 2026-07-01
**Status:** COMPLETED (review posted; note PR merged)
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `editor-ux` seq `7`
**Parent:** `HANDOFF_editor-ux_scene-reparent-inline-rename_2026-07-01.md` (`editor-ux` seq 6)
**Prior chain:** seq 1 (keyboard-shortcuts) > seq 2 (action-toasts) > seq 3 (entity-visibility) > seq 4 (session-summary) > seq 5 (docked-headless-capture) > seq 6 (inline-rename + scene-reparent) > this (seq 7)

> Chain rationale: seq 6 shipped Scene-tree drag-to-reparent (#302) and explicitly listed **"reparent
> undo"** as a remaining editor-UX follow-up. A **separate** session (session_016GhHZnU3cwmKsHtr8xE3km)
> then implemented + merged it as **#304 (v0.98.0)** while this session was between tasks. The user
> asked this session to **review the newly-created PR**. So seq 7 is a review + a small doc-note
> follow-up (#305), not a feature — it closes out the reparent-undo item on the editor-ux chain.

---

## Since Last Handoff

- seq 6's "Where We're Going" listed **reparent undo** as candidate #4. It got done — but by a
  **different session** (#304), not this one. This session's job was to **review** it.
- Review verdict: **approve** — correct, symmetric, well-tested, consistent with the existing
  `EditorCmd` pattern. No blocking issues; the follow-up I'd predicted ("add an `EditorCmd` variant so
  reparent is undoable") is exactly what #304 did, done the right way (pre-move parent capture +
  no-record-on-reject + reuse of the cycle-safe `hierarchy::reparent`).
- One minor, non-blocking observation from the review (an interleaved-cycle undo no-op) was worth a
  one-line doc note → landed as **#305** (comment-only, no behavior change).
- The remaining editor-UX candidates from seq 6 are now: **entity-list type/component icons** and
  **Scene-tab inline rename** (reparent-undo is now done). Board still ACTIVE EMPTY.

## Reference Documents

- `CLAUDE.md` — editor module-map row (updated by #304 to note reparent is undoable).
- `docs/CHANGELOG.md` — 0.98.0 entry (written by #304).
- Parent handoff `HANDOFF_editor-ux_scene-reparent-inline-rename_2026-07-01.md` — the reparent feature (#302) this builds on.

## The Goal

Keep the docked in-game editor usable and correct. This session was a **quality gate**, not a feature:
an external session implemented reparent-undo (#304, v0.98.0), and the task was to code-review it,
confirm correctness, and file any small follow-ups. The broader goal is unchanged — breadth-expand the
editor's usability, self-picking from the seq-6 next-steps while the Dungeon-Merchant wishlist board is
empty.

## Where We Are

- **main @ `dbfe841`** (package **v0.98.0**, CLAUDE.md header **v1.6.185**), tree clean, all green.
- **#304 (v0.98.0, external session) MERGED** — Scene-tree drag-to-reparent is now undoable
  (`EditorCmd::Reparent { entity, old_parent, new_parent }`; Ctrl+Z restores the old parent, Ctrl+Shift+Z
  re-applies; both via `hierarchy::reparent`). Reviewed this session → **approved**.
- **#305 (this session) MERGED** — one-line-plus doc note on `EditorCmd::Reparent` (`src/app/editor/history.rs`)
  documenting the interleaved-cycle undo no-op. Comment-only, no behavior change, docs-only (no version bump).
- **Code-review comment posted on #304**: https://github.com/ChunSam/skeleton-engine/pull/304#issuecomment-4849540286
- Lib unit tests **1017 → 1020** (+3, from #304: undo/redo restore, detach-to-root undo, rejected-move-no-record).
- This session created **no** new feature code — it reviewed #304 and added the #305 note. The prior
  half of this same conversation-session had already shipped #301/#302/#303 (captured in the seq-6 handoff).

## What We Tried (Chronological)

1. **User: "check the newly-created PR and code-review it."** `gh pr list` → found **#304** (already
   merged, authored by a different Claude session per its PR body) making reparent undoable — exactly
   the seq-6 follow-up. Everything I'd created this conversation (#301–#303) was already merged; #304 was new.
2. **Fetched #304's diff** (`src/app/editor/history.rs` + `src/app/editor/ui/reparent.rs`, +108/-8) and
   reviewed for the risk areas: pre-move parent capture timing, undo/redo symmetry, cycle-safety
   interaction, selection handling.
3. **Confirmed the redo-push structure** by reading `history.rs::undo` — a single match over all
   variants then `self.redo.push(cmd)`, so `Reparent` round-trips like every other command. Verdict: approve.
4. **Reported the review to the user (Korean)**, flagging 3 minor non-blocking items; user asked to add
   the one-line note for #1 and post the review as a PR comment.
5. **#305:** branched `docs/reparent-undo-cycle-note`, added the interleave-cycle note to
   `EditorCmd::Reparent`, verified (`fmt --check` + `build --lib`, clean), PR #305, CI **5/5**, merged,
   synced main. Posted the full review as a comment on #304.

## Key Decisions

- **The #305 note is docs-only → no version bump.** It edits only a code doc-comment (not `CLAUDE.md`),
  so per the land-pr docs-only rule: no Cargo/CHANGELOG/header bump, a `docs(editor):` commit, still a
  branch+PR+CI+merge (never push to main directly).
- **Reviewed even though #304 was already merged.** A post-merge review still has value: it either finds
  a fix worth a follow-up PR or confirms correctness on the record. Here it confirmed correctness + one
  doc-worthy note.
- **PR comment written in English** (repo artifact, like a commit/handoff), while the review *report to
  the user* was Korean — per the project doc-language rule.
- **Did NOT "fix" the sibling-order or multi-select observations.** Both are inconsequential (Children
  order isn't used for render/display; multi-select-not-updated matches every existing `EditorCmd`), so
  changing them would add churn without value.

## Evidence & Data

### #304 review — what was verified

| Check | Result |
|---|---|
| `old_parent` captured **before** the move (`get::<Parent>` precedes `reparent`) | ✅ |
| Records `EditorCmd::Reparent` only on a real change (after `reparent` returns true) | ✅ rejected drag → no undo entry |
| undo = `reparent(entity, old_parent)`, redo = `reparent(entity, new_parent)` | ✅ symmetric |
| `undo()`/`redo()` push the popped cmd onto the opposite stack uniformly (`history.rs:156` / `:264`) | ✅ |
| child's local `Transform` untouched by `reparent` → no undo/redo position drift | ✅ |
| detach-to-root (`None`) reattaches on undo | ✅ (test) |
| cycle safety inherited from `hierarchy::reparent` | ✅ |

3 unit tests in #304 (`editor_reparent_undo_redo_restores_parent`, `_undo_reattaches_after_detach_to_root`,
`_rejected_move_records_no_undo`) cover the core paths.

### Minor observations (all non-blocking)

1. **Interleaved-cycle undo no-op** — if an interleaved edit made `old_parent` a descendant of the
   entity, undo's `reparent` is a silent no-op. Normal LIFO undo can't reach that state. → documented in #305.
2. **Sibling order within `Children` not preserved** across undo (`attach` appends) — inconsequential
   (Children order drives neither `Transform.z` render order nor the `entity_list`-ordered Scene tree).
3. **`selected_entities` (multi-select) not updated by undo/redo**, only `inspector_selected` — matches
   every existing `EditorCmd` variant, so not a regression.

### PRs this session (post-seq-6)

| PR | State | Version | Summary |
|---|---|---|---|
| #304 | MERGED (external session) | v0.98.0 | reparent undo — reviewed + approved this session |
| #305 | MERGED (this session) | (docs-only) | `EditorCmd::Reparent` interleave-cycle no-op note |

## Code Analysis

- **`EditorHistory::undo`/`redo` are a single match + a uniform push to the opposite stack** — adding an
  undoable op = one arm in each + the recording call site. #304 followed this exactly. Any future
  undoable editor op should copy this shape.
- **`hierarchy::reparent` is idempotent-safe for undo** because it no-ops a same-parent / cyclic /
  root-to-none move and keeps local `Transform` — so re-applying old/new parent is a clean inverse pair
  as long as the graph between move and undo hasn't been mutated into a conflicting shape (the #305 note).
- **`editor_reparent` records `old_parent` from `world.get::<Parent>(child)` before the mutation** — the
  single most important line for undo correctness; capturing after the move would store the new parent.

## Files Changed (this session)

### Source code
- `src/app/editor/history.rs` — 3-line doc note on `EditorCmd::Reparent` (interleave-cycle undo no-op). #305. No behavior change.

### (Reviewed, not authored this session — landed by #304)
- `src/app/editor/history.rs` — `EditorCmd::Reparent` variant + undo/redo arms.
- `src/app/editor/ui/reparent.rs` — `editor_reparent` records the command; 3 unit tests.

## User Feedback & Preferences

- **"github에 새로 pr 생성된 내용 확인하고 코드리뷰해줘"** — proactively check GitHub for new PRs and
  review them, even ones from other sessions. Post-merge review is wanted.
- **"주석 한 줄 추가하고, pr 코멘트 게시. 이후 /handoff 하고 세션 종료"** — act on the review's minor
  finding with a small note, post the review to the PR, then wrap up. Signal: the user wants findings
  *actioned* (a note PR + a posted comment), not just reported.
- Standing (from memory): merge authority delegated (squash on green CI); user-facing reports Korean,
  repo artifacts (PR comments/commits/handoffs) English; never push to main directly.

## Where We're Going

1. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004) — ask if still empty.
2. **Remaining seq-6 editor-UX candidates** (reparent-undo now done via #304):
   - **entity-list type/component icons** — small per-row glyph (sprite/tilemap/light/animation/generic),
     headless-verifiable via the docked capture. Lowest-risk.
   - **Scene-tab inline rename** — extend #301's rename to the Scene tree; reuse
     `editor_begin/commit/cancel_rename`, add double-click + text box in `scene_tab_body` (mind the DnD
     source interaction order — the node is now a drag source).
3. **Golden-image tests for docked panels** — standing suggestion from seq 5; feasible now.
4. Otherwise **ASK the user** when the board is empty.

## Risks & Blockers

- **GUI playtests still blocked (locked/remote screen)** — reparent-undo's *gesture* path (drag then
  Ctrl+Z) isn't headless-testable; only the command logic is unit-covered. A human drag+undo smoke on a
  real display is the only way to catch an egui/keybinding wiring regression.
- Whole docked editor is native-only. No OS-gated-CI risk. Tree clean, both PRs merged.

## Open Questions

- None blocking. (The seq-6 open questions — world-vs-local reparent, unparent-zone discoverability —
  remain, both need a GUI playtest to answer.)

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5          # tip = dbfe841 #305 note; 741512e #304 reparent-undo (v0.98.0)
git status -s                 # clean

# Board FIRST (ACTIVE EMPTY → ASK / self-pick a remaining editor-UX candidate)
sed -n '53,56p' ../dungeon-merchant/docs/engine-wishlist.md

# Verify (2 audio tests fail locally — env; read exit via echo $?, not ${PIPESTATUS[0]} in zsh)
cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings

# Key files (reparent-undo, for context)
#   src/app/editor/history.rs      — EditorCmd::Reparent undo/redo arms + the interleave-cycle note (#305)
#   src/app/editor/ui/reparent.rs  — editor_reparent records the command (pre-move parent capture)

# Next action
#   Check the board; if empty, pick a remaining editor-UX candidate — entity-list type/component icons
#   (lowest-risk) or Scene-tab inline rename — both headless-verifiable via screenshot_editor_docked_headless.
```
