//! eDEX-DE wgpu GPU renderer.
//!
//! Renders all eDEX UI panels using wgpu (Vulkan backend) and WGSL shaders.
//! Integrates with smithay via DMA-BUF texture import.

pub mod shaders;
pub mod text;
pub mod ui;

pub use ui::EdexRenderer;
