// Arrive agent: moves toward the mouse cursor, decelerating as it gets close.
// Called by ScriptingSystem each frame.

fn on_update(dt) {
    let tx = bb_get_float("target_x");
    let ty = bb_get_float("target_y");
    arrive_at(tx, ty, 200.0, 100.0, 12.0);
}
