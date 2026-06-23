# Follow-up Batch 2 — the HDR / render-format arc (P1 → P5 → P4, v0.58.0 → v0.60.0)

**Date:** 2026-06-23
**Status:** COMPLETE + all merged. main @ `008bfd8`, package **v0.60.0**, clean tree, full gate green, each phase verified by a native GPU smoke.
**Chain:** post-umbrella follow-up run, **Batch 2 of 2** (parent: `HANDOFF_followup-batch1-wgpu-positional-web_2026-06-23.md`; grandparent: the carried `HANDOFF_p1-p4-carried-run_2026-06-23.md` umbrella).
**Auto:** false (user `/goal`: P2+P3 batch → handoff → merge [done in batch 1]; then **P1 → P5 → P4** → ONE final wrap handoff → merge → Korean completion report; intermediate reports minimized).

> This is the **session wrap** over Batch 2. Each feature landed as its own code PR (per-feature MINOR bump + example + native GPU smoke); this single handoff covers all three (the user asked for one wrap handoff, not three).

---

## The run (verbatim intent)

After the carried P1→P4 backlog was exhausted (the umbrella), the user asked for a **priority-ordered list** of the umbrella's five "Where We're Going" follow-up candidates, then chose: **batch 1 = P2 + P3** (low-risk quick wins — `wgpu` re-export + `positional_audio` web, shipped as v0.57.0, handoff `…batch1…`), **batch 2 = P1 → P5 → P4** (the HDR / render-format arc). Cadence: each phase its own code PR, then ONE final wrap handoff + merge + completion report.

## Where We Are (end state)

- **main @ `008bfd8`, package v0.60.0, CLAUDE.md header v1.6.129, clean tree, full gate green.**
- **Batch 2 = 3 code PRs** (each on green CI, squash-merged): **#212** P1 (v0.58.0) · **#213** P5 (v0.59.0) · **#214** P4 (v0.60.0). Plus this wrap handoff PR.
- **Version walk:** 0.57.0 → **0.58.0** (P1) → **0.59.0** (P5) → **0.60.0** (P4) — all MINOR (additive).
- **The whole user-selected follow-up run (batch 1 + batch 2) is COMPLETE.** No backlog remains.

## The three phases

| Phase | Version | PR | What shipped | Example | Native GPU smoke |
|---|---|---|---|---|---|
| **P1** | 0.58.0 | #212 | **HDR tone-mapping in the post-process pass** — `PostProcessConfig.{hdr, exposure, tonemap}` + `Tonemap{None,Reinhard,AcesFilmic}`. `hdr` renders the scene into an `Rgba16Float` intermediate; the post shader applies exposure + the tonemap operator. Default off = byte-identical (`pad0: vec2` repurposed for `exposure`+`tonemap`, layout unchanged). | `tonemap` | `None` clamps over-bright swatches to identical white; `ACES` rolls them off (distinct). HDR-off collapses them (8-bit clamps at store). No format-mismatch panic. |
| **P5** | 0.59.0 | #213 | **Format-matched material + UI pipelines** — `MaterialRenderer.custom_pipelines` keyed by `(hash, format)`; `SpriteRenderer.extra_ui_pipelines` per format. Materials render into non-surface RTs (offscreen) + the HDR post intermediate; UI primitives render through the HDR intermediate. **Lifts P1's material+UI skips** (only GPU particles remain skipped under HDR post). | `offscreen_material` (+ a UI `DrawRect` added to `tonemap`) | Both an `Rgba16Float` and a surface offscreen monitor render the plasma material (HDR one was previously blank). The tonemap UI bar renders under HDR post. |
| **P4** | 0.60.0 | #214 | **`hdr_render_target` shipped to the web** — `web/` harness + `#[wasm_bindgen] run_hdr_render_target` (`main` body → shared `run()`). **Confirms `Rgba16Float` color render targets work on `wgpu`'s WebGL2 backend** (needs `EXT_color_buffer_float`). New `scripts/hdr_web_smoke.sh`. Render targets were already cross-platform; the example's "native-only" docs were wrong + corrected. No library change. | `hdr_render_target` (now web too) | `hdr_web_smoke.sh` PASS (40 KB non-blank frame headless under SwiftShader); eyeballed — HDR monitor keeps core-vs-mid distinct, LDR collapses them, on the **web**. |

## Key cross-cutting decisions

- **P1 needed NO main-path HDR rewrite, so the user's order P1→P5→P4 held (no reorder).** Reading the render path up front (batch-1 handoff "Batch-2 scoping") showed: sprites already format-match (carried-P4 `extra_sprite_pipelines`); text draws *after* post; so a **sprite-only** HDR post example works with just the sprite cache. P1 shipped that + guarded the not-yet-format-matched passes (material/UI/particles) with a **skip + warn-once** (crash-safe). P5 then *lifted* the material + UI skips by format-matching them. This staged the risk cleanly: P1 = the operator + HDR intermediate (sprite scenes); P5 = the remaining pipelines; P4 = the web proof.
- **Same per-format-cache pattern, three times.** Sprites (carried-P4), then materials (`(hash, format)` key), then UI (`extra_ui_pipelines`) — all mirror `ensure_*_pipeline` / `*_pipeline_for` + a `build_*_pipeline` free fn, surface fast-path untouched. The material/UI work was "apply the proven pattern", which is why P5 was low-risk despite touching the hot draw path.
- **Honest scope over fake robustness.** P1 documents that GPU particles + (pre-P5) materials/UI are skipped under HDR post rather than pretending they work. P4 does **not** ship a fake "fall back to 8-bit" path (I wrote that doc, then **removed it** when I realised the engine resolves the RT format at GPU-init, after the example's setup — a real fallback would be an engine-level format-feature query, out of scope); the docs state the `EXT_color_buffer_float` requirement accurately and the smoke proves it holds on SwiftShader.
- **Native GPU smoke per phase (CI can't verify GPU).** Each phase was driven in a real window + screenshotted; P4 additionally got a reusable headless-Chrome smoke.

## Reusable gotchas (carry forward)

- **macOS synthetic key into a winit window: `keystroke` / `keystroke "1"` does NOT land; raw `key code <N>` does.** The tonemap smoke's first `None`-vs-`ACES` toggle silently no-op'd (identical screenshots) via `System Events … keystroke "1"`; `key code 18` worked. (Pairs with the carried gotcha that `set frontmost` must precede the key.) Digit/letter key codes: 1=18, 3=20, H=4, Esc=53, arrows 123–126.
- **`PostProcessUniforms` had spare `pad0: vec2` → repurposed for `exposure` + `tonemap`** with NO buffer-size change (48 B, 16-B aligned). Repurposing existing padding is the cleanest way to add scalar uniform fields without touching the bind layout.
- **A material's WGSL source must be re-clonable per target format.** `collect.rs` clones the frag source only for the first entity of a not-yet-compiled hash; with per-format caching the check must be `contains_key(&(hash, target_format))` (else a hash compiled for format A never re-clones its source for format B → can't compile the B pipeline). Thread the target format into `collect_draw_entries`.
- **`Rgba16Float` IS a usable WebGL2 color render target via `wgpu` (with the `webgl` feature) under SwiftShader** — so modern browsers (which all ship `EXT_color_buffer_float`) work. A headless SwiftShader render is a valid lowest-bound check for float-RT portability.
- **Render targets / the offscreen pass are cross-platform** (`render_offscreen_targets` is called un-gated in `frame.rs`; `create_render_target_impl` has no `cfg`). The "render targets are native" lore was an example doc-comment, not a real constraint.

## Evidence

- **Verify (`VERIFY_EXIT` from the log, never piped):** P1, P5, P4 each `0` first try. The fmt-reflow trap never bit (ran `cargo fmt` before each gate, per `[[cargo-fmt-reflow-trap]]`).
- **CI:** #212 / #213 / #214 all 4/4 green, squash-merged.
- **Merge commits (code):** P1 `60092ef`, P5 `bb8a953`, P4 `008bfd8` (PR #214). This wrap handoff lands as its own docs PR on top.
- **Smokes:** P1 `tonemap` (None/ACES/HDR-toggle screenshots); P5 `offscreen_material` (both monitors render the material) + `tonemap` UI bar under HDR; P4 `scripts/hdr_web_smoke.sh` PASS + eyeballed.

## Where We're Going (offer, NOT committed)

- **The whole follow-up run (batch 1 + 2) is COMPLETE; no backlog remains.** Next session: read `../dungeon-merchant/docs/engine-wishlist.md` FIRST (ACTIVE empty, next ID EW-002), then ASK for direction.
- Remaining follow-ups surfaced this run:
  1. **GPU particles under HDR post** — the one pass still skipped; give `GpuParticleRenderer` a per-format pipeline (it's native-only + niche).
  2. **An engine-level `EXT_color_buffer_float` / format-renderability query** so `create_render_target_with_format` can gracefully fall back instead of erroring on an unsupporting backend (the real P4 fallback).
  3. **A real bloom pass** — P1 kept the existing approximate single-pass bloom; a bright-pass + separable blur would be the next post-process step.
  4. Ship the audio-facade-arc web demos by ear (P2 `positional_audio` + `audio_facade` are built but unheard in a real browser this run).

## Risks & Blockers

- **None blocking.** main clean + green at v0.60.0.
- **Untracked `docs/CODE_QUALITY_FINDINGS_2026-06-23.md`** appeared mid-session (a separate code-quality scan, not from this run) — left untracked + uncommitted on purpose (not mine to land). The next session can review/act on it or delete it.
- Web HDR/audio paths are verified headless (SwiftShader) / by bundle-build, not by a human in a real browser.

## Quick Start for Next Session

```bash
git checkout main && git pull --ff-only        # expect main @ 008bfd8 or later (v0.60.0)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?" >> /tmp/verify.log; grep VERIFY_EXIT /tmp/verify.log   # must be 0
cat ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE empty, next ID EW-002 — ASK for direction
# Live state + gotchas: memory engine-current-state + MEMORY.md.
```

## Related Handoffs (this run)

- Batch 1 — `HANDOFF_followup-batch1-wgpu-positional-web_2026-06-23.md` (P2+P3, v0.57.0)
- Batch 2 (this) — P1/P5/P4, v0.58.0 → v0.60.0
- Parent umbrella — `HANDOFF_p1-p4-carried-run_2026-06-23.md`
