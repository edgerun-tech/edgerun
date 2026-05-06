use std::collections::HashMap;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};

use pixels::wgpu::{self, util::DeviceExt};

use crate::render::GlyphCache;
use crate::terminal::{Rgba, SELECTION, Terminal, ensure_contrast};

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
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
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
        // Use nearest filtering to keep bitmap glyphs crisp and avoid sampling bleed that can
        // distort measured heights compared to the CPU path.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("term_glyph_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
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
            return Some(entry);
        }

        loop {
            let (metrics, bitmap, is_color) = glyphs.rasterize(ch);
            if metrics.width == 0 || metrics.height == 0 {
                return None;
            }

            let w = metrics.width as u32;
            let h = metrics.height as u32;
            let space_needed = w.max(1);
            if space_needed > self.size || h > self.size {
                return None;
            }

            if self.cursor_x + w >= self.size {
                self.cursor_x = 0;
                self.cursor_y = self.cursor_y.saturating_add(self.row_height + 1);
                self.row_height = 0;
            }
            if self.cursor_y + h >= self.size {
                // Atlas full – clear once and retry, then give up.
                if cleared_once {
                    return None;
                }
                self.clear();
                cleared_once = true;
                continue;
            }

            if self.cursor_x + w >= self.size {
                self.cursor_x = 0;
                self.cursor_y = self.cursor_y.saturating_add(self.row_height + 1);
                self.row_height = 0;
            }

            if self.cursor_y + h >= self.size {
                if cleared_once {
                    return None;
                }
                self.clear();
                cleared_once = true;
                continue;
            }

            let x = self.cursor_x;
            let y = self.cursor_y;
            self.cursor_x += w + 1;
            self.row_height = self.row_height.max(h);

            let stride = ((w + 63) / 64) * 64; // keep aligned for wgpu row padding
            let mut padded = vec![0u8; (stride * h * 4) as usize];
            for row in 0..h as usize {
                let src_start = row * w as usize * 4;
                let dst_start = row * stride as usize * 4;
                padded[dst_start..dst_start + w as usize * 4]
                    .copy_from_slice(&bitmap[src_start..src_start + w as usize * 4]);
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
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );

            let uv = [
                x as f32 / self.size as f32,
                y as f32 / self.size as f32,
                (x + w) as f32 / self.size as f32,
                (y + h) as f32 / self.size as f32,
            ];

            let entry = AtlasEntry {
                uv,
                size: [w, h],
                bearing: [metrics.xmin, metrics.ymin],
                color: is_color,
            };
            self.entries.insert(ch, entry);
            return Some(entry);
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.row_height = 0;
        self.cleared = true;
    }

    fn take_cleared(&mut self) -> bool {
        let was_cleared = self.cleared;
        self.cleared = false;
        was_cleared
    }
}

pub struct GpuRenderer {
    rect_pipeline: wgpu::RenderPipeline,
    glyph_pipeline: wgpu::RenderPipeline,
    rect_vertex_buffer: wgpu::Buffer,
    glyph_vertex_buffer: wgpu::Buffer,
    rect_capacity: usize,
    glyph_capacity: usize,
    screen_uniform: wgpu::Buffer,
    screen_bind_group: wgpu::BindGroup,
    glyph_bind_group: wgpu::BindGroup,
    atlas: GlyphAtlas,
    screen_size: [f32; 2],
}

static ATLAS_INSERT_WARNED: AtomicBool = AtomicBool::new(false);

impl GpuRenderer {
    pub fn rainbow(phase: f32) -> [u8; 3] {
        let r = ((phase).sin() * 0.5 + 0.5) * 255.0;
        let g = ((phase + 2.094395_f32).sin() * 0.5 + 0.5) * 255.0;
        let b = ((phase + 4.18879_f32).sin() * 0.5 + 0.5) * 255.0;
        [r as u8, g as u8, b as u8]
    }

    pub fn new(context: &pixels::PixelsContext, surface_format: wgpu::TextureFormat) -> Self {
        let device = &context.device;
        // Default to a larger atlas to reduce eviction/clearing under heavy glyph variety.
        let atlas_size = env::var("TERM_ATLAS_SIZE")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .map(|v| v.clamp(512, 16384))
            .unwrap_or(8192);

        let screen_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("term_screen_uniform"),
            contents: bytemuck::cast_slice(&[0.0f32, 0.0f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let screen_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("term_screen_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
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
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GlyphVertex>() as wgpu::BufferAddress,
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
                        wgpu::VertexAttribute {
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 16,
                        },
                    ],
                }],
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

        Self {
            rect_pipeline,
            glyph_pipeline,
            rect_vertex_buffer,
            glyph_vertex_buffer,
            rect_capacity: 1024,
            glyph_capacity: 1024,
            screen_uniform,
            screen_bind_group,
            glyph_bind_group,
            atlas,
            screen_size: [0.0, 0.0],
        }
    }

    pub fn clear_atlas(&mut self) {
        self.atlas.clear();
    }

    pub fn resize(&mut self, width: u32, height: u32, queue: &wgpu::Queue) {
        if width == 0 || height == 0 {
            return;
        }
        self.screen_size = [width as f32, height as f32];
        queue.write_buffer(
            &self.screen_uniform,
            0,
            bytemuck::cast_slice(&self.screen_size),
        );
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
        let left = base_x + entry.bearing[0] as f32;
        let tl = [left + skew, top];
        let tr = [left + w + skew, top];
        let bl = [left, top + h];
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

        let quad = [
            GlyphVertex {
                pos: tl,
                uv: [uv[0], uv[1]],
                color: col,
            },
            GlyphVertex {
                pos: tr,
                uv: [uv[2], uv[1]],
                color: col,
            },
            GlyphVertex {
                pos: br,
                uv: [uv[2], uv[3]],
                color: col,
            },
            GlyphVertex {
                pos: tl,
                uv: [uv[0], uv[1]],
                color: col,
            },
            GlyphVertex {
                pos: br,
                uv: [uv[2], uv[3]],
                color: col,
            },
            GlyphVertex {
                pos: bl,
                uv: [uv[0], uv[3]],
                color: col,
            },
        ];
        glyph_vertices.extend_from_slice(&quad);

        if bold {
            let offset = 1.0;
            let tl = [tl[0] + offset, tl[1]];
            let tr = [tr[0] + offset, tr[1]];
            let bl = [bl[0] + offset, bl[1]];
            let br = [br[0] + offset, br[1]];
            let quad_bold = [
                GlyphVertex {
                    pos: tl,
                    uv: [uv[0], uv[1]],
                    color: col,
                },
                GlyphVertex {
                    pos: tr,
                    uv: [uv[2], uv[1]],
                    color: col,
                },
                GlyphVertex {
                    pos: br,
                    uv: [uv[2], uv[3]],
                    color: col,
                },
                GlyphVertex {
                    pos: tl,
                    uv: [uv[0], uv[1]],
                    color: col,
                },
                GlyphVertex {
                    pos: br,
                    uv: [uv[2], uv[3]],
                    color: col,
                },
                GlyphVertex {
                    pos: bl,
                    uv: [uv[0], uv[3]],
                    color: col,
                },
            ];
            glyph_vertices.extend_from_slice(&quad_bold);
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
        mut overlay: impl FnMut(
            &mut Vec<RectVertex>,
            &mut Vec<GlyphVertex>,
            &mut GlyphAtlas,
            &wgpu::Queue,
            &mut GlyphCache,
        ),
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

        let mut rects = Vec::new();
        let mut glyph_vertices = Vec::new();

        let mut attempts = 0;
        loop {
            rects.clear();
            glyph_vertices.clear();

            for row in 0..term.rows {
                for col in 0..term.cols {
                    let cell = term.display_cell(col, row);
                    let base_x = origin_x as f32 + col as f32 * cell_w as f32;
                    let base_y = origin_y as f32 + row as f32 * cell_h as f32;

                    if !(cell.bg.r == base_bg.r
                        && cell.bg.g == base_bg.g
                        && cell.bg.b == base_bg.b
                        && cell.bg.a == base_bg.a)
                        && cell.bg.a > 0
                    {
                        Self::push_rect(
                            &mut rects,
                            base_x,
                            base_y,
                            base_x + cell_w as f32,
                            base_y + cell_h as f32,
                            cell.bg,
                        );
                    }

                    if let Some((c0, r0, c1, r1)) = selected
                        && row >= r0
                        && row <= r1
                        && col >= c0
                        && col <= c1
                    {
                        Self::push_selection(
                            &mut rects,
                            base_x,
                            base_y,
                            base_x + cell_w as f32,
                            base_y + cell_h as f32,
                        );
                    }

                    if cell.is_blank() || cell.wide_continuation {
                        continue;
                    }

                    let fg = ensure_contrast(cell.fg, cell.bg);
                    let mut pen_x = base_x;
                    for ch in cell.text.chars() {
                        let drawn = Self::push_glyph(
                            glyphs,
                            &mut self.atlas,
                            &mut glyph_vertices,
                            Some(&mut rects),
                            ch,
                            pen_x,
                            base_y,
                            baseline,
                            Some(cell_h),
                            cell.italic,
                            cell.bold,
                            fg,
                            &context.queue,
                        );
                        if !drawn && !ATLAS_INSERT_WARNED.swap(true, Ordering::Relaxed) {
                            log::warn!(
                                "term: glyph atlas overflowed; drew fallbacks. Increase TERM_ATLAS_SIZE or reduce glyph variety."
                            );
                        }
                        pen_x += glyphs.advance_width(ch) as f32;
                    }

                    if cell.underline {
                        let line_y = base_y + cell_h as f32 - 2.0;
                        Self::push_rect(
                            &mut rects,
                            base_x,
                            line_y,
                            base_x + cell_w as f32,
                            line_y + 1.0,
                            fg,
                        );
                    }
                }
            }

            // Cursor overlay
            if term.view_offset == 0 && term.cursor_row < term.rows && term.cursor_col < term.cols {
                let mut cursor_col = term.cursor_col;
                let cursor_row = term.cursor_row;
                let mut cell = term.display_cell(cursor_col, cursor_row);
                if cell.wide_continuation && cursor_col > 0 {
                    cursor_col -= 1;
                    cell = term.display_cell(cursor_col, cursor_row);
                }
                let cursor_selected = selected
                    .map(|(c0, r0, c1, r1)| {
                        cursor_row >= r0 && cursor_row <= r1 && cursor_col >= c0 && cursor_col <= c1
                    })
                    .unwrap_or(false);

                let span = if cell.wide { 2u32 } else { 1u32 };
                let cursor_x = origin_x.saturating_add(cursor_col as u32 * cell_w);
                let cursor_h = cell_h.max(1).min(2) as f32;
                let row_base_y = origin_y as f32 + cursor_row as f32 * cell_h as f32;
                let cursor_y = (row_base_y + cell_h as f32 - cursor_h).max(0.0);
                if !cursor_selected {
                    Self::push_rect(
                        &mut rects,
                        cursor_x as f32,
                        cursor_y,
                        (cursor_x + span * cell_w) as f32,
                        (cursor_y + cursor_h as f32) as f32,
                        cursor_color,
                    );
                }
                // Overlay draws underline only; glyphs remain from the main pass to avoid extra
                // atlas inserts and clipping differences.
            }

            // Overlays appended after grid data
            overlay(
                &mut rects,
                &mut glyph_vertices,
                &mut self.atlas,
                &context.queue,
                glyphs,
            );

            if self.atlas.take_cleared() {
                attempts += 1;
                if attempts < 3 {
                    continue;
                }
            }
            break;
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
            context
                .queue
                .write_buffer(&self.rect_vertex_buffer, 0, bytemuck::cast_slice(&rects));
        }
        if !glyph_vertices.is_empty() {
            context.queue.write_buffer(
                &self.glyph_vertex_buffer,
                0,
                bytemuck::cast_slice(&glyph_vertices),
            );
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("term_gpu_grid"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: base_bg.r as f64 / 255.0,
                        g: base_bg.g as f64 / 255.0,
                        b: base_bg.b as f64 / 255.0,
                        a: base_bg.a as f64 / 255.0,
                    }),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        if !rects.is_empty() {
            pass.set_pipeline(&self.rect_pipeline);
            pass.set_bind_group(0, &self.screen_bind_group, &[]);
            pass.set_vertex_buffer(0, self.rect_vertex_buffer.slice(..));
            pass.draw(0..rects.len() as u32, 0..1);
        }

        if !glyph_vertices.is_empty() {
            pass.set_pipeline(&self.glyph_pipeline);
            pass.set_bind_group(0, &self.screen_bind_group, &[]);
            pass.set_bind_group(1, &self.glyph_bind_group, &[]);
            pass.set_vertex_buffer(0, self.glyph_vertex_buffer.slice(..));
            pass.draw(0..glyph_vertices.len() as u32, 0..1);
        }
    }
}
