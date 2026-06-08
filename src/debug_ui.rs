/// In-game egui debug overlay resource.
///
/// Inserted as a Resource into the ECS World; draw egui windows from a `System` via `debug_ui.ctx()`.
/// Toggle with the F1 key. Draw calls are skipped when disabled.
///
/// # Usage
/// ```rust,no_run
/// # use engine::{DebugUi, System, World};
/// struct MyDebugPanel;
/// impl System for MyDebugPanel {
///     fn run(&mut self, world: &mut World, _dt: f32) {
///         let debug = world.resource::<DebugUi>().unwrap();
///         if !debug.is_enabled() { return; }
///         egui::Window::new("Stats").show(debug.ctx(), |ui| {
///             ui.label("Hello from debug!");
///         });
///     }
/// }
/// ```
pub struct DebugUi {
    ctx: egui::Context,
    enabled: bool,
}

impl DebugUi {
    pub(crate) fn new_with_ctx(ctx: egui::Context) -> Self {
        Self {
            ctx,
            enabled: false,
        }
    }

    /// Returns the egui draw context. Must only be used between `begin_frame` and `end_frame`.
    ///
    /// Custom paint callbacks are currently unsupported by the engine renderer and are skipped
    /// at render time to preserve the internal render-pass lifetime safety boundary.
    pub fn ctx(&self) -> &egui::Context {
        &self.ctx
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn set_enabled(&mut self, v: bool) {
        self.enabled = v;
    }
}
