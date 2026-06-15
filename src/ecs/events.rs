/// Per-frame event bus.
///
/// Register with `App::register_event::<E>()` to insert it as a World resource.
/// Systems send events via `world.resource_mut::<Events<E>>().send(e)` and read them
/// with `world.resource::<Events<E>>().read()` in a later system of **the same frame**.
/// At the end of every frame, `App` automatically calls `flush()` to drain the queue,
/// so events are **not** available in the following frame.
pub struct Events<E: 'static> {
    items: Vec<E>,
}

impl<E: 'static> Default for Events<E> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<E: 'static> Events<E> {
    /// Appends an event to the current frame's queue.
    pub fn send(&mut self, event: E) {
        self.items.push(event);
    }

    /// Returns a slice of the current frame's events.
    ///
    /// The slice is valid until `flush()` is called (i.e., until the end of the frame).
    pub fn read(&self) -> &[E] {
        &self.items
    }

    /// Called by `App` at end-of-frame. No need to call this directly from outside.
    pub fn flush(&mut self) {
        self.items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_and_read() {
        let mut events: Events<u32> = Events::default();
        assert!(events.read().is_empty());

        events.send(1);
        events.send(2);
        assert_eq!(events.read(), &[1, 2]);
    }

    #[test]
    fn flush_clears_queue() {
        let mut events: Events<u32> = Events::default();
        events.send(42);
        events.flush();
        assert!(events.read().is_empty());
    }
}
