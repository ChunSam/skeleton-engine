# Version reset 10.7.0 → 0.11.0 (pre-1.0) + docs catch-up + tag backfill

**Date:** 2026-06-17
**Status:** COMPLETED — all work shipped (2 CI-green PRs merged, 60 tags pushed, GitHub Release made, branch + memory cleanup done). `main` @ `938c5c2`, package **v0.11.0**, clean, CI green.
**Bead(s):** none (no bead tracker in this repo — `bd` unavailable)
**Epic:** skeleton-engine — engine hardening / release hygiene
**Chain:** `engine-hardening` seq `12`
**Parent:** `HANDOFF_engine-hardening_vision-features-batch_2026-06-17.md` (seq 11)
**Prior chain:** seq 9 (v10 breaking pass) → seq 10 (vision-features-render-split, v10.2.1) → seq 11 (vision-features-batch, v10.3.0–v10.7.0) → **this (12)**

---

## Since Last Handoff

Seq 11 closed the VISION-feature batch (5 features v10.3.0→v10.7.0) and listed 4 options in "Where We're Going": (1) crates.io publish, (2) tag v10.3–10.7, (3) next VISION feature, (4) backfill v5–v9 tags. This session was a **maintenance + strategic-versioning** session, not a feature session:

- Executed **option 4** (tag backfill) — *expanded* to ALL missing tags v5.0.0–v10.7.0 (not just v5–v9), and folded in option 2.
- Added a **docs catch-up** (REFERENCE.html + beginner glossary were stale) — not in seq 11's list, surfaced during investigation.
- Introduced a **NEW strategic decision the user raised mid-session**: was reaching v10.x "overkill"? → led to a deliberate **version reset 10.7.0 → 0.11.0** (a 0.x pre-1.0 line). This supersedes seq 11's whole "10.x" framing.
- **Option 1 (crates.io) still open** — and now reframed as the explicit *point of no return* that a 0.x first-publish gates.
- **Option 3 (next VISION feature)** untouched — the candidate list is still open.
- Merge authority was re-confirmed (per-session) and granted for both PRs this session.

## Reference Documents

- `plans/VERSION_RESET_PLAN_2026-06-17.md` — THIS session's reset plan (target-number rationale, file inventory, go-forward 0.x cadence, tag-wart analysis). Committed to main in PR #91.
- `CLAUDE.md` — agent quick reference (header now v1.6.58 / package v0.11.0; +pre-1.0 cadence note).
- `docs/CHANGELOG.md` — new `## 0.11.0` reset entry; full `0.3.0 → 10.7.0` history preserved.
- `docs/VISION.md` — forkable-skeleton vision (the "honest signal to forkers" reasoning behind the reset).
- `plans/handoffs/HANDOFF_engine-hardening_vision-features-batch_2026-06-17.md` — seq 11 parent (the v10 feature batch this catch-up documents).
- Memory `engine-current-state` / `glossary-obsidian-mirror` — live project-state pointer + the repo↔Obsidian glossary-sync contract.

## The Goal

Keep `skeleton-engine` (forkable, MIT, genre-agnostic 2D wgpu engine) honest and tidy as a release artifact. Concretely this session: (a) backfill the missing release git tags so the history is gapless, (b) bring the two stale reference docs current with the v10 feature batch (incl. the user's Obsidian mirror), and (c) — after the user questioned whether climbing to v10.x was overkill — **reset the version to a 0.x "pre-1.0" line** that honestly signals "the public API is not yet stability-committed," done now because it is free pre-crates.io and impossible-to-do-cleanly afterward.

## Where We Are

- **`main` @ `938c5c2`**, package **v0.11.0** (was v10.7.0 at session start), CLAUDE.md doc header **v1.6.58**, clean working tree.
- **772 lib tests** pass — *unchanged all session* (zero source-code changes; everything was manifests + docs + tags + repo config).
- **Verify gate green** twice (docs branch + reset branch): `all checks passed ✓`.
- **Remote git tags: 60 total**, gapless **v5.0.0–v10.7.0** plus the new **v0.11.0**. (Were 9 at session start: `v0.3.0 v0.4.0 v4.0.0 v4.1.0 v4.2.0 v4.3.0 v10.1.0 v10.2.0 v10.2.1`.) The 59 backfilled tags are annotated, style `vX.Y.Z — <desc>`, on their exact release commits.
- **GitHub Release `v0.11.0`** created, **marked Latest** (badge moved off the prior `v10.2.1` release; `v10.2.1` release object remains but is no longer Latest).
- **2 PRs shipped** (both squash-merged, branches deleted, CI-green):
  - **#90** — docs catch-up (REFERENCE.html + beginner glossary). main → `69f5dfb`.
  - **#91** — version reset 10.7.0 → 0.11.0. main → `938c5c2`.
- **`REFERENCE.html`**: new `<h2>변경 이력 (v9.4–v10.7)</h2>` section (register_inspector_panel, RenderPlugin, ShaderMaterial, Parallax + the 5 batch features, each with code examples); header note `v9.3.0`→`v0.11.0`; sidebar nav link added. Tag balance verified (h2 68/68, pre 170/170, code 1041/1041).
- **`docs/ENGINE_TERMS_FOR_BEGINNERS.md`**: +6 terms (NineSlice, Parallax, Animated Tile, audio Crossfade, Tween Sequence, Coroutine), +1 disambiguation row (`Coroutine` vs `Timeline`/`TweenSequence`), fixed stale paths (`tilemap.rs`→`tilemap/`, `particle.rs`→`particle/`), corrected the "beginning with 1.0.0" semver line.
- **Obsidian mirror updated** (user's vault `/Users/jkl/Documents/메인/skeleton-engine 용어/`): notes `05 렌더링`, `12 타일맵`, `16 오디오`, `21 시간 제어` + `00 인덱스 (MOC)` (summaries, disambiguation, footer → `v0.11.0 기준`).
- **`.claude/skills/ship/SKILL.md`**: bump-inference rewritten to 0.x cadence (breaking → MINOR, never auto-1.0.0).
- **Local branches: only `main`** (deleted 5 stale: `docs/english-conversion`, `feat/v7.1-docked-editor`, `feat/v8-scene-layout-editing`, `feat/v8.1-data-editor`, `fix/macos-mainthread-pacing`; pruned 3 stale remote-tracking refs).
- **Memory**: updated `engine-current-state`, `MEMORY.md`, `local-tooling-skills`; created new `glossary-obsidian-mirror`.
- **Verify gate ran twice** (background, exit 0) — once on the docs branch, once on the reset branch — `all checks passed ✓` each (fmt / clippy --all-targets / wasm lib+bins build / test --all-targets / rustdoc -D warnings). CI then re-ran 4/4 on each PR and once more on main post-merge (`completed / success`).
- **First chain to exercise the seq-11 branch protection end-to-end** — two PRs (#90, #91), both gated by the 4 required checks and merged only on green; the watch was run unmasked both times.
- **crates.io: `skeleton-engine` name still unclaimed / never published.** The CI "Package dry-run" (`cargo package`) passes, so a first publish is mechanically ready — gated only on an explicit user go.
- **Tag push was `git push origin --tags`** (pushes only the 59 not-yet-on-remote tags); the 9 pre-existing remote tags were untouched.
- **Existing tag style discovered + matched:** annotated, message `vX.Y.Z — <short desc>`, tagger `ChunSam` (e.g. `v10.2.0 — Parallax scrolling: ParallaxLayer + ParallaxSystem + example`). The 59 new tags follow it.
- **Doc edits lost Read-tracking across the branch switch** — `Cargo.toml`/`REFERENCE.html` Edits failed "file not read yet" after `git checkout`; re-`Read` then re-`Edit` fixed it (a harness quirk, not a file problem).
- **Both PRs' `git status` confirmed scope** — #90 = only `REFERENCE.html` + glossary; #91 = only the 5 reset files + plan doc. Obsidian / memory / ship-skill changes are outside the repo (absent from git, no PR).
- **No work in flight at close** — the session task list (#1–#11) is fully completed; no scheduled wakeups / cron / loops armed.

## What We Tried (Chronological)

1. **Onboarding.** Read seq-11 handoff; ran `./scripts/verify.sh` as-is (no tail-pipe) → green, 772 tests, main `ad25116` v10.7.0. Read the key files (nine_slice, coroutine, tilemap/animation, sprite 9-quad branch) + adjacent (lib.rs, tilemap/system.rs, examples).
2. **User directive (custom answer to my options question):** "git v9 태그 정리하면서, 프로젝트 내 문서도 최신화, 옵시디언도 확인해서 엔진 용어집 최신화."
3. **Investigation (parallel scans).** Found: tag gap (existing 9; missing v5.x–v9.x + v10.0.0 + v10.3–10.7); built `(version, commit)` map from `git log -p -- Cargo.toml`; REFERENCE.html stale by 7 versions (no ParallaxLayer/ShaderMaterial/batch); README/ROADMAP/NEXT_WORK carry no stale version content; Obsidian vault `/Users/jkl/Documents/메인` has a `skeleton-engine 용어` folder = a 27-note mirror of the glossary.
4. **Asked 2 questions:** tag scope → **전부 (v5–v10.7.0)**; repo-doc merge authority → **CI 그린이면 머지**.
5. **Tags:** generated a dry-run preview (cleaned commit subjects → `vX.Y.Z — desc`), fixed a redundant `5.1.0 — 5.1.0` artifact + the v9.0.0 message by hand, created 59 annotated tags **locally** (held push for confirmation).
6. **REFERENCE.html:** read its structure (sidebar `<details>` nav, `<h3 id>` change-history, escaping conventions), pulled correct API usage from the 5 example sources, verified Parallax/inspector-panel signatures, then added the section + nav + header bump. Validated tag balance.
7. **Glossary:** 9 edits to the repo doc; mirrored into the 4 Obsidian section notes + MOC.
8. **Docs PR #90:** branch `docs/v10-catchup` → commit → push → `gh pr create` → watched 4 checks unmasked (`gh pr checks 90 --watch --fail-fast > log`) → all green → squash-merge → sync main.
9. **Pushed 59 tags** (`git push origin --tags`) → 68 on remote, verified gapless.
10. **Memory** updated (engine-current-state HEAD/tags, MEMORY.md hook, new `glossary-obsidian-mirror` reference memory).
11. **User asked my opinion** on whether v10.x was overkill. Gave a nuanced take; **flipped my recommendation from 0.1.0 → 0.11.0** mid-discussion (signal accuracy).
12. **User asked for a reset plan** → wrote `plans/VERSION_RESET_PLAN_2026-06-17.md` (after scanning for every version reference: no hardcoded versions in `src/`, `engine_reflect_derive` is path-dep/0.1.0, old v0.3/v0.4 tags complicate the 0.x number).
13. **User confirmed:** 0.11.0 / bump CLAUDE doc version / tag now / commit the plan. Executed reset PR #91 (Cargo.toml+lock, CLAUDE.md, CHANGELOG, REFERENCE.html, plan doc) → verify green → push → CI green → squash-merge → tagged + pushed `v0.11.0`.
14. **Post-reset cleanup:** Obsidian MOC footer + memory updated to v0.11.0.
15. **User asked what's pending** → listed backlog (publish, ship-skill check, GitHub Release, next feature, branch cleanup, handoff).
16. **User: "2-6-3-5순서로 진행"** then **"더 우선권 있으면 순서 변경해도 돼."** Did [2] ship-skill 0.x fix (1st edit landed; cosmetic 2nd edit DENIED by classifier — skipped); reordered to **3-5-6** (handoff last). [5] deleted 5 stale branches. [3] GitHub Release (DENIED → user re-confirmed → created, Latest). [6] this handoff.
17. **Explicit confirmations at the right moments** — bundled tag-scope + merge-authority into one `AskUserQuestion`; later a separate explicit release-confirm after the classifier denial. No outward-facing action proceeded without a clear user yes.

## Key Decisions

- **Tag scope = ALL missing v5.0.0–v10.7.0** (user's "전부"), not just v9 — "정리" means a gapless, consistent history; tagging v9 alone would leave v5–v8 + v10 gaps.
- **Keep old v5–v10.7.0 tags after the reset** (do NOT delete) — they map to real commits and are the record. Accept the cosmetic wart: `git tag | sort -V` still ranks `v10.7.0` highest, but `git describe` (commit-distance) and crates.io (10.x never published) behave correctly.
- **Reset target = 0.11.0, not 0.1.0** — the engine is mature in *breadth* but not *stability commitment* (the Bevy-at-0.16 situation); `0.1.0` would undersell the feature set, and `0.11.0` sorts monotonically past the genuine early `v0.3.0`/`v0.4.0` tags. (Rejected 1.0.0: that's a stability *promise* the still-evolving core can't make yet.)
- **Reset NOW, pre-publish** — the only free window. The reset PR is revertible; the irreversible point is the first crates.io publish.
- **Fixed the `ship` skill to 0.x cadence** — its old "breaking → major" rule would auto-bump the next breaking change to **1.0.0** (the exact premature-promise accident). Now: breaking → MINOR, 1.0.0 only on deliberate intent.
- **Handoff reordered to last** (user allowed reordering) so it captures the Release + branch cleanup instead of going stale.
- **Did NOT publish crates.io, did NOT delete the 10.x tags, did NOT create GitHub Releases for historical tags** — conservative; each needs an explicit ask.
- **Edited the Obsidian `.md` files directly** (not via obsidian-cli) — Obsidian picks up file changes live; no PR (vault is outside the repo).
- **Bumped CLAUDE.md doc version v1.6.57 → v1.6.58** alongside the package version — the ship-skill convention (CLAUDE.md itself was edited, so its own leading doc version bumps).
- **Committed the reset plan doc inside PR #91** (not separately) — keeps the rationale next to the change it justifies; the plan is now part of main's history.
- **Skipped the denied cosmetic ship-skill edit** rather than escalating — the substantive 0.x bump-inference fix landed; the leftover was only an example-version string (`v10.2.1`→`v0.11.0`), not worth a permission round-trip.

## Evidence & Data

### PRs shipped (both squash-merged, branches deleted)

| PR | main after | Title | CI |
|---|---|---|---|
| #90 | `69f5dfb` | docs: REFERENCE + beginner glossary up to date (v9.4–v10.7) | 4/4 green |
| #91 | `938c5c2` | chore: reset version line 10.7.0 → 0.11.0 (pre-1.0) | 4/4 green |

### Tag backfill (59 created; key anchors — full list reproducible from `git log -p -- Cargo.toml`)

| Version | Commit | Note |
|---|---|---|
| v5.0.0 | `12deda3d3` | breaking batch (2026-06-10 analysis) |
| v6.0.0 | `25d3387c6` | |
| v7.0.0 | `aa0646e20` | |
| v8.0.0 | `3247a57ad` | scene layout editing |
| v8.27.0 | `6bbd7fc89` | timeline editor MVP (highest v8) |
| v9.0.0 | `7e48794ab` | engine-wide hardening (hand-fixed message) |
| v9.6.1 | `4a4d48842` | highest v9 |
| v10.0.0 | `113f815f7` | tilemap split (the missing v10.0.0) |
| v10.3.0–v10.7.0 | `b818163`…`457e4fb` | the seq-11 batch |
| **v0.11.0** | `938c5c2` | the reset (created at end) |

Remote tag count: **9 → 60**. Existing-before: `v0.3.0 v0.4.0 v4.0.0 v4.1.0 v4.2.0 v4.3.0 v10.1.0 v10.2.0 v10.2.1`.

### Version-reference inventory (the reset's complete change set — verified by scan)

| Location | Old → New |
|---|---|
| `Cargo.toml` L7 | `version = "10.7.0"` → `"0.11.0"` |
| `Cargo.lock` (skeleton-engine, ~L3294) | `10.7.0` → `0.11.0` (via `cargo build`) |
| `CLAUDE.md` L3 | `Version v1.6.57 \| package … v10.7.0` → `v1.6.58 \| … v0.11.0` + cadence note |
| `docs/CHANGELOG.md` | new `## 0.11.0`; "beginning with 1.0.0" → pre-1.0 note; history kept |
| `REFERENCE.html` L282 | `버전 v10.7.0 기준` → `v0.11.0 기준` |
| Obsidian `00 인덱스 (MOC).md` footer | `엔진 v10.7.0 기준` → `v0.11.0 기준 (pre-1.0)` |

Scan facts: **no hardcoded version strings in `src/`/`examples/`; no `CARGO_PKG_VERSION` consumers; `engine_reflect_derive` is a path-only dep (no version pin), independently versioned `0.1.0`.** So source code = zero changes.

### GitHub Releases after

| Release | Latest? | Tag |
|---|---|---|
| v0.11.0 — pre-1.0 version line reset | **Latest** | v0.11.0 |
| v10.2.1 | (no longer Latest) | v10.2.1 |

### CI timings (PR #91, approx)

| Job | Time |
|---|---|
| Build (WASM) | 43s |
| Rustdoc | 49s |
| Package dry-run | 1m12s |
| Test (native) | 3m20s |

### Glossary additions (repo doc + Obsidian mirror, exact)

| Section / Note | Added |
|---|---|
| 렌더링 (05) | Nine-Slice (`NineSlice`), Parallax (`ParallaxLayer`/`ParallaxSystem`) |
| 타일맵 (12) | Animated Tile (`TileAnimation`/`TileAnimationSet`/`AnimatedTileSystem`); 대표 파일 `tilemap.rs`→`tilemap/` |
| 오디오 (16) | Crossfade (`AudioManager::crossfade`) — distinct from the existing *animation* crossfade in 애니메이션 (10), `play_with_crossfade` |
| 시간 제어 (21) | Tween Sequence (`TweenSequence`), Coroutine (`Coroutine`/`CoroutineRunner`/`CoroutineSystem`); 대표 파일 +`coroutine.rs`/`timeline.rs` |
| 헷갈리는 용어 + 00 MOC | row: `Coroutine` vs `Timeline`/`TweenSequence` (imperative closures vs keyframe data vs value interp) |
| 기능별 대표 파일 (repo only) | `tilemap.rs`→`tilemap/`, `particle.rs`→`particle/` |

Repo doc = 9 edits; Obsidian = 4 section notes + MOC (summaries/disambiguation/footer). README/ROADMAP/NEXT_WORK were checked and left untouched (no stale version/feature content).

## Code Analysis

- **No logic code read for behavior this session** — it was a docs/manifest/tag session. The "analysis" was a *version-reference inventory* (see Evidence table) and confirming the reset surface is tiny.
- `engine_reflect_derive` (workspace member) has **always been `0.1.0`**, never synced to the main package — so after the reset both happen to read `0.x` but they remain independently versioned; the main crate depends on it via `path` only (`Cargo.toml:160`), no version requirement.
- REFERENCE.html structure: `<nav class="sidebar">` with `<details>` groups of `<a href="#anchor">`; body `<h2 id>`/`<h3 id>` sections; code blocks `<pre><code class="language-rust">`; generics escaped `&lt;`/`&gt;`, ampersands `&amp;`. Change-history uses `<h3 id="vX.Y.Z…">` per version (those keep their historical version numbers — e.g. `v10.7.0 — NineSlice` stays).
- `ship` skill bump logic lives in `.claude/skills/ship/SKILL.md` "Preconditions" — now pre-1.0 aware.
- **Obsidian note format** (for the next glossary edit): YAML frontmatter (`tags`/`aliases`/`related` wikilinks) + a `용어 | 쉬운 뜻 | 이 엔진에서의 이름` table + `> [!note] 대표 파일` and `> [!example]/[!tip]/[!info]` callouts + footer nav `→ 다음: [[..]] · 돌아가기: [[00 인덱스 (MOC)]]`; MOC footer `*출처: … · 엔진 vX.Y.Z 기준*`. 27 notes total (`00 MOC` + `01`–`26`).
- **REFERENCE.html escaping** (for the next API doc edit): code in `<pre><code class="language-rust">`; generics `&lt;`/`&gt;`, ampersand `&amp;`; nav = `<nav class="sidebar">` of `<details>` groups holding `<a href="#anchor">`; append new sections just before the final `</main>`. Korean prose (`lang="ko"`) — match it (the glossary + this file are the doc-language Korean exceptions; REFERENCE.html is pre-existing Korean).

## Files Changed

### Repo — PR #90 (docs)
- `REFERENCE.html` — +변경 이력 (v9.4–v10.7) section, nav link, header version.
- `docs/ENGINE_TERMS_FOR_BEGINNERS.md` — +6 terms, +1 disambiguation row, path fixes.

### Repo — PR #91 (reset)
- `Cargo.toml`, `Cargo.lock` — 10.7.0 → 0.11.0.
- `CLAUDE.md` — version refs + 0.x cadence note.
- `docs/CHANGELOG.md` — `## 0.11.0` reset entry + semver line fix.
- `REFERENCE.html` — header → v0.11.0.
- `plans/VERSION_RESET_PLAN_2026-06-17.md` — NEW (the plan).

### Local-only (NOT in repo / not in a PR)
- `.claude/skills/ship/SKILL.md` — 0.x cadence bump-inference (gitignored local skill).
- Obsidian vault `/Users/jkl/Documents/메인/skeleton-engine 용어/`: `05 렌더링.md`, `12 타일맵과 경로 탐색.md`, `16 오디오.md`, `21 시간 제어.md`, `00 인덱스 (MOC).md`.

### Memory (`~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/`)
- `engine-current-state.md` — HEAD/version/reset note (+ "do NOT revert upward").
- `MEMORY.md` — hook lines updated.
- `glossary-obsidian-mirror.md` — NEW (reference: keep repo glossary + Obsidian in sync).
- `local-tooling-skills.md` — ship-skill 0.x update logged.

### Repo config (GitHub, not a file)
- 59 annotated tags pushed; `v0.11.0` tag + GitHub Release (Latest).

## User Feedback & Preferences (REQUIRED)

- **Custom 3-part directive:** "git v9 태그 정리하면서, 프로젝트 내에 있는 문서도 최신화 해줘. 내 옵시디언도 확인해서 엔진 용어집도 최신화 해줘." (Note: "v9" was shorthand — they confirmed "전부" when asked.)
- **Tag scope = 전부 (v5–v10.7.0)**; **merge = CI 그린이면 머지** (per-session merge authority).
- **Values honesty / wants real opinions:** asked "내가 10.0.0까지 간 게 오버 아닌가? 어떻게 생각해? 의견 내봐" — wanted a genuine verdict, not validation. (Carry: give reasoned opinions, flag when my own recommendation changes — I flipped 0.1.0→0.11.0 and said so.)
- **Reset decisions:** 0.11.0 / bump CLAUDE doc version / tag now / commit the plan.
- **Sequencing:** "2-6-3-5순서로 진행" then **"더 우선권 있는 작업 있으면 순서 변경해도 돼"** (gave latitude to reorder — used it to move handoff last).
- **Re-confirmed the GitHub Release** explicitly after the safety classifier blocked it.
- **Standing (carried):** Korean prose to user / English code+docs+handoff; Sonnet subagents w/ explicit model; never tag/publish unprompted; **merge authority re-confirm each session**; the beginner glossary is the Korean-doc exception.

## Where We're Going

(No work in flight — session complete. Options for next session, roughly ordered.)
1. **crates.io first publish as 0.x** — the reset's whole purpose was to enable a clean `0.11.0` first publish. **Irreversible (point of no return); needs explicit user go.** Verify `cargo publish --dry-run` (the CI "Package dry-run" already passes) then `cargo publish`.
2. **Validate the `ship` skill on the next real bump** — the 0.x cadence fix is untested against an actual release; first use should confirm it bumps MINOR (not major) on a feature/breaking change.
3. **Next VISION feature** — candidate list still open (dialogue/textbox, screen-shake presets, tilemap layers, gamepad rumble, save-slot UI, particle trails…); run the feature→example loop.
4. **(optional) GitHub Releases for select historical tags** — only `v0.11.0` and `v10.2.1` have Release objects; the rest are bare tags. Cheap, on request.

## crates.io Publish Prerequisites (for the #1 next action)

The reset enables a clean first publish as `0.11.0`, but publishing is NOT just `cargo publish` — there is a non-obvious blocker:

- **Path-dep blocker (must fix first):** `skeleton-engine` depends on the workspace member `engine_reflect_derive` via **`path` only** (`Cargo.toml:160`, no version). `cargo publish` rejects path-only deps. Options: (a) publish `engine_reflect_derive` (0.1.0) to crates.io FIRST, then add `version = "0.1.0"` to the dep line; or (b) the standard workspace-publish pattern — keep `path` for local dev *and* add a `version` so crates.io is satisfied (dual `path` + `version`). Either way `engine_reflect_derive` must exist on crates.io.
- **Name availability:** `skeleton-engine` appears unclaimed (never published); confirm with `cargo search skeleton-engine`. First publish is the irreversible name claim.
- **Metadata for the crates.io page:** `Cargo.toml` already has `description`/`repository`; verify `license` (MIT per VISION), `readme`, and add `keywords`/`categories` for discoverability. The CI "Package dry-run" passes but doesn't grade metadata quality.
- **Order:** reset (done) → fix the path-dep → `cargo publish --dry-run` → `cargo publish`. Irreversible; explicit user go required.

## Risks & Blockers

- **main is PR-only (branch protection, enforce_admins).** All changes (even docs) go through a PR whose 4 CI checks pass (`Build (WASM)`/`Test (native)`/`Rustdoc`/`Package dry-run`); `strict=true` ⇒ rebase if main moved. No admin override.
- **The auto-mode classifier gates outward-facing / self-modifying actions** — this session it DENIED a cosmetic edit to `.claude/skills/ship/SKILL.md` and the `gh release create` until the user re-confirmed. Expect to need *explicit per-action* user authorization for: GitHub Releases, crates.io publish, editing files under `.claude/skills/`, and destructive git. A general "/goal" or task-list selection may not satisfy it.
- **crates.io publish is irreversible** — first-ever name claim; 0.x must be the first version published (never publish 10.x, it would outrank 0.x forever).
- **ship skill 0.x cadence is untested** on a real bump (see Where We're Going #2).
- **crates.io publish has a hidden blocker** — the `engine_reflect_derive` `path`-only dep (see Publish Prerequisites). `cargo publish` will fail until it's resolved; easy to miss because local builds + CI "Package dry-run" both pass.
- **`ship` skill 0.x cadence is untested on a real bump** — first use must confirm it bumps MINOR (not major) and never auto-1.0.0.

## Open Questions

- Publish to crates.io as 0.11.0? (default: no — explicit go needed; it's the point of no return.)
- Next VISION feature, or pause feature work? (user picks.)
- Backfill GitHub Release objects for historical tags, or leave bare? (default: leave.)
- When publishing: publish `engine_reflect_derive` (0.1.0) to crates.io first, or restructure the dep (fold in / dual path+version)? (path-only dep blocks `cargo publish` of `skeleton-engine`.)

## Why the Reset (the opinion exchange — strategic core)

The user asked, unprompted: "큰 엔진들도 1.0.0을 망설이는데 내가 10.0.0까지 간 게 오버 아닌가?" The reasoning that drove the decision (capture in case it's revisited):

- **The 10.x bumps were NOT wrong** — each major was a real SemVer breaking change (v5/v6/v7/v8/v9/v10 batches). So it isn't "version inflation"; it's correct SemVer.
- **But big engines stay at 0.x deliberately** (Bevy at 0.16, the 0ver.org culture) — not because they break *less*, but to **avoid the 1.0 stability *promise*** while still breaking freely. 0.x = "anything may change."
- **The real issue is signal mismatch**, not number size: `v10.7.0` implies "10 stable epochs," but the project is single-author, never-published, still adding core features in batches — i.e. genuinely 0.x-stage. A high major *oversells maturity to forkers* (priority-1 audience) and can *reduce* credibility.
- **0.11.0, not 0.1.0** (I flipped my own initial 0.1.0 recommendation mid-discussion and said so): the engine is mature in *breadth* but not *stability commitment* — `0.1.0` undersells the feature set; a higher 0.x (Bevy-style) is the accurate signal, and sorts past the old `v0.3/v0.4` tags.
- **Now is the only free window** — pre-crates.io. After publish, the highest version always wins and a reset is permanently confusing. So: reset → *then* publish as 0.x. Publish is the point of no return; the reset PR itself is revertible.

## Reusable Gotchas (carry forward)

- **NEVER pipe a gate command to `tail`/`head`.** `./scripts/verify.sh | tail` and `gh pr checks <n> --watch | tail` set the pipeline exit code to `tail`'s (always 0), masking failures (caused seq 11's red merge). Run `cmd > /tmp/x.log 2>&1` with NO trailing pipe so the background task's own exit code is authoritative; then read the log AND eyeball `gh pr checks <n>`. Did this cleanly twice (PR #90, #91 both 4/4 green before merge).
- **The auto-mode safety classifier gates outward-facing + self-modifying actions.** Two denials this session: (1) a *cosmetic* edit to `.claude/skills/ship/SKILL.md` (self-modification of a behavior-controlling file, deemed unauthorized); (2) `gh release create` (outward publish). **Resolution that worked:** get an *explicit per-action* user "yes," then retry — the release went through immediately after re-confirmation. A task-list pick ("do item 3") or a broad directive does NOT satisfy it. Do not try to bypass an outward-facing gate; STOP and ask.
- **main is branch-protected: PR-only, `enforce_admins`, 4 required checks, `strict`.** Even docs/version-only changes need a PR + green CI (they pass trivially since no Rust code changes, but must still run). No admin override; rebase if main moved.
- **Versioning is PRE-1.0 (0.x).** Breaking → MINOR (`0.Y.0`), fix → PATCH; 1.0.0 only on deliberate intent. The `ship` skill was updated to enforce this — but it's untested on a real bump. Do NOT auto-bump to 1.0.0, do NOT revert to 10.x.
- **Obsidian edits = direct `.md` writes** (vault outside the repo, no PR; Obsidian reloads live). The glossary repo doc is source-of-truth; vault notes derived — sync both (`glossary-obsidian-mirror` memory).

## Non-obvious Technical Findings

- **The old `v0.3.0`/`v0.4.0` tags forced the reset number up.** `0.1.0` would sort *below* genuine 2024-era 0.x tags (non-monotonic) and undersell breadth; `0.11.0` sorts above them and reads as "post-10.x, pre-1.0." That's *why* 0.11.0.
- **A version "decrement" is harmless here ONLY because there are no consumers.** crates.io never had 10.x; `git describe` (commit-distance) picks HEAD's newest tag (`v0.11.0`); only a literal `git tag | sort -V` still shows `v10.7.0` highest — cosmetic. Lost once published (crates.io always serves the highest version).
- **`engine_reflect_derive` was never version-synced** to the main crate (always `0.1.0`); `path`-only dep, no version requirement → the reset touched only `skeleton-engine`'s version.
- **Tag messages were auto-derived** from commit subjects: strip the conventional-commit prefix + trailing `(#NN)`/`(vX.Y.Z)` + a redundant leading `X.Y.Z — ` (the `chore(release): 5.1.0 — …` subjects produced `5.1.0 — 5.1.0 — …`); `v9.0.0` was hand-written. Reproducible: `git log --reverse -p -- Cargo.toml` → first `+version` per version → its commit.
- **REFERENCE.html change-history `<h3>` entries keep historical version numbers** (e.g. `v10.7.0 — NineSlice` stays); only the header "버전 … 기준" line tracks the current version. The post-reset `10.7.0` grep correctly retains those + CHANGELOG history.
- **Zero source-code change all session** — the reset surface is manifests + docs only (no hardcoded versions in `src/`, no `CARGO_PKG_VERSION` consumers), so the 772-test count and CI stayed constant.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -6           # 938c5c2 reset; 69f5dfb docs; ad25116 seq-11 handoff
grep -m1 '^version' Cargo.toml # 0.11.0
git status -s                  # clean
git tag | sort -V | tail -5    # …v10.7.0 v0.11.0 (v10.7.0 sorts highest — cosmetic, see Key Decisions)
./scripts/verify.sh            # green, 772 lib tests; RUN AS-IS, do NOT pipe to tail

# Read first
#   THIS handoff (seq 12)
#   plans/VERSION_RESET_PLAN_2026-06-17.md   (reset rationale + 0.x cadence policy)
#   parent: HANDOFF_engine-hardening_vision-features-batch_2026-06-17.md (seq 11)
#   docs/CHANGELOG.md → ## 0.11.0

# PROCESS GUARDS (in force):
#   - main is PR-ONLY (branch protection). No direct push; open a PR, let CI gate it.
#   - NEVER pipe a gate to tail/head: `verify.sh | tail`, `gh pr checks --watch | tail`
#     mask the exit code (=tail's 0). Run `cmd > /tmp/x.log 2>&1`, then eyeball.
#   - Versioning is PRE-1.0 (0.x): MINOR = any release incl. breaking, PATCH = fix.
#     Do NOT bump to 1.0.0 on a breaking change. Do NOT revert the version up to 10.x.
#   - Outward-facing/self-modifying actions (crates.io publish, GitHub Release,
#     .claude/skills edits, destructive git) need EXPLICIT per-action user OK
#     (the safety classifier blocks them otherwise — happened twice this session).
#   - merge authority is per-session — re-confirm before merging anything.

# Next action (pick one — no work in flight):
#   (a) crates.io first publish as 0.11.0 — ONLY on explicit user go (irreversible).
#   (b) Propose/scope the next VISION feature (candidate list open).
#   (c) Nothing — session was a clean close.
```

## Session Closed

**Closed at:** 2026-06-17 (KST)
**Commit:** this handoff committed to main via its own PR (main is branch-protected, PR-only).
**Session status:** Handed off to next session (seq 12). The docs/versioning-maintenance session is COMPLETE — `main` @ `938c5c2`, package **v0.11.0** (pre-1.0); all release tags backfilled (`v5.0.0`–`v10.7.0` + `v0.11.0`, gapless); REFERENCE.html + beginner glossary (+ Obsidian mirror) current; GitHub Release `v0.11.0` is Latest; `ship` skill on 0.x cadence; 5 stale branches cleaned. No work in flight; no wakeup armed. **Next: crates.io first publish as 0.11.0 — resolve the `engine_reflect_derive` path-dep blocker first; explicit user go required (irreversible).** Merge authority was per-session — re-confirm next session.
