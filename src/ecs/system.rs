use super::world::World;

/// A unit of logic executed every frame.
///
/// `dt` is the elapsed time since the previous frame, in seconds.
/// Implement `System` on a struct and register it with `App` for automatic invocation.
pub trait System {
    fn run(&mut self, world: &mut World, dt: f32);

    /// System name shown in the profiler panel. An empty string displays as "anonymous".
    fn name(&self) -> &'static str {
        ""
    }
}
