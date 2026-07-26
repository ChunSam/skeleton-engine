# Next: board gate, then either finish EW-011's deliberate cut or take a fresh breadth item

**Date:** 2026-07-26
**Status:** PLANNED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `board-ew-triple` seq `1`
**Context:** See `HANDOFF_board-ew-triple_three-board-requests_2026-07-26.md` for session data, the three requests as filed, gate history, and all measurements.

---

## Problem Statement

All three downstream board requests filed on 2026-07-26 shipped this session — EW-009 (text measurement, v0.132.0), EW-010 (DataTable union schema, v0.133.0) and EW-011 (scripted input + headless capture, v0.134.0) — leaving `main @ 5944ae1`, 1294 lib tests green, and **an empty board on both channels**. The next session therefore has no assigned work, and the user's standing rule is explicit: when the board is empty, **ASK for direction; do not self-pick**. What the next session *can* prepare for is the one thing this session deliberately did not deliver (windowed frame capture, cut because it needs `COPY_SRC` on the surface configuration — see Key Decisions in the handoff) and the standing breadth candidates. The risk to manage is the opposite of "no work": it is picking work the user did not choose.

## Key Findings

- **The board is empty and all three requests await the *game's* `Verified` mark**, not further engine work. A game-side problem report preempts everything. → drives Phase 1.
- **EW-011 shipped headless-only by explicit decision.** The surface is configured `usage: RENDER_ATTACHMENT` alone (`src/renderer/context.rs:160`); a windowed swapchain read-back needs `COPY_SRC`, which changes surface setup for every app and is disallowed on WebGL2. The board thread offers this as a follow-up **if the game asks**. → drives Phase 2.
- **`load_atlas_bytes` remains the last gap in the single-file/`include_bytes!` story.** `App::load_image_bytes(key, bytes)` exists (`src/app/assets.rs:143`) but `App::load_atlas(path, cols, rows)` (`:186`) is still path-only. → drives Phase 3.
- **The procgen family is complete (rooms/caves/mazes) and the `MapGenerator` trait's trigger has still not fired** — nothing swaps generators at runtime yet. Adding a fourth mode is optional breadth, not a gap. → informs the Phase 1 menu, not a phase of its own.
- **Serial landing works and is now a written recipe** (handoff → Evidence → "Reusable procedure"). Two open release PRs conflict on the four paperwork files, so one-at-a-time is not a preference, it is a constraint. → drives Dependencies & Order.
- **CI's `Test (native)` sat `pending 0s` for ~20 min on two of three PRs.** Budget ~25 min per PR and let auto-merge ride it out. → drives Risks.
- **The background-task notification reports the trailing `echo`'s exit code, not the gate's** — it masked a clippy failure and a rustdoc failure this session. → drives every phase's validation step.

## Anti-Goals (What NOT To Do)

- **Do not self-pick a feature because the board is empty.** This is the user's most-repeated standing rule and the parent chain recorded it too. Phase 1 ends in a question, not in code.
- **Do not add `COPY_SRC` to the surface speculatively.** It was cut on purpose. Only do Phase 2 if the game or the user asks — the request's actual pain (lock screen, macOS permissions) is already solved by headless.
- **Do not build the `MapGenerator` trait "now that there are three generators".** The trigger is a game or example wanting to choose a generator at runtime. It still has not happened; a premature abstraction on a fork-friendly skeleton is worse than three free functions.
- **Do not share the renderer's `FontSystem` with `TextMeasurer` via `Rc<RefCell<…>>`** unless font memory becomes a measured problem. Rejected this session for the double-borrow hazard; the reasoning is in the handoff's "What We Tried" step 6.
- **Do not open two release PRs at once.** Every release touches `Cargo.toml`, `Cargo.lock`, `docs/CHANGELOG.md` and the `CLAUDE.md` header line.
- **Do not trust a background task's "exit code 0".** Read the `.exit` file.

## Plan

### Phase 1: Board gate and direction (decision — blocking)

**Goal:** Establish whether there is assigned work, and if not, get an explicit direction from the user before writing any code.

**Why this approach:** The board preempts self-directed work by standing rule, and all three requests are currently in the game's court — a `Verified` mark or a problem report is the most likely next input. Every prior session in this repo that skipped straight to a feature had a board that was already read.

- Read `../dungeon-merchant/docs/engine-wishlist.md`. Check each of EW-009/010/011 for a `[Game]` reply: a `Verified` mark closes it (move to Done/archive per the board's own rules); a problem report becomes the session's work immediately.
- **Ignore the blank template row at `:202`** — it is the only remaining `Status: \`Proposed\`` grep hit and is not a request. Next free ID is **EW-012**.
- Read `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (expected `_None._`; the project is paused/deprecated — do not chase compatibility).
- Run the starting-state gate: `./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"` — read the code, do not pipe.
- Re-prove this session's three features with the four commands in the handoff's Quick Start (11 / 16 / 13 tests, plus both examples' headless runs).
- **If the board is empty: ask via `AskUserQuestion`, in Korean**, with this menu — (1) windowed frame capture, framing it as finishing EW-011's deliberate cut; (2) `load_atlas_bytes`; (3) a fourth procgen mode or the `MapGenerator` trait; (4) audio-reactive hooks or a second capstone game. Recommend (1) only if the game has asked for it; otherwise recommend (2) as the smallest coherent gap.

**Files:** none (read-only).
**Validates with:** `verify.sh` exit 0; three `cargo test --lib <module>` runs at 11 / 16 / 13; both examples exit 0.
**Rollback:** n/a.

### Phase 2: Windowed frame capture — only if chosen

**Goal:** Let `ENGINE_CAPTURE` photograph the real on-screen frame of a windowed run, closing the half of EW-011 that was deliberately cut.

**Why this approach:** Headless capture already covers the request's stated pain. This adds the case where a human wants to watch a scripted run *and* keep the frames — and it is the only outstanding piece of the three shipped requests. It is gated on demand precisely because it touches every app's surface configuration.

- In `GpuContext::new` (`src/renderer/context.rs`, the `SurfaceConfiguration` at `:160`), request `RENDER_ATTACHMENT | COPY_SRC` **only when `caps.usages` advertises `COPY_SRC`**, else keep `RENDER_ATTACHMENT` alone. Mirror the existing capability-fallback style of `resolve_render_target_format` — warn once when the capability is missing so a failed capture is explainable, never silent.
- Add a `pub(crate) fn read_surface_rgba(&self, texture: &wgpu::Texture) -> Option<(u32, u32, Vec<u8>)>` next to `read_headless_rgba` (`:363`), reusing its 256-byte row-alignment maths (`COPY_BYTES_PER_ROW_ALIGNMENT`). Return `None` when `COPY_SRC` was not granted.
- Hook the copy in `src/app/render/frame.rs` **after the last pass and before `frame.present()`** (`:672`). The docked-editor path has its own present at `src/app/render/docked.rs:64` — decide explicitly whether docked capture is in scope and say so in the doc comment either way.
- Introduce a small `pub(crate)` capture-request resource (frame number + path list) so the windowed loop knows which frames to grab; feed it from the same `parse_capture_plan` the headless path uses — **one parser, two consumers**, mirroring this session's `shape_text`/`font_blobs` pattern.
- Decide and document the divert rule: today `ENGINE_CAPTURE` always diverts to headless. Either add a second variable (e.g. `ENGINE_CAPTURE_WINDOWED=1`) or make windowed the behaviour when a window is already open. Do **not** silently change what `ENGINE_CAPTURE` means for existing users.
- Extend `examples/scripted_capture.rs` documentation with the windowed invocation; the example itself should need no code change.
- Tests: unit-test the new plan/resource plumbing (no GPU). The read-back itself is GPU-bound — add a `tests/render.rs` case if lavapipe can present, otherwise verify locally and say so in the PR (CI cannot verify a swapchain read-back).

**Files:** `src/renderer/context.rs` (surface usage + read-back), `src/app/render/frame.rs` (copy before present), `src/input_script.rs` (shared plan → windowed consumer), `examples/scripted_capture.rs` (docs), `CLAUDE.md` (extend the scripted-input row).
**Validates with:** `verify.sh` exit 0; a windowed `ENGINE_CAPTURE` run writes a PNG matching what is on screen; the existing headless path byte-identical (re-run `HEADLESS_SHOT=/tmp/sc cargo run --example scripted_capture`, expect the same three screens).
**Rollback:** revert the `SurfaceConfiguration` usage change first — it is the only edit that touches every app. Everything else is additive.

### Phase 3: `load_atlas_bytes` — byte-source atlas parity

**Goal:** Let a game build a `TextureAtlas` from `include_bytes!` data, completing the single-file/jam-build story that `load_image_bytes` started.

**Why this approach:** It is the smallest remaining gap with a written rationale (carried from the `asset-root-windows` chain), it is purely additive, and it has an exact template: `App::load_image_bytes` (`src/app/assets.rs:143`) already solved the identity problem this must not re-break.

- Add `App::load_atlas_bytes(key: impl Into<String>, bytes: &[u8], cols: u32, rows: u32) -> Handle<TextureAtlas>` beside `load_atlas` (`src/app/assets.rs:186`), delegating to a matching `AssetServer` method.
- **Reuse `load_image_bytes`'s identity invariant exactly**: the `key` is VERBATIM (not `asset_key`/canonicalised), so `key` == cache key == `Handle::path()` == render key. This is the invariant that dodges the 2026-05-29 white-sprite bug; read that CLAUDE.md row before writing a line.
- Ride the per-frame `upload_asset_server_images_to_gpu` seam like `load_image_bytes` does — no `pending_textures` push, no fs read.
- Cross-platform including wasm (the `image` crate is un-gated), so this also works in a single-file wasm build.
- Corrupt bytes must report through `asset_failures()` like every other loader (the EW-007 rule).
- Example: extend `examples/embedded_image.rs` with an atlas variant, or add a small `embedded_atlas` example that renders an `AtlasSprite` from embedded bytes. Per VISION the example is the acceptance test.
- Tests: cache-key identity (handle path == render key), a corrupt-bytes failure surfacing in `asset_failures()`, and grid maths matching the path-loaded atlas.

**Files:** `src/app/assets.rs`, `src/asset.rs` (AssetServer method), `src/atlas.rs` if the ctor needs a byte path, `examples/embedded_atlas.rs` (new) or `examples/embedded_image.rs`, `CLAUDE.md` (atlas + asset rows).
**Validates with:** `verify.sh` exit 0; the new example renders the embedded atlas natively **and** builds for wasm (`cargo build --example <name> --target wasm32-unknown-unknown` — the verify wasm gate covers lib+bins only, never examples).
**Rollback:** purely additive — delete the new method + example.

### Phase 4: Game-report triage — only if the board comes back with a problem

**Goal:** Turn a `[Game]` problem report on EW-009/010/011 into a fix without re-litigating the design.

**Why this approach:** All three are in the game's court, so this is the single most likely non-empty-board outcome, and each has a known soft spot recorded in the handoff's Open Questions.

- **EW-009 report** → most likely "measurement is off for X". First check whether the game passes the same `FontData`/`ExtraFonts` the renderer has (measurement is exact only against the same stack) and whether it is measuring a rich string with plain `measure`. Only then suspect `shape_text`.
- **EW-010 report** → most likely a column type surprise from `add_row`, or a RON file whose header comments still claim the row-0 rule. Point them at the changed `default_value_for_col` semantics.
- **EW-011 report** → most likely "the click did nothing". First suspect the coordinate: cell edges are **exclusive**, and this session's own script missed by landing exactly on one (see the grid table in the handoff's Evidence). Second suspect an unknown key name — but that is a load error, so it would have been reported.
- Reply in the board thread append-only, dated, `[Engine]`.

**Files:** whichever module the report implicates.
**Validates with:** a regression test reproducing the reported behaviour, added before the fix.
**Rollback:** per-fix.

## Dependencies & Order

- **Phase 1 gates everything.** Phases 2, 3 and 4 are mutually exclusive alternatives selected by its outcome — do not start any of them before the board is read and (if empty) the user has answered.
- **Phase 4 preempts 2 and 3** if a game report exists: a shipped-but-broken request outranks new breadth.
- **Phases 2 and 3 are independent** and touch disjoint files, so either order works — but land them **serially** (one release PR at a time; see the handoff's reusable procedure).
- The handoff/plan docs PR for this session must already be on `main` before the next feature branch is cut, or that branch will go `BEHIND`.

## Risks & Mitigations

- **Self-picking work when the board is empty.** Likelihood: moderate (an empty board reads like a free choice). Mitigation: Phase 1 ends in `AskUserQuestion`; the anti-goal is stated first in this plan and in the handoff.
- **Phase 2's surface change regresses an unrelated app.** Likelihood: low but blast radius is every app on every backend. Mitigation: gate on `caps.usages`, keep the fallback path byte-identical, and revert that single edit first if anything misbehaves.
- **CI `Test (native)` runner backlog.** Likelihood: high — hit two of three PRs this session for ~20 min each. Mitigation: arm `--auto` and walk away; budget ~25 min; never force a re-trigger.
- **A gate failure hidden by a background task's exit code.** Likelihood: high (hit twice). Mitigation: always `./scripts/verify.sh > log 2>&1; echo $? > exit` and read the file; never index `${PIPESTATUS[0]}` in zsh (it is `$pipestatus`, 1-indexed).
- **CLAUDE.md is ~208 lines, over the 200-line soft cap.** Likelihood: certain to be hit again — both Phase 2 and Phase 3 add module-map detail. Mitigation: extend an existing row rather than adding one, or move detail into a `docs/*.md` with a one-line pointer.
- **Phase 3 re-breaking the texture-cache identity invariant.** Likelihood: low if `load_image_bytes` is followed exactly; the failure mode (white sprites) is silent at compile time. Mitigation: the identity test is written first.

## Success Criteria

- **Minimum viable:** the board is read, both channels are correctly triaged, and either a game report is being worked or the user has chosen a direction — with no code written before that choice.
- Starting state re-proved: `verify.sh` exit 0, `text_measure` 11 / `data_table` 16 / `input_script` 13 tests passing, both examples exiting 0 (baseline: 1294 lib tests total).
- **If Phase 2 runs:** a windowed `cargo run` with `ENGINE_CAPTURE` writes a PNG matching the on-screen frame, the headless path produces byte-identical output to today's three screens, and an adapter without `COPY_SRC` degrades with a warning rather than a panic.
- **If Phase 3 runs:** an `AtlasSprite` renders from `include_bytes!` data with no fs access, natively and on wasm; `Handle::path()` equals the render key; corrupt bytes appear in `asset_failures()`.
- **If Phase 4 runs:** the reported behaviour has a failing regression test before the fix, and the board thread carries a dated `[Engine]` reply.
- Whatever ships lands as one squash-merged PR with green CI, the board is updated, and memory advances one seq.

## Quick Start

```bash
# Restore full context
cat plans/handoffs/HANDOFF_board-ew-triple_three-board-requests_2026-07-26.md

# 1. THE BOARD — both channels, before anything else
#    ../dungeon-merchant/docs/engine-wishlist.md       (EW-009/010/011 = Shipped, awaiting game Verify;
#                                                       next free EW-012; the `Proposed` hit at :202 is the template)
#    ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md  (expected _None._)

# 2. Verify starting state — read the exit code, never pipe it
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# 3. Re-prove what shipped
cargo test --lib text_measure && cargo test --lib data_table && cargo test --lib input_script
HEADLESS_SHOT=/tmp/tm.png cargo run --example text_measure
HEADLESS_SHOT=/tmp/sc     cargo run --example scripted_capture

# 4. Key source files (read only what the chosen phase needs)
#   Phase 2: src/renderer/context.rs (:160 surface usage, :363 read_headless_rgba),
#            src/app/render/frame.rs (:672 present), src/input_script.rs (parse_capture_plan)
#   Phase 3: src/app/assets.rs (:143 load_image_bytes, :186 load_atlas), src/atlas.rs

# 5. FIRST CONCRETE ACTION
#    Read ../dungeon-merchant/docs/engine-wishlist.md. If EW-009/010/011 carry no new [Game] reply
#    and nothing new is filed, ASK the user for direction in Korean with the four-option menu above.
#    Do NOT self-pick.
```
