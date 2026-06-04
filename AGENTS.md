# AGENTS.md — skeleton-engine agent reference

> Version v2.0.0 | package `skeleton-engine` | library crate `engine`
> wgpu-based Rust 2D game engine | Full API: `REFERENCE.html` | structure: `ARCHITECTURE.html` | dev history: `docs/HANDOFF.md`

## Project direction (read `docs/VISION.md`)

A **skeleton**: a hackable, MIT-licensed, genre-agnostic 2D engine meant to be forked
and extended. Core feature-work loop: **a new feature is not done until a small,
playable example game in `examples/` exercises it in real play**; if the API feels
awkward while writing that example, fix the API before release. Keep new code
fork-friendly (clear module boundaries, extension points). See `docs/VISION.md`.

## Module map

| Looking for | File |
| --- | --- |
| Engine entry point, main loop, render orchestration, `load_image` | `src/app.rs`, `src/app/` |
| `Handle<T>`, `ImageAsset`, `ScriptAsset`, `AssetServer` | `src/asset.rs`, `src/asset/` |
| `TextureAtlas`, `AtlasSprite` | `src/atlas.rs` |
| `Reflect`, `ReflectValue` | `src/reflect.rs` |
| `ScriptAsset`, `ScriptRunner`, `ScriptingSystem` | `src/scripting.rs`, `src/scripting/` |
| `DebugUi` | `src/debug_ui.rs` |
| Full public API re-export list | `src/lib.rs` |
| `Entity`, `Component`, `Resource`, `Query` | `src/ecs/world.rs`, `src/ecs/world/` |
| Event bus `Events<E>` | `src/ecs/events.rs` |
| `System` trait | `src/ecs/system.rs` |
| Scene transitions: `Scene`, `SceneCmd`, `SceneChange` | `src/scene.rs` |
| `Transform`, `Sprite` | `src/components.rs` |
| `WindowConfig`, `GameState`, `ShouldQuit`, `DebugDrawQueue` | `src/resources.rs` |
| `Camera` | `src/camera.rs` |
| `InputState`, `InputMap` | `src/input/` |
| `GamepadState`, `GamepadButton`, `GamepadAxis` | `src/input/gamepad.rs` |
| `PhysicsWorld`, `PhysicsBody`, `PhysicsSystem`, `CollisionEvent` | `src/physics/` |
| `CharacterController`, `RaycastHit`, raycast, character movement | `src/physics/character.rs`, `src/physics/world.rs`, `src/physics/world/` |
| `add_kinematic_box`, `add_kinematic_circle` | `src/physics/world.rs`, `src/physics/world/` |
| `SpatialGrid`, `Collider`, `CollisionLayer` | `src/collision/` |
| `AnimationPlayer`, `AnimationClip`, `AnimationSystem`, `BlendWeight` | `src/animation/player.rs`, `src/animation/system.rs` |
| `AnimationStateMachine`, `StateMachineSystem`, `TransitionCond`, `AnimParam` | `src/animation/state_machine.rs` |
| `BlendTree1D`, `BlendEntry`, `BlendTreeSystem` | `src/animation/blend_tree.rs`, `src/animation/blend_system.rs` |
| `SkeletalAnimator`, `SkeletalClip`, `BoneTrack`, `SkeletonBuilder`, `SkeletalAnimationSystem` (2D cutout) | `src/skeletal.rs` (details: `docs/SKELETAL.md`) |
| UI: `UiNode`, `Button`, `Label`, `TextInput`, `ScrollView`, `Panel`, `LayoutSystem`, `UiEvent` | `src/ui/`, `src/ui/system/` |
| `Slider`, `CheckBox` | `src/ui/slider.rs`, `src/ui/checkbox.rs` |
| `Tag`, `EntityDef`, `SceneDef`, `Prefab`, prefab spawn functions | `src/prefab.rs` |
| `Timer`, `Tween`, `Easing` | `src/timer.rs`, `src/tween.rs` |
| `ParticleEmitter`, `ParticleSystem` | `src/particle.rs` |
| `Tilemap`, `TilemapAtlas`, `TilemapSystem` | `src/tilemap.rs` |
| `AudioManager` | `src/audio.rs`, `src/audio/` |
| save/load API, `SaveError` | `src/save.rs` |
| `PostProcessConfig`, `PostProcessRenderer` | `src/renderer/post_process.rs` |
| wgpu render pipeline, `SpriteRenderer` internals | `src/renderer/`, `src/renderer/sprite/` |

## Core patterns & task recipes

Detailed in **`docs/PATTERNS.md`**:

- **Architecture patterns** — ECS query API (`query2`/`query_opt2`), borrow-checker
  workaround (collect entities then `get_mut`), render-layer separation
  (`AnimationSystem` → `UvRect` → renderer), UI system order (`LayoutSystem` before
  `UiSystem`), animation state-machine order (`StateMachineSystem` after
  `AnimationSystem`), `PhysicsWorld` encapsulation accessors.
- **Task recipes** — adding a component / system / resource / event, and scene transitions.

## Agent working notes

Follow `docs/AGENT_WORKFLOW.md` for detailed operating rules. `AGENTS.md` is a quick
reference, so keep it **under 200 lines**. Never drop needed content just to hit the
limit — when it would overflow, move detail into a `docs/*.md` (e.g. `docs/PATTERNS.md`)
and leave only a summary and link here.

### Default flow

- Proceed in order: explore → scope → plan if needed → implement → verify → report summary.
- Locate symbols/keywords with `rg` before reading whole files; default reading order is `src/lib.rs` → module map → target file.
- Handle single-file edits with clear requirements directly in the main session.
- Use subagents freely for: exploring 3+ files, changing multiple subsystems, implementing after a long conversation, or work that benefits from parallel review.
- If public API/usage/examples are affected, check whether related docs need updating.
- Before declaring done, run the **Verification** checks below (CI-equivalent).
- stage/commit/push only on user request.
- Confirm beforehand: public API removal/rename, dependency/version changes, large refactors, file deletion, destructive Git.
- Subagent prompts must include file paths, patterns to apply, expected behavior, and the do-not-change scope.

### Verification (run before declaring done)

A change is "done" only after the **CI-equivalent** checks pass *locally* (CI in
`.github/workflows/ci.yml` enforces them on push — run them before committing):

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --target wasm32-unknown-unknown   # lib+bins, NOT --all-targets
cargo test --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps  # CI gate — broken intra-doc links fail
```

Or `./scripts/verify.sh`. The wasm gate uses lib+bins, not `--all-targets`: the
native-only examples (`platformer_game`/`mp_server`/`gpu_particles`) don't compile to
wasm. Don't narrow the bar to `fmt --check` + `test --lib` — that misses wasm/clippy
regressions.

## Documentation structure
Instruction files that agents must auto-detect live at the repo root. General docs are collected under `docs/`.

| Location | Purpose |
| --- | --- |
| `AGENTS.md` | Codex/agent shared quick reference: module map, task checklists |
| `CLAUDE.md` | Quick reference for Claude-family agents |
| `docs/PATTERNS.md` | Core architecture patterns + task recipes (extracted from the quick refs) |
| `README.md`, `REFERENCE.html`, `ARCHITECTURE.html`, `docs/HANDOFF.md` | Intro/usage, public API, engine structure, per-phase dev history |
| `docs/CHANGELOG.md`, `docs/ROADMAP.md` | Release change log, v1.0 historical roadmap |
| `docs/AGENT_WORKFLOW.md` | Detailed agent operating rules |
| ignored local docs | `.gitignore`d work prompts / personal plans. Not referenced as official docs |

**Documentation language**: Write doc prose in **English** to minimize token cost
(English ≈ ⅓ the tokens of equivalent Korean). Code, file paths, identifier tables, and
API names stay as written. Exceptions kept in Korean: the beginner glossary
(`docs/ENGINE_TERMS_FOR_BEGINNERS.md`) and personal/gitignored one-off prompt or plan
docs. New `docs/HANDOFF.md` entries are written in English. Prefer concision.

> **Growth strategy**: when content would push `CLAUDE.md`/`AGENTS.md` past 200 lines,
> move detail into a `docs/*.md` (a new subsystem doc, or `docs/PATTERNS.md`) and leave
> only a one-line reference here. Never drop needed content just to stay under the limit.
