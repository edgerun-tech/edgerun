//! High-level render: given a slice of `GpuRectStyle`, render to a texture and
//! read back as RGBA8 pixels.

use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, Device, Extent3d,
    MapMode, Origin3d, Queue, Texture, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TexelCopyTextureInfo, TexelCopyBufferInfo,
    TexelCopyBufferLayout, PollType,
};

use crate::pipeline::{create_bind_group, create_pipeline, render_pass, RenderPipelineState};
use crate::uniforms::{GpuRectStyle, GpuUniforms, GpuRectBuffer, GpuTextBuffer, GpuTextCommand};

/// A complete GPU renderer that holds the pipeline and temporary resources.
pub struct GpuRenderer {
    pub device: Device,
    pub queue: Queue,
    pub pipeline_state: RenderPipelineState,
    pub uniform_buffer: Buffer,
    pub rect_storage: Buffer,
    pub text_storage: Buffer,
    pub readback_buffer: Buffer,
    pub texture: Texture,
    pub texture_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl GpuRenderer {
    /// Create a new renderer for the given framebuffer dimensions.
    pub fn new(device: Device, queue: Queue, width: u32, height: u32) -> Self {
        let pipeline_state = create_pipeline(&device);

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

        // Create bind group with explicit buffer sizes
        let bind_group = create_bind_group(
            &self.device,
            &self.pipeline_state.bind_group_layout,
            &self.uniform_buffer,
            &self.rect_storage,
            rect_buf.buffer_data.len() as u64,
            &self.text_storage,
            text_buf.buffer_data.len() as u64,
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

/// Convenience function: create a headless renderer, render, and return pixels.
///
/// This is the simplest entry point for the demo.
pub fn render_to_pixels(
    width: u32,
    height: u32,
    rects: &[GpuRectStyle],
    text_cmds: &[GpuTextCommand],
) -> Vec<u8> {
    // Initialize WGPU
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        force_fallback_adapter: false,
        compatible_surface: None,
    }))
    .expect("Failed to find an appropriate GPU adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
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
    let mut renderer = GpuRenderer::new(device, queue, width, height);
    renderer.render_and_readback(rects, text_cmds)
}
