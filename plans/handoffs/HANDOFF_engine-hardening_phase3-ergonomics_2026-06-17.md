# User-experience roadmap — Phase 3 shipped (core API ergonomics)

**Date:** 2026-06-17
**Status:** Phase 3 MERGED (PR #97, v0.13.0, CI-green). `main` @ `d33f7d1`, clean. Driving the
roadmap autonomously per the user's `/goal` (each phase: PR → CI green → merge → handoff). **Phase 4
(dialogue) is next.**
**Bead(s):** none (`bd` unavailable)
**Epic:** skeleton-engine — engine hardening / fork-friendliness
**Chain:** `engine-hardening` seq `15`
**Parent:** `HANDOFF_engine-hardening_phase2-game-feel_2026-06-17.md` (seq 14)
**Prior chain:** seq 13 (Phase 1, v0.11.1) → seq 14 (Phase 2, v0.12.0) → **this (15, Phase 3, v0.13.0)**

## The standing goal (user `/goal`)

> "계획서에 작성된 마지막 phase 완료까지, 한 phase 진행하면 handoff하고 컨텍스트 정리. 각각 pr 진행 후
> ci 그린 확인되면 머지 진행."

Drive `plans/USER_EXPERIENCE_PLAN_2026-06-17.md` to its **last phase (7, WASM audio)**. Per phase:
implement → PR → **CI green → merge** (the goal grants standing merge authority on green) →
handoff + context cleanup. This handoff doc is committed bundled with the NEXT phase's PR to save
CI cycles.

## Since Last Handoff (seq 14)

Seq 14 said: merge #95 (Phase 2), then Phase 3 next. This session:
- Merged **#95** (Phase 2, v0.12.0) + **#96** (seq-14 handoff).
- Implemented + shipped **Phase 3** as **#97** (v0.13.0): `query2_mut`/`query3_mut`, `App::push_scene`/
  `pop_scene`, refactored the flagship WASM demo off the collect-then-`get_mut` anti-pattern.
- Hit + fixed a **wasm-clippy CI gotcha** (below) and hardened `verify.sh` so it can't recur.

## Where We Are

- **`main` @ `d33f7d1`**, package **v0.13.0**, CLAUDE.md header **v1.6.61**, clean, only `main` branch.
- **778 lib tests** (+1 `query2_mut` test over Phase 2's 777). Full `verify.sh` green (now incl. wasm clippy).
- Roadmap: **Phases 1, 2, 3 DONE + merged**; Phase 4 next; 5, 6, 7 open.
- No work in flight. No wakeups armed.

## What We Tried (Phase 3)

1. **`query2_mut<A,B>` / `query3_mut<A,B,C>`** in `src/ecs/world.rs` (after `query_mut`). The hard part —
   two distinct archetype columns need simultaneous `&mut` — is solved with **`HashMap::get_disjoint_mut`**
   (stable since Rust 1.86; MSRV is 1.95 so it's available). Pattern: destructure `Archetype { entities,
   columns, .. }`, `let [ca, cb] = columns.get_disjoint_mut([&ta, &tb])`, then
   `entities.iter().zip(ca.iter_mut()).zip(cb.iter_mut())` → downcast_mut. `A`/`B`/`C` must be distinct.
   Unit test `query2_mut_mutates_both_components` (mutates both + skips entity missing one).
2. **`App::push_scene`/`pop_scene`** in `src/app/scenes.rs` — thin wrappers over `apply_scene_cmd(Push/Pop)`,
   mirroring `set_scene` (Replace). For pause-menu / overlay stacking.
3. **Refactored `run_demo`** (`src/lib.rs`, the wasm demo) from collect-then-`get_mut` to
   `for (_e, t, b) in world.query2_mut::<Transform, BounceVel>()` — the flagship demo no longer teaches
   the anti-pattern.
4. **Docs:** `FORKING.md` borrow-split section now points to `query_mut`/`query2_mut`; `CLAUDE.md` module
   map documents the mutable queries + scene-stack helpers; CHANGELOG `## 0.13.0`.
5. **verify → rustdoc FAILED** on a bare intra-doc link `` [`pop_scene`] `` → qualified to `(App::pop_scene)`.
6. **Pushed PR #97 → CI Build (WASM) FAILED** though local verify was green. Cause: CI's Build (WASM) job
   runs `cargo build` **AND** `cargo clippy --target wasm32 --lib -- -D warnings`; the run_demo refactor
   left `Entity` unused in the wasm-gated `use` (`src/lib.rs:171`) — a *warning* on build (local pass) but
   an *error* under wasm clippy `-D warnings` (CI). Removed the import; added the wasm-clippy step to
   `scripts/verify.sh` so the local gate is now truly CI-equivalent.
7. **Merged #97 on green** (4/4) per the goal.

## Key Decisions

- **`get_disjoint_mut` over an `iter_mut`-collect or unsafe split** — purpose-built, safe, stable on 1.95;
  cleanest way to borrow two map entries mutably at once.
- **Added `query3_mut` alongside `query2_mut`** (parallel to the immutable `query2`/`query3`); skipped
  `query_opt*_mut` (low value for now).
- **Hardened `verify.sh` with wasm clippy** rather than just fixing the one import — prevents the whole
  class of "wasm-only lint passes local build, fails CI" for the remaining phases.
- **Bundling each handoff into the next phase's PR** (not a standalone PR) to cut CI cycles across the
  4 remaining phases, while still committing it durably.

## Evidence & Data

### PRs this session

| PR | main after | Title | CI |
|---|---|---|---|
| #95 | `7f52c92` | Phase 2 game-feel (v0.12.0) | 4/4 (merged) |
| #96 | `09be19f` | seq-14 handoff | 4/4 (merged) |
| #97 | `d33f7d1` | Phase 3 ergonomics (v0.13.0) | 4/4 after wasm-clippy fix (merged) |

### Files changed (Phase 3, #97)
- `src/ecs/world.rs` (+query2_mut/query3_mut), `src/ecs/world/tests.rs` (+test),
  `src/app/scenes.rs` (+push/pop), `src/lib.rs` (run_demo refactor + Entity import drop),
  `FORKING.md`, `CLAUDE.md`, `docs/CHANGELOG.md`, `scripts/verify.sh` (+wasm clippy), `Cargo.toml`/`Cargo.lock`.

## Code Analysis (Phase 4 anchors — gathered)

- **`LocaleResource::t(&key) -> &str`** (translated string; `src/ui/localized.rs` uses it). For DialogueBox
  localization-key support.
- **`LocalizedText { key: String }`** (`src/ui/localized.rs:29`) + `LocalizationSystem` (resolves keys into
  Label/Button each frame). Model for key-driven text.
- **`examples/games/settings_menu/settings_menu.rs`** hand-rolls a dialogue box (line-index advance, localized
  via `dialogue.*` keys, `Space: next`) — the reference to replace with a real primitive.
- **Screen-space text** = `TextQueue` + `DrawText::new(text, pos, size, impl Into<Color>)` (top-left anchor),
  as used in `juice_demo`/`security_camera`. Typewriter = substring of body by `elapsed * chars_per_sec`.
- UI widgets (`Panel`/`Label`, re-exported `src/lib.rs:148`) are screen-space via `LayoutSystem` — heavier;
  a text-first DialogueBox via `DrawText` is simpler and avoids per-frame UI entity churn.

## Where We're Going

**Phase 4 — Dialogue primitive (MINOR → 0.14.0).** New `src/dialogue.rs`: a `DialogueBox` component
(speaker, body/lines, press-to-advance, typewriter `chars_per_sec`, optional portrait
`Option<Handle<ImageAsset>>`, optional localization key) + a `DialogueSystem` (ticks the reveal, handles
advance, renders speaker+revealed text via `TextQueue`). Re-export `engine::DialogueBox`; register serde
(+ maybe editable). Example `dialogue_demo` (2–3 NPC conversation w/ typewriter + advance). Decide:
typewriter on real vs scaled time (recommend real via `RealDt`, or document). Then 5 (WASM save), 6
(particle depth), 7 (WASM audio).

## Risks & Blockers

- **main PR-only** (branch protection, 4 checks incl. wasm clippy in Build (WASM), enforce_admins, strict).
- **Merge authority:** the `/goal` grants standing "CI green → merge" — but the auto-mode classifier may
  still gate a `gh pr merge`; if denied, the explicit goal + a per-action confirm resolves it (happened with
  #95: a bundled merge command was denied, the standalone merge after explicit user go succeeded).
- **`gh pr checks --watch` races** (carried) — poll until checks register first; two quick pushes can spawn
  two runs and confuse a watch (happened on #97 — re-watched the live run).

## Reusable Gotchas (carry forward)

- **NEW — wasm clippy:** CI's Build (WASM) runs `cargo build` **+** `cargo clippy --target wasm32 --lib -D
  warnings`. The build only WARNS on wasm-only unused imports; clippy ERRORS. `verify.sh` now includes the
  wasm clippy step — run the full `verify.sh` (not just `cargo build` wasm) before pushing wasm-affecting code.
- **`gh pr checks --watch` exits 0 instantly if checks aren't registered yet** ("no checks reported") — poll
  `gh pr checks <n>` until it lists checks, THEN watch. Two rapid pushes supersede runs; watch the latest.
- **Don't append `; echo $?` to a backgrounded gate** — task exit becomes the echo's; judge by log content.
- **`cargo fmt` before the gate** — it reformats fresh asserts/ternaries; run it first to avoid a fmt-fail cycle.
- **Pre-1.0 (0.x):** feature/breaking → MINOR, docs/fix → PATCH; never 1.0.0 / 10.x.
- **rust-analyzer phantoms** (ColliderHandle E0308, unlinked-file, inactive-code) are stale — trust cargo/CI.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # d33f7d1 Phase 3 (#97); main
grep -m1 '^version' Cargo.toml  # 0.13.0
./scripts/verify.sh             # green (now incl. wasm clippy); RUN AS-IS, no tail pipe

# Read: plans/USER_EXPERIENCE_PLAN_2026-06-17.md (Phase 4 next), this handoff (seq 15).
# Goal: drive phases 4→7, each PR → CI green → merge → handoff. Standing merge authority on green.
# NEXT: Phase 4 — src/dialogue.rs DialogueBox + typewriter + dialogue_demo (v0.14.0).
#   anchors: LocaleResource::t, LocalizedText, settings_menu hand-rolled dialogue, TextQueue/DrawText.
```
