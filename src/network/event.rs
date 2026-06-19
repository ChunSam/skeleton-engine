pub const DEFAULT_MAX_MESSAGE_BYTES: usize = 64 * 1024;
/// Default max outbound queue size. When the queue is full, new messages are dropped and a warn log is emitted.
pub const DEFAULT_MAX_PENDING_MESSAGES: usize = 256;
/// Default max inbound event queue size. New events are dropped when the limit is exceeded.
pub const DEFAULT_MAX_PENDING_EVENTS: usize = 1024;

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
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            max_pending_messages: DEFAULT_MAX_PENDING_MESSAGES,
            max_pending_events: DEFAULT_MAX_PENDING_EVENTS,
            max_buffered_bytes: None,
        }
    }
}
