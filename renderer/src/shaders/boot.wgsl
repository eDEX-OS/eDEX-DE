struct OverlayUniforms {
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: OverlayUniforms;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 4>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0),
        vec2(-1.0,  1.0), vec2(1.0,  1.0),
    );

    var out: VertexOutput;
    out.pos = vec4(positions[vi], 0.0, 1.0);
    return out;
}

@fragment
fn fs_overlay() -> @location(0) vec4<f32> {
    return uniforms.color;
}
