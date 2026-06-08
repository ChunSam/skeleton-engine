use std::collections::HashMap;

use rodio::{OutputStream, OutputStreamHandle, Sink};

mod bus;
mod effects;
mod playback;
mod positional;
mod source;
mod types;

pub use types::{AudioChannelState, AudioEffect};

// ─── AudioManager ─────────────────────────────────────────────────────────────

/// Audio playback manager (insert as an ECS resource).
///
/// ## Basic playback
/// ```rust,no_run
/// # use engine::AudioManager;
/// # let mut am = AudioManager::new().unwrap();
/// am.play("bgm", "assets/music.ogg", true);
/// am.set_volume("bgm", 0.6);
/// am.stop("bgm");
/// ```
///
/// ## Positional audio
/// ```rust,no_run
/// # use engine::AudioManager;
/// # use glam::Vec2;
/// # let mut am = AudioManager::new().unwrap();
/// let source_pos = Vec2::new(300.0, 200.0);
/// let listener   = Vec2::new(0.0, 0.0);
/// am.play_at("sfx_hit", "assets/hit.wav", false, source_pos, listener, 500.0);
/// ```
///
/// ## Audio buses (group volume)
/// ```rust,no_run
/// # use engine::AudioManager;
/// # let mut am = AudioManager::new().unwrap();
/// am.assign_bus("bgm",      "music");
/// am.assign_bus("sfx_jump", "sfx");
/// am.set_bus_volume("music", 0.5);  // halve the music bus volume
/// ```
pub struct AudioManager {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sinks: HashMap<String, Sink>,
    /// Per-channel base volume (before multiplying by bus volume).
    volume_overrides: HashMap<String, f32>,
    /// Per-channel stereo pan.
    pans: HashMap<String, f32>,
    /// Bus name → volume multiplier.
    bus_volumes: HashMap<String, f32>,
    /// Channel → bus name.
    channel_buses: HashMap<String, String>,
    /// Active fade states.
    fades: HashMap<String, types::Fade>,
    /// Per-channel audio effects.
    effects: HashMap<String, AudioEffect>,
    /// Path → encoded file bytes cache. Prevents re-reading the disk when the same SFX
    /// is replayed (`play`/`play_internal` path only; `play_streaming` for large BGM
    /// streams directly and is not cached).
    file_cache: HashMap<String, std::sync::Arc<[u8]>>,
}

// ─── AudioSystem ──────────────────────────────────────────────────────────────

/// Built-in system that advances audio fades every frame.
///
/// `AudioManager::fade_out` / `fade_volume` only *schedule* a fade; the actual
/// progress requires `AudioManager::update(dt)` to be called each frame. Registering
/// this system makes fades work automatically (added explicitly by the user, like other
/// built-in engine systems).
///
/// ```rust,no_run
/// # use engine::{App, AudioManager, AudioSystem};
/// let mut app = App::new();
/// if let Some(audio) = AudioManager::new() {
///     app.world.insert_resource(audio);
/// }
/// app.add_system(AudioSystem);
/// ```
///
/// Does nothing if the `AudioManager` resource is absent.
#[derive(Default)]
pub struct AudioSystem;

impl crate::ecs::System for AudioSystem {
    fn run(&mut self, world: &mut crate::ecs::World, dt: f32) {
        if let Some(audio) = world.resource_mut::<AudioManager>() {
            audio.update(dt);
        }
    }
}

#[cfg(test)]
mod tests;
