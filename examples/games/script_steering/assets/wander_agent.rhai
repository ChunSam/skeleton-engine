// Wander agent: roams in a random direction, changing every 1.5 seconds.
// Called by ScriptingSystem each frame.

fn on_update(dt) {
    wander(90.0, 1.5);
}
