//! WGPU pipeline creation — loads the generated WGSL shader.

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, ColorTargetState,
    ColorWrites, Device, FragmentState, LoadOp, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    ShaderSource, StoreOp, TextureFormat, Operations,
};

/// The generated WGSL shader source.
const SHADER_SRC: &str = include_str!("../../../shaders/render.wgsl");

/// Holds the WGPU pipeline state for the painter's algorithm renderer.
pub struct RenderPipelineState {
    pub pipeline: RenderPipeline,
    pub bind_group_layout: BindGroupLayout,
}

/// Create the WGPU render pipeline from the generated WGSL shader.
pub fn create_pipeline(device: &Device) -> RenderPipelineState {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("edgerun-generated-wgsl"),
        source: ShaderSource::Wgsl(SHADER_SRC.into()),
    });

    // Bind group layout: one uniform buffer + two storage buffers (rects + text)
    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("uniforms-layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("render-pipeline-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("edgerun-painter-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[], // No vertex buffers — we generate geometry in the vertex shader
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(ColorTargetState {
                format: TextureFormat::Rgba8Unorm,
                blend: None, // We do our own blending in the shader
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    });

    RenderPipelineState {
        pipeline,
        bind_group_layout,
    }
}

/// Create a bind group for the uniform buffer and storage buffers.
pub fn create_bind_group(
    device: &Device,
    layout: &BindGroupLayout,
    uniform_buffer: &Buffer,
    rect_storage: &Buffer,
    rect_storage_size: u64,
    text_storage: &Buffer,
    text_storage_size: u64,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: Some("uniforms-bind-group"),
        layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: rect_storage,
                    offset: 0,
                    size: std::num::NonZeroU64::new(rect_storage_size),
                }),
            },
            BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: text_storage,
                    offset: 0,
                    size: std::num::NonZeroU64::new(text_storage_size),
                }),
            },
        ],
    })
}

/// Run a render pass with the given pipeline and bind group.
pub fn render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    texture_view: &'a wgpu::TextureView,
    pipeline: &'a RenderPipeline,
    bind_group: &'a BindGroup,
    _width: u32,
    _height: u32,
) {
    let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("edgerun-render-pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: texture_view,
            resolve_target: None,
            depth_slice: None,
            ops: Operations {
                load: LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    rpass.set_pipeline(pipeline);
    rpass.set_bind_group(0, bind_group, &[]);
    rpass.draw(0..3, 0..1); // Full-screen triangle (3 vertices)
}
