/// Case-insensitive substring match for the entity-list search box. An empty (or
/// whitespace-only) filter matches every entity.
pub(in crate::app) fn entity_matches_filter(label: &str, filter: &str) -> bool {
    let filter = filter.trim();
    filter.is_empty() || label.to_lowercase().contains(&filter.to_lowercase())
}

pub(in crate::app) fn snap_to_grid(pos: glam::Vec2, snap_size: f32) -> glam::Vec2 {
    glam::Vec2::new(
        (pos.x / snap_size).round() * snap_size,
        (pos.y / snap_size).round() * snap_size,
    )
}
