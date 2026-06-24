// ─── Mip-chain ("dual filter") bloom ─────────────────────────────────────────
// Physically-based bloom (Jimenez — "Next Generation Post Processing in CoD: Advanced Warfare").
// A progressive DOWNSAMPLE chain (13-tap filter) builds a mip pyramid of the bright highlights,
// then an UPSAMPLE chain (3x3 tent, additive) accumulates the levels back up, producing a wide,
// smooth, energy-preserving glow. Replaces the older fixed-half-res separable Gaussian (whose
// spread was bounded by kernel x iterations and looked boxy when pushed).
//
//   fs_prefilter  — 13-tap downsample of the scene + threshold knee   (scene  -> mip0)
//   fs_downsample — 13-tap downsample                                 (mip i-1 -> mip i)
//   fs_upsample   — 3x3 tent, additive blend                          (mip i+1 -> mip i)
//   fs_composite  — mip0 * intensity, additive blend                  (mip0   -> scene)
// Driven by src/renderer/bloom.rs. Distinct from the cheap inline 4-tap in post_process.wgsl.

struct Uniforms {
    threshold: f32,        // fs_prefilter: luminance bright-pass threshold
    intensity: f32,        // fs_composite: additive bloom strength
    radius:    f32,        // fs_upsample:  tent radius, in source texels
    _pad0:     f32,
    texel:     vec2<f32>,  // 1 / source-resolution for this pass
    _pad1:     vec2<f32>,
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

// 13-tap downsample filter (Jimenez 2014). `t` = source texel size. Reads a 4x4 neighbourhood in
// overlapping groups and weights them so the half-resolution result is a stable, alias-resistant
// average (the wide footprint is what keeps the downsampled pyramid from shimmering).
fn downsample_13(uv: vec2<f32>, t: vec2<f32>) -> vec3<f32> {
    let a = textureSample(src_tex, src_sampler, uv + t * vec2<f32>(-2.0,  2.0)).rgb;
    let b = textureSample(src_tex, src_sampler, uv + t * vec2<f32>( 0.0,  2.0)).rgb;
    let c = textureSample(src_tex, src_sampler, uv + t * vec2<f32>( 2.0,  2.0)).rgb;
    let d = textureSample(src_tex, src_sampler, uv + t * vec2<f32>(-2.0,  0.0)).rgb;
    let e = textureSample(src_tex, src_sampler, uv + t * vec2<f32>( 0.0,  0.0)).rgb;
    let f = textureSample(src_tex, src_sampler, uv + t * vec2<f32>( 2.0,  0.0)).rgb;
    let g = textureSample(src_tex, src_sampler, uv + t * vec2<f32>(-2.0, -2.0)).rgb;
    let h = textureSample(src_tex, src_sampler, uv + t * vec2<f32>( 0.0, -2.0)).rgb;
    let i = textureSample(src_tex, src_sampler, uv + t * vec2<f32>( 2.0, -2.0)).rgb;
    let j = textureSample(src_tex, src_sampler, uv + t * vec2<f32>(-1.0,  1.0)).rgb;
    let k = textureSample(src_tex, src_sampler, uv + t * vec2<f32>( 1.0,  1.0)).rgb;
    let l = textureSample(src_tex, src_sampler, uv + t * vec2<f32>(-1.0, -1.0)).rgb;
    let m = textureSample(src_tex, src_sampler, uv + t * vec2<f32>( 1.0, -1.0)).rgb;
    var r = e * 0.125;
    r += (a + c + g + i) * 0.03125;
    r += (b + d + f + h) * 0.0625;
    r += (j + k + l + m) * 0.125;
    return r;
}

// 3x3 tent upsample. `t` = source (smaller-mip) texel size, scaled by the filter radius. Combined
// with bilinear magnification this spreads each level smoothly into the next-larger one.
fn upsample_tent(uv: vec2<f32>, t: vec2<f32>) -> vec3<f32> {
    let r = t * u.radius;
    var s = textureSample(src_tex, src_sampler, uv + vec2<f32>(-r.x,  r.y)).rgb;
    s += textureSample(src_tex, src_sampler, uv + vec2<f32>( 0.0,  r.y)).rgb * 2.0;
    s += textureSample(src_tex, src_sampler, uv + vec2<f32>( r.x,  r.y)).rgb;
    s += textureSample(src_tex, src_sampler, uv + vec2<f32>(-r.x,  0.0)).rgb * 2.0;
    s += textureSample(src_tex, src_sampler, uv + vec2<f32>( 0.0,  0.0)).rgb * 4.0;
    s += textureSample(src_tex, src_sampler, uv + vec2<f32>( r.x,  0.0)).rgb * 2.0;
    s += textureSample(src_tex, src_sampler, uv + vec2<f32>(-r.x, -r.y)).rgb;
    s += textureSample(src_tex, src_sampler, uv + vec2<f32>( 0.0, -r.y)).rgb * 2.0;
    s += textureSample(src_tex, src_sampler, uv + vec2<f32>( r.x, -r.y)).rgb;
    return s * (1.0 / 16.0);
}

// Bright-pass: downsample the scene (13-tap) into mip0, then keep the portion whose luminance
// exceeds the threshold (soft knee — same shape as before, so `bloom_threshold` feels familiar).
@fragment
fn fs_prefilter(in: VOut) -> @location(0) vec4<f32> {
    let c   = downsample_13(in.uv, u.texel);
    let lum = luminance(c);
    let w   = max(0.0, lum - u.threshold) / max(0.001, 1.0 - u.threshold);
    return vec4<f32>(c * w, 1.0);
}

// Downsample one pyramid level into the next (no threshold — only the prefilter thresholds).
@fragment
fn fs_downsample(in: VOut) -> @location(0) vec4<f32> {
    return vec4<f32>(downsample_13(in.uv, u.texel), 1.0);
}

// Upsample the smaller level; the pipeline's additive blend adds it onto the larger level's
// already-downsampled content (mip i = downsample[i] + tent(mip i+1)).
@fragment
fn fs_upsample(in: VOut) -> @location(0) vec4<f32> {
    return vec4<f32>(upsample_tent(in.uv, u.texel), 1.0);
}

// Additive composite: the pipeline's blend adds this onto the scene intermediate, so just emit the
// accumulated bloom (mip0) scaled by intensity. Alpha is held by the blend state (dst preserved).
@fragment
fn fs_composite(in: VOut) -> @location(0) vec4<f32> {
    let bloom = textureSample(src_tex, src_sampler, in.uv).rgb;
    return vec4<f32>(bloom * u.intensity, 1.0);
}
