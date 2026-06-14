use rhai::{Engine, Scope};

use crate::asset::{Handle, ScriptAsset};

mod api;
mod context;
mod execution;

// ─── ScriptRunner ─────────────────────────────────────────────────────────────

/// Script runner component attached to an entity.
///
/// ```rust,no_run
/// # use engine::{ScriptRunner, ScriptingSystem};
/// # let mut app = engine::App::new();
/// let handle = app.load_script("assets/enemy_ai.rhai");
/// // world.add_component(entity, ScriptRunner::new(handle));
/// // app.add_system(Box::new(ScriptingSystem::new()));
/// ```
pub struct ScriptRunner {
    pub script: Handle<ScriptAsset>,
    pub(crate) scope: Scope<'static>,
    pub(crate) started: bool,
}

impl ScriptRunner {
    pub fn new(script: Handle<ScriptAsset>) -> Self {
        let mut scope = Scope::new();
        scope.push("x", 0.0_f64);
        scope.push("y", 0.0_f64);
        scope.push("rot", 0.0_f64);
        scope.push("sx", 1.0_f64);
        scope.push("sy", 1.0_f64);
        Self {
            script,
            scope,
            started: false,
        }
    }

    /// Resets the runner so `on_start()` is called again on the next frame (useful after hot reload).
    pub fn reset(&mut self) {
        self.started = false;
    }

    /// Number of variables currently in the persistent scope. Test-only: used to assert
    /// the per-frame rewind keeps the scope at its 5-var transform baseline (no growth).
    #[cfg(test)]
    pub(crate) fn scope_len(&self) -> usize {
        self.scope.len()
    }
}

// ─── ScriptingSystem ──────────────────────────────────────────────────────────

/// System that runs scripts each frame for every entity that has a `ScriptRunner`.
///
/// This system is intended for trusted local game scripts. Rhai operation limits reduce
/// accidental infinite loops, but do not provide a sandbox boundary safe against hostile
/// or remote user input.
///
/// Scope variables: `x`, `y`, `rot`, `sx`, `sy`  (read/write Transform)
///
/// Lifecycle:
/// - `fn on_start()` — called once on the first frame (optional)
/// - `fn on_update(dt)` — called every frame
///
/// ## Additional Script API (Phase 38d)
///
/// ### Commands
/// ```rhai
/// let id = spawn_entity();   // spawn a new entity → returns ID (i64)
/// let index = entity_index();
/// let generation = entity_generation();
/// despawn_entity(index, generation); // schedule entity for despawn
/// ```
///
/// The negative ID returned by `spawn_entity()` is not a stable handle that can
/// manipulate the real entity within the same script. The actual spawn is applied after
/// script execution. `despawn_entity(index, generation)` constructs a generation-checked
/// ECS handle and silently ignores stale handles.
///
/// ### Blackboard
/// ```rhai
/// bb_set_bool("is_chasing", true);
/// bb_set_float("speed", 150.0);
/// bb_set_int("hp", 100);
/// let chasing = bb_get_bool("is_chasing");  // false if absent
/// let speed   = bb_get_float("speed");       // 0.0 if absent
/// let hp      = bb_get_int("hp");            // 0 if absent
/// ```
///
/// ### Steering
/// ```rhai
/// seek_target(player_x, player_y, 120.0);                    // set Seek component
/// flee_from(enemy_x, enemy_y, 200.0, 80.0);                  // set Flee component
/// arrive_at(tx, ty, speed, slow_radius, stop_radius);        // set Arrive component
/// wander(speed, change_interval);                            // set Wander component
/// stop_steering();                                           // remove all steering + zero velocity
/// ```
///
/// **Steering commands are mutually exclusive from scripts.** Calling any steering
/// function removes the other three steering components (`Seek`, `Flee`, `Arrive`,
/// `Wander`) before attaching its own. This prevents stale components from overriding
/// the newly requested behavior via `SteeringSystem`'s fixed evaluation order
/// (Seek → Flee → Arrive → Wander, last-writer-wins). `stop_steering()` removes all
/// four components *and* zeroes `SteeringVelocity` so the entity stays stopped on
/// subsequent frames.
///
/// Exception: `Wander` preserves its internal `timer`/`current_dir` fields when the
/// script calls `wander()` on an entity that is already wandering — only `speed` and
/// `change_interval` are updated — to keep movement smooth across parameter changes.
///
/// **This exclusivity applies only to the script apply path.** Rust-side code may still
/// compose multiple steering components on the same entity; `SteeringSystem` evaluates
/// them in order (Seek → Flee → Arrive → Wander) and the last one wins, which is a
/// documented feature for Rust-authored behaviors.
pub struct ScriptingSystem {
    engine: Engine,
}

/// Resource limits applied to the Rhai engine for a [`ScriptingSystem`].
///
/// Defaults are conservative but generous for *trusted local* game scripts: they
/// guard against accidental runaways (infinite loops, unbounded allocations, deep
/// recursion) without sandboxing hostile input. A value of `0` for any size/count
/// limit means "unlimited" (Rhai's convention).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptingLimits {
    /// Max operations per script run (guards infinite loops). 0 = unlimited.
    pub max_operations: u64,
    /// Max string length in bytes. 0 = unlimited.
    pub max_string_size: usize,
    /// Max array length. 0 = unlimited.
    pub max_array_size: usize,
    /// Max object-map entry count. 0 = unlimited.
    pub max_map_size: usize,
    /// Max function-call nesting depth (guards runaway recursion).
    pub max_call_levels: usize,
    /// Max expression nesting depth (guards parser stack overflow).
    pub max_expr_depth: usize,
}

impl Default for ScriptingLimits {
    fn default() -> Self {
        Self {
            max_operations: 1_000_000,
            max_string_size: 64 * 1024,
            max_array_size: 100_000,
            max_map_size: 100_000,
            max_call_levels: 64,
            max_expr_depth: 128,
        }
    }
}

impl ScriptingSystem {
    /// Creates a scripting system for trusted local script assets.
    ///
    /// Rhai operation limits reduce accidental runaway scripts, but engine script
    /// assets are still treated as trusted local game code rather than hostile
    /// sandboxed input.
    pub fn new() -> Self {
        Self::with_limits(ScriptingLimits::default())
    }
}

impl Default for ScriptingSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
