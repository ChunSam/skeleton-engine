use super::*;

// ─── Vertex struct uploaded to the GPU ────────────────────────────────────────
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(super) struct Vertex {
    pub(super) position: [f32; 2],
    pub(super) uv: [f32; 2],
}

// Unit quad: center (0,0), size 1×1
pub(super) const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, -0.5],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5],
        uv: [0.0, 1.0],
    },
];
pub(super) const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

// ─── GPU data for a single sprite instance ────────────────────────────────────
// Layout: [model matrix 64B][color 16B][uv_offset 8B][uv_size 8B][to_uv_offset 8B][to_uv_size 8B][blend 4B] = 116B
//
// `to_uv_*` + `blend` are extra fields for animation crossfade. When `blend == 0`
// the shader samples only `uv_offset/uv_size`, giving identical output to the original
// single-frame render (additive — non-crossfading sprites are byte-for-byte identical).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(super) struct InstanceRaw {
    pub(super) model: [[f32; 4]; 4],   // offset   0 — 64 bytes
    pub(super) color: [f32; 4],        // offset  64 — 16 bytes (shader_location 6)
    pub(super) uv_offset: [f32; 2],    // offset  80 —  8 bytes (shader_location 7)  from frame
    pub(super) uv_size: [f32; 2],      // offset  88 —  8 bytes (shader_location 8)
    pub(super) to_uv_offset: [f32; 2], // offset  96 —  8 bytes (shader_location 9)  to frame
    pub(super) to_uv_size: [f32; 2],   // offset 104 —  8 bytes (shader_location 10)
    pub(super) blend: f32,             // offset 112 —  4 bytes (shader_location 11) 0=single,1=to
}

impl InstanceRaw {
    /// Single-frame instance (no blend). Sets `to` equal to `from` and `blend = 0`.
    pub(super) fn single(model: [[f32; 4]; 4], color: [f32; 4], uv: UvRect) -> Self {
        Self {
            model,
            color,
            uv_offset: [uv.u_offset, uv.v_offset],
            uv_size: [uv.u_size, uv.v_size],
            to_uv_offset: [uv.u_offset, uv.v_offset],
            to_uv_size: [uv.u_size, uv.v_size],
            blend: 0.0,
        }
    }

    /// Instance that crossfades `from`→`to` frames by `weight`. The shader computes `mix(from, to, weight)`.
    pub(super) fn blended(
        model: [[f32; 4]; 4],
        color: [f32; 4],
        from: UvRect,
        to: UvRect,
        weight: f32,
    ) -> Self {
        Self {
            model,
            color,
            uv_offset: [from.u_offset, from.v_offset],
            uv_size: [from.u_size, from.v_size],
            to_uv_offset: [to.u_offset, to.v_offset],
            to_uv_size: [to.u_size, to.v_size],
            blend: weight,
        }
    }

    pub(super) fn from(transform: &Transform, sprite: &Sprite, uv: UvRect) -> Self {
        Self::single(
            transform.to_matrix().to_cols_array_2d(),
            sprite.color.to_array(),
            uv,
        )
    }

    pub(super) fn from_global(gt: &GlobalTransform, sprite: &Sprite, uv: UvRect) -> Self {
        Self::single(
            gt.to_matrix().to_cols_array_2d(),
            sprite.color.to_array(),
            uv,
        )
    }

    pub(super) fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceRaw>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 80,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 88,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 96,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 104,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 112,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

// ─── GPU data for a single UI primitive instance ──────────────────────────────
// A trimmed sprite instance (no animation crossfade fields) plus rounded-rect SDF
// params. Rendered by the dedicated UI pipeline (`shaders/ui.wgsl`); the sprite
// pipeline keeps using `InstanceRaw` unchanged.
//
// Layout: [model 64B][color 16B][uv_offset 8B][uv_size 8B][px_size 8B][corner 8B] = 112B.
// `corner = [radius, border]` in screen px; `[0, 0]` makes the shader take a fast
// path identical to the original plain quad.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(super) struct UiInstanceRaw {
    pub(super) model: [[f32; 4]; 4], // offset   0 — 64 bytes
    pub(super) color: [f32; 4],      // offset  64 — 16 bytes (shader_location 6)
    pub(super) uv_offset: [f32; 2],  // offset  80 —  8 bytes (shader_location 7)
    pub(super) uv_size: [f32; 2],    // offset  88 —  8 bytes (shader_location 8)
    pub(super) px_size: [f32; 2],    // offset  96 —  8 bytes (shader_location 9)
    pub(super) corner: [f32; 2],     // offset 104 —  8 bytes (shader_location 10) [radius, border]
}

impl UiInstanceRaw {
    pub(super) fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<UiInstanceRaw>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 80,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 88,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 96,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 104,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

// CameraUniform is defined in `crate::renderer` (shared with gpu_particle).
pub(super) use crate::renderer::CameraUniform;
