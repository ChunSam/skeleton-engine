# Camera motion lookahead — `Camera::lookahead` shipped (v0.86.0, PR #280), fourth breadth feature of the chain

**Date:** 2026-06-29
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `breadth-features` seq `4`
**Parent:** `HANDOFF_breadth-features_hit-flash_2026-06-29.md` (seq 3)
**Prior chain:** `hardcoding-audit` (closed) → pivoted to `breadth-features` at its seq 4

> Chain rationale: same session as the hit-flash handoff (seq 3). After landing hit-flash + its
> handoff PR, the user said "다음 후보 진행해" (proceed with the next candidate). The seq-3 handoff's
> "Where We're Going" listed **tweened camera lookahead as the #1 (and last) self-pick candidate**.
> So this is a **direct continuation**: `breadth-features` seq 4, parent = the seq-3 handoff.

## Related Handoffs

- `HANDOFF_breadth-features_hit-flash_2026-06-29.md` — **the parent** (`breadth-features` seq 3,
  #278 hit-flash). Its "Where We're Going" recommended camera lookahead next, with a design note on
  where the target's velocity comes from (the option this session took: derive it from the
  follow-position delta — no game-side wiring). Read it for the validated 8-step breadth pattern and
  the zsh `${PIPESTATUS[0]}` gotcha.
- `HANDOFF_breadth-features_trigger-zones_2026-06-29.md` — `breadth-features` seq 2 (#276); the
  no-serde transient-component precedent (not directly relevant here, since lookahead adds fields to
  the existing `Camera` rather than a new component).

## Reference Documents

- `CLAUDE.md` — project conventions + module map (the `Camera` row gained a "motion lookahead"
  clause this session). Header bumped to **v1.6.172** / package **v0.86.0**.
- `docs/CHANGELOG.md` — the 0.86.0 entry written this session.
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — bumped
  to **seq 119** (the most detailed per-seq record of this session lives in that bullet).
- `../dungeon-merchant/docs/engine-wishlist.md` — the game↔engine board; **ACTIVE EMPTY** (next free
  ID EW-004). Read FIRST next session.

## The Goal

Continue the `breadth-features` pivot — adding genuinely-missing 2D-engine breadth that a downstream
game would otherwise hand-roll. Camera lookahead = bias the camera ahead of a moving follow target,
so the player sees more of where they are going (a side-scroller / top-down staple). The acceptance
test (per the VISION loop) is a small playable example that exercises it in real play.

## Where We Are

- **main @ `c5dc285`** (package **v0.86.0**, CLAUDE.md header **v1.6.172**), tree **clean**.
  _(This handoff doc lands as its own `docs(handoff)` PR after the code PR; the recorded tip will
  move to that handoff merge.)_
- **PR #280 merged** (squash + branch-deleted, CI **5/5** green): `feat(camera): motion lookahead —
  lead the view ahead of a moving follow target (v0.86.0)`.
- **Modified module** `src/camera.rs` — `Camera` gains two pub fields (`lookahead`,
  `lookahead_speed`) + two private fields (`lookahead_offset`, `last_follow_pos`) + a
  `lookahead_offset()` accessor; `Camera::update` got the lookahead computation. Two new consts:
  `DEFAULT_LOOKAHEAD_SPEED` (pub, =3.0) and `LOOKAHEAD_VEL_EPSILON` (private, =1.0).
- **`Camera::update` logic:** inside the existing `if let Some(pos) = follow_pos` block — if
  `lookahead > 0` and there is a `last_follow_pos` and `dt > 0`, compute `velocity = (pos - last)/dt`;
  if `velocity.length() > LOOKAHEAD_VEL_EPSILON`, the target offset is `velocity.normalize() *
  lookahead`, else `Vec2::ZERO` (recenter when stationary). Ease `lookahead_offset` toward the target
  offset by `(lookahead_speed * dt).clamp(0,1)`; store `last_follow_pos = Some(pos)`; aim the follow
  lerp at `pos + lookahead_offset`.
- **New public API** (all additive, non-breaking): `Camera::lookahead`, `Camera::lookahead_speed`
  (pub fields), `Camera::lookahead_offset()` (accessor), `engine::DEFAULT_LOOKAHEAD_SPEED` (re-export).
- **NO `schedule.rs` change.** The lookahead is entirely internal to `Camera` — the App's per-frame
  `Camera::update(dt, follow_pos)` call site (`src/app/schedule.rs:472-487`) passes the follow
  entity's position exactly as before; `Camera` derives the velocity itself. This is cleaner than the
  parent handoff feared ("may need a new field on Camera **or a small system**"): no system, no field
  threading through the App.
- **New example** `examples/camera_lookahead.rs` (flat, auto-discovered) — a player crosses a field
  of posts; a camera-anchor entity is kept at `player - viewport/2` (the platformer.rs top-left
  follow convention) and the camera follows it with `lookahead`; a screen-space center line
  (`DrawRect` → `UiQueue`) shows the lead. Space toggles lookahead; `HEADLESS_SHOT` auto-drifts the
  player right (default 120 frames).
- **Tests:** 3 new unit tests (in `camera::tests`) + 1 new doctest (on the `Camera` struct).
- **CI:** PR #280 passed the 5-job matrix (Test native 4m53s / Build WASM 49s / Render lavapipe
  1m49s / Rustdoc 37s / Package dry-run 1m12s).
- **Headless render verified on Metal:** the screenshot showed the player (moving right) sitting
  **left of the center line**, with the HUD `lookahead: ON (200) offset.x: +200` and `player
  screen-x: 206 (center 380)` — proving the camera leads in the direction of motion.
- **CLAUDE.md module-map** — the `Camera` row gained a "motion lookahead" clause.
- **Memory** `engine-current-state.md` bumped to seq 119.

## What We Tried (Chronological)

1. **Onboarding context carried from seq 3.** The seq-3 handoff already pointed at camera lookahead
   as the next candidate with a design note; the user said "다음 후보 진행해". No fresh
   AskUserQuestion — the candidate was already chosen.
2. **API/usage fact-finding (grep/read, mostly done in seq 3).** Confirmed `Camera::update(dt,
   follow_pos)` does the smooth-follow lerp and is App-driven from `schedule.rs:472-487` (App reads
   the follow entity's `Transform.position` → `cam.update` → `clamp_to_bounds`). Found the **follow
   convention** via `examples/games/platformer/platformer.rs`: a separate **camera-anchor** entity
   is kept at `player_pos - viewport/2` and the camera follows that (so `follow_pos` is the desired
   top-left). `parallax_scroll.rs` uses the same anchor pattern. `DrawRect::new(x,y,w,h,color)` →
   `UiQueue` is screen-space (from `ui_rounded.rs`) — used for the center line.
3. **Designed lookahead to derive velocity from the follow-position delta** (parent's option (a) —
   no game-side wiring, plug-and-play like the existing smooth-follow). Decided it lives entirely in
   `Camera` (no schedule change, no new system).
4. **Edited `src/camera.rs`** — consts, struct fields (2 pub + 2 private), `Default` impl, the
   `update` block, the `lookahead_offset()` accessor, a struct doctest, and 3 unit tests.
5. **Re-exported** `DEFAULT_LOOKAHEAD_SPEED` from `lib.rs` (alongside `Camera`).
6. **Wrote `examples/camera_lookahead.rs`** — post field + player + camera-anchor follow + center
   line + HUD; Space toggle; headless auto-drift.
7. **Updated the CLAUDE.md `Camera` module-map row** with the lookahead clause.
8. **Verify (gate-by-gate).** fmt → clippy `--all-targets` (0) → camera lib tests (40 passed incl.
   the 3 new) + doctest (1) → wasm lib build (0) → rustdoc -D warnings (0) → `test --all-targets`
   skipping the 2 audio tests (0 failures) → **headless render on Metal** (the lead screenshot).
9. **`/ship`** → v0.85.0 → **v0.86.0** (MINOR): Cargo.toml + `cargo update -p skeleton-engine`
   (lock) + CHANGELOG 0.86.0 + CLAUDE.md header v1.6.172.
10. **`/land-pr`** → branch `feat/camera-lookahead`, commit `3de9439`, push, PR **#280**, watched CI
    (5/5 CLEAN), confirmed `mergeStateStatus == CLEAN`, squash-merged `c5dc285`, synced main, bumped
    memory to seq 119.

## Key Decisions

- **Velocity is derived from the follow-position delta, NOT supplied by the game.** `Camera` stores
  `last_follow_pos` and computes `velocity = (pos - last) / dt` each frame. This keeps lookahead as
  plug-and-play as the existing smooth-follow — a game just sets `cam.lookahead = N` and nothing
  else. Rejected: a `CameraLookahead` component carrying an explicit facing/velocity (more flexible
  but requires game-side wiring; the delta approach needs zero).
- **Lookahead is entirely internal to `Camera` — no `schedule.rs` change, no new system.** The App
  already calls `Camera::update(dt, follow_pos)` every frame; the lookahead computation slots into
  that existing call. The parent handoff flagged this might need a system; it did not. This keeps the
  feature self-contained and the diff small.
- **`lookahead == 0` (the default) is byte-identical to the prior direct follow.** When lookahead is
  off, the target offset is `Vec2::ZERO`, the eased `lookahead_offset` stays `Vec2::ZERO` (it starts
  zero and `ZERO + (ZERO - ZERO)*ease == ZERO`), and `aim = pos + ZERO == pos`, so the position lerp
  is unchanged. The only extra work is writing `last_follow_pos` (internal state, no behavioral
  effect). This is why every existing camera test + every `follow_entity` call site is unaffected.
- **`Camera` stays `Copy`.** The new fields are `f32` / `Vec2` / `Option<Vec2>` — all `Copy` — so the
  `#[derive(Debug, Clone, Copy)]` on `Camera` is preserved (it is a `World` resource cloned/copied in
  several places). No `Box`/`Vec`/heap state.
- **`lookahead`/`lookahead_speed` are pub fields (not builder methods).** Consistent with the
  existing `lerp_factor` / `follow_entity` / `bounds` pub fields on `Camera` — a game sets `cam.lookahead
  = 120.0` the same way it sets `cam.lerp_factor = 8.0`. The runtime state (`lookahead_offset`,
  `last_follow_pos`) is private and read via the `lookahead_offset()` accessor.
- **Recenter when stationary.** Below `LOOKAHEAD_VEL_EPSILON` (1.0 world units/sec) the target offset
  is `Vec2::ZERO`, so a stopped target eases the lead back to center — the expected "tweened" feel,
  and it avoids jitter from near-zero velocities (normalizing a tiny vector is unstable).
- **Versioning: MINOR (v0.86.0).** Additive fields + accessor, pre-1.0 → MINOR (same as the prior
  breadth features 0.81.0–0.85.0).
- **Example uses the camera-anchor convention.** Following `platformer.rs`, a camera-anchor entity
  is kept at `player - viewport/2` and the camera follows it, so without lookahead the player is
  centered and with lookahead it shifts toward the trailing edge — making the lead visually obvious
  against a screen-space center line.

## Evidence & Data

### Commit / PR (this session)

| Hash | PR | Version | Seq | Summary |
|---|---|---|---|---|
| `3de9439` (→ squashed `c5dc285`) | #280 | v0.86.0 | 119 | camera motion lookahead — `Camera::lookahead` |

### New public API surface (all additive / non-breaking)

| Symbol | Kind | Location |
|---|---|---|
| `Camera::lookahead` (world units, default 0.0 = off) | pub field | `camera.rs` |
| `Camera::lookahead_speed` (default `DEFAULT_LOOKAHEAD_SPEED`=3.0) | pub field | `camera.rs` |
| `Camera::lookahead_offset()` | accessor | `camera.rs` |
| `DEFAULT_LOOKAHEAD_SPEED` | const | `camera.rs` (re-exported `engine::DEFAULT_LOOKAHEAD_SPEED`) |

Crate-root re-export updated in `lib.rs`: `pub use camera::{Camera, DEFAULT_LOOKAHEAD_SPEED};`.

### Tests added (3 + 1 doctest)

`camera::tests`: `lookahead_off_is_byte_identical_follow` (default lookahead == 0 → position matches
a plain lerp, offset stays zero), `lookahead_leads_in_direction_of_motion` (offset.x > 0 when moving
right, y≈0, magnitude bounded by and approaching `lookahead`), `lookahead_recenters_when_target_stops`
(build a lead by moving, then hold still → offset eases back toward zero). Doctest: enabling
`cam.lookahead` and reading `lookahead_offset()` (zero until it follows a moving target).

### Test counts

`camera::tests` 40 passed (incl. the 3 new); full `cargo test --all-targets` 0 failures (the 2
environmental audio tests skipped locally, green on CI).

### CI (PR #280 — 5-job matrix, all green)

| Job | Result | Time |
|---|---|---|
| Test (native) | pass | 4m53s |
| Build (WASM) | pass | 49s |
| Render tests (lavapipe) | pass | 1m49s |
| Rustdoc | pass | 37s |
| Package dry-run | pass | 1m12s |

### Headless render verification (Metal, `HEADLESS_SHOT=/tmp/camera_lookahead.png`)

- Screenshot: title `Camera lookahead — lead the view ahead of motion`; HUD `lookahead: ON (200)
  offset.x: +200` and `player screen-x: 206 (center 380) — leading when left of center`; the yellow
  player square is clearly left of the yellow center line, with the post field scrolled — the camera
  leads to the right while the player moves right.

Reproduce: `HEADLESS_SHOT=/tmp/camera_lookahead.png cargo run --example camera_lookahead` (native
GPU; works monitor-off via the surfaceless path; `HEADLESS_FRAMES=N` overrides the 120-frame default).

## Code Analysis

- **`Camera::update`** (`src/camera.rs`): the smooth-follow block is now `if let Some(pos) =
  follow_pos { let target_offset = if self.lookahead > 0.0 { match self.last_follow_pos { Some(last)
  if dt > 0.0 => { let velocity = (pos - last) / dt; if velocity.length() > LOOKAHEAD_VEL_EPSILON {
  velocity.normalize() * self.lookahead } else { Vec2::ZERO } } _ => Vec2::ZERO } } else { Vec2::ZERO
  }; let ease = (self.lookahead_speed * dt).clamp(0.0, 1.0); self.lookahead_offset += (target_offset
  - self.lookahead_offset) * ease; self.last_follow_pos = Some(pos); let aim = pos +
  self.lookahead_offset; let factor = (self.lerp_factor * dt).min(1.0); self.position = self.position
  + (aim - self.position) * factor; }`. The zoom-tween and shake-decay blocks below are unchanged.
- **Byte-identical OFF path:** with `lookahead == 0`, `target_offset == ZERO`, and since
  `lookahead_offset` starts `ZERO`, the ease line keeps it `ZERO`; `aim == pos`; the position update
  is identical to the original `self.position + (pos - self.position) * factor`. (`pos + Vec2::ZERO`
  is exact for ordinary coordinates.)
- **`Camera::default`** initializes `lookahead: 0.0`, `lookahead_speed: DEFAULT_LOOKAHEAD_SPEED`,
  `lookahead_offset: Vec2::ZERO`, `last_follow_pos: None`. Struct literals elsewhere use
  `..Default::default()`, so the added fields break no call site.
- **Example follow wiring** (`examples/camera_lookahead.rs`): the `Demo` system moves the player, then
  sets `anchor.position = player_pos - viewport/2`, then sets `cam.lookahead` from the Space toggle.
  The App's post-update phase then reads the anchor → `cam.update` → applies the lookahead. The HUD
  reads `cam.lookahead_offset()` and `player_pos.x - cam.position.x` (the player's screen-x at zoom 1)
  to display the lead numerically. Target x is fixed per post; the player ping-pongs in `[300, 2900]`
  of a 3200-wide world in headless auto mode.

## Gotchas & Discoveries

- **Adding fields to `Camera` is low-risk because it is constructed via `Default` + `..Default::default()`
  everywhere** (and `Camera::new(pos, zoom)` which spreads `..Self::default()`). No struct literal
  names all fields outside the `Default` impl, so additive fields (even private ones) break nothing.
  Keep the new fields `Copy` to preserve `#[derive(Copy)]` on `Camera`.
- **The follow convention is the camera-anchor pattern**, not following the player directly: a game
  keeps a separate anchor at `player - viewport/2` and points `follow_entity` at it (see
  `platformer.rs` / `parallax_scroll.rs`). Following the player entity directly puts it at the
  viewport top-left. Lookahead works either way (it offsets `follow_pos`), but the example uses the
  anchor so the lead reads cleanly against a centered reference.
- **`DrawRect` → `UiQueue` is screen-space** (unaffected by the camera) — the right tool for a center
  reference line over a scrolling world.
- **Environmental audio (standing):** locked/remote macOS has no audio device → 2 audio-device tests
  fail locally; `--skip` them and let CI gate audio. (CI #280's native job ran them green.)
- **zsh `${PIPESTATUS[0]}` is empty** (carried from seq 3) — read exit codes via `echo $?` on an
  unpiped command, or from the background-task completion notification, not `${PIPESTATUS[0]}` after
  a pipe.

## Files Changed

### Source — modified
- `src/camera.rs` — 2 consts, 4 `Camera` fields (2 pub + 2 private), `Default` update, the lookahead
  block in `update`, `lookahead_offset()` accessor, a struct doctest, 3 unit tests.
- `src/lib.rs` — re-export `DEFAULT_LOOKAHEAD_SPEED` alongside `Camera`.

### Examples — new
- `examples/camera_lookahead.rs` — post field + player + camera-anchor follow + screen-space center
  line + HUD; Space toggle; `HEADLESS_SHOT` (120 frames).

### Docs / paperwork
- `CLAUDE.md` — `Camera` module-map row gained a "motion lookahead" clause; header v1.6.171 →
  v1.6.172 / package v0.85.0 → v0.86.0.
- `docs/CHANGELOG.md` — 0.86.0 entry.
- `Cargo.toml`, `Cargo.lock` — version bump + lock refresh.

## User Feedback & Preferences

- **"다음 후보 진행해" → execute the next candidate end-to-end.** After landing hit-flash + its
  handoff, the user directed proceeding to the next breadth feature (camera lookahead) autonomously
  via the land-pr loop.
- **Per-seq handoff cadence** — each code PR is followed by its own `docs(handoff)` PR; the user
  confirmed "1" (write the camera-lookahead handoff + merge it) when offered.
- **Korean for all user-facing replies; English for code/docs/commits/handoffs** (project rule).
- **Merge authority delegated** — squash on green CI, no per-session re-confirm; PR #280 landed
  without asking.
- **Values evidence over assertion** — the headless render screenshot (with the numeric HUD lead)
  was sent to the user as the acceptance artifact, alongside CI numbers and test counts.

## Where We're Going

The `breadth-features` chain has now shipped **six** features this run (SpriteFlip, YSort,
AnimationEvents, TriggerZone, HitFlash, CameraLookahead). **Read
`../dungeon-merchant/docs/engine-wishlist.md` FIRST each session** (ACTIVE EMPTY, EW-004 next) — a
real downstream request outranks self-picked breadth. **The self-pick candidate list is now thin:**

1. **Editor i18n gap** (lower value) — `editor/ui/audio_panel.rs` (and possibly other editor panels)
   have English status strings that bypass `i18n::tr(en, ko)`, so they stay English in the
   Korean-default editor. A small, editor-only polish; not CI-render-verifiable (editor/native-only).
2. **A Tier-2 hardcoding knob on a concrete request** — the remaining knobs from the
   `hardcoding-audit` are weaker/compromised (MAX_GAMEPADS needs >4 physical pads; material params >4
   breaks the WGSL `vec4` contract; editor app-id is editor-only; frame latency is
   CI-unverifiable). Do one only if a downstream game asks.

Otherwise: **ASK the user for direction** when the board is empty — the obvious additive
component+system breadth features have largely been covered. Bigger next steps would be deeper
(data-driven trigger zones / RON-authored effects, a particle-on-event helper, a richer camera rig)
or a genuine downstream wishlist item.

**The validated 8-step pattern (now applied to a component-on-existing-resource variant):** camera
lookahead departs from the pure "new component + new user-added system" shape — it adds fields to an
existing resource (`Camera`) updated by an existing App call. The lesson: for an additive change to a
widely-constructed `Copy` resource, keep new fields `Copy`, default the OFF path to byte-identical,
and verify the existing tests stay green untouched.

## Risks & Blockers

- **Audio-device tests fail locally** (environmental, not a regression) — `--skip` the two named
  audio tests. CI gates audio.
- **Lookahead derives velocity from the follow-position delta**, so a teleport (a large single-frame
  jump in `follow_pos`) produces a one-frame large velocity → a brief lead spike that eases out. For
  a scene cut, set `cam.lookahead = 0` (or snap `cam.position`) around the teleport. Benign for normal
  movement; not handled specially.
- **No OS-gated code this session** — everything is cross-platform (the lavapipe render job exercised
  the GPU path; lookahead is pure math on the `Camera` resource).
- No upstream/dependency blockers. Tree clean.

## Open Questions

- Should lookahead support a **vertical bias** independent of horizontal (some platformers lead more
  horizontally than vertically)? Currently it leads uniformly in the velocity direction. A per-axis
  `Vec2` lookahead could be added if a game needs it.
- Should there be a **deadzone** (no lead until the target moves past a threshold) in addition to the
  speed epsilon? Kept simple; the speed epsilon already suppresses jitter.
- Should the velocity be **smoothed** (it is currently the raw per-frame delta, and only the offset
  is eased)? The offset easing already smooths the visible result; raw velocity is fine.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5            # confirm main tip (camera lookahead #280 + this handoff PR)
git status -s                   # clean

# 1) Read the game↔engine board FIRST (drives priority)
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick breadth

# Memory (full per-seq detail of this session)
#   ~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md (seq 119 tip)

# Key files if continuing breadth features
#   src/camera.rs (Camera::update / lookahead)    — the pattern just shipped (fields on an existing resource)
#   src/app/schedule.rs:472-487                    — App-driven Camera::update call site (unchanged this seq)
#   src/hit_flash.rs / src/trigger_zone.rs / src/ysort.rs  — prior breadth patterns (new component + system)
#   src/app/editor/ui/audio_panel.rs               — the editor i18n gap (a remaining low-value candidate)

# Verify (NOTE: 2 audio-device tests fail locally — environmental; read exit via `echo $?`, not ${PIPESTATUS[0]} in zsh)
cargo test --lib -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Reproduce this session's example
HEADLESS_SHOT=/tmp/camera_lookahead.png cargo run --example camera_lookahead

# Next action
#   Read the wishlist board; if still empty, ASK which direction (the easy self-pick breadth is largely done).
```

## Session Closed

**Closed:** 2026-06-29
**Chain:** `breadth-features` seq 4 — continuation of `HANDOFF_breadth-features_hit-flash_2026-06-29.md` (seq 3)
**Code landed:** #280 (v0.86.0), main @ `c5dc285`. This handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off. Three breadth features shipped this session (hit-flash, then camera lookahead, each with its handoff PR). Next session starts from the wishlist board or asks for direction.
