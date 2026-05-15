use super::{Color4, GpuScene};
use std::collections::HashMap;
use std::vec::Vec;

const MISSING_GLYPH: char = '\u{fffd}';

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
        Ok(Self::build(&font, &ui_chars(), px))
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

    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    pub fn line_height(&self) -> f32 {
        (self.px * 1.22).ceil()
    }

    fn glyph_for(&self, ch: char) -> Option<&AtlasGlyph> {
        self.glyphs
            .get(&ch)
            .or_else(|| {
                ch.is_ascii()
                    .then(|| self.glyphs.get(&ch.to_ascii_uppercase()))
                    .flatten()
            })
            .or_else(|| self.glyphs.get(&MISSING_GLYPH))
            .or_else(|| self.glyphs.get(&'?'))
    }

    fn advance_for(&self, ch: char) -> f32 {
        self.glyph_for(ch)
            .map(|glyph| glyph.advance.max(self.px * 0.28))
            .unwrap_or(self.px * 0.5)
    }

    pub(super) fn layout_text(
        &self,
        scene: &mut GpuScene,
        mut x: f32,
        y: f32,
        text: &str,
        color: Color4,
    ) {
        let origin_x = x;
        let mut baseline = y + self.px * 0.82;
        for ch in text.chars() {
            if ch == '\n' {
                x = origin_x;
                baseline += self.line_height();
                continue;
            }
            let Some(glyph) = self.glyph_for(ch) else {
                x += self.px * 0.5;
                continue;
            };
            if glyph.size[0] > 0.0 && glyph.size[1] > 0.0 {
                scene.push_text_quad(TextQuad {
                    x: (x + glyph.bearing[0]).round(),
                    y: (baseline - glyph.bearing[1] - glyph.size[1]).round(),
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
        let mut current = 0.0f32;
        let mut widest = 0.0f32;
        for ch in text.chars() {
            if ch == '\n' {
                widest = widest.max(current);
                current = 0.0;
                continue;
            }
            current += self.advance_for(ch);
        }
        widest.max(current)
    }

    pub fn wrap_lines(&self, text: &str, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let max_width = max_width.max(self.px * 0.5);
        for raw_line in text.split('\n') {
            if raw_line.is_empty() {
                lines.push(String::new());
                continue;
            }

            let mut current = String::new();
            for word in raw_line.split_whitespace() {
                if self.text_width(word) > max_width {
                    if !current.is_empty() {
                        lines.push(current);
                        current = String::new();
                    }
                    self.push_wrapped_token(word, max_width, &mut lines, &mut current);
                    continue;
                }

                let candidate = if current.is_empty() {
                    word.to_string()
                } else {
                    format!("{current} {word}")
                };
                if self.text_width(&candidate) <= max_width || current.is_empty() {
                    current = candidate;
                } else {
                    lines.push(current);
                    current = word.to_string();
                }
            }
            lines.push(current);
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        lines
    }

    pub fn wrapped_line_count(&self, text: &str, max_width: f32) -> usize {
        self.wrap_lines(text, max_width).len()
    }

    fn push_wrapped_token(
        &self,
        token: &str,
        max_width: f32,
        lines: &mut Vec<String>,
        current: &mut String,
    ) {
        let mut current_width = self.text_width(current);
        for ch in token.chars() {
            let advance = self.advance_for(ch);
            if !current.is_empty() && current_width + advance > max_width {
                lines.push(std::mem::take(current));
                current_width = 0.0;
            }
            current.push(ch);
            current_width += advance;
        }
    }
}

#[cfg(test)]
fn ascii_chars() -> Vec<char> {
    (32u8..=126u8).map(char::from).collect()
}

fn ui_chars() -> Vec<char> {
    let mut chars = Vec::new();
    push_range(&mut chars, 0x0020, 0x007e);
    push_range(&mut chars, 0x00a0, 0x017f);

    for ch in [
        MISSING_GLYPH,
        '\u{2010}',
        '\u{2011}',
        '\u{2012}',
        '\u{2013}',
        '\u{2014}',
        '\u{2018}',
        '\u{2019}',
        '\u{201c}',
        '\u{201d}',
        '\u{2022}',
        '\u{2026}',
        '\u{2039}',
        '\u{203a}',
        '\u{2044}',
        '\u{20ac}',
        '\u{2190}',
        '\u{2191}',
        '\u{2192}',
        '\u{2193}',
        '\u{21a9}',
        '\u{21aa}',
        '\u{2212}',
        '\u{2318}',
        '\u{232b}',
        '\u{23ce}',
        '\u{25a0}',
        '\u{25cf}',
        '\u{25d0}',
        '\u{25d1}',
        '\u{2605}',
        '\u{2713}',
        '\u{2715}',
        '\u{2717}',
        '\u{27a1}',
    ] {
        chars.push(ch);
    }

    chars.sort_unstable();
    chars.dedup();
    chars
}

fn push_range(chars: &mut Vec<char>, start: u32, end: u32) {
    for codepoint in start..=end {
        if let Some(ch) = char::from_u32(codepoint) {
            chars.push(ch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_atlas() -> FontAtlas {
        let mut glyphs = HashMap::new();
        for ch in ascii_chars() {
            glyphs.insert(
                ch,
                AtlasGlyph {
                    uv: [0.0; 4],
                    size: [8.0, 12.0],
                    bearing: [0.0, 0.0],
                    advance: 8.0,
                },
            );
        }
        FontAtlas {
            width: 1,
            height: 1,
            alpha: Vec::new(),
            glyphs,
            px: 16.0,
        }
    }

    #[test]
    fn font_atlas_wrap_lines_preserves_newlines() {
        let atlas = test_atlas();

        assert_eq!(
            atlas.wrap_lines("alpha beta\ngamma", 500.0),
            vec!["alpha beta", "gamma"]
        );
    }

    #[test]
    fn font_atlas_wrapped_line_count_uses_width() {
        let atlas = test_atlas();

        assert_eq!(atlas.wrapped_line_count("alpha beta gamma", 48.0), 3);
    }

    #[test]
    fn font_atlas_text_width_uses_widest_line() {
        let atlas = test_atlas();

        assert_eq!(atlas.text_width("ab\nabcdef"), 48.0);
    }

    #[test]
    fn font_atlas_wrap_lines_breaks_long_tokens() {
        let atlas = test_atlas();

        assert_eq!(atlas.wrap_lines("abcdef", 24.0), vec!["abc", "def"]);
    }

    #[test]
    fn font_atlas_uses_fallback_for_missing_glyphs() {
        let atlas = test_atlas();

        assert_eq!(atlas.text_width("✓"), atlas.text_width("?"));
    }

    #[test]
    fn ui_chars_include_common_interface_symbols() {
        let chars = ui_chars();

        for ch in ['✓', '→', '•', '…', MISSING_GLYPH] {
            assert!(chars.contains(&ch), "missing {ch}");
        }
    }
}
