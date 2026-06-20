# Make the wasm smoke scripts orphan-safe + port-guarded

**Date:** 2026-06-20 (KST)
**Status:** COMPLETED — PR #167 merged + green; `main` @ `2f60f70`, package **v0.43.6** (unchanged), tree clean.
**Bead(s):** none (repo uses `plans/handoffs/`)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `50`
**Parent:** `HANDOFF_engine-hardening_centered-text-wasm-web_2026-06-20.md` (seq 49)

> Wishlist board was **empty** (next free ID EW-002), so picked up the **deferred** follow-up #2 from
> seq-49's "Where We're Going": port the orphan-safe server pattern that `centered_text_smoke.sh` already
> uses to the three older wasm smoke scripts. The user said proceed. Shipped as a **scripts-only** PR
> (no version bump, same precedent as seq-49).

---

## The Goal

seq-49 fixed two server-handling bugs *in the new `centered_text_smoke.sh`* but explicitly left the same
latent bug in the three pre-existing smokes (`wasm_smoke.sh`, `wasm_save_smoke.sh`, `wasm_audio_smoke.sh`)
as an out-of-scope follow-up. This session closes that gap so **all four** smokes share one orphan-safe,
port-guarded serve step.

## Where We Are

- `main` @ **`2f60f70`** (PR #167), tree clean, CI 4/4 green. Package **v0.43.6** (UNCHANGED — scripts-only).
- **PR #167** (`fix/smoke-orphan-port-guard` → squash-merged, branch deleted), +46 / −3 across 3 files.
- **Wishlist board** (`../dungeon-merchant/docs/engine-wishlist.md`): unchanged — still empty, next free ID EW-002.

## The bug (one root cause, three scripts)

All three launched their static file server via `( cd "$WEB_DIR" && python3 -m http.server "$PORT" ) &`.
That runs python as a **grandchild** of the `&` subshell, so `$!` (saved as `HTTPD_PID`) is the *subshell*,
not python. At cleanup, `kill "$HTTPD_PID"` reaps the subshell and **orphans python onto `$PORT`**. The next
run then bind-fails *silently* and Chrome is served the **stale page** from the orphan. seq-49 hit exactly
this: a render smoke false-passed off the `wasm_save` page held by an orphan on port 8085.

Additionally: `wasm_save_smoke.sh` / `wasm_audio_smoke.sh` had **no busy-port guard** at all, and
`wasm_smoke.sh` guarded only its network `SERVER_PORT` (9002), **not** its static file port (8083).

## What shipped (each script, serve step only — identical 3-part fix)

1. **Serve via `python3 -m http.server "$PORT" --directory "$WEB_DIR"`** (no `( cd && … )` subshell) so
   `$!` **is** python and `kill "$HTTPD_PID"` actually reaps it → **0 orphans** after a run.
2. **Refuse a `$PORT` already in use** (`lsof -nP -iTCP:"$PORT" -sTCP:LISTEN` → `exit 2`) so a stale server
   can't FALSE-PASS. (`wasm_smoke.sh` got this added for its static port alongside the existing 9002 guard.)
3. **Verify our own server came up** (`kill -0 "$HTTPD_PID"` after the `sleep 1`) so a silent bind failure
   can't mislead.

This is a byte-for-byte port of the pattern `centered_text_smoke.sh` already carries (seq-49) — the four
smokes now have a uniform serve step.

## Key Decisions

- **Scripts-only, NO version bump.** No engine `src/`/API/behavior change → v0.43.6 stays (seq-45/46/49
  precedent: example/script/docs-only PRs don't bump).
- **Port the existing pattern verbatim** rather than invent a new one — consistency across the four smokes
  is the point, and `centered_text_smoke.sh` is the proven reference.
- **Left the file mode of `wasm_smoke.sh` (0644) untouched** — pre-existing, invoked fine via the shebang
  path in practice, and out of scope for this fix (the other smokes are 0755; not worth churning the diff).
- **Shipped via branch + PR + squash-merge on green** (merge authority standing-delegated).

## Evidence & Data

### Local verification (these are non-CI checks — need Chrome + GPU; wasm-bindgen-cli 0.2.122 == Cargo.lock)
```
bash -n on all 4 smokes                          → clean
scripts/wasm_save_smoke.sh   → SAVE_CHECK: PASS (7/7),  exit 0, 0 surviving http.server, ports free
scripts/wasm_audio_smoke.sh  → AUDIO_CHECK: PASS (38/38), exit 0, 0 surviving http.server, ports free
scripts/wasm_smoke.sh        → connect + non-blank render (41953 B), exit 0, 0 http.server + 0 coin_race_server
busy-port guard test (held 8085, re-ran save smoke) → "FAIL: port 8085 is already in use" + exit 2 (no false-pass)
```
The `Terminated: 15` lines printed at cleanup are the trap's `kill` reaping **python directly** (the PID is
the python process now, not a subshell) — that *is* the fix working.

### Verify gate / CI (PR #167)
Engine `verify.sh` is all-Rust; this PR changes only shell, so the Rust gates are untouched. CI 4/4 green:
Build(WASM) 36s · Package dry-run 58s · Rustdoc 36s · Test(native) 4m0s.

## Files Changed
- **PR #167 (engine repo):** `scripts/wasm_smoke.sh`, `scripts/wasm_save_smoke.sh`,
  `scripts/wasm_audio_smoke.sh` (serve step only, +46 / −3).
- **Memory:** `engine-current-state.md` + `MEMORY.md` index refreshed to seq 50 / `2f60f70`.
- **NOT touched:** no engine `src/`, no `Cargo.toml`/`Cargo.lock`, no CHANGELOG, no version, no
  REFERENCE.html, no `centered_text_smoke.sh` (already correct), no `docs/WASM_SMOKES.md` (orphan-safety
  is an implementation detail — the doc describes *what each smoke checks*, which is unchanged).

## User Feedback & Preferences
- Board is the front door: when it's empty, ask before backlog. (This session: asked, user said proceed.)
- Standing prefs (unchanged): Korean user-facing reports; English code/handoffs/sub-agent prompts;
  source-verify before writing; merge standing-delegated (squash on green CI); honest gap-naming.

## Where We're Going
1. **Watch the board for EW-002+.** `../dungeon-merchant/docs/engine-wishlist.md` is empty; next free ID
   is EW-002. Read it first every engine session.
2. **Engine-hardening backlog (unchanged, needs a user go):** crates.io publish (irreversible; publish
   `engine_reflect_derive` too), per-OS gamepad input + the deferred analog-stick Y-sign hardware test.
3. **Optional polish:** seq-43 focus-ring corner-radius/pulse. Low value.

## Risks & Blockers
- **None outstanding** — PR merged green, tree clean.
- The smokes are non-CI (need Chrome/GPU) — optional local checks, by design. This PR makes them
  re-runnable back-to-back without manual port cleanup, which is the practical win.

## Quick Start for Next Session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 2f60f70 (#167 smoke orphan/port guard) … d7b6294 (#166 seq-49 handoff)
grep -m1 '^version' Cargo.toml  # 0.43.6
git status -s                   # clean

# FIRST: check the wishlist board for new/updated requests
sed -n '53,70p' ../dungeon-merchant/docs/engine-wishlist.md  # empty; next free ID EW-002

# The smokes are now orphan-safe — run any back-to-back without manual port cleanup:
scripts/wasm_smoke.sh ; scripts/wasm_save_smoke.sh ; scripts/wasm_audio_smoke.sh ; scripts/centered_text_smoke.sh
```

## Cross-cutting Gotchas (expensive-to-rediscover)
1. **`( cd && python3 ) &` orphans python.** `$!` is the subshell, not python; `kill` leaves python on the
   port → the *next* run bind-fails silently and serves a stale page. Use `python3 -m http.server --directory`
   so `$!` IS python. (Now fixed in all four smokes.)
2. **A "non-blank PNG" render smoke FALSE-PASSES off the wrong page** if an orphan holds the port. Always
   guard the port (`lsof` → refuse) AND verify your own server came up (`kill -0`). (Title-verdict smokes
   like save/audio fail *confusingly* instead, but the orphan still breaks the next run — same root fix.)
3. **Scripts/example/docs-only change → no version bump** (seq-45/46/49 precedent). A `src/` change is what
   triggers `/ship`.
4. **The four smokes default to overlapping ports** — coin_race 8083, audio 8084, save 8085, centered_text
   8085 (save & centered_text collide). The orphan guard is what keeps a leak in one from corrupting another.

---

## Session Status
**Goal met** — all three older wasm smokes are now orphan-safe + port-guarded (PR #167 merged green, no
version bump), matching `centered_text_smoke.sh`. Verified locally: all PASS, 0 orphans, busy-port → exit 2.
`main` @ `2f60f70`, tree clean. Board still empty (next ID EW-002). Handed off to next session (seq 51).

## Session Closed
**Closed at:** 2026-06-20 (KST)
**Commits:** #167 (`2f60f70`, smoke orphan/port-guard) → this handoff.
**Session status:** Goal met — the seq-49 follow-up is done; all four wasm smokes share one orphan-safe,
port-guarded serve step. No version bump (scripts-only). Board empty (next free ID EW-002). Handed off to
next session (seq 51).
