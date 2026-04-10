//! Server-side rendering functions for the compositor.

use crate::compositor::surface::{DamageRect, SurfaceBuffer, SurfaceTree};
use crate::compositor::shell::Shell;
use crate::drm;
use crate::drm::device::DrmDevice;
use crate::drm::dumb::DumbBuffer;
use crate::drm::kms;
use crate::render::cursor::Cursor;
use crate::render::shm::{ShmManager, read_shm_buffer_with_fallback};

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

    // Composite layer surfaces: background + bottom layers (below toplevels)
    for ls in shell.layer_surfaces_render_order() {
        if ls.layer > 1 { break; } // only background(0) and bottom(1)
        if let Some(surface) = surfaces.get(ls.surface_id) {
            if surface.buffer.is_some() {
                if let Some(ref buf) = surface.buffer {
                    let transform = surface.buffer_transform;
                    blit_surface_buffer(buf, pixels, ls.x, ls.y, width, height, stride, shm, transform);
                }
            }
        }
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
                let transform = surface.buffer_transform;
                blit_surface_buffer(buf, pixels, surface.x, surface.y, width, height, stride, shm, transform);
            }
        }

        // Composite subsurfaces
        for sub in shell.subsurfaces.for_parent(surface_id) {
            if let Some(sub_surface) = surfaces.get(sub.surface_id) {
                if sub_surface.buffer.is_some() {
                    if let Some(ref buf) = sub_surface.buffer {
                        let transform = sub_surface.buffer_transform;
                        blit_surface_buffer(buf, pixels, sub.x, sub.y, width, height, stride, shm, transform);
                    }
                }
            }
        }
    }

    // Composite layer surfaces: top + overlay layers (above toplevels)
    for ls in shell.layer_surfaces_render_order() {
        if ls.layer < 2 { continue; } // only top(2) and overlay(3)
        if let Some(surface) = surfaces.get(ls.surface_id) {
            if surface.buffer.is_some() {
                if let Some(ref buf) = surface.buffer {
                    let transform = surface.buffer_transform;
                    blit_surface_buffer(buf, pixels, ls.x, ls.y, width, height, stride, shm, transform);
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

    // Determine page flip flags based on topmost surface tearing hint.
    // Check the frontmost toplevel for async/tearing preference.
    let mut flip_flags = crate::drm::ioctl::page_flip::PAGE_FLIP_EVENT;
    for &tl_id in &shell.stack {
        if let Some(tl) = shell.toplevels.get(&tl_id) {
            let surface_id = tl.surface_id;
            if let Some(surface) = surfaces.get(surface_id) {
                if surface.buffer.is_some() {
                    if surface.tearing_hint == 2 {
                        flip_flags |= crate::drm::ioctl::page_flip::PAGE_FLIP_ASYNC;
                    }
                    break;
                }
            }
        }
    }

    let user_data = kms::next_flip_serial();
    let _ = kms::page_flip(drm_device.as_raw_fd(), crtc_id, new_fb_id, flip_flags, user_data);
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
    transform: i32,
) {
    match buf {
        SurfaceBuffer::Shm { pool_fd, offset, width: buf_w, height: buf_h, stride: buf_stride, format } => {
            let len = (*buf_stride as usize) * (*buf_h as usize);
            if let Some(result) = read_shm_buffer_with_fallback(shm, *pool_fd, *offset as usize, len) {
                blit_pixels(
                    result.as_bytes(), *format, *buf_w as u32, *buf_h as u32, *buf_stride as u32,
                    origin_x, origin_y, output_width, output_height, output_stride, pixels,
                    transform,
                );
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
                transform,
            );

            unsafe { libc::munmap(mapping, pool_size) };
        }
        SurfaceBuffer::Dumb { .. } => {}
        SurfaceBuffer::Null => {}
    }
}

/// Shared pixel blitting logic — handles format conversion and clipping.
/// Uses integer alpha blending for performance (no f32).
/// Applies buffer transform (rotation/flip) before compositing.
///
/// The format match is hoisted outside the pixel loop for performance —
/// we dispatch to a format-specific inner loop once per surface.
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
    transform: i32,
) {
    // Transform values match Wayland wl_output.transform
    // 0=normal, 1=90deg CW, 2=180deg, 3=270deg
    // 4=flipped, 5=flipped+90, 6=flipped+180, 7=flipped+270
    let rotated = transform == 1 || transform == 3 || transform == 5 || transform == 7;
    let draw_w = if rotated { buf_h } else { buf_w };
    let draw_h = if rotated { buf_w } else { buf_h };

    // Hoist format match outside the per-pixel loop
    match format {
        0x34325258 /* XRGB8888 */ => {
            blit_xrgb8888(src_data, buf_w, buf_h, buf_stride, origin_x, origin_y,
                output_width, output_height, output_stride, pixels, transform, draw_w, draw_h);
        }
        0x34325241 /* ARGB8888 */ => {
            blit_argb8888(src_data, buf_w, buf_h, buf_stride, origin_x, origin_y,
                output_width, output_height, output_stride, pixels, transform, draw_w, draw_h);
        }
        0x34324241 /* ABGR8888 */ => {
            blit_abgr8888(src_data, buf_w, buf_h, buf_stride, origin_x, origin_y,
                output_width, output_height, output_stride, pixels, transform, draw_w, draw_h);
        }
        _ => {
            // Default: treat as XRGB8888, direct copy
            blit_xrgb8888(src_data, buf_w, buf_h, buf_stride, origin_x, origin_y,
                output_width, output_height, output_stride, pixels, transform, draw_w, draw_h);
        }
    }
}

/// Inner loop for XRGB8888 — same format as framebuffer, direct copy.
#[inline]
fn blit_xrgb8888(
    src_data: &[u8],
    buf_w: u32, buf_h: u32, buf_stride: u32,
    origin_x: i32, origin_y: i32,
    output_width: u32, output_height: u32, output_stride: u32,
    pixels: &mut [u8],
    transform: i32,
    draw_w: u32, draw_h: u32,
) {
    for dy in 0..draw_h {
        for dx_local in 0..draw_w {
            let (sx, sy) = match transform {
                0 => (dx_local, dy),
                1 => (buf_w - 1 - dy, dx_local),
                2 => (buf_w - 1 - dx_local, buf_h - 1 - dy),
                3 => (dy, buf_h - 1 - dx_local),
                4 => (buf_w - 1 - dx_local, dy),
                5 => (buf_w - 1 - dy, buf_h - 1 - dx_local),
                6 => (dx_local, buf_h - 1 - dy),
                7 => (dy, dx_local),
                _ => (dx_local, dy),
            };

            let out_x = origin_x + dx_local as i32;
            let out_y = origin_y + dy as i32;
            if out_x < 0 || out_y < 0 || out_x as u32 >= output_width || out_y as u32 >= output_height {
                continue;
            }
            let src_off = sy as usize * buf_stride as usize + sx as usize * 4;
            if src_off + 4 > src_data.len() { continue; }
            let dst_off = (out_y as u32 * output_stride + out_x as u32 * 4) as usize;
            if dst_off + 4 > pixels.len() { continue; }

            pixels[dst_off] = src_data[src_off];
            pixels[dst_off + 1] = src_data[src_off + 1];
            pixels[dst_off + 2] = src_data[src_off + 2];
            pixels[dst_off + 3] = 0xff;
        }
    }
}

/// Inner loop for ARGB8888 — needs alpha blending.
#[inline]
fn blit_argb8888(
    src_data: &[u8],
    buf_w: u32, buf_h: u32, buf_stride: u32,
    origin_x: i32, origin_y: i32,
    output_width: u32, output_height: u32, output_stride: u32,
    pixels: &mut [u8],
    transform: i32,
    draw_w: u32, draw_h: u32,
) {
    for dy in 0..draw_h {
        for dx_local in 0..draw_w {
            let (sx, sy) = match transform {
                0 => (dx_local, dy),
                1 => (buf_w - 1 - dy, dx_local),
                2 => (buf_w - 1 - dx_local, buf_h - 1 - dy),
                3 => (dy, buf_h - 1 - dx_local),
                4 => (buf_w - 1 - dx_local, dy),
                5 => (buf_w - 1 - dy, buf_h - 1 - dx_local),
                6 => (dx_local, buf_h - 1 - dy),
                7 => (dy, dx_local),
                _ => (dx_local, dy),
            };

            let out_x = origin_x + dx_local as i32;
            let out_y = origin_y + dy as i32;
            if out_x < 0 || out_y < 0 || out_x as u32 >= output_width || out_y as u32 >= output_height {
                continue;
            }
            let src_off = sy as usize * buf_stride as usize + sx as usize * 4;
            if src_off + 4 > src_data.len() { continue; }
            let dst_off = (out_y as u32 * output_stride + out_x as u32 * 4) as usize;
            if dst_off + 4 > pixels.len() { continue; }

            let alpha = src_data[src_off + 3];
            if alpha == 0 { continue; }
            if alpha == 255 {
                pixels[dst_off] = src_data[src_off];
                pixels[dst_off + 1] = src_data[src_off + 1];
                pixels[dst_off + 2] = src_data[src_off + 2];
            } else {
                let a_inv = 255u32 - alpha as u32;
                pixels[dst_off] =
                    ((src_data[src_off] as u32 * alpha as u32 + pixels[dst_off] as u32 * a_inv) / 255) as u8;
                pixels[dst_off + 1] =
                    ((src_data[src_off + 1] as u32 * alpha as u32 + pixels[dst_off + 1] as u32 * a_inv) / 255) as u8;
                pixels[dst_off + 2] =
                    ((src_data[src_off + 2] as u32 * alpha as u32 + pixels[dst_off + 2] as u32 * a_inv) / 255) as u8;
            }
            pixels[dst_off + 3] = 0xff;
        }
    }
}

/// Inner loop for ABGR8888 — needs R↔B swap + alpha blending.
#[inline]
fn blit_abgr8888(
    src_data: &[u8],
    buf_w: u32, buf_h: u32, buf_stride: u32,
    origin_x: i32, origin_y: i32,
    output_width: u32, output_height: u32, output_stride: u32,
    pixels: &mut [u8],
    transform: i32,
    draw_w: u32, draw_h: u32,
) {
    for dy in 0..draw_h {
        for dx_local in 0..draw_w {
            let (sx, sy) = match transform {
                0 => (dx_local, dy),
                1 => (buf_w - 1 - dy, dx_local),
                2 => (buf_w - 1 - dx_local, buf_h - 1 - dy),
                3 => (dy, buf_h - 1 - dx_local),
                4 => (buf_w - 1 - dx_local, dy),
                5 => (buf_w - 1 - dy, buf_h - 1 - dx_local),
                6 => (dx_local, buf_h - 1 - dy),
                7 => (dy, dx_local),
                _ => (dx_local, dy),
            };

            let out_x = origin_x + dx_local as i32;
            let out_y = origin_y + dy as i32;
            if out_x < 0 || out_y < 0 || out_x as u32 >= output_width || out_y as u32 >= output_height {
                continue;
            }
            let src_off = sy as usize * buf_stride as usize + sx as usize * 4;
            if src_off + 4 > src_data.len() { continue; }
            let dst_off = (out_y as u32 * output_stride + out_x as u32 * 4) as usize;
            if dst_off + 4 > pixels.len() { continue; }

            let alpha = src_data[src_off + 3];
            if alpha == 0 { continue; }
            if alpha == 255 {
                pixels[dst_off] = src_data[src_off + 2]; // B <- source B (byte 2)
                pixels[dst_off + 1] = src_data[src_off + 1]; // G <- source G (byte 1)
                pixels[dst_off + 2] = src_data[src_off];     // R <- source R (byte 0)
            } else {
                let a_inv = 255u32 - alpha as u32;
                let src_b = src_data[src_off + 2] as u32;
                let src_g = src_data[src_off + 1] as u32;
                let src_r = src_data[src_off] as u32;
                pixels[dst_off] =
                    ((src_b * alpha as u32 + pixels[dst_off] as u32 * a_inv) / 255) as u8;
                pixels[dst_off + 1] =
                    ((src_g * alpha as u32 + pixels[dst_off + 1] as u32 * a_inv) / 255) as u8;
                pixels[dst_off + 2] =
                    ((src_r * alpha as u32 + pixels[dst_off + 2] as u32 * a_inv) / 255) as u8;
            }
            pixels[dst_off + 3] = 0xff;
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod blit_tests {
    use super::*;

    #[test]
    fn test_blit_xrgb_direct_copy() {
        let src = vec![
            0x00, 0x00, 0xff, 0xff,
            0x00, 0xff, 0x00, 0xff,
            0xff, 0x00, 0x00, 0xff,
            0xff, 0xff, 0x00, 0xff,
        ];
        let mut dst = vec![0u8; 16];
        blit_xrgb8888(&src, 2, 2, 8, 0, 0, 2, 2, 8, &mut dst, 0, 2, 2);
        assert_eq!(&dst[0..4], &[0x00, 0x00, 0xff, 0xff]);
        assert_eq!(&dst[8..12], &[0xff, 0x00, 0x00, 0xff]);
    }

    #[test]
    fn test_blit_argb_alpha_blend() {
        let src = vec![0x00, 0x00, 0xff, 0x80];
        let mut dst = vec![0x00, 0x00, 0x00, 0xff];
        blit_argb8888(&src, 1, 1, 4, 0, 0, 1, 1, 4, &mut dst, 0, 1, 1);
        assert_eq!(dst[2], 128);
        assert_eq!(dst[3], 0xff);
    }

    #[test]
    fn test_blit_argb_opaque_skip() {
        let src = vec![0x00, 0x00, 0xff, 0x00];
        let mut dst = vec![0x00, 0x00, 0x00, 0xff];
        blit_argb8888(&src, 1, 1, 4, 0, 0, 1, 1, 4, &mut dst, 0, 1, 1);
        assert_eq!(&dst[..], &[0x00, 0x00, 0x00, 0xff]);
    }

    #[test]
    fn test_blit_abgr_r_b_swap() {
        let src = vec![0xff, 0x80, 0x40, 0xff];
        let mut dst = vec![0u8; 4];
        blit_abgr8888(&src, 1, 1, 4, 0, 0, 1, 1, 4, &mut dst, 0, 1, 1);
        assert_eq!(dst[0], 0x40);
        assert_eq!(dst[1], 0x80);
        assert_eq!(dst[2], 0xff);
        assert_eq!(dst[3], 0xff);
    }

    #[test]
    fn test_blit_transform_180() {
        let src = vec![
            0x01, 0x00, 0x00, 0xff,
            0x02, 0x00, 0x00, 0xff,
            0x03, 0x00, 0x00, 0xff,
            0x04, 0x00, 0x00, 0xff,
        ];
        let mut dst = vec![0u8; 16];
        blit_xrgb8888(&src, 2, 2, 8, 0, 0, 2, 2, 8, &mut dst, 2, 2, 2);
        assert_eq!(dst[0], 0x04);
        assert_eq!(dst[4], 0x03);
    }
}
