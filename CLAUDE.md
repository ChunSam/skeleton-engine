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
| "What changed, and why?" | `docs/CHANGELOG.md` (release-facing) + the commit bodies, which are unusually detailed here. The 207-file `plans/handoffs/` archive and the frozen `docs/HANDOFF.md` were **deleted 2026-09-04** — recover either with `git show <sha>^:<path>`. A mid-task handoff is now the untracked 15-line `.claude/handoff.md` (the `handoff` skill), never a committed file |
| "What should I build next?" | `docs/NEXT_WORK.md` — the live backlog; **start with its board gate**. `docs/ROADMAP.md` for milestones, `docs/PROGRAM_HISTORY.md` for the finished candidate A–O program |
| GPU render tests | `docs/RENDER_TESTING.md` |
| User-facing docs | `README.md`, `FORKING.md`. The four Korean HTML docs were deleted 2026-08-20 — they described the pre-deletion examples tree and are to be rewritten |

Exploration order: `rg`/`grep` for the symbol → `src/lib.rs` → the `MODULE_MAP` row → the file.
Read only what you need.

---

## Build & run

⚠️ **All five games exist** (2026-08-21). The `examples/` tree — 22 games, ~85 demos — was deleted
2026-08-19 and rebuilt as five by `plans/2026-08-19-examples-rebuild-plan.md`. Each runs with `cargo
run --example <name>` and self-verdicts with `<NAME>_SELFTEST=1` (`netplay_game` needs
`netplay_server` too); `build_wasm_examples.sh` builds the 5 of 8 targets that are not native-only,
and `./scripts/build_wasm.sh` makes a servable `dist/` bundle (the lib's own `run_demo`, not an
example). **Every game installs a logger** (v0.154.3) — ⚠️ natively only; a browser discards it.

Read `docs/PROGRAM_HISTORY.md` + `docs/CHANGELOG.md` before rebuilding one; `git show
4edfd3f^:examples/<path>` still has every deleted file. A game at `examples/<name>/<name>.rs` needs
an explicit `[[example]]` block in `Cargo.toml` — Cargo finds only `*.rs` and `*/main.rs`.

The engine's headless facilities are **in `src/` and still work**:

- `ENGINE_CAPTURE=<frame>:<path.png>[,…]` — render headlessly at those frames, then exit.
- `ENGINE_INPUT=<script.ron>` — replay a scripted `(frame, action)` input list (`src/input_script.rs`).

⚠️ A capture run advances at a fixed `1/60` dt as fast as the CPU allows, so **real-time signals
cannot be photographed** — an audio meter reads `0.0` in a captured frame while sounding correctly,
and a networked game photographs `streaming 0 / 120` while the server is happily sending. Anything
arriving on a wall clock (audio, sockets, file watchers) needs a loop paced off `Instant`. That is
what `<NAME>_SELFTEST` acceptance tests exist for — every game carries one and `scripts/selftests.sh`
discovers them by grepping for the variable.

---

## Verification

`./scripts/verify.sh` is the gate — run it before calling anything done. It covers CI's `wasm` and
`docs` jobs in full and its `test` job bar one step (`cargo build --release`). It is **not** all of
CI; see the not-covered list below, and `grep -nE '^  [a-z_-]+:$|run:' .github/workflows/ci.yml`.

**Run it when the change can move what it measures** — `src/`, `tests/`, `examples/`, or a
**dependency** in `Cargo.toml`.

**Skip it — and say so in the report — when the change is confined to** prose (`docs/`, `plans/`,
`*.md`, comments), `.github/workflows/`, or a bare version bump. Run `cargo check` to keep
`Cargo.lock` honest and let **the PR's own CI run be the gate**: it re-runs everything anyway, on a
clean machine. A `ci.yml` change is the clearest case — the local gate *cannot* run CI, so it proves
nothing there. This is a bright line, not a judgement call: if a change touches both, it is a source
change. (The rule exists because three consecutive docs/CI-only PRs each paid ~6 min for a gate that
could not have failed.)

```sh
./scripts/verify.sh > /tmp/v.log 2>&1; echo "VERIFY_EXIT=$?"
SKELETON_MUTE=1 ./scripts/verify.sh …   # same run, silent speakers
```

**`SKELETON_MUTE=1` silences a run without weakening it.** The gate plays real sound otherwise —
`cargo test` fires 440/880 Hz tones from the audio unit tests. Mute is a final output gain applied
*after* every measurement: level taps are **pre-volume**, so a muted run measures the same numbers.
The device is still opened and exercised, so it is not the same as having no sound card. (The
example-side evidence for this — `audio_reactive` reporting rms `0.654` either way — went with the
examples; the mechanism is in `src/audio/playback.rs` and is unchanged.)

**Read the exit code from an unpiped command.** `docs/VERIFICATION.md` documents seven traps that
have each cost a session; the ones that recur:

- A trailing `| tail` reports the pipe's status and hides a red gate. zsh's array is `$pipestatus`, 1-indexed.
- A background run's completion notification reports the *trailing* command's status — write
  `echo $? > /tmp/v.exit` and **read the file**, after `rm -f`-ing it first (a stale file matches instantly).
- `;` does not short-circuit — branch on the captured code before committing.

⚠️ **The acceptance layer is back to full depth, browser included.** `selftests.sh` and
`build_wasm_examples.sh` both died in the 2026-08-19 deletion and are back — **35 checks across five
games**, 5 of 8 targets compiling for the web, and (2026-08-21) a **`wasm-smokes` CI job** loading
the engine in headless Chrome: Web Audio, the wasm WebSocket path, and two deliberate failure paths.
⚠️ Those smokes need Chrome so **CI gates them, not `verify.sh`**. Native audio still skips with no
device (all of CI). Else: `fmt`, `clippy`, the wasm build, `cargo test`, doctests, `cargo doc`.

What the gate does **not** cover, so get it yourself:

- **The `render` job** — `cargo test --test render` under `SKELETON_REQUIRE_GPU=1`. A machine with a
  GPU can run it, as can `ENGINE_CAPTURE` against a game. (Its three companion smokes —
  `headless_screenshot` / `lighting_cap` / `packaged_assets` — were examples and are gone.)
- **`cargo build --release`** — the only check that the `lto = "thin"` shipping profile links. Same
  for the `package` job's `cargo package --locked`.
- **macOS/Windows behaviour** — both have CI *build* jobs now, so the `cfg` branches compile; nothing
  ever runs them. Anything behavioural there still needs a local run.
- **Windowed playtest and gamepads** — had example-driven coverage, now none at all. Hot-reload got
  its back: `RPG_QUEST_SELFTEST` check 6 drives a real file watcher over wall clock. **Native**
  audio is the near miss — its selftest check skips wherever there is no device; ⚠️ Web Audio gates
  in CI since 2026-08-21 (`survivor_audio_web_smoke.sh`), the only place an audio claim is checked.
- **The `wasm-smokes` job** — `scripts/*_web_smoke.sh`; the `_web_` infix is the contract, not a
  count. ⚠️ Still no **pixel-level** check: a canvas readback needs a surface config that never ships.

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
Rebuilt as five games (2026-08-21); the debt is now the `src/` they do not reach, not the rule relaxed.

**A number pinned in prose is fixed by whoever changes it.** *(권고)* A stale baseline is worse
than none — it reads as an unexplained gain. v0.153.0 moved the lib-test count 1432 → 1443 in two
documents written hours earlier by the same session.

**A filed diagnosis is a hypothesis — so is the gate it names, and so is its size.** *(권고)*
Five times in v0.150.7–v0.152.5 a written cause was wrong and one measurement reversed it (#459,
#461, #462, #464, #473). The *gate* was wrong three times — twice on 2026-08-20 ("needs a GPU"
named a pure function) and on 2026-09-03 (#556) — each deferring a row unnoticed. The *size* was
wrong twice on 2026-08-25: 0.13 estimated against 0.004 measured, and decisive in the half it
never mentioned. Re-derive all three, **including your own claim from ten minutes ago**.

**Where things go.** Unit tests live inline (`mod tests`), beside the code they cover. `plans/` is
tracked — commit multi-session *plans*; a handoff is not one. `.claude/` is gitignored.

**Versioning.** Pre-1.0: MINOR covers any release including breaking changes, PATCH is a bugfix.
A release bump touches `Cargo.toml`, `Cargo.lock`, and `docs/CHANGELOG.md` together. Docs-only
changes take no version bump and no CHANGELOG entry.

**Commits.** `type(scope): summary (vX.Y.Z)` — e.g. `feat(audio): …`, `fix(app): …`,
`docs(patterns): …`. Bodies explain *why* and record real-device evidence CI cannot produce.
The `(#PR)` suffix is appended by the squash-merge **only if you let the subject default** —
`gh pr merge --subject` suppresses it and `main` is then unfixable without a force-push. To add to
a squash message pass `--body`/`--body-file` alone.

**Git.** Stage, commit, and push **only when explicitly asked**. Never `git reset --hard`,
`git checkout --`, or force-push without prior confirmation, and never revert someone else's
in-flight changes. *(권고)* **Push a large deletion's branch as soon as it is committed, even if
the PR waits** — pushing is not merging, so it breaks no merge-order agreement.

**Confirm before:** removing or renaming public API, changing dependencies/versions, large
refactors, deleting files. The verification scope is this repo only — do not build or modify
external projects unless asked.
