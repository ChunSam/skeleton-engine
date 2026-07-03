# dm-adoption seq 3 — demand pipe FLOWING: all three board requests (EW-004/005/006) served and shipped same-day (v0.114.0 → v0.116.0)

**Date:** 2026-07-03
**Status:** COMPLETED (3 feature/fix PRs merged async-unattended; board updated to Shipped ×3; ball is now with the game session — Verified + `[x]` pending on all three)
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `dm-adoption` seq `3`
**Parent:** `HANDOFF_dm-adoption_seamfix-gamefeel-web_2026-07-03.md`
**Prior chain:** `HANDOFF_game-feel-ui-breadth_2026-07-02.md` > `HANDOFF_game-feel-ui-breadth_widget-suite-text-z_2026-07-02.md` > `HANDOFF_game-feel-ui-breadth_capstone-trim-playtest_2026-07-02.md` > `HANDOFF_dm-adoption_board-notice-automerge_2026-07-02.md` > `HANDOFF_dm-adoption_seamfix-gamefeel-web_2026-07-03.md` > this

---

## Stale References

- Parent's "board still ACTIVE EMPTY (EW-004 next free ID), sixth consecutive empty session" — **obsolete.** The game session ran between seq 2 and this session: EW-004/EW-005 were filed 2026-07-02, EW-006 mid-this-session on 2026-07-03. All three are now `Shipped`; next free ID is **EW-007**.
- Parent's "game repo `main...origin/main [ahead 1]`, `19d43a8` unpushed, pin `1c19873` = v0.62.0" — **all obsolete.** The game session pushed its work, bumped the pin to **v0.113.0 (`95fc472`)**, verified it (241 tests, boot smoke exit 0), and adopted `Button`+focus for its pause menu and City-hub buttons. The game repo has since moved through `feat/engine-bump-0.113` to **`feat/save-load`** (its current branch).
- Parent's "game-session starter prompt in Evidence § for re-issue" — **no longer needed** (the game session ran).
- `src/floating_text.rs` ~L210 `DrawText::centered(...)` with no z — **changed by #337 this session**: the draw site now builds a mutable `DrawText` and assigns `draw.z = ft.z` (~L219–221 post-fix).
- Parent's Code Analysis "smoke port map 8085 centered_text / 8087 game_feel" still holds (no new smokes this session).

## Since Last Handoff

- **Parent §1 (land seq-2 handoff async + memory seq 156)** — happened exactly per the pattern: PR #336 auto-merged (`fbf5640`, 2026-07-02T23:32:37Z, 5/5 checks) minutes after the parent session closed; the deferred wrap-up (pull, branch delete, seq 156) was executed at THIS session's start. Second consecutive clean cross-session handshake.
- **Parent §2 ("the game session is the blocking dependency")** — UNBLOCKED. The user ran the game session between seq 2 and seq 3. It bumped the pin 51 minors (v0.62.0 → v0.113.0) in one go — "fully additive for our used surface" per their board note — and completed two adoption slices (pause menu, City-hub buttons).
- **Parent §3 (board FIRST, serve EW-004+ priority-order)** — executed fully: EW-004 (P2) → #337 v0.114.0, EW-005 (P3) → #338 v0.115.0, plus **EW-006 (P3) which the game session filed MID-session** while this session was working — spotted during the board-update step, served same-day → #339 v0.116.0.
- **Parent §4 (if board STILL empty: ASK)** — never triggered; the board had items.
- **Parent §5 (async-mode discipline)** — held: 3 more unattended landings (arm → merge ~11/~15/~15 min), wrap-ups batched at natural checkpoints, every probe via `gh pr view --json state,mergedAt`. Chain total now **7 unattended landings**.
- Parent's open question "does the game bump straight to v0.113.2 or in stages?" — answered: **one jump to v0.113.0** (not .2), verified on first build.
- Parent risk "0.62→0.113 is 51+ minors for the eventual game-side bump" — evaporated: the bump was clean ("the only technically-breaking items are new `UiEvent` variants, which we didn't match on").

## Reference Documents

- `CLAUDE.md` — header now **v1.6.209 / package v0.116.0**; FloatingText row (+`with_z`), UI row (Button builders), save row (text envelope) all updated this session.
- `docs/CHANGELOG.md` — new `## 0.114.0`, `## 0.115.0`, `## 0.116.0` entries (the 0.116.0 entry is the migration guide for the envelope format change).
- `../dungeon-merchant/docs/engine-wishlist.md` — the shared board; all three EW items now `Shipped` with `[Engine]` 2026-07-03 replies containing verification guidance.
- `.claude/skills/land-pr/SKILL.md` Async mode + `.claude/skills/ship/SKILL.md` — followed verbatim ×3.
- Memory `engine-current-state.md` — bumped 4× this session (seq 155 → 156 → 157 → 158 → 159); **seq 160 belongs to THIS handoff PR's merge** (next session's deferred wrap-up).

## The Goal

Continue the `dm-adoption` arc: the Dungeon-Merchant game session files engine requests as EW-NNN items on the shared board, and the engine session serves them priority-order — this is the engine's demand pipe, replacing self-picked work. Seq 1 opened the arc and posted the board notice; seq 2 exhausted the sanctioned self-picks while waiting. This session is the arc's payoff: the game session finally ran, the pipe flowed (three requests), and the mandate was to serve them in priority order via the established feature+example+verify+async-land loop, then hand the ball back via board `Shipped` replies.

## Where We Are

- **main @ `c42a890`** — package **v0.116.0**, CLAUDE.md header **v1.6.209**, tree clean, memory at **seq 159**.
- **3 feature/fix PRs merged this session**, all async-unattended: **#337** FloatingText z passthrough (v0.114.0, `8840980`), **#338** Button styling parity (v0.115.0, `ace83a6`), **#339** versioned-save enum fidelity (v0.116.0, `c42a890`). Plus #336's deferred wrap-up at session start.
- **EW-004 (P2) shipped as requested:** `FloatingText.z: Option<f32>` + `with_z(z)` builder; `FloatingTextSystem` assigns `draw.z = ft.z` on the `DrawText` it queues (the 0.110 layered-text machinery). `None` default = historical on-top pass, byte-identical. Unblocks the game's `popups.rs` → `FloatingText` migration (their scrim is z=100; floats `with_z(<100)` hide behind it).
- **EW-004 regression coverage:** render test `floating_text_with_z_hides_under_a_higher_z_rect` (tests/render.rs, runs on CI lavapipe) — covered layered float vanishes, uncovered control renders, default no-z float stays on top; plus 2 unit tests (default queues `z: None`; `with_z(50.0)` reaches the queued `DrawText`).
- **EW-004 example:** `examples/floating_text.rs` gains **P** (toggle a z-100 overlay scrim, `SCRIM_RECT` = full-width band y 150..340, alpha 0.94) and **Z** (new pops layered `with_z(50)` [default] vs legacy on-top); headless auto mode raises the scrim at frame 30 (`AUTO_SCRIM_FRAME`). Capture verified: numbers dimmed to near-invisible under the band, crisp above its top edge.
- **EW-005 (P3) shipped as requested:** `Button` gains `with_colors(normal, hovered, pressed)` / `with_disabled_color` / `with_text_color` / `with_font_size` / `with_corner_radius` builders + `corner_radius: f32` field. `button_pass` pushes the bg rect `.with_corner_radius(...)` through the UI SDF pipeline (`0.0` = sharp fast path, byte-identical). Reflect `fields()`/`set_field` extended so the editor Inspector can edit the radius. The board's acceptance line compiles: `Button::new("x").with_font_size(22.0).with_corner_radius(8.0)`.
- **EW-005 tests:** builder chain sets every field; old RON without `corner_radius` still loads (fixture strips the field and asserts it actually stripped); reflect roundtrip incl. radius; `UiSystem`-level test that the radius reaches the queued `DrawRect` (19 button-matching tests green).
- **EW-005 example:** `examples/ui_rounded.rs` — both buttons now rounded (radius 12) + fully builder-styled (Play: font 20 + light text; Quit: custom dark-red 3-state colors + pink text); example gained a standard `HEADLESS_SHOT` path (10-frame default). Capture verified.
- **EW-006 (P3) — filed MID-session by the game's save/load work, served same-day:** `save_versioned` envelope now `(version: u32, format: 1, data: "<payload RON text>")`. The pre-0.116 envelope parsed the payload into a `ron::Value`, which cannot represent enum variants — `Closed(reopen_day: 3)` degraded to a bare map at save time and failed at load with "expected enum, found map" (silent until load, the game's exact complaint).
- **EW-006 load semantics:** `(format 1, Value::String)` + stored == current → `ron::from_str::<T>(&text)` directly, **no Value hop → full serde fidelity incl. data-carrying enums**; stored < current → text parses to `ron::Value` for the migration steps (their enum limitation documented + test-pinned); `(format 1, non-String)` → `SaveError::Corrupted`; format absent/0 → legacy tree path unchanged (pre-0.116 saves load + migrate).
- **EW-006 tests (21/21 save tests green):** `versioned_enum_struct_variant_roundtrips_at_current_version` (the game's exact `Vec<(u32, MarketStatus)>` shape — fails pre-fix), `versioned_legacy_tree_envelope_still_loads_and_migrates` (a local `LegacyEnvelope` struct replicates the pre-0.116 writer byte-for-byte, then migrates 1→2 with defaulted `coins`), `versioned_enum_needing_migration_errors_instead_of_corrupting` (pins the documented constraint).
- **EW-006 example:** `examples/save_migration.rs` current format gains `mode: GameMode` (`#[derive(Default)]`, `Normal` / `Custom { multiplier: f32 }`) with `#[serde(default)]` so a migrated v1 save loads as `Normal` without the migration step having to synthesize an enum; the Space re-save round-trip carries `Custom { multiplier: 1.5 }` — exercises the fix in real play.
- **Board fully updated in ONE game-side commit** (`71503a1` on the game's `feat/save-load` branch, docs-only, `[Engine]` prefix, **NOT pushed** — the game session pushes): EW-004 → `Shipped (v0.114.0)`, EW-005 → `Shipped (v0.115.0)`, EW-006 → `Shipped (v0.116.0)`, each with an `[Engine]` 2026-07-03 thread reply naming the test/example and how to verify. EW-006's reply carries two loud caveats (see Risks).
- **Full 7-gate verify ran 4× this session, all `VERIFY_EXIT=0`** (onboarding baseline on main + one per feature branch post-paperwork), every exit read non-piped with the sentinel + `pgrep` liveness check.
- **Memory:** seqs 156 (#336 wrap), 157 (#337), 158 (#338), 159 (#339) bumped in both `engine-current-state.md` and `MEMORY.md`; the board-state line rewritten twice (EMPTY → EW-004/005 ACTIVE → all Shipped).
- Local branches `feat/floating-text-z`, `feat/button-styling`, `fix/versioned-save-enum-fidelity`, `docs/handoff-dm-adoption-seq2` all deleted after merge; `git fetch --prune` at session start cleared 15 stale remote-tracking refs.

## What We Tried (Chronological)

1. **Onboarding (paste-prompt protocol, 5th session running).** The named handoff file did not exist in the working tree — local main (`3e0645c`) was behind origin. Located it via `git log --all --diff-filter=A -- "*seamfix*"` → commit `1f28bc8` on `docs/handoff-dm-adoption-seq2`; `gh pr view 336` showed **MERGED** (5/5 checks). Read the handoff from the git object (`git show 1f28bc8:plans/…`) *without* pulling — respecting the wait-for-go-ahead instruction. Board check FIRST: **the game session had run** (EW-004 P2 + EW-005 P3 filed; pin v0.113.0 `95fc472` verified; adoption slices 1–2 done). Verify baseline launched in background on `3e0645c` (valid: the only delta to origin was the docs-only #336) → `VERIFY_EXIT=0`. Read key files (floating_text.rs full, `DrawText.z` in text/queue.rs, button.rs, tab_bar.rs as the builder-convention reference, button_pass.rs render site). Presented the 4-step plan; got "진행해".
2. **#336 deferred wrap-up.** `git pull --ff-only` (`3e0645c → fbf5640`), branch delete, `fetch --prune` (15 stale refs), memory seq 156 + board-state rewrite in both memory files.
3. **EW-004 implementation (branch `feat/floating-text-z`).** Field + builder + `draw.z = ft.z` passthrough; 2 unit tests; render regression test modeled on `layered_text_is_covered_by_higher_z_rect` — key trick: **no `Camera` resource → `FloatingTextSystem` uses `Transform.position` as screen coordinates directly**, so text lands exactly where the pixel-regions expect; a `still()` fixture (velocity ZERO, fade off, lifetime 100 s, size 32) keeps the texts put across the 4 warm-up frames. Test passed first run on the local real GPU.
4. **EW-004 example + capture.** P/Z toggles added; the 0.94-alpha scrim dims covered layered numbers to near-invisible while "CRIT 27!" emerging above the band renders crisp — exactly the pause-scrim story. One tool-use lesson re-learned: **Edit requires a Read of the target file in-session even when its content was shown in a system-reminder** (first CLAUDE.md edit bounced).
5. **/ship v0.114.0 + land async #337.** Four-file paperwork; background verify `VERIFY_EXIT=0`; commit `8bf6125`; PR #337; `gh pr merge --auto --squash`; moved on immediately.
6. **EW-005 implementation started while #337's CI ran** (branch `feat/button-styling` off pre-#337 main `fbf5640` — the deliberate stacked-branch pattern from seq 2). Builders + field + Reflect + button_pass threading. First version of the `UiSystem`-level test used `r.position` — **rustc diagnostic caught that `DrawRect` has `x`/`y` fields, not `position`** — fixed by find on `r.x == 10.0 && r.y == 10.0`. `ui_rounded` extended (both buttons builder-styled + NEW `HEADLESS_SHOT` path); capture verified.
7. **#337 merged mid-work** (`8840980`, ~11 min after arming — fastest in the chain). Deferred wrap-up + memory seq 157. **Rebase-before-paperwork ordering** (the seq-2 lesson, applied verbatim): WIP commit → checkout main → pull → rebase `feat/button-styling` (clean, 1 commit) → THEN /ship v0.115.0 so the version line (0.114.0 → 0.115.0) never conflicts → verify `VERIFY_EXIT=0` → `git reset --soft main` → one clean commit `a4ebea7` → PR #338 → armed. `ScheduleWakeup` 600 s.
8. **Wakeup: #338 MERGED** (`ace83a6`, 02:02:07Z, ~15 min). Wrap-up + memory seq 158. Re-reading the board for the update step **revealed EW-006, filed mid-session** (2026-07-03) by the game's save/load work — and the game repo had moved from `feat/engine-bump-0.113` to `feat/save-load`. Decision: serve it same-session (board priority-serve is the arc's standing rule), and batch the board commit to ONE at session end instead of two.
9. **EW-006 implementation (branch `fix/versioned-save-enum-fidelity`).** Read `src/save.rs` L380–560: the envelope did payload → RON string → `ron::from_str::<ron::Value>` → envelope, and load did `Value.into_rust::<T>()`. Chose the **format-marker design** over a marker-less try-parse fallback (see Key Decisions). Implemented envelope + both load paths + 3 tests + fn-doc constraint notes + `save_migration` example enum field. All 21 save tests green first run.
10. **/ship v0.116.0 + land async #339.** MINOR (not PATCH — on-disk format change); verify `VERIFY_EXIT=0`; commit `35a9d43`; PR #339; armed; `ScheduleWakeup` 720 s. **#339 merged** (`c42a890`, 02:36:50Z). Wrap-up + memory seq 159.
11. **Single board update.** Three status flips + three `[Engine]` replies with verification pointers; "Next free ID: EW-007" was already correct (the game session updates it when filing). Committed game-side `71503a1` on `feat/save-load` (docs-only), NOT pushed. Final Korean report delivered.

## Key Decisions

- **Served EW-006 without a fresh user go-ahead.** The approved session plan listed only EW-004/005 + board update, but the arc's standing rule ("the engine side prioritizes board requests over self-picked work", written on the board itself and endorsed across seqs 1–2) durably authorizes serving filed board items. EW-006 was P3, small, and had explicit acceptance options. The alternative — parking it and asking — would have burned a session-boundary for a request the pipe explicitly exists to serve.
- **EW-006 fix shape: envelope `format` marker + payload as RON text**, over two rejected alternatives: (a) *marker-less try-parse fallback* (treat `Value::String` as maybe-text) — rejected because an old-format save whose payload was itself a string containing valid RON string syntax would silently strip quotes (a tiny but real silent-corruption vector); (b) *docs-only* (the game's acceptance option B alone) — rejected because the full fix was ~40 lines and their acceptance option A explicitly suggested the text envelope.
- **`#[serde(default)] format: u32` on the owned envelope + `data` kept as `ron::Value`** — this is what makes BOTH formats parse with one deserialize: a text envelope reads `data` as `Value::String`, a legacy envelope as the tree, and a missing `format` field defaults to 0 (legacy). No sniffing, no second parse.
- **Migration-steps enum constraint documented + test-pinned rather than solved.** `SaveMigrator` steps are `Fn(ron::Value) -> ron::Value` — public API; carrying enums through them needs a different value model entirely. The constraint is now loud (both fn docs + board reply + a test asserting it errors instead of silently corrupting). Workaround documented: keep enum fields stable across schema versions, or mirror to structs while a migration is pending.
- **EW-006 bumped MINOR (v0.116.0), not PATCH** despite being a bugfix — the envelope format changes (old builds can't read 0.116+ saves), which is more than a point fix under the pre-1.0 rule's spirit; the CHANGELOG entry doubles as the migration note.
- **EW-005 `with_colors` takes the 3 interaction-state colors** `(normal, hovered, pressed)` with `with_disabled_color` separate — a 4-arg builder is clunky and disabled is rarely styled; mirrors TabBar's grouped-primary + separate-secondary convention.
- **Button label text intentionally NOT given per-corner styling or z changes** — EW-005 was scoped to the styling surface; the label already layers at widget z since 0.110.
- **Stacked-branch cadence reused from seq 2, now twice-proven:** branch the next feature off pre-merge main while CI runs → parent merges → rebase → THEN version paperwork → `reset --soft main` → one clean commit. The version line never conflicts; zero wall-clock wasted on CI waits.
- **ONE game-side board commit at session end** instead of one per landed PR — fewer commits on the game's active feature branch, one coherent `[Engine]` message; acceptable because the game session wasn't concurrently reading the board mid-day.
- **EW-004 render test reuses the layered-text scene geometry verbatim** (covering rect regions, thresholds `visible > 100`, `covered < visible/20`) — same invariants, so a future tolerance retune touches both tests identically.

## Evidence & Data

### PRs / commits this session

| PR | Merge hash | Version | Serves | Landing | Arm → merge |
|---|---|---|---|---|---|
| #336 (parent's handoff) | `fbf5640` | — (docs) | — | auto-merge, armed by seq-2 session | merged 2026-07-02T23:32:37Z, before this session |
| #337 FloatingText z | `8840980` | v0.114.0 | EW-004 (P2) | auto-merge, **unattended** | ~11 min (merged 01:40:58Z) |
| #338 Button styling | `ace83a6` | v0.115.0 | EW-005 (P3) | auto-merge, **unattended** | ~15 min (merged 02:02:07Z) |
| #339 save enum fidelity | `c42a890` | v0.116.0 | EW-006 (P3) | auto-merge, **unattended** | ~15 min (merged 02:36:50Z) |

Chain async-landing total: **7 unattended** (#333–#339 minus the user-watched early ones; 5 in seqs 2–3 alone).

### Version / header / memory progression

| Point | main | package | CLAUDE.md header | memory seq |
|---|---|---|---|---|
| Session start (local view) | `3e0645c` | v0.113.2 | v1.6.206 | 155 |
| After #336 wrap-up | `fbf5640` | v0.113.2 | v1.6.206 | 156 |
| After #337 | `8840980` | v0.114.0 | v1.6.207 | 157 |
| After #338 | `ace83a6` | v0.115.0 | v1.6.208 | 158 |
| After #339 (now) | `c42a890` | v0.116.0 | v1.6.209 | 159 |

Seq 160 = this handoff PR's merge (next session's deferred wrap-up).

### Verify / test log (all exits read non-piped, sentinel + pgrep confirmed)

| Run | Result |
|---|---|
| Onboarding baseline (main `3e0645c`) | `VERIFY_EXIT=0` |
| #337 branch (post-paperwork) | `VERIFY_EXIT=0` |
| #338 branch (post-rebase, post-paperwork) | `VERIFY_EXIT=0` |
| #339 branch (post-paperwork) | `VERIFY_EXIT=0` |
| floating_text unit tests | 8 passed (2 new) |
| render test `floating_text_with_z_…` (local real GPU) | ok in 5.44 s |
| button-matching unit tests | 19 passed (3 new) |
| save::tests | 21 passed (3 new) |

### Board state transition (the session's core outcome)

| Item | At onboarding | At close | Ships as |
|---|---|---|---|
| EW-004 FloatingText z (P2) | `Proposed` (2026-07-02) | `Shipped (v0.114.0)` + [Engine] reply | #337 |
| EW-005 Button styling (P3) | `Proposed` (2026-07-02) | `Shipped (v0.115.0)` + [Engine] reply | #338 |
| EW-006 save enum fidelity (P3) | **did not exist** (filed mid-session 2026-07-03) | `Shipped (v0.116.0)` + [Engine] reply | #339 |
| Next free ID | EW-006 → EW-007 (game updated when filing) | EW-007 (unchanged, was already correct) | — |

Game-side board commit: `71503a1` on `feat/save-load` (docs-only, `[Engine]` prefix), branch now `[ahead 1]`, **NOT pushed**.

### Game repo facts (as observed this session)

| Fact | Value |
|---|---|
| Pin | `rev = "95fc472…"` = **v0.113.0** (bumped from v0.62.0 by the game session, "fully additive for our used surface") |
| Game verification of the bump | `cargo build` 0 warnings · `cargo test` 241 · boot smoke exit 0, first build |
| Adoption done game-side | pause menu → `Button`+focus (slice 1); City-hub 4 action buttons + `UiSystem` moved to top of frame (slice 2) |
| Game-side findings (board notes) | game-feel toolkit mostly N/A (pure immediate-mode UI game — zero `Sprite`/`Transform`/`Camera` entities; only `FloatingText` fits, was blocked on EW-004); `Button` is single-line-label-only (observation, NOT filed) |
| Branch at onboarding → at close | `feat/engine-bump-0.113` → `feat/save-load` (game session actively working between my checkpoints) |

### EW-004 render test geometry (480×240, clear `[0.05,0.05,0.07,1]`, DejaVu injected, 4 frames)

| Element | Position/region | z |
|---|---|---|
| "COVERED" float (`with_z(0.2)`) | centered (130, 105) | 0.2 — under the rect |
| covering rect (bg-colored) | (20, 70, 220×70) | 1.0 |
| "VISIBLE" control float (`with_z(0.2)`) | centered (130, 190) | 0.2 — uncovered |
| "ONTOP" float (default, no z) | centered (370, 105) | None — over a z=50 rect (280, 70, 180×70) |
| Assertions | `visible > 100`, `ontop > 100`, `covered < visible/20` | same thresholds as the 0.110 layered-text test |

Fixture: `still(text)` = `velocity ZERO` + `fade(false)` + `lifetime 100.0` + `size 32`; **no Camera resource** → Transform position is used as screen coords directly (deterministic placement).

### EW-006 envelope — old vs new

| | Pre-0.116 (legacy) | 0.116+ (format 1) |
|---|---|---|
| On-disk shape | `(version: N, data: <Value tree>)` | `(version: N, format: 1, data: "<payload RON text>")` |
| Enum struct variant at save | degraded to bare map (variant name LOST) | preserved verbatim in the text |
| Load, stored == current | `Value.into_rust::<T>()` — "expected enum, found map" | `ron::from_str::<T>(&text)` — full fidelity |
| Load, stored < current (steps run) | Value hop (enum-lossy) | text → Value hop (enum-lossy, **documented + test-pinned as an error**) |
| Read by 0.116 | ✓ (legacy path, `format` defaults 0) | ✓ |
| Read by pre-0.116 build | ✓ | ✗ (forward compat never promised — **game must bump pin before shipping new saves**) |
| format 1 + non-string data | — | `SaveError::Corrupted` |

### Branch / commit mapping

| Branch | Base | Final commit | PR |
|---|---|---|---|
| `feat/floating-text-z` | `fbf5640` | `8bf6125` | #337 |
| `feat/button-styling` | `fbf5640`, rebased onto `8840980` mid-work | `a4ebea7` (after `reset --soft main`) | #338 |
| `fix/versioned-save-enum-fidelity` | `ace83a6` | `35a9d43` | #339 |

All deleted locally post-merge; remote heads auto-deleted (`delete_branch_on_merge`).

### `[Engine]` board replies (condensed — re-apply from here if `71503a1` is ever lost)

- **EW-004 reply:** Shipped v0.114.0 (PR #337). `FloatingText.z: Option<f32>` + `with_z(z)`, passed through to the queued `DrawText` (same semantics as `DrawText::z`); default `None` = on-top, byte-identical. Acceptance = CI render test `floating_text_with_z_hides_under_a_higher_z_rect`. "For your scrim at z=100: spawn floats `with_z(<100)`." Recipe `examples/floating_text.rs` (P scrim / Z mode toggles). Set `Verified` + `[x]` after confirming against the bump.
- **EW-005 reply:** Shipped v0.115.0 (PR #338). "Your acceptance line compiles and renders: `Button::new("x").with_font_size(22.0).with_corner_radius(8.0)`." Full builder set listed; radius rides the TabBar/DrawRect SDF path, `0.0` = sharp byte-identical, old scene RON loads via struct-level `#[serde(default)]`, Reflect field → editor Inspector. Recipe `examples/ui_rounded.rs`.
- **EW-006 reply:** Shipped v0.116.0 (PR #339), same day. "Went with your acceptance option A+B combined": RON-text envelope (`format: 1`); at current version `load_migrated` deserializes straight from text — their `MarketStatus::Closed { reopen_day }` round-trips (regression test uses their exact `Vec<(u32, MarketStatus)>` shape; "you can drop the `SavedMarket` mirror"). **Two loud caveats:** (1) `SaveMigrator` steps still see a `ron::Value` → an enum-carrying payload that *needs migrating* errors instead of silently corrupting (keep enum fields stable across versions or mirror while a migration is pending); (2) 0.116 reads old saves, but **older builds can't read 0.116+ saves — bump the pin before shipping new saves**. Verify by restoring the enum and re-running their `engine_envelope_round_trip_and_version_tag`.

### Wall-clock cadence (KST; session ≈ 09:55–11:45, 3 shipped PRs + board in <2 h)

| Checkpoint | Time (KST) |
|---|---|
| #336 merged (before session, by seq-2's arming) | 08:32 |
| Session start / onboarding / baseline verify | ~09:55 |
| Go-ahead ("진행해") → wrap-up + EW-004 work | ~10:10 |
| #337 armed → merged | ~10:29 → 10:40 (~11 min) |
| #338 armed (EW-005 written during #337 CI) → merged | ~10:47 → 11:02 (~15 min) |
| EW-006 discovered on the board → #339 armed → merged | ~11:05 → ~11:21 → 11:36 (~15 min) |
| Board triple-update committed game-side + final report | ~11:45 |

### Headless captures (scratchpad, session-local — re-capture via HEADLESS_SHOT if needed)

- `…/scratchpad/floating_text_z.png` — scrim ON: layered numbers dimmed to near-invisible inside the 0.94-alpha band, "CRIT 27!" crisp above the top edge, HUD (no-z) crisp over everything.
- `…/scratchpad/ui_rounded_buttons.png` — rounded Play (radius 12, font 20) + rounded dark-red Quit; card/swatches/focus-ring unchanged.

### Async checkpoint pacing (as run)

- Local verify (~6–8 min): background task + harness completion notification; sentinel `VERIFY_EXIT=` + `pgrep verify.sh` on every completion (the spurious-notification protocol, no spurious ones this session).
- GitHub CI: `ScheduleWakeup` 600 s (#338) / 720 s (#339) then one `gh pr view <n> --json state,mergedAt` probe — `state=MERGED` + `mergedAt` authoritative, `mergeStateStatus` read `UNKNOWN` right after merge (ignored, as documented in seq 2).
- Real work proceeded between arm and merge both times (#338's code written during #337's CI; the board recon during #338's).

## Code Analysis

- **`FloatingText.z` passthrough site** (`src/floating_text.rs` system loop): `let mut draw = DrawText::centered(…); draw.z = ft.z;` — direct field assign handles `None` uniformly (no conditional builder call). `DrawText.z: Option<f32>` semantics from 0.110: `Some(z)` composites among UI rects at z (tie → text over surface), `None` = final on-top pass.
- **`Button` render tuple** (`src/ui/system/button_pass.rs` second loop) now `(color, label_text, text_color, font_size, corner_radius)`; bg rect `DrawRect::new(…).with_corner_radius(corner_radius).with_z(z)` — `with_corner_radius(0.0)` is byte-identical to not calling it (SDF fast path), so no branch needed.
- **`DrawRect` fields are `x`/`y`/`w`/`h`, NOT `position`/`size`** (`src/renderer/ui.rs:6-20`) — caught by rustc mid-session; `UiQueue.items: Vec<DrawRect>` is pub (tests read it directly).
- **`UiSystem` output path:** all passes write into a per-frame `UiOutput`; `submit_output` (`src/ui/system/state.rs:163`) pushes rects → `UiQueue`, texts → `TextQueue`, events → `Events<UiEvent>` — each only if the resource exists. A `UiSystem`-level widget test therefore needs `world.insert_resource(UiQueue::default())` to observe rects.
- **Versioned envelope internals** (`src/save.rs`): `VersionedEnvelope<'a> { version, format, data: &'a str }` (write) / `VersionedEnvelopeOwned { version, #[serde(default)] format, data: ron::Value }` (read); `const ENVELOPE_FORMAT_TEXT: u32 = 1`. Load match `(format, data)`: `(1, String)` → direct or via-Value by `stored == current`; `(1, _)` → `Corrupted`; `(_, tree)` → legacy.
- **`into_rust` honors `#[serde(default)]` on missing map keys** — this is what lets the `save_migration` example's migrated v1 payload (no `mode` key) load as `GameMode::Normal` without the migration step synthesizing an enum (which `ron::Value` cannot express).
- **`ron::Value` (0.8) has NO enum representation** — root cause of EW-006: `from_str::<Value>("Closed(reopen_day: 3)")` yields a Map and drops the variant name, so fidelity dies at *save* time in the old format; no loader change alone could fix it.
- **Headless-capture idiom for UI examples** (now in floating_text + ui_rounded): `HEADLESS_SHOT=path` env → `app.save_screenshot_headless(frames, &out)`; default frames 70 (animated demo) / 10 (static UI).
- **New public API added this session (complete list):**
  - `FloatingText::with_z(self, z: f32) -> Self` + `pub z: Option<f32>` (v0.114.0)
  - `Button::with_colors(self, normal: Color, hovered: Color, pressed: Color) -> Self`, `with_disabled_color(self, Color)`, `with_text_color(self, Color)`, `with_font_size(self, f32)`, `with_corner_radius(self, f32)` + `pub corner_radius: f32` (v0.115.0)
  - No new public save API (v0.116.0 changes are internal envelope + docs; `save_versioned`/`load_migrated` signatures unchanged)
- **The seq-154 UiQueue-accumulation test gotcha did NOT apply here** — the new Button geometry test runs `UiSystem` exactly once, so no inter-frame `.items.clear()` was needed (the gotcha only bites multi-frame rect-counting tests).
- **wasm parity for EW-006 needed zero extra work:** the envelope logic lives above the storage backends (`save_with_key`/`load_with_key` handle file vs localStorage), so the text envelope works identically on wasm; the wasm build gate in verify covers compilation, and the `wasm_save` example's versioned self-check (a plain struct payload) is behaviorally unchanged.
- **Onboarding adjacent-file findings that shaped the work:** `DrawText.z` docs confirmed the exact tie-break semantics ("on a z tie the text draws over the rect") reused for the FloatingText docs; `tab_bar.rs` supplied the builder-set convention EW-005 mirrors (grouped primary colors + separate hover/secondary); `focus_pass.rs:676` comment "a plain Button draws no other rects" confirmed the bg rect is the only surface needing the radius.

## Files Changed

### PR #337 (merged `8840980`, v0.114.0 — EW-004)
- `src/floating_text.rs` — `z: Option<f32>` field + `with_z` builder + passthrough; 2 new unit tests.
- `tests/render.rs` — `FloatingTextCoverScene` + `floating_text_with_z_hides_under_a_higher_z_rect`; import list gains `spawn_floating_text`/`FloatingText`/`FloatingTextSystem`.
- `examples/floating_text.rs` — P scrim toggle, Z layered/on-top toggle, `AUTO_SCRIM_FRAME`, HUD mode line, doc header.
- `Cargo.toml`/`Cargo.lock`/`docs/CHANGELOG.md` (`## 0.114.0`)/`CLAUDE.md` (header v1.6.207 + FloatingText row) — /ship paperwork.

### PR #338 (merged `ace83a6`, v0.115.0 — EW-005)
- `src/ui/button.rs` — `corner_radius` field, 5 builders, Reflect extension; 3 new/extended tests.
- `src/ui/system/button_pass.rs` — corner_radius threaded into the bg `DrawRect`.
- `src/ui/system.rs` — `button_corner_radius_reaches_the_queued_rect` test.
- `examples/ui_rounded.rs` — builder-styled rounded Play/Quit + `HEADLESS_SHOT` path + doc header.
- `Cargo.toml`/`Cargo.lock`/`docs/CHANGELOG.md` (`## 0.115.0`)/`CLAUDE.md` (header v1.6.208 + UI-row Button note) — /ship paperwork.

### PR #339 (merged `c42a890`, v0.116.0 — EW-006)
- `src/save.rs` — text envelope (`format: 1`), dual-path `load_migrated_with_key`, `ENVELOPE_FORMAT_TEXT`, constraint docs on both public fns.
- `src/save/tests.rs` — `MarketStatus`/`MarketSave` fixtures + 3 new tests.
- `examples/save_migration.rs` — `GameMode` enum + `#[serde(default)] mode` field + Space round-trip carries `Custom { multiplier: 1.5 }`; doc header.
- `Cargo.toml`/`Cargo.lock`/`docs/CHANGELOG.md` (`## 0.116.0`)/`CLAUDE.md` (header v1.6.209 + save row) — /ship paperwork.

### Game repo (committed `71503a1` on `feat/save-load`, NOT pushed)
- `docs/engine-wishlist.md` — EW-004/005/006 → `Shipped (vX.Y.Z)` + three `[Engine]` 2026-07-03 replies.

### Memory (outside repo)
- `engine-current-state.md` — seqs 156/157/158/159; board-state line rewritten twice (EMPTY → EW-004/005 ACTIVE → served).
- `MEMORY.md` — engine-current-state index line kept in sync at each bump.

## User Feedback & Preferences (REQUIRED)

- **Onboarding paste-prompt protocol, 5th session running** — narrated onboarding demanded again (summary → verify plan → key files + adjacents → planned first action → WAIT), this time with an explicit warning baked into the prompt: "the self-pick queue is EXHAUSTED — if the board is still empty, ASK for direction instead of picking work." The board had items, so the ASK branch never fired.
- **"진행해"** — one terse go-ahead for the 4-step plan; **zero mid-session intervention afterward** across 3 PRs, 4 wrap-ups, 4 memory bumps, and the board update. Second fully-autonomous multi-PR run in the chain, now with a mid-session scope addition (EW-006) absorbed under the standing board-priority rule without a check-in — the user's /handoff invocation right after the final report, with no correction, reads as acceptance of that call.
- **Board-first discipline continues to be the user's core process demand** — the paste prompt again ordered "board check FIRST — EW-004+ may now exist if the game session ran" ahead of even the PR-merge check.
- **/handoff invoked explicitly at session end** — the user closes sessions through the skill, never freeform summaries.
- **Standing (unchanged):** user-facing Korean / repo artifacts English; merge authority delegated (squash on green, direct instruction); never push either repo's main (and now: never push the game repo at all — its session pushes); `cargo fmt` before verify; gate exits read non-piped (zsh `$pipestatus` 1-indexed); pre-1.0 MINOR/PATCH rule; explicit `model:` on any subagent (none needed this session); the deferred-wrap-up handshake for async landings.

## Where We're Going

1. **Land this handoff** as its own `docs(handoff)` PR via async auto-merge; bump memory to **seq 160** when it merges (next session's deferred wrap-up, the established pattern).
2. **The ball is with the game session on all three EW items.** It should: bump the pin v0.113.0 → v0.116.0, verify each item (EW-004: migrate `popups.rs` → `FloatingText::with_z`; EW-005: builder-style its buttons; EW-006: restore the enum in `SaveData.markets`, drop the `SavedMarket` mirror, re-run `engine_envelope_round_trip_and_version_tag`), then set `Verified` + `[x]` and push `71503a1` along with its branch.
3. **Next engine session: board FIRST, as always.** Expected states: `Verified` flips to archive, and/or new EW-007+ items (the game's save/load and City-hub work keeps generating candidates — their "Button is single-line-label-only" observation is a likely future EW).
4. **If the board has nothing actionable** (all three still `Shipped` awaiting game verification, no new items): that is NOT an empty board — the correct move is to tell the user the pipe is in the game's court and ask whether to run the game session, wait, or take a user-chosen direction. Do NOT invent engine work.
5. **Keep the async-landing discipline** — 7 unattended landings and counting; sentinel + pgrep on every background completion; `state`+`mergedAt` as the only authoritative merge probe.

## Risks & Blockers

- **Save-format forward-compat trap:** a pre-0.116 engine build cannot read 0.116+ saves. If the game ships builds to players before bumping its pin, saves written by a dev build on 0.116 would be unreadable by the shipped build. Mitigation: flagged loudly in the EW-006 board reply ("bump the pin before shipping new saves") and in CHANGELOG 0.116.0.
- **Board commit `71503a1` sits unpushed on the game's active `feat/save-load` branch.** If the game session hard-resets or abandons that branch, the board update is lost. Mitigation: the full reply text is reproduced in this handoff's Evidence, and the memory tip line records the Shipped states — either session can re-apply.
- **EW-006's migration-path constraint may bite later:** the first time the game adds a `SaveMigrator` step while `MarketStatus` (or any enum) is in the payload, loads of *old-version* saves will error. Documented in both fn docs + the board reply + pinned by `versioned_enum_needing_migration_errors_instead_of_corrupting`; the workaround (mirror during pending migrations) is spelled out.
- **The game session and engine session now genuinely interleave** (EW-006 appeared mid-session; the game repo's branch changed between my checkpoints). Board edits should keep being single atomic commits, and re-read the board immediately before editing it.

## Open Questions

- **Will the game bump straight to v0.116.0 or verify EW-004/005 on v0.113.0 first?** (EW-004/005 need ≥0.114.0/0.115.0 to verify; EW-006 needs 0.116.0 — one jump is simplest, their call.)
- **Does `popups.rs` get deleted after the FloatingText migration** (their stated intent in the EW-004 thread) — and does that surface new gaps (e.g. per-popup fonts)?
- **Is "Button multi-line label / chrome-less hit-area" the next EW?** (Their slice-2 note said observation-only, "not filing until there's a concrete need.")

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -5          # tip = docs(handoff) dm-adoption seq 3 over c42a890 (#339)
git status -s                 # clean

# Board FIRST — expect Verified flips on EW-004/005/006 and/or new EW-007+.
sed -n '50,120p' ../dungeon-merchant/docs/engine-wishlist.md
git -C ../dungeon-merchant log --oneline -5   # did the game session push 71503a1? bump the pin past 95fc472 (v0.113.0)?
grep -n 'rev = ' ../dungeon-merchant/Cargo.toml

# Deferred wrap-up if this handoff's PR has merged: pull, delete branch, memory seq 160.

# Verify baseline (7 gates; read exit non-piped; 2 audio tests may fail on a no-device box)
cargo fmt && ./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?

# Key files from this session
#   src/floating_text.rs                  — z passthrough (EW-004, v0.114.0)
#   src/ui/button.rs + src/ui/system/button_pass.rs — builder surface + radius (EW-005, v0.115.0)
#   src/save.rs (L380-600)                — text envelope, dual-path load (EW-006, v0.116.0)
#   tests/render.rs                       — floating_text_with_z_hides_under_a_higher_z_rect
#   plans/handoffs/HANDOFF_dm-adoption_board-triple-serve_2026-07-03.md (this file)

# Next action
#   Board has Verified flips → move items to Done/archive (game side may have already).
#   Board has EW-007+ → serve priority-order (/add-feature-example or /add-ui-widget), land async.
#   Board unchanged (all Shipped, game court) → NOT empty: report that the pipe is with the game
#   session and ASK (run game session / wait / user direction). Do NOT invent engine work.
```

## Session Closed

**Closed at:** 2026-07-03
**Commit:** landed via this `docs(handoff)` PR (async auto-merge — would be the chain's 8th unattended landing); engine tip at close = `c42a890` #339 v0.116.0. Memory seq **160** belongs to this PR's merge — the next session's deferred wrap-up, exactly as this session did seq 156 for the parent's PR.
**Session status:** Handed off — the demand pipe worked end-to-end for the first time (filed → served → Shipped, same day, including one request filed mid-session); the ball is with the game session to verify all three.
