# rust-survivors v4 migration VERIFIED green + clippy-debt cleanup + visual playtest + doc sync

**Date:** 2026-06-08
**Status:** COMPLETED — `rust-survivors` re-verified green against engine v4.0.0 (fmt/clippy `-D warnings`/200 tests/release build/live app render); stale docs synced.
**Bead(s):** none (`bd` unavailable)
**Epic:** code-analysis remediation (`docs/CODE_ANALYSIS.md`) — closed; this is the downstream-game verification tail.
**Chain:** `code-analysis-fixes` seq `4`
**Parent:** `HANDOFF_code-analysis-fixes_v4-shipped_2026-06-07.md` (seq 3)
**Prior chain:** seq 1 `high-severity-bugs` → seq 2 `v3-breaking-batch` → seq 3 `v4-shipped` → **seq 4 `rust-survivors-v4-verify` (this)**

---

## Since Last Handoff

Seq 3 (`v4-shipped`) closed the engine code-analysis epic (30/30 → v4.0.0) and migrated `rust-survivors` v2→v4 locally (`da11775`), but left it **unpushed and visually unverified**. Its "Where We're Going" named exactly two tail items; this session did both and found residuals:

1. **Push the rust-survivors v4 migration** → being pushed this session (was local-only `da11775`).
2. **Visual playtest of rust-survivors color rendering** → DONE — launched the actual app, screenshotted the in-game render, confirmed the Color newtype migration doesn't break rendering.
3. **New finding (not anticipated by seq 3):** the migration was real but **not fully gate-clean** — `da11775` claimed "200 tests pass" but never ran `cargo fmt --check` or `clippy -D warnings`. Re-verification surfaced a 2-line fmt slip the commit introduced + **32 pre-existing clippy lints** (from the earlier `e2ad91f` gameplay pass, never clippy-checked). Cleaned all of them.
4. The seq-3 open question "push now or let user bundle with WIP?" was answered by the user: **commit and push** (surgically — see Key Decisions).

Trajectory: epic fully closed; this is the last verification tail. No further code-analysis work remains.

## Reference Documents

- `docs/HANDOFF.md` (engine) — per-phase history; the 2026-06-06 entry's stale "rust-survivors still pinned to v2" note was corrected this session.
- `rust-survivors/docs/ENGINE_MIGRATION_NOTES.md` — game-side migration log; new `Applied Status - 2026-06-08` entry added.
- `plans/handoffs/HANDOFF_code-analysis-fixes_v4-shipped_2026-06-07.md` (seq 3) — the migration + engine v4 story; canonical for what da11775 did.
- `CLAUDE.md` (both repos) — conventions. Engine: 5-command gate w/ `+1.88.0` caveat (memory `ci-toolchain-pin`). Game gate: `rust-survivors/CLAUDE.md` lines 17-21.
- Memory: `ci-toolchain-pin`, `playtest-windowed-examples`, `rust-survivors-engine-pin`, `conversation-language-korean` (new this session), `subagent-usage-preference`.

## The Goal

Prove the `rust-survivors` v2→v4 engine migration is actually production-green (not just unit-test green) and bring its repo state up to date, closing the downstream-verification tail of the engine's code-analysis epic. The user wanted it migrated, runnable, and pushed. VISION priority 2 (engine as a usable foundation for a real game) — the migration is only "done" once the game compiles clean, passes its full CI-equivalent gate, AND renders in real play.

## Where We Are

- **rust-survivors HEAD before this session = `da11775` "Migrate to skeleton-engine v4.0.0 (Color newtype)"** (2026-06-07) — the migration was ALREADY committed by seq 3. The task as framed ("do the migration") was already done; this session = verify + clean + push.
- **Engine pin:** `crates/game/Cargo.toml` `engine = { …, rev = "60328fae…" }` = engine **v4.0.0**, code-identical to engine HEAD `77bd2a5` (which only adds 2 session-doc files). `Cargo.lock` = `skeleton-engine 4.0.0`.
- **Migration scope confirmed:** Color newtype (#11) was the ONLY breaking change touching the game. `PhysicsWorld` (#13) inert — game owns a raw `PhysicsWorld` in `src/main.rs` (platformer demo) only; `PhysicsWorld::new(gravity)` unchanged. Joints (#28) unused. No `Entity.0`/`Entity(` tuple usage. No `Events<E>`/`register_persistent`/`Scene::on_enter` exposure.
- **fmt:** `da11775` left two `.to_array()` method-chains unwrapped (over rustfmt `chain_width`). BOTH stable rustfmt 1.9.0 AND pinned 1.8.0 (1.88.0) flag them → genuine, not toolchain drift. Fixed via `cargo fmt` (test code in `lib.rs`, `weapon.rs`). Both toolchains now clean.
- **clippy:** `cargo +1.88.0 clippy --all-targets -- -D warnings` failed with **32 pre-existing style lints**, NONE in the 6 migration-touched files. Auto-fixed 29 (`uninlined_format_args` ×27, `manual_div_ceil` ×2) via `clippy --fix`; manually fixed 3. Now GREEN.
- **Tests:** `cargo +1.88.0 test -p game --lib --locked -- --test-threads=1` = **200 passed, 0 failed** (against engine v4) — held across all the cleanup edits.
- **Release build:** `cargo +1.88.0 build -p game --bin survivor --release --locked` = clean (compiles `skeleton-engine v4.0.0` + `game`).
- **Live render verified:** launched `target/release/survivor`, screenshotted; renders Mad Forest stage (tilemap), player + bat enemies, full HUD (HP/XP/stats/weapon icons). No crash; clean exit. `/tmp/survivor_shot.png`.
- **`if_same_then_else` was a real latent dead-code finding** (`hud.rs:1448`): a 3-way `if compact_hud { if compact_detailed_hud {A} else {A} } else {A}` where all branches were identical. The "compact detailed HUD" stat-line spacing was scaffolded but never differentiated. Collapsed to `let y = top_layout.stats_y + i as f32 * 20.0 * ui_scale;` → cascaded to a now-dead `TopHudLayout.detail` field → removed the field + its `Self{}` init (the `detail` param stays; it still feeds `rows`/`panel_w`/`info_font_size` matches in `new()`).
- **HUD detail feature still works** — `HudDetail::Minimal/Normal/Detailed` match arms at `hud.rs:1284/1302/1344` and the layout matches at `ui_layout.rs:301-321` are untouched. Only the unread stored field was removed.
- **Engine repo** otherwise clean except my `docs/HANDOFF.md` stale-note fix + this handoff file.
- **rust-survivors** has ~20 unrelated pre-existing WIP files (doc cleanup + `powerup.rs` comment + `README.md`/`AGENTS.md`/`CLAUDE.md`) — the user's in-progress work, deliberately NOT bundled.

## What We Tried (Chronological)

1. **Read prior handoff + next-work docs.** `docs/HANDOFF.md`, `NEXT_WORK.md`, `CODE_ANALYSIS.md`, `ROADMAP.md` (historical), `REMAINING_WORK.md` (historical). Engine HANDOFF said rust-survivors "still pinned to v2 rev 61c09f1" — flagged as a candidate next task.
2. **Two parallel Explore (Sonnet) agents** — (a) inventory rust-survivors' engine API usage, (b) map engine v2→v4 breaking surface. Agent (a) returned the **contradiction**: the game's `Cargo.toml` is pinned to `60328fae` (v4), not v2, and already uses `engine::Color`.
3. **Verified the contradiction directly** (not trusting the subagent on a premise-changing fact): `git -C rust-survivors log` → HEAD = `da11775 "Migrate to skeleton-engine v4.0.0"`; `Cargo.lock` = skeleton-engine 4.0.0; commit message confirms Color-only migration + 200 tests. **Conclusion: migration already done; engine HANDOFF note stale by 1 day.** Engine HEAD `77bd2a5` message even says "migrated rust-survivors v2→v4 … local commit da11775".
4. **AskUserQuestion #1** (task already satisfied) → user picked **"Verify green now + fix docs."**
5. **`cargo fmt --check`** → FAIL on 2 `.to_array()` lines. Checked: stable 1.9.0 AND 1.88.0/1.8.0 both want the wrap → not drift, a real fmt slip in `da11775`. `cargo fmt` applied; both toolchains re-checked green. Touched only `lib.rs`, `weapon.rs`.
6. **`cargo +1.88.0 clippy --all-targets -- -D warnings`** → FAIL, 25 lib + 31 lib-test errors (32 distinct). Mapped error files: `hud.rs`(10), `pickup.rs`(5), `meta.rs`(5), `levelup.rs`(4), `data.rs`(2), `lib.rs`(2), + singles — **zero** in the 6 migration-touched files. Conclusion: pre-existing debt from `e2ad91f`, surfaced by current clippy defaults.
7. **Confirmed migration is functionally green independent of clippy** — ran `cargo +1.88.0 test -p game --lib` (200 pass) + `build --release` (clean) with no `-D warnings`. clippy-only lints don't fire under `cargo build`/`test`.
8. **AskUserQuestion #2** (clippy debt scope) → user picked **"Fix clippy debt now."**
9. **`cargo +1.88.0 clippy --fix --all-targets --allow-dirty`** → auto-fixed 29 (uninlined_format_args ×27, manual_div_ceil ×2). 3 remained: `manual_let_else→?` (`levelup.rs:400`), `too_many_arguments 8/7` (`ui_layout.rs:214`), `if_same_then_else` (`hud.rs:1449`).
10. **Manual fixes:** `levelup.rs` let-else → `?` (fn returns Option); `ui_layout.rs` `#[allow(clippy::too_many_arguments)]` on the layout ctor; `hud.rs` collapsed the identical-branch `if` → exposed dead `TopHudLayout.detail` → removed field + init.
11. **Re-verify gate** (`fmt` both toolchains + `clippy -D warnings` + `test`) → fmt/test green but clippy showed 1 NEW `dead_code` (the `detail` field). NOTE: a `tail -5 | && echo` masked the clippy exit the first time — caught it on re-read.
12. **Removed the dead `detail` field** → re-verify → **all green** (clippy `-D warnings`, fmt ×2, 200 tests). Then `build --release` → clean.
13. **Visual playtest** (memory `playtest-windowed-examples`): launched `target/release/survivor` from workspace root (assets are cwd-relative `assets/...`), `caffeinate -dimsu`, 9s render wait, `osascript` front+bounds, `screencapture -x -o /tmp/survivor_shot.png`, then `kill`. App alive after 9s AND at shot time → no crash. Read the PNG → confirmed live game render.
14. **Doc sync:** corrected engine `HANDOFF.md:2153` stale note; appended `Applied Status - 2026-06-08` to game `ENGINE_MIGRATION_NOTES.md`.

## Key Decisions

- **Did NOT redo the migration** — it was already done (`da11775`). Reframed the task as verify+clean+push and confirmed with the user (AskUserQuestion) rather than blindly "migrating" finished work. (Guidance: when reality contradicts the framing, surface it.)
- **Fixed the fmt slip rather than leaving it** — both available rustfmt versions agree on the wrap, so it's unambiguous and behavior-irrelevant (test code). Not the documented "local stable differs from CI" case (that's `ci-toolchain-pin`); here even 1.88.0 wants it.
- **Treated the 32 clippy lints as in-scope only after the user said so** (AskUserQuestion #2). They're pre-existing/unrelated to the migration — could have been deferred, but user chose to clean now.
- **`if_same_then_else`: collapsed + removed dead field, did NOT invent a different compact value.** All 3 branches were identical, so collapsing is behavior-preserving; guessing a different spacing for detailed mode would be an unrequested product change. Flagged to the user that "detailed-mode stat-line spacing" is an unimplemented feature if they want it.
- **`too_many_arguments` → `#[allow]`, not a params-struct refactor.** It's a layout constructor; an attribute is the conventional, minimal, behavior-preserving fix.
- **Surgical commit, mirror seq 3.** Commit ONLY this session's files (12 `.rs` + `ENGINE_MIGRATION_NOTES.md`); leave the ~20 unrelated WIP files (the seq-3 lesson: "don't bundle the WIP"). `powerup.rs` + `ENGINE_MIGRATION_NOTES.md` carry minor user edits that ride along (can't split within a file without interactive add); disclosed. `powerup.rs` MUST be committed — its clippy fix is required for HEAD to be gate-green.
- **Used `+1.88.0` for the game gate too** — matches the documented CI pin (`ci-toolchain-pin`) and what seq 3 verified the engine with; the fixes (`{x}` inlining etc.) satisfy both toolchains anyway. The game has no `rust-toolchain` pin.
- **Conversational language → Korean** (user request mid-session); artifacts stay English (doc-language rule). Saved as memory `conversation-language-korean`.

## Evidence & Data

### Verification gate — final state (rust-survivors @ engine v4.0.0, toolchain 1.88.0)

| Check | First run | Final |
|---|---|---|
| `cargo fmt --check` (stable 1.9.0 + 1.88.0/1.8.0) | ❌ 2 unwrapped `.to_array()` | ✅ both |
| `cargo +1.88.0 clippy --all-targets --locked -- -D warnings` | ❌ 32 lints | ✅ |
| `cargo +1.88.0 test -p game --lib --locked -- --test-threads=1` | — | ✅ 200 passed, 0 failed |
| `cargo +1.88.0 build -p game --bin survivor --release --locked` | — | ✅ clean |
| Live app launch + render (screenshot) | — | ✅ `/tmp/survivor_shot.png` |

### Clippy lints fixed (32 total, all pre-existing, none migration-related)

| Lint | Count | Fix |
|---|---|---|
| `uninlined_format_args` (`"{}",x` → `"{x}"`) | 27 | `clippy --fix` (auto) |
| `manual_div_ceil` | 2 | `clippy --fix` (auto) |
| `manual_let_else` → `?` (`levelup.rs:400`) | 1 | manual: `let x = world.resource::<LevelUpActions>()?;` |
| `too_many_arguments` 8/7 (`ui_layout.rs:214` `MenuListLayout::new`) | 1 | manual: `#[allow(clippy::too_many_arguments)]` |
| `if_same_then_else` (`hud.rs:1449`) | 1 | manual: collapse → drop dead `TopHudLayout.detail` field |

### Clippy error file distribution (from the first failing run)

`hud.rs` 10 · `pickup.rs` 5 · `meta.rs` 5 · `levelup.rs` 4 · `data.rs` 2 · `lib.rs` 2 · `ui_layout.rs` `sfx.rs` `powerup.rs` `debug_input.rs` `chest.rs` 1 each. Migration-touched files (`background/damage/particle/sprites/ui_icons/weapon`) = 0.

### Live render log (`/tmp/survivor.log`)

```
Restarted.
Game started (stage: Mad Forest (1)).
```
ALIVE_AFTER_9s=yes · ALIVE_AT_SHOT=yes · SHOT_EXIT=0 · screenshot 2.6 MB · no lingering process after kill.

### rust-survivors recent commits

| SHA | Date | Commit |
|---|---|---|
| `da11775` | 2026-06-07 | Migrate to skeleton-engine v4.0.0 (Color newtype) — local, UNPUSHED before this session |
| `e2ad91f` | 2026-06-04 | Implement engine v2 gameplay action pass (← source of the 32 clippy lints) |

### Toolchains present

stable (rustfmt 1.9.0), 1.88.0 (rustfmt 1.8.0), 1.79.0. CI pin = 1.88.0.

## Code Analysis (key shapes touched)

- `TopHudLayout` (`ui_layout.rs:278`): removed `pub(crate) detail: HudDetail` field + its `Self{}` init. `new(viewport_w, viewport_h, detail: HudDetail)` keeps the `detail` PARAM (drives `rows`/`panel_w`/`info_font_size` matches at lines 301-321); only the unread stored copy was dead.
- `HudDetail` (`meta.rs:94`) is alive: `Minimal/Normal/Detailed`, `.step(i)`, used in `hud.rs` match arms (1284/1302/1344) + serialized in `MetaSave.hud_detail`.
- `LevelUpSystem::skip_levelup` (`levelup.rs:399`) returns `Option<(u32,u32)>` → `let-else { return None }` cleanly became `?`.
- Asset loading is **cwd-relative** (`assets/textures/survivor/...` consts in `sprites.rs:9+`); the font is `include_bytes!` (embedded). Run the binary from the workspace root or assets fail to load.
- Migration Color idiom (from `da11775`, unchanged): writes `sprite.color = engine::Color::from(arr)` / `Color::rgba(..)` / `Color::WHITE`; reads `color.to_array()`. Explicit ctors avoid the rapier/simba `[T;N]: Into` ambiguity (E0283).

## Files Changed (this session)

### Engine (`skeleton-engine`)
- `docs/HANDOFF.md` — replaced the stale "rust-survivors still pinned to v2 (rev 61c09f1)" bullet (line ~2153) with the true v4-migrated + re-verified state.
- `plans/handoffs/HANDOFF_code-analysis-fixes_rust-survivors-v4-verify_2026-06-08.md` — this file.

### Game (`rust-survivors`) — committed (12 `.rs` + 1 doc)
- `crates/game/src/lib.rs`, `crates/game/src/survivor/weapon.rs` — `cargo fmt` wrap of the migration's `.to_array()` lines (test code).
- `crates/game/src/survivor/{chest,data,debug_input,meta,pickup,sfx}.rs` — `uninlined_format_args` auto-fixes.
- `crates/game/src/survivor/powerup.rs` — `uninlined_format_args` auto-fix (rides along with the user's pre-existing comment edit).
- `crates/game/src/survivor/levelup.rs` — let-else → `?` + format-arg auto-fixes.
- `crates/game/src/survivor/hud.rs` — `manual_div_ceil` + format-arg auto-fixes + collapsed the identical-branch `if`.
- `crates/game/src/survivor/ui_layout.rs` — `#[allow(too_many_arguments)]` + removed dead `TopHudLayout.detail` field/init.
- `docs/ENGINE_MIGRATION_NOTES.md` — appended `Applied Status - 2026-06-08` (carries the user's pending edit to this file too).

### Game — NOT committed (pre-existing user WIP, ~20 files)
`README.md`, `AGENTS.md`, `CLAUDE.md`, and docs (`NEXT_WORK_PLAN.md`, `PHASE_LOG.md`, `release_checklist.md`, several `*_PROMPT.md` deletions, etc.). Untouched by me.

### Memory (`~/.claude/.../memory/`)
- `conversation-language-korean.md` (new) + `MEMORY.md` pointer.

## User Feedback & Preferences

- **"rust-survivors 마이그레이션 진행해줘"** — asked to do the migration (which turned out already done).
- **"다음번부터 설명은 한글로"** — explanations in Korean from now on (saved to memory `conversation-language-korean`). Artifacts stay English (doc-language rule).
- AskUserQuestion answers: **"Verify green now + fix docs"** (not just docs, not full-idiom refactor); **"Fix clippy debt now"** (not defer, not blanket `#[allow]`).
- **"실행이 잘 되는지 확인할겸 게임 실행시켜줘"** — wanted the actual app run, not just tests (→ playtest + screenshot).
- **"커밋 하고 푸쉬"** (via `/handoff` arg) — commit AND push. Standing prefs (memory): use subagents for parallel work (Sonnet) — used 2 Explore agents; verify before declaring done — re-ran the real gate after every edit batch.

## Gotchas & Lessons (reusable, cost real time)

- **Verify a premise-changing subagent claim before acting.** The Explore agent reported "Cargo.toml pinned to v4, already uses `engine::Color`" — contradicting the engine HANDOFF's "still on v2." Confirmed directly via `git log`/`Cargo.lock` before reframing the whole task. The whole job changed from "migrate" to "verify + clean."
- **A `cmd 2>&1 | tail -N && echo OK` chain masks `cmd`'s exit code** — the pipe's status is `tail`'s (0), so `&& echo OK` runs even when `cmd` failed. Printed a false "CLIPPY_GREEN" once. Use `${PIPESTATUS[0]}` or run the gate command bare (no pipe) for the `&&` chain.
- **fmt slip vs CI-drift are different.** If only stable rustfmt flags it → CI-drift, re-check with `+1.88.0` (memory `ci-toolchain-pin`). If BOTH stable 1.9.0 AND 1.88.0/1.8.0 flag it → a genuine unformatted commit; just `cargo fmt`. Here it was the latter (`da11775` never ran fmt).
- **`clippy --fix` only auto-applies machine-applicable lints.** `if_same_then_else`, `too_many_arguments`, `manual_let_else` (sometimes), and `dead_code` are left for manual handling. Plan to hand-fix the residue after `--fix`.
- **Collapsing identical `if` branches can cascade to dead code.** Removing the only reader of a field/var turns it dead → a NEW `dead_code` error on the next gate run. Expect a 2nd pass after such a collapse.
- **Game binary needs cwd = workspace root.** Asset paths are relative (`assets/...`); the font is `include_bytes!` (embedded) but textures/audio load at runtime. Launch from `/Users/jkl/Projects/rust-survivors`, not from `target/release/`.
- **macOS windowed playtest recipe** (memory `playtest-windowed-examples`): launch in bg → `caffeinate -dimsu` → ~9s render wait → `osascript` front+bounds (tolerate missing a11y perms with `2>/dev/null`) → `screencapture -x -o` → `kill`. Assert the PID is alive at shot time to distinguish "rendered" from "crashed-then-blank."

## Where We're Going

The code-analysis epic and its downstream-verification tail are **closed**. After this session's push, nothing in this chain remains. Possible next initiatives (out of chain scope, none scheduled):

1. **(If desired) commit/organize the ~20 rust-survivors WIP doc files** — left for the user; they predate this session.
2. **`networking` dogfooding cycle** — the last engine subsystem with no playable-game coverage (`NEXT_WORK.md:220`); would need a multiplayer playable example.
3. **(Optional) repo-wide example Korean→English** comment conversion (engine `docs/english-conversion` branch) — only the 5 #27 examples done.
4. **(Optional) git-tag engine `v4.0.0`** — currently version is in `Cargo.toml` only; consumers pin by rev.

## Risks & Blockers

- **rust-survivors push includes `da11775`** (the migration, previously unpushed) AND this session's cleanup commit — both go up on first push. Expected/desired.
- **The ~20 WIP files stay dirty** after the surgical commit — intentional; the user owns them. Next session will see a dirty game tree.
- **`powerup.rs` + `ENGINE_MIGRATION_NOTES.md` bundle a little user WIP** (couldn't split within-file). Minor, disclosed.
- **`--config` path-patch trick still rejected** with the v4 pin only if a v2 pin is used — N/A now (pinned v4). (memory `rust-survivors-engine-pin`.)
- Engine `main` is the default branch; session/doc commits go there directly per established repo convention (every prior `session:` commit did).

## Open Questions

- Tag/release engine `v4.0.0` as a git tag? (seq-3 open Q, still open; not blocking.)
- Should the ~20 rust-survivors WIP doc files be committed, or are they intentionally long-lived scratch? (User's call.)

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine

# Restore context
cat plans/handoffs/HANDOFF_code-analysis-fixes_rust-survivors-v4-verify_2026-06-08.md   # this file
cat plans/handoffs/HANDOFF_code-analysis-fixes_v4-shipped_2026-06-07.md                 # parent (migration story)

git log --oneline -5            # engine: latest = this session's session: commit, head v4.0.0
git -C /Users/jkl/Projects/rust-survivors log --oneline -4   # game: cleanup commit + da11775 (now pushed)
git -C /Users/jkl/Projects/rust-survivors status -s          # ~20 unrelated WIP files still dirty (user's)

# Verify game state (recompiles engine from git first time — slow)
cd /Users/jkl/Projects/rust-survivors
cargo +1.88.0 fmt --check && cargo +1.88.0 clippy --all-targets --locked -- -D warnings && \
  cargo +1.88.0 test -p game --lib --locked -- --test-threads=1   # 200 pass

# Re-run the live playtest (memory: playtest-windowed-examples)
#   ./target/release/survivor   (from workspace root — assets are cwd-relative)

# Next action: nothing in THIS chain. Pick a new initiative (see "Where We're Going"):
#   networking dogfooding example, OR engine v4.0.0 git tag, OR help user organize the WIP docs.
```

## Session Closed
**Closed at:** 2026-06-08 00:59 KST
**Commits:** rust-survivors `6f044b4` (cleanup, pushed with migration `da11775` → `origin/main`); skeleton-engine session commit (this handoff + `docs/HANDOFF.md` note fix, pushed to `origin/main`).
**Session status:** Handed off to next session. Chain `code-analysis-fixes` seq 4 — epic + downstream tail closed.
