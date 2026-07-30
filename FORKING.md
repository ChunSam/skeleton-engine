# Forking skeleton-engine

A short, practical guide to building **your own game** on the engine. For the full API
reference see [`REFERENCE.html`](REFERENCE.html) _(Korean)_; for the architecture and
agent notes see [`CLAUDE.md`](CLAUDE.md) and [`docs/`](docs/).

## The model: fork, don't depend

This engine is **unpublished by design**. It is a *skeleton* — you clone the source, edit
engine code directly under `src/`, and grow it into your own engine. There is no
`cargo add skeleton-engine`; you work inside the repo (or your fork of it).

- **Package name:** `skeleton-engine` (the Cargo package).
- **Library crate name:** `engine` — so all code does `use engine::*;` or
  `use engine::{App, World, Sprite, ...};`.
- **Version line:** pre-1.0 (`0.x`). The public API may change between releases.

## Repo layout

| Path | What |
|------|------|
| `src/` | the engine itself — edit it freely; this is the point |
| `src/lib.rs` | the public API re-export list (the fastest map of what exists) |
| `docs/MODULE_MAP.md` | module map: "where do I find X?" table for every subsystem (grep it) |
| `CLAUDE.md` | agent quick reference: conventions, the verify gate, task checklists |
| `examples/*.rs` | small single-file examples (auto-discovered by cargo) |
| `examples/games/<name>/` | full example games (registered via `[[example]]` in `Cargo.toml`) |
| `examples/assets/` | shared example assets (PNGs, etc.) |
| `docs/` | VISION, PATTERNS, HANDOFF, CHANGELOG (English) |
| `scripts/verify.sh` | the local CI-equivalent gate — run before you commit |

## Start your own game

The fastest path — copy the smallest example and edit it:

```sh
cargo run --example hello_sprite          # see it work first
cp examples/hello_sprite.rs examples/my_game.rs
cargo run --example my_game               # any examples/*.rs is auto-discovered
```

Two other layouts, when you outgrow a single file:

- **Multi-file game** → make `examples/games/my_game/` and register it in `Cargo.toml`:
  ```toml
  [[example]]
  name = "my_game"
  path = "examples/games/my_game/my_game.rs"
  ```
- **A binary** → put it in `src/bin/my_game.rs` and `cargo run --bin my_game`.

## The shape of a game

Four concepts cover almost everything (see `examples/hello_sprite.rs` and `examples/basic.rs`):

- **`App`** — owns the window, the `World`, and the main loop. `App::new()` → configure →
  `app.add_system(...)` → `app.run()`.
- **`World`** — the ECS container. `spawn()` an `Entity`, `add_component(entity, ...)` it,
  and store global state as a **resource** (`insert_resource` / `resource` / `resource_mut`).
- **Components** — plain data on entities (`Transform`, `Sprite`, your own structs).
- **Systems** — `impl System for X { fn run(&mut self, world: &mut World, dt: f32) }`. Added
  with `app.add_system(...)` and run every frame.

```rust
use engine::*;

struct Mover;
impl System for Mover {
    fn run(&mut self, world: &mut World, dt: f32) {
        // ... read input, move entities ...
    }
}
```

## Assets

- Image paths passed to `app.load_image(path)` are **relative to the repository (cargo
  workspace) root**, not to the source file. e.g. `"examples/assets/player.png"`.
- `load_image` registers the texture and returns a `Handle` immediately; the GPU upload
  happens when the app starts, so call it before `app.run()`.
- Draw it: `Sprite::textured_with_handle(path, Some(handle))` (prefers the handle, keeps
  the path as a fallback). For a solid color use `Sprite::colored(r, g, b)`.

## A pattern you will hit early: the borrow split

You cannot hold an immutable borrow of the `World` (a query) while mutating it. The
established idiom is **collect the entities first, then mutate**:

```rust
let entities: Vec<Entity> = world.query::<Player>().map(|(e, _)| e).collect();
for e in entities {
    if let Some(t) = world.get_mut::<Transform>(e) {
        t.position += velocity;
    }
}
```

Better still, when you're updating components in place use the **mutable queries** and skip the
collect entirely: `world.query_mut::<T>()` for one component, `world.query2_mut::<A, B>()` /
`query3_mut` for two or three at once — each yields `&mut` references directly:

```rust
for (_e, transform, vel) in world.query2_mut::<Transform, Velocity>() {
    transform.position += vel.0;
}
```

For a single known entity, store the `Entity` and skip the query entirely (see
`hello_sprite.rs`). See [`docs/PATTERNS.md`](docs/PATTERNS.md) for the query API
(`query2` / `query_opt2`) and more recipes.

## Before you commit

Run the local gate — it mirrors CI (fmt, clippy, wasm build, tests, doc links):

```sh
./scripts/verify.sh
```

Run it as-is; a non-zero exit means something is broken. After WASM-affecting changes,
`./scripts/wasm_smoke.sh` additionally renders the `coin_race` example headless.

## Where to read more

- **`src/lib.rs`** — the public surface, at a glance.
- **`docs/MODULE_MAP.md`** — the "where is X?" module map (72 rows; grep it rather than reading it whole).
- **`CLAUDE.md`** — project conventions and the verify gate.
- **`docs/VISION.md`** — why the engine exists and how features get accepted (every feature
  is validated by a small playable example).
- **`docs/PATTERNS.md`** — architecture patterns and task recipes.
- **`REFERENCE.html`** — full API reference with examples _(Korean)_.
