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
    /// Set by `disconnect()` and checked by the background thread each loop iteration.
    /// This guarantees the Close reaches the thread even when the outbound queue is full.
    /// `pub(super)` so the sibling `tests` module can assert on the flag without going
    /// through the public API.
    pub(super) close_requested: Arc<AtomicBool>,
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
        let (msg_tx, msg_rx) = std::sync::mpsc::sync_channel::<OutMsg>(config.max_pending_messages);
        let url = url.to_string();
        let max_message_bytes = config.max_message_bytes;
        let max_pending_events = config.max_pending_events;
        let connected = Arc::new(AtomicBool::new(false));
        let thread_connected = Arc::clone(&connected);
        let close_requested = Arc::new(AtomicBool::new(false));
        let thread_close_requested = Arc::clone(&close_requested);

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
                // Check the close flag first — reliable close path even when the
                // outbound channel queue is full (task 1).
                if thread_close_requested.load(Ordering::Acquire) {
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
            close_requested,
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
        let len = text.len();
        if self.msg_tx.try_send(OutMsg::Text(text)).is_err() {
            log::warn!("network: send queue full — text message dropped ({len} bytes)");
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

    /// Requests the background thread to close the WebSocket and exit.
    ///
    /// The close flag (`close_requested`) is set unconditionally so the thread will see
    /// it on its next loop iteration even if the outbound message queue is full. A
    /// best-effort `OutMsg::Close` is also enqueued for faster response; if the queue is
    /// full the flag alone is sufficient.
    pub fn disconnect(&self) {
        self.close_requested.store(true, Ordering::Release);
        if self.msg_tx.try_send(OutMsg::Close).is_err() {
            log::warn!(
                "network: disconnect — outbound queue full; close flag set, thread will exit on next tick"
            );
        }
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

/// Signals the background thread to close the socket when the [`NetworkClient`] is dropped.
///
/// The thread checks `close_requested` each loop iteration and exits cleanly (calling
/// `socket.close()`). This prevents the background thread from outliving the resource.
impl Drop for NetworkClient {
    fn drop(&mut self) {
        self.close_requested.store(true, Ordering::Release);
        // Best-effort fast path — ignored if the queue is full; the flag is the guarantee.
        let _ = self.msg_tx.try_send(OutMsg::Close);
    }
}
