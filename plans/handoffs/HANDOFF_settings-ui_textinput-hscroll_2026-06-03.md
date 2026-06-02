# TextInput single-line horizontal scroll + IME max_len honesty — merged as v1.3.0

**Date:** 2026-06-03
**Status:** COMPLETED — PR #6 merged to `main` (`a2eb352`), v1.3.0 released, branch deleted
**Bead(s):** none (no beads system in this repo)
**Epic:** skeleton-engine playable-examples dogfooding loop
**Chain:** `settings-ui` seq `4`
**Parent:** `HANDOFF_settings-ui_macos-drag-freeze_2026-06-03.md` (seq 3)
**Prior chain:** `HANDOFF_settings-ui_2026-06-02.md` (seq 1) > `HANDOFF_settings-ui_engine-ui-fixes_2026-06-02_2.md` (seq 2) > `HANDOFF_settings-ui_macos-drag-freeze_2026-06-03.md` (seq 3) > this (seq 4)

---

## Since Last Handoff

Seq 3 (macOS drag-freeze, v1.2.1) closed one deferred item and listed three remaining: overlay
caret, **TextInput horizontal scroll**, real OS fullscreen. This session executed the h-scroll item.

- Onboarded from seq 3, verified baseline (260→still green), chose deferred item **(a) TextInput
  horizontal scroll** per the user ("a로 완결성 챙기자").
- Used `EnterPlanMode` + an `AskUserQuestion` scope round → scope set to **h-scroll + the bundled
  IME-at-`max_len` fix**, proof via a **dedicated long-text field** in `settings_menu`.
- Shipped + merged as **v1.3.0** (minor — new public API). Two **CI-only** failures surfaced and
  were fixed (a pre-existing flaky prefab test race; a missed `cargo fmt` after the race fix).
- Deferred ledger now: overlay caret, real OS fullscreen remain; `.app` bundler still a separate
  user-raised track. → these drive the paired PLAN file.

## Reference Documents

- `CLAUDE.md` — conventions, module map (version header now `v1.3.0`).
- `docs/VISION.md` — dogfooding loop (a fix isn't done until a playable example exercises it).
- `docs/NEXT_WORK.md` — deferred-item follow-up note (updated: h-scroll + IME now done).
- `docs/PATTERNS.md` — UI system order (`LayoutSystem` → `UiSystem`), render-layer separation.
- Approved plan for this session: `~/.claude/plans/a-linked-matsumoto.md` (gitignored, overwritten from the seq-3 drag plan).

## The Goal

Resolve the next deferred follow-up from the settings-ui dogfooding chain: `TextInput` had no
horizontal scroll, so long single-line values wrapped (then clipped vertically in the one-line-tall
field) and the caret at the end went out of view; bundled with it, IME composition at `max_len`
showed a phantom, uncommittable preedit. Deliver single-line horizontal scrolling that keeps the
caret visible, make the IME honest at capacity, and prove both in a real playable example — without
adding a new render pipeline. End state: merged, released (v1.3.0), example extended, ledger updated.

## Where We Are

- Working tree **clean**, on `main` at `a2eb352` (= `origin/main`, ahead/behind 0/0). Branch
  `feat/textinput-hscroll` merged (merge commit) and **deleted** (local + remote + pruned).
- **PR #6 merged.** Final CI all green: Test (native) 2m32s, Build (WASM) 26s, Rustdoc 38s,
  Package dry-run 4m36s. `mergeStateStatus: CLEAN`.
- **v1.3.0** in `Cargo.toml`, `Cargo.lock`, `CLAUDE.md` header; `docs/CHANGELOG.md` has a `## 1.3.0`
  section (Added / Fixed / Example).
- `cargo test --lib` = **262 passed** (was 260; +`remaining_capacity_tracks_max_len`,
  +`caret_display_offset_accounts_for_preedit`).
- **Renderer** (`src/renderer/text.rs`): `DrawText` gained `single_line_caret: Option<usize>` +
  builder `with_single_line_caret`; render loop branches single-line (`Wrap::None`, unbounded width)
  and computes a horizontal scroll offset; free fn `caret_x(buf, caret_byte)` measures via
  `Buffer::layout_runs()`. `TextArea.left = position.x - scroll`, clip via `TextBounds`.
- **UiSystem** (`src/ui/system.rs`): passes `with_single_line_caret(caret_byte)` —
  `caret_display_offset()` when focused, `0` when unfocused; gates IME preedit on
  `remaining_capacity() >= ime_preedit.len()`.
- **TextInput** (`src/ui/text_input.rs`): added `remaining_capacity()` and `caret_display_offset()`
  + 2 unit tests.
- **Example** (`examples/games/settings_menu/settings_menu.rs`): dedicated narrow long-text field
  (200px, prefilled "The quick brown fox…", `max_len` 48) in the Settings scene, standalone (not in
  the panel) at (500,434).
- **Pre-existing flaky test fixed** (`src/prefab.rs`): `tmp_path` now keys the temp dir by file name
  so parallel cleanup can't race.
- Manual macOS verification by the user: **"확인됨"** — scroll, caret-follow, edge clipping, IME-at-max all good.
- `rust-survivors` builds clean against 1.3.0 (the new `DrawText` field is source-compatible).

## What We Tried (Chronological)

1. **Onboarding (seq 3 → 4).** Read parent handoff; verified 260 tests, identifiers present, no stale refs. Presented next-direction candidates; user chose **(a) TextInput h-scroll** ("완결성 챙기자").
2. **Code exploration (read-only).** Traced the TextInput render path: `UiSystem` builds `DrawText` from `display_with_caret` with `with_bounds`; `text.rs` lays it out with `Wrap::WordOrGlyph` + `TextBounds` clip → long text **wraps** and clips vertically. Confirmed glyphon supports `TextBounds` (clip), `TextArea.left` (shift), `Buffer::layout_runs()` (measure) → h-scroll feasible with **no new pipeline**; measurement must live in `text.rs` (owns `FontSystem`).
3. **`EnterPlanMode` + scope `AskUserQuestion`.** Found the deferred item bundles two defects (no h-scroll; IME phantom preedit at `max_len`). Two scope questions asked:
   - *범위* — "h-scroll + IME max_len 수정" vs "h-scroll만" → user chose **h-scroll + IME** ("완결성").
   - *증명 표면* — "기존 name 필드 활용" vs "전용 긴-텍스트 필드 추가" → user chose **전용 긴-텍스트 필드**.
   Traced IME flow (`src/app.rs`): `Ime::Preedit` → `InputState::set_ime_preedit`; `Ime::Commit` → `InputState::push_text` (into the `text_input_chars` buffer) + `clear_ime_preedit`. `system.rs` each frame sets `ti.preedit = ime_preedit` then consumes `chars` via `insert_char`. Since `insert_char` is a no-op at `max_len`, a full field could never commit a composition while the preedit kept rendering — the "uncommittable preedit".
4. **Verified `DrawText` is builder-only** (no struct literals in engine or rust-survivors) → adding a field defaulted in `::new` is source-compatible. Wrote plan, `ExitPlanMode` → approved.
5. **Implementation** (branch `feat/textinput-hscroll`, in order): (a) `TextInput::remaining_capacity` + `caret_display_offset` + 2 tests; (b) `DrawText.single_line_caret` field + `with_single_line_caret` builder; (c) `caret_x` helper; (d) single-line branch in the buffer build (width `None`, `Wrap::None`) + scroll offset carried in the `(Buffer, DrawText, f32)` tuple + `TextArea.left` shift; (e) UiSystem passes `with_single_line_caret(if focused { caret_display_offset() } else { 0 })` and gates preedit; (f) example long-text field. Built lib (clean) → tests (262) → example (clean) at each milestone.
6. **clippy `never_loop`** on the first `caret_x` (the outer `for run` always returned on iter 1) → rewrote with `layout_runs().next()` + let-else.
7. **Full local verify green:** 262 tests, native+wasm clippy `-D warnings`, wasm build, rust-survivors. User ran the example on macOS → **"확인됨"**.
8. **Docs + version.** CHANGELOG `## Unreleased`, NEXT_WORK update; asked version → user chose **1.3.0 (minor)** (new public API). Bumped Cargo.toml/lock + CLAUDE.md; promoted CHANGELOG to `## 1.3.0`. Committed, pushed, PR #6.
9. **CI failure #1 — Test (native) FAILED** on `prefab::tests::scene_hierarchy_roundtrip`: `save should succeed: Io(NotFound)`. **Unrelated to the diff.** Root cause: three prefab tests shared `engine-prefab-test-{pid}/` and each `remove_dir(parent)` in cleanup; in parallel, one deletes the shared dir between another's `create_dir_all` and `fs::write`. My new tests shifted scheduling and exposed it (passed on #5 by luck). Fixed `tmp_path` to key the dir by file name (unique per test). Ran `cargo test --all-targets` 3× green.
10. **CI failure #2 — `cargo fmt --check` FAILED.** The Test (native) job runs fmt first; I'd edited `prefab.rs` after my last `cargo fmt`. The `format!("…{}-{}…")` wanted multi-line. Ran `cargo fmt`, committed `style:` fix.
11. **CI green** (Test native 2m32s incl. the race fix), `CLEAN` → `gh pr merge 6 --merge --delete-branch`, fetch --prune, synced main.

## Key Decisions

- **Measurement in the renderer, not the UI system.** `text.rs` owns `FontSystem`; `UiSystem` only passes the caret byte offset. Stateless: scroll recomputed each frame from `caret_x` (no scroll state stored on `TextInput`).
- **Opt-in via `DrawText.single_line_caret: Option<usize>`** rather than a separate "no-wrap" flag — one field both selects single-line layout and carries the caret position to scroll to. `None` = unchanged wrapping for labels/buttons.
- **Caret anchor by focus:** focused → `caret_display_offset()` (scroll to caret); unfocused → `0` (show the start). Keeps idle fields readable from the beginning.
- **IME honesty rule = `remaining_capacity() >= preedit.len()`** (not just `> 0`): show the preedit only if it would actually commit. Coarser `> 0` would still show an uncommittable multi-byte composition when 1–2 bytes remain.
- **`type_complexity` left as-is elsewhere; kept inline caret** — overlay caret is a *separate* deferred item; h-scroll measures the existing inline `|`/space caret, no overlay needed.
- **Version 1.3.0 (minor)** — additive public API (DrawText field+builder, 2 TextInput methods). User-confirmed over 1.2.2 patch.
- **Flaky prefab test: fix via per-test unique dir**, not by serializing tests or dropping cleanup — keeps parallelism and cleanup, removes the shared mutable dir entirely.
- **Merge commit (`--merge`)** mirroring #3/#4/#5.

## Evidence & Data

### Commits on the branch (oldest → newest)
| Hash | Summary |
|---|---|
| `02ec251` | feat(ui): TextInput single-line horizontal scroll + IME max_len honesty (v1.3.0) |
| `c99d9fb` | test(prefab): give each tmp_path test its own dir to kill a parallel race |
| `1815c2f` | style: rustfmt prefab tmp_path |
| `a2eb352` | Merge pull request #6 |

### CI iterations (PR #6)
| Run | Test (native) | Cause | Fix |
|---|---|---|---|
| 1 (`02ec251`) | FAIL | `prefab` flaky race — `Io(NotFound)` on shared temp dir | unique per-test dir (`c99d9fb`) |
| 2 (`c99d9fb`) | FAIL | `cargo fmt --check` (race fix unformatted) | `cargo fmt` (`1815c2f`) |
| 3 (`1815c2f`) | PASS (2m32s) | — | merged |

### TextInput render: before → after
| Aspect | Before | After |
|---|---|---|
| Wrap | `Wrap::WordOrGlyph`, width = field width | `Wrap::None`, width `None` (one line) |
| Long text | wraps; 2nd line clipped vertically in 1-line field | scrolls horizontally, clipped at both edges |
| Caret at end | off-screen / on wrapped line | kept visible (`scroll` follows `caret_x`) |
| Unfocused field | same wrap | anchored to start (caret offset `0` → scroll 0) |
| IME at `max_len` | preedit shown but commit dropped (phantom) | preedit hidden when it won't fit |
| `DrawText` for labels/buttons | unchanged | unchanged (`single_line_caret = None`) |

### Verification matrix (final)
| Check | Result |
|---|---|
| `cargo test --lib` | 262 passed (+2) |
| `cargo test --all-targets` (CI cmd, 3× local) | green |
| `cargo fmt --check` / native `clippy --all-targets -D warnings` | clean |
| wasm `clippy --lib --example settings_menu_game -D warnings` | clean |
| wasm build (lib + example) | passes |
| `rust-survivors` cargo check | clean (additive) |
| CI (Test/WASM/Rustdoc/Package) | all pass, CLEAN |

### Diff stat (PR #6): 10 files, +183 / −16.

### Core algorithm (the part most expensive to re-derive)
```rust
// src/renderer/text.rs — caret x for a single-line buffer (left-aligned)
fn caret_x(buf: &Buffer, caret_byte: usize) -> f32 {
    let Some(run) = buf.layout_runs().next() else { return 0.0; };
    for glyph in run.glyphs.iter() {
        if glyph.start >= caret_byte { return glyph.x; }
    }
    run.line_w
}
// in the render loop, when single_line.is_some():
//   width = None;  buf.set_wrap(Wrap::None);
//   scroll = (caret_x(&buf, caret_byte) - (field_w - margin)).max(0.0)   // field_w = bounds.x, margin = size
// in the TextArea build:
//   left = d.position.x - scroll;   bounds: TextBounds (unchanged — clips both edges)
```

### CHANGELOG `## 1.3.0` (verbatim — the released surface)
> ### Added
> - `TextInput` single-line **horizontal scrolling**: long values no longer wrap or clip out of view…
>   New `DrawText` opt-in `with_single_line_caret(caret_byte)` drives it — the renderer measures the
>   caret x via glyphon `Buffer::layout_runs()` and shifts the `TextArea` left, clipped by `TextBounds`
>   (no new render pipeline).
> - `TextInput::remaining_capacity()` and `TextInput::caret_display_offset()` helpers.
>
> ### Fixed
> - IME at `max_len`: composing input when the field is full no longer shows a phantom, uncommittable
>   preedit. `UiSystem` only displays the IME preedit while it still fits (`remaining_capacity() >= preedit.len()`).
>
> ### Example
> - `settings_menu_game` Settings scene gained a dedicated narrow long-text field (prefilled past its
>   width, `max_len` 48) exercising horizontal scroll, caret-follow, and IME-at-capacity.

## Code Analysis

- **`DrawText.single_line_caret: Option<usize>`** + `with_single_line_caret(caret_byte)` (`src/renderer/text.rs`). `new()` defaults it to `None` → all existing call sites (labels/buttons/HUD/rust-survivors) unchanged.
- **`fn caret_x(buf: &Buffer, caret_byte: usize) -> f32`**: `layout_runs().next()` (single line = one run), returns first `glyph.x` with `glyph.start >= caret_byte`, else `run.line_w`. Assumes left alignment.
- **Render loop**: `single_line.is_some()` → width `None`, `Wrap::None`; after `shape_until_scroll`, `scroll = (caret_x - (field_w - margin)).max(0.0)` with `field_w = bounds.x`, `margin = size`. Carried in the `(Buffer, DrawText, f32)` tuple; applied as `TextArea.left = d.position.x - *scroll`, `TextBounds` unchanged (clips both edges).
- **`TextInput::remaining_capacity()`** = `max_len.saturating_sub(text.len())`; **`caret_display_offset()`** = `cursor + preedit.len()` (caret byte position within `display_with_caret` output `head+preedit+caret+tail`).
- **UiSystem TextInput pass** (`src/ui/system.rs`): preedit gate `remaining_capacity() >= ime_preedit.len()`; render passes `with_single_line_caret(if focused { caret_display_offset() } else { 0 })`.
- **`tmp_path`** (`src/prefab.rs`): `temp_dir()/engine-prefab-test-{pid}-{name}/{name}` — unique parent per test.
- Versions unchanged: winit 0.30.13, wgpu 22.1.0, glyphon 0.6.

### glyphon render-path facts (from exploration — useful for the overlay-caret phase)
- The text renderer (`src/renderer/text.rs`) is the *only* glyphon consumer; it builds one `Buffer`
  per `DrawText`, sets wrap width via `Buffer::set_size(font_system, Some(w), Some(h))`, wraps with
  `set_wrap`, shapes with `shape_until_scroll`, then a `Vec<TextArea>` with `left/top/bounds/default_color`.
- `TextBounds { left, top, right, bottom }` is the scissor clip rect — already used to clip TextInput
  text to the field; reused unchanged for h-scroll.
- `TextArea.left/top` position the buffer in screen space and may be negative (how the scroll shifts text left).
- `Buffer::layout_runs()` yields `LayoutRun { line_w, glyphs: [LayoutGlyph { start, end, x, w, .. }] }`
  — `glyph.start` is the byte index, `glyph.x` the pen x. This is the measurement primitive an
  overlay caret would reuse (draw a quad at `caret_x`).
- There is **no quad/rect pipeline inside `text.rs`** — `DrawRect` (UI rects) renders via a separate
  path (`UiQueue`/rect renderer). An overlay caret likely emits a thin `DrawRect` at `position.x +
  caret_x - scroll`, clipped to the field, rather than threading a quad through glyphon.

### Example demo-field rationale
- Placed standalone at `(500, 434)` size `200×32` (not in the vertical `Panel`, which would force full
  width) so the field is **deliberately narrow** → overflow/scroll is obvious. Prefilled with the
  43-byte "The quick brown fox jumps over the lazy dog" (already overflows 200px). `max_len` 48 leaves
  ~5 bytes so the IME-at-capacity path is reachable (compose Korean → preedit hidden when it won't fit).
- A static `Label` header ("Long-text field — type past the edge (scrolls)") sits above it; no locale
  keys added (kept the 3-locale RON untouched).

## Files Changed (PR #6)

### Source
- `src/renderer/text.rs` — `single_line_caret` field+builder; `caret_x` helper; single-line layout + horizontal scroll.
- `src/ui/system.rs` — pass caret offset; IME preedit capacity gate.
- `src/ui/text_input.rs` — `remaining_capacity` + `caret_display_offset` + 2 tests.
- `src/prefab.rs` — `tmp_path` unique per-test dir (flaky-race fix).

### Example / build / docs
- `examples/games/settings_menu/settings_menu.rs` — dedicated long-text field.
- `Cargo.toml` / `Cargo.lock` — 1.2.1 → 1.3.0.
- `CLAUDE.md` — header v1.3.0.
- `docs/CHANGELOG.md` — `## 1.3.0`; `docs/NEXT_WORK.md` — h-scroll/IME marked done.

## User Feedback & Preferences

- **Korean conversation, English docs/artifacts.** Held throughout.
- Picks direction tersely ("a로 완결성 챙기자"), values **thoroughness/completeness** ("완결성") — chose the broader scope (h-scroll **+** IME) when offered.
- **QA's by playing on macOS**, confirms with a terse "확인됨".
- Engages on **version** decisions (chose 1.3.0 minor) — wants semver right.
- Delegates merge+cleanup once CI passes ("CI 통과하면 머지하고 브랜치 정리해줘") — but expects CI failures to be diagnosed and fixed, not worked around.
- Earlier-raised side interest: a **macOS `.app` bundler** (separate track, not started).

## Where We're Going

- **Remaining deferred items** from the settings-ui chain (none scheduled), see paired `PLAN_settings-ui_textinput-hscroll_2026-06-03.md`:
  - **Real OS fullscreen** — the Settings checkbox stores a `fullscreen: bool` preference only; wire it to an actual winit `Window::set_fullscreen`. Smallest, self-contained.
  - **Overlay caret** — the inline `|`/space caret still shifts sub-pixel on blink; proper fix is a renderer caret-**quad** drawn at the measured glyph x (glyphon has no quad pipeline today → needs a small rect/quad path or reuse of `DrawRect`). Biggest, has design uncertainty.
- **macOS `.app` bundler** — separate track: `cargo-bundle`/`cargo-packager` + `[package.metadata.bundle]` + `Info.plist`.
- **Fresh dogfooding candidates** (`docs/NEXT_WORK.md`): 2D lighting (highest-value untested), `BlendTree1D`, `Timeline`/cutscene, `PostProcessConfig`, physics joints, `RenderTarget`, networking.

## Pointers for the Next Phases (fullscreen / overlay caret)

- **Fullscreen** lives entirely in the example today: `Settings.fullscreen: bool` field; the checkbox
  `fullscreen_cb` is spawned in `SettingsScene::on_enter` (`examples/.../settings_menu.rs`, ~line 602)
  with `LocalizedText::new("opt.fullscreen")` (locale strings still say "(preference)"). `SettingsSystem`
  echoes other widgets; the checkbox's toggle currently only updates `Settings.fullscreen`. To make it
  real: on toggle, request `winit::window::Window::set_fullscreen(Some(Fullscreen::Borderless(None)))`
  / `None`. The engine exposes no fullscreen API yet → likely add one (e.g. a `Fullscreen` request on
  a resource the app reads, mirroring `PendingResize` in `src/app.rs`, since the example can't touch
  the `Window` directly). Update the 3 locale strings to drop "(preference)".
- **Overlay caret** — reuse `caret_x` (added this session). The blinking inline `|`/space slot in
  `display_with_caret` still shifts trailing glyphs sub-pixel; a quad caret would drop the inline
  caret char and draw a thin `DrawRect` at `field_x + caret_x - scroll`, clipped to the field. See the
  "glyphon render-path facts" above — `DrawRect` is the existing rect path, not glyphon.

## Risks & Blockers

- **None blocking.** Work merged and released.
- **Pre-push checklist (learned the hard way this session — two CI round-trips):**
  1. `cargo fmt` *after every edit*, then `cargo fmt --check` — the Test (native) CI job runs fmt
     first and fails the whole job on a diff (cost us run #2).
  2. `cargo test --all-targets` (not just `--lib`) — the prefab race only manifests under
     `--all-targets` parallel scheduling; `--lib` alone hid it locally (cost us run #1).
  3. native + wasm `clippy --all-targets -- -D warnings`, wasm build, `rust-survivors` check.
- **Parallel-test hygiene:** any test writing to a shared temp path must use a unique per-test dir
  (now fixed for prefab; watch for the same pattern in `save`/`history`/asset tests).
- **clippy toolchain drift** keeps surfacing lints on wasm-only code; watch on future PRs.

## Success Criteria (this session — all met)

- TextInput long text scrolls horizontally, caret stays visible, both edges clip — **user-confirmed**.
- IME at `max_len` no longer shows an uncommittable preedit — **user-confirmed**.
- 262 lib tests, native + wasm clippy `-D warnings`, wasm build, rust-survivors all green; CI CLEAN.
- Additive only (no public API removed/changed); released as v1.3.0; example exercises the fix.

## Open Questions

- Overlay caret: does the renderer get a `DrawRect`-style quad at a measured glyph x, or a new glyphon path? `caret_x` (added this session) already measures glyph x — likely reusable. To be resolved in the PLAN's caret phase.
- Fullscreen: should the engine expose a general `Window` control API (fullscreen, title, cursor grab) or a one-off `Fullscreen` request resource? Mirroring `PendingResize` is the minimal path; a broader `WindowCommand` enum is the fork-friendly path. Decide in the PLAN's fullscreen phase.
- Locale strings for fullscreen currently say "(preference)" / "(설정값)" / "(preferencia)" — must change when it becomes real (3 keys in the RON).

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git switch main && git pull              # expect a2eb352 or later
cargo test --lib                         # expect 262 passed

# See the shipped h-scroll in play:
cargo run --example settings_menu_game   # Settings → bottom-right long-text field

# Reference
#   plans/handoffs/HANDOFF_settings-ui_textinput-hscroll_2026-06-03.md (this)
#   plans/handoffs/PLAN_settings-ui_textinput-hscroll_2026-06-03.md (next work)
#   docs/NEXT_WORK.md, docs/VISION.md

# Key source files for the next phases
#   src/app.rs                              (PendingResize pattern; where a Fullscreen request would be read)
#   examples/games/settings_menu/settings_menu.rs  (fullscreen_cb ~line 602; SettingsSystem echo)
#   src/renderer/text.rs                    (caret_x — reuse for overlay caret)
#   src/ui/text_input.rs                    (display_with_caret inline caret)

# Verify starting state
cargo test --lib    # 262 passed
cargo run --example settings_menu_game   # confirm h-scroll field works

# Next action: execute the paired PLAN — Phase 1 (real OS fullscreen),
#   first concrete step = add a window-control request path in src/app.rs (mirror PendingResize).
```
