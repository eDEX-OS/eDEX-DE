//! eDEX UI rendering primitives and the Phase 2 GPU renderer.

pub mod colors;
pub mod panels;
pub mod state;

use std::{ptr::NonNull, time::Duration};

use anyhow::{anyhow, Context, Result};
use glyphon::{Color, FontSystem, SwashCache, TextAtlas, TextBounds, TextRenderer, Viewport};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use wayland_client::{protocol::wl_surface, Connection, Proxy};

use crate::{shaders, text::TextSystem};

pub use colors::{Theme, TRON};
pub use panels::{PanelLayout, Rectangle};
pub use state::{FsEntry, UiState};

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

        let panel_pipeline = create_pipeline(
            &device,
            &surface_config,
            &shaders::panel(&device),
            Some("fs_panel"),
            Some(wgpu::BlendState::REPLACE),
        );
        let scanline_pipeline = create_pipeline(
            &device,
            &surface_config,
            &shaders::scanline(&device),
            Some("fs_scanline"),
            Some(wgpu::BlendState::ALPHA_BLENDING),
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

        let layout = PanelLayout::from_size(self.width, self.height);
        self.viewport.update(
            &self.queue,
            glyphon::Resolution {
                width: self.width,
                height: self.height,
            },
        );

        let text_buffers = self.build_text_buffers(state, layout);
        let text_areas = self.build_text_areas(&text_buffers, state, layout);
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
                draw_rect(&mut render_pass, rect);
            }

            render_pass.set_pipeline(&self.scanline_pipeline);
            render_pass.set_scissor_rect(0, 0, self.width, self.height);
            render_pass.draw(0..4, 0..1);

            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut render_pass)
                .context("failed to render text")?;
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        drop(text_buffers);
        Ok(())
    }

    fn build_text_buffers(&mut self, state: &UiState, layout: PanelLayout) -> Vec<glyphon::Buffer> {
        let top_bar_text = format!(" {} ", state.clock);
        let title = "eDEX-DE";
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
                .map(|entry| {
                    if entry.is_dir {
                        format!("[dir] {}", entry.name)
                    } else {
                        entry.name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        );
        let sysinfo_text = format!(
            "HOST   {}\nGPU    Vulkan / wgpu\nPANELS filesystem | terminal | sysinfo\nFPS    target 60",
            state.hostname
        );
        let keyboard_text = "HEX KEYBOARD // PHASE 2 VISUAL FOUNDATION";

        let clock_buffer = TextSystem::make_buffer(
            &mut self.font_system,
            220.0,
            layout.top_bar.height as f32,
            &top_bar_text,
            22.0,
            28.0,
        );
        let title_buffer = TextSystem::make_buffer(
            &mut self.font_system,
            240.0,
            layout.top_bar.height as f32,
            title,
            24.0,
            30.0,
        );
        let terminal_buffer = TextSystem::make_buffer(
            &mut self.font_system,
            layout.terminal.width as f32 - 32.0,
            layout.terminal.height as f32 - 32.0,
            &terminal_text,
            18.0,
            24.0,
        );
        let filesystem_buffer = TextSystem::make_buffer(
            &mut self.font_system,
            layout.filesystem.width as f32 - 32.0,
            layout.filesystem.height as f32 - 32.0,
            &filesystem_text,
            16.0,
            22.0,
        );
        let sysinfo_buffer = TextSystem::make_buffer(
            &mut self.font_system,
            layout.sysinfo.width as f32 - 32.0,
            layout.sysinfo.height as f32 - 32.0,
            &sysinfo_text,
            16.0,
            22.0,
        );
        let keyboard_buffer = TextSystem::make_buffer(
            &mut self.font_system,
            layout.keyboard.width as f32 - 32.0,
            layout.keyboard.height as f32 - 32.0,
            keyboard_text,
            18.0,
            24.0,
        );

        let buffers = vec![
            clock_buffer,
            title_buffer,
            terminal_buffer,
            filesystem_buffer,
            sysinfo_buffer,
            keyboard_buffer,
        ];

        buffers
    }

    fn build_text_areas<'a>(
        &self,
        buffers: &'a [glyphon::Buffer],
        state: &UiState,
        layout: PanelLayout,
    ) -> Vec<glyphon::TextArea<'a>> {
        vec![
            TextSystem::default_area(
                &buffers[0],
                20.0,
                (layout.top_bar.y + 8) as f32,
                bounds_for(layout.top_bar),
                to_glyphon_color(state.theme.text_primary),
            ),
            TextSystem::default_area(
                &buffers[1],
                (self.width as f32 * 0.5) - 60.0,
                (layout.top_bar.y + 6) as f32,
                bounds_for(layout.top_bar),
                to_glyphon_color(state.theme.border),
            ),
            TextSystem::default_area(
                &buffers[2],
                (layout.terminal.x + 16) as f32,
                (layout.terminal.y + 16) as f32,
                bounds_for(layout.terminal),
                to_glyphon_color(state.theme.text_primary),
            ),
            TextSystem::default_area(
                &buffers[3],
                (layout.filesystem.x + 16) as f32,
                (layout.filesystem.y + 16) as f32,
                bounds_for(layout.filesystem),
                to_glyphon_color(state.theme.text_secondary),
            ),
            TextSystem::default_area(
                &buffers[4],
                (layout.sysinfo.x + 16) as f32,
                (layout.sysinfo.y + 16) as f32,
                bounds_for(layout.sysinfo),
                to_glyphon_color(state.theme.text_secondary),
            ),
            TextSystem::default_area(
                &buffers[5],
                (layout.keyboard.x + 16) as f32,
                (layout.keyboard.y + 16) as f32,
                bounds_for(layout.keyboard),
                to_glyphon_color(state.theme.accent),
            ),
        ]
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    surface_config: &wgpu::SurfaceConfiguration,
    shader: &wgpu::ShaderModule,
    fragment_entry: Option<&str>,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("eDEX-DE pipeline layout"),
        bind_group_layouts: &[],
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
            entry_point: fragment_entry,
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

fn draw_rect(render_pass: &mut wgpu::RenderPass<'_>, rect: Rectangle) {
    render_pass.set_scissor_rect(rect.x, rect.y, rect.width.max(1), rect.height.max(1));
    render_pass.draw(0..4, 0..1);
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
