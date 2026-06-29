# Trigger zones — `TriggerZone` + `TriggerZoneSystem` + `ZoneEvent` shipped (v0.84.0, PR #276), second breadth feature of the chain

**Date:** 2026-06-29
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `breadth-features` seq `2`
**Parent:** `HANDOFF_breadth-features_animation-events_2026-06-29.md` (seq 1)
**Prior chain:** `hardcoding-audit` (closed) → pivoted to `breadth-features` at its seq 4

> Chain rationale: this session was started by a paste prompt pointing at the `breadth-features` seq-1 handoff (animation events) and told to "continue from Where We're Going". That section listed **trigger zones as the #1 recommended next candidate** — which is exactly what this session built. So this is a **direct continuation**: `breadth-features` seq 2, parent = the seq-1 handoff.

## Related Handoffs

- `HANDOFF_breadth-features_animation-events_2026-06-29.md` — **the parent** (`breadth-features` seq 1, #274 animation events). Its "Where We're Going" recommended trigger zones first and flagged the **`TriggerEvent` name clash** (physics rapier already has it → use `ZoneEvent`). Read it for the breadth-pivot rationale and the validated 8-step feature pattern.
- `HANDOFF_hardcoding-audit_audit-closed-breadth-pivot_2026-06-29.md` — `hardcoding-audit` seq 4, the breadth pivot's origin (SpriteFlip #271 / YSort #272). The "add a component the way RenderLayer is registered" pattern this session reused.

## Reference Documents

- `CLAUDE.md` — project conventions + module map (a `TriggerZone` row was added under the collision row this session). Header bumped to **v1.6.170** / package **v0.84.0**.
- `docs/CHANGELOG.md` — the 0.84.0 entry written this session.
- `docs/PATTERNS.md` — ECS query API + render-layer separation (not touched, but the canonical pattern doc).
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — bumped to **seq 117** (the most detailed per-seq record of this session lives in that bullet; seq 75 folded into the rollup to keep it compact).
- `../dungeon-merchant/docs/engine-wishlist.md` — the game↔engine board; **ACTIVE EMPTY** (next free ID EW-004). Read FIRST next session.

## The Goal

Continue the `breadth-features` pivot — adding genuinely-missing 2D-engine breadth that a downstream game would otherwise hand-roll. The wishlist board was empty, so per the parent's plan the user was offered breadth candidates and **chose "트리거 존 (권장)"** (trigger zones) via AskUserQuestion. Trigger zones = the 2D "Area2D" pattern: a region that fires enter/stay/exit events as entities overlap it (pickups, damage fields, room/door triggers, aggro ranges, checkpoints), the idiomatic alternative to hand-polling overlaps every frame. The acceptance test (per the VISION loop) is a small playable example that exercises it in real play.

## Where We Are

- **main @ `23fd62a`** (package **v0.84.0**, CLAUDE.md header **v1.6.170**), tree **clean**, **no open PRs**.
- **PR #276 merged** (squash + branch-deleted, CI **5/5** green): `feat(collision): trigger zones — TriggerZone + TriggerZoneSystem + ZoneEvent (v0.84.0)`.
- **New module** `src/trigger_zone.rs` — `TriggerShape { Circle { radius }, Rect { half_extents } }`, `TriggerZone { shape, mask, occupants }` (component, ctors `circle`/`rect`, builder `with_mask`, `contains`), `ZoneEvent { Entered/Stayed/Exited { zone, other } }`, `TriggerZoneSystem`.
- **`TriggerZoneSystem` logic:** reads the `SpatialGrid` resource (mirrored by `CollisionGridSystem`), queries each zone's region (`query_radius` for circle, `query_aabb` for rect) with the zone's `mask`, diffs the current overlap set against the component's stored `occupants`, emits `ZoneEvent` for each transition (Entered/Exited) and persistence (Stayed), writes the new occupants back, then flushes events.
- **New public API** (all additive, non-breaking): `engine::TriggerZone`, `engine::TriggerShape`, `engine::TriggerZoneSystem`, `engine::ZoneEvent` (crate-root re-exports).
- **Registration:** `register_clone::<TriggerZone>` in `core_resources.rs`; editor add/remove in `editor/component_registry.rs` — mirrors `YSort`/`SpriteFlip` (clone + editor add/remove). **NOTE:** unlike YSort/SpriteFlip/AnimationEvents, `TriggerZone` does **NOT** derive `Serialize`/`Deserialize` (it embeds `occupants: Vec<Entity>` runtime state and reuses `CollisionLayer`, which has no serde) — no serde registration was done, and none is needed for clone+editor.
- **New example** `examples/trigger_zones.rs` (flat, auto-discovered) — a player walks left→right through a heal/damage/goal zone; **events** drive an entry counter, **polling** `occupants` tints each zone as it's occupied (shows both consumption styles). `HEADLESS_SHOT` support (auto-drifts the player; default 130 frames).
- **Tests:** lib at **971 passed** (skipping 2 environmental audio), up from 965 at session start — **+6 new** (all in `trigger_zone::tests`) + 1 doctest.
- **CI:** PR #276 passed the 5-job matrix (Test native 5m9s / Build WASM 48s / Render lavapipe 1m28s / Rustdoc 34s / Package dry-run 1m32s). The native job ran the audio tests green (confirming the 2 local failures are environmental); the lavapipe render job gates the GPU path.
- **Headless render verified on Metal:** console fired `entered heal → exited heal → entered damage`; the screenshot showed the HUD `inside: damage / zone entries: 2` and the damage zone lit (player overlapping its left edge).
- **CLAUDE.md module-map** got a `TriggerZone` row under the collision row.
- **Memory** `engine-current-state.md` bumped to seq 117; seq 75 folded into the "Older seqs" rollup to keep the file compact.

## What We Tried (Chronological)

1. **Onboarding (narrated, 5-point).** Read the parent handoff (breadth-features seq 1), confirmed the wishlist board is ACTIVE EMPTY. Read the listed key files: `animation/events.rs`, `ysort.rs`, `ecs/events.rs`, plus the handoff's suggested adjacent files `camera.rs` (lookahead candidate) and `collision/{mod,query,grid}.rs` (trigger-zone substrate). **Key discovery:** `SpatialGrid` already has `query_radius(center, r, mask)` / `query_aabb(min, max, mask)` returning layer-matching overlapping entities, and `CollisionGridSystem` mirrors the grid into the World as a resource every frame → a perfect substrate for a per-frame overlap-diff zone system. Confirmed (grep) the physics `TriggerEvent` already exists → new event must be named differently.
2. **Baseline test** (`965 passed`, 2 audio filtered) — matched the parent handoff.
3. **AskUserQuestion** (trigger zones recommended / hit-flash / camera lookahead) → user chose **"트리거 존 (권장)"**.
4. **API fact-finding (grep/read).** `query2::<A,B>() -> impl Iterator<Item=(Entity,&A,&B)>`; `get_mut::<T>(e) -> Option<&mut T>`; **`CollisionGridSystem::new(cell)` self-inserts a `SpatialGrid` if absent** (so the example just adds it before the zone system); **`CollisionLayer(pub u32)` derives Debug/Clone/Copy/PartialEq/Eq but NOT Default/Serialize**, and **`Collider` derives only Debug/Clone/Copy** (no Default/serde/PartialEq); `Sprite.color` is a `pub Color` field (easy tinting); `Events` is `crate::ecs::Events` (re-exported `engine::Events`); `Color::rgb` exists.
5. **Wrote `src/trigger_zone.rs`** — the 4 types + the system + 6 unit tests + a doctest. Chose `TriggerShape` as its own enum (not `Collider`) and `occupants` on the component (not the system).
6. **Wired** `lib.rs` (`pub mod trigger_zone` + crate-root re-export), `core_resources.rs` (`register_clone`), `editor/component_registry.rs` (add + remove).
7. **Wrote `examples/trigger_zones.rs`** — auto-patrol-in-headless / arrow-control-live player + 3 rect zones; reacts via events (counter) and polling (tint).
8. **Compiled + tested incrementally.** 6 unit tests pass; example builds; doctest passes; **headless render on Metal** confirmed the console enter/exit sequence + the screenshot.
9. **CLAUDE.md module-map row** added.
10. **Verify (full `verify.sh`).** `cargo fmt` first (avoid the reflow trap), then `verify.sh` → `VERIFY_EXIT=101` from **only** the 2 environmental audio tests (971 passed). verify aborts at the test step (before rustdoc).
11. **rustdoc gate (run separately, since verify aborted before it) RED → fixed.** `RUSTDOCFLAGS="-D warnings" cargo doc` failed with `rustdoc::redundant_explicit_links` on `[`CollisionLayer`](crate::CollisionLayer)` / `[`SpatialGrid`](crate::SpatialGrid)` — because the file `use`s those types, the short form `[`CollisionLayer`]` already resolves, making the explicit target redundant. Converted those to the short link form (left `Collider`/`TriggerEvent`, which are NOT imported, and `CollisionLayer::ALL`, which wasn't flagged). rustdoc green.
12. **`/ship`** → v0.83.0 → **v0.84.0** (MINOR): Cargo.toml + `cargo update -p skeleton-engine` (lock) + CHANGELOG 0.84.0 + CLAUDE.md header v1.6.170. **Re-ran full verify** post-bump → `VERIFY_EXIT=101` again (audio only, 971 passed).
13. **`/land-pr`** → branch `feat/trigger-zones`, commit `28f75e4`, push, PR **#276**, watched CI (5/5 CLEAN), confirmed `mergeStateStatus == CLEAN`, squash-merged `23fd62a`, synced main, bumped memory to seq 117 (folded seq 75 into the rollup).

## Key Decisions

- **`TriggerShape` is its own enum, NOT the existing `Collider`.** `Collider` derives only `Debug/Clone/Copy` — no `Default`, `PartialEq`, or serde — so embedding it would force manual impls or block deriving on `TriggerZone`. A dedicated `TriggerShape { Circle { radius }, Rect { half_extents } }` (with its own `Default` = `Circle { radius: 32.0 }`) keeps the component self-contained and decoupled from the collision module's derive set. Rejected: `shape: Collider`.
- **`occupants` lives on the COMPONENT, not in the system.** This gives a game the Area2D-style polling ergonomic (`zone.occupants` / `zone.contains(entity)` = "who's inside right now") in addition to the event stream. Cost: a stateless system (only `warned_*` flags) means a fresh `TriggerZoneSystem::default()` per frame still works (occupants persist on the component) — handy for tests. Documented `occupants` as **runtime-managed / read-only from game code**. Minor accepted edge: `register_clone`/editor-copy carries occupants, which could emit one spurious `Exited` next frame; benign, self-corrects. Rejected: a `HashMap<Entity,Vec<Entity>>` of prev-occupants in the system (loses polling).
- **`mask` reuses `CollisionLayer`; `TriggerZone` has a manual `Default`.** `CollisionLayer` has no `Default` derive, and a derived `CollisionLayer(0)` = `NONE` would detect nothing — a bad default for an editor-added zone. So `TriggerZone::default()` is hand-written: `mask: CollisionLayer::ALL`, `shape: Circle { radius: 32.0 }`, `occupants: empty`.
- **All three of Entered / Stayed / Exited are emitted.** Stayed fires every frame an entity remains (useful for damage-over-time; a game can ignore it). The diff computes it for free, and `occupants` polling is the alternative for "who's inside" without the per-frame event. Tense matches the physics `TriggerEvent` (`Entered`/`Exited`).
- **`TriggerZone` does NOT derive serde** (departs from YSort/SpriteFlip/AnimationEvents, which do). It embeds runtime `occupants` and reuses `CollisionLayer` (no serde). clone + editor registration needs only `Clone` + `Default`, so serde was deliberately omitted. If scene-persistence is ever wanted, add serde to `CollisionLayer` + `TriggerShape` and `#[serde(skip)]` the occupants.
- **3-phase borrow in `TriggerZoneSystem::run`.** Phase 1 holds `grid = world.resource::<SpatialGrid>()` AND iterates `world.query2::<Transform, TriggerZone>()` (both shared `&World` borrows — they coexist), building owned `pending: Vec<ZoneEvent>` + `updates: Vec<(Entity, Vec<Entity>)>`. Phase 2 (`&mut World`) writes occupants back (the `grid` borrow has ended via NLL — its last use is in phase 1). Phase 3 flushes `pending` into `Events<ZoneEvent>`. This avoids any borrow conflict without `collect`-then-`get_mut` gymnastics.
- **Graceful degradation, not panic.** Missing `SpatialGrid` resource → `warned_no_grid` one-time `warn!` + skip. Missing `Events<ZoneEvent>` bus → `warned_no_bus` one-time `warn!` + drop (but occupants still update, so polling works without the bus). Events are opt-in, like the physics CollisionEvent ergonomics.
- **`ZoneEvent` name is deliberately distinct from physics `TriggerEvent`.** As the parent handoff flagged. This is a lightweight, grid-based, physics-free zone.
- **Versioning: MINOR (v0.84.0).** Additive feature, pre-1.0 → MINOR (same as SpriteFlip 0.81.0 / YSort 0.82.0 / AnimationEvents 0.83.0).
- **Example zones are Rect, drawn as colored quads matching the detection area.** A `Sprite::colored` quad renders a rectangle, so Rect zones make the visualization honest (square sprite = square zone). The unit tests cover Circle separately. The example defaults to 130 headless frames and auto-drifts the player rightward (headless has no input) so the capture lands inside the damage zone.

## Evidence & Data

### Commit / PR (this session)

| Hash | PR | Version | Seq | Summary |
|---|---|---|---|---|
| `28f75e4` (→ squashed `23fd62a`) | #276 | v0.84.0 | 117 | trigger zones — `TriggerZone` + `TriggerZoneSystem` + `ZoneEvent` |

### New public API surface (all additive / non-breaking)

| Symbol | Kind | Location |
|---|---|---|
| `TriggerZone { shape, mask, occupants }` | component | `trigger_zone.rs` |
| `TriggerZone::circle(r)` / `rect(half)` / `with_mask(mask)` / `contains(e)` | ctors+builder+poll | `trigger_zone.rs` |
| `TriggerShape { Circle { radius }, Rect { half_extents } }` | enum | `trigger_zone.rs` |
| `ZoneEvent { Entered/Stayed/Exited { zone, other } }` | emitted event | `trigger_zone.rs` |
| `TriggerZoneSystem` | system (user-added) | `trigger_zone.rs` |

Crate-root re-exports added to `lib.rs`: `pub use trigger_zone::{TriggerShape, TriggerZone, TriggerZoneSystem, ZoneEvent};` (placed before the `ysort::{…}` re-export).

### Tests added (6 + 1 doctest)

`trigger_zone::tests`: `enter_stay_exit_lifecycle` (full 4-frame Entered→Stayed→Exited with occupants checks), `mask_filters_by_layer` (enemy-layer actor ignored, player-layer reported), `rect_zone_detects_overlap`, `zone_does_not_report_itself` (zone with its own Collider), `unregistered_bus_drops_events_but_updates_occupants_and_warns_once`, `missing_grid_warns_once_and_is_inert`. Doctest: `TriggerZone::circle(64.0).with_mask(...)`.

### Test counts

`965 passed` (session start) → `971 passed` (+6), 2 environmental audio tests skipped/filtered locally, all green on CI.

### CI (PR #276 — 5-job matrix, all green)

| Job | Result | Time |
|---|---|---|
| Test (native) | pass | 5m9s |
| Build (WASM) | pass | 48s |
| Render tests (lavapipe) | pass | 1m28s |
| Rustdoc | pass | 34s |
| Package dry-run | pass | 1m32s |

### Headless render verification (Metal, `HEADLESS_SHOT=/tmp/trigger_zones.png`)

- Console: `entered heal  (total entries: 1)` → `exited heal  (total entries: 1)` → `entered damage  (total entries: 2)` → `wrote /tmp/trigger_zones.png (130 frames)`.
- Screenshot: title; HUD `inside: damage   zone entries: 2`; three zones — green **heal** (dim), red **damage** (LIT, player overlapping its left edge), blue **goal** (dim); white player quad; `Arrows: move player   Esc: quit`.

Reproduce: `HEADLESS_SHOT=/tmp/trigger_zones.png cargo run --example trigger_zones` (native GPU; works monitor-off via the surfaceless path; `HEADLESS_FRAMES=N` overrides the 130-frame default).

## Code Analysis

- **`TriggerZoneSystem::run`** (`src/trigger_zone.rs`): (1) `let Some(grid) = world.resource::<SpatialGrid>() else { warn-once; return };`. (2) loop `for (zone_e, transform, zone) in world.query2::<Transform, TriggerZone>()` — match `zone.shape` → `grid.query_radius(center, r, zone.mask)` or `grid.query_aabb(center - half, center + half, zone.mask)`; `current.retain(|&e| e != zone_e)` (never report self); diff vs `zone.occupants` → push `Entered`/`Stayed`/`Exited` into `pending`; record `(zone_e, current)` into `updates`. (3) phase 2: `for (zone_e, current) in updates { if let Some(z) = world.get_mut::<TriggerZone>(zone_e) { z.occupants = current; } }`. (4) phase 3: if `pending` non-empty, `world.resource_mut::<Events<ZoneEvent>>()` → send all, else warn-once.
- **Borrow subtlety:** `grid` (a `&SpatialGrid` into `world`) and `world.query2()` (another `&World`) are both shared borrows → legal together. `grid`'s last use is in the phase-1 loop, so NLL releases it before phase 2's `&mut`. `pending`/`updates` are owned locals. `warned_*` are `&mut self` fields, mutated only after the loop (no conflict with the `&self.scratch`-style borrow — there's no scratch here, the query borrow is `world` not `self`).
- **`SpatialGrid`** (`src/collision/grid.rs`): `query_radius` / `query_aabb` take a region + a `CollisionLayer` mask, return `Vec<Entity>` of entities whose `mask.matches(entry.layer)` and whose collider intersects. Watched entities enter the grid via `rebuild` (reads `(Transform, Collider)`; layer defaults to `ALL` if no `CollisionLayer`). `CollisionGridSystem::new(cell).run` does remove→rebuild→insert (self-creates the resource if absent), and exposes `LABEL = "engine::collision_grid"` for `.after(...)` scheduling.
- **Registration template** (confirmed by grep): `RenderLayer`/`SpriteFlip`/`YSort` = `register_clone` + editor add/remove. `TriggerZone` follows that minus serde (see Key Decisions).
- **Example movement model:** the `Demo` system takes `auto: bool` (= `HEADLESS_SHOT` is set). When `auto`, horizontal drift is `self.dir * SPEED * dt` (ping-pongs at the edges); else `(right - left) * SPEED * dt`. Vertical is always arrow-driven. This makes headless deterministic (no input) while keeping live play interactive.

## Gotchas & Discoveries

- **NEW reusable gotcha — `rustdoc::redundant_explicit_links` fires only for IN-SCOPE types.** `[`Foo`](crate::Foo)` is flagged redundant when `Foo` is `use`d in the file (the short `[`Foo`]` already resolves). `trigger_zone.rs` imports `CollisionLayer`/`SpatialGrid` → their explicit-target links were redundant. `ysort.rs`'s `[`RenderLayer`](crate::RenderLayer)` is fine **because ysort.rs does NOT import `RenderLayer`** (the short form wouldn't resolve, so the explicit target is needed). **Rule:** for a type your module imports, use the short intra-doc link `[`Type`]`; keep the explicit `(crate::Type)` only for types not in scope. `CollisionLayer::ALL` (associated const) was NOT flagged — leave such links explicit.
- **`verify.sh` aborts at the test step before rustdoc.** Because the 2 environmental audio tests fail (`VERIFY_EXIT=101`), `verify.sh` stops at `cargo test --all-targets` and never reaches the rustdoc gate. Run `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` **separately** to stay CI-faithful (CI runs it as its own job).
- **`CollisionLayer` / `Collider` lack `Default`/serde/`PartialEq`.** A component embedding them can't blindly `#[derive(Default, Serialize, …)]`. Either add the derives to the collision types (additive) or hand-write `Default` + skip serde (what this PR did).
- **`CollisionGridSystem` self-inserts the `SpatialGrid`** and must be added **before** `TriggerZoneSystem` (the zone system reads the resource the grid system populates). The example adds `CollisionGridSystem::new(128.0)` then `TriggerZoneSystem::default()` then `Demo`.
- **Headless render has no input** (`dt = 1/60` per frame, no key events), so an interactive example must self-drive in headless mode — the `auto` flag drifts the player. (Same lesson as the animation_events example's 60-frame warmup, generalized to movement.)
- **`verify.sh` exit masking (recurring):** a backgrounded `./scripts/verify.sh > log 2>&1; echo "VERIFY_EXIT=$?"` makes the task-completion notification report the trailing `echo`'s `0`, hiding `verify.sh`'s real `101`. ALWAYS read `VERIFY_EXIT=` from the task-output file, not the notification summary.
- **Environmental audio (standing):** locked/remote macOS has no audio device → `play_tone_reports_playing_then_finished_when_audio_device_exists` + `stop_on_drained_sink_is_immediate` always fail locally; never a regression. `--skip` them (or read `VERIFY_EXIT` + confirm only those two) and let CI gate audio.
- **rust-analyzer false positives reconfirmed:** `ColliderHandle` "expected X, found X" in physics examples + `cfg(wasm32)` inactive-code warnings are pre-existing noise — trust `cargo check`/clippy/CI.

## Files Changed

### Source — new
- `src/trigger_zone.rs` — `TriggerShape` + `TriggerZone` (+ ctors/builder/`contains`) + `ZoneEvent` + `TriggerZoneSystem` + 6 tests + doctest.

### Source — modified
- `src/lib.rs` — `pub mod trigger_zone;` + crate-root re-export of the 4 symbols.
- `src/app/core_resources.rs` — `register_clone::<TriggerZone>`.
- `src/app/editor/component_registry.rs` — editor add + remove for `TriggerZone`.

### Examples — new
- `examples/trigger_zones.rs` — heal/damage/goal walk; events→entry counter, occupant-polling→zone tint; `HEADLESS_SHOT` (auto-drift, 130 frames).

### Docs / paperwork
- `CLAUDE.md` — `TriggerZone` module-map row added under the collision row; header v1.6.169 → v1.6.170 / package v0.83.0 → v0.84.0.
- `docs/CHANGELOG.md` — 0.84.0 entry.
- `Cargo.toml`, `Cargo.lock` — version bump + lock refresh.

## User Feedback & Preferences

- **Chose the feature explicitly via AskUserQuestion** — picked "트리거 존 (권장)" from the 3 offered breadth candidates. With an empty board, the user expects me to surface options and let them pick.
- **Go-ahead = the feature choice → execute end-to-end.** After the onboarding narration + the AskUserQuestion answer, the expectation was the full land-pr loop run autonomously, reporting outcomes not options.
- **`/handoff 하고 푸시`** — drives the per-session cadence with short Korean go-aheads; the handoff lands as its own `docs(handoff)` PR.
- **Korean for all user-facing replies; English for code/docs/commits/handoffs** (project doc-language rule).
- **Merge authority delegated** — squash on green CI, no per-session re-confirm; PR #276 landed without asking.
- **Prefers the full land-pr loop per change** — branch → verify → /ship → PR → watch CI → squash-merge → sync → bump memory seq — run without narrating each option.
- **Wants the onboarding narrated** before executing (this session did the 5-point onboarding then asked which feature) — but once go-ahead is given, execute end-to-end.
- **Values evidence over assertion** — verification (headless render, CI numbers, test counts) reported with real numbers.

## Where We're Going

The `breadth-features` chain continues until the wishlist board fills. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST each session** (ACTIVE EMPTY, EW-004 next) — a real downstream request outranks self-picked breadth. If empty, ASK the user which breadth feature, then land it via the land-pr loop. Remaining ordered self-pick candidates (each: additive component/system + playable example + tests):

1. **Hit-flash** (the parent's #2) — a brief tint/white-flash on a sprite when hit (every action game re-implements). A `HitFlash { color, secs }` component + a system that lerps the sprite color and removes itself when done; pairs naturally with the new `ZoneEvent` (a damage zone) or animation events / a damage event. Likely the easiest next one (pure logic + sprite color, fully CI-verifiable via the render test).
2. **Tweened camera lookahead** (the parent's #3) — bias the camera ahead of a moving follow target. Read `src/camera.rs` (`follow_entity`/`lerp_factor`, `update`, `bounds`/`clamp_to_bounds`); the follow happens in `Camera::update(dt, follow_pos)`. Would add a lookahead offset derived from the target's velocity/facing. Note the camera's `update` is App-driven, so this may need a new field on `Camera` or a small system.
3. Lower value: the editor i18n gap (`editor/ui/audio_panel.rs` — English status strings bypass `tr()` in the Korean-default editor), or a deferred Tier-2 hardcoding knob on a concrete request.

**The validated 8-step pattern (now 4× — SpriteFlip, YSort, AnimationEvents, TriggerZone):** (a) grep to confirm the gap + read the subsystem; (b) add a component (+ system/event if needed) in its own module; (c) register clone + editor add/remove like `RenderLayer`/`YSort`; (d) re-export in `lib.rs`; (e) a flat `examples/<name>.rs` with `HEADLESS_SHOT` (self-drive in headless if interactive); (f) unit tests + doctest; (g) CLAUDE.md module-map row; (h) land via the land-pr loop (MINOR), `--skip` the 2 audio tests locally + run rustdoc separately (verify aborts before it).

## Risks & Blockers

- **Audio-device tests fail locally** (environmental, not a regression) — `--skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate`, read `VERIFY_EXIT=` from the log. CI gates audio.
- **`occupants` on clone** (by design) — `register_clone`/editor-copy of an active zone carries its occupants, which could emit one spurious `Exited` next frame. Benign + self-corrects; documented. If it ever matters, store prev-occupants in the system instead.
- **`Stayed` fires every frame** an entity remains (by design). A game wanting only transitions ignores `Stayed` (or polls `occupants`). If per-frame noise is unwanted, a future `emit_stay: bool` on `TriggerZone` could gate it.
- **No OS-gated code this session** — everything is cross-platform (the lavapipe render job exercised the GPU path; trigger zones are pure logic). The standing rule still holds for future work: green CI ≠ verified for `cfg(target_os)` paths.
- No upstream/dependency blockers. Tree clean, no open PRs.

## Open Questions

- Should trigger zones be **data-driven** (authored in RON, like particle/dialogue/anim-clip configs)? Natural future extension — a `RonLoadable` zone config that spawns `TriggerZone`s. Out of scope for this PR.
- Should `ZoneEvent` carry a **richer payload** (e.g. the overlap depth, or the entity's position) instead of just `{ zone, other }`? Kept minimal; extend if a game needs it.
- Should `Stayed` be **opt-in** per zone to avoid per-frame event volume in scenes with many large zones? Left always-on (cheap, and ignorable).
- Should the **default `TriggerShape`** be `Circle { radius: 32 }` or a rect? Chose circle (aggro/pickup radius is the more common zone); editor-added zones get a usable 32px circle.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4            # confirm main @ 23fd62a (v0.84.0)
git status -s                   # clean

# 1) Read the game↔engine board FIRST (drives priority)
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick breadth

# Memory (full per-seq detail of this session)
#   ~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md (seq 117 tip)

# Key files if continuing breadth features
#   src/trigger_zone.rs                          — the pattern just shipped (component + system + Events, grid-backed)
#   src/ysort.rs + src/components.rs (SpriteFlip) — prior breadth patterns (serde-deriving variants)
#   src/collision/{grid,query}.rs                — SpatialGrid query API (substrate for overlap features)
#   src/camera.rs                                — for "camera lookahead" (follow_entity/update/bounds)
#   src/components.rs (Sprite.color)             — for "hit-flash" (lerp the color, remove when done)

# Verify (NOTE: 2 audio-device tests fail locally — environmental; rustdoc runs as its own gate after verify aborts)
cargo test --lib -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Reproduce this session's example
HEADLESS_SHOT=/tmp/trigger_zones.png cargo run --example trigger_zones

# Next action
#   Read the wishlist board; if still empty, ASK which breadth feature
#   (recommended next: hit-flash — easiest, pairs with ZoneEvent damage zones), then land it via the land-pr loop.
```

## Session Closed

**Closed:** 2026-06-29
**Chain:** `breadth-features` seq 2 — continuation of `HANDOFF_breadth-features_animation-events_2026-06-29.md` (seq 1)
**Code landed:** #276 (v0.84.0), main @ `23fd62a`. This handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off. Next session starts from the wishlist board or a new breadth feature (see "Where We're Going").
