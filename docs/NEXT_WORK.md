# Next Work — candidates and alignment

> Status: living document. Derived from `docs/VISION.md` (reset 2026-05-29).
> This lists what to build next and why, under the vision's core loop:
> **a feature is not done until a small, playable example game in `examples/`
> exercises it in real play.**

## Context

`examples/` now separates top-level feature demos from playable example games under
`examples/games/`. The first playable examples are the platformer
(`cargo run --example platformer_game`), scene-flow game
(`cargo run --example scene_flow_game`), and maze-escape
(`cargo run --example maze_escape_game`), which start closing the previous validation gap.
The active direction remains: widen the feature set breadth-first, and prove each feature
with a small playable example.

## Candidate feature × playable-example pairs

Each candidate pairs an example game with the engine capability it validates/extends and
the API gaps it is likely to surface.

| # | Example game | Engine capability validated/extended | Likely gaps to surface |
|---|--------------|----------------------------------------|------------------------|
| **A** | Platformer (jump, run, platforms) ✅ done | `CharacterController`, `move_character`, physics platforms/sensors, `AnimationStateMachine`, atlas animation, camera follow | surfaced + **fixed (v1.1.0)**: one-way platforms (`set_one_way` + `CharacterController::request_drop`, drop-through in `move_character`); tilemap↔physics binding (`PhysicsWorld::add_static_from_tilemap` + `TileCollider`). Platformer level is now a single `Tilemap`. |
| **B** | Top-down maze escape (chasing enemies) ✅ done | `PathGrid`/`find_path`, `BehaviorTree`, `SpatialGrid` collision (`examples/games/maze_escape/maze_escape.rs`) | surfaced + fixed: `BehaviorTree`/`Sequence`/`Selector`/`Inverter`/`AlwaysSucceed`/`BehaviorSystem` were not re-exported from `engine::`; `SpatialGrid` was trapped inside `CollisionGridSystem` (now mirrored to a `World` resource each frame); no `PathGrid::from_tilemap` (added). **Fixed (v1.1.0)**: added `BlackboardValue::Path(Vec<IVec2>)` + `Blackboard::set_path`/`get_path`; `ComputePathToPlayer` now caches the whole path and recomputes only when the player's goal tile changes. |
| **C** | Sokoban (box pushing) ✅ done | discrete grid logic, multi-level progression, undo/redo, `save`/`load` progress (`examples/games/sokoban/sokoban.rs`) | surfaced + fixed: no reusable game-facing undo — only the editor had a private command history; added genre-agnostic `History<T>` snapshot undo/redo (`src/history.rs`, re-exported from `engine::`). `save`/`load_or_default` reused unchanged (no friction). Immediate-mode `DebugDrawQueue` filled rects render board state without ECS entity churn. |
| **D** | Simple shooter (bullets, waves) | `ParticleEmitter`, `Timer`, collision layers, audio buses | pooling/spawn bursts, perf; complements rust-survivors |
| **E** | Scene-flow game (menu → play → result) ✅ done | `SceneCmd` Push/Replace/Pop, UI buttons, `GameState`, scene-owned systems, explicit entity cleanup | surfaced + **fixed (v1.1.0)**: `App::register_persistent::<T>()` preserves resources across the `Replace` World reset; `scene_flow_game` dropped its `Arc<Mutex<_>>` workaround. |
| **F** | Skeletal-animation showcase character ✅ done | NEW: 2D cutout skeletal animation (`src/skeletal.rs`, `examples/skeletal_puppet.rs`) | surfaced + fixed `HierarchySystem` depth-3 cap; scale-vs-attachment-size rule noted in `docs/SKELETAL.md` |

## Done

- **A — Platformer** (`platformer_game`): tile collision, gravity, jump, moving platforms.
- **B — Maze escape** (`maze_escape_game`): PathGrid + BehaviorTree + SpatialGrid.
- **C — Sokoban** (`sokoban_game`): discrete grid, undo/redo (`History<T>`), multi-level + save.
- **D — Simple shooter** (`shooter_game`): pooled bullets (`Pool`), `Timer` fire/wave cadence,
  `SpatialGrid`/`CollisionLayer` hit detection, score/lives + restart, explosion particles, audio buses.
  - **Engine gap closed:** `ParticleEmitter` was continuous-only → added the additive one-shot
    `ParticleBurst` component (+ `ParticleEmitter::for_burst()`); `ParticleSystem` drains it and
    retires the emitter. Re-exported as `engine::ParticleBurst`. Unit tests in `src/particle.rs`.
  - **Surfaced-but-not-a-gap:** `Pool` worked for bullet churn via `remove_resource`/reinsert per
    system; released bullets strip `Sprite`/`Collider`/`CollisionLayer` so they leave the renderer
    and grid. No new pooling API was needed.
- **E — Scene-flow / UI interaction** (`scene_flow_game`): menus, pause, transitions.
- **F — Top-down twin-stick survival** (`survivor_game`): WASD move + Arrow-key 8-way aimed
  fire, `engine::Seek`/`SteeringSystem` seeker enemies, pooled bullets, `SpatialGrid` hits,
  `GpuParticleEmitter` player thruster, `ProfilerData` perf HUD. Debug `G`/`B` keys to reach
  the perf scale on demand.
  - **Engine gap closed:** `SteeringSystem` was **O(N²)** — each entity self-looked-up via
    `query().find()` full scans. The example surfaced it with data (steer **3.67ms @ 200**
    seekers, ~30× expected, quadratic). Fixed to O(1) `world.get`/`get_mut(entity)` access
    (behavior-identical, additive) + a regression test in `src/steering.rs`. After: **0.48ms
    @200, 1.36ms @600** (linear), ~60fps held at 600 (3× the design target). `rust-survivors`
    rebuilt clean (no breakage).
  - **Surfaced-but-not-a-gap:** `GpuParticleEmitter` is native-only (`cfg(not(wasm32))`, like
    `AudioManager`) — the thruster is target-gated, wasm renders nothing. `frame_ms` is
    vsync-capped (`AutoVsync`) so per-system `ProfilerData.systems` (the HUD `steer` readout)
    is the real CPU-cost signal.

## Breadth + depth pass complete

A–E (breadth) and **F** (depth: steering + many entities + GPU particles) have all shipped
under the dogfooding loop. No remaining planned candidate — the playable-examples program is
done for v1.0.0.

## Coverage follow-up — UI / i18n / audio cluster (v1.2.0)

A subsystem-coverage audit found ~10 shipped subsystems with no *playable-game* coverage (only
standalone demos or none). The densest, most universally-needed cluster — UI depth + localization
+ audio buses — is now closed by a new playable example:

- **G — Settings + Dialogue** (`settings_menu_game`, `examples/games/settings_menu/`): Title →
  Settings → Dialogue. First playable-game use of `TextInput`, `Slider`, `CheckBox`, `ScrollView`,
  `Panel`/`LayoutSystem`, rich/multiline `Label`, `LocaleResource` (EN/KO/ES), and `AudioManager`
  buses + `AudioEffect` low-pass. `Settings`/locale/`AudioManager` persist across `SceneCmd::Replace`.
  - **Engine gap closed:** `LocaleResource` had no reactive binding, so switching locale meant
    manually re-resolving every widget's text. Added the additive `LocalizedText` component +
    `LocalizationSystem` (targets `Label`/`Button`/`CheckBox`); the example switches language with
    one `set_locale` call and the whole UI retranslates. Unit tests in `src/ui/localized.rs`.
  - **Documented gaps (not fixed):** runtime per-locale font switching is unsupported
    (`TextRenderer` font is fixed at init; `LocaleData.font` is dead) → non-Latin relies on native
    system-font fallback, absent on Linux CI / wasm; `LocaleData.direction` (RTL) is metadata only
    and not wired into alignment. Both await a future dedicated example.
  - **Deferred-item follow-up:** the macOS input-latency item is now *largely* addressed — the
    live window-drag freeze is gone (frame step factored into `App::step_frame`, driven inline
    from `Resized`, plus `pre_present_notify`); content keeps animating through both resize and
    titlebar-move drags. Residual: a one-frame lag at the *start* of a drag (documented limitation,
    CHANGELOG → 1.2.1). `TextInput` **horizontal scroll** + IME-at-`max_len` honesty are now also
    done (single-line scroll via `DrawText::with_single_line_caret`; CHANGELOG → 1.3.0). Still
    deferred from this cluster: overlay caret, real OS fullscreen.

## Coverage follow-up — lighting + post-process cluster (candidate H, 2026-06-05)

- **H — Lit dungeon** (`lit_dungeon_game`, `examples/games/lit_dungeon/`): first playable-game
  use of 2D lighting (`PointLight`/`AmbientLight`) and `PostProcessConfig`. Dark top-down
  dungeon — the player carries a torch (`PointLight`) whose radius/intensity **decay over time**
  (fuel); lighting one of 16 scattered braziers spawns a persistent `PointLight` and refills the
  torch. Light all braziers to open the exit, then reach it. Bloom = torch/brazier glow,
  vignette = tunnel-vision; `P` toggles post-process. Camera follows the player across a level
  larger than the viewport.
  - **Engine gap closed:** `LightingRenderer::update` took an arbitrary first-16 lights (query
    order) once a scene exceeded the 16-light hard cap, so distant lights popped in/out at
    random. Now it selects the **nearest 16 to the camera** (`select_nearest_lights`,
    distance-sorted) and warns once. The level holds 18 lights at full clear, so the cull is
    exercised in real play. Unit tests in `src/renderer/lighting.rs`.
  - **Engine bug fixed (post-process):** `PostProcessConfig { enabled: true }` panicked on shader
    creation — `post_process.wgsl` indexed its bloom tap-offset array (declared `let`) by a loop
    variable, which naga rejects. Changed to `var` (an addressable array can be dynamically
    indexed). Latent because post-processing had never run in a game and CI doesn't execute the
    windowed app; this example is the first runtime exercise of post-processing.
  - **Engine bug fixed (HiDPI lighting):** the lighting pass projected `PointLight` positions with
    the physical surface size while the sprite pass uses the logical viewport, so on Retina
    (scale 2) every light drifted off its sprite and rendered at half radius. Fixed to pass the
    logical viewport to `LightingRenderer::update` (`src/app/render.rs`). Latent because lighting
    had never been in a game and aligned by coincidence on scale-1.0 displays.
  - **Engine bug fixed (HUD darkened by lighting):** `TextQueue`/`DrawText` rendered into the
    scene *before* post/lighting, so the dark-dungeon HUD was unreadable. Moved the text pass to
    run after post+lighting onto `final_view` (`src/app/render.rs`). Trade-off: `DrawText` is no
    longer post-processed (use egui for that).
  - **Documented limitations (bigger than a "small fix", deliberately left as known limits):**
    no occlusion/shadows (light passes through walls — radial attenuation only); no real
    per-sprite normal maps (removed in v2); lighting is **native-only** (the App render path
    forces it off on wasm, so the wasm build renders unlit — PostProcess still works on wasm);
    the 16-light hard cap is retained (the nearest-16 cull makes it graceful, doesn't raise it).

## Coverage follow-up — animation blend cluster (candidate I, 2026-06-05)

- **I — Blend locomotion** (`blend_locomotion`, `examples/blend_locomotion.rs`, with the
  `gen_blend_sheet` asset generator): first use of `BlendTree1D` in a real interactive loop. One
  speed parameter — driven by accelerate/decelerate input — maps to idle/walk/run clips and the
  engine crossfades between them. A procedurally generated spritesheet (3 hue-distinct rows) makes
  the blend legible.
  - **Engine bug fixed (stranding):** `BlendTreeSystem` recorded `last_clip = target` even when it
    *skipped* the transition under the `is_crossfading()` guard, so crossing two thresholds within
    one crossfade (a fast idle→walk→run) dropped the second transition and stranded the character
    on the intermediate clip. Now it defers and re-evaluates after the crossfade ends. Latent
    because `BlendTree1D` had never been driven by live gameplay. Regression test in
    `src/animation/blend_system.rs`.
  - **Engine feature (true crossfade):** the crossfade was a 50% UV hard-swap (a visible pop) and
    `BlendWeight` was a dead output. Added a true 2-UV shader-lerp cross-dissolve — `AnimationSystem`
    emits a new `BlendUv { to, weight }` component; `InstanceRaw`/`sprite.wgsl` carry a second UV +
    blend factor and the fragment shader `mix`es the two frames (single-sample on the common
    `weight = 0` path). Additive and cross-platform (the blend works on wasm too). First runtime
    exercise of the sprite crossfade path — CI compiles but never runs the windowed app, so this is
    validated by a native run.

## Coverage follow-up — physics joints (candidate J, 2026-06-05)

- **J — Crane wrecking-ball** (`crane_wrecking_ball`, `examples/crane_wrecking_ball.rs`): first
  playable-example use of the physics **joint** API. A kinematic crane cart hangs a revolute-pinned
  arm with a distance-tethered ball; the player rolls the cart left/right to swing the ball and
  knock a 4-block stack off its pedestal (win + `R` reset). Exercises `add_revolute_joint` +
  `add_distance_joint`, which shipped with creation/removal unit tests but **zero game/example
  coverage**. (Note for a future cycle: the prior plan mis-scouted this as a *missing* public API —
  it already existed in `src/physics/world/joints.rs`; the real gap was the missing playable example.)
  - **Engine bug fixed (rotation sync):** `PhysicsSystem` synced only `Transform.position`, silently
    dropping body rotation, so the swinging arm rendered bolt-upright while the physics rotated
    underneath. Now it also writes `Transform.rotation` from the body angle (rotation-locked bodies
    are unaffected — angle always 0). Latent because no in-engine `PhysicsSystem` user had a freely
    rotating body. Regression tests in `src/physics/system.rs`.
  - **Joint constraint coverage:** added tests that step under gravity and assert the constraint
    holds (revolute keeps the arm pinned at the pivot; distance holds rest length), beyond the prior
    creation-only tests.
  - **Deferred (noted, not fixed):** `add_distance_joint` is implemented with `SpringJointBuilder`
    (stiffness 1000 / damping 10) — it is a stiff spring, not a rigid link, and there is no
    `add_fixed_joint`. Fine for ropes/tethers; revisit if a future example needs a rigid weld.

## Coverage follow-up — RenderTarget / OffscreenCamera (candidate K, 2026-06-05)

- **K — Security camera** (`security_camera`, `examples/security_camera.rs`): first **playable-game**
  use of `RenderTarget` / `OffscreenCamera`. A stealth puzzle — a guard patrols an **entirely
  offscreen** room whose only view is a wall monitor (an `OffscreenCamera` → `RenderTarget` sampled
  by a `Sprite`); time the doorway crossing by the guard's position on the monitor, reach the exit to
  escape, get caught to reset. The API already shipped with **two tech demos** (`minimap`,
  `split_screen`), so unlike past candidates the gap was *not* a missing API — it was (a) no playable
  game, and (b) a latent render bug only a disjoint offscreen region could expose.
  - **Engine bug fixed (offscreen renders with main camera):** the sprite renderer's camera uniform
    is one shared buffer written via `queue.write_buffer`; the offscreen pass + main pass shared a
    single command submission, and only the last write to that buffer wins per submit — so every
    offscreen target rendered with the main camera. The two demos hid it (their offscreen content
    overlaps the main view); the disjoint guard room exposed it (the monitor showed the corridor).
    Fixed by submitting each offscreen target in its own command buffer (`src/app/render.rs`). Not
    unit-testable (no GPU in CI); GPU-validated by the native run + playtest.
  - **`split_screen` self-capture crash fixed:** it used `layer_mask: 0`, drawing its RT display
    sprites into the targets they sample (a render-pass usage conflict) — it crashed on frame 2.
    Fixed by masking the display sprites out (`layer_mask: 1 << 0`), matching `minimap`. Pre-existing
    (crashes on the committed code too), surfaced while fixing the offscreen-render bug.
  - **Self-capture is a user-side concern:** the engine renders whatever layers the `OffscreenCamera`
    mask selects; an RT-display sprite must be excluded via `layer_mask` (or layer) so it is not drawn
    into the target it samples. `minimap`/`security_camera` do this; the old `split_screen` did not.
  - **Deferred (noted, not done):** no screen-anchored/HUD helper for RT display (examples place the
    monitor in world space); no RT resize-on-viewport or per-target clear color; retained UI widgets
    (`src/ui/`) cannot sample an RT (the immediate `DrawImage` overlay can). Add only when an example
    needs them. `split_screen`'s on-screen *layout* also predates the top-left camera convention and
    is left as a separate cosmetic item.

## Coverage follow-up — Timeline / cutscene (candidate L, 2026-06-05)

- **L — Timeline cutscene** (`timeline_cutscene`, `examples/timeline_cutscene.rs`): first use of
  `Timeline` in a **playable** scene. Walk into a rune → a cutscene pans/zooms the camera, slides two
  gate panels apart, and fades a full-screen overlay (all `Timeline` tracks); Space skips, control
  returns when it ends, then you cross the now-open gate to the exit. `Timeline` shipped with unit
  tests but **zero** example/game usage — the gap was "no playable game" plus one real API hole.
  - **Engine gap closed (camera could not be Timeline-driven):** `TimelineSystem` only wrote an
    entity's own `Transform`/`Sprite`; the `Camera` is a *Resource*, so a cutscene camera move was
    impossible without bespoke per-example code. Added a zero-size `CameraTarget` marker + a
    `Timeline::zoom` track — a timeline on a `CameraTarget` entity writes its `position`/`zoom`
    tracks straight into the `Camera` resource (a virtual camera rig). Additive; ordinary timelines
    are unaffected (`zoom` is empty by default). Unit-tested in `src/timeline.rs` (camera pos/zoom
    driven; own-Transform ignored for a marked entity; an unmarked timeline leaves the camera alone).
  - **Skip / return-to-control are example-side:** skip sets each timeline's `time = duration` (the
    system then samples the final keyframe), and the example polls the rig timeline's `is_finished()`
    to hand control back — no engine on-finish hook was needed (engine-fix bar = fix only the gap the
    example hits).
  - **Deferred (noted, not done):** no on-finish event/callback (poll `is_finished()`); no camera
    rotation track (pan + zoom only); no multi-timeline sequencing helper. Add only when an example
    needs them.

## Coverage follow-up — Networking / multiplayer (candidate M, 2026-06-08)

- **M — Coin race** (`coin_race_game` + `coin_race_server`,
  `examples/games/coin_race/`): first use of `NetworkClient`/`NetworkSystem`/`NetworkEvent` in a
  **playable** game with a goal and a win condition. Two+ players race to collect coins; the server
  is **authoritative** (owns the coin field + scoreboard), so contested pickups resolve correctly
  (two players touch the same coin → only the first `grab` to reach the server scores). This closes
  the last never-in-a-game subsystem: networking previously had only the `mp_server`/`mp_client`
  position-relay *demos*, no game.
  - **Why authoritative (not a dumb relay like `mp_client`):** a relay can sync positions but can't
    arbitrate a shared resource. The example needed a server that owns state, which exercises the
    full request→authoritative-decision→broadcast loop the relay never touched. The server is a
    standalone `[[example]]` binary (raw `tungstenite`, dependency-free xorshift for coin spawns),
    mirroring the `mp_server` pattern.
  - **No engine gap forced a change.** The networking API carried a real authoritative game as-is:
    `NetworkClient::connect` + `NetworkSystem` (polls → `Events<NetworkEvent>`) + `send_text` round-
    trips, and `NetworkEvent::Disconnected` is emitted on both remote-close and socket error
    (native), so the client shows "server down" from events alone — **no `is_connected()` needed**
    on native (it exists only on wasm; the asymmetry stayed unforced, so it was left as-is per the
    "fix only the gap the example hits" bar).
  - **Surfaced-but-deferred:** every networked game reimplements the `HashMap<id, Entity>`
    remote-entity bookkeeping (spawn/update/despawn by network id) inline — both `mp_client` and
    `coin_race` do it. A reusable helper is a candidate, but two examples isn't enough signal to fix
    the abstraction shape; deferred to avoid premature API.
  - **Verification:** server logic + authority + relay + lifecycle proven end-to-end by a throwaway
    `tungstenite` probe against the real server binary (contested-coin rejection, position relay,
    `bye` on disconnect — all confirmed); engine client render + live multiplayer scoreboard +
    remote-player sync confirmed by a 2-window playtest screenshot. Server has 5 unit tests.
  - **Browser follow-up (wasm, v4.1.0):** `coin_race_game` now also runs on the web — a
    `#[wasm_bindgen] run_coin_race` entry in the example + `examples/games/coin_race/web/`
    (`index.html` + `build.sh`). This closes friction point 3 (no example ran a networked game on
    wasm) and establishes the reusable "ship an engine *example* to the web" path
    (`cargo build --example` + `wasm-bindgen`), distinct from the lib-only `wasm-pack` demo
    (`examples/wasm/`). The game code stays in the example, not the engine library. Verified: a
    browser tab's wasm WebSocket connects to the native authoritative server and renders the
    player avatar + the server-spawned coin field via WebGL2 (headless-Chrome screenshot).

Remaining never-in-a-game subsystems (candidates for later dogfooding cycles, none scheduled):
none — every engine subsystem now has at least one playable example.

## Alignment check — previously "planned" items vs the reset vision

Vision criteria: (1) fork-friendly skeleton, (2) genre-agnostic 2D, breadth-first,
(3) validate via playable examples, (4) semver after v1.0.

| Planned item | Nature | Alignment | Verdict |
|--------------|--------|-----------|---------|
| **Entity Generation v2** (`docs/ENTITY_GENERATION_V2_PLAN.md`) | correctness/safety, breaking | Fits the fork-friendly/learning goal, but it is neither breadth nor example-validated; it is a v2-only breaking change | **Cancelled (archived)** — removed from planned work; design preserved in the archived doc for a possible future v2.0.0. |
| **Dependency security follow-up** (glyphon→lru `RUSTSEC-2026-0002`, `paste` unmaintained) | maintenance/hygiene | Needed for a trustworthy forkable engine, but it is a renderer/wgpu-major migration: high-risk, non-breadth, non-example | **Cancelled (archived)** — removed from planned work; recorded as accepted/known risk in `docs/SECURITY_HARDENING_2026_05.md`. |
| **2D skeletal animation** | new feature | Directly fits genre-agnostic 2D breadth and is naturally validated by a playable example | **Done** — implemented as candidate **F** (`src/skeletal.rs`, `examples/skeletal_puppet.rs`). |

**Takeaway:** of the three pre-existing planned items, only skeletal animation matched the
current breadth-first + dogfooding priority and is now done. The other two were cancelled
from planned work and archived: Entity Generation v2's design is preserved for a possible
v2.0.0, and the dependency advisories are recorded as accepted/known risk.
