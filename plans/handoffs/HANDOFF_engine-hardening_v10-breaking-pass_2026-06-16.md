# v10 breaking architecture pass — 9 PRs shipped (v9.5.1 → v10.0.0 COMPLETE)

**Date:** 2026-06-16
**Status:** COMPLETE — v10 arc done (7/8 items; F descoped). `main` @ `db07194`, v10.0.0, clean + CI-green, 729 lib tests.
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine — engine hardening / architecture
**Chain:** `engine-hardening` seq `9`
**Parent:** `HANDOFF_engine-hardening_session-summary-v9.5.1_2026-06-16.md` (seq 8)
**Prior chain:** seq 1 (v9.0.0 shipped) → 2 (merged) → 3 (priority loop) → 4 (cohesion review) → 5 (v9.4.0) → 6 (v9.4.1) → 7 (v9.5.0) → 8 (session summary) → **this (9)**

> This session drained the last additive backlog (v9.6.0/v9.6.1), then **scoped and executed the
> v10 breaking architecture pass** from the cohesion review. The breaking surface turned out tiny;
> the bulk was internal refactors. One item (F, split `render()`) was deliberately descoped.
> Read alongside `plans/V10_BREAKING_PASS_PLAN_2026-06-16.md` (the per-item breaking-surface analysis).

---

## What this session shipped (9 PRs, all merged + CI-green)

Starting state: `main` @ `1aec558` (v9.5.1). Ending state: **`main` @ `db07194`, v10.0.0, 729 lib tests, clean.**

| PR | Ver | What |
|---|---|---|
| #71 | 9.6.0 | **`RenderPlugin`** trait + `App::add_render_plugin` — fork-friendly custom render-pass hook (additive; runs after sprite/UI/particle passes, before post/lighting). `FrameContext` gained `pub format`. Example `render_plugin` (animated vignette). |
| #72 | 9.6.1 | `Wander::direction_fn`/`with_direction_fn` (swap the wander RNG without forking `SteeringSystem`); **fixed 2 long-broken doctests** (`register_serde_component` stale `App::new` arg; `register_editable_component` imported the `Reflect` trait not the derive macro); **added `cargo test --doc` to CI + `verify.sh`** (doctests were silently rotting — `--all-targets` skips them). |
| #73 | 10.0.0 | **v10 item I:** split `tilemap.rs` (1620L) → `tilemap/{mod,autotile,system}.rs`; removed unused pub `cell_display_uv`; **fixed `verify.sh` exec bit** (`100644`→`100755` in git). |
| #74 | 10.0.0 | **v10 items A/B/C:** `RenderTarget`/`LightingRenderer` wgpu fields → `pub(crate)` + escape-hatch accessors; `RenderTarget::new` builds its own bind-group layout (drops borrowed `texture_layout`; removed `SpriteRenderer::texture_layout()`). |
| #75 | 10.0.0 | **v10 item J (BREAKING):** `UiSystem`/`SteeringSystem` unit structs → reused scratch fields; all `add_system(X)` → `add_system(X::default())` (22 in-repo sites migrated). Killed 11 per-frame `Vec<Entity>` allocs (closes deferred #76). |
| #76 | 10.0.0 | **v10 item K:** `ScriptAsset`/Rhai decoupled from `asset.rs` → `scripting/{asset,loading}.rs` + new `ScriptRegistry` World resource (hot-reload via the `HotReloadable` seam). `asset.rs` now rhai-free. `engine::ScriptAsset` re-exported at crate root (source-compat). |
| #77 | 10.0.0 | **v10 item E:** extracted 14 renderer/texture/egui fields from the `App` god-struct into internal `RenderState` (`src/app/render_state.rs`). `gpu`+`world` stay on `App`. Enabler for G. |
| #78 | 10.0.0 | **v10 item G:** split `schedule::update()` (386L) → `compute_viewport()`/`run_systems()`/`post_systems()` + egui begin/end → `egui_pass.rs`. Order byte-equivalent (test-guarded). |
| #79 | 10.0.0 | **v10 item H:** split the 5-concern `SpriteRenderer` → `SpriteRenderer` (batching+UI) owning `TextureCache` + `MaterialRenderer`. GPU wiring byte-identical; **visually smoke-tested** (`basic`+`security_camera`). Final v10 item. |

## v10 item ledger (cohesion-review items A–K → outcome)

The plan (`plans/V10_BREAKING_PASS_PLAN_2026-06-16.md`) enumerated 11 items. Final disposition:

| Item | What | PR | Breaking? | Effort | Status |
|---|---|---|---|---|---|
| A | `RenderTarget` wgpu fields → `pub(crate)` + accessors | #74 | yes (0 observed users) | S | ✅ done |
| B | `LightingRenderer` fields → `pub(crate)` | #74 | no (not in prelude) | S | ✅ done |
| C | `RenderTarget::new` builds own layout (drop `texture_layout()`) | #74 | no (not in prelude) | S | ✅ done |
| D | `get_collider`→`collider` rename | — | optional | S | ⏭️ skipped (escape-hatch doc already there; rename not worth it) |
| E | extract `RenderState` from `App` | #77 | no (all private) | L | ✅ done |
| F | split `render()` (839L) | — | no (private) | L | ⛔ **DESCOPED** (RenderPlugin covers fork need; no GPU CI test) |
| G | split `update()` (386L) | #78 | no (`pub(super)`) | M | ✅ done |
| H | `SpriteRenderer`→`MaterialRenderer`+`TextureCache` | #79 | no (not in prelude) | L | ✅ done (smoke-tested) |
| I | split `tilemap.rs` (1620L) | #73 | no (re-export) | S | ✅ done |
| J | `UiSystem`/`SteeringSystem` scratch fields | #75 | **YES** (construction) | S | ✅ done |
| K | `ScriptAsset`→`scripting/` + `ScriptRegistry` | #76 | yes (path, bridged) | S | ✅ done |

**9 of 11 done; D skipped (trivial/not worth it); F descoped (risk).** The 4-way analysis's headline —
*the breaking surface is tiny, no item touches an example* — held: the only forker-visible breaks were
J (construction), A (fields), K (module path, re-export-bridged), and the `cell_display_uv` removal (#73).

## The arc (how the session flowed)

1. **Onboarded** from seq-8: confirmed `main` green at v9.5.1, the "clean additive backlog exhausted"
   claim. User ran `/loop 재실행`.
2. **Re-examined the backlog** and found one substantial additive item still open: the **`RenderPlugin`
   render-pass hook**. Shipped it as #71 (v9.6.0) — designed `FrameContext.format` + the dispatch point.
3. **Cleanup loop (#72, v9.6.1):** triaged the 3 remaining LOW-value items. Dropped a `state_machine`
   String-clone "micro-opt" (read the code → zero alloc savings, the write site needs the owned String —
   churn, not a cleanup). Found `register_editable_component`'s wasm doc note already existed. Shipped
   only the real one (`Wander::direction_fn`) + fixed 2 broken doctests + **added the doctest gate** that
   would have caught them.
4. **Declared the autonomous additive backlog truly dry** → asked the user for direction. They chose
   **"scope the v10 breaking pass."**
5. **Scoped v10 via a 4-way parallel read-only analysis** (Theme-3 encapsulation / App-render-update /
   renderer-tilemap-unit-struct / ScriptAsset) → wrote `plans/V10_BREAKING_PASS_PLAN_2026-06-16.md`.
   **Key finding: the breaking surface is tiny — no item has any external/forker call site in
   `examples/`; most architectural value (E/F/G/H/I) is internal-only.** Presented the scope decision.
6. **User greenlit the FULL v10 pass.** Executed PR1–PR4 (the breaking surface: tilemap split,
   encapsulation, unit-struct scratch, ScriptAsset decouple) → PR5 (RenderState extraction, item E).
7. **Before PR6 (item F), flagged a cost/benefit shift**: F's fork-pass-insertion goal is already met by
   the shipped `RenderPlugin`, and F is the only item CI can't verify. **Recommended skipping F.** User
   re-fired the loop (taken as acceptance). Did **item G** (split `update()`) instead.
8. **User ended the loop** ("루프 종료") after G; I wrote the (mid-session) handoff. Then user asked to
   **"finish H with the sprite smoke test."** Did **item H** (SpriteRenderer split) + a visual smoke
   test, completing the v10 arc. F stays descoped.

## Where We're Going (remaining — all need a decision)

**`main` is green + clean at v10.0.0.** What's left:

1. **Item F — split `render()` (839L): DESCOPED, not abandoned.** Its goal ("a forker can't insert a
   render pass without editing `render()`") is met by `RenderPlugin` (#71). What's left is internal
   readability of a *working* function — and F is the ONE item CI cannot verify (no GPU test;
   submit-order / texture-routing / cfg mistakes compile fine but render wrong). **Only revisit with a
   real visual playtest plan** across all render modes (normal/docked/post/lighting/fade/offscreen/
   particles). The macOS screencapture+osascript technique (proven on H this session) can drive it.
2. **`MaterialRenderer` (ShaderMaterial) path is visually unverified** — NO example uses `ShaderMaterial`
   (predates H). H's split of it relies on byte-identical wiring + unit tests. A `shader_material`
   example would close this gap (and is a VISION-style "feature needs an example" item).
3. **v10.0.0 is unpublished.** The CHANGELOG `## 10.0.0` heading is finalized (no "(in progress)"). A
   git tag / `cargo publish` is **on-request only** (tagging lapsed after v4.3.0).
4. **Pre-v10 deferrals still open:** per-locale font auto-select (#5, display-session, spec in seq-3
   handoff); SM/Timeline visual editors (egui-painter, docked cursor-freeze playtest limit).

## Key Decisions (this session)

- **v10 = one accumulating release.** First v10 PR bumped Cargo.toml 9.6.1→10.0.0; subsequent PRs kept
  10.0.0 and appended to a single `## 10.0.0` CHANGELOG section. CLAUDE.md header doc-version bumped per
  PR (v1.6.40 → v1.6.49). H finalized the heading (dropped "(in progress)").
- **Descoped item F** — stopped at the high-risk/low-marginal-value line. RenderPlugin already covers the
  fork need; the internal-readability gain didn't justify a visual-regression risk CI can't guard.
- **`gpu` stays on `App` in RenderState (E)** — deliberate: keeping it disjoint from the grouped renderers
  avoids the destructuring churn that grouping gpu+renderers would force (4-way analysis flagged it).
- **Skipped a non-fix.** The `state_machine::evaluate()` String-clone "micro-opt" was dropped after
  reading the code: the clone only fires on a transition AND the write site (`sm.current`) needs an
  owned String, so returning `&str` saves zero allocs. Don't apply changes that don't change anything.
- **Smoke-tested H, not F.** H is a contained ownership extraction (a layout mismatch panics on startup →
  catchable by running an example); F's whole-orchestration rewrite is far harder to verify visually.
- **Merge authority** was granted for the loop and exercised (PR-based, CI-gated, revertible). One
  AskUserQuestion was correctly blocked by the auto-mode classifier early (the user then granted it).

## Evidence & Data

### main lineage (this session)
`db07194`(handoff) → `25c3c11`(#79 H) → `5aded79`(handoff) → `5d695ab`(#78 G) → `a2a80cd`(#77 E) →
`4c5b6f4`(#76 K) → `0350fb8`(#75 J) → `46ffafb`(#74 ABC) → `113f815`(#73 I) → `4a4d488`(#72 v9.6.1) →
`163b2d0`(#71 v9.6.0) → `1aec558`(seq-8 handoff, v9.5.1 base).

### Per-PR facts (don't re-derive)
- **#71 RenderPlugin:** `trait RenderPlugin { fn record(&mut self, ctx: &mut FrameContext, world: &World, viewport: (u32,u32)); }`. Dispatch = a new "Step 3" in `render()` after the GPU-particle block, before post-process; cross-platform; no-op (byte-identical) when no plugin registered. `FrameContext` gained `pub format: wgpu::TextureFormat` (3 construction sites; all use `gpu.config.format`, incl. offscreen RT since `create_render_target` uses the surface format). +2 tests (726→728).
- **#72:** `Wander.direction_fn: Option<fn(u32, Vec2) -> Vec2>` (fn-pointer keeps `Wander` Clone/Debug); `pub(crate)` fields on Wander mean adding it is non-breaking. Doctests fixed: `App::new()` (was `App::new(Default::default())`) + `use engine_reflect_derive::Reflect`. 728→729.
- **#73 tilemap split:** `tilemap/mod.rs` (data model + re-exports the 10 public names so `lib.rs` is untouched), `autotile.rs` (single+multi-terrain together — share `compute_mask_raw`), `system.rs` (reactive `TilemapSystem`). `cell_display_uv` removed (0 callers; `tilemap` IS a `pub mod`, so technically a public removal → fits the v10 window).
- **#74 encapsulation:** `RenderTarget.{texture,view,sampler,bind_group}` → `pub(crate)` + `texture()/view()/sampler()/bind_group()` accessors; `width/height/clear_color` stay pub. `LightingRenderer.{normal_view,width,height}` → `pub(crate)`. `RenderTarget::new` builds layout via `Texture::bind_group_layout(device)`.
- **#75 unit-struct scratch:** `UiSystem` 6 scratch Vecs (threaded into the 6 `ui/system/*_pass.rs`), `SteeringSystem` 5 scratch Vecs. Both `#[derive(Default)] + new()`. Migration: 14 UiSystem + 8 SteeringSystem sites (examples + tests + doc examples); `X::LABEL` const access untouched.
- **#76 ScriptAsset:** `ScriptRegistry` World resource (`load_script`/`get_script_by_id`/`load_script_inline`); hot-reload chain = `App::load_script` → `AssetServer::watch_path` → `poll_reloads` → forwarder → `ScriptRegistry::reload_path` (recompiles at the SAME AssetId, live handles stay valid). `alloc_id` made `pub(crate)`. `Handle::_marker` made `pub(crate)`.
- **#77 RenderState:** 14 fields moved (sprite/text/post/lighting/fade/gpu-particle renderers + 3 intermediate textures + render_plugins/render_targets + egui_renderer/state/output); 4 destructures in `render.rs` where two RenderState fields are borrowed together; `gpu`+`world` stay on App.
- **#78 update() split:** `update()` is now a 23-line sequencer. `Option<egui::Context>` threads begin → `run_systems` (ref) → `post_systems` (by value, consumed by `end_egui_frame`). Ordering preserved (merge_textures_delta take-before-assign; scene-cmd-after-flush; egui-end-before-flush).
- **#79 SpriteRenderer split:** TextureCache = {white_texture, texture_cache, rt_cache, texture_layout}; MaterialRenderer = {sprite_shader, camera_layout, surface_format, params_layout, mat_instance_*, custom_pipelines, params_buffers, 3 material scratch}. One destructure (`let SpriteRenderer { material, texture_cache, .. } = self`) in compile_material_pipeline. Thin shims keep public method sigs (`load_texture`/`reload_texture`/`register_render_target`).

### Visual smoke test (item H — the only GPU-verified item)
Ran on macOS via `screencapture -x` + `osascript` (raise window) per [[playtest-windowed-examples]]:
- **`basic`** → cyan rotated sprite renders clean (sprite pipeline + `texture_layout` binding + `white_texture` via TextureCache). No panic.
- **`security_camera`** → the "WALL MONITOR" panel correctly displays the offscreen-rendered scene as a textured quad (RenderTarget `rt_cache` + register_render_target path) + scene sprites + HUD text. No panic.
- A `texture_layout` mismatch panics at draw time (wgpu validation), so a clean multi-second render = correct wiring. **MaterialRenderer/ShaderMaterial has no example → unverified visually.**

## Reusable Gotchas (HIGH VALUE — read before next session)

- **Stale mid-edit rustc diagnostics fired 6×** (E0061/E0308/E0423/E0609 in the exact files an agent was
  editing) while the agent's FINAL state was clean. **Every single time, `cargo check --all-targets` was
  green** — they were snapshots from mid-edit, resolved before the agent finished. **ALWAYS independently
  `cargo check`/Gate6 after agent work; trust neither the agent's "green" claim alone NOR the stale
  diagnostics.** Also: agents twice self-reported "cargo fmt PASS" while `cargo fmt --check` actually
  failed (an edit after the last format) — **always run `--check` explicitly** in your re-verify.
- **Agent worktree surprise (PR5 + item H):** long-running general-purpose agents (no `isolation` set)
  did their work in a `.claude/worktrees/agent-*` worktree, NOT the main tree — main tree was clean and
  the changes "missing." Recovery: `git -C <worktree> add -A && commit`, then `git checkout -b <branch>
  && git cherry-pick <sha>` into the main tree, then `git worktree remove --force` + delete the worktree
  branch. **Check `git worktree list` + `git status` if an agent's changes seem absent.**
- **`verify.sh` was committed `100644`** (non-executable) in git all along — `core.fileMode=false` hid it
  locally. `./scripts/verify.sh` fails "permission denied" on a fresh clone. Fixed in #73 via
  `git update-index --chmod=+x` (a plain `chmod`+`git add` does NOT record the bit when
  `core.fileMode=false`). Fallback: run Gate6 as `bash scripts/verify.sh`.
- **CI disk-space flake:** PR3's "Test (native)" failed with `No space left on device` on the runner
  (infra, not code — the test steps never ran; same-commit WASM/Rustdoc/Package passed). `gh run rerun
  <id> --failed` fixed it. **Inspect `gh run view --job <id>` annotations before assuming a code failure.**
- **`gh pr checks <n> --watch | tail` masks the real exit code** (you get tail's 0). Read the actual
  per-check pass/fail lines. For Gate6: `bash verify.sh > log; echo "REAL_EXIT=$?"` then
  `grep 'all checks passed' log` (the in-script `${PIPESTATUS[0]}` won't survive a backgrounded pipe).
- **Doctests are now a CI gate** (#72 added `cargo test --doc` to ci.yml + verify.sh). Broken rustdoc
  examples now FAIL CI — keep doc examples compiling.
- **rust-analyzer phantoms persist** (ColliderHandle E0308, inactive-cfg, unlinked-file) — routine, ignore.
- **Visual smoke test technique (works):** `cargo build --example X` → run the binary in background →
  `sleep` → `osascript ... set frontmost of (first process whose name is "X")` → `screencapture -x out.png`
  → kill → grep the run-log for `panic|validation|wgpu`. Read the PNG to eyeball. A clean run (no panic)
  is itself strong evidence for layout-binding correctness.

### Reusable smoke-test recipe (for F, or any render verification)
```bash
EX=basic   # or security_camera / lit_dungeon (lighting) / a post-process example / etc.
cargo build --example "$EX"
( ./target/debug/examples/"$EX" > /tmp/smoke_$EX.log 2>&1 & echo $! > /tmp/smoke_$EX.pid )
sleep 7
osascript -e "tell application \"System Events\" to set frontmost of (first process whose name is \"$EX\") to true" 2>/dev/null
sleep 1; screencapture -x /tmp/smoke_$EX.png
kill "$(cat /tmp/smoke_$EX.pid)" 2>/dev/null; pkill -f "examples/$EX"
grep -aiE 'panic|validation|error|wgpu' /tmp/smoke_$EX.log   # empty = good
# then Read /tmp/smoke_$EX.png to eyeball
```
For F specifically, run this across: `basic` (normal), a docked-editor session (F2), a `PostProcessConfig`
example, `lit_dungeon` (lighting), a fade transition, `security_camera` (offscreen), and a GPU-particle
example — F's whole point is those passes still compose in the right order/routing.

### Loop mechanics (how the autonomous arc ran)
opus scoped each item inline → ONE background Sonnet impl agent (explicit `model: sonnet`,
[[new-model-subagent-incompat]]) → on completion opus reviewed the diff + re-ran full Gate6 + bumped
version/CHANGELOG/CLAUDE + branch/commit/PR + `gh pr checks --watch` + squash-merge → next item.
Read-only scoping fanned out to 4 concurrent agents. Sequential PRs (not parallel) to avoid
version/CHANGELOG conflicts.

## User Feedback & Preferences (REQUIRED)

- **"재실행" (×4 this session)** — keep executing the backlog autonomously; one PR per item, merge, report.
- **"scope the v10 breaking pass"** → wanted a plan, then greenlit the **full pass**.
- **Re-fired the loop after I flagged F** — taken as acceptance of "skip F, do G+H."
- **"루프 종료"** then **"finish H with the sprite smoke test"** then **"다음 루프 wakeup 종료"** (cancel the
  pending fallback wakeup; I did — no loop re-armed). Then **"/handoff 푸시"**.
- **Standing:** Korean prose to the user, English code/docs/handoff; Sonnet subagents with explicit
  `model:`; no breaking changes beyond the agreed v10 plan without sign-off; never tag unprompted; merge
  authority granted for the loop (re-confirm in a new session).

## Risks & Blockers

- **None.** main green + clean at `db07194` (v10.0.0). All 9 PRs CI-green + squash-merged + branches deleted.

## Open Questions

- Do item **F** (render split) with a full render-mode playtest, or leave it descoped permanently?
- Add a `shader_material` example to visually verify the `MaterialRenderer` path?
- Tag / publish **v10.0.0** (default: no — on request)?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -10       # db07194(handoff) ... 25c3c11(#79 H) ... 163b2d0(#71 v9.6.0)
grep -m1 '^version' Cargo.toml   # 10.0.0
git status -s               # clean
./scripts/verify.sh         # main green (729 lib tests) — exec bit now fixed in git

# Read first
#   plans/V10_BREAKING_PASS_PLAN_2026-06-16.md   (per-item breaking-surface/effort/risk; F/H detail)
#   THIS handoff (seq 9)
#   docs/CHANGELOG.md  → the finalized ## 10.0.0 section

# Next action — needs a DECISION (v10 arc is COMPLETE as-scoped):
#   (a) Leave it — v10 done (F descoped); pick a new VISION feature + example, or pause.
#   (b) Item F (split render()) — ONLY with a full visual playtest across all render modes
#       (macOS screencapture+osascript technique works — see Gotchas).
#   (c) Add a shader_material example to verify the MaterialRenderer path.
#   (d) Tag/publish v10.0.0 (on request only).
#   Standing: Korean prose / English artifacts; Sonnet subagents w/ explicit model:;
#   re-verify Gate6 independently after agent work (stale diagnostics fire constantly);
#   merge authority granted for the loop (re-confirm in a new session).
```

## Session Status
COMPLETE — v10 breaking arc done (9 PRs this session: #71–#79; v9.5.1 → v10.0.0). Item F descoped, H
smoke-tested. main green + clean at `db07194`. Loop ended; no wakeup armed.
