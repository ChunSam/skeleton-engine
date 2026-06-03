# skeleton-engine v2.0 cleanup follow-up fixes handoff

**Date:** 2026-06-03
**Status:** COMPLETED
**Bead(s):** none
**Epic:** v2 cleanup
**Chain:** `v2-cleanup`
**Sequence:** `2`
**Parent:** `plans/handoffs/HANDOFF_v2-cleanup_2026-06-03.md`
**Current branch:** `main`

## Related Handoffs

- Parent handoff: `plans/handoffs/HANDOFF_v2-cleanup_2026-06-03.md`.
- Parent status was `IN PROGRESS`.
- Parent line count was 514 lines.
- This file is the second handoff in the `v2-cleanup` chain.
- This file records the post-parent review and follow-up implementation.
- The first handoff covered the initial v2 cleanup implementation.
- This handoff covers the second pass that fixed review findings before commit.
- No separate plan file was created for this follow-up pass.
- The user asked for a handoff and commit after implementation.

## Since Last Handoff

- A full code review pass was run after the parent handoff.
- The review focused on structural correctness, feature separation, dependency shape, and implementation mismatches.
- The highest-risk issue found was point-light coordinate conversion.
- The second high-risk issue found was render-target lifecycle correctness.
- The third issue found was post plus lighting routing.
- The scripting API was also missing a stable way for a script to despawn its own current entity.
- Documentation still contained stale `SCENE_DEF_VERSION = 1` references.
- `World` rustdoc still described `Entity` as a simple integer-like id.
- The user asked for a priority-ordered fix plan.
- The plan prioritized runtime rendering correctness first.
- The plan then covered scripting handle usability.
- The plan then covered documentation and stale-reference cleanup.
- The user then asked to implement the proposed plan.
- The follow-up implementation is complete.
- The follow-up implementation passed all required verification commands.
- The current user request is to run the handoff skill and commit.
- No push was requested.
- No pull request was requested.
- No external project validation was requested.
- `rust-survivors` remains intentionally out of scope.

## Reference Documents

- `AGENTS.md` is the local agent reference.
- `docs/AGENT_WORKFLOW.md` contains the detailed agent workflow.
- `docs/PATTERNS.md` contains engine architecture patterns.
- `docs/VISION.md` describes the skeleton-engine direction.
- `docs/CHANGELOG.md` records the v2.0.0 breaking changes.
- `README.md` contains the main public usage overview.
- `REFERENCE.html` contains the public API reference.
- `docs/ENTITY_GENERATION_V2_PLAN.md` documents the entity generation migration.
- `docs/HANDOFF.md` contains historical project handoff notes.
- `plans/handoffs/HANDOFF_v2-cleanup_2026-06-03.md` is the parent handoff.

## The Goal

- Complete the skeleton-engine v2.0 cleanup.
- Preserve the user-visible editor and debug behavior.
- Remove misleading normal-map public API.
- Move entity handles to index plus generation semantics.
- Keep old `SceneDef` files loading where practical.
- Fix runtime render routing so enabled passes compose correctly.
- Fix scene reload so core resources are restored consistently.
- Ensure `AssetServer::load_image` can reach GPU upload.
- Make scripting despawn require index plus generation.
- Give scripts a way to discover the current entity index and generation.
- Update public documentation to match the breaking API.
- Remove `rust-survivors` guidance from official agent instructions.
- Verify native and wasm library builds.
- Verify tests, examples, clippy, formatting, and stale-reference scans.
- Commit the complete working tree when the handoff is written.

## Where We Are

- The implementation is currently done and verified.
- The work tree is dirty until this handoff is added and committed.
- The branch is `main`.
- `Cargo.toml` now reports version `2.0.0`.
- `Cargo.lock` matches the package version bump.
- `Entity` is no longer a tuple struct.
- `Entity` now stores `index` and `generation`.
- `Entity::index()` exposes the index.
- `Entity::generation()` exposes the generation.
- `Entity::from_raw_parts(index, generation)` builds a handle.
- `World` tracks generations for entity reuse.
- `World` rejects stale handles across component APIs.
- `World::clone_entity` now returns `Option<Entity>`.
- Commands using stale handles no-op instead of mutating reused entities.
- Hierarchy APIs were updated to avoid stale parent and child attachment.
- `Sprite.normal_texture` was removed.
- `Sprite.normal_handle` was removed.
- Scene version moved to `SCENE_DEF_VERSION = 2`.
- Legacy v1 normal-map fields are ignored during scene loading.
- `SpriteRenderer` material parameters now key by `Entity`.
- `AssetServer` exposes image assets for lazy GPU upload.
- `App::load_image` still keeps the pending upload queue.
- `App` now also lazily uploads images loaded through `AssetServer::load_image`.
- Core resource insertion is centralized in `src/app/core_resources.rs`.
- `App::new` uses the core resource helper.
- `reload_scene` uses the core resource helper.
- Scene replacement preserves required debug and runtime resources.
- `PostProcessRenderer` can render from arbitrary input to arbitrary output.
- `PostProcessRenderer` now exposes `format()`.
- `PostProcessRenderer` now has `reconfigure(...)`.
- `LightingRenderer` no longer owns a fixed output texture.
- `LightingRenderer` renders to an output view supplied by the caller.
- `LightingRenderer` now exposes `format()`.
- `LightingRenderer` now has `reconfigure(...)`.
- App render orchestration now uses a dedicated scene texture for lighting-only flow.
- App render orchestration now uses a dedicated post texture for post-plus-lighting flow.
- Intermediate render textures are recreated on viewport width changes.
- Intermediate render textures are recreated on viewport height changes.
- Intermediate render textures are recreated on format changes.
- Render flow is now `scene -> post(optional) -> lighting(optional) -> final`.
- Post-only flow renders scene into the post-process input and post-process output to the frame.
- Lighting-only flow renders scene into a scene intermediate and lighting output to the frame.
- Post-plus-lighting flow renders scene into the post input, post into a post intermediate, then lighting to the frame.
- Point-light uniforms use world-space light semantics.
- Point-light uniforms account for camera position.
- Point-light uniforms account for camera zoom.
- Point-light y conversion now matches the renderer coordinate convention.
- Point-light radius conversion now uses width-based NDC scaling.
- `ScriptCtx` now carries the current `Entity`.
- Rhai scripts can call `entity_index()`.
- Rhai scripts can call `entity_generation()`.
- Rhai scripts can call `despawn_entity(index, generation)`.
- The old script `despawn_entity(id)` form is removed.
- Documentation explains the new script identity helpers.
- `REFERENCE.html` no longer documents `SCENE_DEF_VERSION = 1`.
- `README.md` no longer advertises removed normal-map fields.
- `AGENTS.md` no longer contains `rust-survivors` related-project guidance.
- `docs/AGENT_WORKFLOW.md` no longer requires external breaking-change checks for this repo by default.

## What We Tried (Chronological)

- The user first requested a full code scan for structural problems.
- The scan focused on source structure, functional separation, dependencies, and intent mismatches.
- The first report identified runtime rendering, ECS handle, normal-map API, reload resources, asset upload, and docs issues.
- The user asked for a risk-ordered correction plan.
- The plan ordered work as runtime correctness, breaking ECS cleanup, public API and docs, then internal separation.
- The user asked to validate the plan with the `grill-me` skill.
- The validation challenged ambiguous public API compatibility and external project checks.
- The user accepted a v2.0 breaking-change cleanup direction.
- The user explicitly excluded `rust-survivors` from this cleanup.
- The user asked to remove `rust-survivors` guidance from `AGENTS.md`.
- The user pasted the skeleton-engine v2.0 cleanup plan.
- The initial v2 cleanup implementation changed ECS, rendering, assets, scripting, docs, and agent instructions.
- A parent handoff was written after that initial implementation.
- The user then asked to inspect the entire codebase again.
- A code-review stance was used for the second inspection.
- The review identified point-light y and radius conversion as P1.
- The review identified format-insensitive render-target reuse as P1.
- The review identified `LightingRenderer.output_view` reuse as a post intermediate as P2.
- The review identified scripting self-despawn usability as P2.
- The review identified stale `SCENE_DEF_VERSION` and stale `World` docs as P3.
- The user asked for a priority-ordered plan and fixes.
- A plan was produced in priority order.
- The user then asked to implement that plan.
- `src/renderer/lighting.rs` was refactored first.
- `LightingRenderer` internal output ownership was removed.
- `light_position_ndc` was corrected.
- Lighting tests were updated and expanded.
- `src/renderer/post_process.rs` gained format inspection and reconfiguration.
- `src/app.rs` render intermediates were updated to carry texture format.
- `src/app.rs` gained a second post-to-lighting intermediate texture.
- `ensure_intermediate_texture` was introduced for width, height, and format checks.
- Render flow was corrected for post-only, lighting-only, and post-plus-lighting.
- `src/scripting.rs` was updated so `ScriptCtx` carries the current entity.
- `entity_index()` and `entity_generation()` were registered for Rhai.
- Scripting tests were added for identity helper behavior.
- Scripting tests were added for self-despawn using index plus generation.
- Stale docs were updated.
- Formatting was run.
- Native checks were run.
- Native tests were run.
- Clippy was run with warnings as errors.
- Wasm library check was run.
- Example checks were run.
- Stale-reference scans were run.
- The user then asked to create a handoff and commit.

## Key Decisions

- Public API compatibility is not preserved for this v2.0 cleanup.
- Existing scene files should still load where practical.
- Removed normal-map fields are ignored when reading old scene definitions.
- Normal-map rendering itself is not implemented in this cleanup.
- `Entity` remains `Copy`, `Clone`, `Debug`, `Eq`, `Hash`, and serializable.
- Entity raw access is intentionally limited to `index`, `generation`, and `from_raw_parts`.
- Scripting uses explicit `index` and `generation` instead of reintroducing an opaque script handle type.
- The script identity helpers are simple functions on the script scope.
- `despawn_entity(index, generation)` is the only script despawn entrypoint.
- Stale entity commands must not mutate reused indices.
- Rendering pass composition is managed by `App`, not by hidden fixed renderer outputs.
- `LightingRenderer` is now a render pass over caller-provided views.
- `PostProcessRenderer` remains configurable but does not own final routing decisions.
- Intermediate texture recreation considers format changes.
- No new third-party dependencies were added.
- Internal separation was kept focused and did not add public renderer extension APIs.
- The deeper `SpriteRenderer` and `UiSystem` split remains future structural work.
- The repository documentation is in English except files explicitly allowed by repo rules.
- `rust-survivors` impact remains intentionally unchecked.
- The final commit will be a single commit because the user asked to commit the current implementation now.

## Evidence & Data

- Current branch before commit: `main`.
- Parent handoff file: `plans/handoffs/HANDOFF_v2-cleanup_2026-06-03.md`.
- Parent handoff line count: `514`.
- Existing handoff directory: `plans/handoffs/`.
- New handoff path: `plans/handoffs/HANDOFF_v2-cleanup_follow-up-fixes_2026-06-03.md`.
- New source module path: `src/app/core_resources.rs`.
- Dirty tracked files before this handoff: 23.
- Untracked files before this handoff: parent handoff file and `src/app/`.
- `git diff --stat` before this handoff reported 651 insertions and 318 deletions.
- The pre-handoff stat excluded untracked file contents.
- `cargo fmt` was run and completed.
- `cargo fmt --check` was run and passed.
- `cargo check --all-targets` was run and passed.
- `cargo test --all-targets` was run and passed.
- The main library test run included 269 tests.
- The `mp_server` example target included 3 tests.
- `cargo clippy --all-targets -- -D warnings` was run and passed.
- `cargo check --target wasm32-unknown-unknown --lib` was run and passed.
- `cargo check --examples` was run and passed.
- `git diff --check` was run and passed.
- Stale-reference scan found only allowed changelog mentions.
- Allowed stale scan result: `docs/CHANGELOG.md` mentions removed direct `entity.0` access.
- Allowed stale scan result: `docs/CHANGELOG.md` mentions removed `despawn_entity(id)`.
- No failing verification command remained at handoff time.

## Verification Commands

```text
cargo fmt
```

Result: completed successfully.

```text
cargo fmt --check
```

Result: passed.

```text
cargo check --all-targets
```

Result: passed.

```text
cargo test --all-targets
```

Result: passed, including 269 library tests and 3 `mp_server` tests.

```text
cargo clippy --all-targets -- -D warnings
```

Result: passed.

```text
cargo check --target wasm32-unknown-unknown --lib
```

Result: passed.

```text
cargo check --examples
```

Result: passed.

```text
git diff --check
```

Result: passed.

```text
rg -n "SCENE_DEF_VERSION = 1|현재 = 1|Entity\(pub u32\)|단순한 u32 ID|despawn_entity\(id|World::clone_entity\(src\) -> Entity|SceneChange\.0|entity\.0" README.md REFERENCE.html docs/CHANGELOG.md docs/PATTERNS.md src examples AGENTS.md
```

Result: only allowed changelog mentions remained.

## Code Analysis

- `src/ecs/world.rs` is the center of the entity generation migration.
- `Entity` stores `index` and `generation`.
- `World::spawn` now returns a live handle for the current generation.
- `World::despawn` increments generation before reuse.
- `World::is_alive` rejects stale generation mismatches.
- `World::get` rejects stale handles.
- `World::get_mut` rejects stale handles.
- `World::add_component` rejects stale handles.
- `World::remove_component` rejects stale handles.
- `World::take_component` rejects stale handles.
- `World::clone_entity` returns `None` for stale sources.
- Command application goes through world APIs and inherits stale-handle protection.
- Hierarchy cleanup avoids stale parent and child cross-attachment.
- `src/scripting.rs` now passes the current entity into each script context.
- `ScriptCtx` became the source for script identity helper functions.
- `entity_index` returns the active entity index as `i64`.
- `entity_generation` returns the active entity generation as `i64`.
- `despawn_entity` validates both index and generation.
- The script self-despawn test proves the identity helpers compose with despawn.
- `src/renderer/lighting.rs` now treats point lights as world-space values.
- `light_position_ndc` applies camera offset and zoom.
- `light_position_ndc` maps positive world y consistently with the renderer.
- `light_position_ndc` scales radius by viewport width.
- `LightingRenderer::render` takes both input and output views.
- `LightingRenderer::reconfigure` updates format-sensitive GPU state.
- `src/renderer/post_process.rs` supports reconfiguration for format changes.
- `PostProcessRenderer::render_to_view` keeps the arbitrary input/output path.
- `src/app.rs` owns render-pass routing.
- `App::ensure_intermediate_texture` handles width, height, and format mismatch.
- `scene_texture_for_lighting` is used for lighting-only.
- `post_texture_for_lighting` is used for post-plus-lighting.
- `post_process_renderer.reconfigure` runs when the target format changes.
- `lighting_renderer.reconfigure` runs when the target format changes.
- `src/app/core_resources.rs` centralizes core resource insertion.
- `insert_core_resources` prevents scene reload from dropping required runtime resources.
- `register_core_component_metadata` keeps component metadata setup in one place.
- `src/asset.rs` exposes image assets for GPU lazy upload.
- `src/app.rs` lazy-upload path ensures `AssetServer::load_image` images can reach texture cache.
- `src/components.rs` no longer exposes normal-map sprite fields.
- `src/prefab.rs` preserves legacy normal fields as ignored serde inputs.
- `src/renderer/sprite.rs` was adjusted for opaque entity handles.
- `src/particle.rs` no longer assigns removed normal fields.
- `src/steering.rs` was adjusted for opaque entity handle access.

## Files Changed

- `Cargo.toml`: version bumped to `2.0.0`.
- `Cargo.lock`: version metadata updated.
- `AGENTS.md`: removed `rust-survivors` instructions and kept agent guidance concise.
- `README.md`: updated v2 breaking changes and removed normal-map public API references.
- `REFERENCE.html`: updated public API reference for v2.
- `docs/AGENT_WORKFLOW.md`: removed broad external-project breaking-change requirement.
- `docs/CHANGELOG.md`: added v2.0.0 breaking changes and follow-up scripting identity notes.
- `docs/ENGINE_TERMS_FOR_BEGINNERS.md`: removed or corrected misleading normal-map wording.
- `docs/ENTITY_GENERATION_V2_PLAN.md`: aligned entity migration details with the implementation.
- `docs/HANDOFF.md`: updated historical handoff notes for v2 cleanup.
- `docs/PATTERNS.md`: corrected direct `SceneChange.0` style references.
- `docs/VISION.md`: aligned playable-example status with the current repo.
- `plans/handoffs/HANDOFF_v2-cleanup_2026-06-03.md`: parent v2 cleanup handoff.
- `plans/handoffs/HANDOFF_v2-cleanup_follow-up-fixes_2026-06-03.md`: this handoff.
- `src/app.rs`: render orchestration, lazy image upload, scene reload, editor labels, and entity handle updates.
- `src/app/core_resources.rs`: new centralized core resource helper.
- `src/asset.rs`: image asset enumeration for lazy GPU upload.
- `src/components.rs`: removed normal-map public fields.
- `src/ecs/world.rs`: entity generation v2 implementation and tests.
- `src/particle.rs`: removed removed normal-field initialization.
- `src/prefab.rs`: legacy scene compatibility for removed normal fields and scene version 2.
- `src/renderer/lighting.rs`: render routing API, format reconfigure, and point-light uniform fixes.
- `src/renderer/post_process.rs`: reconfigure and format support.
- `src/renderer/sprite.rs`: opaque entity material key handling.
- `src/scripting.rs`: generation-aware script despawn and current-entity identity helpers.
- `src/steering.rs`: opaque entity accessor update.

## User Feedback & Preferences

- The user asked for Korean discussion, but repository documentation prose should remain English.
- The user wanted risk-ordered findings.
- The user wanted a concrete correction plan.
- The user explicitly requested `grill-me` validation for the plan.
- The user accepted breaking public API changes for v2.0.
- The user wanted `rust-survivors` excluded.
- The user wanted `AGENTS.md` cleaned of `rust-survivors` impact-check instructions.
- The user wanted implementation, not only a proposal.
- The user then asked for another whole-code review.
- The user then asked for priority-ordered fixes.
- The user asked to implement the proposed plan.
- The current user asked for handoff and commit.
- The user did not ask to push.
- The user did not ask to create a branch.
- The user did not ask to open a pull request.

## Where We're Going

- The immediate next action is to commit the completed v2 cleanup.
- After the commit, the working tree should be clean.
- Future work can optionally run visual smoke tests for renderer paths.
- Future work can optionally add a small render-routing helper unit if a clean test seam appears.
- Future work can split `SpriteRenderer` more deeply.
- Future work can split `UiSystem` widget update and render-queue logic more deeply.
- Future work can update `CLAUDE.md` if that quick reference still has stale agent guidance.
- Future work can add a wasm-compatible example matrix.
- Future work can add a playable example for any newly introduced feature work.
- Future work can audit historical docs for intentionally stale history versus current guidance.
- Future work can create a dedicated release note or migration page for v2.0.
- Future work can run runtime smoke examples manually if a GUI session is desired.

## Risks & Blockers

- No blocker remains for committing.
- The follow-up pass used code-level and test-level verification, not a visual GPU screenshot.
- Native examples compile, but interactive examples were not manually played in this pass.
- Wasm was checked as library-only.
- `rust-survivors` was not checked by design.
- The commit will be large because the user requested committing the complete current implementation.
- The parent plan preferred subsystem commits, but that was not practical after the combined implementation had already been made.
- Some historical documents may still mention past behavior as history rather than current API.
- Deep renderer and UI internal decomposition was intentionally limited.
- No new public renderer extension API was added.
- No new public widget extension API was added.
- If downstream code uses direct `entity.0`, it will break.
- If downstream scripts use `despawn_entity(id)`, they will break.
- If downstream code used `Sprite.normal_texture` or `Sprite.normal_handle`, it will break.
- Existing old `SceneDef` normal fields should deserialize and be ignored.
- The code has not been validated against external repositories.

## Open Questions

- Should a future session run manual visual smoke tests for post-only, lighting-only, and post-plus-lighting examples?
- Should the large v2 commit later be split by subsystem for review readability?
- Should `CLAUDE.md` be reviewed for the same `rust-survivors` guidance cleanup?
- Should `REFERENCE.html` get a dedicated migration section instead of only v2 notes?
- Should script identity helpers return unsigned values through a wrapper instead of Rhai `i64`?
- Should render pass routing be factored into a smaller pure helper for easier unit tests?
- Should the asset lazy-upload path get an integration-style test with a mock renderer cache?
- None of these questions block the current commit.

## Quick Start for Next Session

1. Confirm the latest commit with `git log -1 --oneline`.
2. Confirm cleanliness with `git status -s`.
3. Inspect the commit summary with `git show --stat --oneline --decorate -1`.
4. If reviewing ECS, start at `src/ecs/world.rs`.
5. If reviewing rendering, start at `src/app.rs`.
6. If reviewing point lights, inspect `src/renderer/lighting.rs`.
7. If reviewing post-process routing, inspect `src/renderer/post_process.rs` and `src/app.rs`.
8. If reviewing scripting, inspect `src/scripting.rs`.
9. If reviewing scene compatibility, inspect `src/prefab.rs`.
10. If reviewing docs, inspect `README.md`, `REFERENCE.html`, and `docs/CHANGELOG.md`.
11. Re-run `cargo check --all-targets` if source changed.
12. Re-run `cargo test --all-targets` if behavior changed.
13. Re-run `cargo clippy --all-targets -- -D warnings` before release.
14. Re-run `cargo check --target wasm32-unknown-unknown --lib` before release.
15. Re-run `cargo check --examples` before release.
16. Re-run the stale-reference scan if public API docs changed.
17. Do not check `rust-survivors` unless the user explicitly brings it back into scope.
18. Do not reintroduce direct `entity.0` access.
19. Do not reintroduce `Sprite.normal_texture` or `Sprite.normal_handle`.
20. Keep new docs in English unless editing the allowed Korean beginner glossary.

