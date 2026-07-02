# Dungeon-Merchant adoption arc opened — board [Engine] notice + auto-merge landing infra + wrap trio applied

**Date:** 2026-07-02
**Status:** COMPLETED (scoped session: wrap trio applied + #332 merged + auto-merge enabled + DM scouted + board notice committed + game-session prompt written; session closing)
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `dm-adoption` seq `1`
**Parent:** `HANDOFF_game-feel-ui-breadth_capstone-trim-playtest_2026-07-02.md` (cross-chain pivot parent — the `game-feel-ui-breadth` chain is CLOSED; this session executed its "Where We're Going" §1–§3 and opened the new arc it pointed at)
**Prior chain:** `HANDOFF_game-feel-ui-breadth_2026-07-02.md` > `HANDOFF_game-feel-ui-breadth_widget-suite-text-z_2026-07-02.md` > `HANDOFF_game-feel-ui-breadth_capstone-trim-playtest_2026-07-02.md` > this

---

## Since Last Handoff

- **Parent §1 (board FIRST) executed:** `../dungeon-merchant/docs/engine-wishlist.md` still ACTIVE EMPTY — **fifth consecutive session** (next free ID EW-004). Per parent §2, did NOT invent work; asked the user for direction.
- **Parent §3 (the "natural next arc") is what the user picked:** hand the widget suite + capstone recipe to the Dungeon-Merchant side so real EW requests flow. This session ran its Phase 0 (scout) + Phase 1 (board notice); Phase 2 (game-side adoption) is handed to a game-repo session via a starter prompt (in Evidence below).
- **Parent's minor-leftover list moved:** the Dropdown corner-radius row-seam nit (on record since seq 143) got its **root cause identified during onboarding** (each open-list row is its own `DrawRect` with 4-corner rounding — `src/ui/system/dropdown_pass.rs:166`); still unfixed, now a well-scoped small PR whenever picked up. game_feel wasm-ship and the PATTERNS settings-menu recipe remain untouched (the capstone example still serves as the recipe).
- **Off-parent-plan work happened first (same session, before onboarding):** the /wrap analysis of the parent session's 9 commits produced 3 proposals — ALL applied on user "3건 모두 추가": new local skill `add-ui-widget`, plus two `docs/PATTERNS.md` additions landed as **PR #332** (main @ `731a430`, memory seq 152).
- **Process infrastructure changed (user pain point):** CI babysitting eliminated — GitHub **auto-merge enabled** on the repo + "Render tests (lavapipe)" added as a 5th required check + `land-pr` gained an **Async mode** (arm `--auto` and move on). This session's own handoff PR is the second exercise of it.
- **Parent's spurious-notification gotcha did NOT recur** — but its detection protocol was applied on every background completion (sentinel line + `pgrep`), and a NEW related race was found and documented: `gh pr checks --watch` started immediately after PR creation exits 1 with "no checks reported" (checks register with a lag).

## Reference Documents

- `CLAUDE.md` — module map + verify gates; header **v1.6.204**, package **v0.113.0** (unchanged this session — no engine code shipped).
- `docs/PATTERNS.md` — gained two sections this session (#332): engine-emitted events need `register_event::<E>()` opt-in; "Mid-frame GPU upload rules" (upload-once + range-draw / per-frame renderer pooling).
- `.claude/proposals/2026-07-02.md` — the wrap analysis; all 3 candidates marked applied.
- `.claude/skills/add-ui-widget/SKILL.md` — NEW local skill (10-file widget wiring); `.claude/skills/land-pr/SKILL.md` — Async mode added.
- `../dungeon-merchant/docs/engine-wishlist.md` — the shared board; now carries the `[Engine] 2026-07-02` notice (committed game-side as `19d43a8`, **not pushed**).
- `../dungeon-merchant/CLAUDE.md` — game-side rules that bound Phase 2: §3 pin-bump protocol, §4 work-around-first, completion bar = `cargo build` 0 warnings + `cargo test` (clippy/fmt NOT gates).
- Memory: `engine-current-state.md` (seq 152 → 153 with this handoff's landing), `local-tooling-skills.md` (add-ui-widget added; land-pr async-mode update), `MEMORY.md` (index lines updated).

## The Goal

Open the **Dungeon-Merchant adoption arc**: the engine's self-pick backlog is exhausted (parent chain closed with the capstone), the shared board has been empty for five sessions, and the downstream game — the engine's only real consumer — is pinned 51 minor versions back (v0.62.0 vs v0.113.0) using **zero** engine widgets across ~6,700 lines of hand-rolled UI. The arc's purpose is to make real demand flow onto the board: advertise what shipped since the pin (widget suite, game-feel toolkit, text z-ordering), get the game session to bump the pin and adopt where it pays, and have every gap it hits filed as EW-004+ requests the engine session then serves. Secondary goal completed en route: stop paying ~6 min of CI babysitting per landed PR (GitHub auto-merge + required-check hardening + land-pr Async mode).

## Where We Are

- **main @ `731a430`** (package v0.113.0, header v1.6.204), tree clean, baseline verify **all 7 gates exit 0** (sentinel-confirmed), CI green.
- **PR #332 merged** (this session): `docs(patterns): mid-frame GPU upload rules + engine-event register_event note` — docs-only, +28 lines to `docs/PATTERNS.md`, no version bump. CI 5/5.
- **`.claude/proposals/2026-07-02.md` written and fully applied** — candidate 1 → skill, candidates 2+3 → #332; footer annotated with the applied state so a future /wrap won't re-propose.
- **NEW local skill `add-ui-widget`** (`.claude/skills/add-ui-widget/SKILL.md`): the 10-file widget wiring signature (component → pass → system.rs slot → UiEvent → focus → capture → re-exports → registrations → example → paperwork), the interaction-model decision (click-select w/ drag-off-cancel vs press-open w/ `#[serde(skip)]` transient state), and 6 playtest-lesson guardrails (register_event, dual-modality invariants, HUD clearance, press-away-before-release-cleanup ordering, test-fixture z vs Panel bg 0.89, label z).
- **Repo merge automation live:** `allow_auto_merge=true`, `delete_branch_on_merge=true`; branch protection on `main` now requires **5** checks (was 4 — "Render tests (lavapipe)" added, closing the auto-merge-past-a-red-render-test hole), `strict=true`, `enforce_admins=true`.
- **`land-pr` skill updated:** new "Async mode (DEFAULT for CI-verifiable changes)" section — arm `gh pr merge <n> --auto --squash` after PR creation, move on immediately; deferred wrap-up (pull main + delete local branch + memory bump) batches at the next checkpoint; `BEHIND` → `gh pr update-branch` (auto-merge stays armed); judgment-gated PRs (OS-gated/hardware/playtest) keep watch mode; stale hardcoded Co-Authored-By model line replaced with "use the harness-specified lines"; checks-registration-lag note added to watch mode.
- **DM scout complete** (read-only, Sonnet subagent): game @ `7d46b6f` clean; engine pin `rev 1c19873` = **v0.62.0** (git file:// dep, deliberate freeze, bump-on-request protocol in game CLAUDE.md §3); **zero engine widget usage** (`UiNode`/`Label`/`Slider`/`CheckBox`/`ProgressBar`/`Tooltip`/`Dropdown`/`RadioGroup`/`TabBar`/`ScrollView`/`TextInput`/`UiEvent` all 0 hits; `Panel`×3 + `Button`×1 are comments only); UI = raw `DrawRect`(86)/`DrawText`(76) + own `src/layout.rs::Rect::contains` hit-testing, ~6,700 lines over 13 files.
- **Direct adoption targets identified in game code:** `src/pause.rs` (159 lines — hand-rolled 3-button ESC modal ≈ engine `Button`+focus+`UiEvent`), `src/popups.rs` (55 lines — rise-and-fade gold text ≈ engine `FloatingText`), `src/sell_ui.rs` haggle drag-bar (≈ `Slider`), station screens on roadmap Layers 0–8 (≈ `TabBar`/`ScrollView`/`Dropdown`).
- **Board notice committed game-side:** `19d43a8` adds an `## Engine → Game notices` section ( `[Engine] 2026-07-02` — what shipped since the pin, recipes `game_feel.rs`/`ui_tabs.rs`, likely per-file fits, ask = bump when City-hub UI work makes it worthwhile + file EW-004+). **Committed, NOT pushed** — the game repo has a GitHub remote (`ChunSam/dungeon-merchant`) and the game session owns pushes; all 8 prior board commits were made by the active session with the game repo's identity (engine session had never committed there — this is the first, following the "both sides edit" contract).
- **Game-session starter prompt written** (full text in Evidence) — mirrors the user's proven onboarding-narration opener; scopes the session to pin bump → one small adoption → EW filing; wait-for-go-ahead.
- **Dropdown row-seam nit root-caused** (not fixed): open-list rows each drawn as an individual `DrawRect` with `.with_corner_radius(dd.corner_radius)` (`src/ui/system/dropdown_pass.rs:166-167`) — stacked full-rounded rects leave notch seams. Fix shape: one rounded list-background rect + square (or inset) per-row hover highlight.
- **Memory:** seq 152 recorded (#332 + skill + auto-merge); seq 153 to be recorded when THIS handoff's PR merges (the land-pr handoff-mode rule: the bump belongs to the handoff PR's landing).
- Engine code untouched this session — no version bump anywhere; both PRs (#332, this handoff) are docs-only.

## What We Tried (Chronological)

1. **/wrap analysis (session start).** `git log --since="12 hours ago"` = 9 commits (`6379053`→`ac0e5a7`), patch ~7,084 lines → delegated full-patch reading to a Sonnet subagent (159K tokens, 36 tool uses). Report confirmed: the widget 10-file pattern is repeated **line-for-line** across Dropdown/RadioGroup/TabBar (5× counting ProgressBar/Tooltip); text z-ordering introduced two genuinely new wgpu shapes; #324/#325 fixes reveal 4 recurring mistake classes. Cross-checked no prior widget-skill proposal existed (grep over `.claude/proposals/*.md` — 9 prior files, zero hits).
2. **Wrote `.claude/proposals/2026-07-02.md`** (3 candidates + observations-only section, mirroring the 2026-06-29 proposal format). User: **"3건 모두 추가"**.
3. **Applied candidate 1** — created `.claude/skills/add-ui-widget/SKILL.md`; per [[record-skills-in-memory]] updated `local-tooling-skills.md` (5→6 skills) + `MEMORY.md` index the same turn.
4. **Applied candidates 2+3** — two Edits to `docs/PATTERNS.md`: a note under "Add a new event" (engine-emitted `UiEvent`/`AnimationEvent`/`ZoneEvent` still need `register_event::<E>()`; unregistered bus silently drops — the #324 lesson), and a new "### Mid-frame GPU upload rules" section after the pipeline-cache pattern (`write_buffer` executes at submit → upload-once + draw-by-range via `prepare_ui_primitives`/`render_ui_primitive_range`; glyphon per-frame renderer pooling via `FormatPool`+`used` reset in `end_frame()`; pointer to `interleave_runs`).
5. **Landed #332 via land-pr (watch mode — async didn't exist yet).** Verify `VERIFY_EXIT=0`; PR #332; **hit a race:** `gh pr checks 332 --watch` launched immediately after creation exited 1 "no checks reported" + `mergeStateStatus UNKNOWN` → re-ran with a wait-for-checks loop (`grep -q "no checks reported"` retry, then watch) → `CHECKS_EXIT=0`, 5/5, CLEAN, squash-merged → main `731a430`; memory seq 152 bumped in both files (different anchor phrasings, per parent's warning — both worked).
6. **User pain point:** "매번 ci 대기하고 머지 하는데 시간을 너무 많이 잡아 먹는데, 클라우드 세션으로 loop 돌려서 주기적으로 머지 시킬 수 없을까?" → investigated instead of implementing the literal ask: `gh api .../branches/main/protection` showed protection ALREADY exists (4 required checks, strict, enforce_admins) but `allow_auto_merge=false` and **"Render tests (lavapipe)" missing from required contexts**. Compared 3 options (GitHub auto-merge / cloud /schedule routine / local /loop) — cloud routine rejected: burns tokens per tick, cannot touch local memory or pull local main, duplicates what auto-merge does free.
7. **AskUserQuestion:** user picked "GitHub auto-merge (Recommended)" + "추가 (Recommended)" for the render check. Applied: `gh api -X PATCH` repo settings (`allow_auto_merge`, `delete_branch_on_merge` both true), PATCH `required_status_checks` with the 5-check JSON (`--input` file — nested `checks[]` objects need a JSON body, not `-f` flags). Updated land-pr skill (async mode section + 2 fixes) + memory.
8. **Onboarding (paste-prompt protocol) from the parent handoff.** Board still EMPTY; git tip `731a430` matched this session's own prior work; baseline verify launched in background and later **sentinel-confirmed** (`VERIFY_EXIT=0` + `[verify] all checks passed ✓` + `pgrep` empty — the parent's spurious-notification protocol, applied even though the notification was genuine this time). Read key files (`game_feel.rs` 576 lines, `node.rs:139`, `ui_tabs.rs`) + 3 adjacent (dropdown_pass.rs → seam root cause; `centered_text/web/build.sh` → wasm harness template confirmed reusable; board header/rules).
9. **Presented 4 candidates with a recommendation** (seam fix) per the parent's "ask, don't invent" rule. User instead asked for the plan of option 1 (DM arc) → presented a 4-phase plan with one decision point (who runs Phase 2 — recommended keeping the two-session discipline so work-around-first keeps generating honest engine requirements).
10. **User scoped the session:** "phase 0-1 까지 진행하고 게임 세션에 전달 할 프롬프트 작성후 핸드오프하고 종료".
11. **Phase 0 scout** (Sonnet subagent, 99K tokens, 23 tool uses, read-only): full report in Evidence — pin/version, CLAUDE.md rules, the 13-file hand-rolled UI inventory, board commit archaeology (8 commits, all game-repo-authored — engine had never committed there), roadmap Layers 0–8 implying ever more station screens.
12. **Phase 1 board notice:** read the exact insertion point (`Active requests` at line 53, placeholder at 55, `Done / archive` at 57), inserted the `## Engine → Game notices` section between them (8 lines), committed **in the game repo** as `19d43a8` with an explicit "committed by the engine session per board convention; not pushed" note in the commit body. Push deliberately left to the game session (remote exists; direct-push-to-main is the game repo's own norm to apply, not this session's).
13. **Wrote the game-session starter prompt** (Evidence below) and this handoff; landing it via the new async auto-merge mode is the session's final act.

## Key Decisions

- **Cloud merge-loop rejected in favor of GitHub auto-merge** — the user's literal ask was a scheduled cloud session; the investigation showed native auto-merge does the same job at zero token cost with instant merge-on-green, while a cloud routine can't do the local halves anyway (memory seq bump, `git pull`). Decision confirmed via AskUserQuestion rather than assumed.
- **"Render tests (lavapipe)" promoted to a required check** — pre-existing hole: 4/5 checks required meant auto-merge (or a hasty manual merge) could land a red render test. Now merge-blocking for everyone including admins (`enforce_admins=true` was already set).
- **Async landing is the land-pr DEFAULT, but judgment-gated PRs are excluded** — OS-gated/hardware/playtest changes keep watch-and-confirm; the gate decision moved to arm-time. Local verify remains the backstop since auto-merge removes the last human look at CI.
- **`add-ui-widget` created now, used later** — the widget suite is COMPLETE (parent), so the skill encodes the 5×-repeated pattern while fresh; its trigger is the next widget request (likely from the DM board), not invented work today.
- **Board notice placed as a new `## Engine → Game notices` section** (between Active requests and Done) rather than a pseudo-request or a rewrite of the placeholder — respects the board's append-only ethos and stable-ID scheme (EW numbers stay game-filed; the engine side doesn't file EWs at itself).
- **Engine session committed to the game repo for the first time — but did NOT push.** All 8 prior board commits were authored by game-repo sessions; the contract says "both sides edit". Committing locally matches the house style (same git identity + Claude trailers); pushing crosses into the game session's operational territory (its remote, its push cadence). The commit message states this explicitly.
- **Phase 2 stays in the game session** (two-session discipline kept) — the work-around-first rule is the mechanism that produces honest, real-usage engine requirements; the engine session adopting the game's UI itself would generate self-serving EWs. The starter prompt hands over everything the game session needs.
- **Recommended the game session bump the pin FIRST, then adopt ONE small target** (popups.rs→FloatingText or pause.rs→Button) — 0.62→0.113 is 51 minors of pre-1.0 breaking changes; proving the suite pays on a 55-line file before wider adoption bounds the risk.
- **Dropdown seam nit left unfixed deliberately** — the session was scoped by the user to Phase 0-1 + handoff; the nit is now shovel-ready (root cause + fix shape recorded) for any future small-PR slot.

## Evidence & Data

### PRs / commits this session

| Repo | Ref | Type | Summary |
|---|---|---|---|
| skeleton-engine | #332 → `731a430` | docs | PATTERNS.md: register_event opt-in note + Mid-frame GPU upload rules (+28 lines) |
| skeleton-engine | (this handoff PR) | docs(handoff) | `dm-adoption` seq 1; memory seq 153 on its landing |
| dungeon-merchant | `19d43a8` (local, NOT pushed) | docs(board) | `[Engine]` notice: widget suite + game-feel toolkit available; pin bump + EW-004+ invited |

### #332 CI (watch mode, pre-async)

| Check | Result | Time |
|---|---|---|
| Build (WASM) | pass | 35s |
| Package dry-run | pass | 58s |
| Render tests (lavapipe) | pass | 1m46s |
| Rustdoc | pass | 37s |
| Test (native) | pass | 5m0s |

First watch attempt: exit 1 "no checks reported on the 'docs/patterns-gpu-upload-event-optin' branch", `mergeStateStatus=UNKNOWN` (checks-registration lag); retry loop then clean watch.

### Repo-settings changes (gh api, both idempotent PATCHes)

| Setting | Before | After |
|---|---|---|
| `allow_auto_merge` | false | **true** |
| `delete_branch_on_merge` | false | **true** |
| required status checks | 4 (WASM / native / Rustdoc / package) | **5** (+ Render tests (lavapipe)) |
| `strict` / `enforce_admins` | true / true | unchanged |

Nested `checks[]` required a JSON `--input` body: `{"strict":true,"checks":[{"context":"…","app_id":15368}×5]}` PATCHed to `repos/ChunSam/skeleton-engine/branches/main/protection/required_status_checks`.

### Wrap analysis → application map

| Proposal candidate | Disposition |
|---|---|
| 1. `add-ui-widget` skill (10-file pattern, 5× repeated) | **Created** `.claude/skills/add-ui-widget/SKILL.md`; memory-logged |
| 2. `register_event::<E>()` rule (the #324 silent-drop lesson) | **PATTERNS.md** "Add a new event" note (#332) |
| 3. wgpu mid-frame upload clobber ×2 (upload-once+range-draw / renderer pooling) | **PATTERNS.md** new section (#332) |
| Observations-only (press-open UX, modality invariants, HUD overlap, label-bleed doc practice, game_feel composition idioms, release paperwork) | absorbed into candidate 1 / already covered by existing skills |

### DM scout — engine pin & adoption gap (the arc's raison d'être)

| Fact | Value |
|---|---|
| Game HEAD | `7d46b6f` (main, clean) — last 15 commits all Layer-0 city-reflow |
| Engine dep | `Cargo.toml:8-14` git file:// dep, `rev = "1c19873…"` — deliberate freeze comment |
| Resolved version | **v0.62.0** (`Cargo.lock:3192-3194`) = **51 minors behind** v0.113.0 |
| Engine widgets used | **0** — `UiNode`/`Label`/`Slider`/`CheckBox`/`ProgressBar`/`Tooltip`/`Dropdown`/`RadioGroup`/`TabBar`/`ScrollView`/`TextInput`/`UiEvent` all zero hits; `Panel`×3 `Button`×1 comments only |
| Raw-primitive usage | `DrawRect` 86 · `DrawText` 76 |
| Hit-testing | hand-rolled `src/layout.rs::Rect::contains` (179 lines), used by every screen |
| TODO/FIXME/HACK markers | none — workarounds are written as primary implementations |
| Board history | 8 commits, all `ChunSam` + Claude trailers, all from game-repo sessions; EW-001 (v0.43.6) / EW-002 (v0.61.0) / EW-003 (v0.62.0) all Verified-closed |
| Game completion bar | `cargo build` 0 warnings + `cargo test` (clippy/fmt NOT gates) — differs from engine's 7-gate verify |

### DM hand-rolled UI inventory (~6,700 lines / 13 files — adoption target map)

| File | Lines | Reimplements | Engine counterpart (post-pin) |
|---|---|---|---|
| `src/phases.rs` | 1933 | phase dispatch + debug HUD + settle/city screens | (dispatch stays game-side) |
| `src/sell_ui.rs` | 1126 | customer queue, click-to-offer, **haggle drag-bar** | `Slider` |
| `src/fight_ui.rs` | 651 | click-bag-item turn loop | grid stays custom |
| `src/loot_ui.rs` | 439 | dual-grid drag-drop | grid stays custom |
| `src/home_ui.rs` | 408 | bag↔storage transfer | grid stays custom |
| `src/inventory_ui.rs` | 397 | pack-phase grid + manual hit-test | grid stays custom |
| `src/run.rs` / `src/scenes.rs` | 391/195 | state/scene wiring | — |
| `src/shop_ui.rs` | 297 | catalog rows, click-row buy | `ScrollView` rows |
| `src/craft_ui.rs` | 295 | recipe-row list | `ScrollView` rows |
| `src/balance_ui.rs` | 214 | F3 stats overlay | `Label`s / stays |
| `src/layout.rs` | 179 | Rect + contains (hit-test lib) | `UiNode` + capture |
| `src/pause.rs` | 159 | **3-button ESC modal** | `Button` + focus + `UiEvent` ← **first-adoption pick #2** |
| `src/popups.rs` | 55 | **gold rise-and-fade text** | `FloatingText` ← **first-adoption pick #1** |

### The board notice (as committed, `19d43a8` — 8 insertions)

Section `## Engine → Game notices`, one dated `- [Engine] 2026-07-02` entry: widget-suite list (focus nav / pointer capture / Slider / CheckBox / ProgressBar / Tooltip / Dropdown / RadioGroup / TabBar / corner-radius SDF / `DrawText::with_z`), game-feel toolkit list (`FloatingText` "cf. your `src/popups.rs`" / HitFlash / SpriteTrail / InputBuffer / TimeScale·RealDt / shake), recipes (`examples/game_feel.rs`, `examples/ui_tabs.rs`), likely per-file fits (pause.rs / popups.rs / station screens / haggle bar), ask (bump per game CLAUDE.md §3 when City-hub UI work warrants; CHANGELOG as migration guide; file EW-004+; work-around-first unchanged; engine prioritizes board over self-picks).

### Game-session starter prompt (the Phase 2 deliverable — paste into a `../dungeon-merchant` session)

```
Read `docs/engine-wishlist.md` — there is a new "[Engine] 2026-07-02" notice under
"## Engine → Game notices": the engine's full UI widget suite + game-feel toolkit shipped
after our v0.62.0 pin (engine main is now v0.113.0). This session's mission: decide and
execute the engine pin bump, prove the widget suite pays on ONE small adoption, and file
EW-004+ requests for every real gap you hit.

Before starting work, narrate your onboarding:
1. Read the board notice + CLAUDE.md §3 (pin-bump protocol) + §4 (work-around-first — NEVER
   edit ../skeleton-engine; gaps become EW requests on the board) and summarize what you
   understand.
2. State what you'll verify first (git log/status — note docs/engine-wishlist.md has an
   unpushed engine-session commit 19d43a8, push it with your next push; cargo build
   0-warnings + cargo test baseline — the repo's completion bar).
3. Read the key files: src/pause.rs (hand-rolled 3-button ESC modal), src/popups.rs
   (hand-rolled floating gold text), src/layout.rs (the shared Rect hit-test helper), and
   the engine recipes ../skeleton-engine/examples/game_feel.rs + examples/ui_tabs.rs. Then
   explore 2-3 adjacent files not listed.
4. Explain your planned first action and why. Recommended shape (adjust from what you find):
   a. Bump the pin: Cargo.toml rev → engine main HEAD (`git -C ../skeleton-engine rev-parse
      HEAD`), `cargo update -p skeleton-engine`, fix 0.62→0.113 breaking changes using
      ../skeleton-engine/docs/CHANGELOG.md as the migration guide, build+test green BEFORE
      touching any UI.
   b. ONE small adoption first: src/popups.rs → engine FloatingText (55 lines, lowest risk)
      or src/pause.rs → UiNode+Button+focus (visible payoff). Prove it pays before wider
      adoption.
   c. File EW-004+ for anything missing or awkward while adopting (append-only, absolute
      dates, priority-sorted). The engine session reads the board first every session.
Then wait for my go-ahead before executing.
```

### Merge-automation decision matrix (as presented; user picked column 1 via AskUserQuestion)

| | GitHub auto-merge (picked) | Cloud /schedule routine (user's literal ask) | Local /loop |
|---|---|---|---|
| Merge latency after green | instant (GitHub-side) | up to one cron tick | up to one wake-up |
| Token cost | zero | per tick, even with no PRs | per wake-up (cache churn) |
| Can bump local memory / pull main | no (deferred to next checkpoint) | **no** (cloud env) | yes |
| Moving parts | 2 repo settings | cloud agent + GitHub auth in cloud | session wake-ups |
| Prerequisite | branch protection w/ required checks (already existed) | /schedule + remote env | none |

AskUserQuestion #1: "CI 대기/머지 자동화를 어떤 방식으로?" → **GitHub auto-merge (Recommended)**. AskUserQuestion #2: "Render tests (lavapipe)를 required checks에 추가?" → **추가 (Recommended)**. Hole being closed by #2: with 4/5 required, auto-merge could land a red render test.

### The add-ui-widget 10-file wiring signature (repo-side record — the skill + proposal files are gitignored)

Confirmed line-for-line identical across `6379053` (Dropdown) / `cc0f5ea` (RadioGroup) / `ec8e212` (TabBar); 5× counting ProgressBar #320 / Tooltip #322:

| # | Touchpoint | Per-widget change |
|---|---|---|
| 1 | `src/ui/<widget>.rs` (new) | struct (items + `selected` + style colors), `Default`, `with_*` builders, manual `Reflect`, serde derives + `#[serde(default)]`, single-geometry-source helpers |
| 2 | `src/ui/system/<widget>_pass.rs` (new) | `pub(super) fn run(world, viewport, input, capture, output, scratch)`; test helper set `setup()`/`spawn_*()`/`click()`/`changed_events()` |
| 3 | `src/ui/system.rs` | `mod` + `<widget>_scratch: Vec<Entity>` + run() at a deliberate slot in the fixed pass order (`…CheckBox → RadioGroup → TabBar → Dropdown → Tooltip(last)`) |
| 4 | `src/ui/system/event.rs` | `UiEvent::<X>Changed(Entity, usize)` — emitted only on actual change |
| 5 | `src/ui/system/focus_pass.rs` | ←/→ clamp-step arm + `collect_focusables` line (widget = ONE focus stop) |
| 6 | `src/ui/system/capture.rs` | `extend_kind::<W>()` in `PointerCapture::rebuild` (state-dependent rect → bespoke, cf. `extend_dropdowns`) |
| 7 | `src/ui/mod.rs` + `src/lib.rs` | re-exports |
| 8 | `core_resources.rs` + `editor/component_registry.rs` | reflect/clone/serde registration + editor add/remove closures |
| 9 | `examples/ui_<widget>.rs` (flat) | playable demo + headless self-check + `register_event::<UiEvent>()` |
| 10 | `CLAUDE.md` row + version paperwork | via /ship |

Interaction models (NOT mixable): click-select (RadioGroup/TabBar/CheckBox — press+release ownership, drag-off cancels) vs press-open (Dropdown — opens on press, drag-to-select is the gesture, `#[serde(skip)]` `open`/`press_opened`, press-away checked BEFORE release cleanup).

### Mistake classes from the parent session's fixes (what the wrap distilled; now encoded in skill + PATTERNS)

| Bug (commit) | Mistake class | Now guarded by |
|---|---|---|
| `ui_dropdown` HUD counter dead (#324 b1) | example reads `Events<E>` without `register_event::<E>()` — silent drop | PATTERNS.md note (#332) + skill checklist |
| Enter-open left a mouse-opened list open (#324 b2) | invariant implemented on pointer path only; keyboard path assumed | skill guardrail "dual-modality invariants" |
| Box opened on completed click, not press (#324 b3) | open-on-press vs open-on-click UX distinction — headless tests pass either way | skill interaction-model decision; real-mouse playtest required |
| Same-frame press+release-away left list open (#324, found during fix) | state-clearing branch ordered before press-away check | skill guardrail (ordering) |
| Flip-up dropdown under HUD lines (#325) | bottom-anchored example UI vs bottom-anchored HUD text | skill example checklist |
| RadioGroup/TabBar "covered by panel" red tests | default `UiNode` z 0.9 > Panel bg 0.89 — fixture never actually covered | skill guardrail (test-fixture z) |

### The board notice as committed (verbatim, `19d43a8` — reproduce game-side if the commit is ever lost)

```markdown
## Engine → Game notices

- [Engine] 2026-07-02 — **Widget suite + game-feel toolkit now available upstream (engine
  v0.113.0; your pin = v0.62.0).** Everything below shipped after the pin:
  - **UI widget suite:** `UiNode` keyboard/gamepad focus system (Tab/Shift+Tab, D-pad/stick
    nav, focus ring), pointer capture/occlusion (a covered widget no longer takes
    clicks/hover), `Slider`, `CheckBox`, `ProgressBar`, `Tooltip`, `Dropdown`, `RadioGroup`,
    `TabBar`, `DrawRect` corner-radius/border (SDF), and **text z-ordering**
    (`DrawText::with_z` — an overlay now hides covered labels).
  - **Game-feel toolkit:** `FloatingText` (rise-and-fade numbers — cf. your `src/popups.rs`),
    `HitFlash`, `SpriteTrail`, `InputBuffer` (buffer + coyote), `TimeScale`/`RealDt`
    (pause / hit-stop), `Camera::shake`.
  - **Recipes:** `../skeleton-engine/examples/game_feel.rs` — a pause/settings menu (TabBar +
    RadioGroup + Slider + Dropdown) driving effects live, incl. the spawn-hidden menu
    pattern; `examples/ui_tabs.rs` — the tab-container visibility wiring. Likely fits on
    your side: `src/pause.rs` (hand-rolled 3-button modal → `Button` + focus + `UiEvent`),
    `src/popups.rs` (→ `FloatingText`), the growing station screens + haggle bar (roadmap
    L0–L8 → `TabBar` / `ScrollView` / `Dropdown` / `Slider`).
  - **Ask:** when City-hub UI work makes it worthwhile, bump the pin (game CLAUDE.md §3
    protocol; expect breaking changes across 0.62→0.113 —
    `../skeleton-engine/docs/CHANGELOG.md` is the migration guide) and adopt where it pays.
    File anything missing or awkward as **EW-004+** (work-around-first rule unchanged). The
    engine side prioritizes board requests over self-picked work.
```

(Formatting note: committed as one `- [Engine] 2026-07-02 —` bullet with nested sub-bullets, 8 physical lines in the file; expanded here for readability.)

### The 4-phase DM arc plan (as presented and accepted — Phases 2-3 are the open half)

| Phase | Owner | Content | Status |
|---|---|---|---|
| 0 | engine session | read-only scout of ../dungeon-merchant (subagent) | **DONE** (findings above) |
| 1 | engine session | `[Engine]` board notice, committed game-side, not pushed | **DONE** (`19d43a8`) |
| 2 | **game session** (user pastes the starter prompt) | push 19d43a8 · pin bump 0.62→0.113 · ONE small adoption · file EW-004+ | handed off |
| 3 | next engine session | serve EW-004+ priority-order (/add-feature-example, /add-ui-widget), land async | pending board |

Decision point resolved: Phase 2 stays in the game session (two-session discipline; work-around-first generates honest requirements). The alternative — this session crossing into game code — was offered and not taken.

### Memory anchor phrasings for the seq-153 bump (they differ per file; copy exactly)

- `engine-current-state.md` tip starts: `` **main @ `731a430` — package v0.113.0, CLAUDE.md header v1.6.204, clean+green (2026-07-02; tip = seq 152 DOCS-PATTERNS #332 [ ``
- `MEMORY.md` engine-current-state line starts: `` - [Engine current state](engine-current-state.md) — main @ `731a430` (**v0.113.0**, header v1.6.204, clean+green, 2026-07-02; tip = seq 152 DOCS-PATTERNS #332 [ ``
- Bump = replace `731a430` with the handoff merge hash, `seq 152 DOCS-PATTERNS #332 […]` becomes the `prior =` tail, new `tip = seq 153 HANDOFF #<n> [dm-adoption seq 1 …]` at the front (asserted single-occurrence replace per file).

### Verify / background-task log

| Run | Result |
|---|---|
| #332 branch verify | `VERIFY_EXIT=0` (non-piped, sentinel in task output) |
| Onboarding baseline verify | `VERIFY_EXIT=0` + `[verify] all checks passed ✓` + `pgrep verify.sh` empty (spurious-notification protocol applied; notification was genuine) |
| Wrap subagent | Sonnet, 159K tokens / 36 tool uses / 254s — full 7,084-line patch read |
| DM scout subagent | Sonnet, 99K tokens / 23 tool uses / 178s — read-only |

## Code Analysis

- **Dropdown seam nit (shovel-ready):** `src/ui/system/dropdown_pass.rs:159-174` — open-list rows: `DrawRect::new(pos.x, row_y, size.x, item_h, row_color).with_corner_radius(dd.corner_radius)` per row → every row rounds all 4 corners → notch seams between stacked rows when `corner_radius > 0`. Fix shape: one full-list rounded background rect (`list_pos`, `size.x`, `item_h × n`) + per-row hover highlight without rounding (or inset by the radius); geometry helpers `flips_up`/`list_pos`/`expanded_rect` stay the single source. `DrawRect` has no per-corner radii, so "round only outer corners of first/last row" is NOT available without an SDF change — the one-background approach avoids that.
- **`examples/game_feel.rs` composition guards confirmed in code** (for the game session copying the recipe): HitFlash re-add skip at L119 (`world.get::<HitFlash>(target).is_none()`), trail apply-on-change at L223 (`trail_idx != self.trail_applied`), TimeScale single resolver at L274-283 (paused→0.0 / hitstop→0.06 / else 1.0, hitstop decremented by `RealDt` at L271), InputBuffer contract order at L307-316 (`set_grounded → press → try_consume → tick` LAST), `register_event::<UiEvent>()` at L409 with the "silently dropped" comment.
- **`UiNode::with_visible`** at `src/ui/node.rs:139`, sits beside `with_anchor`/`with_z`; `new()` defaults `visible: true` (L122) — nothing existing changes.
- **game_feel is wasm-clean by imports:** `engine::{spawn_floating_text, App, Camera, …}` — no rapier/tungstenite/GpuParticleEmitter; the headless branch (`save_screenshot_headless`) is env-var-gated so a `#[wasm_bindgen]` entry from /ship-wasm-example can coexist (harness template confirmed at `examples/centered_text/web/build.sh`: `cargo build --release --example` + `wasm-bindgen --target web --out-dir pkg`).
- **land-pr async-mode mechanics:** `gh pr merge <n> --auto --squash` (no `--delete-branch` — repo auto-deletes now); un-merged diagnosis: red check → push fix (auto-merge stays armed); `BEHIND` (strict=true + main moved) → `gh pr update-branch <n>` re-runs CI, arm survives. Serial multi-PR sessions: branch off freshly-pulled main after each merge or pay one update-branch per stacked PR.
- **DM board file structure** (for future engine-side edits): how-to at lines 16-30 (append-only threads, absolute dates, stable EW IDs), `## Active requests` line 53, placeholder line 55, new `## Engine → Game notices` after it, `## Done / archive` (3 verified EWs), `## Request template` at the bottom.

## Files Changed

### skeleton-engine repo (merged via #332)
- `docs/PATTERNS.md` — +28 lines: "Add a new event" register_event note; new "### Mid-frame GPU upload rules" section (upload-once + range-draw, per-frame renderer pooling, `interleave_runs` pointer).

### skeleton-engine repo (this handoff PR)
- `plans/handoffs/HANDOFF_dm-adoption_board-notice-automerge_2026-07-02.md` — this file.

### dungeon-merchant repo (committed `19d43a8`, NOT pushed)
- `docs/engine-wishlist.md` — +8 lines: `## Engine → Game notices` section with the `[Engine] 2026-07-02` entry.

### Local tooling (gitignored — memory is the only record)
- `.claude/skills/add-ui-widget/SKILL.md` — NEW (10-file wiring + interaction-model choice + 6 guardrails).
- `.claude/skills/land-pr/SKILL.md` — Async mode section; Co-Authored-By line un-hardcoded; checks-lag note; description updated.
- `.claude/proposals/2026-07-02.md` — NEW (the wrap analysis); footer marked all-applied.

### Memory (outside repo)
- `engine-current-state.md` — seq 152 bump (#332 + skill + auto-merge); seq 153 due on this handoff's merge.
- `local-tooling-skills.md` — add-ui-widget bullet added; land-pr bullet's async-mode update appended.
- `MEMORY.md` — engine-current-state index line → seq 152; local-tooling-skills hook line updated (6 skills).

### GitHub repo settings (not files — recorded here because nothing else records them)
- `ChunSam/skeleton-engine`: `allow_auto_merge` true, `delete_branch_on_merge` true, required checks 4→5 (+ Render tests (lavapipe)).

## User Feedback & Preferences (REQUIRED)

- **"3건 모두 추가"** — approved all three wrap proposals at once; the numbered-candidates-with-recommendation format keeps working (parent noted the same).
- **"land-pr로 올려줘"** — terse skill-invocation by name; the user tracks which skill owns which loop.
- **"매번 ci 대기하고 머지 하는데 시간을 너무 많이 잡아 먹는데, 클라우드 세션으로 loop 돌려서 주기적으로 머지 시킬 수 없을까?"** — a stated pain point WITH a proposed solution; the right response was to investigate, find the cheaper native mechanism, and present the comparison — the user accepted the alternative immediately via AskUserQuestion. Precedent: don't implement the literal ask when a better-fitting mechanism exists; do bring it as a recommended option, not a fait accompli.
- **AskUserQuestion worked well twice** (merge automation method + render-check promotion) — genuine either-way decisions that change repo settings belong to the user; both picked the Recommended option.
- **"1진행 하면 어떻게 진행할지 계획 알려줘"** — before committing to a direction the user wants the execution plan; deliver the plan and STOP (no execution). The plan's phase structure (0/1/2/3 + one explicit decision point) was accepted as-is.
- **"phase 0-1 까지 진행하고 게임 세션에 전달 할 프롬프트 작성후 핸드오프하고 종료"** — the user scopes sessions explicitly (which phases, which deliverables, then close). Respect the boundary: the dropdown nit and game-side work were NOT started despite being adjacent and tempting.
- **Onboarding paste-prompt protocol** (narrate: summary → verify plan → key files + adjacent → planned first action → WAIT) — third session running; the adjacent-file requirement paid off again (dropdown seam root cause found during onboarding, not during fix work).
- **Standing (unchanged):** user-facing Korean / repo artifacts English; merge authority delegated (squash on green, direct instruction); never push main (engine repo — and extended judgment: don't push the GAME repo's main either, that's the game session's call); `cargo fmt` before verify; gate exits read non-piped (zsh `$pipestatus` 1-indexed); pre-1.0 MINOR versioning; explicit `model:` on every subagent.

## Where We're Going

1. **Land this handoff** as its own `docs(handoff)` PR via the NEW async mode (`gh pr merge --auto --squash`), then bump memory to **seq 153** when it merges (the handoff-mode rule: the bump belongs to this PR's landing).
2. **User pastes the game-session starter prompt** (Evidence above) into a `../dungeon-merchant` session → that session pushes `19d43a8`, bumps the pin 0.62→0.113, adopts ONE small target (popups.rs→FloatingText recommended first), and files EW-004+ for every gap.
3. **Next engine session: board FIRST, as always** — but now with a live expectation: EW-004+ items should start appearing once the game session runs. Serve them priority-order via `/add-feature-example` (or `/add-ui-widget` if widget-shaped), landing async.
4. **If the board is STILL empty** next engine session (game session hasn't run yet): the shovel-ready dropdown row-seam fix (root cause + fix shape in Code Analysis) is the best small self-pick; game_feel wasm-ship (`/ship-wasm-example`) is the runner-up. Do not invent beyond those two.
5. **Watch the async-mode deferred wrap-up discipline:** this handoff PR is only the second async landing — if its deferred steps (pull main, branch delete, seq bump) feel awkward at session close, refine the land-pr skill's close-session guidance next time it's touched.

## Risks & Blockers

- **The unpushed game-repo commit `19d43a8`** — if the user starts the game session on another machine or after a `git pull` conflict, the notice could be lost/duplicated. Mitigation: the starter prompt's step 2 explicitly tells the game session to push it with its next push; the notice text is also reproduced in this handoff's Evidence.
- **0.62→0.113 is 51 minors of pre-1.0 breaking changes** — the game-side bump may be a real slog (the CHANGELOG is the only migration guide). If the game session stalls on the bump, an EW asking for a migration-notes digest is a legitimate first request.
- **Auto-merge removes the human look at CI** — required-check hardening (5/5 now) mitigates; judgment-gated PRs are excluded by skill rule. Watch that the exclusion actually gets honored in practice.
- **Spurious background-task completion notifications** (parent's discovery) — did not recur this session but the checks-registration lag race (new, documented in land-pr) is a cousin; treat every background completion as a hint, confirm via sentinel + process liveness.
- **The board contract now has a precedent of engine-side commits in the game repo** — keep it to board-file edits only, never game code (work-around-first cuts both ways).

## Open Questions

- **Does the game session bump straight to v0.113.0 or in stages?** (e.g. 0.62→0.86→0.113 at prior EW-verified points) — left to the game session's judgment after it sees the breakage surface; the starter prompt says bump-then-green before any UI work.
- **Will grid drag-drop (the game's core interaction, 4 files) ever be an engine ask?** The scout suggests it stays game-side (too genre-specific), but if the game files it as an EW the engine side should probably decline per skeleton-thin policy — decide when/if filed.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # tip = docs(handoff) dm-adoption seq 1 over 731a430 (#332)
git status -s                   # clean

# Board FIRST — EW-004+ items may now exist (the [Engine] notice invites them).
sed -n '50,80p' ../dungeon-merchant/docs/engine-wishlist.md
# Also check: did the game session run? (pin bump + pushed 19d43a8?)
git -C ../dungeon-merchant log --oneline -5
grep -n 'rev = ' ../dungeon-merchant/Cargo.toml

# Verify baseline (7 gates; read exit non-piped; 2 audio tests may fail on a no-device box)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?

# Key files from this session
#   .claude/skills/land-pr/SKILL.md        — Async mode (the new default landing path)
#   .claude/skills/add-ui-widget/SKILL.md  — use for any widget-shaped EW
#   docs/PATTERNS.md                       — new: register_event note + Mid-frame GPU upload rules
#   src/ui/system/dropdown_pass.rs:159-174 — shovel-ready seam fix (see handoff Code Analysis)
#   plans/handoffs/HANDOFF_dm-adoption_board-notice-automerge_2026-07-02.md — this file
#     (Evidence § contains the game-session starter prompt if the user needs it re-issued)

# Next action
#   Board has EW-004+ → serve them priority-order (/add-feature-example or /add-ui-widget),
#   land async. Board still empty → dropdown row-seam fix, then game_feel wasm-ship.
#   Do not invent beyond those two; the demand pipe is now the game session.
```

## Session Closed

**Closed at:** 2026-07-02
**Commit:** landed via this docs(handoff) PR — the first session-close exercised through the async auto-merge mode (engine tip at close = `731a430` #332; game-side board commit `19d43a8` unpushed, game session pushes it)
**Session status:** Handed off — engine side to the next engine session (paste prompt), game side to a `../dungeon-merchant` session (starter prompt in Evidence)
