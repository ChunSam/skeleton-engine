//! Animation events — fire a tagged event when an animation playhead enters a frame.
//!
//! Attach an [`AnimationEvents`] component next to an
//! [`AnimationPlayer`](crate::AnimationPlayer). Whenever
//! [`AnimationSystem`](crate::AnimationSystem) advances that player **onto** a frame listed in the
//! component, it sends an [`AnimationEvent`] on `Events<AnimationEvent>` — register the bus once with
//! `App::register_event::<AnimationEvent>()`, then read it in a later system the same frame. This is
//! the idiomatic way to drive footsteps, attack hit-frames, or VFX/SFX off an animation without
//! polling `AnimationPlayer::current_frame` by hand.
//!
//! This is a **separate component**, not a field on [`AnimationClip`](crate::AnimationClip), so it is
//! purely additive — every existing clip and player keeps working unchanged, and a game can attach
//! different event sets to entities that share the same (possibly registry-owned) clip data.

use serde::{Deserialize, Serialize};

use crate::ecs::Entity;

/// One frame-triggered event: emit `tag` when clip `clip`'s playhead enters `frame`.
///
/// `clip` indexes [`AnimationPlayer::clips`](crate::AnimationPlayer); `frame` indexes that clip's
/// `frames`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameEvent {
    /// Index into [`AnimationPlayer::clips`](crate::AnimationPlayer) this event belongs to.
    pub clip: usize,
    /// The frame (within that clip) whose entry fires the event.
    pub frame: usize,
    /// Game-defined label delivered on the emitted [`AnimationEvent`] (e.g. `"footstep"`).
    pub tag: String,
}

/// Component listing the frame events for an entity's
/// [`AnimationPlayer`](crate::AnimationPlayer).
///
/// Each frame [`AnimationSystem`](crate::AnimationSystem) advances the playhead onto a frame matching
/// one of these defs, it sends an [`AnimationEvent`]. Register the bus once with
/// `App::register_event::<AnimationEvent>()` and read it the same frame via
/// `world.resource::<Events<AnimationEvent>>()`.
///
/// Events fire on frame **transitions**, not on the frame currently shown: the clip's initial frame
/// at spawn does not fire (it is never "entered"), and a looping clip re-fires a frame-0 event each
/// time it wraps back to 0. During a crossfade, only the outgoing (current) clip emits; the incoming
/// clip starts emitting once the crossfade completes and it becomes current.
///
/// ```
/// # use engine::AnimationEvents;
/// // Footsteps on the two contact frames of clip 0:
/// let ev = AnimationEvents::new().on(0, 1, "footstep").on(0, 3, "footstep");
/// assert_eq!(ev.events.len(), 2);
/// assert_eq!(ev.events[0].frame, 1);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationEvents {
    /// The frame-event definitions, matched by `(clip, frame)`.
    pub events: Vec<FrameEvent>,
}

impl AnimationEvents {
    /// An empty event set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder: add an event firing `tag` when `clip` enters `frame`.
    pub fn on(mut self, clip: usize, frame: usize, tag: impl Into<String>) -> Self {
        self.events.push(FrameEvent {
            clip,
            frame,
            tag: tag.into(),
        });
        self
    }
}

/// Emitted by [`AnimationSystem`](crate::AnimationSystem) when an entity's playhead enters a frame
/// that has a matching [`AnimationEvents`] definition.
///
/// Register the bus with `App::register_event::<AnimationEvent>()` and read it the same frame via
/// `world.resource::<Events<AnimationEvent>>()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimationEvent {
    /// The entity whose animation produced the event.
    pub entity: Entity,
    /// The clip index the playhead was on when the frame was entered.
    pub clip: usize,
    /// The frame that was entered.
    pub frame: usize,
    /// The matching [`FrameEvent::tag`].
    pub tag: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_collects_events_in_order() {
        let ev = AnimationEvents::new()
            .on(0, 1, "footstep")
            .on(0, 3, "footstep")
            .on(2, 4, "hit");
        assert_eq!(ev.events.len(), 3);
        assert_eq!(ev.events[2].clip, 2);
        assert_eq!(ev.events[2].frame, 4);
        assert_eq!(ev.events[2].tag, "hit");
    }

    #[test]
    fn default_is_empty() {
        assert!(AnimationEvents::default().events.is_empty());
    }

    #[test]
    fn serde_roundtrip() {
        let ev = AnimationEvents::new().on(1, 2, "step");
        let ron = ron::to_string(&ev).unwrap();
        let back: AnimationEvents = ron::from_str(&ron).unwrap();
        assert_eq!(ev, back);
    }
}
