// ─── UV coordinates ───────────────────────────────────────────────────────────

/// Represents a single frame region within a texture as UV coordinates.
///
/// Example: frame at (col 2, row 1) of a 4-column × 2-row spritesheet:
/// `UvRect::from_grid(2, 1, 4, 2)`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UvRect {
    pub u_offset: f32,
    pub v_offset: f32,
    pub u_size: f32,
    pub v_size: f32,
}

impl UvRect {
    /// Default value covering the entire texture.
    pub const FULL: Self = Self {
        u_offset: 0.0,
        v_offset: 0.0,
        u_size: 1.0,
        v_size: 1.0,
    };

    /// Creates a region from normalized UV coordinates.
    pub const fn new(u_offset: f32, v_offset: f32, u_size: f32, v_size: f32) -> Self {
        Self {
            u_offset,
            v_offset,
            u_size,
            v_size,
        }
    }

    /// Computes the UV for a specific frame in a uniform-grid spritesheet.
    pub fn from_grid(col: u32, row: u32, cols: u32, rows: u32) -> Self {
        if cols == 0 || rows == 0 {
            return Self::FULL;
        }
        let u_size = 1.0 / cols as f32;
        let v_size = 1.0 / rows as f32;
        Self {
            u_offset: col as f32 * u_size,
            v_offset: row as f32 * v_size,
            u_size,
            v_size,
        }
    }

    /// Converts a pixel-space crop region to normalized UV.
    pub fn from_pixels(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        texture_width: f32,
        texture_height: f32,
    ) -> Self {
        if texture_width <= 0.0 || texture_height <= 0.0 {
            return Self::FULL;
        }
        Self {
            u_offset: x / texture_width,
            v_offset: y / texture_height,
            u_size: width / texture_width,
            v_size: height / texture_height,
        }
    }

    /// Samples the same region flipped horizontally.
    pub fn flipped_x(mut self) -> Self {
        self.u_offset += self.u_size;
        self.u_size = -self.u_size;
        self
    }

    /// Samples the same region flipped vertically.
    pub fn flipped_y(mut self) -> Self {
        self.v_offset += self.v_size;
        self.v_size = -self.v_size;
        self
    }
}

#[cfg(test)]
mod uv_tests {
    use super::UvRect;

    #[test]
    fn from_grid_row_zero_is_top_row() {
        assert_eq!(
            UvRect::from_grid(0, 0, 4, 2),
            UvRect::new(0.0, 0.0, 0.25, 0.5)
        );
        assert_eq!(
            UvRect::from_grid(1, 0, 4, 2),
            UvRect::new(0.25, 0.0, 0.25, 0.5)
        );
        assert_eq!(
            UvRect::from_grid(0, 1, 4, 2),
            UvRect::new(0.0, 0.5, 0.25, 0.5)
        );
    }

    #[test]
    fn from_pixels_uses_top_left_origin() {
        let uv = UvRect::from_pixels(10.0, 20.0, 30.0, 40.0, 100.0, 200.0);
        assert_eq!(uv, UvRect::new(0.1, 0.1, 0.3, 0.2));
    }

    #[test]
    fn flips_keep_same_sampled_area_with_negative_size() {
        let top_row = UvRect::from_grid(1, 0, 4, 2);
        assert_eq!(top_row.flipped_y(), UvRect::new(0.25, 0.5, 0.25, -0.5));

        let uv = UvRect::new(0.1, 0.2, 0.3, 0.4).flipped_y();
        assert_eq!(uv, UvRect::new(0.1, 0.6, 0.3, -0.4));

        let uv = UvRect::new(0.1, 0.2, 0.3, 0.4).flipped_x();
        assert_eq!(uv, UvRect::new(0.4, 0.2, -0.3, 0.4));
    }
}

// ─── Blend-weight component ───────────────────────────────────────────────────

/// Component indicating crossfade progress. Updated every frame by `AnimationSystem`.
///
/// - `1.0`: no crossfade (or completed)
/// - `0.0 ~ 1.0`: transition in progress (0 = from clip, 1 = to clip)
///
/// Game code can use this for sprite-alpha interpolation and similar effects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlendWeight(pub f32);

/// Component used by the renderer to alpha-lerp two frames during a crossfade.
///
/// Updated every frame by `AnimationSystem`. During a crossfade `to` holds the current
/// frame UV of the to-clip and `weight` holds the progress (0.0→1.0). When no transition
/// is active `weight = 0.0` (`to` equals from) and the renderer treats it as a single
/// frame. The sprite shader composites both frames via `mix(from_uv, to_uv, weight)` to
/// produce a smooth crossfade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlendUv {
    /// Current frame UV of the to-clip.
    pub to: UvRect,
    /// Crossfade progress [0.0..=1.0]. 0 means no blend (single frame).
    pub weight: f32,
}

// ─── Crossfade state ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct CrossfadeState {
    pub to_clip: usize,
    pub to_frame: usize,
    pub to_timer: f32,
    pub elapsed: f32,
    pub duration: f32,
}

// ─── Animation data ───────────────────────────────────────────────────────────

/// A single animation clip: list of frames and playback speed.
#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub frames: Vec<UvRect>,
    pub fps: f32,
    pub looping: bool,
}

/// Animation player component attached to an entity.
#[derive(Debug, Clone)]
pub struct AnimationPlayer {
    pub clips: Vec<AnimationClip>,
    pub current_clip: usize,
    pub current_frame: usize,
    /// Accumulated time (seconds) until the next frame.
    pub timer: f32,
    pub(crate) crossfade: Option<CrossfadeState>,
}

impl AnimationPlayer {
    pub fn new(clips: Vec<AnimationClip>) -> Self {
        Self {
            clips,
            current_clip: 0,
            current_frame: 0,
            timer: 0.0,
            crossfade: None,
        }
    }

    /// Switches to a clip immediately. Does nothing if that clip is already playing.
    pub fn play(&mut self, clip_index: usize) {
        if self.current_clip != clip_index {
            self.current_clip = clip_index;
            self.current_frame = 0;
            self.timer = 0.0;
            self.crossfade = None;
        }
    }

    /// Switches to a clip with a smooth crossfade over `duration` seconds.
    ///
    /// The `BlendWeight` component is updated from 0.0→1.0 during the transition.
    /// If `duration <= 0.0` the switch is immediate (same as `play`).
    pub fn play_with_crossfade(&mut self, clip_index: usize, duration: f32) {
        if self.current_clip == clip_index {
            return;
        }
        if duration <= 0.0 {
            self.play(clip_index);
            return;
        }
        self.crossfade = Some(CrossfadeState {
            to_clip: clip_index,
            to_frame: 0,
            to_timer: 0.0,
            elapsed: 0.0,
            duration,
        });
    }

    /// Crossfade progress [0.0..=1.0]. Returns `1.0` when no transition is active.
    pub fn blend_weight(&self) -> f32 {
        match &self.crossfade {
            None => 1.0,
            Some(cf) => (cf.elapsed / cf.duration).clamp(0.0, 1.0),
        }
    }

    /// Returns `true` while a crossfade transition is in progress.
    pub fn is_crossfading(&self) -> bool {
        self.crossfade.is_some()
    }

    /// Returns the UV of the current frame. Falls back to the full texture if there are no clips or frames.
    pub fn current_uv(&self) -> UvRect {
        self.clips
            .get(self.current_clip)
            .and_then(|c| c.frames.get(self.current_frame))
            .copied()
            .unwrap_or(UvRect::FULL)
    }

    /// Returns whether the current clip has finished. Always `false` for looping clips.
    pub fn is_finished(&self) -> bool {
        let Some(clip) = self.clips.get(self.current_clip) else {
            return true;
        };
        if clip.looping || clip.frames.is_empty() {
            return false;
        }
        self.current_frame >= clip.frames.len() - 1
    }
}
