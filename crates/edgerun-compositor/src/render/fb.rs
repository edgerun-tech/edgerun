//! Framebuffer abstraction.

/// A render target — a contiguous region of RGBA8888 pixels.
pub struct Framebuffer {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32, // bytes per row
}

/// A framebuffer backed by a raw mutable pointer (e.g., mmap'd DRM dumb buffer).
/// This does NOT own the memory — lifetime must be managed externally.
pub struct RawFramebuffer<'a> {
    pub pixels: &'a mut [u8],
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

impl Framebuffer {
    /// Create a new framebuffer.
    pub fn new(width: u32, height: u32) -> Self {
        let stride = width * 4;
        Self {
            pixels: vec![0u8; (stride * height) as usize],
            width,
            height,
            stride,
        }
    }

    /// Clear to a solid color (RGBA).
    pub fn clear(&mut self, r: u8, g: u8, b: u8, a: u8) {
        // Fill with BGRA (little-endian ARGB8888)
        let pixel = u32::from_le_bytes([b, g, r, a]);
        let pixels_u32 = unsafe {
            std::slice::from_raw_parts_mut(
                self.pixels.as_mut_ptr() as *mut u32,
                (self.pixels.len() / 4) as usize,
            )
        };
        for p in pixels_u32.iter_mut() {
            *p = pixel;
        }
    }

    /// Get a mutable reference to a pixel at (x, y).
    /// Returns None if out of bounds.
    pub fn pixel_mut(&mut self, x: u32, y: u32) -> Option<&mut [u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = (y * self.stride + x * 4) as usize;
        if offset + 4 > self.pixels.len() {
            return None;
        }
        let slice = &mut self.pixels[offset..offset + 4];
        Some(unsafe { &mut *(slice.as_mut_ptr() as *mut [u8; 4]) })
    }

    /// Get the raw pixel bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.pixels
    }

    /// Get mutable raw pixel bytes.
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }
}

impl<'a> RawFramebuffer<'a> {
    /// Wrap existing memory (e.g., mmap'd DRM dumb buffer) as a framebuffer.
    /// This does NOT take ownership of the memory — caller must ensure it lives long enough.
    pub fn from_raw_pixels(
        pixels: &'a mut [u8],
        width: u32,
        height: u32,
        stride: u32,
    ) -> Self {
        Self {
            pixels,
            width,
            height,
            stride,
        }
    }

    /// Clear to a solid color (RGBA).
    pub fn clear(&mut self, r: u8, g: u8, b: u8, a: u8) {
        let pixel = u32::from_le_bytes([b, g, r, a]);
        let pixels_u32 = unsafe {
            std::slice::from_raw_parts_mut(
                self.pixels.as_mut_ptr() as *mut u32,
                (self.pixels.len() / 4) as usize,
            )
        };
        for p in pixels_u32.iter_mut() {
            *p = pixel;
        }
    }

    /// Get a mutable reference to a pixel at (x, y).
    /// Returns None if out of bounds.
    pub fn pixel_mut(&mut self, x: u32, y: u32) -> Option<&mut [u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = (y * self.stride + x * 4) as usize;
        if offset + 4 > self.pixels.len() {
            return None;
        }
        let slice = &mut self.pixels[offset..offset + 4];
        Some(unsafe { &mut *(slice.as_mut_ptr() as *mut [u8; 4]) })
    }

    /// Get the raw pixel bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.pixels
    }

    /// Get mutable raw pixel bytes.
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.pixels
    }
}
