# Vision — skeleton-engine

> Status: living document. This is the "why" behind the engine. Feature work,
> roadmap, and agent behavior should trace back to the goals stated here.
> Last reset: 2026-05-29.

## Why this engine exists

The name is the thesis: **skeleton**. This is meant to be a *bone structure* — a
clean, MIT-licensed 2D engine that other developers can fork and flesh out with
their own muscle. It deliberately favors being hackable and readable over being a
sealed black box.

Three overlapping purposes, in priority order:

1. **An open-source skeleton others can fork and extend.** People should be able to
   take the source, modify engine code directly, and grow it into their own engine.
2. **A personal foundation for my own 2D games.** It is the base layer small 2D games
   can be built on.
3. **A personal learning vehicle.** Building the internals by hand is itself a goal —
   understanding how a game engine works from the ground up.

## What success looks like (scope)

**Genre-agnostic 2D.** Success means a developer can build a complete 2D game in any
common genre — platformer, shooter, RPG, puzzle, top-down action — without hitting a
wall that forces them off the engine. Breadth across genres matters more than depth in
any single one.

## The current problem this reset addresses

The v1.x feature set has been validated by several small playable examples, but each
major API still needs gameplay pressure before it is treated as stable. APIs that look
reasonable in isolation may be awkward in practice. The risk is an engine that is
feature-complete on paper but unpleasant to actually ship a game with.

## Direction: features proven by playable examples

The chosen path combines two things:

- **Keep widening the feature set** (breadth-first, toward genre-agnostic 2D).
- **Validate every feature through small, playable example games.**

These are not separate tracks — they are one loop. The dogfooding vehicle is a growing
set of **small playable example games**, one per genre/feature area, living in `examples/`.

> ⚠️ **The loop is closed again at the dogfooding end, at a fifth of the old surface.** All 22
> playable games and ~85 feature demos were deleted on 2026-08-19 at the maintainer's request, to be
> rebuilt from scratch. **All five are back** as of 2026-08-21: `platformer_game` (phase 1),
> `rpg_quest_game` (phase 2), `survivor_game` (phase 3), `puzzle_grid_game` (phase 4) and
> `netplay_game` + `netplay_server` (phase 5), 35 acceptance checks between them, plus three browser
> smokes under CI (phase 5b) that put Web Audio and the wasm WebSocket path back under gate. Between
> v0.151.0 and the deletion the repo had a playable slice for every genre named above — platformer,
> shooter, RPG (`rpg_quest`), puzzle (`sokoban`), top-down action (`survivor`, `maze_escape`) — plus
> scene-flow and settings/menu demos. **Only the subsystems these five games name meet the
> acceptance bar the principles below state**, and the ~85 demos covered subsystems they do not:
> **gamepads and windowed play have no example driving them at all**, and the editor only sideways,
> through one docked-transition render test. Saying so is more useful than quietly lowering the bar.
> Past games are in git history and `docs/PROGRAM_HISTORY.md` records what each covered.
>
> **The rebuild covers all five genres with four genre-games, not five** — decided 2026-08-19,
> recorded here so the list above does not read as an unclosed gap. `survivor_game` carries
> *both* shooter and top-down action: the two exist to put the engine under **count** pressure
> (pool churn, spatial-grid rebuilds, steering for ~200 agents, the 16-light cap), and that is one
> world, not two. So the mapping that changes is *one game per genre*, not the genre list itself —
> nothing named above is dropped. The full proposal, its four settled decisions and its phasing are
> in `plans/2026-08-19-examples-rebuild-plan.md`; splitting the two back apart is a later edit
> there, not a redesign.

### Operating principles

- **A new feature is not "done" until a small example game exercises it in real play.**
  The example is the acceptance test, not an afterthought.
- **If the API feels awkward while writing the example, fix the API before release.**
  The example exists precisely to surface tabletop-theory mistakes.
- **Add features in a fork-friendly shape.** Even while prioritizing breadth, new code
  should keep clear module boundaries and extension points, so the "skeleton" stays
  forkable. Breadth first, but not at the cost of leaving an unreadable mess.
- **Post-v1.0.0 honors semver.** Breaking changes wait for v2.0.0 (see
  `docs/ENTITY_GENERATION_V2_PLAN.md`).

Concrete next-work candidates and how the previously "planned" items map to this vision
live in `docs/NEXT_WORK.md`.

## Non-goals (for now)

- 3D, or competing with Unity/Unreal/Godot on feature surface.
- A visual editor as the primary authoring path (the egui debug/inspector overlay is a
  tool, not the product).
- Locking into a single genre or a single game.
