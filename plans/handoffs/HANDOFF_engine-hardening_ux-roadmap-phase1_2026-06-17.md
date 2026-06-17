# User-experience roadmap — Phase 1 shipped (first-hour onboarding)

**Date:** 2026-06-17
**Status:** IN PROGRESS — Phase 1 of a 7-phase roadmap shipped (PR #93 squash-merged, v0.11.1, CI-green). Session is CONTINUING into Phase 2 (game-feel core). `main` @ `7ab8987`, clean, CI green.
**Bead(s):** none (no bead tracker in this repo — `bd` unavailable)
**Epic:** skeleton-engine — engine hardening / fork-friendliness
**Chain:** `engine-hardening` seq `13`
**Parent:** `HANDOFF_engine-hardening_version-reset-0x_2026-06-17.md` (seq 12)
**Prior chain:** seq 10 (render-split, v10.2.1) → seq 11 (vision-features-batch, v10.3–10.7) → seq 12 (version reset 10.7.0→0.11.0) → **this (13)**

---

## Since Last Handoff

Seq 12 closed with "Where We're Going #1 = crates.io first publish as 0.11.0 (irreversible; explicit go needed)" and flagged a "hidden blocker: `engine_reflect_derive` path-only dep blocks `cargo publish`." This session went a different direction and produced a new multi-phase plan:

1. **Onboarding caught a stale handoff claim.** While verifying the publish prerequisite, I found the seq-12 "path-only **regular** dep blocker" is **STALE/WRONG** — `engine_reflect_derive` is already a `[dev-dependency]` (commit `4f28baa`, 2026-06-13), so `cargo package`/`publish` is mechanically unblocked. (Detail under "Key Decisions".)
2. **User deferred crates.io publish.** When I surfaced the publish option, the user said they lack confidence / feel publishing is a burden, and asked how the engine compares to peers. I gave an honest comparison → conclusion: for a **fork-first** skeleton, GitHub is the primary channel and crates.io is a low-value, non-urgent secondary channel. **Publish is DEFERRED** (not blocked — deferred by choice).
3. **User pivoted to "raise satisfaction for people who use/fork it"** and asked for feature / core-strengthening ideas.
4. **Ran a 3-agent codebase audit** (onboarding, API rough edges, feature coverage) → synthesized into a prioritized idea set.
5. **User asked to bundle the ideas into a phased plan** (explicitly to avoid context overflow from one mega-goal). Wrote `plans/USER_EXPERIENCE_PLAN_2026-06-17.md` (7 phases).
6. **Shipped Phase 1** (first-hour onboarding) as PR #93 → CI 4/4 green → merged → v0.11.1.

So seq 12's "publish next" is superseded by a **user-experience roadmap**; publish is parked.

## Reference Documents

- `plans/USER_EXPERIENCE_PLAN_2026-06-17.md` — THE plan. 7 phases, each a session-sized PR. Committed to main in PR #93. **Read this first for what's next.**
- `plans/VERSION_RESET_PLAN_2026-06-17.md` — seq-12 reset rationale + 0.x cadence policy.
- `CLAUDE.md` — agent quick reference (header now v1.6.59 / package v0.11.1).
- `docs/CHANGELOG.md` — new `## 0.11.1` entry (Phase 1).
- `FORKING.md` — NEW this session: the English getting-started guide for forkers.
- Memory `engine-current-state` — updated with seq-13 state + the path-dep correction.

## The Goal

Raise satisfaction for **other people who fork / use** `skeleton-engine`. Framing from the audit: **breadth is not the gap** (the engine rivals a tier-2 2D engine in features); satisfaction is gated by (1) first-hour onboarding friction, (2) a thin "juice" layer, (3) a few daily-hit API papercuts. Ship improvements in context-sized phases, each validated by the project's own rule (a feature isn't done until a small playable example exercises it).

## Where We Are

- **`main` @ `7ab8987`**, package **v0.11.1**, CLAUDE.md header **v1.6.59**, clean tree, only `main` branch locally.
- **772 lib tests** pass — unchanged (Phase 1 was docs + 1 example, zero `src/` logic change).
- **Verify gate green 3×** this session (initial onboarding check, Phase 1 pre-bump, Phase 1 post-bump): `all checks passed ✓`.
- **PR #93 shipped** (squash-merged, branch deleted, CI 4/4 green): docs(onboarding) first-hour pass → main `7ab8987`.
- **`hello_sprite` render-verified** on macOS via screencapture — cyan two-eyed sprite centered + spinning, texture loaded, dark background. Acceptance bar met.
- **Phase 1 complete; Phases 2–7 open.** No other work in flight. No wakeups/cron/loops armed.
- **crates.io: still UNPUBLISHED**, now deferred by user choice (was seq-12's #1).

## What We Tried (Chronological)

1. **Onboarding.** Read seq-12 handoff + `VERSION_RESET_PLAN`; ran `./scripts/verify.sh` as-is → green, 772 tests, main `938c5c2` v0.11.0. Explored `Cargo.toml` internals + git history → **found the publish "blocker" was already resolved** (dev-dependency).
2. **Publish discussion.** User hesitant about publishing; asked for peer comparison. Gave an honest tiered comparison (Bevy/Fyrox/comfy/macroquad) → publish deferred, GitHub is the fork-first channel.
3. **Pivot to satisfaction.** User asked for feature/core ideas. Launched **3 parallel Explore agents (Sonnet)**: onboarding/first-run, API ergonomics+error-handling, feature coverage vs peers.
4. **Synthesized** the three reports into a tiered idea set (core hardening / new features / onboarding). Recommended starting with onboarding (the real gate) then game-feel core.
5. **User asked for a phased plan** (context management). Wrote `USER_EXPERIENCE_PLAN_2026-06-17.md` — 7 phases with per-item code anchors, hard constraints, deferred section.
6. **User: "phase 1 착수".** Branched `docs/phase1-onboarding`. Implemented the 4 Phase-1 items.
7. **Generated `examples/assets/player.png`** with a Python stdlib (zlib+struct) PNG writer — a 32×32 RGBA two-eyed cyan sprite (no ImageMagick/PIL on the box).
8. **Verify (pre-bump) green** → `ship` skill bump to **0.11.1** (PATCH — docs+example, no library API) → Cargo.lock refresh → **verify (post-bump) green**.
9. **Built + playtested `hello_sprite`** (osascript window position + screencapture + kill) → confirmed render.
10. **Committed → pushed → PR #93 → watched CI unmasked** (`gh pr checks 93 --watch --fail-fast > log 2>&1`, no tail pipe) → 4/4 green.
11. **User authorized merge + /handoff + Phase 2.** Squash-merged #93, deleted branch, synced main, updated memory, wrote this handoff.

## Key Decisions

- **crates.io publish deferred (not blocked).** Fork-first vision ⇒ GitHub is the primary channel; 0.x already signals "no stability promise"; no external consumers ⇒ no urgency. The only "now" pressure (name-squatting) is low.
- **The seq-12 publish blocker was stale.** `engine_reflect_derive` is a `[dev-dependency]` (`Cargo.toml:159` is the `[dev-dependencies]` header; `:160` is the dep). Cargo strips path-only dev-deps with no version on publish, so `cargo package` already passes in CI. The seq-12 handoff misread the section header. **The real (deferred) publish item:** the `derive` *feature* was removed when this change landed, so `cargo add` users can't `#[derive(Reflect)]` unless `engine_reflect_derive` is also published — forkers (workspace) are unaffected.
- **Phase 1 = PATCH 0.11.1**, not MINOR — docs + a new example are not a library (`src/`) API change. First real exercise of the `ship` skill's 0.x cadence; it correctly chose PATCH and never suggested 1.0.0.
- **README was stale beyond the install line** — also fixed MSRV (`1.88`→`1.95`, real value per Cargo.toml `rust-version`) and removed the obsolete "v2.0 notes" framing (the whole README read as "v2.0-era" while the package is 0.11.x).
- **`hello_sprite` stores the entity in the system** (no query) — deliberately avoids teaching the collect-then-`get_mut` anti-pattern in the beginner example (Phase 3 will add `query2_mut` and the README/FORKING borrow note will cross-link it).
- **Committed the plan doc inside PR #93** (like VERSION_RESET_PLAN in #91) — keeps the roadmap in main's history for the next session.
- **B2 (juice example) folded into Phase 2** — one `juice_demo` will exercise TimeScale + Tween easing AND rescue the already-shipped-but-undemo'd FadeTransition / camera-shake / post-process (the engine currently violates its own "example or it's not done" rule for those three).

## Evidence & Data

### PR shipped

| PR | main after | Title | CI |
|---|---|---|---|
| #93 | `7ab8987` | docs(onboarding): first-hour pass — hello_sprite, fork-first README, FORKING.md (v0.11.1) | 4/4 green |

### PR #93 CI timings

| Check | Result |
|---|---|
| Rustdoc | pass 35s |
| Build (WASM) | pass 38s |
| Package dry-run | pass 1m18s |
| Test (native) | pass 3m53s |

### Files in PR #93 (`git diff --cached --stat`)

| File | Δ |
|---|---|
| `examples/hello_sprite.rs` | +73 (new) |
| `examples/assets/player.png` | new, 186 bytes (32×32 RGBA) |
| `FORKING.md` | +119 (new) |
| `README.md` | +46/−18 |
| `docs/CHANGELOG.md` | +25 |
| `CLAUDE.md` | ±4 (header version + Reflect row) |
| `Cargo.toml` / `Cargo.lock` | version → 0.11.1 |
| `plans/USER_EXPERIENCE_PLAN_2026-06-17.md` | +249 (new) |

### The 7-phase plan (from `USER_EXPERIENCE_PLAN_2026-06-17.md`)

| Phase | Theme | Items | Est. bump | Example | Status |
|---|---|---|---|---|---|
| 1 | First-hour & doc truth | hello_sprite, README, FORKING.md, derive-doc fix | PATCH 0.11.1 | hello_sprite | **DONE (#93)** |
| 2 | Game-feel core (juice) | TimeScale, Tween<T>+easing, juice_demo (folds in shake/fade/postfx) | MINOR 0.12.0 | juice_demo | NEXT |
| 3 | Core API ergonomics | query2_mut (+refactor flagship demo), push/pop scene | MINOR 0.13.0 | (refactor) | open |
| 4 | Dialogue primitive | DialogueBox + typewriter | MINOR 0.14.0 | dialogue_demo | open |
| 5 | WASM persistence | save via localStorage | MINOR 0.15.0 | extend coin_race | open |
| 6 | Particle depth | gravity/rotation/emit-shape | MINOR 0.16.0 | particles_showcase | open |
| 7 (stretch) | WASM audio | SFX on wasm | MINOR | wasm smoke | open |

### 3-agent audit — top findings (full detail in conversation; condensed)

- **Onboarding:** README install lied (`= "2.0.0"`, unpublished); all user docs Korean; no fork-first start path / template; example ladder cliff (basic.rs 74 lines → 239+); asset workflow only shown in advanced examples. → fixed in Phase 1 + Phase 2 of docs.
- **API:** `query2_mut` missing (flagship WASM demo teaches collect-then-`get_mut`); `register_editable_component` 6-trait-bound heaviness; no `push_scene`/`pop_scene`; `DrawText` uses `[u8;4]` not `Color`; internal query `unwrap()`s lack `// invariant` comments; the derive-feature gap. Error handling is otherwise *better than average* (coherent `SaveError`/`AssetLoadError`; ~15-20 prod `unwrap`s, only 1 real TODO at `src/ui/system/button_pass.rs:26`).
- **Coverage:** deep already (render/anim/audio/physics/tilemap/net). Genuine gaps: **no TimeScale** (hit-stop/slow-mo) — highest game-feel ROI; FadeTransition/shake/post-process exist but **zero example coverage**; no DialogueBox/typewriter; no WASM save (localStorage); no WASM audio; thin easing (6 variants, Tween is f32-only); particle emitter lacks gravity/rotation/shapes; localization lacks plural/interpolation.

## Code Analysis

- **`App::load_image(path)`** (`src/app/assets.rs:21`) is synchronous-registration: pushes to `pending_textures`, returns a `Handle` immediately; GPU upload happens at `run()`. So call it before `app.run()` (the nine_slice/platformer pattern). `load_image_async` (`:30`) is the deferred variant that bumps `LoadProgress`.
- **`Sprite` constructors** (`src/components.rs:68-105`): `colored(r,g,b)`, `textured(path)`, `with_handle(handle)`, `textured_with_handle(path, Some(handle))` (handle priority + path fallback — used in hello_sprite). `Transform::default()` scale is `Vec2::ONE * 64.0`.
- **Examples auto-discovery:** any `examples/*.rs` is run via `cargo run --example <name>` with NO `Cargo.toml` `[[example]]` stanza (those are only for the `examples/games/<name>/` subfolder paths). hello_sprite needed no manifest change.
- **`examples/assets/` is NOT in the Cargo.toml `include` list** — so the placeholder PNG (like the existing `blend_locomotion.png`) won't ship in a published crate. Irrelevant for forkers (repo clone has it); a publish-time concern only (note for Phase-related publish work).
- **Phase 2 anchors:** TimeScale → the scheduler dt path (`src/app/schedule.rs`); Tween/Easing → `src/tween.rs` (f32-only `Tween`, 6-variant `Easing` NOT `#[non_exhaustive]`), `Lerp` trait in `src/timer.rs` (already impl'd for Vec2/Color); juice → `FadeTransition` (`fade_out`/`fade_in`), `cam.shake(strength, duration)`, `PostProcessConfig`.

## Files Changed

### Repo — PR #93 (merged)
- `examples/hello_sprite.rs`, `examples/assets/player.png` — new.
- `FORKING.md` — new English getting-started guide.
- `README.md` — fork-first Getting Started, MSRV fix, de-versioned, Korean-docs note.
- `CLAUDE.md` — header v1.6.58→v1.6.59, package v0.11.0→v0.11.1, Reflect-row derive fix.
- `docs/CHANGELOG.md` — `## 0.11.1`.
- `Cargo.toml`/`Cargo.lock` — 0.11.0→0.11.1.
- `plans/USER_EXPERIENCE_PLAN_2026-06-17.md` — new (the roadmap).

### Memory (`~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/`)
- `engine-current-state.md` — seq-13 state line + the path-dep correction.
- (MEMORY.md hook still points correctly; update if it drifts.)

## User Feedback & Preferences (REQUIRED)

- **Values honesty / real opinions** (carried from seq 12): asked how the engine compares to peers and whether publishing was warranted — wanted a genuine verdict, not validation. I recommended deferring publish and said why.
- **Confidence/burden about publishing:** "crates.io는 자주 추천 받긴 하는데, 저장소에 올린다는게 부담이 돼. 아직 자신감이 없어." → publishing is parked until they choose.
- **Direction:** "다른 사람이 사용했을 때 만족도를 높이는 쪽으로 가고싶어" + asked for feature / core ideas.
- **Context management is a stated concern:** "이걸 한번에 goal로 묶으면 컨텍스트관리에 문제가 있을 것 같으니까 적절한 순서로 묶어서 plan 생성해줘" → hence the phased plan; keep phases session-sized.
- **Sequencing this turn:** "머지 진행하고 /handoff 이후 phase2 진행."
- **Standing (carried):** Korean prose to the user / English code+docs+handoff; Sonnet subagents with explicit `model` ([[new-model-subagent-incompat]]); never tag/publish unprompted; **merge authority re-confirm each session** (was granted for #93 this session); beginner glossary is the Korean-doc exception.

## Where We're Going

**Immediately: Phase 2 — game-feel core (MINOR → 0.12.0).** Continuing this session.
- `TimeScale(f32)` resource (default 1.0), multiplied into `dt` in `src/app/schedule.rs`; `App::set_time_scale`. Document whether physics/anim honor scaled dt (default: yes — it's the `dt` all systems get).
- `Tween<T: Lerp>` generic (currently f32-only) + `Easing::Bounce/Elastic/Spring`; mark `Easing` `#[non_exhaustive]` (one-time pre-1.0 break) so future variants are non-breaking. Keep `TweenSequence` working and existing f32 call sites compiling.
- One `juice_demo` example exercising TimeScale hit-stop + Tween easing + camera shake + FadeTransition + post-process vignette (rescues the 3 orphaned features).
- Then verify → `ship` 0.12.0 → PR → CI unmasked → **re-confirm merge** → merge.

After Phase 2: Phase 3 (query2_mut + push/pop scene), then 4–7 per the plan.

## Risks & Blockers

- **main is PR-only (branch protection, `enforce_admins`, 4 required checks, `strict`).** Even docs need a green-CI PR. No admin override; rebase if main moved.
- **Merge authority is per-session** — re-confirm before merging each Phase PR (granted for #93; re-ask for #94+).
- **`ship` 0.x cadence now validated once (PATCH)** but the MINOR path (Phase 2) is still first-use — confirm it bumps `0.12.0`, never 1.0.0.
- **Phase 2 has real breaking-ish edges:** `Tween` generic migration + `Easing` `#[non_exhaustive]` can break exhaustive matches / type inference. Pre-1.0 license to break, but keep in-repo call sites green and note it in CHANGELOG.
- **No GPU in CI** — `juice_demo` visual correctness needs a macOS screencapture playtest (the pattern worked this session: build the example first, then osascript-position + screencapture).

## Open Questions

- Phase 2 scope: ship TimeScale + Tween + juice_demo as ONE PR, or split TimeScale and Tween into two? (default: one PR — they're both small and the example needs both.)
- Tag each phase release (`v0.11.1`, `v0.12.0`, …)? Not done for #93 (tags are an explicit outward action). (default: leave untagged unless asked.)
- crates.io publish: still deferred — revisit only on user request.

## Reusable Gotchas (carry forward)

- **NEVER pipe a gate to `tail`/`head`.** `verify.sh | tail` / `gh pr checks --watch | tail` set the pipeline exit to `tail`'s 0, masking failures (caused a red merge in seq 11). Run `cmd > /tmp/x.log 2>&1` (no trailing pipe) and eyeball `gh pr checks <n>`. Did this cleanly for #93.
- **Verify the handoff's own claims** — seq-12's headline "path-dep blocker" was stale. Re-check `Cargo.toml` section headers, don't trust a cited line number blind.
- **macOS playtest pattern (works):** `cargo build --example <name>` FIRST (clippy/test only check, don't leave a runnable binary), launch the binary in background, poll `osascript ... exists (process "<name>")`, set window position/size, `screencapture -x -R<region> out.png`, kill. The 32×32 sprite read fine at 96px.
- **PNG without ImageMagick/PIL:** Python stdlib `zlib`+`struct` writes a valid RGBA PNG (IHDR color-type 6, per-scanline filter byte 0, IDAT `zlib.compress`, IEND).
- **Versioning is PRE-1.0 (0.x):** docs/example/no-API → PATCH; feature/breaking → MINOR; never 1.0.0 auto, never revert to 10.x. The `ship` skill is 0.x-aware.
- **`examples/*.rs` auto-discovered** — no `[[example]]` stanza needed (only the `examples/games/<name>/` subfolder paths need one).

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4            # 7ab8987 Phase 1; 985a05e seq-12 handoff
grep -m1 '^version' Cargo.toml  # 0.11.1
git status -s                   # clean
./scripts/verify.sh             # green, 772 lib tests; RUN AS-IS, do NOT pipe to tail

# Read first:
#   plans/USER_EXPERIENCE_PLAN_2026-06-17.md   (THE plan — Phase 2 is next)
#   THIS handoff (seq 13)
#   parent: HANDOFF_engine-hardening_version-reset-0x_2026-06-17.md (seq 12)

# PROCESS GUARDS (in force):
#   - main is PR-ONLY (branch protection). Open a PR, watch CI unmasked, no direct push.
#   - NEVER pipe a gate to tail/head (masks exit code).
#   - Pre-1.0 (0.x): feature/breaking → MINOR, docs/fix → PATCH; never 1.0.0, never 10.x.
#   - merge authority is per-session — re-confirm before merging.
#   - outward/self-modifying actions (publish, GitHub Release, .claude/skills edits,
#     destructive git) need explicit per-action user OK.

# NEXT: Phase 2 — TimeScale + Tween<T>+easing + juice_demo (MINOR 0.12.0).
#   anchors: src/app/schedule.rs (dt), src/tween.rs + src/timer.rs (Lerp),
#   FadeTransition / cam.shake / PostProcessConfig for the juice_demo.
```
