//! 2D animation subsystem: frame-based players, blend trees, and state machines.
//!
//! # System registration order
//!
//! The three animation systems must be registered in the following order so that
//! each system's output is ready for the next:
//!
//! 1. **[`BlendTreeSystem`]** (`"engine::blend_tree"`) — reads `BlendTree1D` parameters
//!    and calls `AnimationPlayer::play` / `AnimationPlayer::crossfade` to queue a clip
//!    switch *before* the player advances. Must run **before** `AnimationSystem`.
//!
//! 2. **[`AnimationSystem`]** (`"engine::animation"`) — advances every
//!    `AnimationPlayer` timer, writes `UvRect` / `BlendUv` / `BlendWeight` components
//!    for the renderer. Must run **after** `BlendTreeSystem` and **before**
//!    `StateMachineSystem`.
//!
//! 3. **[`StateMachineSystem`]** (`"engine::state_machine"`) — evaluates transition
//!    conditions against `AnimationPlayer` state (e.g. `AnimationEnd`) and fires clip
//!    transitions. Reading `AnimationPlayer` state requires the player to have already
//!    ticked, so this must run **after** `AnimationSystem`.
//!
//! ```text
//! BlendTreeSystem → AnimationSystem → StateMachineSystem
//! ```
//!
//! Example registration (using labeled ordering):
//! ```rust,ignore
//! use engine::{App, SystemConfig};
//! use engine::animation::{BlendTreeSystem, AnimationSystem, StateMachineSystem};
//!
//! app.add_system_labeled(BlendTreeSystem::new(), SystemConfig::new()
//!     .label(BlendTreeSystem::LABEL)
//!     .before(AnimationSystem::LABEL));
//! app.add_system_labeled(AnimationSystem::new(), SystemConfig::new()
//!     .label(AnimationSystem::LABEL));
//! app.add_system_labeled(StateMachineSystem::new(), SystemConfig::new()
//!     .label(StateMachineSystem::LABEL)
//!     .after(AnimationSystem::LABEL));
//! ```

pub mod blend_system;
pub mod blend_tree;
pub mod player;
pub mod state_machine;
pub mod system;

pub use blend_system::BlendTreeSystem;
pub use blend_tree::{BlendEntry, BlendTree1D};
pub use player::{AnimationClip, AnimationPlayer, BlendWeight};
pub use state_machine::{
    AnimParam, AnimState, AnimTransition, AnimationStateMachine, StateMachineSystem, TransitionCond,
};
pub use system::AnimationSystem;

pub mod clip_set;
pub use clip_set::{AnimationClipRegistry, AnimationClipSet, ClipSetError};
