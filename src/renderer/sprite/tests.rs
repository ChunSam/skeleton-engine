use super::textures::{file_texture_aliases, reload_format};
use super::ui_primitives::{sorted_ui_primitives, UiPrimitiveKind};
use super::*;

/// Verify that the promoted per-frame scratch fields start empty and that the
/// sort + assign pipeline (which uses them) produces the same output whether the
/// scratch storage was just created or was previously populated (reuse).
///
/// This is a logic-only test — it does not require a real GPU device. It exercises
/// the `sort_render_entries` + `assign_instance_offsets` path that the scratch
/// fields feed into, asserting behavior is identical on a fresh vec vs a vec that
/// was pre-populated and then cleared (simulating steady-state reuse).
#[test]
fn scratch_field_reuse_is_behavior_identical() {
    let instance = raw();
    let entries_factory = || {
        vec![
            sprite(0, 1.0, 0, "tex_a"),
            sprite(0, 0.0, 1, "tex_b"),
            sprite(1, 5.0, 2, "tex_a"),
        ]
    };

    // Run 1: fresh scratch vecs.
    let mut entries1 = entries_factory();
    let mut sprite_scratch1: Vec<InstanceRaw> = Vec::new();
    let mut mat_scratch1: Vec<InstanceRaw> = Vec::new();
    sort_render_entries(&mut entries1);
    assign_instance_offsets(&mut entries1, &mut sprite_scratch1, &mut mat_scratch1);

    // Run 2: pre-populate scratch vecs with garbage, then clear (simulates frame reuse).
    let mut entries2 = entries_factory();
    let mut sprite_scratch2: Vec<InstanceRaw> = vec![instance, instance, instance];
    let mut mat_scratch2: Vec<InstanceRaw> = vec![instance];
    sprite_scratch2.clear();
    mat_scratch2.clear();
    sort_render_entries(&mut entries2);
    assign_instance_offsets(&mut entries2, &mut sprite_scratch2, &mut mat_scratch2);

    // Both runs must yield the same instance order and offsets.
    assert_eq!(
        sprite_scratch1.len(),
        sprite_scratch2.len(),
        "sprite instance count must be identical across runs"
    );
    assert_eq!(
        mat_scratch1.len(),
        mat_scratch2.len(),
        "material instance count must be identical across runs"
    );
    let offsets1: Vec<usize> = entries1
        .iter()
        .filter_map(|e| {
            if let SpriteRenderKind::Sprite {
                instance_offset, ..
            } = &e.kind
            {
                Some(*instance_offset)
            } else {
                None
            }
        })
        .collect();
    let offsets2: Vec<usize> = entries2
        .iter()
        .filter_map(|e| {
            if let SpriteRenderKind::Sprite {
                instance_offset, ..
            } = &e.kind
            {
                Some(*instance_offset)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        offsets1, offsets2,
        "sprite instance offsets must be identical whether scratch was fresh or reused"
    );
}

fn raw() -> InstanceRaw {
    InstanceRaw::single([[0.0; 4]; 4], [1.0; 4], UvRect::FULL)
}

fn sprite(layer: i32, z: f32, order: usize, texture_key: &str) -> SpriteRenderEntry {
    SpriteRenderEntry::sprite(layer, z, order, Arc::from(texture_key), raw())
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn file_texture_aliases_include_requested_and_canonical_paths() {
    let dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("texture-alias-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("image.png");
    std::fs::write(&path, b"not a png").unwrap();

    let relative = path.strip_prefix(std::env::current_dir().unwrap()).unwrap();
    let relative = relative.to_string_lossy().to_string();
    let canonical = path.canonicalize().unwrap().to_string_lossy().to_string();
    let aliases = file_texture_aliases(&relative);

    assert_eq!(aliases, vec![relative, canonical]);

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn file_texture_aliases_preserve_missing_or_synthetic_keys() {
    let aliases = file_texture_aliases("__missing_or_synthetic_texture_key__");
    assert_eq!(aliases, vec!["__missing_or_synthetic_texture_key__"]);
}

// --- hot-reload format preservation ------------------------------------------------------
//
// `reload_texture` used to call `Texture::from_path`, which hardcodes `Rgba8UnormSrgb`. A data
// texture uploaded linear through `load_texture_with_format` therefore came back from an edit
// with an sRGB decode attached — the file looked right on the run that loaded it and wrong on
// every run after a save. Building a real cache entry needs a GPU, so the policy is tested here
// through the injected lookup and only `t.texture.format()` is left to the device.

#[test]
fn a_reload_keeps_a_linear_data_texture_linear() {
    let aliases = file_texture_aliases("normal_map.png");
    let format = reload_format(&aliases, |k| {
        (k == "normal_map.png").then_some(wgpu::TextureFormat::Rgba8Unorm)
    });
    assert_eq!(
        format,
        wgpu::TextureFormat::Rgba8Unorm,
        "a hot-reload must not re-upload a linear data texture as sRGB"
    );
}

#[test]
fn a_reload_keeps_an_srgb_colour_texture_srgb() {
    // The common case, pinned so the fix cannot invert into "linear for everything".
    let aliases = file_texture_aliases("hero.png");
    let format = reload_format(&aliases, |_| Some(wgpu::TextureFormat::Rgba8UnormSrgb));
    assert_eq!(format, wgpu::TextureFormat::Rgba8UnormSrgb);
}

#[test]
fn an_uncached_path_reloads_as_srgb() {
    // Nothing cached under any alias → the historical `from_path` default, unchanged.
    let aliases = file_texture_aliases("never_loaded.png");
    assert_eq!(
        reload_format(&aliases, |_| None),
        wgpu::TextureFormat::Rgba8UnormSrgb
    );
}

#[test]
fn a_reload_finds_the_format_under_any_alias() {
    // `load_texture` registers a texture under both the requested path and its canonical key,
    // and `reload_texture` adds every cache key that canonicalises to the same asset. The
    // requested path is not guaranteed to be the one that hits.
    let aliases = vec!["a/b.png".to_string(), "/canonical/a/b.png".to_string()];
    let format = reload_format(&aliases, |k| {
        (k == "/canonical/a/b.png").then_some(wgpu::TextureFormat::Rgba8Unorm)
    });
    assert_eq!(format, wgpu::TextureFormat::Rgba8Unorm);
}

fn material(layer: i32, z: f32, order: usize, entity_id: u32) -> SpriteRenderEntry {
    SpriteRenderEntry::material(
        layer,
        z,
        order,
        crate::ecs::Entity::from_raw_parts(entity_id, 0),
        1,
        String::new(),
        [0.0; 4],
        None,
        raw(),
    )
}

#[test]
fn shared_quad_uses_top_left_uv_origin() {
    assert_eq!(VERTICES[0].position, [-0.5, -0.5]);
    assert_eq!(VERTICES[0].uv, [0.0, 0.0]);
    assert_eq!(VERTICES[1].position, [0.5, -0.5]);
    assert_eq!(VERTICES[1].uv, [1.0, 0.0]);
    assert_eq!(VERTICES[2].position, [0.5, 0.5]);
    assert_eq!(VERTICES[2].uv, [1.0, 1.0]);
    assert_eq!(VERTICES[3].position, [-0.5, 0.5]);
    assert_eq!(VERTICES[3].uv, [0.0, 1.0]);
}

fn describe(entry: &SpriteRenderEntry) -> String {
    match &entry.kind {
        SpriteRenderKind::Sprite { texture_key, .. } => {
            format!("S:{texture_key}@{}", entry.sort.z)
        }
        SpriteRenderKind::Material { entity, .. } => {
            format!(
                "M:{}:{}@{}",
                entity.index(),
                entity.generation(),
                entry.sort.z
            )
        }
    }
}

fn sprite_runs(entries: &[SpriteRenderEntry]) -> Vec<(String, usize)> {
    let mut runs = Vec::new();
    let mut i = 0usize;
    while i < entries.len() {
        match &entries[i].kind {
            SpriteRenderKind::Sprite {
                texture_key,
                instance_offset,
                ..
            } => {
                let run_key = Arc::clone(texture_key);
                let run_start_offset = *instance_offset;
                let mut run_len = 1usize;
                i += 1;
                while i < entries.len() {
                    match &entries[i].kind {
                        SpriteRenderKind::Sprite {
                            texture_key,
                            instance_offset,
                            ..
                        } if texture_key.as_ref() == run_key.as_ref()
                            && *instance_offset == run_start_offset + run_len =>
                        {
                            run_len += 1;
                            i += 1;
                        }
                        _ => break,
                    }
                }
                runs.push((run_key.to_string(), run_len));
            }
            SpriteRenderKind::Material { .. } => i += 1,
        }
    }
    runs
}

/// A hidden `ShaderMaterial` entity stays in the **live** set while dropping out of the **drawn**
/// list.
///
/// `SpriteRenderer::render` ends with `params_buffers.retain(|e, _| live.contains(e))`, so the
/// live set is what keeps an entity's GPU params buffer and bind group alive. Until v0.153.2 the
/// live set was derived from the drawn list, which meant hiding a material sprite destroyed both
/// and showing it again rebuilt them — a GPU allocation per toggle of the editor's visibility
/// checkbox. The two comments above that code already described the behaviour asserted here; the
/// code was the half that was wrong.
#[test]
fn a_hidden_material_entity_keeps_its_gpu_buffers() {
    use crate::components::Hidden;
    use crate::ecs::World;
    use crate::material::ShaderMaterial;

    let mut world = World::new();
    let shown = world.spawn();
    world.add_component(shown, ShaderMaterial::new("// a", [0.0; 4]));
    let hidden = world.spawn();
    world.add_component(hidden, ShaderMaterial::new("// b", [1.0; 4]));
    world.add_component(hidden, Hidden);

    let mut live = std::collections::HashSet::new();
    let mut drawn = Vec::new();
    super::collect::split_material_entities(&world, &mut live, &mut drawn);

    assert!(
        live.contains(&hidden),
        "a hidden material entity must stay live, or the retain frees its params buffer"
    );
    assert!(live.contains(&shown), "control: a shown entity is live too");
    let drawn_ids: Vec<_> = drawn.iter().map(|(e, ..)| *e).collect();
    assert_eq!(
        drawn_ids,
        vec![shown],
        "control: the hidden entity must still be excluded from drawing"
    );

    // Despawning is what may free the buffers — the distinction the live set exists to draw.
    world.despawn(shown);
    super::collect::split_material_entities(&world, &mut live, &mut drawn);
    assert!(
        !live.contains(&shown),
        "a despawned entity must leave the live set so its GPU buffers are reclaimed"
    );
    assert!(live.contains(&hidden), "still hidden, still live");

    // Both outputs are cleared on entry, so a reused scratch set cannot accumulate stale entities.
    assert_eq!(live.len(), 1);
    assert!(drawn.is_empty());
}

/// The untextured-sprite fallback key is **interned**: every `Sprite` with no texture shares one
/// `Arc<str>` instead of allocating its own, every frame.
///
/// Measured against the unfixed `unwrap_or_else(|| Arc::from(""))`, 1000 untextured sprites cost
/// 1000 allocations / 16,000 bytes requested, with all 1000 pointers distinct; interned it is 0
/// and 1. What is asserted here is **pointer identity**, not an allocation count: counting needs a
/// `#[global_allocator]`, which lives in `tests/per_frame_alloc.rs` — and that binary cannot reach
/// this path anyway, since building a `SpriteRenderer` needs a GPU device. Pointer identity is the
/// property that makes the count zero, and it is checkable here.
#[test]
fn the_untextured_fallback_key_is_interned() {
    let a = super::collect::empty_tex_key();
    let b = super::collect::empty_tex_key();
    assert!(
        Arc::ptr_eq(&a, &b),
        "every untextured sprite must share one Arc<str>, not allocate its own each frame"
    );
    assert_eq!(
        &*a, "",
        "the interned key must still be the empty string the texture cache looks up"
    );

    // Control: interning the *empty* key must not collapse distinct keys. A real texture path is
    // its own allocation, and batching still separates it from the untextured run — which it does
    // by contents, never by address, and that is what makes sharing the pointer safe.
    let real: Arc<str> = Arc::from("assets/hero.png");
    assert!(
        !Arc::ptr_eq(&a, &real),
        "a real texture key must not share the interned empty key's allocation"
    );
    assert_ne!(a.as_ref(), real.as_ref());
}

#[test]
fn sort_uses_layer_and_z_before_texture_batching() {
    let mut entries = vec![
        sprite(0, 2.0, 0, "tex_a"),
        sprite(0, 1.0, 1, "tex_b"),
        sprite(0, 0.0, 2, "tex_a"),
        material(0, 1.5, 3, 99),
        sprite(-1, 50.0, 4, "behind"),
        sprite(1, -50.0, 5, "front"),
    ];

    sort_render_entries(&mut entries);
    let mut sprite_instances = Vec::new();
    let mut material_instances = Vec::new();
    assign_instance_offsets(&mut entries, &mut sprite_instances, &mut material_instances);

    let order: Vec<String> = entries.iter().map(describe).collect();
    assert_eq!(
        order,
        vec![
            "S:behind@50",
            "S:tex_a@0",
            "S:tex_b@1",
            "M:99:0@1.5",
            "S:tex_a@2",
            "S:front@-50",
        ]
    );
    assert_eq!(
        sprite_runs(&entries),
        vec![
            ("behind".to_string(), 1),
            ("tex_a".to_string(), 1),
            ("tex_b".to_string(), 1),
            ("tex_a".to_string(), 1),
            ("front".to_string(), 1),
        ]
    );
}

#[test]
fn ui_primitives_sort_by_z_type_then_queue_order() {
    let rects = vec![
        DrawRect::new(0.0, 0.0, 10.0, 10.0, [1.0, 0.0, 0.0, 1.0]).with_z(1.0),
        DrawRect::new(0.0, 0.0, 20.0, 20.0, [0.0, 1.0, 0.0, 1.0]).with_z(0.5),
        DrawRect::new(0.0, 0.0, 30.0, 30.0, [0.0, 0.0, 1.0, 1.0]).with_z(1.0),
    ];
    let images = vec![
        DrawImage::textured(0.0, 0.0, 10.0, 10.0, "image-a.png").with_z(1.0),
        DrawImage::textured(0.0, 0.0, 20.0, 20.0, "image-b.png").with_z(0.25),
        DrawImage::textured(0.0, 0.0, 30.0, 30.0, "image-c.png").with_z(1.0),
    ];

    // The scratch buffer is deliberately pre-dirtied: `sorted_ui_primitives` clears it, and a
    // frame that forgot to would leak the previous frame's primitives into this one's draw order.
    let mut primitives = Vec::new();
    sorted_ui_primitives(&mut primitives, &rects, &[]);
    assert!(
        !primitives.is_empty(),
        "control: the first pass must fill the scratch"
    );
    sorted_ui_primitives(&mut primitives, &rects, &images);

    let order: Vec<String> = primitives
        .iter()
        .map(|primitive| match primitive.kind {
            UiPrimitiveKind::Image => primitive.texture_key.clone().unwrap().to_string(),
            UiPrimitiveKind::Rect => format!("rect-{}", primitive.order),
        })
        .collect();

    assert_eq!(
        order,
        vec![
            "image-b.png",
            "rect-1",
            "image-a.png",
            "image-c.png",
            "rect-0",
            "rect-2",
        ]
    );
}
