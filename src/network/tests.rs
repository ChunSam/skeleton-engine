use super::*;
use crate::ecs::world::World;

#[test]
fn network_config_defaults() {
    let cfg = NetworkConfig::default();
    assert_eq!(cfg.max_message_bytes, DEFAULT_MAX_MESSAGE_BYTES);
    assert_eq!(cfg.max_pending_messages, DEFAULT_MAX_PENDING_MESSAGES);
    assert_eq!(cfg.max_pending_events, DEFAULT_MAX_PENDING_EVENTS);
    assert_eq!(
        cfg.read_timeout, DEFAULT_READ_TIMEOUT,
        "read_timeout defaults to the historical 5 ms"
    );
}

#[test]
fn network_config_carries_custom_read_timeout() {
    let cfg = NetworkConfig {
        read_timeout: std::time::Duration::from_millis(1),
        ..Default::default()
    };
    assert_eq!(cfg.read_timeout, std::time::Duration::from_millis(1));
    // The other fields keep their defaults (struct-update syntax).
    assert_eq!(cfg.max_message_bytes, DEFAULT_MAX_MESSAGE_BYTES);
}

#[test]
fn connect_with_custom_read_timeout_constructs() {
    // The config→thread plumbing must accept a custom read_timeout without panicking and behave
    // like a normal failed connect (port 1 never listens). Exercises the new field end-to-end on
    // the native client path.
    let cfg = NetworkConfig {
        read_timeout: std::time::Duration::from_millis(1),
        ..Default::default()
    };
    let client = native::NetworkClient::connect_with_config("ws://127.0.0.1:1", cfg);
    assert!(
        !client.is_connected(),
        "is_connected stays false against a dead port regardless of read_timeout"
    );
}

#[test]
fn network_bounded_channel_drops_on_full() {
    // SyncSender with capacity 1: first send succeeds, second fails (full).
    let (tx, _rx) = std::sync::mpsc::sync_channel::<u8>(1);
    assert!(tx.try_send(1).is_ok());
    assert!(
        tx.try_send(2).is_err(),
        "queue should be full after capacity is reached"
    );
}

#[test]
fn receive_queue_reports_full_when_capacity_reached() {
    // capacity=1: pushing the first event fills the queue; the second event
    // triggers first-overflow handling, which displaces the queued event and
    // installs the marker with dropped=2 (one displaced + one rejected).
    let buffer = std::sync::Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new()));
    push_event_bounded(&buffer, NetworkEvent::Connected, 1);
    push_event_bounded(&buffer, NetworkEvent::TextMessage("dropped".into()), 1);
    let events: Vec<_> = buffer.lock().unwrap().iter().cloned().collect();
    assert!(matches!(
        events.as_slice(),
        [NetworkEvent::ReceiveQueueFull {
            dropped: 2,
            capacity: 1
        }]
    ));
}

// ── push_event_bounded overflow invariants ────────────────────────────────

/// Helper: create a fresh native event buffer.
fn make_buffer() -> std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<NetworkEvent>>> {
    std::sync::Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new()))
}

/// Helper: drain all events from a buffer.
fn drain_buffer(
    buffer: &std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<NetworkEvent>>>,
) -> Vec<NetworkEvent> {
    buffer.lock().unwrap().drain(..).collect()
}

/// Helper: current length of the buffer.
fn buf_len(
    buffer: &std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<NetworkEvent>>>,
) -> usize {
    buffer.lock().unwrap().len()
}

#[test]
fn overflow_accumulates_dropped_across_multiple_rejections() {
    // capacity=3: fill with 2 real events, then send 4 overflow events.
    // First overflow: displaces the 2nd real event, marker installed with dropped=2.
    // Next 3 overflows: marker incremented to 3, 4, 5.
    let buf = make_buffer();
    let cap = 3;
    push_event_bounded(&buf, NetworkEvent::Connected, cap); // slot 0
    push_event_bounded(&buf, NetworkEvent::TextMessage("a".into()), cap); // slot 1
    push_event_bounded(&buf, NetworkEvent::TextMessage("b".into()), cap); // slot 2 (fills queue)
                                                                          // overflow 1 — displaces "b" (back), installs marker with dropped=2
    push_event_bounded(&buf, NetworkEvent::TextMessage("ov1".into()), cap);
    assert_eq!(buf_len(&buf), 3, "len must stay at capacity");
    // overflow 2, 3, 4 — increment marker
    push_event_bounded(&buf, NetworkEvent::TextMessage("ov2".into()), cap);
    push_event_bounded(&buf, NetworkEvent::TextMessage("ov3".into()), cap);
    push_event_bounded(&buf, NetworkEvent::TextMessage("ov4".into()), cap);
    assert_eq!(buf_len(&buf), 3, "len must stay at capacity");

    let events = drain_buffer(&buf);
    assert_eq!(events.len(), 3);
    assert!(matches!(events[0], NetworkEvent::Connected));
    assert!(matches!(events[1], NetworkEvent::TextMessage(_)));
    assert!(
        matches!(
            events[2],
            NetworkEvent::ReceiveQueueFull {
                dropped: 5,
                capacity: 3
            }
        ),
        "expected dropped=5 (2 on install + 3 increments), got {:?}",
        events[2]
    );
}

#[test]
fn overflow_does_not_remove_events_queued_before_the_back() {
    // With capacity=4 fill 3 slots, then overflow multiple times.
    // Only the event at slot 3 (back) is displaced when the marker is installed;
    // slots 0-2 must survive intact.
    let buf = make_buffer();
    let cap = 4;
    push_event_bounded(&buf, NetworkEvent::Connected, cap); // slot 0
    push_event_bounded(&buf, NetworkEvent::TextMessage("s1".into()), cap); // slot 1
    push_event_bounded(&buf, NetworkEvent::BinaryMessage(vec![1, 2]), cap); // slot 2
    push_event_bounded(&buf, NetworkEvent::TextMessage("s3".into()), cap); // slot 3 (fills)
                                                                           // overflow events
    for _ in 0..5 {
        push_event_bounded(&buf, NetworkEvent::TextMessage("ov".into()), cap);
    }
    assert_eq!(buf_len(&buf), 4, "len must equal capacity");

    let events = drain_buffer(&buf);
    assert!(
        matches!(events[0], NetworkEvent::Connected),
        "slot 0 preserved"
    );
    assert!(
        matches!(events[1], NetworkEvent::TextMessage(_)),
        "slot 1 preserved"
    );
    assert!(
        matches!(events[2], NetworkEvent::BinaryMessage(_)),
        "slot 2 preserved"
    );
    // slot 3 holds the marker (s3 was displaced + 5 overflow events → dropped=6)
    assert!(
        matches!(events[3], NetworkEvent::ReceiveQueueFull { dropped: 6, .. }),
        "marker at slot 3 with dropped=6, got {:?}",
        events[3]
    );
}

#[test]
fn queue_length_never_exceeds_capacity() {
    // Push capacity + 20 events; length must never exceed capacity at any point.
    let buf = make_buffer();
    let cap = 5;
    for i in 0u32..25 {
        push_event_bounded(&buf, NetworkEvent::TextMessage(i.to_string()), cap);
        assert!(buf_len(&buf) <= cap, "len exceeded capacity after push {i}");
    }
}

#[test]
fn marker_at_back_then_normal_push_after_drain() {
    // Fill queue, trigger overflow (installs marker), drain, then push normally.
    let buf = make_buffer();
    let cap = 2;
    push_event_bounded(&buf, NetworkEvent::Connected, cap); // slot 0
    push_event_bounded(&buf, NetworkEvent::TextMessage("a".into()), cap); // slot 1 (fills)
                                                                          // overflow — displaces "a", marker installed with dropped=2
    push_event_bounded(&buf, NetworkEvent::TextMessage("ov".into()), cap);
    let first_drain = drain_buffer(&buf);
    assert_eq!(first_drain.len(), 2);
    assert!(matches!(first_drain[0], NetworkEvent::Connected));
    assert!(matches!(
        first_drain[1],
        NetworkEvent::ReceiveQueueFull {
            dropped: 2,
            capacity: 2
        }
    ));

    // Queue is now empty — normal pushes must work again.
    push_event_bounded(&buf, NetworkEvent::TextMessage("after".into()), cap);
    let second_drain = drain_buffer(&buf);
    assert_eq!(second_drain.len(), 1);
    assert!(matches!(second_drain[0], NetworkEvent::TextMessage(ref s) if s == "after"));
}

#[derive(Clone)]
struct Marker;

#[test]
fn remote_entities_get_or_spawn_reuses_on_second_sight() {
    let mut world = World::new();
    let mut remotes: RemoteEntities<usize> = RemoteEntities::new();
    assert!(remotes.is_empty());

    let first = remotes.get_or_spawn(&mut world, 1, |w| w.spawn());
    assert_eq!(remotes.len(), 1);
    assert!(remotes.contains_key(&1));
    assert_eq!(remotes.get(&1), Some(first));

    // Same key → same entity; the spawn closure is not run again.
    let again = remotes.get_or_spawn(&mut world, 1, |w| w.spawn());
    assert_eq!(again, first);
    assert_eq!(remotes.len(), 1);

    // Different key → distinct entity.
    let second = remotes.get_or_spawn(&mut world, 2, |w| w.spawn());
    assert_ne!(second, first);
    assert_eq!(remotes.len(), 2);
}

#[test]
fn remote_entities_remove_and_clear_despawn() {
    let mut world = World::new();
    let mut remotes: RemoteEntities<usize> = RemoteEntities::new();
    let a = remotes.get_or_spawn(&mut world, 1, |w| {
        let e = w.spawn();
        w.add_component(e, Marker);
        e
    });
    let b = remotes.get_or_spawn(&mut world, 2, |w| {
        let e = w.spawn();
        w.add_component(e, Marker);
        e
    });
    assert!(world.get_mut::<Marker>(a).is_some());

    remotes.remove(&mut world, &1);
    assert!(remotes.get(&1).is_none());
    assert_eq!(remotes.len(), 1);
    assert!(
        world.get_mut::<Marker>(a).is_none(),
        "removed key's entity should be despawned"
    );

    remotes.clear(&mut world);
    assert!(remotes.is_empty());
    assert!(
        world.get_mut::<Marker>(b).is_none(),
        "clear should despawn every tracked entity"
    );
}

#[test]
fn snapshot_buffer_lerps_between_two_samples() {
    let mut buf: SnapshotBuffer<f32> = SnapshotBuffer::new();
    buf.push(0.0, 0.0);
    buf.push(1.0, 10.0);
    assert_eq!(buf.sample(0.5), Some(5.0));
    assert_eq!(buf.sample(0.25), Some(2.5));
    assert_eq!(buf.sample(0.0), Some(0.0));
    assert_eq!(buf.len(), 2);
}

#[test]
fn snapshot_buffer_clamps_outside_range_and_handles_empty() {
    let mut buf: SnapshotBuffer<f32> = SnapshotBuffer::new();
    buf.push(1.0, 1.0);
    buf.push(2.0, 2.0);
    assert_eq!(buf.sample(0.0), Some(1.0), "before range → first");
    assert_eq!(buf.sample(9.0), Some(2.0), "after range → last");
    assert_eq!(
        SnapshotBuffer::<f32>::new().sample(0.0),
        None,
        "empty → None"
    );
}

#[test]
fn snapshot_buffer_ignores_out_of_order_stamps() {
    let mut buf: SnapshotBuffer<f32> = SnapshotBuffer::new();
    buf.push(2.0, 2.0);
    buf.push(1.0, 9.0); // stale → ignored
    buf.push(2.0, 9.0); // duplicate stamp → ignored
    assert_eq!(buf.len(), 1);
    assert_eq!(buf.sample(5.0), Some(2.0));
}

#[test]
fn snapshot_buffer_interpolates_vec2() {
    use glam::Vec2;
    let mut buf: SnapshotBuffer<Vec2> = SnapshotBuffer::new();
    buf.push(0.0, Vec2::new(0.0, 0.0));
    buf.push(1.0, Vec2::new(10.0, 20.0));
    assert_eq!(buf.sample(0.5), Some(Vec2::new(5.0, 10.0)));
}

#[test]
fn snapshot_buffer_trims_to_capacity() {
    let mut buf: SnapshotBuffer<f32> = SnapshotBuffer::with_capacity(3);
    for i in 0..6 {
        buf.push(i as f64, i as f32);
    }
    assert_eq!(
        buf.len(),
        3,
        "only the most recent `capacity` snapshots are kept"
    );
    // Oldest retained sample is t=3; earlier ones were dropped → clamps to 3.0.
    assert_eq!(buf.sample(0.0), Some(3.0));
    assert_eq!(buf.sample(5.0), Some(5.0));
}

#[test]
fn snapshot_buffer_capacity_floor_is_two() {
    let buf: SnapshotBuffer<f32> = SnapshotBuffer::with_capacity(0);
    assert_eq!(buf.capacity, 2, "capacity is clamped to a ≥2 floor");
}

// ── NetworkClient::is_connected ───────────────────────────────────────────
// We cannot construct a live WebSocket without a running server, so we
// verify the two cheapest invariants:
//   1. A fresh client reports false immediately (flag starts false).
//   2. After the background thread exhausts the failed connect and exits,
//      the flag is still false (it was never set to true).
//
// Both branches that *would* set the flag to true require a successful
// TCP+WebSocket handshake, which cannot happen against a non-listening port,
// so the test is deterministic once the thread exits.
#[test]
fn native_client_is_not_connected_before_handshake() {
    // Port 1 is reserved/non-routable on every platform; connect will fail immediately.
    let client = native::NetworkClient::connect("ws://127.0.0.1:1");
    // Flag starts false — readable without waiting.
    assert!(
        !client.is_connected(),
        "is_connected should be false on a freshly created client"
    );
    // Give the background thread a moment to finish the failed connect so
    // we also verify the flag stays false after the thread exits.
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(
        !client.is_connected(),
        "is_connected should remain false after a failed connection attempt"
    );
}

// ── Task 4: RemoteEntities::get_or_spawn re-spawns after external despawn ──

#[test]
fn remote_entities_respawns_dead_entity() {
    let mut world = World::new();
    let mut remotes: RemoteEntities<usize> = RemoteEntities::new();

    // Spawn for key 1 normally.
    let original = remotes.get_or_spawn(&mut world, 1, |w| w.spawn());
    assert!(
        world.is_alive(original),
        "entity should be alive after spawn"
    );

    // Externally despawn the entity (simulating a scene reset that cleared entities).
    world.despawn(original);
    assert!(
        !world.is_alive(original),
        "entity should be dead after despawn"
    );

    // get_or_spawn should detect the stale entry and re-spawn.
    let replacement = remotes.get_or_spawn(&mut world, 1, |w| w.spawn());
    assert_ne!(
        replacement, original,
        "re-spawn must produce a different entity"
    );
    assert!(
        world.is_alive(replacement),
        "replacement entity must be alive"
    );
    assert_eq!(
        remotes.len(),
        1,
        "map still holds exactly one entry for key 1"
    );
}

// ── Task 3: NetworkSystem warns once when Events<NetworkEvent> is absent ───

#[test]
fn network_system_default_initialises_warn_flag_false() {
    let sys = NetworkSystem::default();
    assert!(!sys.warned_missing_events, "warn flag must start false");
}

// ── Task 1: disconnect() sets close_requested flag ────────────────────────

#[test]
fn disconnect_sets_close_flag() {
    use std::sync::atomic::Ordering;
    // Connect to a guaranteed-to-fail address; the thread exits on connect error
    // before setting `connected`, but the close_requested flag is on the struct side.
    let client = native::NetworkClient::connect("ws://127.0.0.1:1");
    assert!(
        !client.close_requested.load(Ordering::Acquire),
        "close_requested starts false"
    );
    client.disconnect();
    assert!(
        client.close_requested.load(Ordering::Acquire),
        "close_requested must be true after disconnect()"
    );
}

// ── Task 5: NetworkConfig carries max_buffered_bytes, defaults to None ────

#[test]
fn network_config_max_buffered_bytes_defaults_to_none() {
    let cfg = NetworkConfig::default();
    assert_eq!(
        cfg.max_buffered_bytes, None,
        "max_buffered_bytes should default to None"
    );
}
