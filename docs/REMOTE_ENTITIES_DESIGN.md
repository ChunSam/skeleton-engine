# RemoteEntities — design notes (for a future deep-dive)

> Status: **minimal helper shipped** (`engine::RemoteEntities<K>`, `src/network.rs`, v4.2.x).
> A *richer* version is **deliberately deferred** — this doc captures the context so the design
> can be reconsidered later without re-deriving it. Chain: `networking-dogfood` (seq 3, Phase 2).
>
> ⚠️ **Every call site cited below was deleted on 2026-08-19** with the `examples/` tree, so the
> helper now ships with **zero consumers** and every "revisit with a 3rd example" gate in this
> document is unreachable. The evidence is preserved rather than deleted: it is what the design
> was derived from, and a rebuilt networked game should be checked against it before anyone
> concludes the minimal slice is still right. Recover the sources from git history.

## What shipped (the minimal slice)

`RemoteEntities<K: Eq + Hash>` owns just the `id → Entity` lifecycle that every networked game
otherwise reimplements inline:

| method | purpose |
|---|---|
| `get_or_spawn(world, key, spawn)` | return the entity for `key`, spawning + inserting via the closure on first sight |
| `get(&key) -> Option<Entity>` | look up without spawning |
| `contains_key(&key)` | membership |
| `remove(world, &key)` | remove from the map **and despawn** (no-op if absent) |
| `clear(world)` | despawn every tracked entity (e.g. on disconnect / scene reset) |
| `len` / `is_empty` / `iter` | inspection |

It owns **only** the mapping + spawn/despawn lifecycle. Three things stay in the game:
1. **What** to spawn — the `spawn` closure (color, size, components differ per game/entity type).
2. **How** to update an existing entity — call `get`, then mutate via the `World` (e.g. set
   `Transform.position`). Updates are not part of the helper.
3. **Parallel game state** — maps like coin_race's `coin_pos` / `claimed` / `scores` are separate.

## Current call sites (the design evidence — only two examples, hence the deferral)

| site | map(s) | spawn | update | remove trigger |
|---|---|---|---|---|
| `examples/mp_client.rs` | `remote_players` | 32px colored square on first `Position` | set `Transform.position` each `Position` | `Bye` |
| `examples/games/coin_race` | `remote_players` | rival square on first `Position` | set position each `Position` | `Bye` |
| `examples/games/coin_race` | `coins` | gold square on `Hello`(bulk) / `Coin`(single) | none (coins are position-static) | `Taken` |

So the minimal helper is exercised **three** ways across **two** examples. All three are the same
shape: `HashMap<usize, Entity>`, JSON protocol, "spawn a colored square, maybe update its
position, despawn on a removal message". That sameness is exactly why the richer abstraction is not
yet designed — see below.

## Why the richer version is deferred (NOT a TODO to grab blindly)

The two examples are **structurally near-identical**, so they cannot reveal whether a richer API's
shape is right. Adding speculative features now bakes assumptions into a **public, semver-bound**
API of a *skeleton* engine, where a wrong API is a worse liability than the ~10 lines of
duplication it would remove. The bar to revisit is **a third, genuinely *distinct* networked
example** that stresses the abstraction in a new direction.

## Open design questions for a richer `RemoteEntities` (revisit with a 3rd example)

1. **Interpolation / smoothing.** Remote positions arrive at the network tick rate (e.g. 10–20 Hz)
   but render at 60 Hz. A richer helper might store `(prev, next, t)` per entity and expose an
   interpolated transform. Open: does the helper own the interpolation buffer, or just the map and
   the game interpolates? (mp_client/coin_race snap — no signal on the buffer shape.)
2. **Client-side prediction + reconciliation** for the *local* entity (not remote). This is a
   different concern (local input replay vs. server correction) — possibly a *separate* type, not
   part of `RemoteEntities`. A 3rd example with prediction would clarify the boundary.
3. **Per-entity update callback.** `get_or_spawn` + manual update works for "one transform". An
   entity with many synced fields might want an `upsert(world, key, spawn, update)` form. Risk:
   over-fitting to a 2-field case. Needs an example with richer per-entity state.
4. **Typed / multiple entity classes.** coin_race already keeps two `RemoteEntities` (players,
   coins). Is N separate maps the right model, or one keyed by `(kind, id)`? Two maps is fine so
   far; a game with many entity kinds would test this.
5. **Staleness / generation.** No timeout for entities the server forgets to `Bye`. A 3rd example
   with lossy/UDP-like semantics (vs. the current reliable WebSocket) might need last-seen eviction.
6. **Binary protocol / id types.** Both call sites use JSON + `usize`. The helper is already generic
   over `K`; a binary-protocol example (e.g. `u16` ids) would confirm the `K` bound is enough.
7. **Disconnect/reset policy.** `clear(world)` exists but neither example calls it (they only show a
   status message and keep stale rivals). A 3rd example that cleanly tears down on disconnect would
   show whether `clear` is the right primitive or if an automatic on-`Disconnected` hook is wanted.

## Candidate 3rd examples (any one unlocks the deep-dive)

- **Client-prediction platformer/shooter** — local prediction + server reconciliation + remote
  interpolation. Stresses #1, #2, #3 hardest.
- **Many-entity / interest-managed world** — typed entities, spawn/despawn churn, culling by
  relevance. Stresses #4, #5.
- **Binary-protocol sync** — non-JSON, compact ids/fields. Stresses #6.

## Decision after the 3rd example (predict_shooter, 2026-06-09)

The 3rd distinct networked example — the **client-prediction shooter**
(`examples/games/predict_shooter/`) — is built and **real-play-verified** (two native clients
connect, see each other at consistent positions, and one client's input moves its predicted avatar
*and* the other client's interpolated view of it). It needs both interpolation (remote players +
bullets) and prediction (local player), so it finally exercises open questions #1 and #2.

**Finding:** interpolation is a *per-entity timestamped position buffer* (`client_net::Interp`) that
is **orthogonal** to the `id → Entity` lifecycle (`RemoteEntities`). The shooter keeps them as two
parallel maps — `remote_players: RemoteEntities<usize>` **and** `player_interp: HashMap<usize,
Interp>` — and they don't overlap: `RemoteEntities` owns spawn/despawn, `Interp` owns the position
history the renderer samples.

**Decision — keep `RemoteEntities` minimal; do NOT couple interpolation into it.**
- The two snap-only call sites (`mp_client`, `coin_race`) don't interpolate at all. Folding an
  interpolation buffer into `RemoteEntities` would force that concept (and cost) onto them, or
  require a generic value-buffer they'd never use. The shooter shows the **separation is correct**:
  lifecycle and interpolation are independent concerns that compose as parallel maps.
- **Do NOT promote `client_net::Interp` to a public engine helper yet** — only one example uses it.
  That's the same single-call-site discipline that (correctly) deferred the richer `RemoteEntities`.
  If a *second* interpolating example appears, `engine::Interp` / `SnapshotBuffer<T>` is a clean
  additive helper to extract then.
- **`Prediction` (local-player) is explicitly out of `RemoteEntities`' scope** — it's a *local*
  concern (input replay vs. server correction), not remote-entity bookkeeping. Stays example-local
  (a future `engine::Prediction` only if reused).

**Net:** the v4.3.0 minimal `RemoteEntities` API is the right shape and stays unchanged (additive
deferral was correct). Open questions #1/#2 are answered: interpolation and prediction are separate,
not-yet-promoted concerns. #3–#7 still await examples that stress them (per-entity update callbacks,
typed entities, staleness, binary protocol, disconnect policy).

## `SnapshotBuffer<T: Lerp>` promoted — 2nd interpolating example (orbital_dodger, 2026-06-09)

The trigger named above ("if a *second* interpolating example appears, `engine::Interp` /
`SnapshotBuffer<T>` is a clean additive helper to extract then") fired. The second interpolating
example is **`orbital_dodger`** (`examples/games/orbital_dodger/`): an **interpolation-only** game
(no prediction) where a broadcast server drifts spinning hazards at a low 10 Hz and the client
interpolates them; the local player is purely client-side and never round-trips. Crucially it
interpolates **two channels per hazard** — position (`Vec2`) *and* spin angle (`f32`) — so the
hardcoded-`(x, y)` `Interp` would not have fit.

**Decision — promote, generically.** `client_net::Interp` is now the public
**`engine::SnapshotBuffer<T: Lerp>`** (`src/network.rs`), reusing the engine's existing `Lerp` trait
(`src/timeline.rs`, impls for `f32` / `Vec2` / `[f32; 4]` / `Color`). Generic-over-`Lerp` is a
*strictly better* shape than the example's `(x, y)`: `orbital_dodger` uses `SnapshotBuffer<Vec2>` +
`SnapshotBuffer<f32>`, and `predict_shooter` migrated its `Interp` to `SnapshotBuffer<Vec2>`
(behavior-identical — `Vec2` lerp == per-component lerp). Two real call sites across two examples,
one of them needing a non-`Vec2` channel, is exactly the evidence that the generic shape is right and
not over-fit. Additive (`v4.4.0`).

**Open question #1 (interpolation) is now closed.** The buffer is per-entity, timestamped, and
**orthogonal** to `RemoteEntities` (lifecycle map) — confirmed across both examples, which keep them
as parallel maps. `RemoteEntities` stays minimal: interpolation was *split out* as a separate type,
not folded in, so the two snap-only call sites (`mp_client`, `coin_race`) pay nothing.

- **`Prediction` stays example-local** (`predict_shooter/client_net.rs`). Still one call site, and
  it is a *local* concern (input replay vs. server correction), not remote-entity state. A future
  `engine::Prediction` only if a second prediction example appears — same discipline.
- **#3–#7 still open** (per-entity update callbacks, typed entities, staleness/eviction, binary
  protocol, disconnect policy) — none of the two interpolating examples stress them yet.

## 5th example (salvage_run, AOI streaming) — 2026-06-10

The "3rd direction" candidate (a many-entity / interest-managed world) was built as **`salvage_run`**
(`examples/games/salvage_run/`): a ship roams a world far larger than the window while the server
simulates ~120 wandering entities of **two typed kinds** (salvage, drones) and streams each client
**only** the entities within an area-of-interest (AOI) radius of its last-reported position. As the
player roams, entities continuously stream in and out (churn). It reuses `SnapshotBuffer<Vec2>` (its
3rd call site) for smooth motion, evicts entities that leave the AOI by an example-local last-seen
timeout, and calls `RemoteEntities::clear` on disconnect. This is the first example to stress the AOI
/ staleness / typed-entity questions. **No engine API change** — purely additive (`v4.5.0`).

Findings on the open questions:

- **#3 (per-entity update / `upsert`)** — *still unmotivated.* Each entity's synced state is still a
  single `Vec2`, and the "update" path is a buffered `SnapshotBuffer::push` consumed later at the
  interpolated render time, **not** an immediate transform write. An `upsert(world, key, spawn,
  update)` wouldn't obviously help; the pressure would need an entity with several
  *immediately-applied* synced scalar fields (health/facing/anim-state). **Keep minimal.**
- **#4 (typed / multiple entity classes)** — *first real datapoint.* Two kinds are kept as **two
  `RemoteEntities<usize>` maps** (the "N maps" baseline). The seam it exposes: eviction/collision
  must probe `contains_key` across both maps to find an id's kind (an O(N-kinds) wart), because the
  id space is global. The clean candidate answer needs **zero engine change**: `RemoteEntities<(Kind,
  usize)>` already compiles today (`K: Eq + Hash`), so a future many-kind example could just use a
  tuple key. Two kinds isn't enough to design a typed-multimap helper → **flag the `(Kind, id)`
  pattern, don't build.**
- **#5 (staleness / eviction)** — *the example #5 was waiting for.* AOI churn produces
  **removal-by-omission**: the server never sends a removal, an entity simply stops appearing in
  snapshots when it leaves the AOI. The `Bye`-driven `remove` can't express that; the client must
  infer eviction from "not seen for T seconds." The `last_seen: HashMap<id, f64>` + timeout pattern
  is clean and example-local. **Candidate additive helper to flag (not build): a generic last-seen
  eviction tracker** (`touch(key, t)` / `expired(now - timeout) -> Vec<K>`), or an optional eviction
  policy on `RemoteEntities`. One call site → defer extraction to a 2nd staleness example, exactly
  as `SnapshotBuffer` was deferred until its 2nd.
- **#7 (disconnect / reset)** — *first example to exercise `clear` on disconnect; it works.* But the
  client must also clear the *parallel* maps the engine doesn't own (the two `SnapshotBuffer` maps +
  `last_seen`), so an automatic on-`Disconnected` engine hook would only do half the job — the same
  parallel-map reality that kept interpolation orthogonal. **Keep `clear` manual; no auto-hook.**

**Net:** v4.5.0 is purely an example. #5 yields the one genuinely new clean single-call-site pattern
(last-seen eviction) → next candidate helper, gated on a 2nd example. #4 gains the datapoint that
`RemoteEntities<(Kind, id)>` is already expressible (a zero-change answer). #3 and #7 reinforce
"keep `RemoteEntities` minimal." #6 (binary protocol) remains the one untouched direction.

## Pointers

- Implementation + doctest + unit tests: `src/network.rs` (`RemoteEntities`, `SnapshotBuffer`).
- 3rd example (interpolation + prediction): `examples/games/predict_shooter/client_net.rs`
  (`Prediction`) + `predict_shooter.rs` (now uses `engine::SnapshotBuffer<Vec2>`).
- 4th example (interpolation only, the `SnapshotBuffer` promotion trigger):
  `examples/games/orbital_dodger/` (`SnapshotBuffer<Vec2>` position + `SnapshotBuffer<f32>` spin).
- Plans: `plans/handoffs/PLAN_networking-dogfood_deferred-polish_2026-06-09.md` (Phase 2) and
  `…_client-prediction-shooter_2026-06-09.md` (the shooter, Phase D) — **both deleted 2026-09-04**
  with the rest of the handoff archive; `git show <sha>^:<path>` still has them.
- The "fix only the gap the example hits" + "playable example validates the feature" bar:
  `docs/VISION.md`.
