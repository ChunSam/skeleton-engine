pub const DEFAULT_MAX_MESSAGE_BYTES: usize = 64 * 1024;
/// Default max outbound queue size. When the queue is full, new messages are dropped and a warn log is emitted.
pub const DEFAULT_MAX_PENDING_MESSAGES: usize = 256;
/// Default max inbound event queue size. New events are dropped when the limit is exceeded.
pub const DEFAULT_MAX_PENDING_EVENTS: usize = 1024;
/// Default native socket read timeout — how often the background client thread wakes to flush
/// queued outbound messages. The historical hardcoded value (5 ms). See [`NetworkConfig::read_timeout`].
pub const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(5);

/// ECS events emitted by [`NetworkSystem`](crate::NetworkSystem) every frame.
///
/// This enum is `#[non_exhaustive]`: external crates matching on it must include a
/// wildcard (`_ =>`) arm to remain forward-compatible as new variants are added.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum NetworkEvent {
    Connected,
    Disconnected { reason: String },
    BinaryMessage(Vec<u8>),
    TextMessage(String),
    MessageTooLarge { len: usize, limit: usize },
    ReceiveQueueFull { dropped: usize, capacity: usize },
    Error(String),
}

/// Per-client networking limits and tuning, passed to
/// [`NetworkClient::connect_with_config`](crate::NetworkClient::connect_with_config).
///
/// All fields default to the historical hardcoded values, so
/// [`NetworkClient::connect`](crate::NetworkClient::connect) (which uses
/// [`NetworkConfig::default`]) is behavior-identical.
///
/// ```no_run
/// use engine::{NetworkClient, NetworkConfig};
/// use std::time::Duration;
///
/// // A latency-sensitive game: poll the socket more often so queued outbound
/// // messages flush sooner (slightly more CPU on the client thread).
/// let config = NetworkConfig {
///     read_timeout: Duration::from_millis(1),
///     ..Default::default()
/// };
/// let client = NetworkClient::connect_with_config("ws://127.0.0.1:9001", config);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetworkConfig {
    pub max_message_bytes: usize,
    /// Max outbound queue size. When exceeded, `send_text`/`send_bytes` drops the message.
    pub max_pending_messages: usize,
    /// Max inbound event queue size. When exceeded, new events are dropped and an overflow event is reported.
    pub max_pending_events: usize,
    /// WASM only: maximum number of bytes the browser's WebSocket send buffer (`bufferedAmount`)
    /// may hold before outbound messages are dropped. `None` means no limit.
    /// Native sends are bounded by `max_pending_messages` instead; this field has no effect
    /// on native targets.
    pub max_buffered_bytes: Option<u32>,
    /// Native only: how often the background client thread wakes to flush queued outbound messages
    /// (the socket read timeout). A smaller value flushes sends sooner at the cost of more wake-ups;
    /// a larger one is gentler on the CPU but adds up to this much latency to an outgoing message.
    /// Clamped to a 1 ms floor (a zero timeout would block the thread forever). Has **no effect** on
    /// WASM, whose receive path is event-driven. Defaults to [`DEFAULT_READ_TIMEOUT`] (5 ms).
    pub read_timeout: std::time::Duration,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            max_pending_messages: DEFAULT_MAX_PENDING_MESSAGES,
            max_pending_events: DEFAULT_MAX_PENDING_EVENTS,
            max_buffered_bytes: None,
            read_timeout: DEFAULT_READ_TIMEOUT,
        }
    }
}
