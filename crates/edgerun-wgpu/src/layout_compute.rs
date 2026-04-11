//! GPU Layout Compute Pipeline — Full CSS cascade + block layout
//!
//! Pipeline:
//!   Pass 0: Cascade resolution (GPU, fully parallel)
//!   Pass 1: Style inheritance (GPU, fully parallel)
//!   Pass 2: Height computation (GPU, fully parallel — reads children's heights)
//!   Y position: Sequential block layout (CPU — reads heights, writes Y positions)
//!
//! This design maximizes GPU parallelism while handling the inherently
//! sequential Y positioning on the CPU.

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, Device, PipelineLayoutDescriptor,
    ShaderSource,
};

use crate::uniforms::{GpuDomNode, GpuCssRule, GpuStyleResult, GpuLayoutResult};

const COMPUTE_SRC: &str = include_str!("../../../shaders/layout.wgsl");

/// Config for the layout compute shader.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LayoutConfig {
    pub node_count: u32,
    pub rule_count: u32,
    pub text_len: u32,
    pub compute_phase: u32,  // 0=cascade, 1=inherit, 2=heights
    pub avail_width: f32,
    pub base_x: f32,
    pub base_y: f32,
    pub _pad1: u32,
}

impl LayoutConfig {
    pub const SIZE: usize = std::mem::size_of::<Self>();
}

/// Holds the GPU resources for layout computation.
pub struct LayoutComputePipeline {
    pub pipeline: ComputePipeline,
    pub bind_group_layout: BindGroupLayout,
    pub node_buffer: Buffer,
    pub rule_buffer: Buffer,
    pub text_buffer: Buffer,
    pub config_buffer: Buffer,
    pub style_results: Buffer,  // GPU cascade output
    pub layout_results: Buffer, // GPU layout output
}

impl LayoutComputePipeline {
    /// Create the layout compute pipeline and buffers.
    pub fn new(device: &Device, max_nodes: u32, max_rules: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("edgerun-layout-compute"),
            source: ShaderSource::Wgsl(COMPUTE_SRC.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("layout-compute-layout"),
            entries: &[
                // 0: nodes (read)
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 1: rules (read)
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 2: text_buffer (read)
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 3: config (uniform)
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 4: style_results (read_write)
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 5: layout_results (read_write)
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("layout-compute-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("edgerun-layout-pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let node_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("layout-node-buffer"),
            size: (max_nodes as u64) * (GpuDomNode::SIZE as u64),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let rule_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("layout-rule-buffer"),
            size: (max_rules as u64) * (GpuCssRule::SIZE as u64),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let text_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("layout-text-buffer"),
            size: 65536,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let config_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("layout-config-buffer"),
            size: LayoutConfig::SIZE as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let style_results = device.create_buffer(&BufferDescriptor {
            label: Some("layout-style-buffer"),
            size: (max_nodes as u64) * (GpuStyleResult::SIZE as u64),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let layout_results = device.create_buffer(&BufferDescriptor {
            label: Some("layout-result-buffer"),
            size: (max_nodes as u64) * (GpuLayoutResult::SIZE as u64),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group_layout,
            node_buffer,
            rule_buffer,
            text_buffer,
            config_buffer,
            style_results,
            layout_results,
        }
    }

    /// Create bind group from current buffer state.
    pub fn create_bind_group(&self, device: &Device) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("layout-compute-bind-group"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.node_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: self.rule_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.text_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: self.config_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: self.style_results.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: self.layout_results.as_entire_binding(),
                },
            ],
        })
    }

    /// Run a single compute pass with the given pass type.
    fn run_pass(
        &self,
        device: &Device,
        queue: &wgpu::Queue,
        nodes: &[GpuDomNode],
        rules: &[GpuCssRule],
        text_data: &[u8],
        config: &LayoutConfig,
    ) {
        // Upload input data
        queue.write_buffer(&self.node_buffer, 0, bytemuck::cast_slice(nodes));
        queue.write_buffer(&self.rule_buffer, 0, bytemuck::cast_slice(rules));
        if !text_data.is_empty() {
            let padded_len = (text_data.len() + 3) & !3;
            let mut padded = text_data.to_vec();
            padded.resize(padded_len, 0);
            queue.write_buffer(&self.text_buffer, 0, bytemuck::cast_slice(&padded));
        }
        queue.write_buffer(&self.config_buffer, 0, bytemuck::bytes_of(config));

        let bind_group = self.create_bind_group(device);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("layout-compute-encoder"),
        });

        let workgroups = (config.node_count + 63) / 64;
        {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("layout-compute-pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(workgroups.max(1), 1, 1);
        }

        queue.submit(Some(encoder.finish()));
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
    }

    /// Run the full GPU cascade + height computation, then do CPU-side Y positioning.
    ///
    /// Returns layout results with x, y, w, h fully populated.
    pub fn run_full_layout(
        &self,
        device: &Device,
        queue: &wgpu::Queue,
        nodes: &[GpuDomNode],
        rules: &[GpuCssRule],
        text_data: &[u8],
        avail_width: f32,
        base_y: f32,
    ) -> Vec<GpuLayoutResult> {
        let node_count = nodes.len() as u32;

        // Pass 0: Cascade resolution
        let config_cascade = LayoutConfig {
            node_count,
            rule_count: rules.len() as u32,
            text_len: text_data.len() as u32,
            compute_phase: 0,
            avail_width,
            base_x: 0.0,
            base_y,
            _pad1: 0,
        };
        self.run_pass(device, queue, nodes, rules, text_data, &config_cascade);

        // Pass 1: Style inheritance
        let config_inherit = LayoutConfig {
            compute_phase: 1,
            ..config_cascade
        };
        self.run_pass(device, queue, nodes, rules, text_data, &config_inherit);

        // Pass 2: Height computation
        // Run twice — first pass computes leaf heights, second sums children
        for _ in 0..2 {
            let config_heights = LayoutConfig {
                compute_phase: 2,
                ..config_cascade
            };
            self.run_pass(device, queue, nodes, rules, text_data, &config_heights);
        }

        // Read back heights
        let heights = self.read_back_results(device, queue, node_count);

        // CPU-side: sequential Y positioning (block layout)
        let mut results = heights;
        block_layout_y(&mut results, nodes, 0, 0.0, base_y, avail_width);

        results
    }

    /// Read back layout results from GPU.
    fn read_back_results(
        &self,
        device: &Device,
        queue: &wgpu::Queue,
        node_count: u32,
    ) -> Vec<GpuLayoutResult> {
        let result_size = (node_count as u64) * (GpuLayoutResult::SIZE as u64);
        let staging = device.create_buffer(&BufferDescriptor {
            label: Some("layout-staging"),
            size: result_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("layout-readback"),
        });
        encoder.copy_buffer_to_buffer(&self.layout_results, 0, &staging, 0, result_size);
        queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().unwrap().unwrap();

        let data = slice.get_mapped_range();
        let results = bytemuck::pod_collect_to_vec::<u8, GpuLayoutResult>(&data);
        drop(data);
        staging.unmap();

        results
    }
}

/// Sequential block layout: compute Y positions from heights.
fn block_layout_y(
    results: &mut [GpuLayoutResult],
    nodes: &[GpuDomNode],
    node_idx: u32,
    cursor_y: f32,
    _parent_content_y: f32,
    avail_width: f32,
) -> f32 {
    let idx = node_idx as usize;
    if idx >= results.len() { return cursor_y; }

    let parent_idx = nodes[idx].parent_idx;
    let first_child = nodes[idx].first_child_idx;
    let padding_top = results[idx].padding_top;
    let padding_bottom = results[idx].padding_bottom;
    let margin_bottom = results[idx].margin_bottom;
    let existing_h = results[idx].h;

    let is_root = parent_idx == u32::MAX;

    // Compute position values
    let (new_y, new_content_y, new_content_h) = if is_root {
        (0.0, padding_top, 0.0)
    } else {
        let cy = cursor_y + padding_top;
        let ch = (existing_h - padding_top - padding_bottom).max(0.0);
        (cursor_y, cy, ch)
    };

    // Apply position
    results[idx].x = 0.0;
    results[idx].y = new_y;
    results[idx].w = avail_width;
    results[idx].content_x = 0.0;
    results[idx].content_y = new_content_y;
    results[idx].content_w = avail_width;
    results[idx].content_h = new_content_h;

    if is_root {
        results[idx].h = 0.0;
    }

    // Layout children
    let content_y = results[idx].content_y;
    let mut child_y = content_y;
    let mut ci = first_child;
    while ci != u32::MAX {
        let ci_idx = ci as usize;
        child_y = block_layout_y(results, nodes, ci, child_y, content_y, avail_width);
        let child_mb = results[ci_idx].margin_bottom;
        child_y += child_mb;
        ci = nodes[ci_idx].next_sibling_idx;
    }

    if is_root {
        let pb = results[idx].padding_bottom;
        results[idx].h = child_y + pb;
        return results[idx].h;
    }

    cursor_y + existing_h + margin_bottom
}
