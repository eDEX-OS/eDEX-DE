struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 4>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0),
        vec2(-1.0,  1.0), vec2(1.0,  1.0),
    );
    var uvs = array<vec2<f32>, 4>(
        vec2(0.0, 1.0), vec2(1.0, 1.0),
        vec2(0.0, 0.0), vec2(1.0, 0.0),
    );

    var out: VertexOutput;
    out.pos = vec4(positions[vi], 0.0, 1.0);
    out.uv = uvs[vi];
    return out;
}

@fragment
fn fs_scanline(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let scan = step(0.5, fract(uv.y * 720.0));
    let vignette = 1.0 - distance(uv, vec2(0.5, 0.5)) * 0.35;
    let alpha = (0.04 + (scan * 0.035)) * clamp(vignette, 0.65, 1.0);
    return vec4(0.0, 0.83, 1.0, alpha * 0.18);
}
