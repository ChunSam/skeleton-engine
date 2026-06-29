# Hardcoding-audit closed (final Tier-2 + real Tier-3) → pivot to new breadth features (SpriteFlip, YSort): 5 merged feature/cleanup PRs + 1 docs PR

**Date:** 2026-06-29
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `hardcoding-audit` seq `4`
**Parent:** `HANDOFF_hardcoding-audit_tier2-fork-knobs_2026-06-28.md` (seq 3 — Tier-2 fork-config knobs #262–#264)
**Prior chain:** `HANDOFF_hardcoding-audit_2026-06-26.md` (seq 1, Tier-1) > `HANDOFF_hardcoding-audit_2026-06-27.md` (seq 2, physics/timing) > `HANDOFF_hardcoding-audit_tier2-fork-knobs_2026-06-28.md` (seq 3, fork-config knobs) > this (seq 4, **audit closed + breadth pivot**)

## Related Handoffs

- `HANDOFF_headless-screenshot_2026-06-28.md` — `headless-screenshot` chain seq 1 (#259–#261). Not a parent, but its `App::save_screenshot_headless` is what every example this session used for `HEADLESS_SHOT` pixel-verification (SpriteFlip / YSort render checks on Metal).

---

## Since Last Handoff

Parent seq 3 closed pointing at the `docs/HARDCODING_AUDIT_2026-06-26.md` "Open — Tier 2" list (it named `STICK_ACTIVATE/RELEASE`, `READ_TIMEOUT`, MAX_GAMEPADS, `desired_maximum_frame_latency`, editor `APP_ID` as remaining candidates) and "read the wishlist board first, ASK if empty." This session executed that and then ran the chain to its natural end:

- **Wishlist board still ACTIVE EMPTY** (EW-004 next free) every time it was checked → drove direction by AskUserQuestion each step.
- Landed the **two cleanest remaining Tier-2 knobs** — `StickNavConfig` (#267, the parent's `STICK_ACTIVATE/RELEASE` item) and `NetworkConfig::read_timeout` (#268, the parent's `READ_TIMEOUT` item).
- **Declared Tier-2 effectively complete** and refreshed the audit doc (#269) — the 5 truly-remaining knobs are all invasive / hardware-gated / editor-only (documented as such).
- User picked **Tier-3 dedup** (#270) → did only the *genuine* duplications; flagged two audit entries as false-positives (left deliberately).
- User then asked to **verify whether the remaining Tier-3 is real functional improvement** → answer: no (mostly cosmetic). This closed the hardcoding-audit chain.
- **Pivoted to new VISION breadth features**: `SpriteFlip` (#271) and `YSort` (#272) — both real gaps a downstream game had hand-rolled.
- The environmental audio-test failure (parent's "new wrinkle") **recurred all session** — same 2 audio-device tests, handled identically (`--skip` + let CI gate).

## Reference Documents

- `docs/HARDCODING_AUDIT_2026-06-26.md` — the audit's found→fixed record. Now has a **Tier-2 shipped** table incl. #267/#268, an **Open — Tier 2** table rewritten as an actionable best-first list (5 remaining, with effort/CI-verifiability/tradeoff columns), and a **Tier-3 resolution log** incl. #270 + the deliberate non-dedup decisions.
- `CLAUDE.md` — module-map rows added/updated for all five features; header bumped per PR to **v1.6.168** (package **v0.82.0**).
- `~/.claude/.../memory/engine-current-state.md` — meticulously bumped seq **110 → 115** (one entry per PR; the per-seq body is the most detailed record of this session).
- `../dungeon-merchant/docs/engine-wishlist.md` — the game↔engine board; ACTIVE EMPTY (EW-004 next).
- **Worked examples shipped this session** (read as templates for the next breadth feature): `examples/sprite_flip.rs` (runtime-generated texture + a controllable component), `examples/ysort.rs` (a system + a Space-toggle showing the before/after), `examples/ui_nav_deadzone.rs` (runtime-tunable config resource + works-without-hardware).
- `src/parallax.rs` — the reference "self-contained component + user-added System + tests" module shape that `src/ysort.rs` mirrors.

## The Goal

Two-phase session. **Phase A (continuation):** finish the hardcoding-audit — close the Tier-2 "a fork must edit engine source" knobs and the genuine Tier-3 magic-number duplications, each additive/default-preserving (non-breaking) with a playable example (the VISION acceptance test). **Phase B (pivot):** once the audit reached diminishing returns, add genuinely-missing 2D-engine breadth features (sprite flip, top-down y-sort) that downstream games had to hand-roll. End state: every clean fork-config knob shipped, and two universal top-down/2D primitives now built in.

## Where We Are

- **main @ `c02c65a`** (package **v0.82.0**, CLAUDE.md header **v1.6.168**), tree **clean**, **no open PRs**.
- **6 PRs merged this session** (all squash + branch-deleted, CI green each):
  - #267 `StickNavConfig` (v0.79.0), #268 `NetworkConfig::read_timeout` (v0.80.0), #269 docs Tier-2-list refresh (no bump), #270 Tier-3 dedup (v0.80.1), #271 `SpriteFlip` (v0.81.0), #272 `YSort` (v0.82.0).
- **New public API** (all additive, non-breaking): `engine::StickNavConfig` + `DEFAULT_STICK_ACTIVATE`/`DEFAULT_STICK_RELEASE`; `NetworkConfig::read_timeout` + `engine::network::DEFAULT_READ_TIMEOUT`; `engine::resources::DEFAULT_WINDOW_WIDTH`/`DEFAULT_WINDOW_HEIGHT`; `engine::SpriteFlip` + `UvRect::flipped(x,y)`; `engine::YSort` + `engine::YSortSystem`.
- **New examples:** `ui_nav_deadzone`, `sprite_flip`, `ysort` (+ `mp_client` now uses `connect_with_config`).
- **New files:** `src/ysort.rs` (component+system+tests), `examples/{ui_nav_deadzone,sprite_flip,ysort}.rs`.
- **Tests:** lib at **957 passed** (skipping 2 environmental audio), up from ~951 at session start (added: StickNav ×2, network ×3, UvRect::flipped ×1, YSort ×2, plus doctests for StickNavConfig/NetworkConfig/SpriteFlip/YSort).
- **CI:** all 6 PRs passed the 5-job matrix (Test native / Build WASM / Render lavapipe / Rustdoc / Package dry-run). The lavapipe render job (seq-110 infra) gates the GPU paths.
- **Memory** `engine-current-state.md` reflects seq 115 as the tip; older seqs 69–73 folded into the "Older seqs" rollup to keep the file compact.
- **Hardcoding-audit chain is effectively CLOSED.** `docs/HARDCODING_AUDIT_2026-06-26.md` Tier-1 (done seqs 96–99), Tier-2 (done across seqs 100–113, incl. this session's #267/#268), and the genuine Tier-3 dedup (#270) are all shipped. The "Open — Tier 2" table's 5 remaining rows are documented as invasive/hardware-gated/editor-only.
- **`mp_client` example** is the first/only call site of `NetworkClient::connect_with_config` in the examples (uses 2ms read_timeout).
- **No new `cfg`-gated or OS-specific code** this session — everything is cross-platform (wasm builds clean; the network knob is a documented native-only no-op on wasm, not a cfg split).
- **`scripting.rs` was deliberately NOT touched** — the `64*1024` Tier-3 "duplication" is a false positive (Rhai string limit ≠ network message cap), recorded in the audit doc.
- **Example count grew by 3** (`ui_nav_deadzone`, `sprite_flip`, `ysort`) — each flat/auto-discovered with `HEADLESS_SHOT` support.

### #267 — `StickNavConfig`, v0.79.0 (seq 111)
Opt-in resource `StickNavConfig { activate: f32, release: f32 }` (default 0.6/0.35 via `DEFAULT_STICK_ACTIVATE`/`DEFAULT_STICK_RELEASE`, all re-exported at crate root). `resolved() -> (f32,f32)` clamps `activate∈[0,1]`, `release∈[0,activate]`. Auto-inserted in `core_resources::insert_core_resources`. `InputSnapshot::from_world` reads it each frame (`unwrap_or_default().resolved()`) and passes the pair to `StickNav::update(x,y,activate,release)` → `step_axis(latched,value,activate,release)`. The module-private consts were deleted (moved to `focus.rs` as the public `DEFAULT_*`). Example `ui_nav_deadzone`. Fully CI-verifiable (pure logic).

### #268 — `NetworkConfig::read_timeout`, v0.80.0 (seq 112)
`read_timeout: Duration` field on the existing `NetworkConfig` (default `DEFAULT_READ_TIMEOUT = 5ms`). Native-only effect: `network/native.rs` extracts `config.read_timeout.max(1ms)` before `thread::spawn`, uses it for the plain-TCP and rustls-TLS `set_read_timeout`. WASM `wasm_impl.rs::connect_with_config` doesn't read it (event-driven). Example `mp_client` switched to `connect_with_config`. Field-add is safe — grep confirmed no `NetworkConfig {…}` struct-literal outside the definition.

### #270 — Tier-3 magic-number dedup, v0.80.1 (seq 113)
`DEFAULT_WINDOW_WIDTH`/`DEFAULT_WINDOW_HEIGHT` (`resources/display.rs`, re-exported from `engine::resources` — NOT crate root, so PATCH per the #248 precedent) used by both `WindowConfig::default` and `ViewportSize::default`. `UI_SUBLAYER_Z_STEP` (`ui/system.rs`, `pub(super)`) used by checkbox + slider passes. `MIN_AUDIO_DURATION_SECS` (`audio.rs`) used by the two fade-floor sites in `audio/types.rs` + `audio/bus.rs`. All value-identical (behavior-preserving). Audit doc updated with the resolution log + the two deliberate-non-dedup decisions.

### #271 — `SpriteFlip`, v0.81.0 (seq 114)
Dedicated component `SpriteFlip { x: bool, y: bool }` (ctors `horizontal()`/`vertical()`/`NONE` + `is_flipped()`; `Copy`/`Default`/serde). `UvRect::flipped(flip_x,flip_y)` composes the formerly-unused `flipped_x`/`flipped_y`. Applied in `collect.rs` to all 3 sprite paths (plain incl. crossfade `to`, atlas, material); NineSlice keeps the raw uv. Registered like `RenderLayer` (clone + editor add/remove). Example `sprite_flip` (4-quadrant texture). lavapipe render job + Metal headless verify.

### #272 — `YSort`, v0.82.0 (seq 115)
`src/ysort.rs`: `YSort { bias: f32 }` (`new`, `Copy`/`Default`/serde) + `YSortSystem` (user-added, like `ParallaxSystem`) iterating `query2_mut::<YSort,Transform>()` to set `z = position.y + bias`. Registered like `RenderLayer`. Example `ysort` (overlapping trees + player + Space toggle). Sign verified against `camera.rs` (Y down) + `sort.rs` (ascending z = front).

## What We Tried (Chronological)

1. **Read handoff + wishlist board** → board ACTIVE EMPTY. AskUserQuestion → user: **"Tier-2 하드코딩 노브 계속"**.
2. **#267 `StickNavConfig` (seq 111, v0.79.0).** The analog-stick → UI-focus-nav hysteresis thresholds (`STICK_ACTIVATE 0.6`/`STICK_RELEASE 0.35`) were module-private consts in `ui/system/state.rs`. Added opt-in `StickNavConfig { activate, release }` in `ui/focus.rs` (beside `FocusRingStyle`), auto-inserted in `core_resources.rs` with the historical defaults → byte-identical untouched. `resolved()` clamps `release∈[0,activate]`, `activate∈[0,1]` so a misconfig can't invert the band. Threaded the resolved pair through `StickNav::update`/`step_axis` + `InputSnapshot::from_world`. Example `ui_nav_deadzone` (focus list + live −1..1 stick-X band/marker bar; `[`/`]`/`,`/`.` runtime tune; works without a gamepad). +2 unit tests + doctest. Headless-render-verified on Metal.
3. **"다음 Tier-2 노브 계속"** → I told the user honestly the remaining knobs are weaker than the prior four, AskUserQuestion (4 options w/ tradeoffs) → user: **"network read_timeout"**.
4. **#268 `NetworkConfig::read_timeout` (seq 112, v0.80.0).** Native `NetworkClient` thread woke on a hardcoded `const READ_TIMEOUT = 5ms` socket read timeout. Added `read_timeout: Duration` to the existing `NetworkConfig` + `DEFAULT_READ_TIMEOUT` (re-exported `engine::network::`). Plumbed into `network/native.rs`, clamped `≥1ms` (a zero timeout = blocking socket → wedges the loop). No-op on WASM (event-driven; `wasm_impl.rs` ignores it — mirrors `max_buffered_bytes`). Example `mp_client` uses `connect_with_config` w/ 2ms. +3 tests + `no_run` doctest.
5. **"남아있는 tier-2 작업 리스트 만들어줘"** → #269 (docs-only, no bump). Moved #267/#268 to the audit doc's shipped table; rewrote "Open — Tier 2" as an actionable best-first table (effort / CI-verifiability / tradeoff). Recommended treating the clean knobs as done.
6. **Presented 3 directions; user: "3 진행"** → #270 Tier-3 dedup (seq 113, v0.80.1). Deduped only the GENUINE same-logical-value duplications: `DEFAULT_WINDOW_WIDTH/HEIGHT` (1280×720 in both `WindowConfig::default` + `ViewportSize::default` — cross-struct drift), `UI_SUBLAYER_Z_STEP` (0.001 in checkbox+slider pass), `MIN_AUDIO_DURATION_SECS` (the 2 fade floors). PATCH because it mirrors the #248 `DEFAULT_CLEAR_COLOR` precedent (a `resources::`-only pub const = PATCH).
7. **"3번 작업이 실질적인 코드의 기능 개선에 도움이 되는지 검증"** → verified the REMAINING Tier-3 against the code (not asserted). Verdict: mostly cosmetic (no functional gain); editor i18n gap = only real-but-tiny value; the particle CPU/GPU default-velocity-sign "drift" is NOT a bug (defaults differ, application is consistent — risky to "fix"). This closed the audit.
8. **"2번으로 진행"** → #271 `SpriteFlip` (seq 114, v0.81.0). New breadth feature. (See Key Decisions for the component-vs-field call.)
9. **"다른 기능 이어서 진행"** → #272 `YSort` (seq 115, v0.82.0). New breadth feature.
10. **`/handoff 하고 머지`** → this doc.

### Investigation done before building (most expensive to re-discover)

- **SpriteFlip gap:** grepped `Sprite` struct (3 fields, no flip), `renderer/sprite/` (no flip), `YSort`/`y_sort` (none). Found `UvRect::flipped_x`/`flipped_y` EXIST but have **zero usages** anywhere → they were latent primitives waiting for a consumer. Confirmed `Sprite {` literals are all `SpriteRenderKind::Sprite` (enum variant), so adding fields would've been safe — but chose a separate component anyway (atlas coverage).
- **Sprite UV flow:** traced `InstanceRaw` build sites — ALL in `collect.rs` (not `draw.rs`, which only records the GPU pass). 3 paths each compute a `uv` → single application point.
- **YSort sign:** read `camera.rs` (`view_proj = orthographic_rh(0,w,h,0,-1,1)`, doc "Y increases downward") + `sort.rs` (`(layer,z)` ascending) → `z = y + bias` proven correct, not guessed.
- **NetworkConfig field-add safety:** grepped for `NetworkConfig {` struct-literals → none outside the def → defaulted field is non-breaking. Confirmed `read_timeout` would be ignored by wasm (`wasm_impl.rs` reads only message/event/buffered fields).
- **Tier-3 semantics:** read every `0.001` site in `src/audio/` (2 floors vs ~9 epsilons of different units) and both `64*1024` sites (WS cap vs Rhai string limit) BEFORE deduping → avoided coupling unrelated values.
- **Registration template:** confirmed `RenderLayer` = serde-derive + `register_clone` + editor add/remove, **no Reflect** → mirrored exactly for SpriteFlip + YSort.

## Key Decisions

- **Surfaced "the remaining Tier-2 knobs are weaker" BEFORE picking** (step 3) instead of silently picking one. The earlier four knobs were clean field+example+test; the rest are invasive (material params >4 breaks the WGSL `vec4<f32>` contract), hardware-gated (MAX_GAMEPADS needs >4 physical pads), editor/tooling-only (app-id, rot-gizmo), or load-bearing+unverifiable (`desired_maximum_frame_latency:2` — its default is documented as fixing a macOS AppKit-blocking bug). Let the user choose with eyes open.
- **Tier-3: dedup only genuine duplications; flag false-positives.** The `0.001` audio sites are a MIX — 2 are duration *floors* (`max(0.001)`, deduped as `MIN_AUDIO_DURATION_SECS`) but ~9 are comparison *epsilons* of **different units** (attack/release seconds vs `pan`/`pitch` ratios) — coupling them under one const would be semantically wrong, so left. `64*1024` in `network/event.rs` (WS message cap, already named) vs `scripting.rs` (Rhai string limit) **coincidentally match but are unrelated** → a shared const would couple unrelated limits → left. Blind dedup is the Tier-3 trap; verified semantics first.
- **`SpriteFlip` is a DEDICATED component, not a `Sprite.flip_x` field.** `AtlasSprite` and `ShaderMaterial` entities have no `Sprite` component, so a field on `Sprite` couldn't flip atlas/animated/material sprites. A separate component flips all three uniformly (applied once per entity in `collect.rs`) and is purely additive (no `Sprite` struct/serde/Reflect change). Rejected: adding `flip_x/flip_y` to `Sprite` (narrower) and to `AtlasSprite` too (duplication).
- **Flip via UV-region swap, no shader change.** `UvRect::flipped` composes the pre-existing (but UNUSED) `flipped_x`/`flipped_y` primitives (`offset += size; size = -size`). The sprite shader's `uv_offset + vertex_uv*uv_size` then samples the region reversed, staying within the sub-rect so atlas tiles don't bleed. `flipped(false,false)` is a byte-identical no-op. **NineSlice excluded** (a 9-patch has no meaningful mirror — uses the raw uv).
- **YSort sign verified against the code, not assumed.** `camera.rs` doc: "Y increases downward"; `renderer/sprite/sort.rs`: ascending `(layer, z)` sort → higher z drawn last = in front. Therefore `z = position.y + bias` (lower-on-screen/larger-Y → higher z → front) is correct. Getting the sign backwards would silently invert overlap.
- **YSort + SpriteFlip registered like `RenderLayer`** (clone + editor add/remove + serde-derive, **no Reflect impl** — RenderLayer has none). Mirrored the closest existing simple component exactly rather than over-integrating.
- **Versioning:** additive feature = MINOR (0.79/0.80/0.81/0.82); the Tier-3 const dedup = PATCH (0.80.1, per the `DEFAULT_CLEAR_COLOR` #248 precedent that a trivial `resources::`-pub const is PATCH); docs-only = no bump (#269).
- **`flipped` involution test dropped the bit-exact double-flip assertion** — float accumulation (`0.1 + 0.3 - 0.3 ≠ 0.1`) made it fail; kept the composition + no-op assertions.

## Evidence & Data

### Commits landed (main, oldest→newest this session)

| Hash | PR | Version | Seq | Summary |
|---|---|---|---|---|
| `cdb9475` | #267 | v0.79.0 | 111 | `StickNavConfig` analog-stick UI-nav deadzone |
| `9eaf582` | #268 | v0.80.0 | 112 | `NetworkConfig::read_timeout` native socket poll |
| `289be2e` | #269 | (docs) | — | refresh remaining Tier-2 list in audit doc |
| `642c3f5` | #270 | v0.80.1 | 113 | Tier-3 magic-number dedup (named consts) |
| `7b15ede` | #271 | v0.81.0 | 114 | `SpriteFlip` sprite mirroring |
| `c02c65a` | #272 | v0.82.0 | 115 | `YSort` top-down depth sorting |

### New public API surface (all additive / non-breaking)

| Symbol | Kind | Location | PR |
|---|---|---|---|
| `StickNavConfig { activate, release }` | resource | `ui/focus.rs` | #267 |
| `StickNavConfig::resolved()` | method | `ui/focus.rs` | #267 |
| `DEFAULT_STICK_ACTIVATE` / `DEFAULT_STICK_RELEASE` | const (crate root) | `ui/focus.rs` | #267 |
| `NetworkConfig::read_timeout` | field | `network/event.rs` | #268 |
| `network::DEFAULT_READ_TIMEOUT` | const | `network/event.rs` | #268 |
| `resources::DEFAULT_WINDOW_WIDTH` / `_HEIGHT` | const | `resources/display.rs` | #270 |
| `SpriteFlip { x, y }` + `horizontal()`/`vertical()`/`NONE`/`is_flipped()` | component | `components.rs` | #271 |
| `UvRect::flipped(x, y)` | method | `renderer/uv.rs` | #271 |
| `YSort { bias }` + `YSort::new(bias)` | component | `ysort.rs` | #272 |
| `YSortSystem` | system | `ysort.rs` | #272 |

Crate-root re-exports added to `lib.rs`: `StickNavConfig`, `DEFAULT_STICK_ACTIVATE`, `DEFAULT_STICK_RELEASE`, `SpriteFlip`, `YSort`, `YSortSystem` (+ `pub mod ysort`). `DEFAULT_READ_TIMEOUT` lives at `engine::network::` and `DEFAULT_WINDOW_*` at `engine::resources::` (matching their sibling consts' visibility, not crate root).

### Version progression

`v0.78.1` (session start, `16f11a9`) → `v0.79.0` → `v0.80.0` → `v0.80.1` → `v0.81.0` → `v0.82.0` (`c02c65a`). CLAUDE.md header `v1.6.163 → v1.6.168`.

### CI (all green — 5-job matrix; native test gates audio)

- #267 5/5 (native 5m9s) · #268 5/5 (native 4m36s) · #269 5/5 · #270 5/5 · #271 5/5 (native 4m55s) · #272 5/5.
- The native job runs the 2 audio-device tests that fail locally → green CI confirms they're environmental.

### Tests added

- `ui/system/state.rs`: `custom_tight_deadzone_fires_earlier`, `config_resolved_clamps_invalid_thresholds`.
- `network/tests.rs`: `network_config_defaults` (extended for read_timeout), `network_config_carries_custom_read_timeout`, `connect_with_custom_read_timeout_constructs`.
- `renderer/uv.rs`: `flipped_composes_axes_and_is_noop_when_both_false`.
- `ysort.rs`: `z_is_y_plus_bias`, `lower_entity_gets_higher_z_so_it_draws_in_front`.
- Doctests: `StickNavConfig`, `NetworkConfig` (`no_run`), `SpriteFlip`, `YSort`.

### Headless render verification (Metal, `HEADLESS_SHOT`)

- `ui_nav_deadzone`: focus ring + deadzone band/marker bar render; thresholds shown.
- `sprite_flip`: 4-quadrant texture (TL red / TR green / BL blue / BR yellow); flipped copy mirrors columns exactly (green/red·yellow/blue).
- `ysort`: 3 overlapping green "trees" — lower occludes upper; red player (y=262) over the middle tree, behind the lower tree → correct depth weave.

Reproduce any of them headlessly (native, needs a GPU — works monitor-off via the seq-105 surfaceless path):
```bash
HEADLESS_SHOT=/tmp/deadzone.png cargo run --example ui_nav_deadzone
HEADLESS_SHOT=/tmp/flip.png     cargo run --example sprite_flip
HEADLESS_SHOT=/tmp/ysort.png    cargo run --example ysort
# HEADLESS_FRAMES=N overrides the default 4-frame warmup before the capture.
```

### The environmental audio-test failure (recurred, as in seq 3)

- `audio::tests::play_tone_reports_playing_then_finished_when_audio_device_exists` + `stop_on_drained_sink_is_immediate` FAIL locally (`Playing != Finished` / "tone should have drained") — this locked/remote macOS session has no audio device.
- `verify.sh` returns **101** purely from these two; the background-task notification still reports "exit code 0" (the trailing-echo masking gotcha). Authoritative verdict is the in-log `VERIFY_EXIT=`. Handled by `cargo test ... -- --skip <both>` + running clippy/wasm/doc/doctest separately, then letting CI gate audio. None of this session's changes touch audio.

## Code Analysis

- **Sprite UV pipeline** (`renderer/sprite/`): `collect.rs` builds `InstanceRaw` in 3 paths — plain `Sprite` (incl. `BlendUv` crossfade `to` frame + NineSlice sub-quads), `AtlasSprite`, `ShaderMaterial`. `geometry.rs::InstanceRaw::{single,blended,from,from_global}` map a `UvRect` to the instance; the shader computes `uv_offset + vertex_uv*uv_size`. Flip is applied to the `uv` in collect.rs (not the shader).
- **`UvRect`** (`renderer/uv.rs`): `{u_offset,v_offset,u_size,v_size}`; had unused `flipped_x`/`flipped_y`; added `flipped(x,y)`.
- **Sort** (`renderer/sprite/sort.rs`): `a.layer.cmp(b.layer).then(a.z.partial_cmp(b.z)).then(a.order)` — ascending; drawn in order → higher z on top.
- **Camera** (`camera.rs`): top-left anchored, **Y increases downward**, `view_proj = orthographic_rh(0,w,h,0,-1,1)`.
- **`NetworkConfig`** (`network/event.rs`): `#[derive(Clone,Copy,Debug,PartialEq,Eq)]`; no struct-literal anywhere outside its def (grep-verified) → field-add safe. Native thread plumbing in `native.rs::connect_with_config` extracts config fields into locals before `thread::spawn`.
- **`query2_mut::<A,B>()`** (`ecs/world/queries.rs`): returns `(Entity, &mut A, &mut B)` via `get_disjoint_mut` on distinct archetype columns — used by `YSortSystem`.
- **Component registration pattern:** `core_resources.rs` `register_clone::<T>()` + `editor/component_registry.rs` `register_component`/`register_component_remover`. `RenderLayer` is the canonical simple-component template (serde-derive, no Reflect).
- **`StickNav` edge detector** (`ui/system/state.rs`): `step_axis(latched: &mut i8, value: f32, activate: f32, release: f32) -> i8` — hysteresis (latch past `±activate`, neutral inside `±release`, hold between). `StickNav::update(x,y,activate,release)` calls it per axis. Now threshold-parameterized (was const-driven).
- **`StickNavConfig::resolved()`** clamps `activate = activate.clamp(0,1)`, `release = release.clamp(0, activate)` — guarantees `release ≤ activate` so the neutral band can't invert (degrades to a hard threshold if equal).
- **`NetworkConfig`**: `{ max_message_bytes, max_pending_messages, max_pending_events, max_buffered_bytes: Option<u32>, read_timeout: Duration }`. Defaults via `DEFAULT_MAX_*` + `DEFAULT_READ_TIMEOUT`. `connect_with_config` exists on BOTH native (`native.rs`) and wasm (`wasm_impl.rs`).
- **`SpriteFlip` application** (`collect.rs`): plain path flips inside the `make_instance` closure (so the NineSlice branch's raw `uv` is untouched) + flips the `BlendUv` `b.to`; atlas + material paths flip the `uv` after computing it. `world.get::<SpriteFlip>(e).copied().unwrap_or_default()`.
- **`YSortSystem::run`** is 1 line: `for (_e, ysort, transform) in world.query2_mut::<YSort, Transform>() { transform.z = transform.position.y + ysort.bias; }`. Operates on the entity's own `Transform` (un-parented top-down case); for hierarchy entities run it before `HierarchySystem`.
- **Render `z` is a pure sort key** (painter's algorithm, no depth buffer — `begin_color_pass` is `LoadOp::Load`), so any finite `z` magnitude is fine; YSort writing `z = y` (range 0..viewport) needs no normalization. Mixing y-sorted + non-y-sorted in one `RenderLayer` is the caller's concern (use layers to separate a static background).

## Gotchas & Discoveries

- **`verify.sh` exit masking (recurring):** a backgrounded `./scripts/verify.sh > log 2>&1; echo "VERIFY_EXIT=$?"` makes the task-completion notification report the trailing `echo`'s exit `0`, hiding `verify.sh`'s real `101`. ALWAYS read `VERIFY_EXIT=` from the log, not the notification. This session it was `101` (audio) on every full run — confirmed harmless by `--skip`ing the 2 audio tests and running the other gates separately, then CI.
- **Float involution isn't bit-exact:** `UvRect::flipped(true,true).flipped(true,true) == original` FAILS (`0.1 + 0.3 - 0.3 = 0.099999994`). Don't assert round-trip equality on flip/transform composition; assert the composition shape + the no-op case instead.
- **`grep 'Sprite {'` false positive:** matches `SpriteRenderKind::Sprite { … }` (a renderer enum variant), NOT the `Sprite` component struct-literal. The component is only built via constructors → field-add safe, but the grep needs a human read.
- **`UvRect::flipped_x`/`flipped_y` were dead pub API** (zero usages) until `SpriteFlip` consumed them — a latent primitive. Worth grepping for more of these when adding a feature (the building block may already exist).
- **Camera is top-left-anchored, Y down** (`camera.rs`): world `+y` = screen down. Examples that place sprites by window fraction must insert `Camera::new(Vec2::ZERO, 1.0)` (the `blend_locomotion`/`sprite_flip`/`ysort` convention) so world coords == window pixels.
- **`image` crate is a normal dependency** (0.25, png) usable from examples — `sprite_flip` generates its 4-quadrant texture at runtime to `std::env::temp_dir()` then `load_image`s it (the `gen_*`-example pattern), so no committed binary asset.
- **PATCH vs MINOR for a pub const:** the project ships a new `resources::`-scoped (non-crate-root) default const as PATCH (the #248 `DEFAULT_CLEAR_COLOR` precedent), but a crate-root pub const as MINOR (#262 `DEFAULT_MAX_LIGHTS`). #270's `DEFAULT_WINDOW_*` stayed `resources::`-only → PATCH.
- **Flat examples are auto-discovered** — `examples/<name>.rs` needs no `[[example]]` Cargo.toml entry (only multi-file/subdir examples do).

## Files Changed

### Source — new
- `src/ysort.rs` — `YSort` component + `YSortSystem` + 2 tests.

### Source — modified
- `src/components.rs` — `SpriteFlip` component (ctors, serde, doctest).
- `src/renderer/uv.rs` — `UvRect::flipped(x,y)` + test.
- `src/renderer/sprite/collect.rs` — apply `SpriteFlip` in all 3 paths (NineSlice excluded).
- `src/ui/focus.rs` — `StickNavConfig` + `DEFAULT_STICK_ACTIVATE/RELEASE`.
- `src/ui/system/state.rs` — `StickNav` reads resolved thresholds; +2 tests.
- `src/ui/system.rs` — `UI_SUBLAYER_Z_STEP` const.
- `src/ui/system/{checkbox_pass,slider_pass}.rs` — use the z-step const.
- `src/network/event.rs` — `read_timeout` field + `DEFAULT_READ_TIMEOUT` + doctest.
- `src/network/native.rs` — plumb `read_timeout` (clamped ≥1ms).
- `src/network.rs` — re-export `DEFAULT_READ_TIMEOUT`.
- `src/network/tests.rs` — +3 tests.
- `src/resources/display.rs` + `resources/mod.rs` — `DEFAULT_WINDOW_WIDTH/HEIGHT`.
- `src/audio.rs` + `audio/{types,bus}.rs` — `MIN_AUDIO_DURATION_SECS`.
- `src/scripting.rs` — (none — 64*1024 left as-is, false positive).
- `src/lib.rs` — re-exports (StickNavConfig, SpriteFlip, YSort/YSortSystem, `pub mod ysort`).
- `src/app/core_resources.rs` — auto-insert `StickNavConfig`; `register_clone` for SpriteFlip + YSort.
- `src/app/editor/component_registry.rs` — editor add/remove for SpriteFlip + YSort.

### Examples — new
- `examples/ui_nav_deadzone.rs`, `examples/sprite_flip.rs`, `examples/ysort.rs`.
- `examples/mp_client.rs` — now `connect_with_config` w/ 2ms read_timeout.

### Docs / paperwork
- `docs/HARDCODING_AUDIT_2026-06-26.md` — Tier-2 shipped/open + Tier-3 resolution log.
- `docs/CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`, `CLAUDE.md` — per-PR version paperwork.

## User Feedback & Preferences

- **Drives direction by short Korean go-aheads** ("계속", "3 진행", "2번으로 진행", "다른 기능 이어서 진행") — expects me to pick the concrete work and execute the full land-pr loop autonomously.
- **Values honest cost/benefit triage**: when asked "다음 Tier-2 노브 계속" I surfaced that the remainder is weaker → user appreciated choosing with tradeoffs visible (picked read_timeout).
- **Wants verification, not assertion**: "3번 작업이 실질적인 코드의 기능 개선에 도움이 되는지 검증" — explicitly asked to PROVE (with code) whether remaining Tier-3 helps. Reward: an evidence-backed "mostly cosmetic" verdict that redirected to breadth features.
- **Korean for all user-facing replies**; English for code/docs/commits (project doc-language rule).
- **Merge authority delegated** (squash on green CI, no re-confirm) — every PR this session was landed without asking.
- Standing env reality: **locked/remote macOS, no audio device** → the 2 audio tests always fail locally; never treat as a regression.
- **Prefers the full land-pr loop per change** — branch → verify → /ship paperwork → PR → watch CI → squash-merge → sync → bump memory seq — run without narration of each option; just report the outcome.
- **Per-session `/handoff` + merge cadence** — the handoff doc lands as its own docs PR (this one), consistent with the seq-2/seq-3 precedent.
- **Comfortable with multiple small PRs in one session** (6 this session) — does not want them bundled; one coherent change per PR.

## Where We're Going

The hardcoding-audit chain is **closed** (Tier-2 + genuine Tier-3 done; remainder documented as not-worth-it). Forward direction = **new VISION breadth features** (the seq 114/115 pivot) until the game wishlist fills. Ordered candidates (each: additive component/system + playable example + tests, land via the land-pr loop):

1. **Animation events** (recommended default) — fire on reaching a frame. Sketch: a component like `AnimationEvents { events: Vec<(usize /*frame*/, String /*tag*/)> }` (or per-clip in `AnimationClip`); `AnimationSystem` detects a frame-advance crossing an event frame and pushes to `Events<AnimationEvent { entity, tag }>`. Read `src/animation/player.rs` + `system.rs` first (the player already tracks current frame + advances it — hook the crossing there). Verify which has no facility (grep `AnimationEvent`). Example: a walk cycle emitting "footstep" on the contact frames + a HUD/log counter; CI-verifiable via a unit test on the frame-crossing logic.
2. **Trigger zones** — `Area2D`-style enter/stay/exit. `rapier` `CollisionEvent`/`TriggerEvent` exist but are physics-bound; a lightweight non-physics overlap trigger (AABB/circle vs `SpatialGrid`, emitting `Events<TriggerEnter/Exit>`) for pickups/doors/regions does not. Read `src/collision/` (`SpatialGrid`, `Collider`, `CollisionLayer`) — likely a new `TriggerZone` component + system diffing per-frame overlap sets. CI-verifiable via overlap-set unit tests; example a player walking through pickup zones.
3. **Re-check `../dungeon-merchant/docs/engine-wishlist.md`** FIRST each session (ACTIVE EMPTY, EW-004 next) — a real downstream request outranks self-picked breadth.
4. Lower value: remaining low-value Tier-3 (editor i18n gap in `editor/ui/audio_panel.rs:34,43` = the only real-but-tiny one — two English status strings bypass `tr()` in the Korean-default editor), or the deferred invasive Tier-2 knobs only if a fork actually needs them.

How SpriteFlip + YSort were found (reusable gap-finding method): the engine is now **very broad** (the CLAUDE.md module map covers nearly every 2D subsystem), so net-new gaps are narrow. Both came from asking "what does a typical top-down / 2D game need that a downstream forced into game-code?" — `rust-survivors` had hand-rolled UV flipping (→ SpriteFlip) and there was no y-sort (→ YSort). Look for engine capabilities that are *almost* there (e.g. `UvRect::flipped_x` existed but unused) or that every genre re-implements (animation events, trigger zones, tweened camera lookahead, hit-flash). Grep first to confirm absence; don't rebuild what exists.

The shipped pattern to copy for any of these: **(a)** verify the gap by grep + reading the subsystem; **(b)** add a component (+ system if needed) in its own module or `components.rs`; **(c)** register clone + editor add/remove like `RenderLayer`; **(d)** re-export in `lib.rs`; **(e)** a flat `examples/<name>.rs` with `HEADLESS_SHOT` support (auto-discovered, no Cargo.toml entry); **(f)** unit tests + a doctest; **(g)** CLAUDE.md module-map row; **(h)** land via the land-pr loop (MINOR for a feature), `--skip` the 2 audio tests locally.

## Chain State

The `hardcoding-audit` chain (seq 1 Tier-1 → seq 2 physics/timing → seq 3 fork-knobs → **seq 4 this, closing**) has delivered: Tier-1 logic/dedup fixes, and Tier-2 fork-config knobs across `RenderTarget` filter, particle spawn cap, `one_way_tolerance`, solver iterations, `FrameConfig::max_dt`, `LightingConfig` (max lights), `Slider::keyboard_step`, `TileAnimationSet::stagger`, `StickNavConfig`, `NetworkConfig::read_timeout` — plus the genuine Tier-3 dedup. The chain should be considered **done**; a future session need not "continue the audit" — start from the wishlist board or a fresh breadth gap. If a new hardcoding need surfaces, it's a new one-off, not a chain revival.

## Risks & Blockers

- **Audio-device tests fail locally** (environmental, not a regression) — `--skip` them + read `VERIFY_EXIT=` from the log (the backgrounded `verify.sh` notification masks the real exit). CI gates audio.
- **OS-gated paths uncovered by CI** (macOS/editor/GPU-windowed) — none touched this session, but the standing rule holds: green CI ≠ verified for `cfg(target_os)` code.
- **`SpriteFlip` + hierarchy:** flip is applied per-entity in `collect.rs` regardless of `GlobalTransform`, so it works for parented sprites too (it's a UV op, not a transform). But `YSort` writes the entity's own `Transform.z` — for a *parented* y-sorted entity, run `YSortSystem` before `HierarchySystem` (documented in the YSort module doc). No current example exercises parented y-sort.
- **NineSlice + SpriteFlip** is intentionally unsupported (raw uv) — if a fork ever wants a mirrored 9-patch, that's a new ask, not a bug.
- No upstream/dependency blockers. Tree clean, no open PRs.

## Open Questions

- Which breadth feature next — **animation events** (recommended default) vs **trigger zones** vs something else? Confirm before building; the wishlist board (EW-004) takes precedence if it has filled.
- Should `DEFAULT_WINDOW_WIDTH/HEIGHT` and `DEFAULT_READ_TIMEOUT` be promoted to crate-root re-exports for discoverability? Left at module scope this session to match their siblings (`DEFAULT_CLEAR_COLOR` / `DEFAULT_MAX_MESSAGE_BYTES`) — revisit only if a fork finds them hard to locate.
- Is there appetite to tackle any *invasive* Tier-2 knob (e.g. variable-length material params, >4 gamepads)? They were deferred as low value-for-effort; only worth it on a concrete request.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -6            # confirm main @ c02c65a (v0.82.0)
git status -s                   # clean

# 1) Read the game↔engine board FIRST (drives priority)
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick breadth

# Memory (full per-seq detail of this session)
#   ~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md (seq 115 tip)

# Key files if continuing breadth features
#   src/ysort.rs + src/components.rs (SpriteFlip)  — the two patterns just shipped
#   src/animation/player.rs + system.rs            — for "animation events"
#   src/collision/ + src/ecs/events.rs             — for "trigger zones"
#   docs/HARDCODING_AUDIT_2026-06-26.md            — audit closed; remaining = low value

# Verify (NOTE: 2 audio-device tests fail locally — environmental)
cargo test --lib -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings

# Next action
#   Read the wishlist board; if still empty, ASK the user which breadth feature
#   (default: animation events), then land it via the standard land-pr loop.
```

## Session Closed

**Closed:** 2026-06-29
**Chain:** `hardcoding-audit` seq 4 (closing) — parent `HANDOFF_hardcoding-audit_tier2-fork-knobs_2026-06-28.md`
**Code landed:** #267–#272 (v0.79.0 → v0.82.0), main @ `c02c65a`. This handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off. The hardcoding-audit chain is complete; next session starts from the wishlist board or a new breadth feature (see "Where We're Going").
