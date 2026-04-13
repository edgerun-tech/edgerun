//! High-level render: given a slice of `GpuRectStyle`, render to a texture and
//! read back as RGBA8 pixels.

use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, Device, Extent3d,
    MapMode, Origin3d, Queue, Texture, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TexelCopyTextureInfo, TexelCopyBufferInfo,
    TexelCopyBufferLayout, PollType, Sampler, SamplerDescriptor,
};

use crate::pipeline::{create_bind_group, create_pipeline, render_pass, RenderPipelineState};
use crate::uniforms::{GpuRectStyle, GpuUniforms, GpuRectBuffer, GpuTextBuffer, GpuTextCommand};
use crate::font_atlas::{FontAtlas, GlyphEntry};

/// Glyph info entry for GPU — matches WGSL GlyphInfo struct.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuGlyphInfo {
    pub atlas_x: f32,
    pub atlas_y: f32,
    pub width: f32,
    pub height: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance: f32,
    pub _pad: f32,
}

impl GpuGlyphInfo {
    pub const SIZE: usize = 32;

    pub fn zeroed() -> Self {
        Self {
            atlas_x: 0.0, atlas_y: 0.0,
            width: 0.0, height: 0.0,
            bearing_x: 0.0, bearing_y: 0.0,
            advance: 0.0, _pad: 0.0,
        }
    }
}

/// A complete GPU renderer that holds the pipeline and temporary resources.
pub struct GpuRenderer {
    pub device: Device,
    pub queue: Queue,
    pub pipeline_state: RenderPipelineState,
    pub uniform_buffer: Buffer,
    pub rect_storage: Buffer,
    pub text_storage: Buffer,
    pub glyph_info_buffer: Buffer,
    pub font_atlas: FontAtlas,
    pub font_sampler: Sampler,
    pub readback_buffer: Buffer,
    pub texture: Texture,
    pub texture_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl GpuRenderer {
    /// Create a new renderer for the given framebuffer dimensions.
    pub fn new(
        device: Device,
        queue: Queue,
        width: u32,
        height: u32,
        font_data: &[u8],
        font_size: f32,
        text: &str,
    ) -> Self {
        let pipeline_state = create_pipeline(&device);

        // Build font atlas from TTF data
        let font_atlas = FontAtlas::new(&device, font_data, font_size, text);
        font_atlas.upload(&queue);

        // Create a linear sampler for the font atlas
        let font_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("font-atlas-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        // Build glyph info buffer
        let mut glyph_entries: Vec<GpuGlyphInfo> = vec![GpuGlyphInfo::zeroed(); 4096];
        for (ch, entry) in &font_atlas.glyphs {
            let idx = *ch as usize;
            if idx < 4096 {
                glyph_entries[idx] = GpuGlyphInfo {
                    atlas_x: entry.atlas_x as f32,
                    atlas_y: entry.atlas_y as f32,
                    width: entry.width as f32,
                    height: entry.height as f32,
                    bearing_x: entry.bearing_x,
                    bearing_y: entry.bearing_y,
                    advance: entry.advance,
                    _pad: 0.0,
                };
            }
        }
        let glyph_info_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("glyph-info-buffer"),
            size: (glyph_entries.len() * GpuGlyphInfo::SIZE) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&glyph_info_buffer, 0, bytemuck::cast_slice(&glyph_entries));

        // Uniform buffer (small: just frame info)
        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("uniform-buffer"),
            size: GpuUniforms::SIZE as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Rect storage buffer: sized to hold max expected rects
        let max_rects = 1024u64;
        let max_rect_storage_size = max_rects * GpuRectStyle::SIZE as u64;
        let rect_storage = device.create_buffer(&BufferDescriptor {
            label: Some("rect-storage-buffer"),
            size: max_rect_storage_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Text storage buffer: sized to hold max expected text commands
        let max_text_cmds = 1024u64;
        let max_text_storage_size = max_text_cmds * GpuTextCommand::SIZE as u64;
        let text_storage = device.create_buffer(&BufferDescriptor {
            label: Some("text-storage-buffer"),
            size: max_text_storage_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Render texture (RGBA8)
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("render-texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Readback buffer (for reading texture back to CPU)
        let bytes_per_row = align_to(width * 4, 256);
        let readback_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("readback-buffer"),
            size: (bytes_per_row * height) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            pipeline_state,
            uniform_buffer,
            rect_storage,
            text_storage,
            glyph_info_buffer,
            font_atlas,
            font_sampler,
            readback_buffer,
            texture,
            texture_view,
            width,
            height,
        }
    }

    /// Render a list of styled rectangles and text commands to the texture and read back as RGBA8 bytes.
    pub fn render_and_readback(&mut self, rects: &[GpuRectStyle], text_cmds: &[GpuTextCommand]) -> Vec<u8> {
        // Build rect buffer
        let rect_buf = GpuRectBuffer::from_rects(rects);

        // Build text buffer
        let text_buf = GpuTextBuffer::from_commands(text_cmds);

        // Build uniforms
        let uniforms = GpuUniforms::new(self.width, self.height, rect_buf.rect_count, text_buf.text_cmd_count);

        // Upload uniforms
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Upload rect storage
        self.queue
            .write_buffer(&self.rect_storage, 0, &rect_buf.buffer_data);

        // Upload text storage
        self.queue
            .write_buffer(&self.text_storage, 0, &text_buf.buffer_data);

        // Create bind group with explicit buffer sizes + font atlas
        let bind_group = create_bind_group_with_atlas(
            &self.device,
            &self.pipeline_state.bind_group_layout,
            &self.uniform_buffer,
            &self.rect_storage,
            rect_buf.buffer_data.len() as u64,
            &self.text_storage,
            text_buf.buffer_data.len() as u64,
            &self.font_atlas.view,
            &self.font_sampler,
            &self.glyph_info_buffer,
        );

        // Encode render commands
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render-encoder"),
            });

        render_pass(
            &mut encoder,
            &self.texture_view,
            &self.pipeline_state.pipeline,
            &bind_group,
            self.width,
            self.height,
        );

        // Copy texture to readback buffer
        let bytes_per_row = align_to(self.width * 4, 256);
        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &self.readback_buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        // Submit
        self.queue.submit(Some(encoder.finish()));

        // Read back
        let readback_slice = self.readback_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        readback_slice.map_async(MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        let _ = self.device.poll(PollType::wait_indefinitely());
        rx.recv().unwrap().unwrap();

        let data = readback_slice.get_mapped_range();
        let bytes_per_row = align_to(self.width * 4, 256);

        // Crop out row padding
        let mut result = Vec::with_capacity((self.width * self.height * 4) as usize);
        for row in 0..self.height {
            let row_start = (row * bytes_per_row) as usize;
            let row_end = row_start + (self.width * 4) as usize;
            result.extend_from_slice(&data[row_start..row_end]);
        }
        drop(data);
        self.readback_buffer.unmap();

        result
    }
}

/// Align `value` up to `alignment` (must be a power of 2).
fn align_to(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}

/// Create bind group with font atlas texture and glyph info buffer.
fn create_bind_group_with_atlas(
    device: &Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buf: &Buffer,
    rect_buf: &Buffer,
    rect_buf_size: u64,
    text_buf: &Buffer,
    text_buf_size: u64,
    atlas_view: &wgpu::TextureView,
    sampler: &Sampler,
    glyph_info_buf: &Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("render-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: rect_buf,
                    offset: 0,
                    size: std::num::NonZeroU64::new(rect_buf_size),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: text_buf,
                    offset: 0,
                    size: std::num::NonZeroU64::new(text_buf_size),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(atlas_view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: glyph_info_buf.as_entire_binding(),
            },
        ],
    })
}

/// Convenience function: create a headless renderer, render, and return pixels.
///
/// This is the simplest entry point for the demo.
pub fn render_to_pixels(
    width: u32,
    height: u32,
    rects: &[GpuRectStyle],
    text_cmds: &[GpuTextCommand],
    font_data: &[u8],
    font_size: f32,
    text: &str,
) -> Vec<u8> {
    // Initialize WGPU
    let instance = wgpu::Instance::default();
    let rt = edgerun_rt::Builder::new_multi_thread().build().unwrap();
    let adapter = rt.block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        force_fallback_adapter: false,
        compatible_surface: None,
    }))
    .expect("Failed to find an appropriate GPU adapter");

    let (device, queue) = rt.block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("edgerun-wgpu-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        },
    ))
    .expect("Failed to create WGPU device");

    // Create renderer and render
    let mut renderer = GpuRenderer::new(device, queue, width, height, font_data, font_size, text);
    renderer.render_and_readback(rects, text_cmds)
}
