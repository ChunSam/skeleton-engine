# Session wrap — `ship-wasm-example` skill hardening + memory-file split

**Date:** 2026-06-20 (KST)
**Status:** COMPLETED — all changes are local/gitignored (skill + proposal + memory); engine tracked tree clean. This handoff is the only tracked artifact.
**Bead(s):** none (repo uses `plans/handoffs/`)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `52`
**Parent:** `HANDOFF_engine-hardening_focus-ring-pulse_2026-06-20.md` (seq 51)

> This is the **wrap** of a long session whose *code* work already shipped under its own handoffs:
> **seq 50** (`HANDOFF_..._smoke-orphan-port-guard_...`, PR #167) and **seq 51**
> (`HANDOFF_..._focus-ring-pulse_..._`, PR #169, v0.44.0). After seq-51 closed, the user ran `/wrap`,
> which produced a skill/rule proposal; the user picked **A·B·D**, and this session executed them. All
> three touch only **local gitignored files** (a skill, the proposal doc, and `~/.claude` memory) — no
> engine `src/`/tracked change. This handoff records that meta/tooling pass so the next session knows
> the memory layout changed and the wasm-example skill improved.

---

## The Goal

Run `/wrap` over the day's commits (seq 46–51) to surface repeated-pattern → skill/rule candidates, then
execute the accepted ones. Accepted: **A** + **B** (improve the `ship-wasm-example` skill) and **D**
(split the oversized `engine-current-state` memory). **C** (a `docs/PATTERNS.md` note) was deferred.

## Where We Are

- `main` @ **`9587144`** (the seq-51 handoff merge), tree clean, package **v0.44.0**. **No new engine commit this session.**
- The day's shipped code: PR #159 (EW-001 fix, v0.43.6) · #162 (centered_text example) · #165 (centered_text→wasm web) · #167 (smoke orphan/port guard) · #169 (focus-ring pulse, v0.44.0) — all merged green, all already handed off (seq 47–51).
- **Wishlist board** (`../dungeon-merchant/docs/engine-wishlist.md`): unchanged — ACTIVE empty, next free ID **EW-002**.

## What this session did (the A·B·D delta — all local/gitignored)

### `/wrap` analysis → `.claude/proposals/2026-06-20.md`
Mined seq 46–51. Found candidates A/B (one skill fix), C (pattern doc), D (memory hygiene). Excluded
already-skilled patterns (`/ship`, `/add-feature-example`, `/ship-wasm-example`, `/handoff`,
`/split-module`). The proposal file now carries a `## Resolution` section marking A·B·D done, C deferred.

### A + B — `.claude/skills/ship-wasm-example/SKILL.md`
- **A (repeated pattern + same-mistake-fixed).** Added a Gotchas bullet: the smoke must serve via
  `python3 -m http.server PORT --directory DIR`, **never** `( cd DIR && python3 … ) &` — the subshell
  makes `$!` the subshell, so cleanup `kill "$HTTPD_PID"` orphans python onto the port and the next run
  bind-fails silently → Chrome served the **stale page** → false-pass. Plus `lsof` busy-port guard +
  `kill -0` own-server check. (seq-49 was bitten; seq-50 unified all four smokes on this — see those handoffs.)
- **B (skill staleness).** Step 6 said "add the Optional-wasm-check bullet to the **CLAUDE.md Verification
  section**" — but seq-49 moved those bullets to **`docs/WASM_SMOKES.md`** (CLAUDE.md only one-line-refs it).
  Fixed the step. Also added `centered_text_smoke.sh` as the **render-only** smoke model (vs the
  title-verdict `wasm_audio_smoke.sh`/`wasm_save_smoke.sh`).
- Logged in memory `local-tooling-skills.md` (gitignored skills → memory is the only record; standing rule [[record-skills-in-memory]]).

### D — memory hygiene: split `engine-current-state.md`
- **Root cause:** the file had grown to **75.6 KB (~33k tokens)** — over the Read/Edit 25k-token cap, so
  the last two memory updates (seq 50, 51) had to be done with throwaway Python `str.replace` scripts.
  The bloat was **line 10**: a single 56 KB "current-state lead" paragraph, because each session's memory
  update *prepended* its seq to that one line (seq 51 → PRIOR seq 50 → PRIOR seq 49 → … → all of v9.x).
- **Fix (Python split, lossless):**
  - **`engine-current-state.md`** rewritten compact (**5.3 KB**): trimmed frontmatter description + recent
    seqs (51/50/49/47) + the **live-gotchas paragraph preserved verbatim** + a pointer to the archive.
  - **`engine-history-archive.md`** (NEW, `type: reference`): the former dense frontmatter summary + the
    full old seq-chain lead + the v9.x→0.x-reset→v0.43.x narrative. Nothing lost.
  - `MEMORY.md`: added the archive index line; the `engine-current-state` line already updated to seq 51.

## Key Decisions

- **A·B as a skill edit, not a CLAUDE.md rule.** CLAUDE.md already delegates smoke detail to
  `docs/WASM_SMOKES.md` and is near its 200-line budget; the orphan-safety belongs in the skill that
  generates smokes. (The proposal noted CLAUDE.md as the alternate home and rejected it.)
- **C deferred, not done.** The "time-driven System animation" pattern (focus-ring pulse + cursor-blink)
  is only **2 instances**; documenting it now risks over-abstraction. Revisit when a 3rd appears.
- **D = split, not trim-in-place.** Archiving (vs deleting the tail) keeps old-version/decision context
  recallable while making the live file editable again. Future updates must keep the live file compact —
  **do NOT resume prepending each seq into one giant lead line** (that is exactly what bloated it).
- **No PR for A·B·D** — they are gitignored personal files. Only this handoff is tracked → commit + push it.

## Evidence & Data

```
engine-current-state.md:  75,580 B  →  5,344 B   (now Read/Edit-able; was over the 25k-token cap)
engine-history-archive.md:  74,030 B (new, type: reference — full prior content, lossless)
ship-wasm-example/SKILL.md: grep "orphan-safe|docs/WASM_SMOKES.md|centered_text_smoke.sh" → 3 hits (A+B landed)
engine repo `git status -s`: clean (all edits were gitignored .claude/ + ~/.claude memory)
```
Live-file split verified: the `**Live gotchas (still valid):**` paragraph (native-only cfg-gating, real
wasm gate = lib+bins not `--all-targets`, `set_scene` resets world, ignore stale rust-analyzer, gate-pipe
exit-code masking, etc.) is present verbatim in the new compact `engine-current-state.md`.

## Files Changed (all local/gitignored — none tracked except this handoff)
- `.claude/skills/ship-wasm-example/SKILL.md` — A (orphan-safe serve Gotcha) + B (doc-location fix + render-only model).
- `.claude/proposals/2026-06-20.md` — the `/wrap` proposal + Resolution section.
- `~/.claude/.../memory/engine-current-state.md` — rewritten compact.
- `~/.claude/.../memory/engine-history-archive.md` — NEW (archived deep history).
- `~/.claude/.../memory/local-tooling-skills.md` — A·B logged.
- `~/.claude/.../memory/MEMORY.md` — archive index line; engine-current-state line at seq 51.
- **Tracked (this PR):** `plans/handoffs/HANDOFF_engine-hardening_wrap-skill-memory-hygiene_2026-06-20.md`.

## User Feedback & Preferences
- Ran `/wrap` then chose **A·B·D** explicitly (C left for later). Board-empty → ask-before-backlog holds.
- Standing prefs (unchanged): Korean user-facing reports; English code/handoffs/sub-agent prompts;
  source-verify before writing; merge standing-delegated (squash on green CI); honest gap-naming;
  log local-skill changes in memory the same turn.

## Where We're Going
1. **Watch the board for EW-002+.** `../dungeon-merchant/docs/engine-wishlist.md` is empty; read it first each session.
2. **Memory layout note for next session:** `engine-current-state.md` is now COMPACT + there is an
   `engine-history-archive.md`. Keep the live file small — append recent seqs and **drop/trim the oldest**
   rather than growing one giant lead line. Edit works again (no Python-script workaround needed).
3. **Pending candidate C:** if a 3rd "accumulate a wrapped dt clock in a System struct, thread into a
   pure sub-pass helper" instance appears, promote it to a `docs/PATTERNS.md` architecture note.
4. **Deferred from seq-51: focus-ring corner-radius** — a renderer feature (rounded-rect SDF in the UI
   frag shader; `DrawRect` is a plain quad), not polish. Only if wanted.
5. **Engine-hardening backlog (needs a user go):** crates.io publish (irreversible; `engine_reflect_derive`
   too), per-OS gamepad input + the deferred analog-stick Y-sign hardware test.

## Risks & Blockers
- **None outstanding.** All changes local; engine tree clean and green at v0.44.0.

## Quick Start for Next Session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 9587144 (#170 seq-51 handoff) … 53f471d (#169 pulse v0.44.0)
grep -m1 '^version' Cargo.toml  # 0.44.0
git status -s                   # clean

# FIRST: check the wishlist board
sed -n '53,70p' ../dungeon-merchant/docs/engine-wishlist.md  # empty; next free ID EW-002

# Memory: engine-current-state.md is now compact; deep history is in engine-history-archive.md.
```

## Cross-cutting Gotchas (expensive-to-rediscover)
1. **Don't let `engine-current-state.md` re-bloat.** It bloated because each memory update *prepended* a
   seq into a single lead paragraph until it crossed the 25k-token Read/Edit cap (forcing Python-script
   edits). Keep the lead to the most recent ~3–4 seqs; older detail goes to `engine-history-archive.md`.
2. **A huge single-frontmatter-`description` is a recall-cost trap too** — keep the description a short
   relevance hook, not a full history dump.
3. **Local skills are gitignored** → every skill create/change must be logged in memory the same turn
   ([[record-skills-in-memory]]); memory is the only record.
4. **The orphan-safe smoke-serve pattern is now baked into the `ship-wasm-example` skill** — future
   generated smokes inherit it; don't hand-write `( cd && python3 ) &` again.

---

## Session Status
**Goal met** — `/wrap` proposal written; **A·B** hardened the `ship-wasm-example` skill (orphan-safe serve
Gotcha + correct doc location + render-only model); **D** split the oversized memory file (75.6 KB → 5.3 KB
live + a lossless `engine-history-archive.md`). **C** deferred. No engine code changed (v0.44.0 holds);
board empty (next ID EW-002). Handed off to next session (seq 53).
