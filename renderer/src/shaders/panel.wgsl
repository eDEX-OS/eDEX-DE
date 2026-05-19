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
fn fs_panel(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let bg = vec4(0.04, 0.055, 0.1, 1.0);
    let border_color = vec4(0.0, 0.83, 1.0, 1.0);
    let border_w = 0.003;

    let is_border = uv.x < border_w || uv.x > (1.0 - border_w) ||
                    uv.y < border_w || uv.y > (1.0 - border_w);

    if is_border {
        return border_color;
    }

    return bg;
}
