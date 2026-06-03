use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) struct RenderSortKey {
    pub(super) layer: i32,
    pub(super) z: f32,
    pub(super) order: usize,
}

pub(super) enum SpriteRenderKind {
    Sprite {
        texture_key: String,
        instance: InstanceRaw,
        instance_offset: usize,
    },
    Material {
        entity: crate::ecs::Entity,
        hash: u64,
        frag_source: String,
        params: [f32; 4],
        texture_key: Option<String>,
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
        texture_key: String,
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
        texture_key: Option<String>,
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

pub(super) fn layer_matches_mask(layer: i32, layer_mask: u32) -> bool {
    if layer_mask == 0 {
        return true;
    }
    let bit = layer.clamp(0, 31) as u32;
    (layer_mask >> bit) & 1 == 1
}

pub(super) fn assign_instance_offsets(
    entries: &mut [SpriteRenderEntry],
) -> (Vec<InstanceRaw>, Vec<InstanceRaw>) {
    let mut sprite_instances = Vec::new();
    let mut material_instances = Vec::new();

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

    (sprite_instances, material_instances)
}
