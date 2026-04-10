//! Framebuffer — raw pixel buffer (XRGB8888, matching DRM dumb buffer layout).
//! DO NOT EDIT. Regenerate with: scripts/generate_rasterizer.py

/// XRGB8888 framebuffer. In little-endian memory: [B, G, R, X].
pub struct Framebuffer {
    pub pixels: &'static mut [u8],
    pub width: u32,
    pub height: u32,
    pub stride: u32, // bytes per row
}

impl Framebuffer {
    pub fn new(pixels: &'static mut [u8], width: u32, height: u32) -> Self {
        let stride = width * 4;
        Self { pixels, width, height, stride }
    }

    #[inline]
    pub fn fill_solid(&mut self, x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8) {
        let x0 = x.min(self.width - 1);
        let y0 = y.min(self.height - 1);
        let x1 = (x + w).min(self.width);
        let y1 = (y + h).min(self.height);
        for cy in y0..y1 {
            let row = (cy * self.stride) as usize;
            for cx in x0..x1 {
                let i = row + (cx as usize) * 4;
                self.pixels[i]     = b; // B
                self.pixels[i + 1] = g; // G
                self.pixels[i + 2] = r; // R
                self.pixels[i + 3] = 0xFF; // X (unused)
            }
        }
    }

    #[inline]
    pub fn fill_alpha(&mut self, x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8, alpha: u8) {
        if alpha == 0 { return; }
        if alpha == 255 {
            self.fill_solid(x, y, w, h, r, g, b);
            return;
        }
        let a = alpha as u32;
        let inv_a = 255 - a;
        let x0 = x.min(self.width - 1);
        let y0 = y.min(self.height - 1);
        let x1 = (x + w).min(self.width);
        let y1 = (y + h).min(self.height);
        for cy in y0..y1 {
            let row = (cy * self.stride) as usize;
            for cx in x0..x1 {
                let i = row + (cx as usize) * 4;
                let db = self.pixels[i] as u32;
                let dg = self.pixels[i + 1] as u32;
                let dr = self.pixels[i + 2] as u32;
                self.pixels[i]     = ((b as u32 * a + db * inv_a) / 255) as u8;
                self.pixels[i + 1] = ((g as u32 * a + dg * inv_a) / 255) as u8;
                self.pixels[i + 2] = ((r as u32 * a + dr * inv_a) / 255) as u8;
            }
        }
    }

    #[inline]
    pub fn set_pixel_alpha(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, alpha: u8) {
        if x >= self.width || y >= self.height || alpha == 0 { return; }
        if alpha == 255 {
            let i = (y * self.stride + x * 4) as usize;
            self.pixels[i] = b; self.pixels[i+1] = g; self.pixels[i+2] = r; return;
        }
        let i = (y * self.stride + x * 4) as usize;
        let a = alpha as u32;
        let inv_a = 255 - a;
        self.pixels[i]     = ((b as u32 * a + self.pixels[i] as u32 * inv_a) / 255) as u8;
        self.pixels[i + 1] = ((g as u32 * a + self.pixels[i+1] as u32 * inv_a) / 255) as u8;
        self.pixels[i + 2] = ((r as u32 * a + self.pixels[i+2] as u32 * inv_a) / 255) as u8;
    }

    pub fn clear(&mut self) {
        let bg = [0x1a, 0x1a, 0x2e, 0xFF]; // dark blue-gray
        for i in (0..self.pixels.len()).step_by(4) {
            self.pixels[i]     = bg[0];
            self.pixels[i + 1] = bg[1];
            self.pixels[i + 2] = bg[2];
            self.pixels[i + 3] = bg[3];
        }
    }
}
