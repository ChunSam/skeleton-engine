use super::{NetworkClient, NetworkEvent};
use crate::ecs::{events::Events, system::System, world::World};

/// Polls the [`NetworkClient`] receive buffer every frame and forwards events to [`Events<NetworkEvent>`].
///
/// Registration:
/// ```text
/// app.world.insert_resource(NetworkClient::connect("ws://localhost:9001"));
/// app.register_event::<NetworkEvent>();
/// app.add_system(NetworkSystem::new());
/// ```
#[derive(Default)]
pub struct NetworkSystem {
    /// Guard for the one-time warning emitted when `Events<NetworkEvent>` is absent.
    /// `pub(super)` so the sibling `tests` module can assert on the flag (it lived in the
    /// same module before the network.rs → network/ split; this keeps that access).
    pub(super) warned_missing_events: bool,
}

impl NetworkSystem {
    /// Creates a new `NetworkSystem`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedule label. Systems consuming `Events<NetworkEvent>` should declare
    /// `.after(NetworkSystem::LABEL)`.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::network";
}

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
        } else if !self.warned_missing_events {
            self.warned_missing_events = true;
            log::warn!(
                "network: NetworkSystem received events but Events<NetworkEvent> is not registered. \
                 Call app.register_event::<NetworkEvent>() to receive network events. \
                 Events will be discarded until it is registered."
            );
        }
    }
}
