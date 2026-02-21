use std::collections::HashMap;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};

use pixels::wgpu::{self, util::DeviceExt};

use crate::render::GlyphCache;
use crate::terminal::{Rgba, SELECTION, Terminal, brightened, ensure_contrast, faintened};

#[derive(Clone, Copy)]
struct AtlasEntry {
    uv: [f32; 4],   // u0, v0, u1, v1
    size: [u32; 2], // width, height
    bearing: [i32; 2],
    color: bool,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RectVertex {
    pos: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlyphVertex {
    rect: [f32; 4],    // x0, y0, x1, y1
    uv_rect: [f32; 4], // u0, v0, u1, v1
    color: [f32; 4],
    flags: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BlitVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct QuadVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

pub struct GlyphAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    size: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    entries: HashMap<char, AtlasEntry>,
    cleared: bool,
    usage: HashMap<char, u64>,
}

impl GlyphAtlas {
    fn new(device: &wgpu::Device, size: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("term_glyph_atlas"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Use linear filtering for MSDF; atlas padding avoids sampling bleed.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("term_glyph_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        Self {
            texture,
            view,
            sampler,
            size,
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            entries: HashMap::new(),
            cleared: false,
            usage: HashMap::new(),
        }
    }

    fn get_or_insert(
        &mut self,
        ch: char,
        glyphs: &mut GlyphCache,
        queue: &wgpu::Queue,
    ) -> Option<AtlasEntry> {
        let mut cleared_once = false;
        if let Some(entry) = self.entries.get(&ch).copied() {
            *self.usage.entry(ch).or_insert(0) += 1;
            return Some(entry);
        }

        loop {
            let (metrics, bitmap, is_color) = glyphs.rasterize(ch);
            if metrics.width == 0 || metrics.height == 0 {
                return None;
            }

            let w = metrics.width as u32;
            let h = metrics.height as u32;
            let pad = 1u32;
            let w_padded = w.saturating_add(pad * 2);
            let h_padded = h.saturating_add(pad * 2);
            let space_needed = w_padded.max(1);
            if space_needed > self.size || h_padded > self.size {
                return None;
            }

            if self.cursor_x + w_padded >= self.size {
                self.cursor_x = 0;
                self.cursor_y = self.cursor_y.saturating_add(self.row_height + 1);
                self.row_height = 0;
            }
            if self.cursor_y + h_padded >= self.size {
                // Atlas full – clear once and retry.
                if !cleared_once {
                    cleared_once = true;
                    self.clear();
                } else {
                    return None;
                }
                continue;
            }

            if self.cursor_x + w_padded >= self.size {
                self.cursor_x = 0;
                self.cursor_y = self.cursor_y.saturating_add(self.row_height + 1);
                self.row_height = 0;
            }

            if self.cursor_y + h_padded >= self.size {
                if cleared_once {
                    return None;
                }
                self.clear();
                cleared_once = true;
                continue;
            }

            let x = self.cursor_x;
            let y = self.cursor_y;
            self.cursor_x += w_padded + 1;
            self.row_height = self.row_height.max(h_padded);

            let stride = ((w_padded + 63) / 64) * 64; // keep aligned for wgpu row padding
            let mut padded = vec![0u8; (stride * h_padded * 4) as usize];
            for row in 0..h as usize {
                let src_start = row * w as usize * 4;
                let dst_start = (row + pad as usize) * stride as usize * 4 + pad as usize * 4;
                padded[dst_start..dst_start + w as usize * 4]
                    .copy_from_slice(&bitmap[src_start..src_start + w as usize * 4]);
            }

            for row in 0..h as usize {
                let src_row_start = (row + pad as usize) * stride as usize * 4;
                let left_px = src_row_start + pad as usize * 4;
                let right_px = left_px + (w.saturating_sub(1) as usize * 4);
                let dst_left = src_row_start;
                let dst_right = src_row_start + (pad as usize + w as usize) * 4;
                let left_pixel = [
                    padded[left_px],
                    padded[left_px + 1],
                    padded[left_px + 2],
                    padded[left_px + 3],
                ];
                let right_pixel = [
                    padded[right_px],
                    padded[right_px + 1],
                    padded[right_px + 2],
                    padded[right_px + 3],
                ];
                padded[dst_left..dst_left + 4].copy_from_slice(&left_pixel);
                padded[dst_right..dst_right + 4].copy_from_slice(&right_pixel);
            }

            let top_src = pad as usize * stride as usize * 4;
            let bottom_src = (pad as usize + h.saturating_sub(1) as usize) * stride as usize * 4;
            let top_row = padded[top_src..top_src + stride as usize * 4].to_vec();
            let bottom_row = padded[bottom_src..bottom_src + stride as usize * 4].to_vec();
            for _ in 0..pad {
                let dst_top = (pad as usize - 1) * stride as usize * 4;
                padded[dst_top..dst_top + stride as usize * 4].copy_from_slice(&top_row);
                let dst_bottom = (pad as usize + h as usize) * stride as usize * 4;
                padded[dst_bottom..dst_bottom + stride as usize * 4].copy_from_slice(&bottom_row);
            }

            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &self.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                &padded,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(stride * 4),
                    rows_per_image: Some(h_padded),
                },
                wgpu::Extent3d {
                    width: w_padded,
                    height: h_padded,
                    depth_or_array_layers: 1,
                },
            );

            let uv = [
                (x + pad) as f32 / self.size as f32,
                (y + pad) as f32 / self.size as f32,
                (x + pad + w) as f32 / self.size as f32,
                (y + pad + h) as f32 / self.size as f32,
            ];

            let entry = AtlasEntry {
                uv,
                size: [w, h],
                bearing: [metrics.xmin, metrics.ymin],
                color: is_color,
            };
            self.entries.insert(ch, entry);
            *self.usage.entry(ch).or_insert(0) += 1;
            return Some(entry);
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.row_height = 0;
        self.cleared = true;
        self.usage.clear();
    }

    fn take_cleared(&mut self) -> bool {
        let was_cleared = self.cleared;
        self.cleared = false;
        was_cleared
    }
}

pub struct GpuRenderer {
    rect_pipeline: wgpu::RenderPipeline,
    rect_add_pipeline: wgpu::RenderPipeline,
    glyph_pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    rect_vertex_buffer: wgpu::Buffer,
    glyph_vertex_buffer: wgpu::Buffer,
    rect_overlay_buffer: wgpu::Buffer,
    glyph_overlay_buffer: wgpu::Buffer,
    rect_cursor_buffer: wgpu::Buffer,
    glyph_cursor_buffer: wgpu::Buffer,
    blit_vertex_buffer: wgpu::Buffer,
    glyph_quad_buffer: wgpu::Buffer,
    rect_capacity: usize,
    glyph_capacity: usize,
    rect_overlay_capacity: usize,
    glyph_overlay_capacity: usize,
    rect_cursor_capacity: usize,
    glyph_cursor_capacity: usize,
    screen_uniform: wgpu::Buffer,
    screen_bind_group: wgpu::BindGroup,
    glyph_layout: wgpu::BindGroupLayout,
    glyph_bind_group: wgpu::BindGroup,
    atlas: GlyphAtlas,
    atlas_size: u32,
    max_atlas_size: u32,
    screen_size: [f32; 2],
    use_sdf_flag: bool,
    msdf_min_width: f32,
    base_texture: wgpu::Texture,
    base_view: wgpu::TextureView,
    base_bind_group: wgpu::BindGroup,
    base_sampler: wgpu::Sampler,
    staging_belt: wgpu::util::StagingBelt,
    base_size: (u32, u32),
    base_valid: bool,
    surface_format: wgpu::TextureFormat,
}

static ATLAS_INSERT_WARNED: AtomicBool = AtomicBool::new(false);

impl GpuRenderer {
    fn env_flag(name: &str, default: bool) -> bool {
        match env::var(name) {
            Ok(value) => {
                let v = value.to_ascii_lowercase();
                !(v == "0" || v == "false" || v == "off" || v == "no")
            }
            Err(_) => default,
        }
    }

    pub fn rainbow(phase: f32) -> [u8; 3] {
        let r = ((phase).sin() * 0.5 + 0.5) * 255.0;
        let g = ((phase + 2.094395_f32).sin() * 0.5 + 0.5) * 255.0;
        let b = ((phase + 4.18879_f32).sin() * 0.5 + 0.5) * 255.0;
        [r as u8, g as u8, b as u8]
    }

    pub fn new(context: &pixels::PixelsContext, surface_format: wgpu::TextureFormat) -> Self {
        let device = &context.device;
        // Default to a larger atlas to reduce eviction/clearing under heavy glyph variety.
        let allow_growth = Self::env_flag("TERM_ATLAS_GROW", true);
        let max_limit = device.limits().max_texture_dimension_2d.max(512).min(1024);
        let atlas_size = env::var("TERM_ATLAS_SIZE")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .map(|v| v.clamp(512, max_limit))
            .unwrap_or(8192.min(max_limit));
        let max_atlas_size = if allow_growth { max_limit } else { atlas_size };
        log::warn!(
            "term: atlas init size={} max={} allow_growth={} max_limit={} env_grow={:?}",
            atlas_size,
            max_atlas_size,
            allow_growth,
            max_limit,
            env::var("TERM_ATLAS_GROW").ok()
        );

        let screen_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("term_screen_uniform"),
            // size: vec2(size), vec2(flags) -> (width, height, use_sdf, msdf_min_width)
            contents: bytemuck::cast_slice(&[
                0.0f32,
                0.0f32,
                1.0f32,
                crate::text::MSDF_DEFAULT_MIN_WIDTH + crate::text::MSDF_AA_BOOST,
            ]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let screen_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("term_screen_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("term_screen_bind_group"),
            layout: &screen_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_uniform.as_entire_binding(),
            }],
        });

        let atlas = GlyphAtlas::new(device, atlas_size);
        // TODO: consider making atlas size configurable; a larger atlas helps avoid evictions for
        // icon-heavy prompts and keeps glyphs visible after scrollback-heavy frames.

        let glyph_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("term_glyph_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let glyph_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("term_glyph_bind_group"),
            layout: &glyph_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas.sampler),
                },
            ],
        });

        let rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("term_rect_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("rect.wgsl").into()),
        });
        let glyph_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("term_glyph_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("glyph.wgsl").into()),
        });
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("term_blit_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blit.wgsl").into()),
        });

        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("term_rect_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("term_rect_layout"),
                    bind_group_layouts: &[&screen_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RectVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                        },
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 8,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let rect_add_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("term_rect_add_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("term_rect_add_layout"),
                    bind_group_layouts: &[&screen_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RectVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                        },
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 8,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let glyph_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("term_glyph_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("term_glyph_layout"),
                    bind_group_layouts: &[&screen_layout, &glyph_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &glyph_shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 0,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 8,
                            },
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GlyphVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 0,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 3,
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 16,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 4,
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 32,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 5,
                                format: wgpu::VertexFormat::Float32,
                                offset: 48,
                            },
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &glyph_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let base_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("term_base_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let base_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("term_base_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let base_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("term_base_texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let base_view = base_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let base_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("term_base_bind_group"),
            layout: &base_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&base_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&base_sampler),
                },
            ],
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("term_blit_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("term_blit_layout"),
                    bind_group_layouts: &[&base_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<BlitVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let rect_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("term_rect_buffer"),
            size: 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let glyph_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("term_glyph_buffer"),
            size: 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let rect_overlay_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("term_rect_overlay_buffer"),
            size: 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let glyph_overlay_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("term_glyph_overlay_buffer"),
            size: 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let rect_cursor_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("term_rect_cursor_buffer"),
            size: 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let glyph_cursor_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("term_glyph_cursor_buffer"),
            size: 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let blit_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("term_blit_buffer"),
            contents: bytemuck::cast_slice(&[
                BlitVertex {
                    pos: [-1.0, -1.0],
                    uv: [0.0, 1.0],
                },
                BlitVertex {
                    pos: [1.0, -1.0],
                    uv: [1.0, 1.0],
                },
                BlitVertex {
                    pos: [-1.0, 1.0],
                    uv: [0.0, 0.0],
                },
                BlitVertex {
                    pos: [-1.0, 1.0],
                    uv: [0.0, 0.0],
                },
                BlitVertex {
                    pos: [1.0, -1.0],
                    uv: [1.0, 1.0],
                },
                BlitVertex {
                    pos: [1.0, 1.0],
                    uv: [1.0, 0.0],
                },
            ]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let glyph_quad = [
            QuadVertex {
                pos: [0.0, 0.0],
                uv: [0.0, 0.0],
            },
            QuadVertex {
                pos: [1.0, 0.0],
                uv: [1.0, 0.0],
            },
            QuadVertex {
                pos: [1.0, 1.0],
                uv: [1.0, 1.0],
            },
            QuadVertex {
                pos: [0.0, 0.0],
                uv: [0.0, 0.0],
            },
            QuadVertex {
                pos: [1.0, 1.0],
                uv: [1.0, 1.0],
            },
            QuadVertex {
                pos: [0.0, 1.0],
                uv: [0.0, 1.0],
            },
        ];
        let glyph_quad_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("term_glyph_quad_buffer"),
            contents: bytemuck::cast_slice(&glyph_quad),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            rect_pipeline,
            rect_add_pipeline,
            glyph_pipeline,
            blit_pipeline,
            rect_vertex_buffer,
            glyph_vertex_buffer,
            rect_overlay_buffer,
            glyph_overlay_buffer,
            rect_cursor_buffer,
            glyph_cursor_buffer,
            blit_vertex_buffer,
            glyph_quad_buffer,
            rect_capacity: 1024,
            glyph_capacity: 1024,
            rect_overlay_capacity: 1024,
            glyph_overlay_capacity: 1024,
            rect_cursor_capacity: 1024,
            glyph_cursor_capacity: 1024,
            screen_uniform,
            screen_bind_group,
            glyph_layout,
            glyph_bind_group,
            atlas,
            atlas_size,
            max_atlas_size,
            screen_size: [0.0, 0.0],
            use_sdf_flag: true,
            msdf_min_width: crate::text::MSDF_DEFAULT_MIN_WIDTH + crate::text::MSDF_AA_BOOST,
            base_texture,
            base_view,
            base_bind_group,
            base_sampler,
            staging_belt: wgpu::util::StagingBelt::new(1024 * 1024),
            base_size: (0, 0),
            base_valid: false,
            surface_format,
        }
    }

    pub fn base_valid(&self) -> bool {
        self.base_valid
    }

    pub fn invalidate_base(&mut self) {
        self.base_valid = false;
    }

    pub fn mark_base_valid(&mut self) {
        self.base_valid = true;
    }

    pub fn ensure_base_target(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.base_size == (width, height) && self.base_size != (0, 0) {
            return;
        }
        self.base_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("term_base_texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.base_view = self
            .base_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.base_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("term_base_bind_group"),
            layout: &self.blit_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.base_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.base_sampler),
                },
            ],
        });
        self.base_size = (width, height);
        self.base_valid = false;
    }

    pub fn base_view(&self) -> &wgpu::TextureView {
        &self.base_view
    }

    pub fn blit_base_to_target(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("term_blit_base"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.blit_pipeline);
        pass.set_bind_group(0, &self.base_bind_group, &[]);
        pass.set_vertex_buffer(0, self.blit_vertex_buffer.slice(..));
        pass.draw(0..6, 0..1);
    }

    pub fn render_overlay(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        context: &pixels::PixelsContext,
        rects: &[RectVertex],
        glyph_vertices: &[GlyphVertex],
    ) {
        if rects.is_empty() && glyph_vertices.is_empty() {
            return;
        }

        let device = &context.device;
        Self::ensure_buffer(
            device,
            &mut self.rect_overlay_buffer,
            &mut self.rect_overlay_capacity,
            rects.len() * std::mem::size_of::<RectVertex>(),
            "term_rect_overlay_buffer",
        );
        Self::ensure_buffer(
            device,
            &mut self.glyph_overlay_buffer,
            &mut self.glyph_overlay_capacity,
            glyph_vertices.len() * std::mem::size_of::<GlyphVertex>(),
            "term_glyph_overlay_buffer",
        );

        if !rects.is_empty() {
            let data: &[u8] = bytemuck::cast_slice(rects);
            context
                .queue
                .write_buffer(&self.rect_overlay_buffer, 0, data);
        }
        if !glyph_vertices.is_empty() {
            let data: &[u8] = bytemuck::cast_slice(glyph_vertices);
            context
                .queue
                .write_buffer(&self.glyph_overlay_buffer, 0, data);
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("term_gpu_overlay"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        if !rects.is_empty() {
            pass.set_pipeline(&self.rect_pipeline);
            pass.set_bind_group(0, &self.screen_bind_group, &[]);
            pass.set_vertex_buffer(0, self.rect_overlay_buffer.slice(..));
            pass.draw(0..rects.len() as u32, 0..1);
        }

        if !glyph_vertices.is_empty() {
            pass.set_pipeline(&self.glyph_pipeline);
            pass.set_bind_group(0, &self.screen_bind_group, &[]);
            pass.set_bind_group(1, &self.glyph_bind_group, &[]);
            pass.set_vertex_buffer(0, self.glyph_quad_buffer.slice(..));
            pass.set_vertex_buffer(1, self.glyph_overlay_buffer.slice(..));
            pass.draw(0..6, 0..glyph_vertices.len() as u32);
        }
    }

    pub fn build_overlay(
        &mut self,
        context: &pixels::PixelsContext,
        glyphs: &mut GlyphCache,
        mut build: impl FnMut(
            &mut Vec<RectVertex>,
            &mut Vec<GlyphVertex>,
            &mut GlyphAtlas,
            &wgpu::Queue,
            &mut GlyphCache,
        ),
    ) -> (Vec<RectVertex>, Vec<GlyphVertex>) {
        let mut rects = Vec::new();
        let mut glyphs_out = Vec::new();
        build(
            &mut rects,
            &mut glyphs_out,
            &mut self.atlas,
            &context.queue,
            glyphs,
        );
        (rects, glyphs_out)
    }

    fn rebuild_atlas(&mut self, device: &wgpu::Device, size: u32) {
        log::warn!(
            "term: atlas rebuild requested size={} current={} max={} limit={}",
            size,
            self.atlas_size,
            self.max_atlas_size,
            device.limits().max_texture_dimension_2d
        );
        self.atlas = GlyphAtlas::new(device, size);
        self.atlas_size = size;
        self.glyph_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("term_glyph_bind_group"),
            layout: &self.glyph_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.atlas.sampler),
                },
            ],
        });
    }

    pub fn clear_atlas(&mut self) {
        self.atlas.clear();
    }

    pub fn resize(&mut self, width: u32, height: u32, queue: &wgpu::Queue) {
        if width == 0 || height == 0 {
            return;
        }
        self.screen_size = [width as f32, height as f32];
        self.write_screen_uniform(queue);
    }

    pub fn set_use_sdf(&mut self, enabled: bool, queue: &wgpu::Queue) {
        if self.use_sdf_flag == enabled {
            return;
        }
        self.use_sdf_flag = enabled;
        self.write_screen_uniform(queue);
    }

    pub fn set_msdf_min_width(&mut self, min_width: f32, queue: &wgpu::Queue) {
        let min_width = min_width.max(0.001);
        if (self.msdf_min_width - min_width).abs() < f32::EPSILON {
            return;
        }
        self.msdf_min_width = min_width;
        self.write_screen_uniform(queue);
    }

    fn write_screen_uniform(&self, queue: &wgpu::Queue) {
        let vals = [
            self.screen_size[0],
            self.screen_size[1],
            if self.use_sdf_flag { 1.0 } else { 0.0 },
            self.msdf_min_width,
        ];
        queue.write_buffer(&self.screen_uniform, 0, bytemuck::cast_slice(&vals));
    }

    fn ensure_buffer(
        device: &wgpu::Device,
        buf: &mut wgpu::Buffer,
        current: &mut usize,
        needed: usize,
        label: &str,
    ) {
        if needed == 0 {
            return;
        }
        if needed as u64 <= *current as u64 {
            return;
        }
        let new_size = needed.next_power_of_two().max(1024) as u64;
        *buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: new_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        *current = new_size as usize;
    }

    pub fn push_rect(rects: &mut Vec<RectVertex>, x0: f32, y0: f32, x1: f32, y1: f32, color: Rgba) {
        let alpha = color.a.max(1) as f32 / 255.0;
        let c = [
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
            alpha,
        ];
        rects.extend_from_slice(&[
            RectVertex {
                pos: [x0, y0],
                color: c,
            },
            RectVertex {
                pos: [x1, y0],
                color: c,
            },
            RectVertex {
                pos: [x1, y1],
                color: c,
            },
            RectVertex {
                pos: [x0, y0],
                color: c,
            },
            RectVertex {
                pos: [x1, y1],
                color: c,
            },
            RectVertex {
                pos: [x0, y1],
                color: c,
            },
        ]);
    }

    fn push_selection(rects: &mut Vec<RectVertex>, x0: f32, y0: f32, x1: f32, y1: f32) {
        let alpha = SELECTION[3].max(1) as f32 / 255.0;
        let c = [
            SELECTION[0] as f32 / 255.0,
            SELECTION[1] as f32 / 255.0,
            SELECTION[2] as f32 / 255.0,
            alpha,
        ];
        rects.extend_from_slice(&[
            RectVertex {
                pos: [x0, y0],
                color: c,
            },
            RectVertex {
                pos: [x1, y0],
                color: c,
            },
            RectVertex {
                pos: [x1, y1],
                color: c,
            },
            RectVertex {
                pos: [x0, y0],
                color: c,
            },
            RectVertex {
                pos: [x1, y1],
                color: c,
            },
            RectVertex {
                pos: [x0, y1],
                color: c,
            },
        ]);
    }

    pub fn push_text_line(
        glyphs: &mut GlyphCache,
        atlas: &mut GlyphAtlas,
        glyph_vertices: &mut Vec<GlyphVertex>,
        text: &str,
        x: f32,
        y: f32,
        color: Rgba,
        queue: &wgpu::Queue,
    ) {
        let baseline = glyphs.baseline();
        let mut pen_x = x;
        for ch in text.chars() {
            let _ = Self::push_glyph(
                glyphs,
                atlas,
                glyph_vertices,
                None,
                ch,
                pen_x,
                y,
                baseline,
                None,
                None,
                false,
                false,
                color,
                queue,
            );
            pen_x += glyphs.advance_width(ch) as f32;
        }
    }

    fn push_glyph(
        glyphs: &mut GlyphCache,
        atlas: &mut GlyphAtlas,
        glyph_vertices: &mut Vec<GlyphVertex>,
        fallback_rects: Option<&mut Vec<RectVertex>>,
        ch: char,
        base_x: f32,
        base_y: f32,
        baseline: i32,
        cell_h: Option<u32>,
        cell_w: Option<u32>,
        italic: bool,
        bold: bool,
        color: Rgba,
        queue: &wgpu::Queue,
    ) -> bool {
        let Some(entry) = atlas.get_or_insert(ch, glyphs, queue) else {
            if let Some(rects) = fallback_rects {
                // Draw a thin fallback bar so missing glyphs are visible.
                let fallback_y = base_y + baseline as f32 - 3.0;
                Self::push_rect(
                    rects,
                    base_x,
                    fallback_y,
                    base_x + glyphs.advance_width(ch).max(1) as f32,
                    fallback_y + 2.0,
                    color,
                );
            }
            return false;
        };
        let w = entry.size[0] as f32;
        let mut h = entry.size[1] as f32;
        let skew = if italic { h / 4.0 } else { 0.0 };

        let desired_top = base_y + baseline as f32 - entry.bearing[1] as f32;
        let top = if let Some(cell_h) = cell_h {
            let max_top = (base_y + (cell_h as f32 - h)).max(base_y);
            desired_top.clamp(base_y, max_top)
        } else {
            desired_top
        };
        if let Some(cell_h) = cell_h {
            h = h.min(cell_h as f32);
        }
        let mut left = base_x + entry.bearing[0] as f32;
        if let Some(cell_w) = cell_w {
            let advance = glyphs.advance_width_f32(ch);
            if advance > 0.0 {
                let pad = ((cell_w as f32 - advance) / 2.0).clamp(-2.0, 2.0);
                if pad.abs() >= 0.25 {
                    left += pad;
                }
            }
        }
        let tl = [left + skew, top];
        let br = [left + w, top + h];

        let uv = entry.uv;
        let base_color = if entry.color {
            Rgba {
                r: 255,
                g: 255,
                b: 255,
                a: color.a,
            }
        } else {
            color
        };
        let alpha = base_color.a.max(1) as f32 / 255.0;
        let col = [
            base_color.r as f32 / 255.0,
            base_color.g as f32 / 255.0,
            base_color.b as f32 / 255.0,
            alpha,
        ];
        let glyph_flags = if entry.color { 1.0 } else { 0.0 };

        let instance = GlyphVertex {
            rect: [tl[0], tl[1], br[0], br[1]],
            uv_rect: [uv[0], uv[1], uv[2], uv[3]],
            color: col,
            flags: glyph_flags,
        };
        glyph_vertices.push(instance);

        if bold {
            let offset = 1.0;
            let instance_bold = GlyphVertex {
                rect: [tl[0] + offset, tl[1], br[0] + offset, br[1]],
                uv_rect: [uv[0], uv[1], uv[2], uv[3]],
                color: col,
                flags: glyph_flags,
            };
            glyph_vertices.push(instance_bold);
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_grid(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        context: &pixels::PixelsContext,
        term: &Terminal,
        base_bg: Rgba,
        cursor_color: Rgba,
        glyphs: &mut GlyphCache,
        cell_w: u32,
        cell_h: u32,
        origin_x: u32,
        origin_y: u32,
        frame_width: u32,
        frame_height: u32,
        selection: Option<((usize, usize), (usize, usize))>,
        hover: Option<(usize, usize)>,
        hover_link_range: Option<(usize, usize, usize)>,
        link_ranges: Option<&[Vec<(usize, usize)>]>,
        cell_blink_on: bool,
        cursor_blink_on: bool,
        draw_cursor: bool,
        mut overlay: impl FnMut(
            &mut Vec<RectVertex>,
            &mut Vec<GlyphVertex>,
            &mut GlyphAtlas,
            &wgpu::Queue,
            &mut GlyphCache,
        ),
        damage_rects: Option<&[(u32, u32, u32, u32)]>,
        dirty_rows: Option<&[usize]>,
    ) {
        if frame_width == 0 || frame_height == 0 {
            return;
        }
        if self.screen_size[0] != frame_width as f32 || self.screen_size[1] != frame_height as f32 {
            self.resize(frame_width, frame_height, &context.queue);
        }

        let baseline = glyphs.baseline();
        let selected = selection.map(|(a, b)| {
            let (mut c0, mut r0) = a;
            let (mut c1, mut r1) = b;
            if r0 > r1 {
                std::mem::swap(&mut r0, &mut r1);
            }
            if c0 > c1 {
                std::mem::swap(&mut c0, &mut c1);
            }
            (c0, r0, c1, r1)
        });

        let mut rects_bg = Vec::new();
        let mut rects_lines = Vec::new();
        let mut rects_lines_add = Vec::new();
        let mut rects_overlay = Vec::new();
        let mut glyph_vertices = Vec::new();
        let mut overlay_glyphs = Vec::new();
        let mut clear_bg = base_bg;
        if clear_bg.a == 0 {
            clear_bg.a = 255;
        }

        let mut attempts = 0;
        loop {
            let mut overflowed = false;
            rects_bg.clear();
            rects_lines.clear();
            rects_lines_add.clear();
            rects_overlay.clear();
            glyph_vertices.clear();
            overlay_glyphs.clear();
            if let Some(damage) = damage_rects {
                if !damage.is_empty() {
                    for &(dx, dy, dw, dh) in damage.iter() {
                        if dw == 0 || dh == 0 {
                            continue;
                        }
                        Self::push_rect(
                            &mut rects_bg,
                            dx as f32,
                            dy as f32,
                            (dx + dw) as f32,
                            (dy + dh) as f32,
                            clear_bg,
                        );
                    }
                }
            }

            let mut emit_row = |row: usize, overflowed: &mut bool| {
                let row_y = origin_y as f32 + row as f32 * cell_h as f32;
                let mut bg_start: Option<usize> = None;
                let mut bg_color = base_bg;
                for col in 0..term.cols {
                    let cell = term.display_cell_ref(col, row);
                    let bg = if cell.bg.r == base_bg.r
                        && cell.bg.g == base_bg.g
                        && cell.bg.b == base_bg.b
                        && cell.bg.a == base_bg.a
                    {
                        None
                    } else if cell.bg.a == 0 {
                        None
                    } else {
                        Some(cell.bg)
                    };
                    match (bg_start, bg) {
                        (Some(_start), Some(color)) if color == bg_color => {}
                        (Some(start), Some(color)) => {
                            let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                            let x1 = origin_x as f32 + col as f32 * cell_w as f32;
                            Self::push_rect(
                                &mut rects_bg,
                                x0,
                                row_y,
                                x1,
                                row_y + cell_h as f32,
                                bg_color,
                            );
                            bg_start = Some(col);
                            bg_color = color;
                        }
                        (Some(start), None) => {
                            let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                            let x1 = origin_x as f32 + col as f32 * cell_w as f32;
                            Self::push_rect(
                                &mut rects_bg,
                                x0,
                                row_y,
                                x1,
                                row_y + cell_h as f32,
                                bg_color,
                            );
                            bg_start = None;
                        }
                        (None, Some(color)) => {
                            bg_start = Some(col);
                            bg_color = color;
                        }
                        (None, None) => {}
                    }
                }
                if let Some(start) = bg_start {
                    let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                    let x1 = origin_x as f32 + term.cols as f32 * cell_w as f32;
                    Self::push_rect(
                        &mut rects_bg,
                        x0,
                        row_y,
                        x1,
                        row_y + cell_h as f32,
                        bg_color,
                    );
                }

                if let Some((c0, r0, c1, r1)) = selected
                    && row >= r0
                    && row <= r1
                {
                    let start = c0.min(term.cols.saturating_sub(1));
                    let end = c1.min(term.cols.saturating_sub(1));
                    if start <= end {
                        let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                        let x1 = origin_x as f32 + (end + 1) as f32 * cell_w as f32;
                        Self::push_selection(&mut rects_bg, x0, row_y, x1, row_y + cell_h as f32);
                    }
                }

                if let Some((hc, hr)) = hover
                    && hr == row
                    && hc < term.cols
                {
                    let x0 = origin_x as f32 + hc as f32 * cell_w as f32;
                    let x1 = origin_x as f32 + (hc + 1) as f32 * cell_w as f32;
                    Self::push_rect(
                        &mut rects_bg,
                        x0,
                        row_y,
                        x1,
                        row_y + cell_h as f32,
                        Rgba {
                            r: 120,
                            g: 180,
                            b: 255,
                            a: 60,
                        },
                    );
                }

                let mut ul_start: Option<usize> = None;
                let mut ul_color = base_bg;
                let mut ol_start: Option<usize> = None;
                let mut ol_color = base_bg;
                let mut st_start: Option<usize> = None;
                let mut st_color = base_bg;
                let mut link_start: Option<usize> = None;
                for col in 0..term.cols {
                    let cell = term.display_cell_ref(col, row);
                    let fg = ensure_contrast(cell.fg, cell.bg);
                    let has_lines =
                        !cell.wide_continuation && (cell.underline || cell.overline || cell.strike);
                    let underline = has_lines && cell.underline;
                    let overline = has_lines && cell.overline;
                    let strike = has_lines && cell.strike;

                    let hovered_link = hover_link_range
                        .filter(|(r, start, end)| *r == row && col >= *start && col <= *end);
                    let row_ranges = link_ranges.and_then(|ranges| ranges.get(row));
                    let in_link_range = row_ranges
                        .map(|ranges| {
                            ranges
                                .iter()
                                .any(|(start, end)| col >= *start && col <= *end)
                        })
                        .unwrap_or(false);
                    let link = !cell.wide_continuation
                        && (cell.hyperlink.is_some() || hovered_link.is_some() || in_link_range);

                    match (ul_start, underline) {
                        (Some(_start), true) if fg == ul_color => {}
                        (Some(start), true) => {
                            let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                            let x1 = origin_x as f32 + col as f32 * cell_w as f32;
                            let y = row_y + cell_h as f32 - 2.0;
                            Self::push_rect(&mut rects_lines, x0, y, x1, y + 1.0, ul_color);
                            ul_start = Some(col);
                            ul_color = fg;
                        }
                        (Some(start), false) => {
                            let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                            let x1 = origin_x as f32 + col as f32 * cell_w as f32;
                            let y = row_y + cell_h as f32 - 2.0;
                            Self::push_rect(&mut rects_lines, x0, y, x1, y + 1.0, ul_color);
                            ul_start = None;
                        }
                        (None, true) => {
                            ul_start = Some(col);
                            ul_color = fg;
                        }
                        (None, false) => {}
                    }

                    match (ol_start, overline) {
                        (Some(_start), true) if fg == ol_color => {}
                        (Some(start), true) => {
                            let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                            let x1 = origin_x as f32 + col as f32 * cell_w as f32;
                            Self::push_rect(&mut rects_lines, x0, row_y, x1, row_y + 1.0, ol_color);
                            ol_start = Some(col);
                            ol_color = fg;
                        }
                        (Some(start), false) => {
                            let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                            let x1 = origin_x as f32 + col as f32 * cell_w as f32;
                            Self::push_rect(&mut rects_lines, x0, row_y, x1, row_y + 1.0, ol_color);
                            ol_start = None;
                        }
                        (None, true) => {
                            ol_start = Some(col);
                            ol_color = fg;
                        }
                        (None, false) => {}
                    }

                    match (st_start, strike) {
                        (Some(_start), true) if fg == st_color => {}
                        (Some(start), true) => {
                            let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                            let x1 = origin_x as f32 + col as f32 * cell_w as f32;
                            let y = row_y + (cell_h as f32 / 2.0);
                            Self::push_rect(&mut rects_lines, x0, y, x1, y + 1.0, st_color);
                            st_start = Some(col);
                            st_color = fg;
                        }
                        (Some(start), false) => {
                            let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                            let x1 = origin_x as f32 + col as f32 * cell_w as f32;
                            let y = row_y + (cell_h as f32 / 2.0);
                            Self::push_rect(&mut rects_lines, x0, y, x1, y + 1.0, st_color);
                            st_start = None;
                        }
                        (None, true) => {
                            st_start = Some(col);
                            st_color = fg;
                        }
                        (None, false) => {}
                    }

                    match (link_start, link) {
                        (Some(_), true) => {}
                        (Some(start), false) => {
                            let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                            let x1 = origin_x as f32 + col as f32 * cell_w as f32;
                            let y = row_y + cell_h as f32 - 2.0;
                            Self::push_rect(
                                &mut rects_lines_add,
                                x0,
                                y,
                                x1,
                                y + 1.0,
                                Rgba {
                                    r: 20,
                                    g: 60,
                                    b: 120,
                                    a: 0,
                                },
                            );
                            link_start = None;
                        }
                        (None, true) => {
                            link_start = Some(col);
                        }
                        (None, false) => {}
                    }
                }
                if let Some(start) = ul_start {
                    let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                    let x1 = origin_x as f32 + term.cols as f32 * cell_w as f32;
                    let y = row_y + cell_h as f32 - 2.0;
                    Self::push_rect(&mut rects_lines, x0, y, x1, y + 1.0, ul_color);
                }
                if let Some(start) = ol_start {
                    let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                    let x1 = origin_x as f32 + term.cols as f32 * cell_w as f32;
                    Self::push_rect(&mut rects_lines, x0, row_y, x1, row_y + 1.0, ol_color);
                }
                if let Some(start) = st_start {
                    let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                    let x1 = origin_x as f32 + term.cols as f32 * cell_w as f32;
                    let y = row_y + (cell_h as f32 / 2.0);
                    Self::push_rect(&mut rects_lines, x0, y, x1, y + 1.0, st_color);
                }
                if let Some(start) = link_start {
                    let x0 = origin_x as f32 + start as f32 * cell_w as f32;
                    let x1 = origin_x as f32 + term.cols as f32 * cell_w as f32;
                    let y = row_y + cell_h as f32 - 2.0;
                    Self::push_rect(
                        &mut rects_lines_add,
                        x0,
                        y,
                        x1,
                        y + 1.0,
                        Rgba {
                            r: 20,
                            g: 60,
                            b: 120,
                            a: 0,
                        },
                    );
                }

                for col in 0..term.cols {
                    let cell = term.display_cell_ref(col, row);
                    let base_x = origin_x as f32 + col as f32 * cell_w as f32;
                    if cell.wide_continuation {
                        continue;
                    }
                    let has_lines = cell.underline || cell.overline || cell.strike;
                    if cell.is_blank() && !has_lines {
                        continue;
                    }

                    let fg = ensure_contrast(cell.fg, cell.bg);
                    if !cell.is_blank() {
                        let mut glyph_fg = if cell.concealed { cell.bg } else { fg };
                        if cell.faint {
                            glyph_fg = faintened(glyph_fg);
                        }
                        if cell.bold {
                            glyph_fg = brightened(glyph_fg);
                        }
                        if cell.blink && !cell_blink_on && !cell.concealed {
                            glyph_fg.a = ((glyph_fg.a as u16 * 128) / 255) as u8;
                        }

                        let span = if cell.wide { 2 } else { 1 };
                        let cell_span_w = cell_w.saturating_mul(span);
                        let is_single_glyph = cell.text.chars().count() == 1;
                        let mut pen_x = base_x;
                        for ch in cell.text.chars() {
                            let drawn = Self::push_glyph(
                                glyphs,
                                &mut self.atlas,
                                &mut glyph_vertices,
                                Some(&mut rects_lines),
                                ch,
                                pen_x,
                                row_y,
                                baseline,
                                Some(cell_h),
                                if is_single_glyph { Some(cell_span_w) } else { None },
                                cell.italic,
                                cell.bold,
                                glyph_fg,
                                &context.queue,
                            );
                            if !drawn {
                                *overflowed = true;
                            }
                            if !drawn && !ATLAS_INSERT_WARNED.swap(true, Ordering::Relaxed) {
                                log::warn!(
                                    "term: glyph atlas overflowed; drew fallbacks. Increase TERM_ATLAS_SIZE or reduce glyph variety."
                                );
                            }
                            pen_x += glyphs.advance_width(ch) as f32;
                        }
                    }
                }
            };

            if let Some(rows) = dirty_rows {
                for &row in rows {
                    emit_row(row, &mut overflowed);
                }
            } else {
                for row in 0..term.rows {
                    emit_row(row, &mut overflowed);
                }
            }

            // Cursor overlay
            if draw_cursor
                && term.view_offset == 0
                && term.cursor_row < term.rows
                && term.cursor_col < term.cols
            {
                let mut cursor_col = term.cursor_col;
                let cursor_row = term.cursor_row;
                let mut cell = term.display_cell_ref(cursor_col, cursor_row);
                if cell.wide_continuation && cursor_col > 0 {
                    cursor_col -= 1;
                    cell = term.display_cell_ref(cursor_col, cursor_row);
                }
                let cursor_selected = selected
                    .map(|(c0, r0, c1, r1)| {
                        cursor_row >= r0 && cursor_row <= r1 && cursor_col >= c0 && cursor_col <= c1
                    })
                    .unwrap_or(false);

                let span = if cell.wide { 2u32 } else { 1u32 };
                let cursor_x = origin_x
                    .saturating_add(cursor_col as u32 * cell_w)
                    .min(frame_width.saturating_sub(1));
                if !cursor_selected && term.cursor_visible() && cursor_blink_on {
                    let (cursor_w, cursor_h, cursor_y, cursor_x) = match term.cursor_shape() {
                        crate::terminal::CursorShape::Underline => {
                            let cursor_thickness = cell_h.max(1).min(2);
                            let base_y = origin_y as i32 + cursor_row as i32 * cell_h as i32;
                            let cursor_y = (base_y + cell_h as i32 - cursor_thickness as i32)
                                .clamp(0, frame_height.saturating_sub(1) as i32)
                                as u32;
                            let cursor_w =
                                (span * cell_w).min(frame_width.saturating_sub(cursor_x));
                            let cursor_h =
                                cursor_thickness.min(frame_height.saturating_sub(cursor_y));
                            (cursor_w, cursor_h, cursor_y, cursor_x)
                        }
                        crate::terminal::CursorShape::Bar => {
                            let bar_w = (cell_w / 6).max(1);
                            let cursor_w = bar_w.min(frame_width.saturating_sub(cursor_x));
                            let cursor_h = cell_h.min(frame_height.saturating_sub(
                                origin_y.saturating_add(cursor_row as u32 * cell_h),
                            ));
                            let cursor_y = origin_y.saturating_add(cursor_row as u32 * cell_h);
                            (cursor_w, cursor_h, cursor_y, cursor_x)
                        }
                        crate::terminal::CursorShape::Block => {
                            let cursor_w =
                                (span * cell_w).min(frame_width.saturating_sub(cursor_x));
                            let cursor_h = cell_h.min(frame_height.saturating_sub(
                                origin_y.saturating_add(cursor_row as u32 * cell_h),
                            ));
                            let cursor_y = origin_y.saturating_add(cursor_row as u32 * cell_h);
                            (cursor_w, cursor_h, cursor_y, cursor_x)
                        }
                    };

                    if cursor_w > 0 && cursor_h > 0 {
                        Self::push_rect(
                            &mut rects_overlay,
                            cursor_x as f32,
                            cursor_y as f32,
                            (cursor_x + cursor_w) as f32,
                            (cursor_y + cursor_h) as f32,
                            cursor_color,
                        );
                    }

                    if matches!(term.cursor_shape(), crate::terminal::CursorShape::Block) {
                        let fg = ensure_contrast(cell.fg, cell.bg);
                        let mut glyph_fg = if cell.concealed { cell.bg } else { fg };
                        if cell.faint {
                            glyph_fg = faintened(glyph_fg);
                        }
                        if cell.bold {
                            glyph_fg = brightened(glyph_fg);
                        }
                        if cell.blink && !cell_blink_on && !cell.concealed {
                            glyph_fg.a = ((glyph_fg.a as u16 * 128) / 255) as u8;
                        }

                        let base_x = origin_x as f32 + cursor_col as f32 * cell_w as f32;
                        let base_y = origin_y as f32 + cursor_row as f32 * cell_h as f32;
                        let span = if cell.wide { 2 } else { 1 };
                        let cell_span_w = cell_w.saturating_mul(span);
                        let is_single_glyph = cell.text.chars().count() == 1;
                        let mut pen_x = base_x;
                        for ch in cell.text.chars() {
                            let drawn = Self::push_glyph(
                                glyphs,
                                &mut self.atlas,
                                &mut overlay_glyphs,
                                Some(&mut rects_overlay),
                                ch,
                                pen_x,
                                base_y,
                                baseline,
                                Some(cell_h),
                                if is_single_glyph { Some(cell_span_w) } else { None },
                                cell.italic,
                                cell.bold,
                                glyph_fg,
                                &context.queue,
                            );
                            if !drawn {
                                overflowed = true;
                            }
                            pen_x += glyphs.advance_width(ch) as f32;
                        }
                    }
                }
            }

            // Overlays appended after grid data
            overlay(
                &mut rects_overlay,
                &mut overlay_glyphs,
                &mut self.atlas,
                &context.queue,
                glyphs,
            );

            if overflowed && self.atlas_size < self.max_atlas_size {
                let next = (self.atlas_size.saturating_mul(2)).min(self.max_atlas_size);
                if next > self.atlas_size {
                    log::warn!(
                        "term: atlas overflowed size={} max={} env_grow={:?}",
                        self.atlas_size,
                        self.max_atlas_size,
                        env::var("TERM_ATLAS_GROW").ok()
                    );
                    log::warn!(
                        "term: glyph atlas overflowed; growing atlas to {} (max {})",
                        next,
                        self.max_atlas_size
                    );
                    self.rebuild_atlas(&context.device, next);
                    attempts += 1;
                    if attempts < 3 {
                        continue;
                    }
                }
            }
            if self.atlas.take_cleared() {
                attempts += 1;
                if attempts < 3 {
                    continue;
                }
            }
            break;
        }

        let bg_len = rects_bg.len();
        let lines_len = rects_lines.len();
        let add_len = rects_lines_add.len();
        let overlay_len = rects_overlay.len();
        let rects_len = bg_len + lines_len + add_len + overlay_len;
        let bg_end = bg_len as u32;
        let lines_end = (bg_len + lines_len) as u32;
        let add_end = (bg_len + lines_len + add_len) as u32;
        let overlay_end = rects_len as u32;

        let overlay_glyphs_len = overlay_glyphs.len();
        let grid_glyphs_len = glyph_vertices.len();
        if overlay_glyphs_len > 0 {
            glyph_vertices.extend_from_slice(&overlay_glyphs);
        }
        let glyphs_len = glyph_vertices.len();
        let grid_glyphs_end = grid_glyphs_len as u32;
        let total_glyphs = glyphs_len as u32;

        let mut rects = Vec::with_capacity(rects_len);
        if bg_len > 0 {
            rects.extend_from_slice(&rects_bg);
        }
        if lines_len > 0 {
            rects.extend_from_slice(&rects_lines);
        }
        if add_len > 0 {
            rects.extend_from_slice(&rects_lines_add);
        }
        if overlay_len > 0 {
            rects.extend_from_slice(&rects_overlay);
        }

        let device = &context.device;
        Self::ensure_buffer(
            device,
            &mut self.rect_vertex_buffer,
            &mut self.rect_capacity,
            rects.len() * std::mem::size_of::<RectVertex>(),
            "term_rect_buffer",
        );
        Self::ensure_buffer(
            device,
            &mut self.glyph_vertex_buffer,
            &mut self.glyph_capacity,
            glyph_vertices.len() * std::mem::size_of::<GlyphVertex>(),
            "term_glyph_buffer",
        );

        if !rects.is_empty() {
            let data: &[u8] = bytemuck::cast_slice(&rects);
            context
                .queue
                .write_buffer(&self.rect_vertex_buffer, 0, data);
        }
        if !glyph_vertices.is_empty() {
            let data: &[u8] = bytemuck::cast_slice(&glyph_vertices);
            context
                .queue
                .write_buffer(&self.glyph_vertex_buffer, 0, data);
        }

        // If damage rects are provided, render only those regions using scissor rects and
        // load existing content. Otherwise, clear full target and render entire frame.
        if let Some(damage) = damage_rects {
            if !damage.is_empty() {
                // Load the existing base and repaint only the dirty geometry without scissoring.
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("term_gpu_grid_damage"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: render_target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

                if bg_len > 0 {
                    pass.set_pipeline(&self.rect_pipeline);
                    pass.set_bind_group(0, &self.screen_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
                    pass.draw(0..bg_end, 0..1);
                }

                if grid_glyphs_len > 0 {
                    pass.set_pipeline(&self.glyph_pipeline);
                    pass.set_bind_group(0, &self.screen_bind_group, &[]);
                    pass.set_bind_group(1, &self.glyph_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.glyph_quad_buffer.slice(..));
                    pass.set_vertex_buffer(1, self.glyph_vertex_buffer.slice(..));
                    pass.draw(0..6, 0..grid_glyphs_end);
                }

                if lines_len > 0 {
                    pass.set_pipeline(&self.rect_pipeline);
                    pass.set_bind_group(0, &self.screen_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
                    pass.draw(bg_end..lines_end, 0..1);
                }

                if add_len > 0 {
                    pass.set_pipeline(&self.rect_add_pipeline);
                    pass.set_bind_group(0, &self.screen_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
                    pass.draw(lines_end..add_end, 0..1);
                }

                if overlay_len > 0 {
                    pass.set_pipeline(&self.rect_pipeline);
                    pass.set_bind_group(0, &self.screen_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
                    pass.draw(add_end..overlay_end, 0..1);
                }

                if overlay_glyphs_len > 0 {
                    pass.set_pipeline(&self.glyph_pipeline);
                    pass.set_bind_group(0, &self.screen_bind_group, &[]);
                    pass.set_bind_group(1, &self.glyph_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.glyph_quad_buffer.slice(..));
                    pass.set_vertex_buffer(1, self.glyph_vertex_buffer.slice(..));
                    pass.draw(0..6, grid_glyphs_end..total_glyphs);
                }
            } else {
                // no damage rects: clear full target
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("term_gpu_grid"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: render_target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: clear_bg.r as f64 / 255.0,
                                g: clear_bg.g as f64 / 255.0,
                                b: clear_bg.b as f64 / 255.0,
                                a: clear_bg.a as f64 / 255.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

                if bg_len > 0 {
                    pass.set_pipeline(&self.rect_pipeline);
                    pass.set_bind_group(0, &self.screen_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
                    pass.draw(0..bg_end, 0..1);
                }

                if grid_glyphs_len > 0 {
                    pass.set_pipeline(&self.glyph_pipeline);
                    pass.set_bind_group(0, &self.screen_bind_group, &[]);
                    pass.set_bind_group(1, &self.glyph_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.glyph_quad_buffer.slice(..));
                    pass.set_vertex_buffer(1, self.glyph_vertex_buffer.slice(..));
                    pass.draw(0..6, 0..grid_glyphs_end);
                }

                if lines_len > 0 {
                    pass.set_pipeline(&self.rect_pipeline);
                    pass.set_bind_group(0, &self.screen_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
                    pass.draw(bg_end..lines_end, 0..1);
                }

                if add_len > 0 {
                    pass.set_pipeline(&self.rect_add_pipeline);
                    pass.set_bind_group(0, &self.screen_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
                    pass.draw(lines_end..add_end, 0..1);
                }

                if overlay_len > 0 {
                    pass.set_pipeline(&self.rect_pipeline);
                    pass.set_bind_group(0, &self.screen_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
                    pass.draw(add_end..overlay_end, 0..1);
                }

                if overlay_glyphs_len > 0 {
                    pass.set_pipeline(&self.glyph_pipeline);
                    pass.set_bind_group(0, &self.screen_bind_group, &[]);
                    pass.set_bind_group(1, &self.glyph_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.glyph_quad_buffer.slice(..));
                    pass.set_vertex_buffer(1, self.glyph_vertex_buffer.slice(..));
                    pass.draw(0..6, grid_glyphs_end..total_glyphs);
                }
            }
        } else {
            // default: clear full target
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("term_gpu_grid"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_bg.r as f64 / 255.0,
                            g: clear_bg.g as f64 / 255.0,
                            b: clear_bg.b as f64 / 255.0,
                            a: clear_bg.a as f64 / 255.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

            if bg_len > 0 {
                pass.set_pipeline(&self.rect_pipeline);
                pass.set_bind_group(0, &self.screen_bind_group, &[]);
                pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
                pass.draw(0..bg_end, 0..1);
            }

            if grid_glyphs_len > 0 {
                pass.set_pipeline(&self.glyph_pipeline);
                pass.set_bind_group(0, &self.screen_bind_group, &[]);
                pass.set_bind_group(1, &self.glyph_bind_group, &[]);
                pass.set_vertex_buffer(0, self.glyph_quad_buffer.slice(..));
                pass.set_vertex_buffer(1, self.glyph_vertex_buffer.slice(..));
                pass.draw(0..6, 0..grid_glyphs_end);
            }

            if lines_len > 0 {
                pass.set_pipeline(&self.rect_pipeline);
                pass.set_bind_group(0, &self.screen_bind_group, &[]);
                pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
                pass.draw(bg_end..lines_end, 0..1);
            }

            if add_len > 0 {
                pass.set_pipeline(&self.rect_add_pipeline);
                pass.set_bind_group(0, &self.screen_bind_group, &[]);
                pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
                pass.draw(lines_end..add_end, 0..1);
            }

            if overlay_len > 0 {
                pass.set_pipeline(&self.rect_pipeline);
                pass.set_bind_group(0, &self.screen_bind_group, &[]);
                pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
                pass.draw(add_end..overlay_end, 0..1);
            }

            if overlay_glyphs_len > 0 {
                pass.set_pipeline(&self.glyph_pipeline);
                pass.set_bind_group(0, &self.screen_bind_group, &[]);
                pass.set_bind_group(1, &self.glyph_bind_group, &[]);
                pass.set_vertex_buffer(0, self.glyph_quad_buffer.slice(..));
                pass.set_vertex_buffer(1, self.glyph_vertex_buffer.slice(..));
                pass.draw(0..6, grid_glyphs_end..total_glyphs);
            }
        }
        self.staging_belt.finish();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_cursor_cell(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        context: &pixels::PixelsContext,
        term: &Terminal,
        cursor_color: Rgba,
        glyphs: &mut GlyphCache,
        cell_w: u32,
        cell_h: u32,
        origin_x: u32,
        origin_y: u32,
        frame_width: u32,
        frame_height: u32,
        selection: Option<((usize, usize), (usize, usize))>,
        cursor_on: bool,
        cell_blink_on: bool,
    ) {
        if frame_width == 0 || frame_height == 0 {
            return;
        }
        if self.screen_size[0] != frame_width as f32 || self.screen_size[1] != frame_height as f32 {
            self.resize(frame_width, frame_height, &context.queue);
        }

        if term.view_offset != 0 || term.cursor_row >= term.rows || term.cursor_col >= term.cols {
            return;
        }

        let selected = selection_bounds(selection);

        let mut rects_overlay = Vec::new();
        let mut overlay_glyphs = Vec::new();
        let baseline = glyphs.baseline();

        let mut attempts = 0;
        loop {
            let mut overflowed = false;
            rects_overlay.clear();
            overlay_glyphs.clear();

            let mut cursor_col = term.cursor_col;
            let cursor_row = term.cursor_row;
            let mut cell = term.display_cell_ref(cursor_col, cursor_row);
            if cell.wide_continuation && cursor_col > 0 {
                cursor_col -= 1;
                cell = term.display_cell_ref(cursor_col, cursor_row);
            }
            let cursor_selected = selected
                .map(|(c0, r0, c1, r1)| {
                    cursor_row >= r0 && cursor_row <= r1 && cursor_col >= c0 && cursor_col <= c1
                })
                .unwrap_or(false);
            if cursor_on && !cursor_selected && term.cursor_visible() {
                let span = if cell.wide { 2u32 } else { 1u32 };
                let cursor_x = origin_x
                    .saturating_add(cursor_col as u32 * cell_w)
                    .min(frame_width.saturating_sub(1));
                let (cursor_w, cursor_h, cursor_y, cursor_x) = match term.cursor_shape() {
                    crate::terminal::CursorShape::Underline => {
                        let cursor_thickness = cell_h.max(1).min(2);
                        let base_y = origin_y as i32 + cursor_row as i32 * cell_h as i32;
                        let cursor_y = (base_y + cell_h as i32 - cursor_thickness as i32)
                            .clamp(0, frame_height.saturating_sub(1) as i32)
                            as u32;
                        let cursor_w = (span * cell_w).min(frame_width.saturating_sub(cursor_x));
                        let cursor_h = cursor_thickness.min(frame_height.saturating_sub(cursor_y));
                        (cursor_w, cursor_h, cursor_y, cursor_x)
                    }
                    crate::terminal::CursorShape::Bar => {
                        let bar_w = (cell_w / 6).max(1);
                        let cursor_w = bar_w.min(frame_width.saturating_sub(cursor_x));
                        let cursor_h =
                            cell_h.min(frame_height.saturating_sub(
                                origin_y.saturating_add(cursor_row as u32 * cell_h),
                            ));
                        let cursor_y = origin_y.saturating_add(cursor_row as u32 * cell_h);
                        (cursor_w, cursor_h, cursor_y, cursor_x)
                    }
                    crate::terminal::CursorShape::Block => {
                        let cursor_w = (span * cell_w).min(frame_width.saturating_sub(cursor_x));
                        let cursor_h =
                            cell_h.min(frame_height.saturating_sub(
                                origin_y.saturating_add(cursor_row as u32 * cell_h),
                            ));
                        let cursor_y = origin_y.saturating_add(cursor_row as u32 * cell_h);
                        (cursor_w, cursor_h, cursor_y, cursor_x)
                    }
                };

                if cursor_w > 0 && cursor_h > 0 {
                    Self::push_rect(
                        &mut rects_overlay,
                        cursor_x as f32,
                        cursor_y as f32,
                        (cursor_x + cursor_w) as f32,
                        (cursor_y + cursor_h) as f32,
                        cursor_color,
                    );
                }

                if matches!(term.cursor_shape(), crate::terminal::CursorShape::Block) {
                    let fg = ensure_contrast(cell.fg, cell.bg);
                    let mut glyph_fg = if cell.concealed { cell.bg } else { fg };
                    if cell.faint {
                        glyph_fg = faintened(glyph_fg);
                    }
                    if cell.bold {
                        glyph_fg = brightened(glyph_fg);
                    }
                    if cell.blink && !cell_blink_on && !cell.concealed {
                        glyph_fg.a = ((glyph_fg.a as u16 * 128) / 255) as u8;
                    }

                    let base_x = origin_x as f32 + cursor_col as f32 * cell_w as f32;
                    let base_y = origin_y as f32 + cursor_row as f32 * cell_h as f32;
                    let span = if cell.wide { 2 } else { 1 };
                    let cell_span_w = cell_w.saturating_mul(span);
                    let is_single_glyph = cell.text.chars().count() == 1;
                    let mut pen_x = base_x;
                    for ch in cell.text.chars() {
                        let drawn = Self::push_glyph(
                            glyphs,
                            &mut self.atlas,
                            &mut overlay_glyphs,
                            Some(&mut rects_overlay),
                            ch,
                            pen_x,
                            base_y,
                            baseline,
                            Some(cell_h),
                            if is_single_glyph { Some(cell_span_w) } else { None },
                            cell.italic,
                            cell.bold,
                            glyph_fg,
                            &context.queue,
                        );
                        if !drawn {
                            overflowed = true;
                        }
                        pen_x += glyphs.advance_width(ch) as f32;
                    }
                }
            }

            if overflowed && self.atlas_size < self.max_atlas_size {
                let next = (self.atlas_size.saturating_mul(2)).min(self.max_atlas_size);
                if next > self.atlas_size {
                    log::warn!(
                        "term: glyph atlas overflowed; growing atlas to {} (max {})",
                        next,
                        self.max_atlas_size
                    );
                    self.rebuild_atlas(&context.device, next);
                    attempts += 1;
                    if attempts < 2 {
                        continue;
                    }
                }
            }
            if self.atlas.take_cleared() {
                attempts += 1;
                if attempts < 2 {
                    continue;
                }
            }
            break;
        }

        let overlay_glyphs_len = overlay_glyphs.len();
        let rects_len = rects_overlay.len();

        let device = &context.device;
        Self::ensure_buffer(
            device,
            &mut self.rect_cursor_buffer,
            &mut self.rect_cursor_capacity,
            rects_overlay.len() * std::mem::size_of::<RectVertex>(),
            "term_rect_cursor_buffer",
        );
        Self::ensure_buffer(
            device,
            &mut self.glyph_cursor_buffer,
            &mut self.glyph_cursor_capacity,
            overlay_glyphs.len() * std::mem::size_of::<GlyphVertex>(),
            "term_glyph_cursor_buffer",
        );

        if !rects_overlay.is_empty() {
            let data: &[u8] = bytemuck::cast_slice(&rects_overlay);
            context
                .queue
                .write_buffer(&self.rect_cursor_buffer, 0, data);
        }
        if !overlay_glyphs.is_empty() {
            let data: &[u8] = bytemuck::cast_slice(&overlay_glyphs);
            context
                .queue
                .write_buffer(&self.glyph_cursor_buffer, 0, data);
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("term_gpu_cursor_cell"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        if rects_len > 0 {
            pass.set_pipeline(&self.rect_pipeline);
            pass.set_bind_group(0, &self.screen_bind_group, &[]);
            pass.set_vertex_buffer(0, self.rect_cursor_buffer.slice(..));
            pass.draw(0..rects_len as u32, 0..1);
        }

        if overlay_glyphs_len > 0 {
            pass.set_pipeline(&self.glyph_pipeline);
            pass.set_bind_group(0, &self.screen_bind_group, &[]);
            pass.set_bind_group(1, &self.glyph_bind_group, &[]);
            pass.set_vertex_buffer(0, self.glyph_quad_buffer.slice(..));
            pass.set_vertex_buffer(1, self.glyph_cursor_buffer.slice(..));
            pass.draw(0..6, 0..overlay_glyphs_len as u32);
        }
    }
}

fn selection_bounds(
    selection: Option<((usize, usize), (usize, usize))>,
) -> Option<(usize, usize, usize, usize)> {
    selection.map(|(a, b)| {
        let (mut c0, mut r0) = a;
        let (mut c1, mut r1) = b;
        if r0 > r1 {
            std::mem::swap(&mut r0, &mut r1);
        }
        if c0 > c1 {
            std::mem::swap(&mut c0, &mut c1);
        }
        (c0, r0, c1, r1)
    })
}
