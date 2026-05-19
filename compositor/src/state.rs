//! Compositor state — will hold smithay State struct in Phase 1.

pub struct CompositorState {
    pub version: &'static str,
}

impl CompositorState {
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}

impl Default for CompositorState {
    fn default() -> Self {
        Self::new()
    }
}
