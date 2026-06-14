# Editor-GUI arc seq 3 — engine-wide adversarial bug-hunt (2 passes, converged): 16 fixes shipped v8.1.5→v8.1.9

**Date:** 2026-06-15
**Status:** COMPLETED (2-pass convergence; all 8 PRs merged to `main`)
**Bead(s):** none (`bd` not installed — `command not found`; tracked in-session)
**Epic:** Editor GUI → engine hardening — a GUI for game devs, then a full-engine reliability sweep
**Chain:** `editor-gui-arc` seq `3`
**Parent:** `HANDOFF_editor-gui-arc_playtest-bugfix-sweep_2026-06-14.md` (seq 2)
**Prior chain:** `HANDOFF_editor-gui-arc_docked-editor-to-data-editor_2026-06-13.md` (seq 1) > `HANDOFF_editor-gui-arc_playtest-bugfix-sweep_2026-06-14.md` (seq 2) > this (seq 3)

---

## Since Last Handoff

Seq 2 fixed the editor-GUI bugs found by live playtest (v8.1.2–v8.1.4, the game-data-editor registration-loss + 10 editor-reliability bugs) and listed under "Where We're Going": #1 PR #27 merge, #2 optional follow-ups, **#3 "extend the hunt to other subsystems (physics/render/net)"**. This session (seq 3) executed exactly #3 — an autonomous `/loop` engine-wide adversarial bug-hunt across every remaining subsystem.

- Seq 2's #3 (extend to other subsystems) → DONE, and then some: **two full passes** over the whole engine, 16 more bugs found+fixed (v8.1.5–v8.1.9), all merged.
- Seq 2's open question "does the user actually publish to crates.io" → still unresolved (Package dry-run gate stays green; no publish attempted).
- Seq 2's editor area stayed fixed — no regressions surfaced in the engine-wide sweep of editor-adjacent code.
- The user kept re-issuing the same `/loop` prompt after each completion ("계속 진행해" / re-paste), driving the hunt to genuine loop-until-dry convergence rather than stopping at the first pass.
- New gotchas discovered this session (cargo stale binaries, synthetic-Ctrl+Z, breaking-API-in-patch, stacked-PR CI) — see Risks.

## Reference Documents

- `CLAUDE.md` (header now v8.1.9) — module map, `./scripts/verify.sh` gates, VISION loop.
- `docs/CHANGELOG.md` — `## 8.1.5` … `## 8.1.9` all written this session (per-bug descriptions).
- `docs/VISION.md` — "a feature is not done until a playable example exercises it" (the principle that started this whole arc in seq 2).
- Memory: `engine-current-state` (rewritten to v8.1.9 / hunt converged), `subagent-usage-preference`, `new-model-subagent-incompat`, `conversation-language-korean`, `playtest-windowed-examples`, `ci-toolchain-pin`.

## The Goal

After the editor-GUI arc, the user wanted "버그 모두 수정" (fix ALL bugs) under an autonomous `/loop`: opus supervises + verifies + commits, sonnet subagents do all implementation, report-and-stop only at a point that fails 3+ times, otherwise opus decides. The objective became a comprehensive adversarial bug-hunt over the ENTIRE engine (not just the editor), driven to convergence (loop-until-dry), with every confirmed bug fixed, tested, and shipped as a CI-green patch release the user merges per-PR.

## Where We Are

- **`main` = `00de9e8`** (Merge #32), version **8.1.9**, working tree CLEAN. All 8 session PRs merged; all fix branches deleted (local + remote). Pre-existing stale branches from PRIOR arcs remain (feat/v7.1-docked-editor, feat/v8-scene-layout-editing, feat/v8.1-data-editor, docs/english-conversion, fix/macos-mainthread-pacing) — not this session's, user can prune.
- **Two full hunt passes, converged.** Pass 1 (per-subsystem breadth): ~25 bugs across seq-2 editor (11) + seq-3 engine (14 in v8.1.5–v8.1.8). Pass 2 (new angles: app-loop + concurrency/WASM/panic): only 2 Low bugs (v8.1.9) + 1 entirely-clean sweep. Findings declined 25 → 2 = strong convergence.
- **16 engine bugs fixed this seq (v8.1.5–v8.1.9):** see the bug table in Evidence.
- **Test count:** 457 (seq-2 end) → 463 (v8.1.5) → 468 (v8.1.6) → 475 (v8.1.7) → 481 (v8.1.8) → 481 (v8.1.9, GPU surface path not headlessly testable). +24 regression tests this seq.
- **Every subsystem swept** (most CLEAN — codebase is well-vetted): editor, ECS/world, rendering, physics, collision, animation, skeletal, audio, particles, tilemap, pathfinding, behavior, steering, UI, asset, save, scripting, timeline, tween, network, app-loop/window, concurrency, WASM divergence, panic-safety.
- **Governance held perfectly:** opus ran `./scripts/verify.sh` (+ per-merge main CI) independently before every commit; sonnet did all code (≈22 subagents: ~13 review sweeps + 9 fix agents); never self-merged (each PR waited for the user's "#N 머지 확인"); 0 three-failure points.
- **Deferred (Low/by-design, NOT fixed — recorded so they aren't re-chased):** Rhai-ctx-panic (documented contract / misuse-only), Rhai-scope-growth (design-debatable), atlas-hot-reload grid rebuild (Low feature-gap), failed-load watcher (Low edge).

## What We Tried (Chronological)

This seq ran as repeated `/loop` iterations; each round = parallel sonnet review sweeps → opus verifies findings in code → sonnet fix agent → opus verify+commit → PR → user "머지 확인".

1. **Round 1 — core ECS + rendering.** Two parallel sweeps. ECS almost entirely CLEAN (generation-checked entity ids, archetype swap-remove sync, events, schedule). Rendering CLEAN (sort key, batching, UV, RT camera, 16-light cull, NDC). Found 2: `SceneCmd::Pop` panic-index aliases the builtin tail (High), `DrawText::centered` half-width wrap (Med) → **v8.1.5 (#28)**.
2. **Round 2 — physics/collision + animation/skeletal.** Physics CLEAN except 2 (Stopped-on-despawn Med, cast_ray ghost Low); animation CLEAN except 2 (1-frame clip is_finished Med, BlendTree stuck Med) → **v8.1.6 (#29)**. The cast_ray fix was first written `&self`→`&mut self` (a BREAKING API change); opus caught it in review and reworked it non-breaking (refresh pipeline in `remove_body`) before shipping.
3. **Round 3 — audio/particles/tilemap + pathfinding/behavior/steering.** Particles/pathfinding/steering CLEAN. Found 3: audio bus-volume applied twice during fades (High), `AlwaysSucceed` swallows Running (Med), tilemap tile-id out-of-range UV (Med) → **v8.1.7 (#30)**.
4. **Round 4 — UI runtime + asset/save/scripting.** UI mostly CLEAN (hit-test coords, slider mapping, TextInput byte-safety, scroll clamp, layout, localization); save/scripting CLEAN (AEAD nonce random per-save, op-limit, error isolation). Found: overlapping-button both fire (Med), scrollview item_height=0 panic (Low), slider double-event (Low), save_path traversal (Low). → part of **v8.1.8**.
5. **Round 5 — timeline/tween/network (final pass-1 cluster).** Almost entirely CLEAN (keyframe clamp, tween easing/completion, SnapshotBuffer, RemoteEntities, network thread shutdown). Found 1: looping timeline single-subtract vs modulo on dt>duration (Low). Bundled into **v8.1.8 (#31)** = 5 fixes (4 UI/save + 1 timeline).
6. **Pass 2, Round 6 — app main-loop/window/render-orchestration + concurrency/WASM/panic.** App-loop sweep: 2 Low (surface Occluded/Timeout log-spam, Suboptimal not reconfigured). Concurrency/WASM/panic sweep: **entirely CLEAN** (network thread, audio, async channels, SCRIPT_CTX re-entrancy, WASM divergence, panic-safety on malformed RON/save/network/script) → **v8.1.9 (#32)**.
7. **Convergence declared.** Pass-2 yielded only 2 Low + 1 clean sweep → loop-until-dry satisfied. Loop stopped.

## Key Decisions

- **opus verifies every finding in code before dispatching a fix** (cloud reviews have ~20% error rate). This dismissed several agent candidates (Track-1 "unbounded growth" was unreachable since register_* are App methods; Rhai-ctx-panic is a documented contract; multiple "clean" confirmations).
- **opus independently re-runs `verify.sh` before every commit; never trusts agent self-reports or IDE diagnostics.** Caught the breaking `&self`→`&mut self` in the cast_ray fix; dismissed a phantom stale rust-analyzer `TypeId` error (cargo compiled clean).
- **Patches must stay non-breaking** (engine does breaking only in major bumps). The cast_ray ghost fix was reworked from `&mut self` (breaking) to `remove_body`-refreshes-pipeline (non-breaking).
- **Bundle fixes per cluster into one patch release; stack PRs when prior isn't merged.** v8.1.6 sweep stacked on v8.1.5 (depended on it); later PRs went off main as prior merged. Stacked PR base set to the prior fix branch (clean diff), retargeted to main on merge.
- **Deferred Low/design items rather than over-engineer.** Rhai-ctx-panic (contract), Rhai-scope (design), atlas-reload/watcher (Low) left noted-not-fixed — forcing fixes on design-debatable behavior is worse than the gap.
- **Stop at 2-pass convergence.** A third full pass was judged near-certain-dry and not worth the cost; offered to the user instead of spending unprompted.
- **Never self-merge.** Every one of the 8 PRs waited for the user's explicit "#N 머지 확인".

## Evidence & Data

### PR / release ledger (this seq)

| Release | PR | Cluster | Bugs | Merge | CI |
|---|---|---|---|---|---|
| 8.1.5 | #28 | ECS + rendering | 2 | `cf5c5e1` | 4/4 |
| 8.1.6 | #29 | physics + animation | 4 | `c460ef1` | 4/4 |
| 8.1.7 | #30 | audio + behavior + tilemap | 3 | `767d561` | 4/4 |
| 8.1.8 | #31 | UI + save + timeline | 5 | `168c1d9` | 4/4 |
| 8.1.9 | #32 | render surface-error | 2 | `00de9e8` | 4/4 |

(Seq 2, already merged before this seq: v8.1.2 #25, v8.1.3 #26, v8.1.4 #27 — editor 11 bugs.)

### Bugs fixed this seq (16)

| # | Release | Bug | Severity |
|---|---|---|---|
| 1 | 8.1.5 | `SceneCmd::Pop` retained panic index aliases builtin tail (HierarchySystem) → GlobalTransform propagation silently stops | High |
| 2 | 8.1.5 | `DrawText::centered` no-bounds buffer width = `w - position.x` (half) → premature wrap | Med |
| 3 | 8.1.6 | `CollisionEvent::Stopped` dropped when a contacting entity despawns (handle gone from per-frame map) | Med |
| 4 | 8.1.6 | `cast_ray` hits a body removed earlier the same frame (query pipeline stale until step) | Low |
| 5 | 8.1.6 | 1-frame non-looping clip `is_finished()` at entry → AnimationEnd state exits same frame | Med |
| 6 | 8.1.6 | BlendTree1D stuck on crossfade target after a param reversal poisons `last_clip` | Med |
| 7 | 8.1.7 | Audio bus volume applied twice during fades (`base × bus²`) → pop + wrong rate | High |
| 8 | 8.1.7 | `AlwaysSucceed` swallows `Running` → multi-frame child abandoned frame 1 | Med |
| 9 | 8.1.7 | `TilemapAtlas::uv_for` out-of-range tile id → UV outside [0,1] (garbage tile) | Med |
| 10 | 8.1.8 | Overlapping `Button`s both fire `ButtonClicked` on one click | Med |
| 11 | 8.1.8 | `ScrollView` `item_height==0` → `usize::MAX + 1` debug panic | Low |
| 12 | 8.1.8 | `Slider` emits two `SliderChanged` (different values) on the press frame | Low |
| 13 | 8.1.8 | `save_path` path traversal (`../..` escapes data dir) | Low (sec) |
| 14 | 8.1.8 | Looping `Timeline` single-subtract on `dt>duration` → multi-frame stutter | Low |
| 15 | 8.1.9 | Surface `Occluded`/`Timeout` → `log::error!` spam every frame while minimized | Low |
| 16 | 8.1.9 | `Suboptimal` surface not reconfigured → degradation after DPI/monitor change | Low |

### Sweep convergence

| Pass | Round | Sweeps | Real bugs | Notable CLEAN (do NOT re-investigate) |
|---|---|---|---|---|
| 1 | 1 | ECS+lifecycle, rendering | 2 | entity-generation ids, archetype swap-remove, events, schedule, sort/batch/UV, RT camera, 16-light cull |
| 1 | 2 | physics+collision, animation+skeletal | 4 | joint handles, tile colliders, one-way, CCD, spatial grid, skeletal bone order, state-machine resumption |
| 1 | 3 | audio+particles+tilemap, pathfinding+behavior+steering | 4 | particle pool/burst, A* heuristic admissibility, steering NaN guards, BT Sequence/Selector resume |
| 1 | 4 | UI runtime, asset+save+scripting | 4 | hit-test coords, TextInput byte-safety, scroll clamp, AEAD nonce (random/save), op-limit, script error isolation |
| 1 | 5 | timeline+tween+network | 1 | keyframe clamp, tween easing/completion, SnapshotBuffer <2-sample, RemoteEntities, thread shutdown-by-channel-drop |
| 2 | 6 | app-loop+window+render-orch, concurrency+WASM+panic | 2 | resource init order, scale-factor, resize guard, WaitUntil pacing, SCRIPT_CTX re-entrancy, panic-safety on malformed data, WASM divergence — **concurrency/WASM/panic sweep entirely clean** |

### Verified Clean (do NOT re-investigate — a next sweep should skip these)

Each was traced in code by a sweep and confirmed correct/safe. Recording so a future bug-hunt doesn't re-chase them:

**ECS / lifecycle:**
- `Entity` is generation-checked — despawn increments generation before recycling the index, so a stale handle fails `entity_location.get`. No id-reuse aliasing.
- Archetype `swap_remove` keeps every column length synced with `entities`; `add_component` migration `binary_search().unwrap_err()` is safe (early-return guarantees absence).
- `Commands` applied after query iterators drop; Kahn-sort schedule deterministic; `Events<E>` flush happens before scene-transition drain.

**Rendering:**
- Sort key `(layer, z, monotonic-order)` has no ties → no flicker; instance-run detection requires matching texture AND consecutive offset; `layer_matches_mask` excludes out-of-`0..31` under a non-zero mask.
- `UvRect::from_grid` exact (no half-texel); each `OffscreenCamera` submits its own encoder (no camera-uniform race); `merge_textures_delta` survives skipped frames; lighting nearest-16 via `select_nth_unstable`; NDC↔UV conventions consistent.

**Physics / collision:**
- Joint handles opaque (no double-free via public API); `remove_body` purges `one_way_colliders` before rapier remove (no handle-reuse aliasing); CCD enabled in `step`.
- Contact-pair order stable (rapier guarantee), sensor pairs normalized via `ordered_pair`; spatial grid `rebuild` clears first + inserts cell-straddling entities into all overlapping cells; drop-through `drop_active` snapshotted before the timer decrement.

**Animation / skeletal:**
- Non-looping clips clamp at the last frame + break the drain loop; `current_uv` is `.get().unwrap_or(FULL)`.
- State-machine transitions registration-ordered, triggers consumed each frame, Inverter passes Running through; skeletal writes LOCAL transforms (HierarchySystem composes global), keyframe interp clamps, missing-bone `filter_map`-skipped, `lerp_angle` shortest-path.

**Audio / particles / tilemap:**
- `set_volume`/`set_bus_volume` clamp `[0,1]`; positional `max_dist.max(0.001)`; SFX cache keyed by path; sink removed before reinsert (no stale-handle control of a new sound).
- Particle age despawn-same-tick, spawn accumulator no drift, one-shot burst fires once, particles are independent ECS entities (no orphan); tilemap `tile_id==0` sentinel skipped, tile-center formula correct.

**AI (pathfinding / behavior / steering):**
- A* 4-dir Manhattan heuristic admissible (no suboptimal / no corner-cut), blocked-goal / unreachable / start==goal handled, `index()` bounds-checks, reconstruction excludes start + includes goal.
- BT Sequence/Selector resume at the running child + reset on terminal; steering Seek/Flee/Arrive guard `length_squared > 1e-6` / radius ranges (no NaN, no div0).

**UI:**
- Hit-test uses `screen_pos(viewport)` (anchor-resolved); click fires on release only; Disabled skipped.
- Slider `set_normalized` clamps + `.max(EPSILON)` guards `max==min`; TextInput caret ops all walk to `is_char_boundary` (Hangul-safe); scroll `clamp_scroll` before index calc; LayoutSystem-before-UiSystem; LocalizedText missing-key → key string.

**Save / scripting:**
- AEAD nonce is a fresh `thread_rng().fill_bytes` per save (NO reuse — security-checked); `load_or_default` maps only `NotFound`→default (propagates Corrupted/Ron); `decrypt` length-guard correct; RON parse propagates errors (no unwrap).
- Script errors isolated per-entity; `set_max_operations(1_000_000)` (no infinite-loop hang); per-entity scope/buffers cleared each iteration.

**Timeline / tween / network:**
- Keyframe clamp before-first/after-last + single-keyframe path; tween zero-duration → fraction 1.0 (no div0), completion fires once, Back-easing overshoot is documented intent.
- `SnapshotBuffer` <2-sample clamps (loop `0..0` empty), out-of-order rejected, capacity-bounded; `RemoteEntities::remove` leaves no stale entry; native net thread exits via channel-drop (no join needed); `RemoteEntities::clear`-on-disconnect is the game's documented responsibility.

**App / concurrency / WASM / panic-safety:**
- Resource init order correct; scale-factor read per-frame; resize-to-0 guarded; WaitUntil clamp 60–240 Hz.
- SCRIPT_CTX non-reentrant (Rhai can't re-borrow → no double-borrow panic); WASM_LOGICAL_SIZE division guarded `≥ 1`; channel `send` after receiver-drop is `let _ =` (no panic); panic-safety on malformed save/network/RON + skeletal/timeline indexing all bounds-guarded.

## Code Analysis

- **`SceneCmd::Pop` fix (src/app/scenes.rs):** `panicked_systems.retain(|&i| i < new_scene_len)` (was `new_len = new_scene_len + tail`). The drained scene's first index equals `new_scene_len`, which after the drain aliases the tail (HierarchySystem); retaining `< new_scene_len` drops drained + tail indices (tail gets a clean retry, consistent with `reload_scene` clearing the set).
- **`CollisionEvent::Stopped` fix (src/physics/system.rs):** added `prev_col_map`; `diff_pairs` resolves "stopped" pairs via current map then falls back to `prev_col_map` (despawned entity's handle still resolvable). `prev_col_map.clone_from(&col_map)` at frame end.
- **`cast_ray` fix (src/physics/world/body_factory.rs):** `remove_body` calls `self.query_pipeline.update(&self.collider_set)` immediately (rapier 0.22 single-arg). `cast_ray`/`cast_ray_with_normal` stay `&self` (the `&mut self` lazy-dirty approach was reverted as breaking).
- **`is_finished` fix (src/animation/player.rs + system.rs):** `AnimationPlayer.finished` flag, set by `AnimationSystem` when the non-looping advance reaches past the last frame; reset on `play()`/`play_with_crossfade()`. `is_finished()` returns the flag.
- **BlendTree fix (src/animation/blend_system.rs):** the "already on target clip" branch guarded with `!player.is_crossfading()` (during A→B crossfade `current_clip` is the FROM `A`; a param reversal to A must defer, not set `last_clip`).
- **Audio fade fix (src/audio/playback.rs):** `fade_start_vol` returns the PRE-bus base volume; `update()` applies `vol * bus_vol` once. (Was `base × bus` start_vol × bus again = `base × bus²`.)
- **Surface fix (src/app/render.rs):** wgpu 29 uses `CurrentSurfaceTexture` enum (NOT `SurfaceError`; `OutOfMemory` removed). `Success`/`Suboptimal(t)` → use texture (suboptimal→`gpu.reconfigure()` after present); `Occluded | Timeout` → silent skip; `Lost | Outdated` → reconfigure; `Validation` (+ others) → `log::error!`.
- **save_path fix (src/save.rs):** `sanitize_path_component` keeps only `std::path::Component::Normal` (strips `ParentDir`/`RootDir`/`Prefix`); legit subdirs preserved.
- **Overlapping-button fix (src/ui/system/button_pass.rs):** two-pass refactor — pass 1 updates each button's visual state + collects a single `click_candidate: Option<(Entity, f32)>` (max-z winner), fires ONE `ButtonClicked` after the loop; pass 2 renders. A `// TODO` notes cross-widget pointer-consumption (a button under a *different* widget type) is still open — left out of scope (needs a shared consume mechanism).
- **ScrollView fix (src/ui/system/scroll_view_pass.rs):** guard `if item_height <= 0.0 { push bg; continue; }` before the `size.y / item_height` division (was `inf.ceil() as usize == usize::MAX`, `+1` overflow → debug panic).
- **Slider fix (src/ui/system/slider_pass.rs):** the held/drag recalculation is now an `else` of the press branch, so the press frame emits only the press-driven `SliderChanged` (was press + same-frame drag = two events, different values).
- **Timeline fix (src/timeline.rs):** loop wrap `if tl.duration > 0.0 { tl.time %= tl.duration }` (was `tl.time -= tl.duration`, a single subtract that left `time` past the end for several frames on `dt > duration`). The `duration > 0` guard avoids `% 0.0 == NaN`.

## Files Changed (this seq — all merged to `main`)

### Source
- `src/app/scenes.rs` — Pop panic-index retain bound.
- `src/renderer/text.rs` — centered/anchor-aware layout buffer width/height helpers.
- `src/physics/system.rs` — `prev_col_map` for Stopped-on-despawn.
- `src/physics/world/body_factory.rs` — `remove_body` refreshes query pipeline.
- `src/animation/{player.rs, system.rs}` — `finished` flag.
- `src/animation/blend_system.rs` — `!is_crossfading()` guard.
- `src/audio/playback.rs` — pre-bus fade start volume.
- `src/behavior.rs` — `AlwaysSucceed` passes Running through.
- `src/tilemap.rs` — `uv_for` out-of-range guard.
- `src/ui/system/{button_pass.rs, scroll_view_pass.rs, slider_pass.rs}` + `src/ui/system.rs` — top-most-button click, item_height guard, slider press-frame.
- `src/save.rs` — `save_path` sanitization.
- `src/timeline.rs` — modulo loop wrap.
- `src/app/render.rs` + `src/app/schedule.rs` — surface-error handling.

### Tests (+24 regression tests this seq — each fails without its fix; run to confirm no regression)
- v8.1.5: `scene_pop_does_not_alias_builtin_tail_panicked_index` (app.rs); `centered_no_bounds_uses_full_viewport_width`, `top_left_no_bounds_subtracts_position`, `explicit_bounds_override_anchor`, `top_left_position_beyond_viewport_clamps_to_zero`, `centered_no_bounds_does_not_use_half_viewport` (renderer/text.rs).
- v8.1.6: `stopped_event_emitted_when_contacting_entity_despawns` (physics/system tests); `cast_ray_no_hit_after_remove_body_before_step` (physics/world/tests.rs); `one_frame_nonlooping_clip_not_finished_at_entry`, `multi_frame_nonlooping_clip_finished_after_last_frame_shown` (animation/player); `param_reversal_during_crossfade_does_not_stick` (animation/blend_system).
- v8.1.7: `bus_fade_volume_applied_exactly_once` (audio/tests); `always_succeed_passes_running_through` + 3 more (behavior.rs); `tilemap_atlas_out_of_range_tile_id_returns_full_uv`, `..._in_range_..._correct_uv` (tilemap.rs).
- v8.1.8: `overlapping_buttons_only_topmost_fires_on_click`, `scroll_view_zero_item_height_does_not_panic`, `slider_press_frame_emits_exactly_one_changed_event` (ui/system); `save_path_traversal_is_blocked`, `save_path_subdir_preserved` (save.rs); `looping_timeline_wraps_large_dt_with_modulo` (timeline.rs).
- v8.1.9: none (GPU surface acquisition is not headlessly testable — no fabricated GPU test).
- Quick run: `cargo test --lib` (481) + integration `cargo test --test editable_component_scene_replace` (seq-2, the game-data-editor flow).

### Docs
- `docs/CHANGELOG.md` (8.1.5–8.1.9), `CLAUDE.md` (version), `Cargo.toml`/`Cargo.lock`.

## User Feedback & Preferences (REQUIRED)

- `/loop` directive, verbatim (repeated each round): "**버그 모두 수정까지 진행. opus가 감독하고 sonnet 시켜서 실무 진행해. 3회이상 실패지점은 나에게 보고하고 정지. 이외의 판단은 opus한테 맡김.**"
- Re-issued the loop / said "계속 진행해" after each completion → wants maximal thoroughness ("모두"), drove the 2nd pass.
- Gates EVERY merge with explicit "#N 머지 확인" — merged #28–#32 one at a time, in order; never wants self-merge.
- Earlier in the session (seq 2) explicitly asked for the HTML test-checklist and the memory updates.
- Korean prose / English code+docs (standing). Values concise scannable status (tables).

## Where We're Going

1. **Engine-wide hunt is COMPLETE/CONVERGED.** main @ v8.1.9, clean, all merged. No more clusters to sweep.
2. **Optional — the 4 deferred items** (Rhai-ctx graceful, Rhai-scope rewind, atlas-hot-reload grid rebuild, failed-load watcher). All Low/design; only if the user wants them.
3. **Optional — a 3rd pass** (near-certain dry; not recommended — diminishing returns).
4. **rust-survivors v8 migration** — the game pins the engine by git rev; v8.x has breaking changes (since v8.0.0). Migration per CHANGELOG is the user's call; NOT done.
5. **New feature per VISION** — the natural next move; user picks genre/subsystem.
6. **Branch cleanup** — prune the ~5 stale pre-existing merged branches if desired (offered, not done).

## Risks & Blockers

- **cargo stale-artifact trap.** `target/debug/examples/<name>` (non-tracked) is NOT reliably rebuilt on engine-source change (`cargo build --example` may say "Finished" without relinking). ALWAYS `cargo clean -p skeleton-engine` before trusting a GUI re-playtest, and check `binary mtime > source mtime`. (Burned ~3 cycles in seq 2.)
- **Synthetic Ctrl+Z can't reach egui.** F2 (winit plain key) works via CGEvent; egui-internal modifier shortcuts (Ctrl+Z) do not (neither CGEvent flags nor `osascript keystroke … using control down`). Verify undo via unit tests, not live keyboard.
- **A public fn `&self`→`&mut self` is BREAKING** — not a patch. Keep patches non-breaking (the cast_ray rework).
- **`gh pr edit --base` does NOT re-trigger CI** (CI is `branches: [main]` only). A stacked PR (base = a feature branch) gets NO CI until its base merges and it retargets to main. Merge in order; or rely on local `verify.sh` (CI-equivalent except `cargo package --locked`, which is CI-only).
- **Stale rust-analyzer diagnostics** are routine mid-edit (phantom type/borrow errors). Trust `cargo check`, never the IDE snapshot.

## Open Questions

- Does the user want the 4 deferred items fixed, a 3rd pass, a new feature, or the rust-survivors migration? (Offered at close; awaiting direction.)
- crates.io publish — still aspirational? (carried from seq 2.)

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3        # 00de9e8 = #32 merge (v8.1.9); main clean
grep '^version' Cargo.toml   # 8.1.9

# Prior context (read in order)
#   plans/handoffs/HANDOFF_editor-gui-arc_engine-wide-bug-hunt_2026-06-15.md  (this — the hunt)
#   plans/handoffs/HANDOFF_editor-gui-arc_playtest-bugfix-sweep_2026-06-14.md  (seq 2 — editor fixes)
#   docs/CHANGELOG.md  — 8.1.2 … 8.1.9

# Verify current state
./scripts/verify.sh             # fmt, clippy -D, wasm build, test --all-targets (481 lib), doc
cargo package --locked          # CI-only gate (workspace/deps)

# Deferred items (if pursuing) — locations
#   src/scripting/context.rs  (Rhai with_ctx expect)         — documented contract, low
#   src/scripting/execution.rs (Rhai persistent Scope)       — design-debatable
#   src/asset/hot_reload.rs + image_loading.rs (atlas/watcher) — low feature gaps

# Next action
#   Engine hunt is DONE + merged. Ask the user: new feature (VISION) / 4 deferred items /
#   rust-survivors v8 migration / 3rd pass (not recommended). Default = new feature, user picks.
```
