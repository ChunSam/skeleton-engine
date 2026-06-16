# Autonomous 4-item loop — shader_material + parallax + render() split (v10.0.0 → v10.2.1)

**Date:** 2026-06-17
**Status:** COMPLETED — 3 PRs shipped + merged, 3 tags + GitHub Release. `main` @ `5410e74`, v10.2.1, clean + CI-green, 732 lib tests. Loop ended per user instruction.
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine — engine hardening / architecture
**Chain:** `engine-hardening` seq `10`
**Parent:** `HANDOFF_engine-hardening_v10-breaking-pass_2026-06-16.md` (seq 9)
**Prior chain:** seq 1 (v9.0.0 shipped) → 2 (merged) → 3 (priority loop) → 4 (cohesion review) → 5 (v9.4.0) → 6 (v9.4.1) → 7 (v9.5.0) → 8 (session summary) → 9 (v10 breaking pass) → **this (10)**

---

## Since Last Handoff

The parent (seq 9) finished the v10 breaking arc (7/8 items) and left **item F (split `App::render()`) descoped** with the open question: "Do item F with a full render-mode playtest, or leave it descoped permanently?" plus "Add a `shader_material` example?" and "Tag/publish v10.0.0?".

This session answered all three:
- **Started a `/loop`** (dynamic self-paced mode) over a 4-item recommended-order backlog the user greenlit ("번호 순서대로 진행 … 머지 진행 가능").
- **Item F: DONE** — but as a deliberately **PARTIAL, risk-managed split** (not the full ~15-helper split the v10 plan sketched), because the render-pass core is borrow-hostile to extraction. Gated on a full render-mode visual playtest (the parent's required gate).
- **shader_material example: DONE** (v10.1.0) — and it revealed `ShaderMaterial` was a *public shipped feature with zero example coverage*, a genuine VISION-bar violation.
- **Tag/publish: DONE** — git tags v10.1.0/v10.2.0/v10.2.1 (first tags since v4.3.0) + GitHub Release v10.2.1. crates.io publish deliberately NOT done (see Open Questions).
- Trajectory: the "clean additive backlog exhausted" note from earlier seqs still holds — new work now requires *picking* a VISION feature (parallax was this session's pick). No regressions; main stayed green throughout.

## Related Handoffs

The `engine-hardening` chain is long (seq 1–9 all `2026-06-16`). Most relevant ancestors for the next session: **seq 9** (`v10-breaking-pass`, the direct parent — v10 arc + the descoped-F decision this session reversed) and **seq 4** (`module-cohesion-review`, the source of the v10 items). Older seqs (v9.x) are superseded. The parent also carries a "Reusable Gotchas" + "Reusable smoke-test recipe" block worth re-reading.

## Broader engine state (orientation for a fresh session)

- The v10 arc (seq 9) shipped items A,B,C,E,G,H,I,J,K + descoped F (now done here). Item D (`get_collider`→`collider` rename) was optional and **skipped**. Every engine subsystem already has ≥1 playable example (since v4.x). The "clean mechanical/additive backlog" is **exhausted** — new work = picking a VISION feature (as parallax was).
- Examples are the acceptance tests. New top-level examples added this session: `shader_material`, `parallax_scroll`. Render-mode coverage examples (for future render playtests): `security_camera` (offscreen), `lit_dungeon_game` (lighting+post), `gpu_particles`, `timeline_cutscene` (fade), any + F2 (docked editor).

## Reference Documents

- `CLAUDE.md` — agent quick reference (module map; bumped to v1.6.52 / package v10.2.1 this session)
- `docs/VISION.md` — the forkable-skeleton vision + the "feature isn't done until a playable example exercises it" acceptance bar (drove items 1 & 2)
- `plans/V10_BREAKING_PASS_PLAN_2026-06-16.md` — the v10 plan; item F detail (effort/risk) is here
- `docs/CHANGELOG.md` — `## 10.1.0`, `## 10.2.0`, `## 10.2.1` sections added this session

## The Goal

Keep hardening/extending the `skeleton-engine` (a forkable, MIT, genre-agnostic 2D wgpu engine) without regressions. This session's concrete goal: execute an autonomous 4-item loop — (1) a `shader_material` example, (2) a new VISION feature + example, (3) v10 item F (`render()` split), (4) tag/publish — then hand off and stop. Each item is one CI-green squash-merged PR following the established loop: opus scopes → (Sonnet agent or opus) implements → opus independently re-verifies Gate6 + visual playtest → version/CHANGELOG/CLAUDE bump → PR → CI → merge.

## Where We Are

- **`main` @ `5410e74`**, package **v10.2.1** (was v10.0.0 at session start), clean working tree.
- **732 lib tests** pass (was 729; +3 from the new parallax unit tests). Full Gate6 green.
- **3 PRs shipped + squash-merged this session**, branches deleted:
  - **#80** (v10.1.0) — `examples/shader_material.rs` (custom-shader VISION acceptance test).
  - **#81** (v10.2.0) — `src/parallax.rs` (`ParallaxLayer` + `ParallaxSystem`) + `examples/parallax_scroll.rs`.
  - **#82** (v10.2.1) — partial split of `App::render()` into 5 private per-pass helpers.
- **3 git tags pushed**: `v10.1.0` @ `8b8ffca`, `v10.2.0` @ `b7b5142`, `v10.2.1` @ `5410e74` (first tags since v4.3.0 — v5.x–v9.x went untagged).
- **GitHub Release v10.2.1** created: https://github.com/ChunSam/skeleton-engine/releases/tag/v10.2.1 (first-ever GH release for this repo).
- **crates.io: still NOT published** — the crate name `skeleton-engine` does not exist on crates.io (never published). Deliberately left for explicit user confirmation.
- New public API surface added: `engine::ParallaxLayer`, `engine::ParallaxSystem`. (`ShaderMaterial` already existed; only its example was new.)
- `render()` shrank from ~890 lines to ~610; `src/app/render.rs` net +58 lines (helper doc comments/signatures offset the call-site shrink).
- CLAUDE.md gained two module-map rows (ShaderMaterial @ #80, ParallaxLayer/System @ #81); now 186 lines (≤200 cap OK).
- The `engine-current-state` auto-memory is now STALE (says v9.5.0 @ 79ba57f) — updated this session to v10.2.1 (see memory step).
- Session opened with a clean Gate6 baseline (`./scripts/verify.sh` green @ e06cf36, v10.0.0, 729 lib tests) before any work.
- `examples/*.rs` (top-level) are **auto-discovered** by cargo — no `Cargo.toml [[example]]` entry needed (only the `examples/games/*/` subdir games are explicitly listed). Both new examples are top-level.
- No engine file other than `src/app/render.rs` + `src/lib.rs` + `src/parallax.rs` was touched; all public APIs from the v10 arc remain intact (no stale refs vs parent seq 9).
- Both background agents wrote to the main working tree (no `.claude/worktrees/agent-*` surprise); confirmed via `git worktree list` after each.

## What We Tried (Chronological)

1. **Onboarding + Gate6 baseline.** Ran `./scripts/verify.sh` at session start → green (main @ e06cf36, v10.0.0). Read the v10 plan + parent handoff "Reusable Gotchas". Listed the recommended order; user fired `/loop`.

2. **Item 1 — shader_material (PR #80, v10.1.0).** Scoped inline: grepped and found `ShaderMaterial` is a public crate-root re-export (`src/material.rs`) with **zero examples** — a VISION gap. Confirmed the WGSL contract: fragment is a *separate* module from the built-in vertex shader (`src/renderer/sprite/material.rs::compile_pipeline`), so each `frag_source` must be a standalone module declaring `@group(1)` texture/sampler, `@group(2)` params, a subset `VertexOutput` (locations 0=uv, 1=color), and `fs_main`. Delegated impl to a background **Sonnet agent** (explicit `model: sonnet`). Result: 301-line example, 3 distinct shaders (hue-cycle, plasma, dissolve), per-frame `params[0]=time` update + ↑/↓ dissolve threshold. Independent Gate6 green; **visual playtest PASS** — all 3 custom pipelines render, dissolve ↑ confirmed 0.50→0.78. The documented contract (subset VertexOutput) validated end-to-end (was never actually run before — only example coverage proved it works).

3. **Item 2 — parallax (PR #81, v10.2.0).** Picked parallax after grepping for missing genre-agnostic primitives (`parallax`/`nine_slice`/`coroutine`/`TweenSequence` all = 0 hits; camera **shake already exists** in `src/camera.rs`). Designed the API myself (lazy base+camera capture so callers do no base bookkeeping; formula `pos = base + (cam - cam_ref) * (1 - factor)` → `screen = base - cam*factor`). Verified camera integration: `Camera.position` is pub (viewport top-left), `Camera` is a World resource, camera follow/clamp runs AFTER the user system loop (schedule.rs ~410-428) → `ParallaxSystem` as a user system has a sub-perceptual 1-frame lag (documented). Delegated to a background Sonnet agent with a precise spec. Result: `src/parallax.rs` (232 lines, 3 unit tests pinning factor 1.0/0.0/0.5) + `examples/parallax_scroll.rs` (277 lines, 4-layer side-scroller). Gate6 green (732 tests); **visual playtest PASS** — before/after scroll showed the 4 bands moving at distinct rates, player stays centered.

4. **Item 3 — render() split (PR #82, v10.2.1).** Read all ~890 lines of `App::render()`. **Found the core obstacle**: the function relies on disjoint partial borrows of `self.{gpu,render,world,editor}`, and `render_view` frequently *aliases into* `self.render` (e.g. `post_renderer.target_view`, `scene_texture_for_lighting`) while the sprite pass also takes `&mut self.render.sprite_renderer` — only legal as disjoint *field* borrows within one function body. So a `&mut self` helper can't wrap those passes. **Decision: partial split** — extracted only the separable concerns into static helpers; left the render_view-aliased pass sequence inline as one annotated flow. Did the edits MYSELF (not an agent — the parent flagged agents struggle on the render path), building after each extraction. Extracted: `setup_post_renderer`, `setup_lighting`, `render_offscreen_targets`, `present_docked_placeholder`, `present_egui`. Hit a `cargo fmt --check` failure on first Gate6 (my hand-formatting) → `cargo fmt` fixed it. **Full render-mode visual playtest PASS** (7 modes — see Evidence). render() → ~610 lines.

5. **Item 4 — tag/publish.** Checked precedent: latest tag was v4.3.0 (huge gap); crates.io has no `skeleton-engine`; no GH releases. Interpreted "태그/배포" as the established git-tag release (the parent treated "Tag / publish" as one git action). Tagged all 3 session versions at their merge commits (verified each commit's Cargo.toml matched), pushed, created GH Release v10.2.1. Skipped crates.io (irreversible first-ever publish).

## Key Decisions

- **Item F as a PARTIAL split, not the full ~15-helper split.** The render-pass core is borrow-hostile (render_view aliases into RenderState). A full split would thread the encoder + target view through ~8 micro-functions — *worse* readability + more GPU-silent-regression surface for zero benefit. Extracting the genuinely separable concerns (setup, offscreen, docked-wait, egui) is the correct design, not a half-measure. Documented this rationale in the PR/CHANGELOG.
- **Did item F's edits by hand (opus), not via a Sonnet agent.** The parent's gotchas warn agents struggle on the render path (worktree surprises, stale diagnostics). Incremental hand-edits + build-after-each were safer.
- **Items 1 & 2 delegated to background Sonnet agents** (per standing preference + `CLAUDE_CODE_SUBAGENT_MODEL=claude-sonnet-4-6` the user set mid-session). Self-contained, well-specified tasks; opus reviewed + Gate6 + playtested each.
- **shader_material uses `Sprite::colored` (white 1×1 fallback texture), no asset files** — keeps the example self-contained while still proving the `t_sprite`/`s_sprite` bindings.
- **Parallax: lazy anchor capture** (base + cam_ref captured on first `ParallaxSystem` run) so callers just place the sprite — no `base` field to set. Works regardless of the camera's starting position.
- **crates.io NOT published.** Irreversible (claims the global name forever), never done before, and "배포" doesn't unambiguously authorize a first-ever publish. Git tag + GH release is the precedent-matching release.
- **Tagged all 3 session versions** (not just v10.2.1) at their exact merge commits — clean history, trivially reversible, more correct than leaving gaps.
- **Version-bump policy applied** (reusable convention, matches repo history): new *example* for an existing feature → **minor** (10.0.0→10.1.0); new *public feature* + example → **minor** (→10.2.0); *internal-only* refactor with no public API change → **patch** (→10.2.1). Each PR bumps `Cargo.toml` + `Cargo.lock` + `docs/CHANGELOG.md` (new `##` section at top) + `CLAUDE.md` header (`Version vX.Y.ZZ | package vA.B.C`); add a module-map row when a new public type/module lands.
- **Used `AskUserQuestion`? No.** Deliberately avoided re-prompting mid-loop — the user's directive was explicit and autonomy was the point. The one genuinely ambiguous call (crates.io publish) was resolved conservatively (skip + flag) rather than by interrupting.

## Evidence & Data

### PRs shipped (all squash-merged, branches deleted)

| PR | Version | Title | Tests | Playtest |
|---|---|---|---|---|
| #80 | 10.1.0 | shader_material example | 729 | 3 shaders render; dissolve ↑ 0.50→0.78 |
| #81 | 10.2.0 | parallax (ParallaxLayer + ParallaxSystem) | 732 (+3) | 4 bands distinct scroll rates |
| #82 | 10.2.1 | partial split of App::render() | 732 | 7 render modes (below) |

### Item F render-mode playtest matrix (the gate item F required)

| Mode | Example | Extraction exercised | Result |
|---|---|---|---|
| offscreen RT | `security_camera` | `render_offscreen_targets` | ✅ monitor shows offscreen view |
| lighting | `lit_dungeon_game` | `setup_lighting` | ✅ lit scene (dark + torch glow) |
| post+lighting | `lit_dungeon_game` + P | `setup_post_renderer` + combined | ✅ post toggled, no errors |
| docked editor | `basic` + F2 | `present_docked_placeholder`/`present_egui` | ✅ docked layout renders |
| normal + custom pipeline | `shader_material` | main pass + MaterialRenderer (inline) | ✅ 3 shaders render |
| GPU particles | `gpu_particles` | inline path | ✅ HUD + renderer init |
| fade | `timeline_cutscene` | inline (unchanged) | ✅ scene renders clean |

All 7 logs scanned for `panic|validation|error|wgpu` → CLEAN.

### Tags + release

| Tag | Commit | Cargo.toml version verified |
|---|---|---|
| v10.1.0 | 8b8ffca | 10.1.0 ✅ |
| v10.2.0 | b7b5142 | 10.2.0 ✅ |
| v10.2.1 | 5410e74 | 10.2.1 ✅ |

GH Release: v10.2.1 (notes cover the 10.1.0→10.2.1 cycle).

### Item F — extraction sequence (built + verified after each step)

| Step | Helper(s) extracted | Inline block removed | Build |
|---|---|---|---|
| 1 | `setup_post_renderer` + `setup_lighting` | post/lighting init (~75 lines) | ✅ |
| 2 | `render_offscreen_targets` | offscreen RT loop (~124 lines) | ✅ |
| 3 | `present_docked_placeholder` | docked-wait early-return (~85 lines) | ✅ |
| 4 | `present_egui` | final egui pass (~38 lines) | ✅ |

`render()` `fn render(&mut self)` started at line 188 (~890 lines through `Ok(())`), ended at ~610 lines after step 4. Then full Gate6 (failed fmt → `cargo fmt` → re-ran green) + the 7-mode playtest.

### Item F — the do-it-or-not deliberation (recorded so it isn't re-litigated)

F was the parent's *descoped* item ("RenderPlugin already covers the fork need; only item CI can't verify; HIGH risk"). The user's `/loop` said "번호 순서대로 진행" with F as item 3 in a list that labeled it "item F … 전 렌더 모드 visual playtest 필수". Resolution: the user had the risk info and chose to proceed → honor it, but execute the *risk-managed partial* version (separable concerns only) gated on the playtest, rather than (a) skipping against instructions or (b) force-extracting the borrow-hostile core. Did NOT re-prompt — re-litigating a clear directive mid-autonomous-loop conflicts with the user's "재실행"/"머지 진행 가능" autonomy signals.

### Gate6 commands (the bar, run independently after every agent + every hand-edit)

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo build --target wasm32-unknown-unknown` (lib+bins, NOT `--all-targets`) · `cargo test --all-targets` · `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` — all via `./scripts/verify.sh` (prints `all checks passed ✓` + exit 0). Doctest count this session: 53 passed / 33 ignored.

### Background Sonnet agent metrics

| Item | Task | Tokens | Tool uses | Duration |
|---|---|---|---|---|
| #80 | shader_material example | 38,680 | 19 | ~133 s |
| #81 | parallax feature + example | 59,322 | 40 | ~385 s |

Item F (#82) was done by opus directly (no agent) — see Key Decisions.

### CI timings (per PR, all 4 checks green)

| PR | Build (WASM) | Package | Rustdoc | Test (native) |
|---|---|---|---|---|
| #80 | 37s | 1m18s | 50s | 4m41s |
| #81 | 39s | 1m9s | 51s | 3m7s |
| #82 | 36s | 1m8s | 48s | 3m38s |

## Code Analysis

- **`render()` helpers extracted** (all private fns on `impl App` in `src/app/render.rs`; static-style, take explicit disjoint borrows to avoid the live-`gpu` conflict):
  - `setup_post_renderer(render: &mut RenderState, device, w, h, fmt)`
  - `setup_lighting(render: &mut RenderState, world: &World, device, w, h, fmt, use_post) -> bool` (`#[cfg(not(wasm32))]`)
  - `render_offscreen_targets(render: &mut RenderState, world: &mut World, gpu: &GpuContext)` — own encoders + submits; the biggest/riskiest extraction.
  - `present_docked_placeholder(render: &mut RenderState, window: Option<&Window>, gpu: &mut GpuContext) -> Result<(), wgpu::CurrentSurfaceTexture>` (`#[cfg(not(wasm32))]`, early-return path)
  - `present_egui(render: &mut RenderState, gpu: &GpuContext, final_view: &wgpu::TextureView)`
- **Why the core stays inline:** `render_view` is computed as a borrow into `self.render` (post_renderer / scene_texture_for_lighting) and the sprite/UI/particle/plugin/post/lighting/text/fade passes use it while also `&mut`-borrowing other `self.render` fields. NLL disjoint-field borrows only hold within a single function body. A `&mut self` (or `&mut RenderState`) helper would conflict with the live `render_view`.
- **`ParallaxLayer`** (`src/parallax.rs`): `pub factor: Vec2` + private `base: Option<Vec2>`, `cam_ref: Option<Vec2>`. Ctors `new`/`horizontal(fx)→(fx,1.0)`/`vertical(fy)→(1.0,fy)`. `ParallaxSystem` (`#[derive(Default)]`) runs `pos = base + (cam - cam_ref) * (Vec2::ONE - factor)`; collect-entities-then-`get_mut` borrow pattern.
- **`ShaderMaterial` WGSL contract** (validated by #80): fragment is a separate module; needs the documented prelude (`@group(1)` tex/sampler, `@group(2)` `params: vec4<f32>`, subset `VertexOutput` locations 0/1, `fs_main`). Renderer caches one pipeline per `source_hash` in `MaterialRenderer`.
- **`examples/shader_material.rs` shaders** (3 `const &str` raw WGSL): (1) *hue cycle* — `hue_to_rgb(fract(uv.x + params[0]*0.3))`; (2) *plasma* — 4 summed `sin()` interference waves (h/v/diag/radial) phase-shifted into RGB; (3) *dissolve* — bilinear value-noise vs `params[1]` threshold, `discard` below, `smoothstep` orange glow at the edge. All multiply by `textureSample(t_sprite,...) * in.color` (white fallback). Driver system writes `params[0]=elapsed` to all 3 each frame; ↑/↓ drive dissolve `params[1]` (clamped 0..1).
- **`examples/parallax_scroll.rs` layout**: WORLD_W=3000, WIN 960×540. 4 layers via `spawn_layer(factor, y, h, color, z, count)` — sky `0.10` (y=80), mountains `0.35` (y=260), trees `0.65` (y=360), foreground bushes `1.10` (y=470, faster-than-world). Player (yellow) + a static ground strip (no parallax). Camera follows an **invisible `CameraAnchor` entity** set to `(player.x - WIN_W/2).clamp(0, WORLD_W-WIN_W)` each frame — needed because `Camera::position` is the viewport top-left, not its center; `cam.bounds` + `lerp_factor=8`. System order: `PlayerSystem` → `CameraFollowSystem` → `ParallaxSystem` → `HudSystem`.

### Non-obvious technical findings (worth preserving)

- **wgpu inter-stage IO accepts a SUBSET.** `ShaderMaterial`'s vertex stage (built-in `sprite.wgsl`) outputs `VertexOutput` with locations 0,1,2,3; the documented custom-frag prelude declares only locations 0 (uv) + 1 (color). This validates fine — wgpu requires fragment inputs to be a *subset* of vertex outputs, not an exact match. This was the docs' promise but had **never been run** (no example) until #80 proved it on a real GPU. If a future wgpu bump tightens this, the `shader_material` playtest is what catches it.
- **Parallax 1-frame-lag is sub-perceptual, by math.** `ParallaxSystem` reads the camera from the previous frame's end (engine finalizes follow/clamp *after* the user loop). During constant camera velocity `v`, a layer's apparent position error is `v*factor` per frame — a constant offset that just reads as a slightly different depth, invisible. Only camera *acceleration* could shimmer, minutely. Making it a built-in post-camera system would remove the lag but adds a hot-loop query + engine surface; chose the explicit user-system (matches `SteeringSystem`/`UiSystem` ergonomics) and documented the lag.
- **`Camera` shake already exists** (`shake(strength, duration)` + `shake_offset()` in `src/camera.rs`, applied inside `view_proj`) — don't re-add it; parallax was chosen partly because it *pairs* with the existing shake/follow as "juice".

## Reusable Gotchas (HIGH VALUE — read before next session)

- **Render path has no CI/GPU test — the visual playtest IS the gate.** A wrong submit order, texture view, or `LoadOp` is *visually silent* (compiles + passes all of Gate6). After ANY `src/app/render.rs` change, run the macOS screencapture playtest across the modes the change touches (recipe in Quick Start). This caught nothing this time *because* the partial split was careful — but it's the only safety net.
- **Stale mid-edit diagnostics fired repeatedly again** (the parent warned of this). Every extraction this session produced a `dead_code: associated function never used` warning blob *after a clean build* — they were snapshots from between the "add helper" and "add call site" edits. **Each time the actual `cargo build`/clippy was green.** Trust the build, not the diagnostic blob. Verified clean by grepping clippy output for `never used` (empty) + confirming call sites exist.
- **`cargo fmt --check` failed on hand-written render.rs edits** (item F first Gate6, `REAL_EXIT=1` on the fmt step). Trivially fixed with `cargo fmt`. Always run `cargo fmt` after hand-editing before the gate — rustfmt reflows match arms / long calls differently than you'll type them.
- **`gh pr checks <n> --watch` exited EARLY on #81** (`exit 1`, all checks still "pending", `REAL_CHECK_EXIT=8`) — a race right after PR creation before checks register. Re-running the watch worked. Mitigation now baked in: `sleep 8` before the watch + `--fail-fast`.
- **No agent-worktree surprise this session** — both Sonnet agents (run with default isolation) wrote to the **main tree** directly. Still ran `git worktree list` + `git status --short` after each agent to confirm (the parent's recovery procedure wasn't needed).
- **Background-task driven loop works cleanly:** spawn agent / verify.sh / `gh pr checks --watch` as `run_in_background` → the harness auto-wakes on completion (no polling). `ScheduleWakeup` was used only as a long (1200–1800s) safety-net fallback, never as the primary signal.

## Files Changed

### Source code
- `src/app/render.rs` — extracted 5 private per-pass helpers from `render()` (#82). +368/−310.
- `src/parallax.rs` — NEW (#81). `ParallaxLayer` + `ParallaxSystem` + 3 unit tests (232 lines).
- `src/lib.rs` — `pub mod parallax;` + `pub use parallax::{ParallaxLayer, ParallaxSystem};` (#81).

### Examples
- `examples/shader_material.rs` — NEW (#80, 301 lines).
- `examples/parallax_scroll.rs` — NEW (#81, 277 lines).

### Docs / meta
- `Cargo.toml` / `Cargo.lock` — version 10.0.0 → 10.1.0 → 10.2.0 → 10.2.1.
- `docs/CHANGELOG.md` — added `## 10.1.0`, `## 10.2.0`, `## 10.2.1`.
- `CLAUDE.md` — version header bumps (v1.6.49→v1.6.52) + 2 new module-map rows (ShaderMaterial, ParallaxLayer/System).

## User Feedback & Preferences (REQUIRED)

- **`/loop 번호 순서대로 진행. 10.0.0태그/배포 이후 handoff 하고 작업 종료. 머지 진행 가능.`** — the session's driving instruction: run the recommended order autonomously, end with tag/publish + handoff, merge authority granted.
- **Asked for the recommended order first** ("추천 작업 순서대로 리스트업 해줘") before firing the loop — wanted a plan, then executed it.
- **Set `export CLAUDE_CODE_SUBAGENT_MODEL=claude-sonnet-4-6` mid-session** — confirms the Sonnet-subagent preference; aligns with the standing `[[new-model-subagent-incompat]]` policy of always setting an explicit model.
- **Standing (carried from parent):** Korean prose to the user / English code+docs+handoff; Sonnet subagents with explicit `model:`; no breaking changes without sign-off; **never tag unprompted** (this session was explicitly prompted); **merge authority must be re-confirmed in a new session** — it was granted for THIS loop only.
- **"번호 순서대로 진행"** taken as authorizing item F despite its prior descope — the user saw it labeled "item F, playtest 필수" in the list and chose to proceed in order.

## Where We're Going

(No work in flight — loop ended. These are options for a fresh session.)
1. **Decide crates.io publish** — if yes, `cargo publish` (irreversible; needs explicit go). Otherwise leave as a GitHub-only release.
2. **Backfill v5.x–v9.x git tags** (optional) — they're all untagged; could tag retroactively at their merge commits for a complete history.
3. **Next VISION feature + example** — the additive backlog is exhausted; pick one and follow the feature→example→fix-API-if-awkward loop. Candidates with 0 current `src` hits, each genre-agnostic + forkable + demoable:
   - **nine-slice / 9-patch sprites** — scalable UI panels/borders without distortion (UI has `Panel` but no 9-slice).
   - **TweenSequence / chained tweens** — `Tween` is single-value-single-shot; no chaining/sequencing primitive (`tween.rs`).
   - **coroutine-style sequencing** — run-after-delay / do-over-time scripting beyond `Timer`/`Timeline`.
   - **animated tiles** — `Tilemap` is static per-cell; per-tile frame animation is common in 2D.
   - **music-track crossfade / playlist** — `AudioManager` has bus mixer + fades but no track-to-track crossfade helper.
4. **Optional GH releases for v10.1.0 / v10.2.0** if a per-version release page is wanted.

## Risks & Blockers

- **None blocking.** main green + clean at `5410e74`, v10.2.1; all 3 PRs CI-green + squash-merged + branches deleted.
- **Render path has no CI/GPU test** — any future `render()` change still needs the manual screencapture playtest (recipe below). The item-F split was verified this way; future edits must be too. The partial split *reduced* this risk surface (the dangerous render_view-aliased core is untouched) but didn't eliminate the need.
- **The 7-mode playtest depends on this macOS box** (screencapture + osascript synthetic input + a real GPU). On a headless/CI runner it won't work — that's exactly why item F couldn't be CI-gated.
- **Merge authority was loop-scoped** — do not assume it carries into the next session.

## Open Questions

- **Publish to crates.io?** Never done; irreversible first-ever name claim. Default: no — needs explicit user confirmation.
- **Backfill the missing v5–v9 tags?** Not asked; low value but cheap.
- **Which VISION feature next?** No signal yet — user picks, or I propose from the 0-hit candidates.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -6        # 5410e74(#82 render split) b7b5142(#81 parallax) 8b8ffca(#80 shader)
grep -m1 '^version' Cargo.toml   # 10.2.1
git status -s               # clean
git tag --sort=-v:refname | head -4   # v10.2.1 v10.2.0 v10.1.0 v4.3.0
./scripts/verify.sh         # main green (732 lib tests)

# Read first
#   THIS handoff (seq 10)
#   plans/V10_BREAKING_PASS_PLAN_2026-06-16.md   (item F context)
#   docs/CHANGELOG.md  → ## 10.2.1 / 10.2.0 / 10.1.0

# Render-mode visual playtest recipe (REQUIRED after any render.rs change — CI can't catch GPU bugs):
#   EX=security_camera   # or lit_dungeon_game / basic(+F2 via key code 120) / gpu_particles / shader_material
#   cargo build --example "$EX"
#   ( ./target/debug/examples/"$EX" > /tmp/s.log 2>&1 & echo $! > /tmp/s.pid )
#   sleep 7; osascript -e "tell application \"System Events\" to set frontmost of (first process whose name is \"$EX\") to true"
#   sleep 1; screencapture -x /tmp/s.png; kill "$(cat /tmp/s.pid)"; pkill -f "examples/$EX"
#   grep -aiE 'panic|validation|error|wgpu' /tmp/s.log   # empty = good; then Read /tmp/s.png
#   Synthetic key input: osascript repeat N times { key code <code> }; D=2 Up=126 Down=125 Right=124 Left=123 P=35 F2=120
#   (focus is finicky — set frontmost first, send 40-150 rapid presses with ~0.02s delay)

# Process gotchas
#   - Re-verify Gate6 INDEPENDENTLY after agent work; run `cargo fmt --check` explicitly (hand-edits drift).
#   - dead_code/E0xxx mid-edit diagnostics are STALE snapshots — trust the build, not the diagnostic blob.
#   - `gh pr checks <n> --watch` can EXIT EARLY (race) right after PR create with all checks "pending"
#     (seen on #81, exit 1, REAL_CHECK_EXIT=8). Re-run the watch; add a `sleep 8` before it.
#   - MERGE AUTHORITY was granted for the prior loop only — RE-CONFIRM before merging in a new session.

# Next action (pick one — no work in flight):
#   (a) Propose/scope the next VISION feature + example (additive backlog exhausted).
#   (b) Publish to crates.io — ONLY on explicit user go (irreversible first-ever publish).
#   (c) Backfill v5–v9 git tags.
```

## Session Closed
**Closed at:** 2026-06-17 00:08 KST
**Commit:** this commit (see `git log` for `session: vision-features-render-split`)
**Session status:** Handed off to next session (seq 10). Loop ended per user instruction; no wakeup armed.
