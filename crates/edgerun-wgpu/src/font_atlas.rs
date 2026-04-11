//! GPU font atlas: rasterizes TTF glyphs via fontdue, uploads to wgpu texture.
//!
//! The atlas is a single RGBA8 texture containing all needed glyphs at their
//! target font size. Each glyph is stored with padding to avoid bleeding.

use std::collections::HashMap;

use fontdue::{Font, FontSettings, Metrics};
use wgpu::{
    Device, Queue, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};

/// Atlas dimensions — power of 2, fits most Latin text.
const ATLAS_WIDTH: u32 = 512;
const ATLAS_HEIGHT: u32 = 512;

/// A single glyph in the atlas.
#[derive(Clone, Copy, Debug)]
pub struct GlyphEntry {
    /// X offset within atlas texture
    pub atlas_x: u32,
    /// Y offset within atlas texture
    pub atlas_y: u32,
    /// Width of the glyph bitmap
    pub width: u32,
    /// Height of the glyph bitmap
    pub height: u32,
    /// Baseline offset from top-left of atlas entry
    pub bearing_x: f32,
    pub bearing_y: f32,
    /// Advance width for next glyph
    pub advance: f32,
}

/// Holds a rasterized font atlas and glyph metrics.
pub struct FontAtlas {
    pub texture: Texture,
    pub view: TextureView,
    /// Font size this atlas was built for
    pub font_size: f32,
    /// Glyph lookup: char → entry
    pub glyphs: HashMap<char, GlyphEntry>,
    /// Atlas pixel data (for upload)
    pub pixels: Vec<u8>,
}

impl FontAtlas {
    /// Build a font atlas from TTF data at the given font size.
    /// Rasterizes all glyphs needed for the given text.
    pub fn new(
        device: &Device,
        font_data: &[u8],
        font_size: f32,
        text: &str,
    ) -> Self {
        let font = Font::from_bytes(font_data, FontSettings::default())
            .expect("Failed to parse font");

        // Collect unique glyphs
        let mut chars: Vec<char> = text.chars().collect();
        chars.sort();
        chars.dedup();

        // Rasterize all glyphs, compute layout
        let mut pixels = vec![0u8; (ATLAS_WIDTH * ATLAS_HEIGHT * 4) as usize];
        let mut glyphs = HashMap::new();

        let padding = 2u32;
        let mut x = padding;
        let mut y = padding;
        let mut max_row_h = 0u32;

        for ch in chars {
            let (metrics, bitmap) = font.rasterize(ch, font_size);
            let w = metrics.width as u32;
            let h = metrics.height as u32;

            if w == 0 || h == 0 {
                // Space-like glyph, still need metrics
                glyphs.insert(ch, GlyphEntry {
                    atlas_x: 0, atlas_y: 0, width: 0, height: 0,
                    bearing_x: metrics.xmin as f32, bearing_y: metrics.ymin as f32,
                    advance: metrics.advance_width,
                });
                continue;
            }

            // Wrap to next row if needed
            if x + w + padding > ATLAS_WIDTH {
                x = padding;
                y += max_row_h + padding;
                max_row_h = 0;
            }

            if y + h + padding > ATLAS_HEIGHT {
                panic!("Font atlas overflow: too many glyphs for {}x{} at {}px",
                    ATLAS_WIDTH, ATLAS_HEIGHT, font_size);
            }

            // Copy bitmap into atlas
            for row in 0..h {
                for col in 0..w {
                    let src_idx = (row * w + col) as usize;
                    let alpha = bitmap[src_idx];
                    let dst_x = x + col;
                    let dst_y = y + row;
                    let dst_idx = ((dst_y * ATLAS_WIDTH + dst_x) * 4) as usize;
                    // White text (color comes from shader uniform)
                    pixels[dst_idx] = 255;
                    pixels[dst_idx + 1] = 255;
                    pixels[dst_idx + 2] = 255;
                    pixels[dst_idx + 3] = alpha;
                }
            }

            glyphs.insert(ch, GlyphEntry {
                atlas_x: x,
                atlas_y: y,
                width: w,
                height: h,
                bearing_x: metrics.xmin as f32,
                bearing_y: metrics.ymin as f32,
                advance: metrics.advance_width,
            });

            x += w + padding;
            max_row_h = max_row_h.max(h);
        }

        // Upload to GPU
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("font-atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_WIDTH,
                height: ATLAS_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        Self {
            texture,
            view,
            font_size,
            glyphs,
            pixels,
        }
    }

    /// Upload the atlas pixels to the GPU.
    pub fn upload(&self, queue: &Queue) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ATLAS_WIDTH * 4),
                rows_per_image: Some(ATLAS_HEIGHT),
            },
            wgpu::Extent3d {
                width: ATLAS_WIDTH,
                height: ATLAS_HEIGHT,
                depth_or_array_layers: 1,
            },
        );
    }
}
