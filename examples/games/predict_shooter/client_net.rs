//! Pure client-side netcode for the prediction shooter — no rendering or I/O, so it is
//! unit-tested headlessly (see the `tests` module). The windowed client (`predict_shooter.rs`)
//! wires these into ECS systems; its interactive *feel* (responsiveness, smoothness) is validated
//! separately in real play.
//!
//! One piece:
//! - [`Prediction`] — local-player client-side prediction + server reconciliation.
//!
//! Remote-entity snapshot interpolation, which used to live here as a private `Interp`, is now the
//! public, generic `engine::SnapshotBuffer<Vec2>` (the client builds it from each snapshot's
//! position) — see `predict_shooter.rs`. `Prediction` stays example-local: it's a *local* concern
//! (input replay vs. server correction), not remote-entity bookkeeping.
#![allow(dead_code)]

use crate::protocol::step_position;
use std::collections::VecDeque;

/// One locally-applied input, retained until the server acks it so it can be replayed during
/// reconciliation.
#[derive(Clone, Copy, Debug)]
struct PendingInput {
    seq: u32,
    mx: f32,
    my: f32,
}

/// Local-player client-side prediction + server reconciliation.
///
/// [`predict`](Self::predict) applies an input immediately (so movement feels instant) and buffers
/// it; [`reconcile`](Self::reconcile) snaps to the server's authoritative position at the acked
/// input and replays the inputs the server hasn't processed yet, so the prediction converges to the
/// server without rubber-banding. Both apply [`step_position`], identical to the server's authority.
pub struct Prediction {
    pub x: f32,
    pub y: f32,
    next_seq: u32,
    pending: VecDeque<PendingInput>,
}

impl Prediction {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            next_seq: 1,
            pending: VecDeque::new(),
        }
    }

    /// Applies an input locally (prediction) and records it for replay. Returns its `seq`.
    pub fn predict(&mut self, mx: f32, my: f32) -> u32 {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        let (nx, ny) = step_position(self.x, self.y, mx, my);
        self.x = nx;
        self.y = ny;
        self.pending.push_back(PendingInput { seq, mx, my });
        seq
    }

    /// Reconciles to the authoritative `(server_x, server_y)` acked at input `ack`: drops acked
    /// inputs, then replays the remaining (unacked) inputs from the server position. The result is
    /// the same position continuous local prediction would reach, but corrected to the server.
    pub fn reconcile(&mut self, server_x: f32, server_y: f32, ack: u32) {
        while let Some(front) = self.pending.front() {
            if front.seq <= ack {
                self.pending.pop_front();
            } else {
                break;
            }
        }
        let (mut x, mut y) = (server_x, server_y);
        for p in &self.pending {
            let (nx, ny) = step_position(x, y, p.mx, p.my);
            x = nx;
            y = ny;
        }
        self.x = x;
        self.y = y;
    }

    pub fn pos(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::step_position;

    #[test]
    fn prediction_moves_immediately_and_matches_step_position() {
        let mut p = Prediction::new(100.0, 100.0);
        let seq = p.predict(1.0, 0.0);
        assert_eq!(seq, 1);
        let (ex, ey) = step_position(100.0, 100.0, 1.0, 0.0);
        assert!((p.x - ex).abs() < 1e-4 && (p.y - ey).abs() < 1e-4);
        assert_eq!(p.pending_len(), 1);
    }

    #[test]
    fn reconcile_drops_acked_and_replays_unacked() {
        let start = (200.0_f32, 200.0_f32);
        let cmds = [(1.0_f32, 0.0_f32), (0.0, 1.0), (1.0, 1.0)];
        let mut p = Prediction::new(start.0, start.1);
        for &(mx, my) in &cmds {
            p.predict(mx, my);
        }

        // Server has applied inputs 1..=2 → compute its authoritative pos.
        let mut sv = start;
        sv = step_position(sv.0, sv.1, cmds[0].0, cmds[0].1);
        sv = step_position(sv.0, sv.1, cmds[1].0, cmds[1].1);
        p.reconcile(sv.0, sv.1, 2);
        assert_eq!(p.pending_len(), 1, "only the unacked input remains");

        // Reconciled == full continuous chain from the start.
        let mut full = start;
        for &(mx, my) in &cmds {
            full = step_position(full.0, full.1, mx, my);
        }
        assert!(
            (p.x - full.0).abs() < 1e-3 && (p.y - full.1).abs() < 1e-3,
            "reconciled {:?} != full chain {:?}",
            (p.x, p.y),
            full
        );
    }

    #[test]
    fn reconcile_with_all_acked_snaps_to_server() {
        let mut p = Prediction::new(0.0, 0.0);
        p.predict(1.0, 0.0);
        p.predict(1.0, 0.0);
        p.reconcile(500.0, 300.0, 2);
        assert_eq!(p.pending_len(), 0);
        assert_eq!((p.x, p.y), (500.0, 300.0));
    }
}
