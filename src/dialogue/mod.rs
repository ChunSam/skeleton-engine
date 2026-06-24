//! Dialogue box primitive — a speaker + typewriter text box for RPG / visual-novel / narrative
//! games.
//!
//! Every narrative 2D game re-implements the same thing: a box that reveals a line of text one
//! character at a time and advances on a key press. [`DialogueBox`] is that, as a reusable
//! component; [`DialogueSystem`] ticks the typewriter and renders the box (screen-space text via
//! the [`TextQueue`](crate::renderer::TextQueue)). The game decides *when* to advance by calling
//! [`DialogueBox::advance`] (e.g. on a Space press) — the system stays input-agnostic.
//!
//! ```no_run
//! use engine::{App, DialogueBox, DialogueSystem};
//! let mut app = App::new();
//! let e = app.world.spawn();
//! app.world.add_component(
//!     e,
//!     DialogueBox::new("Guide", ["Welcome, traveler.", "Press space to continue."])
//!         .with_chars_per_sec(28.0),
//! );
//! app.add_system(DialogueSystem);
//! // in your own system: on Space, `world.get_mut::<DialogueBox>(e).unwrap().advance();`
//! ```
//!
//! The module is split by concern: `model` (the [`DialogueBox`] / [`DialogueChoice`] data types +
//! the world-level [`advance`] / [`choose`] control fns), `style` ([`DialogueStyle`]), the
//! rendering `system` ([`DialogueSystem`]), plus data-driven dialogue (`tree`) and conditional
//! vars/effects (`vars`). All public items are re-exported here, so the `crate::dialogue::*` paths
//! are unchanged.

mod model;
mod style;
mod system;
mod tree;
mod vars;

pub use model::{advance, choose, DialogueBox, DialogueChoice};
pub use style::DialogueStyle;
pub use system::DialogueSystem;
pub use tree::{DialogueRegistry, DialogueTree};
pub use vars::{
    DialogueCond, DialogueEffect, DialogueEvent, DialogueOp, DialogueValue, DialogueVars,
};

#[cfg(test)]
mod tests;
