//! Cursor rendering — full Adwaita-style white cursor set.
//!
//! Provides all standard cursor shapes as 24x24 RGBA bitmaps.

/// Cursor width/height.
pub const CURSOR_SIZE: usize = 24;

/// All cursor shapes as 24x24 RGBA bitmaps.
/// Index matches wp_cursor_shape_manager_v1 shape IDs (with some extras).
/// 
/// Shapes:
/// 0 = default (arrow), 1 = pointer (hand), 2 = text (I-beam),
/// 3 = wait (loading), 4 = progress, 5 = grab, 6 = grabbing,
/// 7 = move, 8 = n-resize, 9 = s-resize, 10 = e-resize, 11 = w-resize,
/// 12 = ne-resize, 13 = nw-resize, 14 = se-resize, 15 = sw-resize,
/// 16 = ew-resize, 17 = ns-resize, 18 = crosshair, 19 = not-allowed,
/// 20 = copy, 21 = alias, 22 = zoom-in, 23 = zoom-out
///
/// Generated at compile time from shape masks.
pub const CURSOR_BITMAPS: [[u8; CURSOR_SIZE * CURSOR_SIZE * 4]; 24] = {
    let masks = [
        ARROW_MASK,        // 0: default
        HAND_MASK,         // 1: pointer
        IBEAM_MASK,        // 2: text
        WATCH_MASK,        // 3: wait
        WATCH_MASK,        // 4: progress (same as wait for now)
        OPEN_HAND_MASK,    // 5: grab
        CLOSED_HAND_MASK,  // 6: grabbing
        MOVE_MASK,         // 7: move
        ARROW_UP_MASK,     // 8: n-resize
        ARROW_DOWN_MASK,   // 9: s-resize
        ARROW_RIGHT_MASK,  // 10: e-resize
        ARROW_LEFT_MASK,   // 11: w-resize
        ARROW_NE_MASK,     // 12: ne-resize
        ARROW_NW_MASK,     // 13: nw-resize
        ARROW_SE_MASK,     // 14: se-resize
        ARROW_SW_MASK,     // 15: sw-resize
        ARROW_EW_MASK,     // 16: ew-resize
        ARROW_NS_MASK,     // 17: ns-resize
        CROSSHAIR_MASK,    // 18: crosshair
        NOT_ALLOWED_MASK,  // 19: not-allowed
        ARROW_PLUS_MASK,   // 20: copy
        ARROW_CURVED_MASK, // 21: alias
        CIRCLE_PLUS_MASK,  // 22: zoom-in
        CIRCLE_MINUS_MASK, // 23: zoom-out
    ];

    let mut bitmaps = [[0u8; CURSOR_SIZE * CURSOR_SIZE * 4]; 24];
    let mut s = 0;
    while s < 24 {
        let mask = &masks[s];
        let mut p = 0;
        while p < 576 {
            let px = p % 24;
            let py = p / 24;
            let inside = mask[p];

            // Check if on edge (any 8-neighbor is outside)
            let mut on_edge = false;
            if inside {
                let mut ny: isize = 0;
                while ny < 3 {
                    let mut nx: isize = 0;
                    while nx < 3 {
                        if nx == 0 && ny == 0 {
                            nx += 1;
                            continue;
                        }
                        let cx = px as isize + nx - 1;
                        let cy = py as isize + ny - 1;
                        if cx < 0 || cx >= 24 || cy < 0 || cy >= 24 {
                            on_edge = true;
                        } else if !mask[(cy as usize) * 24 + (cx as usize)] {
                            on_edge = true;
                        }
                        nx += 1;
                    }
                    ny += 1;
                }
            }

            let b = p * 4;
            if inside && on_edge {
                bitmaps[s][b] = 0;
                bitmaps[s][b + 1] = 0;
                bitmaps[s][b + 2] = 0;
                bitmaps[s][b + 3] = 255;
            } else if inside {
                bitmaps[s][b] = 255;
                bitmaps[s][b + 1] = 255;
                bitmaps[s][b + 2] = 255;
                bitmaps[s][b + 3] = 255;
            } else {
                // Shadow at +2,+2
                let sx = px + 2;
                let sy = py + 2;
                if sx < 24 && sy < 24 && mask[sy * 24 + sx] {
                    bitmaps[s][b] = 0;
                    bitmaps[s][b + 1] = 0;
                    bitmaps[s][b + 2] = 0;
                    bitmaps[s][b + 3] = 40;
                } else {
                    bitmaps[s][b + 3] = 0;
                }
            }
            p += 1;
        }
        s += 1;
    }
    bitmaps
};

/// Cursor hotspot for each shape (x, y).
pub const CURSOR_HOTSPOTS: [(i32, i32); 24] = [
    (3, 3),    // 0: default (arrow tip)
    (8, 6),    // 1: pointer (hand tip)
    (12, 12),  // 2: text (I-beam center)
    (12, 12),  // 3: wait (watch center)
    (12, 12),  // 4: progress (watch center)
    (12, 8),   // 5: grab (hand center)
    (12, 8),   // 6: grabbing (closed hand)
    (12, 12),  // 7: move (cross center)
    (12, 3),   // 8: n-resize (arrow tip)
    (12, 20),  // 9: s-resize (arrow tip)
    (20, 12),  // 10: e-resize (arrow tip)
    (3, 12),   // 11: w-resize (arrow tip)
    (19, 5),   // 12: ne-resize (arrow tip)
    (4, 4),    // 13: nw-resize (arrow tip)
    (19, 19),  // 14: se-resize (arrow tip)
    (4, 19),   // 15: sw-resize (arrow tip)
    (12, 12),  // 16: ew-resize (center)
    (12, 12),  // 17: ns-resize (center)
    (12, 12),  // 18: crosshair (center)
    (12, 12),  // 19: not-allowed (center)
    (3, 3),    // 20: copy (arrow tip)
    (3, 3),    // 21: alias (arrow tip)
    (12, 12),  // 22: zoom-in (center)
    (12, 12),  // 23: zoom-out (center)
];

/// Map wp_cursor_shape_manager_v1 shape_id to our bitmap index.
pub fn shape_id_to_index(shape_id: u32) -> usize {
    match shape_id {
        0 => 0,   // default
        1 => 1,   // pointer (hand)
        2 => 5,   // grab
        3 => 6,   // grabbing
        4 => 12,  // ne-resize
        5 => 13,  // nw-resize
        6 => 14,  // se-resize
        7 => 15,  // sw-resize
        8 => 8,   // n-resize
        9 => 9,   // s-resize
        10 => 10, // e-resize
        11 => 11, // w-resize
        12 => 16, // ew-resize
        13 => 17, // ns-resize
        14 => 12, // nesw-resize (reuse ne)
        15 => 13, // nwse-resize (reuse nw)
        16 => 18, // col-resize (reuse crosshair)
        17 => 18, // row-resize (reuse crosshair)
        18 => 19, // all-scroll (reuse not-allowed)
        19 => 2,  // context-menu (reuse text)
        20 => 3,  // help (reuse wait)
        21 => 22, // zoom-in
        22 => 23, // zoom-out
        _ => 0,   // default
    }
}

// ─── Shape Masks (each 24x24 = 576 pixels) ─────────────────────────

macro_rules! arrow_mask {
    ($dx:expr, $dy:expr, $w:expr, $h:expr, $hw:expr, $hh:expr) => {{
        let mut m = [false; 576];
        let mut i = 0;
        while i < 576 {
            let x = i % 24;
            let y = i / 24;
            m[i] = (x >= $dx && x < $dx + $hw && y >= $dy && y < $dy + $hh)
                || (x.saturating_sub(2) >= $dx && x <= $dx + $hw + 1 && y >= $dy && y <= $dy + 2)
                || (x.saturating_sub(1) >= $dx && x <= $dx + $hw && y >= $dy && y <= $dy + 1);
            i += 1;
        }
        m
    }};
}

const ARROW_MASK: [bool; 576] = arrow_mask!(2, 0, 10, 22, 1, 1);
// Arrow pointing up
const ARROW_UP_MASK: [bool; 576] = arrow_mask!(10, 0, 4, 22, 8, 2);
// Arrow pointing down
const ARROW_DOWN_MASK: [bool; 576] = arrow_mask!(10, 0, 4, 22, 8, 2);
// Arrow pointing right
const ARROW_RIGHT_MASK: [bool; 576] = arrow_mask!(0, 10, 22, 4, 2, 8);
// Arrow pointing left
const ARROW_LEFT_MASK: [bool; 576] = arrow_mask!(0, 10, 22, 4, 2, 8);
// Double arrow horizontal
const ARROW_EW_MASK: [bool; 576] = arrow_mask!(0, 10, 22, 4, 2, 8);
// Double arrow vertical
const ARROW_NS_MASK: [bool; 576] = arrow_mask!(10, 0, 4, 22, 8, 2);

// NE arrow (diagonal up-right)
const ARROW_NE_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        let d = x as i32 + y as i32;
        m[i] = (d >= 14 && d <= 20)
            || (x >= 18 && x <= 23 && y >= 0 && y <= 5)
            || (x >= 18 && x <= 23 && y >= 0 && y <= 2);
        i += 1;
    }
    m
};

// NW arrow (diagonal up-left)
const ARROW_NW_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        let d = x as i32 - y as i32;
        m[i] = (d >= -3 && d <= 3)
            || (x >= 0 && x <= 5 && y >= 0 && y <= 5)
            || (x >= 0 && x <= 2 && y >= 0 && y <= 2);
        i += 1;
    }
    m
};

// SE arrow (diagonal down-right)
const ARROW_SE_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        let d = x as i32 - y as i32;
        m[i] = (d >= -3 && d <= 3)
            || (x >= 18 && x <= 23 && y >= 18 && y <= 23)
            || (x >= 21 && x <= 23 && y >= 21 && y <= 23);
        i += 1;
    }
    m
};

// SW arrow (diagonal down-left)
const ARROW_SW_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        let d = x as i32 + y as i32;
        m[i] = (d >= 30 && d <= 36)
            || (x >= 0 && x <= 5 && y >= 18 && y <= 23)
            || (x >= 0 && x <= 2 && y >= 21 && y <= 23);
        i += 1;
    }
    m
};

// Hand/pointer
const HAND_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        m[i] = (x >= 6 && x <= 14 && y >= 8 && y <= 23)
            || (x >= 7 && x <= 13 && y >= 6 && y <= 7)
            || (x >= 8 && x <= 12 && y >= 4 && y <= 5)
            || (x >= 9 && x <= 11 && y >= 2 && y <= 3)
            || (x == 10 && y >= 0 && y <= 1)
            || (x >= 4 && x <= 16 && y >= 22 && y <= 23);
        i += 1;
    }
    m
};

// I-beam (text)
const IBEAM_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        m[i] = (x >= 10 && x <= 13 && y >= 2 && y <= 21)
            || (x >= 8 && x <= 15 && y >= 2 && y <= 3)
            || (x >= 8 && x <= 15 && y >= 20 && y <= 21);
        i += 1;
    }
    m
};

// Watch/wait (circle)
const WATCH_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        let dx = x as i32 - 11;
        let dy = y as i32 - 11;
        let dist_sq = dx * dx + dy * dy;
        m[i] = (dist_sq >= 36 && dist_sq <= 64)
            || (x >= 10 && x <= 12 && y >= 10 && y <= 12);
        i += 1;
    }
    m
};

// Open hand (grab)
const OPEN_HAND_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        m[i] = (x >= 5 && x <= 18 && y >= 10 && y <= 20)
            || (x >= 6 && x <= 17 && y >= 8 && y <= 9)
            || (x >= 7 && x <= 16 && y >= 6 && y <= 7)
            || (x >= 8 && x <= 15 && y >= 4 && y <= 5)
            || (x >= 9 && x <= 14 && y >= 2 && y <= 3)
            || (x == 11 && y >= 0 && y <= 1);
        i += 1;
    }
    m
};

// Closed hand (grabbing)
const CLOSED_HAND_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        m[i] = (x >= 5 && x <= 18 && y >= 8 && y <= 18)
            || (x >= 6 && x <= 17 && y >= 6 && y <= 7)
            || (x >= 7 && x <= 16 && y >= 4 && y <= 5)
            || (x >= 8 && x <= 15 && y >= 2 && y <= 3)
            || (x == 11 && y >= 0 && y <= 1);
        i += 1;
    }
    m
};

// Move (four-way arrow)
const MOVE_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        m[i] = (x >= 10 && x <= 13 && y >= 0 && y <= 23)
            || (x >= 0 && x <= 23 && y >= 10 && y <= 13)
            || (x >= 7 && x <= 16 && y >= 0 && y <= 3)
            || (x >= 7 && x <= 16 && y >= 20 && y <= 23)
            || (x >= 0 && x <= 3 && y >= 7 && y <= 16)
            || (x >= 20 && x <= 23 && y >= 7 && y <= 16);
        i += 1;
    }
    m
};

// Crosshair
const CROSSHAIR_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        m[i] = (x >= 11 && x <= 12 && y >= 2 && y <= 21)
            || (x >= 2 && x <= 21 && y >= 11 && y <= 12);
        i += 1;
    }
    m
};

// Not-allowed (circle with slash)
const NOT_ALLOWED_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        let dx = x as i32 - 11;
        let dy = y as i32 - 11;
        let dist_sq = dx * dx + dy * dy;
        m[i] = (dist_sq >= 36 && dist_sq <= 64)
            || (x + y >= 16 && x + y <= 22);
        i += 1;
    }
    m
};

// Arrow with plus (copy)
const ARROW_PLUS_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        m[i] = (x >= 2 && x <= 11 && y >= 0 && y <= 21)
            || (x >= 3 && x <= 10 && y >= 0 && y <= 1)
            || (x == 2 && y >= 1 && y <= 21)
            || (x >= 15 && x <= 21 && y >= 3 && y <= 5)
            || (x >= 17 && x <= 19 && y >= 1 && y <= 7);
        i += 1;
    }
    m
};

// Arrow with curved arrow (alias)
const ARROW_CURVED_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        m[i] = (x >= 2 && x <= 11 && y >= 0 && y <= 21)
            || (x >= 3 && x <= 10 && y >= 0 && y <= 1)
            || (x == 2 && y >= 1 && y <= 21)
            || (x >= 15 && x <= 21 && y >= 3 && y <= 5)
            || (x >= 17 && x <= 19 && y >= 1 && y <= 7)
            || (x == 21 && y >= 7 && y <= 10)
            || (x == 20 && y >= 10 && y <= 13);
        i += 1;
    }
    m
};

// Circle with plus (zoom-in)
const CIRCLE_PLUS_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        let dx = x as i32 - 11;
        let dy = y as i32 - 11;
        let dist_sq = dx * dx + dy * dy;
        m[i] = (dist_sq >= 36 && dist_sq <= 64)
            || (x >= 10 && x <= 12 && y >= 4 && y <= 18)
            || (x >= 4 && x <= 18 && y >= 10 && y <= 12);
        i += 1;
    }
    m
};

// Circle with minus (zoom-out)
const CIRCLE_MINUS_MASK: [bool; 576] = {
    let mut m = [false; 576];
    let mut i = 0;
    while i < 576 {
        let x = i % 24;
        let y = i / 24;
        let dx = x as i32 - 11;
        let dy = y as i32 - 11;
        let dist_sq = dx * dx + dy * dy;
        m[i] = (dist_sq >= 36 && dist_sq <= 64)
            || (x >= 4 && x <= 18 && y >= 10 && y <= 12);
        i += 1;
    }
    m
};

/// Cursor state — position and current shape.
pub struct Cursor {
    pub x: i32,
    pub y: i32,
    pub shape_index: usize,
    pub visible: bool,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            shape_index: 0,
            visible: true,
        }
    }

    /// Set the cursor shape by wp_cursor_shape_manager_v1 shape_id.
    pub fn set_shape(&mut self, shape_index: usize) {
        self.shape_index = shape_index;
    }

    /// Draw the current cursor shape into the scanout buffer.
    pub fn draw(
        &self,
        pixels: &mut [u8],
        fb_width: u32,
        fb_height: u32,
        fb_stride: u32,
    ) {
        if !self.visible {
            return;
        }

        let (hx, hy) = CURSOR_HOTSPOTS[self.shape_index];
        let origin_x = self.x - hx;
        let origin_y = self.y - hy;

        let bitmap = &CURSOR_BITMAPS[self.shape_index];

        for cy in 0..CURSOR_SIZE {
            for cx in 0..CURSOR_SIZE {
                let dx = origin_x + cx as i32;
                let dy = origin_y + cy as i32;

                if dx < 0 || dy < 0 || dx as u32 >= fb_width || dy as u32 >= fb_height {
                    continue;
                }

                let src_off = (cy * CURSOR_SIZE + cx) * 4;
                let alpha = bitmap[src_off + 3];
                if alpha == 0 {
                    continue;
                }

                let dst_off = (dy as u32 * fb_stride + dx as u32 * 4) as usize;
                if dst_off + 4 > pixels.len() {
                    continue;
                }

                if alpha == 255 {
                    pixels[dst_off] = bitmap[src_off];
                    pixels[dst_off + 1] = bitmap[src_off + 1];
                    pixels[dst_off + 2] = bitmap[src_off + 2];
                } else {
                    let a_inv = 255u32 - alpha as u32;
                    pixels[dst_off] =
                        ((bitmap[src_off] as u32 * alpha as u32 + pixels[dst_off] as u32 * a_inv) / 255) as u8;
                    pixels[dst_off + 1] =
                        ((bitmap[src_off + 1] as u32 * alpha as u32 + pixels[dst_off + 1] as u32 * a_inv) / 255) as u8;
                    pixels[dst_off + 2] =
                        ((bitmap[src_off + 2] as u32 * alpha as u32 + pixels[dst_off + 2] as u32 * a_inv) / 255) as u8;
                }
                pixels[dst_off + 3] = 0xff;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_default_state() {
        let cursor = Cursor::new();
        assert_eq!(cursor.x, 0);
        assert_eq!(cursor.y, 0);
        assert_eq!(cursor.shape_index, 0);
        assert!(cursor.visible);
    }

    #[test]
    fn test_cursor_set_shape() {
        let mut cursor = Cursor::new();
        cursor.set_shape(1); // pointer
        assert_eq!(cursor.shape_index, 1);
        cursor.set_shape(5); // grab
        assert_eq!(cursor.shape_index, 5);
    }

    #[test]
    fn test_cursor_shape_index_mapping() {
        assert_eq!(shape_id_to_index(0), 0);  // default
        assert_eq!(shape_id_to_index(1), 1);  // pointer
        assert_eq!(shape_id_to_index(2), 5);  // grab
        assert_eq!(shape_id_to_index(3), 6);  // grabbing
        assert_eq!(shape_id_to_index(9), 9);  // s-resize
        assert_eq!(shape_id_to_index(12), 16); // ew-resize
        assert_eq!(shape_id_to_index(18), 19); // all-scroll -> not-allowed
        assert_eq!(shape_id_to_index(99), 0); // unknown → default
    }

    #[test]
    fn test_cursor_draw_visible() {
        let mut cursor = Cursor::new();
        cursor.x = 50;
        cursor.y = 50;
        let mut fb = vec![0u8; 100 * 100 * 4];
        cursor.draw(&mut fb, 100, 100, 400);
        // Should have drawn some non-zero pixels
        let has_content = fb.iter().any(|&p| p != 0);
        assert!(has_content);
    }

    #[test]
    fn test_cursor_draw_hidden() {
        let mut cursor = Cursor::new();
        cursor.visible = false;
        cursor.x = 50;
        cursor.y = 50;
        let mut fb = vec![0u8; 100 * 100 * 4];
        cursor.draw(&mut fb, 100, 100, 400);
        // Should remain all zeros
        let all_zero = fb.iter().all(|&p| p == 0);
        assert!(all_zero);
    }

    #[test]
    fn test_cursor_bitmap_size() {
        // Each bitmap should be 24x24x4 = 2304 bytes
        for (i, bitmap) in CURSOR_BITMAPS.iter().enumerate() {
            assert_eq!(
                bitmap.len(),
                2304,
                "Cursor bitmap {} has wrong size: {}",
                i,
                bitmap.len()
            );
        }
    }

    #[test]
    fn test_cursor_hotspots_count() {
        assert_eq!(CURSOR_HOTSPOTS.len(), CURSOR_BITMAPS.len());
    }

    #[test]
    fn test_cursor_hotspots_within_bounds() {
        for (i, &(hx, hy)) in CURSOR_HOTSPOTS.iter().enumerate() {
            assert!(
                hx >= 0 && hx < CURSOR_SIZE as i32,
                "Cursor {} hotspot_x {} out of bounds [0, {})",
                i, hx, CURSOR_SIZE
            );
            assert!(
                hy >= 0 && hy < CURSOR_SIZE as i32,
                "Cursor {} hotspot_y {} out of bounds [0, {})",
                i, hy, CURSOR_SIZE
            );
        }
    }

    #[test]
    fn test_cursor_draw_offscreen() {
        let cursor = Cursor { x: -100, y: -100, shape_index: 0, visible: true };
        let mut fb = vec![0u8; 100 * 100 * 4];
        cursor.draw(&mut fb, 100, 100, 400);
        let all_zero = fb.iter().all(|&p| p == 0);
        assert!(all_zero);
    }
}
