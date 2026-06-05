# Next dogfooding cycle: the last never-in-a-game subsystem (networking) — or a chosen non-dogfood direction

**Date:** 2026-06-05
**Status:** PLANNED
**Bead(s):** none (`bd` not installed)
**Epic:** VISION feature+example loop — dogfood shipped-but-never-in-a-game subsystems
**Chain:** `timeline-cutscene` seq `1` (paired with this session's handoff)
**Context:** See `HANDOFF_timeline-cutscene_camera-drive_2026-06-05.md` for the completed Timeline session
(data, decisions, the proven loop). This plan defines the **next** cycle.

---

## Problem Statement

The Timeline/cutscene work is done, committed (`8049dfa`), pushed, and CI-green; that closes candidate
**L**. After five dogfooding cycles (H lighting, I blend, J joints, K render-target, L timeline), the
"never-in-a-game" list in `docs/NEXT_WORK.md` is down to **one** subsystem: **networking** (`mp_client`/
`mp_server` exist but there is no small *playable* multiplayer toy). The next session either (a) dogfoods
networking with a minimal playable example — the same loop that closed L — or (b) takes a **non-dogfood
direction** (a deferred follow-up, an editor/tooling pass, or a polish/maintenance item), because
networking is the least self-contained, highest-infra candidate and the prior plans all deprioritised it.
**Candidate choice is the user's call.**

## Key Findings (from this session + the standing candidate list)

- **Networking is the last never-in-a-game candidate, and the hardest.** `mp_client`/`mp_server` are
  native-only examples (tungstenite); a *playable* toy needs two windowed clients + a shared room — more
  infra than any prior candidate (which were all single-window, cross-platform, no-art). → drives the
  Phase-1 recommendation honesty: say this out loud before recommending.
- **VERIFY THE API SURFACE BEFORE PLANNING ENGINE WORK** (the lineage's standing lesson, now paid off
  THREE times). Joints, RenderTarget, AND Timeline all already existed — each "gap" was "no game" plus at
  most an *ergonomics* hole, not a missing core API. For networking, **read `src/network/` + the `mp_*`
  examples first** (what `NetworkConfig`/the bounded channel/the client/server actually do, what's
  headless vs windowed) before claiming a gap. → Phase 1.
- **The loop is proven and fast** (5 sessions): confirm green → recommend + `/grill-me` scope-lock → plan
  → implement engine + playable example → HTML-checklist playtest + screenshot → verify.sh + rust-survivors
  path-patch check → single commit/push → confirm CI. → drives all phases.
- **The agent can run the playtest itself** when delegated, but the user may prefer to run it (this session
  they did). See memory `playtest-windowed-examples`. **macOS gotcha learned this session: Space = `key
  code 49`, NOT `keystroke " "`.** Diagnose render/timing bugs by an `eprintln` trace, not by chasing
  screenshots. → Phase 3.
- **`Camera` is top-left anchored, Y-down** (`src/camera.rs`): a camera at `(0,0)` shows world
  `[0,W]×[0,H]`, +Y down. Any example that moves entities/camera must use this. → Phase 2 footnote.
- **Engine-fix bar discipline holds:** every cycle locked "fix only the gap the example genuinely hits,
  zero change OK" and shipped a minimal additive change (lighting cull, blend crossfade, joint rotation
  sync, per-target offscreen submit, CameraTarget). Don't widen speculatively. → grill + Phase 2.

## Anti-Goals (What NOT To Do)

- **Don't skip `/grill-me`.** Every shipped candidate was scope-locked via grill first. Lock scope (example
  concept, the engine gap/API if any, compat/wasm posture, engine-fix bar, proof bar) before coding.
- **Don't re-scout from re-exports alone.** Read `src/network/` (the module + its tests) and the `mp_*`
  examples to confirm what the API does and doesn't do before claiming a "gap". (Joints/RenderTarget/
  Timeline are the cautionary tales — all already existed.)
- **Don't force networking if a better-value direction exists.** It's the least self-contained candidate.
  If the user prefers a non-dogfood item (a deferred follow-up, tooling, polish), that's legitimate — the
  "every subsystem in a game" goal is nearly met (only networking left).
- **Don't widen the engine surface beyond what the example needs.** Five cycles kept each change minimal
  (Timeline's was +138 incl. tests). Same discipline — don't speculatively add features.
- **Don't declare done on `verify.sh` alone.** CI can't run the windowed app / a live socket. The example's
  native run + playtest is the gate (networking especially — two clients + a server process).

## Plan

### Phase 1: Confirm green + lock the next candidate's scope

**Goal:** Start from a confirmed-green tip and a grill-locked scope.

**Why this approach:** Work is committed and CI was green at handoff; confirm before building on top.
Candidate choice is the user's call (established pattern) — recommend honestly, then `/grill-me`.

- Confirm CI green on `8049dfa`: `gh run list --branch main --limit 3` (expect success on run
  `27017341543`; if red, fix the named job — all checks passed locally so a failure would be environmental).
- Confirm clean state: `git status -s` (clean), `git log --oneline -3` (`8049dfa` + this session's
  `session:` commit at tip), `./scripts/verify.sh` (exit 0, 282 tests).
- **Read the real API surface first** (the anti-mis-scout step): `ls src/network/`; `sed -n '1,120p'` the
  network module(s); read `examples/mp_client.rs` + `examples/mp_server.rs` + their tests; note what's
  headless, what's windowed, what `NetworkConfig`/the bounded receive channel actually expose, and whether
  a second client can join a room today. That delta is the example's job.
- Recommend a candidate to the user — **networking** (the last never-in-a-game subsystem) with a clear
  caveat about its infra cost, OR a **non-dogfood direction** (deferred Timeline follow-up: on-finish
  event / camera rotation / sequencing; RenderTarget HUD-helper/resize; an editor/tooling pass; polish).
  Let them choose.
- Run `/grill-me` on the chosen candidate to lock: example concept, the engine gap/API to add (if any),
  the compat/wasm posture, the engine-fix bar, and the proof bar. Produce a grill decision packet.

**Files:** none (process phase).
**Validates with:** `gh run list` shows success on `8049dfa`; a written grill decision packet with
`plan_allowed: true`.
**Rollback:** n/a (no code).

### Phase 2: Implement the engine gap (if any) + a small playable example

**Goal:** Close the gap the candidate exposes (if any) and prove it with one playable example.

**Why this approach:** VISION — the example is the acceptance test; fix awkward API/bugs before release.
Mirrors how this session added the timeline_cutscene example + the CameraTarget camera-drive.

If **networking** (the last dogfood candidate):
- `mp_client`/`mp_server` exist but there's no small *playable* multiplayer toy. Scope something minimal —
  e.g. two clients each moving a colored dot in a shared room, positions relayed via the server, each
  client rendering both dots. Surface whatever the *example* (not a test) needs that the API didn't provide
  cleanly. Higher infra cost than any prior cycle — scope tightly; expect a multi-process run (server +
  2 clients) and native-only (tungstenite). The playtest will need to launch 3 processes.
- Decide the engine-fix bar in the grill (fix if small; document if big).

If a **non-dogfood direction**:
- Implement whatever the user picked (a deferred follow-up, tooling, polish), still gated by a playable
  example or a concrete proof per VISION where applicable.

- Keep changes additive (public API/behavior preserved); ensure `cargo build --target wasm32` (lib+bins)
  still builds. (Networking examples are native-only — like `platformer_game`/`mp_server` — so they're not
  in the wasm `--all-targets` gate anyway; confirm the lib still builds for wasm.)
- Add unit tests for any NEW engine surface. If a fix is GPU/socket/windowed-only, validate via the native
  run + playtest instead, and say so. Reuse the wrap-a-system / `world.get_mut` / take-then-add patterns.
- Use the **top-left/Y-down** camera convention (`src/camera.rs`).

**Files:** `src/network/...` (+ `src/lib.rs` re-exports if new types) or the chosen module; `examples/...`
(the playable example); tests; docs (CHANGELOG/NEXT_WORK/HANDOFF/CLAUDE.md).
**Validates with:** `./scripts/verify.sh` green after each change; new unit tests pass; native run exercises
the example end-to-end.
**Rollback:** revert the example + any new public API (additive, isolated).

### Phase 3: Playtest, cross-repo check, commit

**Goal:** Prove it in real play and land it.

**Why this approach:** Same proof bar that caught this session's `key code 49` skip gotcha and confirmed
the fade via an eprintln trace (CI can't run the window/socket).

- Native run; drive it; `screencapture` for a self-check. The agent can run the full checklist itself if
  the user delegates ("체크리스트 실행해줘") — see memory `playtest-windowed-examples` (and the **`key code
  49` = Space** note). For networking, the playtest launches a server + 2 clients.
- Build an HTML checklist (reuse `/tmp/timeline_cutscene_test.html` as a template — grouped items,
  localStorage, markdown export); deliver it + a screenshot; the user may run it themselves (this session
  they did) or delegate it.
- Fix anything surfaced (example-level or engine-level per the grill bar).
- `rust-survivors` `cargo check --workspace` against the tip via the `--config` path patch (see memory
  `rust-survivors-engine-pin`), then `git checkout -- Cargo.lock`. Run it even though rust-survivors uses
  no networking — it's a cheap API-compat formality.
- On full sign-off: single commit (`feat(...)` style) + push to `main` (the user's standing direct-to-main
  norm); confirm CI. Then `/handoff` (the user asked for handoff+commit/push at the end this session).

**Files:** the example + any fix; docs.
**Validates with:** playtest all-✅; verify.sh green; rust-survivors clean; CI green.
**Rollback:** `git revert` the feature commit (additive).

## Dependencies & Order

- Phase 1 → 2 → 3 strictly sequential (scope before code; code before playtest).
- Within Phase 2, any engine gap must land before the example can use it.
- Phase 3's rust-survivors check is a compile-compat formality (it uses no networking/Timeline) — run it
  anyway, it's cheap.

## Risks & Mitigations

- **Candidate choice may change at grill** (user's call). Likely. Mitigation: Phase 2 is written for both
  networking and a non-dogfood direction.
- **Networking needs more infra than any prior cycle** (server + 2 clients, native-only, sockets). High.
  Mitigation: scope a *minimal* toy; decide the engine-fix bar explicitly; expect a multi-process playtest;
  if it's too big for one cycle, document the gap and ship the smallest demonstrable slice.
- **Re-scouting a non-existent gap** (the standing lesson — joints/RenderTarget/Timeline all already
  existed). Mitigation: Phase 1 reads `src/network/` + the `mp_*` examples before claiming a gap.
- **CI red on `8049dfa`.** Low (CI already green, run `27017341543`). Mitigation: Phase 1 confirms before
  building.

## Success Criteria

- **Minimum viable:** the chosen subsystem/direction gains a playable example (or concrete proof) that
  exercises it in real play; any engine gap it surfaces is closed (or documented if big); `verify.sh` + new
  tests + native playtest + rust-survivors check all green; committed/pushed/CI-green.
- **Full success:** if networking — the example reads as a small *playable* multiplayer toy (two dots in a
  shared room), any new public API is minimal and fork-friendly, and `docs/NEXT_WORK.md` trims networking
  from the remaining list (candidate **M**), **completing the "every shipped subsystem dogfooded in a
  game" goal.** If a non-dogfood direction — the picked item lands with its own proof bar met.

## Quick Start

```bash
# Restore full context
cat plans/handoffs/HANDOFF_timeline-cutscene_camera-drive_2026-06-05.md

# Verify starting state
git log --oneline -3                       # expect 8049dfa + this session's session: commit at tip
git status -s                              # expect clean
gh run list --branch main --limit 3        # confirm CI green on 8049dfa (run 27017341543)
./scripts/verify.sh                        # 5 checks, expect exit 0 (282 tests)

# Scout the recommended candidate (networking) — READ THE REAL SURFACE FIRST
ls src/network/ 2>/dev/null; sed -n '1,120p' src/network*/*.rs 2>/dev/null
sed -n '1,80p' examples/mp_server.rs; sed -n '1,80p' examples/mp_client.rs
sed -n '198,260p' docs/NEXT_WORK.md        # candidate L entry + remaining (networking only)
cat docs/VISION.md                         # the feature+example loop

# First action
#   1) Confirm CI green on 8049dfa.
#   2) Recommend networking (last never-in-a-game; flag its infra cost) OR a non-dogfood direction;
#      /grill-me the chosen one.
#   3) Phase 2: build a small playable example + fix what real play needs (per the grilled engine-fix bar).
```
