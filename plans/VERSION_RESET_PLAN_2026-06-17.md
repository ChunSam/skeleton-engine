# Version Reset Plan — 10.7.0 → 0.x (pre-1.0 line)

**Date:** 2026-06-17
**Status:** PROPOSED — awaiting target-number sign-off before execution
**Scope:** version renumber only. **Zero source-code changes.**

## Context

`skeleton-engine` is at **v10.7.0** after ~10 SemVer-major epochs of real breaking
changes. It is **never published to crates.io** and has **no external consumers**
(rust-survivors deprecated). The high major version implies a stability/maturity cadence
the project does not actually have (still adding core features in batches, single author,
frequent breaks). Goal: move to a **0.x "pre-1.0" line** that honestly signals "API not
yet stability-committed, expect breakage" — the Bevy / 0ver norm — while it is still free
to do (pre-publish).

## Why now (and what's reversible)

- crates.io was never published → resetting is essentially **free right now**. After the
  first publish the version history is permanent, and on crates.io the **highest** version
  always wins, so a later reset becomes genuinely confusing.
- The reset PR itself is revertible. **The point of no return is the first crates.io
  publish**, not the reset.

## KEY DECISION — target number

0.x in SemVer means "anything may change" — exactly the signal we want. Which 0.x:

| Option | Reads as | Tag-sort vs old v0.3/v0.4 | Verdict |
|---|---|---|---|
| **0.11.0** | "feature-rich, much iterated, not yet stable" (Bevy-style) | sorts **above** (monotonic) | **recommended** |
| 0.1.0 | "clean slate / very early" | sorts **below** (cosmetic quirk) | alt — risks underselling real breadth |

**Why 0.11.0:** the engine is *mature in breadth* (ECS, physics, editor, networking,
animation, tilemap, audio…) but *immature in stability commitment*. `0.1.0` conflates the
two and undersells the feature set; a higher 0.x (Bevy sits at 0.16) accurately says "a lot
is built, no compatibility promise yet." It also keeps the 0.x tag sequence monotonic past
the genuine early `v0.3.0`/`v0.4.0` tags. `0.1.0` stays valid if you prefer maximal-humility.

**Do NOT reset to 1.0.0** — that is the opposite signal (a compatibility *promise*), and the
core is still moving. Save 1.0 for when the API settles.

## Go-forward versioning policy (post-reset, 0.x)

Replace the current "feature→minor, breaking→major" with the standard 0.x cadence:

- **MINOR** (0.11.0 → 0.12.0): any release — new feature OR breaking change (0.x license to break).
- **PATCH** (0.11.0 → 0.11.1): bugfix / docs / no-API point release.
- **1.0.0** later = deliberate "we now honor compatibility" milestone.
- Update CLAUDE.md's versioning note. The `ship` skill's "feature → minor" still holds
  (minor is now also the breaking lever — no skill code change needed).

## The existing v5–v10.7.0 tags (honest wart)

**Keep them.** They map to real release commits and are the historical record. Accept:

- `git tag | sort -V` will still rank **v10.7.0 highest** — purely cosmetic.
- `git describe --tags` is commit-distance based → correctly picks the new 0.x tag on HEAD.
- crates.io is unaffected (10.x was never published; the first publish will be 0.x and canonical there).
- GitHub "Latest release" can be set manually to the 0.x release if Releases are used.

Do **not** delete the 10.x tags — deletion erases the record and contradicts the backfill
just completed, for no real gain.

## File change checklist (complete inventory)

Verified by scan: **no hardcoded version strings in `src/`/`examples/`, no
`CARGO_PKG_VERSION` consumers, `engine_reflect_derive` is a path-only dep (no version pin)**.
So only manifests + docs change.

1. `Cargo.toml` L7 — `version = "10.7.0"` → `"0.11.0"`. (`engine_reflect_derive` stays `0.1.0`; path dep line `Cargo.toml:160` unaffected.)
2. `Cargo.lock` — skeleton-engine entry (L3294) → `0.11.0` (run `cargo build`, or `cargo update -p skeleton-engine --precise 0.11.0`, or hand-edit).
3. `CLAUDE.md` L3 — `package skeleton-engine v10.7.0` → `v0.11.0`; bump the CLAUDE doc version `v1.6.57` → `v1.6.58` (convention); add a one-line 0.x-cadence note to the versioning section.
4. `docs/CHANGELOG.md` — prepend a new `## 0.11.0` section with the reset note (below); **keep all existing history** (`## 10.7.0` … `## 0.3.0`) untouched.
5. `REFERENCE.html` L282 — `버전 v10.7.0 기준` → `버전 v0.11.0 기준`.
6. Obsidian MOC footer (`/Users/jkl/Documents/메인/skeleton-engine 용어/00 인덱스 (MOC).md`) — `엔진 v10.7.0 기준` → `v0.11.0 기준` (vault edit, **no PR**).
7. Memory — `engine-current-state.md` + `MEMORY.md` hook → new version + reset note.
8. (after merge) annotated git tag `v0.11.0` on the merge commit, pushed.

## Proposed CHANGELOG entry

```
## 0.11.0

**Version line reset: pre-1.0.** No code changes. The project moves from the 10.x SemVer
line back to a 0.x ("pre-1.0") line to honestly signal that the public API is not yet
stability-committed and may break between releases — matching the engine's actual state
(feature-rich but still evolving, single author, never published). The full prior history
(0.3.0 → 10.7.0) is preserved below and in git tags; only the go-forward line is
renumbered. 0.x cadence: MINOR = any release (incl. breaking), PATCH = point fix. 1.0.0
will mark a deliberate compatibility commitment later.
```

## Execution sequence (main is branch-protected → PR-only)

1. **Confirm target number** (0.11.0 vs 0.1.0) — blocking.
2. Branch `chore/version-reset-0x` off main.
3. Apply changes 1–5 (manifests + CLAUDE + CHANGELOG + REFERENCE); `cargo build` to refresh Cargo.lock.
4. Local gate: `./scripts/verify.sh` run as-is (no tail-pipe). Expect green — version string only.
5. Commit, push, `gh pr create`. Watch the 4 CI checks unmasked (`gh pr checks <n> --watch --fail-fast > log 2>&1`).
6. Squash-merge when green (re-confirm merge authority).
7. Sync main; create + push annotated tag `v0.11.0` on the merge commit.
8. Edit the Obsidian MOC footer (vault).
9. Update memory.

## Verification

- `grep -rn '10\.7\.0' Cargo.toml Cargo.lock CLAUDE.md REFERENCE.html` → no hits (only CHANGELOG history retains it).
- `cargo metadata --no-deps` shows `0.11.0`; `cargo build` succeeds.
- PR CI: all 4 checks green.

## Out of scope / downstream

- **crates.io publish** — the reset *enables* a clean first publish as 0.x, but publishing
  is a separate, explicit, irreversible decision. Do it only when committed to the 0.x line.
- **GitHub Releases** — optional; if one is made for v0.11.0, mark it "Latest."

## Open decisions to confirm before executing

1. **Target number: 0.11.0 (recommended) or 0.1.0?**
2. Bump CLAUDE.md doc version v1.6.57 → v1.6.58? (convention: yes.)
3. Create the `v0.11.0` git tag now, or defer until first publish?
4. Is this a real go, or decision-support only? (This doc does not execute anything.)
