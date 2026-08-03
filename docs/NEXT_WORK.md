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
| **D** | Simple shooter (bullets, waves) | `ParticleEmitter`, `Timer`, collision layers, audio buses | pooling/spawn bursts, perf |
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

## Release/hardening follow-up — v4.1 finalize + repo-wide English (2026-06-09)

Not a breadth candidate — release hygiene, wasm hardening, and a docs pass closing the loose
ends the wasm work left behind:

- **`v4.1.0` tag moved to `ebd9081`** (was `7c6f9c0`, before the default-font + canvas-size
  fixes) so the tagged wasm `coin_race` renders a correct HUD on Retina; rust-survivors's
  v4.1.0 pin (`e6176fa`) pushed to its origin. All 3 wasm fixes are wasm-only/additive, so
  native consumers are unaffected at any v4.x pin.
- **`examples/wasm` `run_demo` re-verified** on headless Retina (DPR=2): squares fill the full
  1280×720 viewport and the HUD text renders in the embedded DejaVu font (its first wasm text
  render, via the `DEFAULT_FONT` fallback) with no clipping. No engine change needed.
- **`scripts/wasm_smoke.sh`** added — an optional local headless-Chrome render+network smoke
  check (connect + non-blank frame + saved screenshot). It guards the *catastrophic* class
  (CI builds wasm but never runs it); it does **not** auto-catch the subtle 3-bug class
  (off-screen sprites / missing / shifted text) because each yields a wrong-but-NON-blank frame
  — eyeball the saved shot for those. Documented in `CLAUDE.md`.
- **Engine source is now fully English.** All `src/` (~110 files) and example comments plus
  developer-facing diagnostics (log/panic/expect/assert messages) were translated. Deliberate
  Korean DATA is kept: the `"ko"` locale values (`locale.rs`/`ui::localized`) and the Hangul/IME
  test fixtures (`text_input.rs`/`input::state`/`renderer::text`).

## Deferred follow-ups (not breadth) — see `plans/handoffs/PLAN_networking-dogfood_deferred-polish_2026-06-09.md`

These are tracked in the seq-3 PLAN for a future (monitor-on) session; none are breadth gaps:

1. **wasm Retina crispness** ✅ *done (2026-06-09)* — the wasm drawing buffer is now sized to
   logical × devicePixelRatio (uniform, capped at the WebGL2 2048 limit) while the canvas CSS box
   stays logical, so the browser maps it 1:1 (crisp) instead of upscaling a logical-size buffer.
   `ViewportSize` stays logical; `DisplayScaleFactor = buffer/logical`, so text renders at device
   resolution. Logical size = the authored `<canvas>` attributes (`WASM_LOGICAL_SIZE`), not
   `WindowConfig` (which a scene reset reverts). Verified on a real-GPU Retina browser + headless
   DPR=2 (coin_race 800→1600; run_demo 1280→2048 clamp path). (`dce44ae`)
2. **Reusable remote-entity helper** — *minimal slice done (2026-06-09); richer version evaluated
   against the 3rd example → keep minimal.* Shipped `engine::RemoteEntities<K>` (v4.3.0) and
   migrated `mp_client` + `coin_race`. The **3rd distinct example (the client-prediction shooter,
   `examples/games/predict_shooter/`)** is built, real-play-verified, and ships native + to the
   browser (`web/` harness), which answered the key open questions: interpolation
   (`client_net::Interp`) is **orthogonal** to the lifecycle map (they compose as parallel maps),
   so it is **not** folded into `RemoteEntities`; and `Interp`/`Prediction` are **not promoted** to
   public engine helpers yet (single call site — same discipline). The v4.3.0 API is the right
   shape and stays unchanged. Remaining open questions (#3–#7: per-entity update callbacks, typed
   entities, staleness, binary protocol, disconnect policy) await examples that stress them — see
   `docs/REMOTE_ENTITIES_DESIGN.md`.
3. **New breadth feature exploration** — *audit done (2026-06-09).* Conclusion: breadth is genuinely
   complete in the "every subsystem has a playable example" sense; no high-value *new subsystem* is
   compelling now. The audit did surface **two small, broadly-useful API gaps** a first-time forker
   trips on, each worth one focused session (not new subsystems — polish):
   - **Camera world-bounds clamping** — ✅ *done (2026-06-09).* Added `Camera::bounds:
     Option<(Vec2, Vec2)>` + `clamp_to_bounds(viewport_w, viewport_h)`; App auto-clamps each frame
     right after `Camera::update`, so follow-based cameras need no extra code. `lit_dungeon` dropped
     its hand-rolled `CameraFollowSystem` clamp for `camera.bounds`. 6 unit tests. Additive, no
     version bump.
   - **`InputMap` gamepad binding** — ✅ *done (2026-06-09).* Additive `bind_gamepad_button` /
     `bind_gamepad_axis` (+ `AxisBinding`, re-exported as `engine::AxisBinding`) and
     `is_pressed_with_gamepad` / `just_pressed_with_gamepad` / `just_released_with_gamepad` that OR
     keyboard + gamepad; keyboard-only methods unchanged (generic bound widened to
     `A: Eq + Hash + Clone`). `survivor_game` drives every action from keys OR a controller
     (DPad + sticks). 12 unit tests. Additive (minor `Clone` bound), no version bump.
     **Live-controller validation (seq 7):** keyboard + unit paths verified; live gamepad input
     could *not* be validated on macOS — modern macOS's GameController framework exclusively owns
     Xbox/PlayStation pads (`UsbExclusiveOwner`), so gilrs (IOKit HID) sees a `Connected` event but
     zero input. Validate on Linux/Windows or a generic-HID (DInput) pad. See the seq-7 section below.
   Other candidates (tilemap autotiling, runtime tilemap mutation, save-migration, data-driven
   anim/particle assets, diagonal pathfinding, RTL/per-locale fonts, audio ducking) are
   higher-effort/narrower or already documented-as-deferred — none clear the bar now.

## Networking-dogfood seq 7 — INTERP_DELAY settle + gilrs crash fix + macOS gamepad limit (2026-06-09)

Follow-ups to seq 6 (Phase-3 polish). Engine **4.3.0 → 4.3.1** (the gilrs crash fix is a real bug fix).

- **`INTERP_DELAY_DEFAULT` settled at 60 ms** (was 100 ms) by real-play feel testing in
  `predict_shooter` (server + 2 windows): below ~40 ms bullets ghost/trail; above ~70 ms a bullet
  lingers at the shooter's old position when moving-and-firing; 60 ms (≈2× the 33 ms snapshot
  interval) is the sweet spot. The live `[`/`]` tuner stays; its HUD label was clarified to
  `[ -10ms  ] +10ms` (the old `[ / ]` was misread as the slash key).
- **gilrs crash fixed (surfaced by the controller test).** With a controller connected, gilrs
  0.10.10 panicked inside `next_event()` (`gamepad(id).unwrap()` on `None`, gamepad.rs:278/458),
  crashing every windowed example ~1 s after launch. Two fixes: (a) a `catch_unwind` guard around the
  gilrs poll in `src/app/window.rs` (mirrors the per-system isolation in `schedule.rs`) — a flaky
  controller now disables gamepad input for the session instead of crashing; (b) upgraded **gilrs
  0.10 → 0.11.2** (gilrs-core 0.5.15 → 0.6.8, reworked macOS HID backend) — no more crash. The unwrap
  path still exists in 0.11.2, so the guard stays as a durable safety net. Full `+1.88.0` gate green.
- **macOS gamepad limitation documented.** Modern macOS exposes Xbox/PlayStation pads via the
  GameController framework, which takes *exclusive ownership* (`UsbExclusiveOwner=XboxUSBDevice`,
  `com.apple.gamecontroller.driver.XboxGamepad`). gilrs uses IOKit HID, so such pads emit a
  `Connected` event but no input — confirmed across 5 probe runs with a GameSir-G7 Pro (Xbox-licensed)
  and an Xbox Series pad. Gamepad input works on Linux/Windows and with generic-HID (DInput) pads; a
  native macOS GCController backend would be a large separate effort. Noted in the REFERENCE.html
  gamepad section.
- **REFERENCE.html** updated for the seq-6 public APIs (Camera `bounds`/`clamp_to_bounds`, InputMap
  gamepad + `AxisBinding`, gamepad basics) — they were absent from the manual.

## Networking-dogfood seq 8 — 2nd interpolating example + `SnapshotBuffer<T>` promotion (2026-06-09)

Closes the deferred-polish item #2 follow-up ("if a *second* interpolating example appears,
`engine::SnapshotBuffer<T>` is a clean additive helper to extract then" — `docs/REMOTE_ENTITIES_DESIGN.md`).
Engine **4.3.1 → 4.4.0** (additive public API).

- **`engine::SnapshotBuffer<T: Lerp>`** (`src/network.rs`) — the per-entity snapshot-interpolation
  buffer that was `predict_shooter`'s private `Interp`, promoted to a public, generic helper reusing
  the existing `Lerp` trait (`f32`/`Vec2`/`[f32;4]`/`Color`). `push(t, value)` stamps a snapshot at
  the client clock; `sample(rt)` returns the lerped value at a past render time (clamps at the ends).
  Orthogonal to `RemoteEntities` (lifecycle map vs. value history) — games keep them as parallel
  maps. Doctest + 6 unit tests.
- **N — Orbital Dodger** (`orbital_dodger_game` via `orbital_dodger` + `orbital_dodger_server`,
  `examples/games/orbital_dodger/`): the 2nd interpolating example and the promotion's acceptance
  test. **Interpolation-only** (no prediction): a broadcast server drifts spinning hazards at a low
  10 Hz; the client interpolates them (`SnapshotBuffer<Vec2>` position + `SnapshotBuffer<f32>` spin
  angle — two channels per hazard, which is what justified the generic `T`); the local player is
  purely client-side. `I` toggles interpolation off to reveal the raw 10 Hz judder. Ships native +
  to the browser (`web/`). Proves interpolation is a standalone concern, orthogonal to `Prediction`.
- **`predict_shooter` migrated** onto `SnapshotBuffer<Vec2>` (deleted its private `Interp`;
  behavior-identical). `Prediction` stays example-local (still one call site, a *local* concern).
- **Open question #1 (interpolation) closed** in `docs/REMOTE_ENTITIES_DESIGN.md`; #3–#7 still await
  examples that stress them.

## Coverage follow-up — tilemap mutation + autotiling (candidate O, 2026-06-15)

Closes two long-deferred candidates from the 2026-06-09 breadth audit (*runtime tilemap
mutation* + *tilemap autotiling*) in one arc. Engine **8.1.10 → 8.2.0** (additive).

- **O — Dig quest** (`dig_quest_game`, `examples/games/dig_quest/`): a destructible-terrain
  top-down miner — the player digs through a field of solid dirt to a buried gem. First
  playable use of runtime `Tilemap` mutation + the `TilemapAutotile` component. Native
  playtest verified the whole loop: digging a cell updates the autotile outline immediately
  (neighbor propagation keeps tunnel walls continuous) **and** frees the static collider so
  the player walks in; reset restores the field; post-reset re-dig still works.
  - **Engine gaps closed (all additive, v8.2.0):** (1) `Tilemap` was static after the first
    `TilemapSystem` spawn — added `set_tile`/`get_tile`/`dims`/`cell_at_world`/
    `cell_center_world` + a **reactive** `TilemapSystem` that diffs a cached grid and updates
    only changed cells. (2) No autotiling — added `TilemapAutotile` (Edge4 16-tile + Blob8
    47-blob, `edge_16`/`blob_47`, `with_oob_filled`, `compute_tile_mask`) with 8-neighbor UV
    refresh so holes keep continuous outlines. (3) Tile colliders were one-shot — added
    `TileColliderIndex` + `PhysicsWorld::sync_static_from_tilemap` (incremental diff add/remove);
    `add_static_from_tilemap` stays for static maps.
  - **API surfaced-and-fixed by the example (VISION "fix awkward API before release"):** the
    `with_oob_filled` builder (the raw field default was a silent visual footgun); doc notes on
    `set_tile`'s value encoding and on using `sync_static_from_tilemap` for the *initial* build
    (mixing it with `add_static_from_tilemap` double-adds colliders so a dug cell never frees —
    found via a real bug in the first example draft).
  - **Deferred (noted, not done):** multi-terrain autotiling (v1 = single terrain, non-zero
    connects to non-zero — `ConnectRule` is the extension point); a procedurally-generated
    Blob8/47 sheet (the code path is supported; the shipped sheet/example use Edge4/16);
    `PathGrid` runtime sync (no enemies in the example); a `CharacterController::top_down()`
    preset (snap-to-ground is platformer-tuned but didn't visibly hurt the dig demo); a
    `World::with_resource_mut` helper for the remove/insert borrow dance (pre-existing, not
    tilemap-specific); editor tile-painting (a natural reuse of `set_tile` + the reactive
    system, left for a future editor cycle).

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

---

# Open backlog (2026-08-03)

> **This section replaces the per-session handoff for completed sessions.** A handoff is now
> written only when work stops *mid-task*; a finished session updates the backlog here instead.
> Session narrative lives in commit bodies and `docs/CHANGELOG.md`, durable lessons in
> `docs/PATTERNS.md` / `docs/VERIFICATION.md`. What has no other home is the *decision backlog*
> below — and that is exactly what kept getting buried inside 60 KB handoff files.

## Board gate — check this first, every session

Both channels were **empty** as of 2026-08-03:

- `../dungeon-merchant/docs/engine-wishlist.md` — next free **EW-012**, unmoved since 2026-07-27
- `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — `_None._`

Check whether they **moved** (`git log -1 --date=short -- <file>`), not whether they look empty.
A filed request preempts everything below.

## Closed on 2026-08-03

- **`SURVIVOR_SELFTEST=1`** (v0.143.1) — carried three sessions, done.
- **`add_system` before `set_scene` is now loud** (v0.143.2) — the silent-drop footgun that cost
  `beat_crawler` several releases of a dead headline feature now emits a warning naming the count.
- **`play_sfx_metered` has a playable-game caller** (v0.143.3) — `beat_crawler`'s melee impact,
  closing the VISION acceptance test v0.143.0 shipped without.
- **The gate's fusion trap** — Trap 5 now states that the cleanup step and the background run must
  be separate *calls*, which is the part that actually failed on 2026-08-03.

## Open — engineering

| Item | State |
|---|---|
| **4th procgen mode** (drunkard's walk) | Unchanged, still the lowest marginal value: the engine cannot fail at it, so nothing is learned. |
| **`embedded_image` web harness** | Carried unanswered since ~2026-07. Six sessions is enough — **either build it or close it**, do not carry it a seventh. |
| **`add-facade-capability` skill** | n=5 now (the facade + native + wasm + policy-module shape has repeated that many times). Deferred; the next facade capability makes the case by itself. |
| **`bands()` for a metered one-shot** | Deliberately zeros. No use case has appeared in three sessions — treat as **closed** unless one does. |

## Open — process

- **`main`-push blocking hook** — proposed 2026-08-03, not applied, low priority (no observed
  violation). Lives only in `.claude/proposals/2026-08-03.md`, which is gitignored.
- **`handoff` / `wrap` skills exceed the 800-char guideline** (2,245 and 3,195) — split the detail
  into reference files. Also gitignored, so this line is the only tracked record of it.
- ⚠️ **The local gate hook over-matches.** It denies any Bash command containing both the gate
  script's name and a delete, which includes *a commit message or a doc that merely mentions
  them*. It fired twice on 2026-08-03 on exactly that. Workaround in use: write the text to a file
  and pass the file. Narrow the pattern, or change the decision from deny to ask.

## Known-unfalsifiable checks — do not mistake these for guarantees

- **`BEAT_CRAWLER_SELFTEST` exit `8`** ("the two meters are not independent") **cannot fail on
  native.** Each meter is a tap on its own channel, so the spectrum read never sees the mixer
  output — verified by firing the bass-heavy soundtrack as the impact clip and measuring no
  change at all. It is a tripwire for the **wasm** topology, where several sources share one
  `AnalyserNode`. Only its lower bound (the clock keeps working while impacts sound) guards
  anything today.

## Standing risks

- **Audio is outside CI entirely.** Every audio claim in v0.140–v0.143 rests on a local device.
- **A headless capture cannot photograph a meter** — fixed dt, no wall clock. Three sessions have
  now reached for `ENGINE_CAPTURE` before remembering this.
- **An example's headline feature can degrade gracefully into silence.** That is what a
  `<NAME>_SELFTEST` prevents; `beat_crawler` and `survivor` have one, the other games do not.
