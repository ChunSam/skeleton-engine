use crate::ecs::{System, World};

/// Scene trait. Implemented by each game screen (menu, play, game-over, etc.).
///
/// # Example
/// ```rust,no_run
/// # use engine::{scene::{Scene, SceneCmd, SceneChange}, ecs::{System, World}};
/// struct GamePlay;
///
/// impl Scene for GamePlay {
///     fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>) {
///         // spawn entities, insert resources
///     }
///     fn on_exit(&mut self, _world: &mut World) {}
/// }
/// ```
pub trait Scene: 'static {
    /// Called when the scene is entered. Spawn entities, insert resources, and register systems here.
    fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>);
    /// Called when the scene is exited. Implement only when cleanup is needed.
    fn on_exit(&mut self, _world: &mut World) {}
}

/// Scene transition command.
pub enum SceneCmd {
    /// Clears the entire scene stack and replaces it with a new scene (includes world reset).
    Replace(Box<dyn Scene>),
    /// Pushes a new scene on top of the current one (world is preserved; useful for pause menus).
    Push(Box<dyn Scene>),
    /// Pops the top scene off the stack.
    Pop,
}

/// Resource used by systems to request a scene transition.
///
/// # Example
/// ```rust,no_run
/// # use engine::{ecs::World, scene::{SceneChange, SceneCmd}};
/// # struct NextScene;
/// # impl engine::scene::Scene for NextScene {
/// #     fn on_enter(&mut self, _: &mut World, _: &mut Vec<Box<dyn engine::ecs::System>>) {}
/// # }
/// # struct MySystem;
/// # impl engine::ecs::System for MySystem {
/// fn run(&mut self, world: &mut World, _dt: f32) {
///     if let Some(sc) = world.resource_mut::<SceneChange>() {
///         sc.request(SceneCmd::Replace(Box::new(NextScene)));
///     }
/// }
/// # }
/// ```
#[derive(Default)]
pub struct SceneChange(pub(crate) Option<SceneCmd>);

impl SceneChange {
    /// Registers a scene transition command. If called multiple times in the same frame, only the last command takes effect.
    pub fn request(&mut self, cmd: SceneCmd) {
        self.0 = Some(cmd);
    }
}
