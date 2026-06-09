# RemoteEntities — design notes (for a future deep-dive)

> Status: **minimal helper shipped** (`engine::RemoteEntities<K>`, `src/network.rs`, v4.2.x).
> A *richer* version is **deliberately deferred** — this doc captures the context so the design
> can be reconsidered later without re-deriving it. Chain: `networking-dogfood` (seq 3, Phase 2).

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

## Pointers

- Implementation + doctest + unit tests: `src/network.rs` (`RemoteEntities`).
- Plan: `plans/handoffs/PLAN_networking-dogfood_deferred-polish_2026-06-09.md` (Phase 2).
- The "fix only the gap the example hits" + "playable example validates the feature" bar:
  `docs/VISION.md`.
