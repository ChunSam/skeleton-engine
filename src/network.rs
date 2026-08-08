mod event;
#[cfg(not(target_arch = "wasm32"))]
mod native;
mod remote_entities;
mod snapshot;
mod system;
#[cfg(target_arch = "wasm32")]
mod wasm_impl;

pub use event::{
    NetworkConfig, NetworkEvent, DEFAULT_MAX_MESSAGE_BYTES, DEFAULT_MAX_PENDING_EVENTS,
    DEFAULT_MAX_PENDING_MESSAGES, DEFAULT_READ_TIMEOUT,
};
#[cfg(not(target_arch = "wasm32"))]
pub use native::NetworkClient;
pub use remote_entities::RemoteEntities;
pub use snapshot::SnapshotBuffer;
pub use system::NetworkSystem;
#[cfg(target_arch = "wasm32")]
pub use wasm_impl::NetworkClient;

/// The container behind a client's inbound event buffer.
///
/// Native uses a `VecDeque` behind `Arc<Mutex<_>>`, wasm a `Vec` behind `Rc<RefCell<_>>` — the
/// two targets genuinely differ, because only one of them has a background thread. **The
/// overflow policy does not differ**, so it lives exactly once in [`push_bounded`] and each
/// container adapts to it here.
///
/// It used to be hand-mirrored: two copies of the policy, one calling
/// `back_mut`/`pop_back`/`push_back` and the other `last_mut`/`pop`/`push`, held together by a
/// comment on each asking the next editor to keep them identical — while `mod tests` was gated
/// `not(target_arch = "wasm32")`, so the wasm copy had **zero** coverage of the behaviour it was
/// supposed to be mirroring. See `docs/PATTERNS.md` § *Shared policy for cfg-split backends*.
trait EventQueue {
    /// Deliberately not named `len`: an inherent `len` on the implementing type would make the
    /// obvious body (`self.len()`) silently recurse forever.
    fn queued(&self) -> usize;
    fn push_newest(&mut self, event: NetworkEvent);
    fn pop_newest(&mut self);
    fn newest_mut(&mut self) -> Option<&mut NetworkEvent>;
}

impl EventQueue for std::collections::VecDeque<NetworkEvent> {
    fn queued(&self) -> usize {
        self.len()
    }
    fn push_newest(&mut self, event: NetworkEvent) {
        self.push_back(event);
    }
    fn pop_newest(&mut self) {
        self.pop_back();
    }
    fn newest_mut(&mut self) -> Option<&mut NetworkEvent> {
        self.back_mut()
    }
}

impl EventQueue for Vec<NetworkEvent> {
    fn queued(&self) -> usize {
        self.len()
    }
    fn push_newest(&mut self, event: NetworkEvent) {
        self.push(event);
    }
    fn pop_newest(&mut self) {
        self.pop();
    }
    fn newest_mut(&mut self) -> Option<&mut NetworkEvent> {
        self.last_mut()
    }
}

/// Appends `event`, or accounts for it in an overflow marker when the queue is at `capacity`.
///
/// Invariant: `queued() ≤ capacity` at all times. The newest slot is reserved for a
/// `ReceiveQueueFull` marker once the first overflow occurs. Entries older than the newest are
/// never modified or removed after they are queued. The marker itself displaces the youngest
/// real event on first install (counted in `dropped`, hence it starts at 2 — one displaced, one
/// rejected); after that, subsequent rejections only increment `dropped`.
fn push_bounded(queue: &mut impl EventQueue, event: NetworkEvent, capacity: usize) {
    if queue.queued() < capacity {
        queue.push_newest(event);
        return;
    }
    if let Some(NetworkEvent::ReceiveQueueFull { dropped, .. }) = queue.newest_mut() {
        // Marker already present — just accumulate the drop count.
        *dropped += 1;
    } else {
        queue.pop_newest();
        queue.push_newest(NetworkEvent::ReceiveQueueFull {
            dropped: 2,
            capacity,
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn push_event_bounded(
    buffer: &std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<NetworkEvent>>>,
    event: NetworkEvent,
    capacity: usize,
) {
    // A poisoned lock drops the event rather than propagating a panic into a network callback.
    let Ok(mut events) = buffer.lock() else {
        return;
    };
    push_bounded(&mut *events, event, capacity);
}

#[cfg(target_arch = "wasm32")]
fn push_event_bounded(
    buffer: &std::rc::Rc<std::cell::RefCell<Vec<NetworkEvent>>>,
    event: NetworkEvent,
    capacity: usize,
) {
    push_bounded(&mut *buffer.borrow_mut(), event, capacity);
}

/// Overflow-policy tests, deliberately **not** `cfg`-gated to native.
///
/// The sibling `tests` module is native-only because it spawns threads and opens sockets; that
/// gate is what left the wasm event queue untested. These run against both containers on any
/// target, and the wasm one (`Vec`) is the point.
#[cfg(test)]
mod policy_tests {
    use super::*;

    /// A comparable rendering, so two containers can be diffed instead of eyeballed.
    fn summarize(events: &[NetworkEvent]) -> Vec<String> {
        events
            .iter()
            .map(|e| match e {
                NetworkEvent::Connected => "connected".to_string(),
                NetworkEvent::TextMessage(t) => format!("text:{t}"),
                NetworkEvent::BinaryMessage(b) => format!("bin:{}", b.len()),
                NetworkEvent::ReceiveQueueFull { dropped, capacity } => {
                    format!("full:{dropped}/{capacity}")
                }
                other => format!("{other:?}"),
            })
            .collect()
    }

    fn script() -> Vec<NetworkEvent> {
        vec![
            NetworkEvent::Connected,
            NetworkEvent::TextMessage("a".into()),
            NetworkEvent::BinaryMessage(vec![1, 2, 3]),
            NetworkEvent::TextMessage("ov1".into()),
            NetworkEvent::TextMessage("ov2".into()),
            NetworkEvent::TextMessage("ov3".into()),
        ]
    }

    /// The wasm `Vec` and the native `VecDeque` must agree at **every** step, not just at the end.
    ///
    /// This is the coverage the wasm copy never had. Two independent implementations can also be
    /// identically wrong, so the expected sequence is pinned below rather than only compared.
    #[test]
    fn both_targets_overflow_identically() {
        let cap = 3;
        let mut native = std::collections::VecDeque::new();
        let mut wasm: Vec<NetworkEvent> = Vec::new();

        for (i, event) in script().into_iter().enumerate() {
            push_bounded(&mut native, event.clone(), cap);
            push_bounded(&mut wasm, event, cap);

            let n: Vec<_> = native.iter().cloned().collect();
            assert_eq!(
                summarize(&n),
                summarize(&wasm),
                "targets diverged after push {i}"
            );
            assert!(
                wasm.len() <= cap,
                "wasm queue exceeded capacity at push {i}"
            );
        }

        // Pinned, so "identical" cannot quietly mean "identically wrong": 3 events fill the
        // queue, then the first overflow displaces the binary message to install the marker
        // (dropped=2) and the remaining two overflows increment it to 4.
        assert_eq!(
            summarize(&wasm),
            vec!["connected", "text:a", "full:4/3"],
            "wasm (Vec) overflow sequence"
        );
    }

    /// Entries older than the newest slot survive any amount of overflow — asserted on the
    /// wasm container specifically, since the native one already had this test.
    #[test]
    fn wasm_queue_preserves_everything_but_the_newest_slot() {
        let cap = 4;
        let mut wasm: Vec<NetworkEvent> = Vec::new();
        push_bounded(&mut wasm, NetworkEvent::Connected, cap);
        push_bounded(&mut wasm, NetworkEvent::TextMessage("s1".into()), cap);
        push_bounded(&mut wasm, NetworkEvent::BinaryMessage(vec![1, 2]), cap);
        push_bounded(&mut wasm, NetworkEvent::TextMessage("s3".into()), cap);
        for _ in 0..5 {
            push_bounded(&mut wasm, NetworkEvent::TextMessage("ov".into()), cap);
        }

        assert_eq!(
            summarize(&wasm),
            vec!["connected", "text:s1", "bin:2", "full:6/4"],
            "slots 0-2 intact; s3 displaced by the marker, then 5 overflows → dropped=6"
        );
    }

    /// After a drain the queue accepts normal pushes again — the marker is not sticky.
    #[test]
    fn wasm_queue_recovers_after_a_drain() {
        let cap = 2;
        let mut wasm: Vec<NetworkEvent> = Vec::new();
        push_bounded(&mut wasm, NetworkEvent::Connected, cap);
        push_bounded(&mut wasm, NetworkEvent::TextMessage("a".into()), cap);
        push_bounded(&mut wasm, NetworkEvent::TextMessage("ov".into()), cap);
        assert_eq!(summarize(&wasm), vec!["connected", "full:2/2"]);

        // `poll()` is a `std::mem::take` on the wasm side.
        let drained = std::mem::take(&mut wasm);
        assert_eq!(drained.len(), 2);

        push_bounded(&mut wasm, NetworkEvent::TextMessage("after".into()), cap);
        assert_eq!(summarize(&wasm), vec!["text:after"]);
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;
