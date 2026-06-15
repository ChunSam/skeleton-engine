# Execution plan/progress — docs/CODE_ANALYSIS_2026-06-16.md (80 findings)

> Branch: `chore/engine-hardening-2026-06-16` | Loop: Opus plans/reviews/supervises, Sonnet implements.
> Rule: no intermediate reports; full code review + tests only at the end; final report when clean.
> Agents EDIT + write inline `#[cfg(test)]` tests, do NOT run cargo (avoid lock contention); Opus runs the central gate per iteration. Agents must NOT edit `src/lib.rs` (report needed re-exports → Opus adds centrally). Physics is native-only → cfg-gate any `crate::physics` ref. Prefer additive; breaking allowed (pre-1.0, rust-survivors dropped).

## ⚠ RESUME STATE (2026-06-16 — session usage limit hit, resets ~03:50 Asia/Seoul)

Iterations 1 (`0d6da75`) + 2 (`f5260ef`) are COMMITTED & green. **Iteration 3 (WU8-11) is UNCOMMITTED and NON-COMPILING** — the 4 Sonnet agents edited their files but the session limit cut them off before summaries; tree must be repaired before commit. Last good commit: `f5260ef`.

**Known integration errors to fix on resume (from diagnostics — verify with `cargo check --all-targets`):**
1. **NetworkSystem broke 5 examples** (`mp_client.rs:54`, `salvage_run.rs:114`, `coin_race.rs:110`, `predict_shooter.rs:65`, `orbital_dodger.rs:88` — `expected value, found struct NetworkSystem` E0423). WU8 added a field to `NetworkSystem` for warn-once, breaking unit-struct usage `add_system(NetworkSystem)`. FIX: keep `NetworkSystem` a UNIT struct; move the warn-once flag to a `static WARNED: AtomicBool` inside `poll()` (not a struct field). Also `network.rs:1341,1346` access private `close_requested` (test) — keep test in same module or add accessor.
2. **ParticleEmitter `z` field** (WU9): `src/particle/config_set.rs:197` constructs `ParticleEmitter` without `z` (E0063) → add `z: 0.0` (or derive Default + `..Default::default()`). Also `src/particle/mod.rs` EmitterSnapshot collect has `z` inserted in the WRONG tuple position (FromIterator arity/order mismatch) — align the snapshot tuple construction with its consumer (z should be ordered consistently).
3. Re-run gate; verify WU9 tilemap (blob_47 47-mask fix + `generation` dirty-guard + HashSet), WU10 (save with_key, behavior child.reset, steering, pathfinding blocked-start==goal, Track set_value/set_easing), WU11 (editor widget registration, pathfinding-overlay clone, name-only serialize, add_component_selected, timeline keyframe wiring) all compile + pass.

**ACTUAL PARTIAL STATE (from `git status` — agents cut off mid-work):**
- PRESENT in working tree: `src/network.rs` (WU8), `src/tilemap.rs` + `src/data_table.rs` + `src/particle/mod.rs` (WU9 partial), `src/save.rs` + `src/behavior.rs` + `src/steering.rs` (WU10 partial). Review these via `git diff` (no agent summaries — diffs are the source of truth).
- MISSING / NOT DONE: WU9 scripting dead-field removal (`src/scripting/context.rs`/`execution.rs`/`api.rs`); WU10 `src/pathfinding.rs` (blocked start==goal) + `src/timeline.rs` (Track set_value/set_easing); WU11 ENTIRELY (`src/app/editor.rs`, `src/app/editor/ui/docked.rs`, `src/prefab.rs`). These must be COMPLETED on resume (re-run those agents or do directly).
- Note: since WU10's timeline.rs + WU11's docked.rs keyframe wiring are both undone, no cross-dependency issue remains; do timeline.rs (Track API) before docked.rs wiring.

**Resume procedure:** (a) review present diffs + fix compile errors 1+2; (a2) COMPLETE the missing WU9-scripting/WU10-pathfinding+timeline/WU11 work; (b) `cargo test --lib` green; (c) `cargo check --all-targets` green (examples); (d) commit iteration 3; (e) mark WU8-11 done below; (f) Iteration 4 = WU12 App-loop (window.rs Focused→release_all + double-step guard; schedule.rs catch_unwind frame-abort + HotReloadable trait; app.rs raw-ptr→owned views + FadeTransition wasm warn) + WU14 Build/CI (serde_json→dev-deps, MSRV, wasm clippy, integration tests); (g) Iteration 5 final gate (fmt --check, clippy --all-targets -D warnings, wasm lib+bins build, test --all-targets, doc -D warnings) + code review + version bump + docs/CHANGELOG.md + REFERENCE.html + final report. Remember WU12 must wire `InputState::release_all()` (currently dead_code).

## Work units (status: TODO / DONE / REVIEW)

### Iteration 1 (parallel, disjoint dirs)
- [x] WU1 Animation (DONE) — `src/animation/{clip_set,player,state_machine,blend_tree}.rs`, `src/skeletal.rs`: columns=0 panic guard; OOB frame index validate; play(OOB) guard + is_finished()=false fallback; add_transition dead-edge warn; skeletal is_finished() started-guard; BlendTree1D sort entries in new()
- [x] WU2 Physics (DONE) — `src/physics/world/joints.rs`, `physics/system.rs`, `collision/grid.rs`, `physics/world/tile_collider.rs`: prismatic zero-axis guard; contact ordered_pair symmetry; promote 4 scratch Vecs to fields; rebuild no-collect + candidates scratch; SolidTiles::Only→HashSet (additive ctor); TilemapColliders despawn-leak helper (native-gated)
- [x] WU3 Audio (DONE) — `src/audio/{bus,positional,playback,ducking}.rs`: set_bus_volume/set_volume fade guard; update_position fade guard; file_cache clear API; scratch Vec reuse; AudioManager wasm-only rustdoc note
- [x] WU4 UI (DONE) — `src/ui/{panel,slider,localized}.rs`, `src/ui/system/text_input_pass.rs`: Panel::direction reflect (LayoutDir to_i32/from_i32); text_input focus z-order + visible guard; Slider set_field initial_value→value; LocalizationSystem TextInput.placeholder

### Iteration 2 (parallel, disjoint files)
- [x] WU5 Renderer-core (DONE) — `src/renderer/{sprite,text}.rs`, `renderer/shaders/post_process.wgsl`, `src/gpu_particle.rs`(or renderer/gpu_particle.rs), `src/atlas.rs`: atlas texture_path_arc + sprite scratch fields; glyphon prepare/render log + shaped-buffer cache; bloom texel_size uniform; gpu_particle ring-buffer base_slot partition
- [x] WU6 Renderer-lighting/camera (DONE) — `src/renderer/{lighting,render_target}.rs`, `src/camera.rs`, `src/app/render.rs`, `src/components.rs`: light cull viewport-center + frustum prefilter; camera shake in screen_to_world/world_to_screen + zoom/shake accessors + zoom_to guard; RenderTarget clear_color field + create_render_target + render.rs RT clear
- [x] WU7 ECS/reflect/prefab (DONE) — `src/ecs/{world,events}.rs`, `src/prefab.rs`, `src/app/editor/ui/mod.rs`: has_component<T>(); query_added/changed empty guard; events doc fix; serialize_entity ron-err log; spawn_entity_def missing-registry log; inspector write-back TypeId-keyed
- [x] WU13 Input (DONE) — `src/input/{map,gamepad,state,touch}.rs`: gamepad axis in just_pressed/just_released + axis_value(); #[non_exhaustive] on GamepadButton/GamepadAxis; release() guard + release_all(); touch coord docs + swipe_threshold field

### Iteration 3 (parallel, disjoint files)
- [ ] WU8 Network — `src/network.rs`: disconnect close side-channel/log; on_error detail; events-drop log/auto-register; RemoteEntities is_alive check; WASM buffered cap; reconnect Drop impl
- [ ] WU9 Assets/tilemap — `src/tilemap.rs`, `src/data_table.rs`, `src/particle/mod.rs`, `src/scripting/context.rs`: blob_47 VALID_MASKS fix; Tilemap generation dirty-guard; removed-entity HashSet; uv_refresh dedup; data_table extra-columns warn; ParticleEmitter z + with_z; remove dead spawned_ids
- [ ] WU10 Save/behavior/path/timeline — `src/save.rs`, `behavior.rs`, `steering.rs`, `pathfinding.rs`, `timeline.rs`: save_versioned_with_key/load_migrated_with_key; Sequence/Selector child.reset on completion; steering scratch reuse + eval-order doc + Wander::with_initial_dir; pathfinding blocked start==goal (both fns); Track::set_value/set_easing
- [ ] WU11 Editor — `src/app/editor.rs`, `src/app/editor/ui/docked.rs`: register UI widgets in factory/remover; pathfinding overlay clone reduce; docked serialize_entity per-frame→name-only; add_component_selected stale validation; timeline keyframe set_value/set_easing wiring (uses WU10 Track API)

### Iteration 4 (sequential — shared files / cross-cutting)
- [ ] WU12 App-loop — `src/app/window.rs`, `src/app/schedule.rs`, `src/app.rs`: Focused(false)→release_all + double step_frame guard + touch coord; catch_unwind frame-abort + HotReloadable trait + register_hot_reloadable; app.rs raw-pointer→owned views + FadeTransition wasm warn
- [ ] WU14 Build/CI — `Cargo.toml`, `.github/workflows/ci.yml`, `tests/`: serde_json→dev-deps; MSRV align (1.92 job or bump); wasm clippy step; add integration tests per subsystem

### Iteration 5 (final, Opus)
- [ ] Full CI-equivalent gate: fmt --check, clippy --all-targets -D warnings, wasm lib+bins build, test --all-targets, doc -D warnings
- [ ] Code review pass (diff review + /code-review or review subagent)
- [ ] Version bump + docs/CHANGELOG.md + REFERENCE.html note for new public APIs
- [ ] Final report to user (+ PushNotification), stop loop

## Reassignments
- gpu_particle ring-buffer fix moved WU5→WU6 (touches app/render.rs which WU6 owns). post_process.rs kept in WU5. render.rs owned solely by WU6.
- WU13 `InputState::release_all()` is `pub(crate)`, intentionally UNUSED until WU12 wires it into the window.rs `Focused(false)` handler (expected dead_code warning meanwhile).

## Notes / decisions log
- Iter2 (WU5,6,7,13): DONE. WU5 submitted a NON-compiling tree (forgot to update `let buffers:` type annotation to the 4-tuple, and the `assign_instance_offsets` test still passed 1 arg) — Opus fixed both. Text shaping cache = plain-text-only `PlainTextCacheKey` (f32→bits, Option for no-bounds) + generation eviction + logic tests; rich text left on the per-frame path (judged correct, kept). Added `pub(crate) type InspectorCompFields` alias (mod.rs) used in mod.rs + docked.rs to silence clippy::type_complexity from WU7's TypeId tuple. Camera shake added to screen/world transforms (round-trip test); RT clear_color (Option, with_clear_color); gpu_particle shared frame_cursor (native-only). Gate: `cargo test --lib` 677 pass/0 fail (+39); `cargo clippy --lib` clean except expected `release_all` dead_code (WU12 wires it). clippy --all-targets deferred to final (after WU12).
- Iter1 (WU1-4): DONE. Opus fixes during review: (a) derived `#[derive(Debug, Clone)]` on `AnimationClipSet` (new tests panic-format the Result); (b) skeletal `is_finished()` dropped the over-aggressive `duration > 0.0` guard — a zero-duration non-looping clip is finished *after* its first tick, `started` flag handles construction-time; (c) physics `scratch_vecs_cleared_between_frames` test corrected (frame-2 Stopped is correct; assert frame-3 clean). Gate: `cargo test --lib` 638 pass / 0 fail (+35); `cargo clippy --lib -D warnings` clean. Audio fix #4 used a `[Option<String>;8]` stack buffer + overflow Vec; ducking.rs left as-is (noted). Deferred to final clippy --all-targets: inline-test lint check.
