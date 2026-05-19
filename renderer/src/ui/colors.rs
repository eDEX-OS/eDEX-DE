#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub background: [f32; 4],
    pub panel_bg: [f32; 4],
    pub border: [f32; 4],
    pub border_glow: [f32; 4],
    pub text_primary: [f32; 4],
    pub text_secondary: [f32; 4],
    pub text_dim: [f32; 4],
    pub accent: [f32; 4],
    pub warning: [f32; 4],
    pub error: [f32; 4],
}

pub const TRON: Theme = Theme {
    background: [0.04, 0.055, 0.1, 1.0],
    panel_bg: [0.051, 0.067, 0.09, 1.0],
    border: [0.0, 0.831, 1.0, 1.0],
    border_glow: [0.0, 0.831, 1.0, 0.3],
    text_primary: [0.784, 0.902, 1.0, 1.0],
    text_secondary: [0.478, 0.710, 0.816, 1.0],
    text_dim: [0.227, 0.353, 0.478, 1.0],
    accent: [0.0, 1.0, 0.533, 1.0],
    warning: [1.0, 0.624, 0.0, 1.0],
    error: [1.0, 0.267, 0.267, 1.0],
};
