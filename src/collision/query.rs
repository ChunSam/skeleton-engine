use glam::Vec2;

use super::grid::{Collider, CollisionLayer, SpatialGrid};
use crate::ecs::Entity;

impl SpatialGrid {
    /// Returns all entities within a circular region.
    ///
    /// - Excludes entities where `mask` AND entity layer equals 0.
    /// - Circle collider: passes when center distance ≤ radius + collider.radius
    /// - Aabb collider: passes when the query circle intersects the AABB
    pub fn query_radius(&self, center: Vec2, radius: f32, mask: CollisionLayer) -> Vec<Entity> {
        // Narrow candidate cell range using the AABB that wraps the query circle
        let search_min = Vec2::new(center.x - radius, center.y - radius);
        let search_max = Vec2::new(center.x + radius, center.y + radius);

        let candidates = self.candidates_in_aabb(search_min, search_max);
        let mut result = Vec::new();

        for entity in candidates {
            let entry = match self.entries.get(&entity) {
                Some(e) => e,
                None => continue,
            };

            // Check layer mask
            if !mask.matches(entry.layer) {
                continue;
            }

            // Exact distance / intersection test
            if circle_hits_collider(center, radius, entry.center, entry.collider) {
                result.push(entity);
            }
        }

        result
    }

    /// Returns all entities overlapping the given AABB region.
    ///
    /// - Excludes entities where `mask` AND entity layer equals 0.
    pub fn query_aabb(&self, min: Vec2, max: Vec2, mask: CollisionLayer) -> Vec<Entity> {
        let candidates = self.candidates_in_aabb(min, max);
        let mut result = Vec::new();

        for entity in candidates {
            let entry = match self.entries.get(&entity) {
                Some(e) => e,
                None => continue,
            };

            if !mask.matches(entry.layer) {
                continue;
            }

            if aabb_hits_collider(min, max, entry.center, entry.collider) {
                result.push(entity);
            }
        }

        result
    }
}

// ─── Internal intersection helpers ──────────────────────────────────────────────────────

/// Tests whether the query circle (center, radius) overlaps a collider.
fn circle_hits_collider(
    query_center: Vec2,
    query_radius: f32,
    entity_center: Vec2,
    collider: Collider,
) -> bool {
    match collider {
        Collider::Circle { radius } => {
            let dist = (query_center - entity_center).length();
            dist <= query_radius + radius
        }
        Collider::Aabb { half_extents } => {
            // Circle vs AABB: minimum distance from circle center to AABB ≤ radius
            let aabb_min = entity_center - half_extents;
            let aabb_max = entity_center + half_extents;
            let closest = Vec2::new(
                query_center.x.clamp(aabb_min.x, aabb_max.x),
                query_center.y.clamp(aabb_min.y, aabb_max.y),
            );
            (query_center - closest).length() <= query_radius
        }
    }
}

/// Tests whether the query AABB (min, max) overlaps a collider.
fn aabb_hits_collider(
    query_min: Vec2,
    query_max: Vec2,
    entity_center: Vec2,
    collider: Collider,
) -> bool {
    // Compute the collider's AABB and check whether the two AABBs overlap
    let (col_min, col_max) = collider.aabb(entity_center);

    // AABB vs AABB intersection: both axes must overlap
    let overlap_x = query_min.x <= col_max.x && query_max.x >= col_min.x;
    let overlap_y = query_min.y <= col_max.y && query_max.y >= col_min.y;
    overlap_x && overlap_y
}
