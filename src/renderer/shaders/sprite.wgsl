// ─── Camera uniform (orthographic projection matrix) ─────────────────────────
struct Camera {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: Camera;

// ─── Vertex input (shared quad) ──────────────────────────────────────────────
struct Vertex {
    @location(0) position: vec2<f32>,
    @location(1) uv:       vec2<f32>,
};

// ─── Instance input (one per sprite) ─────────────────────────────────────────
struct Instance {
    @location(2)  row0:         vec4<f32>,   // model matrix column 0
    @location(3)  row1:         vec4<f32>,   // model matrix column 1
    @location(4)  row2:         vec4<f32>,   // model matrix column 2
    @location(5)  row3:         vec4<f32>,   // model matrix column 3
    @location(6)  color:        vec4<f32>,   // RGBA color
    @location(7)  uv_offset:    vec2<f32>,   // from-frame UV origin
    @location(8)  uv_size:      vec2<f32>,   // from-frame UV size
    @location(9)  to_uv_offset: vec2<f32>,   // to-frame UV origin (crossfade)
    @location(10) to_uv_size:   vec2<f32>,   // to-frame UV size
    @location(11) blend:        f32,         // 0 = from only, 1 = to only
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv:    vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv_to: vec2<f32>,   // to-frame UV (used only when blend > 0)
    @location(3) blend: f32,
};

@vertex
fn vs_main(v: Vertex, inst: Instance) -> VertexOutput {
    let model = mat4x4<f32>(inst.row0, inst.row1, inst.row2, inst.row3);
    var out: VertexOutput;
    out.clip_pos = camera.view_proj * model * vec4<f32>(v.position, 0.0, 1.0);
    // Map quad UV [0,1] → sprite-sheet sub-region
    out.uv    = inst.uv_offset + v.uv * inst.uv_size;
    out.uv_to = inst.to_uv_offset + v.uv * inst.to_uv_size;
    out.blend = inst.blend;
    out.color = inst.color;
    return out;
}

// ─── Texture (falls back to a white 1×1 pixel when no sprite is set) ─────────
@group(1) @binding(0) var t_sprite: texture_2d<f32>;
@group(1) @binding(1) var s_sprite: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Common path (blend == 0): single sample — identical to the original behavior.
    var sampled = textureSample(t_sprite, s_sprite, in.uv);
    if (in.blend > 0.0) {
        let to = textureSample(t_sprite, s_sprite, in.uv_to);
        sampled = mix(sampled, to, in.blend);
    }
    return sampled * in.color;
}
