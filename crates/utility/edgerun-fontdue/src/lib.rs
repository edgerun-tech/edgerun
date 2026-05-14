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
    data: Vec<u8>,
    tables: Tables,
    units_per_em: u16,
    index_to_loc_format: i16,
    num_glyphs: u16,
    num_h_metrics: u16,
    ascent: i16,
    descent: i16,
}

#[derive(Clone, Copy, Debug, Default)]
struct Tables {
    cmap: Table,
    glyf: Table,
    head: Table,
    hhea: Table,
    hmtx: Table,
    loca: Table,
    maxp: Table,
}

#[derive(Clone, Copy, Debug, Default)]
struct Table {
    off: usize,
    len: usize,
}

#[derive(Clone, Copy, Debug)]
struct Point {
    x: f32,
    y: f32,
    on: bool,
}

#[derive(Clone, Copy, Debug)]
struct Segment {
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
}

impl Font {
    pub fn from_bytes<B: AsRef<[u8]>>(
        bytes: B,
        _settings: FontSettings,
    ) -> Result<Self, &'static str> {
        let data = bytes.as_ref().to_vec();
        if data.len() < 12 {
            return Err("font too small");
        }

        let num_tables = read_u16(&data, 4).ok_or("bad sfnt header")? as usize;
        let mut tables = Tables::default();
        for i in 0..num_tables {
            let off = 12 + i * 16;
            let tag = data.get(off..off + 4).ok_or("bad table record")?;
            let table = Table {
                off: read_u32(&data, off + 8).ok_or("bad table offset")? as usize,
                len: read_u32(&data, off + 12).ok_or("bad table length")? as usize,
            };
            if table
                .off
                .checked_add(table.len)
                .is_none_or(|end| end > data.len())
            {
                return Err("table outside font");
            }
            match tag {
                b"cmap" => tables.cmap = table,
                b"glyf" => tables.glyf = table,
                b"head" => tables.head = table,
                b"hhea" => tables.hhea = table,
                b"hmtx" => tables.hmtx = table,
                b"loca" => tables.loca = table,
                b"maxp" => tables.maxp = table,
                _ => {}
            }
        }

        if tables.cmap.len == 0
            || tables.glyf.len == 0
            || tables.head.len == 0
            || tables.hhea.len == 0
            || tables.hmtx.len == 0
            || tables.loca.len == 0
            || tables.maxp.len == 0
        {
            return Err("missing TrueType tables");
        }

        let units_per_em = read_u16(&data, tables.head.off + 18).ok_or("bad head table")?;
        let index_to_loc_format = read_i16(&data, tables.head.off + 50).ok_or("bad head table")?;
        let ascent = read_i16(&data, tables.hhea.off + 4).ok_or("bad hhea table")?;
        let descent = read_i16(&data, tables.hhea.off + 6).ok_or("bad hhea table")?;
        let num_h_metrics = read_u16(&data, tables.hhea.off + 34).ok_or("bad hhea table")?;
        let num_glyphs = read_u16(&data, tables.maxp.off + 4).ok_or("bad maxp table")?;

        if units_per_em == 0 || num_h_metrics == 0 || num_glyphs == 0 {
            return Err("invalid TrueType metrics");
        }

        Ok(Self {
            data,
            tables,
            units_per_em,
            index_to_loc_format,
            num_glyphs,
            num_h_metrics,
            ascent,
            descent,
        })
    }

    pub fn metrics(&self, ch: char, px: f32) -> Metrics {
        let advance_width = self.advance_width(ch, px);
        if ch.is_whitespace() {
            return Metrics {
                advance_width,
                advance_height: self.line_height(px),
                ..Metrics::default()
            };
        }

        let Some(glyph) = self.glyph_index(ch) else {
            return fallback_metrics(ch, px, advance_width, self.line_height(px));
        };
        let Some((_, _, x_min, y_min, x_max, y_max)) = self.glyph_header(glyph) else {
            return fallback_metrics(ch, px, advance_width, self.line_height(px));
        };
        let scale = px / self.units_per_em as f32;
        let xmin = floor_f32(x_min as f32 * scale) as i32;
        let ymin = floor_f32(y_min as f32 * scale) as i32;
        let xmax = ceil_f32(x_max as f32 * scale) as i32;
        let ymax = ceil_f32(y_max as f32 * scale) as i32;
        let width = xmax.saturating_sub(xmin).max(0) as usize;
        let height = ymax.saturating_sub(ymin).max(0) as usize;

        Metrics {
            xmin,
            ymin,
            width,
            height,
            advance_width,
            advance_height: self.line_height(px),
            bounds: OutlineBounds {
                xmin: xmin as f32,
                ymin: ymin as f32,
                width: width as f32,
                height: height as f32,
            },
        }
    }

    pub fn rasterize(&self, ch: char, px: f32) -> (Metrics, Vec<u8>) {
        let metrics = self.metrics(ch, px);
        if metrics.width == 0 || metrics.height == 0 || ch.is_whitespace() {
            return (metrics, Vec::new());
        }

        let Some(glyph) = self.glyph_index(ch) else {
            return rasterize_fallback(ch, px, metrics);
        };
        let Some(segments) = self.flatten_glyph(glyph, px) else {
            return rasterize_fallback(ch, px, metrics);
        };
        if segments.is_empty() {
            return (metrics, Vec::new());
        }

        let samples = 4usize;
        let inv_samples = 1.0 / (samples * samples) as f32;
        let mut bitmap = vec![0u8; metrics.width * metrics.height];
        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let mut covered = 0usize;
                for sy in 0..samples {
                    for sx in 0..samples {
                        let px_x =
                            metrics.xmin as f32 + x as f32 + (sx as f32 + 0.5) / samples as f32;
                        let px_y = metrics.ymin as f32
                            + (metrics.height - 1 - y) as f32
                            + (sy as f32 + 0.5) / samples as f32;
                        if point_inside(px_x, px_y, &segments) {
                            covered += 1;
                        }
                    }
                }
                bitmap[y * metrics.width + x] = (covered as f32 * inv_samples * 255.0) as u8;
            }
        }

        (metrics, bitmap)
    }

    fn line_height(&self, px: f32) -> f32 {
        let height = i32::from(self.ascent) - i32::from(self.descent);
        (height as f32 * px / self.units_per_em as f32).max(px)
    }

    fn advance_width(&self, ch: char, px: f32) -> f32 {
        let glyph = self.glyph_index(ch).unwrap_or(0) as usize;
        let metric_index = glyph.min(self.num_h_metrics.saturating_sub(1) as usize);
        let off = self.tables.hmtx.off + metric_index * 4;
        let advance = read_u16(&self.data, off).unwrap_or(self.units_per_em / 2);
        (advance as f32 * px / self.units_per_em as f32).max(if ch.is_whitespace() {
            px * 0.28
        } else {
            1.0
        })
    }

    fn glyph_index(&self, ch: char) -> Option<u16> {
        let code = ch as u32;
        let cmap = self.tables.cmap.off;
        let num_tables = read_u16(&self.data, cmap + 2)? as usize;
        let mut best = None;
        for i in 0..num_tables {
            let rec = cmap + 4 + i * 8;
            let platform = read_u16(&self.data, rec)?;
            let encoding = read_u16(&self.data, rec + 2)?;
            let sub_off = cmap + read_u32(&self.data, rec + 4)? as usize;
            let format = read_u16(&self.data, sub_off)?;
            let priority = match (format, platform, encoding) {
                (12, 3, 10) => 0,
                (4, 3, 1 | 10) => 1,
                (4, _, _) => 2,
                _ => 9,
            };
            if priority < best.map_or(usize::MAX, |(_, p)| p) {
                best = Some((sub_off, priority));
            }
        }

        let (sub_off, _) = best?;
        match read_u16(&self.data, sub_off)? {
            4 if code <= u16::MAX as u32 => self.glyph_index_format4(sub_off, code as u16),
            12 => self.glyph_index_format12(sub_off, code),
            _ => None,
        }
    }

    fn glyph_index_format4(&self, off: usize, code: u16) -> Option<u16> {
        let seg_count = read_u16(&self.data, off + 6)? as usize / 2;
        let end_codes = off + 14;
        let start_codes = end_codes + seg_count * 2 + 2;
        let id_deltas = start_codes + seg_count * 2;
        let id_range_offsets = id_deltas + seg_count * 2;

        for i in 0..seg_count {
            let end = read_u16(&self.data, end_codes + i * 2)?;
            let start = read_u16(&self.data, start_codes + i * 2)?;
            if code < start || code > end {
                continue;
            }
            let delta = read_i16(&self.data, id_deltas + i * 2)? as i32;
            let range_offset_pos = id_range_offsets + i * 2;
            let range_offset = read_u16(&self.data, range_offset_pos)? as usize;
            let glyph = if range_offset == 0 {
                ((code as i32 + delta) & 0xffff) as u16
            } else {
                let glyph_off = range_offset_pos + range_offset + (code - start) as usize * 2;
                let raw = read_u16(&self.data, glyph_off)?;
                if raw == 0 {
                    0
                } else {
                    ((raw as i32 + delta) & 0xffff) as u16
                }
            };
            return (glyph < self.num_glyphs).then_some(glyph);
        }
        None
    }

    fn glyph_index_format12(&self, off: usize, code: u32) -> Option<u16> {
        let groups = read_u32(&self.data, off + 12)? as usize;
        for i in 0..groups {
            let rec = off + 16 + i * 12;
            let start = read_u32(&self.data, rec)?;
            let end = read_u32(&self.data, rec + 4)?;
            let start_glyph = read_u32(&self.data, rec + 8)?;
            if code >= start && code <= end {
                let glyph = start_glyph.checked_add(code - start)?;
                return (glyph < self.num_glyphs as u32).then_some(glyph as u16);
            }
        }
        None
    }

    fn glyph_header(&self, glyph: u16) -> Option<(usize, i16, i16, i16, i16, i16)> {
        let start = self.glyph_offset(glyph)?;
        let end = self.glyph_offset(glyph + 1)?;
        if start == end {
            return Some((start, 0, 0, 0, 0, 0));
        }
        if end < start || end.saturating_sub(start) < 10 {
            return None;
        }
        let off = self.tables.glyf.off + start;
        Some((
            off,
            read_i16(&self.data, off)?,
            read_i16(&self.data, off + 2)?,
            read_i16(&self.data, off + 4)?,
            read_i16(&self.data, off + 6)?,
            read_i16(&self.data, off + 8)?,
        ))
    }

    fn glyph_offset(&self, glyph: u16) -> Option<usize> {
        let glyph = glyph.min(self.num_glyphs);
        match self.index_to_loc_format {
            0 => read_u16(&self.data, self.tables.loca.off + glyph as usize * 2)
                .map(|v| v as usize * 2),
            1 => {
                read_u32(&self.data, self.tables.loca.off + glyph as usize * 4).map(|v| v as usize)
            }
            _ => None,
        }
    }

    fn flatten_glyph(&self, glyph: u16, px: f32) -> Option<Vec<Segment>> {
        let (off, number_of_contours, _, _, _, _) = self.glyph_header(glyph)?;
        if number_of_contours <= 0 {
            return None;
        }
        let contours = number_of_contours as usize;
        let mut end_pts = Vec::with_capacity(contours);
        for i in 0..contours {
            end_pts.push(read_u16(&self.data, off + 10 + i * 2)? as usize);
        }
        let point_count = end_pts.last().copied()?.checked_add(1)?;
        let instruction_len_off = off + 10 + contours * 2;
        let instruction_len = read_u16(&self.data, instruction_len_off)? as usize;
        let mut pos = instruction_len_off + 2 + instruction_len;

        let mut flags = Vec::with_capacity(point_count);
        while flags.len() < point_count {
            let flag = *self.data.get(pos)?;
            pos += 1;
            flags.push(flag);
            if flag & 0x08 != 0 {
                let repeat = *self.data.get(pos)? as usize;
                pos += 1;
                for _ in 0..repeat {
                    flags.push(flag);
                }
            }
        }
        flags.truncate(point_count);

        let mut xs = Vec::with_capacity(point_count);
        let mut x = 0i16;
        for &flag in &flags {
            let delta = if flag & 0x02 != 0 {
                let v = *self.data.get(pos)? as i16;
                pos += 1;
                if flag & 0x10 != 0 {
                    v
                } else {
                    -v
                }
            } else if flag & 0x10 != 0 {
                0
            } else {
                let v = read_i16(&self.data, pos)?;
                pos += 2;
                v
            };
            x = x.wrapping_add(delta);
            xs.push(x);
        }

        let mut ys = Vec::with_capacity(point_count);
        let mut y = 0i16;
        for &flag in &flags {
            let delta = if flag & 0x04 != 0 {
                let v = *self.data.get(pos)? as i16;
                pos += 1;
                if flag & 0x20 != 0 {
                    v
                } else {
                    -v
                }
            } else if flag & 0x20 != 0 {
                0
            } else {
                let v = read_i16(&self.data, pos)?;
                pos += 2;
                v
            };
            y = y.wrapping_add(delta);
            ys.push(y);
        }

        let scale = px / self.units_per_em as f32;
        let mut points = Vec::with_capacity(point_count);
        for i in 0..point_count {
            points.push(Point {
                x: xs[i] as f32 * scale,
                y: ys[i] as f32 * scale,
                on: flags[i] & 0x01 != 0,
            });
        }

        let mut segments = Vec::new();
        let mut start = 0usize;
        for end in end_pts {
            if end >= points.len() || start > end {
                return None;
            }
            flatten_contour(&points[start..=end], &mut segments);
            start = end + 1;
        }
        Some(segments)
    }
}

fn flatten_contour(points: &[Point], out: &mut Vec<Segment>) {
    if points.is_empty() {
        return;
    }
    let n = points.len();
    let mut current = if points[0].on {
        points[0]
    } else if points[n - 1].on {
        points[n - 1]
    } else {
        midpoint(points[0], points[n - 1])
    };

    for i in 0..n {
        let p = points[i];
        let next = points[(i + 1) % n];
        if p.on {
            current = p;
            if next.on {
                push_line(out, current, next);
                current = next;
            }
        } else {
            let end = if next.on { next } else { midpoint(p, next) };
            push_quad(out, current, p, end);
            current = end;
        }
    }
}

fn push_line(out: &mut Vec<Segment>, a: Point, b: Point) {
    if (a.x - b.x).abs() > f32::EPSILON || (a.y - b.y).abs() > f32::EPSILON {
        out.push(Segment {
            ax: a.x,
            ay: a.y,
            bx: b.x,
            by: b.y,
        });
    }
}

fn push_quad(out: &mut Vec<Segment>, a: Point, c: Point, b: Point) {
    let steps = 12usize;
    let mut prev = a;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let mt = 1.0 - t;
        let p = Point {
            x: mt * mt * a.x + 2.0 * mt * t * c.x + t * t * b.x,
            y: mt * mt * a.y + 2.0 * mt * t * c.y + t * t * b.y,
            on: true,
        };
        push_line(out, prev, p);
        prev = p;
    }
}

fn midpoint(a: Point, b: Point) -> Point {
    Point {
        x: (a.x + b.x) * 0.5,
        y: (a.y + b.y) * 0.5,
        on: true,
    }
}

fn point_inside(x: f32, y: f32, segments: &[Segment]) -> bool {
    let mut inside = false;
    for segment in segments {
        let y_cross = (segment.ay > y) != (segment.by > y);
        if !y_cross {
            continue;
        }
        let denom = segment.by - segment.ay;
        if denom.abs() <= f32::EPSILON {
            continue;
        }
        let cross_x = segment.ax + (y - segment.ay) * (segment.bx - segment.ax) / denom;
        if cross_x > x {
            inside = !inside;
        }
    }
    inside
}

fn fallback_metrics(ch: char, px: f32, advance_width: f32, advance_height: f32) -> Metrics {
    if ch.is_whitespace() {
        return Metrics {
            advance_width,
            advance_height,
            ..Metrics::default()
        };
    }
    let width = round_f32(px * 0.56).max(1.0) as usize;
    let height = round_f32(px * 0.82).max(1.0) as usize;
    Metrics {
        xmin: 0,
        ymin: 0,
        width,
        height,
        advance_width,
        advance_height,
        bounds: OutlineBounds {
            xmin: 0.0,
            ymin: 0.0,
            width: width as f32,
            height: height as f32,
        },
    }
}

fn rasterize_fallback(ch: char, px: f32, metrics: Metrics) -> (Metrics, Vec<u8>) {
    let mut bitmap = vec![0u8; metrics.width * metrics.height];
    let stroke = (round_f32(px / 13.0) as usize).clamp(1, 3);
    let code = ch as usize;
    for y in 0..metrics.height {
        for x in 0..metrics.width {
            let border = x < stroke
                || y < stroke
                || metrics.width.saturating_sub(x + 1) < stroke
                || metrics.height.saturating_sub(y + 1) < stroke;
            let diagonal = ((x + y + code) % 9) < stroke;
            if border || diagonal {
                bitmap[y * metrics.width + x] = 220;
            }
        }
    }
    (metrics, bitmap)
}

fn read_u16(bytes: &[u8], off: usize) -> Option<u16> {
    let b = bytes.get(off..off + 2)?;
    Some(u16::from_be_bytes([b[0], b[1]]))
}

fn read_i16(bytes: &[u8], off: usize) -> Option<i16> {
    read_u16(bytes, off).map(|v| v as i16)
}

fn read_u32(bytes: &[u8], off: usize) -> Option<u32> {
    let b = bytes.get(off..off + 4)?;
    Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn floor_f32(value: f32) -> f32 {
    let i = value as i32;
    if value < i as f32 {
        (i - 1) as f32
    } else {
        i as f32
    }
}

fn ceil_f32(value: f32) -> f32 {
    let i = value as i32;
    if value > i as f32 {
        (i + 1) as f32
    } else {
        i as f32
    }
}

fn round_f32(value: f32) -> f32 {
    if value.is_sign_negative() {
        (value - 0.5) as i32 as f32
    } else {
        (value + 0.5) as i32 as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEJAVU_MONO: &[u8] =
        include_bytes!("../../../edgerun-term/edgerun-term-core/assets/DejaVuSansMono.ttf");

    #[test]
    fn rasterizes_real_truetype_outlines() {
        let font = Font::from_bytes(DEJAVU_MONO, FontSettings).unwrap();
        let (metrics, bitmap) = font.rasterize('A', 32.0);

        assert!(metrics.width > 8);
        assert!(metrics.height > 12);
        assert_eq!(bitmap.len(), metrics.width * metrics.height);
        assert!(bitmap.iter().any(|alpha| *alpha > 0));

        let left_edge_coverage: usize = (0..metrics.height)
            .map(|y| bitmap[y * metrics.width] as usize)
            .sum();
        assert!(
            left_edge_coverage < metrics.height * 80,
            "outline rasterizer should not draw the old full-height box fallback"
        );
    }

    #[test]
    fn uses_font_metrics_for_advance_width() {
        let font = Font::from_bytes(DEJAVU_MONO, FontSettings).unwrap();

        assert!(font.metrics('W', 18.0).advance_width > 1.0);
        assert!(font.metrics(' ', 18.0).advance_width > 1.0);
        assert_eq!(font.rasterize(' ', 18.0).1.len(), 0);
    }
}
