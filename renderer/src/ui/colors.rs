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
    background: [0.039, 0.055, 0.102, 1.0],
    panel_bg: [0.051, 0.067, 0.09, 1.0],
    border: [0.0, 0.831, 1.0, 1.0],
    border_glow: [0.0, 0.831, 1.0, 0.3],
    text_primary: [0.784, 0.902, 1.0, 1.0],
    text_secondary: [0.478, 0.71, 0.816, 1.0],
    text_dim: [0.478, 0.71, 0.816, 0.5],
    accent: [0.0, 1.0, 0.533, 1.0],
    warning: [1.0, 0.624, 0.0, 1.0],
    error: [1.0, 0.267, 0.267, 1.0],
};

pub const MATRIX: Theme = Theme {
    background: [0.0, 0.039, 0.0, 1.0],
    panel_bg: [0.0, 0.082, 0.0, 1.0],
    border: [0.0, 1.0, 0.255, 1.0],
    border_glow: [0.0, 1.0, 0.255, 0.3],
    text_primary: [0.0, 1.0, 0.255, 1.0],
    text_secondary: [0.0, 0.667, 0.165, 1.0],
    text_dim: [0.0, 0.667, 0.165, 0.5],
    accent: [0.502, 1.0, 0.502, 1.0],
    warning: [1.0, 0.624, 0.0, 1.0],
    error: [1.0, 0.267, 0.267, 1.0],
};

pub const AMBER: Theme = Theme {
    background: [0.039, 0.02, 0.0, 1.0],
    panel_bg: [0.071, 0.039, 0.0, 1.0],
    border: [1.0, 0.6, 0.0, 1.0],
    border_glow: [1.0, 0.6, 0.0, 0.3],
    text_primary: [1.0, 0.8, 0.4, 1.0],
    text_secondary: [0.8, 0.533, 0.0, 1.0],
    text_dim: [0.8, 0.533, 0.0, 0.5],
    accent: [1.0, 0.933, 0.0, 1.0],
    warning: [1.0, 0.624, 0.0, 1.0],
    error: [1.0, 0.267, 0.267, 1.0],
};
