#![no_std]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Default)]
pub struct FontSettings;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Metrics {
    pub xmin: i32,
    pub ymin: i32,
    pub width: usize,
    pub height: usize,
    pub advance_width: f32,
    pub advance_height: f32,
    pub bounds: OutlineBounds,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OutlineBounds {
    pub xmin: f32,
    pub ymin: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug)]
pub struct Font {
    seed: u32,
}

impl Font {
    pub fn from_bytes<B: AsRef<[u8]>>(
        bytes: B,
        _settings: FontSettings,
    ) -> Result<Self, &'static str> {
        let bytes = bytes.as_ref();
        if bytes.is_empty() {
            return Err("empty font bytes");
        }
        Ok(Self {
            seed: stable_seed(bytes),
        })
    }

    pub fn metrics(&self, ch: char, px: f32) -> Metrics {
        glyph_metrics(ch, px)
    }

    pub fn rasterize(&self, ch: char, px: f32) -> (Metrics, Vec<u8>) {
        let metrics = self.metrics(ch, px);
        if metrics.width == 0 || metrics.height == 0 {
            return (metrics, Vec::new());
        }

        let mut bitmap = vec![0u8; metrics.width * metrics.height];
        let code = ch as u32 ^ self.seed;
        let stroke = (round_f32(px / 13.0) as usize).clamp(1, 3);
        let width = metrics.width;
        let height = metrics.height;

        for y in 0..height {
            for x in 0..width {
                let border = x < stroke
                    || y < stroke
                    || width.saturating_sub(x + 1) < stroke
                    || height.saturating_sub(y + 1) < stroke;
                let diag_a = ((x + y + code as usize) % 11) < stroke;
                let diag_b = ((x + height.saturating_sub(y) + (code >> 5) as usize) % 13) < stroke;
                let mid = (code & 1) == 1 && y.abs_diff(height / 2) < stroke;
                let vertical = (code & 2) == 2 && x.abs_diff(width / 2) < stroke;
                let coverage = if border || diag_a || diag_b || mid || vertical {
                    edge_coverage(x, y, width, height)
                } else {
                    0
                };
                bitmap[y * width + x] = coverage;
            }
        }

        (metrics, bitmap)
    }
}

fn glyph_metrics(ch: char, px: f32) -> Metrics {
    if ch.is_whitespace() {
        return Metrics {
            advance_width: whitespace_advance(ch, px),
            advance_height: px,
            ..Metrics::default()
        };
    }

    let width_factor = match ch {
        'i' | 'l' | '!' | '|' | ':' | ';' | '\'' | '`' => 0.32,
        'm' | 'w' | 'M' | 'W' | '@' | '#' | '%' => 0.86,
        '0'..='9' => 0.58,
        _ if ch.is_ascii_uppercase() => 0.66,
        _ => 0.56,
    };
    let width = round_f32(px * width_factor).max(1.0) as usize;
    let height = round_f32(px * 0.82).max(1.0) as usize;
    let advance_width = (width as f32 + px * 0.12).max(px * 0.32);

    Metrics {
        xmin: 0,
        ymin: 0,
        width,
        height,
        advance_width,
        advance_height: px,
        bounds: OutlineBounds {
            xmin: 0.0,
            ymin: 0.0,
            width: width as f32,
            height: height as f32,
        },
    }
}

fn whitespace_advance(ch: char, px: f32) -> f32 {
    match ch {
        '\t' => px * 1.28,
        _ => px * 0.32,
    }
}

fn edge_coverage(x: usize, y: usize, width: usize, height: usize) -> u8 {
    let edge = x
        .min(y)
        .min(width.saturating_sub(x + 1))
        .min(height.saturating_sub(y + 1));
    match edge {
        0 => 180,
        1 => 230,
        _ => 255,
    }
}

fn stable_seed(bytes: &[u8]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for &byte in bytes.iter().take(4096) {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn round_f32(value: f32) -> f32 {
    if value.is_sign_negative() {
        (value - 0.5) as i32 as f32
    } else {
        (value + 0.5) as i32 as f32
    }
}
