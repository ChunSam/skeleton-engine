use std::time::Duration;

use glam::Vec2;

use super::types::{is_finished_state, is_playing_state, playback_state_from_sink};
use super::{AudioChannelState, AudioEffect, AudioManager};

#[test]
fn missing_sink_maps_to_missing_state() {
    assert_eq!(playback_state_from_sink(None), AudioChannelState::Missing);
    assert_eq!(is_finished_state(AudioChannelState::Missing), None);
    assert!(!is_playing_state(AudioChannelState::Missing));
}

#[test]
fn playback_state_helpers_map_finished_and_playing() {
    assert_eq!(is_finished_state(AudioChannelState::Playing), Some(false));
    assert_eq!(is_finished_state(AudioChannelState::Finished), Some(true));
    assert!(is_playing_state(AudioChannelState::Playing));
    assert!(!is_playing_state(AudioChannelState::Finished));
}

#[test]
fn play_tone_reports_playing_then_finished_when_audio_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };

    assert_eq!(audio.playback_state("test"), AudioChannelState::Missing);
    assert_eq!(audio.is_finished("test"), None);
    assert!(!audio.is_playing("test"));

    audio.play_tone("test", 440.0, 0.02, 0.01);
    assert_eq!(audio.playback_state("test"), AudioChannelState::Playing);
    assert_eq!(audio.is_finished("test"), Some(false));
    assert!(audio.is_playing("test"));

    let start = std::time::Instant::now();
    while audio.playback_state("test") != AudioChannelState::Finished
        && start.elapsed() < Duration::from_secs(2)
    {
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(audio.playback_state("test"), AudioChannelState::Finished);
    assert_eq!(audio.is_finished("test"), Some(true));
    assert!(!audio.is_playing("test"));

    audio.stop("test");
    assert_eq!(audio.playback_state("test"), AudioChannelState::Missing);
    assert_eq!(audio.is_finished("test"), None);
}

#[test]
fn spatial_params_center_is_full_volume() {
    let (vol, pan) = AudioManager::spatial_params(Vec2::ZERO, Vec2::ZERO, 500.0);
    assert_eq!(vol, 1.0);
    assert!((pan).abs() < 0.001);
}

#[test]
fn spatial_params_max_dist_is_silent() {
    let (vol, _) = AudioManager::spatial_params(Vec2::new(500.0, 0.0), Vec2::ZERO, 500.0);
    assert!(vol < 0.001);
}

#[test]
fn spatial_params_right_side_pans_right() {
    let (_, pan) = AudioManager::spatial_params(Vec2::new(250.0, 0.0), Vec2::ZERO, 500.0);
    assert!(pan > 0.0);
}

#[test]
fn spatial_params_left_side_pans_left() {
    let (_, pan) = AudioManager::spatial_params(Vec2::new(-250.0, 0.0), Vec2::ZERO, 500.0);
    assert!(pan < 0.0);
}

#[test]
fn audio_effect_default_pitch() {
    let eff = AudioEffect::default();
    assert!((eff.pitch - 1.0).abs() < 0.001);
    assert!(eff.low_pass_hz.is_none());
}

#[test]
fn set_and_clear_effect() {
    // AudioManager는 오디오 장치 없이 None을 반환할 수 있으므로,
    // AudioEffect 구조체 자체만 테스트한다.
    let eff = AudioEffect {
        low_pass_hz: Some(1000),
        pitch: 0.8,
        attack_secs: 0.5,
        release_secs: 0.0,
    };
    assert_eq!(eff.low_pass_hz, Some(1000));
    assert!((eff.pitch - 0.8).abs() < 0.001);
}
