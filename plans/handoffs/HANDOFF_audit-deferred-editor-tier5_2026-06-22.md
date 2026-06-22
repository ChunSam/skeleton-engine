# HANDOFF — Audit deferred items 3/4/8 (editor splits + theme + tier-5) + item-5 rejection

**Chain:** standalone-4365aa4a
**Seq:** 3 (continuation)
**Parent:** plans/handoffs/HANDOFF_audit-followup-refactors_2026-06-22.md (seq 2)
**Grandparent:** plans/handoffs/HANDOFF_engine-audit-fixes_2026-06-22.md (seq 1, the audit pass #184)
**Date:** 2026-06-22
**Branch:** main
**Status:** COMPLETE + all merged. main @ `f82bb0a`, package **v0.49.4**, clean working tree, full gate green on every PR.
**Auto:** false

---

## Goal

The user said "마지막 핸드오프 확인하고 남은 작업 이어서 진행" (check the last handoff and continue the remaining work). The seq-2 handoff's **STILL-OPEN deferred list** (from the seq-1 engine-wide audit) had items 3–8. This session worked them in priority order, landing each as its own PR through the `/land-pr` loop (merge authority is standing-delegated; squash-on-green-CI).

This session **completed items 3, 4, 8**, and **investigated + rejected item 5**. Items 6 and 7 remain (both bigger/riskier — see "Where We're Going").

---

## Since Last Handoff (seq-2 plan vs what happened)

The seq-2 handoff listed deferred items 3–8 and flagged the open question *"How far down items 3–8 does the user want to go?"* This session answered it interactively (4 `AskUserQuestion` checkpoints) and executed:

| Item | seq-2 status | This session |
|---|---|---|
| 3. split god-files `docked.rs`(1233) + `gizmo.rs`(1183) | open | **DONE** — #188 (docked) + #189 (gizmo) |
| 4. central editor theming constants | open | **DONE** — #190 |
| 5. `AudioSurface` trait | open | **INVESTIGATED → REJECTED** (not viable; needs a facade feature) |
| 6. `texture.rs` format param | open | **STILL OPEN** (breaking → feature+example) |
| 7. `world.rs` 56 `unwrap`s | open | **STILL OPEN** (high blast radius) |
| 8. tier-5 small cleanups | open | **DONE** — #191 |

Net version path: v0.49.0 (seq-2 end) → **v0.49.1** (#188) → **v0.49.2** (#189) → **v0.49.3** (#190) → **v0.49.4** (#191).

---

## Where We Are

- main @ `f82bb0a`, **v0.49.4**, clean tree, `./scripts/verify.sh` → exit 0 (fmt + clippy `-D warnings` native & wasm + wasm32 lib/bins build + `test --all-targets` + doc `-D warnings`).
- Memory `engine-current-state` bumped to **seq 66**; `MEMORY.md` index line refreshed; oldest standalone seqs (56→folded earlier; 57, 58, 59 condensed into the "Older seqs" line this session) trimmed to stay compact.
- 4 PRs merged: **#188, #189, #190, #191** — all behavior-preserving, **no public API change**, all PATCH.

---

## What We Did

### PR #188 — split `docked.rs` (v0.49.1, item 3a)

`src/app/editor/ui/docked.rs` 1233 → 958 lines. Extracted 3 self-contained native-only concerns following the existing `audio_panel.rs` / `data_table_panel.rs` pattern:

- **`particle_panel.rs`** (new) ← `particle_tuner_grid` + its private `color_rgba_drags` helper.
- **`lighting_panel.rs`** (new) ← `point_light_grid` + `ambient_light_control` + their private `color_rgb_drags` helper.
- **`grid_overlay.rs`** (new) ← `draw_editor_grid` + `grid_lines_in_range` + its grid-lines unit test.

`uv_rect_to_egui` + the Tile Paint swatch palette **stay** in `docked.rs` (the palette is `uv_rect_to_egui`'s caller). `point_light_grid` / `particle_tuner_grid` are re-exported from their new homes via `ui/mod.rs` (they're called from `component_registry.rs` as `ui::*`); `ambient_light_control` / `draw_editor_grid` are imported directly by `docked.rs` (called only there). Pure code movement — only visibility/imports/paths changed; moved fns + tests verbatim.

### PR #189 — extract `gizmo_math.rs` (v0.49.2, item 3b)

`src/app/editor/ui/gizmo.rs` 1183 → 707 lines. The side-effect-free anchor/resize/rotation math + gizmo size/snap constants moved into a new **`gizmo_math.rs`**, leaving `gizmo.rs` to hold only the `impl App` input-handling + rendering interaction logic.

- **Moved (verbatim):** `anchor_base`, `ui_drag_new_offset`, `handle_centers`, `hit_test_handles`, `ui_resize_new_layout`, `rotation_handle_pos`, `cursor_angle`, `snap_angle`, `applied_rotation`, and the `MIN_UI_SIZE` / `MIN_SPRITE_SCALE` / `HANDLE_SIZE` / `HANDLE_HIT_RADIUS` / `ROT_HANDLE_GAP` / `ROT_HIT_RADIUS` / `ROT_SNAP` constants.
- **10 pure-math unit tests moved** with them; the **1 App-level test** (`rotation_gizmo_drag_rotates_and_undoes`, which drives `update_transform_gizmo_native`) **stays** in `gizmo.rs`.
- **Visibility:** private items the interaction logic still calls became `pub(super)` (the consts + `rotation_handle_pos`/`cursor_angle`/`applied_rotation`); already-`pub(crate)` fns kept; `snap_angle` + `HANDLE_HIT_RADIUS` stayed private (internal to `gizmo_math`).
- **Per-item `#[cfg(not(target_arch = "wasm32"))]` gating preserved** (no module-level cfg — `gizmo.rs` compiles on wasm with native bits gated out); the cross-module `use super::gizmo_math::{…}` import in `gizmo.rs` is itself cfg-gated.

### PR #190 — central editor theming constants (v0.49.3, item 4)

New **`src/app/editor/theme.rs`** collects the editor chrome's inline visual magic numbers: gizmo overlay colors + z-biases (999/1000), grid-overlay line width/alpha + cursor-readout alpha/font size, the central viewport frame fill, and the 3 docked-panel default/min/max sizes. **Every constant == the literal it replaced.** Call sites in `gizmo.rs`, `grid_overlay.rs`, `docked.rs` now reference `theme::*`.

- **`mod theme;`** declared in `src/app/editor.rs` (un-gated — the module is cross-platform).
- **Gating mirrors the call sites:** `GIZMO_SELECT_COLOR` + `GIZMO_SELECT_Z_BIAS` stay cross-platform (the screen-space UI-node selection highlight in `update_transform_gizmo` / `update_ui_node_gizmo` compiles, dead, on wasm via `#[cfg_attr(wasm, allow(dead_code))]`); all other consts are `#[cfg(not(target_arch = "wasm32"))]`.
- Consts are `pub(in crate::app::editor)`.

### PR #191 — tier-5 cleanups (v0.49.4, item 8)

- **`src/pathfinding.rs`** — the identical "walk `came_from` back + reverse" tail of `find_path` and `find_path_diagonal` → shared private **`reconstruct_path(came_from, goal) -> Vec<IVec2>`** helper (was duplicated verbatim). All 16 pathfinding tests unchanged + green.
- **`src/collision/grid.rs`** — `SpatialGrid::candidates_in_aabb` gains a **single-cell fast path**: when `col_min == col_max && row_min == row_max` it returns `self.buckets.get(&(col,row)).cloned().unwrap_or_default()` and skips the per-query dedup `HashSet`, since an entity appears at most once in any one bucket (`rebuild` pushes each entity once per overlapping cell). Multi-cell queries are unchanged (still deduped). New regression test `candidates_dedup_across_cells_and_single_cell_fast_path` covers both paths.

### Item 5 (`AudioSurface` trait) — INVESTIGATED, REJECTED (no code change)

The user selected item 5; investigation showed it is **not viable as the handoff specified**:

1. **The engine has NO cross-platform audio call sites.** The shared spatial math is already DRY'd into `src/audio_spatial.rs` (`spatial_params`); `lib.rs` just re-exports the two types cfg-gated; the editor's `audio_panel.rs` is native-only.
2. **The real cfg-guard duplication is in GAME code** — each game that ships to web writes a native `AudioManager` fn + a wasm no-op stub (e.g. `examples/games/survivor/survivor.rs:977` — `play_sfx` native + a `#[cfg(target_arch = "wasm32")]` no-op). A trait alone would not remove this.
3. **The two backends are too divergent for a shared trait** — `AudioManager` is channel-based + `&mut self` (rodio sinks); `WebAudio` is source/master-based + `&self` (Web Audio nodes via interior mutability). `Sfx` is a different concrete type per platform. Of the same-named methods, only `bus_volume` / `bus_names` / `bus_duck` have **identical signatures** (read-only, low value); `play` / `set_volume` / `stop` / `set_pan` / `play_at` / `update_position` all differ (`&mut` vs `&`, channel vs master, per-platform `Sfx`).

**Conclusion:** cutting the dup requires a cross-platform audio **facade** (a unified API so games write one path), which is a **feature+example loop**, not a behavior-preserving refactor. Punted to a future feature pass. Recorded in memory so a future session doesn't re-investigate.

---

## Key Decisions & Rejected Alternatives

- **Module-map (CLAUDE.md) NOT changed for the editor splits.** The editor row points at the directory `src/app/editor/`, and prior panel extractions (`audio_panel`, `data_table_panel`, `state_machine_panel`, `timeline_panel`) are not individually named there. New sub-files are covered by the directory pointer; the described features are unchanged. Consistent with repo convention.
- **`theme.rs` scope deviated from the handoff's file list.** The handoff named `slider.rs`/`checkbox.rs`, but those are `src/ui/slider.rs` / `src/ui/checkbox.rs` — **game-facing reusable widgets** (`SliderStyle`/`CheckBoxStyle` `Default` fields), NOT editor chrome. Pulling them into an editor-internal module would invert the dependency (a public widget depending on the editor). Left alone; noted in the changelog + PR.
- **`from_white_alpha` colors kept at the call site.** `egui::Color32::from_white_alpha` is **not `const`** and does a *linear* conversion — `from_white_alpha(26) ≠ from_rgba_premultiplied(26,…)`. So only the `u8` alpha was extracted (`GRID_LINE_ALPHA = 26`, `CURSOR_READOUT_ALPHA = 190`); the call stays. `Color32::from_rgb` and `Color::rgba` **are** const → full const values for the viewport fill + gizmo colors.
- **SpatialGrid scratch-buffer (the audit's literal suggestion) REJECTED** for a single-cell fast path instead. A scratch buffer reused across queries needs `&mut self` or interior mutability, but the query API (`query_radius`/`query_aabb`/`candidates_in_aabb`) is `&self` — changing it ripples to every collision-query caller. The single-cell fast path is `&self`-preserving, behavior-identical, and removes the `HashSet` alloc in the common case (small colliders, AABB inside one cell).
- **One PR per coherent change** (4 sequential PRs, not bundled). Each touches a different subsystem and bumps the version (must be sequential to avoid lock/version conflicts). Honors the "one PR = one coherent change" guardrail.
- **All four PRs are PATCH** — internal refactors, no public API change (all moved/new items are `pub(crate)` / `pub(super)` / `pub(in crate::app[::editor])`).

---

## Gotchas Hit (reusable)

- **`use super::*` in a child `mod tests` brings in the PARENT module's PRIVATE `use` aliases + private items.** When the gizmo geometry tests moved to `gizmo_math.rs`, their `use super::*` correctly resolved `ResizeHandle` / `Anchor` (private `use` in `gizmo_math`) and `snap_angle` (private fn) — a child module can see its parent's private items, and the glob imports them. This is why the moved tests compiled without explicit imports.
- **Removing moved code leaves unused imports that clippy `-D warnings` flags.** After the gizmo split, `gizmo.rs` had a now-unused `use crate::ui::Anchor;` and the test module's `use super::*` became unused (the one kept App test uses only fully-qualified paths). Both removed. Run `cargo check --all-targets` (compiles tests) after a move, not just `--lib`.
- **wasm gating must mirror the actual call-site cfg.** For `theme.rs`, the gizmo *selection highlight* (`GIZMO_SELECT_COLOR` + `GIZMO_SELECT_Z_BIAS`) is drawn in `update_transform_gizmo` / `update_ui_node_gizmo`, which are NOT `#[cfg(not(wasm))]` — they're `#[cfg_attr(wasm, allow(dead_code))]` (compiled dead on wasm). So those two consts must be cross-platform; gating them native-only gave `E0425: cannot find value GIZMO_SELECT_Z_BIAS` on the wasm build. The resize/rotation *handles* ARE inside `#[cfg(not(wasm))]` blocks, so their consts stay native.
- **`egui::Color32::from_white_alpha` is non-const + linear** (see Key Decisions). Don't "simplify" it to a const `from_rgba_premultiplied` — that changes the color.
- **Indentation-sensitive `replace_all`.** `tr.z + 1000.0` appeared twice with *different* leading whitespace; `replace_all` on the 24-space form only hit one. Always re-grep for the literal after a `replace_all` to catch indentation variants.
- **rust-analyzer E0255/E0432 diagnostics lag during a large move** (they reference pre-edit line numbers). Trust `cargo check` / the gate, not the live phantoms (a standing repo gotcha).
- **Line-range deletion via a one-off `python3` heredoc** is a clean, deterministic way to drop a large verbatim-moved block (used to cut the 235-line helper section + the pure tests out of `gizmo.rs`). The CLAUDE.md "avoid sed" rule is about *reading*; a surgical line delete is fine. Always re-grep + `cargo fmt` + gate afterward.

---

## Evidence

- `./scripts/verify.sh` → exit 0 on every PR branch, before AND after each version bump; CI (Build WASM / Package dry-run / Rustdoc / Test native) green on #188/#189/#190/#191; all merged `mergeStateStatus: CLEAN`, squash + branch-deleted.
- Line counts: `docked.rs` 1233→958 (−275); `gizmo.rs` 1183→707 (−476). New files: `particle_panel.rs` ~114, `lighting_panel.rs` ~84, `grid_overlay.rs` ~113, `gizmo_math.rs` ~505, `theme.rs` ~78.
- Diffstats (merge commits): #188 = 9 files +338/−282; #189 = 7 files +524/−486; #190 = 9 files +117/−21; #191 = 6 files +74/−21.
- Tests: `gizmo_math::tests::*` (10) + `gizmo::tests::rotation_gizmo_drag_rotates_and_undoes` green; `pathfinding` 16 green; `collision::grid::tests::candidates_dedup_across_cells_and_single_cell_fast_path` (new) green.
- Audio investigation evidence: `audio_spatial.rs` (shared math, no call-site dup); method-surface grep showing only `bus_volume`/`bus_names`/`bus_duck` match exactly; `survivor.rs:977-986` native-fn + wasm-no-op pattern.

---

## Where We're Going — STILL-OPEN deferred items

From the seq-1 audit (parent-of-parent `HANDOFF_engine-audit-fixes_2026-06-22.md`). Items 1–4 + 8 done; item 5 rejected; remaining:

6. **`renderer/texture.rs:130` `Rgba8UnormSrgb` hardcoded** → parameterize the format for HDR / linear workflows. **Breaking** to `from_rgba` — the clean shape is `from_rgba_with_format(…, format)` + keep `from_rgba` as the srgb wrapper. Best as a **feature+example** task (`/add-feature-example`): exercise a linear/HDR render target in an example as the acceptance test.
7. **`ecs/world.rs` 56 real `unwrap`s** — dedicated invariant-hardening review. **High blast radius** (`[profile.release] panic="abort"` ⇒ any live `unwrap` aborts the whole game). Left untouched; needs careful per-unwrap reasoning + tests, not a sweep.

**Item 5 reframed:** a cross-platform **audio facade** (so games write one audio path instead of native-fn + wasm-no-op-stub) is a real feature — if pursued, `/add-feature-example` with a game that plays audio on both native and web.

**Possible bonus (carried from seq-2):** make `RonRegistry<V>` + `RonLoadable` `pub` (crate root) so forks can register their own RON-loaded asset types.

---

## Open Questions

- Items 6 and 7 are each big enough to warrant a dedicated session (item 6 = a breaking feature+example; item 7 = a careful high-risk hardening pass). Which (if either) does the user want next, and in which order? Item 6 is more contained; item 7 is the riskiest remaining audit item.
- The dungeon-merchant wishlist board is EMPTY (next ID EW-002) — check it first each session before picking backlog.

---

## Resume

1. Sanity: `git checkout main && git pull --ff-only`; `./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?` should be 0. main should be at the handoff-doc commit (this doc lands via its own `docs(handoff)` PR, like seq-2's #187).
2. Pick the next item (6 or 7, or a wishlist EW request if one appears). Branch `<type>/<slug>` off main (never commit to main directly).
3. Item 6 → `/add-feature-example` (it's a breaking API + needs an example as the acceptance test). Item 7 → a focused review branch; reason about each `unwrap`'s invariant, add a guard + test only where the invariant can actually break; do NOT blanket-replace.
4. For each change: implement → `cargo fmt` → full gate (capture `$?`, never pipe) → `/ship` (PATCH internal / MINOR additive-or-breaking, pre-1.0) → commit → push → `gh pr create` → `gh pr checks <n> --watch --fail-fast > log 2>&1` → squash-merge on CLEAN → `git pull --ff-only` → bump `engine-current-state` memory seq.

---

## Pointers

- Parent (items 1+2, the prior two deferred refactors): `plans/handoffs/HANDOFF_audit-followup-refactors_2026-06-22.md`.
- Grandparent (the full audit + all 8 deferred items, with the `panic="abort"` weighting lens + WindowConfig-70-call-sites constraint): `plans/handoffs/HANDOFF_engine-audit-fixes_2026-06-22.md`.
- Module map / verify rules / pre-1.0 versioning: `CLAUDE.md`. Patterns: `docs/PATTERNS.md`.
- New modules this session: `src/app/editor/ui/{particle_panel,lighting_panel,grid_overlay,gizmo_math}.rs`, `src/app/editor/theme.rs`.
- Live engine state + gotchas: memory `engine-current-state` (seq 66).

---

## Appendix A — editor `ui/` module after this session

```text
src/app/editor/
├── editor.rs            (mod root: + `mod theme;` un-gated)
├── theme.rs             NEW — editor chrome visual constants (pub(in crate::app::editor))
└── ui/
    ├── mod.rs           (+ mod particle_panel/lighting_panel/grid_overlay/gizmo_math; re-exports)
    ├── docked.rs        1233→958 (sheds 3 panels; keeps uv_rect_to_egui + swatch palette)
    ├── gizmo.rs         1183→707 (impl App interaction logic + 1 App test)
    ├── gizmo_math.rs    NEW — pure anchor/resize/rotation math + consts + 10 tests
    ├── particle_panel.rs   NEW — particle_tuner_grid + color_rgba_drags
    ├── lighting_panel.rs   NEW — point_light_grid + ambient_light_control + color_rgb_drags
    ├── grid_overlay.rs     NEW — draw_editor_grid + grid_lines_in_range + test
    ├── audio_panel.rs / data_table_panel.rs / state_machine_panel.rs / timeline_panel.rs (pre-existing pattern)
    └── tile_paint.rs / shortcuts.rs
```

## Appendix B — audio backend method-surface divergence (why item 5's trait fails)

| method | AudioManager (native, rodio) | WebAudio (wasm, Web Audio) | shared-trait viable? |
|---|---|---|---|
| `bus_volume` | `(&self, bus) -> f32` | `(&self, bus) -> f32` | ✅ identical |
| `bus_names` | `(&self) -> Vec<String>` | `(&self) -> Vec<String>` | ✅ identical |
| `bus_duck` | `(&self, bus) -> f32` | `(&self, bus) -> f32` | ✅ identical |
| `set_bus_volume` | `(&mut self, bus, vol)` | `(&self, bus, v)` | ❌ `&mut` vs `&` |
| `set_volume` | `(&mut self, channel, vol)` | `(&self, v)` (master) | ❌ channel vs master |
| `play` | `(&mut self, channel, path, repeat)` | `(&self, bytes)` | ❌ totally different model |
| `stop` | `(&mut self, channel)` | `(&self)` | ❌ |
| `play_at` | `(…) -> Sfx` (native Sfx) | `(…) -> Sfx` (wasm Sfx) | ❌ per-platform `Sfx` |

Only 3 read-only methods match → no useful trait. The fix is a facade (a feature), not a trait.

## Appendix C — concrete anchors for the remaining items 6 & 7

**Item 6 — texture format parameterization** (`src/renderer/texture.rs`):
- `fn from_rgba(…)` is at **line 108**; it hardcodes **`format: wgpu::TextureFormat::Rgba8UnormSrgb`** at **line 130**.
- Clean shape: add `from_rgba_with_format(width, height, bytes, format: wgpu::TextureFormat)` carrying the real body, and make `from_rgba(…)` a thin wrapper that passes `Rgba8UnormSrgb` (so existing call sites are untouched — keeps the change additive at the call sites even though the new fn is the "breaking" expansion).
- Acceptance test (VISION loop): an example that renders to / samples a **linear or HDR** target via the new format param. `/add-feature-example`.
- Watch the Rust↔WGSL drift guards from seq 60 — a format change can affect sampling assumptions.

**Item 7 — `ecs/world.rs` unwrap hardening** (`src/ecs/world.rs`):
- Current raw `.unwrap()` count ≈ **51** (audit counted 56 "real" ones; the rest are test/doc). `[profile.release] panic="abort"` ⇒ each is a hard game-abort, no unwind.
- Approach: **reason per-unwrap**, do NOT blanket-replace. For each, ask "can this invariant actually break at runtime?" Only the ones reachable with bad/edge input need a guard (`if let` / `?` / `expect` with a real invariant message + a regression test). Many are genuinely infallible (just-inserted key, generation match) and are fine as-is or want an `expect("<invariant>")` for documentation.
- High blast radius → a focused review branch, small batches, gate after each batch.

## Appendix D — session decision cadence (how this session was steered)

The user's single directive ("continue the remaining work") was resolved interactively via **4 `AskUserQuestion` checkpoints**, each at a natural seam:
1. After item 3 (god-file split) landed → user picked **item 4** (theme constants).
2. After item 4 landed → user picked **item 5** (AudioSurface).
3. After item 5 was investigated and found non-viable (reported the finding) → user picked **skip 5, do item 8** (tier-5).
4. After item 8 landed → (this handoff). Items 6/7 deferred to dedicated sessions.

Takeaway for the next session: the user drives item selection at each seam; surface findings honestly (the item-5 rejection was accepted, not overridden) and let them choose. Merge authority stays delegated (squash-on-green-CI, no per-PR re-confirm). Also did **memory hygiene** this session: folded seqs 56–59 into the `engine-current-state` "Older seqs" condensed line to keep the file compact while adding seqs 63–66.

---

## Session Closed
**Closed at:** 2026-06-22
**Session status:** Handed off to next session.
**Landed:** this handoff doc lands on `main` via its own `docs(handoff)` PR (the seq-2 doc did the same — #187). Code work for the session = the 4 already-merged PRs #188–#191 (v0.49.1 → v0.49.4); engine state is at `f82bb0a` + this doc's PR.
