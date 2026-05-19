//! Embedded WGSL shaders used by the renderer.

pub fn panel(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::include_wgsl!("panel.wgsl"))
}

pub fn scanline(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::include_wgsl!("scanline.wgsl"))
}

pub fn boot(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::include_wgsl!("boot.wgsl"))
}

pub fn keyboard(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::include_wgsl!("keyboard.wgsl"))
}
