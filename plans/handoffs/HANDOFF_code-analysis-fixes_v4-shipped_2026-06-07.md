# Code-analysis epic COMPLETE — perf/robustness/API (PR #9), examples (PR #10), #28 JointHandle → v4.0.0 (PR #11), rust-survivors migrated

**Date:** 2026-06-07
**Status:** COMPLETED — all 30 code-analysis issues addressed; `skeleton-engine` `main` is now **v4.0.0**; `rust-survivors` migrated v2→v4.
**Bead(s):** none (`bd` unavailable)
**Epic:** code-analysis remediation (`docs/CODE_ANALYSIS.md`)
**Chain:** `code-analysis-fixes` seq `3`
**Parent:** `HANDOFF_code-analysis-fixes_v3-breaking-batch_2026-06-06.md` (seq 2)
**Prior chain:** seq 1 `high-severity-bugs` → seq 2 `v3-breaking-batch` → **seq 3 `v4-shipped` (this)**

---

## Since Last Handoff

Seq 2 left a phased PLAN for the **12 remaining non-breaking issues** plus the one deferred breaking item (#28). This session executed all of it and then went further than planned:

1. **Executed the seq-2 plan in full (Phases 1–3) as PR #9** — perf (#7/#8/#18/#19), robustness (#20/#22/#23), additive API (#12/#27/#29/#30). **Discovered #15 is a FALSE POSITIVE** (lighting radius already correct) and did NOT apply the plan's suggested fix.
2. **Added PR #10** — made examples actually use the new APIs (VISION "real play" rule): `loading_bar` uses `DrawText::centered`, `minimap` gets `world_to_screen` nameplates, plus a new `audio_fades` example.
3. **Did the deferred breaking item #28 as PR #11 → v4.0.0** — `JointHandle` newtype, plus finished #27's example English-ification (C) and project housekeeping (D: CODE_ANALYSIS/HANDOFF/CLAUDE.md).
4. **Migrated `rust-survivors` v2→v4** — only #11's Color newtype affected it; 200 tests pass; committed locally (not pushed).

Net: the entire `docs/CODE_ANALYSIS.md` epic (30/30) is closed across 5 merged PRs (#7/#8/#9/#10/#11). The engine is v4.0.0. The downstream game builds and tests against it.

## Reference Documents

- `docs/CODE_ANALYSIS.md` — the 30-issue report; now carries a **Resolution status** block (added this session) mapping every issue to its PR. Original findings left as the as-of snapshot.
- `plans/handoffs/HANDOFF_code-analysis-fixes_v3-breaking-batch_2026-06-06.md` (seq 2) — the remaining-13 backlog table with file:line + fix sketches; toolchain investigation.
- `plans/handoffs/PLAN_code-analysis-fixes_v3-breaking-batch_2026-06-06.md` (seq 2) — the 3-phase plan this session executed.
- `docs/HANDOFF.md` — per-phase dev history; new 2026-06-06 entry summarizes this work; header bumped to v4.0.0.
- `CLAUDE.md` — module map (updated: JointHandle, AudioSystem, TextAnchor/centered, world_to_screen, ReflectValue I32); 5-command gate (use the `+1.88.0` caveat — memory `ci-toolchain-pin`).
- Memory: `ci-toolchain-pin`, `v3-breaking-batch` (now marked epic-complete), `rust-survivors-engine-pin` (now marked migrated to v4), `subagent-usage-preference`, `playtest-windowed-examples`.

## The Goal

Close the code-analysis remediation epic so the "skeleton" engine stays trustworthy for forkers (VISION priority 1), and bring the downstream game (`rust-survivors`) up to the current engine so the breaking v3/v4 changes are proven in real use. Both achieved.

## Where We Are

- **`skeleton-engine` `main` = `60328fa`, version `4.0.0`, working tree clean.** PRs #9/#10/#11 all merged via rebase (linear history). No open PRs; feature branches deleted.
- **All 30 analysis issues addressed.** 29 in PRs #7/#8/#9; **#28** in PR #11 (the only breaking one → major bump). **#15 was a false positive** (no code change; locked with a contract test).
- **Test counts (engine, `cargo +1.88.0 test --lib`):** 293 (start) → **296** (PR#9 Phase1, +3) → **306** (Phase2, +10) → **311** (Phase3, +5). Aggregate `test --all-targets` = 311 lib + others, 0 failures. Doctests 33 → **34**.
- **New public API (v4):** `JointHandle` (newtype wrapping rapier `ImpulseJointHandle`); `AudioSystem` (ticks `AudioManager::update`); `DrawText::centered` + `TextAnchor`; `Camera::world_to_screen`; `engine::MouseButton` re-export; expanded `ScriptingLimits`; `ReflectValue::I32` + `#[non_exhaustive]`.
- **Removed (breaking):** `engine::ImpulseJointHandle` (use `engine::JointHandle`).
- **`rust-survivors`:** pin bumped v2 (`61c09f1`) → v4.0.0 (`60328fa`); 12 Color sites converted; **200 tests pass**; committed `da11775` on its `main` (selective — only migration files), **NOT pushed**; its 21 unrelated WIP files left untouched.

## What We Tried (Chronological)

1. **PR #9 Phase 1 (perf, branch `fix/analysis-perf`):**
   - **#19** `CollisionGridSystem`: replaced the per-frame `world.insert_resource(self.grid.clone())` (deep-clones 2 HashMaps) with the **PhysicsWorld remove→rebuild→insert pattern** — the system owns only `cell_size`; the grid lives in the resource between frames and `rebuild()` reuses its allocations. `CollisionDebugSystem` now reads the mirror when present, falls back to its own rebuild when standalone. **Resource type stays `SpatialGrid` → zero reader breakage** (4 example readers unaffected).
   - **#7** Rhai: `ScriptAsset.ast` → `Arc<rhai::AST>` (clone = refcount bump). Dropped the `4×Arc<Mutex>` per scripted-entity-per-frame — `ScriptCtx` now holds plain buffers (single-threaded ECS), and `ScriptingSystem::run` reuses 4 scratch buffers moved through the thread_local via `set_script_ctx`/`take_script_ctx`. Rewrote `api.rs` (mutators use `borrow_mut`) and `scripting/tests.rs`.
   - **#18** A*: added a `closed: HashSet` (skip stale duplicate pops / re-expansion) and a thread_local `AStarScratch` reused across calls — **`find_path` signature unchanged** (non-breaking).
   - **#8** sprite renderer: hoisted ONE `begin_render_pass` to span the whole pre-sorted entry stream (was a new pass per texture-run AND per material — a full attachment load+store each). Matches the existing UI-primitive pass. Guarded with `if !entries.is_empty()`.
   - Verified full `+1.88.0` gate. Commit `db4abb0`.
2. **PR #9 Phase 2 (robustness):** #20 (`AudioSystem` + `read_cached_bytes` file-bytes cache), #22 (`layer_matches_mask` drops `clamp(0,31)`; out-of-range layers excluded under a non-zero mask + warn-once), #23 (`spawn_scene_def` first-wins dup-tag + `log::warn!`). **#15 investigated → FALSE POSITIVE** (see Key Decisions). Commit `5cce7a3`.
3. **PR #9 Phase 3 (additive API):** #27 (`pub use winit::event::MouseButton` + 5 examples to top-level imports + `gpu_particles` `.unwrap()`→`let-else`), #12 (`DrawText::centered`/`TextAnchor` measured from the shaped buffer + `Camera::world_to_screen` + screen-vs-world docs), #30 (expanded `ScriptingLimits`: string/array/map/call/expr-depth), #29 (`ReflectValue` `#[non_exhaustive]` + `I32` + editor arm). Hit a broken intra-doc link (`Camera::world_to_screen` didn't exist yet) → **added `world_to_screen`** (the inverse of `screen_to_world`). Commit `239fd91`. **PR #9 merged (rebase).**
4. **Local sync hiccup after merge:** `git checkout main` reverted the tree to the unpushed seq-2 session commit; `git pull --ff-only` failed (rebase made new SHAs). Fixed with `git reset --hard origin/main` (the only diverging local commit was content-identical to its rebased twin). See Gotchas.
5. **PR #10 (examples exercise new APIs, VISION):** `loading_bar` → `DrawText::centered`; `minimap` → `WorldLabelSystem` drawing `ENEMY` nameplates via `world_to_screen`+`centered`; new `examples/audio_fades.rs` (native-only, `AudioSystem`-driven fades). Merged `02cc9e2`.
6. **PR #11 (A/C/D → v4.0.0):**
   - **A #28:** `JointHandle(pub(crate) ImpulseJointHandle)` in `src/physics/world.rs` (mirrors `CollisionGroups`); `add_*_joint` return it, `remove_joint` takes it; removed the rapier re-export from `physics/mod.rs` + `lib.rs`; bumped `Cargo.toml` 3.0.0→4.0.0; updated `physics/world/tests.rs` (`h.0`). Commit `0893ce0`.
   - **C #27 remaining:** English-ified the 5 examples' Korean comments via **5 parallel Sonnet subagents** (comments-only). Independently verified: 0 Korean left, fmt/clippy/build green, diff comments-only. Commit `e4e0e1a`.
   - **D housekeeping:** CODE_ANALYSIS resolution block, HANDOFF entry + version, CLAUDE.md module-map rows + version header (184 lines, under the 200 cap). Commit `60328fa`. **PR #11 merged (rebase).**
7. **rust-survivors v2→v4:** bumped the pin; `cargo check` surfaced 8 Color errors (lib/bins), `cargo test --no-run` surfaced 4 more in test code — **all #11 Color newtype**. PhysicsWorld (#13) inert (game owns a raw `PhysicsWorld`); joints (#28) unused. Converted all 12 with `Color::from`/`Color::rgba`/`Color::WHITE`/`.to_array()`. `cargo test --workspace` = **200 passing**. Committed `da11775` (selective).

## Key Decisions

- **#15 is a false positive — do NOT "fix" it.** The plan/analysis claimed point lights render ~½ size and to change the CPU radius to `2*radius/viewport_w`. Full derivation (`src/renderer/lighting.rs`):
  - CPU `light_position_ndc`: `screen = (pos-cam)*zoom` (pixels); `ndc = screen/(vp/2) - 1`; `radius_ndc = radius*zoom/viewport_w`.
  - Shader: `uv_light = position_ndc*0.5+0.5` (NDC→UV [0,1]); `diff_uv = uv_light - in.uv` (both UV); `d = length(diff_uv.x, diff_uv.y*aspect)` with `aspect = vp_h/vp_w`; `atten = 1 - d/radius_ndc`.
  - So `d` is in **UV fraction-of-width** (NDC spans 2 across width, UV spans 1). A world radius `r` = `r*zoom` px = `r*zoom/viewport_w` UV — **exactly** the CPU value. They share one space; falloff reaches 0 at the world radius. The "NDC" name is a misnomer (the value is UV, not NDC). The suggested `2*radius` would render lights **2× too large**.
  - Concrete check: zoom=1, 800×600, r=100 → `radius_ndc = 100/800 = 0.125`; a fragment 100 px right is `diff_uv.x = 100/800 = 0.125` → `atten=0` at exactly 100 world px. (Existing test `light_position_uses_camera_transform` already asserts `radius == 0.125`.)
  - Confirmed by an **independent general-purpose subagent re-derivation** (given only the file + the question, no hint of my conclusion) → verdict CORRECT, would-be-double if changed.
  - Action: **no math change**; clarified the `GpuLightData::radius_ndc` field doc + the `light_position_ndc` comment; added contract test `light_radius_falloff_reaches_zero_at_world_radius` that replicates the shader's atten and asserts `radius_uv == edge_d_uv` (fails if anyone applies the 2× change).
- **#19 used remove/rebuild/insert, not `Arc<SpatialGrid>`.** The plan offered either; the chosen pattern keeps the resource type `SpatialGrid` (no reader breakage) AND reuses allocations AND matches the established PhysicsWorld pattern. Strictly better than the Arc option (which was mild-breaking for the 4 example readers).
- **#7 dropped the Mutex entirely.** The thread_local ctx is single-threaded (ECS), so `Arc<Mutex<_>>` was redundant — plain buffers moved in/out give the same sharing with zero locks/allocs per entity.
- **#18 used a thread_local scratch, not a `&mut PathGrid` param.** Keeps `find_path`'s signature (non-breaking) while still reusing the open list / score maps across calls.
- **#28 accepted the major bump (3.0.0 → 4.0.0).** Breaking by nature; done last and alone so it cleanly owns the version bump (the seq-2 plan deliberately kept it out of the non-breaking PR #9).
- **rust-survivors committed selectively, not pushed.** The repo carries 21 unrelated WIP files on `main`; staged only the 9 migration files (Cargo.toml/lock + 7 src) for a clean isolated commit; left WIP and the remote untouched for the user to organize.
- **Color conversions use explicit `Color::from`/`rgba`, never bare `.into()`.** rapier→simba's `From<[T;N]>` makes bare `arr.into()` ambiguous (E0283) in physics-linked crates; explicit constructors avoid it.

## Evidence & Data

### Engine commits on `main` (this session)

| SHA | Commit | PR |
|-----|--------|----|
| `03e04ca` | perf: remove per-frame hot-path costs (#7/#8/#18/#19) | #9 |
| `cd19d98` | fix: subsystem robustness — audio, layer-mask, prefab tags (#20/#22/#23) | #9 |
| `185049c` | feat: additive API polish — MouseButton, text anchor, Rhai limits, ReflectValue (#12/#27/#29/#30) | #9 |
| `02cc9e2` | examples: exercise Phase 3 APIs in real play (VISION compliance) | #10 |
| `0893ce0` | feat(physics)!: wrap rapier ImpulseJointHandle in engine JointHandle newtype (#28) | #11 |
| `e4e0e1a` | docs(examples): English-ify Korean comments in the #27 examples | #11 |
| `60328fa` | docs: record code-analysis completion + v4.0.0 (CODE_ANALYSIS, HANDOFF, CLAUDE.md) | #11 |

(`132e33e` = the rebased seq-2 session commit, now the base.)

### rust-survivors

- `da11775` "Migrate to skeleton-engine v4.0.0 (Color newtype)" on `main` (local, **unpushed**). Pin: `crates/game/Cargo.toml` `rev = 60328fa…`. `cargo test --workspace` = 200 passed.

### Issues resolved (30/30)

| Severity | Resolved |
|----------|----------|
| HIGH (6) | #1 #2 #3 #4 #5 #6 (PR #7) |
| MEDIUM (16) | #9 #10 #11 #13 #14 #16 #17 #21 (PR#7/#8) · #7 #8 #12 #18 #19 #20 #22 (PR#9) · #15 (false positive, PR#9) |
| LOW (8) | #24 #25 #26 (PR#7) · #23 #27 #29 #30 (PR#9) · #28 (PR#11, v4.0.0) |

### CI

PRs #9/#10/#11 each merged with **4/4 green** (Test native, Build WASM, Rustdoc, Package dry-run), rebase-merge, branch auto-deleted.

### New tests added this session (engine, +18 total: 293→311)

grid-mirror identity; A* optimality + scratch-reuse non-contamination; 4 layer-mask (incl. #22 negative-background + OffscreenCamera exclusion); prefab first-wins; lighting radius contract; 4 audio (cache reuse/miss + AudioSystem no-op + device fade); DrawText centered/anchor ×2; Camera world_to_screen round-trip; ReflectValue I32; ScriptingLimits defaults.

## Code Analysis (key API shapes)

- `JointHandle` (`src/physics/world.rs`): `#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub struct JointHandle(pub(crate) ImpulseJointHandle);` — opaque; only obtainable from `add_*_joint`.
- `AudioSystem` (`src/audio.rs`): unit struct; `run` = `if let Some(a) = world.resource_mut::<AudioManager>() { a.update(dt); }`. `AudioManager.file_cache: HashMap<String, Arc<[u8]>>`; `read_cached_bytes(&mut cache, path)` is a device-free free fn (CI-testable).
- `TextAnchor` (`src/renderer/text.rs`): `TopLeft`(default)/`Center`. `DrawText::centered(..)` sets `anchor=Center, align=Center`. Render offsets `position` by half the shaped buffer size when Center.
- `Camera::world_to_screen(world) = (world - position) * zoom` (exact inverse of `screen_to_world`).
- `ScriptingLimits`: + `max_string_size`(64KiB)/`max_array_size`/`max_map_size`(100k)/`max_call_levels`(64)/`max_expr_depth`(128).
- `ReflectValue`: `#[non_exhaustive]`, variants `F32 I32 Vec2 Bool String Color`.
- Lighting `radius_ndc` (misnomer): is UV-fraction-of-width; the value `radius*zoom/viewport_w` is correct as-is.

## Files Changed (this session)

All engine changes are merged to `main` — see the commit table; `git show <sha> --stat` for per-commit files. This handoff + its PLAN are the only new uncommitted files (committed at session close).

### Engine, per-commit (key files)

- `03e04ca` (#7/#8/#18/#19): `src/collision/grid.rs` (+debug.rs), `src/scripting/{execution,context,api,tests}.rs`, `src/asset.rs` + `src/asset/script_loading.rs` (`Arc<AST>`), `src/pathfinding.rs`, `src/renderer/sprite.rs`.
- `cd19d98` (#20/#22/#23 + #15): `src/audio.rs` + `src/audio/{playback,tests}.rs`, `src/renderer/sprite/sort.rs` + `src/components.rs` (layer-mask), `src/prefab.rs`, `src/renderer/lighting.rs` (#15 docs+test only).
- `185049c` (#12/#27/#29/#30): `src/lib.rs` (+`MouseButton`/`TextAnchor`), `src/renderer/text.rs` + `src/renderer/mod.rs`, `src/camera.rs` (`world_to_screen`), `src/scripting.rs` + `src/scripting/api.rs` (limits), `src/reflect.rs` + `src/app/editor/ui/mod.rs` (I32), `src/physics/mod.rs` (#28 TODO), `examples/{gpu_particles,mp_client,minimap,split_screen,loading_bar}.rs`.
- `02cc9e2` (examples): `examples/{loading_bar,minimap}.rs`, `examples/audio_fades.rs` (new).
- `0893ce0` (#28): `Cargo.toml`+`Cargo.lock` (4.0.0), `src/physics/world.rs` (JointHandle), `src/physics/world/{joints,tests}.rs`, `src/physics/mod.rs`, `src/lib.rs`.
- `e4e0e1a` (#27 C): `examples/{gpu_particles,mp_client,minimap,split_screen,loading_bar}.rs` (comments only).
- `60328fa` (D): `docs/CODE_ANALYSIS.md`, `docs/HANDOFF.md`, `CLAUDE.md`.

### rust-survivors migration (`da11775`, the 12 Color sites)

`crates/game/Cargo.toml` (pin v2→v4) + `Cargo.lock`. Source (`crates/game/src/`):
- write `sprite.color = arr` → `engine::Color::from(arr)`: `survivor/background.rs:79` (`tile_color()`), `survivor/particle.rs:77,154`, `survivor/sprites.rs:531`, `survivor/weapon.rs:50`.
- write array-literal → `engine::Color::rgba(..)`: `survivor/damage.rs:47`, `survivor/weapon.rs:69`.
- write white literal → `engine::Color::WHITE`: `survivor/weapon.rs:705` (test).
- read `Color`→`[f32;4]` via `.to_array()`: `survivor/damage.rs:43` (HitFlash stores `[f32;4]`), `survivor/weapon.rs:731` (test), `survivor/ui_icons.rs:544` (test), `lib.rs:367` (test, on the `.map`).
NOT touched (pre-existing WIP): `survivor/powerup.rs`, `survivor/README.md`, + 19 docs.

## User Feedback & Preferences

- Terse, high-trust "keep going" energy throughout: `진행 해` / `진행해` repeatedly authorized each consequential step (commit → push → PR → merge). Authorized the breaking v4.0.0 and the rust-survivors migration explicitly (`a,c,d 순서대로 모두 진행`, then `진행해`).
- **Korean for conversational summaries, English for artifacts** (doc-language rule) — honored throughout; final reports were in Korean on request (`보고사항 한글로 알려줘`).
- Wanted the work fully landed (merge, not just PR) — confirmed merge via AskUserQuestion once, then standing `진행해`.
- Standing prefs (memory): subagents aggressively for parallel work (Sonnet) — used 5 for example translation + 1 for the lighting re-derivation; verify before declaring done — re-ran the real `+1.88.0` gate after every batch.

## Where We're Going

The code-analysis epic is **closed**. Remaining work is small and verification-oriented (see the PLAN):
- **Push the rust-survivors v4 migration** (`da11775`) to its remote — currently local-only.
- **Visual playtest** of the new/changed engine examples + rust-survivors color rendering — only compiled/unit-tested so far; VISION's "real play" bar (memory `playtest-windowed-examples`).
- **(Optional) repo-wide example Korean→English** comment conversion — only the 5 #27 examples were converted; the broader effort is tracked on the `docs/english-conversion` branch.
- Beyond these, future work is new feature initiatives (out of this chain's scope).

## Risks & Blockers

- **rust-survivors `--config` patch is rejected with a v2 pin** (semver-incompatible with local v4) — bump the `rev` to test, don't rely on the patch trick (memory `rust-survivors-engine-pin` updated).
- **rust-survivors has 21 unrelated WIP files on `main`** — be surgical; the migration is isolated in `da11775`. Don't bundle.
- **Pushing rust-survivors** is outward-facing — confirm with the user.
- **No visual verification yet** — all engine example changes are compile+unit-tested only; the lighting #15 conclusion is math-verified (CPU value + independent derivation) but not eyeballed on-screen.

## Gotchas & Lessons (reusable, cost real time)

- **rebase-merge → local main diverges.** After `gh pr merge --rebase`, origin/main has new SHAs; a local main carrying an unpushed commit can't `pull --ff-only`. Fix: `git reset --hard origin/main` (safe when the only local-unique commit is content-identical to its rebased twin — verify first).
- **`--config` patch needs a semver-compatible pin.** `cargo … --config 'patch."<git>".skeleton-engine.path="…"'` is silently rejected ("patch … not used in the crate graph") when the pinned rev is a different MAJOR than the local crate. Bump the `rev` instead.
- **`cargo check --workspace` ≠ test code.** It skips `#[cfg(test)]`; run `cargo test --workspace --no-run` (or `check --all-targets`) to catch test-module breakage (4 extra Color errors surfaced only there).
- **rustfmt reflow from deeper nesting.** Hoisting code into a new `if`/closure (deeper indent) or a long non-ASCII assert string pushes lines over width → `fmt --check` fails. Always `cargo +1.88.0 fmt` after structural edits.
- **`src/app/editor/ui/mod.rs` churn already happened** (PR #7) — editing it again was a clean +6 lines, not the feared ~700-line reflow.
- **simba/rapier `[T;N]: Into` ambiguity** — use `Color::from(arr)`/`Color::rgba(..)`, never bare `arr.into()` for colors in physics-linked crates.
- **Subagents reliable for mechanical fan-out + verify yourself.** 5 translation subagents (Sonnet) succeeded; confirmed via `grep -P '[가-힣]'` = 0, fmt/clippy/build, and a comments-only diff check.
- **Trust the compiler over rust-analyzer after big edits** — diagnostics went stale repeatedly (E0308/E0432) while the real `+1.88.0` build was clean.

## Subsystem invariants (don't re-break)

- **Lighting radius is correct (UV space).** Don't "fix" `radius_ndc` to `2*radius/viewport_w` — see the #15 decision; the contract test guards it.
- **`AudioSystem` is user-registered, not auto-added.** The engine auto-registers NO built-in systems; fades only progress if the game does `app.add_system(AudioSystem)` AND inserts an `AudioManager` resource. A missing `AudioSystem` = frozen fades (the #20 symptom), not a bug.
- **`SpatialGrid` resource type stays `SpatialGrid`** (not `Arc<…>`). `CollisionGridSystem` moves it out/in each frame; readers do `world.resource::<SpatialGrid>()`. `CollisionDebugSystem` reads that mirror but falls back to its own rebuild when standalone.
- **`layer_matches_mask` (#22):** `mask==0` = render all; layers `0..=31` map to bit n; layers outside `0..=31` (incl. negative) **never match a non-zero mask** (warn-once). So `RenderLayer(-1)` backgrounds are excluded from a `layer_mask: 1<<0` offscreen pass (the OffscreenCamera case) — do not re-introduce `clamp(0,31)`.
- **`find_path` signature is fixed** (`(&PathGrid, IVec2, IVec2)`); the A* scratch is a thread_local, not a param — keep it that way (non-breaking).
- **Script buffers are plain (no Mutex)** because the ECS/`ScriptCtx` is single-threaded; if scripting ever goes multi-threaded this must change.
- **`JointHandle.0` is `pub(crate)`** — opaque to forkers by design; only `add_*_joint` produce it.

## Verification reference

```bash
# Engine — CI-equivalent gate (ALWAYS pinned toolchain; default stable rustfmt differs → CI fails)
cargo +1.88.0 fmt --check
cargo +1.88.0 clippy --all-targets -- -D warnings
cargo +1.88.0 test --all-targets          # 311 lib + 3 + example targets, 0 fail
cargo +1.88.0 build --target wasm32-unknown-unknown   # lib+bins (NOT --all-targets; native-only examples break wasm)
RUSTDOCFLAGS="-D warnings" cargo +1.88.0 doc --no-deps
cargo +1.88.0 test --doc                  # 34 doctests
# rust-survivors (separate repo)
cd /Users/jkl/Projects/rust-survivors && cargo test --workspace   # 200 pass against engine v4
```

## Open Questions

- Push the rust-survivors migration now, or let the user bundle it with their WIP?
- Tag/release the engine `v4.0.0` (git tag), or leave version in `Cargo.toml` only? (Not published to crates.io; consumers pin by git rev.)
- Pursue the repo-wide example English-comment conversion (the `docs/english-conversion` branch), or leave it?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine

# Restore context
cat plans/handoffs/PLAN_code-analysis-fixes_v4-shipped_2026-06-07.md
cat plans/handoffs/HANDOFF_code-analysis-fixes_v4-shipped_2026-06-07.md   # this file

git log --oneline -8           # 60328fa head, v4.0.0
grep '^version' Cargo.toml      # 4.0.0

# rust-survivors state (migration is local, unpushed)
git -C /Users/jkl/Projects/rust-survivors log --oneline -2     # da11775 = the migration
git -C /Users/jkl/Projects/rust-survivors status -s            # 21 unrelated WIP files

# Verify engine starting state (PINNED toolchain — never default stable)
cargo +1.88.0 fmt --check && cargo +1.88.0 clippy --all-targets -- -D warnings && cargo +1.88.0 test --all-targets

# First action (Phase 1): push the rust-survivors migration (after confirming with user)
git -C /Users/jkl/Projects/rust-survivors push
```
