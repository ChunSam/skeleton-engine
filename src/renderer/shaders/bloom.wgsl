// ─── Real multi-pass bloom ───────────────────────────────────────────────────
// Three stages, all sharing one bind-group layout (input texture + sampler + uniform):
//   fs_prefilter — bright-pass: keep only luminance above the threshold (scene → bloom tex)
//   fs_blur      — separable Gaussian, one axis per pass, ping-ponged N times
//   fs_composite — output the blurred bloom scaled by intensity (additive blend onto the scene)
// Driven by src/renderer/bloom.rs. Distinct from the cheap inline 4-tap in post_process.wgsl.

struct Uniforms {
    threshold: f32,        // fs_prefilter: luminance bright-pass threshold
    intensity: f32,        // fs_composite: additive bloom strength
    texel:     vec2<f32>,  // fs_blur: (1/bloom_width, 1/bloom_height)
    direction: vec2<f32>,  // fs_blur: blur axis — (1,0) horizontal | (0,1) vertical
    _pad:      vec2<f32>,
}

@group(0) @binding(0) var src_tex:     texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;
@group(0) @binding(2) var<uniform>  u: Uniforms;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0)       uv:  vec2<f32>,
}

// Fullscreen triangle, no vertex buffer (same as the post-process pass).
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VOut {
    var clip = array<vec2<f32>, 3>(
        vec2<f32>(-1.0,  3.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, -1.0),
        vec2<f32>(0.0,  1.0),
        vec2<f32>(2.0,  1.0),
    );
    var out: VOut;
    out.pos = vec4<f32>(clip[vi], 0.0, 1.0);
    out.uv  = uvs[vi];
    return out;
}

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// Bright-pass: keep the portion of the colour whose luminance exceeds the threshold, scaled by a
// soft knee. Same shape as the old inline 4-tap so `bloom_threshold` feels familiar.
@fragment
fn fs_prefilter(in: VOut) -> @location(0) vec4<f32> {
    let c   = textureSample(src_tex, src_sampler, in.uv).rgb;
    let lum = luminance(c);
    let w   = max(0.0, lum - u.threshold) / max(0.001, 1.0 - u.threshold);
    return vec4<f32>(c * w, 1.0);
}

// Separable Gaussian (9-tap, normalized weights). Sampled along `u.direction` (one axis per pass).
@fragment
fn fs_blur(in: VOut) -> @location(0) vec4<f32> {
    // `var` (not `let`): a dynamically-indexed array needs an address space (naga rejects
    // dynamic indexing of a `let` array).
    var weights = array<f32, 5>(0.227027, 0.194595, 0.121622, 0.054054, 0.016216);
    let step = u.texel * u.direction;
    var result = textureSample(src_tex, src_sampler, in.uv).rgb * weights[0];
    for (var i = 1; i < 5; i++) {
        let off = step * f32(i);
        result += textureSample(src_tex, src_sampler, in.uv + off).rgb * weights[i];
        result += textureSample(src_tex, src_sampler, in.uv - off).rgb * weights[i];
    }
    return vec4<f32>(result, 1.0);
}

// Additive composite: the pipeline's blend state adds this onto the scene intermediate, so just
// emit the blurred bloom scaled by intensity. Alpha is held by the blend state (dst preserved).
@fragment
fn fs_composite(in: VOut) -> @location(0) vec4<f32> {
    let bloom = textureSample(src_tex, src_sampler, in.uv).rgb;
    return vec4<f32>(bloom * u.intensity, 1.0);
}
