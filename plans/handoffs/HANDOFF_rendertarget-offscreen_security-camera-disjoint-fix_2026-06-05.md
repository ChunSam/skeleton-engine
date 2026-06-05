# skeleton-engine: Security-camera example (RenderTarget/OffscreenCamera) + per-target offscreen render fix

**Date:** 2026-06-05
**Status:** COMPLETED (committed `cbbdfbd`, pushed `e7203d8..cbbdfbd`, CI `27012114491` **completed success** 4m56s)
**Bead(s):** none (`bd` not installed in this environment)
**Epic:** VISION feature+example loop — dogfood shipped-but-never-in-a-game subsystems
**Chain:** `rendertarget-offscreen` seq `1`
**Parent:** none — first in chain
**Prior chain:** none — first in chain

---

## Related Handoffs

This session *opened* by executing the paired PLAN from the **`physics-joints`** chain
(`PLAN_physics-joints_crane-rotation-sync_2026-06-05.md`, seq 1), which defined "the next dogfooding
cycle" and recommended **RenderTarget/OffscreenCamera** as the candidate. Per the precedent that chain
itself set (it onboarded from `blendtree1d-locomotion` yet started a fresh chain because the feature
stream was new), this RenderTarget work is a **new chain seq 1**, with the physics-joints files listed
here as siblings (reference only, not parents):

- `PLAN_physics-joints_crane-rotation-sync_2026-06-05.md` — the plan this session executed (Phases 1–3).
  Its candidate recommendation (RenderTarget) and "read the real API surface first" warning were both
  followed and both paid off.
- `HANDOFF_physics-joints_crane-rotation-sync_2026-06-05.md` — the prior session's data (crane
  wrecking-ball + `PhysicsSystem` rotation sync, commit `338eb08`). Same dogfooding epic, separate
  feature stream.

## Reference Documents

- `docs/VISION.md` — the feature+playable-example loop (a feature isn't done until a small playable
  example exercises it in real play; fix awkward API/bugs before release; the example is the acceptance
  test).
- `docs/NEXT_WORK.md` — candidate list; this session adds **candidate K (Security camera)** and trims
  `RenderTarget`/`OffscreenCamera` from the "never-in-a-game" remaining list.
- `docs/HANDOFF.md` — per-phase dev history; gained a `## 2026-06-05 — Security-camera example +
  offscreen-render fix` entry.
- `docs/CHANGELOG.md` — `## 2.0.0` Added (security_camera example) + Fixed (offscreen render bug,
  split_screen self-capture).
- `CLAUDE.md` — module map: added a RenderTarget/OffscreenCamera row noting per-target submission.
- `src/camera.rs` — the **top-left-anchored, Y-down** coordinate convention (the lesson behind the first
  example draft rendering off-screen).
- Memory: `playtest-windowed-examples.md` (new this session — how to run a windowed-example playtest
  yourself on macOS via osascript window bounds + caffeinate + screencapture + synthetic key input).

## The Goal

Per VISION: dogfood the `RenderTarget` / `OffscreenCamera` subsystem — which shipped with **two tech
demos** (`minimap`, `split_screen`) but **zero playable-game usage** — by building ONE small playable
artifact that exercises it in real play, and fixing whatever real play needs that the demos didn't. The
end state: a focused playable **security-camera stealth puzzle** where a guard patrols a room that is
*entirely offscreen* and visible only on a wall monitor (an `OffscreenCamera` renders the guard room
into a `RenderTarget` sampled by a `Sprite`), plus the engine fix the disjoint-region example forced out
(offscreen targets were rendering with the **main** camera, not their own).

## Where We Are

**All work complete, verified, playtest-passed 11/11, committed (`cbbdfbd`), pushed, and CI is green.**

- **New example `examples/security_camera.rs`** (~310 lines, top-level so auto-discovered; **cross-platform**
  — no native-only deps, unlike the physics/lighting examples). The player crosses a visible corridor
  toward an exit; a guard patrols an offscreen room shown live on a wall monitor; time the doorway
  crossing by the guard's position on the monitor. Detection → reset to start (`caught` counter +1, the
  doorway tile flashes red); reach the exit → `ESCAPED!` banner; `R` replays, `Esc` quits.
- **Engine fix — offscreen targets rendered with the MAIN camera** (`src/app/render.rs`). The sprite
  renderer's camera uniform is a single shared buffer (`camera_buf`) written via `queue.write_buffer`.
  The offscreen pass and the main pass were recorded into **one command submission**, and within a single
  submit only the **last** write to that buffer takes effect before the command buffer runs — so every
  offscreen target was drawn with the (later-written) main camera's `view_proj`. Fixed: each offscreen
  target now renders into its **own** `CommandEncoder` and is submitted immediately, pairing its camera
  write with its own draws. +20/-2 lines (one `create_command_encoder` + `queue.submit` + a 6-line
  explanatory comment).
- **`split_screen` self-capture crash fixed** (`examples/split_screen.rs`, 2 lines). It used
  `layer_mask: 0` (render all layers), so its RT *display* sprites were drawn into the targets they
  sample — a wgpu validation error (texture used as both color attachment and sampled resource in one
  render pass). It crashed on **frame 2** (frame 1 survived only because the targets weren't registered
  yet). Pre-existing — crashes on the committed `e7203d8` code too (confirmed by reverting the engine
  fix). Fixed by masking the display sprites out (`layer_mask: 1 << 0`), matching `minimap`.
- **The RenderTarget API was already complete** (the session's central scouting finding): both
  `examples/minimap.rs` and `examples/split_screen.rs` already exercise `create_render_target` +
  `OffscreenCamera { target, camera, layer_mask }` + RT-sampled-as-`Sprite` + multiple targets + live
  per-frame feeds. The gap was **"no playable game"** + a latent render bug only a disjoint offscreen
  region could expose — NOT a missing API (the physics-joints chain's mis-scout lesson, avoided here).
- **No unit tests added** — the fix is a GPU command-submission-ordering bug, not unit-testable without a
  GPU (CI has none). Validated by the native run + 11/11 playtest, the same bar lighting/blend used.
- **`docs` updated** (4 files): CHANGELOG (Added example, Fixed 2), NEXT_WORK (candidate K + trimmed RT
  from remaining), HANDOFF, CLAUDE.md (module-map row).
- **Verification:** `./scripts/verify.sh` green (fmt, clippy `-D warnings`, wasm lib+bins build,
  `test --all-targets`, rustdoc `-D warnings`); native `cargo run --example security_camera` renders the
  guard room on the monitor; `minimap` + `split_screen` run without crashing under the fix; `rust-survivors`
  path-patch `cargo check --workspace` clean (uses 0 RenderTarget; render-path change is internal, no
  public API change); 11/11 HTML playtest checklist (run by the agent via synthetic input).
- **Commit `cbbdfbd`** (7 files, +554/-5), pushed `e7203d8..cbbdfbd`. CI run `27012114491` **completed
  success** in 4m56s.
- **`bd` (beads) unavailable** — chains tracked purely by `HANDOFF_*`/`PLAN_*` filenames + headers.

## What We Tried (Chronological)

1. **(early) Onboarded** from the physics-joints PLAN (paste prompt told me to execute its Phase 1).
   Confirmed green start: `git status` clean, tip `e7203d8`; **CI success on both `e7203d8`
   (run 27002938729) and `338eb08` (run 27002241512)**; `./scripts/verify.sh` exit 0.
2. **(early) Scouted the REAL RenderTarget API surface** (the anti-mis-scout step the plan demanded).
   Read `examples/minimap.rs` + grepped `src/renderer/` + `src/app/`. **Found TWO demos, not one** — the
   plan only mentioned `minimap.rs`, but `grep -rln "OffscreenCamera"` also surfaced
   `examples/split_screen.rs`. Read both. Read `src/renderer/render_target.rs`, `src/app/render.rs`
   (offscreen pass 222-311), `src/app/assets.rs` (`create_render_target`), `src/components.rs`
   (`OffscreenCamera`). Learned: the API is feature-complete (create + offscreen cam + `layer_mask` +
   RT-as-sprite + multi-target + live feeds), and `src/ui/` has **no** RenderTarget support (grep empty).
3. **(early) Found the screen-overlay path can sample RTs.** `bind_group_for_texture_key`
   (`src/renderer/sprite/textures.rs:89`) checks `rt_cache` FIRST, and the screen-space `DrawImage`/
   `UiImageQueue` overlay resolves through the same function → a HUD-anchored RT is achievable without
   engine change. This shrank the "no HUD placement" gap and reframed the candidate honestly to the user.
4. **(early) Recommended RenderTarget** via AskUserQuestion (RenderTarget recommended, Timeline +
   networking as alternatives). **User picked RenderTarget.**
5. **(mid) `/grill-me`** to lock scope (3 AskUserQuestion rounds + closure packet). Locked: security-camera
   stealth puzzle · engine-fix bar = **fix only the gap the example genuinely hits (zero change OK)** ·
   compat **same as last session** (wasm lib+bins stays green; example may be cross-platform) · fail =
   **detection → reset**, win = **reach exit** · monitor role = **guard's room offscreen, knowable only
   via the monitor** · threat geometry = **corridor crossing timed to the guard's patrol** (proximity
   detection). Produced `grill_decision_packet` (plan_allowed: true).
6. **(mid) Wrote `security_camera.rs` v1** — using a **center-origin, Y-up** coordinate assumption.
   Native run + screenshot: the window was nearly empty (HUD only) and the monitor was black.
7. **(mid) Diagnosed v1: wrong coordinate convention.** Read `src/camera.rs` — the camera is **top-left
   anchored, Y-down** (`position` = viewport top-left; visible rect `[pos, pos + size/zoom]`; `view_proj`
   = `ortho(left, right, bottom, top)` with `top = pos.y`). So a camera at `(0,0)` shows world
   `[0,1000]×[0,720]` and my origin-centered scene fell off the top-left. **Rewrote v2** with top-left/
   Y-down coordinates; put the guard room offscreen to the right (`x≈1400`, outside the main `[0,1000]`).
8. **(mid) v2 main scene rendered correctly** (cyan player, corridor, doorway, green exit) **but the
   monitor feed showed the CORRIDOR, not the guard room** — the green exit was visible in the feed, which
   the offscreen camera at `(1100,210)` cannot see.
9. **(mid) Diagnosed by EVIDENCE, not theory** (carried lesson). (a) Recolored the guard-room backdrop
   **magenta** → the feed was unchanged (still corridor-gray + green) → the feed is NOT the guard room.
   (b) Added a temporary `eprintln` in the offscreen loop → it printed `cam.pos=(1100,210) zoom=0.64
   rt=384x192 mask=1` → the offscreen camera **was** passed correctly. (c) Read the rest of
   `src/renderer/sprite.rs::render` — the sprite draws bind `self.camera_bind_group` (→ `camera_buf`),
   written once at line 279. **Conclusion:** the offscreen + main passes share `camera_buf` in one submit
   → last write (main camera) wins for both → offscreen renders the main view.
10. **(mid) Fixed: per-target submission.** Each offscreen target gets its own `CommandEncoder`, submitted
    immediately. Rebuilt with the magenta test still in: **the feed turned magenta** (guard room) → bug
    confirmed + fixed. Reverted the magenta backdrop.
11. **(mid) `split_screen` crashed** with a wgpu validation error (self-capture). **Tested the committed
    code** (`git stash` the render.rs fix) → it crashes the SAME way → **pre-existing, not my regression.**
    Fixed `split_screen`'s `layer_mask: 0 → 1 << 0` (exclude its display sprites from the offscreen pass).
    Restored my fix (`git stash pop`).
12. **(mid) Sanity-checked `minimap`** (the third RT consumer, `mask 1<<0`, no self-capture) under the fix
    → ran clean, no crash. `cargo build` + `cargo fmt` + clippy clean. `./scripts/verify.sh` green.
13. **(late) Native run + screenshot self-check** — the monitor shows the guard room (red guard + yellow
    door stripe + crates); the corridor/player/exit render in the main view; the guard is NOT visible in
    the main view (offscreen). Killed the run.
14. **(late) rust-survivors cross-repo check.** Path-patch `cargo check --workspace` → **clean**
    (`skeleton-engine v2.0.0`, 2.72s); restored `Cargo.lock`. rust-survivors uses 0 RenderTarget and the
    render-path change is internal with no public API change → behaviorally inert there.
15. **(late) Built the HTML playtest checklist** (`/tmp/security_camera_test.html`, 11 items, 4 groups,
    localStorage + markdown export — same template as the crane session). Delivered it + a final screenshot.
16. **(late) User said "체크리스트 실행해줘"** (run the checklist yourself). Ran it via synthesized input:
    `osascript` window-bounds query by PID + `caffeinate` (wake display) + `screencapture -R` + `key down/
    up "d"` (held movement), `keystroke "r"`, `key code 53` (Esc). **11/11 ✅**, no crash across the run.
17. **(late) Committed `cbbdfbd`**, pushed `e7203d8..cbbdfbd`, watched CI in background → run `27012114491`
    **completed success** 4m56s. Wrote memory `playtest-windowed-examples`.

## Key Decisions

- **New chain `rendertarget-offscreen` seq 1, not a continuation of `physics-joints`.** I executed the
  physics-joints PLAN, but RenderTarget is a fresh feature stream — mirrors that chain's own precedent of
  starting new despite onboarding from a prior chain. The physics-joints files are siblings, not parents.
- **Candidate = RenderTarget/OffscreenCamera** (user-chosen over Timeline / networking). Honestly reframed
  before recommending: the API is complete (2 demos), so the gap is "no playable game" + whatever real
  play surfaces — NOT a missing API. This avoided repeating the joints mis-scout.
- **Engine-fix bar = fix only the gap the example genuinely hits; zero change acceptable** (user-chosen).
  This turned out to surface a *real* bug (offscreen-uses-main-camera), so the cycle was not zero-change —
  but the bar kept me from speculatively widening the RT surface (no HUD helper, resize, or UI-widget RT).
- **Engine fix = per-target submission, not a second camera buffer** (decided from the evidence). The
  shared `camera_buf` + single-submit is the root cause; submitting each offscreen target separately pairs
  its camera write with its draws and scales to N targets. A per-target dedicated camera buffer would also
  work but needs threading a second bind group through `SpriteRenderer::render`; the submit approach is
  smaller and local to `src/app/render.rs`.
- **Fixed `split_screen` too** (mild scope expansion, judged good stewardship). It's a pre-existing crash
  in the same subsystem I was fixing; the fix is 2 lines and matches `minimap`'s pattern; leaving a
  crashing demo contradicts VISION ("examples are the acceptance test"). Confirmed it was NOT my
  regression before touching it. Did **not** fix `split_screen`'s separate center-origin *layout* issue
  (out of scope, cosmetic, pre-existing).
- **Self-capture is a user-side concern, not an engine guard.** The engine renders whatever layers the
  `OffscreenCamera` mask selects; an RT-display sprite must be excluded via `layer_mask`/layer so it isn't
  drawn into the target it samples. `minimap`/`security_camera` do this; the old `split_screen` didn't.
- **No art / colored sprites** sized in pixels (like `crane_wrecking_ball`/`skeletal_puppet`). Keeps the
  change scoped; no asset pipeline.
- **Agent ran the playtest** (user delegated it). Used macOS synthetic input — see memory
  `playtest-windowed-examples`. This is a reusable capability for the dogfooding loop.
- **Diagnose by screenshot/recolor/print, not theory** (carried lesson, and it was decisive here): a
  magenta recolor test + an `eprintln` pinned the bug to the shared-buffer/single-submit interaction
  rather than the (correct) camera swap.

## Evidence & Data

### Commit (this session)

| Hash | Summary | Files | +/- |
| --- | --- | --- | --- |
| `cbbdfbd` | feat(render): security_camera example + per-target offscreen render fix | 7 | +554 / -5 |

### Diffstat (`git show --stat cbbdfbd`)

```
CLAUDE.md                     |   1 +
docs/CHANGELOG.md             |  24 ++++
docs/HANDOFF.md               |  48 +++++++
docs/NEXT_WORK.md             |  31 ++++-
examples/security_camera.rs   | ~310 +++++++++++++++++++++++++++++  (new)
examples/split_screen.rs      |   4 +-
src/app/render.rs             |  20 ++-
7 files changed, 554 insertions(+), 5 deletions(-)
```

### Verification (final, all green)

| Check | Result |
| --- | --- |
| `cargo fmt --check` | clean (after `cargo fmt` re-wrapped the new `create_command_encoder` call) |
| `cargo clippy --all-targets -- -D warnings` | 0 warnings |
| `cargo build --target wasm32-unknown-unknown` (lib+bins) | clean (example is top-level, not in this gate) |
| `cargo test --all-targets` | pass (no new tests — GPU-submission fix isn't unit-testable) |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps` | clean |
| native `cargo run --example security_camera` | renders guard room on monitor; corridor/player/exit in main view |
| native `cargo run --example split_screen` | no crash under fix (committed code crashes frame 2) |
| native `cargo run --example minimap` | no crash under fix |
| `rust-survivors` `cargo check --workspace` (path-patched) | clean (`skeleton-engine v2.0.0`, 2.72s) |
| user/agent playtest | **11/11** checklist ✅ |
| CI `27012114491` | **completed success** 4m56s |

### Grill scope-lock (decision packet, plan_allowed: true)

| Question | Chosen |
| --- | --- |
| candidate | **RenderTarget/OffscreenCamera** (over Timeline / networking) |
| example concept | **security-camera stealth** (over mirror/portal / single reflection) |
| engine-fix bar | **fix only the gap the example genuinely hits** (zero change accepted) |
| compat/wasm | **same as last session** (native run gate; wasm lib+bins stays green) |
| fail/win loop | **detection → reset + reach exit → win + R replay** (over goal-only) |
| monitor role | **guard room offscreen, knowable only via the monitor** (over planning-aid) |
| threat geometry | **corridor crossing timed to the patrol** (over offscreen direct-cross / vision cone) |

### Playtest (HTML checklist, 11 items, 4 groups) — 11/11 ✅, agent-run, no failures, no crash

A (boot/render: A1 corridor+player+exit+doorway, A2 HUD + monitor label),
B (monitor feed — engine-fix core: B1 monitor shows guard room not corridor, B2 guard moves live, B3
guard only on monitor never in main view),
C (play loop: C1 safe cross when guard away, C2 caught at doorway while watched → reset + counter, C3
reach exit → ESCAPED banner),
D (controls: D1 WASD/held-key move, D2 R replay resets, D3 Esc quits).
Observed during the run: `caught` climbed 0→2 (C2), then `ESCAPED!` (C3), then R reset to `caught: 0`
and player-at-start (D2), then Esc closed the process (D3).

### Example tuning constants (`examples/security_camera.rs`)

```
WINDOW 1000×720 · world = pixels · camera TOP-LEFT anchored, Y-DOWN (main cam (0,0) zoom 1 → x[0,1000] y[0,720])
Corridor (visible): player x∈[40,962] y∈[502,600], PLAYER_START (70,540), PLAYER_SPEED 240, EXIT_X 905
Doorway crossing: DOOR_X 500, THRESH_HALF 55 (player in doorway when |x-500|≤55)
Guard room (OFFSCREEN, x>1000): ROOM_DOOR_X 1400, GUARD_Y 360, patrol x∈[1180,1620], GUARD_SPEED 200,
  DOOR_WATCH_HALF 100 (guard watches door when |x-1400|≤100)
OffscreenCamera: target "camfeed", pos (1100,210), zoom 0.64 → frames x[1100,1700] y[210,510]; layer_mask 1<<0
Monitor: bezel+feed at (500,150), feed 420×210 (RT 384×192, 2:1), RenderLayer(1) (excluded from offscreen)
Detection: caught when in_doorway && door_watched → caught_flash 1.1s, player frozen+reset to start, counter+1
```

### The engine fix (before → after, `src/app/render.rs` offscreen loop)

```rust
// BEFORE: offscreen clear + sprite render recorded into the SHARED `enc`; one submit at frame end.
//         camera_buf is written (offscreen cam) then later overwritten (main cam) → only main wins.
let _pass = enc.begin_render_pass(/* clear rt_view */);
sr.render(FrameContext { encoder: &mut enc, view: rt_view, .. }, &world, rt_w, rt_h, layer_mask);

// AFTER: each target renders into its OWN encoder and submits immediately,
//        so its camera write pairs with its own draws.
let mut oenc = gpu.device.create_command_encoder(&Descriptor { label: "offscreen encoder" });
let _pass = oenc.begin_render_pass(/* clear rt_view */);
sr.render(FrameContext { encoder: &mut oenc, view: rt_view, .. }, &world, rt_w, rt_h, layer_mask);
gpu.queue.submit(std::iter::once(oenc.finish()));   // flush this target's camera write + draws
```

## Code Analysis

- **`App::render`** (`src/app/render.rs`): the offscreen pass (≈222-311) queries all `OffscreenCamera`
  entities, and per target: saves the main `Camera` resource, inserts the offscreen camera, renders, then
  restores. The camera swap was always correct (proven by `eprintln`); the bug was downstream in GPU
  submission ordering. The main pass (lighting/post/final) still uses the shared `enc` submitted at frame
  end — unchanged.
- **`SpriteRenderer::render`** (`src/renderer/sprite.rs:259`): reads `world.resource::<Camera>()`, computes
  `view_proj` + `visible_rect` from it (line 274-283), writes `camera_buf` once (line 279), culls to the
  camera's `visible_rect`, sorts, then draws — each sprite-run binds `self.camera_bind_group` (line 641),
  which points at `camera_buf`. The single shared `camera_buf` + single submit is the root cause: GPU
  applies all `write_buffer`s before the command buffer runs, so the last write wins for every pass in the
  submit.
- **`bind_group_for_texture_key`** (`src/renderer/sprite/textures.rs:89`): checks `rt_cache` FIRST, then
  `texture_cache`, else white fallback. Used by BOTH world sprites and the `DrawImage`/`UiImageQueue`
  screen overlay — so an RT can be sampled by a world `Sprite` OR a screen-space `DrawImage` (no engine
  change needed for HUD placement).
- **`layer_matches_mask`** (`src/renderer/sprite/sort.rs:88`): `mask == 0` → all layers; else
  `(mask >> layer.clamp(0,31)) & 1`. So negative layers (e.g. bg `RenderLayer(-1)`) and no-RenderLayer
  entities (default 0) both map to bit 0; `mask = 1<<0` includes them and excludes `RenderLayer(20)` (the
  split_screen display sprites). This is how the split_screen fix works.
- **`Camera`** (`src/camera.rs`): **top-left anchored, Y-down.** `position` = viewport top-left world px;
  `visible_rect(w,h) = (pos, pos + (w/zoom, h/zoom))`; `view_proj = ortho(pos.x, pos.x+w/zoom,
  pos.y+h/zoom, pos.y, -1, 1)`. To center on an entity you must offset by half the viewport. The two
  demos and the first example draft were loose about this; `security_camera` v2 uses it correctly.
- **`OffscreenCamera`** (`src/components.rs`): `{ target: String, camera: Camera, layer_mask: u32 }`. The
  per-frame `camera` can be mutated via `get_mut` to pan/follow (split_screen does this); security_camera
  leaves it static.
- **`RenderTarget`** (`src/renderer/render_target.rs`): wraps a wgpu texture (RENDER_ATTACHMENT |
  TEXTURE_BINDING), view, sampler (Nearest/ClampToEdge), and an `Arc<BindGroup>`. Fixed size at creation;
  no resize API. Offscreen clear color hardcoded black (`render.rs`).

## Files Changed

### Source — renderer
- `src/app/render.rs` — offscreen pass: each `OffscreenCamera` target now renders into its own
  `CommandEncoder` and is submitted immediately (was: shared `enc` + single end-of-frame submit). +20/-2,
  incl. a 6-line comment explaining the shared-`camera_buf` / single-submit hazard.

### Examples
- `examples/security_camera.rs` — NEW (~310 lines). The playable acceptance test: corridor crossing timed
  to an offscreen guard seen only on a wall monitor; detection→reset, exit→win, `R` replay, `Esc` quit.
  Cross-platform (no native-only deps). Top-left/Y-down coordinates; guard room at `x≈1400` (offscreen).
- `examples/split_screen.rs` — `layer_mask: 0 → 1 << 0` on both `OffscreenCamera`s (exclude the RT display
  sprites from the offscreen pass; fixes a pre-existing frame-2 self-capture crash).

### Docs
- `docs/CHANGELOG.md` — `## 2.0.0` Added (security_camera example), Fixed (offscreen render bug,
  split_screen self-capture).
- `docs/NEXT_WORK.md` — candidate **K** section; trimmed `RenderTarget`/`OffscreenCamera` from the
  never-in-a-game list; noted deferred RT items (HUD helper, resize/clear-color, UI-widget RT) +
  split_screen's separate layout issue.
- `docs/HANDOFF.md` — `## 2026-06-05 — Security-camera example + offscreen-render fix` entry.
- `CLAUDE.md` — module-map row for RenderTarget/OffscreenCamera (per-target submission; exclude
  RT-display sprites via `layer_mask`; example `security_camera`).

### Memory
- `~/.claude/.../memory/playtest-windowed-examples.md` — NEW. How to run a windowed-example playtest
  yourself on macOS (osascript window bounds by PID, `caffeinate` to wake the display, `screencapture -R`,
  synthetic `key down/up`), + diagnose-by-evidence reminder. Indexed in `MEMORY.md`.

## User Feedback & Preferences (REQUIRED — never omit)

- **Works in Korean; wants conversational replies in Korean.** Handoff/docs stay English per the
  doc-language rule.
- **Chose the recommended candidate** (RenderTarget) this round. Calibration from the chain: the user is
  not predictable across rounds — present bounded trade-offs and let them pick; recommend honestly.
- **"체크리스트 실행해줘" — delegated the playtest to the agent.** Contrast prior sessions where the user
  ran `! cargo run` themselves and pasted results. This time they wanted the agent to run it. → keep the
  ability to drive windowed examples via synthetic input (memory `playtest-windowed-examples`), and keep
  delivering the HTML checklist + screenshot regardless.
- **Consistently prefers the engine fix over a workaround** (carried across the chain). Here the example
  forced out a real offscreen-render bug and I fixed the engine, not the example.
- **Direct-to-main commit + push is the established norm** (documented across the chain). Sign-off this
  round came via the agent's own 11/11 playtest after the user delegated it; committed/pushed to main.
- **Likes the diagnose-by-evidence discipline** (implicit; it resolved this session's central bug fast).

## Where We're Going

1. **(next PLAN's job)** Pick the next never-in-a-game dogfooding candidate from `docs/NEXT_WORK.md`:
   **`Timeline`/cutscene** (recommended — self-contained, unit-tested, no example) or **networking**
   (least self-contained, lowest value-per-effort — don't pick first). Run the same loop: confirm green →
   read the REAL API surface first (grep `src/timeline.rs` + any `Timeline` example refs) → recommend +
   `/grill-me` → implement engine + a small playable example → HTML-checklist playtest + screenshot →
   verify.sh + rust-survivors check → single commit/push → confirm CI. **See the paired PLAN file.**
2. **Verify the candidate's API surface before planning engine work** — this chain's standing lesson
   (joints/RenderTarget both already existed). For Timeline, `src/timeline.rs` exports
   `Timeline/Track/Keyframe/Lerp/TimelineSystem` with unit tests but no example — confirm by reading it.
3. **Optional RenderTarget follow-ups** (only if a future example needs them, per the deferred items): a
   screen-anchored/HUD RT helper (a `ViewportSystem`), RT resize-on-viewport, per-target clear color,
   retained-UI-widget (`src/ui/`) RT support, and `split_screen`'s center-origin layout cleanup. None
   blocking; do not widen speculatively (VISION anti-goal).

## Risks & Blockers

- **Low.** verify.sh green; native run + 11/11 playtest; minimap/split_screen no longer crash;
  rust-survivors clean; CI green. The render-path change is internal (private `App::render`), no public
  API change.
- **Behavior-change caveat:** any in-engine consumer that *relied* on an offscreen target rendering the
  main camera's view (i.e. depended on the bug) would now see the offscreen camera honored. That's the
  correct behavior and no in-repo consumer relied on it; the demos are improved by it. Documented in
  CHANGELOG.
- **Per-target extra submits:** the fix adds one `queue.submit` per offscreen camera per frame. Fine for
  the typical small number of RTs; a scene with many offscreen cameras would pay more submits (acceptable,
  and unavoidable without per-target camera buffers).
- **GPU-only validation:** the fix can't be unit-tested (no GPU in CI). Regression guard is the native run
  + playtest; a future contributor must run `security_camera` to catch a re-break.

## Open Questions

- **Screen-anchored RT display** (deferred): examples place the monitor in world space; there's no
  HUD/viewport helper. The screen-space `DrawImage` overlay *can* sample an RT (confirmed), so it's
  achievable today without engine change — but no ergonomic helper exists. Owner: a future example that
  needs a screen-locked RT. Default: world-space sprite. Consequence if wrong: HUD RT placement stays
  manual.
- **RT resize / per-target clear color** (deferred): fixed size + black clear. Owner: a future full-window
  mirror/portal example. Consequence: a window-sized RT would be letterboxed / wrong-bg.
- **Retained UI widget RT** (deferred): `src/ui/` widgets can't sample an RT (the immediate overlay can).
  Owner: a future UI-heavy RT example.
- **`split_screen` layout** (deferred): its on-screen placement predates the top-left/Y-down convention
  and looks off (now that it doesn't crash). Separate cosmetic cleanup.
- Carried from prior chains (not this session's concern): distance-joint-is-a-spring (no `add_fixed_joint`);
  should screen-space `DrawText` ever be post-processed?

## Quick Start for Next Session

```bash
# Restore context
cat plans/handoffs/HANDOFF_rendertarget-offscreen_security-camera-disjoint-fix_2026-06-05.md
cat plans/handoffs/PLAN_rendertarget-offscreen_security-camera-disjoint-fix_2026-06-05.md   # the paired plan

# Confirm state
git log --oneline -3            # expect cbbdfbd at tip (+ the session: commit this skill will add)
git status -s                   # expect clean
gh run list --branch main --limit 3   # confirm CI green on cbbdfbd (run 27012114491)
./scripts/verify.sh             # 5 checks, expect exit 0

# Next-candidate selection — READ THE REAL SURFACE FIRST (chain lesson)
sed -n '1,170p' src/timeline.rs           # Timeline/Track/Keyframe/Lerp/TimelineSystem (unit-tested, no example)
grep -rln "Timeline" examples/            # confirm there is NO Timeline example
sed -n '168,200p' docs/NEXT_WORK.md       # remaining candidates (Timeline, networking) + candidate K entry
cat docs/VISION.md                        # the feature+example loop

# This session's deliverables (reference patterns)
sed -n '1,40p' examples/security_camera.rs               # top-left/Y-down coords + RT-as-monitor idiom
git --no-pager show cbbdfbd -- src/app/render.rs         # the per-target-submit offscreen fix
cat ~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/playtest-windowed-examples.md

# Next action: pick the next dogfooding candidate (recommend Timeline/cutscene),
#   read src/timeline.rs to confirm the surface, /grill-me to lock scope, then implement engine + example.
```
