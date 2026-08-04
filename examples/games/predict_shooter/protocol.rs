//! Shared wire protocol + deterministic simulation constants for the client-prediction shooter.
//!
//! Included by both the server (`server.rs`) and the client (`predict_shooter.rs`) via
//! `#[path = "protocol.rs"] mod protocol;`, so the two binaries compile their own copy of these
//! structurally-identical types and — crucially — the **same** movement integration. Client-side
//! prediction only converges if the client replays inputs with the exact formula the server uses,
//! so `step_position` lives here and is the single source of truth.
//!
//! Each binary uses a subset of these items, so unused-warnings are allowed module-wide.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// WebSocket address (distinct from `coin_race`'s 9002 so both can run at once).
pub const SERVER_ADDR: &str = "127.0.0.1:9003";

/// The address the server actually binds: [`SERVER_ADDR`] unless `PREDICT_SHOOTER_ADDR` overrides it.
///
/// Additive — with the variable unset this is byte-identical to the constant, so every documented
/// way of running the example is unchanged. It exists because the port was hardcoded:
/// `PREDICT_SHOOTER_SELFTEST=1` needs a server of its own on a free port, and binding 9003 would
/// either collide with a server the user is already running or, in CI, with a parallel job. The
/// server `bind`s this and panics on failure, so a collision is a hang-and-fail rather than a skip —
/// which is the reason to make it configurable rather than to retry.
///
/// The windowed client keeps dialling the constant: it is the documented two-terminal flow, and the
/// self-test builds its own [`NetworkClient`](engine::NetworkClient) against the port it reserved.
/// Same split as `salvage_run`.
#[cfg(not(target_arch = "wasm32"))]
pub fn server_addr() -> String {
    std::env::var("PREDICT_SHOOTER_ADDR").unwrap_or_else(|_| SERVER_ADDR.to_string())
}
/// Fixed simulation timestep. Client and server both integrate at this step.
pub const FIXED_DT: f32 = 1.0 / 60.0;
/// Player movement speed (px/s).
pub const MOVE_SPEED: f32 = 240.0;
/// Logical play-field size (matches the client window).
pub const FIELD_W: f32 = 800.0;
pub const FIELD_H: f32 = 600.0;
/// Half-extent of a player/bullet square, used to clamp inside the field.
pub const PLAYER_HALF: f32 = 16.0;
pub const BULLET_SPEED: f32 = 520.0;
pub const BULLET_LIFE: f32 = 1.2;
pub const FIRE_COOLDOWN: f32 = 0.25;
/// Snapshots per second the server broadcasts (deliberately well below the 60 Hz sim so remote
/// interpolation is actually exercised).
pub const SNAPSHOT_HZ: u32 = 30;
pub const MAX_JSON_MESSAGE_BYTES: usize = 8192;

/// Deterministic movement integration for one input command — **identical** on the client
/// (prediction + replay) and the server (authority). Diagonal input is normalized so diagonal
/// movement isn't faster; the result is clamped inside the field.
pub fn step_position(x: f32, y: f32, mx: f32, my: f32) -> (f32, f32) {
    let len_sq = mx * mx + my * my;
    let (dx, dy) = if len_sq > 1.0 {
        let inv = 1.0 / len_sq.sqrt();
        (mx * inv, my * inv)
    } else {
        (mx, my)
    };
    let nx = (x + dx * MOVE_SPEED * FIXED_DT).clamp(PLAYER_HALF, FIELD_W - PLAYER_HALF);
    let ny = (y + dy * MOVE_SPEED * FIXED_DT).clamp(PLAYER_HALF, FIELD_H - PLAYER_HALF);
    (nx, ny)
}

// ── Client → Server ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum ClientMsg {
    /// One input command per client tick. `seq` is monotonic; `mx`/`my` ∈ [-1,1] movement intent;
    /// `fire` requests a shot (rate-limited server-side).
    #[serde(rename = "in")]
    Input {
        seq: u32,
        mx: f32,
        my: f32,
        fire: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: usize,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletState {
    pub id: usize,
    pub x: f32,
    pub y: f32,
}

// ── Server → Client ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum ServerMsg {
    /// Sent once on connect with the client's assigned id.
    #[serde(rename = "welcome")]
    Welcome { id: usize },
    /// Periodic authoritative snapshot. `ack` is the last input `seq` the server has applied **for
    /// the receiving client** — the client reconciles by snapping to its state here and replaying
    /// inputs newer than `ack`.
    #[serde(rename = "snap")]
    Snap {
        tick: u32,
        players: Vec<PlayerState>,
        bullets: Vec<BulletState>,
        ack: u32,
    },
    /// A player disconnected.
    #[serde(rename = "bye")]
    Bye { id: usize },
}
