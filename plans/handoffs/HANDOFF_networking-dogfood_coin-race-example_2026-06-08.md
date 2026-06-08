# Networking dogfood — authoritative multiplayer "coin race" playable example

**Date:** 2026-06-08
**Status:** COMPLETED — first playable multiplayer game shipped (`coin_race_game` + `coin_race_server`); closes networking, the last engine subsystem with no playable-example coverage. Full engine gate green; protocol verified end-to-end; live win confirmed by screenshot.
**Bead(s):** none (`bd` unavailable in this project)
**Epic:** none (VISION feature+example loop — breadth coverage)
**Chain:** `networking-dogfood` seq `1` (NEW chain)
**Parent:** none
**Related:** `HANDOFF_code-analysis-fixes_rust-survivors-v4-verify_2026-06-08.md` (seq 4, code-analysis chain — CLOSED). That handoff's "Where We're Going" named the networking dogfooding example as a possible next initiative; this session picked it up. It is a sibling reference, NOT a parent (different work stream).

---

## Since Last Handoff (launching context)

The `code-analysis-fixes` chain (seq 1→4) closed last session: engine code-analysis epic done → v4.0.0, and `rust-survivors` re-verified green against v4. Its seq-4 "Where We're Going" listed four out-of-chain candidate initiatives, none scheduled:
1. organize the ~20 rust-survivors WIP doc files (user's call),
2. **networking dogfooding cycle** — the last never-in-a-game subsystem,
3. repo-wide example Korean→English comment conversion,
4. git-tag engine `v4.0.0`.

This session: onboarded, ran the engine baseline gate (green), recommended **#2 (networking)** as the highest VISION-alignment work, the user agreed, chose the **Coin race (authoritative server)** game concept, and we built + verified + documented it. Items 1, 3, 4 remain untouched (see "Where We're Going").

## Reference Documents

- `docs/VISION.md` — the feature+example core loop. The bar this work satisfies: "a feature is not done until a small, playable example game in `examples/` exercises it in real play" + "fix only the API gap the example hits."
- `docs/NEXT_WORK.md` — candidate list; the networking gap was line ~220 ("Remaining never-in-a-game subsystems: networking"). This session added **candidate M (Coin race)** and updated that line to "none — every engine subsystem now has at least one playable example."
- `src/network.rs` (568 lines) — the networking subsystem being dogfooded.
- `examples/mp_server.rs` / `examples/mp_client.rs` — the pre-existing Phase 27 *demos* (dumb position relay; the template the new game extends).
- Memory: `playtest-windowed-examples` (updated this session), `ci-toolchain-pin` (`+1.88.0` gate), `conversation-language-korean`, `subagent-usage-preference`, `project-vision`.

## The Goal

Close the networking subsystem's playable-example gap per VISION: build a small, real multiplayer game (goal + win condition, not just a position-echo demo) that exercises `NetworkClient`/`NetworkSystem`/`NetworkEvent` in real play, and fix any engine API gap the example surfaces. Success = the game compiles clean, passes the full engine CI-equivalent gate, AND renders/plays in real play.

## Where We Are

- **Two new examples shipped** under `examples/games/coin_race/`, registered in `Cargo.toml`:
  - `coin_race_server` (`server.rs`, ~290 lines) — **authoritative** game server (raw `tungstenite`, not an engine type). Owns the coin field + scoreboard; resolves contested pickups (first `grab` claim wins). Dependency-free xorshift64 RNG for coin spawns. 5 unit tests.
  - `coin_race_game` (`coin_race.rs`, ~340 lines) — the engine client. `NetworkClient::connect` + `NetworkSystem` + a `CoinRaceSystem` that renders local/remote players + coins, does client-side collision → optimistic `grab`, and draws a scoreboard/status/win banner.
- **No engine source changed.** The networking API carried a full authoritative multiplayer game as-is. The only engine-repo edits are docs (`CLAUDE.md`, `docs/NEXT_WORK.md`, `docs/HANDOFF.md`) + `Cargo.toml` example registration.
- **Engine gate fully green** (toolchain `+1.88.0`, memory `ci-toolchain-pin`): `fmt --check` (stable 1.9.0 AND 1.88.0/1.8.0), `clippy --all-targets -D warnings`, `build --target wasm32` (lib+bins), `test --all-targets` (lib **311** + coin_race_server **5**, 0 failed), `doc -D warnings`.
- **Protocol verified end-to-end** by a throwaway `tungstenite` probe against the real server binary: hello-snapshot, grab→score+respawn, broadcast to peers, **contested-coin rejection** (the authoritative correctness property), position relay, `bye` on disconnect — all 7/7. Probe deleted after use.
- **Live play confirmed by screenshot**: a single client driven by held letter keys collected 10 coins → `Player #1: 10/10`, status "You win! 🏆", big "YOU WIN!" banner. An earlier 2-window shot confirmed remote-player render + position relay (white local + green remote squares + coins + scoreboard).
- **Working tree (uncommitted at handoff time):** `M CLAUDE.md`, `M Cargo.toml`, `M docs/HANDOFF.md`, `M docs/NEXT_WORK.md`, `?? examples/games/coin_race/`. Clean, focused — no stray files (probe removed).
- Engine branch: `main`. Per repo convention, session/example commits go to `main` directly (every prior `session:`/`feat:` commit did).

## What We Tried (Chronological)

1. **Onboarded** — read the launching handoff (code-analysis seq 4), confirmed its chain is CLOSED, narrated the plan, ran the engine baseline gate. All 5 gate commands green; 311 lib tests. Clean baseline.
2. **Dogfooding analysis** — read `src/network.rs` (NetworkClient native=tungstenite thread / wasm=web-sys; NetworkSystem polls client → `Events<NetworkEvent>`; NetworkConfig queue/size caps with backpressure-drop) and `mp_server`/`mp_client` (dumb position relay demos, top-level `examples/`, no game). Identified 4 candidate friction points (see Code Analysis).
3. **Checked native disconnect emission** — confirmed `network.rs:220-250` emits `NetworkEvent::Disconnected` on both remote-Close and socket error (native). This meant the game can show "server down" from events → `is_connected()` (wasm-only) is NOT needed by the example.
4. **AskUserQuestion (game concept)** — offered Coin race (authoritative, recommended) / Tag chase (minimal) / Co-op survival (large). User picked **Coin race (authoritative server)**.
5. **Wrote `server.rs`** — authoritative server: `Arc<Mutex<Server>>` holding players (id→score+sender), coins (id→pos), xorshift RNG, winner. Pre-spawns 6 coins; per-connection thread mirrors `mp_server`'s 5ms-read-timeout loop; `grab` removes coin + scores + respawns + broadcasts; `win` at target 10.
6. **Wrote `coin_race.rs`** — engine client. `CoinRaceSystem` tracks local/remote entities, coins, scores, a `claimed` set (avoid grab spam), send_timer (20Hz pos). Optimistic grab: client sends claim but only removes the coin on the server's `taken` confirmation.
7. **Registered both `[[example]]`** in `Cargo.toml` (`coin_race_game`, `coin_race_server`).
8. **First build failed** — E0507: cannot move `text` out of a pattern guard (`Ok(text) if ws.send(...text.into())`). Rewrote the hello-send as a `let-else` + `if ...is_err()`. Rebuilt clean.
9. **clippy `-D warnings`** on both examples → clean. **`cargo fmt`** wanted reflows (my hand-formatting didn't match rustfmt's wrapping) → ran `cargo +1.88.0 fmt`, verified clean on BOTH stable 1.9.0 and 1.88.0/1.8.0.
10. **Server unit tests** → 5 passed (full coin field on start, spawn within bounds ×1000, monotonic ids, message parse, type-tag serialize).
11. **Headless protocol probe** — no `websocat`/`wscat`/py-websockets available; wrote `examples/zzz_coin_probe.rs` using `tungstenite` (a regular dep) to drive two clients against the running server binary. **7/7 PASS** incl. contested-coin rejection + relay + bye. Deleted the probe.
12. **Full engine gate re-run** with the new examples → all green (fmt both toolchains, clippy all-targets, wasm, test 311+5, doc).
13. **Release build** (`--release --example coin_race_server --example coin_race_game`) → clean (~1m46s, first release compile of engine).
14. **Windowed playtest, attempt 1 (2 windows)** — launched server + 2 clients; both connected (server log `[1]`,`[2]`). Drove the frontmost with **arrow key *codes*** (`key down 124` …) → **no movement** (keys didn't register). Screenshot still confirmed render + multiplayer sync (white local + green remote squares, coins, scoreboard, status).
15. **Diagnosed the input failure** — the `playtest-windowed-examples` memory documents **letter keys held** (`key down "d"`) as the working method; I'd used arrow *codes*. Also: two instances of one binary = one process / two windows → `set frontmost` is ambiguous about which window receives keys.
16. **Windowed playtest, attempt 2 (single client, letter keys)** — server + 1 client; focused by `unix id is $PID` (matching by name fails — per memory); held `key down "d"` etc. → player MOVED. First sweep missed coins (path didn't intersect). Dense lawnmower (go to corner, ~10 passes, small vertical steps) → **collected 10 coins, WON**, "YOU WIN!" banner rendered. Read the screenshot to confirm.
17. **Docs** — `docs/NEXT_WORK.md` candidate M + "networking now covered"; `CLAUDE.md` added the missing networking module-map row (it had none); `docs/HANDOFF.md` completion-table row + status blurb. **Memory** `playtest-windowed-examples` updated (arrow-codes-vs-letter-keys; multiplayer single-client rule).

## Key Decisions

- **Authoritative server, not a dumb relay.** A relay (`mp_client`) syncs positions but can't arbitrate a shared resource. A coin race needs a server that OWNS state, exercising the full request→authoritative-decision→broadcast loop the relay never touched. This is the dogfooding value beyond Phase 27.
- **Did NOT add `NetworkClient::is_connected()` to the engine.** It exists only on wasm (asymmetry with native). But native emits `Disconnected` on remote-close + error, so the example shows connection state from events alone — the example never *hits* the gap. Per VISION ("fix only the gap the example hits"), left it unforced. (If a future example needs a synchronous bool, add a native `Arc<AtomicBool>` set on Connected/Disconnected — small, additive, non-breaking.)
- **Deferred the remote-entity bookkeeping helper.** Both `mp_client` and `coin_race` reimplement `HashMap<network_id, Entity>` spawn/update/despawn inline. A reusable helper is a candidate, but two examples isn't enough signal to fix the abstraction shape — deferred to avoid premature API.
- **Optimistic-but-confirmed grab.** Client detects collision and sends `grab`, but only removes the coin on the server's `taken`. This is what makes contested pickups correct (two clients claim the same coin → only the first scores; the loser's coin vanishes via the same `taken` broadcast). A `claimed` HashSet avoids re-sending grab every frame while standing on a coin awaiting confirmation.
- **Dependency-free xorshift RNG** in the server rather than pulling in `rand` just to scatter coins. Seeded from `SystemTime` nanos (`| 1` so it's never 0).
- **Standalone server binary** (raw `tungstenite`), mirroring `mp_server`. The engine is client-side; the relay/authority server is example infrastructure, not an engine type.
- **New chain, not a continuation.** The code-analysis chain was closed; this is a new initiative it merely pointed to. Logged as `networking-dogfood` seq 1 with the seq-4 handoff as a Related reference.

## Evidence & Data

### Engine gate — final (toolchain 1.88.0, memory `ci-toolchain-pin`)

| Check | Result |
|---|---|
| `cargo +1.88.0 fmt --check` (stable 1.9.0 + 1.88.0/1.8.0) | ✅ both |
| `cargo +1.88.0 clippy --all-targets -- -D warnings` | ✅ |
| `cargo +1.88.0 build --target wasm32-unknown-unknown` (lib+bins) | ✅ |
| `cargo +1.88.0 test --all-targets` | ✅ lib **311** + coin_race_server **5**, 0 failed |
| `RUSTDOCFLAGS="-D warnings" cargo +1.88.0 doc --no-deps` | ✅ |

### Headless protocol probe (throwaway `tungstenite`, vs real server binary)

```
PASS hello: A=#1 target=10 coins=6
PASS hello: B=#2
PASS grab: A scored coin 6, field refilled with coin 7
PASS broadcast: B saw A take coin 6
PASS authority: B's late claim on coin 6 was rejected     <-- authoritative correctness
PASS relay: B received A's position
PASS lifecycle: B saw A's bye
ALL PROBE CHECKS PASSED  (PROBE_EXIT=0)
```
Server log during probe: `[1] connected … [2] connected … [1] disconnected … [2] disconnected`.

### Live playtest

- 2-window shot: white (local) + green (remote #1) squares, 6 gold coins, scoreboard (`▶Player #2: 0/10`, `Player #1: 0/10`), status, controls hint — multiplayer position relay visually confirmed. (Arrow-code input did not move the player.)
- Single-client win shot (`/tmp/coin_grab_shot2.png`, sent to user): status "You win! 🏆", `▶ Player #1: 10/10`, "YOU WIN!" banner, field still full (respawn working). Full client loop confirmed: collision → grab → server score → taken → score++ → respawn → win → banner.

### Server constants (server.rs)

`ADDR=127.0.0.1:9002` · `TARGET_SCORE=10` · `COIN_COUNT=6` · field `x∈[60,740) y∈[60,540)` · `MAX_JSON_MESSAGE_BYTES=4096` · 5ms read timeout.

### Client constants (coin_race.rs)

`SERVER_URL=ws://127.0.0.1:9002` · `PLAYER_SIZE=30` · `COIN_SIZE=20` · `GRAB_RADIUS=(30+20)/2=25` · `MOVE_SPEED=220` · `SPAWN=(400,300)` · pos send 20Hz (0.05s) · 6-color palette by `id % 6`.

### Wire protocol (JSON text — the contract between the two new binaries)

Client → Server:
| Message | Meaning |
|---|---|
| `{"type":"pos","x":<f32>,"y":<f32>}` | my local position (sent at 20 Hz) |
| `{"type":"grab","coin":<N>}` | I claim coin N (server arbitrates) |

Server → Client:
| Message | Meaning |
|---|---|
| `{"type":"hello","id":<N>,"target":<N>,"players":[{"id","score"}],"coins":[{"coin","x","y"}]}` | join snapshot (id + win target + current scoreboard + full coin field) |
| `{"type":"pos","id":<N>,"x":<f32>,"y":<f32>}` | another player's position (relayed to everyone except sender) |
| `{"type":"coin","coin":<N>,"x":<f32>,"y":<f32>}` | a coin (re)spawned |
| `{"type":"taken","coin":<N>,"by":<N>,"score":<N>}` | coin removed; `by` now has total `score` (broadcast to all) |
| `{"type":"win","id":<N>}` | player N reached the target |
| `{"type":"bye","id":<N>}` | player N disconnected |

Notes: `hello` is sent **while holding the server lock** so the snapshot can't race a concurrent `taken`. Existing players learn of a newcomer lazily via that newcomer's first `pos` (client `or_insert`s a score-0 entry); scores are only ever authoritative via `taken`. Both binaries `#[derive(Serialize/Deserialize)]` their own copies of these types — there's no shared crate (each example is self-contained), so a protocol change must be edited in BOTH `server.rs` and `coin_race.rs`.

### Server architecture (server.rs)

- `Shared = Arc<Mutex<Server>>`; `Server { players: HashMap<id, Player{score, mpsc::Sender<Message>}>, coins: HashMap<id,(x,y)>, next_player_id, next_coin_id, winner: Option<id>, rng: Rng }`.
- One thread per connection (`handle_client`): WebSocket with a 5ms read timeout; loop drains the player's `mpsc` outbound queue (relayed frames) then reads one inbound frame (or times out). On disconnect → `cleanup` removes the player + broadcasts `bye`.
- `Server::broadcast(&msg, except: Option<id>)` serializes once, clones the frame to each player's `Sender` (skip `except`). `pos` uses `except=Some(sender)`; `taken`/`coin`/`win`/`bye` use `None` (all).
- Authority lives entirely in `handle_message` under the lock: `grab` does `coins.remove(coin)` (None ⇒ lost the race, ignored) → `score += 1` → broadcast `taken` → `spawn_coin` + broadcast `coin` → `win` at `>= TARGET_SCORE` (set once via `winner`).

## Code Analysis (networking dogfooding findings — for the next networking cycle)

- **`src/network.rs` public surface:** `NetworkClient::{connect, connect_with_config, send_text, send_bytes, try_send_text, try_send_bytes, disconnect}` (native); wasm adds `is_connected()`. `NetworkSystem` (polls `NetworkClient::poll()` → `Events<NetworkEvent>` bus). `NetworkEvent::{Connected, Disconnected{reason}, BinaryMessage, TextMessage, MessageTooLarge, ReceiveQueueFull, JsonParseError, Error}`. `NetworkConfig` (max_message_bytes / max_pending_messages / max_pending_events; SyncSender backpressure drops on full).
- **Setup recipe (proven ergonomic):** `world.insert_resource(NetworkClient::connect(url))` + `systems.push(NetworkSystem)` + `app.register_event::<NetworkEvent>()`. 3 lines.
- **Friction point 1 — `is_connected()` asymmetry** (native lacks it). NOT hit by this example (events suffice). Candidate fix only if a future example needs a synchronous check.
- **Friction point 2 — remote-entity bookkeeping** reimplemented per game. Deferred (insufficient signal).
- **Friction point 3 — wasm client main is empty** in both `mp_client` and `coin_race` (`#[cfg(target_arch="wasm32")] fn main(){}`). The networking code has a wasm path, but no example runs a networked game on wasm. Candidate for a future cycle (would need wasm input/render wiring + a reachable ws server).
- **Non-gap:** server-down detection works via `NetworkEvent::Disconnected` (native emits on Close at `network.rs:220` and on error at `:236`).

## Files Changed (this session)

### Engine (`skeleton-engine`) — to be committed
- **NEW** `examples/games/coin_race/server.rs` — authoritative server (`coin_race_server`), 5 unit tests.
- **NEW** `examples/games/coin_race/coin_race.rs` — engine client (`coin_race_game`).
- `Cargo.toml` — two `[[example]]` registrations.
- `CLAUDE.md` — added the (previously missing) networking module-map row pointing at `src/network.rs` + the demos + coin_race.
- `docs/NEXT_WORK.md` — new "Coverage follow-up — Networking (candidate M)" section; updated the never-in-a-game line to "none".
- `docs/HANDOFF.md` — completion-table row + "Current status" blurb (311 tests, networking covered).
- `plans/handoffs/HANDOFF_networking-dogfood_coin-race-example_2026-06-08.md` — this file.

### Memory (`~/.claude/.../memory/`)
- `playtest-windowed-examples.md` — added: held movement = letter keys not arrow codes; multiplayer = drive a single client (same-process two-window focus is ambiguous); dense lawnmower for random pickups.

### Deleted
- `examples/zzz_coin_probe.rs` — throwaway protocol probe (existed only during verification).

## User Feedback & Preferences

- **"베이스라인부터 진행해"** — greenlit the networking direction; run the baseline gate first.
- AskUserQuestion → **Coin race (authoritative server)** chosen over Tag chase / Co-op survival.
- **`/handoff 하고 커밋 푸쉬`** — create handoff, then commit AND push, close session.
- Standing prefs (memory): conversation in Korean / artifacts in English (`conversation-language-korean`, `doc-language-rule`); use `+1.88.0` for gates (`ci-toolchain-pin`); verify before declaring done (re-ran the real gate after each edit batch); subagents for parallel work (this session was linear single-file authoring, so none were needed).

## Gotchas & Lessons (reusable, cost real time)

- **Synthetic held movement = letter keys (`key down "d"`), NOT arrow key codes.** `key down 124` did not register in winit; `key down "d"` held works (drives `InputState::is_pressed`). The `playtest-windowed-examples` memory already said this — I used arrow codes anyway and wasted a playtest pass. READ the memory's input recipe before driving keys.
- **Multiplayer playtest → drive a SINGLE client.** Two instances of one example = one macOS process, two windows; `set frontmost` can't disambiguate which window gets keys. Verify the input/score loop with 1 client; confirm relay/remote-render separately with a 2-window shot (needs no input).
- **Steering into randomly-placed pickups needs a dense full-field sweep.** Go to a corner first, then ~10 horizontal passes with small (~40px) vertical steps; a single diagonal pass misses.
- **Window focus by `unix id is $PID`, not by process name** (per memory) — name matching fails with duplicate-named processes.
- **No `Date.now()`/`rand` needed for a server** — `SystemTime` nanos seed + xorshift64 is dependency-free and enough for coin scatter.
- **Pattern-guard move (E0507):** `Ok(text) if ws.send(Message::Text(text.into()))` can't move `text` in a guard — use `let-else` + a separate `if …is_err()`.
- **`tungstenite` is a regular dep here** (Cargo.toml:101), so a throwaway protocol probe can be written as a top-level example without adding deps. Delete it after.
- **rustfmt wrapping:** don't hand-format struct-variant fields / long fn signatures / `tq.push(DrawText::new(...))` — let `cargo +1.88.0 fmt` decide; verify on stable too (`ci-toolchain-pin`).

## Where We're Going

The networking playable-example gap is CLOSED. After this commit/push, the `networking-dogfood` chain has no pending step. Possible next initiatives (none scheduled):

1. **wasm networked example** — a coin-race-style game that runs in the browser (the one networking sub-gap left; needs wasm input/render + a reachable ws server). Friction point 3 above.
2. **(If desired) git-tag engine `v4.0.0`** — still untagged (tags are only `v0.3.0`/`v0.4.0`); consumers pin by rev. Trivial; low value. (Carried over from code-analysis seq 4.)
3. **(If desired) reusable remote-entity helper** — only worth it once a 3rd networked example confirms the pattern's shape.
4. **(If desired) organize the ~20 rust-survivors WIP doc files** — user's call (from code-analysis seq 4).
5. **(Optional) repo-wide example Korean→English** comment conversion — only the 5 #27 examples done.

## Risks & Blockers

- **Commit goes to `main` directly** (repo convention; every prior `session:`/`feat:` commit did). `coin_race` is purely additive (new files + example registration + docs) — no engine source touched, so zero risk to existing consumers (`rust-survivors`).
- **No CI run locally for the wasm *example* path** — examples are excluded from the wasm lib+bins gate by design (CLAUDE.md WASM gotcha); the coin_race wasm main is an empty stub, so nothing to break there.
- **Playtest relied on macOS Accessibility for synthetic input** — works locally; not reproducible in headless CI (which is why the protocol probe exists as the deterministic gate).

## Open Questions

- Tag engine `v4.0.0` as a git tag? (Open since code-analysis seq 3; not blocking.)
- Pursue the wasm networked example next, or a different breadth item?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine

# Restore context
cat plans/handoffs/HANDOFF_networking-dogfood_coin-race-example_2026-06-08.md   # this file
git log --oneline -5            # latest = this session's commit (coin_race example)

# Verify the engine gate (memory: ci-toolchain-pin → +1.88.0)
./scripts/verify.sh   # or the 5 commands; 311 lib + 5 coin_race_server tests

# Run the game (server + 2+ client windows)
cargo run --example coin_race_server          # terminal 1
cargo run --example coin_race_game            # terminals 2,3,...

# Re-run the live playtest yourself (memory: playtest-windowed-examples)
#   single client + letter keys held (key down "d"); focus by unix id is $PID; dense lawnmower

# Next action: nothing pending in THIS chain. Pick a new initiative (see "Where We're Going"):
#   wasm networked example, OR engine v4.0.0 git tag, OR a different breadth item.
```

## Session Closed
**Closed at:** 2026-06-08 15:15 KST
**Commit:** `session: coin-race-example [networking-dogfood]` on `main`, pushed to `origin/main`.
**Session status:** Handed off to next session. Chain `networking-dogfood` seq 1 — networking playable-example gap closed; no pending step in this chain.
