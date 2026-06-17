# P1 shipped — data-driven dialogue (RON trees + conditional choices + event hooks, v0.20.0)

**Date:** 2026-06-18
**Status:** **COMPLETED — shipped + merged, CI-green.** `main` @ `c568f70`, package **v0.20.0**, clean tree.
PR **#107** squash-merged + branch deleted. 801 → **841 lib tests** (+40).
**Chain:** `engine-hardening` seq `22` · **Parent:** `HANDOFF_engine-hardening_dialogue-depth_2026-06-17.md` (seq 21)
**Roadmap:** P1 of `plans/handoffs/PLAN_engine-hardening_p1p2p3-roadmap_2026-06-18.md` (P1✅ · P2 next · P3 after).
**Goal directive (active):** execute the roadmap P1→P2→P3; each stage = test + merge + handoff, then continue; stop after P3. Merge authority granted by the goal itself ("테스트, 머지 완료하고").

---

## What shipped (P1 = sub-tasks 1a + 1b + 1c + 1e; 1d deferred)

- **1a — RON dialogue-tree loader** (`src/dialogue/tree.rs`): `DialogueTree` (ordered named nodes;
  each node `id` + line [literal `line` or `line_key`] + `goto`-by-id choices) flattens to a
  `DialogueBox` at parse time (node order = line index, every `goto` id → line index), so the
  existing `DialogueSystem` runtime is reused unchanged. `DialogueRegistry` (World resource) +
  `App::load_dialogue(name, path)` (`src/app/editor.rs`) load + hot-reload, an exact mirror of
  `load_animation_clips`/`load_particle_configs` (lazy-insert registry → `register_persistent`
  → `reg.load` → `assets.watch_path`; `HotReloadable` impl; auto-registered in `App::new`).
  `from_ron_str` validates duplicate ids, unknown gotos, literal-vs-localized consistency.
- **1b — conditional choices** (`src/dialogue/vars.rs`): `DialogueVars` (World resource of
  `DialogueValue` = `Bool/Int/Float/Str`); `DialogueChoice.cond: Option<DialogueCond>` (compare a
  var via `DialogueOp` `Eq/Ne/Gt/Lt/Ge/Le`). Unset var → zero-value of the operand's type, so
  `flag Eq Bool(false)` is true when unset. Ordered ops on non-numeric → false.
- **1c — choice→event/effect hooks**: `DialogueChoice.effect: Option<DialogueEffect>` =
  `SetVar{key,value}` | `EmitEvent{name}`. World-level **free functions** `dialogue::advance(world,e)`
  and `dialogue::choose(world,e,visible_i)` clone `DialogueVars`, drive the box vars-aware, and
  apply effects (write `DialogueVars` / send `DialogueEvent` to `Events<DialogueEvent>`).
- **1e — example `dialogue_quest`** (`examples/dialogue_quest.rs` + `examples/dialogue_quest.dlg.ron`):
  loads the tree via `App::load_dialogue`, "buy the lantern" carries an `EmitEvent` effect → the
  game's `QuestEvents` system grants the lantern (`set_bool("has_lantern", true)`) → the gated
  "secret" choice appears. Live EN↔KO (`L`). The VISION acceptance test; **playtested windowed,
  all 4 goals confirmed** (see Evidence).

## Key API decisions (the "choose re-wiring" the plan flagged)

- `pending_choices()`/`choose(i)`/`advance()` stay vars-agnostic (back-compat, no vars/world
  needed). The **vars-aware path** is new: `DialogueBox::visible_choices(&vars)` (cond-passing
  pending choices), `is_choosing(&vars)`, `advance_with(&vars)` (blocks only while a *visible*
  choice pends — a line whose every choice is gated out advances normally), and
  `choose_visible(i, &vars) -> Option<DialogueEffect>` (jumps, returns effect for the caller).
- The world-level `dialogue::advance`/`choose` free functions are the game-facing API: they own
  the vars clone + effect application. `DialogueSystem` renders only `visible_choices` (clones
  `DialogueVars` once before the query, like the locale clone). The example reads cleanly — no
  API change was needed mid-1e, so the plan's "API awkwardness" risk did **not** materialize.
- `DialogueVars` uses a dedicated serde-friendly `DialogueValue`, **not** the engine's
  `Blackboard` (the plan suggested reuse): `BlackboardValue` derives only `Debug, Clone` (not
  `Serialize`/`Deserialize`), so it can't round-trip RON cond/effect. A self-contained value
  enum was cleaner. Documented deviation.
- RON authors optional `cond`/`effect` **inline** via `ron::Options` with `IMPLICIT_SOME`
  (avoids the verbose `Some(...)` wrapper); optional *string* fields use `String` + `#[serde(default)]`
  (empty = none) so bare `line_key: "dlg.intro"` parses.
- `DialogueChoice` **dropped `Eq`** (it now holds `f32` via `DialogueValue::Float`); keeps
  `PartialEq`. Minor breaking change — fine under the 0.x MINOR cadence, noted in CHANGELOG.

## 1d (per-line portraits) — DEFERRED, with reason

`DialogueBox.portrait` exists but `DialogueSystem` renders **text only** — portraits are not
drawn at all today. "Per-line portraits" therefore means *adding an image-rendering path to the
dialogue system* (a screen-space textured quad), which is its own feature, not a tweak. Deferred
to keep P1 coherent (data-driven + conditional + events). Logged in CHANGELOG "Deferred" and as
the natural P1 follow-up. Also deferred: a node-level unconditional `goto` (jumps use a single
choice today, as both examples do).

## Evidence & Data

**Merged diff (`c568f70`, PR #107).** New: `src/dialogue/tree.rs`, `src/dialogue/vars.rs`,
`examples/dialogue_quest.rs`, `examples/dialogue_quest.dlg.ron`. Renamed: `src/dialogue.rs` →
`src/dialogue/mod.rs`. Edited: `src/app.rs` (auto-register `DialogueRegistry`), `src/app/editor.rs`
(`App::load_dialogue`), `src/lib.rs` (re-exports), release paperwork (`Cargo.toml`/`Cargo.lock`
0.20.0, `docs/CHANGELOG.md`, `CLAUDE.md` header v1.6.69 + dialogue row). Also carried the
`docs/conversation-language-rule` commit + the two plan/handoff docs (swept in by `git add -A`).

**Verify gate (`./scripts/verify.sh`, EXIT 0, post-bump):** fmt --check · clippy --all-targets
-D warnings · wasm build (lib+bins) · wasm clippy --lib · `cargo test --all-targets` (**841 lib**) ·
`cargo test --doc` (65 pass / 33 ignored) · `RUSTDOCFLAGS=-D warnings cargo doc`. All passed ✓.

**CI (PR #107, run 27702981651): ALL GREEN.** Test (native) 6m54s · Build (WASM) 38s · Rustdoc
41s · Package dry-run 1m4s. (Disk-space step from seq 21 held.) Squash-merged on green.

**Playtest captures (`/tmp/dq_*.png`, may be cleared — re-run `cargo run --example dialogue_quest`):**

| File | Confirms |
|---|---|
| `dq_1_ask.png` | RON tree loaded + localized + numbered choices ("Old Merchant" / "Care for a lantern…?" / 1·2, `[en]`, "[ ] No lantern") |
| `dq_2_buy.png` | **event hook** — after pressing 1 (Buy), "[*] Lantern acquired" marker lit + buy line (grant_lantern event → game set has_lantern) |
| `dq_3_secret.png` | **conditional gate** — ask2 shows "1. Know any secrets worth the dark?" (the has_lantern-gated choice now visible) |
| `dq_4_korean.png` | **live locale** — speaker 노상인 + line + choices fully Korean, `[ko]`, position preserved at ask2 |

**macOS playtest keycodes:** Space=49, '1'=18, '2'=19, '3'=20, 'L'=37, ESC=53. Window 960×620 at
{100,100}; capture `-R100,100,980,660`. Synthetic `key code N` reaches winit (clicks don't —
seq-21 finding holds).

## Files Changed
- **New:** `src/dialogue/tree.rs` (DialogueTree/Registry/loader), `src/dialogue/vars.rs`
  (DialogueValue/Op/Cond/Effect/Event/Vars), `examples/dialogue_quest.rs`,
  `examples/dialogue_quest.dlg.ron`.
- **Renamed:** `src/dialogue.rs` → `src/dialogue/mod.rs` (DialogueBox/Choice/System + new
  vars-aware methods + `dialogue::advance`/`choose` free fns).
- **Edited:** `src/app.rs`, `src/app/editor.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`,
  `docs/CHANGELOG.md`, `CLAUDE.md`.

## Risks & Blockers
- **None blocking.** Tree clean, main green, P1 merged.
- Watch: `DialogueVars` is game-owned (not auto-inserted); `dialogue::choose` lazily inserts it
  on a `SetVar` effect, but conditions read an empty default if the game never inserts one. The
  `EmitEvent` path no-ops (logs `debug!`) unless `App::register_event::<DialogueEvent>()` was called.

## Open Questions
- None for P1. (P3A's wasm-AEAD value question remains open for later — see the roadmap PLAN.)

## Next: P2 — Particle RON→GPU builder (→ 0.21.0)
Per the roadmap PLAN §P2 (grounded there with file:line):
- Add `ParticleConfigSet::gpu_emitter(name) -> Option<GpuParticleEmitter>` mirroring `emitter()`
  (`src/particle/config_set.rs:227`); 9 fields map 1:1.
- `size` Vec2→f32 projection: **use `size.x`** (documented); `texture`/`z` ignored on GPU.
- Update `examples/gpu_particles.rs` to load from RON (replace the manual `default()`+assignment
  at lines 45–57) = acceptance test. Add unit tests. Ship 0.21.0, PR, merge, handoff.

## Quick Start
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # c568f70 feat(dialogue) … (#107)
grep -m1 '^version' Cargo.toml  # 0.20.0
./scripts/verify.sh             # green: 841 lib tests
cargo run --example dialogue_quest   # SPACE / 1·2·3 / L / R / ESC
# Then start P2 (particle RON→GPU). Read: src/particle/config_set.rs (emitter@227),
#   src/gpu_particle.rs (GpuParticleEmitter), examples/gpu_particles.rs (manual build@45-57).
```
