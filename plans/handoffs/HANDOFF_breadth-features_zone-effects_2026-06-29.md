# Data-driven Zone→Effect bindings — `ZoneEffectBindings` + `ZoneEffectSystem` + `App::load_zone_effects` shipped (v0.88.0, PR #285), sixth breadth feature of the chain

**Date:** 2026-06-29
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `breadth-features` seq `6`
**Parent:** `HANDOFF_breadth-features_data-trigger-zones_2026-06-29.md` (seq 5)
**Prior chain:** `hardcoding-audit` (closed) → pivoted to `breadth-features` at its seq 4

> Chain rationale: the seq-5 handoff (data-driven trigger zones) ended ACTIVE-BOARD-EMPTY and said
> to **ASK the user for direction**. This session did exactly that — confirmed the wishlist board was
> empty (EW-001/002/003 all Verified+archived, next free ID EW-004), then asked. The user chose
> **"이벤트→효과 바인딩"** (event→effect binding) — the seq-5 handoff's "Where We're Going" candidate 1
> (a `ZoneEvent`/animation-event → spawn-particle-burst / play-sound helper). So this is a direct
> continuation: `breadth-features` seq 6, parent = the seq-5 handoff.

## Related Handoffs

- `HANDOFF_breadth-features_data-trigger-zones_2026-06-29.md` — **the parent** (`breadth-features`
  seq 5, #282 data-driven trigger zones). This feature is its explicit companion: seq 5 authors
  *where* a zone is in RON; seq 6 authors *what happens* on it in RON. Its candidate-1 ("an
  event→effect binding") is exactly what shipped here.
- `HANDOFF_breadth-features_trigger-zones_2026-06-29.md` — `breadth-features` seq 2 (#276), the
  code-built `TriggerZone` + `ZoneEvent` this feature reacts to.
- `HANDOFF_breadth-features_hit-flash_2026-06-29.md` — seq 3 (#278, `HitFlash`), one of the three
  effects this binding can fire.

## Reference Documents

- `CLAUDE.md` — gained a **Zone→Effect bindings** module-map row (after the `TriggerZone` row). Header
  bumped to **v1.6.175** / package **v0.88.0**.
- `docs/CHANGELOG.md` — the 0.88.0 entry written this session.
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — bumped
  to **seq 121** (the most detailed per-seq record of this session lives in that bullet).
- `../dungeon-merchant/docs/engine-wishlist.md` — the game↔engine board; **ACTIVE EMPTY** (next free
  ID EW-004). Read FIRST next session.
- `src/ron_registry.rs`, `src/particle/config_set.rs`, `src/trigger_zone.rs` (Data-driven section) —
  the data-driven config template this feature mirrors (now applied a 5th time).

## The Goal

Continue the `breadth-features` chain with the seq-5 handoff's candidate-1: make event reactions
**data-driven**. A game that authors its zones in RON (seq 5) still has to write Rust to *react* to a
`ZoneEvent`. A level designer wants to author the reactions — a hit-flash, a particle burst, a sound
— in data too, keyed by the zone's name. The acceptance test (per the VISION loop) is a small
playable example that loads zones, particles, AND reactions all from RON and exercises them in real
play.

## Where We Are

- **main @ `b3668b6`** (package **v0.88.0**, CLAUDE.md header **v1.6.175**), tree **clean**.
  _(This handoff doc lands as its own `docs(handoff)` PR after the code PR; the recorded tip will
  move to that handoff merge.)_
- **PR #285 merged** (squash + branch-deleted, CI **5/5** green): `feat(zone_effect): data-driven
  Zone→Effect bindings — ZoneEffectBindings + ZoneEffectSystem (v0.88.0)`.
- **New module** `src/zone_effect.rs` (588 lines incl. tests): the `Effect` vocabulary, the binding
  table, the registry, the system, 5 unit tests + 1 doctest.
- **New public API** (all additive, non-breaking): `engine::Effect`, `engine::EffectAnchor`,
  `engine::ZonePhase`, `engine::ZoneEffectRule`, `engine::ZoneEffectBindings`,
  `engine::ZoneEffectRegistry`, `engine::ZoneEffectError`, `engine::ZoneEffectSystem` (crate-root
  re-exports), plus `App::load_zone_effects(name, path)`.
- **`Effect` enum** — `SpawnParticles { particles: String, count: u32 (#[serde(default)]=16), at:
  EffectAnchor }`, `PlayTone { freq, dur, vol, bus: Option<String> }`, `Flash { color: (f32,f32,f32,f32),
  secs: f32 }`. The three game-feel reactions the engine already produces.
- **`EffectAnchor`** = `Other` (default — the entity that triggered) / `Zone` (the zone entity); only
  used by `SpawnParticles` to pick the burst position.
- **`ZonePhase`** = `Entered` (default) / `Stayed` / `Exited`; gates a rule.
- **`ZoneEffectBindings { bindings: HashMap<String, Vec<ZoneEffectRule>> }`** — `from_ron_str(s)`
  (cross-platform), `rules_for(tag) -> &[ZoneEffectRule]`, `len`/`is_empty`. Deserializes a RON doc
  shaped `(bindings: { "tag": [ (on: …, effect: …), … ] })` directly (no separate `*Doc` wrapper —
  the struct *is* the doc).
- **`ZoneEffectRegistry`** wraps `RonRegistry<ZoneEffectBindings>`; `load`/`insert`/`get`/`names`/
  `reload_path` + `RonLoadable for ZoneEffectBindings` + `HotReloadable for ZoneEffectRegistry`.
  Auto-registered in `app.rs` `App::new` alongside the particle/dialogue/trigger-zone registries.
- **`App::load_zone_effects`** (in `app/editor/loading.rs`): lazily inserts the registry,
  `register_persistent`s it, loads + `watch_path`s the file (native, generic `watch_path`); no-op on
  wasm — mirrors `load_trigger_zones`/`load_dialogue` exactly.
- **`ZoneEffectSystem::new(name)`** (a struct holding `name` + two one-time-warn flags): clones the
  named binding table out of the registry (dodges the `&World`/`&mut World` borrow conflict), reads
  `Events<ZoneEvent>` into `(phase, zone, other)` triples, resolves each zone's `Tag` → key, builds a
  flat `Vec<PendingAction>` (read-only world access), then applies them (mutating). Add it **after**
  `TriggerZoneSystem`.
- **New example** `examples/zone_effects.rs` (+ 3 RON files) — heal/damage/goal zones loaded +
  spawned (seq-5 path), particle emitters loaded, effect bindings loaded; the player auto-drifts
  through the zones and each binding fires. `HEADLESS_SHOT` support (auto-drift, 130 frames default).
- **Tests:** 5 new unit tests + 1 new doctest.
- **CI:** PR #285 passed the 5-job matrix (Test native 5m13s / Build WASM 46s / Render lavapipe
  1m24s / Rustdoc 39s / Package dry-run 1m19s).
- **Headless render verified on Metal:** console fired `entered heal → fired its effects` →
  `exited heal` → `entered damage → fired its effects`; the screenshot showed the player inside the
  damage zone with a **blood particle burst** scattered where it entered (the RON `SpawnParticles`
  effect), HUD `last entered: damage   zone entries: 2`.
- **CLAUDE.md module-map** — gained the Zone→Effect bindings row.
- **Memory** `engine-current-state.md` bumped to seq 121.

## What We Tried (Chronological)

1. **Confirmed the board + asked.** Read `../dungeon-merchant/docs/engine-wishlist.md` (ACTIVE EMPTY,
   EW-004 next). Per the seq-5 handoff's instruction, ASKED the user for direction with the
   candidate list → user chose **"이벤트→효과 바인딩"**.
2. **Grounded the design in real signatures** via an `Explore` agent: exact `ZoneEvent`/
   `AnimationEvent` shapes + read pattern, `ParticleEmitter::burst()`/`ParticleBurst`/
   `ParticleConfigSet::emitter(name)`, the `Audio` facade (`play_tone`/`play_tone_on_bus`),
   `HitFlash::new`, the `ParticleConfigSet` data-driven template, the `System` trait, and that
   `Color` has serde + `prefab::Tag(pub String)`.
3. **Confirmed two correctness-critical mechanics by reading source:** (a) `Events<E>` is
   *within-frame* (`src/ecs/events.rs`: send → read in a later system the same frame, `App` flushes
   at end-of-frame) → a `ZoneEffectSystem` added after `TriggerZoneSystem` reads this frame's events;
   (b) the `ParticleSystem` burst path (`src/particle/mod.rs:381`) emits `ParticleBurst.remaining`
   particles using the emitter's *visual* params (color/size/spread/lifetime/gravity/emit_shape) and
   despawns the emitter → taking a config `emitter()`, forcing `emit=false`/`spawn_rate=0`, and
   adding `ParticleBurst{count}` gives a one-shot burst with the config's look.
4. **Chose the design** (see Key Decisions): ZoneEvent-only vocabulary, three effects, a registry
   mirroring `TriggerZoneRegistry` 1:1, `Effect` color as an `(r,g,b,a)` tuple (not a private mirror).
5. **Wrote `src/zone_effect.rs`** — types + registry + `RonLoadable`/`HotReloadable` + the system +
   `lookup_emitter` helper + 5 tests + a doctest. Lib + tests compiled; 5/5 tests passed first try.
6. **Wired** `lib.rs` (mod + re-export the 8 symbols), `app.rs` (auto-register `ZoneEffectRegistry`),
   `app/editor/loading.rs` (`load_zone_effects`).
7. **Wrote the example** `zone_effects.rs` + `zone_effects{,_zones,_particles}.ron` — load 3 RON
   files → spawn zones → walk → effects fire via `ZoneEffectSystem`. Built + ran headless on Metal
   (console enter logs + a screenshot with a visible blood burst).
8. **Verify (gate-by-gate).** `cargo fmt` (applied; reflowed 2 files) → fmt --check (0) → clippy
   `--all-targets -D warnings` (0) → wasm lib+bins build (0) → doc -D warnings (0) → `test
   --all-targets` skipping the 2 environmental audio tests (0; 5 new zone_effect tests) → `test
   --doc` (0; the new doctest).
9. **CLAUDE.md** module-map row added; header bumped via `/ship`.
10. **`/ship`** → v0.87.0 → **v0.88.0** (MINOR): Cargo.toml + `cargo update -p skeleton-engine`
    (lock) + CHANGELOG 0.88.0 + CLAUDE.md header v1.6.175; re-ran build/clippy/wasm/doc green.
11. **`/land-pr`** → branch `feat/zone-effects`, commit `6dd8760`, push, PR **#285** (a transient
    `gh` HTTP 502 on first create — `gh pr list --head` confirmed NOT created, retried clean),
    watched CI (5/5 CLEAN), confirmed `mergeStateStatus == CLEAN`, squash-merged `b3668b6`, synced
    main, bumped memory to seq 121.

## Key Decisions

- **ZoneEvent-only vocabulary (not a generic event→effect engine).** The binding reacts to
  `ZoneEvent` and resolves the zone's `Tag` to a key — this is concrete and grounded in the seq-5
  data-driven zones. A generic abstraction over event *types* would be speculative (the seq-5
  handoff explicitly warned against it). The `Effect` enum is standalone, so an `AnimationEvent`→
  effect binding can reuse it later without rework. Rejected: a trait-object event source.
- **Three effects = the three game-feel reactions the engine already produces.** `SpawnParticles`
  (reusing the data-driven `ParticleConfigRegistry` — composition, so particle *visuals* stay
  data-driven), `PlayTone` (the `Audio` facade — `play_tone` is fully scalar/data-friendly, unlike
  `play_sfx` which needs bytes), `Flash` (`HitFlash`). Rejected: inline particle params on the
  effect (would duplicate `EmitterDef`); referencing a named emitter is DRY and shows two
  data-driven systems composing.
- **`Effect` color is an `(f32,f32,f32,f32)` tuple, NOT a private `ColorDef`.** A private serde-mirror
  type in a *public* enum variant is `E0446` (private-type-in-public-interface). A primitive tuple
  avoids the leak, deserializes from RON as `(1.0, 0.3, 0.3, 1.0)` (matching the particle-config
  convention), and converts to `Color::rgba` in the system. Rejected: making `ColorDef` public (ugly
  mirror leak), or using `Color`'s own struct serde (verbose `(r:…,g:…)` RON).
- **`SpawnParticles` looks an emitter up by name across ALL loaded sets** (`lookup_emitter` scans
  `ParticleConfigRegistry::names()` → first `set.emitter(name)` match). Keeps the effect to a single
  `particles: String` (no `(set, emitter)` pair). The scan is tiny and only runs when an effect
  fires. The looked-up emitter has `emit`/`spawn_rate` forced to 0 so only the burst fires.
- **`ZoneEffectSystem::new(name)` takes the registry name** (mirrors `spawn_trigger_zones(name)`).
  Clones the named table out of the registry each frame to resolve the resource-borrow-then-mutate
  conflict. Rejected: applying all tables merged (name-collision ambiguity), or a single non-registry
  resource (diverges from the proven template).
- **`Flash` is skipped if the target has no `Sprite`** — `HitFlashSystem` only animates
  `query2_mut::<HitFlash, Sprite>`, so adding a `HitFlash` to a sprite-less entity would leave it
  dangling forever. The apply step guards with `world.get::<Sprite>(target).is_some()`.
- **Audio is optional in the example** — `Audio::new()` returns `None` with no device (headless /
  locked session); the `PlayTone` effect no-ops without an `Audio` resource. No `cfg` guards needed
  (the facade is cross-platform).
- **Versioning: MINOR (v0.88.0).** Additive types + one App method, pre-1.0 → MINOR.

## Evidence & Data

### Commit / PR (this session)

| Hash | PR | Version | Seq | Summary |
|---|---|---|---|---|
| `6dd8760` (→ squashed `b3668b6`) | #285 | v0.88.0 | 121 | data-driven Zone→Effect bindings |

### New public API surface (all additive / non-breaking)

| Symbol | Kind | Location |
|---|---|---|
| `Effect` (`SpawnParticles`/`PlayTone`/`Flash`) | enum | `zone_effect.rs` |
| `EffectAnchor` (`Other`/`Zone`) | enum | `zone_effect.rs` |
| `ZonePhase` (`Entered`/`Stayed`/`Exited`) | enum | `zone_effect.rs` |
| `ZoneEffectRule { on, effect }` | struct | `zone_effect.rs` |
| `ZoneEffectBindings` (`from_ron_str`/`rules_for`/`len`/`is_empty`) | type | `zone_effect.rs` |
| `ZoneEffectRegistry` (`load`/`insert`/`get`/`names`/`reload_path`) | World resource | `zone_effect.rs` |
| `ZoneEffectError` (alias `AssetLoadError`) | error | `zone_effect.rs` |
| `ZoneEffectSystem::new(name)` | System | `zone_effect.rs` |
| `App::load_zone_effects(name, path)` | App method | `app/editor/loading.rs` |

Crate-root re-export in `lib.rs`: `pub use zone_effect::{Effect, EffectAnchor, ZoneEffectBindings,
ZoneEffectError, ZoneEffectRegistry, ZoneEffectRule, ZoneEffectSystem, ZonePhase};`.

### Tests added (5 + 1 doctest)

`zone_effect::tests`: `bindings_parse_lookup_and_defaults` (3-rule "damage" + 1-rule "heal" parse;
checks `on`/`count`/`at` serde defaults + `at: Zone` parse), `malformed_ron_returns_err`,
`registry_insert_get_names`, `system_flashes_entering_entity_end_to_end` (real registry→system→apply:
`Entered` event → `HitFlash` added to the sprite-bearing actor with the bound color/secs),
`system_spawns_burst_from_named_emitter` (an `Entered` event spawns exactly one `ParticleBurst` with
the default count 16, anchored at the actor's Transform). Doctest: `ZoneEffectBindings::from_ron_str`
(len + `rules_for`).

### Test counts

`zone_effect::tests` 5 passed; the `ZoneEffectBindings` doctest passed (83 doctests total in the
crate); full `cargo test --all-targets` 0 failures (2 environmental audio tests skipped locally,
green on CI).

### CI (PR #285 — 5-job matrix, all green)

| Job | Result | Time |
|---|---|---|
| Test (native) | pass | 5m13s |
| Build (WASM) | pass | 46s |
| Render tests (lavapipe) | pass | 1m24s |
| Rustdoc | pass | 39s |
| Package dry-run | pass | 1m19s |

### Headless render verification (Metal, `HEADLESS_SHOT=/tmp/zone_effects.png HEADLESS_FRAMES=150`)

- Console: `entered heal → fired its effects  (total: 1)` → `exited heal` → `entered damage → fired
  its effects  (total: 2)` → `wrote /tmp/zone_effects.png (150 frames)`.
- Screenshot: title `Zone → Effect bindings — zones + particles + reactions, all authored in RON`;
  HUD `last entered: damage   zone entries: 2`; three zones (green heal, red damage, blue goal); the
  white player quad inside the damage zone with a **red blood burst** scattered to its lower-left
  (the `SpawnParticles(particles: "blood", at: Other)` effect, fired on enter). The artifact was
  sent to the user.

Reproduce: `HEADLESS_SHOT=/tmp/zone_effects.png cargo run --example zone_effects` (native GPU;
`HEADLESS_FRAMES=N` overrides the 130-frame default). The RON paths are relative to the workspace
root (cwd under `cargo run`).

## Code Analysis

- **`ZoneEffectSystem::run`** (`src/zone_effect.rs`): (1) clone the named `ZoneEffectBindings` out of
  `ZoneEffectRegistry`; (2) snapshot `Events<ZoneEvent>` → `Vec<(ZonePhase, Entity, Entity)>`; (3)
  for each, `world.get::<Tag>(zone)` → key, match `bindings.rules_for(&key)` whose `on == phase`, and
  push a `PendingAction` (resolving the anchor `Transform` + `lookup_emitter` read-only); (4) apply —
  spawn a `Transform`+emitter+`ParticleBurst` entity, add a `HitFlash` (Sprite-guarded), or call the
  `Audio` facade. The two-pass collect-then-apply is the standard escape from the resource-borrow-
  then-mutate conflict.
- **`lookup_emitter(world, name)`**: `world.resource::<ParticleConfigRegistry>()?` then
  `reg.names().iter().find_map(|set| reg.get(set).and_then(|s| s.emitter(name)))` — first match across
  all loaded sets; missing → a one-time `warn!`.
- **Registry wiring** mirrors `TriggerZoneRegistry` 1:1: `RonRegistry<ZoneEffectBindings>` inner,
  `RonLoadable for ZoneEffectBindings` (`std::fs::read_to_string` → `from_ron_str`), `HotReloadable
  for ZoneEffectRegistry` (UFCS-delegates to the inherent `reload_path`, tagged `"zone_effect"`),
  auto-registered in `App::new`.
- **`App::load_zone_effects`** mirrors `load_trigger_zones`: lazy-insert registry +
  `register_persistent` + (native) `reg.load` + generic `assets.watch_path(&path)`; wasm no-op.

## Gotchas & Discoveries

- **Private-type-in-public-interface (E0446):** a public enum variant field cannot have a private
  type. The particle/trigger-zone code uses private `ColorDef`/`*Def` mirrors because their *runtime*
  types are private-to-conversion; here `Effect` is itself public, so its color is a primitive
  `(f32,f32,f32,f32)` tuple instead (converts to `Color::rgba` in the system). Reuse this when a
  public serde type needs a color in RON.
- **The data-driven config pattern is fully templated (now 5×: particle/dialogue/anim-clip/
  trigger-zone/zone-effect)** — `RonRegistry<V>` + `RonLoadable` + `HotReloadable` + an `App::load_*`
  (lazy-insert + `register_persistent` + generic `watch_path`, wasm no-op) + auto-register in
  `App::new`. No `schedule.rs` change. A new data-driven type is a near-mechanical fill-in;
  `trigger_zone.rs` (Data-driven section) is the cleanest current reference.
- **`Events<E>` is within-frame** — read in a system *after* the sender, same frame; `App` flushes at
  end-of-frame. A `ZoneEffectSystem` added after `TriggerZoneSystem` sees this frame's `ZoneEvent`s.
- **The `ParticleBurst` path uses the emitter's visual params, not `spawn_rate`** — so a config
  `emitter()` + `ParticleBurst{count}` (with `emit`/`spawn_rate` zeroed) yields a one-shot burst with
  the config's look, then the emitter entity is despawned.
- **`gh pr create` transient HTTP 502** — happened once on first create; the PR was NOT created
  (confirmed via `gh pr list --head <branch>` before retrying). Retry succeeded. Always confirm
  before retrying a create.
- **Environmental audio (standing):** locked/remote macOS has no audio device → 2 audio-device tests
  fail locally; `--skip` them and let CI gate audio. (CI #285's native job ran them green.)
- **zsh `${PIPESTATUS[0]}` is empty** (carried from prior seqs; now also a CLAUDE.md note via #284) —
  read exit codes via `echo $?` on an unpiped command, or from a background-task completion
  notification.

## Files Changed

### Source — new
- `src/zone_effect.rs` — the whole feature (types + registry + `RonLoadable`/`HotReloadable` + system
  + `lookup_emitter`) + 5 tests + a doctest.

### Source — modified
- `src/lib.rs` — `pub mod zone_effect;` + re-export the 8 new symbols.
- `src/app.rs` — auto-register `ZoneEffectRegistry` as hot-reloadable in `App::new`.
- `src/app/editor/loading.rs` — `App::load_zone_effects`.

### Examples — new
- `examples/zone_effects.rs` — load zones+particles+effects → spawn → walk → effects fire;
  `HEADLESS_SHOT` (130 frames).
- `examples/zone_effects_zones.ron` — heal/damage/goal trigger zones.
- `examples/zone_effects_particles.ron` — `blood` + `sparkle` emitters.
- `examples/zone_effects.ron` — the effect bindings keyed by zone tag.

### Docs / paperwork
- `CLAUDE.md` — Zone→Effect bindings module-map row; header v1.6.174 → v1.6.175 / package v0.87.0 →
  v0.88.0.
- `docs/CHANGELOG.md` — 0.88.0 entry.
- `Cargo.toml`, `Cargo.lock` — version bump + lock refresh.

## User Feedback & Preferences

- **Empty-board protocol works:** the seq-5 handoff said "ASK when the board is empty"; this session
  confirmed the board empty + asked, and the user picked the direction. Keep doing this.
- **"이벤트→효과 바인딩" → execute the chosen candidate end-to-end** via the land-pr loop,
  autonomously, reporting outcomes.
- **Per-seq handoff cadence** — each code PR is followed by its own `docs(handoff)` PR.
- **Korean for all user-facing replies; English for code/docs/commits/handoffs** (project rule).
- **Merge authority delegated** — squash on green CI, no per-session re-confirm; PR #285 landed
  without asking.
- **Values evidence over assertion** — the headless render screenshot (with the lit blood burst +
  numeric HUD) was sent to the user as the acceptance artifact, alongside CI numbers and test counts.

## Where We're Going

The `breadth-features` chain has now shipped **eight** features this run (SpriteFlip, YSort,
AnimationEvents, TriggerZone, HitFlash, CameraLookahead, data-driven TriggerZones, **Zone→Effect
bindings**). The easy component+system breadth is exhausted and the first two deeper data-driven
items (zones, then their effects) are done. **Read `../dungeon-merchant/docs/engine-wishlist.md`
FIRST each session** (ACTIVE EMPTY, EW-004 next) — a real downstream request outranks self-picked
work. Remaining candidates, roughly ordered:

1. **`AnimationEvent`→effect binding** — the natural next data-driven step: the `Effect` enum is
   standalone and reusable, so an `AnimationEffectSystem` keyed by `AnimationEvent.tag` would reuse
   it directly (footstep dust on a contact frame, a swing whoosh, etc.). Design against a concrete
   need (e.g. if a game asks), not speculatively.
2. **Richer zone/effect payloads** — per-zone effect parameters, an effect that despawns/spawns an
   entity, a `Stayed`-throttle (the current `Stayed` fires every frame). Only on a concrete request.
3. **Editor i18n gap** (lower value) — `editor/ui/audio_panel.rs` English status strings bypass
   `i18n::tr(en, ko)`; editor-only, not CI-render-verifiable.
4. **A Tier-2 hardcoding knob on a concrete request** — the remaining knobs are weak (MAX_GAMEPADS,
   material params >4, editor app-id, frame latency); do one only if asked.

Otherwise: **ASK the user for direction** when the board is empty — the obvious breadth is done.

**The data-driven config template (now applied a 5th time):** private serde-mirror types (or, for a
public type, primitive tuples) → a `from_ron_str` value type → a `RonRegistry`-backed registry with
`RonLoadable` + `HotReloadable` → an `App::load_*` (lazy-insert + `register_persistent` + generic
`watch_path`, no-op on wasm) → auto-register in `App::new`. Adding another data-driven type is
near-mechanical.

## Risks & Blockers

- **Audio-device tests fail locally** (environmental, not a regression) — `--skip` the two named
  audio tests. CI gates audio.
- **`SpawnParticles` cross-set name scan picks the FIRST match** — if two loaded particle sets define
  the same emitter name, the alphabetically-first set wins. Not a problem with one set (the common
  case); document if a game loads many sets with colliding names.
- **`Stayed` fires every frame** — a `Stayed`-bound effect (e.g. a particle burst) would spam every
  frame an entity is inside. The example only binds `Entered`. A throttle/interval is a future
  refinement (noted in Where We're Going).
- **No OS-gated code this session** — everything is cross-platform (the lavapipe render job exercised
  the GPU path; the data layer is pure logic + file I/O). `load_zone_effects` is a no-op on wasm (no
  fs), the documented + mirrored behavior.
- No upstream/dependency blockers. Tree clean.

## Open Questions

- Should `Stayed` effects be **throttled** (an interval) rather than every-frame? Kept simple; a
  game binds `Entered`/`Exited` for one-shots.
- Should `SpawnParticles` reference an emitter by a **`(set, emitter)` pair** instead of a global
  name scan? Single name is cleaner for the common one-set case; revisit if multi-set name collisions
  become real.
- Should the binding support **non-zone events** (`AnimationEvent`, `CollisionEvent`) via the same
  `Effect` enum? Deliberately deferred — `Effect` is standalone so it's a clean future reuse.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5            # confirm main tip (zone-effects #285 + this handoff PR)
git status -s                   # clean

# 1) Read the game↔engine board FIRST (drives priority)
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick

# Memory (full per-seq detail of this session)
#   ~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md (seq 121 tip)

# Key files if continuing data-driven / effect work
#   src/zone_effect.rs                            — the Zone→Effect binding pattern just shipped
#   src/trigger_zone.rs (Data-driven section)     — the registry template (cleanest reference)
#   src/particle/config_set.rs + src/ron_registry.rs  — the data-driven config template
#   src/animation/events.rs                       — AnimationEvent (the next event→effect candidate)
#   src/app/editor/loading.rs                     — App::load_*/spawn_* live here

# Verify (NOTE: 2 audio-device tests fail locally — environmental; read exit via `echo $?`, not ${PIPESTATUS[0]} in zsh)
cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Reproduce this session's example
HEADLESS_SHOT=/tmp/zone_effects.png cargo run --example zone_effects

# Next action
#   Read the wishlist board; if still empty, ASK for direction (easy breadth is done; the natural
#   next data-driven step is an AnimationEvent→effect binding reusing the Effect enum — design
#   against a concrete need, not speculatively).
```

## Session Closed

**Closed:** 2026-06-29
**Chain:** `breadth-features` seq 6 — continuation of `HANDOFF_breadth-features_data-trigger-zones_2026-06-29.md` (seq 5)
**Code landed:** #285 (v0.88.0), main @ `b3668b6`. This handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off. Eight breadth features shipped this run; the data-driven zones→effects pair is complete. Next session starts from the wishlist board or asks for direction.
