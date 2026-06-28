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

#[cfg(not(target_arch = "wasm32"))]
fn push_event_bounded(
    buffer: &std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<NetworkEvent>>>,
    event: NetworkEvent,
    capacity: usize,
) {
    // Mirror of the wasm path below — keep both semantically identical.
    // See also: push_event_bounded (wasm32 cfg) in this file.
    let mut events = match buffer.lock() {
        Ok(events) => events,
        Err(_) => return,
    };
    if events.len() < capacity {
        events.push_back(event);
    } else {
        // Queue is full — reject `event` without touching earlier entries.
        //
        // Invariant: `len ≤ capacity` at all times.  The back slot is reserved
        // for a ReceiveQueueFull marker once the first overflow occurs.  All
        // entries at indices 0..back()-1 are never modified or removed after
        // they are queued.  The marker itself may displace the youngest real
        // event on first install (counted in `dropped`); after that, subsequent
        // rejections only increment `dropped`.
        if let Some(NetworkEvent::ReceiveQueueFull { dropped, .. }) = events.back_mut() {
            // Marker already present — just accumulate the drop count.
            *dropped += 1;
        } else {
            // First overflow: displace the youngest real event to install the
            // marker.  Both the displaced event and `event` become invisible to
            // the consumer, so `dropped` starts at 2.
            events.pop_back();
            events.push_back(NetworkEvent::ReceiveQueueFull {
                dropped: 2,
                capacity,
            });
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn push_event_bounded(
    buffer: &std::rc::Rc<std::cell::RefCell<Vec<NetworkEvent>>>,
    event: NetworkEvent,
    capacity: usize,
) {
    // Mirror of the native path above — keep both semantically identical.
    // See also: push_event_bounded (not wasm32 cfg) in this file.
    let mut events = buffer.borrow_mut();
    if events.len() < capacity {
        events.push(event);
    } else {
        // Queue is full — reject `event` without touching earlier entries.
        //
        // Invariant: `len ≤ capacity` at all times.  The back slot is reserved
        // for a ReceiveQueueFull marker once the first overflow occurs.  All
        // entries at indices 0..back()-1 are never modified or removed after
        // they are queued.  The marker itself may displace the youngest real
        // event on first install (counted in `dropped`); after that, subsequent
        // rejections only increment `dropped`.
        if let Some(NetworkEvent::ReceiveQueueFull { dropped, .. }) = events.last_mut() {
            // Marker already present — just accumulate the drop count.
            *dropped += 1;
        } else {
            // First overflow: displace the youngest real event to install the
            // marker.  Both the displaced event and `event` become invisible to
            // the consumer, so `dropped` starts at 2.
            events.pop();
            events.push(NetworkEvent::ReceiveQueueFull {
                dropped: 2,
                capacity,
            });
        }
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;
