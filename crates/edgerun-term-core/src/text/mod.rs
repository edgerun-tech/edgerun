use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use fdsm::bezier::scanline::FillRule;
use fdsm::generate::generate_msdf;
use fdsm::render::correct_sign_msdf;
use fdsm::shape::Shape;
use fdsm::transform::Transform;
use image::RgbImage;
use nalgebra::{Affine2, Matrix3};
use rustybuzz::{UnicodeBuffer, shape};
use swash::scale::image::Content;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::{FontRef, GlyphId};
use ttf_parser::Face;

use crate::font::load_fallback_fonts;

#[derive(Clone, Debug)]
pub struct GlyphMetrics {
    pub width: u32,
    pub height: u32,
    pub xmin: i32,
    pub ymin: i32,
    pub advance_width: f32,
}

#[derive(Clone, Debug)]
pub struct GlyphBitmap {
    pub metrics: GlyphMetrics,
    pub data: Vec<u8>, // RGBA8
    pub color: bool,
}

#[derive(Clone, Debug)]
pub struct ShapedGlyph {
    pub font_idx: usize,
    pub glyph_id: u16,
    pub x_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
}

pub struct GlyphCache {
    fonts: Vec<Arc<Vec<u8>>>,
    size: f32,
    cell_w: u32,
    cell_h: u32,
    baseline: i32,
    baseline_offset: i32,
    char_cache: HashMap<char, GlyphBitmap>,
    glyph_cache: HashMap<(usize, u16), GlyphBitmap>,
    ctx: ScaleContext,
    use_sdf: bool,
    msdf_smoothing: f32,
    msdf_antialiasing: bool,
}

pub(crate) const MSDF_SPREAD: f32 = 4.0;
pub const MSDF_DEFAULT_MIN_WIDTH: f32 = 0.01;
pub(crate) const MSDF_AA_BOOST: f32 = 0.1;

impl GlyphCache {
    pub fn new(primary: Arc<Vec<u8>>, size: f32) -> Self {
        let baseline_offset = env::var("TERM_BASELINE_OFFSET")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        let mut cache = Self {
            fonts: vec![primary],
            size,
            cell_w: 1,
            cell_h: 1,
            baseline: 0,
            baseline_offset,
            char_cache: HashMap::new(),
            glyph_cache: HashMap::new(),
            ctx: ScaleContext::new(),
            use_sdf: true,
            msdf_smoothing: MSDF_DEFAULT_MIN_WIDTH,
            msdf_antialiasing: true,
        };
        cache.recompute_vertical_metrics();
        cache
    }

    pub fn use_sdf(&self) -> bool {
        self.use_sdf
    }

    pub fn msdf_min_width(&self) -> f32 {
        let boost = if self.msdf_antialiasing {
            MSDF_AA_BOOST
        } else {
            0.0
        };
        (self.msdf_smoothing + boost).max(0.001)
    }

    pub fn set_msdf_smoothing(&mut self, value: f32) {
        self.msdf_smoothing = value.max(0.001);
    }

    pub fn set_msdf_antialiasing(&mut self, enabled: bool) {
        self.msdf_antialiasing = enabled;
    }

    pub fn set_use_sdf(&mut self, enabled: bool) {
        if self.use_sdf == enabled {
            return;
        }
        self.use_sdf = enabled;
        // clear cached glyph bitmaps so we regenerate with/without SDF
        self.char_cache.clear();
        self.glyph_cache.clear();
    }

    pub fn add_fonts(&mut self, fonts: Vec<Arc<Vec<u8>>>) {
        if fonts.is_empty() {
            return;
        }
        self.fonts.extend(fonts);
        self.recompute_vertical_metrics();
        self.char_cache.clear();
        self.glyph_cache.clear();
    }

    pub fn set_primary_font(&mut self, font: Arc<Vec<u8>>) {
        if self.fonts.is_empty() {
            self.fonts.push(font);
        } else {
            self.fonts[0] = font;
        }
        self.recompute_vertical_metrics();
        self.char_cache.clear();
        self.glyph_cache.clear();
    }

    fn recompute_vertical_metrics(&mut self) {
        // Derive cell metrics from the primary font only so large fallback fonts
        // (e.g., emoji) don't explode the terminal row height.
        let (cell_h, baseline) = if let Some(font) = FontRef::from_index(&self.fonts[0], 0) {
            let metrics = font.metrics(&[]).scale(self.size);
            let ascent = metrics.ascent.max(1.0);
            let descent = metrics.descent.abs().max(0.0);
            let leading = metrics.leading.max(0.0);
            let min_height = (self.size * 1.1).ceil().max(1.0);
            let natural = (ascent + descent + leading).ceil().max(1.0) as u32;
            let height = natural.max(min_height as u32);
            let padded = height;
            let baseline = height as i32 - descent.ceil() as i32 + self.baseline_offset;
            (padded, baseline)
        } else {
            (1u32, 1i32)
        };

        // Approximate cell width from "M" advance in primary font.
        let cell_w = if let Some(font) = FontRef::from_index(&self.fonts[0], 0) {
            let id = font.charmap().map('M' as u32);
            if id != 0 {
                self.metrics_for_font_glyph(0, id)
                    .map(|m| m.advance_width.round().max(1.0) as u32)
                    .unwrap_or(1)
            } else {
                1
            }
        } else {
            1
        };

        self.cell_w = cell_w;
        self.cell_h = cell_h;
        self.baseline = baseline.max(1);
    }

    pub fn baseline(&self) -> i32 {
        self.baseline
    }

    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn set_size(&mut self, size: f32) {
        if (self.size - size).abs() < f32::EPSILON {
            return;
        }
        self.size = size;
        self.recompute_vertical_metrics();
        self.char_cache.clear();
        self.glyph_cache.clear();
    }

    pub fn cell_size(&self) -> (u32, u32) {
        (self.cell_w, self.cell_h)
    }

    pub fn cell_height(&self) -> u32 {
        self.cell_h
    }

    pub fn advance_width(&mut self, ch: char) -> i32 {
        let (metrics, _, _) = self.rasterize(ch);
        metrics.advance_width.round().max(0.0) as i32
    }

    pub fn advance_width_f32(&mut self, ch: char) -> f32 {
        let (metrics, _, _) = self.rasterize(ch);
        metrics.advance_width
    }

    fn metrics_for_font_glyph(&mut self, font_idx: usize, glyph: GlyphId) -> Option<GlyphMetrics> {
        let font = FontRef::from_index(&self.fonts[font_idx], 0)?;
        let metrics = font.glyph_metrics(&[]).scale(self.size);
        Some(GlyphMetrics {
            width: 0,
            height: 0,
            xmin: 0,
            ymin: 0,
            advance_width: metrics.advance_width(glyph),
        })
    }

    fn rasterize_glyph(&mut self, font_idx: usize, glyph: GlyphId) -> Option<GlyphBitmap> {
        let font = FontRef::from_index(&self.fonts[font_idx], 0)?;
        let mut scaler = self.ctx.builder(font).size(self.size).hint(true).build();

        // Try color bitmap, then outline, then alpha bitmap.
        let sources = [
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::ColorOutline(0),
            Source::Bitmap(StrikeWith::BestFit),
            Source::Outline,
        ];

        let render = Render::new(&sources);
        let mut image = swash::scale::image::Image::new();
        if !render.render_into(&mut scaler, glyph, &mut image) {
            return None;
        }

        let glyph_metrics = font.glyph_metrics(&[]).scale(self.size);
        let is_color = matches!(image.content, Content::Color);
        let data = if is_color {
            image.data.clone()
        } else {
            image
                .data
                .chunks_exact(1)
                .flat_map(|a| [255u8, 255u8, 255u8, a[0]])
                .collect()
        };

        // Optionally generate an SDF for monochrome glyphs to improve scaling and reduce
        // repeated uploads. We'll produce an SDF in the alpha channel and keep RGB white.
        if !is_color && self.use_sdf {
            if let Some(msdf) = Self::rasterize_msdf(&self.fonts[font_idx], glyph, self.size) {
                return Some(msdf);
            }
        }

        Some(GlyphBitmap {
            metrics: GlyphMetrics {
                width: image.placement.width as u32,
                height: image.placement.height as u32,
                xmin: image.placement.left,
                ymin: image.placement.top,
                advance_width: glyph_metrics.advance_width(glyph),
            },
            data,
            color: is_color,
        })
    }

    fn rasterize_msdf(font_bytes: &[u8], glyph: GlyphId, size: f32) -> Option<GlyphBitmap> {
        let face = Face::parse(font_bytes, 0).ok()?;
        let ttf_glyph = ttf_parser::GlyphId(glyph);
        let bbox = face.glyph_bounding_box(ttf_glyph)?;
        let upem = face.units_per_em();
        if upem == 0 {
            return None;
        }

        let scale = size as f64 / upem as f64;
        let range = MSDF_SPREAD as f64;
        let left = (bbox.x_min as f64 * scale - range).floor();
        let right = (bbox.x_max as f64 * scale + range).ceil();
        let top = (bbox.y_max as f64 * scale + range).ceil();
        let bottom = (bbox.y_min as f64 * scale - range).floor();
        let width = (right - left).max(1.0) as u32;
        let height = (top - bottom).max(1.0) as u32;

        let mut shape = fdsm_ttf_parser::load_shape_from_face(&face, ttf_glyph)?;
        let transform = Affine2::from_matrix_unchecked(Matrix3::new(
            scale, 0.0, -left, 0.0, -scale, top, 0.0, 0.0, 1.0,
        ));
        shape.transform(&transform);

        let colored = Shape::edge_coloring_simple(shape, 0.03, glyph as u64);
        let prepared = colored.prepare();
        let mut msdf = RgbImage::new(width, height);
        generate_msdf(&prepared, range, &mut msdf);
        correct_sign_msdf(&mut msdf, &prepared, FillRule::Nonzero);

        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for pixel in msdf.pixels() {
            out.push(pixel[0]);
            out.push(pixel[1]);
            out.push(pixel[2]);
            out.push(255u8);
        }

        let scale_f = size / upem as f32;
        let advance = face
            .glyph_hor_advance(ttf_glyph)
            .unwrap_or((bbox.x_max - bbox.x_min) as u16) as f32
            * scale_f;
        let xmin = left.floor() as i32;
        let ymin = top.ceil() as i32;

        Some(GlyphBitmap {
            metrics: GlyphMetrics {
                width,
                height,
                xmin,
                ymin,
                advance_width: advance,
            },
            data: out,
            color: false,
        })
    }

    fn find_glyph(&self, ch: char) -> Option<(usize, GlyphId)> {
        for (i, font_data) in self.fonts.iter().enumerate() {
            if let Some(font) = FontRef::from_index(font_data, 0) {
                let id = font.charmap().map(ch as u32);
                if id != 0 {
                    return Some((i, id));
                }
            }
        }
        None
    }

    pub fn rasterize(&mut self, ch: char) -> (GlyphMetrics, &[u8], bool) {
        if !self.char_cache.contains_key(&ch) {
            if let Some((font_idx, glyph)) = self.find_glyph(ch) {
                if let Some(bitmap) = self.rasterize_glyph(font_idx, glyph) {
                    self.char_cache.insert(ch, bitmap);
                }
            }
            if !self.char_cache.contains_key(&ch) {
                // Fallback: space
                self.char_cache.insert(
                    ch,
                    GlyphBitmap {
                        metrics: GlyphMetrics {
                            width: 0,
                            height: 0,
                            xmin: 0,
                            ymin: 0,
                            advance_width: self.cell_w as f32,
                        },
                        data: Vec::new(),
                        color: false,
                    },
                );
            }
        }

        let entry = self.char_cache.get(&ch).unwrap();
        (entry.metrics.clone(), entry.data.as_slice(), entry.color)
    }

    pub fn rasterize_indexed_in_font(
        &mut self,
        font_idx: usize,
        glyph_index: usize,
    ) -> (GlyphMetrics, &[u8], bool) {
        let key = (font_idx, glyph_index.min(u16::MAX as usize) as u16);
        if !self.glyph_cache.contains_key(&key) {
            let glyph = key.1;
            if let Some(bitmap) = self.rasterize_glyph(font_idx, glyph) {
                self.glyph_cache.insert(key, bitmap);
            } else {
                self.glyph_cache.insert(
                    key,
                    GlyphBitmap {
                        metrics: GlyphMetrics {
                            width: 0,
                            height: 0,
                            xmin: 0,
                            ymin: 0,
                            advance_width: 0.0,
                        },
                        data: Vec::new(),
                        color: false,
                    },
                );
            }
        }
        let entry = self.glyph_cache.get(&key).unwrap();
        (entry.metrics.clone(), entry.data.as_slice(), entry.color)
    }

    /// Shape text with HarfBuzz (via rustybuzz) using the primary font.
    /// Returns positioned glyphs in pixel units and font indices.
    pub fn shape_text(&self, text: &str) -> Option<Vec<ShapedGlyph>> {
        if text.is_empty() {
            return Some(Vec::new());
        }
        if text.is_ascii() {
            return None;
        }

        let mut out = Vec::new();
        let mut run = String::new();
        let mut run_font: Option<usize> = None;

        let flush = |run: &mut String, font_idx: usize, out: &mut Vec<ShapedGlyph>| -> bool {
            if run.is_empty() {
                return true;
            }
            let font_bytes = match self.fonts.get(font_idx) {
                Some(f) => f,
                None => return false,
            };
            let mut face = match rustybuzz::Face::from_slice(font_bytes, 0) {
                Some(f) => f,
                None => return false,
            };
            let upem = face.units_per_em() as f32;
            if upem == 0.0 {
                return false;
            }
            let scale = self.size / upem;
            let to_px = |val: i32| val as f32 * scale;

            let mut buffer = UnicodeBuffer::new();
            buffer.push_str(run);
            let shaped = shape(&mut face, &[], buffer);
            for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                out.push(ShapedGlyph {
                    font_idx,
                    glyph_id: info.glyph_id as u16,
                    x_advance: to_px(pos.x_advance),
                    x_offset: to_px(pos.x_offset),
                    y_offset: to_px(pos.y_offset),
                });
            }
            run.clear();
            true
        };

        for ch in text.chars() {
            let target_font = if let Some((idx, _)) = self.find_glyph(ch) {
                idx
            } else {
                // Skip missing glyphs entirely.
                continue;
            };

            match run_font {
                Some(font_idx) if font_idx == target_font => {
                    run.push(ch);
                }
                Some(font_idx) => {
                    if !flush(&mut run, font_idx, &mut out) {
                        return None;
                    }
                    run_font = Some(target_font);
                    run.push(ch);
                }
                None => {
                    run_font = Some(target_font);
                    run.push(ch);
                }
            }
        }

        if let Some(font_idx) = run_font {
            if !flush(&mut run, font_idx, &mut out) {
                return None;
            }
        }

        Some(out)
    }

    pub fn load_fallback_fonts() -> Vec<Arc<Vec<u8>>> {
        load_fallback_fonts()
    }
}
