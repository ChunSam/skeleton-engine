# Hit-flash — `HitFlash` + `HitFlashSystem` shipped (v0.85.0, PR #278), third breadth feature of the chain

**Date:** 2026-06-29
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `breadth-features` seq `3`
**Parent:** `HANDOFF_breadth-features_trigger-zones_2026-06-29.md` (seq 2)
**Prior chain:** `hardcoding-audit` (closed) → pivoted to `breadth-features` at its seq 4

> Chain rationale: this session was started by a paste prompt pointing at the `breadth-features`
> seq-2 handoff (trigger zones) and told to "continue from Where We're Going". That section listed
> **hit-flash as the #1 recommended next candidate** (the parent's #2 overall) — easiest, pairs with
> the new `ZoneEvent` damage zones, fully CI-verifiable. The user chose the recommended action. So
> this is a **direct continuation**: `breadth-features` seq 3, parent = the seq-2 handoff.

## Related Handoffs

- `HANDOFF_breadth-features_trigger-zones_2026-06-29.md` — **the parent** (`breadth-features` seq 2,
  #276 trigger zones). Its "Where We're Going" recommended hit-flash first and noted it pairs with
  damage `ZoneEvent`s. Read it for the validated 8-step breadth pattern and the `TriggerZone`
  no-serde precedent that hit-flash reused.
- `HANDOFF_breadth-features_animation-events_2026-06-29.md` — `breadth-features` seq 1 (#274). The
  serde-deriving variant of the same component+system+example pattern.

## Reference Documents

- `CLAUDE.md` — project conventions + module map (a `HitFlash` row was added under the `YSort` row
  this session). Header bumped to **v1.6.171** / package **v0.85.0**.
- `docs/CHANGELOG.md` — the 0.85.0 entry written this session.
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — bumped
  to **seq 118** (the most detailed per-seq record of this session lives in that bullet).
- `../dungeon-merchant/docs/engine-wishlist.md` — the game↔engine board; **ACTIVE EMPTY** (next free
  ID EW-004). Read FIRST next session.

## The Goal

Continue the `breadth-features` pivot — adding genuinely-missing 2D-engine breadth that a downstream
game would otherwise hand-roll. The wishlist board was empty, so per the parent's plan I narrated
the 5-point onboarding, recommended hit-flash, and the user replied "권장하는 행동으로 진행"
(proceed with the recommended action). Hit-flash = the game-feel staple every action game
re-implements: a sprite pops to a flash color (usually white) for a fraction of a second when hit,
then eases back. The acceptance test (per the VISION loop) is a small playable example that
exercises it in real play.

## Where We Are

- **main @ `9224697`** (package **v0.85.0**, CLAUDE.md header **v1.6.171**), tree **clean**.
  _(This handoff doc lands as its own `docs(handoff)` PR after the code PR; the recorded tip will
  move to that handoff merge.)_
- **PR #278 merged** (squash + branch-deleted, CI **5/5** green): `feat(render): hit-flash —
  HitFlash component + HitFlashSystem (v0.85.0)`.
- **New module** `src/hit_flash.rs` — `HitFlash { color, secs, <private> elapsed, base }` (component,
  ctors `white`/`new`, read accessors `progress`/`is_finished`), `HitFlashSystem`.
- **`HitFlashSystem` logic:** `query2_mut::<HitFlash, Sprite>()`; on the **first** run per flash it
  captures the sprite's current color into `base` (`Option<Color>` via `get_or_insert`), advances
  `elapsed += dt`, sets `sprite.color = Color::lerp(flash.color, base, progress())` (progress 0 →
  full flash color, 1 → original restored), and collects finished entities; after the loop it
  `remove_component::<HitFlash>` each finished entity (no borrow conflict — removal is a separate
  `&mut World` pass after the query iterator is dropped).
- **New public API** (all additive, non-breaking): `engine::HitFlash`, `engine::HitFlashSystem`
  (crate-root re-exports in `lib.rs`).
- **Registration:** `register_clone::<HitFlash>` in `core_resources.rs`; editor add/remove in
  `editor/component_registry.rs` — mirrors `YSort`/`TriggerZone` (clone + editor add/remove). **NOTE:**
  like `TriggerZone` (and unlike YSort/SpriteFlip/AnimationEvents), `HitFlash` does **NOT** derive
  `Serialize`/`Deserialize` — it is a transient runtime effect (a flash in progress), so persisting
  it to a scene is meaningless. clone + editor registration needs only `Clone` + `Default`.
- **New example** `examples/hit_flash.rs` (flat, auto-discovered) — three resting-colored targets;
  **Space** flashes all three white and they fade back. Headless auto-re-flashes each target as soon
  as its previous flash finishes, on **staggered durations** (0.16/0.22/0.28 s) so the targets fall
  out of phase and a capture always lands with at least one mid-flash. `HEADLESS_SHOT` support
  (default 90 frames).
- **Tests:** 6 new unit tests (all in `hit_flash::tests`) + 2 doctests (module `no_run` + the
  `HitFlash` struct doctest).
- **CI:** PR #278 passed the 5-job matrix (Test native 5m24s / Build WASM 46s / Render lavapipe
  1m58s / Rustdoc 50s / Package dry-run 1m15s). The native job ran the audio tests green (confirming
  the 2 local failures are environmental); the lavapipe render job gates the GPU path.
- **Headless render verified on Metal:** the screenshot showed the three targets at different fade
  phases (`ember` nearly back to its orange base, `tide`/`violet` washed toward white) and the HUD
  `flashes fired: 22` — proving the flash-then-fade + continuous re-flash both work.
- **CLAUDE.md module-map** got a `HitFlash` row under the `YSort` row.
- **Memory** `engine-current-state.md` bumped to seq 118.

## What We Tried (Chronological)

1. **Onboarding (narrated, 5-point).** Read the parent handoff (breadth-features seq 2), confirmed
   the wishlist board is ACTIVE EMPTY. Read the listed key files: `trigger_zone.rs`, `ysort.rs`, plus
   the suggested adjacent files `camera.rs` (lookahead candidate), `components.rs` (`Sprite.color`
   for hit-flash), and `schedule.rs:472-487` (the App-driven `Camera::update` call site, relevant to
   the lookahead candidate). Recommended hit-flash; user said "권장하는 행동으로 진행".
2. **Baseline** (`971 passed`, 2 audio filtered) — matched the parent handoff.
3. **API fact-finding (grep/read).** `Color: Default` (= WHITE) / `Copy` / `Lerp` (component-wise
   rgba lerp in `tween.rs:134`); `query2_mut::<A,B>() -> (Entity, &mut A, &mut B)`;
   `remove_component::<T>(&mut self, entity)`; exact registration sites in `core_resources.rs` (after
   the `TriggerZone` `register_clone`) and `editor/component_registry.rs` (after the `TriggerZone`
   add + remove); `lib.rs` alphabetical insertion points (`hit_flash` between `history` and `input`
   in both the `pub mod` list and the `pub use` list).
4. **Wrote `src/hit_flash.rs`** — the component + the system + 6 unit tests + 2 doctests. Chose
   lazy base-capture (on first run) and private `elapsed`/`base`.
5. **Wired** `lib.rs` (`pub mod hit_flash` + crate-root re-export), `core_resources.rs`
   (`register_clone`), `editor/component_registry.rs` (add + remove).
6. **Wrote `examples/hit_flash.rs`** — three targets, Space-to-flash, headless continuous-re-flash.
   Fixed two compile issues found via rust-analyzer diagnostics: (a) a dead `rest: Color` field on
   `Target` (clippy dead_code) → removed; (b) a borrow conflict in the HUD block (`world.get` inside
   a `world.resource_mut::<TextQueue>()` borrow) → stored the target `x` on `Target` at spawn so the
   label loop reads `t.x` instead of re-querying the world.
7. **Compiled + tested incrementally.** 6 unit tests pass; doctests pass; clippy `--all-targets`
   green; **headless render on Metal** produced the staggered-phase screenshot.
8. **CLAUDE.md module-map row** added under `YSort`.
9. **Verify (gate-by-gate).** fmt → clippy `--all-targets` (0) → wasm lib build (0) → `test
   --all-targets` skipping the 2 audio tests (0 failures, +6 tests +2 doctests) → rustdoc -D
   warnings (0) → headless render. All green.
10. **`/ship`** → v0.84.0 → **v0.85.0** (MINOR): Cargo.toml + `cargo update -p skeleton-engine`
    (lock) + CHANGELOG 0.85.0 + CLAUDE.md header v1.6.171. Re-ran the fast gates post-bump (fmt 0,
    clippy 0, wasm 0, rustdoc 0).
11. **`/land-pr`** → branch `feat/hit-flash`, commit `9509b50`, push, PR **#278**, watched CI (5/5
    CLEAN), confirmed `mergeStateStatus == CLEAN`, squash-merged `9224697`, synced main, bumped
    memory to seq 118.

## Key Decisions

- **The pre-flash color is captured on the FIRST system run, not at construction.** `base:
  Option<Color>` starts `None`; the system does `*flash.base.get_or_insert(sprite.color)` on its
  first tick. This means a game just adds `HitFlash::white(0.15)` without telling it the sprite's
  color. Cost: re-adding a `HitFlash` while one is still fading captures the **mid-flash** color as
  the new base (documented). Rejected: storing the base at construction (forces the caller to know
  the color) or a separate "register base color" call (clunky).
- **`elapsed` and `base` are PRIVATE; `color` and `secs` are public.** The authoring inputs are
  public (set them directly); the runtime state is private and read via `progress()` / `is_finished()`.
  Keeps the component self-documenting about what a game writes vs reads.
- **The system fades back to the captured base and removes itself.** At `progress() == 1.0`,
  `Color::lerp(flash.color, base, 1.0) == base` exactly, so the last frame restores the original
  color before removal — no separate restore step, no one-frame color glitch. Rejected: holding the
  flash color for `secs` then snapping back (looks worse; the fade reads as a proper hit-flash).
- **`HitFlash` does NOT derive serde** (departs from YSort/SpriteFlip/AnimationEvents, which do;
  follows the `TriggerZone` precedent). It is a transient in-progress effect — saving a half-faded
  flash to a scene is meaningless. clone + editor registration needs only `Clone` + `Default`. If a
  game ever wants to persist a "flash on load", it can re-add the component on spawn instead.
- **Removal is a separate pass after the query.** You cannot `remove_component` while iterating
  `query2_mut` (the iterator borrows the archetype). So the system collects finished entities into a
  `Vec<Entity>` during the loop and removes them after — the same shape as other systems that mutate
  structure mid-iteration.
- **`secs <= 0` restores+removes the same frame.** `progress()` returns `1.0` when `secs <= 0`
  (avoids divide-by-zero), so a zero-duration flash sets the color to base and is finished
  immediately — a safe degenerate case, not a panic.
- **A `HitFlash` on an entity without a `Sprite` is inert** (and never removed — `query2_mut`
  requires both components, so it is simply never visited). Documented; harmless.
- **Versioning: MINOR (v0.85.0).** Additive feature, pre-1.0 → MINOR (same as the prior breadth
  features: SpriteFlip 0.81.0 / YSort 0.82.0 / AnimationEvents 0.83.0 / TriggerZone 0.84.0).
- **Example: continuous re-flash on staggered durations in headless mode.** With no input, the demo
  re-arms any target whose flash just finished, using three different durations so the targets drift
  out of phase — guaranteeing the single captured frame catches at least one mid-flash (a robust,
  deterministic capture, vs trying to phase one flash to land on the last frame). `HitFlashSystem`
  runs **before** the demo so a just-finished flash is re-added the same frame it ends (no gap).

## Evidence & Data

### Commit / PR (this session)

| Hash | PR | Version | Seq | Summary |
|---|---|---|---|---|
| `9509b50` (→ squashed `9224697`) | #278 | v0.85.0 | 118 | hit-flash — `HitFlash` + `HitFlashSystem` |

### New public API surface (all additive / non-breaking)

| Symbol | Kind | Location |
|---|---|---|
| `HitFlash { color, secs }` (+ private `elapsed`/`base`) | component | `hit_flash.rs` |
| `HitFlash::white(secs)` / `new(color, secs)` | ctors | `hit_flash.rs` |
| `HitFlash::progress()` / `is_finished()` | read accessors | `hit_flash.rs` |
| `HitFlashSystem` | system (user-added) | `hit_flash.rs` |

Crate-root re-exports added to `lib.rs`: `pub use hit_flash::{HitFlash, HitFlashSystem};` (between
the `history` and `input` re-exports).

### Tests added (6 + 2 doctests)

`hit_flash::tests`: `fades_from_flash_color_back_to_base_then_removes` (half-fade color check + full
restore + auto-remove), `flash_color_dominates_at_the_start` (near-white at t≈0), `base_is_captured
_at_first_run_not_construction` (green sprite fades back to green), `zero_duration_restores
_immediately_and_removes`, `sprite_without_hitflash_is_untouched`, `progress_and_is_finished
_accessors`. Doctests: a module `no_run` (`app.add_system(HitFlashSystem)`) + a `HitFlash` struct
doctest (`white(0.2)` field/accessor checks).

### Test counts

`971 passed` (session start) → `+6` new (in `hit_flash::tests`) + 2 doctests, 2 environmental audio
tests skipped/filtered locally, all green on CI.

### CI (PR #278 — 5-job matrix, all green)

| Job | Result | Time |
|---|---|---|
| Test (native) | pass | 5m24s |
| Build (WASM) | pass | 46s |
| Render tests (lavapipe) | pass | 1m58s |
| Rustdoc | pass | 50s |
| Package dry-run | pass | 1m15s |

### Headless render verification (Metal, `HEADLESS_SHOT=/tmp/hit_flash.png`)

- Screenshot: title `HitFlash — flash a sprite when it is hit, then fade back`; HUD `flashes fired:
  22`; three squares — `ember` (orange, near its base = late in its cycle), `tide` (near-white =
  early in flash), `violet` (near-white = early in flash), each labeled; footer `Space: hit (flash)
  the targets   Esc: quit`. The differing phases prove the fade + the staggered re-flash.

Reproduce: `HEADLESS_SHOT=/tmp/hit_flash.png cargo run --example hit_flash` (native GPU; works
monitor-off via the surfaceless path; `HEADLESS_FRAMES=N` overrides the 90-frame default).

## Code Analysis

- **`HitFlashSystem::run`** (`src/hit_flash.rs`): `let mut finished = Vec::new();` then `for (e,
  flash, sprite) in world.query2_mut::<HitFlash, Sprite>() { let base =
  *flash.base.get_or_insert(sprite.color); flash.elapsed += dt; sprite.color =
  Color::lerp(&flash.color, &base, flash.progress()); if flash.is_finished() { finished.push(e); } }`
  then `for e in finished { world.remove_component::<HitFlash>(e); }`.
- **Borrow note:** `flash` (`&mut HitFlash`) and `sprite` (`&mut Sprite`) are distinct bindings from
  the same tuple → no aliasing. `flash.base.get_or_insert(sprite.color)` reads a `Copy` field of
  `sprite` while mutating `flash` — fine. Removal happens after the iterator is dropped (a separate
  `&mut World` pass), so there is no archetype-borrow conflict.
- **`HitFlash::progress`** returns `(elapsed/secs).clamp(0,1)` when `secs > 0`, else `1.0` — the
  divide-by-zero guard that also makes a zero-duration flash finish immediately.
- **Registration template** (confirmed by grep): `RenderLayer`/`YSort`/`TriggerZone` = `register_clone`
  + editor add/remove. `HitFlash` follows that minus serde (see Key Decisions).
- **Example movement/trigger model:** the `Demo` system takes `auto: bool` (= `HEADLESS_SHOT` is
  set). In live mode, `Space` (just_pressed) adds `HitFlash::white(0.18)` to all targets. In auto
  mode, each frame it re-arms any target whose `HitFlash` is absent, using that target's `auto_secs`
  (staggered). Target `x` is stored on the `Target` struct at spawn so the HUD label loop never
  re-queries the world inside the `TextQueue` borrow.

## Gotchas & Discoveries

- **NEW — zsh `${PIPESTATUS[0]}` is EMPTY.** The shell is zsh, where the array is `$pipestatus` and
  is **1-indexed** — `${PIPESTATUS[0]}` (bash-style, 0-indexed) yields an empty string. Several
  `echo "X_EXIT=${PIPESTATUS[0]}"` lines this session printed `X_EXIT=` with no number. Read exit
  codes via `echo $?` on an **unpiped** command, or from the **background-task completion
  notification** (which reports the real exit code), or use zsh `${pipestatus[1]}`. This reinforces
  the existing project rule "read the gate's real exit code, don't pipe it" — and the safest pattern
  here is `cmd > /tmp/x.log 2>&1; echo $?` (no pipe) or a `run_in_background` task.
- **`cargo test --all-targets` exceeds a 2-minute foreground timeout** — it compiles every example
  as its own test binary (dozens). Run it with `run_in_background: true` (it took ~6 min wall here)
  and grep the output file for `FAILED`/`panicked` + the per-binary `test result:` lines.
- **rust-analyzer diagnostics flagged two REAL issues** amid the usual false positives (the
  `ColliderHandle "expected X found X"` + `cfg(wasm32)` inactive-code noise): the dead `rest` field
  and the HUD borrow conflict. Worth scanning the new-diagnostics block for the non-boilerplate ones
  before running cargo.
- **Environmental audio (standing):** locked/remote macOS has no audio device → `play_tone_reports
  _playing_then_finished_when_audio_device_exists` + `stop_on_drained_sink_is_immediate` always fail
  locally; never a regression. `--skip` them and let CI gate audio. (CI #278's native job ran them
  green.)
- **`Color` already implements everything hit-flash needed** — `Default` (WHITE), `Copy`, and `Lerp`
  (component-wise rgba in `tween.rs`). No new color plumbing.

## Files Changed

### Source — new
- `src/hit_flash.rs` — `HitFlash` (+ ctors/accessors) + `HitFlashSystem` + 6 tests + 2 doctests.

### Source — modified
- `src/lib.rs` — `pub mod hit_flash;` + crate-root re-export of `HitFlash`, `HitFlashSystem`.
- `src/app/core_resources.rs` — `register_clone::<HitFlash>`.
- `src/app/editor/component_registry.rs` — editor add + remove for `HitFlash`.

### Examples — new
- `examples/hit_flash.rs` — Space flashes 3 targets white, they fade back; headless auto-re-flash on
  staggered durations; `HEADLESS_SHOT` (90 frames).

### Docs / paperwork
- `CLAUDE.md` — `HitFlash` module-map row under `YSort`; header v1.6.170 → v1.6.171 / package
  v0.84.0 → v0.85.0.
- `docs/CHANGELOG.md` — 0.85.0 entry.
- `Cargo.toml`, `Cargo.lock` — version bump + lock refresh.

## User Feedback & Preferences

- **Narrate the onboarding, then execute on go-ahead.** This session did the 5-point onboarding +
  recommended hit-flash; the user replied "권장하는 행동으로 진행" (proceed with the recommended
  action) → full land-pr loop run autonomously.
- **After landing, the user asked to write the handoff + merge it AND proceed with the next
  candidate** ("핸드오프 작성하고 pr 머지 해. 다음 후보 진행해") — i.e. the per-seq handoff PR cadence
  continues, and work proceeds to the next breadth feature (camera lookahead) in the same session.
- **Korean for all user-facing replies; English for code/docs/commits/handoffs** (project rule).
- **Merge authority delegated** — squash on green CI, no per-session re-confirm; PR #278 landed
  without asking.
- **Prefers the full land-pr loop per change** — branch → verify → /ship → PR → watch CI →
  squash-merge → sync → bump memory seq — run without narrating each option.
- **Values evidence over assertion** — verification (headless render screenshot, CI numbers, test
  counts) reported with real numbers; the screenshot was sent to the user as the acceptance artifact.

## Where We're Going

The `breadth-features` chain continues until the wishlist board fills. **Read
`../dungeon-merchant/docs/engine-wishlist.md` FIRST each session** (ACTIVE EMPTY, EW-004 next) — a
real downstream request outranks self-picked breadth. The user has already directed the **next
candidate this session**:

1. **Tweened camera lookahead** (the parent's #3; the last self-pick candidate) — bias the camera
   ahead of a moving follow target so the player sees more of where they are going. `Camera::update(dt,
   follow_pos)` (`src/camera.rs`) does the smooth-follow lerp; it is **App-driven** from
   `src/app/schedule.rs:472-487` (App reads the follow entity's `Transform.position` → `cam.update`
   → `clamp_to_bounds`). A lookahead needs a new `Camera` field (e.g. `lookahead: f32` + a smoothed
   internal offset) and a velocity/facing source. **Design question to resolve first:** where does
   the target's velocity come from? The camera only gets a position each frame. Options: (a) the
   camera derives velocity from the delta of successive `follow_pos` values (store `last_follow_pos`
   on `Camera`); (b) a new `CameraLookahead` component on the target carrying an explicit
   facing/velocity the game already knows. Option (a) is the most plug-and-play (no game-side wiring,
   same as smooth-follow today) and is the recommended starting point — derive velocity from the
   follow-position delta, smooth it, and offset the follow target by `velocity_dir * lookahead`.
   Acceptance test: a side-scroller-ish example where the camera leads the moving player; `HEADLESS_SHOT`.
2. Lower value: the editor i18n gap (`editor/ui/audio_panel.rs` — English status strings bypass
   `tr()` in the Korean-default editor), or a deferred Tier-2 hardcoding knob on a concrete request.

**The validated 8-step pattern (now 5× — SpriteFlip, YSort, AnimationEvents, TriggerZone, HitFlash):**
(a) grep to confirm the gap + read the subsystem; (b) add a component (+ system/event if needed) in
its own module; (c) register clone + editor add/remove like `RenderLayer`/`YSort`; (d) re-export in
`lib.rs`; (e) a flat `examples/<name>.rs` with `HEADLESS_SHOT` (self-drive in headless if
interactive); (f) unit tests + doctest; (g) CLAUDE.md module-map row; (h) land via the land-pr loop
(MINOR), `--skip` the 2 audio tests locally + read exit codes via `echo $?` (not `${PIPESTATUS[0]}`
in zsh). **Camera lookahead departs slightly** — it modifies an existing resource (`Camera`) + the
App call site (`schedule.rs`) rather than adding a fresh user-added system, so it is NOT pure-additive
in the same way; treat the `Camera::update` signature/field change carefully (it is widely called in
tests).

## Risks & Blockers

- **Audio-device tests fail locally** (environmental, not a regression) — `--skip play_tone_reports
  _playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate`. CI gates
  audio.
- **Re-adding `HitFlash` mid-flash** (by design) captures the mid-flash color as the new base — a
  re-trigger while still fading won't return to the true original. Benign + documented; a game that
  re-flashes rapidly should let the flash finish or set the sprite color before re-adding.
- **`Stayed`-style per-frame concerns do not apply** — hit-flash is one-shot and self-removing.
- **No OS-gated code this session** — everything is cross-platform (the lavapipe render job
  exercised the GPU path). The standing rule still holds for future work: green CI ≠ verified for
  `cfg(target_os)` paths.
- **For the NEXT feature (camera lookahead):** changing `Camera::update`'s signature or adding a
  field touches many camera unit tests in `src/camera.rs`; prefer an additive field with a sensible
  default so existing tests/`Camera::default()` call sites stay green.

## Open Questions

- Should hit-flash support a **flash curve / easing** (e.g. ease-out) instead of a linear `Lerp`?
  Kept linear (matches the engine's other simple effects); a future `Easing` field could gate it.
- Should there be a **flash-and-hold** variant (snap to color, hold, then fade) for heavier hits?
  Kept the single fade-from-color model; extend if a game needs it.
- Should `HitFlash` be **data-driven** (authored in RON)? It is transient runtime state, so probably
  not — a game adds it in code on a hit event. Out of scope.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4            # confirm main tip (hit-flash #278 + this handoff PR)
git status -s                   # clean

# 1) Read the game↔engine board FIRST (drives priority)
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick breadth

# Memory (full per-seq detail of this session)
#   ~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md (seq 118 tip)

# Key files if continuing breadth features
#   src/hit_flash.rs                              — the pattern just shipped (component + system, no serde)
#   src/camera.rs (Camera::update / lerp_factor)  — for "camera lookahead" (NEXT candidate)
#   src/app/schedule.rs:472-487                   — App-driven Camera::update call site (lookahead wires here)
#   src/ysort.rs + src/trigger_zone.rs            — prior breadth patterns

# Verify (NOTE: 2 audio-device tests fail locally — environmental; read exit via `echo $?`, not ${PIPESTATUS[0]} in zsh)
cargo test --lib -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Reproduce this session's example
HEADLESS_SHOT=/tmp/hit_flash.png cargo run --example hit_flash

# Next action (already directed this session)
#   Implement tweened camera lookahead (see "Where We're Going") + land via the land-pr loop.
```

## Session Closed

**Closed:** 2026-06-29
**Chain:** `breadth-features` seq 3 — continuation of `HANDOFF_breadth-features_trigger-zones_2026-06-29.md` (seq 2)
**Code landed:** #278 (v0.85.0), main @ `9224697`. This handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off. The user directed continuing to the next candidate (tweened camera lookahead) in the same session.
