//! eDEX UI rendering primitives and the Phase 4 GPU renderer.

pub mod boot;
pub mod borders;
pub mod colors;
pub mod filesystem;
pub mod keyboard;
pub mod panels;
pub mod resize;
pub mod state;
pub mod statusbar;
pub mod theme;
pub mod topbar;

use std::{ptr::NonNull, time::Duration};

use anyhow::{anyhow, Context, Result};
use bytemuck::{Pod, Zeroable};
use glyphon::{Color, FontSystem, SwashCache, TextAtlas, TextBounds, TextRenderer, Viewport};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use wayland_client::{protocol::wl_surface, Connection, Proxy};
use wgpu::util::DeviceExt;

use crate::{shaders, text::TextSystem};

pub use boot::{BootAnimation, BootPhase};
pub use colors::{Theme, AMBER, MATRIX, TRON};
pub use filesystem::FilesystemPanel;
pub use keyboard::{HexKeyboard, KeyDef};
pub use panels::{PanelLayout, Rectangle};
pub use resize::{DragTarget, ResizeState};
pub use state::{FsEntry, StatusInfo, UiState};
pub use theme::{builtin_theme, parse_color, ThemeConfig, AMBER_TOML, MATRIX_TOML, TRON_TOML};

pub struct EdexRenderer {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub viewport: Viewport,
    pub atlas: TextAtlas,
    pub text_renderer: TextRenderer,
    pub width: u32,
    pub height: u32,
    panel_pipeline: wgpu::RenderPipeline,
    scanline_pipeline: wgpu::RenderPipeline,
    keyboard_pipeline: wgpu::RenderPipeline,
    boot_pipeline: wgpu::RenderPipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    keyboard: HexKeyboard,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PanelUniforms {
    rect: [f32; 4],
    border_color: [f32; 4],
    bg_color: [f32; 4],
    screen_meta: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct KeyboardUniforms {
    rect: [f32; 4],
    border_color: [f32; 4],
    bg_color: [f32; 4],
    screen_state: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OverlayUniforms {
    color: [f32; 4],
}

struct TextSpec {
    text: String,
    width: f32,
    height: f32,
    font_size: f32,
    line_height: f32,
    left: f32,
    top: f32,
    bounds: TextBounds,
    color: Color,
}

impl EdexRenderer {
    pub fn new(display: &Connection, surface: &wl_surface::WlSurface) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("failed to create renderer async runtime")?;

        runtime.block_on(Self::new_async(display, surface))
    }

    async fn new_async(display: &Connection, surface: &wl_surface::WlSurface) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let raw_display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
            NonNull::new(display.backend().display_ptr().cast())
                .context("Wayland display pointer was null")?,
        ));
        let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(
            NonNull::new(surface.id().as_ptr().cast())
                .context("Wayland surface pointer was null")?,
        ));

        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(raw_display_handle),
                raw_window_handle,
            })
        }
        .context("failed to create wgpu Wayland surface")?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .context("failed to find a Vulkan adapter for the eDEX surface")?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("eDEX-DE renderer device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .context("failed to create renderer device")?;

        let width = 1920;
        let height = 1080;
        let surface_config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| anyhow!("surface does not expose a supported configuration"))?;
        surface.configure(&device, &surface_config);

        let TextSystem {
            font_system,
            swash_cache,
            atlas,
            viewport,
            renderer: text_renderer,
        } = TextSystem::new(&device, &queue, surface_config.format);

        let uniform_bind_group_layout = create_uniform_bind_group_layout(&device);
        let panel_pipeline = create_pipeline(
            &device,
            &surface_config,
            &shaders::panel(&device),
            "fs_panel",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            &[Some(&uniform_bind_group_layout)],
        );
        let scanline_pipeline = create_pipeline(
            &device,
            &surface_config,
            &shaders::scanline(&device),
            "fs_scanline",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            &[],
        );
        let keyboard_pipeline = create_pipeline(
            &device,
            &surface_config,
            &shaders::keyboard(&device),
            "fs_keyboard",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            &[Some(&uniform_bind_group_layout)],
        );
        let boot_pipeline = create_pipeline(
            &device,
            &surface_config,
            &shaders::boot(&device),
            "fs_overlay",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            &[Some(&uniform_bind_group_layout)],
        );

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            surface_config,
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            width,
            height,
            panel_pipeline,
            scanline_pipeline,
            keyboard_pipeline,
            boot_pipeline,
            uniform_bind_group_layout,
            keyboard: HexKeyboard::new(),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.width = width;
        self.height = height;
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn frame_interval() -> Duration {
        Duration::from_millis(16)
    }

    pub fn render(&mut self, state: &UiState) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        let layout = PanelLayout::from_state(self.width, self.height, state.resize);
        self.viewport.update(
            &self.queue,
            glyphon::Resolution {
                width: self.width,
                height: self.height,
            },
        );

        let text_specs = self.build_text_specs(state, layout);
        let text_buffers = self.build_text_buffers(&text_specs);
        let text_areas = self.build_text_areas(&text_buffers, &text_specs);
        self.text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas.iter().cloned(),
                &mut self.swash_cache,
            )
            .context("failed to prepare text rendering")?;

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.surface_config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(anyhow!("wgpu surface returned a validation error"));
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("eDEX-DE frame encoder"),
            });

        let mut uniform_buffers = Vec::with_capacity(96);
        let mut uniform_bind_groups = Vec::with_capacity(96);

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("eDEX-DE render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(theme_color(state.theme.background)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.panel_pipeline);
            for rect in [
                layout.filesystem,
                layout.terminal,
                layout.sysinfo,
                layout.top_bar,
                layout.status_bar,
                layout.keyboard,
            ] {
                let uniforms = PanelUniforms {
                    rect: rect_to_uniform(rect),
                    border_color: state.theme.border,
                    bg_color: state.theme.panel_bg,
                    screen_meta: [
                        self.width as f32,
                        self.height as f32,
                        state.border_anim,
                        1.0,
                    ],
                };
                draw_uniform_rect(
                    &self.device,
                    &mut render_pass,
                    &self.uniform_bind_group_layout,
                    rect,
                    &uniforms,
                    &mut uniform_buffers,
                    &mut uniform_bind_groups,
                );
            }

            for (rect, intensity) in [
                (
                    layout.fs_terminal_handle,
                    state.resize.handle_color_intensity(DragTarget::FsTerminal),
                ),
                (
                    layout.terminal_sysinfo_handle,
                    state
                        .resize
                        .handle_color_intensity(DragTarget::TerminalSysinfo),
                ),
            ] {
                let handle_uniforms = PanelUniforms {
                    rect: rect_to_uniform(rect),
                    border_color: state.theme.border,
                    bg_color: [
                        state.theme.border_glow[0],
                        state.theme.border_glow[1],
                        state.theme.border_glow[2],
                        0.2 + 0.25 * intensity,
                    ],
                    screen_meta: [
                        self.width as f32,
                        self.height as f32,
                        state.border_anim,
                        1.1,
                    ],
                };
                draw_uniform_rect(
                    &self.device,
                    &mut render_pass,
                    &self.uniform_bind_group_layout,
                    rect,
                    &handle_uniforms,
                    &mut uniform_buffers,
                    &mut uniform_bind_groups,
                );

                let grab_rect = ResizeState::grab_indicator(rect);
                let grab_uniforms = PanelUniforms {
                    rect: rect_to_uniform(grab_rect),
                    border_color: state.theme.accent,
                    bg_color: [
                        state.theme.accent[0],
                        state.theme.accent[1],
                        state.theme.accent[2],
                        0.18,
                    ],
                    screen_meta: [
                        self.width as f32,
                        self.height as f32,
                        state.border_anim + 0.5,
                        1.0,
                    ],
                };
                draw_uniform_rect(
                    &self.device,
                    &mut render_pass,
                    &self.uniform_bind_group_layout,
                    grab_rect,
                    &grab_uniforms,
                    &mut uniform_buffers,
                    &mut uniform_bind_groups,
                );
            }

            render_pass.set_pipeline(&self.keyboard_pipeline);
            for (row_idx, row) in self.keyboard.rows.iter().enumerate() {
                for (col_idx, _) in row.iter().enumerate() {
                    let rect = self.keyboard.key_rect(&layout.keyboard, row_idx, col_idx);
                    let hover = if self.keyboard.hover_key == Some((row_idx, col_idx)) {
                        1.0
                    } else {
                        0.0
                    };
                    let pressed = if self.keyboard.pressed_key == Some((row_idx, col_idx)) {
                        1.0
                    } else {
                        0.0
                    };
                    let uniforms = KeyboardUniforms {
                        rect: rect_to_uniform(rect),
                        border_color: state.theme.border,
                        bg_color: [
                            state.theme.panel_bg[0] * 0.9,
                            state.theme.panel_bg[1] * 0.9,
                            state.theme.panel_bg[2] * 1.1,
                            1.0,
                        ],
                        screen_state: [self.width as f32, self.height as f32, hover, pressed],
                    };
                    draw_uniform_rect(
                        &self.device,
                        &mut render_pass,
                        &self.uniform_bind_group_layout,
                        rect,
                        &uniforms,
                        &mut uniform_buffers,
                        &mut uniform_bind_groups,
                    );
                }
            }

            render_pass.set_pipeline(&self.scanline_pipeline);
            render_pass.set_scissor_rect(0, 0, self.width, self.height);
            render_pass.draw(0..4, 0..1);

            if state.boot_overlay_alpha > 0.0 {
                render_pass.set_pipeline(&self.boot_pipeline);
                let overlay_uniforms = OverlayUniforms {
                    color: [0.0, 0.0, 0.0, state.boot_overlay_alpha],
                };
                draw_uniform_rect(
                    &self.device,
                    &mut render_pass,
                    &self.uniform_bind_group_layout,
                    Rectangle::new(0, 0, self.width, self.height),
                    &overlay_uniforms,
                    &mut uniform_buffers,
                    &mut uniform_bind_groups,
                );
            }

            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut render_pass)
                .context("failed to render text")?;
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        drop(text_buffers);
        Ok(())
    }

    fn build_text_specs(&self, state: &UiState, layout: PanelLayout) -> Vec<TextSpec> {
        let top_bar = topbar::compose(state);
        let status_bar = statusbar::compose(&state.status);
        let terminal_text = if state.terminal_content.is_empty() {
            "$ renderer online\n$ awaiting shell output".to_string()
        } else {
            state.terminal_content.join("\n")
        };
        let filesystem_text = format!(
            "{}\n{}",
            state.filesystem_cwd,
            state
                .filesystem_entries
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    let prefix = if index == state.selected_fs_entry {
                        ">"
                    } else {
                        " "
                    };
                    format!("{} {}", prefix, entry.name)
                })
                .collect::<Vec<_>>()
                .join("\n")
        );
        let sysinfo_text = format!(
            "HOST       {}\nGPU        Vulkan / wgpu\nTABS       {}/{}\nBORDER FX  {:.2}\nAUDIO      {}%\nNETWORK    ▲ {:.1} / ▼ {:.1} kb/s",
            state.hostname,
            state.active_tab.saturating_add(1),
            state.tab_count.max(1),
            state.border_anim,
            state.status.volume,
            state.status.net_tx_kbps,
            state.status.net_rx_kbps,
        );

        let mut specs = vec![
            TextSpec {
                text: top_bar.left,
                width: (layout.top_bar.width / 3) as f32,
                height: layout.top_bar.height as f32,
                font_size: 18.0,
                line_height: 24.0,
                left: 16.0,
                top: (layout.top_bar.y + 9) as f32,
                bounds: bounds_for(layout.top_bar),
                color: to_glyphon_color(state.theme.text_primary),
            },
            TextSpec {
                text: top_bar.center,
                width: 220.0,
                height: layout.top_bar.height as f32,
                font_size: 28.0,
                line_height: 32.0,
                left: (self.width as f32 * 0.5) - 74.0,
                top: (layout.top_bar.y + 4) as f32,
                bounds: bounds_for(layout.top_bar),
                color: to_glyphon_color(state.theme.border),
            },
            TextSpec {
                text: top_bar.right,
                width: (layout.top_bar.width / 3) as f32,
                height: layout.top_bar.height as f32,
                font_size: 18.0,
                line_height: 24.0,
                left: self.width as f32 - 320.0,
                top: (layout.top_bar.y + 9) as f32,
                bounds: bounds_for(layout.top_bar),
                color: to_glyphon_color(state.theme.text_secondary),
            },
            TextSpec {
                text: status_bar,
                width: layout.status_bar.width as f32 - 32.0,
                height: layout.status_bar.height as f32,
                font_size: 14.0,
                line_height: 18.0,
                left: 16.0,
                top: (layout.status_bar.y + 6) as f32,
                bounds: bounds_for(layout.status_bar),
                color: to_glyphon_color(state.theme.text_secondary),
            },
            TextSpec {
                text: terminal_text,
                width: layout.terminal.width as f32 - 32.0,
                height: layout.terminal.height as f32 - 32.0,
                font_size: 18.0,
                line_height: 24.0,
                left: (layout.terminal.x + 16) as f32,
                top: (layout.terminal.y + 16) as f32,
                bounds: bounds_for(layout.terminal),
                color: to_glyphon_color(state.theme.text_primary),
            },
            TextSpec {
                text: filesystem_text,
                width: layout.filesystem.width as f32 - 32.0,
                height: layout.filesystem.height as f32 - 32.0,
                font_size: 16.0,
                line_height: 22.0,
                left: (layout.filesystem.x + 16) as f32,
                top: (layout.filesystem.y + 16) as f32,
                bounds: bounds_for(layout.filesystem),
                color: to_glyphon_color(state.theme.text_secondary),
            },
            TextSpec {
                text: sysinfo_text,
                width: layout.sysinfo.width as f32 - 32.0,
                height: layout.sysinfo.height as f32 - 32.0,
                font_size: 16.0,
                line_height: 22.0,
                left: (layout.sysinfo.x + 16) as f32,
                top: (layout.sysinfo.y + 16) as f32,
                bounds: bounds_for(layout.sysinfo),
                color: to_glyphon_color(state.theme.text_secondary),
            },
        ];

        for (row_idx, row) in self.keyboard.rows.iter().enumerate() {
            for (col_idx, key) in row.iter().enumerate() {
                let rect = self.keyboard.key_rect(&layout.keyboard, row_idx, col_idx);
                let font_size = if key.label.len() > 5 { 10.0 } else { 12.0 };
                let approx_width = key.label.chars().count() as f32 * font_size * 0.32;
                specs.push(TextSpec {
                    text: key.label.to_string(),
                    width: rect.width as f32,
                    height: rect.height as f32,
                    font_size,
                    line_height: rect.height as f32,
                    left: rect.x as f32 + rect.width as f32 * 0.5 - approx_width,
                    top: rect.y as f32 + rect.height as f32 * 0.22,
                    bounds: bounds_for(rect),
                    color: to_glyphon_color(state.theme.text_primary),
                });
            }
        }

        if !state.boot_done && !state.boot_lines.is_empty() {
            specs.push(TextSpec {
                text: state.boot_lines.join("\n"),
                width: self.width as f32 - 120.0,
                height: self.height as f32 - 120.0,
                font_size: 18.0,
                line_height: 24.0,
                left: 48.0,
                top: 72.0,
                bounds: TextBounds {
                    left: 32,
                    top: 32,
                    right: self.width as i32 - 32,
                    bottom: self.height as i32 - 32,
                },
                color: to_glyphon_color(state.theme.text_primary),
            });
        }

        specs
    }

    fn build_text_buffers(&mut self, specs: &[TextSpec]) -> Vec<glyphon::Buffer> {
        specs
            .iter()
            .map(|spec| {
                TextSystem::make_buffer(
                    &mut self.font_system,
                    spec.width,
                    spec.height,
                    &spec.text,
                    spec.font_size,
                    spec.line_height,
                )
            })
            .collect()
    }

    fn build_text_areas<'a>(
        &self,
        buffers: &'a [glyphon::Buffer],
        specs: &[TextSpec],
    ) -> Vec<glyphon::TextArea<'a>> {
        buffers
            .iter()
            .zip(specs.iter())
            .map(|(buffer, spec)| {
                TextSystem::default_area(buffer, spec.left, spec.top, spec.bounds, spec.color)
            })
            .collect()
    }
}

fn create_uniform_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("eDEX-DE uniform bind group layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

fn create_pipeline(
    device: &wgpu::Device,
    surface_config: &wgpu::SurfaceConfiguration,
    shader: &wgpu::ShaderModule,
    fragment_entry: &str,
    blend: Option<wgpu::BlendState>,
    bind_group_layouts: &[Option<&wgpu::BindGroupLayout>],
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("eDEX-DE pipeline layout"),
        bind_group_layouts,
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("eDEX-DE pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment_entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn draw_uniform_rect<T: Pod>(
    device: &wgpu::Device,
    render_pass: &mut wgpu::RenderPass<'_>,
    bind_group_layout: &wgpu::BindGroupLayout,
    rect: Rectangle,
    uniforms: &T,
    uniform_buffers: &mut Vec<wgpu::Buffer>,
    uniform_bind_groups: &mut Vec<wgpu::BindGroup>,
) {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("eDEX-DE uniform buffer"),
        contents: bytemuck::bytes_of(uniforms),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("eDEX-DE uniform bind group"),
        layout: bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });

    uniform_buffers.push(buffer);
    uniform_bind_groups.push(bind_group);
    render_pass.set_bind_group(
        0,
        uniform_bind_groups.last().expect("bind group exists"),
        &[],
    );
    render_pass.set_scissor_rect(rect.x, rect.y, rect.width.max(1), rect.height.max(1));
    render_pass.draw(0..4, 0..1);
}

fn rect_to_uniform(rect: Rectangle) -> [f32; 4] {
    [
        rect.x as f32,
        rect.y as f32,
        rect.width.max(1) as f32,
        rect.height.max(1) as f32,
    ]
}

fn theme_color(value: [f32; 4]) -> wgpu::Color {
    wgpu::Color {
        r: value[0] as f64,
        g: value[1] as f64,
        b: value[2] as f64,
        a: value[3] as f64,
    }
}

fn to_glyphon_color(color: [f32; 4]) -> Color {
    Color::rgba(
        to_color_channel(color[0]),
        to_color_channel(color[1]),
        to_color_channel(color[2]),
        to_color_channel(color[3]),
    )
}

fn to_color_channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn bounds_for(rect: Rectangle) -> TextBounds {
    TextBounds {
        left: rect.x as i32,
        top: rect.y as i32,
        right: rect.x.saturating_add(rect.width) as i32,
        bottom: rect.y.saturating_add(rect.height) as i32,
    }
}
