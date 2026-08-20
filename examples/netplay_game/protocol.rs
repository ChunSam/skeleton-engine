//! Shared wire protocol + simulation constants for Netplay Salvage.
//!
//! Included by both binaries via `#[path = "protocol.rs"] mod protocol;`, so the client
//! (`netplay_game.rs`) and the server (`server.rs`) each compile their own copy of these
//! structurally-identical types. That duplication is the point: the two are separate processes and
//! a shared crate would let a change to one silently redefine the other's wire format.
//!
//! # One world, four techniques
//!
//! Phase 5 of `plans/2026-08-19-examples-rebuild-plan.md` folds the four deleted networked games
//! into one. They differed by *which* netcode technique they demonstrated, not by genre, so one
//! world carries all four — and each is placed on a different kind of object, because the whole
//! difficulty of netcode is that **the right technique depends on who owns the object**:
//!
//! | Object | Owner | Technique | Deleted game it replaces |
//! |---|---|---|---|
//! | your ship | server, **predicted** locally | prediction + reconciliation | `predict_shooter` |
//! | other ships | server | interpolation | `predict_shooter` (remotes) |
//! | hazard drones | server | interpolation only | `orbital_dodger` |
//! | salvage pickups | server, **claimed** by clients | claim → confirm | `coin_race` |
//! | *the set that is sent at all* | server, per client | AOI + removal-by-omission | `salvage_run` |
//!
//! Any one of them missing is a visibly different game, which is what makes one game an honest
//! replacement for four rather than a merge that drops coverage.
//!
//! # The two rules that make prediction reproducible
//!
//! 1. **[`step_position`] is the single authority.** The client applies it to predict, the client
//!    applies it again to replay during reconciliation, and the server applies it to decide truth.
//!    Three call sites, one function — if the client and server stepped differently, prediction
//!    would diverge from the server *by construction* and reconciliation would rubber-band forever
//!    while every unit test passed.
//! 2. **Input is a discrete tick, not a frame.** A frame is however long the GPU took; an input is
//!    exactly [`INPUT_DT`]. The client sends one input per tick at [`INPUT_HZ`] and predicts
//!    exactly that step, so "replay the unacked inputs" reproduces the same positions the server
//!    will compute. Predicting per *frame* against a server stepping per *input* is the same
//!    divergence as rule 1, wearing a clock.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// WebSocket address. `9006` is deliberately clear of the deleted networked games' ports
/// (`coin_race` 9002, `predict_shooter` 9003, `orbital_dodger` 9004, `salvage_run` 9005) — those
/// examples are gone, but a fork that restored one from `git show 4edfd3f^:` should still be able
/// to run it beside this.
pub const SERVER_ADDR: &str = "127.0.0.1:9006";

/// The address both binaries actually use: [`SERVER_ADDR`] unless `NETPLAY_ADDR` overrides it.
///
/// Additive — with the variable unset this is byte-identical to the constant, so every documented
/// way of running the example is unchanged. It exists because the selftest needs a server of its
/// own on a free port: binding the constant would collide with a server the user is already
/// running or, in CI, with a parallel job. The server `bind`s this and panics on failure, so a
/// collision is a loud failure rather than a silent fallback.
#[cfg(not(target_arch = "wasm32"))]
pub fn server_addr() -> String {
    std::env::var("NETPLAY_ADDR").unwrap_or_else(|_| SERVER_ADDR.to_string())
}

/// The web build has no environment to read, so it always uses the constant.
#[cfg(target_arch = "wasm32")]
pub fn server_addr() -> String {
    SERVER_ADDR.to_string()
}

// ── Simulation constants ────────────────────────────────────────────────────────────────────────

/// Server simulation timestep (60 Hz) — hazard drift only. Player motion advances per *input*
/// ([`INPUT_DT`]), never per server tick, so a client that predicts N inputs and a server that
/// applies the same N inputs land on the same position regardless of how the two clocks interleave.
pub const FIXED_DT: f32 = 1.0 / 60.0;

/// Snapshots per second the server sends each client. Deliberately **low** so client-side
/// interpolation is what makes streamed motion look smooth — at 60 Hz a client that never
/// interpolated would look fine, and check 4 would have nothing to measure.
pub const SNAPSHOT_HZ: u32 = 12;

/// Input ticks per second the client sends. Higher than the snapshot rate: inputs are small and
/// the responsiveness of the local ship is what prediction exists to protect.
pub const INPUT_HZ: u32 = 30;
/// Seconds of motion one input represents. See rule 2 in the module docs.
pub const INPUT_DT: f32 = 1.0 / INPUT_HZ as f32;

/// Logical world size — much larger than the window, so the camera scrolls and entities stream.
pub const WORLD_W: f32 = 2400.0;
pub const WORLD_H: f32 = 1800.0;
/// Client window / viewport size.
pub const VIEW_W: f32 = 1024.0;
pub const VIEW_H: f32 = 640.0;

/// Server-owned salvage pickups (claimable) and hazard drones (deadly, never claimable).
pub const PICKUP_COUNT: u32 = 60;
pub const HAZARD_COUNT: u32 = 40;

/// Area-of-interest radius (px). Client-tunable live; the server clamps into `[MIN, MAX]`.
/// Widening it streams more entities in on the next snapshot — interest management made visible.
pub const AOI_RADIUS_DEFAULT: f32 = 520.0;
pub const AOI_RADIUS_MIN: f32 = 200.0;
pub const AOI_RADIUS_MAX: f32 = 1100.0;
pub const AOI_RADIUS_STEP: f32 = 60.0;

/// How long a tracked entity may go unmentioned before the client evicts it.
///
/// The server never says an entity *left* — it stops including it. So this timeout is the only
/// thing that removes anything, and its value is a real trade: below ~2 snapshot intervals a
/// single dropped packet evicts entities that never left (a visible flicker), and far above it
/// the world fills with ghosts. Three intervals is the smallest value with a packet of slack.
pub const AOI_EVICT_SECS: f64 = 3.0 / SNAPSHOT_HZ as f64;

/// Render-time delay for interpolation: sample the snapshot buffer this far in the past so
/// playback always has two real samples to interpolate *between*. Slightly over one snapshot
/// interval, so a late packet does not leave the buffer momentarily extrapolating.
pub const INTERP_DELAY: f64 = 1.5 / SNAPSHOT_HZ as f64;

/// Collision / render radii (also half the rendered square).
pub const PLAYER_RADIUS: f32 = 14.0;
pub const PICKUP_RADIUS: f32 = 10.0;
pub const HAZARD_RADIUS: f32 = 16.0;

/// Ship speed (px/s), applied by [`step_position`] on both sides.
pub const PLAYER_SPEED: f32 = 320.0;

/// Hazard drift speeds (px/s). Fast enough that 12 Hz snapshots are visibly steppy without
/// interpolation — which is exactly what check 4 measures.
pub const HAZARD_MIN_SPEED: f32 = 70.0;
pub const HAZARD_MAX_SPEED: f32 = 150.0;

/// Collect this many pickups to win.
pub const COLLECT_GOAL: u32 = 12;

/// A claim is only sent when the ship is within this distance of a pickup's **displayed**
/// position. The server re-checks against its own authority with a slack margin, so a client
/// cannot claim a pickup across the map.
pub const CLAIM_RADIUS: f32 = PLAYER_RADIUS + PICKUP_RADIUS;
/// The server's tolerance when validating a claim. Generous, because the client legitimately acts
/// on an interpolated position that is up to [`INTERP_DELAY`] old, plus its own prediction error.
/// Tight enough that a claim from across the world is still rejected.
pub const CLAIM_SLACK: f32 = 96.0;

/// Inbound message cap on the server side. Anything larger is dropped unread — a client cannot
/// make the server allocate by describing a huge message. (`NetworkConfig::max_message_bytes`
/// is the client-side half of the same policy.)
pub const MAX_MESSAGE_BYTES: usize = 16384;

// ── The shared step ─────────────────────────────────────────────────────────────────────────────

/// Applies one input tick of movement, clamped inside the world. **The single authority for player
/// motion** — see rule 1 in the module docs.
///
/// `(mx, my)` is a direction, not a velocity: it is normalized here, so a diagonal is not faster
/// than an axis and a client that sends an over-long vector gains nothing.
pub fn step_position(x: f32, y: f32, mx: f32, my: f32) -> (f32, f32) {
    let len = (mx * mx + my * my).sqrt();
    let (dx, dy) = if len > 1e-6 {
        (mx / len, my / len)
    } else {
        (0.0, 0.0)
    };
    let nx = (x + dx * PLAYER_SPEED * INPUT_DT).clamp(PLAYER_RADIUS, WORLD_W - PLAYER_RADIUS);
    let ny = (y + dy * PLAYER_SPEED * INPUT_DT).clamp(PLAYER_RADIUS, WORLD_H - PLAYER_RADIUS);
    (nx, ny)
}

// ── Wire types ──────────────────────────────────────────────────────────────────────────────────

/// What a streamed entity *is*. A typed enum rather than a bare `u8`: it round-trips by name on
/// the wire, and the client branches on it for colour, collision and — crucially — for which
/// technique applies to it. It is also the reason the client keys its trackers by `(Kind, u32)`
/// rather than a flat id: pickup 3 and hazard 3 are different objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    /// Claimable salvage. Server-owned, removed only by [`ServerMsg::Taken`].
    Pickup,
    /// A drifting hazard. Server-owned, interpolation only, never claimable.
    Hazard,
    /// Another player's ship. Server-owned from this client's point of view.
    Player,
}

/// The client's key for one streamed object. `(Kind, id)` because the three id spaces are
/// independent — the server numbers pickups, hazards and players from 0 each.
pub type NetId = (Kind, u32);

/// One server-owned object in an AOI snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    pub id: u32,
    pub kind: Kind,
    pub x: f32,
    pub y: f32,
}

impl EntityState {
    pub fn net_id(&self) -> NetId {
        (self.kind, self.id)
    }
}

// ── Client → Server ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    /// Sent immediately after `connect`, **before the socket is open**. That is ordinary code on
    /// both targets and it is a deliberate exercise of the native↔wasm contract fixed in v0.150.2:
    /// native queues it in the outbound channel, wasm queues it while `readyState == CONNECTING`
    /// and flushes in `onopen` before `Connected` reaches the game. Before that fix this packet
    /// vanished on the web only, and the game presented as "the server never sends me anything".
    Join,
    /// One input tick: a movement direction and the AOI radius this client wants. `seq` is what
    /// the server echoes back as `ack`, and is what reconciliation replays from.
    Input { seq: u32, mx: f32, my: f32, r: f32 },
    /// "I touched pickup `id`." A request, never a fact — see [`ServerMsg::Taken`].
    Claim { id: u32 },
}

// ── Server → Client ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    /// Sent once on connect: who you are, how big the world is, and how much salvage exists — so
    /// the client's "streaming N / total" readout is not a hardcoded guess.
    Welcome {
        player_id: u32,
        world_w: f32,
        world_h: f32,
        pickups_total: u32,
    },
    /// A per-client snapshot: only the entities within *this* client's AOI, plus the authoritative
    /// state of its own ship.
    ///
    /// `ack` is the last input seq the server has applied. `x`/`y` is where the server thinks this
    /// client's ship is **as of that input** — the pair is what makes reconciliation possible, and
    /// sending the position without the ack would be worse than sending nothing: the client would
    /// snap to a position that is already stale by however many inputs are in flight.
    Snap {
        tick: u32,
        ack: u32,
        x: f32,
        y: f32,
        entities: Vec<EntityState>,
    },
    /// A pickup was taken, by whom, and their new score. Broadcast to **every** client, not just
    /// the winner — this is the message that removes a pickup, and a client that removed it
    /// locally on touch would never need it, which is precisely the bug check 5 exists to catch.
    Taken { id: u32, by: u32, score: u32 },
    /// A player left. Their ship would eventually be evicted by AOI timeout anyway, but only if
    /// they were in range — a disconnect across the map would otherwise leave a tracked id
    /// forever.
    Left { player_id: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_position_normalizes_the_direction() {
        // A diagonal covers the same distance as an axis — not sqrt(2) times more.
        let (ax, ay) = step_position(500.0, 500.0, 1.0, 0.0);
        let (dx, dy) = step_position(500.0, 500.0, 1.0, 1.0);
        let axis = ((ax - 500.0).powi(2) + (ay - 500.0).powi(2)).sqrt();
        let diag = ((dx - 500.0).powi(2) + (dy - 500.0).powi(2)).sqrt();
        assert!(
            (axis - diag).abs() < 1e-3,
            "axis {axis} != diagonal {diag} — the direction is not normalized"
        );
        assert!((axis - PLAYER_SPEED * INPUT_DT).abs() < 1e-3);
    }

    #[test]
    fn step_position_gains_nothing_from_an_over_long_vector() {
        let honest = step_position(500.0, 500.0, 1.0, 0.0);
        let cheat = step_position(500.0, 500.0, 1000.0, 0.0);
        assert!((honest.0 - cheat.0).abs() < 1e-3 && (honest.1 - cheat.1).abs() < 1e-3);
    }

    #[test]
    fn step_position_clamps_inside_the_world() {
        let mut p = (WORLD_W - 20.0, WORLD_H - 20.0);
        for _ in 0..200 {
            p = step_position(p.0, p.1, 1.0, 1.0);
        }
        assert!(p.0 <= WORLD_W - PLAYER_RADIUS + 1e-3 && p.1 <= WORLD_H - PLAYER_RADIUS + 1e-3);
        let mut q = (20.0, 20.0);
        for _ in 0..200 {
            q = step_position(q.0, q.1, -1.0, -1.0);
        }
        assert!(q.0 >= PLAYER_RADIUS - 1e-3 && q.1 >= PLAYER_RADIUS - 1e-3);
    }

    #[test]
    fn a_zero_input_does_not_move() {
        let p = step_position(300.0, 400.0, 0.0, 0.0);
        assert_eq!(p, (300.0, 400.0));
    }

    #[test]
    fn eviction_timeout_has_slack_for_a_dropped_packet() {
        // The value is a trade, so pin the property rather than the number: it must survive one
        // lost snapshot and still be well under a second.
        let interval = 1.0 / SNAPSHOT_HZ as f64;
        assert!(
            AOI_EVICT_SECS > interval * 2.0,
            "a single dropped snapshot would evict entities that never left"
        );
        assert!(
            AOI_EVICT_SECS < 1.0,
            "ghosts would linger for a whole second"
        );
    }

    #[test]
    fn protocol_round_trips() {
        let snap = ServerMsg::Snap {
            tick: 7,
            ack: 3,
            x: 1.0,
            y: 2.0,
            entities: vec![EntityState {
                id: 5,
                kind: Kind::Hazard,
                x: 3.0,
                y: 4.0,
            }],
        };
        let parsed: ServerMsg = ron::from_str(&ron::to_string(&snap).unwrap()).unwrap();
        match parsed {
            ServerMsg::Snap {
                tick,
                ack,
                entities,
                ..
            } => {
                assert_eq!((tick, ack), (7, 3));
                assert_eq!(entities[0].net_id(), (Kind::Hazard, 5));
            }
            other => panic!("expected Snap, got {other:?}"),
        }

        let claim: ClientMsg =
            ron::from_str(&ron::to_string(&ClientMsg::Claim { id: 9 }).unwrap()).unwrap();
        assert!(matches!(claim, ClientMsg::Claim { id: 9 }));
    }

    /// Pins the **wire shape**, not just that a round trip works.
    ///
    /// A pure round-trip test passes even if the encoding silently changes, and here that is not a
    /// theoretical worry: the client and the server compile *separate copies* of this file into
    /// separate processes. They agree because the bytes agree, so the bytes are what the test
    /// asserts. This is also the check that would catch someone adding `#[serde(rename)]` or a
    /// field to one side only.
    #[test]
    fn the_wire_shape_is_pinned() {
        let welcome = ron::to_string(&ServerMsg::Welcome {
            player_id: 1,
            world_w: 100.0,
            world_h: 200.0,
            pickups_total: 3,
        })
        .unwrap();
        assert_eq!(
            welcome,
            "Welcome(player_id:1,world_w:100.0,world_h:200.0,pickups_total:3)"
        );

        let input = ron::to_string(&ClientMsg::Input {
            seq: 2,
            mx: 1.0,
            my: 0.0,
            r: 520.0,
        })
        .unwrap();
        assert_eq!(input, "Input(seq:2,mx:1.0,my:0.0,r:520.0)");

        // A hand-written packet the other side would have to accept.
        let taken: ServerMsg = ron::from_str("Taken(id:4,by:2,score:9)").unwrap();
        assert!(matches!(
            taken,
            ServerMsg::Taken {
                id: 4,
                by: 2,
                score: 9
            }
        ));

        let entity: EntityState = ron::from_str("(id:5,kind:Pickup,x:3.0,y:4.0)").unwrap();
        assert_eq!(entity.net_id(), (Kind::Pickup, 5));
    }

    #[test]
    fn kind_is_part_of_the_key() {
        // Pickup 3 and hazard 3 are different objects; a flat id would collide them.
        let a: NetId = (Kind::Pickup, 3);
        let b: NetId = (Kind::Hazard, 3);
        assert_ne!(a, b);
    }
}
