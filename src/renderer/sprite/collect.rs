use super::*;

impl SpriteRenderer {
    /// Walk the world and collect every renderable into `self.draw_entries` as
    /// `(layer, z)`-sortable entries: plain `Sprite`s (incl. nine-slice sub-quads),
    /// `AtlasSprite`s, and `ShaderMaterial`s. Culls each candidate through the
    /// supplied `is_visible` / `is_above_lod` closures (incrementing
    /// `stats.sprites_culled`) and skips anything outside `layer_mask`.
    ///
    /// Extracted verbatim from `render()`'s collect phase; behavior is unchanged.
    pub(super) fn collect_draw_entries(
        &mut self,
        world: &World,
        layer_mask: u32,
        is_visible: &dyn Fn(glam::Vec2, glam::Vec2, f32) -> bool,
        is_above_lod: &dyn Fn(glam::Vec2) -> bool,
        target_format: wgpu::TextureFormat,
        stats: &mut crate::resources::RenderStats,
    ) {
        // ── Collect all sprites into (layer, z) globally-sorted entries ──────
        // If GlobalTransform is present, use the hierarchy-composed result; otherwise fall back to Transform.
        // Entities without RenderLayer(i32) are treated as layer 0.
        //
        // draw_entries is a field on SpriteRenderer so its backing Vec<> allocation is
        // reused across frames (no per-frame heap alloc on steady state).
        self.draw_entries.clear();
        let mut next_order = 0usize;
        for (entity, sprite) in world.query::<Sprite>() {
            if world.get::<ShaderMaterial>(entity).is_some() {
                continue;
            }

            let uv = world.get::<UvRect>(entity).copied().unwrap_or(UvRect::FULL);
            // During a crossfade, pass the "to" frame + progress for the shader lerp.
            let blend = world.get::<BlendUv>(entity).copied();
            // SpriteFlip mirrors the sampled UV region (absent/default = no flip → unchanged).
            // NineSlice deliberately ignores it (a 9-patch has no meaningful mirror), so the flip
            // is applied inside `make_instance` — the nine-slice branch below keeps the raw `uv`.
            let flip = world
                .get::<crate::components::SpriteFlip>(entity)
                .copied()
                .unwrap_or_default();
            let make_instance = |model: [[f32; 4]; 4]| {
                let uv = uv.flipped(flip.x, flip.y);
                match blend {
                    Some(b) if b.weight > 0.0 => InstanceRaw::blended(
                        model,
                        sprite.color.to_array(),
                        uv,
                        b.to.flipped(flip.x, flip.y),
                        b.weight,
                    ),
                    _ => InstanceRaw::single(model, sprite.color.to_array(), uv),
                }
            };
            let layer = world
                .get::<crate::components::RenderLayer>(entity)
                .map(|l| l.0)
                .unwrap_or(0);
            if !layer_matches_mask(layer, layer_mask) {
                continue;
            }
            // Prefer image_handle path if present; fall back to the texture path.
            // path_arc() is an O(1) refcount bump; Arc::from(path()) would copy the string.
            let tex_key: Arc<str> = sprite
                .image_handle
                .as_ref()
                .map(|h| h.path_arc())
                .or_else(|| sprite.texture.clone())
                .unwrap_or_else(|| Arc::from(""));
            // Nine-slice: emit nine sub-quads (corners fixed-size, edges/center
            // stretched) instead of one. Only this new branch runs when a NineSlice
            // is present — ordinary sprites below remain byte-identical to before.
            if let Some(ns) = world.get::<NineSlice>(entity) {
                let ns = *ns;
                let (pos, panel_size, rotation, z) =
                    if let Some(gt) = world.get::<GlobalTransform>(entity) {
                        (gt.position, gt.scale, gt.rotation, gt.z)
                    } else if let Some(tr) = world.get::<Transform>(entity) {
                        (tr.position, tr.scale, tr.rotation, tr.z)
                    } else {
                        continue;
                    };
                let rot = glam::Vec2::from_angle(rotation);
                let color = sprite.color.to_array();
                for sq in nine_slice_subquads(panel_size, uv, &ns) {
                    let center = pos + rot.rotate(sq.local_center);
                    let model = Mat4::from_scale_rotation_translation(
                        Vec3::new(sq.size.x, sq.size.y, 1.0),
                        Quat::from_rotation_z(rotation),
                        Vec3::new(center.x, center.y, 0.0),
                    )
                    .to_cols_array_2d();
                    push_sprite_if_visible(
                        center,
                        sq.size,
                        rotation,
                        z,
                        layer,
                        tex_key.clone(),
                        InstanceRaw::single(model, color, sq.uv),
                        is_visible,
                        is_above_lod,
                        stats,
                        &mut next_order,
                        &mut self.draw_entries,
                    );
                }
                continue;
            }
            if let Some(gt) = world.get::<GlobalTransform>(entity) {
                push_sprite_if_visible(
                    gt.position,
                    gt.scale,
                    gt.rotation,
                    gt.z,
                    layer,
                    tex_key,
                    make_instance(gt.to_matrix().to_cols_array_2d()),
                    is_visible,
                    is_above_lod,
                    stats,
                    &mut next_order,
                    &mut self.draw_entries,
                );
            } else if let Some(transform) = world.get::<Transform>(entity) {
                push_sprite_if_visible(
                    transform.position,
                    transform.scale,
                    transform.rotation,
                    transform.z,
                    layer,
                    tex_key,
                    make_instance(transform.to_matrix().to_cols_array_2d()),
                    is_visible,
                    is_above_lod,
                    stats,
                    &mut next_order,
                    &mut self.draw_entries,
                );
            }
        }
        // ── Collect AtlasSprites: first collect (index, color, atlas handle) ──
        // Break the query iterator borrow before reading AssetServer and GlobalTransform.
        // atlas_entries_scratch is a field so its backing allocation is reused across frames.
        self.atlas_entries_scratch.clear();
        self.atlas_entries_scratch.extend(
            world
                .query::<AtlasSprite>()
                .map(|(e, s)| (e, s.index, s.color, s.atlas.clone())),
        );

        if !self.atlas_entries_scratch.is_empty() {
            if let Some(server) = world.resource::<AssetServer>() {
                for (entity, index, color, atlas_handle) in &self.atlas_entries_scratch {
                    if let Some(atlas) = server.get_atlas(atlas_handle) {
                        let uv = world
                            .get::<UvRect>(*entity)
                            .copied()
                            .unwrap_or_else(|| atlas.uv_rect(*index));
                        // SpriteFlip mirrors the sampled UV region (absent/default = no flip).
                        let flip = world
                            .get::<crate::components::SpriteFlip>(*entity)
                            .copied()
                            .unwrap_or_default();
                        let uv = uv.flipped(flip.x, flip.y);
                        // O(1) refcount bump; Arc::from(path()) would copy the string bytes.
                        let tex_key: Arc<str> = atlas.texture_path_arc();
                        let layer = world
                            .get::<crate::components::RenderLayer>(*entity)
                            .map(|l| l.0)
                            .unwrap_or(0);
                        if !layer_matches_mask(layer, layer_mask) {
                            continue;
                        }
                        if let Some(gt) = world.get::<GlobalTransform>(*entity) {
                            push_sprite_if_visible(
                                gt.position,
                                gt.scale,
                                gt.rotation,
                                gt.z,
                                layer,
                                tex_key,
                                InstanceRaw::single(
                                    gt.to_matrix().to_cols_array_2d(),
                                    color.to_array(),
                                    uv,
                                ),
                                is_visible,
                                is_above_lod,
                                stats,
                                &mut next_order,
                                &mut self.draw_entries,
                            );
                        } else if let Some(tr) = world.get::<Transform>(*entity) {
                            push_sprite_if_visible(
                                tr.position,
                                tr.scale,
                                tr.rotation,
                                tr.z,
                                layer,
                                tex_key,
                                InstanceRaw::single(
                                    tr.to_matrix().to_cols_array_2d(),
                                    color.to_array(),
                                    uv,
                                ),
                                is_visible,
                                is_above_lod,
                                stats,
                                &mut next_order,
                                &mut self.draw_entries,
                            );
                        }
                    }
                }
            }
        }

        // ── Collect ShaderMaterials: merge into the same (layer, z) stream as regular sprites.
        // No source clone here — the clone decision happens below, after the layer/cull
        // filters, so the *first surviving* entity of a new hash carries the source
        // (cloning per-entity at query time would clone once per entity sharing a new
        // material; deduping at query time could hand the source to an entity that is
        // then culled, leaving the pipeline uncompiled).
        let mat_ids: Vec<(crate::ecs::Entity, u64, [f32; 4])> = world
            .query::<ShaderMaterial>()
            .map(|(e, mat)| (e, mat.source_hash(), mat.params))
            .collect();

        // live_material_entities_scratch: set of entities currently holding a ShaderMaterial.
        // Populated from the full world-query result regardless of culling, so retaining
        // params_buffers to this set at frame end removes GPU buffers only for
        // despawned or material-removed entities. Field is cleared+reused each frame.
        self.material.live_material_entities_scratch.clear();
        self.material
            .live_material_entities_scratch
            .extend(mat_ids.iter().map(|(e, ..)| *e));

        // seen_new_hashes_scratch: hashes whose source has already been claimed by a
        // surviving entity this call — keeps frag_source clones to at most one per new
        // pipeline. Field is cleared+reused each frame.
        self.material.seen_new_hashes_scratch.clear();
        for (entity, hash, params) in mat_ids {
            let uv = world.get::<UvRect>(entity).copied().unwrap_or(UvRect::FULL);
            // SpriteFlip mirrors the sampled UV region (absent/default = no flip).
            let flip = world
                .get::<crate::components::SpriteFlip>(entity)
                .copied()
                .unwrap_or_default();
            let uv = uv.flipped(flip.x, flip.y);
            let sprite = match world.get::<Sprite>(entity) {
                Some(sprite) => sprite,
                None => continue,
            };
            let layer = world
                .get::<crate::components::RenderLayer>(entity)
                .map(|l| l.0)
                .unwrap_or(0);
            if !layer_matches_mask(layer, layer_mask) {
                continue;
            }
            let tex_key: Option<Arc<str>> = sprite
                .image_handle
                .as_ref()
                .map(|h| h.path_arc())
                .or_else(|| sprite.texture.clone());

            let (z, instance) = if let Some(gt) = world.get::<GlobalTransform>(entity) {
                if !is_visible(gt.position, gt.scale, gt.rotation) || !is_above_lod(gt.scale) {
                    stats.sprites_culled += 1;
                    continue;
                }
                (gt.z, InstanceRaw::from_global(gt, sprite, uv))
            } else if let Some(transform) = world.get::<Transform>(entity) {
                if !is_visible(transform.position, transform.scale, transform.rotation)
                    || !is_above_lod(transform.scale)
                {
                    stats.sprites_culled += 1;
                    continue;
                }
                (transform.z, InstanceRaw::from(transform, sprite, uv))
            } else {
                continue;
            };

            // Clone the WGSL source only for the first surviving entity of a hash not yet
            // compiled *for this target format*; every other entry carries an empty string.
            let frag_source = if !self
                .material
                .custom_pipelines
                .contains_key(&(hash, target_format))
                && self.material.seen_new_hashes_scratch.insert(hash)
            {
                world
                    .get::<ShaderMaterial>(entity)
                    .map(|m| m.frag_source().to_owned())
                    .unwrap_or_default()
            } else {
                String::new()
            };

            self.draw_entries.push(SpriteRenderEntry::material(
                layer,
                z,
                next_order,
                entity,
                hash,
                frag_source,
                params,
                tex_key,
                instance,
            ));
            next_order += 1;
        }
    }
}
