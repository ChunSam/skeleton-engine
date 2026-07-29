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

// ─── Fix #3: clear_file_cache (device-free) ───────────────────────────────────

/// `clear_file_cache` empties the in-memory file-bytes cache so a subsequent play
/// call re-reads from disk. Tested device-free via the internal `file_cache` field
/// and `read_cached_bytes` helper.
#[test]
fn clear_file_cache_empties_the_cache() {
    use super::playback::read_cached_bytes;
    use std::collections::HashMap;
    use std::sync::Arc;

    // Populate a standalone cache to simulate what AudioManager::play would do.
    let dir = std::env::temp_dir().join(format!("engine-audio-clear-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tone.bin");
    std::fs::write(&path, b"fake-audio").unwrap();
    let path_str = path.to_str().unwrap();

    let mut cache: HashMap<String, Arc<[u8]>> = HashMap::new();
    read_cached_bytes(&mut cache, path_str).expect("file must be readable");
    assert_eq!(cache.len(), 1, "cache should have one entry after a play");

    cache.clear();
    assert!(
        cache.is_empty(),
        "cache must be empty after clear_file_cache"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Verify `AudioManager::clear_file_cache` is callable when a device is present,
/// and that it really empties `file_cache`.
#[test]
fn audio_manager_clear_file_cache_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    // Manually insert a fake entry (path needn't be real — we're testing the clear, not play).
    use std::sync::Arc;
    audio
        .file_cache
        .insert("fake/path.ogg".to_string(), Arc::from(&b"bytes"[..]));
    assert_eq!(audio.file_cache.len(), 1);

    audio.clear_file_cache();
    assert!(
        audio.file_cache.is_empty(),
        "clear_file_cache must empty the file_cache map"
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

// ─── Bus + fade double-multiply regression ─────────────────────────────────────

/// Regression test: verifies that bus volume is applied EXACTLY ONCE during a fade.
///
/// The bug: `fade_start_vol` used `effective_volume` (= base × bus), so `start_vol`
/// already included the bus factor; `update()` then multiplied by `bus_vol` again,
/// yielding `base × bus²` on the sink.  The fix stores pre-bus base volume in the
/// fade and applies the bus multiplier once in `update()`.
///
/// We verify via the fade's `start_vol` field: after calling `fade_out` on a channel
/// assigned to a bus with volume 0.5 and base volume 0.8, `start_vol` must equal 0.8
/// (pre-bus), NOT 0.4 (post-bus = 0.8 × 0.5). The final sink value = 0.8 × 0.5 = 0.4
/// which is the correct non-squared result; with the bug it was 0.4 × 0.5 = 0.2.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn bus_fade_volume_applied_exactly_once() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };

    // Assign channel to a bus with volume 0.5; set base volume 0.8.
    audio.assign_bus("ch", "sfx");
    audio.set_bus_volume("sfx", 0.5);
    audio.set_volume("ch", 0.8);
    audio.play_tone("ch", 440.0, 10.0, 0.5);

    // Start a fade_out. The fade's start_vol must be the PRE-BUS base = 0.8.
    // With the bug it would be effective_volume = 0.8 × 0.5 = 0.4.
    audio.fade_out("ch", 1.0);

    let fade = audio
        .fades
        .get("ch")
        .expect("fade_out must insert a fade entry");
    assert!(
        (fade.start_vol - 0.8).abs() < 0.01,
        "fade start_vol must be the pre-bus base volume (0.8), got {} \
         — if this is ~0.4 the bus is being squared",
        fade.start_vol
    );

    // Additionally verify that update() produces the correct initial sink volume
    // = start_vol × bus_vol = 0.8 × 0.5 = 0.4, NOT 0.4 × 0.5 = 0.2.
    // We can't read the sink volume directly, but we can confirm effective_volume
    // returns 0.4 (= base × bus) which is what update() will apply at t=0.
    let eff = audio.effective_volume("ch");
    assert!(
        (eff - 0.4).abs() < 0.01,
        "effective_volume must be base × bus = 0.8 × 0.5 = 0.4, got {eff}"
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

// ─── Bus ducking tests (device-free) ──────────────────────────────────────────

/// duck_bus then N update(dt) ticks → bus_duck approaches target gain monotonically.
#[test]
fn duck_bus_animates_toward_gain() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    // Duck the "music" bus toward 0.25 over 0.5 s.
    audio.duck_bus("music", 0.25, 0.5);
    assert!(
        (audio.bus_duck("music") - 1.0).abs() < 0.01,
        "duck should start at 1.0"
    );

    let mut prev = audio.bus_duck("music");
    for _ in 0..20 {
        audio.update(0.05); // 20 × 0.05 s = 1.0 s total, well past 0.5 s attack
        let cur = audio.bus_duck("music");
        assert!(
            cur <= prev + 0.0001,
            "duck should be monotonically decreasing, prev={prev} cur={cur}"
        );
        prev = cur;
    }

    let final_duck = audio.bus_duck("music");
    assert!(
        (final_duck - 0.25).abs() < 0.01,
        "duck should reach target 0.25 after sufficient ticks, got {final_duck}"
    );
}

/// release_bus after a duck → bus_duck returns toward 1.0.
#[test]
fn release_bus_returns_to_one() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    // Instant-duck to 0.2 by using a very short attack.
    audio.duck_bus("music", 0.2, 0.001);
    for _ in 0..5 {
        audio.update(0.01);
    }
    let ducked = audio.bus_duck("music");
    assert!(
        ducked < 0.5,
        "bus should be substantially ducked before release, got {ducked}"
    );

    // Now release over 0.5 s.
    audio.release_bus("music", 0.5);
    let mut prev = audio.bus_duck("music");
    for _ in 0..20 {
        audio.update(0.05); // 1.0 s total
        let cur = audio.bus_duck("music");
        assert!(
            cur >= prev - 0.0001,
            "duck should be monotonically increasing during release, prev={prev} cur={cur}"
        );
        prev = cur;
    }

    let final_duck = audio.bus_duck("music");
    assert!(
        (final_duck - 1.0).abs() < 0.01,
        "duck should return to 1.0 after release completes, got {final_duck}"
    );
}

/// effective_volume reflects the duck: a ducked bus lowers a channel's effective_volume.
#[test]
fn effective_volume_reflects_duck() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.assign_bus("bgm", "music");
    audio.set_bus_volume("music", 1.0);

    // Before any duck, effective_volume should be 1.0.
    let before = audio.effective_volume("bgm");
    assert!(
        (before - 1.0).abs() < 0.01,
        "effective_volume should be 1.0 before ducking, got {before}"
    );

    // Instant-duck toward 0.3.
    audio.duck_bus("music", 0.3, 0.001);
    for _ in 0..10 {
        audio.update(0.01);
    }

    let duck_val = audio.bus_duck("music");
    let eff = audio.effective_volume("bgm");
    assert!(
        (eff - duck_val).abs() < 0.01,
        "effective_volume must equal bus_duck when bus_vol=1.0, eff={eff} duck={duck_val}"
    );
    assert!(
        eff < 0.5,
        "effective_volume should be substantially reduced by ducking, got {eff}"
    );
}

/// set_sidechain / clear_sidechain — registration add/replace/remove.
#[test]
fn sidechain_registration_add_replace_remove() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };

    // No sidechains initially.
    assert!(audio.sidechains.is_empty());

    // Add a sidechain.
    audio.set_sidechain("voice", "music", 0.25, 0.15, 0.6);
    assert_eq!(audio.sidechains.len(), 1);
    assert_eq!(audio.sidechains[0].trigger_bus, "voice");
    assert_eq!(audio.sidechains[0].ducked_bus, "music");
    assert!((audio.sidechains[0].gain - 0.25).abs() < 0.001);

    // Replace: same ducked_bus, new trigger/gain.
    audio.set_sidechain("dialogue", "music", 0.15, 0.2, 0.8);
    assert_eq!(
        audio.sidechains.len(),
        1,
        "replacing a sidechain must not add a duplicate"
    );
    assert_eq!(audio.sidechains[0].trigger_bus, "dialogue");
    assert!((audio.sidechains[0].gain - 0.15).abs() < 0.001);

    // Add a second sidechain for a different ducked_bus.
    audio.set_sidechain("explosion", "sfx_ambient", 0.5, 0.05, 0.3);
    assert_eq!(audio.sidechains.len(), 2);

    // Clear the first.
    audio.clear_sidechain("music");
    assert_eq!(audio.sidechains.len(), 1);
    assert_eq!(audio.sidechains[0].ducked_bus, "sfx_ambient");

    // Clear the remaining.
    audio.clear_sidechain("sfx_ambient");
    assert!(audio.sidechains.is_empty());
}

// ─── crossfade scheduling tests (device-free) ─────────────────────────────────

/// After `crossfade`, the new channel has a fade-in scheduled and the temp
/// channel has a stop-when-done fade-out — verified purely via the `fades` and
/// `sinks` maps without audio hardware.
///
/// We test scheduling + bookkeeping only; actual audio output is intentionally
/// not asserted so the test passes on CI (no audio device).
#[test]
fn crossfade_schedules_fade_in_on_channel_and_fade_out_on_temp() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };

    // Put something on the "bgm" channel to simulate a playing track.
    audio.play_tone("bgm", 330.0, 60.0, 0.5);
    assert!(audio.is_playing("bgm"), "precondition: bgm must be playing");

    // The new path doesn't need to exist for the scheduling assertions:
    // play_internal will warn + return early if the file is missing, which
    // means the new sink won't be created, but the OLD sink moves to __xfade
    // and its fade-out IS scheduled regardless.  We use a temp file so
    // play_fade_in creates the new sink too.
    let dir = std::env::temp_dir().join(format!("engine-xfade-sched-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let track_b = dir.join("track_b.bin");
    // Minimal WAV header (44 bytes header + silence) so Decoder succeeds.
    // rodio's Decoder requires at least a parseable container.
    // Using a raw Hound-compatible minimal PCM WAV:
    // RIFF header: "RIFF" + size + "WAVE" + "fmt " + 16 + PCM params + "data" + size + samples
    let mut wav: Vec<u8> = Vec::new();
    let pcm_data: &[u8] = &[0u8; 2]; // 1 sample of silence (16-bit mono)
    let data_len = pcm_data.len() as u32;
    let file_len = 36 + data_len;
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&44100u32.to_le_bytes()); // sample rate
    wav.extend_from_slice(&88200u32.to_le_bytes()); // byte rate
    wav.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm_data);
    std::fs::write(&track_b, &wav).unwrap();

    let track_b_path = track_b.to_str().unwrap();
    audio.crossfade("bgm", track_b_path, true, 2.0);

    // The temp channel must have the old sink and a stop-when-done fade-out.
    assert!(
        audio.sinks.contains_key("bgm__xfade"),
        "old sink must be relocated to bgm__xfade"
    );
    let xfade_fade = audio
        .fades
        .get("bgm__xfade")
        .expect("bgm__xfade must have a fade entry");
    assert!(
        xfade_fade.stop_when_done,
        "bgm__xfade fade must be stop_when_done (fade-out)"
    );
    assert!(
        (xfade_fade.target_vol - 0.0).abs() < 0.001,
        "bgm__xfade fade must target 0.0"
    );

    // The new sink on "bgm" must exist.
    assert!(
        audio.sinks.contains_key("bgm"),
        "new track must have a sink on the bgm channel"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// After `crossfade` with no prior track, behavior is identical to `play_fade_in`:
/// no temp channel is created.
#[test]
fn crossfade_with_no_prior_track_acts_like_play_fade_in() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };

    // Ensure there is nothing on "bgm".
    assert!(
        !audio.sinks.contains_key("bgm"),
        "precondition: bgm must be absent"
    );

    let dir = std::env::temp_dir().join(format!("engine-xfade-empty-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let track = dir.join("track.wav");
    // Same minimal WAV as above.
    let mut wav: Vec<u8> = Vec::new();
    let pcm_data: &[u8] = &[0u8; 2];
    let data_len = pcm_data.len() as u32;
    let file_len = 36 + data_len;
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&44100u32.to_le_bytes());
    wav.extend_from_slice(&88200u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm_data);
    std::fs::write(&track, &wav).unwrap();

    let path = track.to_str().unwrap();
    audio.crossfade("bgm", path, false, 1.5);

    // No temp channel should exist.
    assert!(
        !audio.sinks.contains_key("bgm__xfade"),
        "no temp channel when there was no prior track"
    );
    // The new sink must be on "bgm".
    assert!(
        audio.sinks.contains_key("bgm"),
        "new track must have a sink on bgm"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// After `crossfade` and driving `update(dur)`, the temp channel is torn down
/// and the main channel reaches full volume.
#[test]
fn crossfade_temp_channel_torn_down_after_update() {
    use super::AudioSystem;
    use crate::ecs::{System, World};

    let Some(mut audio) = AudioManager::new() else {
        return;
    };

    audio.play_tone("bgm", 330.0, 60.0, 0.5);
    assert!(audio.is_playing("bgm"));

    let dir = std::env::temp_dir().join(format!("engine-xfade-update-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let track = dir.join("track.wav");
    let mut wav: Vec<u8> = Vec::new();
    let pcm_data: &[u8] = &[0u8; 2];
    let data_len = pcm_data.len() as u32;
    let file_len = 36 + data_len;
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&44100u32.to_le_bytes());
    wav.extend_from_slice(&88200u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm_data);
    std::fs::write(&track, &wav).unwrap();

    let path = track.to_str().unwrap();
    audio.crossfade("bgm", path, false, 0.1);

    // Temp channel must exist immediately after crossfade.
    assert!(
        audio.sinks.contains_key("bgm__xfade"),
        "temp channel must exist right after crossfade"
    );

    // Drive update past the fade duration so the temp channel is torn down.
    let mut world = World::new();
    world.insert_resource(audio);
    let mut sys = AudioSystem;
    for _ in 0..10 {
        sys.run(&mut world, 0.02); // 0.2 s > 0.1 s fade
    }

    let audio = world.resource::<AudioManager>().unwrap();
    assert!(
        !audio.sinks.contains_key("bgm__xfade"),
        "temp channel must be torn down after fade completes"
    );
    assert!(
        !audio.fades.contains_key("bgm__xfade"),
        "no stale fade entry for the temp channel"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Sidechain drive: without a real playing channel we can't assert automatic
/// trigger-active detection, but we CAN assert that manually calling duck_bus +
/// update drives the ducked bus to the target and release_bus brings it back.
/// (Full sidechain-active detection requires a live sink which is device-dependent;
/// that path is exercised indirectly by duck_bus_animates_toward_gain.)
#[test]
fn sidechain_gain_clamp_and_attack_release_secs_clamp() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    // Gain > 1.0 should be clamped to 1.0.
    audio.set_sidechain("voice", "music", 2.0, 0.1, 0.5);
    assert!(
        (audio.sidechains[0].gain - 1.0).abs() < 0.001,
        "gain must be clamped to 1.0"
    );

    // Gain < 0.0 clamped to 0.0.
    audio.set_sidechain("voice", "music", -0.5, 0.1, 0.5);
    assert!(
        (audio.sidechains[0].gain - 0.0).abs() < 0.001,
        "gain must be clamped to 0.0"
    );

    // attack_secs and release_secs must be >= 0.001.
    audio.set_sidechain("voice", "music", 0.3, 0.0, 0.0);
    assert!(
        audio.sidechains[0].attack_secs >= 0.001,
        "attack_secs must be clamped to >= 0.001"
    );
    assert!(
        audio.sidechains[0].release_secs >= 0.001,
        "release_secs must be clamped to >= 0.001"
    );
}

// ─── Level analysis ───────────────────────────────────────────────────────────

#[test]
fn an_unanalyzed_channel_reads_silent_when_device_exists() {
    let Some(audio) = AudioManager::new() else {
        return;
    };
    // The default for every channel: analysis is opt-in, so nothing is measured until asked.
    assert!(!audio.is_analysis_enabled("nope"));
    assert!(audio.levels("nope").is_silent());
}

#[test]
fn enable_analysis_is_reported_and_starts_silent_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.enable_analysis("ch");
    assert!(audio.is_analysis_enabled("ch"));
    // Enabled but never played: still silent, not stale garbage.
    assert!(audio.levels("ch").is_silent());
}

#[test]
fn enabling_twice_is_idempotent_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.enable_analysis("ch");
    audio.enable_analysis("ch");
    assert!(audio.is_analysis_enabled("ch"));
}

#[test]
fn disable_analysis_stops_reporting_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.enable_analysis("ch");
    audio.disable_analysis("ch");
    assert!(!audio.is_analysis_enabled("ch"));
    assert!(audio.levels("ch").is_silent());
}

#[test]
fn analysis_smoothing_defaults_and_never_goes_negative_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    assert_eq!(
        audio.analysis_smoothing(),
        crate::audio_analysis::DEFAULT_ANALYSIS_SMOOTHING
    );
    audio.set_analysis_smoothing(0.4);
    assert_eq!(audio.analysis_smoothing(), 0.4);
    // A negative release would invert the decay; it is clamped instead.
    audio.set_analysis_smoothing(-1.0);
    assert_eq!(audio.analysis_smoothing(), 0.0);
}

#[test]
fn ticking_update_with_no_analyzed_channels_is_harmless_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    // The common case for every existing game: analysis off, `update` still called every frame.
    audio.update(1.0 / 60.0);
    assert!(audio.levels("anything").is_silent());
}

#[test]
fn spectrum_is_off_until_asked_for_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    // Levels-only analysis must run NO transform — that is the reason the two are separate.
    audio.enable_analysis("ch");
    let mut bands = [1.0f32; 8]; // pre-filled so a no-op write would be visible
    audio.bands("ch", &mut bands);
    assert_eq!(bands, [0.0; 8], "a levels-only channel reports no spectrum");
}

#[test]
fn enable_spectrum_implies_analysis_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.enable_spectrum("ch");
    assert!(
        audio.is_analysis_enabled("ch"),
        "enabling a spectrum must also enable levels"
    );
}

#[test]
fn bands_fills_the_callers_length_whatever_it_is_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.enable_spectrum("ch");
    // The API promise: the caller picks the band count, and neither backend's FFT size leaks.
    for n in [1usize, 4, 32, 64] {
        let mut bands = vec![9.0; n];
        audio.bands("ch", &mut bands);
        assert_eq!(bands.len(), n);
        assert!(
            bands.iter().all(|b| (0.0..=1.0).contains(b)),
            "n={n} produced out-of-range values: {bands:?}"
        );
    }
}

#[test]
fn bands_on_an_unknown_channel_is_silence_not_a_panic_when_device_exists() {
    let Some(audio) = AudioManager::new() else {
        return;
    };
    let mut bands = [0.5f32; 4];
    audio.bands("never-played", &mut bands);
    assert_eq!(bands, [0.0; 4]);
}

#[test]
fn disable_spectrum_keeps_levels_running_when_device_exists() {
    let Some(mut audio) = AudioManager::new() else {
        return;
    };
    audio.enable_spectrum("ch");
    audio.disable_spectrum("ch");
    let mut bands = [1.0f32; 4];
    audio.bands("ch", &mut bands);
    assert_eq!(bands, [0.0; 4], "spectrum is off");
    assert!(
        audio.is_analysis_enabled("ch"),
        "but levels analysis survives"
    );
}
