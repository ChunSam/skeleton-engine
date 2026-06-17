# Roadmap Plan — P1 Dialogue-data / P2 Particle RON→GPU / P3 wasm parity

**Date:** 2026-06-18
**Chain:** `engine-hardening` (follows seq 21 / PR #106 / v0.19.0, `main` @ `9f1ff10`)
**Baseline:** package `skeleton-engine` v0.19.0, 801 lib tests, CI green, tree clean.
**Status:** PLAN — not started. Priorities ordered 1→2→3 per user. Each is an independent epic.
**Release mapping (recommended):** P1 → `0.20.0` · P2 → `0.21.0` · P3 → `0.22.0`+ (different themes; do NOT bundle into one MINOR).

> Grounding for every claim below was gathered by three read-only Explore passes on
> 2026-06-18 (dialogue surface + loader pattern, particle CPU/GPU surface, wasm save/audio
> surface). File:line refs are from that pass — re-verify before editing, code may move.

> **Conversation-language rule (in force):** user-facing reports/questions → Korean;
> agent-to-agent prompts, code, and file-written docs (incl. this plan) → English.

---

## P1 — Dialogue data-driven epic → `0.20.0`

### Goal
Extend the in-code branching DialogueBox (shipped seq 21) to a RON data-driven dialogue
tree, with flag/var-gated conditional choices and choice→event/effect hooks. **Reuse the
existing `DialogueBox`/`DialogueSystem` runtime**; the tree is an authoring format that
flattens to the existing index-based model at spawn. Purely additive — a box with no
tree/cond/effect renders byte-identically to v0.19.0, and old scene RON still loads.
The acceptance test is a small playable example (`dialogue_quest`).

### Current surface (src/dialogue.rs, ~550+ lines)
- `DialogueChoice { text: String, key: Option<String> #[serde(default)], goto: usize }`
  — ctors `new(text, goto)`, `localized(key, goto)`.
- `DialogueBox` fields (serde order): `speaker, lines, current, chars_per_sec,
  line_keys(#[default]), speaker_key(#[default]), choices: Vec<(usize, Vec<DialogueChoice>)>(#[default]),
  elapsed(priv,serialized), full(priv,serialized), portrait(#[serde(skip)])`.
- Methods: `new`, `localized`, `with_chars_per_sec`, `with_portrait`, `with_choices(line, choices)`,
  `resolve(&LocaleResource)` (never touches current/elapsed/full), `tick(dt)`, `advance()`
  (no-op while `pending_choices().is_some()`), `choose(i)` (out-of-range goto clamps to
  `lines.len()`), `reset()`, `current_line`, `visible_text`, `line_fully_revealed`,
  `pending_choices() -> Option<&[DialogueChoice]>`, `is_finished`.
- `DialogueSystem::run` steps: 0 resolve (clone `LocaleResource`, `query_mut`), 1 tick
  (`query_mut`), 2 gather (`query`, collect `(speaker, visible_text, full, choice_labels)`),
  3 render via `TextQueue` (speaker `(60,vh-150)`; body `(60,vh-118)` bounds `(vw-120,90)`;
  choices numbered from `y=vh-86` step 26; else `▼ space` hint).

### Loader template (mirror — verified identical across 3 modules)
Particle `config_set.rs`, `animation/clip_set.rs`, `data_table.rs` share ONE shape:
1. **`*Set`** plain struct + `pub fn from_ron_str(s) -> Result<Self, *Error>` (internal
   storage alphabetically sorted for determinism).
2. **`*Registry`** `#[derive(Default)]` resource: `HashMap<String, *Set>` + (native-only)
   `HashMap<String,String>` path map. Methods `load(name, path)` (native-only),
   `get(name) -> Option<&*Set>`, `reload_path(path)` (native-only).
3. **`impl HotReloadable for *Registry`** (native-only) delegating via UFCS to the
   inherent `reload_path` (avoids self-recursion). Trait in `src/asset/hot_reload.rs`.
4. **`App::new()`** auto-registers via `app.register_hot_reloadable::<*Registry>()`
   (src/app.rs:243–247); forwarders stored in `hot_reload_forwarders`, driven each frame
   by `AssetServer::poll_reloads()` → `schedule.rs`.
5. **`App::load_*(name, path)`** in `src/app/editor.rs` (siblings at :741/:792/:834):
   lazy-insert registry → `register_persistent::<Registry>()` (survives `SceneCmd::Replace`)
   → `reg.load(name, &path)` → `assets.watch_*_path(&path)`. Whole body
   `#[cfg(not(target_arch="wasm32"))]`; wasm no-op. Call site: BEFORE `app.set_scene()`.

RON shapes for reference: particle = top-level `(emitters: { "name": (..) })`; clips =
`(atlas: (..), clips: { "name": (..) })`; data table = a RON *sequence* of rows.

### Sub-tasks
- **PRE — split module.** `src/dialogue.rs` is already large; run `/split-module` to
  `src/dialogue/` → `box.rs` (existing DialogueBox/Choice/System verbatim), `tree.rs`
  (new loader), `vars.rs` (new vars/cond/effect). Behavior-preserving; tests stay green.
  Update CLAUDE.md module-map row.
- **1a — RON tree loader (spine).** In `tree.rs`:
  - `DialogueNode { id: String, speaker(_key), line(_key), choices: Vec<DialogueChoice> }`
    (RON authoring node; `goto` references a node id).
  - `DialogueTree` (= *Set): ordered nodes + `from_ron_str`. RON shape e.g.
    `(nodes: { "intro": (speaker_key:"npc.x", line_key:"dlg.intro", next:"ask"),
    "ask": (line_key:"dlg.ask", choices:[(key:"choice.buy", goto:"buy"), ..]), .. })`.
  - `DialogueRegistry` resource + `HotReloadable` + `App::load_dialogue(name, path)` in
    `src/app/editor.rs` — exact `load_data_table` body shape.
  - **Spawn bridge:** `DialogueTree::to_box()` / `DialogueBox::from_tree(&tree)` flattens
    node-id graph → `lines`/`line_keys` (by node order) + `choices: Vec<(usize, Vec<..>)>`
    with each choice `goto` resolved from node id → line index. This reuses ALL existing
    DialogueBox/DialogueSystem machinery — minimal new runtime.
- **1b — conditional choices.** In `vars.rs`:
  - `DialogueVars` resource = `HashMap<String, BlackboardValue>` (global/cross-scene flags).
    Optionally also read a per-entity `Blackboard` (it is a standalone component, zero
    coupling to BehaviorTree — reuse as-is for NPC-local state).
  - `DialogueChoice.cond: Option<DialogueCond>` where
    `DialogueCond { var: String, op: Eq|Ne|Gt|Lt, value: <scalar> }`; evaluate against
    `DialogueVars` (+ optional Blackboard). `BlackboardValue` is `Bool/Float/Int/Vec2/String/Path`,
    `#[non_exhaustive]` (wildcard arm required in matches).
- **1c — choice→event/effect hooks.** `DialogueChoice.effect: Option<DialogueEffect>` =
  `SetVar { key, value }` | `EmitEvent { name: String }`. On choose: apply `SetVar` to
  `DialogueVars`, push a `DialogueEvent` to `Events<DialogueEvent>` for the game to read.
- **1d — per-line portrait.** Box-level `portrait: Option<Handle<ImageAsset>>`(`#[serde(skip)]`)
  → per-node. Tree carries a portrait *path/key*; resolve to a `Handle` at load (Handle is
  not serde). Lower priority, visual-only, independent.
- **1e — example `dialogue_quest`** (acceptance test): loads a `.dlg.ron`, has a
  flag-gated choice, fires an event the game handles (e.g. receive-key → open-door), live
  en↔ko toggle. Playtest windowed (osascript `key code` reaches winit; clicks do not).

### KEY DESIGN DECISION (blocks 1b/1c) — confirm during 1e
`choose(i)`/`pending_choices()` are `&self`/`&mut self` only; cond-evaluation and
effect-application need variables + world access. Re-wire the show/select path through the
world: `DialogueSystem` renders only cond-passing (visible) choices; the game selects by
visible index; a free fn `dialogue::choose(world, entity, visible_i)` does (a) visible→raw
index mapping, (b) `goto` jump, (c) effect apply in one place. **This is where the API may
feel awkward — fix it while writing 1e (VISION rule).**

### Anti-goals
No node-level scripting/expression language (cond is a single var compare); no visual
graph editor this epic; keep `choices` tuple-keyed (serde-trivial). Conditional/effect are
additive Options — literal & v0.19 branching boxes unchanged.

### Sizing / sequencing
Largest of the three. Commit split: PRE split → 1a → (1b+1c together, shared vars) → 1d → 1e.
Single `0.20.0` MINOR; `/ship` paperwork + windowed playtest + re-confirm merge authority.

---

## P2 — Particle RON→GPU builder → `0.21.0`

### Goal
Close the seq-20 carried gap: RON `gravity`/`emit_shape` currently feed only the CPU
`ParticleEmitter`. Add a RON→`GpuParticleEmitter` path. **Confirmed there is NO existing
path** (grep `gpu_emitter|GpuParticleEmitter` in `src/particle/` = 0 hits).

### Current surface
- `ParticleConfigSet::emitter(name) -> Option<ParticleEmitter>` (config_set.rs:227) — a
  clean field-copy from `EmitterDef` (12 fields incl. `gravity`, `emit_shape`, `texture`,
  `z`). `App::load_particle_configs` (editor.rs:834) already loads + hot-reloads.
- `GpuParticleEmitter` (gpu_particle.rs:33–60) public fields: `spawn_rate, lifetime,
  velocity, velocity_spread, color_start, color_end, size: f32, gravity, emit_shape, emit`
  (+ priv `timer`/`next_slot`). Builders `with_gravity`, `with_emit_shape`; otherwise
  `default()` + field assignment (private fields). Example construction at
  `examples/gpu_particles.rs:45–57`.
- Field parity CPU↔GPU: 9 fields map 1:1; **`size`** is `Vec2`(CPU) vs `f32`(GPU) — TYPE
  GAP; **`texture`** and **`z`** exist on CPU only — absent on GPU.

### Sub-tasks
- **2a** — `ParticleConfigSet::gpu_emitter(name) -> Option<GpuParticleEmitter>` mirroring
  `emitter()`; 9 fields direct, `texture`/`z` ignored.
- **2b** — `size` Vec2→f32 projection. **Recommend `size.x`** (simplest, predictable;
  document it). Alternatives: `(x+y)/2`, or a new optional RON `gpu_size: Option<f32>`.
- **2c** — Update `examples/gpu_particles.rs` to load from RON (replace the manual
  construction) OR add a GPU variant of `data_particles` = acceptance test. Add unit tests.

### Sizing
Small (single builder + example + tests). A quick win; could slot between P1 commits if
desired, but ordered after P1 per request. Single `0.21.0` MINOR.

---

## P3 — wasm parity → `0.22.0`+ (two independent axes)

### 3A. wasm AEAD save/load → `0.22.0`
Today on wasm: `save`/`save_with_key`/`load`/`load_with_key`/`save_versioned*`/
`load_migrated*` all return `SaveError::Unsupported`; only `write_ron`/`read_ron`/`exists`/
`delete` work (via `localStorage`/`wasm_storage`, src/save.rs:272–297). `load_or_default`
is effectively broken on wasm (calls `load` → `Unsupported`). `SaveError` variants:
`Io/Ron/Corrupted/Unsupported`.

- **3A-1** wasm nonce RNG: native `rand::thread_rng()` → `getrandom`/`OsRng` on wasm
  (`getrandom = { features=["js"] }` is already a wasm dep; `chacha20poly1305`/`rand` are
  unconditional deps but all usage is currently native-gated).
- **3A-2** base64-encode ciphertext for the string-only `localStorage` (binary not
  storable) — add a tiny base64 dep or inline encoder.
- **3A-3** implement the wasm branches of `save_with_key`/`load_with_key` and the
  versioned/migrated variants (currently `#[cfg(wasm32)]` → `Unsupported`).

> **OPEN QUESTION (confirm before starting 3A):** `save.rs:270` comment says the author
> *deliberately* skipped wasm AEAD — "a hardcoded key in a browser-inspectable store buys
> little." Browser AEAD is obfuscation, not real security. Confirm it's actually wanted;
> plain `write_ron` may suffice for most wasm games.

### 3B. wasm audio depth → `0.23.0`
Today `WebAudio` (src/audio_wasm.rs) is TWO methods: `new()`, `play(bytes)` (one-shot,
fire-and-forget). Native `AudioManager` channels/volume/pan/fade/crossfade/bus/ducking/
positional/effects are ALL absent on wasm. No `kira` anywhere (confirmed). `rodio` is
native-only (Cargo.toml:166).

- **3B-1** `GainNode` (volume + fade) + `BufferSourceNode.loop` (looping) + channel handles
  + `stop`. (Gas-for-value first pass.)
- **3B-2** `StereoPannerNode` (pan) + minimal bus/duck.
- **3B-3** (deferred) full parity or adopt `kira` — recommend **incremental WebAudio
  extension over a kira swap** (kira is a large dependency change).

### Sizing
Large overall. 3A medium, 3B large. Split across `0.22.0` (save) and `0.23.0` (audio).

---

## Cross-cutting
- **No ordering dependency** between P1/P2/P3 — independent. P2 is short ("quick win").
- **VISION:** every epic's example is its acceptance test; if an API feels awkward while
  writing the example, fix the API before release.
- **Per epic:** verify gate (`./scripts/verify.sh`, no `| tail`) → `/ship`
  (Cargo.toml/Cargo.lock/CHANGELOG/CLAUDE.md) → windowed playtest → re-confirm merge
  authority (per-session) → branch + PR + squash-merge on green CI.
- **Two confirmations still open:** P1 `choose` re-wiring approach (recommended above),
  P3A wasm-AEAD value (author deliberately skipped it).

## Quick start for next session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 9f1ff10 + docs/conversation-language-rule branch
./scripts/verify.sh             # green baseline: 801 lib tests
# P1 first: /split-module src/dialogue.rs → src/dialogue/{box,tree,vars}.rs, then 1a.
# Read: src/dialogue.rs, src/particle/config_set.rs, src/animation/clip_set.rs,
#       src/data_table.rs, src/app/editor.rs (load_* methods), src/behavior.rs (Blackboard).
```
