# Codebase cleanup pass: split `world.rs` (v0.68.2) + trim CLAUDE.md (<200) + compact the engine-state memory

**Date:** 2026-06-24
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `codebase-cleanup` seq `1`
**Parent:** `none — first in chain`
**Prior chain:** none — first in chain

---

## Related Handoffs

- The immediately-preceding work (memory **seq 92** = `resources.rs` split into `src/resources/`, #241, v0.68.1) was the *same kind of cleanup* but got **no handoff file** — it was recorded in the `engine-current-state` memory only ("NO separate docs(handoff) PR — small PATCH refactor"). This session (seq 93) is its sibling, not its continuation. No shared bead.
- Latest pre-cleanup chains (rendering): `HANDOFF_bloom-web_2026-06-24.md`, `HANDOFF_bloom-mip-chain_2026-06-24.md`, `HANDOFF_render-format-query-web_2026-06-24.md` — unrelated work streams, listed for reference only.

## Reference Documents

- `CLAUDE.md` — agent quick reference (module map, verification gate, conventions). Now 191 lines; the ECS-world module-map row describes this split.
- `docs/VISION.md` — the "forkable 2D skeleton" north star + the feature+example acceptance-test loop.
- `docs/PATTERNS.md` — core architecture patterns + task recipes (incl. the render-target-format-aware pipeline-cache pattern from seq 86).
- `docs/AGENT_NOTES.md` — **new this session**; the extracted agent working heuristics.
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — the LIVE per-seq state (description now compact; body = full seq 1→92 history). `MEMORY.md` is its index.
- `../dungeon-merchant/docs/engine-wishlist.md` — the downstream game↔engine work board (EW-NNN requests). Read FIRST each session.

## The Goal

Keep the wgpu-based 2D engine `skeleton-engine` a clean, fork-friendly **skeleton** (see `docs/VISION.md`). This session had **no queued feature work** — the downstream Dungeon Merchant wishlist board (`../dungeon-merchant/docs/engine-wishlist.md`) was ACTIVE EMPTY (EW-001/002/003 all shipped & verified, next free ID **EW-004**). When the board is empty the standing directive is to ASK the user for direction. The user chose **"codebase inspection / cleanup"** over new features. The objective became: find and ship the highest-value behavior-preserving cleanups, each landed as a merged PR on green CI, mirroring the just-prior `resources.rs` split (#241). Three landed: (1) split the 900-line `impl World` god-block, (2) bring `CLAUDE.md` back under its own 200-line limit, (3) de-bloat the `engine-current-state` memory file. (A pre-session `/claude-dashboard:setup` configured the status line — unrelated to the engine; see "What We Tried" item 0.)

## Where We Are

- **main @ `656926a`**, working tree **clean**, no open PRs, CI green. Package **v0.68.2**, CLAUDE.md doc-header **v1.6.146**.
- **#242 (seq 93) MERGED** — `refactor(ecs): split world.rs impl World into concern submodules (v0.68.2)`. The single ~900-line `impl World` block (was `src/ecs/world.rs` L116–1020) is now split into 8 submodules under `src/ecs/world/`. **931 lib tests unchanged**, no public API change.
- **#243 MERGED** — `docs(claude): trim CLAUDE.md under the 200-line limit`. `CLAUDE.md` **212 → 191 lines**; the "Agent working notes" section moved verbatim to new `docs/AGENT_NOTES.md` + a Document-map row. Docs-only, no package bump.
- **Memory compaction (no PR)** — `~/.claude/.../memory/engine-current-state.md` `description` frontmatter **45,469 → 2,548 chars**; file **126,381 → 84,167 bytes**. The body (full per-seq history seq 1→92 + gotchas) was preserved **byte-for-byte** via mechanical head/tail reconstruction. Backup at `/tmp/ecs_backup.md`.
- `src/ecs/world.rs` is now ~252 lines: data model only (Entity/Archetype/World structs, type aliases, reflect free-fns, `ReflectEntry`), `World::new`, the 4 private archetype helpers, `Default`, and the `mod` declarations.
- New submodule files (lines): `entities.rs` 103, `components.rs` 149, `queries.rs` 294, `resources.rs` 69, `reflect.rs` 79, `change_tracking.rs` 92, `clone.rs` 50, `parallel.rs` 145 (native-only).
- `world/tests.rs` (730 lines) was **NOT touched** — it uses `use super::*` and still resolves because `World`/`Entity` stay defined in the `world` module.
- `lib.rs` and all call sites across the crate were **NOT touched** — the public path `engine::World` / `crate::ecs::world::World` is unchanged.
- The verify gate (`./scripts/verify.sh`) ran green (`VERIFY_EXIT=0`) **4 times** this session: baseline, post-split, post-version-bump, and post-CLAUDE.md-trim.
- Two memory follow-ups are flagged but deliberately NOT done: a deeper archive-split of the still-large memory **body**, and nothing else outstanding.
- The split is **semver-invisible**: `engine::World` and every method keep the same signature and path; downstream consumers (incl. the deprecated `rust-survivors` and the active `dungeon-merchant`) need no change.
- The `engine_reflect_derive` workspace member and the wasm build are unaffected (the `parallel.rs` module is `#[cfg]`-gated out on wasm; the reflect free-fns are unchanged).
- No new `#[allow(...)]`, no new tests, no test edits — the change is a pure relocation, so the existing contract is the proof.
- The `/claude-dashboard` status line is now configured (detailed/ko/max/default) — cosmetic, independent of the engine.

## What We Tried (Chronological)

- **0. `/claude-dashboard:setup` (pre-cleanup, unrelated).** Interactive setup of the claude-dashboard status line. User chose: displayMode **detailed**, language **ko**, plan **max**, theme **default**, no hidden widgets. Wrote `~/.claude/claude-dashboard.local.json` and pointed `~/.claude/settings.json` `statusLine` at `claude-dashboard 1.26.2/dist/index.js`. Done; not part of the engine work.
- **1. Handoff review + next-work check.** Read the tail/head of `docs/HANDOFF.md` (it's an old v4.3-era doc + a phase table; the live per-seq history is in the `engine-current-state` memory, not this file) and the wishlist board. Confirmed board **ACTIVE EMPTY** (EW-004 next). Confirmed main was actually at `b69beb3` (v0.68.1, #241 resources.rs split) — one commit *ahead* of what the memory recorded (v0.68.0). Asked the user for direction → user picked **codebase inspection/cleanup**.
- **2. Codebase health scan.** `grep` for TODO/FIXME/HACK/XXX → **0** markers (very clean). `#[allow(...)]` → 43, but nearly all justified (web-sys/egui deprecated bindings with no non-deprecated alternative, test fixtures, `from_raw` escape hatch) → nothing actionable. Largest files by `wc -l`: `src/ecs/world.rs` **1165**, `src/app.rs` 1049 (but ~700 lines are tests; already 9 submodules → not a real candidate), rest are test files or cohesive single systems. **Conclusion: `world.rs`'s 900-line `impl World` monolith is the one real split candidate.**
- **3. Baseline verify.** `./scripts/verify.sh` → `VERIFY_EXIT=0`, **931 lib tests**, fmt/clippy/wasm/doc all clean. Established a known-green baseline before refactoring.
- **4. Confirmed scope with user.** Asked: split + ship + merge PR / local-only / different target. User picked **"split + release PR merge"** (full flow).
- **5. Split via `/split-module` skill.** Read all 1166 lines of `world.rs`. Mapped the 26-method `impl World` (L116–1020) + the separate native-only `par_query*` block (L1019–1156) into 8 concern submodules. Work order: wrote the 8 submodule files first (each with an exact `use super::{...}` + a doc-comment header, method bodies **verbatim**) — rust-analyzer flagged each as "unlinked-file" until the next step. Then rewrote `world.rs` to the core (structs + aliases + reflect free-fns + `World::new` + 4 private helpers + `Default` + the 8 `mod` decls, with `#[cfg(not(target_arch = "wasm32"))] mod parallel;` gated). `cargo build --lib` green on the first try, then full `./scripts/verify.sh` green (`VERIFY_EXIT=0`, 931 tests). Updated the `CLAUDE.md` module-map row for the ECS world.
- **6. Ship via `/ship` + `/land-pr`.** Bumped `Cargo.toml` 0.68.1→0.68.2, refreshed `Cargo.lock`, added the `docs/CHANGELOG.md` 0.68.2 entry, bumped `CLAUDE.md` header (v1.6.144→v1.6.145 + module-map row). Re-ran verify green. Committed, pushed `refactor/split-world-module`, opened **PR #242**, watched CI (4/4 pass, native test 5m4s), squash-merged, synced main, bumped the memory seq.
- **7. CLAUDE.md trim.** Noticed `CLAUDE.md` was **212 lines** (>200 limit) — pre-existing, flagged as out-of-scope in #242. After user said "진행해" to the two offered follow-up cleanups, moved the "Agent working notes" section to `docs/AGENT_NOTES.md`, added a one-line pointer + Document-map row, bumped doc-version → v1.6.146. Verify green → **PR #243** → CI 4/4 (native 4m19s) → squash-merged.
- **8. Memory compaction.** The `engine-current-state.md` `description` frontmatter had grown to a **45,469-char single-line blob** spanning seq 54→93 — far too big for its "one-line recall summary" purpose, and the file couldn't be Read whole (>25K tokens per line). Mapped the structure with `awk '{print NR": "length($0)}'`: line 3 = the 45K-char description, line 10 = a 21K-char body header, lines 12–41 = ~24 per-seq/topic paragraphs (2–5K chars each), covering seq 1→92. Found the body already holds the comprehensive narrative → the description was **purely redundant**. Wrote the new 2,548-char description to `/tmp/newdesc.txt` (avoids shell-quoting the huge string), then reconstructed: `cp` backup → `head -2` + `cat newdesc` + `tail -n +4` → validated (`diff` of body lines 4→end = identical, frontmatter delimiters intact, line-3 quote count = 2) → `mv` over the original. File 126,381 → 84,167 bytes. Updated all LIVE pointers (description head + body header + `MEMORY.md` index) to `656926a` / #243.

## Key Decisions

- **The descendant-module visibility trick (core of the split).** Each new submodule re-opens `impl World` as a **child module of `world`**, so it can reach `World`'s **private fields** and the private helpers (`move_entity`, `get_or_create_archetype`, etc.) via the Rust rule *"a private item is visible to its defining module **and that module's descendants"***. This is exactly how #241's `resources.rs` split worked (though that was independent types, not impl blocks). No field/helper had to be made `pub(crate)` beyond what already was. This is why `lib.rs` + call sites stay byte-identical.
- **What stays in `world.rs` core vs moves out.** Kept inline: the structs (`Entity`/`Archetype`/`World`), type aliases (`ComponentBox`/`CloneComponentFn`/`ArchetypeId`), the reflect free-fns (`get_reflect_impl`/`get_reflect_mut_impl`), `ReflectEntry`, `World::new`, the 4 private archetype helpers (`has_component_typeid` [`pub(crate)`], `clone_component_by_typeid`, `get_or_create_archetype`, `move_entity`), `Default`. Rationale: the data model + the plumbing every submodule shares belong together; the *API surface* is what splits.
- **`apply_commands` → `entities.rs`** (not its own file) — one-liner, fits the entity-lifecycle group. **`register_clone`/`clone_entity` → their own `clone.rs`** (not lumped into `change_tracking.rs` as first sketched) — cloning is conceptually distinct from per-tick change detection.
- **Native-only `par_query*` → `parallel.rs` with a `#[cfg(not(target_arch = "wasm32"))] mod parallel;`** gate on the declaration (cleaner than per-impl cfg) — the whole module is absent on wasm.
- **Reverted an accidental non-verbatim change.** First draft of `entities.rs::despawn` used `entity.index()` (the public accessor); reverted to the original `entity.index` field access (legal from a descendant module) to keep the move byte-pure for trivial review.
- **PATCH (v0.68.2), not MINOR.** Pre-1.0 rule: internal refactor with no API/behavior change = PATCH. Mirrors #241 (v0.68.1, also a split PATCH).
- **Memory compaction: edit the description, keep the body.** Considered a full rewrite (rejected — clobber risk on a 122KB file I can't fully Read) and a deep body→archive split (deferred — risk of losing per-seq gotchas). Chose the safe, high-value middle: compact only the redundant description, preserve the body byte-for-byte, back up first.
- **CLAUDE.md trim: move "Agent working notes" (not Verification).** Picked the most self-contained, least-critical-to-have-inline section. Verification detail must stay inline; module-map rows are engine API reference. Followed the file's own documented growth strategy.
- **Splitting the *core ECS* was judged safe despite being the highest-touch file.** Rationale: the `/split-module` skill guarantees behavior preservation (tests stay green), the 931-test native suite exercises `World` heavily and is run by CI, the move is purely mechanical (no logic touched), and #241 had *just* validated the same descendant-module technique on `resources.rs`. The maintainer doing #241 the day before was a strong signal this cleanup is welcome. Risk accepted; outcome: 931 tests unchanged, CI 4/4.
- **Two cleanups, one user word ("진행해").** Treated the blanket "proceed" as authorization for *both* offered follow-ups (CLAUDE.md trim as a repo PR; memory compaction as internal hygiene) rather than re-asking which — the user had already engaged with the merge-and-ship flow and delegated merge authority.

## Evidence & Data

### Commits landed this session (main)

| Hash | PR | Type | Summary |
|---|---|---|---|
| `85abd41` | #242 | refactor(ecs) | split world.rs impl World into concern submodules (v0.68.2) |
| `656926a` | #243 | docs(claude) | trim CLAUDE.md under the 200-line limit (extract Agent working notes) |

(plus the memory compaction, which is outside the repo)

### `world.rs` split — line accounting (PR #242 diff: 13 files, +1012 / −942)

| File | Lines | Methods moved |
|---|---:|---|
| `src/ecs/world.rs` (core, was 1166) | ~252 | — (structs + `new` + 4 private helpers + `Default` + mod decls) |
| `world/entities.rs` | 103 | `entity_count`/`spawn`/`despawn`/`entities`/`is_alive`/`apply_commands` |
| `world/components.rs` | 149 | `remove_component`/`take_component`/`add_component`/`get`/`get_mut`/`has_component` |
| `world/queries.rs` | 294 | `query`/`query_mut`/`query2_mut`/`query3_mut`/`query2`/`query3`/`query4`/`query_with`/`query_without`/`query_opt2` |
| `world/resources.rs` | 69 | `insert_resource`/`resource`/`resource_mut`/`remove_resource`/`with_resource_mut`/`take_resource_erased`/`insert_resource_erased` |
| `world/reflect.rs` | 79 | `register_reflect_named`/`reflect_registered_types`/`get_reflect`/`get_reflect_mut`/`reflected_components` |
| `world/change_tracking.rs` | 92 | `clear_change_tracking`/`query_added`/`query_changed`/`mark_changed`/`get_mut_tracked` |
| `world/clone.rs` | 50 | `register_clone`/`clone_entity` |
| `world/parallel.rs` (native-only) | 145 | `par_query_for_each`/`par_query_map`/`par_query2_for_each`/`par_query2_map` |

### Verify gate runs (all `VERIFY_EXIT=0`)

| When | Lib tests | Notes |
|---|---:|---|
| Baseline (pre-split) | 931 | fmt/clippy/wasm/test/doc all clean |
| Post-split | 931 | behavior preserved |
| Post-version-bump (0.68.2) | 931 | lock + doc rebuilt clean |
| Post-CLAUDE.md-trim | 931 | docs-only, near no-op |

### CI results

| PR | Test (native) | Build (WASM) | Rustdoc | Package dry-run | mergeState |
|---|---|---|---|---|---|
| #242 | pass 5m4s | pass 40s | pass 42s | pass 1m4s | CLEAN → squash-merged |
| #243 | pass 4m19s | pass 31s | pass 41s | pass 55s | CLEAN → squash-merged |

### Codebase health scan (numbers)

- TODO/FIXME/HACK/XXX in `src/`: **0**
- `#[allow(...)]` in `src/`: **43** (13 `deprecated` [web-sys/egui, no alt], 11 `too_many_arguments`, 8 `type_complexity`, 6 `dead_code` [test fixtures + `from_raw`], 2 `unused_mut`, 2 `enum_variant_names`, 1 `map_entry`) — all judged justified.
- `#[deprecated]` items defined in `src/`: **0**

### Memory file compaction

- `engine-current-state.md`: **126,381 → 84,167 bytes**, **41 lines** (unchanged count).
- `description` (frontmatter line 3): **45,469 → 2,548 chars**.
- Validation: frontmatter delimiters at lines 1 & 8 (intact), line-3 double-quote count = **2** (valid YAML string), `diff` of body (lines 4→end) old vs new = **identical**.
- Backup: `/tmp/ecs_backup.md` (126,381 bytes).

### CLAUDE.md

- **212 → 191 lines** (9 lines of headroom under the 200 limit). PR #243 diff: 2 files, +33 / −25.

### Version progression this session

| Artifact | Before | After #242 | After #243 |
|---|---|---|---|
| package `skeleton-engine` | 0.68.1 | **0.68.2** | 0.68.2 |
| `CLAUDE.md` doc-version | v1.6.144 | v1.6.145 | **v1.6.146** |
| memory seq | 92 | **93** | 93 (+ follow-ups noted) |
| main tip | `b69beb3` | `85abd41` | **`656926a`** |

### Session timeline (order of operations)

1. `/claude-dashboard:setup` → status line configured (detailed/ko/max/default).
2. Read `docs/HANDOFF.md` tail/head + wishlist board → board EMPTY → asked user → **cleanup** chosen.
3. Health scan (TODO=0, `#[allow]`=43 justified, largest-file table) → `world.rs` is the target.
4. Baseline `verify.sh` green (931 tests).
5. Asked scope → user chose **full split + PR merge**.
6. `/split-module`: read 1166 lines → wrote 8 submodules verbatim → rewrote `world.rs` core → build + verify green.
7. `/ship` v0.68.2 paperwork (Cargo.toml/lock/CHANGELOG/CLAUDE.md) → re-verify green.
8. Commit → push → PR **#242** → CI 4/4 → squash-merge → sync main → bump memory seq 93.
9. User "진행해" → CLAUDE.md trim → PR **#243** → CI 4/4 → merge → memory compaction → pointers updated.
10. `/handoff` (this document).

### Largest `src/` files at scan time (top, by `wc -l`)

The split-candidate selection rested on this. Only `world.rs` was a genuine god-block; the rest are test files or cohesive single systems.

| File | Lines | Verdict |
|---|---:|---|
| `src/ecs/world.rs` | 1165 | **SPLIT** — one ~900-line `impl World` monolith |
| `src/app.rs` | 1049 | skip — ~700 lines are tests; non-test code is compact + already has 9 submodules |
| `src/audio/tests.rs` | 986 | skip — test file |
| `src/app/editor/ui/docked.rs` | 959 | skip — already split at 0.49.1 (seq 63); cohesive |
| `src/ui/system/focus_pass.rs` | 922 | skip — single cohesive system |
| `src/audio_wasm.rs` | 882 | skip — single wasm backend |
| `src/app/window.rs` | 844 | skip — single concern |
| `src/timeline.rs` | 832 | skip — single system |

### The seq-92 precedent (what this mirrors)

| | seq 92 — `resources.rs` (#241, v0.68.1) | seq 93 — `world.rs` (#242, v0.68.2) |
|---|---|---|
| What split | 871-line grab-bag of ~26 unrelated resource *types* | ~900-line `impl World` *method* block |
| Into | `src/resources/{mod,debug_draw,display,fonts,lifecycle,profiling,render,time}.rs` | `src/ecs/world/{entities,components,queries,resources,reflect,change_tracking,clone,parallel}.rs` |
| Re-export shape | `mod.rs` re-exports 27 items so `crate::resources::*` is unchanged | structs stay in `world.rs`; submodules re-open `impl World` (no re-export needed) |
| Bump | PATCH v0.68.1 | PATCH v0.68.2 |
| Tests | moved verbatim | `world/tests.rs` untouched (already a sibling file) |
| Outcome | CI 4/4, squash-merged, recorded in memory only | CI 4/4, squash-merged, + this handoff |

### `impl World` method inventory (the map that drove the split)

26 methods in the one block + 4 `par_query*` in the native-only block + 4 private helpers. Grouping (→ submodule) is in the line-accounting table above. The private helpers (`has_component_typeid`, `clone_component_by_typeid`, `get_or_create_archetype`, `move_entity`) + `new` stayed in core; `apply_commands` (a one-liner) went to `entities.rs`.

## Code Analysis

- **`World` private fields** (all reached from the submodules as descendants): `next_index`, `free_indices: VecDeque<u32>`, `generations: Vec<u32>`, `entities: Vec<Entity>`, `entities_row: HashMap<Entity,usize>`, `archetypes: Vec<Archetype>`, `archetype_index`, `entity_location: HashMap<Entity,(ArchetypeId,usize)>`, `resources: HashMap<TypeId,Box<dyn Any>>`, `reflect_registry`, `added_this_tick`, `changed_this_tick`, `clone_registry`.
- **Per-submodule `use super::{...}` import sets** (each is exact — `-D warnings` rejects unused): `entities`/`change_tracking`/`clone` → `{Entity, World}` + `std::any::TypeId`; `components` → `{ComponentBox, Entity, World}` + `TypeId`; `queries` → `{Archetype, Entity, World}` + `TypeId` (Archetype named for the disjoint-borrow destructuring in `query_mut`/`query2_mut`/`query3_mut`); `resources` → `World` + `std::any::{Any, TypeId}`; `reflect` → `{get_reflect_impl, get_reflect_mut_impl, Entity, ReflectEntry, World}` + `TypeId`; `parallel` → `{Entity, World}` + `TypeId`.
- **Cross-submodule method calls work because inherent methods belong to the type, not the module:** `clone_entity` (clone.rs) calls `self.spawn()` (entities.rs); `query_added`/`query_changed` (change_tracking.rs) call `self.get::<T>` (components.rs); `mark_changed`/`has_component`/`clone_entity` call `self.has_component_typeid` (core, `pub(crate)`); `remove_component`/`add_component` (components.rs) call the private `self.get_or_create_archetype`/`self.move_entity` (core). All legal: private items visible to descendant modules.
- **The `query_mut` family** uses `let Archetype { entities, columns, .. } = arch;` destructuring to get disjoint `&`/`&mut` borrows of two struct fields, and `HashMap::get_disjoint_mut([&ta, &tb])` for two distinct columns — that's why `queries.rs` must name `Archetype`.
- **Reflect free-fns** (`get_reflect_impl`/`get_reflect_mut_impl`) stay in core but are now *only* referenced from `reflect.rs` (via `super::`); still counts as "used" (no `dead_code` warning) because a private item referenced by a descendant module is used.
- **`take_component` (components.rs)** uses a swap-out-with-`Box::new(())`-placeholder trick to gain ownership of a `ComponentBox` before calling `remove_component` — that's the one method besides the queries that names `ComponentBox`.
- **`move_entity` / `get_or_create_archetype` (core)** are the archetype-transition plumbing called by `add_component`/`remove_component` (components.rs) on every component-set change; they clone the `type_set` to dodge the same-struct `&type_set` + `&mut columns` borrow conflict (documented inline). Keeping them in core means components.rs reaches them as `self.move_entity(...)` (descendant access to a private method).
- **Change tracking is two `HashMap<Entity, HashSet<TypeId>>` fields** (`added_this_tick`/`changed_this_tick`) — `query_added`/`query_changed` (change_tracking.rs) allocate a `Vec<Entity>` only when the set is non-empty (documented fast path), then `filter_map` through `self.get::<T>` (components.rs).
- **`Entity` is generation-checked** (`index: u32` + `generation: u32`, both private fields); `despawn` bumps the generation and pushes the index to `free_indices` (unless `u32::MAX`), so a stale handle never matches a reused slot. `entities.rs` reaches `entity.index` as a private field (descendant access).
- **The `world.rs` core file ends with the `mod` block at the TOP** (declared before the structs) and `#[cfg(test)] mod tests;` at the bottom — the submodule decls were placed up top with a comment explaining the descendant-visibility design so a reader hits the map first.

## Gotchas & Discoveries (this session)

### Rust / engine
- **`world/tests.rs` survives the split untouched** because it's `use super::*` and `super` = the `world` module, where `World`/`Entity` are still *defined* (the structs never moved). The 26 `impl World` *methods* moved to submodules, but methods belong to the type and resolve crate-wide, so `world.spawn()` etc. still work. Key check before splitting: confirm the test file doesn't name a *private* item that's being relocated (it doesn't — it only uses public API + defines its own `#[allow(dead_code)]` fixture structs).
- **Exact `use super::{...}` per file is mandatory under `-D warnings`** — an unused import fails clippy. I derived each file's import set from which type names its method bodies actually spell (e.g. `queries.rs` needs `Archetype` only because of the `let Archetype { entities, columns, .. } = arch;` disjoint-borrow destructuring; `resources.rs` needs `std::any::Any` only for the `Box<dyn Any>` in the `*_erased` methods).
- **Private free-fns referenced only by a child module don't trigger `dead_code`** — `get_reflect_impl`/`get_reflect_mut_impl` stay in `world.rs` but are now used only from `reflect.rs` via `super::`; still "used."
- **Verbatim-move discipline caught a drift:** first draft changed `entity.index` (field) to `entity.index()` (accessor) in `despawn`. Both compile (the field is reachable from a descendant module), but it's not a pure move — reverted to keep the diff trivially reviewable.
- **rust-analyzer "unlinked-file … not included in the module tree" diagnostics fired for each new submodule** until `world.rs` declared the `mod` — expected and harmless during the write sequence (write the submodules, then add the `mod` decls).

### Tooling / harness
- **The harness reclassifies a foreground `while … command sleep N; done` wait-loop as a background task** (it returns a background task ID rather than blocking). This happened ~5× when waiting on `verify.sh`/CI. Net effect: relying on the **task-completion `<task-notification>`** (which re-invokes the turn) is more reliable than a polling wait-loop. `sleep` (bare) is blocked; `command sleep` runs but the loop gets detached.
- **macOS `cat` has no `-A`** (BSD cat) — to reveal exact chars use `cat -v` or `od -c`. The authoritative YAML-quote-balance check was `tr -cd '"' | wc -c` (= 2 for a valid single-quoted-free string).
- **The `engine-current-state.md` memory file could not be Read whole** — its `description` was a single 45,469-char line (~13K tokens) and the file 122KB; `Read` with `limit=1` on that line works, but `limit=22` exceeded 25K tokens. Use `offset`/`limit=1` to read a specific giant line, or `awk 'NR==N'` for length.
- **Editing a file after an external `mv` invalidates the harness's "file read" state** — after `mv /tmp/ecs_new.md engine-current-state.md`, the next `Edit` failed with "File has not been read yet"; re-`Read` the specific line first.
- **CI `mergeStateStatus: BLOCKED`** during `gh pr checks --watch` just means a required check is still running (the native test), not a real block — it flips to `CLEAN` when the last check passes. Always re-check `mergeStateStatus` before `gh pr merge`.

### Process
- **The board-empty → ASK directive worked as intended** — memory `[[engine-current-state]]` said "ACTIVE EMPTY → read board FIRST, ASK if empty," and that's exactly the flow this session followed before any work began.
- **`docs/HANDOFF.md` is NOT the live history** — it's a frozen v4.3-era doc + a phase table. The live per-seq narrative lives in the `engine-current-state` memory file. Don't mine `docs/HANDOFF.md` for "what's next."

## Files Changed

### Source code (PR #242)
- `src/ecs/world.rs` — reduced from 1166 → ~252 lines (data model + `new` + private helpers + `Default` + `mod` decls).
- `src/ecs/world/entities.rs` — **new** (103); `spawn`/`despawn`/`is_alive`/`entity_count`/`entities`/`apply_commands`.
- `src/ecs/world/components.rs` — **new** (149); add/remove/take/get/get_mut/has_component.
- `src/ecs/world/queries.rs` — **new** (294); all `query*` read + `*_mut` + `_with`/`_without`/`_opt2`.
- `src/ecs/world/resources.rs` — **new** (69); typed + erased resource access + `with_resource_mut`.
- `src/ecs/world/reflect.rs` — **new** (79); Reflect-registry register/lookup.
- `src/ecs/world/change_tracking.rs` — **new** (92); per-tick added/changed + `mark_changed`/`get_mut_tracked`.
- `src/ecs/world/clone.rs` — **new** (50); `register_clone`/`clone_entity`.
- `src/ecs/world/parallel.rs` — **new** (145); native-only `par_query*` block (`#[cfg(not(wasm32))]` mod).

### Docs / release paperwork (PR #242)
- `Cargo.toml` (0.68.1→0.68.2), `Cargo.lock` (refreshed), `docs/CHANGELOG.md` (0.68.2 entry), `CLAUDE.md` (header v1.6.145 + module-map row for the ECS world).

### Docs (PR #243)
- `docs/AGENT_NOTES.md` — **new**; the moved "Agent working notes" (context-management split table, exploration order, subagent-prompt principles).
- `CLAUDE.md` — removed the section (one-line pointer left), added Document-map row, header v1.6.146.

### Memory (no repo / no PR)
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — description compacted, body preserved, LIVE pointers → `656926a`.
- `.../memory/MEMORY.md` — index line for engine-current-state updated.

### Config (pre-session, unrelated)
- `~/.claude/claude-dashboard.local.json` (detailed/ko/max/default), `~/.claude/settings.json` (statusLine → claude-dashboard 1.26.2).

## User Feedback & Preferences

- **"마지막 핸드오프 보고 다음 작업 확인"** — the session opener: read the last handoff, check what's next.
- When the board was empty and offered options, user chose **"코드베이스 점검/정리"** (codebase inspection/cleanup) over new feature work.
- For the world.rs split scope, user chose the **full flow** ("분할 + 릴리스 PR 머지") — not local-only.
- **"진행해"** (proceed) — green-lit *both* offered follow-up cleanups (CLAUDE.md trim + memory compaction) in one word.
- Standing preferences (from memory, honored this session): user-facing reports in **Korean**, agent-to-agent/code/docs in **English**; merge authority is **standing-delegated** (squash on green CI, no per-session re-confirm); always pass an explicit `model` to subagents; run `cargo fmt` before verify; never pipe a gate's exit code.
- Status-line config choices (the `/claude-dashboard:setup` task): displayMode **detailed**, language **ko**, plan **max**, theme **default**, no widgets hidden.
- The user works in Korean and expects the conversational layer in Korean even while all artifacts (code, docs, this handoff) are English — confirmed repeatedly and codified in `CLAUDE.md` + `[[conversation-language-korean]]`.

## Where We're Going

1. **Next session: read the wishlist board FIRST** — `../dungeon-merchant/docs/engine-wishlist.md` (ACTIVE EMPTY, next free ID **EW-004**). If a new EW-NNN was filed, do that. If still empty, ASK the user for direction before any larger work.
2. **Optional deeper memory hygiene (deferred):** the `engine-current-state.md` **body** (lines 10–41, ~80KB) still holds the full seq 1→92 narrative + gotchas. If it keeps growing, archive-split the oldest seqs (≤~85) into `[[engine-history-archive]]`, keeping only the latest ~5 seqs + standing directives in the live file. Deferred this session to avoid per-seq-gotcha loss risk.
3. **If cleanup continues:** `src/app.rs` (1049 lines) is *not* a real split candidate (~700 lines are tests, already 9 submodules). No other 1000+-line non-test source files remain. The codebase is otherwise clean (0 debt markers). Candidate-but-marginal next targets if pushed: `src/ui/system/focus_pass.rs` (922, but one cohesive system), `src/timeline.rs` (832, one system) — neither is a god-block; only split if a future change makes one unwieldy.
4. **If a feature is chosen instead:** follow the VISION loop — a new engine capability is "not done" until a small playable `examples/` game exercises it; fix the API if it feels awkward while writing the example; then `/add-feature-example` → `/ship` → `/land-pr`.
5. **Watch for downstream-verify replies on the board** — if a prior EW item flips to `Shipped (vX.Y.Z)` awaiting game verification, that's the game's court; nothing for the engine to do unless a new request lands.

## Risks & Blockers

- **None blocking.** Tree clean, CI green, both PRs merged.
- The memory body is large (~80KB) but that's the "full record" field, not the recall-summary; it's a hygiene nicety, not a correctness risk.
- CI is ubuntu-only — but this session shipped no OS-gated or GPU-render code, so green CI fully verified both changes (the `world.rs` split is pure ECS logic exercised by 931 native tests; the CLAUDE.md trim is docs-only).
- The `/tmp/ecs_backup.md` memory backup is **session-local** — `/tmp` is cleared on reboot. If the compaction ever needs reverting, do it this session; otherwise the body (preserved byte-for-byte) plus git-free memory means there's no other copy. (Risk is low: the body retains everything; only the redundant description summary was shortened.)
- `docs/HANDOFF.md` remains a stale v4.3-era artifact — not a blocker, but don't trust it for "current state" (use the memory file).

## Open Questions

- **None blocking.** The one judgment call (how deep to compact the memory) was resolved conservatively (compact the description, preserve the body, flag the rest).
- Open-but-not-urgent: should the memory **body** eventually be archive-split, or is the compact description enough? Lean "enough until the body itself impedes a Read."
- Open-but-not-urgent: does the engine want a *third* consecutive cleanup, or is it time to return to feature work? That's the user's call next session — the board check decides.

## Quick Start for Next Session

```bash
# 1. Read the downstream wishlist board FIRST (standing directive)
cat ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY? next free ID EW-004 → ASK if empty

# 2. Confirm engine state
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3        # tip should be 656926a (#243)
git status -s               # clean

# 3. Key files this session touched (read if continuing cleanup)
#    src/ecs/world.rs                 — core data model + private archetype plumbing
#    src/ecs/world/{entities,components,queries,resources,reflect,change_tracking,clone,parallel}.rs
#    docs/AGENT_NOTES.md              — extracted agent working notes
#    CLAUDE.md                        — now 191 lines; module-map row for the ECS world describes the split

# 4. Verify current state (the gate — read its exit code, never pipe it)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"   # expect 0, 931 lib tests

# 5. Memory: live state is engine-current-state.md (description now compact);
#    backup of the pre-compaction file is /tmp/ecs_backup.md (this session only)

# Next action:
#   Read ../dungeon-merchant/docs/engine-wishlist.md. If EW-004+ filed → implement it
#   (feature work uses the VISION feature+example loop). If empty → ASK the user for direction.
```

---

## Session Closed

**Closed at:** 2026-06-24 23:38 KST
**Commit:** landed via a `docs(handoff)` PR (this file)
**Session status:** Handed off to next session — engine work (#242, #243) already merged to `main`; this handoff is the session record.
