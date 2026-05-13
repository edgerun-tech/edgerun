//! Optional real font rendering for std builds.
//!
//! This is deliberately behind the `fontdue-text` feature so the core renderer
//! can remain no_std and dependency-free for tiny/bare-metal builds.

#![cfg(feature = "fontdue-text")]

use std::fs;
use std::path::{Path, PathBuf};

use crate::{Color, Painter};

pub struct FontFace {
    font: fontdue::Font,
}

impl FontFace {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, &'static str> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|_| "font parse failed")?;
        Ok(Self { font })
    }

    pub fn load_best_ui_font() -> Result<Self, String> {
        let path = find_best_ui_font().ok_or_else(|| {
            "no usable UI font found; install inter-font or set EDGE_UI_FONT=/path/to/font.ttf"
                .to_string()
        })?;
        let bytes =
            fs::read(&path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        Self::from_bytes(bytes).map_err(|e| format!("failed to load {}: {e}", path.display()))
    }

    pub fn rasterize(&self, ch: char, px: f32) -> (fontdue::Metrics, Vec<u8>) {
        self.font.rasterize(ch, px)
    }

    pub fn advance_width(&self, ch: char, px: f32) -> f32 {
        self.font.metrics(ch, px).advance_width
    }
}

pub struct TextStyle<'a> {
    pub font: &'a FontFace,
    pub px: f32,
    pub color: Color,
}

impl<'a> Painter<'a> {
    pub fn text_font(&mut self, x: i32, y: i32, text: &str, style: TextStyle<'_>) {
        let mut pen_x = x as f32;
        let baseline = y as f32 + style.px * 0.84;

        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }
            if ch.is_whitespace() {
                pen_x += style.font.advance_width(ch, style.px).max(style.px * 0.32);
                continue;
            }

            let (metrics, bitmap) = style.font.rasterize(ch, style.px);
            let glyph_x = pen_x + metrics.xmin as f32;
            let glyph_y = baseline - metrics.ymin as f32 - metrics.height as f32;
            self.blit_alpha_bitmap(
                glyph_x.round() as i32,
                glyph_y.round() as i32,
                metrics.width as u32,
                metrics.height as u32,
                &bitmap,
                style.color,
            );
            pen_x += metrics.advance_width;
        }
    }

    pub fn measure_text_font(&self, text: &str, style: TextStyle<'_>) -> i32 {
        text.chars()
            .map(|ch| style.font.advance_width(ch, style.px))
            .sum::<f32>()
            .round()
            .max(0.0) as i32
    }

    fn blit_alpha_bitmap(&mut self, x: i32, y: i32, w: u32, h: u32, alpha: &[u8], color: Color) {
        if w == 0 || h == 0 || alpha.is_empty() {
            return;
        }
        for gy in 0..h {
            let dy = y + gy as i32;
            if dy < 0 || dy as u32 >= self.height {
                continue;
            }
            for gx in 0..w {
                let dx = x + gx as i32;
                if dx < 0 || dx as u32 >= self.width {
                    continue;
                }
                let src = gy as usize * w as usize + gx as usize;
                let Some(&coverage) = alpha.get(src) else {
                    continue;
                };
                if coverage == 0 {
                    continue;
                }
                let a = (color.a as u32 * coverage as u32 / 255) as u8;
                self.blend_text_pixel(dx as u32, dy as u32, color.with_alpha(a));
            }
        }
    }

    fn blend_text_pixel(&mut self, x: u32, y: u32, color: Color) {
        if color.a == 0
            || x >= self.width
            || y >= self.height
            || self.pitch < self.width.saturating_mul(4)
        {
            return;
        }
        let off = y as usize * self.pitch as usize + x as usize * 4;
        let Some(px) = self.pixels.get_mut(off..off + 4) else {
            return;
        };
        let a = color.a as u32;
        let inv = 255 - a;
        px[0] = ((color.b as u32 * a + px[0] as u32 * inv) / 255) as u8;
        px[1] = ((color.g as u32 * a + px[1] as u32 * inv) / 255) as u8;
        px[2] = ((color.r as u32 * a + px[2] as u32 * inv) / 255) as u8;
        px[3] = 0xff;
    }
}

pub fn find_best_ui_font() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("EDGE_UI_FONT") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    let candidates = [
        // Arch/CachyOS common paths.
        "/usr/share/fonts/Inter/Inter.ttc",
        "/usr/share/fonts/Inter/Inter-Regular.otf",
        "/usr/share/fonts/inter/Inter-Regular.otf",
        "/usr/share/fonts/inter/Inter-Regular.ttf",
        "/usr/share/fonts/TTF/Inter-Regular.ttf",
        "/usr/share/fonts/TTF/InterVariable.ttf",
        "/usr/share/fonts/OTF/Inter-Regular.otf",
        "/usr/share/fonts/TTF/JetBrainsMono-Regular.ttf",
        "/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf",
        "/usr/share/fonts/TTF/JetBrainsMonoNLNerdFont-Regular.ttf",
        "/usr/share/fonts/TTF/NotoSans-Regular.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];

    for path in candidates {
        let p = Path::new(path);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }

    None
}
