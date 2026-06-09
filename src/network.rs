use crate::ecs::{
    events::Events,
    system::System,
    world::{Entity, World},
};

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
    use std::sync::{Arc, Mutex};

    enum OutMsg {
        Binary(Vec<u8>),
        Text(String),
        Close,
    }

    pub struct NetworkClient {
        event_buffer: Arc<Mutex<VecDeque<NetworkEvent>>>,
        msg_tx: std::sync::mpsc::SyncSender<OutMsg>,
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

            std::thread::spawn(move || {
                let (mut socket, _) = match tungstenite::connect(&url) {
                    Ok(s) => s,
                    Err(e) => {
                        super::push_event_bounded(
                            &thread_event_buffer,
                            NetworkEvent::Error(format!("connect failed: {e}")),
                            max_pending_events,
                        );
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
                                    return;
                                }
                            }
                            Ok(OutMsg::Text(text)) => {
                                if socket
                                    .send(tungstenite::Message::Text(text.into()))
                                    .is_err()
                                {
                                    return;
                                }
                            }
                            Ok(OutMsg::Close) => {
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
                            Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
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

        /// Returns true if the socket is in `web_sys::WebSocket::OPEN(1)` state
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
}
