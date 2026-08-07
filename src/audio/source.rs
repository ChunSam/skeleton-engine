use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rodio::{ChannelCount, Sample, SampleRate, Source};

// ─── Stereo pan source wrapper ────────────────────────────────────────────────

/// A shared, live-writable pan in `-1.0..=1.0`, stored as `f32` bits.
///
/// The pan used to be baked into `left_vol`/`right_vol` at construction, so it was fixed for the
/// lifetime of the sound: `update_position` recomputed a pan every frame, wrote it into
/// `AudioManager::pans`, and it only took effect the *next* time the channel was played. A
/// positional sound therefore tracked the listener in VOLUME while its stereo image stayed frozen
/// wherever it started — the web `StereoPannerNode` path repositioned correctly, so the same game
/// sounded different on the two platforms.
pub(super) type PanHandle = Arc<AtomicU32>;

/// Packs a pan into the shared handle's bit representation.
pub(super) fn pack_pan(pan: f32) -> u32 {
    pan.clamp(-1.0, 1.0).to_bits()
}

/// Creates a handle initialised to `pan`.
pub(super) fn pan_handle(pan: f32) -> PanHandle {
    Arc::new(AtomicU32::new(pack_pan(pan)))
}

pub(super) struct PannedSource<S: Source> {
    pub(super) inner: S,
    pub(super) pan: PanHandle,
    pub(super) current_channel: u16,
    pub(super) total_channels: u16,
}

impl<S: Source> PannedSource<S> {
    pub(super) fn new(inner: S, pan: PanHandle) -> Self {
        let total_channels = inner.channels().get();
        Self {
            pan,
            inner,
            current_channel: 0,
            total_channels,
        }
    }

    /// Current (left, right) gains, read fresh per sample so a live pan write is heard.
    fn gains(&self) -> (f32, f32) {
        let pan = f32::from_bits(self.pan.load(Ordering::Relaxed));
        ((1.0 - pan).clamp(0.0, 1.0), (1.0 + pan).clamp(0.0, 1.0))
    }
}

impl<S: Source + Clone> Clone for PannedSource<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            pan: Arc::clone(&self.pan),
            current_channel: self.current_channel,
            total_channels: self.total_channels,
        }
    }
}

impl<S: Source> Iterator for PannedSource<S> {
    type Item = Sample;
    fn next(&mut self) -> Option<Sample> {
        let sample = self.inner.next()?;
        let channels = self.total_channels;
        let (left_vol, right_vol) = self.gains();
        // NOTE: a MONO source still cannot be panned — `(l + r) * 0.5` is always exactly 1.0,
        // since `l + r == 2` for every pan. Panning mono would mean upmixing to stereo, which
        // changes `channels()` for every sound in the engine and cannot be verified without a
        // real output device; it is deliberately left alone here and recorded in the changelog.
        let vol = if channels < 2 {
            (left_vol + right_vol) * 0.5
        } else if self.current_channel == 0 {
            left_vol
        } else {
            right_vol
        };
        self.current_channel = (self.current_channel + 1) % channels.max(1);
        Some(sample * vol)
    }
}

impl<S: Source> Source for PannedSource<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }
    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }
    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::buffer::SamplesBuffer;

    const RATE: SampleRate = match SampleRate::new(48_000) {
        Some(r) => r,
        None => panic!("48 kHz is non-zero"),
    };
    const STEREO: ChannelCount = match ChannelCount::new(2) {
        Some(c) => c,
        None => panic!("2 is non-zero"),
    };
    const MONO: ChannelCount = ChannelCount::MIN;

    /// Writing the shared handle must change the gains of a sound **already playing**.
    ///
    /// The pan used to be baked into `left_vol`/`right_vol` at construction, so it was fixed for
    /// the sound's lifetime: `update_position` recomputed a pan every frame and it only took
    /// effect the *next* time the channel played. A positional sound therefore tracked the
    /// listener in volume while its stereo image stayed frozen wherever it started — and the web
    /// `StereoPannerNode` path repositioned correctly, so the same game sounded different on the
    /// two platforms.
    #[test]
    fn writing_the_handle_repositions_a_playing_sound() {
        // Stereo, all-ones: each output sample IS the channel gain.
        let buf = SamplesBuffer::new(STEREO, RATE, vec![1.0f32; 8]);
        let handle = pan_handle(0.0);
        let mut src = PannedSource::new(buf, Arc::clone(&handle));

        // Centred: both channels unity.
        let (l, r) = (src.next().unwrap(), src.next().unwrap());
        assert!((l - 1.0).abs() < 1e-6 && (r - 1.0).abs() < 1e-6, "{l} {r}");

        // Hard left WHILE the source is mid-stream. Gains are `clamp(0, 1)`, so left stays at
        // unity and right is silenced.
        handle.store(pack_pan(-1.0), Ordering::Relaxed);
        let (l, r) = (src.next().unwrap(), src.next().unwrap());
        assert!(
            (l - 1.0).abs() < 1e-6 && r.abs() < 1e-6,
            "pan did not take effect mid-stream: {l} {r}"
        );

        // Hard right, still mid-stream.
        handle.store(pack_pan(1.0), Ordering::Relaxed);
        let (l, r) = (src.next().unwrap(), src.next().unwrap());
        assert!(l.abs() < 1e-6 && (r - 1.0).abs() < 1e-6, "{l} {r}");
    }

    /// The centred path must stay arithmetically identical now that every sound is wrapped.
    #[test]
    fn pan_zero_is_unity_gain() {
        let buf = SamplesBuffer::new(STEREO, RATE, vec![0.5f32, -0.25, 0.75, -1.0]);
        let src = PannedSource::new(buf, pan_handle(0.0));
        let out: Vec<f32> = src.collect();
        assert_eq!(out, vec![0.5, -0.25, 0.75, -1.0]);
    }

    /// Documents the known limitation, precisely: panning a MONO source does not move it in the
    /// stereo image at all — it only **attenuates** it, because the mono branch averages the two
    /// gains and both are `clamp(0, 1)`. At hard pan that is a 0.5x volume cut with no
    /// directional cue whatsoever.
    ///
    /// Real mono panning needs a stereo upmix, which changes `channels()` for every sound in the
    /// engine and cannot be verified without a real output device; it is deliberately out of
    /// scope here and recorded in the changelog instead of being half-done.
    #[test]
    fn mono_pan_only_attenuates() {
        let buf = SamplesBuffer::new(MONO, RATE, vec![1.0f32; 4]);
        let out: Vec<f32> = PannedSource::new(buf, pan_handle(-1.0)).collect();
        assert_eq!(
            out,
            vec![0.5; 4],
            "mono hard-pan attenuates rather than pans"
        );

        let buf = SamplesBuffer::new(MONO, RATE, vec![1.0f32; 4]);
        let centred: Vec<f32> = PannedSource::new(buf, pan_handle(0.0)).collect();
        assert_eq!(centred, vec![1.0; 4], "centred mono is untouched");
    }
}
