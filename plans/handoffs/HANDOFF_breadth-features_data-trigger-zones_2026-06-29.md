# Data-driven trigger zones — `TriggerZoneSet` + `App::load_trigger_zones`/`spawn_trigger_zones` shipped (v0.87.0, PR #282), fifth breadth feature of the chain

**Date:** 2026-06-29
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `breadth-features` seq `5`
**Parent:** `HANDOFF_breadth-features_camera-lookahead_2026-06-29.md` (seq 4)
**Prior chain:** `hardcoding-audit` (closed) → pivoted to `breadth-features` at its seq 4

> Chain rationale: same session as the camera-lookahead handoff (seq 4). After landing camera
> lookahead + its handoff PR, the user was offered the now-thin candidate list and chose
> **"데이터 주도 트리거 존 진행해"** (proceed with data-driven trigger zones) — the first "deeper"
> breadth item (the parent's "deeper next steps: data-driven trigger zones / RON-authored effects").
> So this is a **direct continuation**: `breadth-features` seq 5, parent = the seq-4 handoff.

## Related Handoffs

- `HANDOFF_breadth-features_camera-lookahead_2026-06-29.md` — **the parent** (`breadth-features`
  seq 4, #280 camera lookahead). Its "Where We're Going" listed deeper data-driven work as a next
  step once the easy component+system breadth was covered.
- `HANDOFF_breadth-features_trigger-zones_2026-06-29.md` — `breadth-features` seq 2 (#276), the
  **code-built `TriggerZone`** this feature makes data-driven. It explicitly noted: "If
  scene-persistence is ever wanted, add serde to `CollisionLayer` + `TriggerShape`" — this session
  found a cleaner route (private serde-mirror types) that needs **no** serde on those runtime types.

## Reference Documents

- `CLAUDE.md` — the `TriggerZone` module-map row gained a "data-driven (RON)" clause this session.
  Header bumped to **v1.6.173** / package **v0.87.0**.
- `docs/CHANGELOG.md` — the 0.87.0 entry written this session.
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — bumped
  to **seq 120** (the most detailed per-seq record of this session lives in that bullet).
- `../dungeon-merchant/docs/engine-wishlist.md` — the game↔engine board; **ACTIVE EMPTY** (next free
  ID EW-004). Read FIRST next session.
- `src/ron_registry.rs`, `src/particle/config_set.rs` — the data-driven config pattern this feature
  mirrors exactly (the `ParticleConfigSet` precedent).

## The Goal

Continue the `breadth-features` pivot with its first "deeper" item: make `TriggerZone`s
**data-driven**. The code-built `TriggerZone` (seq 117) requires a game to construct each zone in
Rust; a level designer wants to author a level's zones (checkpoints, damage fields, doors, aggro
ranges) in a RON file and load + spawn them, retuning by editing data rather than code. The
acceptance test (per the VISION loop) is a small playable example that loads zones from RON and
exercises them in real play.

## Where We Are

- **main @ `292030f`** (package **v0.87.0**, CLAUDE.md header **v1.6.173**), tree **clean**.
  _(This handoff doc lands as its own `docs(handoff)` PR after the code PR; the recorded tip will
  move to that handoff merge.)_
- **PR #282 merged** (squash + branch-deleted, CI **5/5** green): `feat(trigger_zone): data-driven
  trigger zones — load + spawn zones from RON (v0.87.0)`.
- **Extended module** `src/trigger_zone.rs` — added a "Data-driven trigger zones" section after
  `TriggerZoneSystem`: private serde-mirror types (`Vec2Def`, `TriggerShapeDef`, `TriggerZoneDef`,
  `TriggerZoneDoc`), `TriggerZoneSet`, `TriggerZoneRegistry`, `TriggerZoneSetError`, the `RonLoadable`
  + `HotReloadable` impls, and 4 new tests.
- **New public API** (all additive, non-breaking): `engine::TriggerZoneSet`,
  `engine::TriggerZoneRegistry`, `engine::TriggerZoneSetError` (crate-root re-exports), plus
  `App::load_trigger_zones(name, path)` and `App::spawn_trigger_zones(name) -> Vec<Entity>`.
- **`TriggerZoneSet`:** `from_ron_str(s)` (cross-platform), `len`/`is_empty`/`tags`, and
  `spawn_into(world) -> Vec<Entity>` — one entity per def with a `Transform` at the def's position, a
  `TriggerZone` with the def's shape + mask, and (when the def's tag is non-empty) a
  `crate::prefab::Tag` (reused, not a new `ZoneTag`). Zones carry **no `Sprite`** — rendering is the
  game's concern.
- **`TriggerZoneRegistry`:** wraps `RonRegistry<TriggerZoneSet>`; `load` (native) / `insert` /
  `get` / `names` / `reload_path` (native) + `RonLoadable` for `TriggerZoneSet` + `HotReloadable`
  for the registry. Auto-registered in `app.rs` `App::new` alongside the particle/dialogue
  registries, so loaded files hot-reload (native).
- **`App::load_trigger_zones`** (in `app/editor/loading.rs`): lazily inserts the registry,
  `register_persistent`s it, loads + `watch_path`s the file (native); no-op on wasm — mirrors
  `load_particle_configs`/`load_dialogue`.
- **`App::spawn_trigger_zones`** (cross-platform): clones the named set out of the registry (to
  dodge the `&World`/`&mut World` borrow conflict) then `spawn_into(&mut self.world)`.
- **No serde added to `TriggerShape`/`CollisionLayer`** — private serde-mirror types parse the RON
  (a `u32` mask, a `TriggerShapeDef` enum), then convert to the runtime types in `spawn_into`. This
  follows the `ParticleConfigSet` precedent exactly and keeps the runtime types derive-free.
- **New example** `examples/data_trigger_zones.rs` (+ `examples/data_trigger_zones.ron`) — the seq-117
  heal/damage/goal walk, but the zones are loaded from RON and spawned; the example sizes a debug
  quad from each zone's shape and resolves `ZoneEvent`s to names via `Tag`. `HEADLESS_SHOT` support
  (auto-drift, 130 frames).
- **Tests:** 4 new unit tests (in `trigger_zone::tests`, total now 10) + 1 new doctest (on
  `TriggerZoneSet`, total 2 in this file).
- **CI:** PR #282 passed the 5-job matrix (Test native 4m34s / Build WASM 39s / Render lavapipe
  1m36s / Rustdoc 45s / Package dry-run 1m8s).
- **Headless render verified on Metal:** console fired `entered heal → exited heal → entered
  damage`; the screenshot showed three RON-loaded zones (green heal, red damage [a Circle drawn as a
  square debug quad], blue goal) with the damage zone lit while the player overlapped it (HUD
  `inside: damage`, `zone entries: 2`).
- **CLAUDE.md module-map** — the `TriggerZone` row gained a "data-driven (RON)" clause.
- **Memory** `engine-current-state.md` bumped to seq 120.

## What We Tried (Chronological)

1. **Onboarding into the data-driven pattern.** Read `src/ron_registry.rs` (`RonRegistry<V>` +
   `RonLoadable` — the generic name→value registry with canonical-path hot-reload),
   `src/particle/config_set.rs` (the `ParticleConfigSet` reference: private serde-mirror types →
   `from_ron_str` → registry wrapper + `RonLoadable`/`HotReloadable`), `app/editor/loading.rs`
   (`load_particle_configs`/`load_dialogue`), `app.rs` (the `App::new` `register_hot_reloadable`
   block + `forward_hot_reload`), `asset/hot_reload.rs` (`watch_path` is the canonical entry point).
2. **Confirmed two key facts:** `crate::prefab::Tag(pub String)` exists (reuse it for naming zones —
   no new `ZoneTag` needed), and `CollisionLayer::ALL = Self(u32::MAX)` (so `default_mask()` returns
   `CollisionLayer::ALL.0`). Example RON assets live next to the example (e.g.
   `examples/gpu_particles.ron`, loaded by the relative path `"examples/…"`).
3. **Chose the serde-mirror approach over adding serde to the runtime types.** The trigger-zones
   handoff suggested adding serde to `TriggerShape`/`CollisionLayer`; instead I used private mirror
   types (matching `ParticleConfigSet`), so the runtime types stay derive-free and the data layer is
   self-contained.
4. **Wrote the data-driven section in `src/trigger_zone.rs`** — mirror types, `TriggerZoneSet`,
   `TriggerZoneRegistry`, error alias, `RonLoadable`/`HotReloadable`, + 4 tests.
5. **Wired** `lib.rs` (re-export the 3 new symbols), `app.rs` (auto-register
   `TriggerZoneRegistry`), `app/editor/loading.rs` (`load_trigger_zones` + `spawn_trigger_zones`).
6. **Wrote `examples/data_trigger_zones.ron` + `examples/data_trigger_zones.rs`** — load → spawn →
   decorate-with-debug-quad → walk → react-by-Tag + poll-occupancy-tint.
7. **Verify (gate-by-gate).** fmt → clippy `--all-targets` (0) → 10 lib tests (4 new) + 2 doctests
   → wasm lib build (0) → rustdoc -D warnings (0) → `test --all-targets` skipping the 2 audio tests
   (0 failures) → **headless render on Metal** (console enter/exit + the screenshot).
8. **CLAUDE.md `TriggerZone` row** extended.
9. **`/ship`** → v0.86.0 → **v0.87.0** (MINOR): Cargo.toml + `cargo update -p skeleton-engine`
   (lock) + CHANGELOG 0.87.0 + CLAUDE.md header v1.6.173.
10. **`/land-pr`** → branch `feat/data-trigger-zones` (changes had been authored on main →
    `git checkout -b` carried them), commit `492f945`, push, PR **#282**, watched CI (5/5 CLEAN),
    confirmed `mergeStateStatus == CLEAN`, squash-merged `292030f`, synced main, bumped memory to
    seq 120.

## Key Decisions

- **Private serde-mirror types, NOT serde on the runtime types.** `TriggerZoneDef` carries a `u32`
  mask and a `TriggerShapeDef` enum; `spawn_into` converts to `CollisionLayer`/`TriggerShape`. This
  matches `ParticleConfigSet`'s `Vec2Def`/`EmitShapeDef` exactly, keeps `TriggerShape`/`CollisionLayer`
  derive-free (consistent with their current state), and keeps the data layer fully self-contained.
  Rejected: adding `Serialize`/`Deserialize` to the runtime types (the trigger-zones handoff's
  suggestion) — unnecessary churn given the mirror-type precedent.
- **Reuse `crate::prefab::Tag` to name zones, not a new `ZoneTag`.** `Tag(pub String)` already exists,
  is serde/clone/editor-registered, and is the engine's established "name an entity" component. A
  game resolves a `ZoneEvent { zone, .. }` to a name via `world.get::<Tag>(zone)`. Rejected: a
  dedicated `ZoneTag` component (more surface, no benefit).
- **Zones carry no `Sprite`.** `spawn_into` creates only the logical zone (`Transform` +
  `TriggerZone` + `Tag`). Rendering is the game's job — the example decorates each spawned zone with
  a debug quad sized from its shape. This keeps the data-driven feature about *logic*, not visuals,
  and avoids baking a debug color into the def. (Trade-off: the example must read each zone's shape
  to size the quad — a small, illustrative step.)
- **`App::spawn_trigger_zones` clones the set out of the registry.** Getting the set borrows
  `&World` (the registry resource) while `spawn_into` needs `&mut World` — a borrow conflict. Cloning
  the (cheap, `Vec<TriggerZoneDef>`) set out first resolves it. `TriggerZoneSet` derives `Clone`
  for this. Rejected: threading the spawn through a closure / restructuring `spawn_into` to take the
  registry.
- **`spawn_trigger_zones` is cross-platform; `load_trigger_zones` is native-for-the-load.** Loading
  reads a file (native; no-op on wasm, like the other `load_*`). Spawning just reads the registry +
  mutates the world, so it works on both — a wasm game can `insert` a `from_ron_str` set then spawn.
- **Auto-register the registry as hot-reloadable in `App::new`** (alongside particle/dialogue), so a
  loaded zone file reloads on edit without the game wiring anything. `forward_hot_reload::<T>` no-ops
  when the resource is absent, so registering before the lazy `insert` is safe.
- **Default mask = all layers, default tag = empty.** `#[serde(default = "default_mask")]` →
  `CollisionLayer::ALL.0`; `#[serde(default)]` tag → `""` (→ no `Tag` attached). A minimal zone def
  is `(pos: …, shape: …)`.
- **Versioning: MINOR (v0.87.0).** Additive types + 2 App methods, pre-1.0 → MINOR.

## Evidence & Data

### Commit / PR (this session)

| Hash | PR | Version | Seq | Summary |
|---|---|---|---|---|
| `492f945` (→ squashed `292030f`) | #282 | v0.87.0 | 120 | data-driven trigger zones — `TriggerZoneSet` + load/spawn |

### New public API surface (all additive / non-breaking)

| Symbol | Kind | Location |
|---|---|---|
| `TriggerZoneSet` (`from_ron_str`/`len`/`is_empty`/`tags`/`spawn_into`) | type | `trigger_zone.rs` |
| `TriggerZoneRegistry` (`load`/`insert`/`get`/`names`/`reload_path`) | World resource | `trigger_zone.rs` |
| `TriggerZoneSetError` (alias `AssetLoadError`) | error | `trigger_zone.rs` |
| `App::load_trigger_zones(name, path)` | App method | `app/editor/loading.rs` |
| `App::spawn_trigger_zones(name) -> Vec<Entity>` | App method | `app/editor/loading.rs` |

Crate-root re-export in `lib.rs`: `pub use trigger_zone::{TriggerShape, TriggerZone,
TriggerZoneRegistry, TriggerZoneSet, TriggerZoneSetError, TriggerZoneSystem, ZoneEvent};`.

### Tests added (4 + 1 doctest)

`trigger_zone::tests`: `set_parses_and_spawns_entities_with_zones_and_tags` (3-zone parse → spawn;
checks shape/mask/Transform/Tag, untagged zone → no `Tag`, mask defaults to `ALL`),
`set_malformed_ron_returns_err`, `spawned_zones_detect_overlap_end_to_end` (a spawned zone fires
`Entered` via the real grid+system path), `registry_insert_get_names`. Doctest: `TriggerZoneSet::
from_ron_str` (len + tags).

### Test counts

`trigger_zone::tests` 10 passed (4 new + 6 existing); 2 doctests in the file
(`TriggerZone` + `TriggerZoneSet`); full `cargo test --all-targets` 0 failures (2 environmental
audio tests skipped locally, green on CI).

### CI (PR #282 — 5-job matrix, all green)

| Job | Result | Time |
|---|---|---|
| Test (native) | pass | 4m34s |
| Build (WASM) | pass | 39s |
| Render tests (lavapipe) | pass | 1m36s |
| Rustdoc | pass | 45s |
| Package dry-run | pass | 1m8s |

### Headless render verification (Metal, `HEADLESS_SHOT=/tmp/data_trigger_zones.png`)

- Console: `entered heal  (total entries: 1)` → `exited heal  (total entries: 1)` → `entered damage
  (total entries: 2)` → `wrote /tmp/data_trigger_zones.png (130 frames)`.
- Screenshot: title `Data-driven TriggerZones — loaded from RON, spawned as entities`; HUD `inside:
  damage   zone entries: 2`; three RON-loaded zones (green heal rect, red damage [Circle → 128×128
  square debug quad, LIT], blue goal rect); white player quad overlapping the damage zone; footer
  hint `(edit data_trigger_zones.ron to retune)`.

Reproduce: `HEADLESS_SHOT=/tmp/data_trigger_zones.png cargo run --example data_trigger_zones` (native
GPU; `HEADLESS_FRAMES=N` overrides the 130-frame default). The RON path is relative to the workspace
root (cwd under `cargo run`), like `examples/gpu_particles.ron`.

## Code Analysis

- **`TriggerZoneSet::spawn_into`** (`src/trigger_zone.rs`): `self.defs.iter().map(|d| { let e =
  world.spawn(); world.add_component(e, Transform { position: d.pos.into(), ..Default::default() });
  world.add_component(e, TriggerZone { shape: d.shape.into(), mask: CollisionLayer(d.mask),
  occupants: Vec::new() }); if !d.tag.is_empty() { world.add_component(e,
  crate::prefab::Tag(d.tag.clone())); } e }).collect()`.
- **`App::spawn_trigger_zones`** (`app/editor/loading.rs`): `let set = self.world
  .resource::<TriggerZoneRegistry>().and_then(|reg| reg.get(name).cloned()); match set { Some(set) =>
  set.spawn_into(&mut self.world), None => { warn; Vec::new() } }` — the clone resolves the
  `&World`/`&mut World` borrow conflict.
- **Registry wiring** mirrors `ParticleConfigRegistry` 1:1: `RonRegistry<TriggerZoneSet>` inner,
  `RonLoadable for TriggerZoneSet` (`std::fs::read_to_string` → `from_ron_str`), `HotReloadable for
  TriggerZoneRegistry` (UFCS-delegates to the inherent `reload_path`), auto-registered in `App::new`.
- **Serde mirror conversions:** `Vec2Def(f32,f32) -> Vec2`, `TriggerShapeDef -> TriggerShape`
  (Circle/Rect), `TriggerZoneDef { tag: #[default] String, pos: Vec2Def, shape: TriggerShapeDef,
  mask: #[default=default_mask] u32 }`, `TriggerZoneDoc { zones: Vec<TriggerZoneDef> }`.
- **Example viz** sizes the debug quad with `viz_scale(shape)`: Circle → `Vec2::splat(radius*2)`
  (square approximation of the circular detection area — documented), Rect → `half_extents*2`. The
  zone entity's `Transform.scale`/`z` are set after spawn, then a `Sprite` is added.

## Gotchas & Discoveries

- **The data-driven config pattern is fully templated** — `RonRegistry<V>` + `RonLoadable` +
  `HotReloadable` + an `App::load_*` + auto-registration in `App::new`. A new data-driven config
  type is a near-mechanical fill-in of that template (`ParticleConfigSet` is the cleanest reference).
  No need to touch the hot-reload dispatch in `schedule.rs` — `register_hot_reloadable::<T>` in
  `App::new` is the only wiring.
- **Reuse `prefab::Tag` for naming** rather than minting a per-feature tag component — it is the
  engine's established named-entity component and is already serde/editor-registered + queryable.
- **`spawn_into` needs `&mut World` while the set lives in a `&World` resource** — clone the set out
  first (`TriggerZoneSet: Clone`). This is the standard escape from the resource-borrow-then-mutate
  conflict; the App helper hides it.
- **Changes were authored on `main` this session** (the data-driven work began right after the
  camera-lookahead handoff merge + `git checkout main`). `git checkout -b feat/…` carried the
  uncommitted changes onto the feature branch cleanly — but the safer habit is to branch *before*
  starting. No harm here (nothing was committed to main).
- **Environmental audio (standing):** locked/remote macOS has no audio device → 2 audio-device
  tests fail locally; `--skip` them and let CI gate audio. (CI #282's native job ran them green.)
- **zsh `${PIPESTATUS[0]}` is empty** (carried from seq 3/4) — read exit codes via `echo $?` on an
  unpiped command, or from the background-task completion notification.

## Files Changed

### Source — modified
- `src/trigger_zone.rs` — new "Data-driven trigger zones" section (mirror types + `TriggerZoneSet` +
  `TriggerZoneRegistry` + `TriggerZoneSetError` + `RonLoadable`/`HotReloadable`) + 4 tests + a doctest.
- `src/lib.rs` — re-export `TriggerZoneRegistry`, `TriggerZoneSet`, `TriggerZoneSetError`.
- `src/app.rs` — auto-register `TriggerZoneRegistry` as hot-reloadable in `App::new`.
- `src/app/editor/loading.rs` — `App::load_trigger_zones` + `App::spawn_trigger_zones`.

### Examples — new
- `examples/data_trigger_zones.ron` — heal/damage/goal zone defs.
- `examples/data_trigger_zones.rs` — load → spawn → debug-quad viz → walk; reacts by `Tag`,
  occupant-poll tint; `HEADLESS_SHOT` (130 frames).

### Docs / paperwork
- `CLAUDE.md` — `TriggerZone` row gained a "data-driven (RON)" clause; header v1.6.172 → v1.6.173 /
  package v0.86.0 → v0.87.0.
- `docs/CHANGELOG.md` — 0.87.0 entry.
- `Cargo.toml`, `Cargo.lock` — version bump + lock refresh.

## User Feedback & Preferences

- **"데이터 주도 트리거 존 진행해" → execute the chosen deeper candidate end-to-end** via the land-pr
  loop, autonomously, reporting outcomes.
- **Per-seq handoff cadence** — each code PR is followed by its own `docs(handoff)` PR; the user
  confirmed "1" (write this handoff + merge it) when offered.
- **Korean for all user-facing replies; English for code/docs/commits/handoffs** (project rule).
- **Merge authority delegated** — squash on green CI, no per-session re-confirm; PR #282 landed
  without asking.
- **Values evidence over assertion** — the headless render screenshot (with the lit damage zone +
  numeric HUD) was sent to the user as the acceptance artifact, alongside CI numbers and test counts.

## Where We're Going

The `breadth-features` chain has shipped **seven** features this run (SpriteFlip, YSort,
AnimationEvents, TriggerZone, HitFlash, CameraLookahead, **data-driven TriggerZones**). The easy
component+system breadth is largely covered, and the first deeper data-driven item is done. **Read
`../dungeon-merchant/docs/engine-wishlist.md` FIRST each session** (ACTIVE EMPTY, EW-004 next) — a
real downstream request outranks self-picked work. Remaining candidates, roughly ordered:

1. **More RON-authored content / effects** — the data-driven config pattern is now proven for zones;
   natural next data-driven targets if a game asks: an event→effect binding (e.g. a `ZoneEvent`/
   animation-event → spawn-a-particle-burst / play-a-sound helper), or richer zone defs (per-zone
   payload, an `on_enter`/`on_exit` effect). Design these against a concrete game need, not
   speculatively.
2. **Editor i18n gap** (lower value) — `editor/ui/audio_panel.rs` English status strings bypass
   `i18n::tr(en, ko)`; editor-only, not CI-render-verifiable.
3. **A Tier-2 hardcoding knob on a concrete request** — the remaining knobs are weak/compromised
   (MAX_GAMEPADS, material params >4, editor app-id, frame latency); do one only if asked.

Otherwise: **ASK the user for direction** when the board is empty — the obvious breadth is done, and
the high-value next work is likely a real downstream wishlist item or a deeper subsystem.

**The data-driven config template (now applied a 4th time — particle/dialogue/anim-clip/trigger-zone):**
private serde-mirror types → a `from_ron_str` value type → a `RonRegistry`-backed registry with
`RonLoadable` + `HotReloadable` → an `App::load_*` (lazy-insert + `watch_path`, no-op on wasm) →
auto-register in `App::new`. Adding another data-driven type is a near-mechanical fill-in.

## Risks & Blockers

- **Audio-device tests fail locally** (environmental, not a regression) — `--skip` the two named
  audio tests. CI gates audio.
- **The example's debug quad approximates a Circle zone as a square** (the detection is circular; the
  viz is a square covering it). Documented; cosmetic — the logic is correct (the `spawned_zones_
  detect_overlap_end_to_end` test covers a circle).
- **`spawn_into` does not deduplicate or clear prior zones** — calling `spawn_trigger_zones` twice
  spawns the zones twice. A game that respawns a level should despawn the old zone entities first.
  Not handled (out of scope); worth noting if a "reload the level" flow is built.
- **No OS-gated code this session** — everything is cross-platform (the lavapipe render job
  exercised the GPU path; the data layer is pure logic + file I/O). `load_trigger_zones` is a no-op
  on wasm (no fs), which is the documented and tested behavior.
- No upstream/dependency blockers. Tree clean.

## Open Questions

- Should zone defs carry an **effect/payload** (e.g. `on_enter: SpawnParticles("blood")` or a damage
  value)? Kept minimal (tag/pos/shape/mask) — a game maps the tag to behavior in code. A richer
  def + an event→effect binding is the natural next data-driven step (see "Where We're Going").
- Should `spawn_into` optionally attach a **debug `Sprite`** (gated by a feature/flag) so a zone is
  visible without game code? Kept it logic-only; the example shows the decorate step.
- Should there be a **despawn/replace** helper for reloading a level's zones? Not built; a game
  tracks the returned entities and despawns them itself.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5            # confirm main tip (data-driven trigger zones #282 + this handoff PR)
git status -s                   # clean

# 1) Read the game↔engine board FIRST (drives priority)
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick

# Memory (full per-seq detail of this session)
#   ~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md (seq 120 tip)

# Key files if continuing data-driven / breadth work
#   src/trigger_zone.rs (Data-driven section)     — the pattern just shipped (RON set + registry)
#   src/particle/config_set.rs + src/ron_registry.rs  — the data-driven config template (reference)
#   src/app/editor/loading.rs                     — App::load_*/spawn_* live here
#   src/app/editor/ui/audio_panel.rs              — the editor i18n gap (a remaining low-value candidate)

# Verify (NOTE: 2 audio-device tests fail locally — environmental; read exit via `echo $?`, not ${PIPESTATUS[0]} in zsh)
cargo test --lib -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Reproduce this session's example
HEADLESS_SHOT=/tmp/data_trigger_zones.png cargo run --example data_trigger_zones

# Next action
#   Read the wishlist board; if still empty, ASK for direction (easy breadth is done; next is likely
#   a real downstream request or deeper data-driven/effect work — design against a concrete need).
```

## Session Closed

**Closed:** 2026-06-29
**Chain:** `breadth-features` seq 5 — continuation of `HANDOFF_breadth-features_camera-lookahead_2026-06-29.md` (seq 4)
**Code landed:** #282 (v0.87.0), main @ `292030f`. This handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off. Seven breadth features shipped this run; the first deeper data-driven item is done. Next session starts from the wishlist board or asks for direction.
