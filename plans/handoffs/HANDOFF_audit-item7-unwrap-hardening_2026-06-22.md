# Audit deferred item 7 — ecs/world.rs unwrap hardening (v0.50.1, behavior-preserving)

**Date:** 2026-06-22
**Status:** COMPLETED + merged. main @ `b4e8c5d`, package **v0.50.1**, clean tree, full gate green, CI green, squash-merged (#195).
**Bead(s):** none
**Epic:** engine-audit deferred-items follow-up arc (seq-1 audit #184)
**Chain:** `standalone-4365aa4a` seq `5`
**Parent:** `HANDOFF_audit-item6-texture-format_2026-06-22.md` (seq 4)
**Prior chain:** `HANDOFF_engine-audit-fixes_2026-06-22.md` (seq 1) > `HANDOFF_audit-followup-refactors_2026-06-22.md` (seq 2) > `HANDOFF_audit-deferred-editor-tier5_2026-06-22.md` (seq 3) > `HANDOFF_audit-item6-texture-format_2026-06-22.md` (seq 4) > this (seq 5)
**Auto:** false

---

## Stale References

Parent (seq 4) identifiers all still resolve. One anchor from the parent is now historical:

- The parent's Appendix-style note that `ecs/world.rs` had `~51` raw `.unwrap()` "still holds" — **no longer true after this session**: the count is now **0** (all converted to `.expect(...)`). The 1 intentional `.unwrap_err()` (binary_search insertion position, `world.rs:298`) remains.
- `src/renderer/texture.rs` item-6 anchors (from seq 4) untouched this session — still valid.

## Since Last Handoff (seq-4 plan vs reality)

Parent's "Where We're Going" named **item 7** as the single last open audit item, with an explicit approach: *"reason per-unwrap, do NOT blanket-replace … most are structural archetype invariants … genuinely infallible → at most want an `expect("<invariant>")`. A few (entity allocation / generation / free-list paths around `world.rs:150–208`) are where real edge-input reachability needs checking + a guard + a regression test."* This session executed exactly that — with one **evidence-based correction** to the hypothesis:

- **Did item 7 in full.** All 51 raw `.unwrap()` → `.expect("<invariant>")`. Landed as PATCH **v0.50.1** (#195).
- **Corrected the parent's hypothesis:** the entity **alloc/generation/free-list paths the parent flagged as the likely edge-reachable ones were already defensively guarded** (`spawn`/`despawn` use `.get_mut()` + `if let Some` + a `u32::MAX` overflow guard + early-return on missing `entity_location`). So there were **ZERO edge-reachable unwraps** → **no guard + no regression-test work**. Item 7 reduced to a pure `expect`-documentation pass.
- **Closed the list.** With item 7 done, the **seq-1 engine-audit deferred list (items 1–8) is FULLY CLOSED** (1–4 + 6 + 7 + 8 done, 5 rejected as needing a facade feature).
- **Open question answered:** seq-4 asked "item 7 next or pause the audit arc?" — user said proceed (full pass).

## Reference Documents

- `CLAUDE.md` — module map (`ecs/world.rs` row), verify rules, pre-1.0 versioning (PATCH = internal refactor/no-API-change, which this was).
- `plans/handoffs/HANDOFF_engine-audit-fixes_2026-06-22.md` (grandparent seq 1) — Appendix C item 7 reasoning + the `[profile.release] panic="abort"` weighting lens that justifies the whole pass + the **WindowConfig ~70-call-site** breaking-field constraint.
- `docs/PATTERNS.md` — ECS query API + borrow-workaround patterns (the disjoint-borrow code in `query2_mut`/`query3_mut` lives here conceptually).

## The Goal

Close the seq-1 engine-audit deferred-item list (items 1–8). This session targeted the last open one, **item 7: `src/ecs/world.rs` raw `.unwrap()` hardening**. Under `[profile.release] panic = "abort"`, a fired `unwrap` aborts the whole game process with **no unwind** and only a generic `called Option::unwrap() on a None value` + line number. The goal: convert each to `.expect("<invariant>")` that **names the structural invariant** making it infallible — so if a future refactor ever breaks one, the abort message is actionable triage instead of a mystery. Behavior-preserving (no public API change, no behavior change); the existing ECS test suite is the acceptance test and stays green + untouched.

## Where We Are

- **main @ `b4e8c5d`, package v0.50.1, CLAUDE.md header v1.6.119, clean tree, `./scripts/verify.sh` → exit 0** (fmt + clippy `-D warnings` native + wasm32 lib/bins build + `test --all-targets` + doc `-D warnings`), run twice (after edits, and after the version bump).
- **PR #195 squash-merged** on green CI (Build WASM 41s · Package dry-run 1m5s · Rustdoc 35s · Test native 4m3s), branch `fix/ecs-world-unwrap-hardening` deleted, local main fast-forwarded.
- **`src/ecs/world.rs`: raw `.unwrap()` 51 → 0**; `.expect(` 5 → **56** (5 pre-existing from a prior session in `query2_mut`/`query3_mut` + 51 new); 1 intentional `.unwrap_err()` kept.
- **17 functions touched** (all the public/`pub(crate)` query + mutation + parallel-query methods). Each unwrap site falls into exactly one of two infallible families (see Code Analysis).
- **Zero guard/test changes** — the alloc/generation/free-list paths were already guarded, so no new code-flow and no new test. The 43-test ECS suite (`src/ecs/world/tests.rs` 43 fns + `commands.rs` tests) passed unchanged.
- **No public API change.** `expect` and `unwrap` are codegen-identical on the happy path; the message string only materializes on the cold panic branch → zero runtime cost.
- **Release paperwork done:** `Cargo.toml` 0.50.0→0.50.1, `Cargo.lock` refreshed (`cargo update -p skeleton-engine`), `docs/CHANGELOG.md` 0.50.1 entry (`### Changed (internal)`), `CLAUDE.md` header v1.6.118→v1.6.119 + package v0.50.0→v0.50.1.
- **Memory bumped:** `engine-current-state` → seq 68 (frontmatter description + lead paragraph + new seq-68 bullet + seq-60 stale-tail fixed); `MEMORY.md` index line refreshed.
- **The seq-1 audit deferred list is now fully closed.** This is the terminal handoff of the audit follow-up arc (seqs 60–68 / handoff seqs 1–5).

## What We Tried (Chronological)

1. **Onboarding (read-only).** Read the full seq-4 parent, the grandparent seq-1 (Appendix C item 7 + panic=abort lens), `src/ecs/world.rs` (entire 1102 lines), adjacent files `ecs/mod.rs`, `ecs/world/tests.rs` (43 test fns), `ecs/commands.rs`. Checked the dungeon-merchant wishlist board → **`_None open._` (next ID EW-002)** → no EW request preempts item 7.
2. **Line-by-line unwrap census.** `grep -n '\.unwrap()'` → exactly 51 sites + 1 `.unwrap_err()` + 5 existing `.expect(`. Classified every one of the 51 by hand. **Finding: all 51 are infallible structural invariants** — none edge-reachable.
3. **Audited the parent's "suspect" paths directly.** `spawn` (`world.rs:144`): line 153 `self.generations[index as usize]` is index access (not unwrap) and infallible by construction (every index either just had a generation pushed, or came off the free-list which is only populated *after* a generation exists). `despawn` (`world.rs:165`): line 200 uses `self.generations.get_mut(...)` guarded by `if let Some` + a `u32::MAX` overflow guard, and the fn early-returns on a missing `entity_location`. **Conclusion: already guarded → no guard/test work.** This is the key correction to the parent's hypothesis.
4. **Narrated onboarding to the user (5 steps) + asked a scope question** (full pass vs light pass), recommended full pass, then **waited for go-ahead** per the resume-prompt protocol.
5. **User said "전체패스진행" (full pass).** Branched `fix/ecs-world-unwrap-hardening` off `416b67a`.
6. **Decided to skip a separate baseline verify run** (clean merged main ⇒ baseline guaranteed green; the final gate is the authoritative check and would catch any fmt/syntax slip). Stated this to the user.
7. **Applied 17 block edits**, one per touched function, using full-closure-block `old_string`s for uniqueness (many `.unwrap()` lines are textually identical across functions, e.g. `c.downcast_ref::<T>().unwrap()`). Used a small set of family-shared `expect` messages rather than 51 bespoke strings, for hot-loop readability.
8. **`grep` re-census → 0 raw unwraps, 56 expects.** Ran `cargo fmt` (some `expect` lines exceeded width and got reflowed; +114/−51 in the diff vs the conceptual 51-line change).
9. **Full gate → VERIFY_EXIT=0** (`/tmp/verify_unwrap.log`).
10. **`/ship` paperwork** (the four edits), `cargo update -p skeleton-engine`, **re-ran gate → VERIFY_EXIT=0** (`/tmp/verify_ship.log`).
11. **`/land-pr` loop:** commit `98dc094` → push → `gh pr create` (#195) → `gh pr checks 195 --watch --fail-fast --interval 30 > /tmp/checks_195.log 2>&1` (background, exit 0) → confirmed `mergeStateStatus: CLEAN` → `gh pr merge --squash --delete-branch` (merge commit `b4e8c5d`) → `git pull --ff-only` → bumped memory to seq 68.

## Key Decisions

- **Full pass (all 51), not a light touch.** Offered the user both; recommended full to cleanly close the audit list. User chose full ("전체패스진행"). A light pass would have documented only the load-bearing mutation-path unwraps and left the obvious per-entity downcasts as a module doc-comment.
- **Family-shared `expect` messages, not 51 bespoke strings.** Two canonical message shapes: `"<fn>: archetype was filtered to contain this column"` (or `column A/B/C/D`) for column-presence, and `"column holds type A"` (/B/C/D/T) for downcasts; plus per-fn variants for the own-`type_set` sites (`despawn`/`move_entity`/`add_component`). Rationale: these run in hot query loops; readability of the closure matters more than a unique sentence per call.
- **Left the `[i]` / `[row]` index operations alone.** `ca[i]`, `col[row]` are panic-on-OOB but are NOT `.unwrap()` and were NOT in the audit's "56 unwrap" count. They're guaranteed in-bounds by the archetype length invariant (entities and columns are kept equal-length). Converting them to `.get(i).expect()` changes the access pattern + iterator types → real regression risk for the same infallible-invariant rationale. Out of scope for item 7; documented here as the boundary.
- **Kept the 1 `.unwrap_err()`** (`world.rs:298`, `sig.binary_search(&tid).unwrap_err()`): it deliberately wants the `Err(insertion_index)` — `binary_search` returns `Err` for a not-found key, which is the whole point (the tid is known-absent here). Not a panic-on-None; left as-is.
- **No guard + no regression test added.** Because the census found zero edge-reachable unwraps, there was nothing to guard. Adding a test for an infallible invariant would assert a tautology. The existing 43 ECS tests already cover free-list reuse, generation increment, and stale-handle rejection (`spawn_reuses_freed_index_with_incremented_generation`, `stale_handle_cannot_mutate_reused_entity`, `commands_ignore_stale_handles`).
- **PATCH bump v0.50.1, not MINOR.** Pure internal refactor, no public API change → PATCH, consistent with the other refactor-only audit items (seq 65 v0.49.3 theming, seq 66 v0.49.4 tier-5). (Pre-1.0: PATCH = internal/no-API; MINOR = feature or breaking.)
- **Skipped a separate baseline verify run.** Clean merged main ⇒ baseline green is guaranteed; the post-edit gate is the meaningful check (unwrap→expect cannot break a green tree except via fmt/syntax, which the gate catches). Saved ~5 min serial. Stated to the user.
- **No new example.** Per VISION, "a feature is not done until an example exercises it" — but this is a behavior-preserving hardening refactor (the `/split-module` category), not a feature. The ECS test suite is the acceptance test. Matches how sibling audit items (61–66) were landed.
- **Drove end-to-end through merge** (ship + land + squash-merge + memory bump) on one go-ahead, per the standing merge-authority delegation and the seq-4 precedent.

## Evidence & Data

**Census (before → after):**

| Metric | Before | After |
|---|---|---|
| raw `.unwrap()` in `world.rs` | 51 | **0** |
| `.expect(` in `world.rs` | 5 | **56** |
| `.unwrap_err()` (intentional) | 1 | 1 |
| ECS tests (unchanged + green) | 43 | 43 |
| functions touched | — | 17 |

**The two infallible families (all 51 sites):**

| Family | Why infallible | Example sites |
|---|---|---|
| (a) column presence | `columns.get(&tid)` after an `arch.contains(tid)` filter, or `tid` ∈ the archetype's own `type_set` | `query`/`query2..4`/`query_with`/`query_without`/`query_opt2`/`query_mut`/all `par_query*` (contains-filter); `despawn`/`move_entity`/`add_component` (own type_set) |
| (b) downcast | `downcast_ref/mut::<T>()` on a column fetched by `TypeId::of::<T>()` — the column can only hold `T` | every `(e, c.downcast_*::<T>().unwrap())` map closure |

**Commit / merge:**

| Item | Value |
|---|---|
| Branch | `fix/ecs-world-unwrap-hardening` (deleted post-merge) |
| Local commit | `98dc094` |
| Merged squash commit | `b4e8c5d` |
| PR | #195, `mergeStateStatus: CLEAN`, squash + branch-deleted |
| CI | Build WASM 41s · Package dry-run 1m5s · Rustdoc 35s · Test native 4m3s — all pass |
| Version path | v0.50.0 → **v0.50.1** (PATCH) |

**Diffstat (merge):** 5 files, +124/−54. `src/ecs/world.rs` +114/−51 (the +/− asymmetry is `cargo fmt` reflowing the longer `expect` lines across multiple lines); `Cargo.toml`/`Cargo.lock` 1 each; `docs/CHANGELOG.md` +7; `CLAUDE.md` header.

**Gate logs:** `/tmp/verify_unwrap.log` (post-edit, exit 0), `/tmp/verify_ship.log` (post-bump, exit 0), `/tmp/checks_195.log` (CI watch, exit 0).

**The seq-1 audit deferred list — final status (now fully closed):**

| Item | Topic | Status | Seq |
|---|---|---|---|
| 1 | generic `RonRegistry<V>` | DONE | 62 |
| 2 | scheduler O(V·E)→O((V+E)logV) | DONE | 61 |
| 3 | split god-files (docked.rs / gizmo.rs) | DONE | 63+64 |
| 4 | central editor theming constants | DONE | 65 |
| 5 | `AudioSurface` cross-platform trait | REJECTED (needs a facade feature) | 66 |
| 6 | texture upload pixel-format param | DONE | 67 |
| 7 | `ecs/world.rs` unwrap hardening | **DONE** | **68** |
| 8 | tier-5 cleanups | DONE | 66 |

## Code Analysis

- **`Archetype`** (`world.rs:65`) — `{ type_set: Vec<TypeId> (sorted), entities: Vec<Entity>, columns: HashMap<TypeId, Vec<ComponentBox>> }`. The load-bearing invariant: `columns` has exactly one entry per `type_set` tid, and every column `Vec` has the same length as `entities`. `contains(tid)` is `type_set.binary_search(&tid).is_ok()`.
- **Family (a) column-presence** sites are safe because the iterator chains `.filter(|arch| arch.contains(tid)).flat_map(|arch| { let col = arch.columns.get(&tid).expect(...); ... })` — the filter guarantees the column exists before the `get`. The own-`type_set` sites (`despawn`/`move_entity`) iterate `for &tid in &type_set { columns.get_mut(&tid).expect(...) }` — every tid is by definition a key of `columns`.
- **Family (b) downcast** sites are safe because the column was fetched by `TypeId::of::<T>()`; a `ComponentBox` (`Box<dyn Any + Send + Sync>`) in that column was inserted as `T`, so `downcast_ref/mut::<T>()` always `Some`.
- **`spawn`** (`world.rs:144`) — `generations[index]` index access is infallible: `next_index` path does `self.generations.push(0)` before reading; the free-list path only ever holds indices that already have a `generations` slot. No `.unwrap()` here.
- **`despawn`** (`world.rs:165`) — already defensive: early-returns on missing `entity_location`, `self.generations.get_mut(entity.index)` guarded by `if let Some`, `if *generation != u32::MAX` overflow guard before incrementing + pushing to the free-list. No `.unwrap()` in the alloc/generation logic (the only despawn unwrap was the family-(a) column `get_mut` in the type_set loop).
- **`query2_mut`/`query3_mut`** (`world.rs:387`/`421`) — already used `.expect(...)` (from a prior session) for the `columns.get_disjoint_mut([...])` results; this session added `.expect(...)` only to their inner `downcast_mut` calls.
- **`.unwrap_err()`** (`world.rs:298`, in `add_component`) — `sig.binary_search(&tid).unwrap_err()` returns the sorted insertion index for a known-absent tid (the `contains` branch returned earlier). Correct usage, kept.

**Canonical `expect` messages used (for consistency if a sibling file is ever hardened the same way):**

| Site type | Message |
|---|---|
| contains-filtered column, single-type | `"<fn>: archetype was filtered to contain this column"` |
| contains-filtered column, multi-type | `"<fn>: archetype was filtered to contain column A"` (…`B`/`C`/`D`) |
| own-`type_set` column (`despawn`) | `"despawn: column exists for every type in the archetype's type_set"` |
| own-`type_set` column (`move_entity`) | `"move_entity: source/target column exists for every type in its type_set"` |
| `add_component` checked-contains | `"add_component: archetype contains this column (checked above)"` |
| `add_component` post-move push | `"add_component: target archetype contains the newly added column"` |
| downcast, single-type | `"column holds type T"` |
| downcast, multi-type | `"column holds type A"` (…`B`/`C`/`D`) |

The boundary: `<fn>` is the literal method name (`query`/`query2`/`par_query2_map`/…), so an abort message names both *which* method and *which* invariant broke.

## Files Changed

### Source code
- `src/ecs/world.rs` — all 51 raw `.unwrap()` → `.expect("<invariant>")` across 17 fns (`despawn`, `add_component`, `move_entity`, `query`, `query_mut`, `query2_mut`, `query3_mut`, `query2`, `query3`, `query4`, `query_with`, `query_without`, `query_opt2`, `par_query_for_each`, `par_query_map`, `par_query2_for_each`, `par_query2_map`). No behavior change.

### Release paperwork
- `Cargo.toml` (0.50.1), `Cargo.lock` (refreshed), `docs/CHANGELOG.md` (0.50.1 `### Changed (internal)` entry), `CLAUDE.md` (header v1.6.119 + package v0.50.1).

### Memory (outside repo)
- `engine-current-state.md` → seq 68 (frontmatter description prepend + version/hash bump, lead paragraph, new seq-68 bullet, seq-60 stale-tail fixed); `MEMORY.md` index line refreshed.

### Tests
- None changed. The 43-fn ECS suite (`src/ecs/world/tests.rs`) + `commands.rs` tests stayed untouched and green (the acceptance test).

## User Feedback & Preferences (REQUIRED)

- **"전체패스진행" (proceed with the full pass)** — after the 5-step onboarding narration + the full-vs-light scope question, the user chose the full pass and let me drive design + execution + ship + merge end-to-end (no per-step confirmation).
- **Onboarding-then-wait protocol honored** — the resume prompt asked for an onboarding narration first (summarize handoff, state item pick + per-unwrap approach, state what I'd verify, read key + adjacent files, explain first action) and then **wait for go-ahead before executing**. Followed exactly.
- **Standing merge-authority delegation** — squash-on-green-CI, no per-PR re-confirm (memory `merge-authority-delegated`). Applied: shipped + merged #195 without a separate merge confirmation.
- **`/handoff 푸시`** — the user wants this handoff doc landed via its own `docs(handoff)` PR (matching seq-2 #187, seq-3 #192, seq-4 #194).
- Per global + project conventions: **user-facing reports in Korean, all artifacts/prompts/code in English** — followed throughout (this file is English; the chat was Korean).
- The user values **methodical, evidence-first work** — the census-before-edit + the explicit correction of the parent's hypothesis (rather than blindly adding guards/tests the parent suggested) is the kind of rigor expected.

## Where We're Going

- **The seq-1 engine-audit deferred list (items 1–8) is fully closed.** No more audit backlog. This handoff is the terminal one of the audit follow-up arc.
- **Read the dungeon-merchant wishlist board FIRST next session** (`../dungeon-merchant/docs/engine-wishlist.md`) — ACTIVE is empty (next ID EW-002). A new EW request is now the highest-priority driver since the audit backlog is empty.
- **Bonus (carried since seq 2):** make `RonRegistry<V>` + `RonLoadable` `pub` at the crate root so forks can register their own RON-loaded asset types. Small additive API.
- **Item 5 reframed (future feature, not a refactor):** a cross-platform **audio facade** (so a game writes one audio path instead of native-fn + wasm-no-op-stub) — `/add-feature-example` with a game that plays audio on both native and web. This is the rejected item 5, re-scoped as a feature.
- **Item 6 HDR half (future feature):** a format-matched sprite pipeline variant so a non-surface-format (`Rgba16Float`) render target can be rendered into — the half of item 6 deliberately left out of scope in seq 67. Its own session.
- **General engine direction:** back to the VISION feature+example loop (breadth-first 2D engine capabilities), driven by the wishlist board or a fresh user request.

## Risks & Blockers

- **None blocking.** main is clean + green at v0.50.1; the audit arc is closed.
- The change is the lowest-risk possible (behavior-preserving, codegen-identical happy path, tests untouched + green, CI green) — no residual risk from item 7.

## Open Questions

- **None for item 7** — it's done and merged.
- **Direction question for next session:** with the audit backlog empty, what drives the next work — a new dungeon-merchant EW request, the `RonRegistry` pub bonus, the audio facade (item-5 reframe), the HDR render-target (item-6 half), or a fresh feature? The user drives this at the next seam.

## Quick Start for Next Session

```bash
# Sync + verify clean/green
git checkout main && git pull --ff-only        # expect main @ b4e8c5d or later (this handoff's docs PR)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?   # must be 0

# The audit backlog is EMPTY — check the game wishlist board FIRST (it's now the top driver)
cat ../dungeon-merchant/docs/engine-wishlist.md    # ACTIVE should be empty (next ID EW-002)

# Live engine state + gotchas
#   memory engine-current-state (seq 68) + MEMORY.md index

# If picking up a carried bonus / reframe (no EW request):
#   RonRegistry pub      → src/ron_registry.rs (make RonRegistry<V> + RonLoadable pub at crate root, re-export in src/lib.rs)
#   audio facade (item 5)→ src/audio.rs + src/audio_wasm.rs (a cross-platform facade = a feature+example, NOT a trait — see seq-66 rejection)
#   HDR RT (item 6 half) → src/renderer/sprite.rs (format-matched pipeline variant) + src/renderer/render_target.rs

# Next action
#   No audit work remains. Read the wishlist board; if an EW request exists, do that.
#   Otherwise ASK the user which direction (RonRegistry pub / audio facade / HDR RT / fresh feature).
```

---

## Session Closed

**Closed at:** 2026-06-22
**Session status:** Handed off to next session.
**Code work:** item 7 landed via PR **#195** (v0.50.1, merge commit `b4e8c5d`) — already on main before this handoff.
**Landed:** this handoff doc lands on `main` via its own `docs(handoff)` PR (matching seq-2 #187, seq-3 #192, seq-4 #194). Memory `engine-current-state` is at seq 68; `MEMORY.md` index refreshed. **The seq-1 engine-audit deferred list (items 1–8) is fully closed — this is the terminal handoff of the audit follow-up arc.**
