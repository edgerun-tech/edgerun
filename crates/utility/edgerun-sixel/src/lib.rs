//! Sixel decoder for Edgerun terminal adapters.

#[derive(Clone, Copy, Debug)]
pub struct DcsSettings {
    pub aspect_ratio: Option<u16>,
    pub zero_color: Option<u16>,
    pub grid_size: Option<u16>,
}

impl DcsSettings {
    pub const fn new(
        aspect_ratio: Option<u16>,
        zero_color: Option<u16>,
        grid_size: Option<u16>,
    ) -> Self {
        Self {
            aspect_ratio,
            zero_color,
            grid_size,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SixelError {
    Empty,
    InvalidRepeat,
    InvalidColor,
}

impl std::fmt::Display for SixelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("empty sixel payload"),
            Self::InvalidRepeat => f.write_str("invalid sixel repeat count"),
            Self::InvalidColor => f.write_str("invalid sixel color definition"),
        }
    }
}

impl std::error::Error for SixelError {}

#[derive(Clone, Debug)]
pub struct SixelImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl SixelImage {
    pub fn corrected_dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }
}

#[derive(Clone, Copy)]
struct Rgba([u8; 4]);

impl Default for Rgba {
    fn default() -> Self {
        Self([0, 0, 0, 255])
    }
}

pub fn sixel_decode_from_dcs(
    payload: &[u8],
    settings: DcsSettings,
) -> Result<SixelImage, SixelError> {
    if payload.is_empty() {
        return Err(SixelError::Empty);
    }

    let mut palette = default_palette();
    let mut color = settings.zero_color.unwrap_or(0).min(255) as usize;
    let mut cursor_x = 0usize;
    let mut cursor_y = 0usize;
    let mut max_x = 0usize;
    let mut max_y = settings.grid_size.unwrap_or(0) as usize;
    let mut pixels: Vec<Option<Rgba>> = Vec::new();
    let mut width = settings.grid_size.unwrap_or(0) as usize;
    let mut height = if width > 0 { 6 } else { 0 };
    let mut idx = 0usize;

    while idx < payload.len() {
        match payload[idx] {
            b'?'..=b'~' => {
                let bits = payload[idx] - b'?';
                paint_sixel(
                    &mut pixels,
                    &mut width,
                    &mut height,
                    cursor_x,
                    cursor_y,
                    bits,
                    palette[color],
                );
                cursor_x = cursor_x.saturating_add(1);
                max_x = max_x.max(cursor_x);
                max_y = max_y.max(cursor_y + 6);
                idx += 1;
            }
            b'!' => {
                idx += 1;
                let (repeat, next) = parse_number(payload, idx).ok_or(SixelError::InvalidRepeat)?;
                idx = next;
                if idx >= payload.len() {
                    return Err(SixelError::InvalidRepeat);
                }
                if matches!(payload[idx], b'?'..=b'~') {
                    let bits = payload[idx] - b'?';
                    for _ in 0..repeat {
                        paint_sixel(
                            &mut pixels,
                            &mut width,
                            &mut height,
                            cursor_x,
                            cursor_y,
                            bits,
                            palette[color],
                        );
                        cursor_x = cursor_x.saturating_add(1);
                    }
                    max_x = max_x.max(cursor_x);
                    max_y = max_y.max(cursor_y + 6);
                }
                idx += 1;
            }
            b'#' => {
                idx += 1;
                let (number, next) = parse_number(payload, idx).ok_or(SixelError::InvalidColor)?;
                color = number.min(255);
                idx = next;
                if idx < payload.len() && payload[idx] == b';' {
                    let (values, next) = parse_semicolon_numbers(payload, idx);
                    idx = next;
                    if values.len() >= 4 && values[0] == 2 {
                        palette[color] = rgb_percent(values[1], values[2], values[3]);
                    }
                }
            }
            b'"' => {
                idx += 1;
                let (values, next) = parse_semicolon_numbers(payload, idx);
                idx = next;
                if values.len() >= 4 {
                    width = width.max(values[2]);
                    height = height.max(values[3].max(6));
                    pixels.resize(width.saturating_mul(height), None);
                    max_x = max_x.max(values[2]);
                    max_y = max_y.max(values[3]);
                }
            }
            b'$' => {
                cursor_x = 0;
                idx += 1;
            }
            b'-' => {
                cursor_x = 0;
                cursor_y = cursor_y.saturating_add(6);
                max_y = max_y.max(cursor_y + 6);
                idx += 1;
            }
            _ => idx += 1,
        }
    }

    let final_width = max_x.max(width).max(1);
    let final_height = max_y.max(height).max(6);
    pixels.resize(final_width.saturating_mul(final_height), None);

    let mut out = vec![0; final_width.saturating_mul(final_height).saturating_mul(4)];
    for y in 0..final_height {
        for x in 0..final_width {
            let src = y * final_width + x;
            if let Some(px) = pixels.get(src).and_then(|p| *p) {
                let dst = src * 4;
                out[dst..dst + 4].copy_from_slice(&px.0);
            }
        }
    }

    Ok(SixelImage {
        width: final_width,
        height: final_height,
        pixels: out,
    })
}

fn paint_sixel(
    pixels: &mut Vec<Option<Rgba>>,
    width: &mut usize,
    height: &mut usize,
    x: usize,
    y: usize,
    bits: u8,
    color: Rgba,
) {
    let needed_w = x.saturating_add(1);
    let needed_h = y.saturating_add(6);
    grow(pixels, width, height, needed_w, needed_h);
    for bit in 0..6usize {
        if bits & (1 << bit) != 0 {
            let yy = y + bit;
            let idx = yy * *width + x;
            if let Some(slot) = pixels.get_mut(idx) {
                *slot = Some(color);
            }
        }
    }
}

fn grow(pixels: &mut Vec<Option<Rgba>>, width: &mut usize, height: &mut usize, w: usize, h: usize) {
    if w <= *width && h <= *height {
        return;
    }
    let new_w = (*width).max(w).max(1);
    let new_h = (*height).max(h).max(1);
    let mut next = vec![None; new_w.saturating_mul(new_h)];
    for y in 0..*height {
        let old_start = y * *width;
        let new_start = y * new_w;
        next[new_start..new_start + *width].copy_from_slice(&pixels[old_start..old_start + *width]);
    }
    *pixels = next;
    *width = new_w;
    *height = new_h;
}

fn parse_number(bytes: &[u8], mut idx: usize) -> Option<(usize, usize)> {
    let start = idx;
    let mut value = 0usize;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        value = value
            .saturating_mul(10)
            .saturating_add((bytes[idx] - b'0') as usize);
        idx += 1;
    }
    (idx > start).then_some((value, idx))
}

fn parse_semicolon_numbers(bytes: &[u8], mut idx: usize) -> (Vec<usize>, usize) {
    let mut values = Vec::new();
    while idx < bytes.len() && bytes[idx] == b';' {
        idx += 1;
        if let Some((value, next)) = parse_number(bytes, idx) {
            values.push(value);
            idx = next;
        } else {
            break;
        }
    }
    (values, idx)
}

fn rgb_percent(r: usize, g: usize, b: usize) -> Rgba {
    let scale = |value: usize| -> u8 { value.min(100).saturating_mul(255) as u8 / 100 };
    Rgba([scale(r), scale(g), scale(b), 255])
}

fn default_palette() -> [Rgba; 256] {
    let mut palette = [Rgba::default(); 256];
    palette[0] = Rgba([0, 0, 0, 255]);
    palette[1] = Rgba([51, 51, 204, 255]);
    palette[2] = Rgba([204, 51, 51, 255]);
    palette[3] = Rgba([51, 204, 51, 255]);
    palette[4] = Rgba([204, 51, 204, 255]);
    palette[5] = Rgba([51, 204, 204, 255]);
    palette[6] = Rgba([204, 204, 51, 255]);
    palette[7] = Rgba([229, 229, 229, 255]);
    palette
}

#[cfg(test)]
mod tests {
    use super::{DcsSettings, sixel_decode_from_dcs};

    #[test]
    fn decodes_basic_sixel_payload() {
        let image = sixel_decode_from_dcs(
            b"\"1;1;2;2#0;2;0;0;0#0~~",
            DcsSettings::new(None, None, None),
        )
        .unwrap();
        assert_eq!(image.corrected_dimensions().0, 2);
        assert!(image.corrected_dimensions().1 >= 6);
        assert_eq!(image.pixels.len(), image.width * image.height * 4);
    }

    #[test]
    fn repeat_advances_columns() {
        let image = sixel_decode_from_dcs(b"#0!5~", DcsSettings::new(None, None, None)).unwrap();
        assert_eq!(image.corrected_dimensions().0, 5);
        assert_eq!(image.pixels.len(), image.width * image.height * 4);
    }
}
