# skeleton-engine: BlendTree1D locomotion example + true 2-UV crossfade blend + 2 playtest fixes

**Date:** 2026-06-05
**Status:** COMPLETED (committed `18a6b48`, pushed `431e3ba..18a6b48`, CI in_progress at handoff time)
**Bead(s):** none (`bd` not installed)
**Epic:** VISION feature+example loop — dogfood shipped-but-never-in-a-game subsystems
**Chain:** `blendtree1d-locomotion` seq `1`
**Parent:** none — first in chain
**Prior chain:** none — first in chain

---

## Related Handoffs

This session *opened* by onboarding from `HANDOFF_lit-dungeon-lighting_example-and-engine-fixes_2026-06-05.md`
(chain `lit-dungeon-lighting` seq 1) and executed its "Where We're Going" items 2–3 (rust-survivors
impact check + next dogfooding candidate). But the BlendTree1D work is a **fresh feature stream**, so —
following that same prior session's own precedent (it onboarded from `source-split-refactor` seq 3 yet
started a new chain) — this is a new chain seq 1, with lit-dungeon listed here as a sibling, not a parent:

- `HANDOFF_lit-dungeon-lighting_example-and-engine-fixes_2026-06-05.md` — 2D lighting + PostProcess
  dogfooding (`lit_dungeon_game`); same dogfooding epic, separate work stream. Its render-order /
  nearest-16 changes were the thing this session's rust-survivors check validated.

## Since Last Handoff (lit-dungeon-lighting seq 1 → this)

- Parent's next-step **#1 (commit+push lit_dungeon)** was already done by parent (`2ab09dd`/`431e3ba`).
- Parent's next-step **#2 (check rust-survivors for render-order fallout)** → DONE this session: clean.
- Parent's next-step **#3 (pick next never-in-a-game candidate + run the loop)** → DONE this session:
  picked **BlendTree1D** (candidate **I**), shipped the full loop (grill → plan → impl → playtest → commit).
- Parent's open question "Should screen-space `DrawText` ever be post-processed?" — untouched (still open,
  not relevant to this session).
- Trajectory: still squarely on the VISION dogfooding path. One candidate closed per session.

## Reference Documents

- `docs/VISION.md` — the feature+playable-example loop (a feature isn't done until a small playable
  example exercises it; fix awkward API/bugs before release).
- `docs/NEXT_WORK.md` — candidate list; this session adds **candidate I (Blend locomotion)** and trims
  `BlendTree1D` from the "never-in-a-game" remaining list.
- `docs/HANDOFF.md` — per-phase dev history; gained a `## 2026-06-05 — BlendTree1D ...` session entry.
- `docs/CHANGELOG.md` — `## 2.0.0` Added/Fixed/Changed entries for this work.
- `CLAUDE.md` — module map row updated for `BlendUv` + the 2-UV lerp render path.
- `/Users/jkl/.claude/plans/elegant-jingling-nebula.md` — the approved plan (grill_decision_packet `elegant-jingling-nebula`).
- Memory: `rust-survivors-engine-pin.md` (written this session — how to test unpushed engine changes
  against rust-survivors).

## The Goal

Per VISION: dogfood `BlendTree1D` (1D parameter → `AnimationPlayer` clip auto-switch + crossfade) — a
subsystem that shipped with **zero game/example usage and zero unit tests** — by building ONE small
playable artifact that exercises it in real play, and fixing/upgrading whatever awkwardness or bugs it
surfaces (small fixes inline, big features deliberately chosen and documented). The end state: a focused
interactive locomotion demo where one speed parameter drives idle/walk/run blending, plus the engine
changes the example forced out.

## Where We Are

**All work complete, verified, playtest-passed 13/13, committed (`18a6b48`) and pushed.** CI was
`in_progress` on `18a6b48` at handoff (local `./scripts/verify.sh` — the CI-equivalent bar — was green,
so CI is expected green).

- **New example `examples/blend_locomotion.rs`** (`blend_locomotion`, top-level so auto-discovered — no
  `Cargo.toml` `[[example]]` needed). A `Sprite` (NOT `AtlasSprite` — see Code Analysis) + `AnimationPlayer`
  with 3 clips (idle/walk/run rows of a sheet) + a `BlendTree1D` (thresholds 0.0/0.6/1.6, crossfade 0.3s).
  `ControlSystem` maps accelerate/decelerate/sprint input → a `speed` param; `HudSystem` shows speed bar,
  current clip name, and the live `BlendWeight` bar. `Esc` quits.
- **New generator `examples/gen_blend_sheet.rs`** (`gen_blend_sheet`, manual/deterministic, mirrors
  `gen_platform_tiles.rs`) → writes the committed `examples/assets/blend_locomotion.png` (256×192, 4167 B):
  a 4×3 grid (64px cells), rows = idle(blue)/walk(green)/run(orange), columns = frames with a vertical bob
  (amplitude 2/6/12 px) + leg stride. Strong per-row hue contrast makes the cross-dissolve legible.
- **3 engine changes + 2 playtest-surfaced fixes** (the dogfooding payoff):
  1. **fix `BlendTreeSystem` stranding** (`src/animation/blend_system.rs`) — was recording `last_clip = target`
     even when it *skipped* the transition under the `is_crossfading()` guard. Now it `continue`s (defers)
     without recording when mid-crossfade, and re-evaluates after. +2 regression tests (later +1 = 3 total).
  2. **feat true 2-UV shader-lerp crossfade** — `InstanceRaw` 96B→116B (`to_uv_offset`/`to_uv_size`/`blend`,
     shader_locations 9/10/11) with `single()`/`blended()` constructors; `sprite.wgsl` `mix`es from/to frames
     (single-sample branch on `blend > 0`); new `BlendUv { to, weight }` component written by `AnimationSystem`
     and read by the sprite renderer; `BlendWeight` is finally consumed. Replaced the old 50% UV hard-swap.
  3. **`BlendUv` re-exported** (`engine::BlendUv`) and consumed in the sprite renderer's `Sprite` query loop
     (not AtlasSprite/UI — out of scope; those default the new fields to `weight=0` = identical render).
  4. **(playtest) example centering** — character was spawned at world `(0,0)` but the camera is top-left
     anchored, so it clipped into the screen corner. Fixed to spawn at screen-center `(W/2, H/2)`. Example-only.
  5. **(playtest) fix IME swallowing game keys** — `window.rs` unconditionally `set_ime_allowed(true)`, so a
     CJK (Korean) IME swallowed key-release events → keys stuck "pressed". Added `ImeConfig { allowed: bool }`
     resource (default **off**); window reads it; `settings_menu` opts in. Games now get raw, IME-robust keys.
- **Verification:** `./scripts/verify.sh` green (fmt, clippy `-D warnings`, wasm lib+bins build,
  `test --all-targets`, rustdoc `-D warnings`) after each change; lib tests pass incl. 3 new blend tests;
  `rust-survivors cargo check` against this tip clean; native `cargo run --example blend_locomotion` rendered
  with no shader-validation panic; user playtest **13/13** via an HTML checklist.
- **Demo controls:** hold `→`/`D`/`Space` accelerate · release decelerate · `ShiftLeft`/`ShiftRight` instant
  sprint (param→MAX) · `Esc` quit. `ControlSystem` runs before `BlendTreeSystem` before `AnimationSystem`
  before `HudSystem` (the order matters: input sets param → tree picks clip → animation advances/blends → HUD).
- **Subsystem state before this session:** `BlendTree1D`/`BlendEntry`/`BlendTreeSystem` existed and were
  re-exported from `engine::` but had **no game/example usage and no tests**; `BlendWeight` was emitted by
  `AnimationSystem` every frame but read by nothing; the crossfade was a hard 50% UV swap. All three are now
  exercised/fixed/upgraded.
- **`bd` (beads) is unavailable** in this environment (as in the prior sessions) — chains are tracked purely by
  the `HANDOFF_*`/`PLAN_*` filenames + headers in `plans/handoffs/`.

## What We Tried (Chronological)

1. **(early) Onboarded** from lit-dungeon-lighting seq 1 (`bd` unavailable, per the paste prompt). Confirmed
   clean/green start: `git status` clean, `git log` shows `431e3ba`/`2ab09dd` pushed, **CI on `431e3ba` green**
   (4m59s), `./scripts/verify.sh` exit 0.
2. **(early) rust-survivors impact check** (lit-dungeon next-step #2). Discovered rust-survivors pins the
   engine by **git rev `61c09f1`** in `crates/game/Cargo.toml` — **8 commits behind HEAD**, predating the
   lighting work — so a plain `cargo build` there silently tests the OLD engine. rust-survivors uses **0**
   lighting/post (`PointLight`/`AmbientLight`/`PostProcess` count = 0), only `TextQueue`/`DrawText` HUD (62
   hits), so the render-order change is visually inert there (`render_view == final_view` when lit/post off).
   Tested compile against local HEAD via a non-invasive **`--config` path patch** → clean (`skeleton-engine
   v2.0.0`, 5.22s). The patch dirtied only `Cargo.lock` (dropped the `source = git+...` line) → restored with
   `git checkout -- Cargo.lock`. Saved this as memory `rust-survivors-engine-pin.md`.
3. **(early-mid) BlendTree1D candidate** chosen by user ("blendtree1D 작업 어때?"). Read `blend_tree.rs`,
   `blend_system.rs`, `player.rs`, `system.rs`. Confirmed: used only in `lib.rs` re-export, **0 tests**.
   **Found the stranding bug by reading** (`blend_system.rs:33-48`): traced that `last_clip` is set to the
   target even when the transition is skipped under `is_crossfading()`, stranding the clip when the param
   crosses two thresholds during one crossfade. Also noted: crossfade is a 50% UV hard-swap (`system.rs:71-86`),
   and `BlendWeight` is updated but **consumed nowhere** (a dead output).
4. **(mid) `/grill-me`** locked scope across 3 rounds (packet `elegant-jingling-nebula`, plan_allowed: true).
   Notable: the user did **not** pick all "Recommended" options this time — they chose the *bigger* path:
   true 2-UV shader lerp (over my recommended "document the hard-swap as a limit"), a focused interactive demo
   (over a win/lose game), and a procedural spritesheet. Round 2 locked impl = 2-UV shader lerp (not
   dual-instance over-composite) + additive & wasm-functional. Round 3 confirmed all 8 brief items.
5. **(mid) Plan mode.** Researched the render path: `InstanceRaw` 96B (model/color+alpha/uv_offset/uv_size),
   `InstanceRaw::layout()` used by both sprite pipelines (`sprite.rs:135`, `sprite/material.rs:34`); the UI
   path reuses `self.pipeline`; custom `ShaderMaterial` frags are *separate* modules consuming only
   `@location(0/1)`, so appending vertex outputs/instance attributes is backward-compatible; `mix()` doesn't
   depend on the framebuffer blend mode. Found **no in-memory image API** (`App::load_image`/`load_texture`
   are disk-only), so the procedural sheet uses the `gen_platform_tiles.rs` pattern (gen example + committed
   PNG). AskUserQuestion in plan confirmed gen-example-+-PNG over adding a runtime image API. Wrote
   `/Users/jkl/.claude/plans/elegant-jingling-nebula.md`; ExitPlanMode → approved.
6. **(mid-late) Implementation** in 5 tracked tasks:
   - Stranding fix + tests → `cargo test --lib blend` green (2 tests).
   - `InstanceRaw` extension (loc 9/10/11) + `single()`/`blended()` ctors; updated AtlasSprite ×2,
     `ui_quad_instance`, `sprite/tests.rs raw()`. `sprite.wgsl` Instance/VertexOutput/vs_main/fs_main.
     `BlendUv` component + re-exports. `AnimationSystem` writes `BlendUv` and drops the 50% swap. Sprite
     renderer reads `BlendUv` via a `make_instance` closure. `cargo clippy --lib` clean; 273 lib tests pass.
   - `gen_blend_sheet.rs` + `blend_locomotion.rs`; generated the PNG, viewed it (correct); native run = no
     shader panic. clippy fix: `col % 2 == 0` → `col.is_multiple_of(2)`.
   - Docs (CHANGELOG/NEXT_WORK/HANDOFF/CLAUDE.md).
   - `verify.sh` (needed `cargo fmt` twice — the AtlasSprite/import edits re-wrapped); rust-survivors check
     clean; PNG regen is byte-stable.
7. **(late) Playtest round 1** via an HTML checklist (`/tmp/blend_locomotion_test.html`, 13 items, localStorage,
   markdown export) + launched the demo. **First screenshot: character clipped into the top-left corner.**
   Root-caused to the **top-left anchored camera** (`camera.rs:7` — `camera.position` = viewport top-left);
   a sprite centered at world `(0,0)` lands its center at the screen corner. Fixed the example to spawn at
   `(W/2, H/2) = (400, 300)`; `screencapture` confirmed it centered.
8. **(late) Playtest report round 1 (after centering): 4✅ / 1❌.** **B3 failed** (release → clip doesn't
   return run→walk→idle). User separately reported **"한글 입력시 문제 생김"** (problem with Korean input).
   **Diagnosed by instrumentation, not theory** (lit-dungeon lesson): wrote a headless accel→hold→decel
   regression test — it **PASSED**, proving the blend logic returns to idle correctly. So B3 is an
   **input-layer** issue → traced to `window.rs:378` `set_ime_allowed(true)` (forced on). With a CJK IME the
   OS routes key-release into IME composition → keys stick "pressed" → accelerate key never releases →
   speed never decreases → clip stuck on run. Same root cause as the Korean-input note.
9. **(late) IME fix** — user chose **Option A** (engine fix: IME default OFF, opt-in). Added
   `ImeConfig { allowed: bool }` (default off) in `resources.rs`; `window.rs` reads it (`unwrap_or(false)`);
   `settings_menu` (the only `TextInput` user — verified by grep) opts in with `ImeConfig { allowed: true }`.
   Added the decel regression test as a keeper. `verify.sh` green (fmt fix), rust-survivors check clean again.
10. **(late) Playtest round 2: 13/13 ✅.** Full sign-off → committed `18a6b48` (single commit, 20 files,
    +764/-86) and pushed `431e3ba..18a6b48`. CI started (`26997904904`, in_progress).

## Key Decisions

- **Chain = new (`blendtree1d-locomotion` seq 1), not a continuation of lit-dungeon-lighting.** Onboarded from
  it, but BlendTree1D is a fresh feature stream — mirrors the prior session's own precedent.
- **Blend implementation = true 2-UV shader lerp** (user-chosen, over the recommended "document the hard-swap
  limit"). Per-pixel-correct cross-dissolve; `mix()` is independent of framebuffer blend state. Chosen over
  dual-instance over-composite (which would depend on alpha-over compositing and draw two quads).
- **Additive + wasm-functional posture.** `weight = 0` renders byte-identically to before (shader branches so
  the common path stays a single sample); the sprite path is cross-platform so blend works on wasm (unlike
  lighting). Custom `ShaderMaterial` frags unaffected (they read only `@location(0/1)`).
- **Scope boundary:** blend applies only to the `AnimationPlayer` crossfade path (Sprite loop). `AtlasSprite`/
  UI/tilemap just default the new instance fields. Single texture per clipset assumed. Non-goals: win/lose
  loop, cross-texture clip blending, AtlasSprite/UI blending.
- **Asset = gen example + committed PNG** (over adding a runtime in-memory image API), keeping the engine change
  scoped to the blend feature. Matches the `gen_platform_tiles.rs` precedent.
- **Demo uses `Sprite` + `AnimationPlayer`, NOT `AtlasSprite`** — critical, because `BlendUv` is read only in
  the Sprite render loop. `AnimationSystem` writes `UvRect` (+ `BlendUv`) on the entity; the plain Sprite loop
  reads both. (The platformer uses `AtlasSprite`, which would NOT blend.)
- **Centering is an example bug, not an engine bug.** The top-left camera anchor is a documented convention
  (`camera.rs:7-16`); the example just placed the player wrong.
- **IME fix = engine, Option A (default OFF, opt-in).** IME-on is the unusual case (only text-input apps need
  it); IME-off is the correct default so CJK input sources don't swallow game keys. settings_menu opts in.
  Chosen over opt-out (which would leave every other game broken under a CJK IME) and over doc-only.
- **Diagnose by instrumentation/tests, not theory** (carried lit-dungeon lesson). The decel test was the
  decisive evidence that B3 was input-layer, not blend-logic.

## Evidence & Data

### Commit (this session)

| Hash | Summary | Files | +/- |
| --- | --- | --- | --- |
| `18a6b48` | feat(animation): BlendTree1D locomotion example + true 2-UV crossfade blend | 20 | +764 / -86 |

### Diffstat (`git show --stat 18a6b48`)

```
CLAUDE.md                                     |   2 +-
docs/CHANGELOG.md                             |  23 +++
docs/HANDOFF.md                               |  67 ++++++++
docs/NEXT_WORK.md                             |  24 ++-
examples/assets/blend_locomotion.png          | Bin 0 -> 4167 bytes
examples/blend_locomotion.rs                  | 239 ++++++++++++++++++++++++++
examples/games/settings_menu/settings_menu.rs |  11 +-
examples/gen_blend_sheet.rs                   | 104 +++++++++++
src/animation/blend_system.rs                 | 149 +++++++++++++++-
src/animation/mod.rs                          |   2 +-
src/animation/player.rs                       |  14 ++
src/animation/system.rs                       |  42 +++--
src/app/window.rs                             |  14 +-
src/lib.rs                                    |   6 +-
src/renderer/shaders/sprite.wgsl              |  29 +++-
src/renderer/sprite.rs                        |  28 ++-
src/renderer/sprite/geometry.rs               |  70 ++++++--
src/renderer/sprite/tests.rs                  |   7 +-
src/renderer/sprite/ui_primitives.rs          |   7 +-
src/resources.rs                              |  12 ++
20 files changed, 764 insertions(+), 86 deletions(-)
```

### Verification (final, all green)

| Check | Result |
| --- | --- |
| `cargo fmt --check` | clean (after 2 `cargo fmt` passes during dev) |
| `cargo clippy --all-targets -- -D warnings` | 0 warnings |
| `cargo build --target wasm32-unknown-unknown` (lib+bins) | clean |
| `cargo test --all-targets` | pass (incl. 3 new `blend_system` tests) |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps` | clean |
| native `cargo run --example blend_locomotion` | renders, no shader panic |
| rust-survivors `cargo check --workspace` (path-patched to tip) | clean (`skeleton-engine v2.0.0`) |
| user playtest | **13/13** checklist ✅ |

### New unit tests (`src/animation/blend_system.rs`)

| Test | Asserts |
| --- | --- |
| `target_clip_picks_highest_threshold_at_or_below_param` | param −1→clip0, 0.4→0, 0.5→1, 2.0→2 |
| `fast_param_jump_during_crossfade_reaches_final_clip` | idle→walk crossfade interrupted by param→run reaches run (was stranded on walk pre-fix) |
| `decelerating_param_returns_through_clips_to_idle` | accel to run then decel returns to idle (proves B3 is NOT a blend bug) |

### InstanceRaw layout change (`src/renderer/sprite/geometry.rs`)

```
before: [model 64B][color 16B][uv_offset 8B][uv_size 8B]                                  = 96B
after:  [model 64B][color 16B][uv_offset 8B][uv_size 8B][to_uv_offset 8B][to_uv_size 8B][blend 4B] = 116B
new vertex attrs: loc 9 (offset 96, Float32x2), loc 10 (offset 104, Float32x2), loc 11 (offset 112, Float32)
```

### Demo tuning constants (`examples/blend_locomotion.rs`)

```
WINDOW 800×600 · player Transform scale 280 · spawn (400,300) (= W/2,H/2)
MAX_SPEED 2.4 · ACCEL 4.5 · DECEL 3.5 · WALK_THRESHOLD 0.6 · RUN_THRESHOLD 1.6 · CROSSFADE 0.3
clip fps: idle 4.0 · walk 8.0 · run 14.0 (all looping, 4 frames via UvRect::from_grid(col,row,4,3))
```
ACCEL 4.5 is deliberately brisk so a hold crosses 0.6→1.6 (≈0.22s) inside the 0.3s crossfade → exercises the stranding fix in real play.

### Spritesheet generator (`examples/gen_blend_sheet.rs`)

```
4 cols × 3 rows, CELL 64 → 256×192, 4167 bytes (deterministic; regen is byte-stable)
HUES idle [70,120,225] (blue) · walk [80,195,95] (green) · run [235,135,55] (orange)
BOB amplitude [2,6,12] px · stride apart on even frames (is_multiple_of(2))
```

### rust-survivors cross-repo check (memory: `rust-survivors-engine-pin.md`)

```bash
cd /Users/jkl/Projects/rust-survivors
cargo check --workspace \
  --config 'patch."https://github.com/ChunSam/skeleton-engine".skeleton-engine.path="/Users/jkl/Projects/skeleton-engine"'
git checkout -- Cargo.lock   # the patch drops the git source line; restore it
```
rust-survivors pins engine at rev `61c09f1` (8 commits behind HEAD); uses 0 lighting/post, only `TextQueue`/`DrawText` (62 hits in `crates/game/src/survivor/hud.rs`).

### Grill scope-lock (decision packet `elegant-jingling-nebula`, plan_allowed: true)

Round-by-round choices (the user picked the more ambitious option, not "Recommended"):

| Round | Question | Chosen |
| --- | --- | --- |
| 1 | playable framing | **focused interactive demo** (not a win/lose game loop) |
| 1 | crossfade quality | **add engine true alpha blend** (over recommended "document the 50% hard-swap limit") |
| 1 | sprite asset | **procedurally generated sheet** |
| 2 | blend implementation | **2-UV shader lerp** (over dual-instance over-composite) |
| 2 | compat/target posture | **additive + wasm-functional** |
| plan | asset mechanism | **gen example + committed PNG** (over a new runtime in-memory image API) |

8 locked brief items: (1) 2-UV shader lerp, InstanceRaw +to_uv/blend, shader branches on weight>0;
(2) AnimationSystem emits BlendUv{to,weight}, replaces 50% swap; (3) scope = AnimationPlayer crossfade
path only, single texture per clipset; (4) stranding fix inline + regression test; (5) focused demo in
`examples/`, procedural sheet, keyboard accel/decel; (6) engine-fix bar = VISION default; (7) proof bar =
verify.sh + new tests + native run + rust-survivors check + user sign-off; (8) additive, public API/behavior
preserved at weight=0.

### Playtest iteration history (HTML checklist, 13 items)

| Round | Result | Issue surfaced → fix |
| --- | --- | --- |
| screenshot 1 | — | character clipped into top-left corner → spawn at `(W/2,H/2)` (example) |
| report 1 (post-centering) | 4✅ / 1❌ | **B3** clip doesn't return on decel + "한글 입력시 문제" → IME swallows keyup → `ImeConfig` default off (engine) |
| report 2 (final) | **13✅ / 0❌** | full sign-off → commit `18a6b48` |

HTML checklist (`/tmp/blend_locomotion_test.html`, localStorage + markdown export) — 13 items, 5 groups:
A (boot/render: A1 centered+no panic, A2 HUD), B (param→clip: B1 idle at rest, B2 accel idle→walk→run,
**B3 decel run→walk→idle**), C (true crossfade: C1 color dissolve not pop, C2 blend bar 0→1 sweep, C3
two-frame interpolation), D (stranding: D1 fast accel reaches run, D2 Shift sprint reaches run), E
(controls: E1 Right/D/Space accel, E2 Esc quits, E3 no flicker).

## Code Analysis

- **`BlendTreeSystem::run`** (`blend_system.rs`): per entity, computes `target_clip()` from `param`; an early
  dedup `if already_requested == Some(clip_index) { continue; }` (last_clip == target). The fix restructured
  the player block: `if current_clip == clip_index { /* record */ } else if is_crossfading() { continue; /* do
  NOT record last_clip */ } else { play_with_crossfade(...); }` then record `last_clip`. Deferring (not
  recording) when mid-crossfade is the whole fix.
- **`target_clip()`** (`blend_tree.rs`): returns the entry with the highest threshold ≤ param (iterates, keeps
  last match); clamps to entries[0] when param is below all thresholds. Order-correct only if entries are
  threshold-ascending (documented requirement; not enforced).
- **`AnimationSystem::run`** (`system.rs`): advances both from- and to-frames during a crossfade; now always
  outputs the **from**-frame as `UvRect` and writes `BlendUv { to: to-frame uv, weight }` (weight 0 when not
  crossfading, with `to = uv`). Removed the old `weight >= 0.5` hard-swap. Still updates `BlendWeight`.
- **Sprite render path** (`sprite.rs`): the `Sprite` query loop reads optional `BlendUv` and builds the
  instance via a `make_instance(model)` closure → `InstanceRaw::blended(...)` when `weight > 0`, else
  `single(...)`. `InstanceRaw::from`/`from_global` (now `single`-based) remain used by the `ShaderMaterial`
  path. AtlasSprite inline structs + `ui_quad_instance` route through `single()`.
- **`sprite.wgsl`**: `VertexOutput` gained `@location(2) uv_to`, `@location(3) blend`; `fs_main` samples `uv`,
  and only when `blend > 0` samples `uv_to` and `mix`es — keeping the common path a single texture sample.
- **`ImeConfig`** (`resources.rs`): `#[derive(Debug, Clone, Copy, Default)]`, `allowed: bool` (default false).
  `window.rs` reads `self.world.resource::<crate::resources::ImeConfig>().map(|c| c.allowed).unwrap_or(false)`
  once at window setup and calls `window.set_ime_allowed(ime_allowed)`.
- **Camera convention** (`camera.rs:7-16`): top-left anchored — `position` = viewport top-left world coord;
  visible X∈[pos.x, pos.x+w/zoom], Y∈[pos.y, pos.y+h/zoom] (y-down). To center an entity:
  `camera.position = entity_pos - viewport/(2*zoom)` (or place the entity at screen-center world coords).

### Before → after (the two highest-value edits)

**Stranding fix** (`blend_system.rs`) — the bug was that `last_clip` was recorded unconditionally:

```rust
// BEFORE (buggy): records last_clip even when the transition is skipped
if player.current_clip != clip_index && !player.is_crossfading() {
    player.play_with_crossfade(clip_index, crossfade_dur);
} else if player.current_clip == clip_index { /* already playing */ }
// ... then unconditionally:
tree.last_clip = Some(clip_index);   // <-- BUG: target recorded even though it never played

// AFTER (fixed): defer (continue) when mid-crossfade, do NOT record last_clip
if player.current_clip == clip_index {
    // already on target — fall through to record
} else if player.is_crossfading() {
    continue;                        // defer; re-evaluate next frame (last_clip unchanged)
} else {
    player.play_with_crossfade(clip_index, crossfade_dur);
}
tree.last_clip = Some(clip_index);   // only reached when we acted or are already on target
```

**Crossfade output** (`system.rs`) — replaced the midpoint UV swap with from-frame + BlendUv:

```rust
// BEFORE: snap to to-frame at 50%
let uv = if let Some(cf) = &player.crossfade {
    if weight >= 0.5 { /* to-frame uv */ } else { player.current_uv() }
} else { player.current_uv() };
// AFTER: always from-frame; carry to-frame + weight for the shader to mix
let uv = player.current_uv();
let blend_uv = if let Some(cf) = &player.crossfade {
    BlendUv { to: /* to-frame uv */, weight }
} else { BlendUv { to: uv, weight: 0.0 } };
// writes UvRect(uv) + BlendWeight(weight) + BlendUv(blend_uv)
```

**Shader fragment** (`sprite.wgsl`) — branch keeps the common path single-sample:

```wgsl
var sampled = textureSample(t_sprite, s_sprite, in.uv);
if (in.blend > 0.0) {
    sampled = mix(sampled, textureSample(t_sprite, s_sprite, in.uv_to), in.blend);
}
return sampled * in.color;
```

**IME fix** (`window.rs`) — `set_ime_allowed(true)` → resource-driven (default off):

```rust
let ime_allowed = self.world
    .resource::<crate::resources::ImeConfig>()
    .map(|c| c.allowed)
    .unwrap_or(false);
window.set_ime_allowed(ime_allowed);
```

## Files Changed

### Source — animation
- `src/animation/blend_system.rs` — stranding fix (defer when mid-crossfade) + 3 `#[cfg(test)]` tests.
- `src/animation/player.rs` — new `pub struct BlendUv { pub to: UvRect, pub weight: f32 }`.
- `src/animation/system.rs` — write `BlendUv`, drop the 50% swap, always output the from-frame.
- `src/animation/mod.rs` — re-export `BlendUv`.

### Source — renderer
- `src/renderer/sprite/geometry.rs` — `InstanceRaw` +to_uv/blend (loc 9/10/11), `single()`/`blended()` ctors,
  `from`/`from_global` via `single`, layout() attrs, doc comment.
- `src/renderer/sprite.rs` — import `BlendUv`; Sprite loop reads `BlendUv` and builds via `make_instance` closure.
- `src/renderer/sprite/ui_primitives.rs` — `ui_quad_instance` → `InstanceRaw::single`.
- `src/renderer/sprite/tests.rs` — `raw()` helper → `InstanceRaw::single`.
- `src/renderer/shaders/sprite.wgsl` — Instance/VertexOutput +uv_to/blend; vs_main/fs_main mix.

### Source — input/window/resources
- `src/app/window.rs` — read `ImeConfig` instead of forcing `set_ime_allowed(true)`.
- `src/resources.rs` — new `ImeConfig { allowed: bool }` (default off).
- `src/lib.rs` — re-export `BlendUv` and `ImeConfig`.

### Examples
- `examples/blend_locomotion.rs` — the playable acceptance test (239 lines).
- `examples/gen_blend_sheet.rs` — deterministic spritesheet generator (104 lines).
- `examples/assets/blend_locomotion.png` — committed generated sheet (256×192, 4167 B).
- `examples/games/settings_menu/settings_menu.rs` — opt into IME (`ImeConfig { allowed: true }`) + import.

### Docs
- `docs/CHANGELOG.md` — `## 2.0.0` Added (example, BlendUv, ImeConfig), Fixed (stranding, IME, ...), Changed (2-UV lerp).
- `docs/NEXT_WORK.md` — candidate **I**; trimmed `BlendTree1D` from never-in-a-game list.
- `docs/HANDOFF.md` — `## 2026-06-05 — BlendTree1D ...` session entry + playtest follow-ups.
- `CLAUDE.md` — module-map row for `BlendUv` + 2-UV lerp render path.

## User Feedback & Preferences (REQUIRED — never omit)

- **Works in Korean; wants conversational replies in Korean.** Handoff/docs stay English per the doc-language rule.
- **Did NOT default to "Recommended" this session** — chose the more ambitious options in `/grill-me`: true
  2-UV shader lerp (the bigger engine change) over the recommended documented-limit, focused demo over a game
  loop, procedural generation over asset reuse. (Contrast lit-dungeon, where they took every Recommended.)
  Calibration: present bounded trade-offs, but don't assume they'll always take the conservative one.
- **Asked for a runnable HTML test checklist**: "테스트 사항 정리해서 html로 실행해 줘". Built
  `/tmp/blend_locomotion_test.html` (13 items, localStorage-persisted, markdown export). They ran it and pasted
  a precise per-item ✅/❌ report twice. Keep delivering runnable artifacts + screenshots.
- **Playtests precisely and finds real bugs**: reported "화면이 왼쪽 위 구석에 몰려있음" with a screenshot
  (centering), and "한글 입력시 문제 생김" (IME) — both genuine, both fixed. This precision is what pinned the
  bugs.
- **Chose the engine fix for IME** (Option A, default off) — consistent with lit-dungeon (they chose the engine
  HUD-after-lighting fix over a workaround). They prefer fixing the engine over papering over it in the example.
- **Direct-to-main commit + push** is the established norm; sign-off came via the 13/13 report (I had said
  "전부 ✅면 단일 커밋으로 커밋·푸시하겠습니다" and they delivered all-green).
- Memory of record (from prior sessions): aggressive subagent use for parallel work on Sonnet — **not used this
  session** (single-file/sequential diagnosis + plan-mode reads handled directly; account guidance also
  discourages spawning agents unless asked).

## Where We're Going

1. **Confirm CI green on `18a6b48`** (`gh run list --branch main`). Low risk — local `verify.sh` (CI-equivalent)
   was green. The only thing CI can't cover is the windowed render/shader — already validated by the native run.
2. **Pick the next never-in-a-game dogfooding candidate** from `docs/NEXT_WORK.md` (none scheduled):
   `Timeline`/cutscene, physics joints, `RenderTarget`/`OffscreenCamera` in real play, networking. Run the same
   loop: `/grill-me` to lock scope → plan → implement engine + a small playable example → playtest (HTML
   checklist + screenshots) → verify.sh + rust-survivors check → commit/push. **See the paired PLAN file** for
   the recommended candidate and phasing.
3. **Optional polish for `blend_locomotion`** (only if revisited): the speed/blend HUD bars are text-only;
   `BlendWeight` only sweeps visibly during the 0.3s crossfade; the demo has no camera movement (static center).

## Risks & Blockers

- **Low.** verify.sh green; native 13/13; rust-survivors clean. The render-path change (InstanceRaw format +
  sprite shader) is the highest-blast-radius edit this session, but it's additive — `weight=0` renders
  byte-identically and all instance builders go through `single()`/`blended()`.
- **CI in_progress at handoff** on `18a6b48` — expected green, but confirm. If it fails, it'll be a
  fmt/clippy/wasm/test/rustdoc issue (all passed locally) — investigate the specific job.
- **IME default-off is a behavior change.** Any consumer needing IME text input must now insert
  `ImeConfig { allowed: true }`. Verified only `settings_menu` uses `TextInput`; rust-survivors does not. A
  future text-input example/game must remember to opt in.
- CI cannot run the windowed app, so future blend/render/shader changes still need a **native run** to validate
  (this is why these bugs were latent). Keep using the example as the runtime gate.

## Open Questions

- **CI result on `18a6b48`** — pending confirmation (expected green).
- Carried from lit-dungeon (still open, not this session's concern): should screen-space `DrawText` ever be
  post-processed? FPS/perf HUD via `ProfilerData`?

## Quick Start for Next Session

```bash
# Restore context
cat plans/handoffs/HANDOFF_blendtree1d-locomotion_crossfade-blend-ime_2026-06-05.md
cat plans/handoffs/PLAN_blendtree1d-locomotion_crossfade-blend-ime_2026-06-05.md   # the paired plan

# Confirm state
git log --oneline -3            # expect 18a6b48 at tip
git status -s                   # expect clean
gh run list --branch main --limit 3   # confirm CI green on 18a6b48
./scripts/verify.sh             # 5 checks, expect exit 0

# Key files for the next candidate selection
sed -n '120,146p' docs/NEXT_WORK.md      # remaining never-in-a-game candidates
cat docs/VISION.md                        # the feature+example loop

# This session's deliverables (reference patterns)
sed -n '1,30p' examples/blend_locomotion.rs              # Sprite+AnimationPlayer+BlendTree1D demo pattern
grep -n "BlendUv\|InstanceRaw::\|make_instance" src/renderer/sprite.rs src/animation/system.rs
grep -n "ImeConfig" src/resources.rs src/app/window.rs   # IME opt-in pattern

# Next action: pick the next dogfooding candidate, run /grill-me to lock scope, then plan → implement.
```
