/// A single entry in a 1D blend tree: parameter threshold and the clip index to play.
#[derive(Debug, Clone)]
pub struct BlendEntry {
    /// This clip is selected when `BlendTree1D::param` is greater than or equal to this value.
    pub threshold: f32,
    pub clip_index: usize,
}

/// Component that automatically switches `AnimationPlayer` clips via a 1D parameter.
///
/// Keep `entries` sorted by threshold in ascending order so the closest clip is
/// selected according to `param`. When the clip changes it cross-fades smoothly
/// over `crossfade_duration`.
///
/// # Registration order
/// ```text
/// app.add_system(Box::new(BlendTreeSystem));  // clip selection
/// app.add_system(Box::new(AnimationSystem));  // frame advance
/// ```
///
/// # Interaction with `AnimationStateMachine`
/// `StateMachineSystem` runs **after** `BlendTreeSystem` in the documented system
/// order. When both components drive the same `AnimationPlayer`, a state-machine
/// transition will **interrupt** any in-progress `BlendTree1D` crossfade — the SM
/// wins because state changes are semantic. This is intentional. Do not attach both
/// `AnimationStateMachine` and `BlendTree1D` to the same entity simultaneously
/// unless that interruption behaviour is desired.
///
/// # Example
/// ```rust,ignore
/// let tree = BlendTree1D::new(
///     vec![
///         BlendEntry { threshold: 0.0, clip_index: 0 },  // idle
///         BlendEntry { threshold: 0.5, clip_index: 1 },  // walk
///         BlendEntry { threshold: 1.5, clip_index: 2 },  // run
///     ],
///     0.15,  // 0.15-second crossfade
/// );
/// world.add_component(entity, tree);
///
/// // Update the speed parameter each frame
/// world.get_mut::<BlendTree1D>(entity).unwrap().set_param(speed);
/// ```
#[derive(Debug, Clone)]
pub struct BlendTree1D {
    /// Must be sorted in ascending threshold order.
    pub entries: Vec<BlendEntry>,
    /// Current parameter value. Updated via `set_param()`.
    pub param: f32,
    /// Crossfade duration in seconds when switching clips. 0 means an instant switch.
    pub crossfade_duration: f32,
    // Last selected clip tracked by BlendTreeSystem to suppress duplicate requests.
    pub(crate) last_clip: Option<usize>,
}

impl BlendTree1D {
    /// `entries` must be passed in ascending threshold order.
    pub fn new(entries: Vec<BlendEntry>, crossfade_duration: f32) -> Self {
        Self {
            entries,
            param: 0.0,
            crossfade_duration,
            last_clip: None,
        }
    }

    /// Sets the parameter value. `BlendTreeSystem` will update the clip on the next frame.
    pub fn set_param(&mut self, param: f32) {
        self.param = param;
    }

    /// Returns the clip index that should be selected given the current param.
    /// Returns `None` if entries is empty.
    pub fn target_clip(&self) -> Option<usize> {
        if self.entries.is_empty() {
            return None;
        }
        // Select the entry with the largest threshold where threshold ≤ param
        let mut result = &self.entries[0];
        for entry in &self.entries {
            if entry.threshold <= self.param {
                result = entry;
            }
        }
        Some(result.clip_index)
    }
}
