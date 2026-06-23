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
    exposure:          f32,       // scene colour multiply applied before tone-mapping (1=none)
    tonemap:           u32,       // operator: 0=None, 1=Reinhard, 2=ACES filmic
    bloom_enabled:     u32,       // 1 = real multi-pass bloom ran → skip the inline 4-tap below
    // Three separate u32 (NOT vec3<u32>, which has 16-byte alignment and would inflate the struct
    // to 80B) so this matches the Rust `_pad1: [u32; 3]` 64-byte layout exactly.
    _pad1:             u32,
    _pad2:             u32,
    _pad3:             u32,
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

// Reinhard tone-map: c / (1 + c), per channel.
fn tonemap_reinhard(c: vec3<f32>) -> vec3<f32> {
    return c / (1.0 + c);
}

// ACES filmic approximation (Narkowicz 2015) — filmic highlight roll-off.
fn tonemap_aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Dispatch on the operator id. None (0) and any unknown value pass the colour through unchanged.
fn apply_tonemap(c: vec3<f32>, op: u32) -> vec3<f32> {
    if op == 1u {
        return tonemap_reinhard(c);
    } else if op == 2u {
        return tonemap_aces(c);
    }
    return c;
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

    // ── Exposure ───────────────────────────────────────────────────────────
    // Applied before bloom/tone-mapping. 1.0 (default) is an exact no-op.
    color = vec4<f32>(color.rgb * u.exposure, color.a);

    // ── Approximate inline bloom (4-tap threshold blur) ────────────────────
    // Sample neighbouring pixels and accumulate only luminance above the threshold.
    // texel_size is pre-computed by the CPU (1/width, 1/height) to avoid
    // per-fragment textureDimensions() queries.
    // Skipped when the real multi-pass bloom pass already composited the glow onto the scene
    // (bloom_enabled == 1) — otherwise both would apply and the glow would double up.
    if u.bloom_enabled == 0u {
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
            let s = textureSample(scene_tex, scene_sampler, uv + tap_offsets[i]).rgb * u.exposure;
            let lum = luminance(s);
            let w   = max(0.0, lum - u.bloom_threshold) / max(0.001, 1.0 - u.bloom_threshold);
            bloom  += s * w;
        }
        color = vec4<f32>(color.rgb + bloom * u.bloom_intensity * 0.25, color.a);
    }

    // ── Vignette ───────────────────────────────────────────────────────────
    let vig_dist = length(center) / max(u.vignette_radius, 0.001);
    let vignette = 1.0 - smoothstep(1.0 - u.vignette_strength, 1.0, vig_dist);
    color = vec4<f32>(color.rgb * vignette, color.a);

    // ── Tone-mapping (HDR → display range) ──────────────────────────────────
    // Maps the (possibly > 1.0) scene colour into [0, 1]. None (0) is a no-op, so the
    // non-HDR path is unchanged.
    color = vec4<f32>(apply_tonemap(color.rgb, u.tonemap), color.a);

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
