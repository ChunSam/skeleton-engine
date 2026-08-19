//! Editor/profiling resources — the Inspector's selected entity plus per-frame profiler data.

/// Exposes the currently selected entity in the Inspector as a World resource.
///
/// `App` synchronizes this with `inspector_selected` every frame.
/// Read from systems for selection highlighting, path planning, and other editor integrations.
///
/// ```text
/// if let Some(e) = world.resource::<SelectedEntity>().and_then(|s| s.0) {
///     // e is the entity currently selected in the Inspector
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectedEntity(pub Option<crate::ecs::world::Entity>);

/// Profiling entry for a single system.
#[derive(Debug, Clone, Default)]
pub struct SystemProfile {
    pub name: String,
    /// Execution time of the previous frame (microseconds).
    pub last_us: u64,
    /// Exponential moving average over the last 60 frames (microseconds).
    pub avg_us: f32,
}

/// Renderer pass statistics.
///
/// ⚠️ **Sprite pass only.** Every field here is measured inside `SpriteRenderer`. The UI-primitive
/// pass, the text pass and the post-process / lighting / bloom / GPU-particle passes contribute
/// nothing to any of them, so these are not whole-frame totals — treat them as a sprite-workload
/// gauge, which is what the Engine Stats panel labels them as.
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderStats {
    /// Draw calls issued by the **sprite pass**: one per contiguous same-texture sprite run, plus
    /// one per `ShaderMaterial` entry.
    ///
    /// ⚠️ Not the frame's total. The UI-primitive pass issues one draw per texture run of its own
    /// and does not count them, and the text pass draws through `glyphon`, whose internal draw
    /// count the engine cannot observe at all. A number that claimed to be the total would be
    /// wrong in both directions, so this one is scoped instead.
    pub draw_calls: u32,
    /// Number of sprite instances submitted to the GPU.
    pub sprites_rendered: u32,
    /// Number of sprites skipped by view culling / LOD.
    pub sprites_culled: u32,
}

/// Complete profiler data. Updated every frame by `App` and read by the Engine Stats panel.
#[derive(Debug, Clone, Default)]
pub struct ProfilerData {
    pub systems: Vec<SystemProfile>,
    pub render: RenderStats,
    /// Total frame time (ms).
    pub frame_ms: f32,
}

impl ProfilerData {
    /// EMA α = 1/60
    const ALPHA: f32 = 1.0 / 60.0;

    /// Records a system execution result. Automatically expands if `idx` is out of range.
    pub fn record_system(&mut self, idx: usize, name: &str, elapsed_us: u64) {
        if idx >= self.systems.len() {
            self.systems.resize(idx + 1, SystemProfile::default());
        }
        let s = &mut self.systems[idx];
        s.name = name.to_string();
        s.last_us = elapsed_us;
        s.avg_us = s.avg_us * (1.0 - Self::ALPHA) + elapsed_us as f32 * Self::ALPHA;
    }
}
