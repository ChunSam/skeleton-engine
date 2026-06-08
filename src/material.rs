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
/// world.add_component(e, ShaderMaterial {
///     frag_source: include_str!("shaders/dissolve.wgsl").to_string(),
///     params: [total_time, progress, 0.0, 0.0],
/// });
/// ```
///
/// `params` can be updated every frame inside a system via `world.get_mut::<ShaderMaterial>(e)`.
pub struct ShaderMaterial {
    /// WGSL fragment shader source (must expose an `fs_main` entry point).
    pub frag_source: String,
    /// Four float parameters passed to the shader.
    /// Received on the WGSL side as `@group(2) @binding(0) var<uniform> params: vec4<f32>`.
    /// Convention: `[time, intensity, user_x, user_y]`
    pub params: [f32; 4],
}
