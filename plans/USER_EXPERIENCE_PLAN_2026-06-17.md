# User-Experience / Satisfaction Roadmap (phased)

**Date:** 2026-06-17
**Status:** PROPOSED — phased plan, awaiting go on Phase 1.
**Author intent:** raise satisfaction for *other people who use / fork* `skeleton-engine`.
**Why phased:** doing all of this as one "goal" would overflow a single session's context.
Each phase below is a **self-contained, session-sized batch** that ends with the verify
gate green and ships as its own PR (main is branch-protected, PR-only). The next session
can read just this file + the relevant phase and continue.

## Framing (from a 3-agent investigation of the codebase)

Breadth is **not** the problem — the engine already rivals a tier-2 2D engine in feature
breadth. Satisfaction is gated by three things, in priority order:

1. **First-hour friction** — new forkers bounce before they ever see the features.
2. **Thin "juice" layer** — what makes games *feel* good (hit-stop, easing, fades, shake).
3. **A few API papercuts** a forker hits daily.

A recurring finding: the engine **violates its own acceptance rule** ("a feature is not
done until a small playable example exercises it") — `FadeTransition`, camera shake, and
post-process all exist but have **zero example coverage**. Several phases fix this for free.

## Hard constraints (in force every phase)

- **main is branch-protected**: PR-only, 4 required CI checks (`Build (WASM)`, `Test
  (native)`, `Rustdoc`, `Package dry-run`), `enforce_admins`, `strict`. No direct push.
- **Pre-1.0 (0.x) versioning**: MINOR = any release incl. breaking; PATCH = docs/fix/no-API.
  **Never bump to 1.0.0; never revert up to 10.x.** Use the `ship` skill for the bump.
- **Verify before done**: `./scripts/verify.sh` run **as-is** (NEVER pipe to `tail`/`head` —
  it masks the exit code). For wasm-touching phases also run `./scripts/wasm_smoke.sh`.
- **WASM build gate** = lib+bins only (`cargo build --target wasm32-unknown-unknown`); do
  NOT gate on `--all-targets` (native-only examples fail it).
- **Merge authority is per-session** — re-confirm with the user before merging any PR.
- **Acceptance bar**: every feature phase ships a small example that exercises it in real play.

## Phase overview

| Phase | Theme | Items | Est. bump | New example | Size |
|---|---|---|---|---|---|
| **1** | First-hour & doc truth | C1 README, C2 hello_sprite, C3 FORKING.md, A5-doc | PATCH `0.11.1` | `hello_sprite` | S |
| **2** | Game-feel core (juice) | A1 TimeScale, A2 Tween<T>+easing, B2 (fold-in) | MINOR `0.12.0` | `juice_demo` | S–M |
| **3** | Core API ergonomics | A3 query2_mut, A4 push/pop scene | MINOR `0.13.0` | (refactor demo + reuse) | S |
| **4** | Dialogue primitive | B1 DialogueBox + typewriter | MINOR `0.14.0` | `dialogue_demo` | M |
| **5** | WASM persistence | B3 save via localStorage | MINOR `0.15.0` | extend `coin_race` | M |
| **6** | Particle depth | B4 gravity/rotation/emit-shape | MINOR `0.16.0` | `particles_showcase` | M |
| **7** (stretch) | WASM audio | B5 SFX on wasm | MINOR | wasm smoke | L |

Order rationale: **unblock starting (1) → make it feel good (2) → smooth the daily API
(3) → biggest-want feature (4) → close the WASM gaps (5,7) → polish (6)**. Phases are
independent enough to reorder, except Phase 1 should go first (it's the trust gate) and
Phase 3's demo refactor is cleaner after Phase 2 settles the example set.

---

## Phase 1 — First-hour & doc truth  (PATCH → `0.11.1`)

**Goal:** a brand-new forker can clone, run, and get their own textured sprite on screen
in <10 min, and the docs don't lie. No library API change → PATCH.

**Items + anchors:**
- **C1 — Fix the README install lie.** `README.md:30-38` shows `skeleton-engine = "2.0.0"`
  (unpublished + wrong version) → first command breaks, trust gone. Replace with a
  **fork-first "Getting Started"**: clone → `cargo run --example hello_sprite` → copy it to
  `examples/my_game.rs` + add an `[[example]]` stanza. State that the lib crate is `engine`
  (`use engine::*`) and the package is unpublished (fork, don't `cargo add`).
- **C2 — `examples/hello_sprite.rs` (~30 lines) + a placeholder asset.** Fills the example
  ladder cliff (`basic.rs` 74 lines, no image → next is 239+). Show the real asset path:
  `app.load_image(path)` → `Sprite::textured_with_handle(...)` (or document `Sprite::textured`,
  `src/components.rs:77-83`). Add `examples/assets/player.png` (32×32 placeholder). One sentence
  on asset-path root (relative to workspace root). Register `[[example]]` in `Cargo.toml`.
- **C3 — `FORKING.md` (English, ≤3 pages).** "How to build *your* game": crate name vs package
  name, where to put a new game, asset-path root, and the **collect-then-`get_mut` borrow
  pattern** (currently buried in `docs/PATTERNS.md:28-38`). Link from README. (Phase 3 will
  reduce the need for that pattern via `query2_mut`; cross-link then.)
- **A5-doc — Fix stale derive claim.** `CLAUDE.md:67` still says "`#[derive(Reflect)]` …
  (`derive` feature, default on)" but that feature was removed (commit `4f28baa`; there is no
  `[features]` table). Correct the line to reflect reality: the derive lives in the
  `engine_reflect_derive` workspace crate, used via `engine_reflect_derive::Reflect` (the
  full optional-feature restore is deferred to the publish track, see "Deferred").

**Done when:** README has no false install; `cargo run --example hello_sprite` renders a
textured sprite; `FORKING.md` exists + linked; CLAUDE.md derive line accurate; verify green.

**PR scope:** docs + 1 example + 1 asset + Cargo.toml stanza. Lowest-risk phase (no `src/`
logic). Good warm-up / context-light session.

---

## Phase 2 — Game-feel core (juice)  (MINOR → `0.12.0`)

**Goal:** the single biggest jump in how games *feel*, via tiny core changes, demonstrated
by one consolidated example that also rescues 3 orphaned features.

**Items + anchors:**
- **A1 — `TimeScale` resource.** A `TimeScale(f32)` resource (default 1.0) multiplied into
  `dt` in the scheduler's dt path (`src/app/schedule.rs`) before systems run. `App::set_time_scale(f)`
  is the forker API. Unlocks hit-stop, slow-mo, bullet-time, "animated pause". Decide & document
  whether physics/animation honor scaled dt (recommend: scaled dt is the default `dt` all
  systems already receive, so it's automatic; note any system that must use unscaled real time).
- **A2 — `Tween<T: Lerp>` + richer easing.** `Tween` is currently f32-only (`src/tween.rs`);
  make it generic over the existing `Lerp` trait (`src/timer.rs`, already impl'd for Vec2/Color)
  so `Tween<Vec2>` / `Tween<Color>` work in one line. Add `Easing::Bounce`, `Easing::Elastic`,
  `Easing::Spring { .. }` (enum has only 6 variants today). **Mark `Easing` `#[non_exhaustive]`**
  while breaking once is free (pre-1.0) so future variants are non-breaking. Keep `TweenSequence`
  working.
- **B2 (folded in) — one `juice_demo` example** exercising: `TimeScale` hit-stop on impact,
  `Tween` easing on a sprite/UI, **camera shake** (`cam.shake(strength, duration)`),
  **`FadeTransition`** (`fade_out`/`fade_in`) around a state change, and a `PostProcessConfig`
  vignette. This satisfies the acceptance bar for A1+A2 **and** the 3 existing-but-undemo'd
  features in one shot. (Alternatively wire hit-stop+shake into the existing `shooter` game and
  keep `juice_demo` for the rest — implementer's call.)

**Watch-outs:** confirm `Tween` generic migration doesn't break existing f32 call sites
(add inference-friendly constructors / keep `Tween::new` ergonomic). `Easing` becoming
`#[non_exhaustive]` is a one-time breaking change — fine pre-1.0, note in CHANGELOG.

**Done when:** `TimeScale` + `Tween<Vec2/Color>` + new easings public; `juice_demo` plays and
visibly shows hit-stop/shake/fade/easing; verify green.

---

## Phase 3 — Core API ergonomics  (MINOR → `0.13.0`)

**Goal:** kill the two API papercuts a forker hits most, including the one the **flagship
demo currently teaches**.

**Items + anchors:**
- **A3 — `query2_mut<A,B>` (mutable multi-component query).** Today only `query_mut` (1
  component, `src/ecs/world.rs:358`) and immutable `query2..query4` exist; the canonical WASM
  demo (`src/lib.rs:193-219`) resorts to **collect-then-`get_mut`** — i.e. the first code a new
  user reads teaches the anti-pattern. Add `query2_mut` (and consider `query3_mut`,
  `query_opt3`) via the disjoint split-borrow pattern `query_mut` already proves safe, then
  **refactor `run_demo` to use it** (clean idiom in the showcase). Cross-link from `FORKING.md`
  (Phase 1's borrow-pattern note can then say "or use `query2_mut`").
- **A4 — `App::push_scene` / `App::pop_scene`.** `set_scene` (Replace) is the only App-level
  convenience; Push/Pop require the verbose `world.resource_mut::<SceneChange>().unwrap()
  .request(SceneCmd::Push(..))` from inside a system. Add thin wrappers mirroring `set_scene`
  (`src/app/scenes.rs:62`). Use them in `scene_flow` (pause/overlay) to validate.

**Done when:** `query2_mut` exists + demo refactored to it; `push_scene`/`pop_scene` exist +
used in an example; verify green. (No new example required — A3 improves an existing one,
A4 reuses `scene_flow`.)

---

## Phase 4 — Dialogue primitive  (MINOR → `0.14.0`)

**Goal:** ship the single most re-invented boilerplate as a first-class primitive — a
forker magnet for RPG / visual-novel / narrative games.

**Items + anchors:**
- **B1 — `src/dialogue.rs`: `DialogueBox` + typewriter.** Component with speaker `String`,
  body `String`, press-to-advance gate, optional `chars_per_sec` typewriter (reveal =
  `floor(age * cps)` chars/frame), optional portrait slot (`Option<Handle<ImageAsset>>`), and
  **localization-key support** (integrate with `LocalizedText` / `LocaleResource`). Today a
  2-line dialogue box is hand-rolled in `settings_menu.rs` — replace/augment it as the
  reference. Re-export `engine::DialogueBox`. Register serde + (optionally) editable so it
  persists to scenes and shows in the inspector.
- Example **`dialogue_demo`**: a 2–3 NPC conversation with typewriter + advance + a portrait.

**Watch-outs:** typewriter timing should respect `TimeScale` from Phase 2 (or explicitly use
real time — decide & document). Keep it engine-agnostic about UI skin (use existing UI/Panel).

**Done when:** `DialogueBox` + typewriter public, `dialogue_demo` plays, verify green.

---

## Phase 5 — WASM persistence  (MINOR → `0.15.0`)

**Goal:** unblock browser-deployed games from saving anything (settings/progress/score).

**Items + anchors:**
- **B3 — WASM save backend.** Today `save`/`load`/`save_versioned`/`exists` return
  `Unsupported`/`false` on `wasm32` (`src/save.rs`). Add a `cfg(target_arch = "wasm32")`
  branch routing through `localStorage` via `web-sys`/`js-sys`: serialize RON → base64 →
  `localStorage.setItem(key, ..)`; `load` reverses. Decide crypto story on wasm (skip AEAD or
  use a wasm-compatible primitive — document the difference vs native).
- Example: extend `coin_race` (already ships to web) to persist a high score, **verified via
  `./scripts/wasm_smoke.sh`** (Chrome + matching `wasm-bindgen-cli`).

**Watch-outs:** keep the native path byte-identical; new wasm deps must not break the
lib+bins wasm build gate. This is the first phase that **requires the wasm smoke test**, not
just the build.

**Done when:** wasm save/load round-trips in `coin_race`; native unchanged; verify + wasm
smoke green.

---

## Phase 6 — Particle depth  (MINOR → `0.16.0`)

**Goal:** make fire / explosions / sparks actually buildable.

**Items + anchors:**
- **B4 — `ParticleEmitter` depth.** Add `gravity: Vec2` (default ZERO), `angular_velocity` +
  `angular_spread`, and an `emit_shape` enum (Point/Circle/Ring/Box) in `src/particle/`.
  Apply gravity in the velocity update, rotate instances by `age * angular_velocity`, offset
  spawns by `emit_shape`. Mirror onto `GpuParticleEmitter` (native-only) where feasible. Prefer
  additive defaults (or `#[non_exhaustive]` + `Default`) so existing emitters are unchanged.
- Example **`particles_showcase`** (can replace/augment the `gpu_particles` tech demo):
  fire (gravity −y, cone), ring explosion, sparks (high angular spread).

**Done when:** new emitter fields work, showcase plays, existing particle examples unchanged,
verify green.

---

## Phase 7 — WASM audio (stretch)  (MINOR)

**Goal:** browser games stop being silent. Largest lift; explicitly optional / last.

**Items + anchors:**
- **B5 —** `audio` module is `cfg(not(wasm32))` (`src/audio.rs`). Add a wasm SFX path (e.g.
  `web-sys` `AudioContext`, or evaluate `kira`'s wasm backend). Scope can start at one-shot SFX
  (skip the full bus mixer on wasm initially) to bound the work. Validate via wasm smoke.

**Done when:** at least one-shot SFX plays in a wasm example; native audio unchanged; verify +
wasm smoke green.

---

## Deferred / out of scope (tracked, not in the phases)

- **crates.io publish** — user decided to defer (fork-first ⇒ GitHub is the primary channel;
  no rush; 0.x = no stability promise). The reset already enabled it. **If/when** publishing:
  first publish `engine_reflect_derive` (0.1.0) and restore the `derive` *optional feature*
  (the engineering half of A5) so `cargo add` users get `#[derive(Reflect)]`. Until then the
  derive gap only affects `cargo add` consumers, not forkers. (See seq-12 handoff "crates.io
  Publish Prerequisites" — note its "path-only **regular** dep blocker" is **stale**: the dep
  is already a `[dev-dependency]`, so `cargo package`/publish is mechanically unblocked.)
- **Other depth items** (lower priority, fold in opportunistically): localization plural /
  `{param}` interpolation; text outline/shadow; `// invariant:` comments on archetype
  `unwrap()`s in query hot paths; `DrawText` `[u8;4]` → `impl Into<Color>` consistency;
  English summary of REFERENCE.html.

## Execution notes (per phase)

1. Branch off main (`feat/<phase>` or `docs/<phase>`).
2. Implement items + the phase's example. If an API feels awkward while writing the example,
   **fix the API first** (VISION rule).
3. `./scripts/verify.sh` **as-is** (+ `wasm_smoke.sh` for Phases 5/7). Read the log AND eyeball.
4. Use the `ship` skill for the version bump + CHANGELOG + CLAUDE.md header (it's now 0.x-aware
   but untested on a real bump — **first real use (Phase 1 or 2) should confirm** it bumps
   MINOR/PATCH correctly and never 1.0.0).
5. Commit, push, `gh pr create`, watch CI unmasked (`gh pr checks <n> --watch --fail-fast >
   log 2>&1`, no tail pipe).
6. **Re-confirm merge authority with the user**, then squash-merge on green; sync main.
7. Update memory (`engine-current-state`) + leave a handoff if pausing mid-roadmap.
