# Session wrap — 8 releases (v0.24.0→v0.31.0): verification debts closed + 6 features shipped

**Date:** 2026-06-18
**Status:** COMPLETED — all in-flight work merged + tagged, `main` clean & green
**Bead(s):** none (repo uses `plans/handoffs/`, not beads)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `33`
**Parent:** `HANDOFF_engine-hardening_ui-focus-nav_2026-06-18.md` (seq 32)
**Prior chain:** seq 24 (`p3-wasm-parity`) > 25 `webaudio-verify` > 26 `dialogue-portrait` > 27 `wasm-audio-panning` > 28 `wasm-save-verify` > 29 `iso-tilemap` > 30 `hex-tilemap` > 31 `autotile-unify` > 32 `ui-focus-nav` > **33 this (session wrap)**

> This is a **session-level wrap** tying together eight per-feature handoffs (seq 25–32). Each
> shipped feature has its own detailed handoff with full code-level notes; this file is the index +
> arc synthesis + cross-cutting lessons. For granular detail on any one release, read its handoff.

---

## Since Last Handoff (the whole session vs the seq-24 close)

The seq-24 handoff closed the P1/P2/P3 roadmap (v0.23.0) and flagged exactly **one open item**:
"3B WebAudio runtime needs a browser + ears." This session started from `git log`/handoff review,
then expanded far beyond that single debt:

- **Both verification debts are now CLOSED** — WebAudio acoustic output (user confirmed in a real
  browser) and the wasm AEAD save localStorage round-trip (autonomous headless smoke 7/7). The
  seq-24 handoff only knew about the audio one; the save browser-path debt (from v0.22.0) surfaced
  when the user asked "are there two untested things?" — there were.
- **Six new features shipped** beyond verification: dialogue portraits (A1), wasm SFX pan (A2),
  isometric + hex tilemaps (C1/C2), autotile unification (B1), UI keyboard focus (B2).
- Trajectory: started as "confirm the roadmap is done," became a full feature+cleanup sprint driven
  turn-by-turn by the user picking items off a backlog list I proposed mid-session.

## The Goal

skeleton-engine is a fork-friendly, MIT, genre-agnostic 2D engine (VISION: "the example is the
acceptance test"). This session's goal evolved: (1) verify the post-roadmap engine had no untested
gaps, then (2) work down a prioritized backlog of user-experience/feature items the user selected
one at a time. End state: every requested item shipped as its own CI-green, merged, tagged 0.x
MINOR release with a playable example + tests + handoff, and the two long-standing verification
debts retired.

## Where We Are

- `main` @ `de4152e`, package **v0.31.0**, CLAUDE.md header **v1.6.80**, tree clean, CI green.
- **8 PRs merged + tagged** this session (#112–#119), each a self-contained 0.x MINOR release.
- Tags `v0.24.0`–`v0.31.0` all pushed (annotated, `vX.Y.Z — desc (#NN)`, on their merge commits).
- Test counts grew: lib tests now ~850+ (tilemap 65, ui 104 incl. new focus tests); all green.
- New examples: `web_audio`, `dialogue_portrait`, `wasm_save`, `iso_tilemap`, `hex_tilemap`,
  `ui_focus` (+ generated art assets, all via a pure-Python zlib PNG encoder — no PIL/deps).
- New optional local smoke harnesses: `scripts/wasm_audio_smoke.sh`, `scripts/wasm_save_smoke.sh`
  (headless Chrome, verdict read live over the DevTools `/json` endpoint).
- **Both verification debts retired** (P3-3A wasm save browser path, P3-3B WebAudio acoustic).
- Merge cadence: every release was watch-CI-to-green → squash-merge → delete branch → sync main →
  annotated tag, mostly via a background "wait green then merge+tag" Bash task per PR.

## What We Tried / Did (chronological, one row per release)

See **Evidence & Data** for the release table. Arc in brief:
1. **seq 25 (v0.24.0, #112)** — Built `web_audio` example + a headless lifecycle smoke. First cut
   used `--virtual-time-budget` + `--dump-dom`; the final `resume()` check FLAKED. Root cause +
   fix below (the session's most important gotcha). Added `is_running`/`is_music_playing` accessors.
2. **seq 26 (v0.25.0, #113)** — Dialogue portraits: the `portrait` field existed since v0.19.0 but
   was never rendered. Wired it through `UiImageQueue`. `clippy::type_complexity` forced a `DrawItem`
   struct.
3. **seq 27 (v0.26.0, #114)** — `WebAudio::play_sfx -> Sfx` (per-source pan/volume/stop). Async
   node-creation was the design crux (see Code Analysis). Smoke grew 9/9 → 12/12.
4. **seq 28 (v0.27.0, #115)** — Closed the *second* verification debt: a `wasm_save` example + smoke
   asserting AEAD save/load + ciphertext-in-localStorage + tamper detection (7/7). No engine change.
5. **seq 29/30 (v0.28.0/v0.29.0, #116/#117)** — `TilemapProjection` Isometric then Hexagonal, split
   per the user's "C1=iso, C2=hex" decision. Per-projection coord transform + picking + render.
6. **seq 31 (v0.30.0, #118)** — Unified `TilemapAutotile`+`MultiTerrainAutotile` into one
   `{ mode: AutotileMode }`; dropped the ghost `ConnectRule`. Breaking but tiny call surface.
7. **seq 32 (v0.31.0, #119)** — `UiFocus` + a `UiSystem` focus pass: Tab nav, focus ring,
   Enter/Space activation, slider arrows, click-to-focus.

## Key Decisions

- **Each item = its own MINOR release + PR + tag + handoff**, not one mega-PR. Keeps reviewable,
  matches the repo's established 0.x cadence and the user's "test+merge+handoff" rhythm.
- **Branch each new feature off *fresh main* AFTER the prior PR squash-merges** — squash makes the
  base diverge, so starting C2 before C1 merged (etc.) would cause dirty 3-way diffs. Cost: serial
  merges; benefit: clean PRs. This shaped the whole session's pacing.
- **C1/C2 split** of the big "iso/hex tilemap" item — user chose iso-then-hex over one combined PR.
- **B1 unification was worth it** (not churn): the public two-type API was unified to match the
  dispatch `TilemapSystem` *already did internally* (`AutotileMode {Single,Multi}`). `dig_quest`
  (single-terrain) needed zero changes; only `multi_terrain` changed one call. Dropped `ConnectRule`
  as a genuine ghost (do-nothing marker the docs called a "future extension point").
- **Verify via real-time DevTools title polling, NOT `--virtual-time-budget`** for anything touching
  the AudioContext (see gotcha #1).
- **Generated all example art with a pure-Python zlib PNG encoder** (portraits, iso diamonds, hex
  hexes) — no asset pipeline, no PIL dependency, committed as tiny PNGs.

## Evidence & Data

### Releases shipped this session

| seq | ver | PR | item | what | tests | verify |
|---|---|---|---|---|---|---|
| 25 | 0.24.0 | #112 | debt 3B | `web_audio` example + lifecycle smoke + `is_running`/`is_music_playing` | smoke 9/9 ×4 | green |
| 26 | 0.25.0 | #113 | A1 | dialogue per-line portrait rendering + `dialogue_portrait` | playtest 4 frames | green |
| 27 | 0.26.0 | #114 | A2 | `WebAudio::play_sfx -> Sfx` (pan/volume/stop) | smoke 12/12 | green |
| 28 | 0.27.0 | #115 | debt 3A | `wasm_save` example + smoke (AEAD localStorage round-trip + tamper) | smoke 7/7 | green |
| 29 | 0.28.0 | #116 | C1 | isometric `TilemapProjection` + `iso_tilemap` | tilemap 61 (+4 iso) | green |
| 30 | 0.29.0 | #117 | C2 | hexagonal `TilemapProjection` + `hex_tilemap` | tilemap 65 (+4 hex) | green |
| 31 | 0.30.0 | #118 | B1 | autotile unify `TilemapAutotile { mode }` + drop `ConnectRule` | tilemap 65 | green |
| 32 | 0.31.0 | #119 | B2 | `UiFocus` + Tab/focus nav | ui 104 (+5 focus) | green |

### Merge commits + per-feature handoffs (navigation index)
| ver | merge commit | per-feature handoff (full code-level detail) |
|---|---|---|
| 0.24.0 | `4fa50ea` | `HANDOFF_engine-hardening_webaudio-verify_2026-06-18.md` (seq 25) |
| 0.25.0 | `5c3b912` | `HANDOFF_engine-hardening_dialogue-portrait_2026-06-18.md` (seq 26) |
| 0.26.0 | `adb75ed` | `HANDOFF_engine-hardening_wasm-audio-panning_2026-06-18.md` (seq 27) |
| 0.27.0 | `751df32` | `HANDOFF_engine-hardening_wasm-save-verify_2026-06-18.md` (seq 28) |
| 0.28.0 | `c8988de` | `HANDOFF_engine-hardening_iso-tilemap_2026-06-18.md` (seq 29) |
| 0.29.0 | `a50a18b` | `HANDOFF_engine-hardening_hex-tilemap_2026-06-18.md` (seq 30) |
| 0.30.0 | `c77ed51` | `HANDOFF_engine-hardening_autotile-unify_2026-06-18.md` (seq 31) |
| 0.31.0 | `de4152e` | `HANDOFF_engine-hardening_ui-focus-nav_2026-06-18.md` (seq 32) |

### Backlog status (the seq-25 feature list I proposed)
| item | status |
|---|---|
| verification debt 3A (wasm save browser) | ✅ closed (#115) |
| verification debt 3B (WebAudio acoustic) | ✅ closed (user-confirmed) |
| A1 dialogue portraits | ✅ shipped (#113) |
| A2 fuller wasm audio (per-source + pan) | ✅ shipped (#114) |
| C1 isometric tilemap | ✅ shipped (#116) |
| C2 hex tilemap | ✅ shipped (#117) |
| B1 TilemapAutotile mode unify | ✅ shipped (#118) |
| B2 UI Tab/focus | ✅ shipped (#119) |
| further wasm audio (named buses / wasm crossfade) | ⬜ not started |
| crates.io publish | ⬜ deferred (irreversible, needs explicit go) |

## Code Analysis (cross-cutting facts worth keeping)

- **Tilemap projections** (`src/tilemap/mod.rs`): `TilemapProjection {Orthographic, Isometric,
  Hexagonal}` (plain enum, NOT `#[non_exhaustive]`). Four methods branch on it: `cell_center_world`,
  `cell_at_world`, `cell_z`, `cell_render_size`. Iso = 2:1 diamond (inverse + round picking, depth
  z=row+col). Hex = pointy-top odd-r offset (pixel→axial→`axial_round` cube-round picking, z=-1, hex
  sprite is taller than wide → `cell_render_size`). `TilemapSystem::spawn_tile_entity` is the single
  site that consumes all three.
- **Autotile** (`src/tilemap/autotile.rs`): ONE `TilemapAutotile { neighborhood, oob_filled, mode:
  AutotileMode }`. `AutotileMode::Single{mask_to_tile}` (edge_16/blob_47) | `Multi{rules:
  Vec<TerrainRule>}` (multi_edge_16). `MultiTerrainAutotile` + `ConnectRule` removed.
- **WebAudio** (`src/audio_wasm.rs`, wasm-only): `play`/`play_sfx`→`Sfx`/`play_music`/`stop_music`/
  master `set_volume`/`volume`/`suspend`/`resume`/`is_running`/`is_music_playing`. `Sfx` holds
  sync-created gain+panner (so set_pan/volume work pre-decode) + `Rc<RefCell<Option<source>>>` filled
  when the async decode starts.
- **UI focus** (`src/ui/focus.rs` + `src/ui/system/focus_pass.rs`): `UiFocus { entity: Option<Entity> }`
  resource (auto-inserted). Focus pass runs FIRST in `UiSystem`. `InputSnapshot` gained
  tab/shift/activate. `UiEvent` now derives `PartialEq`.

## Files Changed (by area; full per-file lists in each per-feature handoff)
### Source
- `src/audio_wasm.rs` — accessors + `play_sfx`/`Sfx` (seq 25, 27)
- `src/dialogue/mod.rs` — portrait render + `DrawItem` (seq 26)
- `src/tilemap/{mod,system,autotile}.rs` — projections + autotile unify (seq 29–31)
- `src/ui/focus.rs` (new), `src/ui/system/focus_pass.rs` (new), `src/ui/system/{state,event,system}.rs`,
  `src/app/core_resources.rs` — focus nav (seq 32)
- `src/lib.rs`, `Cargo.toml` — exports + web-sys features (`AudioContextState`, `StereoPannerNode`)
### Examples (new)
- `web_audio`, `dialogue_portrait`, `wasm_save`, `iso_tilemap`, `hex_tilemap`, `ui_focus` (+ generated art)
### Tooling
- `scripts/wasm_audio_smoke.sh`, `scripts/wasm_save_smoke.sh` (new headless checks)
### Docs
- `docs/CHANGELOG.md` (8 entries), `CLAUDE.md` (header + module-map rows), 8 per-feature handoffs

## User Feedback & Preferences (calibrates next session)
- **Drives work item-by-item off a backlog list** — proposed a list, user picked "WebAudio 검증",
  then "A1", "A2", "C1/C2 (분할)", "B1/B2". Offer a concrete prioritized list and let them choose.
- **"머지 진행" / "머지 승인"** — approves merges per-PR; later "c1.c2.진행" and "b1.b2 진행하고
  완료되면 머지까지" = standing approval to carry those items through merge. Still report each merge.
- **Confirmed audio acoustically** ("스타트 오디오 누르면 소리 정상 출력돼") — happy to do the
  human-only verification step when asked; surface those steps explicitly.
- **Asked "두 가지 미검증 있나?"** — wants gaps named honestly; don't over-claim "done".
- Korean for all user-facing reports; English for code/docs/handoffs (per project rule).
- Values the VISION loop: every feature got a playable example + windowed/headless playtest.

## Where We're Going (next session — all optional, none committed)
1. **Further wasm audio** — per-source named buses / wasm crossfade (build on `Sfx`; NOT kira — too
   large a swap). Headless-checkable via the existing smoke pattern; acoustic = human.
2. **crates.io publish** — still deferred; irreversible; needs explicit go. Also publish
   `engine_reflect_derive` so `cargo add` users can `#[derive(Reflect)]`.
3. **Stretch:** gamepad UI focus nav (D-pad/stick move focus, A activates); flat-top hex variant;
   autotile support across iso+hex; focus-ring styling knobs.

## Risks & Blockers
- None blocking. Tree clean, all CI green, all tags pushed.
- Auto-merge is disabled on the repo (`enablePullRequestAutoMerge` GraphQL error) — merges are
  done manually via the "wait green then `gh pr merge --squash`" background task.

## Open Questions
- None outstanding. (The "two untested things" question was answered + both closed.)

## Quick Start for Next Session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -8            # de4152e (#119) … 4fa50ea (#112)
grep -m1 '^version' Cargo.toml  # 0.31.0
./scripts/verify.sh             # green (fmt/clippy/wasm build/test/doc)

# Optional local browser checks (need Chrome + matching wasm-bindgen-cli):
bash scripts/wasm_audio_smoke.sh   # PASS (12/12)
bash scripts/wasm_save_smoke.sh    # PASS (7/7)

# Key files if continuing audio: src/audio_wasm.rs (Sfx); tilemap: src/tilemap/{mod,autotile}.rs;
# ui: src/ui/system/focus_pass.rs

# Next action (only if the user picks one): "further wasm audio (named buses)" or crates.io publish.
# Otherwise: nothing required — the requested backlog (debts + A1/A2/B1/B2/C1/C2) is fully shipped.
```

---

## Reusable verification-harness pattern (for the next wasm feature)
The two new smokes (`wasm_audio_smoke.sh`, `wasm_save_smoke.sh`) share a pattern worth reusing for
any future wasm runtime check:
1. A **wasm-only example** (native `main` is a stub) drives the feature and writes a verdict into
   `document.title` as `XXX_CHECK: PASS (n/n)` / `XXX_CHECK: FAIL: <step>` — the failing step name
   travels with a failure. Per-step ✅/❌ lines also go into a `#status` DOM element for humans.
2. A `web/{index.html,build.sh}` scaffold (`cargo build --example … --target wasm32` + `wasm-bindgen
   --target web`), invoked via `bash` so the exec bit doesn't matter.
3. The script launches **headless Chrome with `--remote-debugging-port`** (real time, NOT virtual
   time), polls `curl http://localhost:$DBG/json` for the tab whose title contains the verdict, then
   reaps Chrome. No GPU flags needed unless the example renders.
4. In-browser-only assertions (e.g. "stored blob is ciphertext, not plaintext"; "AEAD tamper →
   load errs") read/poke `localStorage` via `web_sys::Storage` inside the example.
This closes "compiles on wasm" → "actually works in a browser" without needing a human, for anything
except literally *hearing* audio (which stays a human step).

## Versioning / process notes
- 0.x cadence: **MINOR = any release (incl. breaking); PATCH = bugfix.** B1 was breaking
  (removed two public types) but still a MINOR. Each release bumped Cargo.toml + Cargo.lock +
  CHANGELOG + CLAUDE.md header (`v1.6.72`→`v1.6.80`) together (the `ship`-skill four-edit set).
- Tags are annotated `vX.Y.Z — desc (#NN)` on the squash-merge commit, pushed right after merge.
- The repo is branch-protected (required checks: Build (WASM)/Test (native)/Rustdoc/Package dry-run);
  every change is PR-only. CI Test(native) ≈ 3.5–6.5 min is the long pole per PR.

## Cross-cutting gotchas (the expensive-to-rediscover lessons)
1. **Do NOT use Chrome `--virtual-time-budget` to verify AudioContext state.** It fast-forwards JS
   timers, but `suspend`/`resume` run on the browser's REAL-TIME audio thread — the virtual clock
   races ahead and `resume()` checks flake (passed once / failed once in the first cut). Fix: run
   Chrome in real time + poll the page **title** over the DevTools `/json` endpoint. Both wasm smokes
   use this. (Secondary, from `wasm_smoke.sh`: SwiftShader headless Chrome hangs on *exit* — launch
   it backgrounded and reap after the verdict appears; never wait on it.)
2. **Synthetic mouse-move/clicks don't reach winit** (only `osascript key code N` does). So live
   mouse-hover readouts can't be playtested; cover picking with unit tests, drive examples by keyboard.
3. **Generated-atlas bug reads as a "hole," not an error** — the first `hex_tiles.png` had the sand
   cell's polygon in absolute atlas coords but received local x → fully transparent sand → cells
   looked like holes. Always eyeball a generated atlas.
4. **`clippy::type_complexity`** fires on 5-field tuples — extract a small struct (`DrawItem`).
5. **Squash-merge diverges the base** — branch each next feature off fresh main only AFTER the prior
   merged.
6. **macOS `build.sh` exec bit** — git stores example `build.sh` as 0644; invoke via `bash` in
   scripts (or `git update-index --chmod=+x`) so a fresh checkout doesn't break.
