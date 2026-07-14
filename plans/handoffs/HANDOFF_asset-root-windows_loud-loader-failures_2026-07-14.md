# Loud failures for every path-based loader: the board's EW-007 served, EW-008 answered (v0.127.0)

**Date:** 2026-07-14
**Status:** COMPLETED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `asset-root-windows` seq `2`
**Parent:** `plans/handoffs/HANDOFF_asset-root-windows_downstream-bug-report_2026-07-14.md` (seq 1)
**Prior chain:** `HANDOFF_asset-root-windows_downstream-bug-report_2026-07-14.md` > this

---

## Stale References

Everything the parent named still exists (`asset_path::resolve`, `record_failure`, `asset_failures`, `set_strict_assets`, `resolve_in`, `tests/asset_root.rs`, `examples/packaged_assets.rs`). One correction, though:

- **`scripts/packaged_assets_smoke.sh` did NOT exist** — the seq-1 example's own module doc referenced it ("so `scripts/packaged_assets_smoke.sh` can run it from a foreign working directory as a real test"), but the file was never written; that session ran the check by hand. **This session wrote it**, so the reference is now true. If you find another doc line in this chain pointing at a script, check it exists before trusting it.

## Since Last Handoff

- Parent's plan was: land the seq-1 handoff, then **read the board first; if empty, ASK** — the self-pick shelf was exhausted. Landing happened (#360, `c45f9fa`). **The board was not empty.** dungeon-merchant filed **EW-007** (P1) and **EW-008** (P2) on 2026-07-14, hours after the parent session closed. So this session never had to ask: the work was on the board.
- **The two requests were one story, and half of it was already fixed.** EW-008 ("asset paths are CWD-relative — no exe-relative resolution for packaged builds") is *exactly* the bug the parent session fixed in v0.126.0 — **one day before it was filed**. Two independent downstream games (rust-survivors, dungeon-merchant) hit the identical defect packaging their first Windows builds within a week of each other. Strong signal the fix belonged in the engine.
- **EW-007 was the real work, and the parent left the gap open.** v0.126.0 built `record_failure` (loud `error!` + `asset_failures()` + strict panic) but wired it at **only the 2 image read sites**. Every other loader still `warn!`d and registered nothing. That is precisely what bit dungeon-merchant: 19 data tables resolved to nothing, each registered empty, and the game booted with an empty shop and no dungeons.
- **The parent's "hot-reload under an asset root" open item became load-bearing** — not as work, but as *disclosure*. EW-008's acceptance criteria included "F2 hot-reload keeps watching whatever path was resolved", which the engine does **not** satisfy (watchers still register the caller's path). Told the game plainly rather than claiming the criterion.
- Parent's other candidates (**Phase 3 `App::load_image_bytes`**, the **vorbis/.ogg test gap**) were *not* touched. Both still open, both still non-urgent.
- Trajectory: unchanged in shape (serve the downstream reports), but the request channel moved — seq 1 was driven by `rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md`, seq 2 by dungeon-merchant's EW board. Both channels are now empty of open items.

## Reference Documents

- `CLAUDE.md` — conventions, module map, verify-gate rules. **Updated this session** (header → v1.6.220 / package v0.127.0; the `asset_path` module-map row rewritten to say every loader reports).
- `docs/VISION.md` — the feature+example loop ("the example is the acceptance test"). This session leaned on it hard: the example *is* the EW-007 regression test.
- `docs/CHANGELOG.md` — 0.127.0 entry written this session.
- **The request source:** `../dungeon-merchant/docs/engine-wishlist.md` — the shared EW board. Both sides edit it. Next free ID **EW-009**.

---

## The Goal

Serve **EW-007** (P1): *"`load_data_table` fails SILENTLY on a missing/unreadable file — a packaged build boots as an empty game."* The game shipped its first packaged Windows build (exe + `assets/`), and it ran — with **0 items and 0 dungeons**. No error anywhere. The root cause was the game's (CWD-relative paths), but the reason it took a *player's* bug report to find is the engine's: all 19 `load_data_table(...)` calls hit missing files, each `warn!`d and registered an **empty table**, and every downstream `reg.get("items")` then returned a valid-but-empty table. The game ran "correctly" on no data.

The class of bug is what makes it P1, not the severity: a silent empty table is **indistinguishable from a legitimately empty one**, and the symptom (an empty shop screen) surfaces three scenes away from the cause (the loader). Any engine user shipping a packaged build hits this exactly once — and blames their own content pipeline.

End state: every path-based loader in the engine reports its failures through one channel (`asset_failures()`), the example that demonstrates it is a runnable acceptance test, both board requests are answered, and the game can delete its hand-rolled probe.

---

## Where We Are

- **`main @ bdbbb04`, package v0.127.0, CLAUDE.md header v1.6.220, clean tree, all gates green.**
- **Engine PR #361 — MERGED** `bdbbb04` (2026-07-14T00:46:58Z, auto-merge, CI **6/6** including `Build (Windows / DX12)`).
- **Game PR #27 (dungeon-merchant) — MERGED** `ec78341`. Board updated: **EW-007 → `Shipped (v0.127.0)`**, **EW-008 → `Shipped (v0.126.0)`**. Both are now in the game's court (verify → set `[x]`).
- **7 `App::load_*` RON wrappers** now report through `crate::asset_path::record_failure`: `load_data_table`, `load_animation_clips`, `load_particle_configs`, `load_dialogue`, `load_trigger_zones`, `load_zone_effects`, `load_anim_effects` — all in `src/app/editor/loading.rs`.
- **Rhai scripts report both failure modes** (`src/scripting/loading.rs`): a failed **read** *and* a failed **compile**. The compile case was a silent one nobody had noticed — a script that reads fine but won't compile fell back to an empty AST and the entity just… did nothing.
- **Audio file reads report** (`src/audio/playback.rs`, the `read_cached_bytes` miss path).
- **Failure messages name the registry key**, not just the path: `data table 'items': …`. That is what the game's own `reg.get("items")` looks up, so the error speaks the game's language.
- **No signature changed.** `load_data_table` deliberately still returns `()` — see Key Decisions.
- **Two deliberate non-failures, both test-pinned:** a *hot-reload* failure stays `warn!`; an *intentionally empty* (valid RON, zero rows) table registers normally.
- **Example `packaged_assets` is now the acceptance test.** It loads a real texture + a real data table + a missing texture + a missing data table. Under `HEADLESS_SHOT` it exits non-zero unless: the real texture resolves, the real table resolves, **the table carries rows**, and **the missing table is reported**.
- **`scripts/packaged_assets_smoke.sh` (new)** runs that example **from `/`** — the working directory a shipped build actually gets. Running it from the checkout is the case that always worked, so it proves nothing.
- **The acceptance test was verified non-vacuous.** Reverting `record_failure` in `load_data_table` turns it red with exactly the intended message.
- **Test count: 1243 lib tests** (was 1241 at seq-1 close; +2 new unit tests in `loading.rs`).
- One gate trip, fixed: rustdoc `private_intra_doc_links` (the module doc linked `record_failure`, which is `pub(crate)`).
- Memory bumped: `engine-current-state` → seq **181**; next free board ID recorded as **EW-009**.
- **Both downstream request channels are now empty of open items** — dungeon-merchant's board (EW-007/008 `Shipped`, awaiting their verify) and `rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`, emptied in seq 1).
- **The whole change is additive.** No public signature changed, no example needed editing (beyond the one deliberately extended), and a load that *succeeds* behaves byte-identically. A downstream pin bump cannot break on this.
- **What a game must now write to catch the bug this fixes:** `if !app.asset_failures().is_empty() { … }` after its loads — or `app.set_strict_assets(true)` in a dev build. That is the entire adoption cost.

---

## What We Tried (Chronological)

### Chunk 1 — Reading the handoff, and finding the board had moved (early)

1. **User asked to check the last handoff and report the work** ("마지막 핸드오프 확인하고 작업 알려줘"). Ran `git log --oneline -8` + read the dungeon-merchant board in parallel.

2. **`docs/HANDOFF.md` is a red herring.** It is 2681 lines and its *tail* is ancient (a v4.3.0-era Phase table). The living handoffs are in **`plans/handoffs/`**. Found the seq-1 file via `git show --stat --name-only c45f9fa` after a wrong guess at `docs/handoffs/` returned ENOENT. **Next session: go straight to `ls -t plans/handoffs/`.**

3. **The board had two NEW requests**, filed 2026-07-14 — after the parent handoff was written (which recorded the board as EMPTY, next free ID EW-007):
   - **EW-007** (P1) — `load_data_table` fails silently.
   - **EW-008** (P2) — asset paths are CWD-relative.

4. **Did not take either at face value; checked both against current `main`.** This mattered enormously — one of the two was already fixed:
   ```
   $ grep -rn "resolve\|record_failure" src/data_table.rs
   src/data_table.rs:139:  let text = std::fs::read_to_string(crate::asset_path::resolve(path))?;
   ```
   `DataTable::load` **already** resolves through the asset root → **EW-008 shipped in v0.126.0**, one day before it was filed.

5. **Then found what was *not* fixed** — `App::load_data_table` (`src/app/editor/loading.rs:27`):
   ```rust
   if let Err(e) = reg.load(name, &path) {
       log::warn!("load_data_table: failed to load '{path}': {e}");
       return;
   }
   ```
   A `warn!` and a silent return. The registry keeps no table; the caller gets no signal.

6. **The decisive grep — counted `resolve()` sites against `record_failure()` sites:**
   ```
   $ grep -rn "asset_path::resolve" src/     → 12 sites
   $ grep -rn "record_failure" src/          → 2 sites (renderer/texture.rs, asset/image_loading.rs)
   ```
   v0.126.0 had built the loud-failure machinery and wired it to **images only**. Ten loaders resolved their paths but still swallowed their failures. **That gap *is* EW-007** — and it was wider than the request described (the game only knew about data tables).

7. **Reported to the user in Korean** with the loader table, recommending: implement EW-007, and answer EW-008 as already-shipped. User: **"진행해."**

### Chunk 2 — Implementation (mid)

8. **Read `src/asset_path.rs`** to reuse rather than invent. `record_failure(path, error)` is `pub(crate)`; it attaches **the roots searched** to the message, logs `error!`, **panics first** if strict mode is on, and **dedups by path** (one `load_image` feeds both the `AssetServer` and the renderer's upload queue, so a missing image surfaced twice). All three shapes EW-007 asked for — loud log, an accessor to assert on, a strict panic — already existed. Nothing new needed inventing; the fix was *routing*.

9. **Rewired the 7 RON wrappers.** Each had the identical shape. One wrinkle: `reg.load(name, &path)` **moves** `name`, so the failure message couldn't name the table. Fixed with `name.clone()`:
   ```rust
   if let Err(e) = reg.load(name.clone(), &path) {
       crate::asset_path::record_failure(&path, format!("data table '{name}': {e}"));
       return;
   }
   ```

10. **Rewrote all 7 doc comments.** Every one said *"Errors (file not found, parse failure) are logged via `log::warn!` and silently dropped"* — a docs line that documented the bug as if it were a feature.

11. **Scripts: fixed a second, unreported silence.** `src/scripting/loading.rs` already logged the read failure at `error!`, but its **compile** failure fell back to an empty AST with only a log. A script that reads but won't compile is *just as silent* as a missing one — the entity keeps running and does nothing. Both now `record_failure`.

12. **Audio:** `read_cached_bytes`'s `Err` arm (`src/audio/playback.rs`) went from `warn!("Cannot open audio file …")` to `record_failure`.

13. **Deliberately left hot-reload alone** (`src/ron_registry.rs:115`, the `poll_reloads` path). Reasoning below in Key Decisions.

14. **Added the "One failure list, for every kind of asset" section to `asset_path`'s module doc** — the module's docs were written entirely around the magenta-texture story and said nothing about content assets.

15. **Two unit tests** in `src/app/editor/loading.rs`:
    - `a_missing_data_table_is_recorded_as_an_asset_failure` — also asserts the registry is left **without** the name (no empty table behind it) and that the message names the table.
    - `an_intentionally_empty_data_table_is_not_a_failure` — writes `[]` to a temp file, loads it, asserts it registers with 0 rows and is **not** in `asset_failures()`. This is EW-007's own stated acceptance criterion, and it is the one a careless implementation breaks.

    Both obey the seq-1 rules the hard way (that session shipped a CI-red test by violating them): **key on a path unique to the test, never on `asset_failures()`'s length** (the list is process-global and shared with every parallel test in the binary), and **never call `set_asset_root()`** (also process-global). The empty-table fixture uses an **absolute** temp path, which `resolve` passes through untouched — so it cannot be perturbed by `tests/asset_root.rs` moving the cwd in its own process.

16. **Extended `examples/packaged_assets.rs` rather than writing a new example.** Its story *is* EW-007's story (a packaged build that boots blank), so a second example would have split one lesson in two. It now loads `examples/games/stat_editor_game/items.ron` (a real 2-row table) and `examples/assets/__deliberately_missing__.ron`, and the HUD prints `loaded: items.ron (2 rows)` — because **0 rows is exactly what the silent failure looked like**.

17. **Made the headless mode a 3-part assertion** (it was 1 part): the real texture resolves, the real table resolves **with rows**, and the missing table **is reported**. That last clause is the one that fails on the old behavior.

18. **Wrote `scripts/packaged_assets_smoke.sh`** — the example's own doc comment had promised this script since seq 1, but it never existed. It builds the example and runs it **from `/`**.

### Chunk 3 — Verification (mid/late)

19. **Ran the smoke. It passed:**
    ```
    >>> [2/3] running it from / (the launch that used to render a magenta window)...
    OK: texture + data table (2 rows) resolved; reported failures =
        ["examples/assets/__deliberately_missing__.png", "examples/assets/__deliberately_missing__.ron"]
    PASS: relative assets resolved from /, and the missing data table was reported
    ```
    The `.ron` in that list is the whole fix — before this session it was absent, and its absence was the bug.

20. **Proved the test isn't vacuous** — the step seq 1 taught by example. Reverted `record_failure` back to `warn!` in `load_data_table` only, rebuilt, ran from `/`:
    ```
    FAIL (working dir Ok("/")):
      - 'examples/assets/__deliberately_missing__.ron' is missing but was NOT reported
        — a failed data table must never be silent
    EXIT_WITHOUT_FIX=1
    ```
    Restored the fix. **A regression test that passes without the fix is worthless; this one was checked.**

21. **First full gate: `VERIFY_EXIT=101`.** All 1243 lib tests passed; rustdoc failed:
    ```
    error: public documentation for `asset_path` links to private item `record_failure`
      --> src/asset_path.rs:49:44
      = note: `-D rustdoc::private_intra_doc_links` implied by `-D warnings`
    ```
    My new module-doc section used an intra-doc link `[`record_failure`]` — but it is `pub(crate)`. De-linked to inline code. **Re-ran: `VERIFY_EXIT=0`.**

### Chunk 4 — Shipping and answering the board (late)

22. **`/land-pr` → `/ship`.** Branch `feat/loud-loader-failures`; `Cargo.toml` 0.126.0 → **0.127.0**; `cargo update -p skeleton-engine` (`Locking 0 packages`); CHANGELOG 0.127.0 entry; `CLAUDE.md` header v1.6.219 → **v1.6.220** and the `asset_path` module-map row rewritten (it claimed loud failures were general when they were images-only). **Re-verified after the bump: `VERIFY_EXIT=0`.**

23. **PR #361 opened, auto-merge armed** (`gh pr merge 361 --auto --squash`). No judgment gate applied — the change is pure Rust logic covered by CI's own tests; the GPU-dependent half (the example) I had already run locally.

24. **Answered the board while CI ran.** Both threads got an `[Engine]` reply (append-only, per the board's protocol), and both Status fields moved to `Shipped`.

25. **Verified the game's pin before writing the reply** rather than trusting the board's text:
    ```
    Cargo.toml: skeleton-engine = { git = "…", rev = "c42a8905…" }
    Cargo.lock: version = "0.116.0"
    ```
    So the reply says: bump **once** (0.116 → 0.127) and *both* EW-007 and EW-008 land together.

26. **Board PR (game repo) #27** — dungeon-merchant routes every commit through a PR, so the board update did too. No CI on that repo (`mergeStateStatus: CLEAN`, zero checks) → squash-merged `ec78341` and synced its main. Note: `gh api …/branches/main/protection` returns **403** there ("Upgrade to GitHub Pro or make this repository public") — the repo is private without protection, so the PR convention is discipline, not enforcement.

27. **CI on #361 came green 6/6** (`Test (native)` last, as always) and auto-merge landed `bdbbb04`. Synced main, pruned the branch, bumped memory to seq 181.

### What the board reply actually says (the part that saves the game time)

28. The `[Engine]` reply on **EW-007** does three things beyond "shipped": it maps their **three offered shapes** onto what was built (loud log ✅, accessor ✅ as `asset_failures()`, strict panic ✅) and explains why the fourth (`Result`) was declined; it hands them **the replacement for their probe** (`if !app.asset_failures().is_empty()`, or `set_strict_assets(true)` in dev — covering all 19 tables, not the one they remembered); and it states that **both their acceptance criteria are test-pinned**, naming the non-vacuity check so they don't have to take it on faith.

29. The reply on **EW-008** leads with the fact that it was **already fixed before it was filed**, and — the useful part — that *another* game (`rust-survivors`) hit the identical bug from the *texture* side (a solid magenta window) while they hit it from the *data* side (an empty game). Two independent confirmations of one defect. It then names the **one unmet acceptance clause** (hot-reload watchers) rather than letting them discover it.

---

## Key Decisions

- **`load_data_table` does NOT start returning `Result`.** EW-007 explicitly offered this as an acceptable shape ("returns a `Result`"), and it was tempting. Rejected: `Result` is `#[must_use]`, so adding it would emit an unused-result **warning at every existing call site** — in this repo's examples, in the game, in any fork — and under a `-D warnings` build that is a hard break, for a change whose whole point is to be adoptable in a pin bump. The `asset_failures()` list gives the same power (assert at boot, refuse to start) with zero call-site churn. Strict mode gives the panic option for dev builds.

- **One shared failure channel, not a per-registry accessor.** EW-007 also offered `DataTableRegistry::missing()` / `load_errors()`. Rejected: that solves *data tables* and leaves the same hole in nine other loaders — including the two (script compile, audio) the game hasn't hit yet. A game asserting on `asset_failures()` at boot catches **every** asset kind with one check. The request's framing ("every engine user must reinvent it") argues for the general fix.

- **Hot-reload failures deliberately stay `warn!`.** A reload failure is categorically different from a load failure: the file **already loaded once**, so nothing is silently empty — and `record_failure` *panics* under strict mode. A developer with `set_strict_assets(true)` on, editing a RON file, would have their game die on the half-saved intermediate state. Ruinous ergonomics for zero safety gain. Documented in the module doc so the omission reads as a decision, not an oversight.

- **The failure message names the registry key** (`data table 'items': …`), not only the path. The path is what the *engine* used; the key is what the *game's code* looks up (`reg.get("items")`). The error should be greppable from the symptom the developer sees.

- **Extended the existing example instead of writing a new one.** `packaged_assets` already tells the packaged-build story; the data table is the same story's second half. A new example would have implied they were separate lessons. It also let the smoke script assert both halves in one run.

- **A data table that fails to load must leave NOTHING behind under its name** (test-pinned). Registering an empty table "so the game doesn't crash" is precisely the behavior that caused the bug: `get("items")` returning `Some(empty)` is what let the game sail on.

- **Wrote the smoke script rather than deleting the doc line that promised it.** The seq-1 example's docs referenced a script that did not exist. With the example now carrying a real exit code, the script was ~40 lines. Making the doc true beat making it quieter.

- **Answered EW-008 with its unmet criterion stated plainly.** Its acceptance criteria included "F2 hot-reload keeps watching whatever path was resolved" — which the engine does **not** do (watchers register the caller's path; `resolve` runs at the read only). Marking EW-008 `Shipped` while quietly ignoring that clause would have been a small lie that costs the game an afternoon later. The reply names the gap and offers to fix it if they hit it.

---

## Evidence & Data

### The gap that was EW-007 — `resolve()` sites vs. `record_failure()` sites, before this session

| Read site | `resolve()` (v0.126.0) | Reported failure (v0.126.0) | Reported failure (v0.127.0) |
|---|---|---|---|
| `renderer/texture.rs` | ✅ | ✅ `record_failure` | ✅ |
| `asset/image_loading.rs` | ✅ | ✅ `record_failure` | ✅ |
| `data_table.rs` (via `App::load_data_table`) | ✅ | ❌ `warn!`, empty registry | ✅ **EW-007** |
| `animation/clip_set.rs` (`load_animation_clips`) | ✅ | ❌ `warn!` | ✅ |
| `particle/config_set.rs` (`load_particle_configs`) | ✅ | ❌ `warn!` | ✅ |
| `dialogue/tree.rs` (`load_dialogue`) | ✅ | ❌ `warn!` | ✅ |
| `trigger_zone.rs` (`load_trigger_zones`) | ✅ | ❌ `warn!` | ✅ |
| `zone_effect.rs` (`load_zone_effects`) | ✅ | ❌ `warn!` | ✅ |
| `anim_effect.rs` (`load_anim_effects`) | ✅ | ❌ `warn!` | ✅ |
| `scripting/loading.rs` (read) | ✅ | ❌ `error!` only, no list | ✅ |
| `scripting/loading.rs` (**compile**) | n/a | ❌ `error!` only, empty AST | ✅ |
| `audio/playback.rs` (`read_cached_bytes`) | ✅ | ❌ `warn!` | ✅ |
| `ron_registry.rs` (**hot-reload**) | ✅ | `warn!` | **`warn!` — deliberate** |

### The acceptance test, both directions

| Run | Command | Result |
|---|---|---|
| With the fix | `cd / && HEADLESS_SHOT=… packaged_assets` | `OK: texture + data table (2 rows) resolved; reported failures = [".png", ".ron"]` → **exit 0** |
| `record_failure` reverted to `warn!` in `load_data_table` only | same | `FAIL … '__deliberately_missing__.ron' is missing but was NOT reported` → **exit 1** |

### Gate history

| Run | Result | Cause |
|---|---|---|
| verify #1 (post-implementation) | **101** | rustdoc `private_intra_doc_links` — module doc linked `pub(crate) record_failure` |
| verify #2 (after de-linking) | **0** | 1243 lib tests, 110 doctests |
| verify #3 (after `/ship` bump) | **0** | lock + doc build re-checked |
| CI #361 | **6/6** | incl. `Build (Windows / DX12)`, `Render tests (lavapipe)` |

### Test count trajectory

| Point | Lib tests |
|---|---|
| seq-1 session start | ~1229 |
| seq-1 close (v0.126.0) | 1241 |
| **seq-2 close (v0.127.0)** | **1243** (+2: missing-table recorded, empty-table-is-not-a-failure) |

### Board state — before and after this session

| ID | Priority | Status before | Status after | Notes |
|---|---|---|---|---|
| EW-007 | P1 | `Proposed` (2026-07-14) | **`Shipped (v0.127.0)`** | Served this session |
| EW-008 | P2 | `Proposed` (2026-07-14) | **`Shipped (v0.126.0)`** | Already fixed one day *before* it was filed |
| EW-001…006 | — | `Verified` / archived | unchanged | — |
| Next free ID | — | EW-009 | EW-009 | Board's own header |

### The downstream symptom, in the game's words (why this was P1)

> "We shipped our first packaged Windows build (exe + `assets/`) and it ran — but with **0 items and 0 dungeons**: an empty 상점 catalog and no dungeon to select, no error anywhere. […] all 19 `load_data_table(...)` calls resolved to nonexistent files and each one no-op'd quietly, registering an empty table. […] A silent empty table is indistinguishable from a legitimately empty one, and the failure surfaces *far* from the cause (an empty shop screen, three scenes later)."
>
> — filed after a player bug report: *"던전이 하나도 없고 상점이 비어 있다"*

### The game's workaround it can now delete

```rust
// game src/main.rs — a sentinel probe on ONE table, standing in for all 19
if !std::path::Path::new(&data_path("items.ron")).is_file() {
    eprintln!("[assets] ERROR …");
}
```
Replacement, covering all 19: `if !app.asset_failures().is_empty() { … }` — or `app.set_strict_assets(true)` in a dev build.

### Merge log

| Repo | PR | Commit | What |
|---|---|---|---|
| skeleton-engine | **#361** | `bdbbb04` | v0.127.0 — loud failures for every path-based loader |
| dungeon-merchant | **#27** | `ec78341` | Board: EW-007 `Shipped (v0.127.0)`, EW-008 `Shipped (v0.126.0)` |

### CI on #361 — the 6 required checks (Windows became required in seq 1)

| Check | Result |
|---|---|
| Build (WASM) | ✅ |
| **Build (Windows / DX12)** | ✅ (added + made *required* in seq 1; auto-merge waits for it) |
| Rustdoc | ✅ |
| Package dry-run | ✅ |
| Render tests (lavapipe) | ✅ |
| Test (native) | ✅ — always last (~4–6 min); the others were green while it ran |

`mergeStateStatus` read **BLOCKED** while `Test (native)` was still `IN_PROGRESS` — that is *normal*, not a problem. Auto-merge landed the PR the moment it flipped.

### The data-table fixture the example loads (real, 2 rows)

```ron
# examples/games/stat_editor_game/items.ron
[
    (name: "Potion", heal: 25, price: 10),
    (name: "Sword",  damage: 12, price: 50),
]
```
Chosen because it already existed (loaded by `examples/games/stat_editor_game/stat_editor.rs:214` via `load_data_table`) — no new fixture to maintain, and the **row count is the proof**: the HUD prints `loaded: items.ron (2 rows)`, and `0 rows` is precisely what the silent failure produced.

### The board's protocol (read before editing it — both repos edit this file)

| Field | Meaning |
|---|---|
| `- [ ]` / `- [x]` | open / engine shipped **AND** game verified |
| Status `Proposed` → `Acknowledged` → `In-progress` → `Shipped (vX.Y.Z)` → `Verified` | `Needs-info` and `Verified` are the **game's** court; everything else is the **engine's** |
| Thread | **Append only** — add `- [Engine] YYYY-MM-DD — …` at the bottom; never rewrite an existing line |
| Dates | Absolute only (`YYYY-MM-DD`), no relative dates |
| Move when done | After `- [x]`, the item moves to "Done / archive" |

Marking `Shipped` (not `Verified`) is the engine's terminal state — **only the game closes an item.** This session moved both to `Shipped` and left the `[ ]` boxes unchecked.

### Example changes at a glance (`examples/packaged_assets.rs`)

| Before (v0.126.0) | After (v0.127.0) |
|---|---|
| 2 assets: 1 real texture + 1 missing texture | **4**: + 1 real data table + 1 missing data table |
| HUD: cwd, roots, failures panel (`take(3)`) | + `loaded: items.ron (N rows)` line; failures panel `take(4)`; roots block shifted down (y 106→124, rows 128→146) |
| Headless: 1 assertion (real texture resolved) | **3**: real texture + real table resolve · table has rows · **missing table is reported** |
| Smoke script: referenced in docs, did not exist | `scripts/packaged_assets_smoke.sh` — builds, runs from `/`, asserts the `OK:` line + a real PNG |

---

## Code Analysis

- **`record_failure(path: &str, error: impl Display)`** (`src/asset_path.rs`, `pub(crate)`) — the whole contract in one function: appends the searched roots to the message, `log::error!`s, **panics before recording** if `strict_assets()`, then pushes to a process-global `Mutex<Vec<AssetFailure>>` **deduped by path** (first error for a path wins). Because it panics *before* the dedup check, strict mode fires on the first failure regardless of ordering.
- **`AssetFailure { path: String, error: String }`** — `path` is the caller's **logical** string, never the resolved one. That is deliberate and load-bearing (see the parent's Key Decisions: identity stays logical, resolution stays at the filesystem edge, or the 2026-05-29 white-sprite cache-key bug returns).
- **The 7 wrappers share one shape** (`src/app/editor/loading.rs`): ensure registry resource → `register_persistent::<Registry>()` → `#[cfg(not(wasm32))]` { `reg.load(name.clone(), &path)` → on `Err`, `record_failure` + early return; else `assets.watch_*_path(&path)` } → wasm branch is a documented no-op. The early return on failure is why a failed load leaves **no** entry under the name.
- **`DataTableRegistry::load(&mut self, name: impl Into<String>, path: &str) -> Result<(), SaveError>`** — takes `name` **by value**, hence the `name.clone()` needed to also name it in the failure message.
- **`DataTable::load(path) -> Result<Self, SaveError>`** already resolved (`src/data_table.rs:139`) — the read was fine; only the *reporting* above it was missing. This is why EW-008 was already satisfied for data tables while EW-007 was not.
- **`asset_failures()` is process-global.** For tests this is a footgun with two edges, both of which bit seq 1: never assert on the list's **length** (parallel tests in one binary record into it), and never call `set_asset_root()` in a unit test (it mutates the root every other test's `resolve()` reads). `resolve_in(roots, path)` is the pure escape hatch.
- **Absolute paths bypass the whole candidate search** (`resolve` returns them unchanged) — which is what makes the empty-table test's temp-file fixture immune to the cwd games `tests/asset_root.rs` plays in its own process.
- **The wasm branches record nothing.** `load_*` is a documented no-op on wasm (no filesystem), and script loading there returns an empty source with a `warn!`. Left as-is: turning a documented platform no-op into a recorded "failure" would spam the list on every wasm boot.
- **`App::asset_failures()` / `App::set_strict_assets()`** live in `src/app/assets.rs` (thin forwarders to the `asset_path` free functions). A game never needs to touch the module directly — which is why the board reply quotes the `App` methods, not `asset_path::`.
- **`record_failure` works on wasm** (its `candidate_roots()` is a stub returning an empty vec, and the message then omits the "searched:" suffix), so the script-**compile** path — the one cross-platform site — compiles on both targets without a `cfg`. The new `mod tests` **is** `cfg`-gated (`all(test, not(target_arch = "wasm32"))`) because it constructs an `App` and touches the filesystem.
- **The example's headless assertion is written as a `problems: Vec<String>`**, not a chain of early `exit(1)`s — so a run reports *every* violated clause at once (e.g. "table loaded no rows" *and* "missing table not reported") rather than the first. Cheap, and it turns one run into a full diagnosis.

### Where the session's own tooling lives

- **Handoffs are in `plans/handoffs/`, not `docs/handoffs/`** (which does not exist). `docs/HANDOFF.md` is a 2681-line *historical* doc whose tail is v4.3.0-era — it is **not** where recent sessions write. `ls -t plans/handoffs/ | head` is the fast path.
- **`scripts/verify.sh`** = the whole CI-equivalent gate in order (fmt → clippy → wasm build → wasm clippy → test --all-targets → doctests → rustdoc). Read its exit code from a **non-piped** call; this repo has been bitten twice by `| tail` and once by `;`-chaining (seq 1 pushed a red tree that way).

---

## Files Changed

### Source code
- `src/app/editor/loading.rs` — the 7 `App::load_*` RON wrappers now `record_failure` (naming the registry key) instead of `warn!`; all 7 doc comments rewritten (they documented the silent drop as intended behavior); **+ a new `#[cfg(all(test, not(wasm32)))] mod tests`**.
- `src/scripting/loading.rs` — read failure and **compile** failure both `record_failure`.
- `src/audio/playback.rs` — `read_cached_bytes`'s miss path `record_failure`s instead of `warn!`.
- `src/asset_path.rs` — new module-doc section, "One failure list, for every kind of asset": what reports, why the empty-table case is the dangerous one, and the two deliberate non-failures. (Careful: `record_failure` is `pub(crate)` — do **not** intra-doc-link it, rustdoc fails the build.)

### Tests
- `src/app/editor/loading.rs::tests::a_missing_data_table_is_recorded_as_an_asset_failure` — recorded, message names the table, **no empty table left behind**.
- `src/app/editor/loading.rs::tests::an_intentionally_empty_data_table_is_not_a_failure` — a valid `[]` table registers with 0 rows and is not a failure (EW-007's own acceptance criterion).

### Examples & scripts
- `examples/packaged_assets.rs` — loads a real data table (`examples/games/stat_editor_game/items.ron`, 2 rows) and a missing one alongside the textures; HUD shows the row count; **headless mode is now a 3-part acceptance assertion** that exits non-zero if the missing table goes unreported.
- `scripts/packaged_assets_smoke.sh` (**new**) — builds the example and runs it **from `/`**; asserts the `OK:` line and a non-blank PNG.

### Release paperwork
- `Cargo.toml` / `Cargo.lock` — 0.126.0 → **0.127.0**.
- `docs/CHANGELOG.md` — the 0.127.0 entry (Changed / Added).
- `CLAUDE.md` — header v1.6.219 → **v1.6.220**; the `asset_path` module-map row now records that *every* path-based loader reports, names the two deliberate non-failures, and points at the smoke script + the process-global test rules.

### Other repo
- `../dungeon-merchant/docs/engine-wishlist.md` — `[Engine]` replies on EW-007 and EW-008; both Status fields → `Shipped`. (Landed as game PR #27.)

---

## User Feedback & Preferences (REQUIRED)

- **"마지막 핸드오프 확인하고 작업 알려줘"** — the session's opening ask. The expectation is a *report first*: read the handoff, read the board, and come back with what the work should be — not to start coding.
- **"진행해"** — approval to implement the recommended plan (EW-007 + answer both board items). Terse approval is normal; it does not invite a re-plan.
- **"하고 머지해줘"** (as the `/handoff` argument) — write the handoff and land it. Consistent with the standing merge delegation.
- **Standing: merge authority is delegated** — squash on green CI, no per-PR re-confirm. Honored for engine #361 (auto-merge) and game #27 (manual squash, no CI there). Still worth asking before anything *outside* "merge my own work" (branch protection, someone else's PR) — the seq-1 precedent.
- **Standing: user-facing reports in Korean; code, docs, commit messages, PR bodies, and handoffs in English.** Followed throughout.
- **Standing: the board takes priority over self-picked work.** This session never had to invoke the "shelf exhausted → ASK" rule because the board had filed work — which is the intended outcome of reading the board first.
- **Process note the user did not have to give:** they asked for the handoff *and the merge* in one breath, i.e. don't stop and re-confirm between the two.

---

## Where We're Going

1. **Land this handoff** as its own `docs(handoff)` PR (`docs/handoff-seq2-loud-loader-failures`), per `/land-pr` handoff mode. No version bump. Everything else from this session is already merged (engine #361, game #27).
2. **The ball is with the game.** dungeon-merchant needs to bump its pin (**v0.116.0 `c42a890` → v0.127.0**) and verify both EW-007 and EW-008, then set `[x]`. Expect the bump to be additive for their surface, but 0.116 → 0.127 spans 11 minor releases — `docs/CHANGELOG.md` is the migration guide. **They can delete `data_path()` and the `is_file()` sentinel probe.**
3. **Then the shelf is empty again**, and the seq-1 directive stands: **read both request channels first** — `../dungeon-merchant/docs/engine-wishlist.md` (next free ID **EW-009**) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (currently `_None._`). If both are empty, **ASK** for a direction; there is no pre-baked self-pick queue.
4. **Standing candidates, none urgent** (unchanged from seq 1, plus one new):
   - **Phase 3 — `App::load_image_bytes(key, bytes)`** so `Sprite::textured(key)` resolves an embedded image exactly like a path (`include_bytes!` single-file builds for small/jam games). The audio half already exists. **Read the parent's Open Questions before designing** — it touches the identity/cache-key machinery this chain has twice been careful not to disturb.
   - **The vorbis/`.ogg` test gap** — nothing in the engine exercises ogg decoding after the rodio 0.22 symphonia swap. Needs a small, licence-clean fixture.
   - **NEW: hot-reload under an asset root** — the watchers still register the caller's path, so F2 hot-reload is only guaranteed for dev-from-repo-root. Disclosed to the game on EW-008; do it if they (or anyone) hit it.

---

## Risks & Blockers

- **`asset_failures()` / the asset root are process-global**, and that is a live footgun for **tests**, not for games. Rules, now twice-learned: never assert on the list's **length**; never call `set_asset_root()` in a unit test (use `resolve_in(roots, path)`); `tests/asset_root.rs` may move the cwd **only** because it is the sole test in its own integration binary — **do not add a second test to that file**.
- **Strict mode + hot-reload would be a bad pairing** if a future session routes reload failures through `record_failure`: a half-saved RON file mid-edit would panic the game. If that ever seems desirable, gate it (`set_strict_assets` should probably not apply to reloads at all).
- **`dungeon-merchant` has no CI and no branch protection** (`gh api …/protection` → 403, private repo without Pro). Its PR convention is discipline only — a board PR merges with zero checks. Don't read "CLEAN" there as "verified".
- **`rust-survivors` has auto-merge DISABLED** (unlike the engine repo) — merge its PRs by hand after watching checks. (Carried from seq 1; unchanged.)
- **Vorbis/`.ogg` decoding is still untested anywhere.** The rodio 0.22 codec swap was verified for mp3 + wav only. If a game reports broken `.ogg`, start there.
- The **two audio tests that need a real device** remain the usual suspects on a no-audio box (they pass here and on CI).

## Open Questions

- **Should `App::load_*` eventually return a `Result` in a 0.x-licensed break?** Rejected now for good reason (`#[must_use]` churn at every call site), but if the engine ever does a deliberate error-handling pass, this is the natural place — and `asset_failures()` would become the *fallback*, not the primary. Not worth doing for its own sake.
- **Should the hot-reload watchers resolve their paths?** Currently they register the caller's string, so a packaged build that hot-reloads would watch the wrong path. Nobody has hit it (hot-reload is a dev-from-repo-root activity by nature). Left open, and disclosed to the game rather than silently claimed.
- **Should dungeon-merchant's EW board absorb `rust-survivors`'s request doc as one queue?** Two channels now exist and both have produced real work within a week. Only matters if the user files from that repo again.

---

## Quick Start for Next Session

```bash
# Nothing is dangling — engine #361 and game #27 are merged. Only this handoff needs landing.

# 1. Engine state
cd ~/Projects/skeleton-engine
git log --oneline -3     # expect bdbbb04 (v0.127.0) at or near the tip
git status -s            # expect clean

# 2. READ THE BOARD FIRST — both channels
#   ../dungeon-merchant/docs/engine-wishlist.md          (next free ID EW-009; EW-007/008 awaiting the game's verify)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md     (currently _None._)
#   If both are empty, ASK the user for a direction. There is no self-pick shelf left.

# 3. Read first (if touching assets again)
#   src/asset_path.rs                  — resolve + record_failure + the process-global caveats
#   src/app/editor/loading.rs          — the 7 wrappers + the 2 loader tests
#   examples/packaged_assets.rs        — the acceptance test (real + missing texture AND data table)
#   plans/handoffs/HANDOFF_asset-root-windows_downstream-bug-report_2026-07-14.md  — seq 1 (the cache-key constraint)

# 4. Verify current state (read the exit code — do NOT pipe or `;`-chain it)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# 5. Re-prove the headline fix
scripts/packaged_assets_smoke.sh
# expect: OK: texture + data table (2 rows) resolved; …
#         PASS: relative assets resolved from /, and the missing data table was reported

# 6. Next action
#   Land this handoff (docs(handoff) PR, no version bump), then read the board.
```
