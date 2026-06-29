# Data-driven Animation→Effect bindings + shared `effect` module — `AnimEffectBindings` / `AnimEffectSystem` / `engine::effect` shipped (v0.89.0, PR #287), seventh breadth feature of the chain

**Date:** 2026-06-29
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `breadth-features` seq `7`
**Parent:** `HANDOFF_breadth-features_zone-effects_2026-06-29.md` (seq 6)
**Prior chain:** `hardcoding-audit` (closed) → pivoted to `breadth-features` at its seq 4

> Chain rationale: the seq-6 handoff (data-driven Zone→Effect bindings) named, as the natural next
> data-driven step, an **`AnimationEvent`→effect binding reusing the `Effect` enum**. The user chose
> exactly that ("AnimationEvent→effect 바인딩 진행해"). So this is a direct continuation:
> `breadth-features` seq 7, parent = the seq-6 handoff.

## Related Handoffs

- `HANDOFF_breadth-features_zone-effects_2026-06-29.md` — **the parent** (`breadth-features` seq 6,
  #285 Zone→Effect bindings). It introduced the `Effect` vocabulary this feature reuses, and
  explicitly flagged "the `Effect` enum is standalone so an `AnimationEffectSystem` keyed by
  `AnimationEvent.tag` would reuse it directly."
- `HANDOFF_breadth-features_animation-events_2026-06-29.md` — `breadth-features` seq 1 (#274), the
  `AnimationEvents` + `AnimationEvent` feature this reacts to (and whose example layout the new
  `anim_effects` example mirrors — the same procedurally-generated walk sheet).

## Reference Documents

- `CLAUDE.md` — the Zone→Effect row was updated to point at the shared `src/effect.rs`, and a new
  **Animation→Effect bindings** row added. Header bumped to **v1.6.176** / package **v0.89.0**.
- `docs/CHANGELOG.md` — the 0.89.0 entry (Added + Changed-internal) written this session.
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — bumped
  to **seq 122** (the most detailed per-seq record of this session lives in that bullet).
- `../dungeon-merchant/docs/engine-wishlist.md` — the game↔engine board; **ACTIVE EMPTY** (next free
  ID EW-004). Read FIRST next session.
- `src/effect.rs` (NEW shared module), `src/zone_effect.rs` (refactored reference), `src/ron_registry.rs`
  / `src/particle/config_set.rs` (the data-driven config template).

## The Goal

Continue the `breadth-features` chain with the seq-6 handoff's candidate-1: an
**`AnimationEvent`→effect binding** that reuses the `Effect` vocabulary. A game can already author a
zone's reactions in RON (seq 6); now it can author an animation's reactions — footstep dust on a
contact frame, a swing whoosh — keyed by the frame's tag. The acceptance test (per the VISION loop)
is a small playable example that fires RON-authored effects on tagged animation frames.

## Where We Are

- **main @ `d579c5e`** (package **v0.89.0**, CLAUDE.md header **v1.6.176**), tree **clean**.
  _(This handoff doc lands as its own `docs(handoff)` PR after the code PR; the recorded tip will
  move to that handoff merge.)_
- **PR #287 merged** (squash + branch-deleted, CI **5/5** green): `feat(anim_effect): data-driven
  Animation→Effect bindings + shared effect module (v0.89.0)`.
- **New shared module** `src/effect.rs` (219 lines) — `Effect` + `EffectAnchor` (pub, re-exported at
  crate root) + `pub(crate)` `PendingEffect` / `lookup_emitter` / `resolve_effect` / `apply_pending`.
- **New module** `src/anim_effect.rs` (359 lines incl. tests) — `AnimEffectBindings`,
  `AnimEffectRegistry`, `AnimEffectError`, `AnimEffectSystem` + 5 unit tests + 1 doctest.
- **`src/zone_effect.rs` refactored** (−225 lines): `Effect`/`EffectAnchor`/the apply machinery
  moved out to `crate::effect`; `ZoneEffectSystem::run` is now thin (resolve_effect + apply_pending).
  **Behavior-preserving** — its 5 tests are unchanged and green.
- **New public API** (all additive, non-breaking): `engine::AnimEffectBindings`,
  `engine::AnimEffectRegistry`, `engine::AnimEffectError`, `engine::AnimEffectSystem` (crate-root
  re-exports), plus `App::load_anim_effects(name, path)`. `engine::Effect`/`engine::EffectAnchor`
  keep their crate-root paths (now re-exported from `effect` instead of `zone_effect` — same name,
  non-breaking).
- **`AnimEffectBindings { bindings: HashMap<String, Vec<Effect>> }`** — `from_ron_str(s)`,
  `effects_for(tag) -> &[Effect]`, `len`/`is_empty`. The value per tag is a **bare `Vec<Effect>`** —
  no phase wrapper, because an animation event is an instantaneous "this frame was entered" fire
  (unlike a zone's enter/stay/exit). RON: `(bindings: { "footstep": [ SpawnParticles(...), ... ] })`.
- **`AnimEffectRegistry`** wraps `RonRegistry<AnimEffectBindings>`; auto-registered as hot-reloadable
  in `App::new` alongside the particle/dialogue/trigger-zone/zone-effect registries.
- **`App::load_anim_effects`** mirrors `load_zone_effects` exactly (lazy-insert + `register_persistent`
  + native `reg.load` + generic `watch_path`; wasm no-op).
- **`AnimEffectSystem::new(name)`** reads `Events<AnimationEvent>`, keys on `AnimationEvent.tag`
  **directly** (no `Tag`-component lookup — the tag is on the event), resolves each effect via the
  shared `resolve_effect` anchoring both `SpawnParticles` and `Flash` at the animating `entity`, then
  `apply_pending`. Add it **after** `AnimationSystem`.
- **New example** `examples/anim_effects.rs` (+ 2 RON files) — the seq-1 walk cycle, whose two
  contact frames (1, 3) carry a `"footstep"` event, fires a dust burst + a click tone + a warm flash,
  all authored in RON. `HEADLESS_SHOT` support (default 70 frames).
- **Tests:** 5 new `anim_effect` unit tests + 1 new doctest; `zone_effect`'s 5 unchanged.
- **CI:** PR #287 passed the 5-job matrix (Test native 5m26s / Build WASM 46s / Render lavapipe
  1m32s / Rustdoc 45s / Package dry-run 1m7s).
- **Headless render verified on Metal:** console fired `footstep → fired its effects (total: 1/2)`;
  the screenshot showed the walker mid-cycle with a clear **dust burst** scattered around it (the RON
  `SpawnParticles("dust")` effect), HUD `current frame: 0   footsteps: 2`.
- **CLAUDE.md module-map** — Zone→Effect row updated (points at `src/effect.rs`), new Animation→Effect
  row added.
- **Memory** `engine-current-state.md` bumped to seq 122.

## What We Tried (Chronological)

1. **Confirmed the board + took the user's pick.** The seq-6 handoff named `AnimationEvent`→effect as
   candidate-1; the user said "AnimationEvent→effect 바인딩 진행해".
2. **Grounded the example in real signatures** via an `Explore` agent: the exact `animation_events.rs`
   setup (procedural 4×1 walk sheet via `generate_walk_sheet`, `load_image`, `AnimationPlayer::new`
   + `AnimationClip { frames, fps, looping }`, `AnimationEvents::new().on(clip, frame, tag)`,
   `Sprite::textured_with_handle`, `register_event::<AnimationEvent>`, `AnimationSystem::new()` FIRST,
   `HEADLESS_SHOT`), the `AnimationEvent` fields (`entity`/`clip`/`frame`/`tag`), and the
   `load_particle_configs` pattern.
3. **Designed the shared-module extraction** (see Key Decisions): reuse `Effect` cleanly on the 2nd
   consumer by moving the vocabulary + apply machinery into a neutral `crate::effect` module, rather
   than `anim_effect` importing `Effect` from `zone_effect` (a smell) or duplicating ~40 lines.
4. **Wrote `src/effect.rs`** — `Effect`/`EffectAnchor` + `pub(crate)` `PendingEffect`/`lookup_emitter`/
   `resolve_effect`/`apply_pending`. `resolve_effect(world, effect, particle_anchor, flash_target,
   &mut missing)` is the shared resolver; each system supplies its own anchor/target entities.
5. **Refactored `src/zone_effect.rs`** to `use crate::effect::{...}`, removed the moved items, and
   rewrote `ZoneEffectSystem::run` to call `resolve_effect` (peeking `at` to pick the anchor) +
   `apply_pending`. Added the test-module imports the moved types had supplied. 15 effect tests green.
6. **Wrote `src/anim_effect.rs`** — bindings/registry/error/system + 5 tests + a doctest. Keyed on
   `AnimationEvent.tag`; both anchors = the animating entity.
7. **Wired** `lib.rs` (`pub mod effect/anim_effect` + re-exports; moved `Effect`/`EffectAnchor`
   re-export from `zone_effect` to `effect`), `app.rs` (auto-register `AnimEffectRegistry`),
   `app/editor/loading.rs` (`load_anim_effects`).
8. **Wrote `examples/anim_effects.rs` + `anim_effects{,_particles}.ron`** — walk cycle → footstep
   tag → dust + tone + flash. Built + ran headless on Metal; tuned the dust emitter (faster upward
   velocity + wider spread + count 18) so the burst is clearly visible around the sprite, re-rendered.
9. **Verify (gate-by-gate).** `cargo fmt` → fmt --check (0) → clippy `--all-targets` (0) → wasm build
   (0) → **doc -D warnings FAILED (101)**: fixed two rustdoc-link errors (see Gotchas) → doc (0) →
   `test --all-targets` skipping 2 audio tests (0; 5 anim + 5 zone) → `test --doc` (0; 84 passed).
10. **CLAUDE.md** Zone→Effect row updated + Animation→Effect row added.
11. **`/ship`** → v0.88.0 → **v0.89.0** (MINOR): Cargo.toml + `cargo update -p skeleton-engine` (lock)
    + CHANGELOG 0.89.0 (Added + Changed-internal) + CLAUDE.md header v1.6.176; re-ran the bump-shiftable
    gates (build/clippy/wasm/doc) green.
12. **`/land-pr`** → branch `feat/anim-effects`, commit `79e76dc`, push, PR **#287**, watched CI (5/5
    CLEAN), confirmed `mergeStateStatus == CLEAN`, squash-merged `d579c5e`, synced main, bumped memory
    to seq 122.

## Key Decisions

- **Extracted a shared `crate::effect` module on the 2nd consumer.** `Effect`/`EffectAnchor` + the
  apply machinery (`resolve_effect`/`apply_pending`/`lookup_emitter`/`PendingEffect`) moved out of
  `zone_effect` into a neutral `effect` module. This is **not** the kind of speculative public
  abstraction the seq-3 RemoteEntities handoff warned against: (a) `Effect`/`EffectAnchor` were
  *already* public + shared; (b) the apply machinery is `pub(crate)` internal — not new public API;
  (c) the shape is now clear from two real consumers. Extracting on the second use (not the first) is
  the correct timing. Rejected: `anim_effect` importing `Effect` from `zone_effect` (a dependency
  smell — `Effect` would conceptually "belong" to zones), or duplicating ~40 lines.
- **Behavior-preserving + non-breaking refactor.** `engine::Effect`/`engine::EffectAnchor` keep their
  crate-root re-export paths (now from `effect`), so no consumer breaks; `zone_effect`'s tests are
  unchanged and green, proving the move didn't alter behavior.
- **`AnimEffectBindings` value is a bare `Vec<Effect>` — no phase.** A zone has enter/stay/exit
  (hence `ZoneEffectRule { on: ZonePhase, effect }`); an animation event is a single instantaneous
  "frame entered" fire, so a phase wrapper would be dead weight. The two binding types deliberately
  differ here.
- **Key on `AnimationEvent.tag` directly.** The tag is a field on the event itself (the `FrameEvent`
  label), so — unlike zones, which resolve a zone entity's `Tag` component — no component lookup is
  needed. Simpler.
- **Both anchors = the animating entity; `EffectAnchor` is not consulted.** There is no second entity
  for an animation effect (no "zone"), so `SpawnParticles` and `Flash` both target `event.entity`.
  The shared `Effect::SpawnParticles { at }` field is simply ignored by `anim_effect` (documented).
  Rejected: renaming `EffectAnchor` variants to be source-neutral (a breaking churn one version after
  shipping them).
- **`resolve_effect` takes the two anchor entities as params** (not the `Effect`'s `at`), so the
  shared resolver is source-agnostic — the zone system computes `particle_anchor` from `at`
  (entrant/zone), the anim system passes the entity for both. Keeps the variant-specific anchor logic
  in each system and the common resolution shared.
- **Versioning: MINOR (v0.89.0).** Additive feature + a behavior-preserving internal refactor with no
  public API change, pre-1.0 → MINOR.

## Evidence & Data

### Commit / PR (this session)

| Hash | PR | Version | Seq | Summary |
|---|---|---|---|---|
| `79e76dc` (→ squashed `d579c5e`) | #287 | v0.89.0 | 122 | Animation→Effect bindings + shared `effect` module |

### New public API surface (all additive / non-breaking)

| Symbol | Kind | Location |
|---|---|---|
| `AnimEffectBindings` (`from_ron_str`/`effects_for`/`len`/`is_empty`) | type | `anim_effect.rs` |
| `AnimEffectRegistry` (`load`/`insert`/`get`/`names`/`reload_path`) | World resource | `anim_effect.rs` |
| `AnimEffectError` (alias `AssetLoadError`) | error | `anim_effect.rs` |
| `AnimEffectSystem::new(name)` | System | `anim_effect.rs` |
| `App::load_anim_effects(name, path)` | App method | `app/editor/loading.rs` |
| `Effect`, `EffectAnchor` | re-export moved `zone_effect`→`effect` (path unchanged) | `effect.rs` |

Crate-root re-exports in `lib.rs`: `pub use anim_effect::{AnimEffectBindings, AnimEffectError,
AnimEffectRegistry, AnimEffectSystem};` and `pub use effect::{Effect, EffectAnchor};`.

### Tests added (5 + 1 doctest)

`anim_effect::tests`: `bindings_parse_lookup_and_defaults` (2-tag parse; `count`/`at` defaults on the
bare-Effect list), `malformed_ron_returns_err`, `registry_insert_get_names`,
`system_flashes_animating_entity_end_to_end` (a `"hit"` event → `HitFlash` on the sprite-bearing
entity), `system_spawns_burst_at_animating_entity` (a `"footstep"` event spawns one `ParticleBurst`
count 8 at the entity's Transform). Doctest: `AnimEffectBindings::from_ron_str` (len + `effects_for`).

### Test counts

`anim_effect::tests` 5 + `zone_effect::tests` 5 (unchanged) all green; doctests 84 passed (incl. the
new one); full `cargo test --all-targets` 0 failures (2 environmental audio tests skipped locally,
green on CI).

### CI (PR #287 — 5-job matrix, all green)

| Job | Result | Time |
|---|---|---|
| Test (native) | pass | 5m26s |
| Build (WASM) | pass | 46s |
| Render tests (lavapipe) | pass | 1m32s |
| Rustdoc | pass | 45s |
| Package dry-run | pass | 1m7s |

### Headless render verification (Metal, `HEADLESS_SHOT=/tmp/anim_effects.png HEADLESS_FRAMES=74`)

- Console: `footstep → fired its effects  (total: 1)` → `footstep → fired its effects  (total: 2)` →
  `wrote /tmp/anim_effects.png (74 frames)`.
- Screenshot: title `Animation → Effect bindings — dust + click + flash on the contact frames, all in
  RON`; the walker (a passing-frame cool body) with a clear **dust burst** of light specks scattered
  around/below it; HUD `current frame: 0   footsteps: 2`. The artifact was sent to the user.

Reproduce: `HEADLESS_SHOT=/tmp/anim_effects.png cargo run --example anim_effects` (native GPU;
`HEADLESS_FRAMES=N` overrides the 70-frame default).

## Code Analysis

- **`src/effect.rs`** — `resolve_effect(world, effect, particle_anchor, flash_target, &mut
  missing_particles) -> Option<PendingEffect>`: `Flash`→`PendingEffect::Flash{flash_target,…}`,
  `PlayTone`→`Tone`, `SpawnParticles`→read `particle_anchor`'s `Transform` + `lookup_emitter` (force
  `emit=false`/`spawn_rate=0`) → `Burst`. `apply_pending(world, Vec<PendingEffect>)`: spawn burst
  entities, add `HitFlash` (Sprite-guarded), play tones via `Audio`.
- **`AnimEffectSystem::run`**: clone the named table; snapshot `Events<AnimationEvent>` →
  `Vec<(Entity, tag)>`; for each, `bindings.effects_for(&tag)` → `resolve_effect(world, effect,
  entity, entity, &mut missing)`; `apply_pending`. The one-time `warned_missing_particles`/
  `warned_no_table` flags match `ZoneEffectSystem`.
- **`ZoneEffectSystem::run`** (refactored): same shape, but keys on the zone entity's `Tag`, filters
  by `ZonePhase`, and computes `particle_anchor` from `Effect::SpawnParticles { at }` (Zone→the zone,
  else→the entrant) before `resolve_effect(world, effect, particle_anchor, other, &mut missing)`.

## Gotchas & Discoveries

- **rustdoc `-D warnings` (the doc gate) caught two link errors the other gates missed:** (a) a public
  module doc must not intra-doc-link a **private/`pub(crate)`** item — `[`PendingEffect`]`/
  `[`lookup_emitter`]`/`[`resolve_effect`]`/`[`apply_pending`]` in the `effect` module doc errored
  (`private-intra-doc-links`); use plain ``code spans`` for them, not `[links]`. (b) `[`EffectAnchor`]`
  in the `anim_effect` module doc was **not in scope** (anim_effect doesn't `use EffectAnchor`) →
  errored (`broken-intra-doc-links`); link via the full path `[`EffectAnchor`](crate::EffectAnchor)`.
  Run the doc gate, not just fmt/clippy.
- **`use super::*` in a test module picks up the parent's private `use` imports** — after moving
  `Effect`/`EffectAnchor` out of `zone_effect`, its tests still resolve them via the module's new
  `use crate::effect::{…}`; but the tests *also* needed `Transform`/`Sprite`/`Color`/`HitFlash`/
  `ParticleBurst`/`Vec2` (which were dropped from the module's top-level imports in the refactor), so
  those were added explicitly inside `mod tests`.
- **Extract shared code on the 2nd consumer, not the 1st** — duplicating once is fine; the shape is
  only clear with two real call sites. This is the inverse of the seq-3 RemoteEntities "don't
  over-abstract from 2 similar call sites" lesson, because here the extraction is *internal*
  (`pub(crate)`) dedup of identical leaf logic, not a speculative *public* API.
- **Environmental audio (standing):** locked/remote macOS has no audio device → 2 audio-device tests
  fail locally; `--skip` them and let CI gate audio.
- **zsh `${PIPESTATUS[0]}` is empty** (carried) — read exit codes via `echo $?` on an unpiped command
  or from a background-task completion notification; index a pipe with `${pipestatus[1]}`.

## Files Changed

### Source — new
- `src/effect.rs` — shared `Effect`/`EffectAnchor` + `pub(crate)` apply machinery.
- `src/anim_effect.rs` — the Animation→Effect binding feature + 5 tests + a doctest.

### Source — modified
- `src/zone_effect.rs` — refactored to use `crate::effect` (types + apply logic moved out; system thin).
- `src/lib.rs` — `pub mod effect/anim_effect` + re-exports; `Effect`/`EffectAnchor` re-export moved to `effect`.
- `src/app.rs` — auto-register `AnimEffectRegistry` in `App::new`.
- `src/app/editor/loading.rs` — `App::load_anim_effects`.

### Examples — new
- `examples/anim_effects.rs` — walk cycle → footstep tag → dust + tone + flash; `HEADLESS_SHOT`.
- `examples/anim_effects.ron` — the `"footstep"` → `[SpawnParticles, PlayTone, Flash]` binding.
- `examples/anim_effects_particles.ron` — the `dust` emitter.

### Docs / paperwork
- `CLAUDE.md` — Zone→Effect row updated (shared `effect.rs`) + new Animation→Effect row; header
  v1.6.175 → v1.6.176 / package v0.88.0 → v0.89.0.
- `docs/CHANGELOG.md` — 0.89.0 entry (Added + Changed-internal).
- `Cargo.toml`, `Cargo.lock` — version bump + lock refresh.

## User Feedback & Preferences

- **The empty-board → ASK → user-picks-candidate-1 loop is working** across seq 6 and 7. Keep it.
- **"AnimationEvent→effect 바인딩 진행해" → execute the chosen candidate end-to-end** via the land-pr
  loop, autonomously, reporting outcomes.
- **Per-seq handoff cadence** — each code PR is followed by its own `docs(handoff)` PR.
- **Korean for all user-facing replies; English for code/docs/commits/handoffs** (project rule).
- **Merge authority delegated** — squash on green CI; PR #287 landed without asking.
- **Values evidence over assertion** — the headless render screenshot (dust burst on a footstep) was
  sent to the user as the acceptance artifact, alongside CI numbers and test counts.

## Where We're Going

The `breadth-features` chain has now shipped **nine** features this run (SpriteFlip, YSort,
AnimationEvents, TriggerZone, HitFlash, CameraLookahead, data-driven TriggerZones, Zone→Effect
bindings, **Animation→Effect bindings**). The event→effect pattern now has two sources (zones,
animations) over one shared `Effect` vocabulary. **Read `../dungeon-merchant/docs/engine-wishlist.md`
FIRST each session** (ACTIVE EMPTY, EW-004 next) — a real downstream request outranks self-picked
work. Remaining candidates, roughly ordered:

1. **A 3rd event→effect source** (only if a game asks) — e.g. `CollisionEvent`→effect, another reuse
   of the shared `crate::effect` machinery. The pattern is now proven; a 3rd source is mechanical but
   should be driven by a concrete need, not speculation.
2. **Richer effect payloads** — a per-effect position offset (so a footstep's dust spawns at the feet,
   not the entity center), an effect that despawns/spawns an entity, a `Stayed`-throttle for zones.
   On a concrete request.
3. **Editor i18n gap** (lower value) — `editor/ui/audio_panel.rs` English status strings bypass
   `i18n::tr(en, ko)`; editor-only, not CI-render-verifiable.
4. **A Tier-2 hardcoding knob on a concrete request** — the remaining knobs are weak (MAX_GAMEPADS,
   material params >4, editor app-id, frame latency); do one only if asked.

Otherwise: **ASK the user for direction** when the board is empty.

**The shared-effect pattern:** `crate::effect` holds the `Effect` vocabulary + `pub(crate)`
`resolve_effect`/`apply_pending`. A new event→effect source is a thin module: a `*Bindings` table
(tag→effects), a `RonRegistry`-backed registry (auto-registered), an `App::load_*`, and a `System`
that reads its event stream, keys on a tag, picks the anchor/target entities, and calls
`resolve_effect` + `apply_pending`.

## Risks & Blockers

- **Audio-device tests fail locally** (environmental) — `--skip` the two; CI gates audio.
- **An animation effect's particle burst anchors at the entity center, not the feet** — there is no
  per-effect offset, so footstep dust spawns at the sprite center (the example bumps the dust's
  upward velocity + spread so it spills out visibly). A position offset is a future refinement (see
  Where We're Going).
- **`AnimEffectSystem` ignores `EffectAnchor`** — a RON author who writes `at: Zone` in an *animation*
  binding gets `Other`-like behavior (anchored at the entity) silently. Documented; harmless.
- **No OS-gated code this session** — cross-platform (lavapipe exercised the GPU path; the rest is
  pure logic + file I/O). `load_anim_effects` is a no-op on wasm (no fs), as documented + mirrored.
- No upstream/dependency blockers. Tree clean.

## Open Questions

- Should effects carry a **position offset** (so footstep dust lands at the feet)? Deferred — the
  example tunes the emitter velocity instead; a real offset is the natural next refinement.
- Should there be a **`CollisionEvent`→effect** source too? The shared machinery makes it ~mechanical;
  deferred until a game asks (avoid speculative breadth).
- Should `AnimEffectSystem` warn when a binding's `at: Zone` is used (since it's ignored)? Kept silent
  (the default `Other` is the only sensible anchor anyway).

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5            # confirm main tip (anim-effects #287 + this handoff PR)
git status -s                   # clean

# 1) Read the game↔engine board FIRST (drives priority)
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick

# Memory (full per-seq detail of this session)
#   ~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md (seq 122 tip)

# Key files if continuing event→effect / breadth work
#   src/effect.rs        — the shared Effect vocabulary + resolve_effect/apply_pending (reuse for a 3rd source)
#   src/anim_effect.rs   — the Animation→Effect binding (thinnest reference: read event → key on tag → apply)
#   src/zone_effect.rs   — the Zone→Effect binding (phase + Tag-component key)
#   src/app/editor/loading.rs — App::load_*/spawn_* live here

# Verify (NOTE: 2 audio-device tests fail locally — environmental; read exit via `echo $?`, not ${PIPESTATUS[0]} in zsh)
cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps   # the doc gate catches private/broken intra-doc links

# Reproduce this session's example
HEADLESS_SHOT=/tmp/anim_effects.png cargo run --example anim_effects

# Next action
#   Read the wishlist board; if still empty, ASK for direction (the easy breadth + the event→effect
#   pattern across 2 sources are done; a 3rd source or richer payloads should be driven by a concrete
#   game need, not speculation).
```

## Session Closed

**Closed:** 2026-06-29
**Chain:** `breadth-features` seq 7 — continuation of `HANDOFF_breadth-features_zone-effects_2026-06-29.md` (seq 6)
**Code landed:** #287 (v0.89.0), main @ `d579c5e`. This handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off. Nine breadth features shipped this run; the event→effect pattern now spans zones + animations over one shared `Effect` vocabulary. Next session starts from the wishlist board or asks for direction.
