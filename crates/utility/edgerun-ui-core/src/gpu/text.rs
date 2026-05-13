use super::{Color4, GpuScene};
use std::collections::HashMap;
use std::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextQuad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub color: Color4,
}

#[derive(Clone, Copy, Debug)]
struct AtlasGlyph {
    uv: [f32; 4],
    size: [f32; 2],
    bearing: [f32; 2],
    advance: f32,
}

#[derive(Clone, Debug)]
pub struct FontAtlas {
    pub width: u32,
    pub height: u32,
    pub alpha: Vec<u8>,
    glyphs: HashMap<char, AtlasGlyph>,
    px: f32,
}

impl FontAtlas {
    pub fn from_font_bytes(bytes: &[u8], px: f32) -> Result<Self, String> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|_| "font parse failed".to_string())?;
        Ok(Self::build(&font, &ascii_chars(), px))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_inter(px: f32) -> Result<Self, String> {
        let path = crate::font::find_best_ui_font().ok_or_else(|| {
            "Inter font not found; set EDGE_UI_FONT=/path/to/Inter.ttf".to_string()
        })?;
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        Self::from_font_bytes(&bytes, px)
    }

    fn build(font: &fontdue::Font, chars: &[char], px: f32) -> Self {
        let width = 1024u32;
        let height = 1024u32;
        let mut alpha = vec![0u8; (width * height) as usize];
        let mut glyphs = HashMap::new();
        let mut x = 2u32;
        let mut y = 2u32;
        let mut row_h = 0u32;

        for &ch in chars {
            let (metrics, bitmap) = font.rasterize(ch, px);
            if metrics.width == 0 || metrics.height == 0 {
                glyphs.insert(
                    ch,
                    AtlasGlyph {
                        uv: [0.0; 4],
                        size: [0.0, 0.0],
                        bearing: [metrics.xmin as f32, metrics.ymin as f32],
                        advance: metrics.advance_width,
                    },
                );
                continue;
            }

            let gw = metrics.width as u32;
            let gh = metrics.height as u32;
            if x + gw + 2 >= width {
                x = 2;
                y += row_h + 2;
                row_h = 0;
            }
            if y + gh + 2 >= height {
                break;
            }

            for gy in 0..gh {
                for gx in 0..gw {
                    alpha[((y + gy) * width + x + gx) as usize] = bitmap[(gy * gw + gx) as usize];
                }
            }

            glyphs.insert(
                ch,
                AtlasGlyph {
                    uv: [
                        x as f32 / width as f32,
                        y as f32 / height as f32,
                        (x + gw) as f32 / width as f32,
                        (y + gh) as f32 / height as f32,
                    ],
                    size: [gw as f32, gh as f32],
                    bearing: [metrics.xmin as f32, metrics.ymin as f32],
                    advance: metrics.advance_width,
                },
            );
            x += gw + 2;
            row_h = row_h.max(gh);
        }

        Self {
            width,
            height,
            alpha,
            glyphs,
            px,
        }
    }

    pub(super) fn layout_text(
        &self,
        scene: &mut GpuScene,
        mut x: f32,
        y: f32,
        text: &str,
        color: Color4,
    ) {
        let baseline = y + self.px * 0.82;
        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }
            let Some(glyph) = self
                .glyphs
                .get(&ch)
                .or_else(|| self.glyphs.get(&ch.to_ascii_uppercase()))
            else {
                x += self.px * 0.32;
                continue;
            };
            if glyph.size[0] > 0.0 && glyph.size[1] > 0.0 {
                scene.push_text_quad(TextQuad {
                    x: x + glyph.bearing[0],
                    y: baseline - glyph.bearing[1] - glyph.size[1],
                    w: glyph.size[0],
                    h: glyph.size[1],
                    u0: glyph.uv[0],
                    v0: glyph.uv[1],
                    u1: glyph.uv[2],
                    v1: glyph.uv[3],
                    color,
                });
            }
            x += glyph.advance.max(self.px * 0.28);
        }
    }

    pub fn text_width(&self, text: &str) -> f32 {
        text.chars()
            .map(|ch| {
                self.glyphs
                    .get(&ch)
                    .or_else(|| self.glyphs.get(&ch.to_ascii_uppercase()))
                    .map(|glyph| glyph.advance.max(self.px * 0.28))
                    .unwrap_or(self.px * 0.32)
            })
            .sum()
    }
}

fn ascii_chars() -> Vec<char> {
    (32u8..=126u8).map(char::from).collect()
}
