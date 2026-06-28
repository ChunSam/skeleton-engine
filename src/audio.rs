use std::collections::HashMap;

use rodio::{OutputStream, OutputStreamHandle, Sink};

mod bus;
mod ducking;
mod effects;
mod playback;
mod positional;
mod source;
mod types;

/// Minimum duration (seconds) a fade is clamped to, so the scheduler always gets at least one tick
/// to run instead of a zero-length fade that would complete before it starts. Shared by the fade
/// constructors in `types` and `bus`.
const MIN_AUDIO_DURATION_SECS: f32 = 0.001;

pub use ducking::{BusDuck, Sidechain};
pub use types::{AudioChannelState, AudioEffect};

// ─── AudioManager ─────────────────────────────────────────────────────────────

/// Audio playback manager (insert as an ECS resource).
///
/// ## Platform availability
///
/// `AudioManager` is **native-only** — it is unconditionally `cfg(not(target_arch = "wasm32"))`
/// inside the engine. Game code that targets both native and WASM must guard every use:
///
/// ```rust,no_run
/// // In your game's main() or system:
/// #[cfg(not(target_arch = "wasm32"))]
/// {
///     if let Some(audio) = engine::AudioManager::new() {
///         // use audio here
///     }
/// }
/// ```
///
/// See `examples/audio_fades.rs` for a complete stub-main pattern where the
/// `#[cfg(not(target_arch = "wasm32"))]` guard wraps the entire example body,
/// and a no-op `fn main() {}` is compiled for WASM targets.
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
    /// is replayed (`play`/`play_internal` path only).
    file_cache: HashMap<String, std::sync::Arc<[u8]>>,
    /// Per-bus duck state (gain multiplier riding on top of bus volume).
    bus_ducks: HashMap<String, ducking::BusDuck>,
    /// Automatic sidechain rules (at most one per `ducked_bus`).
    sidechains: Vec<ducking::Sidechain>,
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

impl AudioSystem {
    /// Schedule label for ordering via `add_system_labeled`.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::audio";
}

impl crate::ecs::System for AudioSystem {
    fn run(&mut self, world: &mut crate::ecs::World, dt: f32) {
        if let Some(audio) = world.resource_mut::<AudioManager>() {
            audio.update(dt);
        }
    }
}

#[cfg(test)]
mod tests;
