# Post-v9.0.0 priority loop — 5 backlog items shipped (v9.1.0 → v9.3.0 + 2 docs PRs), all merged

**Date:** 2026-06-16
**Status:** COMPLETED
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine — engine hardening / post-hardening backlog
**Chain:** `engine-hardening` seq `3`
**Parent:** `HANDOFF_engine-hardening_v9.0.0-merged_2026-06-16.md` (seq 2)
**Prior chain:** `HANDOFF_engine-hardening_v9.0.0-shipped_2026-06-16.md` (seq 1) > `..._v9.0.0-merged_...` (seq 2) > this (seq 3)

---

## Related Handoffs

Not chain parents, but relevant — two of this session's PRs (#61 serde, #62 editor depth) drained items from the **editor-tile-painting** chain's deferred backlog:
- `HANDOFF_editor-tile-painting_items1-5-batch_2026-06-16.md` — seq 4 of that chain; its "Where We're Going" listed exactly the SM/Timeline serde + editor-depth + per-locale-font follow-ups this session worked. The remaining editor-tile-painting deferrals (SM visual node-graph, timeline time-ruler) are still open there.
- `HANDOFF_editor-tile-painting_rust-survivors-dropped_2026-06-16.md` — seq 5; records rust-survivors dropped as a consumer ([[rust-survivors-deprecated]]).

## Since Last Handoff

Seq 2 (v9.0.0 merged) closed its `## Where We're Going` with **5 optional follow-ups**. This session executed the backlog directly (the user asked for a prioritized list, then ran an autonomous `/loop` over it). Status of each parent next-action:

- **(2) `HotReloadable` full trait** → ✅ DONE as **#63 / v9.3.0** (was "shipped as macro dedup; full trait deferred — v9.1.0 candidate").
- **(3) REFERENCE.html full refresh** → ✅ DONE as **#65** (blockquote v5→v9.3 + 10 missing subsystem sections).
- **(4) Line-by-line 80-finding Appendix A audit** → ✅ DONE as **#64** (coverage ledger; 69/80 fully addressed, exceeding the analysis's own ~80% prediction).
- **(1) Tag the release** → NOT done (policy — tagging lapsed after v4.3.0, only on explicit request; the user did not ask).
- **(5) Editor-chain follow-ups (SM visual node-graph, timeline time-ruler)** → NOT done (visual editors, blocked by docked cursor-freeze playtest limit — deferred again).

**Beyond the parent's list:** this session also pulled **two items from the editor-tile-painting chain's deferred backlog** into the loop — SM/Timeline **serde persistence** (#61) and SM/Timeline **editor depth** (#62) — because the prioritized list (built from both chains' open items) ranked them Tier-1/Tier-2. **Trajectory:** the post-hardening backlog is now substantially drained; what remains is the genuinely large/visual/breaking tail.

## Reference Documents

- `CLAUDE.md` — project conventions + module map (header now v1.6.36 / package v9.3.0).
- `docs/CODE_ANALYSIS_2026-06-16.md` — the 80-finding audit (Appendix A).
- `docs/CODE_ANALYSIS_2026-06-16_COVERAGE.md` — **NEW this session**, the 1:1 verdict ledger.
- `docs/VISION.md` — the feature+example dogfooding loop (the standing "what next" guide).

## The Goal

The user wanted the post-v9.0.0 backlog worked down autonomously. First they asked for a **prioritized list** of remaining work (built by cross-checking the other session's 5 "optional" items against all handoff/plan docs), then ran `/loop` with: *"예정 작업 진행. opus 감독으로 sonnet 실무. 완료되면 테스트 후 머지하고 handoff 스킬로 문서정리 하고 완료사항 보고."* So: opus supervises (plans, gates, reviews, merges), Sonnet sub-agents implement, each item ships as a tested + CI-green + merged PR, then a handoff + completion report. Merge authority was explicitly granted for this loop ("테스트 후 머지").

## Where We Are

- **`main` = `6033a7f`, package v9.3.0, CLAUDE.md header v1.6.36.** Clean tree (one uncommitted file: the progress-doc RESOLVED banner, staged for this handoff's commit). All 5 PRs CI-green 4/4 + squash-merged + branches deleted.
- **716 lib tests** (hardening baseline 698 → +18 this session) + integration smoke, 0 fail. Full local Gate6 (`./scripts/verify.sh`) green after every code PR.
- **#61 / v9.1.0 — SM+Timeline serde persistence.** `AnimationStateMachine` (+ `AnimParam`/`TransitionCond`/`AnimTransition`/`AnimState`) and `Timeline` (+ `Track<T>`/`Keyframe<T>`/`CameraTarget`) + `Easing` now derive `Serialize`/`Deserialize`; auto-registered for serde in `register_core_component_metadata` (`src/app/core_resources.rs`) alongside UI widgets → editor edits survive scene save/load. +8 tests (698→706).
- **#62 / v9.2.0 — SM+Timeline editor depth.** New tested ops `AnimationStateMachine::set_transition_conditions(from,index,Vec<TransitionCond>)->bool` + `set_transition_crossfade(from,index,f32)->bool`. SM panel: live param editing (bool checkbox / float drag / trigger fire), add-transition (target ComboBox + crossfade), per-transition condition add/remove. Timeline panel: per-track add-keyframe (empty tracks now render an add button) + per-type value editing (Vec2 x/y, f32, Color rgba). `timeline_track_ui` signature changed (added `at_time`, `make_default`, `value_edit` closures; removed the read-only `fmt`). +6 tests (706→712).
- **#63 / v9.3.0 — `HotReloadable` trait.** `engine::HotReloadable { fn reload_path(&mut self, path:&str) }` + `App::register_hot_reloadable::<T>()`. Replaced the hardcoded `forward_reloads!` macro in `src/app/schedule.rs` with a `Vec<fn(&mut World,&[String])>` of monomorphized forwarders. The 3 built-in registries (`DataTableRegistry`/`AnimationClipRegistry`/`ParticleConfigRegistry`) impl the trait (UFCS-delegating to their inherent `reload_path`) and are auto-registered in `App::new`. Native-only. +4 tests (712→716).
- **#64 — 80-finding coverage ledger.** `docs/CODE_ANALYSIS_2026-06-16_COVERAGE.md`: 4-way parallel read-only audit at v9.3.0 → **69/80 fully ADDRESSED (86%), 8 PARTIAL, 3 NOT** — 100% of the HIGH + correctness-MEDIUM tier (#1–46). Docs-only, no version bump.
- **#65 — REFERENCE.html refresh.** Fixed the stale top blockquote (v5.0.0→v9.3.0); added 10 `<h2>` sections for v8.2–v9.3 subsystems + a v9.1–9.3 changelog section + TOC links. +339/−6 lines. HTML structurally verified (tag balance, id uniqueness, 0 href↔id mismatches). Docs-only.
- **Deferred this session:** #5 per-locale font auto-select (reasoned defer — see Key Decisions); SM/Timeline visual editors; ConnectRule unification; v9.3 tag.
- **Memory updated:** `engine-current-state.md` rewritten from a bloated running log to a concise v9.3.0 snapshot (history lives in handoffs/git).

## What We Tried (Chronological)

1. **Built the prioritized backlog list.** Cross-checked the other session's 5 "optional" items (tag / HotReloadable / REFERENCE / Appendix-A audit / new features) against `docs/NEXT_WORK.md`, all `plans/handoffs/*`, and the live code. **Key correction surfaced:** the editor-tile-painting `deferred-candidates` handoff (2026-06-15) listed several "still-deferred" items that had since SHIPPED — verified `AnimationClipSet`/`ParticleConfigSet` (data-driven assets), the whole tile-painting editor chain, and RTL (`ExtraFonts`/`TextAlign::Auto/End`) all exist now. So the genuinely-open backlog was much smaller than the stale docs implied. Produced a 4-tier prioritized list.
2. **User ran `/loop`** (dynamic mode) over that list → I worked it as: scope (opus reads) → spawn Sonnet impl agent (background, explicit `model:sonnet`) → on completion, opus reviews diff + runs full Gate6 + bumps version/CHANGELOG/CLAUDE + commits on a feature branch + PR + `gh pr checks --watch` + squash-merge.
3. **#1 serde persistence.** Scoped the serde surface (concrete SM types = trivial; `Timeline`'s `Track<T>`/`Keyframe<T>` generics = the only nuance). Verified glam `serde` feature ON, `Color` derives serde, **`Easing` did NOT** (agent added it). Agent shipped clean: 706 tests, auto-registration mirroring UI widgets. Reviewed, gated, merged as #61/v9.1.0.
4. **#4 editor depth.** Read the existing panels — found them MORE complete than the handoff implied (Timeline already had `set_value`/`set_easing`; both panels had list/remove/add-state). Real gap = SM transition-**condition** editing (needed a new data op) + live-param wiring + Timeline add-keyframe/value-edit UI. Agent: 712 tests, 2 new tested SM ops, panel rewrite. Merged as #62/v9.2.0.
5. **#5 per-locale fonts — INVESTIGATED, then DEFERRED.** Read the font system: `build_font_system` loads base+extra fonts into one cosmic-text `FontSystem`; the text prepare path uses a **hardcoded** `Attrs::new().family(Family::SansSerif)` + cosmic-text per-glyph script fallback. `LocaleData.font` is a **filename**; cosmic-text wants a **family name**. Concluded: real implementation needs (a) renderer locale-awareness (un-hardcode the default family) + (b) a filename→family resolution layer, AND the result is **not autonomously verifiable** (text rendering needs a human eyeball, like `rtl_text`), AND its incremental value is marginal (script fallback already works). Deferred with a written spec rather than ship a blind, unverifiable render change.
6. **Reordered the loop** (supervisory call): did the high-confidence #6 + #3 before the murky #5, and put #2 (REFERENCE) last so it could document everything.
7. **#6 HotReloadable.** Designed the monomorphized-fn-pointer forwarder approach (no boxing; `fn(&mut World,&[String])`, `Copy`). Agent: 716 tests, trait + register + auto-register + macro removal, wasm build confirmed (no native-only leak). Merged as #63/v9.3.0.
8. **#3 Appendix A audit.** Fanned out **4 concurrent read-only Sonnet agents** (findings 1–20 / 21–40 / 41–60 / 61–80), each verdicting ADDRESSED/PARTIAL/NOT with current file:line evidence. Synthesized into the coverage ledger. Verified the one plan-vs-audit discrepancy myself (#73). Merged as #64 (docs-only).
9. **#2 REFERENCE refresh.** One Sonnet agent: blockquote + 10 sections + TOC, APIs pulled from source. Structurally verified myself (tags balanced, ids unique, href↔id clean, symbols spot-checked against source). Merged as #65 (docs-only).
10. **Wrap-up.** Updated the stale `engine-current-state` memory, added a RESOLVED banner to the stale progress doc, ran this handoff.

**Loop mechanics (how the autonomous `/loop` drove it):** dynamic mode (no interval). Each iteration: opus scoped an item inline (targeted reads/greps to write a precise Sonnet prompt), spawned ONE background Sonnet impl agent (`run_in_background: true`, explicit `model: sonnet`), wrote a short status, and ended the turn with a `ScheduleWakeup(1800s)` **fallback** (the background agent auto-wakes the loop on completion well before that — the wakeup is only a hang safety-net). On the completion notification, opus reviewed the diff, ran Gate6, bumped version/CHANGELOG/CLAUDE, committed on a feature branch, opened the PR, blocked on `gh pr checks <n> --watch --interval 30`, squash-merged, synced main, and spawned the next item's agent. The audit (#3) broke the pattern: 4 agents launched concurrently in one message (read-only ⇒ parallel-safe), synthesized after all 4 returned.

## Key Decisions

- **Deferred #5 (per-locale fonts) rather than ship blind.** It needs renderer-internal change to a delicate, visually-unverifiable hot path for marginal value (cosmic-text already script-falls-back). Wrote a precise spec for a display-access session instead. This is the one planned item NOT shipped — a deliberate quality call, not an omission.
- **Reordered the loop by confidence/verifiability** (#1→#4→#6→#3→#2, with #5 deferred), not by the raw priority numbers — sequence the cleanly-testable items first; do the big doc refresh last so it captures all new APIs.
- **Sequential PRs, not parallel.** The handoffs warn that parallel feature branches conflict on version/CHANGELOG/CLAUDE-header. One-merged-before-next-starts avoided all conflicts (cost: more wall-clock, fine for a background loop).
- **Audit = 4-way parallel read-only fan-out, foreground-concurrent then background.** Read-only ⇒ no write conflicts ⇒ safe to parallelize; 4×20 beats one agent over 80 (depth + speed).
- **Coverage ledger as the #3 deliverable — did NOT force perf-refactors.** The 3 NOT + 8 PARTIAL are all LOW-priority perf/api/by-design (the analysis predicted ~20% wouldn't be strictly closed). Refactoring delicate sprite/steering/lighting hot paths for marginal, visually-unverifiable gain was judged not worth the risk. Documented honestly instead, including the **#73 plan-over-claim** (WU6 note said "frustum prefilter"; only the viewport-center cull #38 shipped).
- **#74 reclassified ADDRESSED** (one auditor waffled): `zoom_target()` + `is_zooming()` expose the meaningful tween state; a raw `zoom_tween_speed` getter is unnecessary.
- **REFERENCE.html new content kept KOREAN** to match the existing user manual (consistency within the artifact beats the English-docs token rule, which targets agent-facing docs).
- **Rewrote the `engine-current-state` memory wholesale** (it had grown into a multi-screen running log with a stale "v9.0.0 IN-FLIGHT" description) → concise current snapshot; history is preserved in handoffs + git.

## Evidence & Data

### PRs shipped (all squash-merged, branches deleted)
| PR | Version | Title | Tests | CI |
|---|---|---|---|---|
| #61 | 9.1.0 | serde for AnimationStateMachine + Timeline | 698→706 | 4/4 |
| #62 | 9.2.0 | SM + Timeline panel editing depth | 706→712 | 4/4 |
| #63 | 9.3.0 | HotReloadable trait + register_hot_reloadable | 712→716 | 4/4 |
| #64 | (docs) | 80-finding hardening coverage ledger | — | 4/4 |
| #65 | (docs) | REFERENCE.html refresh to v9.3 | — | 4/4 |

### main lineage after the loop
`6033a7f`(#65) → `1545c95`(#64) → `73a4106`(#63) → `b706682`(#62) → `9997841`(#61) → `07ddd3c`(seq-2 handoff) → `7e48794`(v9.0.0, #60)

### Appendix-A audit tally (at v9.3.0)
| Batch | ADDRESSED | PARTIAL | NOT |
|---|---|---|---|
| 1–20 | 17 | 3 (11,13,18) | 0 |
| 21–40 | 20 | 0 | 0 |
| 41–60 | 17 | 3 (48,53,59) | 0 |
| 61–80 | 15 | 2 (65,72) | 3 (62,73,76) |
| **Total** | **69** | **8** | **3** |

(#13≡#18 duplicate ⇒ 79 unique issues. 86% fully closed, 96% touched, 100% of HIGH+correctness-MEDIUM. Exceeds the analysis's own ~80% prediction.)

### Audit method (reproducible)
Each of the 4 agents was given its 20 findings as `(#, location-hint, item)`, told to locate by symbol/grep (line numbers from `42de46c` have moved), and to verdict ADDRESSED/PARTIAL/NOT with **current `file:line` + fix mechanism** evidence. The ledger (`docs/CODE_ANALYSIS_2026-06-16_COVERAGE.md`) carries the full 80-row verdict list — not re-derived here. Notable confirmations from the audit (do NOT re-investigate): blob_47 mask table regenerated + tested (#1); gamepad axis bindings now honored (#2); `#[non_exhaustive]` on both gamepad enums (#25); `Focused(false)→release_all` (#28); offscreen-RT clear_color honored (#39); camera shake in screen/world transforms (#40); behavior child.reset on completion (#41); blocked-goal pathfinding returns None (#43); z-order + visibility text-input focus (#44/#46); `SolidTiles::Only` HashSet (#68); ordered_pair on both contact + intersection pairs (#70); save `*_with_key` variants (#78); LocalizationSystem `TextInput.placeholder` (#79); Slider `set_field` thumb sync (#80).

### Residual 11 (all LOW-priority — candidates if a perf/API pass opens)
| # | Verdict | One-liner |
|---|---|---|
| 11 | PARTIAL | data_table now warns on extra columns; still discards (schema row-0 by design) |
| 13/18 | PARTIAL | TilemapSystem generation-guard skips diff; full struct clone remains |
| 48 | PARTIAL | FadeTransition wasm no-op documented; no runtime warn (intended LOW fix) |
| 53 | PARTIAL | audio fade stack-buffer for ≤8; to_stop/overflow still alloc on completion |
| 59 | PARTIAL | query_added/changed empty fast-path added; non-empty still allocs |
| 65 | PARTIAL | NetworkClient Drop fixes thread leak; no reconnect API (medium-effort) |
| 72 | PARTIAL | sprite scratch Vecs are fields; 2 per-frame HashSets remain |
| 62 | NOT | editor-only pathfinding overlay clones tile grid per frame |
| 73 | NOT | no frustum pre-filter before nearest-16 light cull (WU6 over-claimed) |
| 76 | NOT | SteeringSystem collects 5 Vec<Entity>/frame (LOW; mirror #67 to fix) |

### Per-PR diff + CI (native test job dominates wall-clock)
| PR | files | +/− | native test | wasm | rustdoc | pkg |
|---|---|---|---|---|---|---|
| #61 | 8 | +272/−14 | 6m25s | 38s | 50s | 1m16s |
| #62 | 7 | +587/−61 | 5m32s | 38s | 45s | 1m6s |
| #63 | 11 | +224/−19 | 6m8s | 41s | 43s | 1m8s |
| #64 | 1 | +144/−0 | 4m2s | 35s | 55s | 1m5s |
| #65 | 1 | +339/−6 | 4m14s | 32s | 41s | 1m23s |

### Test progression (lib)
698 (hardening baseline) → 706 (#61, +8 serde round-trips) → 712 (#62, +6 SM ops) → 716 (#63, +4 HotReloadable). Docs PRs add none.

## Reusable Engineering Gotchas (this session)

- **opus/Sonnet gate division that worked:** the impl agent self-runs `cargo test --lib` + `cargo clippy --all-targets -- -D warnings` (the `--all-targets` is essential — it compiles the native editor UI that `--lib` skips) + (for native-only features) `cargo build --target wasm32-unknown-unknown` to catch a native-only leak; opus then runs the FULL Gate6 (`./scripts/verify.sh` adds `fmt --check`, `test --all-targets`, `doc -D warnings`). Per-iteration `--lib`-only misses editor-UI + wasm regressions.
- **Read-only work ⇒ parallelize foreground-concurrent; mutating work ⇒ background-single.** The 80-finding audit ran as **4 concurrent foreground Sonnet agents** (no file writes ⇒ no conflict ⇒ synthesize when all return in one batch). Impl agents ran **background** (each auto-wakes the loop on completion) + a long `ScheduleWakeup(1800s)` safety-net fallback.
- **Docs-only PRs need no version bump** (#64, #65) but still go through PR+CI for traceability; the rustdoc/package gates don't touch the new docs, so they pass trivially.
- **CI does NOT verify REFERENCE.html (HTML is never built) or the editor UI (the windowed app is never run).** Autonomous substitute: structural HTML checks (tag balance, `id` uniqueness, `href↔id` match) + spot-check documented symbols against source; for the editor, unit-test the data paths through the real handlers and flag the visual layer for human eyeball.
- **Appendix-A line numbers drift:** they cite commit `42de46c`; agents were told to locate findings by **symbol/grep, not line number**. Every "moved" finding was still found.
- **A plan can over-claim:** WU6's note listed "frustum prefilter" but only the viewport-center cull (#38) actually shipped (#73 NOT-ADDRESSED). The 1:1 audit caught a gap the area-level v9.0.0 review missed — line-by-line audits earn their cost on exactly this.
- **Stale "deferred" lists mislead:** the 2026-06-15 deferred-candidates handoff listed data-driven assets / editor tile-painting / RTL as "still-deferred" — all had since shipped. Verify a handoff's deferred list against current code before planning from it.
- **rust-analyzer phantoms persisted all session** (ColliderHandle E0308, sprite.rs E0308/E0107, inactive-cfg, unlinked-file) — every agent's `cargo`/CI was clean. Trust the compiler, not the IDE snapshot (carried, reconfirmed).
- **same-name trait/inherent method:** the 3 registries have an inherent `reload_path` AND impl `HotReloadable::reload_path`; the trait impls call the inherent via **UFCS** (`DataTableRegistry::reload_path(self, path)`) to avoid recursion ambiguity, validated by a no-infinite-recursion test.

## Code Analysis

- **New public APIs (v9.1–9.3):**
  - `AnimationStateMachine::set_transition_conditions(&mut self, from:&str, index:usize, conditions:Vec<TransitionCond>) -> bool` / `set_transition_crossfade(&mut self, from:&str, index:usize, seconds:f32) -> bool` — false on missing state / OOB index.
  - `engine::HotReloadable { fn reload_path(&mut self, path:&str); }` + `App::register_hot_reloadable::<T: HotReloadable>(&mut self)` (native-only). Internals: `fn forward_hot_reload<T: HotReloadable>(world:&mut World, paths:&[String])` stored as `fn(&mut World,&[String])` in `App.hot_reload_forwarders: Vec<...>`.
  - serde derives on the SM + Timeline + Easing type families; auto-registration via `registry.register::<T>("Name", None)` in `register_core_component_metadata`.
- **`timeline_track_ui<T: Clone + Lerp>` new signature:** `(ui, label, track, at_time: f32, make_default: impl Fn()->T, value_edit: impl Fn(&mut Ui,&mut T)->bool)` — the `make_default`/`value_edit` closures let the generic track UI add keyframes + edit per-type values without a `Default`/widget bound.
- **Font system (for the deferred #5):** `src/renderer/text.rs::build_font_system(font_data, extra_fonts)` loads everything into one cosmic-text `FontSystem`; the prepare path hardcodes `Attrs::new().family(Family::SansSerif)`; `LocaleResource::font()` returns a **filename** (`src/locale.rs`), `TextDirection`/`direction()` exist. Per-locale selection would un-hardcode the family + map filename→cosmic-text family.
- **Borrow pattern (editor panels):** snapshot display data under `world.get::<T>` → collect `Edit` intents into a `Vec` during the egui render closure → apply under one fresh `world.get_mut::<T>` afterward. The agents followed it; it's the canonical editor-UI idiom here.

## Files Changed

### Source (via #61–#63, all merged)
- `src/animation/state_machine.rs` — serde+PartialEq derives; `set_transition_conditions`/`set_transition_crossfade` + tests.
- `src/timeline.rs` — serde derives on Timeline/Track/Keyframe/CameraTarget + round-trip tests.
- `src/tween.rs` — `Easing` serde derive.
- `src/app/core_resources.rs` — auto-register SM/Timeline/CameraTarget for serde.
- `src/app/editor/state.rs` — transient add-transition UI state (`sm_add_trans_target`/`sm_add_trans_xf`).
- `src/app/editor/ui/docked.rs` — SM panel depth + `timeline_track_ui` rewrite (+528 lines).
- `src/asset.rs` — `HotReloadable` trait.
- `src/data_table.rs`, `src/animation/clip_set.rs`, `src/particle/config_set.rs` — trait impls for the 3 registries.
- `src/app.rs` — `hot_reload_forwarders` field, `register_hot_reloadable`, `forward_hot_reload`, auto-register in `App::new`, tests.
- `src/app/schedule.rs` — `forward_reloads!` macro → forwarder loop.
- `src/lib.rs` — re-export `HotReloadable`.

### Docs (via #64/#65 + version bumps in each PR)
- `docs/CODE_ANALYSIS_2026-06-16_COVERAGE.md` — NEW, the 80-finding ledger.
- `REFERENCE.html` — blockquote + 10 sections + TOC (+339/−6).
- `docs/CHANGELOG.md`, `Cargo.toml`, `CLAUDE.md` — 9.1.0/9.2.0/9.3.0 entries + header bumps + module-map updates.
- `plans/code-analysis-2026-06-16-progress.md` — RESOLVED banner (uncommitted; rides this handoff's commit).

### Memory
- `engine-current-state.md` — rewritten to a concise v9.3.0 snapshot.

## User Feedback & Preferences (REQUIRED)

- **"작업 해야하는 내용 우선순위대로 리스트 만들어봐"** — wanted a prioritized backlog list before acting (delivered the 4-tier list).
- **"예정 작업 진행. opus 감독으로 sonnet 실무. 완료되면 테스트 후 머지하고 handoff 스킬로 문서정리 하고 완료사항 보고"** — the loop charter: opus supervises, Sonnet implements, test→merge each, then handoff + report. **Merge authority explicitly granted for this loop.**
- **Standing (carried):** Korean prose to the user, English code/docs/handoff; subagents on Sonnet with explicit `model:` ([[new-model-subagent-incompat]]); never tag unprompted (lapsed after v4.3.0); `set_scene` resets world (use register_persistent / serde auto-registration); rust-survivors dropped as a consumer.
- **Engagement style:** the user delegated the completion-point to opus ("완료시점까지"-style) — supervisory judgment to defer #5 and not force perf-refactors was within scope.

## Where We're Going

The priority loop is drained of its tractable, autonomously-verifiable items. Remaining backlog (none blocking):

1. **Per-locale font auto-select (#5)** — needs a **display-access session** (human-eyeball verification). Detailed implementation sketch (so the next session doesn't re-investigate the font system):
   - **Current state:** `TextRenderer::prepare` hardcodes `let default_attrs = Attrs::new().family(Family::SansSerif);` (~`src/renderer/text.rs:468`). All fonts (base `FontData` + `ExtraFonts` blobs) are loaded into one cosmic-text `FontSystem` by `build_font_system`; multi-script works today via cosmic-text **per-glyph fallback** (that's how `rtl_text` renders Hebrew). `LocaleResource::font()` returns a **filename** string (e.g. `"NotoSansArabic.ttf"`); `LocaleData.direction`/`TextDirection` exist. `DrawText` has NO per-text font field.
   - **The mismatch:** cosmic-text selects by `Family::Name("<family>")`, not by filename. `LocaleData.font` holds a filename. So you must either (a) add `LocaleData.font_family: Option<String>` (additive) holding the real family name, or (b) resolve filename→family from the loaded `fontdb` faces.
   - **Plumbing:** (1) un-hardcode the default `Family` — read it from a renderer-visible resource (e.g. a new `DefaultFontFamily(Option<String>)` resource) that the prepare path consults; (2) a small system (in `src/ui/localized.rs` or locale) sets that resource from the active `LocaleResource`'s family each frame / on locale change; (3) optional per-`DrawText` override field for explicit cases.
   - **Why it's a defer, not a gap:** script fallback already covers the real i18n need; this is a family-*preference* refinement, and the rendered result is **not autonomously verifiable**. Validate with `cargo run --example rtl_text` + a live locale switch + eyeball.
2. **SM visual node-graph + Timeline time-ruler (iteration-2 visual editors)** — list-MVPs shipped; the visual layer is egui-painter work that the docked cursor-freeze makes autonomously-unverifiable. Pair with unit-tested hit-test helpers + human eyeball.
3. **Optional perf/API cleanup** closing the residual audit gaps — #72 + #76 (scratch-field promotion, mirror the shipped #67 physics fix), #65 (reconnect API), #73 (frustum pre-filter). All LOW-priority.
4. **`TilemapAutotile { mode: Single|Multi }` unification + drop ghost `ConnectRule`** — breaking → next v10 window.
5. **A new feature + playable example** per `docs/VISION.md` (breadth is "complete"; this would be net-new or depth — needs user direction).

## Risks & Blockers

- **None blocking.** `main` green + clean at v9.3.0.
- **#62 (editor depth) and #65 (REFERENCE.html) were verified by compile/clippy/structural checks, NOT by visual rendering** — CI does not run the windowed editor or build the HTML manual. A human eyeball pass on the SM/Timeline panel layout and the REFERENCE render is worthwhile (the PRs note this). All *data* paths in #62 are unit-tested.
- **#5 (per-locale fonts) cannot be validated autonomously** — it's the reason it was deferred, not a regression.

## Open Questions

- Tag `v9.3.0` (and backfill v9.0–9.2)? Default: no (tagging lapsed).
- Is per-locale font auto-select worth a dedicated display session, or leave it (script fallback already covers i18n)?
- Next direction: perf cleanup, the visual editors, or a new VISION feature+example? — user picks.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -6        # expect 6033a7f (#65) → ... → 9997841 (#61)
grep -m1 '^version' Cargo.toml   # 9.3.0
git status -s               # clean (this handoff + progress-banner committed)

# Context (read in order)
#   plans/handoffs/HANDOFF_engine-hardening_post-v9-priority-loop_2026-06-16.md   (this, seq 3)
#   docs/CODE_ANALYSIS_2026-06-16_COVERAGE.md   (the 80-finding ledger)
#   CLAUDE.md (Gate6 + module map) · docs/VISION.md (what-next)

# Key files if resuming the deferred items
#   src/renderer/text.rs        (#5 per-locale font: hardcoded Family::SansSerif in the prepare path)
#   src/locale.rs               (#5: LocaleData.font filename, TextDirection)
#   src/app/editor/ui/docked.rs (SM/Timeline panels — iteration-2 visual editors)

# Verify current state
./scripts/verify.sh         # full Gate6 — confirm main still green

# Next action — pick ONE (none blocking):
#   (a) per-locale font auto-select (#5) — DISPLAY-ACCESS session (human eyeball), spec in this handoff
#   (b) SM/Timeline visual editors (iteration 2) — egui-painter + hit-test helpers
#   (c) a new feature + example per docs/VISION.md
#   For any engine work: plans/<name>_plan.md (criteria) → impl → Gate6 → unit-test through real handler → example → PR → merge (re-confirm merge authority in a NEW session).
```

## Session Closed
**Closed at:** 2026-06-16
**Commit:** the `session: post-v9-priority-loop handoff [engine-hardening]` commit on `main`
**Session status:** Handed off to next session — post-v9.0.0 priority loop COMPLETE (5 PRs merged, v9.3.0)
