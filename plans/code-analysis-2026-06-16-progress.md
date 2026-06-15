# Execution plan/progress — docs/CODE_ANALYSIS_2026-06-16.md (80 findings)

> Branch: `chore/engine-hardening-2026-06-16` | Loop: Opus plans/reviews/supervises, Sonnet implements.
> Rule: no intermediate reports; full code review + tests only at the end; final report when clean.
> Agents EDIT + write inline `#[cfg(test)]` tests, do NOT run cargo (avoid lock contention); Opus runs the central gate per iteration. Agents must NOT edit `src/lib.rs` (report needed re-exports → Opus adds centrally). Physics is native-only → cfg-gate any `crate::physics` ref. Prefer additive; breaking allowed (pre-1.0, rust-survivors dropped).

## Work units (status: TODO / DONE / REVIEW)

### Iteration 1 (parallel, disjoint dirs)
- [x] WU1 Animation (DONE) — `src/animation/{clip_set,player,state_machine,blend_tree}.rs`, `src/skeletal.rs`: columns=0 panic guard; OOB frame index validate; play(OOB) guard + is_finished()=false fallback; add_transition dead-edge warn; skeletal is_finished() started-guard; BlendTree1D sort entries in new()
- [x] WU2 Physics (DONE) — `src/physics/world/joints.rs`, `physics/system.rs`, `collision/grid.rs`, `physics/world/tile_collider.rs`: prismatic zero-axis guard; contact ordered_pair symmetry; promote 4 scratch Vecs to fields; rebuild no-collect + candidates scratch; SolidTiles::Only→HashSet (additive ctor); TilemapColliders despawn-leak helper (native-gated)
- [x] WU3 Audio (DONE) — `src/audio/{bus,positional,playback,ducking}.rs`: set_bus_volume/set_volume fade guard; update_position fade guard; file_cache clear API; scratch Vec reuse; AudioManager wasm-only rustdoc note
- [x] WU4 UI (DONE) — `src/ui/{panel,slider,localized}.rs`, `src/ui/system/text_input_pass.rs`: Panel::direction reflect (LayoutDir to_i32/from_i32); text_input focus z-order + visible guard; Slider set_field initial_value→value; LocalizationSystem TextInput.placeholder

### Iteration 2 (parallel, disjoint files)
- [ ] WU5 Renderer-core — `src/renderer/{sprite,text}.rs`, `renderer/shaders/post_process.wgsl`, `src/gpu_particle.rs`(or renderer/gpu_particle.rs), `src/atlas.rs`: atlas texture_path_arc + sprite scratch fields; glyphon prepare/render log + shaped-buffer cache; bloom texel_size uniform; gpu_particle ring-buffer base_slot partition
- [ ] WU6 Renderer-lighting/camera — `src/renderer/{lighting,render_target}.rs`, `src/camera.rs`, `src/app/render.rs`, `src/components.rs`: light cull viewport-center + frustum prefilter; camera shake in screen_to_world/world_to_screen + zoom/shake accessors + zoom_to guard; RenderTarget clear_color field + create_render_target + render.rs RT clear
- [ ] WU7 ECS/reflect/prefab — `src/ecs/{world,events}.rs`, `src/prefab.rs`, `src/app/editor/ui/mod.rs`: has_component<T>(); query_added/changed empty guard; events doc fix; serialize_entity ron-err log; spawn_entity_def missing-registry log; inspector write-back TypeId-keyed
- [ ] WU13 Input — `src/input/{map,gamepad,state,touch}.rs`: gamepad axis in just_pressed/just_released + axis_value(); #[non_exhaustive] on GamepadButton/GamepadAxis; release() guard + release_all(); touch coord docs + swipe_threshold field

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

## Notes / decisions log
- Iter1 (WU1-4): DONE. Opus fixes during review: (a) derived `#[derive(Debug, Clone)]` on `AnimationClipSet` (new tests panic-format the Result); (b) skeletal `is_finished()` dropped the over-aggressive `duration > 0.0` guard — a zero-duration non-looping clip is finished *after* its first tick, `started` flag handles construction-time; (c) physics `scratch_vecs_cleared_between_frames` test corrected (frame-2 Stopped is correct; assert frame-3 clean). Gate: `cargo test --lib` 638 pass / 0 fail (+35); `cargo clippy --lib -D warnings` clean. Audio fix #4 used a `[Option<String>;8]` stack buffer + overflow Vec; ducking.rs left as-is (noted). Deferred to final clippy --all-targets: inline-test lint check.
