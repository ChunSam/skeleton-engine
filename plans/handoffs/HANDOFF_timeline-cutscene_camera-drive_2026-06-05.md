# skeleton-engine: Timeline cutscene example + `CameraTarget` camera-drive (Timeline, candidate L)

**Date:** 2026-06-05
**Status:** COMPLETED (committed `8049dfa`, pushed `3828cf5..8049dfa`, CI `27017341543` **completed success**)
**Bead(s):** none (`bd` not installed in this environment)
**Epic:** VISION feature+example loop — dogfood shipped-but-never-in-a-game subsystems
**Chain:** `timeline-cutscene` seq `1`
**Parent:** none — first in chain
**Prior chain:** none — first in chain

---

## Related Handoffs

This session *opened* by executing the paired PLAN from the **`rendertarget-offscreen`** chain
(`PLAN_rendertarget-offscreen_security-camera-disjoint-fix_2026-06-05.md`, seq 1), which defined "the
next dogfooding cycle" and recommended **`Timeline`/cutscene** as the candidate. Per the precedent that
chain itself set (it executed the `physics-joints` PLAN yet started a fresh chain because the feature
stream was new), this Timeline work is a **new chain seq 1**, with the rendertarget-offscreen files
listed here as siblings (reference only, not parents):

- `PLAN_rendertarget-offscreen_security-camera-disjoint-fix_2026-06-05.md` — the plan this session
  executed (Phases 1–3). Its candidate recommendation (Timeline) and "read the real API surface first"
  warning were both followed and both paid off.
- `HANDOFF_rendertarget-offscreen_security-camera-disjoint-fix_2026-06-05.md` — the prior session's data
  (security_camera + per-target offscreen render fix, commit `cbbdfbd`). Same dogfooding epic, separate
  feature stream. It is also the source of the `playtest-windowed-examples` memory used heavily here.

## Reference Documents

- `docs/VISION.md` — the feature+playable-example loop (a feature isn't done until a small playable
  example exercises it in real play; fix awkward API/bugs before release; the example is the acceptance
  test).
- `docs/NEXT_WORK.md` — candidate list; this session adds **candidate L (Timeline cutscene)** and trims
  `Timeline`/cutscene from the "never-in-a-game" remaining list (leaving **only networking**).
- `docs/HANDOFF.md` — per-phase dev history; gained a `## 2026-06-05 — Timeline cutscene example +
  CameraTarget camera-drive` entry.
- `docs/CHANGELOG.md` — `## 2.0.0` Added (timeline_cutscene example + `CameraTarget`/`Timeline::zoom`).
- `CLAUDE.md` — module map: added a `Timeline`/`CameraTarget` row.
- `src/camera.rs` — the **top-left-anchored, Y-down** coordinate convention (carried camera gotcha).
- Memory: `playtest-windowed-examples.md` — how to run a windowed-example playtest yourself on macOS via
  osascript window bounds + caffeinate + screencapture + synthetic key input. Used + extended this session
  (the `key code 49` = Space discovery; the eprintln evidence trace).

## The Goal

Per VISION: dogfood the `Timeline` keyframe/cutscene subsystem — which shipped with **17 unit tests** but
**zero playable-game usage** — by building ONE small playable artifact that exercises it in real play, and
fixing whatever real play needs that the unit tests didn't. The end state: a focused, **triggerable
cutscene** (press into a rune → camera pans/zooms onto a sealed gate, gate panels slide apart, a screen
overlay fades; Space skips; control returns on finish; cross the open gate to win), plus the one genuine
engine gap a real cutscene forces out — the **camera could not be Timeline-driven** because `Camera` is a
Resource, not an entity's `Transform`.

## Where We Are

**All work complete, verified, playtest-passed 12/12, committed (`8049dfa`), pushed, and CI is green.**

- **Engine addition — `CameraTarget` marker + `Timeline::zoom` track** (`src/timeline.rs`, +138). A
  zero-size `#[derive(Debug, Clone, Copy, Default)] pub struct CameraTarget;` marker; a new
  `pub zoom: Track<f32>` field on `Timeline` (initialised `Track::new()` in `Timeline::new`); and a branch
  at the top of `TimelineSystem::run`'s per-entity loop: `if world.get::<CameraTarget>(entity).is_some()`
  → sample `position`→`Camera::position`, `zoom`→`Camera::zoom` via `world.resource_mut::<Camera>()`,
  `add_component(tl)` back, then `continue` (skips the Transform/Sprite writes). The entity is a virtual
  camera rig with no `Transform`/`Sprite` required.
- **Additive / inert by default.** Ordinary timelines are untouched: the `zoom` track is empty (→ `sample`
  returns `None` → no-op) and the camera branch only fires for marker-tagged entities. Re-exported as
  `engine::CameraTarget` (`src/lib.rs` line 116: `pub use timeline::{CameraTarget, Keyframe, Lerp,
  Timeline, TimelineSystem, Track};`).
- **4 new unit tests** in `src/timeline.rs` (`#[cfg(test)] mod tests`, imports added:
  `crate::camera::Camera`, `crate::components::Transform`, `crate::ecs::{System, World}`):
  `camera_target_timeline_drives_camera_position` (1.0s into a 2.0s linear 0→200 pan → camera.x≈100),
  `camera_target_timeline_drives_camera_zoom` (1.0→2.0 zoom → 1.5), `camera_target_ignores_own_transform`
  (marked entity's own Transform stays put while the camera moves), `non_camera_timeline_leaves_camera_untouched`
  (no marker → camera unchanged, Transform driven as before). Test count 278→**282**, all pass.
- **New example `examples/timeline_cutscene.rs`** (~290 lines, top-level so auto-discovered;
  **cross-platform** — no native-only deps). A `CutsceneSystem` (state machine: `Explore → Cutscene → Play
  → Won`) + a `TimelineSystem`, registered TimelineSystem-first so CutsceneSystem reads freshly-advanced
  state. The cutscene is authored as four `Timeline`s built paused (`playing=false`): a camera rig
  (`CameraTarget` + pos/zoom there-and-back), two gate panels (position slide), a full-screen overlay
  (alpha fade). Walk into the rune → `start_cutscene` calls `restart()` on all four; skip sets each
  `time = duration`; completion = poll the rig's `is_finished()`.
- **`Camera` is top-left anchored, Y-down** (the carried gotcha): the rig frames the gate with
  `pos≈(330,107) zoom 1.55` (visible ≈619×387) so the zoomed view stays inside `[0,960]×[0,600]`. The
  overlay is a single 2400×1600 black quad centered at (480,300) so it covers the screen at any in-room
  pan/zoom (the fade can't be a screen-sized world quad — it'd slide off during the pan).
- **No on-finish hook / skip API added** — per the grilled engine-fix bar ("fix only the gap the example
  hits"). Skip = `time = duration` (the system then samples the final keyframe); return-to-control = the
  example polls `is_finished()`. The camera-drive is the *only* engine change.
- **`docs` updated** (4 files): CHANGELOG (Added example + `CameraTarget`/`zoom`), NEXT_WORK (candidate L +
  trimmed Timeline from remaining → only networking left), HANDOFF, CLAUDE.md (module-map row).
- **Verification:** `./scripts/verify.sh` green (fmt, clippy `-D warnings`, wasm lib+bins build,
  `test --all-targets` incl. the 4 new tests, rustdoc `-D warnings`); native `cargo run --example
  timeline_cutscene` agent-playtested 12/12; `rust-survivors` path-patch `cargo check --workspace` clean
  (`game v0.1.0`, uses 0 Timeline; additive API). User confirmed the playtest 12/12 and signed off.
- **Commit `8049dfa`** (7 files, +215/-2 + new example ~290 lines). CI run `27017341543` **completed
  success**. The prior tip `session:` commit's CI (`27012640513`) also confirmed green this session.
- **`bd` (beads) unavailable** — chains tracked purely by `HANDOFF_*`/`PLAN_*` filenames + headers.

## What We Tried (Chronological)

1. **Onboarded** from the rendertarget-offscreen PLAN (paste prompt told me to execute its Phase 1).
   Created 3 phase tasks. Confirmed green start: `git status` clean, tip `3828cf5`; **CI success on
   `cbbdfbd` (run 27012114491)**; the tip `session:` commit's CI (`27012640513`) was in_progress (docs-only,
   later confirmed green); `./scripts/verify.sh` exit 0 (278 tests).
2. **Scouted the REAL Timeline API surface** (the anti-mis-scout step). Read all of `src/timeline.rs`:
   `Timeline` drives the entity's **own** `Transform`(pos/rot/scale)+`Sprite`(color/alpha) via
   `TimelineSystem`; builder `new/.looping()/.play()/.pause()/.restart()/.is_finished()`; `Track<T:Lerp>`
   with `.add/.sample/.duration`. Re-exported, 17 unit tests, **zero examples** (`grep -rln Timeline
   examples/` empty). Confirmed `Camera`/`FadeTransition` are **Resources** (so Timeline can't touch them);
   examples drive the camera via `resource_mut::<Camera>` (minimap/touch_demo/lit_dungeon).
3. **Recommended Timeline** via AskUserQuestion (Timeline recommended, networking the alternative). **User
   picked Timeline.** Honestly reframed the gap first: Timeline's API is complete for what it binds to; the
   gap is "no game" + the camera can't be timeline-driven (and even that is reachable indirectly via a
   rig-entity + a copy system, so it's an *ergonomics* gap — same shape as RenderTarget).
4. **`/grill-me`** (3 AskUserQuestion rounds + closure packet, `plan_allowed: true`). Locked: **add
   camera-drive to Timeline** · engine-fix bar = **fix only the gap the example hits, zero-change OK** ·
   cutscene = **key-trigger + skip + return-to-control** · API shape = **reuse Timeline + a `CameraTarget`
   marker** (not a separate `CameraTimeline` type) · channels = **pan + zoom** (no rotation) · skip/return
   = **example-side** (`is_finished()` poll, `time=duration`) · compat = **cross-platform, wasm lib+bins
   green** · proof bar = **same as the last 4 cycles**.
5. **Implemented the engine change** (`CameraTarget` + `zoom` track + TimelineSystem branch + 4 tests +
   lib re-export). `cargo test --lib timeline` → 18/18 (14 prior + 4 new). Borrow note: take `tl` out, then
   `world.get::<CameraTarget>()` (immut) ends before `world.resource_mut::<Camera>()` (mut) — no conflict.
6. **Wrote `examples/timeline_cutscene.rs`** using the **top-left/Y-down** convention from the start (did
   NOT repeat the RenderTarget draft's center-origin mistake). Built; `verify.sh` flagged only fmt
   line-wrapping → `cargo fmt` → `verify.sh` green.
7. **Agent-ran the playtest** (memory `playtest-windowed-examples`): launched detached, queried window
   bounds by PID, `caffeinate` + `screencapture`, synthetic `key down/up "d"`. Confirmed via screenshots:
   Explore scene (A1), camera **pan+zoom** onto the gate + **two gate panels sliding apart** (B2/B3),
   return-to-control banner + camera home (C1), **skip** → Play (C2), walk through → **YOU PASSED THE
   GATE** (C3), R replay (D2).
8. **Two playtest gotchas, both diagnosed by evidence:** (a) `osascript ... keystroke " "` does NOT
   register Space — must use `key code 49`; with that, skip worked. (b) The opening fade flash (overlay
   alpha) is ~0.4s wide and kept falling between captures (the `raise` osascript overhead drifted timing).
   Rather than chase it, added a **temporary `eprintln`** of the overlay's `Sprite.color[3]` in
   CutsceneSystem → trace showed `alpha 0.0→0.56→0.64→0.699(peak @rig_t≈0.25)→…→0.0 by t=0.6` —
   definitively confirming B4. Removed the eprintln, re-ran `verify.sh` green.
9. **rust-survivors cross-repo check.** Path-patch `cargo check --workspace` → **clean** (`game v0.1.0`,
   `skeleton-engine v2.0.0`, 4.10s); restored `Cargo.lock`. Uses 0 Timeline; additive API.
10. **Built the HTML playtest checklist** (`/tmp/timeline_cutscene_test.html`, 12 items, 4 groups,
    localStorage + markdown export — same template as security_camera). Delivered it + 4 screenshots.
11. **User chose "내가 체크리스트 먼저 돌려볼게"**, ran `cargo run --example timeline_cutscene` themselves
    (exit 0, clean quit), and returned **12/12 통과**. → sign-off.
12. **Committed `8049dfa`**, pushed `3828cf5..8049dfa`, watched CI in background → run `27017341543`
    **completed success**.
13. **Post-feature:** user ran `/claude-dashboard:setup` (→ displayMode compact→**normal**) and
    `/claude-dashboard:update` (statusLine path normalised to `~`, already v1.26.2), then asked to do
    `/handoff` + commit/push after CI green.

## Key Decisions

- **New chain `timeline-cutscene` seq 1, not a continuation of `rendertarget-offscreen`.** I executed the
  rendertarget-offscreen PLAN, but Timeline is a fresh feature stream — mirrors that chain's own precedent.
  The rendertarget-offscreen files are siblings, not parents.
- **Candidate = Timeline** (user-chosen over networking). The honest reframe before recommending: the gap is
  "no playable game" + the camera-can't-be-timeline-driven ergonomics hole — NOT a missing core API.
- **API shape = reuse `Timeline` + a `CameraTarget` marker** (user-chosen over a separate `CameraTimeline`
  type). Reuses all Track/time logic; smallest surface (1 marker + 1 branch + 1 `zoom` field). A parallel
  `CameraTimeline` type would duplicate the time-advance + Track plumbing.
- **Camera channels = pan + zoom only** (no rotation). Rotation would widen the surface for a beat the
  example doesn't need (anti-goal). The `zoom` track lives on `Timeline` (one more field, empty/inert for
  non-camera entities) rather than a hacky reuse of `scale`.
- **Skip + return-to-control are example-side** (engine-fix bar). Skip = `time=duration` (system samples
  final keyframe); return = poll `is_finished()`. No engine on-finish event/hook added.
- **The camera rig holds the home framing while paused** — its `position`/`zoom` t=0 keyframes are
  `(0,0)`/`1.0` (= the Explore framing), so the always-present `CameraTarget` rig drives the camera to home
  during Explore and there-and-back during the cutscene, ending home again. This avoided add/remove-marker
  juggling: the rig keeps `CameraTarget` the whole time.
- **Overlay = one oversized (2400×1600) black world quad**, not a screen-sized quad. A screen-sized world
  quad would slide off during the camera pan; an oversized one covers any in-room view. (A screen-space
  `DrawImage` overlay would be camera-independent but a `Timeline` can't drive its alpha — Timeline drives
  the `Sprite` *component*.)
- **Diagnose-by-evidence for the fade** (carried lesson, decisive again): a temporary `eprintln` of the
  driven alpha beat chasing a 0.4s flash across a dozen screenshots.
- **No art / colored sprites** sized in pixels (matches security_camera/crane). Keeps it scoped.

## Evidence & Data

### Commit (this session)

| Hash | Summary | Files | +/- |
| --- | --- | --- | --- |
| `8049dfa` | feat(timeline): timeline_cutscene example + CameraTarget camera-drive | 7 | +215/-2 (+ new example ~290 lines) |

### Diffstat (`git show --stat 8049dfa`, approx)

```
CLAUDE.md                       |   1 +
docs/CHANGELOG.md               |  12 ++
docs/HANDOFF.md                 |  40 ++++
docs/NEXT_WORK.md               |  24 ++-
examples/timeline_cutscene.rs   | ~290 ++++++++++++++  (new)
src/lib.rs                      |   2 +-
src/timeline.rs                 | 138 ++++++++++++++
```

### Verification (final, all green)

| Check | Result |
| --- | --- |
| `cargo fmt --check` | clean (after `cargo fmt` re-wrapped the new code) |
| `cargo clippy --all-targets -- -D warnings` | 0 warnings |
| `cargo build --target wasm32-unknown-unknown` (lib+bins) | clean (example is top-level, not in this gate) |
| `cargo test --all-targets` | 282 pass (278 prior + 4 new camera-drive) |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps` | clean (intra-doc links to `CameraTarget`/`Camera` resolve) |
| native `cargo run --example timeline_cutscene` | cutscene renders end-to-end; user playtest 12/12 |
| `rust-survivors` `cargo check --workspace` (path-patched) | clean (`game v0.1.0`, `skeleton-engine v2.0.0`, 4.10s) |
| CI `27017341543` | **completed success** |

### Grill scope-lock (decision packet, plan_allowed: true)

| Question | Chosen |
| --- | --- |
| candidate | **Timeline / cutscene** (over networking) |
| camera approach | **add camera-drive to Timeline** (over example-side rig / fixed camera) |
| engine-fix bar | **fix only the gap the example hits** (zero change accepted) |
| cutscene loop | **key-trigger + skip + return-to-control** |
| API shape | **reuse Timeline + `CameraTarget` marker** (over a separate `CameraTimeline` type) |
| v1 channels | **pan + zoom** (over pan-only / pan+zoom+rotation) |
| skip/return | **example-side** (`is_finished()` poll, `time=duration`; no engine hook) |
| compat/wasm | **cross-platform; wasm lib+bins green** |
| proof bar | **same as the last 4 cycles** |

### Overlay fade alpha trace (the eprintln evidence, opening flash)

```
rig_t=0.00 alpha=0.000   rig_t=0.20 alpha=0.560   rig_t=0.40 alpha=0.229
rig_t=0.07 alpha=0.185   rig_t=0.27 alpha=0.636   rig_t=0.47 alpha=0.102
rig_t=0.13 alpha=0.372   rig_t=0.33 alpha=0.407   rig_t=0.60 alpha=0.000
max alpha observed = 0.699  (matches the 0.7 keyframe at t=0.25)
```

### Playtest (HTML checklist, 12 items, 4 groups) — 12/12 ✅, user-run, no failures

A (boot/render: A1 scene, A2 rune pulse), B (cutscene camera-drive core: B1 trigger, B2 camera pan+zoom,
B3 gate panels slide apart, B4 overlay fade), C (loop/state: C1 finish→return, C2 Space skip, C3 win),
D (controls: D1 move + gate-block, D2 R replay, D3 Esc quit).

### Example tuning constants (`examples/timeline_cutscene.rs`)

```
WINDOW 960×600 · world = pixels · camera TOP-LEFT anchored, Y-DOWN (home cam (0,0) zoom 1 → x[0,960] y[0,600])
Player: HALF 17, START (140,300), SPEED 240, room clamp x[24,936] y[60,540]; GATE_BLOCK_X 575 (pre-cutscene)
Gate (wall strip x≈[600,640], doorway hole y[214,386]): DOOR_X 620; top panel closed (620,257)→open (620,128),
  bottom panel closed (620,343)→open (620,472); panels RenderLayer(1) tuck behind walls RenderLayer(2)
Rune trigger: (430,300) half (46,60); Exit: (740,300) half (42,72)
Cutscene CUT_DUR 4.4s; CAM_GATE_POS (330,107) zoom 1.55 (visible ≈619×387, gate+exit framed in-bounds)
Rig pos: 0→(0,0); 0.5→(0,0); 1.3→(330,107); 3.2→(330,107); 4.4→(0,0)  | zoom: 1.0→1.55→…→1.0 (there-and-back)
Gate panel pos: 0→closed; 1.7→closed; 2.7→open; 4.4→open    Overlay alpha: 0→0.7(t.25)→0(t.6) … 0→0.6(t4.0)→0
Overlay quad 2400×1600 @ (480,300) RenderLayer(5) (covers screen at any in-room pan/zoom)
Controls: WASD/arrows move · Space skip (osascript: key code 49!) · R replay · Esc quit
```

## Code Analysis

- **`TimelineSystem::run`** (`src/timeline.rs`): collects entities with `Timeline`, then per entity:
  `take_component::<Timeline>`, advances `time` iff `playing && !is_finished()` (loop wraps, else clamps to
  `duration`), **then** — NEW — `if world.get::<CameraTarget>(entity).is_some()` writes
  `position`→`Camera::position` + `zoom`→`Camera::zoom` and `continue`s; otherwise the existing
  pos/rot/scale→`Transform`, color/alpha→`Sprite` writes. Sampling/applying happens **every frame**
  regardless of `playing`/`is_finished` — so a paused timeline holds its t=0 frame and a finished one holds
  its last frame (this is why the paused rig holds the home framing and the finished gate stays open).
- **`Timeline`** (`src/timeline.rs`): `{ duration, time, looping, playing, position: Track<Vec2>, rotation:
  Track<f32>, scale: Track<Vec2>, color: Track<[f32;4]>, alpha: Track<f32>, zoom: Track<f32> }`. `new`
  defaults `playing=true` — so a *triggerable* cutscene must set `playing=false` after construction (the
  example's `camera_timeline()/gate_timeline()/overlay_timeline()` builders do exactly that).
- **`Track<T: Lerp>::sample(t)`**: empty → `None`; before first / after last keyframe → clamp to
  first/last; else lerp between bracketing keyframes with `a.easing.apply(local_t)`. NaN-safe.
- **`Camera`** (`src/camera.rs`): public `position: Vec2` (viewport **top-left** world px) + `zoom: f32`;
  `visible_rect(w,h) = (pos, pos + (w/zoom, h/zoom))`, +Y down. A camera at `(0,0)` zoom `1` shows
  `[0,W]×[0,H]`. Zooming in near the right edge shows past it → the example frames the gate at
  `pos (330,107)` so the zoomed rect stays in-bounds.
- **`World`** (`src/ecs/world.rs`): `resource_mut::<T>() -> Option<&mut T>`, `get::<T>(e)`,
  `get_mut::<T>(e)`, `take_component`, `add_component`, `insert_resource`, `spawn`. System `run(&mut self,
  &mut World, dt)`. Systems run in insertion order → example adds `TimelineSystem` before `CutsceneSystem`.

## Files Changed

### Source
- `src/timeline.rs` — `CameraTarget` marker; `Timeline::zoom: Track<f32>`; `TimelineSystem::run` camera
  branch; 4 unit tests + their imports. +138.
- `src/lib.rs` — re-export `CameraTarget` (line 116). +2/-2.

### Examples
- `examples/timeline_cutscene.rs` — NEW (~290 lines). Triggerable cutscene; cross-platform; top-left/Y-down;
  colored sprites; `CutsceneSystem` state machine + four paused `Timeline`s (rig/2 gates/overlay).

### Docs
- `docs/CHANGELOG.md` — `## 2.0.0` Added: timeline_cutscene example + `CameraTarget`/`Timeline::zoom`.
- `docs/NEXT_WORK.md` — candidate **L** section; trimmed Timeline from the never-in-a-game list (only
  networking remains).
- `docs/HANDOFF.md` — `## 2026-06-05 — Timeline cutscene example + CameraTarget camera-drive` entry.
- `CLAUDE.md` — module-map row for `Timeline`/`CameraTarget`.

## User Feedback & Preferences (REQUIRED — never omit)

- **Works in Korean; wants conversational replies in Korean.** Handoff/docs stay English per the
  doc-language rule.
- **Chose the recommended candidate** (Timeline) and every recommended grill option (camera-drive, reuse +
  marker, pan+zoom, example-side skip, same proof bar). Calibration: the user accepts well-reasoned
  recommendations when the trade-offs are laid out — but still present bounded choices.
- **"내가 체크리스트 먼저 돌려볼게" — ran the playtest themselves this time** (contrast the prior session's
  "체크리스트 실행해줘" delegation). Still deliver the HTML checklist + screenshots; let the user decide who
  runs it. Returned a clean 12/12.
- **"ci 체크 후에 /handoff 하고 커밋 푸쉬 까지 해줘"** — wants CI confirmed green *before* the handoff, then
  the handoff committed + pushed. Direct-to-main is the established norm.
- **Asked to open the HTML checklist** (`open …`) — likes the deliverable surfaced, not just mentioned.
- **Ran `/claude-dashboard:setup` (normal mode) + `/claude-dashboard:update` mid-session** — comfortable
  driving plugin/status-line config; these are independent of the engine work.
- **Consistently prefers the engine fix over a workaround** (carried) — here the camera gap was closed in
  the engine (CameraTarget) rather than worked around per-example.
- **Likes diagnose-by-evidence** (implicit; the eprintln alpha trace and the `key code 49` discovery both
  landed fast).

## Where We're Going

1. **(next PLAN's job)** The last never-in-a-game candidate is **networking** (`mp_client`/`mp_server`
   exist but there's no small *playable* multiplayer toy). It is the least self-contained / highest-infra
   candidate — the prior plans deliberately deprioritised it. Options for the next cycle: (a) dogfood
   networking with a minimal playable toy (two dots in a shared room), or (b) a non-dogfood direction
   (e.g. one of the deferred Timeline/RenderTarget follow-ups, or an editor/tooling pass). **User's call —
   recommend, then `/grill-me`.** Run the same loop: confirm green → read the REAL API surface first
   (`src/network/` + the `mp_*` examples) → recommend + grill → implement + a small playable example →
   HTML-checklist playtest → verify.sh + rust-survivors check → single commit/push → confirm CI.
2. **Verify the API surface before planning engine work** — this lineage's standing lesson (joints,
   RenderTarget, AND Timeline all already existed; the gap was always "no game" + at most an ergonomics
   hole). For networking, read `src/network/` and the `mp_client`/`mp_server` examples before claiming a gap.
3. **Deferred Timeline follow-ups** (only if a future example needs them; do NOT widen speculatively):
   an on-finish event/callback (`Events<TimelineFinished>` or similar) for sequencing/return-to-control; a
   camera **rotation** track; a multi-timeline sequencing helper; an explicit `seek(t)`/skip API. None
   blocking.

## Risks & Blockers

- **Low.** verify.sh green; native playtest 12/12; rust-survivors clean; CI green. The camera-drive is
  additive + opt-in (marker-gated) + pure logic (unit-tested, no GPU needed).
- **Behavior-change caveat:** any code that put both a `Timeline` and a `CameraTarget` on an entity and
  *also* relied on that entity's `Transform` being driven would now see the Transform left alone (camera
  takes over). No in-repo consumer does this; it's new surface. Documented in CHANGELOG.
- **`new` defaults `playing=true`** — a contributor authoring a triggerable cutscene must remember to pause
  (the example builders do). A paused camera-rig still drives the camera to its t=0 frame each tick (by
  design — holds the home framing).

## Open Questions

- **On-finish hook (deferred):** should `Timeline` emit a finish event for sequencing / handing control
  back, instead of polling `is_finished()`? Owner: a future multi-shot cutscene example. Default: poll.
- **Camera rotation track (deferred):** pan+zoom only today. Owner: a future example that rolls the camera.
- **Networking dogfood scope:** if chosen next, how minimal? `mp_client`/`mp_server` exist but headless;
  a playable toy needs two windowed clients + a room — higher infra than any prior candidate. Decide in grill.
- Carried (not this session's concern): distance-joint-is-a-spring / no `add_fixed_joint`; RenderTarget
  HUD/resize/clear-color/UI-widget deferrals; `split_screen` center-origin layout.

## Quick Start for Next Session

```bash
# Restore context (bd unavailable — chains are the HANDOFF_/PLAN_ files)
cat plans/handoffs/HANDOFF_timeline-cutscene_camera-drive_2026-06-05.md
cat plans/handoffs/PLAN_timeline-cutscene_camera-drive_2026-06-05.md   # the paired next-cycle plan

# Verify current state
git log --oneline -3                       # expect 8049dfa + this session's session: commit at tip
git status -s                              # expect clean
gh run list --branch main --limit 3        # confirm CI green on 8049dfa (run 27017341543)
./scripts/verify.sh                        # 5 checks, expect exit 0 (282 tests)

# Scout the next candidate — READ THE REAL SURFACE FIRST (standing lesson)
ls src/network/ 2>/dev/null; sed -n '1,80p' src/network*/*.rs 2>/dev/null   # networking API
grep -rln "mp_client\|mp_server\|network" examples/                          # the multiplayer examples
sed -n '198,260p' docs/NEXT_WORK.md        # candidate L entry + remaining (networking only)
cat docs/VISION.md                         # the feature+example loop

# This session's deliverables (reference patterns)
sed -n '1,30p' examples/timeline_cutscene.rs                 # top-left/Y-down + CameraTarget rig idiom
git --no-pager show 8049dfa -- src/timeline.rs               # the CameraTarget camera-drive
cat ~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/playtest-windowed-examples.md

# Next action: pick the next candidate (networking is the last never-in-a-game one; or a non-dogfood
#   direction — user's call), read its REAL API surface, /grill-me to lock scope, then implement + example.
```
