# skeleton-engine: lit_dungeon lighting/post dogfooding example + 4 engine fixes

**Date:** 2026-06-05
**Status:** COMPLETE (uncommitted at handoff time; committing on session close per user request)
**Bead(s):** none (`bd` not installed)
**Epic:** VISION feature+example loop — dogfood shipped-but-uncovered subsystems
**Chain:** `lit-dungeon-lighting` seq `1`
**Parent:** none (new chain). The session *opened* on `source-split-refactor` seq 3 but that
epic was closed+archived here; this feature work is a fresh stream.

---

## Since Last Handoff

This session began by onboarding from `HANDOFF_source-split-refactor_verify-bar-and-splits_2026-06-04.md`
(seq 3) and continuing its "Where We're Going". Two distinct work streams resulted:

1. **Closed out the source-split-refactor chain** (its remaining optional items):
   - Resolved the carried-over open question: **`renderer::FrameContext` stays public** —
     recorded in `docs/HANDOFF.md` → "Architecture decisions worth knowing".
   - Archived the 3 chain handoffs + 1 PLAN to `plans/handoffs/archive/`.
   - Committed + pushed: `acd168e` (CI: docs-only, green).
2. **Started this new chain** — a playable example dogfooding 2D lighting + PostProcess
   (the densest never-in-a-game subsystem cluster per `docs/NEXT_WORK.md`). Scope was locked
   via `/grill-me` (5 question rounds), planned, approved, implemented, and **playtested by the
   user across two rounds**, which surfaced and fixed three additional latent engine bugs.

The source-split-refactor handoffs now live in `plans/handoffs/archive/`; this is the first
handoff of the `lit-dungeon-lighting` chain.

## Reference Documents

- `docs/VISION.md` — the feature+example loop this work serves (a feature isn't done until a
  small playable example exercises it in real play; fix awkward API/bugs before release).
- `docs/NEXT_WORK.md` — candidate list; this session adds **candidate H (Lit dungeon)** and
  trims 2D lighting + PostProcess from the "never-in-a-game" remaining list.
- `docs/HANDOFF.md` — per-phase dev history; gained a `## 2026-06-05` session entry + the
  FrameContext architecture-decision note.
- `docs/CHANGELOG.md` — `## 2.0.0` Added/Fixed/Changed entries for this work.
- `CLAUDE.md` — module map gained a `src/renderer/lighting.rs` row.
- `/Users/jkl/.claude/plans/playful-finding-rivest.md` — the approved plan (grill_decision_packet).

## The Goal

Per VISION: add ONE small playable example that exercises 2D lighting (`PointLight`/`AmbientLight`)
and `PostProcessConfig` in real play, and fix whatever awkwardness/bugs it surfaces (small fixes
inline; big features documented). Hard constraint: public API unchanged; engine fixes additive.

## Where We Are

**All work complete, verified, and playtest-passed (18/18). Uncommitted in the working tree on
`main` at handoff; will commit + push on close (user asked: "하고 커밋 푸쉬").**

- **New example `examples/games/lit_dungeon/lit_dungeon.rs`** (`lit_dungeon_game`), registered in
  `Cargo.toml`. A dark top-down brazier-lighting puzzle:
  - WASD/arrows move; per-axis wall slide via `SpatialGrid` (`CollisionGridSystem` + colliders on
    wall sprites) — reused from the maze_escape pattern.
  - Player carries a torch `PointLight` whose radius/intensity **decay over time** (fuel,
    `MAX_FUEL = 13s`); `torch_from_fuel(fuel)` maps fuel→(radius 36..240, intensity 0..1.7).
  - 16 braziers; press **E** within `INTERACT_RADIUS` (TILE*0.95) to light the nearest unlit one
    → spawns a persistent `PointLight` on it + bright sprite + refills fuel to max.
  - Light all 16 → exit `PointLight` turns on (green) → reach exit → **Won**. Fuel hits 0 →
    **Lost**. **R** restart, **Esc** quit, **P** toggles `PostProcessConfig.enabled`.
  - `AmbientLight {color:[0.55,0.6,0.95], intensity:0.08}` (dark blue, enables the lighting pass).
  - `PostProcessConfig` preset: bloom (torch/brazier glow) + vignette (tunnel-vision) + slight
    contrast/saturation.
  - Camera follows the player across a 960×768 level (15×12 tiles @ TILE=64), viewport 800×600,
    clamped to level bounds.
  - HUD via `TextQueue`/`DrawText`: torch fuel bar+%, braziers X/16, **FPS readout** (EMA of
    1/dt), controls line, win/lose messages.
  - Peak simultaneous lights = torch + 16 braziers + exit = **18 (> the 16 hard cap)** by design,
    so the nearest-16 cull is exercised in real play.

- **4 engine changes** (all surfaced by this example — the dogfooding payoff):
  1. **`src/renderer/lighting.rs` — nearest-16 light culling** (planned). `LightingRenderer::update`
     previously filled the 16-slot GPU array with the first 16 lights in arbitrary query order;
     now `select_nearest_lights(positions, camera_pos)` keeps the nearest 16 (via
     `select_nth_unstable_by` on squared distance) and a `std::sync::Once` warns once when over
     the cap. `MAX_LIGHTS = 16` const. +2 unit tests (`select_nearest_lights_*`).
  2. **`src/renderer/shaders/post_process.wgsl` — fix naga validation crash**. The bloom 4-tap
     loop indexed a `let tap_offsets = array<...>` by the loop variable; naga rejects dynamic
     indexing of a `let` array ("may only be indexed by a constant") → panicked on
     `Device::create_shader_module` whenever `PostProcessConfig.enabled` was true. Changed
     `let`→`var` (an addressable array can be dynamically indexed).
  3. **`src/app/render.rs` — fix HiDPI lighting offset**. The lighting pass was called with the
     **physical** surface size (`gpu.config.width/height`) while the sprite pass uses the
     **logical** viewport (`logical_w/logical_h`, render.rs:392). `light_position_ndc` therefore
     scaled positions+radii by a 2× wrong factor on Retina (scale 2): lights drifted off their
     sprites and rendered at half radius. Fixed to pass `logical_w/logical_h` to `lr.update`.
  4. **`src/app/render.rs` — fix HUD darkened by lighting**. `TextQueue`/`DrawText` rendered into
     the scene texture *before* the post+lighting passes, so screen-space HUD was multiplied down
     to near-black in the dark dungeon. Moved the text pass to run **after** post+lighting onto
     `final_view` (below the fade overlay and egui). Trade-off (accepted by user): `DrawText` is
     no longer affected by `PostProcessConfig` — route text through egui for post-processed text.

- **Verification:** `./scripts/verify.sh` (fmt, clippy `-D warnings`, wasm lib+bins build,
  `test --all-targets`, rustdoc `-D warnings`) → exit 0, run after every change. Native
  `cargo run --example lit_dungeon_game` confirmed by the user: **18/18 checklist items pass**,
  including the nearest-16 cull and the stderr cap-warning.

## What We Tried (Chronological)

1. **Onboarded** from source-split-refactor seq 3; confirmed clean/green start; verified
   `FrameContext` is a `pub use` in `renderer/mod.rs` and the param type of the public
   `SpriteRenderer::render` → decided to **keep it public** (tightening would make `render`
   uncallable externally). Recorded the decision; archived the chain; committed `acd168e`.
2. **`/grill-me`** — 5 rounds locking the example scope (see Key Decisions). Wrote the plan to
   `~/.claude/plans/playful-finding-rivest.md`, ExitPlanMode → approved.
3. **Implemented** the engine cull + example + Cargo registration + docs. Iterated on small
   compile/clippy errors (`String + &String` → `format!`; `&format!` needless-borrow → `format!`).
4. **First native run panicked** → root-caused to the post-process WGSL `let`-array bug (#2);
   fixed `let`→`var`; re-ran → no panic, renders.
5. **First screenshots looked too dark** → realized the lighting model is multiplicative
   (`scene × light`), so dark base sprite colors stay dark even when lit. Brightened floor/wall/
   brazier/exit base colors and nudged ambient 0.05→0.08.
6. **User playtested (round 1)** → reported torch not following the player; braziers lighting at
   the wrong spot; HUD too dark. Diagnosed:
   - Proved on paper that `light_position_ndc` matches the sprite transform (`position_ndc.y ==
     -(sprite clip_y)`), so the math was self-consistent — pointed at a *runtime* mismatch.
   - Instrumented both passes (`eprintln!` of `cam_pos`/`vp`) + a temporary auto-walk. Found
     LIGHT and FOLLOW read the *same* live camera (no stale-camera bug), but **vp=(1600,1200)**
     (physical) while the sprite pass uses logical 800×600 → the HiDPI scale mismatch (#3).
     Fixed render.rs to pass `logical_w/logical_h`; confirmed via auto-walk screenshot that the
     torch now tracks the player and is full-size.
   - Removed all instrumentation/temp code (verified 0 `DIAG`/`TEMP`/`eprintln` left).
7. **User re-tested (round 2)** → 17/18; only HUD/messages still dark. Confirmed via render.rs
   that `tr.render(... render_view ...)` runs at line 506, *before* post (517) and lighting
   (538). Asked the user how to handle it; they chose the **engine fix** → moved the text pass
   after post+lighting to `final_view` (#4). Confirmed HUD now bright.
8. **User re-tested (round 3)** → 17/18; only "fps 표시 확인 불가" (1.1). Added an FPS readout to
   the HUD (`HudSystem { fps }`, EMA of 1/dt, top-right). → **18/18**.
9. `verify.sh` green after every engine/example change.

## Key Decisions

- **Example scope (grill-locked):** brazier-lighting puzzle; **torch-fuel decay** as the fail
  state (drives lighting values from gameplay); **>16 lights** to pressure the cap; **shadows/
  occlusion = NON-GOAL** (light passes through walls; design avoids line-of-sight); PostProcess
  **preset always-on + P toggle** (no slider UI); engine-fix bar = VISION default (small fixes
  inline, big features documented); proof = I run native + screenshots + user sign-off + verify.sh.
- **`renderer::FrameContext` stays public** (carried-over open question, now closed). It's the
  parameter type of the public `SpriteRenderer::render`; hiding it shrinks the fork surface.
- **HUD-darkening fix = engine, not example workaround** (user chose this over egui-HUD or
  accept+document). Accepted trade-off: `DrawText` is no longer post-processed.
- **Brighter base sprite colors in the example**, because lighting is multiplicative — a tuning
  insight worth remembering for any lit content (dark base color = dark even when fully lit).
- **Diagnose by instrumentation, not theory.** The position math was provably self-consistent;
  only `cam_pos`/`vp` prints + auto-walk screenshots revealed the physical-vs-logical viewport
  mismatch. Two earlier "diagnoses" (camera-mismatch, then upper-left-torch) were wrong until the
  vp print settled it.

## Evidence & Data

### Diagnostic prints (the decisive data for bug #3)

```
[DIAG] LIGHT cam_pos=Some(Vec2(160.0, 0.0)) vp=(1600,1200)      <- physical (Retina 2x)
[DIAG] FOLLOW cam_pos=Vec2(160.0, 0.0) player=Vec2(599.4, 96.0) <- sprite pass uses logical 800x600
```
LIGHT == FOLLOW camera (no stale-camera bug) but `vp` is physical while sprites are logical →
2× position/radius error scaling with camera displacement (zero at camera origin, which is why
spawn screenshots looked fine and it was never caught on scale-1.0 displays).

### Coordinate math (why it was self-consistent yet still wrong)

`light_position_ndc.y = 2*zoom*(wy-cam.y)/vp_h - 1`; sprite `clip_y = 1 - 2*zoom*(wy-cam.y)/H`;
shader `uv_light.y = position_ndc.y*0.5+0.5` equals the sprite's screen uv **iff vp_h == H**. The
bug was purely that `vp_h`(physical) ≠ `H`(logical).

### Verification (final, all green)

| Check | Result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 warnings |
| `cargo build --target wasm32-unknown-unknown` (lib+bins) | clean |
| `cargo test --all-targets` | pass (incl. new `select_nearest_lights_*`) |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps` | clean |
| native playtest | 18/18 checklist (user) |

### File diffstat (uncommitted)

```
CLAUDE.md                              |  1 +
Cargo.toml                             |  4 ++
docs/CHANGELOG.md                      | 22 ++
docs/HANDOFF.md                        | 75 ++
docs/NEXT_WORK.md                      | 38 +
src/app/render.rs                      | 37 +-
src/renderer/lighting.rs               | 78 ++-
src/renderer/shaders/post_process.wgsl |  4 +-
examples/games/lit_dungeon/            | (new, ~430 lines)
```

## Code Analysis

- **`select_nearest_lights(positions: &[Vec2], camera_pos: Vec2) -> Vec<usize>`** (lighting.rs):
  returns all indices if `len <= MAX_LIGHTS(16)`, else partitions with
  `select_nth_unstable_by(MAX_LIGHTS-1, |a,b| dist²(a).total_cmp(dist²(b)))` and truncates to 16.
  Order within the 16 is unspecified (additive lighting — order-independent).
- **`LightingRenderer::update`** now: collect `(Vec2, PointLight)` via `query2`, warn-once if
  `>16`, pick nearest-16, fill the `[GpuLightData; MAX_LIGHTS]` array. Uses `camera.position`
  (no shake) for culling distance.
- **Render composition (post-fix order):** `scene sprites (render_view) → post → lighting →
  TEXT/HUD (final_view) → fade overlay → egui`. Lighting is `#[cfg(not(wasm32))]`; on wasm
  `use_lighting=false`, so the example compiles for wasm but renders unlit (PostProcess still
  applies on wasm).
- **`torch_from_fuel(fuel)`** linear-maps fuel fraction to (radius, intensity). HUD FPS uses an
  EMA `fps = fps*0.9 + (1/dt)*0.1` (dt is real frame delta; vsync-capped ~60).

## Files Changed

### Engine (source)
- `src/renderer/lighting.rs` — `MAX_LIGHTS` const, `select_nearest_lights` + nearest-16 in
  `update`, warn-once, 2 unit tests.
- `src/renderer/shaders/post_process.wgsl` — `let tap_offsets` → `var tap_offsets` (naga fix).
- `src/app/render.rs` — lighting pass uses `logical_w/logical_h`; text/HUD pass moved after
  post+lighting onto `final_view`.

### Example (new)
- `examples/games/lit_dungeon/lit_dungeon.rs` — the playable acceptance test (~430 lines).
- `Cargo.toml` — `[[example]] lit_dungeon_game`.

### Docs
- `docs/NEXT_WORK.md` — candidate H section (3 engine gaps closed + documented limits); trimmed
  2D lighting + PostProcess from the remaining list.
- `docs/CHANGELOG.md` — `## 2.0.0` Added (example), Fixed (3 engine bugs), Changed (nearest-16).
- `docs/HANDOFF.md` — `## 2026-06-05` session entry + FrameContext architecture decision.
- `CLAUDE.md` — module-map row for `src/renderer/lighting.rs`.

## User Feedback & Preferences (REQUIRED — never omit)

- **User works in Korean; wants conversational replies in Korean.** Handoff/docs stay English
  per the project doc-language rule.
- Used `/grill-me` and answered every round with the **Recommended** option — decisive, trusts
  recommendations but wants the trade-offs laid out as bounded choices first.
- **Playtests thoroughly and reports precisely** — built an HTML checklist (delivered at
  `/tmp/lit_dungeon_test.html`, localStorage-persisted, generates pasteable markdown) and ran it
  three times, giving per-item ✅/❌ + notes. This precise feedback is what pinned the HiDPI and
  HUD bugs. Keep giving them runnable artifacts + screenshots.
- Wanted me to **run + screenshot** for proof, with the user doing final visual sign-off
  (agreed proof split). Prefers I drive verification as far as possible then hand off the
  interactive part.
- Memory: aggressive subagent use for parallel work on Sonnet (not used here — all single-file/
  sequential diagnosis work the main agent handled directly).
- Direct-to-main commit + push is the established repo norm; user explicitly asked to commit+push
  on this handoff ("하고 커밋 푸쉬").

## Where We're Going

1. **Commit + push (this session's close).** Single commit of the 9 changed/added paths + this
   handoff. Draft message:
   `feat(lighting): lit_dungeon example dogfooding 2D lighting + post-process` with body bullets
   (example, nearest-16 cull, post-process shader fix, HiDPI light fix, HUD-after-lighting fix,
   docs). CI will validate compile/clippy/wasm/rustdoc (it cannot run the windowed app — that's
   why these bugs were latent; native playtest covered it).
2. **Check rust-survivors impact (low risk).** Public API unchanged. The render-order change
   (text after post/lighting) and nearest-16 cull are behavior-corrective. If rust-survivors uses
   lighting/post + `TextQueue` HUD together, the HUD will now render after lighting (an
   improvement); otherwise no visible change. Worth a quick `cargo build` of rust-survivors.
3. **Next dogfooding candidates** (none scheduled) from `docs/NEXT_WORK.md`: `BlendTree1D`,
   `Timeline`/cutscene, physics joints, `RenderTarget`/`OffscreenCamera` in real play, networking.
   Each is a "feature proven by a playable example" loop like this one.
4. **Optional polish for lit_dungeon** (only if revisited): the top-left fuel HUD text overlaps
   the spawn torch glow (washed out until the player moves); FPS is vsync-capped so it reads ~60
   (fine for the check, not a perf profiler — `ProfilerData.systems` is the real CPU signal).

## Risks & Blockers

- **Low.** verify.sh green; native playtest 18/18. The render-composition change is the only
  cross-cutting edit — but only `lit_dungeon` combines lighting/post + `TextQueue` today, and for
  non-lit/non-post games `render_view == final_view` so text placement is unchanged.
- CI runs compile/clippy/wasm/rustdoc only; it will NOT catch a future shader-validation or
  lighting-alignment regression (no GPU/windowed run in CI). Lighting/post changes still need a
  **native run** to validate — keep using the example as the runtime gate.
- The 4 engine fixes are additive/corrective; if any is later tightened, re-run the example
  natively (especially on a Retina display for the HiDPI path).

## Open Questions

- Should screen-space `DrawText` ever be post-processed? The fix makes it always-unlit/un-post;
  egui is the post-after layer. Fine for now; revisit if a game wants CRT-over-HUD.
- FPS readout uses `1/dt` (vsync-capped). If a real perf HUD is wanted later, surface
  `ProfilerData` per-system timings instead (as survivor_game does).

## Quick Start for Next Session

```bash
# Restore context
cat plans/handoffs/HANDOFF_lit-dungeon-lighting_example-and-engine-fixes_2026-06-05.md

# Confirm state
git log --oneline -5
git status -s
./scripts/verify.sh                      # 5 checks, expect exit 0
cargo run --example lit_dungeon_game     # native; WASD move, E light, P post toggle, R restart

# Key files
sed -n '1,40p' examples/games/lit_dungeon/lit_dungeon.rs   # the example (acceptance test)
grep -n "select_nearest_lights\|MAX_LIGHTS" src/renderer/lighting.rs
grep -n "logical_w\|text_renderer\|use_lighting" src/app/render.rs

# Next: pick the next never-in-a-game subsystem from docs/NEXT_WORK.md and run the same loop,
# or build rust-survivors to confirm no fallout from the render-order change.
```
