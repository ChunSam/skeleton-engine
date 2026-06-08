# v4.1.0 finalized + repo-wide Korean→English + wasm smoke check; deferred follow-ups planned

**Date:** 2026-06-09
**Status:** COMPLETED — the seq-2 plan (`PLAN_networking-dogfood_wasm-coin-race-v4.1_2026-06-08.md`) was executed in full (Phases 1–4), then a follow-on request translated all of `src/` Korean→English, and finally this session produced the seq-3 PLAN/HANDOFF for the deferred work. Engine source is now fully English. Full `+1.88.0` gate green throughout.
**Bead(s):** none (`bd` unavailable in this project)
**Epic:** none (VISION feature+example loop — post-breadth polish / dogfooding)
**Chain:** `networking-dogfood` seq `3` (continues seq 2)
**Parent:** `HANDOFF_networking-dogfood_wasm-coin-race-v4.1_2026-06-08.md` (seq 2 — shipped coin_race to wasm, tagged v4.0.0/v4.1.0, fixed 3 latent wasm render bugs; its Phase 1–4 are what this session executed)

---

## Since Last Handoff (launching context)

This session was launched (post-`/clear`) to **execute the seq-2 plan starting at Phase 1**. It did so end-to-end, then the user added a follow-on ("convert the `src/` Korean comments too"), then asked what work remained and — via the `grill-me` skill — scoped a monitor-off autonomous run to **docs/planning only** (this PLAN/HANDOFF + `NEXT_WORK`/`HANDOFF.md` updates), deferring all code (incl. wasm crispness) to a future monitor-on session.

## Reference Documents

- `plans/handoffs/PLAN_networking-dogfood_deferred-polish_2026-06-09.md` — the paired seq-3 plan (4 deferred phases).
- `plans/handoffs/PLAN_networking-dogfood_wasm-coin-race-v4.1_2026-06-08.md` + its HANDOFF — the seq-2 plan this session executed.
- `docs/VISION.md` — feature+example loop; the bar all work meets.
- `docs/NEXT_WORK.md` — updated: added "v4.1 finalize + repo-wide English" + "Deferred follow-ups" sections.
- `docs/HANDOFF.md` — updated: new struck-through dev-history row + status line (engine source now fully English; v4.1.0 tag at `ebd9081`).
- `scripts/wasm_smoke.sh` — the new optional wasm render+network smoke check.
- `CLAUDE.md` — Verification section now references `wasm_smoke.sh` (191 lines, under the 200 cap).
- Memory: `engine-v4.1-wasm-state` (updated to reflect tag move, pin push, smoke script, repo-wide English), `ci-toolchain-pin` (`+1.88.0`), `conversation-language-korean`, `doc-language-rule`, `subagent-usage-preference`, `rust-survivors-engine-pin`, `playtest-windowed-examples`.

## The Goal

Finish the wasm/v4.1.0 release cleanly (tag points at a working commit; consumer pin pushed), harden the wasm path against the 3-bug class, make the engine fully English for fork-friendliness, and leave the deferred polish work planned for the next session. Success = release correct, a repeatable wasm render guard exists, source is English (minus deliberate test fixtures), gate green, and the next session has an executable PLAN.

## Where We Are

**Engine (`skeleton-engine`), branch `main`, all pushed to `origin/main`.** Commits this session:

| Commit | What |
|---|---|
| `da96b75` | docs(examples): translate remaining Korean comments to English |
| `bb98d62` | test(wasm): add optional headless render smoke check (`scripts/wasm_smoke.sh` + CLAUDE.md) |
| `0e9f067` | docs(src): translate engine source comments + diagnostics to English (~110 files) |
| (this doc) | docs/planning: NEXT_WORK + HANDOFF.md updates + seq-3 PLAN/HANDOFF pair |

- **Tag:** `v4.1.0` **moved** `7c6f9c0` → `ebd9081` (annotated, force-pushed); remote dereferences to `ebd9081` (verified). `v4.0.0` unchanged at `5031c3d`.
- **rust-survivors:** v4.1.0 pin commit `e6176fa` **pushed** to its `origin/main` (was local-only). Its ~20 WIP doc changes remain uncommitted (not this session's work).
- **`run_demo` (`examples/wasm/`)**: re-verified on headless DPR=2 — correct. No engine change.
- **Engine source fully English**: only deliberate Korean DATA remains (33 lines across 5 files — see Evidence).
- **Gate (`cargo +1.88.0`):** fmt / clippy `--all-targets` / clippy `--target wasm32` (lib + `coin_race_game`) / build `--target wasm32` / test `--all-targets` (**311 lib + 5 coin_race_server + 3**) / doc `-D warnings` — all green.

## What We Tried (Chronological)

1. **Read the seq-2 PLAN + HANDOFF**, created a TaskCreate task per phase, verified the baseline (`./scripts/verify.sh` green).
2. **Phase 1** — asked the user the tag decision (move vs v4.1.1 vs leave); they chose **move**. `git tag -f -a v4.1.0 … ebd9081` + `git push -f origin v4.1.0`; verified `refs/tags/v4.1.0^{}` = `ebd9081`. Pushed rust-survivors `e6176fa`.
3. **Phase 4 in parallel** — launched a Sonnet subagent to convert the remaining example Korean (mp_server, skeletal_puppet, touch_demo, wasm/build.sh, wasm/index.html). It correctly kept `settings_menu`'s `"ko"` locale data; I additionally converted `touch_demo`'s incidental swipe-direction log strings (display text, not locale data).
4. **Phase 2** — built `examples/wasm` via `wasm-pack` (decoupled from the subagent's cosmetic edits), served it, captured a headless DPR=2 screenshot. **Result: fully correct** — squares spread across the viewport + HUD text in the embedded DejaVu font, no clipping. (Bonus: this Chrome/SwiftShader build *did* render glyphon text headlessly, unlike seq 2's note.) No engine change needed.
5. **Phase 3** — wrote `scripts/wasm_smoke.sh`; iterated on two real bugs: (a) headless Chrome hangs on exit under SwiftShader → background it + poll for the PNG + reap; (b) `cargo run` forks the server as a child so killing the wrapper orphaned it on port 9002 → build first, then exec the binary directly so `$SERVER_PID` is the server. Passes on `main` (connect + 47KB frame).
6. **Phase 3 revert experiment** — reverted the HiDPI fix, rebuilt, ran the smoke test: the broken frame was **60KB (larger than the good 47KB)** and still connected. Confirmed scalar checks can't catch the subtle 3-bug class. Asked the user the smoke-depth question; they chose **coarse guard + saved screenshot**. Restored the fix; finalized honest scope + docs.
7. **Gate + commit** — full `+1.88.0` gate green; committed Phase 4 (`da96b75`) and Phase 3 (`bb98d62`); pushed; asked the user the landing decision (they chose commit-to-main + push).
8. **Follow-on: `src/` Korean→English** — scoped 112 files / ~2318 Hangul lines; categorized (comments vs diagnostic strings vs intentional DATA); bin-packed into 10 balanced groups; dispatched **10 parallel Sonnet subagents** (edits only, no cargo). Fixed a cross-file `should_panic`↔`panic!` coupling one subagent flagged. `cargo fmt` re-wrapped the now-longer assert messages. Full gate green; committed `0e9f067`; pushed.
9. **"What's next?"** → listed candidates. User invoked **`grill-me`**; after researching (NEXT_WORK shows breadth complete; `zzz_coin_probe.rs` untracked/immaterial), scoped a monitor-off autonomous run to **docs/planning only**, **commit-to-main+push**, artifact = **PLAN+HANDOFF pair + docs**, PLAN to cover **all 4 deferred items**.
10. **This deliverable** — updated `docs/HANDOFF.md` + `docs/NEXT_WORK.md`, wrote the seq-3 PLAN + this HANDOFF.

## Key Decisions

- **Moved the v4.1.0 tag rather than cutting v4.1.1.** The tag was created the prior day and the 3 fixes are wasm-only/additive → low blast radius; moving is cleaner than a version churn. (User chose this.)
- **Phase 2 needed no engine change** — `run_demo` already renders correctly on Retina after seq-2's engine-wide fixes; verified rather than assumed.
- **wasm smoke test scoped to the catastrophic class, not the 3-bug class.** Empirically, reverting a fix yields a wrong-but-non-blank (sometimes larger) frame, so a cheap scalar can't separate good from bad; a reference-image diff could but needs determinism + cross-Chrome tolerance (out of scope). The script saves the screenshot for an eyeball. (User chose coarse-guard.)
- **`src/` translation kept intentional Korean DATA.** Test/locale fixtures (`locale.rs`, `ui::localized`, `text_input.rs`, `input::state`, `renderer::text`) stay Korean — translating them would break the tests. Subagents applied the data-vs-message rule; two DATA files I hadn't pre-flagged (`input::state`, `ui::localized`) were correctly preserved by the subagents.
- **10 parallel Sonnet subagents for the `src/` pass** (memory `subagent-usage-preference`), partitioned by Hangul-line balance, edits-only to avoid `target/` contention; the orchestrator ran the single gate.
- **Monitor-off autonomous run = docs/planning only.** Delicate renderer changes (wasm crispness) need real-GPU eyeball; deferred to a monitor-on session and captured in the seq-3 PLAN. (User chose this via grill-me.)

## User Feedback & Preferences

- **AskUserQuestion (Phase 1)** → "move the tag to `ebd9081`".
- **Smoke-depth** → "coarse guard + saved screenshot" (accepts it won't auto-catch the 3-bug class).
- **Landing (both batches)** → "commit to main + push".
- **`grill-me` outcome** → autonomous run = docs/planning only; artifact = PLAN+HANDOFF pair + docs; PLAN covers all 4 deferred items; **GUI tests deferred (monitor will be off)**.
- **Standing prefs (memory):** conversation in Korean / artifacts in English (`conversation-language-korean`, `doc-language-rule`); `cargo +1.88.0` for gates (`ci-toolchain-pin`); verify before declaring done; subagents for parallel work (`subagent-usage-preference`); rust-survivors pins engine by rev (`rust-survivors-engine-pin`).

## Evidence & Data

### wasm smoke test — calibration
- Good `coin_race` frame @ DPR=2: **47,3xx bytes**, server logs `[N] connected`. Stable run-to-run (coins are tiny vs the frame).
- HiDPI-reverted frame: **60,518 bytes** (LARGER — 2× HUD text), still connects → scalar checks pass on a broken frame. This is *why* the smoke test is scoped to the catastrophic class.
- Blank canvas ≈ 4KB (the `MIN_PNG_BYTES=15000` threshold cleanly separates blank from rendered).

### Remaining intentional Korean DATA in `src/` (33 lines, 5 files — must stay)
- `src/ui/text_input.rs` (21) — Hangul/IME composition test fixtures (`"한글"`, `"한"`, `"글"`, …).
- `src/ui/localized.rs` (4) + `src/locale.rs` (3) — `"ko"` locale translation values + their assertions.
- `src/input/state.rs` (3) — IME preedit test fixtures.
- `src/renderer/text.rs` (2) — `"안녕"` Korean-text-handling test.

### Cross-file coupling caught (parallel-edit hazard)
`app.rs:382` `#[should_panic(expected = "system order circular dependency detected")]` ↔ `app/schedule.rs:176` `panic!("system order circular dependency detected …")` — translated by *different* subagents; aligned by hand. `test --all-targets` is the safety net for such couplings.

### `zzz_coin_probe.rs`
Appeared in rust-analyzer diagnostics but is **not tracked by git** and not present outside `target/` → immaterial, nothing to clean.

## Code Analysis (findings for the next cycle)

- **wasm crispness is the one ready, concrete follow-up** — see seq-3 PLAN Phase 1. It's in the 3-bug-prone surface path (`window.rs` Resized/finish_init + `schedule.rs` viewport), so real-GPU verification is mandatory; `wasm_smoke.sh` is the regression guard.
- **`wasm_smoke.sh` is the first thing that actually *runs* wasm in this repo's tooling.** CI still only *builds* wasm. The script is local-only (no Chrome/GPU in CI). Keep leaning on the server `connected` log as the Chrome-version-independent signal.
- **The remote-entity helper remains blocked** on a 3rd distinct networked example — don't extract from two near-identical call sites.
- **Breadth is complete** — `NEXT_WORK.md` says every subsystem has a playable example; "new feature" is now an audit, not a backlog item.

## Files Changed (this session)

### skeleton-engine (committed + pushed)
- `examples/{mp_server,skeletal_puppet,touch_demo}.rs`, `examples/wasm/{build.sh,index.html}` — Korean→English (`da96b75`).
- **NEW** `scripts/wasm_smoke.sh` + `CLAUDE.md` — wasm smoke check + reference (`bb98d62`).
- ~110 `src/` files — Korean→English comments + diagnostics; `app.rs`/`app/schedule.rs` message coupling kept in sync; `cargo fmt` rewrap (`0e9f067`).
- `docs/HANDOFF.md`, `docs/NEXT_WORK.md` + this PLAN/HANDOFF pair — this docs/planning deliverable.

### Tags / other repos
- `skeleton-engine` tag `v4.1.0` moved `7c6f9c0`→`ebd9081` (force-pushed).
- `rust-survivors` `e6176fa` pushed to `origin/main`.

### Memory
- `engine-v4.1-wasm-state.md` — updated (tag move, pin push, smoke script, repo-wide English, the should_panic coupling gotcha).

## Gotchas & Lessons (reusable, cost real time)

- **Headless Chrome hangs on *exit* under SwiftShader** even after the screenshot is written → background it, poll for the PNG to stabilize, then reap; never block on the Chrome process.
- **`cargo run` orphans the spawned binary** — killing the `cargo` wrapper leaves the child holding the port. Build first, then exec the binary directly so the PID you kill is the real process.
- **A broken wasm render can be *larger* than a good one** (2× HUD text from the HiDPI bug) — byte-size/pixel-count can't catch the subtle 3-bug class; only a reference diff (deferred) or a human eyeball can.
- **Parallel comment edits can break cross-file string couplings** — `#[should_panic(expected=…)]` ↔ its `panic!` live in different files; grep all `should_panic` after a parallel string pass and rely on `test --all-targets`.
- **Translating diagnostic strings lengthens lines** → `assert!(cond, "…")` can cross 100 cols and `cargo fmt` will rewrap them; run `cargo fmt` (not just `--check`) after a translation pass.
- **Distinguish Korean DATA from Korean prose** — locale values and IME/Hangul test fixtures must stay; only comments + human-readable diagnostics get translated. Subagents got this right when given the explicit rule + protected-file callouts.

## Where We're Going

All seq-2 work is done and the source is fully English. Remaining work = the seq-3 PLAN's four deferred phases:

1. **wasm Retina crispness** (concrete, primary; monitor-on/real-GPU session).
2. **Reusable remote-entity helper** (blocked on a 3rd distinct networked example).
3. **New breadth feature exploration** (open-ended audit; breadth is complete).
4. **rust-survivors WIP docs cleanup** (separate repo, maintainer's call).

## Risks & Blockers

- **wasm crispness re-breaks Retina** — it's the 3-bug-prone surface path; mitigated by `wasm_smoke.sh` (regression) + mandatory real-GPU eyeball + one-revert rollback.
- **No automated wasm sharpness coverage** — the smoke test guards catastrophic failures only; sharpness/geometry still needs a human eyeball (or a future reference-image diff).
- **rust-survivors origin** still carries ~20 uncommitted WIP doc changes (their repo, maintainer's call).

## Open Questions

- wasm crispness real-GPU verification bar — deferred to the monitor-on session (PLAN Phase 1 default: headless+gate for regression, real-GPU eyeball for sharpness).
- Whether any new breadth feature is worth adding (Phase 3 audit) — open.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
cat plans/handoffs/PLAN_networking-dogfood_deferred-polish_2026-06-09.md     # the plan
cat plans/handoffs/HANDOFF_networking-dogfood_deferred-polish_2026-06-09.md  # this file
git log --oneline -5 && git rev-list -n1 v4.1.0   # v4.1.0 -> ebd9081

./scripts/verify.sh                # baseline (or the 5 cmds with cargo +1.88.0); 311 lib + 5 server
./scripts/wasm_smoke.sh            # optional: confirm wasm still runs+connects+renders (needs Chrome)

# Phase 1 (wasm Retina crispness) is the ready item — do it MONITOR ON (real-GPU eyeball required).
```

## Session Status
seq-2 plan executed in full + `src/` translated + seq-3 PLAN/HANDOFF written. Engine source fully English; `v4.1.0` tag at `ebd9081`; rust-survivors pin pushed; `+1.88.0` gate green (311 lib + 5 server + 3). This docs/planning deliverable is the last step; commit + push to `main`, then hand off.
