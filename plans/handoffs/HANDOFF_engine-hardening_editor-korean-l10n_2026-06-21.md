# Editor Korean (CJK) support + Korean-default editor localization (+ rounded UI corners)

**Date:** 2026-06-21 (KST)
**Status:** COMPLETED — both features merged green to `main`; tree clean. Memory updated to seq 54.
**Bead(s):** none (repo uses `plans/handoffs/`)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `54`
**Parent:** `HANDOFF_engine-hardening_wrap-skill-memory-hygiene_2026-06-20.md` (seq 52)

> This handoff covers a single long session that shipped **two** features:
> - **seq 53** — rounded corners for `DrawRect` + the keyboard-focus ring (PR #172, v0.45.0) plus a
>   `docs/PATTERNS.md` note (PR #173). seq 53 shipped **without its own handoff file**, so it is
>   recorded here for the chain record.
> - **seq 54** — Korean (CJK) font support + a Korean-by-default localization for the in-game editor
>   (PR #174, v0.46.0). This was the main ask of the session.
>
> Parent is seq 52 (not 53) because seq 53 never got a handoff; the memory `engine-current-state.md`
> already uses "seq 53" for corner-radius and "seq 54" for the editor l10n — this file matches that.

---

## The Goal

1. **seq 53 (carried over from the prior session's "what's next" menu).** The user picked the
   seq-51-deferred **corner-radius** renderer feature ("3 완료 후 2 진행" = do corner-radius, then the
   `docs/PATTERNS.md` candidate-C note). seq-51 had *dropped* corner-radius because `DrawRect` was a
   plain quad with no SDF; this session built the SDF path.
2. **seq 54 (the session's headline request).** "F2 에디터에서 한글이 ㅁ(두부)로 나온다. 엔진에 기본
   폰트를 추가하면서 에디터 전체를 한글화해라. 폰트는 게임 프로젝트 asset 폴더에 있다." → bundle a
   Korean font so the egui editor renders Hangul, and localize the whole editor to Korean. The user
   chose the **i18n-layer, Korean-default** approach (over hard-coding Korean or English-default).

## Where We Are

- `main` @ **`a087011`**, tree **clean**, package **v0.46.0**, CLAUDE.md header **v1.6.103**.
- All gates green (`./scripts/verify.sh`: fmt / clippy `-D warnings` / wasm `build` / `test --all-targets`
  / rustdoc `-D warnings`; plus `cargo clippy --target wasm32 --lib -D warnings` and `cargo package`).
- Merged this session: **#172** (corner-radius, v0.45.0) · **#173** (PATTERNS docs) · **#174** (editor
  Korean l10n, v0.46.0). All squash-merged on green CI (merge authority is standing-delegated).
- **Wishlist board** (`../dungeon-merchant/docs/engine-wishlist.md`): ACTIVE still **empty**, next free
  ID **EW-002**. (The editor-l10n request came directly from the user, not via the board.)

## seq 53 — Rounded corners for `DrawRect` + the focus ring (PR #172, v0.45.0)

**Design (sprite hot path untouched):** UI rects + the focus ring previously shared the **sprite**
pipeline (`InstanceRaw` + `shaders/sprite.wgsl`). Rather than add SDF cost there, the UI primitive
pass now uses its **own** pieces:
- `UiInstanceRaw` (model/color/uv + `px_size` + `corner = [radius, border]`) in `geometry.rs`.
- A dedicated `ui_pipeline` in `SpriteRenderer::new` (own shader, reuses the camera + texture BGLs;
  `vertex_layout.clone()` since `VertexBufferLayout` isn't `Copy`).
- `src/renderer/shaders/ui.wgsl` — rounded-rect SDF: filled when `border == 0`, an inset outline ring
  otherwise. **`corner == (0,0)` takes a shader fast path** that returns `texture*color` exactly as
  before → every existing `DrawRect`/`DrawImage` + the sharp 4-bar focus ring are **byte-identical**.
- `DrawRect::with_corner_radius(r)` / `with_border(w)` builders (default 0/0).
- `FocusRingStyle::corner_radius` (default 0.0); `push_ring` emits one rounded outline rect when
  `corner_radius > 0`, else the 4 bars.
- Example **`examples/ui_rounded.rs`** — rounded card panel + three fill modes (sharp/rounded-fill/
  rounded-outline) + a rounded focus ring. **macOS screenshot-verified** (all 4 SDF cases correct).
- Also converted `examples/{diagonal_pathing,loading_bar}.rs` direct `DrawRect { .. }` struct literals
  to the `DrawRect::new(..)` builder (future-proof against added fields).

**PR #173** (docs-only): promoted candidate C → `docs/PATTERNS.md` "Time-driven System animation (no
global clock)" note — accumulate a wrapped `dt` clock in a System field, thread elapsed time into a
pure sub-pass helper (cursor-blink + focus-ring pulse are the two instances).

## seq 54 — Editor Korean support + Korean-default localization (PR #174, v0.46.0)

**Root cause of the □ tofu:** the F2 editor is **egui**, and egui's default fonts cover only Latin +
Cyrillic. (The engine's *own* text renderer already handles Korean via cosmic-text + the game's font;
this bug was egui-specific.) Two parts:

### Part 1 — Bundled Korean font (the actual fix)
- Copied **`NotoSansKR-Regular.ttf`** (6.2 MB, from `../dungeon-merchant/assets/fonts/`) into the engine
  `assets/fonts/`.
- `src/debug_ui.rs`: `const KOREAN_FONT = include_bytes!(.../assets/fonts/NotoSansKR-Regular.ttf)` +
  `install_korean_fallback(ctx)` called inside **`DebugUi::new_with_ctx`** (covers both ctx-creation
  sites: `app/window.rs` native + `app.rs`). Uses egui 0.34's `ctx.add_font(FontInsert::new(..,
  FontData::from_static(KOREAN_FONT), [InsertFontFamily{Proportional, Lowest}, {Monospace, Lowest}]))`
  — **`FontPriority::Lowest`** so Latin/Cyrillic keep the default font (unchanged metrics) and Noto is
  consulted only for missing glyphs (Hangul/CJK). Types live at `egui::epaint::text::{FontInsert,
  InsertFontFamily, FontPriority}` (NOT re-exported at egui top level).
- This also fixes Korean **data** in the editor (RON data-table cell values, entity `Tag`s).

### Part 2 — Editor localization layer (`src/app/editor/i18n.rs`)
- `tr(en, ko) -> &'static str` — **English is the source of truth** (first arg + fallback), Korean
  inline. No central catalog (translations live at the call site → minimal-coordination parallel edits).
- `EditorLocale {English, Korean}` (`#[default] Korean`), thread-local active locale set each frame in
  `update_editor_ui` via `set_locale(self.editor.locale)`; `tr` reads it (no plumbing through the deep
  egui closures). `EditorLocale::label()` ("EN"/"한국어") + `toggled()`.
- Persisted: `EditorSettings::locale` (`#[serde(default)]`, RON) + `EditorState::locale`; a toolbar
  **EN/한국어 toggle button** (`docked.rs`) flips + saves immediately.
- **~130 user-facing strings** wrapped via `tr()` across the editor, done by **5 parallel sonnet
  subagents** over disjoint files: docked.rs (52) · ui/mod.rs (18) · state_machine_panel.rs (16) ·
  timeline_panel.rs (18) · data_table_panel.rs (13) · component_registry.rs (10) · audio_panel.rs (3);
  tile_paint.rs had no UI strings. Each agent got a shared Korean glossary + rules (handle `format!`,
  keep emoji/icons, don't translate ids/keys/paths/data-derived labels, `tr(..).to_string()` where an
  owned String is needed).
- **macOS F2 screenshot-verified**: the docked editor renders fully in Korean — 일시정지·스텝·스냅·
  그리드·경계·패스·설정·한국어·저장·불러오기·종료 / 엔티티·씬 / 인스펙터·환경광 / 에셋·데이터 테이블·오디오.

## Key Decisions

- **seq-54 i18n approach = AskUserQuestion → "i18n layer, Korean default".** Rejected: hard-coding
  Korean (would make the open-source editor Korean-only, against VISION's fork-friendly skeleton) and
  English-default (user would have to toggle every time). The toggle keeps it fork-friendly.
- **`tr(en, ko)` inline, not a keyed central catalog.** Lets 5 agents edit disjoint files with zero
  shared-file contention, and a forker reads the English at a glance.
- **`FontPriority::Lowest` fallback, not `set_fonts` replacement.** Keeps egui's default Latin look;
  Noto only fills missing glyphs. `add_font` is idempotent (skips if name already present).
- **i18n module cross-platform, re-exports split.** `tr` is used by the **shared wasm overlay path**
  (ui/mod.rs has egui code active on wasm — e.g. `ui.strong(tr("Components","컴포넌트"))`), so i18n is
  NOT wasm-gated. But `set_locale`/`EditorLocale` are only used by the native toolbar/settings → split
  the re-export: `pub use i18n::tr;` always, `#[cfg(not(wasm32))] pub use i18n::{set_locale,
  EditorLocale};`. Plus `#![cfg_attr(target_arch="wasm32", allow(dead_code))]` in i18n.rs for
  `set_locale`/`label`/`toggled` (native-only-used).
- **`assets/fonts/**` added to Cargo.toml `include`** (see Gotchas) — chose to package all fonts (not
  just NotoSansKR) so the latent DejaVu-on-wasm-publish gap is also closed.

## Gotchas & Discoveries (expensive to re-discover)

1. **Un-gated `include_bytes!` from `assets/` MUST be in Cargo.toml `include`.** This caused a **red
   Package-dry-run on the first #174 push** (`cargo package --locked` verify-build, fails in ~36s
   reading the missing font). Cargo's `include` list is exclusive — only listed patterns are packaged;
   `assets/` was NOT listed. **DejaVu hid this:** its `include_bytes!` is `#[cfg(target_arch="wasm32")]`,
   so the *native* package verify-build never needed `assets/`. My `KOREAN_FONT` is un-gated (editor
   runs native + wasm overlay) → native package build needs the file. Fix: add `"assets/fonts/**"` to
   `include`. Verify locally with `cargo package --allow-dirty --locked` (does the verify build).
2. **Package is now 11.7 MB compressed > crates.io's 10 MB publish limit.** `cargo package` (the CI
   dry-run) does NOT enforce the limit — only actual `cargo publish` does. So CI is green, but **before
   any crates.io publish (deferred backlog) the 6.2 MB CJK font must be subsetted** (e.g. `pyftsubset`
   to common Hangul) or the limit raised.
3. **egui 0.34 font API:** add a fallback with `ctx.add_font(egui::epaint::text::FontInsert::new(name,
   egui::FontData::from_static(bytes), vec![InsertFontFamily{family, FontPriority::Lowest}, ..]))`.
   `FontInsert`/`InsertFontFamily`/`FontPriority` are under `egui::epaint::text::`, not `egui::`.
4. **Stale rust-analyzer diagnostics throughout** (E0061/E0593 in timeline_panel.rs, inactive-cfg in
   mod.rs) lagged behind agent edits — `cargo check` reported **0 errors**. Trust cargo/CI, not the IDE
   squiggles (existing [[engine-current-state]] gotcha, reconfirmed hard this session).
5. **`cargo fmt` after agent edits is required** — the 5 agents' wrapping wasn't fmt-clean; the first
   verify failed on `fmt --check` only. Run `cargo fmt` then re-verify.
6. **wasm clippy `--lib -D warnings` (a CI step) catches unused imports/dead-code** that the plain wasm
   `build` does not. The split re-export + cfg_attr above were needed to pass it.
7. **macOS windowed-playtest flake:** the first `osascript ... key code 120` (F2) sometimes hits a
   transient "process index invalid (-1719)" right after launch; re-`set frontmost` + resend F2 after a
   longer settle. Screenshot via `screencapture -x -R<x>,<y>,<w>,<h>` against the window geometry from
   `get {position, size} of window 1`.
8. **Editor UI gating is uneven:** `ui/docked.rs` is `#![cfg(not(wasm32))]` (whole file native), but
   `ui/mod.rs` and `component_registry.rs` are un-gated and have egui code active on wasm — so `tr` must
   resolve on wasm there. `tr` resolves via `use super::*` in `ui/*` files but needs an explicit
   `use crate::app::editor::tr;` in files without the glob (component_registry.rs — gate it native-only
   since its tr usage is native-only).

## Evidence & Data

```
PRs (all squash-merged green):
  #172 feat(ui): rounded corners for DrawRect + the keyboard-focus ring (v0.45.0)   → 36ddf49
  #173 docs(patterns): time-driven System animation note                            → 2fcfde2
  #174 feat(editor): Korean (CJK) + Korean-default editor localization (v0.46.0)     → a087011
       (+ fix commit: package the bundled fonts (assets/fonts/**) — 9d6a163 on the branch)

verify gate: all green each PR. cargo package verify-build: PKG_EXIT=0,
  "Packaged 339 files, 29.2MiB (11.7MiB compressed)".
Translation: ~130 strings; 5 parallel sonnet subagents; tile_paint.rs had 0.
Screenshots (visual acceptance): /tmp/editor_ko_full.png (full Korean F2 editor),
  /tmp/ui_rounded.png (rounded SDF cases).
```

## Files Changed (this session, all merged)

**seq 53 (#172):** `src/renderer/shaders/ui.wgsl` (new), `src/renderer/sprite/geometry.rs`
(`UiInstanceRaw`), `src/renderer/sprite.rs` (`ui_pipeline`), `src/renderer/sprite/ui_primitives.rs`,
`src/renderer/ui.rs` (`DrawRect` fields+builders), `src/ui/focus.rs` (`corner_radius`),
`src/ui/system/focus_pass.rs` (`push_ring`), `examples/ui_rounded.rs` (new),
`examples/{diagonal_pathing,loading_bar}.rs`, Cargo.toml/lock, CHANGELOG, CLAUDE.md.
**seq 53 docs (#173):** `docs/PATTERNS.md`.
**seq 54 (#174):** `assets/fonts/NotoSansKR-Regular.ttf` (new), `src/debug_ui.rs`
(`install_korean_fallback`), `src/app/editor/i18n.rs` (new), `src/app/editor.rs` (module + re-exports),
`src/app/editor/state.rs` + `settings.rs` (`locale` field), `src/app/editor/ui/mod.rs`,
`ui/docked.rs` (toolbar toggle + ~52 strings), `ui/state_machine_panel.rs`, `ui/timeline_panel.rs`,
`ui/data_table_panel.rs`, `ui/audio_panel.rs`, `component_registry.rs`, `Cargo.toml`
(`assets/fonts/**` in `include` + version), Cargo.lock, CHANGELOG, CLAUDE.md.

## User Feedback & Preferences (reconfirmed)

- Korean user-facing reports; English code/handoffs/sub-agent prompts/file docs.
- Use subagents aggressively for parallel work; always pass explicit `model` (sonnet here).
- Merge standing-delegated (squash on green CI, no re-confirm). Never push to `main` directly — branch + PR.
- Read a gate's real exit code, never pipe it. Board is the front door (ASK before backlog when empty).
- Keep `engine-current-state.md` compact (append recent seqs, trim oldest to [[engine-history-archive]]).

## Where We're Going

1. **Watch the board for EW-002+.** `../dungeon-merchant/docs/engine-wishlist.md` is empty; read it first.
2. **Before any crates.io publish (deferred backlog): subset the bundled CJK font.** Package is 11.7 MB
   compressed (> 10 MB limit). `pyftsubset NotoSansKR-Regular.ttf` to the common Hangul + Latin range,
   or split fonts into an optional feature, or raise the crates.io limit.
3. **Editor l10n coverage is broad but may have small gaps** in rarely-opened panels — if a new English
   string surfaces, wrap it with `tr(en, ko)` (the layer is in place). The toggle is the place to spot-check.
4. **Engine-hardening backlog (needs a user go):** crates.io publish (now also gated on font size),
   per-OS gamepad input + deferred analog-stick Y-sign hardware test.

## Risks & Blockers

- **None outstanding.** main clean + green at v0.46.0. The only forward-looking caveat is the package
  size vs crates.io limit (item 2 above), which does not affect local builds or CI.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # a087011 (#174 editor l10n) … 2fcfde2 (#173) … 36ddf49 (#172)
grep -m1 '^version' Cargo.toml  # 0.46.0
git status -s                   # clean

# FIRST: check the wishlist board
sed -n '53,70p' ../dungeon-merchant/docs/engine-wishlist.md   # empty; next free ID EW-002

# Editor localization: src/app/editor/i18n.rs (tr(en,ko), EditorLocale default Korean).
# Bundled CJK font: assets/fonts/NotoSansKR-Regular.ttf, installed in src/debug_ui.rs.
# Memory: engine-current-state.md compact; deep history in engine-history-archive.md.
```

---

## Session Status
**Goal met** — corner-radius (seq 53, #172/#173, v0.45.0) and editor Korean support + Korean-default
localization (seq 54, #174, v0.46.0) both shipped, merged green, and visually verified on macOS. Memory
updated to seq 54 (kept compact). Board empty (next ID EW-002). Handed off to next session (seq 55).
