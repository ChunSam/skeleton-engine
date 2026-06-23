# PATTERNS doc — render-target-format-aware pipeline cache (rule + recipe)

**Date:** 2026-06-23
**Status:** COMPLETED
**Bead(s):** none (`bd` unavailable; engine work tracked via the dungeon-merchant wishlist board)
**Epic:** none — a `/wrap`-driven pattern-documentation one-shot
**Chain:** `patterns-doc` seq `1` (new chain — NOT a continuation of `codequality-backlog`, which is COMPLETE)
**Parent:** none
**Next:** nothing queued. Wishlist board ACTIVE is empty (next free ID **EW-004**) → next session read `../dungeon-merchant/docs/engine-wishlist.md` FIRST, then ASK for direction if still empty.

---

## The Goal

This session had no pre-set task. It started from `마지막 핸드오프 확인하고 작업 확인` (check the last handoff + confirm work) on a clean+green `main`, found the queue genuinely empty (the 2026-06-23 code-quality scan fully CLOSED, wishlist board empty), and the user ran **`/wrap`** — analyze the last 12h of commits and propose skill/rule candidates.

The `/wrap` analysis surfaced one strong candidate: today's HDR / offscreen-RenderTarget arc (#212/#213/#214/#220) had turned a **single sprite-pipeline trick into a 4th-instance idiom** — the same "lazy per-target-format pipeline cache" shape was now copy-pasted across four renderers, with later copies literally commenting "Mirrors the sprite cache." That is exactly the threshold for writing a pattern down. The user picked candidates **A (recipe → `docs/PATTERNS.md`) + B (rule → CLAUDE.md)** and said `a+b진행`, then `머지 해`.

So the deliverable: **document the render-target-format-aware pipeline cache pattern** (rule + recipe + the 4 instances) so a fork/contributor adding a new render pass keys it by target format from the start, instead of rediscovering the #213/#220 regressions.

## Where We Are

- **main @ `5a2c66d`, package v0.63.3 (UNCHANGED), CLAUDE.md header v1.6.138, clean + green, no open PRs.** PR **#229** merged (squash, `mergedAt 2026-06-23T10:20:52Z`).
- **`docs/PATTERNS.md`** gained a new section **"Render-target-format-aware pipeline cache"** under *Core architecture patterns* (inserted right after "Render layer separation", before "UI system registration order"). It contains:
  - The **rule** (B): a new render pass must key its pipeline by the *target* `TextureFormat`, not `gpu.config.format` — else the feature **silently vanishes** under HDR post / offscreen `RenderTarget`s.
  - The **recipe** (A): keep the surface-format fast path + lazily build/cache per non-surface format, as a 3-step code snippet (`base_format` + `HashMap<TextureFormat, RenderPipeline>` + `ensure_*` + `*_for`).
  - A **4-instance table** with exact cache-field / build-method / file for each renderer.
  - A note that the keys differ (`ShaderMaterial` keys by `(hash, format)`), so the shape is **duplicated deliberately** — revisit a shared helper only at a 5th instance.
- **`CLAUDE.md`** got a **one-line discoverability hint** in the architecture-patterns bullet list (so a reader scanning CLAUDE.md sees the rule inline) + **doc-version `v1.6.137 → v1.6.138`**.
- **`.claude/proposals/2026-06-23.md`** — the `/wrap` proposal (candidates A/B/C/D). **Gitignored**, so it is NOT in the repo; this handoff + the memory are its only durable record.

## Public API surface

**None.** Docs-only change (markdown only — `docs/PATTERNS.md` + `CLAUDE.md`). No `.rs` touched, no behavior change, no package version bump.

## The pattern (the actual content shipped)

**Why it exists:** a `wgpu::RenderPipeline` bakes in its color-target `TextureFormat` at creation. A pipeline compiled for the surface format **fails wgpu validation** if used against a different target — an offscreen `RenderTarget` or the `Rgba16Float` HDR post-process intermediate. HDR post renders the scene into a non-surface intermediate, so every scene pass needs a pipeline matching *that* format.

**The rule (B):** key a new render pass's pipeline by the **target** format, not `gpu.config.format`. Skipping it makes the feature silently disappear under HDR/offscreen — the exact #213 (UI primitives + `ShaderMaterial`) and #220 (GPU particles) regressions.

**The recipe (A) — 3 steps, surface fast path preserved:**
1. Store the surface pipeline + its `base_format`; keep an empty `HashMap<TextureFormat, RenderPipeline>`.
2. `ensure_*(device, format)` — no-op for `base_format` or a cache hit; else compile + insert (paid once per distinct format, never per frame).
3. `*_for(format)` — base pipeline for the surface format, else the cached extra (fall back to base if somehow missing, to **never panic mid-frame**).

**The 4 instances (all this shape):**

| Renderer | Cache field | Build / select | File |
|---|---|---|---|
| Sprite | `extra_sprite_pipelines` | `ensure_sprite_pipeline` / `sprite_pipeline_for` | `src/renderer/sprite.rs` |
| UI primitive | `extra_ui_pipelines` | (same module) | `src/renderer/sprite.rs` |
| `ShaderMaterial` | `custom_pipelines` keyed by `(hash, format)` | `MaterialRenderer::compile_pipeline` | `src/renderer/sprite/material.rs` |
| GPU particle | `extra_render_pipelines` | `ensure_render_pipeline` / `render_pipeline_for` | `src/renderer/gpu_particle.rs` |

The target format is carried in `FrameContext.format`; the offscreen pass threads the `RenderTarget`'s `format()`.

## What We Tried (process)

1. **Read state, found empty queue.** `git log` (main @ cd20423, v0.63.3, clean), wishlist board (ACTIVE empty, EW-004), memory (scan CLOSED). Confirmed nothing queued → asked the user for direction → user ran `/wrap`.
2. **`/wrap` analysis.** Read `git log --since="12 hours ago" --patch` (20 commits, 07:42–18:10, three arcs: display EW-002/003, HDR/render-format, code-quality scan resolutions). Grepped the 4 cache sites to confirm the per-format pattern was genuinely repeated (it was — `extra_sprite_pipelines` / `extra_ui_pipelines` / `custom_pipelines` / `extra_render_pipelines`, comments cross-referencing each other). Wrote `.claude/proposals/2026-06-23.md` with candidates A (recipe→PATTERNS), B (rule→CLAUDE.md), C (opt-in resource — already documented, reinforced only), D (renderer::common hint — weak).
3. **User picked A+B.** Read `docs/PATTERNS.md` structure + grepped for any existing `format`/`pipeline cache` entry (none). Re-read the 4 cache implementations (`gpu_particle.rs::ensure_render_pipeline`/`render_pipeline_for`, `material.rs::compile_pipeline`, `sprite.rs::ensure_sprite_pipeline`/`sprite_pipeline_for`) to get **exact** method names + file paths for the table.
4. **Wrote the section** into PATTERNS.md + the one-line CLAUDE.md hint + doc-version bump.
5. **Landed via `/land-pr`** (see Evidence).

No failed approaches — the change is small and the only judgment calls were (a) bundle A+B into one PATTERNS section vs. split, and (b) document-vs-abstract.

## Key Decisions

- **A+B bundled into ONE PATTERNS section**, not two. B (the rule) is the "why" for A (the recipe); splitting them would scatter the reasoning. CLAUDE.md carries only a one-line pointer to keep it lean.
- **Document, do NOT abstract.** A shared helper/trait/generic-cache-struct was explicitly **deferred**. The 4 keys differ (`ShaderMaterial` is `(hash, format)` because the frag shader also varies; the others are `format` alone) and the build closures have different pipeline layouts — a premature generic would be more complex than the duplication. Rule recorded: **revisit a shared helper only at a 5th instance.**
- **Doc-version bump but NO package bump.** Per `/land-pr` step 3: editing `CLAUDE.md` bumps its leading doc-version (`v1.6.137 → v1.6.138`), but a docs-only change takes no package version bump and no CHANGELOG entry (mirrors #228/#225). Package stays v0.63.3.
- **New chain, not a continuation.** This is a `/wrap`-driven pattern capture, a distinct work stream from the HDR *implementation* (which lives in the HDR-arc handoffs) and from the code-quality scan *resolution* (the `codequality-backlog` chain, COMPLETE). Conflating them would be wrong.

## Evidence & Data

- **Verify:** `./scripts/verify.sh > /tmp/verify.log 2>&1` → **`VERIFY_EXIT=0`**; "all checks passed ✓"; tests **71 passed / 0 failed / 32 ignored** (doc-tests). fmt + clippy + wasm lib-build + tests + rustdoc all green. (A docs-only tree, so this was effectively a no-op vs. main, but run per the verification discipline.)
- **CI #229:** 4/4 green — Build (WASM) 33s, Package dry-run 53s, Rustdoc 39s, Test (native) 3m38s. `mergeStateStatus=CLEAN`, `mergeable=MERGEABLE` before merge.
- **Merge:** `gh pr merge 229 --squash --delete-branch` → `5a2c66d`. `git checkout main && git pull --ff-only` → fast-forward `cd20423..5a2c66d`, 2 files / +53 −4.
- **Diff:** `CLAUDE.md` +10/−4 (the pattern line wraps + doc-version), `docs/PATTERNS.md` +47.
- **Memory:** `engine-current-state.md` updated — description opener + body lead + a new **seq 86** bullet at the top of "Recent seqs"; `main @` pointer `cd20423 → 5a2c66d`, header `v1.6.137 → v1.6.138`. (NOTE: the engine-state memory uses a global "seq 8x" counter — seq 86 there ≠ this handoff's `patterns-doc` seq 1.)

## Gotchas / discoveries

- **`/wrap` proposals are gitignored.** `.claude/proposals/2026-06-23.md` is NOT committed (the whole `.claude/proposals/` dir is gitignored). So a proposal's reasoning survives only in (a) this handoff and (b) the memory. If you want the proposal itself preserved, it must be copied somewhere tracked.
- **Two `seq` numbering systems — don't conflate.** (1) The memory's global engine-state counter (`seq 83/84/85/86…`). (2) Per-handoff-chain seq (`codequality-backlog seq 1/2/3`; this handoff = `patterns-doc seq 1`). They are unrelated counters.
- **Docs-only + edited CLAUDE.md ⇒ doc-version bump, no package bump.** Easy to get wrong in either direction (bumping the package for a docs change, or forgetting the doc-version when you DID edit CLAUDE.md). The rule: package bump iff `.rs`/`Cargo.toml` changed; doc-version bump iff CLAUDE.md changed.
- **`verify.sh` on a markdown-only tree is a formality but still run it** — the discipline says capture `$?`, never pipe (`| tail` masks a real fmt/clippy red). It came back 0 here.

## Where We're Going

**Nothing is queued.** The session emptied its own queue:
- Code-quality scan: fully CLOSED (P1×2, P2×2, P3×3 + this pattern-doc follow-up).
- Wishlist board: ACTIVE empty, next free ID **EW-004**.

**Next session, in order:**
1. Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST (the game session may have filed EW-004+).
2. If still empty → **ASK** the user for direction (do not start backlog speculatively — this is the standing rule when ACTIVE is empty).
3. Possible un-queued offers if asked for ideas: a real bloom pass (HDR follow-on); engine-level format-renderability query (the "real P4 fallback" deferred in the HDR arc); hearing the audio-facade web demos by ear.

**If a 5th per-format pipeline cache instance ever appears** (a new render pass that must work in offscreen/HDR RTs), THAT is the trigger to revisit abstracting the 4 (now 5) copies into a shared helper — and to update this PATTERNS section. Until then, the deliberate-duplication note stands.

## Related Handoffs (reference only — NOT parents)

- `HANDOFF_followup-batch2-hdr-render-arc_2026-06-23.md` — the HDR/render-format arc (#212/#213/#214/#220) that *created* the 4th instance this pattern documents.
- `HANDOFF_gpu-particles-hdr_2026-06-23.md` — #220, the GPU-particle per-format cache (the most recent instance).
- `HANDOFF_hdr-render-target_2026-06-23.md` — #207 (v0.56.0), the original `create_render_target_with_format` + sprite per-format cache (instance #1).
- `HANDOFF_egui-submit-dedup_2026-06-23.md` — `codequality-backlog` seq 3, the last of the scan-resolution chain (COMPLETE; this session is a separate stream).

---

## Session Closed
**Closed at:** 2026-06-23 22:44 KST
**Commit:** landed as its own `docs(handoff)` PR (squash-merged to `main`; see the PR for the exact hash)
**Session status:** Handed off to next session
