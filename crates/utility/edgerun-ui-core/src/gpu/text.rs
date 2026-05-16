use super::font_renderer::{Font, FontSettings, VariationSetting};
use super::{Color4, GpuScene};
use std::collections::HashMap;
use std::vec::Vec;

const MISSING_GLYPH: char = '\u{fffd}';
pub const INTER_FONT_BYTES: &[u8] = include_bytes!("../../assets/Inter.ttc");
pub const GEIST_VARIABLE_FONT_BYTES: &[u8] = include_bytes!("../../assets/Geist-Variable.ttf");

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
    px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct GlyphKey {
    ch: char,
    px: u16,
}

#[derive(Clone, Debug)]
pub struct FontAtlas {
    pub width: u32,
    pub height: u32,
    pub alpha: Vec<u8>,
    glyphs: HashMap<GlyphKey, AtlasGlyph>,
    px: f32,
    raster_px: u16,
    ascent: f32,
    descent: f32,
    line_height: f32,
    skipped_glyphs: usize,
}

impl FontAtlas {
    pub fn from_font_bytes(bytes: &[u8], px: f32) -> Result<Self, String> {
        Self::from_font_bytes_for_raster_px(bytes, px, px)
    }

    pub fn from_font_bytes_for_raster_px(
        bytes: &[u8],
        css_px: f32,
        raster_px: f32,
    ) -> Result<Self, String> {
        let font =
            renderer_font_from_bytes(bytes, FontSettings::default()).map_err(str::to_string)?;
        Ok(Self::build(&font, &ui_chars(), css_px, raster_px))
    }

    pub fn from_font_bytes_with_variations(
        bytes: &[u8],
        px: f32,
        variations: &[VariationSetting],
    ) -> Result<Self, String> {
        Self::from_font_bytes_with_variations_for_raster_px(bytes, px, px, variations)
    }

    pub fn from_font_bytes_with_variations_for_raster_px(
        bytes: &[u8],
        css_px: f32,
        raster_px: f32,
        variations: &[VariationSetting],
    ) -> Result<Self, String> {
        let font = renderer_font_from_bytes(bytes, FontSettings::with_variations(variations))
            .map_err(str::to_string)?;
        Ok(Self::build(&font, &ui_chars(), css_px, raster_px))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_inter(px: f32) -> Result<Self, String> {
        Self::from_font_bytes(INTER_FONT_BYTES, px)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_geist(px: f32) -> Result<Self, String> {
        Self::from_font_bytes(GEIST_VARIABLE_FONT_BYTES, px)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_geist_for_device_scale(css_px: f32, device_scale: f32) -> Result<Self, String> {
        let raster_px = css_px * device_scale.clamp(1.0, 4.0);
        Self::from_font_bytes_for_raster_px(GEIST_VARIABLE_FONT_BYTES, css_px, raster_px)
    }

    fn build(font: &Font, chars: &[char], css_px: f32, raster_px: f32) -> Self {
        let (width, height) = (2048u32, 2048u32);
        let mut alpha = vec![0u8; (width * height) as usize];
        let mut glyphs = HashMap::new();
        let mut x = 2u32;
        let mut y = 2u32;
        let mut row_h = 0u32;
        let mut skipped_glyphs = 0usize;

        let raster_px = raster_px.clamp(1.0, 160.0);
        let glyph_px = raster_px.round().clamp(1.0, u16::MAX as f32) as u16;
        let vertical_metrics = font.vertical_metrics(raster_px);
        let css_scale = css_px / raster_px.max(1.0);
        for &ch in chars {
            let (metrics, bitmap) = font.rasterize(ch, raster_px);
            let key = GlyphKey { ch, px: glyph_px };
            if metrics.width == 0 || metrics.height == 0 {
                glyphs.insert(
                    key,
                    AtlasGlyph {
                        uv: [0.0; 4],
                        size: [0.0, 0.0],
                        bearing: [metrics.xmin as f32, metrics.ymin as f32],
                        advance: metrics.advance_width,
                        px: glyph_px as f32,
                    },
                );
                continue;
            }

            let gw = metrics.width as u32;
            let gh = metrics.height as u32;
            if x + gw + 4 >= width {
                x = 2;
                y += row_h + 4;
                row_h = 0;
            }
            if y + gh + 4 >= height {
                skipped_glyphs += 1;
                continue;
            }

            for gy in 0..gh {
                for gx in 0..gw {
                    alpha[((y + gy) * width + x + gx) as usize] = bitmap[(gy * gw + gx) as usize];
                }
            }

            glyphs.insert(
                key,
                AtlasGlyph {
                    uv: [
                        (x as f32 + 0.5) / width as f32,
                        (y as f32 + 0.5) / height as f32,
                        (x as f32 + gw as f32 - 0.5) / width as f32,
                        (y as f32 + gh as f32 - 0.5) / height as f32,
                    ],
                    size: [gw as f32, gh as f32],
                    bearing: [metrics.xmin as f32, metrics.ymin as f32],
                    advance: metrics.advance_width,
                    px: glyph_px as f32,
                },
            );
            x += gw + 4;
            row_h = row_h.max(gh);
        }

        Self {
            width,
            height,
            alpha,
            glyphs,
            px: css_px,
            raster_px: glyph_px,
            ascent: vertical_metrics.ascent * css_scale,
            descent: vertical_metrics.descent * css_scale,
            line_height: vertical_metrics.line_height * css_scale,
            skipped_glyphs,
        }
    }

    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    pub fn skipped_glyph_count(&self) -> usize {
        self.skipped_glyphs
    }

    pub fn atlas_overflowed(&self) -> bool {
        self.skipped_glyphs > 0
    }

    pub fn line_height(&self) -> f32 {
        self.line_height.max(self.ascent + self.descent).ceil()
    }

    pub fn line_height_scaled(&self, scale: f32) -> f32 {
        (self.line_height.max(self.ascent + self.descent) * scale / 2.0).ceil()
    }

    fn font_target_px(&self, scale: f32) -> f32 {
        (self.px * scale / 2.0).clamp(1.0, 160.0)
    }

    fn glyph_for_px(&self, ch: char, px: u16) -> Option<&AtlasGlyph> {
        self.glyphs
            .get(&GlyphKey { ch, px })
            .or_else(|| {
                ch.is_ascii()
                    .then(|| {
                        self.glyphs.get(&GlyphKey {
                            ch: ch.to_ascii_uppercase(),
                            px,
                        })
                    })
                    .flatten()
            })
            .or_else(|| {
                self.glyphs.get(&GlyphKey {
                    ch: MISSING_GLYPH,
                    px,
                })
            })
            .or_else(|| self.glyphs.get(&GlyphKey { ch: '?', px }))
    }

    fn advance_for_px(&self, ch: char, target_px: f32) -> f32 {
        self.glyph_for_px(ch, self.raster_px)
            .map(|glyph| glyph.advance.max(glyph.px * 0.28) * (target_px / glyph.px.max(1.0)))
            .unwrap_or(target_px * 0.5)
    }

    fn advance_for(&self, ch: char) -> f32 {
        self.advance_for_px(ch, self.px)
    }

    pub(super) fn layout_text(
        &self,
        scene: &mut GpuScene,
        x: f32,
        y: f32,
        text: &str,
        color: Color4,
    ) {
        self.layout_text_into(scene, x, y, text, color);
    }

    pub fn layout_text_quads(&self, x: f32, y: f32, text: &str, color: Color4) -> Vec<TextQuad> {
        self.layout_text_quads_scaled(x, y, text, 2.0, color)
    }

    pub fn layout_text_quads_scaled(
        &self,
        mut x: f32,
        y: f32,
        text: &str,
        scale: f32,
        color: Color4,
    ) -> Vec<TextQuad> {
        let mut quads = Vec::new();
        let origin_x = x;
        let target_px = self.font_target_px(scale);
        let metric_scale = target_px / self.px.max(1.0);
        let mut baseline = y + self.ascent * metric_scale;
        for ch in text.chars() {
            if ch == '\n' {
                x = origin_x;
                baseline += self.line_height * metric_scale;
                continue;
            }
            let Some(glyph) = self.glyph_for_px(ch, self.raster_px) else {
                x += target_px * 0.5;
                continue;
            };
            let scale = target_px / glyph.px.max(1.0);
            if glyph.size[0] > 0.0 && glyph.size[1] > 0.0 {
                quads.push(TextQuad {
                    x: (x + glyph.bearing[0] * scale).round(),
                    y: (baseline - glyph.bearing[1] * scale - glyph.size[1] * scale).round(),
                    w: glyph.size[0] * scale,
                    h: glyph.size[1] * scale,
                    u0: glyph.uv[0],
                    v0: glyph.uv[1],
                    u1: glyph.uv[2],
                    v1: glyph.uv[3],
                    color,
                });
            }
            x += glyph.advance.max(glyph.px * 0.28) * scale;
        }
        quads
    }

    pub fn visual_center_offset_scaled(&self, text: &str, scale: f32) -> Option<f32> {
        let quads =
            self.layout_text_quads_scaled(0.0, 0.0, text, scale, Color4::rgb_u8(255, 255, 255));
        let mut top = f32::INFINITY;
        let mut bottom = f32::NEG_INFINITY;
        for quad in quads {
            top = top.min(quad.y);
            bottom = bottom.max(quad.y + quad.h);
        }
        top.is_finite().then_some((top + bottom) * 0.5)
    }

    pub fn layout_text_into(
        &self,
        scene: &mut GpuScene,
        x: f32,
        y: f32,
        text: &str,
        color: Color4,
    ) {
        self.layout_text_scaled_into(scene, x, y, text, 2.0, color);
    }

    pub fn layout_text_scaled_into(
        &self,
        scene: &mut GpuScene,
        x: f32,
        y: f32,
        text: &str,
        scale: f32,
        color: Color4,
    ) {
        for quad in self.layout_text_quads_scaled(x, y, text, scale, color) {
            scene.push_text_quad(quad);
        }
    }

    pub fn text_width(&self, text: &str) -> f32 {
        self.text_width_scaled(text, 2.0)
    }

    pub fn text_width_scaled(&self, text: &str, scale: f32) -> f32 {
        let target_px = self.font_target_px(scale);
        let mut current = 0.0f32;
        let mut widest = 0.0f32;
        for ch in text.chars() {
            if ch == '\n' {
                widest = widest.max(current);
                current = 0.0;
                continue;
            }
            current += self.advance_for_px(ch, target_px);
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

fn renderer_font_from_bytes(bytes: &[u8], settings: FontSettings) -> Result<Font, &'static str> {
    Font::from_bytes(bytes, settings.clone()).or_else(|_| {
        first_ttc_face_bytes(bytes).and_then(|bytes| renderer_font_from_sfnt_bytes(bytes, settings))
    })
}

fn renderer_font_from_sfnt_bytes(
    bytes: Vec<u8>,
    settings: FontSettings,
) -> Result<Font, &'static str> {
    Font::from_bytes(bytes, settings).map_err(|_| "font parse failed")
}

fn first_ttc_face_bytes(bytes: &[u8]) -> Result<Vec<u8>, &'static str> {
    if bytes.get(0..4) != Some(b"ttcf") {
        return Err("font parse failed");
    }
    let count = read_u32(bytes, 8).ok_or("bad font collection")? as usize;
    if count == 0 {
        return Err("empty font collection");
    }
    let font_offset = read_u32(bytes, 12).ok_or("bad font collection offset")? as usize;
    let header_end = font_offset.checked_add(12).ok_or("bad font face offset")?;
    let header = bytes
        .get(font_offset..header_end)
        .ok_or("bad font collection face")?;
    let num_tables = read_u16(bytes, font_offset + 4).ok_or("bad sfnt table count")? as usize;
    let records_len = num_tables
        .checked_mul(16)
        .and_then(|len| len.checked_add(12))
        .ok_or("font table overflow")?;
    let records_end = font_offset
        .checked_add(records_len)
        .ok_or("bad sfnt table records")?;
    bytes
        .get(font_offset..records_end)
        .ok_or("bad sfnt table records")?;

    let mut out = vec![0u8; records_len];
    out[0..12].copy_from_slice(header);

    for table_index in 0..num_tables {
        let src_record = font_offset + 12 + table_index * 16;
        let dst_record = 12 + table_index * 16;
        out[dst_record..dst_record + 8].copy_from_slice(
            bytes
                .get(src_record..src_record + 8)
                .ok_or("bad sfnt table record")?,
        );

        let src_table = read_u32(bytes, src_record + 8).ok_or("bad sfnt table offset")? as usize;
        let table_len = read_u32(bytes, src_record + 12).ok_or("bad sfnt table length")? as usize;
        let table_end = src_table
            .checked_add(table_len)
            .ok_or("sfnt table outside collection")?;
        let table = bytes
            .get(src_table..table_end)
            .ok_or("sfnt table outside collection")?;

        while out.len() % 4 != 0 {
            out.push(0);
        }
        let dst_table = out.len();
        write_u32(&mut out[dst_record + 8..dst_record + 12], dst_table as u32);
        write_u32(&mut out[dst_record + 12..dst_record + 16], table_len as u32);
        out.extend_from_slice(table);
    }

    Ok(out)
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let bytes = bytes.get(offset..offset + 2)?;
    Some(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let bytes = bytes.get(offset..offset + 4)?;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn write_u32(dst: &mut [u8], value: u32) {
    dst.copy_from_slice(&value.to_be_bytes());
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
        let px = 16u16;
        for ch in ascii_chars() {
            glyphs.insert(
                GlyphKey { ch, px },
                AtlasGlyph {
                    uv: [0.0; 4],
                    size: [px as f32 * 0.5, px as f32 * 0.75],
                    bearing: [0.0, 0.0],
                    advance: px as f32 * 0.5,
                    px: px as f32,
                },
            );
        }
        FontAtlas {
            width: 1,
            height: 1,
            alpha: Vec::new(),
            glyphs,
            px: 16.0,
            raster_px: px,
            ascent: 12.0,
            descent: 4.0,
            line_height: 20.0,
            skipped_glyphs: 0,
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
    fn font_atlas_scaled_width_and_quads_preserve_hierarchy() {
        let atlas = test_atlas();
        let body_width = atlas.text_width_scaled("abc", 2.0);
        let display_width = atlas.text_width_scaled("abc", 4.0);
        let color = Color4::rgb_u8(255, 255, 255);
        let body_quad = atlas.layout_text_quads_scaled(0.0, 0.0, "a", 2.0, color)[0];
        let display_quad = atlas.layout_text_quads_scaled(0.0, 0.0, "a", 4.0, color)[0];

        assert_eq!(body_width, 24.0);
        assert_eq!(display_width, 48.0);
        assert_eq!(display_quad.w, body_quad.w * 2.0);
        assert_eq!(display_quad.h, body_quad.h * 2.0);
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
    fn test_atlas_reports_no_skipped_glyphs() {
        let atlas = test_atlas();

        assert_eq!(atlas.skipped_glyph_count(), 0);
        assert!(!atlas.atlas_overflowed());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn geist_font_atlas_loads_from_vendored_asset() {
        let atlas = FontAtlas::load_geist(18.0).expect("Geist font atlas");

        assert!(atlas.glyph_count() >= ui_chars().len().saturating_sub(1));
        assert_eq!(atlas.skipped_glyph_count(), 0);
        assert!(!atlas.atlas_overflowed());
        assert!(atlas.text_width("Geist 123") > 0.0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn geist_variable_font_asset_loads() {
        let atlas = FontAtlas::load_geist(16.0).expect("Geist variable font atlas");

        assert_eq!(atlas.skipped_glyph_count(), 0);
        assert!(!atlas.atlas_overflowed());
        assert!(atlas.glyph_count() >= ui_chars().len().saturating_sub(1));
        assert!(atlas.text_width("Variable Geist 123") > 0.0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn geist_font_atlas_applies_variable_weight_instance() {
        let default = FontAtlas::load_geist(24.0).expect("default Geist font atlas");
        let heavy = FontAtlas::from_font_bytes_with_variations(
            GEIST_VARIABLE_FONT_BYTES,
            24.0,
            &[VariationSetting {
                tag: *b"wght",
                value: 900.0,
            }],
        )
        .expect("heavy Geist font atlas");

        assert_eq!(default.width, heavy.width);
        assert_eq!(default.height, heavy.height);
        assert_ne!(default.alpha, heavy.alpha);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn geist_atlas_uses_texel_center_uvs_for_linear_sampling() {
        let atlas = FontAtlas::load_geist(16.0).expect("Geist font atlas");
        let glyph = atlas.glyph_for_px('E', 16).expect("E glyph");
        let half_texel = 0.5 / atlas.width as f32;

        assert!(glyph.uv[0] >= half_texel);
        assert!(glyph.uv[2] <= 1.0 - half_texel);
        assert_ne!((glyph.uv[0] * atlas.width as f32).fract(), 0.0);
        assert_ne!((glyph.uv[2] * atlas.width as f32).fract(), 0.0);
    }

    #[test]
    fn ui_chars_include_common_interface_symbols() {
        let chars = ui_chars();

        for ch in ['✓', '→', '•', '…', MISSING_GLYPH] {
            assert!(chars.contains(&ch), "missing {ch}");
        }
    }
}
