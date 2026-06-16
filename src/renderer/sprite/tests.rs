use super::textures::file_texture_aliases;
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

    let order: Vec<String> = sorted_ui_primitives(&rects, &images)
        .iter()
        .map(|primitive| match primitive.kind {
            UiPrimitiveKind::Image => primitive.texture_key.clone().unwrap(),
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
