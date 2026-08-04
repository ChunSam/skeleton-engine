//! Shared wire protocol + simulation constants for Orbital Dodger.
//!
//! Included by both the server (`server.rs`) and the client (`orbital_dodger.rs`) via
//! `#[path = "protocol.rs"] mod protocol;`, so the two binaries compile their own copy of these
//! structurally-identical types.
//!
//! Unlike `predict_shooter`, there is **no shared `step_*` function**: the server is the sole
//! authority over hazard motion and the client never predicts it — it only *interpolates* the
//! snapshots it receives. That is the whole point of this example (interpolation in isolation,
//! decoupled from prediction), so the integration math lives only in `server.rs`.
//!
//! Each binary uses a subset of these items, so unused-warnings are allowed module-wide.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// WebSocket address (distinct from `coin_race`'s 9002 and `predict_shooter`'s 9003 so all three
/// servers can run at once).
pub const SERVER_ADDR: &str = "127.0.0.1:9004";

/// The address the server actually binds: [`SERVER_ADDR`] unless `ORBITAL_DODGER_ADDR` overrides it.
///
/// Additive — with the variable unset this is byte-identical to the constant, so every documented
/// way of running the example is unchanged. `ORBITAL_DODGER_SELFTEST=1` needs a server of its own on
/// a free port; binding 9004 would collide with a server the user is already running, or in CI with
/// a parallel job. The server `bind`s this with an `expect`, so a collision is a hang-and-fail
/// rather than a skip — which is the reason to make it configurable rather than to retry.
///
/// The windowed client keeps dialling the constant (that is the documented two-terminal flow); the
/// self-test builds its own [`NetworkClient`](engine::NetworkClient) against the port it reserved.
/// Same split as `salvage_run` and `predict_shooter`.
#[cfg(not(target_arch = "wasm32"))]
pub fn server_addr() -> String {
    std::env::var("ORBITAL_DODGER_ADDR").unwrap_or_else(|_| SERVER_ADDR.to_string())
}
/// Fixed server simulation timestep (60 Hz).
pub const FIXED_DT: f32 = 1.0 / 60.0;
/// Snapshots per second the server broadcasts. Deliberately **low** (10 Hz = one snapshot every
/// 100 ms, ⅙ of the sim rate) so client-side interpolation is the only thing that makes hazard
/// motion look smooth — toggle it off in the client (`I`) to see the raw 10 Hz judder.
pub const SNAPSHOT_HZ: u32 = 10;
/// Logical play-field size (matches the client window).
pub const FIELD_W: f32 = 800.0;
pub const FIELD_H: f32 = 600.0;

/// Number of drifting hazards the server simulates.
pub const HAZARD_COUNT: usize = 7;
/// Hazard half-extent (collision radius + half the rendered square).
pub const HAZARD_RADIUS: f32 = 26.0;
/// Hazard linear speed range (px/s).
pub const HAZARD_MIN_SPEED: f32 = 80.0;
pub const HAZARD_MAX_SPEED: f32 = 190.0;
/// Hazard spin range (rad/s); sign is randomized per hazard.
pub const HAZARD_MIN_SPIN: f32 = 1.2;
pub const HAZARD_MAX_SPIN: f32 = 3.4;

/// Player half-extent (collision radius + half the rendered square).
pub const PLAYER_RADIUS: f32 = 12.0;
/// Local player movement speed (px/s). The player is simulated entirely client-side.
pub const PLAYER_SPEED: f32 = 260.0;
/// The vault door: cross this x (right edge) to win.
pub const GOAL_X: f32 = FIELD_W - 48.0;

pub const MAX_JSON_MESSAGE_BYTES: usize = 8192;

/// One server-owned body (a hazard): position + spin angle (radians). `id` is the hazard's stable
/// index, so the client can map it to a persistent entity + interpolation buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyState {
    pub id: usize,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
}

// ── Server → Client ───────────────────────────────────────────────────────────
//
// There is no Client → Server message: the client is a pure receiver. Hazards are wholly
// server-authoritative and the local player never round-trips, so the client sends nothing.

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum ServerMsg {
    /// Sent once on connect with the full hazard set (so the client can spawn every entity up
    /// front instead of waiting for them to appear in `Snap`s).
    #[serde(rename = "hello")]
    Hello { hazards: Vec<BodyState> },
    /// Periodic authoritative snapshot of every hazard's position + angle.
    #[serde(rename = "snap")]
    Snap { tick: u32, hazards: Vec<BodyState> },
}
