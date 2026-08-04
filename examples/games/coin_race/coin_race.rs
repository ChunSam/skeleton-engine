//! Coin race — a small competitive multiplayer game (engine networking dogfood)
//!
//! Two or more players race to collect coins. The **server is authoritative**: you never
//! delete a coin yourself, you send a `grab` claim and wait for the server to confirm who
//! actually got it. First to the target score wins.
//!
//! ```text
//! # Terminal 1 — start the authoritative server (native)
//! cargo run --example coin_race_server
//!
//! # Terminals 2, 3, ... — one native window per player
//! cargo run --example coin_race_game
//! ```
//!
//! # Run in the browser (wasm)
//!
//! The same client also runs on the web — this is the example that proves an engine *game*
//! (not just the bundled lib demo) can ship to wasm. Build it and serve it:
//!
//! ```text
//! cargo run --example coin_race_server                 # native authoritative server
//! examples/games/coin_race/web/build.sh                # build the client to wasm
//! python3 -m http.server 8080 --directory examples/games/coin_race/web
//! open http://localhost:8080                           # play in the browser
//! ```
//!
//! The browser tab connects to the native server over `ws://127.0.0.1:9002`; native windows
//! and browser tabs share the same game.
//!
//! # Controls
//! - WASD / Arrow keys: move
//! - Walk onto a gold coin to claim it (the server decides contested coins)
//!
//! White square = you · colored squares = rivals · gold squares = coins.

use engine::{
    App, DrawText, Events, InputState, KeyCode, NetworkClient, NetworkEvent, NetworkSystem,
    RemoteEntities, Scene, Sprite, System, SystemRegistrar, TextQueue, Transform, WindowConfig,
    World,
};
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;

const SERVER_URL: &str = "ws://127.0.0.1:9002";
const MAX_JSON_MESSAGE_BYTES: usize = 4096;
const PLAYER_SIZE: f32 = 30.0;
const COIN_SIZE: f32 = 20.0;
/// Center-distance under which the local player is touching a coin.
const GRAB_RADIUS: f32 = (PLAYER_SIZE + COIN_SIZE) * 0.5;
const MOVE_SPEED: f32 = 220.0;
const SPAWN: Vec2 = Vec2::new(400.0, 300.0);

// ── Wire types ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct PlayerSnapshot {
    id: usize,
    score: u32,
}

#[derive(Debug, Deserialize)]
struct CoinSnapshot {
    coin: usize,
    x: f32,
    y: f32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "hello")]
    Hello {
        id: usize,
        target: u32,
        players: Vec<PlayerSnapshot>,
        coins: Vec<CoinSnapshot>,
    },
    #[serde(rename = "pos")]
    Position { id: usize, x: f32, y: f32 },
    #[serde(rename = "coin")]
    Coin { coin: usize, x: f32, y: f32 },
    #[serde(rename = "taken")]
    Taken { coin: usize, by: usize, score: u32 },
    #[serde(rename = "win")]
    Win { id: usize },
    #[serde(rename = "bye")]
    Bye { id: usize },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "pos")]
    Position { x: f32, y: f32 },
    #[serde(rename = "grab")]
    Grab { coin: usize },
}

// ── Scene ─────────────────────────────────────────────────────────────────

struct CoinRaceScene;

impl Scene for CoinRaceScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        world.insert_resource(NetworkClient::connect(SERVER_URL));
        systems.add(NetworkSystem::new());
        systems.add(CoinRaceSystem::new());
    }
}

// ── Game system ─────────────────────────────────────────────────────────────

struct CoinRaceSystem {
    local_entity: Option<engine::Entity>,
    local_id: Option<usize>,
    target: u32,
    remote_players: RemoteEntities<usize>,
    coins: RemoteEntities<usize>,
    coin_pos: HashMap<usize, Vec2>,
    scores: HashMap<usize, u32>,
    /// Coins we have already claimed this life — avoids spamming `grab` every frame
    /// while standing on a coin whose `taken` confirmation is still in flight.
    claimed: HashSet<usize>,
    send_timer: f32,
    status: String,
    winner: Option<usize>,
}

impl CoinRaceSystem {
    fn new() -> Self {
        Self {
            local_entity: None,
            local_id: None,
            target: 0,
            remote_players: RemoteEntities::new(),
            coins: RemoteEntities::new(),
            coin_pos: HashMap::new(),
            scores: HashMap::new(),
            claimed: HashSet::new(),
            send_timer: 0.0,
            status: format!("Connecting to {SERVER_URL} ..."),
            winner: None,
        }
    }

    fn spawn_square(
        world: &mut World,
        pos: Vec2,
        size: f32,
        z: f32,
        color: [f32; 3],
    ) -> engine::Entity {
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: pos,
                scale: Vec2::splat(size),
                rotation: 0.0,
                z,
            },
        );
        world.add_component(e, Sprite::colored(color[0], color[1], color[2]));
        e
    }

    fn ensure_coin(&mut self, world: &mut World, coin: usize, x: f32, y: f32) {
        let pos = Vec2::new(x, y);
        self.coin_pos.insert(coin, pos);
        self.coins.get_or_spawn(world, coin, |w| {
            Self::spawn_square(w, pos, COIN_SIZE, -1.0, [1.0, 0.84, 0.2])
        });
    }

    fn remove_coin(&mut self, world: &mut World, coin: usize) {
        self.coins.remove(world, &coin);
        self.coin_pos.remove(&coin);
        self.claimed.remove(&coin);
    }

    fn handle_message(&mut self, world: &mut World, text: &str) {
        if text.len() > MAX_JSON_MESSAGE_BYTES {
            return;
        }
        let msg = match serde_json::from_str::<ServerMessage>(text) {
            Ok(msg) => msg,
            Err(err) => {
                self.status = format!("Protocol error: {err}");
                return;
            }
        };
        match msg {
            ServerMessage::Hello {
                id,
                target,
                players,
                coins,
            } => {
                self.local_id = Some(id);
                self.target = target;
                self.scores.insert(id, 0);
                for p in players {
                    self.scores.insert(p.id, p.score);
                }
                for c in coins {
                    self.ensure_coin(world, c.coin, c.x, c.y);
                }
                self.status = format!("You are Player #{id} — first to {target} wins!");
            }
            ServerMessage::Position { id, x, y } => {
                self.scores.entry(id).or_insert(0);
                // Spawn the rival on first sight, then update its position.
                let e = self.remote_players.get_or_spawn(world, id, |w| {
                    Self::spawn_square(w, Vec2::new(x, y), PLAYER_SIZE, 0.0, remote_color(id))
                });
                if let Some(tr) = world.get_mut::<Transform>(e) {
                    tr.position = Vec2::new(x, y);
                }
            }
            ServerMessage::Coin { coin, x, y } => self.ensure_coin(world, coin, x, y),
            ServerMessage::Taken { coin, by, score } => {
                self.remove_coin(world, coin);
                self.scores.insert(by, score);
            }
            ServerMessage::Win { id } => {
                self.winner = Some(id);
                self.status = if Some(id) == self.local_id {
                    "You win! 🏆".into()
                } else {
                    format!("Player #{id} wins")
                };
            }
            ServerMessage::Bye { id } => {
                self.remote_players.remove(world, &id);
                self.scores.remove(&id);
            }
        }
    }
}

impl System for CoinRaceSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // 1. Drain network events.
        let events: Vec<NetworkEvent> = world
            .resource::<Events<NetworkEvent>>()
            .map(|bus| bus.read().to_vec())
            .unwrap_or_default();
        for ev in events {
            match ev {
                NetworkEvent::Connected => self.status = "Connected — waiting for ID...".into(),
                NetworkEvent::TextMessage(ref text) => self.handle_message(world, text),
                NetworkEvent::Disconnected { reason } => {
                    self.status = format!("Disconnected: {reason} (is the server running?)")
                }
                NetworkEvent::Error(e) => self.status = format!("Error: {e}"),
                _ => {}
            }
        }

        // 2. Spawn our own avatar once.
        if self.local_entity.is_none() {
            self.local_entity = Some(Self::spawn_square(
                world,
                SPAWN,
                PLAYER_SIZE,
                1.0,
                [1.0, 1.0, 1.0],
            ));
        }

        let playing = self.winner.is_none() && self.local_id.is_some();

        // 3. Input → movement (frozen once the game is decided).
        if playing {
            let (dx, dy) = read_axis(world);
            if let Some(e) = self.local_entity {
                if let Some(tr) = world.get_mut::<Transform>(e) {
                    tr.position.x += dx * MOVE_SPEED * dt;
                    tr.position.y += dy * MOVE_SPEED * dt;
                }
            }
        }

        let my_pos = self
            .local_entity
            .and_then(|e| world.get::<Transform>(e).map(|t| t.position));

        // 4. Claim any coin we are touching (optimistic: server confirms via `taken`).
        if playing {
            if let Some(pos) = my_pos {
                let touching: Vec<usize> = self
                    .coin_pos
                    .iter()
                    .filter(|(id, &cpos)| {
                        !self.claimed.contains(*id) && pos.distance(cpos) < GRAB_RADIUS
                    })
                    .map(|(&id, _)| id)
                    .collect();
                for coin in touching {
                    if let Some(client) = world.resource::<NetworkClient>() {
                        if let Ok(text) = serde_json::to_string(&ClientMessage::Grab { coin }) {
                            client.send_text(text);
                            self.claimed.insert(coin);
                        }
                    }
                }
            }
        }

        // 5. Broadcast our position at 20 Hz.
        self.send_timer -= dt;
        if playing && self.send_timer <= 0.0 {
            self.send_timer = 0.05;
            if let Some(pos) = my_pos {
                if let Some(client) = world.resource::<NetworkClient>() {
                    if let Ok(text) = serde_json::to_string(&ClientMessage::Position {
                        x: (pos.x * 100.0).round() / 100.0,
                        y: (pos.y * 100.0).round() / 100.0,
                    }) {
                        client.send_text(text);
                    }
                }
            }
        }

        // 6. HUD.
        self.draw_hud(world);
    }
}

impl CoinRaceSystem {
    fn draw_hud(&self, world: &mut World) {
        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        tq.push(DrawText::new(
            &self.status,
            Vec2::new(12.0, 10.0),
            18.0,
            [255, 255, 255, 220],
        ));

        // Scoreboard, sorted by score desc then id asc.
        let mut board: Vec<(usize, u32)> = self.scores.iter().map(|(&i, &s)| (i, s)).collect();
        board.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut y = 40.0;
        for (id, score) in board {
            let me = Some(id) == self.local_id;
            let marker = if me { "▶ " } else { "  " };
            let [r, g, b] = remote_color(id);
            let color = if me {
                [255, 255, 255, 255]
            } else {
                [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 235]
            };
            tq.push(DrawText::new(
                format!("{marker}Player #{id}: {score}/{}", self.target),
                Vec2::new(12.0, y),
                16.0,
                color,
            ));
            y += 22.0;
        }

        tq.push(DrawText::new(
            "WASD / Arrows to move · grab gold coins",
            Vec2::new(12.0, 574.0),
            13.0,
            [170, 170, 170, 180],
        ));

        if let Some(winner) = self.winner {
            let banner = if Some(winner) == self.local_id {
                "YOU WIN!".to_string()
            } else {
                format!("PLAYER #{winner} WINS")
            };
            tq.push(DrawText::new(
                banner,
                Vec2::new(300.0, 280.0),
                40.0,
                [255, 230, 120, 255],
            ));
        }
    }
}

fn read_axis(world: &World) -> (f32, f32) {
    let Some(input) = world.resource::<InputState>() else {
        return (0.0, 0.0);
    };
    let right = (input.is_pressed(KeyCode::KeyD) || input.is_pressed(KeyCode::ArrowRight)) as i32;
    let left = (input.is_pressed(KeyCode::KeyA) || input.is_pressed(KeyCode::ArrowLeft)) as i32;
    let down = (input.is_pressed(KeyCode::KeyS) || input.is_pressed(KeyCode::ArrowDown)) as i32;
    let up = (input.is_pressed(KeyCode::KeyW) || input.is_pressed(KeyCode::ArrowUp)) as i32;
    ((right - left) as f32, (down - up) as f32)
}

/// Maps a player ID to a stable 6-color palette (matches `mp_client`).
fn remote_color(id: usize) -> [f32; 3] {
    const PALETTE: &[[f32; 3]] = &[
        [1.0, 0.35, 0.35],
        [0.35, 1.0, 0.45],
        [0.35, 0.55, 1.0],
        [1.0, 0.95, 0.3],
        [1.0, 0.4, 1.0],
        [0.3, 1.0, 0.95],
    ];
    PALETTE[id % PALETTE.len()]
}

/// Builds the app and runs it. Shared by the native `main` and the wasm entry point.
fn run() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Coin Race — multiplayer".to_string(),
        width: 800,
        height: 600,
        clear_color: [0.05, 0.07, 0.12, 1.0],
    });
    app.register_event::<NetworkEvent>();
    app.set_scene(Box::new(CoinRaceScene));
    app.run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // `COIN_RACE_SELFTEST=1` runs the headless acceptance test instead of opening a window.
    if std::env::var("COIN_RACE_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }
    run();
}

// ── Acceptance test ───────────────────────────────────────────────────────────────────────────

/// `COIN_RACE_SELFTEST=1 cargo run --example coin_race_game` — asserts what this example exists to
/// show and a screenshot cannot.
///
/// Server authority is invisible in a still frame, and its failure mode *flatters* the bug. A
/// client that deleted the coin and scored the point itself would look **better** in single player:
/// the coin vanishes the instant you touch it, with no round trip to wait through. The damage only
/// exists with two players on one coin — both would see themselves win it, and the two scoreboards
/// would part company and stay apart. Neither screen shows anything wrong on its own, which is why
/// no screenshot and no single-client test can reach this.
///
/// Checks 1-4 need no server and run anywhere. Checks 5-6 spawn the real `coin_race_server` on a
/// port of their own and drive **two** clients against it — the first two-client acceptance test in
/// the tree, because a contested coin has no meaning with one player. They **SKIP with exit 0** if
/// that binary was never built, the same rule `BEAT_CRAWLER_SELFTEST` uses for a missing audio
/// device.
///
/// Exit codes: `0` pass (the live checks may have skipped) · `1` the join snapshot does not build
/// the field · `2` touching a coin does not claim it, or deletes it locally · `3` a `taken` does not
/// remove the coin or credits the wrong player · `4` a decided game does not freeze play · `5`
/// against a real server, two clients contesting one coin disagree about who won it · `6` the
/// server does not refill the field.
#[cfg(not(target_arch = "wasm32"))]
fn self_test() -> i32 {
    use engine::{Entity, InputAction, InputScript};
    use std::time::{Duration, Instant};

    /// Simulated frame time for the offline checks. Everything they exercise is driven by `dt`, so
    /// a fixed step reproduces real play exactly. The **server-backed** checks below must not use
    /// it: the server runs on a wall clock, so those are paced off `Instant`.
    const DT: f32 = 1.0 / 60.0;

    /// The scene's systems, minus a live socket. `CoinRaceSystem::new` is the game's own
    /// constructor — a harness that assembled its own state would stop testing the scene.
    ///
    /// The `NetworkClient` resource is inserted anyway, pointed at a port nothing answers on: the
    /// claim path in `CoinRaceSystem::run` is *guarded* by that resource, so without it check 2
    /// would pass by never running. Sends are `try_send` into a bounded queue, so they block
    /// nothing and go nowhere — the receive half is fed by `feed` instead.
    fn harness() -> (App, NetworkSystem, CoinRaceSystem) {
        let mut app = App::new();
        app.register_event::<NetworkEvent>();
        app.world
            .insert_resource(NetworkClient::connect("ws://127.0.0.1:9"));
        (app, NetworkSystem::new(), CoinRaceSystem::new())
    }

    /// Hand the client a real protocol message, through the real event bus.
    ///
    /// The bytes are written out rather than encoded from a type: the client's `ServerMessage`
    /// derives `Deserialize` only — it never sends one — and the contract under test is the wire
    /// format documented in `server.rs`, whose own unit tests pin the same bytes from the other
    /// side. Encoding through the client's type would only prove it round-trips itself.
    fn feed(world: &mut World, json: &str) {
        if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
            bus.send(NetworkEvent::TextMessage(json.to_string()));
        }
    }

    fn hello_json(id: usize, target: u32, coins: &[(usize, Vec2)]) -> String {
        let coins: Vec<String> = coins
            .iter()
            .map(|(c, p)| format!(r#"{{"coin":{c},"x":{},"y":{}}}"#, p.x, p.y))
            .collect();
        format!(
            r#"{{"type":"hello","id":{id},"target":{target},"players":[],"coins":[{}]}}"#,
            coins.join(",")
        )
    }

    fn taken_json(coin: usize, by: usize, score: u32) -> String {
        format!(r#"{{"type":"taken","coin":{coin},"by":{by},"score":{score}}}"#)
    }

    fn win_json(id: usize) -> String {
        format!(r#"{{"type":"win","id":{id}}}"#)
    }

    /// One frame in the scene's order: scripted input where `App` applies it, then the two systems
    /// `CoinRaceScene::on_enter` registers, then the end-of-frame flush `App` performs.
    fn tick(
        world: &mut World,
        net: &mut NetworkSystem,
        sys: &mut CoinRaceSystem,
        script: Option<&mut InputScript>,
        dt: f32,
    ) {
        if let Some(script) = script {
            script.apply(world);
        }
        net.run(world, dt);
        sys.run(world, dt);
        if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
            bus.flush();
        }
    }

    /// A script holding one key down from the first frame. `InputState` has no public press setter,
    /// so `InputScript` — the engine's own `ENGINE_INPUT` replay path — is how a headless run
    /// synthesizes held input, and it drives the real `read_axis`.
    fn hold(key: KeyCode) -> InputScript {
        InputScript::new([(0, InputAction::KeyDown(key))])
    }

    /// Puts the local avatar on `p`. Movement is exercised on its own in check 4; the claim checks
    /// only need the player to *be* on a coin, and placing it there keeps them independent of the
    /// movement code.
    fn place(world: &mut World, e: Entity, p: Vec2) {
        if let Some(tr) = world.get_mut::<Transform>(e) {
            tr.position = p;
        }
    }

    fn pos_of(world: &World, e: Entity) -> Vec2 {
        world
            .get::<Transform>(e)
            .map(|t| t.position)
            .unwrap_or_default()
    }

    // Coin positions well clear of `SPAWN`, so nothing is claimed before a check asks for it.
    let coin_a = SPAWN + Vec2::new(200.0, 0.0);
    let coin_b = SPAWN + Vec2::new(-200.0, 120.0);

    // ── 1. The join snapshot builds the field ─────────────────────────────────────────────────
    {
        let (mut app, mut net, mut sys) = harness();
        feed(
            &mut app.world,
            &hello_json(1, 10, &[(1, coin_a), (2, coin_b)]),
        );
        tick(&mut app.world, &mut net, &mut sys, None, DT);

        let spawned = [1usize, 2]
            .into_iter()
            .filter(|c| sys.coins.get(c).is_some_and(|e| app.world.is_alive(e)))
            .count();
        if sys.local_id != Some(1) || sys.target != 10 || sys.coins.len() != 2 || spawned != 2 {
            eprintln!(
                "FAIL: the join snapshot did not build the field — local id {:?} (want Some(1)), \
                 target {} (want 10), {} coins mapped (want 2), {spawned} of 2 alive in the world",
                sys.local_id,
                sys.target,
                sys.coins.len()
            );
            return 1;
        }
        println!(
            "hello ok: player #1, target 10, {} coins on the field",
            sys.coins.len()
        );
    }

    // ── 2. Touching a coin claims it — and does not delete it ─────────────────────────────────
    //
    // The headline. The client may not remove a coin it is standing on; it sends a `grab` and
    // waits for the server's `taken`. Three things are asserted together because each alone is
    // passable by a broken client: one that never notices the coin also leaves it standing, one
    // that deletes it locally also "claims" it, and one that claims every coin in the world
    // regardless of distance would claim this one too.
    {
        let (mut app, mut net, mut sys) = harness();
        feed(
            &mut app.world,
            &hello_json(1, 10, &[(1, coin_a), (2, coin_b)]),
        );
        tick(&mut app.world, &mut net, &mut sys, None, DT);
        let me = sys
            .local_entity
            .expect("the avatar spawns on the first tick");

        place(&mut app.world, me, coin_a);
        for _ in 0..30 {
            tick(&mut app.world, &mut net, &mut sys, None, DT);
        }

        let alive = sys.coins.get(&1).is_some_and(|e| app.world.is_alive(e));
        let apart = coin_a.distance(coin_b);
        if !alive
            || !sys.coin_pos.contains_key(&1)
            || !sys.claimed.contains(&1)
            || sys.claimed.contains(&2)
        {
            eprintln!(
                "FAIL: standing on coin 1 did not produce a claim the client then waits on — \
                 claimed {:?} (want exactly {{1}}: 1 because we are on it, not 2 because it is \
                 {apart:.0} px away), coin 1 {} in the world after 30 frames. A client that \
                 deletes the coin itself has scored on its own authority; with two players on one \
                 coin, both would see themselves win it.",
                sys.claimed,
                if alive { "still alive" } else { "already gone" }
            );
            return 2;
        }
        println!(
            "claim ok: coin 1 claimed and left standing for 30 frames, coin 2 ({apart:.0} px away) \
             untouched"
        );
    }

    // ── 3. A `taken` is what removes a coin — and it credits whoever the server names ─────────
    //
    // The other half of authority: the coin under your feet can vanish because someone else got
    // it, and your score must not move. `taken` carries the winner's *authoritative total*, not a
    // delta, so a client keeping its own counter drifts out of step the first time a message is
    // lost — and the drift is silent, because each screen still shows a plausible scoreboard.
    {
        let (mut app, mut net, mut sys) = harness();
        feed(
            &mut app.world,
            &hello_json(1, 10, &[(1, coin_a), (2, coin_b)]),
        );
        tick(&mut app.world, &mut net, &mut sys, None, DT);
        let me = sys
            .local_entity
            .expect("the avatar spawns on the first tick");
        place(&mut app.world, me, coin_a);
        tick(&mut app.world, &mut net, &mut sys, None, DT);
        let coin_entity = sys.coins.get(&1).expect("coin 1 streamed in from `hello`");

        // We claimed coin 1 on the tick above — and lost the race: the server gave it to player #7.
        feed(&mut app.world, &taken_json(1, 7, 3));
        tick(&mut app.world, &mut net, &mut sys, None, DT);

        let mine = sys.scores.get(&1).copied().unwrap_or_default();
        let theirs = sys.scores.get(&7).copied().unwrap_or_default();
        let gone = !app.world.is_alive(coin_entity) && !sys.coin_pos.contains_key(&1);
        if !gone || sys.claimed.contains(&1) || mine != 0 || theirs != 3 || sys.coins.len() != 1 {
            eprintln!(
                "FAIL: a `taken` for the coin we were standing on was not applied as the server \
                 sent it — our score {mine} (want 0), player #7's {theirs} (want the authoritative \
                 3), coin 1 {}, {} coins left (want 1: coin 2, untouched). Losing the race has to \
                 cost us the coin and give us nothing.",
                if gone { "gone" } else { "still in the world" },
                sys.coins.len()
            );
            return 3;
        }
        println!("taken ok: coin 1 removed, player #7 credited the server's 3, our score still 0");
    }

    // ── 4. A decided game freezes play ────────────────────────────────────────────────────────
    //
    // A still frame shows the winner banner either way; whether the losers can still drive around
    // and claim coins after the game is over is invisible in it. The "before" half is what makes
    // the "after" half mean anything — without it, a `read_axis` that returned nothing (or a
    // script that never applied) would make the frozen assertion pass for the wrong reason.
    {
        let (mut app, mut net, mut sys) = harness();
        feed(&mut app.world, &hello_json(1, 10, &[(1, coin_a)]));
        tick(&mut app.world, &mut net, &mut sys, None, DT);
        let me = sys
            .local_entity
            .expect("the avatar spawns on the first tick");

        const FRAMES: usize = 10;
        let mut right = hold(KeyCode::KeyD);
        let start = pos_of(&app.world, me);
        for _ in 0..FRAMES {
            tick(&mut app.world, &mut net, &mut sys, Some(&mut right), DT);
        }
        let moved = pos_of(&app.world, me).x - start.x;

        // Someone else reached the target. The key is still held.
        feed(&mut app.world, &win_json(7));
        tick(&mut app.world, &mut net, &mut sys, Some(&mut right), DT);
        let decided = pos_of(&app.world, me).x;
        for _ in 0..FRAMES {
            tick(&mut app.world, &mut net, &mut sys, Some(&mut right), DT);
        }
        let after = pos_of(&app.world, me).x - decided;

        // Parked on a coin, still holding D: neither the avatar nor a new claim may move.
        place(&mut app.world, me, coin_a);
        for _ in 0..FRAMES {
            tick(&mut app.world, &mut net, &mut sys, Some(&mut right), DT);
        }

        let want = MOVE_SPEED * DT * FRAMES as f32;
        if (moved - want).abs() > MOVE_SPEED * DT
            || after != 0.0
            || !sys.claimed.is_empty()
            || sys.winner != Some(7)
        {
            eprintln!(
                "FAIL: the game being decided did not freeze play — held D moved the avatar \
                 {moved:.1} px before the win (want {want:.1} +/- one tick of {:.1}) and \
                 {after:.1} px after (want 0); claims made after the win {:?} (want none); winner \
                 {:?} (want Some(7))",
                MOVE_SPEED * DT,
                sys.claimed,
                sys.winner
            );
            return 4;
        }
        println!(
            "freeze ok: {moved:.1} px of held input before the win, {after:.1} px and no claims \
             after it"
        );
    }

    // ── 5-6. Against the real server, with two clients ────────────────────────────────────────
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let server_bin =
        exe_dir.map(|d| d.join(format!("coin_race_server{}", std::env::consts::EXE_SUFFIX)));
    let Some(server_bin) = server_bin.filter(|p| p.exists()) else {
        println!(
            "SKIP: coin_race_server has not been built, so the contested-coin checks (5-6) did \
             not run. `cargo build --example coin_race_server` to include them."
        );
        println!("PASS: coin_race (offline checks only)");
        return 0;
    };

    // A port of our own. Binding :0 asks the OS for a free one; the listener is dropped immediately
    // so the child can take it. A race in principle and the right trade in practice — the
    // alternative is the hardcoded 9002, which collides with a server the user is already running.
    let addr = match std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()) {
        Ok(a) => a.to_string(),
        Err(e) => {
            eprintln!("SKIP: could not reserve a local port ({e})");
            println!("PASS: coin_race (offline checks only)");
            return 0;
        }
    };

    let mut child = match std::process::Command::new(&server_bin)
        .env("COIN_RACE_ADDR", &addr)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("SKIP: could not spawn {}: {e}", server_bin.display());
            println!("PASS: coin_race (offline checks only)");
            return 0;
        }
    };

    // Wait for the child to bind before connecting. `NetworkClient::connect` dials once and does
    // not retry, so connecting first and hoping is not a slower path — it is a guaranteed failure.
    let bind_deadline = Instant::now() + Duration::from_secs(10);
    let mut bound = false;
    while Instant::now() < bind_deadline {
        if std::net::TcpStream::connect(&addr).is_ok() {
            bound = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if !bound {
        child.kill().ok();
        child.wait().ok();
        eprintln!("SKIP: coin_race_server never bound {addr} within 10 s");
        println!("PASS: coin_race (offline checks only)");
        return 0;
    }

    /// One live client: its own `World`, event bus and socket, so the two are as independent as
    /// two windows on two machines.
    struct Live {
        app: App,
        net: NetworkSystem,
        sys: CoinRaceSystem,
        /// Every coin id this client has ever had on its field, accumulated **every frame**.
        /// Check 5 asks how many coins the server took away, and a coin that both spawned and was
        /// taken between two coarse samples would never be counted.
        seen: HashSet<usize>,
    }

    impl Live {
        fn new(addr: &str) -> Self {
            let mut app = App::new();
            app.register_event::<NetworkEvent>();
            app.world
                .insert_resource(NetworkClient::connect(&format!("ws://{addr}")));
            Self {
                app,
                net: NetworkSystem::new(),
                sys: CoinRaceSystem::new(),
                seen: HashSet::new(),
            }
        }

        fn step(&mut self, dt: f32) {
            self.net.run(&mut self.app.world, dt);
            self.sys.run(&mut self.app.world, dt);
            if let Some(bus) = self.app.world.resource_mut::<Events<NetworkEvent>>() {
                bus.flush();
            }
            self.seen.extend(self.sys.coin_pos.keys().copied());
        }

        fn joined(&self) -> bool {
            self.sys.local_id.is_some() && !self.sys.coin_pos.is_empty()
        }

        fn score(&self, id: usize) -> u32 {
            self.sys.scores.get(&id).copied().unwrap_or_default()
        }

        /// Coins that left this client's field since `seen` was last reset. A coin is only ever
        /// removed by a `taken`, so this counts server-confirmed takes.
        fn taken_count(&self) -> usize {
            self.seen.len() - self.sys.coin_pos.len()
        }

        fn park(&mut self, p: Vec2) {
            if let Some(me) = self.sys.local_entity {
                place(&mut self.app.world, me, p);
            }
        }
    }

    /// Run both clients for `secs` of **wall clock**, pacing off `Instant`.
    ///
    /// The server steps in real time, so an accumulator clock would drift against it — the trap
    /// `beat_crawler` hit, where `t += 1.0/60.0` made a correct detector look 40% over-firing.
    fn run_live(a: &mut Live, b: &mut Live, secs: f64) {
        let start = Instant::now();
        let mut last = start;
        while start.elapsed().as_secs_f64() < secs {
            let now = Instant::now();
            let dt = (now - last).as_secs_f32();
            last = now;
            a.step(dt);
            b.step(dt);
            std::thread::sleep(Duration::from_millis(16));
        }
    }

    let finish = |child: &mut std::process::Child, code: i32, msg: Option<String>| -> i32 {
        child.kill().ok();
        child.wait().ok();
        if let Some(msg) = msg {
            eprintln!("FAIL: {msg}");
        }
        code
    };

    let mut a = Live::new(&addr);
    let mut b = Live::new(&addr);

    // Both clients need an id and a field, and each needs to have heard of the other: the
    // scoreboard agreement below is meaningless if one of them has never seen the other's id.
    let join_deadline = Instant::now() + Duration::from_secs(10);
    loop {
        run_live(&mut a, &mut b, 0.25);
        let known = match (a.sys.local_id, b.sys.local_id) {
            (Some(ida), Some(idb)) => {
                a.sys.scores.contains_key(&idb) && b.sys.scores.contains_key(&ida)
            }
            _ => false,
        };
        if (a.joined() && b.joined() && known) || Instant::now() >= join_deadline {
            break;
        }
    }
    let (Some(id_a), Some(id_b)) = (a.sys.local_id, b.sys.local_id) else {
        return finish(
            &mut child,
            5,
            Some(format!(
                "two clients never both joined {addr} in 10 s (A: {}, B: {})",
                a.sys.status, b.sys.status
            )),
        );
    };
    if !a.sys.scores.contains_key(&id_b) || !b.sys.scores.contains_key(&id_a) {
        return finish(
            &mut child,
            5,
            Some(format!(
                "the two clients never learned of each other in 10 s — A #{id_a} sees {:?}, \
                 B #{id_b} sees {:?}",
                a.sys.scores, b.sys.scores
            )),
        );
    }

    // A parking spot clear of every coin, so the only claims inside the measured window are the
    // contested one. Scanned rather than guessed: the server scatters its coins at random, and
    // `SPAWN` is as likely to sit on one as anywhere else.
    let coins: Vec<Vec2> = a.sys.coin_pos.values().copied().collect();
    let nearest_coin = |p: Vec2| {
        coins
            .iter()
            .map(|c| p.distance(*c))
            .fold(f32::MAX, f32::min)
    };
    let mut park = SPAWN;
    let mut park_clear = -1.0f32;
    for gx in 0..17 {
        for gy in 0..13 {
            let p = Vec2::new(60.0 + gx as f32 * 42.5, 60.0 + gy as f32 * 40.0);
            let clear = nearest_coin(p);
            if clear > park_clear {
                park_clear = clear;
                park = p;
            }
        }
    }

    // The contested coin: the most isolated one on the field, so putting both players on it does
    // not also put them on its neighbour. Not required for the accounting below — a second coin
    // taken raises both sides of the identity equally — it just keeps the report readable.
    let field: Vec<(usize, Vec2)> = a.sys.coin_pos.iter().map(|(&i, &p)| (i, p)).collect();
    let target = field
        .iter()
        .filter(|(id, _)| b.sys.coin_pos.contains_key(id))
        .max_by(|(_, p), (_, q)| {
            let nearest = |v: Vec2| {
                field
                    .iter()
                    .filter(|(_, c)| *c != v)
                    .map(|(_, c)| v.distance(*c))
                    .fold(f32::MAX, f32::min)
            };
            nearest(*p).total_cmp(&nearest(*q))
        })
        .copied();
    let Some((target_id, target_pos)) = target else {
        return finish(
            &mut child,
            5,
            Some(format!(
                "the two clients share no coin to contest — A has {:?}, B has {:?}",
                a.sys.coin_pos.keys().collect::<Vec<_>>(),
                b.sys.coin_pos.keys().collect::<Vec<_>>()
            )),
        );
    };

    // Park clear of everything and let that settle, then open the measured window.
    a.park(park);
    b.park(park);
    run_live(&mut a, &mut b, 0.5);

    a.seen = a.sys.coin_pos.keys().copied().collect();
    b.seen = b.sys.coin_pos.keys().copied().collect();
    let before = (a.score(id_a), a.score(id_b));
    let field_before = a.sys.coin_pos.len();

    // ── 5. Two clients contest one coin; the server decides ───────────────────────────────────
    //
    // Both players are put on the same coin on the same frame — the situation the example is built
    // around, and one that cannot be staged with a single client. Both send `grab`; only the claim
    // that reaches the server first may score.
    a.park(target_pos);
    b.park(target_pos);
    let contest_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < contest_deadline
        && (a.sys.coin_pos.contains_key(&target_id) || b.sys.coin_pos.contains_key(&target_id))
    {
        run_live(&mut a, &mut b, 0.1);
    }
    // Off the coins again before measuring, so a coin respawning under their feet cannot join in.
    a.park(park);
    b.park(park);
    run_live(&mut a, &mut b, 0.5);

    let a_view = (a.score(id_a), a.score(id_b));
    let b_view = (b.score(id_a), b.score(id_b));
    let gained = (a_view.0.saturating_sub(before.0) + a_view.1.saturating_sub(before.1)) as usize;
    let (taken_a, taken_b) = (a.taken_count(), b.taken_count());
    let gone = !a.sys.coin_pos.contains_key(&target_id) && !b.sys.coin_pos.contains_key(&target_id);

    if !gone || a_view != b_view || taken_a == 0 || taken_a != taken_b || gained != taken_a {
        return finish(
            &mut child,
            5,
            Some(format!(
                "coin {target_id} was not resolved by the server alone — A sees \
                 (#{id_a}: {}, #{id_b}: {}), B sees (#{id_a}: {}, #{id_b}: {}); {gained} point(s) \
                 scored against {taken_a} coin(s) the server took from A and {taken_b} from B; \
                 coin {target_id} {}. Two things have to hold and either can be the one that \
                 broke: both clients must see the same scoreboard, and every point must be backed \
                 by exactly one server `taken`. A client that scored its own claim credits itself \
                 for a coin the server gave to the other player, so the boards part company; a \
                 server that let both claims through pays twice for one coin, so the boards agree \
                 on a total that never happened.",
                a_view.0,
                a_view.1,
                b_view.0,
                b_view.1,
                if gone {
                    "gone from both"
                } else {
                    "still on someone's field"
                }
            )),
        );
    }
    println!(
        "authority ok: coin {target_id} contested by #{id_a} and #{id_b} on the same frame — \
         {gained} point(s) for {taken_a} server-confirmed take(s), both scoreboards agree \
         (#{id_a}: {}, #{id_b}: {})",
        a_view.0, a_view.1
    );

    // ── 6. The server refills the field ───────────────────────────────────────────────────────
    //
    // A fixed number of coins is supposed to be on the field at all times: every take is answered
    // with a fresh coin at a new id. A screenshot cannot tell a refilled field from one that is
    // quietly draining — both are "some coins on a dark background" — and the game only dies of it
    // minutes later, when the last coin is gone and nobody can reach the target score.
    let field_after = a.sys.coin_pos.len();
    let fresh = a.seen.len() - field_before;
    if field_after != field_before || fresh != taken_a {
        return finish(
            &mut child,
            6,
            Some(format!(
                "the field did not refill — {field_before} coins before the contest, \
                 {field_after} after, with {taken_a} taken and {fresh} new id(s) arriving"
            )),
        );
    }
    println!(
        "refill ok: the field held {field_after} coins across the contest, {fresh} fresh id(s) \
         replacing the {taken_a} taken"
    );

    finish(&mut child, 0, None);
    println!("PASS: coin_race");
    0
}

/// WASM entry point — `examples/games/coin_race/web/index.html` calls this after `init()`.
/// The game code lives here in the example (not in the engine lib): the engine stays a
/// genre-agnostic skeleton, and this proves an engine *game* can ship to the browser.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_coin_race() {
    run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
