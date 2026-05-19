//! Embedded WGSL shaders used by the Phase 2 renderer.

pub fn panel(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::include_wgsl!("panel.wgsl"))
}

pub fn scanline(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::include_wgsl!("scanline.wgsl"))
}
