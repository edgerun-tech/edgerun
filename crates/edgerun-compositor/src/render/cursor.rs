//! Cursor rendering — software cursor with built-in shapes.
//!
//! Draws cursor shapes directly into the scanout buffer.

/// Cursor shape definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    Default,
    Pointer,
    Text,
    Grab,
    Grabbing,
    Move,
    ResizeN,
    ResizeS,
    ResizeE,
    ResizeW,
    ResizeNE,
    ResizeNW,
    ResizeSE,
    ResizeSW,
    ResizeEW,
    ResizeNS,
    ResizeNESW,
    ResizeNWSE,
    Wait,
    Progress,
    Help,
    NotAllowed,
    Copy,
    Alias,
    Cell,
    Crosshair,
    ZoomIn,
    ZoomOut,
}

impl CursorShape {
    pub fn from_name(name: &str) -> Self {
        match name {
            "default" | "left_ptr" => Self::Default,
            "pointer" | "hand1" | "hand2" | "pointing_hand" => Self::Pointer,
            "text" | "xterm" | "ibeam" => Self::Text,
            "grab" | "openhand" => Self::Grab,
            "grabbing" | "closedhand" | "dnd-no-drop" => Self::Grabbing,
            "move" | "fleur" | "dnd-move" => Self::Move,
            "copy" | "dnd-copy" => Self::Copy,
            "alias" | "dnd-link" => Self::Alias,
            "cell" => Self::Cell,
            "crosshair" | "cross" => Self::Crosshair,
            "help" | "question_arrow" => Self::Help,
            "not-allowed" | "forbidden" => Self::NotAllowed,
            "progress" | "left_ptr_watch" => Self::Progress,
            "wait" | "watch" | "clock" => Self::Wait,
            "n-resize" | "top_side" => Self::ResizeN,
            "s-resize" | "bottom_side" => Self::ResizeS,
            "e-resize" | "right_side" => Self::ResizeE,
            "w-resize" | "left_side" => Self::ResizeW,
            "ne-resize" | "top_right_corner" => Self::ResizeNE,
            "nw-resize" | "top_left_corner" => Self::ResizeNW,
            "se-resize" | "bottom_right_corner" => Self::ResizeSE,
            "sw-resize" | "bottom_left_corner" => Self::ResizeSW,
            "ew-resize" | "h_double_arrow" | "col-resize" => Self::ResizeEW,
            "ns-resize" | "v_double_arrow" | "row-resize" => Self::ResizeNS,
            "nesw-resize" => Self::ResizeNESW,
            "nwse-resize" => Self::ResizeNWSE,
            "zoom-in" => Self::ZoomIn,
            "zoom-out" => Self::ZoomOut,
            _ => Self::Default,
        }
    }
}

/// Cursor state — position, shape, hotspot, and surface buffer.
pub struct Cursor {
    pub x: i32,
    pub y: i32,
    pub hotspot_x: i32,
    pub hotspot_y: i32,
    pub shape: CursorShape,
    /// Optional cursor surface (client-provided).
    pub surface_id: Option<u32>,
    pub surface_width: i32,
    pub surface_height: i32,
    pub visible: bool,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            hotspot_x: 0,
            hotspot_y: 0,
            shape: CursorShape::Default,
            surface_id: None,
            surface_width: 0,
            surface_height: 0,
            visible: true,
        }
    }

    /// Draw the cursor onto the scanout buffer.
    pub fn draw(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        stride: u32,
        cursor_surface_data: Option<&[u8]>,
        cursor_surface_stride: i32,
    ) {
        if !self.visible {
            return;
        }

        // If client provided a cursor surface, use it
        if let Some(surface_id) = self.surface_id {
            if surface_id != 0 && self.surface_width > 0 && self.surface_height > 0 {
                if let Some(data) = cursor_surface_data {
                    self.draw_surface(
                        pixels, width, height, stride,
                        data, cursor_surface_stride,
                    );
                    return;
                }
            }
        }

        // Otherwise draw built-in shape
        self.draw_builtin_shape(pixels, width, height, stride);
    }

    fn draw_surface(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        stride: u32,
        surface_data: &[u8],
        surface_stride: i32,
    ) {
        let w = self.surface_width.min(64) as i32;
        let h = self.surface_height.min(64) as i32;
        let origin_x = self.x - self.hotspot_x;
        let origin_y = self.y - self.hotspot_y;

        for sy in 0..h {
            for sx in 0..w {
                let dx = origin_x + sx;
                let dy = origin_y + sy;

                if dx < 0 || dy < 0 || dx as u32 >= width || dy as u32 >= height {
                    continue;
                }

                let src_off = (sy as usize * surface_stride as usize + sx as usize * 4);
                if src_off + 4 > surface_data.len() {
                    continue;
                }

                let dst_off = (dy as u32 * stride + dx as u32 * 4) as usize;
                if dst_off + 4 > pixels.len() {
                    continue;
                }

                let alpha = surface_data[src_off + 3];
                if alpha == 0 {
                    continue;
                }

                // Integer alpha blending: dst = (src * alpha + dst * (255 - alpha)) / 255
                if alpha == 255 {
                    pixels[dst_off] = surface_data[src_off];
                    pixels[dst_off + 1] = surface_data[src_off + 1];
                    pixels[dst_off + 2] = surface_data[src_off + 2];
                } else {
                    let a_inv = 255u32 - alpha as u32;
                    pixels[dst_off] =     ((surface_data[src_off] as u32 * alpha as u32 + pixels[dst_off] as u32 * a_inv) / 255) as u8;
                    pixels[dst_off + 1] = ((surface_data[src_off + 1] as u32 * alpha as u32 + pixels[dst_off + 1] as u32 * a_inv) / 255) as u8;
                    pixels[dst_off + 2] = ((surface_data[src_off + 2] as u32 * alpha as u32 + pixels[dst_off + 2] as u32 * a_inv) / 255) as u8;
                }
                pixels[dst_off + 3] = 0xFF;
            }
        }
    }

    fn draw_builtin_shape(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        stride: u32,
    ) {
        // Draw a simple cursor shape (default: arrow pointer)
        // 16x16 or 24x24 arrow
        let cursor_w = 24i32;
        let cursor_h = 24i32;
        let origin_x = self.x - self.hotspot_x;
        let origin_y = self.y - self.hotspot_y;

        for cy in 0..cursor_h {
            for cx in 0..cursor_w {
                let dx = origin_x + cx;
                let dy = origin_y + cy;

                if dx < 0 || dy < 0 || dx as u32 >= width || dy as u32 >= height {
                    continue;
                }

                let is_cursor = self.is_pixel_on_shape(cx, cy);
                if !is_cursor {
                    continue;
                }

                let dst_off = (dy as u32 * stride + dx as u32 * 4) as usize;
                if dst_off + 4 > pixels.len() {
                    continue;
                }

                // White cursor with black outline
                // Simple outline: check if on edge
                let is_edge = self.is_edge(cx, cy);

                if is_edge {
                    // Black outline
                    pixels[dst_off] = 0;
                    pixels[dst_off + 1] = 0;
                    pixels[dst_off + 2] = 0;
                } else {
                    // White fill
                    pixels[dst_off] = 0xFF;
                    pixels[dst_off + 1] = 0xFF;
                    pixels[dst_off + 2] = 0xFF;
                }
                pixels[dst_off + 3] = 0xFF;
            }
        }
    }

    fn is_pixel_on_shape(&self, x: i32, y: i32) -> bool {
        // Simple arrow pointer shape (24x24)
        // This is a simplified bitmap representation
        let shape = [
            // y=0..23, each row is a bitmask for x
            0b100000000000000000000000, // 0
            0b110000000000000000000000, // 1
            0b101000000000000000000000, // 2
            0b100100000000000000000000, // 3
            0b100010000000000000000000, // 4
            0b100001000000000000000000, // 5
            0b100000100000000000000000, // 6
            0b100000010000000000000000, // 7
            0b100000001000000000000000, // 8
            0b100000000100000000000000, // 9
            0b100000000010000000000000, // 10
            0b100000000001000000000000, // 11
            0b100000000000100000000000, // 12
            0b100000000000010000000000, // 13
            0b100000000000001000000000, // 14
            0b100000000000000100000000, // 15
            0b100000000000000010000000, // 16
            0b100000000000000001000000, // 17
            0b111111000000000000100000, // 18
            0b100000100000000001000000, // 19
            0b100000010000000010000000, // 20
            0b100000001000000100000000, // 21
            0b100000000100001000000000, // 22
            0b000000000000000000000000, // 23 (empty)
        ];

        if y < 0 || y >= 24 || x < 0 || x >= 24 {
            return false;
        }

        (shape[y as usize] & (1u32 << (23 - x))) != 0
    }

    fn is_edge(&self, x: i32, y: i32) -> bool {
        // A pixel is on the edge if any of its 8 neighbors is off
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if !self.is_pixel_on_shape(x + dx, y + dy) {
                    return true;
                }
            }
        }
        false
    }
}
