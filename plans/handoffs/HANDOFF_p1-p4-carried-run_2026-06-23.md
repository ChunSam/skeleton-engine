# Session umbrella — the P1→P4 carried-direction `/goal` run (v0.52.0 → v0.56.0)

**Date:** 2026-06-23
**Status:** COMPLETED + all merged. main @ `be24649`, package **v0.56.0**, clean tree, no open PRs, full gate green.
**Bead(s):** none (bd unavailable in this environment)
**Epic:** post-audit feature work — the carried-direction backlog (now **exhausted**)
**Chain:** `standalone-4365aa4a` seq `12` (session umbrella over the four per-phase handoffs seq 8–11)
**Parent:** `HANDOFF_hdr-render-target_2026-06-23.md` (seq 11, P4)
**Auto:** false (driven by a `/goal`: do carried directions P1→P4 in order, each phase test→handoff→merge, final report only, skip intermediate reports)

> This is the **session-level** record. Each phase has its own detailed handoff (seq 8–11) — read those for the deep per-phase detail. This file captures the **whole-session arc**, the cross-cutting decisions/gotchas, and the **start-of-session wrap/skill/memory work that has no other handoff**.

---

## The Goal (verbatim intent)

A `/goal` set this session's directive: **"p1 부터 순서대로 진행 / 각 페이즈 완료 후 테스트 후 handoff 하고 머지 / 모든 작업 완료 후 완료 보고 하고 종료 / 중간 보고 생략."** → Run the four carried directions P1→P4 **in order**; per phase: implement → test (verify gate + native smoke) → write a handoff → merge; **final completion report only** (no intermediate reports); autonomous (drive the whole chain without per-step confirmation). A session-scoped Stop hook enforced "not done until all four land."

The four carried directions (from the seq-7 `play_tone` handoff's "Where We're Going", priority-ordered by me earlier this session and confirmed by the user):
- **P1** — adopt the `Audio` facade in `settings_menu` (the last audio-cfg example game).
- **P2** — positional audio on the facade (excluded since the facade landed).
- **P3** — `RonRegistry<V>` + `RonLoadable` public at the crate root (carried bonus since seq 2).
- **P4** — HDR / linear render-target (the seq-67-deferred item; the riskiest).

## Where We Are (end state)

- **main @ `be24649`, package v0.56.0, CLAUDE.md header v1.6.125, clean tree, no open PRs, `./scripts/verify.sh` → exit 0.**
- **8 PRs merged** (4 code + 4 docs(handoff), each on green CI): #201/#202 (P1), #203/#204 (P2), #205/#206 (P3), #207/#208 (P4). Plus this umbrella.
- **Version walk:** 0.52.0 → **0.53.0** (P1) → **0.54.0** (P2) → **0.55.0** (P3) → **0.56.0** (P4) — all MINOR (additive).
- **Memory** `engine-current-state` at **seq 74** (engine-wide counter); `MEMORY.md` index refreshed; two new rule memories created (see below).
- **Carried backlog EXHAUSTED.** Next session: read `../dungeon-merchant/docs/engine-wishlist.md` FIRST (ACTIVE empty, next ID EW-002), then ASK for direction.

## Start-of-session work (no other handoff — recorded here)

Before P1, the user ran `/wrap` then asked for the skill/rule candidates from the day's commits. Outcome (all applied):

- **Two rule memories created** (`project` type):
  - **`cargo-fmt-reflow-trap`** — `cargo fmt --check` reflows hand-wrapped long lines (big tuple `let`s, wrapped `if a||b` chains) and reds the FIRST verify gate; run `cargo fmt` before the gate. (Bit twice on 2026-06-22.) **Paid off immediately — the fmt trap did NOT bite once across all four phases this session.**
  - **`wasm-gate-excludes-examples`** — `verify.sh`'s wasm step is lib+bins only; after un-gating an example's `cfg(wasm32)` path, separately `cargo build --example <name> --target wasm32`. (Used in P1/P2.)
- **`/land-pr` skill gained a "Handoff mode" section** — codifies the per-seq cadence: a handoff doc lands as its OWN `docs(handoff): seq-N <title>` PR (branch `docs/handoff-seqN-<slug>`, no package bump), and the step-9 memory seq-bump belongs to that handoff PR's landing (so the recorded `main @ <hash>` points at the handoff merge = the session's true tip). `local-tooling-skills` memory updated to note this (skills are gitignored — memory is the only record, per `record-skills-in-memory`).
- `MEMORY.md` index updated with the two new memories.

This cadence was then followed for all four phases: code PR merges first, handoff written referencing the code merge hash, handoff lands as its own PR, memory bumped pointing at the handoff merge.

## The four phases (summary — per-phase handoffs hold the detail)

| Phase | Seq | Version | What shipped | Code/handoff PR | Per-phase handoff |
|---|---|---|---|---|---|
| **P1** | 8 / mem 71 | 0.53.0 | **Named tone channels + low-pass** on the `Audio` facade (`play_tone_on_channel` / `is_channel_playing` / `set_low_pass` / `clear_low_pass`; native `AudioManager` channels + `AudioEffect.low_pass_hz`, web tracked `OscillatorNode`s + `BiquadFilterNode`). `settings_menu` adopts the facade → **closes the LAST audio-cfg game**. `audio_facade` example gains G/L. | #201 / #202 | `HANDOFF_facade-tone-channels-lowpass_2026-06-22.md` |
| **P2** | 9 / mem 72 | 0.54.0 | **Tracked 2D positional sound** on the facade (`play_at_on_channel` / `update_position` / `stop_channel`; native new `AudioManager::play_bytes_at`, web looping `Sfx`+panner in a `spatial_channels` map). Example `positional_audio`. | #203 / #204 | `HANDOFF_facade-positional_2026-06-23.md` |
| **P3** | 10 / mem 73 | 0.55.0 | **`RonRegistry<V>` + `RonLoadable` public** at the crate root — fork-friendly custom-asset registry. Example `ron_registry` (a custom `CreatureStats` RON config). | #205 / #206 | `HANDOFF_ron-registry-pub_2026-06-23.md` |
| **P4** | 11 / mem 74 | 0.56.0 | **HDR / linear render targets** — `App::create_render_target_with_format` + a per-format sprite pipeline cache so an `OffscreenCamera` renders into a non-surface-format (e.g. `Rgba16Float`) RT. Closes the seq-67-deferred item. Example `hdr_render_target`. | #207 / #208 | `HANDOFF_hdr-render-target_2026-06-23.md` |

**Combined audio-facade arc (P1+P2):** the cross-platform `Audio` facade now covers SFX, synthesized tones, named/trackable tone channels + low-pass, tracked positional, music/crossfade, buses/ducking — excluding ONLY untracked positional one-shots + per-channel effects beyond low-pass + automatic sidechain. `settings_menu`/`survivor`/`shooter` are all cfg-guard-free with web audio.

## Key cross-cutting decisions

- **Named-channel pattern, reused across P1 + P2.** Both the tone-channel (P1) and positional (P2) APIs address a *tracked* sound by a stable caller-chosen channel name (`play_*_on_channel` + `update`/`is_playing`/`stop_channel`), rather than returning a cross-platform handle (awkward on native, where mutation needs `&mut AudioManager`). Consistent surface; `update_position` even needed **no `cfg` split** (both backends share the signature).
- **Honest intersection, not lowest-common-denominator lies.** The facade exposes only what BOTH backends do cleanly: low-pass (rodio `low_pass` / Web Audio `BiquadFilterNode`) yes; pitch/attack/release no (web has no cheap equivalent). HDR tone-mapping (P4) is left to the game — the engine just makes the format-flexible RT *render correctly*.
- **Additive everywhere (MINOR bumps).** Every phase kept existing call sites byte-identical: `create_render_target` delegates to `_with_format`; `play_sfx_to` delegates to `play_sfx_to_opts(repeat)`; native `play_at`/`play` untouched (added `play_bytes_at`); the surface-format sprite pipeline fast-path is untouched (P4's cache is empty in the common case).
- **Per-phase test→handoff→merge, code and docs as SEPARATE PRs.** Eight PRs, not four — the handoff doc never bundles with the code (matches the `/land-pr` Handoff-mode written at session start).
- **Example is the acceptance test (VISION).** Every phase shipped a playable example exercising the feature in real play, each verified by a native real-play smoke (not just green CI).

## Reusable gotchas & patterns (carry forward — the expensive discoveries)

- **`Camera::position` is the view's TOP-LEFT corner**, not the center (`view = [pos.x, pos.x + w/zoom] × [pos.y, pos.y + h/zoom]`, Y down — `src/camera.rs`), but **sprites are CENTERED** at `Transform.position` (unit quad ±0.5). Mixing the two silently frames the wrong region → a **BLACK render target** (P4's first smoke bug). Center an `OffscreenCamera` via `Camera::new(scene_center - Vec2::new(rt_w/2, rt_h/2), zoom)`; for the main camera at `ZERO`, on-screen sprites live in `[0,win_w]×[0,win_h]` (positive coords). **A black RT = camera framed off the scene, NOT a pipeline failure.**
- **Sprite `Color` is f32 and `to_array()` does NOT clamp** — values >1.0 reach the shader (only the u8 conversion clamps). This enables HDR demos (over-bright sprites) and the simplest exposure tonemap (display sprite colour = exposure).
- **rustdoc `redundant_explicit_links` under `-D warnings`** — when the linked item is in scope (imported), use the shorthand `[`Foo`]`, not `[`Foo`](path)`. The ONLY verify failure of the whole session (P1, the doc step).
- **New `web_sys::Foo` node type ⇒ add its feature** to `web-sys` in Cargo.toml (`BiquadFilterNode`+`BiquadFilterType` in P1, like `OscillatorNode` in seq 7). Else the wasm build fails E0599.
- **`ron` is NOT example-visible** (only `serde_json` is a dev-dep) — parse RON in an example via `engine::save::read_ron::<T>(&Path)` (returns `SaveError: Display`), not `ron::from_str`. `serde::{Deserialize, Serialize}` ARE available.
- **`MaterialRenderer::new` consumes the sprite `shader` + `camera_layout`** (`sprite.rs:~295`) — the format-cache builder recompiles the sprite shader on demand (the `pipeline_layout` is only borrowed, so it's kept).
- **Borrow pattern for a lazily-cached GPU resource used in a `&self` method:** ensure (`&mut self`) → select a `&T` (immutable borrow) → call the `&self` consumer passing the `&T` — two coexisting immutable borrows compile cleanly (P4's `record_draw_pass` wiring).
- **macOS synthetic-key smoke:** `set frontmost` THEN `keystroke`/`key code`; a readout frozen at its initial value = the key never landed (P2 wasted-smoke). Region-capture with `screencapture -x -o -R<x>,<y>,<w>,<h>` from `get {position, size} of front window` — the only legible way to read a small window. Key codes: Space=49, S=1, G=5, L=37, T=17, ArrowUp=126/Down=125/Left=123/Right=124.

## Evidence & data

**Verify-gate runs (`VERIFY_EXIT` read from the LOG, never piped):**

| Phase | Result | Note |
|---|---|---|
| P1 #1 | `101` | rustdoc `redundant_explicit_links` on an in-scope `[AudioEffect](path)` → shorthand |
| P1 #2, post-bump | `0` | green |
| P2 (×2), P3 (×2), P4 (×2) | `0` | all green first try; **the fmt-reflow trap never bit** (ran `cargo fmt` first each time) |

**wasm example builds (beyond the lib+bins gate):** P1 `settings_menu_game`+`audio_facade` exit 0; P2 `positional_audio`+`audio_facade` exit 0. (P3/P4 examples are native-only → not wasm-built.)

**Native real-play smokes (all process-alive + stderr 0):**
- P1 `audio_facade` G/L/T → readout `BGM: on`, tones played via rodio.
- P2 `positional_audio` → `playing: yes`, `play_at_on_channel`, source orbited to (130,148), pan −0.78 (after a focus retry — first attempt's Space didn't land).
- P3 `ron_registry` → 3 custom configs loaded + reloaded (Dragon HP240 / Goblin HP30 / Slime HP60, correct colours, `names()` sorted).
- P4 `hdr_render_target` (exposure 0.20) → **HDR monitor: dim-bg / olive-mid / WHITE-core distinct; LDR monitor: mid+core collapsed to one flat gray** (the 8-bit clamp at store time). No format-mismatch panic — the `Rgba16Float` offscreen pipeline rendered correctly.

**Merge commits:** P1 code `22e4108` / handoff `619aef2`; P2 `9c413a8` / `0fe2699`; P3 `77cbb20` / `effad03`; P4 `249c5f2` / `be24649`.

## User feedback & preferences

- **`/goal`**: P1→P4 in order, each phase test→handoff→merge, **final report only** (skip intermediate), autonomous (no per-step confirmation). Honored throughout.
- **Direction priority confirmed via the priority-ordered list I presented** (P1 settings_menu strongest VISION continuation, P4 HDR riskiest/last). The user accepted the ordering.
- **Standing merge-authority delegation** (`merge-authority-delegated`) — squash-on-green-CI, no per-PR re-confirm. Applied to all 8 PRs.
- **Conversation language:** user-facing reports in **Korean**, all artifacts/prompts/code/docs in **English** (`conversation-language-korean` + doc-language rule). Followed (chat Korean, all files English).
- **Evidence-first + catch verify traps** — read the real `VERIFY_EXIT` from the log, wasm-example builds beyond the standard gate, a native real-play smoke per phase. Expected rigor; delivered.
- The user values the **`/wrap` → skill/rule-candidate loop** (start of session) — record skills in memory the same turn (gitignored), be honest about weak candidates (the web-sys-feature + native-bus gotchas were judged already-covered and NOT recorded).

## Where We're Going

- **The P1→P4 carried backlog is COMPLETE and exhausted.** No carried direction remains.
- **Next session: read the wishlist board FIRST** (`../dungeon-merchant/docs/engine-wishlist.md`, ACTIVE empty, next ID EW-002). A new EW request is the top driver. **If empty, ASK for direction** — there's no backlog to fall back on.
- **Follow-ups surfaced this run (offer, NOT committed):**
  1. Ship `positional_audio` / `hdr_render_target` to the web (`/ship-wasm-example`) — hear the positional panner / see HDR in a browser.
  2. A real engine **tonemap/bloom pass** (P4 left tonemapping to the game).
  3. **Offscreen UI/material pipeline format-matching** (P4 left those surface-only; the offscreen pass is sprite-only).
  4. A **`wgpu` re-export** from the engine so a game naming `wgpu::TextureFormat` for `create_render_target_with_format` / `load_image_with_format` doesn't need a direct `wgpu` dep.
  5. `settings_menu` still has ONE native-only `cfg` (env_logger, non-audio) — only closeable with a cross-platform logging shim (out of scope, low value).

## Risks & Blockers

- **None blocking.** main clean + green at v0.56.0; every phase verified by native smoke.
- **HDR RT display needs game-side tonemapping** (documented; example shows the simplest exposure multiply). A general engine tonemap is a future feature.
- **Web paths untested by ear** — P1's low-pass (`BiquadFilter`) and P2's positional panner compile + wasm-build but no one heard them in a browser this session. Low risk (thin standard Web Audio calls).

## Quick Start for Next Session

```bash
git checkout main && git pull --ff-only        # expect main @ be24649 (this umbrella's PR) or later
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?" >> /tmp/verify.log; grep VERIFY_EXIT /tmp/verify.log   # must be 0 (read from the LOG)

# The P1→P4 carried backlog is DONE. Read the wishlist board FIRST:
cat ../dungeon-merchant/docs/engine-wishlist.md    # ACTIVE empty, next ID EW-002
# If a new EW request exists → that's the top driver. If empty → ASK for direction
# (or offer a follow-up from "Where We're Going" above). No carried backlog remains.

# Live state + gotchas: memory engine-current-state (seq 74) + MEMORY.md.
# Per-phase detail: HANDOFF_{facade-tone-channels-lowpass, facade-positional, ron-registry-pub, hdr-render-target}_2026-06-2{2,3}.md
```

---

## Related Handoffs (this chain)

- seq 7 — `HANDOFF_facade-play-tone_2026-06-22.md` (the run's starting point; listed the four carried directions)
- seq 8 (P1) — `HANDOFF_facade-tone-channels-lowpass_2026-06-22.md`
- seq 9 (P2) — `HANDOFF_facade-positional_2026-06-23.md`
- seq 10 (P3) — `HANDOFF_ron-registry-pub_2026-06-23.md`
- seq 11 (P4) — `HANDOFF_hdr-render-target_2026-06-23.md`
- seq 12 (this) — session umbrella

---

## Session Closed

**Closed at:** 2026-06-23
**Session status:** the `/goal` P1→P4 carried-direction run is **COMPLETE** — all four features + examples + per-phase handoffs landed on `main` (v0.52.0 → v0.56.0), each verified by a native real-play smoke. Memory `engine-current-state` at seq 74; carried backlog exhausted; next session is wishlist-board-driven.
