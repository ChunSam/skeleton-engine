use super::*;

// ─── GPU에 올라가는 버텍스 구조체 ─────────────────────────────────────────────
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(super) struct Vertex {
    pub(super) position: [f32; 2],
    pub(super) uv: [f32; 2],
}

// 단위 쿼드: 중심 (0,0), 크기 1×1
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

// ─── 인스턴스(스프라이트 1개)의 GPU 데이터 ────────────────────────────────────
// 구조: [모델행렬 64B][color 16B][uv_offset 8B][uv_size 8B][to_uv_offset 8B][to_uv_size 8B][blend 4B] = 116B
//
// `to_uv_*` + `blend`는 애니메이션 크로스페이드용 추가 필드다. `blend == 0`이면
// 셰이더가 `uv_offset/uv_size`만 샘플링하므로 기존 단일 프레임 렌더와 결과가 동일하다
// (추가형 — 비크로스페이드 스프라이트는 바이트 단위로 동일하게 렌더된다).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(super) struct InstanceRaw {
    pub(super) model: [[f32; 4]; 4],   // offset   0 — 64 bytes
    pub(super) color: [f32; 4],        // offset  64 — 16 bytes (shader_location 6)
    pub(super) uv_offset: [f32; 2],    // offset  80 —  8 bytes (shader_location 7)  from 프레임
    pub(super) uv_size: [f32; 2],      // offset  88 —  8 bytes (shader_location 8)
    pub(super) to_uv_offset: [f32; 2], // offset  96 —  8 bytes (shader_location 9)  to 프레임
    pub(super) to_uv_size: [f32; 2],   // offset 104 —  8 bytes (shader_location 10)
    pub(super) blend: f32,             // offset 112 —  4 bytes (shader_location 11) 0=단일,1=to
}

impl InstanceRaw {
    /// 단일 프레임(블렌드 없음) 인스턴스. `to`를 `from`과 동일하게 두고 `blend = 0`.
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

    /// `from`→`to` 프레임을 `weight`로 크로스페이드하는 인스턴스. 셰이더가 `mix(from, to, weight)`.
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

// ─── 카메라 유니폼 ─────────────────────────────────────────────────────────────
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(super) struct CameraUniform {
    pub(super) view_proj: [[f32; 4]; 4],
}
