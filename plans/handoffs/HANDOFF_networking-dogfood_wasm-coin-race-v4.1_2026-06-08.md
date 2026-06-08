# wasm coin_race shipped to the browser + engine v4.0.0/v4.1.0 tagged + three latent wasm render bugs fixed

**Date:** 2026-06-08
**Status:** COMPLETED — coin_race runs and plays in a real browser (render + network + input verified on Retina); engine git-tagged `v4.0.0` and `v4.1.0`; CHANGELOG reconstructed (was stale at 2.0.0); rust-survivors re-pinned to v4.1.0. Full `+1.88.0` gate green. Open: v4.1.0 tag points *before* the two wasm-text fixes; rust-survivors pin commit is local/unpushed.
**Bead(s):** none (`bd` unavailable in this project)
**Epic:** none (VISION feature+example loop — breadth coverage / dogfooding)
**Chain:** `networking-dogfood` seq `2` (continues seq 1)
**Parent:** `HANDOFF_networking-dogfood_coin-race-example_2026-06-08.md` (seq 1 — shipped the *native* authoritative coin_race; its "Where We're Going" #1 was the wasm networked example, picked up here)

---

## Since Last Handoff (launching context)

Seq 1 closed the networking playable-example gap with a **native** authoritative `coin_race` (`coin_race_game` + `coin_race_server`) and listed five out-of-chain candidates in "Where We're Going": (1) **wasm networked example**, (2) **git-tag engine v4.0.0**, (3) reusable remote-entity helper, (4) organize rust-survivors WIP docs, (5) example Korean→English. This session was launched fresh (`/clear`) onboarding from seq 1, then the user chose **(2)+(1)**: tag v4.0.0 first, then build the wasm coin_race example. That expanded into: a CHANGELOG reconstruction (discovered stale), a v4.1.0 cut, three latent wasm render bugs found by actually looking at the example in a browser, and re-pinning rust-survivors.

## Reference Documents

- `docs/VISION.md` — feature+example loop; the bar this work meets ("a feature isn't done until a small playable example exercises it in real play" + "fix the gap the example surfaces").
- `docs/CHANGELOG.md` — reconstructed 3.0.0 + 4.0.0 this session; 4.1.0 added. **Was stale at 2.0.0** before this session.
- `docs/NEXT_WORK.md` — candidate M (networking) gained a "Browser follow-up (wasm, v4.1.0)" sub-bullet.
- `src/app/window.rs` — wasm canvas/text-renderer setup (two fixes here: text-renderer font fallback + canvas display-size lock).
- `src/app/schedule.rs:106-128` — `ViewportSize`/`DisplayScaleFactor` per-frame compute (HiDPI fix here).
- `src/renderer/mod.rs` — `DEFAULT_FONT` const (wasm-only embed).
- `examples/games/coin_race/coin_race.rs` + `examples/games/coin_race/web/` — the dual-target client + browser build harness.
- Memory: `playtest-windowed-examples` (extended with wasm/headless-Chrome technique), `ci-toolchain-pin` (`+1.88.0`), `conversation-language-korean`, `doc-language-rule`, `rust-survivors-engine-pin`, `subagent-usage-preference`, `v3-breaking-batch`.

## The Goal

Ship coin_race to the browser (close the one networking sub-gap — friction point 3 from seq 1: "no example runs a networked game on wasm"), and tag the engine so consumers can pin a version instead of a rev. Success = the wasm client renders + connects + plays in a real browser, the example→web build path is reusable, and the engine has proper version tags + an accurate CHANGELOG.

## Where We Are

**Engine (`skeleton-engine`), branch `main`, all pushed to `origin/main`. Five commits this session:**

| Commit | What | Tag |
|---|---|---|
| `5031c3d` | docs(changelog): add 3.0.0 + 4.0.0 sections (was stale at 2.0.0) | **v4.0.0** here |
| `9199a12` | feat(examples): run coin_race in the browser (wasm) + example→web path | |
| `7c6f9c0` | fix(render): correct wasm HiDPI viewport (was halved on Retina) | **v4.1.0** here |
| `0cd9b03` | feat(text): embed a default font so wasm HUD text renders out of the box | |
| `ebd9081` | fix(wasm): lock canvas display size to its buffer so HUD text isn't clipped | |

- Package version: **4.1.0** (`Cargo.toml`). Tags: `v4.0.0`→`5031c3d`, `v4.1.0`→`7c6f9c0`. Pre-existing tags `v0.3.0`/`v0.4.0` are lightweight; the two new tags are **annotated**.
- **coin_race runs in a browser** (Retina, real GPU): white player avatar + server-spawned coins render via WebGL2, the wasm WebSocket connects to the native `coin_race_server` (`[N] connected` in the server log), held-key movement works, and the **full HUD text renders** (status + scoreboard + hint) after the two text fixes.
- **`examples/games/coin_race/web/`** = the reusable "ship an engine example to the web" harness: `index.html` (canvas#game-canvas + `import init, { run_coin_race }`) + `build.sh` (`cargo build --release --example` + `wasm-bindgen --target web`). `pkg/` is gitignored.
- **`coin_race.rs` refactored to dual-target**: game code lifted from inside `fn main` to module scope; native `fn main(){ run(); }`; wasm `#[wasm_bindgen] pub fn run_coin_race(){ run(); }` + empty `fn main(){}`. Game code stays in the example (engine lib stays a skeleton).

**rust-survivors (`/Users/jkl/Projects/rust-survivors`, branch `main`):**
- Pin bumped `60328fa` (v4.0.0) → **`7c6f9c0` (v4.1.0)** in `crates/game/Cargo.toml` + `Cargo.lock`. Commit **`e6176fa`** `chore(deps): bump engine pin to skeleton-engine v4.1.0` — **LOCAL, NOT PUSHED**.
- Builds green (`cargo build -p game --bin survivor`) and **runs** against v4.1.0 (title screen + Korean menu text render; it embeds `assets/fonts/NotoSansKR-Regular.ttf` via `FontData`).
- ~20 unrelated WIP doc changes remain uncommitted there (left untouched — not this session's work): `M AGENTS.md`, `M CLAUDE.md`, `D docs/ENGINE_FOLLOWUPS.md`, etc.

**Gate (memory `ci-toolchain-pin` → `cargo +1.88.0`):** fmt --check / clippy --all-targets -D warnings / clippy --example coin_race_game --target wasm32 -D warnings / build --target wasm32 (lib+bins) / test --all-targets (**311 lib + 5 coin_race_server**, 0 failed) / doc -D warnings — all exit 0 after every change batch.

## What We Tried (Chronological)

1. **Onboarded** from seq 1; ran the engine baseline gate (green); read `network.rs`, the coin_race files, VISION/NEXT_WORK, `mp_client`. Confirmed seq 1's chain had no pending step → picked NEW initiative.
2. **AskUserQuestion** → user chose "v4.0.0 태그 + wasm 예제"; and (a) wasm architecture = "Option B spike → fallback A", (b) CHANGELOG = "write 3.0.0+4.0.0 then tag".
3. **CHANGELOG reconstruction** — discovered `docs/CHANGELOG.md` topped out at `## 2.0.0` while the package was `4.0.0` (missing 3.0.0 + 4.0.0). Found version-bump boundaries via `git log -S'version     = "X.Y.Z"'`: 3.0.0 at `1e7f91b` (#13), 4.0.0 at `0893ce0` (#28). Delegated the draft to a **Sonnet subagent** (memory `subagent-usage-preference`). **Reviewed and corrected the draft**: (a) version-boundary error — it placed `02cc9e2`'s audio_fades + minimap-WorldLabelSystem under 4.0.0, but `02cc9e2` is *before* `0893ce0` → moved to 3.0.0; (b) **dropped an ungrounded simba/E0283 caveat** (not in any commit/doc); (c) stripped all `(commit XXX)`/`(#NN)` citations to match the existing house style (zero citations). Verified the remaining technical claims against `docs/CODE_ANALYSIS.md` + commit messages. Wrote both sections above `## 2.0.0`.
4. **Tagged v4.0.0** (annotated) at the CHANGELOG commit `5031c3d`, pushed commit + tag.
5. **wasm Option B spike** — installed `wasm-bindgen-cli@0.2.122` (must match the `wasm-bindgen` crate = 0.2.122; `wasm-pack` 0.15 only builds the lib, not examples). First attempt on the **empty** example main → `wasm-bindgen` error **"failed to find intrinsics to enable clone_ref function"**. Disambiguated by running the same CLI on the *lib* wasm (which uses wasm-bindgen) → succeeded → conclusion: the error is **empty-main DCE stripping the bindgen intrinsics**, not a flags/version problem.
6. **Refactored `coin_race.rs`** to dual-target (lifted code out of `fn main`, added `run_coin_race` `#[wasm_bindgen]` entry). Built example → wasm + `wasm-bindgen --target web` → **clean, `run_coin_race` exported**. Confirmed Option B works.
7. **Wrote `web/index.html` + `build.sh`**; confirmed `pkg/` is gitignored (root `.gitignore` has `pkg/`).
8. **First gate after refactor** caught **4 pre-existing `uninlined_format_args` clippy warnings in wasm-only lib code** (`async_loading.rs` ×3, `save.rs` ×1) surfaced by `clippy --example ... --target wasm32` (NOT part of the CLAUDE.md gate, which omits wasm clippy). Fixed them (committed with the wasm example).
9. **Browser playtest #1 (render proof)** — `screencapture` of GUI Chrome came back **all black**. Initially attributed to Screen-Recording permission; later (user: "화면이 꺼져 있어서") root cause = **display asleep** — the `playtest-windowed-examples` memory's `caffeinate -u` wake step was missing. Used **headless Chrome `--screenshot`** (permission-free) → rendered player + 6 coins (but no text). Server log showed the browser tab **connected** (`[1] connected`).
10. **Found wasm bug #1 (HiDPI viewport halving)** — redoing the GUI playtest with `caffeinate -u` worked, but revealed the field rendered **broken on Retina** (player + most sprites off-screen, only an edge coin visible). Reproduced **deterministically** with headless `--force-device-scale-factor=2` (DPR=1 correct, DPR=2 broken). Root-caused in `schedule.rs:117`: `ViewportSize = gpu.config / scale_factor` for all targets, but the wasm surface is already the canvas-DOM (CSS-logical) size, so dividing by DPR halved it → `coin_race`'s fixed 800×600 coords fell off-screen. (`run_demo` masked it by adapting to `ViewportSize`.) Fixed: DPR division **native-only** (`#[cfg(wasm)] scale_factor = 1.0`). Re-verified DPR=2 → correct, on real Retina GUI → correct, held-D moved the player. Committed `7c6f9c0`; tagged **v4.1.0**, bumped Cargo.toml 4.0.0→4.1.0, added the 4.1.0 CHANGELOG section, updated NEXT_WORK/HANDOFF/CLAUDE.md.
11. **Ran rust-survivors** (`cargo run -p game --bin survivor`) at the user's request — built green, played (timer/combat/HUD), then title screen on re-run.
12. **Found wasm bug #2 (no font)** — verifying the HUD text revealed it **never rendered on wasm**. Root cause: `FontSystem::new()` loads OS fonts (absent in the wasm sandbox), and `window.rs:347` deliberately skips creating the text renderer on wasm when `font_bytes` is empty (cosmic-text panics shaping with no fonts). `coin_race` supplies no `FontData` → no text. Fix (user chose "engine default font"): embedded **DejaVu Sans** (`assets/fonts/DejaVuSans.ttf`, 757KB, Bitstream Vera/Arev license) under a wasm-only `cfg`, fall back to it when no `FontData`, create the text renderer unconditionally. Committed `0cd9b03`. Verified: HUD text rendered in a real browser.
13. **Found wasm bug #3 (canvas stretch)** — the now-rendering text was **shifted ~120px left and clipped** ("You are Playe" off-screen), text-only (sprites fine). Added a runtime `web_sys::console::log_1` debug line, captured via headless Chrome `--enable-logging=stderr --v=1`: all engine inputs were correct (`res=(800,600), sprite_vp=(800,600), scale=1, pos=(12,10)`) but **`canvas.width=800` vs `client_width=1280`**. Root cause: winit sets the canvas CSS *display* box to the window's logical size (default **1280** when `WindowConfig` isn't applied at canvas creation) via inline style, while the buffer stays 800 → the browser stretches 800→1280 and, being wider than the window, centres + clips the left → fixed-position HUD text falls off-screen (sprites at mid-canvas survive). Confirmed by a CSS `!important` test (forced `client_width≈800` → text fully rendered). Fixed at **engine level**: `finish_init` sets the canvas CSS width/height to its drawing-buffer size (after winit sizes it; normal inline so a game can override with `!important`). Removed the debug log + the html `!important` test; index.html ended net-zero. Committed `ebd9081`. Verified full HUD on real Retina GUI (status + scoreboard + bottom hint, no clipping) with the clean index.html.
14. **Committed the rust-survivors pin** (`e6176fa`, local) — Cargo.toml + Cargo.lock only, leaving their WIP docs untouched.

## Key Decisions

- **Tag v4.0.0 at the CHANGELOG commit, not retroactively.** Cut the tag once the 4.0.0 release notes existed. (Annotated, unlike the legacy lightweight `v0.3.0`/`v0.4.0`.)
- **Option B (example→web), not A (lib `run_coin_race`).** Keeps game code in the example (engine stays a genre-agnostic skeleton) AND fixes the real gap — there was no path to ship an engine *example* to the web (only the hardcoded lib `run_demo`). Spiked first per user choice; it worked, so A (fallback) wasn't needed.
- **`#[wasm_bindgen] run_coin_race` in the example, called explicitly by index.html** (mirroring `run_demo`), rather than relying on a bin `main` auto-running — sidesteps uncertainty about winit `spawn_app` from a bin start + the lib's `#[wasm_bindgen(start)]` co-existing.
- **Three wasm fixes are all wasm-only / additive; native untouched.** HiDPI fix is `cfg`-gated (native keeps dividing by DPR). Font is `include_bytes!`'d under a wasm `cfg` (native binaries don't carry it). Canvas lock is in a wasm `cfg` block. So `rust-survivors` (native) is unaffected — its pin can stay at any v4.x.
- **Embedded DejaVu Sans (full, 757KB) rather than a smaller Latin font.** Couldn't reliably download Roboto from CDNs; DejaVu (jsdelivr npm) was a valid TTF with broad glyph coverage (`▶ ·`) and a permissive license. Size is acceptable for a wasm-only embed.
- **Canvas-size fix at engine level, not in each example's html.** The user prefers engine robustness; this makes *all* wasm examples display 1:1 without a per-html hack. Used normal inline style (not `!important`) so a forker can still override.
- **rust-survivors pin committed but not pushed; WIP docs not bundled.** "커밋해줘" = commit; pushing the game repo is the user's call. Kept the commit surgical (Cargo.toml + Cargo.lock).
- **CHANGELOG accuracy over the subagent draft.** Corrected a version-boundary misplacement + dropped an unverifiable simba/E0283 claim + matched the citation-free house style.

## User Feedback & Preferences

- **AskUserQuestion (initiative)** → "v4.0.0 태그 + wasm 예제" (over Korean→English-only or tag-only). Then **wasm architecture** → "B spike → 안 되면 A fallback (추천)"; **CHANGELOG** → "3.0.0+4.0.0 작성 후 태그 (추천)".
- **"베이스라인부터"** ethos carried from seq 1 — ran the gate before/after every change batch.
- **"화면이 꺼져 있어서 완료 못한 테스트 항목 다시 테스트"** — the black `screencapture` was the display being asleep, not a permission issue. (Direct user insight that unblocked the GUI playtest + surfaced bug #1.)
- **"rust-survivor 실행해줘"** → ran the game; left it running on their display.
- **HUD-text fix choice** → "엔진에 기본 폰트 임베드" (engine-level robustness for all wasm examples, accepting binary-size cost) over an example-local font or documenting-as-limitation.
- **"텍스트 offset 추적해서 고치고 pin도 커밋해줘"** → root-caused + fixed bug #3 at engine level; committed the rust-survivors pin (locally, surgical — Cargo.toml+Cargo.lock only).
- **Standing prefs (memory):** conversation in Korean / artifacts in English (`conversation-language-korean`, `doc-language-rule`); `cargo +1.88.0` for gates (`ci-toolchain-pin`); verify before declaring done; subagents for parallel work (`subagent-usage-preference` — used the Sonnet CHANGELOG subagent); rust-survivors pins engine by rev (`rust-survivors-engine-pin`).

## Evidence & Data

### The three latent wasm render bugs (all masked because wasm was never *looked at* in a browser)
| # | Symptom (Retina) | Root cause | Fix (file) | Commit |
|---|---|---|---|---|
| 1 HiDPI viewport | sprites off-screen; only edge coin visible | `ViewportSize = surface/DPR` for all targets, but wasm surface is already CSS-logical → halved | DPR division native-only (`schedule.rs`) | `7c6f9c0` |
| 2 no font | text entirely absent | `FontSystem::new()` = OS fonts (none on wasm); engine skips text renderer w/o `FontData` | embed DejaVu Sans, wasm fallback (`renderer/mod.rs` + `window.rs`) | `0cd9b03` |
| 3 canvas stretch | text shifted ~120px left, clipped (sprites fine) | winit sets canvas CSS display = window logical (1280) ≠ buffer (800) → stretched + centred-clipped | lock canvas CSS size = buffer in `finish_init` (`window.rs`) | `ebd9081` |

### Runtime debug capture (the key to bug #3)
`[textdbg] res(w,h)=(800,600) sprite_vp=Some((800.0,600.0)) scale=1 pos=Some(Vec2(12.0,10.0)) | "canvas.width=800 client_width=1280 dpr=2"` — every engine input correct, but display (1280) ≠ buffer (800).

### Fix details (before → after)
- **Bug #1 (`schedule.rs:106-128`):** was one `let scale_factor = window.scale_factor()...` for all targets. Now `#[cfg(not(wasm))] let scale_factor = window.scale_factor()...; #[cfg(wasm)] let scale_factor = 1.0_f32;` — wasm `ViewportSize = gpu.config / 1` (surface already logical), `DisplayScaleFactor = 1`.
- **Bug #2 (`renderer/mod.rs` + `window.rs:333-352`):** added `#[cfg(wasm)] pub(crate) const DEFAULT_FONT: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/DejaVuSans.ttf"));`. `window.rs` was `#[cfg(wasm)] let text_renderer = if !font_bytes.is_empty() { Some(..) } else { None };` → now `#[cfg(wasm)] let font_bytes = if font_bytes.is_empty() { DEFAULT_FONT.to_vec() } else { font_bytes };` then `let text_renderer = Some(TextRenderer::new(...))` unconditionally.
- **Bug #3 (`window.rs:310` `finish_init`):** added a wasm block at the top of `finish_init` that grabs `#game-canvas` and `style.set_property("width", &format!("{}px", canvas.width()))` (+ height) — locks CSS display = drawing buffer, after winit has sized it.

### CHANGELOG content written (so the next session knows what's documented)
- **3.0.0** Added: `Color` newtype, `AudioSystem` (drives fades + SFX file-bytes cache), `DrawText::centered`+`TextAnchor`+`Camera::world_to_screen`, `MouseButton` re-export, `ReflectValue::I32`+`#[non_exhaustive]`, `ScriptingLimits` fields, `spawn_scene_def` dup-tag detection, `audio_fades` example, `minimap` `WorldLabelSystem`. Breaking: PhysicsWorld→resource, all color fields→`Color`. Fixed: per-frame hot-path costs (#7/#8/#18/#19), `RenderLayer` negative-fold (#22), point-light radius contract test (#15 false-positive).
- **4.0.0** Added: native `coin_race` example. Breaking: `ImpulseJointHandle` re-export → `engine::JointHandle` newtype.
- **4.1.0** Added: coin_race-on-wasm + example→web path; embedded default font (wasm text). Fixed: HiDPI viewport halving; canvas-stretch HUD-clip.

### Verification techniques (reusable — also in memory `playtest-windowed-examples`)
- **macOS `screencapture` of a browser → all black unless the display is awake**: `caffeinate -u -t N &` before capturing. (Lost a pass to this; the memory already said it, missed it.)
- **Headless Chrome `--screenshot` is permission-free** and renders WebGL2 via `--enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader`. **Defaults to DPR=1, which MASKS Retina bugs** — add `--force-device-scale-factor=2`. SwiftShader renders sprites but **not glyphon text** (don't read its absence as a bug).
- **Console capture**: `--enable-logging=stderr --v=1` dumps `console.log` to stderr; grep it.
- **Browser caches wasm aggressively** — after rebuilding `pkg/`, a cached tab runs OLD wasm; relaunch with a fresh `--user-data-dir`.
- **Chrome Translate popup** covers the top-right; doesn't affect top-left HUD; `--disable-features=Translate` did NOT suppress it (only `!important` CSS or a fresh profile helped framing).

### Option B build path (the reusable example→web recipe)
`cargo build --release --example coin_race_game --target wasm32-unknown-unknown` then `wasm-bindgen --target web --out-dir <web>/pkg target/wasm32-unknown-unknown/release/examples/coin_race_game.wasm`. Requires `wasm-bindgen-cli` **exactly matching** the `wasm-bindgen` crate (Cargo.lock = 0.2.122). Empty example `main` → `wasm-bindgen` "failed to find intrinsics to enable clone_ref" (DCE); a real entry that exercises the engine fixes it.

### CHANGELOG version boundaries (from `git log -S` on Cargo.toml version)
- 2.0.0 introduced by `5f75a91`; 3.0.0 by `1e7f91b` (#13 PhysicsWorld→resource); 4.0.0 by `0893ce0` (#28 JointHandle newtype).
- 3.0.0 range = `1e7f91b..0893ce0` (Color newtype #11, PhysicsWorld-resource #13 [both breaking], perf #7/#8/#18/#19, fixes #20/#22/#23, additive #12/#27/#29/#30, audio_fades + minimap WorldLabelSystem from `02cc9e2`).
- 4.0.0 range = `0893ce0..` (JointHandle breaking + native coin_race example) — the rest are docs/session commits.

### Engine gate — final (`cargo +1.88.0`)
fmt ✅ · clippy --all-targets ✅ · clippy --example coin_race_game --target wasm32 ✅ · build --target wasm32 (lib+bins) ✅ · test --all-targets ✅ (lib **311** + coin_race_server **5**, 0 failed) · doc -D warnings ✅.

### rust-survivors
Pinned `7c6f9c0` (Cargo.lock `source = "git+...?rev=7c6f9c0...#7c6f9c0..."`). Bin `survivor` (`crates/game/src/bin/survivor.rs`), run from repo root (`assets/` relative). Controls: WASD/arrows move, ENTER start, 1/2/3 level-up cards, ESC pause, R restart. Embeds NotoSansKR via `FontData` → text works natively.

## Code Analysis (findings for the next cycle)

- **The wasm path had 3 independent latent bugs**, all because nothing ever *rendered* wasm and looked at it. CI builds wasm (lib+bins) but never runs it; the example loop's "look at it in real play" caught all three. → A deterministic wasm render smoke test (headless-Chrome screenshot, the technique proven here) would catch regressions. → drives a possible Phase.
- **`examples/wasm/` (the lib `run_demo` demo) has a stale committed `pkg/`** (built weeks ago, `engine_bg.wasm` ~10.7MB tracked? no — `pkg/` is gitignored; the committed one predates). run_demo *adapts* to `ViewportSize` so HiDPI didn't break it, but the **canvas-stretch fix (bug #3) now applies engine-wide**, and run_demo's text (if any) would have had bug #2/#3 too. → Verify run_demo renders correctly on Retina now; refresh/rebuild its pkg.
- **`finish_init` canvas-size lock uses normal inline style** set once after winit. It converges (setting style → ResizeObserver → Resized → reads buffer, no-op). If a future case shows winit re-overriding, escalate to `set_property_with_priority(..., "important")`.
- **`mp_client`/`mp_server` (Phase 27 demos)** are still top-level `examples/`, native-only wasm-stub mains, no `FontData`, and reimplement remote-entity bookkeeping inline — same deferred helper candidate as seq 1.

## Files Changed (this session)

### skeleton-engine (committed + pushed)
- **NEW** `assets/fonts/DejaVuSans.ttf` (757KB) + `assets/fonts/DejaVuSans-LICENSE.txt` — embedded default font (`0cd9b03`).
- **NEW** `examples/games/coin_race/web/index.html` + `build.sh` — browser harness (`9199a12`).
- `examples/games/coin_race/coin_race.rs` — dual-target refactor + `run_coin_race` (`9199a12`).
- `src/app/schedule.rs` — HiDPI fix: DPR division native-only (`7c6f9c0`).
- `src/renderer/mod.rs` — `DEFAULT_FONT` wasm const (`0cd9b03`).
- `src/app/window.rs` — text-renderer font fallback (`0cd9b03`) + canvas display-size lock in `finish_init` (`ebd9081`).
- `src/asset/async_loading.rs`, `src/save.rs` — wasm-only `format!` arg-inlining (`9199a12`).
- `Cargo.toml`/`Cargo.lock` — version 4.0.0→4.1.0 (`9199a12`).
- `docs/CHANGELOG.md` — 3.0.0+4.0.0 reconstruction (`5031c3d`) + 4.1.0 section (wasm example, HiDPI fix, font, canvas fix).
- `docs/HANDOFF.md`, `docs/NEXT_WORK.md`, `CLAUDE.md` — completion entries + v4.1.0 references.

### rust-survivors (committed local, UNPUSHED)
- `crates/game/Cargo.toml` + `Cargo.lock` — engine pin `60328fa`→`7c6f9c0` (`e6176fa`).

### Memory
- `playtest-windowed-examples.md` — added the wasm/headless-Chrome verification technique (DPR=2 repro, console capture, cache busting).

## Gotchas & Lessons (reusable, cost real time)

- **Wake the display before `screencapture`** (`caffeinate -u`) — black shots otherwise. The memory said this; missing it cost a pass.
- **Headless Chrome defaults to DPR=1 → masks Retina/HiDPI bugs.** Always also test `--force-device-scale-factor=2`. This is *the* lesson — bug #1 was invisible at DPR=1.
- **wasm-bindgen on an empty example `main` → `clone_ref` intrinsics error** (DCE). Fill the entry first.
- **`wasm-bindgen-cli` must exactly match the `wasm-bindgen` crate version** (Cargo.lock).
- **Browser wasm cache** survives `pkg/` rebuilds — fresh `--user-data-dir` to be sure.
- **winit owns the canvas CSS display size on wasm** and beats CSS *rules* with inline style; `!important` CSS or an engine-set inline style (after winit) wins.
- **All engine inputs can be correct while the render is wrong** — the bug was in the *canvas display geometry* (browser layer), not the engine math. Runtime logging of both engine values AND DOM values (canvas.width vs client_width) localized it.
- **Subagent CHANGELOG drafts need verification** — the draft had a version-boundary error + an ungrounded compiler-error claim. Trust git boundaries; verify specifics against docs/commits.
- **macOS window bounds via `unix id is $PID`**, not process name; `caffeinate -u` + `screencapture -R x,y,w,h` for native windows.
- **Sourcing a permissive font via curl**: GitHub `.../raw/...` for the dejavu-fonts repo returns an HTML 404 (TTFs live in releases, not the tree). jsdelivr npm worked: `https://cdn.jsdelivr.net/npm/dejavu-fonts-ttf@2.37.3/ttf/DejaVuSans.ttf` (757KB, valid TrueType) + `/LICENSE`. jsdelivr `gh/google/fonts` paths for Roboto returned 143-byte error stubs (verify with `file` — must say "TrueType"/"OpenType"). `include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/..."))` keeps the asset out of `src/`; `pkg/` rebuild grew ~757KB confirming it's embedded.
- **rust-survivors run**: built in ~7s (engine deps cached even across the v4.0.0→v4.1.0 pin bump — only engine source differs). First launch dropped straight into gameplay (timer 00:18, 2 kills, bat weapon, zombie waves); a fresh relaunch shows the title screen ("RUST SURVIVORS" / 시작 / 캐릭터·스테이지·상점·업적·설정).

## Where We're Going

The wasm coin_race + tagging initiative is essentially done and verified. Remaining work (the PLAN file covers Phases):

1. **Release hygiene** — the `v4.1.0` tag is at `7c6f9c0`, **before** the font (`0cd9b03`) + canvas (`ebd9081`) fixes, so the tagged wasm example renders broken HUD on Retina. Decide: **move `v4.1.0` to HEAD** (recommended — today's tag, safe force-update), OR cut `v4.1.1`, OR leave. Also **push rust-survivors `e6176fa`** (or leave local).
2. **Verify + refresh the `examples/wasm` lib demo (`run_demo`)** — it now benefits from all 3 engine wasm fixes; confirm it renders correctly on Retina and rebuild its committed `pkg/`.
3. **(Optional) wasm render smoke test** — a headless-Chrome screenshot check so the 3-bug class (latent because CI never renders wasm) is caught going forward.
4. **(Carryover) example Korean→English** — 5 files: `mp_server.rs`, `skeletal_puppet.rs`, `touch_demo.rs`, `settings_menu.rs`, `examples/wasm/build.sh`+`index.html`.
5. **(Deferred) reusable remote-entity helper** — only worth it once a 3rd *distinct* networked example confirms the shape (coin_race wasm is the same example).
6. **(Deferred) wasm Retina crispness** — the engine renders wasm at the canvas DOM (logical) size, so it's slightly soft on Retina; a DPR-aware buffer (capped at the WebGL2 2048 limit) would sharpen it. Bigger change.
7. **(Carryover) organize the ~20 rust-survivors WIP docs** — user's call; their repo has uncommitted doc churn.

## Risks & Blockers

- **Tagged v4.1.0 shows a broken wasm example on Retina** (font/canvas fixes are post-tag). Low impact (native consumers unaffected; the *version* 4.1.0 in CHANGELOG describes the working state), but the tag→commit mismatch should be resolved (Phase 1).
- **rust-survivors pin commit is unpushed** — origin still references it at the old rev. No harm; just incomplete.
- **No automated wasm render coverage** — these 3 bugs reached `main` (well, were latent pre-existing) because CI never renders wasm. The protocol probe + native playtest don't cover wasm rendering.
- **`finish_init` canvas lock relies on winit not re-overriding** the inline style after init; verified converging in this session but not stress-tested across resizes.

## Open Questions

- Move `v4.1.0` tag to HEAD vs cut `v4.1.1` vs leave? (Recommended: move — it was created today.)
- Push rust-survivors `e6176fa`?
- Is the `examples/wasm` lib demo worth refreshing now, or leave its stale pkg?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
cat plans/handoffs/PLAN_networking-dogfood_wasm-coin-race-v4.1_2026-06-08.md   # the plan
cat plans/handoffs/HANDOFF_networking-dogfood_wasm-coin-race-v4.1_2026-06-08.md # this file
git log --oneline -6 && git tag | grep v4   # 5031c3d..ebd9081, v4.0.0/v4.1.0

# Verify the engine gate (memory: ci-toolchain-pin → +1.88.0)
./scripts/verify.sh   # or the 5 commands; 311 lib + 5 coin_race_server tests

# Run coin_race in the browser (the verified-working state):
cargo run --example coin_race_server                                  # terminal 1
examples/games/coin_race/web/build.sh                                 # build wasm
python3 -m http.server 8080 --directory examples/games/coin_race/web  # terminal 2
open http://localhost:8080
# Headless render check (Retina repro): Chrome --headless=new --force-device-scale-factor=2
#   --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader --screenshot=out.png URL

# Phase 1 first action — move the v4.1.0 tag to the working HEAD:
#   git tag -f -a v4.1.0 -m "v4.1.0 (incl. wasm font + canvas fixes)" ebd9081 && git push -f origin v4.1.0
#   (confirm with the user first — moving a pushed tag)
```

## Session Status
Engine work committed + pushed (`5031c3d..ebd9081`, tags `v4.0.0`/`v4.1.0`). rust-survivors pin committed local (`e6176fa`, unpushed). Handed off to next session via the paired PLAN file.
