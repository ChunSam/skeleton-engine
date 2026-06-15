// ─── Post-processing: vignette · chromatic aberration · approximate bloom ────

struct Uniforms {
    vignette_strength: f32,      // 0=none, 1=heavy darkening
    vignette_radius:   f32,      // radius at which darkening starts (0~1)
    chroma_offset:     f32,      // chromatic aberration intensity
    bloom_threshold:   f32,      // luminance threshold above which bloom fires
    bloom_intensity:   f32,      // bloom brightness multiplier
    brightness:        f32,      // brightness offset (-1~1, 0=original)
    contrast:          f32,      // contrast multiplier (1=original)
    saturation:        f32,      // saturation multiplier (1=original, 0=grayscale)
    // Pre-computed from the render-target size by PostProcessRenderer::update_uniforms.
    // Avoids per-fragment textureDimensions() calls which emit a driver query on some
    // backends. Populated as (1/width, 1/height).
    texel_size:        vec2<f32>,
    // Explicit padding to match the Rust PostProcessUniforms layout (10 f32s → 48 B).
    pad0:              vec2<f32>,
}

@group(0) @binding(0) var scene_tex:     texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform>  u: Uniforms;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0)       uv:  vec2<f32>,
}

// Generate a fullscreen triangle without a vertex buffer
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VOut {
    // Three vertices: (-1,3), (-1,-1), (3,-1) — covers the entire NDC quad
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

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let uv = in.uv;

    // ── Chromatic Aberration ───────────────────────────────────────────────
    // Sample each RGB channel at a radially offset position, offset increasing with distance from center
    let center = uv - 0.5;
    let dist   = length(center);
    let dir    = select(normalize(center), vec2<f32>(1.0, 0.0), dist < 0.0001);
    let shift  = dir * dist * u.chroma_offset;

    let r_samp = textureSample(scene_tex, scene_sampler, uv + shift);
    let g_samp = textureSample(scene_tex, scene_sampler, uv);
    let b_samp = textureSample(scene_tex, scene_sampler, uv - shift);
    var color  = vec4<f32>(r_samp.r, g_samp.g, b_samp.b, g_samp.a);

    // ── Approximate bloom (4-tap threshold blur) ───────────────────────────
    // Sample neighbouring pixels and accumulate only luminance above the threshold.
    // texel_size is pre-computed by the CPU (1/width, 1/height) to avoid
    // per-fragment textureDimensions() queries.
    let texel = u.texel_size;
    let spread = 4.0;
    var bloom = vec3<f32>(0.0);
    // `var` (not `let`): a dynamically-indexed array needs an address space.
    // naga rejects dynamic indexing of a `let` array ("may only be indexed by a constant").
    var tap_offsets = array<vec2<f32>, 4>(
        vec2<f32>( texel.x,  0.0) * spread,
        vec2<f32>(-texel.x,  0.0) * spread,
        vec2<f32>( 0.0,  texel.y) * spread,
        vec2<f32>( 0.0, -texel.y) * spread,
    );
    for (var i = 0; i < 4; i++) {
        let s = textureSample(scene_tex, scene_sampler, uv + tap_offsets[i]).rgb;
        let lum = luminance(s);
        let w   = max(0.0, lum - u.bloom_threshold) / max(0.001, 1.0 - u.bloom_threshold);
        bloom  += s * w;
    }
    color = vec4<f32>(color.rgb + bloom * u.bloom_intensity * 0.25, color.a);

    // ── Vignette ───────────────────────────────────────────────────────────
    let vig_dist = length(center) / max(u.vignette_radius, 0.001);
    let vignette = 1.0 - smoothstep(1.0 - u.vignette_strength, 1.0, vig_dist);
    color = vec4<f32>(color.rgb * vignette, color.a);

    // ── Color Grading ──────────────────────────────────────────────────────
    var graded = color.rgb;
    // Brightness
    graded = graded + vec3<f32>(u.brightness);
    // Contrast (centered at 0.5)
    graded = (graded - 0.5) * u.contrast + 0.5;
    // Saturation (luminance preserved)
    let lum = dot(graded, vec3<f32>(0.2126, 0.7152, 0.0722));
    graded = mix(vec3<f32>(lum), graded, u.saturation);
    color = vec4<f32>(clamp(graded, vec3<f32>(0.0), vec3<f32>(1.0)), color.a);

    return color;
}
