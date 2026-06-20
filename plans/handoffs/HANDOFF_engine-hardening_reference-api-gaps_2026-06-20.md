# Minor REFERENCE API gaps backfilled + dungeon-merchant wishlist board wired in

**Date:** 2026-06-20 (KST)
**Status:** COMPLETED — PR #156 merged + green; `main` @ `18ca817`, package **v0.43.5** (unchanged, docs-only), tree clean.
**Bead(s):** none (repo uses `plans/handoffs/`)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `46`
**Parent:** `HANDOFF_engine-hardening_doc-refresh_2026-06-20.md` (seq 45)

> Short, focused session. Two things happened: (1) a NEW downstream game, **`dungeon-merchant`**, started
> consuming the engine and introduced a shared **engine-wishlist board** — I registered its protocol as
> durable memory and confirmed the board is currently empty (next ID EW-001). (2) The user picked item **#3**
> from seq-45's "Where We're Going" backlog — the **minor REFERENCE.html API gaps** — and I backfilled all
> three (mutable queries, App-level `push_scene`/`pop_scene`, wasm save `localStorage` parity) in PR #156.
> Docs-only, no code/version change. This **closes** the seq-45 #1 follow-up.

---

## Since Last Handoff (vs seq-45's "Where We're Going")

Seq-45 listed three forward items. This session resolved **#1** and surfaced new context that reframes the rest:

- **#1 (minor REFERENCE gaps) — DONE this session** via PR #156. Was: "`App::push_scene`/`pop_scene`,
  `World::query2_mut`/`query3_mut`, wasm `save`/`load` localStorage parity note … Add only if asked." The user
  asked ("3"). All three documented.
- **#2 (engine-hardening backlog: per-OS gamepad input + crates.io publish) — UNCHANGED, still open.** Both
  need a user decision / hardware (see Where We're Going). Not touched.
- **#3 (optional focus-ring extensions / changelog-only features) — UNCHANGED.** The changelog-only features
  already got sections in seq-45 (#154); the focus-ring corner-radius/pulse polish remains optional.
- **NEW:** the `dungeon-merchant` game + wishlist board (no seq-45 mention — it started 2026-06-20). This is
  now the expected *driver* of future engine work: the game logs gaps, the engine session implements them.

## Reference Documents

- Parent `HANDOFF_engine-hardening_doc-refresh_2026-06-20.md` (seq 45) — defined the three minor gaps this
  session closed; its REFERENCE.html QA recipe (broken-anchor + tag-balance Python scan) was reused verbatim.
- `CLAUDE.md` — module-map rows grounded the doc edits (the `App::push_scene`/`pop_scene` row at line 101, the
  `query_mut`/`query2_mut`/`query3_mut` note in the ECS row). Header still **v1.6.99 / package v0.43.5**.
- Memory `dungeon-merchant-engine-wishlist.md` (reference type) — the board protocol, written this session
  (well, finalized at the start of this session per the compaction summary). Related: `engine-current-state`.
- `../dungeon-merchant/docs/engine-wishlist.md` — the actual shared board (lives in the GAME repo).

## The Goal

Two unrelated asks, both small: (a) acknowledge + durably register the new `dungeon-merchant` engine-wishlist
workflow so future sessions consult it; (b) backfill the three minor REFERENCE.html API gaps the user selected
("3"), source-verified and docs-only (no version bump).

## Where We Are

- `main` @ **`18ca817`** (PR #156), tree clean, all CI green (4/4). Package version unchanged at **v0.43.5**
  (docs-only PRs do not bump).
- **PR #156 (`docs/reference-minor-api-gaps` → squash-merged, branch deleted)** added 3 things to `REFERENCE.html`
  (+14 / −3 net):
  | gap | where in REFERENCE | what was added |
  |---|---|---|
  | mutable queries | `#쿼리` section | 3 table rows (`query_mut`/`query2_mut`/`query3_mut`) + a blockquote on disjoint borrows (`HashMap::get_disjoint_mut`, distinct types, same-type panics) + a `query2_mut` code example + a `query_changed`/`mark_changed` change-detection caveat |
  | App scene control | `#씬-전환` section | App-level `set_scene`/`push_scene`/`pop_scene` convenience block (= `SceneCmd::Replace`/`Push`/`Pop`), placed alongside the existing in-system `SceneChange` runtime path |
  | wasm save parity | `#저장불러오기` section | a blockquote: all of `save`/`load`/`save_versioned`/`load_migrated`/`write_ron`/`read_ron`/`exists`/`delete` work on wasm via a hex-encoded AEAD blob keyed by the path string (`OsRng` nonce); embedded key = obfuscation + tamper-detection, not secrecy; browser-verified by `wasm_save` + `scripts/wasm_save_smoke.sh` |
- **dungeon-merchant wishlist:** board read, currently **empty** ("Active requests: None yet", next free ID
  **EW-001**). Memory + `MEMORY.md` pointer written. Nothing to implement until the game files a request.

## What We Did (chronological)

1. **Opened on the wishlist context** (carried from the compaction boundary): confirmed `dungeon-merchant`
   exists, consumes `skeleton-engine = { path = "../skeleton-engine" }`, and its
   `../dungeon-merchant/docs/engine-wishlist.md` is a two-way EW-NNN board. Gave the user a Korean summary of
   the protocol; confirmed the board is empty (next ID EW-001) so there is nothing to act on yet.
2. **User asked to check the handoff for pending work.** Read seq-45 in full; reported the clean state and the
   open backlog as a table (crates.io publish, per-OS gamepad, minor REFERENCE gaps, focus-ring polish).
3. **User chose "3"** = the minor REFERENCE gaps.
4. **Source-verified every API before writing** (no invented API in the doc):
   - `src/app/scenes.rs:64/72/78` — `pub fn set_scene` / `push_scene` / `pop_scene`, each a thin
     `apply_scene_cmd(SceneCmd::Replace|Push|Pop)` wrapper. Doc comments confirm semantics (Push suspends, not
     destroys; Pop resumes beneath, no-op on empty stack).
   - `src/ecs/world.rs:358/387/421` — `query_mut`/`query2_mut`/`query3_mut` return
     `(Entity, &mut …)`; multi-component use `columns.get_disjoint_mut([…])`, types must be distinct.
   - `src/ecs/world.rs:785/807` — `query_changed::<T>()` / `mark_changed::<T>(entity) -> bool`; mutable queries
     do NOT auto-record changes (confirmed in the doc comments at lines ~356, ~419).
   - `src/components.rs:14` — `Transform { position: Vec2, scale, rotation, z }` — so the example uses
     `t.position`, NOT `translation`. `Velocity` is NOT an engine type → documented inline as a user-defined
     example component (`struct Velocity(Vec2)`), consistent with the doc's existing `Enemy`/`Health` style.
   - `src/save.rs` — `localStorage` keyed by `path.to_string_lossy()`; `to_hex`/`from_hex` for the blob;
     `OsRng` nonce; comments confirm "obfuscation+tamper-detection, not secrecy".
5. **Read the three target REFERENCE.html sections** to match house style (Korean prose, `&lt;`/`&amp;`-escaped
   code, `<div class="table-wrap"><table>` rows).
6. **Made 4 Edits** (one extra to fix `translation`→`position` after verifying Transform's real fields).
7. **Ran the QA scan** (the seq-45 recipe): 0 broken anchors of internal hrefs, `<pre>` 196/196, `<table>`
   52/52, `<code>` 1547/1547 (counted with attributes — see Gotcha #1), 0 double-escapes.
8. **Branch → commit → push → PR #156 → poll-then-watch CI → squash-merge → ff main.** Followed the seq-44/45
   poll-before-watch discipline (checks registered after ~6s, then `--watch` to 4/4 green).

## Key Decisions

- **Docs-only, no version bump.** Matches docs PRs #153/#154/#90. The package is already v0.43.5 and these are
  pure additions to an existing Korean doc; nothing in `Cargo.toml`/`CHANGELOG.md`/`CLAUDE.md` needed touching.
- **Source-verify, never paraphrase from memory.** Every type/field/signature was grepped from `src/` before
  being written. This caught `Transform.translation` (wrong) → `position` (right), and confirmed `Velocity` is
  a game-defined component, not an engine export — documented as such inline.
- **Match the doc's existing Korean + illustrative-component style.** REFERENCE.html is a Korean external doc
  (per seq-45's "match in-doc language" decision); new prose is Korean, code/identifiers stay as-is, and
  example components follow the existing `Enemy`/`Health` convention.
- **Do NOT pre-emptively mine the backlog.** With the wishlist board now the intended driver and its queue
  empty, the right move is to wait for EW-001 rather than start crates.io publish / gamepad work unprompted
  (both need an explicit user go anyway).

## Evidence & Data

### PR #156 — REFERENCE.html QA (post-edit)
```
broken anchors:   0  (of all internal href="#…")
tag balance:      <pre> 196/196 · <table> 52/52 · <code> 1547/1547 (attr-aware count)
double-escape:    0 occurrences of &amp;lt; in code blocks
diff:             REFERENCE.html  +14 / −3  (1 file)
```

### CI (PR #156, 4/4 green)
| Job | Result | Time |
|---|---|---|
| Rustdoc | pass | 31s |
| Build (WASM) | pass | 40s |
| Package dry-run | pass | 1m5s |
| Test (native) | pass | 3m31s |
(CI does not validate HTML — the manual anchor/tag/escape scan is the real QA, as in seq-45.)

### Verified source coordinates (for the next editor)
| API | file:line | signature/fact |
|---|---|---|
| `App::set_scene` | `src/app/scenes.rs:64` | `apply_scene_cmd(SceneCmd::Replace(scene))` |
| `App::push_scene` | `src/app/scenes.rs:72` | `SceneCmd::Push` — suspends current, not destroys |
| `App::pop_scene` | `src/app/scenes.rs:78` | `SceneCmd::Pop` — resumes beneath, no-op if empty |
| `World::query_mut` | `src/ecs/world.rs:358` | `impl Iterator<Item=(Entity, &mut T)>` |
| `World::query2_mut` | `src/ecs/world.rs:387` | `get_disjoint_mut([&ta,&tb])`, distinct types |
| `World::query3_mut` | `src/ecs/world.rs:421` | `get_disjoint_mut([&ta,&tb,&tc])`, distinct types |
| `World::query_changed` | `src/ecs/world.rs:785` | mutable queries do NOT feed this |
| `World::mark_changed` | `src/ecs/world.rs:807` | `(entity) -> bool` — call after a mut query if change-detection needed |
| `Transform` | `src/components.rs:14` | `position: Vec2` (NOT `translation`), `scale`, `rotation`, `z` |
| wasm save | `src/save.rs:140-143, 290-302` | hex blob in `localStorage` keyed by path string, `OsRng` nonce |

## Files Changed

- **PR #156:** `REFERENCE.html` only (+14 / −3). No source, Cargo, CHANGELOG, or CLAUDE.md change.
- **Memory (not in git):** `dungeon-merchant-engine-wishlist.md` (reference) + `MEMORY.md` pointer line —
  written at the session start (per the compaction summary); confirmed present this session.

## User Feedback & Preferences

- **"3"** — terse selection of backlog item #3. The user picks from a presented numbered menu; keep the
  "next actions" list numbered and stable so a bare number resolves unambiguously.
- **Wishlist workflow** ("게임개발하면서 엔진에 요청 할 부분 … engine-wishlist.md 에 공유 할테니까 참고해줘") —
  the user will drive engine work through the board; consult it each engine session.
- Standing prefs (unchanged): Korean for user-facing reports; English for code/handoffs/sub-agent prompts;
  REFERENCE/ARCHITECTURE maintained in Korean; merge standing-delegated (squash on green CI, expressed as a
  direct instruction, never an AskUserQuestion option); source-verify docs; honest gap-naming.

## Where We're Going

1. **dungeon-merchant wishlist — the primary driver now.** Each engine session: read
   `../dungeon-merchant/docs/engine-wishlist.md`, pick up any `Proposed`/`Acknowledged`/`In-progress` EW-NNN,
   implement engine-side (VISION loop: a small example doubles as the game's acceptance test), then set status
   `Shipped (vX.Y.Z)` and reply in-thread. **Currently empty (next ID EW-001) — nothing queued.**
2. **Engine-hardening backlog (unchanged, needs a user go):**
   - **crates.io publish** — irreversible; needs explicit go; publish `engine_reflect_derive` too.
   - **per-OS gamepad input** (Win/Mac) + the deferred analog-stick Y-sign hardware test (BT Xbox pad
     enumerates but sends no input to gilrs on macOS — `gilrs-macos-xbox-no-input` memory).
3. **Optional polish:** seq-43 focus-ring extensions (corner radius / pulse). Low value.
4. **REFERENCE — effectively complete.** All known gaps (subsystems #154, minor APIs #156) are closed. The only
   remaining caveat: the OLD body prose is still "as-of ~v9.3/v10.7" (header says so); the 6 backfilled
   sections + these 3 minor additions ARE source-verified-current.

## Risks & Blockers

- **None outstanding** — PR merged green, tree clean, docs reflect v0.43.5.
- HTML correctness is **not** CI-gated (no HTML linter) — re-run the anchor/tag/escape scan after ANY future
  `REFERENCE.html` edit (recipe in Quick Start below).
- The wishlist board lives in a **separate repo** (`../dungeon-merchant`) — editing it is a cross-repo write;
  only touch it in response to an actual EW request (the engine session updates status / replies in-thread,
  but does not invent requests).

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 18ca817 (#156) … f4d4042 (#155 seq-45 handoff) … 329764a (#154)
grep -m1 '^version' Cargo.toml  # 0.43.5 (unchanged — docs were docs-only)
git status -s                   # clean

# FIRST: check whether the game queued any engine work
sed -n '53,62p' ../dungeon-merchant/docs/engine-wishlist.md   # "Active requests" — empty as of 2026-06-20

# Re-run the REFERENCE.html QA after ANY edit to it (no HTML linter in CI):
python3 - <<'PY'
import re; h=open("REFERENCE.html").read()
ids=set(re.findall(r'id="([^"]+)"',h)); hrefs=re.findall(r'href="#([^"]+)"',h)
print("broken anchors:", [x for x in hrefs if x not in ids])              # expect []
print("pre", h.count('<pre>'),'/',h.count('</pre>'),
      "table", h.count('<table>'),'/',h.count('</table>'),
      "code(attr-aware)", len(re.findall(r'<code(?:\s[^>]*)?>',h)),'/',h.count('</code>'),
      "double-escape", h.count('&amp;lt;'))                                # balanced + 0
PY

# Next action (nothing required):
#   (A) implement an EW-NNN once the game files one (status → Shipped, reply in-thread), OR
#   (B) engine-hardening backlog: crates.io publish (explicit go) / per-OS gamepad input.
```

## Cross-cutting Gotchas (expensive-to-rediscover)

1. **`h.count('<code>')` UNDER-counts opens — REFERENCE uses `<code class="language-rust">` for code blocks.**
   A bare-tag count gave `1357 open / 1547 close` (looked like a 190-tag imbalance, a false alarm). The
   attribute-aware regex `re.findall(r'<code(?:\s[^>]*)?>', h)` gives the true `1547/1547`. Always count opens
   attribute-aware when balance-checking `<code>` here. (`<pre>`/`<table>` have no attributes, so plain
   `.count()` is fine for those.)
2. **Source-verify field/type names in doc examples.** `Transform` has `position`, not `translation` — caught
   only by grepping `src/components.rs`. A plausible-but-wrong field in a doc example is worse than none. Also
   confirm whether an example type is an engine export or a user component (`Velocity` was the latter →
   documented inline as `struct Velocity(Vec2)`).
3. **Poll `gh pr checks` before `--watch`.** `gh pr checks <n> --watch` exits immediately ("no checks
   reported") if run <~30s after `gh pr create`. Loop on `gh pr checks` until a check registers, THEN `--watch`.
   (Carried from seq-44/45; held again here — checks registered after ~6s.)
4. **Docs-only → skip `/ship`.** No version bump, no Cargo/CHANGELOG/CLAUDE.md edit; just the doc file +
   branch→PR→merge. Confirmed by precedent (#90, #153, #154).
5. **The wishlist board is the new front door for engine requests.** Don't start backlog items (crates.io,
   gamepad) unprompted now that the game drives priorities — wait for an EW-NNN or an explicit user go.

---

## Session Status
**Goal met** — minor REFERENCE gaps closed (#156, merged green), wishlist workflow registered + confirmed
empty. `main` @ `18ca817`, v0.43.5, tree clean. Handed off to next session (seq 46).
