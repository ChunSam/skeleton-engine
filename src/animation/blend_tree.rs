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
/// app.add_system(BlendTreeSystem::new());  // clip selection
/// app.add_system(AnimationSystem::new());  // frame advance
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
    /// Creates a new `BlendTree1D`. Entries are sorted by threshold ascending so that
    /// `target_clip()` always returns the correct result regardless of the order in
    /// which entries are supplied. Passing entries that are already sorted is fine;
    /// the sort is stable and O(n log n).
    pub fn new(entries: Vec<BlendEntry>, crossfade_duration: f32) -> Self {
        let mut entries = entries;
        entries.sort_by(|a, b| {
            a.threshold
                .partial_cmp(&b.threshold)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
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
        // Select the entry with the largest threshold where threshold ≤ param.
        // Relies on entries being sorted ascending (guaranteed by new()).
        let mut result = &self.entries[0];
        for entry in &self.entries {
            if entry.threshold <= self.param {
                result = entry;
            }
        }
        Some(result.clip_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(threshold: f32, clip_index: usize) -> BlendEntry {
        BlendEntry {
            threshold,
            clip_index,
        }
    }

    /// Entries supplied in descending order must be sorted by new() so that
    /// target_clip() returns the same results as for ascending-order construction.
    #[test]
    fn new_sorts_entries_ascending() {
        // Supply in reverse order: run (1.5), walk (0.5), idle (0.0).
        let tree = BlendTree1D::new(vec![entry(1.5, 2), entry(0.5, 1), entry(0.0, 0)], 0.2);
        // After sorting: thresholds must be 0.0, 0.5, 1.5.
        assert_eq!(tree.entries[0].threshold, 0.0);
        assert_eq!(tree.entries[1].threshold, 0.5);
        assert_eq!(tree.entries[2].threshold, 1.5);
    }

    /// target_clip() must pick the correct clip index regardless of input order.
    #[test]
    fn target_clip_correct_with_reversed_input() {
        // Reversed input: run at 1.5, walk at 0.5, idle at 0.0.
        let mut tree = BlendTree1D::new(vec![entry(1.5, 2), entry(0.5, 1), entry(0.0, 0)], 0.2);

        tree.set_param(-1.0);
        assert_eq!(
            tree.target_clip(),
            Some(0),
            "below all thresholds → idle (0)"
        );

        tree.set_param(0.0);
        assert_eq!(tree.target_clip(), Some(0), "at threshold 0.0 → idle (0)");

        tree.set_param(0.4);
        assert_eq!(
            tree.target_clip(),
            Some(0),
            "between idle and walk → idle (0)"
        );

        tree.set_param(0.5);
        assert_eq!(tree.target_clip(), Some(1), "at threshold 0.5 → walk (1)");

        tree.set_param(1.0);
        assert_eq!(
            tree.target_clip(),
            Some(1),
            "between walk and run → walk (1)"
        );

        tree.set_param(1.5);
        assert_eq!(tree.target_clip(), Some(2), "at threshold 1.5 → run (2)");

        tree.set_param(3.0);
        assert_eq!(
            tree.target_clip(),
            Some(2),
            "above all thresholds → run (2)"
        );
    }

    /// Ascending input must produce the same results (no regression).
    #[test]
    fn target_clip_correct_with_ascending_input() {
        let mut tree = BlendTree1D::new(vec![entry(0.0, 0), entry(0.5, 1), entry(1.5, 2)], 0.2);
        tree.set_param(0.6);
        assert_eq!(tree.target_clip(), Some(1));
        tree.set_param(2.0);
        assert_eq!(tree.target_clip(), Some(2));
    }

    /// Empty entries must return None.
    #[test]
    fn target_clip_empty_returns_none() {
        let tree = BlendTree1D::new(vec![], 0.0);
        assert_eq!(tree.target_clip(), None);
    }
}
