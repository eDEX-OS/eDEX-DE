struct PanelUniforms {
    rect: vec4<f32>,
    border_color: vec4<f32>,
    bg_color: vec4<f32>,
    screen_meta: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: PanelUniforms;

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
fn fs_panel(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let local = clamp(
        (pos.xy - uniforms.rect.xy) / max(uniforms.rect.zw, vec2(1.0, 1.0)),
        vec2(0.0, 0.0),
        vec2(1.0, 1.0),
    );
    let border_w = 0.02;
    let border = local.x < border_w || local.x > (1.0 - border_w) ||
        local.y < border_w || local.y > (1.0 - border_w);

    if border {
        let pulse = 0.7 + 0.3 * sin(uniforms.screen_meta.z * 3.14159);
        return vec4(
            uniforms.border_color.rgb,
            uniforms.border_color.a * (0.8 + pulse * uniforms.screen_meta.w * 0.3),
        );
    }

    return uniforms.bg_color;
}
