# CLAUDE.md — agent quick reference

Package `skeleton-engine`, library crate **`engine`** — all code says `use engine::*`.
The version lives in `Cargo.toml` and nowhere else.

The engine is **unpublished by design**: it is a *skeleton* meant to be forked and edited, not
added as a dependency. Read `docs/VISION.md` once — it is the "why" every feature traces back to.

> **This file stays ≤ 200 lines.** It is auto-loaded into every session, so detail belongs in
> `docs/*.md` with only a one-line pointer here. Do not paste module tables or long rationale back
> into it.

---

## Orientation — read this, not the whole tree

| Question | Where |
|---|---|
| "Where is X implemented?" | `docs/MODULE_MAP.md` — one dense row per subsystem. **Grep it, never read it whole** (`grep -in 'audio' docs/MODULE_MAP.md`) |
| "What is the public API?" | `src/lib.rs` — the full re-export list, the fastest map of what exists |
| "How do I write this correctly?" | `docs/PATTERNS.md` — architecture patterns + task recipes |
| "Why does the gate look like that?" | `docs/VERIFICATION.md` — the traps that have actually bitten |
| "How should I work?" | `docs/AGENT_WORKFLOW.md` (scope/verify/report rules), `docs/AGENT_NOTES.md` (context + subagent heuristics) |
| "What changed, and why?" | `docs/CHANGELOG.md` (release-facing), `docs/HANDOFF.md` + `plans/handoffs/` (session history) |
| "What should I build next?" | `docs/NEXT_WORK.md` — the live backlog; **start with its board gate**. `docs/ROADMAP.md` for milestones, `docs/PROGRAM_HISTORY.md` for the finished candidate A–O program |
| GPU render tests / wasm smokes | `docs/RENDER_TESTING.md`, `docs/WASM_SMOKES.md` |
| User-facing docs | `README.md`, `FORKING.md`, `REFERENCE.html` + `ARCHITECTURE.html` (Korean) |

Exploration order: `rg`/`grep` for the symbol → `src/lib.rs` → the `MODULE_MAP` row → the file.
Read only what you need.

---

## Build & run

Any `examples/*.rs` is auto-discovered as `cargo run --example <name>`; `hello_sprite` is the
smallest. `cargo build --target wasm32-unknown-unknown` for wasm, `./scripts/build_wasm.sh` for a
servable bundle in `dist/`.

Headless, no window or display needed (native only) — the way to *see* what a change did:

- `HEADLESS_SHOT=/tmp/x.png cargo run --example <name>` — writes a PNG, in the examples that opt in
  (about 40% of them; `grep -l HEADLESS_SHOT examples/*.rs` to check).
- `ENGINE_CAPTURE=<frame>:<path.png>[,…]` — any game: render headlessly at those frames, then exit.
- `ENGINE_INPUT=<script.ron>` — replay a scripted `(frame, action)` input list (`src/input_script.rs`).

⚠️ A capture run advances at a fixed `1/60` dt as fast as the CPU allows, so **real-time signals
cannot be photographed** — an audio meter reads `0.0` in a captured frame while sounding correctly.

---

## Verification

`./scripts/verify.sh` is the gate — it mirrors CI. Run it before calling anything done.

```sh
./scripts/verify.sh > /tmp/v.log 2>&1; echo "VERIFY_EXIT=$?"
```

**Read the exit code from an unpiped command.** `docs/VERIFICATION.md` documents seven traps that
have each cost a session; the ones that recur:

- A trailing `| tail` reports the pipe's status and hides a red gate. zsh's array is `$pipestatus`, 1-indexed.
- A background run's completion notification reports the *trailing* command's status — write
  `echo $? > /tmp/v.exit` and **read the file**, after `rm -f`-ing it first (a stale file matches instantly).
- `;` does not short-circuit — branch on the captured code before committing.

What the gate does **not** cover, so get it yourself:

- **Examples on wasm** — the wasm step is lib+bins only. Touched an example's wasm path?
  `cargo build --example <name> --target wasm32-unknown-unknown`.
- **macOS/Windows `cfg` branches** — CI is ubuntu (plus one Windows *build* job). Build both branches locally.
- **Anything CI cannot run** — windowed playtest, audio playback, hot-reload, gamepads. Compiling
  for wasm is not running on wasm; for a runtime web claim, run the matching `scripts/*_smoke.sh`.

If you skip a verification step, **say so in the report**.

---

## Core patterns

Full detail + code in `docs/PATTERNS.md`. The ones that bite hardest:

- **Borrow split** — you cannot `get_mut` while a query iterator is alive. Collect entities first,
  then mutate; or better, use `query_mut` / `query2_mut` / `query3_mut` and skip the collect.
- **Per-frame allocation** — a system that runs every frame keeps its temporaries as scratch fields
  (`clear()` + refill) or uses `std::mem::take`. One-shot/editor paths may allocate freely.
- **Render layer separation** — the renderer reads `UvRect`, never `AnimationPlayer`; `DebugDraw` is
  pure data converted at the `App` render stage.
- **Pipeline cache keyed by *target* format** — a pipeline compiled for the surface format fails
  validation against an offscreen `RenderTarget` or the HDR intermediate. Four renderers do this;
  skipping it makes a feature silently vanish under HDR/offscreen.
- **`queue.write_buffer` lands at submit time** — never re-upload one buffer between passes in a
  frame. Upload once + draw by byte-range, or pool per-frame renderers.
- **System ordering is explicit** — every built-in system has a `LABEL`; use
  `add_system_labeled(..., SystemConfig::new().after(X::LABEL))` rather than relying on insertion
  order. `LayoutSystem` before `UiSystem`, `StateMachineSystem` after `AnimationSystem`, etc.
- **Scene reset rebuilds the World** — every resource is dropped unless registered persistent.
  Ask of each new resource: *scene state* (correct to drop) or *session state* (config, device
  handles, caches → must persist)? The v0.139.1 audit classified all 27; don't re-run it.
- **cfg-split backends share the policy, not the implementation** — put a derived value's *formula*
  in an un-gated module both native and wasm call, or the two platforms drift silently.

## Adding things

Recipes for a component / system / resource are in `docs/PATTERNS.md` § Common task patterns.
Whatever you add: re-export it from `src/lib.rs` and add or extend a `docs/MODULE_MAP.md` row.
Two that have their own failure mode:

- **Event** → `#[derive(Clone)]` type → `app.register_event::<E>()`. **Engine-emitted events need
  this too** — an unregistered bus silently drops every send, which presents as "the event never
  arrives" while the engine is fine.
- **A hardcoded constant made configurable** → a `Copy` resource whose `Default` reproduces the old
  constants byte-for-byte, read with `unwrap_or_default()` at the call site. Additive; ship as MINOR.

---

## Conventions

**A feature is not done until a small playable example exercises it in real play.** The example is
the acceptance test, not an afterthought — if the API feels awkward while writing it, fix the API.

**Where things go.** Unit tests live inline (`mod tests`), beside the code they cover. `plans/` is
tracked — commit session plans and handoffs. `.claude/` is gitignored.

**Versioning.** Pre-1.0: MINOR covers any release including breaking changes, PATCH is a bugfix.
A release bump touches `Cargo.toml`, `Cargo.lock`, and `docs/CHANGELOG.md` together. Docs-only
changes take no version bump and no CHANGELOG entry.

**Commits.** `type(scope): summary (vX.Y.Z)` — e.g. `feat(audio): …`, `fix(app): …`,
`docs(patterns): …`. The `(#PR)` suffix is appended by the squash-merge, not typed. Bodies explain
*why* and record real-device evidence CI cannot produce.

**Git.** Stage, commit, and push **only when explicitly asked**. Never `git reset --hard`,
`git checkout --`, or force-push without prior confirmation, and never revert someone else's
in-flight changes.

**Confirm before:** removing or renaming public API, changing dependencies/versions, large
refactors, deleting files. The verification scope is this repo only — do not build or modify
external projects unless asked.
