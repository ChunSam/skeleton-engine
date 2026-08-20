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

/// Per-frame shaped-text cache counters — how many plain [`DrawText`](crate::DrawText)s this frame
/// were served from the cross-frame shaped-buffer cache, and how many had to be re-shaped.
///
/// Written by `App` after the frame's last text pass and **reset every frame**, so it always
/// describes the frame that just finished. Rich text is not cached and is counted in neither field.
///
/// ⚠️ **Deliberately its own resource, not a field on [`RenderStats`]** — that type is documented
/// as sprite-pass-only, and hanging text counters on it would make that sentence false.
///
/// # Writing a check against it
///
/// The obvious assertion is vacuous. **`hits > 0` is satisfied by a static HUD**, and was
/// satisfied before v0.153.2 too — the property that release actually changed is that a text whose
/// **position** moves every frame still hits, because position provably cannot affect layout for a
/// centered or bounds-set draw. So:
///
/// - Drive a **moving** `DrawText::centered` and require the miss count to stop rising once the
///   cache is warm. Name the warm-up window explicitly and exclude it — frame 1 is all-miss by
///   definition, and a check that leaves the window implicit regresses silently when it changes
///   (`docs/CHANGELOG.md` § *"A warm-up is part of the property, not setup noise"*).
/// - Pair it with a text whose **content** changes every frame, which must miss every frame.
///   Without that control, a counter that never increments `misses` passes.
/// - `hits + misses > 0` is the only external sign the instrument ran at all. Assert it, and say so
///   in the message, or the next reader deletes it as redundant.
///
/// `tests/render.rs::text_cache_hits_a_moving_centered_text` is that check.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextCacheStats {
    /// Plain draws served from the shaped-buffer cache this frame.
    pub hits: u32,
    /// Plain draws that had to be shaped from scratch this frame.
    pub misses: u32,
}

impl TextCacheStats {
    /// Plain draws seen this frame — `hits + misses`. Zero means the text pass saw nothing, which
    /// is not the same as a perfect cache.
    pub fn total(self) -> u32 {
        self.hits + self.misses
    }
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
