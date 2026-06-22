# HANDOFF — Audit deferred follow-ups: scheduler + registry refactors

**Chain:** standalone-4365aa4a
**Seq:** 2 (continuation)
**Parent:** plans/handoffs/HANDOFF_engine-audit-fixes_2026-06-22.md (seq 1)
**Date:** 2026-06-22
**Branch:** main
**Status:** COMPLETE + merged. main @ `2055069`, package **v0.49.0**, clean working tree, full gate green.
**Auto:** false

---

## Goal

The user said "다음진행" (proceed) after the seq-1 audit landed, opting to work the **deferred follow-up list** from that handoff in priority order. This session took the two highest-value / lowest-risk items:

1. **Generic `RonRegistry<V>`** — dedup the duplicated config-registry/hot-reload boilerplate (handoff item #1).
2. **`ecs/schedule.rs` O(V·E) topo-sort** → adjacency list + min-heap (handoff item #2).

Both are behavior-preserving refactors, each landed as its own PR through the full `/land-pr` loop (the user also asked to PR-land, and merge authority is standing-delegated).

---

## Since Last Handoff (what changed vs the seq-1 plan)

The seq-1 handoff's "Where We're Going" listed 8 deferred items. This session **completed items 1 and 2**; the rest remain open (see below). Net version path this session: v0.48.0 (seq-1 audit, already merged) → **v0.48.1** (scheduler) → **v0.49.0** (registry).

PRs merged this session: **#185** (scheduler, v0.48.1 PATCH) and **#186** (registry, v0.49.0 MINOR). (Seq-1's audit was #184.)

---

## Where We Are

- main @ `2055069`, v0.49.0, **clean tree**, `./scripts/verify.sh` → exit 0 (fmt + clippy `-D warnings` + wasm32 lib/bins build + `test --all-targets` + doc `-D warnings`).
- Memory updated: `engine-current-state` now seq 62 (lead + recent-seqs bullets for 61/62 + seq-60 deferred list marked ✓DONE for the two items); `MEMORY.md` index line refreshed; seq-55 folded to the archive line to stay compact.

---

## What We Did

### Refactor A — scheduler topo-sort (#185, v0.48.1 PATCH)

`src/ecs/schedule.rs::compute_order`. The Kahn ready-set was a `Vec` scanned with `iter().min()` + `retain` (O(V) per pop), and — the bigger cost — the relaxation **re-scanned the entire `edges` set on every pop** (O(V·E)).

Rewrite (lines ~57–125):
- `use std::cmp::Reverse; use std::collections::{BinaryHeap, HashMap};`
- Build `adj: Vec<Vec<usize>>` + `indeg` once from `edges`.
- Ready-set = `BinaryHeap<Reverse<usize>>` (min-heap → O(log V) pop, **same deterministic lowest-index tie-break** as the old `min()`).
- Relax each node's out-edges exactly once via `adj[next]`.
- Cycle `remaining` set: `(0..n).filter(|&i| indeg[i] > 0)` — equivalent to the old `!order.contains(i)` because a node is popped iff its in-degree reached 0.

Result: **O((V + E)·log V)**, identical execution order. Scheduler tests (`no_constraints_keeps_insertion_order`, `before_orders_correctly`, `shared_label_barrier`, `cycle_detected`, app-level schedule tests) **unchanged and green** — determinism + cycle detection both covered. No public API change.

### Refactor B — generic `RonRegistry<V>` (#186, v0.49.0 MINOR)

New crate-internal module **`src/ron_registry.rs`** (`mod ron_registry;` in lib.rs, NOT re-exported):
- `trait RonLoadable { type Err: Display; fn load_ron(path: &str) -> Result<Self, Self::Err>; }` — **native-gated** (`#[cfg(not(target_arch="wasm32"))]`).
- `struct RonRegistry<V> { items: HashMap<String,V>, #[cfg(not(wasm32))] paths: HashMap<String,String> }`.
- Hand-written `impl Default` (so `V` need not be `Default`).
- Cross-platform methods: `insert`, `get`, `names`.
- Native-gated `impl<V: RonLoadable>`: `load(name, path)` (stores `crate::asset::asset_key(path)` canonical key) and `reload_path(path, what)` (matches by re-canonicalizing the incoming path: `p == asset_key(path)`, then `V::load_ron`).
- Own unit tests: `insert_get_names_sorted`, `load_then_reload_by_canonical_path`.

The three homogeneous registries became thin wrappers delegating to an `inner: RonRegistry<V>`, keeping their exact public types/signatures:
- `src/particle/config_set.rs` — `ParticleConfigRegistry` + `impl RonLoadable for ParticleConfigSet` (`read_to_string` + `from_ron_str`). **Added `ParticleConfigRegistry::insert`** (the one additive API — the other two already had it; needed because a test poked the old private `.sets` field, now rewritten to use the public `insert`).
- `src/dialogue/tree.rs` — `DialogueRegistry` + `impl RonLoadable for DialogueTree` (`DialogueTree::load`). `box_of` now reads `self.inner.get`.
- `src/animation/clip_set.rs` — `AnimationClipRegistry` + `impl RonLoadable for AnimationClipSet` (`AnimationClipSet::load`).

Each keeps its inherent `reload_path` (delegating `self.inner.reload_path(path, "<tag>")`) + its `impl HotReloadable` (UFCS-delegating to the inherent method).

~130 lines of triplicated reload/path logic removed; all existing registry tests unchanged + green (incl. `reload_path_matches_canonical_key_from_poll_reloads`).

---

## Key Decisions & Rejected Alternatives

- **`DataTableRegistry` deliberately NOT folded into `RonRegistry`.** It is structurally different: no separate `paths` map (the path lives on each `DataTable.path`), a per-table `dirty` flag, and `reload_path` returns a 4-variant `ReloadOutcome` (Reloaded / SkippedDirty / NotFound / Err) to protect the editor's unsaved edits. Unifying it would either pollute the generic or risk that dirty-skip behavior. Left bespoke; noted in the changelog.
- **Canonical-key matching strategy unified to particle's** (store `asset_key` at load, match by re-canonicalizing incoming). The three registries had cosmetically divergent strategies (particle: store canonical + `==`; dialogue/animation: store raw + canonicalize-both per call). All three converge to the same observable behavior under `AssetServer::poll_reloads` (which passes the canonical path); `asset_key` is idempotent so the particle regression test still passes.
- **`RonRegistry` kept crate-internal** (not `pub`/re-exported). Making it public would be a fork-friendly bonus (custom RON-asset registries) but would add public surface + wasm export-gating; out of scope for a behavior-preserving refactor. Flagged as a possible future enhancement.
- **`get_mut` dropped from `RonRegistry`** — no wrapper uses it; kept the generic minimal rather than carrying an `#[allow(dead_code)]`.
- **Two separate PRs, sequential** (not one bundle): different subsystems, and each bumps the version (must be sequential to avoid lock/version conflicts). Honors the "one PR = one coherent change" guardrail.
- Version: scheduler = PATCH (no API change); registry = MINOR (the additive `ParticleConfigRegistry::insert`).

---

## Gotchas Hit (reusable)

- **`cat >>`-appended tests are not fmt/clippy-clean.** Two failures this session: (1) `cargo fmt` reformatted long `assert!`/`assert_eq!` lines (also seen in seq-1); (2) clippy `-D warnings` flagged `asset_key(&p).to_string()` as `unnecessary_to_owned` — `Arc<str>` → use `.as_ref()` to get `&str`. Always run `cargo fmt` + the full gate after appending test code, not just `cargo test`.
- **Renaming a private struct field breaks tests that poke it.** `ParticleConfigRegistry`'s test inserted into `reg.sets`/`reg.paths` directly; the wrapper rewrite (field `inner`) broke it. Fixed by adding the public `insert` and using it. Grep tests for direct field access before a struct refactor.
- **wasm gating for the generic:** `load`/`reload_path`/`RonLoadable` are native-only; the wrapper structs must still compile on wasm with only `items` (no `paths`). The `wasm32` build in the gate is what verifies this — don't skip it.
- The memory `engine-current-state.md` description field + lead are large; edit with **targeted small-span** Edits (prepend new lead, retag prior) rather than rewriting the whole line, to stay clear of the 25k Edit cap.

---

## Evidence

- `./scripts/verify.sh` → exit 0 on both branches before push, and the CI (Build WASM / Package dry-run / Rustdoc / Test native) passed green on #185 and #186; both merged `mergeStateStatus: CLEAN`.
- Diffstats: #185 = 5 files, +30/−17 (schedule.rs +34/−... net); #186 = 9 files, +216/−134 (incl. new `src/ron_registry.rs`, ~130 lines triplication removed across the 3 registries).

---

## Where We're Going — STILL-OPEN deferred items (priority order)

From the seq-1 audit (parent handoff `HANDOFF_engine-audit-fixes_2026-06-22.md`). Items 1–2 done this session; remaining:

3. **Split god-files** `src/app/editor/ui/docked.rs` (1233 lines) and `gizmo.rs` (1183). Extract `particle_tuner_grid` / `point_light_grid` into dedicated files — the pattern already exists (`audio_panel.rs`, `data_table_panel.rs`). `/split-module`.
4. **Central editor theming constants module** — pervasive inline magic numbers (panel sizes, gizmo handle sizes, Z-offsets 999/1000, colors, font sizes, drag ranges) across `docked.rs`/`gizmo.rs`/`mod.rs`/`slider.rs`/`checkbox.rs`.
5. **`AudioSurface` trait** shared by `audio.rs` (native) and `audio_wasm.rs` to cut cfg-guard duplication at cross-platform call sites.
6. **`renderer/texture.rs:130` `Rgba8UnormSrgb` hardcoded** → parameterize format for HDR/linear workflows (breaking to `from_rgba`; add `from_rgba_with_format` + keep `from_rgba` as the srgb wrapper). Best as a feature+example task.
7. **`ecs/world.rs` 56 real `unwrap`s** — dedicated invariant-hardening review (high blast radius; left untouched).
8. **Tier-5 remainder:** `SpatialGrid::candidates_in_aabb` per-query `HashSet` alloc → scratch buffer; `pathfinding` reconstruct-path dedup → helper; named-constant extractions; editor asset-browser `"[ ]"` stub.

**Possible bonus:** make `RonRegistry<V>` + `RonLoadable` `pub` (crate root) so forks can register their own RON-loaded asset types.

---

## Open Questions

- How far down items 3–8 does the user want to go? Items 3–4 (editor split + theming) are the next natural pair (both `/split-module`-shaped, low risk). Item 6 (texture format) is the one breaking change and should go through the feature+example loop.

---

## Resume

1. Sanity: `git checkout main && git pull --ff-only`; `./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?` should be 0.
2. Pick the next deferred item; branch `<type>/<slug>` off main (never commit to main directly).
3. For each: implement → `cargo fmt` → full gate (capture `$?`, never pipe) → `/ship` (PATCH for internal refactor, MINOR for additive API) → commit → push → `gh pr create` → `gh pr checks <n> --watch --fail-fast > log 2>&1` → squash-merge on CLEAN → `git pull --ff-only` → bump `engine-current-state` memory seq.
4. The dungeon-merchant wishlist board is EMPTY (next ID EW-002) — when picking work with no user direction, check it first, then ASK before backlog.

## Pointers

- Parent handoff (full audit detail + all 8 deferred items): `plans/handoffs/HANDOFF_engine-audit-fixes_2026-06-22.md`.
- Module map / verify rules: `CLAUDE.md`. Patterns: `docs/PATTERNS.md`.
- New module this session: `src/ron_registry.rs` (crate-internal generic registry).

---

## Appendix A — Why the three registries unified but DataTable didn't

| Registry | value store | path store | reload match (pre-refactor) | extra state | folded? |
|---|---|---|---|---|---|
| ParticleConfigRegistry | `sets: Map<name,Set>` | `paths: Map<name,canonical>` | `stored == incoming` (canonical) | — | ✅ → `RonRegistry<ParticleConfigSet>` |
| DialogueRegistry | `trees: Map<name,Tree>` | `paths: Map<name,raw>` | canonicalize-both per call | — | ✅ → `RonRegistry<DialogueTree>` |
| AnimationClipRegistry | `sets: Map<name,Set>` | `paths: Map<name,raw>` | canonicalize-both per call | — | ✅ → `RonRegistry<AnimationClipSet>` |
| **DataTableRegistry** | `tables: Map<name,Table>` | **path on `Table.path`** | canonicalize-both, by `t.path` | **`dirty` flag + `ReloadOutcome`** | ❌ bespoke |

The first three differ only cosmetically (what's stored + how matched) but produce identical observable behavior under `poll_reloads`. DataTable's path-on-value + dirty-skip + 4-variant outcome is a different contract (editor unsaved-edit protection), so it stays bespoke.

## Appendix B — `RonRegistry<V>` public (crate-internal) surface

```text
trait RonLoadable                       (native-only)
  type Err: Display
  fn load_ron(path: &str) -> Result<Self, Self::Err>

struct RonRegistry<V>                    (Default hand-impl; no V: Default bound)
  fn insert(&mut self, name: impl Into<String>, value: V)        (cross-platform)
  fn get(&self, name: &str) -> Option<&V>                        (cross-platform)
  fn names(&self) -> Vec<String>                                 (cross-platform, sorted)
  fn load(&mut self, name, path) -> Result<(), V::Err>           (native-only)
  fn reload_path(&mut self, path: &str, what: &str)              (native-only)
```

Wrapper pattern (each registry): `pub struct XRegistry { inner: RonRegistry<XValue> }` with delegating `load`/`get`/`names` (+ `insert` for all three now), `impl RonLoadable for XValue`, and `impl HotReloadable for XRegistry` (UFCS-delegates to inherent `reload_path`).

## Appendix C — scheduler rewrite (before → after)

Before (O(V·E) + O(V) per-pop scans):
```rust
let mut available: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
while let Some(&next) = available.iter().min() {          // O(V) per pop
    available.retain(|&x| x != next);                    // O(V) per pop
    order.push(next);
    for &(from, to) in &edges {                          // O(E) per pop → O(V·E)
        if from == next { indeg[to] -= 1; if indeg[to]==0 { available.push(to); } }
    }
}
// cycle: (0..n).filter(|i| !order.contains(i))          // O(V²)
```
After (O((V+E)·log V)):
```rust
let mut adj = vec![Vec::new(); n];                        // built once
for &(from,to) in &edges { adj[from].push(to); indeg[to]+=1; }
let mut ready: BinaryHeap<Reverse<usize>> =
    (0..n).filter(|&i| indeg[i]==0).map(Reverse).collect();
while let Some(Reverse(next)) = ready.pop() {             // O(log V)
    order.push(next);
    for &to in &adj[next] { indeg[to]-=1; if indeg[to]==0 { ready.push(Reverse(to)); } }
}
// cycle: (0..n).filter(|&i| indeg[i] > 0)                // popped ⟺ indeg hit 0
```

## Appendix D — test inventory (added/affected this session)

- `ecs::schedule::tests::*` — unchanged, still green (no_constraints insertion order, before_orders, shared_label_barrier, cycle_detected) → cover the scheduler determinism + cycle contract.
- `ron_registry::tests::insert_get_names_sorted`, `::load_then_reload_by_canonical_path` — **new**, cover the generic directly (incl. canonical-path reload).
- `particle::config_set::tests::reload_path_matches_canonical_key_from_poll_reloads` — unchanged, still green (the seq-1 regression guard) → proves the unified matching preserved behavior.
- `particle::config_set::tests::registry_insert_and_get_round_trip` — **edited** to use the new public `insert` instead of poking `.sets`/`.paths`.
- dialogue/animation registry tests — unchanged, green.
