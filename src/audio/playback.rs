use std::fs::File;
use std::io::{BufReader, Cursor};
use std::time::Duration;

use rodio::source::SineWave;
use rodio::{Decoder, Sink, Source};

use super::source::PannedSource;
use super::types::{is_finished_state, is_playing_state, playback_state_from_sink};
use super::{AudioChannelState, AudioManager};

impl AudioManager {
    /// 오디오 장치를 초기화한다. 실패 시 `None` 반환, 게임은 무음으로 계속 실행된다.
    pub fn new() -> Option<Self> {
        use rodio::OutputStream;
        use std::collections::HashMap;
        match OutputStream::try_default() {
            Ok((_stream, stream_handle)) => Some(Self {
                _stream,
                stream_handle,
                sinks: HashMap::new(),
                volume_overrides: HashMap::new(),
                pans: HashMap::new(),
                bus_volumes: HashMap::new(),
                channel_buses: HashMap::new(),
                fades: HashMap::new(),
                effects: HashMap::new(),
            }),
            Err(e) => {
                log::warn!("오디오 초기화 실패 (오디오 없이 실행됩니다): {e}");
                None
            }
        }
    }

    // ── 기본 재생 ─────────────────────────────────────────────────────────────

    /// 오디오 파일을 채널에서 재생한다. 같은 채널이 있으면 먼저 정지한다.
    pub fn play(&mut self, channel: &str, path: &str, repeat: bool) {
        self.play_internal(channel, path, repeat, None);
    }

    /// 페이드인을 적용해 재생한다.
    pub fn play_fade_in(&mut self, channel: &str, path: &str, repeat: bool, fade_secs: f32) {
        self.play_internal(channel, path, repeat, Some(fade_secs));
    }

    /// 채널 재생을 즉시 정지한다.
    pub fn stop(&mut self, channel: &str) {
        self.fades.remove(channel);
        if let Some(sink) = self.sinks.remove(channel) {
            sink.stop();
        }
    }

    /// Returns the playback state for a channel.
    ///
    /// A missing channel reports [`AudioChannelState::Missing`]. A non-looping
    /// sound that naturally reaches the end remains queryable as
    /// [`AudioChannelState::Finished`] until the channel is stopped or reused.
    pub fn playback_state(&self, channel: &str) -> AudioChannelState {
        playback_state_from_sink(self.sinks.get(channel))
    }

    /// Returns whether a channel has finished playback.
    ///
    /// `None` means the channel has no sink. `Some(false)` means it still has
    /// audio queued. `Some(true)` means it exists and has drained.
    pub fn is_finished(&self, channel: &str) -> Option<bool> {
        is_finished_state(self.playback_state(channel))
    }

    /// Returns true when a channel exists and still has queued audio.
    pub fn is_playing(&self, channel: &str) -> bool {
        is_playing_state(self.playback_state(channel))
    }

    /// 순수 사인파 톤을 재생한다.
    ///
    /// `volume` 은 톤 자체의 진폭이고, 채널이 속한 버스 볼륨은 `play_internal` 과
    /// 동일하게 sink 볼륨(`effective_volume`)으로 곱해진다. `set_effect` 로 지정한
    /// 채널 이펙트(로우패스·피치·페이드인)도 톤에 적용된다.
    pub fn play_tone(&mut self, channel: &str, freq: f32, duration_secs: f32, volume: f32) {
        if let Some(old) = self.sinks.remove(channel) {
            old.stop();
        }
        let sink = match Sink::try_new(&self.stream_handle) {
            Ok(s) => s,
            Err(_) => return,
        };
        // 버스/채널 볼륨을 sink 에 반영 (set_bus_volume 이 즉시 갱신할 수 있도록).
        sink.set_volume(self.effective_volume(channel));

        let base = SineWave::new(freq)
            .take_duration(Duration::from_secs_f32(duration_secs))
            .amplify(volume);

        // SineWave 는 f32 샘플이라 low_pass/speed/fade_in 을 변환 없이 직접 적용한다.
        let source: Box<dyn Source<Item = f32> + Send + 'static> =
            match self.effects.get(channel).cloned() {
                Some(eff) => {
                    let pitch = if eff.pitch > 0.0 { eff.pitch } else { 1.0 };
                    let s = base.speed(pitch);
                    match (eff.low_pass_hz, eff.attack_secs) {
                        (Some(hz), a) if a > 0.001 => {
                            Box::new(s.low_pass(hz).fade_in(Duration::from_secs_f32(a)))
                        }
                        (Some(hz), _) => Box::new(s.low_pass(hz)),
                        (None, a) if a > 0.001 => Box::new(s.fade_in(Duration::from_secs_f32(a))),
                        (None, _) => Box::new(s),
                    }
                }
                None => Box::new(base),
            };
        sink.append(source);
        self.sinks.insert(channel.to_string(), sink);
    }

    /// 페이드 처리를 진행한다. 매 프레임 System에서 호출한다.
    ///
    /// `fade_out` / `fade_volume`을 사용할 경우 반드시 이 메서드를 호출해야 한다.
    pub fn update(&mut self, dt: f32) {
        let channels: Vec<String> = self.fades.keys().cloned().collect();
        for ch in channels {
            let done = {
                let fade = self.fades.get_mut(&ch).unwrap();
                fade.elapsed += dt;
                let t = (fade.elapsed / fade.duration).clamp(0.0, 1.0);
                let vol = fade.start_vol + (fade.target_vol - fade.start_vol) * t;
                if let Some(sink) = self.sinks.get(&ch) {
                    let bus_vol = self
                        .channel_buses
                        .get(&ch)
                        .and_then(|b| self.bus_volumes.get(b))
                        .copied()
                        .unwrap_or(1.0);
                    sink.set_volume(vol * bus_vol);
                }
                if t >= 1.0 {
                    let stop = fade.stop_when_done;
                    self.volume_overrides.insert(ch.clone(), fade.target_vol);
                    stop
                } else {
                    false
                }
            };
            if done {
                self.fades.remove(&ch);
                self.stop(&ch);
            } else if self
                .fades
                .get(&ch)
                .map(|f| f.elapsed >= f.duration)
                .unwrap_or(false)
            {
                self.fades.remove(&ch);
            }
        }
    }

    // ── 내부 헬퍼 ─────────────────────────────────────────────────────────────

    pub(super) fn play_internal(
        &mut self,
        channel: &str,
        path: &str,
        repeat: bool,
        fade_in_secs: Option<f32>,
    ) {
        if let Some(old) = self.sinks.remove(channel) {
            old.stop();
        }
        self.fades.remove(channel);

        let sink = match Sink::try_new(&self.stream_handle) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("오디오 싱크 생성 실패: {e}");
                return;
            }
        };

        let eff_vol = self.effective_volume(channel);
        sink.set_volume(eff_vol);

        let pan = self.pans.get(channel).copied().unwrap_or(0.0);

        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                log::warn!("오디오 파일을 열 수 없습니다 '{path}': {e}");
                return;
            }
        };
        let source = match Decoder::new(Cursor::new(bytes)) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("오디오 디코딩 실패 '{path}': {e}");
                return;
            }
        };

        // ── 이펙트 적용 ──────────────────────────────────────────────────────
        // Box<dyn Source<Item=i16> + Send>로 통일해 타입 복잡도를 줄인다.
        let effect = self.effects.get(channel).cloned();
        let effected: Box<dyn Source<Item = i16> + Send + 'static> = if let Some(eff) = effect {
            if (eff.pitch - 1.0).abs() > 0.001 {
                let s = source.speed(eff.pitch);
                if let Some(hz) = eff.low_pass_hz {
                    let s = s
                        .convert_samples::<f32>()
                        .low_pass(hz)
                        .convert_samples::<i16>();
                    if eff.attack_secs > 0.001 {
                        Box::new(s.fade_in(Duration::from_secs_f32(eff.attack_secs)))
                    } else {
                        Box::new(s)
                    }
                } else if eff.attack_secs > 0.001 {
                    Box::new(
                        s.convert_samples::<i16>()
                            .fade_in(Duration::from_secs_f32(eff.attack_secs)),
                    )
                } else {
                    Box::new(s.convert_samples::<i16>())
                }
            } else if let Some(hz) = eff.low_pass_hz {
                let s = source
                    .convert_samples::<f32>()
                    .low_pass(hz)
                    .convert_samples::<i16>();
                if eff.attack_secs > 0.001 {
                    Box::new(s.fade_in(Duration::from_secs_f32(eff.attack_secs)))
                } else {
                    Box::new(s)
                }
            } else if eff.attack_secs > 0.001 {
                Box::new(source.fade_in(Duration::from_secs_f32(eff.attack_secs)))
            } else {
                Box::new(source)
            }
        } else {
            Box::new(source)
        };

        // ── 팬 / 페이드인 / 반복 적용 ────────────────────────────────────────
        // 팬 없고 페이드인 없을 때는 BufReader 경로가 더 효율적이지만,
        // 여기서는 Cursor 경로로 통일 (이미 bytes로 읽었으므로 비용 동일)
        if pan.abs() > 0.001 {
            let panned = PannedSource::new(effected.convert_samples::<f32>(), pan);
            if let Some(fade_dur) = fade_in_secs {
                let faded = panned.fade_in(Duration::from_secs_f32(fade_dur));
                if repeat {
                    sink.append(faded.repeat_infinite());
                } else {
                    sink.append(faded);
                }
            } else if repeat {
                sink.append(panned.repeat_infinite());
            } else {
                sink.append(panned);
            }
        } else if let Some(fade_dur) = fade_in_secs {
            let faded = effected.fade_in(Duration::from_secs_f32(fade_dur));
            if repeat {
                sink.append(faded.repeat_infinite());
            } else {
                sink.append(faded);
            }
        } else if repeat {
            sink.append(effected.repeat_infinite());
        } else {
            sink.append(effected);
        }

        self.sinks.insert(channel.to_string(), sink);
    }

    /// 채널의 실효 볼륨 = 기본 볼륨 × 버스 볼륨
    pub(super) fn effective_volume(&self, channel: &str) -> f32 {
        let base = self.volume_overrides.get(channel).copied().unwrap_or(1.0);
        self.effective_volume_params(base, channel)
    }

    pub(super) fn effective_volume_params(&self, base: f32, channel: &str) -> f32 {
        let bus_vol = self
            .channel_buses
            .get(channel)
            .and_then(|b| self.bus_volumes.get(b))
            .copied()
            .unwrap_or(1.0);
        base * bus_vol
    }

    // ── 호환성 유지 (이전 직접 read+BufReader 패턴) ──────────────────────────

    /// `play` 의 낮은 수준 버전. 팬 없을 때 BufReader로 스트리밍한다.
    #[allow(dead_code)]
    fn play_streaming(&mut self, channel: &str, path: &str, repeat: bool) {
        if let Some(old) = self.sinks.remove(channel) {
            old.stop();
        }
        let sink = match Sink::try_new(&self.stream_handle) {
            Ok(s) => s,
            Err(_) => return,
        };
        sink.set_volume(self.effective_volume(channel));
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("오디오 파일을 열 수 없습니다 '{path}': {e}");
                return;
            }
        };
        let source = match Decoder::new(BufReader::new(file)) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("오디오 디코딩 실패 '{path}': {e}");
                return;
            }
        };
        if repeat {
            sink.append(source.repeat_infinite());
        } else {
            sink.append(source);
        }
        self.sinks.insert(channel.to_string(), sink);
    }
}
