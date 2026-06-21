# macOS GameController gamepad backend + editor-l10n / Korean-font-subset cleanup (v0.46.0 → v0.47.0)

**Date:** 2026-06-21 (KST)
**Status:** COMPLETED — 6 PRs (#176–#181) all squash-merged green to `main`; tree clean. Memory updated through seq 58.
**Bead(s):** none (repo uses `plans/handoffs/`)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `58`
**Parent:** `HANDOFF_engine-hardening_editor-korean-l10n_2026-06-21.md` (seq 54)
**Prior chain:** `…_wrap-skill-memory-hygiene_2026-06-20.md` (52) > `…_editor-korean-l10n_2026-06-21.md` (54) > this (58)

> This handoff covers a single long session that executed the **entire "Where We're Going" backlog**
> from the seq-54 handoff and shipped **five sequenced units** (seq 55–58 + a `/wrap`):
> - **seq 55** (#176, v0.46.1) — editor l10n coverage fix (2 strings the seq-54 sweep missed).
> - **seq 56** (#177, v0.46.2) — subset the bundled Korean font → crate fits the crates.io 10 MB limit.
> - **seq 57** (#178 v0.46.3 + #179 v0.46.4) — `gamepad_probe` diagnostic example + stdout log.
> - **seq 58** (#180, v0.47.0) — **the headline: a macOS GameController backend → gamepads work on macOS.**
> - **`/wrap`** (#181, docs) — extracted a CI-is-ubuntu rule + an objc2-FFI reference + the `land-pr` skill.
> The session also began by merging the **seq-54 handoff doc PR #175** (the only thing pending at start).

---

## Stale References

(From parent seq 54 — one identifier changed this session.)

- `assets/fonts/NotoSansKR-Regular.ttf` — **removed** in seq 56; replaced by
  `assets/fonts/NotoSansKR-Regular-subset.ttf` (a ~2.3 MB Hangul-only subset). The
  `KOREAN_FONT` `include_bytes!` in `src/debug_ui.rs` now points at the subset file.
- Everything else from parent (`i18n::tr`, `EditorLocale`, `install_korean_fallback`,
  `set_locale`) still exists and is unchanged.

## Since Last Handoff

The seq-54 handoff's **"Where We're Going"** listed: (1) watch the wishlist board, (2) subset the
CJK font before any crates.io publish, (3) editor-l10n coverage gaps, (4) gamepad/publish backlog.
This session executed **2, 3, and 4** in full (board stayed empty all session, EW-002 still free):

- **(3) l10n coverage** → seq 55 (#176): a full audit found exactly 2 missed strings; both fixed.
- **(2) font subset** → seq 56 (#177): the parent's flagged **11.7 MB > 10 MB publish gotcha is now
  RESOLVED** (pkg 9.7 MB). The parent's deferred "subset before publish" is done.
- **(4) gamepad** → seq 57+58 (#178–#180): the long-deferred per-OS gamepad pass landed — and the
  macOS gilrs-can't-read-pads **limitation is now fully RESOLVED** (was the [[gilrs-macos-xbox-no-input]]
  memory's whole premise; hardware-confirmed working).
- Trajectory: still squarely on the engine-hardening path; backlog meaningfully drained. The only
  un-cleared backlog items are crates.io publish (now unblocked, needs a user go) and Windows-pad
  verification.

## Reference Documents

- `CLAUDE.md` — project conventions, module map, **verification rules** (this session added the
  "CI is ubuntu-only" rule there).
- `docs/MACOS_FFI.md` — **new this session**; how to add an objc2 Apple-framework binding.
- `docs/CHANGELOG.md` — entries 0.46.1 → 0.47.0 added this session.
- Memory: `[[engine-current-state]]` (seq 58), `[[gilrs-macos-xbox-no-input]]` (now RESOLVED),
  `[[local-tooling-skills]]` (now lists `land-pr`), `[[merge-authority-delegated]]`.

## The Goal

Drain the seq-54 handoff backlog and harden the engine: close the crates.io publish blocker (the
6 MB Korean font), finish the editor's Korean localization coverage, and complete the deferred
**per-OS gamepad input pass** so gamepads actually work on macOS (Apple's GameController framework
claims modern pads, leaving the IOKit-HID `gilrs` backend blind). End state: a clean, green `main`
at a higher version with macOS gamepad support working, the crate publishable, and the session's
reusable patterns captured as rules/skills. All shipped as small, separately-verified, squash-merged PRs.

## Where We Are

- `main` @ **`5306478`**, tree **clean**, package **v0.47.0**, CLAUDE.md header **v1.6.109**.
- All 6 PRs merged green; the full `./scripts/verify.sh` gate (fmt / clippy `-D warnings` / wasm
  lib+bins build / `test --all-targets` / rustdoc `-D warnings`) passed locally before each push.
- **seq 55 / #176 / v0.46.1** — `src/app/editor/prefab.rs` (5 prefab save/load status messages) +
  `src/app/editor/ui/mod.rs` (the `Entity {idx}:{gen}` inspector fallback label + the "Add Name"
  default) now wrapped in `tr(en, ko)`. Both were built with `format!()` (not an inline egui call),
  so the seq-54 sweep — which scanned egui-method lines — missed them.
- **seq 56 / #177 / v0.46.2** — `assets/fonts/NotoSansKR-Regular.ttf` (5.9 MB) → `…-subset.ttf`
  (2.3 MB) via `pyftsubset`; package **11.7 MB → 9.7 MB compressed** (under the crates.io 10 MB
  publish ceiling). Added `assets/fonts/NotoSansKR-OFL.txt` (license was never bundled),
  `scripts/subset_korean_font.sh` (regen), and an `ab_glyph` regression test in `src/debug_ui.rs`.
- **seq 57 / #178+#179 / v0.46.3+v0.46.4** — `examples/gamepad_probe.rs`: a diagnostic that shows
  the engine's `GamepadState` (gilrs) vs a direct macOS GameController read side by side + a
  throttled stdout log. `objc2-game-controller` added as a macOS-only **dev**-dep.
- **seq 58 / #180 / v0.47.0** — `src/input/gamepad_macos.rs` (new): polls `GCController` each frame
  (from `about_to_wait`) → feeds `GamepadState`. gilrs left uninitialized on macOS. `objc2-game-controller`
  promoted to a macOS-only **lib** dep. **No public API change** — `GamepadState`/`InputMap` identical
  cross-platform. **Hardware-confirmed** on a Bluetooth Xbox pad.
- **`/wrap` / #181 / docs** — `CLAUDE.md` CI-is-ubuntu rule + `docs/MACOS_FFI.md` + the `land-pr`
  local skill (gitignored, logged in `[[local-tooling-skills]]`).
- The 3 `gh pr checks --watch` background runs per merge worked cleanly (native test ≈ 4–6 min each).

## What We Tried (Chronological)

1. **Start: merge the pending seq-54 handoff doc (#175).** Session opened post-`/clear` with a
   task-notification that "Watch handoff PR #175 CI" had exited 0. Confirmed 4/4 checks SUCCESS +
   `mergeStateStatus CLEAN`, squash-merged per delegated authority, synced main (a087011 → 885c86d).

2. **Picked next work via the board + AskUserQuestion.** Wishlist board empty (EW-002 free), so —
   per the ASK-before-backlog rule — surfaced the parent's backlog as options. User chose **"editor
   l10n coverage audit"** first.

3. **l10n audit (seq 55).** Grepped all 20 editor files for egui text methods (`.button`/`.label`/
   `.heading`/`.strong`/`.checkbox`/`.selectable_label`/`.collapsing`/`.menu_button`/`.monospace`/
   `.small`/`.hint_text`/`.on_hover_text`/`.selected_text`/`Button::new`/`Label::new`/`RichText::new`/
   `ComboBox`/`painter.text`) + `*_status`/`*_message` assignments + match-arm display strings, minus
   `tr(`/doc-comments. Result: only **2 genuine misses** (both `format!()`-built, hence off the
   egui-method lines). Confirmed `tile_paint`/`gizmo`/`overlays`/`docked_rt`/`shortcuts` have **0**
   user-facing strings. Deliberately left untranslated: units (px/s/rad), axis/channel prefixes,
   symbols (#/[ ]/🔍/+), FPS/ms/µs, data interpolation, egui id-salts, error `{e}`.

4. **Font subset (seq 56).** Inspected the font: `NotoSansKR-Regular.ttf` = 5.9 MB, 24,853 glyphs
   (glyf TrueType), copyright "(c) 2014-2021 Adobe … Reserved Font Name 'Source'", OFL 1.1, 11,172
   Hangul syllables present. Set up a venv + `fonttools`; subset to Latin-1 + full modern Hangul
   block + Jamo + CJK/general punctuation + fullwidth + ₩. Compared 3 variants (see Evidence) and
   chose **variant B** (drop hinting + all OpenType layout tables — egui's `ab_glyph` uses none of
   them). Renamed to `-subset.ttf` for provenance; removed the full font; added OFL + regen script +
   an `ab_glyph` test asserting Hangul glyphs survive.

5. **Gamepad: probe-first (seq 57).** User picked **"diagnostic probe first"** + **"logic-level only,
   no HW test this pass."** Built `gamepad_probe` comparing gilrs/HID (`GamepadState`) vs a direct
   GameController read. Discovered the objc2 ecosystem split (objc2 0.5.2 AND 0.6.4 in tree from
   winit/wgpu); pinned `objc2-game-controller = "0.2"` to reuse objc2 0.5 (no new version). Learned
   the GC API by reading the crate's `src/generated/*.rs` for exact signatures + per-type feature
   flags. Compile+link-verified on macOS.

6. **User hardware-tested anyway → "input goes in, check the log."** The v0.46.3 probe rendered
   on-screen only (no stdout) — so added a throttled stdout log (seq 57b, #179). User re-ran:
   **`GC-only` confirmed across the board** — gilrs all-zero, GameController full input. Hypothesis +
   the FFI's runtime correctness both proven.

7. **GC backend (seq 58).** User chose **"GC primary, gilrs disabled on macOS."** Read GamepadState
   internals to design a poll-based feed (`apply_macos_snapshot` diffs held set vs previous frame for
   `just_pressed`/`just_released`). Moved objc2 deps dev→lib, added `src/input/gamepad_macos.rs`,
   disabled gilrs init on macOS, wired the poll into `about_to_wait`. Negated stick/dpad Y to the
   engine `up = −Y` convention. Reframed the probe from bug-demo to backend cross-check.

8. **Version self-correction.** First wrote the backend as 0.46.5 (PATCH); corrected to **0.47.0
   (MINOR)** — it's an additive feature/capability, matching how 0.45.0/0.46.0 treated additive work.

9. **Hardware verification of the backend.** User ran the reframed probe: **all inputs flow into the
   engine column** (`OK — engine GamepadState matches GameController`), engine Y = −(raw GC Y) as
   designed, and **"up → character moves up"** confirmed the sign. Merged on green CI **+** this HW
   check (CI alone can't verify the macOS path).

10. **`/wrap`.** Analyzed the 5 code PRs; proposed 3 candidates (A CI-rule, B land-pr skill, C objc2
    reference). User said "3건 모두 추가" → implemented all: A+C as docs PR #181, B as the local skill.

## Key Decisions

- **Each unit shipped as its own small PR + version bump**, not one big PR. Matches the user's
  established cadence (verified, squash-merged, memory-bumped per unit) and keeps CHANGELOG honest.
- **Keep the FULL modern Hangul block (11,172 syllables), not the common 2,350 (KS X 1001).** The
  font also renders arbitrary Korean *data* (entity tags, data-table values), so a restricted set
  would tofu valid syllables. Cost: ~0.3 MB margin under the 10 MB limit (accepted; documented).
- **Subset variant B** (drop GSUB/GPOS/GDEF/BASE/STAT/vhea/vmtx + hinting). Rejected A (keep all,
  2.66 MB) and C (keep layout, 2.6 MB): egui rasterizes via `ab_glyph`, which consults none of those
  tables, so dropping them is zero-risk and smallest.
- **Rename the font to `-subset.ttf`** (not silent in-place overwrite). Honest provenance for an
  OSS skeleton; a forker sees it's modified and can regenerate via the script.
- **Probe-first, then a real backend** (user-chosen). The probe both confirmed the bug on hardware
  AND was where the GC-reading FFI got written + compile-verified — so the backend was low-risk.
- **GC primary, gilrs disabled on macOS** (user-chosen over "GC + gilrs fallback"). Simplest, no slot
  conflicts; macOS 11+ GameController handles essentially all modern pads. gilrs stays on Win/Linux.
- **gilrs still *called* (no-op) on macOS** rather than fully cfg-removed: keeps `process_event`/
  `map_button`/`map_axis` referenced so they don't trip `dead_code` under `-D warnings`. Less churn
  than excluding gilrs from the macOS dependency set.
- **Merge OS-gated changes on green CI *plus* a hardware/manual check, not CI alone.** CI is ubuntu
  and never compiles the macOS path — so green CI doesn't verify it. This overrode the usual
  "squash on green CI" delegation for #180 (waited for the user's pad confirmation).
- **`objc2-game-controller` pinned to 0.2** to reuse the objc2 0.5 already pulled by winit/wgpu (no
  third objc2 version); **macOS-only target dep** so wasm/Linux/Windows + the published crate elsewhere
  are untouched.
- **Candidate C shipped as a `docs/*.md` reference, not a skill** (per the proposal's own judgment —
  niche, "how to investigate" content suits a doc).

## Evidence & Data

### Commit / PR log (this session)

| PR | Commit | Ver | Summary |
|----|--------|-----|---------|
| #175 | 885c86d | (doc) | seq-54 handoff doc merged (session start) |
| #176 | 6accdb5 | 0.46.1 | editor l10n coverage fix (prefab status + Entity fallback) |
| #177 | 11d52ed | 0.46.2 | subset Korean font → pkg 9.7 MB (+ OFL + regen script + test) |
| #178 | 275a3ec | 0.46.3 | `gamepad_probe` diagnostic example |
| #179 | 1fd36bf | 0.46.4 | `gamepad_probe` throttled stdout log |
| #180 | f834332 | 0.47.0 | **macOS GameController backend** |
| #181 | 5306478 | (doc) | CI-is-ubuntu rule + `docs/MACOS_FFI.md` + (local) `land-pr` skill |

### Font subset variants (pyftsubset, all keep 11,172 Hangul + Latin + ₩)

| Variant | Flags | Size | numGlyphs |
|---------|-------|------|-----------|
| A (safe) | defaults | 2,791,564 B (2.66 MB) | 14,036 |
| C (mid) | `--no-hinting` + drop vhea/vmtx | 2,735,336 B (2.6 MB) | 14,036 |
| **B (chosen)** | `--no-hinting --layout-features='' --drop-tables+=vhea,vmtx,BASE,STAT,GSUB,GPOS,GDEF` | **2,440,816 B (2.33 MB)** | 11,978 |

- Original: 5.9 MB, 24,853 glyphs. Package: **11.7 MB → 9.7 MB compressed** (`Packaged 341 files,
  25.6MiB (9.7MiB compressed)`). crates.io enforces ≤10 MiB on the `.crate` gzip (only on real
  `cargo publish`; `cargo package` dry-run does NOT enforce it — that's why CI was green at 11.7 MB).

### Gamepad probe — hardware logs (BT Xbox pad, macOS)

Before the backend (v0.46.4 probe, gilrs view = engine `GamepadState`):
```
[gamepad_probe] GC-only (HID blind — GameController backend is the fix)
    gilrs/HID    L(+0.00,+0.00) A0B0X0Y0 LT0.00 RT0.00
    GameController L(-1.00,-0.52) A0B0X0Y0 LT0.00 RT0.00
```
After the backend (v0.47.0 probe, engine `GamepadState` now GC-backed):
```
[gamepad_probe] OK — engine GamepadState matches GameController (backend active)
    engine GamepadState  L(-0.07,-1.00) A0B0X0Y0 LT0.00 RT0.00
    GameController (raw) L(-0.07,+1.00) A0B0X0Y0 LT0.00 RT0.00
```
Engine Y = −(raw GC Y) (the up = −Y flip, by design). All buttons (A/B/X/Y), both triggers, both
sticks confirmed flowing. User confirmed **"up → character moves up"** (sign correct).

### objc2 dependency alignment

- Tree already had `objc2` 0.5.2 AND 0.6.4 (winit/wgpu), `objc2-foundation` 0.2.2, `block2` 0.5.1.
- `objc2-game-controller = "0.2"` resolved to 0.2.2, reusing objc2 **0.5.2** — no new objc2 version.

### Verify gate

- Every PR: `VERIFY_EXIT=0`. Native test 4m4s / 3m51s / 4m46s across the watched runs; WASM ~39s,
  Rustdoc ~40s, Package dry-run ~1m11s.

## Code Analysis

- **`GamepadState`** (`src/input/gamepad.rs`): `slots: [Option<Slot>; 4]`; `Slot` = `pressed` /
  `just_pressed` / `just_released` (`HashSet<GamepadButton>`) + `axes` (`HashMap<GamepadAxis, f32>`).
  Public: `is_connected`/`any_connected`/`primary`/`is_pressed`(held)/`just_pressed`/`just_released`/
  `axis`. gilrs path is event-driven (`process_event` + per-frame `flush()` at `schedule.rs:494`).
- **`apply_macos_snapshot(pad, buttons, axes)`** (new, `cfg(target_os="macos")`): poll-based — sets
  `just_pressed = buttons − slot.pressed`, `just_released = slot.pressed − buttons`, then overwrites
  `pressed`/`axes`. Works because `pressed` persists across `flush()` (which only clears the edges),
  so the frame-to-frame diff is correct. `disconnect_macos(pad)` clears a slot.
- **`gamepad_macos::poll`**: `GCController::controllers()` (unsafe) → `NSArray::get_retained(i)` →
  `extendedGamepad()` → full read. Maps face/shoulder/trigger(digital)/dpad/menu→Start/options→Select/
  thumbstick-clicks; axes LeftStick/RightStick (Y negated) / triggers(analog) / DPadX/DPadY(Y negated).
- **GC API shapes** (from `objc2-game-controller-0.2.2/src/generated/`): all accessors `unsafe`;
  `buttonA()`/`leftShoulder()`/`leftTrigger()`/dpad `up()` etc. → `Retained<…>`; `buttonOptions()`/
  `leftThumbstickButton()`/`rightThumbstickButton()` → `Option<Retained<…>>`; `…ButtonInput::isPressed()
  -> bool`, `…AxisInput::value() -> c_float` (= f32). NSArray safe helpers: `len`/`first_retained`/
  `get_retained`.
- **gilrs disabled on macOS** (`src/app.rs`): `#[cfg(all(not(wasm32), not(macos)))] let gilrs =
  Gilrs::new().ok();` / `#[cfg(all(not(wasm32), macos))] let gilrs: Option<Gilrs> = None;`.
  `poll_gilrs` still called in `about_to_wait` (no-ops when `None`), then the macOS GC poll runs.
- **`install_korean_fallback`** (`src/debug_ui.rs`): unchanged logic; `KOREAN_FONT` now embeds the
  subset path. New `#[cfg(test)] mod tests` loads it via `ab_glyph::FontRef` and asserts glyphs for
  가/힣/한/국/어/엔/티/저/장 + 'A'.

## Gotchas & Discoveries (expensive to re-discover)

1. **`cargo package` dry-run does NOT enforce the 10 MB limit.** Only a real `cargo publish` does.
   That's why CI's "Package dry-run" stayed green at 11.7 MB (the parent's gotcha). Don't trust a
   green Package-dry-run as "publishable" — check the reported compressed size against 10 MiB.
2. **egui rasterizes with `ab_glyph`, which uses no OpenType layout/hinting tables.** So a fallback
   font can drop GSUB/GPOS/GDEF/BASE/STAT + hinting with zero rendering impact (the basis for subset
   variant B). The `ab_glyph` regression test confirms glyph presence after a re-subset.
3. **objc2 framework FFI: read the registry source, don't guess.** `~/.cargo/registry/src/*/objc2-*-<ver>/
   src/generated/*.rs` has exact signatures + the per-type `#[cfg(feature=…)]` gate for every type.
   Pin the framework crate to the objc2 major already in the tree (winit/wgpu) or you duplicate the
   whole objc2 ecosystem. `c_float` == `f32`. NSArray has safe `len`/`first_retained`/`get_retained`.
   The framework crate auto-emits the `#[link]` — no build.rs / `-framework` flag. (All now in
   `docs/MACOS_FFI.md`.)
4. **`cargo fmt` (not just `--check`) after hand-writing code.** Hit `fmt --check` red TWICE on
   `gamepad_probe.rs` (long `format!` strings / multi-line calls rustfmt rewraps). Run `cargo fmt`
   then re-verify — the first verify failed on fmt only both times.
5. **Stale rust-analyzer diagnostics were heavy this session** — E0425 `cannot find value hid` after a
   `perl -0pi` rename (lagged behind the edit), plus the usual inactive-code / unlinked-file phantoms.
   `cargo build` reported 0 errors throughout. Trust cargo/CI, not the IDE squiggles (reconfirmed).
6. **The macOS GC path is invisible to CI (ubuntu).** Green CI never compiled `gamepad_macos.rs` or
   pulled `objc2-game-controller`. The dual-cfg `dead_code` risk (e.g. `HidView::active` unused on the
   non-macOS branch) is only caught by building each OS locally. This is now a CLAUDE.md rule.
7. **End-to-end Y-sign check = play, not just values.** The probe shows the flip (engine Y = −raw GC Y),
   but "up = −Y is *correct*" was confirmed by **the character moving up** (survivor-style). gilrs's
   empirical convention here is up = −Y (per the existing memory), and GC reports up = +Y → negate.

## Files Changed

### Source code
- `src/input/gamepad_macos.rs` — **new**: macOS GameController backend (poll + read_extended).
- `src/input/gamepad.rs` — `apply_macos_snapshot` + `disconnect_macos` (cfg macos).
- `src/input/mod.rs` — `#[cfg(target_os="macos")] pub(crate) mod gamepad_macos;`.
- `src/app.rs` — gilrs uninitialized on macOS.
- `src/app/window.rs` — macOS GC poll in `about_to_wait`.
- `src/app/editor/prefab.rs` — prefab status messages via `tr()`.
- `src/app/editor/ui/mod.rs` — `Entity` fallback label + default name via `tr()`.
- `src/debug_ui.rs` — embed subset font + `ab_glyph` regression test.

### Examples
- `examples/gamepad_probe.rs` — **new** (seq 57), then reframed (seq 58) from bug-demo to backend
  cross-check; throttled stdout log.

### Assets / scripts / docs
- `assets/fonts/NotoSansKR-Regular-subset.ttf` (new, 2.3 MB), `NotoSansKR-OFL.txt` (new);
  `NotoSansKR-Regular.ttf` (deleted).
- `scripts/subset_korean_font.sh` (new), `docs/MACOS_FFI.md` (new).
- `Cargo.toml` / `Cargo.lock` — objc2 deps (macOS), ab_glyph dev-dep, versions.
- `docs/CHANGELOG.md`, `CLAUDE.md` (module map + header + CI rule).

### Local (gitignored, not in repo)
- `.claude/skills/land-pr/SKILL.md` (new skill), `.claude/proposals/2026-06-21.md` (wrap proposal).

## User Feedback & Preferences (REQUIRED)

- **"3 → 에디터 l10n 커버리지 점검"**, then later **"3. os별게임패드작업"** — picked next work from the
  offered backlog options (the board was the front door; user drove ordering).
- **Gamepad approach: "진단 프로브 먼저"** + **"아니오, 로직 레벨까지만 (no HW test this pass)."** Then
  the user *did* hardware-test anyway ("입력 들어가는거 확인 했어 로그 확인해봐") — appetite was higher
  than stated; treat "logic-level only" as a floor, not a ceiling.
- **"방향 위가 캐릭터 위로 이동 맞음. 전체 확인 햇고 머지해줘"** — explicit hardware sign-off + merge order.
- **GC coexistence: "예 — GC 우선, macOS에서 gilrs 비활성 (권장)"** — took the recommended option.
- **"3건 모두 추가"** — implement all 3 `/wrap` proposals (no cherry-picking).
- **`/wrap` arg "푸시"** and **`/handoff` arg "푸시"** — wants commits pushed at wrap/close.
- Standing prefs (reconfirmed): Korean to the user, English in code/docs/handoffs; squash-merge on
  green CI (delegated); read a gate's real exit code, never pipe it; keep `engine-current-state.md`
  compact; use subagents with explicit `model` for parallel work (none needed this session — all
  changes were tightly interdependent, done inline).

## Where We're Going

1. **Board first** — `../dungeon-merchant/docs/engine-wishlist.md` (still empty; next free **EW-002**).
   ASK before backlog when empty.
2. **crates.io first publish (now UNBLOCKED).** The 10 MB font gate is gone (pkg 9.7 MB). Needs a
   user go. Before publishing, re-confirm `cargo publish --dry-run` size and the bundled OFL files.
3. **Windows gamepad verification.** gilrs (xinput) is the Windows backend and *should* work, but is
   untested in this repo. A Windows pad run of `gamepad_probe` would confirm; no code expected.
4. **Optional probe polish (cosmetic):** `gamepad_probe`'s raw-GC cross-check column (`mod gc`) doesn't
   read the R-stick, so touching the R-stick shows a benign `engine-only` tag (engine reads R-stick,
   the cross-check doesn't). Add R-stick to `GcView` if it bothers anyone. Non-blocking.
5. **CLAUDE.md is 209 lines** (soft limit 200; was ~202 before this session). If a trim pass is
   wanted, move a verbose section to a `docs/*.md` — don't drop content to hit the number.

## Risks & Blockers

- **None outstanding.** main clean + green at v0.47.0.
- macOS gamepad backend's *runtime* is verified by hardware only (CI is ubuntu, can't compile the
  path). Any future change to `gamepad_macos.rs` needs a local macOS build + a pad re-check.
- The 0.3 MB package margin (9.7/10 MB) erodes slowly as CHANGELOG/docs grow (text compresses well,
  so ~KB per release). Levers documented in `[[engine-current-state]]` seq 56 if it ever regrows.

## Open Questions

- Does the gilrs xinput path actually work on Windows in this repo? (Untested — see Where We're Going #3.)
- Is the user ready to do the first crates.io publish, or keep it deferred?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 5306478 (#181 docs) … f834332 (#180 macOS gamepad) … 1fd36bf (#179)
grep -m1 '^version' Cargo.toml  # 0.47.0
git status -s                   # clean

# FIRST: the wishlist board (front door)
sed -n '50,70p' ../dungeon-merchant/docs/engine-wishlist.md   # empty; next free ID EW-002

# Key files this session touched (read first):
#   src/input/gamepad_macos.rs        — macOS GameController backend (poll + read_extended)
#   src/input/gamepad.rs              — GamepadState + apply_macos_snapshot
#   examples/gamepad_probe.rs         — hardware diagnostic / backend cross-check
#   docs/MACOS_FFI.md                 — how the objc2 binding was built
#   scripts/subset_korean_font.sh     — regenerate the bundled Korean font subset

# Verify current state
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"   # expect 0

# Hardware re-check (macOS, pad connected)
cargo run --example gamepad_probe   # engine column should track the pad; log "OK …"

# Next action: pick from EW-002 (if filed) OR ask the user re: crates.io first publish
#   (now unblocked — pkg 9.7 MB < 10 MB) or Windows gamepad verification.
```

---

## Session Closed
**Closed at:** 2026-06-21 (KST)
**Commit:** via PR (branch `docs/handoff-seq-58` → `main`, squash-merge)
**Session status:** Handed off to next session (engine-hardening seq 58 → next)
