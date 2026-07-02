# UI widget suite completed + text z-ordering — 7 shipped PRs (Tooltip, Dropdown ×3, text-z, RadioGroup, TabBar)

**Date:** 2026-07-02
**Status:** COMPLETED (7 PRs shipped + merged; tree clean, all green)
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `game-feel-ui-breadth` seq `2`
**Parent:** `HANDOFF_game-feel-ui-breadth_2026-07-02.md`
**Prior chain:** `HANDOFF_game-feel-ui-breadth_2026-07-02.md` > this

---

## Since Last Handoff

- **Parent's "Where We're Going" item 2 (UI widgets: tooltip, dropdown, radio group, tab container) is now FULLY DONE** — all four candidates shipped this session (#322 Tooltip, #323 Dropdown, #327 RadioGroup, #328 TabBar). The game-UI widget set is effectively complete: Button / Label / TextInput / ScrollView / Slider / CheckBox / ProgressBar / Tooltip / Dropdown / RadioGroup / TabBar (+ Panel / VirtualJoystick / LocalizedText).
- **A renderer improvement NOT on the parent's list emerged and shipped:** #326 text z-ordering (v0.110.0). The Dropdown capture surfaced an engine-wide "text bleeds through overlays" limitation (all text drew in one final pass after all UI rects); fixing it became the user-picked work ("1번 진행") and is the session's most architecturally significant change.
- **Parent risk "GUI playtest is blocked" partially LIFTED:** the user ran the first real-mouse playtest of the session's widgets on a live display ("todo 진행 가능해") — it caught 3 real dropdown bugs that headless verification could not (#324), validating the standing concern that gesture paths need a human.
- **Parent's open question (game-feel capstone example) is still open** — deferred again; it and the `app.rs` split remain the top self-pick candidates.
- **Board still ACTIVE EMPTY (EW-004)** — unchanged from parent; direction came from the user each time (pick #1 twice, then "후속 위젯 진행해줘").
- **Memory tip-line growth risk continues:** `engine-current-state.md` is now ~177 KB (was ~77 KB tip line at parent); every bump still goes through in-place Python `str.replace`. A trim into `engine-history-archive.md` is increasingly due.

## Reference Documents

- `CLAUDE.md` — module map + verify gates + conventions; edited every PR (kept at exactly **200 lines**; now header **v1.6.203**, package **v0.112.0**).
- `docs/VISION.md` — the feature+example loop; ran 5× this session (tooltip, dropdown, text-z, radio, tabs each with a new example).
- `docs/CHANGELOG.md` — entries 0.108.0 → 0.112.0 (this session).
- Memory: `engine-current-state.md` (tip = seq 148, `ec8e212`), `MEMORY.md` index, `merge-authority-delegated.md` (used 7×), `new-model-subagent-incompat.md` (no subagents were needed this session — all inline).
- Parent handoff `HANDOFF_game-feel-ui-breadth_2026-07-02.md` — the widget-registration touchpoint list (§Code Analysis) was the direct recipe reused 4× this session.

## The Goal

Continue the engine's breadth run (hackable, fork-friendly, genre-agnostic MIT 2D skeleton per `docs/VISION.md`) by completing the **game-UI widget set** the parent handoff identified as the top candidates — tooltip, dropdown, radio group, tab container — plus whatever the work surfaces. The Dungeon-Merchant wishlist board stayed empty, so the session ran on the standing protocol: present candidates with a recommendation, let the user pick tersely, ship each pick through the full VISION loop (feature + example + tests + verify + PR + CI + squash-merge). End state: 4 new widgets, 1 renderer-level text-layering capability, 2 playtest-driven fix PRs — 7 merges, v0.107.0 → v0.112.0.

## Where We Are

- **main @ `ec8e212`** (package **v0.112.0**, CLAUDE.md header **v1.6.203**), tree clean, all green, no open PRs.
- **7 PRs merged this session** (all squash-merged on green CI 5/5 per delegated authority):
  - **#322 v0.108.0** — `Tooltip` hover popup widget (`src/ui/tooltip.rs` + `system/tooltip_pass.rs`, `PointerCapture::occludes`, `InputState::set_cursor` now `pub`).
  - **#323 v0.109.0** — `Dropdown` combobox (`src/ui/dropdown.rs` + `system/dropdown_pass.rs`, `DROPDOWN_LIST_Z`=90, `UiEvent::DropdownChanged`).
  - **#324 v0.109.1** — 3 dropdown fixes from the first real-mouse playtest (example event bus; Enter-open closes other lists; open-on-PRESS with transient `press_opened`).
  - **#325 v0.109.2** — example-only nit (bottom flip-up dropdown moved x=320→520 clear of HUD lines).
  - **#326 v0.110.0** — **text z-ordering**: `DrawText.z: Option<f32>` + `with_z`; layered text composites AMONG UI rects (pre-post, HDR grades with widget); `None` = byte-identical legacy on-top pass. All widget passes set label z → overlays now really hide covered labels.
  - **#327 v0.111.0** — `RadioGroup` (`src/ui/radio_group.rs` + `system/radio_group_pass.rs`, `UiEvent::RadioChanged`).
  - **#328 v0.112.0** — `TabBar` (`src/ui/tab_bar.rs` + `system/tab_bar_pass.rs`, `UiEvent::TabChanged`).
- **Memory seqs 141→148** (142 tooltip, 143 dropdown, 144 playtest-fixes, 145 example-nit, 146 text-z, 147 radio, 148 tabs).
- **Lib unit tests grew ~1075 → 1125** (+13 tooltip, +14→17 dropdown, +7 interleave_runs, +13 radio, +12 tabs, + assorted); **+4 doctests** (tooltip/dropdown/radio/tabs one each). **Render tests 11 → 12** (`layered_text_is_covered_by_higher_z_rect` runs on CI lavapipe).
- **Every widget verified headlessly** (HEADLESS_SHOT captures visually inspected) **AND** — new this session — **tooltip + dropdown verified by a real-mouse user playtest** with a fix→re-test cycle (all four #324 fixes confirmed by the user on re-test).
- **`UiEvent` gained 3 variants** (`DropdownChanged`, `RadioChanged`, `TabChanged`) — each technically breaking for exhaustive matches (pre-1.0 license, CHANGELOG-noted each time).
- **The land-loop ran identically 7×:** branch → `cargo fmt` → verify 7 gates (exit read non-piped) → /ship paperwork → commit → push → PR → `gh pr checks --watch` (background) → CI 5/5 → `mergeStateStatus == CLEAN` → squash-merge → `git pull --ff-only` → memory bump.
- **New for the last 2 PRs: a stacked-branch pipeline** — TabBar was implemented on a branch stacked on the un-merged RadioGroup branch while #327's CI ran, then rebased onto main after the squash-merge. Zero conflicts, zero idle CI-wait time (see Key Decisions).
- **2 audio tests still `--skip`ped locally** (no audio device on this box; CI gates them). Standing pattern, unchanged.

## What We Tried (Chronological)

1. **Onboarding.** Read parent handoff; board `../dungeon-merchant/docs/engine-wishlist.md` still ACTIVE EMPTY (EW-004). Presented candidates with Tooltip as the recommendation; user: **"1번진행"**.
2. **#322 Tooltip.** `Tooltip{text, delay_secs=0.4, fade_secs=0.1, font_size, text/bg/border colors, corner_radius, border, padding, offset, size_override}` + private hover timer. `tooltip_pass` runs **last** so tooltip text draws over other widget text. Hover = cursor-in-rect AND NOT `PointerCapture::occludes(cursor, entity, z)` — a NEW capture query ("is a pointer-opaque surface strictly above z covering this point?"), needed because tooltip hosts (Label/ProgressBar) aren't capture items themselves; equal-z is forgiving. Box auto-sizes via a shaped-width ESTIMATE (0.5 em/ASCII, 1 em/CJK, 1.2× line height, `\n` breaks); `with_size` pins exactly. Viewport clamp: right overflow slides left, bottom overflow flips above cursor (`FLIP_GAP`=8). **API opening forced by the example (VISION rule):** `InputState::set_cursor` was `pub(crate)` — the example needed synthetic hover; made `pub` (justified: virtual/gamepad cursor). 13 tests + 1 doctest.
3. **User: "테스트는 todo로 남겨두고 다음작업은 후속 위젯 추가 진행해줘"** → real-mouse playtest deferred as a todo; proceed to Dropdown.
4. **#323 Dropdown.** `Dropdown{items, selected, open[transient #[serde(skip)]], bg/hover/text colors, font_size, corner_radius, item_height[0=node height]}`. **Single-geometry-source design:** `flips_up`/`list_pos`/`expanded_rect` shared by render + click-row resolution + capture so they can never disagree. Open list registers its whole expanded rect in `PointerCapture` at `DROPDOWN_LIST_Z`=90 (< `TOOLTIP_Z`=100) → absorbs clicks/hover over covered widgets. Focusable: Enter/Space/A toggles, ←/→ steps selection clamped. Bottom-edge flip-up. 14 tests + 1 doctest. **Two test traps fixed:** (a) `UiNode::new` defaults z=0.9, ABOVE a panel bg at 0.89 — the "covered" test needed an explicit widget z=0.2; (b) `InputState::press` on an already-held key doesn't re-fire `just_pressed` and `flush()` doesn't clear `pressed` → insert a FRESH `InputState` per keypress (`press_key` helper, reused in every later widget's tests). **Surfaced (not fixed):** engine-wide text-after-rects bleed — labels showed through the open list.
5. **User: "todo 진행 가능해. 내가 해야될 일 알려줘"** → launched the example windows and gave Korean click-through checklists.
6. **User playtest findings (verbatim structure):** tooltip 정상 all-pass. Dropdown: (2,3,4) HUD event line + Apply counter dead; (7) Enter-opening a list did NOT close a mouse-opened one; (8-2) press-and-hold on the box did not open; (8-1) press-drag-release DID select correctly.
7. **#324 the 3 fixes.** (a) Example missed `app.register_event::<UiEvent>()` — without the bus every event is silently dropped (engine behavior correct; example-only). (b) `focus_pass` Enter-open now closes all other open Dropdowns (matches the pointer path where a press on one dropdown is a press-away for the rest). (c) Open on **PRESS** with a transient `press_opened` flag so the opening gesture's own release-on-box doesn't immediately re-close (native combobox one-gesture press-drag-release). **Two red tests during the fix:** `press_away_closes_without_selecting` — the press-away branch sat AFTER the bare `just_released` cleanup, so a same-frame press+release away only cleared the flag; reordered press-away FIRST. `press_drag_release_selects_in_one_gesture` — used example geometry (rows y 186..218) instead of the test fixture's (rows y 80..170). +3 tests → 17.
8. **User re-test: all four fixes pass.** One cosmetic note (bottom dropdown box overlapped HUD text — "테스트 프로그램이라 중요한 문제는 아니지만 알려줌") → **#325** example-only x=320→520.
9. **User: "1번 진행"** (text z-ordering, my #1 candidate) → **#326.** The big one:
   - `DrawText.z: Option<f32>` + `with_z`. `None` = historical final on-top pass (post-post, right for HUD, byte-identical). `Some(z)` = interleaved among UI rects at that z; tie → text over surface; rendered PRE-post so HDR/bloom grades text with its widget.
   - `text/layering.rs::interleave_runs(surface_zs, text_zs) -> Vec<Run{surfaces,texts}>` — pure function via `partition_point`; rule: text at z draws after every surface with z <= its z. 7 unit tests.
   - **wgpu constraint that shaped the design:** `queue.write_buffer` executes at submit start — re-uploading the same instance buffer mid-frame clobbers ALL earlier passes. Hence `prepare_ui_primitives` (sort + upload ONCE, returns `PreparedUiPrimitives{keys, zs}`) + `render_ui_primitive_range(start, end)` (per-run instance-buffer byte-offset sub-range draws). Same reason each glyphon batch needs its own pooled `TextRenderer` (one prepared batch's vertex buffers per renderer).
   - `TextRenderer` refactor: `FormatPool{format, atlas, renderers, used}` — per-target-format atlas + per-batch renderer pool (text analogue of the repo's format-matched pipeline caches; HDR `Rgba16Float` intermediate gets its own pool lazily). `render_batch(...)` + `end_frame()` (cache evict by generation, atlas trim, pool reset — once per frame).
   - `frame.rs` step 2.7: `TextQueue::take_layered()` partitions; no layered text → fast path unchanged; else stable-sort by z → alternate rect-range passes and text batches per `interleave_runs`.
   - Every widget pass got `.with_z(...)` on its labels (label/button/checkbox/text-input/scroll-view/dropdown box+rows/tooltip) → the open list and tooltips now hide covered labels for real.
   - Example `text_layers` (Space raises/lowers an overlay card; its caption covers/uncovers) + CI lavapipe render test `layered_text_is_covered_by_higher_z_rect`.
   - **One red verify (VERIFY_EXIT=101):** clippy `too_many_arguments` (8/7) on `prepare_ui_primitives` + the render-test helper → `#[allow(clippy::too_many_arguments)]` (precedent: `TextRenderer::render`).
10. **User: "후속 위젯 진행해줘"** → the remaining two widgets as a 2-PR batch.
11. **#327 RadioGroup.** ONE entity = the whole group (options can never disagree — same single-owner shape as Dropdown/ScrollView, deliberately NOT one-entity-per-option). Rows: SDF circle ring (`DrawRect` with `corner_radius=d/2` + `with_border(2.0)`) + selected dot (inset 0.3×d) + label + hover tint. `row_at` = single geometry source, **clamped to the node rect** (= the capture surface): overflowing rows (item_height too big) render but don't select; dead space below short fixed rows selects nothing. Click = CheckBox-style press+release ownership (drag-off cancels); `RadioChanged` only on actual change. Focus: ONE stop; ←/→ steps clamped (same code shape as Dropdown's step). `item_height=0` divides node height evenly (`node_h / n` — different default than Dropdown's node-height because the group node spans all rows). 13 tests + 1 doctest, green on first run. Example `ui_radio` (2 groups, one custom-styled).
12. **#328 TabBar — stacked-branch pipelining.** While #327's CI ran, branched `feat/ui-tab-bar` off the un-merged radio branch and implemented TabBar (same recipe: component + pass + focus/capture/registrations + tests). Equal-width headers = `(node_w - gap×(n-1)) / n` floored at 0; `tab_rect`/`tab_at` single geometry source — **gaps select nothing**; headers render active/hover/inactive bg + centered titles (`TextAlign::Center`). Bar renders **headers only** — content switching is the game's job (toggle per-tab widgets' `UiNode.visible`; hidden widgets are already skipped by focus cycling, so pad/keyboard nav never lands on inactive-tab content — a free composition win). 12 tests + 1 doctest. Example `ui_tabs` (Stats/Inventory/Options: gauges / scroll list / checkbox+slider, per-frame visibility system). After #327 merged: `git rebase --onto main e8d594a feat/ui-tab-bar` (hash as upstream — see gotcha below), amend wip → release commit, PR, CI 5/5, merge.
13. **Memory bumped 147+148 in ONE Python patch** (both `engine-current-state.md` and `MEMORY.md`, asserted single-occurrence prefix replace).
14. **User: "/handoff 하고 머지 해서 한번 정리하고 가자"** → this handoff, to be landed as its own docs(handoff) PR.

## Key Decisions

- **Tooltip hover uses a NEW capture query (`occludes`) instead of `topmost_at`.** Tooltip hosts (Label, ProgressBar) are not pointer-opaque capture items, so "am I the topmost?" is unanswerable for them; "is something opaque strictly ABOVE me here?" is. Equal-z does not occlude (forgiving for same-depth stacks).
- **`InputState::set_cursor` promoted to `pub`.** Forced by the example (VISION rule: fix the API when the example is awkward); justified on its own merits — virtual/gamepad-driven cursors and synthetic hover in headless tests.
- **Dropdown/RadioGroup/TabBar all follow the single-geometry-source pattern** (one set of pure component methods drives render + click resolution + capture). This is now the house style for any widget with sub-element hit zones; it eliminated a whole class of "render and hit-test disagree" bugs.
- **Open-on-PRESS (not click) for Dropdown** — the native combobox gesture (press, drag onto a row, release selects). The transient `press_opened` flag (never serialized) distinguishes the opening press's own release from a later closing click. Decided by the user's real-mouse playtest finding 8-2, not by spec guessing.
- **Text z-ordering is opt-in per DrawText (`Option<f32>`), not a global change.** `None` keeps the exact historical on-top pass — HUD code and every existing game render byte-identical; only widget-pass labels moved. Rejected: always-layered (would re-grade HUD text under HDR and break "text always on top" games).
- **Single instance-buffer upload + range draws** (not per-run re-upload) and **pooled per-batch glyphon renderers** — both dictated by wgpu's submit-time `write_buffer` semantics. This is the third instance of the repo's "format-matched cache" pattern (sprite pipelines, material pipelines, now text atlas pools).
- **RadioGroup = ONE entity per group.** Rejected one-entity-per-option with a shared group id: selection lives in N places and can disagree; scene-save + inspector get messier. Matches Dropdown/ScrollView precedent.
- **RadioGroup `row_at` clamps to the node rect** so click resolution can never disagree with the capture surface (which only claims the node rect). Overflowing rows render (visible authoring feedback) but don't select.
- **TabBar renders headers only; content switching stays in game code.** Rejected: engine-managed tab-content containers (child-entity visibility magic). The manual wiring is 10 lines in the example, composes with focus for free, and keeps the widget genre-agnostic/fork-friendly.
- **Stacked-branch pipelining for the 2-PR batch** — implement PR N+1 on a branch off PR N's un-merged branch while N's CI runs; after N squash-merges, `rebase --onto main <N-tip-hash>`. Conflict-free because squashed-main content == branch-N content. Saved a full CI wait (~7 min) with zero risk.
- **Each fix/nit its own PR** (#324 engine fixes vs #325 example nit) — one PR = one coherent change, even when tiny.

## Evidence & Data

### PRs this session

| PR | Version | Type | Summary | Tests |
|---|---|---|---|---|
| #322 | v0.108.0 | feat | Tooltip hover popup (+`PointerCapture::occludes`, `set_cursor` pub) | +13 unit, +1 doctest |
| #323 | v0.109.0 | feat | Dropdown combobox with occluding open list | +14 unit, +1 doctest |
| #324 | v0.109.1 | fix | Playtest fixes: example event bus, Enter single-open, open-on-press | +3 unit (→17) |
| #325 | v0.109.2 | fix | Example nit: flip-up dropdown clear of HUD lines | 0 |
| #326 | v0.110.0 | feat | Text z-ordering: DrawText.with_z interleaves text among UI rects | +7 unit, +1 render test |
| #327 | v0.111.0 | feat | RadioGroup — mutually exclusive options, one entity per group | +13 unit, +1 doctest |
| #328 | v0.112.0 | feat | TabBar — equal-width headers, one active | +12 unit, +1 doctest |

### Commit log (this session, newest first)

| Hash | PR | Subject |
|---|---|---|
| `ec8e212` | #328 | feat(ui): TabBar — equal-width tab headers, one active (v0.112.0) |
| `cc0f5ea` | #327 | feat(ui): RadioGroup — mutually exclusive option widget (v0.111.0) |
| `ea24626` | #326 | feat(render): text z-ordering — DrawText.with_z composites text among UI rects (v0.110.0) |
| `426bf0f` | #325 | fix(examples): ui_dropdown — move the bottom flip-up dropdown clear of the HUD lines (v0.109.2) |
| `3f6a9b5` | #324 | fix(ui): dropdown playtest fixes — example event bus, Enter single-open, open-on-press (v0.109.1) |
| `6379053` | #323 | feat(ui): Dropdown — combobox widget with an occluding open list (v0.109.0) |
| `658318b` | #322 | feat(ui): Tooltip — hover popup widget for game UI (v0.108.0) |

### Version / header / memory progression

| After PR | package | CLAUDE.md header | memory seq | main @ |
|---|---|---|---|---|
| (start) | 0.107.0 | v1.6.196 | 141 | 30d9d8b |
| #322 | 0.108.0 | v1.6.197 | 142 | 658318b |
| #323 | 0.109.0 | v1.6.198 | 143 | 6379053 |
| #324 | 0.109.1 | v1.6.199 | 144 | 3f6a9b5 |
| #325 | 0.109.2 | v1.6.200 | 145 | 426bf0f |
| #326 | 0.110.0 | v1.6.201 | 146 | ea24626 |
| #327 | 0.111.0 | v1.6.202 | 147 | cc0f5ea |
| #328 | 0.112.0 | v1.6.203 | 148 | ec8e212 |

### Real-mouse playtest findings → fixes (the fix→re-test cycle)

| # | User finding (playtest 1) | Root cause | Fix (PR) | Re-test |
|---|---|---|---|---|
| 2/3/4 | Dropdown HUD line + Apply counter dead | Example never called `register_event::<UiEvent>()` — no bus → events silently dropped | Example fix (#324) | PASS |
| 7 | Enter-opening list B left mouse-opened list A open | focus_pass Enter-toggle didn't close others | Close all other Dropdowns on Enter-open (#324) | PASS |
| 8-2 | Press-and-hold on the box did not open the list | Opened only on completed click | Open on PRESS + transient `press_opened` (#324) | PASS |
| 8-1 | Press-drag-release selected the release row | (already correct) | — | PASS |
| nit | Bottom dropdown box overlapped HUD text | Example layout | x=320→520 (#325) | PASS |

Tooltip playtest: all checklist items passed first try (delay, fade, flip, occlusion under a panel).

### The UiEvent surface after this session

`ButtonClicked` · `TextChanged` · `TextSubmitted` · `TextFocused` · `TextBlurred` · `SliderChanged(f32)` · `CheckBoxToggled(bool)` · **`DropdownChanged(usize)`** · **`RadioChanged(usize)`** · **`TabChanged(usize)`** — the three new variants each emit ONLY on an actual index change (re-pick is silent), and each addition is technically breaking for exhaustive matches (pre-1.0, CHANGELOG-noted).

### UI z-constant ladder (now load-bearing across 3 widgets + text layering)

| Constant | Value | Meaning |
|---|---|---|
| `UiNode.z` recommended range | 0.0..=1.0 | normal widgets |
| Panel bg offset | z − 0.01 | `PANEL_BG_Z_OFFSET` (capture registers panels here) |
| `UI_SUBLAYER_Z_STEP` | 0.001 | widget sub-element step (tick, fill, dot, label) |
| `DROPDOWN_LIST_Z` | 90.0 | open dropdown expanded rect (capture + render) |
| `TOOLTIP_Z` | 100.0 | tooltips above everything incl. open lists |
| `DrawText.z = None` | ∞ effectively | legacy final on-top pass (post-post) |

### Headless captures (all visually confirmed)

| Example | What the capture showed |
|---|---|
| `ui_tooltip` | box near cursor with border + fade, flipped above at bottom edge |
| `ui_dropdown` | open list overlaying buttons; after #326 re-capture: button labels no longer bleed through the list |
| `text_layers` | overlay card cuts the lower caption exactly at its edge; HUD (no z) on top |
| `ui_radio` | rings as circles, dots on Normal/Orchestral, custom colors/sizes, HUD "Picked: Normal / Orchestral" |
| `ui_tabs` | Stats tab blue/active, inactive tabs gray, only HP/MP gauges visible, titles centered |

### Verify gate (identical 7×; exit read non-piped)

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · wasm build (lib+bins) · wasm clippy `--lib` · `cargo test --all-targets` (2 audio skips locally) · `cargo test --doc` · `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`. Each PR's CI: Build (WASM) · Package dry-run · Render tests (lavapipe) · Rustdoc · Test (native) — 5/5 every time. RadioGroup ran verify **twice** (pre- and post-version-bump); TabBar folded paperwork in before its single full verify.

### Unit test names (the tested contracts — a future edit to these modules must keep them green)

- **Tooltip pass (4):** `tooltip_appears_only_after_the_delay`, `leaving_the_widget_resets_the_delay`, `covered_widget_shows_no_tooltip` (occludes + control with the panel dropped below), `tooltip_clamps_inside_the_viewport` (right slide + bottom flip). Component tests (9) cover delay/fade accessors, size estimate (ASCII vs CJK widths), builders, serde/reflect.
- **Dropdown pass (11):** `click_opens_then_row_click_selects_and_emits`, `reselecting_the_same_item_closes_without_an_event`, `press_away_closes_without_selecting`, `open_list_occludes_a_button_underneath` (+ closed-control), `bottom_edge_dropdown_opens_upward`, `covered_dropdown_does_not_open`, `focused_dropdown_enter_toggles_and_arrows_step_selection`, `press_drag_release_selects_in_one_gesture`, `opening_press_released_on_the_box_keeps_it_open_then_click_closes`, `enter_opening_one_dropdown_closes_another`, `open_list_rows_reach_the_ui_queue_at_the_list_z`. Component (6): `selection_clamps_on_read_without_mutating`, `item_height_falls_back_to_the_node_height`, `list_opens_below_and_flips_above_at_the_viewport_bottom`, `builders_set_fields`, `serde_roundtrip_keeps_items_and_selection_but_drops_open`, `reflect_roundtrip`.
- **RadioGroup (13):** component — `selection_clamps_on_read_without_mutating`, `item_height_divides_the_node_evenly_by_default`, `row_at_maps_cursor_to_rows`, `row_clicks_clamp_to_the_node_rect`, `builders_set_fields`, `serde_roundtrip_keeps_items_and_selection`, `reflect_roundtrip`; pass — `clicking_a_row_selects_it_and_emits`, `reselecting_the_current_row_is_silent`, `covered_radio_group_does_not_select`, `drag_off_the_widget_cancels_the_selection`, `focused_arrows_step_selection_clamped`, `rows_render_rings_and_one_dot`.
- **TabBar (12):** component — `selection_clamps_on_read_without_mutating`, `tab_width_splits_evenly_minus_gaps` (incl. gap-wider-than-node floors at 0), `tab_at_maps_cursor_to_headers_and_gaps_to_none`, `builders_set_fields`, `serde_roundtrip_keeps_tabs_and_selection`, `reflect_roundtrip`; pass — `clicking_a_header_selects_it_and_emits`, `reselecting_the_current_header_is_silent`, `clicking_a_gap_selects_nothing`, `covered_tab_bar_does_not_select`, `focused_arrows_step_selection_clamped`, `headers_render_one_rect_each_and_titles`.
- **Text z-ordering (#326):** `layering.rs` 7 unit tests on `interleave_runs` (empty inputs, all-below, all-above, tie-goes-to-text, interleaved mix); render test `layered_text_is_covered_by_higher_z_rect` (covered layered text < visible/20 pixels, visible control > 100, no-z on-top text > 100 — bg-relative counts, renderer-tolerant).
- **Lib test totals:** 1100 after #326 → 1113 after #327 → **1125** after #328 (13 radio + 12 tabs, exact filter counts from `cargo test --lib`).

### Process gotchas (this session's new ones — beyond the standing zsh/pipe rules)

- **`gh pr merge --delete-branch` deletes the LOCAL branch too** (not just remote) when it isn't checked out. A stacked branch that named it as rebase upstream must use the **commit hash** instead: `git rebase --onto main e8d594a feat/ui-tab-bar`. Conflict-free because squashed-main content == pre-squash branch content.
- **`git checkout main` fails with uncommitted stacked work** ("would be overwritten") — commit a `wip:` on the stacked branch first, then `--amend` it into the release commit after paperwork+verify. Amend keeps one clean commit per PR.
- **Do not edit `src/` while a verify gate runs** — later gates would compile different code than earlier gates, invalidating the run. Safe meanwhile: CHANGELOG/CLAUDE.md/handoff (not compiled). This is why RadioGroup's paperwork was staged md-first, Cargo.toml-after-verify-1.
- **Background-log grep for "error" false-triggers on test names** (e.g. `...returns_decode_error ... ok`) — trust the background task's completion exit code, not a text grep (bit once at #326).
- **The #324 lesson is now baked into every new example:** `app.register_event::<UiEvent>()` before any widget event is read — `ui_radio` and `ui_tabs` both ship with it; without the bus, events are SILENTLY dropped (no warning at the widget layer).

### Test-fixture traps (reusable — bit this session, will bite again)

- `UiNode::new(...)` defaults **z = 0.9**, which is ABOVE a panel bg at `0.9 − 0.01 = 0.89`. Any "covered widget" test must set the widget's z explicitly low (0.2) or the panel never covers it.
- `InputState::press(key)` on an already-held key does NOT re-fire `just_pressed`, and `flush()` clears just-pressed but not held → per-keypress tests must insert a **fresh `InputState`** (`press_key` helper, now copy-pasted in dropdown/radio/tab pass tests).
- Dropdown fixture geometry ≠ example geometry (test rows open at y 80/110/140; the example's rows differ) — write row-click tests against the FIXTURE's numbers.

## Code Analysis

- **`PointerCapture`** (`src/ui/system/capture.rs`) — `rebuild` registers Button/CheckBox/Slider/**RadioGroup**/**TabBar**/TextInput/ScrollView (node rect @ node z), Panel (@ z−0.01), Dropdown (closed: node rect @ z; open: `expanded_rect` @ 90). Queries: `topmost_at(cursor) -> Option<Entity>` (max z, tie → higher entity index) and `occludes(cursor, entity, z) -> bool` (any OTHER item with `it.z > z` containing cursor).
- **Widget-pass click contract** (checkbox/radio/tab): `clicked = just_released && topmost_at(press_cursor)==Some(e) && topmost_at(release_cursor)==Some(e)`; resolve the sub-element from **release_cursor**; mutate + emit inside `get_mut`, render from an immutable re-read after.
- **`interleave_runs(surface_zs: &[f32], text_zs: &[f32]) -> Vec<Run{surfaces, texts}>`** — both inputs pre-sorted ascending; text at z draws after every surface with z <= its z (tie → text on top).
- **`prepare_ui_primitives(...) -> PreparedUiPrimitives{keys, zs}`** — sorts + uploads instances ONCE + writes the camera uniform; `render_ui_primitive_range(ctx, prepared, start, end)` draws `[start, end)` with per-texture-key runs via instance-buffer byte offsets. Both `#[allow(clippy::too_many_arguments)]`.
- **`TextRenderer::FormatPool{format, atlas, renderers: Vec<GlyphonTextRenderer>, used}`** — `pool_index(device, queue, format)` lazily creates; `render_batch` acquires/grows a renderer per batch (one prepared batch's vertex buffers each); `end_frame()` evicts the shaped-buffer cache by generation, trims atlases, resets `used`.
- **`Dropdown` geometry** — `resolved_item_height(node_h)` (`0` → node_h), `list_height`, `flips_up(pos,size,vh)` (overflow below AND room above), `list_pos`, `expanded_rect -> (top_left, size)`.
- **`RadioGroup` geometry** — `resolved_item_height(node_h)` (`0` → node_h / n), `row_at(cursor,pos,size)` clamped to `min(rows_h, node_h)`. Render: ring = `DrawRect(d×d).with_corner_radius(d/2).with_border(2.0)`; dot inset = `0.3×d`; `d = circle_size.min(item_h)`.
- **`TabBar` geometry** — `tab_width(w) = ((w − gap×(n−1)) / n).max(0)`, `tab_rect(i)`, `tab_at` (iterates rects; gaps → None). Titles: `DrawText.with_bounds(tsize).with_align(TextAlign::Center)`.
- **Focus-pass step chain** (`focus_pass.rs`, nav_left/right): Slider → Dropdown → RadioGroup → TabBar, all the same clamped-step shape emitting their `*Changed` event. `collect_focusables` unions Button/TextInput/Slider/CheckBox/Dropdown/RadioGroup/TabBar, sorts by entity index, retains visible (+ button-not-disabled).
- **Widget registration touchpoints** (unchanged recipe, used 4×): `src/ui/mod.rs` (mod + re-export), `src/lib.rs` (`pub use ui::{...}`), `src/ui/system.rs` (mod + scratch field + pass call + doc list), `src/app/core_resources.rs` (reflect_named + clone + serde registry), `src/app/editor/component_registry.rs` (add + remover).

## Files Changed

### New source modules
- `src/ui/tooltip.rs` + `src/ui/system/tooltip_pass.rs` (#322)
- `src/ui/dropdown.rs` + `src/ui/system/dropdown_pass.rs` (#323, evolved by #324)
- `src/renderer/text/layering.rs` (#326, new pure module)
- `src/ui/radio_group.rs` + `src/ui/system/radio_group_pass.rs` (#327)
- `src/ui/tab_bar.rs` + `src/ui/system/tab_bar_pass.rs` (#328)

### Renderer (#326)
- `src/renderer/text/queue.rs` — `DrawText.z: Option<f32>` + `with_z` + `TextQueue::take_layered`.
- `src/renderer/text/renderer.rs` — FormatPool refactor + `render_batch` + `end_frame`.
- `src/renderer/sprite/ui_primitives.rs` — `PreparedUiPrimitives` + prepare/range split.
- `src/app/render/frame.rs` — step 2.7 interleave; step 4.7 simplified + unconditional `end_frame`.

### Wiring (multiple PRs)
- `src/ui/system/capture.rs` — `occludes` (#322); RadioGroup/TabBar registration (#327/#328).
- `src/ui/system/focus_pass.rs` — Dropdown Enter/step (#323), Enter single-open (#324), Radio/Tab steps (#327/#328).
- `src/ui/system/event.rs` — 3 new `UiEvent` variants.
- `src/ui/system.rs`, `src/ui/mod.rs`, `src/lib.rs`, `src/app/core_resources.rs`, `src/app/editor/component_registry.rs` — per-widget registration (4×).
- `src/input/state.rs` — `set_cursor` now `pub` (#322).
- Widget passes label z: `label/button/checkbox/text_input/scroll_view/dropdown/tooltip` passes (#326).

### Examples & render tests
- `examples/ui_tooltip.rs`, `examples/ui_dropdown.rs` (+#324/#325 edits), `examples/text_layers.rs`, `examples/ui_radio.rs`, `examples/ui_tabs.rs` (all flat — Cargo auto-discovers).
- `tests/render.rs` — `layered_text_is_covered_by_higher_z_rect` + `region_count_far` helper (#326).

### Release paperwork (per PR)
- `Cargo.toml`/`Cargo.lock`, `docs/CHANGELOG.md` (0.108.0→0.112.0), `CLAUDE.md` (header + module-map rows; exactly 200 lines each time).

### Memory (outside repo)
- `engine-current-state.md` + `MEMORY.md` — seqs 142→148 (Python in-place `str.replace`, asserted single occurrence; file now ~177 KB).

## User Feedback & Preferences (REQUIRED)

- **"핸드오프 확인하고 다음 작업 알려줘"** — session opener; wants the handoff to drive onboarding. The chain protocol works.
- **"1번진행" / "1번 진행" (twice)** — picks the #1 (recommended) candidate tersely. Keep leading the list with the recommendation.
- **"테스트는 todo로 남겨두고 다음작업은 후속 위젯 추가 진행해줘"** — comfortable deferring a human-verification todo to keep shipping; expects the todo to be tracked and resurfaced (it was, and it caught 3 bugs).
- **"todo 진행 가능해. 내가 해야될 일 알려줘"** — when unblocking a human-in-the-loop step, wants a concrete instruction list ("what do I do"), not a status update. The Korean click-through checklist format worked well (numbered findings came back keyed to it).
- **Playtest findings arrived as a numbered Korean list keyed to my checklist** — precise, reproducible, includes what DID work (8-1 정상) and severity judgments ("테스트 프로그램이라 중요한 문제는 아니지만 알려줌"). Honor the numbering in fixes and the fix report.
- **"후속 위젯 진행해줘"** — plural follow-up widgets = both remaining candidates (RadioGroup + TabBar), interpreted as a 2-PR batch and stated as such before executing; no objection.
- **"/handoff 하고 머지 해서 한번 정리하고 가자"** — handoff should be written AND landed (merged) as part of tidying up; don't leave it uncommitted.
- **Standing (unchanged):** merge authority delegated (squash on green CI, express as direct action, never an AskUserQuestion option); user-facing Korean / repo artifacts English; never push main directly; `cargo fmt` before verify; gate exits read non-piped (zsh `$pipestatus` 1-indexed, never `${PIPESTATUS[0]}`); 2 audio tests `--skip` locally.

## Where We're Going

1. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY / EW-004 throughout this session). A real game request beats all self-picks.
2. **If still empty, the remaining self-pick candidates, in recommended order:**
   - **Game-feel capstone example** (VISION capstone): one small playable combining FloatingText + InputBuffer + SpriteTrail + HitFlash + `Camera::shake` + the new widgets (pause/settings menu with Dropdown/RadioGroup/TabBar/Slider). Validates composition; the widget suite finally makes a real settings menu possible.
   - **`app.rs` (1063 lines) split** via `/split-module` — the parent's split method (§docked.rs split, seq 1) is the recipe. Deferred two sessions now; no longer "stacking refactors."
   - **Real-mouse playtest of RadioGroup/TabBar/text_layers** — tooltip/dropdown got one; the three newest did not (headless + unit only). Cheap to run when the user has a display.
   - Polish nit on record: dropdown rows with `corner_radius > 0` leave tiny seam notches between rows (cosmetic, noted seq 143).
3. **Widget-set follow-ons only if asked** (the set is complete for settings-menu purposes): multi-line text area, list box with selection, modal dialog helper.
4. **Memory maintenance:** trim `engine-current-state.md` (~177 KB) oldest per-seq detail into `engine-history-archive.md` (last done 2026-06-20) before the tip line becomes unmanageable.

## Risks & Blockers

- **Real-mouse verification gap for the newest 3 features** (text_layers / RadioGroup / TabBar): unit + headless verified only. The dropdown experience proved a live playtest finds real gesture bugs (open-on-press was invisible headlessly). Not blocking, but the first candidate when the user offers display time.
- **`UiEvent` exhaustive-match breakage** — 3 variants added this session; any downstream game matching exhaustively must add arms (pre-1.0 license, but worth remembering when Dungeon-Merchant consumes these widgets).
- **Layered text renders pre-post** — under HDR/bloom, widget labels are now tone-mapped with their widgets (deliberate; arguably correct). A game relying on the old "labels always un-graded" look would see a difference ONLY if it uses HDR post. HUD text (no z) unchanged.
- **CI is ubuntu-only** — unchanged standing risk for any future OS-gated work; nothing this session was OS-gated.
- **Memory tip-line size** (~177 KB file, single huge line) — every bump must stay on the Python `str.replace` path with asserted single-occurrence anchors; an Edit-tool attempt will fail its read gate.

## Open Questions

- **Game-feel capstone vs app.rs split first?** Both are ready; capstone is more VISION-aligned, split is hygiene. User's call next session.
- **Should Dropdown/RadioGroup/TabBar get a shared "settings menu" recipe doc** (docs/PATTERNS.md entry) now that the trio exists? Deferred — the examples serve as the pattern for now.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -8          # tip = ec8e212 #328 TabBar (v0.112.0); 7 PRs this session
git status -s                 # clean

# Board FIRST (ACTIVE EMPTY all session → if still empty, recommend: capstone example or app.rs split)
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md

# Verify (2 audio tests fail locally — env; read exit non-piped)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?

# Key files from this session (the reusable patterns)
#   src/ui/dropdown.rs + system/dropdown_pass.rs   — single-geometry-source widget + open-on-press state machine
#   src/ui/radio_group.rs / src/ui/tab_bar.rs      — the same recipe, simpler (good templates for any new widget)
#   src/ui/system/capture.rs                        — topmost_at vs occludes; every pointer-opaque widget registers here
#   src/renderer/text/layering.rs + renderer.rs     — interleave_runs + FormatPool (submit-time write_buffer constraint)
#   src/app/render/frame.rs (step 2.7)              — the interleaved rect/text composition

# Examples to run for a live check (each supports HEADLESS_SHOT=path)
#   cargo run --example ui_dropdown / ui_radio / ui_tabs / text_layers / ui_tooltip

# Next action
#   Check the board. If empty: recommend the game-feel capstone example (now unblocked by the
#   completed widget suite) with app.rs split as runner-up. Ship via /add-feature-example →
#   land-loop (branch → verify 7 gates → /ship → PR → CI 5/5 → squash-merge → memory bump).
```

## Session Closed

**Closed at:** 2026-07-02
**Commit:** landed via this docs(handoff) PR (#329; session code tip = `ec8e212` #328)
**Session status:** Handed off to next session
