/// Attaching this to an entity replaces the built-in sprite shader with a custom fragment shader.
///
/// Vertex input is identical to the standard sprite, so only the fragment shader needs to be written.
///
/// ## Available bindings in the custom shader
///
/// ```wgsl
/// @group(1) @binding(0) var t_sprite: texture_2d<f32>;
/// @group(1) @binding(1) var s_sprite: sampler;
/// @group(2) @binding(0) var<uniform> params: vec4<f32>;  // ShaderMaterial::params
///
/// struct VertexOutput {
///     @builtin(position) clip_pos: vec4<f32>,
///     @location(0) uv:    vec2<f32>,
///     @location(1) color: vec4<f32>,
/// };
/// @fragment
/// fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { ... }
/// ```
///
/// ## Usage example
///
/// ```text
/// world.add_component(e, ShaderMaterial::new(
///     include_str!("shaders/dissolve.wgsl"),
///     [total_time, progress, 0.0, 0.0],
/// ));
/// ```
///
/// `params` can be updated every frame inside a system via `world.get_mut::<ShaderMaterial>(e)`.
pub struct ShaderMaterial {
    /// WGSL fragment shader source (must expose an `fs_main` entry point).
    frag_source: String,
    /// Cached hash of `frag_source`, computed once at construction (or on [`set_frag_source`]).
    /// Used by the renderer as the pipeline cache key — avoids re-hashing the source every frame.
    ///
    /// [`set_frag_source`]: ShaderMaterial::set_frag_source
    source_hash: u64,
    /// Four float parameters passed to the shader.
    /// Received on the WGSL side as `@group(2) @binding(0) var<uniform> params: vec4<f32>`.
    /// Convention: `[time, intensity, user_x, user_y]`
    pub params: [f32; 4],
}

impl ShaderMaterial {
    /// Create a new `ShaderMaterial`, hashing `frag_source` once at construction.
    ///
    /// The hash is stored in the struct and reused by the renderer every frame instead of
    /// re-hashing the source string. Call [`set_frag_source`] to replace the source and
    /// automatically recompute the hash.
    ///
    /// [`set_frag_source`]: ShaderMaterial::set_frag_source
    pub fn new(frag_source: impl Into<String>, params: [f32; 4]) -> Self {
        let frag_source = frag_source.into();
        let source_hash = Self::hash_source(&frag_source);
        Self {
            frag_source,
            source_hash,
            params,
        }
    }

    /// Returns the WGSL fragment shader source.
    pub fn frag_source(&self) -> &str {
        &self.frag_source
    }

    /// Returns the cached hash of the fragment shader source.
    ///
    /// This is computed once at construction (or when [`set_frag_source`] is called) and
    /// used by the renderer as the pipeline cache key.
    ///
    /// [`set_frag_source`]: ShaderMaterial::set_frag_source
    pub fn source_hash(&self) -> u64 {
        self.source_hash
    }

    /// Replace the fragment shader source and recompute the cached hash.
    ///
    /// The renderer detects the new hash on the next frame and compiles a new pipeline.
    pub fn set_frag_source(&mut self, src: impl Into<String>) {
        self.frag_source = src.into();
        self.source_hash = Self::hash_source(&self.frag_source);
    }

    fn hash_source(src: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        src.hash(&mut h);
        h.finish()
    }
}
