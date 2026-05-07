use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use edgerun_unicode::terminal_width_char;

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
    pub data: Vec<u8>,
    pub color: bool,
}

pub struct GlyphCache {
    size: f32,
    cell_w: u32,
    cell_h: u32,
    baseline: i32,
    baseline_offset: i32,
    char_cache: HashMap<char, GlyphBitmap>,
}

pub(crate) const MSDF_SPREAD: f32 = 4.0;
pub const MSDF_DEFAULT_MIN_WIDTH: f32 = 0.01;

impl GlyphCache {
    pub fn new(_primary: Arc<Vec<u8>>, size: f32) -> Self {
        let baseline_offset = env::var("TERM_BASELINE_OFFSET")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        let mut cache = Self {
            size,
            cell_w: 1,
            cell_h: 1,
            baseline: 0,
            baseline_offset,
            char_cache: HashMap::new(),
        };
        cache.recompute_vertical_metrics();
        cache
    }

    pub fn use_sdf(&self) -> bool {
        false
    }

    pub fn msdf_min_width(&self) -> f32 {
        MSDF_DEFAULT_MIN_WIDTH
    }

    pub fn set_msdf_smoothing(&mut self, _value: f32) {}

    pub fn set_msdf_antialiasing(&mut self, _enabled: bool) {}

    pub fn set_use_sdf(&mut self, _enabled: bool) {}

    pub fn add_fonts(&mut self, _fonts: Vec<Arc<Vec<u8>>>) {}

    pub fn set_primary_font(&mut self, _font: Arc<Vec<u8>>) {
        self.char_cache.clear();
    }

    fn recompute_vertical_metrics(&mut self) {
        self.cell_w = (self.size * 0.6).round().max(1.0) as u32;
        self.cell_h = (self.size * 1.2).round().max(1.0) as u32;
        self.baseline = (self.cell_h as f32 * 0.78).round().max(1.0) as i32 + self.baseline_offset;
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
    }

    pub fn cell_size(&self) -> (u32, u32) {
        (self.cell_w, self.cell_h)
    }

    pub fn cell_height(&self) -> u32 {
        self.cell_h
    }

    pub fn advance_width(&mut self, ch: char) -> i32 {
        self.advance_width_f32(ch).round().max(0.0) as i32
    }

    pub fn advance_width_f32(&mut self, ch: char) -> f32 {
        char_cell_width(ch) as f32 * self.cell_w as f32
    }

    pub fn rasterize(&mut self, ch: char) -> (GlyphMetrics, &[u8], bool) {
        if !self.char_cache.contains_key(&ch) {
            let bitmap = self.rasterize_cell_glyph(ch);
            self.char_cache.insert(ch, bitmap);
        }
        let entry = self.char_cache.get(&ch).unwrap();
        (entry.metrics.clone(), entry.data.as_slice(), entry.color)
    }

    pub fn emoji_prefix<'a>(&self, text: &'a str) -> Option<&'a str> {
        #[cfg(feature = "embedded-emoji")]
        {
            return edgerun_emoji::longest_match_prefix(text).map(|asset| asset.sequence);
        }
        #[cfg(not(feature = "embedded-emoji"))]
        {
            let _ = text;
            None
        }
    }

    pub fn rasterize_emoji_sequence(&self, sequence: &str) -> Option<GlyphBitmap> {
        #[cfg(feature = "embedded-emoji")]
        {
            return self.rasterize_embedded_emoji_sequence(sequence);
        }
        #[cfg(not(feature = "embedded-emoji"))]
        {
            let _ = sequence;
            None
        }
    }

    #[cfg(feature = "embedded-emoji")]
    fn rasterize_embedded_emoji_sequence(&self, sequence: &str) -> Option<GlyphBitmap> {
        let cells = emoji_cell_width(sequence);
        let width = self.cell_w.saturating_mul(cells as u32).max(1);
        let height = self.cell_h.max(1);
        let data = edgerun_emoji::rasterize_sequence(sequence, width, height)?;
        Some(GlyphBitmap {
            metrics: GlyphMetrics {
                width,
                height,
                xmin: 0,
                ymin: self.baseline,
                advance_width: width as f32,
            },
            data,
            color: true,
        })
    }

    fn rasterize_cell_glyph(&self, ch: char) -> GlyphBitmap {
        let cells = char_cell_width(ch);
        let advance_width = cells as f32 * self.cell_w as f32;
        #[cfg(feature = "embedded-emoji")]
        {
            if let Some(data) = edgerun_emoji::rasterize_char(
                ch,
                self.cell_w.saturating_mul(cells as u32).max(1),
                self.cell_h.max(1),
            ) {
                return GlyphBitmap {
                    metrics: GlyphMetrics {
                        width: self.cell_w.saturating_mul(cells as u32).max(1),
                        height: self.cell_h.max(1),
                        xmin: 0,
                        ymin: self.baseline,
                        advance_width,
                    },
                    data,
                    color: true,
                };
            }
        }
        if cells == 0 || ch.is_whitespace() {
            return GlyphBitmap {
                metrics: GlyphMetrics {
                    width: 0,
                    height: 0,
                    xmin: 0,
                    ymin: 0,
                    advance_width,
                },
                data: Vec::new(),
                color: false,
            };
        }

        let width = (self.cell_w.saturating_mul(cells as u32))
            .saturating_sub(1)
            .max(1);
        let height = self.cell_h.saturating_sub(2).max(1);
        let mut data = vec![0u8; width as usize * height as usize * 4];
        let seed = ch as u32;
        for y in 0..height {
            for x in 0..width {
                let edge = x == 0 || y == 0 || x + 1 == width || y + 1 == height;
                let stroke = ((x as u32 * 17 + y as u32 * 31 + seed) % 11) < 2;
                if edge || stroke {
                    let idx = (y as usize * width as usize + x as usize) * 4;
                    data[idx] = 255;
                    data[idx + 1] = 255;
                    data[idx + 2] = 255;
                    data[idx + 3] = 255;
                }
            }
        }

        GlyphBitmap {
            metrics: GlyphMetrics {
                width,
                height,
                xmin: 0,
                ymin: self.baseline,
                advance_width,
            },
            data,
            color: false,
        }
    }

    pub fn load_fallback_fonts() -> Vec<Arc<Vec<u8>>> {
        Vec::new()
    }
}

fn char_cell_width(ch: char) -> usize {
    terminal_width_char(ch).clamp(0, 2)
}

#[cfg(feature = "embedded-emoji")]
fn emoji_cell_width(sequence: &str) -> usize {
    sequence
        .chars()
        .map(terminal_width_char)
        .sum::<usize>()
        .clamp(1, 2)
}
