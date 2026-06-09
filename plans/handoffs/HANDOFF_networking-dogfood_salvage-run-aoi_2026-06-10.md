# `salvage_run` — AOI-streaming networked example (5th example, the "3rd direction") + seq-8 follow-ups closed (v4.5.0)

**Date:** 2026-06-10
**Status:** COMPLETED — built `salvage_run` (area-of-interest streaming world), the 3rd of the design doc's candidate networked directions; engine **bumped 4.4.0 → 4.5.0** (additive: example only, no engine API change). Also closed both schedulable seq-8 "Where We're Going" items: orbital_dodger `INTERP_DELAY_DEFAULT` **settled at 120 ms** by real play (no change — already the default), and **rust-survivors engine pin bumped v4.1.0 → v4.4.0** (verified + committed + pushed). All committed + **pushed**: skeleton-engine `origin/main` `18d9e0e..8dd042c` (2 commits), rust-survivors `origin/main` `e6176fa..f686e8a` (1 commit). Tree clean. Full `+1.88.0` + plain-stable gate green; GUI playtest (7 captures) confirmed the whole feature set.
**Bead(s):** none (`bd` unavailable — tracked with TaskCreate this session, tasks #6–#13 all completed)
**Epic:** VISION feature+example loop — networking dogfood (interest management / large-world netcode as the next dogfood direction)
**Chain:** `networking-dogfood` seq `9`
**Parent:** `HANDOFF_networking-dogfood_snapshotbuffer-orbital-dodger_2026-06-09.md` (seq 8)
**Prior chain:** `coin-race-example` (1) > `wasm-coin-race-v4.1` (2) > `deferred-polish` (3) > `client-prediction-shooter` (4) > `phase-d-realplay` (5) > `phase3-polish` (6) > `interp-gilrs-fix` (7) > `snapshotbuffer-orbital-dodger` (8) > this (9)

---

## Stale References

Parent (seq 8) state that changed this session — next session beware:

- **Engine version** — `4.4.0` → **`4.5.0`** (`Cargo.toml` + `Cargo.lock`). Additive (new example only).
- **`origin/main` (skeleton-engine)** — was `f62df90` (seq 8 said pushed there; seq-8's own doc commit was `18d9e0e`) → now **`8dd042c`**.
- **rust-survivors engine pin** — seq 8 listed "bump to v4.4.0" as a *pending optional* item; it is now **DONE**: `crates/game/Cargo.toml` rev `7c6f9c0` (v4.1.0) → `f62df900b50c3d3a72f4fd951d77db5271eb96d8` (v4.4.0), committed `f686e8a`, pushed. (The pin lagged at **v4.1.0**, not v4.4.0-minus-one as one might assume — 3 minors behind.)
- **orbital_dodger `INTERP_DELAY_DEFAULT`** — seq 8 left it "to settle by feel (0.12 s analytic)". Now **validated by real play and kept at 0.12 s (120 ms)** — no code change. 80 ms is the smoothness floor; 120 ms is the chosen safe value.
- **No engine *API* identifiers changed.** `SnapshotBuffer<T: Lerp>`, `RemoteEntities<K>`, `NetworkClient`, `Camera`, `Lerp` are all unchanged. salvage_run is a pure consumer of the existing public API.
- New example-local identifiers (only matter inside `examples/games/salvage_run/`): `Kind {Salvage,Drone}`, `EntityState`, `ClientMsg::Move{x,y,r}`, `ServerMsg::{Welcome,Snap}`, `SalvageClient`, `start_pos()` (edge spawn).

## Since Last Handoff

Seq 8 left four optional, unscheduled "Where We're Going" items: (1) settle orbital_dodger `interp_delay` by feel, (2) bump rust-survivors' engine pin to v4.4.0, (3) promote `Prediction` / design Q#3–#7 (await examples), (4) survivor gamepad / macOS GCController (env-blocked). This session the user chose to **knock out #1 and #2 and then build a fresh example for the #3 direction**.

- **#1 settled.** User playtested orbital_dodger live (this session relaunched it for them); reported "smooth from ~80 ms", then "120 ms with a safe margin is fine." 120 ms is already the default → **no change**, just validation. The seq-8 open question #1 (optimal interp_delay) is now answered for this example.
- **#2 done + pushed.** Verified rust-survivors builds clean against the local engine v4.4.0 (found the pin was at **v4.1.0**), then bumped the rev to the pushed `f62df90`, ran the full `+1.88.0` game gate (fmt/clippy/`test -p game --lib` = 200), committed surgically (2 files only — the repo carries ~19 unrelated WIP doc files on main), pushed.
- **#3 direction advanced.** Built **`salvage_run`**, the "many-entity / interest-managed world" candidate from `docs/REMOTE_ENTITIES_DESIGN.md`. This finally stresses open questions **#4 (typed/multi-class), #5 (staleness), #7 (disconnect)** — documented as findings (no engine change warranted yet). **#6 (binary protocol) is now the single untouched direction.**
- **#4 untouched** (gamepad/GCController — still macOS-blocked).

## Reference Documents

- `docs/REMOTE_ENTITIES_DESIGN.md` — the design rationale; now has a "5th example (salvage_run, AOI streaming)" findings section closing the analysis on #4/#5/#7 (no engine change; candidate helpers flagged, not built). #6 still open.
- `docs/VISION.md` — the feature+playable-example loop (the example IS the acceptance test; if gameplay feels awkward, fix it before release — which is exactly why the idle-auto-win was fixed mid-playtest).
- `docs/NEXT_WORK.md` — seq-8 section exists; NOT updated this session (consider a seq-9 line next time).
- `CLAUDE.md` — module-map network row updated (salvage_run = 5th example, AOI streaming).
- Templates mirrored: `examples/games/orbital_dodger/{server,protocol,orbital_dodger}.rs` + `web/` (the structural blueprint) and `examples/games/predict_shooter/server.rs` (per-client send pattern). `examples/minimap.rs` (camera-follow + background-tile pattern).
- Plan: `/Users/jkl/.claude/plans/lively-singing-cocke.md` (this session's approved salvage_run plan).

## The Goal

Continue the networking dogfood: after two interpolating examples (predict_shooter, orbital_dodger) answered design questions #1/#2 and triggered the `SnapshotBuffer<T>` promotion, build the **3rd distinct direction** — a many-entity, **interest-managed (AOI streaming)** world — to stress the still-open `RemoteEntities` design questions (#4 typed entities, #5 staleness/eviction, #7 disconnect). End state: a fork-friendly, genuinely playable example that demonstrates server-side area-of-interest streaming + client-side eviction + interpolation reuse, green on the CI-pinned gate, version-bumped additively, committed + pushed. Plus tidy up the two ready seq-8 loose ends (interp_delay feel, rust-survivors pin).

## Where We Are

- **Branch `main`, pushed to `origin/main` at `8dd042c`** (was `18d9e0e`). Tree clean. Engine **v4.5.0**.
- **2 commits this session** (`8dbd557`, `8dd042c`): example+manifest+version · docs.
- **rust-survivors** pushed to its `origin/main` at `f686e8a` (the pin bump); its 19 WIP doc files remain safely uncommitted.
- **Full gate GREEN** on `cargo +1.88.0` AND plain stable (`./scripts/verify.sh ✓`): fmt · clippy `--all-targets -D warnings` · wasm lib+bins · `build --example salvage_run --target wasm32` · `test --all-targets` (**lib 339 unchanged** + salvage_run_server **5**, predict_shooter 7, 0 failures) · `RUSTDOCFLAGS=-D warnings doc`.
- **`salvage_run` example** (`examples/games/salvage_run/`): `protocol.rs`, `server.rs`, `salvage_run.rs`, `web/{index.html,build.sh}`. Two `[[example]]` entries: `salvage_run` + `salvage_run_server`. Port **9005** (coin_race 9002, predict_shooter 9003, orbital_dodger 9004).
- **Server** (`server.rs`): a large world (2400×1800) of 120 wandering entities (80 `Salvage` + 40 `Drone`); 60 Hz sim thread; **12 Hz** per-client snapshots. Each client reports its AOI centre + radius via `ClientMsg::Move{x,y,r}`; the server streams it **only** the entities within `r` (squared-distance filter, `entities_within(cx,cy,r)`). Dependency-free xorshift `Rng`; wall-bounce (snap-back+flip-sign); no spin. 5 unit tests.
- **Client** (`salvage_run.rs`): `SalvageScene` + `SalvageClient`. Two `RemoteEntities<usize>` maps (`salvage`, `drones`) + two `SnapshotBuffer<Vec2>` maps (`salvage_buf`, `drone_buf`) keyed by the same global id space; `last_seen: HashMap<usize,f64>` for eviction; `collected: HashSet<usize>` to suppress re-streaming of grabbed salvage. Camera scrolls the large world (manual `camera.position = player - viewport/2`, bounds set, App auto-clamps). An AOI **boundary ring** of 28 dim dots is repositioned around the player each frame (circular — matches the server's filter). Local player roams (WASD/arrows), reports `Move` at ~15 Hz (immediate on AOI resize). `-`/`=` resize the AOI, `[`/`]` tune interp delay, `R` reset, `Esc` quit.
- **The mechanic:** drive to collect 12 salvage (`COLLECT_GOAL`) while dodging drones (touch a drone → respawn to start). Salvage drifts very slowly (near-static collectibles, 3–11 px/s); drones roam fast (70–150 px/s) — the drones' speed is what makes the 12 Hz interpolation visible.
- **GUI playtest passed (VISION acceptance test)** — 7 captures, all behaviors confirmed (see Evidence).
- **Docs:** CHANGELOG (`## 4.5.0`), REMOTE_ENTITIES_DESIGN (5th-example findings), CLAUDE.md (network row). REFERENCE.html intentionally NOT touched (no new public API).
- **Memory updated:** `engine-current-state` → v4.5.0/salvage_run; `rust-survivors-engine-pin` → bumped+pushed + the cargo-update patch gotcha; `MEMORY.md` index hook.

## What We Tried (Chronological)

1. **Onboarded from the seq-8 handoff** (the session opened with its paste prompt). Read it fully; verified `HEAD == origin/main == 18d9e0e`, tree clean, **lib 339 tests** baseline on `+1.88.0`. Read key files (SnapshotBuffer in network.rs, orbital_dodger client/protocol/server, the `Lerp` trait) + cross-checked the orthogonality claim (predict_shooter uses `SnapshotBuffer<Vec2>`, `Interp` gone; coin_race/mp_client don't reference `SnapshotBuffer`). Narrated onboarding, proposed rust-survivors pin verify (#2) as the first action, waited.
2. **User: "run #2 and #3 in the background while I real-play #1."** Relaunched orbital_dodger (server already up on 9004; client #2 connected) for the user.
3. **#2 verification — patch gotcha.** First `cargo check --config 'patch...path=...'` was REJECTED (`patch ... was not used in the crate graph`) and Cargo.lock still showed v4.1.0. Cause: the pin lagged the local version, so cargo wouldn't substitute the patch into an already-locked git dep without a re-resolve. Fix: `cargo update -p skeleton-engine --config 'patch...'` re-locked the git source onto the local path (dropping git v4.1.0, adding local v4.4.0 + gilrs 0.10→0.11.2 transitively); then `cargo check --workspace` was clean. Restored Cargo.lock (`git checkout`). Recorded the 2-step gotcha in memory.
4. **#3 prep research.** Read `docs/REMOTE_ENTITIES_DESIGN.md` open questions #3–#7; confirmed `Prediction` promotion is gated (no 2nd prediction example) → #3 has no actionable code. The unlocking candidate directions: many-entity/interest-managed (#4/#5), binary-protocol (#6), disconnect-teardown (#7).
5. **User playtest feedback (#1):** "ON smooth / OFF judders" (feature validated), then "smooth from ~80 ms", then **"120 ms with a safe margin is fine"** → settled, no change.
6. **#2 finished.** Bumped the pin rev to `f62df90`, `cargo update -p skeleton-engine`, ran the game gate (fmt/clippy/`test -p game --lib`=**200**), committed `f686e8a` (2 files only), left the 19 WIP docs untouched.
7. **User chose the 3rd direction = AOI streaming.** Entered plan mode. Two parallel Explore agents (Sonnet) mapped (a) the Camera follow / large-world / DrawText-world-anchor APIs, (b) the per-client server pattern + example scaffolding + RemoteEntities surface. Confirmed exports (`Camera` lib.rs:68, `ViewportSize` :113, `DrawRect` :107, `SnapshotBuffer`/`RemoteEntities` :85) + `NetworkClient::send_text` (network.rs:276). One Plan agent produced the execution-ready design. Wrote the plan, AskUserQuestion confirmed direction = AOI streaming + theme = space salvage. ExitPlanMode → approved.
8. **Built the example.** `protocol.rs` (consts, `Kind`, `EntityState`, `ClientMsg::Move`, `ServerMsg::{Welcome,Snap}`) → `server.rs` (AOI filter + per-client send + 5 tests) → `salvage_run.rs` (client). Early native compile caught one **borrow error**: held the `InputState` borrow across `self.reset(world)` — fixed by reading all key states into locals first, then acting. Both examples compiled; 5 server tests passed.
9. **Web harness + manifest + version.** Cloned orbital_dodger's `web/{build.sh,index.html}` (swap names + port 9005 + `run_salvage_run`), exec bit set. Added two `[[example]]` entries; version 4.4.0 → 4.5.0.
10. **Gate, round 1.** `cargo +1.88.0 fmt` reformatted; the chained gate showed all `=0` markers BUT a real `could not compile salvage_run_server` — the `=0` were **`tail` exit codes masking the piped command** (a scripting trap). Actual cause: clippy `uninlined_format_args` on the server's startup `println!` (both trailing args `WORLD_W`/`WORLD_H` are plain consts → clippy demands inlining). Fixed by inlining `{WORLD_W:.0}x{WORLD_H:.0}`. (The client HUD formats mix in field accesses, so clippy skips them — confirmed by re-run.)
11. **Gate, round 2 (true exit codes).** clippy clean, `test --all-targets` (lib 339 + server 5, 0 fail), then `./scripts/verify.sh` → **all checks passed ✓**.
12. **GUI playtest (the acceptance test).** Launched server+client, osascript window control + screencapture. First capture revealed a **playability flaw**: the HUD already said "Salvage run complete — 12 collected!" — the player auto-won while idle (spawned at the dense world centre; drifting salvage wandered into the stationary player). Per VISION, fixed gameplay: (a) spawn at a sparse left edge (`start_pos() = (WORLD_W*0.12, WORLD_H*0.5)`, replacing `world_center()` everywhere), (b) slow salvage to 3–11 px/s (near-static collectibles you must drive to), (c) fixed the respawn message "centre" → "start". Rebuilt; re-playtested: now `collected 0/12` after idling — a proper roam-to-collect game.
13. **Playtest evidence (7 captures).** Confirmed: rendering + typed colours, AOI streaming `21/120`, roam → camera scroll + different entity set, **AOI grow 520→820 = 15→40 entities**, **AOI shrink 820→220 = 40→5 (eviction)**, the boundary ring tightening to the radius, collection, drone→respawn, and **disconnect → streaming 0/120 (clear)**.
14. **Re-ran fmt/clippy/test on the post-playtest edits** (edge start, slower salvage, wording) — all green.
15. **Committed (2 logical commits), pushed both repos, updated 3 memory files.** Then this handoff.

## Key Decisions

- **AOI streaming was the right "3rd direction."** It stresses the most open questions at once (#4 typed, #5 staleness, #7 disconnect) and is genuinely distinct (all prior examples have a small FIXED entity set; this has spawn/despawn churn + interest management). Binary-protocol (#6) is narrower; disconnect-teardown (#7) is narrowest. (Confirmed with the user via AskUserQuestion.)
- **No engine API change — purely an additive example (v4.5.0).** Per the design-doc discipline, the example DRIVES the API: built example-local first (two `RemoteEntities` maps, a `last_seen`+timeout eviction). Findings documented; candidate helpers FLAGGED not built (extraction gated on a 2nd stressing example, exactly as `SnapshotBuffer` was). Minor bump since the public API surface is unchanged but a new shipped example is a meaningful addition.
- **Per-client tailored snapshots (not broadcast).** Unlike orbital_dodger's broadcast server, each `Snap`'s *entity set* differs per client (its AOI). The client sends `ClientMsg::Move{x,y,r}` — the first networked example here with a client→server channel that's pure interest-management (no input/prediction).
- **`Kind` as a 2-variant enum, two `RemoteEntities` maps.** The "N maps" baseline for #4. The seam it exposes (a `contains_key` probe across both maps on eviction to find an id's kind) is the documented #4 datapoint — and the clean answer needs **zero engine change** (`RemoteEntities<(Kind,usize)>` already compiles, `K: Eq+Hash`).
- **Eviction by client-side last-seen timeout (removal-by-omission).** The server never sends a removal; it just stops including out-of-AOI entities. `STALE_TIMEOUT = 3/SNAPSHOT_HZ ≈ 0.25 s` is also the edge-flicker hysteresis (survives 2 missed snapshots). This is the clean single-call-site pattern flagged as the next candidate engine helper for #5.
- **Fixed the idle-auto-win mid-playtest (VISION).** Centre spawn + touch-collect of drifting salvage = AFK win. Fix: edge spawn + near-static salvage so the player must actively roam — which also better showcases AOI streaming (entities stream in as you move). The example must be genuinely playable, not just technically working.
- **Circular AOI boundary ring (28 dots), not a square `DrawRect`.** The server filter is circular (`dx²+dy²≤r²`); a square outline would mismatch and confuse ("why didn't that corner entity stream?"). 28 repositioned dot sprites match the filter exactly and need no new draw API.
- **`collected: HashSet` to suppress re-streaming.** Without it, a collected salvage (server unaware of collection) re-streams next snapshot and re-counts. With it, collection is final client-side — correct for a "collect them all" goal.
- **rust-survivors pin: surgical 2-file commit, not bundled with WIP.** The repo carries ~19 unrelated WIP doc edits on main; staged only `crates/game/Cargo.toml` + `Cargo.lock`.

## Evidence & Data

### Commits this session
| Repo | Hash | Summary |
|---|---|---|
| skeleton-engine | `8dbd557` | example: salvage_run — AOI-streaming networked world (v4.5.0) |
| skeleton-engine | `8dd042c` | docs(v4.5.0): salvage_run AOI example; RemoteEntities #4/#5/#7 findings |
| rust-survivors | `f686e8a` | deps: bump skeleton-engine pin 4.1.0 -> 4.4.0 (f62df90) |

Each engine commit is bisectable: #1 = the example + manifest + version (no doc refs needed); #2 = docs only.

### `salvage_run` constants (`protocol.rs`)
| Const | Value |
|---|---|
| `SERVER_ADDR` | `127.0.0.1:9005` |
| `FIXED_DT` / `SNAPSHOT_HZ` | `1/60` / `12` (snap_every = 5) |
| `WORLD_W` / `WORLD_H` | `2400` / `1800` |
| `VIEW_W` / `VIEW_H` | `800` / `600` |
| `ENTITY_COUNT` | `120` (`SALVAGE_COUNT` 80 + `DRONE_COUNT` 40) |
| `AOI_RADIUS_DEFAULT` / MIN / MAX / STEP | `520` / `200` / `1100` / `60` |
| `SALVAGE_RADIUS` / `DRONE_RADIUS` / `PLAYER_RADIUS` | `12` / `16` / `14` |
| `PLAYER_SPEED` | `320` px/s |
| `SALVAGE_MIN/MAX_SPEED` | `3` / `11` px/s (lowered from 8/28 in playtest) |
| `DRONE_MIN/MAX_SPEED` | `70` / `150` px/s |
| `COLLECT_GOAL` / `MOVE_SEND_HZ` | `12` / `15` |

### Client tuning (`salvage_run.rs`)
| Const | Value | Note |
|---|---|---|
| `INTERP_DELAY_DEFAULT` | `0.125` s | ~1.5 snapshot intervals |
| `INTERP_DELAY_MIN/MAX/STEP` | `0.0` / `0.40` / `0.02` | `[`/`]` tune |
| `STALE_TIMEOUT` | `3 / SNAPSHOT_HZ` ≈ 0.25 s | eviction + hysteresis |
| `MOVE_SEND_INTERVAL` | `1 / MOVE_SEND_HZ` ≈ 0.067 s | throttle, force-sent on AOI resize |
| `AOI_RING_DOTS` | `28` | circular boundary |
| `start_pos()` | `(WORLD_W*0.12, WORLD_H*0.5)` = (288, 900) | sparse edge spawn |

### Test counts (all green)
| Suite | Count | Note |
|---|---|---|
| lib (`src/`) | **339** | unchanged (salvage_run adds no lib tests) |
| salvage_run_server | 5 | typed spawn, in-bounds@2000, AOI filter correctness + monotonic, radius extremes, protocol round-trip |
| predict_shooter (client_net) | 3 | unchanged |
| predict_shooter_server | 7 | unchanged |
| rust-survivors game (`-p game --lib`) | 200 | against v4.4.0 (+1.88.0) |

### Gate results (skeleton-engine, all green)
| Check | `+1.88.0` | plain (`verify.sh`) |
|---|---|---|
| fmt --check | OK | OK |
| clippy --all-targets -D warnings | OK (after the println inline fix) | OK |
| build --target wasm32 (lib+bins) | OK | OK |
| build --example salvage_run --target wasm32 | OK | — |
| test --all-targets | 339 lib + 5 server, 0 fail | same |
| RUSTDOCFLAGS=-D warnings doc | OK | OK |
| **umbrella** | — | **all checks passed ✓** |

### GUI playtest captures (`/tmp/sr_*.png`)
- `sr_02_edge_start` — "Connected — collect salvage, dodge the drones!", player at the sparse left edge, `streaming 21/120`, typed colours (cyan salvage / warm drones), `collected 4/12` (pre-salvage-slowdown).
- `sr_03_idle` — after the salvage-speed fix: `collected 0/12` while idle (auto-win gone); the 28-dot AOI ring visible around the player.
- `sr_04_roamed` — held right+down 3 s: camera scrolled (shifted grid), different entity set, `collected 1/12`, "Hit a drone — back to the centre!" (movement + drone collision + respawn).
- `sr_05_aoi_grown` — pressed `=` ×5: **AOI 520→820, streaming 15→40** (interest management made visible).
- `sr_06_aoi_shrunk` — pressed `-` ×10: **AOI 820→220, streaming 40→5** (eviction); the ring tight around the player.
- `sr_08_disconnect2` — killed the server: "Disconnected: error (is the server running?)", **streaming 0/120** (all streamed entities cleared; only local player + ring remain).
- (`sr_07_disconnect` accidentally captured the terminal — the shell `pkill` brought it frontmost; re-focused + re-captured as `sr_08`.)

### Primary-evidence code (expensive to re-derive)

Server AOI filter (`server.rs`) — the core new algorithm:
```rust
fn entities_within(&self, cx: f32, cy: f32, r: f32) -> Vec<EntityState> {
    let r2 = r * r;
    self.entities.iter().enumerate()
        .filter(|(_, e)| { let dx=e.x-cx; let dy=e.y-cy; dx*dx + dy*dy <= r2 })
        .map(|(id, e)| EntityState { id, kind: e.kind, x: e.x, y: e.y })
        .collect()
}
// broadcast_snapshot() loops clients.values() and builds a per-client Snap from this.
```

Client eviction (`salvage_run.rs`) — the #5 removal-by-omission pattern (probing both maps = the #4 wart):
```rust
let cutoff = self.client_time - STALE_TIMEOUT;
let stale: Vec<usize> = self.last_seen.iter().filter(|(_, &t)| t < cutoff).map(|(&id,_)| id).collect();
for id in stale {
    if self.salvage.contains_key(&id) { self.salvage.remove(world, &id); self.salvage_buf.remove(&id); }
    else if self.drones.contains_key(&id) { self.drones.remove(world, &id); self.drone_buf.remove(&id); }
    self.last_seen.remove(&id);
}
```

## Code Analysis

- **No engine change confirmed.** Every need maps onto existing public API: `RemoteEntities<usize>` (×2), `SnapshotBuffer<Vec2>` (×2, 3rd call site), `Camera` (manual `position` + `bounds` auto-clamp via `src/app/schedule.rs:~279`), `NetworkClient::send_text`, `ViewportSize`-equivalent (used the `VIEW_W/H` consts instead), `DrawText`/`TextQueue`, `Sprite::colored`, `Transform`.
- **Camera follow pattern:** set `cam.bounds = Some((ZERO,(WORLD_W,WORLD_H)))` + `follow_entity=None` once in `on_enter`; each frame `cam.position = player_pos - (VIEW_W,VIEW_H)/2`; App auto-clamps after the system runs. World (2400×1800) > viewport on both axes so the "world smaller than viewport" pin-to-min branch never fires.
- **Collect-then-apply borrow discipline (mirrors orbital_dodger):** build `updates: Vec<(usize,Entity,Vec2,Kind)>` from immutable `RemoteEntities::iter()` + `SnapshotBuffer::sample(rt)` reads FIRST, then loop and `world.get_mut::<Transform>`. The same vec feeds collision/collect. Also: read all key `just_pressed` states into locals before any `world` mutation (the borrow error fixed in step 8).
- **Re-entry after eviction is safe by construction:** eviction drops the `SnapshotBuffer` (not just the map), so a re-streamed entity gets `or_default()` → a fresh empty buffer → `sample` clamps to the single new sample (no NaN, no stale interpolation across the gap).
- **`uninlined_format_args` trap:** clippy `-D warnings` fires when ALL trailing format args are inlinable (plain consts, as in the server's `println!("... {WORLD_W} ...", WORLD_W, WORLD_H)`); it SKIPS the macro when any arg is a field access / expression (the client HUD formats). Inline const-only args.
- **Piped exit-code trap:** `cmd 2>&1 | tail -N; echo $?` returns `tail`'s exit, masking `cmd` failure. Use `cmd > log 2>&1; echo $?` or `${PIPESTATUS[0]}` when gating in a chained script.
- **`snap_every`:** `round(1/FIXED_DT / SNAPSHOT_HZ) = round(60/12) = 5` — snapshot every 5th tick.

## Files Changed

### New example (`examples/games/salvage_run/`)
- `protocol.rs` — consts, `Kind {Salvage,Drone}`, `EntityState{id,kind,x,y}`, `ClientMsg::Move{x,y,r}`, `ServerMsg::{Welcome{world_w,world_h,total},Snap{tick,entities}}`. `#![allow(dead_code)]`.
- `server.rs` — `Entity`/`Client`/`Server`, xorshift `Rng`, `step()` (drift+bounce), `entities_within()` (AOI filter), per-client `broadcast_snapshot()`, `handle_client` with `Move` handling, 5 unit tests.
- `salvage_run.rs` — `SalvageScene` + `SalvageClient` (2 `RemoteEntities` + 2 `SnapshotBuffer` maps, `last_seen`, `collected`, camera follow, 28-dot AOI ring, collect/win, drone-respawn, `disconnect_cleanup`), wasm entry `run_salvage_run`, `start_pos()`.
- `web/index.html` + `web/build.sh` (exec bit set) — wasm harness, port 9005.

### Modified
- `Cargo.toml` — version `4.4.0` → `4.5.0`; two `[[example]]` entries (`salvage_run_server`, `salvage_run`).
- `Cargo.lock` — skeleton-engine `4.5.0`.
- `docs/CHANGELOG.md` — `## 4.5.0` → `### Added` (salvage_run).
- `docs/REMOTE_ENTITIES_DESIGN.md` — "5th example (salvage_run, AOI streaming)" findings section (#4/#5/#7) + net conclusion.
- `CLAUDE.md` — module-map network row (5th example, AOI streaming).
- **rust-survivors:** `crates/game/Cargo.toml` (pin rev) + `Cargo.lock` (`f686e8a`).

### Memory
- `engine-current-state.md` → v4.5.0/salvage_run; `rust-survivors-engine-pin.md` → bumped+pushed + 2-step patch gotcha; `MEMORY.md` index hook.

## User Feedback & Preferences (this session)

- **Parallelized the work:** "run #2 and #3 in the background while I real-play #1." (Then chose AOI streaming for #3, push both repos at the end, and "memory update + prepare to close" → `/handoff`.)
- **Subjective feel is the user's call:** settled orbital_dodger interp_delay at 120 ms themselves (reported the 80 ms smoothness floor); confirmed the safe default.
- **Standing prefs (memory):** Korean conversation / English artifacts; gate with `cargo +1.88.0` (not plain cargo); release/version decisions are the user's; use subagents for parallel work (used 2 Explore + 1 Plan on Sonnet this session); surgical commits in rust-survivors (don't bundle WIP).
- Followed the onboarding ritual (narrate understanding → state verification → read key + adjacent files → propose first action → wait) at session start.

## Where We're Going

Optional, none scheduled (the chosen work stream is complete):

1. **#6 — binary-protocol sync example** (the one untouched design direction). A non-JSON, compact-id (`u16`) networked example would stress design Q#6 and confirm whether `RemoteEntities<u16>` + a binary `SnapshotBuffer` payload need anything. This is the natural "next dogfood example."
2. **Extract the flagged candidate helpers IF a 2nd stressing example appears:** the **last-seen eviction tracker** (#5 — salvage_run is the 1st call site) and/or adopting **`RemoteEntities<(Kind,id)>`** as the #4 answer. Single call site now → defer (the SnapshotBuffer discipline).
3. **salvage_run `web/pkg/` is NOT built** (gitignored). To ship to the browser, run `examples/games/salvage_run/web/build.sh` (needs a `wasm-bindgen-cli` matching the crate in Cargo.lock, then serve over http). The wasm *compile* is gated; the browser render was not eyeballed.
4. **Optional gameplay polish** for salvage_run if the user wants a tighter feel: drone density/speed, collision sizes, a minimap, or making collection server-authoritative (a different example about server game state).
5. **Still-open seq-7/8 env-gated items:** survivor gamepad live-validation / macOS GCController backend (macOS can't read Xbox/PS pads via gilrs); `Prediction` promotion (awaits a 2nd prediction example).
6. **rust-survivors** is now pinned to engine v4.4.0; future engine bumps need a re-pin (same `--config patch` + `cargo update` verify dance — see `rust-survivors-engine-pin` memory).

## Risks & Blockers

- **salvage_run server is native-only** (tungstenite) — do NOT gate it on `wasm32 --all-targets` (documented WASM gotcha). The client builds to wasm (verified). CI's lib+bins wasm gate is the real gate.
- **Collected salvage is client-side only** (server is unaware of collection) — a documented demo simplification (`collected: HashSet` suppresses re-streaming). Making it authoritative is a different example.
- **AOI edge behaviour is timeout-based**, not server-hysteresis — fine for reliable WebSocket; a lossy transport (#6 territory) might want server-side relevance hysteresis.
- **rust-survivors WIP** (~19 doc files) sits uncommitted on its main — leave it; only the pin commit was made/pushed.
- Gate with `cargo +1.88.0` for CI parity (local stable diverges — `ci-toolchain-pin` memory). Watch the piped-exit-code trap when scripting the gate.

## Open Questions

- **#6 (binary protocol)** is the only untouched design direction — the obvious next example.
- **Should `SnapshotBuffer` ever own latest()/extrapolation?** Still not needed (the `rt = client_time` trick already gives raw-latest; salvage_run doesn't even use it).
- **salvage_run feel** — is the current collect-12 / dodge-drones loop fun enough, or does it want a polish pass? (Subjective — the user's call.)

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4 && git status -s     # clean; main @ 8dd042c == origin/main; v4.5.0
# bd is UNAVAILABLE — track with TaskCreate.

# Verify (gate with the CI pin, NOT plain cargo — local stable diverges):
cargo +1.88.0 test --lib                   # 339 pass
cargo +1.88.0 test --example salvage_run_server   # 5 pass
./scripts/verify.sh                        # green on local stable too

# Key files this session:
#   examples/games/salvage_run/{protocol,server,salvage_run}.rs + web/   (AOI streaming example)
#   src/network.rs (SnapshotBuffer<Vec2> reused here; RemoteEntities ×2)
#   docs/REMOTE_ENTITIES_DESIGN.md (5th-example findings; #4/#5/#7 stressed, #6 open)

# Run it (interest management made visible):
#   T1: cargo run --example salvage_run_server
#   T2: cargo run --example salvage_run    # WASD roam · -/= resize AOI (watch streaming X/120) · [ ] interp

# Next action (optional, none scheduled): build the #6 binary-protocol example,
#   OR a salvage_run gameplay-polish pass, OR extract the #5 eviction helper on a 2nd example.
```

## Session Closed
**Closed at:** 2026-06-10
**Commit:** `8dd042c` (salvage_run = 2 feature commits, pushed to `origin/main`) + rust-survivors `f686e8a` (pin bump, pushed) + this handoff doc
**Session status:** Handed off to next session. networking-dogfood seq 9 complete — `salvage_run` AOI-streaming example shipped (engine v4.5.0, additive), design Q#4/#5/#7 stressed + documented, orbital_dodger interp settled at 120 ms, rust-survivors pinned to v4.4.0. Full `+1.88.0` + plain-stable gate green (339 lib + 5 server); GUI playtest confirmed streaming/eviction/interpolation/disconnect-clear.
