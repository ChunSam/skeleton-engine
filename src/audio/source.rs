use std::time::Duration;

use rodio::Source;

// ─── 스테레오 팬 소스 래퍼 ────────────────────────────────────────────────────

pub(super) struct PannedSource<S: Source<Item = f32>> {
    pub(super) inner: S,
    pub(super) left_vol: f32,
    pub(super) right_vol: f32,
    pub(super) current_channel: u16,
    pub(super) total_channels: u16,
}

impl<S: Source<Item = f32>> PannedSource<S> {
    pub(super) fn new(inner: S, pan: f32) -> Self {
        let total_channels = inner.channels();
        Self {
            left_vol: (1.0 - pan).clamp(0.0, 1.0),
            right_vol: (1.0 + pan).clamp(0.0, 1.0),
            inner,
            current_channel: 0,
            total_channels,
        }
    }
}

impl<S: Source<Item = f32> + Clone> Clone for PannedSource<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            left_vol: self.left_vol,
            right_vol: self.right_vol,
            current_channel: self.current_channel,
            total_channels: self.total_channels,
        }
    }
}

impl<S: Source<Item = f32>> Iterator for PannedSource<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        let channels = self.total_channels;
        let vol = if channels < 2 {
            (self.left_vol + self.right_vol) * 0.5
        } else if self.current_channel == 0 {
            self.left_vol
        } else {
            self.right_vol
        };
        self.current_channel = (self.current_channel + 1) % channels.max(1);
        Some(sample * vol)
    }
}

impl<S: Source<Item = f32>> Source for PannedSource<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.inner.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}
