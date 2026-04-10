//! Server-side rendering functions for the compositor.

use crate::compositor::surface::{DamageRect, SurfaceBuffer, SurfaceTree};
use crate::compositor::shell::Shell;
use crate::drm;
use crate::drm::device::DrmDevice;
use crate::drm::dumb::DumbBuffer;
use crate::drm::kms;
use crate::render::cursor::Cursor;
use crate::render::shm::ShmManager;

/// Accumulated damage regions for incremental rendering.
pub struct DamageAccumulator {
    rects: Vec<DamageRect>,
    /// Whether to render the entire screen next frame.
    pub damage_all: bool,
}

impl DamageAccumulator {
    pub fn new() -> Self {
        Self { rects: Vec::new(), damage_all: true }
    }

    /// Add a damage rectangle.
    pub fn add(&mut self, rect: DamageRect) {
        self.rects.push(rect);
    }

    /// Mark the entire output as damaged.
    pub fn mark_all(&mut self) {
        self.damage_all = true;
    }

    /// Take the current damage rectangles and reset for the next frame.
    pub fn take_rects(&mut self) -> Vec<DamageRect> {
        std::mem::take(&mut self.rects)
    }

    /// Check if there is any pending damage.
    pub fn is_dirty(&self) -> bool {
        self.damage_all || !self.rects.is_empty()
    }
}

/// Render all surfaces to the scanout buffer and issue a page flip.
///
/// If `damage` is `None` or `damage_all` is true, the entire framebuffer is cleared.
/// Otherwise, only the damaged regions are re-rendered.
pub fn render_and_flip(
    dumb: &mut DumbBuffer,
    fb_id: u32,
    crtc_id: u32,
    drm_device: &DrmDevice,
    surfaces: &SurfaceTree,
    shell: &Shell,
    cursor: &Cursor,
    shm: &ShmManager,
    damage: &mut DamageAccumulator,
    old_fb_ids: &mut Vec<u32>,
) {
    // Map the dumb buffer once — DumbBuffer::map() caches the mapping internally
    let fb_slice = match dumb.map() {
        Ok(p) => p,
        Err(_) => return,
    };
    let pixels = unsafe { std::slice::from_raw_parts_mut(fb_slice.as_mut_ptr(), fb_slice.len()) };

    let width = dumb.width;
    let height = dumb.height;
    let stride = dumb.pitch;

    if damage.damage_all {
        // Clear entire framebuffer to dark blue-gray
        // XRGB8888 little-endian: [B=0x2e, G=0x1a, R=0x1a, X=0xff] => 0xff1a1a2e
        let pixel_count = (width * height) as usize;
        let pixels_u32 = unsafe {
            std::slice::from_raw_parts_mut(pixels.as_mut_ptr() as *mut u32, pixel_count)
        };
        let bg_pixel = 0xff1a1a2eu32;
        for p in pixels_u32.iter_mut() {
            *p = bg_pixel;
        }
        damage.damage_all = false;
    }

    // Composite surfaces in z-order (toplevels + subsurfaces)
    // Always render surfaces with buffers — surfaces are composited
    // back-to-front onto the cleared framebuffer each frame.
    for toplevel in shell.toplevels_z_order() {
        let surface_id = toplevel.surface_id;
        if let Some(surface) = surfaces.get(surface_id) {
            if surface.buffer.is_none() {
                continue;
            }
            if let Some(ref buf) = surface.buffer {
                blit_surface_buffer(buf, pixels, surface.x, surface.y, width, height, stride, shm);
            }
        }

        // Composite subsurfaces
        for sub in shell.subsurfaces.for_parent(surface_id) {
            if let Some(sub_surface) = surfaces.get(sub.surface_id) {
                if sub_surface.buffer.is_some() {
                    if let Some(ref buf) = sub_surface.buffer {
                        blit_surface_buffer(buf, pixels, sub.x, sub.y, width, height, stride, shm);
                    }
                }
            }
        }
    }

    // Draw cursor
    cursor.draw(pixels, width, height, stride);

    // Page flip to display the rendered buffer
    let new_fb_id = match dumb.add_fb() {
        Ok(id) => id,
        Err(_) => return,
    };

    // Track old FB IDs for cleanup (avoid resource leak)
    if fb_id != 0 {
        old_fb_ids.push(fb_id);
    }

    let user_data = kms::next_flip_serial();
    let _ = kms::page_flip(drm_device.as_raw_fd(), crtc_id, new_fb_id, user_data);
}

/// Blit a surface buffer onto the pixel array.
///
/// Uses cached SHM pool mappings instead of re-mmap/munmap per call.
/// Uses integer alpha blending for performance.
fn blit_surface_buffer(
    buf: &SurfaceBuffer,
    pixels: &mut [u8],
    origin_x: i32,
    origin_y: i32,
    output_width: u32,
    output_height: u32,
    output_stride: u32,
    shm: &ShmManager,
) {
    match buf {
        SurfaceBuffer::Shm { pool_fd, offset, width: buf_w, height: buf_h, stride: buf_stride, format } => {
            // Use cached SHM pool mapping via O(1) fd lookup
            if let Some(pool) = shm.get_pool_by_fd(*pool_fd) {
                let len = (*buf_stride as usize) * (*buf_h as usize);
                if let Some(data) = pool.read(*offset as usize, len) {
                    blit_pixels(
                        data, *format, *buf_w as u32, *buf_h as u32, *buf_stride as u32,
                        origin_x, origin_y, output_width, output_height, output_stride, pixels,
                    );
                }
            } else {
                // Fallback: mmap temporarily if pool not in manager
                let pool_size = match drm::fd_size(*pool_fd) {
                    Ok(s) => s,
                    Err(_) => return,
                };
                let mapping = unsafe {
                    libc::mmap(std::ptr::null_mut(), pool_size, libc::PROT_READ,
                               libc::MAP_SHARED, *pool_fd, 0)
                };
                if mapping == libc::MAP_FAILED { return; }
                let pool_data = unsafe { std::slice::from_raw_parts(mapping as *const u8, pool_size) };
                let off = *offset as usize;
                let len = (*buf_stride as usize) * (*buf_h as usize);
                if off + len <= pool_data.len() {
                    let data = &pool_data[off..off + len];
                    blit_pixels(
                        data, *format, *buf_w as u32, *buf_h as u32, *buf_stride as u32,
                        origin_x, origin_y, output_width, output_height, output_stride, pixels,
                    );
                }
                unsafe { libc::munmap(mapping, pool_size) };
            }
        }
        SurfaceBuffer::DmaBuf { width: buf_w, height: buf_h, format, plane_fds, offsets, strides, num_planes: _ } => {
            if plane_fds.is_empty() || plane_fds[0] < 0 { return; }
            let fd = plane_fds[0];
            let buf_stride = strides.first().copied().unwrap_or(*buf_w as u32 * 4);
            let offset = offsets.first().copied().unwrap_or(0);

            let pool_size = match drm::fd_size(fd) {
                Ok(s) => s,
                Err(_) => return,
            };
            let mapping = unsafe {
                libc::mmap(std::ptr::null_mut(), pool_size, libc::PROT_READ,
                           libc::MAP_SHARED, fd, 0)
            };
            if mapping == libc::MAP_FAILED { return; }

            let buf_data = unsafe { std::slice::from_raw_parts(mapping as *const u8, pool_size) };
            let off = offset as usize;
            let len = (buf_stride as usize) * (*buf_h as usize);
            if off + len > buf_data.len() {
                unsafe { libc::munmap(mapping, pool_size) };
                return;
            }
            let data = &buf_data[off..off + len];

            blit_pixels(
                data, *format, *buf_w as u32, *buf_h as u32, buf_stride,
                origin_x, origin_y, output_width, output_height, output_stride, pixels,
            );

            unsafe { libc::munmap(mapping, pool_size) };
        }
        SurfaceBuffer::Dumb { .. } => {}
        SurfaceBuffer::Null => {}
    }
}

/// Shared pixel blitting logic — handles format conversion and clipping.
/// Uses integer alpha blending for performance (no f32).
fn blit_pixels(
    src_data: &[u8],
    format: u32,
    buf_w: u32,
    buf_h: u32,
    buf_stride: u32,
    origin_x: i32,
    origin_y: i32,
    output_width: u32,
    output_height: u32,
    output_stride: u32,
    pixels: &mut [u8],
) {
    for sy in 0..buf_h {
        for sx in 0..buf_w {
            let dx = origin_x + sx as i32;
            let dy = origin_y + sy as i32;
            if dx < 0 || dy < 0 || dx as u32 >= output_width || dy as u32 >= output_height {
                continue;
            }
            let src_off = sy as usize * buf_stride as usize + sx as usize * 4;
            if src_off + 4 > src_data.len() { continue; }
            let dst_off = (dy as u32 * output_stride + dx as u32 * 4) as usize;
            if dst_off + 4 > pixels.len() { continue; }

            // Target framebuffer is XRGB8888 (little-endian memory: [B, G, R, X])
            match format {
                0x34325258 /* XRGB8888 */ => {
                    // Same format as framebuffer — direct copy
                    // Source LE memory: [B, G, R, X], Dest LE memory: [B, G, R, X]
                    pixels[dst_off] = src_data[src_off];
                    pixels[dst_off + 1] = src_data[src_off + 1];
                    pixels[dst_off + 2] = src_data[src_off + 2];
                    pixels[dst_off + 3] = 0xff;
                }
                0x34325241 /* ARGB8888 */ => {
                    // ARGB8888 little-endian: [B, G, R, A]
                    // Same RGB order as framebuffer, needs alpha blending
                    let alpha = src_data[src_off + 3];
                    if alpha == 0 { continue; }
                    if alpha == 255 {
                        pixels[dst_off] = src_data[src_off];
                        pixels[dst_off + 1] = src_data[src_off + 1];
                        pixels[dst_off + 2] = src_data[src_off + 2];
                    } else {
                        // Integer alpha blending: dst = (src * alpha + dst * (255 - alpha)) / 255
                        let a_inv = 255u32 - alpha as u32;
                        pixels[dst_off] =     ((src_data[src_off] as u32 * alpha as u32 + pixels[dst_off] as u32 * a_inv) / 255) as u8;
                        pixels[dst_off + 1] = ((src_data[src_off + 1] as u32 * alpha as u32 + pixels[dst_off + 1] as u32 * a_inv) / 255) as u8;
                        pixels[dst_off + 2] = ((src_data[src_off + 2] as u32 * alpha as u32 + pixels[dst_off + 2] as u32 * a_inv) / 255) as u8;
                    }
                    pixels[dst_off + 3] = 0xff;
                }
                0x34324241 /* ABGR8888 */ => {
                    // ABGR8888 little-endian: [R, G, B, A]
                    // Need to swap R↔B to match XRGB8888 framebuffer [B, G, R, X]
                    let alpha = src_data[src_off + 3];
                    if alpha == 0 { continue; }
                    if alpha == 255 {
                        pixels[dst_off] =     src_data[src_off + 2]; // B <- source B (byte 2)
                        pixels[dst_off + 1] = src_data[src_off + 1]; // G <- source G (byte 1)
                        pixels[dst_off + 2] = src_data[src_off];     // R <- source R (byte 0)
                    } else {
                        let a_inv = 255u32 - alpha as u32;
                        // Source: [R, G, B, A] → we need [B, G, R] for dest
                        let src_b = src_data[src_off + 2] as u32;
                        let src_g = src_data[src_off + 1] as u32;
                        let src_r = src_data[src_off] as u32;
                        pixels[dst_off] =     ((src_b * alpha as u32 + pixels[dst_off] as u32 * a_inv) / 255) as u8;
                        pixels[dst_off + 1] = ((src_g * alpha as u32 + pixels[dst_off + 1] as u32 * a_inv) / 255) as u8;
                        pixels[dst_off + 2] = ((src_r * alpha as u32 + pixels[dst_off + 2] as u32 * a_inv) / 255) as u8;
                    }
                    pixels[dst_off + 3] = 0xff;
                }
                _ => {
                    // Default: treat as XRGB8888, direct copy
                    pixels[dst_off] = src_data[src_off];
                    pixels[dst_off + 1] = src_data[src_off + 1];
                    pixels[dst_off + 2] = src_data[src_off + 2];
                    pixels[dst_off + 3] = 0xff;
                }
            }
        }
    }
}
