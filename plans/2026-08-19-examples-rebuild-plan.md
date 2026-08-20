# Examples rebuild — design proposal

> Written 2026-08-19, immediately after v0.153.0 deleted the whole `examples/` tree (22 playable
> games + ~85 feature demos, 247 files, 19,154 lines of games). **Nothing here is built yet.**
> This is a proposal to argue with, not a spec — see the CLAUDE.md rule about filed diagnoses.

## The problem the old tree actually had

The maintainer's reason for deleting it was "too many". That is the symptom; the mechanism is
worth naming, because rebuilding *fewer of the same shape* would reproduce it.

The old tree grew **one example per feature**. 85 top-level demos, each opening a window to show
one API, plus 22 games. Three consequences, all measurable in what was deleted:

1. **Coverage was wide but shallow.** 85 demos proved 85 APIs compile and draw something. Almost
   none proved two subsystems work *together*, which is where the engine actually broke — every
   bug the games caught (the `Audio` scene-reset drop, the unregistered event bus, the pipeline
   cache keyed on the wrong format) was an interaction bug.
2. **Only 11 of 22 games carried a `<NAME>_SELFTEST`.** The other 11 were "the failure is visible
   in a screenshot", which means a human had to look, which means nobody did. Half the acceptance
   layer was decorative.
3. **6 of 12 browser smokes asserted byte sizes only.** A green run meant "a frame drew", not "the
   right frame drew" — documented as eyeball-it, and therefore not run.

So the rebuild's design rule is the inverse: **few games, each structurally dense, every one
self-verdicting.**

> **Structurally dense** is the `beat_crawler` test: remove any one subsystem the game names and
> the game *stops working*, rather than losing a decoration. That is what makes a game an
> acceptance test instead of a screenshot.

## Proposal: 5 games, ~4,000 lines

Down from 22 games / 19,154 lines — a ~78% cut, with **more** gated coverage than before because
all five carry selftests where only 11 of 22 did.

| # | Name | Genre | Lines (est.) | Selftest | Web |
|---|---|---|---|---|---|
| 1 | `platformer_game` | platformer | ~800 | ✅ | — (rapier2d is native-only) |
| 2 | `survivor_game` | top-down action + shooter | ~900 | ✅ | ✅ **audio smoke** (GPU particles stay native-only) |
| 3 | `rpg_quest_game` | RPG | ~1,000 | ✅ | ✅ save/localStorage smoke |
| 4 | `puzzle_grid_game` | puzzle | ~600 | ✅ | ✅ render smoke |
| 5 | `netplay_game` | networked (+ `netplay_server`) | ~900 → **3,006 actual** | ✅ | ✅ render + WebSocket smoke (phase 5b) |

⚠️ **This ships four of VISION's five genres**: `survivor_game` covers both "shooter" and
"top-down action". **Decided 2026-08-19** — both exist to put the engine under *count* pressure
(pool churn, spatial grid, steering, the light cap), and that is one world, not two. Splitting them
back out is a later edit, not a redesign. `docs/VISION.md` needs its genre list amended to say four
covered + one folded, or the next session will read it as a gap.

---

### 1. `platformer_game` — platformer

**Structurally required:** `PhysicsWorld` / `PhysicsBody` / `PhysicsSystem`, `CharacterController`
(+ `one_way_tolerance`, `request_drop`), `Tilemap` + `TilemapColliders` + autotile,
`AnimationStateMachine` + `BlendTree1D` + `AnimationPlayer` + `BlendWeight`, `SpriteFlip`,
`TextureAtlas` / `AtlasSprite`, `Camera` lookahead + `clamp`, `ParallaxLayer`, `InputBuffer`
(coyote + jump buffer), `HitFlash`, `TriggerZone`, a joint (`add_prismatic_joint`) for the moving
platform.

**Why these together:** the animation state machine is driven by *physics* state (grounded,
vertical velocity), and the one-way platform only behaves if the controller's tolerance and the
tilemap's collider sync agree. Neither is checkable in isolation.

**Selftest (`PLATFORMER_SELFTEST`)** — `ENGINE_INPUT` replays a fixed script, asserting invariants:
- coyote-time jump **succeeds** at `DEFAULT_COYOTE_SECS - 1` frame off-ledge and **fails** at
  `+1`. Two-sided: a check that only tests the success side passes on an engine with infinite
  coyote time.
- the state machine reaches `fall` → `land` in that order (assert the transition, not the final
  state — the player is idle at the end either way).
- `request_drop` puts the player *below* the one-way platform within N frames, and a normal jump
  from beneath crosses it upward.

### 2. `survivor_game` — top-down action + shooter

**Structurally required:** `Pool`, `SpatialGrid` / `Collider` / `CollisionLayer`, steering
(`Seek` / `Arrive` / `Wander` / `Flee` + `SteeringSystem`), `Timer`, CPU `ParticleEmitter` +
`ParticleBurst`, **native-only** `GpuParticleEmitter`, `YSort`, `PointLight` / `AmbientLight`
(>16 sources on purpose, to pressure the cap), `PostProcessConfig` + `Tonemap` + bloom, `Rng`
(seeded waves ⇒ reproducible runs), audio bus mixer + ducking + positional + `AudioLevels`,
`FloatingText`, `SpriteTrail`, `ProfilerData`.

**Why these together:** this is the only game that puts the engine under *count* pressure. Pool
churn, grid rebuilds, steering for ~200 agents and the 16-light cap are all N-dependent, and none
of them fail at N=3.

**Selftest (`SURVIVOR_SELFTEST`)** — seeded, headless, invariant-based:
- **pool conservation**: after every spawned bullet is released, active count returns to its
  starting value. Asserts the invariant, not a number, so a background spawner cannot skew it.
- light count exceeds 16 and the **nearest-to-camera** light is the one that survives culling.
- `AudioLevels.rms > 0` while a clip plays. ⚠️ The device probe must be a throwaway `Audio::new()`
  **before the app is built** — see `docs/VERIFICATION.md`: sampling "no device" from the thing
  under test lets a real failure forge a skip.

### 3. `rpg_quest_game` — RPG

**Structurally required:** `DialogueBox` / `DialogueChoice` (incl. `cond_all` / `cond_any`) /
`DialogueSystem`, `DataTable` + `RonRegistry` + hot reload, `Prefab` (`EntityDef` / `SceneDef`),
`save` (AEAD + migration + plaintext RON), the UI set (`Panel` / `LayoutSystem` / `Slider` /
`CheckBox` / `ScrollView` / `TextInput` / dropdown / tabs / tooltip), `LocalizedText` +
`LocaleResource`, scene `Replace` **and** `Push`/`Pop` + `register_persistent`, `SceneTransition`,
`PathGrid` + A*, `BehaviorTree`, `Timeline` / `Coroutine` cutscene, `TextMeasurer`, `NineSlice`,
and the editor hook `register_editable_component` + Save Scene.

**Why these together:** this is the game that owns *state that must survive*. The scene stack, the
persistence registry and the save file all answer the same question — what is allowed to be
dropped — and the v0.139.1 audit showed that question is only answerable under real transitions.

**Carry a docked-editor capture (added 2026-08-19, from the engine session).** This game is the only
one of the five that has both a `SceneTransition` and the editor, so it is the only place the render
review's *docked `IrisIn`/`IrisOut` draws elliptical* row can be settled — `TransitionRenderer::update`
is handed `gpu.config` while `run_pass` targets the docked RT (`src/app/render/frame.rs:725`; the
correct pair, `(text_w, text_h)`, is already computed 340 lines above and used by the text pass).

⚠️ **The capture needs a control assertion or it proves nothing**, and this is exactly the trap
`docs/VERIFICATION.md` § *Sabotage each half separately* describes: if the docked RT's aspect ratio
happens to equal the surface's, the iris renders as a correct circle and the check passes green
whether or not the bug is present. So the capture must **first assert the two aspect ratios actually
differ** at the captured frame, then assert the iris is circular *in the docked RT's own pixel space*.

**The fix landed first (v0.153.3), so this is a regression guard, not a demonstration** — run against
any commit before it, the check is expected to be RED. The control assertion matters *more* now, not
less: after the fix "the iris looks circular" is the passing condition, and on a window where the two
aspect ratios coincide the pre-fix code produces a circle too, so a green run there proves nothing.
The engine session's note on what the fix actually was is worth carrying: the correct value was
already in the same function and the text pass was already using it, but it was **named** `text_w` /
`text_h` when it meant *the scene target's physical size* — so it read as text-specific and nobody
reused it. It is now `scene_target_w` / `scene_target_h`, with a comment saying any pass drawing into
`scene_target` must use it rather than `gpu.config`. A misnamed value is a value that gets
re-derived wrongly somewhere else.

**Selftest (`RPG_QUEST_SELFTEST`)** — over `App::step_headless`, which crosses scene boundaries a
hand-ticked schedule cannot:
- a registered resource **survives** `Replace`, an **unregistered** one is **dropped**. Both
  halves, in that order — survival alone passes on an engine that never resets, which is exactly
  the bug the check exists for.
- `Push`/`Pop` reset **nothing**: `play_enters` stays `1`. A `Pop` written as `Replace` reads `2`
  and photographs identically.
- the quest gate flips only once `cond_all` is satisfied, and not on either term alone.
- save → load round-trips the quest state; a v1 save file migrates.
- a locale switch retranslates every widget with no manual rebuild.

### 4. `puzzle_grid_game` — puzzle

**Structurally required:** `History` undo/redo, runtime `Tilemap::set_tile` + `multi_edge_16`
autotile + reactive `TilemapSystem`, all three `mapgen` generators + `FovMap`, seeded `Rng`,
progress `save`, immediate-mode `DebugDraw`, `Coroutine`, `Tween`, and the editor's tile-paint
panel.

**Why these together:** the board is fully reconstructed from snapshot state each move, which is
what makes `History` meaningful and what makes autotile correctness observable — a wrong tile
index after an undo is a visible, assertable difference.

**Selftest (`PUZZLE_GRID_SELFTEST`)**:
- **undo restores the exact prior snapshot** — compare serialized board state, not a win flag.
- every generator produces a **connected** map for K seeds (the property each guarantees by a
  different mechanism).
- `FovMap` reveals exactly the cells `PathGrid` says are visible — the two must agree, since
  `FovMap::from_path_grid` is the whole point of that bridge.

### 5. `netplay_game` — networked (+ `netplay_server`)

Folds all four old networked games (`coin_race`, `predict_shooter`, `orbital_dodger`,
`salvage_run`) into one client + one authoritative server.

**Structurally required:** `NetworkClient` / `NetworkSystem`, `RemoteEntities`, `SnapshotBuffer`,
server authority (claim → confirm), client prediction + reconciliation, interpolation, and AOI
streaming with **removal-by-omission**.

**Why one game and not four:** the four differed by *which* networking technique they used, not by
genre. One world can carry all four — an authoritative pickup (claim/confirm), a locally predicted
ship (prediction + reconciliation), server-owned hazards (interpolation only), and a world larger
than the window (AOI). Any one missing is a visibly different game.

**Bonus:** AOI streaming restores the `RemoteEntities` last-seen eviction gate, which
`docs/NEXT_WORK.md` records as **n=0** since the deletion (it was n=1, gated on a 2nd call site).

**Selftest (`NETPLAY_SELFTEST`)**: stands up the server, connects, and asserts reconciliation
converges and AOI eviction fires by omission. ⚠️ It must **fail** when the server binary is
absent, not skip — `cargo run --example netplay_game` does not build `netplay_server`, and a naive step
that lets that skip drops exactly the checks that cover the most. That was measured on the old
tree: with the server hidden, the raw exit code was **0**.

✅ **Shipped 2026-08-21 with 7 checks.** 1-5 are device-free (stream-in; removal-by-omission, both
halves; prediction wired to the ship *and* reconciliation replaying rather than snapping;
interpolation *and* collision reading the drawn position; claim-does-not-delete). 6-7 spawn the real
server on an OS-assigned port and drive **two clients at once**. A missing server binary exits **8**.
All 13 sabotages were verified to fire on the intended check — one had to be narrowed to get there,
which is recorded in `docs/VERIFICATION.md` § *A sabotage that fails the wrong check has not
verified anything*. Two design notes worth carrying forward: the wire format is **RON** (`serde_json`
is not a dependency; `ron` already is), and check 4 needed a hazard faster than any real one, because
at real speeds the drawn/newest gap is smaller than the collision reach and the check could not
discriminate.

---

## The acceptance layer — the part that matters more than the games

The old tree's real failure was here, so this is designed first and the games are built to fit it.

**Five rules, each paid for once already** (`docs/VERIFICATION.md` has the incidents):

1. **Every game carries a selftest.** No "the failure is visible in a screenshot" exemption. That
   exemption is what produced 11 unverified games.
2. **The runner derives its list, never hardcodes it.** A game is a selftest iff it reads a
   `<NAME>_SELFTEST` env var. v0.143.9 shipped a green gate that had never run the selftest which
   was the entire point of that change, because the list was hand-maintained.
3. **A skip is never a pass.** Tolerate only "no audio device", and only when the probe comes from
   a source the failure cannot forge. Everything else is a failure.
4. **Assert invariants, not end states** — especially where a background process (a spawner, a
   respawner) can move what you are counting.
5. **Sabotage-verify every check when you write it.** Break the thing; the check must go red, and
   only the matching half. A check nobody has seen fire is not a check.

**Browser smokes: rebuild 4, not 12.** Only self-verdicting ones (`*_CHECK: PASS` read from the
page title). The 6 byte-size-only smokes were documented as eyeball-it and are not worth rebuilding.

| Smoke | Asserts | Priority |
|---|---|---|
| **audio** (`survivor_game` web build, or a bare harness) | `Audio::levels` reports a live level **and** `Audio::bands` a low-biased spectrum | **1st — the only genuinely lost measured signal** |
| `rpg_quest_game` | AEAD `localStorage` save round-trip | 2nd |
| `netplay_game` | renders **and** the WebSocket handshake really happened | 3rd |
| `puzzle_grid_game` | non-blank render at DPR=2 | 4th |

**Audio is first, and this is measured rather than assumed.** Web Audio genuinely gated from
v0.143.17 to v0.153.0 — `wasm_audio` reported 38/38 and `audio_reactive` reported `rms=0.643` with
bands `low=9.41` / `high=0.00` on a 110 Hz tone, i.e. real spectral discrimination. Native
rodio/ALSA cannot be tested in CI (v0.143.10, five runs, see `docs/VERIFICATION.md`), so the
browser half was the *only* automated audio evidence in the tree. It is the one item where
deletion removed a working measurement rather than an aspiration.

**Two things this deliberately does NOT size up for** — both checked in code on 2026-08-19 rather
than argued:

- **The wasm queue-discard branch** (`gpu == None`, the async adapter warm-up window) is covered by
  **any single wasm smoke**, structurally. `resumed()` sets `window` immediately on wasm and awaits
  the adapter via `spawn_local`; `about_to_wait` requests a redraw every iteration;
  `RedrawRequested` calls `step_frame_once` → `step_frame` → `update(dt)` with **no gpu gate**
  (`src/app/render/frame.rs:710`). So every browser run pushes into the queues and early-returns in
  `render` before the adapter resolves. Rebuilding one wasm smoke covers it; four is not better
  than one here.
- **Layered-text z-interleave** (`interleave_runs`, reached only when `take_layered()` is
  non-empty) is **not** covered by any of the four above, and — verified — was **not covered by the
  deleted gate either**. All seven CI-gating browser smokes had `with_z = 0` and zero widgets, so
  the path never ran in a browser. Covering it needs a *different kind* of smoke — a **widget UI
  example built for wasm** — not more of the same kind. Worth doing, but it closes a hole that
  predates this deletion; do not file it as a regression.

**Restore to CI, in this order:** `scripts/selftests.sh` (rebuilt, derived) → the `wasm-smokes`
job → `scripts/build_wasm_examples.sh`. ⚠️ Restoring the `wasm-smokes` job means **adding its
context back to branch protection**; it was removed in v0.153.0 because a required check for a
deleted job blocks every merge.

## Phasing

Deliberately runner-first. Building game 2 before the runner exists is how 11/22 happened.

| Phase | Deliverable | Why here |
|---|---|---|
| 0 | `scripts/selftests.sh` rebuilt (derived list) + its CI step | Must exist before game 2, or drift starts immediately — **done 2026-08-19** |
| 1 | `platformer_game` + selftest | Densest single genre; proves the runner on a real target — **done 2026-08-19**, 1,779 lines and 7 checks (see `docs/NEXT_WORK.md` on the 2× line estimate) |
| 2 | `rpg_quest_game` + selftest + save smoke | Owns the scene/persistence questions, the highest-risk area — **done 2026-08-20**, 7 checks + the docked-iris render test; 1,885 lines |
| 3 | `survivor_game` + selftest | Needs the audio-probe pattern from phase 2's lessons — **done 2026-08-20**, 7 checks + the nearest-light render test; 1,590 lines |
| 4 | `puzzle_grid_game` + selftest + render smoke | Cheapest game; good place to restore `build_wasm_examples.sh` — **done 2026-08-20**, 7 checks + the script (derived list, two-sided NATIVE_ONLY check); 1,193 lines |
| 5a | `netplay_game` + `netplay_server` + selftest | Most infrastructure per line — **done 2026-08-21**, 7 checks over all four folded techniques; 3,006 lines across client/server/protocol, the only game that is two binaries |
| 5b | restore the `wasm-smokes` job **and its branch-protection context** | **done 2026-08-21** — 2 of the 4 planned smokes (audio + wasm WebSocket), the two with no coverage anywhere; context re-added in the same change. Caught the #494 wasm entry point on its first run. Save round-trip and DPR render deferred — both have native equivalents |

## What this deliberately does NOT cover

Stated because a gap named is a decision and a gap unnamed is a bug:

- **Skeletal animation** (`SkeletalAnimator`, `SkeletalClip`, `BoneTrack`) — no game here needs
  it. Its only demo (`skeletal_puppet`) is deleted. Either fold a skeletal actor into `platformer_game` or
  accept it as uncovered; do not add a 6th game for it.
- **`RenderPlugin`, `ShaderMaterial`, `RenderTarget` / `OffscreenCamera`, split-screen, minimap.**
  Fork-facing extension points. `RenderTarget` is worth folding into `survivor_game` (a minimap) because
  the **pipeline-cache-keyed-by-target-format** trap in CLAUDE.md silently vanishes under
  offscreen targets and nothing else would catch it.
- **Gamepads, windowed playtest, native audio playback.** Unautomatable; they were manual before
  and stay manual. `docs/MACOS_FFI.md` now has no `gamepad_probe` to point at — that is a real
  regression in the ability to check a pad at all, and worth a tiny throwaway binary rather than a
  game.
- **Hot-reload beyond `rpg_quest_game`'s data tables.** The old `DATA_ANIM` / `DATA_PARTICLES` selftests
  did real `notify` watching; folding that into `rpg_quest_game` covers the mechanism once, not three
  times.
- **RTL text, IME, touch, design resolution, window modes.** Previously one demo each. Suggest
  they return as **unit or render tests**, not games — none of them needs a game loop, which is
  why they were shallow demos in the first place.

## Decisions (2026-08-19, maintainer)

All four open questions are settled. Recorded here so they are not re-litigated.

| Question | Decision | Consequence |
|---|---|---|
| Four genres or five? | **Four** — `survivor_game` folds shooter + top-down action | `docs/VISION.md`'s genre list must be amended, or it reads as an unclosed gap |
| Naming | **Descriptive**, `<thing>_game` | Matches the old tree, so `grep -in 'rpg' docs/MODULE_MAP.md` still lands. Costs a repeated `_game` suffix |
| Directory layout | **Flat one level** — `examples/<name>/<name>.rs` | Drops the `games/` tier; every doc path gets one segment shorter. Per-game dirs stay, because assets and `web/` need them |
| Networking game | **Keep it** — 5 games | `RemoteEntities` / `SnapshotBuffer` / prediction keep a consumer, and `docs/NEXT_WORK.md`'s last-seen eviction row (now n=0) becomes reachable again |

### Resulting layout

```
examples/
  platformer_game/    platformer_game.rs   assets/
  survivor_game/      survivor_game.rs     assets/
  rpg_quest_game/     rpg_quest_game.rs    assets/  web/
  puzzle_grid_game/   puzzle_grid_game.rs  assets/  web/
  netplay_game/       netplay_game.rs      server.rs  protocol.rs  web/
```

Selftest variables follow the name: `PLATFORMER_SELFTEST`, `SURVIVOR_SELFTEST`,
`RPG_QUEST_SELFTEST`, `PUZZLE_GRID_SELFTEST`, `NETPLAY_SELFTEST`. The rebuilt
`scripts/selftests.sh` **derives** that list by grepping for the variable, so none of these names
is written down twice.

### Still open

Not decisions so much as things the first phase will answer:

- **Skeletal animation** has no home in these five (see the not-covered list above). Fold a
  skeletal actor into `platformer_game`, or accept it as uncovered — decide when phase 1 lands,
  with the file open, rather than now.
- **`RenderTarget` as a minimap in `survivor_game`** is proposed but unbudgeted. It is the only
  thing that would catch the pipeline-cache-keyed-by-target-format trap, so it is closer to
  required than optional.
