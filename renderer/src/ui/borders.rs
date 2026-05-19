#[derive(Clone, Copy, Debug)]
pub struct BorderAnimState {
    pub phase: f32,
    pub glow_intensity: f32,
}

impl Default for BorderAnimState {
    fn default() -> Self {
        Self {
            phase: 0.0,
            glow_intensity: 1.0,
        }
    }
}
