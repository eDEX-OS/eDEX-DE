//! eDEX-DE wgpu GPU renderer.
//!
//! Renders the eDEX shell as a Wayland layer-shell client using wgpu.

pub mod shaders;
pub mod text;
pub mod ui;
pub mod wayland_client;

pub use ui::EdexRenderer;
