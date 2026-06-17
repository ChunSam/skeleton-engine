# DialogueBox depth shipped — localization + branching choices (v0.19.0), CI disk-space fix

**Date:** 2026-06-17
**Status:** **COMPLETED — shipped + merged, CI-green.** `main` @ `9f1ff10`, package **v0.19.0**, clean tree.
PR **#106** squash-merged (merge authority re-confirmed this session). 789 → **801 lib tests**.
**Bead(s):** none (beads unavailable in this environment)
**Epic:** Narrative dialogue — make the Phase-4 `DialogueBox` production-grade
**Chain:** `engine-hardening` seq `21` · **Parent:** `HANDOFF_engine-hardening_gpu-ron-particle-depth_2026-06-17.md` (seq 20)
**Prior chain:** … → 18 (P6) → 19 (P7, UX roadmap 7/7) → 20 (GPU/RON particle depth) > **21 (this: DialogueBox depth)**

---

## Since Last Handoff

The seq-20 handoff was a feature-complete close that **recommended** "DialogueBox localization + branching choices" as the next epic and shipped the paired `PLAN_engine-hardening_gpu-ron-particle-depth_2026-06-17.md`. This session executed that plan, all three phases, exactly as scoped:

- **Parent's "Where We're Going" → reality:** the recommended next epic was DialogueBox depth. Done in full (Phases 1–3), shipped as a single `0.19.0` MINOR (the plan offered "batch as one 0.19.0 or one bump per phase" — I batched, since the example is the acceptance test that proves Phases 1–2).
- **Parent open question "does `LocaleResource::t` return the key on a miss?" → ANSWERED:** yes. `src/locale.rs:118` `t()` falls back current-locale → default-locale → the key itself (`unwrap_or(key)`). Confirmed by reading + the existing `missing_key_falls_back_to_default_then_key` test. So a missing dialogue key degrades visibly, never panics. Phase 1 relied on this safely.
- **Parent risk "borrow conflict resolving locale during `query_mut`" → materialized, handled:** `resolve()` needs both `&mut DialogueBox` (from `query_mut`) and `&LocaleResource`. Solved by `world.resource::<LocaleResource>().cloned()` before the loop (clone the resource to release the world borrow), per the plan's "gather then mutate" guidance.
- **Parent risk "API awkwardness surfaces in Phase 3" → did NOT materialize.** Writing the example, `localized()` / `with_choices()` / `pending_choices()` / `choose(i)` all read cleanly; no API change was needed mid-example. VISION core-loop check passed on the first try.
- **New, unplanned detour:** CI's Test (native) job hit a GitHub-runner disk-exhaustion failure (`No space left on device`), twice. Not a code defect — fixed with a `Free disk space` step in `ci.yml` (user-approved). This is the one thing that wasn't in the plan.

## Reference Documents

- `CLAUDE.md` — project conventions, module map (the dialogue row was updated this session).
- `docs/PATTERNS.md` — borrow-workaround / gather-then-mutate, system ordering.
- `plans/handoffs/PLAN_engine-hardening_gpu-ron-particle-depth_2026-06-17.md` — the plan this session executed (Phases 1–3 spec, anti-goals, risks).

## The Goal

Make the Phase-4 `DialogueBox` (`src/dialogue.rs`, a linear typewriter over `Vec<String>`) production-grade for RPG / visual-novel games, on the two axes a real narrative game needs: **localization** (drive lines/speaker/choices from the existing `LocaleResource`/`t(key)` i18n the UI already uses, so a game ships in multiple languages) and **branching** (player choices that change the conversation path). Both must be purely additive — a box with no keys and no choices renders byte-identically to before, and old scene RON still loads. Per the engine's VISION rule, the feature isn't done until a small playable example exercises it in real play; that example is the acceptance test and surfaces API awkwardness before release.

## Where We Are

- **Shipped + merged.** `main` @ `9f1ff10`, package `v0.19.0`, CLAUDE.md header `v1.6.67`, clean tree, all CI green. PR #106 squash-merged + branch deleted + pruned.
- **Phase 1 (localization) — `src/dialogue.rs`:** added `pub line_keys: Vec<String>` + `pub speaker_key: Option<String>` to `DialogueBox` (both `#[serde(default)]`); constructor `DialogueBox::localized(speaker_key, line_keys)`; method `resolve(&mut self, locale: &LocaleResource)` fills `lines` from `line_keys` and `speaker` from `speaker_key`, and **does not touch `current`/`elapsed`/`full`** (safe every frame).
- **Phase 1 system wiring:** `DialogueSystem::run` step 0 — `if let Some(locale) = world.resource::<LocaleResource>().cloned() { for (_e, d) in world.query_mut::<DialogueBox>() { d.resolve(&locale); } }`, before the tick loop. No-op when no `LocaleResource` (literal-dialogue games never clone).
- **Phase 2 (branching) — `src/dialogue.rs`:** new `pub struct DialogueChoice { pub text: String, pub key: Option<String>, pub goto: usize }` with `new(text, goto)` + `localized(key, goto)` ctors; `pub choices: Vec<(usize, Vec<DialogueChoice>)>` field (`#[serde(default)]`); `with_choices(line, choices)` builder; `pending_choices() -> Option<&[DialogueChoice]>`; `choose(i)`.
- **Phase 2 semantics:** `pending_choices()` is `Some` only when `!is_finished() && line_fully_revealed()` and the current line has a non-empty choice list. `advance()` no-ops while `pending_choices().is_some()` (a plain advance can't skip a decision; the first advance still completes the reveal, *then* choices appear). `choose(i)` sets `current = goto.min(lines.len())` (out-of-range goto clamps to end = finishes, no panic), resets `elapsed`/`full`; no-op if no choices pending or `i` out of range.
- **Phase 2 resolve extension:** `resolve()` also localizes choice labels — `for (_line, choices) in &mut self.choices { for c in choices { if let Some(k) = &c.key { c.text = locale.t(k).to_string(); } } }` — independent of `line_keys` (a literal-line box can have localized choices and vice versa).
- **Phase 2 render:** `DialogueSystem` gather now collects `Vec<(String, String, bool, Vec<String>)>` (added the choice-label vec); render draws a numbered list (`"1. <label>"`, x0+16, stepping `vh-86` by +26) **in place of** the `▼ space` hint when choices are pending.
- **Export:** `src/lib.rs:91` now `pub use dialogue::{DialogueBox, DialogueChoice, DialogueSystem};`.
- **Phase 3 (example) — `examples/dialogue_branching.rs` (NEW, 181 lines):** a localized merchant scene. `SPACE` advance, `1`/`2` choose, `L` live en↔ko toggle, `R` replay, `ESC` quit. In-code `LocaleResource::from_ron_str(LOCALES_RON)` with en+ko bundles. 5-line conversation (0 intro → 1 ask → {2 buy | 3 dark} → 4 farewell); line 1 branches, lines 2 & 3 each reconverge on line 4 via a "go" choice (so each branch shows only its own payoff before a shared farewell).
- **Tests:** +12 new unit tests in `src/dialogue.rs` (4 Phase-1 localization + 8 Phase-2 branching), all green. Lib total 789 → 801. Doctests 64 → 65 (the `localized()` ctor doc example).
- **CI fix — `.github/workflows/ci.yml`:** added a `Free disk space` step to the **Test (native)** job (`sudo rm -rf` android/dotnet/ghc/CodeQL/boost/swift ≈ +25 GB) after checkout, before the cache restore + builds.
- **Release paperwork (4-file `/ship` pass):** `Cargo.toml` 0.18.0→0.19.0; `Cargo.lock` via `cargo update -p skeleton-engine`; `docs/CHANGELOG.md` new `## 0.19.0` (Added); `CLAUDE.md` header `v1.6.66→v1.6.67` + package `v0.18.0→v0.19.0` + the dialogue module-map row rewritten.
- **Playtested windowed** (4 screencaptures, all 4 acceptance goals confirmed — see Evidence).

## What We Tried (Chronological)

1. **Read plan + parent handoff + 4 Quick-Start source files** (`src/dialogue.rs`, `src/locale.rs`, `src/ui/localized.rs`, `examples/dialogue_demo.rs`). Confirmed the resolve-each-frame pattern from `LocalizationSystem` (collect entity+key → read locale → apply via get_mut) and the input-agnostic `DialogueSystem`. Ran `./scripts/verify.sh` → green baseline (789 lib tests). Created 3 phase tasks (1→2→3 blocked-by chain).
2. **Phase 1 field + ctor + resolve.** Added `line_keys`/`speaker_key`, `localized()`, `resolve()`. **Decision on the system wiring:** rather than duplicate resolve logic inline (the `LocalizationSystem` collect-then-apply shape), I `cloned()` the `LocaleResource` and called `resolve()` directly in `query_mut` — faithful to the plan ("call resolve on each box, collect the resource read first"), reuses one code path. Per-frame clone only happens when a `LocaleResource` exists. `cargo test --lib dialogue` → 9 pass (5 old + 4 new). `cargo clippy --lib` clean.
3. **Phase 2 struct + field + methods.** Added `DialogueChoice`, `choices`, `with_choices`, `pending_choices`, `choose`, the `advance()` guard, and the render branch. **Borrow note:** `choose()` extracts `goto` (a `usize`, Copy) out of the `pending_choices()` borrow before mutating `self` — compiles cleanly. **Resolve note:** the choice loop borrows `&choice.key` and assigns `choice.text` (disjoint fields → NLL allows it). `cargo test --lib dialogue` → 17 pass. `cargo clippy --lib` clean.
4. **Phase 3 example.** Confirmed `KeyCode` is winit's (`src/lib.rs:58` re-export) → `Digit1`/`Digit2`/`KeyL`/`KeyR` exist; mirrored `settings_menu`'s `LocaleResource::from_ron_str` insert pattern. Wrote `dialogue_branching.rs`. Designed branches to **reconverge** (the only bleed-free layout with pure `+1` advance + forward `goto`; each branch ends in a single "go" choice → line 4). `cargo build --example dialogue_branching` → ok.
5. **Windowed playtest.** Launched `target/debug/examples/dialogue_branching` under `caffeinate -dimsu`, positioned the window via `osascript … set position of front window to {100,100}`, captured frames with `screencapture -x -R100,100,960,640`. **Discovery: synthetic KEYSTROKES via `osascript … key code N` DO reach a winit window** (intro→Space→choice line→`1`→buy payoff→`L`→Korean), unlike the seq-20 finding that `System Events "click at"` does NOT reach winit. All 4 goals confirmed. `ESC` (key 53) quit; `pkill` cleanup.
6. **Full `verify.sh` (run 1) → FAILED at `cargo fmt --check`.** rustfmt wanted different line-wrapping (a split string literal on one line; an `assert_eq!` expanded). Pure cosmetic. Ran `cargo fmt`, re-ran `verify.sh` → green (801 lib tests, all gates).
7. **`/ship` paperwork** (4 files) + `cargo update -p skeleton-engine` + re-ran `verify.sh` post-bump → green. Confirmed merge authority via AskUserQuestion → "Branch, PR, squash-merge on green CI".
8. **Commit, push, open PR #106.** CI watch (run 27695312677) → **Test (native) FAILED at 1m37s.** Investigated: `gh run view --job … --log` showed NO test/assertion/compile failure; the annotation was `System.IO.IOException: No space left on device` — a GitHub *runner process* crash on disk I/O.
9. **Re-ran the failed job** (`gh run rerun … --failed`). **Failed AGAIN at 2m2s, identical disk error.** `--log-failed` was empty (runner died before capturing). Confirmed it's persistent infra, not luck. Read `ci.yml`: the `test` job builds `--all-targets` (debug) + `--release` into one cached `target/`; a Cargo.lock change (version bump) makes the exact cache key miss → restore-key fallback stacks a stale `target/` → ~14 GB root disk fills.
10. **Asked the user** (3 options: fix in PR / re-run / hold) → "Add disk-cleanup step in this PR". Added `Free disk space` to the `test` job only (validated YAML with `python3 -c yaml.safe_load`), committed + pushed.
11. **CI run 27695976672 → ALL GREEN.** Test (native) 4m42s (healthy, cf. #104's 4m5s), the other 3 jobs pass. `gh pr merge 106 --squash --delete-branch`; synced + pruned main → `9f1ff10`.
12. **Updated memory** (`engine-current-state`, `MEMORY.md` index) to v0.19.0 + the two new gotchas.

## Key Decisions

- **Batched Phases 1–3 as one `0.19.0` PR** (the plan allowed per-phase or batched). Rationale: the VISION rule says the feature isn't done without the example, so shipping Phase 1+2 without Phase 3 would be incomplete; one coherent feature, one MINOR bump.
- **`resolve()` clones the `LocaleResource` per frame** instead of the collect-then-apply shape `LocalizationSystem` uses. Reuses the single `resolve()` code path and is faithful to the plan's intent; cost is one HashMap clone/frame and only when a locale exists (literal games never pay it). Rejected: duplicating the resolve body inline (DRY loss) and a borrow dance with two query passes (more code, same cost).
- **Branches reconverge via a single "go" choice** rather than each branch being terminal. With pure `+1` advance and forward `goto` indices, a branch payoff line would otherwise bleed into the next branch's lines; reconvergence is the bleed-free layout AND it demonstrates that `choose`/`goto` can merge paths, which is a good thing to show in a branching demo.
- **`choices` keyed by `Vec<(usize, Vec<DialogueChoice>)>`** (a list of (line, choices) pairs), per the plan, not a `HashMap<usize, …>`. Keeps it serde-trivial (tuples), order-stable, and tiny; `pending_choices` does a linear `find` over a handful of entries.
- **Out-of-range `goto` clamps to `lines.len()`** (finishes) rather than panicking — robustness for hand-authored or data-driven conversations. Out-of-range `choose(i)` index is a silent no-op (the game can spam number keys safely).
- **CI disk fix in the feature PR, native job only** (not a separate PR, not all jobs). It's a same-session blocker for this merge; the lighter wasm/docs/package jobs passed, so scoping the cleanup to the one heavy job keeps the change minimal.
- **Asked the user before two outward/repo steps** (merge authority; the CI infra change). Merge authority is per-session; modifying `ci.yml` is a repo-infra decision beyond the plan — both are the user's call, consistent with the standing "re-confirm merge authority" rule.

## Evidence & Data

**Merged diff (`9f1ff10`, PR #106): 8 files, +642 / −8.**

| File | Δ | What |
|---|---|---|
| `src/dialogue.rs` | +422/−? | fields, `localized`, `resolve`, `DialogueChoice`, `choices`, `with_choices`, `pending_choices`, `choose`, advance guard, system resolve+render, 12 tests |
| `examples/dialogue_branching.rs` | +181 (new) | localized branching merchant scene |
| `docs/CHANGELOG.md` | +28 | `## 0.19.0` Added entry |
| `.github/workflows/ci.yml` | +9 | `Free disk space` step on Test (native) |
| `CLAUDE.md` | +4/−? | header v1.6.67 + package v0.19.0 + dialogue module-map row |
| `Cargo.toml` / `Cargo.lock` | +2/+2 | 0.18.0 → 0.19.0 |
| `src/lib.rs` | +1/−1 | export `DialogueChoice` |

**Verify gate (`./scripts/verify.sh`, run post version-bump, EXIT=0):** fmt --check · clippy --all-targets -D warnings · wasm build (lib+bins) · wasm clippy --lib -D warnings · `cargo test --all-targets` (**801 lib** + smaller suites) · `cargo test --doc` (**65 pass / 33 ignored**) · `RUSTDOCFLAGS=-D warnings cargo doc`. `[verify] all checks passed ✓`.

**CI runs (PR #106):**

| Run | Test (native) | Build (WASM) | Rustdoc | Package | Result |
|---|---|---|---|---|---|
| 27695312677 (initial) | **fail 1m37s** (disk) | pass 46s | pass 47s | pass 1m23s | RED — runner OOD |
| 27695312677 (rerun --failed) | **fail 2m2s** (disk) | pass | pass | pass | RED — same |
| 27695976672 (after disk fix) | **pass 4m42s** | pass 39s | pass 45s | pass 54s | **GREEN → merged** |

**CI failure root cause (verbatim annotation):** `System.IO.IOException: No space left on device : '/home/runner/actions-runner/cached/2.335.1/_diag/Worker_….log'` — thrown in `GitHub.Runner.Worker.Program`, NOT in cargo. No test/assertion/compile error in the logs; `--log-failed` empty. The Test (native) job uniquely runs `cargo clippy --all-targets` + `cargo test --all-targets` + `cargo test --doc` + `cargo build --release`, all into one cached `target/`.

**Playtest captures (`/tmp/dlg_*.png`, may be cleared — re-run `cargo run --example dialogue_branching` to regenerate):**

| File | Frame | Confirms |
|---|---|---|
| `dlg_0_intro.png` | intro line | localized EN ("Old Merchant" / "Welcome, traveler…"), HUD `[en]`, ▼ space hint |
| `dlg_1_choices.png` | line 1 (ask) | numbered choice list "1. Buy the lantern (5 coin)" / "2. Travel in the dark", ▼ hint suppressed |
| `dlg_2_buy.png` | line 2 (after `1`) | buy payoff shown, dark line skipped (visible branch) + "1. Step into the night" |
| `dlg_3_korean.png` | after `L` | speaker 노상인, body + choice fully Korean, HUD `[ko]`, **same buy-payoff line — position preserved** |

**macOS playtest keycodes used:** Space=49, '1'=18, 'L'=37 (KeyL), ESC=53. Window 960×600 placed at screen {100,100}; capture region `-R100,100,960,640` (includes title bar).

**Locale fixture in the example (`LOCALES_RON`):** keys `npc.merchant`, `dlg.{intro,ask,buy,dark,bye}`, `choice.{buy,dark,go}` in `en` + `ko`.

**Test inventory — `src/dialogue.rs#tests` (17 total: 5 pre-existing + 12 new):**

| Phase | Test | Asserts |
|---|---|---|
| (old) | typewriter_reveals_over_time, instant_when_cps_zero, advance_completes_then_moves_to_next_line, finishes_after_last_line, utf8_safe_reveal | unchanged linear behavior |
| P1 | localized_box_resolves_then_reresolves_on_locale_switch | en resolve → set_locale("ko") → re-resolve; lines/speaker empty pre-resolve |
| P1 | resolve_is_noop_for_literal_box | no `line_keys` → speaker/lines untouched |
| P1 | resolve_preserves_typewriter_progress | tick 0.3 → "Wel", re-resolve → still "Wel", `current`==0 |
| P1 | serde_roundtrip_and_legacy_back_compat | localized box round-trips keys; legacy RON (with `elapsed`/`full`, no new fields) loads |
| P2 | advance_is_blocked_while_choices_pending_then_choose_branches | advance no-op at `current`==0; `choose(0)` → 1 |
| P2 | choose_lands_on_distinct_targets | `choose(0)`→1, `choose(1)`→2 |
| P2 | out_of_range_goto_finishes_safely | goto 99 on 1-line box → finished, `current`==1 |
| P2 | out_of_range_choice_index_is_noop | `choose(5)` on 1-choice line → no-op |
| P2 | first_advance_completes_reveal_then_choices_appear | mid-reveal `pending_choices()`==None; after advance == Some |
| P2 | empty_choice_list_for_a_line_is_not_pending | `with_choices(0, [])` → no pending, advances normally |
| P2 | localized_choices_resolve_text | choice `text` resolves en→ko via `resolve()` |
| P2 | serde_roundtrip_with_choices | `choices` + `DialogueChoice` round-trip |

**Example conversation goto-map (`merchant_box()`):**
```
line 0  dlg.intro    (linear → space → 1)
line 1  dlg.ask      choices: [buy→2, dark→3]
line 2  dlg.buy      choices: [go→4]      (buy branch)
line 3  dlg.dark     choices: [go→4]      (dark branch)
line 4  dlg.bye      (linear → space → finish)
```

**The two load-bearing method bodies (primary evidence — exact semantics):**
```rust
pub fn resolve(&mut self, locale: &LocaleResource) {
    for (_line, choices) in &mut self.choices {        // choice labels resolve independent of line_keys
        for choice in choices {
            if let Some(key) = &choice.key { choice.text = locale.t(key).to_string(); }
        }
    }
    if self.line_keys.is_empty() { return; }           // literal mode → no-op
    self.lines = self.line_keys.iter().map(|k| locale.t(k).to_string()).collect();
    if let Some(key) = &self.speaker_key { self.speaker = locale.t(key).to_string(); }
}

pub fn choose(&mut self, i: usize) {
    let goto = match self.pending_choices().and_then(|cs| cs.get(i)) {
        Some(choice) => choice.goto,                   // usize (Copy) — borrow ends here
        None => return,                                // no choices pending / index OOB → no-op
    };
    self.current = goto.min(self.lines.len());         // OOB goto clamps to end (finishes)
    self.elapsed = 0.0;
    self.full = false;
}
```

## Code Analysis

- **`DialogueBox` field order (serde):** `speaker, lines, current, chars_per_sec, line_keys(#[default]), speaker_key(#[default]), choices(#[default]), elapsed, full, portrait(#[skip])`. `elapsed`/`full` are **private but serialized** (no default) — so a *pre-localization* scene RON (which already contained `elapsed`/`full`) loads fine via the 3 new `#[serde(default)]` fields; a hand-written legacy RON must still include `elapsed:0.0,full:false` (the back-compat test does).
- **`LocaleResource::t` (`src/locale.rs:118`)** — `&self, key: &str -> &str`; current-locale → default-locale → `unwrap_or(key)`. Never panics on a missing key; returns the key. `LocaleResource` derives `Clone` (HashMap-backed) — cheap enough to clone per frame for the resolve step.
- **`pending_choices`/`choose` borrow:** `choose` does `match self.pending_choices().and_then(|cs| cs.get(i)) { Some(c) => c.goto, None => return }` — `goto` is `usize` (Copy), so the immutable borrow ends before `self.current = …`. Compiles without a clone.
- **`DialogueSystem::run` order:** step 0 resolve (clone locale + `query_mut`), step 1 tick (`query_mut`), step 2 gather (`query` → `Vec<(speaker, body, full, choice_labels)>`, releases borrow), step 3 render via `TextQueue`. Choice labels are pre-resolved by step 0, so the gather just clones `c.text`.
- **System registration order in the example:** `DialogueSystem` added first, then `DialogueDriver` (input). 1-frame input→render lag, imperceptible; matches `dialogue_demo`'s convention.
- **`DialogueChoice` derives `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`** — `Eq` is fine (String/Option<String>/usize) and is used by `assert_eq!` in tests.

## Files Changed

### Source code
- `src/dialogue.rs` — Phase 1 (localization: `line_keys`/`speaker_key`/`localized()`/`resolve()` + system resolve-step), Phase 2 (`DialogueChoice` + `choices`/`with_choices`/`pending_choices`/`choose` + advance guard + numbered-choice render), +12 unit tests.
- `src/lib.rs` — re-export `DialogueChoice` (line 91).

### Examples
- `examples/dialogue_branching.rs` (NEW) — localized branching merchant scene; the VISION acceptance test.

### Tests
- 12 tests inside `src/dialogue.rs#tests` (localization: resolve/re-resolve, literal-noop, progress-preserved, serde round-trip + legacy back-compat; branching: advance-blocked-then-choose, distinct targets, out-of-range goto, out-of-range index, reveal-then-choices, empty-list, localized choices, serde with choices).

### Config / CI
- `.github/workflows/ci.yml` — `Free disk space` step on the Test (native) job.

### Release paperwork
- `Cargo.toml`, `Cargo.lock` (0.19.0), `docs/CHANGELOG.md` (`## 0.19.0`), `CLAUDE.md` (header + module-map row).

## User Feedback & Preferences

- **"Execute the plan starting at Phase 1… Do NOT onboard or ask questions — the plan has everything. Build."** → autonomous execution mode; don't ask about the plan/approach, just ship.
- **"ship per the pre-1.0 cadence and re-confirm merge authority before merging."** → the one thing to confirm is merge authority (asked, answered: branch + PR + squash-merge on green CI).
- **Merge authority Q → "Branch, PR, squash-merge on green CI"** — same flow as recent sessions.
- **CI disk Q → "Add disk-cleanup step in this PR"** — user opted to fix the infra in-PR rather than gamble on re-runs or hold.
- **Standing prefs (from memory, still in force):** use subagents aggressively on the Sonnet model for parallel work; never `| tail` a gate/CI-watch (masks exit); `cargo build --example` before a playtest; pre-1.0 0.x cadence (MINOR = any release, do NOT revert version upward); crates.io stays UNPUBLISHED until explicit go.
- **Conversation-language memory says Korean prose**, but the project `CLAUDE.md`'s explicit "# Language: Always respond in English" directive governed this session (responded in English). *Unresolved tension — see Open Questions.*

## Where We're Going

The seq-20 epic is now closed. Open follow-ups, none blocking (pick per next `/goal` or user direction):

1. **GpuParticleEmitter via the RON path** — `ParticleConfigSet::emitter()` builds a CPU `ParticleEmitter` only; there's no RON→`GpuParticleEmitter` builder. The two new RON fields (gravity/emit_shape) feed CPU emitters only. (Carried from seq 20.)
2. **Dialogue, further depth (optional next epic):** a data-driven `.dlg` / RON dialogue-tree loader (the plan explicitly deferred this — branching is in-code only this epic); conditional choices (gated on a flag/var); portraits per line; choice→event hooks.
3. **wasm AEAD `save`/`load`** (currently `Unsupported`); fuller wasm audio (kira).
4. **crates.io publish** (deferred, fork-first, irreversible — explicit go needed); optionally tag `v0.11.1`–`v0.19.0`.

## Risks & Blockers

- **CI disk headroom is now thin but handled.** The `Free disk space` step frees ~25 GB; Test (native) passed at 4m42s. If a future PR adds more heavy example crates, the native `target/` could creep up again — watch for a recurrence of `No space left on device` and extend the cleanup (or split debug/release builds) if it returns.
- **No other blockers.** Tree clean, main green, feature fully merged + verified.

## Open Questions

- **Conversation language conflict:** the `conversation-language-korean` memory says write prose to the user in Korean, but `CLAUDE.md`'s "# Language: Always respond in English" overrode it this session. The `/handoff` skill itself mandates Korean user-facing prose. Worth a one-time reconciliation with the user — which wins for normal (non-skill) turns? (This handoff's user-facing prose is Korean per the skill; the file is English per the doc-language rule.)

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 9f1ff10 feat(dialogue) … (#106)
grep -m1 '^version' Cargo.toml  # 0.19.0

# Prior context (if OV available)
# /memory-recall dialogue localization branching

# Read first (this session's surface)
#   src/dialogue.rs                  — DialogueBox + DialogueChoice + DialogueSystem (localization + branching)
#   examples/dialogue_branching.rs   — the acceptance-test example (localized, branching, live en↔ko)
#   src/locale.rs                    — LocaleResource::t / set_locale (the i18n backing)
#   plans/handoffs/HANDOFF_engine-hardening_dialogue-depth_2026-06-17.md  — this handoff
#   plans/handoffs/HANDOFF_engine-hardening_gpu-ron-particle-depth_2026-06-17.md  — parent (seq 20)

# Verify current state (RUN AS-IS, no `| tail` — it masks the exit code)
./scripts/verify.sh             # green: 801 lib tests, 65 doctests

# Re-see the feature in play
cargo run --example dialogue_branching   # SPACE / 1·2 / L (locale) / R / ESC

# Next action
#   This session is DONE + merged. No required follow-on. If continuing the dialogue epic,
#   the obvious next step is a data-driven RON dialogue-tree loader (deferred this epic);
#   otherwise pick a follow-up from "Where We're Going" or await a new /goal.
```
