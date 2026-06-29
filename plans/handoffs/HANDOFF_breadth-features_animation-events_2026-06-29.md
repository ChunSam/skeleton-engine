# Frame-triggered animation events — `AnimationEvents` component shipped (v0.83.0, PR #274), first breadth feature of a new chain

**Date:** 2026-06-29
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `breadth-features` seq `1`
**Parent:** `none — first in chain`
**Prior chain:** none — first in chain

> Chain rationale: the parent session's `hardcoding-audit` chain was **explicitly CLOSED** (its "Chain State" said a future session "need not 'continue the audit' — start from the wishlist board or a fresh breadth gap"). This session did breadth-feature work (animation events), the feature that parent's "Where We're Going" recommended as the default. So this is a **new forward chain `breadth-features`** (the SpriteFlip #271 / YSort #272 / AnimationEvents #274 stream), not a revival of the audit chain.

## Related Handoffs

- `HANDOFF_hardcoding-audit_audit-closed-breadth-pivot_2026-06-29.md` — `hardcoding-audit` seq 4 (the session's true parent-by-paste; #267–#273). It closed the audit and **pivoted to breadth features** (SpriteFlip, YSort), and its "Where We're Going" listed **animation events as the recommended next feature** — which this session built. Read it for the breadth-pivot rationale + the reusable gap-finding method.
- `HANDOFF_headless-screenshot_2026-06-28.md` — `headless-screenshot` chain. Its `App::save_screenshot_headless` is what the `animation_events` example used for the Metal pixel/console verification (`HEADLESS_SHOT`).

## Reference Documents

- `CLAUDE.md` — project conventions + module map (the animation row was updated this session). Header bumped to **v1.6.169** / package **v0.83.0**.
- `docs/CHANGELOG.md` — the 0.83.0 entry written this session.
- `docs/PATTERNS.md` — ECS query API, render-layer separation, the per-format pipeline cache (not touched, but the canonical pattern doc).
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — bumped to **seq 116** (the most detailed per-seq record of this session lives in that bullet).
- `../dungeon-merchant/docs/engine-wishlist.md` — the game↔engine board; **ACTIVE EMPTY** (next free ID EW-004). Read FIRST next session.

## The Goal

The engine is now very broad (the CLAUDE.md module map covers nearly every 2D subsystem), so net-new gaps are narrow. The parent session pivoted from the (now-closed) hardcoding audit to adding **genuinely-missing 2D-engine breadth features** that downstream games had to hand-roll — shipping SpriteFlip (#271) and YSort (#272). This session continued that pivot by building **animation events**: a way to fire a tagged event when an animation playhead reaches a specific frame (footsteps, attack hit-frames, VFX/SFX triggers) — the idiomatic alternative to hand-polling `AnimationPlayer::current_frame` every frame. The end state: any game can attach events to clips and react to them through the standard `Events<E>` bus, with zero changes to existing animation code. The acceptance test (per the VISION loop) is a small playable example that exercises it in real play.

## Where We Are

- **main @ `d0dc587`** (package **v0.83.0**, CLAUDE.md header **v1.6.169**), tree **clean**, **no open PRs**.
- **PR #274 merged** (squash + branch-deleted, CI **5/5** green): `feat(animation): frame-triggered animation events — AnimationEvents component (v0.83.0)`.
- **New module** `src/animation/events.rs` — `FrameEvent { clip, frame, tag }`, `AnimationEvents { events: Vec<FrameEvent> }` (component, builder `new().on(clip,frame,tag)`), `AnimationEvent { entity, clip, frame, tag }` (emitted).
- **`AnimationSystem::run` wired** (`src/animation/system.rs`) — records the frames each playhead transitions onto per tick, matches them against the entity's `AnimationEvents`, flushes into `Events<AnimationEvent>` after the per-entity loop. Added a `warned_no_event_bus: bool` field for the one-time unregistered-bus warning.
- **New public API** (all additive, non-breaking): `engine::AnimationEvents`, `engine::FrameEvent`, `engine::AnimationEvent` (crate-root re-exports + `animation::` re-exports).
- **Registration:** `register_clone::<AnimationEvents>` in `core_resources.rs`; editor add/remove in `editor/component_registry.rs` — mirrors `RenderLayer`/`SpriteFlip`/`YSort` (clone + editor add/remove, **no serde registration, no Reflect** — confirmed siblings have none).
- **New example** `examples/animation_events.rs` (flat, auto-discovered) — runtime-generated 4-frame walk sheet, `"footstep"` events on contact frames 1/3, a reader system that counts steps + flashes "STEP!" + logs each one; `HEADLESS_SHOT` support (defaults to 60 warmup frames).
- **Tests:** lib at **965 passed** (skipping 2 environmental audio), up from 957 at session start — **+8 new** (3 in `events.rs`, 5 in `system.rs`) + 1 doctest.
- **CI:** PR #274 passed the 5-job matrix (Test native 4m34s / Build WASM / Render lavapipe / Rustdoc / Package dry-run). The native job ran the audio tests green (confirming the 2 local failures are environmental); the lavapipe render job gates the GPU path.
- **Headless render verified on Metal:** console fired `footstep! (frame 1)` then `footstep! (frame 3)`; the screenshot showed the sprite (frame 0 after wrap), HUD `footsteps: 2`, and the `STEP! (frame 3)` flash.
- **CLAUDE.md module-map** animation row updated with the AnimationEvents description.
- **Memory** `engine-current-state.md` bumped to seq 116; the oldest detailed bullet (seq 74) folded into the "Older seqs" rollup to keep the file compact.

## What We Tried (Chronological)

1. **Onboarding.** Read the parent handoff (hardcoding-audit seq 4), confirmed the wishlist board is ACTIVE EMPTY, ran the baseline test (`957 passed`, 2 audio filtered — matched the handoff). Read the key files: `animation/player.rs`, `animation/system.rs`, `ysort.rs`, `components.rs` (SpriteFlip), `ecs/events.rs`, `animation/mod.rs`. Grep-confirmed both candidate gaps are absent (`AnimationEvent`, `TriggerZone`/`Area2D` — zero hits in `src/`).
2. **Asked the user which breadth feature** via AskUserQuestion (animation events recommended / trigger zones / other) → user chose **"Animation events (권장)"**.
3. **Data-model investigation (the key decision).** Grepped `'AnimationClip {'` struct-literal sites → **26 across 8 files** (`clip_set.rs`, `system.rs`, `blend_system.rs`, `player.rs`, `state_machine/tests.rs`, examples `blend_locomotion`/`sm_crossfade`/`platformer`). A field on `AnimationClip` would break all of them. Confirmed `AnimationClip` has no `new()` constructor (only `AnimationPlayer::new`). → Chose a **separate `AnimationEvents` component** (mirrors the SpriteFlip #271 decision exactly). Also confirmed `world.resource::`/`resource_mut::` both return `Option`, and `Entity` is `Copy + Eq + Hash`.
4. **Wrote `src/animation/events.rs`** — the 3 types + builder + 3 unit tests (builder order, default empty, serde round-trip via `ron::to_string`/`from_str`).
5. **Wired `AnimationSystem::run`.** Added a local `pending: Vec<AnimationEvent>` before the entity loop and per-entity locals `entered_frames`/`active_clip` (locals, not `self` fields — the loop holds `&self.scratch`, so a second `&mut self` borrow is impossible). In the main-clip advance `while` loop, recorded each frame transition (`prev_frame != current_frame`). After the component writes, matched `entered_frames` against the entity's `AnimationEvents`. After the loop, flushed `pending` into `Events<AnimationEvent>` if registered, else a one-time `warn!`.
6. **Re-exports + registration** — `animation/mod.rs` (`pub mod events` + re-export), `lib.rs` (crate-root), `core_resources.rs` (register_clone), `editor/component_registry.rs` (add/remove).
7. **Added 5 system tests** (`event_fires_when_playhead_enters_event_frame`, `non_matching_frame_emits_nothing`, `initial_frame_does_not_fire_but_loop_wrap_does`, `unregistered_bus_drops_events_and_warns_once_without_panic`, `entity_without_events_component_emits_nothing`). **First run RED:** `initial_frame_does_not_fire_but_loop_wrap_does` expected 1 event but got 0 — my test math was wrong (350 ms of a 100 ms/frame 4-frame clip = 3 transitions 0→1→2→3, NOT a wrap; a wrap needs 4 transitions = 450 ms total). Fixed the test to `run(0.40)` after the initial `run(0.05)`. All 8 green.
8. **Wrote `examples/animation_events.rs`** — first compile error: `Sprite::textured_with_handle(&sheet, …)` needs `&str` not `&String` (`Into<Arc<str>>` doesn't deref-coerce); fixed to `sheet.as_str()`. Confirmed `Color::rgba`/`rgb` exist, `App::register_event` exists.
9. **Verified.** `cargo fmt` (reformatted 6 files), then clippy `--all-targets -D warnings` (clean, compiled every example), wasm lib+bins build, rustdoc `-D warnings`, lib tests (965), doctests (77). The full `--all-targets` test timed out at 2 min locally (recompiles all examples) — but clippy already compiled them, so ran lib + doc separately.
10. **Headless render** on Metal — console showed both footsteps; the PNG showed sprite + `footsteps: 2` + `STEP! (frame 3)`.
11. **Full `verify.sh`** (CI-faithful) → `VERIFY_EXIT=101` from **only** the 2 environmental audio tests (965 passed). The full test step ran `--all-targets` incl. the lavapipe-equivalent render path locally (this box has a GPU) — only audio failed.
12. **`/ship`** → v0.82.0 → v0.83.0 (MINOR): Cargo.toml + `cargo update -p skeleton-engine` (lock) + CHANGELOG 0.83.0 + CLAUDE.md header v1.6.169. Re-verified fmt + build.
13. **`/land-pr`** → branch `feat/animation-events`, commit `3baec89`, push, PR **#274**, watched CI (5/5 CLEAN), squash-merged `d0dc587`, synced main, bumped memory to seq 116.

## Key Decisions

- **`AnimationEvents` is a DEDICATED component, not a field on `AnimationClip`.** 26 `AnimationClip { … }` struct-literals across 8 files would break with a field add; a separate component is purely additive AND more flexible (a game can attach different event sets to entities that share registry-owned clip data). This mirrors the SpriteFlip #271 reasoning exactly. Rejected: `AnimationClip.events` field (breaking + couples events to clip data).
- **Events fire on frame TRANSITIONS only, never on the currently-shown frame.** Detection is `prev_frame != current_frame` inside the advance loop. Consequences (all documented): the initial frame at spawn never fires (it's never "entered"); a looping clip re-fires a frame-0 event on each wrap; a held last frame (non-looping) does not re-emit. This is the natural footstep/hit-frame semantic and avoids spurious repeats.
- **During a crossfade, only the outgoing (current) clip emits.** The to-clip's `to_frame` advance (separate code path) is NOT hooked for v1 — kept the borrow simple. The incoming clip starts emitting once the crossfade completes and it becomes current. Documented as a known limitation.
- **Emission buffers are LOCALS, not `self` fields.** The entity loop iterates `for &entity in &self.scratch`, which immutably borrows `self` — so pushing to a `self.pending` would be a borrow conflict. `pending` (per-run) and `entered_frames`/`active_clip` (per-entity) are locals. They allocate only when a frame actually advances / an event matches, so the hot path (no component, or no frame advance) is **zero-allocation** (`Vec::new()` doesn't allocate until first push). `warned_no_event_bus` IS a `self` field — safe because it's mutated only after the scratch loop ends.
- **Unregistered bus → drop + one-time warn, not panic, not require.** `world.resource_mut::<Events<AnimationEvent>>()` returns `Option`; if `None`, pending events are dropped and a single `warn!` fires (guarded by `warned_no_event_bus`). Keeps `AnimationEvents` opt-in — a game that ignores events needn't register the bus. Mirrors the physics CollisionEvent ergonomics but lighter.
- **Registered like `RenderLayer` (clone + editor add/remove, NO serde registration, NO Reflect).** Grep-confirmed SpriteFlip/YSort/RenderLayer have no `register_serde_component` call → mirrored that exactly rather than over-integrating.
- **Versioning: MINOR (v0.83.0).** Additive feature, pre-1.0 → MINOR per the project rule (same as SpriteFlip 0.81.0 / YSort 0.82.0).
- **Example defaults to 60 headless warmup frames** (vs the static examples' 4). Headless uses `dt = 1/60` per frame; at 4 fps a footstep lands at frame 1 ≈ 15 frames, so 60 frames (~1 s) guarantees a couple of steps + a live flash in the capture. Documented in the example's header.

## Evidence & Data

### Commit / PR (this session)

| Hash | PR | Version | Seq | Summary |
|---|---|---|---|---|
| `3baec89` (→ squashed `d0dc587`) | #274 | v0.83.0 | 116 | `AnimationEvents` frame-triggered animation events |

### New public API surface (all additive / non-breaking)

| Symbol | Kind | Location |
|---|---|---|
| `AnimationEvents { events: Vec<FrameEvent> }` | component | `animation/events.rs` |
| `AnimationEvents::new()` / `.on(clip, frame, tag)` | builder | `animation/events.rs` |
| `FrameEvent { clip, frame, tag }` | struct | `animation/events.rs` |
| `AnimationEvent { entity, clip, frame, tag }` | emitted event | `animation/events.rs` |

Crate-root re-exports added to `lib.rs` (in the `pub use animation::{…}` block): `AnimationEvent`, `AnimationEvents`, `FrameEvent`. Also re-exported from `animation::`.

### Tests added (8 + 1 doctest)

- `animation::events::tests`: `builder_collects_events_in_order`, `default_is_empty`, `serde_roundtrip`.
- `animation::system::tests`: `event_fires_when_playhead_enters_event_frame` (incl. a 250 ms multi-frame jump → one match on frame 2), `non_matching_frame_emits_nothing`, `initial_frame_does_not_fire_but_loop_wrap_does`, `unregistered_bus_drops_events_and_warns_once_without_panic`, `entity_without_events_component_emits_nothing`.
- Doctest: `AnimationEvents` (builder example).

### Test counts

`957 passed` (session start) → `965 passed` (+8), 2 environmental audio tests skipped/filtered locally, all green on CI.

### CI (PR #274 — 5-job matrix, all green)

| Job | Result | Time |
|---|---|---|
| Test (native) | pass | 4m34s |
| Build (WASM) | pass | 43s |
| Render tests (lavapipe) | pass | 1m14s |
| Rustdoc | pass | 40s |
| Package dry-run | pass | 1m11s |

### Headless render verification (Metal, `HEADLESS_SHOT=/tmp/anim_events.png`)

- Console: `footstep! (frame 1) total = 1` then `footstep! (frame 3) total = 2`, then `wrote /tmp/anim_events.png (60 frames)`.
- Screenshot: title; the sprite at frame 0 (cool/raised "passing" pose, since the playhead wrapped by frame 60); HUD `current frame: 0   footsteps: 2`; `STEP!  (frame 3)` flash still active; `Esc: quit` hint.

Reproduce: `HEADLESS_SHOT=/tmp/anim_events.png cargo run --example animation_events` (native GPU; works monitor-off via the surfaceless path; `HEADLESS_FRAMES=N` overrides the 60-frame default).

## Code Analysis

- **`AnimationSystem::run`** (`src/animation/system.rs`): clears + repopulates `self.scratch` with all `AnimationPlayer` entities, then `for &entity in &self.scratch`. The per-entity block borrows `AnimationPlayer` mutably to advance crossfade + main clip, returns `(uv, weight, blend_uv)`, releases the borrow, writes those 3 components, THEN reads `AnimationEvents` + matches. After the loop, flushes `pending` into the bus.
- **Frame-entry detection** lives in the main-clip advance `while player.timer >= dur` loop (NOT the crossfade `to_frame` loop): `let prev_frame = player.current_frame;` then after the `% n` (looping) or `.min(n-1)` (non-looping) update, `if player.current_frame != prev_frame { entered_frames.push(player.current_frame); }`. `active_clip = player.current_clip` is captured before the loop (current_clip doesn't change during the main advance).
- **`crossfade_just_completed`**: when a crossfade finishes this tick the main advance block is SKIPPED → `entered_frames` stays empty, `active_clip` stays 0, no events. Correct (the to-frame advance isn't hooked).
- **Borrow constraint** (the subtle part): `for &entity in &self.scratch` immutably borrows `self`, so neither `pending` nor `entered_frames` can be `self` fields (would be a second mutable borrow). `world` mutation inside the loop is fine (`world` is the `&mut World` param, distinct from `self`). `warned_no_event_bus` is mutated only after the loop → fine as a `self` field.
- **`Events<E>`** (`src/ecs/events.rs`): per-frame bus, `send`/`read` + end-of-frame `flush` (App auto-flushes). `world.resource_mut::<Events<AnimationEvent>>()` returns `Option<&mut>` → graceful skip when unregistered.
- **`UvRect::from_grid(col, row, cols, rows)`** slices a sprite sheet into per-frame UVs — used by the example's clip (`(0..4).map(|c| UvRect::from_grid(c, 0, 4, 1))`).
- **Registration template** (confirmed by grep): `RenderLayer`/`SpriteFlip`/`YSort` = serde-derive + `register_clone` + editor add/remove, **no `register_serde_component`, no Reflect**. AnimationEvents follows the same.

## Gotchas & Discoveries

- **The wrap-around test math (caused the only RED):** a 4-frame, 10-fps (100 ms/frame) looping clip needs **4 transitions** (0→1→2→3→0) to re-enter frame 0. After an initial `run(0.05)` (timer = 50 ms), `run(0.30)` adds only 350 ms total → 3 transitions, stops at frame 3 (NOT a wrap). Use `run(0.40)` (450 ms total) for the wrap. When asserting loop-wrap events, count transitions, not elapsed-vs-duration.
- **`Sprite::textured_with_handle` needs `&str`, not `&String`.** Its arg is `impl Into<Arc<str>>`, which does NOT deref-coerce a `&String`. `app.load_image(&sheet)` accepts `&String` (deref to `&str`), but the sprite ctor doesn't — pass `sheet.as_str()`. (The example bound `sheet` as a `String` from `to_string_lossy().to_string()`.)
- **Headless render uses `dt = 1/60` per frame** (`src/app/headless.rs`), so an *animated* example needs far more warmup frames than the static examples' default of 4. At 4 fps a footstep first lands ≈15 frames in; the example defaults `HEADLESS_FRAMES` to **60** so the capture shows steps + a live flash. A static example (sprite_flip/ysort) is fine at 4.
- **`verify.sh` exit masking (recurring):** a backgrounded `./scripts/verify.sh > log 2>&1; echo "VERIFY_EXIT=$?"` makes the task-completion notification report the trailing `echo`'s exit `0`, hiding `verify.sh`'s real `101`. ALWAYS read `VERIFY_EXIT=` from the task-output file, not the notification summary. This session it was `101` from the 2 audio tests only.
- **`cargo test --all-targets` times out locally** (~2 min cap) because it recompiles every example. But `cargo clippy --all-targets -D warnings` already compiled them all (with lint enforcement), so running `--lib` + `--doc` separately covers the new code; CI runs the full `--all-targets`.
- **rust-analyzer false positives are noise here:** `ColliderHandle` "expected X, found X" (identical-type) errors in physics examples and `cfg(wasm32)` inactive-code warnings are pre-existing r-a confusion — trust `cargo check`/clippy/CI, not the inline diagnostics. (Carried from prior sessions; reconfirmed.)
- **`world.resource::`/`resource_mut::` both return `Option`** (not panicking) — the clean way to make a system's event emission opt-in (skip when the bus is unregistered). `Entity` is `Copy + PartialEq + Eq + Hash`, so it drops straight into the `AnimationEvent` struct + `==` filters in the example/tests.
- **Flat examples are auto-discovered** — `examples/<name>.rs` needs no `[[example]]` Cargo.toml entry (only multi-file/subdir examples do). `animation_events.rs` is flat.

## Files Changed

### Source — new
- `src/animation/events.rs` — `FrameEvent` + `AnimationEvents` (+ builder) + `AnimationEvent` + 3 tests + doctest.

### Source — modified
- `src/animation/system.rs` — emission wiring (`warned_no_event_bus` field, `pending`/`entered_frames`/`active_clip`, frame-transition recording, match + flush) + 5 tests.
- `src/animation/mod.rs` — `pub mod events` + re-export `AnimationEvent`/`AnimationEvents`/`FrameEvent`.
- `src/lib.rs` — crate-root re-exports in the `pub use animation::{…}` block.
- `src/app/core_resources.rs` — `register_clone::<AnimationEvents>`.
- `src/app/editor/component_registry.rs` — editor add + remove for `AnimationEvents`.

### Examples — new
- `examples/animation_events.rs` — walk cycle + footstep events + reader system + HUD + `HEADLESS_SHOT`.

### Docs / paperwork
- `CLAUDE.md` — animation module-map row updated; header v1.6.168 → v1.6.169 / package v0.82.0 → v0.83.0.
- `docs/CHANGELOG.md` — 0.83.0 entry.
- `Cargo.toml`, `Cargo.lock` — version bump + lock refresh.

## User Feedback & Preferences

- **Chose the feature explicitly via AskUserQuestion** — picked "Animation events (권장)" from the 3 offered breadth candidates. The board was empty, so the user expects me to surface options and let them pick when there's no wishlist request.
- **`/handoff 하고 푸시`** — drives the per-session cadence with short Korean go-aheads; expects the full land-pr loop + handoff run autonomously, reporting outcomes not options.
- **Korean for all user-facing replies; English for code/docs/commits/handoffs** (project doc-language rule).
- **Merge authority delegated** — squash on green CI, no per-session re-confirm; PR #274 landed without asking.
- **Prefers the full land-pr loop per change** — branch → verify → /ship → PR → watch CI → squash-merge → sync → bump memory seq — run without narrating each option.
- **Wants the onboarding narrated** before executing (this session's opening did the 5-point onboarding then waited for go-ahead) — but once go-ahead is given, execute end-to-end.
- **Values evidence over assertion** (carried from the parent session) — verification (headless render, CI, test counts) reported with real numbers.
- Standing env reality: **locked/remote macOS, no audio device** → the 2 audio tests always fail locally; never treat as a regression.

## Where We're Going

The `breadth-features` chain continues until the wishlist board fills. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST each session** (ACTIVE EMPTY, EW-004 next) — a real downstream request outranks self-picked breadth. If empty, ASK the user which breadth feature, then land it via the land-pr loop. Ordered self-pick candidates (each: additive component/system + playable example + tests):

1. **Trigger zones** (the parent's #2 candidate) — `Area2D`-style enter/stay/exit. ⚠️ **`TriggerEvent` already exists in physics (rapier)** — a non-physics zone must use a different name (`ZoneEnter`/`ZoneExit` or `AreaEvent`). Read `src/collision/` (`SpatialGrid`, `Collider`, `CollisionLayer`) — likely a new `TriggerZone` component + a system diffing per-frame overlap sets, emitting on a new `Events<…>`. CI-verifiable via overlap-set unit tests; example a player walking through pickup zones.
2. **Hit-flash** — a brief tint/white-flash on a sprite when hit (every action game re-implements). A `HitFlash { color, secs }` component + a system that lerps the sprite color and removes itself; pairs naturally with the new animation events / a damage event.
3. **Tweened camera lookahead** — bias the camera ahead of a moving follow target. Read `src/camera.rs` (`follow`, `bounds`/`clamp_to_bounds`).
4. Lower value: the editor i18n gap (`editor/ui/audio_panel.rs:34,43` — two English status strings bypass `tr()` in the Korean-default editor), or any deferred invasive Tier-2 knob only on a concrete request.

The shipped pattern to copy (validated 3× now — SpriteFlip, YSort, AnimationEvents): **(a)** grep to confirm the gap + read the subsystem; **(b)** add a component (+ system/event if needed) in its own module; **(c)** register clone + editor add/remove like `RenderLayer`; **(d)** re-export in `lib.rs` + the subsystem `mod.rs`; **(e)** a flat `examples/<name>.rs` with `HEADLESS_SHOT`; **(f)** unit tests + doctest; **(g)** CLAUDE.md module-map row; **(h)** land via the land-pr loop (MINOR), `--skip` the 2 audio tests locally.

## Risks & Blockers

- **Audio-device tests fail locally** (environmental, not a regression) — `--skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate` + read `VERIFY_EXIT=` from the log (the backgrounded `verify.sh` notification masks the real exit with its trailing `echo`). CI gates audio.
- **Crossfade-target events are not emitted** (by design, v1) — if a game needs an event on a clip while it's still crossing IN, that's a new ask (hook the `to_frame` advance loop too). Documented in the `AnimationEvents` doc + the CHANGELOG.
- **No OS-gated code this session** — everything is cross-platform (wasm builds clean; events are pure logic). The standing rule still holds for future work: green CI ≠ verified for `cfg(target_os)` paths.
- No upstream/dependency blockers. Tree clean, no open PRs.

## Open Questions

- Should the crossfade **to-clip** also emit events while blending in? Left out for borrow simplicity; revisit only on a concrete need.
- Should animation events be **data-driven** (authored per-clip in the `clip_set.rs` RON, like particle/dialogue configs)? Natural future extension — would attach an `AnimationEvents` from the loaded data. Out of scope for this PR.
- Should `FrameEvent`/`AnimationEvent` carry a **richer payload** (e.g. an `f32`/`Vec2` param) instead of just a `String` tag? Kept minimal; extend if a game needs it.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4            # confirm main @ d0dc587 (v0.83.0)
git status -s                   # clean

# 1) Read the game↔engine board FIRST (drives priority)
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick breadth

# Memory (full per-seq detail of this session)
#   ~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md (seq 116 tip)

# Key files if continuing breadth features
#   src/animation/events.rs + src/animation/system.rs   — the pattern just shipped (component + Events emission)
#   src/ysort.rs + src/components.rs (SpriteFlip)        — the two prior breadth patterns
#   src/collision/ + src/ecs/events.rs                  — for "trigger zones" (NOTE: physics TriggerEvent exists → new name)
#   src/camera.rs                                       — for "camera lookahead"

# Verify (NOTE: 2 audio-device tests fail locally — environmental)
cargo test --lib -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings

# Reproduce this session's example
HEADLESS_SHOT=/tmp/anim_events.png cargo run --example animation_events

# Next action
#   Read the wishlist board; if still empty, ASK which breadth feature
#   (recommended next: trigger zones — mind the physics TriggerEvent name clash), then land it via the land-pr loop.
```

## Session Closed

**Closed:** 2026-06-29
**Chain:** `breadth-features` seq 1 (first in chain) — origin: the breadth pivot in `HANDOFF_hardcoding-audit_audit-closed-breadth-pivot_2026-06-29.md`
**Code landed:** #274 (v0.83.0), main @ `d0dc587`. This handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off. Next session starts from the wishlist board or a new breadth feature (see "Where We're Going").
