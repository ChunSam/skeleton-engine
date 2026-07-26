# Shipped: all three downstream board requests — text measurement, DataTable union schema, scripted input + headless capture (v0.132.0 → v0.134.0)

**Date:** 2026-07-26
**Status:** COMPLETED (PRs #372, #373, #374 all merged; `main @ 5944ae1`, v0.134.0, clean + green)
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `board-ew-triple` seq `1`
**Parent:** none — first in chain
**Prior chain:** none — first in chain

---

## Related Handoffs

- `plans/handoffs/HANDOFF_procgen-modes_maze-generation_2026-07-20.md` — the document this session **onboarded from** (the user's first message was "마지막 핸드오프 확인하고 다음 작업 알려줘"). It is a **separate work stream** (procgen generators), not this chain's parent. Its board-gate instruction is what routed this session.
- `plans/handoffs/HANDOFF_procgen-modes_cellular-caves_2026-07-20.md` — procgen seq 1, reference only.

## Since Last Handoff

Framed against the procgen-modes seq-2 handoff above (onboarding source, not chain parent):

- **Its central prediction was falsified.** It said, in bold, "The next session will again find both channels empty. It MUST ask, not self-pick." The board was **not** empty — `dungeon-merchant` had filed **EW-009, EW-010 and EW-011 on 2026-07-26**, the same day. Its *rule* ("board gate FIRST; a newly-filed request preempts everything below") is what made the session correct anyway: the board was read before anything else, and the procgen "optional breadth" menu was never opened.
- **The procgen family stays complete and untouched** (rooms / caves / mazes). No fourth generator, no `MapGenerator` trait — that open question is still open and still not triggered.
- **Its `cargo fmt` reflow warning held.** Run `cargo fmt` before `verify.sh` — done proactively all session, and no fmt gate ever went red.
- **Its "read the gate's exit non-piped" warning held, and was violated twice anyway** — see Risks. Both violations came from the *background-task notification*, which reports the trailing `echo`'s status rather than `verify.sh`'s.
- **The strict-checks `BEHIND` risk it flagged did not materialize**, because PRs were landed strictly serially (each branch cut from freshly-pulled `main`). One rebase was needed instead, and it was clean.
- **The GitHub runner-backlog risk it flagged DID recur** — `Test (native)` sat `pending 0s` for roughly 20 minutes on both #372 and #374 while the other five checks went green in 1–3 minutes. Auto-merge rode it out exactly as the parent predicted.

## Reference Documents

- `CLAUDE.md` — project conventions, module map (three rows added this session), verify-gate list.
- `docs/CHANGELOG.md` — 0.132.0 / 0.133.0 / 0.134.0 entries are the migration notes.
- `docs/VISION.md` — the "a feature is not done until a playable example exercises it" rule that shaped both examples.
- `../dungeon-merchant/docs/engine-wishlist.md` — the shared board; the source of all three requests.

---

## The Goal

`dungeon-merchant` (the downstream game) files engine requests on a shared board. On 2026-07-26 it filed three at once — the first non-empty board in several sessions. The goal was to close them: **EW-009** (P1, measure shaped text so UI panels stop being sized by a `chars × px` guess), **EW-010** (P2, derive a `DataTable`'s column schema from all rows so optional columns stop being silently discarded), and **EW-011** (P2, headless frame capture + scripted input playback so GUI verification stops needing a live desktop and macOS Accessibility/Screen-Recording permissions). Each had to ship as its own release with its own playable example per the VISION loop, be marked `Shipped` on the board, and leave the board in the game's court for verification.

## The Three Requests As Filed

The board entries are the specification; each was closed against its own stated acceptance criteria. Reproduced here so the next session does not have to re-read the game repo.

### EW-009 · Text measurement API — P1

> **Need:** `App::measure_text(text, font_size) -> (w, h)` (or a `TextQueue`/free-fn equivalent), honoring the same font stack + script fallback the renderer uses (`FontData` + `ExtraFonts`). Single-line is the need; multi-line/bounds-aware is bonus.
> **Why:** the images-v2 hover tooltip sizes its panel as `max(name_chars, value_chars) × 15.0 + 22.0`. Korean glyphs ≈ 15px at size 15 but Latin/digits ("45g", "200g") are roughly half that — mixed strings get a loose panel, and a long-name + small-value pair can clip.
> **Workaround:** chars×15 heuristic + generous padding; "can't be made correct, only padded".

| Acceptance criterion | How it was met |
|---|---|
| `measure_text("사슬갑옷", 15.0).0` within ~1px of the rendered width | **Structural**: renderer + measurer share `shape_text`. Demonstrated visually — the example's tick lands on the glyph end. |
| Mixed 한글+Latin measures correctly through the fallback stack | Example loads DejaVu (`FontData`) + Noto Sans KR subset (`ExtraFonts`); headless check asserts Hangul ≥ 1.3× same-count Latin (actual: 1.49×) |
| Callable from a `System::run` without borrow conflicts against the render pass | `measure_text(world, …)` / `text_measurer(world)` borrow only the `World`; `TextMeasurer` owns its own `FontSystem` |
| Multi-line / bounds-aware (bonus) | `measure_wrapped` — returns the longest wrapped line, not the bound |

### EW-010 · DataTable union schema — P2

> **Need:** `columns` = union over ALL rows, not row 0 alone.
> **Why:** every optional column (`durability`, `shop`, `vuln`, `named`, `depth_bias`, and this week `icon` ×27 + `emblem` ×4) required a "MUST be on every row incl. row 0" discipline, enforced by header comments in 4+ RON files, violated silently — "the feature just doesn't apply to some rows", far from the cause.

| Acceptance criterion | How it was met |
|---|---|
| Row 3 alone carries `icon:` → `icon` in `columns`, `Unit` elsewhere | `schema_is_the_union_of_all_rows` |
| A row-0-complete table parses byte-identically | `row_zero_complete_table_parses_unchanged` (asserts no cell is `Unit`) |
| The F2 editor shows the union columns | Automatic — `data_table_panel.rs` is column-driven; verified by reading, no code change |
| The extra-column `warn!` retires (or demotes to debug) | **Removed** — unreachable under a union schema, so a `debug!` would imply a condition that cannot occur |

### EW-011 · Headless GUI verification — P2

> **Need:** (a) capture rendered frame N to a PNG — env-driven (e.g. `ENGINE_CAPTURE=120:/tmp/shot.png`) or an `App` API — at the design resolution; and ideally (b) scripted input playback: a `(frame, key/mouse event)` list injected into the normal input path, windowed or headless.
> **Why:** verifying the images-v2 art needed a live *unlocked* desktop, macOS Accessibility + Screen-Recording permissions, osascript (where **typed digits never reach winit — only `key code` works**), and a Swift `CGEvent` one-liner for mouse moves; one whole verification attempt died on the lock screen.
> **Hint in the request:** "If the `HEADLESS_SHOT` machinery … already covers part of (a), documenting/exposing it for games may be most of this request."

| Acceptance criterion | How it was met |
|---|---|
| Capture writes a PNG matching the on-screen render at design resolution | `ENGINE_CAPTURE=<frame>:<path>[,…]` → `capture_frames_headless`; renders through the normal path, so `DesignResolution` letterboxing applies as on screen |
| Scripted `(frame, key)` triggers the same phase transitions real input does | Injected into `InputState` at step 0 of `update`; example asserts the script reached `Detail`; `maze_generation` driven with zero changes |
| Both run under plain `cargo run` with no OS automation permissions | No window, no display, no permissions — capture diverts `App::run` |
| (implicit) the `HEADLESS_SHOT` machinery exposed for games | `capture_frames_headless` generalises it to N frames and the env var makes it zero-code |

**Deliberately not delivered:** windowed capture (the request's "windowed or headless" applies to *driving*, and headless is what removes the permissions problem). Stated in the PR, CHANGELOG and board thread with an offer to follow up.

## Where We Are

- **`main @ 5944ae1`, package v0.134.0, CLAUDE.md header v1.6.228, clean tree, all gates green.** Local `main` == `origin/main`.
- **All three PRs merged**, serially, each with its own MINOR/PATCH-appropriate bump: **#372 `7c6af6d` v0.132.0**, **#373 `ec7e18d` v0.133.0**, **#374 `5944ae1` v0.134.0**.
- **Lib tests 1267 → 1294 (+27)**: 11 (`text_measure`) + 3 (`data_table`) + 13 (`input_script`). Every one passed on its first run except the example-level checks discussed below.
- **The board is empty again.** All three entries read `Status: Shipped (vX.Y.Z)` and sit in the **game's** court for `Verified`. Next free ID is **EW-012**. `rust-survivors` is still `_None._`. The one remaining `Status: \`Proposed\`` grep hit in the board file is the blank `EW-NNN` template row, not a request — confirmed by line number (`:202`).
- **Two board PRs landed in the game repo**: `dungeon-merchant` **#45** (`1702d27`, EW-009 + EW-010) and **#46** (`51ed4c6`, EW-011).
- **EW-009 — `src/text_measure.rs` (367 lines).** `TextMeasurer` with `measure` / `measure_rich` / `measure_wrapped` / `line_height`; world-level `measure_text` / `measure_text_wrapped` / `text_measurer(world)`; `App::measure_text{,_wrapped}`. Empty text measures `Vec2::ZERO`.
- **The renderer and the measurer now share one shaping path.** `shape_text(&mut FontSystem, &ShapeSpec)` was extracted from the buffer-construction closure inside `TextRenderer::render`; the renderer passes exactly the arguments it used inline before, so rendering is byte-identical. `LINE_HEIGHT_FACTOR = 1.2` replaced two hardcoded `1.2` literals (the `Metrics` and the `TextAnchor::Center` offset).
- **They also share one font stack.** `font_blobs(world)` (in `text_measure.rs`, `pub(crate)`) now owns the `FontData` + `ExtraFonts` + wasm-`DEFAULT_FONT`-fallback collection that used to be inline in `App::init_gpu_renderers` (`src/app/window.rs`).
- **EW-010 — `src/data_table.rs`.** `DataTable::parse` collects `columns` from the **union** of every row's keys (still alphabetical). The extra-column `log::warn!` is **removed as unreachable**, not demoted. `add_row` now types a cell from the first row carrying a non-`Unit` value, not from row 0.
- **The editor needed no change for EW-010** — `src/app/editor/ui/data_table_panel.rs:105` clones `table.columns` and `:122` fills each cell with a `Unit` fallback, so union columns appear automatically. Verified by reading, not assumed.
- **EW-011 — `src/input_script.rs` (867 lines).** `InputScript` / `InputAction` / `ScriptedInput` / `InputScriptError`, `key_from_name` (100 keys) / `key_names` / `mouse_button_from_name`, `parse_capture_plan` / `capture_plan_from_env`, and `App::set_input_script` / `apply_input_script_env` / `apply_input_script_frame`.
- **`App::capture_frames_headless(&[(frame, path)])`** added to `src/app/headless.rs` (+78 lines), writing a PNG at each listed frame in one run; `save_rgba_png` factored out and shared with `save_screenshot_headless`.
- **Injection point: step 0 of `App::update`** (`src/app/schedule.rs`), before `run_systems` and after the previous frame's `input.flush()` (which lives in `post_systems` at `:516`). That ordering is what makes a scripted press read as `just_pressed` inside its own frame.
- **Env entry points work with zero game code**: `ENGINE_INPUT=<script.ron>` and `ENGINE_CAPTURE=<frame>:<path>[,…]`, both read at the top of `App::run` (native) before `EventLoop::new`. Capture **diverts** the run — headless, then `return`; it never also opens a window.
- **Genericity proved on an unrelated example**: a throwaway script pressing `KeyB` (frame 5) and `KeyR` (frame 20) drove `maze_generation` — zero changes to that example — and the captured HUD read `seed 2 · braided`.
- **Two playable examples shipped**: `examples/text_measure.rs` (448 lines) and `examples/scripted_capture.rs` (346) + `examples/scripted_capture.ron` (27).
- **Memory updated to seq 193** (`engine-current-state.md` tip line + a new BOARD paragraph), and the `MEMORY.md` index hook refreshed to `main @ 5944ae1` / v0.134.0.

## What We Tried (Chronological)

### Chunk 1 — Onboarding and the board gate (early)

1. **Read `docs/HANDOFF.md` tail, then head — wrong file.** Its tail held 2026-05-31 phase records and its head a 2026-05-24 header; it is the long-lived architecture history, not the per-session handoff. The real latest handoff was `plans/handoffs/HANDOFF_procgen-modes_maze-generation_2026-07-20.md`, found via `git show --stat 0e43b8c`. Cost: two wasted reads. **Next session: go straight to `ls -t plans/handoffs/`.**
2. **Ran the board gate on both channels in parallel with the state checks.** `../dungeon-merchant/docs/engine-wishlist.md` → three `Proposed` entries dated 2026-07-26. `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` → `_None._`. This is the first non-empty board in several sessions, so the parent handoff's "ASK for direction" path was *not* taken.
3. **Kicked `./scripts/verify.sh` off in the background before touching anything** → `/tmp/verify_start.exit` = **0**. Clean starting state confirmed.
4. **Reported to the user in Korean with a recommended order** (EW-009 first as the only P1, then EW-010, then EW-011 as a separate session because of its size) and asked which to take. The user replied **"EW-009부터 ew-010까지 진행"** — two, not three. EW-011 was only started later, after being proposed again and approved with **"진행해"**.

### Chunk 2 — EW-009 design and implementation (early-mid)

5. **Read the text stack before designing.** `src/renderer/text/renderer.rs`: `shaped_center_x` (`:531`), `layout_buffer_width` (`:555`), `layout_buffer_height` (`:571`), `build_font_system` (`:76`, then `pub(super)`), and the shaping closure inside `render` using `Metrics::new(size, size * 1.2)` + `Shaping::Advanced` + `Attrs::new().family(Family::SansSerif)`. `glyphon = "0.11"`, un-gated → works on wasm.
6. **Rejected sharing the renderer's `FontSystem` via `Rc<RefCell<…>>`.** It would guarantee identical state and avoid a duplicate system-font scan, and World resources are only `T: 'static` (no `Send`/`Sync` bound), so it would compile. Rejected because it means `borrow_mut()` calls threaded through every `TextRenderer` method and a double-borrow panic hazard in a fork-friendly skeleton. Took a separate `FontSystem` instead.
7. **Rejected an `App`-only `measure_text`.** EW-009's acceptance explicitly required "callable from a `System::run` without borrow conflicts against the render pass", and the renderer lives on `App`, not in the `World`. Hence the resource + world-level free functions, with the `App` methods as a thin convenience twin.
8. **Chose to make exactness structural rather than tuned.** Instead of matching the renderer's shaping parameters by hand (and hoping they never drift), extracted `shape_text(font_system, &ShapeSpec)` and had both callers go through it, plus `font_blobs(world)` for the font stack. This is the single most important decision in the PR — see Key Decisions.
9. **Hit the `DEFAULT_FONT` cfg trap in tests.** `crate::renderer::DEFAULT_FONT` is `#[cfg(target_arch = "wasm32")]` (deliberately, to keep DejaVu out of native binaries), so the unit tests could not use it. Followed the existing pattern in `src/renderer/text/tests.rs:235` and `include_bytes!`-ed `assets/fonts/DejaVuSans.ttf` directly — which also makes widths machine-independent (a bare system-font scan would make them depend on what is installed).
10. **11 unit tests passed on the first run.** Included the deliberately adversarial `same_char_count_can_differ_in_width` (`"iiii"` vs `"WWWW"`) and `rich_markup_is_not_counted` (which also asserts that measuring a rich string as *plain* over-reports by >2×, documenting why `measure_rich` exists).

### Chunk 2b — The EW-009 example, and fixing it twice

11. **First headless run of `examples/text_measure` passed** (exit 0): `사슬갑옷=51.9px  200g=34.9px  worst heuristic error=127%  wrapped=223×54`.
12. **Eyeballed the captured PNG — the acceptance criterion was visible.** The green ticks marking `measure_text`'s reported width land exactly on the end of each drawn string, for Hangul and Latin alike. That *is* EW-009's "within ~1px of the rendered width", demonstrated rather than asserted.
13. **Found a dead branch: the "clips" case was unreachable.** The example's heuristic was `chars × size + 22`, which scales with the font size, so it can never under-measure — the red panel colour was decorative. Fixed by making the heuristic use a fixed `HEURISTIC_TUNED_SIZE = 15.0` (which is what the *real* game does — a constant baked in at one size), so raising the size makes it clip. This also made the ↑↓ control meaningful instead of cosmetic.
14. **Tried asserting `ROWS.iter().all(…)` clip at size 28 → FAILED.** `"Elixir of Warding"` is 17 chars → heuristic 277 px, measured ≈ 250 px at size 28: still loose. Reverted to `any()`, which is the honest claim, and reworded the failure message and the `MAX_FONT_SIZE` doc comment to match.
15. **Mis-read a gate exit via `$pipestatus`.** Used `echo "EXIT=${pipestatus[2]}"` on `cargo fmt && … cargo run … | tail`; index 2 is `tail`, so it printed `EXIT=0` while the example had actually `exit(1)`-ed. The FAIL line in the output is what gave it away. Switched to `cmd > log 2>&1; echo $?`.

### Chunk 2c — Three red verify runs, three different causes

16. **verify #1 → exit 101: clippy `explicit_auto_deref`.** Four hits in `examples/text_measure.rs:221,223` — `push_row(tq, *name, *value, …)` where auto-deref suffices. Fixed with a `perl -pi` rewrite; clippy then clean.
17. **verify #2 → exit 101: self-inflicted cross-contamination.** `cannot find type HashSet` ×4 — because I had begun editing `src/data_table.rs` (EW-010) *while* the EW-009 verify was running, and the `HashSet` import landed after the compile started. Diagnosed from the log rather than re-run blindly. **This is why the two changes were then physically separated** (see step 20).
18. **verify #3 → exit 101: rustdoc `redundant_explicit_links`.** `src/text_measure.rs:11` wrote ``[`ExtraFonts`](crate::resources::ExtraFonts)`` where the label already resolves (`ExtraFonts` is imported in the module). Fixed by dropping the explicit target; `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` then exit 0.
19. **verify #4 → exit 0**, 148 `test result: ok` groups. Then `/ship` to 0.132.0 and a **second** full verify after the bump → exit 0.

### Chunk 3 — EW-010, and the branch-isolation problem

20. **Stashed `src/data_table.rs` to isolate EW-009.** `git stash push -m … src/data_table.rs` left a tree containing only EW-009's eight files. The two changes touch **disjoint source files**, which is what made a clean split possible at all.
21. **Read `DataTable::parse` (`:100–126`) in full before changing it.** Row 0's pairs built `columns`; every later row was matched against that schema, with a `log::warn!` for any extra key and a `ron::Value::Unit` fill for any missing one.
22. **Implemented the union in one pass**: extract all rows' pairs once (`Result<Vec<_>, _>` collect), build `columns` with a `HashSet<&str>` seen-set borrowed from `all_pairs`, `sort()`, then build rows with the unchanged `Unit` fill. Removed the `warn!` entirely — with a union schema the condition is unreachable, so demoting it to `debug!` would have been dishonest.
23. **Found a second bug the request did not mention.** `default_value_for_col` read row 0 only. Under a union schema an optional column is `Unit` at row 0, so `add_row` in the editor would seed a late-appearing column with nothing. Changed it to scan for the first row holding a non-`Unit` value. Covered by `add_row_types_a_late_column_from_the_row_that_has_it`.
24. **Checked the editor rather than assuming.** `src/app/editor/ui/data_table_panel.rs:105` clones `table.columns`; `:122` builds each row's values by looking each column up with a `Unit` fallback. Column-driven, so EW-010's "F2 editor shows the union columns" criterion needed no code.
25. **Branched EW-010 off `main`, not off the EW-009 branch.** The changes are independent; stacking them would have meant a guaranteed `BEHIND` + a `Cargo.toml`/`CHANGELOG` conflict on the version bump.
26. **Waited for #372 to merge before doing EW-010's version paperwork.** Deliberate: `Cargo.toml`, `Cargo.lock`, `docs/CHANGELOG.md` and the `CLAUDE.md` header line are touched by *every* release, so bumping before the first merge would have made the rebase conflict. After #372 merged, `git rebase main` applied cleanly (the CLAUDE.md hunks were the header line vs. the DataTable row — different regions), *then* the bump to 0.133.0.

### Chunk 4 — EW-011, and the scope decision

27. **Read `src/app/headless.rs` (262 lines) and `GpuContext` before designing.** `screenshot_headless` builds a headless GPU, inserts `ViewportSize`, runs N `update`+`render` pairs, and calls `read_headless_rgba` (`src/renderer/context.rs:363`).
28. **Checked whether a *windowed* capture was cheap — it is not.** The surface is configured with `usage: wgpu::TextureUsages::RENDER_ATTACHMENT` alone (`src/renderer/context.rs:160`). Reading back the swapchain needs `COPY_SRC`, which means changing the surface configuration for **every** app on every backend (and it is not permitted on WebGL2 at all). **Scoped out deliberately** — see Key Decisions.
29. **Found the injection point by locating the flush.** `input.flush()` is called in `post_systems` (`src/app/schedule.rs:516`), i.e. at the *end* of the frame. So injection had to be at the *start* of `update` — after the flush, before `run_systems` — for `just_pressed` to survive into the systems. Added as "step 0" with a comment saying exactly that.
30. **Found the injection API already existed at the right visibility.** `press` / `release` / `press_mouse` / `release_mouse` / `add_scroll` are `pub(crate)` (`src/input/state.rs:110–174`) and `set_cursor` is already `pub` (widened earlier for the Tooltip work). So the playback needed **no widening of the public input API**.
31. **Rejected enabling winit's `serde` feature.** `KeyCode` has no serde support in the current build. Enabling it would add a feature to a large dependency *and* force RON authors to write exact enum syntax. Wrote `key_from_name` instead — 100 arms, generated with a Python script to avoid typos (26 letters + 10 digits + 12 F-keys + 36 named + 16 numpad) — which also gives friendlier script text.
32. **Made an unknown key name an error, not a dropped event.** Given that the requesting game's whole complaint is about invisible failures (`osascript` digits never reaching winit), a silently-ignored `KeyPress("Nope")` would look exactly like a broken feature. `an_unknown_key_name_is_an_error` asserts the message names the typo.
33. **13 unit tests passed on the first run**, including the frame semantics (`just_pressed` in its own frame, `just_released` in the next), the click position feeding a hit-test, and three `ENGINE_CAPTURE` parser cases.

### Chunk 4b — The example failed twice, and both failures were worth having

34. **First headless run FAILED (exit 1): `after the script the app is on Shop, expected Detail`.** The `Digit2` press had worked (Menu → Shop), but the click selected nothing. Cause: my script clicked `(556, 250)`. Columns start at x = 60 / 316 / 572 with `CELL_W = 240`, `GAP = 16`; `316 + 240 = 556`, and the hit-test uses an **exclusive** right edge, so 556 is in the gap — and `y = 250` is past `130 + 110 = 240`, in the row gap too. Fixed to `(690, 185)` (middle of cell 2) and added a comment to the `.ron` warning that edges are exclusive.
35. **Second run passed — and the screenshot exposed a real z-order bug in the example.** The detail popup drew, but the shop's item labels (`Iron Sword`, `45g`, `Oak Shield`, …) read straight over it. Cause: `DrawText` without a `z` renders in the final on-top pass (correct, documented engine behaviour — right for a HUD), so a popup can never cover it. Fixed by layering: labels `z 0.3`, scrim `0.5`, popup `0.6`, popup text `0.7`. **This is precisely the defect class EW-011 says the blind boot smoke misses, caught by the tool's own first output** — recorded in a code comment and in the PR body.
36. **Verified the zero-code env path twice.** First on `scripted_capture` itself through `App::run` (3 PNGs, `ENGINE_CAPTURE wrote …` ×3). Then — the stronger test — on **`maze_generation`, an unrelated example with no knowledge of any of this**: a throwaway `/tmp/maze_script.ron` pressing `KeyB` at frame 5 and `KeyR` at frame 20 produced a capture whose HUD reads `seed 2 · braided`, proving both keys reached the app.

### Chunk 4c — Landing and bookkeeping

37. **`/ship` → 0.134.0**, full verify before and after the bump (both exit 0), commit, push, PR #374, `gh pr merge --auto --squash`.
38. **Board updated in two PRs, not one.** `dungeon-merchant` #45 covered EW-009 + EW-010 (written after both merged); #46 covered EW-011. Each thread comment is `[Engine] 2026-07-26` and append-only, per the board's own rules.
39. **Memory bumped twice** — seq 191/192 after the first two merges, seq 193 after the third — plus a new BOARD paragraph in `engine-current-state.md` and a refreshed `MEMORY.md` index hook.

## Key Decisions

- **Make measurement exactness structural, not tuned (EW-009).** Rather than duplicating the renderer's shaping parameters in the measurer and hoping they stay in sync, extract `shape_text(font_system, &ShapeSpec)` and route both callers through it, and collect the font stack once in `font_blobs(world)`. A future change to `Metrics`, `Shaping`, attrs or wrap mode now moves both paths together. The alternative — matching parameters by hand — would have satisfied EW-009's "within ~1px" criterion *today* and silently broken later.
- **A separate `FontSystem` for the measurer, not a shared `Rc<RefCell<FontSystem>>`.** Costs a duplicate system-font scan and some font-DB memory; buys no `borrow_mut()` plumbing through the renderer and no double-borrow panic hazard. Mitigated by making construction lazy (a game that never measures never pays) and `register_persistent` (a scene change never re-pays).
- **Measurements are in `DrawText`'s logical pixels**, pre-`DisplayScaleFactor`/`Letterbox`, so a measured width is directly a `DrawRect` width. The renderer shapes at the display-scaled size for crispness; that is a HiDPI implementation detail, documented as such rather than exposed.
- **`measure_rich` exists to close a footgun.** Measuring a markup string with plain `measure` counts the tags and over-reports by >2×. A doc note would have been cheaper; a method that cannot be got wrong is better, and it costs ~6 lines because `shape_text` already takes a `rich` flag.
- **Remove EW-010's extra-column `warn!` rather than demote it.** The board offered "retires (or demotes to debug)". With a union schema no row can carry a column outside the schema, so the branch is unreachable — leaving a `debug!` would imply a condition that can no longer occur.
- **Fix `add_row`'s row-0 typing even though EW-010 did not ask for it.** The union change is what *creates* the hazard (an optional column is `Unit` at row 0), so shipping the union without it would have handed the game a new bug in the editor. Called out explicitly in the board thread so the game side knows it changed.
- **EW-011's capture is headless only — windowed capture deliberately scoped out.** Reading back the on-screen swapchain needs `COPY_SRC` on the surface configuration, which changes surface setup for every app on every backend and is disallowed on WebGL2. The requesting pain (a verification attempt lost to the lock screen, plus Accessibility/Screen-Recording permissions) is removed entirely by headless. Scripted **playback** works windowed too. Stated in the PR body, the CHANGELOG and the board thread, with an offer to do windowed capture as a follow-up.
- **Capture *diverts* `App::run` instead of layering onto the windowed loop.** `ENGINE_CAPTURE` runs the headless pass and returns; it never also opens a window. This is what makes the feature zero-code for an existing game — `fn main()` still just calls `app.run()`.
- **Key names as strings resolved by `key_from_name`, not winit's `serde` feature.** Avoids adding a feature to a large dependency and gives a friendlier script format. An unknown name is a load **error** — for a request whose entire premise is invisible failures, a dropped event was not an acceptable failure mode.
- **Land each request as its own PR, serially.** One PR = one coherent change (repo rule), and every release touches the same four paperwork files, so stacking would guarantee a rebase conflict on the bump. Cost: ~20 min of CI wait each. Paid it.
- **Do EW-011 only after proposing it and getting approval.** The user's instruction was explicitly "EW-009부터 ew-010까지" — two requests. EW-011 was reported as the remaining board item and started only after "진행해".

## Evidence & Data

### Shipped: request → release → commit

| Request | Priority | Version | PR | Merge commit | Merged at (UTC) |
|---|---|---|---|---|---|
| EW-009 · Text measurement API | P1 | **0.132.0** | #372 | `7c6af6d` | 2026-07-26T04:48:26Z |
| EW-010 · DataTable union schema | P2 | **0.133.0** | #373 | `ec7e18d` | 2026-07-26T05:03:34Z |
| EW-011 · Headless capture + input playback | P2 | **0.134.0** | #374 | `5944ae1` | 2026-07-26T05:37:18Z |

Board PRs in the game repo: `dungeon-merchant` **#45** `1702d27` (EW-009 + EW-010), **#46** `51ed4c6` (EW-011).

### Diffstat per PR

| PR | Files | Insertions | Deletions | Largest file |
|---|---|---|---|---|
| #372 | 11 | 934 | 74 | `examples/text_measure.rs` (+448) |
| #373 | 5 | 128 | 37 | `src/data_table.rs` (+145/−…) |
| #374 | 11 | 1364 | 6 | `src/input_script.rs` (+867) |

### Test counts

| Point | Lib tests | Added |
|---|---|---|
| Session start (post-maze, v0.131.0) | 1267 | — |
| After EW-009 (v0.132.0) | 1278 | +11 `text_measure` |
| After EW-010 (v0.133.0) | 1281 | +3 `data_table` (suite total 16) |
| **After EW-011 (v0.134.0)** | **1294** | +13 `input_script` |

### Verify-gate history (every run, real exit codes read non-piped)

| # | Tree | Exit | Cause / result |
|---|---|---|---|
| 0 | session start, clean `main` | **0** | baseline confirmed before any edit |
| 1 | EW-009 | **101** | clippy `explicit_auto_deref` ×4, `examples/text_measure.rs:221,223` |
| 2 | EW-009 + stray EW-010 edits | **101** | `cannot find type HashSet` ×4 — self-inflicted, EW-010 edited mid-run |
| 3 | EW-009 only (after stash) | **101** | rustdoc `redundant_explicit_links`, `src/text_measure.rs:11` |
| 4 | EW-009 only | **0** | 148 `test result: ok` groups |
| 5 | EW-009 + 0.132.0 bump | **0** | lock + doc re-checked |
| 6 | EW-010 | **0** | — |
| 7 | EW-010 + 0.133.0 bump | **0** | — |
| 8 | EW-011 | **0** | — |
| 9 | EW-011 + 0.134.0 bump | **0** | — |

### CI behaviour (the runner backlog recurred)

| PR | Fast checks | `Test (native)` | Total wall clock to merge |
|---|---|---|---|
| #372 | 5/6 green in ~1–3 min (WASM 42s, Rustdoc 42s, Package 1m13s, Render 1m10s, Windows 1m39s) | `pending 0s` for ~20 min | ~22 min |
| #373 | 5/6 green | trailed similarly | ~12 min |
| #374 | 5/6 green (WASM 47s, Package 1m21s, Render 1m21s, Windows 2m47s) | `pending 0s` for ~20 min | ~25 min |

`mergeStateStatus` sat at `BLOCKED` the whole time (a required check still running), never `BEHIND` — serial landing avoided the strict-checks catch-up entirely.

### EW-009 — measured vs heuristic (example's headless output, font size 15)

| String | Chars | `measure_text` width | `chars × 15 + 22` heuristic | Error |
|---|---|---|---|---|
| `사슬갑옷` | 4 | **51.9 px** | 82 | +8 px (panel incl. padding) |
| `200g` | 4 | **34.9 px** | 82 | — |
| `Iron Sword` | 10 | 89 (panel) | 172 | **+83 px** |
| `Elixir of Warding` | 17 | 122 (panel) | 277 | **+155 px** |
| `MMMM WWWW` | 9 | 130 (panel) | 157 | +27 px |
| paragraph, wrapped to 240 | — | **223 × 54** | — | fits the bound |

Worst-row heuristic error: **127 %**. Two 4-character strings (`사슬갑옷`, `200g`) differ by **1.49×** in real width — the distinction a per-character guess cannot make.

### EW-011 — the click that missed (grid geometry)

| Quantity | Value |
|---|---|
| Column origins (x) | 60, 316, **572** (`GRID_X=60`, `CELL_W=240`, `GAP=16`) |
| Row origins (y) | 130, 256 (`GRID_Y=130`, `CELL_H=110`) |
| First scripted click | `(556, 250)` → x is the **exclusive** right edge of column 1 (`316+240`), y is past row 0's bottom (`130+110`) → **gap, selects nothing** |
| Fixed click | `(690, 185)` — middle of cell index 2 |
| Result | `Menu → Shop → Detail(2)`, exit 0 |

### EW-011 — key-name table composition (`key_from_name`, 100 entries)

| Group | Count | Examples |
|---|---|---|
| Letters | 26 | `KeyA` … `KeyZ` |
| Digits | 10 | `Digit0` … `Digit9` |
| Function | 12 | `F1` … `F12` |
| Named (nav/modifier/punctuation) | 36 | `Space`, `Escape`, `ArrowUp`, `ShiftLeft`, `BracketLeft`, `Backquote` |
| Numpad | 16 | `Numpad0` … `Numpad9`, `NumpadEnter`, `NumpadAdd` |

`key_names()` returns the same list; the test `key_and_button_names_resolve` asserts **every advertised name resolves**, so the table and the matcher cannot drift.

### Public API added (the whole surface, for a fork or a downstream game)

**EW-009 — `engine::text_measure`, re-exported at the crate root:**

| Item | Signature / note |
|---|---|
| `TextMeasurer` | resource struct owning a `FontSystem` |
| `TextMeasurer::new` | `(font_data: &[u8], extra_fonts: &[Vec<u8>]) -> Self` — insert at setup to pre-pay the font scan |
| `TextMeasurer::measure` | `(&mut self, text: &str, font_size: f32) -> Vec2` — single line, no wrap |
| `TextMeasurer::measure_rich` | same, but `[color=…]`/`[b]`/`[i]` markup parsed away first |
| `TextMeasurer::measure_wrapped` | `(&mut self, text, font_size, max_width) -> Vec2` — `x` = longest wrapped line; `max_width <= 0` measures unwrapped |
| `TextMeasurer::line_height` | `(font_size: f32) -> f32` — associated fn, `= size × 1.2` |
| `text_measurer` | `(world: &mut World) -> &mut TextMeasurer` — lazily inserts |
| `measure_text` / `measure_text_wrapped` | world-level free fns |
| `App::measure_text` / `App::measure_text_wrapped` | `&mut App` twins |

**EW-011 — `engine::input_script`, re-exported at the crate root:**

| Item | Signature / note |
|---|---|
| `InputScript` | resource; `Default` = empty |
| `InputScript::new` | `(impl IntoIterator<Item = (u32, InputAction)>) -> Self` — sorted, stable within a frame |
| `InputScript::from_ron_str` / `load` | `-> Result<Self, InputScriptError>`; `load` resolves against asset roots (native) |
| `InputScript::apply` | `(&mut self, world: &mut World)` — one frame; releases pending, applies batch, `frame += 1` |
| `InputScript::frame` / `len` / `is_empty` / `is_finished` / `last_frame` | accessors |
| `InputAction` | `#[non_exhaustive]`: `KeyDown` / `KeyUp` / `KeyPress` / `MouseMove(Vec2)` / `MouseDown` / `MouseUp` / `Click` / `Scroll(f32)` / `Quit` |
| `ScriptedInput` | `{ frame: u32, action: InputAction }` |
| `InputScriptError` | `#[non_exhaustive]`: `Ron(String)` / `Io(String)`; impls `Display` + `Error` |
| `key_from_name` / `key_names` / `mouse_button_from_name` | name → `KeyCode` / `&[&str]` / name → `MouseButton` |
| `App::set_input_script` | `(&mut self, InputScript) -> &mut Self`; also `register_persistent` |
| `App::capture_frames_headless` | `(&mut self, &[(u32, impl AsRef<Path>)]) -> Result<Vec<PathBuf>, String>` |

### Test inventory (all 27 added, by module)

| Module | Tests |
|---|---|
| `src/text_measure.rs` (11) | `measures_ascii_text`, `empty_text_measures_zero`, `longer_text_is_wider`, `same_char_count_can_differ_in_width`, `width_scales_with_font_size`, `newline_adds_a_line_of_height`, `wrapping_respects_max_width_and_grows_taller`, `non_positive_max_width_does_not_wrap`, `rich_markup_is_not_counted`, `line_height_is_the_shared_factor`, `world_level_measure_creates_the_resource_lazily` |
| `src/data_table.rs` (3) | `schema_is_the_union_of_all_rows`, `row_zero_complete_table_parses_unchanged`, `add_row_types_a_late_column_from_the_row_that_has_it` |
| `src/input_script.rs` (13) | `a_scripted_press_reads_as_just_pressed_in_its_own_frame`, `key_down_is_held_until_key_up`, `a_scripted_click_moves_the_cursor_and_clicks_there`, `scroll_and_quit_reach_their_resources`, `events_are_sorted_by_frame_and_stable_within_one`, `an_event_whose_frame_passed_is_not_dropped`, `parses_a_ron_script`, `an_unknown_key_name_is_an_error`, `key_and_button_names_resolve`, `an_empty_script_is_finished_immediately`, `capture_plan_parses_frames_and_paths`, `capture_plan_keeps_a_drive_letter_in_the_path`, `capture_plan_skips_malformed_entries` |

### CHANGELOG section headings used (house format)

| Version | Sections |
|---|---|
| 0.132.0 | `### Added`, `### Changed (internal)` |
| 0.133.0 | `### Fixed`, `### Added` |
| 0.134.0 | `### Added`, `### Changed (internal)` |

Each entry opens with a bold one-line summary plus 3–6 sentences of context, and explicitly states "Purely additive: no existing type or signature changed" where true.

### The example self-checks (what each headless run asserts)

| Example | Checks | Exit on failure |
|---|---|---|
| `text_measure` | (1) Hangul ≥ `size × 3` wide (fallback font present, not tofu/zero); (2) Hangul > 1.3 × same-char-count Latin (script-aware); (3) worst-row heuristic error > 10 % (the example proves something); (4) at `MAX_FONT_SIZE` at least one row clips (size-dependence is real); (5) wrapped width ≤ bound and taller than one line | `eprintln!("FAIL: …")` per failure, then `exit(1)` |
| `scripted_capture` | script is non-empty; after the run the app is on `Screen::Detail(_)` | `exit(1)` with the observed screen |

### Reusable procedure — landing N independent requests serially

This session's mechanics, worth repeating verbatim next time the board has several entries:

1. Verify clean `main` **before any edit** (`./scripts/verify.sh > log 2>&1; echo $?`).
2. Branch request A off `main`. Implement + test A only.
3. If request B was started accidentally, `git stash push -m … <B's files>` — works cleanly only because the requests touch **disjoint** files. Check that first.
4. Full verify on A alone → `/ship` (bump + lock + CHANGELOG + CLAUDE.md header) → **re-verify** → commit → push → PR → `gh pr merge <n> --auto --squash`.
5. **Wait for A to merge** before B's paperwork. Branch B off `main`, `git stash pop`, implement, verify.
6. After A merges: `git checkout main && git pull --ff-only`, `git checkout <B>`, `git rebase main`, `git stash pop` if needed — *then* do B's version bump (never before, or the bump conflicts).
7. Repeat. Update the shared board once per batch (one PR for the first two, one for the third here), and bump memory after each merge.

### New-file line counts

| File | Lines |
|---|---|
| `src/input_script.rs` | 867 |
| `examples/text_measure.rs` | 448 |
| `src/text_measure.rs` | 367 |
| `examples/scripted_capture.rs` | 346 |
| `examples/scripted_capture.ron` | 27 |

### The verification command the game can now run (EW-011's deliverable)

```sh
ENGINE_INPUT=examples/scripted_capture.ron \
ENGINE_CAPTURE=8:/tmp/menu.png,28:/tmp/shop.png,52:/tmp/detail.png \
cargo run --example scripted_capture
# → ENGINE_CAPTURE wrote /tmp/menu.png
#   ENGINE_CAPTURE wrote /tmp/shop.png
#   ENGINE_CAPTURE wrote /tmp/detail.png
# No window opened. No display required. No OS permissions.
```

### EW-010 — the parse loop, before and after (primary evidence)

```rust
// BEFORE (src/data_table.rs, ~L100–126)
let first_pairs = extract_pairs(rows_raw[0].clone())?;          // ← row 0 IS the schema
let mut columns: Vec<String> = first_pairs.iter().map(|(c, _)| c.clone()).collect();
columns.sort();
for (idx, raw) in rows_raw.into_iter().enumerate() {
    let pair_map: HashMap<String, ron::Value> = extract_pairs(raw)?.into_iter().collect();
    for key in pair_map.keys() {
        if !columns.contains(key) {
            log::warn!("data_table: row {idx} has extra column '{key}' … value discarded");
        }                                                        // ← silent data loss
    }
    …
}

// AFTER
let all_pairs = rows_raw.into_iter().map(extract_pairs).collect::<Result<Vec<_>, _>>()?;
let mut columns: Vec<String> = Vec::new();
let mut seen: HashSet<&str> = HashSet::new();
for pairs in &all_pairs {
    for (col, _) in pairs {
        if seen.insert(col.as_str()) { columns.push(col.clone()); }
    }
}
columns.sort();                                                  // ← union, then the SAME Unit fill
```

### EW-011 — the z-layering the first capture forced (the bug it caught)

| Element | `z` | Why |
|---|---|---|
| Shop cell rects | 0.2 | base |
| Shop labels + `SHOP` title | **0.3** | **added after the capture** — without a `z` they draw in the final on-top pass and read straight over the popup |
| Detail scrim | **0.5** | added after the capture |
| Detail popup rect | 0.6 | above the scrim |
| Detail popup text | 0.7 | above its own panel |
| Bottom HUD hint | *none* | deliberately on-top — demonstrates the correct use of un-z'd text |

### Screenshot evidence (what the captures actually showed)

| File | Content | Verdict |
|---|---|---|
| `/tmp/text_measure.png` | 5 tooltip rows, heuristic panels (amber, loose) left vs measured panels (teal, tight) right, per-row deltas `+8/+83/+20/+155/+27 px`, wrapped paragraph panel inside its 240 px bound | Ticks land on the glyph ends → the ~1px criterion, visually |
| `/tmp/sc_detail.png` (first) | Detail popup drawn, but `Iron Sword` / `45g` / `Oak Shield` / `Rope` / `80g` / `6g` reading **over** it | **BUG** — un-z'd text |
| `/tmp/sc2_detail.png` (after fix) | Same popup, labels correctly hidden behind the scrim; clicked item (`체력 물약`) tinted green in the grid | Correct |
| `/tmp/mz_b.png` | `maze_generation` HUD reads `seed 2 · braided · 635 floor · 16 dead ends` | Scripted `B` + `R` reached an unmodified example |

### The verify gate (7 steps, all must pass)

`./scripts/verify.sh` runs, in order: `cargo fmt --check` → `cargo clippy --all-targets -- -D warnings` → `cargo build --target wasm32-unknown-unknown` (lib+bins only) → `cargo clippy --target wasm32 --lib -- -D warnings` → `cargo test --all-targets` → `cargo test --doc` → `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`. All three of this session's red runs failed at a *different* step (clippy, test-compile, rustdoc), which is why reading the log rather than re-running blind mattered each time.

### The bundled script (primary artifact — `examples/scripted_capture.ron`)

```ron
(
    events: [
        (frame: 15, action: KeyPress("Digit2")),
        (frame: 40, action: MouseMove(690.0, 185.0)),
        (frame: 41, action: Click("Left")),
        (frame: 70, action: Quit),
    ],
)
```

## Code Analysis

- **`shape_text(font_system: &mut FontSystem, spec: &ShapeSpec<'_>) -> Buffer`** (`src/renderer/text/renderer.rs`, `pub(crate)`). `ShapeSpec { text, size, width: Option<f32>, height: Option<f32>, wrap: Wrap, align: Option<glyphon::cosmic_text::Align>, rich: bool }`. Builds `Metrics::new(size, size * LINE_HEIGHT_FACTOR)`, `set_size`, `set_wrap`, `set_text`/`set_rich_text` with `Shaping::Advanced` and `Attrs::new().family(Family::SansSerif)`, per-line `set_align`, then `shape_until_scroll`.
- **`LINE_HEIGHT_FACTOR: f32 = 1.2`** (`pub(crate)`) — now the single source for the renderer's `Metrics`, the `TextAnchor::Center` vertical offset, and `TextMeasurer::line_height`.
- **`TextMeasurer::measure_inner(text, font_size, width, wrap, rich) -> Vec2`** — early-returns `Vec2::ZERO` for empty text, shapes with `height: None` (a measurement must never be clipped by a box it does not have) and `align: None` (alignment moves glyphs inside the box; it cannot change how wide they are), then takes `max(run.line_w)` over `layout_runs()` and `lines × line_height`.
- **`font_blobs(world: &World) -> (Vec<u8>, Vec<Vec<u8>>)`** (`src/text_measure.rs`, `pub(crate)`) — `FontData` + `ExtraFonts` with the wasm `DEFAULT_FONT` fallback. Called by both `App::init_gpu_renderers` (`src/app/window.rs`) and `text_measurer(world)`.
- **`DataTable::parse`** — `all_pairs: Vec<Vec<(String, ron::Value)>>` collected once via `Result<Vec<_>, _>`; `columns` built with a `HashSet<&str>` seen-set borrowed from `all_pairs` (NLL ends the borrow before the later `into_iter()`), then `sort()`; rows built by looking each column up in a per-row `HashMap` with `unwrap_or(ron::Value::Unit)`.
- **`DataTable::default_value_for_col`** — now `(0..rows.len()).filter_map(|r| self.get(r, col)).find(|v| !matches!(v, ron::Value::Unit)).map(default_ron_value).unwrap_or(Unit)`.
- **`InputScript`** — `events: Vec<ScriptedInput>` (sorted by frame, `sort_by_key` so same-frame order is stable), `next: usize` cursor, `frame: u32` counter, `pending_keys: Vec<KeyCode>` / `pending_buttons: Vec<MouseButton>` for one-frame-later releases. `apply(&mut World)` drains pending releases **first**, then applies this frame's batch, then `frame += 1` (saturating).
- **`InputScript::apply` catch-up behaviour** — an event whose frame has already passed is applied rather than dropped (only reachable if the caller skips frames); the `break` only fires for `event.frame > self.frame`. Covered by `an_event_whose_frame_passed_is_not_dropped`.
- **`App::capture_frames_headless(&[(u32, impl AsRef<Path>)])`** — builds one headless `GpuContext`, inserts `ViewportSize`, then loops `0..=max(frame)` doing `update` + `render`, reading back and writing a PNG at each listed frame. Returns `Vec<PathBuf>`.
- **`parse_capture_plan(spec: &str)`** — splits on `,`, then `split_once(':')` (**first** colon only, so a Windows `C:\…` path survives), logs and skips malformed entries.
- **Injection ordering** — `App::update` step 0 calls `apply_input_script_frame`, which `remove_resource::<InputScript>()` → `apply` → `insert_resource` (the remove/re-insert dance the repo uses elsewhere for `Pool`), before `run_systems`. `input.flush()` remains at `src/app/schedule.rs:516` in `post_systems`.
- **Surface configuration** — `src/renderer/context.rs:160`: `usage: wgpu::TextureUsages::RENDER_ATTACHMENT` (no `COPY_SRC`). This single line is why windowed capture was scoped out.

## Files Changed

### Source code
- `src/text_measure.rs` — **NEW** (367). `TextMeasurer`, world-level fns, `App` methods, `font_blobs`, 11 tests.
- `src/input_script.rs` — **NEW** (867). `InputScript`/`InputAction`/`ScriptedInput`/`InputScriptError`, key + button name resolution (100 keys), RON serde mirror, `ENGINE_CAPTURE` parser, `App::set_input_script`/`apply_input_script_env`/`apply_input_script_frame`, 13 tests.
- `src/renderer/text/renderer.rs` — extracted `shape_text` + `ShapeSpec` + `LINE_HEIGHT_FACTOR`; `build_font_system` `pub(super)` → `pub(crate)`; the render closure now calls `shape_text` with its former inline arguments.
- `src/renderer/text.rs` — `pub(crate) use renderer::{build_font_system, shape_text, ShapeSpec, LINE_HEIGHT_FACTOR};`.
- `src/data_table.rs` — union column schema; `warn!` removed; `add_row` typing fixed; doc comments on the struct, the `columns` field and `parse` rewritten; 3 tests.
- `src/app/headless.rs` — `App::capture_frames_headless` + shared `save_rgba_png`.
- `src/app/window.rs` — `App::run` reads `ENGINE_INPUT`/`ENGINE_CAPTURE` before the event loop; font-blob collection replaced by `font_blobs(&self.world)`.
- `src/app/schedule.rs` — `App::update` step 0 calls `apply_input_script_frame`.
- `src/app.rs` — `register_persistent::<TextMeasurer>()` in `App::new`; unused `FontData` import removed.
- `src/lib.rs` — `pub mod text_measure` / `pub mod input_script` + their re-exports.

### Examples (the acceptance tests)
- `examples/text_measure.rs` — **NEW** (448). Heuristic-vs-measured tooltip panels, tick at the true text edge, ↑↓ size, ←→ wrap width; headless self-check (script-awareness, heuristic disagreement, clipping at a larger size, wrap bound).
- `examples/scripted_capture.rs` — **NEW** (346). Three-screen toy shop; headless pass captures one PNG per screen and asserts the script reached `Detail`.
- `examples/scripted_capture.ron` — **NEW** (27). The bundled input script.

### Docs / release
- `docs/CHANGELOG.md` — 0.132.0, 0.133.0, 0.134.0 entries.
- `Cargo.toml` / `Cargo.lock` — 0.131.0 → 0.132.0 → 0.133.0 → 0.134.0.
- `CLAUDE.md` — header v1.6.225 → v1.6.228; three module-map rows (TextMeasurer; DataTable row extended; scripted input + capture).

### Cross-repo (not in this repo's PRs)
- `../dungeon-merchant/docs/engine-wishlist.md` — EW-009/010/011 marked `Shipped`, with `[Engine] 2026-07-26` thread comments (PRs #45, #46).

### Memory (not in any PR)
- `engine-current-state.md` — seq 191/192/193 tip-line prepends + a new BOARD paragraph.
- `MEMORY.md` — index hook refreshed to `main @ 5944ae1` / v0.134.0.

## User Feedback & Preferences (REQUIRED)

- **"마지막 핸드오프 확인하고 다음 작업 알려줘"** — the opening instruction. Report first, act second: the user wanted the board/handoff state and a recommendation, not immediate work.
- **"EW-009부터 ew-010까지 진행"** — an explicit, bounded scope: two requests, not all three. EW-011 was *not* authorized by this message.
- **"진행해"** — terse approval for EW-011, after it was proposed at the end of the EW-009/010 report. Same terse-approval pattern the parent handoff recorded; no re-planning invited.
- **"머지 해줘"** (as `/handoffplan` argument) — merge the handoff/plan work as well; do not leave it uncommitted.
- **Board-first is standing.** Read `../dungeon-merchant/docs/engine-wishlist.md` and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` before anything else; a filed request preempts self-directed breadth. When both are empty, **ASK — do not self-pick.**
- **Merge authority is standing-delegated** — squash on green CI, async auto-merge default, no per-PR confirmation. Exercised on all three PRs plus both board PRs.
- **Korean to the user, English in artifacts** — every status report, question and summary in Korean; code, comments, commit messages, PR bodies, CHANGELOG, board threads and this handoff in English.
- **No mid-session course corrections.** After each go-ahead the session ran end-to-end (design → implement → verify → ship → PR → merge → board → memory) with no further input. Calibration: once scope is approved, proceed autonomously through the whole loop.
- **Scope discipline is expected in both directions.** The user's "EW-009부터 ew-010까지" was honoured literally (EW-011 held back and re-proposed), and conversely two out-of-scope-but-necessary fixes (`add_row` typing, the example's z-order) were made and reported rather than silently included or silently skipped.

## Where We're Going

1. **Board gate FIRST, every session** — `../dungeon-merchant/docs/engine-wishlist.md` (next free **EW-012**; EW-001–011 all closed or awaiting game Verify) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`). A newly-filed request preempts everything below. Note the board's one remaining `Proposed` grep hit is the blank `EW-NNN` template row at `:202` — not a request.
2. **If both are empty (likely): ASK — do NOT self-pick.** Concrete menu, in rough priority order:
   - **Windowed frame capture** — the one thing deliberately cut from EW-011. Needs `COPY_SRC` added to the surface configuration (`src/renderer/context.rs:160`), guarded by `caps.usages`, plus a swapchain read-back before `present()`. Offer it to the game side first; they may not need it.
   - **Byte-source atlas parity (`load_atlas_bytes`)** — still open from the `asset-root-windows` chain; closes the single-file/`include_bytes!` build story that `load_image_bytes` started.
   - **A fourth procgen mode or the deferred `MapGenerator` trait** — the procgen family is complete, so this is optional breadth. The trait's trigger (a game wanting to pick a generator at runtime) still has not fired.
   - **Audio-reactive hooks** or a **second capstone game** — fresh areas.
3. **Watch for the game's `Verified` marks.** All three requests are in the game's court. If the game reports a problem, that preempts the menu above.
4. **This handoff and its plan still need to land** as their own `docs(handoff)` PR (repo convention). The memory seq bump for the *handoff* belongs to that PR's landing — bump to **seq 194** after it merges, so the recorded `main @ <hash>` points at the handoff merge rather than at `5944ae1`.

## Risks & Blockers

- **The background-task notification lies about gate exit codes.** A `run_in_background` `./scripts/verify.sh > log 2>&1; echo $? > exit` reports the *trailing `echo`'s* status (always 0) in its completion summary. Hit **twice** this session — once masking a clippy failure, once a rustdoc failure. **Always read the `.exit` file**, never the notification's "exit code 0".
- **zsh's pipe-status array is `$pipestatus` and is 1-indexed.** Used `${pipestatus[2]}` (which was `tail`) and read a false `EXIT=0` over a real `exit(1)`. For a pipeline, index 1 is the first command. Better: don't pipe a gate at all.
- **GitHub hosted-runner backlog is live.** `Test (native)` sat `pending 0s` for ~20 min on two of three PRs while the other five checks finished in 1–3 min. It is GitHub-side, clears itself, and auto-merge rides it out. Budget ~25 min per PR, and do not force re-triggers.
- **Serial PRs cost wall-clock but avoid conflicts.** Every release touches `Cargo.toml`, `Cargo.lock`, `docs/CHANGELOG.md` and the `CLAUDE.md` header line, so two open release PRs conflict on the version bump. Branch from freshly-pulled `main` after each merge, and do the `/ship` paperwork *after* the previous PR has landed.
- **Do not edit a second feature's files while a verify run is in flight.** Doing so produced a red gate (`cannot find type HashSet`) that had nothing to do with the tree being verified, and cost a diagnosis cycle.
- **CLAUDE.md is at ~208 lines**, over the 200-line soft cap and now three rows heavier. A future edit that adds a module-map row should trim detail into a `docs/*.md` per the doc rule.
- **`dungeon-merchant` has no CI/branch protection** — its board is discipline, not enforcement. `rust-survivors` is paused/deprecated; do not chase compatibility.
- **2 audio tests can fail on a no-audio box** (carried gotcha) — not hit this session; relevant to any future audio work.

## Open Questions

- **Does the game actually need windowed capture?** EW-011 shipped headless-only and said so on the board with an offer to follow up. The answer decides whether the `COPY_SRC` surface change is worth making.
- **Should the `TextMeasurer`'s `FontSystem` eventually be shared with the renderer's?** Today they are two instances built from the same blobs (duplicate system-font scan, duplicate font DB). If font memory ever matters, the `Rc<RefCell<…>>` design in "What We Tried" step 6 is the sketch — with its double-borrow hazard.
- **Should `InputScript` gain a `Capture(path)` action?** Multiple captures are already covered by `ENGINE_CAPTURE`'s comma list, so it was left out. A game wanting a capture at a *state-dependent* moment (rather than a fixed frame) would be the trigger.
- **Is the 100-key `key_from_name` table the right surface?** `KeyCode` is `#[non_exhaustive]`; media/international keys are deliberately absent. Adding them is mechanical if a game asks.
- **Does `measure_wrapped` need a `measure_rich_wrapped`?** The three-method surface covers the filed need; the fourth combination was left out rather than added speculatively.

## Quick Start for Next Session

```bash
# Nothing is dangling — all three PRs merged, main is clean at 5944ae1 (v0.134.0).

cd ~/Projects/skeleton-engine
git log --oneline -4      # expect 5944ae1 (v0.134.0) at the tip
git status -s             # expect clean

# 1. READ THE BOARD FIRST — both channels (a new request preempts everything)
#   ../dungeon-merchant/docs/engine-wishlist.md        (next free EW-012; EW-009/010/011 = Shipped,
#                                                       awaiting the GAME's Verified mark)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md   (currently _None._)
#   NOTE: the lone remaining `Status: `Proposed`` hit is the blank EW-NNN template row at :202.

# 2. Verify starting state — read the exit code, do NOT pipe or ;-chain it
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# 3. Re-prove this session's three features
cargo test --lib text_measure    # expect 11 passed
cargo test --lib data_table      # expect 16 passed
cargo test --lib input_script    # expect 13 passed
HEADLESS_SHOT=/tmp/tm.png cargo run --example text_measure
# expect: 사슬갑옷=51.9px  200g=34.9px  worst heuristic error=127%  wrapped=223×54 ; exit 0
HEADLESS_SHOT=/tmp/sc cargo run --example scripted_capture
# expect: 3 PNGs + "OK: scripted keys + click drove Menu → Shop → Detail(2) with no window"; exit 0

# 4. Key files to read first (not exhaustive)
#   src/text_measure.rs                  — TextMeasurer + the world-level API
#   src/renderer/text/renderer.rs        — shape_text / ShapeSpec / LINE_HEIGHT_FACTOR (the shared path)
#   src/input_script.rs                  — InputScript + key_from_name + ENGINE_CAPTURE parsing
#   src/app/headless.rs                  — capture_frames_headless
#   examples/scripted_capture.rs + .ron  — the example shape and the bundled script

# 5. First action: board gate → if empty, ASK for direction. Do NOT self-pick.
#    Menu: windowed capture (the deliberate EW-011 cut — needs COPY_SRC on the surface),
#    load_atlas_bytes, a 4th procgen mode / the MapGenerator trait, audio-reactive hooks,
#    or a 2nd capstone game.
```

---

## Session Closed

**Closed at:** 2026-07-26
**Code shipped:** PRs #372 `7c6af6d` (v0.132.0), #373 `ec7e18d` (v0.133.0), #374 `5944ae1` (v0.134.0) — all merged and on `main`.
**Board:** `dungeon-merchant` PRs #45 `1702d27` and #46 `51ed4c6` — EW-009/010/011 all `Shipped`, awaiting the game's `Verified`.
**This handoff lands as:** its own `docs(handoff): board-ew-triple seq-1` PR (project convention). Memory `engine-current-state` bumps to **seq 194** after that docs PR merges.
**Session status:** Handed off to next session.
