# dm-adoption seq 2 — both sanctioned self-picks landed (dropdown seam fix v0.113.1 + game_feel web ship v0.113.2), async auto-merge proven unattended

**Date:** 2026-07-03
**Status:** COMPLETED (both parent-sanctioned self-picks merged; board still empty; game session still hasn't run; self-pick queue now FULLY exhausted — next empty-board session must ASK)
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `dm-adoption` seq `2`
**Parent:** `HANDOFF_dm-adoption_board-notice-automerge_2026-07-02.md`
**Prior chain:** `HANDOFF_game-feel-ui-breadth_2026-07-02.md` > `HANDOFF_game-feel-ui-breadth_widget-suite-text-z_2026-07-02.md` > `HANDOFF_game-feel-ui-breadth_capstone-trim-playtest_2026-07-02.md` > `HANDOFF_dm-adoption_board-notice-automerge_2026-07-02.md` > this

---

## Stale References

- `examples/game_feel.rs` — no longer at that path; **moved to `examples/game_feel/game_feel.rs`** (directory style + a `[[example]]` Cargo.toml entry) by #335 this session. Parent's Code Analysis line references (HitFlash skip L119, TimeScale resolver L274-283, etc.) still hold in the moved file, shifted by ~+5 lines (the added web doc comment + cfg import).
- `src/ui/system/dropdown_pass.rs:159-174` per-row rect loop — **gone**; #334 replaced it with the one-background + inset-highlight render (~L143-192 now). The parent's "shovel-ready seam nit" is CLOSED.

## Since Last Handoff

- **Parent §1 (land handoff async + seq 153) — happened, split across the session boundary:** #333 auto-merged (`898e467`) minutes after the parent session closed; its deferred wrap-up (pull main, delete branch, memory seq 153) was executed at THIS session's start. The async-mode deferred-wrap-up handshake across a session boundary worked exactly as designed.
- **Parent §2 (user pastes the game-session starter prompt) — has NOT happened yet:** game repo still `main...origin/main [ahead 1]` (`19d43a8` unpushed), pin still `1c19873` = v0.62.0. The demand pipe is still dry.
- **Parent §3 (board FIRST, EW-004+ expected) — board still ACTIVE EMPTY**, sixth consecutive session (next free ID EW-004). The `[Engine]` notice sits on the board unanswered.
- **Parent §4 (if board empty: seam fix, then game_feel wasm-ship, nothing else) — BOTH executed and merged** (#334 v0.113.1, #335 v0.113.2). The sanctioned self-pick list is now EMPTY.
- **Parent §5 (watch the async wrap-up discipline) — exercised twice unattended:** #334 and #335 both merged by GitHub while the session worked on the next task; deferred wrap-ups (pull, branch delete, memory bump) batched cleanly at natural checkpoints. No land-pr skill refinement needed.
- Parent risk "checks-registration lag" did not bite — PR state was polled via `gh pr view --json state` at checkpoints instead of an immediate `checks --watch`.
- Parent's spurious-notification protocol applied to all 3 background completions this session (2 verifies + 1 leftover PR-watch task from the parent session); all were genuine.

## Reference Documents

- `CLAUDE.md` — header now **v1.6.206 / package v0.113.2**; UI-row `game_feel` mention gained the web-ship note.
- `docs/CHANGELOG.md` — new `## 0.113.1` (seam fix) + `## 0.113.2` (web ship) entries.
- `docs/WASM_SMOKES.md` — new game_feel render-smoke bullet (after the centered_text one).
- `.claude/skills/land-pr/SKILL.md` — Async mode section (followed verbatim, including the serial-multi-PR rebase guidance).
- `.claude/skills/ship-wasm-example/SKILL.md` — the template followed for #335 (dual-target `build_app()` rule, smoke-model choice, gotcha list).
- Parent handoff — Evidence § still holds the game-session starter prompt (re-issue it from there if needed).
- Memory: `engine-current-state.md` bumped 3× this session (seq 153 → 154 → 155); seq 156 due when THIS handoff's PR merges.

## The Goal

Continue the `dm-adoption` arc opened in seq 1. The arc's demand pipe (the Dungeon-Merchant game session filing EW-004+ requests) had not started flowing yet, so this session's job was the parent's explicit fallback: execute the only two sanctioned self-picks — the shovel-ready dropdown row-seam fix and the game_feel wasm-ship — while keeping main clean/green, and prove the brand-new async auto-merge landing infra works unattended (it was only one landing old). Everything beyond those two was explicitly forbidden ("do not invent beyond those two; the demand pipe is now the game session").

## Where We Are

- **main @ `3e0645c`** — package **v0.113.2**, CLAUDE.md header **v1.6.206**, tree clean, memory at **seq 155**.
- **3 PRs on main this session:** #333's wrap-up (merged `898e467` before session start), **#334** dropdown seam fix (merged `4b44534`, v0.113.1), **#335** game_feel web ship (merged `3e0645c`, v0.113.2). #334 and #335 both landed via `gh pr merge --auto --squash` with zero CI babysitting.
- **Dropdown seam fix (#334):** `src/ui/system/dropdown_pass.rs` open-list render = ONE full-list rounded background (`DrawRect(list_pos.x, list_pos.y, size.x, list_h)` at `DROPDOWN_LIST_Z + UI_SUBLAYER_Z_STEP`) + a square hover highlight inset horizontally by the clamped corner radius at `+2×UI_SUBLAYER_Z_STEP` + row labels moved up to the same `+2×step` (equal-z tie renders text over surface, so the hovered label stays visible above its highlight).
- The old per-row `DrawRect::new(pos.x, row_y, size.x, item_h, row_color).with_corner_radius(...)` loop is gone — stacked 4-corner-rounded rows were what left the notch seams (and let covered widget labels bleed through the gaps).
- Highlight inset formula: `dd.corner_radius.clamp(0.0, size.x.min(list_h) / 2.0)` — a square highlight at full width would poke its corners outside the background's rounded first/last-row corners; `DrawRect` has no per-corner radii so inset was the only clean shape.
- **18 dropdown tests green** (was 17): existing `open_list_rows_reach_the_ui_queue_at_the_list_z` updated (box + 1 bg ≥ 2 rects, was box + 3 rows ≥ 4), new `open_list_draws_one_background_plus_an_inset_hover_highlight` asserts exact geometry — bg `(50, 80, 120, 90)` `corner_radius 6.0`, highlight `(56, 110, 108, 30)` `corner_radius 0.0`, `hl.z > bg.z`.
- **Headless A/B captures** confirmed the fix: before = seam notches between rows + faint covered-label bleed between "High"/"Ultra"; after = continuous background, zero bleed, box/arrow/`•` marker/hover/HUD byte-identical (paths in Evidence).
- **game_feel web ship (#335):** example moved to `examples/game_feel/game_feel.rs` + `[[example]]` entry; setup factored into a shared `fn build_app() -> App` called by the native `main()` (windowed + `HEADLESS_SHOT`) and the new `#[cfg(target_arch = "wasm32")] #[wasm_bindgen] pub fn run_game_feel()`.
- The wasm example build **failed before the refactor** — `E0599: no method named save_screenshot_headless` — because the headless branch was env-var-gated at runtime but not cfg-gated at compile time (exactly what the parent's Code Analysis predicted). Now the whole HEADLESS_SHOT branch lives inside the `#[cfg(not(wasm32))] main()`.
- **`examples/game_feel/web/{build.sh,index.html}`** — verbatim centered_text template (cargo build --release --example → wasm-bindgen --target web → pkg/): Start button (winit needs a user gesture), `?autostart=1` for headless, canvas 900×540 with `tabindex` + focus for ←/→/Space/X/Esc.
- **`scripts/game_feel_web_smoke.sh`** — render-only model (centered_text): builds, serves orphan-safe on :8087, renders one headless DPR=2 SwiftShader frame, asserts non-blank. **PASS: 66,611 bytes ≥ 15,000**; frame eyeballed (two pads + gap, three dummies, yellow player, both HUD lines).
- `docs/WASM_SMOKES.md` gained the game_feel bullet; CLAUDE.md UI row notes the web ship; `pkg/` confirmed gitignored.
- **Full 7-gate verify ran 3× this session, all `VERIFY_EXIT=0`** (onboarding baseline on main, #334 branch, #335 branch), every exit read non-piped with the sentinel + `pgrep` liveness check.
- **Board still ACTIVE EMPTY** (EW-004 next free ID); game repo untouched since seq 1 (`19d43a8` unpushed, pin v0.62.0).
- Memory: seq 153 (#333 wrap), seq 154 (#334, incl. the new UiQueue test gotcha), seq 155 (#335) all bumped in both `engine-current-state.md` and `MEMORY.md`.
- **NEW test gotcha discovered and memory-logged (seq 154):** in unit tests, `UiQueue.items` **accumulates across `system.run()` calls** — frame.rs normally drains it each frame, tests don't — so a rect-counting test spanning two frames must `world.resource_mut::<UiQueue>().unwrap().items.clear()` between frames (first version of the new geometry test counted 3 rects instead of 2 because the opening frame's bg was still queued).

## What We Tried (Chronological)

1. **Onboarding (paste-prompt protocol, 4th session running).** Board checked FIRST — still empty, game session hadn't run (game repo `ahead 1`, pin v0.62.0). Git fetch showed origin/main at `731a430`… then a leftover background task from the parent session ("Watch PR 333") completed; sentinel `PR333_MERGED` + `gh pr view` confirmed #333 had merged (`898e467`) — the fetch had raced the merge by seconds. Verify baseline launched in background; key files + 3 adjacent read (dropdown.rs geometry helpers already unit-tested; `DrawRect` confirmed to have NO per-corner radii; ui_dropdown's `HEADLESS_SHOT` + `auto_open` support = the visual-verification path). All green → presented plan, got go-ahead.
2. **#333 deferred wrap-up.** `git checkout main && git pull --ff-only` (`731a430 → 898e467`), deleted `docs/handoff-dm-adoption-seq1`, bumped memory to seq 153 using the parent's recorded anchor phrasings (both matched exactly — copy-paste worked).
3. **Seam fix implementation.** Replaced the per-row rounded-rect loop with bg + inset highlight + relayered text. First run of the new geometry test **FAILED: 3 rects, expected 2** → root cause = UiQueue accumulation across test frames (not a render bug) → cleared the queue between the open-click frame and the hover frame → 18/18 green.
4. **Headless A/B.** Captured post-fix `ui_dropdown` shot, then `git stash` → captured pre-fix baseline → `git stash pop`. Diff confirmed: seams + inter-row label bleed gone; the "Back" text visible in BOTH shots sits genuinely BELOW the list's bottom edge (y=250) — pre-existing example layout, not a regression.
5. **/ship v0.113.1 + land async #334.** Four-file paperwork; full verify green (`VERIFY_EXIT=0`); commit; push; PR #334; `gh pr merge --auto --squash`; moved on immediately.
6. **game_feel wasm-ship (started while #334's CI ran).** Branched `feat/game-feel-web` off main@`898e467` (pre-#334 — deliberate, see Key Decisions). wasm-bindgen crate/CLI versions matched (0.2.122). First `cargo build --example game_feel --target wasm32` **FAILED E0599** on `save_screenshot_headless` → refactored to `build_app()` + cfg-gated native `main()` + `run_game_feel()` (the /ship-wasm-example dual-target rule) → both native and wasm builds green.
7. **Directory-style move + harness.** `git mv examples/game_feel.rs examples/game_feel/game_feel.rs` + `[[example]]` entry (the centered_text/web_audio/wasm_save precedent); wrote `web/build.sh` + `web/index.html`; `build.sh` generated `pkg/` (gitignored ✓). rust-analyzer threw stale intermediate-state errors mid-edit (E0601/E0069/E0308) — ignored per the live-gotchas memory; real cargo builds were green.
8. **Smoke + docs.** Wrote `scripts/game_feel_web_smoke.sh` (render-only model incl. the orphan-safe server + port-guard pattern); PASS 66,611 bytes; frame eyeballed. Added the WASM_SMOKES.md bullet + the CLAUDE.md UI-row web-ship note.
9. **#334 merged mid-work** (`4b44534`, ~19 min after arming). Committed the WIP, synced main, deleted the fix branch, **rebased `feat/game-feel-web` onto main** (clean — no overlapping hunks), bumped memory to seq 154.
10. **/ship v0.113.2 on the rebased branch** (version line now 0.113.1 → 0.113.2 with no conflict — the rebase-first ordering avoided the stacked-PR version-line collision). Full verify green. `git reset --soft main` → one clean commit → push → PR #335 → armed auto-merge.
11. **#335 merged** (`3e0645c`, ~20 min). Deferred wrap-up + memory seq 155. Final report to the user; `/handoff` (this file).

## Key Decisions

- **Hover highlight = square + horizontally inset by the clamped radius**, not rounded. Rejected: rounding the highlight (back to a rounded-row look, odd against the flat bg mid-list); per-corner rounding of first/last rows (impossible — `DrawRect` has one `corner_radius` for all four corners; adding per-corner radii = an SDF/shader change, out of scope for a PATCH).
- **Row labels moved to the highlight's z (+2×step)** rather than leaving them at +1×step — a label below the highlight's z would be hidden by it; the equal-z tie rule (text over surface) is the engine-sanctioned way to keep text above its own widget surface.
- **Distinct z values for bg vs highlight** instead of relying on stable-sort push order at equal z — explicit ordering, no dependence on sort implementation details.
- **Started the wasm-ship on a branch off pre-#334 main, deliberately.** Waiting for #334 would have serialized ~20 min for nothing; the known cost (with `strict=true`, a stacked branch goes BEHIND) was paid as one clean rebase BEFORE the version-bump paperwork — so the version line (the one guaranteed-conflict hunk) never conflicted. Ordering: code work → parent PR merges → rebase → /ship paperwork → verify → land.
- **WIP commit + `git reset --soft main` + single clean commit** for #335 — keeps the landed history one-coherent-change while allowing a mid-work rebase.
- **Both bumps PATCH** (v0.113.1 visual bugfix, v0.113.2 examples/scripts/docs only) per the pre-1.0 rule; neither touches public API.
- **Cargo.toml `include` list NOT extended** for `examples/game_feel/game_feel.rs` — the four existing web-shipped examples (`centered_text`, `web_audio`, `wasm_save`, `audio_facade`) are equally outside `include` and the Package dry-run gate is green on all of them; cargo strips example targets whose sources are excluded from the package.
- **game_feel web smoke = render-only model** (non-blank frame), not a boolean self-check — the game-feel behaviors are interactive with no portable cross-backend invariant to assert; the centered_text model is the exact fit. Not made a CI gate (CI has no Chrome/GPU).
- **Session stayed strictly inside the parent's sanctioned list** — the tempting adjacents (pushing the game repo's commit, PATTERNS settings-menu recipe, more widget work) were all left untouched.

## Evidence & Data

### PRs / commits this session

| PR | Merge hash | Version | Type | Landing | CI wall-clock |
|---|---|---|---|---|---|
| #333 (parent's handoff) | `898e467` | — (docs) | docs(handoff) | auto-merge, armed by parent session | merged 2026-07-02 14:46:52Z, before this session |
| #334 dropdown seam fix | `4b44534` | v0.113.1 | fix(ui) | auto-merge, **unattended** | armed → merged 15:19:33Z (~19 min) |
| #335 game_feel web ship | `3e0645c` | v0.113.2 | feat(examples) | auto-merge, **unattended** | armed → merged 15:40:02Z (~20 min) |

### Version / header / memory progression

| Point | main | package | CLAUDE.md header | memory seq |
|---|---|---|---|---|
| Session start | `731a430`* | v0.113.0 | v1.6.204 | 152 |
| After #333 wrap-up | `898e467` | v0.113.0 | v1.6.204 | 153 |
| After #334 | `4b44534` | v0.113.1 | v1.6.205 | 154 |
| After #335 (now) | `3e0645c` | v0.113.2 | v1.6.206 | 155 |

*origin had already merged #333; the local view caught up at wrap-up.

### Verify / background-task log (all exits read non-piped)

| Run | Result |
|---|---|
| Onboarding baseline (main) | `VERIFY_EXIT=0` + sentinel + pgrep empty |
| #334 branch (post-paperwork) | `VERIFY_EXIT=0` + sentinel + pgrep empty |
| #335 branch (post-rebase, post-paperwork) | `VERIFY_EXIT=0` + sentinel + pgrep empty |
| dropdown unit tests | 18 passed / 0 failed (1109 filtered) |
| game_feel native + wasm example builds | both exit 0 (wasm failed pre-refactor: E0599) |
| game_feel web smoke | PASS — 66,611 bytes ≥ 15,000 min |

### New geometry test — exact asserted values (3-item fixture, node 50,50,120×30, radius 6)

| Rect | x | y | w | h | corner_radius | z |
|---|---|---|---|---|---|---|
| list background | 50.0 | 80.0 | 120.0 | 90.0 | 6.0 | `LIST_Z + 0.001` |
| hover highlight (row 1) | 56.0 | 110.0 | 108.0 | 30.0 | 0.0 | `LIST_Z + 0.002` |

### A/B captures (scratchpad, session-local)

- After: `/private/tmp/claude-501/…/7e0c1f82…/scratchpad/ui_dropdown_seamfix.png` — continuous list bg, High row inset-highlighted, no bleed.
- Before: same dir `ui_dropdown_before.png` — notch seams at row corners + faint covered-label bleed between High/Ultra.
- Web smoke frame: `/tmp/game_feel_web_smoke.png` — arena renders (pads/dummies/player/HUD).
- Note: "Back" label visible in both A/B shots is the real button BELOW the list's bottom edge (y=250) — pre-existing, unchanged.

### Dropdown open-list z-layout — old vs new (the actual render change)

| Element | OLD z | NEW z | OLD rounding | NEW rounding |
|---|---|---|---|---|
| closed box (while open) | `LIST_Z` (90.0) | unchanged | `dd.corner_radius` | unchanged |
| per-row rect × n | `LIST_Z + 0.001` | **removed** | all 4 corners each ← the seam source | — |
| full-list background × 1 | — | `LIST_Z + 0.001` | — | `dd.corner_radius` |
| hover highlight × 0..1 | (was the row's own rect) | `LIST_Z + 0.002` | — | **0.0** (square, inset) |
| row labels × n | `LIST_Z + 0.001` | `LIST_Z + 0.002` | text-over-surface on tie | same rule, now vs the highlight |

Rect count while open: `1 + n` → `2` (bg + box) or `3` (hovered). `corner_radius: 0.0` → inset 0, full-width highlight — visually identical to the old render.

### Branch / commit mapping

| Branch | Final commit subject | PR |
|---|---|---|
| `fix/dropdown-list-seam` (off `898e467`) | `fix(ui): dropdown open list — one rounded background + inset hover highlight, no row seams (v0.113.1)` | #334 |
| `feat/game-feel-web` (off `898e467`, rebased onto `4b44534` mid-work) | `feat(examples): game_feel web ship — wasm-bindgen harness + render smoke (v0.113.2)` | #335 |

Both branches deleted locally after merge; remote heads auto-deleted (`delete_branch_on_merge`).

### Async-landing checkpoint mechanics (as actually run — reusable pacing)

- While a local verify ran (~6-8 min): `ScheduleWakeup` at 240-270s (inside the 5-min prompt-cache window), re-checking the log's `[verify]` progress lines + the `VERIFY_EXIT=` sentinel + `pgrep verify.sh`.
- While a PR's CI ran (GitHub-side, no harness notification): one `ScheduleWakeup` at 600s (~= CI wall-clock + margin), then a single `gh pr view <n> --json state,mergedAt,mergeCommit,mergeStateStatus` probe.
- Probe semantics observed: `state=MERGED` + `mergedAt` are authoritative; `mergeStateStatus` reads `BLOCKED` while checks run and `UNKNOWN` right after merge — don't gate on it.
- Between arming and merging, real work proceeded (the #335 code was written while #334's CI ran) — the async mode's whole point.

### Board / game-repo state (checked at onboarding)

| Fact | Value |
|---|---|
| `## Active requests` | `_(none open … Next free ID: EW-004.)_` — 6th consecutive empty session |
| `[Engine] 2026-07-02` notice | present on the board (from seq 1) |
| Game repo | `main...origin/main [ahead 1]` — `19d43a8` still unpushed |
| Engine pin | `rev = "1c19873…"` = v0.62.0, unchanged |

## Code Analysis

- **New dropdown open-list render** (`src/ui/system/dropdown_pass.rs` ~L143-192): compute `list_pos`/`item_h`/`list_h` once → push bg rect (rounded, `LIST_Z+step`) → if `hovered_row`, push inset highlight (`LIST_Z+2step`) → per-row texts at `LIST_Z+2step`. `UI_SUBLAYER_Z_STEP = 0.001` (`src/ui/system.rs:26`); `DROPDOWN_LIST_Z = 90.0` so even `+2step` stays far under `TOOLTIP_Z = 100`.
- **Inset clamp:** `dd.corner_radius.clamp(0.0, size.x.min(list_h) / 2.0)` — mirrors the shader's own "clamped to half the smaller side" rule so the inset can never exceed the effective rendered radius; `w = size.x - 2*inset` degrades to 0 (draws nothing) at the pathological radius = half-width.
- **`game_feel.rs` entry shape:** `fn build_app() -> App` (all setup incl. `let headless = std::env::var("HEADLESS_SHOT").is_ok()` for the `auto` field — errs on wasm → false); `#[cfg(not(wasm32))] main()` holds the HEADLESS_SHOT screenshot branch; `#[cfg(wasm32)] run_game_feel() { build_app().run(); }` + empty `#[cfg(wasm32)] main()`; `#[cfg(wasm32)] use engine::wasm_bindgen;` at top.
- **UiQueue test-side accumulation:** `UiSystem` pushes into `UiQueue` every `run()`; only frame.rs drains. Any test counting queued rects across >1 simulated frame must clear `.items` in between (memory-logged, seq 154).
- **`gh pr view <n> --json state,mergedAt,mergeCommit`** proved the reliable async-mode checkpoint probe; `mergeStateStatus` reads `UNKNOWN` right after merge and `BLOCKED` while checks run — state+mergedAt are the authoritative fields.
- **Smoke port map now in use:** 8085 centered_text, 8087 game_feel (avoids collision when smokes run back-to-back).

## Files Changed

### PR #334 (merged `4b44534`, v0.113.1)
- `src/ui/system/dropdown_pass.rs` — open-list render replaced (bg + inset highlight + relayered text); doc comment tweak ("its list draws over"); 1 test updated + 1 added.
- `Cargo.toml` / `Cargo.lock` / `docs/CHANGELOG.md` (`## 0.113.1`) / `CLAUDE.md` (header v1.6.205) — /ship paperwork.

### PR #335 (merged `3e0645c`, v0.113.2)
- `examples/game_feel.rs` → `examples/game_feel/game_feel.rs` — moved; `build_app()` refactor + `run_game_feel()` wasm entry + web doc-comment.
- `examples/game_feel/web/build.sh` + `web/index.html` — NEW (centered_text template).
- `scripts/game_feel_web_smoke.sh` — NEW render-only smoke.
- `Cargo.toml` — `[[example]] game_feel` entry + version 0.113.2; `Cargo.lock`; `docs/CHANGELOG.md` (`## 0.113.2`); `docs/WASM_SMOKES.md` (game_feel bullet); `CLAUDE.md` (header v1.6.206 + UI-row web-ship note).

### This handoff PR
- `plans/handoffs/HANDOFF_dm-adoption_seamfix-gamefeel-web_2026-07-03.md` — this file.

### Memory (outside repo)
- `engine-current-state.md` — seqs 153/154/155 bumped (154 carries the UiQueue test gotcha; 155 carries the cfg-gate lesson).
- `MEMORY.md` — engine-current-state index line kept in sync at each bump.

## User Feedback & Preferences (REQUIRED)

- **Onboarding paste-prompt protocol, 4th session running** — the user's prompt again demanded narrated onboarding (summary → verify plan → key files + adjacents → planned first action → WAIT). The adjacent-file requirement paid off again: the `DrawRect` no-per-corner-radii check settled the fix shape before any code was written.
- **"진행해"** — a single terse go-ahead approved the full two-part plan (wrap-up + seam fix, with the wasm-ship as the stated "여유 시" follow-on). After that, **zero mid-session user intervention**: 2 PRs + 3 memory bumps + all wrap-ups ran unattended. This is the first fully-autonomous multi-PR run in the chain — the combination of a scoped plan, the sanctioned-list discipline, and async auto-merge is what made it possible.
- **/handoff invoked explicitly at session end** — the user closes sessions through the skill, not freeform summaries.
- **Standing (unchanged):** user-facing Korean / repo artifacts English; merge authority delegated (squash on green, direct instruction); never push either repo's main directly; `cargo fmt` before verify; gate exits read non-piped (zsh `$pipestatus` is 1-indexed — never `${PIPESTATUS[0]}`); pre-1.0 PATCH/MINOR rule; explicit `model:` on every subagent (none were needed this session); board FIRST every session.

## Where We're Going

1. **Land this handoff** as its own `docs(handoff)` PR via async auto-merge; bump memory to **seq 156** when it merges (handoff-mode rule: the bump belongs to this PR's landing).
2. **The game session is still the blocking dependency.** The user hasn't pasted the starter prompt into a `../dungeon-merchant` session yet (parent Evidence § has the full text). Until it runs: no pushed `19d43a8`, no pin bump, no EW-004+.
3. **Next engine session: board FIRST, as always.** If EW-004+ items exist → serve priority-order via `/add-feature-example` / `/add-ui-widget`, land async.
4. **If the board is STILL empty: ASK the user for direction.** The sanctioned self-pick list is now EXHAUSTED (seam fix ✓ #334, game_feel wasm-ship ✓ #335 — both were the parent's only two). Do NOT invent work; the honest options to present are (a) remind the user to run the game session, (b) wait, or (c) a user-chosen new direction.
5. **Async-mode discipline: two clean unattended landings** — the deferred wrap-up batching worked at both checkpoints. No land-pr changes needed; keep applying the sentinel + `pgrep` check on every background completion.

## Risks & Blockers

- **`19d43a8` still unpushed in the game repo** (carried from seq 1, now one session older). Mitigation unchanged: the starter prompt tells the game session to push it; the notice text is reproduced in the parent's Evidence §.
- **0.62→0.113(.2) is now 51+ minors of pre-1.0 breaking changes** for the eventual game-side bump — the CHANGELOG (now with 0.113.1/0.113.2 entries) remains the migration guide.
- **Self-pick queue empty** — the next empty-board session has no pre-authorized work; treating anything as sanctioned would violate the parent's "do not invent" rule.
- **Stacked-PR BEHIND risk under `strict=true`** — real but managed: this session's rebase-before-paperwork ordering avoided both the CI re-run and the version-line conflict; keep that ordering for serial multi-PR sessions.

## Open Questions

- **Does the game session bump straight to v0.113.2 or in stages?** (carried from seq 1 — unanswerable until it runs.)
- **Grid drag-drop as a future EW?** (carried from seq 1 — decide when/if filed.)
- None new from this session.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4          # tip = docs(handoff) dm-adoption seq 2 over 3e0645c (#335)
git status -s                 # clean

# Board FIRST — EW-004+ items may exist IF the game session finally ran.
sed -n '50,80p' ../dungeon-merchant/docs/engine-wishlist.md
git -C ../dungeon-merchant log --oneline -3   # did it push 19d43a8? bump the pin?
grep -n 'rev = ' ../dungeon-merchant/Cargo.toml

# Verify baseline (7 gates; read exit non-piped; 2 audio tests may fail on a no-device box)
cargo fmt && ./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?

# Key files from this session
#   src/ui/system/dropdown_pass.rs        — new open-list render (bg + inset highlight)
#   examples/game_feel/game_feel.rs       — dual-target entry (build_app/run_game_feel)
#   examples/game_feel/web/               — wasm harness; scripts/game_feel_web_smoke.sh
#   plans/handoffs/HANDOFF_dm-adoption_board-notice-automerge_2026-07-02.md
#     (parent — Evidence § holds the game-session starter prompt for re-issue)

# Next action
#   Board has EW-004+ → serve priority-order (/add-feature-example or /add-ui-widget), land async.
#   Board STILL empty → ASK the user (self-picks EXHAUSTED — seam fix #334 ✓, wasm-ship #335 ✓;
#   options: run the game session / wait / user-chosen direction). Do NOT invent work.
```

## Session Closed

**Closed at:** 2026-07-03
**Commit:** landed via this docs(handoff) PR (async auto-merge — the chain's 3rd unattended landing); engine tip at close = `3e0645c` #335. Memory seq **156** belongs to this PR's merge — the next session's deferred wrap-up, exactly as this session did seq 153 for the parent's PR.
**Session status:** Handed off — next engine session reads the board FIRST; the game session (parent's starter prompt) remains the arc's blocking dependency.
