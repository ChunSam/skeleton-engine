# User-experience roadmap — Phase 4 shipped (dialogue primitive)

**Date:** 2026-06-17
**Status:** Phase 4 MERGED (PR #98, v0.14.0, CI-green). `main` @ `c184987`, clean. Driving the roadmap
autonomously per `/goal` (each phase: PR → CI green → merge → handoff). **Phase 5 (WASM save) next.**
**Chain:** `engine-hardening` seq `16` · **Parent:** seq 15 (`HANDOFF_engine-hardening_phase3-ergonomics_2026-06-17.md`)
**Prior:** seq 13 (P1 v0.11.1) → 14 (P2 v0.12.0) → 15 (P3 v0.13.0) → **16 (P4 v0.14.0)**

## The standing goal (user `/goal`)

Drive `plans/USER_EXPERIENCE_PLAN_2026-06-17.md` to its last phase (7). Per phase: implement → PR →
**CI green → merge** (goal grants standing merge authority on green) → handoff. Handoff docs are
committed bundled into the NEXT phase's PR to save CI cycles (seq-15 rode in #98).

## ⚠️ Environment note (carries forward)

**The machine locked mid-session** (display asleep → lock screen). Consequences for the remaining
phases: **live playtests are blocked** — `screencapture` returns black/lock-screen while locked, and
the **wasm smoke test (Chrome) can't run**. Phase 4's `dialogue_demo` was therefore NOT eyeballed live;
it relies on 5 unit tests + the `TextQueue`/`DrawText` render path already visually verified in
`juice_demo`/`hello_sprite` this session. Phases 5 (WASM save) and 7 (WASM audio) need browser
verification that is currently blocked — implement + unit-test + rely on CI compile/clippy; defer the
browser round-trip check until unlocked. Phase 6 (particles) native visual check is also deferred.

## Since Last Handoff (seq 15)

Seq 15 said Phase 4 (dialogue) next. This session: merged #97 (Phase 3) + shipped Phase 4 as **#98**
(v0.14.0). Two CI-class issues caught locally + fixed before green (see Gotchas).

## Where We Are

- **`main` @ `c184987`**, package **v0.14.0**, CLAUDE.md header **v1.6.62**, clean, only `main` branch.
- **783 lib tests** (+5 dialogue tests over Phase 3's 778). Full `verify.sh` green (incl. wasm clippy).
- Roadmap: **Phases 1–4 DONE + merged**; Phase 5 next; 6, 7 open.

## What We Tried (Phase 4)

1. **`src/dialogue.rs`: `DialogueBox` + `DialogueSystem`.** `DialogueBox` = `speaker` + `Vec<String>`
   lines, `current`, `chars_per_sec`, private `elapsed` + `full` flag, optional `portrait`
   (`#[serde(skip)]`). Methods: `new`/`with_chars_per_sec`/`with_portrait`, `tick(dt)`, `current_line`,
   `visible_text` (UTF-8-safe substring via `char_indices().nth(n)`), `line_fully_revealed`, two-stage
   `advance()` (press 1 → set `full`; press 2 → next line / finish), `is_finished`, `reset`.
   `DialogueSystem` (plain unit struct — NOT `#[derive(Default)]`, see gotcha): ticks all boxes via
   `world.query_mut::<DialogueBox>()` (the Phase-3 API), then collects (speaker, visible_text, full) for
   non-finished boxes and renders via `TextQueue`/`DrawText` near the viewport bottom. Input-agnostic.
2. **Re-exports** in `src/lib.rs` (`pub mod dialogue;` + `pub use dialogue::{DialogueBox, DialogueSystem};`).
3. **`examples/dialogue_demo.rs`** — `ConversationSystem` feeds Space → `advance()`, swaps in the next
   `(speaker, lines)` segment when one finishes; 3 segments, 2 speakers.
4. **Docs:** CHANGELOG `## 0.14.0`, CLAUDE.md module-map row.
5. **Playtest blocked by lock** (see Environment note) — full-screen capture returned the macOS lock
   screen; not eyeballed live.

## Key Decisions

- **`DialogueSystem` is a plain unit struct** (no `Default` derive) — clippy `default_constructed_unit_structs`
  errors on `X::default()` for a fieldless struct; constructed as `DialogueSystem`.
- **Render is text-only** (speaker + revealed text + hint via `TextQueue`); no background panel and the
  `portrait` field is left for the game to draw — keeps the primitive composable and avoids a
  screen-space-rect dependency. Localization = resolve keys into the lines before constructing the box.
- **Typewriter `advance` is two-stage** (complete-then-next) — the common dialogue UX; needs no resolved
  text length (works for any lines).

## Evidence & Data

| PR | main after | Title | CI |
|---|---|---|---|
| #97 | `d33f7d1` | Phase 3 ergonomics (v0.13.0) | merged |
| #98 | `c184987` | Phase 4 dialogue (v0.14.0) | 4/4 (after clippy+rustdoc fixes) |

Phase 4 files: `src/dialogue.rs` (new), `src/lib.rs`, `examples/dialogue_demo.rs` (new),
`docs/CHANGELOG.md`, `CLAUDE.md`, `Cargo.toml`/`Cargo.lock`. (seq-15 handoff also rode in #98.)

## Code Analysis (Phase 5 anchors — WASM save)

- **`src/save.rs`** — `save`/`load`/`save_versioned`/`exists`/`delete` return `Unsupported`/`false` on
  `cfg(target_arch = "wasm32")`. Phase 5 adds a `localStorage` backend there.
- **`web-sys` features** (`Cargo.toml` wasm deps, ~L182) currently lack **`Storage`** — must add it
  (and likely use `window().local_storage()`). `js-sys` is available.
- Native save uses **AEAD (chacha20poly1305)** → binary. localStorage stores **strings**: either
  base64-encode the bytes, or store the RON/plaintext directly and document that wasm save is unencrypted
  (or use a wasm-compatible primitive). Decide & document the wasm vs native crypto difference.
- **Acceptance:** plan says verify via `./scripts/wasm_smoke.sh` — **blocked while locked**; do CI compile
  + keep native tests green, and defer the browser round-trip. Pair with extending `coin_race` to persist a
  high score.

## Where We're Going

- **Phase 5 — WASM persistence (0.15.0):** `localStorage` save backend in `save.rs` (+ web-sys `Storage`
  feature). Keep native byte-identical. Defer browser smoke test (locked).
- **Phase 6 — particle depth (0.16.0):** `ParticleEmitter` gravity / angular velocity / emit-shape (+
  `GpuParticleEmitter`), `particles_showcase` example. Unit-test the math; defer visual.
- **Phase 7 — WASM audio (stretch):** wasm SFX path; defer browser check.

## Risks & Blockers

- **Locked machine** → no live/browser verification for 5/6/7 (see Environment note). Lean on unit tests + CI.
- **main PR-only**; merge authority standing-on-green per `/goal` (the auto-mode classifier once gated a
  bundled merge command — run `gh pr merge` standalone).
- **wasm clippy** is now in `verify.sh` (catches wasm-only lint locally).

## Reusable Gotchas (carry forward)

- **clippy `default_constructed_unit_structs`** — don't `#[derive(Default)]` + `::default()` a fieldless
  unit struct; construct it directly. (Tripped `DialogueSystem`.)
- **rustdoc `redundant_explicit_links`** (`-D warnings`) — `[`Foo`](path::Foo)` where the label `Foo`
  itself resolves is an ERROR; write `[`Foo`]`. (Tripped a `TextQueue` link.) Also bare `[`x`]` that
  doesn't resolve errors → qualify as `[`x`](Path::x)`.
- **wasm clippy** (`cargo clippy --target wasm32 --lib -D warnings`) — now in verify.sh; catches wasm-only
  unused imports the wasm *build* only warns on.
- **`gh pr checks --watch`** — poll until checks register first (else instant false-0); watch the latest
  run after rapid pushes; don't append `; echo $?` (masks task exit — capture in log instead).
- **`cargo fmt` before the gate** (reformats fresh asserts/ternaries). Pre-1.0: feature→MINOR, fix/docs→PATCH.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # c184987 Phase 4 (#98)
grep -m1 '^version' Cargo.toml  # 0.14.0
./scripts/verify.sh             # green (incl. wasm clippy); RUN AS-IS, no tail pipe
# Read: plans/USER_EXPERIENCE_PLAN_2026-06-17.md (Phase 5 next), this handoff (seq 16).
# Goal: phases 5→7, each PR → CI green → merge → handoff. Standing merge authority on green.
# NEXT: Phase 5 — WASM save via localStorage in src/save.rs (+ web-sys Storage feature); defer the
#   browser smoke test if the machine is still locked.
```
