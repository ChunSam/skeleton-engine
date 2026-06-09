//! Shared wire protocol + simulation constants for Salvage Run.
//!
//! Included by both the server (`server.rs`) and the client (`salvage_run.rs`) via
//! `#[path = "protocol.rs"] mod protocol;`, so the two binaries compile their own copy of these
//! structurally-identical types.
//!
//! Salvage Run is an **area-of-interest (AOI) streaming** world. A single ship roams a world far
//! larger than the window; the server owns ~120 wandering entities of two kinds and sends each
//! client **only** the entities within an interest radius of that client's last-reported position.
//! So, unlike the broadcast-only `orbital_dodger`, here the client *does* talk back: it reports its
//! position + desired AOI radius (`ClientMsg::Move`) and the server tailors each `Snap` per client.
//! Entities stream in and out as the player moves; the client interpolates their motion
//! (`engine::SnapshotBuffer<Vec2>`) and evicts ones that fall out of the AOI by last-seen timeout.
//!
//! Each binary uses a subset of these items, so unused-warnings are allowed module-wide.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// WebSocket address (distinct from `coin_race`'s 9002, `predict_shooter`'s 9003 and
/// `orbital_dodger`'s 9004 so all four servers can run at once).
pub const SERVER_ADDR: &str = "127.0.0.1:9005";
/// Fixed server simulation timestep (60 Hz).
pub const FIXED_DT: f32 = 1.0 / 60.0;
/// AOI snapshots per second the server broadcasts. Deliberately **low** (12 Hz) so client-side
/// interpolation (`SnapshotBuffer`) is what makes the streamed motion look smooth.
pub const SNAPSHOT_HZ: u32 = 12;

/// Logical world size — much larger than the window, so the camera scrolls and entities stream.
pub const WORLD_W: f32 = 2400.0;
pub const WORLD_H: f32 = 1800.0;
/// Client window / viewport size.
pub const VIEW_W: f32 = 800.0;
pub const VIEW_H: f32 = 600.0;

/// Number of server-simulated entities, split across the two kinds.
pub const ENTITY_COUNT: usize = 120;
pub const SALVAGE_COUNT: usize = 80; // kind 0: ids 0..SALVAGE_COUNT
pub const DRONE_COUNT: usize = 40; // kind 1: ids SALVAGE_COUNT..ENTITY_COUNT

/// Area-of-interest radius (px). Client-tunable live; the server clamps the requested value into
/// `[MIN, MAX]`. Widening it streams more entities in on the next snapshot — interest management
/// made visible.
pub const AOI_RADIUS_DEFAULT: f32 = 520.0;
pub const AOI_RADIUS_MIN: f32 = 200.0;
pub const AOI_RADIUS_MAX: f32 = 1100.0;
pub const AOI_RADIUS_STEP: f32 = 60.0;

/// Collision / render radii per kind (also half the rendered square).
pub const SALVAGE_RADIUS: f32 = 12.0;
pub const DRONE_RADIUS: f32 = 16.0;
pub const PLAYER_RADIUS: f32 = 14.0;

/// Local player speed (px/s). The player is simulated entirely client-side.
pub const PLAYER_SPEED: f32 = 320.0;
/// Salvage drifts very slowly (near-static collectibles you must drive to); drones roam faster
/// (px/s). The drones' speed is what makes the low-rate interpolation visible.
pub const SALVAGE_MIN_SPEED: f32 = 3.0;
pub const SALVAGE_MAX_SPEED: f32 = 11.0;
pub const DRONE_MIN_SPEED: f32 = 70.0;
pub const DRONE_MAX_SPEED: f32 = 150.0;

/// Collect this many salvage to win.
pub const COLLECT_GOAL: u32 = 12;
/// How often the client reports its AOI center to the server (Hz). ~ the snapshot rate; throttled so
/// continuous 60 fps movement doesn't flood the server with position updates.
pub const MOVE_SEND_HZ: f32 = 15.0;

pub const MAX_JSON_MESSAGE_BYTES: usize = 8192;

/// The two entity classes. A small enum (rather than a bare `u8`) is the idiomatic "typed entity"
/// signal: it round-trips by name on the wire and lets the client branch + colour by kind. This is
/// the concrete datapoint for `docs/REMOTE_ENTITIES_DESIGN.md` open question #4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Salvage,
    Drone,
}

/// One server-owned entity in an AOI snapshot: a stable `id` (its index in the server's entity
/// vector), its `kind`, and its position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    pub id: usize,
    pub kind: Kind,
    pub x: f32,
    pub y: f32,
}

// ── Client → Server ─────────────────────────────────────────────────────────────
//
// The client reports where it is (the AOI centre) and how big an AOI it wants, so the server can
// tailor each snapshot. This is the key difference from `orbital_dodger` (which sends nothing).

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum ClientMsg {
    /// "My ship is here; stream me entities within radius `r`." Sent throttled to `MOVE_SEND_HZ`,
    /// plus immediately when the player resizes the AOI.
    #[serde(rename = "move")]
    Move { x: f32, y: f32, r: f32 },
}

// ── Server → Client ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum ServerMsg {
    /// Sent once on connect: the world size (for the camera bounds) and the total entity count (so
    /// the client can render a "streaming X / total" readout without hardcoding it).
    #[serde(rename = "welcome")]
    Welcome {
        world_w: f32,
        world_h: f32,
        total: usize,
    },
    /// Periodic per-client snapshot of **only** the entities within this client's AOI.
    #[serde(rename = "snap")]
    Snap {
        tick: u32,
        entities: Vec<EntityState>,
    },
}
