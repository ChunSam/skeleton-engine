# CLAUDE.md — skeleton-engine agent reference

> Version v1.6.245 | package `skeleton-engine` v0.141.2, library crate `engine` | wgpu-based Rust 2D game engine (wgpu 29, MSRV 1.95, CI pin Rust 1.95.0) | **Cargo workspace** (members `.` + `engine_reflect_derive` proc-macro)  
> WASM support: `cargo build --target wasm32-unknown-unknown` passes; an example ships to the web via `cargo build --example` + `wasm-bindgen` (see `examples/games/coin_race/web/`)  
> **Where is X? → `docs/MODULE_MAP.md`** (grep it) | Full API: `REFERENCE.html` | dev history / architecture decisions: `docs/HANDOFF.md`  
> **Versioning: pre-1.0 (0.x)** — MINOR = any release (incl. breaking), PATCH = bugfix; 1.0.0 later. (Reset from 10.7.0, 2026-06-17, pre-publish — see CHANGELOG 0.11.0.)

---

## Conversation language

- **User-facing reports/questions → Korean; everything else → English** — agent-to-agent
  (subagent/Workflow prompts, handoffs), code, paths, identifiers, command output, and
  file-written docs (see Documentation rules). User ruling 2026-06-18; supersedes the harness default.

---

## Verification (run before declaring done)

A code/refactor change is **not done** until the **CI-equivalent** checks pass
*locally*. CI (`.github/workflows/ci.yml`) enforces these on every push, but run them
**before committing** so a regression never reaches `main`:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --target wasm32-unknown-unknown   # lib+bins only — NOT --all-targets
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings  # wasm-only lints
cargo test --all-targets
cargo test --doc                                # --all-targets skips doctests
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps  # CI fails docs on broken intra-doc links
```

Or run all of them in order via `./scripts/verify.sh`.

- **Read the gate's exit code, don't pipe it:** `./scripts/verify.sh > /tmp/verify.log 2>&1;
  echo "VERIFY_EXIT=$?"` is the authoritative verdict. A trailing `| tail` reports the pipe's
  `0` and **hides** a real failure; `;` does not stop on failure (so a chained `git commit`
  runs anyway); a `run_in_background` notification reports the trailing `echo`, not the gate;
  a **stale `.exit` file from a previous session** makes an `until [ -f … ]` waiter return an
  old code instantly. In zsh the pipe-status array is **`$pipestatus`, 1-indexed** —
  `${PIPESTATUS[0]}` is always empty. All six traps, with the sessions they cost:
  **`docs/VERIFICATION.md`**.
- **Don't narrow the bar:** a prior "done" on only `fmt --check` + `test --lib` shipped the wasm-build + clippy regressions the full gate list above catches.
- **The gate has three blind spots** — the wasm step is **lib+bins, never examples** (build a
  touched example explicitly: `cargo build --example <name> --target wasm32-unknown-unknown`);
  **CI is ubuntu only**, so an OS-gated change needs a local build on that OS + a real check;
  and **compiling for wasm is not running on wasm**. Details + the local render smokes (Chrome,
  non-CI) and the lavapipe `render` job: **`docs/VERIFICATION.md`**,
  `docs/WASM_SMOKES.md`, `docs/RENDER_TESTING.md`.

---

## Project direction (read `docs/VISION.md`)

A hackable, MIT-licensed, genre-agnostic 2D engine meant to be forked and extended.
Priorities — (1) open-source skeleton others can fork, (2) personal foundation for 2D
games, (3) learning vehicle. Feature work follows the core loop from `docs/VISION.md`:

- **A new feature is not done until a small, playable example game in `examples/`
  exercises it in real play.** The example is the acceptance test.
- **If the API feels awkward while writing that example, fix the API before release.**
- Keep new code fork-friendly: clear module boundaries and extension points. Breadth
  first, but not by leaving an unreadable mess.

---

## Module map

**Moved to `docs/MODULE_MAP.md`** — 72 rows of "where do I read to find X". It is *not*
auto-loaded into a session, which is the point: **grep it for your topic**
(`grep -in audio docs/MODULE_MAP.md`) instead of carrying all 72 rows. Record every new
feature there — **extend an existing row before adding one**.

---

## Core patterns & task recipes

Detailed in **`docs/PATTERNS.md`**:

- **Architecture patterns** — ECS query API (`query2`/`query_opt2`), borrow-checker
  workaround (collect entities then `get_mut`), render-layer separation
  (`AnimationSystem` → `UvRect` → renderer), **render-target-format-aware pipeline cache**
  (a new render pass keys its pipeline by *target* format, not `gpu.config.format`, so it
  survives an offscreen/HDR RT — else it silently vanishes; sprite/UI/material/GPU-particle),
  UI system order (`LayoutSystem` before `UiSystem`), animation state-machine order
  (`StateMachineSystem` after `AnimationSystem`), `PhysicsWorld` encapsulation accessors,
  **shared policy for cfg-split backends** (a value both derive lives in ONE un-gated module —
  duplicated policy compiles and silently diverges), **real-time audio-thread producers**
  (no lock/alloc in `Source::next`; a stoppable producer needs a liveness counter or it freezes),
  **surviving a scene reset** (a scene swap rebuilds the `World`; a resource not registered via
  `register_persistent` is silently replaced by the engine default — `WindowConfig`/`SceneTransition`/
  `TextMeasurer`/`InputScript`/the 7 RON registries, the v0.139.1 config family and **`Audio`**
  (v0.141.1) are automatic, **but a type the game itself defines is the game's job**. Line:
  auto-persist what the engine defines and only reads, whoever inserts it — plus `Audio`, which the
  engine drives but which owns an OS device handle. Includes the v0.139.1 audit's negative result
  (the 20 that are correctly scene state, so it is not re-run) and what would reopen `Audio`).
- **Task recipes** — adding a component / system / resource / event, and scene transitions.

---

## Agent working notes

Context-management heuristics (when to split work into a subagent by task type),
efficient-exploration order, and subagent-prompt principles → **`docs/AGENT_NOTES.md`**.

---

## Documentation rules

- **Language**: doc prose (incl. new `docs/HANDOFF.md` entries) in **English** — ≈⅓ the tokens
  of Korean; code, paths, identifier tables and API names stay as written. Korean is kept only
  for the beginner glossary (`docs/ENGINE_TERMS_FOR_BEGINNERS.md`) and personal/gitignored
  one-off prompt or plan docs.
- **Length**: keep `CLAUDE.md` / `AGENTS.md` **≤200 lines**. Prefer concision, but **never drop
  needed content to hit the limit** — move the detail into a `docs/*.md` and leave a one-line
  pointer here (how `docs/MODULE_MAP.md`, `docs/PATTERNS.md`, `docs/AGENT_NOTES.md` and
  `docs/VERIFICATION.md` were split out).
- ⚠️ **Lines are a poor proxy for context cost, and this file is auto-loaded every session.**
  The module map was **88 KB — 94% of this file's bytes — in 41% of its lines**, one row
  reaching 8,012 characters, all of it invisible to a line cap (2026-07-30 measurement; that
  is why it moved out). So: if a section grows **dense** rather than **long**, split it out —
  and never merge rows to buy line headroom, which shrinks the count while the cost stays.

---

## Document map

| Document | Purpose |
|------|------|
| `CLAUDE.md` (this file) | Agent quick reference — conventions, verify gate, task checklists |
| **`docs/MODULE_MAP.md`** | **"Where do I read to find X" — 72 rows, one per subsystem (extracted from this file). Grep it; don't read it whole.** |
| `docs/PATTERNS.md` | Core architecture patterns + task recipes (extracted from this file) |
| `docs/VERIFICATION.md` | Why behind the verify gate — its 6 exit-code traps and its 3 blind spots (extracted from this file) |
| `docs/AGENT_NOTES.md` | Agent working heuristics — context-management split, exploration order, subagent-prompt principles (extracted from this file) |
| `docs/MACOS_FFI.md` | How to add an objc2 Apple-framework binding (version-pin, discover the API from registry source, feature flags) — e.g. the macOS GameController gamepad backend |
| `docs/RENDER_TESTING.md` | CI-verifying the GPU render path — `tests/render.rs` + the lavapipe `render` job |
| `REFERENCE.html` | Full public API + code examples (detailed) |
| `docs/HANDOFF.md` | Per-phase dev history, background on architecture decisions |
