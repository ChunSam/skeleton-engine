# Plan: hot-reload under an asset root (close the last EW-008 acceptance clause)

**Date:** 2026-07-15
**Status:** PLANNED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `asset-root-windows` seq `3`
**Context:** See `HANDOFF_asset-root-windows_embedded-images_2026-07-15.md` for this session's data (Phase 3 / `load_image_bytes` shipped), and its parents (seq 1 = asset roots, seq 2 = loud loaders) for the resolve()/identity constraints.

---

## Problem Statement

`engine::asset_path::resolve()` is applied at the **filesystem read only** — never to an asset's cache key, `Handle::path()`, or the hot-reload **watchers**. So the `notify` watcher registers the *caller's* logical path (e.g. `"assets/data/items.ron"`), while the file actually lives somewhere `resolve()` found it (next to the exe, in a bundle's `Resources`, an ancestor). In a packaged layout the logical path does not exist relative to the working directory, so `notify` silently fails to watch it (it cannot watch a nonexistent path on macOS/Linux) and F2 hot-reload never fires. This is the **one EW-008 acceptance clause the engine does not meet** — "F2 hot-reload keeps watching whatever path was resolved" — disclosed to dungeon-merchant in seq 2 rather than silently claimed. It is low-severity (hot-reload is a dev-from-repo-root activity, where logical == resolved and it works today), which is why it was deferred; this plan closes it cleanly.

## Key Findings

*(Conclusions from this chain — raw data in the handoffs.)*

- **`resolve()` runs only at ~10 fs-read sites; identity stays the caller's logical string** — the load-bearing invariant of the whole chain (rewriting identity re-breaks the 2026-05-29 white-sprite cache-key bug). → **constrains every phase: the fix must keep logical as identity.**
- **`asset_key(path)` canonicalizes *relative to the cwd***: it returns the absolute path when the file exists at `path` relative to cwd, else the raw string (`src/asset.rs:237`). → **drives Phase 1**: in a packaged layout `asset_key(logical)` = raw string, but `notify` reports the OS-canonical absolute path — so `watched_paths` (keyed by `asset_key(logical)`) and the notify event disagree even if the watch registered.
- **`watch_path(logical)` watches `logical` verbatim and stores `asset_key(logical)` in `watched_paths`** (`src/asset/hot_reload.rs:73-83`); the image path does the same at `src/asset/image_loading.rs:53` (`w.watch(path.as_ref(), …)`). → **drives Phase 2**: these two watch sites must watch `resolve(logical)` instead.
- **`poll_reloads` re-keys the notify-reported changed path with `asset_key` and checks membership in `path_to_id` / `watched_paths` / `atlas_path_to_id`, then dispatches by that key** (`src/asset/hot_reload.rs:36-57`); images re-decode via `decode_image_with_state(key)` (which re-`resolve()`s internally). → **drives Phase 2**: a notify event on the *resolved* file must be translated back to the *logical* key before the membership check and before the registries reload by name.
- **Registries (Data-table / RON / Script) key by the logical name/path the game passed** to `load_data_table(name, logical)` etc.; the `HotReloadable` forwarder (`schedule.rs`) reloads by the returned path string. → the dispatch must hand registries the **logical** path, not the resolved one.
- **Hot-reload failures deliberately stay `warn!`, never `record_failure`** (seq-2 decision — a `record_failure` panics under strict mode, and a half-saved RON mid-edit must not kill a strict dev build). → **Anti-Goal.**
- **The notify round-trip is not deterministically unit-testable** (`asset/tests.rs` tests `watched_paths` membership directly, noting "poll_reloads is not unit-testable without a real watcher"). → **drives Phase 3**: test the resolved→logical *mapping* directly + a manual/scripted hot-reload check, not the timing-dependent event loop.

## Anti-Goals (What NOT To Do)

- **Do NOT rewrite cache keys / `Handle::path()` / registry keys to the resolved path.** That reintroduces the 2026-05-29 white-sprite bug and breaks registry-by-name lookups. Identity stays logical; only the *watch target* and an internal *reverse map* become resolved.
- **Do NOT route hot-reload failures through `record_failure` / strict panic.** A reload failure is not a load failure (the file loaded once); a half-saved RON must not panic a strict dev build. Keep the existing `warn!`/skip behavior.
- **Do NOT add a second test to `tests/asset_root.rs`.** It moves the process-global cwd and must stay the only test in its integration binary.
- **Do NOT try to make the notify event loop a deterministic unit test.** Test the pure mapping; verify the round-trip manually or via a scripted smoke.
- **Do NOT resolve at read sites differently than today** — this plan only adds a watch-target resolution + reverse map; the ~10 read sites are already correct.

## Plan

### Phase 1: Map the watcher↔reload matching and choose the resolved→logical strategy

**Goal:** Pin down exactly how a notify event on the resolved file must be translated back to the logical key, for BOTH images (`path_to_id`) and registries (`watched_paths`), before writing code.

**Why this approach:** The finding that `asset_key` canonicalizes relative to cwd means the mismatch is real and subtle — building without confirming the match path risks a watcher that fires but dispatches under a key nothing recognizes. Design first (this is the grandparent's "worth designing before building" rule).

- Read `src/asset/hot_reload.rs` (all of `poll_reloads` + `watch_path`), `src/asset.rs` (the `notify::recommended_watcher` init, `watched_paths`, `path_to_id`, `atlas_path_to_id` fields), `src/asset/image_loading.rs:44-56` (the image watch site), `src/asset_path.rs` (`resolve`), and `src/app/schedule.rs` around the `poll_reloads` call + the `HotReloadable` forwarder dispatch.
- Answer concretely: when `notify` reports a changed path P (OS-absolute), what does `asset_key(P)` produce, and what key would the registry / `path_to_id` need to match? Confirm the mismatch in the packaged case and the *accidental* match in the dev-from-repo-root case (why it works today).
- Decide the data structure: a `watched_resolved_to_logical: HashMap<Arc<str>, Arc<str>>` on `AssetServer` mapping `asset_key(resolve(logical))` → the logical key. Confirm it covers images (whose `path_to_id` key is the logical string) and registries (whose dispatch key is the logical string) uniformly.
- Decide the translation point in `poll_reloads`: after reading a changed path, look it up in the reverse map (via `asset_key`) → logical key; fall back to the current `asset_key(path)` behavior when unmapped (so dev-from-repo-root and absolute-path callers are byte-identical).
- Write the design as 3-5 bullets in a scratch note (or inline in Phase 2's commit message) before coding.

**Files:** none modified (read-only design).
**Validates with:** a written statement of the mapping + the two cases (packaged mismatch / dev accidental-match), reviewed against the code. Success = you can name the exact key `poll_reloads` will compare on, in both layouts.
**Rollback:** n/a (no code changes).

### Phase 2: Watch the resolved path; translate events back to the logical key

**Goal:** `notify` watches the file that actually exists; `poll_reloads` dispatches under the logical key; identity is untouched.

**Why this approach:** Watching `resolve(logical)` is the only way `notify` registers a real file in a packaged layout; the reverse map is the only way to keep the logical key as the dispatch identity (the chain invariant).

- Add `watched_resolved_to_logical: HashMap<Arc<str>, Arc<str>>` to `AssetServer` (native-only field, like `watched_paths`).
- In `watch_path` (`hot_reload.rs:73`): resolve once — `let resolved = crate::asset_path::resolve(&path_str)` — call `w.watch(&resolved, NonRecursive)`, and insert `asset_key(&resolved) → asset_key(&path_str)` into the reverse map. Keep `watched_paths` keyed by the **logical** `asset_key(path_str)` (so existing `is_known`/registry logic is unchanged). Preserve idempotency (the `watched_paths.contains` guard).
- In the image watch site (`image_loading.rs:53-55`): watch `resolve(&*key)` instead of `path.as_ref()`, and register the same reverse-map entry. (The image's `path_to_id` key stays the logical `key` — unchanged.)
- In `poll_reloads` (`hot_reload.rs:36-45`): for each received path, compute `asset_key(path)`, look it up in `watched_resolved_to_logical` → the logical key; if unmapped, fall back to `asset_key(path)` (today's behavior). Run the `is_known` membership check and the `seen` dispatch on the **logical** key, so images re-decode under their `path_to_id` key and registries reload by their logical name.
- Keep everything `#[cfg(not(target_arch = "wasm32"))]` (hot-reload is native-only; wasm `poll_reloads` still returns empty).
- Do NOT touch the read sites, cache keys, `Handle::path()`, or the failure policy.

**Files:** `src/asset.rs` (new field + init in both `new()` branches — native map, wasm omit), `src/asset/hot_reload.rs` (`watch_path` + `poll_reloads`), `src/asset/image_loading.rs` (the watch site).
**Validates with:** `cargo test --lib` green; `./scripts/verify.sh` exit 0; the existing `watch_path_round_trips_and_is_idempotent` / `atlas_path_is_recognized_as_known_in_poll_reloads` tests still pass (they key on `watched_paths`, which stays logical).
**Rollback:** revert the three files; the reverse map is additive, so reverting restores exact prior behavior.

### Phase 3: Test the mapping + a real hot-reload proof

**Goal:** A regression test that the resolved→logical translation works, plus a runnable proof that an edit to a resolved-but-not-cwd-relative file triggers a reload.

**Why this approach:** The notify event loop isn't deterministically unit-testable (timing + a real watcher), so unit-test the pure mapping and prove the end-to-end behavior with a scripted/manual check — mirroring how seq-1 proved `resolve()` with `tests/asset_root.rs`.

- Unit test in `src/asset/tests.rs`: after `watch_path(logical)` where `logical` resolves to a temp file NOT under cwd, assert `watched_resolved_to_logical` maps `asset_key(resolved) → asset_key(logical)`, and (if a pure helper is extracted) that translating a simulated notify path yields the logical key. Key on paths **unique to the test**; never assert on process-global list lengths; do not call `set_asset_root()` (use an absolute temp path so `resolve` passes it through, per the seq-2 rule).
- Consider extracting the translation as a pure fn (`fn logical_for_changed(&self, changed: &Path) -> Option<Arc<str>>`) so it IS unit-testable without a watcher — this is the cleanest way to pin the behavior.
- Runnable proof: either extend the existing `packaged_assets` example / add a small `scripts/hot_reload_smoke.sh` that (a) writes a RON data table to a temp dir, (b) `set_asset_root`s (or launches from `/`) so it's resolved-not-cwd, (c) loads it, (d) edits the file, (e) drives a few `App::update` frames, (f) asserts the reload fired. If a deterministic script proves too flaky (notify latency), document a manual verification recipe instead and say so explicitly (no silent gap).
- Prove non-vacuous where possible: the mapping unit test should fail if Phase 2's reverse-map insertion is removed.

**Files:** `src/asset/tests.rs` (+1-2 tests), optionally `scripts/hot_reload_smoke.sh` (new) or a doc note in the module.
**Validates with:** the new unit test(s) pass and fail-without-the-fix; the smoke (or a documented manual run) shows a reload firing from a foreign cwd.
**Rollback:** delete the added test(s)/script; no runtime impact.

## Dependencies & Order

- **Phase 1 → Phase 2 → Phase 3, strictly sequential.** Phase 2 depends on Phase 1's chosen key-translation strategy; Phase 3 tests Phase 2's map.
- **Board-check gate precedes Phase 1** (see Quick Start): a newly-filed EW/rust-survivors request preempts this entire plan.
- No parallelism — it's one small, coupled change across three files.

## Risks & Mitigations

- **The reverse-map key doesn't match what notify reports** (macOS may report a `/private`-prefixed canonical path; `asset_key` uses `canonicalize`, which should normalize both sides). *Likely: medium.* Mitigation: key the map by `asset_key(resolved)` on both the insert and the lookup, so both go through the same canonicalization; Phase 1 confirms this on macOS specifically.
- **notify latency makes a scripted smoke flaky.** *Likely: medium.* Mitigation: Phase 3's fallback is a documented manual recipe + the pure-mapping unit test as the real regression guard; do not ship a flaky test.
- **A resolved watch on a directory the OS coalesces events for** could double-fire. *Likely: low.* Mitigation: `poll_reloads` already dedups via `seen.contains`; the logical-key translation happens before the dedup, so duplicates collapse.
- **Touching `AssetServer::new()` on both targets.** *Likely: low.* Mitigation: the new field is native-only (`#[cfg(not(wasm32))]`); the wasm `new()` branch omits it, matching `_watcher`/`reload_rx`.

## Success Criteria

- **Minimum viable:** `watch_path(logical)` in a packaged/foreign-cwd layout registers a `notify` watch on the *resolved* file, and a `poll_reloads` event on that file returns the *logical* key. Unit-test-pinned. `./scripts/verify.sh` exit 0. Dev-from-repo-root behavior byte-identical (existing tests green).
- **Full:** a runnable proof (smoke or documented manual run) shows F2 hot-reload firing for a data table / image whose file is resolved-not-cwd-relative — the EW-008 clause met. The dungeon-merchant board thread on EW-008 can be updated (or the clause noted as now-closed) if the user wants.
- **Invariant preserved:** no cache key, `Handle::path()`, or registry key changed; no sprite renders white; `cargo test --lib` count only grows by the new tests.

## Quick Start

```bash
# Restore context
cat plans/handoffs/HANDOFF_asset-root-windows_embedded-images_2026-07-15.md

# BOARD-CHECK GATE (before any code) — a new request preempts this plan
#   ../dungeon-merchant/docs/engine-wishlist.md          (next free ID EW-009; EW-007/008 awaiting the game's verify)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md     (currently _None._)

# Key files for Phase 1 (read before touching anything)
#   src/asset/hot_reload.rs            — poll_reloads + watch_path (the match logic)
#   src/asset.rs                       — AssetServer fields (watched_paths, path_to_id), the notify init
#   src/asset/image_loading.rs         — the image watch site (line ~53) + load_image
#   src/asset_path.rs                  — resolve()
#   src/app/schedule.rs                — the poll_reloads call + HotReloadable forwarder dispatch

# Verify starting state (read the exit code — do NOT pipe or `;`-chain it)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# First concrete action (Phase 1): read hot_reload.rs + trace how asset_key(notify_path)
# compares against watched_paths in the packaged vs dev-from-repo-root layouts, then decide
# the watched_resolved_to_logical map shape. NO code until the mapping is written down.
```
