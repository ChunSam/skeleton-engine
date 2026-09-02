/// Case-insensitive substring match for the entity-list search box. An empty (or
/// whitespace-only) filter matches every entity.
pub(in crate::app) fn entity_matches_filter(label: &str, filter: &str) -> bool {
    let filter = filter.trim();
    filter.is_empty() || label.to_lowercase().contains(&filter.to_lowercase())
}

/// Snap `pos` to the grid of `snap_size`. A snap size that is not a positive finite number —
/// `0.0` from a hand-edited settings file, say — means *no* snapping, rather than the NaN that
/// `(x / 0.0).round() * 0.0` used to produce and the gizmo then wrote into `Transform` with no
/// undo entry (v0.156.14). Every snap in the editor goes through here, so there is one spelling.
pub(in crate::app) fn snap_to_grid(pos: glam::Vec2, snap_size: f32) -> glam::Vec2 {
    if snap_size <= 0.0 || !snap_size.is_finite() {
        // (A NaN fails the `<=` and is caught by `is_finite`.)
        return pos;
    }
    glam::Vec2::new(
        (pos.x / snap_size).round() * snap_size,
        (pos.y / snap_size).round() * snap_size,
    )
}
