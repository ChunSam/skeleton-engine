# v10 architecture pass — scoped plan (for sign-off)

**Date:** 2026-06-16 · **Base:** `main` @ v9.6.1 · **Source:** cohesion review
(`docs/MODULE_COHESION_REVIEW_2026-06-16.md`, items 5/6 + Themes 1/3) + a 4-way parallel
read-only analysis (breaking-surface + migration + effort/risk per item, file:line at v9.6.1).

> **Status: PROPOSED — needs sign-off. No breaking code written yet.**

---

## Executive summary (the key finding)

The cohesion review flagged items 5–7 as "breaking → v10." The detailed analysis shows the
**actual breaking surface is very small**, and most of the *architectural* value is **internal-only
(zero semver impact)**:

- **No item has any external/forker call site in `examples/`.** Every "leaking" field/method is read
  only by engine-internal code (`src/app/render.rs` etc.).
- **Genuinely breaking the public API:** only **A** (RenderTarget field removal), **J** (unit-struct
  construction change), optionally **D** (a rename), and **K** (a module-path move, mitigable with a
  re-export bridge). All have trivial migrations.
- **Internal-only (could even ship in v9.x):** **B, C, E, F, G, H, I** — no public surface changes
  (non-prelude types, private fields/fns, or re-exported module splits).
- The **real risk** is not breakage — it's **regression on the render path, which has no GPU test in
  CI**. That risk is concentrated in **F (split `render()`)** and to a lesser extent **E/H**.

So "v10" is justified by a handful of surgical breaks (A/J/±D/±K); the heavy refactors (E/F/G/H/I)
are internal and can be sequenced for safety, not semver.

---

## Per-item table

| Item | What | Breaking (forker)? | Effort | Risk | CI catches? | Dep |
|---|---|---|---|---|---|---|
| **A** | `RenderTarget` `texture/view/sampler/bind_group` → `pub(crate)` + accessors | Technically yes (prelude field removal); **0 observed external users** | S ~20 LOC | Low | partial | — |
| **B** | `LightingRenderer` `normal_view/width/height` → `pub(crate)` | No (not in prelude) | S ~10 LOC | Very low | partial | — |
| **C** | `texture_layout()` removed; `RenderTarget::new` builds its own layout via `Texture::bind_group_layout(device)` | No (not in prelude) | S ~15 LOC | Low | yes (build) | — |
| **D** | `get_collider`→`collider` rename (escape-hatch doc already done) | Only if renamed; **optional** | S ~5 LOC | Very low | yes | — |
| **E** | Extract `RenderState` from `App` (~14 renderer/texture fields; all private, `world` stays on `App`) | **No** (all private) | L ~400 LOC | Medium (borrow destructuring on grouped `&mut self.render`) | partial | — |
| **F** | Split `App::render()` (839 lines) into ~15 per-pass helpers | **No** (`render()` private) | L ~600 LOC | **HIGH** (submit ordering, texture routing, cfg gates; no GPU test) | **no** | E |
| **G** | Split `schedule::update()` (386 lines) → `compute_viewport`/`run_systems`/`post_systems` + egui begin/end → `egui_pass.rs` | **No** (`update()` `pub(super)`) | M ~200 LOC | Low-Med (725 tests cover system loop + egui-delta merge) | yes | E (soft) |
| **H** | `SpriteRenderer` → `MaterialRenderer` + `TextureCache` | **No** (not in prelude; only internal ctor) | L ~180 LOC | Medium (layout ownership; panics if mis-borrowed) | partial | — |
| **I** | Split `tilemap.rs` (1620 lines) → `tilemap/{mod,autotile,system}.rs`, re-export 9 names | **No** (re-export preserves paths) | **S** | **Low** | yes (build) | — |
| **J** | `UiSystem`/`SteeringSystem` unit structs → scratch fields (`X::default()`) | **YES** — `add_system(X)` → `add_system(X::default())` | S ~65 LOC | Low | yes | — |
| **K** | `ScriptAsset` → `src/scripting/`, `ast`→`pub(crate)`, new `ScriptRegistry` resource via `HotReloadable` | Yes (module path) — **mitigable** w/ `pub use` bridge | S ~200 LOC | Low | yes | — |

---

## Breaking surface (the complete forker migration)

This is the ENTIRE public break of the proposed pass:

1. **J:** `add_system(UiSystem)` → `add_system(UiSystem::default())`; `add_system(SteeringSystem)` →
   `add_system(SteeringSystem::default())`. (In-repo: 6 example + 8 test `UiSystem` sites; 2 example
   + 6 test `SteeringSystem` sites — all enumerated in the analysis; we migrate them.)
2. **A:** anyone reading `RenderTarget::{texture,view,sampler,bind_group}` directly uses the new
   accessors instead. (0 observed users.)
3. **D (optional):** `get_collider(h)` → `collider(h)`. (Skippable.)
4. **K:** `engine::asset::ScriptAsset` → `engine::scripting::ScriptAsset` — softened by keeping
   `pub use scripting::ScriptAsset;` in the `asset` re-export for one release.

Everything else (B, C, E, F, G, H, I) is invisible to a forker.

---

## Recommended PR sequence

**Phase 0 — safe internal wins (low risk, ship first):**
- **PR1: I** (split `tilemap.rs`). Pure file split + re-export. S / Low. Cheapest, zero behavior change.

**Phase 1 — the breaking surface (small, surgical — this is what "v10" is *for*):**
- **PR2: A + B + C** (Theme-3 encapsulation batch). wgpu fields → `pub(crate)` + accessors;
  `RenderTarget::new` builds its own layout. ~45 LOC, Low risk.
- **PR3: J** (`UiSystem` + `SteeringSystem` scratch fields). Breaking construction + the deferred
  per-frame-alloc fix (#76). Migrate all in-repo sites. S / Low.
- **PR4: K** (`ScriptAsset` → `scripting/` + `ScriptRegistry`). With the re-export bridge. S / Low.
- *(optional)* **D** rename — fold into PR2 or skip.

**Phase 2 — the big internal refactor (high value, high effort/risk — gated on playtest):**
- **PR5: E** (extract `RenderState`). Enabler for F/G. L / Medium.
- **PR6: F** (split `render()`). **HIGHEST RISK.** L / High. **Requires a visual playtest** across all
  render modes before merge (see Verification). Depends on E.
- **PR7: G** (split `update()`). M / Low-Med. Depends on E (soft).
- **PR8: H** (`SpriteRenderer` split). L / Medium. Independent — can interleave.

Phases 0+1 (PR1–PR4) are tractable and mostly low-risk: ~330 LOC, the whole breaking surface, all
CI-or-build verifiable. Phase 2 (PR5–PR8) is the ambitious render-architecture refactor that carries
genuine regression risk on the CI-untested GPU path.

---

## Verification strategy

- **Phases 0–1 + G:** `./scripts/verify.sh` (now incl. `cargo test --doc`) is a real gate — these are
  build/test/clippy-verifiable. J + tilemap + encapsulation all have unit-test or compile coverage.
- **F (and E/H):** CI has **no GPU test** — a wrong texture view, submit order, or `LoadOp` is
  visually silent. Before merging F, run a **visual playtest** of every render mode: (a) normal,
  (b) docked editor, (c) `PostProcessConfig`, (d) `AmbientLight`+point lights, (e) fade transition,
  (f) `OffscreenCamera` (`security_camera`), (g) GPU particles. The macOS windowed-playtest harness
  (osascript bounds + screencapture, per the `playtest-windowed-examples` note) can do this
  headlessly on this machine, or the user eyeballs it.

---

## Sign-off decision

Pick the scope:

- **(a) Full pass** — PR1–PR8 (all of v10). ~1675 LOC across 8 PRs; Phase 2 needs the render playtest.
- **(b) Tractable subset** — Phase 0+1 only (PR1–PR4): the entire breaking surface + tilemap split,
  all low-risk and CI/build-verifiable. Defer the render refactor (E/F/G/H) to a later dedicated arc.
  **Recommended** if we want v10's value without the high-risk render surgery this round.
- **(c) Cherry-pick** — e.g. just I (tilemap, non-breaking, could even be v9.7) + J (the real perf
  win) and skip the rest.

After sign-off, each PR follows the loop: scope → impl (Sonnet) → independent Gate6 → version/CHANGELOG
→ PR → CI → merge. **No breaking PR opens until this is signed off.**
