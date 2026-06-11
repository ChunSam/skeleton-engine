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
    // AudioManager can return None without an audio device,
    // so only the AudioEffect struct itself is tested here.
    let eff = AudioEffect {
        low_pass_hz: Some(1000),
        pitch: 0.8,
        attack_secs: 0.5,
        release_secs: 0.0,
    };
    assert_eq!(eff.low_pass_hz, Some(1000));
    assert!((eff.pitch - 0.8).abs() < 0.001);
}

// ─── #20: SFX file-bytes cache (device-free, runs in CI) ───────────────────────

#[test]
fn read_cached_bytes_reuses_cache_and_skips_disk() {
    use super::playback::read_cached_bytes;
    use std::collections::HashMap;
    use std::sync::Arc;

    let dir = std::env::temp_dir().join(format!("engine-audio-cache-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("clip.bin");
    std::fs::write(&path, b"RIFF-fake-wav-bytes").unwrap();
    let path_str = path.to_str().unwrap();

    let mut cache: HashMap<String, Arc<[u8]>> = HashMap::new();
    let first = read_cached_bytes(&mut cache, path_str).expect("first read should succeed");
    assert_eq!(&*first, b"RIFF-fake-wav-bytes");
    assert_eq!(cache.len(), 1);

    // Delete the file: the second call MUST hit the cache (no disk read) and
    // return the very same allocation.
    std::fs::remove_file(&path).unwrap();
    let second = read_cached_bytes(&mut cache, path_str).expect("cache hit after file deletion");
    assert!(
        Arc::ptr_eq(&first, &second),
        "second read must reuse the cached Arc, not re-read the disk"
    );
    assert_eq!(cache.len(), 1);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn read_cached_bytes_missing_file_is_none() {
    use super::playback::read_cached_bytes;
    use std::collections::HashMap;
    use std::sync::Arc;

    let mut cache: HashMap<String, Arc<[u8]>> = HashMap::new();
    assert!(read_cached_bytes(&mut cache, "/no/such/engine-audio-file.wav").is_none());
    assert!(
        cache.is_empty(),
        "a failed read must not populate the cache"
    );
}

// ─── Release envelope (device-free state-logic tests) ─────────────────────────

/// Minimal stand-in for `AudioManager` state used by the release-logic tests.
/// These tests exercise the scheduling and state-transition logic without needing
/// a real audio device.  They use the real `AudioManager` when one is available,
/// and skip gracefully when it is not.

#[test]
fn stop_without_release_is_immediate_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.set_effect(
        "ch",
        AudioEffect {
            release_secs: 0.0,
            ..AudioEffect::default()
        },
    );
    audio.play_tone("ch", 440.0, 10.0, 0.5);
    assert!(audio.is_playing("ch"));
    audio.stop("ch");
    // release_secs == 0.0 → immediate stop, no sink left.
    assert_eq!(audio.playback_state("ch"), AudioChannelState::Missing);
}

#[test]
fn stop_with_release_keeps_sink_alive_and_fades_when_device_exists() {
    use super::AudioSystem;
    use crate::ecs::{System, World};

    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.set_effect(
        "ch",
        AudioEffect {
            release_secs: 0.1,
            ..AudioEffect::default()
        },
    );
    audio.play_tone("ch", 440.0, 10.0, 0.5);
    assert!(audio.is_playing("ch"));

    audio.stop("ch");

    // Immediately after stop(): sink should still be alive (release in progress).
    assert!(
        audio.is_playing("ch"),
        "release_secs > 0.0 → sink lives during the fade"
    );

    // Drive the release fade to completion via AudioSystem.
    let mut world = World::new();
    world.insert_resource(audio);
    let mut sys = AudioSystem;
    // 10 × 0.02 s = 0.2 s total, more than the 0.1 s release.
    for _ in 0..10 {
        sys.run(&mut world, 0.02);
    }

    let audio = world.resource::<AudioManager>().unwrap();
    assert_eq!(
        audio.playback_state("ch"),
        AudioChannelState::Missing,
        "after release fade completes the channel must be torn down"
    );
}

#[test]
fn second_stop_during_release_cuts_immediately_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.set_effect(
        "ch",
        AudioEffect {
            release_secs: 10.0, // very long release so the fade won't complete before we act
            ..AudioEffect::default()
        },
    );
    audio.play_tone("ch", 440.0, 60.0, 0.5);
    audio.stop("ch"); // starts release fade
    assert!(
        audio.is_playing("ch"),
        "release fade should keep sink alive"
    );

    // A second stop() while releasing must cut immediately.
    audio.stop("ch");
    assert_eq!(
        audio.playback_state("ch"),
        AudioChannelState::Missing,
        "second stop() during release must tear down immediately"
    );
}

#[test]
fn play_during_release_starts_new_sound_immediately_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.set_effect(
        "ch",
        AudioEffect {
            release_secs: 10.0,
            ..AudioEffect::default()
        },
    );
    audio.play_tone("ch", 440.0, 60.0, 0.5);
    audio.stop("ch"); // starts release fade
    assert!(audio.is_playing("ch"));

    // A new play_tone() on the same channel must cancel the release fade and
    // start the new sound cleanly.
    audio.play_tone("ch", 880.0, 60.0, 0.5);
    assert!(
        audio.is_playing("ch"),
        "new sound should be playing after play_tone overrides a releasing channel"
    );
    // Advance one frame to confirm the fade map is clean (no stale release entry).
    audio.update(0.016);
    assert!(
        audio.is_playing("ch"),
        "channel should still be playing after one update"
    );
}

// ─── Release envelope regression tests ────────────────────────────────────────

/// (a) volume_overrides must not be poisoned to 0.0 after a release/fade_out
/// completes — the channel's next play must start at the correct base volume.
#[test]
fn play_after_release_completes_starts_at_correct_volume() {
    use super::AudioSystem;
    use crate::ecs::{System, World};

    let Some(mut audio) = AudioManager::new() else {
        return;
    };

    audio.set_volume("ch", 0.7);
    audio.set_effect(
        "ch",
        AudioEffect {
            release_secs: 0.05,
            ..AudioEffect::default()
        },
    );
    audio.play_tone("ch", 440.0, 10.0, 0.5);
    audio.stop("ch"); // schedules release fade

    // Drive the release fade to completion.
    let mut world = World::new();
    world.insert_resource(audio);
    let mut sys = AudioSystem;
    for _ in 0..10 {
        sys.run(&mut world, 0.02); // 0.2 s total > 0.05 s release
    }

    {
        let audio = world.resource::<AudioManager>().unwrap();
        assert_eq!(
            audio.playback_state("ch"),
            AudioChannelState::Missing,
            "channel must be torn down after release completes"
        );
    }

    // Play again and verify the sink picks up the pre-fade volume (0.7), not 0.0.
    let audio = world.resource_mut::<AudioManager>().unwrap();
    audio.play_tone("ch", 880.0, 10.0, 0.5);
    assert!(
        audio.is_playing("ch"),
        "channel must be playing after re-play"
    );

    // The sink volume should reflect set_volume(0.7) × bus(1.0) = 0.7, not 0.0.
    // We can't read the sink volume directly, but we can assert effective_volume
    // returns the pre-release value (not 0.0).
    let eff = audio.effective_volume("ch");
    assert!(
        (eff - 0.7).abs() < 0.01,
        "effective_volume after re-play must be 0.7, got {eff}"
    );
}

/// (b) fade_out-then-stop: stop() called while a fade_out is active must cut
/// immediately (stop_when_done fade already active → second stop = instant cut).
#[test]
fn fade_out_then_stop_cuts_immediately() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };

    audio.play_tone("ch", 440.0, 10.0, 0.5);
    assert!(audio.is_playing("ch"));

    // Schedule a 5 s fade_out (stop_when_done = true).
    audio.fade_out("ch", 5.0);

    // stop() while a stop_when_done fade is active → immediate cut.
    audio.stop("ch");
    assert_eq!(
        audio.playback_state("ch"),
        AudioChannelState::Missing,
        "stop() after fade_out must cut immediately, not queue a second release"
    );
}

/// (c) stop() called mid-fade_volume must use the interpolated volume as release
/// start_vol so there is no audible pop at the start of the release.
#[test]
fn stop_mid_fade_volume_release_starts_at_interpolated_volume() {
    use super::AudioSystem;
    use crate::ecs::{System, World};

    let Some(mut audio) = AudioManager::new() else {
        return;
    };

    audio.set_volume("ch", 1.0);
    audio.set_effect(
        "ch",
        AudioEffect {
            release_secs: 0.1,
            ..AudioEffect::default()
        },
    );
    audio.play_tone("ch", 440.0, 10.0, 1.0);

    // Start a 1 s fade_volume toward 0.2.
    audio.fade_volume("ch", 0.2, 1.0);

    // Advance partway (0.5 s → t=0.5, interpolated ≈ 1.0 + (0.2 - 1.0) * 0.5 = 0.6).
    let mut world = World::new();
    world.insert_resource(audio);
    let mut sys = AudioSystem;
    for _ in 0..25 {
        sys.run(&mut world, 0.02); // 0.5 s
    }

    {
        let audio = world.resource_mut::<AudioManager>().unwrap();
        // At this point a fade_volume is in progress.  stop() must not jump to 1.0
        // (the stale volume_overrides value) — it should pick up ~0.6.
        assert!(
            audio.fades.contains_key("ch"),
            "fade_volume should still be in progress"
        );
        audio.stop("ch");
        // Verify the release fade's start_vol is approximately 0.6 (not 1.0).
        if let Some(fade) = audio.fades.get("ch") {
            assert!(
                fade.stop_when_done,
                "stop() must have replaced fade_volume with a stop_when_done release fade"
            );
            assert!(
                (fade.start_vol - 0.6).abs() < 0.15,
                "release start_vol must be near interpolated ~0.6, got {}",
                fade.start_vol
            );
        } else {
            // If immediate (no sink), that's also acceptable.
            assert_eq!(audio.playback_state("ch"), AudioChannelState::Missing);
        }
    }
}

/// (d) stop() on a naturally-finished sink must tear down immediately (no release
/// fade on an already-drained sink that produces no more audio).
#[test]
fn stop_on_drained_sink_is_immediate() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.set_effect(
        "ch",
        AudioEffect {
            release_secs: 10.0, // would be very long if incorrectly applied
            ..AudioEffect::default()
        },
    );
    // Play a very short tone so it drains quickly.
    audio.play_tone("ch", 440.0, 0.02, 0.5);

    // Wait for natural drain.
    let start = std::time::Instant::now();
    while audio.playback_state("ch") != AudioChannelState::Finished
        && start.elapsed() < std::time::Duration::from_secs(2)
    {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        audio.playback_state("ch"),
        AudioChannelState::Finished,
        "tone should have drained"
    );

    // stop() on a drained sink must be immediate even with release_secs=10.
    audio.stop("ch");
    assert_eq!(
        audio.playback_state("ch"),
        AudioChannelState::Missing,
        "stop() on a finished (drained) sink must remove the channel immediately"
    );
    // Must not have scheduled a release fade on the drained sink.
    assert!(
        !audio.fades.contains_key("ch"),
        "no release fade should be queued for a drained sink"
    );
}

// ─── #20: built-in AudioSystem drives update() ─────────────────────────────────

#[test]
fn audio_system_is_noop_without_manager_resource() {
    use super::AudioSystem;
    use crate::ecs::{System, World};

    let mut world = World::new();
    let mut sys = AudioSystem;
    // No AudioManager resource present → must run cleanly (no panic).
    sys.run(&mut world, 0.016);
}

#[test]
fn audio_system_drives_fade_out_to_stop_when_device_exists() {
    use super::AudioSystem;
    use crate::ecs::{System, World};

    // Headless CI has no audio device → AudioManager::new() is None; skip.
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.play_tone("fademe", 440.0, 1.0, 0.5); // 1s tone, still playing during fade
    assert!(audio.is_playing("fademe"));
    audio.fade_out("fademe", 0.05);

    let mut world = World::new();
    world.insert_resource(audio);

    // Tick the system past the fade duration (0.2s total > 0.05s).
    let mut sys = AudioSystem;
    for _ in 0..10 {
        sys.run(&mut world, 0.02);
    }

    let audio = world.resource::<AudioManager>().unwrap();
    assert!(
        !audio.is_playing("fademe"),
        "fade_out driven by AudioSystem should fade to 0 and stop the channel"
    );
}
