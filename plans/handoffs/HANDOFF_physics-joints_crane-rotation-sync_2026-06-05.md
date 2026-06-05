# skeleton-engine: Crane wrecking-ball example (physics joints) + PhysicsSystem rotation sync

**Date:** 2026-06-05
**Status:** COMPLETED (committed `338eb08`, pushed `b94af88..338eb08`, CI `27002241512` **completed success** 4m51s)
**Bead(s):** none (`bd` not installed in this environment)
**Epic:** VISION feature+example loop — dogfood shipped-but-never-in-a-game subsystems
**Chain:** `physics-joints` seq `1`
**Parent:** none — first in chain
**Prior chain:** none — first in chain

---

## Related Handoffs

This session *opened* by executing the paired PLAN from the **`blendtree1d-locomotion`** chain
(`PLAN_blendtree1d-locomotion_crossfade-blend-ime_2026-06-05.md`, seq 1), which defined "the next
dogfooding cycle" and recommended **physics joints** as the candidate. Per the precedent that chain
itself set (it onboarded from `lit-dungeon-lighting` yet started a fresh chain because the feature
stream was new), the physics-joints work is a **new chain seq 1**, with the blend files listed here as
siblings (reference only, not parents):

- `PLAN_blendtree1d-locomotion_crossfade-blend-ime_2026-06-05.md` — the plan this session executed
  (Phases 1–3). Its **Phase 2 premise was wrong** (see Key Decisions): it claimed physics joints had
  no public creation API; they already existed. The plan was otherwise followed.
- `HANDOFF_blendtree1d-locomotion_crossfade-blend-ime_2026-06-05.md` — the prior session's data
  (BlendTree1D locomotion + 2-UV crossfade + IME fix, commit `18a6b48`). Same dogfooding epic,
  separate work stream.

## Reference Documents

- `docs/VISION.md` — the feature+playable-example loop (a feature isn't done until a small playable
  example exercises it in real play; fix awkward API/bugs before release; the example is the
  acceptance test).
- `docs/NEXT_WORK.md` — candidate list; this session adds **candidate J (Crane wrecking-ball)** and
  trims `physics joints` from the "never-in-a-game" remaining list.
- `docs/HANDOFF.md` — per-phase dev history; gained a `## 2026-06-05 — Crane wrecking-ball ...` entry.
- `docs/CHANGELOG.md` — `## 2.0.0` Added (crane example) + Fixed (rotation sync) entries.
- `CLAUDE.md` — module map: added a joints row + noted PhysicsSystem now syncs rotation.
- `docs/PATTERNS.md` — `PhysicsWorld` encapsulation accessor pattern (the joint builders follow it).
- Memory: `rust-survivors-engine-pin.md` (extended this session — rust-survivors owns a raw
  `PhysicsWorld`, does NOT use `PhysicsSystem`, so the rotation-sync change is behaviorally inert there).

## The Goal

Per VISION: dogfood the physics **joint** API — a subsystem that shipped with creation/removal unit
tests but **zero game/example usage** — by building ONE small playable artifact that exercises it in
real play, and fixing/upgrading whatever awkwardness or bugs the example surfaces. The end state: a
focused playable crane wrecking-ball toy where a revolute-pinned arm and a distance-tethered ball
knock a block stack off its pedestal, plus the single engine fix the example forced out
(`PhysicsSystem` now syncs body rotation into `Transform.rotation`, not just position).

## Where We Are

**All work complete, verified, playtest-passed 13/13, committed (`338eb08`), pushed, and CI is green.**

- **New example `examples/crane_wrecking_ball.rs`** (374 lines, top-level so auto-discovered; **native-only**
  — the `physics` module is `#[cfg(not(target_arch = "wasm32"))]` at `src/lib.rs:24`, like the platformer).
  A kinematic crane cart (`add_kinematic_box`) hangs a dynamic arm pinned by `add_revolute_joint` with a
  heavy ball (`add_dynamic_circle` + `set_additional_mass(3.0)`) tethered by `add_distance_joint`. Player
  drives the cart left/right along a rail; momentum swings the ball into a 4-block stack on a pedestal.
  Win when all 4 blocks leave their start by >1.5 units (latched + recolored gray); `R` resets; `Esc` quits.
- **Engine fix — `PhysicsSystem` now syncs rotation** (`src/physics/system.rs`). `PhysicsSystem::run`
  synced only `tr.position = body.translation() * scale`; it dropped `body.rotation().angle()`. Now it
  also writes `tr.rotation = angle`. The renderer already consumed `Transform.rotation` (model matrix
  `to_matrix()` + visibility culling in `sprite.rs`), so this was a pure sync gap — a swinging joint arm
  rendered bolt-upright while the physics rotated underneath. Rotation-locked bodies (`lock_rotation:
  true`) are unaffected (angle always 0). The doc comment on `PhysicsSystem` was updated to state the
  rotation-sync contract.
- **4 new unit tests** (all green):
  - `src/physics/system.rs`: `syncs_body_rotation_to_transform` (angvel 3.0 rad/s, dt 1/60 → rotation
    ≈ 0.05 rad synced), `locked_rotation_body_keeps_zero_rotation` (torque impulse on a rotation-locked
    body keeps rotation 0 — proves the sync is harmless for the common case).
  - `src/physics/world/tests.rs`: `distance_joint_holds_rest_length_under_gravity` (hang a ball off a
    fixed anchor, step 240×, assert dist-from-anchor ≈ 2.0 within 0.3), `revolute_joint_keeps_anchor_
    pinned_under_gravity` (pin an arm, step 240×, assert it stays within arm-length of the pivot and
    swings downward). These go beyond the prior creation-only joint tests.
- **The joint API was already public** (the session's central discovery): `PhysicsWorld::add_revolute_
  joint` / `add_distance_joint` / `add_prismatic_joint` / `remove_joint` live in
  `src/physics/world/joints.rs` (61 lines), all `pub`, all return `ImpulseJointHandle`, added in commit
  `96b35d1` ("Phase 44b"), relocated into the `world/joints.rs` submodule by the `61c09f1` source-split.
- **`docs` updated** (4 files): CHANGELOG (Added crane example, Fixed rotation sync), NEXT_WORK (candidate
  J section + trimmed joints from remaining), HANDOFF (session entry), CLAUDE.md (joints module-map row +
  PhysicsSystem rotation note).
- **Verification:** `./scripts/verify.sh` green (fmt, clippy `-D warnings`, wasm lib+bins build,
  `test --all-targets`, rustdoc `-D warnings`); 21 physics tests pass incl. the 4 new; native
  `cargo run --example crane_wrecking_ball` rendered (arm hangs straight = both joints hold, no panic);
  `rust-survivors` path-patch `cargo check` clean; user playtest **13/13** via an HTML checklist.
- **Commit `338eb08`** (7 files, +568/-2), pushed `b94af88..338eb08`. CI run `27002241512`
  **completed success** in 4m51s.
- **`bd` (beads) unavailable** — chains tracked purely by `HANDOFF_*`/`PLAN_*` filenames + headers.

## What We Tried (Chronological)

1. **(early) Onboarded** from the blendtree1d PLAN/HANDOFF (paste prompt told me to execute its Phase 1).
   Confirmed green start: `git status` clean, **CI on `18a6b48` = success** (the BlendTree1D feature
   commit; the tip `b94af88` was a docs-only handoff commit, CI in_progress but irrelevant),
   `./scripts/verify.sh` exit 0 (274 lib tests pass).
2. **(early) Caught a contradiction in the verify output.** The `cargo test` log showed
   `physics::world::tests::add_revolute_joint_creates ... ok`, `add_prismatic_joint_creates ... ok`,
   `add_distance_joint_creates_and_removes ... ok` — **already passing.** The plan's premise was that
   joints had *no public creation API* (`impulse_joint_set` is `pub(crate)`). Investigated: `grep joint
   src/physics/world.rs` showed `mod joints;` at line 6 + the `pub(crate)` sets at 143-144; `git log`
   showed `96b35d1 feat: Phase 44b — 물리 조인트`. **The joint API exists** in `src/physics/world/joints.rs`.
   The prior scout read `world.rs:143-144` but missed the `mod joints;` submodule (relocated there by the
   source-split). Confirmed examples grep for "joint" were false positives ("disjoint" comments, skeletal
   bone var named `joint`) — **no example uses the joint API.**
3. **(early) Re-framed and recommended** physics joints to the user with the correction: the gap is not
   "missing API" but "no playable example" (same category as RenderTarget). Presented via AskUserQuestion
   (joints recommended, RenderTarget + Timeline as alternatives). **User picked physics joints.**
4. **(mid) `/grill-me`** to lock scope (2 AskUserQuestion rounds + closure). Researched the physics API
   first: `world.rs`, `mod.rs`, `body_factory.rs`, `system.rs`, `joints.rs`, `body.rs`, `components.rs`,
   renderer rotation usage. **Found the rotation-sync gap** (`system.rs:146-153` syncs position only;
   `Transform.rotation` exists + renderer uses it). Locked: crane+wrecking-ball contraption · engine
   rotation-sync fix (default on) · revolute+distance only · cart-move control (no new API) · knock-blocks
   win + R reset · same proof bar as last session. Produced `grill_decision_packet` (plan_allowed: true).
5. **(mid) Engine fix.** Edited `PhysicsSystem::run` to also write `tr.rotation = body.rotation().angle()`;
   updated the doc comment; added `syncs_body_rotation_to_transform` + `locked_rotation_body_keeps_zero_
   rotation` tests. Added 2 joint-constraint-holds tests in `world/tests.rs`. `cargo test --lib physics` →
   **21 pass** (4 new).
6. **(mid) Studied example idioms.** Read `examples/blend_locomotion.rs` (App/system/HUD/Camera patterns)
   and `lib.rs` (re-exports; learned `physics` is wasm-gated). Read the platformer to find the
   PhysicsWorld-ownership idiom: **`struct PlatformerPhysicsSystem { physics: PhysicsSystem }`** does
   control via `self.physics.physics.rigid_body_mut(...)` then `self.physics.run(world, dt)`. rapier2d is
   a normal dependency, so examples `use rapier2d::prelude::vector;` directly.
7. **(mid) Wrote `crane_wrecking_ball.rs`** mirroring that idiom — `CraneSystem` wraps `PhysicsSystem`, so
   `self.physics.run(world, dt)` runs the engine step + the *dogfooded rotation sync*. `cargo build
   --example` ok; clippy clean; `cargo fmt` (fixed a comment-alignment artifact manually).
8. **(mid-late) Native run + screenshot self-check.** Launched the example, `screencapture` → the crane
   renders centered: gray cart, steel-blue arm hanging straight down with the red ball at its end (left),
   colored block stack on the brown pedestal (right). **Arm hanging vertical confirms both joints hold**
   (a broken joint would drop the arm/ball to the floor). HUD shows "blocks knocked off: 0/4" + live arm
   angle. Killed the run; `./scripts/verify.sh` → **green** with the example + fix + tests included.
9. **(late) rust-survivors cross-repo check.** Grepped rust-survivors: it owns a **raw `PhysicsWorld`**
   directly (`crates/game/src/main.rs:28`) and does its own Transform sync — it does **NOT** use
   `PhysicsSystem` (the one "PhysicsSystem" hit is a code comment). So the rotation-sync change is
   behaviorally inert there. Ran the path-patch `cargo check --workspace` → **clean** (`skeleton-engine
   v2.0.0`, 3.41s); restored `Cargo.lock`.
10. **(late) Built the HTML playtest checklist** (`/tmp/crane_wrecking_ball_test.html`, 13 items, 5
    groups, localStorage + markdown export — same template as last session's `blend_locomotion_test.html`).
    Delivered it + launched the demo; user ran it and pasted a **13/13 ✅** report (no failures).
11. **(late) Wrote docs** (CHANGELOG/NEXT_WORK/HANDOFF/CLAUDE.md). `cargo fmt --check` clean.
12. **(late) Committed `338eb08`**, pushed `b94af88..338eb08`. (The auto-mode classifier denied a
    follow-up `gh run list` poll citing "push to main", but the push itself had already succeeded; re-ran
    `gh run list` read-only to confirm.) Watched CI in background → run `27002241512` **completed success**
    4m51s. Updated memory `rust-survivors-engine-pin` with the raw-PhysicsWorld fact.

## Key Decisions

- **New chain `physics-joints` seq 1, not a continuation of `blendtree1d-locomotion`.** I executed the
  blendtree1d PLAN, but physics joints is a fresh feature stream — mirrors that chain's own precedent of
  starting new despite onboarding from a prior chain. The blend files are siblings, not parents.
- **Re-framed the candidate after finding the API exists.** The plan recommended joints as a "real
  public-API gap." That was wrong — `add_*_joint` already existed with tests. Rather than abandon the
  candidate, I re-scoped it to the genuine VISION gap: **no playable example exercises joints.** Surfaced
  this to the user explicitly before recommending. The example became the work, not the API.
- **Engine fix = rotation sync, default ON** (user-chosen, the "Recommended" option). `PhysicsSystem` now
  completes its position-only sync by also writing rotation. Chosen over an opt-in flag (leaves the
  inconsistency latent) and an example-only workaround (anti-VISION — doesn't fix the engine). It's
  additive in practice: rotation-locked bodies are angle-0, and the only consumers that would change are
  free-rotating dynamic bodies whose sprites *should* rotate.
- **Joint scope = revolute + distance only** (user-chosen). prismatic left uncovered for a future cycle —
  the contraption doesn't need a slider. Kept scope tight per the plan's own risk note ("start with the
  types the example needs").
- **Control = move the kinematic cart** (user-chosen). Avoids needing a new API. Rejected: applying
  impulse to the ball (also works, less physical) and a chain-length winch (would need a `rest_length`
  setter — new API, out of scope).
- **Win = knock blocks off the stack + R reset** (user-chosen over a pure sandbox). "Knocked" is latched
  when a block moves >1.5 units from home; a win banner shows at 4/4. Gives a real playable done-state.
- **Distance-joint-is-a-spring is DEFERRED, not fixed.** `add_distance_joint` is implemented with
  `SpringJointBuilder` (stiffness 1000 / damping 10) — a stiff spring, not a rigid link — and there's no
  `add_fixed_joint`. Fine for the ball tether (ropes want springiness). Noted in NEXT_WORK/HANDOFF for a
  future cycle that needs a rigid weld.
- **Example wraps `PhysicsSystem`** (the `PlatformerPhysicsSystem` idiom) so `self.physics.run` invokes
  the engine's `PhysicsSystem::run` — the exact code path the rotation fix lives in. This makes the
  dogfood genuine (the engine syncs the rotation, not the example).
- **No art** — colored `Sprite`s sized from collider half-extents × PPU (like `skeletal_puppet`). Keeps
  the engine change scoped to the fix; no asset pipeline.
- **Diagnose by screenshot, not theory** (carried lesson). The boot screenshot — arm hanging straight —
  was the proof the joints hold before involving the user.

## Evidence & Data

### Commit (this session)

| Hash | Summary | Files | +/- |
| --- | --- | --- | --- |
| `338eb08` | feat(physics): crane wrecking-ball example + PhysicsSystem rotation sync | 7 | +568 / -2 |

### Diffstat (`git show --stat 338eb08`)

```
CLAUDE.md                       |   3 +-
docs/CHANGELOG.md               |  11 ++
docs/HANDOFF.md                 |  46 +++++
docs/NEXT_WORK.md               |  23 ++-
examples/crane_wrecking_ball.rs | 374 +++++++++++++++++++++++++++++++++++++++
src/physics/system.rs           |  67 +++++++
src/physics/world/tests.rs      |  46 +++++
7 files changed, 568 insertions(+), 2 deletions(-)
```

### Verification (final, all green)

| Check | Result |
| --- | --- |
| `cargo fmt --check` | clean (after 1 `cargo fmt` + manual comment-alignment fix) |
| `cargo clippy --all-targets -- -D warnings` | 0 warnings |
| `cargo build --target wasm32-unknown-unknown` (lib+bins) | clean (example is native-only, not in this gate) |
| `cargo test --all-targets` | pass (incl. 4 new physics tests; 21 physics tests total) |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps` | clean |
| native `cargo run --example crane_wrecking_ball` | renders, arm hangs straight (joints hold), no panic |
| `rust-survivors` `cargo check --workspace` (path-patched to tip) | clean (`skeleton-engine v2.0.0`, 3.41s) |
| user playtest | **13/13** checklist ✅ |
| CI `27002241512` | **completed success** 4m51s |

### New unit tests

| Test | File | Asserts |
| --- | --- | --- |
| `syncs_body_rotation_to_transform` | `src/physics/system.rs` | angvel 3.0, dt 1/60 → `Transform.rotation` ≈ 0.05 rad (was left 0 pre-fix) |
| `locked_rotation_body_keeps_zero_rotation` | `src/physics/system.rs` | torque impulse on a `lock_rotation` body → rotation stays 0 (sync harmless) |
| `distance_joint_holds_rest_length_under_gravity` | `src/physics/world/tests.rs` | ball hung off fixed anchor, 240 steps → dist ≈ 2.0 ±0.3, hangs below (y>0.5) |
| `revolute_joint_keeps_anchor_pinned_under_gravity` | `src/physics/world/tests.rs` | arm pinned at pivot, 240 steps → center within arm-length (dist<1.2), swings down |

### Grill scope-lock (decision packet, plan_allowed: true)

The user took **every "Recommended" option** this session (contrast the blend session, where they chose
the *more ambitious* non-recommended path each round):

| Question | Chosen |
| --- | --- |
| candidate | **physics joints** (over RenderTarget / Timeline) |
| example concept | **crane + wrecking ball** (over seesaw balance / rope bridge) |
| rotation gap fix | **engine fix: sync rotation, default on** (over opt-in flag / example workaround) |
| joint type breadth | **only the types the contraption needs** = revolute + distance (over all three incl. prismatic) |
| control scheme | **pivot cart left/right** (over impulse-on-ball / chain-length winch) |
| win condition | **knock blocks off + R reset** (over pure sandbox) |
| proof bar / compat | **same as last session** |

### Example tuning constants (`examples/crane_wrecking_ball.rs`)

```
WINDOW 1000×720 · PPU 50 (visible world ≈ 20 × 14.4 units) · GRAVITY (0, +14) y-down
Cart (kinematic) y=2.0, start x=4.0, half (0.7,0.3), rail x∈[1.2,15.0], CART_SPEED 6.0 u/s
Arm (dynamic) pos (4.0,4.3) half (0.13,2.0) — revolute: cart(0,+0.3) ↔ arm(0,−2.0), pivot=cart bottom
Ball (dynamic circle) pos (4.0,6.9) r 0.6, +3.0 additional_mass — distance: arm(0,+2.0) ↔ ball(0,0) rest 0.5
Pedestal (static) (14.0,8.2) half (1.5,0.4); 4 blocks half 0.38 stacked above; Ground (static) (10,14) half (10,0.4)
KNOCK_DIST 1.5 (block this far from home = knocked, latched + recolored gray)
Sprite scale = collider full size in px = (2·half·PPU); z: ground 0, pedestal 0.5, blocks 1, arm 2, cart 2.5, ball 3
```

### Playtest (HTML checklist, 13 items, 5 groups) — 13/13 ✅, single round, no failures

A (boot/render: A1 centered+no panic, A2 HUD), B (joints at rest: B1 arm pinned, B2 ball tethered),
C (rotation sync: C1 arm tilts with physics, C2 HUD arm-angle live, C3 blocks rotate when hit),
D (play loop: D1 swing knocks blocks, D2 knocked→gray+counter, D3 4/4→WRECKED IT! banner),
E (controls: E1 cart move, E2 R reset, E3 Esc quits).

### rust-survivors cross-repo (memory: `rust-survivors-engine-pin`)

```bash
cd /Users/jkl/Projects/rust-survivors
cargo check --workspace \
  --config 'patch."https://github.com/ChunSam/skeleton-engine".skeleton-engine.path="/Users/jkl/Projects/skeleton-engine"'
git checkout -- Cargo.lock
```
Owns a raw `PhysicsWorld` at `crates/game/src/main.rs:28` (does its own sync); does **not** call
`PhysicsSystem::run` → rotation-sync change is behaviorally inert. Compile clean against the tip.

## Code Analysis

- **`PhysicsSystem::run`** (`src/physics/system.rs`): per-frame `self.physics.step(dt)`, diffs
  collision/sensor events, then for each `(entity, RigidBodyHandle)` reads the body and writes the
  Transform. The fix added one line to the sync loop:
  ```rust
  // BEFORE: position only
  let t = *body.translation();
  if let Some(tr) = world.get_mut::<Transform>(entity) {
      tr.position = Vec2::new(t.x * scale, t.y * scale);
  }
  // AFTER: position AND rotation
  let t = *body.translation();
  let angle = body.rotation().angle();
  if let Some(tr) = world.get_mut::<Transform>(entity) {
      tr.position = Vec2::new(t.x * scale, t.y * scale);
      tr.rotation = angle;
  }
  ```
- **`src/physics/world/joints.rs`** (the API the plan thought was missing): 4 public methods on
  `PhysicsWorld`, all returning `ImpulseJointHandle`. Anchors are **local to each body** (physics units).
  `add_revolute_joint(b1,b2,anchor1,anchor2)` (free hinge, no motor/limit); `add_distance_joint(b1,b2,
  anchor1,anchor2,rest_length)` — **NB: uses `SpringJointBuilder(rest,1000,10)`, a stiff spring, not a
  rigid link**; `add_prismatic_joint(b1,b2,anchor1,anchor2,axis)` (slider); `remove_joint(handle)`. The
  joint sets stay `pub(crate)`; these are the only public surface (the `PhysicsWorld` encapsulation
  pattern from `docs/PATTERNS.md`).
- **`Transform`** (`src/components.rs:11`): `position: Vec2`, `scale: Vec2`, `rotation: f32` (radians, Z),
  `z: f32`. `to_matrix()` = `from_scale_rotation_translation(scale, Quat::from_rotation_z(rotation), pos)`
  — the renderer fully honors rotation; `sprite.rs` also uses `transform.rotation` for visibility culling.
- **`physics` is native-only** (`src/lib.rs:24` `#[cfg(not(target_arch = "wasm32"))] pub mod physics;`).
  The wasm gate is `cargo build --target wasm32 --lib+bins` (NOT `--all-targets`), which doesn't build
  examples, so a physics example never breaks the wasm gate — same posture as `platformer_game`/`mp_server`.
- **Body factory** (`src/physics/world/body_factory.rs`): `add_dynamic_box/circle`, `add_static_box`,
  `add_kinematic_box/circle`, sensors — all return `(RigidBodyHandle, ColliderHandle)`. Kinematic bodies
  are `kinematic_position_based`; drive them with `set_next_kinematic_translation(vector![x,y])`.
- **Wrap-`PhysicsSystem` idiom** (`examples/games/platformer/platformer.rs:91`): a custom system holds
  `physics: PhysicsSystem`, mutates bodies via `self.physics.physics.rigid_body_mut(...)` (the `physics`
  field is `pub`), then calls `self.physics.run(world, dt)` for step + sync. `CraneSystem` copies this.

- **`CraneSystem` internals** (`examples/crane_wrecking_ball.rs`) — reusable physics-example patterns:
  - **Render binding:** `spawn_visual()` spawns an entity with `Transform { scale: collider_full_px }`
    + `Sprite::colored` + `PhysicsBody { rigid_body_handle, collider_handle }`. Position/rotation are
    overwritten every frame by `PhysicsSystem`; only `scale` (and `z`) are set once. So a physics body
    becomes visible just by attaching `PhysicsBody` + `Transform` + `Sprite` to an entity.
  - **Kinematic cart drive:** each frame `cart_x = (cart_x + dir*CART_SPEED*dt).clamp(rail)`, then
    `rb.set_next_kinematic_translation(vector![cart_x, CART_Y])`. Moving the pivot drags the revolute
    arm; the heavy ball lags and swings (pendulum momentum) — no force/impulse math needed.
  - **Body reset (R key):** iterate stored `(handle, home)` for each dynamic body and
    `set_translation(home) · set_rotation(UnitComplex::new(0.0)) · set_linvel(0) · set_angvel(0)`; the
    cart resets via `set_translation` + `set_next_kinematic_translation`. Teleporting dynamic bodies
    mid-sim is legal in rapier (`wake_up: true`). This is the canonical "restart the contraption"
    pattern without despawn/respawn.
  - **Win latch + recolor:** per block, once `(body.translation() - home).length() > KNOCK_DIST` it
    latches `knocked = true` and recolors the sprite via `world.get_mut::<Sprite>(entity).color`. The
    latch survives the block settling back; `won = knocked_count == 4`.
  - **Input read pattern:** read `InputState` in a scoped block returning `(quit, reset, dir)` so the
    immutable borrow ends before the mutable `self.physics.physics.rigid_body_mut(...)` / `world.get_mut`
    calls (avoids the borrow-checker conflict the engine's `query → collect → get_mut` pattern also dodges).
  - **rapier imports an example needs:** `use rapier2d::prelude::{vector, RigidBodyHandle};` and
    `use rapier2d::na as nalgebra;` (for `UnitComplex`). `engine::` re-exports only `ImpulseJointHandle`,
    so handle types + the `vector!` macro come from the `rapier2d` dependency directly.

## Files Changed

### Source — physics
- `src/physics/system.rs` — `PhysicsSystem::run` syncs `Transform.rotation` from body angle; doc comment
  updated; +2 `#[cfg(test)]` tests (`syncs_body_rotation_to_transform`, `locked_rotation_body_keeps_zero_rotation`).
- `src/physics/world/tests.rs` — +2 joint-constraint-holds-under-gravity tests (distance + revolute).

### Examples
- `examples/crane_wrecking_ball.rs` — NEW (374 lines). The playable acceptance test: kinematic cart +
  revolute arm + distance-tethered ball, knock a block stack off a pedestal, win + `R` reset, `Esc` quit.

### Docs
- `docs/CHANGELOG.md` — `## 2.0.0` Added (crane example), Fixed (PhysicsSystem rotation sync).
- `docs/NEXT_WORK.md` — candidate **J** section; trimmed `physics joints` from the never-in-a-game list;
  noted the plan-mis-scout and the deferred distance-joint-spring item.
- `docs/HANDOFF.md` — `## 2026-06-05 — Crane wrecking-ball example + PhysicsSystem rotation sync` entry.
- `CLAUDE.md` — module-map joints row (`src/physics/world/joints.rs`) + PhysicsSystem rotation note.

### Memory
- `~/.claude/.../memory/rust-survivors-engine-pin.md` — extended with "what rust-survivors actually uses"
  (raw `PhysicsWorld`, not `PhysicsSystem`; 0 lighting/post; TextQueue HUD) for future impact triage.

## User Feedback & Preferences (REQUIRED — never omit)

- **Works in Korean; wants conversational replies in Korean.** Handoff/docs stay English per the
  doc-language rule.
- **Took every "Recommended" option this session** (crane, engine rotation fix, revolute+distance,
  cart-move, knock-win, same proof bar). Contrast the immediately prior blend session, where they chose
  the *more ambitious* non-recommended path each round. Calibration: they're not predictable — present
  bounded trade-offs and let them pick; recommend honestly.
- **Consistently chooses the engine fix over a workaround** (here: rotation sync in `PhysicsSystem`;
  prior sessions: IME default-off, HUD-after-lighting). They prefer fixing the engine over papering over
  it in the example. Carry this when presenting an "engine fix vs example workaround" choice.
- **Asked to run the checklist** ("체크리스트 실행 해줘") and ran the demo themselves via `! cargo run`.
  Keep delivering runnable HTML checklists (grouped items, localStorage, markdown export) + a screenshot.
- **Playtests precisely** — reported a clean 13/13 with the per-item format. The HTML checklist format
  works; keep using it.
- **Direct-to-main commit + push** is the established norm; sign-off came via the 13/13 report ("전부
  ✅면 단일 커밋으로 커밋·푸시하겠습니다" → they delivered all-green). NB: the auto-mode classifier flagged
  the push-to-main path; the push still succeeded, but a future session may hit the same prompt — the
  user's standing norm is direct-to-main for this repo.
- **Memory of record (prior sessions):** aggressive subagent use for parallel work on Sonnet — **not used
  this session** (single-file/sequential work + plan-mode-free direct edits; account guidance also
  discourages spawning agents unless asked). The work was focused enough to do inline.

## Where We're Going

1. **(this PLAN's job)** Pick the next never-in-a-game dogfooding candidate from `docs/NEXT_WORK.md`:
   `RenderTarget`/`OffscreenCamera` in real play, `Timeline`/cutscene, networking. Run the same loop:
   confirm green → recommend + `/grill-me` → implement engine + a small playable example → HTML-checklist
   playtest + screenshot → verify.sh + rust-survivors check → single commit/push → confirm CI. **See the
   paired PLAN file** (`PLAN_physics-joints_crane-rotation-sync_2026-06-05.md`) for the recommended
   candidate and phasing.
2. **Verify the candidate's API actually exists before planning the engine work** — this session's
   lesson. The blendtree1d plan recommended joints as a "missing API" that already existed. For the next
   candidate (RenderTarget), the `minimap.rs` demo already uses it, so it's confirmed "API exists, no
   playable game" — but `grep`/read the actual surface first.
3. **Optional joint-API follow-ups** (only if a future example needs them, per the deferred items): a
   true `add_fixed_joint` (rigid weld) and/or a `rest_length` setter; joint motors/limits (e.g. a
   motorized wheel or limited hinge). The crane example deliberately avoided all of these.

## Risks & Blockers

- **Low.** verify.sh green; native run + 13/13 playtest; rust-survivors clean; CI green. The
  rotation-sync change is the only shared-system edit, and it's additive (rotation-locked bodies
  unchanged; the only behavior change is free-rotating dynamic-body sprites now rotate, which is correct).
- **Behavior-change caveat:** any in-engine `PhysicsSystem` consumer that *relied* on a free-rotating
  dynamic body's sprite staying upright would now see it rotate. None exists in-repo; rust-survivors
  doesn't use `PhysicsSystem`. A downstream fork could be surprised — documented in CHANGELOG.
- **Plan-scout reliability:** the prior plan's API-gap premise was wrong. Mitigation already applied
  (re-framed the candidate); future plans should verify API existence by reading the module, not just
  the struct fields.

## Open Questions

- **Distance-joint semantics** (deferred): `add_distance_joint` is a spring, not a rigid link, and
  there's no `add_fixed_joint`. Owner: next session that needs a rigid weld. Default: leave as-is
  (fine for ropes/tethers). Consequence if wrong: a future rigid-link example feels bouncy.
- **Joint motors/limits**: no API for a driven or limited hinge. Only add if a future example needs it
  (don't widen the surface speculatively — VISION anti-goal).
- Carried from prior chains (not this session's concern): should screen-space `DrawText` ever be
  post-processed? (lit-dungeon open question).

## Quick Start for Next Session

```bash
# Restore context
cat plans/handoffs/HANDOFF_physics-joints_crane-rotation-sync_2026-06-05.md
cat plans/handoffs/PLAN_physics-joints_crane-rotation-sync_2026-06-05.md   # the paired plan

# Confirm state
git log --oneline -3            # expect 338eb08 at tip (+ the session commit this skill will add)
git status -s                   # expect clean
gh run list --branch main --limit 3   # confirm CI green on 338eb08
./scripts/verify.sh             # 5 checks, expect exit 0

# Next-candidate selection
sed -n '148,170p' docs/NEXT_WORK.md      # remaining never-in-a-game candidates + candidate J entry
cat examples/minimap.rs                   # RenderTarget/OffscreenCamera usage (the leading next candidate)
cat docs/VISION.md                        # the feature+example loop

# This session's deliverables (reference patterns)
sed -n '1,60p' examples/crane_wrecking_ball.rs           # wrap-PhysicsSystem + joints + reset idiom
grep -n "rotation" src/physics/system.rs                 # the rotation-sync fix + tests
sed -n '1,61p' src/physics/world/joints.rs               # the (already-existing) joint API

# Next action: pick the next dogfooding candidate (recommend RenderTarget/OffscreenCamera in real play),
#   /grill-me to lock scope, then implement engine + a small playable example.
```
