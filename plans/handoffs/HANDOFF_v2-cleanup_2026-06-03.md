# skeleton-engine v2.0 cleanup implementation handoff

**Date:** 2026-06-03
**Status:** IN PROGRESS
**Bead(s):** none
**Epic:** v2 cleanup
**Chain:** `v2-cleanup` seq `1`
**Parent:** none - first handoff in this chain
**Prior chain:** none - first handoff in this chain
**Repo:** `/Users/jkl/Projects/skeleton-engine`
**Branch at handoff time:** `main`
**Working tree:** dirty, all v2 cleanup changes are unstaged
**Last verification:** all required Rust checks passed before this handoff file was created

## Reference Documents

- `AGENTS.md` - current quick reference, now updated for v2.0 and engine-only scope.
- `CLAUDE.md` - companion quick reference if another agent relies on it. It was not edited in this pass.
- `README.md` - public install and v2 breaking-change summary.
- `REFERENCE.html` - public API reference, partially updated for v2.0.
- `docs/CHANGELOG.md` - v2.0.0 release notes added.
- `docs/VISION.md` - project direction and playable-example status.
- `docs/PATTERNS.md` - scene-change recipe updated away from tuple-field access.
- `docs/ENTITY_GENERATION_V2_PLAN.md` - now reflects the implemented generation-handle design.
- `docs/AGENT_WORKFLOW.md` - default external-project checking guidance changed.
- `docs/HANDOFF.md` - long historical development handoff, with a current v2 status note added.

## Goal

Implement the user-approved `skeleton-engine v2.0 Cleanup Plan`.

The plan priority order was:

1. Runtime/rendering correctness.
2. ECS/API breaking cleanup.
3. Public API and documentation consistency.
4. Internal structure separation.

The user explicitly allowed breaking public API changes and explicitly excluded downstream
`rust-survivors` validation from this engine cleanup. The user also asked to delete
`rust-survivors`-related guidance from `AGENTS.md`.

## User Feedback And Decisions

The session started with the user asking, in Korean, to scan the full codebase for structural,
functional separation, dependency, and intent-mismatch problems.

The next user request was to rank problems by risk and create a fix plan.

The user then asked to use `/grill-me` to validate the plan. The `grill-me` skill was used
before implementation. The pressure-test decisions were:

- Scope includes structural refactoring, not only bug fixes.
- Public API compatibility does not need to be preserved.
- Normal-map mismatch should be resolved by API cleanup/removal.
- Changes should be grouped by subsystem for rollback.
- Downstream `rust-survivors` validation is excluded from this scope.
- `AGENTS.md` should remove `rust-survivors` guidance.
- Entity cleanup should implement generation-checked handles.
- Entity public shape should be an opaque struct, not `Entity(pub u32)`.
- Scripting should use `index + generation`, not index-only entity IDs.
- Release posture is a v2.0 transition.
- Work should be ordered by risk.
- Old `SceneDef` loading should remain practical, with version bump and ignored removed fields.
- App refactor should be internal module movement only.
- Normal-map cleanup should remove misleading public API, not implement real normal maps.
- Verification bar should be extended beyond `cargo check`.
- Renderer/UI refactors should not add new public extension APIs.
- Public docs should be updated.
- Rollback model is subsystem commit revert, not runtime feature flags.

The final user command before implementation was `PLEASE IMPLEMENT THIS PLAN:` followed by
the full v2 cleanup plan.

The latest user command was `/handoff`, so no further implementation should be started from
this handoff request unless the user asks to continue.

## Current State Snapshot

- The implementation is present in the working tree.
- No files have been staged.
- No commit has been created.
- The branch is still `main`.
- The crate version was bumped from `1.3.0` to `2.0.0`.
- `Cargo.lock` was updated by the Rust toolchain.
- Verification passed before this handoff file was created.
- The only new source module is `src/app/core_resources.rs`.
- The major known caveat is that deep internal splitting of `SpriteRenderer` and `UiSystem`
  was not completed. The implementation focused on runtime correctness, ECS/API, asset loading,
  scene reload resources, normal API removal, docs, and a small app resource helper extraction.
- The prior assistant already told the user this caveat in the final implementation response.
- GUI examples were not run interactively because they open windows and event loops.
- Example compilation smoke was run with `cargo check --examples` and passed.
- wasm verification was limited to library check, as requested in the plan.

## Working Tree At Handoff

`git status -s` showed:

```text
 M AGENTS.md
 M Cargo.lock
 M Cargo.toml
 M README.md
 M REFERENCE.html
 M docs/AGENT_WORKFLOW.md
 M docs/CHANGELOG.md
 M docs/ENGINE_TERMS_FOR_BEGINNERS.md
 M docs/ENTITY_GENERATION_V2_PLAN.md
 M docs/HANDOFF.md
 M docs/PATTERNS.md
 M docs/VISION.md
 M src/app.rs
 M src/asset.rs
 M src/components.rs
 M src/ecs/world.rs
 M src/particle.rs
 M src/prefab.rs
 M src/renderer/lighting.rs
 M src/renderer/sprite.rs
 M src/scripting.rs
 M src/steering.rs
?? src/app/
```

`git diff --stat` before this handoff file was added showed:

```text
 AGENTS.md                          |   9 +-
 Cargo.lock                         |   2 +-
 Cargo.toml                         |   2 +-
 README.md                          |  11 +-
 REFERENCE.html                     |  61 +++++------
 docs/AGENT_WORKFLOW.md             |  13 +--
 docs/CHANGELOG.md                  |  31 ++++++
 docs/ENGINE_TERMS_FOR_BEGINNERS.md |   4 +-
 docs/ENTITY_GENERATION_V2_PLAN.md  |  13 +--
 docs/HANDOFF.md                    |  17 ++-
 docs/PATTERNS.md                   |   6 +-
 docs/VISION.md                     |  17 +--
 src/app.rs                         | 218 +++++++++++++++++++------------------
 src/asset.rs                       |  12 ++
 src/components.rs                  |  15 +--
 src/ecs/world.rs                   | 138 +++++++++++++++++------
 src/particle.rs                    |   2 -
 src/prefab.rs                      |  27 ++++-
 src/renderer/lighting.rs           |  55 ++++++++--
 src/renderer/sprite.rs             |  24 +++-
 src/scripting.rs                   |  16 ++-
 src/steering.rs                    |   3 +-
 22 files changed, 445 insertions(+), 251 deletions(-)
```

This handoff file itself was created after the verification pass, so later `git status`
will also include:

```text
?? plans/handoffs/HANDOFF_v2-cleanup_2026-06-03.md
```

## Evidence And Verification

Commands run successfully before handoff creation:

| Command | Result |
| --- | --- |
| `cargo fmt --check` | passed |
| `cargo check --all-targets` | passed |
| `cargo test --all-targets` | passed |
| `cargo clippy --all-targets -- -D warnings` | passed |
| `cargo check --target wasm32-unknown-unknown --lib` | passed |
| `cargo check --examples` | passed |

Observed test count from the test run:

- Library tests: 267 passed.
- Example test target `mp_server`: 3 passed.

The implementation did not run interactive GUI examples. It only compile-checked examples.

## What Changed - ECS Entity Generation V2

File: `src/ecs/world.rs`

- Replaced `Entity(pub u32)` with an opaque struct:
  - `index: u32`
  - `generation: u32`
- Added public accessors:
  - `Entity::index(self) -> u32`
  - `Entity::generation(self) -> u32`
  - `Entity::from_raw_parts(index, generation) -> Entity`
- Reworked `World` allocation fields:
  - `next_index`
  - `free_indices`
  - `generations: Vec<u32>`
- `spawn()` now creates a live handle using the current slot generation.
- `despawn()` validates the exact handle before removing storage state.
- `despawn()` increments the slot generation and requeues the index for reuse.
- Stale handles are treated as missing/no-op by world mutation paths.
- `clone_entity(src)` now returns `Option<Entity>`.
- `clone_entity(stale)` returns `None`.
- Existing tests were updated for the breaking API.
- New/updated tests cover stale handle behavior and reused generation behavior.

Important next-review targets:

- Review all remaining `Entity::from_raw_parts` call sites and confirm they are only used at
  serialization/script/test boundaries.
- Confirm any future public docs explain that raw-part construction is for boundaries only.
- Watch for manually constructed handles in examples if new examples are added later.

## What Changed - Scripting API

File: `src/scripting.rs`

- Rhai `despawn_entity(id)` was removed.
- New Rhai API is `despawn_entity(index, generation)`.
- The script function converts the arguments through `Entity::from_raw_parts`.
- Negative values are ignored.
- Stale handles are eventually ignored by `World::despawn`.
- Script docs in the source were updated.

Potential follow-up:

- There is still no stable engine-issued script-side entity handle type. The current API exposes
  index/generation as two values by design for v2.0.

## What Changed - Entity Call Sites

Files touched:

- `src/app.rs`
- `src/renderer/sprite.rs`
- `src/steering.rs`
- `src/ecs/world.rs`

Examples of changes:

- Inspector/editor labels now display `index:generation`.
- Duplicate action now handles `world.clone_entity(sel)` returning `Option<Entity>`.
- Render debug material descriptions use `entity.index()` and `entity.generation()`.
- Steering deterministic wander seed uses `entity.index()`.

Search check used:

```text
rg -n "from_raw_parts|generation\\(|clone_entity|despawn_entity" AGENTS.md README.md REFERENCE.html docs src
```

This found expected remaining public docs, tests, and implementation references.

## What Changed - Normal Map Public API Removal

Files touched:

- `src/components.rs`
- `src/particle.rs`
- `src/prefab.rs`
- `README.md`
- `REFERENCE.html`
- `docs/CHANGELOG.md`
- `docs/ENGINE_TERMS_FOR_BEGINNERS.md`
- `docs/HANDOFF.md`

Implementation summary:

- Removed public `Sprite.normal_texture`.
- Removed public `Sprite.normal_handle`.
- Removed constructor initialization of these fields.
- Updated particle sprite literals that directly constructed `Sprite`.
- Kept internal flat-normal lighting buffer behavior.
- Clarified docs so normal maps are not presented as an implemented public sprite feature.

Scene compatibility:

- `SCENE_DEF_VERSION` was bumped from `1` to `2`.
- A fixture-like test was added so legacy v1 `SceneDef` data containing `normal_texture`
  still deserializes and ignores the removed field.

Potential follow-up:

- `docs/HANDOFF.md` is a historical document and still contains older phase notes that mention
  old normal-map fields. A v2 note was added, but the historical sections were not rewritten.

## What Changed - Scene Reload And Core Resources

Files touched:

- `src/app.rs`
- `src/app/core_resources.rs`

New file:

- `src/app/core_resources.rs`

Implementation summary:

- Extracted core resource insertion into `insert_core_resources(world)`.
- Extracted core component metadata registration into `register_core_component_metadata(world)`.
- `App::new()` now uses the helper.
- `reload_scene()` now uses the helper after replacing world state.
- `reload_scene()` preserves `DebugUi`.
- `reload_scene()` re-adds runtime/debug resources that could previously disappear.
- Added a test that scene reload restores core resources and preserves debug UI state.

Known limitation:

- This is the only structural module split that was completed. The broader debug/editor/asset-browser
  split was not fully done.

## What Changed - AssetServer GPU Upload Path

Files touched:

- `src/asset.rs`
- `src/app.rs`
- `src/renderer/sprite.rs`

Implementation summary:

- Added `AssetServer::image_assets_for_gpu()`.
- Added `SpriteRenderer::has_texture_key()`.
- Added lazy GPU upload path from loaded `AssetServer` images into `SpriteRenderer`.
- `App::load_image` pending upload queue remains in place.
- `finish_init()` uploads already-loaded `AssetServer` image assets.
- The async load polling flow also uploads newly loaded `AssetServer` image assets.

Why this mattered:

- Previously `App::load_image` pending upload was effectively the only reliable route to GPU
  texture cache insertion.
- Images loaded through `AssetServer::load_image` could become CPU-loaded assets without ever
  entering the renderer texture cache.

Potential follow-up:

- There is no dedicated renderer-independent asset upload test. The path is currently covered by
  app-level structure and compile/test coverage rather than a narrow GPU cache unit test.

## What Changed - Render Flow And Lighting

Files touched:

- `src/app.rs`
- `src/renderer/lighting.rs`

Implementation summary:

- Render order is now explicitly treated as `scene -> post(optional) -> lighting(optional) -> final`.
- Post+lighting routing now feeds post output into lighting.
- Lighting-only path uses an intermediate scene texture.
- Scene intermediate textures are recreated when viewport size or format changes.
- Post+lighting clears the old lighting scene texture handoff when post is active.
- Lighting output is then composited to the final swapchain target.

Lighting semantic update:

- `PointLight` positions are treated as world-space values.
- Light uniform generation now considers camera position.
- Light uniform generation now considers camera zoom.
- Light uniform generation considers camera shake offset through the camera helper.
- Added a helper test for camera-transformed light positioning.

Potential follow-up:

- The plan asked for helper tests covering post-only, lighting-only, and post+lighting target routing
  "where possible." The implemented tests focus on the camera-space light helper and resource reload.
  Render routing is still mostly verified by compile and structural review, not by a narrow unit test.

## What Changed - Sprite Renderer Internals

File: `src/renderer/sprite.rs`

Implementation summary:

- The material parameter buffer cache is now keyed by `Entity`, not only by index.
- This avoids stale/reused-index collisions after entity generation v2.
- Test helper entity construction was updated to `Entity::from_raw_parts(entity_id, 0)`.
- Added `has_texture_key()` for app-side lazy upload deduplication.

Not completed:

- The larger requested split of `SpriteRenderer` into world sprite rendering, UI rendering,
  and render-target/material management submodules was not done.
- This should be treated as deferred structural cleanup, not a runtime blocker.

## What Changed - Docs And Agent Instructions

Files touched:

- `AGENTS.md`
- `docs/AGENT_WORKFLOW.md`
- `README.md`
- `REFERENCE.html`
- `docs/CHANGELOG.md`
- `docs/ENTITY_GENERATION_V2_PLAN.md`
- `docs/HANDOFF.md`
- `docs/PATTERNS.md`
- `docs/VISION.md`
- `docs/ENGINE_TERMS_FOR_BEGINNERS.md`

Highlights:

- `AGENTS.md` version is now v2.0.0.
- `AGENTS.md` no longer contains `rust-survivors` related-project guidance.
- `docs/AGENT_WORKFLOW.md` now says external projects are checked only on explicit user request
  or clear task scope.
- `README.md` includes v2 breaking-change notes.
- `REFERENCE.html` includes entity accessors, new scripting API, updated clone behavior, and
  flat-normal lighting language.
- `docs/CHANGELOG.md` includes `2.0.0`.
- `docs/VISION.md` playable example status was aligned with current repo direction.
- `docs/PATTERNS.md` scene transition example uses `scene_change.request(...)`.
- Beginner glossary explains entity generations and that public normal map API is not exposed.

Important caveat:

- `REFERENCE.html` still had search hits showing `SCENE_DEF_VERSION = 1` in the versioning section
  during the final grep check. This may need correction to `2` if it was not already changed after
  that grep. Before closing the task, re-run:

```text
rg -n "SCENE_DEF_VERSION = 1|현재 = 1|current = 1" REFERENCE.html README.md docs src
```

If any non-historical public reference remains, update it.

## Known Caveats And Risks

- The work is large and currently unstaged. Use focused review before committing.
- The deep internal module split for `SpriteRenderer` and `UiSystem` is not completed.
- `REFERENCE.html` is manually edited HTML and can easily drift from source APIs.
- `docs/HANDOFF.md` is historical and still contains legacy mentions of normal-map fields and
  `rust-survivors`; this is mostly historical context but can confuse future searches.
- External downstream validation was intentionally skipped by user decision.
- Interactive renderer examples were not run.
- Runtime render routing needs visual or GPU-level smoke testing when convenient.
- `Entity::from_raw_parts` is intentionally public, but misuse can still create stale handles.
- Script API consumers must be updated manually if they still call old `despawn_entity(id)`.
- Any local examples or docs outside the searched path may still show `entity.0`.

## Open Questions

- Should the deferred `SpriteRenderer`/`UiSystem` structural split be done before the v2 commit,
  or moved to a follow-up v2.0.x cleanup?
- Should `docs/HANDOFF.md` historical sections be left as history, or annotated more aggressively
  to avoid old normal-map and `rust-survivors` guidance being rediscovered as current truth?
- Should `CLAUDE.md` receive the same v2 quick-reference cleanup as `AGENTS.md`?
- Should a small non-window render-routing unit test harness be added, or is compile plus manual
  smoke enough for this cleanup?
- Should script-side entity identity eventually become a string or object handle instead of
  two integer arguments?

## Suggested Next Action

Start by re-running the stale-doc scans and verification because this handoff file was added
after the previous test pass:

```text
rg -n "entity\\.0|despawn_entity\\([^,\\n]+\\)|normal_texture|normal_handle|SCENE_DEF_VERSION = 1|현재 = 1|SceneChange\\.0" README.md REFERENCE.html docs src examples
cargo fmt --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo check --target wasm32-unknown-unknown --lib
cargo check --examples
```

Then inspect the key diffs in this order:

```text
git diff -- src/ecs/world.rs
git diff -- src/scripting.rs src/components.rs src/prefab.rs
git diff -- src/app.rs src/app/core_resources.rs src/asset.rs src/renderer/sprite.rs
git diff -- src/renderer/lighting.rs
git diff -- AGENTS.md README.md REFERENCE.html docs
```

If the user wants commits, commit by subsystem rather than one giant commit:

1. ECS/entity generation and scripting.
2. Normal public API removal and scene version compatibility.
3. Render flow, lighting, scene reload, and asset upload.
4. Docs and agent instructions.
5. Optional follow-up structural split if completed.

## Do Not Change Scope

- Do not validate or update `rust-survivors` unless the user explicitly asks.
- Do not add new public renderer/widget extension APIs while doing internal splits.
- Do not reintroduce `Sprite.normal_texture` or `Sprite.normal_handle`.
- Do not revert user changes with destructive Git commands.
- Do not stage or commit without explicit user request.
- Do not treat historical docs as authoritative over the newly updated v2 docs.

## Quick Start For The Next Agent

1. Read this file.
2. Run `git status -s`.
3. Run the stale-doc `rg` command from Suggested Next Action.
4. Fix any current-doc references that still describe v1 APIs as current.
5. Re-run the required verification commands.
6. Decide with the user whether to leave the deferred internal renderer/UI split for a follow-up.
7. If asked to commit, split commits by subsystem.

## Self-Check

- This handoff records the user decisions that shaped the v2 cleanup.
- It lists the current dirty working tree.
- It records exact verification commands and results.
- It names the incomplete part of the plan.
- It includes concrete restart commands.
- It avoids claiming that commits or staging happened.
- It identifies one specific stale-doc risk to re-check: `SCENE_DEF_VERSION = 1` in `REFERENCE.html`.
