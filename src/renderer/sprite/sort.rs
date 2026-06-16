use std::sync::Arc;

use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) struct RenderSortKey {
    pub(super) layer: i32,
    pub(super) z: f32,
    pub(super) order: usize,
}

pub(super) enum SpriteRenderKind {
    Sprite {
        texture_key: Arc<str>,
        instance: InstanceRaw,
        instance_offset: usize,
    },
    Material {
        entity: crate::ecs::Entity,
        hash: u64,
        frag_source: String,
        params: [f32; 4],
        texture_key: Option<Arc<str>>,
        instance: InstanceRaw,
        instance_offset: usize,
    },
}

pub(super) struct SpriteRenderEntry {
    pub(super) sort: RenderSortKey,
    pub(super) kind: SpriteRenderKind,
}

impl SpriteRenderEntry {
    pub(super) fn sprite(
        layer: i32,
        z: f32,
        order: usize,
        texture_key: Arc<str>,
        instance: InstanceRaw,
    ) -> Self {
        Self {
            sort: RenderSortKey { layer, z, order },
            kind: SpriteRenderKind::Sprite {
                texture_key,
                instance,
                instance_offset: 0,
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn material(
        layer: i32,
        z: f32,
        order: usize,
        entity: crate::ecs::Entity,
        hash: u64,
        frag_source: String,
        params: [f32; 4],
        texture_key: Option<Arc<str>>,
        instance: InstanceRaw,
    ) -> Self {
        Self {
            sort: RenderSortKey { layer, z, order },
            kind: SpriteRenderKind::Material {
                entity,
                hash,
                frag_source,
                params,
                texture_key,
                instance,
                instance_offset: 0,
            },
        }
    }
}

pub(super) fn compare_render_sort_key(a: RenderSortKey, b: RenderSortKey) -> Ordering {
    a.layer
        .cmp(&b.layer)
        .then_with(|| a.z.partial_cmp(&b.z).unwrap_or(Ordering::Equal))
        .then_with(|| a.order.cmp(&b.order))
}

pub(super) fn sort_render_entries(entries: &mut [SpriteRenderEntry]) {
    entries.sort_by(|a, b| compare_render_sort_key(a.sort, b.sort));
}

/// Layer-mask membership test.
///
/// `layer_mask == 0` is the "render everything" sentinel. Otherwise each
/// `RenderLayer(n)` for `n` in `0..=31` maps to bit `n` of the mask and renders
/// only when that bit is set. Layers outside `0..=31` (negative, or `> 31`)
/// cannot be addressed by a 32-bit mask, so under a non-zero mask they never
/// match (they still render under the `0` "render everything" mask).
///
/// This replaces the old `layer.clamp(0, 31)`, which folded e.g. a
/// `RenderLayer(-1)` background onto bit 0 — leaking it into `layer_mask: 1 << 0`.
pub(super) fn layer_matches_mask(layer: i32, layer_mask: u32) -> bool {
    if layer_mask == 0 {
        return true;
    }
    if !(0..=31).contains(&layer) {
        warn_unmaskable_layer_once(layer);
        return false;
    }
    (layer_mask >> layer as u32) & 1 == 1
}

/// Warn (once per process) that a layer outside `0..=31` can't be selected by a
/// layer mask. Called from the per-sprite hot path, so it must not log per frame.
fn warn_unmaskable_layer_once(layer: i32) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        log::warn!(
            "RenderLayer({layer}) is outside the maskable range 0..=31; a non-zero \
             layer_mask cannot select it, so it is excluded from this masked pass. \
             Only layers 0..=31 can be addressed by a layer mask."
        );
    }
}

/// Assign `instance_offset` fields and fill the caller-provided scratch buffers.
///
/// Accepts pre-allocated `&mut Vec<InstanceRaw>` so the renderer can reuse the
/// same backing allocation across frames (no per-frame heap alloc on steady state).
/// Both vecs are cleared before use.
pub(super) fn assign_instance_offsets(
    entries: &mut [SpriteRenderEntry],
    sprite_instances: &mut Vec<InstanceRaw>,
    material_instances: &mut Vec<InstanceRaw>,
) {
    sprite_instances.clear();
    material_instances.clear();

    for entry in entries {
        match &mut entry.kind {
            SpriteRenderKind::Sprite {
                instance,
                instance_offset,
                ..
            } => {
                *instance_offset = sprite_instances.len();
                sprite_instances.push(*instance);
            }
            SpriteRenderKind::Material {
                instance,
                instance_offset,
                ..
            } => {
                *instance_offset = material_instances.len();
                material_instances.push(*instance);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::layer_matches_mask;

    #[test]
    fn mask_zero_renders_every_layer() {
        // 0 = "render everything" — even negative/out-of-range layers.
        assert!(layer_matches_mask(0, 0));
        assert!(layer_matches_mask(-1, 0));
        assert!(layer_matches_mask(5, 0));
        assert!(layer_matches_mask(999, 0));
    }

    #[test]
    fn positive_layers_map_to_their_bit() {
        assert!(layer_matches_mask(0, 1 << 0));
        assert!(!layer_matches_mask(0, 1 << 1));
        assert!(layer_matches_mask(1, 1 << 1));
        assert!(!layer_matches_mask(1, 1 << 0));
        assert!(layer_matches_mask(31, 1 << 31)); // upper boundary
    }

    /// Regression for #22: a `RenderLayer(-1)` background must NOT leak into a
    /// `layer_mask: 1 << 0` (the OffscreenCamera "layer 0 only" case). The old
    /// `clamp(0, 31)` folded -1 onto bit 0 and let it through.
    #[test]
    fn negative_background_layer_excluded_by_layer0_mask() {
        assert!(!layer_matches_mask(-1, 1 << 0));
        assert!(!layer_matches_mask(-5, 1 << 0));
    }

    #[test]
    fn out_of_range_high_layers_not_addressable_by_mask() {
        // > 31 can't be represented in a u32 mask → excluded under a non-zero mask.
        assert!(!layer_matches_mask(32, 1 << 0));
        assert!(!layer_matches_mask(64, u32::MAX));
    }
}
