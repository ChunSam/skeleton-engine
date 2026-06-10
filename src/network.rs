use crate::ecs::{
    events::Events,
    system::System,
    world::{Entity, World},
};
use crate::tween::Lerp;
use std::collections::VecDeque;

pub const DEFAULT_MAX_MESSAGE_BYTES: usize = 64 * 1024;
/// Default max outbound queue size. When the queue is full, new messages are dropped and a warn log is emitted.
pub const DEFAULT_MAX_PENDING_MESSAGES: usize = 256;
/// Default max inbound event queue size. New events are dropped when the limit is exceeded.
pub const DEFAULT_MAX_PENDING_EVENTS: usize = 1024;

/// ECS events emitted by [`NetworkSystem`] every frame.
#[derive(Clone, Debug)]
pub enum NetworkEvent {
    Connected,
    Disconnected { reason: String },
    BinaryMessage(Vec<u8>),
    TextMessage(String),
    MessageTooLarge { len: usize, limit: usize },
    ReceiveQueueFull { dropped: usize, capacity: usize },
    JsonParseError { message: String },
    Error(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetworkConfig {
    pub max_message_bytes: usize,
    /// Max outbound queue size. When exceeded, `send_text`/`send_bytes` drops the message.
    pub max_pending_messages: usize,
    /// Max inbound event queue size. When exceeded, new events are dropped and an overflow event is reported.
    pub max_pending_events: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            max_pending_messages: DEFAULT_MAX_PENDING_MESSAGES,
            max_pending_events: DEFAULT_MAX_PENDING_EVENTS,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn push_event_bounded(
    buffer: &std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<NetworkEvent>>>,
    event: NetworkEvent,
    capacity: usize,
) {
    let mut events = match buffer.lock() {
        Ok(events) => events,
        Err(_) => return,
    };
    if events.len() < capacity {
        events.push_back(event);
    } else if !matches!(events.back(), Some(NetworkEvent::ReceiveQueueFull { .. })) {
        events.pop_back();
        events.push_back(NetworkEvent::ReceiveQueueFull {
            dropped: 1,
            capacity,
        });
    }
}

#[cfg(target_arch = "wasm32")]
fn push_event_bounded(
    buffer: &std::rc::Rc<std::cell::RefCell<Vec<NetworkEvent>>>,
    event: NetworkEvent,
    capacity: usize,
) {
    let mut events = buffer.borrow_mut();
    if events.len() < capacity {
        events.push(event);
    } else if !matches!(events.last(), Some(NetworkEvent::ReceiveQueueFull { .. })) {
        events.pop();
        events.push(NetworkEvent::ReceiveQueueFull {
            dropped: 1,
            capacity,
        });
    }
}

// ── Native implementation ────────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{NetworkConfig, NetworkEvent};
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    enum OutMsg {
        Binary(Vec<u8>),
        Text(String),
        Close,
    }

    pub struct NetworkClient {
        event_buffer: Arc<Mutex<VecDeque<NetworkEvent>>>,
        msg_tx: std::sync::mpsc::SyncSender<OutMsg>,
        connected: Arc<AtomicBool>,
    }

    impl NetworkClient {
        /// Starts a WebSocket connection on a background thread.
        /// Emits [`NetworkEvent::Connected`] on success or [`NetworkEvent::Error`] on failure.
        pub fn connect(url: &str) -> Self {
            Self::connect_with_config(url, NetworkConfig::default())
        }

        pub fn connect_with_config(url: &str, config: NetworkConfig) -> Self {
            let event_buffer = Arc::new(Mutex::new(VecDeque::<NetworkEvent>::new()));
            let thread_event_buffer = Arc::clone(&event_buffer);
            let (msg_tx, msg_rx) =
                std::sync::mpsc::sync_channel::<OutMsg>(config.max_pending_messages);
            let url = url.to_string();
            let max_message_bytes = config.max_message_bytes;
            let max_pending_events = config.max_pending_events;
            let connected = Arc::new(AtomicBool::new(false));
            let thread_connected = Arc::clone(&connected);

            std::thread::spawn(move || {
                let (mut socket, _) = match tungstenite::connect(&url) {
                    Ok(s) => s,
                    Err(e) => {
                        super::push_event_bounded(
                            &thread_event_buffer,
                            NetworkEvent::Error(format!("connect failed: {e}")),
                            max_pending_events,
                        );
                        // connected stays false — no cleanup needed
                        return;
                    }
                };

                // 5 ms read timeout → the loop checks the outbound channel every 5 ms.
                // Set directly on the inner TcpStream for both plain TCP and rustls TLS.
                const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(5);
                let stream = socket.get_mut();
                if let tungstenite::stream::MaybeTlsStream::Plain(tcp) = stream {
                    tcp.set_read_timeout(Some(READ_TIMEOUT)).ok();
                } else if let tungstenite::stream::MaybeTlsStream::Rustls(tls) = stream {
                    // rustls::StreamOwned.sock is a pub field (rustls 0.22+)
                    // so wss:// connections also check the outbound channel every 5 ms.
                    tls.sock.set_read_timeout(Some(READ_TIMEOUT)).ok();
                }

                thread_connected.store(true, Ordering::Release);
                super::push_event_bounded(
                    &thread_event_buffer,
                    NetworkEvent::Connected,
                    max_pending_events,
                );

                loop {
                    // Process outbound messages
                    loop {
                        match msg_rx.try_recv() {
                            Ok(OutMsg::Binary(data)) => {
                                if socket
                                    .send(tungstenite::Message::Binary(data.into()))
                                    .is_err()
                                {
                                    thread_connected.store(false, Ordering::Release);
                                    return;
                                }
                            }
                            Ok(OutMsg::Text(text)) => {
                                if socket
                                    .send(tungstenite::Message::Text(text.into()))
                                    .is_err()
                                {
                                    thread_connected.store(false, Ordering::Release);
                                    return;
                                }
                            }
                            Ok(OutMsg::Close) => {
                                thread_connected.store(false, Ordering::Release);
                                socket.close(None).ok();
                                super::push_event_bounded(
                                    &thread_event_buffer,
                                    NetworkEvent::Disconnected {
                                        reason: "local close".into(),
                                    },
                                    max_pending_events,
                                );
                                return;
                            }
                            Err(std::sync::mpsc::TryRecvError::Empty) => break,
                            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                thread_connected.store(false, Ordering::Release);
                                return;
                            }
                        }
                    }

                    // Process inbound messages (WouldBlock / TimedOut on timeout)
                    match socket.read() {
                        Ok(tungstenite::Message::Binary(data)) => {
                            if data.len() > max_message_bytes {
                                super::push_event_bounded(
                                    &thread_event_buffer,
                                    NetworkEvent::MessageTooLarge {
                                        len: data.len(),
                                        limit: max_message_bytes,
                                    },
                                    max_pending_events,
                                );
                                continue;
                            }
                            super::push_event_bounded(
                                &thread_event_buffer,
                                NetworkEvent::BinaryMessage(data.to_vec()),
                                max_pending_events,
                            );
                        }
                        Ok(tungstenite::Message::Text(text)) => {
                            if text.len() > max_message_bytes {
                                super::push_event_bounded(
                                    &thread_event_buffer,
                                    NetworkEvent::MessageTooLarge {
                                        len: text.len(),
                                        limit: max_message_bytes,
                                    },
                                    max_pending_events,
                                );
                                continue;
                            }
                            super::push_event_bounded(
                                &thread_event_buffer,
                                NetworkEvent::TextMessage(text.to_string()),
                                max_pending_events,
                            );
                        }
                        Ok(tungstenite::Message::Close(frame)) => {
                            let reason = frame.map(|f| f.reason.to_string()).unwrap_or_default();
                            thread_connected.store(false, Ordering::Release);
                            super::push_event_bounded(
                                &thread_event_buffer,
                                NetworkEvent::Disconnected { reason },
                                max_pending_events,
                            );
                            return;
                        }
                        Ok(_) => {} // Ping / Pong / Frame — handled internally by tungstenite
                        Err(tungstenite::Error::Io(e))
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                || e.kind() == std::io::ErrorKind::TimedOut =>
                        {
                            // No data available — restart the loop
                        }
                        Err(e) => {
                            thread_connected.store(false, Ordering::Release);
                            super::push_event_bounded(
                                &thread_event_buffer,
                                NetworkEvent::Error(e.to_string()),
                                max_pending_events,
                            );
                            super::push_event_bounded(
                                &thread_event_buffer,
                                NetworkEvent::Disconnected {
                                    reason: "error".into(),
                                },
                                max_pending_events,
                            );
                            return;
                        }
                    }
                }
            });

            Self {
                event_buffer,
                msg_tx,
                connected,
            }
        }

        pub fn send_bytes(&self, data: &[u8]) {
            if self.msg_tx.try_send(OutMsg::Binary(data.to_vec())).is_err() {
                log::warn!(
                    "network: send queue full — binary message dropped ({} bytes)",
                    data.len()
                );
            }
        }

        pub fn send_text(&self, text: impl Into<String>) {
            let text = text.into();
            if self.msg_tx.try_send(OutMsg::Text(text.clone())).is_err() {
                log::warn!(
                    "network: send queue full — text message dropped ({} bytes)",
                    text.len()
                );
            }
        }

        /// Sends only if the outbound queue is not full; returns whether the send succeeded.
        pub fn try_send_bytes(&self, data: &[u8]) -> bool {
            self.msg_tx.try_send(OutMsg::Binary(data.to_vec())).is_ok()
        }

        /// Sends only if the outbound queue is not full; returns whether the send succeeded.
        pub fn try_send_text(&self, text: impl Into<String>) -> bool {
            self.msg_tx.try_send(OutMsg::Text(text.into())).is_ok()
        }

        pub fn disconnect(&self) {
            let _ = self.msg_tx.try_send(OutMsg::Close);
        }

        /// Returns `true` while the WebSocket handshake has completed and no disconnect or error
        /// has been observed yet. Mirrors the WASM implementation, which checks
        /// `WebSocket.readyState === OPEN`.
        ///
        /// The flag is set by the background thread immediately before emitting
        /// [`NetworkEvent::Connected`] and cleared on every exit path (remote close, local
        /// disconnect, I/O error, or channel drop), so it is safe to poll from the game thread at
        /// any time. Note that there is an inherent race between setting the flag and the main
        /// thread reading it (the same race that exists when reacting to [`NetworkEvent::Connected`]
        /// / [`NetworkEvent::Disconnected`]); treat this as a best-effort snapshot, not a
        /// synchronisation barrier.
        pub fn is_connected(&self) -> bool {
            self.connected.load(Ordering::Acquire)
        }

        pub(super) fn poll(&mut self) -> Vec<NetworkEvent> {
            match self.event_buffer.lock() {
                Ok(mut events) => events.drain(..).collect(),
                Err(_) => Vec::new(),
            }
        }
    }
}

// ── WASM implementation ────────────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod wasm_impl {
    use super::{push_event_bounded, NetworkConfig, NetworkEvent};
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    pub struct NetworkClient {
        socket: Option<web_sys::WebSocket>,
        buffer: Rc<RefCell<Vec<NetworkEvent>>>,
        // Keep closures alive
        _on_open: Option<Closure<dyn FnMut()>>,
        _on_message: Option<Closure<dyn FnMut(web_sys::MessageEvent)>>,
        _on_error: Option<Closure<dyn FnMut(web_sys::Event)>>,
        _on_close: Option<Closure<dyn FnMut(web_sys::CloseEvent)>>,
    }

    impl NetworkClient {
        pub fn connect(url: &str) -> Self {
            Self::connect_with_config(url, NetworkConfig::default())
        }

        pub fn connect_with_config(url: &str, config: NetworkConfig) -> Self {
            let buffer: Rc<RefCell<Vec<NetworkEvent>>> = Rc::new(RefCell::new(Vec::new()));
            let max_message_bytes = config.max_message_bytes;
            let max_pending_events = config.max_pending_events;

            let ws = match web_sys::WebSocket::new(url) {
                Ok(ws) => ws,
                Err(e) => {
                    push_event_bounded(
                        &buffer,
                        NetworkEvent::Error(format!(
                            "WebSocket::new failed: {}",
                            js_value_to_string(&e)
                        )),
                        max_pending_events,
                    );
                    return Self {
                        socket: None,
                        buffer,
                        _on_open: None,
                        _on_message: None,
                        _on_error: None,
                        _on_close: None,
                    };
                }
            };
            ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

            let buf = buffer.clone();
            let on_open = Closure::<dyn FnMut()>::new(move || {
                push_event_bounded(&buf, NetworkEvent::Connected, max_pending_events);
            });
            ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

            let buf = buffer.clone();
            let on_message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(
                move |ev: web_sys::MessageEvent| {
                    let data = ev.data();
                    if let Some(text) = data.as_string() {
                        if text.len() > max_message_bytes {
                            push_event_bounded(
                                &buf,
                                NetworkEvent::MessageTooLarge {
                                    len: text.len(),
                                    limit: max_message_bytes,
                                },
                                max_pending_events,
                            );
                        } else {
                            push_event_bounded(
                                &buf,
                                NetworkEvent::TextMessage(text),
                                max_pending_events,
                            );
                        }
                    } else {
                        let array = js_sys::Uint8Array::new(&data);
                        let bytes = array.to_vec();
                        if bytes.len() > max_message_bytes {
                            push_event_bounded(
                                &buf,
                                NetworkEvent::MessageTooLarge {
                                    len: bytes.len(),
                                    limit: max_message_bytes,
                                },
                                max_pending_events,
                            );
                        } else {
                            push_event_bounded(
                                &buf,
                                NetworkEvent::BinaryMessage(bytes),
                                max_pending_events,
                            );
                        }
                    }
                },
            );
            ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

            let buf = buffer.clone();
            let on_error = Closure::<dyn FnMut(web_sys::Event)>::new(move |_ev: web_sys::Event| {
                push_event_bounded(
                    &buf,
                    NetworkEvent::Error("WebSocket error".into()),
                    max_pending_events,
                );
            });
            ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));

            let buf = buffer.clone();
            let on_close =
                Closure::<dyn FnMut(web_sys::CloseEvent)>::new(move |ev: web_sys::CloseEvent| {
                    push_event_bounded(
                        &buf,
                        NetworkEvent::Disconnected {
                            reason: ev.reason(),
                        },
                        max_pending_events,
                    );
                });
            ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

            Self {
                socket: Some(ws),
                buffer,
                _on_open: Some(on_open),
                _on_message: Some(on_message),
                _on_error: Some(on_error),
                _on_close: Some(on_close),
            }
        }

        pub fn send_bytes(&self, data: &[u8]) {
            if !self.try_send_bytes(data) {
                log::warn!("network: binary message send failed ({} bytes)", data.len());
            }
        }

        pub fn send_text(&self, text: impl Into<String>) {
            let text = text.into();
            if !self.try_send_text(text.clone()) {
                log::warn!("network: text message send failed ({} bytes)", text.len());
            }
        }

        pub fn try_send_bytes(&self, data: &[u8]) -> bool {
            match &self.socket {
                Some(socket) => socket.send_with_u8_array(data).is_ok(),
                None => false,
            }
        }

        pub fn try_send_text(&self, text: impl Into<String>) -> bool {
            let text = text.into();
            match &self.socket {
                Some(socket) => socket.send_with_str(&text).is_ok(),
                None => false,
            }
        }

        pub fn disconnect(&self) {
            if let Some(socket) = &self.socket {
                socket.close().ok();
            }
        }

        /// Returns `true` while the WebSocket is in the `OPEN` (`readyState == 1`) state.
        /// Mirrors the native implementation, which tracks the same lifecycle via an
        /// `Arc<AtomicBool>` shared with the background thread.
        ///
        /// The value reflects the browser's real-time socket state, so it transitions to `false`
        /// as soon as the browser fires the `close` event. Treat this as a best-effort snapshot
        /// consistent with [`NetworkEvent::Connected`] / [`NetworkEvent::Disconnected`].
        pub fn is_connected(&self) -> bool {
            match &self.socket {
                Some(socket) => socket.ready_state() == web_sys::WebSocket::OPEN,
                None => false,
            }
        }

        pub(super) fn poll(&mut self) -> Vec<NetworkEvent> {
            std::mem::take(&mut *self.buffer.borrow_mut())
        }
    }

    fn js_value_to_string(value: &JsValue) -> String {
        value.as_string().unwrap_or_else(|| format!("{value:?}"))
    }
}

// ── Platform re-exports ────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub use native::NetworkClient;

#[cfg(target_arch = "wasm32")]
pub use wasm_impl::NetworkClient;

// ── NetworkSystem ─────────────────────────────────────────────────────────────

/// Polls the [`NetworkClient`] receive buffer every frame and forwards events to [`Events<NetworkEvent>`].
///
/// Registration:
/// ```text
/// app.world.insert_resource(NetworkClient::connect("ws://localhost:9001"));
/// app.world.register_event::<NetworkEvent>();
/// app.add_system(NetworkSystem);
/// ```
pub struct NetworkSystem;

impl System for NetworkSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let incoming: Vec<NetworkEvent> = {
            match world.resource_mut::<NetworkClient>() {
                Some(c) => c.poll(),
                None => return,
            }
        };
        if incoming.is_empty() {
            return;
        }
        if let Some(bus) = world.resource_mut::<Events<NetworkEvent>>() {
            for ev in incoming {
                bus.send(ev);
            }
        }
    }
}

/// Tracks server-owned "remote" entities by a network id, handling the spawn-on-first-sight /
/// despawn-on-removal lifecycle that every networked game otherwise reimplements inline (a
/// `HashMap<id, Entity>` plus get-or-spawn and remove-and-despawn).
///
/// It owns only the `id → Entity` mapping and the spawn/despawn lifecycle. Deciding *what* to spawn
/// (the `spawn` closure), *how* to update an existing entity (call [`get`](Self::get), then mutate
/// it through the `World`), and any parallel game-state maps stay in the game — keeping this a thin,
/// genre-agnostic slice.
///
/// ```
/// # use engine::{RemoteEntities, World};
/// let mut world = World::new();
/// let mut remotes: RemoteEntities<u32> = RemoteEntities::new();
/// // On the first update for network id 7, spawn + insert; later updates reuse the entity.
/// let e = remotes.get_or_spawn(&mut world, 7, |w| w.spawn());
/// let again = remotes.get_or_spawn(&mut world, 7, |w| w.spawn());
/// assert_eq!(again, e);
/// assert_eq!(remotes.get(&7), Some(e));
/// assert_eq!(remotes.len(), 1);
/// // On a "bye" for id 7, remove + despawn.
/// remotes.remove(&mut world, &7);
/// assert!(remotes.get(&7).is_none());
/// ```
///
/// # Deliberately minimal — future deep-dive
///
/// This is intentionally just the lifecycle map. A *richer* version (state interpolation,
/// client-side prediction/reconciliation, per-entity update callbacks, staleness/generation
/// handling) is deferred: the two shipping call sites (`mp_client`, `coin_race`) are structurally
/// similar (JSON relay, `HashMap<usize, Entity>`, spawn-square-on-update), so they don't yet reveal
/// the right shape for those features, and a wrong public (semver-bound) API is worse than the small
/// duplication it removes. Revisit once a *third, distinct* networked example exists — see
/// `docs/REMOTE_ENTITIES_DESIGN.md`.
pub struct RemoteEntities<K: Eq + std::hash::Hash> {
    map: std::collections::HashMap<K, Entity>,
}

impl<K: Eq + std::hash::Hash> Default for RemoteEntities<K> {
    fn default() -> Self {
        Self {
            map: std::collections::HashMap::new(),
        }
    }
}

impl<K: Eq + std::hash::Hash> RemoteEntities<K> {
    /// Creates an empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the entity mapped to `key`, spawning and inserting one via `spawn` on first sight.
    pub fn get_or_spawn(
        &mut self,
        world: &mut World,
        key: K,
        spawn: impl FnOnce(&mut World) -> Entity,
    ) -> Entity {
        if let Some(&entity) = self.map.get(&key) {
            entity
        } else {
            let entity = spawn(world);
            self.map.insert(key, entity);
            entity
        }
    }

    /// The entity currently mapped to `key`, if any.
    pub fn get(&self, key: &K) -> Option<Entity> {
        self.map.get(key).copied()
    }

    /// Whether `key` is currently tracked.
    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// Removes `key` and despawns its entity. No-op if `key` is absent.
    pub fn remove(&mut self, world: &mut World, key: &K) {
        if let Some(entity) = self.map.remove(key) {
            world.despawn(entity);
        }
    }

    /// Despawns every tracked entity and clears the map (e.g. on disconnect or scene reset).
    pub fn clear(&mut self, world: &mut World) {
        for (_, entity) in self.map.drain() {
            world.despawn(entity);
        }
    }

    /// Number of tracked entities.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether no entities are tracked.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Iterates `(&key, entity)` pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, Entity)> {
        self.map.iter().map(|(k, &e)| (k, e))
    }
}

/// A timestamped value sample for [`SnapshotBuffer`].
#[derive(Clone)]
struct Snapshot<T> {
    t: f64,
    value: T,
}

/// Default number of snapshots retained per [`SnapshotBuffer`].
const DEFAULT_SNAPSHOT_CAPACITY: usize = 8;

/// Buffers timestamped snapshots of one remote value and returns an interpolated value at a render
/// time *in the past* (`now - interp_delay`), so server-owned motion stays smooth despite a low
/// snapshot rate. Clamps at the ends when the render time is outside the buffered range.
///
/// Remote state arrives at the network tick rate (often 10–30 Hz) but renders at 60 Hz. Stamp each
/// snapshot with the client's monotonic clock as it arrives ([`push`](Self::push)), then each frame
/// [`sample`](Self::sample) the value at a slightly delayed render time so playback always has two
/// real samples to interpolate between.
///
/// It is generic over any [`Lerp`] value — `f32` (e.g. a rotation angle), [`glam::Vec2`] (a
/// position), [`Color`](crate::color::Color), etc. — and is **orthogonal** to
/// [`RemoteEntities`]: that owns the `id → Entity` lifecycle, this owns the per-entity value
/// history the renderer reads. A game keeps them as parallel maps (see
/// `examples/games/orbital_dodger` and `examples/games/predict_shooter`).
///
/// ```
/// # use engine::SnapshotBuffer;
/// let mut buf: SnapshotBuffer<f32> = SnapshotBuffer::new();
/// buf.push(0.0, 0.0);
/// buf.push(1.0, 10.0);
/// assert_eq!(buf.sample(0.5), Some(5.0)); // halfway between the two samples
/// assert_eq!(buf.sample(-1.0), Some(0.0)); // before range → clamps to the first
/// assert_eq!(buf.sample(2.0), Some(10.0)); // after range → clamps to the last
/// ```
pub struct SnapshotBuffer<T: Lerp> {
    samples: VecDeque<Snapshot<T>>,
    capacity: usize,
}

impl<T: Lerp> Default for SnapshotBuffer<T> {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_SNAPSHOT_CAPACITY)
    }
}

impl<T: Lerp> SnapshotBuffer<T> {
    /// Creates an empty buffer retaining the 8 most recent snapshots (use
    /// [`with_capacity`](Self::with_capacity) to choose a different bound).
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty buffer retaining the most recent `capacity` snapshots (clamped to ≥ 2 so
    /// there is always a pair to interpolate between).
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            samples: VecDeque::new(),
            capacity: capacity.max(2),
        }
    }

    /// Records a snapshot stamped at client time `t` (seconds, monotonic). Out-of-order or duplicate
    /// stamps (`t` not strictly greater than the latest) are ignored so the buffer stays monotonic;
    /// the oldest snapshot is dropped once `capacity` is exceeded.
    pub fn push(&mut self, t: f64, value: T) {
        if let Some(back) = self.samples.back() {
            if t <= back.t {
                return;
            }
        }
        self.samples.push_back(Snapshot { t, value });
        while self.samples.len() > self.capacity {
            self.samples.pop_front();
        }
    }

    /// Interpolated value at render time `rt`. Returns `None` only when the buffer is empty; clamps
    /// to the first/last snapshot when `rt` is before/after the buffered range.
    pub fn sample(&self, rt: f64) -> Option<T> {
        let front = self.samples.front()?;
        if rt <= front.t {
            return Some(front.value.clone());
        }
        let back = self.samples.back()?;
        if rt >= back.t {
            return Some(back.value.clone());
        }
        for i in 0..self.samples.len() - 1 {
            let a = &self.samples[i];
            let b = &self.samples[i + 1];
            if a.t <= rt && rt <= b.t {
                let span = b.t - a.t;
                let f = if span > 0.0 {
                    ((rt - a.t) / span) as f32
                } else {
                    0.0
                };
                return Some(T::lerp(&a.value, &b.value, f));
            }
        }
        Some(back.value.clone())
    }

    /// Whether no snapshots are buffered (so [`sample`](Self::sample) returns `None`).
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Number of buffered snapshots.
    pub fn len(&self) -> usize {
        self.samples.len()
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;

    #[test]
    fn network_config_defaults() {
        let cfg = NetworkConfig::default();
        assert_eq!(cfg.max_message_bytes, DEFAULT_MAX_MESSAGE_BYTES);
        assert_eq!(cfg.max_pending_messages, DEFAULT_MAX_PENDING_MESSAGES);
        assert_eq!(cfg.max_pending_events, DEFAULT_MAX_PENDING_EVENTS);
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
        let buffer = std::sync::Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new()));
        push_event_bounded(&buffer, NetworkEvent::Connected, 1);
        push_event_bounded(&buffer, NetworkEvent::TextMessage("dropped".into()), 1);
        let events: Vec<_> = buffer.lock().unwrap().iter().cloned().collect();
        assert!(matches!(
            events.as_slice(),
            [NetworkEvent::ReceiveQueueFull {
                dropped: 1,
                capacity: 1
            }]
        ));
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
}
