struct KeyUniforms {
    rect: vec4<f32>,
    border_color: vec4<f32>,
    bg_color: vec4<f32>,
    screen_state: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: KeyUniforms;

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

fn outside_bevel(local: vec2<f32>, bevel: f32) -> bool {
    return local.x + local.y < bevel ||
        (1.0 - local.x) + local.y < bevel ||
        local.x + (1.0 - local.y) < bevel ||
        (1.0 - local.x) + (1.0 - local.y) < bevel;
}

@fragment
fn fs_keyboard(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let local = clamp(
        (pos.xy - uniforms.rect.xy) / max(uniforms.rect.zw, vec2(1.0, 1.0)),
        vec2(0.0, 0.0),
        vec2(1.0, 1.0),
    );
    let outer_bevel = 0.18;
    let inner_bevel = 0.24;

    if outside_bevel(local, outer_bevel) {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }

    let hovered = uniforms.screen_state.z;
    let pressed = uniforms.screen_state.w;
    let bg = uniforms.bg_color + vec4(0.0, hovered * 0.08, hovered * 0.1, 0.0);

    if !outside_bevel(local, inner_bevel) {
        return bg - vec4(pressed * 0.06, pressed * 0.04, 0.0, 0.0);
    }

    return uniforms.border_color + vec4(hovered * 0.05, hovered * 0.08, hovered * 0.08, 0.0);
}
