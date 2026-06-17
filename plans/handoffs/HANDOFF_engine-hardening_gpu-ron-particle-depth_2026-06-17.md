# Particle depth completed across the GPU + RON paths (v0.18.0) + seq-19 verification-record fix

**Date:** 2026-06-17
**Status:** **Shipped + merged, CI-green.** `main` @ `93205ad`, package **v0.18.0**, clean tree.
PR **#104** squash-merged (merge authority re-confirmed this session). 786 → **789 lib tests**.
**Chain:** `engine-hardening` seq `20` · **Parent:** seq 19 (`HANDOFF_engine-hardening_phase7-roadmap-complete_2026-06-17.md`)
**Prior:** … → 18 (P6) → 19 (P7; UX roadmap 7/7 complete) → **20 (this: particle-depth follow-up closed + record fix)**

---

## Outcome

Two deliverables this session:

1. **Doc-record fix (the user's first ask).** The seq-19 handoff still flagged its Phase 4–7 demos as
   "Outstanding verification (locked-machine debt)", but those demos **were** live-verified in a later
   session that shipped **no handoff of its own** — so the record lagged while the `engine-current-state`
   memory already said "ALL PHASES LIVE-VERIFIED". The same memory even held a contradictory leftover
   "MACHINE LOCKED" sentence. Reconciled both: added a **✅ RESOLUTION banner** + "RESOLVED" marker to the
   seq-19 handoff (kept the as-written section verbatim), and cleaned the memory's stale sentence + stale
   version token. This shipped as the docs commit inside PR #104.

2. **v0.18.0 — particle depth completed on the GPU + RON fronts (the Phase-6 follow-up).** Phase 6
   (v0.16.0) added `gravity` + `emit_shape` to the **CPU** `ParticleEmitter` only. This mirrors them onto
   the compute-shader **`GpuParticleEmitter`** and the data-driven **RON `ParticleConfigSet`**. Purely
   additive — zero gravity / `Point` shape / omitted RON fields reproduce prior behavior byte-for-byte.

| Front | Change | Files |
|---|---|---|
| GPU emitter | `gravity` + `emit_shape` fields + `with_*` builders; CPU-samples shape at emit, writes per-particle gravity | `src/gpu_particle.rs` |
| GPU buffer | `GpuParticle` 64 → 80 bytes (`gravity: vec2` @offset 64 + pad) | `src/renderer/gpu_particle.rs` |
| GPU shaders | compute integrates `p.vel += p.gravity*dt`; both WGSL `Particle` structs grow to match stride | `src/renderer/shaders/gpu_particle_{compute,render}.wgsl` |
| RON config | `EmitterDef` gains optional `gravity` + `emit_shape` via `EmitShapeDef` serde mirror; `emitter()` wires them | `src/particle/config_set.rs` |
| Examples | `gpu_particles` (arc + Circle scatter); `data_particles/particles.ron` (gravity on fire, Circle on fountain) | `examples/…` |

---

## Phase-6 baseline (what the next reader must know we mirrored)

Before this session the CPU `ParticleEmitter` (`src/particle/mod.rs`, v0.16.0) already had:
`pub gravity: Vec2` (default `ZERO`, integrated in `ParticleSystem` as `vel += gravity*dt; pos += vel*dt`,
velocity written back to the `Particle`), `pub emit_shape: EmitShape` (default `Point`), and builders
`with_gravity`/`with_emit_shape`. `EmitShape` is `#[non_exhaustive]`-free, `Copy`,
`{ Point, Circle{radius}, Ring{radius}, Box{half_extents:Vec2} }` with a `pub(crate) sample_offset(rng)
-> Vec2` (uniform-area disc for `Circle`, hollow for `Ring`, box for `Box`) — already unit-tested
(`emit_shape_samples_stay_in_bounds`). **This session changed none of that** — it reused `sample_offset`
and copied the same semantics onto the GPU + RON sides.

## Verification-record fix — exact edits (the user's first ask)

The user said the seq-19 verification debt was resolved in an unrecorded session; "fix it." Edits made:
- `plans/handoffs/HANDOFF_…phase7-roadmap-complete_2026-06-17.md`: (1) RESOLUTION banner under the chain
  line; (2) "⚠️ Outstanding verification … — ✅ RESOLVED" header + reworded intro (kept the as-written list
  verbatim); (3) Follow-ups "Hear/eyeball the 4 demos" struck through → DONE + GPU-particle-mirror promoted
  to top follow-up; (4) Quick-Start "next step" reworded from "VERIFY demos" to "follow-up".
- `engine-current-state` memory: replaced the contradictory leftover "⚠️ MACHINE LOCKED mid-session …"
  sentence with a "superseded — debt CLEARED, do not re-flag" note; fixed the frontmatter `description`
  version `v0.11.0`/`v0.17.0` → current.
- **Corroboration:** the memory already held "✅ ALL PHASES LIVE-VERIFIED (2026-06-17 …): … Phase 5
  browser localStorage reload 1→7 persists; Phase 7 WebAudio beep HEARD in a browser …" — so this was a
  reconciliation, not a fabrication. The verification session had used a throwaway `tmp/browser-test`
  branch (temp `#[wasm_bindgen]` exports + generated-WAV beep), discarded; `main` was untouched.

## What We Did (engineering detail)

### GPU half (opus-by-hand — delicate render path)

- **Per-particle gravity, not a global uniform.** The CPU `Particle` carries `gravity` copied from its
  emitter at spawn, so multiple emitters can have different gravity. The faithful mirror is a per-particle
  field on the GPU struct, NOT a single `ComputeUniforms.gravity` (which couldn't represent per-emitter
  gravity across the shared 4096-slot buffer). So `GpuParticle` grew a `gravity: [f32;2]`.
- **`GpuParticle` 64 → 80 bytes, std430-correct.** Layout (offsets): pos@0, vel@8, life@16, max_life@20,
  size@24, _pad@28, color_start(vec4)@32, color_end(vec4)@48, **gravity(vec2)@64, _pad2(vec2)@72** → size
  80. Struct alignment = 16 (vec4) → array **stride rounds to 80** (a multiple of 16). Rust `repr(C)`
  packs to the identical 80 with the same offsets. Verified the std430 math by hand because **`cargo test`
  cannot catch a stride mismatch** — naga validates WGSL only at shader-module creation (runtime), and CI
  has no GPU. A wrong stride → particles read garbage pos/color.
- **emit_shape is CPU-sampled, no shader change.** `collect_new_particles` already runs on the CPU
  (it builds `GpuParticle` structs then uploads them), and `EmitShape::sample_offset(&mut rng)` is
  `pub(crate)` — so the spawn offset is computed CPU-side (`pos + emitter.emit_shape.sample_offset(rng)`).
  Only **gravity** needed the shader (`p.vel += p.gravity * uniforms.dt;` before the existing
  `p.pos += p.vel * dt`). Both WGSL `Particle` structs (compute AND render) gained the two trailing fields
  so the buffer stride matches in every binding — the render shader ignores them but must agree on stride.

### RON half (Sonnet background subagent — mechanical serde, file-isolated)

- Delegated to a background Sonnet agent (model pinned per [[new-model-subagent-incompat]]) touching ONLY
  `src/particle/config_set.rs` + `particles.ron`, while opus did the GPU half concurrently — disjoint files,
  no coordination needed. Returned green (its own fmt/clippy/test/doc gates).
- Pattern: the file uses **private serde-mirror types** (`Vec2Def`, `ColorDef`) instead of deriving serde
  on glam/engine types. The agent followed it: added `Default` to `Vec2Def` (→ ZERO), a private
  `EmitShapeDef` enum (`Point`/`Circle{radius}`/`Ring{radius}`/`Box{half_extents:Vec2Def}`) + `From` for
  `EmitShape`, two `#[serde(default)]` fields on `EmitterDef`, and wired `def.gravity.into()` /
  `def.emit_shape.into()` into `emitter()`. RON enum syntax: `emit_shape: Circle(radius: 10.0)`,
  `Box(half_extents: (10.0, 4.0))`. +4 unit tests (gravity parse, Circle, Box, defaults-on-omit).

---

## Evidence & Data

- **Merged diff (`93205ad`, #104):** 12 files, +204/−29. Heaviest: `config_set.rs` (+109), `gpu_particle.rs`
  (+32), `CHANGELOG.md` (+25), seq-19 handoff (+28/doc-fix).
- **`./scripts/verify.sh` green** (run twice — once pre-paperwork, once post-version-bump): fmt --check,
  clippy --all-targets -D warnings, wasm build (lib+bins), **wasm clippy --lib -D warnings**, `cargo test
  --all-targets` (**789 lib** tests), `cargo test --doc` (64 pass / 33 ignored), `RUSTDOCFLAGS=-D warnings
  cargo doc`. EXIT=0 captured to `/tmp/verify_*.log` (no `| tail` masking — see gotchas).
- **CI on PR #104 (run 27691038494):** Build (WASM) pass 1m0s · Rustdoc pass 52s · Package dry-run pass
  1m33s · **Test (native) pass 4m5s** · `WATCH_EXIT=0`. Then squash-merge + `--delete-branch` + prune.
- **Subagent:** 40.9k tokens, 14 tool-uses, ~122s; reported `cargo test --lib particle` 21 pass / `--doc`
  3 pass / clippy clean.
- **RON tests added (4):** `gravity_field_parses_correctly` (`gravity:(0.0,200.0)` → `Vec2(0,200)`),
  `emit_shape_circle_parses_correctly` (`Circle(radius:12.0)`), `emit_shape_box_parses_correctly`
  (`Box(half_extents:(10.0,4.0))`), + extended `minimal_ron_uses_serde_defaults` to assert
  `gravity==ZERO` & `emit_shape==Point` on omission.
- **RON acceptance (`data_particles/particles.ron`):** `"fire"` got `gravity:(0.0,180.0)` (sparks arc +
  fall), `"fountain"` got `emit_shape:Circle(radius:10.0)` (disc spawn) — both hot-reloadable in the
  running `data_particles` example.
- **Release paperwork (`/ship`, 4 files in sync):** `Cargo.toml` 0.17.0→0.18.0; `Cargo.lock` via
  `cargo update -p skeleton-engine`; `docs/CHANGELOG.md` new `## 0.18.0` (Added/Changed); `CLAUDE.md`
  header `v1.6.65→v1.6.66` + package `v0.17.0→v0.18.0` + the particle module-map row.

### Verification — the GPU playtest saga (read this; it explains the harness limits)

The GPU compute path is native-only with no headless harness, so it was eyeballed on a real window:

1. **Synthetic mouse click → winit does NOT receive it.** `osascript … System Events click at {x,y}`
   moved/raised the window (so AX permission exists) but the real `gpu_particles` example stayed at
   "Emitters: 0" — clicks never reached the winit window. (HUD text DID render → the render pipeline is
   fine post-struct-change.)
2. **Switched to an auto-spawn temp harness** (`examples/_tmp_gpu_playtest.rs`, since deleted) that spawns
   one emitter at startup — no input needed.
3. **Coordinate-system gotcha:** first auto-spawn was empty because the emitter sat at world `(0,0)`. The
   **default Camera has `position = ZERO` and world (0,0) = screen TOP-LEFT, Y down** (`src/camera.rs:8`).
   With upward velocity the particles flew off the top edge. Moving the emitter to ~`(640, 520)` (screen
   center-ish) made them visible.
4. **Particles render correctly but SPARSE (~8, not the expected ~600).** Root cause: a **GUI app launched
   bare from a shell never properly activates**, so its `ControlFlow::WaitUntil` run loop (`window.rs:493`)
   is pumped only sporadically (mostly on the osascript focus events) — few frames run, few particles
   accumulate, and the compute barely dispatches. This is a **harness artifact, not a bug** (the app DOES
   use frame-paced redraws when active). `color_end` being transparent also faded the risen particles,
   hiding the arc on the first try.
5. **Decisive confirmation via a double-capture** 1.5s apart: the particle cluster **moved and spread**
   between A and B → the compute shader IS integrating velocity + gravity. Combined with: particles render
   at the correct position/color (⇒ 80-byte stride correct) and scatter over a disc (⇒ `Circle` emit-shape
   works). **All three GPU concerns confirmed.**

---

## Key Decisions

- **Parallelized the two halves** (opus GPU + background Sonnet RON) because they touch disjoint files and
  the RON half is mechanical serde; opus kept the delicate shader/byte-layout work (a subagent on
  std430 alignment is high-risk — cf. nine-slice being opus-by-hand in seq 11).
- **Per-particle gravity field** over a global compute uniform (see engineering detail) — faithful to the
  CPU semantics; cost is 16 B/particle (4096 slots ≈ 320 KB, negligible, native-only).
- **Did NOT touch the native `AudioManager` or CPU particle paths** — additive, low-blast-radius.
- **Record-fix shipped inside the feature PR** as a separate clearly-labeled docs commit (`977756f`),
  rather than a standalone PR — both are this-session docs; one PR, two commits.
- **v0.18.0 = MINOR** (additive feature, pre-1.0 cadence: MINOR = any release). Not a tag yet (tagging is
  an explicit outward act; `v0.11.1`–`v0.18.0` exist as CHANGELOG + commits only).
- **Asked before the outward step** (PR + merge) — merge authority is per-session; the user re-confirmed
  "branch + PR + merge on green" this session.

---

## Reusable Gotchas (carry forward)

- **Default Camera: world `(0,0)` = screen TOP-LEFT, Y down**, `position = ZERO`, visible `X:[0,w] Y:[0,h]`
  (`src/camera.rs:8`). Place playtest entities near `(w/2, h/2)`, not the origin.
- **Bare shell-launched GUI apps step their run loop sparsely** (not truly "active" → `WaitUntil` rarely
  fires). A static screencapture shows few/frozen particles even when the code is correct. To prove motion,
  **double-capture ~1.5s apart and diff** rather than counting particles in one shot.
- **`osascript … System Events "click at"` does NOT reach a winit window** (it can move/raise it, but click
  events don't land). For click-driven examples, **auto-spawn via a throwaway temp example** instead. (No
  `cliclick` installed.)
- **`cargo test` cannot validate a GPU struct stride / WGSL layout** — naga validates only at shader-module
  creation (runtime, no GPU in CI). Hand-verify std430 offsets/stride when growing a `Pod` GPU struct, and
  keep **every** WGSL binding's struct (compute AND render) in sync with the Rust `repr(C)`.
- **External crates can't struct-literal a type with a `pub(crate)` field (E0451)** — `GpuParticleEmitter`
  has private `timer`/`next_slot`, so examples build via `Default::default()` + field assignment.
- `color_end` alpha 0 fades particles to invisible over life — fine for VFX, but use an **opaque** end color
  when you want to *see* a full trajectory in a screenshot.
- **Never `| tail` a gate/CI watch** (pipefail masks exit). Capture `verify.sh > log 2>&1; echo EXIT=$?`
  and `gh pr checks <n> --watch --fail-fast > log 2>&1` then read `WATCH_EXIT`. Poll until checks register
  before `--watch` (else instant false-0).
- **main is branch-protected** (Build WASM / Test native / Rustdoc / Package dry-run required; enforce_admins)
  → PR-only, squash-merge. Merge authority **re-confirm each session**.
- rust-analyzer phantoms (unlinked-file on `examples/`, inactive-code for `cfg(wasm32)`) are stale — trust
  cargo/CI. (~30 fired this session from a new temp example; all noise.)

---

## Follow-ups (open, none blocking)

- **DialogueBox localization + branching choices** — the recommended next epic; see the paired
  `PLAN_engine-hardening_gpu-ron-particle-depth_2026-06-17.md`.
- **GPU emitter via the RON path** — `ParticleConfigSet::emitter()` builds a CPU `ParticleEmitter` only;
  there is no RON→`GpuParticleEmitter` builder yet (the two new RON fields feed CPU emitters).
- **`gravity`/`emit_shape` in WGSL/`GpuParticleEmitter` RON configs** is the only remaining particle gap.
- wasm AEAD `save`/`load` (currently `Unsupported`); fuller wasm audio (kira); crates.io publish
  (deferred, fork-first, irreversible — explicit go needed); optionally tag `v0.11.1`–`v0.18.0`.

---

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 93205ad feat(particle) … (#104)
grep -m1 '^version' Cargo.toml  # 0.18.0
./scripts/verify.sh             # green (789 lib tests); RUN AS-IS, no tail pipe
# This session is DONE + merged. The paired PLAN targets DialogueBox depth (localization + branching).
# Read: this handoff (seq 20) + PLAN_engine-hardening_gpu-ron-particle-depth_2026-06-17.md.
# Particle/GPU subsystem: src/gpu_particle.rs, src/renderer/gpu_particle.rs, src/particle/config_set.rs.
```
