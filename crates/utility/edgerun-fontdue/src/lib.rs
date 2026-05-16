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

#[derive(Clone, Copy, Debug)]
struct Transform {
    xx: f32,
    xy: f32,
    yx: f32,
    yy: f32,
    dx: f32,
    dy: f32,
}

impl Transform {
    const IDENTITY: Self = Self {
        xx: 1.0,
        xy: 0.0,
        yx: 0.0,
        yy: 1.0,
        dx: 0.0,
        dy: 0.0,
    };

    fn then(self, next: Self) -> Self {
        Self {
            xx: self.xx * next.xx + self.xy * next.yx,
            xy: self.xx * next.xy + self.xy * next.yy,
            yx: self.yx * next.xx + self.yy * next.yx,
            yy: self.yx * next.xy + self.yy * next.yy,
            dx: self.xx * next.dx + self.xy * next.dy + self.dx,
            dy: self.yx * next.dx + self.yy * next.dy + self.dy,
        }
    }

    fn point(self, point: Point) -> Point {
        Point {
            x: point.x * self.xx + point.y * self.xy + self.dx,
            y: point.x * self.yx + point.y * self.yy + self.dy,
            on: point.on,
        }
    }
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

        let glyph = self.glyph_index(ch).unwrap_or(0);
        let Some((_, _, x_min, y_min, x_max, y_max)) = self.glyph_header(glyph) else {
            return Metrics {
                advance_width,
                advance_height: self.line_height(px),
                ..Metrics::default()
            };
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

        let glyph = self.glyph_index(ch).unwrap_or(0);
        let Some(segments) = self.flatten_glyph(glyph, px) else {
            return (metrics, Vec::new());
        };
        if segments.is_empty() {
            return (metrics, Vec::new());
        }

        let samples = raster_samples(px);
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
                        if point_inside_non_zero(px_x, px_y, &segments) {
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
        let mut segments = Vec::new();
        self.flatten_glyph_inner(glyph, px, Transform::IDENTITY, 0, &mut segments)?;
        Some(segments)
    }

    fn flatten_glyph_inner(
        &self,
        glyph: u16,
        px: f32,
        transform: Transform,
        depth: u8,
        segments: &mut Vec<Segment>,
    ) -> Option<()> {
        if depth > 8 {
            return None;
        }

        let (off, number_of_contours, _, _, _, _) = self.glyph_header(glyph)?;
        if number_of_contours <= 0 {
            return self.flatten_composite_glyph(off, px, transform, depth, segments);
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
                if flag & 0x10 != 0 { v } else { -v }
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
                if flag & 0x20 != 0 { v } else { -v }
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
            points.push(transform.point(Point {
                x: xs[i] as f32 * scale,
                y: ys[i] as f32 * scale,
                on: flags[i] & 0x01 != 0,
            }));
        }

        let mut start = 0usize;
        for end in end_pts {
            if end >= points.len() || start > end {
                return None;
            }
            flatten_contour(&points[start..=end], &mut *segments);
            start = end + 1;
        }
        Some(())
    }

    fn flatten_composite_glyph(
        &self,
        off: usize,
        px: f32,
        transform: Transform,
        depth: u8,
        segments: &mut Vec<Segment>,
    ) -> Option<()> {
        const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
        const ARGS_ARE_XY_VALUES: u16 = 0x0002;
        const WE_HAVE_A_SCALE: u16 = 0x0008;
        const MORE_COMPONENTS: u16 = 0x0020;
        const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
        const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;
        const WE_HAVE_INSTRUCTIONS: u16 = 0x0100;

        let scale = px / self.units_per_em as f32;
        let mut pos = off + 10;
        loop {
            let flags = read_u16(&self.data, pos)?;
            let component_glyph = read_u16(&self.data, pos + 2)?;
            pos += 4;

            let (arg1, arg2) = if flags & ARG_1_AND_2_ARE_WORDS != 0 {
                let arg1 = read_i16(&self.data, pos)?;
                let arg2 = read_i16(&self.data, pos + 2)?;
                pos += 4;
                (arg1, arg2)
            } else {
                let arg1 = *self.data.get(pos)? as i8 as i16;
                let arg2 = *self.data.get(pos + 1)? as i8 as i16;
                pos += 2;
                (arg1, arg2)
            };

            if flags & ARGS_ARE_XY_VALUES == 0 {
                return None;
            }

            let mut component = Transform {
                xx: 1.0,
                xy: 0.0,
                yx: 0.0,
                yy: 1.0,
                dx: arg1 as f32 * scale,
                dy: arg2 as f32 * scale,
            };

            if flags & WE_HAVE_A_SCALE != 0 {
                let s = f2dot14(read_i16(&self.data, pos)?);
                pos += 2;
                component.xx = s;
                component.yy = s;
            } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
                component.xx = f2dot14(read_i16(&self.data, pos)?);
                component.yy = f2dot14(read_i16(&self.data, pos + 2)?);
                pos += 4;
            } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
                component.xx = f2dot14(read_i16(&self.data, pos)?);
                component.yx = f2dot14(read_i16(&self.data, pos + 2)?);
                component.xy = f2dot14(read_i16(&self.data, pos + 4)?);
                component.yy = f2dot14(read_i16(&self.data, pos + 6)?);
                pos += 8;
            }

            self.flatten_glyph_inner(
                component_glyph,
                px,
                transform.then(component),
                depth + 1,
                segments,
            )?;

            if flags & MORE_COMPONENTS == 0 {
                if flags & WE_HAVE_INSTRUCTIONS != 0 {
                    let instruction_len = read_u16(&self.data, pos)? as usize;
                    pos = pos.checked_add(2 + instruction_len)?;
                    self.data.get(pos.saturating_sub(1))?;
                }
                return Some(());
            }
        }
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

fn point_inside_non_zero(x: f32, y: f32, segments: &[Segment]) -> bool {
    let mut winding = 0i32;
    for segment in segments {
        if segment.ay <= y {
            if segment.by > y && is_left(segment, x, y) > 0.0 {
                winding += 1;
            }
        } else if segment.by <= y && is_left(segment, x, y) < 0.0 {
            winding -= 1;
        }
    }
    winding != 0
}

fn is_left(segment: &Segment, x: f32, y: f32) -> f32 {
    (segment.bx - segment.ax) * (y - segment.ay) - (x - segment.ax) * (segment.by - segment.ay)
}

fn raster_samples(px: f32) -> usize {
    if px <= 14.0 {
        8
    } else if px <= 28.0 {
        6
    } else {
        4
    }
}

fn f2dot14(value: i16) -> f32 {
    value as f32 / 16384.0
}

#[cfg(test)]
fn bitmap_has_soft_edges(bitmap: &[u8]) -> bool {
    bitmap.iter().any(|alpha| *alpha > 0 && *alpha < 255)
}

#[cfg(test)]
fn bitmap_edge_coverage(bitmap: &[u8], width: usize, height: usize) -> usize {
    if width == 0 || height == 0 {
        return 0;
    }
    let mut total = 0usize;
    for x in 0..width {
        total += bitmap[x] as usize;
        total += bitmap[(height - 1) * width + x] as usize;
    }
    for y in 0..height {
        total += bitmap[y * width] as usize;
        total += bitmap[y * width + width - 1] as usize;
    }
    total
}

#[cfg(test)]
fn looks_like_fallback_box(bitmap: &[u8], width: usize, height: usize) -> bool {
    if width == 0 || height == 0 {
        return false;
    }
    let edge = bitmap_edge_coverage(bitmap, width, height);
    let max_edge = (width * 2 + height * 2) * 255;
    edge > max_edge * 7 / 10
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

#[cfg(test)]
mod tests {
    use super::*;

    const DEJAVU_MONO: &[u8] =
        include_bytes!("../../../edgerun-term/edgerun-term-core/assets/DejaVuSansMono.ttf");
    const GEIST_VARIABLE: &[u8] = include_bytes!("../../edgerun-ui-core/assets/Geist-Variable.ttf");

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
        assert!(
            bitmap_has_soft_edges(&bitmap),
            "coverage rasterizer should preserve antialiased edge alpha"
        );
    }

    #[test]
    fn uses_font_metrics_for_advance_width() {
        let font = Font::from_bytes(DEJAVU_MONO, FontSettings).unwrap();

        assert!(font.metrics('W', 18.0).advance_width > 1.0);
        assert!(font.metrics(' ', 18.0).advance_width > 1.0);
        assert_eq!(font.rasterize(' ', 18.0).1.len(), 0);
    }

    #[test]
    fn rasterizes_composite_glyphs_from_real_outlines() {
        let font = Font::from_bytes(GEIST_VARIABLE, FontSettings).unwrap();
        let (metrics, bitmap) = font.rasterize('é', 30.0);

        assert!(metrics.width > 8);
        assert!(metrics.height > 12);
        assert_eq!(bitmap.len(), metrics.width * metrics.height);
        assert!(bitmap.iter().any(|alpha| *alpha > 0));
        assert!(
            !looks_like_fallback_box(&bitmap, metrics.width, metrics.height),
            "composite glyph should be flattened from components, not fallback-rasterized"
        );
    }

    #[test]
    fn rasterizes_variable_font_default_instance_with_real_edges() {
        let font = Font::from_bytes(GEIST_VARIABLE, FontSettings).unwrap();
        let (metrics, bitmap) = font.rasterize('a', 16.0);

        assert!(metrics.width > 4);
        assert!(metrics.height > 6);
        assert!(bitmap_has_soft_edges(&bitmap));
        assert!(
            !looks_like_fallback_box(&bitmap, metrics.width, metrics.height),
            "default variable-font instance should use glyf outlines"
        );
    }
}
