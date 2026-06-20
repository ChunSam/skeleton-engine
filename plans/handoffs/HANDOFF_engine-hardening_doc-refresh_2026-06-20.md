# External-facing docs brought current to v0.43.5 (REFERENCE + ARCHITECTURE refresh + subsystem backfill)

**Date:** 2026-06-20 (KST)
**Status:** COMPLETED — PRs #153 + #154 merged + green; `main` @ `329764a`, package **v0.43.5**, tree clean.
**Bead(s):** none (repo uses `plans/handoffs/`)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `45`
**Parent:** `HANDOFF_engine-hardening_refactor-sweep_2026-06-20.md` (seq 44)

> After the seq-44 refactor sweep, the user asked to **check whether the external-facing shared docs are
> up to date**. Audit found `REFERENCE.html` and `ARCHITECTURE.html` badly stale (version markers + module
> layout + ~30 releases of undocumented 0.x features). Two docs-only PRs brought them current: **#153**
> (core refresh — version stamps, 0.x changelog summary, module-map fixes) and **#154** (backfill of 6
> missing subsystem sections, drafted by 6 parallel sub-agents and integrated here). No code changed.

---

## Since Last Handoff (vs seq-44's "Where We're Going")

Seq-44 (the refactor sweep) listed next-actions: per-OS gamepad input, crates.io publish, optional further
refactors. **None of those were the work this session** — instead the user opened a NEW thread: "레퍼런스,
아키텍쳐 등 외부 공유용 문서가 최신화 되어있는지 체크." So seq-45 is a continuation of the engine-hardening
*epic* but addresses a fresh ask (external-doc currency), not the seq-44 backlog (which remains open,
unchanged).

Note: seq-44 itself was written EARLIER in this same long session (it covered the hex blob atlas v0.43.0 +
the P3–P5 module-split sweep v0.43.1–v0.43.5). This handoff covers only the doc work that followed it.

## Reference Documents

- Parent `HANDOFF_engine-hardening_refactor-sweep_2026-06-20.md` (seq 44) — the refactor sweep whose module
  splits this doc refresh had to reflect in ARCHITECTURE's module map.
- `CLAUDE.md` — the module-map rows are the accurate API summaries the sub-agents (and I) grounded the doc
  sections on. Now header **v1.6.99 / package v0.43.5** (bumped during the seq-44 sweep, not this session).
- `docs/CHANGELOG.md` — already current (top = 0.43.5); the new REFERENCE 0.x changelog section points here
  for per-release detail.

## The Goal

Make the crates.io-packaged, external-facing documentation set accurately reflect the current engine
(v0.43.5): correct version markers, a module layout that matches the post-refactor `src/` tree, and actual
API documentation for the ~30 releases of 0.x features that had accumulated with no REFERENCE coverage.
Docs-only — no package version bump, no code change.

## Where We Are

- `main` @ **`329764a`** (PR #154), tree clean, all CI green. Package version unchanged at **v0.43.5**
  (docs-only PRs do not bump it).
- **Audit verdict (5 external docs):**
  | doc | verdict | action |
  |---|---|---|
  | `README.md` | current (version-agnostic, fork-first, no stale refs) | none |
  | `docs/CHANGELOG.md` | current (0.43.5, `/ship` maintains) | none |
  | `FORKING.md` | current (no now-directory files referenced as `.rs`) | none |
  | `REFERENCE.html` | **stale** | #153 + #154 |
  | `ARCHITECTURE.html` | **stale** | #153 |
- **#153 (core refresh, `08b899a`):**
  - REFERENCE header `버전 v0.11.0 기준` → **`v0.43.5 기준 (2026-06-20)`** + a note that the body is as-of
    ~v9.3/v10.7 and the API is mostly additive since; pointer to the new 0.x changelog + `docs/CHANGELOG.md`.
  - REFERENCE new **`변경 이력 (v0.11–v0.43.5, 0.x 리셋 이후)`** section (TOC link + body), grouping the whole
    0.x line by version range.
  - ARCHITECTURE **`v0.43.5 기준 (2026-06-20)`** stamp; fixed stale file refs `src/tilemap.rs`/`src/particle.rs`
    → `src/tilemap/`/`src/particle/`; module map now reflects the refactor sweep (`app/render/`, `app/editor/`,
    `renderer/text/`, `renderer/sprite/` collect/batch/draw) + a new **Audio / net / narrative** row
    (`src/audio*`, `src/network/`, `src/dialogue/`) the 8-row map had been missing.
- **#154 (subsystem backfill, `329764a`):** added 6 dedicated REFERENCE API sections (Korean, example-grounded),
  inserted in the main body *before* the changelog cluster, each with a TOC link. h2 sections 69 → **75**.
  +504 lines. The 6: 다이얼로그, 타일맵 프로젝션(iso/hex), UI 포커스 내비+FocusRingStyle, WebAudio(wasm),
  TimeScale/RealDt, 기타 신규(Coroutine/TweenSequence/Parallax/ShaderMaterial/animated tiles/NineSlice).

## What We Did (chronological)

1. **Audited the external docs.** Extracted REFERENCE's full h2/h3 heading list; grepped version markers + the
   refactor-split file paths across all docs. Found: REFERENCE header at v0.11.0, changelog stops at v10.7.0;
   ARCHITECTURE (June 3, no stamp) references several now-directory modules as `.rs` files. README/CHANGELOG/
   FORKING current.
2. **Asked the user the update scope** (AskUserQuestion) → chose **"둘 다 핵심만"** (both, core only) over a full
   REFERENCE backfill or report-only.
3. **#153 core refresh.** Edited ARCHITECTURE (stamp, file→dir, module-map rows) + REFERENCE (header, 0.x
   changelog section + TOC). Verified anchor consistency, tag balance, no leftover stale `.rs` refs. Committed,
   pushed, PR #153, CI green, squash-merged.
4. **User then asked for the backfill too** ("REFERENCE 누락 서브시스템 백필도 진행해줘").
5. **Pinned the true gap** (keyword grep has false negatives — English terms like `NineSlice` aren't caught by
   Korean keywords; the changelog-only features have code but no dedicated sections). Confirmed genuinely-absent:
   dialogue, tilemap iso/hex projections + hex autotile, UI focus + FocusRingStyle, WebAudio(wasm),
   TimeScale/RealDt, and the changelog-only Coroutine/TweenSequence/Parallax/ShaderMaterial/animated-tiles/NineSlice.
6. **Delegated drafting to 6 parallel Opus sub-agents** (one per section), each instructed to read the REAL
   source (`src/`, `src/lib.rs`) + the actual compiling example and return a Korean HTML fragment + a TOC `<a>`
   line — NO file writes (so they can't conflict on REFERENCE.html).
7. **Extracted the fragments from the sub-agent transcripts** (see Gotcha #1) and **integrated them with a Python
   script**: inserted the 6 bodies before the `<h2 id="변경-이력-v91v93">` changelog anchor, inserted the 6 TOC
   links before the changelog TOC link in the nav.
8. **Verified the result**: broken-anchor scan (0 broken of 141 hrefs), tag balance (`<pre>` 195/195, `<table>`
   52/52, `<code>` 1510/1510, one `<main>`), code-block escaping single-level (no `&amp;lt;` double-escape),
   preamble-leakage scan (0), fixed one stale cross-link `#audiomanager` → `#오디오`. Committed, pushed, PR #154,
   CI green, squash-merged.

## Key Decisions

- **Match the existing doc language (Korean).** `REFERENCE.html`/`ARCHITECTURE.html` are already Korean external
  docs (README labels them "written in Korean"). The English-default doc rule yields to in-doc consistency for
  this established Korean external-doc set — so all new sections are Korean (code/identifiers stay as-is).
- **Two-step delivery matching the user's scope choice.** #153 = the bounded "core" refresh (version + layout +
  changelog summary). #154 = the larger backfill, run only after the user explicitly asked. Kept PRs focused.
- **Sub-agents DRAFT, orchestrator INTEGRATES.** Each sub-agent read real source + a compiling example and
  returned an HTML fragment (no file writes → parallel-safe, no REFERENCE.html merge conflicts). I did all the
  insertion + anchor/structure verification + the cross-link fix in the main thread. This separated "accurate
  content generation" (parallelizable) from "single-file integration" (sequential, conflict-free).
- **Docs-only, no version bump.** Matches the prior docs PR (#90). The package version (v0.43.5) already
  matches the new doc stamps, so nothing to bump.
- **Did NOT do a full per-release REFERENCE rewrite.** The 0.x changelog is a *summary* pointing to
  `docs/CHANGELOG.md` for detail; the backfill covers the *subsystems*, not every minor API tweak (e.g.
  `push_scene`/`pop_scene`, `query2_mut`/`query3_mut` remain undocumented as standalone — low value, see Where
  We're Going).

## Evidence & Data

### Audit — REFERENCE subsystem coverage (the gap that drove #154)
Grep of REFERENCE.html headings/refs before #154 (Korean-keyword counts, with English-term false-negatives noted):
| feature | pre-#154 state |
|---|---|
| dialogue (다이얼로그/대화) | 0 — entirely absent |
| coroutine (코루틴) | changelog-only (v10.5.0) |
| TimeScale/RealDt | 0 — absent |
| iso/hex tilemap (아이소/헥사), Hex6 | 0 — absent from the tilemap section |
| UI focus (UiFocus/FocusRingStyle) | 0 — absent |
| WebAudio (wasm) | minimal — native `AudioManager` only |
| TweenSequence/Parallax/ShaderMaterial/NineSlice/animated tiles | changelog-only |

### #154 integration verification
```
broken anchors:        0 of 141 internal hrefs
tag balance:           <pre> 195/195 · <table> 52/52 · <code> 1510/1510 · <main> 1/1
code-block escaping:   single-level (Option&lt;Self&gt;), 0 occurrences of &amp;lt; (no double-escape)
preamble leakage:      0
cross-link fix:        #audiomanager → #오디오 (1)
h2 sections:           69 → 75 (+6),  +504 lines
```

### CI (both PRs, 4/4 green)
| PR | Build WASM | Package dry-run | Rustdoc | Test native |
|---|---|---|---|---|
| #153 | pass | pass | pass | pass |
| #154 | pass | pass | pass | pass |
(CI does NOT validate HTML — these confirm the docs-only change broke nothing; the HTML QA was the manual
anchor/tag/escape scan above.)

## Files Changed

- **#153:** `ARCHITECTURE.html` (+19/−6: stamp, file→dir, 3 module-map rows + new row), `REFERENCE.html`
  (+13/−1: header, 0.x changelog section + TOC link).
- **#154:** `REFERENCE.html` (+504: 6 `<h2>` sections + 6 TOC links + 1 cross-link fix).
- No source, Cargo, or CHANGELOG changes (docs-only).

## User Feedback & Preferences

- **"외부 공유용 문서가 최신화 되어있는지 체크"** → wanted an audit first, not blind edits. I audited, reported a
  table, then asked scope.
- **Scope choice "둘 다 핵심만"** (AskUserQuestion) — preferred the bounded core refresh over a full backfill…
- **…then "REFERENCE 누락 서브시스템 백필도 진행해줘"** — followed up authorizing the full backfill.
- Standing prefs (unchanged): Korean for user-facing reports; English for code/handoffs/sub-agent prompts;
  REFERENCE/ARCHITECTURE maintained in Korean; merge standing-delegated (squash on green CI, direct instruction);
  use sub-agents for parallel/bulky work with an explicit `model` (Opus here); honest gap-naming valued.

## Where We're Going

1. **Minor REFERENCE gaps still open (low value):** small 0.x API additions that have no standalone section —
   `App::push_scene`/`pop_scene`, `World::query2_mut`/`query3_mut`, wasm `save`/`load` localStorage parity note.
   These are mentioned in the 0.x changelog summary but not documented as their own entries. Add only if asked.
2. **Engine-hardening backlog (unchanged from seq-44):** per-OS (Win/Mac) gamepad input + the deferred
   analog-stick Y-sign hardware test (`gilrs-macos-xbox-no-input` memory); **crates.io publish** (irreversible,
   needs explicit go; publish `engine_reflect_derive` too).
3. **Optional:** the seq-43 focus-ring extensions (corner radius / pulse); the v10.x changelog-only features
   (Coroutine etc.) now have proper sections, so the doc is in good shape.

## Risks & Blockers

- **None outstanding** — both PRs merged green, tree clean. Docs accurately reflect v0.43.5.
- HTML correctness is **not** CI-gated (no HTML linter in the pipeline) — the manual anchor/tag/escape scan is
  the QA. Re-run it (the Python broken-anchor + tag-balance checks in this handoff's Evidence) after any future
  REFERENCE.html edit.
- `REFERENCE.html` body prose is still "as-of ~v9.3/v10.7" for the *older* subsystems — the header now says so
  explicitly, but a reader could still hit a pre-0.x example that drifted. None found, but not exhaustively
  re-verified against current signatures (the 6 NEW sections were source-verified; the old body was not).

## The 6 backfilled sections (#154) — anchors for the next editor

Each is a `<h2>` in REFERENCE.html body (before the changelog cluster), with `<h3>` subsections; all
source-verified against `src/` + the cited compiling example:

| `<h2 id>` | example(s) | sub-sections (`<h3 id>`) |
|---|---|---|
| `다이얼로그` | dialogue_demo/branching/portrait/quest | `다이얼로그-기본` (타이프라이터), `-분기`, `-ron` (DialogueTree+load_dialogue), `-조건효과` (DialogueVars/Cond/Effect + world-level `dialogue::advance`/`choose`), `-현지화`, `-초상화` |
| `타일맵-프로젝션` | iso_tilemap, hex_tilemap(_flat), hex_autotile, hex_blob_autotile | `-iso`, `-hex`, `-hex-flat`, `-autotile-투영무관`, `-blob64` (64-tile, tile index == 6-bit mask) |
| `ui-포커스-내비` | ui_focus | `-uifocus`, `-패스` (kbd+gamepad table), `-스틱히스테리시스`, `-링` (FocusRingStyle) |
| `webaudio-wasm` | web_audio | `-기본`, `-sfx` (+2D positional), `-버스` (manual duck), `-표면` (full method table) |
| `timescale` | juice_demo | `-realdt` |
| `기타-신규-컴포넌트` | coroutine_demo, tween_sequence, parallax_scroll, shader_material, animated_tiles, nine_slice | `신규-coroutine`/`-tween-sequence`/`-parallax`/`-shader-material`/`-animated-tiles`/`-nine-slice` |

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 329764a (#154) … 08b899a (#153) … 7802066 (#152 seq-44 handoff)
grep -m1 '^version' Cargo.toml  # 0.43.5 (unchanged — docs were docs-only)

# Re-run the REFERENCE.html QA after ANY edit to it (no HTML linter in CI):
python3 - <<'PY'
import re; h=open("REFERENCE.html").read()
ids=set(re.findall(r'id="([^"]+)"',h)); hrefs=re.findall(r'href="#([^"]+)"',h)
print("broken anchors:", [x for x in hrefs if x not in ids])     # expect []
print("pre", h.count('<pre>'),'/',h.count('</pre>'),
      "table", h.count('<table>'),'/',h.count('</table>'),
      "double-escape &lt; in code:", h.count('&amp;lt;'))         # expect balanced + 0
PY

# Key docs:
#   REFERENCE.html      (Korean API ref — header "v0.43.5 기준"; 6 new subsystem sections + 0.x changelog)
#   ARCHITECTURE.html   (Korean structure doc — "v0.43.5 기준" stamp, module map matches the refactor sweep)
#   docs/CHANGELOG.md   (per-release detail; REFERENCE's 0.x changelog points here)

# Next action (nothing required — external docs are current):
#   (A) minor REFERENCE gaps (push_scene/pop_scene, query2_mut/query3_mut) — only if asked, OR
#   (B) engine-hardening backlog: per-OS gamepad input, OR crates.io publish (explicit go required).
```

## Cross-cutting Gotchas (expensive-to-rediscover)

1. **Sub-agent HTML fragments arrive HTML-ESCAPED in the task-notification, but the transcript stores RAW HTML.**
   The completion notification rendered `&lt;h2 id=...&gt;` (escaped for display), which is NOT pasteable. Do
   NOT transcribe from the notification. Instead extract the final assistant message text directly from the
   agent's `.output` JSONL transcript via bash/python WITHOUT loading it into context:
   ```python
   # message.content[*].text of the LAST role=="assistant" line == the raw fragment
   for line in open(transcript):
       o=json.loads(line); m=o.get("message")
       if m and m.get("role")=="assistant":
           last="".join(c["text"] for c in m["content"] if c.get("type")=="text")
   open(out,"w").write(last)   # raw <h2>, not &lt;h2&gt;
   ```
   Verified: extracted text had `<h2 id=` (raw), zero `&lt;h2 id=` — the notification escaping was display-only.
   This sidestepped the "huge JSONL overflows context if Read" warning (bash extraction writes to a file).
2. **Integrate large multi-fragment HTML with a script, not Edit.** A Python pass inserted 6 bodies before the
   changelog `<h2>` anchor and 6 TOC links before the changelog nav link — both anchors are distinct strings
   (`<h2 id="변경-이력-v91v93">` in body vs `href="#변경-이력-v91v93"` in nav), so `.replace(..., 1)` is safe.
3. **Always run a broken-anchor + tag-balance scan after editing REFERENCE.html.** A sub-agent invented a
   cross-link `#audiomanager` that didn't exist (real id is `오디오`); the scan (every `href="#X"` must have a
   matching `id="X"`) caught it. Also check `<pre>`/`<table>`/`<code>` open==close and that code blocks are
   single-escaped (`&lt;`, never `&amp;lt;` which renders literally, never raw `<` which breaks the tag).
4. **Korean-keyword grep under-reports doc coverage.** Searching REFERENCE for `나인`/`코루틴` returned 0 even
   though NineSlice/Coroutine ARE present (as English code terms in changelog entries). Confirm a "missing"
   section by checking the actual `<h2>/<h3>` heading list + code-term refs, not just a translated keyword.
5. **`gh pr checks --watch` exits immediately ("no checks reported") if run <~30s after `gh pr create`** —
   poll `gh pr checks` until a check registers, THEN `--watch` (carried over from seq-44; held all session).

## Process / Versioning Notes

- Docs-only changes → no `/ship`, no version bump (the package is already v0.43.5; the doc stamps now match).
- Each docs PR went through the same flow as code PRs: branch → commit → push → PR → poll-then-watch CI →
  squash-merge on 4/4 green → ff `main`. Merge standing-delegated (direct instruction, never an AskUserQuestion
  option — the Korean-classifier-safe pattern).
- The 6-sub-agent fan-out (drafting) + single-thread integration is a reusable pattern for backfilling a large
  doc accurately: parallel source-grounded drafting, deterministic script integration, manual structural QA.

---

## Session Closed
**Closed at:** 2026-06-20 (KST)
**Commits:** #153 (`08b899a`, core doc refresh) → #154 (`329764a`, REFERENCE subsystem backfill); this handoff
committed separately.
**Session status:** Goal met — external docs current to v0.43.5, merged. Handed off to next session.
