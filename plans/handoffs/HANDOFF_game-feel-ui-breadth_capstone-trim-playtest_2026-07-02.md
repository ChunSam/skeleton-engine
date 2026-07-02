# Game-feel capstone shipped + memory trimmed + 4-example playtest all-pass — chain likely complete

**Date:** 2026-07-02
**Status:** COMPLETED (1 PR shipped + merged; memory maintenance done; playtest todo closed; tree clean, all green)
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `game-feel-ui-breadth` seq `3`
**Parent:** `HANDOFF_game-feel-ui-breadth_widget-suite-text-z_2026-07-02.md`
**Prior chain:** `HANDOFF_game-feel-ui-breadth_2026-07-02.md` > `HANDOFF_game-feel-ui-breadth_widget-suite-text-z_2026-07-02.md` > this

---

## Since Last Handoff

- **Parent's top recommendation (game-feel capstone example) is DONE** — PR #330 `examples/game_feel.rs` (v0.113.0), merged on CI 5/5. The parent's framing held exactly: the completed widget suite made a real settings menu possible, and the capstone validates the whole 0.104.0–0.112.0 game-feel + widget run composing in one playable.
- **Parent's "Where We're Going" item 3 (real-mouse playtest of the newest features) is DONE and ALL-PASS** — the user ran game_feel (11 checklist items) + ui_radio (3) + ui_tabs (3) + text_layers (2) live; zero findings. Unlike the parent session's dropdown playtest (3 real bugs), this one confirmed the shipped gesture paths — the open-on-press/drag-off/keyboard-nav lessons from #324 were already baked in.
- **Parent's "Where We're Going" item 4 (memory trim) is DONE** — `engine-current-state.md` 178.9 KB → 25.9 KB (−86%), `MEMORY.md` 17.5 KB → 5.5 KB; the trimmed detail moved to `engine-history-archive.md` under dated section headers. The per-session context load drops ~3k tokens.
- **Parent's runner-up (app.rs split) was investigated and DEPRIORITIZED** — onboarding exploration found `src/app.rs`'s 1063 lines are ~352 lines of code + ~710 lines of tests (`mod tests` starts at line 355), and the code is already split into 10 `src/app/*` submodules. The "1063-line file" framing in prior handoffs overstated it.
- **One parent open question resolved implicitly:** "capstone vs app.rs split first?" → user picked capstone ("진행해" after my recommendation); split demoted on the line-count discovery.
- **Board still ACTIVE EMPTY (EW-004)** — fourth consecutive session. With the capstone + playtest + trim done, the self-pick backlog is now genuinely exhausted (see Where We're Going).
- **New process gotcha discovered:** the harness can emit a **spurious "completed (exit 0)" notification for a background task that is still running** (verify.sh was on gate 2 of 7). Detection + workaround documented in Process Gotchas below.

## Reference Documents

- `CLAUDE.md` — module map + verify gates + conventions; header now **v1.6.204**, package **v0.113.0**; UI module-map row updated for `with_visible` + `game_feel` (kept at exactly **200 lines**).
- `docs/VISION.md` — the feature+example loop; the capstone is its explicit "compose the toolkit in a playable" milestone; the `UiNode::with_visible` addition is a textbook "fix the API when the example is awkward" case.
- `docs/CHANGELOG.md` — new 0.113.0 entry.
- Memory: `engine-current-state.md` (tip = seq 150, now trimmed + structure documented below), `MEMORY.md` (compacted index line), `engine-history-archive.md` (absorbed the trim), `merge-authority-delegated.md` (used 1×), `playtest-windowed-examples.md` (the launch-windows-and-checklist protocol, reused).
- Parent handoff — its §Quick Start key-file list was the onboarding read set and was accurate; its §Code Analysis widget-registration recipe was NOT needed this session (no new widget).

## The Goal

Close out the `game-feel-ui-breadth` chain by (1) shipping the **game-feel capstone example** the chain was building toward — one small playable that composes FloatingText + InputBuffer + SpriteTrail + HitFlash + `Camera::shake` + a `TimeScale` hit-stop, all configured live from a pause/settings menu made of the new widget suite (TabBar / RadioGroup / Slider / Dropdown); (2) paying down the two standing maintenance debts — the ~180 KB memory tip-file and the real-mouse verification gap on the newest four features. All three were parent "Where We're Going" items; all three completed this session. The engine remains a hackable, fork-friendly, genre-agnostic 2D skeleton (docs/VISION.md), now with its game-feel toolkit demonstrably composable.

## Where We Are

- **main @ `d6b83ed`** (package **v0.113.0**, CLAUDE.md header **v1.6.204**), tree clean, no open PRs, CI green.
- **PR #330 merged** (squash, delegated authority): `feat(examples): game_feel — game-feel capstone example + UiNode::with_visible (v0.113.0)`. 6 files, +604/−5.
- **`examples/game_feel.rs`** (~490 lines, flat — Cargo auto-discovers): a 900×540 training arena. Two pads with a jumpable 80 px gap (`PAD_A` x 40–700, `PAD_B` x 780–880, `FLOOR_TOP` 430); three dummies (28×44) at x 480/560/640; a 36 px player spawning at (260, 412).
- **Gameplay wiring:** ←/→ move, Space jump through `InputBuffer` (contract order `set_grounded → press → try_consume → tick` LAST), X attacks the nearest dummy in range (64, 70) — each hit fires `HitFlash::white(0.22)` + a `FloatingText` damage number + `Camera::shake` + a hit-stop; Esc pauses (TimeScale 0) into the settings menu; Q quits.
- **Settings menu drives the effects LIVE:** TabBar page *Feel* = RadioGroup shake Off/Low/High (`SHAKE_LEVELS` [0, 5, 12]), Slider hit-stop 0–150 ms (default 90), Dropdown trail Off/Subtle/Heavy; page *Movement* = Sliders move-speed 120–380 (default 240), jump-buffer 0–300 ms (default 120), coyote 0–300 ms (default 100). Both forgiveness sliders at 0 = a deliberately strict jump (the "feel how unfair" teaching moment).
- **New public API: `UiNode::with_visible(bool)`** (`src/ui/node.rs`) + unit test `with_visible_spawns_hidden` — the example's only engine change, surfaced by the VISION rule (spawn-hidden pause-menu entities previously needed a mutable temp).
- **Damage cycle** [18, 24, 31, 47]; every 4th hit is a crit (orange `Color::rgb(1.0, 0.55, 0.2)`, 24 px, `"47!"`, shake ×1.6). Numbers stagger sideways `((hits % 3) − 1) × 26` px so rapid same-dummy hits stay readable (was 16 px — first capture read "1824").
- **Headless auto-mode** (`HEADLESS_SHOT`, default 62 frames): moves right frames <42, jumps at 6, attacks at 44 + 50, opens the menu at 54 — one capture shows the settings panel beside the frozen flash/numbers/trail. A second capture at `HEADLESS_FRAMES=48` verified the live HUD (hits/last/trail/shake/changes readout).
- **CI 5/5:** Render lavapipe 1m26s · Rustdoc 41s · Test native 4m25s · Build WASM 41s · Package dry-run 1m10s. Local verify: 7 gates exit 0, twice (baseline on clean main at session start; branch run after paperwork). Notably the 2 audio tests **passed** locally this session (audio device present) — the standing `--skip` was not needed.
- **Memory trim executed** (details + numbers in Evidence): `engine-current-state.md` is now 18 lines / 25.9 KB with a 22.4 KB tip line covering seq 150→137 only; `MEMORY.md`'s index line is 1,107 chars; the archive holds two new dated sections. Frontmatter fixed (`name` was the empty string; recall `description` added). New compaction rule written into the file: bump the tip line per PR, cut the chain tail every ~10 seqs.
- **Memory seq 150** recorded (#330), then updated same-day: the real-mouse-playtest standing todo is CLOSED (ALL-PASS marker in both files).
- **Real-mouse playtest: 19/19 checklist items pass** across game_feel / ui_radio / ui_tabs / text_layers; all four windows exited 0 (no crash). game_feel's items covered coyote, buffered jump, the 4-effect juice stack, crit styling, pause-freeze, all five live settings (shake radio, hit-stop slider, trail dropdown incl. press-drag-release, speed + forgiveness sliders), and keyboard-only menu operation.
- Lib test count: 1125 → **1126** (+`with_visible_spawns_hidden`). Render tests unchanged at 12.
- **Board `../dungeon-merchant/docs/engine-wishlist.md`: ACTIVE EMPTY (EW-004)**, verified at onboarding.

## What We Tried (Chronological)

1. **Onboarding (paste-prompt protocol).** Read the parent handoff fully; verified board (EMPTY), git state (main @ `a120c75`, clean, v0.112.0), and launched the 7-gate baseline verify in background (exit 0). Read the parent's key files (dropdown.rs + dropdown_pass.rs, radio_group.rs, tab_bar.rs, capture.rs, layering.rs, frame.rs step 2.7, renderer.rs FormatPool locations) plus 3 adjacent files not in the list. **Adjacent-file discovery that changed a recommendation:** `src/app.rs` `mod tests` starts at line 355 → only ~352 code lines, already split into `src/app/{assets,core_resources,editor,egui_pass,headless,render,render_state,scenes,schedule,window}` — the "split app.rs" candidate was overbilled; demoted it in my onboarding report. Presented capstone as #1; user: **"진행해"**.
2. **Design-first pass for the example (per /add-feature-example step 1).** Read `examples/input_buffer.rs` (platformer sim pattern: kinematic integrate, grounded check, InputBuffer contract, auto/headless script), `examples/ui_tabs.rs` (per-tab visibility system — the "tab container" wiring), `examples/juice_demo.rs` lines 50–95 (hit-stop pattern: countdown in `RealDt`, `TimeScale::set`, `cam.shake(16.0, 0.5)`). Confirmed ctors: `FloatingText::colored/with_size`, `spawn_floating_text(world, pos, ft)` free fn (usable inside a `System`, unlike the `App::` helpers), `HitFlash::white(secs)`, `SpriteTrail::new(interval, lifetime).with_start_alpha`, unit-struct systems (`HitFlashSystem` etc.), `UiEvent` variants all `(Entity, payload)`, `Slider.value` pub, `world.remove_component::<T>`.
3. **Backdrop decision.** Considered `Panel` for the menu background → rejected: it's a children-layout container needing `LayoutSystem`, and **no example uses it standalone**. Chose a game-pushed `DrawRect` into the public `UiQueue` (the `text_layers`/`ui_rounded` pattern) at `MENU_Z − 0.1` with corner radius 10.
4. **Menu-overlap layout decision.** First sketch had the menu centered — but `FloatingText` draws via the **no-z always-on-top text pass**, so damage numbers over the dummies would bleed over a centered menu. Moved the menu to the LEFT (x 32–392) and the dummies/action right (x ≥ 480): numbers can never overlap the panel. (Deliberately did NOT give FloatingText a z — its HUD-like on-top behavior is the engine default and correct for gameplay; the layout dodge is the example's job.)
5. **Wrote `examples/game_feel.rs`.** Compiled clean on the first `cargo build --example game_feel`. One self-correction before compiling: an initial `std::mem::take` dance around `set_visible(world, &self.menu.feel_group, …)` was unnecessary (`world` and `self` are disjoint borrows) — simplified to direct calls.
6. **First headless capture** (62 frames): composition confirmed (menu Feel tab, radio dot on Low, hit-stop slider at 90/150, dropdown "Subtle", white mid-flash dummy, jump-arc trail ghosts) — but the two damage numbers overlapped into "1824". **Fix:** stagger multiplier 16 → 26 px. Second capture: "18 24" readable. Third capture at `HEADLESS_FRAMES=48` (pre-pause) verified the live HUD line.
7. **VISION-rule API fix.** Writing the menu spawns exposed the awkwardness: `UiNode` had `with_anchor`/`with_z` builders but no way to spawn hidden — needed a `let mut n = …; n.visible = false;` temp in a helper. Added `UiNode::with_visible(bool)` + test; example helper became a one-liner (`UiNode::new(…).with_z(MENU_Z).with_visible(false)`).
8. **Land loop (PR #330).** Branched `feat/game-feel-capstone` carrying the changes; `cargo fmt` (reflowed 2 hand-wrapped spots); **paperwork BEFORE the single full verify** (the parent's TabBar pattern): Cargo.toml 0.113.0, `cargo update -p skeleton-engine`, CHANGELOG 0.113.0, CLAUDE.md header v1.6.204 + module-map row via two single-line edits (line count stays 200).
9. **The spurious-completion incident.** Launched verify in background → a "completed (exit code 0)" task notification arrived while the log had only 5 lines (gate 2, clippy compiling) and the task output file was **empty**. Detection chain, in order: (a) `grep VERIFY_EXIT <task output>` matched nothing; (b) `tail` of the verify log ended mid-clippy; (c) `pgrep -fl "cargo|rustc"` showed `cargo test --all-targets` live; (d) `pgrep -fl verify.sh` showed the wrapper zsh (92733) AND `bash ./scripts/verify.sh` (92735) both alive. Set a `while kill -0 92735; do sleep 15; done` watcher; a SECOND notification for the SAME task id arrived later — this one real, with `VERIFY_EXIT=0` in the task output and `[verify] all checks passed ✓` at the log tail. **Lesson: a background-task completion notification is not proof — read the output file's sentinel line and check process liveness before acting; the same task id can notify twice.**
10. **PR → CI → merge.** PR #330 with a what/how-verified body; `gh pr checks 330 --watch` in background → `CHECKS_EXIT=0`, 5/5 pass; `mergeStateStatus == CLEAN`; squash-merged; `git pull --ff-only` → main @ `d6b83ed`.
11. **Memory seq 150 bump.** The two memory files turned out to have **different anchor phrasings** for the same tip (ecs: ``main @ `a120c75` — package v0.112.0, CLAUDE.md header v1.6.203, clean+green (2026-07-02…``; MEMORY.md: ``main @ `a120c75` (**v0.112.0**, header v1.6.203, …``) — one Python patch with per-file asserted single-occurrence anchors.
12. **User: "2 진행해" → memory trim.** Backed up all 3 files to the session scratchpad first. Structure inspection technique (reusable): print per-line char counts to find the giant lines, then regex-scan the tip line for `(prior|tip) = seq \d+` marker positions — they mapped seq 150 @91, 149 @1531, … 137 @19084, 136 @21939, 126 @25718, then **nothing for the remaining ~71 K chars** = a stale narrative tail (pre-0.100 era — it still said "next free ID **EW-002**"). The body's "Recent seqs" bullet list covered seq 117→76 and had stopped being updated at 117; frontmatter `name` was `""` with no `description`. The marker scan is what made the cut point obvious (the chain boundary at seq 137, not a round char count).
13. **Trim execution (one Python patch, asserted anchors, backups kept).** Tip line cut at `'; prior = seq 136'` — kept seq 150→137 (exactly the `game-feel-ui-breadth` chain) + a bridge sentence with the board/standing/gotcha pointers; body bullets (lines 10–46) moved out; both cuts appended to `engine-history-archive.md` as `## Trim 2026-07-02 — …` sections inserted before the 2026-06-20 sections; compaction rule line rewritten; frontmatter fixed. `MEMORY.md` line 14 compacted 13,197 → 1,107 chars; the archive's index line annotated. Structural verification: ecs = 18 lines, all sections intact, archive headers clean.
14. **User: "준비 됐어. 테스트 시작해" → playtest.** Launched all four example windows as background `cargo run`s; issued numbered Korean checklists (game_feel 11 / ui_radio 3 / ui_tabs 3 / text_layers 2 — full lists in Evidence). All four windows exited 0 as the user finished each. User verdict: **"모두 테스트 통과"** (all pass, no per-item findings). Closed the standing todo in both memory files (asserted replaces; the seq-150 historical mention also updated to "ALL-PASS same day").

## Key Decisions

- **Capstone is an example, not a framework.** All composition logic (pause gating, settings→effect application, tab-page visibility) lives in the example's one `Arena` system — no new engine "menu system" or "settings resource". Keeps the engine skeleton-thin; the example IS the recipe.
- **`UiNode::with_visible` was the only engine change** — deliberately minimal. Rejected: a broader `MenuGroup`/visibility-group helper (games differ too much; the 5-line `set_visible` free fn in the example is the pattern).
- **Pause = `TimeScale 0`, hit-stop = `TimeScale 0.06`, one resolver.** A single per-frame resolution (`paused → 0.0; hitstop > 0 → 0.06; else 1.0`) instead of scattered writes; hit-stop counts down in `RealDt` so it can end while scaled time is frozen (juice_demo precedent). The gameplay sim is additionally gated on `!paused` so a Space pressed while paused can't queue a buffered jump for resume.
- **Guard against HitFlash re-add mid-flash** (`world.get::<HitFlash>(target).is_none()`): re-adding replaces the component and its first-run base-color capture would snapshot the flashed white → dummy stuck bright. This is a real composition footgun worth copying into any game using HitFlash on rapid hits.
- **Trail preset applied on CHANGE only** (cached `trail_applied` index): re-adding `SpriteTrail` every frame resets its private emit timer → no ghosts ever spawn. Same class of footgun as the HitFlash one.
- **InputBuffer rebuilt only when the rounded-ms slider pair changes** — `InputBuffer::new` resets transient press/coyote state, which is fine mid-menu (game is paused) but shouldn't happen every frame.
- **Menu on the left, action on the right** — chosen so the engine-default on-top FloatingText pass can never draw over the menu (see What We Tried #4). Alternative (give FloatingText a layered z) rejected: on-top damage numbers are the right default for gameplay; the engine shouldn't special-case them for one example's layout.
- **Memory trim keeps exactly the current chain (seq 137+) in the tip line.** Cut point chosen at the chain boundary, not a round number — the working set a next session needs is "this chain + its parent handoff pointer", everything older is archive material. Codified as the new compaction rule in the file itself.
- **MEMORY.md index line compacted to a hook** (~1.1 K chars): MEMORY.md loads into EVERY session's context, so its per-char cost is the highest of any file; the seq-chain detail it duplicated lives one Read away in `engine-current-state.md`. The established `main @ \`hash\`` anchor prefix shape was preserved so the per-PR bump workflow (asserted Python replace) is unchanged.
- **Playtest run as user-driven with numbered checklists** (the parent session's proven protocol) — synthetic clicks don't reach winit on macOS, so gesture paths need a human; numbered items make findings unambiguous. This session: zero findings, which itself is signal (the #324 gesture lessons generalized).
- **`JUMP_SPEED` −480 (not input_buffer's −600):** airtime 2·480/1500 ≈ 0.64 s clears the 80 px gap at default speed (≈153 px horizontal) while keeping the jump apex (≈77 px) low enough that a mid-air X can still reach a dummy (`ATTACK_RANGE.y` = 70) — and it makes the auto-script's land-then-attack timeline fit in a 62-frame capture.
- **Window 900×540 (vs input_buffer's 760×460):** sized so the left-docked 360 px menu and the right-side action zone coexist without overlap-compromises; both headless captures show the full composition in one frame.
- **All four launched windows at once for the playtest** (not sequential): matches the parent session's protocol, lets the user self-pace, and the background tasks' exit-0 completions doubled as a no-crash check per example.

## Evidence & Data

### The session's PR

| PR | Version | Type | Summary | Tests |
|---|---|---|---|---|
| #330 | v0.113.0 | feat | `game_feel` capstone example + `UiNode::with_visible` | +1 unit (1125→1126 lib) |

Commit: `d6b83ed` — `feat(examples): game_feel — game-feel capstone example + UiNode::with_visible (v0.113.0) (#330)`; 6 files, +604/−5.

### Version / header / memory progression

| After | package | CLAUDE.md header | memory seq | main @ |
|---|---|---|---|---|
| (start) | 0.112.0 | v1.6.203 | 149 (handoff #329) | a120c75 |
| #330 | 0.113.0 | v1.6.204 | 150 | d6b83ed |
| playtest close | — | — | 150 (todo→CLOSED edit) | d6b83ed |

### Memory trim (before → after)

| File | Before | After | Δ |
|---|---|---|---|
| `engine-current-state.md` | 178,859 chars / 53 lines | 25,940 chars / 18 lines | **−86%** |
| — its tip line | 97,007 chars (chain to seq 126 + ~71 K stale pre-0.100 tail) | 22,434 chars (seq 150→137 + bridge) | −77% |
| — body "Recent seqs" bullets | seqs 117→76 (~40 K, stale since seq 117) | 1 pointer line | moved to archive |
| `MEMORY.md` | 17,523 chars (index line 13,197) | 5,504 chars (index line 1,107) | **−69%**, ~3 K tokens/session saved |
| `engine-history-archive.md` | 72,841 chars | 226,978 chars | +154 K absorbed |

Char-conservation check: 269,223 total before vs 258,422 after; the −10.8 K delta = the MEMORY.md line dedup (its seq detail duplicated the ecs tip line) + small bridge/header additions. Nothing unique was dropped; backups of all three originals in the session scratchpad.

### game_feel constants (the tuned feel numbers)

| Constant | Value | Note |
|---|---|---|
| Window | 900×540 | menu x 32–392, action x ≥ 480 |
| `PAD_A` / `PAD_B` | x 40–700 / x 780–880 | 80 px gap, jumpable |
| `GRAVITY` / `JUMP_SPEED` | 1500 / −480 | airtime ≈ 0.64 s ≈ 153 px horizontal at default speed |
| `ATTACK_RANGE` | (64, 70) | vs dummy centers at x 480/560/640 |
| Damage cycle | [18, 24, 31, 47] | idx 3 = crit: orange, 24 px, `!` suffix, shake ×1.6 |
| Number stagger | `((hits % 3) − 1) × 26` px | 16 px overlapped ("1824") |
| `SHAKE_LEVELS` | [0.0, 5.0, 12.0] | radio Off/Low/High; `cam.shake(s, 0.3)` |
| Hit-stop | slider 0–150 ms, default 90; scale 0.06 | counted down in `RealDt` |
| Trail presets | Subtle (0.055, 0.30, α 0.35) / Heavy (0.03, 0.55, α 0.70) | Off = `remove_component` |
| Forgiveness | buffer 0–300 ms (120), coyote 0–300 ms (100) | rebuilt on rounded-ms change |
| `MENU_Z` | 6.0 (backdrop 5.9) | below `DROPDOWN_LIST_Z` 90 / `TOOLTIP_Z` 100 |
| Auto-script | move <42, jump @6, attack @44+50, pause @54, 62 frames | third capture at 48 frames = live HUD |

### Verify / CI runs

| Run | Result |
|---|---|
| Baseline verify (clean main, session start) | exit **0**, all 7 gates — audio tests passed locally this time (device present) |
| Branch verify (after paperwork, single run) | `VERIFY_EXIT=0`, `[verify] all checks passed ✓` |
| PR #330 CI | 5/5 — Render lavapipe 1m26s · Rustdoc 41s · Test native 4m25s · Build WASM 41s · Package dry-run 1m10s |
| `gh pr checks --watch` | `CHECKS_EXIT=0` (read from log, non-piped) |

### Real-mouse playtest — 19/19 pass (user: "모두 테스트 통과")

| Example | Items | Coverage highlights |
|---|---|---|
| game_feel | 11/11 | move/jump feel; coyote off the gap edge; buffered pre-landing jump; 4-effect juice stack + 4th-hit crit; Esc pause full-freeze + resume; shake radio High/Off live; hit-stop slider 0↔150; trail dropdown Off/Heavy incl. press-drag-release gesture; Movement tab speed + both-forgiveness-at-0 strict jump; keyboard-only menu nav (Tab + ←/→ on radio/tabs/sliders/dropdown); Q quit |
| ui_radio | 3/3 | click-select + silent re-pick; drag-off cancel; Tab focus + ←/→ step |
| ui_tabs | 3/3 | header switching + content swap; gap clicks select nothing; focus skips inactive-tab widgets |
| text_layers | 2/2 | caption cut exactly at the overlay edge (Space raise/lower); no-z HUD always on top |

All four windows exited 0 (no crash). Zero findings — contrast with the parent session's first dropdown playtest (3 real gesture bugs), i.e. the #324 lessons (open-on-press, single-open, event-bus registration) held in new code.

### game_feel playtest checklist (reusable as the example's regression checklist)

1. ←/→ move, Space jump — base feel. 2. **Coyote:** walk off the gap edge, Space just after leaving → jump still fires. 3. **Buffer:** press Space just before landing → jump fires on touchdown. 4. **X near a dummy:** white flash + damage number + shake + hit-stop land as ONE impact; 4th hit = orange crit (`47!`, larger). 5. **Esc:** menu opens, world fully freezes (ghosts + numbers too); Esc again resumes. 6. **Feel tab, shake radio:** High → visibly stronger on X; Off → none. 7. **Hit-stop slider:** 0 → no freeze; 150 → clearly longer. 8. **Trail dropdown:** Off → no ghosts; Heavy → denser/longer; select via press-drag-release in one gesture. 9. **Movement tab:** speed slider changes movement immediately; buffer+coyote both 0 → items 2 and 3 now fail (strict jump). 10. **Keyboard-only:** Tab cycles menu focus; ←/→ operates radio/tabs/sliders/dropdown. 11. Q quits.

(ui_radio: click-select + silent re-pick / drag-off cancel / focus step. ui_tabs: header switch + content swap / gap = no-op / focus skips hidden. text_layers: Space raise-lower cuts the caption at the card edge / no-z HUD on top.)

### Headless captures (regenerate on demand — scratchpad copies are session-ephemeral)

| Capture | Command | Shows |
|---|---|---|
| paused composition | `HEADLESS_SHOT=/tmp/gf.png cargo run --example game_feel` (62 frames default) | menu Feel tab + frozen flash/numbers/trail |
| live HUD | same + `HEADLESS_FRAMES=48` | pre-pause frame: hits/last/trail/shake/changes line |

### Skill-chain used (the full loop, nested as designed)

`/add-feature-example` (design-against-example → implement → headless-verify) → `/land-pr` (branch → fmt → paperwork-before-single-verify, the parent's TabBar ordering) → `/ship` (4-file paperwork: Cargo.toml + Cargo.lock + CHANGELOG + CLAUDE.md header) → back in `/land-pr` (verify → commit → push → PR → CI watch → CLEAN check → squash-merge → main sync → memory bump). One pass, no rework; the only off-script step was the spurious-notification watcher.

### The merge commit (primary evidence — the repo's release-commit house style)

```
d6b83ed feat(examples): game_feel — game-feel capstone example + UiNode::with_visible (v0.113.0) (#330)

One small playable arena composing the whole juice toolkit — InputBuffer
(buffered + coyote jumps), SpriteTrail, HitFlash + FloatingText damage
numbers + Camera::shake + a TimeScale hit-stop on every landed hit — with
an Esc pause/settings menu built from the widget suite (TabBar, RadioGroup,
Slider, Dropdown) whose settings drive the effects live.

The example surfaced one API opening (VISION rule): UiNode::with_visible —
spawn a widget hidden (the pause-menu pattern) without post-construction
mutation. +1 unit test; no other public API change.
```

### CLAUDE.md module-map row edits (both single-line — the 200-line invariant held)

- Row 129 start: `UI (UiNode, Button, …` → `UI (UiNode [+ \`with_visible\` builder — spawn a widget hidden, e.g. pause-menu entities revealed later], Button, …`
- Row 129 end, before the scratch-buffer note: inserted `**game-feel capstone example \`game_feel\`** — one playable arena composing InputBuffer + SpriteTrail + HitFlash + FloatingText + Camera::shake + TimeScale hit-stop with an Esc pause/settings menu (TabBar/RadioGroup/Slider/Dropdown) whose settings drive the effects LIVE`

### CHANGELOG 0.113.0 lead (as shipped)

> **The game-feel capstone example — the juice toolkit and the widget suite proven to compose.** `examples/game_feel.rs` is one small playable arena that drives the engine's game-feel helpers **together**, configured live from a real pause/settings menu — the composition target the 0.107.0–0.112.0 widget run was building toward. Engine change is one tiny additive builder the example surfaced (the VISION "fix the API when the example is awkward" rule); everything else composes on the existing public API, unchanged.

### Memory-file anchor phrasings (for the next bump — they differ per file)

- `engine-current-state.md` tip: `` **main @ `d6b83ed` — package v0.113.0, CLAUDE.md header v1.6.204, clean+green (2026-07-02; tip = seq 150 GAME-FEEL-CAPSTONE #330 … ``
- `MEMORY.md` line 14: `` - [Engine current state](engine-current-state.md) — main @ `d6b83ed` (**v0.113.0**, header v1.6.204, clean+green, 2026-07-02; tip = seq 150 … ``
- Both end their STANDING section with `real-mouse playtest … ALL-PASS (2026-07-02 user-verified, todo CLOSED)`.

## Code Analysis

- **`examples/game_feel.rs` shape:** `Menu` struct (7 widget entities + `feel_group`/`move_group` vecs) + `Arena` system (player/targets/menu, `vy`, `jump: InputBuffer`, `forgiveness: (u32, u32)` cache, `trail_applied: usize` cache, `hits`, `hitstop`, `paused`, `changes`, `auto`, `frame`). Per frame: input (live or scripted) → poll widget state (`RadioGroup::selected_index`, `Slider.value`, `Dropdown::selected_index`) → apply-on-change (trail, forgiveness) → visibility (`set_visible` per group, gated `paused && active == N`) → event count (`Events<UiEvent>`, 4 variant matches) → TimeScale resolve → `!paused` sim (integrate, land, respawn, InputBuffer, attack) → backdrop `DrawRect` → HUD.
- **`Arena::attack`** collects the nearest in-range dummy immutably first, then mutates (the collect-then-`get_mut` house pattern): HitFlash guard, `spawn_floating_text`, `cam.shake`, hitstop set, `last_hit` string.
- **`UiNode::with_visible`** sits beside `with_anchor`/`with_z` in `src/ui/node.rs` (~line 139); `UiNode::new` still defaults `visible: true`, so nothing existing changes.
- **`app.rs` reality check** (the deprioritization evidence): `mod tests` at line 355 of 1063; module decls at lines 10–22 name the 10 existing submodules; the remaining code is the `App` struct + `new()` + registration plumbing + `run` entry.
- **Widget event surface used by the example:** `UiEvent::{RadioChanged, SliderChanged, DropdownChanged, TabChanged}` — all `(Entity, payload)`; the example counts them for the HUD "menu changes" readout and polls state for application (polling is the robust default; events fire only on actual change).
- **Engine systems order in the example:** `UiSystem` → `Arena` → `HitFlashSystem` → `FloatingTextSystem` → `SpriteTrailSystem` (Arena after UiSystem = same-frame widget state, the ui_tabs precedent; effect systems last is fine — worst case one-frame pickup).
- **Menu geometry** (all `menu_node(x, y, w, h)` = `UiNode::new(…).with_z(MENU_Z).with_visible(false)`): backdrop DrawRect (32, 64, 360, 420) r10; TabBar (52, 84, 320, 30); Feel page — label (52, 130) / RadioGroup (52, 154, 240, 78) / label (52, 250) / hit-stop Slider (52, 276, 320, 20) / label (52, 318) / trail Dropdown (52, 342, 220, 30); Movement page — speed (52, 156) / buffer (52, 226) / coyote (52, 296) Sliders + labels above each + the "Both at 0 = a strict, unfair jump" hint label (52, 340). The dropdown's open list (3 × 30) ends at y 462 — inside the backdrop (bottom 484), opens downward, never flips. HUD text stays clear of the menu rect: title y 20 / status y 46 (above menu top 64), hint y 514 (below menu bottom 484).
- **Auto-script math** (why those frame numbers): airtime = 2·|JUMP_SPEED|/GRAVITY = 2·480/1500 ≈ 0.64 s ≈ 38 frames → jump @6 lands ≈ @44; x(42 frames · 240 px/s) = 260 + 168 = 428 → dx to the first dummy (480) = 52 ≤ 64 range, so attacks @44 + @50 both connect; hit-stop (90 ms @ scale 0.06) freezes frames ~44–49 so both numbers are still fresh at the frame-54 pause and the frame-62 capture.
- **UiSystem pass order re-confirmed at onboarding** (matches docs): focus → button → text_input → scroll_view → label → progress_bar → slider → checkbox → radio_group → tab_bar → dropdown → tooltip (last, so tooltip text overdraws).
- **Widget startup defaults align with caches:** radio `with_selected(1)` (Low) matches `SHAKE_LEVELS[1]`; dropdown `with_selected(1)` (Subtle) matches `trail_applied: 1` and the `SpriteTrail::new(0.055, 0.30)` attached at spawn; forgiveness cache `(120, 100)` matches the sliders' defaults — so frame 1 applies no spurious "change".

## Files Changed

### Repo (PR #330, merged)
- `examples/game_feel.rs` — NEW, the capstone example (~490 lines).
- `src/ui/node.rs` — `UiNode::with_visible` builder + `with_visible_spawns_hidden` test.
- `CLAUDE.md` — header v1.6.204 / v0.113.0; UI module-map row gains `with_visible` + the `game_feel` capstone sentence (two single-line edits; 200 lines exactly).
- `docs/CHANGELOG.md` — 0.113.0 entry.
- `Cargo.toml` / `Cargo.lock` — 0.113.0.

### Memory (outside repo, not in any PR)
- `engine-current-state.md` — seq 150 bump; then the trim (tip line cut at seq 137, body bullets removed, frontmatter `name`/`description` fixed, compaction rule rewritten); then the playtest-CLOSED edit.
- `MEMORY.md` — seq 150 bump; index line compacted to 1,107 chars; archive hook annotated; playtest-CLOSED edit.
- `engine-history-archive.md` — two new `## Trim 2026-07-02` sections (tip-line tail ≤ seq 136 + body bullets 117→76).
- Backups of all three pre-trim files in the session scratchpad.

### This handoff
- `plans/handoffs/HANDOFF_game-feel-ui-breadth_capstone-trim-playtest_2026-07-02.md` — to land as its own docs(handoff) PR.

## User Feedback & Preferences (REQUIRED)

- **Paste-prompt onboarding with narration + wait-for-go-ahead** — the user's session opener explicitly required narrating the onboarding (summary → verification plan → key files + adjacent explorations → planned first action) and waiting. The adjacent-file requirement paid off directly (the app.rs discovery). Keep this exact protocol.
- **"진행해"** — terse approval of the #1 recommendation, consistent with the chain's pattern ("1번 진행" ×2 in the parent). Lead with a recommendation; the user picks fast.
- **"2 진행해"** — picked memory trim from the numbered follow-up list in my completion report. Numbered follow-up options at the end of a completed task work well with this user.
- **"준비 됐어. 테스트 시작해"** — the user resurfaced the deferred playtest todo themselves when display time became available; they expect standing todos to be tracked and actionable the moment they unblock (same as parent session).
- **"모두 테스트 통과"** — a single-line all-pass verdict (no per-item numbering needed when nothing failed). Record it and close the todo without asking for elaboration.
- **"/handoff 하고 머지 해서 정리하자"** — same close pattern as the parent: write the handoff AND land it as its own merged PR; don't leave it uncommitted.
- **Standing (unchanged):** user-facing Korean / repo artifacts English; merge authority delegated (squash on green CI, expressed as direct action); never push main; `cargo fmt` before verify; gate exits read non-piped (zsh `$pipestatus` is 1-indexed; `${PIPESTATUS[0]}` is silently empty); pre-1.0 MINOR-for-any-release versioning.

## Where We're Going

1. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY / EW-004 for a fourth consecutive session). A real game request beats everything below.
2. **The self-pick backlog is genuinely exhausted.** Capstone: done. Playtest: done, all-pass. Memory trim: done. app.rs split: deprioritized (~350 code lines, already modularized). If the board is still empty, **ask the user for direction** rather than inventing work. Honest remaining candidates, all minor:
   - Dropdown `corner_radius > 0` row-seam cosmetic nit (on record since seq 143).
   - Widget follow-ons **only if asked** (multi-line text area, list box, modal dialog — the parent's judgment that the set is complete for settings menus was confirmed by the capstone).
   - A `docs/PATTERNS.md` "settings menu" recipe entry (parent open question; the `game_feel` example now serves as the de-facto recipe, which may be answer enough).
   - Ship `game_feel` to the web via `/ship-wasm-example` (it uses no native-only deps) — showcase value only.
3. **Natural next arc if the user wants one:** hand the widget suite + capstone recipe to the Dungeon-Merchant side (it consumes the engine via the wishlist board) — e.g. suggest the game side file EW-004+ requests from real settings-menu needs. That decision belongs to the user (it crosses into the game repo's session).
4. **Memory hygiene is now self-sustaining:** the compaction rule lives in `engine-current-state.md` itself (bump per PR; cut the chain tail every ~10 seqs). Next trim should be a 10-minute job, not a session item.

## Risks & Blockers

- **Spurious background-task completion notifications** (new, harness-level): a "completed (exit 0)" notification arrived for a still-running verify task. Mitigation now proven: treat the notification as a hint; confirm via the task output file's sentinel line (`VERIFY_EXIT=…`) + `pgrep`/`kill -0` process liveness; if still running, set a PID watcher loop. Expect recurrence.
- **Memory bump anchors differ per file** (ecs `— package vX` prose vs MEMORY.md `(**vX**,` parenthetical) — a copy-pasted single anchor will assert-fail on one of them. The Evidence section records both current phrasings.
- **The trimmed memory relies on the archive for anything ≤ seq 136** — if a next session needs old-decision context (pre-0.100 rationale), it must Read `engine-history-archive.md` explicitly; the recall description says "old-version/past-decision context only".
- **CI is ubuntu-only** — standing; nothing this session was OS-gated (the playtest covered the macOS-windowed reality directly).
- **2 audio tests passed locally this session** (device present) — do not un-learn the standing `--skip` pattern; on a locked/remote box they still fail (environment, not code).

## Open Questions

- **None blocking.** The parent's "settings-menu recipe doc?" question is softly answered by the capstone example itself; formalizing it into `docs/PATTERNS.md` remains optional.
- Whether to push the widget suite toward Dungeon-Merchant consumption (item 3 above) is a user-level direction call.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5           # tip = docs(handoff) PR over d6b83ed #330 game_feel (v0.113.0)
git status -s                  # clean

# Board FIRST (ACTIVE EMPTY for 4 sessions → if still empty, ASK the user for direction;
# the self-pick backlog is exhausted — see Where We're Going §2 for the minor leftovers)
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md

# Verify baseline (7 gates; read exit non-piped; audio tests may fail on a no-device box)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?

# Key files from this session
#   examples/game_feel.rs        — the capstone: settings-menu recipe + juice-stack composition
#                                  (HitFlash re-add guard, trail apply-on-change, TimeScale resolver)
#   src/ui/node.rs               — UiNode::with_visible (~line 139)
#   examples/ui_tabs.rs          — the underlying tab-container visibility pattern
# Memory structure (post-trim): engine-current-state.md = 18 lines, tip line seq 150→137 only,
#   compaction rule at the bottom; MEMORY.md index line is a 1.1 KB hook; older detail →
#   engine-history-archive.md "## Trim 2026-07-02" sections. Bump anchors differ per file —
#   see this handoff's Evidence §anchor phrasings.

# Live check of the capstone (auto-plays; menu opens at frame 54)
HEADLESS_SHOT=/tmp/game_feel.png cargo run --example game_feel

# Next action
#   Check the board. If empty: ask the user for direction (recommend: either hand the widget
#   suite to the Dungeon-Merchant side to generate real EW requests, or pick a minor leftover
#   from Where We're Going §2). Ship anything via /add-feature-example → land-pr loop.
```

## Session Closed

**Closed at:** 2026-07-02
**Commit:** landed via this docs(handoff) PR (#331; session code tip = `d6b83ed` #330)
**Session status:** Handed off to next session
