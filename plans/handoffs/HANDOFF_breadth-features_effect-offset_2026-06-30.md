# Richer effect payload — per-effect `SpawnParticles` offset shipped (v0.90.0, PR #289), eighth breadth feature of the chain

**Date:** 2026-06-30
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `breadth-features` seq `8`
**Parent:** `HANDOFF_breadth-features_anim-effects_2026-06-29.md` (seq 7)
**Prior chain:** `hardcoding-audit` (closed) → pivoted to `breadth-features` at its seq 4

> Chain rationale: the seq-7 handoff (Animation→Effect bindings) flagged, twice (Where We're Going #2
> + Open Questions), that **a per-effect position offset** was "the natural next refinement" — so a
> footstep's dust spawns at the feet, not the entity center. The board was ACTIVE EMPTY and the user,
> after the editor-i18n candidate turned out already-done, picked exactly this. Direct continuation:
> `breadth-features` seq 8, parent = the seq-7 handoff.

## Related Handoffs

- `HANDOFF_breadth-features_anim-effects_2026-06-29.md` — **the parent** (`breadth-features` seq 7,
  #287). It introduced the shared `crate::effect` module + the `anim_effects` example this feature
  extends, and explicitly named the position-offset refinement under "Risks" and "Open Questions".
- `HANDOFF_breadth-features_zone-effects_2026-06-29.md` — seq 6 (#285); the first `Effect` consumer.
  The new `offset` field lives on the shared `Effect` vocabulary, so zone effects get it too (unused
  by the example, but available).

## Reference Documents

- `CLAUDE.md` — the Zone→Effect row's `SpawnParticles{…}` signature now lists `offset:(x,y)`; the
  Animation→Effect row notes the example spawns dust at the feet. Header → **v1.6.177** / package
  **v0.90.0**.
- `docs/CHANGELOG.md` — the 0.90.0 entry (Added) written this session.
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — bumped
  to **seq 123** on this handoff PR's landing (per the per-seq cadence; the most detailed per-seq
  record lives in that bullet).
- `../dungeon-merchant/docs/engine-wishlist.md` — the game↔engine board; **ACTIVE EMPTY** (next free
  ID EW-004). Read FIRST next session.
- `src/effect.rs` (the shared `Effect` + `resolve_effect`), `examples/anim_effects.rs` + `.ron`.

## The Goal

Take the seq-7 handoff's named refinement: give a data-authored particle burst a **position offset**
so it can spawn off-center (footstep dust at the feet, a muzzle flash at the barrel) instead of always
at the anchor's `Transform`. Acceptance test (VISION loop): the existing `anim_effects` example now
spawns its footstep dust at the walker's feet via the offset, visibly (a headless shot showing the
burst at a `◦ feet` marker, not the `＋ center` one).

## Where We Are

- **main @ `5aeeb42`** (package **v0.90.0**, CLAUDE.md header **v1.6.177**), tree **clean**.
  _(This handoff doc lands as its own `docs(handoff)` PR after the code PR; the recorded tip will
  move to that handoff merge — and the memory seq-bump to 123 lands with it.)_
- **PR #289 merged** (squash `5aeeb42` + branch-deleted, CI **5/5** green): `feat(effect): per-effect
  SpawnParticles offset — spawn bursts off-center (v0.90.0)`.
- **New public API** (additive / non-breaking): `Effect::SpawnParticles` gains
  **`offset: (f32, f32)`** (`#[serde(default)]` → `(0.0, 0.0)`). No symbol added/renamed/removed;
  `engine::Effect` keeps its path. Pre-1.0 additive → **MINOR** (0.89.0 → 0.90.0).
- **Applied in `resolve_effect`** (`src/effect.rs`): `pos = anchor.Transform.position +
  Vec2::new(offset.0, offset.1)`. World-space; **not** rotated/scaled by the anchor's transform
  (documented). Shared by **both** effect sources — `zone_effect` + `anim_effect` call the same
  `resolve_effect`, so neither system changed.
- **Example updated** `examples/anim_effects` — `anim_effects.ron`'s footstep `SpawnParticles` now
  carries `offset: (0.0, 70.0)` (the walker is at world (320,210), scale 180; feet ≈ +70 **down**,
  since `view_proj = orthographic_rh(0, w, h, 0, …)` makes world **Y increase downward**). The HUD
  draws `＋ center` / `◦ feet (offset)` guide markers (via `Camera::world_to_screen`) so the
  displacement is unmistakable. Closes the seq-7 "dust spawns at the entity center" limitation.
- **Tests:** new `anim_effect::tests::spawn_particles_offset_displaces_burst` (burst Transform ==
  anchor + offset, `(70,20)+(0,50) → (70,70)`); `zone_effect`'s default-parse test now also asserts
  `offset` defaults to `(0.0, 0.0)` (proves the byte-identical default).
- **CLAUDE.md** module-map rows updated; **CHANGELOG** 0.90.0 entry; **memory** to seq 123 (on the
  handoff PR).

## What We Tried (Chronological)

1. **Read the last handoff (seq 7) + the board.** Board ACTIVE EMPTY (EW-004 next). Per the rule,
   asked the user for direction.
2. **User first picked "editor i18n gap"** (a seq-7-listed thin candidate naming
   `editor/ui/audio_panel.rs`). **Investigated and found the gap already closed:** the whole editor
   was localized in **v0.46.0 (#174)** "Korean-by-default editor localization"; `audio_panel.rs` +
   every toolbar/tab/button/panel/status string already use `tr(en, ko)`. A definitive scan (every
   egui text API across `src/app/editor/**`) surfaced only false positives — `#[cfg(…"wasm32")]`
   attrs, test assertions, egui `id_source` strings, multi-line `tr(...)` argument lines, `log::warn!`
   developer diagnostics (English by rule), and **intentional type identifiers** (the `+ Add
   Component` combo's component-type names + `state_machine_panel`'s `TransitionCond` variant labels
   `BoolEq/FloatGt/…`). **The seq-7 handoff's i18n claim was stale by ~43 minor versions.** Reported
   this honestly (did not manufacture work); **corrected the memory candidate list** to mark i18n
   "NOT a gap". User chose **"완료로 간주, 다른 작업 선택"**.
3. **Proposed + user picked "richer effect payload — per-effect 위치 오프셋"** (the seq-7 named
   refinement) over a 3rd event→effect source (speculative without a game ask) or a Tier-2 knob.
4. **Designed the field** (see Key Decisions): `offset` belongs only on `SpawnParticles` (the sole
   positioned effect — `Flash` is a sprite tint, `PlayTone` has no position); `(f32, f32)` tuple to
   match the codebase's RON convention (`Flash.color` is a tuple; particle configs mirror `Vec2` as
   `(x,y)` even with glam's `serde` feature on, for RON readability); world-space.
5. **Edited `src/effect.rs`** — added the field + `#[serde(default)]`; applied `position + offset`
   in `resolve_effect`. Fixed the one literal `Effect::SpawnParticles { … }` **pattern** that didn't
   use `..` (`zone_effect`'s default-parse test) — added `offset` + a default assertion. The runtime
   `zone_effect` match (`at: EffectAnchor::Zone, ..`) and `anim_effect`'s test match already used `..`.
6. **Added the unit test** + extended the example RON + `anim_effects.rs` (FEET_OFFSET const mirroring
   the RON for the guide, `Camera::world_to_screen` center/feet markers, refreshed title/copy).
7. **Verify (gate-by-gate, exit codes read un-piped).** `cargo fmt` → fmt --check (0), clippy
   --all-targets (0), wasm build (0), wasm `--lib` clippy (0), `test --all-targets` skipping the 2
   audio tests (0; 995 lib + 12 + 12), `test --doc` (0; 84), `doc -D warnings` (0).
8. **Headless render on Metal** (`HEADLESS_SHOT`, 74 frames): console `footstep → fired its effects
   (total: 2)`; the shot shows the dust burst at the `◦ feet (offset)` marker (below the `＋ center`
   one), falling under gravity — the offset working. Sent to the user as the acceptance artifact.
9. **`/ship`** → v0.90.0 (MINOR): Cargo.toml + `cargo update -p skeleton-engine` (lock 0.90.0) +
   CHANGELOG 0.90.0 (Added) + CLAUDE.md header v1.6.177 + module-map rows; re-ran the bump-shiftable
   gates (fmt/wasm/doc) green.
10. **`/land-pr`** → branch `feat/effect-offset`, commit `723e436`, push, PR **#289**, watched CI
    (5/5 CLEAN), confirmed `mergeStateStatus == CLEAN`, squash-merged `5aeeb42`, synced main.
11. **This handoff** lands as its own `docs(handoff)` PR; **memory → seq 123** on its merge.

## Key Decisions

- **`offset` lives only on `Effect::SpawnParticles`.** It's the only positioned effect — `Flash` adds
  a `HitFlash` (a tint, no position) and `PlayTone` is non-spatial. Adding an offset to those would be
  dead weight. The shared `Effect` enum still carries it once; both sources inherit it.
- **`(f32, f32)` tuple, not `glam::Vec2`, in the RON-facing type.** The codebase convention is a
  tuple mirror for RON ergonomics/readability (`Flash.color` is `(f32,f32,f32,f32)`; `config_set.rs`
  mirrors every `Vec2` as `Vec2Def(f32,f32)` *despite* glam's `serde` feature being enabled). Matching
  it keeps RON authoring uniform (`offset: (0.0, 70.0)`); converted to `Vec2` once inside
  `resolve_effect`.
- **World-space offset, not transform-relative.** Simplest useful version; documented as not
  rotated/scaled by the anchor. A rotation-aware / facing-aware offset (so it follows a flipped/turned
  sprite) is a future refinement — deferred to avoid over-engineering from one call site (the
  seq-3 RemoteEntities lesson).
- **Applied in the shared `resolve_effect`, not per-system.** Keeps the two binding systems unchanged
  and the offset semantics identical across both sources. The zone system still computes its
  `particle_anchor` (entrant vs zone) before calling `resolve_effect`; the offset then displaces from
  whichever anchor it chose.
- **Default `(0.0, 0.0)` = byte-identical.** `#[serde(default)]` + the existing `system_spawns_burst_
  at_animating_entity` test (asserts the burst is exactly at the anchor with no offset) prove the
  non-offset path is unchanged.
- **Versioning: MINOR (v0.90.0).** Additive field, non-breaking, pre-1.0 → MINOR.
- **Did not manufacture i18n work.** The chosen candidate was already done; reported the finding with
  evidence and let the user redirect, rather than inventing a refactor.

## Evidence & Data

### Commit / PR (this session)

| Hash | PR | Version | Seq | Summary |
|---|---|---|---|---|
| `723e436` (→ squashed `5aeeb42`) | #289 | v0.90.0 | 123 | per-effect `SpawnParticles` offset |

### New public API surface (additive / non-breaking)

| Symbol | Kind | Location |
|---|---|---|
| `Effect::SpawnParticles.offset: (f32, f32)` | enum field (`#[serde(default)]` = `(0,0)`) | `src/effect.rs` |

### Tests

`anim_effect::tests::spawn_particles_offset_displaces_burst` (new): a `"footstep"` binding with
`offset: (0.0, 50.0)`, actor at `(70, 20)` → the spawned `ParticleBurst`'s Transform is `(70, 70)`.
`zone_effect::tests` default-parse: now asserts `offset == (0.0, 0.0)` on an omitted-field
`SpawnParticles`. Full `cargo test --all-targets` (audio-2 skipped) = **995 lib** + 12 + 12, 0
failed; `test --doc` 84 passed.

### CI (PR #289 — 5-job matrix, all green)

| Job | Result | Time |
|---|---|---|
| Test (native) | pass | 4m0s |
| Render tests (lavapipe) | pass | 1m48s |
| Build (WASM) | pass | 40s |
| Rustdoc | pass | 39s |
| Package dry-run | pass | 1m21s |

`mergeStateStatus == CLEAN` confirmed before squash-merge.

### Headless render verification (Metal, `HEADLESS_SHOT=/tmp/anim_effects_offset.png HEADLESS_FRAMES=74`)

Console: `footstep → fired its effects (total: 1)` → `(total: 2)` → `wrote … (74 frames)`. Shot:
title "footstep dust spawns at the FEET via SpawnParticles offset (RON)"; the cool walker body with
a `＋ center` mark at its center and a `◦ feet (offset)` mark below; the grey dust specks cluster at
the **feet** marker and drift down — not at center. Sent to the user as the acceptance artifact.

Reproduce: `HEADLESS_SHOT=/tmp/anim_effects_offset.png cargo run --example anim_effects` (native GPU;
`HEADLESS_FRAMES=N` overrides the 70-frame default).

## Code Analysis

- **`src/effect.rs`** — `Effect::SpawnParticles { particles, count, at, offset }`; `resolve_effect`'s
  `SpawnParticles` arm reads the anchor's `Transform`, then `pos = position + Vec2::new(offset.0,
  offset.1)` (z unchanged), `lookup_emitter`, force `emit=false`/`spawn_rate=0`, return `Burst`.
- **`src/zone_effect.rs`** — unchanged runtime; the system match arm `Effect::SpawnParticles { at:
  Zone, .. }` picks the anchor as before; `resolve_effect` then applies the offset from that anchor.
  Only the test's literal pattern needed `offset` added.
- **`src/anim_effect.rs`** — unchanged runtime (passes the animating entity as both anchors); only a
  new test added.
- **`examples/anim_effects.rs`** — `FEET_OFFSET` const (mirrors RON, for the guide only); HUD computes
  `center/feet` screen positions via `Camera::world_to_screen` under immutable borrows, then draws two
  `DrawText::centered` markers.

## Gotchas & Discoveries

- **The seq-7 i18n "gap" was already closed in v0.46.0 (#174)** — ~43 minor versions stale. Verify a
  named gap against the actual code before chasing it; the editor is fully localized (only intentional
  type-identifier labels remain English, consistent with leaving component-type names untranslated).
  Memory candidate list corrected so the next session doesn't re-chase it.
- **A literal `Effect::SpawnParticles { … }` pattern without `..` breaks on a new field** — only
  `zone_effect`'s default-parse test was exhaustive; every runtime match already used `..`. Grep
  `SpawnParticles {` (construction/match, not `.ron`) when adding an enum-struct field.
- **World Y is DOWN here** — `view_proj = orthographic_rh(0, w, h, 0, …)` (top=0, bottom=h), camera at
  origin + zoom 1 maps world→screen 1:1. The feet (bottom of the sprite, larger Y) are `+Y` from
  center. Got the sign right by reading `camera.rs` before picking `(0.0, 70.0)`, then confirmed on the
  headless shot.
- **glam's `serde` feature is on, but the codebase still uses `(f32,f32)` tuple mirrors for RON** —
  follow the convention (don't deserialize `Vec2` directly) for readable, consistent RON.
- **Environmental audio (standing):** locked/remote macOS has no audio device → 2 audio-device tests
  fail locally; `--skip` them and let CI gate audio.
- **zsh `${PIPESTATUS[0]}` is empty** (carried) — read exit codes via `echo $?` on an unpiped command
  or `$pipestatus[1]`; never pipe a gate's exit through `| tail`.

## Files Changed (PR #289)

### Source
- `src/effect.rs` — `Effect::SpawnParticles.offset: (f32,f32)` + apply in `resolve_effect`.
- `src/zone_effect.rs` — test pattern adds `offset` + asserts the `(0,0)` default (runtime unchanged).
- `src/anim_effect.rs` — new `spawn_particles_offset_displaces_burst` test (runtime unchanged).

### Example
- `examples/anim_effects.rs` — `FEET_OFFSET` const, `Camera::world_to_screen` center/feet HUD guides,
  refreshed module doc + title.
- `examples/anim_effects.ron` — footstep `SpawnParticles(... offset: (0.0, 70.0))` + header note.

### Docs / paperwork
- `CLAUDE.md` — module-map rows (Zone→Effect signature + Animation→Effect note) + header v1.6.177 /
  package v0.90.0.
- `docs/CHANGELOG.md` — 0.90.0 entry. `Cargo.toml`/`Cargo.lock` — version bump + lock refresh.

## User Feedback & Preferences

- **Empty-board → ASK → pick → execute-end-to-end via land-pr** is the working loop (seq 6, 7, 8).
- **Honesty over busywork** — when the chosen i18n candidate turned out already-done, the user valued
  the evidenced finding ("완료로 간주, 다른 작업 선택") over a manufactured refactor. Don't invent a gap.
- **Per-seq handoff cadence** — each code PR is followed by its own `docs(handoff)` PR; the memory
  seq-bump (→ 123) lands with the handoff so `main @ <hash>` = the handoff merge.
- **Korean for user-facing replies; English for code/docs/commits/handoffs.**
- **Merge authority delegated** — squash on green CI; #289 landed without asking.
- **Values evidence** — the headless render (dust at the feet) was sent as the acceptance artifact.

## Where We're Going

The `breadth-features` chain has shipped **ten** features this run (SpriteFlip, YSort, AnimationEvents,
TriggerZone, HitFlash, CameraLookahead, data-driven TriggerZones, Zone→Effect bindings,
Animation→Effect bindings, **per-effect offset**). The event→effect system now has a richer payload
(offset) over two sources. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST each session**
(ACTIVE EMPTY, EW-004 next) — a real downstream request outranks self-picked breadth. Remaining
candidates, roughly ordered, all **driven by a concrete game need, not speculation**:

1. **Richer effect payloads, continued** — a transform-relative / facing-aware offset (follows a
   flipped/rotated sprite), an effect that despawns/spawns an entity, a `Stayed`-throttle for zones.
   On a concrete request.
2. **A 3rd event→effect source** (only if a game asks) — e.g. `CollisionEvent`→effect, another reuse
   of the shared `crate::effect` machinery. Mechanical; the pattern is proven.
3. **A Tier-2 hardcoding knob on a concrete request** — the remaining knobs are weak (MAX_GAMEPADS,
   material params >4, editor app-id, frame latency); do one only if asked.
4. **~~Editor i18n~~ — NOT a gap** (verified this session; fully localized since v0.46.0). Do not
   re-chase.

Otherwise: **ASK the user for direction** when the board is empty.

**The shared-effect pattern (carried):** `crate::effect` holds the `Effect` vocabulary + `pub(crate)`
`resolve_effect`/`apply_pending`. A new event→effect source is a thin module: a `*Bindings` table
(tag→effects), a `RonRegistry`-backed registry (auto-registered), an `App::load_*`, and a `System`
that reads its event stream, keys on a tag, picks the anchor/target entities, and calls
`resolve_effect` + `apply_pending`. New `Effect` payload fields (like `offset`) flow to all sources
for free.

## Risks & Blockers

- **The offset is world-space, not transform-relative** — a burst won't follow a flipped/rotated/
  scaled sprite's facing. Fine for the example (a non-rotating top-down walker); a facing-aware offset
  is the next refinement (see Where We're Going).
- **Audio-device tests fail locally** (environmental) — `--skip` the two; CI gates audio.
- **No OS-gated code this session** — cross-platform pure logic + file I/O (lavapipe exercised the GPU
  render path; the offset is pure math, also verified on Metal headless). Nothing CI couldn't gate.
- No upstream/dependency blockers. Tree clean.

## Open Questions

- Should the offset be **transform-relative** (rotate/scale with the anchor) so a facing/flipped
  sprite's dust follows it? Deferred — world-space is enough for the example; revisit on a concrete
  need.
- Should other effects gain spatial payloads (e.g. a `Flash` ring radius)? No — `Flash`/`PlayTone`
  aren't positioned; only `SpawnParticles` is.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5            # confirm main tip (effect-offset #289 + this handoff PR)
git status -s                   # clean

# 1) Read the game↔engine board FIRST (drives priority)
sed -n '1,60p' ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY → ASK / pick

# Memory (full per-seq detail of this session)
#   ~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md (seq 123 tip)

# Key files if continuing event→effect / breadth work
#   src/effect.rs        — the shared Effect vocabulary (offset lives here; new payload fields flow to both sources)
#   src/anim_effect.rs   — Animation→Effect (thinnest reference)
#   src/zone_effect.rs   — Zone→Effect (phase + Tag-component key)
#   examples/anim_effects.{rs,ron} — the offset acceptance test (dust at the feet)

# Verify (NOTE: 2 audio-device tests fail locally — environmental; read exit via `echo $?`, not ${PIPESTATUS[0]} in zsh)
cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Reproduce this session's example (dust at the feet)
HEADLESS_SHOT=/tmp/anim_effects_offset.png cargo run --example anim_effects

# Next action
#   Read the wishlist board; if still empty, ASK for direction. Self-pick breadth + the event→effect
#   pattern (2 sources, richer payload) are well-covered; further refinements (facing-aware offset,
#   3rd source) should be driven by a concrete game need. Editor i18n is DONE — do not re-chase.
```

## Session Closed

**Closed:** 2026-06-30
**Chain:** `breadth-features` seq 8 — continuation of `HANDOFF_breadth-features_anim-effects_2026-06-29.md` (seq 7)
**Code landed:** #289 (v0.90.0), main @ `5aeeb42`. This handoff lands as its own `docs(handoff)` PR; the memory seq-bump to 123 lands with it.
**Session status:** Handed off. Ten breadth features shipped this run; the event→effect system now carries a per-effect position offset across both sources. Next session starts from the wishlist board or asks for direction.
