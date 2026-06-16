# Pluggable inspector panels — v9.5.0 (PR #69)

**Date:** 2026-06-16
**Status:** COMPLETED
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine — engine hardening / architecture
**Chain:** `engine-hardening` seq `7`
**Parent:** `HANDOFF_engine-hardening_module-moves-v9.4.1_2026-06-16.md` (seq 6)
**Prior chain:** seq 3 (priority loop) > 4 (cohesion review) > 5 (v9.4.0) > 6 (v9.4.1 reorg) > this (7)

---

## Since Last Handoff

Seq 6 shipped v9.4.1 (cohesion item-3 module moves) and concluded "safe autonomous subset
exhausted, remaining needs sign-off." The user re-ran `/loop` ("재실행"). On re-examination I
found that **part of cohesion item 7 (new extension points) is ADDITIVE, not breaking** — adding a
hook is non-breaking. So I shipped the clear one: **`register_inspector_panel`** as **PR #69 /
v9.5.0** (no sign-off needed — additive). Corrects seq 6's over-broad "exhausted" claim.

## Reference Documents

- `docs/MODULE_COHESION_REVIEW_2026-06-16.md` — the review; item 7 = new extension points.
- `CLAUDE.md` (module map) · `docs/VISION.md`.

## The Goal

Turn the cohesion review's "no inspector-panel registration → docked.rs hardcodes component
checks" finding into an additive fork-friendly extension point, without any breaking change.

## Where We Are

- **`main` = `79ba57f` (PR #69), v9.5.0, CLAUDE.md header v1.6.39, clean tree.** 725 lib tests (721 → +4). CI 4/4 green; full local Gate6 green (independently re-verified — see Risks).
- **Shipped:** `App::register_inspector_panel::<T>(title, draw: Fn(&mut egui::Ui, &mut App, Entity))` (native-only). Entry = `(presence: fn(&World, Entity)->bool, title, Box<dyn Fn…>)`, `presence` = monomorphized `World::has_component::<T>`. Stored on `EditorState.inspector_panels`. Dispatch in `docked.rs` uses the **take/restore** borrow pattern (panels live on `app.editor`, draw takes `&mut App`). The 4 uniform built-ins (Particle Tuner / Point Light / State Machine / Timeline) now register through it; **Tile Paint stays hardcoded** (non-uniform: `else` clears `paint_mode` + atlas-dims logic). `docked.rs` −81 lines.
- **Session arc: 9 PRs** — #61(v9.1) → #62(v9.2) → #63(v9.3) → #64(coverage ledger) → #65(REFERENCE) → #66(cohesion review) → #67(v9.4.0) → #68(v9.4.1) → #69(v9.5.0).

## Key Decisions

- **Re-classified item 7 as ADDITIVE.** Seq 6 lumped all of items 5–7 as "breaking→v10." Adding extension HOOKS (`register_inspector_panel`, `RenderPlugin`) is additive; only the field-encapsulation (item 5) and the `App`/`render()` extraction + unit-struct scratch (item 6) are breaking. So `register_inspector_panel` was safe to ship autonomously on the re-loop.
- **Migrated only the 4 uniform panels; left Tile Paint hardcoded.** Tile Paint isn't a uniform `has_component → CollapsingHeader` panel (it has side-effects), so forcing it through the registry wasn't a clean move.
- **take/restore for the stored-closure-takes-&mut-App borrow** — same pattern as `hot_reload_forwarders`/`component_factories`.

## Evidence & Data

- New API: `App::register_inspector_panel` (editor.rs:970) + `InspectorPanel` entry (state.rs:340, `pub(in crate::app)`). 4 built-ins registered in `register_default_components`.
- +4 tests (721 → **725**): registration push, presence-fn correctness, distinct-type non-confusion, take/restore dispatch gating.
- `docked.rs` −81 lines (4 hardcoded blocks → 1 dispatch loop).

## Files Changed (PR #69)

`src/app/editor.rs` (API + built-in registrations), `src/app/editor/state.rs` (`InspectorPanel` + field + tests), `src/app/editor/ui/docked.rs` (dispatch loop; `particle_tuner_grid`/`point_light_grid` → `pub(in crate::app)`), `src/app/editor/ui/mod.rs` (re-exports), `Cargo.toml`/`docs/CHANGELOG.md`/`CLAUDE.md` (version).

## User Feedback & Preferences (REQUIRED)

- **"재실행" (×5+ this session)** — keep executing the backlog autonomously; batch → merge → handoff → report each loop.
- **Standing:** Korean prose / English artifacts; Sonnet subagents w/ explicit `model:`; merge authority for the loop; never tag unprompted; **no breaking changes without explicit sign-off.**

## Where We're Going

Remaining cohesion work, by safety:
1. **Still-additive (autonomous-OK):** `AssetServer` hot-reload **watchlist unification** (collapse the 3 path-sidecars + 3 `watch_*` methods into one, routed through the existing `HotReloadable` forwarders; keep public methods → behavior-identical internal cleanup — clean but low user-value). `RenderPlugin` render-pass hook (additive, BUT needs careful `FrameContext` API design + a call point in the 839-line `render()` — more deliberation than a mechanical add).
2. **Breaking → v10 (needs explicit sign-off):** item 5 (`pub`→`pub(crate)` the leaking wgpu/rapier fields + accessors); item 6 (`RenderState` extraction from `App`, split `render()`/`update()`, `SpriteRenderer`→`MaterialRenderer`+`TextureCache`, split `tilemap.rs`, `UiSystem`/`SteeringSystem` scratch [unit structs]); `ScriptAsset`→rhai decouple (design change); `CameraUniform` dedup (trivial, the two defs differ in field visibility).
3. **Display-session features:** per-locale fonts (#5, spec in seq 3); SM/Timeline visual editors; v9.x git tag (policy).

## Risks & Blockers

- **None blocking.** main green + clean at `79ba57f`.
- **Process (recurring, handled):** the impl agent self-reported "all gates green" while harness diagnostics showed rustc E0603 (private module) + dead_code errors. As in seq 6, these were **mid-edit snapshots** the agent resolved before finishing — I **re-ran full Gate6 independently** → genuinely green. Always re-verify after editor moves; don't trust the agent's claim alone.

## Open Questions

- Keep grinding the still-additive items (AssetServer watchlist / RenderPlugin), or pause the loop and scope a v10 breaking pass / pivot to a feature?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3   # 79ba57f (#69) → 142fcda (seq-6 handoff) → b6106f9 (#68)
grep -m1 '^version' Cargo.toml   # 9.5.0
git status -s          # clean
./scripts/verify.sh    # confirm main green

# Next — pick ONE:
#   (a) still-additive: AssetServer hot-reload watchlist unification (clean internal) — autonomous-OK.
#   (b) RenderPlugin render-pass hook — additive but design the FrameContext carefully first.
#   (c) v10 breaking architecture pass (items 5/6) — needs explicit user sign-off.
#   (d) pivot to a feature (per-locale fonts / new VISION feature).
#   Reference: docs/MODULE_COHESION_REVIEW_2026-06-16.md. No breaking work without sign-off.
```

## Session Closed
**Closed at:** 2026-06-16
**Commit:** the `session: inspector-panel-hook-v9.5 handoff [engine-hardening]` commit on `main`
**Session status:** Handed off — v9.5.0 shipped (9 PRs total this session); remaining additive items getting more involved, rest needs v10 sign-off
