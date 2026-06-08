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
    App, DrawText, Events, InputState, KeyCode, NetworkClient, NetworkEvent, NetworkSystem, Scene,
    Sprite, System, TextQueue, Transform, WindowConfig, World,
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
    fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>) {
        world.insert_resource(NetworkClient::connect(SERVER_URL));
        systems.push(Box::new(NetworkSystem));
        systems.push(Box::new(CoinRaceSystem::new()));
    }
}

// ── Game system ─────────────────────────────────────────────────────────────

struct CoinRaceSystem {
    local_entity: Option<engine::Entity>,
    local_id: Option<usize>,
    target: u32,
    remote_players: HashMap<usize, engine::Entity>,
    coins: HashMap<usize, engine::Entity>,
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
            remote_players: HashMap::new(),
            coins: HashMap::new(),
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
        self.coins
            .entry(coin)
            .or_insert_with(|| Self::spawn_square(world, pos, COIN_SIZE, -1.0, [1.0, 0.84, 0.2]));
    }

    fn remove_coin(&mut self, world: &mut World, coin: usize) {
        if let Some(e) = self.coins.remove(&coin) {
            world.despawn(e);
        }
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
                if let Some(&e) = self.remote_players.get(&id) {
                    if let Some(tr) = world.get_mut::<Transform>(e) {
                        tr.position = Vec2::new(x, y);
                    }
                } else {
                    let color = remote_color(id);
                    let e = Self::spawn_square(world, Vec2::new(x, y), PLAYER_SIZE, 0.0, color);
                    self.remote_players.insert(id, e);
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
                if let Some(e) = self.remote_players.remove(&id) {
                    world.despawn(e);
                }
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
    run();
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
