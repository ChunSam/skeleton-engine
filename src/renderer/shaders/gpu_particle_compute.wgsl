// GPU particle compute shader — particle lifetime + position update

struct Particle {
    pos:        vec2<f32>,  //  0: position
    vel:        vec2<f32>,  //  8: velocity
    life:       f32,        // 16: current lifetime (0 = inactive)
    max_life:   f32,        // 20: maximum lifetime
    size:       f32,        // 24: size (pixels)
    _pad:       f32,        // 28: padding
    color_start: vec4<f32>, // 32: start color
    color_end:   vec4<f32>, // 48: end color
    gravity:    vec2<f32>,  // 64: per-particle constant acceleration
    _pad2:      vec2<f32>,  // 72: padding to 80
}                           // 80 bytes total

struct ComputeUniforms {
    dt:    f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform>             uniforms:  ComputeUniforms;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if i >= arrayLength(&particles) { return; }
    var p = particles[i];
    if p.life <= 0.0 { return; }
    p.life -= uniforms.dt;
    p.vel  += p.gravity * uniforms.dt; // integrate gravity (no-op when zero)
    p.pos  += p.vel * uniforms.dt;
    particles[i] = p;
}
