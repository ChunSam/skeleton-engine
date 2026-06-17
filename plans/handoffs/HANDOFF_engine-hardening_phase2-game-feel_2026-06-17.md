# User-experience roadmap — Phase 2 shipped (game-feel core)

**Date:** 2026-06-17
**Status:** IN PROGRESS — Phase 2 implemented + verified; **PR #95 (v0.12.0) is GREEN + MERGEABLE but NOT YET MERGED — it needs explicit merge authorization.** Currently on branch `feat/phase2-game-feel` (clean tree). `main` @ `0e44a24`.
**Bead(s):** none (no bead tracker — `bd` unavailable)
**Epic:** skeleton-engine — engine hardening / fork-friendliness
**Chain:** `engine-hardening` seq `14`
**Parent:** `HANDOFF_engine-hardening_ux-roadmap-phase1_2026-06-17.md` (seq 13)
**Prior chain:** seq 12 (version reset 0.11.0) → seq 13 (UX roadmap + Phase 1, v0.11.1) → **this (14)**

---

## ⚠️ FIRST ACTION FOR NEXT SESSION

**PR #95 is open, CI 4/4 green, `MERGEABLE`/`CLEAN`, but UNMERGED.** The user invoked `/handoff`
instead of answering the merge question, so the merge was correctly NOT performed (merge authority
is per-session + per-action). To finish Phase 2:
1. Re-confirm merge authority with the user.
2. `git checkout main && gh pr merge 95 --squash --delete-branch && git pull --ff-only origin main`.
3. Update `engine-current-state` memory (main HEAD + "Phase 2 merged").
4. (optional) tag `v0.12.0` — only if the user asks.
Then Phase 3 is next (see Where We're Going).

## Since Last Handoff (seq 13)

Seq 13's "Where We're Going" said: **Phase 2 = game-feel core (TimeScale + Tween<T>+easing +
juice_demo)**, continuing the same session. This session did exactly that:
- Implemented `TimeScale` + `RealDt` + scaled-dt scheduler injection + `App::set_time_scale`.
- Made `Tween` generic (`Tween<T: Lerp = f32>`); added 4 bounce/elastic easings; marked `Easing`
  `#[non_exhaustive]`.
- Wrote `examples/juice_demo.rs` (folds in B2 — rescues FadeTransition/shake/post-process).
- Verified (777 lib tests), bumped to **0.12.0** via the `ship` skill, opened **PR #95**, CI 4/4 green.
- **Did NOT merge** (awaiting auth — see above).

Also merged this session before Phase 2: **#93** (Phase 1, v0.11.1) and **#94** (the seq-13 handoff).

## Reference Documents

- `plans/USER_EXPERIENCE_PLAN_2026-06-17.md` — THE roadmap (7 phases). Phase 1 DONE, Phase 2 awaiting
  merge, Phase 3 next. **Read this for what's next.**
- `HANDOFF_engine-hardening_ux-roadmap-phase1_2026-06-17.md` (seq 13) — Phase 1 + the 3-agent audit
  that produced the plan + the publish-deferral decision.
- `CLAUDE.md` — header now v1.6.60 / package v0.12.0 (committed on the PR branch, not yet on main).
- `docs/CHANGELOG.md` — new `## 0.12.0` entry (on the PR branch).

## The Goal

Raise satisfaction for people who fork/use `skeleton-engine`. Phase 2's slice: the highest-leverage
"game feel" primitives — global time-scaling (hit-stop/slow-mo) and value/easing tweening — plus an
example that gives three already-shipped-but-undemonstrated features (`FadeTransition`,
`Camera::shake`, `PostProcessConfig`) their first playable demo (the engine was violating its own
"a feature needs an example" rule for those).

## Where We Are

- **Branch `feat/phase2-game-feel`** (clean), commit `31407f8`. `main` @ `0e44a24` (does NOT yet have
  Phase 2).
- **Package version 0.12.0** on the branch (Cargo.toml + lock + CLAUDE.md header v1.6.60 + CHANGELOG).
- **777 lib tests** pass (+5 over Phase 1's 772). Full `./scripts/verify.sh` green (fmt / clippy
  --all-targets / wasm lib+bins / test --all-targets / rustdoc -D warnings).
- **PR #95 OPEN, CI 4/4 green, MERGEABLE/CLEAN** — Build (WASM) 39s, Package dry-run 1m13s, Rustdoc
  38s, Test (native) 4m20s. **Awaiting merge authorization.**
- **`juice_demo` render-verified** on macOS: two screenshots captured — steady (easing row at distinct
  heights + slid-in player) and post-impact (yellow "HIT-STOP" + intensified vignette). Sent to user.
- No work in flight beyond the pending merge. No wakeups/cron/loops armed.

## What We Tried (Chronological)

1. **TimeScale (A1).** Added `TimeScale(pub f32)` resource (default 1.0, `get()` clamps ≥0) in
   `resources.rs`; inserted in `core_resources.rs`; re-exported in `lib.rs`. In
   `schedule.rs::run_systems`, compute `scaled_dt = dt * time_scale` and pass `sys_dt = if i <
   tail_start { scaled_dt } else { dt }` to each system's `run` — so **scene systems are scaled,
   built-in tail systems (hierarchy/gizmo) stay real-time**. Added `App::set_time_scale` /
   `App::time_scale` next to `set_system_panic_policy`. Unit test `time_scale_scales_scene_system_dt`.
2. **RealDt (A1 companion — a mid-design necessity).** Realized hit-stop is **self-timed**: a system
   that sets `TimeScale(0)` would freeze its own recovery timer (it runs on scaled dt = 0). Added
   `RealDt(pub f32)` resource, written each frame in `run_systems` with the unscaled `dt` before
   scaling. Re-exported. This makes TimeScale actually usable for hit-stop.
3. **Tween<T> generic (A2).** Changed `pub struct Tween` → `pub struct Tween<T: Lerp = f32>` (default
   type parameter). `value()` now `T::lerp(&start, &end, easing.apply(fraction))`. The `= f32` default
   keeps every existing `Tween::new(0.0, …)` call site and `TweenSequence` (still f32) compiling
   unchanged. Verified `Tween` (struct) had no external users (only a comment in camera.rs).
4. **Easings + `#[non_exhaustive]` (A2).** Added `EaseInBounce`/`EaseOutBounce`/`EaseInElastic`/
   `EaseOutElastic` to the `Easing` enum + `apply()` arms + an `out_bounce` helper. Marked the enum
   `#[non_exhaustive]`. Grepped first: no exhaustive external `match` on Easing exists (the editor
   compares by debug string), so the only sync needed was the editor's `easing_variants()` array in
   `timeline_panel.rs` (`[Easing; 6]` → `[Easing; 10]`, +4 variants).
5. **Tests.** Added generic `Tween<Vec2>`/`Tween<Color>` midpoint tests + bounce/elastic endpoint +
   elastic-overshoot tests in tween.rs. 772 → 777.
6. **juice_demo (B2 folded in).** Wrote `examples/juice_demo.rs`: one `JuiceSystem` that does input →
   hit-stop (TimeScale via RealDt countdown) → camera shake → vignette pulse (PostProcessConfig) →
   timed fade cycle (FadeTransition) → `Tween<Vec2>` player slide-in → easing-row bob → HUD text.
   Demonstrates the scaled/real split (row freezes on hit-stop; shake+vignette keep going).
7. **First verify FAILED on fmt.** `cargo fmt --check` flagged a multi-line `assert!` + the `ping`
   ternary in juice_demo. Ran `cargo fmt`; re-verified → green. (Also caught that appending
   `; echo VERIFY_EXIT=$?` to a backgrounded gate makes the task's reported exit = echo's, not the
   gate's — judged by log content instead.)
8. **Playtest.** Built `juice_demo`, launched, osascript-positioned the window, captured steady +
   (space-keystroke) hit-stop screenshots. Both correct.
9. **ship 0.12.0.** Verify green → `ship` skill → Cargo.toml + lock + CHANGELOG + CLAUDE.md → `cargo
   build` + `cargo fmt --check` confirm → commit → push → PR #95 → CI watch (waited for checks to
   register first to dodge the race) → 4/4 green.
10. **Asked for merge auth + sent screenshots → user invoked `/handoff`** (this file).

## Key Decisions

- **Scaled-vs-real split:** only scene systems (index `< tail_start`) get scaled dt; built-in tail
  systems + `post_systems` (fades, hot-reload, asset upload, camera) keep real dt. Keeps editor +
  transitions responsive at `time_scale = 0`.
- **Added `RealDt` rather than forcing Instant in the demo** — a hit-stop controller genuinely needs
  real time to end its own freeze; exposing the unscaled dt as a resource is the clean, reusable fix
  (otherwise TimeScale isn't usable for its #1 use case).
- **`Tween<T: Lerp = f32>` (default type parameter), not a new type** — zero churn for existing call
  sites and `TweenSequence` stays f32-only via the default.
- **`Easing` `#[non_exhaustive]`** — one-time pre-1.0 break so future curves are additive forever.
  Safe because no external exhaustive match exists; only the editor's variant list needed syncing.
- **juice_demo is ONE example covering A1+A2+B2** — rescues 3 orphaned features in one acceptance test.
- **Phase 2 = MINOR 0.12.0** — first real-world MINOR via the 0.x-aware `ship` skill; it correctly
  picked MINOR (feature) and never suggested 1.0.0 (the `#[non_exhaustive]` break would have tripped
  the old "breaking → major" rule). Validates the seq-12 ship-skill fix on a real bump.
- **Did NOT merge #95** — the user invoked `/handoff` instead of confirming; merge stays gated.

## Evidence & Data

### PR #95 (open, awaiting merge)

| Check | Result |
|---|---|
| Build (WASM) | pass 39s |
| Package dry-run | pass 1m13s |
| Rustdoc | pass 38s |
| Test (native) | pass 4m20s |

`mergeable: MERGEABLE`, `mergeStateStatus: CLEAN`, branch `feat/phase2-game-feel`, commit `31407f8`.

### Files in PR #95 (`git diff --cached --stat`)

| File | Δ |
|---|---|
| `examples/juice_demo.rs` | +236 (new) |
| `src/tween.rs` | +141/− (generic + 4 easings + 4 tests) |
| `src/app/schedule.rs` | +84 (scaled dt + App methods + test) |
| `src/resources.rs` | +40 (TimeScale + RealDt) |
| `docs/CHANGELOG.md` | +29 (## 0.12.0) |
| `CLAUDE.md` | ±6 (header + 2 module-map rows) |
| `src/lib.rs` | ±4 (re-exports) |
| `src/app/core_resources.rs`, `src/app/editor/ui/timeline_panel.rs` | ±6 each |
| `Cargo.toml` / `Cargo.lock` | → 0.12.0 |

### Roadmap status (from `USER_EXPERIENCE_PLAN_2026-06-17.md`)

| Phase | Theme | Status |
|---|---|---|
| 1 | First-hour & doc truth | **DONE** — PR #93, v0.11.1 (merged) |
| 2 | Game-feel core | **DONE, awaiting merge** — PR #95, v0.12.0 |
| 3 | Core API ergonomics (query2_mut + push/pop scene) | NEXT |
| 4 | Dialogue primitive (DialogueBox + typewriter) | open |
| 5 | WASM persistence (localStorage save) | open |
| 6 | Particle depth | open |
| 7 (stretch) | WASM audio | open |

## Code Analysis (Phase 2 anchors + landmarks)

- **TimeScale injection:** `src/app/schedule.rs::run_systems` — `let scaled_dt = dt * time_scale;`
  then in the system loop `let sys_dt = if i < tail_start { scaled_dt } else { dt };
  self.systems[i].run(&mut self.world, sys_dt);`. `RealDt` written just after computing `scaled_dt`.
- **`App::set_time_scale` / `time_scale`** live in the `impl App` block in `schedule.rs` (after
  `set_system_panic_policy`).
- **`Tween` generic:** `value()` = `T::lerp(&self.start, &self.end, self.easing.apply(self.timer.fraction()))`.
  `Lerp` is impl'd for `f32`/`Vec2`/`[f32;4]`/`Color` in `tween.rs`.
- **`Easing::out_bounce` helper** is private; `EaseInBounce = 1 - out_bounce(1-t)`; elastic uses
  `C4 = TAU/3`, guards t≤0/≥1.
- **FadeTransition has NO public getters** (`finished`/`alpha` are private) — the demo times the
  fade-out→fade-in flip itself with `RealDt`. App auto-updates the resource + renders the overlay.
- **`DrawText::new(text, pos, size, color: impl Into<Color>)`** — accepts `Color` directly (the audit's
  "[u8;4]" note was wrong). `TextQueue` is screen-space, top-left origin.
- **`Camera::shake(strength, duration)`** at `src/camera.rs:175`; Camera is a core resource.
- **`PostProcessConfig`** (`src/renderer/post_process.rs`) is `enabled:false` by default — the demo
  inserts it with `enabled:true` and drives `vignette_strength` each frame.

## Files Changed (all on branch `feat/phase2-game-feel`, in commit `31407f8`)

See the table above. New: `examples/juice_demo.rs`. Modified: tween.rs, schedule.rs, resources.rs,
core_resources.rs, lib.rs, timeline_panel.rs, CHANGELOG.md, CLAUDE.md, Cargo.toml, Cargo.lock.

### Memory (updated for Phase 1 in seq 13; needs Phase-2 update after merge)
- `engine-current-state.md` currently reflects seq-13/Phase-1 (v0.11.1). **Update after #95 merges**
  to main @ <new>, v0.12.0, Phase 2 done.

## User Feedback & Preferences (REQUIRED)

- **Direction (carried):** "raise satisfaction for people who fork/use it"; chose feature/core work
  over crates.io publish (publish deferred — fork-first ⇒ GitHub is primary).
- **Context management matters to the user** → the phased plan; each phase is one session-sized PR.
- **Sequencing this session:** "머지 진행하고 /handoff 이후 phase2 진행" (merge #93 → handoff → do
  Phase 2). Then after Phase 2 went green, the user invoked `/handoff` again (pausing here).
- **Merge pattern:** the user explicitly authorizes merges per PR this session (granted #93, #94).
  **#95 has NOT been authorized** — they invoked `/handoff` instead of confirming. Re-ask next time.
- **Standing:** Korean prose to user / English code+docs+handoff; Sonnet subagents w/ explicit model
  ([[new-model-subagent-incompat]]); never tag/publish unprompted; merge authority re-confirm each
  session + per outward action; beginner glossary is the Korean-doc exception.

## Where We're Going

1. **Merge PR #95** (needs auth — see "FIRST ACTION"). Then update memory; optionally tag `v0.12.0`.
2. **Phase 3 — core API ergonomics (MINOR → 0.13.0):**
   - `query2_mut<A,B>` (mutable multi-component query) via the disjoint split-borrow pattern
     `query_mut` (`src/ecs/world.rs:358`) already proves; then **refactor the flagship WASM demo**
     (`src/lib.rs::run_demo`, the collect-then-`get_mut` anti-pattern) to use it. Consider
     `query3_mut` / `query_opt3`. Cross-link from `FORKING.md`'s borrow-pattern note.
   - `App::push_scene` / `App::pop_scene` thin wrappers (mirror `set_scene` at `src/app/scenes.rs:62`);
     use in `scene_flow` to validate.
3. Phases 4–7 per the plan (Dialogue, WASM save, particle depth, WASM audio).

## Risks & Blockers

- **PR #95 is UNMERGED** — Phase 2 is not on main until it merges. Branch `feat/phase2-game-feel` holds
  the only copy of the work (pushed to origin, so safe).
- **main is PR-only** (branch protection, 4 checks, enforce_admins, strict). Merge auth is per-session
  + per-action. `strict` ⇒ if main moves, rebase #95 before merge (currently CLEAN).
- **`Easing` `#[non_exhaustive]`** is a one-time break — fine pre-1.0, noted in CHANGELOG; any future
  in-repo exhaustive match on Easing must add a `_` arm.
- **No GPU in CI** — juice_demo visual correctness relied on the macOS screencapture playtest (done).

## Open Questions

- Merge #95 now? (default: yes once the user confirms — it's green.)
- Continue into Phase 3 this session, or pause? (user picks.)
- Tag releases per phase (`v0.11.1`, `v0.12.0`)? Not done yet — tags are an explicit outward action.

## Reusable Gotchas (carry forward)

- **`cargo fmt` reformats fresh test asserts / ternaries** — a multi-line `assert!(cond, "msg")` and a
  long `if/else` expr tripped `fmt --check`. Run `cargo fmt` before the gate, or expect verify to fail
  on fmt first. (Memory: re-Read a file after `cargo fmt` before editing it again.)
- **`gh pr checks <n> --watch` race:** if run before GitHub registers the workflow it prints
  "no checks reported" and exits **0 instantly** (false green). Poll `gh pr checks <n>` until it stops
  saying "no checks reported", THEN `--watch`. (Hit this on PR #94.)
- **Don't append `; echo $?` to a backgrounded gate** — the background task's reported exit code
  becomes the echo's (always 0), masking the gate's real exit. Run the gate as the sole command
  (task exit = gate exit) and/or judge by log content (`grep "all checks passed"`).
- **NEVER pipe a gate to tail/head** (carried) — masks exit code.
- **macOS playtest pattern (works):** `cargo build --example <name>` first, launch binary, poll
  `osascript ... exists (process "<name>")`, set window pos/size, `screencapture -x -R<region>`, kill.
  Send a key with `osascript -e 'tell application "System Events" to key code 49'` (49 = space).
- **Pre-1.0 (0.x):** feature/breaking → MINOR, docs/fix → PATCH; never 1.0.0, never 10.x.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git branch --show-current        # feat/phase2-game-feel (Phase 2 lives here, UNMERGED)
gh pr view 95 --json state,mergeable,mergeStateStatus   # OPEN / MERGEABLE / CLEAN
git log --oneline -3             # 31407f8 Phase 2; 0e44a24 (#94) is main's HEAD

# Read first:
#   THIS handoff (seq 14)
#   plans/USER_EXPERIENCE_PLAN_2026-06-17.md  (Phase 3 is next)
#   parent: HANDOFF_engine-hardening_ux-roadmap-phase1_2026-06-17.md (seq 13)

# FIRST ACTION: re-confirm merge auth, then
#   git checkout main && gh pr merge 95 --squash --delete-branch && git pull --ff-only origin main
#   (then update engine-current-state memory; optionally tag v0.12.0 if asked)

# PROCESS GUARDS (in force):
#   - main PR-only; NEVER pipe a gate to tail; poll before `gh pr checks --watch` (race).
#   - Pre-1.0 (0.x): feature/breaking → MINOR, docs/fix → PATCH; never 1.0.0 / 10.x.
#   - merge authority per-session + per outward action — re-confirm before merging.

# THEN: Phase 3 — query2_mut (+ refactor src/lib.rs run_demo) + App::push_scene/pop_scene → 0.13.0.
```
