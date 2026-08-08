use super::{push_event_bounded, NetworkConfig, NetworkEvent};
use std::collections::VecDeque;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// An outbound message queued while the socket is still `CONNECTING`.
///
/// The native client gets this for free: `send_*` pushes into a `sync_channel`, and the
/// background thread only starts draining it after `tungstenite::connect` returns. The web
/// client had no equivalent, so it handed the message straight to a `CONNECTING` socket, which
/// throws — and the message was gone. See [`NetworkClient::try_send_bytes`].
enum OutMsg {
    Binary(Vec<u8>),
    Text(String),
}

/// WebSocket client for WASM targets.
///
/// Drop this resource (or replace it in the World) to close the socket and unregister all
/// browser callbacks. To reconnect, create a new `NetworkClient::connect(...)` and insert it
/// as the resource.
pub struct NetworkClient {
    socket: Option<web_sys::WebSocket>,
    buffer: Rc<RefCell<Vec<NetworkEvent>>>,
    max_buffered_bytes: Option<u32>,
    /// Messages sent before `onopen` fired, flushed in order by that callback. Bounded by
    /// `max_pending_messages`, the same field that bounds the native outbound channel.
    pending: Rc<RefCell<VecDeque<OutMsg>>>,
    max_pending_messages: usize,
    // Keep closures alive; set to None on drop to release them.
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
        let pending: Rc<RefCell<VecDeque<OutMsg>>> = Rc::new(RefCell::new(VecDeque::new()));
        let max_message_bytes = config.max_message_bytes;
        let max_pending_events = config.max_pending_events;
        let max_buffered_bytes = config.max_buffered_bytes;
        let max_pending_messages = config.max_pending_messages;

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
                    max_buffered_bytes,
                    pending,
                    max_pending_messages,
                    _on_open: None,
                    _on_message: None,
                    _on_error: None,
                    _on_close: None,
                };
            }
        };
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let buf = buffer.clone();
        let pend = pending.clone();
        let open_socket = ws.clone();
        let on_open = Closure::<dyn FnMut()>::new(move || {
            // Flush anything the game sent while the handshake was in flight, in order and
            // BEFORE `Connected` reaches the game — so a handshake/join message written as
            // `connect(); send(join)` arrives ahead of whatever the game sends on `Connected`,
            // which is the order the native client already produced.
            //
            // Drain into a local first: the sends below re-enter the browser, and holding a
            // `RefCell` borrow across a foreign call is how a re-entrant `borrow_mut` panics.
            let queued: Vec<OutMsg> = pend.borrow_mut().drain(..).collect();
            for msg in queued {
                let sent = match &msg {
                    OutMsg::Binary(data) => open_socket.send_with_u8_array(data).is_ok(),
                    OutMsg::Text(text) => open_socket.send_with_str(text).is_ok(),
                };
                if !sent {
                    log::warn!("network: queued message failed to send on open");
                }
            }
            push_event_bounded(&buf, NetworkEvent::Connected, max_pending_events);
        });
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

        let buf = buffer.clone();
        let on_message =
            Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |ev: web_sys::MessageEvent| {
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
            });
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        let buf = buffer.clone();
        // Extract error detail from ErrorEvent when available; fall back to a generic
        // message otherwise (the browser fires a plain Event, not ErrorEvent, for
        // network-level errors where no message is available).
        let on_error = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
            let msg = ev
                .dyn_ref::<web_sys::ErrorEvent>()
                .map(|e| e.message())
                .unwrap_or_else(|| "WebSocket error".into());
            push_event_bounded(&buf, NetworkEvent::Error(msg), max_pending_events);
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
            max_buffered_bytes,
            pending,
            max_pending_messages,
            _on_open: Some(on_open),
            _on_message: Some(on_message),
            _on_error: Some(on_error),
            _on_close: Some(on_close),
        }
    }

    /// Queues a message for the `onopen` flush. Returns whether it was accepted.
    ///
    /// Bounded by `max_pending_messages` and drops on overflow, which is what the native
    /// `sync_channel` of the same capacity does when `try_send` finds it full.
    fn queue_until_open(&self, msg: OutMsg) -> bool {
        let mut pending = self.pending.borrow_mut();
        if pending.len() >= self.max_pending_messages {
            log::warn!(
                "network: pending queue full ({} messages) while still CONNECTING — message dropped",
                self.max_pending_messages
            );
            return false;
        }
        pending.push_back(msg);
        true
    }

    pub fn send_bytes(&self, data: &[u8]) {
        if !self.try_send_bytes(data) {
            log::warn!("network: binary message send failed ({} bytes)", data.len());
        }
    }

    pub fn send_text(&self, text: impl Into<String>) {
        let text = text.into();
        let len = text.len();
        if !self.try_send_text(text) {
            log::warn!("network: text message send failed ({} bytes)", len);
        }
    }

    /// Sends, or queues for the `onopen` flush while the socket is still `CONNECTING`.
    ///
    /// **Parity note.** `connect()` returns before the handshake completes on both targets, so
    /// `let c = connect(url); c.send_bytes(join);` is ordinary game code. Native queues that
    /// message in a `sync_channel` and delivers it once the thread is up. The web client used to
    /// hand it to a `CONNECTING` socket, which throws `InvalidStateError` — so `is_ok()` was
    /// false, the message was silently gone, and the same game lost its join packet on the web
    /// and nowhere else. It now queues, bounded by the same `max_pending_messages`.
    pub fn try_send_bytes(&self, data: &[u8]) -> bool {
        let Some(socket) = &self.socket else {
            return false;
        };
        if socket.ready_state() == web_sys::WebSocket::CONNECTING {
            return self.queue_until_open(OutMsg::Binary(data.to_vec()));
        }
        if let Some(limit) = self.max_buffered_bytes {
            if socket.buffered_amount() >= limit {
                log::warn!(
                    "network: WASM send buffer full (bufferedAmount={} >= limit={}) — binary message dropped ({} bytes)",
                    socket.buffered_amount(),
                    limit,
                    data.len()
                );
                return false;
            }
        }
        socket.send_with_u8_array(data).is_ok()
    }

    /// Sends, or queues for the `onopen` flush while the socket is still `CONNECTING`.
    /// See [`try_send_bytes`](Self::try_send_bytes) for why the queue exists.
    pub fn try_send_text(&self, text: impl Into<String>) -> bool {
        let text = text.into();
        let Some(socket) = &self.socket else {
            return false;
        };
        if socket.ready_state() == web_sys::WebSocket::CONNECTING {
            return self.queue_until_open(OutMsg::Text(text));
        }
        if let Some(limit) = self.max_buffered_bytes {
            if socket.buffered_amount() >= limit {
                log::warn!(
                    "network: WASM send buffer full (bufferedAmount={} >= limit={}) — text message dropped ({} bytes)",
                    socket.buffered_amount(),
                    limit,
                    text.len()
                );
                return false;
            }
        }
        socket.send_with_str(&text).is_ok()
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

/// Closes the WebSocket and nulls all browser callbacks when the [`NetworkClient`] is dropped.
///
/// Nulling the callbacks (`set_on*` to `None`) before the `Closure` values are dropped
/// prevents in-flight browser events from invoking already-freed Rust closures.
///
/// Note: unlike the native `Drop` (which emits `NetworkEvent::Disconnected`), dropping the
/// client on WASM does NOT emit `Disconnected` — the `on_close` callback is nulled before
/// `close()`. Drive any reconnect-on-drop logic by other means on the WASM target.
impl Drop for NetworkClient {
    fn drop(&mut self) {
        if let Some(socket) = &self.socket {
            // Unregister callbacks first so no in-flight events fire into freed state.
            socket.set_onopen(None);
            socket.set_onmessage(None);
            socket.set_onerror(None);
            socket.set_onclose(None);
            socket.close().ok();
        }
        // The Option fields drop here, releasing the Closure allocations.
    }
}

fn js_value_to_string(value: &JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}
