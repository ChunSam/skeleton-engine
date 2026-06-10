// Shared fullscreen-quad vertex stage.
//
// Emits a 6-vertex (2-triangle) quad that covers the entire NDC clip space
// [-1,+1] × [-1,+1]. UV coordinates map the quad to [0,1] × [0,1] with
// (0,0) at the top-left and (1,0) at the top-right.
//
// Used by fade.rs and lighting.rs. Fragment shaders that don't need UV
// may simply omit the `@location(0) uv` input — unused interpolants are
// silently dropped by the GPU.

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VOut {
    var pos = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0),
        vec2(-1.0,  1.0), vec2(1.0, -1.0), vec2( 1.0, 1.0),
    );
    var uv = array<vec2<f32>, 6>(
        vec2(0.0, 1.0), vec2(1.0, 1.0), vec2(0.0, 0.0),
        vec2(0.0, 0.0), vec2(1.0, 1.0), vec2(1.0, 0.0),
    );
    var out: VOut;
    out.pos = vec4(pos[idx], 0.0, 1.0);
    out.uv  = uv[idx];
    return out;
}
