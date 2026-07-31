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

### Render-target-format-aware pipeline cache

A render pipeline bakes in its color-target `TextureFormat`, so a pipeline compiled for the
surface format **fails wgpu validation if used against a different target** — an offscreen
`RenderTarget` or the `Rgba16Float` HDR post-process intermediate. **Rule: a new render pass
must key its pipeline by the *target* format, not `gpu.config.format`.** Skipping this makes
the feature silently vanish under HDR post / offscreen RTs (exactly the #213 + #220
regressions — UI primitives, `ShaderMaterial`, and GPU particles each disappeared until their
pipeline was made format-aware).

Keep the surface-format fast path and lazily build + cache a matching pipeline per non-surface
format, paid once per distinct format (never per frame):

```rust
// 1. Store the surface pipeline + its format; an empty per-format cache.
base_format: wgpu::TextureFormat,
extra_pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,

// 2. ensure_*: no-op for base format or a cache hit; else compile + insert.
fn ensure_pipeline(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
    if format == self.base_format || self.extra_pipelines.contains_key(&format) { return; }
    self.extra_pipelines.insert(format, build_pipeline(device, /* … */, format));
}

// 3. *_for: base pipeline for the surface format, else the cached extra
//    (fall back to base if somehow missing — never panic mid-frame).
fn pipeline_for(&self, format: wgpu::TextureFormat) -> &wgpu::RenderPipeline {
    if format == self.base_format { &self.pipeline }
    else { self.extra_pipelines.get(&format).unwrap_or(&self.pipeline) }
}
```

The renderer carries the target format in `FrameContext.format`; the offscreen pass threads the
`RenderTarget`'s `format()`. Four instances, all this shape (later ones say "Mirrors the sprite
cache"):

| Renderer | Cache field | Build / select | File |
|---|---|---|---|
| Sprite | `extra_sprite_pipelines` | `ensure_sprite_pipeline` / `sprite_pipeline_for` | `src/renderer/sprite.rs` |
| UI primitive | `extra_ui_pipelines` | (same module) | `src/renderer/sprite.rs` |
| `ShaderMaterial` | `custom_pipelines` keyed by `(hash, format)` | `MaterialRenderer::compile_pipeline` | `src/renderer/sprite/material.rs` |
| GPU particle | `extra_render_pipelines` | `ensure_render_pipeline` / `render_pipeline_for` | `src/renderer/gpu_particle.rs` |

The keys differ slightly (`ShaderMaterial` keys by `(source-hash, format)` since the frag shader
also varies), so the shape is **duplicated deliberately rather than abstracted** — revisit a
shared helper only if a fifth pipeline needs it.

### Mid-frame GPU upload rules (upload-once + range-draw, renderer pooling)

`queue.write_buffer` executes at **submit time**, not call time — writing the same buffer
twice within one frame means the second write lands before *any* pass executes, so the
earlier pass silently renders the later data. Two sanctioned shapes, both from the text
z-interleave (v0.110.0, #326):

- **Upload once + draw by range** — upload ALL of a frame's instance data in one
  `prepare_*` call, then issue sub-draws against byte-offset ranges
  (`SpriteRenderer::prepare_ui_primitives` → `render_ui_primitive_range`,
  `src/renderer/sprite/ui_primitives.rs`). Never re-upload the buffer between passes.
- **Per-frame renderer pooling** — when a third-party renderer owns its vertex buffers per
  `prepare` call (glyphon's `TextRenderer`: one prepared batch per renderer instance), keep
  a pool indexed by a `used` counter: take a fresh renderer per batch, reset the counter
  once per frame in `end_frame()` (`FormatPool` in `src/renderer/text/renderer.rs`; pools
  are per-target-format — the text analogue of the pipeline cache above).

The pure interleaving algorithm is `src/renderer/text/layering.rs::interleave_runs`
(two pre-sorted z lists → alternating surface/text run counts; tie = text after surface;
7 unit tests) — reuse it for any future z-interleave of two draw streams.

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

### Shared policy for cfg-split backends (drift prevention)

When the native and wasm backends each compute the **same derived value**, put the *formula* in
an **un-gated module both call**. The implementations may diverge — device access, node graphs,
threading — but the **policy must be single**. If it is duplicated, both sides compile and the two
platforms silently behave differently, which no test catches unless someone thought to write a
cross-platform one.

Three instances, all in audio:

| Module | Shared |
|---|---|
| `src/audio_spatial.rs` | `spatial_params` — distance falloff + stereo pan |
| `src/audio_analysis.rs` | `smooth_toward` — the meter's attack/release policy |
| `src/audio_analysis.rs` | `log_band_range`, `MIN_DB`/`MAX_DB` — spectrum band edges + dB window |

`audio_spatial` states the reason in its own header: *"a single canonical implementation so the
two builds can't drift."*

**The sharper move, when one platform already owns the constant:** `MIN_DB`/`MAX_DB` are not
tuned values — they are **Web Audio's `AnalyserNode` defaults** (−100/−30 dB), and the wasm
backend **sets them explicitly on the node** rather than relying on the default. Matching the
other platform's constant makes the two comparable; re-setting it explicitly means a browser
changing its default cannot silently desync them.

**Consequence for the `Audio` facade:** if both backends name a method identically with the same
signature, the facade forwards with a bare `self.inner.x(...)` and needs **no `cfg` at all** —
v0.136.0 added seven facade methods with zero `cfg` lines and *deleted* an existing pair. Agree on
the name before writing either implementation; a naming mismatch breeds `cfg` in the facade.

**Footnote — reaching for a new `web_sys` type costs a `Cargo.toml` edit.** `web-sys` gates every
DOM type behind its own feature, so a first use of one fails with `E0425: cannot find type`
until it is added to the `web-sys` `features` list. Easy to forget because it comes up so
rarely — `git log -S'web-sys' -- Cargo.toml` shows only **two** such edits in the project's
history (the latest added `"AnalyserNode"` for the spectrum). The wasm **lib** build in the gate
does catch it, so this costs a cycle rather than shipping a bug.

### Real-time (audio-thread) producers

Code inside a rodio `Source::next()` runs on the **playback thread**, not the game thread.

1. **No locks, no allocation.** Allocate scratch buffers in the constructor (game thread) and
   publish results through atomics — e.g. `f32::to_bits()` into an `AtomicU32`. A mutex in an
   audio callback is a dropout.
2. **A producer that can stop needs a liveness signal.** Otherwise the consumer cannot tell
   "value unchanged" from "producer dead". Concretely: a level meter whose source ended **freezes
   at its last value**, which reads on screen as a stuck bar rather than as silence. The fix is a
   monotonic sequence counter — unchanged since the previous tick means *not producing*, so decay
   toward zero (`LevelSlot::seq` / `tick_analysis`).
3. **This is a native-only concern, and say so.** A Web Audio `AnalyserNode` reads the live graph,
   so silence decays on its own and needs no counter. Record the asymmetry, or the next person
   either adds a redundant counter on wasm or deletes the necessary one on native.

### Surviving a scene reset (resource persistence)

`App::set_scene` / `SceneCmd::Replace` **rebuild the `World` from scratch** (`App::reload_scene`).
Every resource is dropped and re-created from engine defaults unless its type was registered with
`App::register_persistent::<T>()`. Component data on entities goes too — that is the point of a
scene swap — but *resources* are where this surprises people, because a resource is usually
**set-up state a game inserts once before `run()` and never thinks about again**.

**The failure mode is silence, not a crash.** A dropped resource is replaced by the engine default
(or simply absent), so the game keeps running with the wrong value. `WindowConfig` did this for a
long time: every `Scene`-based game lost its `clear_color` and every headless capture came out at
the default 1280×720 (fixed in v0.137.1 — see the CHANGELOG).

**Registered automatically by the engine** (a game never does these):

| Type | Why |
|---|---|
| `WindowConfig` | set-up config, inserted once before `run()` (v0.137.1) |
| `SceneTransition` | must outlive the *mid-transition* swap or the reveal half never renders |
| `TextMeasurer` | lazily built after a one-time system-font scan; re-paying it per scene is waste |
| `InputScript` | a scripted run must not be cancelled by a scene change |
| the 7 RON registries | `DataTable` / `AnimationClip` / `ParticleConfig` / `Dialogue` / `TriggerZone` / `ZoneEffect` / `AnimEffect` |
| `FocusRingStyle` · `StickNavConfig` · `FrameConfig` | engine-inserted config a game overrides once; the audit below (v0.140.0) |
| `DesignResolution` · `WindowOptions` · `LightingConfig` · `DialogueStyle` | engine-*defined* config the **game** inserts; same audit |

(`DebugUi` is also carried across, by hand inside `reload_scene` rather than through the registry.)

#### The audit — done once, so it does not need doing again (v0.140.0)

`WindowConfig` was found by accident, after the engine had routed around it twice. All 27 resources
`insert_core_resources` inserts were then classified against the session-vs-scene test below, plus
the engine-defined config types a game inserts itself. **The result is in the table above: seven
more were being silently reverted, all with the same one-line fix.** Each has a regression test in
`src/app.rs` that was confirmed to fail before the fix.

The other 20 are correctly scene state, and this is the negative result worth keeping so nobody
re-runs the audit:

| Group | Members | Why resetting is right |
|---|---|---|
| Device / derived state | `InputState`, `GamepadState`, `TouchState`, `RealDt`, `ViewportSize`, `PendingResize` | Rewritten from a live source every frame; self-heals on the next tick |
| Per-frame draw queues | `TextQueue`, `UiQueue`, `UiImageQueue`, `DebugDraw` | Cleared and refilled every frame |
| Scene-scoped state | `GameState`, `Camera`, `UiFocus`, `SelectedEntity`, `ProfilerData`, `SceneChange`, `LoadProgress`, `ShouldQuit` | Per-scene by definition; several hold `Entity` ids from the destroyed world |
| Rebuilt by another mechanism | `AssetServer`, `ScriptRegistry`, `SerdeComponentRegistry`, `PanickedSystems` | Path-keyed caches (a reset costs a re-load, not wrongness), or re-registered by `reload_scene` itself |

**`TimeScale` is the one deliberate exclusion.** It is engine-inserted config-shaped state, but it
is a *live gameplay effect* (hit-stop, slow-mo) that games drive moment-to-moment — a frozen or
slowed **old** scene leaking into the next one is the worse bug. Resetting it to `1.0` per scene is
correct, not an oversight.

The line the audit settled on: **auto-persist a type the engine defines and only ever reads**
(no production path mutates one), whoever inserts it. That is what lets the four game-inserted
types above be covered — `register_persistent` on an absent resource is free, because
`reload_scene` skips types it does not find.

**Everything a game inserts itself is the game's responsibility**, and the one that bites is
**`Audio`**: lose it and the device handle goes with it, so music stops and any audio-driven logic
stops with it — silently. Register it at setup:

```rust
app.register_persistent::<Audio>();   // precedent: examples/games/settings_menu
```

`examples/games/beat_crawler` needs this for a sharper reason than most: its **turn clock** is
`Audio::bands()`, so dropping the resource would not merely mute the game — it would stop the
world from taking turns at all.

> **Known rough edge, still not fixed — but its reopen trigger has now fired (2026-07-31).**
> "Forget one line and audio silently dies" is the same class of footgun `WindowConfig` was. It was
> left game-side because `Audio` is inserted *by the game*, not the engine, and auto-persisting
> everything a game happens to insert would empty the mechanism of meaning.
>
> One of the listed triggers — *"another engine-inserted config type turns out to be dropped the
> way `WindowConfig` was"* — fired in the v0.140.0 audit above: **seven** did. And the fix drew a
> new line that weakens the original argument, because four of the seven are inserted by the
> **game**, not the engine. Who inserts it turned out not to be the distinction that matters; the
> distinction is whether the **engine defines the type and only reads it**.
>
> `Audio` sits just outside that line: the engine defines it, but `AudioFacadeSystem` *drives* it
> every frame rather than reading it as config, and it owns an OS device handle rather than a
> value. That is a real difference, not a dodge — but it is thinner than the original "the game
> inserted it" argument, and it is now the only thing holding the decision open.
>
> **This is a decision to take deliberately, not to let drift.** Left unchanged here on purpose:
> the audit that fired the trigger is its own change, and folding an `Audio` behaviour change into
> it would bury the question. The fix, if it comes, is still one line in `App::new` plus a
> regression test — the same shape as v0.137.1.

**When adding a resource, ask which kind it is:** *scene state* (dies with the scene — correct) or
*session state* (config, device handles, caches, cross-scene progress — must be registered). If it
is session state, register it in the same commit that introduces it.

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

1. Define the struct in the appropriate file under `src/resources/` (e.g.
   `display.rs`, `time.rs`, `render.rs`), then re-export it from `src/resources/mod.rs`
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

**Engine-emitted events need the same opt-in.** Widget passes emit `UiEvent`,
`AnimationSystem` emits `AnimationEvent`, `TriggerZoneSystem` emits `ZoneEvent` — an
example/game that reads any of these must still call `app.register_event::<E>()` at
startup. An unregistered bus silently drops (or one-time-warns) every send, which
presents as "the event never arrives" (bit `examples/ui_dropdown` in #324 — the HUD
counter never moved; the engine was fine).

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
