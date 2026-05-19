use glyphon::{
    Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport,
};

pub struct TextSystem {
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub atlas: TextAtlas,
    pub viewport: Viewport,
    pub renderer: TextRenderer,
}

impl TextSystem {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, surface_format);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        Self {
            font_system,
            swash_cache,
            atlas,
            viewport,
            renderer,
        }
    }

    pub fn make_buffer(
        font_system: &mut FontSystem,
        width: f32,
        height: f32,
        text: &str,
        font_size: f32,
        line_height: f32,
    ) -> Buffer {
        let mut buffer = Buffer::new(font_system, Metrics::new(font_size, line_height));
        buffer.set_size(font_system, Some(width.max(1.0)), Some(height.max(1.0)));
        let attrs = glyphon::Attrs::new().family(Family::Monospace);
        buffer.set_text(font_system, text, &attrs, Shaping::Advanced, None);
        buffer
    }

    pub fn default_area<'a>(
        buffer: &'a Buffer,
        left: f32,
        top: f32,
        bounds: TextBounds,
        color: Color,
    ) -> TextArea<'a> {
        TextArea {
            buffer,
            left,
            top,
            scale: 1.0,
            bounds,
            default_color: color,
            custom_glyphs: &[],
        }
    }

    pub fn update_viewport(&mut self, queue: &wgpu::Queue, width: u32, height: u32) {
        self.viewport.update(queue, Resolution { width, height });
    }
}
