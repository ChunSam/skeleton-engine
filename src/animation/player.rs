// ─── UV coordinates ───────────────────────────────────────────────────────────

// `UvRect` and `BlendUv` moved to `crate::renderer::uv` — they are pure GPU UV
// types consumed engine-wide (atlas, tilemap, sprite/UI renderers), not
// animation concepts. Re-exported here so existing
// `animation::player::{UvRect, BlendUv}` paths keep compiling.
pub use crate::renderer::uv::{BlendUv, UvRect};

// ─── Blend-weight component ───────────────────────────────────────────────────

/// Component indicating crossfade progress. Updated every frame by `AnimationSystem`.
///
/// - `1.0`: no crossfade (or completed)
/// - `0.0 ~ 1.0`: transition in progress (0 = from clip, 1 = to clip)
///
/// Game code can use this for sprite-alpha interpolation and similar effects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlendWeight(pub f32);

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
