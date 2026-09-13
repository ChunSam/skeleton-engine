//! Shared spatial-audio math, compiled on all targets (native and wasm32).
//!
//! Both [`crate::audio::AudioManager`] (native) and [`crate::audio_wasm::WebAudio`] (wasm) compute
//! the same distance-based volume and stereo-pan values. This module provides a single canonical
//! implementation for those control values, not identical output mixing. Native decoded clips use
//! linear stereo balance (mono clips only attenuate); Web Audio uses `StereoPannerNode`, which
//! upmixes mono and crossfeeds stereo input. Both preserve centred stereo input, but their output
//! away from centre differs.

use glam::Vec2;

/// Independent user/fade volume and distance attenuation, shared by both backends.
/// A non-positional sound starts with unit attenuation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChannelVolume {
    pub base: f32,
    pub spatial: f32,
}

impl Default for ChannelVolume {
    fn default() -> Self {
        Self {
            base: 1.0,
            spatial: 1.0,
        }
    }
}

impl ChannelVolume {
    pub fn set_base(&mut self, base: f32) {
        self.base = base.clamp(0.0, 1.0);
    }

    pub fn set_spatial(&mut self, spatial: f32) {
        self.spatial = spatial;
    }

    /// Substitute an interpolated fade value without baking attenuation into the fade.
    pub fn gain_at(self, base: f32) -> f32 {
        base * self.spatial
    }

    pub fn gain(self) -> f32 {
        self.gain_at(self.base)
    }
}

/// Computes `(volume, pan)` for a positional sound.
///
/// - **`volume`** falls off linearly: `1.0` at the listener position, `0.0` at `max_dist` and
///   beyond.
/// - **`pan`** follows the x-axis offset: `-1.0` (full left) … `0.0` (centre) … `1.0` (full
///   right), clamped.
///
/// Both the native [`AudioManager`](crate::audio::AudioManager) and the wasm
/// [`WebAudio`](crate::audio_wasm::WebAudio) use this function via their respective
/// `play_at` / `update_position` paths so the two implementations stay in sync.
pub(crate) fn spatial_params(source: Vec2, listener: Vec2, max_dist: f32) -> (f32, f32) {
    let delta = source - listener;
    let dist = delta.length();
    let max = max_dist.max(0.001);
    let volume = (1.0 - (dist / max).min(1.0)).max(0.0);
    let pan = (delta.x / max).clamp(-1.0, 1.0);
    (volume, pan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spatial_volume_updates_commute_and_recover_from_silence() {
        let mut first = ChannelVolume::default();
        first.set_base(0.4);
        first.set_spatial(0.5);
        let mut second = ChannelVolume::default();
        second.set_spatial(0.5);
        second.set_base(0.4);
        assert_eq!(first, second);
        assert_eq!(first.gain(), 0.2);
        first.set_spatial(0.0);
        first.set_base(0.8);
        assert_eq!(first.gain(), 0.0);
        first.set_spatial(0.25);
        assert_eq!(first.gain(), 0.2);
        assert_eq!(first.gain_at(0.4), 0.1);
        assert_eq!(
            first.base, 0.8,
            "fade sampling must not rewrite resting volume"
        );
    }

    #[test]
    fn spatial_volume_defaults_and_clamps_only_user_volume() {
        let mut volume = ChannelVolume::default();
        assert_eq!(volume.gain(), 1.0);
        volume.set_base(2.0);
        volume.set_spatial(0.5);
        assert_eq!(volume.gain(), 0.5);
        volume.set_base(-1.0);
        assert_eq!(volume.gain(), 0.0);
    }

    #[test]
    fn spatial_params_at_listener_is_full_volume_center_pan() {
        let (vol, pan) = spatial_params(Vec2::ZERO, Vec2::ZERO, 100.0);
        assert!(
            (vol - 1.0).abs() < 1e-6,
            "at listener: volume should be 1.0, got {vol}"
        );
        assert!(
            pan.abs() < 1e-6,
            "at listener: pan should be 0.0, got {pan}"
        );
    }

    #[test]
    fn spatial_params_at_max_dist_is_silent() {
        let (vol, _) = spatial_params(Vec2::new(100.0, 0.0), Vec2::ZERO, 100.0);
        assert!(vol < 1e-6, "at max_dist: volume should be 0.0, got {vol}");
    }

    #[test]
    fn spatial_params_beyond_max_dist_clamps_to_zero() {
        let (vol, _) = spatial_params(Vec2::new(200.0, 0.0), Vec2::ZERO, 100.0);
        assert!(
            vol < 1e-6,
            "beyond max_dist: volume should be 0.0, got {vol}"
        );
    }

    #[test]
    fn spatial_params_pan_clamps_to_unit_range() {
        let (_, pan_right) = spatial_params(Vec2::new(999.0, 0.0), Vec2::ZERO, 100.0);
        let (_, pan_left) = spatial_params(Vec2::new(-999.0, 0.0), Vec2::ZERO, 100.0);
        assert!(
            (pan_right - 1.0).abs() < 1e-6,
            "far right: pan should clamp to 1.0, got {pan_right}"
        );
        assert!(
            (pan_left + 1.0).abs() < 1e-6,
            "far left: pan should clamp to -1.0, got {pan_left}"
        );
    }

    #[test]
    fn spatial_params_midpoint_half_volume() {
        let (vol, _) = spatial_params(Vec2::new(50.0, 0.0), Vec2::ZERO, 100.0);
        assert!(
            (vol - 0.5).abs() < 1e-6,
            "at half max_dist: volume should be 0.5, got {vol}"
        );
    }

    #[test]
    fn spatial_params_zero_max_dist_does_not_panic() {
        // max_dist=0 is clamped to 0.001 internally; should not divide by zero.
        let (vol, pan) = spatial_params(Vec2::new(1.0, 0.0), Vec2::ZERO, 0.0);
        // source is beyond effective max_dist (0.001), so volume=0 and pan is clamped.
        assert!(vol < 1e-6, "zero max_dist: volume should be 0.0, got {vol}");
        assert!(
            (pan - 1.0).abs() < 1e-6,
            "zero max_dist: pan should be 1.0, got {pan}"
        );
    }
}
