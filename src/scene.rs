use crate::ecs::schedule::SystemConfig;
use crate::ecs::{System, World};

/// A registrar passed to [`Scene::on_enter`] for adding systems with optional ordering.
///
/// Systems added via [`SystemRegistrar::add`] receive a default (no-constraint)
/// [`SystemConfig`]; systems added via [`SystemRegistrar::add_labeled`] receive a
/// caller-supplied config, enabling label-based ordering (`.after`, `.before`,
/// `.in_set`) exactly like [`App::add_system_labeled`](crate::App::add_system_labeled).
pub struct SystemRegistrar<'a> {
    systems: &'a mut Vec<Box<dyn System>>,
    configs: &'a mut Vec<SystemConfig>,
}

impl<'a> SystemRegistrar<'a> {
    /// Creates a new registrar wrapping the given parallel vecs.
    pub(crate) fn new(
        systems: &'a mut Vec<Box<dyn System>>,
        configs: &'a mut Vec<SystemConfig>,
    ) -> Self {
        Self { systems, configs }
    }

    /// Registers a system with no ordering constraints (insertion order preserved).
    pub fn add(&mut self, system: impl System + 'static) {
        self.systems.push(Box::new(system));
        self.configs.push(SystemConfig::default());
    }

    /// Registers a system with labels/ordering via [`SystemConfig`].
    ///
    /// # Example
    /// ```rust,no_run
    /// # use engine::{scene::{Scene, SceneCmd, SceneChange, SystemRegistrar}, ecs::{System, World}, SystemConfig, LayoutSystem, UiSystem};
    /// # struct MyScene;
    /// # impl Scene for MyScene {
    /// fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
    ///     systems.add(LayoutSystem);
    ///     // UiSystem reads layout results — must run after LayoutSystem.
    ///     systems.add_labeled(UiSystem, SystemConfig::new().after(LayoutSystem::LABEL));
    /// }
    /// # fn on_exit(&mut self, _: &mut World) {}
    /// # }
    /// ```
    pub fn add_labeled(&mut self, system: impl System + 'static, config: SystemConfig) {
        self.systems.push(Box::new(system));
        self.configs.push(config);
    }
}

/// Scene trait. Implemented by each game screen (menu, play, game-over, etc.).
///
/// # Example
/// ```rust,no_run
/// # use engine::{scene::{Scene, SceneCmd, SceneChange, SystemRegistrar}, ecs::{System, World}};
/// struct GamePlay;
///
/// impl Scene for GamePlay {
///     fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
///         // spawn entities, insert resources
///     }
///     fn on_exit(&mut self, _world: &mut World) {}
/// }
/// ```
pub trait Scene: 'static {
    /// Called when the scene is entered. Spawn entities, insert resources, and register systems here.
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar);
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
/// # use engine::{ecs::World, scene::{SceneChange, SceneCmd, SystemRegistrar}};
/// # struct NextScene;
/// # impl engine::scene::Scene for NextScene {
/// #     fn on_enter(&mut self, _: &mut World, _: &mut SystemRegistrar) {}
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

    /// Returns `true` if a scene transition has been requested this frame.
    pub fn is_pending(&self) -> bool {
        self.0.is_some()
    }

    /// Takes the pending [`SceneCmd`] out, leaving the resource empty.
    ///
    /// Intended for `App` internals and test code that needs to inspect the pending
    /// command without holding a mutable reference across frame boundaries.
    /// Returns `None` if no transition was requested.
    pub fn take(&mut self) -> Option<SceneCmd> {
        self.0.take()
    }
}
