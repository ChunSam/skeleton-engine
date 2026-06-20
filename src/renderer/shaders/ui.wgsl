// UI primitive shader — textured/colored quad with an optional rounded-rect SDF.
//
// When `corner == (0, 0)` the fragment shader takes a fast path that is byte-identical
// to the plain sprite-UI quad (texture * color), so ordinary `DrawRect`s and every
// `DrawImage` render exactly as before. A positive `corner.x` (corner radius, px)
// rounds the quad; a positive `corner.y` (border width, px) turns the fill into an
// inset outline ring — this is how the keyboard focus ring draws rounded corners.

// ─── Camera uniform (orthographic screen-space projection) ───────────────────
struct Camera {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: Camera;

// ─── Vertex input (shared unit quad) ─────────────────────────────────────────
struct Vertex {
    @location(0) position: vec2<f32>,
    @location(1) uv:       vec2<f32>,
};

// ─── Instance input (one per UI primitive) ───────────────────────────────────
struct Instance {
    @location(2)  row0:      vec4<f32>,   // model matrix column 0
    @location(3)  row1:      vec4<f32>,   // model matrix column 1
    @location(4)  row2:      vec4<f32>,   // model matrix column 2
    @location(5)  row3:      vec4<f32>,   // model matrix column 3
    @location(6)  color:     vec4<f32>,   // RGBA color
    @location(7)  uv_offset: vec2<f32>,   // UV origin
    @location(8)  uv_size:   vec2<f32>,   // UV size
    @location(9)  px_size:   vec2<f32>,   // quad size in screen px
    @location(10) corner:    vec2<f32>,   // x = corner radius (px), y = border width (px)
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv:      vec2<f32>,
    @location(1) color:   vec4<f32>,
    @location(2) local:   vec2<f32>,   // centered px coords, range [-half, half]
    @location(3) px_half: vec2<f32>,
    @location(4) corner:  vec2<f32>,
};

@vertex
fn vs_main(v: Vertex, inst: Instance) -> VertexOutput {
    let model = mat4x4<f32>(inst.row0, inst.row1, inst.row2, inst.row3);
    var out: VertexOutput;
    out.clip_pos = camera.view_proj * model * vec4<f32>(v.position, 0.0, 1.0);
    out.uv      = inst.uv_offset + v.uv * inst.uv_size;
    out.color   = inst.color;
    out.local   = v.position * inst.px_size;   // v.position ∈ [-0.5, 0.5]
    out.px_half = inst.px_size * 0.5;
    out.corner  = inst.corner;
    return out;
}

// ─── Texture (falls back to a white 1×1 pixel for solid-color rects) ─────────
@group(1) @binding(0) var t_tex: texture_2d<f32>;
@group(1) @binding(1) var s_tex: sampler;

// Signed distance to a rounded box centered at the origin (Inigo Quilez's formula).
// `b` is the half-extent, `r` the corner radius.
fn sd_round_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSample(t_tex, s_tex, in.uv) * in.color;
    // Fast path: plain quad (no rounding, no border) — identical to the old sprite UI pass.
    if (in.corner.x <= 0.0 && in.corner.y <= 0.0) {
        return base;
    }
    // Clamp the radius so it never exceeds the half-size on either axis.
    let radius = min(in.corner.x, min(in.px_half.x, in.px_half.y));
    let d = sd_round_box(in.local, in.px_half, radius);
    let aa = max(fwidth(d), 1e-4);
    var mask = 1.0 - smoothstep(-aa, aa, d);     // filled rounded rect
    if (in.corner.y > 0.0) {
        // Inset outline ring of width `corner.y`, matching the historical 4-bar focus ring
        // (which drew its bars just inside the widget bounds).
        let inner = 1.0 - smoothstep(-aa, aa, d + in.corner.y);
        mask = clamp(mask - inner, 0.0, 1.0);
    }
    return vec4<f32>(base.rgb, base.a * mask);
}
