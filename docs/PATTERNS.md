# Core architecture & task patterns

Detailed engine patterns and task recipes, extracted from `CLAUDE.md` / `AGENTS.md` to
keep those quick-reference files under 200 lines. Both reference this document.

---

## Core architecture patterns

### ECS query API

```rust
// Single component
for (entity, comp) in world.query::<MyComp>() { ... }

// Multiple components (query2 / query3 / query4)
for (e, a, b) in world.query2::<A, B>() { ... }

// A required, B optional
for (e, a, b_opt) in world.query_opt2::<A, B>() { ... }

// System signature
impl System for MySystem {
    fn run(&mut self, world: &mut World, dt: f32) { ... }
}
```

### Borrow checker workaround pattern (required)

You cannot call `get_mut` on the same World while a query iterator is alive. Standard pattern:

```rust
// First collect the entity list, then iterate and get_mut
let entities: Vec<Entity> = world.query::<Foo>().map(|(e, _)| e).collect();
for entity in entities {
    world.get_mut::<Foo>(entity).unwrap().update();
}
```

### Per-frame scratch buffers (allocation convention)

Collect-then-mutate forces temporary collections; in a **per-frame** system, do not
allocate them fresh every call. Two sanctioned patterns:

- **Scratch fields** — promote the temporaries to private struct fields and
  `clear()` + refill each frame (`CollisionGridSystem`, `PhysicsSystem`).
- **`std::mem::take`** — when a queue/Vec resource is drained each frame, take it
  instead of cloning, and put it back if it must survive
  (`TextQueue` drain in `renderer/text.rs`, `exec_order` in `app/schedule.rs`).

One-shot or editor-only paths may allocate freely — this convention is for code that
runs every frame.

### Time-driven System animation (no global clock)

The engine has **no global elapsed-time resource** — a `System` receives only per-frame
`dt`. When one needs a continuous time signal (a caret blink, a "breathing" pulse, any
periodic animation), accumulate the clock locally rather than reaching for a shared one:

1. **Accumulate in a System struct field** — `self.elapsed += dt` each frame.
2. **Wrap it to bound `f32` precision** — `self.elapsed = (self.elapsed + dt).rem_euclid(period)`
   (or `-= period` once past it), so the accumulator never grows large enough to degrade the
   `sin`/`fract` math downstream.
3. **Thread the elapsed time into a *pure* sub-pass helper** that maps `time → output`, so the
   timing is unit-testable without a live frame loop, and keep the no-effect default a flat
   passthrough so a disabled animation stays byte-identical.

Instances: `TextInput::cursor_blink` (caret blink, `src/ui/system/text_input_pass.rs`) and
`UiSystem::ring_elapsed` → `FocusRingStyle::pulse_alpha(t)` (focus-ring pulse,
`src/ui/system/focus_pass.rs`). Both return the unmodulated value when the effect is off.

### Render layer separation

- `AnimationSystem` → syncs the `UvRect` component → the renderer reads only `UvRect`  
  (the renderer referencing `AnimationPlayer` directly is a layer violation)
- `DebugDraw` = pure data (`DebugShape` / filled rects) → converted to `DrawRect` in the `App` render stage
- Render order: Systems → Events flush → Input flush → Scene command handling → Render (sprites → UI → text)

### UI system registration order

When using `Panel`, register `LayoutSystem` **before** `UiSystem`:

```rust
app.add_system(LayoutSystem);  // recomputes child UiNode.offset
app.add_system(UiSystem);      // reads positions and renders
```

`UiEvent` implements `Clone` but not `Copy` (TextChanged/TextSubmitted carry a String).  
`InputState::text_chars()` — this frame's char slice. `'\x08'`=Backspace, `'\n'`=Enter.

### Animation state machine registration order

Register `StateMachineSystem` **after** `AnimationSystem` so `is_finished()` is reflected in the same frame:

```rust
app.add_system(AnimationSystem);     // frame advance + UvRect sync
app.add_system(StateMachineSystem);  // evaluate transition conditions → call play()
```

Manipulate parameters inside a system via `world.get_mut::<AnimationStateMachine>(entity)`:

```rust
sm.set_bool("is_running", true);   // for BoolEq conditions
sm.set_float("speed", 3.5);        // for FloatGt / FloatLt conditions
sm.fire_trigger("jump");           // for Trigger conditions (auto-consumed each frame)
```

`TransitionCond::AnimationEnd` becomes true when a non-looping clip reaches its last frame.

### System ordering with labels

Insertion order works, but it is implicit — reordering registrations silently breaks the
constraints above. Every built-in system exposes a `LABEL` constant; declare ordering
explicitly with `add_system_labeled` (demonstrated in `examples/games/platformer/`):

```rust
app.add_system_labeled(AnimationSystem, SystemConfig::new().label(AnimationSystem::LABEL));
app.add_system_labeled(
    StateMachineSystem,
    SystemConfig::new().label(StateMachineSystem::LABEL).after(AnimationSystem::LABEL),
);
```

Known ordering constraints expressed this way:

| Constraint | Why |
|---|---|
| `StateMachineSystem` after `AnimationSystem` | reads frame state produced by the tick |
| `BlendTreeSystem` before `AnimationSystem` | clip transitions take effect in the same frame |
| `LayoutSystem` before `UiSystem` | UiSystem reads recomputed offsets |
| readers of `SpatialGrid` after `CollisionGridSystem` | it mirrors the grid resource |
| `CollisionDebugSystem` after `CollisionGridSystem` | same |
| consumers of `Events<NetworkEvent>` after `NetworkSystem` | it polls the socket into the bus |
| `LocalizationSystem` before `UiSystem` | resolved text rendered same frame |
| readers of `GlobalTransform` after `HierarchySystem` | propagation runs last in the frame; `.after(HierarchySystem::LABEL)` guarantees current-frame world transforms |

Scenes order systems the same way (since v5): `Scene::on_enter` receives a
`SystemRegistrar` whose `add_labeled` takes the same `SystemConfig` builder
(demonstrated in `examples/games/settings_menu/` — `UiSystem` after `LayoutSystem`):

```rust
fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
    systems.add(LayoutSystem);
    systems.add_labeled(UiSystem, SystemConfig::new().after(LayoutSystem::LABEL));
}
```

### PhysicsWorld encapsulation

Internal rapier2d fields are `pub(crate)`. Do not access them directly from outside. Available accessors:

```
rigid_body() / rigid_body_mut()
get_collider() / get_collider_mut()
add_dynamic_circle() / add_dynamic_box() / add_static_box()
remove_body()
```

---

## Common task patterns

### Add a new component

1. Define the struct in `src/components.rs` or the relevant module file
2. Add a re-export in `src/lib.rs`

### Add a new system

1. Implement the `System` trait
2. Register with `app.add_system(MySystem)` (or `add_system_labeled` for explicit
   ordering), or via the `SystemRegistrar` in `Scene::on_enter` (`systems.add(MySystem)`
   / `systems.add_labeled(...)`)

### Add a new resource

1. Define the struct in `src/resources.rs`
2. Register with `app.world.insert_resource(MyResource { ... })`
3. Add a re-export in `src/lib.rs` if needed

### Make a hardcoded constant configurable (default-preserving resource)

To let a game or fork override hardcoded constants (a color, a threshold, a layout
dimension) **without changing the default behavior for anyone who doesn't opt in**, extract
them into a small `Copy` World resource whose `Default` reproduces the old constants
byte-for-byte (pattern from `FocusRingStyle`, `src/ui/focus.rs`, v0.42.0):

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusRingStyle {
    pub color: Color,
    pub thickness: f32,
    pub enabled: bool,
}

impl Default for FocusRingStyle {
    fn default() -> Self {
        // Reproduce the OLD hardcoded constants EXACTLY.
        Self { color: Color::rgba(1.0, 0.85, 0.3, 1.0), thickness: 3.0, enabled: true }
    }
}
```

1. **Auto-insert** the default in `insert_core_resources` (`src/app/core_resources.rs`),
   next to the related resource, so games override it via `world.resource_mut::<T>()`.
2. **Read it with `unwrap_or_default()`** at the (per-frame) call site, so a `World` that
   never inserts it — including hand-built unit-test worlds — stays byte-identical to the
   old constant:
   ```rust
   let style = world.resource::<FocusRingStyle>().copied().unwrap_or_default();
   ```
   `Copy` + `.copied()` also sidesteps holding a `&World` borrow across the call that uses it.
3. **Re-export** the type from its module and `src/lib.rs` (it is public API now).
4. **Demonstrate the override in the example** (the VISION acceptance test) and add a unit
   test asserting `Default` matches the historical value, so a future edit can't silently
   change the out-of-the-box behavior.

Additive — no public API removed; ship as a MINOR under the 0.x cadence.

### Add a new event

```rust
// 1. Define the type (needs Clone + 'static)
#[derive(Clone)]
struct MyEvent { pub data: f32 }

// 2. Register during App setup
app.register_event::<MyEvent>();

// 3. Use inside a system
world.resource_mut::<Events<MyEvent>>().unwrap().send(MyEvent { data: 1.0 });
for ev in world.resource::<Events<MyEvent>>().unwrap().read() { ... }
```

### Scene transitions

```rust
world
    .resource_mut::<SceneChange>()
    .unwrap()
    .request(SceneCmd::Replace(Box::new(MyScene)));
// SceneCmd::Push(Box::new(MyScene)) — push onto the stack
// SceneCmd::Pop                      — return to the previous scene
```

### Add a custom asset type (fork extension point)

`AssetServer` is deliberately closed to generic registration — **forking it is the
intended extension path**. Each asset type is a pair of maps plus load/get methods;
mirror the existing `scripts`/`atlases` pair in `src/asset.rs`:

1. Storage: `things: HashMap<AssetId, ThingAsset>` + `thing_path_to_id: HashMap<Arc<str>, AssetId>`
2. `load_thing(path) -> Handle<ThingAsset>` — key via `asset_key`, dedupe through the
   path map, insert into storage
3. `get_thing(&Handle<ThingAsset>) -> Option<&ThingAsset>`
4. Optional hot reload: handle your extension in the `reload_rx` drain (native only)
